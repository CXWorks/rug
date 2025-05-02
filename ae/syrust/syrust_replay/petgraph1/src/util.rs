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

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: LinkedList<i32> = LinkedList::new();
        p0.push_back(rug_fuzz_0);
        p0.push_back(rug_fuzz_1);
        crate::util::enumerate(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_507 {
    use super::*;
    use std::os::unix::net::UnixListener;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(&str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let listener1 = UnixListener::bind(rug_fuzz_0).unwrap();
        let listener2 = UnixListener::bind(rug_fuzz_1).unwrap();
        let p0: &UnixListener = &listener1;
        let p1: &UnixListener = &listener2;
        crate::util::zip(p0, p1);
             }
}
}
}    }
}
