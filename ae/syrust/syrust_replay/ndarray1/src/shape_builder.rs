use crate::dimension::IntoDimension;
use crate::Dimension;
use crate::{Shape, StrideShape};
/// A trait for `Shape` and `D where D: Dimension` that allows
/// customizing the memory layout (strides) of an array shape.
///
/// This trait is used together with array constructor methods like
/// `Array::from_shape_vec`.
pub trait ShapeBuilder {
    type Dim: Dimension;
    type Strides;
    fn into_shape(self) -> Shape<Self::Dim>;
    fn f(self) -> Shape<Self::Dim>;
    fn set_f(self, is_f: bool) -> Shape<Self::Dim>;
    fn strides(self, strides: Self::Strides) -> StrideShape<Self::Dim>;
}
impl<D> From<D> for Shape<D>
where
    D: Dimension,
{
    /// Create a `Shape` from `dimension`, using the default memory layout.
    fn from(dimension: D) -> Shape<D> {
        dimension.into_shape()
    }
}
impl<T, D> From<T> for StrideShape<D>
where
    D: Dimension,
    T: ShapeBuilder<Dim = D>,
{
    fn from(value: T) -> Self {
        let shape = value.into_shape();
        let d = shape.dim;
        let st = if shape.is_c { d.default_strides() } else { d.fortran_strides() };
        StrideShape {
            strides: st,
            dim: d,
            custom: false,
        }
    }
}
impl<T> ShapeBuilder for T
where
    T: IntoDimension,
{
    type Dim = T::Dim;
    type Strides = T;
    fn into_shape(self) -> Shape<Self::Dim> {
        Shape {
            dim: self.into_dimension(),
            is_c: true,
        }
    }
    fn f(self) -> Shape<Self::Dim> {
        self.set_f(true)
    }
    fn set_f(self, is_f: bool) -> Shape<Self::Dim> {
        self.into_shape().set_f(is_f)
    }
    fn strides(self, st: T) -> StrideShape<Self::Dim> {
        self.into_shape().strides(st.into_dimension())
    }
}
impl<D> ShapeBuilder for Shape<D>
where
    D: Dimension,
{
    type Dim = D;
    type Strides = D;
    fn into_shape(self) -> Shape<D> {
        self
    }
    fn f(self) -> Self {
        self.set_f(true)
    }
    fn set_f(mut self, is_f: bool) -> Self {
        self.is_c = !is_f;
        self
    }
    fn strides(self, st: D) -> StrideShape<D> {
        StrideShape {
            dim: self.dim,
            strides: st,
            custom: true,
        }
    }
}
impl<D> Shape<D>
where
    D: Dimension,
{
    /// Return the size of the shape in number of elements
    pub fn size(&self) -> usize {
        self.dim.size()
    }
}
#[cfg(test)]
mod tests_rug_519 {
    use super::*;
    use crate::prelude::{IxDyn, Dimension};
    use crate::shape_builder::Shape;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = IxDyn::zeros(rug_fuzz_0);
        <Shape<IxDyn>>::from(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_520 {
    use super::*;
    use crate::prelude::*;
    use crate::{Shape, StrideShape};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(usize, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Shape<_> = (rug_fuzz_0, rug_fuzz_1, rug_fuzz_2).into_shape().f();
        <StrideShape<_>>::from(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_521 {
    use super::*;
    use crate::prelude::ShapeBuilder;
    #[cfg(test)]
    mod tests_rug_521_prepare {
        #[test]
        fn sample() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4)) = <(i32, i32, i32, i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

            let _rug_st_tests_rug_521_rrrruuuugggg_sample = rug_fuzz_0;
            let mut v57 = std::vec::Vec::new();
            v57.push(rug_fuzz_0);
            v57.push(rug_fuzz_1);
            v57.push(rug_fuzz_2);
            let _rug_ed_tests_rug_521_rrrruuuugggg_sample = rug_fuzz_4;
             }
}
}
}        }
    }
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = vec![rug_fuzz_0, 2, 3];
        p0.into_shape();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_522 {
    use super::*;
    use crate::prelude::ShapeBuilder;
    use std::vec::Vec;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Vec::new();
        p0.push(rug_fuzz_0);
        p0.push(rug_fuzz_1);
        p0.f();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_523 {
    use super::*;
    use crate::prelude::ShapeBuilder;
    use std::vec::Vec;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(usize, usize, usize, bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut v57 = std::vec::Vec::new();
        v57.push(rug_fuzz_0);
        v57.push(rug_fuzz_1);
        v57.push(rug_fuzz_2);
        let is_f = rug_fuzz_3;
        let p0 = v57;
        let p1 = is_f;
        p0.set_f(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_524 {
    use super::*;
    use crate::prelude::ShapeBuilder;
    use std::vec::Vec;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5)) = <(usize, usize, usize, usize, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Vec<usize> = Vec::new();
        p0.push(rug_fuzz_0);
        p0.push(rug_fuzz_1);
        p0.push(rug_fuzz_2);
        let mut p1: Vec<usize> = Vec::new();
        p1.push(rug_fuzz_3);
        p1.push(rug_fuzz_4);
        p1.push(rug_fuzz_5);
        p0.strides(p1.into());
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_529 {
    use super::*;
    use crate::ShapeBuilder;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let dims = vec![rug_fuzz_0, 3, 4];
        let p0 = dims.as_slice().into_shape();
        p0.size();
             }
}
}
}    }
}
