use std::ops;
use crate::document::Document;
use crate::key::Key;
use crate::table::TableKeyValue;
use crate::{value, InlineTable, InternalString, Item, Table, Value};
pub trait Index: crate::private::Sealed {
    #[doc(hidden)]
    fn index<'v>(&self, val: &'v Item) -> Option<&'v Item>;
    #[doc(hidden)]
    fn index_mut<'v>(&self, val: &'v mut Item) -> Option<&'v mut Item>;
}
impl Index for usize {
    fn index<'v>(&self, v: &'v Item) -> Option<&'v Item> {
        match *v {
            Item::ArrayOfTables(ref aot) => aot.values.get(*self),
            Item::Value(ref a) if a.is_array() => {
                a.as_array().and_then(|a| a.values.get(*self))
            }
            _ => None,
        }
    }
    fn index_mut<'v>(&self, v: &'v mut Item) -> Option<&'v mut Item> {
        match *v {
            Item::ArrayOfTables(ref mut vec) => vec.values.get_mut(*self),
            Item::Value(ref mut a) => {
                a.as_array_mut().and_then(|a| a.values.get_mut(*self))
            }
            _ => None,
        }
    }
}
impl Index for str {
    fn index<'v>(&self, v: &'v Item) -> Option<&'v Item> {
        match *v {
            Item::Table(ref t) => t.get(self),
            Item::Value(ref v) => {
                v
                    .as_inline_table()
                    .and_then(|t| t.items.get(self))
                    .and_then(|kv| {
                        if !kv.value.is_none() { Some(&kv.value) } else { None }
                    })
            }
            _ => None,
        }
    }
    fn index_mut<'v>(&self, v: &'v mut Item) -> Option<&'v mut Item> {
        if let Item::None = *v {
            let mut t = InlineTable::default();
            t.items
                .insert(
                    InternalString::from(self),
                    TableKeyValue::new(Key::new(self), Item::None),
                );
            *v = value(Value::InlineTable(t));
        }
        match *v {
            Item::Table(ref mut t) => Some(t.entry(self).or_insert(Item::None)),
            Item::Value(ref mut v) => {
                v
                    .as_inline_table_mut()
                    .map(|t| {
                        &mut t
                            .items
                            .entry(InternalString::from(self))
                            .or_insert_with(|| TableKeyValue::new(
                                Key::new(self),
                                Item::None,
                            ))
                            .value
                    })
            }
            _ => None,
        }
    }
}
impl Index for String {
    fn index<'v>(&self, v: &'v Item) -> Option<&'v Item> {
        self[..].index(v)
    }
    fn index_mut<'v>(&self, v: &'v mut Item) -> Option<&'v mut Item> {
        self[..].index_mut(v)
    }
}
impl<'a, T: ?Sized> Index for &'a T
where
    T: Index,
{
    fn index<'v>(&self, v: &'v Item) -> Option<&'v Item> {
        (**self).index(v)
    }
    fn index_mut<'v>(&self, v: &'v mut Item) -> Option<&'v mut Item> {
        (**self).index_mut(v)
    }
}
impl<I> ops::Index<I> for Item
where
    I: Index,
{
    type Output = Item;
    fn index(&self, index: I) -> &Item {
        index.index(self).expect("index not found")
    }
}
impl<I> ops::IndexMut<I> for Item
where
    I: Index,
{
    fn index_mut(&mut self, index: I) -> &mut Item {
        index.index_mut(self).expect("index not found")
    }
}
impl<'s> ops::Index<&'s str> for Table {
    type Output = Item;
    fn index(&self, key: &'s str) -> &Item {
        self.get(key).expect("index not found")
    }
}
impl<'s> ops::IndexMut<&'s str> for Table {
    fn index_mut(&mut self, key: &'s str) -> &mut Item {
        self.entry(key).or_insert(Item::None)
    }
}
impl<'s> ops::Index<&'s str> for InlineTable {
    type Output = Value;
    fn index(&self, key: &'s str) -> &Value {
        self.get(key).expect("index not found")
    }
}
impl<'s> ops::IndexMut<&'s str> for InlineTable {
    fn index_mut(&mut self, key: &'s str) -> &mut Value {
        self.get_mut(key).expect("index not found")
    }
}
impl<'s> ops::Index<&'s str> for Document {
    type Output = Item;
    fn index(&self, key: &'s str) -> &Item {
        self.root.index(key)
    }
}
impl<'s> ops::IndexMut<&'s str> for Document {
    fn index_mut(&mut self, key: &'s str) -> &mut Item {
        self.root.index_mut(key)
    }
}
#[cfg(test)]
mod tests_rug_846 {
    use super::*;
    use crate::index::Index;
    use crate::item::Item;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_846_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let mut p0: usize = rug_fuzz_0;
        let mut p1: Item = Item::None;
        p0.index_mut(&mut p1);
        let _rug_ed_tests_rug_846_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_847 {
    use super::*;
    use crate::index::Index;
    use crate::item::Item;
    #[test]
    fn test_index() {
        let _rug_st_tests_rug_847_rrrruuuugggg_test_index = 0;
        let rug_fuzz_0 = "table.key";
        let p0: &str = rug_fuzz_0;
        let p1: Item = Item::Table(Table::new());
        p0.index(&p1);
        let _rug_ed_tests_rug_847_rrrruuuugggg_test_index = 0;
    }
}
#[cfg(test)]
mod tests_rug_850 {
    use super::*;
    use crate::index::Index;
    use crate::item::Item;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_850_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample string";
        let mut p0 = String::from(rug_fuzz_0);
        let mut p1 = Item::default();
        p0.index_mut(&mut p1);
        let _rug_ed_tests_rug_850_rrrruuuugggg_test_rug = 0;
    }
}
