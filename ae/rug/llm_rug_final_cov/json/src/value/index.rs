use super::Value;
use crate::map::Map;
use alloc::borrow::ToOwned;
use alloc::string::String;
use core::fmt::{self, Display};
use core::ops;
/// A type that can be used to index into a `serde_json::Value`.
///
/// The [`get`] and [`get_mut`] methods of `Value` accept any type that
/// implements `Index`, as does the [square-bracket indexing operator]. This
/// trait is implemented for strings which are used as the index into a JSON
/// map, and for `usize` which is used as the index into a JSON array.
///
/// [`get`]: ../enum.Value.html#method.get
/// [`get_mut`]: ../enum.Value.html#method.get_mut
/// [square-bracket indexing operator]: ../enum.Value.html#impl-Index%3CI%3E
///
/// This trait is sealed and cannot be implemented for types outside of
/// `serde_json`.
///
/// # Examples
///
/// ```
/// # use serde_json::json;
/// #
/// let data = json!({ "inner": [1, 2, 3] });
///
/// // Data is a JSON map so it can be indexed with a string.
/// let inner = &data["inner"];
///
/// // Inner is a JSON array so it can be indexed with an integer.
/// let first = &inner[0];
///
/// assert_eq!(first, 1);
/// ```
pub trait Index: private::Sealed {
    /// Return None if the key is not already in the array or object.
    #[doc(hidden)]
    fn index_into<'v>(&self, v: &'v Value) -> Option<&'v Value>;
    /// Return None if the key is not already in the array or object.
    #[doc(hidden)]
    fn index_into_mut<'v>(&self, v: &'v mut Value) -> Option<&'v mut Value>;
    /// Panic if array index out of bounds. If key is not already in the object,
    /// insert it with a value of null. Panic if Value is a type that cannot be
    /// indexed into, except if Value is null then it can be treated as an empty
    /// object.
    #[doc(hidden)]
    fn index_or_insert<'v>(&self, v: &'v mut Value) -> &'v mut Value;
}
impl Index for usize {
    fn index_into<'v>(&self, v: &'v Value) -> Option<&'v Value> {
        match v {
            Value::Array(vec) => vec.get(*self),
            _ => None,
        }
    }
    fn index_into_mut<'v>(&self, v: &'v mut Value) -> Option<&'v mut Value> {
        match v {
            Value::Array(vec) => vec.get_mut(*self),
            _ => None,
        }
    }
    fn index_or_insert<'v>(&self, v: &'v mut Value) -> &'v mut Value {
        match v {
            Value::Array(vec) => {
                let len = vec.len();
                vec.get_mut(*self)
                    .unwrap_or_else(|| {
                        panic!(
                            "cannot access index {} of JSON array of length {}", self,
                            len
                        )
                    })
            }
            _ => panic!("cannot access index {} of JSON {}", self, Type(v)),
        }
    }
}
impl Index for str {
    fn index_into<'v>(&self, v: &'v Value) -> Option<&'v Value> {
        match v {
            Value::Object(map) => map.get(self),
            _ => None,
        }
    }
    fn index_into_mut<'v>(&self, v: &'v mut Value) -> Option<&'v mut Value> {
        match v {
            Value::Object(map) => map.get_mut(self),
            _ => None,
        }
    }
    fn index_or_insert<'v>(&self, v: &'v mut Value) -> &'v mut Value {
        if let Value::Null = v {
            *v = Value::Object(Map::new());
        }
        match v {
            Value::Object(map) => map.entry(self.to_owned()).or_insert(Value::Null),
            _ => panic!("cannot access key {:?} in JSON {}", self, Type(v)),
        }
    }
}
impl Index for String {
    fn index_into<'v>(&self, v: &'v Value) -> Option<&'v Value> {
        self[..].index_into(v)
    }
    fn index_into_mut<'v>(&self, v: &'v mut Value) -> Option<&'v mut Value> {
        self[..].index_into_mut(v)
    }
    fn index_or_insert<'v>(&self, v: &'v mut Value) -> &'v mut Value {
        self[..].index_or_insert(v)
    }
}
impl<'a, T> Index for &'a T
where
    T: ?Sized + Index,
{
    fn index_into<'v>(&self, v: &'v Value) -> Option<&'v Value> {
        (**self).index_into(v)
    }
    fn index_into_mut<'v>(&self, v: &'v mut Value) -> Option<&'v mut Value> {
        (**self).index_into_mut(v)
    }
    fn index_or_insert<'v>(&self, v: &'v mut Value) -> &'v mut Value {
        (**self).index_or_insert(v)
    }
}
mod private {
    pub trait Sealed {}
    impl Sealed for usize {}
    impl Sealed for str {}
    impl Sealed for alloc::string::String {}
    impl<'a, T> Sealed for &'a T
    where
        T: ?Sized + Sealed,
    {}
}
/// Used in panic messages.
struct Type<'a>(&'a Value);
impl<'a> Display for Type<'a> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match *self.0 {
            Value::Null => formatter.write_str("null"),
            Value::Bool(_) => formatter.write_str("boolean"),
            Value::Number(_) => formatter.write_str("number"),
            Value::String(_) => formatter.write_str("string"),
            Value::Array(_) => formatter.write_str("array"),
            Value::Object(_) => formatter.write_str("object"),
        }
    }
}
impl<I> ops::Index<I> for Value
where
    I: Index,
{
    type Output = Value;
    /// Index into a `serde_json::Value` using the syntax `value[0]` or
    /// `value["k"]`.
    ///
    /// Returns `Value::Null` if the type of `self` does not match the type of
    /// the index, for example if the index is a string and `self` is an array
    /// or a number. Also returns `Value::Null` if the given key does not exist
    /// in the map or the given index is not within the bounds of the array.
    ///
    /// For retrieving deeply nested values, you should have a look at the
    /// `Value::pointer` method.
    ///
    /// # Examples
    ///
    /// ```
    /// # use serde_json::json;
    /// #
    /// let data = json!({
    ///     "x": {
    ///         "y": ["z", "zz"]
    ///     }
    /// });
    ///
    /// assert_eq!(data["x"]["y"], json!(["z", "zz"]));
    /// assert_eq!(data["x"]["y"][0], json!("z"));
    ///
    /// assert_eq!(data["a"], json!(null)); // returns null for undefined values
    /// assert_eq!(data["a"]["b"], json!(null)); // does not panic
    /// ```
    fn index(&self, index: I) -> &Value {
        static NULL: Value = Value::Null;
        index.index_into(self).unwrap_or(&NULL)
    }
}
impl<I> ops::IndexMut<I> for Value
where
    I: Index,
{
    /// Write into a `serde_json::Value` using the syntax `value[0] = ...` or
    /// `value["k"] = ...`.
    ///
    /// If the index is a number, the value must be an array of length bigger
    /// than the index. Indexing into a value that is not an array or an array
    /// that is too small will panic.
    ///
    /// If the index is a string, the value must be an object or null which is
    /// treated like an empty object. If the key is not already present in the
    /// object, it will be inserted with a value of null. Indexing into a value
    /// that is neither an object nor null will panic.
    ///
    /// # Examples
    ///
    /// ```
    /// # use serde_json::json;
    /// #
    /// let mut data = json!({ "x": 0 });
    ///
    /// // replace an existing key
    /// data["x"] = json!(1);
    ///
    /// // insert a new key
    /// data["y"] = json!([false, false, false]);
    ///
    /// // replace an array value
    /// data["y"][0] = json!(true);
    ///
    /// // inserted a deeply nested key
    /// data["a"]["b"]["c"]["d"] = json!(true);
    ///
    /// println!("{}", data);
    /// ```
    fn index_mut(&mut self, index: I) -> &mut Value {
        index.index_or_insert(self)
    }
}
#[cfg(test)]
mod tests_rug_629 {
    use super::*;
    use crate::value::Index;
    use crate::{json, Map, Value};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_629_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 2;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 3;
        let mut p0: usize = rug_fuzz_0;
        let mut p1: Value = json!([rug_fuzz_1, rug_fuzz_2, rug_fuzz_3]);
        <usize>::index_into(&p0, &p1);
        let _rug_ed_tests_rug_629_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_630 {
    use super::*;
    use crate::value::Index;
    use crate::{json, Map, Value};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_630_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let mut p0: usize = rug_fuzz_0;
        let mut p1 = Value::default();
        <usize>::index_into_mut(&p0, &mut p1);
        let _rug_ed_tests_rug_630_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_631 {
    use super::*;
    use crate::value::Index;
    use crate::{json, Map, Value};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_631_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 5;
        let mut p0: usize = rug_fuzz_0;
        let mut p1 = Value::default();
        <usize as Index>::index_or_insert(&p0, &mut p1);
        let _rug_ed_tests_rug_631_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_632 {
    use super::*;
    use crate::value::Index;
    use crate::{json, Map, Value};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_632_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key";
        let mut p0: &str = rug_fuzz_0;
        let mut p1 = Value::default();
        p0.index_into(&p1);
        let _rug_ed_tests_rug_632_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_633 {
    use super::*;
    use crate::value::Index;
    use crate::{json, Map, Value};
    #[test]
    fn test_index_into_mut() {
        let _rug_st_tests_rug_633_rrrruuuugggg_test_index_into_mut = 0;
        let rug_fuzz_0 = "key";
        let mut v31 = Value::default();
        let p1: &str = rug_fuzz_0;
        <str>::index_into_mut(&p1, &mut v31);
        let _rug_ed_tests_rug_633_rrrruuuugggg_test_index_into_mut = 0;
    }
}
#[cfg(test)]
mod tests_rug_634 {
    use super::*;
    use crate::value::Index;
    use crate::{json, Map, Value};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_634_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key";
        let mut p0: &str = rug_fuzz_0;
        let mut p1: &mut Value = &mut Value::default();
        <str as Index>::index_or_insert(p0, p1);
        let _rug_ed_tests_rug_634_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_635 {
    use super::*;
    use crate::Value;
    use crate::value::Index;
    #[test]
    fn test_index_into() {
        let _rug_st_tests_rug_635_rrrruuuugggg_test_index_into = 0;
        let rug_fuzz_0 = "some string";
        let mut p0: String = String::from(rug_fuzz_0);
        let mut v31 = Value::default();
        let p1: &Value = &v31;
        <String as Index>::index_into(&p0, p1);
        let _rug_ed_tests_rug_635_rrrruuuugggg_test_index_into = 0;
    }
}
#[cfg(test)]
mod tests_rug_636 {
    use super::*;
    use crate::value::Index;
    use crate::json;
    use crate::Map;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_636_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "test_string";
        let mut p0: std::string::String = rug_fuzz_0.to_string();
        let mut p1 = Value::default();
        <std::string::String as Index>::index_into_mut(&p0, &mut p1);
        let _rug_ed_tests_rug_636_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_637 {
    use super::*;
    use crate::{json, Map, Value};
    use crate::value::Index;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_637_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "test";
        let mut p0: String = String::from(rug_fuzz_0);
        let mut p1 = Value::default();
        p0.index_or_insert(&mut p1);
        let _rug_ed_tests_rug_637_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_638 {
    use super::*;
    use crate::value::Index;
    use crate::{json, Map, Value};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_638_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "test";
        let mut v31 = Value::default();
        let p1 = &v31;
        let mut p0: std::string::String = rug_fuzz_0.to_string();
        p0.index_into(p1);
        let _rug_ed_tests_rug_638_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_639 {
    use super::*;
    use crate::value::Value;
    use crate::value::index::Index;
    #[test]
    fn test_index_into_mut() {
        let _rug_st_tests_rug_639_rrrruuuugggg_test_index_into_mut = 0;
        let rug_fuzz_0 = "test";
        use crate::{json, Map, Value};
        let mut v31 = Value::default();
        let p0: std::string::String = rug_fuzz_0.to_string();
        let mut p1: Value = json!({ "key" : "value" });
        p0.index_into_mut(&mut p1);
        let _rug_ed_tests_rug_639_rrrruuuugggg_test_index_into_mut = 0;
    }
}
#[cfg(test)]
mod tests_rug_640 {
    use super::*;
    use crate::value::Index;
    use crate::{json, Map, Value};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_640_rrrruuuugggg_test_rug = 0;
        let mut p0: String = String::default();
        let mut p1: Value = Value::default();
        p0.index_or_insert(&mut p1);
        let _rug_ed_tests_rug_640_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_642 {
    use super::*;
    use crate::{json, Map, Value};
    #[test]
    fn test_index_mut() {
        let _rug_st_tests_rug_642_rrrruuuugggg_test_index_mut = 0;
        let rug_fuzz_0 = "x";
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = "y";
        let rug_fuzz_3 = false;
        let rug_fuzz_4 = false;
        let rug_fuzz_5 = false;
        let rug_fuzz_6 = "y";
        let rug_fuzz_7 = 0;
        let rug_fuzz_8 = true;
        let rug_fuzz_9 = "a";
        let rug_fuzz_10 = "b";
        let rug_fuzz_11 = "c";
        let rug_fuzz_12 = "d";
        let rug_fuzz_13 = true;
        let rug_fuzz_14 = "x";
        let mut data = json!({ "x" : 0 });
        data[rug_fuzz_0] = json!(rug_fuzz_1);
        data[rug_fuzz_2] = json!([rug_fuzz_3, rug_fuzz_4, rug_fuzz_5]);
        data[rug_fuzz_6][rug_fuzz_7] = json!(rug_fuzz_8);
        data[rug_fuzz_9][rug_fuzz_10][rug_fuzz_11][rug_fuzz_12] = json!(rug_fuzz_13);
        debug_assert_eq!(data[rug_fuzz_14], json!(1));
        let _rug_ed_tests_rug_642_rrrruuuugggg_test_index_mut = 0;
    }
}
