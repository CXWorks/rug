use super::Value;
use crate::lib::iter::FromIterator;
use crate::lib::*;
use crate::map::Map;
use crate::number::Number;
#[cfg(feature = "arbitrary_precision")]
use serde::serde_if_integer128;
macro_rules! from_integer {
    ($($ty:ident)*) => {
        $(impl From <$ty > for Value { fn from(n : $ty) -> Self { Value::Number(n.into())
        } })*
    };
}
from_integer! {
    i8 i16 i32 i64 isize u8 u16 u32 u64 usize
}
#[cfg(feature = "arbitrary_precision")]
serde_if_integer128! {
    from_integer! { i128 u128 }
}
impl From<f32> for Value {
    /// Convert 32-bit floating point number to `Value`
    ///
    /// # Examples
    ///
    /// ```
    /// use serde_json::Value;
    ///
    /// let f: f32 = 13.37;
    /// let x: Value = f.into();
    /// ```
    fn from(f: f32) -> Self {
        From::from(f as f64)
    }
}
impl From<f64> for Value {
    /// Convert 64-bit floating point number to `Value`
    ///
    /// # Examples
    ///
    /// ```
    /// use serde_json::Value;
    ///
    /// let f: f64 = 13.37;
    /// let x: Value = f.into();
    /// ```
    fn from(f: f64) -> Self {
        Number::from_f64(f).map_or(Value::Null, Value::Number)
    }
}
impl From<bool> for Value {
    /// Convert boolean to `Value`
    ///
    /// # Examples
    ///
    /// ```
    /// use serde_json::Value;
    ///
    /// let b = false;
    /// let x: Value = b.into();
    /// ```
    fn from(f: bool) -> Self {
        Value::Bool(f)
    }
}
impl From<String> for Value {
    /// Convert `String` to `Value`
    ///
    /// # Examples
    ///
    /// ```
    /// use serde_json::Value;
    ///
    /// let s: String = "lorem".to_string();
    /// let x: Value = s.into();
    /// ```
    fn from(f: String) -> Self {
        Value::String(f)
    }
}
impl<'a> From<&'a str> for Value {
    /// Convert string slice to `Value`
    ///
    /// # Examples
    ///
    /// ```
    /// use serde_json::Value;
    ///
    /// let s: &str = "lorem";
    /// let x: Value = s.into();
    /// ```
    fn from(f: &str) -> Self {
        Value::String(f.to_string())
    }
}
impl<'a> From<Cow<'a, str>> for Value {
    /// Convert copy-on-write string to `Value`
    ///
    /// # Examples
    ///
    /// ```
    /// use serde_json::Value;
    /// use std::borrow::Cow;
    ///
    /// let s: Cow<str> = Cow::Borrowed("lorem");
    /// let x: Value = s.into();
    /// ```
    ///
    /// ```
    /// use serde_json::Value;
    /// use std::borrow::Cow;
    ///
    /// let s: Cow<str> = Cow::Owned("lorem".to_string());
    /// let x: Value = s.into();
    /// ```
    fn from(f: Cow<'a, str>) -> Self {
        Value::String(f.into_owned())
    }
}
impl From<Number> for Value {
    /// Convert `Number` to `Value`
    ///
    /// # Examples
    ///
    /// ```
    /// use serde_json::{Number, Value};
    ///
    /// let n = Number::from(7);
    /// let x: Value = n.into();
    /// ```
    fn from(f: Number) -> Self {
        Value::Number(f)
    }
}
impl From<Map<String, Value>> for Value {
    /// Convert map (with string keys) to `Value`
    ///
    /// # Examples
    ///
    /// ```
    /// use serde_json::{Map, Value};
    ///
    /// let mut m = Map::new();
    /// m.insert("Lorem".to_string(), "ipsum".into());
    /// let x: Value = m.into();
    /// ```
    fn from(f: Map<String, Value>) -> Self {
        Value::Object(f)
    }
}
impl<T: Into<Value>> From<Vec<T>> for Value {
    /// Convert a `Vec` to `Value`
    ///
    /// # Examples
    ///
    /// ```
    /// use serde_json::Value;
    ///
    /// let v = vec!["lorem", "ipsum", "dolor"];
    /// let x: Value = v.into();
    /// ```
    fn from(f: Vec<T>) -> Self {
        Value::Array(f.into_iter().map(Into::into).collect())
    }
}
impl<'a, T: Clone + Into<Value>> From<&'a [T]> for Value {
    /// Convert a slice to `Value`
    ///
    /// # Examples
    ///
    /// ```
    /// use serde_json::Value;
    ///
    /// let v: &[&str] = &["lorem", "ipsum", "dolor"];
    /// let x: Value = v.into();
    /// ```
    fn from(f: &'a [T]) -> Self {
        Value::Array(f.iter().cloned().map(Into::into).collect())
    }
}
impl<T: Into<Value>> FromIterator<T> for Value {
    /// Convert an iteratable type to a `Value`
    ///
    /// # Examples
    ///
    /// ```
    /// use serde_json::Value;
    ///
    /// let v = std::iter::repeat(42).take(5);
    /// let x: Value = v.collect();
    /// ```
    ///
    /// ```
    /// use serde_json::Value;
    ///
    /// let v: Vec<_> = vec!["lorem", "ipsum", "dolor"];
    /// let x: Value = v.into_iter().collect();
    /// ```
    ///
    /// ```
    /// use std::iter::FromIterator;
    /// use serde_json::Value;
    ///
    /// let x: Value = Value::from_iter(vec!["lorem", "ipsum", "dolor"]);
    /// ```
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Value::Array(iter.into_iter().map(Into::into).collect())
    }
}
impl<K: Into<String>, V: Into<Value>> FromIterator<(K, V)> for Value {
    /// Convert an iteratable type to a `Value`
    ///
    /// # Examples
    ///
    /// ```
    /// use serde_json::Value;
    ///
    /// let v: Vec<_> = vec![("lorem", 40), ("ipsum", 2)];
    /// let x: Value = v.into_iter().collect();
    /// ```
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        Value::Object(iter.into_iter().map(|(k, v)| (k.into(), v.into())).collect())
    }
}
impl From<()> for Value {
    /// Convert `()` to `Value`
    ///
    /// # Examples
    ///
    /// ```
    /// use serde_json::Value;
    ///
    /// let u = ();
    /// let x: Value = u.into();
    /// ```
    fn from((): ()) -> Self {
        Value::Null
    }
}
#[cfg(test)]
mod tests_rug_582 {
    use super::*;
    use crate::value::from::Value;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_582_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let p0: i8 = rug_fuzz_0;
        Value::from(p0);
        let _rug_ed_tests_rug_582_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_583 {
    use super::*;
    use crate::value::Value;
    use std::convert::From;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_583_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let p0: i16 = rug_fuzz_0;
        <Value as From<i16>>::from(p0);
        let _rug_ed_tests_rug_583_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_584 {
    use super::*;
    use crate::value::from;
    use crate::value::Value;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_584_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: i32 = rug_fuzz_0;
        Value::from(p0);
        let _rug_ed_tests_rug_584_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_585 {
    use super::*;
    use crate::Value;
    #[test]
    fn test_from() {
        let _rug_st_tests_rug_585_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = 42;
        let p0: i64 = rug_fuzz_0;
        <Value as std::convert::From<i64>>::from(p0);
        let _rug_ed_tests_rug_585_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_rug_586 {
    use super::*;
    use crate::value::Value;
    #[test]
    fn test_from() {
        let _rug_st_tests_rug_586_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = 42;
        let p0: isize = rug_fuzz_0;
        let result = Value::from(p0);
        debug_assert_eq!(result, Value::Number(42.into()));
        let _rug_ed_tests_rug_586_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_rug_587 {
    use super::*;
    use crate::value::Value;
    #[test]
    fn test_from() {
        let _rug_st_tests_rug_587_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = 10;
        let p0: u8 = rug_fuzz_0;
        Value::from(p0);
        let _rug_ed_tests_rug_587_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_rug_588 {
    use super::*;
    use crate::value::Value;
    use std::convert::From;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_588_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let p0: u16 = rug_fuzz_0;
        debug_assert_eq!(Value::from(p0), Value::Number(p0.into()));
        let _rug_ed_tests_rug_588_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_589 {
    use super::*;
    use crate::value::from;
    use crate::value::Value;
    #[test]
    fn test_from() {
        let _rug_st_tests_rug_589_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = 42;
        let p0: u32 = rug_fuzz_0;
        Value::from(p0);
        let _rug_ed_tests_rug_589_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_rug_590 {
    use super::*;
    use crate::value::Value;
    #[test]
    fn test_from() {
        let _rug_st_tests_rug_590_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = 42;
        let p0: u64 = rug_fuzz_0;
        Value::from(p0);
        let _rug_ed_tests_rug_590_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_rug_591 {
    use super::*;
    use crate::Value;
    #[test]
    fn test_from() {
        let _rug_st_tests_rug_591_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = 42;
        let p0: usize = rug_fuzz_0;
        Value::from(p0);
        let _rug_ed_tests_rug_591_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_rug_592 {
    use super::*;
    use crate::Value;
    #[test]
    fn test_from() {
        let _rug_st_tests_rug_592_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = 13.37;
        let f: f32 = rug_fuzz_0;
        let p0: f32 = f;
        <Value as From<f32>>::from(p0);
        let _rug_ed_tests_rug_592_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_rug_593 {
    use super::*;
    use crate::{Value, Number};
    #[test]
    fn test_from() {
        let _rug_st_tests_rug_593_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = 13.37;
        let p0: f64 = rug_fuzz_0;
        let result: Value = <_>::from(p0);
        debug_assert_eq!(result, Value::Number(Number::from_f64(p0).unwrap()));
        let _rug_ed_tests_rug_593_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_rug_594 {
    use super::*;
    use crate::lib::From;
    use crate::Value;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_594_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = false;
        let p0: bool = rug_fuzz_0;
        <Value>::from(p0);
        let _rug_ed_tests_rug_594_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_595 {
    use super::*;
    use crate::Value;
    #[test]
    fn test_from() {
        let _rug_st_tests_rug_595_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = "lorem";
        let p0: std::string::String = rug_fuzz_0.to_string();
        <Value as std::convert::From<std::string::String>>::from(p0);
        let _rug_ed_tests_rug_595_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_rug_596 {
    use super::*;
    use crate::value;
    #[test]
    fn test_from() {
        let _rug_st_tests_rug_596_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = "lorem";
        let p0: &str = rug_fuzz_0;
        <value::Value>::from(p0);
        let _rug_ed_tests_rug_596_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_rug_597 {
    use super::*;
    use crate::Value;
    use std::borrow::Cow;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_597_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample data";
        let mut p0: Cow<str> = Cow::Borrowed(rug_fuzz_0);
        <Value>::from(p0);
        let _rug_ed_tests_rug_597_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_598 {
    use super::*;
    use crate::value::Number;
    use crate::value::Value;
    #[test]
    fn test_from() {
        let _rug_st_tests_rug_598_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = 7;
        let mut p0: Number = rug_fuzz_0.into();
        let _ = <Value as std::convert::From<Number>>::from(p0);
        let _rug_ed_tests_rug_598_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_rug_599 {
    use super::*;
    use crate::{Map, Value};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_599_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "Lorem";
        let rug_fuzz_1 = "ipsum";
        let mut p0 = Map::new();
        p0.insert(rug_fuzz_0.to_string(), Value::String(rug_fuzz_1.to_string()));
        <Value>::from(p0);
        let _rug_ed_tests_rug_599_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_600 {
    use super::*;
    use crate::Value;
    #[test]
    fn test_from() {
        let _rug_st_tests_rug_600_rrrruuuugggg_test_from = 0;
        let mut p0: Vec<Value> = Vec::new();
        let _ = <Value>::from(p0);
        let _rug_ed_tests_rug_600_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_rug_601 {
    use super::*;
    use crate::Value;
    #[test]
    fn test_from() {
        let p0: &[&str] = &["lorem", "ipsum", "dolor"];
        <Value as std::convert::From<&'static [&str]>>::from(p0);
    }
}
#[cfg(test)]
mod tests_rug_602 {
    use super::*;
    use crate::map::Map;
    use crate::Value;
    #[test]
    fn test_from_iter() {
        let _rug_st_tests_rug_602_rrrruuuugggg_test_from_iter = 0;
        let mut p0 = Map::<String, Value>::new();
        let result: Value = Value::from_iter(p0);
        let _rug_ed_tests_rug_602_rrrruuuugggg_test_from_iter = 0;
    }
}
#[cfg(test)]
mod tests_rug_603 {
    use super::*;
    use crate::Value;
    use crate::map::Map;
    #[test]
    fn test_from_iter() {
        let _rug_st_tests_rug_603_rrrruuuugggg_test_from_iter = 0;
        let mut p0 = Map::<String, Value>::new();
        let _ = <Value as std::iter::FromIterator<(_, _)>>::from_iter(p0);
        let _rug_ed_tests_rug_603_rrrruuuugggg_test_from_iter = 0;
    }
}
#[cfg(test)]
mod tests_rug_604 {
    use super::*;
    use crate::lib::From;
    use crate::Value;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_604_rrrruuuugggg_test_rug = 0;
        let p0: () = ();
        <Value>::from(p0);
        let _rug_ed_tests_rug_604_rrrruuuugggg_test_rug = 0;
    }
}
