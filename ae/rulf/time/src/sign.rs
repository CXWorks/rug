#![allow(deprecated, clippy::missing_const_for_fn)]
use core::ops::{Div, DivAssign, Mul, MulAssign, Neg, Not};
#[cfg(feature = "serde")]
use standback::convert::TryInto;
use Sign::{Negative, Positive, Zero};
/// Contains the sign of a value: positive, negative, or zero.
///
/// For ease of use, `Sign` implements [`Mul`] and [`Div`] on all signed numeric
/// types. `Sign`s can also be multiplied and divided by another `Sign`, which
/// follows the same rules as real numbers.
#[repr(i8)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(into = "crate::serde::Sign"))]
#[deprecated(
    since = "0.2.7",
    note = "The only use for this (obtaining the sign of a `Duration`) can be replaced with \
            `Duration::is_{positive|negative|zero}`"
)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Sign {
    /// A positive value.
    Positive = 1,
    /// A negative value.
    Negative = -1,
    /// A value that is exactly zero.
    Zero = 0,
}
#[cfg(feature = "serde")]
impl<'a> serde::Deserialize<'a> for Sign {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        crate::serde::Sign::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}
impl Default for Sign {
    /// `Sign` defaults to `Zero`.
    ///
    /// ```rust
    /// # #![allow(deprecated)]
    /// # use time::Sign;
    /// assert_eq!(Sign::default(), Sign::Zero);
    /// ```
    fn default() -> Self {
        Zero
    }
}
macro_rules! sign_mul {
    ($($type:ty),+ $(,)?) => {
        $(impl Mul <$type > for Sign { type Output = $type;
        #[allow(trivial_numeric_casts)] fn mul(self, rhs : $type) -> Self::Output { (self
        as i8) as $type * rhs } } impl Mul < Sign > for $type { type Output = Self; fn
        mul(self, rhs : Sign) -> Self::Output { rhs * self } } impl MulAssign < Sign >
        for $type { #[allow(trivial_numeric_casts)] fn mul_assign(& mut self, rhs : Sign)
        { * self *= rhs as i8 as $type; } } impl Div < Sign > for $type { type Output =
        Self; fn div(self, rhs : Sign) -> Self::Output { self * rhs } } impl DivAssign <
        Sign > for $type { fn div_assign(& mut self, rhs : Sign) { * self *= rhs } })*
    };
}
sign_mul![i8, i16, i32, i64, i128, f32, f64];
impl Mul<Sign> for Sign {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Zero, _) | (_, Zero) => Zero,
            (Positive, Positive) | (Negative, Negative) => Positive,
            (Positive, Negative) | (Negative, Positive) => Negative,
        }
    }
}
impl MulAssign<Sign> for Sign {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}
impl Div<Sign> for Sign {
    type Output = Self;
    fn div(self, rhs: Self) -> Self::Output {
        self * rhs
    }
}
impl DivAssign<Sign> for Sign {
    fn div_assign(&mut self, rhs: Self) {
        *self *= rhs;
    }
}
impl Neg for Sign {
    type Output = Self;
    fn neg(self) -> Self::Output {
        self.negate()
    }
}
impl Not for Sign {
    type Output = Self;
    fn not(self) -> Self::Output {
        self.negate()
    }
}
impl Sign {
    /// Return the opposite of the current sign.
    ///
    /// ```rust
    /// # #![allow(deprecated)]
    /// # use time::Sign;
    /// assert_eq!(Sign::Positive.negate(), Sign::Negative);
    /// assert_eq!(Sign::Negative.negate(), Sign::Positive);
    /// assert_eq!(Sign::Zero.negate(), Sign::Zero);
    /// ```
    pub fn negate(self) -> Self {
        match self {
            Positive => Negative,
            Negative => Positive,
            Zero => Zero,
        }
    }
    /// Is the sign positive?
    ///
    /// ```rust
    /// # #![allow(deprecated)]
    /// # use time::Sign;
    /// assert!(Sign::Positive.is_positive());
    /// assert!(!Sign::Negative.is_positive());
    /// assert!(!Sign::Zero.is_positive());
    /// ```
    pub const fn is_positive(self) -> bool {
        self as u8 == Positive as u8
    }
    /// Is the sign negative?
    ///
    /// ```rust
    /// # #![allow(deprecated)]
    /// # use time::Sign;
    /// assert!(!Sign::Positive.is_negative());
    /// assert!(Sign::Negative.is_negative());
    /// assert!(!Sign::Zero.is_negative());
    /// ```
    pub const fn is_negative(self) -> bool {
        self as u8 == Negative as u8
    }
    /// Is the value exactly zero?
    ///
    /// ```rust
    /// # #![allow(deprecated)]
    /// # use time::Sign;
    /// assert!(!Sign::Positive.is_zero());
    /// assert!(!Sign::Negative.is_zero());
    /// assert!(Sign::Zero.is_zero());
    /// ```
    pub const fn is_zero(self) -> bool {
        self as u8 == Zero as u8
    }
}
#[cfg(test)]
mod tests_llm_16_282 {
    use super::*;
    use crate::*;
    #[test]
    fn test_default() {
        let _rug_st_tests_llm_16_282_rrrruuuugggg_test_default = 0;
        debug_assert_eq!(
            < sign::Sign as std::default::Default > ::default(), sign::Sign::Zero
        );
        let _rug_ed_tests_llm_16_282_rrrruuuugggg_test_default = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_284_llm_16_283 {
    use crate::sign::Sign;
    use std::ops::Div;
    #[test]
    fn test_div() {
        let _rug_st_tests_llm_16_284_llm_16_283_rrrruuuugggg_test_div = 0;
        let sign1 = Sign::Positive;
        let sign2 = Sign::Negative;
        let sign3 = Sign::Zero;
        let result1 = sign1.div(sign2);
        let result2 = sign2.div(sign3);
        let result3 = sign3.div(sign1);
        debug_assert_eq!(result1, Sign::Negative);
        debug_assert_eq!(result2, Sign::Zero);
        debug_assert_eq!(result3, Sign::Zero);
        let _rug_ed_tests_llm_16_284_llm_16_283_rrrruuuugggg_test_div = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_286_llm_16_285 {
    use crate::sign::Sign;
    use std::ops::DivAssign;
    #[test]
    fn test_div_assign() {
        let _rug_st_tests_llm_16_286_llm_16_285_rrrruuuugggg_test_div_assign = 0;
        let mut sign = Sign::Positive;
        sign.div_assign(Sign::Negative);
        debug_assert_eq!(sign, Sign::Negative);
        let mut sign = Sign::Positive;
        sign.div_assign(Sign::Zero);
        debug_assert_eq!(sign, Sign::Zero);
        let mut sign = Sign::Negative;
        sign.div_assign(Sign::Positive);
        debug_assert_eq!(sign, Sign::Negative);
        let mut sign = Sign::Negative;
        sign.div_assign(Sign::Negative);
        debug_assert_eq!(sign, Sign::Positive);
        let mut sign = Sign::Zero;
        sign.div_assign(Sign::Positive);
        debug_assert_eq!(sign, Sign::Zero);
        let mut sign = Sign::Zero;
        sign.div_assign(Sign::Negative);
        debug_assert_eq!(sign, Sign::Zero);
        let _rug_ed_tests_llm_16_286_llm_16_285_rrrruuuugggg_test_div_assign = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_291 {
    use super::*;
    use crate::*;
    use std::ops::Mul;
    #[test]
    fn test_mul() {
        let _rug_st_tests_llm_16_291_rrrruuuugggg_test_mul = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 10.5;
        let rug_fuzz_4 = 10.5;
        let rug_fuzz_5 = 10.5;
        let sign_pos = Sign::Positive;
        let sign_neg = Sign::Negative;
        let sign_zero = Sign::Zero;
        let result_pos = sign_pos.mul(rug_fuzz_0);
        let result_neg = sign_neg.mul(rug_fuzz_1);
        let result_zero = sign_zero.mul(rug_fuzz_2);
        debug_assert_eq!(result_pos, 10);
        debug_assert_eq!(result_neg, - 10);
        debug_assert_eq!(result_zero, 0);
        let result_pos_float = sign_pos.mul(rug_fuzz_3);
        let result_neg_float = sign_neg.mul(rug_fuzz_4);
        let result_zero_float = sign_zero.mul(rug_fuzz_5);
        debug_assert_eq!(result_pos_float, 10.5);
        debug_assert_eq!(result_neg_float, - 10.5);
        debug_assert_eq!(result_zero_float, 0.0);
        let _rug_ed_tests_llm_16_291_rrrruuuugggg_test_mul = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_292 {
    use super::*;
    use crate::*;
    use sign::Sign;
    #[test]
    fn test_mul_positive_positive() {
        let _rug_st_tests_llm_16_292_rrrruuuugggg_test_mul_positive_positive = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let sign = Sign::Positive;
        let rhs = rug_fuzz_0;
        let expected = rug_fuzz_1;
        let result = sign.mul(rhs);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_292_rrrruuuugggg_test_mul_positive_positive = 0;
    }
    #[test]
    fn test_mul_positive_negative() {
        let _rug_st_tests_llm_16_292_rrrruuuugggg_test_mul_positive_negative = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let sign = Sign::Positive;
        let rhs = -rug_fuzz_0;
        let expected = -rug_fuzz_1;
        let result = sign.mul(rhs);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_292_rrrruuuugggg_test_mul_positive_negative = 0;
    }
    #[test]
    fn test_mul_positive_zero() {
        let _rug_st_tests_llm_16_292_rrrruuuugggg_test_mul_positive_zero = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let sign = Sign::Positive;
        let rhs = rug_fuzz_0;
        let expected = rug_fuzz_1;
        let result = sign.mul(rhs);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_292_rrrruuuugggg_test_mul_positive_zero = 0;
    }
    #[test]
    fn test_mul_negative_positive() {
        let _rug_st_tests_llm_16_292_rrrruuuugggg_test_mul_negative_positive = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let sign = Sign::Negative;
        let rhs = rug_fuzz_0;
        let expected = -rug_fuzz_1;
        let result = sign.mul(rhs);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_292_rrrruuuugggg_test_mul_negative_positive = 0;
    }
    #[test]
    fn test_mul_negative_negative() {
        let _rug_st_tests_llm_16_292_rrrruuuugggg_test_mul_negative_negative = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let sign = Sign::Negative;
        let rhs = -rug_fuzz_0;
        let expected = rug_fuzz_1;
        let result = sign.mul(rhs);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_292_rrrruuuugggg_test_mul_negative_negative = 0;
    }
    #[test]
    fn test_mul_negative_zero() {
        let _rug_st_tests_llm_16_292_rrrruuuugggg_test_mul_negative_zero = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let sign = Sign::Negative;
        let rhs = rug_fuzz_0;
        let expected = rug_fuzz_1;
        let result = sign.mul(rhs);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_292_rrrruuuugggg_test_mul_negative_zero = 0;
    }
    #[test]
    fn test_mul_zero_positive() {
        let _rug_st_tests_llm_16_292_rrrruuuugggg_test_mul_zero_positive = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 0;
        let sign = Sign::Zero;
        let rhs = rug_fuzz_0;
        let expected = rug_fuzz_1;
        let result = sign.mul(rhs);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_292_rrrruuuugggg_test_mul_zero_positive = 0;
    }
    #[test]
    fn test_mul_zero_negative() {
        let _rug_st_tests_llm_16_292_rrrruuuugggg_test_mul_zero_negative = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 0;
        let sign = Sign::Zero;
        let rhs = -rug_fuzz_0;
        let expected = rug_fuzz_1;
        let result = sign.mul(rhs);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_292_rrrruuuugggg_test_mul_zero_negative = 0;
    }
    #[test]
    fn test_mul_zero_zero() {
        let _rug_st_tests_llm_16_292_rrrruuuugggg_test_mul_zero_zero = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let sign = Sign::Zero;
        let rhs = rug_fuzz_0;
        let expected = rug_fuzz_1;
        let result = sign.mul(rhs);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_292_rrrruuuugggg_test_mul_zero_zero = 0;
    }
}
#[allow(trivial_numeric_casts)]
#[test]
fn test_mul() {
    use crate::Sign;
    assert_eq!(Sign::Positive.mul(5), 5);
    assert_eq!(Sign::Negative.mul(5), - 5);
    assert_eq!(Sign::Zero.mul(5), 0);
    assert_eq!(Sign::Positive * Sign::Positive, Sign::Positive);
    assert_eq!(Sign::Positive * Sign::Negative, Sign::Negative);
    assert_eq!(Sign::Negative * Sign::Positive, Sign::Negative);
    assert_eq!(Sign::Negative * Sign::Negative, Sign::Positive);
    assert_eq!(Sign::Positive / Sign::Positive, Sign::Positive);
    assert_eq!(Sign::Positive / Sign::Negative, Sign::Negative);
    assert_eq!(Sign::Negative / Sign::Positive, Sign::Negative);
    assert_eq!(Sign::Negative / Sign::Negative, Sign::Positive);
    assert_eq!(- Sign::Positive, Sign::Negative);
    assert_eq!(- Sign::Negative, Sign::Positive);
    assert_eq!(- Sign::Zero, Sign::Zero);
    assert_eq!(! Sign::Positive, Sign::Negative);
    assert_eq!(! Sign::Negative, Sign::Positive);
    assert_eq!(! Sign::Zero, Sign::Zero);
    assert!(Sign::Positive.is_positive());
    assert!(! Sign::Negative.is_positive());
    assert!(! Sign::Zero.is_positive());
    assert!(! Sign::Positive.is_negative());
    assert!(Sign::Negative.is_negative());
    assert!(! Sign::Zero.is_negative());
    assert!(! Sign::Positive.is_zero());
    assert!(! Sign::Negative.is_zero());
    assert!(Sign::Zero.is_zero());
}
#[cfg(test)]
mod tests_llm_16_295_llm_16_294 {
    use crate::sign::Sign;
    use std::ops::Mul;
    #[test]
    fn test_mul() {
        let _rug_st_tests_llm_16_295_llm_16_294_rrrruuuugggg_test_mul = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 5;
        let rug_fuzz_2 = 5;
        let sign_1 = Sign::Positive;
        let sign_2 = Sign::Negative;
        let sign_3 = Sign::Zero;
        let result_1 = sign_1.mul(rug_fuzz_0);
        let result_2 = sign_2.mul(rug_fuzz_1);
        let result_3 = sign_3.mul(rug_fuzz_2);
        debug_assert_eq!(result_1, 5);
        debug_assert_eq!(result_2, - 5);
        debug_assert_eq!(result_3, 0);
        let _rug_ed_tests_llm_16_295_llm_16_294_rrrruuuugggg_test_mul = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_299_llm_16_298 {
    use crate::sign::Sign;
    use std::ops::Mul;
    #[test]
    fn test_mul() {
        let _rug_st_tests_llm_16_299_llm_16_298_rrrruuuugggg_test_mul = 0;
        debug_assert_eq!(Sign::Zero.mul(Sign::Positive), Sign::Zero);
        debug_assert_eq!(Sign::Zero.mul(Sign::Negative), Sign::Zero);
        debug_assert_eq!(Sign::Positive.mul(Sign::Zero), Sign::Zero);
        debug_assert_eq!(Sign::Negative.mul(Sign::Zero), Sign::Zero);
        debug_assert_eq!(Sign::Positive.mul(Sign::Positive), Sign::Positive);
        debug_assert_eq!(Sign::Negative.mul(Sign::Negative), Sign::Positive);
        debug_assert_eq!(Sign::Positive.mul(Sign::Negative), Sign::Negative);
        debug_assert_eq!(Sign::Negative.mul(Sign::Positive), Sign::Negative);
        let _rug_ed_tests_llm_16_299_llm_16_298_rrrruuuugggg_test_mul = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_300 {
    use super::*;
    use crate::*;
    use crate::Sign;
    #[test]
    fn test_mul_assign() {
        let _rug_st_tests_llm_16_300_rrrruuuugggg_test_mul_assign = 0;
        let mut sign = Sign::Positive;
        sign.mul_assign(Sign::Negative);
        debug_assert_eq!(sign, Sign::Negative);
        let mut sign = Sign::Negative;
        sign.mul_assign(Sign::Negative);
        debug_assert_eq!(sign, Sign::Positive);
        let mut sign = Sign::Negative;
        sign.mul_assign(Sign::Positive);
        debug_assert_eq!(sign, Sign::Negative);
        let mut sign = Sign::Zero;
        sign.mul_assign(Sign::Positive);
        debug_assert_eq!(sign, Sign::Zero);
        let _rug_ed_tests_llm_16_300_rrrruuuugggg_test_mul_assign = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_301 {
    use super::*;
    use crate::*;
    #[test]
    fn test_neg() {
        let _rug_st_tests_llm_16_301_rrrruuuugggg_test_neg = 0;
        debug_assert_eq!(Sign::Positive.neg(), Sign::Negative);
        debug_assert_eq!(Sign::Negative.neg(), Sign::Positive);
        debug_assert_eq!(Sign::Zero.neg(), Sign::Zero);
        let _rug_ed_tests_llm_16_301_rrrruuuugggg_test_neg = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_302 {
    use super::*;
    use crate::*;
    #[test]
    fn test_not_positive() {
        let _rug_st_tests_llm_16_302_rrrruuuugggg_test_not_positive = 0;
        let sign = sign::Sign::Positive;
        let result = <sign::Sign as std::ops::Not>::not(sign);
        debug_assert_eq!(result, sign::Sign::Negative);
        let _rug_ed_tests_llm_16_302_rrrruuuugggg_test_not_positive = 0;
    }
    #[test]
    fn test_not_negative() {
        let _rug_st_tests_llm_16_302_rrrruuuugggg_test_not_negative = 0;
        let sign = sign::Sign::Negative;
        let result = <sign::Sign as std::ops::Not>::not(sign);
        debug_assert_eq!(result, sign::Sign::Positive);
        let _rug_ed_tests_llm_16_302_rrrruuuugggg_test_not_negative = 0;
    }
    #[test]
    fn test_not_zero() {
        let _rug_st_tests_llm_16_302_rrrruuuugggg_test_not_zero = 0;
        let sign = sign::Sign::Zero;
        let result = <sign::Sign as std::ops::Not>::not(sign);
        debug_assert_eq!(result, sign::Sign::Zero);
        let _rug_ed_tests_llm_16_302_rrrruuuugggg_test_not_zero = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_946 {
    use super::*;
    use crate::*;
    #[test]
    fn test_div() {
        let _rug_st_tests_llm_16_946_rrrruuuugggg_test_div = 0;
        let rug_fuzz_0 = 10.0;
        let sign_positive = Sign::Positive;
        let sign_negative = Sign::Negative;
        let sign_zero = Sign::Zero;
        let value = rug_fuzz_0;
        let result_positive = value.div(sign_positive);
        let result_negative = value.div(sign_negative);
        let result_zero = value.div(sign_zero);
        debug_assert_eq!(result_positive, value);
        debug_assert_eq!(result_negative, - value);
        debug_assert_eq!(result_zero, 0.0);
        let _rug_ed_tests_llm_16_946_rrrruuuugggg_test_div = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_949 {
    use super::*;
    use crate::*;
    #[test]
    fn test_div() {
        let _rug_st_tests_llm_16_949_rrrruuuugggg_test_div = 0;
        let a = Sign::Positive;
        let b = Sign::Negative;
        debug_assert_eq!(a.div(b), Sign::Negative);
        let c = Sign::Negative;
        let d = Sign::Positive;
        debug_assert_eq!(c.div(d), Sign::Negative);
        let e = Sign::Positive;
        let f = Sign::Zero;
        debug_assert_eq!(e.div(f), Sign::Zero);
        let g = Sign::Zero;
        let h = Sign::Negative;
        debug_assert_eq!(g.div(h), Sign::Zero);
        let i = Sign::Zero;
        let j = Sign::Positive;
        debug_assert_eq!(i.div(j), Sign::Zero);
        let k = Sign::Positive;
        let l = Sign::Positive;
        debug_assert_eq!(k.div(l), Sign::Positive);
        let m = Sign::Negative;
        let n = Sign::Negative;
        debug_assert_eq!(m.div(n), Sign::Positive);
        let _rug_ed_tests_llm_16_949_rrrruuuugggg_test_div = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_951 {
    use crate::sign::Sign;
    use std::ops::Div;
    #[test]
    fn test_div() {
        let _rug_st_tests_llm_16_951_rrrruuuugggg_test_div = 0;
        let sign1 = Sign::Positive;
        let sign2 = Sign::Negative;
        let result = sign1.div(sign2);
        debug_assert_eq!(result, Sign::Negative);
        let _rug_ed_tests_llm_16_951_rrrruuuugggg_test_div = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_952 {
    use crate::sign::Sign;
    use std::ops::Div;
    #[test]
    fn test_div_positive_positive() {
        let _rug_st_tests_llm_16_952_rrrruuuugggg_test_div_positive_positive = 0;
        let result = Sign::Positive.div(Sign::Positive);
        debug_assert_eq!(result, Sign::Positive);
        let _rug_ed_tests_llm_16_952_rrrruuuugggg_test_div_positive_positive = 0;
    }
    #[test]
    fn test_div_positive_negative() {
        let _rug_st_tests_llm_16_952_rrrruuuugggg_test_div_positive_negative = 0;
        let result = Sign::Positive.div(Sign::Negative);
        debug_assert_eq!(result, Sign::Negative);
        let _rug_ed_tests_llm_16_952_rrrruuuugggg_test_div_positive_negative = 0;
    }
    #[test]
    fn test_div_positive_zero() {
        let _rug_st_tests_llm_16_952_rrrruuuugggg_test_div_positive_zero = 0;
        let result = Sign::Positive.div(Sign::Zero);
        debug_assert_eq!(result, Sign::Zero);
        let _rug_ed_tests_llm_16_952_rrrruuuugggg_test_div_positive_zero = 0;
    }
    #[test]
    fn test_div_negative_positive() {
        let _rug_st_tests_llm_16_952_rrrruuuugggg_test_div_negative_positive = 0;
        let result = Sign::Negative.div(Sign::Positive);
        debug_assert_eq!(result, Sign::Negative);
        let _rug_ed_tests_llm_16_952_rrrruuuugggg_test_div_negative_positive = 0;
    }
    #[test]
    fn test_div_negative_negative() {
        let _rug_st_tests_llm_16_952_rrrruuuugggg_test_div_negative_negative = 0;
        let result = Sign::Negative.div(Sign::Negative);
        debug_assert_eq!(result, Sign::Positive);
        let _rug_ed_tests_llm_16_952_rrrruuuugggg_test_div_negative_negative = 0;
    }
    #[test]
    fn test_div_negative_zero() {
        let _rug_st_tests_llm_16_952_rrrruuuugggg_test_div_negative_zero = 0;
        let result = Sign::Negative.div(Sign::Zero);
        debug_assert_eq!(result, Sign::Zero);
        let _rug_ed_tests_llm_16_952_rrrruuuugggg_test_div_negative_zero = 0;
    }
    #[test]
    fn test_div_zero_positive() {
        let _rug_st_tests_llm_16_952_rrrruuuugggg_test_div_zero_positive = 0;
        let result = Sign::Zero.div(Sign::Positive);
        debug_assert_eq!(result, Sign::Zero);
        let _rug_ed_tests_llm_16_952_rrrruuuugggg_test_div_zero_positive = 0;
    }
    #[test]
    fn test_div_zero_negative() {
        let _rug_st_tests_llm_16_952_rrrruuuugggg_test_div_zero_negative = 0;
        let result = Sign::Zero.div(Sign::Negative);
        debug_assert_eq!(result, Sign::Zero);
        let _rug_ed_tests_llm_16_952_rrrruuuugggg_test_div_zero_negative = 0;
    }
    #[test]
    fn test_div_zero_zero() {
        let _rug_st_tests_llm_16_952_rrrruuuugggg_test_div_zero_zero = 0;
        let result = Sign::Zero.div(Sign::Zero);
        debug_assert_eq!(result, Sign::Zero);
        let _rug_ed_tests_llm_16_952_rrrruuuugggg_test_div_zero_zero = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_954 {
    use crate::sign::Sign;
    use std::ops::Div;
    #[test]
    fn test_div() {
        let _rug_st_tests_llm_16_954_rrrruuuugggg_test_div = 0;
        let sign1 = Sign::Positive;
        let sign2 = Sign::Negative;
        let result = sign1.div(sign2);
        let _rug_ed_tests_llm_16_954_rrrruuuugggg_test_div = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_955 {
    use super::*;
    use crate::*;
    #[test]
    fn test_div_assign() {
        let _rug_st_tests_llm_16_955_rrrruuuugggg_test_div_assign = 0;
        let mut sign = Sign::Positive;
        sign.div_assign(Sign::Negative);
        debug_assert_eq!(sign, Sign::Negative);
        let mut sign = Sign::Negative;
        sign.div_assign(Sign::Positive);
        debug_assert_eq!(sign, Sign::Negative);
        let mut sign = Sign::Negative;
        sign.div_assign(Sign::Negative);
        debug_assert_eq!(sign, Sign::Positive);
        let mut sign = Sign::Zero;
        sign.div_assign(Sign::Positive);
        debug_assert_eq!(sign, Sign::Zero);
        let mut sign = Sign::Positive;
        sign.div_assign(Sign::Zero);
        debug_assert_eq!(sign, Sign::Zero);
        let _rug_ed_tests_llm_16_955_rrrruuuugggg_test_div_assign = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_956 {
    use super::*;
    use crate::*;
    use std::ops::DivAssign;
    #[test]
    fn test_div_assign() {
        let _rug_st_tests_llm_16_956_rrrruuuugggg_test_div_assign = 0;
        let mut sign = Sign::Positive;
        sign.div_assign(Sign::Negative);
        debug_assert_eq!(sign, Sign::Negative);
        let mut sign = Sign::Negative;
        sign.div_assign(Sign::Positive);
        debug_assert_eq!(sign, Sign::Negative);
        let mut sign = Sign::Negative;
        sign.div_assign(Sign::Negative);
        debug_assert_eq!(sign, Sign::Positive);
        let mut sign = Sign::Positive;
        sign.div_assign(Sign::Zero);
        debug_assert_eq!(sign, Sign::Zero);
        let _rug_ed_tests_llm_16_956_rrrruuuugggg_test_div_assign = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_957 {
    use super::*;
    use crate::*;
    #[test]
    fn test_div_assign_positive_positive() {
        let _rug_st_tests_llm_16_957_rrrruuuugggg_test_div_assign_positive_positive = 0;
        let mut sign = Sign::Positive;
        sign.div_assign(Sign::Positive);
        debug_assert_eq!(sign, Sign::Positive);
        let _rug_ed_tests_llm_16_957_rrrruuuugggg_test_div_assign_positive_positive = 0;
    }
    #[test]
    fn test_div_assign_positive_negative() {
        let _rug_st_tests_llm_16_957_rrrruuuugggg_test_div_assign_positive_negative = 0;
        let mut sign = Sign::Positive;
        sign.div_assign(Sign::Negative);
        debug_assert_eq!(sign, Sign::Negative);
        let _rug_ed_tests_llm_16_957_rrrruuuugggg_test_div_assign_positive_negative = 0;
    }
    #[test]
    fn test_div_assign_positive_zero() {
        let _rug_st_tests_llm_16_957_rrrruuuugggg_test_div_assign_positive_zero = 0;
        let mut sign = Sign::Positive;
        sign.div_assign(Sign::Zero);
        debug_assert_eq!(sign, Sign::Zero);
        let _rug_ed_tests_llm_16_957_rrrruuuugggg_test_div_assign_positive_zero = 0;
    }
    #[test]
    fn test_div_assign_negative_positive() {
        let _rug_st_tests_llm_16_957_rrrruuuugggg_test_div_assign_negative_positive = 0;
        let mut sign = Sign::Negative;
        sign.div_assign(Sign::Positive);
        debug_assert_eq!(sign, Sign::Negative);
        let _rug_ed_tests_llm_16_957_rrrruuuugggg_test_div_assign_negative_positive = 0;
    }
    #[test]
    fn test_div_assign_negative_negative() {
        let _rug_st_tests_llm_16_957_rrrruuuugggg_test_div_assign_negative_negative = 0;
        let mut sign = Sign::Negative;
        sign.div_assign(Sign::Negative);
        debug_assert_eq!(sign, Sign::Positive);
        let _rug_ed_tests_llm_16_957_rrrruuuugggg_test_div_assign_negative_negative = 0;
    }
    #[test]
    fn test_div_assign_negative_zero() {
        let _rug_st_tests_llm_16_957_rrrruuuugggg_test_div_assign_negative_zero = 0;
        let mut sign = Sign::Negative;
        sign.div_assign(Sign::Zero);
        debug_assert_eq!(sign, Sign::Zero);
        let _rug_ed_tests_llm_16_957_rrrruuuugggg_test_div_assign_negative_zero = 0;
    }
    #[test]
    fn test_div_assign_zero_positive() {
        let _rug_st_tests_llm_16_957_rrrruuuugggg_test_div_assign_zero_positive = 0;
        let mut sign = Sign::Zero;
        sign.div_assign(Sign::Positive);
        debug_assert_eq!(sign, Sign::Zero);
        let _rug_ed_tests_llm_16_957_rrrruuuugggg_test_div_assign_zero_positive = 0;
    }
    #[test]
    fn test_div_assign_zero_negative() {
        let _rug_st_tests_llm_16_957_rrrruuuugggg_test_div_assign_zero_negative = 0;
        let mut sign = Sign::Zero;
        sign.div_assign(Sign::Negative);
        debug_assert_eq!(sign, Sign::Zero);
        let _rug_ed_tests_llm_16_957_rrrruuuugggg_test_div_assign_zero_negative = 0;
    }
    #[test]
    fn test_div_assign_zero_zero() {
        let _rug_st_tests_llm_16_957_rrrruuuugggg_test_div_assign_zero_zero = 0;
        let mut sign = Sign::Zero;
        sign.div_assign(Sign::Zero);
        debug_assert_eq!(sign, Sign::Zero);
        let _rug_ed_tests_llm_16_957_rrrruuuugggg_test_div_assign_zero_zero = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_958 {
    use super::*;
    use crate::*;
    use crate::Sign;
    #[test]
    fn test_div_assign() {
        let _rug_st_tests_llm_16_958_rrrruuuugggg_test_div_assign = 0;
        let mut sign = Sign::Positive;
        sign.div_assign(Sign::Negative);
        debug_assert_eq!(sign, Sign::Negative);
        let _rug_ed_tests_llm_16_958_rrrruuuugggg_test_div_assign = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_959 {
    use crate::sign::{Sign, DivAssign};
    #[test]
    fn test_div_assign() {
        let _rug_st_tests_llm_16_959_rrrruuuugggg_test_div_assign = 0;
        let mut sign = Sign::Positive;
        sign.div_assign(Sign::Negative);
        debug_assert_eq!(sign, Sign::Negative);
        let mut sign = Sign::Negative;
        sign.div_assign(Sign::Positive);
        debug_assert_eq!(sign, Sign::Negative);
        let mut sign = Sign::Zero;
        sign.div_assign(Sign::Positive);
        debug_assert_eq!(sign, Sign::Zero);
        let mut sign = Sign::Zero;
        sign.div_assign(Sign::Negative);
        debug_assert_eq!(sign, Sign::Zero);
        let _rug_ed_tests_llm_16_959_rrrruuuugggg_test_div_assign = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_960 {
    use super::*;
    use crate::*;
    use sign::Sign;
    #[test]
    fn test_div_assign_positive_positive() {
        let _rug_st_tests_llm_16_960_rrrruuuugggg_test_div_assign_positive_positive = 0;
        let mut sign = Sign::Positive;
        sign.div_assign(Sign::Positive);
        debug_assert_eq!(sign, Sign::Positive);
        let _rug_ed_tests_llm_16_960_rrrruuuugggg_test_div_assign_positive_positive = 0;
    }
    #[test]
    fn test_div_assign_positive_negative() {
        let _rug_st_tests_llm_16_960_rrrruuuugggg_test_div_assign_positive_negative = 0;
        let mut sign = Sign::Positive;
        sign.div_assign(Sign::Negative);
        debug_assert_eq!(sign, Sign::Negative);
        let _rug_ed_tests_llm_16_960_rrrruuuugggg_test_div_assign_positive_negative = 0;
    }
    #[test]
    fn test_div_assign_positive_zero() {
        let _rug_st_tests_llm_16_960_rrrruuuugggg_test_div_assign_positive_zero = 0;
        let mut sign = Sign::Positive;
        sign.div_assign(Sign::Zero);
        debug_assert_eq!(sign, Sign::Zero);
        let _rug_ed_tests_llm_16_960_rrrruuuugggg_test_div_assign_positive_zero = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_961 {
    use super::*;
    use crate::*;
    use crate::Sign;
    #[test]
    fn test_div_assign() {
        let _rug_st_tests_llm_16_961_rrrruuuugggg_test_div_assign = 0;
        let mut sign = Sign::Positive;
        sign.div_assign(Sign::Positive);
        debug_assert_eq!(sign, Sign::Positive);
        let mut sign = Sign::Positive;
        sign.div_assign(Sign::Negative);
        debug_assert_eq!(sign, Sign::Negative);
        let mut sign = Sign::Negative;
        sign.div_assign(Sign::Positive);
        debug_assert_eq!(sign, Sign::Negative);
        let mut sign = Sign::Negative;
        sign.div_assign(Sign::Negative);
        debug_assert_eq!(sign, Sign::Positive);
        let mut sign = Sign::Zero;
        sign.div_assign(Sign::Positive);
        debug_assert_eq!(sign, Sign::Zero);
        let mut sign = Sign::Zero;
        sign.div_assign(Sign::Negative);
        debug_assert_eq!(sign, Sign::Zero);
        let _rug_ed_tests_llm_16_961_rrrruuuugggg_test_div_assign = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_963 {
    use super::*;
    use crate::*;
    #[test]
    fn test_mul_positive_positive() {
        let _rug_st_tests_llm_16_963_rrrruuuugggg_test_mul_positive_positive = 0;
        let result = Sign::Positive * Sign::Positive;
        debug_assert_eq!(result, Sign::Positive);
        let _rug_ed_tests_llm_16_963_rrrruuuugggg_test_mul_positive_positive = 0;
    }
    #[test]
    fn test_mul_positive_negative() {
        let _rug_st_tests_llm_16_963_rrrruuuugggg_test_mul_positive_negative = 0;
        let result = Sign::Positive * Sign::Negative;
        debug_assert_eq!(result, Sign::Negative);
        let _rug_ed_tests_llm_16_963_rrrruuuugggg_test_mul_positive_negative = 0;
    }
    #[test]
    fn test_mul_positive_zero() {
        let _rug_st_tests_llm_16_963_rrrruuuugggg_test_mul_positive_zero = 0;
        let result = Sign::Positive * Sign::Zero;
        debug_assert_eq!(result, Sign::Zero);
        let _rug_ed_tests_llm_16_963_rrrruuuugggg_test_mul_positive_zero = 0;
    }
    #[test]
    fn test_mul_negative_positive() {
        let _rug_st_tests_llm_16_963_rrrruuuugggg_test_mul_negative_positive = 0;
        let result = Sign::Negative * Sign::Positive;
        debug_assert_eq!(result, Sign::Negative);
        let _rug_ed_tests_llm_16_963_rrrruuuugggg_test_mul_negative_positive = 0;
    }
    #[test]
    fn test_mul_negative_negative() {
        let _rug_st_tests_llm_16_963_rrrruuuugggg_test_mul_negative_negative = 0;
        let result = Sign::Negative * Sign::Negative;
        debug_assert_eq!(result, Sign::Positive);
        let _rug_ed_tests_llm_16_963_rrrruuuugggg_test_mul_negative_negative = 0;
    }
    #[test]
    fn test_mul_negative_zero() {
        let _rug_st_tests_llm_16_963_rrrruuuugggg_test_mul_negative_zero = 0;
        let result = Sign::Negative * Sign::Zero;
        debug_assert_eq!(result, Sign::Zero);
        let _rug_ed_tests_llm_16_963_rrrruuuugggg_test_mul_negative_zero = 0;
    }
    #[test]
    fn test_mul_zero_positive() {
        let _rug_st_tests_llm_16_963_rrrruuuugggg_test_mul_zero_positive = 0;
        let result = Sign::Zero * Sign::Positive;
        debug_assert_eq!(result, Sign::Zero);
        let _rug_ed_tests_llm_16_963_rrrruuuugggg_test_mul_zero_positive = 0;
    }
    #[test]
    fn test_mul_zero_negative() {
        let _rug_st_tests_llm_16_963_rrrruuuugggg_test_mul_zero_negative = 0;
        let result = Sign::Zero * Sign::Negative;
        debug_assert_eq!(result, Sign::Zero);
        let _rug_ed_tests_llm_16_963_rrrruuuugggg_test_mul_zero_negative = 0;
    }
    #[test]
    fn test_mul_zero_zero() {
        let _rug_st_tests_llm_16_963_rrrruuuugggg_test_mul_zero_zero = 0;
        let result = Sign::Zero * Sign::Zero;
        debug_assert_eq!(result, Sign::Zero);
        let _rug_ed_tests_llm_16_963_rrrruuuugggg_test_mul_zero_zero = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_965_llm_16_964 {
    use super::*;
    use crate::*;
    use crate::sign::Sign;
    #[test]
    fn test_mul() {
        let _rug_st_tests_llm_16_965_llm_16_964_rrrruuuugggg_test_mul = 0;
        debug_assert_eq!(Sign::Positive.mul(Sign::Positive), Sign::Positive);
        debug_assert_eq!(Sign::Positive.mul(Sign::Negative), Sign::Negative);
        debug_assert_eq!(Sign::Negative.mul(Sign::Positive), Sign::Negative);
        debug_assert_eq!(Sign::Negative.mul(Sign::Negative), Sign::Positive);
        debug_assert_eq!(Sign::Zero.mul(Sign::Positive), Sign::Zero);
        debug_assert_eq!(Sign::Zero.mul(Sign::Negative), Sign::Zero);
        debug_assert_eq!(Sign::Positive.mul(Sign::Zero), Sign::Zero);
        debug_assert_eq!(Sign::Negative.mul(Sign::Zero), Sign::Zero);
        debug_assert_eq!(Sign::Zero.mul(Sign::Zero), Sign::Zero);
        let _rug_ed_tests_llm_16_965_llm_16_964_rrrruuuugggg_test_mul = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_967_llm_16_966 {
    use crate::sign::Sign;
    use std::ops::Mul;
    #[test]
    fn test_mul() {
        let _rug_st_tests_llm_16_967_llm_16_966_rrrruuuugggg_test_mul = 0;
        let sign = Sign::Positive;
        let rhs = Sign::Negative;
        let result = sign.mul(rhs);
        debug_assert_eq!(result, Sign::Negative);
        let _rug_ed_tests_llm_16_967_llm_16_966_rrrruuuugggg_test_mul = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_969_llm_16_968 {
    use super::*;
    use crate::*;
    use crate::sign::Sign;
    #[test]
    fn test_mul() {
        let _rug_st_tests_llm_16_969_llm_16_968_rrrruuuugggg_test_mul = 0;
        debug_assert_eq!(Sign::Positive.mul(Sign::Positive), Sign::Positive);
        debug_assert_eq!(Sign::Positive.mul(Sign::Negative), Sign::Negative);
        debug_assert_eq!(Sign::Positive.mul(Sign::Zero), Sign::Zero);
        debug_assert_eq!(Sign::Negative.mul(Sign::Positive), Sign::Negative);
        debug_assert_eq!(Sign::Negative.mul(Sign::Negative), Sign::Positive);
        debug_assert_eq!(Sign::Negative.mul(Sign::Zero), Sign::Zero);
        debug_assert_eq!(Sign::Zero.mul(Sign::Positive), Sign::Zero);
        debug_assert_eq!(Sign::Zero.mul(Sign::Negative), Sign::Zero);
        debug_assert_eq!(Sign::Zero.mul(Sign::Zero), Sign::Zero);
        let _rug_ed_tests_llm_16_969_llm_16_968_rrrruuuugggg_test_mul = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_971 {
    use super::*;
    use crate::*;
    #[test]
    fn test_mul() {
        let _rug_st_tests_llm_16_971_rrrruuuugggg_test_mul = 0;
        debug_assert_eq!(Sign::Positive.mul(Sign::Positive), Sign::Positive);
        debug_assert_eq!(Sign::Positive.mul(Sign::Negative), Sign::Negative);
        debug_assert_eq!(Sign::Negative.mul(Sign::Positive), Sign::Negative);
        debug_assert_eq!(Sign::Negative.mul(Sign::Negative), Sign::Positive);
        debug_assert_eq!(Sign::Positive.mul(Sign::Zero), Sign::Zero);
        debug_assert_eq!(Sign::Zero.mul(Sign::Positive), Sign::Zero);
        debug_assert_eq!(Sign::Negative.mul(Sign::Zero), Sign::Zero);
        debug_assert_eq!(Sign::Zero.mul(Sign::Negative), Sign::Zero);
        debug_assert_eq!(Sign::Zero.mul(Sign::Zero), Sign::Zero);
        let _rug_ed_tests_llm_16_971_rrrruuuugggg_test_mul = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_972 {
    use super::*;
    use crate::*;
    use crate::sign::Sign;
    #[test]
    fn test_mul_positive_positive() {
        let _rug_st_tests_llm_16_972_rrrruuuugggg_test_mul_positive_positive = 0;
        let sign1 = Sign::Positive;
        let sign2 = Sign::Positive;
        let result = sign1.mul(sign2);
        debug_assert_eq!(result, Sign::Positive);
        let _rug_ed_tests_llm_16_972_rrrruuuugggg_test_mul_positive_positive = 0;
    }
    #[test]
    fn test_mul_positive_negative() {
        let _rug_st_tests_llm_16_972_rrrruuuugggg_test_mul_positive_negative = 0;
        let sign1 = Sign::Positive;
        let sign2 = Sign::Negative;
        let result = sign1.mul(sign2);
        debug_assert_eq!(result, Sign::Negative);
        let _rug_ed_tests_llm_16_972_rrrruuuugggg_test_mul_positive_negative = 0;
    }
    #[test]
    fn test_mul_negative_positive() {
        let _rug_st_tests_llm_16_972_rrrruuuugggg_test_mul_negative_positive = 0;
        let sign1 = Sign::Negative;
        let sign2 = Sign::Positive;
        let result = sign1.mul(sign2);
        debug_assert_eq!(result, Sign::Negative);
        let _rug_ed_tests_llm_16_972_rrrruuuugggg_test_mul_negative_positive = 0;
    }
    #[test]
    fn test_mul_negative_negative() {
        let _rug_st_tests_llm_16_972_rrrruuuugggg_test_mul_negative_negative = 0;
        let sign1 = Sign::Negative;
        let sign2 = Sign::Negative;
        let result = sign1.mul(sign2);
        debug_assert_eq!(result, Sign::Positive);
        let _rug_ed_tests_llm_16_972_rrrruuuugggg_test_mul_negative_negative = 0;
    }
    #[test]
    fn test_mul_positive_zero() {
        let _rug_st_tests_llm_16_972_rrrruuuugggg_test_mul_positive_zero = 0;
        let sign1 = Sign::Positive;
        let sign2 = Sign::Zero;
        let result = sign1.mul(sign2);
        debug_assert_eq!(result, Sign::Zero);
        let _rug_ed_tests_llm_16_972_rrrruuuugggg_test_mul_positive_zero = 0;
    }
    #[test]
    fn test_mul_negative_zero() {
        let _rug_st_tests_llm_16_972_rrrruuuugggg_test_mul_negative_zero = 0;
        let sign1 = Sign::Negative;
        let sign2 = Sign::Zero;
        let result = sign1.mul(sign2);
        debug_assert_eq!(result, Sign::Zero);
        let _rug_ed_tests_llm_16_972_rrrruuuugggg_test_mul_negative_zero = 0;
    }
    #[test]
    fn test_mul_zero_positive() {
        let _rug_st_tests_llm_16_972_rrrruuuugggg_test_mul_zero_positive = 0;
        let sign1 = Sign::Zero;
        let sign2 = Sign::Positive;
        let result = sign1.mul(sign2);
        debug_assert_eq!(result, Sign::Zero);
        let _rug_ed_tests_llm_16_972_rrrruuuugggg_test_mul_zero_positive = 0;
    }
    #[test]
    fn test_mul_zero_negative() {
        let _rug_st_tests_llm_16_972_rrrruuuugggg_test_mul_zero_negative = 0;
        let sign1 = Sign::Zero;
        let sign2 = Sign::Negative;
        let result = sign1.mul(sign2);
        debug_assert_eq!(result, Sign::Zero);
        let _rug_ed_tests_llm_16_972_rrrruuuugggg_test_mul_zero_negative = 0;
    }
    #[test]
    fn test_mul_zero_zero() {
        let _rug_st_tests_llm_16_972_rrrruuuugggg_test_mul_zero_zero = 0;
        let sign1 = Sign::Zero;
        let sign2 = Sign::Zero;
        let result = sign1.mul(sign2);
        debug_assert_eq!(result, Sign::Zero);
        let _rug_ed_tests_llm_16_972_rrrruuuugggg_test_mul_zero_zero = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_973 {
    use super::*;
    use crate::*;
    #[test]
    fn test_mul_positive_positive() {
        let _rug_st_tests_llm_16_973_rrrruuuugggg_test_mul_positive_positive = 0;
        let x = Sign::Positive;
        let y = Sign::Positive;
        let expected = Sign::Positive;
        debug_assert_eq!(x.mul(y), expected);
        let _rug_ed_tests_llm_16_973_rrrruuuugggg_test_mul_positive_positive = 0;
    }
    #[test]
    fn test_mul_positive_negative() {
        let _rug_st_tests_llm_16_973_rrrruuuugggg_test_mul_positive_negative = 0;
        let x = Sign::Positive;
        let y = Sign::Negative;
        let expected = Sign::Negative;
        debug_assert_eq!(x.mul(y), expected);
        let _rug_ed_tests_llm_16_973_rrrruuuugggg_test_mul_positive_negative = 0;
    }
    #[test]
    fn test_mul_positive_zero() {
        let _rug_st_tests_llm_16_973_rrrruuuugggg_test_mul_positive_zero = 0;
        let x = Sign::Positive;
        let y = Sign::Zero;
        let expected = Sign::Zero;
        debug_assert_eq!(x.mul(y), expected);
        let _rug_ed_tests_llm_16_973_rrrruuuugggg_test_mul_positive_zero = 0;
    }
    #[test]
    fn test_mul_negative_positive() {
        let _rug_st_tests_llm_16_973_rrrruuuugggg_test_mul_negative_positive = 0;
        let x = Sign::Negative;
        let y = Sign::Positive;
        let expected = Sign::Negative;
        debug_assert_eq!(x.mul(y), expected);
        let _rug_ed_tests_llm_16_973_rrrruuuugggg_test_mul_negative_positive = 0;
    }
    #[test]
    fn test_mul_negative_negative() {
        let _rug_st_tests_llm_16_973_rrrruuuugggg_test_mul_negative_negative = 0;
        let x = Sign::Negative;
        let y = Sign::Negative;
        let expected = Sign::Positive;
        debug_assert_eq!(x.mul(y), expected);
        let _rug_ed_tests_llm_16_973_rrrruuuugggg_test_mul_negative_negative = 0;
    }
    #[test]
    fn test_mul_negative_zero() {
        let _rug_st_tests_llm_16_973_rrrruuuugggg_test_mul_negative_zero = 0;
        let x = Sign::Negative;
        let y = Sign::Zero;
        let expected = Sign::Zero;
        debug_assert_eq!(x.mul(y), expected);
        let _rug_ed_tests_llm_16_973_rrrruuuugggg_test_mul_negative_zero = 0;
    }
    #[test]
    fn test_mul_zero_positive() {
        let _rug_st_tests_llm_16_973_rrrruuuugggg_test_mul_zero_positive = 0;
        let x = Sign::Zero;
        let y = Sign::Positive;
        let expected = Sign::Zero;
        debug_assert_eq!(x.mul(y), expected);
        let _rug_ed_tests_llm_16_973_rrrruuuugggg_test_mul_zero_positive = 0;
    }
    #[test]
    fn test_mul_zero_negative() {
        let _rug_st_tests_llm_16_973_rrrruuuugggg_test_mul_zero_negative = 0;
        let x = Sign::Zero;
        let y = Sign::Negative;
        let expected = Sign::Zero;
        debug_assert_eq!(x.mul(y), expected);
        let _rug_ed_tests_llm_16_973_rrrruuuugggg_test_mul_zero_negative = 0;
    }
    #[test]
    fn test_mul_zero_zero() {
        let _rug_st_tests_llm_16_973_rrrruuuugggg_test_mul_zero_zero = 0;
        let x = Sign::Zero;
        let y = Sign::Zero;
        let expected = Sign::Zero;
        debug_assert_eq!(x.mul(y), expected);
        let _rug_ed_tests_llm_16_973_rrrruuuugggg_test_mul_zero_zero = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_974 {
    use super::*;
    use crate::*;
    use std::ops::MulAssign;
    #[test]
    fn test_mul_assign() {
        let _rug_st_tests_llm_16_974_rrrruuuugggg_test_mul_assign = 0;
        let mut sign = Sign::Positive;
        sign.mul_assign(Sign::Positive);
        debug_assert_eq!(sign, Sign::Positive);
        let mut sign = Sign::Positive;
        sign.mul_assign(Sign::Negative);
        debug_assert_eq!(sign, Sign::Negative);
        let mut sign = Sign::Positive;
        sign.mul_assign(Sign::Zero);
        debug_assert_eq!(sign, Sign::Zero);
        let mut sign = Sign::Negative;
        sign.mul_assign(Sign::Positive);
        debug_assert_eq!(sign, Sign::Negative);
        let mut sign = Sign::Negative;
        sign.mul_assign(Sign::Negative);
        debug_assert_eq!(sign, Sign::Positive);
        let mut sign = Sign::Negative;
        sign.mul_assign(Sign::Zero);
        debug_assert_eq!(sign, Sign::Zero);
        let mut sign = Sign::Zero;
        sign.mul_assign(Sign::Positive);
        debug_assert_eq!(sign, Sign::Zero);
        let mut sign = Sign::Zero;
        sign.mul_assign(Sign::Negative);
        debug_assert_eq!(sign, Sign::Zero);
        let mut sign = Sign::Zero;
        sign.mul_assign(Sign::Zero);
        debug_assert_eq!(sign, Sign::Zero);
        let _rug_ed_tests_llm_16_974_rrrruuuugggg_test_mul_assign = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_976 {
    use crate::sign::Sign;
    use std::ops::MulAssign;
    #[test]
    fn test_mul_assign() {
        let _rug_st_tests_llm_16_976_rrrruuuugggg_test_mul_assign = 0;
        let mut sign = Sign::Positive;
        sign.mul_assign(Sign::Negative);
        debug_assert_eq!(sign, Sign::Negative);
        let mut sign = Sign::Negative;
        sign.mul_assign(Sign::Positive);
        debug_assert_eq!(sign, Sign::Negative);
        let mut sign = Sign::Positive;
        sign.mul_assign(Sign::Zero);
        debug_assert_eq!(sign, Sign::Zero);
        let mut sign = Sign::Negative;
        sign.mul_assign(Sign::Zero);
        debug_assert_eq!(sign, Sign::Zero);
        let mut sign = Sign::Zero;
        sign.mul_assign(Sign::Positive);
        debug_assert_eq!(sign, Sign::Zero);
        let mut sign = Sign::Zero;
        sign.mul_assign(Sign::Negative);
        debug_assert_eq!(sign, Sign::Zero);
        let mut sign = Sign::Zero;
        sign.mul_assign(Sign::Zero);
        debug_assert_eq!(sign, Sign::Zero);
        let _rug_ed_tests_llm_16_976_rrrruuuugggg_test_mul_assign = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_977 {
    use super::*;
    use crate::*;
    use sign::Sign;
    #[test]
    fn test_mul_assign() {
        let _rug_st_tests_llm_16_977_rrrruuuugggg_test_mul_assign = 0;
        let mut sign = Sign::Positive;
        sign.mul_assign(Sign::Negative);
        debug_assert_eq!(sign, Sign::Negative);
        let mut sign = Sign::Negative;
        sign.mul_assign(Sign::Positive);
        debug_assert_eq!(sign, Sign::Negative);
        let mut sign = Sign::Zero;
        sign.mul_assign(Sign::Positive);
        debug_assert_eq!(sign, Sign::Zero);
        let mut sign = Sign::Positive;
        sign.mul_assign(Sign::Positive);
        debug_assert_eq!(sign, Sign::Positive);
        let mut sign = Sign::Negative;
        sign.mul_assign(Sign::Negative);
        debug_assert_eq!(sign, Sign::Positive);
        let mut sign = Sign::Zero;
        sign.mul_assign(Sign::Negative);
        debug_assert_eq!(sign, Sign::Zero);
        let _rug_ed_tests_llm_16_977_rrrruuuugggg_test_mul_assign = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_980 {
    use super::*;
    use crate::*;
    #[test]
    fn test_mul_assign() {
        let _rug_st_tests_llm_16_980_rrrruuuugggg_test_mul_assign = 0;
        let mut sign = Sign::Positive;
        let rhs = Sign::Negative;
        sign.mul_assign(rhs);
        debug_assert_eq!(sign, Sign::Negative);
        let mut sign = Sign::Negative;
        let rhs = Sign::Positive;
        sign.mul_assign(rhs);
        debug_assert_eq!(sign, Sign::Negative);
        let mut sign = Sign::Negative;
        let rhs = Sign::Negative;
        sign.mul_assign(rhs);
        debug_assert_eq!(sign, Sign::Positive);
        let mut sign = Sign::Positive;
        let rhs = Sign::Zero;
        sign.mul_assign(rhs);
        debug_assert_eq!(sign, Sign::Zero);
        let mut sign = Sign::Negative;
        let rhs = Sign::Zero;
        sign.mul_assign(rhs);
        debug_assert_eq!(sign, Sign::Zero);
        let mut sign = Sign::Zero;
        let rhs = Sign::Positive;
        sign.mul_assign(rhs);
        debug_assert_eq!(sign, Sign::Zero);
        let mut sign = Sign::Zero;
        let rhs = Sign::Negative;
        sign.mul_assign(rhs);
        debug_assert_eq!(sign, Sign::Zero);
        let mut sign = Sign::Zero;
        let rhs = Sign::Zero;
        sign.mul_assign(rhs);
        debug_assert_eq!(sign, Sign::Zero);
        let _rug_ed_tests_llm_16_980_rrrruuuugggg_test_mul_assign = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_981 {
    use crate::sign::Sign;
    use std::ops::MulAssign;
    #[test]
    fn test_mul_assign() {
        let _rug_st_tests_llm_16_981_rrrruuuugggg_test_mul_assign = 0;
        let mut sign = Sign::Positive;
        sign.mul_assign(Sign::Negative);
        debug_assert_eq!(sign, Sign::Negative);
        let mut sign = Sign::Negative;
        sign.mul_assign(Sign::Zero);
        debug_assert_eq!(sign, Sign::Zero);
        let mut sign = Sign::Zero;
        sign.mul_assign(Sign::Positive);
        debug_assert_eq!(sign, Sign::Zero);
        let mut sign = Sign::Zero;
        sign.mul_assign(Sign::Negative);
        debug_assert_eq!(sign, Sign::Zero);
        let mut sign = Sign::Positive;
        sign.mul_assign(Sign::Positive);
        debug_assert_eq!(sign, Sign::Positive);
        let _rug_ed_tests_llm_16_981_rrrruuuugggg_test_mul_assign = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_983_llm_16_982 {
    use crate::sign::Sign;
    use std::ops::MulAssign;
    #[test]
    fn test_mul_assign() {
        let _rug_st_tests_llm_16_983_llm_16_982_rrrruuuugggg_test_mul_assign = 0;
        let mut sign = Sign::Positive;
        sign.mul_assign(Sign::Negative);
        debug_assert_eq!(sign, Sign::Negative);
        let mut sign = Sign::Negative;
        sign.mul_assign(Sign::Positive);
        debug_assert_eq!(sign, Sign::Negative);
        let mut sign = Sign::Positive;
        sign.mul_assign(Sign::Positive);
        debug_assert_eq!(sign, Sign::Positive);
        let mut sign = Sign::Negative;
        sign.mul_assign(Sign::Negative);
        debug_assert_eq!(sign, Sign::Positive);
        let mut sign = Sign::Positive;
        sign.mul_assign(Sign::Zero);
        debug_assert_eq!(sign, Sign::Zero);
        let mut sign = Sign::Zero;
        sign.mul_assign(Sign::Positive);
        debug_assert_eq!(sign, Sign::Zero);
        let mut sign = Sign::Negative;
        sign.mul_assign(Sign::Zero);
        debug_assert_eq!(sign, Sign::Zero);
        let _rug_ed_tests_llm_16_983_llm_16_982_rrrruuuugggg_test_mul_assign = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_984 {
    use crate::Sign;
    #[test]
    #[allow(deprecated)]
    fn test_is_negative() {
        let _rug_st_tests_llm_16_984_rrrruuuugggg_test_is_negative = 0;
        debug_assert!(! Sign::Positive.is_negative());
        debug_assert!(Sign::Negative.is_negative());
        debug_assert!(! Sign::Zero.is_negative());
        let _rug_ed_tests_llm_16_984_rrrruuuugggg_test_is_negative = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_985 {
    use crate::sign::Sign;
    #[test]
    fn test_is_positive() {
        let _rug_st_tests_llm_16_985_rrrruuuugggg_test_is_positive = 0;
        debug_assert_eq!(Sign::Positive.is_positive(), true);
        debug_assert_eq!(Sign::Negative.is_positive(), false);
        debug_assert_eq!(Sign::Zero.is_positive(), false);
        let _rug_ed_tests_llm_16_985_rrrruuuugggg_test_is_positive = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_986 {
    use crate::sign::Sign;
    #[test]
    fn test_is_zero() {
        let _rug_st_tests_llm_16_986_rrrruuuugggg_test_is_zero = 0;
        debug_assert_eq!(Sign::Positive.is_zero(), false);
        debug_assert_eq!(Sign::Negative.is_zero(), false);
        debug_assert_eq!(Sign::Zero.is_zero(), true);
        let _rug_ed_tests_llm_16_986_rrrruuuugggg_test_is_zero = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_987 {
    use super::*;
    use crate::*;
    use crate::sign::Sign::*;
    #[test]
    fn test_negate() {
        let _rug_st_tests_llm_16_987_rrrruuuugggg_test_negate = 0;
        debug_assert_eq!(Positive.negate(), Negative);
        debug_assert_eq!(Negative.negate(), Positive);
        debug_assert_eq!(Zero.negate(), Zero);
        let _rug_ed_tests_llm_16_987_rrrruuuugggg_test_negate = 0;
    }
}
#[cfg(test)]
mod tests_rug_477 {
    use super::*;
    use crate::Sign;
    #[test]
    fn test_mul() {
        let _rug_st_tests_rug_477_rrrruuuugggg_test_mul = 0;
        let rug_fuzz_0 = 3;
        let mut p0: Sign = Sign::default();
        let mut p1: i8 = rug_fuzz_0;
        <Sign as std::ops::Mul<i8>>::mul(p0, p1);
        let _rug_ed_tests_rug_477_rrrruuuugggg_test_mul = 0;
    }
}
