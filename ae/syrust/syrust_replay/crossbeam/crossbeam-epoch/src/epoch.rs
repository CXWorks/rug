//! The global epoch
//!
//! The last bit in this number is unused and is always zero. Every so often the global epoch is
//! incremented, i.e. we say it "advances". A pinned participant may advance the global epoch only
//! if all currently pinned participants have been pinned in the current epoch.
//!
//! If an object became garbage in some epoch, then we can be sure that after two advancements no
//! participant will hold a reference to it. That is the crux of safe memory reclamation.
use core::sync::atomic::{AtomicUsize, Ordering};
/// An epoch that can be marked as pinned or unpinned.
///
/// Internally, the epoch is represented as an integer that wraps around at some unspecified point
/// and a flag that represents whether it is pinned or unpinned.
#[derive(Copy, Clone, Default, Debug, Eq, PartialEq)]
pub struct Epoch {
    /// The least significant bit is set if pinned. The rest of the bits hold the epoch.
    data: usize,
}
impl Epoch {
    /// Returns the starting epoch in unpinned state.
    #[inline]
    pub fn starting() -> Self {
        Self::default()
    }
    /// Returns the number of epochs `self` is ahead of `rhs`.
    ///
    /// Internally, epochs are represented as numbers in the range `(isize::MIN / 2) .. (isize::MAX
    /// / 2)`, so the returned distance will be in the same interval.
    pub fn wrapping_sub(self, rhs: Self) -> isize {
        self.data.wrapping_sub(rhs.data & !1) as isize >> 1
    }
    /// Returns `true` if the epoch is marked as pinned.
    #[inline]
    pub fn is_pinned(self) -> bool {
        (self.data & 1) == 1
    }
    /// Returns the same epoch, but marked as pinned.
    #[inline]
    pub fn pinned(self) -> Epoch {
        Epoch { data: self.data | 1 }
    }
    /// Returns the same epoch, but marked as unpinned.
    #[inline]
    pub fn unpinned(self) -> Epoch {
        Epoch { data: self.data & !1 }
    }
    /// Returns the successor epoch.
    ///
    /// The returned epoch will be marked as pinned only if the previous one was as well.
    #[inline]
    pub fn successor(self) -> Epoch {
        Epoch {
            data: self.data.wrapping_add(2),
        }
    }
}
/// An atomic value that holds an `Epoch`.
#[derive(Default, Debug)]
pub struct AtomicEpoch {
    /// Since `Epoch` is just a wrapper around `usize`, an `AtomicEpoch` is similarly represented
    /// using an `AtomicUsize`.
    data: AtomicUsize,
}
impl AtomicEpoch {
    /// Creates a new atomic epoch.
    #[inline]
    pub fn new(epoch: Epoch) -> Self {
        let data = AtomicUsize::new(epoch.data);
        AtomicEpoch { data }
    }
    /// Loads a value from the atomic epoch.
    #[inline]
    pub fn load(&self, ord: Ordering) -> Epoch {
        Epoch { data: self.data.load(ord) }
    }
    /// Stores a value into the atomic epoch.
    #[inline]
    pub fn store(&self, epoch: Epoch, ord: Ordering) {
        self.data.store(epoch.data, ord);
    }
    /// Stores a value into the atomic epoch if the current value is the same as `current`.
    ///
    /// The return value is always the previous value. If it is equal to `current`, then the value
    /// is updated.
    ///
    /// The `Ordering` argument describes the memory ordering of this operation.
    #[inline]
    pub fn compare_and_swap(&self, current: Epoch, new: Epoch, ord: Ordering) -> Epoch {
        let data = self.data.compare_and_swap(current.data, new.data, ord);
        Epoch { data }
    }
}
#[cfg(test)]
mod tests_rug_478 {
    use super::*;
    use crate::epoch;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_478_rrrruuuugggg_test_rug = 0;
        epoch::Epoch::starting();
        let _rug_ed_tests_rug_478_rrrruuuugggg_test_rug = 0;
    }
}
use crate::epoch;
#[cfg(test)]
mod tests_rug_479 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_479_rrrruuuugggg_test_rug = 0;
        let mut p0 = epoch::Epoch::starting();
        let mut p1 = epoch::Epoch::starting();
        p0.wrapping_sub(p1);
        let _rug_ed_tests_rug_479_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_480 {
    use super::*;
    use crate::epoch;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_480_rrrruuuugggg_test_rug = 0;
        let mut p0 = epoch::Epoch::starting();
        debug_assert_eq!(p0.is_pinned(), false);
        let _rug_ed_tests_rug_480_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_481 {
    use super::*;
    use crate::epoch;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_481_rrrruuuugggg_test_rug = 0;
        let mut p0 = epoch::Epoch::starting();
        epoch::Epoch::pinned(p0);
        let _rug_ed_tests_rug_481_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_482 {
    use super::*;
    use crate::epoch;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_482_rrrruuuugggg_test_rug = 0;
        let mut p0 = epoch::Epoch::starting();
        epoch::Epoch::unpinned(p0);
        let _rug_ed_tests_rug_482_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_483 {
    use super::*;
    use crate::epoch;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_483_rrrruuuugggg_test_rug = 0;
        let mut p0 = epoch::Epoch::starting();
        p0.successor();
        let _rug_ed_tests_rug_483_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_484 {
    use super::*;
    use crate::epoch;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_484_rrrruuuugggg_test_rug = 0;
        let mut p0 = epoch::Epoch::starting();
        <epoch::AtomicEpoch>::new(p0);
        let _rug_ed_tests_rug_484_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_486 {
    use super::*;
    use crate::epoch::{self, AtomicEpoch};
    use std::sync::atomic::Ordering;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_486_rrrruuuugggg_test_rug = 0;
        let mut p0 = AtomicEpoch::default();
        let mut p1 = epoch::Epoch::starting();
        let p2 = Ordering::Relaxed;
        p0.store(p1, p2);
        let _rug_ed_tests_rug_486_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_487 {
    use super::*;
    use crate::epoch;
    use std::sync::atomic::Ordering;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = epoch::AtomicEpoch::default();
        let mut p1 = epoch::Epoch { data: rug_fuzz_0 };
        let mut p2 = epoch::Epoch { data: rug_fuzz_1 };
        let p3 = Ordering::Relaxed;
        p0.compare_and_swap(p1, p2, p3);
             }
}
}
}    }
}
