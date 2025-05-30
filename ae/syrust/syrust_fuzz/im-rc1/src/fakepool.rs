#![allow(dead_code)]
use std::marker::PhantomData;
use std::ops::Deref;
use std::rc::Rc as RRc;
use std::sync::Arc as RArc;
use crate::nodes::chunk::Chunk;
pub(crate) trait PoolDefault: Default {}
pub(crate) trait PoolClone: Clone {}
impl<A> PoolDefault for Chunk<A> {}
impl<A> PoolClone for Chunk<A>
where
    A: Clone,
{}
pub(crate) struct Pool<A>(PhantomData<A>);
impl<A> Pool<A> {
    pub(crate) fn new(_size: usize) -> Self {
        Pool(PhantomData)
    }
    pub(crate) fn get_pool_size(&self) -> usize {
        0
    }
    pub(crate) fn fill(&self) {}
}
impl<A> Clone for Pool<A> {
    fn clone(&self) -> Self {
        Self::new(0)
    }
}
#[derive(Default)]
pub(crate) struct Rc<A>(RRc<A>);
impl<A> Rc<A> {
    #[inline(always)]
    pub(crate) fn default(_pool: &Pool<A>) -> Self
    where
        A: PoolDefault,
    {
        Self(Default::default())
    }
    #[inline(always)]
    pub(crate) fn new(_pool: &Pool<A>, value: A) -> Self {
        Rc(RRc::new(value))
    }
    #[inline(always)]
    pub(crate) fn clone_from(_pool: &Pool<A>, value: &A) -> Self
    where
        A: PoolClone,
    {
        Rc(RRc::new(value.clone()))
    }
    #[inline(always)]
    pub(crate) fn make_mut<'a>(_pool: &Pool<A>, this: &'a mut Self) -> &'a mut A
    where
        A: PoolClone,
    {
        RRc::make_mut(&mut this.0)
    }
    #[inline(always)]
    pub(crate) fn ptr_eq(left: &Self, right: &Self) -> bool {
        RRc::ptr_eq(&left.0, &right.0)
    }
    pub(crate) fn unwrap_or_clone(this: Self) -> A
    where
        A: PoolClone,
    {
        RRc::try_unwrap(this.0).unwrap_or_else(|r| (*r).clone())
    }
}
impl<A> Clone for Rc<A> {
    #[inline(always)]
    fn clone(&self) -> Self {
        Rc(self.0.clone())
    }
}
impl<A> Deref for Rc<A> {
    type Target = A;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}
impl<A> PartialEq for Rc<A>
where
    A: PartialEq,
{
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}
impl<A> Eq for Rc<A>
where
    A: Eq,
{}
impl<A> std::fmt::Debug for Rc<A>
where
    A: std::fmt::Debug,
{
    #[inline(always)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.0.fmt(f)
    }
}
#[derive(Default)]
pub(crate) struct Arc<A>(RArc<A>);
impl<A> Arc<A> {
    #[inline(always)]
    pub(crate) fn default(_pool: &Pool<A>) -> Self
    where
        A: PoolDefault,
    {
        Self(Default::default())
    }
    #[inline(always)]
    pub(crate) fn new(_pool: &Pool<A>, value: A) -> Self {
        Self(RArc::new(value))
    }
    #[inline(always)]
    pub(crate) fn clone_from(_pool: &Pool<A>, value: &A) -> Self
    where
        A: PoolClone,
    {
        Self(RArc::new(value.clone()))
    }
    #[inline(always)]
    pub(crate) fn make_mut<'a>(_pool: &Pool<A>, this: &'a mut Self) -> &'a mut A
    where
        A: PoolClone,
    {
        RArc::make_mut(&mut this.0)
    }
    #[inline(always)]
    pub(crate) fn ptr_eq(left: &Self, right: &Self) -> bool {
        RArc::ptr_eq(&left.0, &right.0)
    }
    pub(crate) fn unwrap_or_clone(this: Self) -> A
    where
        A: PoolClone,
    {
        RArc::try_unwrap(this.0).unwrap_or_else(|r| (*r).clone())
    }
}
impl<A> Clone for Arc<A> {
    #[inline(always)]
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<A> Deref for Arc<A> {
    type Target = A;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}
impl<A> PartialEq for Arc<A>
where
    A: PartialEq,
{
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}
impl<A> Eq for Arc<A>
where
    A: Eq,
{}
impl<A> std::fmt::Debug for Arc<A>
where
    A: std::fmt::Debug,
{
    #[inline(always)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.0.fmt(f)
    }
}
#[cfg(test)]
mod tests_rug_683 {
    use super::*;
    use crate::fakepool::Pool;
    use std::marker::PhantomData;
    struct A;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let size: usize = rug_fuzz_0;
        let p0: usize = size;
        Pool::<A>::new(p0);
             }
});    }
}
