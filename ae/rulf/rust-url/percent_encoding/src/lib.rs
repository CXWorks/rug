//! URLs use special chacters to indicate the parts of the request.
//! For example, a `?` question mark marks the end of a path and the start of a query string.
//! In order for that character to exist inside a path, it needs to be encoded differently.
//!
//! Percent encoding replaces reserved characters with the `%` escape character
//! followed by a byte value as two hexadecimal digits.
//! For example, an ASCII space is replaced with `%20`.
//!
//! When encoding, the set of characters that can (and should, for readability) be left alone
//! depends on the context.
//! The `?` question mark mentioned above is not a separator when used literally
//! inside of a query string, and therefore does not need to be encoded.
//! The [`AsciiSet`] parameter of [`percent_encode`] and [`utf8_percent_encode`]
//! lets callers configure this.
//!
//! This crate delibarately does not provide many different sets.
//! Users should consider in what context the encoded string will be used,
//! read relevant specifications, and define their own set.
//! This is done by using the `add` method of an existing set.
//!
//! # Examples
//!
//! ```
//! use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
//!
//! /// https://url.spec.whatwg.org/#fragment-percent-encode-set
//! const FRAGMENT: &AsciiSet = &CONTROLS.add(b' ').add(b'"').add(b'<').add(b'>').add(b'`');
//!
//! assert_eq!(utf8_percent_encode("foo <bar>", FRAGMENT).to_string(), "foo%20%3Cbar%3E");
//! ```
#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(not(feature = "std"))]
use core::{fmt, mem, slice, str};
#[cfg(feature = "std")]
use std::borrow::Cow;
#[cfg(feature = "std")]
use std::{fmt, mem, slice, str};
/// Represents a set of characters or bytes in the ASCII range.
///
/// This used in [`percent_encode`] and [`utf8_percent_encode`].
/// This is simlar to [percent-encode sets](https://url.spec.whatwg.org/#percent-encoded-bytes).
///
/// Use the `add` method of an existing set to define a new set. For example:
///
/// ```
/// use percent_encoding::{AsciiSet, CONTROLS};
///
/// /// https://url.spec.whatwg.org/#fragment-percent-encode-set
/// const FRAGMENT: &AsciiSet = &CONTROLS.add(b' ').add(b'"').add(b'<').add(b'>').add(b'`');
/// ```
pub struct AsciiSet {
    mask: [Chunk; ASCII_RANGE_LEN / BITS_PER_CHUNK],
}
type Chunk = u32;
const ASCII_RANGE_LEN: usize = 0x80;
const BITS_PER_CHUNK: usize = 8 * mem::size_of::<Chunk>();
impl AsciiSet {
    /// Called with UTF-8 bytes rather than code points.
    /// Not used for non-ASCII bytes.
    const fn contains(&self, byte: u8) -> bool {
        let chunk = self.mask[byte as usize / BITS_PER_CHUNK];
        let mask = 1 << (byte as usize % BITS_PER_CHUNK);
        (chunk & mask) != 0
    }
    fn should_percent_encode(&self, byte: u8) -> bool {
        !byte.is_ascii() || self.contains(byte)
    }
    pub const fn add(&self, byte: u8) -> Self {
        let mut mask = self.mask;
        mask[byte as usize / BITS_PER_CHUNK] |= 1 << (byte as usize % BITS_PER_CHUNK);
        AsciiSet { mask }
    }
    pub const fn remove(&self, byte: u8) -> Self {
        let mut mask = self.mask;
        mask[byte as usize / BITS_PER_CHUNK] &= !(1 << (byte as usize % BITS_PER_CHUNK));
        AsciiSet { mask }
    }
}
/// The set of 0x00 to 0x1F (C0 controls), and 0x7F (DEL).
///
/// Note that this includes the newline and tab characters, but not the space 0x20.
///
/// <https://url.spec.whatwg.org/#c0-control-percent-encode-set>
pub const CONTROLS: &AsciiSet = &AsciiSet {
    mask: [!0_u32, 0, 0, 1 << (0x7F_u32 % 32)],
};
macro_rules! static_assert {
    ($($bool:expr,)+) => {
        fn _static_assert() { $(let _ = mem::transmute::< [u8; $bool as usize], u8 >;)+ }
    };
}
static_assert! {
    CONTROLS.contains(0x00), CONTROLS.contains(0x1F), ! CONTROLS.contains(0x20), !
    CONTROLS.contains(0x7E), CONTROLS.contains(0x7F),
}
/// Everything that is not an ASCII letter or digit.
///
/// This is probably more eager than necessary in any context.
pub const NON_ALPHANUMERIC: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'!')
    .add(b'"')
    .add(b'#')
    .add(b'$')
    .add(b'%')
    .add(b'&')
    .add(b'\'')
    .add(b'(')
    .add(b')')
    .add(b'*')
    .add(b'+')
    .add(b',')
    .add(b'-')
    .add(b'.')
    .add(b'/')
    .add(b':')
    .add(b';')
    .add(b'<')
    .add(b'=')
    .add(b'>')
    .add(b'?')
    .add(b'@')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'^')
    .add(b'_')
    .add(b'`')
    .add(b'{')
    .add(b'|')
    .add(b'}')
    .add(b'~');
/// Return the percent-encoding of the given byte.
///
/// This is unconditional, unlike `percent_encode()` which has an `AsciiSet` parameter.
///
/// # Examples
///
/// ```
/// use percent_encoding::percent_encode_byte;
///
/// assert_eq!("foo bar".bytes().map(percent_encode_byte).collect::<String>(),
///            "%66%6F%6F%20%62%61%72");
/// ```
pub fn percent_encode_byte(byte: u8) -> &'static str {
    let index = usize::from(byte) * 3;
    &"\
      %00%01%02%03%04%05%06%07%08%09%0A%0B%0C%0D%0E%0F\
      %10%11%12%13%14%15%16%17%18%19%1A%1B%1C%1D%1E%1F\
      %20%21%22%23%24%25%26%27%28%29%2A%2B%2C%2D%2E%2F\
      %30%31%32%33%34%35%36%37%38%39%3A%3B%3C%3D%3E%3F\
      %40%41%42%43%44%45%46%47%48%49%4A%4B%4C%4D%4E%4F\
      %50%51%52%53%54%55%56%57%58%59%5A%5B%5C%5D%5E%5F\
      %60%61%62%63%64%65%66%67%68%69%6A%6B%6C%6D%6E%6F\
      %70%71%72%73%74%75%76%77%78%79%7A%7B%7C%7D%7E%7F\
      %80%81%82%83%84%85%86%87%88%89%8A%8B%8C%8D%8E%8F\
      %90%91%92%93%94%95%96%97%98%99%9A%9B%9C%9D%9E%9F\
      %A0%A1%A2%A3%A4%A5%A6%A7%A8%A9%AA%AB%AC%AD%AE%AF\
      %B0%B1%B2%B3%B4%B5%B6%B7%B8%B9%BA%BB%BC%BD%BE%BF\
      %C0%C1%C2%C3%C4%C5%C6%C7%C8%C9%CA%CB%CC%CD%CE%CF\
      %D0%D1%D2%D3%D4%D5%D6%D7%D8%D9%DA%DB%DC%DD%DE%DF\
      %E0%E1%E2%E3%E4%E5%E6%E7%E8%E9%EA%EB%EC%ED%EE%EF\
      %F0%F1%F2%F3%F4%F5%F6%F7%F8%F9%FA%FB%FC%FD%FE%FF\
      "[index..index
        + 3]
}
/// Percent-encode the given bytes with the given set.
///
/// Non-ASCII bytes and bytes in `ascii_set` are encoded.
///
/// The return type:
///
/// * Implements `Iterator<Item = &str>` and therefore has a `.collect::<String>()` method,
/// * Implements `Display` and therefore has a `.to_string()` method,
/// * Implements `Into<Cow<str>>` borrowing `input` when none of its bytes are encoded.
///
/// # Examples
///
/// ```
/// use percent_encoding::{percent_encode, NON_ALPHANUMERIC};
///
/// assert_eq!(percent_encode(b"foo bar?", NON_ALPHANUMERIC).to_string(), "foo%20bar%3F");
/// ```
#[inline]
pub fn percent_encode<'a>(
    input: &'a [u8],
    ascii_set: &'static AsciiSet,
) -> PercentEncode<'a> {
    PercentEncode {
        bytes: input,
        ascii_set,
    }
}
/// Percent-encode the UTF-8 encoding of the given string.
///
/// See [`percent_encode`] regarding the return type.
///
/// # Examples
///
/// ```
/// use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
///
/// assert_eq!(utf8_percent_encode("foo bar?", NON_ALPHANUMERIC).to_string(), "foo%20bar%3F");
/// ```
#[inline]
pub fn utf8_percent_encode<'a>(
    input: &'a str,
    ascii_set: &'static AsciiSet,
) -> PercentEncode<'a> {
    percent_encode(input.as_bytes(), ascii_set)
}
/// The return type of [`percent_encode`] and [`utf8_percent_encode`].
#[derive(Clone)]
pub struct PercentEncode<'a> {
    bytes: &'a [u8],
    ascii_set: &'static AsciiSet,
}
impl<'a> Iterator for PercentEncode<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        if let Some((&first_byte, remaining)) = self.bytes.split_first() {
            if self.ascii_set.should_percent_encode(first_byte) {
                self.bytes = remaining;
                Some(percent_encode_byte(first_byte))
            } else {
                for (i, &byte) in remaining.iter().enumerate() {
                    if self.ascii_set.should_percent_encode(byte) {
                        let (unchanged_slice, remaining) = self.bytes.split_at(1 + i);
                        self.bytes = remaining;
                        return Some(unsafe {
                            str::from_utf8_unchecked(unchanged_slice)
                        });
                    }
                }
                let unchanged_slice = self.bytes;
                self.bytes = &[][..];
                Some(unsafe { str::from_utf8_unchecked(unchanged_slice) })
            }
        } else {
            None
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.bytes.is_empty() { (0, Some(0)) } else { (1, Some(self.bytes.len())) }
    }
}
impl<'a> fmt::Display for PercentEncode<'a> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for c in (*self).clone() {
            formatter.write_str(c)?
        }
        Ok(())
    }
}
#[cfg(feature = "std")]
impl<'a> From<PercentEncode<'a>> for Cow<'a, str> {
    fn from(mut iter: PercentEncode<'a>) -> Self {
        match iter.next() {
            None => "".into(),
            Some(first) => {
                match iter.next() {
                    None => first.into(),
                    Some(second) => {
                        let mut string = first.to_owned();
                        string.push_str(second);
                        string.extend(iter);
                        string.into()
                    }
                }
            }
        }
    }
}
/// Percent-decode the given string.
///
/// <https://url.spec.whatwg.org/#string-percent-decode>
///
/// See [`percent_decode`] regarding the return type.
#[inline]
pub fn percent_decode_str(input: &str) -> PercentDecode<'_> {
    percent_decode(input.as_bytes())
}
/// Percent-decode the given bytes.
///
/// <https://url.spec.whatwg.org/#percent-decode>
///
/// Any sequence of `%` followed by two hexadecimal digits is decoded.
/// The return type:
///
/// * Implements `Into<Cow<u8>>` borrowing `input` when it contains no percent-encoded sequence,
/// * Implements `Iterator<Item = u8>` and therefore has a `.collect::<Vec<u8>>()` method,
/// * Has `decode_utf8()` and `decode_utf8_lossy()` methods.
///
#[cfg_attr(
    feature = "std",
    doc = r##"
# Examples

```
use percent_encoding::percent_decode;

assert_eq!(percent_decode(b"foo%20bar%3f").decode_utf8().unwrap(), "foo bar?");
```
"##
)]
#[inline]
pub fn percent_decode(input: &[u8]) -> PercentDecode<'_> {
    PercentDecode {
        bytes: input.iter(),
    }
}
/// The return type of [`percent_decode`].
#[derive(Clone, Debug)]
pub struct PercentDecode<'a> {
    bytes: slice::Iter<'a, u8>,
}
fn after_percent_sign(iter: &mut slice::Iter<'_, u8>) -> Option<u8> {
    let mut cloned_iter = iter.clone();
    let h = char::from(*cloned_iter.next()?).to_digit(16)?;
    let l = char::from(*cloned_iter.next()?).to_digit(16)?;
    *iter = cloned_iter;
    Some(h as u8 * 0x10 + l as u8)
}
impl<'a> Iterator for PercentDecode<'a> {
    type Item = u8;
    fn next(&mut self) -> Option<u8> {
        self.bytes
            .next()
            .map(|&byte| {
                if byte == b'%' {
                    after_percent_sign(&mut self.bytes).unwrap_or(byte)
                } else {
                    byte
                }
            })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let bytes = self.bytes.len();
        ((bytes + 2) / 3, Some(bytes))
    }
}
#[cfg(feature = "std")]
impl<'a> From<PercentDecode<'a>> for Cow<'a, [u8]> {
    fn from(iter: PercentDecode<'a>) -> Self {
        match iter.if_any() {
            Some(vec) => Cow::Owned(vec),
            None => Cow::Borrowed(iter.bytes.as_slice()),
        }
    }
}
impl<'a> PercentDecode<'a> {
    /// If the percent-decoding is different from the input, return it as a new bytes vector.
    #[cfg(feature = "std")]
    fn if_any(&self) -> Option<Vec<u8>> {
        let mut bytes_iter = self.bytes.clone();
        while bytes_iter.any(|&b| b == b'%') {
            if let Some(decoded_byte) = after_percent_sign(&mut bytes_iter) {
                let initial_bytes = self.bytes.as_slice();
                let unchanged_bytes_len = initial_bytes.len() - bytes_iter.len() - 3;
                let mut decoded = initial_bytes[..unchanged_bytes_len].to_owned();
                decoded.push(decoded_byte);
                decoded.extend(PercentDecode { bytes: bytes_iter });
                return Some(decoded);
            }
        }
        None
    }
    /// Decode the result of percent-decoding as UTF-8.
    ///
    /// This is return `Err` when the percent-decoded bytes are not well-formed in UTF-8.
    #[cfg(feature = "std")]
    pub fn decode_utf8(self) -> Result<Cow<'a, str>, str::Utf8Error> {
        match self.clone().into() {
            Cow::Borrowed(bytes) => {
                match str::from_utf8(bytes) {
                    Ok(s) => Ok(s.into()),
                    Err(e) => Err(e),
                }
            }
            Cow::Owned(bytes) => {
                match String::from_utf8(bytes) {
                    Ok(s) => Ok(s.into()),
                    Err(e) => Err(e.utf8_error()),
                }
            }
        }
    }
    /// Decode the result of percent-decoding as UTF-8, lossily.
    ///
    /// Invalid UTF-8 percent-encoded byte sequences will be replaced � U+FFFD,
    /// the replacement character.
    #[cfg(feature = "std")]
    pub fn decode_utf8_lossy(self) -> Cow<'a, str> {
        decode_utf8_lossy(self.clone().into())
    }
}
#[cfg(feature = "std")]
fn decode_utf8_lossy(input: Cow<'_, [u8]>) -> Cow<'_, str> {
    match input {
        Cow::Borrowed(bytes) => String::from_utf8_lossy(bytes),
        Cow::Owned(bytes) => {
            match String::from_utf8_lossy(&bytes) {
                Cow::Borrowed(utf8) => {
                    let raw_utf8: *const [u8];
                    raw_utf8 = utf8.as_bytes();
                    debug_assert!(raw_utf8 == &* bytes as * const [u8]);
                    Cow::Owned(unsafe { String::from_utf8_unchecked(bytes) })
                }
                Cow::Owned(s) => Cow::Owned(s),
            }
        }
    }
}
#[cfg(test)]
mod tests_llm_16_1 {
    use super::*;
    use crate::*;
    #[test]
    fn test_next() {
        let _rug_st_tests_llm_16_1_rrrruuuugggg_test_next = 0;
        let rug_fuzz_0 = b'A';
        let bytes: Vec<u8> = vec![
            rug_fuzz_0, b'%', b'3', b'4', b'%', b'6', b'1', b'%', b'0', b'0'
        ];
        let mut decoder = PercentDecode {
            bytes: bytes.iter(),
        };
        debug_assert_eq!(decoder.next(), Some(b'A'));
        debug_assert_eq!(decoder.next(), Some(b'4'));
        debug_assert_eq!(decoder.next(), Some(b'a'));
        debug_assert_eq!(decoder.next(), Some(b'\0'));
        debug_assert_eq!(decoder.next(), None);
        let _rug_ed_tests_llm_16_1_rrrruuuugggg_test_next = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_2 {
    use crate::PercentDecode;
    #[test]
    fn size_hint_returns_correct_values() {
        let _rug_st_tests_llm_16_2_rrrruuuugggg_size_hint_returns_correct_values = 0;
        let rug_fuzz_0 = 0x25;
        let rug_fuzz_1 = 0x68;
        let rug_fuzz_2 = 0x65;
        let rug_fuzz_3 = 0x6c;
        let rug_fuzz_4 = 0x6c;
        let bytes: [u8; 5] = [
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
        ];
        let percent_decode = PercentDecode {
            bytes: bytes.iter(),
        };
        let (lower, upper) = percent_decode.size_hint();
        debug_assert_eq!(lower, 2);
        debug_assert_eq!(upper, Some(5));
        let _rug_ed_tests_llm_16_2_rrrruuuugggg_size_hint_returns_correct_values = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_8 {
    use crate::PercentDecode;
    use std::borrow::Cow;
    #[test]
    fn test_from_percent_decode() {
        let _rug_st_tests_llm_16_8_rrrruuuugggg_test_from_percent_decode = 0;
        let rug_fuzz_0 = b"hello%20world%21";
        let rug_fuzz_1 = b"hello%20world%21";
        let data: &[u8] = rug_fuzz_0;
        let percent_decode = PercentDecode {
            bytes: data.iter(),
        };
        let cow: Cow<[u8]> = From::from(percent_decode);
        let expected: Cow<[u8]> = Cow::Borrowed(rug_fuzz_1);
        debug_assert_eq!(cow, expected);
        let _rug_ed_tests_llm_16_8_rrrruuuugggg_test_from_percent_decode = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_9 {
    use super::*;
    use crate::*;
    use std::borrow::Cow;
    #[test]
    fn test_from() {
        let _rug_st_tests_llm_16_9_rrrruuuugggg_test_from = 0;
        let iter: PercentEncode = unimplemented!();
        let result: Cow<str> = From::from(iter);
        unimplemented!();
        let _rug_ed_tests_llm_16_9_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_10 {
    use super::*;
    use crate::*;
    #[test]
    fn test_add() {
        let _rug_st_tests_llm_16_10_rrrruuuugggg_test_add = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = b'A';
        let rug_fuzz_2 = 1;
        let set = AsciiSet {
            mask: [rug_fuzz_0; ASCII_RANGE_LEN / BITS_PER_CHUNK],
        };
        let new_set = set.add(rug_fuzz_1);
        let expected_set = AsciiSet {
            mask: [rug_fuzz_2; ASCII_RANGE_LEN / BITS_PER_CHUNK],
        };
        debug_assert_eq!(new_set.mask, expected_set.mask);
        let _rug_ed_tests_llm_16_10_rrrruuuugggg_test_add = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_13 {
    use super::*;
    use crate::*;
    #[test]
    fn test_remove() {
        let _rug_st_tests_llm_16_13_rrrruuuugggg_test_remove = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = b'A';
        let set = AsciiSet {
            mask: [rug_fuzz_0; ASCII_RANGE_LEN / BITS_PER_CHUNK],
        };
        let byte = rug_fuzz_1;
        let result = set.remove(byte);
        debug_assert_eq!(result.mask[byte as usize / BITS_PER_CHUNK], 0);
        let _rug_ed_tests_llm_16_13_rrrruuuugggg_test_remove = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_14 {
    use super::*;
    use crate::*;
    #[test]
    fn test_should_percent_encode_non_ascii() {
        let _rug_st_tests_llm_16_14_rrrruuuugggg_test_should_percent_encode_non_ascii = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 128;
        let ascii_set = AsciiSet {
            mask: [rug_fuzz_0; ASCII_RANGE_LEN / BITS_PER_CHUNK],
        };
        debug_assert_eq!(ascii_set.should_percent_encode(rug_fuzz_1), true);
        let _rug_ed_tests_llm_16_14_rrrruuuugggg_test_should_percent_encode_non_ascii = 0;
    }
    #[test]
    fn test_should_percent_encode_in_set() {
        let _rug_st_tests_llm_16_14_rrrruuuugggg_test_should_percent_encode_in_set = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 32;
        let ascii_set = AsciiSet {
            mask: [rug_fuzz_0; ASCII_RANGE_LEN / BITS_PER_CHUNK],
        };
        debug_assert_eq!(ascii_set.should_percent_encode(rug_fuzz_1), true);
        let _rug_ed_tests_llm_16_14_rrrruuuugggg_test_should_percent_encode_in_set = 0;
    }
    #[test]
    fn test_should_percent_encode_not_in_set() {
        let _rug_st_tests_llm_16_14_rrrruuuugggg_test_should_percent_encode_not_in_set = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 33;
        let ascii_set = AsciiSet {
            mask: [rug_fuzz_0; ASCII_RANGE_LEN / BITS_PER_CHUNK],
        };
        debug_assert_eq!(ascii_set.should_percent_encode(rug_fuzz_1), false);
        let _rug_ed_tests_llm_16_14_rrrruuuugggg_test_should_percent_encode_not_in_set = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_17 {
    use super::*;
    use crate::*;
    use std::borrow::Cow;
    use std::str::Utf8Error;
    #[test]
    fn test_decode_utf8_lossy() {
        let _rug_st_tests_llm_16_17_rrrruuuugggg_test_decode_utf8_lossy = 0;
        let rug_fuzz_0 = 116;
        let rug_fuzz_1 = 101;
        let rug_fuzz_2 = 115;
        let rug_fuzz_3 = 116;
        let rug_fuzz_4 = 37;
        let rug_fuzz_5 = 50;
        let rug_fuzz_6 = 48;
        let rug_fuzz_7 = 116;
        let rug_fuzz_8 = 101;
        let rug_fuzz_9 = 120;
        let rug_fuzz_10 = 116;
        let rug_fuzz_11 = "test%20text";
        let bytes: &[u8] = &[
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
            rug_fuzz_6,
            rug_fuzz_7,
            rug_fuzz_8,
            rug_fuzz_9,
            rug_fuzz_10,
        ];
        let percent_decode = PercentDecode {
            bytes: bytes.iter(),
        };
        let result: Cow<str> = percent_decode.decode_utf8_lossy();
        let expected: Cow<str> = rug_fuzz_11.into();
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_17_rrrruuuugggg_test_decode_utf8_lossy = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_18 {
    use super::*;
    use crate::*;
    #[test]
    fn test_if_any() {
        let _rug_st_tests_llm_16_18_rrrruuuugggg_test_if_any = 0;
        let rug_fuzz_0 = b'a';
        let rug_fuzz_1 = b'%';
        let rug_fuzz_2 = b'2';
        let rug_fuzz_3 = b'0';
        let rug_fuzz_4 = b' ';
        let rug_fuzz_5 = b't';
        let rug_fuzz_6 = b'e';
        let rug_fuzz_7 = b's';
        let rug_fuzz_8 = b't';
        let rug_fuzz_9 = b'%';
        let rug_fuzz_10 = b'2';
        let rug_fuzz_11 = b'5';
        let rug_fuzz_12 = b'%';
        let rug_fuzz_13 = b'3';
        let rug_fuzz_14 = b'0';
        let rug_fuzz_15 = b'%';
        let rug_fuzz_16 = b'4';
        let rug_fuzz_17 = b'5';
        let rug_fuzz_18 = b'%';
        let rug_fuzz_19 = b'4';
        let rug_fuzz_20 = b'0';
        let rug_fuzz_21 = b' ';
        let rug_fuzz_22 = b's';
        let rug_fuzz_23 = b't';
        let rug_fuzz_24 = b'r';
        let rug_fuzz_25 = b'i';
        let rug_fuzz_26 = b'n';
        let rug_fuzz_27 = b'g';
        let rug_fuzz_28 = b'a';
        let rug_fuzz_29 = b'b';
        let rug_fuzz_30 = b'c';
        let rug_fuzz_31 = b' ';
        let rug_fuzz_32 = b'd';
        let rug_fuzz_33 = b'e';
        let rug_fuzz_34 = b'f';
        let bytes: &[u8] = &[
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
            rug_fuzz_6,
            rug_fuzz_7,
            rug_fuzz_8,
            rug_fuzz_9,
            rug_fuzz_10,
            rug_fuzz_11,
            rug_fuzz_12,
            rug_fuzz_13,
            rug_fuzz_14,
            rug_fuzz_15,
            rug_fuzz_16,
            rug_fuzz_17,
            rug_fuzz_18,
            rug_fuzz_19,
            rug_fuzz_20,
            rug_fuzz_21,
            rug_fuzz_22,
            rug_fuzz_23,
            rug_fuzz_24,
            rug_fuzz_25,
            rug_fuzz_26,
            rug_fuzz_27,
        ];
        let decoded = PercentDecode {
            bytes: bytes.iter(),
        }
            .if_any();
        debug_assert_eq!(decoded.unwrap(), b"a%20 test%25%30%45%40 string");
        let bytes: &[u8] = &[
            rug_fuzz_28,
            rug_fuzz_29,
            rug_fuzz_30,
            rug_fuzz_31,
            rug_fuzz_32,
            rug_fuzz_33,
            rug_fuzz_34,
        ];
        let decoded = PercentDecode {
            bytes: bytes.iter(),
        }
            .if_any();
        debug_assert_eq!(decoded, None);
        let _rug_ed_tests_llm_16_18_rrrruuuugggg_test_if_any = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_21 {
    use super::*;
    use crate::*;
    #[test]
    fn test_after_percent_sign_with_valid_hex() {
        let mut iter = [b'A', b'B'].iter();
        assert_eq!(after_percent_sign(& mut iter), Some(0xAB));
    }
    #[test]
    fn test_after_percent_sign_with_invalid_hex() {
        let mut iter = [b'G', b'H'].iter();
        assert_eq!(after_percent_sign(& mut iter), None);
    }
    #[test]
    fn test_after_percent_sign_with_empty_iter() {
        let mut iter = [].iter();
        assert_eq!(after_percent_sign(& mut iter), None);
    }
}
#[cfg(test)]
mod tests_llm_16_22 {
    use std::borrow::Cow;
    #[test]
    fn test_decode_utf8_lossy() {
        let _rug_st_tests_llm_16_22_rrrruuuugggg_test_decode_utf8_lossy = 0;
        let rug_fuzz_0 = 72;
        let rug_fuzz_1 = 227;
        let borrowed_input: Vec<u8> = vec![rug_fuzz_0, 101, 108, 108, 111];
        let borrowed_cow: Cow<'_, [u8]> = Cow::Borrowed(&borrowed_input);
        let borrowed_result: Cow<'_, str> = super::decode_utf8_lossy(borrowed_cow);
        debug_assert_eq!(borrowed_result, "Hello");
        let owned_input: Vec<u8> = vec![
            rug_fuzz_1, 129, 130, 227, 129, 132, 227, 129, 139
        ];
        let owned_cow: Cow<'_, [u8]> = Cow::Owned(owned_input);
        let owned_result: Cow<'_, str> = super::decode_utf8_lossy(owned_cow);
        debug_assert_eq!(owned_result, "あいこ");
        let _rug_ed_tests_llm_16_22_rrrruuuugggg_test_decode_utf8_lossy = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_23 {
    use super::*;
    use crate::*;
    #[test]
    fn test_percent_decode() {
        let _rug_st_tests_llm_16_23_rrrruuuugggg_test_percent_decode = 0;
        let rug_fuzz_0 = b"foo%20bar%3f";
        let rug_fuzz_1 = "foo bar?";
        let input = rug_fuzz_0;
        let expected_output = rug_fuzz_1;
        let decoded = percent_decode(input).decode_utf8().unwrap();
        debug_assert_eq!(decoded, expected_output);
        let _rug_ed_tests_llm_16_23_rrrruuuugggg_test_percent_decode = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_24 {
    use super::*;
    use crate::*;
    use std::borrow::Cow;
    #[test]
    fn test_percent_decode_str() {
        let _rug_st_tests_llm_16_24_rrrruuuugggg_test_percent_decode_str = 0;
        let rug_fuzz_0 = "Hello%20World";
        let rug_fuzz_1 = "Hello%2BWorld";
        debug_assert_eq!(
            percent_decode_str(rug_fuzz_0).collect:: < Vec < u8 > > (), b"Hello World"
        );
        debug_assert_eq!(
            percent_decode_str(rug_fuzz_1).collect:: < Vec < u8 > > (), b"Hello+World"
        );
        let _rug_ed_tests_llm_16_24_rrrruuuugggg_test_percent_decode_str = 0;
    }
    #[test]
    #[cfg(feature = "std")]
    fn test_percent_decode_str_if_any() {
        let _rug_st_tests_llm_16_24_rrrruuuugggg_test_percent_decode_str_if_any = 0;
        let rug_fuzz_0 = "Hello%20World";
        let percent_decoded = percent_decode_str(rug_fuzz_0);
        debug_assert_eq!(percent_decoded.if_any(), None);
        let _rug_ed_tests_llm_16_24_rrrruuuugggg_test_percent_decode_str_if_any = 0;
    }
    #[test]
    #[cfg(feature = "std")]
    fn test_percent_decode_str_decode_utf8() {
        let _rug_st_tests_llm_16_24_rrrruuuugggg_test_percent_decode_str_decode_utf8 = 0;
        let rug_fuzz_0 = "Hello%20World";
        let rug_fuzz_1 = "Hello%2BWorld";
        let percent_decoded = percent_decode_str(rug_fuzz_0);
        debug_assert_eq!(
            percent_decoded.decode_utf8(), Ok(Cow::Borrowed("Hello World"))
        );
        let percent_decoded = percent_decode_str(rug_fuzz_1);
        debug_assert_eq!(
            percent_decoded.decode_utf8(), Ok(Cow::Borrowed("Hello+World"))
        );
        let _rug_ed_tests_llm_16_24_rrrruuuugggg_test_percent_decode_str_decode_utf8 = 0;
    }
    #[test]
    #[cfg(feature = "std")]
    fn test_percent_decode_str_decode_utf8_lossy() {
        let _rug_st_tests_llm_16_24_rrrruuuugggg_test_percent_decode_str_decode_utf8_lossy = 0;
        let rug_fuzz_0 = "Hello%20World";
        let rug_fuzz_1 = "Hello%2BWorld";
        let percent_decoded = percent_decode_str(rug_fuzz_0);
        debug_assert_eq!(
            percent_decoded.decode_utf8_lossy(), Cow::Borrowed("Hello World")
        );
        let percent_decoded = percent_decode_str(rug_fuzz_1);
        debug_assert_eq!(
            percent_decoded.decode_utf8_lossy(), Cow::Borrowed("Hello+World")
        );
        let _rug_ed_tests_llm_16_24_rrrruuuugggg_test_percent_decode_str_decode_utf8_lossy = 0;
    }
}
#[cfg(test)]
mod tests_rug_1 {
    use super::*;
    use std::mem;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_1_rrrruuuugggg_test_rug = 0;
        _static_assert();
        let _rug_ed_tests_rug_1_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_2 {
    use super::*;
    #[test]
    fn test_percent_encode_byte() {
        let _rug_st_tests_rug_2_rrrruuuugggg_test_percent_encode_byte = 0;
        let rug_fuzz_0 = 97;
        let p0: u8 = rug_fuzz_0;
        percent_encode_byte(p0);
        let _rug_ed_tests_rug_2_rrrruuuugggg_test_percent_encode_byte = 0;
    }
}
#[cfg(test)]
mod tests_rug_3 {
    use super::*;
    use crate::{percent_encode, AsciiSet, NON_ALPHANUMERIC};
    #[test]
    fn test_percent_encode() {
        let _rug_st_tests_rug_3_rrrruuuugggg_test_percent_encode = 0;
        let rug_fuzz_0 = b"foo bar?";
        let p0: &[u8] = rug_fuzz_0;
        let p1: &'static AsciiSet = &NON_ALPHANUMERIC;
        percent_encode(p0, p1);
        let _rug_ed_tests_rug_3_rrrruuuugggg_test_percent_encode = 0;
    }
}
#[cfg(test)]
mod tests_rug_4 {
    use super::*;
    use crate::{utf8_percent_encode, NON_ALPHANUMERIC, AsciiSet};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_4_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "foo bar?";
        let p0: &str = rug_fuzz_0;
        let p1: &'static AsciiSet = &NON_ALPHANUMERIC;
        crate::utf8_percent_encode(&p0, p1);
        let _rug_ed_tests_rug_4_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_5 {
    use super::*;
    use crate::AsciiSet;
    use crate::CONTROLS;
    #[test]
    fn test_ascii_set_contains() {
        let _rug_st_tests_rug_5_rrrruuuugggg_test_ascii_set_contains = 0;
        let rug_fuzz_0 = b' ';
        let rug_fuzz_1 = b'"';
        let rug_fuzz_2 = b'<';
        let rug_fuzz_3 = b'>';
        let rug_fuzz_4 = b'`';
        let rug_fuzz_5 = 32;
        let mut v1 = CONTROLS
            .add(rug_fuzz_0)
            .add(rug_fuzz_1)
            .add(rug_fuzz_2)
            .add(rug_fuzz_3)
            .add(rug_fuzz_4);
        let mut byte: u8 = rug_fuzz_5;
        AsciiSet::contains(&v1, byte);
        let _rug_ed_tests_rug_5_rrrruuuugggg_test_ascii_set_contains = 0;
    }
}
