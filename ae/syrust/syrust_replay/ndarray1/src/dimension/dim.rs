use std::fmt;
use super::Dimension;
use super::IntoDimension;
use crate::itertools::zip;
use crate::Ix;
/// Dimension description.
///
/// `Dim` describes the number of axes and the length of each axis
/// in an array. It is also used as an index type.
///
/// See also the [`Dimension` trait](trait.Dimension.html) for its methods and
/// operations.
///
/// # Examples
///
/// To create an array with a particular dimension, you'd just pass
/// a tuple (in this example (3, 2) is used), which is converted to
/// `Dim` by the array constructor.
///
/// ```
/// use ndarray::Array2;
/// use ndarray::Dim;
///
/// let mut array = Array2::zeros((3, 2));
/// array[[0, 0]] = 1.;
/// assert_eq!(array.raw_dim(), Dim([3, 2]));
/// ```
#[derive(Copy, Clone, PartialEq, Eq, Hash, Default)]
pub struct Dim<I: ?Sized> {
    index: I,
}
impl<I> Dim<I> {
    /// Private constructor and accessors for Dim
    pub(crate) fn new(index: I) -> Dim<I> {
        Dim { index }
    }
    #[inline(always)]
    pub(crate) fn ix(&self) -> &I {
        &self.index
    }
    #[inline(always)]
    pub(crate) fn ixm(&mut self) -> &mut I {
        &mut self.index
    }
}
/// Create a new dimension value.
#[allow(non_snake_case)]
pub fn Dim<T>(index: T) -> T::Dim
where
    T: IntoDimension,
{
    index.into_dimension()
}
impl<I: ?Sized> PartialEq<I> for Dim<I>
where
    I: PartialEq,
{
    fn eq(&self, rhs: &I) -> bool {
        self.index == *rhs
    }
}
impl<I> fmt::Debug for Dim<I>
where
    I: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.index)
    }
}
use std::ops::{Add, AddAssign, Mul, MulAssign, Sub, SubAssign};
macro_rules! impl_op {
    ($op:ident, $op_m:ident, $opassign:ident, $opassign_m:ident, $expr:ident) => {
        impl < I > $op for Dim < I > where Dim < I >: Dimension, { type Output = Self; fn
        $op_m (mut self, rhs : Self) -> Self { $expr ! (self, & rhs); self } } impl < I >
        $opassign for Dim < I > where Dim < I >: Dimension, { fn $opassign_m (& mut self,
        rhs : Self) { $expr ! (* self, & rhs); } } impl <'a, I > $opassign <&'a Dim < I
        >> for Dim < I > where Dim < I >: Dimension, { fn $opassign_m (& mut self, rhs :
        & Self) { for (x, & y) in zip(self.slice_mut(), rhs.slice()) { $expr ! (* x, y);
        } } }
    };
}
macro_rules! impl_single_op {
    ($op:ident, $op_m:ident, $opassign:ident, $opassign_m:ident, $expr:ident) => {
        impl $op < Ix > for Dim < [Ix; 1] > { type Output = Self; #[inline] fn $op_m (mut
        self, rhs : Ix) -> Self { $expr ! (self, rhs); self } } impl $opassign < Ix > for
        Dim < [Ix; 1] > { #[inline] fn $opassign_m (& mut self, rhs : Ix) { $expr ! ((*
        self) [0], rhs); } }
    };
}
macro_rules! impl_scalar_op {
    ($op:ident, $op_m:ident, $opassign:ident, $opassign_m:ident, $expr:ident) => {
        impl < I > $op < Ix > for Dim < I > where Dim < I >: Dimension, { type Output =
        Self; fn $op_m (mut self, rhs : Ix) -> Self { $expr ! (self, rhs); self } } impl
        < I > $opassign < Ix > for Dim < I > where Dim < I >: Dimension, { fn $opassign_m
        (& mut self, rhs : Ix) { for x in self.slice_mut() { $expr ! (* x, rhs); } } }
    };
}
macro_rules! add {
    ($x:expr, $y:expr) => {
        $x += $y;
    };
}
macro_rules! sub {
    ($x:expr, $y:expr) => {
        $x -= $y;
    };
}
macro_rules! mul {
    ($x:expr, $y:expr) => {
        $x *= $y;
    };
}
impl_op!(Add, add, AddAssign, add_assign, add);
impl_single_op!(Add, add, AddAssign, add_assign, add);
impl_op!(Sub, sub, SubAssign, sub_assign, sub);
impl_single_op!(Sub, sub, SubAssign, sub_assign, sub);
impl_op!(Mul, mul, MulAssign, mul_assign, mul);
impl_scalar_op!(Mul, mul, MulAssign, mul_assign, mul);
#[cfg(test)]
mod tests_rug_259 {
    use super::*;
    use std::vec::Vec;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = vec![rug_fuzz_0, 2, 3];
        crate::dimension::dim::Dim(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_260 {
    use super::*;
    use crate::dimension::dim::Dim;
    #[test]
    fn test_dim_new() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: usize = rug_fuzz_0;
        Dim::<usize>::new(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_264 {
    use super::*;
    use crate::prelude::{IxDyn, Dimension};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = IxDyn::zeros(rug_fuzz_0);
        let mut p1 = IxDyn::zeros(rug_fuzz_1);
        p0.add(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_270 {
    use super::*;
    use crate::prelude::{IxDyn, Dimension};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = IxDyn::zeros(rug_fuzz_0);
        let mut p1 = IxDyn::zeros(rug_fuzz_1);
        p0.sub_assign(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_274 {
    use super::*;
    use crate::prelude::{IxDyn, Dimension};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = IxDyn::zeros(rug_fuzz_0);
        let mut p1 = IxDyn::zeros(rug_fuzz_1);
        p0.mul(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_275 {
    use super::*;
    use crate::prelude::{IxDyn, Dimension};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = IxDyn::zeros(rug_fuzz_0);
        let mut p1 = IxDyn::zeros(rug_fuzz_1);
        p0.mul_assign(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_277 {
    use super::*;
    use crate::prelude::{IxDyn, Dimension};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = IxDyn::zeros(rug_fuzz_0);
        let p1: usize = rug_fuzz_1;
        p0.mul(p1);
             }
}
}
}    }
}
