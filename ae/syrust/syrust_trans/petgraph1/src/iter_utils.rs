pub trait IterUtilsExt: Iterator {
    /// Return the first element that maps to `Some(_)`, or None if the iterator
    /// was exhausted.
    fn ex_find_map<F, R>(&mut self, mut f: F) -> Option<R>
    where
        F: FnMut(Self::Item) -> Option<R>,
    {
        for elt in self {
            if let result @ Some(_) = f(elt) {
                return result;
            }
        }
        None
    }
    /// Return the last element from the back that maps to `Some(_)`, or
    /// None if the iterator was exhausted.
    fn ex_rfind_map<F, R>(&mut self, mut f: F) -> Option<R>
    where
        F: FnMut(Self::Item) -> Option<R>,
        Self: DoubleEndedIterator,
    {
        while let Some(elt) = self.next_back() {
            if let result @ Some(_) = f(elt) {
                return result;
            }
        }
        None
    }
}
impl<I> IterUtilsExt for I
where
    I: Iterator,
{}
#[cfg(test)]
mod tests_rug_609 {
    use super::*;
    use crate::iter_utils::IterUtilsExt;
    use std::iter::{DoubleEndedIterator, Iterator};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_609_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 2;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 2;
        let mut p0 = Vec::<i32>::new();
        p0.push(rug_fuzz_0);
        p0.push(rug_fuzz_1);
        p0.push(rug_fuzz_2);
        let mut p1 = |x: &i32| {
            if *x % rug_fuzz_3 == rug_fuzz_4 { Some(*x * rug_fuzz_5) } else { None }
        };
        IterUtilsExt::ex_rfind_map(&mut p0.iter(), &mut p1);
        let _rug_ed_tests_rug_609_rrrruuuugggg_test_rug = 0;
    }
}
