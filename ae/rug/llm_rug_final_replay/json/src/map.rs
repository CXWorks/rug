//! A map of String to serde_json::Value.
//!
//! By default the map is backed by a [`BTreeMap`]. Enable the `preserve_order`
//! feature of serde_json to use [`IndexMap`] instead.
//!
//! [`BTreeMap`]: https://doc.rust-lang.org/std/collections/struct.BTreeMap.html
//! [`IndexMap`]: https://docs.rs/indexmap/*/indexmap/map/struct.IndexMap.html
use crate::value::Value;
use alloc::string::String;
use core::borrow::Borrow;
use core::fmt::{self, Debug};
use core::hash::Hash;
use core::iter::{FromIterator, FusedIterator};
#[cfg(feature = "preserve_order")]
use core::mem;
use core::ops;
use serde::de;
#[cfg(not(feature = "preserve_order"))]
use alloc::collections::{btree_map, BTreeMap};
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
        self.map.clear();
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
    /// Returns the key-value pair matching the given key.
    ///
    /// The key may be any borrowed form of the map's key type, but the ordering
    /// on the borrowed form *must* match the ordering on the key type.
    #[inline]
    #[cfg(any(feature = "preserve_order", not(no_btreemap_get_key_value)))]
    pub fn get_key_value<Q>(&self, key: &Q) -> Option<(&String, &Value)>
    where
        String: Borrow<Q>,
        Q: ?Sized + Ord + Eq + Hash,
    {
        self.map.get_key_value(key)
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
            use core::ops::{Bound, RangeBounds};
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
    /// Moves all elements from other into self, leaving other empty.
    #[inline]
    pub fn append(&mut self, other: &mut Self) {
        #[cfg(feature = "preserve_order")]
        self.map.extend(mem::replace(&mut other.map, MapImpl::default()));
        #[cfg(not(feature = "preserve_order"))] self.map.append(&mut other.map);
    }
    /// Gets the given key's corresponding entry in the map for in-place
    /// manipulation.
    pub fn entry<S>(&mut self, key: S) -> Entry
    where
        S: Into<String>,
    {
        #[cfg(not(feature = "preserve_order"))]
        use alloc::collections::btree_map::Entry as EntryImpl;
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
    /// Retains only the elements specified by the predicate.
    ///
    /// In other words, remove all pairs `(k, v)` such that `f(&k, &mut v)`
    /// returns `false`.
    #[cfg(not(no_btreemap_retain))]
    #[inline]
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&String, &mut Value) -> bool,
    {
        self.map.retain(f);
    }
}
#[allow(clippy::derivable_impls)]
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
    #[inline]
    fn clone_from(&mut self, source: &Self) {
        self.map.clone_from(&source.map);
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
/// match val {
///     Value::String(s) => Some(s.as_str()),
///     Value::Array(arr) => arr[0].as_str(),
///     Value::Object(map) => map["type"].as_str(),
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
        match self {
            Entry::Vacant(e) => e.key(),
            Entry::Occupied(e) => e.key(),
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
    /// Provides in-place mutable access to an occupied entry before any
    /// potential inserts into the map.
    ///
    /// # Examples
    ///
    /// ```
    /// # use serde_json::json;
    /// #
    /// let mut map = serde_json::Map::new();
    /// map.entry("serde")
    ///     .and_modify(|e| *e = json!("rust"))
    ///     .or_insert(json!("cpp"));
    ///
    /// assert_eq!(map["serde"], "cpp");
    ///
    /// map.entry("serde")
    ///     .and_modify(|e| *e = json!("rust"))
    ///     .or_insert(json!("cpp"));
    ///
    /// assert_eq!(map["serde"], "rust");
    /// ```
    pub fn and_modify<F>(self, f: F) -> Self
    where
        F: FnOnce(&mut Value),
    {
        match self {
            Entry::Occupied(mut entry) => {
                f(entry.get_mut());
                Entry::Occupied(entry)
            }
            Entry::Vacant(entry) => Entry::Vacant(entry),
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
mod tests_rug_643 {
    use super::*;
    use crate::{map::Map, value::Value};
    use std::string::String;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_643_rrrruuuugggg_test_rug = 0;
        Map::<String, Value>::new();
        let _rug_ed_tests_rug_643_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_644 {
    use super::*;
    use crate::{Map, Value};
    #[test]
    fn test_with_capacity() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: usize = rug_fuzz_0;
        let _ = Map::<String, Value>::with_capacity(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_645 {
    use super::*;
    use crate::value::Value;
    use crate::map::Map;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_645_rrrruuuugggg_test_rug = 0;
        let mut p0: Map<String, Value> = Map::new();
        <Map<String, Value>>::clear(&mut p0);
        let _rug_ed_tests_rug_645_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_646 {
    use super::*;
    use crate::{Map, value::Value};
    #[test]
    fn test_get() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Map<std::string::String, Value> = Map::new();
        let mut p1: &str = rug_fuzz_0;
        Map::get::<str>(&p0, &p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_648 {
    use super::*;
    use crate::map::Map;
    use crate::value::Value;
    use std::borrow::Borrow;
    use std::hash::Hash;
    #[test]
    fn test_get_mut() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4)) = <(&str, &str, &str, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut map: Map<std::string::String, Value> = Map::new();
        map.insert(rug_fuzz_0.to_string(), Value::String(rug_fuzz_1.to_string()));
        map.insert(rug_fuzz_2.to_string(), Value::String(rug_fuzz_3.to_string()));
        let key: std::string::String = rug_fuzz_4.to_string();
        map.get_mut(&key);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_649 {
    use super::*;
    use crate::{map, value::Value};
    #[test]
    fn test_get_key_value() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(&str, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut map: map::Map<String, Value> = Map::new();
        let key = rug_fuzz_0.to_string();
        let value = Value::from(rug_fuzz_1);
        map.insert(key.clone(), value.clone());
        let result = map.get_key_value(&key);
        debug_assert_eq!(result, Some((& key, & value)));
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_650 {
    use super::*;
    use crate::{json, Map, Value};
    #[test]
    fn test_insert() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Map::<String, Value>::new();
        let mut p1 = String::from(rug_fuzz_0);
        let mut p2 = Value::default();
        <Map<String, Value>>::insert(&mut p0, p1, p2);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_651 {
    use super::*;
    use crate::map::Map;
    use crate::value::Value;
    #[test]
    fn test_remove() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Map<String, Value> = Map::new();
        let mut p1: String = String::from(rug_fuzz_0);
        Map::<String, Value>::remove(&mut p0, &p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_652 {
    use super::*;
    use crate::{Map, Value};
    #[test]
    fn test_remove_entry() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(&str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Map<String, Value> = Map::new();
        let mut p1: String = String::from(rug_fuzz_0);
        p0.insert(String::from(rug_fuzz_1), Value::Null);
        let result = Map::<String, Value>::remove_entry(&mut p0, &p1);
        debug_assert_eq!(result.unwrap(), (String::from("key"), Value::Null));
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_653 {
    use super::*;
    use std::collections::HashMap;
    use crate::{Map, Value};
    #[test]
    fn test_append() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5, mut rug_fuzz_6, mut rug_fuzz_7)) = <(&str, i32, &str, &str, &str, i32, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Map<String, Value> = Map::new();
        let mut p1: Map<String, Value> = Map::new();
        let mut p0_data: HashMap<String, Value> = HashMap::new();
        p0_data.insert(String::from(rug_fuzz_0), Value::from(rug_fuzz_1));
        p0_data.insert(String::from(rug_fuzz_2), Value::from(rug_fuzz_3));
        p0.extend(p0_data);
        let mut p1_data: HashMap<String, Value> = HashMap::new();
        p1_data.insert(String::from(rug_fuzz_4), Value::from(rug_fuzz_5));
        p1_data.insert(String::from(rug_fuzz_6), Value::from(rug_fuzz_7));
        p1.extend(p1_data);
        crate::map::Map::<String, Value>::append(&mut p0, &mut p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_654 {
    use super::*;
    use crate::map::Map;
    use crate::value::Value;
    #[test]
    fn test_entry() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Map<String, Value> = Map::new();
        let p1: String = rug_fuzz_0.to_string();
        <Map<String, Value>>::entry(&mut p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_655 {
    use super::*;
    use crate::{Map, Value};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(&str, &str, bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Map::<std::string::String, Value>::new();
        p0.insert(rug_fuzz_0.to_string(), Value::Null);
        p0.insert(rug_fuzz_1.to_string(), Value::Bool(rug_fuzz_2));
        Map::<std::string::String, Value>::len(&p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_656 {
    use super::*;
    use crate::map::Map;
    use crate::value::Value;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(&str, &str, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Map::<std::string::String, Value>::new();
        p0.insert(rug_fuzz_0.to_string(), Value::String(rug_fuzz_1.to_string()));
        p0.insert(rug_fuzz_2.to_string(), Value::String(rug_fuzz_3.to_string()));
        <Map<std::string::String, Value>>::is_empty(&p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_657 {
    use super::*;
    use crate::map::Map;
    use std::string::String;
    use crate::value::Value;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(&str, i32, &str, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Map<String, Value> = Map::new();
        p0.insert(String::from(rug_fuzz_0), Value::from(rug_fuzz_1));
        p0.insert(String::from(rug_fuzz_2), Value::from(rug_fuzz_3));
        Map::<String, Value>::iter(&p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_658 {
    use super::*;
    use crate::map::Map;
    use crate::value::Value;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_658_rrrruuuugggg_test_rug = 0;
        let mut p0: Map<String, Value> = Map::new();
        let mut iter = Map::<String, Value>::iter_mut(&mut p0);
        let _rug_ed_tests_rug_658_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_659 {
    use super::*;
    use crate::map::Map;
    use crate::value::Value;
    use std::string::String;
    #[test]
    fn test_keys() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(&str, &str, bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Map<String, Value> = Map::new();
        p0.insert(rug_fuzz_0.to_string(), Value::Null);
        p0.insert(rug_fuzz_1.to_string(), Value::Bool(rug_fuzz_2));
        <Map<String, Value>>::keys(&p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_660 {
    use super::*;
    use crate::map::Map;
    use crate::value::Value;
    #[test]
    fn test_values() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(&str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Map<String, Value> = Map::new();
        p0.insert(rug_fuzz_0.to_string(), Value::String(rug_fuzz_1.to_string()));
        let result = p0.values();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_661 {
    use super::*;
    use std::collections::HashMap;
    use crate::value::{Map, Value};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(&str, i32, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Map<String, Value> = Map::new();
        p0.insert(String::from(rug_fuzz_0), Value::from(rug_fuzz_1));
        p0.insert(String::from(rug_fuzz_2), Value::from(rug_fuzz_3));
        <Map<String, Value>>::values_mut(&mut p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_663 {
    use super::*;
    use crate::{Map, Value};
    use std::default::Default;
    use std::string::String;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_663_rrrruuuugggg_test_rug = 0;
        let result: Map<String, Value> = <Map<String, Value> as Default>::default();
        let _rug_ed_tests_rug_663_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_664 {
    use super::*;
    use crate::map::Map;
    use crate::value::Value;
    use std::clone::Clone;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(&str, &str, &str, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Map<std::string::String, Value> = Map::new();
        p0.insert(rug_fuzz_0.to_string(), Value::String(rug_fuzz_1.to_string()));
        p0.insert(rug_fuzz_2.to_string(), Value::Number(rug_fuzz_3.into()));
        <Map<std::string::String, Value> as std::clone::Clone>::clone(&p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_665 {
    use super::*;
    use crate::{Map, Value};
    #[test]
    fn test_clone_from() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5, mut rug_fuzz_6, mut rug_fuzz_7, mut rug_fuzz_8)) = <(&str, &str, &str, i32, &str, bool, &str, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Map<String, Value> = Map::new();
        p0.insert(rug_fuzz_0.to_owned(), Value::String(rug_fuzz_1.to_owned()));
        p0.insert(rug_fuzz_2.to_owned(), Value::Number(rug_fuzz_3.into()));
        let mut p1: Map<String, Value> = Map::new();
        p1.insert(rug_fuzz_4.to_owned(), Value::Bool(rug_fuzz_5));
        p1.insert(rug_fuzz_6.to_owned(), Value::Null);
        <Map<String, Value> as Clone>::clone_from(&mut p0, &p1);
        debug_assert_eq!(p0.len(), 2);
        debug_assert_eq!(p0.get(rug_fuzz_7), Some(& Value::Bool(true)));
        debug_assert_eq!(p0.get(rug_fuzz_8), Some(& Value::Null));
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_666 {
    use super::*;
    use crate::value::{Value, Map};
    use crate::map::Entry;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5, mut rug_fuzz_6, mut rug_fuzz_7, mut rug_fuzz_8, mut rug_fuzz_9)) = <(&str, &str, &str, i32, &str, bool, &str, &str, &str, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Map::new();
        p0.insert(rug_fuzz_0.to_string(), Value::String(rug_fuzz_1.to_string()));
        p0.insert(rug_fuzz_2.to_string(), Value::Number(rug_fuzz_3.into()));
        p0.insert(rug_fuzz_4.to_string(), Value::Bool(rug_fuzz_5));
        let mut p1 = Map::new();
        p1.insert(rug_fuzz_6.to_string(), Value::String(rug_fuzz_7.to_string()));
        p1.insert(rug_fuzz_8.to_string(), Value::Number(rug_fuzz_9.into()));
        <Map<String, Value> as PartialEq>::eq(&p0, &p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_675 {
    use super::*;
    use crate::{value, Map, Value};
    use std::iter::Extend;
    #[test]
    fn test_extend() {
        let _rug_st_tests_rug_675_rrrruuuugggg_test_extend = 0;
        let mut p0: Map<std::string::String, value::Value> = Map::new();
        let mut p1: Map<std::string::String, value::Value> = Map::new();
        p0.extend(p1);
        let _rug_ed_tests_rug_675_rrrruuuugggg_test_extend = 0;
    }
}
#[cfg(test)]
mod tests_rug_677 {
    use super::*;
    use crate::{json, Map, Value};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut map = Map::new();
        let mut p1 = Value::default();
        map.entry(rug_fuzz_0).or_insert(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_678 {
    use super::*;
    use crate::{Map, Value, json};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(&str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut map: Map<String, Value> = Map::new();
        let key = rug_fuzz_0;
        let default = || json!(rug_fuzz_1);
        map.entry(key).or_insert_with(default);
        debug_assert_eq!(map[key], "hoho".to_owned());
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_680 {
    use super::*;
    use crate::map::Entry;
    use crate::map::VacantEntry;
    use crate::Map;
    #[test]
    fn test_key() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut map = Map::new();
        match map.entry(rug_fuzz_0) {
            Entry::Vacant(vacant) => {
                debug_assert_eq!(vacant.key(), & "serde");
            }
            Entry::Occupied(_) => unimplemented!(),
        }
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_681 {
    use super::*;
    use crate::{json, Map, Value};
    use crate::map::Entry;
    #[test]
    fn test_insert() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(&str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Map<String, Value> = Map::new();
        let p1 = json!(rug_fuzz_0);
        match p0.entry(rug_fuzz_1) {
            Entry::Vacant(vacant) => {
                vacant.insert(p1);
            }
            Entry::Occupied(_) => unimplemented!(),
        }
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_686 {
    use super::*;
    use crate::{json, Map, Value, map};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_686_rrrruuuugggg_test_rug = 0;
        let mut p0: map::OccupiedEntry<'static> = unimplemented!();
        let mut v31 = Value::default();
        let p1 = v31;
        p0.insert(p1);
        let _rug_ed_tests_rug_686_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_687 {
    use super::*;
    use crate::{Map, json, Value};
    use crate::map::Entry;
    #[test]
    fn test_remove() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(&str, i32, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut map: Map<String, Value> = Map::new();
        map.insert(rug_fuzz_0.to_owned(), json!(rug_fuzz_1));
        match map.entry(rug_fuzz_2) {
            Entry::Occupied(occupied) => {
                debug_assert_eq!(occupied.remove(), 12);
            }
            Entry::Vacant(_) => unimplemented!(),
        }
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_688 {
    use super::*;
    use crate::map::Map;
    use crate::value::Value;
    use std::string::String;
    #[test]
    fn test_into_iter() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(&str, i32, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Map<String, Value> = Map::new();
        p0.insert(String::from(rug_fuzz_0), Value::from(rug_fuzz_1));
        p0.insert(String::from(rug_fuzz_2), Value::from(rug_fuzz_3));
        <&Map<String, Value> as std::iter::IntoIterator>::into_iter(&p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_693 {
    use super::*;
    use crate::map::Map;
    use crate::value::Value;
    use std::iter::IntoIterator;
    #[test]
    fn test_into_iter() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(&str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Map<std::string::String, Value> = Map::new();
        p0.insert(rug_fuzz_0.to_owned(), Value::String(rug_fuzz_1.to_owned()));
        <&mut Map<std::string::String, Value> as IntoIterator>::into_iter(&mut p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_709 {
    use super::*;
    use crate::map::Values;
    use std::iter::DoubleEndedIterator;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_709_rrrruuuugggg_test_rug = 0;
        let mut p0: Values<'static> = unimplemented!();
        p0.next_back();
        let _rug_ed_tests_rug_709_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_713 {
    use super::*;
    use crate::map::ValuesMut;
    use std::iter::DoubleEndedIterator;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_713_rrrruuuugggg_test_rug = 0;
        let mut p0: ValuesMut<'_> = unimplemented!();
        <ValuesMut<'_> as DoubleEndedIterator>::next_back(&mut p0);
        let _rug_ed_tests_rug_713_rrrruuuugggg_test_rug = 0;
    }
}
