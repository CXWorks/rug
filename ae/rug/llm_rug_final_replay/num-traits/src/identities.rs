use core::num::Wrapping;
use core::ops::{Add, Mul};
/// Defines an additive identity element for `Self`.
///
/// # Laws
///
/// ```{.text}
/// a + 0 = a       ∀ a ∈ Self
/// 0 + a = a       ∀ a ∈ Self
/// ```
pub trait Zero: Sized + Add<Self, Output = Self> {
    /// Returns the additive identity element of `Self`, `0`.
    /// # Purity
    ///
    /// This function should return the same result at all times regardless of
    /// external mutable state, for example values stored in TLS or in
    /// `static mut`s.
    fn zero() -> Self;
    /// Sets `self` to the additive identity element of `Self`, `0`.
    fn set_zero(&mut self) {
        *self = Zero::zero();
    }
    /// Returns `true` if `self` is equal to the additive identity.
    fn is_zero(&self) -> bool;
}
macro_rules! zero_impl {
    ($t:ty, $v:expr) => {
        impl Zero for $t { #[inline] fn zero() -> $t { $v } #[inline] fn is_zero(& self)
        -> bool { * self == $v } }
    };
}
zero_impl!(usize, 0);
zero_impl!(u8, 0);
zero_impl!(u16, 0);
zero_impl!(u32, 0);
zero_impl!(u64, 0);
zero_impl!(u128, 0);
zero_impl!(isize, 0);
zero_impl!(i8, 0);
zero_impl!(i16, 0);
zero_impl!(i32, 0);
zero_impl!(i64, 0);
zero_impl!(i128, 0);
zero_impl!(f32, 0.0);
zero_impl!(f64, 0.0);
impl<T: Zero> Zero for Wrapping<T>
where
    Wrapping<T>: Add<Output = Wrapping<T>>,
{
    fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
    fn set_zero(&mut self) {
        self.0.set_zero();
    }
    fn zero() -> Self {
        Wrapping(T::zero())
    }
}
/// Defines a multiplicative identity element for `Self`.
///
/// # Laws
///
/// ```{.text}
/// a * 1 = a       ∀ a ∈ Self
/// 1 * a = a       ∀ a ∈ Self
/// ```
pub trait One: Sized + Mul<Self, Output = Self> {
    /// Returns the multiplicative identity element of `Self`, `1`.
    ///
    /// # Purity
    ///
    /// This function should return the same result at all times regardless of
    /// external mutable state, for example values stored in TLS or in
    /// `static mut`s.
    fn one() -> Self;
    /// Sets `self` to the multiplicative identity element of `Self`, `1`.
    fn set_one(&mut self) {
        *self = One::one();
    }
    /// Returns `true` if `self` is equal to the multiplicative identity.
    ///
    /// For performance reasons, it's best to implement this manually.
    /// After a semver bump, this method will be required, and the
    /// `where Self: PartialEq` bound will be removed.
    #[inline]
    fn is_one(&self) -> bool
    where
        Self: PartialEq,
    {
        *self == Self::one()
    }
}
macro_rules! one_impl {
    ($t:ty, $v:expr) => {
        impl One for $t { #[inline] fn one() -> $t { $v } #[inline] fn is_one(& self) ->
        bool { * self == $v } }
    };
}
one_impl!(usize, 1);
one_impl!(u8, 1);
one_impl!(u16, 1);
one_impl!(u32, 1);
one_impl!(u64, 1);
one_impl!(u128, 1);
one_impl!(isize, 1);
one_impl!(i8, 1);
one_impl!(i16, 1);
one_impl!(i32, 1);
one_impl!(i64, 1);
one_impl!(i128, 1);
one_impl!(f32, 1.0);
one_impl!(f64, 1.0);
impl<T: One> One for Wrapping<T>
where
    Wrapping<T>: Mul<Output = Wrapping<T>>,
{
    fn set_one(&mut self) {
        self.0.set_one();
    }
    fn one() -> Self {
        Wrapping(T::one())
    }
}
/// Returns the additive identity, `0`.
#[inline(always)]
pub fn zero<T: Zero>() -> T {
    Zero::zero()
}
/// Returns the multiplicative identity, `1`.
#[inline(always)]
pub fn one<T: One>() -> T {
    One::one()
}
#[test]
fn wrapping_identities() {
    macro_rules! test_wrapping_identities {
        ($($t:ty)+) => {
            $(assert_eq!(zero::<$t > (), zero::< Wrapping <$t >> ().0);
            assert_eq!(one::<$t > (), one::< Wrapping <$t >> ().0); assert_eq!((0 as $t)
            .is_zero(), Wrapping(0 as $t).is_zero()); assert_eq!((1 as $t).is_zero(),
            Wrapping(1 as $t).is_zero());)+
        };
    }
    test_wrapping_identities!(isize i8 i16 i32 i64 usize u8 u16 u32 u64);
}
#[test]
fn wrapping_is_zero() {
    fn require_zero<T: Zero>(_: &T) {}
    require_zero(&Wrapping(42));
}
#[test]
fn wrapping_is_one() {
    fn require_one<T: One>(_: &T) {}
    require_one(&Wrapping(42));
}
#[cfg(test)]
mod tests_rug_719 {
    use super::*;
    use std::num::Wrapping;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Wrapping<i8> = Wrapping(rug_fuzz_0);
        crate::identities::Zero::set_zero(&mut p0);
        debug_assert_eq!(p0, Wrapping(0));
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_720 {
    use super::*;
    use std::num::Wrapping;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u128) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Wrapping<u128> = Wrapping(rug_fuzz_0);
        crate::identities::One::set_one(&mut p0);
        debug_assert_eq!(p0, Wrapping(1));
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_723 {
    use super::*;
    use crate::Zero;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: usize = rug_fuzz_0;
        <usize as Zero>::is_zero(&p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_724 {
    use super::*;
    use crate::identities::Zero;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_724_rrrruuuugggg_test_rug = 0;
        <u8 as Zero>::zero();
        let _rug_ed_tests_rug_724_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_725 {
    use super::*;
    use crate::Zero;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: u8 = rug_fuzz_0;
        p0.is_zero();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_726 {
    use super::*;
    use crate::identities::Zero;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_726_rrrruuuugggg_test_rug = 0;
        <u16 as Zero>::zero();
        let _rug_ed_tests_rug_726_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_727 {
    use super::*;
    use crate::Zero;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: u16 = rug_fuzz_0;
        p0.is_zero();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_732 {
    use super::*;
    use crate::identities::Zero;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_732_rrrruuuugggg_test_rug = 0;
        let _: u128 = Zero::zero();
        let _rug_ed_tests_rug_732_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_733 {
    use super::*;
    use crate::Zero;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u128) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: u128 = rug_fuzz_0;
        p0.is_zero();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_735 {
    use super::*;
    use crate::Zero;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(isize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: isize = rug_fuzz_0;
        p0.is_zero();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_737 {
    use super::*;
    use crate::Zero;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: i8 = rug_fuzz_0;
        p0.is_zero();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_739 {
    use super::*;
    use crate::Zero;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: i16 = rug_fuzz_0;
        p0.is_zero();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_741 {
    use super::*;
    use crate::Zero;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: i32 = rug_fuzz_0;
        debug_assert_eq!(p0.is_zero(), true);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_743 {
    use super::*;
    use crate::Zero;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: i64 = rug_fuzz_0;
        <i64 as Zero>::is_zero(&p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_747 {
    use super::*;
    use crate::Zero;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: f32 = rug_fuzz_0;
        debug_assert_eq!(< f32 as Zero > ::is_zero(& p0), true);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_750 {
    use super::*;
    use std::num::Wrapping;
    use crate::Zero;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: Wrapping<i16> = Wrapping(rug_fuzz_0);
        p0.is_zero();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_751 {
    use super::*;
    use crate::Zero;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(isize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: std::num::Wrapping<isize> = std::num::Wrapping(rug_fuzz_0);
        <std::num::Wrapping<isize>>::set_zero(&mut p0);
        debug_assert_eq!(p0.0, 0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_754 {
    use super::*;
    use crate::identities::One;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: usize = rug_fuzz_0;
        p0.is_one();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_758 {
    use super::*;
    use crate::One;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: u16 = rug_fuzz_0;
        p0.is_one();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_761 {
    use super::*;
    use crate::One;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_761_rrrruuuugggg_test_rug = 0;
        let result: u64 = u64::one();
        debug_assert_eq!(result, 1);
        let _rug_ed_tests_rug_761_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_767 {
    use super::*;
    use crate::identities::One;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_767_rrrruuuugggg_test_rug = 0;
        let result: i8 = One::one();
        debug_assert_eq!(result, 1);
        let _rug_ed_tests_rug_767_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_771 {
    use super::*;
    use crate::identities::One;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_771_rrrruuuugggg_test_rug = 0;
        let result: i32 = <i32 as One>::one();
        debug_assert_eq!(result, 1);
        let _rug_ed_tests_rug_771_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_774 {
    use super::*;
    use crate::One;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = rug_fuzz_0;
        <i64 as One>::is_one(&p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_775 {
    use super::*;
    use crate::identities::One;
    #[test]
    fn test_i128() {
        let _rug_st_tests_rug_775_rrrruuuugggg_test_i128 = 0;
        let result: i128 = <i128 as One>::one();
        debug_assert_eq!(result, 1);
        let _rug_ed_tests_rug_775_rrrruuuugggg_test_i128 = 0;
    }
    #[test]
    fn test_u128() {
        let _rug_st_tests_rug_775_rrrruuuugggg_test_u128 = 0;
        let result: u128 = <u128 as One>::one();
        debug_assert_eq!(result, 1);
        let _rug_ed_tests_rug_775_rrrruuuugggg_test_u128 = 0;
    }
    #[test]
    fn test_i64() {
        let _rug_st_tests_rug_775_rrrruuuugggg_test_i64 = 0;
        let result: i64 = <i64 as One>::one();
        debug_assert_eq!(result, 1);
        let _rug_ed_tests_rug_775_rrrruuuugggg_test_i64 = 0;
    }
    #[test]
    fn test_u64() {
        let _rug_st_tests_rug_775_rrrruuuugggg_test_u64 = 0;
        let result: u64 = <u64 as One>::one();
        debug_assert_eq!(result, 1);
        let _rug_ed_tests_rug_775_rrrruuuugggg_test_u64 = 0;
    }
}
#[cfg(test)]
mod tests_rug_780 {
    use super::*;
    use crate::One;
    #[test]
    fn test_is_one() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: f64 = rug_fuzz_0;
        p0.is_one();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_781 {
    use super::*;
    use crate::One;
    use std::num::Wrapping;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Wrapping<i32> = Wrapping(rug_fuzz_0);
        p0.set_one();
             }
}
}
}    }
}
