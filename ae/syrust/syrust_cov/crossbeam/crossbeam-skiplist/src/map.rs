//! TODO: docs

use std::borrow::Borrow;
use std::fmt;
use std::iter::FromIterator;
use std::mem::ManuallyDrop;
use std::ops::{Bound, RangeBounds};
use std::ptr;

use crate::base::{self, try_pin_loop};
use crate::epoch;

/// A map based on a lock-free skip list.
pub struct SkipMap<K, V> {
    inner: base::SkipList<K, V>,
}

impl<K, V> SkipMap<K, V> {
    /// Returns a new, empty map.
    pub fn new() -> SkipMap<K, V> {
        SkipMap {
            inner: base::SkipList::new(epoch::default_collector().clone()),
        }
    }

    /// Returns `true` if the map is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns the number of entries in the map.
    ///
    /// If the map is being concurrently modified, consider the returned number just an
    /// approximation without any guarantees.
    pub fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<K, V> SkipMap<K, V>
where
    K: Ord,
{
    /// Returns the entry with the smallest key.
    pub fn front(&self) -> Option<Entry<'_, K, V>> {
        let guard = &epoch::pin();
        try_pin_loop(|| self.inner.front(guard)).map(Entry::new)
    }

    /// Returns the entry with the largest key.
    pub fn back(&self) -> Option<Entry<'_, K, V>> {
        let guard = &epoch::pin();
        try_pin_loop(|| self.inner.back(guard)).map(Entry::new)
    }

    /// Returns `true` if the map contains a value for the specified key.
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        let guard = &epoch::pin();
        self.inner.contains_key(key, guard)
    }

    /// Returns an entry with the specified `key`.
    pub fn get<Q>(&self, key: &Q) -> Option<Entry<'_, K, V>>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        let guard = &epoch::pin();
        try_pin_loop(|| self.inner.get(key, guard)).map(Entry::new)
    }

    /// Returns an `Entry` pointing to the lowest element whose key is above
    /// the given bound. If no such element is found then `None` is
    /// returned.
    pub fn lower_bound<'a, Q>(&'a self, bound: Bound<&Q>) -> Option<Entry<'a, K, V>>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        let guard = &epoch::pin();
        try_pin_loop(|| self.inner.lower_bound(bound, guard)).map(Entry::new)
    }

    /// Returns an `Entry` pointing to the highest element whose key is below
    /// the given bound. If no such element is found then `None` is
    /// returned.
    pub fn upper_bound<'a, Q>(&'a self, bound: Bound<&Q>) -> Option<Entry<'a, K, V>>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        let guard = &epoch::pin();
        try_pin_loop(|| self.inner.upper_bound(bound, guard)).map(Entry::new)
    }

    /// Finds an entry with the specified key, or inserts a new `key`-`value` pair if none exist.
    pub fn get_or_insert(&self, key: K, value: V) -> Entry<'_, K, V> {
        let guard = &epoch::pin();
        Entry::new(self.inner.get_or_insert(key, value, guard))
    }

    /// Returns an iterator over all entries in the map.
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            inner: self.inner.ref_iter(),
        }
    }

    /// Returns an iterator over a subset of entries in the skip list.
    pub fn range<Q, R>(&self, range: R) -> Range<'_, Q, R, K, V>
    where
        K: Borrow<Q>,
        R: RangeBounds<Q>,
        Q: Ord + ?Sized,
    {
        Range {
            inner: self.inner.ref_range(range),
        }
    }
}

impl<K, V> SkipMap<K, V>
where
    K: Ord + Send + 'static,
    V: Send + 'static,
{
    /// Inserts a `key`-`value` pair into the map and returns the new entry.
    ///
    /// If there is an existing entry with this key, it will be removed before inserting the new
    /// one.
    pub fn insert(&self, key: K, value: V) -> Entry<'_, K, V> {
        let guard = &epoch::pin();
        Entry::new(self.inner.insert(key, value, guard))
    }

    /// Removes an entry with the specified `key` from the map and returns it.
    pub fn remove<Q>(&self, key: &Q) -> Option<Entry<'_, K, V>>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        let guard = &epoch::pin();
        self.inner.remove(key, guard).map(Entry::new)
    }

    /// Removes an entry from the front of the map.
    pub fn pop_front(&self) -> Option<Entry<'_, K, V>> {
        let guard = &epoch::pin();
        self.inner.pop_front(guard).map(Entry::new)
    }

    /// Removes an entry from the back of the map.
    pub fn pop_back(&self) -> Option<Entry<'_, K, V>> {
        let guard = &epoch::pin();
        self.inner.pop_back(guard).map(Entry::new)
    }

    /// Iterates over the map and removes every entry.
    pub fn clear(&self) {
        let guard = &mut epoch::pin();
        self.inner.clear(guard);
    }
}

impl<K, V> Default for SkipMap<K, V> {
    fn default() -> SkipMap<K, V> {
        SkipMap::new()
    }
}

impl<K, V> fmt::Debug for SkipMap<K, V>
where
    K: Ord + fmt::Debug,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("SkipMap { .. }")
    }
}

impl<K, V> IntoIterator for SkipMap<K, V> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;

    fn into_iter(self) -> IntoIter<K, V> {
        IntoIter {
            inner: self.inner.into_iter(),
        }
    }
}

impl<'a, K, V> IntoIterator for &'a SkipMap<K, V>
where
    K: Ord,
{
    type Item = Entry<'a, K, V>;
    type IntoIter = Iter<'a, K, V>;

    fn into_iter(self) -> Iter<'a, K, V> {
        self.iter()
    }
}

impl<K, V> FromIterator<(K, V)> for SkipMap<K, V>
where
    K: Ord,
{
    fn from_iter<I>(iter: I) -> SkipMap<K, V>
    where
        I: IntoIterator<Item = (K, V)>,
    {
        let s = SkipMap::new();
        for (k, v) in iter {
            s.get_or_insert(k, v);
        }
        s
    }
}

/// A reference-counted entry in a map.
pub struct Entry<'a, K, V> {
    inner: ManuallyDrop<base::RefEntry<'a, K, V>>,
}

impl<'a, K, V> Entry<'a, K, V> {
    fn new(inner: base::RefEntry<'a, K, V>) -> Entry<'a, K, V> {
        Entry {
            inner: ManuallyDrop::new(inner),
        }
    }

    /// Returns a reference to the key.
    pub fn key(&self) -> &K {
        self.inner.key()
    }

    /// Returns a reference to the value.
    pub fn value(&self) -> &V {
        self.inner.value()
    }

    /// Returns `true` if the entry is removed from the map.
    pub fn is_removed(&self) -> bool {
        self.inner.is_removed()
    }
}

impl<K, V> Drop for Entry<'_, K, V> {
    fn drop(&mut self) {
        unsafe {
            ManuallyDrop::into_inner(ptr::read(&self.inner)).release_with_pin(epoch::pin);
        }
    }
}

impl<'a, K, V> Entry<'a, K, V>
where
    K: Ord,
{
    /// Moves to the next entry in the map.
    pub fn move_next(&mut self) -> bool {
        let guard = &epoch::pin();
        self.inner.move_next(guard)
    }

    /// Moves to the previous entry in the map.
    pub fn move_prev(&mut self) -> bool {
        let guard = &epoch::pin();
        self.inner.move_prev(guard)
    }

    /// Returns the next entry in the map.
    pub fn next(&self) -> Option<Entry<'a, K, V>> {
        let guard = &epoch::pin();
        self.inner.next(guard).map(Entry::new)
    }

    /// Returns the previous entry in the map.
    pub fn prev(&self) -> Option<Entry<'a, K, V>> {
        let guard = &epoch::pin();
        self.inner.prev(guard).map(Entry::new)
    }
}

impl<K, V> Entry<'_, K, V>
where
    K: Ord + Send + 'static,
    V: Send + 'static,
{
    /// Removes the entry from the map.
    ///
    /// Returns `true` if this call removed the entry and `false` if it was already removed.
    pub fn remove(&self) -> bool {
        let guard = &epoch::pin();
        self.inner.remove(guard)
    }
}

impl<'a, K, V> Clone for Entry<'a, K, V> {
    fn clone(&self) -> Entry<'a, K, V> {
        Entry {
            inner: self.inner.clone(),
        }
    }
}

impl<K, V> fmt::Debug for Entry<'_, K, V>
where
    K: fmt::Debug,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Entry")
            .field(self.key())
            .field(self.value())
            .finish()
    }
}

/// An owning iterator over the entries of a `SkipMap`.
pub struct IntoIter<K, V> {
    inner: base::IntoIter<K, V>,
}

impl<K, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);

    fn next(&mut self) -> Option<(K, V)> {
        self.inner.next()
    }
}

impl<K, V> fmt::Debug for IntoIter<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("IntoIter { .. }")
    }
}

/// An iterator over the entries of a `SkipMap`.
pub struct Iter<'a, K, V> {
    inner: base::RefIter<'a, K, V>,
}

impl<'a, K, V> Iterator for Iter<'a, K, V>
where
    K: Ord,
{
    type Item = Entry<'a, K, V>;

    fn next(&mut self) -> Option<Entry<'a, K, V>> {
        let guard = &epoch::pin();
        self.inner.next(guard).map(Entry::new)
    }
}

impl<'a, K, V> DoubleEndedIterator for Iter<'a, K, V>
where
    K: Ord,
{
    fn next_back(&mut self) -> Option<Entry<'a, K, V>> {
        let guard = &epoch::pin();
        self.inner.next_back(guard).map(Entry::new)
    }
}

impl<K, V> fmt::Debug for Iter<'_, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("Iter { .. }")
    }
}

/// An iterator over the entries of a `SkipMap`.
pub struct Range<'a, Q, R, K, V>
where
    K: Ord + Borrow<Q>,
    R: RangeBounds<Q>,
    Q: Ord + ?Sized,
{
    pub(crate) inner: base::RefRange<'a, Q, R, K, V>,
}

impl<'a, Q, R, K, V> Iterator for Range<'a, Q, R, K, V>
where
    K: Ord + Borrow<Q>,
    R: RangeBounds<Q>,
    Q: Ord + ?Sized,
{
    type Item = Entry<'a, K, V>;

    fn next(&mut self) -> Option<Entry<'a, K, V>> {
        let guard = &epoch::pin();
        self.inner.next(guard).map(Entry::new)
    }
}

impl<'a, Q, R, K, V> DoubleEndedIterator for Range<'a, Q, R, K, V>
where
    K: Ord + Borrow<Q>,
    R: RangeBounds<Q>,
    Q: Ord + ?Sized,
{
    fn next_back(&mut self) -> Option<Entry<'a, K, V>> {
        let guard = &epoch::pin();
        self.inner.next_back(guard).map(Entry::new)
    }
}

impl<Q, R, K, V> fmt::Debug for Range<'_, Q, R, K, V>
where
    K: Ord + Borrow<Q> + fmt::Debug,
    V: fmt::Debug,
    R: RangeBounds<Q> + fmt::Debug,
    Q: Ord + ?Sized,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Range")
            .field("range", &self.inner.range)
            .field("head", &self.inner.head)
            .field("tail", &self.inner.tail)
            .finish()
    }
}
#[cfg(test)]
mod tests_rug_593 {
    use super::*;
    use crate::map::SkipMap;
    
    #[test]
    fn test_new() {
        let map: SkipMap<i32, &str> = SkipMap::new();
        // Add assertion or further test logic here
    }
}#[cfg(test)]
mod tests_rug_594 {
    use super::*;
    use crate::map::SkipMap;

    #[test]
    fn test_rug() {
        let mut p0: SkipMap<i32, i32> = SkipMap::new();

        SkipMap::<i32, i32>::is_empty(&p0);
    }
}#[cfg(test)]
mod tests_rug_595 {
    use super::*;
    use crate::map::SkipMap;

    #[test]
    fn test_len() {
        let skip_map: SkipMap<i32, &str> = SkipMap::new();

        assert_eq!(skip_map.len(), 0);
    }
}#[cfg(test)]
mod tests_rug_596 {
    use super::*;
    use crate::map::SkipMap;
    use crate::map::Entry;
    use crate::epoch;

    #[test]
    fn test_rug() {
        let p0: SkipMap<i32, &str> = SkipMap::new();

        p0.front();
    }
}#[cfg(test)]
mod tests_rug_597 {
    use super::*;
    use crate::map;
    use crate::epoch;

    #[test]
    fn test_back() {
        let p0: map::SkipMap<i32, i32> = map::SkipMap::new();

        p0.back();

    }
}#[cfg(test)]
mod tests_rug_598 {
    use super::*;
    use crate::map::SkipMap;
    use crate::epoch;

    #[test]
    fn test_contains_key() {
        let skip_map: SkipMap<i64, i64> = SkipMap::new();

        let key_to_check = 42;

        let result = skip_map.contains_key(&key_to_check);

        assert_eq!(result, false);
    }
}#[cfg(test)]
mod tests_rug_599 {
    use super::*;
    use crate::map::SkipMap;
    use crate::map::Entry;
    
    #[test]
    fn test_rug() {
        let mut p0: SkipMap<u32, u32> = SkipMap::new();
        let mut p1: u32 = 42;

        p0.get(&p1);
    }
}#[cfg(test)]
mod tests_rug_600 {
    use super::*;
    use crate::map::SkipMap;
    use crate::map::Entry;
    use crossbeam_epoch as epoch;
    use crossbeam_epoch::Owned;
    use std::borrow::Borrow;
    use std::ops::Bound;

    #[test]
    fn test_rug() {
        let mut p0: SkipMap<i32, i32> = SkipMap::new();
        let p1: Bound<&i32> = Bound::Excluded(&42);
        
        p0.lower_bound(p1);
    }
}#[cfg(test)]
mod tests_rug_602 {
    use super::*;
    use crate::map::SkipMap;
    use crate::map::Entry;
    use crate::epoch;

    #[test]
    fn test_rug() {
        let mut p0: SkipMap<usize, usize> = SkipMap::new();
        let p1: usize = 10;
        let p2: usize = 100;

        p0.get_or_insert(p1, p2);
    }
}#[cfg(test)]
mod tests_rug_603 {
    use super::*;
    use crate::map::SkipMap;

    #[test]
    fn test_iter() {
        let mut p0: SkipMap<i32, i32> = SkipMap::new();

        SkipMap::<i32, i32>::iter(&p0);
    }
}#[cfg(test)]
mod tests_rug_604 {
    use super::*;
    use crate::map::SkipMap;
    use std::ops::RangeTo;

    #[test]
    fn test_rug() {
        let mut p0: SkipMap<(), ()> = SkipMap::new();
        let p1 = ..;

        p0.range(p1);
    }
}#[cfg(test)]
mod tests_rug_606 {
    use super::*;
    use crate::map::{SkipMap, Entry};
    use std::borrow::Borrow;
    
    #[test]
    fn test_rug() {
        let mut p0: SkipMap<i8, i8> = SkipMap::new();
        let p1: i8 = 0;

        p0.remove(&p1);
    }
}#[cfg(test)]
mod tests_rug_607 {
    use super::*;
    use crate::map;

    #[test]
    fn test_pop_front() {
        let skiplist: map::SkipMap<i32, i32> = map::SkipMap::new();
        
        skiplist.pop_front();
    }
}#[cfg(test)]
mod tests_rug_608 {
    use super::*;
    use crate::map;
    
    #[test]
    fn test_pop_back() {
        let mut p0: map::SkipMap<i32, i32> = map::SkipMap::new();

        map::SkipMap::<i32, i32>::pop_back(&p0);
    }
}#[cfg(test)]
mod tests_rug_609 {
    use super::*;
    use crate::map::SkipMap;
    
    #[test]
    fn test_clear() {
        let p0: SkipMap<i32, i32> = SkipMap::new();
        
        p0.clear();
    }
}#[cfg(test)]
mod tests_rug_610 {
    use super::*;
    use crate::map::SkipMap;
    
    use std::default::Default;

    #[test]
    fn test_default() {
        SkipMap::<i32, i32>::default();
    }
}#[cfg(test)]
mod tests_rug_613 {
    use super::*;
    use crate::set::SkipSet;
    use crate::map::SkipMap;
    
    use std::iter::FromIterator;

    #[test]
    fn test_rug() {
        let mut p0: SkipSet<i32> = SkipSet::new();
        p0.insert(1);
        p0.insert(2);
        p0.insert(3);
        
        SkipMap::<i32, i32>::from_iter(p0.into_iter().map(|item| (item, item * 2)));
    }
}
#[cfg(test)]
mod tests_rug_615 {
    use super::*;

    use crate::map;

    #[test]
    fn test_rug() {
        let mut inner_map: map::SkipMap<i32, i32> = map::SkipMap::new();
        let entry = inner_map.insert(1, 100);

        map::Entry::<i32, i32>::key(&entry);
    }
}

#[cfg(test)]
mod tests_rug_616 {
    use super::*;
    use crate::map::Entry;

    #[test]
    fn test_rug() {
        let mut p0: Entry<'_, i32, String> = unimplemented!();

        p0.value();
    }
}
#[cfg(test)]
mod tests_rug_617 {
    use super::*;
    use crate::map;

    #[test]
    fn test_is_removed() {
        let key: i32 = 42;
        let value: i32 = 100;
        let skiplist_map = map::SkipMap::new();
        let entry = skiplist_map.insert(key, value);

        assert_eq!(map::Entry::<i32, i32>::is_removed(&entry), false);
        skiplist_map.remove(&key);
        assert_eq!(map::Entry::<i32, i32>::is_removed(&entry), true);
    }
}#[cfg(test)]
mod tests_rug_621 {
    use super::*;
    use crate::map;
    use crate::epoch;
    
    #[test]
    fn test_next() {
        let k = 1;
        let v = "value";
        
        let guard = &epoch::pin();
        let skiplist = map::SkipMap::new();
        let entry = skiplist.insert(k, v);
        
        map::Entry::<'_, i32, &str>::next(&entry);
    }
}#[cfg(test)]
mod tests_rug_622 {
    use super::*;
    use crate::map::{Entry, SkipMap};
    use crossbeam_epoch as epoch;

    #[test]
    fn test_prev() {
        // Create a SkipMap for testing
        let map: SkipMap<i32, i32> = SkipMap::new();
        map.insert(1, 10);
        
        // Get the first entry in the map
        let first_entry = map.get(&1).unwrap();
        
        // Get the previous entry
        let prev_entry = first_entry.prev().unwrap();
        
        // Perform assertions or further operations as needed
        assert_eq!(prev_entry.key(), &1);
    }
}