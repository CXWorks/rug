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
            CaptureLimitExceeded => {
                write!(
                    f,
                    "exceeded the maximum number of \
                 capturing groups ({})",
                    ::std::u32::MAX
                )
            }
            ClassEscapeInvalid => {
                write!(f, "invalid escape sequence found in character class")
            }
            ClassRangeInvalid => {
                write!(
                    f,
                    "invalid character class range, \
                 the start must be <= the end"
                )
            }
            ClassRangeLiteral => write!(f, "invalid range boundary, must be a literal"),
            ClassUnclosed => write!(f, "unclosed character class"),
            DecimalEmpty => write!(f, "decimal literal empty"),
            DecimalInvalid => write!(f, "decimal literal invalid"),
            EscapeHexEmpty => write!(f, "hexadecimal literal empty"),
            EscapeHexInvalid => {
                write!(f, "hexadecimal literal is not a Unicode scalar value")
            }
            EscapeHexInvalidDigit => write!(f, "invalid hexadecimal digit"),
            EscapeUnexpectedEof => {
                write!(
                    f,
                    "incomplete escape sequence, \
                 reached end of pattern prematurely"
                )
            }
            EscapeUnrecognized => write!(f, "unrecognized escape sequence"),
            FlagDanglingNegation => write!(f, "dangling flag negation operator"),
            FlagDuplicate { .. } => write!(f, "duplicate flag"),
            FlagRepeatedNegation { .. } => write!(f, "flag negation operator repeated"),
            FlagUnexpectedEof => write!(f, "expected flag but got end of regex"),
            FlagUnrecognized => write!(f, "unrecognized flag"),
            GroupNameDuplicate { .. } => write!(f, "duplicate capture group name"),
            GroupNameEmpty => write!(f, "empty capture group name"),
            GroupNameInvalid => write!(f, "invalid capture group character"),
            GroupNameUnexpectedEof => write!(f, "unclosed capture group name"),
            GroupUnclosed => write!(f, "unclosed group"),
            GroupUnopened => write!(f, "unopened group"),
            NestLimitExceeded(limit) => {
                write!(
                    f,
                    "exceed the maximum number of \
                 nested parentheses/brackets ({})",
                    limit
                )
            }
            RepetitionCountInvalid => {
                write!(
                    f,
                    "invalid repetition count range, \
                 the start must be <= the end"
                )
            }
            RepetitionCountDecimalEmpty => {
                write!(f, "repetition quantifier expects a valid decimal")
            }
            RepetitionCountUnclosed => write!(f, "unclosed counted repetition"),
            RepetitionMissing => write!(f, "repetition operator missing expression"),
            UnicodeClassInvalid => write!(f, "invalid Unicode character class"),
            UnsupportedBackreference => write!(f, "backreferences are not supported"),
            UnsupportedLookAround => {
                write!(
                    f,
                    "look-around, including look-ahead and look-behind, \
                 is not supported"
                )
            }
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
            f, "Position(o: {:?}, l: {:?}, c: {:?})", self.offset, self.line, self.column
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
        Position {
            offset: offset,
            line: line,
            column: column,
        }
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
            ClassUnicodeKind::NamedValue { op: ClassUnicodeOpKind::NotEqual, .. } => {
                !self.negated
            }
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
            ClassSet::Item(ref item) => {
                match *item {
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
                }
            }
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
                ClassSet::Item(ref mut item) => {
                    match *item {
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
                    }
                }
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
            assert!(! ast.is_empty());
        };
        thread::Builder::new().stack_size(1 << 10).spawn(run).unwrap().join().unwrap();
    }
}
#[cfg(test)]
mod tests_llm_16_5 {
    use super::*;
    use crate::*;
    use std::mem;
    #[test]
    fn test_drop() {
        let _rug_st_tests_llm_16_5_rrrruuuugggg_test_drop = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 'a';
        let rug_fuzz_4 = 'a';
        let rug_fuzz_5 = 'a';
        let rug_fuzz_6 = 'b';
        let empty_span = || Span::splat(
            Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2),
        );
        let empty_set = || ClassSet::Item(ClassSetItem::Empty(empty_span()));
        let mut class_set = ClassSet::Item(ClassSetItem::Empty(empty_span()));
        std::mem::drop(&mut class_set);
        let mut class_set = ClassSet::Item(
            ClassSetItem::Literal(Literal {
                span: empty_span(),
                kind: LiteralKind::Verbatim,
                c: rug_fuzz_3,
            }),
        );
        std::mem::drop(&mut class_set);
        let mut class_set = ClassSet::Item(
            ClassSetItem::Union(ClassSetUnion {
                span: empty_span(),
                items: vec![],
            }),
        );
        std::mem::drop(&mut class_set);
        let mut class_set = ClassSet::Item(
            ClassSetItem::Union(ClassSetUnion {
                span: empty_span(),
                items: vec![
                    ClassSetItem::Literal(Literal { span : empty_span(), kind :
                    LiteralKind::Verbatim, c : rug_fuzz_4, })
                ],
            }),
        );
        std::mem::drop(&mut class_set);
        let mut class_set = ClassSet::BinaryOp(ClassSetBinaryOp {
            span: empty_span(),
            kind: ClassSetBinaryOpKind::Intersection,
            lhs: Box::new(empty_set()),
            rhs: Box::new(empty_set()),
        });
        std::mem::drop(&mut class_set);
        let mut class_set = ClassSet::BinaryOp(ClassSetBinaryOp {
            span: empty_span(),
            kind: ClassSetBinaryOpKind::Intersection,
            lhs: Box::new(
                ClassSet::Item(
                    ClassSetItem::Literal(Literal {
                        span: empty_span(),
                        kind: LiteralKind::Verbatim,
                        c: rug_fuzz_5,
                    }),
                ),
            ),
            rhs: Box::new(
                ClassSet::Item(
                    ClassSetItem::Literal(Literal {
                        span: empty_span(),
                        kind: LiteralKind::Verbatim,
                        c: rug_fuzz_6,
                    }),
                ),
            ),
        });
        std::mem::drop(&mut class_set);
        let _rug_ed_tests_llm_16_5_rrrruuuugggg_test_drop = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_8 {
    use std::cmp::Ordering;
    use super::*;
    use crate::*;
    #[test]
    fn test_cmp() {
        let _rug_st_tests_llm_16_8_rrrruuuugggg_test_cmp = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 4;
        let rug_fuzz_4 = 5;
        let rug_fuzz_5 = 6;
        let rug_fuzz_6 = 1;
        let rug_fuzz_7 = 2;
        let rug_fuzz_8 = 3;
        let pos1 = Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        let pos2 = Position::new(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5);
        let pos3 = Position::new(rug_fuzz_6, rug_fuzz_7, rug_fuzz_8);
        debug_assert_eq!(pos1.cmp(& pos2), Ordering::Less);
        debug_assert_eq!(pos2.cmp(& pos1), Ordering::Greater);
        debug_assert_eq!(pos1.cmp(& pos3), Ordering::Equal);
        let _rug_ed_tests_llm_16_8_rrrruuuugggg_test_cmp = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_9 {
    use super::*;
    use crate::*;
    use std::cmp::Ordering;
    #[test]
    fn test_partial_cmp() {
        let _rug_st_tests_llm_16_9_rrrruuuugggg_test_partial_cmp = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 10;
        let rug_fuzz_4 = 1;
        let rug_fuzz_5 = 1;
        let rug_fuzz_6 = 20;
        let rug_fuzz_7 = 2;
        let rug_fuzz_8 = 2;
        let pos1 = Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        let pos2 = Position::new(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5);
        let pos3 = Position::new(rug_fuzz_6, rug_fuzz_7, rug_fuzz_8);
        debug_assert_eq!(pos1.partial_cmp(& pos2), Some(Ordering::Equal));
        debug_assert_eq!(pos1.partial_cmp(& pos3), Some(Ordering::Less));
        debug_assert_eq!(pos3.partial_cmp(& pos1), Some(Ordering::Greater));
        let _rug_ed_tests_llm_16_9_rrrruuuugggg_test_partial_cmp = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_10 {
    use crate::ast::{Position, Span};
    use std::cmp::Ordering;
    #[test]
    fn test_cmp() {
        let _rug_st_tests_llm_16_10_rrrruuuugggg_test_cmp = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 5;
        let rug_fuzz_4 = 1;
        let rug_fuzz_5 = 6;
        let rug_fuzz_6 = 6;
        let rug_fuzz_7 = 1;
        let rug_fuzz_8 = 7;
        let rug_fuzz_9 = 10;
        let rug_fuzz_10 = 1;
        let rug_fuzz_11 = 11;
        let pos1 = Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        let pos2 = Position::new(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5);
        let span1 = Span::new(pos1, pos2);
        let pos3 = Position::new(rug_fuzz_6, rug_fuzz_7, rug_fuzz_8);
        let pos4 = Position::new(rug_fuzz_9, rug_fuzz_10, rug_fuzz_11);
        let span2 = Span::new(pos3, pos4);
        let result = span1.cmp(&span2);
        debug_assert_eq!(result, Ordering::Less);
        let _rug_ed_tests_llm_16_10_rrrruuuugggg_test_cmp = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_104 {
    use super::*;
    use crate::*;
    #[test]
    fn test_into_ast_empty() {
        let _rug_st_tests_llm_16_104_rrrruuuugggg_test_into_ast_empty = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let alternation = Alternation {
            span: Span::splat(Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2)),
            asts: vec![],
        };
        let ast = alternation.into_ast();
        debug_assert_eq!(
            Ast::Empty(Span::splat(Position::new(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5))),
            ast
        );
        let _rug_ed_tests_llm_16_104_rrrruuuugggg_test_into_ast_empty = 0;
    }
    #[test]
    fn test_into_ast_single_ast() {
        let _rug_st_tests_llm_16_104_rrrruuuugggg_test_into_ast_single_ast = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 'a';
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 0;
        let rug_fuzz_7 = 0;
        let rug_fuzz_8 = 0;
        let rug_fuzz_9 = 0;
        let rug_fuzz_10 = 'a';
        let ast = Ast::Literal(Literal {
            span: Span::splat(Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2)),
            kind: LiteralKind::Verbatim,
            c: rug_fuzz_3,
        });
        let mut alternation = Alternation {
            span: Span::splat(Position::new(rug_fuzz_4, rug_fuzz_5, rug_fuzz_6)),
            asts: vec![ast],
        };
        let ast = alternation.into_ast();
        debug_assert_eq!(
            Ast::Literal(Literal { span : Span::splat(Position::new(rug_fuzz_7,
            rug_fuzz_8, rug_fuzz_9)), kind : LiteralKind::Verbatim, c : rug_fuzz_10, }),
            ast
        );
        let _rug_ed_tests_llm_16_104_rrrruuuugggg_test_into_ast_single_ast = 0;
    }
    #[test]
    fn test_into_ast_multiple_asts() {
        let _rug_st_tests_llm_16_104_rrrruuuugggg_test_into_ast_multiple_asts = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 'a';
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 0;
        let rug_fuzz_7 = 'b';
        let rug_fuzz_8 = 0;
        let rug_fuzz_9 = 0;
        let rug_fuzz_10 = 0;
        let rug_fuzz_11 = 0;
        let rug_fuzz_12 = 0;
        let rug_fuzz_13 = 0;
        let rug_fuzz_14 = 0;
        let rug_fuzz_15 = 0;
        let rug_fuzz_16 = 0;
        let rug_fuzz_17 = 'a';
        let ast1 = Ast::Literal(Literal {
            span: Span::splat(Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2)),
            kind: LiteralKind::Verbatim,
            c: rug_fuzz_3,
        });
        let ast2 = Ast::Literal(Literal {
            span: Span::splat(Position::new(rug_fuzz_4, rug_fuzz_5, rug_fuzz_6)),
            kind: LiteralKind::Verbatim,
            c: rug_fuzz_7,
        });
        let mut alternation = Alternation {
            span: Span::splat(Position::new(rug_fuzz_8, rug_fuzz_9, rug_fuzz_10)),
            asts: vec![ast1, ast2],
        };
        let ast = alternation.into_ast();
        debug_assert_eq!(
            Ast::Alternation(Alternation { span : Span::splat(Position::new(rug_fuzz_11,
            rug_fuzz_12, rug_fuzz_13)), asts : vec![Ast::Literal(Literal { span :
            Span::splat(Position::new(rug_fuzz_14, rug_fuzz_15, rug_fuzz_16)), kind :
            LiteralKind::Verbatim, c : rug_fuzz_17, }), Ast::Literal(Literal { span :
            Span::splat(Position::new(0, 0, 0)), kind : LiteralKind::Verbatim, c : 'b',
            })] }), ast
        );
        let _rug_ed_tests_llm_16_104_rrrruuuugggg_test_into_ast_multiple_asts = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_106 {
    use super::*;
    use crate::*;
    use ast::*;
    #[test]
    fn test_is_empty_empty_case() {
        let _rug_st_tests_llm_16_106_rrrruuuugggg_test_is_empty_empty_case = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let ast = Ast::Empty(
            Span::splat(Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2)),
        );
        debug_assert!(ast.is_empty());
        let _rug_ed_tests_llm_16_106_rrrruuuugggg_test_is_empty_empty_case = 0;
    }
    #[test]
    fn test_is_empty_non_empty_case() {
        let _rug_st_tests_llm_16_106_rrrruuuugggg_test_is_empty_non_empty_case = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 'a';
        let ast = Ast::Literal(Literal {
            span: Span::splat(Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2)),
            kind: LiteralKind::Verbatim,
            c: rug_fuzz_3,
        });
        debug_assert!(! ast.is_empty());
        let _rug_ed_tests_llm_16_106_rrrruuuugggg_test_is_empty_non_empty_case = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_110 {
    use super::*;
    use crate::*;
    use crate::ast::*;
    #[test]
    fn test_span() {
        let _rug_st_tests_llm_16_110_rrrruuuugggg_test_span = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = false;
        let rug_fuzz_7 = 0;
        let rug_fuzz_8 = 0;
        let rug_fuzz_9 = 0;
        let rug_fuzz_10 = 0;
        let rug_fuzz_11 = 0;
        let rug_fuzz_12 = 0;
        let rug_fuzz_13 = false;
        let rug_fuzz_14 = 0;
        let rug_fuzz_15 = 0;
        let rug_fuzz_16 = 0;
        let rug_fuzz_17 = 0;
        let rug_fuzz_18 = 0;
        let rug_fuzz_19 = 0;
        let rug_fuzz_20 = false;
        let perl_span = Span::new(
            Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2),
            Position::new(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5),
        );
        let perl = ClassPerl {
            span: perl_span.clone(),
            kind: ClassPerlKind::Digit,
            negated: rug_fuzz_6,
        };
        debug_assert_eq!(perl.span, perl_span);
        let unicode_span = Span::new(
            Position::new(rug_fuzz_7, rug_fuzz_8, rug_fuzz_9),
            Position::new(rug_fuzz_10, rug_fuzz_11, rug_fuzz_12),
        );
        let unicode = ClassUnicode {
            span: unicode_span.clone(),
            negated: rug_fuzz_13,
            kind: ClassUnicodeKind::NamedValue {
                op: ClassUnicodeOpKind::Equal,
                name: String::new(),
                value: String::new(),
            },
        };
        debug_assert_eq!(unicode.span, unicode_span);
        let bracketed_span = Span::new(
            Position::new(rug_fuzz_14, rug_fuzz_15, rug_fuzz_16),
            Position::new(rug_fuzz_17, rug_fuzz_18, rug_fuzz_19),
        );
        let bracketed = ClassBracketed {
            span: bracketed_span.clone(),
            negated: rug_fuzz_20,
            kind: ClassSet::union(ClassSetUnion {
                span: bracketed_span.clone(),
                items: vec![],
            }),
        };
        debug_assert_eq!(bracketed.span, bracketed_span);
        let _rug_ed_tests_llm_16_110_rrrruuuugggg_test_span = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_114 {
    use super::*;
    use crate::*;
    #[test]
    fn test_span_item() {
        let _rug_st_tests_llm_16_114_rrrruuuugggg_test_span_item = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 'a';
        let class_set_item = ClassSetItem::Literal(Literal {
            span: Span::splat(Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2)),
            kind: LiteralKind::Verbatim,
            c: rug_fuzz_3,
        });
        let class_set = ClassSet::Item(class_set_item);
        debug_assert_eq!(class_set.span(), & Span::splat(Position::new(0, 0, 0)));
        let _rug_ed_tests_llm_16_114_rrrruuuugggg_test_span_item = 0;
    }
    #[test]
    fn test_span_binary_op() {
        let _rug_st_tests_llm_16_114_rrrruuuugggg_test_span_binary_op = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 'a';
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 0;
        let class_set_item = ClassSetItem::Literal(Literal {
            span: Span::splat(Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2)),
            kind: LiteralKind::Verbatim,
            c: rug_fuzz_3,
        });
        let class_set = ClassSet::BinaryOp(ClassSetBinaryOp {
            span: Span::splat(Position::new(rug_fuzz_4, rug_fuzz_5, rug_fuzz_6)),
            kind: ClassSetBinaryOpKind::Intersection,
            lhs: Box::new(ClassSet::Item(class_set_item.clone())),
            rhs: Box::new(ClassSet::Item(class_set_item)),
        });
        debug_assert_eq!(class_set.span(), & Span::splat(Position::new(0, 0, 0)));
        let _rug_ed_tests_llm_16_114_rrrruuuugggg_test_span_binary_op = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_115 {
    use super::*;
    use crate::*;
    #[test]
    fn test_union() {
        let _rug_st_tests_llm_16_115_rrrruuuugggg_test_union = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 0;
        let rug_fuzz_7 = 0;
        let rug_fuzz_8 = 0;
        let rug_fuzz_9 = 0;
        let rug_fuzz_10 = 0;
        let rug_fuzz_11 = 0;
        let rug_fuzz_12 = 'a';
        let ast = ClassSetUnion {
            span: Span::new(
                Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2),
                Position::new(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5),
            ),
            items: vec![
                ClassSetItem::Literal(Literal { span :
                Span::new(Position::new(rug_fuzz_6, rug_fuzz_7, rug_fuzz_8),
                Position::new(rug_fuzz_9, rug_fuzz_10, rug_fuzz_11)), kind :
                LiteralKind::Verbatim, c : rug_fuzz_12, }), ClassSetItem::Literal(Literal
                { span : Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)), kind
                : LiteralKind::Verbatim, c : 'b', })
            ],
        };
        let result = ClassSet::union(ast);
        debug_assert_eq!(
            result, ClassSet::Item(ClassSetItem::Union(ClassSetUnion { span :
            Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)), items :
            vec![ClassSetItem::Literal(Literal { span : Span::new(Position::new(0, 0, 0),
            Position::new(0, 0, 0)), kind : LiteralKind::Verbatim, c : 'a', }),
            ClassSetItem::Literal(Literal { span : Span::new(Position::new(0, 0, 0),
            Position::new(0, 0, 0)), kind : LiteralKind::Verbatim, c : 'b', }),], }))
        );
        let _rug_ed_tests_llm_16_115_rrrruuuugggg_test_union = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_118 {
    use super::*;
    use crate::*;
    use std::cmp::Ordering;
    #[test]
    fn test_is_valid() {
        let _rug_st_tests_llm_16_118_rrrruuuugggg_test_is_valid = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 1;
        let rug_fuzz_5 = 1;
        let rug_fuzz_6 = 'a';
        let rug_fuzz_7 = 0;
        let rug_fuzz_8 = 1;
        let rug_fuzz_9 = 1;
        let rug_fuzz_10 = 0;
        let rug_fuzz_11 = 1;
        let rug_fuzz_12 = 1;
        let rug_fuzz_13 = 'z';
        let rug_fuzz_14 = 0;
        let rug_fuzz_15 = 1;
        let rug_fuzz_16 = 1;
        let rug_fuzz_17 = 0;
        let rug_fuzz_18 = 1;
        let rug_fuzz_19 = 1;
        let start = Literal {
            span: Span::new(
                Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2),
                Position::new(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5),
            ),
            kind: LiteralKind::Verbatim,
            c: rug_fuzz_6,
        };
        let end = Literal {
            span: Span::new(
                Position::new(rug_fuzz_7, rug_fuzz_8, rug_fuzz_9),
                Position::new(rug_fuzz_10, rug_fuzz_11, rug_fuzz_12),
            ),
            kind: LiteralKind::Verbatim,
            c: rug_fuzz_13,
        };
        let range = ClassSetRange {
            span: Span::new(
                Position::new(rug_fuzz_14, rug_fuzz_15, rug_fuzz_16),
                Position::new(rug_fuzz_17, rug_fuzz_18, rug_fuzz_19),
            ),
            start: start,
            end: end,
        };
        debug_assert_eq!(range.is_valid(), true);
        let _rug_ed_tests_llm_16_118_rrrruuuugggg_test_is_valid = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_119 {
    use super::*;
    use crate::*;
    #[test]
    fn test_into_item_empty() {
        let _rug_st_tests_llm_16_119_rrrruuuugggg_test_into_item_empty = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 1;
        let union = ClassSetUnion {
            span: Span::splat(Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2)),
            items: Vec::new(),
        };
        let result = union.into_item();
        debug_assert_eq!(
            result, ClassSetItem::Empty(Span::splat(Position::new(0, 1, 1)))
        );
        let _rug_ed_tests_llm_16_119_rrrruuuugggg_test_into_item_empty = 0;
    }
    #[test]
    fn test_into_item_single() {
        let _rug_st_tests_llm_16_119_rrrruuuugggg_test_into_item_single = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 1;
        let rug_fuzz_5 = 1;
        let rug_fuzz_6 = 'a';
        let union = ClassSetUnion {
            span: Span::splat(Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2)),
            items: vec![
                ClassSetItem::Literal(Literal { span :
                Span::splat(Position::new(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5)), kind :
                LiteralKind::Verbatim, c : rug_fuzz_6, })
            ],
        };
        let result = union.into_item();
        debug_assert_eq!(
            result, ClassSetItem::Literal(Literal { span : Span::splat(Position::new(0,
            1, 1)), kind : LiteralKind::Verbatim, c : 'a', })
        );
        let _rug_ed_tests_llm_16_119_rrrruuuugggg_test_into_item_single = 0;
    }
    #[test]
    fn test_into_item_multiple() {
        let _rug_st_tests_llm_16_119_rrrruuuugggg_test_into_item_multiple = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 1;
        let rug_fuzz_5 = 1;
        let rug_fuzz_6 = 'a';
        let union = ClassSetUnion {
            span: Span::splat(Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2)),
            items: vec![
                ClassSetItem::Literal(Literal { span :
                Span::splat(Position::new(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5)), kind :
                LiteralKind::Verbatim, c : rug_fuzz_6, }), ClassSetItem::Literal(Literal
                { span : Span::splat(Position::new(1, 1, 2)), kind :
                LiteralKind::Verbatim, c : 'b', })
            ],
        };
        let result = union.into_item();
        debug_assert_eq!(
            result, ClassSetItem::Union(ClassSetUnion { span :
            Span::splat(Position::new(0, 1, 1)), items :
            vec![ClassSetItem::Literal(Literal { span : Span::splat(Position::new(0, 1,
            1)), kind : LiteralKind::Verbatim, c : 'a', }), ClassSetItem::Literal(Literal
            { span : Span::splat(Position::new(1, 1, 2)), kind : LiteralKind::Verbatim, c
            : 'b', }),], })
        );
        let _rug_ed_tests_llm_16_119_rrrruuuugggg_test_into_item_multiple = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_120 {
    use super::*;
    use crate::*;
    #[test]
    fn test_push() {
        let _rug_st_tests_llm_16_120_rrrruuuugggg_test_push = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 0;
        let rug_fuzz_7 = 0;
        let rug_fuzz_8 = 0;
        let rug_fuzz_9 = 0;
        let rug_fuzz_10 = 0;
        let rug_fuzz_11 = 0;
        let rug_fuzz_12 = 0;
        let mut class_set_union = ClassSetUnion {
            span: Span::new(
                Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2),
                Position::new(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5),
            ),
            items: vec![],
        };
        let item = ClassSetItem::Empty(
            Span::new(
                Position::new(rug_fuzz_6, rug_fuzz_7, rug_fuzz_8),
                Position::new(rug_fuzz_9, rug_fuzz_10, rug_fuzz_11),
            ),
        );
        class_set_union.push(item);
        debug_assert_eq!(class_set_union.span.start, Position::new(0, 0, 0));
        debug_assert_eq!(class_set_union.span.end, Position::new(0, 0, 0));
        debug_assert_eq!(class_set_union.items.len(), 1);
        debug_assert_eq!(
            class_set_union.items[rug_fuzz_12],
            ClassSetItem::Empty(Span::new(Position::new(0, 0, 0), Position::new(0, 0,
            0)))
        );
        let _rug_ed_tests_llm_16_120_rrrruuuugggg_test_push = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_121 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_negated_not_equal() {
        let _rug_st_tests_llm_16_121_rrrruuuugggg_test_is_negated_not_equal = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = false;
        let class_unicode = ClassUnicode {
            span: Span::new(
                Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2),
                Position::new(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5),
            ),
            negated: rug_fuzz_6,
            kind: ClassUnicodeKind::NamedValue {
                op: ClassUnicodeOpKind::NotEqual,
                name: String::new(),
                value: String::new(),
            },
        };
        debug_assert_eq!(class_unicode.is_negated(), true);
        let _rug_ed_tests_llm_16_121_rrrruuuugggg_test_is_negated_not_equal = 0;
    }
    #[test]
    fn test_is_negated_not_equal_negated() {
        let _rug_st_tests_llm_16_121_rrrruuuugggg_test_is_negated_not_equal_negated = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = true;
        let class_unicode = ClassUnicode {
            span: Span::new(
                Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2),
                Position::new(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5),
            ),
            negated: rug_fuzz_6,
            kind: ClassUnicodeKind::NamedValue {
                op: ClassUnicodeOpKind::NotEqual,
                name: String::new(),
                value: String::new(),
            },
        };
        debug_assert_eq!(class_unicode.is_negated(), false);
        let _rug_ed_tests_llm_16_121_rrrruuuugggg_test_is_negated_not_equal_negated = 0;
    }
    #[test]
    fn test_is_negated_equal() {
        let _rug_st_tests_llm_16_121_rrrruuuugggg_test_is_negated_equal = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = false;
        let class_unicode = ClassUnicode {
            span: Span::new(
                Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2),
                Position::new(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5),
            ),
            negated: rug_fuzz_6,
            kind: ClassUnicodeKind::NamedValue {
                op: ClassUnicodeOpKind::Equal,
                name: String::new(),
                value: String::new(),
            },
        };
        debug_assert_eq!(class_unicode.is_negated(), false);
        let _rug_ed_tests_llm_16_121_rrrruuuugggg_test_is_negated_equal = 0;
    }
    #[test]
    fn test_is_negated_not_equal_colon() {
        let _rug_st_tests_llm_16_121_rrrruuuugggg_test_is_negated_not_equal_colon = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = false;
        let class_unicode = ClassUnicode {
            span: Span::new(
                Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2),
                Position::new(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5),
            ),
            negated: rug_fuzz_6,
            kind: ClassUnicodeKind::NamedValue {
                op: ClassUnicodeOpKind::NotEqual,
                name: String::new(),
                value: String::new(),
            },
        };
        debug_assert_eq!(class_unicode.is_negated(), true);
        let _rug_ed_tests_llm_16_121_rrrruuuugggg_test_is_negated_not_equal_colon = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_122 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_equal_when_equal_should_return_true() {
        let _rug_st_tests_llm_16_122_rrrruuuugggg_test_is_equal_when_equal_should_return_true = 0;
        let op = ClassUnicodeOpKind::Equal;
        debug_assert_eq!(op.is_equal(), true);
        let _rug_ed_tests_llm_16_122_rrrruuuugggg_test_is_equal_when_equal_should_return_true = 0;
    }
    #[test]
    fn test_is_equal_when_colon_should_return_true() {
        let _rug_st_tests_llm_16_122_rrrruuuugggg_test_is_equal_when_colon_should_return_true = 0;
        let op = ClassUnicodeOpKind::Colon;
        debug_assert_eq!(op.is_equal(), true);
        let _rug_ed_tests_llm_16_122_rrrruuuugggg_test_is_equal_when_colon_should_return_true = 0;
    }
    #[test]
    fn test_is_equal_when_not_equal_should_return_false() {
        let _rug_st_tests_llm_16_122_rrrruuuugggg_test_is_equal_when_not_equal_should_return_false = 0;
        let op = ClassUnicodeOpKind::NotEqual;
        debug_assert_eq!(op.is_equal(), false);
        let _rug_ed_tests_llm_16_122_rrrruuuugggg_test_is_equal_when_not_equal_should_return_false = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_125 {
    use crate::ast::{Error, ErrorKind, Span, Position};
    #[test]
    fn test_auxiliary_span() {
        let _rug_st_tests_llm_16_125_rrrruuuugggg_test_auxiliary_span = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 4;
        let rug_fuzz_4 = 1;
        let rug_fuzz_5 = 5;
        let rug_fuzz_6 = 0;
        let rug_fuzz_7 = 1;
        let rug_fuzz_8 = 1;
        let rug_fuzz_9 = 0;
        let rug_fuzz_10 = 1;
        let rug_fuzz_11 = 1;
        let original_span = Span::new(
            Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2),
            Position::new(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5),
        );
        let error = Error {
            kind: ErrorKind::FlagDuplicate {
                original: original_span,
            },
            pattern: String::new(),
            span: Span::new(
                Position::new(rug_fuzz_6, rug_fuzz_7, rug_fuzz_8),
                Position::new(rug_fuzz_9, rug_fuzz_10, rug_fuzz_11),
            ),
        };
        let auxiliary_span = error.auxiliary_span();
        debug_assert_eq!(auxiliary_span, Some(& original_span));
        let _rug_ed_tests_llm_16_125_rrrruuuugggg_test_auxiliary_span = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_126 {
    use super::*;
    use crate::*;
    use ast::{Error, ErrorKind, Position, Span};
    #[test]
    fn test_error_kind() {
        let _rug_st_tests_llm_16_126_rrrruuuugggg_test_error_kind = 0;
        let error_kind = ErrorKind::CaptureLimitExceeded;
        debug_assert_eq!(
            error_kind.to_string(),
            "exceeded the maximum number of capturing groups (4294967295)"
        );
        let error_kind = ErrorKind::ClassEscapeInvalid;
        debug_assert_eq!(
            error_kind.to_string(), "invalid escape sequence found in character class"
        );
        let _rug_ed_tests_llm_16_126_rrrruuuugggg_test_error_kind = 0;
    }
    #[test]
    fn test_error_position() {
        let _rug_st_tests_llm_16_126_rrrruuuugggg_test_error_position = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let position = Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        debug_assert_eq!(position.offset, 10);
        debug_assert_eq!(position.line, 2);
        debug_assert_eq!(position.column, 3);
        let _rug_ed_tests_llm_16_126_rrrruuuugggg_test_error_position = 0;
    }
    #[test]
    fn test_error_span() {
        let _rug_st_tests_llm_16_126_rrrruuuugggg_test_error_span = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 20;
        let rug_fuzz_4 = 4;
        let rug_fuzz_5 = 5;
        let start = Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        let end = Position::new(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5);
        let span = Span::new(start, end);
        debug_assert_eq!(span.start.offset, 10);
        debug_assert_eq!(span.start.line, 2);
        debug_assert_eq!(span.start.column, 3);
        debug_assert_eq!(span.end.offset, 20);
        debug_assert_eq!(span.end.line, 4);
        debug_assert_eq!(span.end.column, 5);
        let _rug_ed_tests_llm_16_126_rrrruuuugggg_test_error_span = 0;
    }
    #[test]
    fn test_error() {
        let _rug_st_tests_llm_16_126_rrrruuuugggg_test_error = 0;
        let rug_fuzz_0 = "abc";
        let rug_fuzz_1 = 10;
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 3;
        let rug_fuzz_4 = 20;
        let rug_fuzz_5 = 4;
        let rug_fuzz_6 = 5;
        let error_kind = ErrorKind::CaptureLimitExceeded;
        let pattern = String::from(rug_fuzz_0);
        let start = Position::new(rug_fuzz_1, rug_fuzz_2, rug_fuzz_3);
        let end = Position::new(rug_fuzz_4, rug_fuzz_5, rug_fuzz_6);
        let span = Span::new(start, end);
        let error = Error {
            kind: error_kind,
            pattern: pattern,
            span: span,
        };
        let error_kind = error.kind();
        debug_assert_eq!(error_kind, & ErrorKind::CaptureLimitExceeded);
        let pattern = error.pattern();
        debug_assert_eq!(pattern, "abc");
        let error_span = error.span();
        debug_assert_eq!(error_span.start.offset, 10);
        debug_assert_eq!(error_span.start.line, 2);
        debug_assert_eq!(error_span.start.column, 3);
        debug_assert_eq!(error_span.end.offset, 20);
        debug_assert_eq!(error_span.end.line, 4);
        debug_assert_eq!(error_span.end.column, 5);
        let _rug_ed_tests_llm_16_126_rrrruuuugggg_test_error = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_128_llm_16_127 {
    use super::*;
    use crate::*;
    use crate::ast::Error;
    use crate::ast::ErrorKind;
    use crate::ast::Position;
    use crate::ast::Span;
    #[test]
    fn test_pattern() {
        let _rug_st_tests_llm_16_128_llm_16_127_rrrruuuugggg_test_pattern = 0;
        let rug_fuzz_0 = "abc";
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = 2;
        let rug_fuzz_5 = 1;
        let rug_fuzz_6 = 3;
        let error = Error {
            kind: ErrorKind::ClassUnclosed,
            pattern: String::from(rug_fuzz_0),
            span: Span::new(
                Position::new(rug_fuzz_1, rug_fuzz_2, rug_fuzz_3),
                Position::new(rug_fuzz_4, rug_fuzz_5, rug_fuzz_6),
            ),
        };
        debug_assert_eq!(error.pattern(), "abc");
        let _rug_ed_tests_llm_16_128_llm_16_127_rrrruuuugggg_test_pattern = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_131 {
    use super::*;
    use crate::*;
    #[test]
    fn test_add_item() {
        let _rug_st_tests_llm_16_131_rrrruuuugggg_test_add_item = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 0;
        let rug_fuzz_7 = 0;
        let rug_fuzz_8 = 0;
        let rug_fuzz_9 = 2;
        let mut flags = Flags {
            span: Span::splat(Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2)),
            items: vec![
                FlagsItem { span : Span::splat(Position::new(rug_fuzz_3, rug_fuzz_4,
                rug_fuzz_5)), kind : FlagsItemKind::Flag(Flag::CaseInsensitive), },
                FlagsItem { span : Span::splat(Position::new(0, 0, 0)), kind :
                FlagsItemKind::Flag(Flag::MultiLine), }
            ],
        };
        let result = flags
            .add_item(FlagsItem {
                span: Span::splat(Position::new(rug_fuzz_6, rug_fuzz_7, rug_fuzz_8)),
                kind: FlagsItemKind::Flag(Flag::DotMatchesNewLine),
            });
        debug_assert_eq!(result, None);
        debug_assert_eq!(flags.items.len(), 3);
        debug_assert_eq!(
            flags.items[rug_fuzz_9].kind, FlagsItemKind::Flag(Flag::DotMatchesNewLine)
        );
        let _rug_ed_tests_llm_16_131_rrrruuuugggg_test_add_item = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_134 {
    use crate::ast::{Flag, FlagsItemKind};
    #[test]
    fn test_is_negation() {
        let _rug_st_tests_llm_16_134_rrrruuuugggg_test_is_negation = 0;
        let negation = FlagsItemKind::Negation;
        let flag = FlagsItemKind::Flag(Flag::CaseInsensitive);
        debug_assert!(negation.is_negation());
        debug_assert!(! flag.is_negation());
        let _rug_ed_tests_llm_16_134_rrrruuuugggg_test_is_negation = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_141 {
    use super::*;
    use crate::*;
    #[test]
    fn test_digits_x() {
        let _rug_st_tests_llm_16_141_rrrruuuugggg_test_digits_x = 0;
        let kind = HexLiteralKind::X;
        let result = kind.digits();
        debug_assert_eq!(result, 2);
        let _rug_ed_tests_llm_16_141_rrrruuuugggg_test_digits_x = 0;
    }
    #[test]
    fn test_digits_unicode_short() {
        let _rug_st_tests_llm_16_141_rrrruuuugggg_test_digits_unicode_short = 0;
        let kind = HexLiteralKind::UnicodeShort;
        let result = kind.digits();
        debug_assert_eq!(result, 4);
        let _rug_ed_tests_llm_16_141_rrrruuuugggg_test_digits_unicode_short = 0;
    }
    #[test]
    fn test_digits_unicode_long() {
        let _rug_st_tests_llm_16_141_rrrruuuugggg_test_digits_unicode_long = 0;
        let kind = HexLiteralKind::UnicodeLong;
        let result = kind.digits();
        debug_assert_eq!(result, 8);
        let _rug_ed_tests_llm_16_141_rrrruuuugggg_test_digits_unicode_long = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_142 {
    use super::*;
    use crate::*;
    use ast::{Literal, LiteralKind, HexLiteralKind, SpecialLiteralKind, Span, Position};
    #[test]
    fn test_byte() {
        let _rug_st_tests_llm_16_142_rrrruuuugggg_test_byte = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = 1;
        let rug_fuzz_5 = 2;
        let rug_fuzz_6 = 'a';
        let literal = Literal {
            span: Span::new(
                Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2),
                Position::new(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5),
            ),
            kind: LiteralKind::HexFixed(HexLiteralKind::X),
            c: rug_fuzz_6,
        };
        debug_assert_eq!(literal.byte(), Some(97u8));
        let _rug_ed_tests_llm_16_142_rrrruuuugggg_test_byte = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_143 {
    use super::*;
    use crate::*;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_143_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let position = Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        debug_assert_eq!(position.offset, 10);
        debug_assert_eq!(position.line, 2);
        debug_assert_eq!(position.column, 3);
        let _rug_ed_tests_llm_16_143_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_144 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_valid_bounded_valid() {
        let _rug_st_tests_llm_16_144_rrrruuuugggg_test_is_valid_bounded_valid = 0;
        let rug_fuzz_0 = 2;
        let rug_fuzz_1 = 5;
        let range = RepetitionRange::Bounded(rug_fuzz_0, rug_fuzz_1);
        debug_assert!(range.is_valid());
        let _rug_ed_tests_llm_16_144_rrrruuuugggg_test_is_valid_bounded_valid = 0;
    }
    #[test]
    fn test_is_valid_bounded_invalid() {
        let _rug_st_tests_llm_16_144_rrrruuuugggg_test_is_valid_bounded_invalid = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 2;
        let range = RepetitionRange::Bounded(rug_fuzz_0, rug_fuzz_1);
        debug_assert!(! range.is_valid());
        let _rug_ed_tests_llm_16_144_rrrruuuugggg_test_is_valid_bounded_invalid = 0;
    }
    #[test]
    fn test_is_valid_exactly_valid() {
        let _rug_st_tests_llm_16_144_rrrruuuugggg_test_is_valid_exactly_valid = 0;
        let rug_fuzz_0 = 3;
        let range = RepetitionRange::Exactly(rug_fuzz_0);
        debug_assert!(range.is_valid());
        let _rug_ed_tests_llm_16_144_rrrruuuugggg_test_is_valid_exactly_valid = 0;
    }
    #[test]
    fn test_is_valid_at_least_valid() {
        let _rug_st_tests_llm_16_144_rrrruuuugggg_test_is_valid_at_least_valid = 0;
        let rug_fuzz_0 = 4;
        let range = RepetitionRange::AtLeast(rug_fuzz_0);
        debug_assert!(range.is_valid());
        let _rug_ed_tests_llm_16_144_rrrruuuugggg_test_is_valid_at_least_valid = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_145 {
    use crate::ast::{Position, Span};
    #[test]
    fn test_is_empty() {
        let _rug_st_tests_llm_16_145_rrrruuuugggg_test_is_empty = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 1;
        let rug_fuzz_5 = 1;
        let start = Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        let end = Position::new(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5);
        let span = Span::new(start, end);
        debug_assert_eq!(span.is_empty(), true);
        let _rug_ed_tests_llm_16_145_rrrruuuugggg_test_is_empty = 0;
    }
    #[test]
    fn test_is_empty_false() {
        let _rug_st_tests_llm_16_145_rrrruuuugggg_test_is_empty_false = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = 1;
        let rug_fuzz_5 = 2;
        let start = Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        let end = Position::new(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5);
        let span = Span::new(start, end);
        debug_assert_eq!(span.is_empty(), false);
        let _rug_ed_tests_llm_16_145_rrrruuuugggg_test_is_empty_false = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_146 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_one_line() {
        let _rug_st_tests_llm_16_146_rrrruuuugggg_test_is_one_line = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 1;
        let rug_fuzz_5 = 1;
        let rug_fuzz_6 = 1;
        let rug_fuzz_7 = 1;
        let rug_fuzz_8 = 2;
        let rug_fuzz_9 = 0;
        let rug_fuzz_10 = 1;
        let rug_fuzz_11 = 1;
        let rug_fuzz_12 = 10;
        let rug_fuzz_13 = 2;
        let rug_fuzz_14 = 2;
        let position = Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        let span = Span::new(position.clone(), position.clone());
        debug_assert_eq!(span.is_one_line(), true);
        let position1 = Position::new(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5);
        let position2 = Position::new(rug_fuzz_6, rug_fuzz_7, rug_fuzz_8);
        let span = Span::new(position1, position2);
        debug_assert_eq!(span.is_one_line(), true);
        let position1 = Position::new(rug_fuzz_9, rug_fuzz_10, rug_fuzz_11);
        let position2 = Position::new(rug_fuzz_12, rug_fuzz_13, rug_fuzz_14);
        let span = Span::new(position1, position2);
        debug_assert_eq!(span.is_one_line(), false);
        let _rug_ed_tests_llm_16_146_rrrruuuugggg_test_is_one_line = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_149 {
    use super::*;
    use crate::*;
    #[test]
    fn test_splat() {
        let _rug_st_tests_llm_16_149_rrrruuuugggg_test_splat = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 1;
        let pos = Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        let span = Span::splat(pos);
        debug_assert_eq!(span.start.offset, 10);
        debug_assert_eq!(span.start.line, 1);
        debug_assert_eq!(span.start.column, 1);
        debug_assert_eq!(span.end.offset, 10);
        debug_assert_eq!(span.end.line, 1);
        debug_assert_eq!(span.end.column, 1);
        let _rug_ed_tests_llm_16_149_rrrruuuugggg_test_splat = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_150 {
    use super::*;
    use crate::*;
    #[test]
    fn test_with_end() {
        let _rug_st_tests_llm_16_150_rrrruuuugggg_test_with_end = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 5;
        let rug_fuzz_4 = 1;
        let rug_fuzz_5 = 6;
        let rug_fuzz_6 = 10;
        let rug_fuzz_7 = 2;
        let rug_fuzz_8 = 5;
        let start = Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        let end = Position::new(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5);
        let span = Span::new(start, end);
        let new_end = Position::new(rug_fuzz_6, rug_fuzz_7, rug_fuzz_8);
        let updated_span = span.with_end(new_end);
        debug_assert_eq!(updated_span.start, span.start);
        debug_assert_eq!(updated_span.end, new_end);
        let _rug_ed_tests_llm_16_150_rrrruuuugggg_test_with_end = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_151 {
    use crate::ast::{Position, Span};
    #[test]
    fn test_with_start() {
        let _rug_st_tests_llm_16_151_rrrruuuugggg_test_with_start = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 5;
        let rug_fuzz_4 = 1;
        let rug_fuzz_5 = 6;
        let rug_fuzz_6 = 10;
        let rug_fuzz_7 = 2;
        let rug_fuzz_8 = 1;
        let start_pos = Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        let end_pos = Position::new(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5);
        let span = Span::new(start_pos, end_pos);
        let new_start_pos = Position::new(rug_fuzz_6, rug_fuzz_7, rug_fuzz_8);
        let new_span = span.with_start(new_start_pos);
        debug_assert_eq!(new_span.start, new_start_pos);
        debug_assert_eq!(new_span.end, end_pos);
        let _rug_ed_tests_llm_16_151_rrrruuuugggg_test_with_start = 0;
    }
}
#[cfg(test)]
mod tests_rug_268 {
    use super::*;
    use crate::ast::{Span, Position};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_268_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 1;
        let rug_fuzz_5 = 1;
        let mut p0 = Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        let mut p1 = Position::new(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5);
        Span::new(p0, p1);
        let _rug_ed_tests_rug_268_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_269 {
    use super::*;
    use crate::ast::{self, Ast, Span, Position};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_269_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let mut v19 = Ast::Empty(
            Span::splat(Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2)),
        );
        Ast::span(&v19);
        let _rug_ed_tests_rug_269_rrrruuuugggg_test_rug = 0;
    }
}
