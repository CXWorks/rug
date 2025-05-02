use std::iter::FromIterator;
use crate::key::Key;
use crate::repr::Decor;
use crate::table::{Iter, IterMut, KeyValuePairs, TableKeyValue, TableLike};
use crate::{InternalString, Item, KeyMut, RawString, Table, Value};
/// Type representing a TOML inline table,
/// payload of the `Value::InlineTable` variant
#[derive(Debug, Default, Clone)]
pub struct InlineTable {
    preamble: RawString,
    decor: Decor,
    pub(crate) span: Option<std::ops::Range<usize>>,
    dotted: bool,
    pub(crate) items: KeyValuePairs,
}
/// Constructors
///
/// See also `FromIterator`
impl InlineTable {
    /// Creates an empty table.
    pub fn new() -> Self {
        Default::default()
    }
    pub(crate) fn with_pairs(items: KeyValuePairs) -> Self {
        Self {
            items,
            ..Default::default()
        }
    }
    /// Convert to a table
    pub fn into_table(self) -> Table {
        let mut t = Table::with_pairs(self.items);
        t.fmt();
        t
    }
}
/// Formatting
impl InlineTable {
    /// Get key/values for values that are visually children of this table
    ///
    /// For example, this will return dotted keys
    pub fn get_values(&self) -> Vec<(Vec<&Key>, &Value)> {
        let mut values = Vec::new();
        let root = Vec::new();
        self.append_values(&root, &mut values);
        values
    }
    pub(crate) fn append_values<'s, 'c>(
        &'s self,
        parent: &[&'s Key],
        values: &'c mut Vec<(Vec<&'s Key>, &'s Value)>,
    ) {
        for value in self.items.values() {
            let mut path = parent.to_vec();
            path.push(&value.key);
            match &value.value {
                Item::Value(Value::InlineTable(table)) if table.is_dotted() => {
                    table.append_values(&path, values);
                }
                Item::Value(value) => {
                    values.push((path, value));
                }
                _ => {}
            }
        }
    }
    /// Auto formats the table.
    pub fn fmt(&mut self) {
        decorate_inline_table(self);
    }
    /// Sorts the key/value pairs by key.
    pub fn sort_values(&mut self) {
        self.items.sort_keys();
        for kv in self.items.values_mut() {
            match &mut kv.value {
                Item::Value(Value::InlineTable(table)) if table.is_dotted() => {
                    table.sort_values();
                }
                _ => {}
            }
        }
    }
    /// Sort Key/Value Pairs of the table using the using the comparison function `compare`.
    ///
    /// The comparison function receives two key and value pairs to compare (you can sort by keys or
    /// values or their combination as needed).
    pub fn sort_values_by<F>(&mut self, mut compare: F)
    where
        F: FnMut(&Key, &Value, &Key, &Value) -> std::cmp::Ordering,
    {
        self.sort_values_by_internal(&mut compare);
    }
    fn sort_values_by_internal<F>(&mut self, compare: &mut F)
    where
        F: FnMut(&Key, &Value, &Key, &Value) -> std::cmp::Ordering,
    {
        let modified_cmp = |
            _: &InternalString,
            val1: &TableKeyValue,
            _: &InternalString,
            val2: &TableKeyValue,
        | -> std::cmp::Ordering {
            match (val1.value.as_value(), val2.value.as_value()) {
                (Some(v1), Some(v2)) => compare(&val1.key, v1, &val2.key, v2),
                (Some(_), None) => std::cmp::Ordering::Greater,
                (None, Some(_)) => std::cmp::Ordering::Less,
                (None, None) => std::cmp::Ordering::Equal,
            }
        };
        self.items.sort_by(modified_cmp);
        for kv in self.items.values_mut() {
            match &mut kv.value {
                Item::Value(Value::InlineTable(table)) if table.is_dotted() => {
                    table.sort_values_by_internal(compare);
                }
                _ => {}
            }
        }
    }
    /// Change this table's dotted status
    pub fn set_dotted(&mut self, yes: bool) {
        self.dotted = yes;
    }
    /// Check if this is a wrapper for dotted keys, rather than a standard table
    pub fn is_dotted(&self) -> bool {
        self.dotted
    }
    /// Returns the surrounding whitespace
    pub fn decor_mut(&mut self) -> &mut Decor {
        &mut self.decor
    }
    /// Returns the surrounding whitespace
    pub fn decor(&self) -> &Decor {
        &self.decor
    }
    /// Returns the decor associated with a given key of the table.
    pub fn key_decor_mut(&mut self, key: &str) -> Option<&mut Decor> {
        self.items.get_mut(key).map(|kv| &mut kv.key.decor)
    }
    /// Returns the decor associated with a given key of the table.
    pub fn key_decor(&self, key: &str) -> Option<&Decor> {
        self.items.get(key).map(|kv| &kv.key.decor)
    }
    /// Set whitespace after before element
    pub fn set_preamble(&mut self, preamble: impl Into<RawString>) {
        self.preamble = preamble.into();
    }
    /// Whitespace after before element
    pub fn preamble(&self) -> &RawString {
        &self.preamble
    }
    /// Returns the location within the original document
    pub(crate) fn span(&self) -> Option<std::ops::Range<usize>> {
        self.span.clone()
    }
    pub(crate) fn despan(&mut self, input: &str) {
        self.span = None;
        self.decor.despan(input);
        self.preamble.despan(input);
        for kv in self.items.values_mut() {
            kv.key.despan(input);
            kv.value.despan(input);
        }
    }
}
impl InlineTable {
    /// Returns an iterator over key/value pairs.
    pub fn iter(&self) -> InlineTableIter<'_> {
        Box::new(
            self
                .items
                .iter()
                .filter(|&(_, kv)| kv.value.is_value())
                .map(|(k, kv)| (&k[..], kv.value.as_value().unwrap())),
        )
    }
    /// Returns an iterator over key/value pairs.
    pub fn iter_mut(&mut self) -> InlineTableIterMut<'_> {
        Box::new(
            self
                .items
                .iter_mut()
                .filter(|(_, kv)| kv.value.is_value())
                .map(|(_, kv)| (kv.key.as_mut(), kv.value.as_value_mut().unwrap())),
        )
    }
    /// Returns the number of key/value pairs.
    pub fn len(&self) -> usize {
        self.iter().count()
    }
    /// Returns true iff the table is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Clears the table, removing all key-value pairs. Keeps the allocated memory for reuse.
    pub fn clear(&mut self) {
        self.items.clear()
    }
    /// Gets the given key's corresponding entry in the Table for in-place manipulation.
    pub fn entry(&'_ mut self, key: impl Into<InternalString>) -> InlineEntry<'_> {
        match self.items.entry(key.into()) {
            indexmap::map::Entry::Occupied(mut entry) => {
                let scratch = std::mem::take(&mut entry.get_mut().value);
                let scratch = Item::Value(
                    scratch
                        .into_value()
                        .unwrap_or_else(|_| Value::InlineTable(Default::default())),
                );
                entry.get_mut().value = scratch;
                InlineEntry::Occupied(InlineOccupiedEntry { entry })
            }
            indexmap::map::Entry::Vacant(entry) => {
                InlineEntry::Vacant(InlineVacantEntry {
                    entry,
                    key: None,
                })
            }
        }
    }
    /// Gets the given key's corresponding entry in the Table for in-place manipulation.
    pub fn entry_format<'a>(&'a mut self, key: &Key) -> InlineEntry<'a> {
        match self.items.entry(key.get().into()) {
            indexmap::map::Entry::Occupied(mut entry) => {
                let scratch = std::mem::take(&mut entry.get_mut().value);
                let scratch = Item::Value(
                    scratch
                        .into_value()
                        .unwrap_or_else(|_| Value::InlineTable(Default::default())),
                );
                entry.get_mut().value = scratch;
                InlineEntry::Occupied(InlineOccupiedEntry { entry })
            }
            indexmap::map::Entry::Vacant(entry) => {
                InlineEntry::Vacant(InlineVacantEntry {
                    entry,
                    key: Some(key.clone()),
                })
            }
        }
    }
    /// Return an optional reference to the value at the given the key.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.items.get(key).and_then(|kv| kv.value.as_value())
    }
    /// Return an optional mutable reference to the value at the given the key.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Value> {
        self.items.get_mut(key).and_then(|kv| kv.value.as_value_mut())
    }
    /// Return references to the key-value pair stored for key, if it is present, else None.
    pub fn get_key_value<'a>(&'a self, key: &str) -> Option<(&'a Key, &'a Item)> {
        self.items
            .get(key)
            .and_then(|kv| {
                if !kv.value.is_none() { Some((&kv.key, &kv.value)) } else { None }
            })
    }
    /// Return mutable references to the key-value pair stored for key, if it is present, else None.
    pub fn get_key_value_mut<'a>(
        &'a mut self,
        key: &str,
    ) -> Option<(KeyMut<'a>, &'a mut Item)> {
        self.items
            .get_mut(key)
            .and_then(|kv| {
                if !kv.value.is_none() {
                    Some((kv.key.as_mut(), &mut kv.value))
                } else {
                    None
                }
            })
    }
    /// Returns true iff the table contains given key.
    pub fn contains_key(&self, key: &str) -> bool {
        if let Some(kv) = self.items.get(key) { kv.value.is_value() } else { false }
    }
    /// Inserts a key/value pair if the table does not contain the key.
    /// Returns a mutable reference to the corresponding value.
    pub fn get_or_insert<V: Into<Value>>(
        &mut self,
        key: impl Into<InternalString>,
        value: V,
    ) -> &mut Value {
        let key = key.into();
        self.items
            .entry(key.clone())
            .or_insert(TableKeyValue::new(Key::new(key), Item::Value(value.into())))
            .value
            .as_value_mut()
            .expect("non-value type in inline table")
    }
    /// Inserts a key-value pair into the map.
    pub fn insert(
        &mut self,
        key: impl Into<InternalString>,
        value: Value,
    ) -> Option<Value> {
        let key = key.into();
        let kv = TableKeyValue::new(Key::new(key.clone()), Item::Value(value));
        self.items.insert(key, kv).and_then(|kv| kv.value.into_value().ok())
    }
    /// Inserts a key-value pair into the map.
    pub fn insert_formatted(&mut self, key: &Key, value: Value) -> Option<Value> {
        let kv = TableKeyValue::new(key.to_owned(), Item::Value(value));
        self.items
            .insert(InternalString::from(key.get()), kv)
            .filter(|kv| kv.value.is_value())
            .map(|kv| kv.value.into_value().unwrap())
    }
    /// Removes an item given the key.
    pub fn remove(&mut self, key: &str) -> Option<Value> {
        self.items.shift_remove(key).and_then(|kv| kv.value.into_value().ok())
    }
    /// Removes a key from the map, returning the stored key and value if the key was previously in the map.
    pub fn remove_entry(&mut self, key: &str) -> Option<(Key, Value)> {
        self.items
            .shift_remove(key)
            .and_then(|kv| {
                let key = kv.key;
                kv.value.into_value().ok().map(|value| (key, value))
            })
    }
}
impl std::fmt::Display for InlineTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        crate::encode::Encode::encode(self, f, None, ("", ""))
    }
}
impl<K: Into<Key>, V: Into<Value>> Extend<(K, V)> for InlineTable {
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        for (key, value) in iter {
            let key = key.into();
            let value = Item::Value(value.into());
            let value = TableKeyValue::new(key, value);
            self.items.insert(InternalString::from(value.key.get()), value);
        }
    }
}
impl<K: Into<Key>, V: Into<Value>> FromIterator<(K, V)> for InlineTable {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
    {
        let mut table = InlineTable::new();
        table.extend(iter);
        table
    }
}
impl IntoIterator for InlineTable {
    type Item = (InternalString, Value);
    type IntoIter = InlineTableIntoIter;
    fn into_iter(self) -> Self::IntoIter {
        Box::new(
            self
                .items
                .into_iter()
                .filter(|(_, kv)| kv.value.is_value())
                .map(|(k, kv)| (k, kv.value.into_value().unwrap())),
        )
    }
}
impl<'s> IntoIterator for &'s InlineTable {
    type Item = (&'s str, &'s Value);
    type IntoIter = InlineTableIter<'s>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
fn decorate_inline_table(table: &mut InlineTable) {
    for (key_decor, value) in table
        .items
        .iter_mut()
        .filter(|&(_, ref kv)| kv.value.is_value())
        .map(|(_, kv)| (&mut kv.key.decor, kv.value.as_value_mut().unwrap()))
    {
        key_decor.clear();
        value.decor_mut().clear();
    }
}
/// An owned iterator type over key/value pairs of an inline table.
pub type InlineTableIntoIter = Box<dyn Iterator<Item = (InternalString, Value)>>;
/// An iterator type over key/value pairs of an inline table.
pub type InlineTableIter<'a> = Box<dyn Iterator<Item = (&'a str, &'a Value)> + 'a>;
/// A mutable iterator type over key/value pairs of an inline table.
pub type InlineTableIterMut<'a> = Box<
    dyn Iterator<Item = (KeyMut<'a>, &'a mut Value)> + 'a,
>;
impl TableLike for InlineTable {
    fn iter(&self) -> Iter<'_> {
        Box::new(self.items.iter().map(|(key, kv)| (&key[..], &kv.value)))
    }
    fn iter_mut(&mut self) -> IterMut<'_> {
        Box::new(self.items.iter_mut().map(|(_, kv)| (kv.key.as_mut(), &mut kv.value)))
    }
    fn clear(&mut self) {
        self.clear();
    }
    fn entry<'a>(&'a mut self, key: &str) -> crate::Entry<'a> {
        match self.items.entry(key.into()) {
            indexmap::map::Entry::Occupied(entry) => {
                crate::Entry::Occupied(crate::OccupiedEntry { entry })
            }
            indexmap::map::Entry::Vacant(entry) => {
                crate::Entry::Vacant(crate::VacantEntry {
                    entry,
                    key: None,
                })
            }
        }
    }
    fn entry_format<'a>(&'a mut self, key: &Key) -> crate::Entry<'a> {
        match self.items.entry(key.get().into()) {
            indexmap::map::Entry::Occupied(entry) => {
                crate::Entry::Occupied(crate::OccupiedEntry { entry })
            }
            indexmap::map::Entry::Vacant(entry) => {
                crate::Entry::Vacant(crate::VacantEntry {
                    entry,
                    key: Some(key.to_owned()),
                })
            }
        }
    }
    fn get<'s>(&'s self, key: &str) -> Option<&'s Item> {
        self.items.get(key).map(|kv| &kv.value)
    }
    fn get_mut<'s>(&'s mut self, key: &str) -> Option<&'s mut Item> {
        self.items.get_mut(key).map(|kv| &mut kv.value)
    }
    fn get_key_value<'a>(&'a self, key: &str) -> Option<(&'a Key, &'a Item)> {
        self.get_key_value(key)
    }
    fn get_key_value_mut<'a>(
        &'a mut self,
        key: &str,
    ) -> Option<(KeyMut<'a>, &'a mut Item)> {
        self.get_key_value_mut(key)
    }
    fn contains_key(&self, key: &str) -> bool {
        self.contains_key(key)
    }
    fn insert(&mut self, key: &str, value: Item) -> Option<Item> {
        self.insert(key, value.into_value().unwrap()).map(Item::Value)
    }
    fn remove(&mut self, key: &str) -> Option<Item> {
        self.remove(key).map(Item::Value)
    }
    fn get_values(&self) -> Vec<(Vec<&Key>, &Value)> {
        self.get_values()
    }
    fn fmt(&mut self) {
        self.fmt()
    }
    fn sort_values(&mut self) {
        self.sort_values()
    }
    fn set_dotted(&mut self, yes: bool) {
        self.set_dotted(yes)
    }
    fn is_dotted(&self) -> bool {
        self.is_dotted()
    }
    fn key_decor_mut(&mut self, key: &str) -> Option<&mut Decor> {
        self.key_decor_mut(key)
    }
    fn key_decor(&self, key: &str) -> Option<&Decor> {
        self.key_decor(key)
    }
}
pub(crate) const DEFAULT_INLINE_KEY_DECOR: (&str, &str) = (" ", " ");
/// A view into a single location in a map, which may be vacant or occupied.
pub enum InlineEntry<'a> {
    /// An occupied Entry.
    Occupied(InlineOccupiedEntry<'a>),
    /// A vacant Entry.
    Vacant(InlineVacantEntry<'a>),
}
impl<'a> InlineEntry<'a> {
    /// Returns the entry key
    ///
    /// # Examples
    ///
    /// ```
    /// use toml_edit::Table;
    ///
    /// let mut map = Table::new();
    ///
    /// assert_eq!("hello", map.entry("hello").key());
    /// ```
    pub fn key(&self) -> &str {
        match self {
            InlineEntry::Occupied(e) => e.key(),
            InlineEntry::Vacant(e) => e.key(),
        }
    }
    /// Ensures a value is in the entry by inserting the default if empty, and returns
    /// a mutable reference to the value in the entry.
    pub fn or_insert(self, default: Value) -> &'a mut Value {
        match self {
            InlineEntry::Occupied(entry) => entry.into_mut(),
            InlineEntry::Vacant(entry) => entry.insert(default),
        }
    }
    /// Ensures a value is in the entry by inserting the result of the default function if empty,
    /// and returns a mutable reference to the value in the entry.
    pub fn or_insert_with<F: FnOnce() -> Value>(self, default: F) -> &'a mut Value {
        match self {
            InlineEntry::Occupied(entry) => entry.into_mut(),
            InlineEntry::Vacant(entry) => entry.insert(default()),
        }
    }
}
/// A view into a single occupied location in a `IndexMap`.
pub struct InlineOccupiedEntry<'a> {
    entry: indexmap::map::OccupiedEntry<'a, InternalString, TableKeyValue>,
}
impl<'a> InlineOccupiedEntry<'a> {
    /// Gets a reference to the entry key
    ///
    /// # Examples
    ///
    /// ```
    /// use toml_edit::Table;
    ///
    /// let mut map = Table::new();
    ///
    /// assert_eq!("foo", map.entry("foo").key());
    /// ```
    pub fn key(&self) -> &str {
        self.entry.key().as_str()
    }
    /// Gets a mutable reference to the entry key
    pub fn key_mut(&mut self) -> KeyMut<'_> {
        self.entry.get_mut().key.as_mut()
    }
    /// Gets a reference to the value in the entry.
    pub fn get(&self) -> &Value {
        self.entry.get().value.as_value().unwrap()
    }
    /// Gets a mutable reference to the value in the entry.
    pub fn get_mut(&mut self) -> &mut Value {
        self.entry.get_mut().value.as_value_mut().unwrap()
    }
    /// Converts the OccupiedEntry into a mutable reference to the value in the entry
    /// with a lifetime bound to the map itself
    pub fn into_mut(self) -> &'a mut Value {
        self.entry.into_mut().value.as_value_mut().unwrap()
    }
    /// Sets the value of the entry, and returns the entry's old value
    pub fn insert(&mut self, value: Value) -> Value {
        let mut value = Item::Value(value);
        std::mem::swap(&mut value, &mut self.entry.get_mut().value);
        value.into_value().unwrap()
    }
    /// Takes the value out of the entry, and returns it
    pub fn remove(self) -> Value {
        self.entry.shift_remove().value.into_value().unwrap()
    }
}
/// A view into a single empty location in a `IndexMap`.
pub struct InlineVacantEntry<'a> {
    entry: indexmap::map::VacantEntry<'a, InternalString, TableKeyValue>,
    key: Option<Key>,
}
impl<'a> InlineVacantEntry<'a> {
    /// Gets a reference to the entry key
    ///
    /// # Examples
    ///
    /// ```
    /// use toml_edit::Table;
    ///
    /// let mut map = Table::new();
    ///
    /// assert_eq!("foo", map.entry("foo").key());
    /// ```
    pub fn key(&self) -> &str {
        self.entry.key().as_str()
    }
    /// Sets the value of the entry with the VacantEntry's key,
    /// and returns a mutable reference to it
    pub fn insert(self, value: Value) -> &'a mut Value {
        let entry = self.entry;
        let key = self.key.unwrap_or_else(|| Key::new(entry.key().as_str()));
        let value = Item::Value(value);
        entry.insert(TableKeyValue::new(key, value)).value.as_value_mut().unwrap()
    }
}
#[cfg(test)]
mod tests_rug_447 {
    use super::*;
    use crate::inline_table::InlineTable;
    #[test]
    fn test_decorate_inline_table() {
        let _rug_st_tests_rug_447_rrrruuuugggg_test_decorate_inline_table = 0;
        let mut p0: InlineTable = InlineTable::new();
        decorate_inline_table(&mut p0);
        let _rug_ed_tests_rug_447_rrrruuuugggg_test_decorate_inline_table = 0;
    }
}
#[cfg(test)]
mod tests_rug_448 {
    use super::*;
    use crate::inline_table::InlineTable;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_448_rrrruuuugggg_test_rug = 0;
        let _result: InlineTable = InlineTable::new();
        let _rug_ed_tests_rug_448_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_449 {
    use super::*;
    use crate::inline_table::InlineTable;
    use crate::table::TableKeyValue;
    use crate::internal_string::InternalString;
    use indexmap::IndexMap;
    #[test]
    fn test_with_pairs() {
        let _rug_st_tests_rug_449_rrrruuuugggg_test_with_pairs = 0;
        let mut p0: IndexMap<InternalString, TableKeyValue> = IndexMap::new();
        InlineTable::with_pairs(p0);
        let _rug_ed_tests_rug_449_rrrruuuugggg_test_with_pairs = 0;
    }
}
#[cfg(test)]
mod tests_rug_450 {
    use super::*;
    use crate::{Table, Value, InlineTable};
    use serde::de::Error;
    #[test]
    fn test_into_table() {
        let _rug_st_tests_rug_450_rrrruuuugggg_test_into_table = 0;
        let mut p0: InlineTable = InlineTable::new();
        let result: Table = p0.into_table();
        let _rug_ed_tests_rug_450_rrrruuuugggg_test_into_table = 0;
    }
}
#[cfg(test)]
mod tests_rug_451 {
    use super::*;
    use crate::inline_table;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_451_rrrruuuugggg_test_rug = 0;
        let p0: inline_table::InlineTable = inline_table::InlineTable::new();
        crate::inline_table::InlineTable::get_values(&p0);
        let _rug_ed_tests_rug_451_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_452 {
    use super::*;
    use crate::inline_table::{InlineTable, Key, Value};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_452_rrrruuuugggg_test_rug = 0;
        let mut p0 = InlineTable::default();
        let p1: &[&Key] = &[];
        let mut p2: Vec<(Vec<&Key>, &Value)> = Vec::new();
        p0.append_values(p1, &mut p2);
        let _rug_ed_tests_rug_452_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_453 {
    use super::*;
    use crate::inline_table::InlineTable;
    use serde::de::Error;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_453_rrrruuuugggg_test_rug = 0;
        let mut p0: InlineTable = Default::default();
        InlineTable::fmt(&mut p0);
        let _rug_ed_tests_rug_453_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_454 {
    use super::*;
    use crate::{Value, Item, InlineTable};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_454_rrrruuuugggg_test_rug = 0;
        let mut p0 = InlineTable::new();
        p0.sort_values();
        let _rug_ed_tests_rug_454_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_455 {
    use super::*;
    use crate::inline_table::{InlineTable, Key, Value};
    use std::cmp::Ordering;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_455_rrrruuuugggg_test_rug = 0;
        let mut p0 = InlineTable::default();
        let mut p1 = |_: &Key, _: &Value, _: &Key, _: &Value| Ordering::Less;
        InlineTable::sort_values_by(&mut p0, &mut p1);
        let _rug_ed_tests_rug_455_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_456 {
    use super::*;
    use crate::inline_table::InlineTable;
    use std::cmp::Ordering;
    use crate::{key, value, Item, Key, Value};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_456_rrrruuuugggg_test_rug = 0;
        let mut p0 = InlineTable::default();
        let mut p1 = |_: &Key, _: &Value, _: &Key, _: &Value| Ordering::Equal;
        InlineTable::sort_values_by_internal(&mut p0, &mut p1);
        let _rug_ed_tests_rug_456_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_457 {
    use super::*;
    use crate::inline_table::InlineTable;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_457_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = true;
        let mut p0 = InlineTable::new();
        let p1 = rug_fuzz_0;
        p0.set_dotted(p1);
        let _rug_ed_tests_rug_457_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_458 {
    use super::*;
    use crate::inline_table::InlineTable;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_458_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = true;
        let mut p0: InlineTable = InlineTable::new();
        p0.set_dotted(rug_fuzz_0);
        p0.is_dotted();
        let _rug_ed_tests_rug_458_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_459 {
    use super::*;
    use crate::inline_table::InlineTable;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_459_rrrruuuugggg_test_rug = 0;
        let mut p0: InlineTable = InlineTable::new();
        crate::inline_table::InlineTable::decor_mut(&mut p0);
        let _rug_ed_tests_rug_459_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_460 {
    use super::*;
    use crate::inline_table::{Decor, InlineTable};
    use serde::de::Error;
    #[test]
    fn test_decor() {
        let _rug_st_tests_rug_460_rrrruuuugggg_test_decor = 0;
        let mut p0: InlineTable = InlineTable::new();
        let decor = p0.decor();
        let _rug_ed_tests_rug_460_rrrruuuugggg_test_decor = 0;
    }
}
#[cfg(test)]
mod tests_rug_461 {
    use super::*;
    use crate::inline_table::{self, InlineTable};
    #[test]
    fn test_key_decor_mut() {
        let _rug_st_tests_rug_461_rrrruuuugggg_test_key_decor_mut = 0;
        let rug_fuzz_0 = "key";
        let mut p0 = InlineTable::new();
        let p1 = rug_fuzz_0;
        p0.key_decor_mut(p1);
        let _rug_ed_tests_rug_461_rrrruuuugggg_test_key_decor_mut = 0;
    }
}
#[cfg(test)]
mod tests_rug_462 {
    use super::*;
    use crate::inline_table::InlineTable;
    #[test]
    fn test_key_decor() {
        let _rug_st_tests_rug_462_rrrruuuugggg_test_key_decor = 0;
        let rug_fuzz_0 = "key1";
        let rug_fuzz_1 = "value1";
        let rug_fuzz_2 = "key2";
        let rug_fuzz_3 = "value2";
        let rug_fuzz_4 = "key1";
        let rug_fuzz_5 = "key2";
        let mut table = InlineTable::default();
        table[rug_fuzz_0] = rug_fuzz_1.into();
        table[rug_fuzz_2] = rug_fuzz_3.into();
        let key = rug_fuzz_4;
        debug_assert_eq!(table.key_decor(key), None);
        let key2 = rug_fuzz_5;
        debug_assert_eq!(table.key_decor(key2), None);
        let _rug_ed_tests_rug_462_rrrruuuugggg_test_key_decor = 0;
    }
}
#[cfg(test)]
mod tests_rug_463 {
    use super::*;
    use crate::{inline_table, RawString};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_463_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "example preamble";
        let mut p0 = inline_table::InlineTable::default();
        let p1: RawString = rug_fuzz_0.into();
        p0.set_preamble(p1);
        let _rug_ed_tests_rug_463_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_464 {
    use super::*;
    use crate::inline_table::InlineTable;
    use crate::RawString;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_464_rrrruuuugggg_test_rug = 0;
        let mut p0: InlineTable = InlineTable::new();
        InlineTable::preamble(&p0);
        let _rug_ed_tests_rug_464_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_465 {
    use super::*;
    use crate::inline_table::InlineTable;
    #[test]
    fn test_span() {
        let _rug_st_tests_rug_465_rrrruuuugggg_test_span = 0;
        let p0: InlineTable = Default::default();
        InlineTable::span(&p0);
        let _rug_ed_tests_rug_465_rrrruuuugggg_test_span = 0;
    }
}
#[cfg(test)]
mod tests_rug_466 {
    use super::*;
    use crate::inline_table::InlineTable;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_466_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample_input";
        let mut p0: InlineTable = InlineTable::new();
        let p1: &str = rug_fuzz_0;
        p0.despan(p1);
        let _rug_ed_tests_rug_466_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_467 {
    use super::*;
    use crate::de::Error;
    use crate::InlineTable;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_467_rrrruuuugggg_test_rug = 0;
        let p0 = InlineTable::new();
        InlineTable::iter(&p0);
        let _rug_ed_tests_rug_467_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_468 {
    use super::*;
    use crate::inline_table::InlineTable;
    #[test]
    fn test_iter_mut() {
        let _rug_st_tests_rug_468_rrrruuuugggg_test_iter_mut = 0;
        let mut p0: InlineTable = InlineTable::new();
        InlineTable::iter_mut(&mut p0);
        let _rug_ed_tests_rug_468_rrrruuuugggg_test_iter_mut = 0;
    }
}
#[cfg(test)]
mod tests_rug_469 {
    use super::*;
    use crate::inline_table::InlineTable;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_469_rrrruuuugggg_test_rug = 0;
        let mut p0: InlineTable = Default::default();
        InlineTable::len(&p0);
        let _rug_ed_tests_rug_469_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_470 {
    use super::*;
    use crate::inline_table::InlineTable;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_470_rrrruuuugggg_test_rug = 0;
        let mut p0: InlineTable = Default::default();
        debug_assert!(p0.is_empty());
        let _rug_ed_tests_rug_470_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_471 {
    use super::*;
    use crate::inline_table::InlineTable;
    #[test]
    fn test_clear() {
        let _rug_st_tests_rug_471_rrrruuuugggg_test_clear = 0;
        let mut p0 = InlineTable::new();
        InlineTable::clear(&mut p0);
        let _rug_ed_tests_rug_471_rrrruuuugggg_test_clear = 0;
    }
}
#[cfg(test)]
mod tests_rug_472 {
    use super::*;
    use crate::inline_table::InlineTable;
    use std::string::String;
    use crate::value::Value;
    use std::convert::Into;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_472_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key";
        let mut p0: InlineTable = InlineTable::default();
        let mut p1: String = String::from(rug_fuzz_0);
        InlineTable::entry(&mut p0, p1);
        let _rug_ed_tests_rug_472_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_473 {
    use super::*;
    use crate::inline_table::InlineTable;
    use crate::key::Key;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_473_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key";
        let mut p0 = InlineTable::new();
        let p1 = Key::from(rug_fuzz_0);
        InlineTable::entry_format(&mut p0, &p1);
        let _rug_ed_tests_rug_473_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_474 {
    use super::*;
    use crate::inline_table::{self, InlineTable};
    use crate::value::Value;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_474_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key";
        let mut p0: InlineTable = Default::default();
        let p1: &str = rug_fuzz_0;
        p0.get(p1);
        let _rug_ed_tests_rug_474_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_475 {
    use super::*;
    use crate::{Value, InlineTable};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_475_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key";
        let mut p0 = InlineTable::default();
        let p1 = rug_fuzz_0;
        p0.get_mut(p1);
        let _rug_ed_tests_rug_475_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_476 {
    use super::*;
    use crate::InlineTable;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_476_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key1";
        let rug_fuzz_1 = "value1";
        let rug_fuzz_2 = "key2";
        let rug_fuzz_3 = "value2";
        let rug_fuzz_4 = "key1";
        let mut p0 = InlineTable::new();
        p0.get_or_insert(rug_fuzz_0, rug_fuzz_1);
        p0.get_or_insert(rug_fuzz_2, rug_fuzz_3);
        let p1 = rug_fuzz_4;
        p0.get_key_value(&p1);
        let _rug_ed_tests_rug_476_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_477 {
    use super::*;
    use crate::inline_table::InlineTable;
    #[test]
    fn test_get_key_value_mut() {
        let _rug_st_tests_rug_477_rrrruuuugggg_test_get_key_value_mut = 0;
        let rug_fuzz_0 = "key";
        let mut inline_table = InlineTable::new();
        inline_table.get_key_value_mut(rug_fuzz_0);
        let _rug_ed_tests_rug_477_rrrruuuugggg_test_get_key_value_mut = 0;
    }
}
#[cfg(test)]
mod tests_rug_478 {
    use super::*;
    use crate::inline_table::InlineTable;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_478_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key_name";
        let mut p0: InlineTable = InlineTable::new();
        let p1: &str = rug_fuzz_0;
        p0.contains_key(p1);
        let _rug_ed_tests_rug_478_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_479 {
    use super::*;
    use crate::{InlineTable, InternalString, Value};
    #[test]
    fn test_get_or_insert() {
        let _rug_st_tests_rug_479_rrrruuuugggg_test_get_or_insert = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = "value";
        let mut p0 = InlineTable::new();
        let p1: InternalString = rug_fuzz_0.into();
        let p2: Value = rug_fuzz_1.into();
        p0.get_or_insert(p1, p2);
        let _rug_ed_tests_rug_479_rrrruuuugggg_test_get_or_insert = 0;
    }
}
#[cfg(test)]
mod tests_rug_480 {
    use super::*;
    use crate::{inline_table, InternalString, value::Value};
    #[test]
    fn test_insert() {
        let _rug_st_tests_rug_480_rrrruuuugggg_test_insert = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = "value";
        let mut p0 = inline_table::InlineTable::new();
        let p1: InternalString = rug_fuzz_0.into();
        let p2 = Value::from(rug_fuzz_1);
        p0.insert(p1, p2);
        let _rug_ed_tests_rug_480_rrrruuuugggg_test_insert = 0;
    }
}
#[cfg(test)]
mod tests_rug_481 {
    use super::*;
    use crate::inline_table::InlineTable;
    use crate::key::Key;
    use crate::value::Value;
    #[test]
    fn test_insert_formatted() {
        let _rug_st_tests_rug_481_rrrruuuugggg_test_insert_formatted = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = "value";
        let mut inline_table = InlineTable::new();
        let key = Key::from(rug_fuzz_0);
        let value = Value::from(rug_fuzz_1);
        inline_table.insert_formatted(&key, value);
        let _rug_ed_tests_rug_481_rrrruuuugggg_test_insert_formatted = 0;
    }
}
#[cfg(test)]
mod tests_rug_482 {
    use super::*;
    use crate::inline_table::InlineTable;
    #[test]
    fn test_remove() {
        let _rug_st_tests_rug_482_rrrruuuugggg_test_remove = 0;
        let rug_fuzz_0 = "key1";
        let rug_fuzz_1 = "value1";
        let rug_fuzz_2 = "key2";
        let rug_fuzz_3 = 42;
        let rug_fuzz_4 = "key2";
        let mut p0 = InlineTable::new();
        p0[rug_fuzz_0] = Value::from(rug_fuzz_1);
        p0[rug_fuzz_2] = Value::from(rug_fuzz_3);
        let p1 = rug_fuzz_4;
        p0.remove(p1);
        let _rug_ed_tests_rug_482_rrrruuuugggg_test_remove = 0;
    }
}
#[cfg(test)]
mod tests_rug_483 {
    use super::*;
    use crate::inline_table::{InlineTable, Key, Value};
    #[test]
    fn test_remove_entry() {
        let _rug_st_tests_rug_483_rrrruuuugggg_test_remove_entry = 0;
        let rug_fuzz_0 = "key";
        let mut p0 = InlineTable::default();
        let mut p1 = rug_fuzz_0;
        InlineTable::remove_entry(&mut p0, &p1);
        let _rug_ed_tests_rug_483_rrrruuuugggg_test_remove_entry = 0;
    }
}
#[cfg(test)]
mod tests_rug_488 {
    use super::*;
    use crate::{TableLike, InlineTable};
    #[test]
    fn test_iter() {
        let _rug_st_tests_rug_488_rrrruuuugggg_test_iter = 0;
        let mut p0: InlineTable = InlineTable::new();
        p0.iter();
        let _rug_ed_tests_rug_488_rrrruuuugggg_test_iter = 0;
    }
}
#[cfg(test)]
mod tests_rug_490 {
    use super::*;
    use crate::{Table, TableLike};
    #[test]
    fn test_clear() {
        let _rug_st_tests_rug_490_rrrruuuugggg_test_clear = 0;
        let mut p0: Table = Table::new();
        <Table as TableLike>::clear(&mut p0);
        let _rug_ed_tests_rug_490_rrrruuuugggg_test_clear = 0;
    }
}
#[cfg(test)]
mod tests_rug_492 {
    use super::*;
    use crate::TableLike;
    use crate::inline_table::InlineTable;
    use crate::key::Key;
    #[test]
    fn test_entry_format() {
        let _rug_st_tests_rug_492_rrrruuuugggg_test_entry_format = 0;
        let rug_fuzz_0 = "key";
        let mut p0: InlineTable = InlineTable::new();
        let p1: Key = Key::from(rug_fuzz_0);
        p0.entry_format(&p1);
        let _rug_ed_tests_rug_492_rrrruuuugggg_test_entry_format = 0;
    }
}
#[cfg(test)]
mod tests_rug_493 {
    use super::*;
    use crate::{TableLike, Item};
    use crate::inline_table::InlineTable;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_493_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key";
        let mut p0 = InlineTable::new();
        let p1 = rug_fuzz_0;
        <InlineTable as TableLike>::get(&p0, &p1);
        let _rug_ed_tests_rug_493_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_494 {
    use super::*;
    use crate::{InlineTable, TableLike};
    #[test]
    fn test_get_mut() {
        let _rug_st_tests_rug_494_rrrruuuugggg_test_get_mut = 0;
        let rug_fuzz_0 = "key";
        let mut p0: InlineTable = InlineTable::new();
        let mut p1: &str = rug_fuzz_0;
        <InlineTable as TableLike>::get_mut(&mut p0, &p1);
        let _rug_ed_tests_rug_494_rrrruuuugggg_test_get_mut = 0;
    }
}
#[cfg(test)]
mod tests_rug_495 {
    use super::*;
    use crate::TableLike;
    use crate::inline_table::InlineTable;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_495_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample_key";
        let mut p0: InlineTable = InlineTable::new();
        let p1: &str = rug_fuzz_0;
        p0.get_key_value(&p1);
        let _rug_ed_tests_rug_495_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_496 {
    use super::*;
    use crate::TableLike;
    use crate::{Item, KeyMut, InlineTable};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_496_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key_name";
        let mut p0 = InlineTable::new();
        let p1 = rug_fuzz_0;
        p0.get_key_value_mut(p1);
        let _rug_ed_tests_rug_496_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_497 {
    use super::*;
    use crate::{TableLike, inline_table::InlineTable};
    #[test]
    fn test_contains_key() {
        let _rug_st_tests_rug_497_rrrruuuugggg_test_contains_key = 0;
        let rug_fuzz_0 = "key";
        let p0 = InlineTable::new();
        let p1 = rug_fuzz_0;
        <InlineTable as TableLike>::contains_key(&p0, &p1);
        let _rug_ed_tests_rug_497_rrrruuuugggg_test_contains_key = 0;
    }
}
#[cfg(test)]
mod tests_rug_498 {
    use super::*;
    use crate::{TableLike, inline_table::InlineTable, item::Item};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_498_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key";
        let mut p0: InlineTable = InlineTable::new();
        let p1: &str = rug_fuzz_0;
        let p2: Item = Item::None;
        <InlineTable as TableLike>::insert(&mut p0, p1, p2);
        let _rug_ed_tests_rug_498_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_499 {
    use super::*;
    use crate::{Item, TableLike};
    use crate::{InlineTable, de};
    #[test]
    fn test_remove() {
        let _rug_st_tests_rug_499_rrrruuuugggg_test_remove = 0;
        let rug_fuzz_0 = "key";
        let mut p0: InlineTable = InlineTable::default();
        let p1: &str = rug_fuzz_0;
        p0.remove(p1);
        let _rug_ed_tests_rug_499_rrrruuuugggg_test_remove = 0;
    }
}
#[cfg(test)]
mod tests_rug_500 {
    use super::*;
    use crate::{InlineTable, TableLike};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_500_rrrruuuugggg_test_rug = 0;
        let mut p0: InlineTable = InlineTable::default();
        p0.get_values();
        let _rug_ed_tests_rug_500_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_501 {
    use super::*;
    use crate::{TableLike, InlineTable};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_501_rrrruuuugggg_test_rug = 0;
        let mut p0: InlineTable = InlineTable::default();
        <InlineTable as TableLike>::sort_values(&mut p0);
        let _rug_ed_tests_rug_501_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_502 {
    use super::*;
    use crate::{TableLike, Value, InlineTable};
    #[test]
    fn test_set_dotted() {
        let _rug_st_tests_rug_502_rrrruuuugggg_test_set_dotted = 0;
        let rug_fuzz_0 = true;
        let mut inline_table = InlineTable::new();
        let yes = rug_fuzz_0;
        inline_table.set_dotted(yes);
        let _rug_ed_tests_rug_502_rrrruuuugggg_test_set_dotted = 0;
    }
}
#[cfg(test)]
mod tests_rug_505 {
    use super::*;
    use crate::{InlineTable, Decor, TableLike};
    #[test]
    fn test_key_decor() {
        let _rug_st_tests_rug_505_rrrruuuugggg_test_key_decor = 0;
        let rug_fuzz_0 = "sample_key";
        let mut p0: InlineTable = InlineTable::default();
        let p1: &str = rug_fuzz_0;
        <InlineTable as TableLike>::key_decor(&p0, p1);
        let _rug_ed_tests_rug_505_rrrruuuugggg_test_key_decor = 0;
    }
}
#[cfg(test)]
mod tests_rug_507 {
    use super::*;
    use crate::inline_table::InlineEntry;
    use crate::value::Value;
    use serde::de::Error;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_507_rrrruuuugggg_test_rug = 0;
        let mut p0: InlineEntry<'static> = unimplemented!();
        let p1: Value = unimplemented!();
        p0.or_insert(p1);
        let _rug_ed_tests_rug_507_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_509 {
    use super::*;
    use crate::Table;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_509_rrrruuuugggg_test_rug = 0;
        let mut p0: InlineOccupiedEntry = unimplemented!();
        p0.key();
        let _rug_ed_tests_rug_509_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_510 {
    use super::*;
    use crate::inline_table::InlineOccupiedEntry;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_510_rrrruuuugggg_test_rug = 0;
        let mut p0: InlineOccupiedEntry<'_> = unimplemented!();
        InlineOccupiedEntry::key_mut(&mut p0);
        let _rug_ed_tests_rug_510_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_513 {
    use super::*;
    use crate::inline_table::InlineOccupiedEntry;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_513_rrrruuuugggg_test_rug = 0;
        let mut p0: InlineOccupiedEntry<'static> = unimplemented!();
        InlineOccupiedEntry::<'static>::into_mut(p0);
        let _rug_ed_tests_rug_513_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_514 {
    use super::*;
    use crate::inline_table::InlineOccupiedEntry;
    use crate::value::Value;
    use std::mem;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_514_rrrruuuugggg_test_rug = 0;
        let mut p0: InlineOccupiedEntry = unimplemented!();
        let p1: Value = unimplemented!();
        InlineOccupiedEntry::insert(&mut p0, p1);
        let _rug_ed_tests_rug_514_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_515 {
    use super::*;
    use crate::inline_table::InlineOccupiedEntry;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_515_rrrruuuugggg_test_rug = 0;
        let mut p0: InlineOccupiedEntry = todo!();
        p0.remove();
        let _rug_ed_tests_rug_515_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_516 {
    use super::*;
    use crate::Table;
    #[test]
    fn test_key() {
        let _rug_st_tests_rug_516_rrrruuuugggg_test_key = 0;
        let rug_fuzz_0 = "foo";
        let rug_fuzz_1 = "foo";
        let mut map = Table::new();
        let entry = map.entry(rug_fuzz_0);
        debug_assert_eq!(rug_fuzz_1, entry.key());
        let _rug_ed_tests_rug_516_rrrruuuugggg_test_key = 0;
    }
}
