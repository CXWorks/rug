//! Type aliases for common array sizes
//!
use crate::dimension::Dim;
#[allow(deprecated)]
use crate::{ArcArray, Array, ArrayView, ArrayViewMut, Ix, IxDynImpl, RcArray};
/// Create a zero-dimensional index
#[allow(non_snake_case)]
#[inline(always)]
pub fn Ix0() -> Ix0 {
    Dim::new([])
}
/// Create a one-dimensional index
#[allow(non_snake_case)]
#[inline(always)]
pub fn Ix1(i0: Ix) -> Ix1 {
    Dim::new([i0])
}
/// Create a two-dimensional index
#[allow(non_snake_case)]
#[inline(always)]
pub fn Ix2(i0: Ix, i1: Ix) -> Ix2 {
    Dim::new([i0, i1])
}
/// Create a three-dimensional index
#[allow(non_snake_case)]
#[inline(always)]
pub fn Ix3(i0: Ix, i1: Ix, i2: Ix) -> Ix3 {
    Dim::new([i0, i1, i2])
}
/// Create a four-dimensional index
#[allow(non_snake_case)]
#[inline(always)]
pub fn Ix4(i0: Ix, i1: Ix, i2: Ix, i3: Ix) -> Ix4 {
    Dim::new([i0, i1, i2, i3])
}
/// Create a five-dimensional index
#[allow(non_snake_case)]
#[inline(always)]
pub fn Ix5(i0: Ix, i1: Ix, i2: Ix, i3: Ix, i4: Ix) -> Ix5 {
    Dim::new([i0, i1, i2, i3, i4])
}
/// Create a six-dimensional index
#[allow(non_snake_case)]
#[inline(always)]
pub fn Ix6(i0: Ix, i1: Ix, i2: Ix, i3: Ix, i4: Ix, i5: Ix) -> Ix6 {
    Dim::new([i0, i1, i2, i3, i4, i5])
}
/// Create a dynamic-dimensional index
#[allow(non_snake_case)]
#[inline(always)]
pub fn IxDyn(ix: &[Ix]) -> IxDyn {
    Dim(ix)
}
/// zero-dimensionial
pub type Ix0 = Dim<[Ix; 0]>;
/// one-dimensional
pub type Ix1 = Dim<[Ix; 1]>;
/// two-dimensional
pub type Ix2 = Dim<[Ix; 2]>;
/// three-dimensional
pub type Ix3 = Dim<[Ix; 3]>;
/// four-dimensional
pub type Ix4 = Dim<[Ix; 4]>;
/// five-dimensional
pub type Ix5 = Dim<[Ix; 5]>;
/// six-dimensional
pub type Ix6 = Dim<[Ix; 6]>;
/// dynamic-dimensional
///
/// You can use the `IxDyn` function to create a dimension for an array with
/// dynamic number of dimensions.  (`Vec<usize>` and `&[usize]` also implement
/// `IntoDimension` to produce `IxDyn`).
///
/// ```
/// use ndarray::ArrayD;
/// use ndarray::IxDyn;
///
/// // Create a 5 × 6 × 3 × 4 array using the dynamic dimension type
/// let mut a = ArrayD::<f64>::zeros(IxDyn(&[5, 6, 3, 4]));
/// // Create a 1 × 3 × 4 array using the dynamic dimension type
/// let mut b = ArrayD::<f64>::zeros(IxDyn(&[1, 3, 4]));
///
/// // We can use broadcasting to add arrays of compatible shapes together:
/// a += &b;
///
/// // We can index into a, b using fixed size arrays:
/// a[[0, 0, 0, 0]] = 0.;
/// b[[0, 2, 3]] = a[[0, 0, 2, 3]];
/// // Note: indexing will panic at runtime if the number of indices given does
/// // not match the array.
///
/// // We can keep them in the same vector because both the arrays have
/// // the same type `Array<f64, IxDyn>` a.k.a `ArrayD<f64>`:
/// let arrays = vec![a, b];
/// ```
pub type IxDyn = Dim<IxDynImpl>;
/// zero-dimensional array
pub type Array0<A> = Array<A, Ix0>;
/// one-dimensional array
pub type Array1<A> = Array<A, Ix1>;
/// two-dimensional array
pub type Array2<A> = Array<A, Ix2>;
/// three-dimensional array
pub type Array3<A> = Array<A, Ix3>;
/// four-dimensional array
pub type Array4<A> = Array<A, Ix4>;
/// five-dimensional array
pub type Array5<A> = Array<A, Ix5>;
/// six-dimensional array
pub type Array6<A> = Array<A, Ix6>;
/// dynamic-dimensional array
pub type ArrayD<A> = Array<A, IxDyn>;
/// zero-dimensional array view
pub type ArrayView0<'a, A> = ArrayView<'a, A, Ix0>;
/// one-dimensional array view
pub type ArrayView1<'a, A> = ArrayView<'a, A, Ix1>;
/// two-dimensional array view
pub type ArrayView2<'a, A> = ArrayView<'a, A, Ix2>;
/// three-dimensional array view
pub type ArrayView3<'a, A> = ArrayView<'a, A, Ix3>;
/// four-dimensional array view
pub type ArrayView4<'a, A> = ArrayView<'a, A, Ix4>;
/// five-dimensional array view
pub type ArrayView5<'a, A> = ArrayView<'a, A, Ix5>;
/// six-dimensional array view
pub type ArrayView6<'a, A> = ArrayView<'a, A, Ix6>;
/// dynamic-dimensional array view
pub type ArrayViewD<'a, A> = ArrayView<'a, A, IxDyn>;
/// zero-dimensional read-write array view
pub type ArrayViewMut0<'a, A> = ArrayViewMut<'a, A, Ix0>;
/// one-dimensional read-write array view
pub type ArrayViewMut1<'a, A> = ArrayViewMut<'a, A, Ix1>;
/// two-dimensional read-write array view
pub type ArrayViewMut2<'a, A> = ArrayViewMut<'a, A, Ix2>;
/// three-dimensional read-write array view
pub type ArrayViewMut3<'a, A> = ArrayViewMut<'a, A, Ix3>;
/// four-dimensional read-write array view
pub type ArrayViewMut4<'a, A> = ArrayViewMut<'a, A, Ix4>;
/// five-dimensional read-write array view
pub type ArrayViewMut5<'a, A> = ArrayViewMut<'a, A, Ix5>;
/// six-dimensional read-write array view
pub type ArrayViewMut6<'a, A> = ArrayViewMut<'a, A, Ix6>;
/// dynamic-dimensional read-write array view
pub type ArrayViewMutD<'a, A> = ArrayViewMut<'a, A, IxDyn>;
/// one-dimensional shared ownership array
#[allow(deprecated)]
#[deprecated(note = "`RcArray` has been renamed to `ArcArray`")]
pub type RcArray1<A> = RcArray<A, Ix1>;
/// two-dimensional shared ownership array
#[allow(deprecated)]
#[deprecated(note = "`RcArray` has been renamed to `ArcArray`")]
pub type RcArray2<A> = RcArray<A, Ix2>;
/// one-dimensional shared ownership array
pub type ArcArray1<A> = ArcArray<A, Ix1>;
/// two-dimensional shared ownership array
pub type ArcArray2<A> = ArcArray<A, Ix2>;
#[cfg(test)]
mod tests_rug_1 {
    use super::*;
    use crate::aliases::Ix0;
    use crate::Dim;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_1_rrrruuuugggg_test_rug = 0;
        let _result: Ix0 = Ix0();
        let _rug_ed_tests_rug_1_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_2 {
    use super::*;
    use crate::aliases::{Ix1, Ix, Dim};
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: usize = rug_fuzz_0;
        Ix1(p0);
             }
});    }
}
#[cfg(test)]
mod tests_rug_3 {
    use super::*;
    use crate::aliases::Ix2;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: usize = rug_fuzz_0;
        let mut p1: usize = rug_fuzz_1;
        Ix2(p0, p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_4 {
    use super::*;
    use crate::aliases::Ix3;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(usize, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: usize = rug_fuzz_0;
        let mut p1: usize = rug_fuzz_1;
        let mut p2: usize = rug_fuzz_2;
        Ix3(p0, p1, p2);
             }
});    }
}
#[cfg(test)]
mod tests_rug_5 {
    use super::*;
    use crate::aliases::Ix4;
    #[test]
    fn test_ix4() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(usize, usize, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: usize = rug_fuzz_0;
        let mut p1: usize = rug_fuzz_1;
        let mut p2: usize = rug_fuzz_2;
        let mut p3: usize = rug_fuzz_3;
        let _ = Ix4(p0, p1, p2, p3);
             }
});    }
}
#[cfg(test)]
mod tests_rug_6 {
    use super::*;
    use crate::aliases::Ix5;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4)) = <(usize, usize, usize, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: usize = rug_fuzz_0;
        let mut p1: usize = rug_fuzz_1;
        let mut p2: usize = rug_fuzz_2;
        let mut p3: usize = rug_fuzz_3;
        let mut p4: usize = rug_fuzz_4;
        Ix5(p0, p1, p2, p3, p4);
             }
});    }
}
#[cfg(test)]
mod tests_rug_7 {
    use super::*;
    use crate::aliases::{Ix, Ix6, Dim};
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5)) = <(usize, usize, usize, usize, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: usize = rug_fuzz_0;
        let p1: usize = rug_fuzz_1;
        let p2: usize = rug_fuzz_2;
        let p3: usize = rug_fuzz_3;
        let p4: usize = rug_fuzz_4;
        let p5: usize = rug_fuzz_5;
        crate::aliases::Ix6(p0, p1, p2, p3, p4, p5);
             }
});    }
}
#[cfg(test)]
mod tests_rug_8 {
    use super::*;
    use crate::{Dim, IxDyn};
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(usize, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: &[usize] = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        crate::aliases::IxDyn(p0);
             }
});    }
}
