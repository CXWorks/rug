//! Tuple to array conversion, IntoDimension, and related things
use num_traits::Zero;
use std::ops::{Index, IndexMut};
use crate::{Dim, Dimension, Ix, Ix1, IxDyn, IxDynImpl};
/// $m: macro callback
/// $m is called with $arg and then the indices corresponding to the size argument
macro_rules! index {
    ($m:ident $arg:tt 0) => {
        $m ! ($arg)
    };
    ($m:ident $arg:tt 1) => {
        $m ! ($arg 0)
    };
    ($m:ident $arg:tt 2) => {
        $m ! ($arg 0 1)
    };
    ($m:ident $arg:tt 3) => {
        $m ! ($arg 0 1 2)
    };
    ($m:ident $arg:tt 4) => {
        $m ! ($arg 0 1 2 3)
    };
    ($m:ident $arg:tt 5) => {
        $m ! ($arg 0 1 2 3 4)
    };
    ($m:ident $arg:tt 6) => {
        $m ! ($arg 0 1 2 3 4 5)
    };
    ($m:ident $arg:tt 7) => {
        $m ! ($arg 0 1 2 3 4 5 6)
    };
}
macro_rules! index_item {
    ($m:ident $arg:tt 0) => {};
    ($m:ident $arg:tt 1) => {
        $m ! ($arg 0);
    };
    ($m:ident $arg:tt 2) => {
        $m ! ($arg 0 1);
    };
    ($m:ident $arg:tt 3) => {
        $m ! ($arg 0 1 2);
    };
    ($m:ident $arg:tt 4) => {
        $m ! ($arg 0 1 2 3);
    };
    ($m:ident $arg:tt 5) => {
        $m ! ($arg 0 1 2 3 4);
    };
    ($m:ident $arg:tt 6) => {
        $m ! ($arg 0 1 2 3 4 5);
    };
    ($m:ident $arg:tt 7) => {
        $m ! ($arg 0 1 2 3 4 5 6);
    };
}
/// Argument conversion a dimension.
pub trait IntoDimension {
    type Dim: Dimension;
    fn into_dimension(self) -> Self::Dim;
}
impl IntoDimension for Ix {
    type Dim = Ix1;
    #[inline(always)]
    fn into_dimension(self) -> Ix1 {
        Ix1(self)
    }
}
impl<D> IntoDimension for D
where
    D: Dimension,
{
    type Dim = D;
    #[inline(always)]
    fn into_dimension(self) -> Self {
        self
    }
}
impl IntoDimension for IxDynImpl {
    type Dim = IxDyn;
    #[inline(always)]
    fn into_dimension(self) -> Self::Dim {
        Dim::new(self)
    }
}
impl IntoDimension for Vec<Ix> {
    type Dim = IxDyn;
    #[inline(always)]
    fn into_dimension(self) -> Self::Dim {
        Dim::new(IxDynImpl::from(self))
    }
}
pub trait Convert {
    type To;
    fn convert(self) -> Self::To;
}
macro_rules! sub {
    ($_x:tt $y:tt) => {
        $y
    };
}
macro_rules! tuple_type {
    ([$T:ident] $($index:tt)*) => {
        ($(sub!($index $T),)*)
    };
}
macro_rules! tuple_expr {
    ([$self_:expr] $($index:tt)*) => {
        ($($self_ [$index],)*)
    };
}
macro_rules! array_expr {
    ([$self_:expr] $($index:tt)*) => {
        [$($self_ . $index,)*]
    };
}
macro_rules! array_zero {
    ([] $($index:tt)*) => {
        [$(sub!($index 0),)*]
    };
}
macro_rules! tuple_to_array {
    ([] $($n:tt)*) => {
        $(impl Convert for [Ix; $n] { type To = index!(tuple_type[Ix] $n); #[inline] fn
        convert(self) -> Self::To { index!(tuple_expr[self] $n) } } impl IntoDimension
        for [Ix; $n] { type Dim = Dim < [Ix; $n] >; #[inline(always)] fn
        into_dimension(self) -> Self::Dim { Dim::new(self) } } impl IntoDimension for
        index!(tuple_type[Ix] $n) { type Dim = Dim < [Ix; $n] >; #[inline(always)] fn
        into_dimension(self) -> Self::Dim { Dim::new(index!(array_expr[self] $n)) } }
        impl Index < usize > for Dim < [Ix; $n] > { type Output = usize;
        #[inline(always)] fn index(& self, index : usize) -> & Self::Output { & self.ix()
        [index] } } impl IndexMut < usize > for Dim < [Ix; $n] > { #[inline(always)] fn
        index_mut(& mut self, index : usize) -> & mut Self::Output { & mut self.ixm()
        [index] } } impl Zero for Dim < [Ix; $n] > { #[inline] fn zero() -> Self {
        Dim::new(index!(array_zero[] $n)) } fn is_zero(& self) -> bool { self.slice()
        .iter().all(| x | * x == 0) } })*
    };
}
index_item!(tuple_to_array[] 7);
#[cfg(test)]
mod tests_rug_750 {
    use super::*;
    use crate::dimension::IntoDimension;
    use crate::Ix1;
    #[test]
    fn test_into_dimension() {
        let _rug_st_tests_rug_750_rrrruuuugggg_test_into_dimension = 0;
        let rug_fuzz_0 = 5;
        let p0: usize = rug_fuzz_0;
        <usize>::into_dimension(p0);
        let _rug_ed_tests_rug_750_rrrruuuugggg_test_into_dimension = 0;
    }
}
#[cfg(test)]
mod tests_rug_751 {
    use super::*;
    use crate::dimension::IntoDimension;
    use crate::prelude::{IxDyn, Dimension};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_751_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3;
        let mut p0: IxDyn = IxDyn::zeros(rug_fuzz_0);
        <IxDyn as IntoDimension>::into_dimension(p0);
        let _rug_ed_tests_rug_751_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_752 {
    use super::*;
    use crate::dimension::IntoDimension;
    use crate::dimension::{IxDynImpl, Dim};
    #[test]
    fn test_into_dimension() {
        let _rug_st_tests_rug_752_rrrruuuugggg_test_into_dimension = 0;
        let p0: IxDynImpl = IxDynImpl::default();
        let _ = <IxDynImpl as IntoDimension>::into_dimension(p0);
        let _rug_ed_tests_rug_752_rrrruuuugggg_test_into_dimension = 0;
    }
}
#[cfg(test)]
mod tests_rug_753 {
    use super::*;
    use crate::dimension::IntoDimension;
    use crate::dimension::{Dim, IxDynImpl};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_753_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let mut p0: std::vec::Vec<usize> = {
            let mut v282: Vec<usize> = Vec::new();
            v282.push(rug_fuzz_0);
            v282.push(rug_fuzz_1);
            v282.push(rug_fuzz_2);
            v282
        };
        <std::vec::Vec<usize> as crate::dimension::IntoDimension>::into_dimension(p0);
        let _rug_ed_tests_rug_753_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_754 {
    use super::*;
    use crate::dimension::conversion::Convert;
    #[test]
    fn test_convert() {
        let _rug_st_tests_rug_754_rrrruuuugggg_test_convert = 0;
        let p0: [usize; 0] = [];
        p0.convert();
        let _rug_ed_tests_rug_754_rrrruuuugggg_test_convert = 0;
    }
}
#[cfg(test)]
mod tests_rug_755 {
    use super::*;
    use crate::dimension::IntoDimension;
    use crate::dimension::Dim;
    #[test]
    fn test_into_dimension() {
        let _rug_st_tests_rug_755_rrrruuuugggg_test_into_dimension = 0;
        let p0: [usize; 0] = [];
        p0.into_dimension();
        let _rug_ed_tests_rug_755_rrrruuuugggg_test_into_dimension = 0;
    }
}
#[cfg(test)]
mod tests_rug_756 {
    use super::*;
    use crate::dimension::{IntoDimension, Dim};
    #[test]
    fn test_into_dimension() {
        let _rug_st_tests_rug_756_rrrruuuugggg_test_into_dimension = 0;
        let mut p0: () = ();
        p0.into_dimension();
        let _rug_ed_tests_rug_756_rrrruuuugggg_test_into_dimension = 0;
    }
}
#[cfg(test)]
mod tests_rug_758 {
    use super::*;
    use crate::dimension::{dim::Dim, conversion::IndexMut};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_758_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3;
        let mut p0: Dim<[usize; 0]> = Dim::default();
        let mut p1: usize = rug_fuzz_0;
        p0.index_mut(p1);
        let _rug_ed_tests_rug_758_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_763 {
    use super::*;
    use crate::dimension::IntoDimension;
    use crate::dimension::{self, Dim};
    #[test]
    fn test_into_dimension() {
        let _rug_st_tests_rug_763_rrrruuuugggg_test_into_dimension = 0;
        let rug_fuzz_0 = 5usize;
        let mut p0 = (rug_fuzz_0,);
        p0.into_dimension();
        let _rug_ed_tests_rug_763_rrrruuuugggg_test_into_dimension = 0;
    }
}
#[cfg(test)]
mod tests_rug_764 {
    use super::*;
    use crate::dimension::conversion::Index;
    use crate::dimension::dim::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_764_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0_usize;
        let mut p0 = Dim::<[usize; 1]>::default();
        let mut p1 = rug_fuzz_0;
        <Dim<[usize; 1]> as Index<usize>>::index(&p0, p1);
        let _rug_ed_tests_rug_764_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_769 {
    use super::*;
    use crate::dimension::IntoDimension;
    use crate::dimension::Dim;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_769_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3;
        let rug_fuzz_1 = 4;
        let mut p0: [usize; 2] = [rug_fuzz_0, rug_fuzz_1];
        <[usize; 2]>::into_dimension(p0);
        let _rug_ed_tests_rug_769_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_776 {
    use super::*;
    use crate::dimension::IntoDimension;
    use crate::dimension::Dim;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_776_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let mut p0: [usize; 3] = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        <[usize; 3]>::into_dimension(p0);
        let _rug_ed_tests_rug_776_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_777 {
    use super::*;
    use crate::dimension::IntoDimension;
    use crate::dimension::conversion::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_777_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1usize;
        let rug_fuzz_1 = 2usize;
        let rug_fuzz_2 = 3usize;
        let mut p0 = (rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        <(usize, usize, usize) as IntoDimension>::into_dimension(p0);
        let _rug_ed_tests_rug_777_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_779 {
    use super::*;
    use crate::dimension::{dim::Dim, conversion::IndexMut};
    #[test]
    fn test_index_mut() {
        let _rug_st_tests_rug_779_rrrruuuugggg_test_index_mut = 0;
        let rug_fuzz_0 = 2;
        let rug_fuzz_1 = 3;
        let rug_fuzz_2 = 4;
        let rug_fuzz_3 = 1;
        let mut p0: Dim<[usize; 3]> = Dim([rug_fuzz_0, rug_fuzz_1, rug_fuzz_2]);
        let p1: usize = rug_fuzz_3;
        p0.index_mut(p1);
        let _rug_ed_tests_rug_779_rrrruuuugggg_test_index_mut = 0;
    }
}
#[cfg(test)]
mod tests_rug_780 {
    use super::*;
    use crate::Dimension;
    use crate::dimension::Dim;
    #[test]
    fn test_zero() {
        let _rug_st_tests_rug_780_rrrruuuugggg_test_zero = 0;
        let result = Dim::<[usize; 3]>::zero();
        let _rug_ed_tests_rug_780_rrrruuuugggg_test_zero = 0;
    }
}
#[cfg(test)]
mod tests_rug_783 {
    use super::*;
    use crate::dimension::IntoDimension;
    use crate::{Dimension, Dim};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_783_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 4;
        let p0: [usize; 4] = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3];
        <[usize; 4] as IntoDimension>::into_dimension(p0);
        let _rug_ed_tests_rug_783_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_784 {
    use super::*;
    use crate::dimension::IntoDimension;
    use crate::dimension::Dim;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_784_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 4;
        let mut p0: (usize, usize, usize, usize) = (
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
        );
        <(usize, usize, usize, usize)>::into_dimension(p0);
        let _rug_ed_tests_rug_784_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_789 {
    use super::*;
    use crate::dimension::conversion::Convert;
    #[test]
    fn test_convert_method() {
        let _rug_st_tests_rug_789_rrrruuuugggg_test_convert_method = 0;
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
        p0.convert();
        let _rug_ed_tests_rug_789_rrrruuuugggg_test_convert_method = 0;
    }
}
#[cfg(test)]
mod tests_rug_790 {
    use super::*;
    use crate::dimension::IntoDimension;
    use crate::dimension::Dim;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_790_rrrruuuugggg_test_rug = 0;
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
        <[usize; 5]>::into_dimension(p0);
        let _rug_ed_tests_rug_790_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_791 {
    use super::*;
    use crate::dimension::{IntoDimension, Dim};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_791_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 4;
        let rug_fuzz_4 = 5;
        let p0: (usize, usize, usize, usize, usize) = (
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
        );
        let _ = p0.into_dimension();
        let _rug_ed_tests_rug_791_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_796 {
    use super::*;
    use crate::dimension::conversion::Convert;
    use crate::dimension::conversion;
    #[test]
    fn test_convert() {
        let _rug_st_tests_rug_796_rrrruuuugggg_test_convert = 0;
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
        <[usize; 6] as conversion::Convert>::convert(p0);
        let _rug_ed_tests_rug_796_rrrruuuugggg_test_convert = 0;
    }
}
#[cfg(test)]
mod tests_rug_797 {
    use super::*;
    use crate::dimension::{IntoDimension, Dim};
    use crate::Dimension;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_797_rrrruuuugggg_test_rug = 0;
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
        p0.into_dimension();
        let _rug_ed_tests_rug_797_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_798 {
    use super::*;
    use crate::{dimension::IntoDimension, dimension::Dim};
    #[test]
    fn test_into_dimension() {
        let _rug_st_tests_rug_798_rrrruuuugggg_test_into_dimension = 0;
        let rug_fuzz_0 = 1usize;
        let rug_fuzz_1 = 2usize;
        let rug_fuzz_2 = 3usize;
        let rug_fuzz_3 = 4usize;
        let rug_fuzz_4 = 5usize;
        let rug_fuzz_5 = 6usize;
        let mut p0 = (
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
        );
        <(
            usize,
            usize,
            usize,
            usize,
            usize,
            usize,
        ) as IntoDimension>::into_dimension(p0);
        let _rug_ed_tests_rug_798_rrrruuuugggg_test_into_dimension = 0;
    }
}
