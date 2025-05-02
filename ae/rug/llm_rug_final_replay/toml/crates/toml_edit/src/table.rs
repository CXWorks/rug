use std::iter::FromIterator;
use indexmap::map::IndexMap;
use crate::key::Key;
use crate::repr::Decor;
use crate::value::DEFAULT_VALUE_DECOR;
use crate::{InlineTable, InternalString, Item, KeyMut, Value};
/// Type representing a TOML non-inline table
#[derive(Clone, Debug, Default)]
pub struct Table {
    pub(crate) decor: Decor,
    pub(crate) implicit: bool,
    pub(crate) dotted: bool,
    doc_position: Option<usize>,
    pub(crate) span: Option<std::ops::Range<usize>>,
    pub(crate) items: KeyValuePairs,
}
/// Constructors
///
/// See also `FromIterator`
impl Table {
    /// Creates an empty table.
    pub fn new() -> Self {
        Default::default()
    }
    pub(crate) fn with_pos(doc_position: Option<usize>) -> Self {
        Self {
            doc_position,
            ..Default::default()
        }
    }
    pub(crate) fn with_pairs(items: KeyValuePairs) -> Self {
        Self {
            items,
            ..Default::default()
        }
    }
    /// Convert to an inline table
    pub fn into_inline_table(mut self) -> InlineTable {
        for (_, kv) in self.items.iter_mut() {
            kv.value.make_value();
        }
        let mut t = InlineTable::with_pairs(self.items);
        t.fmt();
        t
    }
}
/// Formatting
impl Table {
    /// Get key/values for values that are visually children of this table
    ///
    /// For example, this will return dotted keys
    pub fn get_values(&self) -> Vec<(Vec<&Key>, &Value)> {
        let mut values = Vec::new();
        let root = Vec::new();
        self.append_values(&root, &mut values);
        values
    }
    fn append_values<'s, 'c>(
        &'s self,
        parent: &[&'s Key],
        values: &'c mut Vec<(Vec<&'s Key>, &'s Value)>,
    ) {
        for value in self.items.values() {
            let mut path = parent.to_vec();
            path.push(&value.key);
            match &value.value {
                Item::Table(table) if table.is_dotted() => {
                    table.append_values(&path, values);
                }
                Item::Value(value) => {
                    if let Some(table) = value.as_inline_table() {
                        if table.is_dotted() {
                            table.append_values(&path, values);
                        } else {
                            values.push((path, value));
                        }
                    } else {
                        values.push((path, value));
                    }
                }
                _ => {}
            }
        }
    }
    /// Auto formats the table.
    pub fn fmt(&mut self) {
        decorate_table(self);
    }
    /// Sorts Key/Value Pairs of the table.
    ///
    /// Doesn't affect subtables or subarrays.
    pub fn sort_values(&mut self) {
        self.items.sort_keys();
        for kv in self.items.values_mut() {
            match &mut kv.value {
                Item::Table(table) if table.is_dotted() => {
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
        F: FnMut(&Key, &Item, &Key, &Item) -> std::cmp::Ordering,
    {
        self.sort_values_by_internal(&mut compare);
    }
    fn sort_values_by_internal<F>(&mut self, compare: &mut F)
    where
        F: FnMut(&Key, &Item, &Key, &Item) -> std::cmp::Ordering,
    {
        let modified_cmp = |
            _: &InternalString,
            val1: &TableKeyValue,
            _: &InternalString,
            val2: &TableKeyValue,
        | -> std::cmp::Ordering {
            compare(&val1.key, &val1.value, &val2.key, &val2.value)
        };
        self.items.sort_by(modified_cmp);
        for kv in self.items.values_mut() {
            match &mut kv.value {
                Item::Table(table) if table.is_dotted() => {
                    table.sort_values_by_internal(compare);
                }
                _ => {}
            }
        }
    }
    /// If a table has no key/value pairs and implicit, it will not be displayed.
    ///
    /// # Examples
    ///
    /// ```notrust
    /// [target."x86_64/windows.json".dependencies]
    /// ```
    ///
    /// In the document above, tables `target` and `target."x86_64/windows.json"` are implicit.
    ///
    /// ```
    /// use toml_edit::Document;
    /// let mut doc = "[a]\n[a.b]\n".parse::<Document>().expect("invalid toml");
    ///
    /// doc["a"].as_table_mut().unwrap().set_implicit(true);
    /// assert_eq!(doc.to_string(), "[a.b]\n");
    /// ```
    pub fn set_implicit(&mut self, implicit: bool) {
        self.implicit = implicit;
    }
    /// If a table has no key/value pairs and implicit, it will not be displayed.
    pub fn is_implicit(&self) -> bool {
        self.implicit
    }
    /// Change this table's dotted status
    pub fn set_dotted(&mut self, yes: bool) {
        self.dotted = yes;
    }
    /// Check if this is a wrapper for dotted keys, rather than a standard table
    pub fn is_dotted(&self) -> bool {
        self.dotted
    }
    /// Sets the position of the `Table` within the `Document`.
    pub fn set_position(&mut self, doc_position: usize) {
        self.doc_position = Some(doc_position);
    }
    /// The position of the `Table` within the `Document`.
    ///
    /// Returns `None` if the `Table` was created manually (i.e. not via parsing)
    /// in which case its position is set automatically.  This can be overridden with
    /// [`Table::set_position`].
    pub fn position(&self) -> Option<usize> {
        self.doc_position
    }
    /// Returns the surrounding whitespace
    pub fn decor_mut(&mut self) -> &mut Decor {
        &mut self.decor
    }
    /// Returns the decor associated with a given key of the table.
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
    /// Returns the location within the original document
    pub(crate) fn span(&self) -> Option<std::ops::Range<usize>> {
        self.span.clone()
    }
    pub(crate) fn despan(&mut self, input: &str) {
        self.span = None;
        self.decor.despan(input);
        for kv in self.items.values_mut() {
            kv.key.despan(input);
            kv.value.despan(input);
        }
    }
}
impl Table {
    /// Returns an iterator over all key/value pairs, including empty.
    pub fn iter(&self) -> Iter<'_> {
        Box::new(
            self
                .items
                .iter()
                .filter(|(_, kv)| !kv.value.is_none())
                .map(|(key, kv)| (&key[..], &kv.value)),
        )
    }
    /// Returns an mutable iterator over all key/value pairs, including empty.
    pub fn iter_mut(&mut self) -> IterMut<'_> {
        Box::new(
            self
                .items
                .iter_mut()
                .filter(|(_, kv)| !kv.value.is_none())
                .map(|(_, kv)| (kv.key.as_mut(), &mut kv.value)),
        )
    }
    /// Returns the number of non-empty items in the table.
    pub fn len(&self) -> usize {
        self.items.iter().filter(|i| !(i.1).value.is_none()).count()
    }
    /// Returns true if the table is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Clears the table, removing all key-value pairs. Keeps the allocated memory for reuse.
    pub fn clear(&mut self) {
        self.items.clear()
    }
    /// Gets the given key's corresponding entry in the Table for in-place manipulation.
    pub fn entry<'a>(&'a mut self, key: &str) -> Entry<'a> {
        match self.items.entry(key.into()) {
            indexmap::map::Entry::Occupied(entry) => {
                Entry::Occupied(OccupiedEntry { entry })
            }
            indexmap::map::Entry::Vacant(entry) => {
                Entry::Vacant(VacantEntry { entry, key: None })
            }
        }
    }
    /// Gets the given key's corresponding entry in the Table for in-place manipulation.
    pub fn entry_format<'a>(&'a mut self, key: &Key) -> Entry<'a> {
        match self.items.entry(key.get().into()) {
            indexmap::map::Entry::Occupied(entry) => {
                Entry::Occupied(OccupiedEntry { entry })
            }
            indexmap::map::Entry::Vacant(entry) => {
                Entry::Vacant(VacantEntry {
                    entry,
                    key: Some(key.to_owned()),
                })
            }
        }
    }
    /// Returns an optional reference to an item given the key.
    pub fn get<'a>(&'a self, key: &str) -> Option<&'a Item> {
        self.items
            .get(key)
            .and_then(|kv| { if !kv.value.is_none() { Some(&kv.value) } else { None } })
    }
    /// Returns an optional mutable reference to an item given the key.
    pub fn get_mut<'a>(&'a mut self, key: &str) -> Option<&'a mut Item> {
        self.items
            .get_mut(key)
            .and_then(|kv| {
                if !kv.value.is_none() { Some(&mut kv.value) } else { None }
            })
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
    /// Returns true if the table contains an item with the given key.
    pub fn contains_key(&self, key: &str) -> bool {
        if let Some(kv) = self.items.get(key) { !kv.value.is_none() } else { false }
    }
    /// Returns true if the table contains a table with the given key.
    pub fn contains_table(&self, key: &str) -> bool {
        if let Some(kv) = self.items.get(key) { kv.value.is_table() } else { false }
    }
    /// Returns true if the table contains a value with the given key.
    pub fn contains_value(&self, key: &str) -> bool {
        if let Some(kv) = self.items.get(key) { kv.value.is_value() } else { false }
    }
    /// Returns true if the table contains an array of tables with the given key.
    pub fn contains_array_of_tables(&self, key: &str) -> bool {
        if let Some(kv) = self.items.get(key) {
            kv.value.is_array_of_tables()
        } else {
            false
        }
    }
    /// Inserts a key-value pair into the map.
    pub fn insert(&mut self, key: &str, item: Item) -> Option<Item> {
        let kv = TableKeyValue::new(Key::new(key), item);
        self.items.insert(key.into(), kv).map(|kv| kv.value)
    }
    /// Inserts a key-value pair into the map.
    pub fn insert_formatted(&mut self, key: &Key, item: Item) -> Option<Item> {
        let kv = TableKeyValue::new(key.to_owned(), item);
        self.items.insert(key.get().into(), kv).map(|kv| kv.value)
    }
    /// Removes an item given the key.
    pub fn remove(&mut self, key: &str) -> Option<Item> {
        self.items.shift_remove(key).map(|kv| kv.value)
    }
    /// Removes a key from the map, returning the stored key and value if the key was previously in the map.
    pub fn remove_entry(&mut self, key: &str) -> Option<(Key, Item)> {
        self.items.shift_remove(key).map(|kv| (kv.key, kv.value))
    }
}
impl std::fmt::Display for Table {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use crate::encode::Encode;
        let children = self.get_values();
        for (key_path, value) in children {
            key_path.as_slice().encode(f, None, DEFAULT_KEY_DECOR)?;
            write!(f, "=")?;
            value.encode(f, None, DEFAULT_VALUE_DECOR)?;
            writeln!(f)?;
        }
        Ok(())
    }
}
impl<K: Into<Key>, V: Into<Value>> Extend<(K, V)> for Table {
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        for (key, value) in iter {
            let key = key.into();
            let value = Item::Value(value.into());
            let value = TableKeyValue::new(key, value);
            self.items.insert(value.key.get().into(), value);
        }
    }
}
impl<K: Into<Key>, V: Into<Value>> FromIterator<(K, V)> for Table {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
    {
        let mut table = Table::new();
        table.extend(iter);
        table
    }
}
impl IntoIterator for Table {
    type Item = (InternalString, Item);
    type IntoIter = IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        Box::new(self.items.into_iter().map(|(k, kv)| (k, kv.value)))
    }
}
impl<'s> IntoIterator for &'s Table {
    type Item = (&'s str, &'s Item);
    type IntoIter = Iter<'s>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
pub(crate) type KeyValuePairs = IndexMap<InternalString, TableKeyValue>;
fn decorate_table(table: &mut Table) {
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
pub(crate) const DEFAULT_KEY_DECOR: (&str, &str) = ("", " ");
pub(crate) const DEFAULT_TABLE_DECOR: (&str, &str) = ("\n", "");
pub(crate) const DEFAULT_KEY_PATH_DECOR: (&str, &str) = ("", "");
#[derive(Debug, Clone)]
pub(crate) struct TableKeyValue {
    pub(crate) key: Key,
    pub(crate) value: Item,
}
impl TableKeyValue {
    pub(crate) fn new(key: Key, value: Item) -> Self {
        TableKeyValue { key, value }
    }
}
/// An owned iterator type over `Table`'s key/value pairs.
pub type IntoIter = Box<dyn Iterator<Item = (InternalString, Item)>>;
/// An iterator type over `Table`'s key/value pairs.
pub type Iter<'a> = Box<dyn Iterator<Item = (&'a str, &'a Item)> + 'a>;
/// A mutable iterator type over `Table`'s key/value pairs.
pub type IterMut<'a> = Box<dyn Iterator<Item = (KeyMut<'a>, &'a mut Item)> + 'a>;
/// This trait represents either a `Table`, or an `InlineTable`.
pub trait TableLike: crate::private::Sealed {
    /// Returns an iterator over key/value pairs.
    fn iter(&self) -> Iter<'_>;
    /// Returns an mutable iterator over all key/value pairs, including empty.
    fn iter_mut(&mut self) -> IterMut<'_>;
    /// Returns the number of nonempty items.
    fn len(&self) -> usize {
        self.iter().filter(|&(_, v)| !v.is_none()).count()
    }
    /// Returns true if the table is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Clears the table, removing all key-value pairs. Keeps the allocated memory for reuse.
    fn clear(&mut self);
    /// Gets the given key's corresponding entry in the Table for in-place manipulation.
    fn entry<'a>(&'a mut self, key: &str) -> Entry<'a>;
    /// Gets the given key's corresponding entry in the Table for in-place manipulation.
    fn entry_format<'a>(&'a mut self, key: &Key) -> Entry<'a>;
    /// Returns an optional reference to an item given the key.
    fn get<'s>(&'s self, key: &str) -> Option<&'s Item>;
    /// Returns an optional mutable reference to an item given the key.
    fn get_mut<'s>(&'s mut self, key: &str) -> Option<&'s mut Item>;
    /// Return references to the key-value pair stored for key, if it is present, else None.
    fn get_key_value<'a>(&'a self, key: &str) -> Option<(&'a Key, &'a Item)>;
    /// Return mutable references to the key-value pair stored for key, if it is present, else None.
    fn get_key_value_mut<'a>(
        &'a mut self,
        key: &str,
    ) -> Option<(KeyMut<'a>, &'a mut Item)>;
    /// Returns true if the table contains an item with the given key.
    fn contains_key(&self, key: &str) -> bool;
    /// Inserts a key-value pair into the map.
    fn insert(&mut self, key: &str, value: Item) -> Option<Item>;
    /// Removes an item given the key.
    fn remove(&mut self, key: &str) -> Option<Item>;
    /// Get key/values for values that are visually children of this table
    ///
    /// For example, this will return dotted keys
    fn get_values(&self) -> Vec<(Vec<&Key>, &Value)>;
    /// Auto formats the table.
    fn fmt(&mut self);
    /// Sorts Key/Value Pairs of the table.
    ///
    /// Doesn't affect subtables or subarrays.
    fn sort_values(&mut self);
    /// Change this table's dotted status
    fn set_dotted(&mut self, yes: bool);
    /// Check if this is a wrapper for dotted keys, rather than a standard table
    fn is_dotted(&self) -> bool;
    /// Returns the decor associated with a given key of the table.
    fn key_decor_mut(&mut self, key: &str) -> Option<&mut Decor>;
    /// Returns the decor associated with a given key of the table.
    fn key_decor(&self, key: &str) -> Option<&Decor>;
}
impl TableLike for Table {
    fn iter(&self) -> Iter<'_> {
        self.iter()
    }
    fn iter_mut(&mut self) -> IterMut<'_> {
        self.iter_mut()
    }
    fn clear(&mut self) {
        self.clear();
    }
    fn entry<'a>(&'a mut self, key: &str) -> Entry<'a> {
        self.entry(key)
    }
    fn entry_format<'a>(&'a mut self, key: &Key) -> Entry<'a> {
        self.entry_format(key)
    }
    fn get<'s>(&'s self, key: &str) -> Option<&'s Item> {
        self.get(key)
    }
    fn get_mut<'s>(&'s mut self, key: &str) -> Option<&'s mut Item> {
        self.get_mut(key)
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
        self.insert(key, value)
    }
    fn remove(&mut self, key: &str) -> Option<Item> {
        self.remove(key)
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
    fn is_dotted(&self) -> bool {
        self.is_dotted()
    }
    fn set_dotted(&mut self, yes: bool) {
        self.set_dotted(yes)
    }
    fn key_decor_mut(&mut self, key: &str) -> Option<&mut Decor> {
        self.key_decor_mut(key)
    }
    fn key_decor(&self, key: &str) -> Option<&Decor> {
        self.key_decor(key)
    }
}
/// A view into a single location in a map, which may be vacant or occupied.
pub enum Entry<'a> {
    /// An occupied Entry.
    Occupied(OccupiedEntry<'a>),
    /// A vacant Entry.
    Vacant(VacantEntry<'a>),
}
impl<'a> Entry<'a> {
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
            Entry::Occupied(e) => e.key(),
            Entry::Vacant(e) => e.key(),
        }
    }
    /// Ensures a value is in the entry by inserting the default if empty, and returns
    /// a mutable reference to the value in the entry.
    pub fn or_insert(self, default: Item) -> &'a mut Item {
        match self {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(default),
        }
    }
    /// Ensures a value is in the entry by inserting the result of the default function if empty,
    /// and returns a mutable reference to the value in the entry.
    pub fn or_insert_with<F: FnOnce() -> Item>(self, default: F) -> &'a mut Item {
        match self {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(default()),
        }
    }
}
/// A view into a single occupied location in a `IndexMap`.
pub struct OccupiedEntry<'a> {
    pub(crate) entry: indexmap::map::OccupiedEntry<'a, InternalString, TableKeyValue>,
}
impl<'a> OccupiedEntry<'a> {
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
    pub fn get(&self) -> &Item {
        &self.entry.get().value
    }
    /// Gets a mutable reference to the value in the entry.
    pub fn get_mut(&mut self) -> &mut Item {
        &mut self.entry.get_mut().value
    }
    /// Converts the OccupiedEntry into a mutable reference to the value in the entry
    /// with a lifetime bound to the map itself
    pub fn into_mut(self) -> &'a mut Item {
        &mut self.entry.into_mut().value
    }
    /// Sets the value of the entry, and returns the entry's old value
    pub fn insert(&mut self, mut value: Item) -> Item {
        std::mem::swap(&mut value, &mut self.entry.get_mut().value);
        value
    }
    /// Takes the value out of the entry, and returns it
    pub fn remove(self) -> Item {
        self.entry.shift_remove().value
    }
}
/// A view into a single empty location in a `IndexMap`.
pub struct VacantEntry<'a> {
    pub(crate) entry: indexmap::map::VacantEntry<'a, InternalString, TableKeyValue>,
    pub(crate) key: Option<Key>,
}
impl<'a> VacantEntry<'a> {
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
    pub fn insert(self, value: Item) -> &'a mut Item {
        let entry = self.entry;
        let key = self.key.unwrap_or_else(|| Key::new(entry.key().as_str()));
        &mut entry.insert(TableKeyValue::new(key, value)).value
    }
}
#[cfg(test)]
mod tests_rug_710 {
    use super::*;
    use crate::de::Error as SerdeError;
    use crate::table::Table;
    #[test]
    fn test_decorate_table() {
        let _rug_st_tests_rug_710_rrrruuuugggg_test_decorate_table = 0;
        let mut p0: Table = Table::default();
        crate::table::decorate_table(&mut p0);
        let _rug_ed_tests_rug_710_rrrruuuugggg_test_decorate_table = 0;
    }
}
#[cfg(test)]
mod tests_rug_711 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_711_rrrruuuugggg_test_rug = 0;
        let mut p0 = Table::new();
        p0.len();
        let _rug_ed_tests_rug_711_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_712 {
    use super::*;
    use crate::InlineTable;
    use crate::Table;
    #[test]
    fn test_table_is_empty() {
        let _rug_st_tests_rug_712_rrrruuuugggg_test_table_is_empty = 0;
        let rug_fuzz_0 = "key1";
        let rug_fuzz_1 = "value1";
        let rug_fuzz_2 = "key1";
        let mut p0: Table = Table::new();
        debug_assert!(p0.is_empty());
        p0.insert(rug_fuzz_0, Item::Value(Value::from(rug_fuzz_1)));
        debug_assert!(! p0.is_empty());
        p0.remove(rug_fuzz_2);
        debug_assert!(p0.is_empty());
        let _rug_ed_tests_rug_712_rrrruuuugggg_test_table_is_empty = 0;
    }
    #[test]
    fn test_inline_table_is_empty() {
        let _rug_st_tests_rug_712_rrrruuuugggg_test_inline_table_is_empty = 0;
        let rug_fuzz_0 = "key1";
        let rug_fuzz_1 = "value1";
        let rug_fuzz_2 = "key1";
        let mut p0: InlineTable = InlineTable::new();
        debug_assert!(p0.is_empty());
        p0.insert(rug_fuzz_0, Value::from(rug_fuzz_1));
        debug_assert!(! p0.is_empty());
        p0.remove(rug_fuzz_2);
        debug_assert!(p0.is_empty());
        let _rug_ed_tests_rug_712_rrrruuuugggg_test_inline_table_is_empty = 0;
    }
}
#[cfg(test)]
mod tests_rug_713 {
    use super::*;
    use crate::table::Table;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_713_rrrruuuugggg_test_rug = 0;
        let table: Table = Table::new();
        let _rug_ed_tests_rug_713_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_714 {
    use super::*;
    use crate::table::Table;
    #[test]
    fn test_with_pos() {
        let _rug_st_tests_rug_714_rrrruuuugggg_test_with_pos = 0;
        let rug_fuzz_0 = 10;
        let p0: Option<usize> = Some(rug_fuzz_0);
        Table::with_pos(p0);
        let _rug_ed_tests_rug_714_rrrruuuugggg_test_with_pos = 0;
    }
}
#[cfg(test)]
mod tests_rug_716 {
    use super::*;
    use crate::table::Table;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_716_rrrruuuugggg_test_rug = 0;
        let mut p0: Table = Table::new();
        crate::table::Table::into_inline_table(p0);
        let _rug_ed_tests_rug_716_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_717 {
    use super::*;
    use crate::{de, Table};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_717_rrrruuuugggg_test_rug = 0;
        let mut p0: Table = Table::new();
        Table::get_values(&p0);
        let _rug_ed_tests_rug_717_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_718 {
    use super::*;
    use crate::key::Key;
    use crate::value::Value;
    use crate::table::Table;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_718_rrrruuuugggg_test_rug = 0;
        let mut p0: Table = Table::new();
        let p1: [&Key; 0] = [];
        let mut p2: Vec<(Vec<&Key>, &Value)> = Vec::new();
        p0.append_values(&p1, &mut p2);
        let _rug_ed_tests_rug_718_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_719 {
    use super::*;
    use crate::table::Table;
    use serde::de::Error;
    #[test]
    fn test_fmt() {
        let _rug_st_tests_rug_719_rrrruuuugggg_test_fmt = 0;
        let mut p0: Table = Table::new();
        Table::fmt(&mut p0);
        let _rug_ed_tests_rug_719_rrrruuuugggg_test_fmt = 0;
    }
}
#[cfg(test)]
mod tests_rug_720 {
    use super::*;
    use crate::table::Table;
    use crate::Item;
    #[test]
    fn test_sort_values() {
        let _rug_st_tests_rug_720_rrrruuuugggg_test_sort_values = 0;
        let mut p0: Table = Table::new();
        Table::sort_values(&mut p0);
        let _rug_ed_tests_rug_720_rrrruuuugggg_test_sort_values = 0;
    }
}
#[cfg(test)]
mod tests_rug_721 {
    use super::*;
    use crate::table::Table;
    use std::cmp::Ordering;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_721_rrrruuuugggg_test_rug = 0;
        let mut p0: Table = Table::new();
        let mut p1 = |k1: &Key, i1: &Item, k2: &Key, i2: &Item| -> Ordering {
            Ordering::Equal
        };
        p0.sort_values_by(p1);
        let _rug_ed_tests_rug_721_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_722 {
    use super::*;
    use crate::table::{Table, TableKeyValue, InternalString, Item, Key};
    use std::cmp::Ordering;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_722_rrrruuuugggg_test_rug = 0;
        let mut p0: Table = Table::new();
        let mut p1: &mut dyn FnMut(&Key, &Item, &Key, &Item) -> Ordering = &mut |
            _,
            _,
            _,
            _|
        Ordering::Equal;
        p0.sort_values_by_internal(&mut p1);
        let _rug_ed_tests_rug_722_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_723 {
    use super::*;
    use crate::table::Table;
    #[test]
    fn test_set_implicit() {
        let _rug_st_tests_rug_723_rrrruuuugggg_test_set_implicit = 0;
        let rug_fuzz_0 = true;
        let mut table = Table::new();
        let implicit = rug_fuzz_0;
        table.set_implicit(implicit);
        let _rug_ed_tests_rug_723_rrrruuuugggg_test_set_implicit = 0;
    }
}
#[cfg(test)]
mod tests_rug_724 {
    use super::*;
    use crate::table::Table;
    use serde::de::Error;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_724_rrrruuuugggg_test_rug = 0;
        let mut p0: Table = Table::new();
        Table::is_implicit(&p0);
        let _rug_ed_tests_rug_724_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_725 {
    use super::*;
    use crate::table;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_725_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = true;
        let mut p0 = table::Table::default();
        let p1 = rug_fuzz_0;
        p0.set_dotted(p1);
        let _rug_ed_tests_rug_725_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_726 {
    use super::*;
    use crate::table::Table;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_726_rrrruuuugggg_test_rug = 0;
        let mut p0: Table = Table::default();
        Table::is_dotted(&p0);
        let _rug_ed_tests_rug_726_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_727 {
    use super::*;
    use crate::table::Table;
    #[test]
    fn test_set_position() {
        let _rug_st_tests_rug_727_rrrruuuugggg_test_set_position = 0;
        let rug_fuzz_0 = 10;
        let mut p0 = Table::new();
        let p1 = rug_fuzz_0;
        p0.set_position(p1);
        let _rug_ed_tests_rug_727_rrrruuuugggg_test_set_position = 0;
    }
}
#[cfg(test)]
mod tests_rug_728 {
    use super::*;
    use crate::table::Table;
    use serde::de::Error;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_728_rrrruuuugggg_test_rug = 0;
        let mut p0: Table = Table::default();
        Table::position(&p0);
        let _rug_ed_tests_rug_728_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_729 {
    use super::*;
    use crate::de;
    use serde::de::Error;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_729_rrrruuuugggg_test_rug = 0;
        let mut p0 = Table::new();
        Table::decor_mut(&mut p0);
        let _rug_ed_tests_rug_729_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_730 {
    use super::*;
    use crate::table::Table;
    use crate::de::Error;
    use serde::de::Error as DeError;
    #[test]
    fn test_decor() {
        let _rug_st_tests_rug_730_rrrruuuugggg_test_decor = 0;
        let mut p0: Table = Table::new();
        let decor = Table::decor(&p0);
        let _rug_ed_tests_rug_730_rrrruuuugggg_test_decor = 0;
    }
}
#[cfg(test)]
mod tests_rug_731 {
    use super::*;
    use crate::{table::Table, Decor};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_731_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key";
        let mut p0: Table = Table::new();
        let mut p1: &str = rug_fuzz_0;
        p0.key_decor_mut(p1);
        let _rug_ed_tests_rug_731_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_732 {
    use super::*;
    use crate::table::Table;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_732_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample_key";
        let mut p0: Table = Table::new();
        let p1: &str = rug_fuzz_0;
        <Table>::key_decor(&p0, &p1);
        let _rug_ed_tests_rug_732_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_733 {
    use super::*;
    use serde::de::Error;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_733_rrrruuuugggg_test_rug = 0;
        let p0: Table = Table::new();
        Table::span(&p0);
        let _rug_ed_tests_rug_733_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_734 {
    use super::*;
    use crate::table::Table;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_734_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "input string";
        let mut p0: Table = Default::default();
        let p1: &str = rug_fuzz_0;
        p0.despan(p1);
        let _rug_ed_tests_rug_734_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_735 {
    use super::*;
    use crate::de;
    use serde::de::Error;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_735_rrrruuuugggg_test_rug = 0;
        let mut p0: Table = Table::new();
        crate::table::Table::iter(&p0);
        let _rug_ed_tests_rug_735_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_736 {
    use super::*;
    use crate::table::Table;
    use serde::de::Error;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_736_rrrruuuugggg_test_rug = 0;
        let mut p0: Table = Table::new();
        Table::iter_mut(&mut p0);
        let _rug_ed_tests_rug_736_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_737 {
    use super::*;
    use crate::de::Error;
    use serde::de::Error as DeError;
    #[test]
    fn test_len() {
        let _rug_st_tests_rug_737_rrrruuuugggg_test_len = 0;
        let mut p0: Table = Table::new();
        crate::table::Table::len(&p0);
        let _rug_ed_tests_rug_737_rrrruuuugggg_test_len = 0;
    }
}
#[cfg(test)]
mod tests_rug_738 {
    use super::*;
    use crate::table::Table;
    use serde::de::Error;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_738_rrrruuuugggg_test_rug = 0;
        let mut p0: Table = Table::new();
        Table::is_empty(&p0);
        let _rug_ed_tests_rug_738_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_739 {
    use super::*;
    use crate::table::Table;
    use serde::de::Error;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_739_rrrruuuugggg_test_rug = 0;
        let mut p0: Table = Table::new();
        Table::clear(&mut p0);
        let _rug_ed_tests_rug_739_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_740 {
    use super::*;
    use indexmap::map::Entry;
    use crate::table::{Table, OccupiedEntry, VacantEntry};
    #[test]
    fn test_entry() {
        let _rug_st_tests_rug_740_rrrruuuugggg_test_entry = 0;
        let rug_fuzz_0 = "key";
        let mut p0: Table = Table::new();
        let p1: &str = rug_fuzz_0;
        p0.entry(p1);
        let _rug_ed_tests_rug_740_rrrruuuugggg_test_entry = 0;
    }
}
#[cfg(test)]
mod tests_rug_741 {
    use super::*;
    use crate::table::{Table, Entry, Key};
    #[test]
    fn test_entry_format() {
        let _rug_st_tests_rug_741_rrrruuuugggg_test_entry_format = 0;
        let rug_fuzz_0 = "key_name";
        let mut table = Table::new();
        let key = Key::new(rug_fuzz_0);
        let entry = table.entry_format(&key);
        let _rug_ed_tests_rug_741_rrrruuuugggg_test_entry_format = 0;
    }
}
#[cfg(test)]
mod tests_rug_742 {
    use super::*;
    use crate::table;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_742_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key";
        let mut p0: table::Table = Table::new();
        let p1 = rug_fuzz_0;
        p0.get(p1);
        let _rug_ed_tests_rug_742_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_743 {
    use super::*;
    use crate::table::Table;
    #[test]
    fn test_get_mut() {
        let _rug_st_tests_rug_743_rrrruuuugggg_test_get_mut = 0;
        let rug_fuzz_0 = "some_key";
        let mut p0: Table = Table::new();
        let p1: &str = rug_fuzz_0;
        p0.get_mut(p1);
        let _rug_ed_tests_rug_743_rrrruuuugggg_test_get_mut = 0;
    }
}
#[cfg(test)]
mod tests_rug_744 {
    use super::*;
    use crate::table::Table;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_744_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample_key";
        let mut p0: Table = Table::new();
        let p1: &str = rug_fuzz_0;
        p0.get_key_value(p1);
        let _rug_ed_tests_rug_744_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_745 {
    use super::*;
    use crate::table::{Table, KeyMut, Item};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_745_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key1";
        let mut p0: Table = Table::new();
        let mut p1 = rug_fuzz_0;
        p0.get_key_value_mut(p1);
        let _rug_ed_tests_rug_745_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_746 {
    use super::*;
    use crate::table::Table;
    #[test]
    fn test_contains_key() {
        let _rug_st_tests_rug_746_rrrruuuugggg_test_contains_key = 0;
        let rug_fuzz_0 = "key";
        let mut p0 = Table::default();
        let mut p1 = rug_fuzz_0;
        p0.contains_key(&p1);
        let _rug_ed_tests_rug_746_rrrruuuugggg_test_contains_key = 0;
    }
}
#[cfg(test)]
mod tests_rug_747 {
    use super::*;
    use crate::table::Table;
    #[test]
    fn test_contains_table() {
        let _rug_st_tests_rug_747_rrrruuuugggg_test_contains_table = 0;
        let rug_fuzz_0 = "test";
        let mut table = Table::default();
        let key = rug_fuzz_0;
        debug_assert_eq!(table.contains_table(key), false);
        let _rug_ed_tests_rug_747_rrrruuuugggg_test_contains_table = 0;
    }
}
#[cfg(test)]
mod tests_rug_748 {
    use super::*;
    use crate::table::Table;
    #[test]
    fn test_contains_value() {
        let _rug_st_tests_rug_748_rrrruuuugggg_test_contains_value = 0;
        let rug_fuzz_0 = "key";
        let mut p0 = Table::default();
        let p1 = rug_fuzz_0;
        debug_assert_eq!(p0.contains_value(& p1), false);
        let _rug_ed_tests_rug_748_rrrruuuugggg_test_contains_value = 0;
    }
}
#[cfg(test)]
mod tests_rug_749 {
    use super::*;
    use crate::table::Table;
    #[test]
    fn test_contains_array_of_tables() {
        let _rug_st_tests_rug_749_rrrruuuugggg_test_contains_array_of_tables = 0;
        let rug_fuzz_0 = "key";
        let mut p0 = Table::new();
        let p1 = rug_fuzz_0;
        p0.contains_array_of_tables(&p1);
        let _rug_ed_tests_rug_749_rrrruuuugggg_test_contains_array_of_tables = 0;
    }
}
#[cfg(test)]
mod tests_rug_750 {
    use super::*;
    use crate::table::Table;
    use crate::item::Item;
    #[test]
    fn test_insert() {
        let _rug_st_tests_rug_750_rrrruuuugggg_test_insert = 0;
        let rug_fuzz_0 = "key";
        let mut p0 = Table::new();
        let p1 = rug_fuzz_0;
        let p2 = Item::None;
        p0.insert(p1, p2);
        let _rug_ed_tests_rug_750_rrrruuuugggg_test_insert = 0;
    }
}
#[cfg(test)]
mod tests_rug_752 {
    use super::*;
    use crate::{Table, Item};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_752_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key";
        let mut p0: Table = Table::new();
        let mut p1: &str = rug_fuzz_0;
        Table::remove(&mut p0, p1);
        let _rug_ed_tests_rug_752_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_753 {
    use super::*;
    use crate::table::{Table, Key, Item};
    #[test]
    fn test_remove_entry() {
        let _rug_st_tests_rug_753_rrrruuuugggg_test_remove_entry = 0;
        let rug_fuzz_0 = "key";
        let mut p0 = Table::new();
        let mut p1 = rug_fuzz_0;
        p0.remove_entry(p1);
        let _rug_ed_tests_rug_753_rrrruuuugggg_test_remove_entry = 0;
    }
}
#[cfg(test)]
mod tests_rug_759 {
    use super::*;
    use crate::{TableLike, de};
    #[test]
    fn test_iter() {
        let _rug_st_tests_rug_759_rrrruuuugggg_test_iter = 0;
        let mut p0 = Table::new();
        p0.iter();
        let _rug_ed_tests_rug_759_rrrruuuugggg_test_iter = 0;
    }
}
#[cfg(test)]
mod tests_rug_760 {
    use super::*;
    use crate::TableLike;
    #[test]
    fn test_iter_mut() {
        let _rug_st_tests_rug_760_rrrruuuugggg_test_iter_mut = 0;
        let mut p0: Table = Table::new();
        p0.iter_mut();
        let _rug_ed_tests_rug_760_rrrruuuugggg_test_iter_mut = 0;
    }
}
#[cfg(test)]
mod tests_rug_763 {
    use super::*;
    use crate::TableLike;
    use crate::table::Table;
    use crate::key::Key;
    #[test]
    fn test_entry_format() {
        let _rug_st_tests_rug_763_rrrruuuugggg_test_entry_format = 0;
        let rug_fuzz_0 = "key";
        let mut p0: Table = Table::new();
        let p1: Key = Key::from(rug_fuzz_0);
        p0.entry_format(&p1);
        let _rug_ed_tests_rug_763_rrrruuuugggg_test_entry_format = 0;
    }
}
#[cfg(test)]
mod tests_rug_764 {
    use super::*;
    use crate::{TableLike, table::{Table, Item}};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_764_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key";
        let mut p0: Table = Table::new();
        let mut p1: &str = rug_fuzz_0;
        <Table as TableLike>::get(&p0, &p1);
        let _rug_ed_tests_rug_764_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_765 {
    use super::*;
    use crate::{table::TableLike, table::Table};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_765_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "some_key";
        let mut tbl: Table = Table::new();
        let key = rug_fuzz_0;
        <Table as TableLike>::get_mut(&mut tbl, key);
        let _rug_ed_tests_rug_765_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_766 {
    use super::*;
    use crate::{Table, TableLike};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_766_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key1";
        let mut p0: Table = Table::new();
        let p1: &str = rug_fuzz_0;
        <Table as TableLike>::get_key_value(&p0, &p1);
        let _rug_ed_tests_rug_766_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_767 {
    use super::*;
    use crate::{Table, TableLike};
    #[test]
    fn test_get_key_value_mut() {
        let _rug_st_tests_rug_767_rrrruuuugggg_test_get_key_value_mut = 0;
        let rug_fuzz_0 = "key";
        let mut p0 = Table::new();
        let p1 = rug_fuzz_0;
        p0.get_key_value_mut(&p1);
        let _rug_ed_tests_rug_767_rrrruuuugggg_test_get_key_value_mut = 0;
    }
}
#[cfg(test)]
mod tests_rug_768 {
    use super::*;
    use crate::{TableLike, table::Table};
    #[test]
    fn test_contains_key() {
        let _rug_st_tests_rug_768_rrrruuuugggg_test_contains_key = 0;
        let rug_fuzz_0 = "some_key";
        let mut p0 = Table::new();
        let p1 = rug_fuzz_0;
        p0.contains_key(&p1);
        let _rug_ed_tests_rug_768_rrrruuuugggg_test_contains_key = 0;
    }
}
#[cfg(test)]
mod tests_rug_770 {
    use super::*;
    use crate::{Table, TableLike};
    #[test]
    fn test_remove() {
        let _rug_st_tests_rug_770_rrrruuuugggg_test_remove = 0;
        let rug_fuzz_0 = "key";
        let mut p0 = Table::new();
        let p1 = rug_fuzz_0;
        Table::remove(&mut p0, &p1);
        let _rug_ed_tests_rug_770_rrrruuuugggg_test_remove = 0;
    }
}
#[cfg(test)]
mod tests_rug_771 {
    use super::*;
    use crate::{Table, TableLike};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_771_rrrruuuugggg_test_rug = 0;
        let mut p0: Table = Table::new();
        <Table as TableLike>::get_values(&p0);
        let _rug_ed_tests_rug_771_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_772 {
    use super::*;
    use crate::TableLike;
    #[test]
    fn test_sort_values() {
        let _rug_st_tests_rug_772_rrrruuuugggg_test_sort_values = 0;
        let mut p0: Table = Table::new();
        <Table as TableLike>::sort_values(&mut p0);
        let _rug_ed_tests_rug_772_rrrruuuugggg_test_sort_values = 0;
    }
}
#[cfg(test)]
mod tests_rug_773 {
    use super::*;
    use crate::{Table, TableLike};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_773_rrrruuuugggg_test_rug = 0;
        let mut p0 = Table::new();
        p0.is_dotted();
        let _rug_ed_tests_rug_773_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_774 {
    use super::*;
    use crate::{Table, TableLike};
    #[test]
    fn test_set_dotted() {
        let _rug_st_tests_rug_774_rrrruuuugggg_test_set_dotted = 0;
        let rug_fuzz_0 = true;
        let mut p0 = Table::new();
        let p1: bool = rug_fuzz_0;
        p0.set_dotted(p1);
        let _rug_ed_tests_rug_774_rrrruuuugggg_test_set_dotted = 0;
    }
}
#[cfg(test)]
mod tests_rug_775 {
    use super::*;
    use crate::{Decor, Table, TableLike};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_775_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample_key";
        let mut p0: Table = Table::new();
        let p1: &str = rug_fuzz_0;
        <Table as TableLike>::key_decor_mut(&mut p0, p1);
        let _rug_ed_tests_rug_775_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_776 {
    use super::*;
    use crate::TableLike;
    #[test]
    fn test_key_decor() {
        let _rug_st_tests_rug_776_rrrruuuugggg_test_key_decor = 0;
        let rug_fuzz_0 = "sample data";
        let mut p0 = crate::Table::new();
        let p1 = rug_fuzz_0;
        p0.key_decor(p1);
        let _rug_ed_tests_rug_776_rrrruuuugggg_test_key_decor = 0;
    }
}
#[cfg(test)]
mod tests_rug_778 {
    use super::*;
    use crate::table::Entry;
    use crate::item::Item;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_778_rrrruuuugggg_test_rug = 0;
        let mut p0: Entry<'static> = unimplemented!();
        let p1: Item = unimplemented!();
        p0.or_insert(p1);
        let _rug_ed_tests_rug_778_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_780 {
    use super::*;
    use crate::Table;
    use crate::table::OccupiedEntry;
    #[test]
    fn test_key() {
        let _rug_st_tests_rug_780_rrrruuuugggg_test_key = 0;
        let rug_fuzz_0 = "foo";
        let mut map = Table::new();
        let key = rug_fuzz_0;
        map.entry(key).key();
        let _rug_ed_tests_rug_780_rrrruuuugggg_test_key = 0;
    }
}
#[cfg(test)]
mod tests_rug_781 {
    use super::*;
    use crate::table::OccupiedEntry;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_781_rrrruuuugggg_test_rug = 0;
        let mut p0: OccupiedEntry<'_> = unimplemented!();
        OccupiedEntry::<'_>::key_mut(&mut p0);
        let _rug_ed_tests_rug_781_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_782 {
    use super::*;
    use crate::table::{self, OccupiedEntry};
    #[test]
    fn test_rug() {
        let mut p0: OccupiedEntry<'_> = table::OccupiedEntry {
            entry: todo!("construct Entry<'a> type here"),
        };
        p0.get();
    }
}
#[cfg(test)]
mod tests_rug_784 {
    use super::*;
    use crate::table::{OccupiedEntry, Item};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_784_rrrruuuugggg_test_rug = 0;
        let mut p0: OccupiedEntry<'static> = unreachable!();
        OccupiedEntry::<'static>::into_mut(p0);
        let _rug_ed_tests_rug_784_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_785 {
    use super::*;
    use crate::table::OccupiedEntry;
    use crate::item::Item;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_785_rrrruuuugggg_test_rug = 0;
        let mut p0: OccupiedEntry<'static> = unimplemented!();
        let mut p1: Item = unimplemented!();
        OccupiedEntry::<'static>::insert(&mut p0, p1);
        let _rug_ed_tests_rug_785_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_786 {
    use super::*;
    use crate::table::OccupiedEntry;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_786_rrrruuuugggg_test_rug = 0;
        let mut p0: OccupiedEntry<'static> = unimplemented!();
        OccupiedEntry::remove(p0);
        let _rug_ed_tests_rug_786_rrrruuuugggg_test_rug = 0;
    }
}
