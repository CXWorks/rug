#![allow(missing_docs)]

//! Document tree traversal to walk a shared borrow of a document tree.
//!
//! Each method of the [`Visit`] trait is a hook that can be overridden
//! to customize the behavior when mutating the corresponding type of node.
//! By default, every method recursively visits the substructure of the
//! input by invoking the right visitor method of each of its fields.
//!
//! ```
//! # use toml_edit::{Item, ArrayOfTables, Table, Value};
//!
//! pub trait Visit<'doc> {
//!     /* ... */
//!
//!     fn visit_item(&mut self, i: &'doc Item) {
//!         visit_item(self, i);
//!     }
//!
//!     /* ... */
//!     # fn visit_value(&mut self, i: &'doc Value);
//!     # fn visit_table(&mut self, i: &'doc Table);
//!     # fn visit_array_of_tables(&mut self, i: &'doc ArrayOfTables);
//! }
//!
//! pub fn visit_item<'doc, V>(v: &mut V, node: &'doc Item)
//! where
//!     V: Visit<'doc> + ?Sized,
//! {
//!     match node {
//!         Item::None => {}
//!         Item::Value(value) => v.visit_value(value),
//!         Item::Table(table) => v.visit_table(table),
//!         Item::ArrayOfTables(array) => v.visit_array_of_tables(array),
//!     }
//! }
//! ```
//!
//! The API is modeled after [`syn::visit`](https://docs.rs/syn/1/syn/visit).
//!
//! # Examples
//!
//! This visitor stores every string in the document.
//!
//! ```
//! # use toml_edit::*;
//! use toml_edit::visit::*;
//!
//! #[derive(Default)]
//! struct StringCollector<'doc> {
//!     strings: Vec<&'doc str>,
//! }
//!
//! impl<'doc> Visit<'doc> for StringCollector<'doc> {
//!     fn visit_string(&mut self, node: &'doc Formatted<String>) {
//!          self.strings.push(node.value().as_str());
//!     }
//! }
//!
//! let input = r#"
//! laputa = "sky-castle"
//! the-force = { value = "surrounds-you" }
//! "#;
//!
//! let mut document: Document = input.parse().unwrap();
//! let mut visitor = StringCollector::default();
//! visitor.visit_document(&document);
//!
//! assert_eq!(visitor.strings, vec!["sky-castle", "surrounds-you"]);
//! ```
//!
//! For a more complex example where the visitor has internal state, see `examples/visit.rs`
//! [on GitHub](https://github.com/ordian/toml_edit/blob/master/examples/visit.rs).

use crate::{
    Array, ArrayOfTables, Datetime, Document, Formatted, InlineTable, Item, Table, TableLike, Value,
};

/// Document tree traversal to mutate an exclusive borrow of a document tree in-place.
///
/// See the [module documentation](self) for details.
pub trait Visit<'doc> {
    fn visit_document(&mut self, node: &'doc Document) {
        visit_document(self, node);
    }

    fn visit_item(&mut self, node: &'doc Item) {
        visit_item(self, node);
    }

    fn visit_table(&mut self, node: &'doc Table) {
        visit_table(self, node);
    }

    fn visit_inline_table(&mut self, node: &'doc InlineTable) {
        visit_inline_table(self, node)
    }

    fn visit_table_like(&mut self, node: &'doc dyn TableLike) {
        visit_table_like(self, node);
    }

    fn visit_table_like_kv(&mut self, key: &'doc str, node: &'doc Item) {
        visit_table_like_kv(self, key, node);
    }

    fn visit_array(&mut self, node: &'doc Array) {
        visit_array(self, node);
    }

    fn visit_array_of_tables(&mut self, node: &'doc ArrayOfTables) {
        visit_array_of_tables(self, node);
    }

    fn visit_value(&mut self, node: &'doc Value) {
        visit_value(self, node);
    }

    fn visit_boolean(&mut self, node: &'doc Formatted<bool>) {
        visit_boolean(self, node)
    }

    fn visit_datetime(&mut self, node: &'doc Formatted<Datetime>) {
        visit_datetime(self, node);
    }

    fn visit_float(&mut self, node: &'doc Formatted<f64>) {
        visit_float(self, node)
    }

    fn visit_integer(&mut self, node: &'doc Formatted<i64>) {
        visit_integer(self, node)
    }

    fn visit_string(&mut self, node: &'doc Formatted<String>) {
        visit_string(self, node)
    }
}

pub fn visit_document<'doc, V>(v: &mut V, node: &'doc Document)
where
    V: Visit<'doc> + ?Sized,
{
    v.visit_table(node.as_table());
}

pub fn visit_item<'doc, V>(v: &mut V, node: &'doc Item)
where
    V: Visit<'doc> + ?Sized,
{
    match node {
        Item::None => {}
        Item::Value(value) => v.visit_value(value),
        Item::Table(table) => v.visit_table(table),
        Item::ArrayOfTables(array) => v.visit_array_of_tables(array),
    }
}

pub fn visit_table<'doc, V>(v: &mut V, node: &'doc Table)
where
    V: Visit<'doc> + ?Sized,
{
    v.visit_table_like(node)
}

pub fn visit_inline_table<'doc, V>(v: &mut V, node: &'doc InlineTable)
where
    V: Visit<'doc> + ?Sized,
{
    v.visit_table_like(node)
}

pub fn visit_table_like<'doc, V>(v: &mut V, node: &'doc dyn TableLike)
where
    V: Visit<'doc> + ?Sized,
{
    for (key, item) in node.iter() {
        v.visit_table_like_kv(key, item)
    }
}

pub fn visit_table_like_kv<'doc, V>(v: &mut V, _key: &'doc str, node: &'doc Item)
where
    V: Visit<'doc> + ?Sized,
{
    v.visit_item(node)
}

pub fn visit_array<'doc, V>(v: &mut V, node: &'doc Array)
where
    V: Visit<'doc> + ?Sized,
{
    for value in node.iter() {
        v.visit_value(value);
    }
}

pub fn visit_array_of_tables<'doc, V>(v: &mut V, node: &'doc ArrayOfTables)
where
    V: Visit<'doc> + ?Sized,
{
    for table in node.iter() {
        v.visit_table(table);
    }
}

pub fn visit_value<'doc, V>(v: &mut V, node: &'doc Value)
where
    V: Visit<'doc> + ?Sized,
{
    match node {
        Value::String(s) => v.visit_string(s),
        Value::Integer(i) => v.visit_integer(i),
        Value::Float(f) => v.visit_float(f),
        Value::Boolean(b) => v.visit_boolean(b),
        Value::Datetime(dt) => v.visit_datetime(dt),
        Value::Array(array) => v.visit_array(array),
        Value::InlineTable(table) => v.visit_inline_table(table),
    }
}

macro_rules! empty_visit {
    ($name: ident, $t: ty) => {
        fn $name<'doc, V>(_v: &mut V, _node: &'doc $t)
        where
            V: Visit<'doc> + ?Sized,
        {
        }
    };
}

empty_visit!(visit_boolean, Formatted<bool>);
empty_visit!(visit_datetime, Formatted<Datetime>);
empty_visit!(visit_float, Formatted<f64>);
empty_visit!(visit_integer, Formatted<i64>);
empty_visit!(visit_string, Formatted<String>);
#[cfg(test)]
mod tests_rug_789 {
    use super::*;
    use crate::Document;
    use crate::visit::{Visit, visit_document};
    
    #[test]
    fn test_visit_document() {
        let mut visitor = MyVisitor;
        let document = Document::new();
        
        visit_document(&mut visitor, &document);
    }
    
    struct MyVisitor;
    
    impl<'doc> Visit<'doc> for MyVisitor {}
}#[cfg(test)]
mod tests_rug_790 {
    use super::*;
    use crate::visit::{self, Visit};
    use crate::item::Item;

    #[test]
    fn test_visit_item() {
        struct MyVisitor;

        impl<'doc> Visit<'doc> for MyVisitor {
            fn visit_value(&mut self, value: &'doc Value) {
                // Implementation of visit_value for MyVisitor
            }

            fn visit_table(&mut self, table: &'doc Table) {
                // Implementation of visit_table for MyVisitor
            }

            fn visit_array_of_tables(&mut self, array: &'doc ArrayOfTables) {
                // Implementation of visit_array_of_tables for MyVisitor
            }
        }

        let mut visitor = MyVisitor {};

        let item = Item::None;

        crate::visit::visit_item(&mut visitor, &item);
    }
}#[cfg(test)]
mod tests_rug_791 {
    use super::*;
    use crate::Table;
    use crate::visit::{Visit, visit_table};
    
    struct MyVisitor;
    
    impl<'doc> Visit<'doc> for MyVisitor {
        fn visit_table(&mut self, node: &'doc Table) {
            // Implement your test logic here
        }
    }
    
    #[test]
    fn test_visit_table() {
        let mut visitor = MyVisitor;
        let table = Table::new();
        
        visit_table(&mut visitor, &table);
        
        // Assert your expectations here
    }
}#[cfg(test)]
mod tests_rug_792 {
    use super::*;
    use crate::visit::{Visit, visit_table_like, visit_inline_table};
    use crate::inline_table::InlineTable;

    #[test]
    fn test_rug() {
        let mut p0 = ConcreteImplementation {};

        let p1 = InlineTable::new();

        visit_inline_table(&mut p0, &p1);
    }

    // Concrete implementation of `Visit` trait
    struct ConcreteImplementation;

    impl<'doc> Visit<'doc> for ConcreteImplementation {}
}
#[cfg(test)]
mod tests_rug_793 {
    use super::*;
    use crate::visit::{Visit, visit_table_like_kv};
    use crate::table::TableLike;
    
    struct MyVisitor;
    
    impl<'doc> Visit<'doc> for MyVisitor {
        fn visit_table_like_kv(&mut self, key: &'doc str, node: &'doc Item) {
            // implement your custom logic here
        }
    }
    
    #[test]
    fn test_rug() {
        let mut p0 = MyVisitor {};
        let p1: &(dyn TableLike + 'static) = unimplemented!("fill in the value for p1");
        
        visit_table_like(&mut p0, p1);
    
        // add assertions if needed
    }
}#[cfg(test)]
mod tests_rug_794 {
    use super::*;
    use crate::{Document, Item};

    struct ConcreteVisit;

    impl<'doc> Visit<'doc> for ConcreteVisit {}

    #[test]
    fn test_visit_table_like_kv() {
        let mut visit = ConcreteVisit;
        let key = "example_key";
        let node = Item::Table(crate::Table::new());
        
        visit.visit_table_like_kv(key, &node);
    }
}#[cfg(test)]
mod tests_rug_796 {
    use super::*;
    use crate::array_of_tables::ArrayOfTables;

    struct MyVisitor;

    impl<'doc> Visit<'doc> for MyVisitor {}

    #[test]
    fn test_visit_array_of_tables() {
        let mut visitor = MyVisitor;
        let array_of_tables: ArrayOfTables = ArrayOfTables::default();

        visit_array_of_tables(&mut visitor, &array_of_tables);
    }
}
#[cfg(test)]
mod tests_rug_801 {
    use super::*;
    use crate::{Document, Item, Table, InlineTable, TableLike, Array, ArrayOfTables, Value};
    use crate::visit::{Visit, visit_integer};
    use crate::repr::Formatted;

    #[test]
    fn test_visit_integer() {
        let mut p0: MyVisit = MyVisit{};
        let p1: Formatted<i64> = Formatted::new(42);

        visit_integer(&mut p0, &p1);
    }

    struct MyVisit;

    impl<'doc> Visit<'doc> for MyVisit {
        fn visit_integer(&mut self, node: &'doc Formatted<i64>) {
            // implement your test logic here
        }
    }
}

#[cfg(test)]
mod tests_rug_803 {
    use super::*;
    use crate::{Item, Document, InlineTable, Table, Array, ArrayOfTables, Value, Formatted};

    struct MyVisitor;

    impl<'doc> Visit<'doc> for MyVisitor {}

    #[test]
    fn test_rug() {
        let mut p0 = MyVisitor;
        let mut p1 = Document::new();

        p0.visit_document(&p1);
    }
}
#[cfg(test)]
mod tests_rug_806 {
    use super::*;
    use crate::visit::*;

    #[test]
    fn test_rug() {
        let mut p0: TestVisitor = TestVisitor {};  // replace TestVisitor with your concrete implementation of Visit
        let p1: InlineTable = InlineTable::default(); // replace InlineTable with your concrete implementation of T

        p0.visit_inline_table(&p1);
    }

    #[derive(Default)]
    struct TestVisitor {}

    impl<'doc> Visit<'doc> for TestVisitor {}
}#[cfg(test)]
mod tests_rug_810 {
    use super::*;
    use crate::{visit, Document, Item, Table, InlineTable, TableLike, Array, ArrayOfTables, Value, Formatted, Datetime};

    struct MyVisitor;

    impl<'doc> visit::Visit<'doc> for MyVisitor {
        fn visit_array_of_tables(&mut self, node: &'doc ArrayOfTables) {
            visit_array_of_tables(self, node);
        }
    }

    #[test]
    fn test_visit_array_of_tables() {
        let doc = Document::new();
        let array_of_tables = doc["array_of_tables"].as_array_of_tables().unwrap();
        
        let mut visitor = MyVisitor;
        
        visitor.visit_array_of_tables(array_of_tables);
    }
}#[cfg(test)]
mod tests_rug_814 {
    use super::*;
    use crate::visit::*;

    #[test]
    fn test_visit_float() {
        let mut p0 = MyVisitor {}; // replace MyVisitor with a concrete implementation that satisfies the bounds: visit::Visit
        let p1 = Formatted::new(3.14); // replace 3.14 with the actual value

        p0.visit_float(&p1);
    }

    struct MyVisitor;

    impl<'doc> Visit<'doc> for MyVisitor {
        fn visit_float(&mut self, node: &'doc Formatted<f64>) {
            visit_float(self, node)
        }
    }
}