use std::iter::FromIterator;

use crate::{Array, Item, Table};

/// Type representing a TOML array of tables
#[derive(Clone, Debug, Default)]
pub struct ArrayOfTables {
    // Always Vec<Item::Table>, just `Item` to make `Index` work
    pub(crate) span: Option<std::ops::Range<usize>>,
    pub(crate) values: Vec<Item>,
}

/// Constructors
///
/// See also `FromIterator`
impl ArrayOfTables {
    /// Creates an empty array of tables.
    pub fn new() -> Self {
        Default::default()
    }
}

/// Formatting
impl ArrayOfTables {
    /// Convert to an inline array
    pub fn into_array(mut self) -> Array {
        for value in self.values.iter_mut() {
            value.make_value();
        }
        let mut a = Array::with_vec(self.values);
        a.fmt();
        a
    }

    /// Returns the location within the original document
    pub(crate) fn span(&self) -> Option<std::ops::Range<usize>> {
        self.span.clone()
    }

    pub(crate) fn despan(&mut self, input: &str) {
        self.span = None;
        for value in &mut self.values {
            value.despan(input);
        }
    }
}

impl ArrayOfTables {
    /// Returns an iterator over tables.
    pub fn iter(&self) -> ArrayOfTablesIter<'_> {
        Box::new(self.values.iter().filter_map(Item::as_table))
    }

    /// Returns an iterator over tables.
    pub fn iter_mut(&mut self) -> ArrayOfTablesIterMut<'_> {
        Box::new(self.values.iter_mut().filter_map(Item::as_table_mut))
    }

    /// Returns the length of the underlying Vec.
    /// To get the actual number of items use `a.iter().count()`.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Returns true iff `self.len() == 0`.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Removes all the tables.
    pub fn clear(&mut self) {
        self.values.clear()
    }

    /// Returns an optional reference to the table.
    pub fn get(&self, index: usize) -> Option<&Table> {
        self.values.get(index).and_then(Item::as_table)
    }

    /// Returns an optional mutable reference to the table.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Table> {
        self.values.get_mut(index).and_then(Item::as_table_mut)
    }

    /// Appends a table to the array.
    pub fn push(&mut self, table: Table) {
        self.values.push(Item::Table(table));
    }

    /// Removes a table with the given index.
    pub fn remove(&mut self, index: usize) {
        self.values.remove(index);
    }
}

/// An iterator type over `ArrayOfTables`'s values.
pub type ArrayOfTablesIter<'a> = Box<dyn Iterator<Item = &'a Table> + 'a>;
/// An iterator type over `ArrayOfTables`'s values.
pub type ArrayOfTablesIterMut<'a> = Box<dyn Iterator<Item = &'a mut Table> + 'a>;
/// An iterator type over `ArrayOfTables`'s values.
pub type ArrayOfTablesIntoIter = Box<dyn Iterator<Item = Table>>;

impl Extend<Table> for ArrayOfTables {
    fn extend<T: IntoIterator<Item = Table>>(&mut self, iter: T) {
        for value in iter {
            self.push(value);
        }
    }
}

impl FromIterator<Table> for ArrayOfTables {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = Table>,
    {
        let v = iter.into_iter().map(Item::Table);
        ArrayOfTables {
            values: v.collect(),
            span: None,
        }
    }
}

impl IntoIterator for ArrayOfTables {
    type Item = Table;
    type IntoIter = ArrayOfTablesIntoIter;

    fn into_iter(self) -> Self::IntoIter {
        Box::new(
            self.values
                .into_iter()
                .filter(|v| v.is_table())
                .map(|v| v.into_table().unwrap()),
        )
    }
}

impl<'s> IntoIterator for &'s ArrayOfTables {
    type Item = &'s Table;
    type IntoIter = ArrayOfTablesIter<'s>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl std::fmt::Display for ArrayOfTables {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // HACK: Without the header, we don't really have a proper way of printing this
        self.clone().into_array().fmt(f)
    }
}
#[cfg(test)]
mod tests_rug_886 {
    use super::*;
    use crate::array_of_tables::ArrayOfTables;
    
    #[test]
    fn test_rug() {
        ArrayOfTables::new();
    }
}#[cfg(test)]
mod tests_rug_887 {
    use super::*;
    use crate::{ArrayOfTables, Array};

    #[test]
    fn test_into_array() {
        let mut p0 = ArrayOfTables::new();

        let result = p0.into_array();
        // assert statements...
    }
}#[cfg(test)]
mod tests_rug_889 {
    use super::*;
    use crate::array_of_tables::ArrayOfTables;
    
    #[test]
    fn test_rug() {
        let mut p0: ArrayOfTables = ArrayOfTables::new();
        let p1 = "sample input";

        p0.despan(p1);
    }
}        
    #[cfg(test)]
    mod tests_rug_890 {
        use super::*;
        use crate::array_of_tables::{ArrayOfTables, ArrayOfTablesIter};
        use crate::Item;

        #[test]
        fn test_rug() {
            let mut p0 = ArrayOfTables::new();

            ArrayOfTables::iter(&p0);

        }
    }
    #[cfg(test)]
mod tests_rug_891 {
    use super::*;
    use crate::array_of_tables::{ArrayOfTables, ArrayOfTablesIterMut, Item};  // Add necessary use statements


    #[test]
    fn test_rug() {
        let mut p0: ArrayOfTables = ArrayOfTables::new();
        // Construct the test data


        ArrayOfTables::iter_mut(&mut p0);

    }
}#[cfg(test)]
mod tests_rug_892 {
    use super::*;
    use crate::array_of_tables::ArrayOfTables;

    #[test]
    fn test_rug() {
        let mut p0 = ArrayOfTables::new();

        ArrayOfTables::len(&p0);
    }
}
#[cfg(test)]
mod tests_rug_893 {
    use super::*;
    use crate::{array_of_tables, de};

    #[test]
    fn test_rug() {
        let mut p0: array_of_tables::ArrayOfTables = array_of_tables::ArrayOfTables::new();
        
        assert_eq!(<array_of_tables::ArrayOfTables>::is_empty(&p0), true);
    }
}
#[cfg(test)]
mod tests_rug_894 {
    use super::*;
    use crate::array_of_tables::ArrayOfTables;
    
    #[test]
    fn test_rug() {
        let mut p0: ArrayOfTables = ArrayOfTables::new();

        ArrayOfTables::clear(&mut p0);
    }
}#[cfg(test)]
mod tests_rug_895 {
    use super::*;
    use crate::{ArrayOfTables, Table};
    
    #[test]
    fn test_get() {
        let mut p0 = ArrayOfTables::new();
        let mut p1 = 0;
        
        p0.get(p1);
    }
}
#[cfg(test)]
mod tests_rug_896 {
    use super::*;
    use crate::array_of_tables::ArrayOfTables;
    use crate::Table;

    #[test]
    fn test_rug() {
        let mut p0 = ArrayOfTables::new();
        p0.push(Table::new());

        let p1: usize = 0;

        p0.get_mut(p1);
    }
}
#[cfg(test)]
mod tests_rug_897 {
    use super::*;
    use crate::array_of_tables::ArrayOfTables;
    use crate::table::Table;
    use serde::de::Error;

    #[test]
    fn test_push() {
        let mut p0: ArrayOfTables = ArrayOfTables::new(); // create a new ArrayOfTables
        let mut p1: Table = Table::new(); // create a new Table
        
        ArrayOfTables::push(&mut p0, p1);
    }
}
#[cfg(test)]
mod tests_rug_898 {
    use super::*;
    use crate::array_of_tables::ArrayOfTables;

    #[test]
    fn test_rug() {
        let mut p0 = ArrayOfTables::new(); // Construct ArrayOfTables instance

        let mut p1: usize = 0; // Initialize usize

        ArrayOfTables::remove(&mut p0, p1);
    }
}
