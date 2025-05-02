//! A map of String to serde_json::Value.
//!
//! By default the map is backed by a [`BTreeMap`]. Enable the `preserve_order`
//! feature of serde_json to use [`IndexMap`] instead.
//!
//! [`BTreeMap`]: https://doc.rust-lang.org/std/collections/struct.BTreeMap.html
//! [`IndexMap`]: https://docs.rs/indexmap/*/indexmap/map/struct.IndexMap.html
use crate::lib::borrow::Borrow;
use crate::lib::iter::FromIterator;
use crate::lib::*;
use crate::value::Value;
use serde::de;
#[cfg(feature = "preserve_order")]
use indexmap::{self, IndexMap};
/// Represents a JSON key/value type.
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
    /// Makes a new empty Map with the given initial capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Map {
            #[cfg(not(feature = "preserve_order"))]
            map: {
                let _ = capacity;
                BTreeMap::new()
            },
            #[cfg(feature = "preserve_order")]
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
    pub fn get<Q>(&self, key: &Q) -> Option<&Value>
    where
        String: Borrow<Q>,
        Q: ?Sized + Ord + Eq + Hash,
    {
        self.map.get(key)
    }
    /// Returns true if the map contains a value for the specified key.
    ///
    /// The key may be any borrowed form of the map's key type, but the ordering
    /// on the borrowed form *must* match the ordering on the key type.
    #[inline]
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        String: Borrow<Q>,
        Q: ?Sized + Ord + Eq + Hash,
    {
        self.map.contains_key(key)
    }
    /// Returns a mutable reference to the value corresponding to the key.
    ///
    /// The key may be any borrowed form of the map's key type, but the ordering
    /// on the borrowed form *must* match the ordering on the key type.
    #[inline]
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut Value>
    where
        String: Borrow<Q>,
        Q: ?Sized + Ord + Eq + Hash,
    {
        self.map.get_mut(key)
    }
    /// Inserts a key-value pair into the map.
    ///
    /// If the map did not have this key present, `None` is returned.
    ///
    /// If the map did have this key present, the value is updated, and the old
    /// value is returned.
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
    pub fn remove<Q>(&mut self, key: &Q) -> Option<Value>
    where
        String: Borrow<Q>,
        Q: ?Sized + Ord + Eq + Hash,
    {
        #[cfg(feature = "preserve_order")] return self.map.swap_remove(key);
        #[cfg(not(feature = "preserve_order"))] return self.map.remove(key);
    }
    /// Removes a key from the map, returning the stored key and value if the
    /// key was previously in the map.
    ///
    /// The key may be any borrowed form of the map's key type, but the ordering
    /// on the borrowed form *must* match the ordering on the key type.
    pub fn remove_entry<Q>(&mut self, key: &Q) -> Option<(String, Value)>
    where
        String: Borrow<Q>,
        Q: ?Sized + Ord + Eq + Hash,
    {
        #[cfg(any(feature = "preserve_order", not(no_btreemap_remove_entry)))]
        return self.map.remove_entry(key);
        #[cfg(
            all(
                not(feature = "preserve_order"),
                no_btreemap_remove_entry,
                not(no_btreemap_get_key_value),
            )
        )]
        {
            let (key, _value) = self.map.get_key_value(key)?;
            let key = key.clone();
            let value = self.map.remove::<String>(&key)?;
            Some((key, value))
        }
        #[cfg(
            all(
                not(feature = "preserve_order"),
                no_btreemap_remove_entry,
                no_btreemap_get_key_value,
            )
        )]
        {
            struct Key<'a, Q: ?Sized>(&'a Q);
            impl<'a, Q: ?Sized> RangeBounds<Q> for Key<'a, Q> {
                fn start_bound(&self) -> Bound<&Q> {
                    Bound::Included(self.0)
                }
                fn end_bound(&self) -> Bound<&Q> {
                    Bound::Included(self.0)
                }
            }
            let mut range = self.map.range(Key(key));
            let (key, _value) = range.next()?;
            let key = key.clone();
            let value = self.map.remove::<String>(&key)?;
            Some((key, value))
        }
    }
    /// Moves all elements from other into Self, leaving other empty.
    #[inline]
    pub fn append(&mut self, other: &mut Self) {
        #[cfg(feature = "preserve_order")]
        for (k, v) in mem::replace(&mut other.map, MapImpl::default()) {
            self.map.insert(k, v);
        }
        #[cfg(not(feature = "preserve_order"))] self.map.append(&mut other.map);
    }
    /// Gets the given key's corresponding entry in the map for in-place
    /// manipulation.
    pub fn entry<S>(&mut self, key: S) -> Entry
    where
        S: Into<String>,
    {
        #[cfg(not(feature = "preserve_order"))]
        use crate::lib::btree_map::Entry as EntryImpl;
        #[cfg(feature = "preserve_order")]
        use indexmap::map::Entry as EntryImpl;
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
    pub fn iter(&self) -> Iter {
        Iter { iter: self.map.iter() }
    }
    /// Gets a mutable iterator over the entries of the map.
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut {
        IterMut {
            iter: self.map.iter_mut(),
        }
    }
    /// Gets an iterator over the keys of the map.
    #[inline]
    pub fn keys(&self) -> Keys {
        Keys { iter: self.map.keys() }
    }
    /// Gets an iterator over the values of the map.
    #[inline]
    pub fn values(&self) -> Values {
        Values { iter: self.map.values() }
    }
    /// Gets an iterator over mutable values of the map.
    #[inline]
    pub fn values_mut(&mut self) -> ValuesMut {
        ValuesMut {
            iter: self.map.values_mut(),
        }
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
impl Eq for Map<String, Value> {}
/// Access an element of this map. Panics if the given key is not present in the
/// map.
///
/// ```
/// # use serde_json::Value;
/// #
/// # let val = &Value::String("".to_owned());
/// # let _ =
/// match *val {
///     Value::String(ref s) => Some(s.as_str()),
///     Value::Array(ref arr) => arr[0].as_str(),
///     Value::Object(ref map) => map["type"].as_str(),
///     _ => None,
/// }
/// # ;
/// ```
impl<'a, Q> ops::Index<&'a Q> for Map<String, Value>
where
    String: Borrow<Q>,
    Q: ?Sized + Ord + Eq + Hash,
{
    type Output = Value;
    fn index(&self, index: &Q) -> &Value {
        self.map.index(index)
    }
}
/// Mutably access an element of this map. Panics if the given key is not
/// present in the map.
///
/// ```
/// # use serde_json::json;
/// #
/// # let mut map = serde_json::Map::new();
/// # map.insert("key".to_owned(), serde_json::Value::Null);
/// #
/// map["key"] = json!("value");
/// ```
impl<'a, Q> ops::IndexMut<&'a Q> for Map<String, Value>
where
    String: Borrow<Q>,
    Q: ?Sized + Ord + Eq + Hash,
{
    fn index_mut(&mut self, index: &Q) -> &mut Value {
        self.map.get_mut(index).expect("no entry found for key")
    }
}
impl Debug for Map<String, Value> {
    #[inline]
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.map.fmt(formatter)
    }
}
#[cfg(any(feature = "std", feature = "alloc"))]
impl serde::ser::Serialize for Map<String, Value> {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = tri!(serializer.serialize_map(Some(self.len())));
        for (k, v) in self {
            tri!(map.serialize_entry(k, v));
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
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map")
            }
            #[inline]
            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Map::new())
            }
            #[cfg(any(feature = "std", feature = "alloc"))]
            #[inline]
            fn visit_map<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                let mut values = Map::new();
                while let Some((key, value)) = tri!(visitor.next_entry()) {
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
        #[inline] fn len(& self) -> usize { self.iter.len() } } impl $($generics)*
        FusedIterator for $name $($generics)* {}
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
    ///
    /// # Examples
    ///
    /// ```
    /// let mut map = serde_json::Map::new();
    /// assert_eq!(map.entry("serde").key(), &"serde");
    /// ```
    pub fn key(&self) -> &String {
        match *self {
            Entry::Vacant(ref e) => e.key(),
            Entry::Occupied(ref e) => e.key(),
        }
    }
    /// Ensures a value is in the entry by inserting the default if empty, and
    /// returns a mutable reference to the value in the entry.
    ///
    /// # Examples
    ///
    /// ```
    /// # use serde_json::json;
    /// #
    /// let mut map = serde_json::Map::new();
    /// map.entry("serde").or_insert(json!(12));
    ///
    /// assert_eq!(map["serde"], 12);
    /// ```
    pub fn or_insert(self, default: Value) -> &'a mut Value {
        match self {
            Entry::Vacant(entry) => entry.insert(default),
            Entry::Occupied(entry) => entry.into_mut(),
        }
    }
    /// Ensures a value is in the entry by inserting the result of the default
    /// function if empty, and returns a mutable reference to the value in the
    /// entry.
    ///
    /// # Examples
    ///
    /// ```
    /// # use serde_json::json;
    /// #
    /// let mut map = serde_json::Map::new();
    /// map.entry("serde").or_insert_with(|| json!("hoho"));
    ///
    /// assert_eq!(map["serde"], "hoho".to_owned());
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// use serde_json::map::Entry;
    ///
    /// let mut map = serde_json::Map::new();
    ///
    /// match map.entry("serde") {
    ///     Entry::Vacant(vacant) => {
    ///         assert_eq!(vacant.key(), &"serde");
    ///     }
    ///     Entry::Occupied(_) => unimplemented!(),
    /// }
    /// ```
    #[inline]
    pub fn key(&self) -> &String {
        self.vacant.key()
    }
    /// Sets the value of the entry with the VacantEntry's key, and returns a
    /// mutable reference to it.
    ///
    /// # Examples
    ///
    /// ```
    /// # use serde_json::json;
    /// #
    /// use serde_json::map::Entry;
    ///
    /// let mut map = serde_json::Map::new();
    ///
    /// match map.entry("serde") {
    ///     Entry::Vacant(vacant) => {
    ///         vacant.insert(json!("hoho"));
    ///     }
    ///     Entry::Occupied(_) => unimplemented!(),
    /// }
    /// ```
    #[inline]
    pub fn insert(self, value: Value) -> &'a mut Value {
        self.vacant.insert(value)
    }
}
impl<'a> OccupiedEntry<'a> {
    /// Gets a reference to the key in the entry.
    ///
    /// # Examples
    ///
    /// ```
    /// # use serde_json::json;
    /// #
    /// use serde_json::map::Entry;
    ///
    /// let mut map = serde_json::Map::new();
    /// map.insert("serde".to_owned(), json!(12));
    ///
    /// match map.entry("serde") {
    ///     Entry::Occupied(occupied) => {
    ///         assert_eq!(occupied.key(), &"serde");
    ///     }
    ///     Entry::Vacant(_) => unimplemented!(),
    /// }
    /// ```
    #[inline]
    pub fn key(&self) -> &String {
        self.occupied.key()
    }
    /// Gets a reference to the value in the entry.
    ///
    /// # Examples
    ///
    /// ```
    /// # use serde_json::json;
    /// #
    /// use serde_json::map::Entry;
    ///
    /// let mut map = serde_json::Map::new();
    /// map.insert("serde".to_owned(), json!(12));
    ///
    /// match map.entry("serde") {
    ///     Entry::Occupied(occupied) => {
    ///         assert_eq!(occupied.get(), 12);
    ///     }
    ///     Entry::Vacant(_) => unimplemented!(),
    /// }
    /// ```
    #[inline]
    pub fn get(&self) -> &Value {
        self.occupied.get()
    }
    /// Gets a mutable reference to the value in the entry.
    ///
    /// # Examples
    ///
    /// ```
    /// # use serde_json::json;
    /// #
    /// use serde_json::map::Entry;
    ///
    /// let mut map = serde_json::Map::new();
    /// map.insert("serde".to_owned(), json!([1, 2, 3]));
    ///
    /// match map.entry("serde") {
    ///     Entry::Occupied(mut occupied) => {
    ///         occupied.get_mut().as_array_mut().unwrap().push(json!(4));
    ///     }
    ///     Entry::Vacant(_) => unimplemented!(),
    /// }
    ///
    /// assert_eq!(map["serde"].as_array().unwrap().len(), 4);
    /// ```
    #[inline]
    pub fn get_mut(&mut self) -> &mut Value {
        self.occupied.get_mut()
    }
    /// Converts the entry into a mutable reference to its value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use serde_json::json;
    /// #
    /// use serde_json::map::Entry;
    ///
    /// let mut map = serde_json::Map::new();
    /// map.insert("serde".to_owned(), json!([1, 2, 3]));
    ///
    /// match map.entry("serde") {
    ///     Entry::Occupied(mut occupied) => {
    ///         occupied.into_mut().as_array_mut().unwrap().push(json!(4));
    ///     }
    ///     Entry::Vacant(_) => unimplemented!(),
    /// }
    ///
    /// assert_eq!(map["serde"].as_array().unwrap().len(), 4);
    /// ```
    #[inline]
    pub fn into_mut(self) -> &'a mut Value {
        self.occupied.into_mut()
    }
    /// Sets the value of the entry with the `OccupiedEntry`'s key, and returns
    /// the entry's old value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use serde_json::json;
    /// #
    /// use serde_json::map::Entry;
    ///
    /// let mut map = serde_json::Map::new();
    /// map.insert("serde".to_owned(), json!(12));
    ///
    /// match map.entry("serde") {
    ///     Entry::Occupied(mut occupied) => {
    ///         assert_eq!(occupied.insert(json!(13)), 12);
    ///         assert_eq!(occupied.get(), 13);
    ///     }
    ///     Entry::Vacant(_) => unimplemented!(),
    /// }
    /// ```
    #[inline]
    pub fn insert(&mut self, value: Value) -> Value {
        self.occupied.insert(value)
    }
    /// Takes the value of the entry out of the map, and returns it.
    ///
    /// # Examples
    ///
    /// ```
    /// # use serde_json::json;
    /// #
    /// use serde_json::map::Entry;
    ///
    /// let mut map = serde_json::Map::new();
    /// map.insert("serde".to_owned(), json!(12));
    ///
    /// match map.entry("serde") {
    ///     Entry::Occupied(occupied) => {
    ///         assert_eq!(occupied.remove(), 12);
    ///     }
    ///     Entry::Vacant(_) => unimplemented!(),
    /// }
    /// ```
    #[inline]
    pub fn remove(self) -> Value {
        #[cfg(feature = "preserve_order")] return self.occupied.swap_remove();
        #[cfg(not(feature = "preserve_order"))] return self.occupied.remove();
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
/// An iterator over a serde_json::Map's entries.
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
/// A mutable iterator over a serde_json::Map's entries.
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
/// An owning iterator over a serde_json::Map's entries.
pub struct IntoIter {
    iter: IntoIterImpl,
}
#[cfg(not(feature = "preserve_order"))]
type IntoIterImpl = btree_map::IntoIter<String, Value>;
#[cfg(feature = "preserve_order")]
type IntoIterImpl = indexmap::map::IntoIter<String, Value>;
delegate_iterator!((IntoIter) => (String, Value));
/// An iterator over a serde_json::Map's keys.
pub struct Keys<'a> {
    iter: KeysImpl<'a>,
}
#[cfg(not(feature = "preserve_order"))]
type KeysImpl<'a> = btree_map::Keys<'a, String, Value>;
#[cfg(feature = "preserve_order")]
type KeysImpl<'a> = indexmap::map::Keys<'a, String, Value>;
delegate_iterator!((Keys <'a >) => &'a String);
/// An iterator over a serde_json::Map's values.
pub struct Values<'a> {
    iter: ValuesImpl<'a>,
}
#[cfg(not(feature = "preserve_order"))]
type ValuesImpl<'a> = btree_map::Values<'a, String, Value>;
#[cfg(feature = "preserve_order")]
type ValuesImpl<'a> = indexmap::map::Values<'a, String, Value>;
delegate_iterator!((Values <'a >) => &'a Value);
/// A mutable iterator over a serde_json::Map's values.
pub struct ValuesMut<'a> {
    iter: ValuesMutImpl<'a>,
}
#[cfg(not(feature = "preserve_order"))]
type ValuesMutImpl<'a> = btree_map::ValuesMut<'a, String, Value>;
#[cfg(feature = "preserve_order")]
type ValuesMutImpl<'a> = indexmap::map::ValuesMut<'a, String, Value>;
delegate_iterator!((ValuesMut <'a >) => &'a mut Value);
#[cfg(test)]
mod tests_llm_16_1 {
    use crate::{map::Map, value::Value};
    use std::iter::FromIterator;
    #[test]
    fn test_into_iter() {
        let _rug_st_tests_llm_16_1_rrrruuuugggg_test_into_iter = 0;
        let rug_fuzz_0 = "key1";
        let rug_fuzz_1 = "value1";
        let rug_fuzz_2 = "key2";
        let rug_fuzz_3 = "value2";
        let mut map = Map::new();
        map.insert(rug_fuzz_0.to_owned(), Value::String(rug_fuzz_1.to_owned()));
        map.insert(rug_fuzz_2.to_owned(), Value::String(rug_fuzz_3.to_owned()));
        let mut iter = map.into_iter();
        debug_assert_eq!(
            iter.next(), Some(("key1".to_owned(), Value::String("value1".to_owned())))
        );
        debug_assert_eq!(
            iter.next(), Some(("key2".to_owned(), Value::String("value2".to_owned())))
        );
        debug_assert_eq!(iter.next(), None);
        let _rug_ed_tests_llm_16_1_rrrruuuugggg_test_into_iter = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_84_llm_16_83 {
    use super::*;
    use crate::*;
    use crate::map::Map;
    #[test]
    fn test_into_iter() {
        let _rug_st_tests_llm_16_84_llm_16_83_rrrruuuugggg_test_into_iter = 0;
        let rug_fuzz_0 = "key1";
        let rug_fuzz_1 = "value1";
        let rug_fuzz_2 = "key2";
        let rug_fuzz_3 = "value2";
        let rug_fuzz_4 = "key3";
        let rug_fuzz_5 = "value3";
        let mut map: Map<String, Value> = Map::new();
        map.insert(rug_fuzz_0.to_string(), Value::String(rug_fuzz_1.to_string()));
        map.insert(rug_fuzz_2.to_string(), Value::String(rug_fuzz_3.to_string()));
        map.insert(rug_fuzz_4.to_string(), Value::String(rug_fuzz_5.to_string()));
        let mut iter = map.into_iter();
        debug_assert_eq!(
            iter.next(), Some(("key1".to_string(), Value::String("value1".to_string())))
        );
        debug_assert_eq!(
            iter.next(), Some(("key2".to_string(), Value::String("value2".to_string())))
        );
        debug_assert_eq!(
            iter.next(), Some(("key3".to_string(), Value::String("value3".to_string())))
        );
        debug_assert_eq!(iter.next(), None);
        let _rug_ed_tests_llm_16_84_llm_16_83_rrrruuuugggg_test_into_iter = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_253 {
    use super::*;
    use crate::*;
    use crate::map::Map;
    use crate::value::Value;
    #[test]
    fn test_next_back() {
        let _rug_st_tests_llm_16_253_rrrruuuugggg_test_next_back = 0;
        let rug_fuzz_0 = "a";
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = "b";
        let rug_fuzz_3 = 2;
        let rug_fuzz_4 = "c";
        let rug_fuzz_5 = 3;
        let mut map: Map<String, Value> = Map::new();
        map.insert(
            rug_fuzz_0.to_string(),
            Value::Number(crate::Number::from(rug_fuzz_1)),
        );
        map.insert(
            rug_fuzz_2.to_string(),
            Value::Number(crate::Number::from(rug_fuzz_3)),
        );
        map.insert(
            rug_fuzz_4.to_string(),
            Value::Number(crate::Number::from(rug_fuzz_5)),
        );
        let mut iter = map.into_iter();
        debug_assert_eq!(
            iter.next_back(), Some(("c".to_string(), Value::Number(crate
            ::Number::from(3))))
        );
        debug_assert_eq!(
            iter.next_back(), Some(("b".to_string(), Value::Number(crate
            ::Number::from(2))))
        );
        debug_assert_eq!(
            iter.next_back(), Some(("a".to_string(), Value::Number(crate
            ::Number::from(1))))
        );
        debug_assert_eq!(iter.next_back(), None);
        let _rug_ed_tests_llm_16_253_rrrruuuugggg_test_next_back = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_270 {
    use crate::map::IterMut;
    #[test]
    fn test_len() {
        let _rug_st_tests_llm_16_270_rrrruuuugggg_test_len = 0;
        let rug_fuzz_0 = "key1";
        let rug_fuzz_1 = "value1";
        let rug_fuzz_2 = "key2";
        let rug_fuzz_3 = "value2";
        let rug_fuzz_4 = "key3";
        let rug_fuzz_5 = "value3";
        let mut map = crate::map::Map::new();
        map.insert(rug_fuzz_0.to_string(), crate::json!(rug_fuzz_1));
        map.insert(rug_fuzz_2.to_string(), crate::json!(rug_fuzz_3));
        map.insert(rug_fuzz_4.to_string(), crate::json!(rug_fuzz_5));
        let iter_mut = map.iter_mut();
        let len = iter_mut.len();
        debug_assert_eq!(len, 3);
        let _rug_ed_tests_llm_16_270_rrrruuuugggg_test_len = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_271 {
    use super::*;
    use crate::*;
    #[test]
    fn test_next() {
        let _rug_st_tests_llm_16_271_rrrruuuugggg_test_next = 0;
        let rug_fuzz_0 = "key1";
        let rug_fuzz_1 = "value1";
        let rug_fuzz_2 = "key2";
        let rug_fuzz_3 = "value2";
        let mut map = crate::Map::new();
        map.insert(rug_fuzz_0.to_string(), crate::json!(rug_fuzz_1));
        map.insert(rug_fuzz_2.to_string(), crate::json!(rug_fuzz_3));
        let mut iter = map.iter_mut();
        debug_assert_eq!(
            iter.next(), Some((& "key1".to_string(), & mut crate ::json!("value1")))
        );
        debug_assert_eq!(
            iter.next(), Some((& "key2".to_string(), & mut crate ::json!("value2")))
        );
        debug_assert_eq!(iter.next(), None);
        let _rug_ed_tests_llm_16_271_rrrruuuugggg_test_next = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_281 {
    use super::*;
    use crate::*;
    use crate::{Map, Value};
    #[test]
    fn test_size_hint() {
        let _rug_st_tests_llm_16_281_rrrruuuugggg_test_size_hint = 0;
        let rug_fuzz_0 = "key1";
        let rug_fuzz_1 = "value1";
        let rug_fuzz_2 = "key2";
        let rug_fuzz_3 = "value2";
        let rug_fuzz_4 = "key3";
        let rug_fuzz_5 = "value3";
        let mut map = Map::new();
        map.insert(rug_fuzz_0.to_owned(), Value::String(rug_fuzz_1.to_owned()));
        map.insert(rug_fuzz_2.to_owned(), Value::String(rug_fuzz_3.to_owned()));
        map.insert(rug_fuzz_4.to_owned(), Value::String(rug_fuzz_5.to_owned()));
        let keys = map.keys();
        let mut keys_iter = keys.into_iter();
        let (lower, upper) = keys_iter.size_hint();
        debug_assert_eq!(lower, keys_iter.len());
        let _rug_ed_tests_llm_16_281_rrrruuuugggg_test_size_hint = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_286 {
    use crate::{Map, Value};
    #[test]
    fn test_clone() {
        let _rug_st_tests_llm_16_286_rrrruuuugggg_test_clone = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = "value";
        let mut map = Map::new();
        map.insert(rug_fuzz_0.to_string(), Value::String(rug_fuzz_1.to_string()));
        let cloned = map.clone();
        debug_assert_eq!(map, cloned);
        let _rug_ed_tests_llm_16_286_rrrruuuugggg_test_clone = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_287 {
    use crate::{Map, Value};
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_287_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "key1";
        let rug_fuzz_1 = "value1";
        let rug_fuzz_2 = "key2";
        let rug_fuzz_3 = "value2";
        let rug_fuzz_4 = "key1";
        let rug_fuzz_5 = "value1";
        let rug_fuzz_6 = "key2";
        let rug_fuzz_7 = "value2";
        let mut map1 = Map::new();
        map1.insert(rug_fuzz_0.to_string(), Value::String(rug_fuzz_1.to_string()));
        map1.insert(rug_fuzz_2.to_string(), Value::String(rug_fuzz_3.to_string()));
        let mut map2 = Map::new();
        map2.insert(rug_fuzz_4.to_string(), Value::String(rug_fuzz_5.to_string()));
        map2.insert(rug_fuzz_6.to_string(), Value::String(rug_fuzz_7.to_string()));
        debug_assert_eq!(map1.eq(& map2), true);
        let _rug_ed_tests_llm_16_287_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_288 {
    use super::*;
    use crate::*;
    #[test]
    fn test_default() {
        let _rug_st_tests_llm_16_288_rrrruuuugggg_test_default = 0;
        let default_map: Map<String, Value> = Default::default();
        debug_assert_eq!(default_map.len(), 0);
        let _rug_ed_tests_llm_16_288_rrrruuuugggg_test_default = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_289 {
    use crate::Map;
    use crate::Value;
    #[test]
    fn test_extend() {
        let _rug_st_tests_llm_16_289_rrrruuuugggg_test_extend = 0;
        let rug_fuzz_0 = "key1";
        let rug_fuzz_1 = "value1";
        let rug_fuzz_2 = "key2";
        let rug_fuzz_3 = "value2";
        let rug_fuzz_4 = "key1";
        let rug_fuzz_5 = "key2";
        let rug_fuzz_6 = "key3";
        let mut map = Map::new();
        map.insert(rug_fuzz_0.to_owned(), Value::String(rug_fuzz_1.to_owned()));
        let iter = vec![
            (rug_fuzz_2.to_owned(), Value::String(rug_fuzz_3.to_owned())), ("key3"
            .to_owned(), Value::String("value3".to_owned()))
        ];
        map.extend(iter);
        debug_assert_eq!(map.len(), 3);
        debug_assert_eq!(
            map.get(rug_fuzz_4), Some(& Value::String("value1".to_owned()))
        );
        debug_assert_eq!(
            map.get(rug_fuzz_5), Some(& Value::String("value2".to_owned()))
        );
        debug_assert_eq!(
            map.get(rug_fuzz_6), Some(& Value::String("value3".to_owned()))
        );
        let _rug_ed_tests_llm_16_289_rrrruuuugggg_test_extend = 0;
    }
}
#[test]
fn test_from_iter() {
    use std::iter::FromIterator;
    use crate::map::Map;
    use crate::value::Value;
    let test_data: Vec<(String, Value)> = vec![
        ("key1".to_string(), Value::String("value1".to_string())), ("key2".to_string(),
        Value::String("value2".to_string())), ("key3".to_string(), Value::String("value3"
        .to_string())),
    ];
    let map: Map<String, Value> = Map::from_iter(test_data);
    assert_eq!(map.len(), 3);
    assert_eq!(map.get("key1"), Some(& Value::String("value1".to_string())));
    assert_eq!(map.get("key2"), Some(& Value::String("value2".to_string())));
    assert_eq!(map.get("key3"), Some(& Value::String("value3".to_string())));
}
#[cfg(test)]
mod tests_llm_16_291 {
    use super::*;
    use crate::*;
    use crate::Value;
    #[test]
    fn test_into_iter() {
        let _rug_st_tests_llm_16_291_rrrruuuugggg_test_into_iter = 0;
        let rug_fuzz_0 = "key1";
        let rug_fuzz_1 = "value1";
        let rug_fuzz_2 = "key2";
        let rug_fuzz_3 = "value2";
        let rug_fuzz_4 = "key3";
        let rug_fuzz_5 = "value3";
        let mut map = Map::new();
        map.insert(rug_fuzz_0.to_string(), Value::String(rug_fuzz_1.to_string()));
        map.insert(rug_fuzz_2.to_string(), Value::String(rug_fuzz_3.to_string()));
        map.insert(rug_fuzz_4.to_string(), Value::String(rug_fuzz_5.to_string()));
        let mut iter = map.into_iter();
        debug_assert_eq!(
            iter.next(), Some(("key1".to_string(), Value::String("value1".to_string())))
        );
        debug_assert_eq!(
            iter.next(), Some(("key2".to_string(), Value::String("value2".to_string())))
        );
        debug_assert_eq!(
            iter.next(), Some(("key3".to_string(), Value::String("value3".to_string())))
        );
        debug_assert_eq!(iter.next(), None);
        let _rug_ed_tests_llm_16_291_rrrruuuugggg_test_into_iter = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_294 {
    use super::*;
    use crate::*;
    #[test]
    fn test_len() {
        let _rug_st_tests_llm_16_294_rrrruuuugggg_test_len = 0;
        let map = crate::map::Map::new();
        debug_assert_eq!(map.values().len(), 0);
        let _rug_ed_tests_llm_16_294_rrrruuuugggg_test_len = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_302_llm_16_301 {
    use crate::map::ValuesMut;
    use crate::value::{Map, Value};
    use std::iter::{DoubleEndedIterator, ExactSizeIterator, FusedIterator, Iterator};
    #[test]
    fn test_len() {
        let _rug_st_tests_llm_16_302_llm_16_301_rrrruuuugggg_test_len = 0;
        let rug_fuzz_0 = "key1";
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = "key2";
        let rug_fuzz_3 = 2;
        let mut map = Map::new();
        map.insert(rug_fuzz_0.to_string(), Value::Number(rug_fuzz_1.into()));
        map.insert(rug_fuzz_2.to_string(), Value::Number(rug_fuzz_3.into()));
        let values_mut: ValuesMut = map.values_mut();
        debug_assert_eq!(values_mut.len(), 2);
        let _rug_ed_tests_llm_16_302_llm_16_301_rrrruuuugggg_test_len = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_305 {
    use crate::map::ValuesMut;
    #[test]
    fn test_size_hint() {
        let mut values: ValuesMut<'_> = unimplemented!(
            "initialize values with correct value"
        );
        let (lower, upper) = values.size_hint();
        unimplemented!("assert the values of lower and upper");
    }
}
#[cfg(test)]
mod tests_llm_16_908 {
    use crate::map::{Entry, Map};
    use crate::Value;
    #[test]
    fn test_key() {
        let _rug_st_tests_llm_16_908_rrrruuuugggg_test_key = 0;
        let rug_fuzz_0 = "serde";
        let rug_fuzz_1 = true;
        let rug_fuzz_2 = "serde";
        let mut map = Map::new();
        map.insert(rug_fuzz_0.to_owned(), Value::Bool(rug_fuzz_1));
        let entry = map.entry(rug_fuzz_2);
        debug_assert_eq!(entry.key(), & "serde");
        let _rug_ed_tests_llm_16_908_rrrruuuugggg_test_key = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_909 {
    use super::*;
    use crate::*;
    use crate::{to_value, Value};
    #[test]
    fn test_append() {
        let _rug_st_tests_llm_16_909_rrrruuuugggg_test_append = 0;
        let rug_fuzz_0 = "key1";
        let rug_fuzz_1 = "value1";
        let rug_fuzz_2 = "key2";
        let rug_fuzz_3 = "value2";
        let rug_fuzz_4 = "key1";
        let rug_fuzz_5 = "key2";
        let mut map1: Map<String, Value> = Map::new();
        let mut map2: Map<String, Value> = Map::new();
        map1.insert(rug_fuzz_0.to_string(), to_value(rug_fuzz_1).unwrap());
        map2.insert(rug_fuzz_2.to_string(), to_value(rug_fuzz_3).unwrap());
        map1.append(&mut map2);
        debug_assert_eq!(map1.len(), 2);
        debug_assert_eq!(map2.len(), 0);
        debug_assert_eq!(map1.get(rug_fuzz_4), Some(& to_value("value1").unwrap()));
        debug_assert_eq!(map1.get(rug_fuzz_5), Some(& to_value("value2").unwrap()));
        let _rug_ed_tests_llm_16_909_rrrruuuugggg_test_append = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_910 {
    use super::*;
    use crate::*;
    use crate::Value;
    #[test]
    fn test_clear() {
        let _rug_st_tests_llm_16_910_rrrruuuugggg_test_clear = 0;
        let rug_fuzz_0 = "key1";
        let rug_fuzz_1 = "value1";
        let rug_fuzz_2 = "key2";
        let rug_fuzz_3 = "value2";
        let rug_fuzz_4 = "key3";
        let rug_fuzz_5 = "value3";
        let mut map: Map<String, Value> = Map::new();
        map.insert(rug_fuzz_0.to_string(), Value::String(rug_fuzz_1.to_string()));
        map.insert(rug_fuzz_2.to_string(), Value::String(rug_fuzz_3.to_string()));
        map.insert(rug_fuzz_4.to_string(), Value::String(rug_fuzz_5.to_string()));
        debug_assert_eq!(map.len(), 3);
        map.clear();
        debug_assert_eq!(map.len(), 0);
        debug_assert!(map.is_empty());
        let _rug_ed_tests_llm_16_910_rrrruuuugggg_test_clear = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_911 {
    use crate::{Map, Value};
    #[test]
    fn test_contains_key() {
        let _rug_st_tests_llm_16_911_rrrruuuugggg_test_contains_key = 0;
        let rug_fuzz_0 = "key1";
        let rug_fuzz_1 = true;
        let rug_fuzz_2 = "key2";
        let rug_fuzz_3 = "key1";
        let rug_fuzz_4 = "key2";
        let rug_fuzz_5 = "key3";
        let mut map: Map<String, Value> = Map::new();
        map.insert(rug_fuzz_0.to_string(), Value::Bool(rug_fuzz_1));
        map.insert(rug_fuzz_2.to_string(), Value::Null);
        debug_assert_eq!(map.contains_key(rug_fuzz_3), true);
        debug_assert_eq!(map.contains_key(rug_fuzz_4), true);
        debug_assert_eq!(map.contains_key(rug_fuzz_5), false);
        let _rug_ed_tests_llm_16_911_rrrruuuugggg_test_contains_key = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_914 {
    use crate::map::Map;
    use crate::value::Value;
    use std::borrow::Borrow;
    use std::collections::BTreeMap;
    use std::hash::Hash;
    #[test]
    fn test_get() {
        let _rug_st_tests_llm_16_914_rrrruuuugggg_test_get = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = "value";
        let rug_fuzz_2 = "key2";
        let rug_fuzz_3 = "value2";
        let rug_fuzz_4 = "key";
        let rug_fuzz_5 = "key2";
        let rug_fuzz_6 = "key3";
        let mut map: Map<String, Value> = Map::new();
        map.insert(rug_fuzz_0.to_string(), Value::String(rug_fuzz_1.to_string()));
        map.insert(rug_fuzz_2.to_string(), Value::String(rug_fuzz_3.to_string()));
        debug_assert_eq!(
            map.get(rug_fuzz_4), Some(& Value::String("value".to_string()))
        );
        debug_assert_eq!(
            map.get(rug_fuzz_5), Some(& Value::String("value2".to_string()))
        );
        debug_assert_eq!(map.get(rug_fuzz_6), None);
        let _rug_ed_tests_llm_16_914_rrrruuuugggg_test_get = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_917 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_empty_empty_map() {
        let _rug_st_tests_llm_16_917_rrrruuuugggg_test_is_empty_empty_map = 0;
        let map: Map<String, Value> = Map::new();
        debug_assert!(map.is_empty());
        let _rug_ed_tests_llm_16_917_rrrruuuugggg_test_is_empty_empty_map = 0;
    }
    #[test]
    fn test_is_empty_non_empty_map() {
        let _rug_st_tests_llm_16_917_rrrruuuugggg_test_is_empty_non_empty_map = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = "value";
        let mut map: Map<String, Value> = Map::new();
        map.insert(rug_fuzz_0.to_string(), Value::String(rug_fuzz_1.to_string()));
        debug_assert!(! map.is_empty());
        let _rug_ed_tests_llm_16_917_rrrruuuugggg_test_is_empty_non_empty_map = 0;
    }
    #[test]
    fn test_is_empty_after_clear() {
        let _rug_st_tests_llm_16_917_rrrruuuugggg_test_is_empty_after_clear = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = "value";
        let mut map: Map<String, Value> = Map::new();
        map.insert(rug_fuzz_0.to_string(), Value::String(rug_fuzz_1.to_string()));
        map.clear();
        debug_assert!(map.is_empty());
        let _rug_ed_tests_llm_16_917_rrrruuuugggg_test_is_empty_after_clear = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_918 {
    use crate::{map::Map, value::Value};
    #[test]
    fn test_iter() {
        let _rug_st_tests_llm_16_918_rrrruuuugggg_test_iter = 0;
        let rug_fuzz_0 = "foo";
        let rug_fuzz_1 = "bar";
        let rug_fuzz_2 = "abc";
        let rug_fuzz_3 = 123;
        let mut map = Map::new();
        map.insert(rug_fuzz_0.to_string(), Value::String(rug_fuzz_1.to_string()));
        map.insert(rug_fuzz_2.to_string(), Value::Number(rug_fuzz_3.into()));
        let mut iter = map.iter();
        let (key, value) = iter.next().unwrap();
        debug_assert_eq!(key, & "foo".to_string());
        debug_assert_eq!(value, & Value::String("bar".to_string()));
        let (key, value) = iter.next().unwrap();
        debug_assert_eq!(key, & "abc".to_string());
        debug_assert_eq!(value, & Value::Number(123.into()));
        debug_assert!(iter.next().is_none());
        let _rug_ed_tests_llm_16_918_rrrruuuugggg_test_iter = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_919 {
    use super::*;
    use crate::*;
    #[test]
    fn test_iter_mut() {
        let _rug_st_tests_llm_16_919_rrrruuuugggg_test_iter_mut = 0;
        let rug_fuzz_0 = "key1";
        let rug_fuzz_1 = "value1";
        let rug_fuzz_2 = "key2";
        let rug_fuzz_3 = "value2";
        let rug_fuzz_4 = "key3";
        let rug_fuzz_5 = "value3";
        let rug_fuzz_6 = "key4";
        let rug_fuzz_7 = "value4";
        let mut map: Map<String, Value> = Map::new();
        map.insert(rug_fuzz_0.to_string(), Value::String(rug_fuzz_1.to_string()));
        map.insert(rug_fuzz_2.to_string(), Value::String(rug_fuzz_3.to_string()));
        map.insert(rug_fuzz_4.to_string(), Value::String(rug_fuzz_5.to_string()));
        map.insert(rug_fuzz_6.to_string(), Value::String(rug_fuzz_7.to_string()));
        let mut iter_mut = map.iter_mut();
        debug_assert_eq!(
            iter_mut.next(), Some((& "key1".to_string(), & mut Value::String("value1"
            .to_string())))
        );
        debug_assert_eq!(
            iter_mut.next(), Some((& "key2".to_string(), & mut Value::String("value2"
            .to_string())))
        );
        debug_assert_eq!(
            iter_mut.next(), Some((& "key3".to_string(), & mut Value::String("value3"
            .to_string())))
        );
        debug_assert_eq!(
            iter_mut.next(), Some((& "key4".to_string(), & mut Value::String("value4"
            .to_string())))
        );
        debug_assert_eq!(iter_mut.next(), None);
        let _rug_ed_tests_llm_16_919_rrrruuuugggg_test_iter_mut = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_921 {
    use super::*;
    use crate::*;
    use crate::{Map, Value};
    #[test]
    fn test_keys() {
        let _rug_st_tests_llm_16_921_rrrruuuugggg_test_keys = 0;
        let rug_fuzz_0 = "name";
        let rug_fuzz_1 = "age";
        let rug_fuzz_2 = "city";
        let rug_fuzz_3 = "John";
        let rug_fuzz_4 = 30;
        let rug_fuzz_5 = "New York";
        let mut map = Map::new();
        let name = rug_fuzz_0.to_string();
        let age = rug_fuzz_1.to_string();
        let city = rug_fuzz_2.to_string();
        map.insert(name.clone(), Value::String(rug_fuzz_3.to_string()));
        map.insert(age.clone(), Value::Number(rug_fuzz_4.into()));
        map.insert(city.clone(), Value::String(rug_fuzz_5.to_string()));
        let keys = map.keys();
        let expected_keys: Vec<&String> = vec![& name, & age, & city];
        let actual_keys: Vec<&String> = keys.collect();
        debug_assert_eq!(actual_keys, expected_keys);
        let _rug_ed_tests_llm_16_921_rrrruuuugggg_test_keys = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_922 {
    use super::*;
    use crate::*;
    use crate::Map;
    use crate::Value;
    #[test]
    fn test_len() {
        let _rug_st_tests_llm_16_922_rrrruuuugggg_test_len = 0;
        let map: Map<String, Value> = Map::new();
        debug_assert_eq!(map.len(), 0);
        let _rug_ed_tests_llm_16_922_rrrruuuugggg_test_len = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_923 {
    use super::*;
    use crate::*;
    use crate::Value;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_923_rrrruuuugggg_test_new = 0;
        let map: Map<String, Value> = Map::new();
        debug_assert!(map.is_empty());
        let _rug_ed_tests_llm_16_923_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_924 {
    use super::*;
    use crate::*;
    #[test]
    fn test_remove() {
        let _rug_st_tests_llm_16_924_rrrruuuugggg_test_remove = 0;
        let rug_fuzz_0 = "key1";
        let rug_fuzz_1 = "value1";
        let rug_fuzz_2 = "key2";
        let rug_fuzz_3 = "value2";
        let rug_fuzz_4 = "key1";
        let rug_fuzz_5 = "key3";
        let mut map = Map::new();
        map.insert(rug_fuzz_0.to_string(), Value::String(rug_fuzz_1.to_string()));
        map.insert(rug_fuzz_2.to_string(), Value::String(rug_fuzz_3.to_string()));
        let result1 = map.remove(rug_fuzz_4);
        let result2 = map.remove(rug_fuzz_5);
        debug_assert_eq!(result1, Some(Value::String("value1".to_string())));
        debug_assert_eq!(result2, None);
        debug_assert_eq!(map.len(), 1);
        let _rug_ed_tests_llm_16_924_rrrruuuugggg_test_remove = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_925 {
    use super::*;
    use crate::*;
    use crate::Value;
    #[test]
    fn test_remove_entry() {
        let _rug_st_tests_llm_16_925_rrrruuuugggg_test_remove_entry = 0;
        let rug_fuzz_0 = "key1";
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = "key2";
        let rug_fuzz_3 = 2;
        let rug_fuzz_4 = "key3";
        let rug_fuzz_5 = 3;
        let rug_fuzz_6 = "key2";
        let rug_fuzz_7 = "key2";
        let mut map: Map<String, Value> = Map::new();
        map.insert(String::from(rug_fuzz_0), Value::from(rug_fuzz_1));
        map.insert(String::from(rug_fuzz_2), Value::from(rug_fuzz_3));
        map.insert(String::from(rug_fuzz_4), Value::from(rug_fuzz_5));
        let result = map.remove_entry(rug_fuzz_6);
        debug_assert_eq!(result, Some((String::from("key2"), Value::from(2))));
        debug_assert_eq!(map.len(), 2);
        debug_assert_eq!(map.get(rug_fuzz_7), None);
        let _rug_ed_tests_llm_16_925_rrrruuuugggg_test_remove_entry = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_926 {
    use super::*;
    use crate::*;
    #[test]
    fn test_values() {
        let _rug_st_tests_llm_16_926_rrrruuuugggg_test_values = 0;
        let rug_fuzz_0 = "key1";
        let rug_fuzz_1 = "value1";
        let rug_fuzz_2 = "key2";
        let rug_fuzz_3 = "value2";
        let rug_fuzz_4 = "key3";
        let rug_fuzz_5 = "value3";
        let mut map = map::Map::new();
        map.insert(rug_fuzz_0.to_string(), value::Value::String(rug_fuzz_1.to_string()));
        map.insert(rug_fuzz_2.to_string(), value::Value::String(rug_fuzz_3.to_string()));
        map.insert(rug_fuzz_4.to_string(), value::Value::String(rug_fuzz_5.to_string()));
        let mut values = map.values();
        debug_assert_eq!(
            values.next(), Some(& value::Value::String("value1".to_string()))
        );
        debug_assert_eq!(
            values.next(), Some(& value::Value::String("value2".to_string()))
        );
        debug_assert_eq!(
            values.next(), Some(& value::Value::String("value3".to_string()))
        );
        debug_assert_eq!(values.next(), None);
        let _rug_ed_tests_llm_16_926_rrrruuuugggg_test_values = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_927 {
    use crate::map::Map;
    use crate::value::Value;
    #[test]
    fn test_values_mut() {
        let _rug_st_tests_llm_16_927_rrrruuuugggg_test_values_mut = 0;
        let rug_fuzz_0 = "key1";
        let rug_fuzz_1 = "value1";
        let rug_fuzz_2 = "key2";
        let rug_fuzz_3 = "value2";
        let rug_fuzz_4 = "key3";
        let rug_fuzz_5 = "value3";
        let mut map: Map<String, Value> = Map::new();
        map.insert(String::from(rug_fuzz_0), Value::String(String::from(rug_fuzz_1)));
        map.insert(String::from(rug_fuzz_2), Value::String(String::from(rug_fuzz_3)));
        map.insert(String::from(rug_fuzz_4), Value::String(String::from(rug_fuzz_5)));
        let mut iter = map.values_mut();
        debug_assert_eq!(iter.next(), Some(& mut Value::String(String::from("value1"))));
        debug_assert_eq!(iter.next(), Some(& mut Value::String(String::from("value2"))));
        debug_assert_eq!(iter.next(), Some(& mut Value::String(String::from("value3"))));
        debug_assert_eq!(iter.next(), None);
        let _rug_ed_tests_llm_16_927_rrrruuuugggg_test_values_mut = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_930 {
    use super::*;
    use crate::*;
    use crate::json;
    use crate::map::Entry;
    #[test]
    fn test_key() {
        let _rug_st_tests_llm_16_930_rrrruuuugggg_test_key = 0;
        let rug_fuzz_0 = "serde";
        let rug_fuzz_1 = 12;
        let rug_fuzz_2 = "serde";
        let mut map = crate::Map::new();
        map.insert(rug_fuzz_0.to_owned(), json!(rug_fuzz_1));
        match map.entry(rug_fuzz_2) {
            Entry::Occupied(occupied) => {
                debug_assert_eq!(occupied.key(), & "serde");
            }
            Entry::Vacant(_) => unimplemented!(),
        }
        let _rug_ed_tests_llm_16_930_rrrruuuugggg_test_key = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_931 {
    use crate::{Map, Value, map::Entry};
    #[test]
    fn test_vacant_entry_key() {
        let _rug_st_tests_llm_16_931_rrrruuuugggg_test_vacant_entry_key = 0;
        let rug_fuzz_0 = "serde";
        let mut map = Map::new();
        match map.entry(rug_fuzz_0) {
            Entry::Vacant(vacant) => {
                debug_assert_eq!(vacant.key(), & String::from("serde"));
            }
            Entry::Occupied(_) => unimplemented!(),
        }
        let _rug_ed_tests_llm_16_931_rrrruuuugggg_test_vacant_entry_key = 0;
    }
}
#[cfg(test)]
mod tests_rug_546 {
    use super::*;
    use crate::{Map, value::Value};
    #[test]
    fn test_with_capacity() {
        let _rug_st_tests_rug_546_rrrruuuugggg_test_with_capacity = 0;
        let rug_fuzz_0 = 10;
        let p0: usize = rug_fuzz_0;
        Map::<String, Value>::with_capacity(p0);
        let _rug_ed_tests_rug_546_rrrruuuugggg_test_with_capacity = 0;
    }
}
#[cfg(test)]
mod tests_rug_548 {
    use super::*;
    use crate::{Number, Value, Map};
    #[test]
    fn test_insert() {
        let _rug_st_tests_rug_548_rrrruuuugggg_test_insert = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = "key";
        let rug_fuzz_2 = 42;
        let mut p0 = Map::<String, Value>::new();
        let p1 = String::from(rug_fuzz_0);
        let mut p2 = Value::Object(Map::new());
        p2[rug_fuzz_1] = Value::Number(Number::from(rug_fuzz_2));
        Map::<String, Value>::insert(&mut p0, p1, p2);
        let _rug_ed_tests_rug_548_rrrruuuugggg_test_insert = 0;
    }
}
#[cfg(test)]
mod tests_rug_549 {
    use super::*;
    use crate::map::Map;
    use crate::value::Value;
    #[test]
    fn test_entry() {
        let _rug_st_tests_rug_549_rrrruuuugggg_test_entry = 0;
        let rug_fuzz_0 = "key";
        let mut map: Map<String, Value> = Map::new();
        let key: String = rug_fuzz_0.to_string();
        map.entry(key);
        let _rug_ed_tests_rug_549_rrrruuuugggg_test_entry = 0;
    }
}
#[cfg(test)]
mod tests_rug_557 {
    use super::*;
    use crate::{Number, Value, Map};
    #[test]
    fn test_or_insert() {
        let _rug_st_tests_rug_557_rrrruuuugggg_test_or_insert = 0;
        let rug_fuzz_0 = "serde";
        let rug_fuzz_1 = 12;
        let rug_fuzz_2 = "serde";
        let mut map = Map::new();
        map.entry(rug_fuzz_0).or_insert(json!(rug_fuzz_1));
        debug_assert_eq!(map[rug_fuzz_2], 12);
        let _rug_ed_tests_rug_557_rrrruuuugggg_test_or_insert = 0;
    }
}
#[cfg(test)]
mod tests_rug_558 {
    use super::*;
    use crate::Map;
    use crate::Value;
    use crate::json;
    #[test]
    fn test_or_insert_with() {
        let _rug_st_tests_rug_558_rrrruuuugggg_test_or_insert_with = 0;
        let rug_fuzz_0 = "hoho";
        let rug_fuzz_1 = "serde";
        let rug_fuzz_2 = "serde";
        let mut map: Map<String, Value> = Map::new();
        let default = || json!(rug_fuzz_0);
        map.entry(rug_fuzz_1).or_insert_with(default);
        debug_assert_eq!(map[rug_fuzz_2], "hoho".to_owned());
        let _rug_ed_tests_rug_558_rrrruuuugggg_test_or_insert_with = 0;
    }
}
#[cfg(test)]
mod tests_rug_559 {
    use super::*;
    use crate::{json, map::Entry, Map, Number, Value};
    #[test]
    fn test_insert() {
        let _rug_st_tests_rug_559_rrrruuuugggg_test_insert = 0;
        let rug_fuzz_0 = "serde";
        let rug_fuzz_1 = "hoho";
        let mut map = Map::new();
        let key = rug_fuzz_0;
        let value = json!(rug_fuzz_1);
        match map.entry(key) {
            Entry::Vacant(vacant) => {
                vacant.insert(value);
            }
            Entry::Occupied(_) => unimplemented!(),
        }
        let _rug_ed_tests_rug_559_rrrruuuugggg_test_insert = 0;
    }
}
#[cfg(test)]
mod tests_rug_561 {
    use crate::Value;
    use crate::map::Entry::*;
    use crate::map::*;
    use crate::json;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_561_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "serde";
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 3;
        let rug_fuzz_4 = "serde";
        let rug_fuzz_5 = 4;
        let rug_fuzz_6 = "serde";
        let mut map = Map::new();
        map.insert(rug_fuzz_0.to_owned(), json!([rug_fuzz_1, rug_fuzz_2, rug_fuzz_3]));
        match map.entry(rug_fuzz_4) {
            Occupied(mut occupied) => {
                occupied.get_mut().as_array_mut().unwrap().push(json!(rug_fuzz_5));
            }
            Vacant(_) => unimplemented!(),
        }
        debug_assert_eq!(map[rug_fuzz_6].as_array().unwrap().len(), 4);
        let _rug_ed_tests_rug_561_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_563 {
    use super::*;
    use crate::{Map, Number, Value};
    use crate::map::Entry;
    #[test]
    fn test_insert() {
        let _rug_st_tests_rug_563_rrrruuuugggg_test_insert = 0;
        let rug_fuzz_0 = "serde";
        let rug_fuzz_1 = 12;
        let rug_fuzz_2 = "serde";
        let rug_fuzz_3 = 13;
        let mut map = Map::new();
        map.insert(rug_fuzz_0.to_owned(), Value::Number(Number::from(rug_fuzz_1)));
        match map.entry(rug_fuzz_2) {
            Entry::Occupied(mut occupied) => {
                debug_assert_eq!(
                    occupied.insert(Value::Number(Number::from(rug_fuzz_3))),
                    Value::Number(Number::from(12))
                );
                debug_assert_eq!(occupied.get(), & Value::Number(Number::from(13)));
            }
            Entry::Vacant(_) => unimplemented!(),
        }
        let _rug_ed_tests_rug_563_rrrruuuugggg_test_insert = 0;
    }
}
#[cfg(test)]
mod tests_rug_564 {
    use super::*;
    use crate::{json, Map, Value};
    use crate::map::Entry;
    #[test]
    fn test_remove() {
        let _rug_st_tests_rug_564_rrrruuuugggg_test_remove = 0;
        let rug_fuzz_0 = "serde";
        let rug_fuzz_1 = 12;
        let rug_fuzz_2 = "serde";
        let mut map = Map::new();
        map.insert(rug_fuzz_0.to_owned(), json!(rug_fuzz_1));
        match map.entry(rug_fuzz_2) {
            Entry::Occupied(occupied) => {
                debug_assert_eq!(occupied.remove(), 12);
            }
            Entry::Vacant(_) => unimplemented!(),
        }
        let _rug_ed_tests_rug_564_rrrruuuugggg_test_remove = 0;
    }
}
#[cfg(test)]
mod tests_rug_566 {
    use super::*;
    use crate::map::Iter;
    use crate::Value;
    use crate::Map;
    use std::iter::Iterator;
    #[test]
    fn test_size_hint() {
        let _rug_st_tests_rug_566_rrrruuuugggg_test_size_hint = 0;
        let map: Map<String, Value> = Map::new();
        let iter: Iter<'_> = map.iter();
        let p0: Iter<'_> = iter;
        <Iter<'_> as Iterator>::size_hint(&p0);
        let _rug_ed_tests_rug_566_rrrruuuugggg_test_size_hint = 0;
    }
}
#[cfg(test)]
mod tests_rug_576 {
    use super::*;
    use crate::map::Keys;
    use crate::map::Map;
    use crate::Value;
    #[test]
    fn test_len() {
        let _rug_st_tests_rug_576_rrrruuuugggg_test_len = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = "value";
        let mut map: Map<String, Value> = Map::new();
        map.insert(rug_fuzz_0.to_string(), Value::String(rug_fuzz_1.to_string()));
        let keys: Keys = map.keys();
        debug_assert_eq!(< Keys as ExactSizeIterator > ::len(& keys), 1);
        let _rug_ed_tests_rug_576_rrrruuuugggg_test_len = 0;
    }
}
#[cfg(test)]
mod tests_rug_578 {
    use super::*;
    use crate::map::Values;
    use crate::from_str;
    use crate::Value;
    #[test]
    fn test_size_hint() {
        let _rug_st_tests_rug_578_rrrruuuugggg_test_size_hint = 0;
        let rug_fuzz_0 = r#"{ "name": "John", "age": 30, "city": "New York" }"#;
        let json_str = rug_fuzz_0;
        let json_value: Value = from_str(json_str).unwrap();
        let values: Values<'_> = json_value.as_object().unwrap().values();
        let p0: &Values<'_> = &values;
        <Values<'_> as Iterator>::size_hint(p0);
        let _rug_ed_tests_rug_578_rrrruuuugggg_test_size_hint = 0;
    }
}
