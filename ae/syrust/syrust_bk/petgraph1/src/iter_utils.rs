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

impl<I> IterUtilsExt for I where I: Iterator {}
#[cfg(test)]
mod tests_rug_609 {
    use super::*;
    use crate::iter_utils::IterUtilsExt;
    use std::iter::{DoubleEndedIterator, Iterator};

    #[test]
    fn test_rug() {
        let mut p0 = Vec::<i32>::new();
        p0.push(1);
        p0.push(2);
        p0.push(3);

        let mut p1 = |x: &i32| {
            if *x % 2 == 0 {
                Some(*x * 2)
            } else {
                None
            }
        };

        IterUtilsExt::ex_rfind_map(&mut p0.iter(), &mut p1);
    }
}