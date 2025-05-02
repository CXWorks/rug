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

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5)) = <(i32, i32, i32, i32, i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Vec::<i32>::new();
        p0.push(rug_fuzz_0);
        p0.push(rug_fuzz_1);
        p0.push(rug_fuzz_2);
        let mut p1 = |x: &i32| {
            if *x % rug_fuzz_3 == rug_fuzz_4 { Some(*x * rug_fuzz_5) } else { None }
        };
        IterUtilsExt::ex_rfind_map(&mut p0.iter(), &mut p1);
             }
}
}
}    }
}
