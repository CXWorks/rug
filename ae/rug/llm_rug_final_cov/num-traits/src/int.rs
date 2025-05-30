use core::ops::{BitAnd, BitOr, BitXor, Not, Shl, Shr};
use crate::bounds::Bounded;
use crate::ops::checked::*;
use crate::ops::saturating::Saturating;
use crate::{Num, NumCast};
/// Generic trait for primitive integers.
///
/// The `PrimInt` trait is an abstraction over the builtin primitive integer types (e.g., `u8`,
/// `u32`, `isize`, `i128`, ...). It inherits the basic numeric traits and extends them with
/// bitwise operators and non-wrapping arithmetic.
///
/// The trait explicitly inherits `Copy`, `Eq`, `Ord`, and `Sized`. The intention is that all
/// types implementing this trait behave like primitive types that are passed by value by default
/// and behave like builtin integers. Furthermore, the types are expected to expose the integer
/// value in binary representation and support bitwise operators. The standard bitwise operations
/// (e.g., bitwise-and, bitwise-or, right-shift, left-shift) are inherited and the trait extends
/// these with introspective queries (e.g., `PrimInt::count_ones()`, `PrimInt::leading_zeros()`),
/// bitwise combinators (e.g., `PrimInt::rotate_left()`), and endianness converters (e.g.,
/// `PrimInt::to_be()`).
///
/// All `PrimInt` types are expected to be fixed-width binary integers. The width can be queried
/// via `T::zero().count_zeros()`. The trait currently lacks a way to query the width at
/// compile-time.
///
/// While a default implementation for all builtin primitive integers is provided, the trait is in
/// no way restricted to these. Other integer types that fulfil the requirements are free to
/// implement the trait was well.
///
/// This trait and many of the method names originate in the unstable `core::num::Int` trait from
/// the rust standard library. The original trait was never stabilized and thus removed from the
/// standard library.
pub trait PrimInt: Sized + Copy + Num + NumCast + Bounded + PartialOrd + Ord + Eq + Not<
        Output = Self,
    > + BitAnd<
        Output = Self,
    > + BitOr<
        Output = Self,
    > + BitXor<
        Output = Self,
    > + Shl<
        usize,
        Output = Self,
    > + Shr<
        usize,
        Output = Self,
    > + CheckedAdd<
        Output = Self,
    > + CheckedSub<
        Output = Self,
    > + CheckedMul<Output = Self> + CheckedDiv<Output = Self> + Saturating {
    /// Returns the number of ones in the binary representation of `self`.
    ///
    /// # Examples
    ///
    /// ```
    /// use num_traits::PrimInt;
    ///
    /// let n = 0b01001100u8;
    ///
    /// assert_eq!(n.count_ones(), 3);
    /// ```
    fn count_ones(self) -> u32;
    /// Returns the number of zeros in the binary representation of `self`.
    ///
    /// # Examples
    ///
    /// ```
    /// use num_traits::PrimInt;
    ///
    /// let n = 0b01001100u8;
    ///
    /// assert_eq!(n.count_zeros(), 5);
    /// ```
    fn count_zeros(self) -> u32;
    /// Returns the number of leading ones in the binary representation
    /// of `self`.
    ///
    /// # Examples
    ///
    /// ```
    /// use num_traits::PrimInt;
    ///
    /// let n = 0xF00Du16;
    ///
    /// assert_eq!(n.leading_ones(), 4);
    /// ```
    fn leading_ones(self) -> u32 {
        (!self).leading_zeros()
    }
    /// Returns the number of leading zeros in the binary representation
    /// of `self`.
    ///
    /// # Examples
    ///
    /// ```
    /// use num_traits::PrimInt;
    ///
    /// let n = 0b0101000u16;
    ///
    /// assert_eq!(n.leading_zeros(), 10);
    /// ```
    fn leading_zeros(self) -> u32;
    /// Returns the number of trailing ones in the binary representation
    /// of `self`.
    ///
    /// # Examples
    ///
    /// ```
    /// use num_traits::PrimInt;
    ///
    /// let n = 0xBEEFu16;
    ///
    /// assert_eq!(n.trailing_ones(), 4);
    /// ```
    fn trailing_ones(self) -> u32 {
        (!self).trailing_zeros()
    }
    /// Returns the number of trailing zeros in the binary representation
    /// of `self`.
    ///
    /// # Examples
    ///
    /// ```
    /// use num_traits::PrimInt;
    ///
    /// let n = 0b0101000u16;
    ///
    /// assert_eq!(n.trailing_zeros(), 3);
    /// ```
    fn trailing_zeros(self) -> u32;
    /// Shifts the bits to the left by a specified amount, `n`, wrapping
    /// the truncated bits to the end of the resulting integer.
    ///
    /// # Examples
    ///
    /// ```
    /// use num_traits::PrimInt;
    ///
    /// let n = 0x0123456789ABCDEFu64;
    /// let m = 0x3456789ABCDEF012u64;
    ///
    /// assert_eq!(n.rotate_left(12), m);
    /// ```
    fn rotate_left(self, n: u32) -> Self;
    /// Shifts the bits to the right by a specified amount, `n`, wrapping
    /// the truncated bits to the beginning of the resulting integer.
    ///
    /// # Examples
    ///
    /// ```
    /// use num_traits::PrimInt;
    ///
    /// let n = 0x0123456789ABCDEFu64;
    /// let m = 0xDEF0123456789ABCu64;
    ///
    /// assert_eq!(n.rotate_right(12), m);
    /// ```
    fn rotate_right(self, n: u32) -> Self;
    /// Shifts the bits to the left by a specified amount, `n`, filling
    /// zeros in the least significant bits.
    ///
    /// This is bitwise equivalent to signed `Shl`.
    ///
    /// # Examples
    ///
    /// ```
    /// use num_traits::PrimInt;
    ///
    /// let n = 0x0123456789ABCDEFu64;
    /// let m = 0x3456789ABCDEF000u64;
    ///
    /// assert_eq!(n.signed_shl(12), m);
    /// ```
    fn signed_shl(self, n: u32) -> Self;
    /// Shifts the bits to the right by a specified amount, `n`, copying
    /// the "sign bit" in the most significant bits even for unsigned types.
    ///
    /// This is bitwise equivalent to signed `Shr`.
    ///
    /// # Examples
    ///
    /// ```
    /// use num_traits::PrimInt;
    ///
    /// let n = 0xFEDCBA9876543210u64;
    /// let m = 0xFFFFEDCBA9876543u64;
    ///
    /// assert_eq!(n.signed_shr(12), m);
    /// ```
    fn signed_shr(self, n: u32) -> Self;
    /// Shifts the bits to the left by a specified amount, `n`, filling
    /// zeros in the least significant bits.
    ///
    /// This is bitwise equivalent to unsigned `Shl`.
    ///
    /// # Examples
    ///
    /// ```
    /// use num_traits::PrimInt;
    ///
    /// let n = 0x0123456789ABCDEFi64;
    /// let m = 0x3456789ABCDEF000i64;
    ///
    /// assert_eq!(n.unsigned_shl(12), m);
    /// ```
    fn unsigned_shl(self, n: u32) -> Self;
    /// Shifts the bits to the right by a specified amount, `n`, filling
    /// zeros in the most significant bits.
    ///
    /// This is bitwise equivalent to unsigned `Shr`.
    ///
    /// # Examples
    ///
    /// ```
    /// use num_traits::PrimInt;
    ///
    /// let n = -8i8; // 0b11111000
    /// let m = 62i8; // 0b00111110
    ///
    /// assert_eq!(n.unsigned_shr(2), m);
    /// ```
    fn unsigned_shr(self, n: u32) -> Self;
    /// Reverses the byte order of the integer.
    ///
    /// # Examples
    ///
    /// ```
    /// use num_traits::PrimInt;
    ///
    /// let n = 0x0123456789ABCDEFu64;
    /// let m = 0xEFCDAB8967452301u64;
    ///
    /// assert_eq!(n.swap_bytes(), m);
    /// ```
    fn swap_bytes(self) -> Self;
    /// Reverses the order of bits in the integer.
    ///
    /// The least significant bit becomes the most significant bit, second least-significant bit
    /// becomes second most-significant bit, etc.
    ///
    /// # Examples
    ///
    /// ```
    /// use num_traits::PrimInt;
    ///
    /// let n = 0x12345678u32;
    /// let m = 0x1e6a2c48u32;
    ///
    /// assert_eq!(n.reverse_bits(), m);
    /// assert_eq!(0u32.reverse_bits(), 0);
    /// ```
    fn reverse_bits(self) -> Self {
        reverse_bits_fallback(self)
    }
    /// Convert an integer from big endian to the target's endianness.
    ///
    /// On big endian this is a no-op. On little endian the bytes are swapped.
    ///
    /// # Examples
    ///
    /// ```
    /// use num_traits::PrimInt;
    ///
    /// let n = 0x0123456789ABCDEFu64;
    ///
    /// if cfg!(target_endian = "big") {
    ///     assert_eq!(u64::from_be(n), n)
    /// } else {
    ///     assert_eq!(u64::from_be(n), n.swap_bytes())
    /// }
    /// ```
    fn from_be(x: Self) -> Self;
    /// Convert an integer from little endian to the target's endianness.
    ///
    /// On little endian this is a no-op. On big endian the bytes are swapped.
    ///
    /// # Examples
    ///
    /// ```
    /// use num_traits::PrimInt;
    ///
    /// let n = 0x0123456789ABCDEFu64;
    ///
    /// if cfg!(target_endian = "little") {
    ///     assert_eq!(u64::from_le(n), n)
    /// } else {
    ///     assert_eq!(u64::from_le(n), n.swap_bytes())
    /// }
    /// ```
    fn from_le(x: Self) -> Self;
    /// Convert `self` to big endian from the target's endianness.
    ///
    /// On big endian this is a no-op. On little endian the bytes are swapped.
    ///
    /// # Examples
    ///
    /// ```
    /// use num_traits::PrimInt;
    ///
    /// let n = 0x0123456789ABCDEFu64;
    ///
    /// if cfg!(target_endian = "big") {
    ///     assert_eq!(n.to_be(), n)
    /// } else {
    ///     assert_eq!(n.to_be(), n.swap_bytes())
    /// }
    /// ```
    fn to_be(self) -> Self;
    /// Convert `self` to little endian from the target's endianness.
    ///
    /// On little endian this is a no-op. On big endian the bytes are swapped.
    ///
    /// # Examples
    ///
    /// ```
    /// use num_traits::PrimInt;
    ///
    /// let n = 0x0123456789ABCDEFu64;
    ///
    /// if cfg!(target_endian = "little") {
    ///     assert_eq!(n.to_le(), n)
    /// } else {
    ///     assert_eq!(n.to_le(), n.swap_bytes())
    /// }
    /// ```
    fn to_le(self) -> Self;
    /// Raises self to the power of `exp`, using exponentiation by squaring.
    ///
    /// # Examples
    ///
    /// ```
    /// use num_traits::PrimInt;
    ///
    /// assert_eq!(2i32.pow(4), 16);
    /// ```
    fn pow(self, exp: u32) -> Self;
}
fn one_per_byte<P: PrimInt>() -> P {
    let mut ret = P::one();
    let mut shift = 8;
    let mut b = ret.count_zeros() >> 3;
    while b != 0 {
        ret = (ret << shift) | ret;
        shift <<= 1;
        b >>= 1;
    }
    ret
}
fn reverse_bits_fallback<P: PrimInt>(i: P) -> P {
    let rep_01: P = one_per_byte();
    let rep_03 = (rep_01 << 1) | rep_01;
    let rep_05 = (rep_01 << 2) | rep_01;
    let rep_0f = (rep_03 << 2) | rep_03;
    let rep_33 = (rep_03 << 4) | rep_03;
    let rep_55 = (rep_05 << 4) | rep_05;
    let mut ret = i.swap_bytes();
    ret = ((ret & rep_0f) << 4) | ((ret >> 4) & rep_0f);
    ret = ((ret & rep_33) << 2) | ((ret >> 2) & rep_33);
    ret = ((ret & rep_55) << 1) | ((ret >> 1) & rep_55);
    ret
}
macro_rules! prim_int_impl {
    ($T:ty, $S:ty, $U:ty) => {
        impl PrimInt for $T { #[inline] fn count_ones(self) -> u32 { <$T
        >::count_ones(self) } #[inline] fn count_zeros(self) -> u32 { <$T
        >::count_zeros(self) } #[cfg(has_leading_trailing_ones)] #[inline] fn
        leading_ones(self) -> u32 { <$T >::leading_ones(self) } #[inline] fn
        leading_zeros(self) -> u32 { <$T >::leading_zeros(self) }
        #[cfg(has_leading_trailing_ones)] #[inline] fn trailing_ones(self) -> u32 { <$T
        >::trailing_ones(self) } #[inline] fn trailing_zeros(self) -> u32 { <$T
        >::trailing_zeros(self) } #[inline] fn rotate_left(self, n : u32) -> Self { <$T
        >::rotate_left(self, n) } #[inline] fn rotate_right(self, n : u32) -> Self { <$T
        >::rotate_right(self, n) } #[inline] fn signed_shl(self, n : u32) -> Self {
        ((self as $S) << n) as $T } #[inline] fn signed_shr(self, n : u32) -> Self {
        ((self as $S) >> n) as $T } #[inline] fn unsigned_shl(self, n : u32) -> Self {
        ((self as $U) << n) as $T } #[inline] fn unsigned_shr(self, n : u32) -> Self {
        ((self as $U) >> n) as $T } #[inline] fn swap_bytes(self) -> Self { <$T
        >::swap_bytes(self) } #[cfg(has_reverse_bits)] #[inline] fn reverse_bits(self) ->
        Self { <$T >::reverse_bits(self) } #[inline] fn from_be(x : Self) -> Self { <$T
        >::from_be(x) } #[inline] fn from_le(x : Self) -> Self { <$T >::from_le(x) }
        #[inline] fn to_be(self) -> Self { <$T >::to_be(self) } #[inline] fn to_le(self)
        -> Self { <$T >::to_le(self) } #[inline] fn pow(self, exp : u32) -> Self { <$T
        >::pow(self, exp) } }
    };
}
prim_int_impl!(u8, i8, u8);
prim_int_impl!(u16, i16, u16);
prim_int_impl!(u32, i32, u32);
prim_int_impl!(u64, i64, u64);
prim_int_impl!(u128, i128, u128);
prim_int_impl!(usize, isize, usize);
prim_int_impl!(i8, i8, u8);
prim_int_impl!(i16, i16, u16);
prim_int_impl!(i32, i32, u32);
prim_int_impl!(i64, i64, u64);
prim_int_impl!(i128, i128, u128);
prim_int_impl!(isize, isize, usize);
#[cfg(test)]
mod tests {
    use crate::int::PrimInt;
    #[test]
    pub fn reverse_bits() {
        use core::{i16, i32, i64, i8};
        assert_eq!(
            PrimInt::reverse_bits(0x0123_4567_89ab_cdefu64), 0xf7b3_d591_e6a2_c480
        );
        assert_eq!(PrimInt::reverse_bits(0i8), 0);
        assert_eq!(PrimInt::reverse_bits(- 1i8), - 1);
        assert_eq!(PrimInt::reverse_bits(1i8), i8::MIN);
        assert_eq!(PrimInt::reverse_bits(i8::MIN), 1);
        assert_eq!(PrimInt::reverse_bits(- 2i8), i8::MAX);
        assert_eq!(PrimInt::reverse_bits(i8::MAX), - 2);
        assert_eq!(PrimInt::reverse_bits(0i16), 0);
        assert_eq!(PrimInt::reverse_bits(- 1i16), - 1);
        assert_eq!(PrimInt::reverse_bits(1i16), i16::MIN);
        assert_eq!(PrimInt::reverse_bits(i16::MIN), 1);
        assert_eq!(PrimInt::reverse_bits(- 2i16), i16::MAX);
        assert_eq!(PrimInt::reverse_bits(i16::MAX), - 2);
        assert_eq!(PrimInt::reverse_bits(0i32), 0);
        assert_eq!(PrimInt::reverse_bits(- 1i32), - 1);
        assert_eq!(PrimInt::reverse_bits(1i32), i32::MIN);
        assert_eq!(PrimInt::reverse_bits(i32::MIN), 1);
        assert_eq!(PrimInt::reverse_bits(- 2i32), i32::MAX);
        assert_eq!(PrimInt::reverse_bits(i32::MAX), - 2);
        assert_eq!(PrimInt::reverse_bits(0i64), 0);
        assert_eq!(PrimInt::reverse_bits(- 1i64), - 1);
        assert_eq!(PrimInt::reverse_bits(1i64), i64::MIN);
        assert_eq!(PrimInt::reverse_bits(i64::MIN), 1);
        assert_eq!(PrimInt::reverse_bits(- 2i64), i64::MAX);
        assert_eq!(PrimInt::reverse_bits(i64::MAX), - 2);
    }
    #[test]
    pub fn reverse_bits_i128() {
        use core::i128;
        assert_eq!(PrimInt::reverse_bits(0i128), 0);
        assert_eq!(PrimInt::reverse_bits(- 1i128), - 1);
        assert_eq!(PrimInt::reverse_bits(1i128), i128::MIN);
        assert_eq!(PrimInt::reverse_bits(i128::MIN), 1);
        assert_eq!(PrimInt::reverse_bits(- 2i128), i128::MAX);
        assert_eq!(PrimInt::reverse_bits(i128::MAX), - 2);
    }
}
#[cfg(test)]
mod tests_rug_788 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_788_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0b10101010;
        let mut p0: u8 = rug_fuzz_0;
        p0.count_ones();
        let _rug_ed_tests_rug_788_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_792 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_792_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0b10101010;
        let mut p0: u8 = rug_fuzz_0;
        p0.trailing_ones();
        let _rug_ed_tests_rug_792_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_794 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rotate_left() {
        let _rug_st_tests_rug_794_rrrruuuugggg_test_rotate_left = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 3;
        let p0: u8 = rug_fuzz_0;
        let p1: u32 = rug_fuzz_1;
        p0.rotate_left(p1);
        let _rug_ed_tests_rug_794_rrrruuuugggg_test_rotate_left = 0;
    }
}
#[cfg(test)]
mod tests_rug_797 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_signed_shr() {
        let _rug_st_tests_rug_797_rrrruuuugggg_test_signed_shr = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 2;
        let p0: u8 = rug_fuzz_0;
        let p1: u32 = rug_fuzz_1;
        p0.signed_shr(p1);
        let _rug_ed_tests_rug_797_rrrruuuugggg_test_signed_shr = 0;
    }
}
#[cfg(test)]
mod tests_rug_802 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_802_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0b11001100;
        let mut p0: u8 = rug_fuzz_0;
        <u8 as PrimInt>::from_be(p0);
        let _rug_ed_tests_rug_802_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_805 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_805_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let p0: u8 = rug_fuzz_0;
        <u8>::to_le(p0);
        let _rug_ed_tests_rug_805_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_806 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_806_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 3;
        let mut p0: u8 = rug_fuzz_0;
        let mut p1: u32 = rug_fuzz_1;
        p0.pow(p1);
        let _rug_ed_tests_rug_806_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_807 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_807_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0b10101010;
        let mut p0: u16 = rug_fuzz_0;
        p0.count_ones();
        let _rug_ed_tests_rug_807_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_811 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_811_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0b0101010101010101;
        let mut p0: u16 = rug_fuzz_0;
        p0.trailing_ones();
        let _rug_ed_tests_rug_811_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_812 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_812_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let p0: u16 = rug_fuzz_0;
        <u16>::trailing_zeros(p0);
        let _rug_ed_tests_rug_812_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_814 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_814_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 3;
        let mut p0: u16 = rug_fuzz_0;
        let mut p1: u32 = rug_fuzz_1;
        p0.rotate_right(p1);
        let _rug_ed_tests_rug_814_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_816 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_signed_shr() {
        let _rug_st_tests_rug_816_rrrruuuugggg_test_signed_shr = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 2;
        let p0: u16 = rug_fuzz_0;
        let p1: u32 = rug_fuzz_1;
        p0.signed_shr(p1);
        let _rug_ed_tests_rug_816_rrrruuuugggg_test_signed_shr = 0;
    }
}
#[cfg(test)]
mod tests_rug_817 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_817_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 2;
        let mut p0: u16 = rug_fuzz_0;
        let mut p1: u32 = rug_fuzz_1;
        <u16 as PrimInt>::unsigned_shl(p0, p1);
        let _rug_ed_tests_rug_817_rrrruuuugggg_test_rug = 0;
    }
}
mod tests_rug_820 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_820_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0b1010101010101010;
        let mut p0: u16 = rug_fuzz_0;
        <u16>::reverse_bits(p0);
        let _rug_ed_tests_rug_820_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_821 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_821_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0xFFFF;
        let mut p0: u16 = rug_fuzz_0;
        <u16 as crate::int::PrimInt>::from_be(p0);
        let _rug_ed_tests_rug_821_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_822 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_822_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u16 = rug_fuzz_0;
        <u16 as PrimInt>::from_le(p0);
        let _rug_ed_tests_rug_822_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_823 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_823_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 12345;
        let mut p0: u16 = rug_fuzz_0;
        <u16 as PrimInt>::to_be(p0);
        let _rug_ed_tests_rug_823_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_824 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_824_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 12345;
        let p0: u16 = rug_fuzz_0;
        <u16>::to_le(p0);
        let _rug_ed_tests_rug_824_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_825 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_825_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 3;
        let mut p0: u16 = rug_fuzz_0;
        let mut p1: u32 = rug_fuzz_1;
        <u16 as PrimInt>::pow(p0, p1);
        let _rug_ed_tests_rug_825_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_826 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_826_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: u32 = rug_fuzz_0;
        <u32>::count_ones(p0);
        let _rug_ed_tests_rug_826_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_831 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_trailing_zeros() {
        let _rug_st_tests_rug_831_rrrruuuugggg_test_trailing_zeros = 0;
        let rug_fuzz_0 = 10;
        let p0: u32 = rug_fuzz_0;
        p0.trailing_zeros();
        let _rug_ed_tests_rug_831_rrrruuuugggg_test_trailing_zeros = 0;
    }
}
#[cfg(test)]
mod tests_rug_833 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rotate_right() {
        let _rug_st_tests_rug_833_rrrruuuugggg_test_rotate_right = 0;
        let rug_fuzz_0 = 123;
        let rug_fuzz_1 = 3;
        let mut p0: u32 = rug_fuzz_0;
        let mut p1: u32 = rug_fuzz_1;
        p0.rotate_right(p1);
        let _rug_ed_tests_rug_833_rrrruuuugggg_test_rotate_right = 0;
    }
}
#[cfg(test)]
mod tests_rug_835 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_signed_shr() {
        let _rug_st_tests_rug_835_rrrruuuugggg_test_signed_shr = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 2;
        let mut p0: u32 = rug_fuzz_0;
        let mut p1: u32 = rug_fuzz_1;
        <u32 as PrimInt>::signed_shr(p0, p1);
        let _rug_ed_tests_rug_835_rrrruuuugggg_test_signed_shr = 0;
    }
}
#[cfg(test)]
mod tests_rug_839 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_reverse_bits() {
        let _rug_st_tests_rug_839_rrrruuuugggg_test_reverse_bits = 0;
        let rug_fuzz_0 = 0b10101010101010101010101010101010;
        let p0: u32 = rug_fuzz_0;
        <u32>::reverse_bits(p0);
        let _rug_ed_tests_rug_839_rrrruuuugggg_test_reverse_bits = 0;
    }
}
#[cfg(test)]
mod tests_rug_841 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_from_le() {
        let _rug_st_tests_rug_841_rrrruuuugggg_test_from_le = 0;
        let rug_fuzz_0 = 12345;
        let mut p0: u32 = rug_fuzz_0;
        <u32>::from_le(p0);
        let _rug_ed_tests_rug_841_rrrruuuugggg_test_from_le = 0;
    }
}
#[cfg(test)]
mod tests_rug_842 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_to_be() {
        let _rug_st_tests_rug_842_rrrruuuugggg_test_to_be = 0;
        let rug_fuzz_0 = 500;
        let p0: u32 = rug_fuzz_0;
        p0.to_be();
        let _rug_ed_tests_rug_842_rrrruuuugggg_test_to_be = 0;
    }
}
#[cfg(test)]
mod tests_rug_843 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_843_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123;
        let mut p0: u32 = rug_fuzz_0;
        <u32>::to_le(p0);
        let _rug_ed_tests_rug_843_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_844 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_844_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 2;
        let p0: u32 = rug_fuzz_0;
        let p1: u32 = rug_fuzz_1;
        <u32 as PrimInt>::pow(p0, p1);
        let _rug_ed_tests_rug_844_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_849 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_trailing_ones() {
        let _rug_st_tests_rug_849_rrrruuuugggg_test_trailing_ones = 0;
        let rug_fuzz_0 = 0b111000;
        let p0: u64 = rug_fuzz_0;
        p0.trailing_ones();
        let _rug_ed_tests_rug_849_rrrruuuugggg_test_trailing_ones = 0;
    }
}
#[cfg(test)]
mod tests_rug_851 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_851_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0xABCD_EF12_3456_7890;
        let rug_fuzz_1 = 16;
        let mut p0: u64 = rug_fuzz_0;
        let mut p1: u32 = rug_fuzz_1;
        p0.rotate_left(p1);
        let _rug_ed_tests_rug_851_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_853 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_signed_shl() {
        let _rug_st_tests_rug_853_rrrruuuugggg_test_signed_shl = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 2;
        let mut p0: u64 = rug_fuzz_0;
        let mut p1: u32 = rug_fuzz_1;
        p0.signed_shl(p1);
        let _rug_ed_tests_rug_853_rrrruuuugggg_test_signed_shl = 0;
    }
}
#[cfg(test)]
mod tests_rug_855 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_unsigned_shl() {
        let _rug_st_tests_rug_855_rrrruuuugggg_test_unsigned_shl = 0;
        let rug_fuzz_0 = 123;
        let rug_fuzz_1 = 5;
        let mut p0: u64 = rug_fuzz_0;
        let mut p1: u32 = rug_fuzz_1;
        <u64>::unsigned_shl(p0, p1);
        let _rug_ed_tests_rug_855_rrrruuuugggg_test_unsigned_shl = 0;
    }
}
#[cfg(test)]
mod tests_rug_858 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_858_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0b10101010101010101010101010101010;
        let mut p0: u64 = rug_fuzz_0;
        <u64>::reverse_bits(p0);
        let _rug_ed_tests_rug_858_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_860 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_860_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 12345;
        let mut p0: u64 = rug_fuzz_0;
        <u64 as PrimInt>::from_le(p0);
        let _rug_ed_tests_rug_860_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_862 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_862_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123456789;
        let mut p0: u64 = rug_fuzz_0;
        <u64>::to_le(p0);
        let _rug_ed_tests_rug_862_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_866 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_866_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1234567890;
        let mut p0: u128 = rug_fuzz_0;
        <u128>::leading_ones(p0);
        let _rug_ed_tests_rug_866_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_869 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_869_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1234567890;
        let mut p0: u128 = rug_fuzz_0;
        <u128>::trailing_zeros(p0);
        let _rug_ed_tests_rug_869_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_873 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_signed_shr() {
        let _rug_st_tests_rug_873_rrrruuuugggg_test_signed_shr = 0;
        let rug_fuzz_0 = 1234567890;
        let rug_fuzz_1 = 5;
        let mut p0: u128 = rug_fuzz_0;
        let mut p1: u32 = rug_fuzz_1;
        p0.signed_shr(p1);
        let _rug_ed_tests_rug_873_rrrruuuugggg_test_signed_shr = 0;
    }
}
#[cfg(test)]
mod tests_rug_874 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_unsigned_shl() {
        let _rug_st_tests_rug_874_rrrruuuugggg_test_unsigned_shl = 0;
        let rug_fuzz_0 = 123456789;
        let rug_fuzz_1 = 4;
        let p0: u128 = rug_fuzz_0;
        let p1: u32 = rug_fuzz_1;
        p0.unsigned_shl(p1);
        let _rug_ed_tests_rug_874_rrrruuuugggg_test_unsigned_shl = 0;
    }
}
#[cfg(test)]
mod tests_rug_875 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_875_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 12345;
        let rug_fuzz_1 = 5;
        let mut p0: u128 = rug_fuzz_0;
        let mut p1: u32 = rug_fuzz_1;
        <u128>::unsigned_shr(p0, p1);
        let _rug_ed_tests_rug_875_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_878 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_878_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0x1234567890abcdef0123456789abcdef;
        let mut p0: u128 = rug_fuzz_0;
        <u128>::from_be(p0);
        let _rug_ed_tests_rug_878_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_879 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_879_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1234567890;
        let mut p0: u128 = rug_fuzz_0;
        <u128 as PrimInt>::from_le(p0);
        let _rug_ed_tests_rug_879_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_880 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_880_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123456789;
        let mut p0: u128 = rug_fuzz_0;
        p0.to_be();
        let _rug_ed_tests_rug_880_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_881 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_881_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123456789012345678901234567890;
        let mut p0: u128 = rug_fuzz_0;
        p0.to_le();
        let _rug_ed_tests_rug_881_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_882 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_882_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 12345678901234567890;
        let rug_fuzz_1 = 5;
        let mut p0: u128 = rug_fuzz_0;
        let mut p1: u32 = rug_fuzz_1;
        <u128 as PrimInt>::pow(p0, p1);
        let _rug_ed_tests_rug_882_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_885 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_885_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: usize = rug_fuzz_0;
        p0.leading_ones();
        let _rug_ed_tests_rug_885_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_888 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_trailing_zeros() {
        let _rug_st_tests_rug_888_rrrruuuugggg_test_trailing_zeros = 0;
        let rug_fuzz_0 = 10;
        let mut p0: usize = rug_fuzz_0;
        <usize>::trailing_zeros(p0);
        let _rug_ed_tests_rug_888_rrrruuuugggg_test_trailing_zeros = 0;
    }
}
#[cfg(test)]
mod tests_rug_890 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rotate_right() {
        let _rug_st_tests_rug_890_rrrruuuugggg_test_rotate_right = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 2;
        let mut p0: usize = rug_fuzz_0;
        let mut p1: u32 = rug_fuzz_1;
        <usize as PrimInt>::rotate_right(p0, p1);
        let _rug_ed_tests_rug_890_rrrruuuugggg_test_rotate_right = 0;
    }
}
#[cfg(test)]
mod tests_rug_891 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_891_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123;
        let rug_fuzz_1 = 5;
        let mut p0: usize = rug_fuzz_0;
        let mut p1: u32 = rug_fuzz_1;
        <usize as PrimInt>::signed_shl(p0, p1);
        let _rug_ed_tests_rug_891_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_896 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_896_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: usize = rug_fuzz_0;
        p0.reverse_bits();
        let _rug_ed_tests_rug_896_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_898 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_898_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: usize = rug_fuzz_0;
        <usize>::from_le(p0);
        let _rug_ed_tests_rug_898_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_899 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_899_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: usize = rug_fuzz_0;
        <usize>::to_be(p0);
        let _rug_ed_tests_rug_899_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_900 {
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_900_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        use super::*;
        use crate::PrimInt;
        let mut p0: usize = rug_fuzz_0;
        p0.to_le();
        let _rug_ed_tests_rug_900_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_902 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_902_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: i8 = rug_fuzz_0;
        <i8>::count_ones(p0);
        let _rug_ed_tests_rug_902_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_904 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_904_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 5;
        let mut p0: i8 = rug_fuzz_0;
        p0.leading_ones();
        let _rug_ed_tests_rug_904_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_906 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_trailing_ones() {
        let _rug_st_tests_rug_906_rrrruuuugggg_test_trailing_ones = 0;
        let rug_fuzz_0 = 10;
        let p0: i8 = rug_fuzz_0;
        <i8 as PrimInt>::trailing_ones(p0);
        let _rug_ed_tests_rug_906_rrrruuuugggg_test_trailing_ones = 0;
    }
}
#[cfg(test)]
mod tests_rug_907 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_907_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: i8 = rug_fuzz_0;
        p0.trailing_zeros();
        let _rug_ed_tests_rug_907_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_910 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_signed_shl() {
        let _rug_st_tests_rug_910_rrrruuuugggg_test_signed_shl = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 3;
        let p0: i8 = -rug_fuzz_0;
        let p1: u32 = rug_fuzz_1;
        p0.signed_shl(p1);
        let _rug_ed_tests_rug_910_rrrruuuugggg_test_signed_shl = 0;
    }
}
#[cfg(test)]
mod tests_rug_912 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_912_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 3;
        let mut p0: i8 = rug_fuzz_0;
        let mut p1: u32 = rug_fuzz_1;
        <i8 as PrimInt>::unsigned_shl(p0, p1);
        let _rug_ed_tests_rug_912_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_917 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_917_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i8 = rug_fuzz_0;
        <i8>::from_le(p0);
        let _rug_ed_tests_rug_917_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_918 {
    use super::*;
    use crate::int::PrimInt;
    #[test]
    fn test_to_be() {
        let _rug_st_tests_rug_918_rrrruuuugggg_test_to_be = 0;
        let rug_fuzz_0 = 42;
        let p0: i8 = rug_fuzz_0;
        <i8 as PrimInt>::to_be(p0);
        let _rug_ed_tests_rug_918_rrrruuuugggg_test_to_be = 0;
    }
}
#[cfg(test)]
mod tests_rug_919 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_919_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i8 = rug_fuzz_0;
        <i8>::to_le(p0);
        let _rug_ed_tests_rug_919_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_921 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_921_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i16 = rug_fuzz_0;
        <i16>::count_ones(p0);
        let _rug_ed_tests_rug_921_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_927 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_927_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 12345;
        let rug_fuzz_1 = 5;
        let mut p0: i16 = -rug_fuzz_0;
        let mut p1: u32 = rug_fuzz_1;
        p0.rotate_left(p1);
        let _rug_ed_tests_rug_927_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_930 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_930_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 2;
        let p0: i16 = rug_fuzz_0;
        let p1: u32 = rug_fuzz_1;
        <i16 as PrimInt>::signed_shr(p0, p1);
        let _rug_ed_tests_rug_930_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_931 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_931_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 3;
        let mut p0: i16 = rug_fuzz_0;
        let mut p1: u32 = rug_fuzz_1;
        <i16 as PrimInt>::unsigned_shl(p0, p1);
        let _rug_ed_tests_rug_931_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_932 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_932_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let rug_fuzz_1 = 5;
        let p0: i16 = rug_fuzz_0;
        let p1: u32 = rug_fuzz_1;
        <i16 as PrimInt>::unsigned_shr(p0, p1);
        let _rug_ed_tests_rug_932_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_934 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_934_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 6;
        let mut p0: i16 = rug_fuzz_0;
        <i16>::reverse_bits(p0);
        let _rug_ed_tests_rug_934_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_937 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_937_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i16 = rug_fuzz_0;
        p0.to_be();
        let _rug_ed_tests_rug_937_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_938 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_938_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i16 = rug_fuzz_0;
        <i16>::to_le(p0);
        let _rug_ed_tests_rug_938_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_939 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_939_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 2;
        let mut p0: i16 = rug_fuzz_0;
        let mut p1: u32 = rug_fuzz_1;
        p0.pow(p1);
        let _rug_ed_tests_rug_939_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_942 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_leading_ones() {
        let _rug_st_tests_rug_942_rrrruuuugggg_test_leading_ones = 0;
        let rug_fuzz_0 = 123;
        let p0: i32 = rug_fuzz_0;
        <i32 as PrimInt>::leading_ones(p0);
        let _rug_ed_tests_rug_942_rrrruuuugggg_test_leading_ones = 0;
    }
}
#[cfg(test)]
mod tests_rug_943 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_943_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123;
        let p0: i32 = rug_fuzz_0;
        p0.leading_zeros();
        let _rug_ed_tests_rug_943_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_945 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_945_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: i32 = rug_fuzz_0;
        <i32 as PrimInt>::trailing_zeros(p0);
        let _rug_ed_tests_rug_945_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_950 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_950_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 2;
        let mut p0: i32 = rug_fuzz_0;
        let mut p1: u32 = rug_fuzz_1;
        p0.unsigned_shl(p1);
        let _rug_ed_tests_rug_950_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_952 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_952_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 12345;
        let mut p0: i32 = rug_fuzz_0;
        p0.swap_bytes();
        let _rug_ed_tests_rug_952_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_953 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_953_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: i32 = rug_fuzz_0;
        p0.reverse_bits();
        let _rug_ed_tests_rug_953_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_954 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_954_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 200;
        let mut p0: i32 = rug_fuzz_0;
        <i32 as PrimInt>::from_be(p0);
        let _rug_ed_tests_rug_954_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_955 {
    use super::*;
    use crate::int::PrimInt;
    #[test]
    fn test_from_le() {
        let _rug_st_tests_rug_955_rrrruuuugggg_test_from_le = 0;
        let rug_fuzz_0 = 123;
        let p0: i32 = rug_fuzz_0;
        <i32 as PrimInt>::from_le(p0);
        let _rug_ed_tests_rug_955_rrrruuuugggg_test_from_le = 0;
    }
}
#[cfg(test)]
mod tests_rug_956 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_956_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i32 = rug_fuzz_0;
        <i32>::to_be(p0);
        let _rug_ed_tests_rug_956_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_957 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_957_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123;
        let mut p0: i32 = rug_fuzz_0;
        <i32>::to_le(p0);
        let _rug_ed_tests_rug_957_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_961 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_961_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 12345;
        let mut p0: i64 = rug_fuzz_0;
        <i64>::leading_ones(p0);
        let _rug_ed_tests_rug_961_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_963 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_963_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: i64 = rug_fuzz_0;
        p0.trailing_ones();
        let _rug_ed_tests_rug_963_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_965 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rotate_left() {
        let _rug_st_tests_rug_965_rrrruuuugggg_test_rotate_left = 0;
        let rug_fuzz_0 = 123;
        let rug_fuzz_1 = 4;
        let mut p0: i64 = rug_fuzz_0;
        let mut p1: u32 = rug_fuzz_1;
        p0.rotate_left(p1);
        let _rug_ed_tests_rug_965_rrrruuuugggg_test_rotate_left = 0;
    }
}
#[cfg(test)]
mod tests_rug_968 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_signed_shr() {
        let _rug_st_tests_rug_968_rrrruuuugggg_test_signed_shr = 0;
        let rug_fuzz_0 = 100;
        let rug_fuzz_1 = 3;
        let p0: i64 = rug_fuzz_0;
        let p1: u32 = rug_fuzz_1;
        <i64 as PrimInt>::signed_shr(p0, p1);
        let _rug_ed_tests_rug_968_rrrruuuugggg_test_signed_shr = 0;
    }
}
#[cfg(test)]
mod tests_rug_971 {
    use super::*;
    use crate::int::PrimInt;
    #[test]
    fn test_swap_bytes() {
        let _rug_st_tests_rug_971_rrrruuuugggg_test_swap_bytes = 0;
        let rug_fuzz_0 = 1234567890;
        let mut p0: i64 = rug_fuzz_0;
        p0.swap_bytes();
        let _rug_ed_tests_rug_971_rrrruuuugggg_test_swap_bytes = 0;
    }
}
#[cfg(test)]
mod tests_rug_972 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_972_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123456789;
        let mut p0: i64 = rug_fuzz_0;
        p0.reverse_bits();
        let _rug_ed_tests_rug_972_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_977 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_977_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 3;
        let mut p0: i64 = rug_fuzz_0;
        let mut p1: u32 = rug_fuzz_1;
        p0.pow(p1);
        let _rug_ed_tests_rug_977_rrrruuuugggg_test_rug = 0;
    }
}
#[test]
fn test_rug() {
    let mut p0: i128 = 123;
    p0.count_zeros();
}
#[cfg(test)]
mod tests_rug_983 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_trailing_zeros() {
        let _rug_st_tests_rug_983_rrrruuuugggg_test_trailing_zeros = 0;
        let rug_fuzz_0 = 123456789;
        let mut p0: i128 = rug_fuzz_0;
        p0.trailing_zeros();
        let _rug_ed_tests_rug_983_rrrruuuugggg_test_trailing_zeros = 0;
    }
}
#[cfg(test)]
mod tests_rug_986 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_signed_shl() {
        let _rug_st_tests_rug_986_rrrruuuugggg_test_signed_shl = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 3;
        let p0: i128 = rug_fuzz_0;
        let p1: u32 = rug_fuzz_1;
        p0.signed_shl(p1);
        let _rug_ed_tests_rug_986_rrrruuuugggg_test_signed_shl = 0;
    }
}
mod tests_rug_987 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_signed_shr() {
        let _rug_st_tests_rug_987_rrrruuuugggg_test_signed_shr = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 3;
        let mut p0: i128 = rug_fuzz_0;
        let mut p1: u32 = rug_fuzz_1;
        <i128 as PrimInt>::signed_shr(p0, p1);
        let _rug_ed_tests_rug_987_rrrruuuugggg_test_signed_shr = 0;
    }
}
#[cfg(test)]
mod tests_rug_988 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_988_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123456789012345678901234567890;
        let rug_fuzz_1 = 10;
        let mut p0: i128 = rug_fuzz_0;
        let mut p1: u32 = rug_fuzz_1;
        p0.unsigned_shl(p1);
        let _rug_ed_tests_rug_988_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_992 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_992_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1234567890;
        let mut p0: i128 = rug_fuzz_0;
        <i128 as PrimInt>::from_be(p0);
        let _rug_ed_tests_rug_992_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_994 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_994_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123456789;
        let mut p0: i128 = rug_fuzz_0;
        p0.to_be();
        let _rug_ed_tests_rug_994_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_995 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_995_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1234567890;
        let mut p0: i128 = rug_fuzz_0;
        <i128 as PrimInt>::to_le(p0);
        let _rug_ed_tests_rug_995_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_996 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_996_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123456789012345678901234567890;
        let rug_fuzz_1 = 10;
        let mut p0: i128 = rug_fuzz_0;
        let mut p1: u32 = rug_fuzz_1;
        <i128>::pow(p0, p1);
        let _rug_ed_tests_rug_996_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_997 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_997_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 15;
        let mut p0: isize = -rug_fuzz_0;
        <isize as crate::int::PrimInt>::count_ones(p0);
        let _rug_ed_tests_rug_997_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_1001 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_1001_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: isize = rug_fuzz_0;
        <isize as PrimInt>::trailing_ones(p0);
        let _rug_ed_tests_rug_1001_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_1003 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rotate_left() {
        let _rug_st_tests_rug_1003_rrrruuuugggg_test_rotate_left = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 3;
        let p0: isize = rug_fuzz_0;
        let p1: u32 = rug_fuzz_1;
        <isize as PrimInt>::rotate_left(p0, p1);
        let _rug_ed_tests_rug_1003_rrrruuuugggg_test_rotate_left = 0;
    }
}
#[cfg(test)]
mod tests_rug_1004 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_1004_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 3;
        let p0: isize = rug_fuzz_0;
        let p1: u32 = rug_fuzz_1;
        p0.rotate_right(p1);
        let _rug_ed_tests_rug_1004_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_1006 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_1006_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 3;
        let mut p0: isize = rug_fuzz_0;
        let mut p1: u32 = rug_fuzz_1;
        <isize as PrimInt>::signed_shr(p0, p1);
        let _rug_ed_tests_rug_1006_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_1009 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_1009_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 12345;
        let mut p0: isize = rug_fuzz_0;
        <isize>::swap_bytes(p0);
        let _rug_ed_tests_rug_1009_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_1010 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_reverse_bits() {
        let _rug_st_tests_rug_1010_rrrruuuugggg_test_reverse_bits = 0;
        let rug_fuzz_0 = 10;
        let p0: isize = rug_fuzz_0;
        <isize>::reverse_bits(p0);
        let _rug_ed_tests_rug_1010_rrrruuuugggg_test_reverse_bits = 0;
    }
}
#[cfg(test)]
mod tests_rug_1011 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_from_be() {
        let _rug_st_tests_rug_1011_rrrruuuugggg_test_from_be = 0;
        let rug_fuzz_0 = 123;
        let p0: isize = rug_fuzz_0;
        <isize as PrimInt>::from_be(p0);
        let _rug_ed_tests_rug_1011_rrrruuuugggg_test_from_be = 0;
    }
}
#[cfg(test)]
mod tests_rug_1012 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_1012_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: isize = rug_fuzz_0;
        <isize>::from_le(p0);
        let _rug_ed_tests_rug_1012_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_1013 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_1013_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: isize = rug_fuzz_0;
        p0.to_be();
        let _rug_ed_tests_rug_1013_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_1014 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_1014_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123;
        let p0: isize = rug_fuzz_0;
        <isize>::to_le(p0);
        let _rug_ed_tests_rug_1014_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_1015 {
    use super::*;
    use crate::PrimInt;
    #[test]
    fn test_pow() {
        let _rug_st_tests_rug_1015_rrrruuuugggg_test_pow = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 3;
        let p0: isize = rug_fuzz_0;
        let p1: u32 = rug_fuzz_1;
        let result = <isize as PrimInt>::pow(p0, p1);
        debug_assert_eq!(result, 125);
        let _rug_ed_tests_rug_1015_rrrruuuugggg_test_pow = 0;
    }
}
