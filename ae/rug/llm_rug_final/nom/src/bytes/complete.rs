//! Parsers recognizing bytes streams, complete input version

use crate::error::ErrorKind;
use crate::error::ParseError;
use crate::internal::{Err, IResult, Parser};
use crate::lib::std::result::Result::*;
use crate::traits::{Compare, CompareResult, FindSubstring, FindToken, InputLength, ToUsize};
use crate::Input;

/// Recognizes a pattern
///
/// The input data will be compared to the tag combinator's argument and will return the part of
/// the input that matches the argument
///
/// It will return `Err(Err::Error((_, ErrorKind::Tag)))` if the input doesn't match the pattern
/// # Example
/// ```rust
/// # use nom::{Err, error::{Error, ErrorKind}, Needed, IResult};
/// use nom::bytes::complete::tag;
///
/// fn parser(s: &str) -> IResult<&str, &str> {
///   tag("Hello")(s)
/// }
///
/// assert_eq!(parser("Hello, World!"), Ok((", World!", "Hello")));
/// assert_eq!(parser("Something"), Err(Err::Error(Error::new("Something", ErrorKind::Tag))));
/// assert_eq!(parser(""), Err(Err::Error(Error::new("", ErrorKind::Tag))));
/// ```
pub fn tag<T, I, Error: ParseError<I>>(tag: T) -> impl Fn(I) -> IResult<I, I, Error>
where
  I: Input + Compare<T>,
  T: InputLength + Clone,
{
  move |i: I| {
    let tag_len = tag.input_len();
    let t = tag.clone();
    let res: IResult<_, _, Error> = match i.compare(t) {
      CompareResult::Ok => Ok(i.take_split(tag_len)),
      _ => {
        let e: ErrorKind = ErrorKind::Tag;
        Err(Err::Error(Error::from_error_kind(i, e)))
      }
    };
    res
  }
}

/// Recognizes a case insensitive pattern.
///
/// The input data will be compared to the tag combinator's argument and will return the part of
/// the input that matches the argument with no regard to case.
///
/// It will return `Err(Err::Error((_, ErrorKind::Tag)))` if the input doesn't match the pattern.
/// # Example
/// ```rust
/// # use nom::{Err, error::{Error, ErrorKind}, Needed, IResult};
/// use nom::bytes::complete::tag_no_case;
///
/// fn parser(s: &str) -> IResult<&str, &str> {
///   tag_no_case("hello")(s)
/// }
///
/// assert_eq!(parser("Hello, World!"), Ok((", World!", "Hello")));
/// assert_eq!(parser("hello, World!"), Ok((", World!", "hello")));
/// assert_eq!(parser("HeLlO, World!"), Ok((", World!", "HeLlO")));
/// assert_eq!(parser("Something"), Err(Err::Error(Error::new("Something", ErrorKind::Tag))));
/// assert_eq!(parser(""), Err(Err::Error(Error::new("", ErrorKind::Tag))));
/// ```
pub fn tag_no_case<T, I, Error: ParseError<I>>(tag: T) -> impl Fn(I) -> IResult<I, I, Error>
where
  I: Input + Compare<T>,
  T: InputLength + Clone,
{
  move |i: I| {
    let tag_len = tag.input_len();
    let t = tag.clone();

    let res: IResult<_, _, Error> = match (i).compare_no_case(t) {
      CompareResult::Ok => Ok(i.take_split(tag_len)),
      _ => {
        let e: ErrorKind = ErrorKind::Tag;
        Err(Err::Error(Error::from_error_kind(i, e)))
      }
    };
    res
  }
}

/// Parse till certain characters are met.
///
/// The parser will return the longest slice till one of the characters of the combinator's argument are met.
///
/// It doesn't consume the matched character.
///
/// It will return a `Err::Error(("", ErrorKind::IsNot))` if the pattern wasn't met.
/// # Example
/// ```rust
/// # use nom::{Err, error::{Error, ErrorKind}, Needed, IResult};
/// use nom::bytes::complete::is_not;
///
/// fn not_space(s: &str) -> IResult<&str, &str> {
///   is_not(" \t\r\n")(s)
/// }
///
/// assert_eq!(not_space("Hello, World!"), Ok((" World!", "Hello,")));
/// assert_eq!(not_space("Sometimes\t"), Ok(("\t", "Sometimes")));
/// assert_eq!(not_space("Nospace"), Ok(("", "Nospace")));
/// assert_eq!(not_space(""), Err(Err::Error(Error::new("", ErrorKind::IsNot))));
/// ```
pub fn is_not<T, I, Error: ParseError<I>>(arr: T) -> impl Fn(I) -> IResult<I, I, Error>
where
  I: Input,
  T: FindToken<<I as Input>::Item>,
{
  move |i: I| {
    let e: ErrorKind = ErrorKind::IsNot;
    i.split_at_position1_complete(|c| arr.find_token(c), e)
  }
}

/// Returns the longest slice of the matches the pattern.
///
/// The parser will return the longest slice consisting of the characters in provided in the
/// combinator's argument.
///
/// It will return a `Err(Err::Error((_, ErrorKind::IsA)))` if the pattern wasn't met.
/// # Example
/// ```rust
/// # use nom::{Err, error::{Error, ErrorKind}, Needed, IResult};
/// use nom::bytes::complete::is_a;
///
/// fn hex(s: &str) -> IResult<&str, &str> {
///   is_a("1234567890ABCDEF")(s)
/// }
///
/// assert_eq!(hex("123 and voila"), Ok((" and voila", "123")));
/// assert_eq!(hex("DEADBEEF and others"), Ok((" and others", "DEADBEEF")));
/// assert_eq!(hex("BADBABEsomething"), Ok(("something", "BADBABE")));
/// assert_eq!(hex("D15EA5E"), Ok(("", "D15EA5E")));
/// assert_eq!(hex(""), Err(Err::Error(Error::new("", ErrorKind::IsA))));
/// ```
pub fn is_a<T, I, Error: ParseError<I>>(arr: T) -> impl Fn(I) -> IResult<I, I, Error>
where
  I: Input,
  T: FindToken<<I as Input>::Item>,
{
  move |i: I| {
    let e: ErrorKind = ErrorKind::IsA;
    i.split_at_position1_complete(|c| !arr.find_token(c), e)
  }
}

/// Returns the longest input slice (if any) that matches the predicate.
///
/// The parser will return the longest slice that matches the given predicate *(a function that
/// takes the input and returns a bool)*.
/// # Example
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed, IResult};
/// use nom::bytes::complete::take_while;
/// use nom::character::is_alphabetic;
///
/// fn alpha(s: &[u8]) -> IResult<&[u8], &[u8]> {
///   take_while(is_alphabetic)(s)
/// }
///
/// assert_eq!(alpha(b"latin123"), Ok((&b"123"[..], &b"latin"[..])));
/// assert_eq!(alpha(b"12345"), Ok((&b"12345"[..], &b""[..])));
/// assert_eq!(alpha(b"latin"), Ok((&b""[..], &b"latin"[..])));
/// assert_eq!(alpha(b""), Ok((&b""[..], &b""[..])));
/// ```
pub fn take_while<F, I, Error: ParseError<I>>(cond: F) -> impl Fn(I) -> IResult<I, I, Error>
where
  I: Input,
  F: Fn(<I as Input>::Item) -> bool,
{
  move |i: I| i.split_at_position_complete(|c| !cond(c))
}

/// Returns the longest (at least 1) input slice that matches the predicate.
///
/// The parser will return the longest slice that matches the given predicate *(a function that
/// takes the input and returns a bool)*.
///
/// It will return an `Err(Err::Error((_, ErrorKind::TakeWhile1)))` if the pattern wasn't met.
/// # Example
/// ```rust
/// # use nom::{Err, error::{Error, ErrorKind}, Needed, IResult};
/// use nom::bytes::complete::take_while1;
/// use nom::character::is_alphabetic;
///
/// fn alpha(s: &[u8]) -> IResult<&[u8], &[u8]> {
///   take_while1(is_alphabetic)(s)
/// }
///
/// assert_eq!(alpha(b"latin123"), Ok((&b"123"[..], &b"latin"[..])));
/// assert_eq!(alpha(b"latin"), Ok((&b""[..], &b"latin"[..])));
/// assert_eq!(alpha(b"12345"), Err(Err::Error(Error::new(&b"12345"[..], ErrorKind::TakeWhile1))));
/// ```
pub fn take_while1<F, I, Error: ParseError<I>>(cond: F) -> impl Fn(I) -> IResult<I, I, Error>
where
  I: Input,
  F: Fn(<I as Input>::Item) -> bool,
{
  move |i: I| {
    let e: ErrorKind = ErrorKind::TakeWhile1;
    i.split_at_position1_complete(|c| !cond(c), e)
  }
}

/// Returns the longest (m <= len <= n) input slice that matches the predicate.
///
/// The parser will return the longest slice that matches the given predicate *(a function that
/// takes the input and returns a bool)*.
///
/// It will return an `Err::Error((_, ErrorKind::TakeWhileMN))` if the pattern wasn't met or is out
/// of range (m <= len <= n).
/// # Example
/// ```rust
/// # use nom::{Err, error::{Error, ErrorKind}, Needed, IResult};
/// use nom::bytes::complete::take_while_m_n;
/// use nom::character::is_alphabetic;
///
/// fn short_alpha(s: &[u8]) -> IResult<&[u8], &[u8]> {
///   take_while_m_n(3, 6, is_alphabetic)(s)
/// }
///
/// assert_eq!(short_alpha(b"latin123"), Ok((&b"123"[..], &b"latin"[..])));
/// assert_eq!(short_alpha(b"lengthy"), Ok((&b"y"[..], &b"length"[..])));
/// assert_eq!(short_alpha(b"latin"), Ok((&b""[..], &b"latin"[..])));
/// assert_eq!(short_alpha(b"ed"), Err(Err::Error(Error::new(&b"ed"[..], ErrorKind::TakeWhileMN))));
/// assert_eq!(short_alpha(b"12345"), Err(Err::Error(Error::new(&b"12345"[..], ErrorKind::TakeWhileMN))));
/// ```
pub fn take_while_m_n<F, I, Error: ParseError<I>>(
  m: usize,
  n: usize,
  cond: F,
) -> impl Fn(I) -> IResult<I, I, Error>
where
  I: Input,
  F: Fn(<I as Input>::Item) -> bool,
{
  move |i: I| {
    let input = i;
    let mut count = 0;
    for (i, (index, item)) in input.iter_indices().enumerate() {
      if i == n {
        return Ok(input.take_split(index));
      }

      if !cond(item) {
        if i >= m {
          return Ok(input.take_split(index));
        } else {
          return Err(Err::Error(Error::from_error_kind(
            input,
            ErrorKind::TakeWhileMN,
          )));
        }
      }

      count += 1;
    }

    if count >= m {
      Ok(input.take_split(input.input_len()))
    } else {
      Err(Err::Error(Error::from_error_kind(
        input,
        ErrorKind::TakeWhileMN,
      )))
    }
  }
}

/// Returns the longest input slice (if any) till a predicate is met.
///
/// The parser will return the longest slice till the given predicate *(a function that
/// takes the input and returns a bool)*.
/// # Example
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed, IResult};
/// use nom::bytes::complete::take_till;
///
/// fn till_colon(s: &str) -> IResult<&str, &str> {
///   take_till(|c| c == ':')(s)
/// }
///
/// assert_eq!(till_colon("latin:123"), Ok((":123", "latin")));
/// assert_eq!(till_colon(":empty matched"), Ok((":empty matched", ""))); //allowed
/// assert_eq!(till_colon("12345"), Ok(("", "12345")));
/// assert_eq!(till_colon(""), Ok(("", "")));
/// ```
#[allow(clippy::redundant_closure)]
pub fn take_till<F, I, Error: ParseError<I>>(cond: F) -> impl Fn(I) -> IResult<I, I, Error>
where
  I: Input,
  F: Fn(<I as Input>::Item) -> bool,
{
  move |i: I| i.split_at_position_complete(|c| cond(c))
}

/// Returns the longest (at least 1) input slice till a predicate is met.
///
/// The parser will return the longest slice till the given predicate *(a function that
/// takes the input and returns a bool)*.
///
/// It will return `Err(Err::Error((_, ErrorKind::TakeTill1)))` if the input is empty or the
/// predicate matches the first input.
/// # Example
/// ```rust
/// # use nom::{Err, error::{Error, ErrorKind}, Needed, IResult};
/// use nom::bytes::complete::take_till1;
///
/// fn till_colon(s: &str) -> IResult<&str, &str> {
///   take_till1(|c| c == ':')(s)
/// }
///
/// assert_eq!(till_colon("latin:123"), Ok((":123", "latin")));
/// assert_eq!(till_colon(":empty matched"), Err(Err::Error(Error::new(":empty matched", ErrorKind::TakeTill1))));
/// assert_eq!(till_colon("12345"), Ok(("", "12345")));
/// assert_eq!(till_colon(""), Err(Err::Error(Error::new("", ErrorKind::TakeTill1))));
/// ```
#[allow(clippy::redundant_closure)]
pub fn take_till1<F, I, Error: ParseError<I>>(cond: F) -> impl Fn(I) -> IResult<I, I, Error>
where
  I: Input,
  F: Fn(<I as Input>::Item) -> bool,
{
  move |i: I| {
    let e: ErrorKind = ErrorKind::TakeTill1;
    i.split_at_position1_complete(|c| cond(c), e)
  }
}

/// Returns an input slice containing the first N input elements (Input[..N]).
///
/// It will return `Err(Err::Error((_, ErrorKind::Eof)))` if the input is shorter than the argument.
/// # Example
/// ```rust
/// # use nom::{Err, error::{Error, ErrorKind}, Needed, IResult};
/// use nom::bytes::complete::take;
///
/// fn take6(s: &str) -> IResult<&str, &str> {
///   take(6usize)(s)
/// }
///
/// assert_eq!(take6("1234567"), Ok(("7", "123456")));
/// assert_eq!(take6("things"), Ok(("", "things")));
/// assert_eq!(take6("short"), Err(Err::Error(Error::new("short", ErrorKind::Eof))));
/// assert_eq!(take6(""), Err(Err::Error(Error::new("", ErrorKind::Eof))));
/// ```
///
/// The units that are taken will depend on the input type. For example, for a
/// `&str` it will take a number of `char`'s, whereas for a `&[u8]` it will
/// take that many `u8`'s:
///
/// ```rust
/// use nom::error::Error;
/// use nom::bytes::complete::take;
///
/// assert_eq!(take::<_, _, Error<_>>(1usize)("💙"), Ok(("", "💙")));
/// assert_eq!(take::<_, _, Error<_>>(1usize)("💙".as_bytes()), Ok((b"\x9F\x92\x99".as_ref(), b"\xF0".as_ref())));
/// ```
pub fn take<C, I, Error: ParseError<I>>(count: C) -> impl Fn(I) -> IResult<I, I, Error>
where
  I: Input,
  C: ToUsize,
{
  let c = count.to_usize();
  move |i: I| match i.slice_index(c) {
    Err(_needed) => Err(Err::Error(Error::from_error_kind(i, ErrorKind::Eof))),
    Ok(index) => Ok(i.take_split(index)),
  }
}

/// Returns the input slice up to the first occurrence of the pattern.
///
/// It doesn't consume the pattern. It will return `Err(Err::Error((_, ErrorKind::TakeUntil)))`
/// if the pattern wasn't met.
/// # Example
/// ```rust
/// # use nom::{Err, error::{Error, ErrorKind}, Needed, IResult};
/// use nom::bytes::complete::take_until;
///
/// fn until_eof(s: &str) -> IResult<&str, &str> {
///   take_until("eof")(s)
/// }
///
/// assert_eq!(until_eof("hello, worldeof"), Ok(("eof", "hello, world")));
/// assert_eq!(until_eof("hello, world"), Err(Err::Error(Error::new("hello, world", ErrorKind::TakeUntil))));
/// assert_eq!(until_eof(""), Err(Err::Error(Error::new("", ErrorKind::TakeUntil))));
/// assert_eq!(until_eof("1eof2eof"), Ok(("eof2eof", "1")));
/// ```
pub fn take_until<T, I, Error: ParseError<I>>(tag: T) -> impl Fn(I) -> IResult<I, I, Error>
where
  I: Input + FindSubstring<T>,
  T: InputLength + Clone,
{
  move |i: I| {
    let t = tag.clone();
    let res: IResult<_, _, Error> = match i.find_substring(t) {
      None => Err(Err::Error(Error::from_error_kind(i, ErrorKind::TakeUntil))),
      Some(index) => Ok(i.take_split(index)),
    };
    res
  }
}

/// Returns the non empty input slice up to the first occurrence of the pattern.
///
/// It doesn't consume the pattern. It will return `Err(Err::Error((_, ErrorKind::TakeUntil)))`
/// if the pattern wasn't met.
/// # Example
/// ```rust
/// # use nom::{Err, error::{Error, ErrorKind}, Needed, IResult};
/// use nom::bytes::complete::take_until1;
///
/// fn until_eof(s: &str) -> IResult<&str, &str> {
///   take_until1("eof")(s)
/// }
///
/// assert_eq!(until_eof("hello, worldeof"), Ok(("eof", "hello, world")));
/// assert_eq!(until_eof("hello, world"), Err(Err::Error(Error::new("hello, world", ErrorKind::TakeUntil))));
/// assert_eq!(until_eof(""), Err(Err::Error(Error::new("", ErrorKind::TakeUntil))));
/// assert_eq!(until_eof("1eof2eof"), Ok(("eof2eof", "1")));
/// assert_eq!(until_eof("eof"), Err(Err::Error(Error::new("eof", ErrorKind::TakeUntil))));
/// ```
pub fn take_until1<T, I, Error: ParseError<I>>(tag: T) -> impl Fn(I) -> IResult<I, I, Error>
where
  I: Input + FindSubstring<T>,
  T: InputLength + Clone,
{
  move |i: I| {
    let t = tag.clone();
    let res: IResult<_, _, Error> = match i.find_substring(t) {
      None => Err(Err::Error(Error::from_error_kind(i, ErrorKind::TakeUntil))),
      Some(0) => Err(Err::Error(Error::from_error_kind(i, ErrorKind::TakeUntil))),
      Some(index) => Ok(i.take_split(index)),
    };
    res
  }
}

/// Matches a byte string with escaped characters.
///
/// * The first argument matches the normal characters (it must not accept the control character)
/// * The second argument is the control character (like `\` in most languages)
/// * The third argument matches the escaped characters
/// # Example
/// ```
/// # use nom::{Err, error::ErrorKind, Needed, IResult};
/// # use nom::character::complete::digit1;
/// use nom::bytes::complete::escaped;
/// use nom::character::complete::one_of;
///
/// fn esc(s: &str) -> IResult<&str, &str> {
///   escaped(digit1, '\\', one_of(r#""n\"#))(s)
/// }
///
/// assert_eq!(esc("123;"), Ok((";", "123")));
/// assert_eq!(esc(r#"12\"34;"#), Ok((";", r#"12\"34"#)));
/// ```
///
pub fn escaped<'a, I: 'a, Error, F, G>(
  mut normal: F,
  control_char: char,
  mut escapable: G,
) -> impl FnMut(I) -> IResult<I, I, Error>
where
  I: Clone + crate::traits::Offset + Input,
  <I as Input>::Item: crate::traits::AsChar,
  F: Parser<I, Error = Error>,
  G: Parser<I, Error = Error>,
  Error: ParseError<I>,
{
  use crate::traits::AsChar;

  move |input: I| {
    let mut i = input.clone();

    while i.input_len() > 0 {
      let current_len = i.input_len();

      match normal.parse(i.clone()) {
        Ok((i2, _)) => {
          // return if we consumed everything or if the normal parser
          // does not consume anything
          if i2.input_len() == 0 {
            return Ok(input.take_split(input.input_len()));
          } else if i2.input_len() == current_len {
            let index = input.offset(&i2);
            return Ok(input.take_split(index));
          } else {
            i = i2;
          }
        }
        Err(Err::Error(_)) => {
          // unwrap() should be safe here since index < $i.input_len()
          if i.iter_elements().next().unwrap().as_char() == control_char {
            let next = control_char.len_utf8();
            if next >= i.input_len() {
              return Err(Err::Error(Error::from_error_kind(
                input,
                ErrorKind::Escaped,
              )));
            } else {
              match escapable.parse(i.take_from(next)) {
                Ok((i2, _)) => {
                  if i2.input_len() == 0 {
                    return Ok(input.take_split(input.input_len()));
                  } else {
                    i = i2;
                  }
                }
                Err(e) => return Err(e),
              }
            }
          } else {
            let index = input.offset(&i);
            if index == 0 {
              return Err(Err::Error(Error::from_error_kind(
                input,
                ErrorKind::Escaped,
              )));
            }
            return Ok(input.take_split(index));
          }
        }
        Err(e) => {
          return Err(e);
        }
      }
    }

    Ok(input.take_split(input.input_len()))
  }
}

/// Matches a byte string with escaped characters.
///
/// * The first argument matches the normal characters (it must not match the control character)
/// * The second argument is the control character (like `\` in most languages)
/// * The third argument matches the escaped characters and transforms them
///
/// As an example, the chain `abc\tdef` could be `abc    def` (it also consumes the control character)
///
/// ```
/// # use nom::{Err, error::ErrorKind, Needed, IResult};
/// # use std::str::from_utf8;
/// use nom::bytes::complete::{escaped_transform, tag};
/// use nom::character::complete::alpha1;
/// use nom::branch::alt;
/// use nom::combinator::value;
///
/// fn parser(input: &str) -> IResult<&str, String> {
///   escaped_transform(
///     alpha1,
///     '\\',
///     alt((
///       value("\\", tag("\\")),
///       value("\"", tag("\"")),
///       value("\n", tag("n")),
///     ))
///   )(input)
/// }
///
/// assert_eq!(parser("ab\\\"cd"), Ok(("", String::from("ab\"cd"))));
/// assert_eq!(parser("ab\\ncd"), Ok(("", String::from("ab\ncd"))));
/// ```
#[cfg(feature = "alloc")]
#[cfg_attr(feature = "docsrs", doc(cfg(feature = "alloc")))]
pub fn escaped_transform<I, Error, F, G, O1, O2, ExtendItem, Output>(
  mut normal: F,
  control_char: char,
  mut transform: G,
) -> impl FnMut(I) -> IResult<I, Output, Error>
where
  I: Clone + crate::traits::Offset + Input,
  I: crate::traits::ExtendInto<Item = ExtendItem, Extender = Output>,
  O1: crate::traits::ExtendInto<Item = ExtendItem, Extender = Output>,
  O2: crate::traits::ExtendInto<Item = ExtendItem, Extender = Output>,
  <I as Input>::Item: crate::traits::AsChar,
  F: Parser<I, Output = O1, Error = Error>,
  G: Parser<I, Output = O2, Error = Error>,
  Error: ParseError<I>,
{
  use crate::traits::AsChar;

  move |input: I| {
    let mut index = 0;
    let mut res = input.new_builder();

    let i = input.clone();

    while index < i.input_len() {
      let current_len = i.input_len();
      let remainder = i.take_from(index);
      match normal.parse(remainder.clone()) {
        Ok((i2, o)) => {
          o.extend_into(&mut res);
          if i2.input_len() == 0 {
            return Ok((i.take_from(i.input_len()), res));
          } else if i2.input_len() == current_len {
            return Ok((remainder, res));
          } else {
            index = input.offset(&i2);
          }
        }
        Err(Err::Error(_)) => {
          // unwrap() should be safe here since index < $i.input_len()
          if remainder.iter_elements().next().unwrap().as_char() == control_char {
            let next = index + control_char.len_utf8();
            let input_len = input.input_len();

            if next >= input_len {
              return Err(Err::Error(Error::from_error_kind(
                remainder,
                ErrorKind::EscapedTransform,
              )));
            } else {
              match transform.parse(i.take_from(next)) {
                Ok((i2, o)) => {
                  o.extend_into(&mut res);
                  if i2.input_len() == 0 {
                    return Ok((i.take_from(i.input_len()), res));
                  } else {
                    index = input.offset(&i2);
                  }
                }
                Err(e) => return Err(e),
              }
            }
          } else {
            if index == 0 {
              return Err(Err::Error(Error::from_error_kind(
                remainder,
                ErrorKind::EscapedTransform,
              )));
            }
            return Ok((remainder, res));
          }
        }
        Err(e) => return Err(e),
      }
    }
    Ok((input.take_from(index), res))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::AsChar;

  #[test]
  fn complete_take_while_m_n_utf8_all_matching() {
    let result: IResult<&str, &str> =
      super::take_while_m_n(1, 4, |c: char| c.is_alphabetic())("øn");
    assert_eq!(result, Ok(("", "øn")));
  }

  #[test]
  fn complete_take_while_m_n_utf8_all_matching_substring() {
    let result: IResult<&str, &str> =
      super::take_while_m_n(1, 1, |c: char| c.is_alphabetic())("øn");
    assert_eq!(result, Ok(("n", "ø")));
  }

  // issue #1336 "escaped hangs if normal parser accepts empty"
  fn escaped_string(input: &str) -> IResult<&str, &str> {
    use crate::character::complete::{alpha0, one_of};
    escaped(alpha0, '\\', one_of("n"))(input)
  }

  // issue #1336 "escaped hangs if normal parser accepts empty"
  #[test]
  fn escaped_hang() {
    escaped_string("7").unwrap();
    escaped_string("a7").unwrap();
  }

  // issue ##1118 escaped does not work with empty string
  fn unquote(input: &str) -> IResult<&str, &str> {
    use crate::bytes::complete::*;
    use crate::character::complete::*;
    use crate::combinator::opt;
    use crate::sequence::delimited;

    delimited(
      char('"'),
      escaped(opt(none_of(r#"\""#)), '\\', one_of(r#"\"rnt"#)),
      char('"'),
    )(input)
  }

  #[test]
  fn escaped_hang_1118() {
    assert_eq!(unquote(r#""""#), Ok(("", "")));
  }

  // issue #1630 take_while_m_n is invalid for multi-byte UTF-8 characters
  #[test]
  fn complete_take_while_m_n_multibyte() {
    use crate::error::Error;

    fn multi_byte_chars(s: &str, m: usize, n: usize) -> IResult<&str, &str> {
      take_while_m_n(m, n, |c: char| c.len() > 1)(s)
    }

    assert_eq!(
      multi_byte_chars("€ latin", 0, 64),
      Ok((&" latin"[..], &"€"[..]))
    );
    assert_eq!(
      multi_byte_chars("𝄠 latin", 0, 1),
      Ok((&" latin"[..], &"𝄠"[..]))
    );
    assert_eq!(
      multi_byte_chars("باب latin", 0, 64),
      Ok((&" latin"[..], &"باب"[..]))
    );
    assert_eq!(
      multi_byte_chars("💣💢ᾠ latin", 3, 3),
      Ok((&" latin"[..], &"💣💢ᾠ"[..]))
    );
    assert_eq!(
      multi_byte_chars("latin", 0, 64),
      Ok((&"latin"[..], &""[..]))
    );
    assert_eq!(multi_byte_chars("باب", 1, 3), Ok((&""[..], &"باب"[..])));
    assert_eq!(multi_byte_chars("باب", 1, 2), Ok((&"ب"[..], &"با"[..])));
    assert_eq!(
      multi_byte_chars("latin", 1, 64),
      Err(Err::Error(Error::new(&"latin"[..], ErrorKind::TakeWhileMN)))
    );
  }
}

#[cfg(test)]
mod tests_rug_322 {
  use super::*;
  use crate::{Err, error::{Error, ErrorKind}, Needed, IResult};

  fn take6(s: &str) -> IResult<&str, &str> {
    take::<usize, _, Error<_>>(6usize)(s)
  }

  #[test]
  fn test_take6() {
    assert_eq!(take6("1234567"), Ok(("7", "123456")));
    assert_eq!(take6("things"), Ok(("", "things")));
    assert_eq!(take6("short"), Err(Err::Error(Error::new("short", ErrorKind::Eof))));
    assert_eq!(take6(""), Err(Err::Error(Error::new("", ErrorKind::Eof))));
  }
  
  #[test]
  fn test_take_utf8_chars() {
    use crate::error::Error;
    
    assert_eq!(take::<usize, _, Error<_>>(1usize)("💙"), Ok(("", "💙")));
    assert_eq!(take::<usize, _, Error<_>>(1usize)("💙".as_bytes()), Ok(("".as_bytes(), "💙".as_bytes())));
  }
}
    
#[cfg(test)]
mod tests_rug_324 {
    use super::*;
    use crate::{
        error::{make_error, Error, ErrorKind},
        Err, IResult,
    };
    use crate::bytes::complete::take_until1;
    
    #[test]
    fn test_rug() {
        
        fn until_eof(s: &str) -> IResult<&str, &str> {
            take_until1("eof")(s)
        }

        assert_eq!(until_eof("hello, worldeof"), Ok(("eof", "hello, world")));
        assert_eq!(until_eof("hello, world"), Err(Err::Error(make_error("hello, world", ErrorKind::TakeUntil))));
        assert_eq!(until_eof(""), Err(Err::Error(make_error("", ErrorKind::TakeUntil))));
        assert_eq!(until_eof("1eof2eof"), Ok(("eof2eof", "1")));
        assert_eq!(until_eof("eof"), Err(Err::Error(make_error("eof", ErrorKind::TakeUntil))));
        
    }
}