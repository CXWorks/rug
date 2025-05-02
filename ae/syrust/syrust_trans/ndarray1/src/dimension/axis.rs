/// An axis index.
///
/// An axis one of an array’s “dimensions”; an *n*-dimensional array has *n* axes.
/// Axis *0* is the array’s outermost axis and *n*-1 is the innermost.
///
/// All array axis arguments use this type to make the code easier to write
/// correctly and easier to understand.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Axis(pub usize);
impl Axis {
    /// Return the index of the axis.
    #[inline(always)]
    pub fn index(self) -> usize {
        self.0
    }
}
#[cfg(test)]
mod tests_rug_1078 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_1078_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 2;
        let mut p0 = Axis(rug_fuzz_0);
        debug_assert_eq!(p0.index(), 2);
        let _rug_ed_tests_rug_1078_rrrruuuugggg_test_rug = 0;
    }
}
