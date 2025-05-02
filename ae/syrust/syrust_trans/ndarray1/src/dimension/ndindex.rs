use std::fmt::Debug;
use super::{stride_offset, stride_offset_checked};
use crate::itertools::zip;
use crate::{
    Dim, Dimension, IntoDimension, Ix, Ix0, Ix1, Ix2, Ix3, Ix4, Ix5, Ix6, IxDyn,
    IxDynImpl,
};
/// Tuple or fixed size arrays that can be used to index an array.
///
/// ```
/// use ndarray::arr2;
///
/// let mut a = arr2(&[[0, 1],
///                    [2, 3]]);
/// assert_eq!(a[[0, 1]], 1);
/// assert_eq!(a[[1, 1]], 3);
/// a[[1, 1]] += 1;
/// assert_eq!(a[(1, 1)], 4);
/// ```
pub unsafe trait NdIndex<E>: Debug {
    #[doc(hidden)]
    fn index_checked(&self, dim: &E, strides: &E) -> Option<isize>;
    #[doc(hidden)]
    fn index_unchecked(&self, strides: &E) -> isize;
}
unsafe impl<D> NdIndex<D> for D
where
    D: Dimension,
{
    fn index_checked(&self, dim: &D, strides: &D) -> Option<isize> {
        dim.stride_offset_checked(strides, self)
    }
    fn index_unchecked(&self, strides: &D) -> isize {
        D::stride_offset(self, strides)
    }
}
unsafe impl NdIndex<Ix0> for () {
    #[inline]
    fn index_checked(&self, dim: &Ix0, strides: &Ix0) -> Option<isize> {
        dim.stride_offset_checked(strides, &Ix0())
    }
    #[inline(always)]
    fn index_unchecked(&self, _strides: &Ix0) -> isize {
        0
    }
}
unsafe impl NdIndex<Ix2> for (Ix, Ix) {
    #[inline]
    fn index_checked(&self, dim: &Ix2, strides: &Ix2) -> Option<isize> {
        dim.stride_offset_checked(strides, &Ix2(self.0, self.1))
    }
    #[inline]
    fn index_unchecked(&self, strides: &Ix2) -> isize {
        stride_offset(self.0, get!(strides, 0)) + stride_offset(self.1, get!(strides, 1))
    }
}
unsafe impl NdIndex<Ix3> for (Ix, Ix, Ix) {
    #[inline]
    fn index_checked(&self, dim: &Ix3, strides: &Ix3) -> Option<isize> {
        dim.stride_offset_checked(strides, &self.into_dimension())
    }
    #[inline]
    fn index_unchecked(&self, strides: &Ix3) -> isize {
        stride_offset(self.0, get!(strides, 0)) + stride_offset(self.1, get!(strides, 1))
            + stride_offset(self.2, get!(strides, 2))
    }
}
unsafe impl NdIndex<Ix4> for (Ix, Ix, Ix, Ix) {
    #[inline]
    fn index_checked(&self, dim: &Ix4, strides: &Ix4) -> Option<isize> {
        dim.stride_offset_checked(strides, &self.into_dimension())
    }
    #[inline]
    fn index_unchecked(&self, strides: &Ix4) -> isize {
        zip(strides.ix(), self.into_dimension().ix())
            .map(|(&s, &i)| stride_offset(i, s))
            .sum()
    }
}
unsafe impl NdIndex<Ix5> for (Ix, Ix, Ix, Ix, Ix) {
    #[inline]
    fn index_checked(&self, dim: &Ix5, strides: &Ix5) -> Option<isize> {
        dim.stride_offset_checked(strides, &self.into_dimension())
    }
    #[inline]
    fn index_unchecked(&self, strides: &Ix5) -> isize {
        zip(strides.ix(), self.into_dimension().ix())
            .map(|(&s, &i)| stride_offset(i, s))
            .sum()
    }
}
unsafe impl NdIndex<Ix1> for Ix {
    #[inline]
    fn index_checked(&self, dim: &Ix1, strides: &Ix1) -> Option<isize> {
        dim.stride_offset_checked(strides, &Ix1(*self))
    }
    #[inline(always)]
    fn index_unchecked(&self, strides: &Ix1) -> isize {
        stride_offset(*self, get!(strides, 0))
    }
}
unsafe impl NdIndex<IxDyn> for Ix {
    #[inline]
    fn index_checked(&self, dim: &IxDyn, strides: &IxDyn) -> Option<isize> {
        debug_assert_eq!(dim.ndim(), 1);
        stride_offset_checked(dim.ix(), strides.ix(), &[*self])
    }
    #[inline(always)]
    fn index_unchecked(&self, strides: &IxDyn) -> isize {
        debug_assert_eq!(strides.ndim(), 1);
        stride_offset(*self, get!(strides, 0))
    }
}
macro_rules! ndindex_with_array {
    ($([$n:expr, $ix_n:ident $($index:tt)*])+) => {
        $(unsafe impl NdIndex <$ix_n > for [Ix; $n] { #[inline] fn index_checked(& self,
        dim : &$ix_n, strides : &$ix_n) -> Option < isize > { dim
        .stride_offset_checked(strides, & self.into_dimension()) } #[inline] fn
        index_unchecked(& self, _strides : &$ix_n) -> isize {
        $(stride_offset(self[$index], get!(_strides, $index)) +)* 0 } } unsafe impl
        NdIndex < IxDyn > for Dim < [Ix; $n] > { #[inline] fn index_checked(& self, dim :
        & IxDyn, strides : & IxDyn) -> Option < isize > { debug_assert_eq!(strides
        .ndim(), $n, "Attempted to index with {:?} in array with {} axes", self, strides
        .ndim()); stride_offset_checked(dim.ix(), strides.ix(), self.ix()) } #[inline] fn
        index_unchecked(& self, strides : & IxDyn) -> isize { debug_assert_eq!(strides
        .ndim(), $n, "Attempted to index with {:?} in array with {} axes", self, strides
        .ndim()); $(stride_offset(get!(self, $index), get!(strides, $index)) +)* 0 } }
        unsafe impl NdIndex < IxDyn > for [Ix; $n] { #[inline] fn index_checked(& self,
        dim : & IxDyn, strides : & IxDyn) -> Option < isize > { debug_assert_eq!(strides
        .ndim(), $n, "Attempted to index with {:?} in array with {} axes", self, strides
        .ndim()); stride_offset_checked(dim.ix(), strides.ix(), self) } #[inline] fn
        index_unchecked(& self, strides : & IxDyn) -> isize { debug_assert_eq!(strides
        .ndim(), $n, "Attempted to index with {:?} in array with {} axes", self, strides
        .ndim()); $(stride_offset(self[$index], get!(strides, $index)) +)* 0 } })+
    };
}
ndindex_with_array! {
    [0, Ix0] [1, Ix1 0] [2, Ix2 0 1] [3, Ix3 0 1 2] [4, Ix4 0 1 2 3] [5, Ix5 0 1 2 3 4]
    [6, Ix6 0 1 2 3 4 5]
}
impl<'a> IntoDimension for &'a [Ix] {
    type Dim = IxDyn;
    fn into_dimension(self) -> Self::Dim {
        Dim(IxDynImpl::from(self))
    }
}
unsafe impl<'a> NdIndex<IxDyn> for &'a IxDyn {
    fn index_checked(&self, dim: &IxDyn, strides: &IxDyn) -> Option<isize> {
        (**self).index_checked(dim, strides)
    }
    fn index_unchecked(&self, strides: &IxDyn) -> isize {
        (**self).index_unchecked(strides)
    }
}
unsafe impl<'a> NdIndex<IxDyn> for &'a [Ix] {
    fn index_checked(&self, dim: &IxDyn, strides: &IxDyn) -> Option<isize> {
        stride_offset_checked(dim.ix(), strides.ix(), *self)
    }
    fn index_unchecked(&self, strides: &IxDyn) -> isize {
        zip(strides.ix(), *self).map(|(&s, &i)| stride_offset(i, s)).sum()
    }
}
#[cfg(test)]
mod tests_rug_914 {
    use super::*;
    use crate::dimension::NdIndex;
    use crate::prelude::{IxDyn, Dimension};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_914_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3;
        let rug_fuzz_1 = 3;
        let rug_fuzz_2 = 3;
        let mut p0 = IxDyn::zeros(rug_fuzz_0);
        let mut p1 = IxDyn::zeros(rug_fuzz_1);
        let mut p2 = IxDyn::zeros(rug_fuzz_2);
        <IxDyn as NdIndex<IxDyn>>::index_checked(&p0, &p1, &p2);
        let _rug_ed_tests_rug_914_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_916 {
    use super::*;
    use crate::dimension::{NdIndex, Dim, Dimension};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_916_rrrruuuugggg_test_rug = 0;
        let p0 = ();
        let p1 = Dim::default();
        let p2 = Dim::default();
        p0.index_checked(&p1, &p2);
        let _rug_ed_tests_rug_916_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_918 {
    use super::*;
    use crate::dimension::NdIndex;
    use crate::dimension::Dim;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_918_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 3;
        let rug_fuzz_4 = 4;
        let rug_fuzz_5 = 5;
        let p0: (usize, usize) = (rug_fuzz_0, rug_fuzz_1);
        let p1: Dim<[usize; 2]> = Dim([rug_fuzz_2, rug_fuzz_3]);
        let p2: Dim<[usize; 2]> = Dim([rug_fuzz_4, rug_fuzz_5]);
        p0.index_checked(&p1, &p2);
        let _rug_ed_tests_rug_918_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_925 {
    use super::*;
    use crate::dimension::NdIndex;
    use crate::dimension::dim::Dim;
    use crate::Dimension;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_925_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1usize;
        let rug_fuzz_1 = 2usize;
        let rug_fuzz_2 = 3usize;
        let rug_fuzz_3 = 4usize;
        let rug_fuzz_4 = 5usize;
        let rug_fuzz_5 = 6usize;
        let rug_fuzz_6 = 7usize;
        let rug_fuzz_7 = 8usize;
        let rug_fuzz_8 = 9usize;
        let rug_fuzz_9 = 10usize;
        let mut p0 = (rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3, rug_fuzz_4);
        let mut p1 = Dim([rug_fuzz_5, rug_fuzz_6, rug_fuzz_7, rug_fuzz_8, rug_fuzz_9]);
        <(
            usize,
            usize,
            usize,
            usize,
            usize,
        ) as crate::dimension::ndindex::NdIndex<
            crate::dimension::dim::Dim<[usize; 5]>,
        >>::index_unchecked(&p0, &p1);
        let _rug_ed_tests_rug_925_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_926 {
    use super::*;
    use crate::dimension::{NdIndex, Dim};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_926_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 3;
        let rug_fuzz_2 = 2;
        let mut p0: usize = rug_fuzz_0;
        let mut p1: Dim<[usize; 1]> = Dim([rug_fuzz_1]);
        let mut p2: Dim<[usize; 1]> = Dim([rug_fuzz_2]);
        p0.index_checked(&p1, &p2);
        let _rug_ed_tests_rug_926_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_927 {
    use super::*;
    use crate::dimension::NdIndex;
    use crate::dimension::dim::Dim;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_927_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 10;
        let mut p0: usize = rug_fuzz_0;
        let mut p1: Dim<[usize; 1]> = Dim([rug_fuzz_1]);
        <usize as NdIndex<Dim<[usize; 1]>>>::index_unchecked(&p0, &p1);
        let _rug_ed_tests_rug_927_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_932 {
    use super::*;
    use crate::dimension::{NdIndex, Dim, dynindeximpl::IxDynImpl};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_932_rrrruuuugggg_test_rug = 0;
        let mut p0 = Dim::<[usize; 0]>::default();
        let p1 = Dim::<IxDynImpl>::default();
        let p2 = Dim::<IxDynImpl>::default();
        <Dim<[usize; 0]>>::index_checked(&p0, &p1, &p2);
        let _rug_ed_tests_rug_932_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_936 {
    use super::*;
    use crate::dimension::{NdIndex, Dim};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_936_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let mut p0 = [rug_fuzz_0];
        let mut p1 = Dim::default();
        let mut p2 = Dim::default();
        <[usize; 1] as NdIndex<Dim<[usize; 1]>>>::index_checked(&p0, &p1, &p2);
        let _rug_ed_tests_rug_936_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_937 {
    use super::*;
    use crate::dimension::NdIndex;
    use crate::dimension::dim::Dim;
    #[test]
    fn test_index_unchecked() {
        let _rug_st_tests_rug_937_rrrruuuugggg_test_index_unchecked = 0;
        let rug_fuzz_0 = 3;
        let mut p0: [usize; 1] = [rug_fuzz_0];
        let p1: Dim<[usize; 1]> = Dim::<[usize; 1]>::default();
        <[usize; 1] as NdIndex<Dim<[usize; 1]>>>::index_unchecked(&p0, &p1);
        let _rug_ed_tests_rug_937_rrrruuuugggg_test_index_unchecked = 0;
    }
}
#[cfg(test)]
mod tests_rug_938 {
    use crate::dimension::{NdIndex, Dim};
    use crate::dimension::dynindeximpl::IxDynImpl;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_938_rrrruuuugggg_test_rug = 0;
        let mut p0 = Dim::<[usize; 1]>::default();
        let p1 = Dim::<IxDynImpl>::default();
        let p2 = Dim::<IxDynImpl>::default();
        p0.index_checked(&p1, &p2);
        let _rug_ed_tests_rug_938_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_939 {
    use super::*;
    use crate::dimension::NdIndex;
    use crate::dimension::dim::Dim;
    use crate::dimension::dynindeximpl::IxDynImpl;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_939_rrrruuuugggg_test_rug = 0;
        let mut p0: Dim<[usize; 1]> = Dim::<[usize; 1]>::default();
        let mut p1: Dim<IxDynImpl> = Dim::<IxDynImpl>::default();
        p0.index_unchecked(&p1);
        let _rug_ed_tests_rug_939_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_942 {
    use super::*;
    use crate::dimension::NdIndex;
    use crate::dimension::Dim;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_942_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 3;
        let rug_fuzz_4 = 2;
        let rug_fuzz_5 = 3;
        let mut p0: [usize; 2] = [rug_fuzz_0, rug_fuzz_1];
        let mut p1: Dim<[usize; 2]> = Dim([rug_fuzz_2, rug_fuzz_3]);
        let mut p2: Dim<[usize; 2]> = Dim([rug_fuzz_4, rug_fuzz_5]);
        p0.index_checked(&p1, &p2);
        let _rug_ed_tests_rug_942_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_943 {
    use super::*;
    use crate::dimension::NdIndex;
    use crate::dimension::Dim;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_943_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 3;
        let mut p0: [usize; 2] = [rug_fuzz_0, rug_fuzz_1];
        let mut p1: Dim<[usize; 2]> = Dim([rug_fuzz_2, rug_fuzz_3]);
        <[usize; 2] as NdIndex<Dim<[usize; 2]>>>::index_unchecked(&p0, &p1);
        let _rug_ed_tests_rug_943_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_949 {
    use super::*;
    use crate::dimension::NdIndex;
    use crate::dimension::dim::Dim;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_949_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let mut p0: [usize; 3] = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        let mut p1: Dim<[usize; 3]> = Dim::<[usize; 3]>::default();
        <[usize; 3] as NdIndex<Dim<[usize; 3]>>>::index_unchecked(&p0, &p1);
        let _rug_ed_tests_rug_949_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_954 {
    use super::*;
    use crate::dimension::NdIndex;
    use crate::dimension::dim::Dim;
    #[test]
    fn test_index_checked() {
        let _rug_st_tests_rug_954_rrrruuuugggg_test_index_checked = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 4;
        let mut p0: [usize; 4] = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3];
        let mut p1: Dim<[usize; 4]> = Dim::<[usize; 4]>::default();
        let mut p2: Dim<[usize; 4]> = Dim::<[usize; 4]>::default();
        <[usize; 4] as NdIndex<Dim<[usize; 4]>>>::index_checked(&p0, &p1, &p2);
        let _rug_ed_tests_rug_954_rrrruuuugggg_test_index_checked = 0;
    }
}
#[cfg(test)]
mod tests_rug_955 {
    use super::*;
    use crate::dimension::NdIndex;
    use crate::dimension::dim::Dim;
    #[test]
    fn test_index_unchecked() {
        let _rug_st_tests_rug_955_rrrruuuugggg_test_index_unchecked = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 4;
        let mut p0: [usize; 4] = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3];
        let mut p1: Dim<[usize; 4]> = Dim::<[usize; 4]>::default();
        <[usize; 4] as NdIndex<Dim<[usize; 4]>>>::index_unchecked(&p0, &p1);
        let _rug_ed_tests_rug_955_rrrruuuugggg_test_index_unchecked = 0;
    }
}
#[cfg(test)]
mod tests_rug_960 {
    use super::*;
    use crate::dimension::{NdIndex, dim::Dim};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_960_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 4;
        let rug_fuzz_4 = 5;
        let mut p0: [usize; 5] = [
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
        ];
        let mut p1: Dim<[usize; 5]> = Dim(p0);
        let mut p2: Dim<[usize; 5]> = Dim(p0);
        p0.index_checked(&p1, &p2);
        let _rug_ed_tests_rug_960_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_961 {
    use super::*;
    use crate::dimension::{NdIndex, Dim};
    #[test]
    fn test_index_unchecked() {
        let _rug_st_tests_rug_961_rrrruuuugggg_test_index_unchecked = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 4;
        let rug_fuzz_4 = 5;
        let mut p0: [usize; 5] = [
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
        ];
        let p1: Dim<[usize; 5]> = Dim::<[usize; 5]>::default();
        <[usize; 5] as NdIndex<Dim<[usize; 5]>>>::index_unchecked(&p0, &p1);
        let _rug_ed_tests_rug_961_rrrruuuugggg_test_index_unchecked = 0;
    }
}
#[cfg(test)]
mod tests_rug_962 {
    use super::*;
    use crate::dimension::{NdIndex, Dim};
    use crate::dimension::dynindeximpl::IxDynImpl;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_962_rrrruuuugggg_test_rug = 0;
        let mut p0 = Dim::<[usize; 5]>::default();
        let mut p1 = Dim::<IxDynImpl>::default();
        let mut p2 = Dim::<IxDynImpl>::default();
        <Dim<[usize; 5]> as NdIndex<Dim<IxDynImpl>>>::index_checked(&p0, &p1, &p2);
        let _rug_ed_tests_rug_962_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_966 {
    use super::*;
    use crate::dimension::{NdIndex, Dim};
    #[test]
    fn test_index_checked() {
        let _rug_st_tests_rug_966_rrrruuuugggg_test_index_checked = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 4;
        let rug_fuzz_4 = 5;
        let rug_fuzz_5 = 6;
        let mut p0: [usize; 6] = [
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
        ];
        let mut p1: Dim<[usize; 6]> = Dim(p0);
        let mut p2: Dim<[usize; 6]> = Dim(p0);
        p0.index_checked(&p1, &p2);
        let _rug_ed_tests_rug_966_rrrruuuugggg_test_index_checked = 0;
    }
}
#[cfg(test)]
mod tests_rug_967 {
    use super::*;
    use crate::dimension::NdIndex;
    use crate::dimension::dim::Dim;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_967_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 4;
        let rug_fuzz_4 = 5;
        let rug_fuzz_5 = 6;
        let mut p0: [usize; 6] = [
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
        ];
        let mut p1: Dim<[usize; 6]> = Dim::<[usize; 6]>::default();
        p0.index_unchecked(&p1);
        let _rug_ed_tests_rug_967_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_972 {
    use super::*;
    use crate::dimension::{IntoDimension, Dim, IxDynImpl};
    #[test]
    fn test_into_dimension() {
        let _rug_st_tests_rug_972_rrrruuuugggg_test_into_dimension = 0;
        let rug_fuzz_0 = 3;
        let rug_fuzz_1 = 4;
        let mut p0 = &[rug_fuzz_0, rug_fuzz_1];
        <&[usize]>::into_dimension(p0);
        let _rug_ed_tests_rug_972_rrrruuuugggg_test_into_dimension = 0;
    }
}
