//! This crate implements a structure that can be used as a generic array type.use
//! Core Rust array types `[T; N]` can't be used generically with
//! respect to `N`, so for example this:
//!
//! ```{should_fail}
//! struct Foo<T, N> {
//!     data: [T; N]
//! }
//! ```
//!
//! won't work.
//!
//! **generic-array** exports a `GenericArray<T,N>` type, which lets
//! the above be implemented as:
//!
//! ```
//! # use generic_array::{ArrayLength, GenericArray};
//! struct Foo<T, N: ArrayLength<T>> {
//!     data: GenericArray<T,N>
//! }
//! ```
//!
//! The `ArrayLength<T>` trait is implemented by default for
//! [unsigned integer types](../typenum/uint/index.html) from
//! [typenum](../typenum/index.html).
//!
//! For ease of use, an `arr!` macro is provided - example below:
//!
//! ```
//! # #[macro_use]
//! # extern crate generic_array;
//! # extern crate typenum;
//! # fn main() {
//! let array = arr![u32; 1, 2, 3];
//! assert_eq!(array[2], 3);
//! # }
//! ```
extern crate core;
#[cfg(feature = "serde")]
extern crate serde;
#[cfg(test)]
extern crate bincode;
pub extern crate typenum;
mod hex;
mod impls;
#[cfg(feature = "serde")]
pub mod impl_serde;
use core::iter::FromIterator;
use core::marker::PhantomData;
use core::mem::ManuallyDrop;
use core::ops::{Deref, DerefMut};
use core::{mem, ptr, slice};
use typenum::bit::{B0, B1};
use typenum::uint::{UInt, UTerm, Unsigned};
#[cfg_attr(test, macro_use)]
pub mod arr;
pub mod functional;
pub mod iter;
pub mod sequence;
use functional::*;
pub use iter::GenericArrayIter;
use sequence::*;
/// Trait making `GenericArray` work, marking types to be used as length of an array
pub unsafe trait ArrayLength<T>: Unsigned {
    /// Associated type representing the array type for the number
    type ArrayType;
}
unsafe impl<T> ArrayLength<T> for UTerm {
    #[doc(hidden)]
    type ArrayType = ();
}
/// Internal type used to generate a struct of appropriate size
#[allow(dead_code)]
#[repr(C)]
#[doc(hidden)]
pub struct GenericArrayImplEven<T, U> {
    parent1: U,
    parent2: U,
    _marker: PhantomData<T>,
}
impl<T: Clone, U: Clone> Clone for GenericArrayImplEven<T, U> {
    fn clone(&self) -> GenericArrayImplEven<T, U> {
        GenericArrayImplEven {
            parent1: self.parent1.clone(),
            parent2: self.parent2.clone(),
            _marker: PhantomData,
        }
    }
}
impl<T: Copy, U: Copy> Copy for GenericArrayImplEven<T, U> {}
/// Internal type used to generate a struct of appropriate size
#[allow(dead_code)]
#[repr(C)]
#[doc(hidden)]
pub struct GenericArrayImplOdd<T, U> {
    parent1: U,
    parent2: U,
    data: T,
}
impl<T: Clone, U: Clone> Clone for GenericArrayImplOdd<T, U> {
    fn clone(&self) -> GenericArrayImplOdd<T, U> {
        GenericArrayImplOdd {
            parent1: self.parent1.clone(),
            parent2: self.parent2.clone(),
            data: self.data.clone(),
        }
    }
}
impl<T: Copy, U: Copy> Copy for GenericArrayImplOdd<T, U> {}
unsafe impl<T, N: ArrayLength<T>> ArrayLength<T> for UInt<N, B0> {
    #[doc(hidden)]
    type ArrayType = GenericArrayImplEven<T, N::ArrayType>;
}
unsafe impl<T, N: ArrayLength<T>> ArrayLength<T> for UInt<N, B1> {
    #[doc(hidden)]
    type ArrayType = GenericArrayImplOdd<T, N::ArrayType>;
}
/// Struct representing a generic array - `GenericArray<T, N>` works like [T; N]
#[allow(dead_code)]
pub struct GenericArray<T, U: ArrayLength<T>> {
    data: U::ArrayType,
}
unsafe impl<T: Send, N: ArrayLength<T>> Send for GenericArray<T, N> {}
unsafe impl<T: Sync, N: ArrayLength<T>> Sync for GenericArray<T, N> {}
impl<T, N> Deref for GenericArray<T, N>
where
    N: ArrayLength<T>,
{
    type Target = [T];
    fn deref(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self as *const Self as *const T, N::to_usize()) }
    }
}
impl<T, N> DerefMut for GenericArray<T, N>
where
    N: ArrayLength<T>,
{
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self as *mut Self as *mut T, N::to_usize()) }
    }
}
/// Creates an array one element at a time using a mutable iterator
/// you can write to with `ptr::write`.
///
/// Incremenent the position while iterating to mark off created elements,
/// which will be dropped if `into_inner` is not called.
#[doc(hidden)]
pub struct ArrayBuilder<T, N: ArrayLength<T>> {
    array: ManuallyDrop<GenericArray<T, N>>,
    position: usize,
}
impl<T, N: ArrayLength<T>> ArrayBuilder<T, N> {
    #[doc(hidden)]
    #[inline]
    pub unsafe fn new() -> ArrayBuilder<T, N> {
        ArrayBuilder {
            array: ManuallyDrop::new(mem::uninitialized()),
            position: 0,
        }
    }
    /// Creates a mutable iterator for writing to the array using `ptr::write`.
    ///
    /// Increment the position value given as a mutable reference as you iterate
    /// to mark how many elements have been created.
    #[doc(hidden)]
    #[inline]
    pub unsafe fn iter_position(&mut self) -> (slice::IterMut<T>, &mut usize) {
        (self.array.iter_mut(), &mut self.position)
    }
    /// When done writing (assuming all elements have been written to),
    /// get the inner array.
    #[doc(hidden)]
    #[inline]
    pub unsafe fn into_inner(self) -> GenericArray<T, N> {
        let array = ptr::read(&self.array);
        mem::forget(self);
        ManuallyDrop::into_inner(array)
    }
}
impl<T, N: ArrayLength<T>> Drop for ArrayBuilder<T, N> {
    fn drop(&mut self) {
        for value in &mut self.array[..self.position] {
            unsafe {
                ptr::drop_in_place(value);
            }
        }
    }
}
/// Consumes an array.
///
/// Increment the position while iterating and any leftover elements
/// will be dropped if position does not go to N
#[doc(hidden)]
pub struct ArrayConsumer<T, N: ArrayLength<T>> {
    array: ManuallyDrop<GenericArray<T, N>>,
    position: usize,
}
impl<T, N: ArrayLength<T>> ArrayConsumer<T, N> {
    #[doc(hidden)]
    #[inline]
    pub unsafe fn new(array: GenericArray<T, N>) -> ArrayConsumer<T, N> {
        ArrayConsumer {
            array: ManuallyDrop::new(array),
            position: 0,
        }
    }
    /// Creates an iterator and mutable reference to the internal position
    /// to keep track of consumed elements.
    ///
    /// Increment the position as you iterate to mark off consumed elements
    #[doc(hidden)]
    #[inline]
    pub unsafe fn iter_position(&mut self) -> (slice::Iter<T>, &mut usize) {
        (self.array.iter(), &mut self.position)
    }
}
impl<T, N: ArrayLength<T>> Drop for ArrayConsumer<T, N> {
    fn drop(&mut self) {
        for value in &mut self.array[self.position..N::to_usize()] {
            unsafe {
                ptr::drop_in_place(value);
            }
        }
    }
}
impl<'a, T: 'a, N> IntoIterator for &'a GenericArray<T, N>
where
    N: ArrayLength<T>,
{
    type IntoIter = slice::Iter<'a, T>;
    type Item = &'a T;
    fn into_iter(self: &'a GenericArray<T, N>) -> Self::IntoIter {
        self.as_slice().iter()
    }
}
impl<'a, T: 'a, N> IntoIterator for &'a mut GenericArray<T, N>
where
    N: ArrayLength<T>,
{
    type IntoIter = slice::IterMut<'a, T>;
    type Item = &'a mut T;
    fn into_iter(self: &'a mut GenericArray<T, N>) -> Self::IntoIter {
        self.as_mut_slice().iter_mut()
    }
}
impl<T, N> FromIterator<T> for GenericArray<T, N>
where
    N: ArrayLength<T>,
{
    fn from_iter<I>(iter: I) -> GenericArray<T, N>
    where
        I: IntoIterator<Item = T>,
    {
        unsafe {
            let mut destination = ArrayBuilder::new();
            {
                let (destination_iter, position) = destination.iter_position();
                for (src, dst) in iter.into_iter().zip(destination_iter) {
                    ptr::write(dst, src);
                    *position += 1;
                }
            }
            if destination.position < N::to_usize() {
                from_iter_length_fail(destination.position, N::to_usize());
            }
            destination.into_inner()
        }
    }
}
#[inline(never)]
#[cold]
fn from_iter_length_fail(length: usize, expected: usize) -> ! {
    panic!(
        "GenericArray::from_iter received {} elements but expected {}", length, expected
    );
}
unsafe impl<T, N> GenericSequence<T> for GenericArray<T, N>
where
    N: ArrayLength<T>,
    Self: IntoIterator<Item = T>,
{
    type Length = N;
    type Sequence = Self;
    fn generate<F>(mut f: F) -> GenericArray<T, N>
    where
        F: FnMut(usize) -> T,
    {
        unsafe {
            let mut destination = ArrayBuilder::new();
            {
                let (destination_iter, position) = destination.iter_position();
                for (i, dst) in destination_iter.enumerate() {
                    ptr::write(dst, f(i));
                    *position += 1;
                }
            }
            destination.into_inner()
        }
    }
    #[doc(hidden)]
    fn inverted_zip<B, U, F>(
        self,
        lhs: GenericArray<B, Self::Length>,
        mut f: F,
    ) -> MappedSequence<GenericArray<B, Self::Length>, B, U>
    where
        GenericArray<
            B,
            Self::Length,
        >: GenericSequence<B, Length = Self::Length> + MappedGenericSequence<B, U>,
        Self: MappedGenericSequence<T, U>,
        Self::Length: ArrayLength<B> + ArrayLength<U>,
        F: FnMut(B, Self::Item) -> U,
    {
        unsafe {
            let mut left = ArrayConsumer::new(lhs);
            let mut right = ArrayConsumer::new(self);
            let (left_array_iter, left_position) = left.iter_position();
            let (right_array_iter, right_position) = right.iter_position();
            FromIterator::from_iter(
                left_array_iter
                    .zip(right_array_iter)
                    .map(|(l, r)| {
                        let left_value = ptr::read(l);
                        let right_value = ptr::read(r);
                        *left_position += 1;
                        *right_position += 1;
                        f(left_value, right_value)
                    }),
            )
        }
    }
    #[doc(hidden)]
    fn inverted_zip2<B, Lhs, U, F>(self, lhs: Lhs, mut f: F) -> MappedSequence<Lhs, B, U>
    where
        Lhs: GenericSequence<B, Length = Self::Length> + MappedGenericSequence<B, U>,
        Self: MappedGenericSequence<T, U>,
        Self::Length: ArrayLength<B> + ArrayLength<U>,
        F: FnMut(Lhs::Item, Self::Item) -> U,
    {
        unsafe {
            let mut right = ArrayConsumer::new(self);
            let (right_array_iter, right_position) = right.iter_position();
            FromIterator::from_iter(
                lhs
                    .into_iter()
                    .zip(right_array_iter)
                    .map(|(left_value, r)| {
                        let right_value = ptr::read(r);
                        *right_position += 1;
                        f(left_value, right_value)
                    }),
            )
        }
    }
}
unsafe impl<T, U, N> MappedGenericSequence<T, U> for GenericArray<T, N>
where
    N: ArrayLength<T> + ArrayLength<U>,
    GenericArray<U, N>: GenericSequence<U, Length = N>,
{
    type Mapped = GenericArray<U, N>;
}
unsafe impl<T, N> FunctionalSequence<T> for GenericArray<T, N>
where
    N: ArrayLength<T>,
    Self: GenericSequence<T, Item = T, Length = N>,
{
    fn map<U, F>(self, mut f: F) -> MappedSequence<Self, T, U>
    where
        Self::Length: ArrayLength<U>,
        Self: MappedGenericSequence<T, U>,
        F: FnMut(T) -> U,
    {
        unsafe {
            let mut source = ArrayConsumer::new(self);
            let (array_iter, position) = source.iter_position();
            FromIterator::from_iter(
                array_iter
                    .map(|src| {
                        let value = ptr::read(src);
                        *position += 1;
                        f(value)
                    }),
            )
        }
    }
    #[inline]
    fn zip<B, Rhs, U, F>(self, rhs: Rhs, f: F) -> MappedSequence<Self, T, U>
    where
        Self: MappedGenericSequence<T, U>,
        Rhs: MappedGenericSequence<B, U, Mapped = MappedSequence<Self, T, U>>,
        Self::Length: ArrayLength<B> + ArrayLength<U>,
        Rhs: GenericSequence<B, Length = Self::Length>,
        F: FnMut(T, Rhs::Item) -> U,
    {
        rhs.inverted_zip(self, f)
    }
    fn fold<U, F>(self, init: U, mut f: F) -> U
    where
        F: FnMut(U, T) -> U,
    {
        unsafe {
            let mut source = ArrayConsumer::new(self);
            let (array_iter, position) = source.iter_position();
            array_iter
                .fold(
                    init,
                    |acc, src| {
                        let value = ptr::read(src);
                        *position += 1;
                        f(acc, value)
                    },
                )
        }
    }
}
impl<T, N> GenericArray<T, N>
where
    N: ArrayLength<T>,
{
    /// Extracts a slice containing the entire array.
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        self.deref()
    }
    /// Extracts a mutable slice containing the entire array.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self.deref_mut()
    }
    /// Converts slice to a generic array reference with inferred length;
    ///
    /// Length of the slice must be equal to the length of the array.
    #[inline]
    pub fn from_slice(slice: &[T]) -> &GenericArray<T, N> {
        slice.into()
    }
    /// Converts mutable slice to a mutable generic array reference
    ///
    /// Length of the slice must be equal to the length of the array.
    #[inline]
    pub fn from_mut_slice(slice: &mut [T]) -> &mut GenericArray<T, N> {
        slice.into()
    }
}
impl<'a, T, N: ArrayLength<T>> From<&'a [T]> for &'a GenericArray<T, N> {
    /// Converts slice to a generic array reference with inferred length;
    ///
    /// Length of the slice must be equal to the length of the array.
    #[inline]
    fn from(slice: &[T]) -> &GenericArray<T, N> {
        assert_eq!(slice.len(), N::to_usize());
        unsafe { &*(slice.as_ptr() as *const GenericArray<T, N>) }
    }
}
impl<'a, T, N: ArrayLength<T>> From<&'a mut [T]> for &'a mut GenericArray<T, N> {
    /// Converts mutable slice to a mutable generic array reference
    ///
    /// Length of the slice must be equal to the length of the array.
    #[inline]
    fn from(slice: &mut [T]) -> &mut GenericArray<T, N> {
        assert_eq!(slice.len(), N::to_usize());
        unsafe { &mut *(slice.as_mut_ptr() as *mut GenericArray<T, N>) }
    }
}
impl<T: Clone, N> GenericArray<T, N>
where
    N: ArrayLength<T>,
{
    /// Construct a `GenericArray` from a slice by cloning its content
    ///
    /// Length of the slice must be equal to the length of the array
    #[inline]
    pub fn clone_from_slice(list: &[T]) -> GenericArray<T, N> {
        Self::from_exact_iter(list.iter().cloned())
            .expect("Slice must be the same length as the array")
    }
}
impl<T, N> GenericArray<T, N>
where
    N: ArrayLength<T>,
{
    /// Creates a new `GenericArray` instance from an iterator with a known exact size.
    ///
    /// Returns `None` if the size is not equal to the number of elements in the `GenericArray`.
    pub fn from_exact_iter<I>(iter: I) -> Option<Self>
    where
        I: IntoIterator<Item = T>,
        <I as IntoIterator>::IntoIter: ExactSizeIterator,
    {
        let iter = iter.into_iter();
        if iter.len() == N::to_usize() {
            unsafe {
                let mut destination = ArrayBuilder::new();
                {
                    let (destination_iter, position) = destination.iter_position();
                    for (dst, src) in destination_iter.zip(iter.into_iter()) {
                        ptr::write(dst, src);
                        *position += 1;
                    }
                }
                Some(destination.into_inner())
            }
        } else {
            None
        }
    }
}
/// A reimplementation of the `transmute` function, avoiding problems
/// when the compiler can't prove equal sizes.
#[inline]
#[doc(hidden)]
pub unsafe fn transmute<A, B>(a: A) -> B {
    let b = ::core::ptr::read(&a as *const A as *const B);
    ::core::mem::forget(a);
    b
}
#[cfg(test)]
mod test {
    #[inline(never)]
    pub fn black_box<T>(val: T) -> T {
        use core::{mem, ptr};
        let ret = unsafe { ptr::read_volatile(&val) };
        mem::forget(val);
        ret
    }
    #[test]
    fn test_assembly() {
        use functional::*;
        let a = black_box(arr![i32; 1, 3, 5, 7]);
        let b = black_box(arr![i32; 2, 4, 6, 8]);
        let c = (&a).zip(b, |l, r| l + r);
        let d = a.fold(0, |a, x| a + x);
        assert_eq!(c, arr![i32; 3, 7, 11, 15]);
        assert_eq!(d, 16);
    }
}
#[cfg(test)]
mod tests_rug_1 {
    use super::*;
    #[test]
    fn test_from_iter_length_fail() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: usize = rug_fuzz_0;
        let mut p1: usize = rug_fuzz_1;
        crate::from_iter_length_fail(p0, p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_3 {
    use super::*;
    use crate::core::clone::Clone;
    use crate::core::marker::PhantomData;
    #[test]
    fn test_clone() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0 = GenericArrayImplEven::<u32, usize> {
            parent1: rug_fuzz_0,
            parent2: rug_fuzz_1,
            _marker: PhantomData,
        };
        <GenericArrayImplEven<u32, usize> as core::clone::Clone>::clone(&p0);
             }
});    }
}
#[cfg(test)]
mod tests_rug_5 {
    use super::*;
    use crate::core::ops::Deref;
    use crate::GenericArray;
    use crate::typenum::U3;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_5_rrrruuuugggg_test_rug = 0;
        let mut p0: GenericArray<i32, U3> = GenericArray::<i32, U3>::default();
        <GenericArray<i32, U3> as core::ops::Deref>::deref(&p0);
        let _rug_ed_tests_rug_5_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_6 {
    use super::*;
    use crate::core::ops::DerefMut;
    use crate::GenericArray;
    use crate::typenum::U3;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_6_rrrruuuugggg_test_rug = 0;
        let mut p0 = GenericArray::<u32, U3>::default();
        <GenericArray<u32, U3> as core::ops::DerefMut>::deref_mut(&mut p0);
        let _rug_ed_tests_rug_6_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_16 {
    use super::*;
    use crate::core::iter::FromIterator;
    use crate::{GenericArray, ArrayBuilder};
    use core::iter;
    use core::ptr;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4)) = <(u32, u32, u32, u32, u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3, rug_fuzz_4];
        <GenericArray<u32, typenum::U5>>::from_iter(p0.iter().cloned());
             }
});    }
}
#[cfg(test)]
mod tests_rug_18 {
    use super::*;
    use crate::sequence::GenericSequence;
    use core::ops::FnMut;
    use crate::arr;
    use crate::typenum::{U4, U8};
    use crate::GenericArray;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_18_rrrruuuugggg_test_rug = 0;
        let p0: GenericArray<u32, U8> = arr![u32; 1, 2, 3, 4, 5, 6, 7, 8];
        let p1: GenericArray<u32, U8> = arr![u32; 11, 12, 13, 14, 15, 16, 17, 18];
        let mut p2 = |x: u32, y: u32| x + y;
        p0.inverted_zip(p1, &mut p2);
        let _rug_ed_tests_rug_18_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_20 {
    use super::*;
    use crate::functional::FunctionalSequence;
    use crate::{GenericArray, arr};
    use crate::typenum::U4;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_20_rrrruuuugggg_test_rug = 0;
        let p0: GenericArray<u32, U4> = arr![u32; 1, 2, 3, 4];
        let p1 = |x: u32| x * x;
        <GenericArray<u32, U4> as FunctionalSequence<u32>>::map(p0, p1);
        let _rug_ed_tests_rug_20_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_21 {
    use super::*;
    use crate::functional::FunctionalSequence;
    use crate::arr;
    use crate::typenum::{U4, U8};
    use crate::GenericArray;
    use crate::{MappedSequence, MappedGenericSequence};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_21_rrrruuuugggg_test_rug = 0;
        let p0: GenericArray<u32, U8> = arr![u32; 1, 2, 3, 4, 5, 6, 7, 8];
        let p1: GenericArray<u64, U8> = arr![u64; 9, 10, 11, 12, 13, 14, 15, 16];
        let p2 = |x: u32, y: u64| x as u64 + y;
        p0.zip(p1, p2);
        let _rug_ed_tests_rug_21_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_23 {
    use super::*;
    use crate::{GenericArray, typenum::U5};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_23_rrrruuuugggg_test_rug = 0;
        let mut p0: GenericArray<u32, U5> = GenericArray::default();
        <GenericArray<u32, U5>>::as_slice(&p0);
        let _rug_ed_tests_rug_23_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_24 {
    use super::*;
    use crate::{GenericArray, typenum::U4};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_24_rrrruuuugggg_test_rug = 0;
        let mut p0 = GenericArray::<u32, U4>::default();
        <GenericArray<u32, U4>>::as_mut_slice(&mut p0);
        let _rug_ed_tests_rug_24_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_25 {
    use super::*;
    use crate::{GenericArray, typenum::U3};
    #[test]
    fn test_from_slice() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(u32, u32, u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: &[u32; 3] = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        GenericArray::<u32, U3>::from_slice(p0);
             }
});    }
}
#[cfg(test)]
mod tests_rug_26 {
    use super::*;
    use crate::{GenericArray, typenum::consts::*};
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(u32, u32, u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: &mut [u32; 3] = &mut [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        <GenericArray<u32, U3>>::from_mut_slice(p0);
             }
});    }
}
