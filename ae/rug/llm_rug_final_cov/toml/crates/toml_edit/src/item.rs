use std::str::FromStr;
use toml_datetime::*;
use crate::array_of_tables::ArrayOfTables;
use crate::table::TableLike;
use crate::{Array, InlineTable, Table, Value};
/// Type representing either a value, a table, an array of tables, or none.
#[derive(Debug, Clone)]
pub enum Item {
    /// Type representing none.
    None,
    /// Type representing value.
    Value(Value),
    /// Type representing table.
    Table(Table),
    /// Type representing array of tables.
    ArrayOfTables(ArrayOfTables),
}
impl Item {
    /// Sets `self` to the given item iff `self` is none and
    /// returns a mutable reference to `self`.
    pub fn or_insert(&mut self, item: Item) -> &mut Item {
        if self.is_none() {
            *self = item;
        }
        self
    }
}
/// Downcasting
impl Item {
    /// Text description of value type
    pub fn type_name(&self) -> &'static str {
        match self {
            Item::None => "none",
            Item::Value(v) => v.type_name(),
            Item::Table(..) => "table",
            Item::ArrayOfTables(..) => "array of tables",
        }
    }
    /// Index into a TOML array or map. A string index can be used to access a
    /// value in a map, and a usize index can be used to access an element of an
    /// array.
    ///
    /// Returns `None` if:
    /// - The type of `self` does not match the type of the
    ///   index, for example if the index is a string and `self` is an array or a
    ///   number.
    /// - The given key does not exist in the map
    ///   or the given index is not within the bounds of the array.
    pub fn get<I: crate::index::Index>(&self, index: I) -> Option<&Item> {
        index.index(self)
    }
    /// Mutably index into a TOML array or map. A string index can be used to
    /// access a value in a map, and a usize index can be used to access an
    /// element of an array.
    ///
    /// Returns `None` if:
    /// - The type of `self` does not match the type of the
    ///   index, for example if the index is a string and `self` is an array or a
    ///   number.
    /// - The given key does not exist in the map
    ///   or the given index is not within the bounds of the array.
    pub fn get_mut<I: crate::index::Index>(&mut self, index: I) -> Option<&mut Item> {
        index.index_mut(self)
    }
    /// Casts `self` to value.
    pub fn as_value(&self) -> Option<&Value> {
        match *self {
            Item::Value(ref v) => Some(v),
            _ => None,
        }
    }
    /// Casts `self` to table.
    pub fn as_table(&self) -> Option<&Table> {
        match *self {
            Item::Table(ref t) => Some(t),
            _ => None,
        }
    }
    /// Casts `self` to array of tables.
    pub fn as_array_of_tables(&self) -> Option<&ArrayOfTables> {
        match *self {
            Item::ArrayOfTables(ref a) => Some(a),
            _ => None,
        }
    }
    /// Casts `self` to mutable value.
    pub fn as_value_mut(&mut self) -> Option<&mut Value> {
        match *self {
            Item::Value(ref mut v) => Some(v),
            _ => None,
        }
    }
    /// Casts `self` to mutable table.
    pub fn as_table_mut(&mut self) -> Option<&mut Table> {
        match *self {
            Item::Table(ref mut t) => Some(t),
            _ => None,
        }
    }
    /// Casts `self` to mutable array of tables.
    pub fn as_array_of_tables_mut(&mut self) -> Option<&mut ArrayOfTables> {
        match *self {
            Item::ArrayOfTables(ref mut a) => Some(a),
            _ => None,
        }
    }
    /// Casts `self` to value.
    pub fn into_value(self) -> Result<Value, Self> {
        match self {
            Item::None => Err(self),
            Item::Value(v) => Ok(v),
            Item::Table(v) => {
                let v = v.into_inline_table();
                Ok(Value::InlineTable(v))
            }
            Item::ArrayOfTables(v) => {
                let v = v.into_array();
                Ok(Value::Array(v))
            }
        }
    }
    /// In-place convert to a value
    pub fn make_value(&mut self) {
        let other = std::mem::take(self);
        let other = other.into_value().map(Item::Value).unwrap_or(Item::None);
        *self = other;
    }
    /// Casts `self` to table.
    pub fn into_table(self) -> Result<Table, Self> {
        match self {
            Item::Table(t) => Ok(t),
            Item::Value(Value::InlineTable(t)) => Ok(t.into_table()),
            _ => Err(self),
        }
    }
    /// Casts `self` to array of tables.
    pub fn into_array_of_tables(self) -> Result<ArrayOfTables, Self> {
        match self {
            Item::ArrayOfTables(a) => Ok(a),
            Item::Value(Value::Array(a)) => {
                if a.is_empty() {
                    Err(Item::Value(Value::Array(a)))
                } else if a.iter().all(|v| v.is_inline_table()) {
                    let mut aot = ArrayOfTables::new();
                    aot.values = a.values;
                    for value in aot.values.iter_mut() {
                        value.make_item();
                    }
                    Ok(aot)
                } else {
                    Err(Item::Value(Value::Array(a)))
                }
            }
            _ => Err(self),
        }
    }
    pub(crate) fn make_item(&mut self) {
        let other = std::mem::take(self);
        let other = match other.into_table().map(crate::Item::Table) {
            Ok(i) => i,
            Err(i) => i,
        };
        let other = match other.into_array_of_tables().map(crate::Item::ArrayOfTables) {
            Ok(i) => i,
            Err(i) => i,
        };
        *self = other;
    }
    /// Returns true iff `self` is a value.
    pub fn is_value(&self) -> bool {
        self.as_value().is_some()
    }
    /// Returns true iff `self` is a table.
    pub fn is_table(&self) -> bool {
        self.as_table().is_some()
    }
    /// Returns true iff `self` is an array of tables.
    pub fn is_array_of_tables(&self) -> bool {
        self.as_array_of_tables().is_some()
    }
    /// Returns true iff `self` is `None`.
    pub fn is_none(&self) -> bool {
        matches!(* self, Item::None)
    }
    /// Casts `self` to integer.
    pub fn as_integer(&self) -> Option<i64> {
        self.as_value().and_then(Value::as_integer)
    }
    /// Returns true iff `self` is an integer.
    pub fn is_integer(&self) -> bool {
        self.as_integer().is_some()
    }
    /// Casts `self` to float.
    pub fn as_float(&self) -> Option<f64> {
        self.as_value().and_then(Value::as_float)
    }
    /// Returns true iff `self` is a float.
    pub fn is_float(&self) -> bool {
        self.as_float().is_some()
    }
    /// Casts `self` to boolean.
    pub fn as_bool(&self) -> Option<bool> {
        self.as_value().and_then(Value::as_bool)
    }
    /// Returns true iff `self` is a boolean.
    pub fn is_bool(&self) -> bool {
        self.as_bool().is_some()
    }
    /// Casts `self` to str.
    pub fn as_str(&self) -> Option<&str> {
        self.as_value().and_then(Value::as_str)
    }
    /// Returns true iff `self` is a string.
    pub fn is_str(&self) -> bool {
        self.as_str().is_some()
    }
    /// Casts `self` to date-time.
    pub fn as_datetime(&self) -> Option<&Datetime> {
        self.as_value().and_then(Value::as_datetime)
    }
    /// Returns true iff `self` is a date-time.
    pub fn is_datetime(&self) -> bool {
        self.as_datetime().is_some()
    }
    /// Casts `self` to array.
    pub fn as_array(&self) -> Option<&Array> {
        self.as_value().and_then(Value::as_array)
    }
    /// Casts `self` to mutable array.
    pub fn as_array_mut(&mut self) -> Option<&mut Array> {
        self.as_value_mut().and_then(Value::as_array_mut)
    }
    /// Returns true iff `self` is an array.
    pub fn is_array(&self) -> bool {
        self.as_array().is_some()
    }
    /// Casts `self` to inline table.
    pub fn as_inline_table(&self) -> Option<&InlineTable> {
        self.as_value().and_then(Value::as_inline_table)
    }
    /// Casts `self` to mutable inline table.
    pub fn as_inline_table_mut(&mut self) -> Option<&mut InlineTable> {
        self.as_value_mut().and_then(Value::as_inline_table_mut)
    }
    /// Returns true iff `self` is an inline table.
    pub fn is_inline_table(&self) -> bool {
        self.as_inline_table().is_some()
    }
    /// Casts `self` to either a table or an inline table.
    pub fn as_table_like(&self) -> Option<&dyn TableLike> {
        self.as_table()
            .map(|t| t as &dyn TableLike)
            .or_else(|| self.as_inline_table().map(|t| t as &dyn TableLike))
    }
    /// Casts `self` to either a table or an inline table.
    pub fn as_table_like_mut(&mut self) -> Option<&mut dyn TableLike> {
        match self {
            Item::Table(t) => Some(t as &mut dyn TableLike),
            Item::Value(Value::InlineTable(t)) => Some(t as &mut dyn TableLike),
            _ => None,
        }
    }
    /// Returns true iff `self` is either a table, or an inline table.
    pub fn is_table_like(&self) -> bool {
        self.as_table_like().is_some()
    }
    /// Returns the location within the original document
    pub(crate) fn span(&self) -> Option<std::ops::Range<usize>> {
        match self {
            Item::None => None,
            Item::Value(v) => v.span(),
            Item::Table(v) => v.span(),
            Item::ArrayOfTables(v) => v.span(),
        }
    }
    pub(crate) fn despan(&mut self, input: &str) {
        match self {
            Item::None => {}
            Item::Value(v) => v.despan(input),
            Item::Table(v) => v.despan(input),
            Item::ArrayOfTables(v) => v.despan(input),
        }
    }
}
impl Default for Item {
    fn default() -> Self {
        Item::None
    }
}
impl FromStr for Item {
    type Err = crate::TomlError;
    /// Parses a value from a &str
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value = s.parse::<Value>()?;
        Ok(Item::Value(value))
    }
}
impl std::fmt::Display for Item {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Item::None => Ok(()),
            Item::Value(v) => v.fmt(f),
            Item::Table(v) => v.fmt(f),
            Item::ArrayOfTables(v) => v.fmt(f),
        }
    }
}
/// Returns a formatted value.
///
/// Since formatting is part of a `Value`, the right hand side of the
/// assignment needs to be decorated with a space before the value.
/// The `value` function does just that.
///
/// # Examples
/// ```rust
/// # use snapbox::assert_eq;
/// # use toml_edit::*;
/// let mut table = Table::default();
/// let mut array = Array::default();
/// array.push("hello");
/// array.push("\\, world"); // \ is only allowed in a literal string
/// table["key1"] = value("value1");
/// table["key2"] = value(42);
/// table["key3"] = value(array);
/// assert_eq(table.to_string(),
/// r#"key1 = "value1"
/// key2 = 42
/// key3 = ["hello", '\, world']
/// "#);
/// ```
pub fn value<V: Into<Value>>(v: V) -> Item {
    Item::Value(v.into())
}
/// Returns an empty table.
pub fn table() -> Item {
    Item::Table(Table::new())
}
/// Returns an empty array of tables.
pub fn array() -> Item {
    Item::ArrayOfTables(ArrayOfTables::new())
}
#[cfg(test)]
mod tests_rug_518 {
    use super::*;
    use crate::{Item, Value, Table, Array};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_518_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "value1";
        let rug_fuzz_1 = "hello";
        let rug_fuzz_2 = "\\, world";
        let rug_fuzz_3 = "key1";
        let rug_fuzz_4 = "key2";
        let rug_fuzz_5 = 42;
        let rug_fuzz_6 = "key3";
        let rug_fuzz_7 = r#"key1 = "value1"
key2 = 42
key3 = ["hello", '\, world']
"#;
        let mut p0: Value = rug_fuzz_0.into();
        let mut table = Table::default();
        let mut array = Array::default();
        array.push(rug_fuzz_1);
        array.push(rug_fuzz_2);
        table[rug_fuzz_3] = value(p0);
        table[rug_fuzz_4] = value(rug_fuzz_5);
        table[rug_fuzz_6] = value(array);
        let expected_output = rug_fuzz_7;
        debug_assert_eq!(table.to_string(), expected_output);
        let _rug_ed_tests_rug_518_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_519 {
    use super::*;
    use crate::item::Item;
    use crate::item::Table;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_519_rrrruuuugggg_test_rug = 0;
        table();
        let _rug_ed_tests_rug_519_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_520 {
    use super::*;
    use crate::item::Item;
    use crate::item::ArrayOfTables;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_520_rrrruuuugggg_test_rug = 0;
        array();
        let _rug_ed_tests_rug_520_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_521 {
    use super::*;
    use crate::item::Item;
    #[test]
    fn test_or_insert() {
        let _rug_st_tests_rug_521_rrrruuuugggg_test_or_insert = 0;
        let mut p0 = Item::None;
        let p1 = Item::Table(crate::table::Table::new());
        p0.or_insert(p1);
        let _rug_ed_tests_rug_521_rrrruuuugggg_test_or_insert = 0;
    }
}
#[cfg(test)]
mod tests_rug_522 {
    use super::*;
    use crate::item::Item;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_522_rrrruuuugggg_test_rug = 0;
        let mut p0: Item = Item::None;
        crate::item::Item::type_name(&p0);
        let _rug_ed_tests_rug_522_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_525 {
    use super::*;
    use crate::value::Value;
    use crate::item::Item;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_525_rrrruuuugggg_test_rug = 0;
        let mut p0: Item = Item::None;
        Item::as_value(&p0);
        let _rug_ed_tests_rug_525_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_526 {
    use super::*;
    use crate::item::{Item, Table};
    use serde::de::Error;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_526_rrrruuuugggg_test_rug = 0;
        let mut p0: Item = Item::Table(Table::new());
        <Item>::as_table(&p0);
        let _rug_ed_tests_rug_526_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_527 {
    use super::*;
    use crate::item::{Item, ArrayOfTables};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_527_rrrruuuugggg_test_rug = 0;
        let mut p0: &Item = &Item::ArrayOfTables(ArrayOfTables::new());
        <Item>::as_array_of_tables(p0);
        let _rug_ed_tests_rug_527_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_530 {
    use super::*;
    use crate::item::Item;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_530_rrrruuuugggg_test_rug = 0;
        let mut p0: Item = Item::ArrayOfTables(ArrayOfTables::new());
        <Item>::as_array_of_tables_mut(&mut p0);
        let _rug_ed_tests_rug_530_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_531 {
    use super::*;
    use crate::de::Error;
    use crate::item::Item;
    use serde::de::Error as SerdeError;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_531_rrrruuuugggg_test_rug = 0;
        let mut p0: Item = Item::None;
        crate::item::Item::into_value(p0);
        let _rug_ed_tests_rug_531_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_532 {
    use super::*;
    use std::convert::TryInto;
    use crate::item::Item;
    #[test]
    fn test_make_value() {
        let _rug_st_tests_rug_532_rrrruuuugggg_test_make_value = 0;
        let rug_fuzz_0 = "test_value";
        let mut p0: Item = Item::Value(rug_fuzz_0.try_into().unwrap());
        p0.make_value();
        let _rug_ed_tests_rug_532_rrrruuuugggg_test_make_value = 0;
    }
}
#[cfg(test)]
mod tests_rug_533 {
    use super::*;
    use crate::item::{Item, Table, Value};
    #[test]
    fn test_into_table() {
        let _rug_st_tests_rug_533_rrrruuuugggg_test_into_table = 0;
        let mut p0: Item = Item::Table(Table::new());
        let result = p0.into_table();
        debug_assert!(result.is_ok());
        let _rug_ed_tests_rug_533_rrrruuuugggg_test_into_table = 0;
    }
}
#[cfg(test)]
mod tests_rug_534 {
    use super::*;
    use crate::item::{Item, ArrayOfTables, Value};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_534_rrrruuuugggg_test_rug = 0;
        let mut p0 = Item::ArrayOfTables(ArrayOfTables::new());
        crate::item::Item::into_array_of_tables(p0);
        let _rug_ed_tests_rug_534_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_535 {
    use super::*;
    use crate::item::Item;
    #[test]
    fn test_make_item() {
        let _rug_st_tests_rug_535_rrrruuuugggg_test_make_item = 0;
        let mut p0: Item = todo!();
        Item::make_item(&mut p0);
        let _rug_ed_tests_rug_535_rrrruuuugggg_test_make_item = 0;
    }
}
#[cfg(test)]
mod tests_rug_536 {
    use super::*;
    use crate::de::Error as DeError;
    use crate::Item;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_536_rrrruuuugggg_test_rug = 0;
        let mut p0: Item = Item::Table(Table::new());
        p0.is_value();
        let _rug_ed_tests_rug_536_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_537 {
    use super::*;
    use crate::Item;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_537_rrrruuuugggg_test_rug = 0;
        let p0: Item = Item::default();
        Item::is_table(&p0);
        let _rug_ed_tests_rug_537_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_538 {
    use super::*;
    use crate::item::Item;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_538_rrrruuuugggg_test_rug = 0;
        let mut p0: Item = Item::Table(Table::new());
        Item::is_array_of_tables(&p0);
        let _rug_ed_tests_rug_538_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_539 {
    use super::*;
    use crate::item::Item;
    use serde::de::Error;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_539_rrrruuuugggg_test_rug = 0;
        let mut p0: Item = Item::None;
        Item::is_none(&p0);
        let _rug_ed_tests_rug_539_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_542 {
    use super::*;
    use crate::item::Item;
    use crate::value::Value;
    use serde::de::Error;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_542_rrrruuuugggg_test_rug = 0;
        let mut p0: Item = Item::Table(crate::table::Table::new());
        Item::as_float(&p0);
        let _rug_ed_tests_rug_542_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_544 {
    use super::*;
    use crate::item::Item;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_544_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "true";
        let mut p0 = Item::Value(rug_fuzz_0.to_owned().into());
        Item::as_bool(&p0);
        let _rug_ed_tests_rug_544_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_546 {
    use super::*;
    use crate::item;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_546_rrrruuuugggg_test_rug = 0;
        let mut p0: item::Item = Default::default();
        item::Item::as_str(&p0);
        let _rug_ed_tests_rug_546_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_547 {
    use super::*;
    use crate::item::Item;
    #[test]
    fn test_rug() {
        let p0: Item = todo!("construct Item here");
        Item::is_str(&p0);
    }
}
#[cfg(test)]
mod tests_rug_548 {
    use super::*;
    use crate::de::Error;
    use crate::Datetime;
    use crate::Value;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_548_rrrruuuugggg_test_rug = 0;
        let mut p0 = Item::Table(Table::new());
        Item::as_datetime(&p0);
        let _rug_ed_tests_rug_548_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_549 {
    use super::*;
    use crate::item::Item;
    use serde::de::Error;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_549_rrrruuuugggg_test_rug = 0;
        let mut p0: Item = Item::default();
        p0.is_datetime();
        let _rug_ed_tests_rug_549_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_550 {
    use super::*;
    use crate::item;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_550_rrrruuuugggg_test_rug = 0;
        let mut p0 = item::Item::Table(item::Table::new());
        <item::Item>::as_array(&p0);
        let _rug_ed_tests_rug_550_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_552 {
    use super::*;
    use crate::item::Item;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_552_rrrruuuugggg_test_rug = 0;
        let mut p0: Item = Item::None;
        Item::is_array(&p0);
        let _rug_ed_tests_rug_552_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_553 {
    use super::*;
    use crate::{InlineTable, Value};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_553_rrrruuuugggg_test_rug = 0;
        let mut p0: Item = unimplemented!();
        p0.as_inline_table();
        let _rug_ed_tests_rug_553_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_555 {
    use super::*;
    use crate::item;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_555_rrrruuuugggg_test_rug = 0;
        let mut p0: item::Item = unimplemented!();
        item::Item::is_inline_table(&p0);
        let _rug_ed_tests_rug_555_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_556 {
    use super::*;
    use crate::item::{self, TableLike};
    #[test]
    fn test_rug() {
        let mut p0: item::Item = unimplemented!("construct the item here");
        item::Item::as_table_like(&p0);
    }
}
#[cfg(test)]
mod tests_rug_557 {
    use super::*;
    use crate::item::{Item, TableLike, Value};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_557_rrrruuuugggg_test_rug = 0;
        let mut p0: &mut Item = &mut Item::Table(Default::default());
        <Item>::as_table_like_mut(p0);
        let _rug_ed_tests_rug_557_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_558 {
    use super::*;
    use crate::item::Item;
    use crate::de;
    use serde::de::Error;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_558_rrrruuuugggg_test_rug = 0;
        let mut p0: Item = Item::Table(crate::table::Table::new());
        Item::is_table_like(&p0);
        let _rug_ed_tests_rug_558_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_559 {
    use super::*;
    use crate::Item;
    use std::ops::Range;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_559_rrrruuuugggg_test_rug = 0;
        let p0: Item = Item::None;
        let result: Option<Range<usize>> = crate::item::Item::span(&p0);
        let _rug_ed_tests_rug_559_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_560 {
    use super::*;
    use crate::item;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_560_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample_data";
        let mut p0 = item::Item::None;
        let p1 = rug_fuzz_0;
        p0.despan(p1);
        let _rug_ed_tests_rug_560_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_561 {
    use super::*;
    use crate::item::Item;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_561_rrrruuuugggg_test_rug = 0;
        <Item as std::default::Default>::default();
        let _rug_ed_tests_rug_561_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_562 {
    use super::*;
    use crate::item::Item;
    use crate::value::Value;
    #[test]
    fn test_from_str() {
        let _rug_st_tests_rug_562_rrrruuuugggg_test_from_str = 0;
        let rug_fuzz_0 = "42";
        let p0 = rug_fuzz_0;
        let result = <Item as FromStr>::from_str(&p0);
        debug_assert!(result.is_ok());
        let _rug_ed_tests_rug_562_rrrruuuugggg_test_from_str = 0;
    }
}
