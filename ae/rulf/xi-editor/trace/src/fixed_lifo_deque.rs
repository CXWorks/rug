// Copyright 2018 The xi-editor Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::cmp::{self, Ordering};
use std::collections::vec_deque::{Drain, IntoIter, Iter, IterMut, VecDeque};
use std::hash::{Hash, Hasher};
use std::ops::{Index, IndexMut, RangeBounds};

/// Provides fixed size ring buffer that overwrites elements in FIFO order on
/// insertion when full.  API provided is similar to VecDeque & uses a VecDeque
/// internally. One distinction is that only append-like insertion is allowed.
/// This means that insert & push_front are not allowed.  The reasoning is that
/// there is ambiguity on how such functions should operate since it would be
/// pretty impossible to maintain a FIFO ordering.
///
/// All operations that would cause growth beyond the limit drop the appropriate
/// number of elements from the front.  For example, on a full buffer push_front
/// replaces the first element.
///
/// The removal of elements on operation that would cause excess beyond the
/// limit happens first to make sure the space is available in the underlying
/// VecDeque, thus guaranteeing O(1) operations always.
#[derive(Clone, Debug)]
pub struct FixedLifoDeque<T> {
    storage: VecDeque<T>,
    limit: usize,
}

impl<T> FixedLifoDeque<T> {
    /// Constructs a ring buffer that will reject all insertions as no-ops.
    /// This also construct the underlying VecDeque with_capacity(0) which
    /// in the current stdlib implementation allocates 2 Ts.
    #[inline]
    pub fn new() -> Self {
        FixedLifoDeque::with_limit(0)
    }

    /// Constructs a fixed size ring buffer with the given number of elements.
    /// Attempts to insert more than this number of elements will cause excess
    /// elements to first be evicted in FIFO order (i.e. from the front).
    pub fn with_limit(n: usize) -> Self {
        FixedLifoDeque { storage: VecDeque::with_capacity(n), limit: n }
    }

    /// This sets a new limit on the container.  Excess elements are dropped in
    /// FIFO order.  The new capacity is reset to the requested limit which will
    /// likely result in re-allocation + copies/clones even if the limit
    /// shrinks.
    pub fn reset_limit(&mut self, n: usize) {
        if n < self.limit {
            let overflow = self.limit - n;
            self.drop_excess_for_inserting(overflow);
        }
        self.limit = n;
        self.storage.reserve_exact(n);
        self.storage.shrink_to_fit();
        debug_assert!(self.storage.len() <= self.limit);
    }

    /// Returns the current limit this ring buffer is configured with.
    #[inline]
    pub fn limit(&self) -> usize {
        self.limit
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<&T> {
        self.storage.get(index)
    }

    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.storage.get_mut(index)
    }

    #[inline]
    pub fn swap(&mut self, i: usize, j: usize) {
        self.storage.swap(i, j);
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.limit
    }

    #[inline]
    pub fn iter(&self) -> Iter<T> {
        self.storage.iter()
    }

    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<T> {
        self.storage.iter_mut()
    }

    /// Returns a tuple of 2 slices that represents the ring buffer. [0] is the
    /// beginning of the buffer to the physical end of the array or the last
    /// element (whichever comes first).  [1] is the continuation of [0] if the
    /// ring buffer has wrapped the contiguous storage.
    #[inline]
    pub fn as_slices(&self) -> (&[T], &[T]) {
        self.storage.as_slices()
    }

    #[inline]
    pub fn as_mut_slices(&mut self) -> (&mut [T], &mut [T]) {
        self.storage.as_mut_slices()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.storage.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.storage.is_empty()
    }

    #[inline]
    pub fn drain<R>(&mut self, range: R) -> Drain<T>
    where
        R: RangeBounds<usize>,
    {
        self.storage.drain(range)
    }

    #[inline]
    pub fn clear(&mut self) {
        self.storage.clear();
    }

    #[inline]
    pub fn contains(&self, x: &T) -> bool
    where
        T: PartialEq<T>,
    {
        self.storage.contains(x)
    }

    #[inline]
    pub fn front(&self) -> Option<&T> {
        self.storage.front()
    }

    #[inline]
    pub fn front_mut(&mut self) -> Option<&mut T> {
        self.storage.front_mut()
    }

    #[inline]
    pub fn back(&self) -> Option<&T> {
        self.storage.back()
    }

    #[inline]
    pub fn back_mut(&mut self) -> Option<&mut T> {
        self.storage.back_mut()
    }

    #[inline]
    fn drop_excess_for_inserting(&mut self, n_to_be_inserted: usize) {
        if self.storage.len() + n_to_be_inserted > self.limit {
            let overflow =
                self.storage.len().min(self.storage.len() + n_to_be_inserted - self.limit);
            self.storage.drain(..overflow);
        }
    }

    /// Always an O(1) operation.  Memory is never reclaimed.
    #[inline]
    pub fn pop_front(&mut self) -> Option<T> {
        self.storage.pop_front()
    }

    /// Always an O(1) operation.  If the number of elements is at the limit,
    /// the element at the front is overwritten.
    ///
    /// Post condition: The number of elements is <= limit
    pub fn push_back(&mut self, value: T) {
        self.drop_excess_for_inserting(1);
        self.storage.push_back(value);
        // For when limit == 0
        self.drop_excess_for_inserting(0);
    }

    /// Always an O(1) operation.  Memory is never reclaimed.
    #[inline]
    pub fn pop_back(&mut self) -> Option<T> {
        self.storage.pop_back()
    }

    #[inline]
    pub fn swap_remove_back(&mut self, index: usize) -> Option<T> {
        self.storage.swap_remove_back(index)
    }

    #[inline]
    pub fn swap_remove_front(&mut self, index: usize) -> Option<T> {
        self.storage.swap_remove_front(index)
    }

    /// Always an O(1) operation.
    #[inline]
    pub fn remove(&mut self, index: usize) -> Option<T> {
        self.storage.remove(index)
    }

    pub fn split_off(&mut self, at: usize) -> FixedLifoDeque<T> {
        FixedLifoDeque { storage: self.storage.split_off(at), limit: self.limit }
    }

    /// Always an O(m) operation where m is the length of `other'.
    pub fn append(&mut self, other: &mut VecDeque<T>) {
        self.drop_excess_for_inserting(other.len());
        self.storage.append(other);
        // For when limit == 0
        self.drop_excess_for_inserting(0);
    }

    #[inline]
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&T) -> bool,
    {
        self.storage.retain(f);
    }
}

impl<T: Clone> FixedLifoDeque<T> {
    /// Resizes a fixed queue.  This doesn't change the limit so the resize is
    /// capped to the limit.  Additionally, resizing drops the elements from the
    /// front unlike with a regular VecDeque.
    pub fn resize(&mut self, new_len: usize, value: T) {
        if new_len < self.len() {
            let to_drop = self.len() - new_len;
            self.storage.drain(..to_drop);
        } else {
            self.storage.resize(cmp::min(self.limit, new_len), value);
        }
    }
}

impl<A: PartialEq> PartialEq for FixedLifoDeque<A> {
    #[inline]
    fn eq(&self, other: &FixedLifoDeque<A>) -> bool {
        self.storage == other.storage
    }
}

impl<A: Eq> Eq for FixedLifoDeque<A> {}

impl<A: PartialOrd> PartialOrd for FixedLifoDeque<A> {
    #[inline]
    fn partial_cmp(&self, other: &FixedLifoDeque<A>) -> Option<Ordering> {
        self.storage.partial_cmp(&other.storage)
    }
}

impl<A: Ord> Ord for FixedLifoDeque<A> {
    #[inline]
    fn cmp(&self, other: &FixedLifoDeque<A>) -> Ordering {
        self.storage.cmp(&other.storage)
    }
}

impl<A: Hash> Hash for FixedLifoDeque<A> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.storage.hash(state);
    }
}

impl<A> Index<usize> for FixedLifoDeque<A> {
    type Output = A;

    #[inline]
    fn index(&self, index: usize) -> &A {
        &self.storage[index]
    }
}

impl<A> IndexMut<usize> for FixedLifoDeque<A> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut A {
        &mut self.storage[index]
    }
}

impl<T> IntoIterator for FixedLifoDeque<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    /// Consumes the list into a front-to-back iterator yielding elements by
    /// value.
    #[inline]
    fn into_iter(self) -> IntoIter<T> {
        self.storage.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a FixedLifoDeque<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    #[inline]
    fn into_iter(self) -> Iter<'a, T> {
        self.storage.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut FixedLifoDeque<T> {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;

    #[inline]
    fn into_iter(self) -> IterMut<'a, T> {
        self.storage.iter_mut()
    }
}

impl<A> Extend<A> for FixedLifoDeque<A> {
    fn extend<T: IntoIterator<Item = A>>(&mut self, iter: T) {
        for elt in iter {
            self.push_back(elt);
        }
    }
}

impl<'a, T: 'a + Copy> Extend<&'a T> for FixedLifoDeque<T> {
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) {
        self.extend(iter.into_iter().cloned());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "benchmarks")]
    use test::Bencher;

    #[test]
    fn test_basic_insertions() {
        let mut tester = FixedLifoDeque::with_limit(3);
        assert_eq!(tester.len(), 0);
        assert_eq!(tester.capacity(), 3);
        assert_eq!(tester.front(), None);
        assert_eq!(tester.back(), None);

        tester.push_back(1);
        assert_eq!(tester.len(), 1);
        assert_eq!(tester.front(), Some(1).as_ref());
        assert_eq!(tester.back(), Some(1).as_ref());

        tester.push_back(2);
        assert_eq!(tester.len(), 2);
        assert_eq!(tester.front(), Some(1).as_ref());
        assert_eq!(tester.back(), Some(2).as_ref());

        tester.push_back(3);
        tester.push_back(4);
        assert_eq!(tester.len(), 3);
        assert_eq!(tester.front(), Some(2).as_ref());
        assert_eq!(tester.back(), Some(4).as_ref());
        assert_eq!(tester[0], 2);
        assert_eq!(tester[1], 3);
        assert_eq!(tester[2], 4);
    }

    #[cfg(feature = "benchmarks")]
    #[bench]
    fn bench_push_back(b: &mut Bencher) {
        let mut q = FixedLifoDeque::with_limit(10);
        b.iter(|| q.push_back(5));
    }

    #[cfg(feature = "benchmarks")]
    #[bench]
    fn bench_deletion_from_empty(b: &mut Bencher) {
        let mut q = FixedLifoDeque::<u32>::with_limit(10000);
        b.iter(|| q.pop_front());
    }

    #[cfg(feature = "benchmarks")]
    #[bench]
    fn bench_deletion_from_non_empty(b: &mut Bencher) {
        let mut q = FixedLifoDeque::with_limit(1000000);
        for i in 0..q.limit() {
            q.push_back(i);
        }
        b.iter(|| q.pop_front());
    }
}
#[cfg(test)]
mod tests_llm_16_2 {
    use super::*;

use crate::*;
    use std::collections::VecDeque;
    
    #[test]
    fn test_into_iter() {
        let mut deque: FixedLifoDeque<i32> = FixedLifoDeque::with_limit(3);
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);

        let mut iter = deque.into_iter();

        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(3));
        assert_eq!(iter.next(), None);
    }
}#[cfg(test)]
mod tests_llm_16_3 {
    use super::*;

use crate::*;
    use std::collections::VecDeque;
    use std::iter::FromIterator;

    #[test]
    fn test_into_iter() {
        let mut deque: FixedLifoDeque<i32> = FixedLifoDeque::with_limit(3);
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);

        let mut iter = deque.into_iter();
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(3));
        assert_eq!(iter.next(), None);
    }
}#[cfg(test)]
mod tests_llm_16_52_llm_16_51 {
    use super::*;

use crate::*;
    use std::collections::VecDeque;
    
    #[test]
    fn test_cmp() {
        let mut deque1: FixedLifoDeque<i32> = FixedLifoDeque::with_limit(5);
        let mut deque2: FixedLifoDeque<i32> = FixedLifoDeque::with_limit(5);
        deque1.push_back(1);
        deque1.push_back(2);
        deque2.push_back(1);
        deque2.push_back(2);
        assert_eq!(deque1.cmp(&deque2), Ordering::Equal);
        deque1.push_back(3);
        deque2.push_back(4);
        assert_eq!(deque1.cmp(&deque2), Ordering::Less);
        assert_eq!(deque2.cmp(&deque1), Ordering::Greater);
        deque1.push_back(5);
        deque2.push_back(5);
        assert_eq!(deque1.cmp(&deque2), Ordering::Equal);
    }
}#[cfg(test)]
mod tests_llm_16_53 {
    use super::*;

use crate::*;
    use std::collections::VecDeque;

    #[test]
    fn test_eq() {
        let deque1: FixedLifoDeque<i32> = FixedLifoDeque::new();
        let deque2: FixedLifoDeque<i32> = FixedLifoDeque::new();
        assert_eq!(deque1.eq(&deque2), true);

        let deque3 = FixedLifoDeque {
            storage: VecDeque::from(vec![1, 2, 3]),
            limit: 3,
        };
        let deque4 = FixedLifoDeque {
            storage: VecDeque::from(vec![1, 2, 3]),
            limit: 3,
        };
        assert_eq!(deque3.eq(&deque4), true);

        let deque5 = FixedLifoDeque {
            storage: VecDeque::from(vec![1, 2, 3]),
            limit: 3,
        };
        let deque6 = FixedLifoDeque {
            storage: VecDeque::from(vec![4, 5, 6]),
            limit: 3,
        };
        assert_eq!(deque5.eq(&deque6), false);
    }
}#[cfg(test)]
mod tests_llm_16_54 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_partial_cmp() {
        let deque1: FixedLifoDeque<i32> = FixedLifoDeque::new();
        let deque2: FixedLifoDeque<i32> = FixedLifoDeque::new();
        assert_eq!(deque1.partial_cmp(&deque2), Some(Ordering::Equal));
        
        let mut deque3: FixedLifoDeque<i32> = FixedLifoDeque::new();
        deque3.push_back(1);
        assert_eq!(deque1.partial_cmp(&deque3), Some(Ordering::Less));
        assert_eq!(deque3.partial_cmp(&deque1), Some(Ordering::Greater));
        
        let mut deque4: FixedLifoDeque<i32> = FixedLifoDeque::new();
        deque4.push_back(1);
        assert_eq!(deque3.partial_cmp(&deque4), Some(Ordering::Equal));
        
        let mut deque5: FixedLifoDeque<i32> = FixedLifoDeque::with_limit(3);
        deque5.push_back(1);
        deque5.push_back(2);
        deque5.push_back(3);
        let mut deque6: FixedLifoDeque<i32> = FixedLifoDeque::with_limit(3);
        deque6.push_back(1);
        deque6.push_back(2);
        deque6.push_back(4);
        assert_eq!(deque5.partial_cmp(&deque6), Some(Ordering::Less));
        assert_eq!(deque6.partial_cmp(&deque5), Some(Ordering::Greater));
        
        let mut deque7: FixedLifoDeque<i32> = FixedLifoDeque::with_limit(3);
        deque7.push_back(1);
        deque7.push_back(2);
        deque7.push_back(3);
        let mut deque8: FixedLifoDeque<i32> = FixedLifoDeque::with_limit(3);
        deque8.push_back(1);
        deque8.push_back(2);
        deque8.push_back(3);
        assert_eq!(deque7.partial_cmp(&deque8), Some(Ordering::Equal));
    }
}#[cfg(test)]
mod tests_llm_16_55 {
    use super::*;

use crate::*;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    #[test]
    fn test_hash() {
        let mut deque = FixedLifoDeque::new();
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        
        let mut hasher = DefaultHasher::new();
        deque.hash(&mut hasher);
        let hash1 = hasher.finish();

        let mut deque2 = FixedLifoDeque::new();
        deque2.push_back(1);
        deque2.push_back(2);
        deque2.push_back(3);
        
        let mut hasher2 = DefaultHasher::new();
        deque2.hash(&mut hasher2);
        let hash2 = hasher2.finish();

        assert_eq!(hash1, hash2);
    }
}#[cfg(test)]
mod tests_llm_16_56 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_extend() {
        let mut deque = FixedLifoDeque::new();
        deque.extend(vec![1, 2, 3]);
        assert_eq!(deque.len(), 3);
        assert_eq!(deque[0], 1);
        assert_eq!(deque[1], 2);
        assert_eq!(deque[2], 3);
    }
}#[cfg(test)]
mod tests_llm_16_57 {
    use super::*;

use crate::*;
    use std::collections::VecDeque;

    #[test]
    fn test_index() {
        let mut deque: FixedLifoDeque<i32> = FixedLifoDeque {
            storage: VecDeque::new(),
            limit: 4,
        };
        
        deque.push_back(10);
        deque.push_back(20);
        deque.push_back(30);
        deque.push_back(40);
        
        let index1 = deque.index(1);
        let index2 = deque.index(2);
        let index3 = deque.index(3);
        
        assert_eq!(*index1, 20);
        assert_eq!(*index2, 30);
        assert_eq!(*index3, 40);
    }
}#[cfg(test)]
mod tests_llm_16_58 {
    use super::*;

use crate::*;
    use std::ops::IndexMut;

    #[test]
    fn test_index_mut() {
        let mut deque: FixedLifoDeque<i32> = FixedLifoDeque::with_limit(3);
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);

        *deque.index_mut(1) = 5;

        assert_eq!(deque[0], 1);
        assert_eq!(deque[1], 5);
        assert_eq!(deque[2], 3);
    }
}#[cfg(test)]
mod tests_llm_16_59 {
    use super::*;

use crate::*;
    use std::collections::VecDeque;

    #[test]
    fn test_extend() {
        let mut deque: FixedLifoDeque<i32> = FixedLifoDeque::with_limit(5);
        deque.extend(&vec![1, 2, 3]);

        let expected: VecDeque<i32> = vec![1, 2, 3].into_iter().collect::<VecDeque<_>>();
        assert_eq!(deque.storage, expected);
    }
}#[cfg(test)]
mod tests_llm_16_60 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_into_iter() {
        let mut deque: FixedLifoDeque<i32> = FixedLifoDeque::new();
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
    
        let mut iter = deque.into_iter();
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(3));
        assert_eq!(iter.next(), None);
    }
}#[cfg(test)]
mod tests_llm_16_127 {
    use super::*;

use crate::*;
    use std::collections::VecDeque;

    #[test]
    fn test_append() {
        let mut deque: FixedLifoDeque<i32> = FixedLifoDeque::new();
        let mut other: VecDeque<i32> = VecDeque::new();
        other.push_back(1);
        other.push_back(2);
        other.push_back(3);

        deque.append(&mut other);

        assert_eq!(deque.len(), 3);
        assert_eq!(deque.get(0), Some(&1));
        assert_eq!(deque.get(1), Some(&2));
        assert_eq!(deque.get(2), Some(&3));
    }
}#[cfg(test)]
mod tests_llm_16_130 {
    use super::*;

use crate::*;
    use std::cmp::Ordering;

    #[test]
    fn test_as_slices() {
        let deque: FixedLifoDeque<u32> = FixedLifoDeque::new();
        let slices = deque.as_slices();
        assert_eq!(slices, (&[][..], &[][..]));

        let mut deque: FixedLifoDeque<u32> = FixedLifoDeque::with_limit(3);
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        let slices = deque.as_slices();
        assert_eq!(slices, (&[1, 2, 3][..], &[][..]));

        deque.push_back(4);
        deque.push_back(5);
        let slices = deque.as_slices();
        assert_eq!(slices, (&[3, 4, 5][..], &[][..]));

        deque.push_back(6);
        let slices = deque.as_slices();
        assert_eq!(slices, (&[4, 5, 6][..], &[][..]));

        deque.push_back(7);
        let slices = deque.as_slices();
        assert_eq!(slices, (&[5, 6, 7][..], &[][..]));

        deque.pop_front();
        let slices = deque.as_slices();
        assert_eq!(slices, (&[6, 7][..], &[][..]));

        deque.push_back(8);
        deque.push_back(9);
        let slices = deque.as_slices();
        assert_eq!(slices, (&[6, 7, 8][..], &[9][..]));

        deque.pop_front();
        deque.pop_front();
        let slices = deque.as_slices();
        assert_eq!(slices, (&[8][..], &[9][..]));

        deque.push_back(10);
        deque.push_back(11);
        deque.push_back(12);
        deque.push_back(13);
        deque.push_back(14);
        let slices = deque.as_slices();
        assert_eq!(slices, (&[8, 9, 10][..], &[11, 12, 13, 14][..]));
    }
}#[cfg(test)]
mod tests_llm_16_131 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_back_empty() {
        let deque: FixedLifoDeque<i32> = FixedLifoDeque::new();
        assert_eq!(deque.back(), None);
    }
    
    #[test]
    fn test_back_one_element() {
        let mut deque: FixedLifoDeque<i32> = FixedLifoDeque::new();
        deque.push_back(5);
        assert_eq!(deque.back(), Some(&5));
    }
    
    #[test]
    fn test_back_multiple_elements() {
        let mut deque: FixedLifoDeque<i32> = FixedLifoDeque::new();
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        assert_eq!(deque.back(), Some(&3));
    }
}#[cfg(test)]
mod tests_llm_16_133_llm_16_132 {
    use super::*;

use crate::*;

    #[test]
    fn test_back_mut() {
        let mut deque: FixedLifoDeque<i32> = FixedLifoDeque::new();
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        
        let mut back = deque.back_mut().unwrap();
        *back = 4;
        
        assert_eq!(*deque.back().unwrap(), 4);
    }
}#[cfg(test)]
mod tests_llm_16_134 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_capacity() {
        let deque: FixedLifoDeque<u32> = FixedLifoDeque::with_limit(5);
        assert_eq!(deque.capacity(), 5);
    }
}#[cfg(test)]
mod tests_llm_16_135 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_clear() {
        let mut deque: FixedLifoDeque<i32> = FixedLifoDeque::with_limit(5);
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        
        deque.clear();
        
        assert!(deque.is_empty());
    }
}#[cfg(test)]
mod tests_llm_16_136 {
    use super::*;

use crate::*;
    use std::vec::Vec;

    #[test]
    fn test_contains() {
        let deque: FixedLifoDeque<CategoriesT> = FixedLifoDeque::new();
        assert_eq!(deque.contains(&CategoriesT::StaticArray(&[])), false);
        assert_eq!(deque.contains(&CategoriesT::DynamicArray(Vec::new())), false);
    }
}#[cfg(test)]
mod tests_llm_16_137 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_drain() {
        let mut deque: FixedLifoDeque<u32> = FixedLifoDeque::new();
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        deque.push_back(4);
        deque.push_back(5);
        
        let result: Vec<u32> = deque.drain(1..4).collect();
        assert_eq!(result, vec![2, 3, 4]);
        assert_eq!(deque.len(), 2);
        assert_eq!(deque[0], 1);
        assert_eq!(deque[1], 5);
    }
}#[cfg(test)]
mod tests_llm_16_138 {
    use super::*;

use crate::*;
    use std::collections::VecDeque;

    #[test]
    fn test_drop_excess_for_inserting() {
        let mut deque: FixedLifoDeque<i32> = FixedLifoDeque::with_limit(3);
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        deque.drop_excess_for_inserting(2);
        assert_eq!(deque.storage, VecDeque::from(vec![2, 3]));
    }
}#[cfg(test)]
mod tests_llm_16_139 {
    use super::*;

use crate::*;

    #[test]
    fn test_front_returns_some_value_when_storage_is_not_empty() {
        let mut deque = FixedLifoDeque::new();
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);

        assert_eq!(deque.front(), Some(&1));
    }

    #[test]
    fn test_front_returns_none_when_storage_is_empty() {
        let deque: FixedLifoDeque<i32> = FixedLifoDeque::new();

        assert_eq!(deque.front(), None);
    }
}#[cfg(test)]
mod tests_llm_16_142 {
    use super::*;

use crate::*;
    use std::collections::VecDeque;

    #[test]
    fn test_get_index_less_than_limit() {
        let mut deque: FixedLifoDeque<u32> = FixedLifoDeque::with_limit(3);
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        let result = deque.get(1);
        assert_eq!(result, Some(&2));
    }

    #[test]
    fn test_get_index_equal_to_limit() {
        let mut deque: FixedLifoDeque<u32> = FixedLifoDeque::with_limit(3);
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        let result = deque.get(3);
        assert_eq!(result, Some(&1));
    }

    #[test]
    fn test_get_index_greater_than_limit() {
        let mut deque: FixedLifoDeque<u32> = FixedLifoDeque::with_limit(3);
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        let result = deque.get(4);
        assert_eq!(result, None);
    }
}#[cfg(test)]
mod tests_llm_16_143 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_get_mut_returns_mut_ref() {
        let mut deque: FixedLifoDeque<u32> = FixedLifoDeque::with_limit(3);
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        let mut_ref = deque.get_mut(1).unwrap();
        *mut_ref = 20;
        assert_eq!(deque[1], 20);
        assert_eq!(deque.get_mut(1), Some(&mut 20));
    }
    
    #[test]
    fn test_get_mut_returns_none() {
        let mut deque: FixedLifoDeque<u32> = FixedLifoDeque::with_limit(3);
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        assert!(deque.get_mut(3).is_none());
    }
}#[cfg(test)]
mod tests_llm_16_144 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_is_empty_empty_queue() {
        let queue: FixedLifoDeque<i32> = FixedLifoDeque::new();
        assert!(queue.is_empty());
    }
    
    #[test]
    fn test_is_empty_non_empty_queue() {
        let mut queue = FixedLifoDeque::new();
        queue.push_back(1);
        assert!(!queue.is_empty());
    }
}#[cfg(test)]
mod tests_llm_16_145 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_iter() {
        let mut deque = FixedLifoDeque::with_limit(5);
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        
        let mut iter = deque.iter();
        
        assert_eq!(iter.next(), Some(&1));
        assert_eq!(iter.next(), Some(&2));
        assert_eq!(iter.next(), Some(&3));
        assert_eq!(iter.next(), None);
    }
}#[cfg(test)]
mod tests_llm_16_146 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_iter_mut() {
        let mut deque: FixedLifoDeque<i32> = FixedLifoDeque::with_limit(3);
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);

        let mut iter = deque.iter_mut();
        assert_eq!(iter.next(), Some(&mut 1));
        assert_eq!(iter.next(), Some(&mut 2));
        assert_eq!(iter.next_back(), Some(&mut 3));
        assert_eq!(iter.next(), None);
    }
}#[cfg(test)]
mod tests_llm_16_147 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_len_empty() {
        let deque: FixedLifoDeque<u32> = FixedLifoDeque::new();
        assert_eq!(deque.len(), 0);
    }
    
    #[test]
    fn test_len_non_empty() {
        let mut deque: FixedLifoDeque<u32> = FixedLifoDeque::new();
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        assert_eq!(deque.len(), 3);
    }
    
    #[test]
    fn test_len_resize() {
        let mut deque: FixedLifoDeque<u32> = FixedLifoDeque::new();
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        deque.resize(2, 0);
        assert_eq!(deque.len(), 2);
    }
    
    #[test]
    fn test_len_resize_limit() {
        let mut deque: FixedLifoDeque<u32> = FixedLifoDeque::with_limit(3);
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        deque.resize(4, 0);
        assert_eq!(deque.len(), 3);
    }
    
    #[test]
    fn test_len_push_back_limit() {
        let mut deque: FixedLifoDeque<u32> = FixedLifoDeque::with_limit(3);
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        deque.push_back(4);
        assert_eq!(deque.len(), 3);
    }
    
    #[test]
    fn test_len_append() {
        let mut deque1: FixedLifoDeque<u32> = FixedLifoDeque::with_limit(3);
        deque1.push_back(1);
        deque1.push_back(2);
        deque1.push_back(3);
        
        let mut deque2: VecDeque<u32> = vec![4, 5, 6].into_iter().collect();
        
        deque1.append(&mut deque2);
        assert_eq!(deque1.len(), 6);
        assert_eq!(deque2.len(), 0);
    }
    
    #[test]
    fn test_len_extend() {
        let mut deque: FixedLifoDeque<u32> = FixedLifoDeque::with_limit(3);
        deque.extend(&[1, 2, 3, 4, 5]);
        assert_eq!(deque.len(), 3);
    }
    
    #[test]
    fn test_len_drop_front() {
        let mut deque: FixedLifoDeque<u32> = FixedLifoDeque::with_limit(3);
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        deque.push_back(4);
        deque.pop_front();
        assert_eq!(deque.len(), 3);
    }
    
    #[test]
    fn test_len_pop_back() {
        let mut deque: FixedLifoDeque<u32> = FixedLifoDeque::with_limit(3);
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        deque.push_back(4);
        deque.pop_back();
        assert_eq!(deque.len(), 3);
    }
}#[cfg(test)]
mod tests_llm_16_148 {
    use super::*;

use crate::*;
    use std::cmp::Ordering;
    
    #[test]
    fn test_limit() {
        let deque: FixedLifoDeque<i32> = FixedLifoDeque::with_limit(3);
        assert_eq!(deque.limit(), 3);
    }
    
    #[test]
    fn test_limit_after_resize() {
        let mut deque: FixedLifoDeque<i32> = FixedLifoDeque::with_limit(3);
        deque.resize(5, 0);
        assert_eq!(deque.limit(), 3);
    }
    
    #[test]
    fn test_limit_after_reset_limit() {
        let mut deque: FixedLifoDeque<i32> = FixedLifoDeque::with_limit(3);
        deque.reset_limit(5);
        assert_eq!(deque.limit(), 5);
    }
    
    #[derive(Debug, PartialEq, Eq)]
    struct Dummy;
    
    #[test]
    fn test_fixed_lifo_deque_eq() {
        let deque1: FixedLifoDeque<Dummy> = FixedLifoDeque::new();
        let deque2: FixedLifoDeque<Dummy> = FixedLifoDeque::new();
        assert_eq!(deque1, deque2);
        
        let deque3: FixedLifoDeque<Dummy> = FixedLifoDeque::with_limit(3);
        assert_ne!(deque1, deque3);
    }
    
    #[test]
    fn test_fixed_lifo_deque_cmp() {
        let deque1: FixedLifoDeque<i32> = FixedLifoDeque::new();
        let deque2: FixedLifoDeque<i32> = FixedLifoDeque::new();
        assert_eq!(deque1.cmp(&deque2), Ordering::Equal);
        
        let deque3: FixedLifoDeque<i32> = FixedLifoDeque::with_limit(3);
        assert_eq!(deque1.cmp(&deque3), Ordering::Less);
    }
    
    #[test]
    fn test_fixed_lifo_deque_extend() {
        let mut deque: FixedLifoDeque<i32> = FixedLifoDeque::new();
        deque.extend(&[1, 2, 3]);
        assert_eq!(deque.len(), 3);
    }
    
    #[test]
    fn test_fixed_lifo_deque_resize() {
        let mut deque: FixedLifoDeque<i32> = FixedLifoDeque::with_limit(3);
        deque.resize(5, 0);
        assert_eq!(deque.len(), 3);
    }
    
    #[test]
    fn test_fixed_lifo_deque_push_back() {
        let mut deque: FixedLifoDeque<i32> = FixedLifoDeque::with_limit(3);
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        deque.push_back(4);
        assert_eq!(deque.len(), 3);
        assert_eq!(deque[0], 2);
        assert_eq!(deque[1], 3);
        assert_eq!(deque[2], 4);
    }
    
    #[test]
    fn test_fixed_lifo_deque_pop_back() {
        let mut deque: FixedLifoDeque<i32> = FixedLifoDeque::with_limit(3);
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        assert_eq!(deque.pop_back(), Some(3));
        assert_eq!(deque.pop_back(), Some(2));
        assert_eq!(deque.pop_back(), Some(1));
        assert_eq!(deque.pop_back(), None);
    }
    
    #[test]
    fn test_fixed_lifo_deque_pop_front() {
        let mut deque: FixedLifoDeque<i32> = FixedLifoDeque::with_limit(3);
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        assert_eq!(deque.pop_front(), Some(1));
        assert_eq!(deque.pop_front(), None);
    }
    
    #[test]
    fn test_fixed_lifo_deque_append() {
        let mut deque1: FixedLifoDeque<i32> = FixedLifoDeque::with_limit(3);
        deque1.push_back(1);
        let mut deque2: VecDeque<i32> = VecDeque::new();
        deque2.push_back(2);
        deque2.push_back(3);
        deque1.append(&mut deque2);
        assert_eq!(deque1.len(), 3);
        assert_eq!(deque1[0], 1);
        assert_eq!(deque1[1], 2);
        assert_eq!(deque1[2], 3);
        assert_eq!(deque2.len(), 0);
    }
    
    #[test]
    fn test_fixed_lifo_deque_remove() {
        let mut deque: FixedLifoDeque<i32> = FixedLifoDeque::with_limit(3);
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        assert_eq!(deque.remove(1), Some(2));
        assert_eq!(deque.remove(0), Some(1));
        assert_eq!(deque.len(), 1);
        assert_eq!(deque[0], 3);
    }
}#[cfg(test)]
mod tests_llm_16_150 {
    use super::*;

use crate::*;
    use std::collections::VecDeque;

    #[test]
    fn test_new() {
        let mut deque: FixedLifoDeque<i32> = FixedLifoDeque::new();
        assert_eq!(deque.limit(), 0);
        assert_eq!(deque.len(), 0);
        assert_eq!(deque.capacity(), 0);
        assert!(deque.is_empty());
        assert_eq!(deque.iter().count(), 0);
        assert_eq!(deque.as_slices().0.len(), 0);
        assert_eq!(deque.as_slices().1.len(), 0);
        assert_eq!(deque.front(), None);
        assert_eq!(deque.front_mut(), None);
        assert_eq!(deque.back(), None);
        assert_eq!(deque.back_mut(), None);
        assert_eq!(deque.pop_front(), None);
        assert_eq!(deque.pop_back(), None);
    }
}#[cfg(test)]
mod tests_llm_16_151 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_pop_back() {
        let mut deque: FixedLifoDeque<u32> = FixedLifoDeque::new();
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        
        assert_eq!(deque.pop_back(), Some(3));
        assert_eq!(deque.pop_back(), Some(2));
        assert_eq!(deque.pop_back(), Some(1));
        assert_eq!(deque.pop_back(), None);
    }
}#[cfg(test)]
mod tests_llm_16_152 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_pop_front_empty() {
        let mut deque: FixedLifoDeque<u32> = FixedLifoDeque::new();
        assert_eq!(deque.pop_front(), None);
    }
    
    #[test]
    fn test_pop_front_single_element() {
        let mut deque: FixedLifoDeque<u32> = FixedLifoDeque::new();
        deque.push_back(1);
        assert_eq!(deque.pop_front(), Some(1));
        assert_eq!(deque.len(), 0);
    }
    
    #[test]
    fn test_pop_front_multiple_elements() {
        let mut deque: FixedLifoDeque<u32> = FixedLifoDeque::new();
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        assert_eq!(deque.pop_front(), Some(1));
        assert_eq!(deque.pop_front(), Some(2));
        assert_eq!(deque.pop_front(), Some(3));
        assert_eq!(deque.len(), 0);
    }
    
    #[test]
    fn test_pop_front_over_limit() {
        let mut deque: FixedLifoDeque<u32> = FixedLifoDeque::with_limit(2);
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        assert_eq!(deque.pop_front(), Some(2));
        assert_eq!(deque.pop_front(), Some(3));
        assert_eq!(deque.len(), 0);
    }
}#[cfg(test)]
mod tests_llm_16_153 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_push_back() {
        let mut deque: FixedLifoDeque<i32> = FixedLifoDeque::with_limit(5);
        
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        deque.push_back(4);
        deque.push_back(5);
        
        assert_eq!(deque.len(), 5);
        assert_eq!(deque.front(), Some(&1));
        assert_eq!(deque.back(), Some(&5));
        
        deque.push_back(6);
        
        assert_eq!(deque.len(), 5);
        assert_eq!(deque.front(), Some(&2));
        assert_eq!(deque.back(), Some(&6));
        
        deque.push_back(7);
        
        assert_eq!(deque.len(), 5);
        assert_eq!(deque.front(), Some(&3));
        assert_eq!(deque.back(), Some(&7));
        
        deque.push_back(8);
        
        assert_eq!(deque.len(), 5);
        assert_eq!(deque.front(), Some(&4));
        assert_eq!(deque.back(), Some(&8));
        
        deque.push_back(9);
        
        assert_eq!(deque.len(), 5);
        assert_eq!(deque.front(), Some(&5));
        assert_eq!(deque.back(), Some(&9));
    }
}#[cfg(test)]
mod tests_llm_16_154 {
    use super::*;

use crate::*;

    #[test]
    fn test_remove() {
        let mut deque: FixedLifoDeque<i32> = FixedLifoDeque::new();
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        deque.push_back(4);
        deque.push_back(5);

        assert_eq!(deque.remove(2), Some(3));
        assert_eq!(deque.remove(0), Some(1));
        assert_eq!(deque.remove(3), Some(5));
        assert_eq!(deque.remove(2), None);

        assert_eq!(deque.len(), 1);
        assert_eq!(deque.front(), Some(&4));
        assert_eq!(deque.back(), Some(&4));
    }
}#[cfg(test)]
mod tests_llm_16_155 {
    use super::*;

use crate::*;
    use std::collections::VecDeque;

    #[test]
    fn test_reset_limit() {
        let mut deque: FixedLifoDeque<u32> = FixedLifoDeque::with_limit(5);
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        assert_eq!(deque.len(), 3);
        deque.reset_limit(2);
        assert_eq!(deque.len(), 2);
        assert_eq!(deque.get(0), Some(&1));
        assert_eq!(deque.get(1), Some(&2));
    }
}#[cfg(test)]
mod tests_llm_16_156 {
    use super::*;

use crate::*;
      
    #[test]
    fn test_resize_smaller() {
        let mut deque: FixedLifoDeque<i32> = FixedLifoDeque::with_limit(5);
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        deque.push_back(4);
        deque.push_back(5);
      
        // Resize to a smaller length
        deque.resize(3, 0);
      
        assert_eq!(deque.len(), 3);
        assert_eq!(deque[0], 1);
        assert_eq!(deque[1], 2);
        assert_eq!(deque[2], 3);
    }
      
    #[test]
    fn test_resize_same() {
        let mut deque: FixedLifoDeque<i32> = FixedLifoDeque::with_limit(5);
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        deque.push_back(4);
        deque.push_back(5);
      
        // Resize to the same length
        deque.resize(5, 0);
      
        assert_eq!(deque.len(), 5);
        assert_eq!(deque[0], 1);
        assert_eq!(deque[1], 2);
        assert_eq!(deque[2], 3);
        assert_eq!(deque[3], 4);
        assert_eq!(deque[4], 5);
    }
      
    #[test]
    fn test_resize_larger_within_limit() {
        let mut deque: FixedLifoDeque<i32> = FixedLifoDeque::with_limit(5);
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        deque.push_back(4);
        deque.push_back(5);
      
        // Resize to a larger length within the limit
        deque.resize(7, 0);
      
        assert_eq!(deque.len(), 7);
        assert_eq!(deque[0], 1);
        assert_eq!(deque[1], 2);
        assert_eq!(deque[2], 3);
        assert_eq!(deque[3], 4);
        assert_eq!(deque[4], 5);
        assert_eq!(deque[5], 0);
        assert_eq!(deque[6], 0);
    }
      
    #[test]
    fn test_resize_larger_exceed_limit() {
        let mut deque: FixedLifoDeque<i32> = FixedLifoDeque::with_limit(5);
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        deque.push_back(4);
        deque.push_back(5);
      
        // Resize to a larger length exceeding the limit
        deque.resize(7, 0);
      
        assert_eq!(deque.len(), 5);
        assert_eq!(deque[0], 3);
        assert_eq!(deque[1], 4);
        assert_eq!(deque[2], 5);
        assert_eq!(deque[3], 0);
        assert_eq!(deque[4], 0);
    }
}#[cfg(test)]
mod tests_llm_16_157 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_retain() {
        let mut deque: FixedLifoDeque<i32> = FixedLifoDeque::new();
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        deque.push_back(4);
        deque.push_back(5);
        
        deque.retain(|x| x % 2 == 0);
        
        assert_eq!(deque.len(), 2);
        assert_eq!(*deque.get(0).unwrap(), 2);
        assert_eq!(*deque.get(1).unwrap(), 4);
    }
}#[cfg(test)]
mod tests_llm_16_158 {
    use super::*;

use crate::*;
    use std::collections::VecDeque;

    #[test]
    fn test_split_off() {
        let mut deque: FixedLifoDeque<i32> = FixedLifoDeque {
            storage: VecDeque::from(vec![1, 2, 3, 4, 5]),
            limit: 5,
        };
        let split_off = deque.split_off(2);
        assert_eq!(deque.len(), 2);
        assert_eq!(deque[0], 1);
        assert_eq!(deque[1], 2);
        assert_eq!(split_off.len(), 3);
        assert_eq!(split_off[0], 3);
        assert_eq!(split_off[1], 4);
        assert_eq!(split_off[2], 5);
    }
}#[cfg(test)]
mod tests_llm_16_159 {
    use super::*;

use crate::*;

    #[test]
    fn test_swap() {
        let mut deque = FixedLifoDeque::new();
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        deque.swap(0, 2);
        assert_eq!(deque[0], 3);
        assert_eq!(deque[2], 1);
    }
}#[cfg(test)]
mod tests_llm_16_160 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_swap_remove_back() {
        let mut deque: FixedLifoDeque<i32> = FixedLifoDeque::new();
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        
        assert_eq!(Some(3), deque.swap_remove_back(2));
        assert_eq!(vec![1, 2], deque.into_iter().collect::<Vec<_>>());
    }
}#[cfg(test)]
mod tests_llm_16_161 {
    use super::*;

use crate::*;
    use std::collections::VecDeque;

    #[test]
    fn test_swap_remove_front() {
        let mut deque: FixedLifoDeque<i32> = FixedLifoDeque::with_limit(5);
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        deque.push_back(4);
        deque.push_back(5);

        assert_eq!(deque.swap_remove_front(2), Some(3));
        assert_eq!(deque.swap_remove_front(3), Some(5));
        assert_eq!(deque.swap_remove_front(0), Some(1));
        assert_eq!(deque.swap_remove_front(1), Some(4));

        assert_eq!(deque.len(), 0);

        let mut deque: FixedLifoDeque<i32> = FixedLifoDeque::with_limit(3);
        deque.push_back(1);
        deque.push_back(2);

        assert_eq!(deque.swap_remove_front(0), Some(1));
        assert_eq!(deque.swap_remove_front(0), Some(2));

        assert_eq!(deque.len(), 0);

        let mut deque: FixedLifoDeque<i32> = FixedLifoDeque::with_limit(2);
        deque.push_back(1);
        deque.push_back(2);

        assert_eq!(deque.swap_remove_front(0), Some(1));
        assert_eq!(deque.swap_remove_front(0), Some(2));

        assert_eq!(deque.len(), 0);

        let mut deque: FixedLifoDeque<i32> = FixedLifoDeque::with_limit(1);
        deque.push_back(1);

        assert_eq!(deque.swap_remove_front(0), Some(1));

        assert_eq!(deque.len(), 0);

        let mut deque: FixedLifoDeque<i32> = FixedLifoDeque::with_limit(0);

        assert_eq!(deque.swap_remove_front(0), None);
        assert_eq!(deque.swap_remove_front(1), None);
    }
}#[cfg(test)]
mod tests_llm_16_162 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_with_limit() {
        let deque: FixedLifoDeque<u32> = FixedLifoDeque::with_limit(4);
        assert_eq!(deque.len(), 0);
        assert_eq!(deque.limit(), 4);
        
        let deque: FixedLifoDeque<u32> = FixedLifoDeque::with_limit(0);
        assert_eq!(deque.len(), 0);
        assert_eq!(deque.limit(), 0);
    }
}
#[cfg(test)]
mod tests_rug_244 {
    use super::*;
    use fixed_lifo_deque::FixedLifoDeque;
    
    #[test]
    fn test_rug() {
        let mut p0: FixedLifoDeque<T> = FixedLifoDeque::new();
        
        <FixedLifoDeque<T>>::as_mut_slices(&mut p0);

    }
}
                            
#[cfg(test)]
mod tests_rug_245 {
    use super::*;
    use fixed_lifo_deque::FixedLifoDeque;

    #[test]
    fn test_rug() {
        let mut p0: FixedLifoDeque<T> = FixedLifoDeque::new();

        fixed_lifo_deque::FixedLifoDeque::<T>::front_mut(&mut p0);
      
    }
}
