use crate::imp_prelude::*;
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut, Index, IndexMut};
const CAP: usize = 4;
/// T is usize or isize
#[derive(Debug)]
enum IxDynRepr<T> {
    Inline(u32, [T; CAP]),
    Alloc(Box<[T]>),
}
impl<T> Deref for IxDynRepr<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        match *self {
            IxDynRepr::Inline(len, ref ar) => {
                debug_assert!(len as usize <= ar.len());
                unsafe { ar.get_unchecked(..len as usize) }
            }
            IxDynRepr::Alloc(ref ar) => &*ar,
        }
    }
}
impl<T> DerefMut for IxDynRepr<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        match *self {
            IxDynRepr::Inline(len, ref mut ar) => {
                debug_assert!(len as usize <= ar.len());
                unsafe { ar.get_unchecked_mut(..len as usize) }
            }
            IxDynRepr::Alloc(ref mut ar) => &mut *ar,
        }
    }
}
/// The default is equivalent to `Self::from(&[0])`.
impl Default for IxDynRepr<Ix> {
    fn default() -> Self {
        Self::copy_from(&[0])
    }
}
use num_traits::Zero;
impl<T: Copy + Zero> IxDynRepr<T> {
    pub fn copy_from(x: &[T]) -> Self {
        if x.len() <= CAP {
            let mut arr = [T::zero(); CAP];
            arr[..x.len()].copy_from_slice(&x[..]);
            IxDynRepr::Inline(x.len() as _, arr)
        } else {
            Self::from(x)
        }
    }
}
impl<T: Copy + Zero> IxDynRepr<T> {
    fn from_vec_auto(v: Vec<T>) -> Self {
        if v.len() <= CAP { Self::copy_from(&v) } else { Self::from_vec(v) }
    }
}
impl<T: Copy> IxDynRepr<T> {
    fn from_vec(v: Vec<T>) -> Self {
        IxDynRepr::Alloc(v.into_boxed_slice())
    }
    fn from(x: &[T]) -> Self {
        Self::from_vec(x.to_vec())
    }
}
impl<T: Copy> Clone for IxDynRepr<T> {
    fn clone(&self) -> Self {
        match *self {
            IxDynRepr::Inline(len, arr) => IxDynRepr::Inline(len, arr),
            _ => Self::from(&self[..]),
        }
    }
}
impl<T: Eq> Eq for IxDynRepr<T> {}
impl<T: PartialEq> PartialEq for IxDynRepr<T> {
    fn eq(&self, rhs: &Self) -> bool {
        match (self, rhs) {
            (&IxDynRepr::Inline(slen, ref sarr), &IxDynRepr::Inline(rlen, ref rarr)) => {
                slen == rlen
                    && (0..CAP as usize)
                        .filter(|&i| i < slen as usize)
                        .all(|i| sarr[i] == rarr[i])
            }
            _ => self[..] == rhs[..],
        }
    }
}
impl<T: Hash> Hash for IxDynRepr<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(&self[..], state)
    }
}
/// Dynamic dimension or index type.
///
/// Use `IxDyn` directly. This type implements a dynamic number of
/// dimensions or indices. Short dimensions are stored inline and don't need
/// any dynamic memory allocation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct IxDynImpl(IxDynRepr<Ix>);
impl IxDynImpl {
    pub(crate) fn insert(&self, i: usize) -> Self {
        let len = self.len();
        debug_assert!(i <= len);
        IxDynImpl(
            if len < CAP {
                let mut out = [1; CAP];
                out[0..i].copy_from_slice(&self[0..i]);
                out[i + 1..=len].copy_from_slice(&self[i..len]);
                IxDynRepr::Inline((len + 1) as u32, out)
            } else {
                let mut out = Vec::with_capacity(len + 1);
                out.extend_from_slice(&self[0..i]);
                out.push(1);
                out.extend_from_slice(&self[i..len]);
                IxDynRepr::from_vec(out)
            },
        )
    }
    fn remove(&self, i: usize) -> Self {
        IxDynImpl(
            match self.0 {
                IxDynRepr::Inline(0, _) => IxDynRepr::Inline(0, [0; CAP]),
                IxDynRepr::Inline(1, _) => IxDynRepr::Inline(0, [0; CAP]),
                IxDynRepr::Inline(2, ref arr) => {
                    let mut out = [0; CAP];
                    out[0] = arr[1 - i];
                    IxDynRepr::Inline(1, out)
                }
                ref ixdyn => {
                    let len = ixdyn.len();
                    let mut result = IxDynRepr::copy_from(&ixdyn[..len - 1]);
                    for j in i..len - 1 {
                        result[j] = ixdyn[j + 1];
                    }
                    result
                }
            },
        )
    }
}
impl<'a> From<&'a [Ix]> for IxDynImpl {
    #[inline]
    fn from(ix: &'a [Ix]) -> Self {
        IxDynImpl(IxDynRepr::copy_from(ix))
    }
}
impl From<Vec<Ix>> for IxDynImpl {
    #[inline]
    fn from(ix: Vec<Ix>) -> Self {
        IxDynImpl(IxDynRepr::from_vec_auto(ix))
    }
}
impl<J> Index<J> for IxDynImpl
where
    [Ix]: Index<J>,
{
    type Output = <[Ix] as Index<J>>::Output;
    fn index(&self, index: J) -> &Self::Output {
        &self.0[index]
    }
}
impl<J> IndexMut<J> for IxDynImpl
where
    [Ix]: IndexMut<J>,
{
    fn index_mut(&mut self, index: J) -> &mut Self::Output {
        &mut self.0[index]
    }
}
impl Deref for IxDynImpl {
    type Target = [Ix];
    #[inline]
    fn deref(&self) -> &[Ix] {
        &self.0
    }
}
impl DerefMut for IxDynImpl {
    #[inline]
    fn deref_mut(&mut self) -> &mut [Ix] {
        &mut self.0
    }
}
impl<'a> IntoIterator for &'a IxDynImpl {
    type Item = &'a Ix;
    type IntoIter = <&'a [Ix] as IntoIterator>::IntoIter;
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self[..].iter()
    }
}
impl RemoveAxis for Dim<IxDynImpl> {
    fn remove_axis(&self, axis: Axis) -> Self {
        debug_assert!(axis.index() < self.ndim());
        Dim::new(self.ix().remove(axis.index()))
    }
}
impl IxDyn {
    /// Create a new dimension value with `n` axes, all zeros
    #[inline]
    pub fn zeros(n: usize) -> IxDyn {
        const ZEROS: &[usize] = &[0; 4];
        if n <= ZEROS.len() { Dim(&ZEROS[..n]) } else { Dim(vec![0; n]) }
    }
}
#[cfg(test)]
mod tests_rug_1081 {
    use super::*;
    use crate::dimension::dynindeximpl::IxDynRepr;
    use std::default::Default;
    #[test]
    fn test_default() {
        let _rug_st_tests_rug_1081_rrrruuuugggg_test_default = 0;
        let default_value: IxDynRepr<usize> = <IxDynRepr<usize> as Default>::default();
        let _rug_ed_tests_rug_1081_rrrruuuugggg_test_default = 0;
    }
}
#[cfg(test)]
mod tests_rug_1082 {
    use super::*;
    use crate::dimension::dynindeximpl::IxDynRepr;
    use crate::dimension::dynindeximpl::IxDynRepr::*;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4)) = <(i32, i32, i32, i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: [i32; 5] = [
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
        ];
        IxDynRepr::<i32>::copy_from(&p0);
             }
});    }
}
#[cfg(test)]
mod tests_rug_1083 {
    use super::*;
    use crate::dimension::dynindeximpl::IxDynRepr;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let v51: Vec<i32> = array![rug_fuzz_0, 2, 3].to_vec();
        IxDynRepr::<i32>::from_vec_auto(v51);
             }
});    }
}
#[cfg(test)]
mod tests_rug_1084 {
    use super::*;
    use crate::dimension::dynindeximpl::IxDynRepr;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Vec<i32> = vec![rug_fuzz_0, 2, 3];
        let result = IxDynRepr::<i32>::from_vec(p0);
             }
});    }
}
#[cfg(test)]
mod tests_rug_1089 {
    use super::*;
    use crate::dimension::dynindeximpl::IxDynImpl;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0 = IxDynImpl::default();
        let p1: usize = rug_fuzz_0;
        IxDynImpl::insert(&p0, p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_1090 {
    use super::*;
    use crate::dimension::IxDynImpl;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = IxDynImpl::default();
        let p1: usize = rug_fuzz_0;
        let result = p0.remove(p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_1091 {
    use super::*;
    use crate::dimension::dynindeximpl::IxDynImpl;
    use crate::dimension::Dim;
    #[test]
    fn test_from() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(usize, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: &[usize] = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        <IxDynImpl as std::convert::From<&[usize]>>::from(p0);
             }
});    }
}
#[cfg(test)]
mod tests_rug_1092 {
    use super::*;
    use std::convert::From;
    use crate::dimension::IxDynImpl;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(usize, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Vec<usize> = Vec::new();
        p0.push(rug_fuzz_0);
        p0.push(rug_fuzz_1);
        p0.push(rug_fuzz_2);
        <IxDynImpl as std::convert::From<std::vec::Vec<usize>>>::from(p0);
             }
});    }
}
#[cfg(test)]
mod tests_rug_1095 {
    use super::*;
    use crate::dimension::dynindeximpl::IxDynImpl;
    use std::ops::Deref;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_1095_rrrruuuugggg_test_rug = 0;
        let mut p0 = IxDynImpl::default();
        p0.deref();
        let _rug_ed_tests_rug_1095_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_1096 {
    use super::*;
    use crate::dimension::IxDynImpl;
    use std::ops::DerefMut;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_1096_rrrruuuugggg_test_rug = 0;
        let mut p0 = IxDynImpl::default();
        p0.deref_mut();
        let _rug_ed_tests_rug_1096_rrrruuuugggg_test_rug = 0;
    }
}
