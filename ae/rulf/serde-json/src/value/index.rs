use super::Value;
use crate::lib::*;
use crate::map::Map;
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
        match *v {
            Value::Array(ref vec) => vec.get(*self),
            _ => None,
        }
    }
    fn index_into_mut<'v>(&self, v: &'v mut Value) -> Option<&'v mut Value> {
        match *v {
            Value::Array(ref mut vec) => vec.get_mut(*self),
            _ => None,
        }
    }
    fn index_or_insert<'v>(&self, v: &'v mut Value) -> &'v mut Value {
        match *v {
            Value::Array(ref mut vec) => {
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
        match *v {
            Value::Object(ref map) => map.get(self),
            _ => None,
        }
    }
    fn index_into_mut<'v>(&self, v: &'v mut Value) -> Option<&'v mut Value> {
        match *v {
            Value::Object(ref mut map) => map.get_mut(self),
            _ => None,
        }
    }
    fn index_or_insert<'v>(&self, v: &'v mut Value) -> &'v mut Value {
        if let Value::Null = *v {
            *v = Value::Object(Map::new());
        }
        match *v {
            Value::Object(ref mut map) => {
                map.entry(self.to_owned()).or_insert(Value::Null)
            }
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
    impl Sealed for super::String {}
    impl<'a, T> Sealed for &'a T
    where
        T: ?Sized + Sealed,
    {}
}
/// Used in panic messages.
struct Type<'a>(&'a Value);
impl<'a> fmt::Display for Type<'a> {
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
mod tests_rug_532 {
    use super::*;
    use crate::value::Index;
    use crate::{Number, Value, Map};
    #[test]
    fn test_index_into() {
        let _rug_st_tests_rug_532_rrrruuuugggg_test_index_into = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = "key";
        let rug_fuzz_2 = 42;
        let p0: usize = rug_fuzz_0;
        let mut v29 = Value::Object(Map::new());
        v29[rug_fuzz_1] = Value::Number(Number::from(rug_fuzz_2));
        let p1: &Value = &v29;
        <usize as Index>::index_into(&p0, p1);
        let _rug_ed_tests_rug_532_rrrruuuugggg_test_index_into = 0;
    }
}
#[cfg(test)]
mod tests_rug_533 {
    use super::*;
    use crate::value::Index;
    use crate::{Number, Value, Map};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_533_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = "key";
        let rug_fuzz_2 = 42;
        let mut p0: usize = rug_fuzz_0;
        let mut v29 = Value::Object(Map::new());
        v29[rug_fuzz_1] = Value::Number(Number::from(rug_fuzz_2));
        let mut p1 = &mut v29 as &mut Value;
        <usize>::index_into_mut(&p0, &mut p1);
        let _rug_ed_tests_rug_533_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_534 {
    use super::*;
    use crate::value::Index;
    use crate::{Number, Value, Map};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_534_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0usize;
        let rug_fuzz_1 = "key";
        let rug_fuzz_2 = 42;
        let mut p0 = rug_fuzz_0;
        let mut p1 = {
            let mut v29 = Value::Object(Map::new());
            v29[rug_fuzz_1] = Value::Number(Number::from(rug_fuzz_2));
            v29
        };
        <usize>::index_or_insert(&p0, &mut p1);
        let _rug_ed_tests_rug_534_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_535 {
    use super::*;
    use crate::value::Index;
    use crate::{Number, Value, Map};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_535_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = "key";
        let rug_fuzz_2 = 42;
        let mut p0: &str = rug_fuzz_0;
        let mut p1 = {
            let mut v29 = Value::Object(Map::new());
            v29[rug_fuzz_1] = Value::Number(Number::from(rug_fuzz_2));
            v29
        };
        p0.index_into(&p1);
        let _rug_ed_tests_rug_535_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_536 {
    use super::*;
    use crate::value::Index;
    use crate::{Number, Value, Map};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_536_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = "key";
        let rug_fuzz_2 = 42;
        let mut p0: &str = rug_fuzz_0;
        let mut p1 = {
            let mut v29 = Value::Object(Map::new());
            v29[rug_fuzz_1] = Value::Number(Number::from(rug_fuzz_2));
            v29
        };
        <str>::index_into_mut(&p0, &mut p1);
        let _rug_ed_tests_rug_536_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_537 {
    use super::*;
    use crate::value::Index;
    use crate::{Number, Value, Map};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_537_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key";
        #[allow(unused_mut)]
        let mut p0: &str = rug_fuzz_0;
        #[allow(unused_mut)]
        let mut p1 = &mut Value::Object(Map::new());
        if let Value::Null = *p1 {
            *p1 = Value::Object(Map::new());
        }
        match *p1 {
            Value::Object(ref mut map) => map.entry(p0.to_owned()).or_insert(Value::Null),
            _ => panic!("cannot access key {:?} in JSON {}", p0, Type(p1)),
        };
        let _rug_ed_tests_rug_537_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_538 {
    use super::*;
    use crate::value::Index;
    use crate::{Number, Value, Map};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_538_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "p0_sample";
        let rug_fuzz_1 = "key";
        let rug_fuzz_2 = 42;
        let mut p0: std::string::String = String::from(rug_fuzz_0);
        let mut p1: Value = {
            let mut v29 = Value::Object(Map::new());
            v29[rug_fuzz_1] = Value::Number(Number::from(rug_fuzz_2));
            v29
        };
        p0.index_into(&p1);
        let _rug_ed_tests_rug_538_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_539 {
    use super::*;
    use crate::value::Index;
    use crate::{Number, Value, Map};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_539_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "some_string";
        let rug_fuzz_1 = "key";
        let rug_fuzz_2 = 42;
        let p0 = String::from(rug_fuzz_0);
        let mut p1 = Value::Object(Map::new());
        p1[rug_fuzz_1] = Value::Number(Number::from(rug_fuzz_2));
        <String as Index>::index_into_mut(&p0, &mut p1);
        let _rug_ed_tests_rug_539_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_540 {
    use super::*;
    use crate::value::Index;
    use crate::{Number, Value, Map};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_540_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample";
        let rug_fuzz_1 = "key";
        let rug_fuzz_2 = 42;
        let mut p0 = String::from(rug_fuzz_0);
        let mut p1 = Value::Object(Map::new());
        p1[rug_fuzz_1] = Value::Number(Number::from(rug_fuzz_2));
        <String as crate::value::index::Index>::index_or_insert(&p0, &mut p1);
        let _rug_ed_tests_rug_540_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_541 {
    use super::*;
    use crate::{Number, Value, Map};
    use crate::value::Index;
    #[test]
    fn test_index_into() {
        let _rug_st_tests_rug_541_rrrruuuugggg_test_index_into = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let mut p0: String = String::new();
        let mut p1 = Value::Object(Map::new());
        p1[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
        p0.index_into(&p1);
        let _rug_ed_tests_rug_541_rrrruuuugggg_test_index_into = 0;
    }
}
#[cfg(test)]
mod tests_rug_542 {
    use super::*;
    use crate::value::Index;
    use std::string::String;
    use crate::{Number, Value, Map};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_542_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let mut p0: String = String::new();
        let mut p1 = Value::Object(Map::new());
        p1[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
        p0.index_into_mut(&mut p1);
        let _rug_ed_tests_rug_542_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_543 {
    use super::*;
    use crate::value::Index;
    use std::string::String;
    use crate::{Number, Value, Map};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_543_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let mut p0: String = String::new();
        let mut p1 = Value::Object(Map::new());
        p1[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
        p0.index_or_insert(&mut p1);
        let _rug_ed_tests_rug_543_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_545 {
    use super::*;
    use crate::{Value, Number, Map};
    use std::ops::IndexMut;
    #[test]
    fn test_index_mut() {
        let _rug_st_tests_rug_545_rrrruuuugggg_test_index_mut = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let mut v: Value = Value::Object(Map::new());
        v[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
        let mut index: String = String::new();
        v.index_mut(index);
        let _rug_ed_tests_rug_545_rrrruuuugggg_test_index_mut = 0;
    }
}
