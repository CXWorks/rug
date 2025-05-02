use std::mem;
use std::mem::ManuallyDrop;
use std::ptr::NonNull;
use std::slice;
use crate::extension::nonnull;
/// Array's representation.
///
/// *Don’t use this type directly—use the type alias
/// [`Array`](type.Array.html) for the array type!*
#[derive(Debug)]
pub struct OwnedRepr<A> {
    ptr: NonNull<A>,
    len: usize,
    capacity: usize,
}
impl<A> OwnedRepr<A> {
    pub(crate) fn from(v: Vec<A>) -> Self {
        let mut v = ManuallyDrop::new(v);
        let len = v.len();
        let capacity = v.capacity();
        let ptr = nonnull::nonnull_from_vec_data(&mut v);
        Self { ptr, len, capacity }
    }
    pub(crate) fn into_vec(self) -> Vec<A> {
        ManuallyDrop::new(self).take_as_vec()
    }
    pub(crate) fn as_slice(&self) -> &[A] {
        unsafe { slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }
    pub(crate) fn len(&self) -> usize {
        self.len
    }
    pub(crate) fn as_ptr(&self) -> *const A {
        self.ptr.as_ptr()
    }
    pub(crate) fn as_nonnull_mut(&mut self) -> NonNull<A> {
        self.ptr
    }
    fn take_as_vec(&mut self) -> Vec<A> {
        let capacity = self.capacity;
        let len = self.len;
        self.len = 0;
        self.capacity = 0;
        unsafe { Vec::from_raw_parts(self.ptr.as_ptr(), len, capacity) }
    }
}
impl<A> Clone for OwnedRepr<A>
where
    A: Clone,
{
    fn clone(&self) -> Self {
        Self::from(self.as_slice().to_owned())
    }
    fn clone_from(&mut self, other: &Self) {
        let mut v = self.take_as_vec();
        let other = other.as_slice();
        if v.len() > other.len() {
            v.truncate(other.len());
        }
        let (front, back) = other.split_at(v.len());
        v.clone_from_slice(front);
        v.extend_from_slice(back);
        *self = Self::from(v);
    }
}
impl<A> Drop for OwnedRepr<A> {
    fn drop(&mut self) {
        if self.capacity > 0 {
            if !mem::needs_drop::<A>() {
                self.len = 0;
            }
            self.take_as_vec();
        }
    }
}
unsafe impl<A> Sync for OwnedRepr<A>
where
    A: Sync,
{}
unsafe impl<A> Send for OwnedRepr<A>
where
    A: Send,
{}
#[cfg(test)]
mod tests_rug_989 {
    use super::*;
    use crate::data_repr::OwnedRepr;
    use std::{ptr, mem::{ManuallyDrop, MaybeUninit}};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        #[derive(Debug)]
        struct A {
            data: i32,
        }
        let v21: Vec<A> = vec![
            A { data : rug_fuzz_0 }, A { data : 10 }, A { data : 15 }
        ];
        let mut p0 = v21;
        OwnedRepr::<A>::from(p0);
             }
}
}
}    }
}
