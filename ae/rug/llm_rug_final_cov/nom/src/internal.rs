//! Basic types to build the parsers

use self::Needed::*;
use crate::error::{self, ErrorKind, FromExternalError, ParseError};
use crate::lib::std::fmt;
use core::num::NonZeroUsize;

/// Holds the result of parsing functions
///
/// It depends on the input type `I`, the output type `O`, and the error type `E`
/// (by default `(I, nom::ErrorKind)`)
///
/// The `Ok` side is a pair containing the remainder of the input (the part of the data that
/// was not parsed) and the produced value. The `Err` side contains an instance of `nom::Err`.
///
/// Outside of the parsing code, you can use the [Finish::finish] method to convert
/// it to a more common result type
pub type IResult<I, O, E = error::Error<I>> = Result<(I, O), Err<E>>;

/// Helper trait to convert a parser's result to a more manageable type
pub trait Finish<I, O, E> {
  /// converts the parser's result to a type that is more consumable by error
  /// management libraries. It keeps the same `Ok` branch, and merges `Err::Error`
  /// and `Err::Failure` into the `Err` side.
  ///
  /// *warning*: if the result is `Err(Err::Incomplete(_))`, this method will panic.
  /// - "complete" parsers: It will not be an issue, `Incomplete` is never used
  /// - "streaming" parsers: `Incomplete` will be returned if there's not enough data
  /// for the parser to decide, and you should gather more data before parsing again.
  /// Once the parser returns either `Ok(_)`, `Err(Err::Error(_))` or `Err(Err::Failure(_))`,
  /// you can get out of the parsing loop and call `finish()` on the parser's result
  fn finish(self) -> Result<(I, O), E>;
}

impl<I, O, E> Finish<I, O, E> for IResult<I, O, E> {
  fn finish(self) -> Result<(I, O), E> {
    match self {
      Ok(res) => Ok(res),
      Err(Err::Error(e)) | Err(Err::Failure(e)) => Err(e),
      Err(Err::Incomplete(_)) => {
        panic!("Cannot call `finish()` on `Err(Err::Incomplete(_))`: this result means that the parser does not have enough data to decide, you should gather more data and try to reapply  the parser instead")
      }
    }
  }
}

/// Contains information on needed data if a parser returned `Incomplete`
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(nightly, warn(rustdoc::missing_doc_code_examples))]
pub enum Needed {
  /// Needs more data, but we do not know how much
  Unknown,
  /// Contains the required data size in bytes
  Size(NonZeroUsize),
}

impl Needed {
  /// Creates `Needed` instance, returns `Needed::Unknown` if the argument is zero
  pub fn new(s: usize) -> Self {
    match NonZeroUsize::new(s) {
      Some(sz) => Needed::Size(sz),
      None => Needed::Unknown,
    }
  }

  /// Indicates if we know how many bytes we need
  pub fn is_known(&self) -> bool {
    *self != Unknown
  }

  /// Maps a `Needed` to `Needed` by applying a function to a contained `Size` value.
  #[inline]
  pub fn map<F: Fn(NonZeroUsize) -> usize>(self, f: F) -> Needed {
    match self {
      Unknown => Unknown,
      Size(n) => Needed::new(f(n)),
    }
  }
}

/// The `Err` enum indicates the parser was not successful
///
/// It has three cases:
///
/// * `Incomplete` indicates that more data is needed to decide. The `Needed` enum
/// can contain how many additional bytes are necessary. If you are sure your parser
/// is working on full data, you can wrap your parser with the `complete` combinator
/// to transform that case in `Error`
/// * `Error` means some parser did not succeed, but another one might (as an example,
/// when testing different branches of an `alt` combinator)
/// * `Failure` indicates an unrecoverable error. For example, when a prefix has been
/// recognised and the next parser has been confirmed, if that parser fails, then the
/// entire process fails; there are no more parsers to try.
///
/// Distinguishing `Failure` this from `Error` is only relevant inside the parser's code. For
/// external consumers, both mean that parsing failed.
///
/// See also: [`nom::Finish`].
///
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(nightly, warn(rustdoc::missing_doc_code_examples))]
pub enum Err<E> {
  /// There was not enough data
  Incomplete(Needed),
  /// The parser had an error (recoverable)
  Error(E),
  /// The parser had an unrecoverable error: we got to the right
  /// branch and we know other branches won't work, so backtrack
  /// as fast as possible
  Failure(E),
}

impl<E> Err<E> {
  /// Tests if the result is Incomplete
  pub fn is_incomplete(&self) -> bool {
    matches!(self, Err::Incomplete(..))
  }

  /// Applies the given function to the inner error
  pub fn map<E2, F>(self, f: F) -> Err<E2>
  where
    F: FnOnce(E) -> E2,
  {
    match self {
      Err::Incomplete(n) => Err::Incomplete(n),
      Err::Failure(t) => Err::Failure(f(t)),
      Err::Error(t) => Err::Error(f(t)),
    }
  }

  /// Automatically converts between errors if the underlying type supports it
  pub fn convert<F>(e: Err<F>) -> Self
  where
    E: From<F>,
  {
    e.map(crate::lib::std::convert::Into::into)
  }
}

impl<T> Err<(T, ErrorKind)> {
  /// Maps `Err<(T, ErrorKind)>` to `Err<(U, ErrorKind)>` with the given `F: T -> U`
  pub fn map_input<U, F>(self, f: F) -> Err<(U, ErrorKind)>
  where
    F: FnOnce(T) -> U,
  {
    match self {
      Err::Incomplete(n) => Err::Incomplete(n),
      Err::Failure((input, k)) => Err::Failure((f(input), k)),
      Err::Error((input, k)) => Err::Error((f(input), k)),
    }
  }
}

impl<T> Err<error::Error<T>> {
  /// Maps `Err<error::Error<T>>` to `Err<error::Error<U>>` with the given `F: T -> U`
  pub fn map_input<U, F>(self, f: F) -> Err<error::Error<U>>
  where
    F: FnOnce(T) -> U,
  {
    match self {
      Err::Incomplete(n) => Err::Incomplete(n),
      Err::Failure(error::Error { input, code }) => Err::Failure(error::Error {
        input: f(input),
        code,
      }),
      Err::Error(error::Error { input, code }) => Err::Error(error::Error {
        input: f(input),
        code,
      }),
    }
  }
}

#[cfg(feature = "alloc")]
use crate::lib::std::{borrow::ToOwned, string::String, vec::Vec};
#[cfg(feature = "alloc")]
impl Err<(&[u8], ErrorKind)> {
  /// Obtaining ownership
  #[cfg_attr(feature = "docsrs", doc(cfg(feature = "alloc")))]
  pub fn to_owned(self) -> Err<(Vec<u8>, ErrorKind)> {
    self.map_input(ToOwned::to_owned)
  }
}

#[cfg(feature = "alloc")]
impl Err<(&str, ErrorKind)> {
  /// Obtaining ownership
  #[cfg_attr(feature = "docsrs", doc(cfg(feature = "alloc")))]
  pub fn to_owned(self) -> Err<(String, ErrorKind)> {
    self.map_input(ToOwned::to_owned)
  }
}

#[cfg(feature = "alloc")]
impl Err<error::Error<&[u8]>> {
  /// Obtaining ownership
  #[cfg_attr(feature = "docsrs", doc(cfg(feature = "alloc")))]
  pub fn to_owned(self) -> Err<error::Error<Vec<u8>>> {
    self.map_input(ToOwned::to_owned)
  }
}

#[cfg(feature = "alloc")]
impl Err<error::Error<&str>> {
  /// Obtaining ownership
  #[cfg_attr(feature = "docsrs", doc(cfg(feature = "alloc")))]
  pub fn to_owned(self) -> Err<error::Error<String>> {
    self.map_input(ToOwned::to_owned)
  }
}

impl<E: Eq> Eq for Err<E> {}

impl<E> fmt::Display for Err<E>
where
  E: fmt::Debug,
{
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Err::Incomplete(Needed::Size(u)) => write!(f, "Parsing requires {} bytes/chars", u),
      Err::Incomplete(Needed::Unknown) => write!(f, "Parsing requires more data"),
      Err::Failure(c) => write!(f, "Parsing Failure: {:?}", c),
      Err::Error(c) => write!(f, "Parsing Error: {:?}", c),
    }
  }
}

#[cfg(feature = "std")]
use std::error::Error;

#[cfg(feature = "std")]
impl<E> Error for Err<E>
where
  E: fmt::Debug,
{
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    None // no underlying error
  }
}

/// All nom parsers implement this trait
pub trait Parser<Input> {
  /// Type of the produced value
  type Output;
  /// Error type of this parser
  type Error: ParseError<Input>;

  /// A parser takes in input type, and returns a `Result` containing
  /// either the remaining input and the output value, or an error
  fn parse(&mut self, input: Input) -> IResult<Input, Self::Output, Self::Error>;

  /// Maps a function over the result of a parser
  fn map<G, O2>(self, g: G) -> Map<Self, G>
  where
    G: FnMut(Self::Output) -> O2,
    Self: core::marker::Sized,
  {
    Map { f: self, g }
  }

  /// Applies a function returning a `Result` over the result of a parser.
  fn map_res<G, O2, E2>(self, g: G) -> MapRes<Self, G>
  where
    G: Fn(Self::Output) -> Result<O2, E2>,
    Self::Error: FromExternalError<Input, E2>,
    Self: core::marker::Sized,
  {
    MapRes { f: self, g }
  }

  /// Applies a function returning an `Option` over the result of a parser.
  fn map_opt<G, O2>(self, g: G) -> MapOpt<Self, G>
  where
    G: Fn(Self::Output) -> Option<O2>,
    Self: core::marker::Sized,
  {
    MapOpt { f: self, g }
  }

  /// Creates a second parser from the output of the first one, then apply over the rest of the input
  fn flat_map<G, H>(self, g: G) -> FlatMap<Self, G>
  where
    G: FnMut(Self::Output) -> H,
    H: Parser<Input, Error = Self::Error>,
    Self: core::marker::Sized,
  {
    FlatMap { f: self, g }
  }

  /// Applies a second parser over the output of the first one
  fn and_then<G>(self, g: G) -> AndThen<Self, G>
  where
    G: Parser<Self::Output, Error = Self::Error>,
    Self: core::marker::Sized,
  {
    AndThen { f: self, g }
  }

  /// Applies a second parser after the first one, return their results as a tuple
  fn and<G, O2>(self, g: G) -> And<Self, G>
  where
    G: Parser<Input, Output = O2, Error = Self::Error>,
    Self: core::marker::Sized,
  {
    And { f: self, g }
  }

  /// Applies a second parser over the input if the first one failed
  fn or<G>(self, g: G) -> Or<Self, G>
  where
    G: Parser<Input, Output = Self::Output, Error = Self::Error>,
    Self: core::marker::Sized,
  {
    Or { f: self, g }
  }

  /// automatically converts the parser's output and error values to another type, as long as they
  /// implement the `From` trait
  fn into<O2: From<Self::Output>, E2: From<Self::Error>>(self) -> Into<Self, O2, E2>
  where
    Self: core::marker::Sized,
  {
    Into {
      f: self,
      phantom_out2: core::marker::PhantomData,
      phantom_err2: core::marker::PhantomData,
    }
  }
}

impl<I, O, E: ParseError<I>, F> Parser<I> for F
where
  F: FnMut(I) -> IResult<I, O, E>,
{
  type Output = O;
  type Error = E;
  fn parse(&mut self, i: I) -> IResult<I, O, E> {
    self(i)
  }
}

macro_rules! impl_parser_for_tuple {
  ($($parser:ident $output:ident),+) => (
    #[allow(non_snake_case)]
    impl<I, $($output),+, E: ParseError<I>, $($parser),+> Parser<I> for ($($parser),+,)
    where
      $($parser: Parser<I, Output = $output, Error = E>),+
    {
      type Output = ($($output),+,);
      type Error = E;
      fn parse(&mut self, i: I) -> IResult<I, ($($output),+,), E> {
        let ($(ref mut $parser),+,) = *self;

        $(let(i, $output) = $parser.parse(i)?;)+

        Ok((i, ($($output),+,)))
      }
    }
  )
}

macro_rules! impl_parser_for_tuples {
    ($parser1:ident $output1:ident, $($parser:ident $output:ident),+) => {
        impl_parser_for_tuples!(__impl $parser1 $output1; $($parser $output),+);
    };
    (__impl $($parser:ident $output:ident),+; $parser1:ident $output1:ident $(,$parser2:ident $output2:ident)*) => {
        impl_parser_for_tuple!($($parser $output),+);
        impl_parser_for_tuples!(__impl $($parser $output),+, $parser1 $output1; $($parser2 $output2),*);
    };
    (__impl $($parser:ident $output:ident),+;) => {
        impl_parser_for_tuple!($($parser $output),+);
    }
}

impl_parser_for_tuples!(P1 O1, P2 O2, P3 O3, P4 O4, P5 O5, P6 O6, P7 O7, P8 O8, P9 O9, P10 O10, P11 O11, P12 O12, P13 O13, P14 O14, P15 O15, P16 O16, P17 O17, P18 O18, P19 O19, P20 O20, P21 O21);

#[cfg(feature = "alloc")]
use alloc::boxed::Box;

#[cfg(feature = "alloc")]
impl<I, O, E: ParseError<I>> Parser<I> for Box<dyn Parser<I, Output = O, Error = E>> {
  type Output = O;
  type Error = E;
  fn parse(&mut self, input: I) -> IResult<I, O, E> {
    (**self).parse(input)
  }
}

/// Implementation of `Parser::map`
#[cfg_attr(nightly, warn(rustdoc::missing_doc_code_examples))]
pub struct Map<F, G> {
  f: F,
  g: G,
}

impl<I, O2, E: ParseError<I>, F: Parser<I, Error = E>, G: FnMut(<F as Parser<I>>::Output) -> O2>
  Parser<I> for Map<F, G>
{
  type Output = O2;
  type Error = E;

  fn parse(&mut self, i: I) -> IResult<I, O2, E> {
    match self.f.parse(i) {
      Err(e) => Err(e),
      Ok((i, o)) => Ok((i, (self.g)(o))),
    }
  }
}

/// Implementation of `Parser::map_res`
pub struct MapRes<F, G> {
  f: F,
  g: G,
}

impl<I, O2, E2, F, G> Parser<I> for MapRes<F, G>
where
  I: Clone,
  <F as Parser<I>>::Error: FromExternalError<I, E2>,
  F: Parser<I>,
  G: Fn(<F as Parser<I>>::Output) -> Result<O2, E2>,
{
  type Output = O2;
  type Error = <F as Parser<I>>::Error;
  fn parse(&mut self, input: I) -> IResult<I, O2, <F as Parser<I>>::Error> {
    let i = input.clone();
    let (input, o1) = self.f.parse(input)?;
    match (self.g)(o1) {
      Ok(o2) => Ok((input, o2)),
      Err(e) => Err(Err::Error(<F as Parser<I>>::Error::from_external_error(
        i,
        ErrorKind::MapRes,
        e,
      ))),
    }
  }
}

/// Implementation of `Parser::map_opt`
pub struct MapOpt<F, G> {
  f: F,
  g: G,
}

impl<I, O2, F, G> Parser<I> for MapOpt<F, G>
where
  I: Clone,
  F: Parser<I>,
  G: Fn(<F as Parser<I>>::Output) -> Option<O2>,
{
  type Output = O2;
  type Error = <F as Parser<I>>::Error;

  fn parse(&mut self, input: I) -> IResult<I, O2, <F as Parser<I>>::Error> {
    let i = input.clone();
    let (input, o1) = self.f.parse(input)?;
    match (self.g)(o1) {
      Some(o2) => Ok((input, o2)),
      None => Err(Err::Error(<F as Parser<I>>::Error::from_error_kind(
        i,
        ErrorKind::MapOpt,
      ))),
    }
  }
}

/// Implementation of `Parser::flat_map`
#[cfg_attr(nightly, warn(rustdoc::missing_doc_code_examples))]
pub struct FlatMap<F, G> {
  f: F,
  g: G,
}

impl<
    I,
    E: ParseError<I>,
    F: Parser<I, Error = E>,
    G: FnMut(<F as Parser<I>>::Output) -> H,
    H: Parser<I, Error = E>,
  > Parser<I> for FlatMap<F, G>
{
  type Output = <H as Parser<I>>::Output;
  type Error = E;

  fn parse(&mut self, i: I) -> IResult<I, Self::Output, E> {
    let (i, o1) = self.f.parse(i)?;
    (self.g)(o1).parse(i)
  }
}

/// Implementation of `Parser::and_then`
#[cfg_attr(nightly, warn(rustdoc::missing_doc_code_examples))]
pub struct AndThen<F, G> {
  f: F,
  g: G,
}

impl<I, F: Parser<I>, G: Parser<<F as Parser<I>>::Output, Error = <F as Parser<I>>::Error>>
  Parser<I> for AndThen<F, G>
{
  type Output = <G as Parser<<F as Parser<I>>::Output>>::Output;
  type Error = <F as Parser<I>>::Error;

  fn parse(&mut self, i: I) -> IResult<I, Self::Output, Self::Error> {
    let (i, o1) = self.f.parse(i)?;
    let (_, o2) = self.g.parse(o1)?;
    Ok((i, o2))
  }
}

/// Implementation of `Parser::and`
#[cfg_attr(nightly, warn(rustdoc::missing_doc_code_examples))]
pub struct And<F, G> {
  f: F,
  g: G,
}

impl<I, E: ParseError<I>, F: Parser<I, Error = E>, G: Parser<I, Error = E>> Parser<I>
  for And<F, G>
{
  type Output = (<F as Parser<I>>::Output, <G as Parser<I>>::Output);
  type Error = E;

  fn parse(&mut self, i: I) -> IResult<I, Self::Output, Self::Error> {
    let (i, o1) = self.f.parse(i)?;
    let (i, o2) = self.g.parse(i)?;
    Ok((i, (o1, o2)))
  }
}

/// Implementation of `Parser::or`
#[cfg_attr(nightly, warn(rustdoc::missing_doc_code_examples))]
pub struct Or<F, G> {
  f: F,
  g: G,
}

impl<
    I: Clone,
    O,
    E: ParseError<I>,
    F: Parser<I, Output = O, Error = E>,
    G: Parser<I, Output = O, Error = E>,
  > Parser<I> for Or<F, G>
{
  type Output = <F as Parser<I>>::Output;
  type Error = <F as Parser<I>>::Error;

  fn parse(&mut self, i: I) -> IResult<I, Self::Output, Self::Error> {
    match self.f.parse(i.clone()) {
      Err(Err::Error(e1)) => match self.g.parse(i) {
        Err(Err::Error(e2)) => Err(Err::Error(e1.or(e2))),
        res => res,
      },
      res => res,
    }
  }
}

/// Implementation of `Parser::into`
#[cfg_attr(nightly, warn(rustdoc::missing_doc_code_examples))]
pub struct Into<F, O2, E2> {
  f: F,
  phantom_out2: core::marker::PhantomData<O2>,
  phantom_err2: core::marker::PhantomData<E2>,
}

impl<
    I: Clone,
    O2: From<<F as Parser<I>>::Output>,
    E2: crate::error::ParseError<I> + From<<F as Parser<I>>::Error>,
    F: Parser<I>,
  > Parser<I> for Into<F, O2, E2>
{
  type Output = O2;
  type Error = E2;

  fn parse(&mut self, i: I) -> IResult<I, Self::Output, Self::Error> {
    match self.f.parse(i) {
      Ok((i, o)) => Ok((i, o.into())),
      Err(Err::Error(e)) => Err(Err::Error(e.into())),
      Err(Err::Failure(e)) => Err(Err::Failure(e.into())),
      Err(Err::Incomplete(e)) => Err(Err::Incomplete(e)),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::error::ErrorKind;

  use crate::bytes::streaming::{tag, take};
  use crate::number::streaming::be_u16;
  use crate::sequence::terminated;

  #[doc(hidden)]
  #[macro_export]
  macro_rules! assert_size (
    ($t:ty, $sz:expr) => (
      assert_eq!($crate::lib::std::mem::size_of::<$t>(), $sz);
    );
  );

  #[test]
  #[cfg(target_pointer_width = "64")]
  fn size_test() {
    assert_size!(IResult<&[u8], &[u8], (&[u8], u32)>, 40);
    //FIXME: since rust 1.65, this is now 32 bytes, likely thanks to https://github.com/rust-lang/rust/pull/94075
    // deactivating that test for now because it'll have different values depending on the rust version
    // assert_size!(IResult<&str, &str, u32>, 40);
    assert_size!(Needed, 8);
    assert_size!(Err<u32>, 16);
    assert_size!(ErrorKind, 1);
  }

  #[test]
  fn err_map_test() {
    let e = Err::Error(1);
    assert_eq!(e.map(|v| v + 1), Err::Error(2));
  }

  #[test]
  fn native_tuple_test() {
    fn tuple_3(i: &[u8]) -> IResult<&[u8], (u16, &[u8])> {
      terminated((be_u16, take(3u8)), tag("fg"))(i)
    }

    assert_eq!(
      tuple_3(&b"abcdefgh"[..]),
      Ok((&b"h"[..], (0x6162u16, &b"cde"[..])))
    );
    assert_eq!(tuple_3(&b"abcd"[..]), Err(Err::Incomplete(Needed::new(1))));
    assert_eq!(tuple_3(&b"abcde"[..]), Err(Err::Incomplete(Needed::new(2))));
    assert_eq!(
      tuple_3(&b"abcdejk"[..]),
      Err(Err::Error(error_position!(&b"jk"[..], ErrorKind::Tag)))
    );
  }
}
#[cfg(test)]
mod tests_rug_527 {
    use super::*;
    use crate::internal::Needed;
    use std::num::NonZeroUsize;
    
    #[test]
    fn test_rug() {
        let p0: usize = 8;

        Needed::new(p0);
    }
}
#[cfg(test)]
mod tests_rug_528 {
    use super::*;
    use crate::internal::Needed;

    #[test]
    fn test_rug() {
        let p0: Needed = Needed::Unknown;

        assert_eq!(p0.is_known(), false);
    }
}
#[cfg(test)]
mod tests_rug_529 {
    use super::*;
    use crate::internal::Needed;
    use std::num::NonZeroUsize;

    #[test]
    fn test_rug() {
        let p0: Needed = Needed::new(5);
        let p1: fn(NonZeroUsize) -> usize = |n| n.get() * 2;

        let _ = Needed::map::<fn(NonZeroUsize) -> usize>(p0, p1);
    }
}#[cfg(test)]
mod tests_rug_531 {
    use super::*;
    use crate::internal::Err;
    
    #[test]
    fn test_rug() {
        let mut p0: Err<()> = Err::Failure(());
        let mut p1: fn(()) -> usize = |_: ()| 0;
        
        p0.map(p1);
    }
}
#[cfg(test)]
mod tests_rug_535 {
    use super::*;
    use crate::internal::{Err, error::ErrorKind};

    #[test]
    fn test_rug() {
        let mut p0: Err<(&[u8], ErrorKind)> = Err::Error((&b"input"[..], ErrorKind::Tag));
      
        p0.to_owned();
    }
}#[cfg(test)]
mod tests_rug_536 {
    use super::*;
    use crate::internal::{Err, ErrorKind};

    #[test]
    fn test_to_owned() {
        let p0: Err<(&str, ErrorKind)> = Err::Error(("input", ErrorKind::Alpha));

        p0.to_owned();
    }
}
#[cfg(test)]
mod tests_rug_537 {
    use super::*;
    use crate::Err;
    use crate::error::Error;
    use crate::error::ErrorKind;

    #[test]
    fn test_rug() {
        let mut p0: Err<Error<&[u8]>> = Err::Failure(Error::new(&b"input"[..], ErrorKind::Digit));
        
        let owned = p0.to_owned();
        // add assertions or further test code here
    }
}