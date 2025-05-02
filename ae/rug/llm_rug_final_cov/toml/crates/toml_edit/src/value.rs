use std::iter::FromIterator;
use std::str::FromStr;
use toml_datetime::*;
use crate::key::Key;
use crate::parser;
use crate::repr::{Decor, Formatted};
use crate::{Array, InlineTable, InternalString, RawString};
/// Representation of a TOML Value (as part of a Key/Value Pair).
#[derive(Debug, Clone)]
pub enum Value {
    /// A string value.
    String(Formatted<String>),
    /// A 64-bit integer value.
    Integer(Formatted<i64>),
    /// A 64-bit float value.
    Float(Formatted<f64>),
    /// A boolean value.
    Boolean(Formatted<bool>),
    /// An RFC 3339 formatted date-time with offset.
    Datetime(Formatted<Datetime>),
    /// An inline array of values.
    Array(Array),
    /// An inline table of key/value pairs.
    InlineTable(InlineTable),
}
/// Downcasting
impl Value {
    /// Text description of value type
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::String(..) => "string",
            Value::Integer(..) => "integer",
            Value::Float(..) => "float",
            Value::Boolean(..) => "boolean",
            Value::Datetime(..) => "datetime",
            Value::Array(..) => "array",
            Value::InlineTable(..) => "inline table",
        }
    }
    /// Casts `self` to str.
    pub fn as_str(&self) -> Option<&str> {
        match *self {
            Value::String(ref value) => Some(value.value()),
            _ => None,
        }
    }
    /// Returns true iff `self` is a string.
    pub fn is_str(&self) -> bool {
        self.as_str().is_some()
    }
    /// Casts `self` to integer.
    pub fn as_integer(&self) -> Option<i64> {
        match *self {
            Value::Integer(ref value) => Some(*value.value()),
            _ => None,
        }
    }
    /// Returns true iff `self` is an integer.
    pub fn is_integer(&self) -> bool {
        self.as_integer().is_some()
    }
    /// Casts `self` to float.
    pub fn as_float(&self) -> Option<f64> {
        match *self {
            Value::Float(ref value) => Some(*value.value()),
            _ => None,
        }
    }
    /// Returns true iff `self` is a float.
    pub fn is_float(&self) -> bool {
        self.as_float().is_some()
    }
    /// Casts `self` to boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            Value::Boolean(ref value) => Some(*value.value()),
            _ => None,
        }
    }
    /// Returns true iff `self` is a boolean.
    pub fn is_bool(&self) -> bool {
        self.as_bool().is_some()
    }
    /// Casts `self` to date-time.
    pub fn as_datetime(&self) -> Option<&Datetime> {
        match *self {
            Value::Datetime(ref value) => Some(value.value()),
            _ => None,
        }
    }
    /// Returns true iff `self` is a date-time.
    pub fn is_datetime(&self) -> bool {
        self.as_datetime().is_some()
    }
    /// Casts `self` to array.
    pub fn as_array(&self) -> Option<&Array> {
        match *self {
            Value::Array(ref value) => Some(value),
            _ => None,
        }
    }
    /// Casts `self` to mutable array.
    pub fn as_array_mut(&mut self) -> Option<&mut Array> {
        match *self {
            Value::Array(ref mut value) => Some(value),
            _ => None,
        }
    }
    /// Returns true iff `self` is an array.
    pub fn is_array(&self) -> bool {
        self.as_array().is_some()
    }
    /// Casts `self` to inline table.
    pub fn as_inline_table(&self) -> Option<&InlineTable> {
        match *self {
            Value::InlineTable(ref value) => Some(value),
            _ => None,
        }
    }
    /// Casts `self` to mutable inline table.
    pub fn as_inline_table_mut(&mut self) -> Option<&mut InlineTable> {
        match *self {
            Value::InlineTable(ref mut value) => Some(value),
            _ => None,
        }
    }
    /// Returns true iff `self` is an inline table.
    pub fn is_inline_table(&self) -> bool {
        self.as_inline_table().is_some()
    }
}
impl Value {
    /// Get the decoration of the value.
    /// # Example
    /// ```rust
    /// let v = toml_edit::Value::from(true);
    /// assert_eq!(v.decor().suffix(), None);
    ///```
    pub fn decor_mut(&mut self) -> &mut Decor {
        match self {
            Value::String(f) => f.decor_mut(),
            Value::Integer(f) => f.decor_mut(),
            Value::Float(f) => f.decor_mut(),
            Value::Boolean(f) => f.decor_mut(),
            Value::Datetime(f) => f.decor_mut(),
            Value::Array(a) => a.decor_mut(),
            Value::InlineTable(t) => t.decor_mut(),
        }
    }
    /// Get the decoration of the value.
    /// # Example
    /// ```rust
    /// let v = toml_edit::Value::from(true);
    /// assert_eq!(v.decor().suffix(), None);
    ///```
    pub fn decor(&self) -> &Decor {
        match *self {
            Value::String(ref f) => f.decor(),
            Value::Integer(ref f) => f.decor(),
            Value::Float(ref f) => f.decor(),
            Value::Boolean(ref f) => f.decor(),
            Value::Datetime(ref f) => f.decor(),
            Value::Array(ref a) => a.decor(),
            Value::InlineTable(ref t) => t.decor(),
        }
    }
    /// Sets the prefix and the suffix for value.
    /// # Example
    /// ```rust
    /// let mut v = toml_edit::Value::from(42);
    /// assert_eq!(&v.to_string(), "42");
    /// let d = v.decorated(" ", " ");
    /// assert_eq!(&d.to_string(), " 42 ");
    /// ```
    pub fn decorated(
        mut self,
        prefix: impl Into<RawString>,
        suffix: impl Into<RawString>,
    ) -> Self {
        self.decorate(prefix, suffix);
        self
    }
    pub(crate) fn decorate(
        &mut self,
        prefix: impl Into<RawString>,
        suffix: impl Into<RawString>,
    ) {
        let decor = self.decor_mut();
        *decor = Decor::new(prefix, suffix);
    }
    /// Returns the location within the original document
    pub(crate) fn span(&self) -> Option<std::ops::Range<usize>> {
        match self {
            Value::String(f) => f.span(),
            Value::Integer(f) => f.span(),
            Value::Float(f) => f.span(),
            Value::Boolean(f) => f.span(),
            Value::Datetime(f) => f.span(),
            Value::Array(a) => a.span(),
            Value::InlineTable(t) => t.span(),
        }
    }
    pub(crate) fn despan(&mut self, input: &str) {
        match self {
            Value::String(f) => f.despan(input),
            Value::Integer(f) => f.despan(input),
            Value::Float(f) => f.despan(input),
            Value::Boolean(f) => f.despan(input),
            Value::Datetime(f) => f.despan(input),
            Value::Array(a) => a.despan(input),
            Value::InlineTable(t) => t.despan(input),
        }
    }
}
impl FromStr for Value {
    type Err = crate::TomlError;
    /// Parses a value from a &str
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parser::parse_value(s)
    }
}
impl<'b> From<&'b Value> for Value {
    fn from(s: &'b Value) -> Self {
        s.clone()
    }
}
impl<'b> From<&'b str> for Value {
    fn from(s: &'b str) -> Self {
        s.to_owned().into()
    }
}
impl<'b> From<&'b String> for Value {
    fn from(s: &'b String) -> Self {
        s.to_owned().into()
    }
}
impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(Formatted::new(s))
    }
}
impl<'b> From<&'b InternalString> for Value {
    fn from(s: &'b InternalString) -> Self {
        s.as_str().into()
    }
}
impl From<InternalString> for Value {
    fn from(s: InternalString) -> Self {
        s.as_str().into()
    }
}
impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Value::Integer(Formatted::new(i))
    }
}
impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Value::Float(Formatted::new(f))
    }
}
impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Boolean(Formatted::new(b))
    }
}
impl From<Datetime> for Value {
    fn from(d: Datetime) -> Self {
        Value::Datetime(Formatted::new(d))
    }
}
impl From<Date> for Value {
    fn from(d: Date) -> Self {
        let d: Datetime = d.into();
        d.into()
    }
}
impl From<Time> for Value {
    fn from(d: Time) -> Self {
        let d: Datetime = d.into();
        d.into()
    }
}
impl From<Array> for Value {
    fn from(array: Array) -> Self {
        Value::Array(array)
    }
}
impl From<InlineTable> for Value {
    fn from(table: InlineTable) -> Self {
        Value::InlineTable(table)
    }
}
impl<V: Into<Value>> FromIterator<V> for Value {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = V>,
    {
        let array: Array = iter.into_iter().collect();
        Value::Array(array)
    }
}
impl<K: Into<Key>, V: Into<Value>> FromIterator<(K, V)> for Value {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
    {
        let table: InlineTable = iter.into_iter().collect();
        Value::InlineTable(table)
    }
}
impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        crate::encode::Encode::encode(self, f, None, ("", ""))
    }
}
pub(crate) const DEFAULT_VALUE_DECOR: (&str, &str) = (" ", "");
pub(crate) const DEFAULT_TRAILING_VALUE_DECOR: (&str, &str) = (" ", " ");
pub(crate) const DEFAULT_LEADING_VALUE_DECOR: (&str, &str) = ("", "");
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn from_iter_formatting() {
        let features = vec!["node".to_owned(), "mouth".to_owned()];
        let features: Value = features.iter().cloned().collect();
        assert_eq!(features.to_string(), r#"["node", "mouth"]"#);
    }
}
#[cfg(test)]
mod tests_rug_20 {
    use super::*;
    use crate::value::Value;
    #[test]
    fn test_as_str() {
        let _rug_st_tests_rug_20_rrrruuuugggg_test_as_str = 0;
        let rug_fuzz_0 = "hello";
        let p0 = Value::from(rug_fuzz_0);
        debug_assert_eq!(p0.as_str(), Some("hello"));
        let _rug_ed_tests_rug_20_rrrruuuugggg_test_as_str = 0;
    }
}
#[cfg(test)]
mod tests_rug_21 {
    use super::*;
    use crate::value;
    #[test]
    fn test_is_str() {
        let _rug_st_tests_rug_21_rrrruuuugggg_test_is_str = 0;
        let rug_fuzz_0 = "hello";
        let p0: value::Value = value::Value::from(rug_fuzz_0);
        debug_assert_eq!(p0.is_str(), true);
        let _rug_ed_tests_rug_21_rrrruuuugggg_test_is_str = 0;
    }
}
#[cfg(test)]
mod tests_rug_22 {
    use super::*;
    use crate::value::Value;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_22_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0 = Value::from(rug_fuzz_0);
        Value::as_integer(&p0);
        let _rug_ed_tests_rug_22_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_23 {
    use super::*;
    use crate::value::Value;
    #[test]
    fn test_is_integer() {
        let _rug_st_tests_rug_23_rrrruuuugggg_test_is_integer = 0;
        let rug_fuzz_0 = 42;
        let mut p0 = Value::from(rug_fuzz_0);
        debug_assert!(p0.is_integer());
        let _rug_ed_tests_rug_23_rrrruuuugggg_test_is_integer = 0;
    }
}
#[cfg(test)]
mod tests_rug_24 {
    use super::*;
    use crate::value::Value;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_24_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3.14;
        let rug_fuzz_1 = 10;
        let rug_fuzz_2 = "hello";
        let mut p0: Value = Value::from(rug_fuzz_0);
        debug_assert_eq!(p0.as_float(), Some(3.14));
        p0 = Value::from(rug_fuzz_1);
        debug_assert_eq!(p0.as_float(), None);
        p0 = Value::from(rug_fuzz_2);
        debug_assert_eq!(p0.as_float(), None);
        let _rug_ed_tests_rug_24_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_25 {
    use super::*;
    use crate::value::Value;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_25_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let rug_fuzz_1 = 3.14;
        let rug_fuzz_2 = "string";
        let mut p0: Value = Value::try_from(rug_fuzz_0).unwrap();
        debug_assert_eq!(p0.is_float(), false);
        p0 = Value::try_from(rug_fuzz_1).unwrap();
        debug_assert_eq!(p0.is_float(), true);
        p0 = Value::try_from(rug_fuzz_2).unwrap();
        debug_assert_eq!(p0.is_float(), false);
        let _rug_ed_tests_rug_25_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_26 {
    use super::*;
    use crate::value::Value;
    #[test]
    fn test_as_bool() {
        let _rug_st_tests_rug_26_rrrruuuugggg_test_as_bool = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = false;
        let rug_fuzz_2 = 42;
        let rug_fuzz_3 = "true";
        let p0 = Value::from(rug_fuzz_0);
        debug_assert_eq!(p0.as_bool(), Some(true));
        let p1 = Value::from(rug_fuzz_1);
        debug_assert_eq!(p1.as_bool(), Some(false));
        let p2 = Value::from(rug_fuzz_2);
        debug_assert_eq!(p2.as_bool(), None);
        let p3 = Value::from(rug_fuzz_3);
        debug_assert_eq!(p3.as_bool(), None);
        let _rug_ed_tests_rug_26_rrrruuuugggg_test_as_bool = 0;
    }
}
#[cfg(test)]
mod tests_rug_27 {
    use super::*;
    use crate::value::Value;
    use serde::de::DeserializeOwned;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_27_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3;
        let rug_fuzz_1 = true;
        let mut p0: Value = Value::try_from(rug_fuzz_0).unwrap();
        debug_assert_eq!(Value::is_bool(& p0), false);
        p0 = Value::try_from(rug_fuzz_1).unwrap();
        debug_assert_eq!(Value::is_bool(& p0), true);
        let _rug_ed_tests_rug_27_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_29 {
    use super::*;
    use crate::value::Value;
    #[test]
    fn test_is_datetime() {
        let _rug_st_tests_rug_29_rrrruuuugggg_test_is_datetime = 0;
        let rug_fuzz_0 = "2022-12-31T23:59:59Z";
        let rug_fuzz_1 = "2022-12-31";
        let mut p0: Value = Value::try_from(rug_fuzz_0).unwrap();
        debug_assert_eq!(Value::is_datetime(& p0), true);
        let mut p1: Value = Value::try_from(rug_fuzz_1).unwrap();
        debug_assert_eq!(Value::is_datetime(& p1), false);
        let _rug_ed_tests_rug_29_rrrruuuugggg_test_is_datetime = 0;
    }
}
#[cfg(test)]
mod tests_rug_30 {
    use super::*;
    use crate::value::Value;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_30_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0 = Value::from(rug_fuzz_0);
        Value::as_array(&p0);
        let _rug_ed_tests_rug_30_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_31 {
    use super::*;
    use crate::value::Value;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_31_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "test";
        let mut p0: Value = Value::from(rug_fuzz_0);
        Value::as_array_mut(&mut p0);
        let _rug_ed_tests_rug_31_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_32 {
    use super::*;
    use crate::value::Value;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_32_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "[]";
        let mut p0: Value = Value::from_str(rug_fuzz_0).unwrap();
        Value::is_array(&p0);
        let _rug_ed_tests_rug_32_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_35 {
    use super::*;
    use crate::value::Value;
    use std::convert::From;
    #[test]
    fn test_from() {
        let _rug_st_tests_rug_35_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = 42;
        let p0: i64 = rug_fuzz_0;
        <Value as std::convert::From<i64>>::from(p0);
        let _rug_ed_tests_rug_35_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_rug_955 {
    use super::*;
    use crate::value::Value;
    #[test]
    fn test_as_str() {
        let _rug_st_tests_rug_955_rrrruuuugggg_test_as_str = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = "hello";
        let p0: Value = Value::from(rug_fuzz_0);
        debug_assert_eq!(p0.as_str(), None);
        let p1: Value = Value::from(rug_fuzz_1);
        debug_assert_eq!(p1.as_str(), Some("hello"));
        let _rug_ed_tests_rug_955_rrrruuuugggg_test_as_str = 0;
    }
}
#[cfg(test)]
mod tests_rug_956 {
    use super::*;
    use crate::value::Value;
    #[test]
    fn test_is_str() {
        let _rug_st_tests_rug_956_rrrruuuugggg_test_is_str = 0;
        let rug_fuzz_0 = "Hello Rust";
        let p0 = Value::from(rug_fuzz_0);
        debug_assert_eq!(p0.is_str(), true);
        let _rug_ed_tests_rug_956_rrrruuuugggg_test_is_str = 0;
    }
}
#[cfg(test)]
mod tests_rug_957 {
    use super::*;
    use crate::value::Value;
    #[test]
    fn test_as_integer() {
        let _rug_st_tests_rug_957_rrrruuuugggg_test_as_integer = 0;
        let rug_fuzz_0 = 10;
        let p0 = Value::from(rug_fuzz_0);
        debug_assert_eq!(p0.as_integer(), Some(10));
        let _rug_ed_tests_rug_957_rrrruuuugggg_test_as_integer = 0;
    }
}
#[cfg(test)]
mod tests_rug_958 {
    use super::*;
    use crate::value::Value;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_958_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123;
        let mut p0: Value = Value::from(rug_fuzz_0);
        debug_assert_eq!(p0.is_integer(), true);
        let _rug_ed_tests_rug_958_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_959 {
    use super::*;
    use crate::value;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_959_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0 = value::Value::from(rug_fuzz_0);
        value::Value::as_float(&p0);
        let _rug_ed_tests_rug_959_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_960 {
    use super::*;
    use crate::value::Value;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_960_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 12.56;
        let mut p0: Value = Value::from(rug_fuzz_0);
        Value::is_float(&p0);
        let _rug_ed_tests_rug_960_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_961 {
    use super::*;
    use crate::Value;
    #[test]
    fn test_value_as_bool() {
        let _rug_st_tests_rug_961_rrrruuuugggg_test_value_as_bool = 0;
        let rug_fuzz_0 = true;
        let p0 = Value::from(rug_fuzz_0);
        debug_assert_eq!(Value::as_bool(& p0), Some(true));
        let _rug_ed_tests_rug_961_rrrruuuugggg_test_value_as_bool = 0;
    }
}
#[cfg(test)]
mod tests_rug_962 {
    use super::*;
    use crate::value::Value;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_962_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = true;
        let mut p0 = Value::from(rug_fuzz_0);
        Value::is_bool(&p0);
        let _rug_ed_tests_rug_962_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_964 {
    use super::*;
    use crate::value::Value;
    #[test]
    fn test_is_datetime() {
        let _rug_st_tests_rug_964_rrrruuuugggg_test_is_datetime = 0;
        let rug_fuzz_0 = "";
        let p0 = Value::from(rug_fuzz_0);
        debug_assert_eq!(p0.is_datetime(), false);
        let _rug_ed_tests_rug_964_rrrruuuugggg_test_is_datetime = 0;
    }
}
#[cfg(test)]
mod tests_rug_965 {
    use super::*;
    use crate::value::{Value, Array};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_965_rrrruuuugggg_test_rug = 0;
        let p0: &Value = &Value::Array(Array::new());
        Value::as_array(p0);
        let _rug_ed_tests_rug_965_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_966 {
    use super::*;
    use crate::value::Value;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_966_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: Value = Value::from(rug_fuzz_0);
        Value::as_array_mut(&mut p0);
        let _rug_ed_tests_rug_966_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_967 {
    use super::*;
    use crate::value::Value;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_967_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let mut p0 = Value::from(rug_fuzz_0);
        Value::is_array(&p0);
        let _rug_ed_tests_rug_967_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_968 {
    use super::*;
    use crate::value::Value;
    use crate::value::InlineTable;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_968_rrrruuuugggg_test_rug = 0;
        let mut p0: Value = Value::InlineTable(InlineTable::new());
        Value::as_inline_table(&p0);
        let _rug_ed_tests_rug_968_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_969 {
    use super::*;
    use crate::value::{Value, InlineTable};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_969_rrrruuuugggg_test_rug = 0;
        let mut p0: Value = Value::InlineTable(InlineTable::new());
        p0.as_inline_table_mut();
        let _rug_ed_tests_rug_969_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_970 {
    use super::*;
    use crate::value::Value;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_970_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "inline_table";
        let mut p0 = Value::from(rug_fuzz_0);
        Value::is_inline_table(&p0);
        let _rug_ed_tests_rug_970_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_971 {
    use super::*;
    use crate::value::{Decor, Value};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_971_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = true;
        let mut p0 = Value::from(rug_fuzz_0);
        Value::decor_mut(&mut p0);
        let _rug_ed_tests_rug_971_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_972 {
    use super::*;
    use crate::{Value, Decor};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_972_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = true;
        let mut p0: Value = Value::from(rug_fuzz_0);
        Value::decor(&p0);
        let _rug_ed_tests_rug_972_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_973 {
    use super::*;
    use crate::{Value, RawString};
    #[test]
    fn test_decorated() {
        let _rug_st_tests_rug_973_rrrruuuugggg_test_decorated = 0;
        let rug_fuzz_0 = 42;
        let rug_fuzz_1 = " ";
        let rug_fuzz_2 = " ";
        let mut p0 = Value::from(rug_fuzz_0);
        let p1 = RawString::from(rug_fuzz_1);
        let p2 = RawString::from(rug_fuzz_2);
        let result = p0.decorated(p1, p2);
        debug_assert_eq!(result.to_string(), " 42 ");
        let _rug_ed_tests_rug_973_rrrruuuugggg_test_decorated = 0;
    }
}
#[cfg(test)]
mod tests_rug_974 {
    use super::*;
    use crate::value::{Value, Decor, RawString};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_974_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample_value";
        let rug_fuzz_1 = "prefix";
        let rug_fuzz_2 = "suffix";
        let mut p0 = Value::from_str(rug_fuzz_0).unwrap();
        let p1 = RawString::from(rug_fuzz_1);
        let p2 = RawString::from(rug_fuzz_2);
        p0.decorate(p1, p2);
        let _rug_ed_tests_rug_974_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_977 {
    use super::*;
    use crate::{parser, value::Value};
    use std::str::FromStr;
    #[test]
    fn test_from_str() {
        let _rug_st_tests_rug_977_rrrruuuugggg_test_from_str = 0;
        let rug_fuzz_0 = "42";
        let p0: &str = rug_fuzz_0;
        let _ = <Value as FromStr>::from_str(p0);
        let _rug_ed_tests_rug_977_rrrruuuugggg_test_from_str = 0;
    }
}
#[cfg(test)]
mod tests_rug_979 {
    use super::*;
    use crate::value::Value;
    use std::convert::From;
    #[test]
    fn test_from() {
        let _rug_st_tests_rug_979_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = "example";
        let p0: &str = rug_fuzz_0;
        <Value as std::convert::From<&str>>::from(&p0);
        let _rug_ed_tests_rug_979_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_rug_981 {
    use super::*;
    use crate::value::Value;
    use std::convert::From;
    #[test]
    fn test_from() {
        let _rug_st_tests_rug_981_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = "test string";
        let p0: std::string::String = rug_fuzz_0.to_string();
        let _ = <Value as std::convert::From<std::string::String>>::from(p0);
        let _rug_ed_tests_rug_981_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_rug_984 {
    use super::*;
    use crate::value::Value;
    use std::convert::From;
    #[test]
    fn test_from() {
        let _rug_st_tests_rug_984_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = 10;
        let p0: i64 = rug_fuzz_0;
        Value::from(p0);
        let _rug_ed_tests_rug_984_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_rug_985 {
    use super::*;
    use crate::value::Value;
    use std::convert::From;
    #[test]
    fn test_from() {
        let _rug_st_tests_rug_985_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = 3.14;
        let p0: f64 = rug_fuzz_0;
        let _ = <Value as std::convert::From<f64>>::from(p0);
        let _rug_ed_tests_rug_985_rrrruuuugggg_test_from = 0;
    }
}
