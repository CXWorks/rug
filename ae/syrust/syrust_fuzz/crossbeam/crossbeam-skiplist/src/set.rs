//! TODO: docs
use std::borrow::Borrow;
use std::fmt;
use std::iter::FromIterator;
use std::ops::{Bound, RangeBounds};
use crate::map;
/// A set based on a lock-free skip list.
pub struct SkipSet<T> {
    inner: map::SkipMap<T, ()>,
}
impl<T> SkipSet<T> {
    /// Returns a new, empty set.
    pub fn new() -> SkipSet<T> {
        SkipSet {
            inner: map::SkipMap::new(),
        }
    }
    /// Returns `true` if the set is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    /// Returns the number of entries in the set.
    ///
    /// If the set is being concurrently modified, consider the returned number just an
    /// approximation without any guarantees.
    pub fn len(&self) -> usize {
        self.inner.len()
    }
}
impl<T> SkipSet<T>
where
    T: Ord,
{
    /// Returns the entry with the smallest key.
    pub fn front(&self) -> Option<Entry<'_, T>> {
        self.inner.front().map(Entry::new)
    }
    /// Returns the entry with the largest key.
    pub fn back(&self) -> Option<Entry<'_, T>> {
        self.inner.back().map(Entry::new)
    }
    /// Returns `true` if the set contains a value for the specified key.
    pub fn contains<Q>(&self, key: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.inner.contains_key(key)
    }
    /// Returns an entry with the specified `key`.
    pub fn get<Q>(&self, key: &Q) -> Option<Entry<'_, T>>
    where
        T: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.inner.get(key).map(Entry::new)
    }
    /// Returns an `Entry` pointing to the lowest element whose key is above
    /// the given bound. If no such element is found then `None` is
    /// returned.
    pub fn lower_bound<'a, Q>(&'a self, bound: Bound<&Q>) -> Option<Entry<'a, T>>
    where
        T: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.inner.lower_bound(bound).map(Entry::new)
    }
    /// Returns an `Entry` pointing to the highest element whose key is below
    /// the given bound. If no such element is found then `None` is
    /// returned.
    pub fn upper_bound<'a, Q>(&'a self, bound: Bound<&Q>) -> Option<Entry<'a, T>>
    where
        T: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.inner.upper_bound(bound).map(Entry::new)
    }
    /// Finds an entry with the specified key, or inserts a new `key`-`value` pair if none exist.
    pub fn get_or_insert(&self, key: T) -> Entry<'_, T> {
        Entry::new(self.inner.get_or_insert(key, ()))
    }
    /// Returns an iterator over all entries in the map.
    pub fn iter(&self) -> Iter<'_, T> {
        Iter { inner: self.inner.iter() }
    }
    /// Returns an iterator over a subset of entries in the skip list.
    pub fn range<Q, R>(&self, range: R) -> Range<'_, Q, R, T>
    where
        T: Borrow<Q>,
        R: RangeBounds<Q>,
        Q: Ord + ?Sized,
    {
        Range {
            inner: self.inner.range(range),
        }
    }
}
impl<T> SkipSet<T>
where
    T: Ord + Send + 'static,
{
    /// Inserts a `key`-`value` pair into the set and returns the new entry.
    ///
    /// If there is an existing entry with this key, it will be removed before inserting the new
    /// one.
    pub fn insert(&self, key: T) -> Entry<'_, T> {
        Entry::new(self.inner.insert(key, ()))
    }
    /// Removes an entry with the specified key from the set and returns it.
    pub fn remove<Q>(&self, key: &Q) -> Option<Entry<'_, T>>
    where
        T: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.inner.remove(key).map(Entry::new)
    }
    /// Removes an entry from the front of the map.
    pub fn pop_front(&self) -> Option<Entry<'_, T>> {
        self.inner.pop_front().map(Entry::new)
    }
    /// Removes an entry from the back of the map.
    pub fn pop_back(&self) -> Option<Entry<'_, T>> {
        self.inner.pop_back().map(Entry::new)
    }
    /// Iterates over the set and removes every entry.
    pub fn clear(&self) {
        self.inner.clear();
    }
}
impl<T> Default for SkipSet<T> {
    fn default() -> SkipSet<T> {
        SkipSet::new()
    }
}
impl<T> fmt::Debug for SkipSet<T>
where
    T: Ord + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("SkipSet { .. }")
    }
}
impl<T> IntoIterator for SkipSet<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;
    fn into_iter(self) -> IntoIter<T> {
        IntoIter {
            inner: self.inner.into_iter(),
        }
    }
}
impl<'a, T> IntoIterator for &'a SkipSet<T>
where
    T: Ord,
{
    type Item = Entry<'a, T>;
    type IntoIter = Iter<'a, T>;
    fn into_iter(self) -> Iter<'a, T> {
        self.iter()
    }
}
impl<T> FromIterator<T> for SkipSet<T>
where
    T: Ord,
{
    fn from_iter<I>(iter: I) -> SkipSet<T>
    where
        I: IntoIterator<Item = T>,
    {
        let s = SkipSet::new();
        for t in iter {
            s.get_or_insert(t);
        }
        s
    }
}
/// TODO
pub struct Entry<'a, T> {
    inner: map::Entry<'a, T, ()>,
}
impl<'a, T> Entry<'a, T> {
    fn new(inner: map::Entry<'a, T, ()>) -> Entry<'a, T> {
        Entry { inner }
    }
    /// Returns a reference to the key.
    pub fn value(&self) -> &T {
        self.inner.key()
    }
    /// Returns `true` if the entry is removed from the set.
    pub fn is_removed(&self) -> bool {
        self.inner.is_removed()
    }
}
impl<'a, T> Entry<'a, T>
where
    T: Ord,
{
    /// TODO
    pub fn move_next(&mut self) -> bool {
        self.inner.move_next()
    }
    /// TODO
    pub fn move_prev(&mut self) -> bool {
        self.inner.move_prev()
    }
    /// Returns the next entry in the set.
    pub fn next(&self) -> Option<Entry<'a, T>> {
        self.inner.next().map(Entry::new)
    }
    /// Returns the previous entry in the set.
    pub fn prev(&self) -> Option<Entry<'a, T>> {
        self.inner.prev().map(Entry::new)
    }
}
impl<T> Entry<'_, T>
where
    T: Ord + Send + 'static,
{
    /// Removes the entry from the set.
    ///
    /// Returns `true` if this call removed the entry and `false` if it was already removed.
    pub fn remove(&self) -> bool {
        self.inner.remove()
    }
}
impl<'a, T> Clone for Entry<'a, T> {
    fn clone(&self) -> Entry<'a, T> {
        Entry { inner: self.inner.clone() }
    }
}
impl<T> fmt::Debug for Entry<'_, T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Entry").field("value", self.value()).finish()
    }
}
/// An owning iterator over the entries of a `SkipSet`.
pub struct IntoIter<T> {
    inner: map::IntoIter<T, ()>,
}
impl<T> Iterator for IntoIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.inner.next().map(|(k, ())| k)
    }
}
impl<T> fmt::Debug for IntoIter<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("IntoIter { .. }")
    }
}
/// An iterator over the entries of a `SkipSet`.
pub struct Iter<'a, T> {
    inner: map::Iter<'a, T, ()>,
}
impl<'a, T> Iterator for Iter<'a, T>
where
    T: Ord,
{
    type Item = Entry<'a, T>;
    fn next(&mut self) -> Option<Entry<'a, T>> {
        self.inner.next().map(Entry::new)
    }
}
impl<'a, T> DoubleEndedIterator for Iter<'a, T>
where
    T: Ord,
{
    fn next_back(&mut self) -> Option<Entry<'a, T>> {
        self.inner.next_back().map(Entry::new)
    }
}
impl<T> fmt::Debug for Iter<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("Iter { .. }")
    }
}
/// An iterator over the entries of a `SkipMap`.
pub struct Range<'a, Q, R, T>
where
    T: Ord + Borrow<Q>,
    R: RangeBounds<Q>,
    Q: Ord + ?Sized,
{
    inner: map::Range<'a, Q, R, T, ()>,
}
impl<'a, Q, R, T> Iterator for Range<'a, Q, R, T>
where
    T: Ord + Borrow<Q>,
    R: RangeBounds<Q>,
    Q: Ord + ?Sized,
{
    type Item = Entry<'a, T>;
    fn next(&mut self) -> Option<Entry<'a, T>> {
        self.inner.next().map(Entry::new)
    }
}
impl<'a, Q, R, T> DoubleEndedIterator for Range<'a, Q, R, T>
where
    T: Ord + Borrow<Q>,
    R: RangeBounds<Q>,
    Q: Ord + ?Sized,
{
    fn next_back(&mut self) -> Option<Entry<'a, T>> {
        self.inner.next_back().map(Entry::new)
    }
}
impl<Q, R, T> fmt::Debug for Range<'_, Q, R, T>
where
    T: Ord + Borrow<Q> + fmt::Debug,
    R: RangeBounds<Q> + fmt::Debug,
    Q: Ord + ?Sized,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Range")
            .field("range", &self.inner.inner.range)
            .field("head", &self.inner.inner.head.as_ref().map(|e| e.key()))
            .field("tail", &self.inner.inner.tail.as_ref().map(|e| e.key()))
            .finish()
    }
}
#[cfg(test)]
mod tests_rug_630 {
    use super::*;
    use crate::set::SkipSet;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_630_rrrruuuugggg_test_rug = 0;
        let set: SkipSet<i32> = SkipSet::new();
        let _rug_ed_tests_rug_630_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_631 {
    use super::*;
    use crate::set::SkipSet;
    #[test]
    fn test_is_empty() {
        let _rug_st_tests_rug_631_rrrruuuugggg_test_is_empty = 0;
        let p0: SkipSet<u32> = SkipSet::new();
        debug_assert_eq!(p0.is_empty(), true);
        let _rug_ed_tests_rug_631_rrrruuuugggg_test_is_empty = 0;
    }
}
#[cfg(test)]
mod tests_rug_632 {
    use super::*;
    use crate::set::SkipSet;
    #[test]
    fn test_len() {
        let _rug_st_tests_rug_632_rrrruuuugggg_test_len = 0;
        let p0: SkipSet<i32> = SkipSet::new();
        p0.len();
        let _rug_ed_tests_rug_632_rrrruuuugggg_test_len = 0;
    }
}
#[cfg(test)]
mod tests_rug_633 {
    use super::*;
    use crate::set::SkipSet;
    #[test]
    fn test_front() {
        let _rug_st_tests_rug_633_rrrruuuugggg_test_front = 0;
        let mut p0: SkipSet<i32> = SkipSet::new();
        p0.front();
        let _rug_ed_tests_rug_633_rrrruuuugggg_test_front = 0;
    }
}
#[cfg(test)]
mod tests_rug_634 {
    use super::*;
    use crate::set::SkipSet;
    #[test]
    fn test_back() {
        let _rug_st_tests_rug_634_rrrruuuugggg_test_back = 0;
        let p0: SkipSet<i32> = SkipSet::new();
        p0.back();
        let _rug_ed_tests_rug_634_rrrruuuugggg_test_back = 0;
    }
}
#[cfg(test)]
mod tests_rug_635 {
    use super::*;
    use crate::set::SkipSet;
    #[test]
    fn test_contains() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u8, u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: SkipSet<u8> = SkipSet::new();
        p0.insert(rug_fuzz_0);
        let p1 = rug_fuzz_1;
        debug_assert_eq!(p0.contains(& p1), true);
             }
});    }
}
#[cfg(test)]
mod tests_rug_636 {
    use super::*;
    use crate::set::SkipSet;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: SkipSet<usize> = SkipSet::new();
        let mut p1: usize = rug_fuzz_0;
        p0.get(&p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_638 {
    use super::*;
    use crate::set::SkipSet;
    use std::collections::Bound;
    #[test]
    fn test_upper_bound() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(usize, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: SkipSet<usize> = SkipSet::new();
        p0.insert(rug_fuzz_0);
        p0.insert(rug_fuzz_1);
        let mut p1: Bound<&usize> = Bound::Included(&rug_fuzz_2);
        p0.upper_bound(p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_639 {
    use super::*;
    use crate::set::SkipSet;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = SkipSet::<bool>::new();
        let mut p1 = rug_fuzz_0;
        p0.get_or_insert(p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_640 {
    use super::*;
    use crate::set::SkipSet;
    #[test]
    fn test_iter() {
        let _rug_st_tests_rug_640_rrrruuuugggg_test_iter = 0;
        let mut p0: SkipSet<i32> = SkipSet::new();
        SkipSet::<i32>::iter(&p0);
        let _rug_ed_tests_rug_640_rrrruuuugggg_test_iter = 0;
    }
}
#[cfg(test)]
mod tests_rug_641 {
    use super::*;
    use crate::set::SkipSet;
    use std::ops::RangeFrom;
    #[test]
    fn test_range() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: SkipSet<i32> = SkipSet::new();
        let p1 = RangeFrom { start: &rug_fuzz_0 };
        p0.range(p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_642 {
    use super::*;
    use crate::set::SkipSet;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: SkipSet<i32> = SkipSet::new();
        let mut p1: i32 = rug_fuzz_0;
        p0.insert(p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_643 {
    use super::*;
    use crate::set::SkipSet;
    use crate::set::Entry;
    use std::borrow::Borrow;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: SkipSet<u32> = SkipSet::new();
        let key = rug_fuzz_0;
        let mut p1 = &key;
        p0.remove(p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_644 {
    use super::*;
    use crate::set;
    #[test]
    fn test_pop_front() {
        let _rug_st_tests_rug_644_rrrruuuugggg_test_pop_front = 0;
        let mut p0: set::SkipSet<i32> = set::SkipSet::new();
        p0.pop_front();
        let _rug_ed_tests_rug_644_rrrruuuugggg_test_pop_front = 0;
    }
}
#[cfg(test)]
mod tests_rug_645 {
    use super::*;
    use crate::set;
    #[test]
    fn test_pop_back() {
        let _rug_st_tests_rug_645_rrrruuuugggg_test_pop_back = 0;
        let mut p0: set::SkipSet<i32> = set::SkipSet::new();
        set::SkipSet::<i32>::pop_back(&p0);
        let _rug_ed_tests_rug_645_rrrruuuugggg_test_pop_back = 0;
    }
}
#[cfg(test)]
mod tests_rug_646 {
    use super::*;
    use crate::set::SkipSet;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(i32, i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: SkipSet<i32> = SkipSet::new();
        p0.insert(rug_fuzz_0);
        p0.insert(rug_fuzz_1);
        p0.insert(rug_fuzz_2);
        p0.clear();
        debug_assert!(p0.is_empty());
             }
});    }
}
#[cfg(test)]
mod tests_rug_647 {
    use super::*;
    use crate::set::SkipSet;
    use std::default::Default;
    #[test]
    fn test_default() {
        let _rug_st_tests_rug_647_rrrruuuugggg_test_default = 0;
        let set_default: SkipSet<i32> = Default::default();
        let _rug_ed_tests_rug_647_rrrruuuugggg_test_default = 0;
    }
}
#[cfg(test)]
mod tests_rug_651 {
    use super::*;
    use crate::map;
    #[test]
    fn test_entry_new() {
        let _rug_st_tests_rug_651_rrrruuuugggg_test_entry_new = 0;
        let inner: map::Entry<'_, i32, ()> = unimplemented!();
        crate::set::Entry::<i32>::new(inner);
        let _rug_ed_tests_rug_651_rrrruuuugggg_test_entry_new = 0;
    }
}
#[cfg(test)]
mod tests_rug_652 {
    use super::*;
    use crate::set;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_652_rrrruuuugggg_test_rug = 0;
        let mut p0: set::Entry<'_, i32> = unimplemented!();
        set::Entry::<'_, i32>::value(&p0);
        let _rug_ed_tests_rug_652_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_653 {
    use super::*;
    use crate::set;
    #[test]
    fn test_is_removed() {
        let _rug_st_tests_rug_653_rrrruuuugggg_test_is_removed = 0;
        let mut p0: set::Entry<'_, i32> = unimplemented!();
        debug_assert_eq!(p0.is_removed(), false);
        let _rug_ed_tests_rug_653_rrrruuuugggg_test_is_removed = 0;
    }
}
#[cfg(test)]
mod tests_rug_654 {
    use super::*;
    use crate::set::Entry;
    #[test]
    fn test_move_next() {
        let _rug_st_tests_rug_654_rrrruuuugggg_test_move_next = 0;
        let mut p0: Entry<'_, i32> = unimplemented!();
        Entry::<'_, i32>::move_next(&mut p0);
        let _rug_ed_tests_rug_654_rrrruuuugggg_test_move_next = 0;
    }
}
#[cfg(test)]
mod tests_rug_655 {
    use super::*;
    use crate::set;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_655_rrrruuuugggg_test_rug = 0;
        let mut p0: set::Entry<'_, i32> = unimplemented!();
        set::Entry::move_prev(&mut p0);
        let _rug_ed_tests_rug_655_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_656 {
    use super::*;
    use crate::set::Entry;
    #[test]
    fn test_next() {
        let _rug_st_tests_rug_656_rrrruuuugggg_test_next = 0;
        let mut p0: Entry<'_, i32> = unimplemented!();
        p0.next();
        let _rug_ed_tests_rug_656_rrrruuuugggg_test_next = 0;
    }
}
#[cfg(test)]
mod tests_rug_657 {
    use super::*;
    use crate::set;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_657_rrrruuuugggg_test_rug = 0;
        let mut p0: set::Entry<'_, i32> = unimplemented!();
        p0.prev();
        let _rug_ed_tests_rug_657_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_658 {
    use super::*;
    use crate::set;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_658_rrrruuuugggg_test_rug = 0;
        let mut p0: set::Entry<'_, i32> = unimplemented!();
        set::Entry::<'_, i32>::remove(&p0);
        let _rug_ed_tests_rug_658_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_660 {
    use super::*;
    use crate::set::IntoIter;
    use crate::set;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut skiplist = set::SkipSet::new();
        skiplist.insert(rug_fuzz_0);
        skiplist.insert(rug_fuzz_1);
        let mut iter = skiplist.into_iter();
        debug_assert_eq!(iter.next(), Some(1));
        debug_assert_eq!(iter.next(), Some(2));
        debug_assert_eq!(iter.next(), None);
             }
});    }
}
#[cfg(test)]
mod tests_rug_662 {
    use super::*;
    use std::iter::DoubleEndedIterator;
    use crate::set::{Iter, Entry};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_662_rrrruuuugggg_test_rug = 0;
        let mut p0: Iter<'_, i32> = unimplemented!();
        p0.next_back();
        let _rug_ed_tests_rug_662_rrrruuuugggg_test_rug = 0;
    }
}
