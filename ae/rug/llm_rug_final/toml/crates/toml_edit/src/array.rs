use std::iter::FromIterator;
use std::mem;

use crate::repr::Decor;
use crate::value::{DEFAULT_LEADING_VALUE_DECOR, DEFAULT_VALUE_DECOR};
use crate::{Item, RawString, Value};

/// Type representing a TOML array,
/// payload of the `Value::Array` variant's value
#[derive(Debug, Default, Clone)]
pub struct Array {
    // `trailing` represents whitespaces, newlines
    // and comments in an empty array or after the trailing comma
    trailing: RawString,
    trailing_comma: bool,
    // prefix before `[` and suffix after `]`
    decor: Decor,
    pub(crate) span: Option<std::ops::Range<usize>>,
    // always Vec<Item::Value>
    pub(crate) values: Vec<Item>,
}

/// An owned iterator type over `Table`'s key/value pairs.
pub type ArrayIntoIter = Box<dyn Iterator<Item = Value>>;
/// An iterator type over `Array`'s values.
pub type ArrayIter<'a> = Box<dyn Iterator<Item = &'a Value> + 'a>;
/// An iterator type over `Array`'s values.
pub type ArrayIterMut<'a> = Box<dyn Iterator<Item = &'a mut Value> + 'a>;

/// Constructors
///
/// See also `FromIterator`
impl Array {
    /// Create an empty `Array`
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut arr = toml_edit::Array::new();
    /// ```
    pub fn new() -> Self {
        Default::default()
    }

    pub(crate) fn with_vec(values: Vec<Item>) -> Self {
        Self {
            values,
            ..Default::default()
        }
    }
}

/// Formatting
impl Array {
    /// Auto formats the array.
    pub fn fmt(&mut self) {
        decorate_array(self);
    }

    /// Set whether the array will use a trailing comma
    pub fn set_trailing_comma(&mut self, yes: bool) {
        self.trailing_comma = yes;
    }

    /// Whether the array will use a trailing comma
    pub fn trailing_comma(&self) -> bool {
        self.trailing_comma
    }

    /// Set whitespace after last element
    pub fn set_trailing(&mut self, trailing: impl Into<RawString>) {
        self.trailing = trailing.into();
    }

    /// Whitespace after last element
    pub fn trailing(&self) -> &RawString {
        &self.trailing
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
    pub(crate) fn span(&self) -> Option<std::ops::Range<usize>> {
        self.span.clone()
    }

    pub(crate) fn despan(&mut self, input: &str) {
        self.span = None;
        self.decor.despan(input);
        self.trailing.despan(input);
        for value in &mut self.values {
            value.despan(input);
        }
    }
}

impl Array {
    /// Returns an iterator over all values.
    pub fn iter(&self) -> ArrayIter<'_> {
        Box::new(self.values.iter().filter_map(Item::as_value))
    }

    /// Returns an iterator over all values.
    pub fn iter_mut(&mut self) -> ArrayIterMut<'_> {
        Box::new(self.values.iter_mut().filter_map(Item::as_value_mut))
    }

    /// Returns the length of the underlying Vec.
    ///
    /// In some rare cases, placeholder elements will exist.  For a more accurate count, call
    /// `a.iter().count()`
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut arr = toml_edit::Array::new();
    /// arr.push(1);
    /// arr.push("foo");
    /// assert_eq!(arr.len(), 2);
    /// ```
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Return true iff `self.len() == 0`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut arr = toml_edit::Array::new();
    /// assert!(arr.is_empty());
    ///
    /// arr.push(1);
    /// arr.push("foo");
    /// assert!(! arr.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clears the array, removing all values. Keeps the allocated memory for reuse.
    pub fn clear(&mut self) {
        self.values.clear()
    }

    /// Returns a reference to the value at the given index, or `None` if the index is out of
    /// bounds.
    pub fn get(&self, index: usize) -> Option<&Value> {
        self.values.get(index).and_then(Item::as_value)
    }

    /// Returns a reference to the value at the given index, or `None` if the index is out of
    /// bounds.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Value> {
        self.values.get_mut(index).and_then(Item::as_value_mut)
    }

    /// Appends a new value to the end of the array, applying default formatting to it.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut arr = toml_edit::Array::new();
    /// arr.push(1);
    /// arr.push("foo");
    /// ```
    pub fn push<V: Into<Value>>(&mut self, v: V) {
        self.value_op(v.into(), true, |items, value| {
            items.push(Item::Value(value))
        })
    }

    /// Appends a new, already formatted value to the end of the array.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let formatted_value = "'literal'".parse::<toml_edit::Value>().unwrap();
    /// let mut arr = toml_edit::Array::new();
    /// arr.push_formatted(formatted_value);
    /// ```
    pub fn push_formatted(&mut self, v: Value) {
        self.values.push(Item::Value(v));
    }

    /// Inserts an element at the given position within the array, applying default formatting to
    /// it and shifting all values after it to the right.
    ///
    /// # Panics
    ///
    /// Panics if `index > len`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut arr = toml_edit::Array::new();
    /// arr.push(1);
    /// arr.push("foo");
    ///
    /// arr.insert(0, "start");
    /// ```
    pub fn insert<V: Into<Value>>(&mut self, index: usize, v: V) {
        self.value_op(v.into(), true, |items, value| {
            items.insert(index, Item::Value(value))
        })
    }

    /// Inserts an already formatted value at the given position within the array, shifting all
    /// values after it to the right.
    ///
    /// # Panics
    ///
    /// Panics if `index > len`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut arr = toml_edit::Array::new();
    /// arr.push(1);
    /// arr.push("foo");
    ///
    /// let formatted_value = "'start'".parse::<toml_edit::Value>().unwrap();
    /// arr.insert_formatted(0, formatted_value);
    /// ```
    pub fn insert_formatted(&mut self, index: usize, v: Value) {
        self.values.insert(index, Item::Value(v))
    }

    /// Replaces the element at the given position within the array, preserving existing formatting.
    ///
    /// # Panics
    ///
    /// Panics if `index >= len`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut arr = toml_edit::Array::new();
    /// arr.push(1);
    /// arr.push("foo");
    ///
    /// arr.replace(0, "start");
    /// ```
    pub fn replace<V: Into<Value>>(&mut self, index: usize, v: V) -> Value {
        // Read the existing value's decor and preserve it.
        let existing_decor = self
            .get(index)
            .unwrap_or_else(|| panic!("index {} out of bounds (len = {})", index, self.len()))
            .decor();
        let mut value = v.into();
        *value.decor_mut() = existing_decor.clone();
        self.replace_formatted(index, value)
    }

    /// Replaces the element at the given position within the array with an already formatted value.
    ///
    /// # Panics
    ///
    /// Panics if `index >= len`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut arr = toml_edit::Array::new();
    /// arr.push(1);
    /// arr.push("foo");
    ///
    /// let formatted_value = "'start'".parse::<toml_edit::Value>().unwrap();
    /// arr.replace_formatted(0, formatted_value);
    /// ```
    pub fn replace_formatted(&mut self, index: usize, v: Value) -> Value {
        match mem::replace(&mut self.values[index], Item::Value(v)) {
            Item::Value(old_value) => old_value,
            x => panic!("non-value item {:?} in an array", x),
        }
    }

    /// Removes the value at the given index.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut arr = toml_edit::Array::new();
    /// arr.push(1);
    /// arr.push("foo");
    ///
    /// arr.remove(0);
    /// assert_eq!(arr.len(), 1);
    /// ```
    pub fn remove(&mut self, index: usize) -> Value {
        let removed = self.values.remove(index);
        match removed {
            Item::Value(v) => v,
            x => panic!("non-value item {:?} in an array", x),
        }
    }

    fn value_op<T>(
        &mut self,
        v: Value,
        decorate: bool,
        op: impl FnOnce(&mut Vec<Item>, Value) -> T,
    ) -> T {
        let mut value = v;
        if !self.is_empty() && decorate {
            value.decorate(" ", "");
        } else if decorate {
            value.decorate("", "");
        }
        op(&mut self.values, value)
    }
}

impl std::fmt::Display for Array {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        crate::encode::Encode::encode(self, f, None, ("", ""))
    }
}

impl<V: Into<Value>> Extend<V> for Array {
    fn extend<T: IntoIterator<Item = V>>(&mut self, iter: T) {
        for value in iter {
            self.push_formatted(value.into());
        }
    }
}

impl<V: Into<Value>> FromIterator<V> for Array {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = V>,
    {
        let v = iter.into_iter().map(|a| Item::Value(a.into()));
        Array {
            values: v.collect(),
            ..Default::default()
        }
    }
}

impl IntoIterator for Array {
    type Item = Value;
    type IntoIter = ArrayIntoIter;

    fn into_iter(self) -> Self::IntoIter {
        Box::new(
            self.values
                .into_iter()
                .filter(|v| v.is_value())
                .map(|v| v.into_value().unwrap()),
        )
    }
}

impl<'s> IntoIterator for &'s Array {
    type Item = &'s Value;
    type IntoIter = ArrayIter<'s>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

fn decorate_array(array: &mut Array) {
    for (i, value) in array
        .values
        .iter_mut()
        .filter_map(Item::as_value_mut)
        .enumerate()
    {
        // [value1, value2, value3]
        if i == 0 {
            value.decorate(DEFAULT_LEADING_VALUE_DECOR.0, DEFAULT_LEADING_VALUE_DECOR.1);
        } else {
            value.decorate(DEFAULT_VALUE_DECOR.0, DEFAULT_VALUE_DECOR.1);
        }
    }
    // Since everything is now on the same line, remove trailing commas and whitespace.
    array.set_trailing_comma(false);
    array.set_trailing("");
}

#[cfg(test)]
mod tests_rug_396 {
    use super::*;
    use crate::array::Array;

    #[test]
    fn test_rug() {
        let mut arr: Array = Array::new();
    }
}
#[cfg(test)]
mod tests_rug_397 {
    use super::*;
    use crate::array::Array;
    use crate::item::Item;
    use std::vec::Vec;

    #[test]
    fn test_rug() {
        let mut p0: Vec<Item> = Vec::new();

        Array::with_vec(p0);
    }
}#[cfg(test)]
mod tests_rug_398 {
    use super::*;
    use crate::array::Array;
    use serde::de::Error;

    #[test]
    fn test_fmt() {
        let mut p0: Array = Array::new();

        Array::fmt(&mut p0);
    }
}#[cfg(test)]
mod tests_rug_399 {
    use super::*;
    use crate::array::Array;
    
    #[test]
    fn test_set_trailing_comma() {
        let mut p0: Array = Array::default();
        let p1: bool = true;
        
        p0.set_trailing_comma(p1);
        
        // Add assertions if needed
    }
}#[cfg(test)]
mod tests_rug_400 {
    use super::*;
    use crate::array::Array;
    use serde::de::Error;

    #[test]
    fn test_trailing_comma() {
        let mut p0: Array = Array::new();
        p0.trailing_comma();

        // Add assertions here
    }
}        #[cfg(test)]
        mod tests_rug_402 {
            use super::*;
            use crate::array::Array;
            use crate::RawString;
            
            #[test]
            fn test_trailing() {
                let p0: Array = Array::default();
                
                Array::trailing(&p0);
                
            }
        }#[cfg(test)]
mod tests_rug_403 {
    use super::*;
    use crate::array::Array;
    
    #[test]
    fn test_decor_mut() {
        let mut p0: Array = Array::default();

        Array::decor_mut(&mut p0);

    }
}#[cfg(test)]
mod tests_rug_404 {
    use super::*;
    use crate::array::Array;
    
    #[test]
    fn test_rug() {
        let mut p0: Array = Array::new();

        Array::decor(&p0);
    }
}#[cfg(test)]
mod tests_rug_405 {
    use super::*;
    use crate::array::Array;
    
    #[test]
    fn test_span() {
        let p0: Array = Array::new();

        Array::span(&p0);
    }
}#[cfg(test)]
mod tests_rug_406 {
    use super::*;
    use crate::array::Array;
    
    #[test]
    fn test_rug() {
        let mut p0: Array = Default::default();
        let p1: &str = "some input";
               
        p0.despan(p1);
    }
}
#[cfg(test)]
mod tests_rug_407 {
    use super::*;
    use crate::array::Array;

    #[test]
    fn test_iter() {
        let mut p0 = Array::default();

        Array::iter(&p0);
    }
}
#[cfg(test)]
mod tests_rug_408 {
    use super::*;
    use crate::array::Array;

    #[test]
    fn test_iter_mut() {
        // Construct the variable
        let mut p0: Array = Array::new();

        // Call the target function
        Array::iter_mut(&mut p0);

        // Assertions can be added here
    }
}#[cfg(test)]
mod tests_rug_409 {
    use super::*;
    use crate::array::Array;

    #[test]
    fn test_rug() {
        let mut p0: Array = Array::new();

        p0.push(1);
        p0.push("foo");

        assert_eq!(Array::len(&p0), 2);
    }
}#[cfg(test)]
mod tests_rug_410 {
    use super::*;
    use crate::array::Array;
    use crate::de::Error;
    use serde::de::Error as SerdeError;

    #[test]
    fn test_rug() {
        let mut p0: Array = Array::new();

        assert!(Array::is_empty(&p0));

        p0.push(1);
        p0.push("foo");
        assert!(!Array::is_empty(&p0));
    }
}                    
#[cfg(test)]
mod tests_rug_411 {
    use super::*;
    use crate::array::Array;
    use serde::de::Error;
    
    #[test]
    fn test_rug() {
        let mut p0: Array = Array::new();
        
        <Array>::clear(&mut p0);
        
        assert_eq!(p0.len(), 0);
    }
}
                              
#[cfg(test)]
mod tests_rug_412 {
    use super::*;
    use crate::array::Array;
    
    #[test]
    fn test_rug() {
        let mut p0 = Array::default();
        let mut p1 = 0;
        
        <Array>::get(&p0, p1);
    }
}
                        
#[cfg(test)]
mod tests_rug_413 {
    use super::*;
    use crate::array::Array;
    use crate::value::Value;
    
    #[test]
    fn test_rug() {
        let mut p0 = Array::new();
        p0.push(Value::from(1));
        p0.push(Value::from(2));
        p0.push(Value::from(3));
        let mut p1 = 1;

        Array::get_mut(&mut p0, p1);
        
    }
}
#[cfg(test)]
mod tests_rug_414 {
    use super::*;
    use crate::array::Array;
    
    #[test]
    fn test_array_push() {
        let mut arr = Array::new();
        arr.push(1);
        arr.push("foo");
        
        // Add assertions here
    }
}
#[cfg(test)]
mod tests_rug_415 {
    use super::*;
    use crate::array::Array;
    use crate::value::Value;
    
    #[test]
    fn test_rug() {
        let mut p0: Array = Array::new();
        let p1: Value = "'literal'".parse::<Value>().unwrap();

        Array::push_formatted(&mut p0, p1);

    }
}
#[cfg(test)]
mod tests_rug_416 {
    use super::*;
    use crate::{array::Array, value::Value};

    #[test]
    fn test_insert() {
        let mut arr = Array::new();
        arr.push(Value::from(1));
        arr.push(Value::from("foo"));

        arr.insert(0, "start");

        assert_eq!(arr.get(0).unwrap().as_str().unwrap(), "start");
    }
}        
#[cfg(test)]
mod tests_rug_417 {
    use super::*;
    use crate::array::Array;
    use crate::value::Value;
    
    #[test]
    fn test_insert_formatted() {
        let mut arr = Array::new();
        arr.push(1);
        arr.push("foo");
        
        let formatted_value = "'start'".parse::<Value>().unwrap();
        
        Array::insert_formatted(&mut arr, 0, formatted_value);
        
        // Add assertions here
    }
}
#[cfg(test)]
mod tests_rug_418 {
    use super::*;
    use crate::{Array, Value};

    #[test]
    fn test_rug() {
        let mut p0: Array = Array::new();
        p0.push(1);
        p0.push("foo");

        let p1: usize = 0;
        let p2: Value = "start".into();

        p0.replace(p1, p2);
    }
}
#[cfg(test)]
mod tests_rug_419 {
    use super::*;
    use crate::{array::Array, value::Value};
    
    #[test]
    fn test_replace_formatted() {
        let mut arr = Array::new();
        arr.push(1);
        arr.push("foo");
        
        let index = 0;
        
        let formatted_value = "'start'".parse::<Value>().unwrap();
        
        arr.replace_formatted(index, formatted_value);
    }
}                        
#[cfg(test)]
mod tests_rug_420 {
    use super::*;
    use crate::array::Array; // Importing the Array struct from the crate
    
    #[test]
    fn test_remove() {
        let mut arr = Array::new(); // Creating a new Array
        arr.push(1); // Pushing a value
        arr.push("foo");

        let index = 0; // Defining the index

        arr.remove(index); // Calling the remove function on the Array

        assert_eq!(arr.len(), 1); // Asserting the length of the Array
    }
}#[cfg(test)]
mod tests_rug_421 {
    use super::*;
    use crate::array::Array;
    use crate::array::Item;
    use crate::value::Value;

    #[test]
    fn test_rug() {
        let mut p0: Array = Array::new();
        let mut p1: Value = Value::from("sample");
        let mut p2: bool = true;
        let mut p3 = |vec: &mut Vec<Item>, val: Value| {
            vec.push(Item::Value(val));
            vec.len()
        };

        Array::value_op(&mut p0, p1, p2, &mut p3);
    }
}#[cfg(test)]
mod tests_rug_424 {
    use super::*;
    use crate::array::Array;
    
    #[test]
    fn test_into_iter() {
        let mut p0: Array = Array::default();

        p0.into_iter();
    }
}