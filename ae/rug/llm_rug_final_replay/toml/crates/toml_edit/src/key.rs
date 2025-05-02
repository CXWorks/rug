use std::borrow::Cow;
use std::str::FromStr;
use crate::encode::{to_string_repr, StringStyle};
use crate::parser;
use crate::parser::key::is_unquoted_char;
use crate::repr::{Decor, Repr};
use crate::InternalString;
/// Key as part of a Key/Value Pair or a table header.
///
/// # Examples
///
/// ```notrust
/// [dependencies."nom"]
/// version = "5.0"
/// 'literal key' = "nonsense"
/// "basic string key" = 42
/// ```
///
/// There are 3 types of keys:
///
/// 1. Bare keys (`version` and `dependencies`)
///
/// 2. Basic quoted keys (`"basic string key"` and `"nom"`)
///
/// 3. Literal quoted keys (`'literal key'`)
///
/// For details see [toml spec](https://github.com/toml-lang/toml/#keyvalue-pair).
///
/// To parse a key use `FromStr` trait implementation: `"string".parse::<Key>()`.
#[derive(Debug, Clone)]
pub struct Key {
    key: InternalString,
    pub(crate) repr: Option<Repr>,
    pub(crate) decor: Decor,
}
impl Key {
    /// Create a new table key
    pub fn new(key: impl Into<InternalString>) -> Self {
        Self {
            key: key.into(),
            repr: None,
            decor: Default::default(),
        }
    }
    /// Parse a TOML key expression
    ///
    /// Unlike `"".parse<Key>()`, this supports dotted keys.
    pub fn parse(repr: &str) -> Result<Vec<Self>, crate::TomlError> {
        Self::try_parse_path(repr)
    }
    pub(crate) fn with_repr_unchecked(mut self, repr: Repr) -> Self {
        self.repr = Some(repr);
        self
    }
    /// While creating the `Key`, add `Decor` to it
    pub fn with_decor(mut self, decor: Decor) -> Self {
        self.decor = decor;
        self
    }
    /// Access a mutable proxy for the `Key`.
    pub fn as_mut(&mut self) -> KeyMut<'_> {
        KeyMut { key: self }
    }
    /// Returns the parsed key value.
    pub fn get(&self) -> &str {
        &self.key
    }
    pub(crate) fn get_internal(&self) -> &InternalString {
        &self.key
    }
    /// Returns key raw representation, if available.
    pub fn as_repr(&self) -> Option<&Repr> {
        self.repr.as_ref()
    }
    /// Returns the default raw representation.
    pub fn default_repr(&self) -> Repr {
        to_key_repr(&self.key)
    }
    /// Returns a raw representation.
    pub fn display_repr(&self) -> Cow<'_, str> {
        self.as_repr()
            .and_then(|r| r.as_raw().as_str())
            .map(Cow::Borrowed)
            .unwrap_or_else(|| {
                Cow::Owned(self.default_repr().as_raw().as_str().unwrap().to_owned())
            })
    }
    /// Returns the surrounding whitespace
    pub fn decor_mut(&mut self) -> &mut Decor {
        &mut self.decor
    }
    /// Returns the surrounding whitespace
    pub fn decor(&self) -> &Decor {
        &self.decor
    }
    /// Returns the location within the original document
    #[cfg(feature = "serde")]
    pub(crate) fn span(&self) -> Option<std::ops::Range<usize>> {
        self.repr.as_ref().and_then(|r| r.span())
    }
    pub(crate) fn despan(&mut self, input: &str) {
        self.decor.despan(input);
        if let Some(repr) = &mut self.repr {
            repr.despan(input)
        }
    }
    /// Auto formats the key.
    pub fn fmt(&mut self) {
        self.repr = Some(to_key_repr(&self.key));
        self.decor.clear();
    }
    fn try_parse_simple(s: &str) -> Result<Key, crate::TomlError> {
        let mut key = parser::parse_key(s)?;
        key.despan(s);
        Ok(key)
    }
    fn try_parse_path(s: &str) -> Result<Vec<Key>, crate::TomlError> {
        let mut keys = parser::parse_key_path(s)?;
        for key in &mut keys {
            key.despan(s);
        }
        Ok(keys)
    }
}
impl std::ops::Deref for Key {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.get()
    }
}
impl std::hash::Hash for Key {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.get().hash(state);
    }
}
impl Ord for Key {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.get().cmp(other.get())
    }
}
impl PartialOrd for Key {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Eq for Key {}
impl PartialEq for Key {
    #[inline]
    fn eq(&self, other: &Key) -> bool {
        PartialEq::eq(self.get(), other.get())
    }
}
impl PartialEq<str> for Key {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        PartialEq::eq(self.get(), other)
    }
}
impl<'s> PartialEq<&'s str> for Key {
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        PartialEq::eq(self.get(), *other)
    }
}
impl PartialEq<String> for Key {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        PartialEq::eq(self.get(), other.as_str())
    }
}
impl std::fmt::Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        crate::encode::Encode::encode(self, f, None, ("", ""))
    }
}
impl FromStr for Key {
    type Err = crate::TomlError;
    /// Tries to parse a key from a &str,
    /// if fails, tries as basic quoted key (surrounds with "")
    /// and then literal quoted key (surrounds with '')
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Key::try_parse_simple(s)
    }
}
fn to_key_repr(key: &str) -> Repr {
    if key.as_bytes().iter().copied().all(is_unquoted_char) && !key.is_empty() {
        Repr::new_unchecked(key)
    } else {
        to_string_repr(key, Some(StringStyle::OnelineSingle), Some(false))
    }
}
impl<'b> From<&'b str> for Key {
    fn from(s: &'b str) -> Self {
        Key::new(s)
    }
}
impl<'b> From<&'b String> for Key {
    fn from(s: &'b String) -> Self {
        Key::new(s)
    }
}
impl From<String> for Key {
    fn from(s: String) -> Self {
        Key::new(s)
    }
}
impl From<InternalString> for Key {
    fn from(s: InternalString) -> Self {
        Key::new(s)
    }
}
#[doc(hidden)]
impl From<Key> for InternalString {
    fn from(key: Key) -> InternalString {
        key.key
    }
}
/// A mutable reference to a `Key`
#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct KeyMut<'k> {
    key: &'k mut Key,
}
impl<'k> KeyMut<'k> {
    /// Returns the parsed key value.
    pub fn get(&self) -> &str {
        self.key.get()
    }
    /// Returns the raw representation, if available.
    pub fn as_repr(&self) -> Option<&Repr> {
        self.key.as_repr()
    }
    /// Returns the default raw representation.
    pub fn default_repr(&self) -> Repr {
        self.key.default_repr()
    }
    /// Returns a raw representation.
    pub fn display_repr(&self) -> Cow<str> {
        self.key.display_repr()
    }
    /// Returns the surrounding whitespace
    pub fn decor_mut(&mut self) -> &mut Decor {
        self.key.decor_mut()
    }
    /// Returns the surrounding whitespace
    pub fn decor(&self) -> &Decor {
        self.key.decor()
    }
    /// Auto formats the key.
    pub fn fmt(&mut self) {
        self.key.fmt()
    }
}
impl<'k> std::ops::Deref for KeyMut<'k> {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.get()
    }
}
impl<'s> PartialEq<str> for KeyMut<'s> {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        PartialEq::eq(self.get(), other)
    }
}
impl<'s> PartialEq<&'s str> for KeyMut<'s> {
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        PartialEq::eq(self.get(), *other)
    }
}
impl<'s> PartialEq<String> for KeyMut<'s> {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        PartialEq::eq(self.get(), other.as_str())
    }
}
impl<'k> std::fmt::Display for KeyMut<'k> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.key, f)
    }
}
#[cfg(test)]
mod tests_rug_563 {
    use super::*;
    use crate::key::{Repr, StringStyle, to_key_repr};
    #[test]
    fn test_to_key_repr() {
        let _rug_st_tests_rug_563_rrrruuuugggg_test_to_key_repr = 0;
        let rug_fuzz_0 = "rust";
        let p0: &str = rug_fuzz_0;
        to_key_repr(p0);
        let _rug_ed_tests_rug_563_rrrruuuugggg_test_to_key_repr = 0;
    }
}
#[cfg(test)]
mod tests_rug_565 {
    use super::*;
    use crate::TomlError;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_565_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "foo.bar.baz";
        let mut p0 = rug_fuzz_0;
        crate::key::Key::parse(&p0);
        let _rug_ed_tests_rug_565_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_567 {
    use super::*;
    use crate::key::Key;
    use crate::repr::Decor;
    #[test]
    fn test_with_decor() {
        let _rug_st_tests_rug_567_rrrruuuugggg_test_with_decor = 0;
        let rug_fuzz_0 = "test_key";
        let p0: Key = Key::from(rug_fuzz_0);
        let p1: Decor = Decor::default();
        let result = p0.with_decor(p1);
        debug_assert_eq!(
            result, Key { decor : Default::default(), ..Key::from("test_key") }
        );
        let _rug_ed_tests_rug_567_rrrruuuugggg_test_with_decor = 0;
    }
}
#[cfg(test)]
mod tests_rug_568 {
    use super::*;
    use crate::{key, KeyMut};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_568_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "test";
        let mut p0: key::Key = key::Key::new(rug_fuzz_0);
        <key::Key>::as_mut(&mut p0);
        let _rug_ed_tests_rug_568_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_569 {
    use super::*;
    use crate::{de, key};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_569_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "some_key";
        let mut p0: key::Key = rug_fuzz_0.parse().unwrap();
        <key::Key>::get(&p0);
        let _rug_ed_tests_rug_569_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_570 {
    use super::*;
    use crate::key::Key;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_570_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "example";
        let mut p0: Key = Key::from(rug_fuzz_0);
        Key::get_internal(&p0);
        let _rug_ed_tests_rug_570_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_571 {
    use super::*;
    use crate::key::Key;
    use serde::de::Error;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_571_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "my_key";
        let mut p0: Key = Key::from(rug_fuzz_0);
        <Key>::as_repr(&p0);
        let _rug_ed_tests_rug_571_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_572 {
    use super::*;
    use crate::key::Key;
    #[test]
    fn test_default_repr() {
        let _rug_st_tests_rug_572_rrrruuuugggg_test_default_repr = 0;
        let rug_fuzz_0 = "example";
        let p0: Key = Key::from(rug_fuzz_0);
        p0.default_repr();
        let _rug_ed_tests_rug_572_rrrruuuugggg_test_default_repr = 0;
    }
}
#[cfg(test)]
mod tests_rug_573 {
    use super::*;
    use crate::key::Key;
    use std::borrow::Cow;
    #[test]
    fn test_display_repr() {
        let _rug_st_tests_rug_573_rrrruuuugggg_test_display_repr = 0;
        let rug_fuzz_0 = "...";
        let p0: Key = rug_fuzz_0.into();
        Key::display_repr(&p0);
        let _rug_ed_tests_rug_573_rrrruuuugggg_test_display_repr = 0;
    }
}
#[cfg(test)]
mod tests_rug_574 {
    use super::*;
    use crate::de;
    use serde::de::Error;
    use crate::{Decor, key};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_574_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "example";
        let mut p0: key::Key = rug_fuzz_0.into();
        <key::Key>::decor_mut(&mut p0);
        let _rug_ed_tests_rug_574_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_575 {
    use super::*;
    use crate::{key::Key, Decor};
    #[test]
    fn test_decor() {
        let _rug_st_tests_rug_575_rrrruuuugggg_test_decor = 0;
        let rug_fuzz_0 = "example.key";
        let mut p0: Key = Key::new(rug_fuzz_0);
        Key::decor(&p0);
        let _rug_ed_tests_rug_575_rrrruuuugggg_test_decor = 0;
    }
}
#[cfg(test)]
mod tests_rug_576 {
    use super::*;
    use crate::key::Key;
    #[test]
    fn test_despan() {
        let _rug_st_tests_rug_576_rrrruuuugggg_test_despan = 0;
        let rug_fuzz_0 = "test";
        let rug_fuzz_1 = "sample_input";
        let mut p0: Key = Key::new(rug_fuzz_0);
        let p1: &str = rug_fuzz_1;
        p0.despan(p1);
        let _rug_ed_tests_rug_576_rrrruuuugggg_test_despan = 0;
    }
}
#[cfg(test)]
mod tests_rug_577 {
    use super::*;
    use crate::key::Key;
    use serde::de::Error;
    #[test]
    fn test_fmt() {
        let _rug_st_tests_rug_577_rrrruuuugggg_test_fmt = 0;
        let rug_fuzz_0 = "my_key";
        let mut p0: Key = Key::new(rug_fuzz_0);
        Key::fmt(&mut p0);
        let _rug_ed_tests_rug_577_rrrruuuugggg_test_fmt = 0;
    }
}
#[cfg(test)]
mod tests_rug_578 {
    use super::*;
    use crate::TomlError;
    use crate::parser;
    use crate::key::Key;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_578_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample_data";
        let mut p0 = rug_fuzz_0;
        <Key>::try_parse_simple(&p0);
        let _rug_ed_tests_rug_578_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_579 {
    use super::*;
    use crate::parser;
    use crate::TomlError;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_579_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "foo.bar.baz";
        let p0 = rug_fuzz_0;
        <Key>::try_parse_path(&p0).unwrap();
        let _rug_ed_tests_rug_579_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_589 {
    use super::*;
    use crate::key::Key;
    use std::convert::From;
    #[test]
    fn test_from() {
        let _rug_st_tests_rug_589_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = "...";
        let p0: &str = rug_fuzz_0;
        let _ = <Key as std::convert::From<&str>>::from(&p0);
        let _rug_ed_tests_rug_589_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_rug_592 {
    use super::*;
    use crate::key::Key;
    use crate::internal_string::InternalString;
    use std::convert::From;
    #[test]
    fn test_from() {
        let _rug_st_tests_rug_592_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = "test";
        let p0: InternalString = rug_fuzz_0.into();
        let _ = <Key as From<InternalString>>::from(p0);
        let _rug_ed_tests_rug_592_rrrruuuugggg_test_from = 0;
    }
}
