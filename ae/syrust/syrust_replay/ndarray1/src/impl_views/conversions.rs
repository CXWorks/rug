use std::slice;
use crate::imp_prelude::*;
use crate::{Baseiter, ElementsBase, ElementsBaseMut, Iter, IterMut};
use crate::iter::{self, AxisIter, AxisIterMut};
use crate::IndexLonger;
/// Methods for read-only array views.
impl<'a, A, D> ArrayView<'a, A, D>
where
    D: Dimension,
{
    /// Convert the view into an `ArrayView<'b, A, D>` where `'b` is a lifetime
    /// outlived by `'a'`.
    pub fn reborrow<'b>(self) -> ArrayView<'b, A, D>
    where
        'a: 'b,
    {
        unsafe { ArrayView::new(self.ptr, self.dim, self.strides) }
    }
    /// Return the array’s data as a slice, if it is contiguous and in standard order.
    /// Return `None` otherwise.
    #[deprecated(note = "`into_slice` has been renamed to `to_slice`", since = "0.13.0")]
    #[allow(clippy::wrong_self_convention)]
    pub fn into_slice(&self) -> Option<&'a [A]> {
        if self.is_standard_layout() {
            unsafe { Some(slice::from_raw_parts(self.ptr.as_ptr(), self.len())) }
        } else {
            None
        }
    }
    /// Return the array’s data as a slice, if it is contiguous and in standard order.
    /// Return `None` otherwise.
    pub fn to_slice(&self) -> Option<&'a [A]> {
        if self.is_standard_layout() {
            unsafe { Some(slice::from_raw_parts(self.ptr.as_ptr(), self.len())) }
        } else {
            None
        }
    }
    /// Converts to a raw array view.
    pub(crate) fn into_raw_view(self) -> RawArrayView<A, D> {
        unsafe { RawArrayView::new(self.ptr, self.dim, self.strides) }
    }
}
/// Methods specific to `ArrayView0`.
///
/// ***See also all methods for [`ArrayView`] and [`ArrayBase`]***
///
/// [`ArrayBase`]: struct.ArrayBase.html
/// [`ArrayView`]: struct.ArrayView.html
impl<'a, A> ArrayView<'a, A, Ix0> {
    /// Consume the view and return a reference to the single element in the array.
    ///
    /// The lifetime of the returned reference matches the lifetime of the data
    /// the array view was pointing to.
    ///
    /// ```
    /// use ndarray::{arr0, Array0};
    ///
    /// // `Foo` doesn't implement `Clone`.
    /// #[derive(Debug, Eq, PartialEq)]
    /// struct Foo;
    ///
    /// let array: Array0<Foo> = arr0(Foo);
    /// let view = array.view();
    /// let scalar: &Foo = view.into_scalar();
    /// assert_eq!(scalar, &Foo);
    /// ```
    pub fn into_scalar(self) -> &'a A {
        self.index(Ix0())
    }
}
/// Methods specific to `ArrayViewMut0`.
///
/// ***See also all methods for [`ArrayViewMut`] and [`ArrayBase`]***
///
/// [`ArrayBase`]: struct.ArrayBase.html
/// [`ArrayViewMut`]: struct.ArrayViewMut.html
impl<'a, A> ArrayViewMut<'a, A, Ix0> {
    /// Consume the mutable view and return a mutable reference to the single element in the array.
    ///
    /// The lifetime of the returned reference matches the lifetime of the data
    /// the array view was pointing to.
    ///
    /// ```
    /// use ndarray::{arr0, Array0};
    ///
    /// let mut array: Array0<f64> = arr0(5.);
    /// let view = array.view_mut();
    /// let mut scalar = view.into_scalar();
    /// *scalar = 7.;
    /// assert_eq!(scalar, &7.);
    /// assert_eq!(array[()], 7.);
    /// ```
    pub fn into_scalar(self) -> &'a mut A {
        self.index(Ix0())
    }
}
/// Methods for read-write array views.
impl<'a, A, D> ArrayViewMut<'a, A, D>
where
    D: Dimension,
{
    /// Return the array’s data as a slice, if it is contiguous and in standard order.
    /// Return `None` otherwise.
    pub fn into_slice(self) -> Option<&'a mut [A]> {
        self.into_slice_().ok()
    }
}
/// Private array view methods
impl<'a, A, D> ArrayView<'a, A, D>
where
    D: Dimension,
{
    #[inline]
    pub(crate) fn into_base_iter(self) -> Baseiter<A, D> {
        unsafe { Baseiter::new(self.ptr.as_ptr(), self.dim, self.strides) }
    }
    #[inline]
    pub(crate) fn into_elements_base(self) -> ElementsBase<'a, A, D> {
        ElementsBase::new(self)
    }
    pub(crate) fn into_iter_(self) -> Iter<'a, A, D> {
        Iter::new(self)
    }
    /// Return an outer iterator for this view.
    #[doc(hidden)]
    #[deprecated(note = "This method will be replaced.")]
    pub fn into_outer_iter(self) -> iter::AxisIter<'a, A, D::Smaller>
    where
        D: RemoveAxis,
    {
        AxisIter::new(self, Axis(0))
    }
}
impl<'a, A, D> ArrayViewMut<'a, A, D>
where
    D: Dimension,
{
    pub(crate) fn into_view(self) -> ArrayView<'a, A, D> {
        unsafe { ArrayView::new(self.ptr, self.dim, self.strides) }
    }
    /// Converts to a mutable raw array view.
    pub(crate) fn into_raw_view_mut(self) -> RawArrayViewMut<A, D> {
        unsafe { RawArrayViewMut::new(self.ptr, self.dim, self.strides) }
    }
    #[inline]
    pub(crate) fn into_base_iter(self) -> Baseiter<A, D> {
        unsafe { Baseiter::new(self.ptr.as_ptr(), self.dim, self.strides) }
    }
    #[inline]
    pub(crate) fn into_elements_base(self) -> ElementsBaseMut<'a, A, D> {
        ElementsBaseMut::new(self)
    }
    pub(crate) fn into_slice_(self) -> Result<&'a mut [A], Self> {
        if self.is_standard_layout() {
            unsafe { Ok(slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len())) }
        } else {
            Err(self)
        }
    }
    pub(crate) fn into_iter_(self) -> IterMut<'a, A, D> {
        IterMut::new(self)
    }
    /// Return an outer iterator for this view.
    #[doc(hidden)]
    #[deprecated(note = "This method will be replaced.")]
    pub fn into_outer_iter(self) -> iter::AxisIterMut<'a, A, D::Smaller>
    where
        D: RemoveAxis,
    {
        AxisIterMut::new(self, Axis(0))
    }
}
#[cfg(test)]
mod tests_rug_1473 {
    use super::*;
    use crate::{ArrayBase, Data, ArrayView, Array1, ViewRepr};
    use std::slice;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5, mut rug_fuzz_6, mut rug_fuzz_7)) = <(i32, i32, i32, i32, i32, i32, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let data: [i32; 6] = [
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
        ];
        let arr = ArrayView::from_shape((rug_fuzz_6, rug_fuzz_7), &data).unwrap();
        let p0: ArrayBase<ViewRepr<&i32>, _> = arr.view();
        p0.to_slice();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1475 {
    use super::*;
    use crate::{ArrayBase, ViewRepr, Ix0, Array0};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_1475_rrrruuuugggg_test_rug = 0;
        #[derive(Debug, Eq, PartialEq)]
        struct Foo;
        let array: Array0<Foo> = arr0(Foo);
        let view = array.view();
        let scalar: &Foo = view.into_scalar();
        debug_assert_eq!(scalar, & Foo);
        let _rug_ed_tests_rug_1475_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_1477 {
    use crate::{ArrayBase, ViewRepr, Array};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(i32, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut array_data = vec![rug_fuzz_0, 2, 3, 4, 5, 6];
        let mut array = Array::from_shape_vec((rug_fuzz_1, rug_fuzz_2), array_data)
            .unwrap();
        let mut v23 = array.view_mut();
        v23.into_slice();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1479 {
    use super::*;
    use crate::{ArrayBase, ViewRepr, Data, Array};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(f32, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let data: Vec<f32> = vec![rug_fuzz_0, 2.0, 3.0, 4.0];
        let array: Array<f32, _> = ArrayBase::from_shape_vec(
                (rug_fuzz_1, rug_fuzz_2),
                data,
            )
            .unwrap();
        let p0 = array.view();
        p0.into_elements_base();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1482 {
    use super::*;
    use crate::{ArrayBase, Array, ViewRepr};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(i32, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut array_data = vec![rug_fuzz_0, 2, 3, 4, 5, 6];
        let mut array = Array::from_shape_vec((rug_fuzz_1, rug_fuzz_2), array_data)
            .unwrap();
        let mut v23 = array.view_mut();
        v23.into_view();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1483 {
    use super::*;
    use crate::{ArrayBase, ViewRepr, Array, ArrayViewMut};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(i32, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut array_data = vec![rug_fuzz_0, 2, 3, 4, 5, 6];
        let mut array = Array::from_shape_vec((rug_fuzz_1, rug_fuzz_2), array_data)
            .unwrap();
        let mut v23 = array.view_mut();
        let p0: ArrayBase<ViewRepr<&mut i32>, _> = v23;
        p0.into_raw_view_mut();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1484 {
    use super::*;
    use crate::{ArrayBase, ViewRepr, Array};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(i32, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut array_data = vec![rug_fuzz_0, 2, 3, 4, 5, 6];
        let mut array = Array::from_shape_vec((rug_fuzz_1, rug_fuzz_2), array_data)
            .unwrap();
        let mut v23 = array.view_mut();
        v23.into_base_iter();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1485 {
    use super::*;
    use crate::{ArrayBase, ViewRepr, Array};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(i32, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut array_data = vec![rug_fuzz_0, 2, 3, 4, 5, 6];
        let mut array = Array::from_shape_vec((rug_fuzz_1, rug_fuzz_2), array_data)
            .unwrap();
        let mut v23 = array.view_mut();
        ArrayBase::<ViewRepr<&mut i32>, _>::into_elements_base(v23);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1486 {
    use super::*;
    use crate::{ArrayBase, ViewRepr};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(i32, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut array_data = vec![rug_fuzz_0, 2, 3, 4, 5, 6];
        let mut array = Array::from_shape_vec((rug_fuzz_1, rug_fuzz_2), array_data)
            .unwrap();
        let mut v23 = array.view_mut();
        v23.into_slice_();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1487 {
    use super::*;
    use crate::{ArrayBase, ViewRepr, Array};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(i32, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut array_data = vec![rug_fuzz_0, 2, 3, 4, 5, 6];
        let mut array = Array::from_shape_vec((rug_fuzz_1, rug_fuzz_2), array_data)
            .unwrap();
        let mut v23 = array.view_mut();
        <ArrayBase<ViewRepr<&mut i32>, _>>::into_iter_(v23);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1488 {
    use super::*;
    use crate::{ArrayBase, ViewRepr, Axis, Array};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(i32, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut array_data = vec![rug_fuzz_0, 2, 3, 4, 5, 6];
        let mut array = Array::from_shape_vec((rug_fuzz_1, rug_fuzz_2), array_data)
            .unwrap();
        let mut v23 = array.view_mut();
        v23.into_outer_iter();
             }
}
}
}    }
}
