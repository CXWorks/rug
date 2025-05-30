//! Character specific parsers and combinators, streaming version
//!
//! Functions recognizing specific characters

use crate::branch::alt;
use crate::combinator::opt;
use crate::error::ErrorKind;
use crate::error::ParseError;
use crate::internal::{Err, IResult, Needed};
use crate::traits::{AsChar, FindToken, Input};
use crate::traits::{Compare, CompareResult};

/// Recognizes one character.
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data.
/// # Example
///
/// ```
/// # use nom::{Err, error::{ErrorKind, Error}, Needed, IResult};
/// # use nom::character::streaming::char;
/// fn parser(i: &str) -> IResult<&str, char> {
///     char('a')(i)
/// }
/// assert_eq!(parser("abc"), Ok(("bc", 'a')));
/// assert_eq!(parser("bc"), Err(Err::Error(Error::new("bc", ErrorKind::Char))));
/// assert_eq!(parser(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn char<I, Error: ParseError<I>>(c: char) -> impl Fn(I) -> IResult<I, char, Error>
where
  I: Input,
  <I as Input>::Item: AsChar,
{
  move |i: I| match (i).iter_elements().next().map(|t| {
    let b = t.as_char() == c;
    (&c, b)
  }) {
    None => Err(Err::Incomplete(Needed::new(c.len() - i.input_len()))),
    Some((_, false)) => Err(Err::Error(Error::from_char(i, c))),
    Some((c, true)) => Ok((i.take_from(c.len()), c.as_char())),
  }
}

/// Recognizes one character and checks that it satisfies a predicate
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data.
/// # Example
///
/// ```
/// # use nom::{Err, error::{ErrorKind, Error}, Needed, IResult};
/// # use nom::character::streaming::satisfy;
/// fn parser(i: &str) -> IResult<&str, char> {
///     satisfy(|c| c == 'a' || c == 'b')(i)
/// }
/// assert_eq!(parser("abc"), Ok(("bc", 'a')));
/// assert_eq!(parser("cd"), Err(Err::Error(Error::new("cd", ErrorKind::Satisfy))));
/// assert_eq!(parser(""), Err(Err::Incomplete(Needed::Unknown)));
/// ```
pub fn satisfy<F, I, Error: ParseError<I>>(cond: F) -> impl Fn(I) -> IResult<I, char, Error>
where
  I: Input,
  <I as Input>::Item: AsChar,
  F: Fn(char) -> bool,
{
  move |i: I| match (i).iter_elements().next().map(|t| {
    let c = t.as_char();
    let b = cond(c);
    (c, b)
  }) {
    None => Err(Err::Incomplete(Needed::Unknown)),
    Some((_, false)) => Err(Err::Error(Error::from_error_kind(i, ErrorKind::Satisfy))),
    Some((c, true)) => Ok((i.take_from(c.len()), c)),
  }
}

/// Recognizes one of the provided characters.
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data.
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::character::streaming::one_of;
/// assert_eq!(one_of::<_, _, (_, ErrorKind)>("abc")("b"), Ok(("", 'b')));
/// assert_eq!(one_of::<_, _, (_, ErrorKind)>("a")("bc"), Err(Err::Error(("bc", ErrorKind::OneOf))));
/// assert_eq!(one_of::<_, _, (_, ErrorKind)>("a")(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn one_of<I, T, Error: ParseError<I>>(list: T) -> impl Fn(I) -> IResult<I, char, Error>
where
  I: Input,
  <I as Input>::Item: AsChar,
  T: FindToken<<I as Input>::Item>,
{
  move |i: I| match (i).iter_elements().next().map(|c| (c, list.find_token(c))) {
    None => Err(Err::Incomplete(Needed::new(1))),
    Some((_, false)) => Err(Err::Error(Error::from_error_kind(i, ErrorKind::OneOf))),
    Some((c, true)) => Ok((i.take_from(c.len()), c.as_char())),
  }
}

/// Recognizes a character that is not in the provided characters.
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data.
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::character::streaming::none_of;
/// assert_eq!(none_of::<_, _, (_, ErrorKind)>("abc")("z"), Ok(("", 'z')));
/// assert_eq!(none_of::<_, _, (_, ErrorKind)>("ab")("a"), Err(Err::Error(("a", ErrorKind::NoneOf))));
/// assert_eq!(none_of::<_, _, (_, ErrorKind)>("a")(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn none_of<I, T, Error: ParseError<I>>(list: T) -> impl Fn(I) -> IResult<I, char, Error>
where
  I: Input,
  <I as Input>::Item: AsChar,
  T: FindToken<<I as Input>::Item>,
{
  move |i: I| match (i).iter_elements().next().map(|c| (c, !list.find_token(c))) {
    None => Err(Err::Incomplete(Needed::new(1))),
    Some((_, false)) => Err(Err::Error(Error::from_error_kind(i, ErrorKind::NoneOf))),
    Some((c, true)) => Ok((i.take_from(c.len()), c.as_char())),
  }
}

/// Recognizes the string "\r\n".
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data.
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::streaming::crlf;
/// assert_eq!(crlf::<_, (_, ErrorKind)>("\r\nc"), Ok(("c", "\r\n")));
/// assert_eq!(crlf::<_, (_, ErrorKind)>("ab\r\nc"), Err(Err::Error(("ab\r\nc", ErrorKind::CrLf))));
/// assert_eq!(crlf::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(2))));
/// ```
pub fn crlf<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: Input,
  T: Compare<&'static str>,
{
  match input.compare("\r\n") {
    //FIXME: is this the right index?
    CompareResult::Ok => Ok(input.take_split(2)),
    CompareResult::Incomplete => Err(Err::Incomplete(Needed::new(2))),
    CompareResult::Error => {
      let e: ErrorKind = ErrorKind::CrLf;
      Err(Err::Error(E::from_error_kind(input, e)))
    }
  }
}

/// Recognizes a string of any char except '\r\n' or '\n'.
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data.
/// # Example
///
/// ```
/// # use nom::{Err, error::{Error, ErrorKind}, IResult, Needed};
/// # use nom::character::streaming::not_line_ending;
/// assert_eq!(not_line_ending::<_, (_, ErrorKind)>("ab\r\nc"), Ok(("\r\nc", "ab")));
/// assert_eq!(not_line_ending::<_, (_, ErrorKind)>("abc"), Err(Err::Incomplete(Needed::Unknown)));
/// assert_eq!(not_line_ending::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::Unknown)));
/// assert_eq!(not_line_ending::<_, (_, ErrorKind)>("a\rb\nc"), Err(Err::Error(("a\rb\nc", ErrorKind::Tag ))));
/// assert_eq!(not_line_ending::<_, (_, ErrorKind)>("a\rbc"), Err(Err::Error(("a\rbc", ErrorKind::Tag ))));
/// ```
pub fn not_line_ending<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: Input,
  T: Compare<&'static str>,
  <T as Input>::Item: AsChar,
{
  match input.position(|item| {
    let c = item.as_char();
    c == '\r' || c == '\n'
  }) {
    None => Err(Err::Incomplete(Needed::Unknown)),
    Some(index) => {
      let mut it = input.take_from(index).iter_elements();
      let nth = it.next().unwrap().as_char();
      if nth == '\r' {
        let sliced = input.take_from(index);
        let comp = sliced.compare("\r\n");
        match comp {
          //FIXME: calculate the right index
          CompareResult::Incomplete => Err(Err::Incomplete(Needed::Unknown)),
          CompareResult::Error => {
            let e: ErrorKind = ErrorKind::Tag;
            Err(Err::Error(E::from_error_kind(input, e)))
          }
          CompareResult::Ok => Ok(input.take_split(index)),
        }
      } else {
        Ok(input.take_split(index))
      }
    }
  }
}

/// Recognizes an end of line (both '\n' and '\r\n').
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data.
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::streaming::line_ending;
/// assert_eq!(line_ending::<_, (_, ErrorKind)>("\r\nc"), Ok(("c", "\r\n")));
/// assert_eq!(line_ending::<_, (_, ErrorKind)>("ab\r\nc"), Err(Err::Error(("ab\r\nc", ErrorKind::CrLf))));
/// assert_eq!(line_ending::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn line_ending<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: Input,
  T: Compare<&'static str>,
{
  match input.compare("\n") {
    CompareResult::Ok => Ok(input.take_split(1)),
    CompareResult::Incomplete => Err(Err::Incomplete(Needed::new(1))),
    CompareResult::Error => {
      match input.compare("\r\n") {
        //FIXME: is this the right index?
        CompareResult::Ok => Ok(input.take_split(2)),
        CompareResult::Incomplete => Err(Err::Incomplete(Needed::new(2))),
        CompareResult::Error => Err(Err::Error(E::from_error_kind(input, ErrorKind::CrLf))),
      }
    }
  }
}

/// Matches a newline character '\\n'.
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data.
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::streaming::newline;
/// assert_eq!(newline::<_, (_, ErrorKind)>("\nc"), Ok(("c", '\n')));
/// assert_eq!(newline::<_, (_, ErrorKind)>("\r\nc"), Err(Err::Error(("\r\nc", ErrorKind::Char))));
/// assert_eq!(newline::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn newline<I, Error: ParseError<I>>(input: I) -> IResult<I, char, Error>
where
  I: Input,
  <I as Input>::Item: AsChar,
{
  char('\n')(input)
}

/// Matches a tab character '\t'.
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data.
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::streaming::tab;
/// assert_eq!(tab::<_, (_, ErrorKind)>("\tc"), Ok(("c", '\t')));
/// assert_eq!(tab::<_, (_, ErrorKind)>("\r\nc"), Err(Err::Error(("\r\nc", ErrorKind::Char))));
/// assert_eq!(tab::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn tab<I, Error: ParseError<I>>(input: I) -> IResult<I, char, Error>
where
  I: Input,
  <I as Input>::Item: AsChar,
{
  char('\t')(input)
}

/// Matches one byte as a character. Note that the input type will
/// accept a `str`, but not a `&[u8]`, unlike many other nom parsers.
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data.
/// # Example
///
/// ```
/// # use nom::{character::streaming::anychar, Err, error::ErrorKind, IResult, Needed};
/// assert_eq!(anychar::<_, (_, ErrorKind)>("abc"), Ok(("bc",'a')));
/// assert_eq!(anychar::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn anychar<T, E: ParseError<T>>(input: T) -> IResult<T, char, E>
where
  T: Input,
  <T as Input>::Item: AsChar,
{
  let mut it = input.iter_elements();
  match it.next() {
    None => Err(Err::Incomplete(Needed::new(1))),
    Some(c) => Ok((input.take_from(c.len()), c.as_char())),
  }
}

/// Recognizes zero or more lowercase and uppercase ASCII alphabetic characters: a-z, A-Z
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data,
/// or if no terminating token is found (a non alphabetic character).
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::streaming::alpha0;
/// assert_eq!(alpha0::<_, (_, ErrorKind)>("ab1c"), Ok(("1c", "ab")));
/// assert_eq!(alpha0::<_, (_, ErrorKind)>("1c"), Ok(("1c", "")));
/// assert_eq!(alpha0::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn alpha0<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: Input,
  <T as Input>::Item: AsChar,
{
  input.split_at_position(|item| !item.is_alpha())
}

/// Recognizes one or more lowercase and uppercase ASCII alphabetic characters: a-z, A-Z
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data,
/// or if no terminating token is found (a non alphabetic character).
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::streaming::alpha1;
/// assert_eq!(alpha1::<_, (_, ErrorKind)>("aB1c"), Ok(("1c", "aB")));
/// assert_eq!(alpha1::<_, (_, ErrorKind)>("1c"), Err(Err::Error(("1c", ErrorKind::Alpha))));
/// assert_eq!(alpha1::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn alpha1<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: Input,
  <T as Input>::Item: AsChar,
{
  input.split_at_position1(|item| !item.is_alpha(), ErrorKind::Alpha)
}

/// Recognizes zero or more ASCII numerical characters: 0-9
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data,
/// or if no terminating token is found (a non digit character).
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::streaming::digit0;
/// assert_eq!(digit0::<_, (_, ErrorKind)>("21c"), Ok(("c", "21")));
/// assert_eq!(digit0::<_, (_, ErrorKind)>("a21c"), Ok(("a21c", "")));
/// assert_eq!(digit0::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn digit0<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: Input,
  <T as Input>::Item: AsChar,
{
  input.split_at_position(|item| !item.is_dec_digit())
}

/// Recognizes one or more ASCII numerical characters: 0-9
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data,
/// or if no terminating token is found (a non digit character).
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::streaming::digit1;
/// assert_eq!(digit1::<_, (_, ErrorKind)>("21c"), Ok(("c", "21")));
/// assert_eq!(digit1::<_, (_, ErrorKind)>("c1"), Err(Err::Error(("c1", ErrorKind::Digit))));
/// assert_eq!(digit1::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn digit1<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: Input,
  <T as Input>::Item: AsChar,
{
  input.split_at_position1(|item| !item.is_dec_digit(), ErrorKind::Digit)
}

/// Recognizes zero or more ASCII hexadecimal numerical characters: 0-9, A-F, a-f
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data,
/// or if no terminating token is found (a non hexadecimal digit character).
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::streaming::hex_digit0;
/// assert_eq!(hex_digit0::<_, (_, ErrorKind)>("21cZ"), Ok(("Z", "21c")));
/// assert_eq!(hex_digit0::<_, (_, ErrorKind)>("Z21c"), Ok(("Z21c", "")));
/// assert_eq!(hex_digit0::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn hex_digit0<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: Input,
  <T as Input>::Item: AsChar,
{
  input.split_at_position(|item| !item.is_hex_digit())
}

/// Recognizes one or more ASCII hexadecimal numerical characters: 0-9, A-F, a-f
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data,
/// or if no terminating token is found (a non hexadecimal digit character).
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::streaming::hex_digit1;
/// assert_eq!(hex_digit1::<_, (_, ErrorKind)>("21cZ"), Ok(("Z", "21c")));
/// assert_eq!(hex_digit1::<_, (_, ErrorKind)>("H2"), Err(Err::Error(("H2", ErrorKind::HexDigit))));
/// assert_eq!(hex_digit1::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn hex_digit1<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: Input,
  <T as Input>::Item: AsChar,
{
  input.split_at_position1(|item| !item.is_hex_digit(), ErrorKind::HexDigit)
}

/// Recognizes zero or more octal characters: 0-7
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data,
/// or if no terminating token is found (a non octal digit character).
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::streaming::oct_digit0;
/// assert_eq!(oct_digit0::<_, (_, ErrorKind)>("21cZ"), Ok(("cZ", "21")));
/// assert_eq!(oct_digit0::<_, (_, ErrorKind)>("Z21c"), Ok(("Z21c", "")));
/// assert_eq!(oct_digit0::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn oct_digit0<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: Input,
  <T as Input>::Item: AsChar,
{
  input.split_at_position(|item| !item.is_oct_digit())
}

/// Recognizes one or more octal characters: 0-7
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data,
/// or if no terminating token is found (a non octal digit character).
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::streaming::oct_digit1;
/// assert_eq!(oct_digit1::<_, (_, ErrorKind)>("21cZ"), Ok(("cZ", "21")));
/// assert_eq!(oct_digit1::<_, (_, ErrorKind)>("H2"), Err(Err::Error(("H2", ErrorKind::OctDigit))));
/// assert_eq!(oct_digit1::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn oct_digit1<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: Input,
  <T as Input>::Item: AsChar,
{
  input.split_at_position1(|item| !item.is_oct_digit(), ErrorKind::OctDigit)
}

/// Recognizes zero or more ASCII numerical and alphabetic characters: 0-9, a-z, A-Z
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data,
/// or if no terminating token is found (a non alphanumerical character).
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::streaming::alphanumeric0;
/// assert_eq!(alphanumeric0::<_, (_, ErrorKind)>("21cZ%1"), Ok(("%1", "21cZ")));
/// assert_eq!(alphanumeric0::<_, (_, ErrorKind)>("&Z21c"), Ok(("&Z21c", "")));
/// assert_eq!(alphanumeric0::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn alphanumeric0<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: Input,
  <T as Input>::Item: AsChar,
{
  input.split_at_position(|item| !item.is_alphanum())
}

/// Recognizes one or more ASCII numerical and alphabetic characters: 0-9, a-z, A-Z
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data,
/// or if no terminating token is found (a non alphanumerical character).
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::streaming::alphanumeric1;
/// assert_eq!(alphanumeric1::<_, (_, ErrorKind)>("21cZ%1"), Ok(("%1", "21cZ")));
/// assert_eq!(alphanumeric1::<_, (_, ErrorKind)>("&H2"), Err(Err::Error(("&H2", ErrorKind::AlphaNumeric))));
/// assert_eq!(alphanumeric1::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn alphanumeric1<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: Input,
  <T as Input>::Item: AsChar,
{
  input.split_at_position1(|item| !item.is_alphanum(), ErrorKind::AlphaNumeric)
}

/// Recognizes zero or more spaces and tabs.
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data,
/// or if no terminating token is found (a non space character).
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::streaming::space0;
/// assert_eq!(space0::<_, (_, ErrorKind)>(" \t21c"), Ok(("21c", " \t")));
/// assert_eq!(space0::<_, (_, ErrorKind)>("Z21c"), Ok(("Z21c", "")));
/// assert_eq!(space0::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn space0<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: Input,
  <T as Input>::Item: AsChar,
{
  input.split_at_position(|item| {
    let c = item.as_char();
    !(c == ' ' || c == '\t')
  })
}
/// Recognizes one or more spaces and tabs.
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data,
/// or if no terminating token is found (a non space character).
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::streaming::space1;
/// assert_eq!(space1::<_, (_, ErrorKind)>(" \t21c"), Ok(("21c", " \t")));
/// assert_eq!(space1::<_, (_, ErrorKind)>("H2"), Err(Err::Error(("H2", ErrorKind::Space))));
/// assert_eq!(space1::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn space1<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: Input,
  <T as Input>::Item: AsChar,
{
  input.split_at_position1(
    |item| {
      let c = item.as_char();
      !(c == ' ' || c == '\t')
    },
    ErrorKind::Space,
  )
}

/// Recognizes zero or more spaces, tabs, carriage returns and line feeds.
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data,
/// or if no terminating token is found (a non space character).
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::streaming::multispace0;
/// assert_eq!(multispace0::<_, (_, ErrorKind)>(" \t\n\r21c"), Ok(("21c", " \t\n\r")));
/// assert_eq!(multispace0::<_, (_, ErrorKind)>("Z21c"), Ok(("Z21c", "")));
/// assert_eq!(multispace0::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn multispace0<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: Input,
  <T as Input>::Item: AsChar,
{
  input.split_at_position(|item| {
    let c = item.as_char();
    !(c == ' ' || c == '\t' || c == '\r' || c == '\n')
  })
}

/// Recognizes one or more spaces, tabs, carriage returns and line feeds.
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data,
/// or if no terminating token is found (a non space character).
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::streaming::multispace1;
/// assert_eq!(multispace1::<_, (_, ErrorKind)>(" \t\n\r21c"), Ok(("21c", " \t\n\r")));
/// assert_eq!(multispace1::<_, (_, ErrorKind)>("H2"), Err(Err::Error(("H2", ErrorKind::MultiSpace))));
/// assert_eq!(multispace1::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn multispace1<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: Input,
  <T as Input>::Item: AsChar,
{
  input.split_at_position1(
    |item| {
      let c = item.as_char();
      !(c == ' ' || c == '\t' || c == '\r' || c == '\n')
    },
    ErrorKind::MultiSpace,
  )
}

pub(crate) fn sign<T, E: ParseError<T>>(input: T) -> IResult<T, bool, E>
where
  T: Clone + Input,
  T: for<'a> Compare<&'a [u8]>,
{
  use crate::bytes::streaming::tag;
  use crate::combinator::value;

  let (i, opt_sign) = opt(alt((
    value(false, tag(&b"-"[..])),
    value(true, tag(&b"+"[..])),
  )))(input)?;
  let sign = opt_sign.unwrap_or(true);

  Ok((i, sign))
}

#[doc(hidden)]
macro_rules! ints {
    ($($t:tt)+) => {
        $(
        /// will parse a number in text form to a number
        ///
        /// *Complete version*: can parse until the end of input.
        pub fn $t<T, E: ParseError<T>>(input: T) -> IResult<T, $t, E>
            where
            T: Input +  Clone,
            <T as Input>::Item: AsChar,
            T: for <'a> Compare<&'a[u8]>,
            {
              let (i, sign) = sign(input.clone())?;

                if i.input_len() == 0 {
                    return Err(Err::Incomplete(Needed::new(1)));
                }

                let mut value: $t = 0;
                if sign {
                    let mut pos = 0;
                    for c in i.iter_elements() {
                        match c.as_char().to_digit(10) {
                            None => {
                                if pos == 0 {
                                    return Err(Err::Error(E::from_error_kind(input, ErrorKind::Digit)));
                                } else {
                                    return Ok((i.take_from(pos), value));
                                }
                            },
                            Some(d) => match value.checked_mul(10).and_then(|v| v.checked_add(d as $t)) {
                                None => return Err(Err::Error(E::from_error_kind(input, ErrorKind::Digit))),
                                Some(v) => {
                                  pos += c.len();
                                  value = v;
                                },
                            }
                        }
                    }
                } else {
                    let mut pos = 0;
                    for c in i.iter_elements() {
                        match c.as_char().to_digit(10) {
                            None => {
                                if pos == 0 {
                                    return Err(Err::Error(E::from_error_kind(input, ErrorKind::Digit)));
                                } else {
                                    return Ok((i.take_from(pos), value));
                                }
                            },
                            Some(d) => match value.checked_mul(10).and_then(|v| v.checked_sub(d as $t)) {
                                None => return Err(Err::Error(E::from_error_kind(input, ErrorKind::Digit))),
                                Some(v) => {
                                  pos += c.len();
                                  value = v;
                                },
                            }
                        }
                    }
                }

                Err(Err::Incomplete(Needed::new(1)))
            }
        )+
    }
}

ints! { i8 i16 i32 i64 i128 }

#[doc(hidden)]
macro_rules! uints {
    ($($t:tt)+) => {
        $(
        /// will parse a number in text form to a number
        ///
        /// *Complete version*: can parse until the end of input.
        pub fn $t<T, E: ParseError<T>>(input: T) -> IResult<T, $t, E>
            where
            T: Input ,
            <T as Input>::Item: AsChar,
            {
                let i = input;

                if i.input_len() == 0 {
                    return Err(Err::Incomplete(Needed::new(1)));
                }

                let mut value: $t = 0;
                let mut pos = 0;
                for c in i.iter_elements() {
                    match c.as_char().to_digit(10) {
                        None => {
                            if pos == 0 {
                                return Err(Err::Error(E::from_error_kind(i, ErrorKind::Digit)));
                            } else {
                                return Ok((i.take_from(pos), value));
                            }
                        },
                        Some(d) => match value.checked_mul(10).and_then(|v| v.checked_add(d as $t)) {
                            None => return Err(Err::Error(E::from_error_kind(i, ErrorKind::Digit))),
                            Some(v) => {
                              pos += c.len();
                              value = v;
                            },
                        }
                    }
                }

                Err(Err::Incomplete(Needed::new(1)))
            }
        )+
    }
}

uints! { u8 u16 u32 u64 u128 }

#[cfg(test)]
mod tests {
  use super::*;
  use crate::error::ErrorKind;
  use crate::internal::{Err, Needed};
  use crate::sequence::pair;
  use crate::traits::ParseTo;
  use proptest::prelude::*;

  macro_rules! assert_parse(
    ($left: expr, $right: expr) => {
      let res: $crate::IResult<_, _, (_, ErrorKind)> = $left;
      assert_eq!(res, $right);
    };
  );

  #[test]
  fn anychar_str() {
    use super::anychar;
    assert_eq!(anychar::<_, (&str, ErrorKind)>("Ә"), Ok(("", 'Ә')));
  }

  #[test]
  fn character() {
    let a: &[u8] = b"abcd";
    let b: &[u8] = b"1234";
    let c: &[u8] = b"a123";
    let d: &[u8] = "azé12".as_bytes();
    let e: &[u8] = b" ";
    let f: &[u8] = b" ;";
    //assert_eq!(alpha1::<_, (_, ErrorKind)>(a), Err(Err::Incomplete(Needed::new(1))));
    assert_parse!(alpha1(a), Err(Err::Incomplete(Needed::new(1))));
    assert_eq!(alpha1(b), Err(Err::Error((b, ErrorKind::Alpha))));
    assert_eq!(alpha1::<_, (_, ErrorKind)>(c), Ok((&c[1..], &b"a"[..])));
    assert_eq!(
      alpha1::<_, (_, ErrorKind)>(d),
      Ok(("é12".as_bytes(), &b"az"[..]))
    );
    assert_eq!(digit1(a), Err(Err::Error((a, ErrorKind::Digit))));
    assert_eq!(
      digit1::<_, (_, ErrorKind)>(b),
      Err(Err::Incomplete(Needed::new(1)))
    );
    assert_eq!(digit1(c), Err(Err::Error((c, ErrorKind::Digit))));
    assert_eq!(digit1(d), Err(Err::Error((d, ErrorKind::Digit))));
    assert_eq!(
      hex_digit1::<_, (_, ErrorKind)>(a),
      Err(Err::Incomplete(Needed::new(1)))
    );
    assert_eq!(
      hex_digit1::<_, (_, ErrorKind)>(b),
      Err(Err::Incomplete(Needed::new(1)))
    );
    assert_eq!(
      hex_digit1::<_, (_, ErrorKind)>(c),
      Err(Err::Incomplete(Needed::new(1)))
    );
    assert_eq!(
      hex_digit1::<_, (_, ErrorKind)>(d),
      Ok(("zé12".as_bytes(), &b"a"[..]))
    );
    assert_eq!(hex_digit1(e), Err(Err::Error((e, ErrorKind::HexDigit))));
    assert_eq!(oct_digit1(a), Err(Err::Error((a, ErrorKind::OctDigit))));
    assert_eq!(
      oct_digit1::<_, (_, ErrorKind)>(b),
      Err(Err::Incomplete(Needed::new(1)))
    );
    assert_eq!(oct_digit1(c), Err(Err::Error((c, ErrorKind::OctDigit))));
    assert_eq!(oct_digit1(d), Err(Err::Error((d, ErrorKind::OctDigit))));
    assert_eq!(
      alphanumeric1::<_, (_, ErrorKind)>(a),
      Err(Err::Incomplete(Needed::new(1)))
    );
    //assert_eq!(fix_error!(b,(), alphanumeric1), Ok((empty, b)));
    assert_eq!(
      alphanumeric1::<_, (_, ErrorKind)>(c),
      Err(Err::Incomplete(Needed::new(1)))
    );
    assert_eq!(
      alphanumeric1::<_, (_, ErrorKind)>(d),
      Ok(("é12".as_bytes(), &b"az"[..]))
    );
    assert_eq!(
      space1::<_, (_, ErrorKind)>(e),
      Err(Err::Incomplete(Needed::new(1)))
    );
    assert_eq!(space1::<_, (_, ErrorKind)>(f), Ok((&b";"[..], &b" "[..])));
  }

  #[cfg(feature = "alloc")]
  #[test]
  fn character_s() {
    let a = "abcd";
    let b = "1234";
    let c = "a123";
    let d = "azé12";
    let e = " ";
    assert_eq!(
      alpha1::<_, (_, ErrorKind)>(a),
      Err(Err::Incomplete(Needed::new(1)))
    );
    assert_eq!(alpha1(b), Err(Err::Error((b, ErrorKind::Alpha))));
    assert_eq!(alpha1::<_, (_, ErrorKind)>(c), Ok((&c[1..], "a")));
    assert_eq!(alpha1::<_, (_, ErrorKind)>(d), Ok(("é12", "az")));
    assert_eq!(digit1(a), Err(Err::Error((a, ErrorKind::Digit))));
    assert_eq!(
      digit1::<_, (_, ErrorKind)>(b),
      Err(Err::Incomplete(Needed::new(1)))
    );
    assert_eq!(digit1(c), Err(Err::Error((c, ErrorKind::Digit))));
    assert_eq!(digit1(d), Err(Err::Error((d, ErrorKind::Digit))));
    assert_eq!(
      hex_digit1::<_, (_, ErrorKind)>(a),
      Err(Err::Incomplete(Needed::new(1)))
    );
    assert_eq!(
      hex_digit1::<_, (_, ErrorKind)>(b),
      Err(Err::Incomplete(Needed::new(1)))
    );
    assert_eq!(
      hex_digit1::<_, (_, ErrorKind)>(c),
      Err(Err::Incomplete(Needed::new(1)))
    );
    assert_eq!(hex_digit1::<_, (_, ErrorKind)>(d), Ok(("zé12", "a")));
    assert_eq!(hex_digit1(e), Err(Err::Error((e, ErrorKind::HexDigit))));
    assert_eq!(oct_digit1(a), Err(Err::Error((a, ErrorKind::OctDigit))));
    assert_eq!(
      oct_digit1::<_, (_, ErrorKind)>(b),
      Err(Err::Incomplete(Needed::new(1)))
    );
    assert_eq!(oct_digit1(c), Err(Err::Error((c, ErrorKind::OctDigit))));
    assert_eq!(oct_digit1(d), Err(Err::Error((d, ErrorKind::OctDigit))));
    assert_eq!(
      alphanumeric1::<_, (_, ErrorKind)>(a),
      Err(Err::Incomplete(Needed::new(1)))
    );
    //assert_eq!(fix_error!(b,(), alphanumeric1), Ok((empty, b)));
    assert_eq!(
      alphanumeric1::<_, (_, ErrorKind)>(c),
      Err(Err::Incomplete(Needed::new(1)))
    );
    assert_eq!(alphanumeric1::<_, (_, ErrorKind)>(d), Ok(("é12", "az")));
    assert_eq!(
      space1::<_, (_, ErrorKind)>(e),
      Err(Err::Incomplete(Needed::new(1)))
    );
  }

  use crate::traits::Offset;
  #[test]
  fn offset() {
    let a = &b"abcd;"[..];
    let b = &b"1234;"[..];
    let c = &b"a123;"[..];
    let d = &b" \t;"[..];
    let e = &b" \t\r\n;"[..];
    let f = &b"123abcDEF;"[..];

    match alpha1::<_, (_, ErrorKind)>(a) {
      Ok((i, _)) => {
        assert_eq!(a.offset(i) + i.len(), a.len());
      }
      _ => panic!("wrong return type in offset test for alpha"),
    }
    match digit1::<_, (_, ErrorKind)>(b) {
      Ok((i, _)) => {
        assert_eq!(b.offset(i) + i.len(), b.len());
      }
      _ => panic!("wrong return type in offset test for digit"),
    }
    match alphanumeric1::<_, (_, ErrorKind)>(c) {
      Ok((i, _)) => {
        assert_eq!(c.offset(i) + i.len(), c.len());
      }
      _ => panic!("wrong return type in offset test for alphanumeric"),
    }
    match space1::<_, (_, ErrorKind)>(d) {
      Ok((i, _)) => {
        assert_eq!(d.offset(i) + i.len(), d.len());
      }
      _ => panic!("wrong return type in offset test for space"),
    }
    match multispace1::<_, (_, ErrorKind)>(e) {
      Ok((i, _)) => {
        assert_eq!(e.offset(i) + i.len(), e.len());
      }
      _ => panic!("wrong return type in offset test for multispace"),
    }
    match hex_digit1::<_, (_, ErrorKind)>(f) {
      Ok((i, _)) => {
        assert_eq!(f.offset(i) + i.len(), f.len());
      }
      _ => panic!("wrong return type in offset test for hex_digit"),
    }
    match oct_digit1::<_, (_, ErrorKind)>(f) {
      Ok((i, _)) => {
        assert_eq!(f.offset(i) + i.len(), f.len());
      }
      _ => panic!("wrong return type in offset test for oct_digit"),
    }
  }

  #[test]
  fn is_not_line_ending_bytes() {
    let a: &[u8] = b"ab12cd\nefgh";
    assert_eq!(
      not_line_ending::<_, (_, ErrorKind)>(a),
      Ok((&b"\nefgh"[..], &b"ab12cd"[..]))
    );

    let b: &[u8] = b"ab12cd\nefgh\nijkl";
    assert_eq!(
      not_line_ending::<_, (_, ErrorKind)>(b),
      Ok((&b"\nefgh\nijkl"[..], &b"ab12cd"[..]))
    );

    let c: &[u8] = b"ab12cd\r\nefgh\nijkl";
    assert_eq!(
      not_line_ending::<_, (_, ErrorKind)>(c),
      Ok((&b"\r\nefgh\nijkl"[..], &b"ab12cd"[..]))
    );

    let d: &[u8] = b"ab12cd";
    assert_eq!(
      not_line_ending::<_, (_, ErrorKind)>(d),
      Err(Err::Incomplete(Needed::Unknown))
    );
  }

  #[test]
  fn is_not_line_ending_str() {
    /*
    let a: &str = "ab12cd\nefgh";
    assert_eq!(not_line_ending(a), Ok((&"\nefgh"[..], &"ab12cd"[..])));

    let b: &str = "ab12cd\nefgh\nijkl";
    assert_eq!(not_line_ending(b), Ok((&"\nefgh\nijkl"[..], &"ab12cd"[..])));

    let c: &str = "ab12cd\r\nefgh\nijkl";
    assert_eq!(not_line_ending(c), Ok((&"\r\nefgh\nijkl"[..], &"ab12cd"[..])));

    let d = "βèƒôřè\nÂßÇáƒƭèř";
    assert_eq!(not_line_ending(d), Ok((&"\nÂßÇáƒƭèř"[..], &"βèƒôřè"[..])));

    let e = "βèƒôřè\r\nÂßÇáƒƭèř";
    assert_eq!(not_line_ending(e), Ok((&"\r\nÂßÇáƒƭèř"[..], &"βèƒôřè"[..])));
    */

    let f = "βèƒôřè\rÂßÇáƒƭèř";
    assert_eq!(not_line_ending(f), Err(Err::Error((f, ErrorKind::Tag))));

    let g2: &str = "ab12cd";
    assert_eq!(
      not_line_ending::<_, (_, ErrorKind)>(g2),
      Err(Err::Incomplete(Needed::Unknown))
    );
  }

  #[test]
  fn hex_digit_test() {
    let i = &b"0123456789abcdefABCDEF;"[..];
    assert_parse!(hex_digit1(i), Ok((&b";"[..], &i[..i.len() - 1])));

    let i = &b"g"[..];
    assert_parse!(
      hex_digit1(i),
      Err(Err::Error(error_position!(i, ErrorKind::HexDigit)))
    );

    let i = &b"G"[..];
    assert_parse!(
      hex_digit1(i),
      Err(Err::Error(error_position!(i, ErrorKind::HexDigit)))
    );

    assert!(crate::character::is_hex_digit(b'0'));
    assert!(crate::character::is_hex_digit(b'9'));
    assert!(crate::character::is_hex_digit(b'a'));
    assert!(crate::character::is_hex_digit(b'f'));
    assert!(crate::character::is_hex_digit(b'A'));
    assert!(crate::character::is_hex_digit(b'F'));
    assert!(!crate::character::is_hex_digit(b'g'));
    assert!(!crate::character::is_hex_digit(b'G'));
    assert!(!crate::character::is_hex_digit(b'/'));
    assert!(!crate::character::is_hex_digit(b':'));
    assert!(!crate::character::is_hex_digit(b'@'));
    assert!(!crate::character::is_hex_digit(b'\x60'));
  }

  #[test]
  fn oct_digit_test() {
    let i = &b"01234567;"[..];
    assert_parse!(oct_digit1(i), Ok((&b";"[..], &i[..i.len() - 1])));

    let i = &b"8"[..];
    assert_parse!(
      oct_digit1(i),
      Err(Err::Error(error_position!(i, ErrorKind::OctDigit)))
    );

    assert!(crate::character::is_oct_digit(b'0'));
    assert!(crate::character::is_oct_digit(b'7'));
    assert!(!crate::character::is_oct_digit(b'8'));
    assert!(!crate::character::is_oct_digit(b'9'));
    assert!(!crate::character::is_oct_digit(b'a'));
    assert!(!crate::character::is_oct_digit(b'A'));
    assert!(!crate::character::is_oct_digit(b'/'));
    assert!(!crate::character::is_oct_digit(b':'));
    assert!(!crate::character::is_oct_digit(b'@'));
    assert!(!crate::character::is_oct_digit(b'\x60'));
  }

  #[test]
  fn full_line_windows() {
    fn take_full_line(i: &[u8]) -> IResult<&[u8], (&[u8], &[u8])> {
      pair(not_line_ending, line_ending)(i)
    }
    let input = b"abc\r\n";
    let output = take_full_line(input);
    assert_eq!(output, Ok((&b""[..], (&b"abc"[..], &b"\r\n"[..]))));
  }

  #[test]
  fn full_line_unix() {
    fn take_full_line(i: &[u8]) -> IResult<&[u8], (&[u8], &[u8])> {
      pair(not_line_ending, line_ending)(i)
    }
    let input = b"abc\n";
    let output = take_full_line(input);
    assert_eq!(output, Ok((&b""[..], (&b"abc"[..], &b"\n"[..]))));
  }

  #[test]
  fn check_windows_lineending() {
    let input = b"\r\n";
    let output = line_ending(&input[..]);
    assert_parse!(output, Ok((&b""[..], &b"\r\n"[..])));
  }

  #[test]
  fn check_unix_lineending() {
    let input = b"\n";
    let output = line_ending(&input[..]);
    assert_parse!(output, Ok((&b""[..], &b"\n"[..])));
  }

  #[test]
  fn cr_lf() {
    assert_parse!(crlf(&b"\r\na"[..]), Ok((&b"a"[..], &b"\r\n"[..])));
    assert_parse!(crlf(&b"\r"[..]), Err(Err::Incomplete(Needed::new(2))));
    assert_parse!(
      crlf(&b"\ra"[..]),
      Err(Err::Error(error_position!(&b"\ra"[..], ErrorKind::CrLf)))
    );

    assert_parse!(crlf("\r\na"), Ok(("a", "\r\n")));
    assert_parse!(crlf("\r"), Err(Err::Incomplete(Needed::new(2))));
    assert_parse!(
      crlf("\ra"),
      Err(Err::Error(error_position!("\ra", ErrorKind::CrLf)))
    );
  }

  #[test]
  fn end_of_line() {
    assert_parse!(line_ending(&b"\na"[..]), Ok((&b"a"[..], &b"\n"[..])));
    assert_parse!(line_ending(&b"\r\na"[..]), Ok((&b"a"[..], &b"\r\n"[..])));
    assert_parse!(
      line_ending(&b"\r"[..]),
      Err(Err::Incomplete(Needed::new(2)))
    );
    assert_parse!(
      line_ending(&b"\ra"[..]),
      Err(Err::Error(error_position!(&b"\ra"[..], ErrorKind::CrLf)))
    );

    assert_parse!(line_ending("\na"), Ok(("a", "\n")));
    assert_parse!(line_ending("\r\na"), Ok(("a", "\r\n")));
    assert_parse!(line_ending("\r"), Err(Err::Incomplete(Needed::new(2))));
    assert_parse!(
      line_ending("\ra"),
      Err(Err::Error(error_position!("\ra", ErrorKind::CrLf)))
    );
  }

  fn digit_to_i16(input: &str) -> IResult<&str, i16> {
    let i = input;
    let (i, opt_sign) = opt(alt((char('+'), char('-'))))(i)?;
    let sign = match opt_sign {
      Some('+') => true,
      Some('-') => false,
      _ => true,
    };

    let (i, s) = match digit1::<_, crate::error::Error<_>>(i) {
      Ok((i, s)) => (i, s),
      Err(Err::Incomplete(i)) => return Err(Err::Incomplete(i)),
      Err(_) => {
        return Err(Err::Error(crate::error::Error::from_error_kind(
          input,
          ErrorKind::Digit,
        )))
      }
    };
    match s.parse_to() {
      Some(n) => {
        if sign {
          Ok((i, n))
        } else {
          Ok((i, -n))
        }
      }
      None => Err(Err::Error(crate::error::Error::from_error_kind(
        i,
        ErrorKind::Digit,
      ))),
    }
  }

  fn digit_to_u32(i: &str) -> IResult<&str, u32> {
    let (i, s) = digit1(i)?;
    match s.parse_to() {
      Some(n) => Ok((i, n)),
      None => Err(Err::Error(crate::error::Error::from_error_kind(
        i,
        ErrorKind::Digit,
      ))),
    }
  }

  proptest! {
    #[test]
    fn ints(s in "\\PC*") {
        let res1 = digit_to_i16(&s);
        let res2 = i16(s.as_str());
        assert_eq!(res1, res2);
    }

    #[test]
    fn uints(s in "\\PC*") {
        let res1 = digit_to_u32(&s);
        let res2 = u32(s.as_str());
        assert_eq!(res1, res2);
    }
  }
}

#[cfg(test)]
mod tests_rug_378 {
    use super::*;
    use crate::{
        error::ErrorKind,
        error::ParseError,
        character::streaming::{Input, one_of},
        Needed,
        Err,
        IResult
    };

    
    #[test]
    fn test_rug() {
        let mut p0: &str = "abc";

        
        let result: IResult<&str, char, (_, ErrorKind)> = one_of::<_, _, (_, ErrorKind)>(p0)("");

    }
}
    #[cfg(test)]
mod tests_rug_379 {
    use super::*;
    use crate::{Err, error::ErrorKind, Needed};
    use crate::character::streaming::{AsChar, Input, none_of};

    struct TokenList<T: PartialEq> {
        tokens: Vec<T>,
    }

    impl<T: PartialEq> FindToken<T> for TokenList<T> {
        fn find_token(&self, token: T) -> bool {
            self.tokens.contains(&token)
        }
    }

    #[test]
    fn test_none_of() {
        let list = TokenList { tokens: vec!['a', 'b', 'c'] };
        let none_of_func = none_of::<_, _, (_, ErrorKind)>(list);
        
        assert_eq!(none_of_func("z"), Ok(("", 'z')));
        assert_eq!(none_of_func("a"), Err(Err::Error(("a", ErrorKind::NoneOf))));
        assert_eq!(none_of_func(""), Err(Err::Incomplete(Needed::new(1))));
    }
} 
 #[cfg(test)]
    mod tests_rug_380 {
        use super::*;
        use crate::{
            error::{ErrorKind, ParseError},
            sequence::terminated,
            FindSubstring,
        };

        #[test]
        fn test_rug() {
            let mut p0: &str = "ab\r\nc";
            let o: IResult<&str, &str, (&str, ErrorKind)> = terminated(crate::character::streaming::crlf::<&str, (_, ErrorKind)>, crate::character::streaming::crlf::<&str, (_, ErrorKind)>)(p0);

            let mut p0: &str = "";
            let o: IResult<&str, &str, (&str, ErrorKind)> = terminated(crate::character::streaming::crlf::<&str, (_, ErrorKind)>, crate::character::streaming::crlf::<&str, (_, ErrorKind)>)(p0);

           
        }
    }
                #[cfg(test)]
mod tests_rug_381 {
    use super::*;
    use crate::{
        Err,
        error::{Error, ErrorKind},
        IResult, Needed,
        character::streaming::CompareResult
    };

    #[test]
    fn test_not_line_ending() {
        let mut p0: &str = "ab\r\nc";
        
        assert_eq!(not_line_ending::<_, (_, ErrorKind)>(p0), Ok(("\r\nc", "ab")));
        
        p0 = "abc";
        assert_eq!(not_line_ending::<_, (_, ErrorKind)>(p0), Err(Err::Incomplete(Needed::Unknown)));
        
        p0 = "";
        assert_eq!(not_line_ending::<_, (_, ErrorKind)>(p0), Err(Err::Incomplete(Needed::Unknown)));
        
        p0 = "a\rb\nc";
        assert_eq!(
            not_line_ending::<_, (_, ErrorKind)>(p0),
            Err(Err::Error(("a\rb\nc", ErrorKind::Tag)))
        );
        
        p0 = "a\rbc";
        assert_eq!(
            not_line_ending::<_, (_, ErrorKind)>(p0),
            Err(Err::Error(("a\rbc", ErrorKind::Tag)))
        );
    }
}

#[cfg(test)]
mod tests_rug_382 {
    use super::*;
    use crate::character::streaming::line_ending;
    use crate::{Err, IResult, error::ErrorKind, Needed};
    
    #[test]
    fn test_line_ending() {
        let p0 = "\r\nc";

        assert_eq!(line_ending::<_, (_, ErrorKind)>(p0), Ok(("c", "\r\n")));

        let p1 = "ab\r\nc";
        assert_eq!(line_ending::<_, (_, ErrorKind)>(p1), Err(Err::Error(("ab\r\nc", ErrorKind::CrLf))));

        let p2 = "";
        assert_eq!(line_ending::<_, (_, ErrorKind)>(p2), Err(Err::Incomplete(Needed::new(1))));
    
    }

}
#[cfg(test)]
mod tests_rug_383 {
    use super::*;
    use crate::error::ErrorKind;
    use crate::error::ParseError;
    use crate::Err;
    use crate::IResult;
    use crate::Needed;
    use crate::character::streaming::newline;

    #[test]
    fn test_newline() {
        let mut p0: &str = "\nc";

        assert_eq!(newline::<_, (_, ErrorKind)>(p0), Ok(("c", '\n')));

        let mut p1: &str = "\r\nc";
        assert_eq!(newline::<_, (_, ErrorKind)>(p1), Err(Err::Error(("\r\nc", ErrorKind::Char))));

        let mut p2: &str = "";
        assert_eq!(newline::<_, (_, ErrorKind)>(p2), Err(Err::Incomplete(Needed::new(1))));
        
    }
}


#[cfg(test)]
mod tests_rug_384 {
    use super::*;
    use crate::{
        error::ErrorKind,
        character::streaming::tab,
        IResult, 
        Err,
        error::ParseError, 
        Needed
    };
    
    #[test]
    fn test_tab() {
        let p0: &str = "\tc";
       
       assert_eq!(tab::<_, (_, ErrorKind)>(p0), Ok(("c", '\t')));
       assert_eq!(tab::<_, (_, ErrorKind)>("\r\nc"), Err(Err::Error(("\r\nc", ErrorKind::Char))));
       assert_eq!(tab::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
    }

}

#[cfg(test)]
mod tests_rug_385 {
    use super::*;
    use crate::{
        error::{ErrorKind, ParseError},
        Err, IResult, Needed,
    };

    #[test]
    fn test_anychar() {
        let mut p0: &str = "abc";
        assert_eq!(
            anychar::<_, (_, ErrorKind)>(p0),
            Ok(("bc", 'a'))
        );

        p0 = "";
        assert_eq!(
            anychar::<_, (_, ErrorKind)>(p0),
            Err(Err::Incomplete(Needed::new(1)))
        );
    }
}   
#[cfg(test)]
mod tests_rug_387 {
    use super::*;
    use crate::{
        character::streaming::alpha1,
        error::{ErrorKind, ParseError},
        Err, IResult, Needed,
    };

    #[test]
    fn test_alpha1() {
        // Test case 1
        let input1 = "aB1c";
        let expected_output1 = Ok(("1c", "aB"));
        let result1: IResult<_, _, (_, ErrorKind)> = alpha1::<_, (_, ErrorKind)>(input1);
        assert_eq!(result1, expected_output1);

        // Test case 2
        let input2 = "1c";
        let expected_output2 = Err(Err::Error(("1c", ErrorKind::Alpha)));
        let result2: IResult<_, _, (_, ErrorKind)> = alpha1::<_, (_, ErrorKind)>(input2);
        assert_eq!(result2, expected_output2);

        // Test case 3
        let input3 = "";
        let expected_output3 = Err(Err::Incomplete(Needed::new(1)));
        let result3: IResult<_, _, (_, ErrorKind)> = alpha1::<_, (_, ErrorKind)>(input3);
        assert_eq!(result3, expected_output3);
    }
}

#[cfg(test)]
mod tests_rug_388 {
    use super::*;
    use crate::{
        error::{ErrorKind, ParseError},
        Err, IResult, Needed,
    };
    
    #[test]
    fn test_digit0() {
        let input = "21c";
        assert_eq!(digit0::<_, (_, ErrorKind)>(input), Ok(("c", "21")));

        let input = "a21c";
        assert_eq!(digit0::<_, (_, ErrorKind)>(input), Ok(("a21c", "")));

        let input = "";
        assert_eq!(digit0::<_, (_, ErrorKind)>(input), Err(Err::Incomplete(Needed::new(1))));
    }
}
#[cfg(test)]
mod tests_rug_389 {
    use super::*;
    use crate::{Err, error::ErrorKind, IResult, Needed};
    use crate::character::streaming::digit1;

    #[test]
    fn test_digit1() {
        assert_eq!(digit1::<_, (_, ErrorKind)>("21c"), Ok(("c", "21")));
        assert_eq!(digit1::<_, (_, ErrorKind)>("c1"), Err(Err::Error(("c1", ErrorKind::Digit))));
        assert_eq!(digit1::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
    }
}#[cfg(test)]
mod tests_rug_390 {
    use super::*;
    use crate::character::streaming::{Input, ParseError};
    use crate::{Err, error::ErrorKind, IResult, Needed};
  
    #[test]
    fn test_hex_digit0() {
        let input: &str = "21cZ";
        let expected: IResult<_, _, (_, ErrorKind)> = Ok(("Z", "21c"));

        assert_eq!(hex_digit0::<_, (_, ErrorKind)>(input), expected);
    }
}#[cfg(test)]
mod tests_rug_391 {
    use super::*;
    use crate::{
        Err, error::ErrorKind, IResult, Needed,
        character::streaming::{hex_digit1}
    };

    #[test]
    fn test_hex_digit1() {
        assert_eq!(hex_digit1::<_, (_, ErrorKind)>("21cZ"), Ok(("Z", "21c")));
        assert_eq!(hex_digit1::<_, (_, ErrorKind)>("H2"), Err(Err::Error(("H2", ErrorKind::HexDigit))));
        assert_eq!(hex_digit1::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
    }
}#[cfg(test)]
mod tests_rug_392 {
    use super::*;
    use crate::error::ErrorKind;
    use crate::Err;
    use crate::Needed;
    use crate::character::streaming::oct_digit0;

    #[test]
    fn test_oct_digit0() {
        let mut p0: &str = "21cZ";

        assert_eq!(oct_digit0::<_, (_, ErrorKind)>(p0), Ok(("cZ", "21")));

        p0 = "Z21c";
        assert_eq!(oct_digit0::<_, (_, ErrorKind)>(p0), Ok(("Z21c", "")));

        p0 = "";
        assert_eq!(oct_digit0::<_, (_, ErrorKind)>(p0), Err(Err::Incomplete(Needed::new(1))));
    }
}
#[cfg(test)]
mod tests_rug_394 {
    use super::*;
    use crate::error::ErrorKind;
    use crate::{Err, IResult, Needed};
    use crate::character::streaming::alphanumeric0;

    #[test]
    fn test_alphanumeric0() {
        assert_eq!(alphanumeric0::<_, (_, ErrorKind)>("21cZ%1"), Ok(("%1", "21cZ")));
        assert_eq!(alphanumeric0::<_, (_, ErrorKind)>("&Z21c"), Ok(("&Z21c", "")));
        assert_eq!(alphanumeric0::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
    }
}
#[cfg(test)]
mod tests_rug_395 {
    use super::*;
    use crate::{
        Err,
        error::ErrorKind,
        IResult,
        Needed,
        character::streaming::alphanumeric1,
    };
    
    #[test]
    fn test_alphanumeric1() {
        let p0: &str = "21cZ%1";
        assert_eq!(alphanumeric1::<_, (_, ErrorKind)>(p0), Ok(("%1", "21cZ")));
        
        let p1: &str = "&H2";
        assert_eq!(alphanumeric1::<_, (_, ErrorKind)>(p1), Err(Err::Error((p1, ErrorKind::AlphaNumeric))));
        
        let p2: &str = "";
        assert_eq!(alphanumeric1::<_, (_, ErrorKind)>(p2), Err(Err::Incomplete(Needed::new(1))));
    }
}#[cfg(test)]
mod tests_rug_396 {
    use super::*;
    use crate::{Err, error::ErrorKind, IResult, Needed};
    use crate::character::streaming::space0;

    #[test]
    fn test_space0() {
        let mut p0: &str = " \t21c";
        assert_eq!(space0::<_, (_, ErrorKind)>(p0), Ok(("21c", " \t")));

        let mut p1: &str = "Z21c";
        assert_eq!(space0::<_, (_, ErrorKind)>(p1), Ok(("Z21c", "")));

        let mut p2: &str = "";
        assert_eq!(space0::<_, (_, ErrorKind)>(p2), Err(Err::Incomplete(Needed::new(1))));
    }
}#[cfg(test)]
mod tests_rug_397 {
    use super::*;
    use crate::{Err, error::ErrorKind, IResult, Needed};
    use crate::character::streaming::space1;
    
    #[test]
    fn test_space1() {
        let p0: &str = " \t21c";
        assert_eq!(space1::<_, (_, ErrorKind)>(p0), Ok(("21c", " \t")));
        
        let p1: &str = "H2";
        assert_eq!(space1::<_, (_, ErrorKind)>(p1), Err(Err::Error(("H2", ErrorKind::Space))));
        
        let p2: &str = "";
        assert_eq!(space1::<_, (_, ErrorKind)>(p2), Err(Err::Incomplete(Needed::new(1))));
    }
}
#[cfg(test)]
mod tests_rug_398 {
    use super::*;
    use crate::{
        Err,
        error::ErrorKind,
        IResult,
        Needed
    };
    use crate::character::streaming::multispace0;
    
    #[test]
    fn test_multispace0() {
        let p0: &str = " \t\n\r21c";
        
        assert_eq!(multispace0::<_, (_, ErrorKind)>(p0), Ok(("21c", " \t\n\r")));
    }
}
#[cfg(test)]
mod tests_rug_399 {
    use super::*;
    use crate::{Input,Needed};
    use crate::{Err, IResult, error::ErrorKind, character::streaming::multispace1};
    
    #[test]
    fn test_multispace1() {
        let p0: &str = " \t\n\r21c";
      
        assert_eq!(multispace1::<_, (_, ErrorKind)>(p0), Ok(("21c", " \t\n\r")));
        let p1: &str = "H2";
        assert_eq!(multispace1::<_, (_, ErrorKind)>(p1), Err(Err::Error(("H2", ErrorKind::MultiSpace))));
        let p2: &str = "";
        assert_eq!(multispace1::<_, (_, ErrorKind)>(p2), Err(Err::Incomplete(Needed::new(1))));
    }
}#[cfg(test)]
mod tests_rug_404 {
    use super::*;
    use crate::error::{ParseError, ErrorKind};
    use crate::{Err, IResult, Needed};
    use crate::character::complete::one_of;
    use crate::traits::{Compare, CompareResult};
    
    #[test]
    fn test_i64() {
        let p0: &str = "123";
        
        let result: IResult<_, _, ()> = crate::character::streaming::i64(p0);
        
        assert_eq!(result.unwrap(), ("", 123));
    }
}