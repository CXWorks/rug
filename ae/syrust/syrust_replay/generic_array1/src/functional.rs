//! Functional programming with generic sequences
//!
//! Please see `tests/generics.rs` for examples of how to best use these in your generic functions.
use super::ArrayLength;
use core::iter::FromIterator;
use sequence::*;
/// Defines the relationship between one generic sequence and another,
/// for operations such as `map` and `zip`.
pub unsafe trait MappedGenericSequence<T, U>: GenericSequence<T>
where
    Self::Length: ArrayLength<U>,
{
    /// Mapped sequence type
    type Mapped: GenericSequence<U, Length = Self::Length>;
}
unsafe impl<'a, T, U, S: MappedGenericSequence<T, U>> MappedGenericSequence<T, U>
for &'a S
where
    &'a S: GenericSequence<T>,
    S: GenericSequence<T, Length = <&'a S as GenericSequence<T>>::Length>,
    <S as GenericSequence<T>>::Length: ArrayLength<U>,
{
    type Mapped = <S as MappedGenericSequence<T, U>>::Mapped;
}
unsafe impl<'a, T, U, S: MappedGenericSequence<T, U>> MappedGenericSequence<T, U>
for &'a mut S
where
    &'a mut S: GenericSequence<T>,
    S: GenericSequence<T, Length = <&'a mut S as GenericSequence<T>>::Length>,
    <S as GenericSequence<T>>::Length: ArrayLength<U>,
{
    type Mapped = <S as MappedGenericSequence<T, U>>::Mapped;
}
/// Accessor type for a mapped generic sequence
pub type MappedSequence<S, T, U> = <<S as MappedGenericSequence<
    T,
    U,
>>::Mapped as GenericSequence<U>>::Sequence;
/// Defines functional programming methods for generic sequences
pub unsafe trait FunctionalSequence<T>: GenericSequence<T> {
    /// Maps a `GenericSequence` to another `GenericSequence`.
    ///
    /// If the mapping function panics, any already initialized elements in the new sequence
    /// will be dropped, AND any unused elements in the source sequence will also be dropped.
    fn map<U, F>(self, f: F) -> MappedSequence<Self, T, U>
    where
        Self: MappedGenericSequence<T, U>,
        Self::Length: ArrayLength<U>,
        F: FnMut(Self::Item) -> U,
    {
        FromIterator::from_iter(self.into_iter().map(f))
    }
    /// Combines two `GenericSequence` instances and iterates through both of them,
    /// initializing a new `GenericSequence` with the result of the zipped mapping function.
    ///
    /// If the mapping function panics, any already initialized elements in the new sequence
    /// will be dropped, AND any unused elements in the source sequences will also be dropped.
    #[inline]
    fn zip<B, Rhs, U, F>(self, rhs: Rhs, f: F) -> MappedSequence<Self, T, U>
    where
        Self: MappedGenericSequence<T, U>,
        Rhs: MappedGenericSequence<B, U, Mapped = MappedSequence<Self, T, U>>,
        Self::Length: ArrayLength<B> + ArrayLength<U>,
        Rhs: GenericSequence<B, Length = Self::Length>,
        F: FnMut(Self::Item, Rhs::Item) -> U,
    {
        rhs.inverted_zip2(self, f)
    }
    /// Folds (or reduces) a sequence of data into a single value.
    ///
    /// If the fold function panics, any unused elements will be dropped.
    fn fold<U, F>(self, init: U, f: F) -> U
    where
        F: FnMut(U, Self::Item) -> U,
    {
        self.into_iter().fold(init, f)
    }
}
unsafe impl<'a, T, S: GenericSequence<T>> FunctionalSequence<T> for &'a S
where
    &'a S: GenericSequence<T>,
{}
unsafe impl<'a, T, S: GenericSequence<T>> FunctionalSequence<T> for &'a mut S
where
    &'a mut S: GenericSequence<T>,
{}
#[cfg(test)]
mod tests_rug_31 {
    use crate::arr;
    use crate::typenum::{U4, U8};
    use crate::GenericArray;
    use super::*;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: GenericArray<u32, U8> = arr![u32; 1, 2, 3, 4, 5, 6, 7, 8];
        let mut p1 = |item: u32| item * rug_fuzz_0;
        crate::functional::FunctionalSequence::map(p0, &mut p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_32 {
    use super::*;
    use crate::GenericArray;
    use crate::typenum::U4;
    use crate::functional::{MappedGenericSequence, GenericSequence};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: GenericArray<u32, U4> = GenericArray::generate(|i| i as u32);
        let p1: GenericArray<u32, U4> = GenericArray::generate(|i| {
            i as u32 * rug_fuzz_0
        });
        let p2 = |x: u32, y: u32| x + y;
        crate::functional::FunctionalSequence::zip(p0, p1, p2);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_33 {
    use super::*;
    use crate::arr;
    use crate::typenum::U8;
    use crate::GenericArray;
    #[test]
    fn test_fold() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: GenericArray<u32, U8> = arr![u32; 1, 2, 3, 4, 5, 6, 7, 8];
        let p1: u32 = rug_fuzz_0;
        let p2 = |acc: u32, x: u32| -> u32 { acc + x };
        p0.fold(p1, p2);
             }
}
}
}    }
}
