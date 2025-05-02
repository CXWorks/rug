use std::mem::{forget, size_of};
use std::slice;
use crate::imp_prelude::*;
use crate::{dimension, ArcArray1, ArcArray2};
/// Create an [**`Array`**](type.Array.html) with one, two or
/// three dimensions.
///
/// ```
/// use ndarray::array;
/// let a1 = array![1, 2, 3, 4];
///
/// let a2 = array![[1, 2],
///                 [3, 4]];
///
/// let a3 = array![[[1, 2], [3, 4]],
///                 [[5, 6], [7, 8]]];
///
/// assert_eq!(a1.shape(), &[4]);
/// assert_eq!(a2.shape(), &[2, 2]);
/// assert_eq!(a3.shape(), &[2, 2, 2]);
/// ```
///
/// This macro uses `vec![]`, and has the same ownership semantics;
/// elements are moved into the resulting `Array`.
///
/// Use `array![...].into_shared()` to create an `ArcArray`.
#[macro_export]
macro_rules! array {
    ($([$([$($x:expr),* $(,)*]),+ $(,)*]),+ $(,)*) => {
        { $crate ::Array3::from(vec![$([$([$($x,)*],)*],)*]) }
    };
    ($([$($x:expr),* $(,)*]),+ $(,)*) => {
        { $crate ::Array2::from(vec![$([$($x,)*],)*]) }
    };
    ($($x:expr),* $(,)*) => {
        { $crate ::Array::from(vec![$($x,)*]) }
    };
}
/// Create a zero-dimensional array with the element `x`.
pub fn arr0<A>(x: A) -> Array0<A> {
    unsafe { ArrayBase::from_shape_vec_unchecked((), vec![x]) }
}
/// Create a one-dimensional array with elements from `xs`.
pub fn arr1<A: Clone>(xs: &[A]) -> Array1<A> {
    ArrayBase::from(xs.to_vec())
}
/// Create a one-dimensional array with elements from `xs`.
pub fn rcarr1<A: Clone>(xs: &[A]) -> ArcArray1<A> {
    arr1(xs).into_shared()
}
/// Create a zero-dimensional array view borrowing `x`.
pub fn aview0<A>(x: &A) -> ArrayView0<'_, A> {
    unsafe { ArrayView::from_shape_ptr(Ix0(), x) }
}
/// Create a one-dimensional array view with elements borrowing `xs`.
///
/// ```
/// use ndarray::aview1;
///
/// let data = [1.0; 1024];
///
/// // Create a 2D array view from borrowed data
/// let a2d = aview1(&data).into_shape((32, 32)).unwrap();
///
/// assert_eq!(a2d.sum(), 1024.0);
/// ```
pub fn aview1<A>(xs: &[A]) -> ArrayView1<'_, A> {
    ArrayView::from(xs)
}
/// Create a two-dimensional array view with elements borrowing `xs`.
///
/// **Panics** if the product of non-zero axis lengths overflows `isize`. (This
/// can only occur when `V` is zero-sized.)
pub fn aview2<A, V: FixedInitializer<Elem = A>>(xs: &[V]) -> ArrayView2<'_, A> {
    let cols = V::len();
    let rows = xs.len();
    let dim = Ix2(rows, cols);
    if size_of::<V>() == 0 {
        dimension::size_of_shape_checked(&dim)
            .expect("Product of non-zero axis lengths must not overflow isize.");
    }
    unsafe {
        let data = slice::from_raw_parts(xs.as_ptr() as *const A, cols * rows);
        ArrayView::from_shape_ptr(dim, data.as_ptr())
    }
}
/// Create a one-dimensional read-write array view with elements borrowing `xs`.
///
/// ```
/// use ndarray::{aview_mut1, s};
/// // Create an array view over some data, then slice it and modify it.
/// let mut data = [0; 1024];
/// {
///     let mut a = aview_mut1(&mut data).into_shape((32, 32)).unwrap();
///     a.slice_mut(s![.., ..;3]).fill(5);
/// }
/// assert_eq!(&data[..10], [5, 0, 0, 5, 0, 0, 5, 0, 0, 5]);
/// ```
pub fn aview_mut1<A>(xs: &mut [A]) -> ArrayViewMut1<'_, A> {
    ArrayViewMut::from(xs)
}
/// Create a two-dimensional read-write array view with elements borrowing `xs`.
///
/// **Panics** if the product of non-zero axis lengths overflows `isize`. (This
/// can only occur when `V` is zero-sized.)
///
/// # Example
///
/// ```
/// use ndarray::aview_mut2;
///
/// // The inner (nested) array must be of length 1 to 16, but the outer
/// // can be of any length.
/// let mut data = [[0.; 2]; 128];
/// {
///     // Make a 128 x 2 mut array view then turn it into 2 x 128
///     let mut a = aview_mut2(&mut data).reversed_axes();
///     // Make the first row ones and second row minus ones.
///     a.row_mut(0).fill(1.);
///     a.row_mut(1).fill(-1.);
/// }
/// // look at the start of the result
/// assert_eq!(&data[..3], [[1., -1.], [1., -1.], [1., -1.]]);
/// ```
pub fn aview_mut2<A, V: FixedInitializer<Elem = A>>(
    xs: &mut [V],
) -> ArrayViewMut2<'_, A> {
    let cols = V::len();
    let rows = xs.len();
    let dim = Ix2(rows, cols);
    if size_of::<V>() == 0 {
        dimension::size_of_shape_checked(&dim)
            .expect("Product of non-zero axis lengths must not overflow isize.");
    }
    unsafe {
        let data = slice::from_raw_parts_mut(xs.as_mut_ptr() as *mut A, cols * rows);
        ArrayViewMut::from_shape_ptr(dim, data.as_mut_ptr())
    }
}
/// Fixed-size array used for array initialization
pub unsafe trait FixedInitializer {
    type Elem;
    fn as_init_slice(&self) -> &[Self::Elem];
    fn len() -> usize;
}
macro_rules! impl_arr_init {
    (__impl $n:expr) => {
        unsafe impl < T > FixedInitializer for [T; $n] { type Elem = T; fn
        as_init_slice(& self) -> & [T] { self } fn len() -> usize { $n } }
    };
    () => {};
    ($n:expr, $($m:expr,)*) => {
        impl_arr_init!(__impl $n); impl_arr_init!($($m,)*);
    };
}
impl_arr_init!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,);
/// Create a two-dimensional array with elements from `xs`.
///
/// ```
/// use ndarray::arr2;
///
/// let a = arr2(&[[1, 2, 3],
///                [4, 5, 6]]);
/// assert!(
///     a.shape() == [2, 3]
/// );
/// ```
pub fn arr2<A: Clone, V: FixedInitializer<Elem = A>>(xs: &[V]) -> Array2<A>
where
    V: Clone,
{
    Array2::from(xs.to_vec())
}
impl<A, V> From<Vec<V>> for Array2<A>
where
    V: FixedInitializer<Elem = A>,
{
    /// Converts the `Vec` of arrays to an owned 2-D array.
    ///
    /// **Panics** if the product of non-zero axis lengths overflows `isize`.
    fn from(mut xs: Vec<V>) -> Self {
        let dim = Ix2(xs.len(), V::len());
        let ptr = xs.as_mut_ptr();
        let cap = xs.capacity();
        let expand_len = dimension::size_of_shape_checked(&dim)
            .expect("Product of non-zero axis lengths must not overflow isize.");
        forget(xs);
        unsafe {
            let v = if size_of::<A>() == 0 {
                Vec::from_raw_parts(ptr as *mut A, expand_len, expand_len)
            } else if V::len() == 0 {
                Vec::new()
            } else {
                let expand_cap = cap * V::len();
                Vec::from_raw_parts(ptr as *mut A, expand_len, expand_cap)
            };
            ArrayBase::from_shape_vec_unchecked(dim, v)
        }
    }
}
impl<A, V, U> From<Vec<V>> for Array3<A>
where
    V: FixedInitializer<Elem = U>,
    U: FixedInitializer<Elem = A>,
{
    /// Converts the `Vec` of arrays to an owned 3-D array.
    ///
    /// **Panics** if the product of non-zero axis lengths overflows `isize`.
    fn from(mut xs: Vec<V>) -> Self {
        let dim = Ix3(xs.len(), V::len(), U::len());
        let ptr = xs.as_mut_ptr();
        let cap = xs.capacity();
        let expand_len = dimension::size_of_shape_checked(&dim)
            .expect("Product of non-zero axis lengths must not overflow isize.");
        forget(xs);
        unsafe {
            let v = if size_of::<A>() == 0 {
                Vec::from_raw_parts(ptr as *mut A, expand_len, expand_len)
            } else if V::len() == 0 || U::len() == 0 {
                Vec::new()
            } else {
                let expand_cap = cap * V::len() * U::len();
                Vec::from_raw_parts(ptr as *mut A, expand_len, expand_cap)
            };
            ArrayBase::from_shape_vec_unchecked(dim, v)
        }
    }
}
/// Create a two-dimensional array with elements from `xs`.
///
pub fn rcarr2<A: Clone, V: Clone + FixedInitializer<Elem = A>>(
    xs: &[V],
) -> ArcArray2<A> {
    arr2(xs).into_shared()
}
/// Create a three-dimensional array with elements from `xs`.
///
/// **Panics** if the slices are not all of the same length.
///
/// ```
/// use ndarray::arr3;
///
/// let a = arr3(&[[[1, 2],
///                 [3, 4]],
///                [[5, 6],
///                 [7, 8]],
///                [[9, 0],
///                 [1, 2]]]);
/// assert!(
///     a.shape() == [3, 2, 2]
/// );
/// ```
pub fn arr3<A: Clone, V: FixedInitializer<Elem = U>, U: FixedInitializer<Elem = A>>(
    xs: &[V],
) -> Array3<A>
where
    V: Clone,
    U: Clone,
{
    Array3::from(xs.to_vec())
}
/// Create a three-dimensional array with elements from `xs`.
pub fn rcarr3<A: Clone, V: FixedInitializer<Elem = U>, U: FixedInitializer<Elem = A>>(
    xs: &[V],
) -> ArcArray<A, Ix3>
where
    V: Clone,
    U: Clone,
{
    arr3(xs).into_shared()
}
#[cfg(test)]
mod tests_rug_34 {
    use super::*;
    use crate::Array0;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: i32 = rug_fuzz_0;
        crate::free_functions::arr0(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_35 {
    use super::*;
    use crate::{Array1, ArrayBase, prelude::*};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(i32, i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: &[i32] = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        crate::free_functions::arr1(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_36 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(f64, f64, f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: &[f64] = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        crate::free_functions::rcarr1(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_37 {
    use super::*;
    use crate::{ArrayView0, ArrayView, Ix0};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: i32 = rug_fuzz_0;
        crate::free_functions::aview0(&p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_38 {
    use super::*;
    use crate::{ArrayView1, ArrayView};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let data = [rug_fuzz_0; 1024];
        let p0: &[f64] = &data;
        crate::free_functions::aview1(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_40 {
    use super::*;
    use crate::{ArrayViewMut1, arr1};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: &mut [usize] = &mut [rug_fuzz_0; 1024];
        crate::free_functions::aview_mut1(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_41 {
    use super::*;
    use crate::{aview_mut2, ArrayViewMut2, FixedInitializer};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = &mut [[rug_fuzz_0; 2]; 128];
        crate::free_functions::aview_mut2(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_42 {
    use super::*;
    use crate::{arr2, Array2, FixedInitializer};
    #[test]
    fn test_arr2() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(i32, i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: Vec<[i32; 3]> = vec![[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2], [4, 5, 6]];
        crate::free_functions::arr2(&p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_48 {
    use super::*;
    use crate::free_functions::FixedInitializer;
    #[test]
    fn test_as_init_slice() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: [i32; 1] = [rug_fuzz_0];
        <[i32; 1] as FixedInitializer>::as_init_slice(&p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_49 {
    use super::*;
    use crate::FixedInitializer;
    #[test]
    fn test_len() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let n: usize = rug_fuzz_0;
        let expected_len: usize = rug_fuzz_1;
        debug_assert_eq!(< [usize; 1] as FixedInitializer > ::len(), expected_len);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_51 {
    use super::*;
    use crate::FixedInitializer;
    #[test]
    fn test_len() {
        let _rug_st_tests_rug_51_rrrruuuugggg_test_len = 0;
        let len_result: usize = <[u32; 2]>::len();
        debug_assert_eq!(len_result, 2);
        let _rug_ed_tests_rug_51_rrrruuuugggg_test_len = 0;
    }
}
#[cfg(test)]
mod tests_rug_52 {
    use super::*;
    use crate::free_functions::FixedInitializer;
    #[test]
    fn test_as_init_slice() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(i32, i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        <[i32; 3]>::as_init_slice(&p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_53 {
    use super::*;
    use crate::{Array2, free_functions::FixedInitializer};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_53_rrrruuuugggg_test_rug = 0;
        <[usize; 3] as FixedInitializer>::len();
        let _rug_ed_tests_rug_53_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_56 {
    use super::*;
    use crate::free_functions::FixedInitializer;
    #[test]
    fn test_as_init_slice() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4)) = <(i32, i32, i32, i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: [i32; 5] = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3, rug_fuzz_4];
        p0.as_init_slice();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_58 {
    use super::*;
    use crate::free_functions::FixedInitializer;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5)) = <(u32, u32, u32, u32, u32, u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: [u32; 6] = [
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
        ];
        <[u32; 6]>::as_init_slice(&p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_61 {
    use super::*;
    use crate::FixedInitializer;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_61_rrrruuuugggg_test_rug = 0;
        let len_result: usize = <[u32; 7] as FixedInitializer>::len();
        debug_assert_eq!(len_result, 7);
        let _rug_ed_tests_rug_61_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_63 {
    use super::*;
    use crate::free_functions::FixedInitializer;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_63_rrrruuuugggg_test_rug = 0;
        <[usize; 8]>::len();
        let _rug_ed_tests_rug_63_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_64 {
    use super::*;
    use crate::free_functions::FixedInitializer;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5, mut rug_fuzz_6, mut rug_fuzz_7, mut rug_fuzz_8)) = <(u32, u32, u32, u32, u32, u32, u32, u32, u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: [u32; 9] = [
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
            rug_fuzz_6,
            rug_fuzz_7,
            rug_fuzz_8,
        ];
        <[u32; 9]>::as_init_slice(&p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_66 {
    use super::*;
    use crate::FixedInitializer;
    use crate::Array;
    use crate::ArrayBase;
    use crate::arr1;
    use crate::ArrayView;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: [f64; 10] = [rug_fuzz_0; 10];
        let p0_ndarray = arr1(&p0);
        <[f64; 10]>::as_init_slice(&p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_68 {
    use super::*;
    use crate::free_functions::FixedInitializer;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5, mut rug_fuzz_6, mut rug_fuzz_7, mut rug_fuzz_8, mut rug_fuzz_9, mut rug_fuzz_10)) = <(i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: [i32; 11] = [
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
            rug_fuzz_6,
            rug_fuzz_7,
            rug_fuzz_8,
            rug_fuzz_9,
            rug_fuzz_10,
        ];
        <[i32; 11] as FixedInitializer>::as_init_slice(&p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_71 {
    use super::*;
    use crate::free_functions::FixedInitializer;
    #[test]
    fn test_len() {
        let _rug_st_tests_rug_71_rrrruuuugggg_test_len = 0;
        <[(); 12] as FixedInitializer>::len();
        let _rug_ed_tests_rug_71_rrrruuuugggg_test_len = 0;
    }
}
#[cfg(test)]
mod tests_rug_72 {
    use super::*;
    use crate::free_functions::FixedInitializer;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5, mut rug_fuzz_6, mut rug_fuzz_7, mut rug_fuzz_8, mut rug_fuzz_9, mut rug_fuzz_10, mut rug_fuzz_11, mut rug_fuzz_12)) = <(i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: [i32; 13] = [
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
            rug_fuzz_6,
            rug_fuzz_7,
            rug_fuzz_8,
            rug_fuzz_9,
            rug_fuzz_10,
            rug_fuzz_11,
            rug_fuzz_12,
        ];
        <[i32; 13] as FixedInitializer>::as_init_slice(&p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_77 {
    use super::*;
    use crate::FixedInitializer;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_77_rrrruuuugggg_test_rug = 0;
        <[usize; 15] as FixedInitializer>::len();
        let _rug_ed_tests_rug_77_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_78 {
    use super::*;
    use crate::free_functions::FixedInitializer;
    use crate::free_functions;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5, mut rug_fuzz_6, mut rug_fuzz_7, mut rug_fuzz_8, mut rug_fuzz_9, mut rug_fuzz_10, mut rug_fuzz_11, mut rug_fuzz_12, mut rug_fuzz_13, mut rug_fuzz_14, mut rug_fuzz_15)) = <(u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: [u32; 16] = [
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
            rug_fuzz_6,
            rug_fuzz_7,
            rug_fuzz_8,
            rug_fuzz_9,
            rug_fuzz_10,
            rug_fuzz_11,
            rug_fuzz_12,
            rug_fuzz_13,
            rug_fuzz_14,
            rug_fuzz_15,
        ];
        <[u32; 16] as free_functions::FixedInitializer>::as_init_slice(&p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_79 {
    use super::*;
    use crate::free_functions::FixedInitializer;
    #[test]
    fn test_len() {
        let _rug_st_tests_rug_79_rrrruuuugggg_test_len = 0;
        <[usize; 16]>::len();
        let _rug_ed_tests_rug_79_rrrruuuugggg_test_len = 0;
    }
}
