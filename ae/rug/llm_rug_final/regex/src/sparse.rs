use std::fmt;
use std::ops::Deref;
use std::slice;

/// A sparse set used for representing ordered NFA states.
///
/// This supports constant time addition and membership testing. Clearing an
/// entire set can also be done in constant time. Iteration yields elements
/// in the order in which they were inserted.
///
/// The data structure is based on: https://research.swtch.com/sparse
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
        mod tests_rug_589 {
            use super::*;
            use crate::sparse::SparseSet;
            
            #[test]
            fn test_rug() {
                let mut p0: usize = 10;

                SparseSet::new(p0);
            }
        }#[cfg(test)]
mod tests_rug_590 {
    use super::*;
    use crate::sparse::SparseSet;

    #[test]
    fn test_rug() {
        let mut p0: SparseSet = SparseSet::new(10);

        <SparseSet>::len(&p0);
    }
}
#[cfg(test)]
mod tests_rug_591 {
    use super::*;
    use crate::sparse::SparseSet;
    
    #[test]
    fn test_rug() {
        let mut p0: SparseSet = SparseSet::new(10);

        <SparseSet>::is_empty(&p0);
    }
}

#[cfg(test)]
mod tests_rug_592 {
    use super::*;
    
    #[test]
    fn test_rug() {
        use crate::sparse::SparseSet;
        
        let mut v35: SparseSet = SparseSet::new(10);
        
        <SparseSet>::capacity(&v35);

    }
}#[cfg(test)]
mod tests_rug_593 {
    use super::*;
    use crate::sparse::SparseSet;
    
    #[test]
    fn test_insert() {
        let mut p0: SparseSet = SparseSet::new(10);
        let p1: usize = 5;

        p0.insert(p1);
    }
}#[cfg(test)]
mod tests_rug_594 {
    use super::*;
    use crate::sparse::SparseSet;
    
    #[test]
    fn test_rug() {
        let mut p0: SparseSet = SparseSet::new(10);
        let p1: usize = 5;
        
        p0.contains(p1);
    }
}
#[cfg(test)]
mod tests_rug_595 {
    use super::*;
    use crate::sparse::SparseSet;

    #[test]
    fn test_rug() {
        let mut p0: SparseSet = SparseSet::new(10);
        <SparseSet>::clear(&mut p0);
    }
}