//! When serializing or deserializing JSON goes wrong.
use crate::io;
use crate::lib::str::FromStr;
use crate::lib::*;
use serde::{de, ser};
/// This type represents all possible errors that can occur when serializing or
/// deserializing JSON data.
pub struct Error {
    /// This `Box` allows us to keep the size of `Error` as small as possible. A
    /// larger `Error` type was substantially slower due to all the functions
    /// that pass around `Result<T, Error>`.
    err: Box<ErrorImpl>,
}
/// Alias for a `Result` with the error type `serde_json::Error`.
pub type Result<T> = result::Result<T, Error>;
impl Error {
    /// One-based line number at which the error was detected.
    ///
    /// Characters in the first line of the input (before the first newline
    /// character) are in line 1.
    pub fn line(&self) -> usize {
        self.err.line
    }
    /// One-based column number at which the error was detected.
    ///
    /// The first character in the input and any characters immediately
    /// following a newline character are in column 1.
    ///
    /// Note that errors may occur in column 0, for example if a read from an IO
    /// stream fails immediately following a previously read newline character.
    pub fn column(&self) -> usize {
        self.err.column
    }
    /// Categorizes the cause of this error.
    ///
    /// - `Category::Io` - failure to read or write bytes on an IO stream
    /// - `Category::Syntax` - input that is not syntactically valid JSON
    /// - `Category::Data` - input data that is semantically incorrect
    /// - `Category::Eof` - unexpected end of the input data
    pub fn classify(&self) -> Category {
        match self.err.code {
            ErrorCode::Message(_) => Category::Data,
            ErrorCode::Io(_) => Category::Io,
            ErrorCode::EofWhileParsingList
            | ErrorCode::EofWhileParsingObject
            | ErrorCode::EofWhileParsingString
            | ErrorCode::EofWhileParsingValue => Category::Eof,
            ErrorCode::ExpectedColon
            | ErrorCode::ExpectedListCommaOrEnd
            | ErrorCode::ExpectedObjectCommaOrEnd
            | ErrorCode::ExpectedSomeIdent
            | ErrorCode::ExpectedSomeValue
            | ErrorCode::InvalidEscape
            | ErrorCode::InvalidNumber
            | ErrorCode::NumberOutOfRange
            | ErrorCode::InvalidUnicodeCodePoint
            | ErrorCode::ControlCharacterWhileParsingString
            | ErrorCode::KeyMustBeAString
            | ErrorCode::LoneLeadingSurrogateInHexEscape
            | ErrorCode::TrailingComma
            | ErrorCode::TrailingCharacters
            | ErrorCode::UnexpectedEndOfHexEscape
            | ErrorCode::RecursionLimitExceeded => Category::Syntax,
        }
    }
    /// Returns true if this error was caused by a failure to read or write
    /// bytes on an IO stream.
    pub fn is_io(&self) -> bool {
        self.classify() == Category::Io
    }
    /// Returns true if this error was caused by input that was not
    /// syntactically valid JSON.
    pub fn is_syntax(&self) -> bool {
        self.classify() == Category::Syntax
    }
    /// Returns true if this error was caused by input data that was
    /// semantically incorrect.
    ///
    /// For example, JSON containing a number is semantically incorrect when the
    /// type being deserialized into holds a String.
    pub fn is_data(&self) -> bool {
        self.classify() == Category::Data
    }
    /// Returns true if this error was caused by prematurely reaching the end of
    /// the input data.
    ///
    /// Callers that process streaming input may be interested in retrying the
    /// deserialization once more data is available.
    pub fn is_eof(&self) -> bool {
        self.classify() == Category::Eof
    }
}
/// Categorizes the cause of a `serde_json::Error`.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Category {
    /// The error was caused by a failure to read or write bytes on an IO
    /// stream.
    Io,
    /// The error was caused by input that was not syntactically valid JSON.
    Syntax,
    /// The error was caused by input data that was semantically incorrect.
    ///
    /// For example, JSON containing a number is semantically incorrect when the
    /// type being deserialized into holds a String.
    Data,
    /// The error was caused by prematurely reaching the end of the input data.
    ///
    /// Callers that process streaming input may be interested in retrying the
    /// deserialization once more data is available.
    Eof,
}
#[cfg(feature = "std")]
#[allow(clippy::fallible_impl_from)]
impl From<Error> for io::Error {
    /// Convert a `serde_json::Error` into an `io::Error`.
    ///
    /// JSON syntax and data errors are turned into `InvalidData` IO errors.
    /// EOF errors are turned into `UnexpectedEof` IO errors.
    ///
    /// ```
    /// use std::io;
    ///
    /// enum MyError {
    ///     Io(io::Error),
    ///     Json(serde_json::Error),
    /// }
    ///
    /// impl From<serde_json::Error> for MyError {
    ///     fn from(err: serde_json::Error) -> MyError {
    ///         use serde_json::error::Category;
    ///         match err.classify() {
    ///             Category::Io => {
    ///                 MyError::Io(err.into())
    ///             }
    ///             Category::Syntax | Category::Data | Category::Eof => {
    ///                 MyError::Json(err)
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    fn from(j: Error) -> Self {
        if let ErrorCode::Io(err) = j.err.code {
            err
        } else {
            match j.classify() {
                Category::Io => unreachable!(),
                Category::Syntax | Category::Data => {
                    io::Error::new(io::ErrorKind::InvalidData, j)
                }
                Category::Eof => io::Error::new(io::ErrorKind::UnexpectedEof, j),
            }
        }
    }
}
struct ErrorImpl {
    code: ErrorCode,
    line: usize,
    column: usize,
}
pub(crate) enum ErrorCode {
    /// Catchall for syntax error messages
    Message(Box<str>),
    /// Some IO error occurred while serializing or deserializing.
    Io(io::Error),
    /// EOF while parsing a list.
    EofWhileParsingList,
    /// EOF while parsing an object.
    EofWhileParsingObject,
    /// EOF while parsing a string.
    EofWhileParsingString,
    /// EOF while parsing a JSON value.
    EofWhileParsingValue,
    /// Expected this character to be a `':'`.
    ExpectedColon,
    /// Expected this character to be either a `','` or a `']'`.
    ExpectedListCommaOrEnd,
    /// Expected this character to be either a `','` or a `'}'`.
    ExpectedObjectCommaOrEnd,
    /// Expected to parse either a `true`, `false`, or a `null`.
    ExpectedSomeIdent,
    /// Expected this character to start a JSON value.
    ExpectedSomeValue,
    /// Invalid hex escape code.
    InvalidEscape,
    /// Invalid number.
    InvalidNumber,
    /// Number is bigger than the maximum value of its type.
    NumberOutOfRange,
    /// Invalid unicode code point.
    InvalidUnicodeCodePoint,
    /// Control character found while parsing a string.
    ControlCharacterWhileParsingString,
    /// Object key is not a string.
    KeyMustBeAString,
    /// Lone leading surrogate in hex escape.
    LoneLeadingSurrogateInHexEscape,
    /// JSON has a comma after the last value in an array or map.
    TrailingComma,
    /// JSON has non-whitespace trailing characters after the value.
    TrailingCharacters,
    /// Unexpected end of hex excape.
    UnexpectedEndOfHexEscape,
    /// Encountered nesting of JSON maps and arrays more than 128 layers deep.
    RecursionLimitExceeded,
}
impl Error {
    #[cold]
    pub(crate) fn syntax(code: ErrorCode, line: usize, column: usize) -> Self {
        Error {
            err: Box::new(ErrorImpl { code, line, column }),
        }
    }
    #[doc(hidden)]
    #[cold]
    pub fn io(error: io::Error) -> Self {
        Error {
            err: Box::new(ErrorImpl {
                code: ErrorCode::Io(error),
                line: 0,
                column: 0,
            }),
        }
    }
    #[cold]
    pub(crate) fn fix_position<F>(self, f: F) -> Self
    where
        F: FnOnce(ErrorCode) -> Error,
    {
        if self.err.line == 0 { f(self.err.code) } else { self }
    }
}
impl Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ErrorCode::Message(ref msg) => f.write_str(msg),
            ErrorCode::Io(ref err) => Display::fmt(err, f),
            ErrorCode::EofWhileParsingList => f.write_str("EOF while parsing a list"),
            ErrorCode::EofWhileParsingObject => {
                f.write_str("EOF while parsing an object")
            }
            ErrorCode::EofWhileParsingString => f.write_str("EOF while parsing a string"),
            ErrorCode::EofWhileParsingValue => f.write_str("EOF while parsing a value"),
            ErrorCode::ExpectedColon => f.write_str("expected `:`"),
            ErrorCode::ExpectedListCommaOrEnd => f.write_str("expected `,` or `]`"),
            ErrorCode::ExpectedObjectCommaOrEnd => f.write_str("expected `,` or `}`"),
            ErrorCode::ExpectedSomeIdent => f.write_str("expected ident"),
            ErrorCode::ExpectedSomeValue => f.write_str("expected value"),
            ErrorCode::InvalidEscape => f.write_str("invalid escape"),
            ErrorCode::InvalidNumber => f.write_str("invalid number"),
            ErrorCode::NumberOutOfRange => f.write_str("number out of range"),
            ErrorCode::InvalidUnicodeCodePoint => {
                f.write_str("invalid unicode code point")
            }
            ErrorCode::ControlCharacterWhileParsingString => {
                f
                    .write_str(
                        "control character (\\u0000-\\u001F) found while parsing a string",
                    )
            }
            ErrorCode::KeyMustBeAString => f.write_str("key must be a string"),
            ErrorCode::LoneLeadingSurrogateInHexEscape => {
                f.write_str("lone leading surrogate in hex escape")
            }
            ErrorCode::TrailingComma => f.write_str("trailing comma"),
            ErrorCode::TrailingCharacters => f.write_str("trailing characters"),
            ErrorCode::UnexpectedEndOfHexEscape => {
                f.write_str("unexpected end of hex escape")
            }
            ErrorCode::RecursionLimitExceeded => f.write_str("recursion limit exceeded"),
        }
    }
}
impl serde::de::StdError for Error {
    #[cfg(feature = "std")]
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self.err.code {
            ErrorCode::Io(ref err) => Some(err),
            _ => None,
        }
    }
}
impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&*self.err, f)
    }
}
impl Display for ErrorImpl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.line == 0 {
            Display::fmt(&self.code, f)
        } else {
            write!(f, "{} at line {} column {}", self.code, self.line, self.column)
        }
    }
}
impl Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f, "Error({:?}, line: {}, column: {})", self.err.code.to_string(), self.err
            .line, self.err.column
        )
    }
}
impl de::Error for Error {
    #[cold]
    fn custom<T: Display>(msg: T) -> Error {
        make_error(msg.to_string())
    }
    #[cold]
    fn invalid_type(unexp: de::Unexpected, exp: &dyn de::Expected) -> Self {
        if let de::Unexpected::Unit = unexp {
            Error::custom(format_args!("invalid type: null, expected {}", exp))
        } else {
            Error::custom(format_args!("invalid type: {}, expected {}", unexp, exp))
        }
    }
}
impl ser::Error for Error {
    #[cold]
    fn custom<T: Display>(msg: T) -> Error {
        make_error(msg.to_string())
    }
}
fn make_error(mut msg: String) -> Error {
    let (line, column) = parse_line_col(&mut msg).unwrap_or((0, 0));
    Error {
        err: Box::new(ErrorImpl {
            code: ErrorCode::Message(msg.into_boxed_str()),
            line,
            column,
        }),
    }
}
fn parse_line_col(msg: &mut String) -> Option<(usize, usize)> {
    let start_of_suffix = match msg.rfind(" at line ") {
        Some(index) => index,
        None => return None,
    };
    let start_of_line = start_of_suffix + " at line ".len();
    let mut end_of_line = start_of_line;
    while starts_with_digit(&msg[end_of_line..]) {
        end_of_line += 1;
    }
    if !msg[end_of_line..].starts_with(" column ") {
        return None;
    }
    let start_of_column = end_of_line + " column ".len();
    let mut end_of_column = start_of_column;
    while starts_with_digit(&msg[end_of_column..]) {
        end_of_column += 1;
    }
    if end_of_column < msg.len() {
        return None;
    }
    let line = match usize::from_str(&msg[start_of_line..end_of_line]) {
        Ok(line) => line,
        Err(_) => return None,
    };
    let column = match usize::from_str(&msg[start_of_column..end_of_column]) {
        Ok(column) => column,
        Err(_) => return None,
    };
    msg.truncate(start_of_suffix);
    Some((line, column))
}
fn starts_with_digit(slice: &str) -> bool {
    match slice.as_bytes().get(0) {
        None => false,
        Some(&byte) => byte >= b'0' && byte <= b'9',
    }
}
#[cfg(test)]
mod tests_llm_16_247_llm_16_246 {
    use super::*;
    use crate::*;
    use crate::*;
    use serde::de::{Error as _, Unexpected};
    #[test]
    fn test_custom_error() {
        let error = crate::Error::custom("custom error message");
        assert_eq!(
            error.to_string(), "Error(\"custom error message\", line: 0, column: 0)"
        );
        assert_eq!(error.line(), 0);
        assert_eq!(error.column(), 0);
        assert_eq!(error.classify(), crate ::error::Category::Syntax);
        assert!(! error.is_io());
        assert!(error.is_syntax());
        assert!(! error.is_data());
        assert!(error.is_eof());
    }
    #[test]
    fn test_custom_error_with_display() {
        let error = crate::Error::custom(format_args!("custom error message"));
        assert_eq!(
            error.to_string(), "Error(\"custom error message\", line: 0, column: 0)"
        );
        assert_eq!(error.line(), 0);
        assert_eq!(error.column(), 0);
        assert_eq!(error.classify(), crate ::error::Category::Syntax);
        assert!(! error.is_io());
        assert!(error.is_syntax());
        assert!(! error.is_data());
        assert!(error.is_eof());
    }
    #[test]
    fn test_custom_error_with_unexpected() {
        let unexpected = Unexpected::Str("unexpected");
        let error = crate::Error::invalid_type(unexpected, &"expected");
        assert_eq!(
            error.to_string(),
            "Error(\"invalid type: unexpected, expected expected\", line: 0, column: 0)"
        );
        assert_eq!(error.line(), 0);
        assert_eq!(error.column(), 0);
        assert_eq!(error.classify(), crate ::error::Category::Syntax);
        assert!(! error.is_io());
        assert!(error.is_syntax());
        assert!(! error.is_data());
        assert!(error.is_eof());
    }
}
#[cfg(test)]
mod tests_llm_16_249 {
    use std::error::Error;
    use crate::error::{Error as MyError, ErrorCode};
    #[test]
    fn test_source() {
        let _rug_st_tests_llm_16_249_rrrruuuugggg_test_source = 0;
        let rug_fuzz_0 = "Custom error";
        let err = MyError::io(
            std::io::Error::new(std::io::ErrorKind::Other, rug_fuzz_0),
        );
        debug_assert_eq!(err.source().unwrap().to_string(), "Custom error");
        let _rug_ed_tests_llm_16_249_rrrruuuugggg_test_source = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_876_llm_16_875 {
    use std::io;
    use crate::{Error, error::{Category, ErrorCode, ErrorImpl}};
    #[test]
    fn test_from() {
        let _rug_st_tests_llm_16_876_llm_16_875_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 1;
        let err = Error {
            err: Box::new(ErrorImpl {
                code: ErrorCode::EofWhileParsingList,
                line: rug_fuzz_0,
                column: rug_fuzz_1,
            }),
        };
        let result: io::Error = From::<Error>::from(err);
        debug_assert_eq!(result.kind(), io::ErrorKind::UnexpectedEof);
        let _rug_ed_tests_llm_16_876_llm_16_875_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_878 {
    use super::*;
    use crate::*;
    use crate::error::ErrorCode;
    #[test]
    fn test_classify() {
        let _rug_st_tests_llm_16_878_rrrruuuugggg_test_classify = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 1;
        let error = Error {
            err: Box::new(ErrorImpl {
                code: ErrorCode::EofWhileParsingList,
                line: rug_fuzz_0,
                column: rug_fuzz_1,
            }),
        };
        debug_assert_eq!(error.classify(), Category::Eof);
        let _rug_ed_tests_llm_16_878_rrrruuuugggg_test_classify = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_880_llm_16_879 {
    use crate::error::Error;
    use crate::error::ErrorCode;
    #[test]
    fn test_column() {
        let _rug_st_tests_llm_16_880_llm_16_879_rrrruuuugggg_test_column = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 5;
        let error = Error::syntax(ErrorCode::InvalidNumber, rug_fuzz_0, rug_fuzz_1);
        debug_assert_eq!(error.column(), 5);
        let _rug_ed_tests_llm_16_880_llm_16_879_rrrruuuugggg_test_column = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_883 {
    use std::error::Error as StdError;
    use std::fmt::{Debug, Display};
    use serde::de;
    use serde::ser;
    use crate::{
        error::{Category, ErrorCode},
        Error,
    };
    #[test]
    fn test_io_error() {
        let _rug_st_tests_llm_16_883_rrrruuuugggg_test_io_error = 0;
        let rug_fuzz_0 = "test error";
        let error = std::io::Error::new(std::io::ErrorKind::Other, rug_fuzz_0);
        let serde_error = Error::io(error);
        let io_error: std::io::Error = serde_error.into();
        debug_assert_eq!(io_error.kind(), std::io::ErrorKind::Other);
        debug_assert_eq!(io_error.description(), "test error");
        let _rug_ed_tests_llm_16_883_rrrruuuugggg_test_io_error = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_885_llm_16_884 {
    use super::*;
    use crate::*;
    use std::io;
    use std::error::Error as StdError;
    use std::fmt::{self, Display, Debug};
    use serde::de::{self, Unexpected};
    use serde::ser;
    #[test]
    fn test_is_data() {
        let _rug_st_tests_llm_16_885_llm_16_884_rrrruuuugggg_test_is_data = 0;
        let rug_fuzz_0 = "error message";
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let error = Error {
            err: Box::new(ErrorImpl {
                code: ErrorCode::Message(rug_fuzz_0.to_owned().into()),
                line: rug_fuzz_1,
                column: rug_fuzz_2,
            }),
        };
        debug_assert_eq!(error.is_data(), true);
        let _rug_ed_tests_llm_16_885_llm_16_884_rrrruuuugggg_test_is_data = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_886 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_eof() {
        let _rug_st_tests_llm_16_886_rrrruuuugggg_test_is_eof = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 1;
        let err = Error {
            err: Box::new(ErrorImpl {
                code: ErrorCode::EofWhileParsingList,
                line: rug_fuzz_0,
                column: rug_fuzz_1,
            }),
        };
        debug_assert_eq!(err.is_eof(), true);
        let _rug_ed_tests_llm_16_886_rrrruuuugggg_test_is_eof = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_890_llm_16_889 {
    use crate::error::{Error, ErrorCode};
    #[test]
    fn test_is_syntax_returns_true_when_error_is_syntax() {
        let _rug_st_tests_llm_16_890_llm_16_889_rrrruuuugggg_test_is_syntax_returns_true_when_error_is_syntax = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 1;
        let error = Error::syntax(ErrorCode::InvalidNumber, rug_fuzz_0, rug_fuzz_1);
        debug_assert_eq!(error.is_syntax(), true);
        let _rug_ed_tests_llm_16_890_llm_16_889_rrrruuuugggg_test_is_syntax_returns_true_when_error_is_syntax = 0;
    }
    #[test]
    fn test_is_syntax_returns_false_when_error_is_not_syntax() {
        let _rug_st_tests_llm_16_890_llm_16_889_rrrruuuugggg_test_is_syntax_returns_false_when_error_is_not_syntax = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 1;
        let error = Error::syntax(ErrorCode::InvalidEscape, rug_fuzz_0, rug_fuzz_1);
        debug_assert_eq!(error.is_syntax(), false);
        let _rug_ed_tests_llm_16_890_llm_16_889_rrrruuuugggg_test_is_syntax_returns_false_when_error_is_not_syntax = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_897 {
    use crate::error::parse_line_col;
    #[test]
    fn test_parse_line_col() {
        let _rug_st_tests_llm_16_897_rrrruuuugggg_test_parse_line_col = 0;
        let rug_fuzz_0 = "Error at line 10 column 5";
        let mut msg = String::from(rug_fuzz_0);
        let result = parse_line_col(&mut msg);
        debug_assert_eq!(result, Some((10, 5)));
        let _rug_ed_tests_llm_16_897_rrrruuuugggg_test_parse_line_col = 0;
    }
    #[test]
    fn test_parse_line_col_invalid() {
        let _rug_st_tests_llm_16_897_rrrruuuugggg_test_parse_line_col_invalid = 0;
        let rug_fuzz_0 = "Invalid error message";
        let mut msg = String::from(rug_fuzz_0);
        let result = parse_line_col(&mut msg);
        debug_assert_eq!(result, None);
        let _rug_ed_tests_llm_16_897_rrrruuuugggg_test_parse_line_col_invalid = 0;
    }
    #[test]
    fn test_parse_line_col_no_line() {
        let _rug_st_tests_llm_16_897_rrrruuuugggg_test_parse_line_col_no_line = 0;
        let rug_fuzz_0 = "Error at column 5";
        let mut msg = String::from(rug_fuzz_0);
        let result = parse_line_col(&mut msg);
        debug_assert_eq!(result, None);
        let _rug_ed_tests_llm_16_897_rrrruuuugggg_test_parse_line_col_no_line = 0;
    }
    #[test]
    fn test_parse_line_col_no_column() {
        let _rug_st_tests_llm_16_897_rrrruuuugggg_test_parse_line_col_no_column = 0;
        let rug_fuzz_0 = "Error at line 10";
        let mut msg = String::from(rug_fuzz_0);
        let result = parse_line_col(&mut msg);
        debug_assert_eq!(result, None);
        let _rug_ed_tests_llm_16_897_rrrruuuugggg_test_parse_line_col_no_column = 0;
    }
}
#[cfg(test)]
mod tests_rug_99 {
    use super::*;
    #[test]
    fn test_make_error() {
        let _rug_st_tests_rug_99_rrrruuuugggg_test_make_error = 0;
        let rug_fuzz_0 = "Sample error message";
        let mut p0 = String::from(rug_fuzz_0);
        crate::error::make_error(p0);
        let _rug_ed_tests_rug_99_rrrruuuugggg_test_make_error = 0;
    }
}
#[cfg(test)]
mod tests_rug_100 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_100_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "123ABC";
        let mut p0: &str = rug_fuzz_0;
        crate::error::starts_with_digit(&p0);
        let _rug_ed_tests_rug_100_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_101 {
    use super::*;
    use crate::error::Error;
    #[test]
    fn test_line() {
        let _rug_st_tests_rug_101_rrrruuuugggg_test_line = 0;
        let rug_fuzz_0 = "File not found";
        let err = Error::io(
            std::io::Error::new(std::io::ErrorKind::NotFound, rug_fuzz_0),
        );
        let p0 = err.line();
        debug_assert_eq!(p0, 0);
        let _rug_ed_tests_rug_101_rrrruuuugggg_test_line = 0;
    }
}
#[cfg(test)]
mod tests_rug_102 {
    use super::*;
    use crate::error::{Error, Category};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_102_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "test error";
        let p0: Error = Error::io(
            std::io::Error::new(std::io::ErrorKind::Other, rug_fuzz_0),
        );
        debug_assert_eq!(p0.is_io(), true);
        let _rug_ed_tests_rug_102_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_103 {
    use super::*;
    use crate::error::{Error, ErrorCode};
    #[test]
    fn test_rug() {
        #[cold]
        fn syntax(code: ErrorCode, line: usize, column: usize) -> Error {
            Error {
                err: Box::new(ErrorImpl { code, line, column }),
            }
        }
        let mut p0: ErrorCode = ErrorCode::Message(
            Box::<str>::from("Sample Error Message"),
        );
        let mut p1: usize = 1;
        let mut p2: usize = 2;
        Error::syntax(p0, p1, p2);
    }
}
