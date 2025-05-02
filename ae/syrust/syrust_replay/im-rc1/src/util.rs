use std::cmp::Ordering;
use std::ops::{Bound, IndexMut, Range, RangeBounds};
use std::ptr;
#[cfg(feature = "pool")]
pub(crate) use refpool::{PoolClone, PoolDefault};
#[cfg(all(threadsafe))]
pub(crate) use crate::fakepool::{Arc as PoolRef, Pool, PoolClone, PoolDefault};
#[cfg(threadsafe)]
pub(crate) type Ref<A> = std::sync::Arc<A>;
#[cfg(all(not(threadsafe), not(feature = "pool")))]
pub(crate) use crate::fakepool::{Pool, PoolClone, PoolDefault, Rc as PoolRef};
#[cfg(all(not(threadsafe), feature = "pool"))]
pub(crate) type PoolRef<A> = refpool::PoolRef<A>;
#[cfg(all(not(threadsafe), feature = "pool"))]
pub(crate) type Pool<A> = refpool::Pool<A>;
#[cfg(not(threadsafe))]
pub(crate) type Ref<A> = std::rc::Rc<A>;
pub(crate) fn clone_ref<A>(r: Ref<A>) -> A
where
    A: Clone,
{
    Ref::try_unwrap(r).unwrap_or_else(|r| (*r).clone())
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Side {
    Left,
    Right,
}
/// Swap two values of anything implementing `IndexMut`.
///
/// Like `slice::swap`, but more generic.
#[allow(unsafe_code)]
pub(crate) fn swap_indices<V>(vector: &mut V, a: usize, b: usize)
where
    V: IndexMut<usize>,
    V::Output: Sized,
{
    if a == b {
        return;
    }
    let pa: *mut V::Output = &mut vector[a];
    let pb: *mut V::Output = &mut vector[b];
    unsafe {
        ptr::swap(pa, pb);
    }
}
#[allow(dead_code)]
pub(crate) fn linear_search_by<'a, A, I, F>(
    iterable: I,
    mut cmp: F,
) -> Result<usize, usize>
where
    A: 'a,
    I: IntoIterator<Item = &'a A>,
    F: FnMut(&A) -> Ordering,
{
    let mut pos = 0;
    for value in iterable {
        match cmp(value) {
            Ordering::Equal => return Ok(pos),
            Ordering::Greater => return Err(pos),
            Ordering::Less => {}
        }
        pos += 1;
    }
    Err(pos)
}
pub(crate) fn to_range<R>(range: &R, right_unbounded: usize) -> Range<usize>
where
    R: RangeBounds<usize>,
{
    let start_index = match range.start_bound() {
        Bound::Included(i) => *i,
        Bound::Excluded(i) => *i + 1,
        Bound::Unbounded => 0,
    };
    let end_index = match range.end_bound() {
        Bound::Included(i) => *i + 1,
        Bound::Excluded(i) => *i,
        Bound::Unbounded => right_unbounded,
    };
    start_index..end_index
}
macro_rules! def_pool {
    ($name:ident <$($arg:tt),*>, $pooltype:ty) => {
        #[doc = " A memory pool for the appropriate node type."] pub struct $name
        <$($arg,)*> (Pool <$pooltype >); impl <$($arg,)*> $name <$($arg,)*> { #[doc =
        " Create a new pool with the given size."] pub fn new(size : usize) -> Self {
        Self(Pool::new(size)) } #[doc = " Fill the pool with preallocated chunks."] pub
        fn fill(& self) { self.0.fill(); } #[doc = "Get the current size of the pool."]
        pub fn pool_size(& self) -> usize { self.0.get_pool_size() } } impl <$($arg,)*>
        Default for $name <$($arg,)*> { fn default() -> Self { Self::new($crate
        ::config::POOL_SIZE) } } impl <$($arg,)*> Clone for $name <$($arg,)*> { fn
        clone(& self) -> Self { Self(self.0.clone()) } }
    };
}
#[cfg(test)]
mod tests_rug_41 {
    use super::*;
    use crate::util::linear_search_by;
    use std::cmp::Ordering;
    use std::collections::HashSet;
    use std::collections::hash_map::RandomState;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_41_rrrruuuugggg_sample = 0;
        #[cfg(test)]
        mod tests_rug_41_prepare {
            use crate::hash::set::HashSet;
            use std::collections::hash_map::RandomState;
            #[test]
            fn sample() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

                let _rug_st_tests_rug_41_rrrruuuugggg_sample = rug_fuzz_0;
                let v2 = RandomState::new();
                let mut v8: HashSet<i32> = HashSet::new();
                let _rug_ed_tests_rug_41_rrrruuuugggg_sample = rug_fuzz_1;
             }
}
}
}            }
        }
        let mut p0 = HashSet::<i32>::new();
        p0.insert(1);
        let p1 = |x: &i32| match x.cmp(&1) {
            Ordering::Equal => Ordering::Equal,
            Ordering::Greater => Ordering::Greater,
            Ordering::Less => Ordering::Less,
        };
        assert_eq!(linear_search_by(& p0, p1), Ok(1));
        let _rug_ed_tests_rug_41_rrrruuuugggg_sample = 0;
    }
}
#[cfg(test)]
mod tests_rug_42 {
    use super::*;
    use std::ops::{Bound, RangeBounds};
    #[test]
    fn test_im_to_range() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(usize, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: std::ops::RangeInclusive<&usize> = (&rug_fuzz_0..=&rug_fuzz_1);
        let p1: usize = rug_fuzz_2;
        let result = crate::util::to_range(&p0, p1);
        debug_assert_eq!(result, 0..11);
             }
}
}
}    }
}
