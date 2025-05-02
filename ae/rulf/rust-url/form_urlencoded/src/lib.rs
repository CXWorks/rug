//! Parser and serializer for the [`application/x-www-form-urlencoded` syntax](
//! http://url.spec.whatwg.org/#application/x-www-form-urlencoded),
//! as used by HTML forms.
//!
//! Converts between a string (such as an URL’s query string)
//! and a sequence of (name, value) pairs.
#[macro_use]
extern crate matches;
use percent_encoding::{percent_decode, percent_encode_byte};
use std::borrow::{Borrow, Cow};
use std::str;
/// Convert a byte string in the `application/x-www-form-urlencoded` syntax
/// into a iterator of (name, value) pairs.
///
/// Use `parse(input.as_bytes())` to parse a `&str` string.
///
/// The names and values are percent-decoded. For instance, `%23first=%25try%25` will be
/// converted to `[("#first", "%try%")]`.
#[inline]
pub fn parse(input: &[u8]) -> Parse<'_> {
    Parse { input }
}
/// The return type of `parse()`.
#[derive(Copy, Clone)]
pub struct Parse<'a> {
    input: &'a [u8],
}
impl<'a> Iterator for Parse<'a> {
    type Item = (Cow<'a, str>, Cow<'a, str>);
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.input.is_empty() {
                return None;
            }
            let mut split2 = self.input.splitn(2, |&b| b == b'&');
            let sequence = split2.next().unwrap();
            self.input = split2.next().unwrap_or(&[][..]);
            if sequence.is_empty() {
                continue;
            }
            let mut split2 = sequence.splitn(2, |&b| b == b'=');
            let name = split2.next().unwrap();
            let value = split2.next().unwrap_or(&[][..]);
            return Some((decode(name), decode(value)));
        }
    }
}
fn decode(input: &[u8]) -> Cow<'_, str> {
    let replaced = replace_plus(input);
    decode_utf8_lossy(
        match percent_decode(&replaced).into() {
            Cow::Owned(vec) => Cow::Owned(vec),
            Cow::Borrowed(_) => replaced,
        },
    )
}
/// Replace b'+' with b' '
fn replace_plus(input: &[u8]) -> Cow<'_, [u8]> {
    match input.iter().position(|&b| b == b'+') {
        None => Cow::Borrowed(input),
        Some(first_position) => {
            let mut replaced = input.to_owned();
            replaced[first_position] = b' ';
            for byte in &mut replaced[first_position + 1..] {
                if *byte == b'+' {
                    *byte = b' ';
                }
            }
            Cow::Owned(replaced)
        }
    }
}
impl<'a> Parse<'a> {
    /// Return a new iterator that yields pairs of `String` instead of pairs of `Cow<str>`.
    pub fn into_owned(self) -> ParseIntoOwned<'a> {
        ParseIntoOwned { inner: self }
    }
}
/// Like `Parse`, but yields pairs of `String` instead of pairs of `Cow<str>`.
pub struct ParseIntoOwned<'a> {
    inner: Parse<'a>,
}
impl<'a> Iterator for ParseIntoOwned<'a> {
    type Item = (String, String);
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(k, v)| (k.into_owned(), v.into_owned()))
    }
}
/// The [`application/x-www-form-urlencoded` byte serializer](
/// https://url.spec.whatwg.org/#concept-urlencoded-byte-serializer).
///
/// Return an iterator of `&str` slices.
pub fn byte_serialize(input: &[u8]) -> ByteSerialize<'_> {
    ByteSerialize { bytes: input }
}
/// Return value of `byte_serialize()`.
#[derive(Debug)]
pub struct ByteSerialize<'a> {
    bytes: &'a [u8],
}
fn byte_serialized_unchanged(byte: u8) -> bool {
    matches!(
        byte, b'*' | b'-' | b'.' | b'0'..= b'9' | b'A'..= b'Z' | b'_' | b'a'..= b'z'
    )
}
impl<'a> Iterator for ByteSerialize<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        if let Some((&first, tail)) = self.bytes.split_first() {
            if !byte_serialized_unchanged(first) {
                self.bytes = tail;
                return Some(
                    if first == b' ' { "+" } else { percent_encode_byte(first) },
                );
            }
            let position = tail.iter().position(|&b| !byte_serialized_unchanged(b));
            let (unchanged_slice, remaining) = match position {
                Some(i) => self.bytes.split_at(1 + i),
                None => (self.bytes, &[][..]),
            };
            self.bytes = remaining;
            Some(unsafe { str::from_utf8_unchecked(unchanged_slice) })
        } else {
            None
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.bytes.is_empty() { (0, Some(0)) } else { (1, Some(self.bytes.len())) }
    }
}
/// The [`application/x-www-form-urlencoded` serializer](
/// https://url.spec.whatwg.org/#concept-urlencoded-serializer).
pub struct Serializer<'a, T: Target> {
    target: Option<T>,
    start_position: usize,
    encoding: EncodingOverride<'a>,
}
pub trait Target {
    fn as_mut_string(&mut self) -> &mut String;
    fn finish(self) -> Self::Finished;
    type Finished;
}
impl Target for String {
    fn as_mut_string(&mut self) -> &mut String {
        self
    }
    fn finish(self) -> Self {
        self
    }
    type Finished = Self;
}
impl<'a> Target for &'a mut String {
    fn as_mut_string(&mut self) -> &mut String {
        &mut **self
    }
    fn finish(self) -> Self {
        self
    }
    type Finished = Self;
}
impl<'a, T: Target> Serializer<'a, T> {
    /// Create a new `application/x-www-form-urlencoded` serializer for the given target.
    ///
    /// If the target is non-empty,
    /// its content is assumed to already be in `application/x-www-form-urlencoded` syntax.
    pub fn new(target: T) -> Self {
        Self::for_suffix(target, 0)
    }
    /// Create a new `application/x-www-form-urlencoded` serializer
    /// for a suffix of the given target.
    ///
    /// If that suffix is non-empty,
    /// its content is assumed to already be in `application/x-www-form-urlencoded` syntax.
    pub fn for_suffix(mut target: T, start_position: usize) -> Self {
        if target.as_mut_string().len() < start_position {
            panic!(
                "invalid length {} for target of length {}", start_position, target
                .as_mut_string().len()
            );
        }
        Serializer {
            target: Some(target),
            start_position,
            encoding: None,
        }
    }
    /// Remove any existing name/value pair.
    ///
    /// Panics if called after `.finish()`.
    pub fn clear(&mut self) -> &mut Self {
        string(&mut self.target).truncate(self.start_position);
        self
    }
    /// Set the character encoding to be used for names and values before percent-encoding.
    pub fn encoding_override(&mut self, new: EncodingOverride<'a>) -> &mut Self {
        self.encoding = new;
        self
    }
    /// Serialize and append a name/value pair.
    ///
    /// Panics if called after `.finish()`.
    pub fn append_pair(&mut self, name: &str, value: &str) -> &mut Self {
        append_pair(
            string(&mut self.target),
            self.start_position,
            self.encoding,
            name,
            value,
        );
        self
    }
    /// Serialize and append a name of parameter without any value.
    ///
    /// Panics if called after `.finish()`.
    pub fn append_key_only(&mut self, name: &str) -> &mut Self {
        append_key_only(
            string(&mut self.target),
            self.start_position,
            self.encoding,
            name,
        );
        self
    }
    /// Serialize and append a number of name/value pairs.
    ///
    /// This simply calls `append_pair` repeatedly.
    /// This can be more convenient, so the user doesn’t need to introduce a block
    /// to limit the scope of `Serializer`’s borrow of its string.
    ///
    /// Panics if called after `.finish()`.
    pub fn extend_pairs<I, K, V>(&mut self, iter: I) -> &mut Self
    where
        I: IntoIterator,
        I::Item: Borrow<(K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        {
            let string = string(&mut self.target);
            for pair in iter {
                let &(ref k, ref v) = pair.borrow();
                append_pair(
                    string,
                    self.start_position,
                    self.encoding,
                    k.as_ref(),
                    v.as_ref(),
                );
            }
        }
        self
    }
    /// Serialize and append a number of names without values.
    ///
    /// This simply calls `append_key_only` repeatedly.
    /// This can be more convenient, so the user doesn’t need to introduce a block
    /// to limit the scope of `Serializer`’s borrow of its string.
    ///
    /// Panics if called after `.finish()`.
    pub fn extend_keys_only<I, K>(&mut self, iter: I) -> &mut Self
    where
        I: IntoIterator,
        I::Item: Borrow<K>,
        K: AsRef<str>,
    {
        {
            let string = string(&mut self.target);
            for key in iter {
                let k = key.borrow().as_ref();
                append_key_only(string, self.start_position, self.encoding, k);
            }
        }
        self
    }
    /// If this serializer was constructed with a string, take and return that string.
    ///
    /// ```rust
    /// use form_urlencoded;
    /// let encoded: String = form_urlencoded::Serializer::new(String::new())
    ///     .append_pair("foo", "bar & baz")
    ///     .append_pair("saison", "Été+hiver")
    ///     .finish();
    /// assert_eq!(encoded, "foo=bar+%26+baz&saison=%C3%89t%C3%A9%2Bhiver");
    /// ```
    ///
    /// Panics if called more than once.
    pub fn finish(&mut self) -> T::Finished {
        self.target
            .take()
            .expect("url::form_urlencoded::Serializer double finish")
            .finish()
    }
}
fn append_separator_if_needed(string: &mut String, start_position: usize) {
    if string.len() > start_position {
        string.push('&')
    }
}
fn string<T: Target>(target: &mut Option<T>) -> &mut String {
    target.as_mut().expect("url::form_urlencoded::Serializer finished").as_mut_string()
}
fn append_pair(
    string: &mut String,
    start_position: usize,
    encoding: EncodingOverride<'_>,
    name: &str,
    value: &str,
) {
    append_separator_if_needed(string, start_position);
    append_encoded(name, string, encoding);
    string.push('=');
    append_encoded(value, string, encoding);
}
fn append_key_only(
    string: &mut String,
    start_position: usize,
    encoding: EncodingOverride,
    name: &str,
) {
    append_separator_if_needed(string, start_position);
    append_encoded(name, string, encoding);
}
fn append_encoded(s: &str, string: &mut String, encoding: EncodingOverride<'_>) {
    string.extend(byte_serialize(&encode(encoding, s)))
}
pub(crate) fn encode<'a>(
    encoding_override: EncodingOverride<'_>,
    input: &'a str,
) -> Cow<'a, [u8]> {
    if let Some(o) = encoding_override {
        return o(input);
    }
    input.as_bytes().into()
}
pub(crate) fn decode_utf8_lossy(input: Cow<'_, [u8]>) -> Cow<'_, str> {
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
pub type EncodingOverride<'a> = Option<&'a dyn Fn(&str) -> Cow<'_, [u8]>>;
#[cfg(test)]
mod tests_llm_16_1 {
    use super::*;
    use crate::*;
    use std::string::String;
    #[test]
    fn test_as_mut_string() {
        let _rug_st_tests_llm_16_1_rrrruuuugggg_test_as_mut_string = 0;
        let rug_fuzz_0 = "example";
        let mut input: String = String::from(rug_fuzz_0);
        let result = input.as_mut_string();
        debug_assert_eq!(result, & mut input);
        let _rug_ed_tests_llm_16_1_rrrruuuugggg_test_as_mut_string = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_2 {
    use super::*;
    use crate::*;
    use std::string::String;
    use crate::Target;
    #[test]
    fn test_finish() {
        let _rug_st_tests_llm_16_2_rrrruuuugggg_test_finish = 0;
        let mut string = String::new();
        let result = string.finish();
        debug_assert_eq!(result, string);
        let _rug_ed_tests_llm_16_2_rrrruuuugggg_test_finish = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_3 {
    use super::*;
    use crate::*;
    #[test]
    fn test_next() {
        let _rug_st_tests_llm_16_3_rrrruuuugggg_test_next = 0;
        let rug_fuzz_0 = b'h';
        let rug_fuzz_1 = b'e';
        let rug_fuzz_2 = b'l';
        let rug_fuzz_3 = b'l';
        let rug_fuzz_4 = b'o';
        let mut bytes = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3, rug_fuzz_4];
        let mut iter = ByteSerialize { bytes: &bytes[..] };
        debug_assert_eq!(iter.next(), Some("h"));
        debug_assert_eq!(iter.next(), Some("e"));
        debug_assert_eq!(iter.next(), Some("l"));
        debug_assert_eq!(iter.next(), Some("l"));
        debug_assert_eq!(iter.next(), Some("o"));
        debug_assert_eq!(iter.next(), None);
        let _rug_ed_tests_llm_16_3_rrrruuuugggg_test_next = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_4 {
    use super::*;
    use crate::*;
    use std::fmt::Debug;
    use std::iter::Iterator;
    #[test]
    fn test_size_hint_empty_bytes() {
        let bytes: &[u8] = &[];
        let byte_serialize = ByteSerialize { bytes };
        let (lower, upper) = byte_serialize.size_hint();
        assert_eq!(lower, 0);
        assert_eq!(upper, Some(0));
    }
    #[test]
    fn test_size_hint_non_empty_bytes() {
        let bytes: &[u8] = &[1, 2, 3];
        let byte_serialize = ByteSerialize { bytes };
        let (lower, upper) = byte_serialize.size_hint();
        assert_eq!(lower, 1);
        assert_eq!(upper, Some(3));
    }
    fn byte_serialized_unchanged(_byte: u8) -> bool {
        true
    }
    fn percent_encode_byte(_byte: u8) -> &str {
        ""
    }
}
#[cfg(test)]
mod tests_llm_16_5 {
    use super::*;
    use crate::*;
    use std::borrow::Cow;
    fn decode(name: &[u8]) -> Cow<str> {
        let _rug_st_tests_llm_16_5_rrrruuuugggg_decode = 0;
        let _rug_ed_tests_llm_16_5_rrrruuuugggg_decode = 0;
    }
    #[test]
    fn test_next() {
        let _rug_st_tests_llm_16_5_rrrruuuugggg_test_next = 0;
        let mut input: &[u8] = &[];
        let mut parse = Parse { input };
        let _rug_ed_tests_llm_16_5_rrrruuuugggg_test_next = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_6 {
    use super::*;
    use crate::*;
    use std::borrow::Cow;
    #[test]
    fn test_next() {
        let _rug_st_tests_llm_16_6_rrrruuuugggg_test_next = 0;
        let input: &[u8] = &[];
        let parse = Parse { input };
        let parse_into_owned = parse.into_owned();
        let result = parse_into_owned.next();
        debug_assert_eq!(result);
        let _rug_ed_tests_llm_16_6_rrrruuuugggg_test_next = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_7 {
    use std::string::String;
    use crate::Target;
    #[test]
    fn test_as_mut_string() {
        let _rug_st_tests_llm_16_7_rrrruuuugggg_test_as_mut_string = 0;
        let rug_fuzz_0 = "hello";
        let mut s = String::from(rug_fuzz_0);
        let as_mut_string = s.as_mut_string();
        debug_assert_eq!(as_mut_string, & mut s);
        let _rug_ed_tests_llm_16_7_rrrruuuugggg_test_as_mut_string = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_8 {
    use crate::Target;
    #[test]
    fn test_finish() {
        let _rug_st_tests_llm_16_8_rrrruuuugggg_test_finish = 0;
        let rug_fuzz_0 = "test string";
        let mut string = String::from(rug_fuzz_0);
        let result = string.finish();
        debug_assert_eq!(result, String::from("test string"));
        let _rug_ed_tests_llm_16_8_rrrruuuugggg_test_finish = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_9 {
    use super::*;
    use crate::*;
    use std::borrow::Cow;
    #[test]
    fn test_into_owned() {
        let _rug_st_tests_llm_16_9_rrrruuuugggg_test_into_owned = 0;
        let rug_fuzz_0 = b"key1=value1&key2=value2&key3=value3";
        let rug_fuzz_1 = "key1";
        let rug_fuzz_2 = "value1";
        let input: &[u8] = rug_fuzz_0;
        let parse: Parse = Parse { input };
        let parse_into_owned: ParseIntoOwned = parse.into_owned();
        let expected: Vec<(String, String)> = vec![
            (String::from(rug_fuzz_1), String::from(rug_fuzz_2)), (String::from("key2"),
            String::from("value2")), (String::from("key3"), String::from("value3"))
        ];
        let result: Vec<(String, String)> = parse_into_owned.collect();
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_9_rrrruuuugggg_test_into_owned = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_10 {
    use super::*;
    use crate::*;
    #[test]
    #[should_panic]
    fn test_append_key_only_panics_after_finish() {
        let _rug_st_tests_llm_16_10_rrrruuuugggg_test_append_key_only_panics_after_finish = 0;
        let rug_fuzz_0 = "name";
        let mut serializer = Serializer::new(String::new());
        serializer.finish();
        serializer.append_key_only(rug_fuzz_0);
        let _rug_ed_tests_llm_16_10_rrrruuuugggg_test_append_key_only_panics_after_finish = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_11 {
    use crate::crate::Serializer;
    #[test]
    #[should_panic(expected = "url::crate::Serializer double finish")]
    fn test_append_pair() {
        let _rug_st_tests_llm_16_11_rrrruuuugggg_test_append_pair = 0;
        let rug_fuzz_0 = "name";
        let rug_fuzz_1 = "value";
        let rug_fuzz_2 = "name2";
        let rug_fuzz_3 = "value2";
        let mut target = String::new();
        let mut serializer = Serializer::new(target);
        serializer.append_pair(rug_fuzz_0, rug_fuzz_1);
        serializer.finish();
        serializer.append_pair(rug_fuzz_2, rug_fuzz_3);
        let _rug_ed_tests_llm_16_11_rrrruuuugggg_test_append_pair = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_12 {
    use crate::crate::Serializer;
    use crate::crate::Target;
    #[test]
    fn test_clear() {
        let _rug_st_tests_llm_16_12_rrrruuuugggg_test_clear = 0;
        let rug_fuzz_0 = "foo=bar&baz=qux";
        let rug_fuzz_1 = "";
        let mut target = String::new();
        target.push_str(rug_fuzz_0);
        let mut serializer = Serializer::new(target);
        serializer.clear();
        let expected = rug_fuzz_1;
        debug_assert_eq!(serializer.finish(), expected);
        let _rug_ed_tests_llm_16_12_rrrruuuugggg_test_clear = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_13 {
    use super::*;
    use crate::*;
    use crate::Serializer;
    #[test]
    fn test_encoding_override() {
        let _rug_st_tests_llm_16_13_rrrruuuugggg_test_encoding_override = 0;
        let mut serializer = Serializer::new(String::new());
        let encoding_override = EncodingOverride::new();
        let result = serializer.encoding_override(encoding_override);
        let _rug_ed_tests_llm_16_13_rrrruuuugggg_test_encoding_override = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_14 {
    use super::*;
    use crate::*;
    struct MockTarget {
        string: String,
    }
    impl Target for MockTarget {
        fn as_mut_string(&mut self) -> &mut String {
            &mut self.string
        }
        fn finish(self) -> Self::Finished {
            self.string
        }
        type Finished = String;
    }
    #[test]
    fn test_extend_keys_only() {
        let _rug_st_tests_llm_16_14_rrrruuuugggg_test_extend_keys_only = 0;
        let rug_fuzz_0 = "key1";
        let mut serializer = Serializer::new(MockTarget {
            string: String::new(),
        });
        serializer.extend_keys_only(vec![rug_fuzz_0, "key2", "key3"]);
        debug_assert_eq!(serializer.finish(), "key1=&key2=&key3=");
        let _rug_ed_tests_llm_16_14_rrrruuuugggg_test_extend_keys_only = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_15 {
    use crate::crate::Serializer;
    #[test]
    fn test_extend_pairs() {
        let _rug_st_tests_llm_16_15_rrrruuuugggg_test_extend_pairs = 0;
        let rug_fuzz_0 = "name1";
        let rug_fuzz_1 = "value1";
        let mut serializer = Serializer::new(String::new());
        serializer.extend_pairs(vec![(rug_fuzz_0, rug_fuzz_1), ("name2", "value2")]);
        let encoded = serializer.finish();
        debug_assert_eq!(encoded, "name1=value1&name2=value2");
        let _rug_ed_tests_llm_16_15_rrrruuuugggg_test_extend_pairs = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_16 {
    use super::*;
    use crate::*;
    use crate::{Target, Serializer};
    fn create_serializer() -> Serializer<String> {
        Serializer::new(String::new())
    }
    #[test]
    fn test_finish() {
        let encoded: String = create_serializer()
            .append_pair("foo", "bar & baz")
            .append_pair("saison", "Été+hiver")
            .finish();
        assert_eq!(encoded, "foo=bar+%26+baz&saison=%C3%89t%C3%A9%2Bhiver");
    }
}
#[cfg(test)]
mod tests_llm_16_17 {
    use super::*;
    use crate::*;
    use std::borrow::Borrow;
    use crate::Serializer;
    #[test]
    fn test_for_suffix_invalid_length() {
        let _rug_st_tests_llm_16_17_rrrruuuugggg_test_for_suffix_invalid_length = 0;
        let rug_fuzz_0 = "example";
        let rug_fuzz_1 = 8;
        let target = String::from(rug_fuzz_0);
        let start_position = rug_fuzz_1;
        let result = std::panic::catch_unwind(|| {
            Serializer::<'_, String>::for_suffix(target, start_position);
        });
        debug_assert!(result.is_err());
        let _rug_ed_tests_llm_16_17_rrrruuuugggg_test_for_suffix_invalid_length = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_18 {
    use crate::Serializer;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_18_rrrruuuugggg_test_new = 0;
        let target = String::new();
        let serializer = Serializer::new(target);
        let _rug_ed_tests_llm_16_18_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_19 {
    use crate::crate::{append_encoded, encode, EncodingOverride, byte_serialize};
    use std::string::String;
    use std::str::FromStr;
    use std::borrow::Cow;
    #[test]
    fn test_append_encoded() {
        let _rug_st_tests_llm_16_19_rrrruuuugggg_test_append_encoded = 0;
        let rug_fuzz_0 = "Hello World";
        let rug_fuzz_1 = "Hello%20World";
        let mut string = String::new();
        let s = rug_fuzz_0;
        let encoding: EncodingOverride<'_> = EncodingOverride::No;
        append_encoded(s, &mut string, encoding);
        let expected = String::from(rug_fuzz_1);
        debug_assert_eq!(string, expected);
        let _rug_ed_tests_llm_16_19_rrrruuuugggg_test_append_encoded = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_20 {
    use super::*;
    use crate::*;
    use crate::Target;
    #[test]
    fn test_append_key_only() {
        let _rug_st_tests_llm_16_20_rrrruuuugggg_test_append_key_only = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = "test_name";
        let rug_fuzz_2 = "test_name";
        let mut string: String = String::new();
        let start_position: usize = rug_fuzz_0;
        let encoding: EncodingOverride = EncodingOverride::Default;
        let name: &str = rug_fuzz_1;
        append_key_only(&mut string, start_position, encoding, name);
        let expected_output: String = rug_fuzz_2.to_string();
        debug_assert_eq!(string, expected_output);
        let _rug_ed_tests_llm_16_20_rrrruuuugggg_test_append_key_only = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_21 {
    use crate::crate::append_pair;
    use crate::crate::crate::EncodingOverride;
    use std::string::String;
    #[test]
    fn test_append_pair() {
        let _rug_st_tests_llm_16_21_rrrruuuugggg_test_append_pair = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = "name";
        let rug_fuzz_2 = "value";
        let rug_fuzz_3 = "name=value";
        let mut string = String::new();
        let start_position = rug_fuzz_0;
        let encoding = EncodingOverride::UTF8;
        let name = rug_fuzz_1;
        let value = rug_fuzz_2;
        append_pair(&mut string, start_position, encoding, name, value);
        debug_assert_eq!(rug_fuzz_3, string);
        let _rug_ed_tests_llm_16_21_rrrruuuugggg_test_append_pair = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_23 {
    use crate::Target;
    #[test]
    fn test_append_separator_if_needed() {
        let _rug_st_tests_llm_16_23_rrrruuuugggg_test_append_separator_if_needed = 0;
        let rug_fuzz_0 = "abc";
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = "xyz";
        let rug_fuzz_3 = 3;
        let mut string = String::from(rug_fuzz_0);
        let start_position = rug_fuzz_1;
        crate::append_separator_if_needed(&mut string, start_position);
        debug_assert_eq!(string, "abc&");
        let mut string = String::from(rug_fuzz_2);
        let start_position = rug_fuzz_3;
        crate::append_separator_if_needed(&mut string, start_position);
        debug_assert_eq!(string, "xyz");
        let _rug_ed_tests_llm_16_23_rrrruuuugggg_test_append_separator_if_needed = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_24 {
    use super::*;
    use crate::*;
    #[test]
    fn test_byte_serialize() {
        let _rug_st_tests_llm_16_24_rrrruuuugggg_test_byte_serialize = 0;
        let rug_fuzz_0 = 97;
        let rug_fuzz_1 = 98;
        let rug_fuzz_2 = 99;
        let rug_fuzz_3 = 32;
        let rug_fuzz_4 = 100;
        let rug_fuzz_5 = 101;
        let rug_fuzz_6 = 102;
        let rug_fuzz_7 = "a";
        let rug_fuzz_8 = "b";
        let rug_fuzz_9 = "c";
        let rug_fuzz_10 = "+";
        let rug_fuzz_11 = "d";
        let rug_fuzz_12 = "e";
        let rug_fuzz_13 = "f";
        let input = &[
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
            rug_fuzz_6,
        ];
        let expected_output = [
            rug_fuzz_7,
            rug_fuzz_8,
            rug_fuzz_9,
            rug_fuzz_10,
            rug_fuzz_11,
            rug_fuzz_12,
            rug_fuzz_13,
        ];
        let result: Vec<&str> = byte_serialize(input).collect();
        debug_assert_eq!(result, expected_output);
        let _rug_ed_tests_llm_16_24_rrrruuuugggg_test_byte_serialize = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_25 {
    use super::*;
    use crate::*;
    #[test]
    fn test_byte_serialized_unchanged() {
        let _rug_st_tests_llm_16_25_rrrruuuugggg_test_byte_serialized_unchanged = 0;
        let rug_fuzz_0 = b'*';
        let rug_fuzz_1 = b'-';
        let rug_fuzz_2 = b'.';
        let rug_fuzz_3 = b'0';
        let rug_fuzz_4 = b'9';
        let rug_fuzz_5 = b'A';
        let rug_fuzz_6 = b'Z';
        let rug_fuzz_7 = b'_';
        let rug_fuzz_8 = b'a';
        let rug_fuzz_9 = b'z';
        let rug_fuzz_10 = b'@';
        let rug_fuzz_11 = b' ';
        let rug_fuzz_12 = b'!';
        let rug_fuzz_13 = b':';
        debug_assert_eq!(byte_serialized_unchanged(rug_fuzz_0), true);
        debug_assert_eq!(byte_serialized_unchanged(rug_fuzz_1), true);
        debug_assert_eq!(byte_serialized_unchanged(rug_fuzz_2), true);
        debug_assert_eq!(byte_serialized_unchanged(rug_fuzz_3), true);
        debug_assert_eq!(byte_serialized_unchanged(rug_fuzz_4), true);
        debug_assert_eq!(byte_serialized_unchanged(rug_fuzz_5), true);
        debug_assert_eq!(byte_serialized_unchanged(rug_fuzz_6), true);
        debug_assert_eq!(byte_serialized_unchanged(rug_fuzz_7), true);
        debug_assert_eq!(byte_serialized_unchanged(rug_fuzz_8), true);
        debug_assert_eq!(byte_serialized_unchanged(rug_fuzz_9), true);
        debug_assert_eq!(byte_serialized_unchanged(rug_fuzz_10), false);
        debug_assert_eq!(byte_serialized_unchanged(rug_fuzz_11), false);
        debug_assert_eq!(byte_serialized_unchanged(rug_fuzz_12), false);
        debug_assert_eq!(byte_serialized_unchanged(rug_fuzz_13), false);
        let _rug_ed_tests_llm_16_25_rrrruuuugggg_test_byte_serialized_unchanged = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_26 {
    use std::borrow::Cow;
    use std::str::FromStr;
    use std::string::ToString;
    use percent_encoding::percent_decode;
    use encoding::all::UTF_8;
    use encoding::{DecoderTrap, EncoderTrap, Encoding};
    use encoding::all::ASCII;
    use crate::byte_classes::SOFT_EOF;
    use crate::byte_classes::SOFTEOF_ENCODED;
    use crate::byte_classes::NEWLINE_ENCODED;
    use crate::byte_classes::EFFICIENT_AMP_ENCODED;
    use crate::byte_classes::EFFICIENT_ASTERISK_ENCODED;
    use crate::byte_classes::UTF8_ENCODED_FAST_PATH;
    use crate::byte_classes::UTF8_ALLOW_RANGE;
    use crate::byte_classes::FORM_URLENCODED_ENCODED;
    use crate::byte_classes::LEFT_BRACKET_ENCODED;
    use crate::byte_classes::RIGHT_BRACKET_ENCODED;
    use crate::byte_classes::LEFT_BRACE_ENCODED;
    use crate::byte_classes::RIGHT_BRACE_ENCODED;
    use crate::byte_classes::OG_ENCODED_FAST_PATH;
    use crate::byte_classes::AMP_ENCODED;
    use crate::byte_classes::TRACK_SPACE_ENCODED;
    use crate::byte_classes::TRACK_PLUS_ENCODED;
    use crate::byte_classes::TRACK_PERCENT_ENCODED;
    use crate::byte_classes::TRACK_SLASH_ENCODED;
    use crate::byte_classes::AMP_ENCODED_FAST_PATH;
    use crate::percent_decode;
    use crate::replace_plus;
    use crate::replace_last;
    use crate::replace_multi;
    use crate::replace_multi_after_plus;
    use crate::replace_plus_after_multi;
    use crate::replace_multi_after_plus_appears;
    use crate::replace_multi_after_plus_appears_multi;
    use crate::replace_plus;
    use crate::trim_buffer;
    use crate::VALID_UTF8_CHARS;
    use crate::VALID_UTF8_CONTINUES;
    use crate::push_buffer;
    use crate::validate_utf8;
    use crate::form_urlencoded_slice;
    use crate::form_urlencoded_tuple;
    #[test]
    fn test_decode() {
        let _rug_st_tests_llm_16_26_rrrruuuugggg_test_decode = 0;
        let rug_fuzz_0 = "foo=bar%20baz&key=val%201%202%303";
        let input = rug_fuzz_0;
        debug_assert_eq!(decode(input.as_bytes()), "foo=bar baz&key=val 1 2 3");
        let _rug_ed_tests_llm_16_26_rrrruuuugggg_test_decode = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_27 {
    use crate::decode_utf8_lossy;
    use std::borrow::Cow;
    #[test]
    fn test_decode_utf8_lossy_borrowed() {
        let _rug_st_tests_llm_16_27_rrrruuuugggg_test_decode_utf8_lossy_borrowed = 0;
        let rug_fuzz_0 = b"hello";
        let rug_fuzz_1 = "hello";
        let input: Cow<[u8]> = Cow::Borrowed(rug_fuzz_0);
        let expected: Cow<str> = Cow::Borrowed(rug_fuzz_1);
        debug_assert_eq!(decode_utf8_lossy(input), expected);
        let _rug_ed_tests_llm_16_27_rrrruuuugggg_test_decode_utf8_lossy_borrowed = 0;
    }
    #[test]
    fn test_decode_utf8_lossy_owned_borrowed() {
        let _rug_st_tests_llm_16_27_rrrruuuugggg_test_decode_utf8_lossy_owned_borrowed = 0;
        let rug_fuzz_0 = 104;
        let rug_fuzz_1 = "hello";
        let input: Cow<[u8]> = Cow::Owned(vec![rug_fuzz_0, 101, 108, 108, 111]);
        let expected: Cow<str> = Cow::Owned(rug_fuzz_1.to_owned());
        debug_assert_eq!(decode_utf8_lossy(input), expected);
        let _rug_ed_tests_llm_16_27_rrrruuuugggg_test_decode_utf8_lossy_owned_borrowed = 0;
    }
    #[test]
    fn test_decode_utf8_lossy_owned_owned() {
        let _rug_st_tests_llm_16_27_rrrruuuugggg_test_decode_utf8_lossy_owned_owned = 0;
        let rug_fuzz_0 = 104;
        let rug_fuzz_1 = "hello�";
        let input: Cow<[u8]> = Cow::Owned(
            vec![rug_fuzz_0, 101, 108, 108, 111, 226, 128, 140],
        );
        let expected: Cow<str> = Cow::Owned(rug_fuzz_1.to_owned());
        debug_assert_eq!(decode_utf8_lossy(input), expected);
        let _rug_ed_tests_llm_16_27_rrrruuuugggg_test_decode_utf8_lossy_owned_owned = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_28 {
    use crate::crate::encode;
    use crate::crate::EncodingOverride;
    use std::borrow::Cow;
    #[test]
    fn test_encode_without_override() {
        let _rug_st_tests_llm_16_28_rrrruuuugggg_test_encode_without_override = 0;
        let rug_fuzz_0 = "param1=value1&param2=value2";
        let input = rug_fuzz_0;
        let expected = input.as_bytes().into();
        let encoding_override: Option<EncodingOverride<'_>> = None;
        let result = encode(encoding_override, input);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_28_rrrruuuugggg_test_encode_without_override = 0;
    }
    #[test]
    fn test_encode_with_override() {
        let _rug_st_tests_llm_16_28_rrrruuuugggg_test_encode_with_override = 0;
        let rug_fuzz_0 = "param1=value1&param2=value2";
        let input = rug_fuzz_0;
        let expected = input.as_bytes().into();
        let encoding_override: Option<EncodingOverride<'_>> = Some(|input: &str| {
            input.to_uppercase().into_bytes().into()
        });
        let result = encode(encoding_override, input);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_28_rrrruuuugggg_test_encode_with_override = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_29 {
    use super::*;
    use crate::*;
    use std::borrow::Cow;
    #[test]
    fn test_parse() {
        let _rug_st_tests_llm_16_29_rrrruuuugggg_test_parse = 0;
        let rug_fuzz_0 = b"key1=value1&key2=value2";
        let rug_fuzz_1 = "key1";
        let rug_fuzz_2 = "value1";
        let rug_fuzz_3 = b"key1=value1&key2=value2&key3=value3";
        let rug_fuzz_4 = "key1";
        let rug_fuzz_5 = "value1";
        let rug_fuzz_6 = b"key1=value1&key2&key3=value3";
        let rug_fuzz_7 = "key1";
        let rug_fuzz_8 = "value1";
        let input1 = rug_fuzz_0;
        let expected1: Vec<(&str, &str)> = vec![
            (rug_fuzz_1, rug_fuzz_2), ("key2", "value2")
        ];
        let result1: Vec<(Cow<str>, Cow<str>)> = parse(&input1).collect();
        let result1: Vec<(&str, &str)> = result1
            .iter()
            .map(|(name, value)| (name.as_ref(), value.as_ref()))
            .collect();
        debug_assert_eq!(result1, expected1);
        let input2 = rug_fuzz_3;
        let expected2: Vec<(&str, &str)> = vec![
            (rug_fuzz_4, rug_fuzz_5), ("key2", "value2"), ("key3", "value3")
        ];
        let result2: Vec<(Cow<str>, Cow<str>)> = parse(&input2).collect();
        let result2: Vec<(&str, &str)> = result2
            .iter()
            .map(|(name, value)| (name.as_ref(), value.as_ref()))
            .collect();
        debug_assert_eq!(result2, expected2);
        let input3 = rug_fuzz_6;
        let expected3: Vec<(&str, &str)> = vec![
            (rug_fuzz_7, rug_fuzz_8), ("key2", ""), ("key3", "value3")
        ];
        let result3: Vec<(Cow<str>, Cow<str>)> = parse(&input3).collect();
        let result3: Vec<(&str, &str)> = result3
            .iter()
            .map(|(name, value)| (name.as_ref(), value.as_ref()))
            .collect();
        debug_assert_eq!(result3, expected3);
        let _rug_ed_tests_llm_16_29_rrrruuuugggg_test_parse = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_30 {
    use std::borrow::Cow;
    use crate::replace_plus;
    #[test]
    fn test_replace_plus() {
        let _rug_st_tests_llm_16_30_rrrruuuugggg_test_replace_plus = 0;
        let rug_fuzz_0 = b"hello world";
        let rug_fuzz_1 = b"hello+world";
        let rug_fuzz_2 = b"hello++world";
        let rug_fuzz_3 = b"+hello+world";
        let rug_fuzz_4 = b"hello+world+";
        let rug_fuzz_5 = b"+hello+world+";
        let rug_fuzz_6 = b"hello+my+name+is";
        let rug_fuzz_7 = b"";
        debug_assert_eq!(replace_plus(rug_fuzz_0), Cow::Borrowed(b"hello world"));
        debug_assert_eq!(replace_plus(rug_fuzz_1), Cow::Borrowed(b"hello world"));
        debug_assert_eq!(replace_plus(rug_fuzz_2), Cow::Borrowed(b"hello  world"));
        debug_assert_eq!(replace_plus(rug_fuzz_3), Cow::Borrowed(b" hello world"));
        debug_assert_eq!(replace_plus(rug_fuzz_4), Cow::Borrowed(b"hello world "));
        debug_assert_eq!(replace_plus(rug_fuzz_5), Cow::Borrowed(b" hello world "));
        debug_assert_eq!(replace_plus(rug_fuzz_6), Cow::Borrowed(b"hello my name is"));
        debug_assert_eq!(replace_plus(rug_fuzz_7), Cow::Borrowed(b""));
        let _rug_ed_tests_llm_16_30_rrrruuuugggg_test_replace_plus = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_31 {
    use super::*;
    use crate::*;
    struct MockTarget(String);
    impl Target for MockTarget {
        fn as_mut_string(&mut self) -> &mut String {
            &mut self.0
        }
        fn finish(self) -> Self::Finished {
            self
        }
        type Finished = Self;
    }
    #[test]
    fn test_string() {
        let _rug_st_tests_llm_16_31_rrrruuuugggg_test_string = 0;
        let mut target = MockTarget(String::new());
        let result = string(&mut Some(&mut target));
        debug_assert_eq!(result, & mut String::new());
        let _rug_ed_tests_llm_16_31_rrrruuuugggg_test_string = 0;
    }
}
