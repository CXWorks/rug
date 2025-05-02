use std::fmt;
use std::ops::Deref;
use std::slice;
/// A sparse set used for representing ordered NFA states.
///
/// This supports constant time addition and membership testing. Clearing an
/// entire set can also be done in constant time. Iteration yields elements
/// in the order in which they were inserted.
///
/// The data structure is based on: http://research.swtch.com/sparse
/// Note though that we don't actually use uninitialized memory. We generally
/// reuse allocations, so the initial allocation cost is bareable. However,
/// its other properties listed above are extremely useful.
#[derive(Clone)]
pub struct SparseSet {
    /// Dense contains the instruction pointers in the order in which they
    /// were inserted.
    dense: Vec<usize>,
    /// Sparse maps instruction pointers to their location in dense.
    ///
    /// An instruction pointer is in the set if and only if
    /// sparse[ip] < dense.len() && ip == dense[sparse[ip]].
    sparse: Box<[usize]>,
}
impl SparseSet {
    pub fn new(size: usize) -> SparseSet {
        SparseSet {
            dense: Vec::with_capacity(size),
            sparse: vec![0; size].into_boxed_slice(),
        }
    }
    pub fn len(&self) -> usize {
        self.dense.len()
    }
    pub fn is_empty(&self) -> bool {
        self.dense.is_empty()
    }
    pub fn capacity(&self) -> usize {
        self.dense.capacity()
    }
    pub fn insert(&mut self, value: usize) {
        let i = self.len();
        assert!(i < self.capacity());
        self.dense.push(value);
        self.sparse[value] = i;
    }
    pub fn contains(&self, value: usize) -> bool {
        let i = self.sparse[value];
        self.dense.get(i) == Some(&value)
    }
    pub fn clear(&mut self) {
        self.dense.clear();
    }
}
impl fmt::Debug for SparseSet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SparseSet({:?})", self.dense)
    }
}
impl Deref for SparseSet {
    type Target = [usize];
    fn deref(&self) -> &Self::Target {
        &self.dense
    }
}
impl<'a> IntoIterator for &'a SparseSet {
    type Item = &'a usize;
    type IntoIter = slice::Iter<'a, usize>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
#[cfg(test)]
mod tests_llm_16_24_llm_16_23 {
    use crate::sparse::SparseSet;
    use std::ops::Deref;
    #[test]
    fn test_into_iter() {
        let _rug_st_tests_llm_16_24_llm_16_23_rrrruuuugggg_test_into_iter = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 5;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 8;
        let mut set = SparseSet::new(rug_fuzz_0);
        set.insert(rug_fuzz_1);
        set.insert(rug_fuzz_2);
        set.insert(rug_fuzz_3);
        let iter = set.into_iter();
        let vec: Vec<usize> = iter.cloned().collect();
        debug_assert_eq!(vec, vec![5, 3, 8]);
        debug_assert_eq!(set.len(), 0);
        let _rug_ed_tests_llm_16_24_llm_16_23_rrrruuuugggg_test_into_iter = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_745 {
    use crate::sparse::SparseSet;
    #[test]
    fn test_capacity() {
        let _rug_st_tests_llm_16_745_rrrruuuugggg_test_capacity = 0;
        let rug_fuzz_0 = 10;
        let set = SparseSet::new(rug_fuzz_0);
        debug_assert_eq!(set.capacity(), 10);
        let _rug_ed_tests_llm_16_745_rrrruuuugggg_test_capacity = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_746 {
    use super::*;
    use crate::*;
    #[test]
    fn test_clear() {
        let _rug_st_tests_llm_16_746_rrrruuuugggg_test_clear = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 3;
        let mut set = SparseSet::new(rug_fuzz_0);
        set.insert(rug_fuzz_1);
        set.insert(rug_fuzz_2);
        set.insert(rug_fuzz_3);
        set.clear();
        debug_assert!(set.is_empty());
        debug_assert_eq!(set.len(), 0);
        let _rug_ed_tests_llm_16_746_rrrruuuugggg_test_clear = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_747 {
    use super::*;
    use crate::*;
    #[test]
    fn test_contains() {
        let _rug_st_tests_llm_16_747_rrrruuuugggg_test_contains = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 3;
        let rug_fuzz_4 = 1;
        let rug_fuzz_5 = 2;
        let rug_fuzz_6 = 3;
        let rug_fuzz_7 = 0;
        let rug_fuzz_8 = 4;
        let mut ss = SparseSet::new(rug_fuzz_0);
        ss.insert(rug_fuzz_1);
        ss.insert(rug_fuzz_2);
        ss.insert(rug_fuzz_3);
        debug_assert!(ss.contains(rug_fuzz_4));
        debug_assert!(ss.contains(rug_fuzz_5));
        debug_assert!(ss.contains(rug_fuzz_6));
        debug_assert!(! ss.contains(rug_fuzz_7));
        debug_assert!(! ss.contains(rug_fuzz_8));
        let _rug_ed_tests_llm_16_747_rrrruuuugggg_test_contains = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_748 {
    use super::*;
    use crate::*;
    #[test]
    #[should_panic]
    fn test_insert_panic() {
        let _rug_st_tests_llm_16_748_rrrruuuugggg_test_insert_panic = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 10;
        let mut set = SparseSet::new(rug_fuzz_0);
        set.insert(rug_fuzz_1);
        set.insert(rug_fuzz_2);
        let _rug_ed_tests_llm_16_748_rrrruuuugggg_test_insert_panic = 0;
    }
    #[test]
    fn test_insert() {
        let _rug_st_tests_llm_16_748_rrrruuuugggg_test_insert = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 2;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 1;
        let rug_fuzz_6 = 2;
        let rug_fuzz_7 = 3;
        let mut set = SparseSet::new(rug_fuzz_0);
        set.insert(rug_fuzz_1);
        set.insert(rug_fuzz_2);
        set.insert(rug_fuzz_3);
        debug_assert_eq!(set.len(), 3);
        debug_assert_eq!(set.capacity(), 10);
        debug_assert!(set.contains(rug_fuzz_4));
        debug_assert!(set.contains(rug_fuzz_5));
        debug_assert!(set.contains(rug_fuzz_6));
        debug_assert!(! set.contains(rug_fuzz_7));
        let _rug_ed_tests_llm_16_748_rrrruuuugggg_test_insert = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_749 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_empty() {
        let _rug_st_tests_llm_16_749_rrrruuuugggg_test_is_empty = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 1;
        let mut set = SparseSet::new(rug_fuzz_0);
        debug_assert_eq!(set.is_empty(), true);
        set.insert(rug_fuzz_1);
        debug_assert_eq!(set.is_empty(), false);
        set.clear();
        debug_assert_eq!(set.is_empty(), true);
        let _rug_ed_tests_llm_16_749_rrrruuuugggg_test_is_empty = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_750 {
    use super::*;
    use crate::*;
    #[test]
    fn test_len() {
        let _rug_st_tests_llm_16_750_rrrruuuugggg_test_len = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 5;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 15;
        let mut set = SparseSet::new(rug_fuzz_0);
        debug_assert_eq!(set.len(), 0);
        set.insert(rug_fuzz_1);
        debug_assert_eq!(set.len(), 1);
        set.insert(rug_fuzz_2);
        debug_assert_eq!(set.len(), 2);
        set.insert(rug_fuzz_3);
        debug_assert_eq!(set.len(), 3);
        set.clear();
        debug_assert_eq!(set.len(), 0);
        let _rug_ed_tests_llm_16_750_rrrruuuugggg_test_len = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_751 {
    use super::*;
    use crate::*;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_751_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = 10;
        let size = rug_fuzz_0;
        let result = SparseSet::new(size);
        debug_assert_eq!(result.len(), 0);
        debug_assert_eq!(result.capacity(), size);
        debug_assert!(result.is_empty());
        let _rug_ed_tests_llm_16_751_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_rug_354 {
    use super::*;
    use crate::sparse::SparseSet;
    use crate::std::ops::Deref;
    #[test]
    fn test_deref() {
        let _rug_st_tests_rug_354_rrrruuuugggg_test_deref = 0;
        let rug_fuzz_0 = 10;
        let mut p0 = SparseSet::new(rug_fuzz_0);
        p0.deref();
        let _rug_ed_tests_rug_354_rrrruuuugggg_test_deref = 0;
    }
}
