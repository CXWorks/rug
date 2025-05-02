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
        let mut p0: LinkedList<i32> = LinkedList::new();
        p0.push_back(10);
        p0.push_back(20);

        crate::util::enumerate(p0);
    }
}#[cfg(test)]
mod tests_rug_507 {
    use super::*;
    use std::os::unix::net::UnixListener;

    #[test]
    fn test_rug() {
        let listener1 = UnixListener::bind("/tmp/socket1").unwrap();
        let listener2 = UnixListener::bind("/tmp/socket2").unwrap();

        let p0: &UnixListener = &listener1;
        let p1: &UnixListener = &listener2;

        crate::util::zip(p0, p1);
    }
}