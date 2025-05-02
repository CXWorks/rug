//! `UnionFind<K>` is a disjoint-set data structure.
use super::graph::IndexType;
use std::cmp::Ordering;
/// `UnionFind<K>` is a disjoint-set data structure. It tracks set membership of *n* elements
/// indexed from *0* to *n - 1*. The scalar type is `K` which must be an unsigned integer type.
///
/// <http://en.wikipedia.org/wiki/Disjoint-set_data_structure>
///
/// Too awesome not to quote:
///
/// “The amortized time per operation is **O(α(n))** where **α(n)** is the
/// inverse of **f(x) = A(x, x)** with **A** being the extremely fast-growing Ackermann function.”
#[derive(Debug, Clone)]
pub struct UnionFind<K> {
    parent: Vec<K>,
    rank: Vec<u8>,
}
#[inline]
unsafe fn get_unchecked<K>(xs: &[K], index: usize) -> &K {
    debug_assert!(index < xs.len());
    xs.get_unchecked(index)
}
#[inline]
unsafe fn get_unchecked_mut<K>(xs: &mut [K], index: usize) -> &mut K {
    debug_assert!(index < xs.len());
    xs.get_unchecked_mut(index)
}
impl<K> UnionFind<K>
where
    K: IndexType,
{
    /// Create a new `UnionFind` of `n` disjoint sets.
    pub fn new(n: usize) -> Self {
        let rank = vec![0; n];
        let parent = (0..n).map(K::new).collect::<Vec<K>>();
        UnionFind { parent, rank }
    }
    /// Return the representative for `x`.
    ///
    /// **Panics** if `x` is out of bounds.
    pub fn find(&self, x: K) -> K {
        assert!(x.index() < self.parent.len());
        unsafe {
            let mut x = x;
            loop {
                let xparent = *get_unchecked(&self.parent, x.index());
                if xparent == x {
                    break;
                }
                x = xparent;
            }
            x
        }
    }
    /// Return the representative for `x`.
    ///
    /// Write back the found representative, flattening the internal
    /// datastructure in the process and quicken future lookups.
    ///
    /// **Panics** if `x` is out of bounds.
    pub fn find_mut(&mut self, x: K) -> K {
        assert!(x.index() < self.parent.len());
        unsafe { self.find_mut_recursive(x) }
    }
    unsafe fn find_mut_recursive(&mut self, mut x: K) -> K {
        let mut parent = *get_unchecked(&self.parent, x.index());
        while parent != x {
            let grandparent = *get_unchecked(&self.parent, parent.index());
            *get_unchecked_mut(&mut self.parent, x.index()) = grandparent;
            x = parent;
            parent = grandparent;
        }
        x
    }
    /// Returns `true` if the given elements belong to the same set, and returns
    /// `false` otherwise.
    pub fn equiv(&self, x: K, y: K) -> bool {
        self.find(x) == self.find(y)
    }
    /// Unify the two sets containing `x` and `y`.
    ///
    /// Return `false` if the sets were already the same, `true` if they were unified.
    ///
    /// **Panics** if `x` or `y` is out of bounds.
    pub fn union(&mut self, x: K, y: K) -> bool {
        if x == y {
            return false;
        }
        let xrep = self.find_mut(x);
        let yrep = self.find_mut(y);
        if xrep == yrep {
            return false;
        }
        let xrepu = xrep.index();
        let yrepu = yrep.index();
        let xrank = self.rank[xrepu];
        let yrank = self.rank[yrepu];
        match xrank.cmp(&yrank) {
            Ordering::Less => self.parent[xrepu] = yrep,
            Ordering::Greater => self.parent[yrepu] = xrep,
            Ordering::Equal => {
                self.parent[yrepu] = xrep;
                self.rank[xrepu] += 1;
            }
        }
        true
    }
    /// Return a vector mapping each element to its representative.
    pub fn into_labeling(mut self) -> Vec<K> {
        unsafe {
            for ix in 0..self.parent.len() {
                let k = *get_unchecked(&self.parent, ix);
                let xrep = self.find_mut_recursive(k);
                *self.parent.get_unchecked_mut(ix) = xrep;
            }
        }
        self.parent
    }
}
#[cfg(test)]
mod tests_rug_497 {
    use super::*;
    use crate::unionfind;
    #[test]
    fn test_get_unchecked() {
        let _rug_st_tests_rug_497_rrrruuuugggg_test_get_unchecked = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 20;
        let rug_fuzz_2 = 30;
        let rug_fuzz_3 = 1;
        let p0: &[i32] = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        let p1: usize = rug_fuzz_3;
        unsafe {
            let result = unionfind::get_unchecked(p0, p1);
            debug_assert_eq!(* result, 20);
        }
        let _rug_ed_tests_rug_497_rrrruuuugggg_test_get_unchecked = 0;
    }
}
#[cfg(test)]
mod tests_rug_498 {
    use super::*;
    use crate::unionfind;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_498_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let mut p0: Vec<i32> = vec![rug_fuzz_0, 2, 3, 4, 5];
        let index = rug_fuzz_1;
        unsafe {
            crate::unionfind::get_unchecked_mut(&mut p0, index);
        };
        let _rug_ed_tests_rug_498_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_500 {
    use super::*;
    use crate::unionfind::UnionFind;
    use crate::graph::IndexType;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_500_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 3;
        let mut p0: UnionFind<usize> = UnionFind::new(rug_fuzz_0);
        let mut p1: usize = rug_fuzz_1;
        p0.find(p1);
        let _rug_ed_tests_rug_500_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_501 {
    use super::*;
    use crate::unionfind::UnionFind;
    use crate::graph::IndexType;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_501_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 5;
        let mut p0: UnionFind<u32> = UnionFind::new(rug_fuzz_0);
        let p1: u32 = rug_fuzz_1;
        p0.find_mut(p1);
        let _rug_ed_tests_rug_501_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_503 {
    use super::*;
    use crate::graph::IndexType;
    use crate::unionfind::UnionFind;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_503_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 2;
        let mut p0: UnionFind<usize> = UnionFind::new(rug_fuzz_0);
        let mut p1: usize = rug_fuzz_1;
        let mut p2: usize = rug_fuzz_2;
        debug_assert_eq!(p0.equiv(p1, p2), false);
        let _rug_ed_tests_rug_503_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_504 {
    use super::*;
    use crate::unionfind;
    use crate::graph;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_504_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 1;
        let mut p0 = unionfind::UnionFind::<usize>::new(rug_fuzz_0);
        let mut p1 = unionfind::UnionFind::<usize>::new(rug_fuzz_1);
        let mut p2 = unionfind::UnionFind::<usize>::new(rug_fuzz_2);
        debug_assert_eq!(p0.union(rug_fuzz_3, rug_fuzz_4), true);
        let _rug_ed_tests_rug_504_rrrruuuugggg_test_rug = 0;
    }
}
