/*!
Defines an abstract syntax for regular expressions.
*/

use std::cmp::Ordering;
use std::error;
use std::fmt;

pub use ast::visitor::{visit, Visitor};

pub mod parse;
pub mod print;
mod visitor;

/// An error that occurred while parsing a regular expression into an abstract
/// syntax tree.
///
/// Note that note all ASTs represents a valid regular expression. For example,
/// an AST is constructed without error for `\p{Quux}`, but `Quux` is not a
/// valid Unicode property name. That particular error is reported when
/// translating an AST to the high-level intermediate representation (`HIR`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Error {
    /// The kind of error.
    kind: ErrorKind,
    /// The original pattern that the parser generated the error from. Every
    /// span in an error is a valid range into this string.
    pattern: String,
    /// The span of this error.
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

    /// Return an auxiliary span. This span exists only for some errors that
    /// benefit from being able to point to two locations in the original
    /// regular expression. For example, "duplicate" errors will have the
    /// main error position set to the duplicate occurrence while its
    /// auxiliary span will be set to the initial occurrence.
    pub fn auxiliary_span(&self) -> Option<&Span> {
        use self::ErrorKind::*;
        match self.kind {
            FlagDuplicate { ref original } => Some(original),
            FlagRepeatedNegation { ref original, .. } => Some(original),
            GroupNameDuplicate { ref original, .. } => Some(original),
            _ => None,
        }
    }
}

/// The type of an error that occurred while building an AST.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    /// The capturing group limit was exceeded.
    ///
    /// Note that this represents a limit on the total number of capturing
    /// groups in a regex and not necessarily the number of nested capturing
    /// groups. That is, the nest limit can be low and it is still possible for
    /// this error to occur.
    CaptureLimitExceeded,
    /// An invalid escape sequence was found in a character class set.
    ClassEscapeInvalid,
    /// An invalid character class range was found. An invalid range is any
    /// range where the start is greater than the end.
    ClassRangeInvalid,
    /// An invalid range boundary was found in a character class. Range
    /// boundaries must be a single literal codepoint, but this error indicates
    /// that something else was found, such as a nested class.
    ClassRangeLiteral,
    /// An opening `[` was found with no corresponding closing `]`.
    ClassUnclosed,
    /// Note that this error variant is no longer used. Namely, a decimal
    /// number can only appear as a repetition quantifier. When the number
    /// in a repetition quantifier is empty, then it gets its own specialized
    /// error, `RepetitionCountDecimalEmpty`.
    DecimalEmpty,
    /// An invalid decimal number was given where one was expected.
    DecimalInvalid,
    /// A bracketed hex literal was empty.
    EscapeHexEmpty,
    /// A bracketed hex literal did not correspond to a Unicode scalar value.
    EscapeHexInvalid,
    /// An invalid hexadecimal digit was found.
    EscapeHexInvalidDigit,
    /// EOF was found before an escape sequence was completed.
    EscapeUnexpectedEof,
    /// An unrecognized escape sequence.
    EscapeUnrecognized,
    /// A dangling negation was used when setting flags, e.g., `i-`.
    FlagDanglingNegation,
    /// A flag was used twice, e.g., `i-i`.
    FlagDuplicate {
        /// The position of the original flag. The error position
        /// points to the duplicate flag.
        original: Span,
    },
    /// The negation operator was used twice, e.g., `-i-s`.
    FlagRepeatedNegation {
        /// The position of the original negation operator. The error position
        /// points to the duplicate negation operator.
        original: Span,
    },
    /// Expected a flag but got EOF, e.g., `(?`.
    FlagUnexpectedEof,
    /// Unrecognized flag, e.g., `a`.
    FlagUnrecognized,
    /// A duplicate capture name was found.
    GroupNameDuplicate {
        /// The position of the initial occurrence of the capture name. The
        /// error position itself points to the duplicate occurrence.
        original: Span,
    },
    /// A capture group name is empty, e.g., `(?P<>abc)`.
    GroupNameEmpty,
    /// An invalid character was seen for a capture group name. This includes
    /// errors where the first character is a digit (even though subsequent
    /// characters are allowed to be digits).
    GroupNameInvalid,
    /// A closing `>` could not be found for a capture group name.
    GroupNameUnexpectedEof,
    /// An unclosed group, e.g., `(ab`.
    ///
    /// The span of this error corresponds to the unclosed parenthesis.
    GroupUnclosed,
    /// An unopened group, e.g., `ab)`.
    GroupUnopened,
    /// The nest limit was exceeded. The limit stored here is the limit
    /// configured in the parser.
    NestLimitExceeded(u32),
    /// The range provided in a counted repetition operator is invalid. The
    /// range is invalid if the start is greater than the end.
    RepetitionCountInvalid,
    /// An opening `{` was not followed by a valid decimal value.
    /// For example, `x{}` or `x{]}` would fail.
    RepetitionCountDecimalEmpty,
    /// An opening `{` was found with no corresponding closing `}`.
    RepetitionCountUnclosed,
    /// A repetition operator was applied to a missing sub-expression. This
    /// occurs, for example, in the regex consisting of just a `*` or even
    /// `(?i)*`. It is, however, possible to create a repetition operating on
    /// an empty sub-expression. For example, `()*` is still considered valid.
    RepetitionMissing,
    /// The Unicode class is not valid. This typically occurs when a `\p` is
    /// followed by something other than a `{`.
    UnicodeClassInvalid,
    /// When octal support is disabled, this error is produced when an octal
    /// escape is used. The octal escape is assumed to be an invocation of
    /// a backreference, which is the common case.
    UnsupportedBackreference,
    /// When syntax similar to PCRE's look-around is used, this error is
    /// returned. Some example syntaxes that are rejected include, but are
    /// not necessarily limited to, `(?=re)`, `(?!re)`, `(?<=re)` and
    /// `(?<!re)`. Note that all of these syntaxes are otherwise invalid; this
    /// error is used to improve the user experience.
    UnsupportedLookAround,
    /// Hints that destructuring should not be exhaustive.
    ///
    /// This enum may grow additional variants, so this makes sure clients
    /// don't count on exhaustive matching. (Otherwise, adding a new variant
    /// could break existing code.)
    #[doc(hidden)]
    __Nonexhaustive,
}

impl error::Error for Error {
    // TODO: Remove this method entirely on the next breaking semver release.
    #[allow(deprecated)]
    fn description(&self) -> &str {
        use self::ErrorKind::*;
        match self.kind {
            CaptureLimitExceeded => "capture group limit exceeded",
            ClassEscapeInvalid => "invalid escape sequence in character class",
            ClassRangeInvalid => "invalid character class range",
            ClassRangeLiteral => "invalid range boundary, must be a literal",
            ClassUnclosed => "unclosed character class",
            DecimalEmpty => "empty decimal literal",
            DecimalInvalid => "invalid decimal literal",
            EscapeHexEmpty => "empty hexadecimal literal",
            EscapeHexInvalid => "invalid hexadecimal literal",
            EscapeHexInvalidDigit => "invalid hexadecimal digit",
            EscapeUnexpectedEof => "unexpected eof (escape sequence)",
            EscapeUnrecognized => "unrecognized escape sequence",
            FlagDanglingNegation => "dangling flag negation operator",
            FlagDuplicate { .. } => "duplicate flag",
            FlagRepeatedNegation { .. } => "repeated negation",
            FlagUnexpectedEof => "unexpected eof (flag)",
            FlagUnrecognized => "unrecognized flag",
            GroupNameDuplicate { .. } => "duplicate capture group name",
            GroupNameEmpty => "empty capture group name",
            GroupNameInvalid => "invalid capture group name",
            GroupNameUnexpectedEof => "unclosed capture group name",
            GroupUnclosed => "unclosed group",
            GroupUnopened => "unopened group",
            NestLimitExceeded(_) => "nest limit exceeded",
            RepetitionCountInvalid => "invalid repetition count range",
            RepetitionCountUnclosed => "unclosed counted repetition",
            RepetitionMissing => "repetition operator missing expression",
            UnicodeClassInvalid => "invalid Unicode character class",
            UnsupportedBackreference => "backreferences are not supported",
            UnsupportedLookAround => "look-around is not supported",
            _ => unreachable!(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        ::error::Formatter::from(self).fmt(f)
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::ErrorKind::*;
        match *self {
            CaptureLimitExceeded => write!(
                f,
                "exceeded the maximum number of \
                 capturing groups ({})",
                ::std::u32::MAX
            ),
            ClassEscapeInvalid => {
                write!(f, "invalid escape sequence found in character class")
            }
            ClassRangeInvalid => write!(
                f,
                "invalid character class range, \
                 the start must be <= the end"
            ),
            ClassRangeLiteral => {
                write!(f, "invalid range boundary, must be a literal")
            }
            ClassUnclosed => write!(f, "unclosed character class"),
            DecimalEmpty => write!(f, "decimal literal empty"),
            DecimalInvalid => write!(f, "decimal literal invalid"),
            EscapeHexEmpty => write!(f, "hexadecimal literal empty"),
            EscapeHexInvalid => {
                write!(f, "hexadecimal literal is not a Unicode scalar value")
            }
            EscapeHexInvalidDigit => write!(f, "invalid hexadecimal digit"),
            EscapeUnexpectedEof => write!(
                f,
                "incomplete escape sequence, \
                 reached end of pattern prematurely"
            ),
            EscapeUnrecognized => write!(f, "unrecognized escape sequence"),
            FlagDanglingNegation => {
                write!(f, "dangling flag negation operator")
            }
            FlagDuplicate { .. } => write!(f, "duplicate flag"),
            FlagRepeatedNegation { .. } => {
                write!(f, "flag negation operator repeated")
            }
            FlagUnexpectedEof => {
                write!(f, "expected flag but got end of regex")
            }
            FlagUnrecognized => write!(f, "unrecognized flag"),
            GroupNameDuplicate { .. } => {
                write!(f, "duplicate capture group name")
            }
            GroupNameEmpty => write!(f, "empty capture group name"),
            GroupNameInvalid => write!(f, "invalid capture group character"),
            GroupNameUnexpectedEof => write!(f, "unclosed capture group name"),
            GroupUnclosed => write!(f, "unclosed group"),
            GroupUnopened => write!(f, "unopened group"),
            NestLimitExceeded(limit) => write!(
                f,
                "exceed the maximum number of \
                 nested parentheses/brackets ({})",
                limit
            ),
            RepetitionCountInvalid => write!(
                f,
                "invalid repetition count range, \
                 the start must be <= the end"
            ),
            RepetitionCountDecimalEmpty => {
                write!(f, "repetition quantifier expects a valid decimal")
            }
            RepetitionCountUnclosed => {
                write!(f, "unclosed counted repetition")
            }
            RepetitionMissing => {
                write!(f, "repetition operator missing expression")
            }
            UnicodeClassInvalid => {
                write!(f, "invalid Unicode character class")
            }
            UnsupportedBackreference => {
                write!(f, "backreferences are not supported")
            }
            UnsupportedLookAround => write!(
                f,
                "look-around, including look-ahead and look-behind, \
                 is not supported"
            ),
            _ => unreachable!(),
        }
    }
}

/// Span represents the position information of a single AST item.
///
/// All span positions are absolute byte offsets that can be used on the
/// original regular expression that was parsed.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Span {
    /// The start byte offset.
    pub start: Position,
    /// The end byte offset.
    pub end: Position,
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Span({:?}, {:?})", self.start, self.end)
    }
}

impl Ord for Span {
    fn cmp(&self, other: &Span) -> Ordering {
        (&self.start, &self.end).cmp(&(&other.start, &other.end))
    }
}

impl PartialOrd for Span {
    fn partial_cmp(&self, other: &Span) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A single position in a regular expression.
///
/// A position encodes one half of a span, and include the byte offset, line
/// number and column number.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Position {
    /// The absolute offset of this position, starting at `0` from the
    /// beginning of the regular expression pattern string.
    pub offset: usize,
    /// The line number, starting at `1`.
    pub line: usize,
    /// The approximate column number, starting at `1`.
    pub column: usize,
}

impl fmt::Debug for Position {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Position(o: {:?}, l: {:?}, c: {:?})",
            self.offset, self.line, self.column
        )
    }
}

impl Ord for Position {
    fn cmp(&self, other: &Position) -> Ordering {
        self.offset.cmp(&other.offset)
    }
}

impl PartialOrd for Position {
    fn partial_cmp(&self, other: &Position) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Span {
    /// Create a new span with the given positions.
    pub fn new(start: Position, end: Position) -> Span {
        Span { start: start, end: end }
    }

    /// Create a new span using the given position as the start and end.
    pub fn splat(pos: Position) -> Span {
        Span::new(pos, pos)
    }

    /// Create a new span by replacing the starting the position with the one
    /// given.
    pub fn with_start(self, pos: Position) -> Span {
        Span { start: pos, ..self }
    }

    /// Create a new span by replacing the ending the position with the one
    /// given.
    pub fn with_end(self, pos: Position) -> Span {
        Span { end: pos, ..self }
    }

    /// Returns true if and only if this span occurs on a single line.
    pub fn is_one_line(&self) -> bool {
        self.start.line == self.end.line
    }

    /// Returns true if and only if this span is empty. That is, it points to
    /// a single position in the concrete syntax of a regular expression.
    pub fn is_empty(&self) -> bool {
        self.start.offset == self.end.offset
    }
}

impl Position {
    /// Create a new position with the given information.
    ///
    /// `offset` is the absolute offset of the position, starting at `0` from
    /// the beginning of the regular expression pattern string.
    ///
    /// `line` is the line number, starting at `1`.
    ///
    /// `column` is the approximate column number, starting at `1`.
    pub fn new(offset: usize, line: usize, column: usize) -> Position {
        Position { offset: offset, line: line, column: column }
    }
}

/// An abstract syntax tree for a singular expression along with comments
/// found.
///
/// Comments are not stored in the tree itself to avoid complexity. Each
/// comment contains a span of precisely where it occurred in the original
/// regular expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WithComments {
    /// The actual ast.
    pub ast: Ast,
    /// All comments found in the original regular expression.
    pub comments: Vec<Comment>,
}

/// A comment from a regular expression with an associated span.
///
/// A regular expression can only contain comments when the `x` flag is
/// enabled.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Comment {
    /// The span of this comment, including the beginning `#` and ending `\n`.
    pub span: Span,
    /// The comment text, starting with the first character following the `#`
    /// and ending with the last character preceding the `\n`.
    pub comment: String,
}

/// An abstract syntax tree for a single regular expression.
///
/// An `Ast`'s `fmt::Display` implementation uses constant stack space and heap
/// space proportional to the size of the `Ast`.
///
/// This type defines its own destructor that uses constant stack space and
/// heap space proportional to the size of the `Ast`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Ast {
    /// An empty regex that matches everything.
    Empty(Span),
    /// A set of flags, e.g., `(?is)`.
    Flags(SetFlags),
    /// A single character literal, which includes escape sequences.
    Literal(Literal),
    /// The "any character" class.
    Dot(Span),
    /// A single zero-width assertion.
    Assertion(Assertion),
    /// A single character class. This includes all forms of character classes
    /// except for `.`. e.g., `\d`, `\pN`, `[a-z]` and `[[:alpha:]]`.
    Class(Class),
    /// A repetition operator applied to an arbitrary regular expression.
    Repetition(Repetition),
    /// A grouped regular expression.
    Group(Group),
    /// An alternation of regular expressions.
    Alternation(Alternation),
    /// A concatenation of regular expressions.
    Concat(Concat),
}

impl Ast {
    /// Return the span of this abstract syntax tree.
    pub fn span(&self) -> &Span {
        match *self {
            Ast::Empty(ref span) => span,
            Ast::Flags(ref x) => &x.span,
            Ast::Literal(ref x) => &x.span,
            Ast::Dot(ref span) => span,
            Ast::Assertion(ref x) => &x.span,
            Ast::Class(ref x) => x.span(),
            Ast::Repetition(ref x) => &x.span,
            Ast::Group(ref x) => &x.span,
            Ast::Alternation(ref x) => &x.span,
            Ast::Concat(ref x) => &x.span,
        }
    }

    /// Return true if and only if this Ast is empty.
    pub fn is_empty(&self) -> bool {
        match *self {
            Ast::Empty(_) => true,
            _ => false,
        }
    }

    /// Returns true if and only if this AST has any (including possibly empty)
    /// subexpressions.
    fn has_subexprs(&self) -> bool {
        match *self {
            Ast::Empty(_)
            | Ast::Flags(_)
            | Ast::Literal(_)
            | Ast::Dot(_)
            | Ast::Assertion(_) => false,
            Ast::Class(_)
            | Ast::Repetition(_)
            | Ast::Group(_)
            | Ast::Alternation(_)
            | Ast::Concat(_) => true,
        }
    }
}

/// Print a display representation of this Ast.
///
/// This does not preserve any of the original whitespace formatting that may
/// have originally been present in the concrete syntax from which this Ast
/// was generated.
///
/// This implementation uses constant stack space and heap space proportional
/// to the size of the `Ast`.
impl fmt::Display for Ast {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ast::print::Printer;
        Printer::new().print(self, f)
    }
}

/// An alternation of regular expressions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Alternation {
    /// The span of this alternation.
    pub span: Span,
    /// The alternate regular expressions.
    pub asts: Vec<Ast>,
}

impl Alternation {
    /// Return this alternation as an AST.
    ///
    /// If this alternation contains zero ASTs, then Ast::Empty is
    /// returned. If this alternation contains exactly 1 AST, then the
    /// corresponding AST is returned. Otherwise, Ast::Alternation is returned.
    pub fn into_ast(mut self) -> Ast {
        match self.asts.len() {
            0 => Ast::Empty(self.span),
            1 => self.asts.pop().unwrap(),
            _ => Ast::Alternation(self),
        }
    }
}

/// A concatenation of regular expressions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Concat {
    /// The span of this concatenation.
    pub span: Span,
    /// The concatenation regular expressions.
    pub asts: Vec<Ast>,
}

impl Concat {
    /// Return this concatenation as an AST.
    ///
    /// If this concatenation contains zero ASTs, then Ast::Empty is
    /// returned. If this concatenation contains exactly 1 AST, then the
    /// corresponding AST is returned. Otherwise, Ast::Concat is returned.
    pub fn into_ast(mut self) -> Ast {
        match self.asts.len() {
            0 => Ast::Empty(self.span),
            1 => self.asts.pop().unwrap(),
            _ => Ast::Concat(self),
        }
    }
}

/// A single literal expression.
///
/// A literal corresponds to a single Unicode scalar value. Literals may be
/// represented in their literal form, e.g., `a` or in their escaped form,
/// e.g., `\x61`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Literal {
    /// The span of this literal.
    pub span: Span,
    /// The kind of this literal.
    pub kind: LiteralKind,
    /// The Unicode scalar value corresponding to this literal.
    pub c: char,
}

impl Literal {
    /// If this literal was written as a `\x` hex escape, then this returns
    /// the corresponding byte value. Otherwise, this returns `None`.
    pub fn byte(&self) -> Option<u8> {
        let short_hex = LiteralKind::HexFixed(HexLiteralKind::X);
        if self.c as u32 <= 255 && self.kind == short_hex {
            Some(self.c as u8)
        } else {
            None
        }
    }
}

/// The kind of a single literal expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LiteralKind {
    /// The literal is written verbatim, e.g., `a` or `☃`.
    Verbatim,
    /// The literal is written as an escape because it is punctuation, e.g.,
    /// `\*` or `\[`.
    Punctuation,
    /// The literal is written as an octal escape, e.g., `\141`.
    Octal,
    /// The literal is written as a hex code with a fixed number of digits
    /// depending on the type of the escape, e.g., `\x61` or or `\u0061` or
    /// `\U00000061`.
    HexFixed(HexLiteralKind),
    /// The literal is written as a hex code with a bracketed number of
    /// digits. The only restriction is that the bracketed hex code must refer
    /// to a valid Unicode scalar value.
    HexBrace(HexLiteralKind),
    /// The literal is written as a specially recognized escape, e.g., `\f`
    /// or `\n`.
    Special(SpecialLiteralKind),
}

/// The type of a special literal.
///
/// A special literal is a special escape sequence recognized by the regex
/// parser, e.g., `\f` or `\n`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SpecialLiteralKind {
    /// Bell, spelled `\a` (`\x07`).
    Bell,
    /// Form feed, spelled `\f` (`\x0C`).
    FormFeed,
    /// Tab, spelled `\t` (`\x09`).
    Tab,
    /// Line feed, spelled `\n` (`\x0A`).
    LineFeed,
    /// Carriage return, spelled `\r` (`\x0D`).
    CarriageReturn,
    /// Vertical tab, spelled `\v` (`\x0B`).
    VerticalTab,
    /// Space, spelled `\ ` (`\x20`). Note that this can only appear when
    /// parsing in verbose mode.
    Space,
}

/// The type of a Unicode hex literal.
///
/// Note that all variants behave the same when used with brackets. They only
/// differ when used without brackets in the number of hex digits that must
/// follow.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HexLiteralKind {
    /// A `\x` prefix. When used without brackets, this form is limited to
    /// two digits.
    X,
    /// A `\u` prefix. When used without brackets, this form is limited to
    /// four digits.
    UnicodeShort,
    /// A `\U` prefix. When used without brackets, this form is limited to
    /// eight digits.
    UnicodeLong,
}

impl HexLiteralKind {
    /// The number of digits that must be used with this literal form when
    /// used without brackets. When used with brackets, there is no
    /// restriction on the number of digits.
    pub fn digits(&self) -> u32 {
        match *self {
            HexLiteralKind::X => 2,
            HexLiteralKind::UnicodeShort => 4,
            HexLiteralKind::UnicodeLong => 8,
        }
    }
}

/// A single character class expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Class {
    /// A Unicode character class, e.g., `\pL` or `\p{Greek}`.
    Unicode(ClassUnicode),
    /// A perl character class, e.g., `\d` or `\W`.
    Perl(ClassPerl),
    /// A bracketed character class set, which may contain zero or more
    /// character ranges and/or zero or more nested classes. e.g.,
    /// `[a-zA-Z\pL]`.
    Bracketed(ClassBracketed),
}

impl Class {
    /// Return the span of this character class.
    pub fn span(&self) -> &Span {
        match *self {
            Class::Perl(ref x) => &x.span,
            Class::Unicode(ref x) => &x.span,
            Class::Bracketed(ref x) => &x.span,
        }
    }
}

/// A Perl character class.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassPerl {
    /// The span of this class.
    pub span: Span,
    /// The kind of Perl class.
    pub kind: ClassPerlKind,
    /// Whether the class is negated or not. e.g., `\d` is not negated but
    /// `\D` is.
    pub negated: bool,
}

/// The available Perl character classes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClassPerlKind {
    /// Decimal numbers.
    Digit,
    /// Whitespace.
    Space,
    /// Word characters.
    Word,
}

/// An ASCII character class.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassAscii {
    /// The span of this class.
    pub span: Span,
    /// The kind of ASCII class.
    pub kind: ClassAsciiKind,
    /// Whether the class is negated or not. e.g., `[[:alpha:]]` is not negated
    /// but `[[:^alpha:]]` is.
    pub negated: bool,
}

/// The available ASCII character classes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClassAsciiKind {
    /// `[0-9A-Za-z]`
    Alnum,
    /// `[A-Za-z]`
    Alpha,
    /// `[\x00-\x7F]`
    Ascii,
    /// `[ \t]`
    Blank,
    /// `[\x00-\x1F\x7F]`
    Cntrl,
    /// `[0-9]`
    Digit,
    /// `[!-~]`
    Graph,
    /// `[a-z]`
    Lower,
    /// `[ -~]`
    Print,
    /// `[!-/:-@\[-`{-~]`
    Punct,
    /// `[\t\n\v\f\r ]`
    Space,
    /// `[A-Z]`
    Upper,
    /// `[0-9A-Za-z_]`
    Word,
    /// `[0-9A-Fa-f]`
    Xdigit,
}

impl ClassAsciiKind {
    /// Return the corresponding ClassAsciiKind variant for the given name.
    ///
    /// The name given should correspond to the lowercase version of the
    /// variant name. e.g., `cntrl` is the name for `ClassAsciiKind::Cntrl`.
    ///
    /// If no variant with the corresponding name exists, then `None` is
    /// returned.
    pub fn from_name(name: &str) -> Option<ClassAsciiKind> {
        use self::ClassAsciiKind::*;
        match name {
            "alnum" => Some(Alnum),
            "alpha" => Some(Alpha),
            "ascii" => Some(Ascii),
            "blank" => Some(Blank),
            "cntrl" => Some(Cntrl),
            "digit" => Some(Digit),
            "graph" => Some(Graph),
            "lower" => Some(Lower),
            "print" => Some(Print),
            "punct" => Some(Punct),
            "space" => Some(Space),
            "upper" => Some(Upper),
            "word" => Some(Word),
            "xdigit" => Some(Xdigit),
            _ => None,
        }
    }
}

/// A Unicode character class.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassUnicode {
    /// The span of this class.
    pub span: Span,
    /// Whether this class is negated or not.
    ///
    /// Note: be careful when using this attribute. This specifically refers
    /// to whether the class is written as `\p` or `\P`, where the latter
    /// is `negated = true`. However, it also possible to write something like
    /// `\P{scx!=Katakana}` which is actually equivalent to
    /// `\p{scx=Katakana}` and is therefore not actually negated even though
    /// `negated = true` here. To test whether this class is truly negated
    /// or not, use the `is_negated` method.
    pub negated: bool,
    /// The kind of Unicode class.
    pub kind: ClassUnicodeKind,
}

impl ClassUnicode {
    /// Returns true if this class has been negated.
    ///
    /// Note that this takes the Unicode op into account, if it's present.
    /// e.g., `is_negated` for `\P{scx!=Katakana}` will return `false`.
    pub fn is_negated(&self) -> bool {
        match self.kind {
            ClassUnicodeKind::NamedValue {
                op: ClassUnicodeOpKind::NotEqual,
                ..
            } => !self.negated,
            _ => self.negated,
        }
    }
}

/// The available forms of Unicode character classes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClassUnicodeKind {
    /// A one letter abbreviated class, e.g., `\pN`.
    OneLetter(char),
    /// A binary property, general category or script. The string may be
    /// empty.
    Named(String),
    /// A property name and an associated value.
    NamedValue {
        /// The type of Unicode op used to associate `name` with `value`.
        op: ClassUnicodeOpKind,
        /// The property name (which may be empty).
        name: String,
        /// The property value (which may be empty).
        value: String,
    },
}

/// The type of op used in a Unicode character class.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClassUnicodeOpKind {
    /// A property set to a specific value, e.g., `\p{scx=Katakana}`.
    Equal,
    /// A property set to a specific value using a colon, e.g.,
    /// `\p{scx:Katakana}`.
    Colon,
    /// A property that isn't a particular value, e.g., `\p{scx!=Katakana}`.
    NotEqual,
}

impl ClassUnicodeOpKind {
    /// Whether the op is an equality op or not.
    pub fn is_equal(&self) -> bool {
        match *self {
            ClassUnicodeOpKind::Equal | ClassUnicodeOpKind::Colon => true,
            _ => false,
        }
    }
}

/// A bracketed character class, e.g., `[a-z0-9]`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassBracketed {
    /// The span of this class.
    pub span: Span,
    /// Whether this class is negated or not. e.g., `[a]` is not negated but
    /// `[^a]` is.
    pub negated: bool,
    /// The type of this set. A set is either a normal union of things, e.g.,
    /// `[abc]` or a result of applying set operations, e.g., `[\pL--c]`.
    pub kind: ClassSet,
}

/// A character class set.
///
/// This type corresponds to the internal structure of a bracketed character
/// class. That is, every bracketed character is one of two types: a union of
/// items (literals, ranges, other bracketed classes) or a tree of binary set
/// operations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClassSet {
    /// An item, which can be a single literal, range, nested character class
    /// or a union of items.
    Item(ClassSetItem),
    /// A single binary operation (i.e., &&, -- or ~~).
    BinaryOp(ClassSetBinaryOp),
}

impl ClassSet {
    /// Build a set from a union.
    pub fn union(ast: ClassSetUnion) -> ClassSet {
        ClassSet::Item(ClassSetItem::Union(ast))
    }

    /// Return the span of this character class set.
    pub fn span(&self) -> &Span {
        match *self {
            ClassSet::Item(ref x) => x.span(),
            ClassSet::BinaryOp(ref x) => &x.span,
        }
    }

    /// Return true if and only if this class set is empty.
    fn is_empty(&self) -> bool {
        match *self {
            ClassSet::Item(ClassSetItem::Empty(_)) => true,
            _ => false,
        }
    }
}

/// A single component of a character class set.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClassSetItem {
    /// An empty item.
    ///
    /// Note that a bracketed character class cannot contain a single empty
    /// item. Empty items can appear when using one of the binary operators.
    /// For example, `[&&]` is the intersection of two empty classes.
    Empty(Span),
    /// A single literal.
    Literal(Literal),
    /// A range between two literals.
    Range(ClassSetRange),
    /// An ASCII character class, e.g., `[:alnum:]` or `[:punct:]`.
    Ascii(ClassAscii),
    /// A Unicode character class, e.g., `\pL` or `\p{Greek}`.
    Unicode(ClassUnicode),
    /// A perl character class, e.g., `\d` or `\W`.
    Perl(ClassPerl),
    /// A bracketed character class set, which may contain zero or more
    /// character ranges and/or zero or more nested classes. e.g.,
    /// `[a-zA-Z\pL]`.
    Bracketed(Box<ClassBracketed>),
    /// A union of items.
    Union(ClassSetUnion),
}

impl ClassSetItem {
    /// Return the span of this character class set item.
    pub fn span(&self) -> &Span {
        match *self {
            ClassSetItem::Empty(ref span) => span,
            ClassSetItem::Literal(ref x) => &x.span,
            ClassSetItem::Range(ref x) => &x.span,
            ClassSetItem::Ascii(ref x) => &x.span,
            ClassSetItem::Perl(ref x) => &x.span,
            ClassSetItem::Unicode(ref x) => &x.span,
            ClassSetItem::Bracketed(ref x) => &x.span,
            ClassSetItem::Union(ref x) => &x.span,
        }
    }
}

/// A single character class range in a set.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassSetRange {
    /// The span of this range.
    pub span: Span,
    /// The start of this range.
    pub start: Literal,
    /// The end of this range.
    pub end: Literal,
}

impl ClassSetRange {
    /// Returns true if and only if this character class range is valid.
    ///
    /// The only case where a range is invalid is if its start is greater than
    /// its end.
    pub fn is_valid(&self) -> bool {
        self.start.c <= self.end.c
    }
}

/// A union of items inside a character class set.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassSetUnion {
    /// The span of the items in this operation. e.g., the `a-z0-9` in
    /// `[^a-z0-9]`
    pub span: Span,
    /// The sequence of items that make up this union.
    pub items: Vec<ClassSetItem>,
}

impl ClassSetUnion {
    /// Push a new item in this union.
    ///
    /// The ending position of this union's span is updated to the ending
    /// position of the span of the item given. If the union is empty, then
    /// the starting position of this union is set to the starting position
    /// of this item.
    ///
    /// In other words, if you only use this method to add items to a union
    /// and you set the spans on each item correctly, then you should never
    /// need to adjust the span of the union directly.
    pub fn push(&mut self, item: ClassSetItem) {
        if self.items.is_empty() {
            self.span.start = item.span().start;
        }
        self.span.end = item.span().end;
        self.items.push(item);
    }

    /// Return this union as a character class set item.
    ///
    /// If this union contains zero items, then an empty union is
    /// returned. If this concatenation contains exactly 1 item, then the
    /// corresponding item is returned. Otherwise, ClassSetItem::Union is
    /// returned.
    pub fn into_item(mut self) -> ClassSetItem {
        match self.items.len() {
            0 => ClassSetItem::Empty(self.span),
            1 => self.items.pop().unwrap(),
            _ => ClassSetItem::Union(self),
        }
    }
}

/// A Unicode character class set operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassSetBinaryOp {
    /// The span of this operation. e.g., the `a-z--[h-p]` in `[a-z--h-p]`.
    pub span: Span,
    /// The type of this set operation.
    pub kind: ClassSetBinaryOpKind,
    /// The left hand side of the operation.
    pub lhs: Box<ClassSet>,
    /// The right hand side of the operation.
    pub rhs: Box<ClassSet>,
}

/// The type of a Unicode character class set operation.
///
/// Note that this doesn't explicitly represent union since there is no
/// explicit union operator. Concatenation inside a character class corresponds
/// to the union operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClassSetBinaryOpKind {
    /// The intersection of two sets, e.g., `\pN&&[a-z]`.
    Intersection,
    /// The difference of two sets, e.g., `\pN--[0-9]`.
    Difference,
    /// The symmetric difference of two sets. The symmetric difference is the
    /// set of elements belonging to one but not both sets.
    /// e.g., `[\pL~~[:ascii:]]`.
    SymmetricDifference,
}

/// A single zero-width assertion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Assertion {
    /// The span of this assertion.
    pub span: Span,
    /// The assertion kind, e.g., `\b` or `^`.
    pub kind: AssertionKind,
}

/// An assertion kind.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AssertionKind {
    /// `^`
    StartLine,
    /// `$`
    EndLine,
    /// `\A`
    StartText,
    /// `\z`
    EndText,
    /// `\b`
    WordBoundary,
    /// `\B`
    NotWordBoundary,
}

/// A repetition operation applied to a regular expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Repetition {
    /// The span of this operation.
    pub span: Span,
    /// The actual operation.
    pub op: RepetitionOp,
    /// Whether this operation was applied greedily or not.
    pub greedy: bool,
    /// The regular expression under repetition.
    pub ast: Box<Ast>,
}

/// The repetition operator itself.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepetitionOp {
    /// The span of this operator. This includes things like `+`, `*?` and
    /// `{m,n}`.
    pub span: Span,
    /// The type of operation.
    pub kind: RepetitionKind,
}

/// The kind of a repetition operator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RepetitionKind {
    /// `?`
    ZeroOrOne,
    /// `*`
    ZeroOrMore,
    /// `+`
    OneOrMore,
    /// `{m,n}`
    Range(RepetitionRange),
}

/// A range repetition operator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RepetitionRange {
    /// `{m}`
    Exactly(u32),
    /// `{m,}`
    AtLeast(u32),
    /// `{m,n}`
    Bounded(u32, u32),
}

impl RepetitionRange {
    /// Returns true if and only if this repetition range is valid.
    ///
    /// The only case where a repetition range is invalid is if it is bounded
    /// and its start is greater than its end.
    pub fn is_valid(&self) -> bool {
        match *self {
            RepetitionRange::Bounded(s, e) if s > e => false,
            _ => true,
        }
    }
}

/// A grouped regular expression.
///
/// This includes both capturing and non-capturing groups. This does **not**
/// include flag-only groups like `(?is)`, but does contain any group that
/// contains a sub-expression, e.g., `(a)`, `(?P<name>a)`, `(?:a)` and
/// `(?is:a)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Group {
    /// The span of this group.
    pub span: Span,
    /// The kind of this group.
    pub kind: GroupKind,
    /// The regular expression in this group.
    pub ast: Box<Ast>,
}

impl Group {
    /// If this group is non-capturing, then this returns the (possibly empty)
    /// set of flags. Otherwise, `None` is returned.
    pub fn flags(&self) -> Option<&Flags> {
        match self.kind {
            GroupKind::NonCapturing(ref flags) => Some(flags),
            _ => None,
        }
    }

    /// Returns true if and only if this group is capturing.
    pub fn is_capturing(&self) -> bool {
        match self.kind {
            GroupKind::CaptureIndex(_) | GroupKind::CaptureName(_) => true,
            GroupKind::NonCapturing(_) => false,
        }
    }

    /// Returns the capture index of this group, if this is a capturing group.
    ///
    /// This returns a capture index precisely when `is_capturing` is `true`.
    pub fn capture_index(&self) -> Option<u32> {
        match self.kind {
            GroupKind::CaptureIndex(i) => Some(i),
            GroupKind::CaptureName(ref x) => Some(x.index),
            GroupKind::NonCapturing(_) => None,
        }
    }
}

/// The kind of a group.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GroupKind {
    /// `(a)`
    CaptureIndex(u32),
    /// `(?P<name>a)`
    CaptureName(CaptureName),
    /// `(?:a)` and `(?i:a)`
    NonCapturing(Flags),
}

/// A capture name.
///
/// This corresponds to the name itself between the angle brackets in, e.g.,
/// `(?P<foo>expr)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaptureName {
    /// The span of this capture name.
    pub span: Span,
    /// The capture name.
    pub name: String,
    /// The capture index.
    pub index: u32,
}

/// A group of flags that is not applied to a particular regular expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetFlags {
    /// The span of these flags, including the grouping parentheses.
    pub span: Span,
    /// The actual sequence of flags.
    pub flags: Flags,
}

/// A group of flags.
///
/// This corresponds only to the sequence of flags themselves, e.g., `is-u`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Flags {
    /// The span of this group of flags.
    pub span: Span,
    /// A sequence of flag items. Each item is either a flag or a negation
    /// operator.
    pub items: Vec<FlagsItem>,
}

impl Flags {
    /// Add the given item to this sequence of flags.
    ///
    /// If the item was added successfully, then `None` is returned. If the
    /// given item is a duplicate, then `Some(i)` is returned, where
    /// `items[i].kind == item.kind`.
    pub fn add_item(&mut self, item: FlagsItem) -> Option<usize> {
        for (i, x) in self.items.iter().enumerate() {
            if x.kind == item.kind {
                return Some(i);
            }
        }
        self.items.push(item);
        None
    }

    /// Returns the state of the given flag in this set.
    ///
    /// If the given flag is in the set but is negated, then `Some(false)` is
    /// returned.
    ///
    /// If the given flag is in the set and is not negated, then `Some(true)`
    /// is returned.
    ///
    /// Otherwise, `None` is returned.
    pub fn flag_state(&self, flag: Flag) -> Option<bool> {
        let mut negated = false;
        for x in &self.items {
            match x.kind {
                FlagsItemKind::Negation => {
                    negated = true;
                }
                FlagsItemKind::Flag(ref xflag) if xflag == &flag => {
                    return Some(!negated);
                }
                _ => {}
            }
        }
        None
    }
}

/// A single item in a group of flags.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlagsItem {
    /// The span of this item.
    pub span: Span,
    /// The kind of this item.
    pub kind: FlagsItemKind,
}

/// The kind of an item in a group of flags.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FlagsItemKind {
    /// A negation operator applied to all subsequent flags in the enclosing
    /// group.
    Negation,
    /// A single flag in a group.
    Flag(Flag),
}

impl FlagsItemKind {
    /// Returns true if and only if this item is a negation operator.
    pub fn is_negation(&self) -> bool {
        match *self {
            FlagsItemKind::Negation => true,
            _ => false,
        }
    }
}

/// A single flag.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Flag {
    /// `i`
    CaseInsensitive,
    /// `m`
    MultiLine,
    /// `s`
    DotMatchesNewLine,
    /// `U`
    SwapGreed,
    /// `u`
    Unicode,
    /// `x`
    IgnoreWhitespace,
}

/// A custom `Drop` impl is used for `Ast` such that it uses constant stack
/// space but heap space proportional to the depth of the `Ast`.
impl Drop for Ast {
    fn drop(&mut self) {
        use std::mem;

        match *self {
            Ast::Empty(_)
            | Ast::Flags(_)
            | Ast::Literal(_)
            | Ast::Dot(_)
            | Ast::Assertion(_)
            // Classes are recursive, so they get their own Drop impl.
            | Ast::Class(_) => return,
            Ast::Repetition(ref x) if !x.ast.has_subexprs() => return,
            Ast::Group(ref x) if !x.ast.has_subexprs() => return,
            Ast::Alternation(ref x) if x.asts.is_empty() => return,
            Ast::Concat(ref x) if x.asts.is_empty() => return,
            _ => {}
        }

        let empty_span = || Span::splat(Position::new(0, 0, 0));
        let empty_ast = || Ast::Empty(empty_span());
        let mut stack = vec![mem::replace(self, empty_ast())];
        while let Some(mut ast) = stack.pop() {
            match ast {
                Ast::Empty(_)
                | Ast::Flags(_)
                | Ast::Literal(_)
                | Ast::Dot(_)
                | Ast::Assertion(_)
                // Classes are recursive, so they get their own Drop impl.
                | Ast::Class(_) => {}
                Ast::Repetition(ref mut x) => {
                    stack.push(mem::replace(&mut x.ast, empty_ast()));
                }
                Ast::Group(ref mut x) => {
                    stack.push(mem::replace(&mut x.ast, empty_ast()));
                }
                Ast::Alternation(ref mut x) => {
                    stack.extend(x.asts.drain(..));
                }
                Ast::Concat(ref mut x) => {
                    stack.extend(x.asts.drain(..));
                }
            }
        }
    }
}

/// A custom `Drop` impl is used for `ClassSet` such that it uses constant
/// stack space but heap space proportional to the depth of the `ClassSet`.
impl Drop for ClassSet {
    fn drop(&mut self) {
        use std::mem;

        match *self {
            ClassSet::Item(ref item) => match *item {
                ClassSetItem::Empty(_)
                | ClassSetItem::Literal(_)
                | ClassSetItem::Range(_)
                | ClassSetItem::Ascii(_)
                | ClassSetItem::Unicode(_)
                | ClassSetItem::Perl(_) => return,
                ClassSetItem::Bracketed(ref x) => {
                    if x.kind.is_empty() {
                        return;
                    }
                }
                ClassSetItem::Union(ref x) => {
                    if x.items.is_empty() {
                        return;
                    }
                }
            },
            ClassSet::BinaryOp(ref op) => {
                if op.lhs.is_empty() && op.rhs.is_empty() {
                    return;
                }
            }
        }

        let empty_span = || Span::splat(Position::new(0, 0, 0));
        let empty_set = || ClassSet::Item(ClassSetItem::Empty(empty_span()));
        let mut stack = vec![mem::replace(self, empty_set())];
        while let Some(mut set) = stack.pop() {
            match set {
                ClassSet::Item(ref mut item) => match *item {
                    ClassSetItem::Empty(_)
                    | ClassSetItem::Literal(_)
                    | ClassSetItem::Range(_)
                    | ClassSetItem::Ascii(_)
                    | ClassSetItem::Unicode(_)
                    | ClassSetItem::Perl(_) => {}
                    ClassSetItem::Bracketed(ref mut x) => {
                        stack.push(mem::replace(&mut x.kind, empty_set()));
                    }
                    ClassSetItem::Union(ref mut x) => {
                        stack.extend(x.items.drain(..).map(ClassSet::Item));
                    }
                },
                ClassSet::BinaryOp(ref mut op) => {
                    stack.push(mem::replace(&mut op.lhs, empty_set()));
                    stack.push(mem::replace(&mut op.rhs, empty_set()));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // We use a thread with an explicit stack size to test that our destructor
    // for Ast can handle arbitrarily sized expressions in constant stack
    // space. In case we run on a platform without threads (WASM?), we limit
    // this test to Windows/Unix.
    #[test]
    #[cfg(any(unix, windows))]
    fn no_stack_overflow_on_drop() {
        use std::thread;

        let run = || {
            let span = || Span::splat(Position::new(0, 0, 0));
            let mut ast = Ast::Empty(span());
            for i in 0..200 {
                ast = Ast::Group(Group {
                    span: span(),
                    kind: GroupKind::CaptureIndex(i),
                    ast: Box::new(ast),
                });
            }
            assert!(!ast.is_empty());
        };

        // We run our test on a thread with a small stack size so we can
        // force the issue more easily.
        thread::Builder::new()
            .stack_size(1 << 10)
            .spawn(run)
            .unwrap()
            .join()
            .unwrap();
    }
}
#[cfg(test)]
mod tests_llm_16_4 {
    use super::*;

use crate::*;

    #[test]
    fn test_drop() {
        let mut ast = Ast::Empty(Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)));
        drop(&mut ast);
    }
}#[cfg(test)]
mod tests_llm_16_6_llm_16_5 {
    use super::*;

use crate::*;
    use crate::*;

    #[test]
    fn test_drop() {
        let empty_span = || Span::splat(Position::new(0, 0, 0));
        let empty_set = || ClassSet::Item(ClassSetItem::Empty(empty_span()));
        let mut class_set = empty_set();
        drop(&mut class_set);
        let expected_class_set = empty_set();
        assert_eq!(class_set, expected_class_set);
    }
}#[cfg(test)]
mod tests_llm_16_9 {
    use super::*;

use crate::*;
    use std::cmp::Ordering;
    
    #[test]
    fn test_cmp_less() {
        let position1 = Position::new(5, 1, 1);
        let position2 = Position::new(10, 2, 1);
        let result = position1.cmp(&position2);
        assert_eq!(result, Ordering::Less);
    }
    
    #[test]
    fn test_cmp_equal() {
        let position1 = Position::new(5, 1, 1);
        let position2 = Position::new(5, 1, 1);
        let result = position1.cmp(&position2);
        assert_eq!(result, Ordering::Equal);
    }
    
    #[test]
    fn test_cmp_greater() {
        let position1 = Position::new(10, 2, 1);
        let position2 = Position::new(5, 1, 1);
        let result = position1.cmp(&position2);
        assert_eq!(result, Ordering::Greater);
    }
}#[cfg(test)]
mod tests_llm_16_10 {
    use super::*;

use crate::*;
    use std::cmp::Ordering;

    #[test]
    fn test_partial_cmp() {
        let pos1 = Position::new(0, 1, 1);
        let pos2 = Position::new(0, 1, 1);
        let pos3 = Position::new(0, 2, 1);
        let pos4 = Position::new(1, 2, 3);
        
        assert_eq!(pos1.partial_cmp(&pos2), Some(Ordering::Equal));
        assert_eq!(pos1.partial_cmp(&pos3), Some(Ordering::Less));
        assert_eq!(pos3.partial_cmp(&pos1), Some(Ordering::Greater));
        assert_eq!(pos3.partial_cmp(&pos4), Some(Ordering::Equal));
    }
}#[cfg(test)]
mod tests_llm_16_11 {
    use super::*;

use crate::*;

    #[test]
    fn test_cmp() {
        let position1 = Position::new(0, 1, 1);
        let position2 = Position::new(1, 2, 1);
        let position3 = Position::new(2, 2, 2);

        let span1 = Span::new(position1, position2);
        let span2 = Span::new(position2, position3);

        assert_eq!(span1.cmp(&span2), Ordering::Less);
        assert_eq!(span2.cmp(&span1), Ordering::Greater);
        assert_eq!(span1.cmp(&span1), Ordering::Equal);
    }
}#[cfg(test)]
mod tests_llm_16_12 {
    use super::*;

use crate::*;
    use std::cmp::Ordering;

    #[test]
    fn test_partial_cmp() {
        let position1 = Position::new(0, 1, 1);
        let position2 = Position::new(5, 1, 6);
        let span1 = Span::new(position1.clone(), position2.clone());
        let span2 = Span::new(position1.clone(), position2.clone());

        assert_eq!(span1.partial_cmp(&span2), Some(Ordering::Equal));
    }
}#[cfg(test)]
mod tests_llm_16_109 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_into_ast_empty() {
        let alternation = Alternation {
            span: Span {
                start: Position::new(0, 0, 0),
                end: Position::new(0, 0, 0),
            },
            asts: vec![],
        };
        let ast = alternation.into_ast();
        assert_eq!(ast, Ast::Empty(Span {
            start: Position::new(0, 0, 0),
            end: Position::new(0, 0, 0),
        }));
    }
    
    #[test]
    fn test_into_ast_single_ast() {
        let asts = vec![
            Ast::Literal(Literal {
                span: Span {
                    start: Position::new(0, 0, 0),
                    end: Position::new(0, 0, 0),
                },
                kind: LiteralKind::Verbatim,
                c: 'a',
            }),
        ];
        let alternation = Alternation {
            span: Span {
                start: Position::new(0, 0, 0),
                end: Position::new(0, 0, 0),
            },
            asts: asts,
        };
        let ast = alternation.into_ast();
        assert_eq!(ast, Ast::Literal(Literal {
            span: Span {
                start: Position::new(0, 0, 0),
                end: Position::new(0, 0, 0),
            },
            kind: LiteralKind::Verbatim,
            c: 'a',
        }));
    }
    
    #[test]
    fn test_into_ast_multiple_ast() {
        let asts = vec![
            Ast::Literal(Literal {
                span: Span {
                    start: Position::new(0, 0, 0),
                    end: Position::new(0, 0, 0),
                },
                kind: LiteralKind::Verbatim,
                c: 'a',
            }),
            Ast::Literal(Literal {
                span: Span {
                    start: Position::new(0, 0, 0),
                    end: Position::new(0, 0, 0),
                },
                kind: LiteralKind::Verbatim,
                c: 'b',
            }),
        ];
        let alternation = Alternation {
            span: Span {
                start: Position::new(0, 0, 0),
                end: Position::new(0, 0, 0),
            },
            asts: asts,
        };
        let ast = alternation.into_ast();
        assert_eq!(ast, Ast::Alternation(Alternation {
            span: Span {
                start: Position::new(0, 0, 0),
                end: Position::new(0, 0, 0),
            },
            asts: vec![
                Ast::Literal(Literal {
                    span: Span {
                        start: Position::new(0, 0, 0),
                        end: Position::new(0, 0, 0),
                    },
                    kind: LiteralKind::Verbatim,
                    c: 'a',
                }),
                Ast::Literal(Literal {
                    span: Span {
                        start: Position::new(0, 0, 0),
                        end: Position::new(0, 0, 0),
                    },
                    kind: LiteralKind::Verbatim,
                    c: 'b',
                }),
            ],
        }));
    }
}#[cfg(test)]
mod tests_llm_16_113 {
    use super::*;

use crate::*;
    use ast::LiteralKind;
    use ast::RepetitionKind;
    use ast::RepetitionRange;

    #[test]
    fn test_is_empty() {
        let ast_empty = Ast::Empty(Span::splat(Position::new(0, 0, 0)));
        assert_eq!(ast_empty.is_empty(), true);

        let ast_non_empty = Ast::Repetition(Repetition {
            span: Span::splat(Position::new(0, 0, 0)),
            op: RepetitionOp {
                span: Span::splat(Position::new(0, 0, 0)),
                kind: RepetitionKind::Range(RepetitionRange::Exactly(2)),
            },
            greedy: true,
            ast: Box::new(Ast::Empty(Span::splat(Position::new(0, 0, 0)))),
        });
        assert_eq!(ast_non_empty.is_empty(), false);
    }
}#[cfg(test)]
mod tests_llm_16_116 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_class_span_perl() {
        let span = Span::splat(Position::new(0, 0, 0));
        let class = Class::Perl(ClassPerl {
            span: span,
            kind: ClassPerlKind::Digit,
            negated: false,
        });
        let result = class.span();
        assert_eq!(result, &span);
    }
    
    #[test]
    fn test_class_span_unicode() {
        let span = Span::splat(Position::new(0, 0, 0));
        let class = Class::Unicode(ClassUnicode {
            span: span,
            kind: ClassUnicodeKind::OneLetter('L'),
            negated: true,
        });
        let result = class.span();
        assert_eq!(result, &span);
    }
    
    #[test]
    fn test_class_span_bracketed() {
        let span = Span::splat(Position::new(0, 0, 0));
        let class = Class::Bracketed(ClassBracketed {
            span: span,
            negated: false,
            kind: ClassSet::Item(ClassSetItem::Literal(Literal {
                span: span,
                kind: LiteralKind::Verbatim,
                c: 'a',
            })),
        });
        let result = class.span();
        assert_eq!(result, &span);
    }
}#[cfg(test)]
mod tests_llm_16_118 {
    use super::*;

use crate::*;

    #[test]
    fn test_from_name() {
        assert_eq!(ClassAsciiKind::from_name("alnum"), Some(ClassAsciiKind::Alnum));
        assert_eq!(ClassAsciiKind::from_name("alpha"), Some(ClassAsciiKind::Alpha));
        assert_eq!(ClassAsciiKind::from_name("ascii"), Some(ClassAsciiKind::Ascii));
        assert_eq!(ClassAsciiKind::from_name("blank"), Some(ClassAsciiKind::Blank));
        assert_eq!(ClassAsciiKind::from_name("cntrl"), Some(ClassAsciiKind::Cntrl));
        assert_eq!(ClassAsciiKind::from_name("digit"), Some(ClassAsciiKind::Digit));
        assert_eq!(ClassAsciiKind::from_name("graph"), Some(ClassAsciiKind::Graph));
        assert_eq!(ClassAsciiKind::from_name("lower"), Some(ClassAsciiKind::Lower));
        assert_eq!(ClassAsciiKind::from_name("print"), Some(ClassAsciiKind::Print));
        assert_eq!(ClassAsciiKind::from_name("punct"), Some(ClassAsciiKind::Punct));
        assert_eq!(ClassAsciiKind::from_name("space"), Some(ClassAsciiKind::Space));
        assert_eq!(ClassAsciiKind::from_name("upper"), Some(ClassAsciiKind::Upper));
        assert_eq!(ClassAsciiKind::from_name("word"), Some(ClassAsciiKind::Word));
        assert_eq!(ClassAsciiKind::from_name("xdigit"), Some(ClassAsciiKind::Xdigit));
        assert_eq!(ClassAsciiKind::from_name("invalid"), None);
    }
}#[cfg(test)]
mod tests_llm_16_119 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_is_empty_returns_true_for_empty_class_set_item() {
        let class_set = ClassSet::Item(ClassSetItem::Empty(Span::new(
            Position::new(0, 0, 0),
            Position::new(0, 0, 0),
        )));
        
        assert!(class_set.is_empty());
    }
    
    #[test]
    fn test_is_empty_returns_false_for_non_empty_class_set_item() {
        let class_set = ClassSet::Item(ClassSetItem::Literal(Literal {
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            kind: LiteralKind::Verbatim,
            c: 'a',
        }));
        
        assert!(!class_set.is_empty());
    }
    
    #[test]
    fn test_is_empty_returns_false_for_class_set_binary_op() {
        let class_set = ClassSet::BinaryOp(ClassSetBinaryOp {
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            kind: ClassSetBinaryOpKind::Intersection,
            lhs: Box::new(ClassSet::Item(ClassSetItem::Literal(Literal {
                span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
                kind: LiteralKind::Verbatim,
                c: 'a',
            }))),
            rhs: Box::new(ClassSet::Item(ClassSetItem::Literal(Literal {
                span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
                kind: LiteralKind::Verbatim,
                c: 'b',
            }))),
        });
        
        assert!(!class_set.is_empty());
    }
}#[test]
fn test_class_ascii_kind_from_name() {
    assert_eq!(ClassAsciiKind::from_name("alnum"), Some(ClassAsciiKind::Alnum));
    assert_eq!(ClassAsciiKind::from_name("alpha"), Some(ClassAsciiKind::Alpha));
    assert_eq!(ClassAsciiKind::from_name("ascii"), Some(ClassAsciiKind::Ascii));
    assert_eq!(ClassAsciiKind::from_name("blank"), Some(ClassAsciiKind::Blank));
    assert_eq!(ClassAsciiKind::from_name("cntrl"), Some(ClassAsciiKind::Cntrl));
    assert_eq!(ClassAsciiKind::from_name("digit"), Some(ClassAsciiKind::Digit));
    assert_eq!(ClassAsciiKind::from_name("graph"), Some(ClassAsciiKind::Graph));
    assert_eq!(ClassAsciiKind::from_name("lower"), Some(ClassAsciiKind::Lower));
    assert_eq!(ClassAsciiKind::from_name("print"), Some(ClassAsciiKind::Print));
    assert_eq!(ClassAsciiKind::from_name("punct"), Some(ClassAsciiKind::Punct));
    assert_eq!(ClassAsciiKind::from_name("space"), Some(ClassAsciiKind::Space));
    assert_eq!(ClassAsciiKind::from_name("upper"), Some(ClassAsciiKind::Upper));
    assert_eq!(ClassAsciiKind::from_name("word"), Some(ClassAsciiKind::Word));
    assert_eq!(ClassAsciiKind::from_name("xdigit"), Some(ClassAsciiKind::Xdigit));
    assert_eq!(ClassAsciiKind::from_name("invalid"), None);
}

#[test]
fn test_class_set_span() {
    let span = Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0));
    let class_set = ClassSet::Item(ClassSetItem::Literal(Literal {
        span: span.clone(),
        kind: LiteralKind::Verbatim,
        c: 'a',
    }));
    assert_eq!(class_set.span(), &span);

    let binary_op_span = Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0));
    let binary_op_class_set = ClassSet::BinaryOp(ClassSetBinaryOp {
        span: binary_op_span.clone(),
        kind: ClassSetBinaryOpKind::Intersection,
        lhs: Box::new(class_set.clone()),
        rhs: Box::new(class_set),
    });
    assert_eq!(binary_op_class_set.span(), &binary_op_span);
}#[cfg(test)]
mod tests_llm_16_123 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_union() {
        let ast = ClassSetUnion {
            span: Span::new(
                Position::new(0, 0, 0),
                Position::new(0, 0, 0),
            ),
            items: vec![
                ClassSetItem::Literal(Literal {
                    span: Span::new(
                        Position::new(0, 0, 0),
                        Position::new(0, 0, 0),
                    ),
                    kind: LiteralKind::Verbatim,
                    c: 'a',
                }),
                ClassSetItem::Literal(Literal {
                    span: Span::new(
                        Position::new(0, 0, 0),
                        Position::new(0, 0, 0),
                    ),
                    kind: LiteralKind::Verbatim,
                    c: 'b',
                }),
            ],
        };
        
        let expected = ClassSet::Item(ClassSetItem::Union(ast.clone()));
        let result = ClassSet::union(ast.clone());
        
        assert_eq!(result, expected);
    }
}#[cfg(test)]
mod tests_llm_16_126 {
    use super::*;

use crate::*;
    use ast::{
        ClassSetRange, HexLiteralKind, Literal, LiteralKind, Position, Span, SpecialLiteralKind,
    };

    #[test]
    fn test_is_valid() {
        // Create test data
        let start = Literal {
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            kind: LiteralKind::Verbatim,
            c: 'a',
        };
        let end = Literal {
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            kind: LiteralKind::Verbatim,
            c: 'b',
        };
        let range = ClassSetRange {
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            start: start,
            end: end,
        };

        // Test is_valid method
        assert!(range.is_valid());
    }
}#[cfg(test)]
mod tests_llm_16_128_llm_16_127 {
    use super::*;

use crate::*;

    #[test]
    fn test_into_item_empty() {
        let span = Span::new(Position::new(0, 1, 1), Position::new(0, 1, 1));
        let mut union = ClassSetUnion {
            span: span,
            items: Vec::new(),
        };
        assert_eq!(union.into_item(), ClassSetItem::Empty(span));
    }
    
    #[test]
    fn test_into_item_single() {
        let span = Span::new(Position::new(0, 1, 1), Position::new(0, 1, 1));
        let item = ClassSetItem::Literal(Literal {
            span: span,
            kind: LiteralKind::Verbatim,
            c: 'a',
        });
        let mut union = ClassSetUnion {
            span: span,
            items: vec![item.clone()],
        };
        assert_eq!(union.into_item(), item);
    }
    
    #[test]
    fn test_into_item_union() {
        let span = Span::new(Position::new(0, 1, 1), Position::new(0, 1, 1));
        let item1 = ClassSetItem::Literal(Literal {
            span: span,
            kind: LiteralKind::Verbatim,
            c: 'a',
        });
        let item2 = ClassSetItem::Literal(Literal {
            span: span,
            kind: LiteralKind::Verbatim,
            c: 'b',
        });
        let mut union = ClassSetUnion {
            span: span,
            items: vec![item1.clone(), item2.clone()],
        };
        assert_eq!(union.clone().into_item(), ClassSetItem::Union(union));
    }
}#[cfg(test)]
mod tests_llm_16_129 {
    use super::*;

use crate::*;

    #[test]
    fn test_push() {
        let mut union = ClassSetUnion {
            span: Span::new(
                Position::new(0, 1, 1),
                Position::new(0, 1, 1)
            ),
            items: Vec::new(),
        };
        let item = ClassSetItem::Literal(Literal {
            span: Span::new(
                Position::new(0, 1, 1),
                Position::new(0, 1, 1)
            ),
            kind: LiteralKind::Verbatim,
            c: 'a',
        });
        union.push(item.clone());

        assert_eq!(union.span.start, item.span().start);
        assert_eq!(union.span.end, item.span().end);
        assert_eq!(union.items, vec![item]);
    }
}#[cfg(test)]
mod tests_llm_16_130 {
    use super::*;

use crate::*;

    #[test]
    fn test_is_negated_not_negated() {
        let class_unicode = ClassUnicode {
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            negated: false,
            kind: ClassUnicodeKind::NamedValue {
                op: ClassUnicodeOpKind::Equal,
                name: String::from("scx"),
                value: String::from("Katakana"),
            },
        };
        assert_eq!(class_unicode.is_negated(), false);
    }

    #[test]
    fn test_is_negated_negated() {
        let class_unicode = ClassUnicode {
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            negated: true,
            kind: ClassUnicodeKind::NamedValue {
                op: ClassUnicodeOpKind::NotEqual,
                name: String::from("scx"),
                value: String::from("Katakana"),
            },
        };
        assert_eq!(class_unicode.is_negated(), true);
    }

    #[test]
    fn test_is_negated_not_negated_with_colon() {
        let class_unicode = ClassUnicode {
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            negated: false,
            kind: ClassUnicodeKind::NamedValue {
                op: ClassUnicodeOpKind::Colon,
                name: String::from("scx"),
                value: String::from("Katakana"),
            },
        };
        assert_eq!(class_unicode.is_negated(), false);
    }

    #[test]
    fn test_is_negated_not_negated_one_letter() {
        let class_unicode = ClassUnicode {
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            negated: false,
            kind: ClassUnicodeKind::OneLetter('N'),
        };
        assert_eq!(class_unicode.is_negated(), false);
    }

    #[test]
    fn test_is_negated_negated_one_letter() {
        let class_unicode = ClassUnicode {
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            negated: true,
            kind: ClassUnicodeKind::OneLetter('N'),
        };
        assert_eq!(class_unicode.is_negated(), true);
    }
}#[cfg(test)]
mod tests_llm_16_131 {
    use super::*;

use crate::*;

    #[test]
    fn test_is_equal_equal() {
        let op_kind = ClassUnicodeOpKind::Equal;
        assert_eq!(op_kind.is_equal(), true);
    }

    #[test]
    fn test_is_equal_colon() {
        let op_kind = ClassUnicodeOpKind::Colon;
        assert_eq!(op_kind.is_equal(), true);
    }

    #[test]
    fn test_is_equal_not_equal() {
        let op_kind = ClassUnicodeOpKind::NotEqual;
        assert_eq!(op_kind.is_equal(), false);
    }
}#[cfg(test)]
mod tests_llm_16_135_llm_16_134 {
    use super::*;

use crate::*;
    use std::error::Error;

    #[test]
    fn test_auxiliary_span() {
        let span = Span::new(Position::new(0, 1, 1), Position::new(5, 1, 6));
        let error = ast::Error {
            kind: ast::ErrorKind::FlagDuplicate { original: span },
            pattern: String::from("ab"),
            span: span,
        };
        assert_eq!(error.auxiliary_span(), Some(&span));
    }
}#[cfg(test)]
mod tests_llm_16_142 {
    use super::*;

use crate::*;
    use ast::{Flag, Flags, FlagsItem, FlagsItemKind, Position, Span};

    #[test]
    fn test_add_item() {
        let mut flags = Flags {
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            items: Vec::new(),
        };
        let item = FlagsItem {
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            kind: FlagsItemKind::Flag(Flag::CaseInsensitive),
        };
        assert_eq!(flags.add_item(item), None);
    }
}#[cfg(test)]
mod tests_llm_16_143 {
    use super::*;

use crate::*;

    #[test]
    fn test_flag_state_returns_none_if_flag_not_in_set() {
        let flags = Flags {
            span: Span::new(Position::new(0, 1, 1), Position::new(0, 1, 1)),
            items: vec![
                FlagsItem {
                    kind: FlagsItemKind::Negation,
                    span: Span::new(Position::new(0, 1, 1), Position::new(0, 1, 1)),
                },
                FlagsItem {
                    kind: FlagsItemKind::Flag(Flag::MultiLine),
                    span: Span::new(Position::new(0, 1, 1), Position::new(0, 1, 1)),
                },
            ],
        };
        let flag = Flag::CaseInsensitive;
        let result = flags.flag_state(flag);
        assert_eq!(result, None);
    }

    #[test]
    fn test_flag_state_returns_false_if_flag_in_set_and_negated() {
        let flags = Flags {
            span: Span::new(Position::new(0, 1, 1), Position::new(0, 1, 1)),
            items: vec![
                FlagsItem {
                    kind: FlagsItemKind::Negation,
                    span: Span::new(Position::new(0, 1, 1), Position::new(0, 1, 1)),
                },
                FlagsItem {
                    kind: FlagsItemKind::Flag(Flag::CaseInsensitive),
                    span: Span::new(Position::new(0, 1, 1), Position::new(0, 1, 1)),
                },
            ],
        };
        let flag = Flag::CaseInsensitive;
        let result = flags.flag_state(flag);
        assert_eq!(result, Some(false));
    }

    #[test]
    fn test_flag_state_returns_true_if_flag_in_set_and_not_negated() {
        let flags = Flags {
            span: Span::new(Position::new(0, 1, 1), Position::new(0, 1, 1)),
            items: vec![
                FlagsItem {
                    kind: FlagsItemKind::Flag(Flag::CaseInsensitive),
                    span: Span::new(Position::new(0, 1, 1), Position::new(0, 1, 1)),
                },
                FlagsItem {
                    kind: FlagsItemKind::Flag(Flag::MultiLine),
                    span: Span::new(Position::new(0, 1, 1), Position::new(0, 1, 1)),
                },
            ],
        };
        let flag = Flag::CaseInsensitive;
        let result = flags.flag_state(flag);
        assert_eq!(result, Some(true));
    }
}#[cfg(test)]
mod tests_llm_16_144 {
    use super::*;

use crate::*;
    use crate::ast::{Flag, FlagsItemKind};

    #[test]
    fn test_is_negation_for_negation() {
        let negation = FlagsItemKind::Negation;
        assert!(negation.is_negation());
    }

    #[test]
    fn test_is_negation_for_flag() {
        let flag = FlagsItemKind::Flag(Flag::CaseInsensitive);
        assert!(!flag.is_negation());
    }
}#[cfg(test)]
mod tests_llm_16_151 {
    use super::*;

use crate::*;
    use ast::HexLiteralKind;

    #[test]
    fn test_digits() {
        assert_eq!(HexLiteralKind::X.digits(), 2);
        assert_eq!(HexLiteralKind::UnicodeShort.digits(), 4);
        assert_eq!(HexLiteralKind::UnicodeLong.digits(), 8);
    }
}#[cfg(test)]
mod tests_llm_16_152 {
    use super::*;

use crate::*;

    #[test]
    fn test_byte_hex_fixed() {
        let literal = Literal {
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            kind: LiteralKind::HexFixed(HexLiteralKind::X),
            c: 'a',
        };
        assert_eq!(literal.byte(), Some(0x61));
    }

    #[test]
    fn test_byte_not_hex_fixed() {
        let literal = Literal {
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            kind: LiteralKind::Verbatim,
            c: 'a',
        };
        assert_eq!(literal.byte(), None);
    }

    #[test]
    fn test_byte_not_byte() {
        let literal = Literal {
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            kind: LiteralKind::HexFixed(HexLiteralKind::X),
            c: 'ü',
        };
        assert_eq!(literal.byte(), None);
    }

    #[test]
    fn test_byte_not_hex_fixed_digits() {
        let literal = Literal {
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            kind: LiteralKind::HexFixed(HexLiteralKind::UnicodeShort),
            c: 'a',
        };
        assert_eq!(literal.byte(), None);
    }
}#[cfg(test)]
mod tests_llm_16_153 {
    use super::*;

use crate::*;

    #[test]
    fn test_new_position() {
        let position = Position::new(0, 1, 1);

        assert_eq!(position.offset, 0);
        assert_eq!(position.line, 1);
        assert_eq!(position.column, 1);
    }
}#[cfg(test)]
mod tests_llm_16_154 {
    use super::*;

use crate::*;
    use ast::RepetitionRange;

    #[test]
    fn test_repetition_range_is_valid_bounded_valid() {
        let repetition_range = RepetitionRange::Bounded(2, 5);
        assert_eq!(repetition_range.is_valid(), true);
    }

    #[test]
    fn test_repetition_range_is_valid_bounded_invalid() {
        let repetition_range = RepetitionRange::Bounded(5, 2);
        assert_eq!(repetition_range.is_valid(), false);
    }

    #[test]
    fn test_repetition_range_is_valid_exactly() {
        let repetition_range = RepetitionRange::Exactly(10);
        assert_eq!(repetition_range.is_valid(), true);
    }

    #[test]
    fn test_repetition_range_is_valid_at_least() {
        let repetition_range = RepetitionRange::AtLeast(5);
        assert_eq!(repetition_range.is_valid(), true);
    }
}#[cfg(test)]
mod tests_llm_16_155 {
    use super::*;

use crate::*;

    #[test]
    fn test_is_empty() {
        let start = Position::new(0, 1, 1);
        let end = Position::new(0, 1, 1);
        let span = Span::new(start, end);
        assert!(span.is_empty());

        let start = Position::new(0, 1, 1);
        let end = Position::new(1, 1, 2);
        let span = Span::new(start, end);
        assert!(!span.is_empty());
    }
}#[cfg(test)]
mod tests_llm_16_156 {
    use crate::{ast::Position, ast::Span};

    #[test]
    fn test_is_one_line() {
        let position1 = Position::new(0, 1, 1);
        let position2 = Position::new(5, 1, 6);
        let span = Span::new(position1, position2);
        assert_eq!(span.is_one_line(), true);
    }
}#[cfg(test)]
mod tests_llm_16_157 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_new_span() {
        let start = Position::new(0, 1, 1);
        let end = Position::new(5, 1, 6);
        let span = Span::new(start, end);
        
        assert_eq!(span.start.offset, 0);
        assert_eq!(span.start.line, 1);
        assert_eq!(span.start.column, 1);
        
        assert_eq!(span.end.offset, 5);
        assert_eq!(span.end.line, 1);
        assert_eq!(span.end.column, 6);
    }
    
    #[test]
    fn test_splat_span() {
        let pos = Position::new(0, 1, 1);
        let span = Span::splat(pos);
        
        assert_eq!(span.start.offset, 0);
        assert_eq!(span.start.line, 1);
        assert_eq!(span.start.column, 1);
        
        assert_eq!(span.end.offset, 0);
        assert_eq!(span.end.line, 1);
        assert_eq!(span.end.column, 1);
    }
    
    #[test]
    fn test_with_start_span() {
        let start = Position::new(0, 1, 1);
        let end = Position::new(5, 1, 6);
        let span = Span::new(start, end);
        
        let new_start = Position::new(1, 1, 2);
        let new_span = span.with_start(new_start);
        
        assert_eq!(new_span.start.offset, 1);
        assert_eq!(new_span.start.line, 1);
        assert_eq!(new_span.start.column, 2);
        
        assert_eq!(new_span.end.offset, 5);
        assert_eq!(new_span.end.line, 1);
        assert_eq!(new_span.end.column, 6);
    }
    
    #[test]
    fn test_with_end_span() {
        let start = Position::new(0, 1, 1);
        let end = Position::new(5, 1, 6);
        let span = Span::new(start, end);
        
        let new_end = Position::new(6, 1, 7);
        let new_span = span.with_end(new_end);
        
        assert_eq!(new_span.start.offset, 0);
        assert_eq!(new_span.start.line, 1);
        assert_eq!(new_span.start.column, 1);
        
        assert_eq!(new_span.end.offset, 6);
        assert_eq!(new_span.end.line, 1);
        assert_eq!(new_span.end.column, 7);
    }
    
    #[test]
    fn test_is_one_line() {
        let start = Position::new(0, 1, 1);
        let end = Position::new(5, 1, 6);
        let span = Span::new(start, end);
        
        assert_eq!(span.is_one_line(), true);
        
        let start = Position::new(0, 1, 1);
        let end = Position::new(5, 2, 6);
        let span = Span::new(start, end);
        
        assert_eq!(span.is_one_line(), false);
    }
    
    #[test]
    fn test_is_empty() {
        let start = Position::new(0, 1, 1);
        let end = Position::new(0, 1, 1);
        let span = Span::new(start, end);
        
        assert_eq!(span.is_empty(), true);
        
        let start = Position::new(0, 1, 1);
        let end = Position::new(5, 1, 6);
        let span = Span::new(start, end);
        
        assert_eq!(span.is_empty(), false);
    }
}#[cfg(test)]
mod tests_llm_16_158 {
    use super::*;

use crate::*;
    use crate::ast::{Span, Position};

    #[test]
    fn test_splat() {
        let pos = Position::new(10, 5, 3);
        let expected = Span::new(pos, pos);
        let result = Span::splat(pos);
        assert_eq!(result, expected);
    }
}#[cfg(test)]
mod tests_llm_16_159 {
    use super::*;

use crate::*;
    use crate::ast::{Span, Position};

    #[test]
    fn test_with_end() {
        let start = Position::new(0, 1, 1);
        let end = Position::new(5, 1, 6);
        let span = Span::new(start, end);
        
        let new_end = Position::new(10, 1, 11);
        let expected = Span::new(start, new_end);
        let result = span.with_end(new_end);
        
        assert_eq!(result, expected);
    }
}#[cfg(test)]
mod tests_llm_16_160 {
    use crate::ast::{Span, Position};

    #[test]
    fn test_with_start() {
        // Create a test span
        let start = Position::new(0, 1, 1);
        let end = Position::new(5, 1, 6);
        let span = Span::new(start, end);

        // Create a new start position
        let new_start = Position::new(10, 2, 1);

        // Call the with_start function
        let new_span = span.with_start(new_start);

        // Check the result
        assert_eq!(new_span.start.offset, 10);
        assert_eq!(new_span.start.line, 2);
        assert_eq!(new_span.start.column, 1);
        assert_eq!(new_span.end.offset, 5);
        assert_eq!(new_span.end.line, 1);
        assert_eq!(new_span.end.column, 6);
    }
}#[cfg(test)]
mod tests_rug_623 {
    use super::*;
    use crate::ast::{Error, ErrorKind};
    use crate::span::Span;

    #[test]
    fn test_rug() {
        let mut p0 = Error {
            kind: ErrorKind::FlagDuplicate {
                original: Span { start: 0, end: 5 },
            },
            pattern: String::from("abcde"),
            span: Span { start: 0, end: 5 },
        };

        <ast::Error>::kind(&p0);
    }
}#[cfg(test)]
mod tests_rug_624 {
    use super::*;
    use crate::ast::{Error, ErrorKind};
    use crate::span::Span;

    #[test]
    fn test_pattern() {
        let mut p0 = Error {
            kind: ErrorKind::FlagDuplicate { original: Span { start: 0, end: 5 } },
            pattern: String::from("abcde"),
            span: Span { start: 0, end: 5 },
        };

        <Error>::pattern(&p0);
    }
}
#[cfg(test)]
mod tests_rug_625 {
    use super::*;
    use crate::ast::{Span, Error, ErrorKind};

    #[test]
    fn test_rug() {
        let mut p0 = Error {
            kind: ErrorKind::FlagDuplicate { original: Span { start: 0, end: 5 } },
            pattern: String::from("abcde"),
            span: Span { start: 0, end: 5 },
        };

       <ast::Error>::span(&p0);

    }
}
#[cfg(test)]
mod tests_rug_626 {
    use super::*;
    use crate::ast::{Error, ErrorKind};
    use crate::span::Span;

    #[test]
    fn test_description() {
        let mut p0 = Error {
            kind: ErrorKind::FlagDuplicate { original: Span { start: 0, end: 5 } },
            pattern: String::from("abcde"),
            span: Span { start: 0, end: 5 },
        };

        <ast::Error as std::error::Error>::description(&p0);
    }
}#[cfg(test)]
mod tests_rug_627 {
    use super::*;
    use crate::ast::{Ast, Concat};

    #[test]
    fn test_rug() {
        let mut p0 = Ast::Concat(Concat {
            asts: vec![], // fill in the desired value for the `asts` field
            span: Span::splat(Position::new(0, 0, 0)), // fill in the desired `Span` value
        });

        <ast::Ast>::span(&p0);
    }
}#[cfg(test)]
mod tests_rug_628 {
    use super::*;
    use crate::ast::{Ast, Concat};

    #[test]
    fn test_rug() {
        // construct the variable
        let p0 = Ast::Concat(Concat {
            asts: vec![],
            span: Span::splat(Position::new(0, 0, 0)),
        });

        // call the target function
        <Ast>::has_subexprs(&p0);
    }
}                
#[cfg(test)]
mod tests_rug_629 {
    use super::*;
    use crate::ast::{Ast, Concat};

    #[test]
    fn test_rug() {
        let span = Span { start: 0, end: 0 }; // create a sample Span
        let asts = vec![Ast::Empty(span)]; // create a sample Vec<Ast>
        let p0 = Concat { span, asts }; // create the local variable p0 with type ast::Concat

        <ast::Concat>::into_ast(p0);
    }
}
                            
#[cfg(test)]
mod tests_rug_630 {
    use super::*;
    use crate::ast::{ClassSetItem, ClassSetRange, Literal, Span};

    #[test]
    fn test_rug() {
        let mut p0 = ClassSetItem::Range(ClassSetRange {
            start: Literal::Unicode('\u{0041}'),
            end: Literal::Unicode('\u{005A}'),
            span: Span::default(),
        });

        <ast::ClassSetItem>::span(&p0);
    }
}#[cfg(test)]
mod tests_rug_631 {

    use super::*;
    use crate::ast::{Group, Span, GroupKind};

    #[test]
    fn test_flags() {
        let mut p0 = Group {
            span: Span { start: 0, end: 10 },
            kind: GroupKind::NonCapturing(None),
            ast: Box::new(Ast {}),
        };

        <ast::Group>::flags(&p0);
    }
}#[cfg(test)]
mod tests_rug_632 {
    use super::*;
    use crate::ast::{Group, Span, GroupKind};

    #[test]
    fn test_group_is_capturing() {
        let mut p0 = Group {
            span: Span { start: 0, end: 10 },
            kind: GroupKind::NonCapturing(None),
            ast: Box::new(Ast {}),
        };

        <Group as ast::Group>::is_capturing(&p0);
    }
}#[cfg(test)]
mod tests_rug_633 {
    use super::*;
    use crate::ast::{Group, Span, GroupKind};

    #[test]
    fn test_capture_index() {
        let mut p0 = Group {
            span: Span { start: 0, end: 10 },
            kind: GroupKind::NonCapturing(None),
            ast: Box::new(Ast {}),
        };

        <ast::Group>::capture_index(&p0);
    }
}