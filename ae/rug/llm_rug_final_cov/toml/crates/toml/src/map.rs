//! A map of `String` to [Value].
//!
//! By default the map is backed by a [`BTreeMap`]. Enable the `preserve_order`
//! feature of toml-rs to use [`IndexMap`] instead.
//!
//! [`BTreeMap`]: https://doc.rust-lang.org/std/collections/struct.BTreeMap.html
//! [`IndexMap`]: https://docs.rs/indexmap
use crate::value::Value;
use serde::{de, ser};
use std::borrow::Borrow;
use std::fmt::{self, Debug};
use std::hash::Hash;
use std::iter::FromIterator;
use std::ops;
#[cfg(not(feature = "preserve_order"))]
use std::collections::{btree_map, BTreeMap};
#[cfg(feature = "preserve_order")]
use indexmap::{self, IndexMap};
/// Represents a TOML key/value type.
pub struct Map<K, V> {
    map: MapImpl<K, V>,
}
#[cfg(not(feature = "preserve_order"))]
type MapImpl<K, V> = BTreeMap<K, V>;
#[cfg(feature = "preserve_order")]
type MapImpl<K, V> = IndexMap<K, V>;
impl Map<String, Value> {
    /// Makes a new empty Map.
    #[inline]
    pub fn new() -> Self {
        Map { map: MapImpl::new() }
    }
    #[cfg(not(feature = "preserve_order"))]
    /// Makes a new empty Map with the given initial capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        let _ = capacity;
        Map { map: BTreeMap::new() }
    }
    #[cfg(feature = "preserve_order")]
    /// Makes a new empty Map with the given initial capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Map {
            map: IndexMap::with_capacity(capacity),
        }
    }
    /// Clears the map, removing all values.
    #[inline]
    pub fn clear(&mut self) {
        self.map.clear()
    }
    /// Returns a reference to the value corresponding to the key.
    ///
    /// The key may be any borrowed form of the map's key type, but the ordering
    /// on the borrowed form *must* match the ordering on the key type.
    #[inline]
    pub fn get<Q: ?Sized>(&self, key: &Q) -> Option<&Value>
    where
        String: Borrow<Q>,
        Q: Ord + Eq + Hash,
    {
        self.map.get(key)
    }
    /// Returns true if the map contains a value for the specified key.
    ///
    /// The key may be any borrowed form of the map's key type, but the ordering
    /// on the borrowed form *must* match the ordering on the key type.
    #[inline]
    pub fn contains_key<Q: ?Sized>(&self, key: &Q) -> bool
    where
        String: Borrow<Q>,
        Q: Ord + Eq + Hash,
    {
        self.map.contains_key(key)
    }
    /// Returns a mutable reference to the value corresponding to the key.
    ///
    /// The key may be any borrowed form of the map's key type, but the ordering
    /// on the borrowed form *must* match the ordering on the key type.
    #[inline]
    pub fn get_mut<Q: ?Sized>(&mut self, key: &Q) -> Option<&mut Value>
    where
        String: Borrow<Q>,
        Q: Ord + Eq + Hash,
    {
        self.map.get_mut(key)
    }
    /// Inserts a key-value pair into the map.
    ///
    /// If the map did not have this key present, `None` is returned.
    ///
    /// If the map did have this key present, the value is updated, and the old
    /// value is returned. The key is not updated, though; this matters for
    /// types that can be `==` without being identical.
    #[inline]
    pub fn insert(&mut self, k: String, v: Value) -> Option<Value> {
        self.map.insert(k, v)
    }
    /// Removes a key from the map, returning the value at the key if the key
    /// was previously in the map.
    ///
    /// The key may be any borrowed form of the map's key type, but the ordering
    /// on the borrowed form *must* match the ordering on the key type.
    #[inline]
    pub fn remove<Q: ?Sized>(&mut self, key: &Q) -> Option<Value>
    where
        String: Borrow<Q>,
        Q: Ord + Eq + Hash,
    {
        self.map.remove(key)
    }
    /// Gets the given key's corresponding entry in the map for in-place
    /// manipulation.
    pub fn entry<S>(&mut self, key: S) -> Entry<'_>
    where
        S: Into<String>,
    {
        #[cfg(feature = "preserve_order")]
        use indexmap::map::Entry as EntryImpl;
        #[cfg(not(feature = "preserve_order"))]
        use std::collections::btree_map::Entry as EntryImpl;
        match self.map.entry(key.into()) {
            EntryImpl::Vacant(vacant) => Entry::Vacant(VacantEntry { vacant }),
            EntryImpl::Occupied(occupied) => Entry::Occupied(OccupiedEntry { occupied }),
        }
    }
    /// Returns the number of elements in the map.
    #[inline]
    pub fn len(&self) -> usize {
        self.map.len()
    }
    /// Returns true if the map contains no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
    /// Gets an iterator over the entries of the map.
    #[inline]
    pub fn iter(&self) -> Iter<'_> {
        Iter { iter: self.map.iter() }
    }
    /// Gets a mutable iterator over the entries of the map.
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_> {
        IterMut {
            iter: self.map.iter_mut(),
        }
    }
    /// Gets an iterator over the keys of the map.
    #[inline]
    pub fn keys(&self) -> Keys<'_> {
        Keys { iter: self.map.keys() }
    }
    /// Gets an iterator over the values of the map.
    #[inline]
    pub fn values(&self) -> Values<'_> {
        Values { iter: self.map.values() }
    }
}
impl Default for Map<String, Value> {
    #[inline]
    fn default() -> Self {
        Map { map: MapImpl::new() }
    }
}
impl Clone for Map<String, Value> {
    #[inline]
    fn clone(&self) -> Self {
        Map { map: self.map.clone() }
    }
}
impl PartialEq for Map<String, Value> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.map.eq(&other.map)
    }
}
/// Access an element of this map. Panics if the given key is not present in the
/// map.
impl<'a, Q: ?Sized> ops::Index<&'a Q> for Map<String, Value>
where
    String: Borrow<Q>,
    Q: Ord + Eq + Hash,
{
    type Output = Value;
    fn index(&self, index: &Q) -> &Value {
        self.map.index(index)
    }
}
/// Mutably access an element of this map. Panics if the given key is not
/// present in the map.
impl<'a, Q: ?Sized> ops::IndexMut<&'a Q> for Map<String, Value>
where
    String: Borrow<Q>,
    Q: Ord + Eq + Hash,
{
    fn index_mut(&mut self, index: &Q) -> &mut Value {
        self.map.get_mut(index).expect("no entry found for key")
    }
}
impl Debug for Map<String, Value> {
    #[inline]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        self.map.fmt(formatter)
    }
}
impl ser::Serialize for Map<String, Value> {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.len()))?;
        for (k, v) in self {
            map.serialize_key(k)?;
            map.serialize_value(v)?;
        }
        map.end()
    }
}
impl<'de> de::Deserialize<'de> for Map<String, Value> {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Map<String, Value>;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a map")
            }
            #[inline]
            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Map::new())
            }
            #[inline]
            fn visit_map<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                let mut values = Map::new();
                while let Some((key, value)) = visitor.next_entry()? {
                    values.insert(key, value);
                }
                Ok(values)
            }
        }
        deserializer.deserialize_map(Visitor)
    }
}
impl FromIterator<(String, Value)> for Map<String, Value> {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = (String, Value)>,
    {
        Map {
            map: FromIterator::from_iter(iter),
        }
    }
}
impl Extend<(String, Value)> for Map<String, Value> {
    fn extend<T>(&mut self, iter: T)
    where
        T: IntoIterator<Item = (String, Value)>,
    {
        self.map.extend(iter);
    }
}
macro_rules! delegate_iterator {
    (($name:ident $($generics:tt)*) => $item:ty) => {
        impl $($generics)* Iterator for $name $($generics)* { type Item = $item;
        #[inline] fn next(& mut self) -> Option < Self::Item > { self.iter.next() }
        #[inline] fn size_hint(& self) -> (usize, Option < usize >) { self.iter
        .size_hint() } } impl $($generics)* DoubleEndedIterator for $name $($generics)* {
        #[inline] fn next_back(& mut self) -> Option < Self::Item > { self.iter
        .next_back() } } impl $($generics)* ExactSizeIterator for $name $($generics)* {
        #[inline] fn len(& self) -> usize { self.iter.len() } }
    };
}
/// A view into a single entry in a map, which may either be vacant or occupied.
/// This enum is constructed from the [`entry`] method on [`Map`].
///
/// [`entry`]: struct.Map.html#method.entry
/// [`Map`]: struct.Map.html
pub enum Entry<'a> {
    /// A vacant Entry.
    Vacant(VacantEntry<'a>),
    /// An occupied Entry.
    Occupied(OccupiedEntry<'a>),
}
/// A vacant Entry. It is part of the [`Entry`] enum.
///
/// [`Entry`]: enum.Entry.html
pub struct VacantEntry<'a> {
    vacant: VacantEntryImpl<'a>,
}
/// An occupied Entry. It is part of the [`Entry`] enum.
///
/// [`Entry`]: enum.Entry.html
pub struct OccupiedEntry<'a> {
    occupied: OccupiedEntryImpl<'a>,
}
#[cfg(not(feature = "preserve_order"))]
type VacantEntryImpl<'a> = btree_map::VacantEntry<'a, String, Value>;
#[cfg(feature = "preserve_order")]
type VacantEntryImpl<'a> = indexmap::map::VacantEntry<'a, String, Value>;
#[cfg(not(feature = "preserve_order"))]
type OccupiedEntryImpl<'a> = btree_map::OccupiedEntry<'a, String, Value>;
#[cfg(feature = "preserve_order")]
type OccupiedEntryImpl<'a> = indexmap::map::OccupiedEntry<'a, String, Value>;
impl<'a> Entry<'a> {
    /// Returns a reference to this entry's key.
    pub fn key(&self) -> &String {
        match *self {
            Entry::Vacant(ref e) => e.key(),
            Entry::Occupied(ref e) => e.key(),
        }
    }
    /// Ensures a value is in the entry by inserting the default if empty, and
    /// returns a mutable reference to the value in the entry.
    pub fn or_insert(self, default: Value) -> &'a mut Value {
        match self {
            Entry::Vacant(entry) => entry.insert(default),
            Entry::Occupied(entry) => entry.into_mut(),
        }
    }
    /// Ensures a value is in the entry by inserting the result of the default
    /// function if empty, and returns a mutable reference to the value in the
    /// entry.
    pub fn or_insert_with<F>(self, default: F) -> &'a mut Value
    where
        F: FnOnce() -> Value,
    {
        match self {
            Entry::Vacant(entry) => entry.insert(default()),
            Entry::Occupied(entry) => entry.into_mut(),
        }
    }
}
impl<'a> VacantEntry<'a> {
    /// Gets a reference to the key that would be used when inserting a value
    /// through the VacantEntry.
    #[inline]
    pub fn key(&self) -> &String {
        self.vacant.key()
    }
    /// Sets the value of the entry with the VacantEntry's key, and returns a
    /// mutable reference to it.
    #[inline]
    pub fn insert(self, value: Value) -> &'a mut Value {
        self.vacant.insert(value)
    }
}
impl<'a> OccupiedEntry<'a> {
    /// Gets a reference to the key in the entry.
    #[inline]
    pub fn key(&self) -> &String {
        self.occupied.key()
    }
    /// Gets a reference to the value in the entry.
    #[inline]
    pub fn get(&self) -> &Value {
        self.occupied.get()
    }
    /// Gets a mutable reference to the value in the entry.
    #[inline]
    pub fn get_mut(&mut self) -> &mut Value {
        self.occupied.get_mut()
    }
    /// Converts the entry into a mutable reference to its value.
    #[inline]
    pub fn into_mut(self) -> &'a mut Value {
        self.occupied.into_mut()
    }
    /// Sets the value of the entry with the `OccupiedEntry`'s key, and returns
    /// the entry's old value.
    #[inline]
    pub fn insert(&mut self, value: Value) -> Value {
        self.occupied.insert(value)
    }
    /// Takes the value of the entry out of the map, and returns it.
    #[inline]
    pub fn remove(self) -> Value {
        self.occupied.remove()
    }
}
impl<'a> IntoIterator for &'a Map<String, Value> {
    type Item = (&'a String, &'a Value);
    type IntoIter = Iter<'a>;
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        Iter { iter: self.map.iter() }
    }
}
/// An iterator over a toml::Map's entries.
pub struct Iter<'a> {
    iter: IterImpl<'a>,
}
#[cfg(not(feature = "preserve_order"))]
type IterImpl<'a> = btree_map::Iter<'a, String, Value>;
#[cfg(feature = "preserve_order")]
type IterImpl<'a> = indexmap::map::Iter<'a, String, Value>;
delegate_iterator!((Iter <'a >) => (&'a String, &'a Value));
impl<'a> IntoIterator for &'a mut Map<String, Value> {
    type Item = (&'a String, &'a mut Value);
    type IntoIter = IterMut<'a>;
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        IterMut {
            iter: self.map.iter_mut(),
        }
    }
}
/// A mutable iterator over a toml::Map's entries.
pub struct IterMut<'a> {
    iter: IterMutImpl<'a>,
}
#[cfg(not(feature = "preserve_order"))]
type IterMutImpl<'a> = btree_map::IterMut<'a, String, Value>;
#[cfg(feature = "preserve_order")]
type IterMutImpl<'a> = indexmap::map::IterMut<'a, String, Value>;
delegate_iterator!((IterMut <'a >) => (&'a String, &'a mut Value));
impl IntoIterator for Map<String, Value> {
    type Item = (String, Value);
    type IntoIter = IntoIter;
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            iter: self.map.into_iter(),
        }
    }
}
/// An owning iterator over a toml::Map's entries.
pub struct IntoIter {
    iter: IntoIterImpl,
}
#[cfg(not(feature = "preserve_order"))]
type IntoIterImpl = btree_map::IntoIter<String, Value>;
#[cfg(feature = "preserve_order")]
type IntoIterImpl = indexmap::map::IntoIter<String, Value>;
delegate_iterator!((IntoIter) => (String, Value));
/// An iterator over a toml::Map's keys.
pub struct Keys<'a> {
    iter: KeysImpl<'a>,
}
#[cfg(not(feature = "preserve_order"))]
type KeysImpl<'a> = btree_map::Keys<'a, String, Value>;
#[cfg(feature = "preserve_order")]
type KeysImpl<'a> = indexmap::map::Keys<'a, String, Value>;
delegate_iterator!((Keys <'a >) => &'a String);
/// An iterator over a toml::Map's values.
pub struct Values<'a> {
    iter: ValuesImpl<'a>,
}
#[cfg(not(feature = "preserve_order"))]
type ValuesImpl<'a> = btree_map::Values<'a, String, Value>;
#[cfg(feature = "preserve_order")]
type ValuesImpl<'a> = indexmap::map::Values<'a, String, Value>;
delegate_iterator!((Values <'a >) => &'a Value);
#[cfg(test)]
mod tests_rug_317 {
    use super::*;
    use std::string::String;
    use crate::value::Value;
    use crate::map::Map;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_317_rrrruuuugggg_test_rug = 0;
        let result: Map<String, Value> = Map::<String, Value>::new();
        let _rug_ed_tests_rug_317_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_318 {
    use super::*;
    use std::collections::BTreeMap;
    use std::string::String;
    use crate::value::Value;
    #[test]
    fn test_with_capacity() {
        let _rug_st_tests_rug_318_rrrruuuugggg_test_with_capacity = 0;
        let rug_fuzz_0 = 10;
        let p0: usize = rug_fuzz_0;
        let _ = Map::<String, Value>::with_capacity(p0);
        let _rug_ed_tests_rug_318_rrrruuuugggg_test_with_capacity = 0;
    }
}
#[cfg(test)]
mod tests_rug_319 {
    use super::*;
    use crate::map::Map;
    use crate::value::Value;
    use std::string::String;
    #[test]
    fn test_clear() {
        let _rug_st_tests_rug_319_rrrruuuugggg_test_clear = 0;
        let mut p0: Map<String, Value> = Map::new();
        Map::<String, Value>::clear(&mut p0);
        let _rug_ed_tests_rug_319_rrrruuuugggg_test_clear = 0;
    }
}
#[cfg(test)]
mod tests_rug_320 {
    use super::*;
    #[test]
    fn test_get() {
        let _rug_st_tests_rug_320_rrrruuuugggg_test_get = 0;
        let rug_fuzz_0 = "key1";
        let rug_fuzz_1 = "value1";
        let rug_fuzz_2 = "key2";
        let rug_fuzz_3 = 42;
        let rug_fuzz_4 = "key1";
        let mut p0 = Map::<String, Value>::new();
        p0.insert(rug_fuzz_0.to_string(), Value::String(rug_fuzz_1.to_string()));
        p0.insert(rug_fuzz_2.to_string(), Value::Integer(rug_fuzz_3));
        let p1: &str = rug_fuzz_4;
        p0.get(p1);
        let _rug_ed_tests_rug_320_rrrruuuugggg_test_get = 0;
    }
}
#[cfg(test)]
mod tests_rug_321 {
    use super::*;
    use crate::map::Map;
    use crate::value::Value;
    use std::string::String;
    #[test]
    fn test_contains_key() {
        let _rug_st_tests_rug_321_rrrruuuugggg_test_contains_key = 0;
        let rug_fuzz_0 = "key1";
        let rug_fuzz_1 = 10;
        let rug_fuzz_2 = "key2";
        let rug_fuzz_3 = true;
        let rug_fuzz_4 = "key1";
        let mut p0: Map<String, Value> = Map::new();
        p0.insert(String::from(rug_fuzz_0), Value::Integer(rug_fuzz_1));
        p0.insert(String::from(rug_fuzz_2), Value::Boolean(rug_fuzz_3));
        let p1: &str = rug_fuzz_4;
        debug_assert_eq!(< Map < String, Value > > ::contains_key(& p0, p1), true);
        let _rug_ed_tests_rug_321_rrrruuuugggg_test_contains_key = 0;
    }
}
#[cfg(test)]
mod tests_rug_323 {
    use super::*;
    use std::string::String;
    use crate::map::Map;
    use crate::value::Value;
    #[test]
    fn test_insert() {
        let _rug_st_tests_rug_323_rrrruuuugggg_test_insert = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = "value";
        let mut p0: Map<String, Value> = Map::new();
        let p1: String = String::from(rug_fuzz_0);
        let p2: Value = Value::from(rug_fuzz_1);
        p0.insert(p1, p2);
        let _rug_ed_tests_rug_323_rrrruuuugggg_test_insert = 0;
    }
}
#[cfg(test)]
mod tests_rug_324 {
    use super::*;
    use std::collections::HashMap;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_324_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key1";
        let rug_fuzz_1 = "value1";
        let rug_fuzz_2 = "key2";
        let rug_fuzz_3 = "value2";
        let rug_fuzz_4 = "key1";
        let mut p0: Map<String, Value> = Map::new();
        p0.insert(rug_fuzz_0.to_owned(), Value::String(rug_fuzz_1.to_owned()));
        p0.insert(rug_fuzz_2.to_owned(), Value::String(rug_fuzz_3.to_owned()));
        let p1: String = rug_fuzz_4.to_owned();
        p0.remove(&p1);
        let _rug_ed_tests_rug_324_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_325 {
    use super::*;
    use crate::map::*;
    use crate::value::*;
    #[cfg(feature = "preserve_order")]
    use indexmap::map::Entry as EntryImpl;
    #[cfg(not(feature = "preserve_order"))]
    use std::collections::btree_map::Entry as EntryImpl;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_325_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key";
        let mut p0 = Map::<String, Value>::new();
        let p1: String = rug_fuzz_0.into();
        Map::<String, Value>::entry(&mut p0, p1);
        let _rug_ed_tests_rug_325_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_326 {
    use super::*;
    use crate::map::Map;
    use crate::value::Value;
    use std::string::String;
    #[test]
    fn test_len() {
        let _rug_st_tests_rug_326_rrrruuuugggg_test_len = 0;
        let rug_fuzz_0 = "key1";
        let rug_fuzz_1 = "value1";
        let rug_fuzz_2 = "key2";
        let rug_fuzz_3 = 42;
        let mut p0 = Map::<String, Value>::new();
        p0.insert(rug_fuzz_0.to_string(), Value::String(rug_fuzz_1.to_string()));
        p0.insert(rug_fuzz_2.to_string(), Value::Integer(rug_fuzz_3));
        debug_assert_eq!(< Map < String, Value > > ::len(& p0), 2);
        let _rug_ed_tests_rug_326_rrrruuuugggg_test_len = 0;
    }
}
#[cfg(test)]
mod tests_rug_327 {
    use super::*;
    use crate::map::Map;
    use crate::value::Value;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_327_rrrruuuugggg_test_rug = 0;
        let mut p0: Map<String, Value> = Map::new();
        debug_assert!(p0.is_empty());
        let _rug_ed_tests_rug_327_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_328 {
    use super::*;
    use crate::value::Value;
    use std::collections::BTreeMap;
    use std::str::FromStr;
    use crate::map::Map;
    #[test]
    fn test_iter() {
        let _rug_st_tests_rug_328_rrrruuuugggg_test_iter = 0;
        let rug_fuzz_0 = "key1";
        let rug_fuzz_1 = "value1";
        let rug_fuzz_2 = "key2";
        let rug_fuzz_3 = "value2";
        let mut p0 = Map::<String, Value>::new();
        p0.insert(rug_fuzz_0.to_string(), Value::from_str(rug_fuzz_1).unwrap());
        p0.insert(rug_fuzz_2.to_string(), Value::from_str(rug_fuzz_3).unwrap());
        Map::<String, Value>::iter(&p0);
        let _rug_ed_tests_rug_328_rrrruuuugggg_test_iter = 0;
    }
}
#[cfg(test)]
mod tests_rug_329 {
    use super::*;
    use crate::map::Map;
    use std::string::String;
    use crate::value::Value;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_329_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key1";
        let rug_fuzz_1 = 10;
        let rug_fuzz_2 = "key2";
        let rug_fuzz_3 = 3.14;
        let rug_fuzz_4 = "key3";
        let rug_fuzz_5 = true;
        let mut p0: Map<String, Value> = Map::new();
        p0.insert(rug_fuzz_0.to_string(), Value::Integer(rug_fuzz_1));
        p0.insert(rug_fuzz_2.to_string(), Value::Float(rug_fuzz_3));
        p0.insert(rug_fuzz_4.to_string(), Value::Boolean(rug_fuzz_5));
        Map::<String, Value>::iter_mut(&mut p0);
        let _rug_ed_tests_rug_329_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_330 {
    use super::*;
    use crate::map::Map;
    use crate::value::Value;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_330_rrrruuuugggg_test_rug = 0;
        let mut p0: Map<String, Value> = Map::new();
        Map::<String, Value>::keys(&p0);
        let _rug_ed_tests_rug_330_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_331 {
    use super::*;
    use std::string::String;
    use crate::map::Map;
    use crate::value::Value;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_331_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key1";
        let rug_fuzz_1 = "value1";
        let rug_fuzz_2 = "key2";
        let rug_fuzz_3 = 100;
        let mut p0: Map<String, Value> = Map::new();
        p0.insert(String::from(rug_fuzz_0), Value::String(String::from(rug_fuzz_1)));
        p0.insert(String::from(rug_fuzz_2), Value::Integer(rug_fuzz_3));
        Map::<String, Value>::values(&p0);
        let _rug_ed_tests_rug_331_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_332 {
    use super::*;
    use crate::map::Map;
    use crate::value::Value;
    use std::default::Default;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_332_rrrruuuugggg_test_rug = 0;
        let map: Map<String, Value> = Map::<String, Value>::default();
        debug_assert_eq!(map.map.len(), 0);
        let _rug_ed_tests_rug_332_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_333 {
    use super::*;
    use crate::map::Map;
    use crate::value::Value;
    use std::clone::Clone;
    #[test]
    fn test_clone() {
        let _rug_st_tests_rug_333_rrrruuuugggg_test_clone = 0;
        let rug_fuzz_0 = "key1";
        let rug_fuzz_1 = "value1";
        let rug_fuzz_2 = "key2";
        let rug_fuzz_3 = 100;
        let mut p0 = Map::<String, Value>::new();
        p0.insert(rug_fuzz_0.to_owned(), Value::String(rug_fuzz_1.to_owned()));
        p0.insert(rug_fuzz_2.to_owned(), Value::Integer(rug_fuzz_3));
        p0.clone();
        let _rug_ed_tests_rug_333_rrrruuuugggg_test_clone = 0;
    }
}
#[cfg(test)]
mod tests_rug_334 {
    use super::*;
    use crate::map::Map;
    use crate::value::Value;
    use std::cmp::PartialEq;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_334_rrrruuuugggg_test_rug = 0;
        let mut p0 = Map::<std::string::String, Value>::new();
        let mut p1 = Map::<std::string::String, Value>::new();
        <Map<std::string::String, Value> as PartialEq>::eq(&p0, &p1);
        let _rug_ed_tests_rug_334_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_343 {
    use super::*;
    use std::iter::Extend;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_343_rrrruuuugggg_test_rug = 0;
        let mut p0 = Map::<String, Value>::default();
        let mut p1 = Map::<String, Value>::default();
        p0.extend(p1);
        let _rug_ed_tests_rug_343_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_344 {
    use super::*;
    use crate::map::Entry;
    #[test]
    fn test_key() {
        let _rug_st_tests_rug_344_rrrruuuugggg_test_key = 0;
        let p0: Entry<'static> = unimplemented!();
        p0.key();
        let _rug_ed_tests_rug_344_rrrruuuugggg_test_key = 0;
    }
}
#[cfg(test)]
mod tests_rug_345 {
    use super::*;
    use crate::map::Entry;
    use crate::value::Value;
    #[test]
    fn test_or_insert() {
        let _rug_st_tests_rug_345_rrrruuuugggg_test_or_insert = 0;
        let mut p0: Entry<'static> = unimplemented!();
        let p1: Value = unimplemented!();
        p0.or_insert(p1);
        let _rug_ed_tests_rug_345_rrrruuuugggg_test_or_insert = 0;
    }
}
#[cfg(test)]
mod tests_rug_348 {
    use super::*;
    use crate::map::VacantEntry;
    use crate::value::Value;
    #[test]
    fn test_insert() {
        let _rug_st_tests_rug_348_rrrruuuugggg_test_insert = 0;
        let mut p0: VacantEntry<'static> = unimplemented!();
        let p1: Value = unimplemented!();
        p0.insert(p1);
        let _rug_ed_tests_rug_348_rrrruuuugggg_test_insert = 0;
    }
}
#[cfg(test)]
mod tests_rug_349 {
    use super::*;
    use crate::map::OccupiedEntry;
    #[test]
    fn test_key() {
        let _rug_st_tests_rug_349_rrrruuuugggg_test_key = 0;
        let mut p0: OccupiedEntry<'_> = unimplemented!();
        p0.key();
        let _rug_ed_tests_rug_349_rrrruuuugggg_test_key = 0;
    }
}
#[cfg(test)]
mod tests_rug_350 {
    use super::*;
    use crate::Value;
    use crate::map::OccupiedEntry;
    #[test]
    fn test_rug() {
        let mut p0: OccupiedEntry<'static> = todo!("construct p0 here");
        p0.get();
    }
}
#[cfg(test)]
mod tests_rug_353 {
    use super::*;
    use crate::map::OccupiedEntry;
    use crate::de::Error;
    use crate::value::Value;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_353_rrrruuuugggg_test_rug = 0;
        let mut p0: OccupiedEntry<'static> = unimplemented!();
        let p1: Value = unimplemented!();
        OccupiedEntry::insert(&mut p0, p1);
        let _rug_ed_tests_rug_353_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_355 {
    use super::*;
    use crate::map::Map;
    use crate::value::Value;
    #[test]
    fn test_into_iter() {
        let _rug_st_tests_rug_355_rrrruuuugggg_test_into_iter = 0;
        let rug_fuzz_0 = "key1";
        let rug_fuzz_1 = 123;
        let rug_fuzz_2 = "key2";
        let rug_fuzz_3 = "value";
        let mut p0 = Map::<std::string::String, Value>::new();
        p0.insert(rug_fuzz_0.to_string(), Value::Integer(rug_fuzz_1));
        p0.insert(rug_fuzz_2.to_string(), Value::String(rug_fuzz_3.to_string()));
        p0.into_iter();
        let _rug_ed_tests_rug_355_rrrruuuugggg_test_into_iter = 0;
    }
}
#[cfg(test)]
mod tests_rug_375 {
    use super::*;
    use crate::map::Values;
    use std::iter::Iterator;
    #[test]
    fn test_size_hint() {
        let _rug_st_tests_rug_375_rrrruuuugggg_test_size_hint = 0;
        let mut p0: Values<'static> = unimplemented!();
        <Values<'static> as Iterator>::size_hint(&p0);
        let _rug_ed_tests_rug_375_rrrruuuugggg_test_size_hint = 0;
    }
}
