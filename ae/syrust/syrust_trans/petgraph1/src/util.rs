use std::iter;
pub fn enumerate<I>(iterable: I) -> iter::Enumerate<I::IntoIter>
where
    I: IntoIterator,
{
    iterable.into_iter().enumerate()
}
#[cfg(feature = "serde-1")]
pub fn rev<I>(iterable: I) -> iter::Rev<I::IntoIter>
where
    I: IntoIterator,
    I::IntoIter: DoubleEndedIterator,
{
    iterable.into_iter().rev()
}
pub fn zip<I, J>(i: I, j: J) -> iter::Zip<I::IntoIter, J::IntoIter>
where
    I: IntoIterator,
    J: IntoIterator,
{
    i.into_iter().zip(j)
}
#[cfg(test)]
mod tests_rug_506 {
    use std::collections::LinkedList;
    use crate::prelude::*;
    use crate::algo::is_isomorphic;
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_506_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 20;
        let mut p0: LinkedList<i32> = LinkedList::new();
        p0.push_back(rug_fuzz_0);
        p0.push_back(rug_fuzz_1);
        crate::util::enumerate(p0);
        let _rug_ed_tests_rug_506_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_507 {
    use super::*;
    use std::os::unix::net::UnixListener;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_507_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "/tmp/socket1";
        let rug_fuzz_1 = "/tmp/socket2";
        let listener1 = UnixListener::bind(rug_fuzz_0).unwrap();
        let listener2 = UnixListener::bind(rug_fuzz_1).unwrap();
        let p0: &UnixListener = &listener1;
        let p1: &UnixListener = &listener2;
        crate::util::zip(p0, p1);
        let _rug_ed_tests_rug_507_rrrruuuugggg_test_rug = 0;
    }
}
