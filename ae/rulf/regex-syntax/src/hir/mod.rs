/*!
Defines a high-level intermediate representation for regular expressions.
*/
use std::char;
use std::cmp;
use std::error;
use std::fmt;
use std::result;
use std::u8;
use ast::Span;
use hir::interval::{Interval, IntervalSet, IntervalSetIter};
use unicode;
pub use hir::visitor::{visit, Visitor};
pub use unicode::CaseFoldError;
mod interval;
pub mod literal;
pub mod print;
pub mod translate;
mod visitor;
/// An error that can occur while translating an `Ast` to a `Hir`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Error {
    /// The kind of error.
    kind: ErrorKind,
    /// The original pattern that the translator's Ast was parsed from. Every
    /// span in an error is a valid range into this string.
    pattern: String,
    /// The span of this error, derived from the Ast given to the translator.
    span: Span,
}
impl Error {
    /// Return the type of this error.
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }
    /// The original pattern string in which this error occurred.
    ///
    /// Every span reported by this error is reported in terms of this string.
    pub fn pattern(&self) -> &str {
        &self.pattern
    }
    /// Return the span at which this error occurred.
    pub fn span(&self) -> &Span {
        &self.span
    }
}
/// The type of an error that occurred while building an `Hir`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    /// This error occurs when a Unicode feature is used when Unicode
    /// support is disabled. For example `(?-u:\pL)` would trigger this error.
    UnicodeNotAllowed,
    /// This error occurs when translating a pattern that could match a byte
    /// sequence that isn't UTF-8 and `allow_invalid_utf8` was disabled.
    InvalidUtf8,
    /// This occurs when an unrecognized Unicode property name could not
    /// be found.
    UnicodePropertyNotFound,
    /// This occurs when an unrecognized Unicode property value could not
    /// be found.
    UnicodePropertyValueNotFound,
    /// This occurs when a Unicode-aware Perl character class (`\w`, `\s` or
    /// `\d`) could not be found. This can occur when the `unicode-perl`
    /// crate feature is not enabled.
    UnicodePerlClassNotFound,
    /// This occurs when the Unicode simple case mapping tables are not
    /// available, and the regular expression required Unicode aware case
    /// insensitivity.
    UnicodeCaseUnavailable,
    /// This occurs when the translator attempts to construct a character class
    /// that is empty.
    ///
    /// Note that this restriction in the translator may be removed in the
    /// future.
    EmptyClassNotAllowed,
    /// Hints that destructuring should not be exhaustive.
    ///
    /// This enum may grow additional variants, so this makes sure clients
    /// don't count on exhaustive matching. (Otherwise, adding a new variant
    /// could break existing code.)
    #[doc(hidden)]
    __Nonexhaustive,
}
impl ErrorKind {
    #[allow(deprecated)]
    fn description(&self) -> &str {
        use self::ErrorKind::*;
        match *self {
            UnicodeNotAllowed => "Unicode not allowed here",
            InvalidUtf8 => "pattern can match invalid UTF-8",
            UnicodePropertyNotFound => "Unicode property not found",
            UnicodePropertyValueNotFound => "Unicode property value not found",
            UnicodePerlClassNotFound => {
                "Unicode-aware Perl class not found \
                 (make sure the unicode-perl feature is enabled)"
            }
            UnicodeCaseUnavailable => {
                "Unicode-aware case insensitivity matching is not available \
                 (make sure the unicode-case feature is enabled)"
            }
            EmptyClassNotAllowed => "empty character classes are not allowed",
            __Nonexhaustive => unreachable!(),
        }
    }
}
impl error::Error for Error {
    #[allow(deprecated)]
    fn description(&self) -> &str {
        self.kind.description()
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        ::error::Formatter::from(self).fmt(f)
    }
}
impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        #[allow(deprecated)] f.write_str(self.description())
    }
}
/// A high-level intermediate representation (HIR) for a regular expression.
///
/// The HIR of a regular expression represents an intermediate step between its
/// abstract syntax (a structured description of the concrete syntax) and
/// compiled byte codes. The purpose of HIR is to make regular expressions
/// easier to analyze. In particular, the AST is much more complex than the
/// HIR. For example, while an AST supports arbitrarily nested character
/// classes, the HIR will flatten all nested classes into a single set. The HIR
/// will also "compile away" every flag present in the concrete syntax. For
/// example, users of HIR expressions never need to worry about case folding;
/// it is handled automatically by the translator (e.g., by translating `(?i)A`
/// to `[aA]`).
///
/// If the HIR was produced by a translator that disallows invalid UTF-8, then
/// the HIR is guaranteed to match UTF-8 exclusively.
///
/// This type defines its own destructor that uses constant stack space and
/// heap space proportional to the size of the HIR.
///
/// The specific type of an HIR expression can be accessed via its `kind`
/// or `into_kind` methods. This extra level of indirection exists for two
/// reasons:
///
/// 1. Construction of an HIR expression *must* use the constructor methods
///    on this `Hir` type instead of building the `HirKind` values directly.
///    This permits construction to enforce invariants like "concatenations
///    always consist of two or more sub-expressions."
/// 2. Every HIR expression contains attributes that are defined inductively,
///    and can be computed cheaply during the construction process. For
///    example, one such attribute is whether the expression must match at the
///    beginning of the text.
///
/// Also, an `Hir`'s `fmt::Display` implementation prints an HIR as a regular
/// expression pattern string, and uses constant stack space and heap space
/// proportional to the size of the `Hir`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Hir {
    /// The underlying HIR kind.
    kind: HirKind,
    /// Analysis info about this HIR, computed during construction.
    info: HirInfo,
}
/// The kind of an arbitrary `Hir` expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HirKind {
    /// The empty regular expression, which matches everything, including the
    /// empty string.
    Empty,
    /// A single literal character that matches exactly this character.
    Literal(Literal),
    /// A single character class that matches any of the characters in the
    /// class. A class can either consist of Unicode scalar values as
    /// characters, or it can use bytes.
    Class(Class),
    /// An anchor assertion. An anchor assertion match always has zero length.
    Anchor(Anchor),
    /// A word boundary assertion, which may or may not be Unicode aware. A
    /// word boundary assertion match always has zero length.
    WordBoundary(WordBoundary),
    /// A repetition operation applied to a child expression.
    Repetition(Repetition),
    /// A possibly capturing group, which contains a child expression.
    Group(Group),
    /// A concatenation of expressions. A concatenation always has at least two
    /// child expressions.
    ///
    /// A concatenation matches only if each of its child expression matches
    /// one after the other.
    Concat(Vec<Hir>),
    /// An alternation of expressions. An alternation always has at least two
    /// child expressions.
    ///
    /// An alternation matches only if at least one of its child expression
    /// matches. If multiple expressions match, then the leftmost is preferred.
    Alternation(Vec<Hir>),
}
impl Hir {
    /// Returns a reference to the underlying HIR kind.
    pub fn kind(&self) -> &HirKind {
        &self.kind
    }
    /// Consumes ownership of this HIR expression and returns its underlying
    /// `HirKind`.
    pub fn into_kind(mut self) -> HirKind {
        use std::mem;
        mem::replace(&mut self.kind, HirKind::Empty)
    }
    /// Returns an empty HIR expression.
    ///
    /// An empty HIR expression always matches, including the empty string.
    pub fn empty() -> Hir {
        let mut info = HirInfo::new();
        info.set_always_utf8(true);
        info.set_all_assertions(true);
        info.set_anchored_start(false);
        info.set_anchored_end(false);
        info.set_line_anchored_start(false);
        info.set_line_anchored_end(false);
        info.set_any_anchored_start(false);
        info.set_any_anchored_end(false);
        info.set_match_empty(true);
        info.set_literal(false);
        info.set_alternation_literal(false);
        Hir {
            kind: HirKind::Empty,
            info: info,
        }
    }
    /// Creates a literal HIR expression.
    ///
    /// If the given literal has a `Byte` variant with an ASCII byte, then this
    /// method panics. This enforces the invariant that `Byte` variants are
    /// only used to express matching of invalid UTF-8.
    pub fn literal(lit: Literal) -> Hir {
        if let Literal::Byte(b) = lit {
            assert!(b > 0x7F);
        }
        let mut info = HirInfo::new();
        info.set_always_utf8(lit.is_unicode());
        info.set_all_assertions(false);
        info.set_anchored_start(false);
        info.set_anchored_end(false);
        info.set_line_anchored_start(false);
        info.set_line_anchored_end(false);
        info.set_any_anchored_start(false);
        info.set_any_anchored_end(false);
        info.set_match_empty(false);
        info.set_literal(true);
        info.set_alternation_literal(true);
        Hir {
            kind: HirKind::Literal(lit),
            info: info,
        }
    }
    /// Creates a class HIR expression.
    pub fn class(class: Class) -> Hir {
        let mut info = HirInfo::new();
        info.set_always_utf8(class.is_always_utf8());
        info.set_all_assertions(false);
        info.set_anchored_start(false);
        info.set_anchored_end(false);
        info.set_line_anchored_start(false);
        info.set_line_anchored_end(false);
        info.set_any_anchored_start(false);
        info.set_any_anchored_end(false);
        info.set_match_empty(false);
        info.set_literal(false);
        info.set_alternation_literal(false);
        Hir {
            kind: HirKind::Class(class),
            info: info,
        }
    }
    /// Creates an anchor assertion HIR expression.
    pub fn anchor(anchor: Anchor) -> Hir {
        let mut info = HirInfo::new();
        info.set_always_utf8(true);
        info.set_all_assertions(true);
        info.set_anchored_start(false);
        info.set_anchored_end(false);
        info.set_line_anchored_start(false);
        info.set_line_anchored_end(false);
        info.set_any_anchored_start(false);
        info.set_any_anchored_end(false);
        info.set_match_empty(true);
        info.set_literal(false);
        info.set_alternation_literal(false);
        if let Anchor::StartText = anchor {
            info.set_anchored_start(true);
            info.set_line_anchored_start(true);
            info.set_any_anchored_start(true);
        }
        if let Anchor::EndText = anchor {
            info.set_anchored_end(true);
            info.set_line_anchored_end(true);
            info.set_any_anchored_end(true);
        }
        if let Anchor::StartLine = anchor {
            info.set_line_anchored_start(true);
        }
        if let Anchor::EndLine = anchor {
            info.set_line_anchored_end(true);
        }
        Hir {
            kind: HirKind::Anchor(anchor),
            info: info,
        }
    }
    /// Creates a word boundary assertion HIR expression.
    pub fn word_boundary(word_boundary: WordBoundary) -> Hir {
        let mut info = HirInfo::new();
        info.set_always_utf8(true);
        info.set_all_assertions(true);
        info.set_anchored_start(false);
        info.set_anchored_end(false);
        info.set_line_anchored_start(false);
        info.set_line_anchored_end(false);
        info.set_any_anchored_start(false);
        info.set_any_anchored_end(false);
        info.set_literal(false);
        info.set_alternation_literal(false);
        info.set_match_empty(word_boundary.is_negated());
        if let WordBoundary::AsciiNegate = word_boundary {
            info.set_always_utf8(false);
        }
        Hir {
            kind: HirKind::WordBoundary(word_boundary),
            info: info,
        }
    }
    /// Creates a repetition HIR expression.
    pub fn repetition(rep: Repetition) -> Hir {
        let mut info = HirInfo::new();
        info.set_always_utf8(rep.hir.is_always_utf8());
        info.set_all_assertions(rep.hir.is_all_assertions());
        info.set_anchored_start(!rep.is_match_empty() && rep.hir.is_anchored_start());
        info.set_anchored_end(!rep.is_match_empty() && rep.hir.is_anchored_end());
        info.set_line_anchored_start(
            !rep.is_match_empty() && rep.hir.is_anchored_start(),
        );
        info.set_line_anchored_end(!rep.is_match_empty() && rep.hir.is_anchored_end());
        info.set_any_anchored_start(rep.hir.is_any_anchored_start());
        info.set_any_anchored_end(rep.hir.is_any_anchored_end());
        info.set_match_empty(rep.is_match_empty() || rep.hir.is_match_empty());
        info.set_literal(false);
        info.set_alternation_literal(false);
        Hir {
            kind: HirKind::Repetition(rep),
            info: info,
        }
    }
    /// Creates a group HIR expression.
    pub fn group(group: Group) -> Hir {
        let mut info = HirInfo::new();
        info.set_always_utf8(group.hir.is_always_utf8());
        info.set_all_assertions(group.hir.is_all_assertions());
        info.set_anchored_start(group.hir.is_anchored_start());
        info.set_anchored_end(group.hir.is_anchored_end());
        info.set_line_anchored_start(group.hir.is_line_anchored_start());
        info.set_line_anchored_end(group.hir.is_line_anchored_end());
        info.set_any_anchored_start(group.hir.is_any_anchored_start());
        info.set_any_anchored_end(group.hir.is_any_anchored_end());
        info.set_match_empty(group.hir.is_match_empty());
        info.set_literal(false);
        info.set_alternation_literal(false);
        Hir {
            kind: HirKind::Group(group),
            info: info,
        }
    }
    /// Returns the concatenation of the given expressions.
    ///
    /// This flattens the concatenation as appropriate.
    pub fn concat(mut exprs: Vec<Hir>) -> Hir {
        match exprs.len() {
            0 => Hir::empty(),
            1 => exprs.pop().unwrap(),
            _ => {
                let mut info = HirInfo::new();
                info.set_always_utf8(true);
                info.set_all_assertions(true);
                info.set_any_anchored_start(false);
                info.set_any_anchored_end(false);
                info.set_match_empty(true);
                info.set_literal(true);
                info.set_alternation_literal(true);
                for e in &exprs {
                    let x = info.is_always_utf8() && e.is_always_utf8();
                    info.set_always_utf8(x);
                    let x = info.is_all_assertions() && e.is_all_assertions();
                    info.set_all_assertions(x);
                    let x = info.is_any_anchored_start() || e.is_any_anchored_start();
                    info.set_any_anchored_start(x);
                    let x = info.is_any_anchored_end() || e.is_any_anchored_end();
                    info.set_any_anchored_end(x);
                    let x = info.is_match_empty() && e.is_match_empty();
                    info.set_match_empty(x);
                    let x = info.is_literal() && e.is_literal();
                    info.set_literal(x);
                    let x = info.is_alternation_literal() && e.is_alternation_literal();
                    info.set_alternation_literal(x);
                }
                info.set_anchored_start(
                    exprs
                        .iter()
                        .take_while(|e| {
                            e.is_anchored_start() || e.is_all_assertions()
                        })
                        .any(|e| e.is_anchored_start()),
                );
                info.set_anchored_end(
                    exprs
                        .iter()
                        .rev()
                        .take_while(|e| { e.is_anchored_end() || e.is_all_assertions() })
                        .any(|e| e.is_anchored_end()),
                );
                info.set_line_anchored_start(
                    exprs
                        .iter()
                        .take_while(|e| {
                            e.is_line_anchored_start() || e.is_all_assertions()
                        })
                        .any(|e| e.is_line_anchored_start()),
                );
                info.set_line_anchored_end(
                    exprs
                        .iter()
                        .rev()
                        .take_while(|e| {
                            e.is_line_anchored_end() || e.is_all_assertions()
                        })
                        .any(|e| e.is_line_anchored_end()),
                );
                Hir {
                    kind: HirKind::Concat(exprs),
                    info: info,
                }
            }
        }
    }
    /// Returns the alternation of the given expressions.
    ///
    /// This flattens the alternation as appropriate.
    pub fn alternation(mut exprs: Vec<Hir>) -> Hir {
        match exprs.len() {
            0 => Hir::empty(),
            1 => exprs.pop().unwrap(),
            _ => {
                let mut info = HirInfo::new();
                info.set_always_utf8(true);
                info.set_all_assertions(true);
                info.set_anchored_start(true);
                info.set_anchored_end(true);
                info.set_line_anchored_start(true);
                info.set_line_anchored_end(true);
                info.set_any_anchored_start(false);
                info.set_any_anchored_end(false);
                info.set_match_empty(false);
                info.set_literal(false);
                info.set_alternation_literal(true);
                for e in &exprs {
                    let x = info.is_always_utf8() && e.is_always_utf8();
                    info.set_always_utf8(x);
                    let x = info.is_all_assertions() && e.is_all_assertions();
                    info.set_all_assertions(x);
                    let x = info.is_anchored_start() && e.is_anchored_start();
                    info.set_anchored_start(x);
                    let x = info.is_anchored_end() && e.is_anchored_end();
                    info.set_anchored_end(x);
                    let x = info.is_line_anchored_start() && e.is_line_anchored_start();
                    info.set_line_anchored_start(x);
                    let x = info.is_line_anchored_end() && e.is_line_anchored_end();
                    info.set_line_anchored_end(x);
                    let x = info.is_any_anchored_start() || e.is_any_anchored_start();
                    info.set_any_anchored_start(x);
                    let x = info.is_any_anchored_end() || e.is_any_anchored_end();
                    info.set_any_anchored_end(x);
                    let x = info.is_match_empty() || e.is_match_empty();
                    info.set_match_empty(x);
                    let x = info.is_alternation_literal() && e.is_literal();
                    info.set_alternation_literal(x);
                }
                Hir {
                    kind: HirKind::Alternation(exprs),
                    info: info,
                }
            }
        }
    }
    /// Build an HIR expression for `.`.
    ///
    /// A `.` expression matches any character except for `\n`. To build an
    /// expression that matches any character, including `\n`, use the `any`
    /// method.
    ///
    /// If `bytes` is `true`, then this assumes characters are limited to a
    /// single byte.
    pub fn dot(bytes: bool) -> Hir {
        if bytes {
            let mut cls = ClassBytes::empty();
            cls.push(ClassBytesRange::new(b'\0', b'\x09'));
            cls.push(ClassBytesRange::new(b'\x0B', b'\xFF'));
            Hir::class(Class::Bytes(cls))
        } else {
            let mut cls = ClassUnicode::empty();
            cls.push(ClassUnicodeRange::new('\0', '\x09'));
            cls.push(ClassUnicodeRange::new('\x0B', '\u{10FFFF}'));
            Hir::class(Class::Unicode(cls))
        }
    }
    /// Build an HIR expression for `(?s).`.
    ///
    /// A `(?s).` expression matches any character, including `\n`. To build an
    /// expression that matches any character except for `\n`, then use the
    /// `dot` method.
    ///
    /// If `bytes` is `true`, then this assumes characters are limited to a
    /// single byte.
    pub fn any(bytes: bool) -> Hir {
        if bytes {
            let mut cls = ClassBytes::empty();
            cls.push(ClassBytesRange::new(b'\0', b'\xFF'));
            Hir::class(Class::Bytes(cls))
        } else {
            let mut cls = ClassUnicode::empty();
            cls.push(ClassUnicodeRange::new('\0', '\u{10FFFF}'));
            Hir::class(Class::Unicode(cls))
        }
    }
    /// Return true if and only if this HIR will always match valid UTF-8.
    ///
    /// When this returns false, then it is possible for this HIR expression
    /// to match invalid UTF-8.
    pub fn is_always_utf8(&self) -> bool {
        self.info.is_always_utf8()
    }
    /// Returns true if and only if this entire HIR expression is made up of
    /// zero-width assertions.
    ///
    /// This includes expressions like `^$\b\A\z` and even `((\b)+())*^`, but
    /// not `^a`.
    pub fn is_all_assertions(&self) -> bool {
        self.info.is_all_assertions()
    }
    /// Return true if and only if this HIR is required to match from the
    /// beginning of text. This includes expressions like `^foo`, `^(foo|bar)`,
    /// `^foo|^bar` but not `^foo|bar`.
    pub fn is_anchored_start(&self) -> bool {
        self.info.is_anchored_start()
    }
    /// Return true if and only if this HIR is required to match at the end
    /// of text. This includes expressions like `foo$`, `(foo|bar)$`,
    /// `foo$|bar$` but not `foo$|bar`.
    pub fn is_anchored_end(&self) -> bool {
        self.info.is_anchored_end()
    }
    /// Return true if and only if this HIR is required to match from the
    /// beginning of text or the beginning of a line. This includes expressions
    /// like `^foo`, `(?m)^foo`, `^(foo|bar)`, `^(foo|bar)`, `(?m)^foo|^bar`
    /// but not `^foo|bar` or `(?m)^foo|bar`.
    ///
    /// Note that if `is_anchored_start` is `true`, then
    /// `is_line_anchored_start` will also be `true`. The reverse implication
    /// is not true. For example, `(?m)^foo` is line anchored, but not
    /// `is_anchored_start`.
    pub fn is_line_anchored_start(&self) -> bool {
        self.info.is_line_anchored_start()
    }
    /// Return true if and only if this HIR is required to match at the
    /// end of text or the end of a line. This includes expressions like
    /// `foo$`, `(?m)foo$`, `(foo|bar)$`, `(?m)(foo|bar)$`, `foo$|bar$`,
    /// `(?m)(foo|bar)$`, but not `foo$|bar` or `(?m)foo$|bar`.
    ///
    /// Note that if `is_anchored_end` is `true`, then
    /// `is_line_anchored_end` will also be `true`. The reverse implication
    /// is not true. For example, `(?m)foo$` is line anchored, but not
    /// `is_anchored_end`.
    pub fn is_line_anchored_end(&self) -> bool {
        self.info.is_line_anchored_end()
    }
    /// Return true if and only if this HIR contains any sub-expression that
    /// is required to match at the beginning of text. Specifically, this
    /// returns true if the `^` symbol (when multiline mode is disabled) or the
    /// `\A` escape appear anywhere in the regex.
    pub fn is_any_anchored_start(&self) -> bool {
        self.info.is_any_anchored_start()
    }
    /// Return true if and only if this HIR contains any sub-expression that is
    /// required to match at the end of text. Specifically, this returns true
    /// if the `$` symbol (when multiline mode is disabled) or the `\z` escape
    /// appear anywhere in the regex.
    pub fn is_any_anchored_end(&self) -> bool {
        self.info.is_any_anchored_end()
    }
    /// Return true if and only if the empty string is part of the language
    /// matched by this regular expression.
    ///
    /// This includes `a*`, `a?b*`, `a{0}`, `()`, `()+`, `^$`, `a|b?`, `\B`,
    /// but not `a`, `a+` or `\b`.
    pub fn is_match_empty(&self) -> bool {
        self.info.is_match_empty()
    }
    /// Return true if and only if this HIR is a simple literal. This is only
    /// true when this HIR expression is either itself a `Literal` or a
    /// concatenation of only `Literal`s.
    ///
    /// For example, `f` and `foo` are literals, but `f+`, `(foo)`, `foo()`,
    /// `` are not (even though that contain sub-expressions that are literals).
    pub fn is_literal(&self) -> bool {
        self.info.is_literal()
    }
    /// Return true if and only if this HIR is either a simple literal or an
    /// alternation of simple literals. This is only
    /// true when this HIR expression is either itself a `Literal` or a
    /// concatenation of only `Literal`s or an alternation of only `Literal`s.
    ///
    /// For example, `f`, `foo`, `a|b|c`, and `foo|bar|baz` are alternation
    /// literals, but `f+`, `(foo)`, `foo()`, ``
    /// are not (even though that contain sub-expressions that are literals).
    pub fn is_alternation_literal(&self) -> bool {
        self.info.is_alternation_literal()
    }
}
impl HirKind {
    /// Return true if and only if this HIR is the empty regular expression.
    ///
    /// Note that this is not defined inductively. That is, it only tests if
    /// this kind is the `Empty` variant. To get the inductive definition,
    /// use the `is_match_empty` method on [`Hir`](struct.Hir.html).
    pub fn is_empty(&self) -> bool {
        match *self {
            HirKind::Empty => true,
            _ => false,
        }
    }
    /// Returns true if and only if this kind has any (including possibly
    /// empty) subexpressions.
    pub fn has_subexprs(&self) -> bool {
        match *self {
            HirKind::Empty
            | HirKind::Literal(_)
            | HirKind::Class(_)
            | HirKind::Anchor(_)
            | HirKind::WordBoundary(_) => false,
            HirKind::Group(_)
            | HirKind::Repetition(_)
            | HirKind::Concat(_)
            | HirKind::Alternation(_) => true,
        }
    }
}
/// Print a display representation of this Hir.
///
/// The result of this is a valid regular expression pattern string.
///
/// This implementation uses constant stack space and heap space proportional
/// to the size of the `Hir`.
impl fmt::Display for Hir {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use hir::print::Printer;
        Printer::new().print(self, f)
    }
}
/// The high-level intermediate representation of a literal.
///
/// A literal corresponds to a single character, where a character is either
/// defined by a Unicode scalar value or an arbitrary byte. Unicode characters
/// are preferred whenever possible. In particular, a `Byte` variant is only
/// ever produced when it could match invalid UTF-8.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Literal {
    /// A single character represented by a Unicode scalar value.
    Unicode(char),
    /// A single character represented by an arbitrary byte.
    Byte(u8),
}
impl Literal {
    /// Returns true if and only if this literal corresponds to a Unicode
    /// scalar value.
    pub fn is_unicode(&self) -> bool {
        match *self {
            Literal::Unicode(_) => true,
            Literal::Byte(b) if b <= 0x7F => true,
            Literal::Byte(_) => false,
        }
    }
}
/// The high-level intermediate representation of a character class.
///
/// A character class corresponds to a set of characters. A character is either
/// defined by a Unicode scalar value or a byte. Unicode characters are used
/// by default, while bytes are used when Unicode mode (via the `u` flag) is
/// disabled.
///
/// A character class, regardless of its character type, is represented by a
/// sequence of non-overlapping non-adjacent ranges of characters.
///
/// Note that unlike [`Literal`](enum.Literal.html), a `Bytes` variant may
/// be produced even when it exclusively matches valid UTF-8. This is because
/// a `Bytes` variant represents an intention by the author of the regular
/// expression to disable Unicode mode, which in turn impacts the semantics of
/// case insensitive matching. For example, `(?i)k` and `(?i-u)k` will not
/// match the same set of strings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Class {
    /// A set of characters represented by Unicode scalar values.
    Unicode(ClassUnicode),
    /// A set of characters represented by arbitrary bytes (one byte per
    /// character).
    Bytes(ClassBytes),
}
impl Class {
    /// Apply Unicode simple case folding to this character class, in place.
    /// The character class will be expanded to include all simple case folded
    /// character variants.
    ///
    /// If this is a byte oriented character class, then this will be limited
    /// to the ASCII ranges `A-Z` and `a-z`.
    pub fn case_fold_simple(&mut self) {
        match *self {
            Class::Unicode(ref mut x) => x.case_fold_simple(),
            Class::Bytes(ref mut x) => x.case_fold_simple(),
        }
    }
    /// Negate this character class in place.
    ///
    /// After completion, this character class will contain precisely the
    /// characters that weren't previously in the class.
    pub fn negate(&mut self) {
        match *self {
            Class::Unicode(ref mut x) => x.negate(),
            Class::Bytes(ref mut x) => x.negate(),
        }
    }
    /// Returns true if and only if this character class will only ever match
    /// valid UTF-8.
    ///
    /// A character class can match invalid UTF-8 only when the following
    /// conditions are met:
    ///
    /// 1. The translator was configured to permit generating an expression
    ///    that can match invalid UTF-8. (By default, this is disabled.)
    /// 2. Unicode mode (via the `u` flag) was disabled either in the concrete
    ///    syntax or in the parser builder. By default, Unicode mode is
    ///    enabled.
    pub fn is_always_utf8(&self) -> bool {
        match *self {
            Class::Unicode(_) => true,
            Class::Bytes(ref x) => x.is_all_ascii(),
        }
    }
}
/// A set of characters represented by Unicode scalar values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassUnicode {
    set: IntervalSet<ClassUnicodeRange>,
}
impl ClassUnicode {
    /// Create a new class from a sequence of ranges.
    ///
    /// The given ranges do not need to be in any specific order, and ranges
    /// may overlap.
    pub fn new<I>(ranges: I) -> ClassUnicode
    where
        I: IntoIterator<Item = ClassUnicodeRange>,
    {
        ClassUnicode {
            set: IntervalSet::new(ranges),
        }
    }
    /// Create a new class with no ranges.
    pub fn empty() -> ClassUnicode {
        ClassUnicode::new(vec![])
    }
    /// Add a new range to this set.
    pub fn push(&mut self, range: ClassUnicodeRange) {
        self.set.push(range);
    }
    /// Return an iterator over all ranges in this class.
    ///
    /// The iterator yields ranges in ascending order.
    pub fn iter(&self) -> ClassUnicodeIter {
        ClassUnicodeIter(self.set.iter())
    }
    /// Return the underlying ranges as a slice.
    pub fn ranges(&self) -> &[ClassUnicodeRange] {
        self.set.intervals()
    }
    /// Expand this character class such that it contains all case folded
    /// characters, according to Unicode's "simple" mapping. For example, if
    /// this class consists of the range `a-z`, then applying case folding will
    /// result in the class containing both the ranges `a-z` and `A-Z`.
    ///
    /// # Panics
    ///
    /// This routine panics when the case mapping data necessary for this
    /// routine to complete is unavailable. This occurs when the `unicode-case`
    /// feature is not enabled.
    ///
    /// Callers should prefer using `try_case_fold_simple` instead, which will
    /// return an error instead of panicking.
    pub fn case_fold_simple(&mut self) {
        self.set.case_fold_simple().expect("unicode-case feature must be enabled");
    }
    /// Expand this character class such that it contains all case folded
    /// characters, according to Unicode's "simple" mapping. For example, if
    /// this class consists of the range `a-z`, then applying case folding will
    /// result in the class containing both the ranges `a-z` and `A-Z`.
    ///
    /// # Error
    ///
    /// This routine returns an error when the case mapping data necessary
    /// for this routine to complete is unavailable. This occurs when the
    /// `unicode-case` feature is not enabled.
    pub fn try_case_fold_simple(&mut self) -> result::Result<(), CaseFoldError> {
        self.set.case_fold_simple()
    }
    /// Negate this character class.
    ///
    /// For all `c` where `c` is a Unicode scalar value, if `c` was in this
    /// set, then it will not be in this set after negation.
    pub fn negate(&mut self) {
        self.set.negate();
    }
    /// Union this character class with the given character class, in place.
    pub fn union(&mut self, other: &ClassUnicode) {
        self.set.union(&other.set);
    }
    /// Intersect this character class with the given character class, in
    /// place.
    pub fn intersect(&mut self, other: &ClassUnicode) {
        self.set.intersect(&other.set);
    }
    /// Subtract the given character class from this character class, in place.
    pub fn difference(&mut self, other: &ClassUnicode) {
        self.set.difference(&other.set);
    }
    /// Compute the symmetric difference of the given character classes, in
    /// place.
    ///
    /// This computes the symmetric difference of two character classes. This
    /// removes all elements in this class that are also in the given class,
    /// but all adds all elements from the given class that aren't in this
    /// class. That is, the class will contain all elements in either class,
    /// but will not contain any elements that are in both classes.
    pub fn symmetric_difference(&mut self, other: &ClassUnicode) {
        self.set.symmetric_difference(&other.set);
    }
    /// Returns true if and only if this character class will either match
    /// nothing or only ASCII bytes. Stated differently, this returns false
    /// if and only if this class contains a non-ASCII codepoint.
    pub fn is_all_ascii(&self) -> bool {
        self.set.intervals().last().map_or(true, |r| r.end <= '\x7F')
    }
}
/// An iterator over all ranges in a Unicode character class.
///
/// The lifetime `'a` refers to the lifetime of the underlying class.
#[derive(Debug)]
pub struct ClassUnicodeIter<'a>(IntervalSetIter<'a, ClassUnicodeRange>);
impl<'a> Iterator for ClassUnicodeIter<'a> {
    type Item = &'a ClassUnicodeRange;
    fn next(&mut self) -> Option<&'a ClassUnicodeRange> {
        self.0.next()
    }
}
/// A single range of characters represented by Unicode scalar values.
///
/// The range is closed. That is, the start and end of the range are included
/// in the range.
#[derive(Clone, Copy, Default, Eq, PartialEq, PartialOrd, Ord)]
pub struct ClassUnicodeRange {
    start: char,
    end: char,
}
impl fmt::Debug for ClassUnicodeRange {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let start = if !self.start.is_whitespace() && !self.start.is_control() {
            self.start.to_string()
        } else {
            format!("0x{:X}", self.start as u32)
        };
        let end = if !self.end.is_whitespace() && !self.end.is_control() {
            self.end.to_string()
        } else {
            format!("0x{:X}", self.end as u32)
        };
        f.debug_struct("ClassUnicodeRange")
            .field("start", &start)
            .field("end", &end)
            .finish()
    }
}
impl Interval for ClassUnicodeRange {
    type Bound = char;
    #[inline]
    fn lower(&self) -> char {
        self.start
    }
    #[inline]
    fn upper(&self) -> char {
        self.end
    }
    #[inline]
    fn set_lower(&mut self, bound: char) {
        self.start = bound;
    }
    #[inline]
    fn set_upper(&mut self, bound: char) {
        self.end = bound;
    }
    /// Apply simple case folding to this Unicode scalar value range.
    ///
    /// Additional ranges are appended to the given vector. Canonical ordering
    /// is *not* maintained in the given vector.
    fn case_fold_simple(
        &self,
        ranges: &mut Vec<ClassUnicodeRange>,
    ) -> Result<(), unicode::CaseFoldError> {
        if !unicode::contains_simple_case_mapping(self.start, self.end)? {
            return Ok(());
        }
        let start = self.start as u32;
        let end = (self.end as u32).saturating_add(1);
        let mut next_simple_cp = None;
        for cp in (start..end).filter_map(char::from_u32) {
            if next_simple_cp.map_or(false, |next| cp < next) {
                continue;
            }
            let it = match unicode::simple_fold(cp)? {
                Ok(it) => it,
                Err(next) => {
                    next_simple_cp = next;
                    continue;
                }
            };
            for cp_folded in it {
                ranges.push(ClassUnicodeRange::new(cp_folded, cp_folded));
            }
        }
        Ok(())
    }
}
impl ClassUnicodeRange {
    /// Create a new Unicode scalar value range for a character class.
    ///
    /// The returned range is always in a canonical form. That is, the range
    /// returned always satisfies the invariant that `start <= end`.
    pub fn new(start: char, end: char) -> ClassUnicodeRange {
        ClassUnicodeRange::create(start, end)
    }
    /// Return the start of this range.
    ///
    /// The start of a range is always less than or equal to the end of the
    /// range.
    pub fn start(&self) -> char {
        self.start
    }
    /// Return the end of this range.
    ///
    /// The end of a range is always greater than or equal to the start of the
    /// range.
    pub fn end(&self) -> char {
        self.end
    }
}
/// A set of characters represented by arbitrary bytes (where one byte
/// corresponds to one character).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassBytes {
    set: IntervalSet<ClassBytesRange>,
}
impl ClassBytes {
    /// Create a new class from a sequence of ranges.
    ///
    /// The given ranges do not need to be in any specific order, and ranges
    /// may overlap.
    pub fn new<I>(ranges: I) -> ClassBytes
    where
        I: IntoIterator<Item = ClassBytesRange>,
    {
        ClassBytes {
            set: IntervalSet::new(ranges),
        }
    }
    /// Create a new class with no ranges.
    pub fn empty() -> ClassBytes {
        ClassBytes::new(vec![])
    }
    /// Add a new range to this set.
    pub fn push(&mut self, range: ClassBytesRange) {
        self.set.push(range);
    }
    /// Return an iterator over all ranges in this class.
    ///
    /// The iterator yields ranges in ascending order.
    pub fn iter(&self) -> ClassBytesIter {
        ClassBytesIter(self.set.iter())
    }
    /// Return the underlying ranges as a slice.
    pub fn ranges(&self) -> &[ClassBytesRange] {
        self.set.intervals()
    }
    /// Expand this character class such that it contains all case folded
    /// characters. For example, if this class consists of the range `a-z`,
    /// then applying case folding will result in the class containing both the
    /// ranges `a-z` and `A-Z`.
    ///
    /// Note that this only applies ASCII case folding, which is limited to the
    /// characters `a-z` and `A-Z`.
    pub fn case_fold_simple(&mut self) {
        self.set.case_fold_simple().expect("ASCII case folding never fails");
    }
    /// Negate this byte class.
    ///
    /// For all `b` where `b` is a any byte, if `b` was in this set, then it
    /// will not be in this set after negation.
    pub fn negate(&mut self) {
        self.set.negate();
    }
    /// Union this byte class with the given byte class, in place.
    pub fn union(&mut self, other: &ClassBytes) {
        self.set.union(&other.set);
    }
    /// Intersect this byte class with the given byte class, in place.
    pub fn intersect(&mut self, other: &ClassBytes) {
        self.set.intersect(&other.set);
    }
    /// Subtract the given byte class from this byte class, in place.
    pub fn difference(&mut self, other: &ClassBytes) {
        self.set.difference(&other.set);
    }
    /// Compute the symmetric difference of the given byte classes, in place.
    ///
    /// This computes the symmetric difference of two byte classes. This
    /// removes all elements in this class that are also in the given class,
    /// but all adds all elements from the given class that aren't in this
    /// class. That is, the class will contain all elements in either class,
    /// but will not contain any elements that are in both classes.
    pub fn symmetric_difference(&mut self, other: &ClassBytes) {
        self.set.symmetric_difference(&other.set);
    }
    /// Returns true if and only if this character class will either match
    /// nothing or only ASCII bytes. Stated differently, this returns false
    /// if and only if this class contains a non-ASCII byte.
    pub fn is_all_ascii(&self) -> bool {
        self.set.intervals().last().map_or(true, |r| r.end <= 0x7F)
    }
}
/// An iterator over all ranges in a byte character class.
///
/// The lifetime `'a` refers to the lifetime of the underlying class.
#[derive(Debug)]
pub struct ClassBytesIter<'a>(IntervalSetIter<'a, ClassBytesRange>);
impl<'a> Iterator for ClassBytesIter<'a> {
    type Item = &'a ClassBytesRange;
    fn next(&mut self) -> Option<&'a ClassBytesRange> {
        self.0.next()
    }
}
/// A single range of characters represented by arbitrary bytes.
///
/// The range is closed. That is, the start and end of the range are included
/// in the range.
#[derive(Clone, Copy, Default, Eq, PartialEq, PartialOrd, Ord)]
pub struct ClassBytesRange {
    start: u8,
    end: u8,
}
impl Interval for ClassBytesRange {
    type Bound = u8;
    #[inline]
    fn lower(&self) -> u8 {
        self.start
    }
    #[inline]
    fn upper(&self) -> u8 {
        self.end
    }
    #[inline]
    fn set_lower(&mut self, bound: u8) {
        self.start = bound;
    }
    #[inline]
    fn set_upper(&mut self, bound: u8) {
        self.end = bound;
    }
    /// Apply simple case folding to this byte range. Only ASCII case mappings
    /// (for a-z) are applied.
    ///
    /// Additional ranges are appended to the given vector. Canonical ordering
    /// is *not* maintained in the given vector.
    fn case_fold_simple(
        &self,
        ranges: &mut Vec<ClassBytesRange>,
    ) -> Result<(), unicode::CaseFoldError> {
        if !ClassBytesRange::new(b'a', b'z').is_intersection_empty(self) {
            let lower = cmp::max(self.start, b'a');
            let upper = cmp::min(self.end, b'z');
            ranges.push(ClassBytesRange::new(lower - 32, upper - 32));
        }
        if !ClassBytesRange::new(b'A', b'Z').is_intersection_empty(self) {
            let lower = cmp::max(self.start, b'A');
            let upper = cmp::min(self.end, b'Z');
            ranges.push(ClassBytesRange::new(lower + 32, upper + 32));
        }
        Ok(())
    }
}
impl ClassBytesRange {
    /// Create a new byte range for a character class.
    ///
    /// The returned range is always in a canonical form. That is, the range
    /// returned always satisfies the invariant that `start <= end`.
    pub fn new(start: u8, end: u8) -> ClassBytesRange {
        ClassBytesRange::create(start, end)
    }
    /// Return the start of this range.
    ///
    /// The start of a range is always less than or equal to the end of the
    /// range.
    pub fn start(&self) -> u8 {
        self.start
    }
    /// Return the end of this range.
    ///
    /// The end of a range is always greater than or equal to the start of the
    /// range.
    pub fn end(&self) -> u8 {
        self.end
    }
}
impl fmt::Debug for ClassBytesRange {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut debug = f.debug_struct("ClassBytesRange");
        if self.start <= 0x7F {
            debug.field("start", &(self.start as char));
        } else {
            debug.field("start", &self.start);
        }
        if self.end <= 0x7F {
            debug.field("end", &(self.end as char));
        } else {
            debug.field("end", &self.end);
        }
        debug.finish()
    }
}
/// The high-level intermediate representation for an anchor assertion.
///
/// A matching anchor assertion is always zero-length.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Anchor {
    /// Match the beginning of a line or the beginning of text. Specifically,
    /// this matches at the starting position of the input, or at the position
    /// immediately following a `\n` character.
    StartLine,
    /// Match the end of a line or the end of text. Specifically,
    /// this matches at the end position of the input, or at the position
    /// immediately preceding a `\n` character.
    EndLine,
    /// Match the beginning of text. Specifically, this matches at the starting
    /// position of the input.
    StartText,
    /// Match the end of text. Specifically, this matches at the ending
    /// position of the input.
    EndText,
}
/// The high-level intermediate representation for a word-boundary assertion.
///
/// A matching word boundary assertion is always zero-length.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WordBoundary {
    /// Match a Unicode-aware word boundary. That is, this matches a position
    /// where the left adjacent character and right adjacent character
    /// correspond to a word and non-word or a non-word and word character.
    Unicode,
    /// Match a Unicode-aware negation of a word boundary.
    UnicodeNegate,
    /// Match an ASCII-only word boundary. That is, this matches a position
    /// where the left adjacent character and right adjacent character
    /// correspond to a word and non-word or a non-word and word character.
    Ascii,
    /// Match an ASCII-only negation of a word boundary.
    AsciiNegate,
}
impl WordBoundary {
    /// Returns true if and only if this word boundary assertion is negated.
    pub fn is_negated(&self) -> bool {
        match *self {
            WordBoundary::Unicode | WordBoundary::Ascii => false,
            WordBoundary::UnicodeNegate | WordBoundary::AsciiNegate => true,
        }
    }
}
/// The high-level intermediate representation for a group.
///
/// This represents one of three possible group types:
///
/// 1. A non-capturing group (e.g., `(?:expr)`).
/// 2. A capturing group (e.g., `(expr)`).
/// 3. A named capturing group (e.g., `(?P<name>expr)`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Group {
    /// The kind of this group. If it is a capturing group, then the kind
    /// contains the capture group index (and the name, if it is a named
    /// group).
    pub kind: GroupKind,
    /// The expression inside the capturing group, which may be empty.
    pub hir: Box<Hir>,
}
/// The kind of group.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GroupKind {
    /// A normal unnamed capturing group.
    ///
    /// The value is the capture index of the group.
    CaptureIndex(u32),
    /// A named capturing group.
    CaptureName {
        /// The name of the group.
        name: String,
        /// The capture index of the group.
        index: u32,
    },
    /// A non-capturing group.
    NonCapturing,
}
/// The high-level intermediate representation of a repetition operator.
///
/// A repetition operator permits the repetition of an arbitrary
/// sub-expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Repetition {
    /// The kind of this repetition operator.
    pub kind: RepetitionKind,
    /// Whether this repetition operator is greedy or not. A greedy operator
    /// will match as much as it can. A non-greedy operator will match as
    /// little as it can.
    ///
    /// Typically, operators are greedy by default and are only non-greedy when
    /// a `?` suffix is used, e.g., `(expr)*` is greedy while `(expr)*?` is
    /// not. However, this can be inverted via the `U` "ungreedy" flag.
    pub greedy: bool,
    /// The expression being repeated.
    pub hir: Box<Hir>,
}
impl Repetition {
    /// Returns true if and only if this repetition operator makes it possible
    /// to match the empty string.
    ///
    /// Note that this is not defined inductively. For example, while `a*`
    /// will report `true`, `()+` will not, even though `()` matches the empty
    /// string and one or more occurrences of something that matches the empty
    /// string will always match the empty string. In order to get the
    /// inductive definition, see the corresponding method on
    /// [`Hir`](struct.Hir.html).
    pub fn is_match_empty(&self) -> bool {
        match self.kind {
            RepetitionKind::ZeroOrOne => true,
            RepetitionKind::ZeroOrMore => true,
            RepetitionKind::OneOrMore => false,
            RepetitionKind::Range(RepetitionRange::Exactly(m)) => m == 0,
            RepetitionKind::Range(RepetitionRange::AtLeast(m)) => m == 0,
            RepetitionKind::Range(RepetitionRange::Bounded(m, _)) => m == 0,
        }
    }
}
/// The kind of a repetition operator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RepetitionKind {
    /// Matches a sub-expression zero or one times.
    ZeroOrOne,
    /// Matches a sub-expression zero or more times.
    ZeroOrMore,
    /// Matches a sub-expression one or more times.
    OneOrMore,
    /// Matches a sub-expression within a bounded range of times.
    Range(RepetitionRange),
}
/// The kind of a counted repetition operator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RepetitionRange {
    /// Matches a sub-expression exactly this many times.
    Exactly(u32),
    /// Matches a sub-expression at least this many times.
    AtLeast(u32),
    /// Matches a sub-expression at least `m` times and at most `n` times.
    Bounded(u32, u32),
}
/// A custom `Drop` impl is used for `HirKind` such that it uses constant stack
/// space but heap space proportional to the depth of the total `Hir`.
impl Drop for Hir {
    fn drop(&mut self) {
        use std::mem;
        match *self.kind() {
            HirKind::Empty
            | HirKind::Literal(_)
            | HirKind::Class(_)
            | HirKind::Anchor(_)
            | HirKind::WordBoundary(_) => return,
            HirKind::Group(ref x) if !x.hir.kind.has_subexprs() => return,
            HirKind::Repetition(ref x) if !x.hir.kind.has_subexprs() => return,
            HirKind::Concat(ref x) if x.is_empty() => return,
            HirKind::Alternation(ref x) if x.is_empty() => return,
            _ => {}
        }
        let mut stack = vec![mem::replace(self, Hir::empty())];
        while let Some(mut expr) = stack.pop() {
            match expr.kind {
                HirKind::Empty
                | HirKind::Literal(_)
                | HirKind::Class(_)
                | HirKind::Anchor(_)
                | HirKind::WordBoundary(_) => {}
                HirKind::Group(ref mut x) => {
                    stack.push(mem::replace(&mut x.hir, Hir::empty()));
                }
                HirKind::Repetition(ref mut x) => {
                    stack.push(mem::replace(&mut x.hir, Hir::empty()));
                }
                HirKind::Concat(ref mut x) => {
                    stack.extend(x.drain(..));
                }
                HirKind::Alternation(ref mut x) => {
                    stack.extend(x.drain(..));
                }
            }
        }
    }
}
/// A type that documents various attributes of an HIR expression.
///
/// These attributes are typically defined inductively on the HIR.
#[derive(Clone, Debug, Eq, PartialEq)]
struct HirInfo {
    /// Represent yes/no questions by a bitfield to conserve space, since
    /// this is included in every HIR expression.
    ///
    /// If more attributes need to be added, it is OK to increase the size of
    /// this as appropriate.
    bools: u16,
}
macro_rules! define_bool {
    ($bit:expr, $is_fn_name:ident, $set_fn_name:ident) => {
        fn $is_fn_name (& self) -> bool { self.bools & (0b1 << $bit) > 0 } fn
        $set_fn_name (& mut self, yes : bool) { if yes { self.bools |= 1 << $bit; } else
        { self.bools &= ! (1 << $bit); } }
    };
}
impl HirInfo {
    fn new() -> HirInfo {
        HirInfo { bools: 0 }
    }
    define_bool!(0, is_always_utf8, set_always_utf8);
    define_bool!(1, is_all_assertions, set_all_assertions);
    define_bool!(2, is_anchored_start, set_anchored_start);
    define_bool!(3, is_anchored_end, set_anchored_end);
    define_bool!(4, is_line_anchored_start, set_line_anchored_start);
    define_bool!(5, is_line_anchored_end, set_line_anchored_end);
    define_bool!(6, is_any_anchored_start, set_any_anchored_start);
    define_bool!(7, is_any_anchored_end, set_any_anchored_end);
    define_bool!(8, is_match_empty, set_match_empty);
    define_bool!(9, is_literal, set_literal);
    define_bool!(10, is_alternation_literal, set_alternation_literal);
}
#[cfg(test)]
mod tests {
    use super::*;
    fn uclass(ranges: &[(char, char)]) -> ClassUnicode {
        let ranges: Vec<ClassUnicodeRange> = ranges
            .iter()
            .map(|&(s, e)| ClassUnicodeRange::new(s, e))
            .collect();
        ClassUnicode::new(ranges)
    }
    fn bclass(ranges: &[(u8, u8)]) -> ClassBytes {
        let ranges: Vec<ClassBytesRange> = ranges
            .iter()
            .map(|&(s, e)| ClassBytesRange::new(s, e))
            .collect();
        ClassBytes::new(ranges)
    }
    fn uranges(cls: &ClassUnicode) -> Vec<(char, char)> {
        cls.iter().map(|x| (x.start(), x.end())).collect()
    }
    #[cfg(feature = "unicode-case")]
    fn ucasefold(cls: &ClassUnicode) -> ClassUnicode {
        let mut cls_ = cls.clone();
        cls_.case_fold_simple();
        cls_
    }
    fn uunion(cls1: &ClassUnicode, cls2: &ClassUnicode) -> ClassUnicode {
        let mut cls_ = cls1.clone();
        cls_.union(cls2);
        cls_
    }
    fn uintersect(cls1: &ClassUnicode, cls2: &ClassUnicode) -> ClassUnicode {
        let mut cls_ = cls1.clone();
        cls_.intersect(cls2);
        cls_
    }
    fn udifference(cls1: &ClassUnicode, cls2: &ClassUnicode) -> ClassUnicode {
        let mut cls_ = cls1.clone();
        cls_.difference(cls2);
        cls_
    }
    fn usymdifference(cls1: &ClassUnicode, cls2: &ClassUnicode) -> ClassUnicode {
        let mut cls_ = cls1.clone();
        cls_.symmetric_difference(cls2);
        cls_
    }
    fn unegate(cls: &ClassUnicode) -> ClassUnicode {
        let mut cls_ = cls.clone();
        cls_.negate();
        cls_
    }
    fn branges(cls: &ClassBytes) -> Vec<(u8, u8)> {
        cls.iter().map(|x| (x.start(), x.end())).collect()
    }
    fn bcasefold(cls: &ClassBytes) -> ClassBytes {
        let mut cls_ = cls.clone();
        cls_.case_fold_simple();
        cls_
    }
    fn bunion(cls1: &ClassBytes, cls2: &ClassBytes) -> ClassBytes {
        let mut cls_ = cls1.clone();
        cls_.union(cls2);
        cls_
    }
    fn bintersect(cls1: &ClassBytes, cls2: &ClassBytes) -> ClassBytes {
        let mut cls_ = cls1.clone();
        cls_.intersect(cls2);
        cls_
    }
    fn bdifference(cls1: &ClassBytes, cls2: &ClassBytes) -> ClassBytes {
        let mut cls_ = cls1.clone();
        cls_.difference(cls2);
        cls_
    }
    fn bsymdifference(cls1: &ClassBytes, cls2: &ClassBytes) -> ClassBytes {
        let mut cls_ = cls1.clone();
        cls_.symmetric_difference(cls2);
        cls_
    }
    fn bnegate(cls: &ClassBytes) -> ClassBytes {
        let mut cls_ = cls.clone();
        cls_.negate();
        cls_
    }
    #[test]
    fn class_range_canonical_unicode() {
        let range = ClassUnicodeRange::new('\u{00FF}', '\0');
        assert_eq!('\0', range.start());
        assert_eq!('\u{00FF}', range.end());
    }
    #[test]
    fn class_range_canonical_bytes() {
        let range = ClassBytesRange::new(b'\xFF', b'\0');
        assert_eq!(b'\0', range.start());
        assert_eq!(b'\xFF', range.end());
    }
    #[test]
    fn class_canonicalize_unicode() {
        let cls = uclass(&[('a', 'c'), ('x', 'z')]);
        let expected = vec![('a', 'c'), ('x', 'z')];
        assert_eq!(expected, uranges(& cls));
        let cls = uclass(&[('x', 'z'), ('a', 'c')]);
        let expected = vec![('a', 'c'), ('x', 'z')];
        assert_eq!(expected, uranges(& cls));
        let cls = uclass(&[('x', 'z'), ('w', 'y')]);
        let expected = vec![('w', 'z')];
        assert_eq!(expected, uranges(& cls));
        let cls = uclass(
            &[('c', 'f'), ('a', 'g'), ('d', 'j'), ('a', 'c'), ('m', 'p'), ('l', 's')],
        );
        let expected = vec![('a', 'j'), ('l', 's')];
        assert_eq!(expected, uranges(& cls));
        let cls = uclass(&[('x', 'z'), ('u', 'w')]);
        let expected = vec![('u', 'z')];
        assert_eq!(expected, uranges(& cls));
        let cls = uclass(&[('\x00', '\u{10FFFF}'), ('\x00', '\u{10FFFF}')]);
        let expected = vec![('\x00', '\u{10FFFF}')];
        assert_eq!(expected, uranges(& cls));
        let cls = uclass(&[('a', 'a'), ('b', 'b')]);
        let expected = vec![('a', 'b')];
        assert_eq!(expected, uranges(& cls));
    }
    #[test]
    fn class_canonicalize_bytes() {
        let cls = bclass(&[(b'a', b'c'), (b'x', b'z')]);
        let expected = vec![(b'a', b'c'), (b'x', b'z')];
        assert_eq!(expected, branges(& cls));
        let cls = bclass(&[(b'x', b'z'), (b'a', b'c')]);
        let expected = vec![(b'a', b'c'), (b'x', b'z')];
        assert_eq!(expected, branges(& cls));
        let cls = bclass(&[(b'x', b'z'), (b'w', b'y')]);
        let expected = vec![(b'w', b'z')];
        assert_eq!(expected, branges(& cls));
        let cls = bclass(
            &[
                (b'c', b'f'),
                (b'a', b'g'),
                (b'd', b'j'),
                (b'a', b'c'),
                (b'm', b'p'),
                (b'l', b's'),
            ],
        );
        let expected = vec![(b'a', b'j'), (b'l', b's')];
        assert_eq!(expected, branges(& cls));
        let cls = bclass(&[(b'x', b'z'), (b'u', b'w')]);
        let expected = vec![(b'u', b'z')];
        assert_eq!(expected, branges(& cls));
        let cls = bclass(&[(b'\x00', b'\xFF'), (b'\x00', b'\xFF')]);
        let expected = vec![(b'\x00', b'\xFF')];
        assert_eq!(expected, branges(& cls));
        let cls = bclass(&[(b'a', b'a'), (b'b', b'b')]);
        let expected = vec![(b'a', b'b')];
        assert_eq!(expected, branges(& cls));
    }
    #[test]
    #[cfg(feature = "unicode-case")]
    fn class_case_fold_unicode() {
        let cls = uclass(
            &[
                ('C', 'F'),
                ('A', 'G'),
                ('D', 'J'),
                ('A', 'C'),
                ('M', 'P'),
                ('L', 'S'),
                ('c', 'f'),
            ],
        );
        let expected = uclass(
            &[('A', 'J'), ('L', 'S'), ('a', 'j'), ('l', 's'), ('\u{17F}', '\u{17F}')],
        );
        assert_eq!(expected, ucasefold(& cls));
        let cls = uclass(&[('A', 'Z')]);
        let expected = uclass(
            &[('A', 'Z'), ('a', 'z'), ('\u{17F}', '\u{17F}'), ('\u{212A}', '\u{212A}')],
        );
        assert_eq!(expected, ucasefold(& cls));
        let cls = uclass(&[('a', 'z')]);
        let expected = uclass(
            &[('A', 'Z'), ('a', 'z'), ('\u{17F}', '\u{17F}'), ('\u{212A}', '\u{212A}')],
        );
        assert_eq!(expected, ucasefold(& cls));
        let cls = uclass(&[('A', 'A'), ('_', '_')]);
        let expected = uclass(&[('A', 'A'), ('_', '_'), ('a', 'a')]);
        assert_eq!(expected, ucasefold(& cls));
        let cls = uclass(&[('A', 'A'), ('=', '=')]);
        let expected = uclass(&[('=', '='), ('A', 'A'), ('a', 'a')]);
        assert_eq!(expected, ucasefold(& cls));
        let cls = uclass(&[('\x00', '\x10')]);
        assert_eq!(cls, ucasefold(& cls));
        let cls = uclass(&[('k', 'k')]);
        let expected = uclass(&[('K', 'K'), ('k', 'k'), ('\u{212A}', '\u{212A}')]);
        assert_eq!(expected, ucasefold(& cls));
        let cls = uclass(&[('@', '@')]);
        assert_eq!(cls, ucasefold(& cls));
    }
    #[test]
    #[cfg(not(feature = "unicode-case"))]
    fn class_case_fold_unicode_disabled() {
        let mut cls = uclass(
            &[
                ('C', 'F'),
                ('A', 'G'),
                ('D', 'J'),
                ('A', 'C'),
                ('M', 'P'),
                ('L', 'S'),
                ('c', 'f'),
            ],
        );
        assert!(cls.try_case_fold_simple().is_err());
    }
    #[test]
    #[should_panic]
    #[cfg(not(feature = "unicode-case"))]
    fn class_case_fold_unicode_disabled_panics() {
        let mut cls = uclass(
            &[
                ('C', 'F'),
                ('A', 'G'),
                ('D', 'J'),
                ('A', 'C'),
                ('M', 'P'),
                ('L', 'S'),
                ('c', 'f'),
            ],
        );
        cls.case_fold_simple();
    }
    #[test]
    fn class_case_fold_bytes() {
        let cls = bclass(
            &[
                (b'C', b'F'),
                (b'A', b'G'),
                (b'D', b'J'),
                (b'A', b'C'),
                (b'M', b'P'),
                (b'L', b'S'),
                (b'c', b'f'),
            ],
        );
        let expected = bclass(&[(b'A', b'J'), (b'L', b'S'), (b'a', b'j'), (b'l', b's')]);
        assert_eq!(expected, bcasefold(& cls));
        let cls = bclass(&[(b'A', b'Z')]);
        let expected = bclass(&[(b'A', b'Z'), (b'a', b'z')]);
        assert_eq!(expected, bcasefold(& cls));
        let cls = bclass(&[(b'a', b'z')]);
        let expected = bclass(&[(b'A', b'Z'), (b'a', b'z')]);
        assert_eq!(expected, bcasefold(& cls));
        let cls = bclass(&[(b'A', b'A'), (b'_', b'_')]);
        let expected = bclass(&[(b'A', b'A'), (b'_', b'_'), (b'a', b'a')]);
        assert_eq!(expected, bcasefold(& cls));
        let cls = bclass(&[(b'A', b'A'), (b'=', b'=')]);
        let expected = bclass(&[(b'=', b'='), (b'A', b'A'), (b'a', b'a')]);
        assert_eq!(expected, bcasefold(& cls));
        let cls = bclass(&[(b'\x00', b'\x10')]);
        assert_eq!(cls, bcasefold(& cls));
        let cls = bclass(&[(b'k', b'k')]);
        let expected = bclass(&[(b'K', b'K'), (b'k', b'k')]);
        assert_eq!(expected, bcasefold(& cls));
        let cls = bclass(&[(b'@', b'@')]);
        assert_eq!(cls, bcasefold(& cls));
    }
    #[test]
    fn class_negate_unicode() {
        let cls = uclass(&[('a', 'a')]);
        let expected = uclass(&[('\x00', '\x60'), ('\x62', '\u{10FFFF}')]);
        assert_eq!(expected, unegate(& cls));
        let cls = uclass(&[('a', 'a'), ('b', 'b')]);
        let expected = uclass(&[('\x00', '\x60'), ('\x63', '\u{10FFFF}')]);
        assert_eq!(expected, unegate(& cls));
        let cls = uclass(&[('a', 'c'), ('x', 'z')]);
        let expected = uclass(
            &[('\x00', '\x60'), ('\x64', '\x77'), ('\x7B', '\u{10FFFF}')],
        );
        assert_eq!(expected, unegate(& cls));
        let cls = uclass(&[('\x00', 'a')]);
        let expected = uclass(&[('\x62', '\u{10FFFF}')]);
        assert_eq!(expected, unegate(& cls));
        let cls = uclass(&[('a', '\u{10FFFF}')]);
        let expected = uclass(&[('\x00', '\x60')]);
        assert_eq!(expected, unegate(& cls));
        let cls = uclass(&[('\x00', '\u{10FFFF}')]);
        let expected = uclass(&[]);
        assert_eq!(expected, unegate(& cls));
        let cls = uclass(&[]);
        let expected = uclass(&[('\x00', '\u{10FFFF}')]);
        assert_eq!(expected, unegate(& cls));
        let cls = uclass(&[('\x00', '\u{10FFFD}'), ('\u{10FFFF}', '\u{10FFFF}')]);
        let expected = uclass(&[('\u{10FFFE}', '\u{10FFFE}')]);
        assert_eq!(expected, unegate(& cls));
        let cls = uclass(&[('\x00', '\u{D7FF}')]);
        let expected = uclass(&[('\u{E000}', '\u{10FFFF}')]);
        assert_eq!(expected, unegate(& cls));
        let cls = uclass(&[('\x00', '\u{D7FE}')]);
        let expected = uclass(&[('\u{D7FF}', '\u{10FFFF}')]);
        assert_eq!(expected, unegate(& cls));
        let cls = uclass(&[('\u{E000}', '\u{10FFFF}')]);
        let expected = uclass(&[('\x00', '\u{D7FF}')]);
        assert_eq!(expected, unegate(& cls));
        let cls = uclass(&[('\u{E001}', '\u{10FFFF}')]);
        let expected = uclass(&[('\x00', '\u{E000}')]);
        assert_eq!(expected, unegate(& cls));
    }
    #[test]
    fn class_negate_bytes() {
        let cls = bclass(&[(b'a', b'a')]);
        let expected = bclass(&[(b'\x00', b'\x60'), (b'\x62', b'\xFF')]);
        assert_eq!(expected, bnegate(& cls));
        let cls = bclass(&[(b'a', b'a'), (b'b', b'b')]);
        let expected = bclass(&[(b'\x00', b'\x60'), (b'\x63', b'\xFF')]);
        assert_eq!(expected, bnegate(& cls));
        let cls = bclass(&[(b'a', b'c'), (b'x', b'z')]);
        let expected = bclass(
            &[(b'\x00', b'\x60'), (b'\x64', b'\x77'), (b'\x7B', b'\xFF')],
        );
        assert_eq!(expected, bnegate(& cls));
        let cls = bclass(&[(b'\x00', b'a')]);
        let expected = bclass(&[(b'\x62', b'\xFF')]);
        assert_eq!(expected, bnegate(& cls));
        let cls = bclass(&[(b'a', b'\xFF')]);
        let expected = bclass(&[(b'\x00', b'\x60')]);
        assert_eq!(expected, bnegate(& cls));
        let cls = bclass(&[(b'\x00', b'\xFF')]);
        let expected = bclass(&[]);
        assert_eq!(expected, bnegate(& cls));
        let cls = bclass(&[]);
        let expected = bclass(&[(b'\x00', b'\xFF')]);
        assert_eq!(expected, bnegate(& cls));
        let cls = bclass(&[(b'\x00', b'\xFD'), (b'\xFF', b'\xFF')]);
        let expected = bclass(&[(b'\xFE', b'\xFE')]);
        assert_eq!(expected, bnegate(& cls));
    }
    #[test]
    fn class_union_unicode() {
        let cls1 = uclass(&[('a', 'g'), ('m', 't'), ('A', 'C')]);
        let cls2 = uclass(&[('a', 'z')]);
        let expected = uclass(&[('a', 'z'), ('A', 'C')]);
        assert_eq!(expected, uunion(& cls1, & cls2));
    }
    #[test]
    fn class_union_bytes() {
        let cls1 = bclass(&[(b'a', b'g'), (b'm', b't'), (b'A', b'C')]);
        let cls2 = bclass(&[(b'a', b'z')]);
        let expected = bclass(&[(b'a', b'z'), (b'A', b'C')]);
        assert_eq!(expected, bunion(& cls1, & cls2));
    }
    #[test]
    fn class_intersect_unicode() {
        let cls1 = uclass(&[]);
        let cls2 = uclass(&[('a', 'a')]);
        let expected = uclass(&[]);
        assert_eq!(expected, uintersect(& cls1, & cls2));
        let cls1 = uclass(&[('a', 'a')]);
        let cls2 = uclass(&[('a', 'a')]);
        let expected = uclass(&[('a', 'a')]);
        assert_eq!(expected, uintersect(& cls1, & cls2));
        let cls1 = uclass(&[('a', 'a')]);
        let cls2 = uclass(&[('b', 'b')]);
        let expected = uclass(&[]);
        assert_eq!(expected, uintersect(& cls1, & cls2));
        let cls1 = uclass(&[('a', 'a')]);
        let cls2 = uclass(&[('a', 'c')]);
        let expected = uclass(&[('a', 'a')]);
        assert_eq!(expected, uintersect(& cls1, & cls2));
        let cls1 = uclass(&[('a', 'b')]);
        let cls2 = uclass(&[('a', 'c')]);
        let expected = uclass(&[('a', 'b')]);
        assert_eq!(expected, uintersect(& cls1, & cls2));
        let cls1 = uclass(&[('a', 'b')]);
        let cls2 = uclass(&[('b', 'c')]);
        let expected = uclass(&[('b', 'b')]);
        assert_eq!(expected, uintersect(& cls1, & cls2));
        let cls1 = uclass(&[('a', 'b')]);
        let cls2 = uclass(&[('c', 'd')]);
        let expected = uclass(&[]);
        assert_eq!(expected, uintersect(& cls1, & cls2));
        let cls1 = uclass(&[('b', 'c')]);
        let cls2 = uclass(&[('a', 'd')]);
        let expected = uclass(&[('b', 'c')]);
        assert_eq!(expected, uintersect(& cls1, & cls2));
        let cls1 = uclass(&[('a', 'b'), ('d', 'e'), ('g', 'h')]);
        let cls2 = uclass(&[('a', 'h')]);
        let expected = uclass(&[('a', 'b'), ('d', 'e'), ('g', 'h')]);
        assert_eq!(expected, uintersect(& cls1, & cls2));
        let cls1 = uclass(&[('a', 'b'), ('d', 'e'), ('g', 'h')]);
        let cls2 = uclass(&[('a', 'b'), ('d', 'e'), ('g', 'h')]);
        let expected = uclass(&[('a', 'b'), ('d', 'e'), ('g', 'h')]);
        assert_eq!(expected, uintersect(& cls1, & cls2));
        let cls1 = uclass(&[('a', 'b'), ('g', 'h')]);
        let cls2 = uclass(&[('d', 'e'), ('k', 'l')]);
        let expected = uclass(&[]);
        assert_eq!(expected, uintersect(& cls1, & cls2));
        let cls1 = uclass(&[('a', 'b'), ('d', 'e'), ('g', 'h')]);
        let cls2 = uclass(&[('h', 'h')]);
        let expected = uclass(&[('h', 'h')]);
        assert_eq!(expected, uintersect(& cls1, & cls2));
        let cls1 = uclass(&[('a', 'b'), ('e', 'f'), ('i', 'j')]);
        let cls2 = uclass(&[('c', 'd'), ('g', 'h'), ('k', 'l')]);
        let expected = uclass(&[]);
        assert_eq!(expected, uintersect(& cls1, & cls2));
        let cls1 = uclass(&[('a', 'b'), ('c', 'd'), ('e', 'f')]);
        let cls2 = uclass(&[('b', 'c'), ('d', 'e'), ('f', 'g')]);
        let expected = uclass(&[('b', 'f')]);
        assert_eq!(expected, uintersect(& cls1, & cls2));
    }
    #[test]
    fn class_intersect_bytes() {
        let cls1 = bclass(&[]);
        let cls2 = bclass(&[(b'a', b'a')]);
        let expected = bclass(&[]);
        assert_eq!(expected, bintersect(& cls1, & cls2));
        let cls1 = bclass(&[(b'a', b'a')]);
        let cls2 = bclass(&[(b'a', b'a')]);
        let expected = bclass(&[(b'a', b'a')]);
        assert_eq!(expected, bintersect(& cls1, & cls2));
        let cls1 = bclass(&[(b'a', b'a')]);
        let cls2 = bclass(&[(b'b', b'b')]);
        let expected = bclass(&[]);
        assert_eq!(expected, bintersect(& cls1, & cls2));
        let cls1 = bclass(&[(b'a', b'a')]);
        let cls2 = bclass(&[(b'a', b'c')]);
        let expected = bclass(&[(b'a', b'a')]);
        assert_eq!(expected, bintersect(& cls1, & cls2));
        let cls1 = bclass(&[(b'a', b'b')]);
        let cls2 = bclass(&[(b'a', b'c')]);
        let expected = bclass(&[(b'a', b'b')]);
        assert_eq!(expected, bintersect(& cls1, & cls2));
        let cls1 = bclass(&[(b'a', b'b')]);
        let cls2 = bclass(&[(b'b', b'c')]);
        let expected = bclass(&[(b'b', b'b')]);
        assert_eq!(expected, bintersect(& cls1, & cls2));
        let cls1 = bclass(&[(b'a', b'b')]);
        let cls2 = bclass(&[(b'c', b'd')]);
        let expected = bclass(&[]);
        assert_eq!(expected, bintersect(& cls1, & cls2));
        let cls1 = bclass(&[(b'b', b'c')]);
        let cls2 = bclass(&[(b'a', b'd')]);
        let expected = bclass(&[(b'b', b'c')]);
        assert_eq!(expected, bintersect(& cls1, & cls2));
        let cls1 = bclass(&[(b'a', b'b'), (b'd', b'e'), (b'g', b'h')]);
        let cls2 = bclass(&[(b'a', b'h')]);
        let expected = bclass(&[(b'a', b'b'), (b'd', b'e'), (b'g', b'h')]);
        assert_eq!(expected, bintersect(& cls1, & cls2));
        let cls1 = bclass(&[(b'a', b'b'), (b'd', b'e'), (b'g', b'h')]);
        let cls2 = bclass(&[(b'a', b'b'), (b'd', b'e'), (b'g', b'h')]);
        let expected = bclass(&[(b'a', b'b'), (b'd', b'e'), (b'g', b'h')]);
        assert_eq!(expected, bintersect(& cls1, & cls2));
        let cls1 = bclass(&[(b'a', b'b'), (b'g', b'h')]);
        let cls2 = bclass(&[(b'd', b'e'), (b'k', b'l')]);
        let expected = bclass(&[]);
        assert_eq!(expected, bintersect(& cls1, & cls2));
        let cls1 = bclass(&[(b'a', b'b'), (b'd', b'e'), (b'g', b'h')]);
        let cls2 = bclass(&[(b'h', b'h')]);
        let expected = bclass(&[(b'h', b'h')]);
        assert_eq!(expected, bintersect(& cls1, & cls2));
        let cls1 = bclass(&[(b'a', b'b'), (b'e', b'f'), (b'i', b'j')]);
        let cls2 = bclass(&[(b'c', b'd'), (b'g', b'h'), (b'k', b'l')]);
        let expected = bclass(&[]);
        assert_eq!(expected, bintersect(& cls1, & cls2));
        let cls1 = bclass(&[(b'a', b'b'), (b'c', b'd'), (b'e', b'f')]);
        let cls2 = bclass(&[(b'b', b'c'), (b'd', b'e'), (b'f', b'g')]);
        let expected = bclass(&[(b'b', b'f')]);
        assert_eq!(expected, bintersect(& cls1, & cls2));
    }
    #[test]
    fn class_difference_unicode() {
        let cls1 = uclass(&[('a', 'a')]);
        let cls2 = uclass(&[('a', 'a')]);
        let expected = uclass(&[]);
        assert_eq!(expected, udifference(& cls1, & cls2));
        let cls1 = uclass(&[('a', 'a')]);
        let cls2 = uclass(&[]);
        let expected = uclass(&[('a', 'a')]);
        assert_eq!(expected, udifference(& cls1, & cls2));
        let cls1 = uclass(&[]);
        let cls2 = uclass(&[('a', 'a')]);
        let expected = uclass(&[]);
        assert_eq!(expected, udifference(& cls1, & cls2));
        let cls1 = uclass(&[('a', 'z')]);
        let cls2 = uclass(&[('a', 'a')]);
        let expected = uclass(&[('b', 'z')]);
        assert_eq!(expected, udifference(& cls1, & cls2));
        let cls1 = uclass(&[('a', 'z')]);
        let cls2 = uclass(&[('z', 'z')]);
        let expected = uclass(&[('a', 'y')]);
        assert_eq!(expected, udifference(& cls1, & cls2));
        let cls1 = uclass(&[('a', 'z')]);
        let cls2 = uclass(&[('m', 'm')]);
        let expected = uclass(&[('a', 'l'), ('n', 'z')]);
        assert_eq!(expected, udifference(& cls1, & cls2));
        let cls1 = uclass(&[('a', 'c'), ('g', 'i'), ('r', 't')]);
        let cls2 = uclass(&[('a', 'z')]);
        let expected = uclass(&[]);
        assert_eq!(expected, udifference(& cls1, & cls2));
        let cls1 = uclass(&[('a', 'c'), ('g', 'i'), ('r', 't')]);
        let cls2 = uclass(&[('d', 'v')]);
        let expected = uclass(&[('a', 'c')]);
        assert_eq!(expected, udifference(& cls1, & cls2));
        let cls1 = uclass(&[('a', 'c'), ('g', 'i'), ('r', 't')]);
        let cls2 = uclass(&[('b', 'g'), ('s', 'u')]);
        let expected = uclass(&[('a', 'a'), ('h', 'i'), ('r', 'r')]);
        assert_eq!(expected, udifference(& cls1, & cls2));
        let cls1 = uclass(&[('a', 'c'), ('g', 'i'), ('r', 't')]);
        let cls2 = uclass(&[('b', 'd'), ('e', 'g'), ('s', 'u')]);
        let expected = uclass(&[('a', 'a'), ('h', 'i'), ('r', 'r')]);
        assert_eq!(expected, udifference(& cls1, & cls2));
        let cls1 = uclass(&[('x', 'z')]);
        let cls2 = uclass(&[('a', 'c'), ('e', 'g'), ('s', 'u')]);
        let expected = uclass(&[('x', 'z')]);
        assert_eq!(expected, udifference(& cls1, & cls2));
        let cls1 = uclass(&[('a', 'z')]);
        let cls2 = uclass(&[('a', 'c'), ('e', 'g'), ('s', 'u')]);
        let expected = uclass(&[('d', 'd'), ('h', 'r'), ('v', 'z')]);
        assert_eq!(expected, udifference(& cls1, & cls2));
    }
    #[test]
    fn class_difference_bytes() {
        let cls1 = bclass(&[(b'a', b'a')]);
        let cls2 = bclass(&[(b'a', b'a')]);
        let expected = bclass(&[]);
        assert_eq!(expected, bdifference(& cls1, & cls2));
        let cls1 = bclass(&[(b'a', b'a')]);
        let cls2 = bclass(&[]);
        let expected = bclass(&[(b'a', b'a')]);
        assert_eq!(expected, bdifference(& cls1, & cls2));
        let cls1 = bclass(&[]);
        let cls2 = bclass(&[(b'a', b'a')]);
        let expected = bclass(&[]);
        assert_eq!(expected, bdifference(& cls1, & cls2));
        let cls1 = bclass(&[(b'a', b'z')]);
        let cls2 = bclass(&[(b'a', b'a')]);
        let expected = bclass(&[(b'b', b'z')]);
        assert_eq!(expected, bdifference(& cls1, & cls2));
        let cls1 = bclass(&[(b'a', b'z')]);
        let cls2 = bclass(&[(b'z', b'z')]);
        let expected = bclass(&[(b'a', b'y')]);
        assert_eq!(expected, bdifference(& cls1, & cls2));
        let cls1 = bclass(&[(b'a', b'z')]);
        let cls2 = bclass(&[(b'm', b'm')]);
        let expected = bclass(&[(b'a', b'l'), (b'n', b'z')]);
        assert_eq!(expected, bdifference(& cls1, & cls2));
        let cls1 = bclass(&[(b'a', b'c'), (b'g', b'i'), (b'r', b't')]);
        let cls2 = bclass(&[(b'a', b'z')]);
        let expected = bclass(&[]);
        assert_eq!(expected, bdifference(& cls1, & cls2));
        let cls1 = bclass(&[(b'a', b'c'), (b'g', b'i'), (b'r', b't')]);
        let cls2 = bclass(&[(b'd', b'v')]);
        let expected = bclass(&[(b'a', b'c')]);
        assert_eq!(expected, bdifference(& cls1, & cls2));
        let cls1 = bclass(&[(b'a', b'c'), (b'g', b'i'), (b'r', b't')]);
        let cls2 = bclass(&[(b'b', b'g'), (b's', b'u')]);
        let expected = bclass(&[(b'a', b'a'), (b'h', b'i'), (b'r', b'r')]);
        assert_eq!(expected, bdifference(& cls1, & cls2));
        let cls1 = bclass(&[(b'a', b'c'), (b'g', b'i'), (b'r', b't')]);
        let cls2 = bclass(&[(b'b', b'd'), (b'e', b'g'), (b's', b'u')]);
        let expected = bclass(&[(b'a', b'a'), (b'h', b'i'), (b'r', b'r')]);
        assert_eq!(expected, bdifference(& cls1, & cls2));
        let cls1 = bclass(&[(b'x', b'z')]);
        let cls2 = bclass(&[(b'a', b'c'), (b'e', b'g'), (b's', b'u')]);
        let expected = bclass(&[(b'x', b'z')]);
        assert_eq!(expected, bdifference(& cls1, & cls2));
        let cls1 = bclass(&[(b'a', b'z')]);
        let cls2 = bclass(&[(b'a', b'c'), (b'e', b'g'), (b's', b'u')]);
        let expected = bclass(&[(b'd', b'd'), (b'h', b'r'), (b'v', b'z')]);
        assert_eq!(expected, bdifference(& cls1, & cls2));
    }
    #[test]
    fn class_symmetric_difference_unicode() {
        let cls1 = uclass(&[('a', 'm')]);
        let cls2 = uclass(&[('g', 't')]);
        let expected = uclass(&[('a', 'f'), ('n', 't')]);
        assert_eq!(expected, usymdifference(& cls1, & cls2));
    }
    #[test]
    fn class_symmetric_difference_bytes() {
        let cls1 = bclass(&[(b'a', b'm')]);
        let cls2 = bclass(&[(b'g', b't')]);
        let expected = bclass(&[(b'a', b'f'), (b'n', b't')]);
        assert_eq!(expected, bsymdifference(& cls1, & cls2));
    }
    #[test]
    #[should_panic]
    fn hir_byte_literal_non_ascii() {
        Hir::literal(Literal::Byte(b'a'));
    }
    #[test]
    #[cfg(any(unix, windows))]
    fn no_stack_overflow_on_drop() {
        use std::thread;
        let run = || {
            let mut expr = Hir::empty();
            for _ in 0..100 {
                expr = Hir::group(Group {
                    kind: GroupKind::NonCapturing,
                    hir: Box::new(expr),
                });
                expr = Hir::repetition(Repetition {
                    kind: RepetitionKind::ZeroOrOne,
                    greedy: true,
                    hir: Box::new(expr),
                });
                expr = Hir {
                    kind: HirKind::Concat(vec![expr]),
                    info: HirInfo::new(),
                };
                expr = Hir {
                    kind: HirKind::Alternation(vec![expr]),
                    info: HirInfo::new(),
                };
            }
            assert!(! expr.kind.is_empty());
        };
        thread::Builder::new().stack_size(1 << 10).spawn(run).unwrap().join().unwrap();
    }
}
#[cfg(test)]
mod tests_llm_16_50 {
    use super::*;
    use crate::*;
    #[test]
    fn test_lower() {
        let _rug_st_tests_llm_16_50_rrrruuuugggg_test_lower = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 20;
        let range = ClassBytesRange::new(rug_fuzz_0, rug_fuzz_1);
        debug_assert_eq!(range.lower(), 10);
        let _rug_ed_tests_llm_16_50_rrrruuuugggg_test_lower = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_51 {
    use super::*;
    use crate::*;
    #[test]
    fn test_set_lower() {
        let _rug_st_tests_llm_16_51_rrrruuuugggg_test_set_lower = 0;
        let rug_fuzz_0 = b'a';
        let rug_fuzz_1 = b'z';
        let rug_fuzz_2 = b'c';
        let mut range = ClassBytesRange::new(rug_fuzz_0, rug_fuzz_1);
        range.set_lower(rug_fuzz_2);
        debug_assert_eq!(range.start(), b'c');
        let _rug_ed_tests_llm_16_51_rrrruuuugggg_test_set_lower = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_52 {
    use super::*;
    use crate::*;
    use crate::hir::interval::Interval;
    #[test]
    fn test_set_upper() {
        let _rug_st_tests_llm_16_52_rrrruuuugggg_test_set_upper = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 20;
        let rug_fuzz_2 = 30;
        let mut range = ClassBytesRange::new(rug_fuzz_0, rug_fuzz_1);
        range.set_upper(rug_fuzz_2);
        debug_assert_eq!(range.end(), 30);
        let _rug_ed_tests_llm_16_52_rrrruuuugggg_test_set_upper = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_54 {
    use crate::hir::ClassBytesRange;
    use crate::hir::interval::Interval;
    #[test]
    fn test_upper() {
        let _rug_st_tests_llm_16_54_rrrruuuugggg_test_upper = 0;
        let rug_fuzz_0 = 'a';
        let rug_fuzz_1 = 'f';
        let rug_fuzz_2 = 'A';
        let rug_fuzz_3 = 'Z';
        let range = ClassBytesRange::new(rug_fuzz_0 as u8, rug_fuzz_1 as u8);
        debug_assert_eq!(< ClassBytesRange as Interval > ::upper(& range), 'f' as u8);
        let range = ClassBytesRange::new(rug_fuzz_2 as u8, rug_fuzz_3 as u8);
        debug_assert_eq!(< ClassBytesRange as Interval > ::upper(& range), 'Z' as u8);
        let _rug_ed_tests_llm_16_54_rrrruuuugggg_test_upper = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_59 {
    use super::*;
    use crate::*;
    #[test]
    fn test_lower() {
        let _rug_st_tests_llm_16_59_rrrruuuugggg_test_lower = 0;
        let rug_fuzz_0 = 'a';
        let rug_fuzz_1 = 'z';
        let range = ClassUnicodeRange::new(rug_fuzz_0, rug_fuzz_1);
        debug_assert_eq!(range.lower(), 'a');
        let _rug_ed_tests_llm_16_59_rrrruuuugggg_test_lower = 0;
    }
}
mod tests_llm_16_60 {
    use super::*;
    use crate::*;
    #[test]
    fn test_set_lower() {
        let _rug_st_tests_llm_16_60_rrrruuuugggg_test_set_lower = 0;
        let rug_fuzz_0 = 'a';
        let rug_fuzz_1 = 'z';
        let rug_fuzz_2 = 'b';
        let mut range = ClassUnicodeRange::new(rug_fuzz_0, rug_fuzz_1);
        range.set_lower(rug_fuzz_2);
        debug_assert_eq!(range.start(), 'b');
        let _rug_ed_tests_llm_16_60_rrrruuuugggg_test_set_lower = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_61 {
    use super::*;
    use crate::*;
    #[test]
    fn test_set_upper() {
        let _rug_st_tests_llm_16_61_rrrruuuugggg_test_set_upper = 0;
        let rug_fuzz_0 = 'a';
        let rug_fuzz_1 = 'z';
        let rug_fuzz_2 = 'D';
        let mut range = ClassUnicodeRange::new(rug_fuzz_0, rug_fuzz_1);
        range.set_upper(rug_fuzz_2);
        debug_assert_eq!(range.end(), 'D');
        let _rug_ed_tests_llm_16_61_rrrruuuugggg_test_set_upper = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_62 {
    use super::*;
    use crate::*;
    use crate::hir::interval::Interval;
    #[test]
    fn test_upper() {
        let _rug_st_tests_llm_16_62_rrrruuuugggg_test_upper = 0;
        let rug_fuzz_0 = 'A';
        let rug_fuzz_1 = 'Z';
        let range = ClassUnicodeRange::new(rug_fuzz_0, rug_fuzz_1);
        debug_assert_eq!(range.upper(), 'Z');
        let _rug_ed_tests_llm_16_62_rrrruuuugggg_test_upper = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_66 {
    use super::*;
    use crate::*;
    use crate::*;
    #[test]
    fn test_drop() {
        let _rug_st_tests_llm_16_66_rrrruuuugggg_test_drop = 0;
        let rug_fuzz_0 = 'a';
        let mut hir = Hir::literal(Literal::Unicode(rug_fuzz_0));
        drop(&mut hir);
        let _rug_ed_tests_llm_16_66_rrrruuuugggg_test_drop = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_298 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_always_utf8_unicode() {
        let _rug_st_tests_llm_16_298_rrrruuuugggg_test_is_always_utf8_unicode = 0;
        let rug_fuzz_0 = '\u{0000}';
        let rug_fuzz_1 = '\u{007F}';
        let class = Class::Unicode(
            ClassUnicode::new(vec![ClassUnicodeRange::new(rug_fuzz_0, rug_fuzz_1)]),
        );
        debug_assert_eq!(class.is_always_utf8(), true);
        let _rug_ed_tests_llm_16_298_rrrruuuugggg_test_is_always_utf8_unicode = 0;
    }
    #[test]
    fn test_is_always_utf8_bytes() {
        let _rug_st_tests_llm_16_298_rrrruuuugggg_test_is_always_utf8_bytes = 0;
        let rug_fuzz_0 = b'A';
        let rug_fuzz_1 = b'Z';
        let class = Class::Bytes(
            ClassBytes::new(
                vec![
                    ClassBytesRange::new(rug_fuzz_0, rug_fuzz_1),
                    ClassBytesRange::new(b'a', b'z')
                ],
            ),
        );
        debug_assert_eq!(class.is_always_utf8(), false);
        let _rug_ed_tests_llm_16_298_rrrruuuugggg_test_is_always_utf8_bytes = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_305 {
    use super::*;
    use crate::*;
    #[test]
    fn test_empty() {
        let _rug_st_tests_llm_16_305_rrrruuuugggg_test_empty = 0;
        let class_bytes = ClassBytes::empty();
        debug_assert_eq!(class_bytes.ranges().len(), 0);
        let _rug_ed_tests_llm_16_305_rrrruuuugggg_test_empty = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_309 {
    use super::*;
    use crate::*;
    use hir::{ClassBytes, ClassBytesRange};
    #[test]
    fn test_is_all_ascii() {
        let _rug_st_tests_llm_16_309_rrrruuuugggg_test_is_all_ascii = 0;
        let rug_fuzz_0 = b'a';
        let rug_fuzz_1 = b'z';
        let rug_fuzz_2 = b'A';
        let rug_fuzz_3 = b'Z';
        let rug_fuzz_4 = 0x80;
        let rug_fuzz_5 = 0xFF;
        let empty_class = ClassBytes::empty();
        debug_assert_eq!(empty_class.is_all_ascii(), true);
        let mut class = ClassBytes::new(
            vec![ClassBytesRange::new(rug_fuzz_0, rug_fuzz_1)],
        );
        debug_assert_eq!(class.is_all_ascii(), true);
        class.push(ClassBytesRange::new(rug_fuzz_2, rug_fuzz_3));
        debug_assert_eq!(class.is_all_ascii(), true);
        class.push(ClassBytesRange::new(rug_fuzz_4, rug_fuzz_5));
        debug_assert_eq!(class.is_all_ascii(), false);
        let _rug_ed_tests_llm_16_309_rrrruuuugggg_test_is_all_ascii = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_310 {
    use super::*;
    use crate::*;
    #[test]
    fn test_iter() {
        let _rug_st_tests_llm_16_310_rrrruuuugggg_test_iter = 0;
        let rug_fuzz_0 = b'a';
        let rug_fuzz_1 = b'z';
        let class = ClassBytes::new(
            vec![
                ClassBytesRange::new(rug_fuzz_0, rug_fuzz_1), ClassBytesRange::new(b'A',
                b'Z')
            ],
        );
        let mut iter = class.iter();
        debug_assert_eq!(iter.next(), Some(& ClassBytesRange::new(b'a', b'z')));
        debug_assert_eq!(iter.next(), Some(& ClassBytesRange::new(b'A', b'Z')));
        debug_assert_eq!(iter.next(), None);
        let _rug_ed_tests_llm_16_310_rrrruuuugggg_test_iter = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_312 {
    use super::*;
    use crate::*;
    use hir::interval::IntervalSet;
    #[test]
    fn test_negate() {
        let _rug_st_tests_llm_16_312_rrrruuuugggg_test_negate = 0;
        let rug_fuzz_0 = b'a';
        let rug_fuzz_1 = b'z';
        let rug_fuzz_2 = b'@';
        let ranges = vec![
            ClassBytesRange::new(rug_fuzz_0, rug_fuzz_1), ClassBytesRange::new(b'0',
            b'9')
        ];
        let mut class_bytes = ClassBytes::new(ranges);
        class_bytes.negate();
        let expected_ranges = vec![
            ClassBytesRange::new(std::u8::MIN, rug_fuzz_2), ClassBytesRange::new(b'[',
            b'`'), ClassBytesRange::new(b'{', std::u8::MAX)
        ];
        debug_assert_eq!(class_bytes.ranges(), expected_ranges.as_slice());
        let _rug_ed_tests_llm_16_312_rrrruuuugggg_test_negate = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_313 {
    use super::*;
    use crate::*;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_313_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = b'a';
        let rug_fuzz_1 = b'z';
        let ranges = vec![
            ClassBytesRange::new(rug_fuzz_0, rug_fuzz_1), ClassBytesRange::new(b'0',
            b'9')
        ];
        let result = ClassBytes::new(ranges);
        debug_assert_eq!(result.set.intervals().len(), 2);
        let _rug_ed_tests_llm_16_313_rrrruuuugggg_test_new = 0;
    }
    #[test]
    fn test_push() {
        let _rug_st_tests_llm_16_313_rrrruuuugggg_test_push = 0;
        let rug_fuzz_0 = b'a';
        let rug_fuzz_1 = b'z';
        let range = ClassBytesRange::new(rug_fuzz_0, rug_fuzz_1);
        let mut class_bytes = ClassBytes::empty();
        class_bytes.push(range);
        debug_assert_eq!(class_bytes.set.intervals().len(), 1);
        let _rug_ed_tests_llm_16_313_rrrruuuugggg_test_push = 0;
    }
    #[test]
    fn test_iter() {
        let _rug_st_tests_llm_16_313_rrrruuuugggg_test_iter = 0;
        let rug_fuzz_0 = b'a';
        let rug_fuzz_1 = b'z';
        let ranges = vec![
            ClassBytesRange::new(rug_fuzz_0, rug_fuzz_1), ClassBytesRange::new(b'0',
            b'9')
        ];
        let class_bytes = ClassBytes::new(ranges);
        let mut iter = class_bytes.iter();
        debug_assert_eq!(iter.next().unwrap().lower(), b'0');
        debug_assert_eq!(iter.next().unwrap().lower(), b'a');
        debug_assert_eq!(iter.next().unwrap().lower(), b'z');
        debug_assert_eq!(iter.next(), None);
        let _rug_ed_tests_llm_16_313_rrrruuuugggg_test_iter = 0;
    }
    #[test]
    fn test_ranges() {
        let _rug_st_tests_llm_16_313_rrrruuuugggg_test_ranges = 0;
        let rug_fuzz_0 = b'a';
        let rug_fuzz_1 = b'z';
        let ranges = vec![
            ClassBytesRange::new(rug_fuzz_0, rug_fuzz_1), ClassBytesRange::new(b'0',
            b'9')
        ];
        let class_bytes = ClassBytes::new(ranges);
        debug_assert_eq!(class_bytes.ranges().len(), 2);
        let _rug_ed_tests_llm_16_313_rrrruuuugggg_test_ranges = 0;
    }
    #[test]
    fn test_case_fold_simple() {
        let _rug_st_tests_llm_16_313_rrrruuuugggg_test_case_fold_simple = 0;
        let rug_fuzz_0 = b'a';
        let rug_fuzz_1 = b'z';
        let ranges = vec![ClassBytesRange::new(rug_fuzz_0, rug_fuzz_1)];
        let mut class_bytes = ClassBytes::new(ranges);
        class_bytes.case_fold_simple();
        debug_assert_eq!(class_bytes.set.intervals().len(), 2);
        let _rug_ed_tests_llm_16_313_rrrruuuugggg_test_case_fold_simple = 0;
    }
    #[test]
    fn test_negate() {
        let _rug_st_tests_llm_16_313_rrrruuuugggg_test_negate = 0;
        let rug_fuzz_0 = b'a';
        let rug_fuzz_1 = b'b';
        let ranges = vec![ClassBytesRange::new(rug_fuzz_0, rug_fuzz_1)];
        let mut class_bytes = ClassBytes::new(ranges);
        class_bytes.negate();
        debug_assert_eq!(class_bytes.set.intervals().len(), 2);
        let _rug_ed_tests_llm_16_313_rrrruuuugggg_test_negate = 0;
    }
    #[test]
    fn test_union() {
        let _rug_st_tests_llm_16_313_rrrruuuugggg_test_union = 0;
        let rug_fuzz_0 = b'a';
        let rug_fuzz_1 = b'z';
        let rug_fuzz_2 = b'0';
        let rug_fuzz_3 = b'9';
        let ranges1 = vec![ClassBytesRange::new(rug_fuzz_0, rug_fuzz_1)];
        let ranges2 = vec![ClassBytesRange::new(rug_fuzz_2, rug_fuzz_3)];
        let class_bytes1 = ClassBytes::new(ranges1);
        let class_bytes2 = ClassBytes::new(ranges2);
        let mut class_bytes = class_bytes1.clone();
        class_bytes.union(&class_bytes2);
        debug_assert_eq!(class_bytes.set.intervals().len(), 2);
        let _rug_ed_tests_llm_16_313_rrrruuuugggg_test_union = 0;
    }
    #[test]
    fn test_intersect() {
        let _rug_st_tests_llm_16_313_rrrruuuugggg_test_intersect = 0;
        let rug_fuzz_0 = b'a';
        let rug_fuzz_1 = b'z';
        let rug_fuzz_2 = b'0';
        let rug_fuzz_3 = b'9';
        let ranges1 = vec![
            ClassBytesRange::new(rug_fuzz_0, rug_fuzz_1), ClassBytesRange::new(b'0',
            b'9')
        ];
        let ranges2 = vec![ClassBytesRange::new(rug_fuzz_2, rug_fuzz_3)];
        let class_bytes1 = ClassBytes::new(ranges1);
        let class_bytes2 = ClassBytes::new(ranges2);
        let mut class_bytes = class_bytes1.clone();
        class_bytes.intersect(&class_bytes2);
        debug_assert_eq!(class_bytes.set.intervals().len(), 1);
        let _rug_ed_tests_llm_16_313_rrrruuuugggg_test_intersect = 0;
    }
    #[test]
    fn test_difference() {
        let _rug_st_tests_llm_16_313_rrrruuuugggg_test_difference = 0;
        let rug_fuzz_0 = b'a';
        let rug_fuzz_1 = b'z';
        let rug_fuzz_2 = b'0';
        let rug_fuzz_3 = b'9';
        let ranges1 = vec![
            ClassBytesRange::new(rug_fuzz_0, rug_fuzz_1), ClassBytesRange::new(b'0',
            b'9')
        ];
        let ranges2 = vec![ClassBytesRange::new(rug_fuzz_2, rug_fuzz_3)];
        let class_bytes1 = ClassBytes::new(ranges1);
        let class_bytes2 = ClassBytes::new(ranges2);
        let mut class_bytes = class_bytes1.clone();
        class_bytes.difference(&class_bytes2);
        debug_assert_eq!(class_bytes.set.intervals().len(), 1);
        let _rug_ed_tests_llm_16_313_rrrruuuugggg_test_difference = 0;
    }
    #[test]
    fn test_symmetric_difference() {
        let _rug_st_tests_llm_16_313_rrrruuuugggg_test_symmetric_difference = 0;
        let rug_fuzz_0 = b'a';
        let rug_fuzz_1 = b'z';
        let rug_fuzz_2 = b'0';
        let rug_fuzz_3 = b'9';
        let ranges1 = vec![ClassBytesRange::new(rug_fuzz_0, rug_fuzz_1)];
        let ranges2 = vec![ClassBytesRange::new(rug_fuzz_2, rug_fuzz_3)];
        let class_bytes1 = ClassBytes::new(ranges1);
        let class_bytes2 = ClassBytes::new(ranges2);
        let mut class_bytes = class_bytes1.clone();
        class_bytes.symmetric_difference(&class_bytes2);
        debug_assert_eq!(class_bytes.set.intervals().len(), 2);
        let _rug_ed_tests_llm_16_313_rrrruuuugggg_test_symmetric_difference = 0;
    }
    #[test]
    fn test_is_all_ascii() {
        let _rug_st_tests_llm_16_313_rrrruuuugggg_test_is_all_ascii = 0;
        let rug_fuzz_0 = b'a';
        let rug_fuzz_1 = b'z';
        let ranges1 = vec![ClassBytesRange::new(rug_fuzz_0, rug_fuzz_1)];
        let class_bytes1 = ClassBytes::new(ranges1);
        debug_assert_eq!(class_bytes1.is_all_ascii(), true);
        let _rug_ed_tests_llm_16_313_rrrruuuugggg_test_is_all_ascii = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_314 {
    use super::*;
    use crate::*;
    #[test]
    fn test_push() {
        let _rug_st_tests_llm_16_314_rrrruuuugggg_test_push = 0;
        let rug_fuzz_0 = 97;
        let rug_fuzz_1 = 122;
        let mut class_bytes = ClassBytes::empty();
        let range = ClassBytesRange::new(rug_fuzz_0, rug_fuzz_1);
        class_bytes.push(range);
        debug_assert_eq!(class_bytes.ranges(), & [ClassBytesRange::new(97, 122)]);
        let _rug_ed_tests_llm_16_314_rrrruuuugggg_test_push = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_315 {
    use crate::hir::ClassBytes;
    use crate::hir::ClassBytesRange;
    #[test]
    fn test_ranges() {
        let _rug_st_tests_llm_16_315_rrrruuuugggg_test_ranges = 0;
        let rug_fuzz_0 = 0x41;
        let rug_fuzz_1 = 0x5A;
        let rug_fuzz_2 = 0x61;
        let rug_fuzz_3 = 0x7A;
        let rug_fuzz_4 = 0x30;
        let rug_fuzz_5 = 0x39;
        let range1 = ClassBytesRange::new(rug_fuzz_0, rug_fuzz_1);
        let range2 = ClassBytesRange::new(rug_fuzz_2, rug_fuzz_3);
        let range3 = ClassBytesRange::new(rug_fuzz_4, rug_fuzz_5);
        let class_bytes = ClassBytes::new(vec![range1, range2, range3]);
        debug_assert_eq!(class_bytes.ranges(), & [range1, range2, range3]);
        let _rug_ed_tests_llm_16_315_rrrruuuugggg_test_ranges = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_316 {
    use super::*;
    use crate::*;
    fn create_class_bytes() -> ClassBytes {
        ClassBytes::new(
            vec![
                ClassBytesRange::create(0x41, 0x5A), ClassBytesRange::create(0x61, 0x7A),
            ],
        )
    }
    #[test]
    fn test_symmetric_difference() {
        let mut class_bytes1 = create_class_bytes();
        let class_bytes2 = create_class_bytes();
        class_bytes1.symmetric_difference(&class_bytes2);
        assert_eq!(
            class_bytes1.ranges(), & [ClassBytesRange::create(0xA, 0x40),
            ClassBytesRange::create(0x5B, 0x60), ClassBytesRange::create(0x7B, 0x7F)]
        );
    }
}
#[cfg(test)]
mod tests_llm_16_318 {
    use super::*;
    use crate::*;
    use crate::hir::interval::Interval;
    fn create_range(lower: u8, upper: u8) -> ClassBytesRange {
        ClassBytesRange::new(lower, upper)
    }
    #[test]
    fn test_union() {
        let mut class_bytes1 = ClassBytes::new(vec![create_range(b'a', b'z')]);
        let class_bytes2 = ClassBytes::new(vec![create_range(b'A', b'Z')]);
        class_bytes1.union(&class_bytes2);
        let expected = ClassBytes::new(
            vec![create_range(b'a', b'z'), create_range(b'A', b'Z')],
        );
        assert_eq!(class_bytes1, expected);
    }
}
#[cfg(test)]
mod tests_llm_16_319 {
    use crate::hir::{ClassBytesRange, Interval};
    use std::cmp;
    #[test]
    fn test_end() {
        let _rug_st_tests_llm_16_319_rrrruuuugggg_test_end = 0;
        let rug_fuzz_0 = b'a';
        let rug_fuzz_1 = b'z';
        let rug_fuzz_2 = b'A';
        let rug_fuzz_3 = b'Z';
        let rug_fuzz_4 = b'0';
        let rug_fuzz_5 = b'9';
        let range = ClassBytesRange::new(rug_fuzz_0, rug_fuzz_1);
        debug_assert_eq!(range.end(), b'z');
        let range = ClassBytesRange::new(rug_fuzz_2, rug_fuzz_3);
        debug_assert_eq!(range.end(), b'Z');
        let range = ClassBytesRange::new(rug_fuzz_4, rug_fuzz_5);
        debug_assert_eq!(range.end(), b'9');
        let _rug_ed_tests_llm_16_319_rrrruuuugggg_test_end = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_320 {
    use crate::hir::{ClassBytesRange, Interval};
    use std::cmp;
    use crate::unicode::CaseFoldError;
    use std::fmt;
    #[test]
    fn test_new() {
        let start: u8 = 97;
        let end: u8 = 122;
        let result = ClassBytesRange::new(start, end);
        assert_eq!(result.start, 97);
        assert_eq!(result.end, 122);
    }
    #[test]
    fn test_start() {
        let start: u8 = 97;
        let end: u8 = 122;
        let range = ClassBytesRange::new(start, end);
        assert_eq!(range.start(), 97);
    }
    #[test]
    fn test_end() {
        let start: u8 = 97;
        let end: u8 = 122;
        let range = ClassBytesRange::new(start, end);
        assert_eq!(range.end(), 122);
    }
    #[test]
    fn test_lower() {
        let start: u8 = 97;
        let end: u8 = 122;
        let range = ClassBytesRange::new(start, end);
        assert_eq!(range.lower(), 97);
    }
    #[test]
    fn test_upper() {
        let start: u8 = 97;
        let end: u8 = 122;
        let range = ClassBytesRange::new(start, end);
        assert_eq!(range.upper(), 122);
    }
    #[test]
    fn test_set_lower() {
        let start: u8 = 97;
        let end: u8 = 122;
        let mut range = ClassBytesRange::new(start, end);
        range.set_lower(65);
        assert_eq!(range.start(), 65);
    }
    #[test]
    fn test_set_upper() {
        let start: u8 = 97;
        let end: u8 = 122;
        let mut range = ClassBytesRange::new(start, end);
        range.set_upper(90);
        assert_eq!(range.end(), 90);
    }
    #[test]
    fn test_case_fold_simple() -> Result<(), CaseFoldError> {
        let start: u8 = 97;
        let end: u8 = 122;
        let range = ClassBytesRange::new(start, end);
        let mut ranges: Vec<ClassBytesRange> = Vec::new();
        range.case_fold_simple(&mut ranges)?;
        assert_eq!(ranges.len(), 1);
        let case_folded = ranges.get(0).unwrap();
        assert_eq!(case_folded.start, 65);
        assert_eq!(case_folded.end, 90);
        Ok(())
    }
    #[test]
    fn test_debug() {
        let start: u8 = 97;
        let end: u8 = 122;
        let range = ClassBytesRange::new(start, end);
        let debug = format!("{:?}", range);
        assert_eq!(debug, "ClassBytesRange { start: 'a', end: 'z' }");
    }
}
#[cfg(test)]
mod tests_llm_16_321 {
    use super::*;
    use crate::*;
    #[test]
    fn test_class_bytes_range_start() {
        let _rug_st_tests_llm_16_321_rrrruuuugggg_test_class_bytes_range_start = 0;
        let rug_fuzz_0 = b'A';
        let rug_fuzz_1 = b'Z';
        let range = ClassBytesRange::new(rug_fuzz_0, rug_fuzz_1);
        debug_assert_eq!(range.start(), b'A');
        let _rug_ed_tests_llm_16_321_rrrruuuugggg_test_class_bytes_range_start = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_324 {
    use super::*;
    use crate::*;
    #[test]
    fn test_difference() {
        let _rug_st_tests_llm_16_324_rrrruuuugggg_test_difference = 0;
        let rug_fuzz_0 = '\u{0041}';
        let rug_fuzz_1 = '\u{0045}';
        let rug_fuzz_2 = '\u{0041}';
        let rug_fuzz_3 = '\u{0043}';
        let rug_fuzz_4 = '\u{0044}';
        let rug_fuzz_5 = '\u{0045}';
        let mut class1 = ClassUnicode::new(
            vec![
                ClassUnicodeRange::new(rug_fuzz_0, rug_fuzz_1),
                ClassUnicodeRange::new('\u{0061}', '\u{0065}'),
                ClassUnicodeRange::new('\u{0071}', '\u{0075}')
            ],
        );
        let class2 = ClassUnicode::new(
            vec![
                ClassUnicodeRange::new(rug_fuzz_2, rug_fuzz_3),
                ClassUnicodeRange::new('\u{0062}', '\u{0064}'),
                ClassUnicodeRange::new('\u{0071}', '\u{0073}')
            ],
        );
        let expected = ClassUnicode::new(
            vec![
                ClassUnicodeRange::new(rug_fuzz_4, rug_fuzz_5),
                ClassUnicodeRange::new('\u{0061}', '\u{0061}'),
                ClassUnicodeRange::new('\u{0065}', '\u{0065}'),
                ClassUnicodeRange::new('\u{0074}', '\u{0075}')
            ],
        );
        class1.difference(&class2);
        debug_assert_eq!(class1, expected);
        let _rug_ed_tests_llm_16_324_rrrruuuugggg_test_difference = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_325 {
    use crate::hir::{ClassUnicode, ClassUnicodeRange, IntervalSet};
    #[test]
    fn test_empty() {
        let _rug_st_tests_llm_16_325_rrrruuuugggg_test_empty = 0;
        let class_unicode = ClassUnicode::empty();
        let expected_ranges: Vec<ClassUnicodeRange> = Vec::new();
        let expected_set = IntervalSet::new(expected_ranges);
        debug_assert_eq!(class_unicode.set, expected_set);
        let _rug_ed_tests_llm_16_325_rrrruuuugggg_test_empty = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_328 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_all_ascii_true() {
        let _rug_st_tests_llm_16_328_rrrruuuugggg_test_is_all_ascii_true = 0;
        let rug_fuzz_0 = 'A';
        let rug_fuzz_1 = 'Z';
        let class_unicode = ClassUnicode::new(
            vec![ClassUnicodeRange::new(rug_fuzz_0, rug_fuzz_1)],
        );
        debug_assert_eq!(class_unicode.is_all_ascii(), true);
        let _rug_ed_tests_llm_16_328_rrrruuuugggg_test_is_all_ascii_true = 0;
    }
    #[test]
    fn test_is_all_ascii_false() {
        let _rug_st_tests_llm_16_328_rrrruuuugggg_test_is_all_ascii_false = 0;
        let rug_fuzz_0 = '\u{80}';
        let rug_fuzz_1 = '\u{FF}';
        let class_unicode = ClassUnicode::new(
            vec![ClassUnicodeRange::new(rug_fuzz_0, rug_fuzz_1)],
        );
        debug_assert_eq!(class_unicode.is_all_ascii(), false);
        let _rug_ed_tests_llm_16_328_rrrruuuugggg_test_is_all_ascii_false = 0;
    }
    #[test]
    fn test_is_all_ascii_empty() {
        let _rug_st_tests_llm_16_328_rrrruuuugggg_test_is_all_ascii_empty = 0;
        let class_unicode = ClassUnicode::new(vec![]);
        debug_assert_eq!(class_unicode.is_all_ascii(), true);
        let _rug_ed_tests_llm_16_328_rrrruuuugggg_test_is_all_ascii_empty = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_329 {
    use super::*;
    use crate::*;
    #[test]
    fn test_iter() {
        let _rug_st_tests_llm_16_329_rrrruuuugggg_test_iter = 0;
        let rug_fuzz_0 = 'a';
        let rug_fuzz_1 = 'z';
        let rug_fuzz_2 = 'a';
        let rug_fuzz_3 = 'z';
        let ranges = vec![
            ClassUnicodeRange::new(rug_fuzz_0, rug_fuzz_1), ClassUnicodeRange::new('A',
            'Z'), ClassUnicodeRange::new('0', '9')
        ];
        let class = ClassUnicode::new(ranges);
        let iter = class.iter();
        let expected_ranges = vec![
            ClassUnicodeRange::new(rug_fuzz_2, rug_fuzz_3), ClassUnicodeRange::new('A',
            'Z'), ClassUnicodeRange::new('0', '9')
        ];
        for (expected, actual) in expected_ranges.iter().zip(iter) {
            debug_assert_eq!(expected, actual);
        }
        let _rug_ed_tests_llm_16_329_rrrruuuugggg_test_iter = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_338 {
    use super::*;
    use crate::*;
    #[test]
    fn test_symmetric_difference() {
        let _rug_st_tests_llm_16_338_rrrruuuugggg_test_symmetric_difference = 0;
        let rug_fuzz_0 = 'a';
        let rug_fuzz_1 = 'z';
        let rug_fuzz_2 = 'A';
        let rug_fuzz_3 = 'Z';
        let rug_fuzz_4 = '0';
        let rug_fuzz_5 = '9';
        let mut class1 = ClassUnicode::new(
            vec![
                ClassUnicodeRange::new(rug_fuzz_0, rug_fuzz_1),
                ClassUnicodeRange::new('A', 'Z')
            ],
        );
        let class2 = ClassUnicode::new(
            vec![
                ClassUnicodeRange::new(rug_fuzz_2, rug_fuzz_3),
                ClassUnicodeRange::new('0', '9')
            ],
        );
        class1.symmetric_difference(&class2);
        let expected = ClassUnicode::new(
            vec![
                ClassUnicodeRange::new(rug_fuzz_4, rug_fuzz_5),
                ClassUnicodeRange::new('a', 'z')
            ],
        );
        debug_assert_eq!(class1, expected);
        let _rug_ed_tests_llm_16_338_rrrruuuugggg_test_symmetric_difference = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_341 {
    use super::*;
    use crate::*;
    #[test]
    fn test_union() {
        let _rug_st_tests_llm_16_341_rrrruuuugggg_test_union = 0;
        let rug_fuzz_0 = 'a';
        let rug_fuzz_1 = 'z';
        let rug_fuzz_2 = 'A';
        let rug_fuzz_3 = 'Z';
        let mut class1 = ClassUnicode::new(
            vec![
                ClassUnicodeRange::new(rug_fuzz_0, rug_fuzz_1),
                ClassUnicodeRange::new('0', '9')
            ],
        );
        let class2 = ClassUnicode::new(
            vec![
                ClassUnicodeRange::new(rug_fuzz_2, rug_fuzz_3),
                ClassUnicodeRange::new('a', 'z')
            ],
        );
        class1.union(&class2);
        let _rug_ed_tests_llm_16_341_rrrruuuugggg_test_union = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_344 {
    use super::*;
    use crate::*;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_344_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = 'a';
        let rug_fuzz_1 = 'z';
        let start = rug_fuzz_0;
        let end = rug_fuzz_1;
        let result = ClassUnicodeRange::new(start, end);
        debug_assert_eq!(result.start, start);
        debug_assert_eq!(result.end, end);
        let _rug_ed_tests_llm_16_344_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_345 {
    use crate::hir::ClassUnicodeRange;
    #[test]
    fn test_start() {
        let _rug_st_tests_llm_16_345_rrrruuuugggg_test_start = 0;
        let rug_fuzz_0 = 'a';
        let rug_fuzz_1 = 'z';
        let range = ClassUnicodeRange::new(rug_fuzz_0, rug_fuzz_1);
        debug_assert_eq!(range.start(), 'a');
        let _rug_ed_tests_llm_16_345_rrrruuuugggg_test_start = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_352 {
    use super::*;
    use crate::*;
    use hir::ErrorKind::*;
    #[test]
    fn test_error_kind_description() {
        let _rug_st_tests_llm_16_352_rrrruuuugggg_test_error_kind_description = 0;
        debug_assert_eq!(UnicodeNotAllowed.description(), "Unicode not allowed here");
        debug_assert_eq!(InvalidUtf8.description(), "pattern can match invalid UTF-8");
        debug_assert_eq!(
            UnicodePropertyNotFound.description(), "Unicode property not found"
        );
        debug_assert_eq!(
            UnicodePropertyValueNotFound.description(),
            "Unicode property value not found"
        );
        debug_assert_eq!(
            UnicodePerlClassNotFound.description(),
            "Unicode-aware Perl class not found \
                                                            (make sure the unicode-perl feature is enabled)"
        );
        debug_assert_eq!(
            UnicodeCaseUnavailable.description(),
            "Unicode-aware case insensitivity matching is not available \
                                                         (make sure the unicode-case feature is enabled)"
        );
        debug_assert_eq!(
            EmptyClassNotAllowed.description(), "empty character classes are not allowed"
        );
        let _rug_ed_tests_llm_16_352_rrrruuuugggg_test_error_kind_description = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_353 {
    use super::*;
    use crate::*;
    use hir::interval::IntervalSet;
    use std::cmp::max;
    use std::ops::Not;
    #[test]
    fn test_alternation_empty() {
        let _rug_st_tests_llm_16_353_rrrruuuugggg_test_alternation_empty = 0;
        let exprs = vec![];
        let hir = Hir::alternation(exprs);
        debug_assert_eq!(hir.kind, HirKind::Empty);
        let _rug_ed_tests_llm_16_353_rrrruuuugggg_test_alternation_empty = 0;
    }
    #[test]
    fn test_alternation_single() {
        let _rug_st_tests_llm_16_353_rrrruuuugggg_test_alternation_single = 0;
        let rug_fuzz_0 = 'a';
        let exprs = vec![Hir::literal(Literal::Unicode(rug_fuzz_0))];
        let hir = Hir::alternation(exprs);
        debug_assert_eq!(
            hir, Hir { kind : HirKind::Literal(Literal::Unicode('a')), info : HirInfo {
            bools : 0b0000000000000000000000000000000000000000000000000000000000000000 }
            }
        );
        let _rug_ed_tests_llm_16_353_rrrruuuugggg_test_alternation_single = 0;
    }
    #[test]
    fn test_alternation_multiple() {
        let _rug_st_tests_llm_16_353_rrrruuuugggg_test_alternation_multiple = 0;
        let rug_fuzz_0 = 'a';
        let exprs = vec![
            Hir::literal(Literal::Unicode(rug_fuzz_0)),
            Hir::literal(Literal::Unicode('b'))
        ];
        let hir = Hir::alternation(exprs);
        debug_assert_eq!(
            hir, Hir { kind : HirKind::Literal(Literal::Unicode('a')), info : HirInfo {
            bools : 0b0000000000000000000000000000000000000000000000000000000000000000 }
            }
        );
        let _rug_ed_tests_llm_16_353_rrrruuuugggg_test_alternation_multiple = 0;
    }
    #[test]
    fn test_alternation_info() {
        let _rug_st_tests_llm_16_353_rrrruuuugggg_test_alternation_info = 0;
        let rug_fuzz_0 = 'a';
        let exprs = vec![
            Hir::literal(Literal::Unicode(rug_fuzz_0)),
            Hir::literal(Literal::Unicode('b'))
        ];
        let hir = Hir::alternation(exprs);
        debug_assert_eq!(hir.info.is_always_utf8().not(), true);
        debug_assert_eq!(hir.info.is_all_assertions().not(), true);
        debug_assert_eq!(hir.info.is_anchored_start(), true);
        debug_assert_eq!(hir.info.is_anchored_end(), true);
        debug_assert_eq!(hir.info.is_line_anchored_start(), true);
        debug_assert_eq!(hir.info.is_line_anchored_end(), true);
        debug_assert_eq!(hir.info.is_any_anchored_start().not(), true);
        debug_assert_eq!(hir.info.is_any_anchored_end().not(), true);
        debug_assert_eq!(hir.info.is_match_empty().not(), true);
        debug_assert_eq!(hir.info.is_literal().not(), true);
        debug_assert_eq!(hir.info.is_alternation_literal(), true);
        let _rug_ed_tests_llm_16_353_rrrruuuugggg_test_alternation_info = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_358 {
    use crate::hir::{
        Anchor, Class, ClassBytes, ClassBytesRange, ClassUnicode, ClassUnicodeRange, Hir,
        Literal, Repetition, RepetitionRange, WordBoundary,
    };
    #[test]
    fn test_dot() {
        let _rug_st_tests_llm_16_358_rrrruuuugggg_test_dot = 0;
        let rug_fuzz_0 = false;
        let dot = Hir::dot(rug_fuzz_0);
        debug_assert_eq!(
            dot,
            Hir::class(Class::Unicode(ClassUnicode::new(vec![ClassUnicodeRange::new('\0',
            '\x09'), ClassUnicodeRange::new('\x0B', '\u{10FFFF}'),])))
        );
        let _rug_ed_tests_llm_16_358_rrrruuuugggg_test_dot = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_359 {
    use super::*;
    use crate::*;
    #[test]
    fn test_empty() {
        let _rug_st_tests_llm_16_359_rrrruuuugggg_test_empty = 0;
        let hir = Hir::empty();
        debug_assert_eq!(hir.kind(), & HirKind::Empty);
        debug_assert!(hir.is_always_utf8());
        debug_assert!(hir.is_match_empty());
        debug_assert!(! hir.is_literal());
        debug_assert!(! hir.is_alternation_literal());
        debug_assert!(! hir.is_anchored_start());
        debug_assert!(! hir.is_anchored_end());
        debug_assert!(! hir.is_line_anchored_start());
        debug_assert!(! hir.is_line_anchored_end());
        debug_assert!(! hir.is_any_anchored_start());
        debug_assert!(! hir.is_any_anchored_end());
        let _rug_ed_tests_llm_16_359_rrrruuuugggg_test_empty = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_361 {
    use super::*;
    use crate::*;
    #[test]
    fn test_into_kind() {
        let _rug_st_tests_llm_16_361_rrrruuuugggg_test_into_kind = 0;
        let rug_fuzz_0 = 'a';
        let mut h = Hir::literal(Literal::Unicode(rug_fuzz_0));
        let kind = h.into_kind();
        debug_assert_eq!(kind, HirKind::Literal(Literal::Unicode('a')));
        let _rug_ed_tests_llm_16_361_rrrruuuugggg_test_into_kind = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_362 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_all_assertions() {
        let _rug_st_tests_llm_16_362_rrrruuuugggg_test_is_all_assertions = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 1;
        let hir = Hir {
            info: HirInfo {
                bools: rug_fuzz_0 << rug_fuzz_1,
            },
            kind: HirKind::Empty,
        };
        debug_assert_eq!(hir.is_all_assertions(), true);
        let _rug_ed_tests_llm_16_362_rrrruuuugggg_test_is_all_assertions = 0;
    }
    #[test]
    fn test_is_all_assertions_false() {
        let _rug_st_tests_llm_16_362_rrrruuuugggg_test_is_all_assertions_false = 0;
        let rug_fuzz_0 = 0;
        let hir = Hir {
            info: HirInfo { bools: rug_fuzz_0 },
            kind: HirKind::Empty,
        };
        debug_assert_eq!(hir.is_all_assertions(), false);
        let _rug_ed_tests_llm_16_362_rrrruuuugggg_test_is_all_assertions_false = 0;
    }
    #[test]
    fn test_is_all_assertions_alternation() {
        let _rug_st_tests_llm_16_362_rrrruuuugggg_test_is_all_assertions_alternation = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 1;
        let hir = Hir {
            info: HirInfo {
                bools: rug_fuzz_0 << rug_fuzz_1,
            },
            kind: HirKind::Alternation(
                vec![
                    Hir { info : HirInfo { bools : rug_fuzz_2 << rug_fuzz_3, }, kind :
                    HirKind::Empty, }, Hir { info : HirInfo::new(), kind :
                    HirKind::Empty, }
                ],
            ),
        };
        debug_assert_eq!(hir.is_all_assertions(), true);
        let _rug_ed_tests_llm_16_362_rrrruuuugggg_test_is_all_assertions_alternation = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_363 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_alternation_literal() {
        let _rug_st_tests_llm_16_363_rrrruuuugggg_test_is_alternation_literal = 0;
        let rug_fuzz_0 = 'a';
        let rug_fuzz_1 = 'b';
        let rug_fuzz_2 = 'a';
        let rug_fuzz_3 = 'b';
        let rug_fuzz_4 = 'a';
        let rug_fuzz_5 = 'a';
        let rug_fuzz_6 = true;
        let rug_fuzz_7 = 'b';
        let literal = Literal::Unicode(rug_fuzz_0);
        let hir = Hir::literal(literal);
        debug_assert!(hir.is_alternation_literal());
        let hir1 = Hir::anchor(Anchor::StartLine);
        let hir2 = Hir::literal(Literal::Unicode(rug_fuzz_1));
        let hir = Hir::alternation(vec![hir1, hir2]);
        debug_assert!(hir.is_alternation_literal());
        let hir1 = Hir::literal(Literal::Unicode(rug_fuzz_2));
        let hir2 = Hir::literal(Literal::Unicode(rug_fuzz_3));
        let hir = Hir::alternation(vec![hir1, hir2]);
        debug_assert!(hir.is_alternation_literal());
        let hir1 = Hir::literal(Literal::Unicode(rug_fuzz_4));
        let hir2 = Hir::anchor(Anchor::StartLine);
        let hir = Hir::alternation(vec![hir1, hir2]);
        debug_assert!(! hir.is_alternation_literal());
        let hir1 = Hir::literal(Literal::Unicode(rug_fuzz_5));
        let hir2 = Hir::repetition(Repetition {
            kind: RepetitionKind::ZeroOrMore,
            greedy: rug_fuzz_6,
            hir: Box::new(Hir::literal(Literal::Unicode(rug_fuzz_7))),
        });
        let hir = Hir::alternation(vec![hir1, hir2]);
        debug_assert!(! hir.is_alternation_literal());
        let _rug_ed_tests_llm_16_363_rrrruuuugggg_test_is_alternation_literal = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_364 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_always_utf8() {
        let _rug_st_tests_llm_16_364_rrrruuuugggg_test_is_always_utf8 = 0;
        let hir = Hir::empty();
        debug_assert_eq!(hir.is_always_utf8(), true);
        let _rug_ed_tests_llm_16_364_rrrruuuugggg_test_is_always_utf8 = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_365 {
    use crate::hir::{
        Anchor, Class, ClassBytes, ClassBytesRange, ClassUnicode, Hir, HirKind,
        WordBoundary,
    };
    #[test]
    fn test_hir_is_anchored_end() {
        let _rug_st_tests_llm_16_365_rrrruuuugggg_test_hir_is_anchored_end = 0;
        let hir = Hir::anchor(Anchor::EndText);
        debug_assert!(hir.is_anchored_end());
        let _rug_ed_tests_llm_16_365_rrrruuuugggg_test_hir_is_anchored_end = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_366 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_anchored_start() {
        let _rug_st_tests_llm_16_366_rrrruuuugggg_test_is_anchored_start = 0;
        let rug_fuzz_0 = true;
        let hir = Hir::anchor(Anchor::StartText);
        debug_assert_eq!(rug_fuzz_0, hir.is_anchored_start());
        let _rug_ed_tests_llm_16_366_rrrruuuugggg_test_is_anchored_start = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_367 {
    use super::*;
    use crate::*;
    use hir::{Anchor, Class, ClassBytes, ClassBytesRange, Hir, Literal};
    #[test]
    fn test_is_any_anchored_end() {
        let _rug_st_tests_llm_16_367_rrrruuuugggg_test_is_any_anchored_end = 0;
        let hir = Hir::anchor(Anchor::EndText);
        debug_assert_eq!(hir.is_any_anchored_end(), true);
        let _rug_ed_tests_llm_16_367_rrrruuuugggg_test_is_any_anchored_end = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_368 {
    use super::*;
    use crate::*;
    use crate::hir::interval::{Interval, IntervalSet};
    use crate::hir::ClassUnicodeRange;
    #[test]
    fn test_is_any_anchored_start() {
        let _rug_st_tests_llm_16_368_rrrruuuugggg_test_is_any_anchored_start = 0;
        let hir = Hir::anchor(Anchor::StartText);
        debug_assert_eq!(hir.is_any_anchored_start(), true);
        let _rug_ed_tests_llm_16_368_rrrruuuugggg_test_is_any_anchored_start = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_369 {
    use crate::hir::{
        Anchor, Class, ClassBytes, ClassBytesRange, ClassUnicode, ClassUnicodeRange,
        Group, GroupKind, Hir, Literal, Repetition, RepetitionKind, RepetitionRange,
        WordBoundary,
    };
    #[test]
    fn test_is_line_anchored_end() {
        let _rug_st_tests_llm_16_369_rrrruuuugggg_test_is_line_anchored_end = 0;
        let hir = Hir::anchor(Anchor::EndLine);
        debug_assert_eq!(hir.is_line_anchored_end(), true);
        let _rug_ed_tests_llm_16_369_rrrruuuugggg_test_is_line_anchored_end = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_370 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_line_anchored_start() {
        let _rug_st_tests_llm_16_370_rrrruuuugggg_test_is_line_anchored_start = 0;
        let hir = Hir::anchor(Anchor::StartText);
        debug_assert_eq!(hir.is_line_anchored_start(), true);
        let _rug_ed_tests_llm_16_370_rrrruuuugggg_test_is_line_anchored_start = 0;
    }
    #[test]
    fn test_is_line_anchored_start_2() {
        let _rug_st_tests_llm_16_370_rrrruuuugggg_test_is_line_anchored_start_2 = 0;
        let hir = Hir::anchor(Anchor::EndText);
        debug_assert_eq!(hir.is_line_anchored_start(), false);
        let _rug_ed_tests_llm_16_370_rrrruuuugggg_test_is_line_anchored_start_2 = 0;
    }
    #[test]
    fn test_is_line_anchored_start_3() {
        let _rug_st_tests_llm_16_370_rrrruuuugggg_test_is_line_anchored_start_3 = 0;
        let hir = Hir::anchor(Anchor::StartLine);
        debug_assert_eq!(hir.is_line_anchored_start(), true);
        let _rug_ed_tests_llm_16_370_rrrruuuugggg_test_is_line_anchored_start_3 = 0;
    }
    #[test]
    fn test_is_line_anchored_start_4() {
        let _rug_st_tests_llm_16_370_rrrruuuugggg_test_is_line_anchored_start_4 = 0;
        let hir = Hir::anchor(Anchor::EndLine);
        debug_assert_eq!(hir.is_line_anchored_start(), false);
        let _rug_ed_tests_llm_16_370_rrrruuuugggg_test_is_line_anchored_start_4 = 0;
    }
    #[test]
    fn test_is_line_anchored_start_5() {
        let _rug_st_tests_llm_16_370_rrrruuuugggg_test_is_line_anchored_start_5 = 0;
        let hir = Hir::anchor(Anchor::StartText);
        debug_assert_eq!(hir.is_line_anchored_start(), true);
        let _rug_ed_tests_llm_16_370_rrrruuuugggg_test_is_line_anchored_start_5 = 0;
    }
    #[test]
    fn test_is_line_anchored_start_6() {
        let _rug_st_tests_llm_16_370_rrrruuuugggg_test_is_line_anchored_start_6 = 0;
        let hir = Hir::anchor(Anchor::EndText);
        debug_assert_eq!(hir.is_line_anchored_start(), false);
        let _rug_ed_tests_llm_16_370_rrrruuuugggg_test_is_line_anchored_start_6 = 0;
    }
    #[test]
    fn test_is_line_anchored_start_7() {
        let _rug_st_tests_llm_16_370_rrrruuuugggg_test_is_line_anchored_start_7 = 0;
        let hir = Hir::anchor(Anchor::StartLine);
        debug_assert_eq!(hir.is_line_anchored_start(), true);
        let _rug_ed_tests_llm_16_370_rrrruuuugggg_test_is_line_anchored_start_7 = 0;
    }
    #[test]
    fn test_is_line_anchored_start_8() {
        let _rug_st_tests_llm_16_370_rrrruuuugggg_test_is_line_anchored_start_8 = 0;
        let hir = Hir::anchor(Anchor::EndLine);
        debug_assert_eq!(hir.is_line_anchored_start(), false);
        let _rug_ed_tests_llm_16_370_rrrruuuugggg_test_is_line_anchored_start_8 = 0;
    }
    #[test]
    fn test_is_line_anchored_start_9() {
        let _rug_st_tests_llm_16_370_rrrruuuugggg_test_is_line_anchored_start_9 = 0;
        let hir = Hir::anchor(Anchor::EndLine);
        debug_assert_eq!(hir.is_line_anchored_start(), false);
        let _rug_ed_tests_llm_16_370_rrrruuuugggg_test_is_line_anchored_start_9 = 0;
    }
    #[test]
    fn test_is_line_anchored_start_10() {
        let _rug_st_tests_llm_16_370_rrrruuuugggg_test_is_line_anchored_start_10 = 0;
        let hir = Hir::anchor(Anchor::StartLine);
        debug_assert_eq!(hir.is_line_anchored_start(), true);
        let _rug_ed_tests_llm_16_370_rrrruuuugggg_test_is_line_anchored_start_10 = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_372 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_match_empty() {
        let _rug_st_tests_llm_16_372_rrrruuuugggg_test_is_match_empty = 0;
        let rug_fuzz_0 = 'a';
        let rug_fuzz_1 = 'a';
        let rug_fuzz_2 = true;
        let rug_fuzz_3 = 0;
        let hir = Hir::empty();
        debug_assert_eq!(hir.is_match_empty(), true);
        let hir = Hir::literal(Literal::Unicode(rug_fuzz_0));
        debug_assert_eq!(hir.is_match_empty(), false);
        let hir = Hir::literal(Literal::Unicode(rug_fuzz_1));
        let repetition = Repetition {
            kind: RepetitionKind::ZeroOrMore,
            greedy: rug_fuzz_2,
            hir: Box::new(hir),
        };
        debug_assert_eq!(repetition.is_match_empty(), true);
        let hir = Hir::group(Group {
            kind: GroupKind::CaptureIndex(rug_fuzz_3),
            hir: Box::new(Hir::empty()),
        });
        debug_assert_eq!(hir.is_match_empty(), true);
        let _rug_ed_tests_llm_16_372_rrrruuuugggg_test_is_match_empty = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_379 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_alternation_literal() {
        let _rug_st_tests_llm_16_379_rrrruuuugggg_test_is_alternation_literal = 0;
        let hirinfo = HirInfo::new();
        debug_assert_eq!(hirinfo.is_alternation_literal(), false);
        let _rug_ed_tests_llm_16_379_rrrruuuugggg_test_is_alternation_literal = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_380 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_always_utf8() {
        let _rug_st_tests_llm_16_380_rrrruuuugggg_test_is_always_utf8 = 0;
        let rug_fuzz_0 = 0b0000000000000001;
        let rug_fuzz_1 = 0b0000000000000010;
        let hir_info = HirInfo::new();
        debug_assert_eq!(hir_info.is_always_utf8(), false);
        let hir_info = HirInfo { bools: rug_fuzz_0 };
        debug_assert_eq!(hir_info.is_always_utf8(), true);
        let hir_info = HirInfo { bools: rug_fuzz_1 };
        debug_assert_eq!(hir_info.is_always_utf8(), false);
        let _rug_ed_tests_llm_16_380_rrrruuuugggg_test_is_always_utf8 = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_381 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_anchored_end() {
        let _rug_st_tests_llm_16_381_rrrruuuugggg_test_is_anchored_end = 0;
        let rug_fuzz_0 = true;
        let hir = HirInfo::new();
        debug_assert!(! hir.is_anchored_end());
        let mut hir2 = hir.clone();
        hir2.set_anchored_end(rug_fuzz_0);
        debug_assert!(hir2.is_anchored_end());
        let _rug_ed_tests_llm_16_381_rrrruuuugggg_test_is_anchored_end = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_384 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_any_anchored_end() {
        let _rug_st_tests_llm_16_384_rrrruuuugggg_test_is_any_anchored_end = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = true;
        let rug_fuzz_2 = false;
        let hir_info = HirInfo::new();
        debug_assert_eq!(hir_info.is_any_anchored_end(), false);
        let mut hir_info = HirInfo::new();
        hir_info.set_any_anchored_end(rug_fuzz_0);
        debug_assert_eq!(hir_info.is_any_anchored_end(), true);
        let mut hir_info = HirInfo::new();
        hir_info.set_any_anchored_end(rug_fuzz_1);
        hir_info.set_any_anchored_end(rug_fuzz_2);
        debug_assert_eq!(hir_info.is_any_anchored_end(), false);
        let _rug_ed_tests_llm_16_384_rrrruuuugggg_test_is_any_anchored_end = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_389 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_line_anchored_start() {
        let _rug_st_tests_llm_16_389_rrrruuuugggg_test_is_line_anchored_start = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = false;
        let mut hir_info = HirInfo::new();
        hir_info.set_line_anchored_start(rug_fuzz_0);
        debug_assert_eq!(hir_info.is_line_anchored_start(), true);
        hir_info.set_line_anchored_start(rug_fuzz_1);
        debug_assert_eq!(hir_info.is_line_anchored_start(), false);
        let _rug_ed_tests_llm_16_389_rrrruuuugggg_test_is_line_anchored_start = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_390 {
    use super::*;
    use crate::*;
    use crate::hir::HirInfo;
    #[test]
    fn test_is_literal_returns_true_when_literal_flag_is_set() {
        let _rug_st_tests_llm_16_390_rrrruuuugggg_test_is_literal_returns_true_when_literal_flag_is_set = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 9;
        let hir_info = HirInfo {
            bools: rug_fuzz_0 << rug_fuzz_1,
        };
        debug_assert_eq!(hir_info.is_literal(), true);
        let _rug_ed_tests_llm_16_390_rrrruuuugggg_test_is_literal_returns_true_when_literal_flag_is_set = 0;
    }
    #[test]
    fn test_is_literal_returns_false_when_literal_flag_is_not_set() {
        let _rug_st_tests_llm_16_390_rrrruuuugggg_test_is_literal_returns_false_when_literal_flag_is_not_set = 0;
        let rug_fuzz_0 = 0;
        let hir_info = HirInfo { bools: rug_fuzz_0 };
        debug_assert_eq!(hir_info.is_literal(), false);
        let _rug_ed_tests_llm_16_390_rrrruuuugggg_test_is_literal_returns_false_when_literal_flag_is_not_set = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_393 {
    use super::*;
    use crate::*;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_393_rrrruuuugggg_test_new = 0;
        let hir_info = HirInfo::new();
        debug_assert_eq!(hir_info.bools, 0);
        let _rug_ed_tests_llm_16_393_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_394 {
    use super::*;
    use crate::*;
    #[test]
    fn test_set_all_assertions() {
        let _rug_st_tests_llm_16_394_rrrruuuugggg_test_set_all_assertions = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = false;
        let mut hir_info = HirInfo::new();
        hir_info.set_all_assertions(rug_fuzz_0);
        debug_assert_eq!(hir_info.bools, 2);
        hir_info.set_all_assertions(rug_fuzz_1);
        debug_assert_eq!(hir_info.bools, 0);
        let _rug_ed_tests_llm_16_394_rrrruuuugggg_test_set_all_assertions = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_395 {
    use crate::hir::HirInfo;
    #[test]
    fn test_set_alternation_literal() {
        let _rug_st_tests_llm_16_395_rrrruuuugggg_test_set_alternation_literal = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = false;
        let mut hir_info = HirInfo::new();
        hir_info.set_alternation_literal(rug_fuzz_0);
        debug_assert_eq!(hir_info.bools, 1 << 10);
        hir_info.set_alternation_literal(rug_fuzz_1);
        debug_assert_eq!(hir_info.bools, 0);
        let _rug_ed_tests_llm_16_395_rrrruuuugggg_test_set_alternation_literal = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_396 {
    use super::*;
    use crate::*;
    #[test]
    fn test_set_always_utf8() {
        let _rug_st_tests_llm_16_396_rrrruuuugggg_test_set_always_utf8 = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = false;
        let mut hir_info = HirInfo::new();
        debug_assert_eq!(hir_info.is_always_utf8(), false);
        hir_info.set_always_utf8(rug_fuzz_0);
        debug_assert_eq!(hir_info.is_always_utf8(), true);
        hir_info.set_always_utf8(rug_fuzz_1);
        debug_assert_eq!(hir_info.is_always_utf8(), false);
        let _rug_ed_tests_llm_16_396_rrrruuuugggg_test_set_always_utf8 = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_397 {
    use super::*;
    use crate::*;
    #[test]
    fn test_set_anchored_end() {
        let _rug_st_tests_llm_16_397_rrrruuuugggg_test_set_anchored_end = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = false;
        let rug_fuzz_4 = 1;
        let rug_fuzz_5 = 3;
        let mut hir_info = HirInfo::new();
        hir_info.set_anchored_end(rug_fuzz_0);
        debug_assert_eq!(hir_info.bools & (rug_fuzz_1 << rug_fuzz_2), (1 << 3));
        hir_info.set_anchored_end(rug_fuzz_3);
        debug_assert_eq!(hir_info.bools & (rug_fuzz_4 << rug_fuzz_5), 0);
        let _rug_ed_tests_llm_16_397_rrrruuuugggg_test_set_anchored_end = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_398 {
    use super::*;
    use crate::*;
    #[test]
    fn test_set_anchored_start() {
        let _rug_st_tests_llm_16_398_rrrruuuugggg_test_set_anchored_start = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = false;
        let mut hir_info = HirInfo::new();
        hir_info.set_anchored_start(rug_fuzz_0);
        debug_assert_eq!(hir_info.bools, 1 << 2);
        hir_info.set_anchored_start(rug_fuzz_1);
        debug_assert_eq!(hir_info.bools, 0);
        let _rug_ed_tests_llm_16_398_rrrruuuugggg_test_set_anchored_start = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_401 {
    use super::*;
    use crate::*;
    #[test]
    fn test_set_any_anchored_start() {
        let _rug_st_tests_llm_16_401_rrrruuuugggg_test_set_any_anchored_start = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = false;
        let mut hir_info = HirInfo::new();
        hir_info.set_any_anchored_start(rug_fuzz_0);
        debug_assert_eq!(hir_info.bools, 1 << 6);
        hir_info.set_any_anchored_start(rug_fuzz_1);
        debug_assert_eq!(hir_info.bools, 0);
        let _rug_ed_tests_llm_16_401_rrrruuuugggg_test_set_any_anchored_start = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_402 {
    use super::*;
    use crate::*;
    #[test]
    fn test_set_line_anchored_end() {
        let _rug_st_tests_llm_16_402_rrrruuuugggg_test_set_line_anchored_end = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = 5;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = false;
        let rug_fuzz_4 = 5;
        let rug_fuzz_5 = 1;
        let mut info = HirInfo::new();
        info.set_line_anchored_end(rug_fuzz_0);
        debug_assert_eq!((info.bools >> rug_fuzz_1) & rug_fuzz_2, 1);
        info.set_line_anchored_end(rug_fuzz_3);
        debug_assert_eq!((info.bools >> rug_fuzz_4) & rug_fuzz_5, 0);
        let _rug_ed_tests_llm_16_402_rrrruuuugggg_test_set_line_anchored_end = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_403 {
    use super::*;
    use crate::*;
    #[test]
    fn test_set_line_anchored_start() {
        let _rug_st_tests_llm_16_403_rrrruuuugggg_test_set_line_anchored_start = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 4;
        let rug_fuzz_3 = false;
        let rug_fuzz_4 = 1;
        let rug_fuzz_5 = 4;
        let mut hi = HirInfo::new();
        hi.set_line_anchored_start(rug_fuzz_0);
        debug_assert_eq!(hi.bools & (rug_fuzz_1 << rug_fuzz_2), 1 << 4);
        hi.set_line_anchored_start(rug_fuzz_3);
        debug_assert_eq!(hi.bools & (rug_fuzz_4 << rug_fuzz_5), 0);
        let _rug_ed_tests_llm_16_403_rrrruuuugggg_test_set_line_anchored_start = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_404 {
    use super::*;
    use crate::*;
    #[test]
    fn test_set_literal() {
        let _rug_st_tests_llm_16_404_rrrruuuugggg_test_set_literal = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = false;
        let mut hir_info = HirInfo::new();
        hir_info.set_literal(rug_fuzz_0);
        debug_assert_eq!(hir_info.bools, 1 << 9);
        hir_info.set_literal(rug_fuzz_1);
        debug_assert_eq!(hir_info.bools, 0);
        let _rug_ed_tests_llm_16_404_rrrruuuugggg_test_set_literal = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_405 {
    use super::*;
    use crate::*;
    #[test]
    fn test_set_match_empty() {
        let _rug_st_tests_llm_16_405_rrrruuuugggg_test_set_match_empty = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = false;
        let mut hir_info = HirInfo::new();
        hir_info.set_match_empty(rug_fuzz_0);
        debug_assert_eq!(hir_info.bools, 1 << 8);
        hir_info.set_match_empty(rug_fuzz_1);
        debug_assert_eq!(hir_info.bools, 0);
        let _rug_ed_tests_llm_16_405_rrrruuuugggg_test_set_match_empty = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_408 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_empty() {
        let _rug_st_tests_llm_16_408_rrrruuuugggg_test_is_empty = 0;
        let rug_fuzz_0 = 'a';
        let rug_fuzz_1 = true;
        let rug_fuzz_2 = false;
        let hir_empty = HirKind::Empty;
        let hir_not_empty = HirKind::Literal(Literal::Unicode(rug_fuzz_0));
        debug_assert_eq!(rug_fuzz_1, hir_empty.is_empty());
        debug_assert_eq!(rug_fuzz_2, hir_not_empty.is_empty());
        let _rug_ed_tests_llm_16_408_rrrruuuugggg_test_is_empty = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_409 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_unicode_unicode() {
        let _rug_st_tests_llm_16_409_rrrruuuugggg_test_is_unicode_unicode = 0;
        let rug_fuzz_0 = 'a';
        let literal = Literal::Unicode(rug_fuzz_0);
        debug_assert_eq!(literal.is_unicode(), true);
        let _rug_ed_tests_llm_16_409_rrrruuuugggg_test_is_unicode_unicode = 0;
    }
    #[test]
    fn test_is_unicode_byte_within_limit() {
        let _rug_st_tests_llm_16_409_rrrruuuugggg_test_is_unicode_byte_within_limit = 0;
        let rug_fuzz_0 = 0x41;
        let literal = Literal::Byte(rug_fuzz_0);
        debug_assert_eq!(literal.is_unicode(), true);
        let _rug_ed_tests_llm_16_409_rrrruuuugggg_test_is_unicode_byte_within_limit = 0;
    }
    #[test]
    fn test_is_unicode_byte_outside_limit() {
        let _rug_st_tests_llm_16_409_rrrruuuugggg_test_is_unicode_byte_outside_limit = 0;
        let rug_fuzz_0 = 0xFF;
        let literal = Literal::Byte(rug_fuzz_0);
        debug_assert_eq!(literal.is_unicode(), false);
        let _rug_ed_tests_llm_16_409_rrrruuuugggg_test_is_unicode_byte_outside_limit = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_412 {
    use crate::hir::WordBoundary;
    #[test]
    fn test_is_negated() {
        let _rug_st_tests_llm_16_412_rrrruuuugggg_test_is_negated = 0;
        let unicode = WordBoundary::Unicode;
        let unicode_negate = WordBoundary::UnicodeNegate;
        let ascii = WordBoundary::Ascii;
        let ascii_negate = WordBoundary::AsciiNegate;
        debug_assert_eq!(unicode.is_negated(), false);
        debug_assert_eq!(unicode_negate.is_negated(), true);
        debug_assert_eq!(ascii.is_negated(), false);
        debug_assert_eq!(ascii_negate.is_negated(), true);
        let _rug_ed_tests_llm_16_412_rrrruuuugggg_test_is_negated = 0;
    }
}
#[cfg(test)]
mod tests_rug_289 {
    use super::*;
    use crate::hir::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_289_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 'a';
        let mut p0 = Hir::literal(Literal::Unicode(rug_fuzz_0));
        crate::hir::Hir::kind(&p0);
        let _rug_ed_tests_rug_289_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_290 {
    use super::*;
    use crate::hir::{Hir, Literal};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_290_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = '\u{0071}';
        let mut p0 = Literal::Unicode(rug_fuzz_0);
        let _ = <Hir>::literal(p0);
        let _rug_ed_tests_rug_290_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_292 {
    use super::*;
    use crate::hir::{Hir, Anchor};
    #[test]
    fn test_anchor() {
        let _rug_st_tests_rug_292_rrrruuuugggg_test_anchor = 0;
        let mut p0: Anchor = Anchor::StartLine;
        Hir::anchor(p0);
        let _rug_ed_tests_rug_292_rrrruuuugggg_test_anchor = 0;
    }
}
#[cfg(test)]
mod tests_rug_293 {
    use super::*;
    use crate::hir::{Hir, HirKind, HirInfo, WordBoundary};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_293_rrrruuuugggg_test_rug = 0;
        let mut p0 = WordBoundary::Unicode;
        Hir::word_boundary(p0);
        let _rug_ed_tests_rug_293_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_294 {
    use super::*;
    use crate::hir::{Repetition, RepetitionKind, Hir};
    #[test]
    fn test_repetition() {
        let _rug_st_tests_rug_294_rrrruuuugggg_test_repetition = 0;
        let rug_fuzz_0 = false;
        let mut v95 = Repetition {
            kind: RepetitionKind::ZeroOrMore,
            greedy: rug_fuzz_0,
            hir: Box::new(Hir::empty()),
        };
        let p0 = v95;
        crate::hir::Hir::repetition(p0);
        let _rug_ed_tests_rug_294_rrrruuuugggg_test_repetition = 0;
    }
}
#[cfg(test)]
mod tests_rug_295 {
    use super::*;
    use crate::hir::{Group, GroupKind, Hir};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_295_rrrruuuugggg_test_rug = 0;
        let mut p0 = Group {
            kind: GroupKind::NonCapturing,
            hir: Box::new(Hir::empty()),
        };
        crate::hir::Hir::group(p0);
        let _rug_ed_tests_rug_295_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_296 {
    use super::*;
    use crate::hir;
    use std::vec::Vec;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_296_rrrruuuugggg_test_rug = 0;
        let mut p0: Vec<hir::Hir> = Vec::new();
        crate::hir::Hir::concat(p0);
        let _rug_ed_tests_rug_296_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_297 {
    use super::*;
    use crate::hir::{
        Hir, Class, ClassBytes, ClassBytesRange, ClassUnicode, ClassUnicodeRange,
    };
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_297_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = true;
        let p0: bool = rug_fuzz_0;
        Hir::any(p0);
        let _rug_ed_tests_rug_297_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_298 {
    use super::*;
    use crate::hir::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_298_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 'a';
        let mut p0 = Hir::literal(Literal::Unicode(rug_fuzz_0));
        p0.is_literal();
        let _rug_ed_tests_rug_298_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_299_prepare {
    use crate::hir::{HirKind, Literal, Class, Anchor, WordBoundary, Repetition, Group};
    #[test]
    fn sample() {
        let _rug_st_tests_rug_299_prepare_rrrruuuugggg_sample = 0;
        let mut v98: HirKind = HirKind::Empty;
        let _rug_ed_tests_rug_299_prepare_rrrruuuugggg_sample = 0;
    }
}
#[cfg(test)]
mod tests_rug_299 {
    use super::*;
    use crate::hir::{HirKind, Literal, Class, Anchor, WordBoundary, Repetition, Group};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_299_rrrruuuugggg_test_rug = 0;
        let mut p0: HirKind = HirKind::Empty;
        crate::hir::HirKind::has_subexprs(&p0);
        let _rug_ed_tests_rug_299_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_302 {
    use super::*;
    use crate::hir::{ClassUnicode, ClassUnicodeRange};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_302_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = '\u{61}';
        let rug_fuzz_1 = '\u{62}';
        let p0: Vec<ClassUnicodeRange> = vec![
            ClassUnicodeRange::new(rug_fuzz_0, rug_fuzz_1),
            ClassUnicodeRange::new('\u{63}', '\u{64}')
        ];
        ClassUnicode::new(p0);
        let _rug_ed_tests_rug_302_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_303 {
    use super::*;
    use crate::hir::ClassUnicodeRange;
    use crate::hir::ClassUnicode;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_303_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = '\u{80}';
        let rug_fuzz_1 = '\u{80}';
        let mut p0 = ClassUnicode::empty();
        let mut p1 = ClassUnicodeRange::new(rug_fuzz_0, rug_fuzz_1);
        p0.push(p1);
        let _rug_ed_tests_rug_303_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_304 {
    use super::*;
    use crate::hir::ClassUnicode;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_304_rrrruuuugggg_test_rug = 0;
        let mut p0 = ClassUnicode::empty();
        ClassUnicode::ranges(&p0);
        let _rug_ed_tests_rug_304_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_310 {
    use super::*;
    use crate::hir::interval::Interval;
    #[test]
    fn test_case_fold_simple() {
        let _rug_st_tests_rug_310_rrrruuuugggg_test_case_fold_simple = 0;
        let rug_fuzz_0 = '\u{80}';
        let rug_fuzz_1 = '\u{80}';
        use crate::hir::{ClassUnicodeRange, interval::Interval};
        use crate::unicode::CaseFoldError;
        let mut p0 = ClassUnicodeRange::new(rug_fuzz_0, rug_fuzz_1);
        let mut p1: Vec<ClassUnicodeRange> = Vec::new();
        debug_assert!(
            < ClassUnicodeRange as Interval > ::case_fold_simple(& mut p0, & mut p1)
            .is_ok()
        );
        let _rug_ed_tests_rug_310_rrrruuuugggg_test_case_fold_simple = 0;
    }
}
#[cfg(test)]
mod tests_rug_311 {
    use super::*;
    use crate::hir::ClassUnicodeRange;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_311_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = '\u{80}';
        let rug_fuzz_1 = '\u{80}';
        let mut p0 = ClassUnicodeRange::new(rug_fuzz_0, rug_fuzz_1);
        <ClassUnicodeRange>::end(&p0);
        let _rug_ed_tests_rug_311_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_312 {
    use super::*;
    use crate::hir::{ClassBytes, ClassBytesRange};
    #[test]
    fn test_case_fold_simple() {
        let _rug_st_tests_rug_312_rrrruuuugggg_test_case_fold_simple = 0;
        let rug_fuzz_0 = b'a';
        let rug_fuzz_1 = b'z';
        let mut p0 = ClassBytes::new(
            vec![
                ClassBytesRange::new(rug_fuzz_0, rug_fuzz_1), ClassBytesRange::new(b'A',
                b'Z')
            ],
        );
        p0.case_fold_simple();
        let _rug_ed_tests_rug_312_rrrruuuugggg_test_case_fold_simple = 0;
    }
}
#[cfg(test)]
mod tests_rug_313 {
    use super::*;
    use crate::hir::{ClassBytes, ClassBytesRange};
    #[test]
    fn test_intersect() {
        let _rug_st_tests_rug_313_rrrruuuugggg_test_intersect = 0;
        let rug_fuzz_0 = b'a';
        let rug_fuzz_1 = b'z';
        let rug_fuzz_2 = b'a';
        let rug_fuzz_3 = b'z';
        let mut p0 = ClassBytes::new(
            vec![
                ClassBytesRange::new(rug_fuzz_0, rug_fuzz_1), ClassBytesRange::new(b'A',
                b'Z')
            ],
        );
        let p1 = ClassBytes::new(
            vec![
                ClassBytesRange::new(rug_fuzz_2, rug_fuzz_3), ClassBytesRange::new(b'0',
                b'9')
            ],
        );
        p0.intersect(&p1);
        let _rug_ed_tests_rug_313_rrrruuuugggg_test_intersect = 0;
    }
}
#[cfg(test)]
mod tests_rug_314 {
    use super::*;
    use crate::hir::{ClassBytes, ClassBytesRange};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_314_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b'a';
        let rug_fuzz_1 = b'z';
        let rug_fuzz_2 = b'a';
        let rug_fuzz_3 = b'z';
        let mut p0 = {
            let mut v52 = ClassBytes::new(
                vec![
                    ClassBytesRange::new(rug_fuzz_0, rug_fuzz_1),
                    ClassBytesRange::new(b'A', b'Z')
                ],
            );
            v52
        };
        let p1 = {
            let v52 = ClassBytes::new(
                vec![ClassBytesRange::new(rug_fuzz_2, rug_fuzz_3)],
            );
            v52
        };
        p0.difference(&p1);
        let _rug_ed_tests_rug_314_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_316 {
    use super::*;
    use crate::hir::{ClassBytesRange, interval::Interval};
    #[test]
    fn test_case_fold_simple() {
        let _rug_st_tests_rug_316_rrrruuuugggg_test_case_fold_simple = 0;
        let rug_fuzz_0 = 0x41;
        let rug_fuzz_1 = 0x7A;
        let mut p0 = ClassBytesRange::new(rug_fuzz_0, rug_fuzz_1);
        let mut p1: Vec<ClassBytesRange> = Vec::new();
        p0.case_fold_simple(&mut p1).unwrap();
        let _rug_ed_tests_rug_316_rrrruuuugggg_test_case_fold_simple = 0;
    }
}
#[cfg(test)]
mod tests_rug_317 {
    use crate::hir::{Repetition, RepetitionRange, RepetitionKind};
    use crate::hir::Hir;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_317_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = false;
        let mut p0 = Repetition {
            kind: RepetitionKind::ZeroOrMore,
            greedy: rug_fuzz_0,
            hir: Box::new(Hir::empty()),
        };
        p0.is_match_empty();
        let _rug_ed_tests_rug_317_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_318 {
    use super::*;
    use crate::hir::HirInfo;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_318_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0b1;
        let mut p0: HirInfo = HirInfo::new();
        p0.bools = rug_fuzz_0;
        debug_assert!(p0.is_all_assertions());
        let _rug_ed_tests_rug_318_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_319 {
    use super::*;
    use crate::hir::HirInfo;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_319_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0b100;
        let mut p0: HirInfo = HirInfo::new();
        p0.bools = rug_fuzz_0;
        let result = p0.is_anchored_start();
        debug_assert_eq!(result, false);
        let _rug_ed_tests_rug_319_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_320 {
    use super::*;
    use crate::hir::HirInfo;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_320_rrrruuuugggg_test_rug = 0;
        let mut v103: HirInfo = HirInfo::new();
        crate::hir::HirInfo::is_line_anchored_end(&v103);
        let _rug_ed_tests_rug_320_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_321 {
    use super::*;
    use crate::hir::HirInfo;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_321_rrrruuuugggg_test_rug = 0;
        let mut p0: HirInfo = HirInfo::new();
        crate::hir::HirInfo::is_any_anchored_start(&mut p0);
        let _rug_ed_tests_rug_321_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_322 {
    use super::*;
    use crate::hir::HirInfo;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_322_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = true;
        let mut p0: HirInfo = HirInfo::new();
        let p1: bool = rug_fuzz_0;
        p0.set_any_anchored_end(p1);
        let _rug_ed_tests_rug_322_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_323 {
    use crate::hir::HirInfo;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_323_rrrruuuugggg_test_rug = 0;
        let mut p0: HirInfo = HirInfo::new();
        crate::hir::HirInfo::is_match_empty(&mut p0);
        let _rug_ed_tests_rug_323_rrrruuuugggg_test_rug = 0;
    }
}
