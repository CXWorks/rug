// Copyright 2013-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Rational numbers
//!
//! ## Compatibility
//!
//! The `num-rational` crate is tested for rustc 1.31 and greater.

#![doc(html_root_url = "https://docs.rs/num-rational/0.3")]
//#![no_std]
// Ratio ops often use other "suspicious" ops
#![allow(clippy::suspicious_arithmetic_impl)]
#![allow(clippy::suspicious_op_assign_impl)]

//#[cfg(feature = "std")]
#[macro_use]
extern crate std;

use core::cmp;
use core::fmt;
use core::fmt::{Binary, Display, Formatter, LowerExp, LowerHex, Octal, UpperExp, UpperHex};
use core::hash::{Hash, Hasher};
use core::ops::{Add, Div, Mul, Neg, Rem, ShlAssign, Sub};
use core::str::FromStr;
#[cfg(feature = "std")]
use std::error::Error;

#[cfg(feature = "num-bigint")]
use num_bigint::{BigInt, BigUint, Sign, ToBigInt};

use num_integer::Integer;
use num_traits::float::FloatCore;
use num_traits::ToPrimitive;
use num_traits::{
    Bounded, CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, FromPrimitive, Inv, Num, NumCast, One,
    Pow, Signed, Zero,
};

mod pow;

/// Represents the ratio between two numbers.
#[derive(Copy, Clone, Debug)]
#[allow(missing_docs)]
pub struct Ratio<T> {
    /// Numerator.
    numer: T,
    /// Denominator.
    denom: T,
}

/// Alias for a `Ratio` of machine-sized integers.
pub type Rational = Ratio<isize>;
/// Alias for a `Ratio` of 32-bit-sized integers.
pub type Rational32 = Ratio<i32>;
/// Alias for a `Ratio` of 64-bit-sized integers.
pub type Rational64 = Ratio<i64>;

#[cfg(feature = "num-bigint")]
/// Alias for arbitrary precision rationals.
pub type BigRational = Ratio<BigInt>;

/// These method are `const` for Rust 1.31 and later.
impl<T> Ratio<T> {
    /// Creates a `Ratio` without checking for `denom == 0` or reducing.
    ///
    /// **There are several methods that will panic if used on a `Ratio` with
    /// `denom == 0`.**
    #[inline]
    pub const fn new_raw(numer: T, denom: T) -> Ratio<T> {
        Ratio { numer, denom }
    }

    /// Gets an immutable reference to the numerator.
    #[inline]
    pub const fn numer(&self) -> &T {
        &self.numer
    }

    /// Gets an immutable reference to the denominator.
    #[inline]
    pub const fn denom(&self) -> &T {
        &self.denom
    }
}

impl<T: Clone + Integer> Ratio<T> {
    /// Creates a new `Ratio`.
    ///
    /// **Panics if `denom` is zero.**
    #[inline]
    pub fn new(numer: T, denom: T) -> Ratio<T> {
        let mut ret = Ratio::new_raw(numer, denom);
        ret.reduce();
        ret
    }

    /// Creates a `Ratio` representing the integer `t`.
    #[inline]
    pub fn from_integer(t: T) -> Ratio<T> {
        Ratio::new_raw(t, One::one())
    }

    /// Converts to an integer, rounding towards zero.
    #[inline]
    pub fn to_integer(&self) -> T {
        self.trunc().numer
    }

    /// Returns true if the rational number is an integer (denominator is 1).
    #[inline]
    pub fn is_integer(&self) -> bool {
        self.denom.is_one()
    }

    /// Puts self into lowest terms, with `denom` > 0.
    ///
    /// **Panics if `denom` is zero.**
    fn reduce(&mut self) {
        if self.denom.is_zero() {
            panic!("denominator == 0");
        }
        if self.numer.is_zero() {
            self.denom.set_one();
            return;
        }
        if self.numer == self.denom {
            self.set_one();
            return;
        }
        let g: T = self.numer.gcd(&self.denom);

        // FIXME(#5992): assignment operator overloads
        // self.numer /= g;
        // T: Clone + Integer != T: Clone + NumAssign
        self.numer = self.numer.clone() / g.clone();
        // FIXME(#5992): assignment operator overloads
        // self.denom /= g;
        // T: Clone + Integer != T: Clone + NumAssign
        self.denom = self.denom.clone() / g;

        // keep denom positive!
        if self.denom < T::zero() {
            self.numer = T::zero() - self.numer.clone();
            self.denom = T::zero() - self.denom.clone();
        }
    }

    /// Returns a reduced copy of self.
    ///
    /// In general, it is not necessary to use this method, as the only
    /// method of procuring a non-reduced fraction is through `new_raw`.
    ///
    /// **Panics if `denom` is zero.**
    pub fn reduced(&self) -> Ratio<T> {
        let mut ret = self.clone();
        ret.reduce();
        ret
    }

    /// Returns the reciprocal.
    ///
    /// **Panics if the `Ratio` is zero.**
    #[inline]
    pub fn recip(&self) -> Ratio<T> {
        self.clone().into_recip()
    }

    #[inline]
    fn into_recip(self) -> Ratio<T> {
        match self.numer.cmp(&T::zero()) {
            cmp::Ordering::Equal => panic!("division by zero"),
            cmp::Ordering::Greater => Ratio::new_raw(self.denom, self.numer),
            cmp::Ordering::Less => Ratio::new_raw(T::zero() - self.denom, T::zero() - self.numer),
        }
    }

    /// Rounds towards minus infinity.
    #[inline]
    pub fn floor(&self) -> Ratio<T> {
        if *self < Zero::zero() {
            let one: T = One::one();
            Ratio::from_integer(
                (self.numer.clone() - self.denom.clone() + one) / self.denom.clone(),
            )
        } else {
            Ratio::from_integer(self.numer.clone() / self.denom.clone())
        }
    }

    /// Rounds towards plus infinity.
    #[inline]
    pub fn ceil(&self) -> Ratio<T> {
        if *self < Zero::zero() {
            Ratio::from_integer(self.numer.clone() / self.denom.clone())
        } else {
            let one: T = One::one();
            Ratio::from_integer(
                (self.numer.clone() + self.denom.clone() - one) / self.denom.clone(),
            )
        }
    }

    /// Rounds to the nearest integer. Rounds half-way cases away from zero.
    #[inline]
    pub fn round(&self) -> Ratio<T> {
        let zero: Ratio<T> = Zero::zero();
        let one: T = One::one();
        let two: T = one.clone() + one.clone();

        // Find unsigned fractional part of rational number
        let mut fractional = self.fract();
        if fractional < zero {
            fractional = zero - fractional
        };

        // The algorithm compares the unsigned fractional part with 1/2, that
        // is, a/b >= 1/2, or a >= b/2. For odd denominators, we use
        // a >= (b/2)+1. This avoids overflow issues.
        let half_or_larger = if fractional.denom.is_even() {
            fractional.numer >= fractional.denom / two
        } else {
            fractional.numer >= (fractional.denom / two) + one
        };

        if half_or_larger {
            let one: Ratio<T> = One::one();
            if *self >= Zero::zero() {
                self.trunc() + one
            } else {
                self.trunc() - one
            }
        } else {
            self.trunc()
        }
    }

    /// Rounds towards zero.
    #[inline]
    pub fn trunc(&self) -> Ratio<T> {
        Ratio::from_integer(self.numer.clone() / self.denom.clone())
    }

    /// Returns the fractional part of a number, with division rounded towards zero.
    ///
    /// Satisfies `self == self.trunc() + self.fract()`.
    #[inline]
    pub fn fract(&self) -> Ratio<T> {
        Ratio::new_raw(self.numer.clone() % self.denom.clone(), self.denom.clone())
    }

    /// Raises the `Ratio` to the power of an exponent.
    #[inline]
    pub fn pow(&self, expon: i32) -> Ratio<T>
    where
        for<'a> &'a T: Pow<u32, Output = T>,
    {
        Pow::pow(self, expon)
    }
}

#[cfg(feature = "num-bigint")]
impl Ratio<BigInt> {
    /// Converts a float into a rational number.
    pub fn from_float<T: FloatCore>(f: T) -> Option<BigRational> {
        if !f.is_finite() {
            return None;
        }
        let (mantissa, exponent, sign) = f.integer_decode();
        let bigint_sign = if sign == 1 { Sign::Plus } else { Sign::Minus };
        if exponent < 0 {
            let one: BigInt = One::one();
            let denom: BigInt = one << ((-exponent) as usize);
            let numer: BigUint = FromPrimitive::from_u64(mantissa).unwrap();
            Some(Ratio::new(BigInt::from_biguint(bigint_sign, numer), denom))
        } else {
            let mut numer: BigUint = FromPrimitive::from_u64(mantissa).unwrap();
            numer <<= exponent as usize;
            Some(Ratio::from_integer(BigInt::from_biguint(
                bigint_sign,
                numer,
            )))
        }
    }
}

// From integer
impl<T> From<T> for Ratio<T>
where
    T: Clone + Integer,
{
    fn from(x: T) -> Ratio<T> {
        Ratio::from_integer(x)
    }
}

// From pair (through the `new` constructor)
impl<T> From<(T, T)> for Ratio<T>
where
    T: Clone + Integer,
{
    fn from(pair: (T, T)) -> Ratio<T> {
        Ratio::new(pair.0, pair.1)
    }
}

// Comparisons

// Mathematically, comparing a/b and c/d is the same as comparing a*d and b*c, but it's very easy
// for those multiplications to overflow fixed-size integers, so we need to take care.

impl<T: Clone + Integer> Ord for Ratio<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        // With equal denominators, the numerators can be directly compared
        if self.denom == other.denom {
            let ord = self.numer.cmp(&other.numer);
            return if self.denom < T::zero() {
                ord.reverse()
            } else {
                ord
            };
        }

        // With equal numerators, the denominators can be inversely compared
        if self.numer == other.numer {
            if self.numer.is_zero() {
                return cmp::Ordering::Equal;
            }
            let ord = self.denom.cmp(&other.denom);
            return if self.numer < T::zero() {
                ord
            } else {
                ord.reverse()
            };
        }

        // Unfortunately, we don't have CheckedMul to try.  That could sometimes avoid all the
        // division below, or even always avoid it for BigInt and BigUint.
        // FIXME- future breaking change to add Checked* to Integer?

        // Compare as floored integers and remainders
        let (self_int, self_rem) = self.numer.div_mod_floor(&self.denom);
        let (other_int, other_rem) = other.numer.div_mod_floor(&other.denom);
        match self_int.cmp(&other_int) {
            cmp::Ordering::Greater => cmp::Ordering::Greater,
            cmp::Ordering::Less => cmp::Ordering::Less,
            cmp::Ordering::Equal => {
                match (self_rem.is_zero(), other_rem.is_zero()) {
                    (true, true) => cmp::Ordering::Equal,
                    (true, false) => cmp::Ordering::Less,
                    (false, true) => cmp::Ordering::Greater,
                    (false, false) => {
                        // Compare the reciprocals of the remaining fractions in reverse
                        let self_recip = Ratio::new_raw(self.denom.clone(), self_rem);
                        let other_recip = Ratio::new_raw(other.denom.clone(), other_rem);
                        self_recip.cmp(&other_recip).reverse()
                    }
                }
            }
        }
    }
}

impl<T: Clone + Integer> PartialOrd for Ratio<T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Clone + Integer> PartialEq for Ratio<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == cmp::Ordering::Equal
    }
}

impl<T: Clone + Integer> Eq for Ratio<T> {}

// NB: We can't just `#[derive(Hash)]`, because it needs to agree
// with `Eq` even for non-reduced ratios.
impl<T: Clone + Integer + Hash> Hash for Ratio<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        recurse(&self.numer, &self.denom, state);

        fn recurse<T: Integer + Hash, H: Hasher>(numer: &T, denom: &T, state: &mut H) {
            if !denom.is_zero() {
                let (int, rem) = numer.div_mod_floor(denom);
                int.hash(state);
                recurse(denom, &rem, state);
            } else {
                denom.hash(state);
            }
        }
    }
}

mod iter_sum_product {
    use crate::Ratio;
    use core::iter::{Product, Sum};
    use num_integer::Integer;
    use num_traits::{One, Zero};

    impl<T: Integer + Clone> Sum for Ratio<T> {
        fn sum<I>(iter: I) -> Self
        where
            I: Iterator<Item = Ratio<T>>,
        {
            iter.fold(Self::zero(), |sum, num| sum + num)
        }
    }

    impl<'a, T: Integer + Clone> Sum<&'a Ratio<T>> for Ratio<T> {
        fn sum<I>(iter: I) -> Self
        where
            I: Iterator<Item = &'a Ratio<T>>,
        {
            iter.fold(Self::zero(), |sum, num| sum + num)
        }
    }

    impl<T: Integer + Clone> Product for Ratio<T> {
        fn product<I>(iter: I) -> Self
        where
            I: Iterator<Item = Ratio<T>>,
        {
            iter.fold(Self::one(), |prod, num| prod * num)
        }
    }

    impl<'a, T: Integer + Clone> Product<&'a Ratio<T>> for Ratio<T> {
        fn product<I>(iter: I) -> Self
        where
            I: Iterator<Item = &'a Ratio<T>>,
        {
            iter.fold(Self::one(), |prod, num| prod * num)
        }
    }
}

mod opassign {
    use core::ops::{AddAssign, DivAssign, MulAssign, RemAssign, SubAssign};

    use crate::Ratio;
    use num_integer::Integer;
    use num_traits::NumAssign;

    impl<T: Clone + Integer + NumAssign> AddAssign for Ratio<T> {
        fn add_assign(&mut self, other: Ratio<T>) {
            if self.denom == other.denom {
                self.numer += other.numer
            } else {
                let lcm = self.denom.lcm(&other.denom);
                let lhs_numer = self.numer.clone() * (lcm.clone() / self.denom.clone());
                let rhs_numer = other.numer * (lcm.clone() / other.denom);
                self.numer = lhs_numer + rhs_numer;
                self.denom = lcm;
            }
            self.reduce();
        }
    }

    // (a/b) / (c/d) = (a/gcd_ac)*(d/gcd_bd) / ((c/gcd_ac)*(b/gcd_bd))
    impl<T: Clone + Integer + NumAssign> DivAssign for Ratio<T> {
        fn div_assign(&mut self, other: Ratio<T>) {
            let gcd_ac = self.numer.gcd(&other.numer);
            let gcd_bd = self.denom.gcd(&other.denom);
            self.numer /= gcd_ac.clone();
            self.numer *= other.denom / gcd_bd.clone();
            self.denom /= gcd_bd;
            self.denom *= other.numer / gcd_ac;
            self.reduce(); // TODO: remove this line. see #8.
        }
    }

    // a/b * c/d = (a/gcd_ad)*(c/gcd_bc) / ((d/gcd_ad)*(b/gcd_bc))
    impl<T: Clone + Integer + NumAssign> MulAssign for Ratio<T> {
        fn mul_assign(&mut self, other: Ratio<T>) {
            let gcd_ad = self.numer.gcd(&other.denom);
            let gcd_bc = self.denom.gcd(&other.numer);
            self.numer /= gcd_ad.clone();
            self.numer *= other.numer / gcd_bc.clone();
            self.denom /= gcd_bc;
            self.denom *= other.denom / gcd_ad;
            self.reduce(); // TODO: remove this line. see #8.
        }
    }

    impl<T: Clone + Integer + NumAssign> RemAssign for Ratio<T> {
        fn rem_assign(&mut self, other: Ratio<T>) {
            if self.denom == other.denom {
                self.numer %= other.numer
            } else {
                let lcm = self.denom.lcm(&other.denom);
                let lhs_numer = self.numer.clone() * (lcm.clone() / self.denom.clone());
                let rhs_numer = other.numer * (lcm.clone() / other.denom);
                self.numer = lhs_numer % rhs_numer;
                self.denom = lcm;
            }
            self.reduce();
        }
    }

    impl<T: Clone + Integer + NumAssign> SubAssign for Ratio<T> {
        fn sub_assign(&mut self, other: Ratio<T>) {
            if self.denom == other.denom {
                self.numer -= other.numer
            } else {
                let lcm = self.denom.lcm(&other.denom);
                let lhs_numer = self.numer.clone() * (lcm.clone() / self.denom.clone());
                let rhs_numer = other.numer * (lcm.clone() / other.denom);
                self.numer = lhs_numer - rhs_numer;
                self.denom = lcm;
            }
            self.reduce();
        }
    }

    // a/b + c/1 = (a*1 + b*c) / (b*1) = (a + b*c) / b
    impl<T: Clone + Integer + NumAssign> AddAssign<T> for Ratio<T> {
        fn add_assign(&mut self, other: T) {
            self.numer += self.denom.clone() * other;
            self.reduce();
        }
    }

    impl<T: Clone + Integer + NumAssign> DivAssign<T> for Ratio<T> {
        fn div_assign(&mut self, other: T) {
            let gcd = self.numer.gcd(&other);
            self.numer /= gcd.clone();
            self.denom *= other / gcd;
            self.reduce(); // TODO: remove this line. see #8.
        }
    }

    impl<T: Clone + Integer + NumAssign> MulAssign<T> for Ratio<T> {
        fn mul_assign(&mut self, other: T) {
            let gcd = self.denom.gcd(&other);
            self.denom /= gcd.clone();
            self.numer *= other / gcd;
            self.reduce(); // TODO: remove this line. see #8.
        }
    }

    // a/b % c/1 = (a*1 % b*c) / (b*1) = (a % b*c) / b
    impl<T: Clone + Integer + NumAssign> RemAssign<T> for Ratio<T> {
        fn rem_assign(&mut self, other: T) {
            self.numer %= self.denom.clone() * other;
            self.reduce();
        }
    }

    // a/b - c/1 = (a*1 - b*c) / (b*1) = (a - b*c) / b
    impl<T: Clone + Integer + NumAssign> SubAssign<T> for Ratio<T> {
        fn sub_assign(&mut self, other: T) {
            self.numer -= self.denom.clone() * other;
            self.reduce();
        }
    }

    macro_rules! forward_op_assign {
        (impl $imp:ident, $method:ident) => {
            impl<'a, T: Clone + Integer + NumAssign> $imp<&'a Ratio<T>> for Ratio<T> {
                #[inline]
                fn $method(&mut self, other: &Ratio<T>) {
                    self.$method(other.clone())
                }
            }
            impl<'a, T: Clone + Integer + NumAssign> $imp<&'a T> for Ratio<T> {
                #[inline]
                fn $method(&mut self, other: &T) {
                    self.$method(other.clone())
                }
            }
        };
    }

    forward_op_assign!(impl AddAssign, add_assign);
    forward_op_assign!(impl DivAssign, div_assign);
    forward_op_assign!(impl MulAssign, mul_assign);
    forward_op_assign!(impl RemAssign, rem_assign);
    forward_op_assign!(impl SubAssign, sub_assign);
}

macro_rules! forward_ref_ref_binop {
    (impl $imp:ident, $method:ident) => {
        impl<'a, 'b, T: Clone + Integer> $imp<&'b Ratio<T>> for &'a Ratio<T> {
            type Output = Ratio<T>;

            #[inline]
            fn $method(self, other: &'b Ratio<T>) -> Ratio<T> {
                self.clone().$method(other.clone())
            }
        }
        impl<'a, 'b, T: Clone + Integer> $imp<&'b T> for &'a Ratio<T> {
            type Output = Ratio<T>;

            #[inline]
            fn $method(self, other: &'b T) -> Ratio<T> {
                self.clone().$method(other.clone())
            }
        }
    };
}

macro_rules! forward_ref_val_binop {
    (impl $imp:ident, $method:ident) => {
        impl<'a, T> $imp<Ratio<T>> for &'a Ratio<T>
        where
            T: Clone + Integer,
        {
            type Output = Ratio<T>;

            #[inline]
            fn $method(self, other: Ratio<T>) -> Ratio<T> {
                self.clone().$method(other)
            }
        }
        impl<'a, T> $imp<T> for &'a Ratio<T>
        where
            T: Clone + Integer,
        {
            type Output = Ratio<T>;

            #[inline]
            fn $method(self, other: T) -> Ratio<T> {
                self.clone().$method(other)
            }
        }
    };
}

macro_rules! forward_val_ref_binop {
    (impl $imp:ident, $method:ident) => {
        impl<'a, T> $imp<&'a Ratio<T>> for Ratio<T>
        where
            T: Clone + Integer,
        {
            type Output = Ratio<T>;

            #[inline]
            fn $method(self, other: &Ratio<T>) -> Ratio<T> {
                self.$method(other.clone())
            }
        }
        impl<'a, T> $imp<&'a T> for Ratio<T>
        where
            T: Clone + Integer,
        {
            type Output = Ratio<T>;

            #[inline]
            fn $method(self, other: &T) -> Ratio<T> {
                self.$method(other.clone())
            }
        }
    };
}

macro_rules! forward_all_binop {
    (impl $imp:ident, $method:ident) => {
        forward_ref_ref_binop!(impl $imp, $method);
        forward_ref_val_binop!(impl $imp, $method);
        forward_val_ref_binop!(impl $imp, $method);
    };
}

// Arithmetic
forward_all_binop!(impl Mul, mul);
// a/b * c/d = (a/gcd_ad)*(c/gcd_bc) / ((d/gcd_ad)*(b/gcd_bc))
impl<T> Mul<Ratio<T>> for Ratio<T>
where
    T: Clone + Integer,
{
    type Output = Ratio<T>;
    #[inline]
    fn mul(self, rhs: Ratio<T>) -> Ratio<T> {
        let gcd_ad = self.numer.gcd(&rhs.denom);
        let gcd_bc = self.denom.gcd(&rhs.numer);
        Ratio::new(
            self.numer / gcd_ad.clone() * (rhs.numer / gcd_bc.clone()),
            self.denom / gcd_bc * (rhs.denom / gcd_ad),
        )
    }
}
// a/b * c/1 = (a*c) / (b*1) = (a*c) / b
impl<T> Mul<T> for Ratio<T>
where
    T: Clone + Integer,
{
    type Output = Ratio<T>;
    #[inline]
    fn mul(self, rhs: T) -> Ratio<T> {
        let gcd = self.denom.gcd(&rhs);
        Ratio::new(self.numer * (rhs / gcd.clone()), self.denom / gcd)
    }
}

forward_all_binop!(impl Div, div);
// (a/b) / (c/d) = (a/gcd_ac)*(d/gcd_bd) / ((c/gcd_ac)*(b/gcd_bd))
impl<T> Div<Ratio<T>> for Ratio<T>
where
    T: Clone + Integer,
{
    type Output = Ratio<T>;

    #[inline]
    fn div(self, rhs: Ratio<T>) -> Ratio<T> {
        let gcd_ac = self.numer.gcd(&rhs.numer);
        let gcd_bd = self.denom.gcd(&rhs.denom);
        Ratio::new(
            self.numer / gcd_ac.clone() * (rhs.denom / gcd_bd.clone()),
            self.denom / gcd_bd * (rhs.numer / gcd_ac),
        )
    }
}
// (a/b) / (c/1) = (a*1) / (b*c) = a / (b*c)
impl<T> Div<T> for Ratio<T>
where
    T: Clone + Integer,
{
    type Output = Ratio<T>;

    #[inline]
    fn div(self, rhs: T) -> Ratio<T> {
        let gcd = self.numer.gcd(&rhs);
        Ratio::new(self.numer / gcd.clone(), self.denom * (rhs / gcd))
    }
}

macro_rules! arith_impl {
    (impl $imp:ident, $method:ident) => {
        forward_all_binop!(impl $imp, $method);
        // Abstracts a/b `op` c/d = (a*lcm/b `op` c*lcm/d)/lcm where lcm = lcm(b,d)
        impl<T: Clone + Integer> $imp<Ratio<T>> for Ratio<T> {
            type Output = Ratio<T>;
            #[inline]
            fn $method(self, rhs: Ratio<T>) -> Ratio<T> {
                if self.denom == rhs.denom {
                    return Ratio::new(self.numer.$method(rhs.numer), rhs.denom);
                }
                let lcm = self.denom.lcm(&rhs.denom);
                let lhs_numer = self.numer * (lcm.clone() / self.denom);
                let rhs_numer = rhs.numer * (lcm.clone() / rhs.denom);
                Ratio::new(lhs_numer.$method(rhs_numer), lcm)
            }
        }
        // Abstracts the a/b `op` c/1 = (a*1 `op` b*c) / (b*1) = (a `op` b*c) / b pattern
        impl<T: Clone + Integer> $imp<T> for Ratio<T> {
            type Output = Ratio<T>;
            #[inline]
            fn $method(self, rhs: T) -> Ratio<T> {
                Ratio::new(self.numer.$method(self.denom.clone() * rhs), self.denom)
            }
        }
    };
}

arith_impl!(impl Add, add);
arith_impl!(impl Sub, sub);
arith_impl!(impl Rem, rem);

// a/b * c/d = (a*c)/(b*d)
impl<T> CheckedMul for Ratio<T>
where
    T: Clone + Integer + CheckedMul,
{
    #[inline]
    fn checked_mul(&self, rhs: &Ratio<T>) -> Option<Ratio<T>> {
        let gcd_ad = self.numer.gcd(&rhs.denom);
        let gcd_bc = self.denom.gcd(&rhs.numer);
        Some(Ratio::new(
            (self.numer.clone() / gcd_ad.clone())
                .checked_mul(&(rhs.numer.clone() / gcd_bc.clone()))?,
            (self.denom.clone() / gcd_bc).checked_mul(&(rhs.denom.clone() / gcd_ad))?,
        ))
    }
}

// (a/b) / (c/d) = (a*d)/(b*c)
impl<T> CheckedDiv for Ratio<T>
where
    T: Clone + Integer + CheckedMul,
{
    #[inline]
    fn checked_div(&self, rhs: &Ratio<T>) -> Option<Ratio<T>> {
        if rhs.is_zero() {
            return None;
        }
        let (numer, denom) = if self.denom == rhs.denom {
            (self.numer.clone(), rhs.numer.clone())
        } else if self.numer == rhs.numer {
            (rhs.denom.clone(), self.denom.clone())
        } else {
            let gcd_ac = self.numer.gcd(&rhs.numer);
            let gcd_bd = self.denom.gcd(&rhs.denom);
            (
                (self.numer.clone() / gcd_ac.clone())
                    .checked_mul(&(rhs.denom.clone() / gcd_bd.clone()))?,
                (self.denom.clone() / gcd_bd).checked_mul(&(rhs.numer.clone() / gcd_ac))?,
            )
        };
        // Manual `reduce()`, avoiding sharp edges
        if denom.is_zero() {
            None
        } else if numer.is_zero() {
            Some(Self::zero())
        } else if numer == denom {
            Some(Self::one())
        } else {
            let g = numer.gcd(&denom);
            let numer = numer / g.clone();
            let denom = denom / g;
            let raw = if denom < T::zero() {
                // We need to keep denom positive, but 2's-complement MIN may
                // overflow negation -- instead we can check multiplying -1.
                let n1 = T::zero() - T::one();
                Ratio::new_raw(numer.checked_mul(&n1)?, denom.checked_mul(&n1)?)
            } else {
                Ratio::new_raw(numer, denom)
            };
            Some(raw)
        }
    }
}

// As arith_impl! but for Checked{Add,Sub} traits
macro_rules! checked_arith_impl {
    (impl $imp:ident, $method:ident) => {
        impl<T: Clone + Integer + CheckedMul + $imp> $imp for Ratio<T> {
            #[inline]
            fn $method(&self, rhs: &Ratio<T>) -> Option<Ratio<T>> {
                let gcd = self.denom.clone().gcd(&rhs.denom);
                let lcm = (self.denom.clone() / gcd.clone()).checked_mul(&rhs.denom)?;
                let lhs_numer = (lcm.clone() / self.denom.clone()).checked_mul(&self.numer)?;
                let rhs_numer = (lcm.clone() / rhs.denom.clone()).checked_mul(&rhs.numer)?;
                Some(Ratio::new(lhs_numer.$method(&rhs_numer)?, lcm))
            }
        }
    };
}

// a/b + c/d = (lcm/b*a + lcm/d*c)/lcm, where lcm = lcm(b,d)
checked_arith_impl!(impl CheckedAdd, checked_add);

// a/b - c/d = (lcm/b*a - lcm/d*c)/lcm, where lcm = lcm(b,d)
checked_arith_impl!(impl CheckedSub, checked_sub);

impl<T> Neg for Ratio<T>
where
    T: Clone + Integer + Neg<Output = T>,
{
    type Output = Ratio<T>;

    #[inline]
    fn neg(self) -> Ratio<T> {
        Ratio::new_raw(-self.numer, self.denom)
    }
}

impl<'a, T> Neg for &'a Ratio<T>
where
    T: Clone + Integer + Neg<Output = T>,
{
    type Output = Ratio<T>;

    #[inline]
    fn neg(self) -> Ratio<T> {
        -self.clone()
    }
}

impl<T> Inv for Ratio<T>
where
    T: Clone + Integer,
{
    type Output = Ratio<T>;

    #[inline]
    fn inv(self) -> Ratio<T> {
        self.recip()
    }
}

impl<'a, T> Inv for &'a Ratio<T>
where
    T: Clone + Integer,
{
    type Output = Ratio<T>;

    #[inline]
    fn inv(self) -> Ratio<T> {
        self.recip()
    }
}

// Constants
impl<T: Clone + Integer> Zero for Ratio<T> {
    #[inline]
    fn zero() -> Ratio<T> {
        Ratio::new_raw(Zero::zero(), One::one())
    }

    #[inline]
    fn is_zero(&self) -> bool {
        self.numer.is_zero()
    }

    #[inline]
    fn set_zero(&mut self) {
        self.numer.set_zero();
        self.denom.set_one();
    }
}

impl<T: Clone + Integer> One for Ratio<T> {
    #[inline]
    fn one() -> Ratio<T> {
        Ratio::new_raw(One::one(), One::one())
    }

    #[inline]
    fn is_one(&self) -> bool {
        self.numer == self.denom
    }

    #[inline]
    fn set_one(&mut self) {
        self.numer.set_one();
        self.denom.set_one();
    }
}

impl<T: Clone + Integer> Num for Ratio<T> {
    type FromStrRadixErr = ParseRatioError;

    /// Parses `numer/denom` where the numbers are in base `radix`.
    fn from_str_radix(s: &str, radix: u32) -> Result<Ratio<T>, ParseRatioError> {
        if s.splitn(2, '/').count() == 2 {
            let mut parts = s.splitn(2, '/').map(|ss| {
                T::from_str_radix(ss, radix).map_err(|_| ParseRatioError {
                    kind: RatioErrorKind::ParseError,
                })
            });
            let numer: T = parts.next().unwrap()?;
            let denom: T = parts.next().unwrap()?;
            if denom.is_zero() {
                Err(ParseRatioError {
                    kind: RatioErrorKind::ZeroDenominator,
                })
            } else {
                Ok(Ratio::new(numer, denom))
            }
        } else {
            Err(ParseRatioError {
                kind: RatioErrorKind::ParseError,
            })
        }
    }
}

impl<T: Clone + Integer + Signed> Signed for Ratio<T> {
    #[inline]
    fn abs(&self) -> Ratio<T> {
        if self.is_negative() {
            -self.clone()
        } else {
            self.clone()
        }
    }

    #[inline]
    fn abs_sub(&self, other: &Ratio<T>) -> Ratio<T> {
        if *self <= *other {
            Zero::zero()
        } else {
            self - other
        }
    }

    #[inline]
    fn signum(&self) -> Ratio<T> {
        if self.is_positive() {
            Self::one()
        } else if self.is_zero() {
            Self::zero()
        } else {
            -Self::one()
        }
    }

    #[inline]
    fn is_positive(&self) -> bool {
        (self.numer.is_positive() && self.denom.is_positive())
            || (self.numer.is_negative() && self.denom.is_negative())
    }

    #[inline]
    fn is_negative(&self) -> bool {
        (self.numer.is_negative() && self.denom.is_positive())
            || (self.numer.is_positive() && self.denom.is_negative())
    }
}

// String conversions
macro_rules! impl_formatting {
    ($fmt_trait:ident, $prefix:expr, $fmt_str:expr, $fmt_alt:expr) => {
        impl<T: $fmt_trait + Clone + Integer> $fmt_trait for Ratio<T> {
            #[cfg(feature = "std")]
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                let pre_pad = if self.denom.is_one() {
                    format!($fmt_str, self.numer)
                } else {
                    if f.alternate() {
                        format!(concat!($fmt_str, "/", $fmt_alt), self.numer, self.denom)
                    } else {
                        format!(concat!($fmt_str, "/", $fmt_str), self.numer, self.denom)
                    }
                };
                // TODO: replace with strip_prefix, when stabalized
                let (pre_pad, non_negative) = {
                    if pre_pad.starts_with("-") {
                        (&pre_pad[1..], false)
                    } else {
                        (&pre_pad[..], true)
                    }
                };
                f.pad_integral(non_negative, $prefix, pre_pad)
            }
            #[cfg(not(feature = "std"))]
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                let plus = if f.sign_plus() && self.numer >= T::zero() {
                    "+"
                } else {
                    ""
                };
                if self.denom.is_one() {
                    if f.alternate() {
                        write!(f, concat!("{}", $fmt_alt), plus, self.numer)
                    } else {
                        write!(f, concat!("{}", $fmt_str), plus, self.numer)
                    }
                } else {
                    if f.alternate() {
                        write!(
                            f,
                            concat!("{}", $fmt_alt, "/", $fmt_alt),
                            plus, self.numer, self.denom
                        )
                    } else {
                        write!(
                            f,
                            concat!("{}", $fmt_str, "/", $fmt_str),
                            plus, self.numer, self.denom
                        )
                    }
                }
            }
        }
    };
}

impl_formatting!(Display, "", "{}", "{:#}");
impl_formatting!(Octal, "0o", "{:o}", "{:#o}");
impl_formatting!(Binary, "0b", "{:b}", "{:#b}");
impl_formatting!(LowerHex, "0x", "{:x}", "{:#x}");
impl_formatting!(UpperHex, "0x", "{:X}", "{:#X}");
impl_formatting!(LowerExp, "", "{:e}", "{:#e}");
impl_formatting!(UpperExp, "", "{:E}", "{:#E}");

impl<T: FromStr + Clone + Integer> FromStr for Ratio<T> {
    type Err = ParseRatioError;

    /// Parses `numer/denom` or just `numer`.
    fn from_str(s: &str) -> Result<Ratio<T>, ParseRatioError> {
        let mut split = s.splitn(2, '/');

        let n = split.next().ok_or(ParseRatioError {
            kind: RatioErrorKind::ParseError,
        })?;
        let num = FromStr::from_str(n).map_err(|_| ParseRatioError {
            kind: RatioErrorKind::ParseError,
        })?;

        let d = split.next().unwrap_or("1");
        let den = FromStr::from_str(d).map_err(|_| ParseRatioError {
            kind: RatioErrorKind::ParseError,
        })?;

        if Zero::is_zero(&den) {
            Err(ParseRatioError {
                kind: RatioErrorKind::ZeroDenominator,
            })
        } else {
            Ok(Ratio::new(num, den))
        }
    }
}

impl<T> Into<(T, T)> for Ratio<T> {
    fn into(self) -> (T, T) {
        (self.numer, self.denom)
    }
}

#[cfg(feature = "serde")]
impl<T> serde::Serialize for Ratio<T>
where
    T: serde::Serialize + Clone + Integer + PartialOrd,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        (self.numer(), self.denom()).serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de, T> serde::Deserialize<'de> for Ratio<T>
where
    T: serde::Deserialize<'de> + Clone + Integer + PartialOrd,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        use serde::de::Unexpected;
        let (numer, denom): (T, T) = serde::Deserialize::deserialize(deserializer)?;
        if denom.is_zero() {
            Err(Error::invalid_value(
                Unexpected::Signed(0),
                &"a ratio with non-zero denominator",
            ))
        } else {
            Ok(Ratio::new_raw(numer, denom))
        }
    }
}

// FIXME: Bubble up specific errors
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ParseRatioError {
    kind: RatioErrorKind,
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum RatioErrorKind {
    ParseError,
    ZeroDenominator,
}

impl fmt::Display for ParseRatioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.description().fmt(f)
    }
}

#[cfg(feature = "std")]
impl Error for ParseRatioError {
    #[allow(deprecated)]
    fn description(&self) -> &str {
        self.kind.description()
    }
}

impl RatioErrorKind {
    fn description(&self) -> &'static str {
        match *self {
            RatioErrorKind::ParseError => "failed to parse integer",
            RatioErrorKind::ZeroDenominator => "zero value denominator",
        }
    }
}

#[cfg(feature = "num-bigint")]
impl FromPrimitive for Ratio<BigInt> {
    fn from_i64(n: i64) -> Option<Self> {
        Some(Ratio::from_integer(n.into()))
    }

    fn from_i128(n: i128) -> Option<Self> {
        Some(Ratio::from_integer(n.into()))
    }

    fn from_u64(n: u64) -> Option<Self> {
        Some(Ratio::from_integer(n.into()))
    }

    fn from_u128(n: u128) -> Option<Self> {
        Some(Ratio::from_integer(n.into()))
    }

    fn from_f32(n: f32) -> Option<Self> {
        Ratio::from_float(n)
    }

    fn from_f64(n: f64) -> Option<Self> {
        Ratio::from_float(n)
    }
}

macro_rules! from_primitive_integer {
    ($typ:ty, $approx:ident) => {
        impl FromPrimitive for Ratio<$typ> {
            fn from_i64(n: i64) -> Option<Self> {
                <$typ as FromPrimitive>::from_i64(n).map(Ratio::from_integer)
            }

            fn from_i128(n: i128) -> Option<Self> {
                <$typ as FromPrimitive>::from_i128(n).map(Ratio::from_integer)
            }

            fn from_u64(n: u64) -> Option<Self> {
                <$typ as FromPrimitive>::from_u64(n).map(Ratio::from_integer)
            }

            fn from_u128(n: u128) -> Option<Self> {
                <$typ as FromPrimitive>::from_u128(n).map(Ratio::from_integer)
            }

            fn from_f32(n: f32) -> Option<Self> {
                $approx(n, 10e-20, 30)
            }

            fn from_f64(n: f64) -> Option<Self> {
                $approx(n, 10e-20, 30)
            }
        }
    };
}

from_primitive_integer!(i8, approximate_float);
from_primitive_integer!(i16, approximate_float);
from_primitive_integer!(i32, approximate_float);
from_primitive_integer!(i64, approximate_float);
from_primitive_integer!(i128, approximate_float);
from_primitive_integer!(isize, approximate_float);

from_primitive_integer!(u8, approximate_float_unsigned);
from_primitive_integer!(u16, approximate_float_unsigned);
from_primitive_integer!(u32, approximate_float_unsigned);
from_primitive_integer!(u64, approximate_float_unsigned);
from_primitive_integer!(u128, approximate_float_unsigned);
from_primitive_integer!(usize, approximate_float_unsigned);

impl<T: Integer + Signed + Bounded + NumCast + Clone> Ratio<T> {
    pub fn approximate_float<F: FloatCore + NumCast>(f: F) -> Option<Ratio<T>> {
        // 1/10e-20 < 1/2**32 which seems like a good default, and 30 seems
        // to work well. Might want to choose something based on the types in the future, e.g.
        // T::max().recip() and T::bits() or something similar.
        let epsilon = <F as NumCast>::from(10e-20).expect("Can't convert 10e-20");
        approximate_float(f, epsilon, 30)
    }
}

fn approximate_float<T, F>(val: F, max_error: F, max_iterations: usize) -> Option<Ratio<T>>
where
    T: Integer + Signed + Bounded + NumCast + Clone,
    F: FloatCore + NumCast,
{
    let negative = val.is_sign_negative();
    let abs_val = val.abs();

    let r = approximate_float_unsigned(abs_val, max_error, max_iterations)?;

    // Make negative again if needed
    Some(if negative { r.neg() } else { r })
}

// No Unsigned constraint because this also works on positive integers and is called
// like that, see above
fn approximate_float_unsigned<T, F>(val: F, max_error: F, max_iterations: usize) -> Option<Ratio<T>>
where
    T: Integer + Bounded + NumCast + Clone,
    F: FloatCore + NumCast,
{
    // Continued fractions algorithm
    // http://mathforum.org/dr.math/faq/faq.fractions.html#decfrac

    if val < F::zero() || val.is_nan() {
        return None;
    }

    let mut q = val;
    let mut n0 = T::zero();
    let mut d0 = T::one();
    let mut n1 = T::one();
    let mut d1 = T::zero();

    let t_max = T::max_value();
    let t_max_f = <F as NumCast>::from(t_max.clone())?;

    // 1/epsilon > T::MAX
    let epsilon = t_max_f.recip();

    // Overflow
    if q > t_max_f {
        return None;
    }

    for _ in 0..max_iterations {
        let a = match <T as NumCast>::from(q) {
            None => break,
            Some(a) => a,
        };

        let a_f = match <F as NumCast>::from(a.clone()) {
            None => break,
            Some(a_f) => a_f,
        };
        let f = q - a_f;

        // Prevent overflow
        if !a.is_zero()
            && (n1 > t_max.clone() / a.clone()
                || d1 > t_max.clone() / a.clone()
                || a.clone() * n1.clone() > t_max.clone() - n0.clone()
                || a.clone() * d1.clone() > t_max.clone() - d0.clone())
        {
            break;
        }

        let n = a.clone() * n1.clone() + n0.clone();
        let d = a.clone() * d1.clone() + d0.clone();

        n0 = n1;
        d0 = d1;
        n1 = n.clone();
        d1 = d.clone();

        // Simplify fraction. Doing so here instead of at the end
        // allows us to get closer to the target value without overflows
        let g = Integer::gcd(&n1, &d1);
        if !g.is_zero() {
            n1 = n1 / g.clone();
            d1 = d1 / g.clone();
        }

        // Close enough?
        let (n_f, d_f) = match (<F as NumCast>::from(n), <F as NumCast>::from(d)) {
            (Some(n_f), Some(d_f)) => (n_f, d_f),
            _ => break,
        };
        if (n_f / d_f - val).abs() < max_error {
            break;
        }

        // Prevent division by ~0
        if f < epsilon {
            break;
        }
        q = f.recip();
    }

    // Overflow
    if d1.is_zero() {
        return None;
    }

    Some(Ratio::new(n1, d1))
}

#[cfg(not(feature = "num-bigint"))]
macro_rules! to_primitive_small {
    ($($type_name:ty)*) => ($(
        impl ToPrimitive for Ratio<$type_name> {
            fn to_i64(&self) -> Option<i64> {
                self.to_integer().to_i64()
            }

            fn to_i128(&self) -> Option<i128> {
                self.to_integer().to_i128()
            }

            fn to_u64(&self) -> Option<u64> {
                self.to_integer().to_u64()
            }

            fn to_u128(&self) -> Option<u128> {
                self.to_integer().to_u128()
            }

            fn to_f64(&self) -> Option<f64> {
                let float = self.numer.to_f64().unwrap() / self.denom.to_f64().unwrap();
                if float.is_nan() {
                    None
                } else {
                    Some(float)
                }
            }
        }
    )*)
}

#[cfg(not(feature = "num-bigint"))]
to_primitive_small!(u8 i8 u16 i16 u32 i32);

#[cfg(all(target_pointer_width = "32", not(feature = "num-bigint")))]
to_primitive_small!(usize isize);

#[cfg(not(feature = "num-bigint"))]
macro_rules! to_primitive_64 {
    ($($type_name:ty)*) => ($(
        impl ToPrimitive for Ratio<$type_name> {
            fn to_i64(&self) -> Option<i64> {
                self.to_integer().to_i64()
            }

            fn to_i128(&self) -> Option<i128> {
                self.to_integer().to_i128()
            }

            fn to_u64(&self) -> Option<u64> {
                self.to_integer().to_u64()
            }

            fn to_u128(&self) -> Option<u128> {
                self.to_integer().to_u128()
            }

            fn to_f64(&self) -> Option<f64> {
                let float = ratio_to_f64(
                    self.numer as i128,
                    self.denom as i128
                );
                if float.is_nan() {
                    None
                } else {
                    Some(float)
                }
            }
        }
    )*)
}

#[cfg(not(feature = "num-bigint"))]
to_primitive_64!(u64 i64);

#[cfg(all(target_pointer_width = "64", not(feature = "num-bigint")))]
to_primitive_64!(usize isize);

#[cfg(feature = "num-bigint")]
impl<T: Clone + Integer + ToPrimitive + ToBigInt> ToPrimitive for Ratio<T> {
    fn to_i64(&self) -> Option<i64> {
        self.to_integer().to_i64()
    }

    fn to_i128(&self) -> Option<i128> {
        self.to_integer().to_i128()
    }

    fn to_u64(&self) -> Option<u64> {
        self.to_integer().to_u64()
    }

    fn to_u128(&self) -> Option<u128> {
        self.to_integer().to_u128()
    }

    fn to_f64(&self) -> Option<f64> {
        let float = match (self.numer.to_i64(), self.denom.to_i64()) {
            (Some(numer), Some(denom)) => ratio_to_f64(
                <i128 as From<_>>::from(numer),
                <i128 as From<_>>::from(denom),
            ),
            _ => {
                let numer: BigInt = self.numer.to_bigint()?;
                let denom: BigInt = self.denom.to_bigint()?;
                ratio_to_f64(numer, denom)
            }
        };
        if float.is_nan() {
            None
        } else {
            Some(float)
        }
    }
}

trait Bits {
    fn bits(&self) -> u64;
}

#[cfg(feature = "num-bigint")]
impl Bits for BigInt {
    fn bits(&self) -> u64 {
        self.bits()
    }
}

impl Bits for i128 {
    fn bits(&self) -> u64 {
        (128 - self.wrapping_abs().leading_zeros()).into()
    }
}

/// Converts a ratio of `T` to an f64.
///
/// In addition to stated trait bounds, `T` must be able to hold numbers 56 bits larger than
/// the largest of `numer` and `denom`. This is automatically true if `T` is `BigInt`.
fn ratio_to_f64<T: Bits + Clone + Integer + Signed + ShlAssign<usize> + ToPrimitive>(
    numer: T,
    denom: T,
) -> f64 {
    assert_eq!(
        core::f64::RADIX,
        2,
        "only floating point implementations with radix 2 are supported"
    );

    // Inclusive upper and lower bounds to the range of exactly-representable ints in an f64.
    const MAX_EXACT_INT: i64 = 1i64 << core::f64::MANTISSA_DIGITS;
    const MIN_EXACT_INT: i64 = -MAX_EXACT_INT;

    let flo_sign = numer.signum().to_f64().unwrap() / denom.signum().to_f64().unwrap();
    if !flo_sign.is_normal() {
        return flo_sign;
    }

    // Fast track: both sides can losslessly be converted to f64s. In this case, letting the
    // FPU do the job is faster and easier. In any other case, converting to f64s may lead
    // to an inexact result: https://stackoverflow.com/questions/56641441/.
    if let (Some(n), Some(d)) = (numer.to_i64(), denom.to_i64()) {
        if MIN_EXACT_INT <= n && n <= MAX_EXACT_INT && MIN_EXACT_INT <= d && d <= MAX_EXACT_INT {
            return n.to_f64().unwrap() / d.to_f64().unwrap();
        }
    }

    // Otherwise, the goal is to obtain a quotient with at least 55 bits. 53 of these bits will
    // be used as the mantissa of the resulting float, and the remaining two are for rounding.
    // There's an error of up to 1 on the number of resulting bits, so we may get either 55 or
    // 56 bits.
    let mut numer = numer.abs();
    let mut denom = denom.abs();
    let (is_diff_positive, absolute_diff) = match numer.bits().checked_sub(denom.bits()) {
        Some(diff) => (true, diff),
        None => (false, denom.bits() - numer.bits()),
    };

    // Filter out overflows and underflows. After this step, the signed difference fits in an
    // isize.
    if is_diff_positive && absolute_diff > core::f64::MAX_EXP as u64 {
        return core::f64::INFINITY * flo_sign;
    }
    if !is_diff_positive
        && absolute_diff > -core::f64::MIN_EXP as u64 + core::f64::MANTISSA_DIGITS as u64 + 1
    {
        return 0.0 * flo_sign;
    }
    let diff = if is_diff_positive {
        absolute_diff.to_isize().unwrap()
    } else {
        -absolute_diff.to_isize().unwrap()
    };

    // Shift is chosen so that the quotient will have 55 or 56 bits. The exception is if the
    // quotient is going to be subnormal, in which case it may have fewer bits.
    let shift: isize =
        diff.max(core::f64::MIN_EXP as isize) - core::f64::MANTISSA_DIGITS as isize - 2;
    if shift >= 0 {
        denom <<= shift as usize
    } else {
        numer <<= -shift as usize
    };

    let (quotient, remainder) = numer.div_rem(&denom);

    // This is guaranteed to fit since we've set up quotient to be at most 56 bits.
    let mut quotient = quotient.to_u64().unwrap();
    let n_rounding_bits = {
        let quotient_bits = 64 - quotient.leading_zeros() as isize;
        let subnormal_bits = core::f64::MIN_EXP as isize - shift;
        quotient_bits.max(subnormal_bits) - core::f64::MANTISSA_DIGITS as isize
    } as usize;
    debug_assert!(n_rounding_bits == 2 || n_rounding_bits == 3);
    let rounding_bit_mask = (1u64 << n_rounding_bits) - 1;

    // Round to 53 bits with round-to-even. For rounding, we need to take into account both
    // our rounding bits and the division's remainder.
    let ls_bit = quotient & (1u64 << n_rounding_bits) != 0;
    let ms_rounding_bit = quotient & (1u64 << (n_rounding_bits - 1)) != 0;
    let ls_rounding_bits = quotient & (rounding_bit_mask >> 1) != 0;
    if ms_rounding_bit && (ls_bit || ls_rounding_bits || !remainder.is_zero()) {
        quotient += 1u64 << n_rounding_bits;
    }
    quotient &= !rounding_bit_mask;

    // The quotient is guaranteed to be exactly representable as it's now 53 bits + 2 or 3
    // trailing zeros, so there is no risk of a rounding error here.
    let q_float = quotient as f64;
    q_float * 2f64.powi(shift as i32) * flo_sign
}

#[cfg(test)]
#[cfg(feature = "std")]
fn hash<T: Hash>(x: &T) -> u64 {
    use std::collections::hash_map::RandomState;
    use std::hash::BuildHasher;
    let mut hasher = <RandomState as BuildHasher>::Hasher::new();
    x.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod test {
    #[cfg(all(feature = "num-bigint"))]
    use super::BigInt;
    #[cfg(feature = "num-bigint")]
    use super::BigRational;
    use super::{Ratio, Rational, Rational64};

    use core::f64;
    use core::i32;
    use core::isize;
    use core::str::FromStr;
    use num_integer::Integer;
    use num_traits::ToPrimitive;
    use num_traits::{FromPrimitive, One, Pow, Signed, Zero};

    pub const _0: Rational = Ratio { numer: 0, denom: 1 };
    pub const _1: Rational = Ratio { numer: 1, denom: 1 };
    pub const _2: Rational = Ratio { numer: 2, denom: 1 };
    pub const _NEG2: Rational = Ratio {
        numer: -2,
        denom: 1,
    };
    pub const _8: Rational = Ratio { numer: 8, denom: 1 };
    pub const _15: Rational = Ratio {
        numer: 15,
        denom: 1,
    };
    pub const _16: Rational = Ratio {
        numer: 16,
        denom: 1,
    };

    pub const _1_2: Rational = Ratio { numer: 1, denom: 2 };
    pub const _1_8: Rational = Ratio { numer: 1, denom: 8 };
    pub const _1_15: Rational = Ratio {
        numer: 1,
        denom: 15,
    };
    pub const _1_16: Rational = Ratio {
        numer: 1,
        denom: 16,
    };
    pub const _3_2: Rational = Ratio { numer: 3, denom: 2 };
    pub const _5_2: Rational = Ratio { numer: 5, denom: 2 };
    pub const _NEG1_2: Rational = Ratio {
        numer: -1,
        denom: 2,
    };
    pub const _1_NEG2: Rational = Ratio {
        numer: 1,
        denom: -2,
    };
    pub const _NEG1_NEG2: Rational = Ratio {
        numer: -1,
        denom: -2,
    };
    pub const _1_3: Rational = Ratio { numer: 1, denom: 3 };
    pub const _NEG1_3: Rational = Ratio {
        numer: -1,
        denom: 3,
    };
    pub const _2_3: Rational = Ratio { numer: 2, denom: 3 };
    pub const _NEG2_3: Rational = Ratio {
        numer: -2,
        denom: 3,
    };
    pub const _MIN: Rational = Ratio {
        numer: isize::MIN,
        denom: 1,
    };
    pub const _MIN_P1: Rational = Ratio {
        numer: isize::MIN + 1,
        denom: 1,
    };
    pub const _MAX: Rational = Ratio {
        numer: isize::MAX,
        denom: 1,
    };
    pub const _MAX_M1: Rational = Ratio {
        numer: isize::MAX - 1,
        denom: 1,
    };
    pub const _BILLION: Rational = Ratio {
        numer: 1_000_000_000,
        denom: 1,
    };

    #[cfg(feature = "num-bigint")]
    pub fn to_big(n: Rational) -> BigRational {
        Ratio::new(
            FromPrimitive::from_isize(n.numer).unwrap(),
            FromPrimitive::from_isize(n.denom).unwrap(),
        )
    }
    #[cfg(not(feature = "num-bigint"))]
    pub fn to_big(n: Rational) -> Rational {
        Ratio::new(
            FromPrimitive::from_isize(n.numer).unwrap(),
            FromPrimitive::from_isize(n.denom).unwrap(),
        )
    }

    #[test]
    fn test_test_constants() {
        // check our constants are what Ratio::new etc. would make.
        assert_eq!(_0, Zero::zero());
        assert_eq!(_1, One::one());
        assert_eq!(_2, Ratio::from_integer(2));
        assert_eq!(_1_2, Ratio::new(1, 2));
        assert_eq!(_3_2, Ratio::new(3, 2));
        assert_eq!(_NEG1_2, Ratio::new(-1, 2));
        assert_eq!(_2, From::from(2));
    }

    #[test]
    fn test_new_reduce() {
        assert_eq!(Ratio::new(2, 2), One::one());
        assert_eq!(Ratio::new(0, i32::MIN), Zero::zero());
        assert_eq!(Ratio::new(i32::MIN, i32::MIN), One::one());
    }
    #[test]
    #[should_panic]
    fn test_new_zero() {
        let _a = Ratio::new(1, 0);
    }

    #[test]
    fn test_approximate_float() {
        assert_eq!(Ratio::from_f32(0.5f32), Some(Ratio::new(1i64, 2)));
        assert_eq!(Ratio::from_f64(0.5f64), Some(Ratio::new(1i32, 2)));
        assert_eq!(Ratio::from_f32(5f32), Some(Ratio::new(5i64, 1)));
        assert_eq!(Ratio::from_f64(5f64), Some(Ratio::new(5i32, 1)));
        assert_eq!(Ratio::from_f32(29.97f32), Some(Ratio::new(2997i64, 100)));
        assert_eq!(Ratio::from_f32(-29.97f32), Some(Ratio::new(-2997i64, 100)));

        assert_eq!(Ratio::<i8>::from_f32(63.5f32), Some(Ratio::new(127i8, 2)));
        assert_eq!(Ratio::<i8>::from_f32(126.5f32), Some(Ratio::new(126i8, 1)));
        assert_eq!(Ratio::<i8>::from_f32(127.0f32), Some(Ratio::new(127i8, 1)));
        assert_eq!(Ratio::<i8>::from_f32(127.5f32), None);
        assert_eq!(Ratio::<i8>::from_f32(-63.5f32), Some(Ratio::new(-127i8, 2)));
        assert_eq!(
            Ratio::<i8>::from_f32(-126.5f32),
            Some(Ratio::new(-126i8, 1))
        );
        assert_eq!(
            Ratio::<i8>::from_f32(-127.0f32),
            Some(Ratio::new(-127i8, 1))
        );
        assert_eq!(Ratio::<i8>::from_f32(-127.5f32), None);

        assert_eq!(Ratio::<u8>::from_f32(-127f32), None);
        assert_eq!(Ratio::<u8>::from_f32(127f32), Some(Ratio::new(127u8, 1)));
        assert_eq!(Ratio::<u8>::from_f32(127.5f32), Some(Ratio::new(255u8, 2)));
        assert_eq!(Ratio::<u8>::from_f32(256f32), None);

        assert_eq!(Ratio::<i64>::from_f64(-10e200), None);
        assert_eq!(Ratio::<i64>::from_f64(10e200), None);
        assert_eq!(Ratio::<i64>::from_f64(f64::INFINITY), None);
        assert_eq!(Ratio::<i64>::from_f64(f64::NEG_INFINITY), None);
        assert_eq!(Ratio::<i64>::from_f64(f64::NAN), None);
        assert_eq!(
            Ratio::<i64>::from_f64(f64::EPSILON),
            Some(Ratio::new(1, 4503599627370496))
        );
        assert_eq!(Ratio::<i64>::from_f64(0.0), Some(Ratio::new(0, 1)));
        assert_eq!(Ratio::<i64>::from_f64(-0.0), Some(Ratio::new(0, 1)));
    }

    #[test]
    #[allow(clippy::eq_op)]
    fn test_cmp() {
        assert!(_0 == _0 && _1 == _1);
        assert!(_0 != _1 && _1 != _0);
        assert!(_0 < _1 && !(_1 < _0));
        assert!(_1 > _0 && !(_0 > _1));

        assert!(_0 <= _0 && _1 <= _1);
        assert!(_0 <= _1 && !(_1 <= _0));

        assert!(_0 >= _0 && _1 >= _1);
        assert!(_1 >= _0 && !(_0 >= _1));

        let _0_2: Rational = Ratio::new_raw(0, 2);
        assert_eq!(_0, _0_2);
    }

    #[test]
    fn test_cmp_overflow() {
        use core::cmp::Ordering;

        // issue #7 example:
        let big = Ratio::new(128u8, 1);
        let small = big.recip();
        assert!(big > small);

        // try a few that are closer together
        // (some matching numer, some matching denom, some neither)
        let ratios = [
            Ratio::new(125_i8, 127_i8),
            Ratio::new(63_i8, 64_i8),
            Ratio::new(124_i8, 125_i8),
            Ratio::new(125_i8, 126_i8),
            Ratio::new(126_i8, 127_i8),
            Ratio::new(127_i8, 126_i8),
        ];

        fn check_cmp(a: Ratio<i8>, b: Ratio<i8>, ord: Ordering) {
            #[cfg(feature = "std")]
            println!("comparing {} and {}", a, b);
            assert_eq!(a.cmp(&b), ord);
            assert_eq!(b.cmp(&a), ord.reverse());
        }

        for (i, &a) in ratios.iter().enumerate() {
            check_cmp(a, a, Ordering::Equal);
            check_cmp(-a, a, Ordering::Less);
            for &b in &ratios[i + 1..] {
                check_cmp(a, b, Ordering::Less);
                check_cmp(-a, -b, Ordering::Greater);
                check_cmp(a.recip(), b.recip(), Ordering::Greater);
                check_cmp(-a.recip(), -b.recip(), Ordering::Less);
            }
        }
    }

    #[test]
    fn test_to_integer() {
        assert_eq!(_0.to_integer(), 0);
        assert_eq!(_1.to_integer(), 1);
        assert_eq!(_2.to_integer(), 2);
        assert_eq!(_1_2.to_integer(), 0);
        assert_eq!(_3_2.to_integer(), 1);
        assert_eq!(_NEG1_2.to_integer(), 0);
    }

    #[test]
    fn test_numer() {
        assert_eq!(_0.numer(), &0);
        assert_eq!(_1.numer(), &1);
        assert_eq!(_2.numer(), &2);
        assert_eq!(_1_2.numer(), &1);
        assert_eq!(_3_2.numer(), &3);
        assert_eq!(_NEG1_2.numer(), &(-1));
    }
    #[test]
    fn test_denom() {
        assert_eq!(_0.denom(), &1);
        assert_eq!(_1.denom(), &1);
        assert_eq!(_2.denom(), &1);
        assert_eq!(_1_2.denom(), &2);
        assert_eq!(_3_2.denom(), &2);
        assert_eq!(_NEG1_2.denom(), &2);
    }

    #[test]
    fn test_is_integer() {
        assert!(_0.is_integer());
        assert!(_1.is_integer());
        assert!(_2.is_integer());
        assert!(!_1_2.is_integer());
        assert!(!_3_2.is_integer());
        assert!(!_NEG1_2.is_integer());
    }

    #[cfg(not(feature = "std"))]
    use core::fmt::{self, Write};
    #[cfg(not(feature = "std"))]
    #[derive(Debug)]
    struct NoStdTester {
        cursor: usize,
        buf: [u8; NoStdTester::BUF_SIZE],
    }

    #[cfg(not(feature = "std"))]
    impl NoStdTester {
        fn new() -> NoStdTester {
            NoStdTester {
                buf: [0; Self::BUF_SIZE],
                cursor: 0,
            }
        }

        fn clear(&mut self) {
            self.buf = [0; Self::BUF_SIZE];
            self.cursor = 0;
        }

        const WRITE_ERR: &'static str = "Formatted output too long";
        const BUF_SIZE: usize = 32;
    }

    #[cfg(not(feature = "std"))]
    impl Write for NoStdTester {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            for byte in s.bytes() {
                self.buf[self.cursor] = byte;
                self.cursor += 1;
                if self.cursor >= self.buf.len() {
                    return Err(fmt::Error {});
                }
            }
            Ok(())
        }
    }

    #[cfg(not(feature = "std"))]
    impl PartialEq<str> for NoStdTester {
        fn eq(&self, other: &str) -> bool {
            let other = other.as_bytes();
            for index in 0..self.cursor {
                if self.buf.get(index) != other.get(index) {
                    return false;
                }
            }
            true
        }
    }

    macro_rules! assert_fmt_eq {
        ($fmt_args:expr, $string:expr) => {
            #[cfg(not(feature = "std"))]
            {
                let mut tester = NoStdTester::new();
                write!(tester, "{}", $fmt_args).expect(NoStdTester::WRITE_ERR);
                assert_eq!(tester, *$string);
                tester.clear();
            }
            #[cfg(feature = "std")]
            {
                assert_eq!(std::fmt::format($fmt_args), $string);
            }
        };
    }

    #[test]
    fn test_show() {
        // Test:
        // :b :o :x, :X, :?
        // alternate or not (#)
        // positive and negative
        // padding
        // does not test precision (i.e. truncation)
        assert_fmt_eq!(format_args!("{}", _2), "2");
        assert_fmt_eq!(format_args!("{:+}", _2), "+2");
        assert_fmt_eq!(format_args!("{:-}", _2), "2");
        assert_fmt_eq!(format_args!("{}", _1_2), "1/2");
        assert_fmt_eq!(format_args!("{}", -_1_2), "-1/2"); // test negatives
        assert_fmt_eq!(format_args!("{}", _0), "0");
        assert_fmt_eq!(format_args!("{}", -_2), "-2");
        assert_fmt_eq!(format_args!("{:+}", -_2), "-2");
        assert_fmt_eq!(format_args!("{:b}", _2), "10");
        assert_fmt_eq!(format_args!("{:#b}", _2), "0b10");
        assert_fmt_eq!(format_args!("{:b}", _1_2), "1/10");
        assert_fmt_eq!(format_args!("{:+b}", _1_2), "+1/10");
        assert_fmt_eq!(format_args!("{:-b}", _1_2), "1/10");
        assert_fmt_eq!(format_args!("{:b}", _0), "0");
        assert_fmt_eq!(format_args!("{:#b}", _1_2), "0b1/0b10");
        // no std does not support padding
        #[cfg(feature = "std")]
        assert_eq!(&format!("{:010b}", _1_2), "0000001/10");
        #[cfg(feature = "std")]
        assert_eq!(&format!("{:#010b}", _1_2), "0b001/0b10");
        let half_i8: Ratio<i8> = Ratio::new(1_i8, 2_i8);
        assert_fmt_eq!(format_args!("{:b}", -half_i8), "11111111/10");
        assert_fmt_eq!(format_args!("{:#b}", -half_i8), "0b11111111/0b10");
        #[cfg(feature = "std")]
        assert_eq!(&format!("{:05}", Ratio::new(-1_i8, 1_i8)), "-0001");

        assert_fmt_eq!(format_args!("{:o}", _8), "10");
        assert_fmt_eq!(format_args!("{:o}", _1_8), "1/10");
        assert_fmt_eq!(format_args!("{:o}", _0), "0");
        assert_fmt_eq!(format_args!("{:#o}", _1_8), "0o1/0o10");
        #[cfg(feature = "std")]
        assert_eq!(&format!("{:010o}", _1_8), "0000001/10");
        #[cfg(feature = "std")]
        assert_eq!(&format!("{:#010o}", _1_8), "0o001/0o10");
        assert_fmt_eq!(format_args!("{:o}", -half_i8), "377/2");
        assert_fmt_eq!(format_args!("{:#o}", -half_i8), "0o377/0o2");

        assert_fmt_eq!(format_args!("{:x}", _16), "10");
        assert_fmt_eq!(format_args!("{:x}", _15), "f");
        assert_fmt_eq!(format_args!("{:x}", _1_16), "1/10");
        assert_fmt_eq!(format_args!("{:x}", _1_15), "1/f");
        assert_fmt_eq!(format_args!("{:x}", _0), "0");
        assert_fmt_eq!(format_args!("{:#x}", _1_16), "0x1/0x10");
        #[cfg(feature = "std")]
        assert_eq!(&format!("{:010x}", _1_16), "0000001/10");
        #[cfg(feature = "std")]
        assert_eq!(&format!("{:#010x}", _1_16), "0x001/0x10");
        assert_fmt_eq!(format_args!("{:x}", -half_i8), "ff/2");
        assert_fmt_eq!(format_args!("{:#x}", -half_i8), "0xff/0x2");

        assert_fmt_eq!(format_args!("{:X}", _16), "10");
        assert_fmt_eq!(format_args!("{:X}", _15), "F");
        assert_fmt_eq!(format_args!("{:X}", _1_16), "1/10");
        assert_fmt_eq!(format_args!("{:X}", _1_15), "1/F");
        assert_fmt_eq!(format_args!("{:X}", _0), "0");
        assert_fmt_eq!(format_args!("{:#X}", _1_16), "0x1/0x10");
        #[cfg(feature = "std")]
        assert_eq!(format!("{:010X}", _1_16), "0000001/10");
        #[cfg(feature = "std")]
        assert_eq!(format!("{:#010X}", _1_16), "0x001/0x10");
        assert_fmt_eq!(format_args!("{:X}", -half_i8), "FF/2");
        assert_fmt_eq!(format_args!("{:#X}", -half_i8), "0xFF/0x2");

        #[cfg(has_int_exp_fmt)]
        {
            assert_fmt_eq!(format_args!("{:e}", -_2), "-2e0");
            assert_fmt_eq!(format_args!("{:#e}", -_2), "-2e0");
            assert_fmt_eq!(format_args!("{:+e}", -_2), "-2e0");
            assert_fmt_eq!(format_args!("{:e}", _BILLION), "1e9");
            assert_fmt_eq!(format_args!("{:+e}", _BILLION), "+1e9");
            assert_fmt_eq!(format_args!("{:e}", _BILLION.recip()), "1e0/1e9");
            assert_fmt_eq!(format_args!("{:+e}", _BILLION.recip()), "+1e0/1e9");

            assert_fmt_eq!(format_args!("{:E}", -_2), "-2E0");
            assert_fmt_eq!(format_args!("{:#E}", -_2), "-2E0");
            assert_fmt_eq!(format_args!("{:+E}", -_2), "-2E0");
            assert_fmt_eq!(format_args!("{:E}", _BILLION), "1E9");
            assert_fmt_eq!(format_args!("{:+E}", _BILLION), "+1E9");
            assert_fmt_eq!(format_args!("{:E}", _BILLION.recip()), "1E0/1E9");
            assert_fmt_eq!(format_args!("{:+E}", _BILLION.recip()), "+1E0/1E9");
        }
    }

    mod arith {
        use super::super::{Ratio, Rational};
        use super::{to_big, _0, _1, _1_2, _2, _3_2, _5_2, _MAX, _MAX_M1, _MIN, _MIN_P1, _NEG1_2};
        use core::fmt::Debug;
        use num_integer::Integer;
        use num_traits::{Bounded, CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, NumAssign};

        #[test]
        fn test_add() {
            fn test(a: Rational, b: Rational, c: Rational) {
                assert_eq!(a + b, c);
                assert_eq!(
                    {
                        let mut x = a;
                        x += b;
                        x
                    },
                    c
                );
                assert_eq!(to_big(a) + to_big(b), to_big(c));
                assert_eq!(a.checked_add(&b), Some(c));
                assert_eq!(to_big(a).checked_add(&to_big(b)), Some(to_big(c)));
            }
            fn test_assign(a: Rational, b: isize, c: Rational) {
                assert_eq!(a + b, c);
                assert_eq!(
                    {
                        let mut x = a;
                        x += b;
                        x
                    },
                    c
                );
            }

            test(_1, _1_2, _3_2);
            test(_1, _1, _2);
            test(_1_2, _3_2, _2);
            test(_1_2, _NEG1_2, _0);
            test_assign(_1_2, 1, _3_2);
        }

        #[test]
        fn test_add_overflow() {
            // compares Ratio(1, T::max_value()) + Ratio(1, T::max_value())
            // to Ratio(1+1, T::max_value()) for each integer type.
            // Previously, this calculation would overflow.
            fn test_add_typed_overflow<T>()
            where
                T: Integer + Bounded + Clone + Debug + NumAssign,
            {
                let _1_max = Ratio::new(T::one(), T::max_value());
                let _2_max = Ratio::new(T::one() + T::one(), T::max_value());
                assert_eq!(_1_max.clone() + _1_max.clone(), _2_max);
                assert_eq!(
                    {
                        let mut tmp = _1_max.clone();
                        tmp += _1_max;
                        tmp
                    },
                    _2_max
                );
            }
            test_add_typed_overflow::<u8>();
            test_add_typed_overflow::<u16>();
            test_add_typed_overflow::<u32>();
            test_add_typed_overflow::<u64>();
            test_add_typed_overflow::<usize>();
            test_add_typed_overflow::<u128>();

            test_add_typed_overflow::<i8>();
            test_add_typed_overflow::<i16>();
            test_add_typed_overflow::<i32>();
            test_add_typed_overflow::<i64>();
            test_add_typed_overflow::<isize>();
            test_add_typed_overflow::<i128>();
        }

        #[test]
        fn test_sub() {
            fn test(a: Rational, b: Rational, c: Rational) {
                assert_eq!(a - b, c);
                assert_eq!(
                    {
                        let mut x = a;
                        x -= b;
                        x
                    },
                    c
                );
                assert_eq!(to_big(a) - to_big(b), to_big(c));
                assert_eq!(a.checked_sub(&b), Some(c));
                assert_eq!(to_big(a).checked_sub(&to_big(b)), Some(to_big(c)));
            }
            fn test_assign(a: Rational, b: isize, c: Rational) {
                assert_eq!(a - b, c);
                assert_eq!(
                    {
                        let mut x = a;
                        x -= b;
                        x
                    },
                    c
                );
            }

            test(_1, _1_2, _1_2);
            test(_3_2, _1_2, _1);
            test(_1, _NEG1_2, _3_2);
            test_assign(_1_2, 1, _NEG1_2);
        }

        #[test]
        fn test_sub_overflow() {
            // compares Ratio(1, T::max_value()) - Ratio(1, T::max_value()) to T::zero()
            // for each integer type. Previously, this calculation would overflow.
            fn test_sub_typed_overflow<T>()
            where
                T: Integer + Bounded + Clone + Debug + NumAssign,
            {
                let _1_max: Ratio<T> = Ratio::new(T::one(), T::max_value());
                assert!(T::is_zero(&(_1_max.clone() - _1_max.clone()).numer));
                {
                    let mut tmp: Ratio<T> = _1_max.clone();
                    tmp -= _1_max;
                    assert!(T::is_zero(&tmp.numer));
                }
            }
            test_sub_typed_overflow::<u8>();
            test_sub_typed_overflow::<u16>();
            test_sub_typed_overflow::<u32>();
            test_sub_typed_overflow::<u64>();
            test_sub_typed_overflow::<usize>();
            test_sub_typed_overflow::<u128>();

            test_sub_typed_overflow::<i8>();
            test_sub_typed_overflow::<i16>();
            test_sub_typed_overflow::<i32>();
            test_sub_typed_overflow::<i64>();
            test_sub_typed_overflow::<isize>();
            test_sub_typed_overflow::<i128>();
        }

        #[test]
        fn test_mul() {
            fn test(a: Rational, b: Rational, c: Rational) {
                assert_eq!(a * b, c);
                assert_eq!(
                    {
                        let mut x = a;
                        x *= b;
                        x
                    },
                    c
                );
                assert_eq!(to_big(a) * to_big(b), to_big(c));
                assert_eq!(a.checked_mul(&b), Some(c));
                assert_eq!(to_big(a).checked_mul(&to_big(b)), Some(to_big(c)));
            }
            fn test_assign(a: Rational, b: isize, c: Rational) {
                assert_eq!(a * b, c);
                assert_eq!(
                    {
                        let mut x = a;
                        x *= b;
                        x
                    },
                    c
                );
            }

            test(_1, _1_2, _1_2);
            test(_1_2, _3_2, Ratio::new(3, 4));
            test(_1_2, _NEG1_2, Ratio::new(-1, 4));
            test_assign(_1_2, 2, _1);
        }

        #[test]
        fn test_mul_overflow() {
            fn test_mul_typed_overflow<T>()
            where
                T: Integer + Bounded + Clone + Debug + NumAssign + CheckedMul,
            {
                let two = T::one() + T::one();
                let _3 = T::one() + T::one() + T::one();

                // 1/big * 2/3 = 1/(max/4*3), where big is max/2
                // make big = max/2, but also divisible by 2
                let big = T::max_value() / two.clone() / two.clone() * two.clone();
                let _1_big: Ratio<T> = Ratio::new(T::one(), big.clone());
                let _2_3: Ratio<T> = Ratio::new(two.clone(), _3.clone());
                assert_eq!(None, big.clone().checked_mul(&_3.clone()));
                let expected = Ratio::new(T::one(), big / two.clone() * _3.clone());
                assert_eq!(expected.clone(), _1_big.clone() * _2_3.clone());
                assert_eq!(
                    Some(expected.clone()),
                    _1_big.clone().checked_mul(&_2_3.clone())
                );
                assert_eq!(expected, {
                    let mut tmp = _1_big;
                    tmp *= _2_3;
                    tmp
                });

                // big/3 * 3 = big/1
                // make big = max/2, but make it indivisible by 3
                let big = T::max_value() / two / _3.clone() * _3.clone() + T::one();
                assert_eq!(None, big.clone().checked_mul(&_3.clone()));
                let big_3 = Ratio::new(big.clone(), _3.clone());
                let expected = Ratio::new(big, T::one());
                assert_eq!(expected, big_3.clone() * _3.clone());
                assert_eq!(expected, {
                    let mut tmp = big_3;
                    tmp *= _3;
                    tmp
                });
            }
            test_mul_typed_overflow::<u16>();
            test_mul_typed_overflow::<u8>();
            test_mul_typed_overflow::<u32>();
            test_mul_typed_overflow::<u64>();
            test_mul_typed_overflow::<usize>();
            test_mul_typed_overflow::<u128>();

            test_mul_typed_overflow::<i8>();
            test_mul_typed_overflow::<i16>();
            test_mul_typed_overflow::<i32>();
            test_mul_typed_overflow::<i64>();
            test_mul_typed_overflow::<isize>();
            test_mul_typed_overflow::<i128>();
        }

        #[test]
        fn test_div() {
            fn test(a: Rational, b: Rational, c: Rational) {
                assert_eq!(a / b, c);
                assert_eq!(
                    {
                        let mut x = a;
                        x /= b;
                        x
                    },
                    c
                );
                assert_eq!(to_big(a) / to_big(b), to_big(c));
                assert_eq!(a.checked_div(&b), Some(c));
                assert_eq!(to_big(a).checked_div(&to_big(b)), Some(to_big(c)));
            }
            fn test_assign(a: Rational, b: isize, c: Rational) {
                assert_eq!(a / b, c);
                assert_eq!(
                    {
                        let mut x = a;
                        x /= b;
                        x
                    },
                    c
                );
            }

            test(_1, _1_2, _2);
            test(_3_2, _1_2, _1 + _2);
            test(_1, _NEG1_2, _NEG1_2 + _NEG1_2 + _NEG1_2 + _NEG1_2);
            test_assign(_1, 2, _1_2);
        }

        #[test]
        fn test_div_overflow() {
            fn test_div_typed_overflow<T>()
            where
                T: Integer + Bounded + Clone + Debug + NumAssign + CheckedMul,
            {
                let two = T::one() + T::one();
                let _3 = T::one() + T::one() + T::one();

                // 1/big / 3/2 = 1/(max/4*3), where big is max/2
                // big ~ max/2, and big is divisible by 2
                let big = T::max_value() / two.clone() / two.clone() * two.clone();
                assert_eq!(None, big.clone().checked_mul(&_3.clone()));
                let _1_big: Ratio<T> = Ratio::new(T::one(), big.clone());
                let _3_two: Ratio<T> = Ratio::new(_3.clone(), two.clone());
                let expected = Ratio::new(T::one(), big / two.clone() * _3.clone());
                assert_eq!(expected.clone(), _1_big.clone() / _3_two.clone());
                assert_eq!(
                    Some(expected.clone()),
                    _1_big.clone().checked_div(&_3_two.clone())
                );
                assert_eq!(expected, {
                    let mut tmp = _1_big;
                    tmp /= _3_two;
                    tmp
                });

                // 3/big / 3 = 1/big where big is max/2
                // big ~ max/2, and big is not divisible by 3
                let big = T::max_value() / two / _3.clone() * _3.clone() + T::one();
                assert_eq!(None, big.clone().checked_mul(&_3.clone()));
                let _3_big = Ratio::new(_3.clone(), big.clone());
                let expected = Ratio::new(T::one(), big);
                assert_eq!(expected, _3_big.clone() / _3.clone());
                assert_eq!(expected, {
                    let mut tmp = _3_big;
                    tmp /= _3;
                    tmp
                });
            }
            test_div_typed_overflow::<u8>();
            test_div_typed_overflow::<u16>();
            test_div_typed_overflow::<u32>();
            test_div_typed_overflow::<u64>();
            test_div_typed_overflow::<usize>();
            test_div_typed_overflow::<u128>();

            test_div_typed_overflow::<i8>();
            test_div_typed_overflow::<i16>();
            test_div_typed_overflow::<i32>();
            test_div_typed_overflow::<i64>();
            test_div_typed_overflow::<isize>();
            test_div_typed_overflow::<i128>();
        }

        #[test]
        fn test_rem() {
            fn test(a: Rational, b: Rational, c: Rational) {
                assert_eq!(a % b, c);
                assert_eq!(
                    {
                        let mut x = a;
                        x %= b;
                        x
                    },
                    c
                );
                assert_eq!(to_big(a) % to_big(b), to_big(c))
            }
            fn test_assign(a: Rational, b: isize, c: Rational) {
                assert_eq!(a % b, c);
                assert_eq!(
                    {
                        let mut x = a;
                        x %= b;
                        x
                    },
                    c
                );
            }

            test(_3_2, _1, _1_2);
            test(_3_2, _1_2, _0);
            test(_5_2, _3_2, _1);
            test(_2, _NEG1_2, _0);
            test(_1_2, _2, _1_2);
            test_assign(_3_2, 1, _1_2);
        }

        #[test]
        fn test_rem_overflow() {
            // tests that Ratio(1,2) % Ratio(1, T::max_value()) equals 0
            // for each integer type. Previously, this calculation would overflow.
            fn test_rem_typed_overflow<T>()
            where
                T: Integer + Bounded + Clone + Debug + NumAssign,
            {
                let two = T::one() + T::one();
                // value near to maximum, but divisible by two
                let max_div2 = T::max_value() / two.clone() * two.clone();
                let _1_max: Ratio<T> = Ratio::new(T::one(), max_div2);
                let _1_two: Ratio<T> = Ratio::new(T::one(), two);
                assert!(T::is_zero(&(_1_two.clone() % _1_max.clone()).numer));
                {
                    let mut tmp: Ratio<T> = _1_two;
                    tmp %= _1_max;
                    assert!(T::is_zero(&tmp.numer));
                }
            }
            test_rem_typed_overflow::<u8>();
            test_rem_typed_overflow::<u16>();
            test_rem_typed_overflow::<u32>();
            test_rem_typed_overflow::<u64>();
            test_rem_typed_overflow::<usize>();
            test_rem_typed_overflow::<u128>();

            test_rem_typed_overflow::<i8>();
            test_rem_typed_overflow::<i16>();
            test_rem_typed_overflow::<i32>();
            test_rem_typed_overflow::<i64>();
            test_rem_typed_overflow::<isize>();
            test_rem_typed_overflow::<i128>();
        }

        #[test]
        fn test_neg() {
            fn test(a: Rational, b: Rational) {
                assert_eq!(-a, b);
                assert_eq!(-to_big(a), to_big(b))
            }

            test(_0, _0);
            test(_1_2, _NEG1_2);
            test(-_1, _1);
        }
        #[test]
        #[allow(clippy::eq_op)]
        fn test_zero() {
            assert_eq!(_0 + _0, _0);
            assert_eq!(_0 * _0, _0);
            assert_eq!(_0 * _1, _0);
            assert_eq!(_0 / _NEG1_2, _0);
            assert_eq!(_0 - _0, _0);
        }
        #[test]
        #[should_panic]
        fn test_div_0() {
            let _a = _1 / _0;
        }

        #[test]
        fn test_checked_failures() {
            let big = Ratio::new(128u8, 1);
            let small = Ratio::new(1, 128u8);
            assert_eq!(big.checked_add(&big), None);
            assert_eq!(small.checked_sub(&big), None);
            assert_eq!(big.checked_mul(&big), None);
            assert_eq!(small.checked_div(&big), None);
            assert_eq!(_1.checked_div(&_0), None);
        }

        #[test]
        fn test_checked_zeros() {
            assert_eq!(_0.checked_add(&_0), Some(_0));
            assert_eq!(_0.checked_sub(&_0), Some(_0));
            assert_eq!(_0.checked_mul(&_0), Some(_0));
            assert_eq!(_0.checked_div(&_0), None);
        }

        #[test]
        fn test_checked_min() {
            assert_eq!(_MIN.checked_add(&_MIN), None);
            assert_eq!(_MIN.checked_sub(&_MIN), Some(_0));
            assert_eq!(_MIN.checked_mul(&_MIN), None);
            assert_eq!(_MIN.checked_div(&_MIN), Some(_1));
            assert_eq!(_0.checked_add(&_MIN), Some(_MIN));
            assert_eq!(_0.checked_sub(&_MIN), None);
            assert_eq!(_0.checked_mul(&_MIN), Some(_0));
            assert_eq!(_0.checked_div(&_MIN), Some(_0));
            assert_eq!(_1.checked_add(&_MIN), Some(_MIN_P1));
            assert_eq!(_1.checked_sub(&_MIN), None);
            assert_eq!(_1.checked_mul(&_MIN), Some(_MIN));
            assert_eq!(_1.checked_div(&_MIN), None);
            assert_eq!(_MIN.checked_add(&_0), Some(_MIN));
            assert_eq!(_MIN.checked_sub(&_0), Some(_MIN));
            assert_eq!(_MIN.checked_mul(&_0), Some(_0));
            assert_eq!(_MIN.checked_div(&_0), None);
            assert_eq!(_MIN.checked_add(&_1), Some(_MIN_P1));
            assert_eq!(_MIN.checked_sub(&_1), None);
            assert_eq!(_MIN.checked_mul(&_1), Some(_MIN));
            assert_eq!(_MIN.checked_div(&_1), Some(_MIN));
        }

        #[test]
        fn test_checked_max() {
            assert_eq!(_MAX.checked_add(&_MAX), None);
            assert_eq!(_MAX.checked_sub(&_MAX), Some(_0));
            assert_eq!(_MAX.checked_mul(&_MAX), None);
            assert_eq!(_MAX.checked_div(&_MAX), Some(_1));
            assert_eq!(_0.checked_add(&_MAX), Some(_MAX));
            assert_eq!(_0.checked_sub(&_MAX), Some(_MIN_P1));
            assert_eq!(_0.checked_mul(&_MAX), Some(_0));
            assert_eq!(_0.checked_div(&_MAX), Some(_0));
            assert_eq!(_1.checked_add(&_MAX), None);
            assert_eq!(_1.checked_sub(&_MAX), Some(-_MAX_M1));
            assert_eq!(_1.checked_mul(&_MAX), Some(_MAX));
            assert_eq!(_1.checked_div(&_MAX), Some(_MAX.recip()));
            assert_eq!(_MAX.checked_add(&_0), Some(_MAX));
            assert_eq!(_MAX.checked_sub(&_0), Some(_MAX));
            assert_eq!(_MAX.checked_mul(&_0), Some(_0));
            assert_eq!(_MAX.checked_div(&_0), None);
            assert_eq!(_MAX.checked_add(&_1), None);
            assert_eq!(_MAX.checked_sub(&_1), Some(_MAX_M1));
            assert_eq!(_MAX.checked_mul(&_1), Some(_MAX));
            assert_eq!(_MAX.checked_div(&_1), Some(_MAX));
        }

        #[test]
        fn test_checked_min_max() {
            assert_eq!(_MIN.checked_add(&_MAX), Some(-_1));
            assert_eq!(_MIN.checked_sub(&_MAX), None);
            assert_eq!(_MIN.checked_mul(&_MAX), None);
            assert_eq!(
                _MIN.checked_div(&_MAX),
                Some(Ratio::new(_MIN.numer, _MAX.numer))
            );
            assert_eq!(_MAX.checked_add(&_MIN), Some(-_1));
            assert_eq!(_MAX.checked_sub(&_MIN), None);
            assert_eq!(_MAX.checked_mul(&_MIN), None);
            assert_eq!(_MAX.checked_div(&_MIN), None);
        }
    }

    #[test]
    fn test_round() {
        assert_eq!(_1_3.ceil(), _1);
        assert_eq!(_1_3.floor(), _0);
        assert_eq!(_1_3.round(), _0);
        assert_eq!(_1_3.trunc(), _0);

        assert_eq!(_NEG1_3.ceil(), _0);
        assert_eq!(_NEG1_3.floor(), -_1);
        assert_eq!(_NEG1_3.round(), _0);
        assert_eq!(_NEG1_3.trunc(), _0);

        assert_eq!(_2_3.ceil(), _1);
        assert_eq!(_2_3.floor(), _0);
        assert_eq!(_2_3.round(), _1);
        assert_eq!(_2_3.trunc(), _0);

        assert_eq!(_NEG2_3.ceil(), _0);
        assert_eq!(_NEG2_3.floor(), -_1);
        assert_eq!(_NEG2_3.round(), -_1);
        assert_eq!(_NEG2_3.trunc(), _0);

        assert_eq!(_1_2.ceil(), _1);
        assert_eq!(_1_2.floor(), _0);
        assert_eq!(_1_2.round(), _1);
        assert_eq!(_1_2.trunc(), _0);

        assert_eq!(_NEG1_2.ceil(), _0);
        assert_eq!(_NEG1_2.floor(), -_1);
        assert_eq!(_NEG1_2.round(), -_1);
        assert_eq!(_NEG1_2.trunc(), _0);

        assert_eq!(_1.ceil(), _1);
        assert_eq!(_1.floor(), _1);
        assert_eq!(_1.round(), _1);
        assert_eq!(_1.trunc(), _1);

        // Overflow checks

        let _neg1 = Ratio::from_integer(-1);
        let _large_rat1 = Ratio::new(i32::MAX, i32::MAX - 1);
        let _large_rat2 = Ratio::new(i32::MAX - 1, i32::MAX);
        let _large_rat3 = Ratio::new(i32::MIN + 2, i32::MIN + 1);
        let _large_rat4 = Ratio::new(i32::MIN + 1, i32::MIN + 2);
        let _large_rat5 = Ratio::new(i32::MIN + 2, i32::MAX);
        let _large_rat6 = Ratio::new(i32::MAX, i32::MIN + 2);
        let _large_rat7 = Ratio::new(1, i32::MIN + 1);
        let _large_rat8 = Ratio::new(1, i32::MAX);

        assert_eq!(_large_rat1.round(), One::one());
        assert_eq!(_large_rat2.round(), One::one());
        assert_eq!(_large_rat3.round(), One::one());
        assert_eq!(_large_rat4.round(), One::one());
        assert_eq!(_large_rat5.round(), _neg1);
        assert_eq!(_large_rat6.round(), _neg1);
        assert_eq!(_large_rat7.round(), Zero::zero());
        assert_eq!(_large_rat8.round(), Zero::zero());
    }

    #[test]
    fn test_fract() {
        assert_eq!(_1.fract(), _0);
        assert_eq!(_NEG1_2.fract(), _NEG1_2);
        assert_eq!(_1_2.fract(), _1_2);
        assert_eq!(_3_2.fract(), _1_2);
    }

    #[test]
    fn test_recip() {
        assert_eq!(_1 * _1.recip(), _1);
        assert_eq!(_2 * _2.recip(), _1);
        assert_eq!(_1_2 * _1_2.recip(), _1);
        assert_eq!(_3_2 * _3_2.recip(), _1);
        assert_eq!(_NEG1_2 * _NEG1_2.recip(), _1);

        assert_eq!(_3_2.recip(), _2_3);
        assert_eq!(_NEG1_2.recip(), _NEG2);
        assert_eq!(_NEG1_2.recip().denom(), &1);
    }

    #[test]
    #[should_panic(expected = "division by zero")]
    fn test_recip_fail() {
        let _a = Ratio::new(0, 1).recip();
    }

    #[test]
    fn test_pow() {
        fn test(r: Rational, e: i32, expected: Rational) {
            assert_eq!(r.pow(e), expected);
            assert_eq!(Pow::pow(r, e), expected);
            assert_eq!(Pow::pow(r, &e), expected);
            assert_eq!(Pow::pow(&r, e), expected);
            assert_eq!(Pow::pow(&r, &e), expected);
            #[cfg(feature = "num-bigint")]
            test_big(r, e, expected);
        }

        #[cfg(feature = "num-bigint")]
        fn test_big(r: Rational, e: i32, expected: Rational) {
            let r = BigRational::new_raw(r.numer.into(), r.denom.into());
            let expected = BigRational::new_raw(expected.numer.into(), expected.denom.into());
            assert_eq!((&r).pow(e), expected);
            assert_eq!(Pow::pow(r.clone(), e), expected);
            assert_eq!(Pow::pow(r.clone(), &e), expected);
            assert_eq!(Pow::pow(&r, e), expected);
            assert_eq!(Pow::pow(&r, &e), expected);
        }

        test(_1_2, 2, Ratio::new(1, 4));
        test(_1_2, -2, Ratio::new(4, 1));
        test(_1, 1, _1);
        test(_1, i32::MAX, _1);
        test(_1, i32::MIN, _1);
        test(_NEG1_2, 2, _1_2.pow(2i32));
        test(_NEG1_2, 3, -_1_2.pow(3i32));
        test(_3_2, 0, _1);
        test(_3_2, -1, _3_2.recip());
        test(_3_2, 3, Ratio::new(27, 8));
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_to_from_str() {
        use std::string::{String, ToString};
        fn test(r: Rational, s: String) {
            assert_eq!(FromStr::from_str(&s), Ok(r));
            assert_eq!(r.to_string(), s);
        }
        test(_1, "1".to_string());
        test(_0, "0".to_string());
        test(_1_2, "1/2".to_string());
        test(_3_2, "3/2".to_string());
        test(_2, "2".to_string());
        test(_NEG1_2, "-1/2".to_string());
    }
    #[test]
    fn test_from_str_fail() {
        fn test(s: &str) {
            let rational: Result<Rational, _> = FromStr::from_str(s);
            assert!(rational.is_err());
        }

        let xs = ["0 /1", "abc", "", "1/", "--1/2", "3/2/1", "1/0"];
        for &s in xs.iter() {
            test(s);
        }
    }

    #[cfg(feature = "num-bigint")]
    #[test]
    fn test_from_float() {
        use num_traits::float::FloatCore;
        fn test<T: FloatCore>(given: T, (numer, denom): (&str, &str)) {
            let ratio: BigRational = Ratio::from_float(given).unwrap();
            assert_eq!(
                ratio,
                Ratio::new(
                    FromStr::from_str(numer).unwrap(),
                    FromStr::from_str(denom).unwrap()
                )
            );
        }

        // f32
        test(core::f32::consts::PI, ("13176795", "4194304"));
        test(2f32.powf(100.), ("1267650600228229401496703205376", "1"));
        test(
            -(2f32.powf(100.)),
            ("-1267650600228229401496703205376", "1"),
        );
        test(
            1.0 / 2f32.powf(100.),
            ("1", "1267650600228229401496703205376"),
        );
        test(684729.48391f32, ("1369459", "2"));
        test(-8573.5918555f32, ("-4389679", "512"));

        // f64
        test(
            core::f64::consts::PI,
            ("884279719003555", "281474976710656"),
        );
        test(2f64.powf(100.), ("1267650600228229401496703205376", "1"));
        test(
            -(2f64.powf(100.)),
            ("-1267650600228229401496703205376", "1"),
        );
        test(684729.48391f64, ("367611342500051", "536870912"));
        test(-8573.5918555f64, ("-4713381968463931", "549755813888"));
        test(
            1.0 / 2f64.powf(100.),
            ("1", "1267650600228229401496703205376"),
        );
    }

    #[cfg(feature = "num-bigint")]
    #[test]
    fn test_from_float_fail() {
        use core::{f32, f64};

        assert_eq!(Ratio::from_float(f32::NAN), None);
        assert_eq!(Ratio::from_float(f32::INFINITY), None);
        assert_eq!(Ratio::from_float(f32::NEG_INFINITY), None);
        assert_eq!(Ratio::from_float(f64::NAN), None);
        assert_eq!(Ratio::from_float(f64::INFINITY), None);
        assert_eq!(Ratio::from_float(f64::NEG_INFINITY), None);
    }

    #[test]
    fn test_signed() {
        assert_eq!(_NEG1_2.abs(), _1_2);
        assert_eq!(_3_2.abs_sub(&_1_2), _1);
        assert_eq!(_1_2.abs_sub(&_3_2), Zero::zero());
        assert_eq!(_1_2.signum(), One::one());
        assert_eq!(_NEG1_2.signum(), -<Ratio<isize>>::one());
        assert_eq!(_0.signum(), Zero::zero());
        assert!(_NEG1_2.is_negative());
        assert!(_1_NEG2.is_negative());
        assert!(!_NEG1_2.is_positive());
        assert!(!_1_NEG2.is_positive());
        assert!(_1_2.is_positive());
        assert!(_NEG1_NEG2.is_positive());
        assert!(!_1_2.is_negative());
        assert!(!_NEG1_NEG2.is_negative());
        assert!(!_0.is_positive());
        assert!(!_0.is_negative());
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_hash() {
        assert!(crate::hash(&_0) != crate::hash(&_1));
        assert!(crate::hash(&_0) != crate::hash(&_3_2));

        // a == b -> hash(a) == hash(b)
        let a = Rational::new_raw(4, 2);
        let b = Rational::new_raw(6, 3);
        assert_eq!(a, b);
        assert_eq!(crate::hash(&a), crate::hash(&b));

        let a = Rational::new_raw(123456789, 1000);
        let b = Rational::new_raw(123456789 * 5, 5000);
        assert_eq!(a, b);
        assert_eq!(crate::hash(&a), crate::hash(&b));
    }

    #[test]
    fn test_into_pair() {
        assert_eq!((0, 1), _0.into());
        assert_eq!((-2, 1), _NEG2.into());
        assert_eq!((1, -2), _1_NEG2.into());
    }

    #[test]
    fn test_from_pair() {
        assert_eq!(_0, Ratio::from((0, 1)));
        assert_eq!(_1, Ratio::from((1, 1)));
        assert_eq!(_NEG2, Ratio::from((-2, 1)));
        assert_eq!(_1_NEG2, Ratio::from((1, -2)));
    }

    #[test]
    fn ratio_iter_sum() {
        // generic function to assure the iter method can be called
        // for any Iterator with Item = Ratio<impl Integer> or Ratio<&impl Integer>
        fn iter_sums<T: Integer + Clone>(slice: &[Ratio<T>]) -> [Ratio<T>; 3] {
            let mut manual_sum = Ratio::new(T::zero(), T::one());
            for ratio in slice {
                manual_sum = manual_sum + ratio;
            }
            [manual_sum, slice.iter().sum(), slice.iter().cloned().sum()]
        }
        // collect into array so test works on no_std
        let mut nums = [Ratio::new(0, 1); 1000];
        for (i, r) in (0..1000).map(|n| Ratio::new(n, 500)).enumerate() {
            nums[i] = r;
        }
        let sums = iter_sums(&nums[..]);
        assert_eq!(sums[0], sums[1]);
        assert_eq!(sums[0], sums[2]);
    }

    #[test]
    fn ratio_iter_product() {
        // generic function to assure the iter method can be called
        // for any Iterator with Item = Ratio<impl Integer> or Ratio<&impl Integer>
        fn iter_products<T: Integer + Clone>(slice: &[Ratio<T>]) -> [Ratio<T>; 3] {
            let mut manual_prod = Ratio::new(T::one(), T::one());
            for ratio in slice {
                manual_prod = manual_prod * ratio;
            }
            [
                manual_prod,
                slice.iter().product(),
                slice.iter().cloned().product(),
            ]
        }

        // collect into array so test works on no_std
        let mut nums = [Ratio::new(0, 1); 1000];
        for (i, r) in (0..1000).map(|n| Ratio::new(n, 500)).enumerate() {
            nums[i] = r;
        }
        let products = iter_products(&nums[..]);
        assert_eq!(products[0], products[1]);
        assert_eq!(products[0], products[2]);
    }

    #[test]
    fn test_num_zero() {
        let zero = Rational64::zero();
        assert!(zero.is_zero());

        let mut r = Rational64::new(123, 456);
        assert!(!r.is_zero());
        assert_eq!(r + zero, r);

        r.set_zero();
        assert!(r.is_zero());
    }

    #[test]
    fn test_num_one() {
        let one = Rational64::one();
        assert!(one.is_one());

        let mut r = Rational64::new(123, 456);
        assert!(!r.is_one());
        assert_eq!(r * one, r);

        r.set_one();
        assert!(r.is_one());
    }

    #[test]
    fn test_const() {
        const N: Ratio<i32> = Ratio::new_raw(123, 456);
        const N_NUMER: &i32 = N.numer();
        const N_DENOM: &i32 = N.denom();

        assert_eq!(N_NUMER, &123);
        assert_eq!(N_DENOM, &456);

        let r = N.reduced();
        assert_eq!(r.numer(), &(123 / 3));
        assert_eq!(r.denom(), &(456 / 3));
    }

    #[test]
    fn test_ratio_to_i64() {
        assert_eq!(5, Rational64::new(70, 14).to_u64().unwrap());
        assert_eq!(-3, Rational64::new(-31, 8).to_i64().unwrap());
        assert_eq!(None, Rational64::new(-31, 8).to_u64());
    }

    #[test]
    #[cfg(feature = "num-bigint")]
    fn test_ratio_to_i128() {
        assert_eq!(
            1i128 << 70,
            Ratio::<i128>::new(1i128 << 77, 1i128 << 7)
                .to_i128()
                .unwrap()
        );
    }

    #[test]
    #[cfg(feature = "num-bigint")]
    fn test_big_ratio_to_f64() {
        assert_eq!(
            BigRational::new(
                "1234567890987654321234567890987654321234567890"
                    .parse()
                    .unwrap(),
                "3".parse().unwrap()
            )
            .to_f64(),
            Some(411522630329218100000000000000000000000000000f64)
        );
        assert_eq!(
            BigRational::new(BigInt::one(), BigInt::one() << 1050).to_f64(),
            Some(0f64)
        );
        assert_eq!(
            BigRational::from(BigInt::one() << 1050).to_f64(),
            Some(core::f64::INFINITY)
        );
        assert_eq!(
            BigRational::from((-BigInt::one()) << 1050).to_f64(),
            Some(core::f64::NEG_INFINITY)
        );
        assert_eq!(
            BigRational::new(
                "1234567890987654321234567890".parse().unwrap(),
                "987654321234567890987654321".parse().unwrap()
            )
            .to_f64(),
            Some(1.2499999893125f64)
        );
        assert_eq!(
            BigRational::new_raw(BigInt::one(), BigInt::zero()).to_f64(),
            Some(core::f64::INFINITY)
        );
        assert_eq!(
            BigRational::new_raw(-BigInt::one(), BigInt::zero()).to_f64(),
            Some(core::f64::NEG_INFINITY)
        );
        assert_eq!(
            BigRational::new_raw(BigInt::zero(), BigInt::zero()).to_f64(),
            None
        );
    }

    #[test]
    fn test_ratio_to_f64() {
        assert_eq!(Ratio::<u8>::new(1, 2).to_f64(), Some(0.5f64));
        assert_eq!(Rational64::new(1, 2).to_f64(), Some(0.5f64));
        assert_eq!(Rational64::new(1, -2).to_f64(), Some(-0.5f64));
        assert_eq!(Rational64::new(0, 2).to_f64(), Some(0.0f64));
        assert_eq!(Rational64::new(0, -2).to_f64(), Some(-0.0f64));
        assert_eq!(Rational64::new((1 << 57) + 1, 1 << 54).to_f64(), Some(8f64));
        assert_eq!(
            Rational64::new((1 << 52) + 1, 1 << 52).to_f64(),
            Some(1.0000000000000002f64),
        );
        assert_eq!(
            Rational64::new((1 << 60) + (1 << 8), 1 << 60).to_f64(),
            Some(1.0000000000000002f64),
        );
        assert_eq!(
            Ratio::<i32>::new_raw(1, 0).to_f64(),
            Some(core::f64::INFINITY)
        );
        assert_eq!(
            Ratio::<i32>::new_raw(-1, 0).to_f64(),
            Some(core::f64::NEG_INFINITY)
        );
        assert_eq!(Ratio::<i32>::new_raw(0, 0).to_f64(), None);
    }
}
#[cfg(test)]
mod tests_rug_4 {
    use super::*;
    use num_traits::{Signed, ToPrimitive};
    use num_integer::Integer;
    use core::ops::ShlAssign;
    use core::marker::Sized;
    use core::clone::Clone;
    use core::cmp::{Ordering, PartialEq, PartialOrd};

    #[test]
    fn test_rug() {
        let mut p0: BigInt = num_bigint::BigInt::from(123456789);
        let mut p1: BigInt = num_bigint::BigInt::from(987654321);

        crate::ratio_to_f64(p0, p1);
    }
}#[cfg(test)]
mod tests_rug_5 {
    use super::*;
    use crate::Ratio;

    #[test]
    fn test_rug() {
        let mut p0: i32 = 10;
        let mut p1: i32 = 5;

        Ratio::<i32>::new_raw(p0, p1);
    }
}#[cfg(test)]
mod tests_rug_6 {
    use super::*;
    use crate::Ratio;

    #[test]
    fn test_rug() {
        let p0 = Ratio::<i32>::new_raw(5, 2);

        Ratio::<i32>::numer(&p0);
    }
}#[cfg(test)]
mod tests_rug_7 {
    use super::*;
    use crate::Ratio;

    #[test]
    fn test_rug() {
        let p0 = Ratio::<i32>::new(3, 4);

        <Ratio<i32>>::denom(&p0);
    }
}#[cfg(test)]
mod tests_rug_8 {
    use super::*;
    use crate::{Ratio, Integer};

    #[test]
    fn test_ratio_new() {
        let mut p0: i32 = 10;
        let mut p1: i32 = 5;
        
        Ratio::<i32>::new(p0, p1);
    }
}
#[cfg(test)]
mod tests_rug_9 {
    use super::*;
    use num_traits::{One, Num};
    use crate::Ratio;
    
    #[test]
    fn test_rug() {
        let mut p0: i32 = 5;

        <Ratio<i32>>::from_integer(p0);

    }
}
#[cfg(test)]
mod tests_rug_10 {
    use super::*;
    use crate::Ratio;

    #[test]
    fn test_to_integer() {
        let p0 = Ratio::from_integer(5);

        assert_eq!(Ratio::to_integer(&p0), 5);
    }
}#[cfg(test)]
mod tests_rug_11 {
    use super::*;
    use crate::Ratio;
    use num_traits::identities::One;

    #[test]
    fn test_rug() {
        let mut p0: Ratio<i32> = Ratio::new(5, 1);

        assert_eq!(p0.is_integer(), true);
    }
}#[cfg(test)]
mod tests_rug_12 {
    use super::*;
    use crate::Ratio;
    use num_integer::Integer;

    #[test]
    fn test_ratio_reduce() {
        let mut p0 = Ratio::new(10, 20);

        p0.reduce();

        assert_eq!(p0, Ratio::new(1, 2));
    }
}#[cfg(test)]
mod tests_rug_13 {
    use super::*;
    use crate::Ratio;
    
    #[test]
    fn test_reduced() {
        // Create a Ratio<T> instance
        let p0 = Ratio::new(6, 8);

        let reduced_ratio = p0.reduced();

        assert_eq!(reduced_ratio, Ratio::new(3, 4));
    }
}#[cfg(test)]
mod tests_rug_14 {
    use super::*;
    use crate::Ratio;

    #[test]
    fn test_rug() {
        let mut p0: Ratio<u32> = Ratio::new_raw(5, 10);

        p0.recip();
    }
}
#[cfg(test)]
mod tests_rug_15 {
    use super::*;
    use crate::Ratio;
    use num_integer::Integer;
    use std::cmp;

    #[test]
    fn test_rug() {
        let mut p0 = Ratio::new(5, 10);

        Ratio::<i32>::into_recip(p0);
    }
}
#[cfg(test)]
mod tests_rug_16 {
    use super::*;
    use crate::{Ratio, Zero, One};

    #[test]
    fn test_floor() {
        let p0 = Ratio::<i32>::new(7, 2);

        let result = p0.floor();
        
        // Add assertions here
    }

}
#[cfg(test)]
mod tests_rug_17 {
    use super::*;
    use num_traits::{Zero, One};

    #[test]
    fn test_ceil() {
        let mut p0 = Ratio::new(10, 3);

        p0.ceil();
    }
}
#[cfg(test)]
mod tests_rug_18 {
    use super::*;
    use crate::{Ratio, Zero, One};

    #[test]
    fn test_round() {
        let p0: Ratio<i32> = Ratio::new(5, 2);

        p0.round();
    }
}#[cfg(test)]
mod tests_rug_19 {
    use super::*;
    use crate::{Ratio, Integer, Zero};

    #[test]
    fn test_rug() {
        let mut p0 = Ratio::new(10, 3);

        Ratio::<i32>::trunc(&p0);
    }
}#[cfg(test)]
mod tests_rug_20 {
    use super::*;
    use crate::Ratio;

    #[test]
    fn test_rug() {
        let numer = 10;
        let denom = 3;
        let p0 = Ratio::new(numer, denom);

        let result = p0.fract();
        assert_eq!(result, Ratio::new(1, 3));
    }
}#[cfg(test)]
mod tests_rug_21 {
    use super::*;
    use crate::Ratio;

    #[test]
    fn test_pow() {
        let p0 = Ratio::new(3, 2); // Sample data for Ratio<T>
        let p1: i32 = 3; // Sample data for the exponent

        let result = p0.pow(p1);

        assert_eq!(result, Ratio::new(27, 8)); // Expected result for the given input
    }
}#[cfg(test)]
mod tests_rug_22 {
    use super::*;
    use num_traits::float::FloatCore;
    use num_bigint::{BigInt, Sign, ToBigInt};
    use num_traits::{One, FromPrimitive};
    use crate::{BigRational, Ratio};
    use num_bigint::BigUint;

    #[test]
    fn test_rug() {
        let mut p0: f64 = 3.14; // Sample value for testing

        <Ratio<num_bigint::BigInt>>::from_float(p0);
    }
}#[cfg(test)]
mod tests_rug_23 {
    use super::*;
    use crate::Ratio;
    use num_integer::Integer;
    use core::convert::From;

    #[test]
    fn test_rug() {
        let mut p0: i32 = 10;

        <Ratio<i32> as core::convert::From<i32>>::from(p0);
    }
}#[cfg(test)]
mod tests_rug_25 {
    use super::*;
    use num_traits::{One, Zero};
    
    #[test]
    fn test_ratio_cmp() {
        let p0 = Ratio::<i32>::new_raw(5, 3);
        let p1 = Ratio::<i32>::new_raw(7, 4);

        assert_eq!(p0.cmp(&p1), std::cmp::Ordering::Less);
    }
}
#[cfg(test)]
mod tests_rug_28 {
    use super::*;
    use core::hash::{Hash, Hasher};
    use num_integer::Integer;
    
    #[test]
    fn test_num_rational() {
        let numer = 10;
        let denom = 5;
        let ratio = Ratio::<i32> { numer, denom };
        
        struct TestHasher(u64);
        
        impl Hasher for TestHasher {
            fn finish(&self) -> u64 { self.0 }
            fn write(&mut self, _bytes: &[u8]) {}
        }
        
        let mut hasher = TestHasher(0);
        
        ratio.hash(&mut hasher);
        
        // Write your assertions here based on the expected output of the hash function
    }
}
#[cfg(test)]
mod tests_rug_29 {
    use super::*;
    use crate::{Ratio, iter_sum_product};
    use core::iter::Sum;

    #[test]
    fn test_rug() {
        let mut p0: std::vec::IntoIter<Ratio<i32>> = vec![Ratio::new(1, 2), Ratio::new(3, 4), Ratio::new(5, 6)].into_iter();

        <Ratio<i32>>::sum(p0);
    }
}#[cfg(test)]
mod tests_rug_32 {
    use super::*;
    use crate::Ratio;
    use core::iter::Product;
    
    #[test]
    fn test_ratio_product() {
        let nums = vec![
            Ratio::new_raw(1, 2),
            Ratio::new_raw(3, 4),
            Ratio::new_raw(5, 6),
        ];
        let p0: &mut dyn Iterator<Item = &Ratio<i32>> = &mut nums.iter();

        let result = Ratio::<i32>::product(p0);

        assert_eq!(result, Ratio::new(15, 16));
    }
}#[cfg(test)]
mod tests_rug_33 {
    use super::*;
    use crate::Ratio;
    use core::ops::AddAssign;
    
    #[test]
    fn test_add_assign() {
        let mut p0: Ratio<i32> = Ratio::new_raw(3, 4);
        let p1: Ratio<i32> = Ratio::new_raw(1, 4);

        p0.add_assign(p1);

        assert_eq!(p0, Ratio::new_raw(1, 1));
    }
}#[cfg(test)]
mod tests_rug_36 {
    use super::*;
    use crate::Ratio;
    use core::ops::RemAssign;
    
    #[test]
    fn test_rem_assign() {
        let mut p0: Ratio<i32> = Ratio::new(10, 3); // Sample data
        let p1: Ratio<i32> = Ratio::new(4, 3); // Sample data
        
        p0.rem_assign(p1);
        
        assert_eq!(p0, Ratio::new(2, 3)); // Sample assertion based on the calculation
    }
}#[cfg(test)]
mod tests_rug_37 {
    use super::*;
    use crate::Ratio;
    use core::ops::SubAssign;
    
    #[test]
    fn test_sub_assign() {
        let mut p0: Ratio<i32> = Ratio::new_raw(7, 8);
        let p1: Ratio<i32> = Ratio::new_raw(3, 4);
        
        p0.sub_assign(p1);
        
        assert_eq!(p0, Ratio::new_raw(1, 8));
    }
}#[cfg(test)]
mod tests_rug_53 {
    use super::*;
    use crate::Ratio;
    use core::ops::Mul;

    #[test]
    fn test_rug() {
        let mut p0: Ratio<i32> = Ratio::new_raw(5, 3);
        let mut p1: Ratio<i32> = Ratio::new_raw(2, 7);
        
        p0.mul(&p1);
    }
}#[cfg(test)]
mod tests_rug_54 {
    use super::*;
    use crate::Ratio;
    use num_integer::Integer;
    use core::clone::Clone;
    use core::marker::Sized;
    
    #[test]
    fn test_rug() {
        let mut p0: Ratio<i32> = Ratio::new_raw(10, 5);
        let mut p1: i32 = 5;
        
        p0.mul(&p1);
    }
}#[cfg(test)]
mod tests_rug_55 {
    use super::*;
    use crate::Ratio;
    use core::ops::Mul;
    
    #[test]
    fn test_rug() {
        let mut p0: Ratio<u32> = Ratio::new(10, 2);
        let mut p1: Ratio<u32> = Ratio::new(5, 3);
                
        p0.mul(p1);
    }
}#[cfg(test)]
mod tests_rug_57 {
    use super::*;
    use crate::{Ratio, Mul};

    #[test]
    fn test_rug() {
        let mut p0: Ratio<i32> = Ratio::new(3, 4);
        let p1: Ratio<i32> = Ratio::new(2, 5);
        
        p0.mul(&p1);
    }
}#[cfg(test)]
mod tests_rug_58 {
    use super::*;
    use crate::Ratio;
    use num_integer::Integer;
    use core::marker::Sized;
    use core::ops::Mul;

    #[test]
    fn test_rug() {
        let numer = 10_u32;
        let p0 = Ratio::new_raw(numer, 1);

        let other = 5_u32;
        p0.mul(&other);
    }
}#[cfg(test)]
mod tests_rug_59 {
    use super::*;
    use crate::Ratio;
    use num_integer::Integer;
    
    #[test]
    fn test_rug() {
        let p0 = Ratio::new(8, 12);
        let p1 = Ratio::new(5, 7);
                
        p0.mul(p1);
    }
}
#[cfg(test)]
mod tests_rug_60 {
    use super::*;
    use crate::{Ratio, Integer};
    use core::{ops::Mul, clone::Clone};
    
    #[test]
    fn test_rug() {
        let mut p0: Ratio<i32> = Ratio::new(10, 2);
        let mut p1: i32 = 5;

        <Ratio<i32> as Mul<i32>>::mul(p0, p1);
    }
}
#[cfg(test)]
mod tests_rug_65 {
    use super::*;
    use crate::Ratio;
    use core::ops::Div;

    #[test]
    fn test_rug() {
        let mut p0: Ratio<i32> = Ratio::new(10, 2);
        let mut p1: Ratio<i32> = Ratio::new(5, 3);

        p0.div(&p1);
    }
}#[cfg(test)]
mod tests_rug_66 {
    use super::*;
    use core::ops::Div;
    use crate::Ratio;
    use num_integer::Integer;

    #[test]
    fn test_rug() {
        let mut p0: Ratio<u32> = Ratio::new_raw(3, 5);
        let mut p1: &u32 = &10;

        <Ratio<u32> as Div<&u32>>::div(p0, p1);
    }
}#[cfg(test)]
mod tests_rug_68 {
    use super::*;
    use crate::Ratio;
    use num_integer::Integer;
    use core::ops::Div;

    #[test]
    fn test_rug() {
        let mut p0: Ratio<i32> = Ratio::new(10, 5);
        let mut p1: i32 = 2;

        p0.div(p1);
    }
}#[cfg(test)]
mod tests_rug_69 {
    use super::*;
    use core::ops::Add;

    use crate::Ratio;

    #[test]
    fn test_rug() {
        let mut p0: Ratio<i32> = Ratio::new_raw(5, 10);
        let mut p1: Ratio<i32> = Ratio::new_raw(3, 5);
        
        p0.add(&p1);
    }
}#[cfg(test)]
mod tests_rug_70 {
    use super::*;
    use crate::Ratio;
    use num_integer::Integer;
    use core::ops::Add;
    
    #[test]
    fn test_rug() {
        let numer = 10;
        let denom = 2;
        let ratio = Ratio::new(numer, denom);
        
        let other: i32 = 5;

        ratio.add(&other);
    }
}#[cfg(test)]
mod tests_rug_73 {
    use super::*;
    use crate::Ratio;
    use core::ops::Add;

    #[test]
    fn test_rug() {
        let mut p0: Ratio<i32> = Ratio::new(3, 4);
        let p1: Ratio<i32> = Ratio::new(1, 2);

        p0.add(&p1);
    }
}#[cfg(test)]
mod tests_rug_74 {
    use super::*;
    use crate::Ratio;
    use num_integer::Integer;

    #[test]
    fn test_rug() {
        let p0: Ratio<i32> = Ratio::new(10, 3);
        let p1: i32 = 5;

        <Ratio<i32> as core::ops::Add<&i32>>::add(p0, &p1);
    }
}#[cfg(test)]
mod tests_rug_75 {
    use super::*;
    use num_traits::identities::One;
    use num_traits::ops::checked::{CheckedAdd, CheckedMul};
    use num_integer::Integer;
    
    #[test]
    fn test_rug() {
        let mut p0: Ratio<i32> = Ratio::new(10, 5);
        let mut p1: Ratio<i32> = Ratio::new(2, 3);

        p0.add(p1);
    }
}#[cfg(test)]
mod tests_rug_77 {
    use super::*;
    use num_traits::identities::Zero;
    use crate::Ratio;
    use core::ops::Sub;

    #[test]
    fn test_rug() {
        let mut p0: Ratio<i32> = Ratio::new_raw(15, 5);
        let mut p1: Ratio<i32> = Ratio::new_raw(5, 5);
        
        p0.sub(&p1);
    }
}#[cfg(test)]
mod tests_rug_80 {
    use super::*;
    use core::ops::Sub;
    use crate::Ratio;
    
    #[test]
    fn test_rug() {
        let mut p0: Ratio<i32> = Ratio::new_raw(10, 5);
        let mut p1: i32 = 2;
        
        p0.sub(p1);
    }
}#[cfg(test)]
mod tests_rug_81 {
    use super::*;
    use crate::Ratio;

    #[test]
    fn test_rug() {
        let mut p0: Ratio<i32> = Ratio::new_raw(10, 5);

        let mut p1: Ratio<i32> = Ratio::new_raw(3, 2);

        p0.sub(&p1);
    }
}#[cfg(test)]
mod tests_rug_83 {
    use super::*;
    use crate::Ratio;
    use num_integer::Integer;
    use std::ops::Sub;

    #[test]
    fn test_rug() {
        let mut p0 = Ratio::new(10, 2);
        let mut p1 = Ratio::new(5, 2);

        <Ratio<i32> as core::ops::Sub>::sub(p0, p1);
    }
}#[cfg(test)]
mod tests_rug_84 {
    use super::*;
    use crate::Ratio;
    use num_integer::Integer;
    use core::clone::Clone;
    use core::marker::Sized;

    #[test]
    fn test_rug() {
        let mut p0: Ratio<i32> = Ratio::new(4, 2);
        let mut p1: i32 = 2;

        p0.sub(p1);
    }
}#[cfg(test)]
mod tests_rug_85 {
    use super::*;
    use crate::Ratio;
    use core::ops::Rem;

    #[test]
    fn test_rug() {
        let numer0 = 10;
        let denom0 = 5;
        let numer1 = 5;
        let denom1 = 2;

        let mut p0: Ratio<i32> = Ratio::new(numer0, denom0);
        let mut p1: Ratio<i32> = Ratio::new(numer1, denom1);
        
        p0.rem(&p1);

    }
}#[cfg(test)]
mod tests_rug_90 {
    use super::*;
    use crate::Ratio;
    use num_integer::Integer;
    use core::ops::Rem;

    #[test]
    fn test_rug() {
        let mut p0: Ratio<i32> = Ratio::new(4, 5);
        let mut p1: &i32 = &10;

        <Ratio<i32> as core::ops::Rem<&i32>>::rem(p0, p1);
    }
}#[cfg(test)]
mod tests_rug_93 {
    use super::*;
    use crate::{Ratio, CheckedMul};
    
    #[test]
    fn test_checked_mul() {
        let mut p0 = Ratio::new(10, 3);
        let mut p1 = Ratio::new(5, 2);
        
        p0.checked_mul(&p1);
    }
}#[cfg(test)]
mod tests_rug_97 {
    use super::*;
    use crate::Ratio;
    use core::ops::Neg;

    #[test]
    fn test_neg() {
        let p0: Ratio<i32> = Ratio::new(10, 5);

        p0.neg();
    }
}#[cfg(test)]
mod tests_rug_98 {
    use super::*;
    use crate::Ratio;
    use core::ops::Neg;

    #[test]
    fn test_rug() {
        let numer: i32 = 5;
        let denom: i32 = 2;
        let p0: Ratio<i32> = Ratio::new(numer, denom);

        p0.neg();
    }
}#[cfg(test)]
mod tests_rug_99 {
    use super::*;
    use crate::Ratio;
    use num_traits::Inv;
    
    #[test]
    fn test_rug() {
        let mut p0 = Ratio::new(10, 3);

        <Ratio<_> as Inv>::inv(p0);
    }
}#[cfg(test)]
mod tests_rug_105 {
    use super::*;
    use crate::Ratio;
    use num_traits::One;

    #[test]
    fn test_rug() {
        let mut p0: Ratio<u32> = Ratio::new(5, 5);

        p0.is_one();
    }
}#[cfg(test)]
mod tests_rug_108 {
    use super::*;
    use crate::Ratio;
    use num_traits::Signed;

    #[test]
    fn test_abs() {
        let mut p0 = Ratio::new(5, 10);

        <Ratio<_> as Signed>::abs(&p0);
    }
}
#[cfg(test)]
mod tests_rug_109 {
    use super::*;
    use crate::{Ratio, Signed, Zero}; // Combined use statements

    #[test]
    fn test_rug() {
        let mut p0: Ratio<_> = Ratio::new_raw(10, 3); // Sample variable
        let mut p1: Ratio<_> = Ratio::new_raw(5, 2); // Sample variable

        p0.abs_sub(&p1);

    }
}
#[cfg(test)]
mod tests_rug_110 {
    use super::*;
    use num_traits::One;
    use num_traits::Zero;
    use num_traits::Signed;
    use crate::Ratio;

    #[test]
    fn test_rug() {
        let mut p0: Ratio<i32> = Ratio::new_raw(2, 3);

        <Ratio<i32> as Signed>::signum(&p0);
    }
}#[cfg(test)]
mod tests_rug_111 {
    use super::*;
    use crate::Ratio;
    use num_traits::Signed;

    #[test]
    fn test_rug() {
        let numer = 5;
        let denom = 2;
        let p0 = Ratio {
            numer,
            denom,
        };

        assert_eq!(<Ratio<_> as Signed>::is_positive(&p0), true);
    }
}#[cfg(test)]
mod tests_rug_112 {
    use super::*;
    use crate::Ratio;
    use num_traits::Signed;

    #[test]
    fn test_rug() {
        let p0 = Ratio {
            numer: -5,
            denom: 2,
        };

        assert_eq!(true, <Ratio<i32> as Signed>::is_negative(&p0));
    }
}#[cfg(test)]
mod tests_rug_113 {
    use super::*;
    use core::str::FromStr;

    #[test]
    fn test_from_str() {
        // Sample input data
        let p0 = "3/4";

        let result = <Ratio<i32> as core::str::FromStr>::from_str(&p0);

        assert_eq!(result, Ok(Ratio::new(3, 4)));
    }
}#[cfg(test)]
mod tests_rug_114 {
    use super::*;
    use crate::Ratio;
    use core::convert::Into;

    #[test]
    fn test_into() {
        let p0: Ratio<i32> = Ratio::new(3, 5);

        <Ratio<i32> as core::convert::Into<(i32, i32)>>::into(p0);
    }
}#[cfg(test)]
mod tests_rug_116 {
    use super::*;
    use crate::RatioErrorKind;

    #[test]
    fn test_rug() {
        let p0 = RatioErrorKind::ParseError;

        assert_eq!(p0.description(), "failed to parse integer");
    }
}#[cfg(test)]
mod tests_rug_117 {
    use super::*;
    use num_traits::FromPrimitive; // Removed duplicate use statement

    #[test]
    fn test_rug() {
        let p0: i64 = 10;

        <Ratio<num_bigint::BigInt> as FromPrimitive>::from_i64(p0);
    }
}#[cfg(test)]
mod tests_rug_118 {
    use super::*;
    use crate::Ratio;
    use num_bigint::BigInt;
    use num_traits::FromPrimitive;

    #[test]
    fn test_from_i128() {
        let p0: i128 = 100;

        <Ratio<BigInt> as FromPrimitive>::from_i128(p0);
    }
}#[cfg(test)]
mod tests_rug_119 {
    use super::*;
    use num_traits::FromPrimitive;
    use num_bigint::BigInt;
    use num_traits::identities::One;
    
    #[test]
    fn test_from_u64() {
        let p0: u64 = 42;

        let result = <Ratio<BigInt> as FromPrimitive>::from_u64(p0);
        assert_eq!(result, Some(Ratio::from_integer(One::one())));
    }
}#[cfg(test)]
mod tests_rug_120 {
    use super::*;
    use num_traits::FromPrimitive;
    use crate::Ratio;
    use num_bigint::BigInt;

    #[test]
    fn test_from_u128() {
        let p0: u128 = 123;

        let res = Ratio::<BigInt>::from_u128(p0);
        assert_eq!(res, Some(Ratio::from_integer(BigInt::from(p0))));
    }
}#[cfg(test)]
mod tests_rug_121 {
    use super::*;
    use num_traits::FromPrimitive;
    use num_bigint::BigInt;
    use crate::{Ratio};

    #[test]
    fn test_rug() {
        let p0: f32 = 3.14;
        
        <Ratio<BigInt> as FromPrimitive>::from_f32(p0);
    }
}#[cfg(test)]
mod tests_rug_122 {
    use super::*;
    use crate::Ratio;
    use num_bigint::BigInt;
    use num_traits::FromPrimitive;

    #[test]
    fn test_from_f64() {
        let mut p0: f64 = 3.14;

        let result = <Ratio<BigInt> as FromPrimitive>::from_f64(p0);
        assert_eq!(result, Some(Ratio::new_raw(BigInt::from(157), BigInt::from(50))));
    }
}#[cfg(test)]
mod tests_rug_123 {
    use super::*;
    use num_traits::FromPrimitive;
    use crate::Ratio;

    #[test]
    fn test_from_i64() {
        let p0: i64 = 10;

        let result = <Ratio<i8> as FromPrimitive>::from_i64(p0);
        let expected = Some(Ratio::from_integer(10));

        assert_eq!(result, expected);
    }
}#[cfg(test)]
mod tests_rug_124 {
    use super::*;
    use num_traits::FromPrimitive;
    use crate::Ratio;

    #[test]
    fn test_rug() {
        let p0: i128 = 100;

        <Ratio<i8> as FromPrimitive>::from_i128(p0);
    }
}#[cfg(test)]
mod tests_rug_126 {
    use super::*;
    use num_traits::FromPrimitive;
    use crate::Ratio;

    #[test]
    fn test_from_u128() {
        let n: u128 = 123456789;

        let result = Ratio::<i8>::from_u128(n);

        assert_eq!(result, None);
    }
}#[cfg(test)]
mod tests_rug_127 {
    use super::*;
    use crate::Ratio;
    use num_traits::FromPrimitive;

    #[test]
    fn test_rug() {
        let p0: f32 = 3.14;

        assert_eq!(Ratio::<i8>::from_f32(p0), None);
    }
}#[cfg(test)]
mod tests_rug_128 {
    use super::*;
    use num_traits::FromPrimitive;
    use crate::Ratio;

    #[test]
    fn test_from_f64() {
        let p0: f64 = 3.14;

        assert_eq!(Ratio::<i8>::from_f64(p0), None);
    }
}#[cfg(test)]
mod tests_rug_129 {
    use super::*;
    use crate::{Ratio, FromPrimitive};

    #[test]
    fn test_from_i64() {
        let p0: i64 = 10;

        assert_eq!(Ratio::<i16>::from_i64(p0), Some(Ratio::from_integer(10)));
    }
}#[cfg(test)]
mod tests_rug_131 {
    use super::*;
    use crate::Ratio;
    use num_traits::FromPrimitive;

    #[test]
    fn test_from_u64() {
        let n: u64 = 10;
        let result = <Ratio<i16> as FromPrimitive>::from_u64(n);
        
        assert_eq!(result, Some(Ratio::from_integer(10)));
    }
}#[cfg(test)]
mod tests_rug_132 {
    use super::*;
    use crate::Ratio;
    use num_traits::FromPrimitive;

    #[test]
    fn test_from_u128() {
        let n: u128 = 123;

        let result = <Ratio<i16> as FromPrimitive>::from_u128(n);

        assert_eq!(result, Some(Ratio::from_integer(n as i16)));
    }
}#[cfg(test)]
mod tests_rug_133 {
    use super::*;
    use crate::Ratio;
    use num_traits::FromPrimitive;

    #[test]
    fn test_from_f32() {
        let p0: f32 = 3.14;

        let result = <Ratio<i16> as FromPrimitive>::from_f32(p0);
        assert_eq!(result, None);
    }
}#[cfg(test)]
mod tests_rug_134 {
    use super::*;
    use crate::Ratio;
    use num_traits::FromPrimitive;

    #[test]
    fn test_from_f64() {
        let p0: f64 = 3.14159;

        let result = <Ratio<i16> as num_traits::FromPrimitive>::from_f64(p0);
        assert_eq!(result, Some(Ratio::new(707, 225)));
    }
}#[cfg(test)]
mod tests_rug_136 {
    use super::*;
    use num_traits::FromPrimitive;
    use crate::Ratio;
    
    #[test]
    fn test_rug() {
        let p0: i128 = 123;

        let _ = Ratio::<i32>::from_i128(p0);

        // Add your assertions here
    }
}#[cfg(test)]
mod tests_rug_137 {
    use super::*;
    use crate::{Ratio, FromPrimitive}; // Assuming num_rational crate is imported as num_rational

    #[test]
    fn test_from_u64() {
        let p0: u64 = 10;

        let result = <Ratio<i32> as FromPrimitive>::from_u64(p0);
    }
}#[cfg(test)]
mod tests_rug_138 {
    use super::*;
    use crate::Ratio;
    use num_traits::FromPrimitive;

    #[test]
    fn test_from_u128() {
        let n: u128 = 100;

        let result = Ratio::<i32>::from_u128(n);

        assert_eq!(result, None);
    }
}#[cfg(test)]
mod tests_rug_139 {
    use super::*;
    use crate::Ratio;
    use num_traits::FromPrimitive;

    #[test]
    fn test_from_f32() {
        let p0: f32 = 3.14;

        <Ratio<i32> as FromPrimitive>::from_f32(p0);
    }
}#[cfg(test)]
mod tests_rug_140 {
    use super::*;
    use crate::Ratio;
    use num_traits::FromPrimitive;

    #[test]
    fn test_rug() {
        let p0: f64 = 3.14159;

        assert_eq!(Ratio::<i32>::from_f64(p0), Some(Ratio::new(707065141, 224422223))); // asserting against a sample return value
    }
}#[cfg(test)]
mod tests_rug_141 {
    use super::*;
    use crate::{Ratio, FromPrimitive};

    #[test]
    fn test_rug() {
        let mut p0: i64 = 10;

        <Ratio<i64> as FromPrimitive>::from_i64(p0);

    }
}#[cfg(test)]
mod tests_rug_142 {
    use super::*;
    use crate::Ratio;
    use num_traits::FromPrimitive;

    #[test]
    fn test_ratio_from_i128() {
        // Sample i128 input value
        let n: i128 = 100;

        let result = <Ratio<i64> as FromPrimitive>::from_i128(n);

        // Add assertions as needed
        // For example:
        assert_eq!(result, Some(Ratio::new(100, 1)));
    }
}#[cfg(test)]
mod tests_rug_143 {
    use super::*;
    use crate::Ratio;
    use num_traits::FromPrimitive;

    #[test]
    fn test_from_u64() {
        let p0: u64 = 10;

        let result = <Ratio<i64> as FromPrimitive>::from_u64(p0);

        assert_eq!(result, Some(Ratio::from_integer(10)));
    }
}#[cfg(test)]
mod tests_rug_144 {
    use super::*;
    use num_traits::FromPrimitive;
    use crate::Ratio;

    #[test]
    fn test_from_u128() {
        let p0: u128 = 12345;

        let result = Ratio::<i64>::from_u128(p0);

        assert_eq!(result, Some(Ratio::from_integer(12345)));
    }
}#[cfg(test)]
mod tests_rug_145 {
    use super::*;
    use crate::{Ratio, Rational};
    use num_traits::FromPrimitive;

    #[test]
    fn test_rug() {
        let mut p0: f32 = 3.14;
        
        <Ratio<i64> as FromPrimitive>::from_f32(p0);
    }
}#[cfg(test)]
mod tests_rug_146 {
    use super::*;
    use num_traits::FromPrimitive;
    use num_traits::identities::One;
    use crate::Ratio;
    
    #[test]
    fn test_rug() {
        let mut p0: f64 = 3.14159;

        let result = <Ratio<i64> as FromPrimitive>::from_f64(p0);
        assert_eq!(result, Some(Ratio::new(7070651414971679, 2251799813685248))); // These values are just for demonstration, make sure to update with actual expected values
    }
}#[cfg(test)]
mod tests_rug_149 {
    use super::*;
    use crate::Ratio;
    use num_traits::FromPrimitive;

    #[test]
    fn test_from_u64() {
        let p0: u64 = 10;

        Ratio::<i128>::from_u64(p0);
    }
}#[cfg(test)]
mod tests_rug_151 {
    use super::*;
    use crate::Ratio;
    use num_traits::FromPrimitive;
    
    #[test]
    fn test_from_f32() {
        let p0: f32 = 3.14;
        
        assert_eq!(<Ratio<i128> as FromPrimitive>::from_f32(p0), None);
    }
}#[cfg(test)]
mod tests_rug_152 {
    use super::*;
    use num_traits::FromPrimitive;
    use crate::Ratio;

    #[test]
    fn test_rug() {
        let p0: f64 = 0.5;

        let result = Ratio::<i128>::from_f64(p0);
        assert_eq!(result, Some(Ratio::new_raw(1, 2)));
    }
}#[cfg(test)]
mod tests_rug_153 {
    use super::*;
    use num_traits::FromPrimitive;
    use crate::{Ratio, Rational};

    #[test]
    fn test_from_i64() {
        let n: i64 = 10;
        
        let result = Ratio::<isize>::from_i64(n);

        assert_eq!(result, Some(Ratio::new(10, 1)));
    }
}
#[cfg(test)]
mod tests_rug_154 {
    use super::*;
    use crate::{Ratio, FromPrimitive}; //assuming num_rational is the crate name

    #[test]
    fn test_from_i128() {
        let p0: i128 = 100;

        let result = Ratio::<isize>::from_i128(p0);

        // Add your assertions here based on the expected behavior of the function
        // For example, you can assert that the result is Some(Ratio) or any other expected behavior
        // assert_eq!(result, Some(expected_value));
    }
}
#[cfg(test)]
mod tests_rug_155 {
    use super::*;
    use crate::Ratio;
    use num_traits::FromPrimitive;

    #[test]
    fn test_ratio_from_u64() {
        let mut p0: u64 = 10;

        let result = <Ratio<isize> as num_traits::FromPrimitive>::from_u64(p0);

        assert_eq!(result, Some(Ratio::from_integer(10)));
    }
}#[cfg(test)]
mod tests_rug_157 {
    use super::*;
    use crate::Ratio;
    use num_traits::FromPrimitive;

    #[test]
    fn test_from_f32() {
        let n: f32 = 3.14;

        let result = <Ratio<isize> as FromPrimitive>::from_f32(n);

        assert_eq!(result, Some(Ratio::new(707065141, 2251799813685248)));
    }
}#[cfg(test)]
mod tests_rug_158 {
    use super::*;
    use num_traits::FromPrimitive;

    #[test]
    fn test_from_f64() {
        // Sample f64 data
        let p0: f64 = 3.14159;

        assert_eq!(<Ratio<isize> as FromPrimitive>::from_f64(p0), None);
    }
}#[cfg(test)]
mod tests_rug_159 {
    use super::*;
    use num_traits::FromPrimitive;

    #[test]
    fn test_rug() {
        let p0: i64 = 10;

        let result = <Ratio<u8> as FromPrimitive>::from_i64(p0);
    }
}
#[cfg(test)]
mod tests_rug_160 {
    use super::*;
    use crate::{Ratio, FromPrimitive};

    #[test]
    fn test_rug() {
        let mut p0: i128 = 100;

        let result = <Ratio<u8> as FromPrimitive>::from_i128(p0);
        assert_eq!(result, Some(Ratio::from_integer(100)));
    }
}
#[cfg(test)]
mod tests_rug_161 {
    use super::*;
    use crate::Ratio;
    use num_traits::FromPrimitive;
    
    #[test]
    fn test_from_u64() {
        let p0: u64 = 10;
        
        let result = <Ratio<u8> as FromPrimitive>::from_u64(p0).unwrap();
        assert_eq!(<Ratio<u8>>::from_u64(p0), Some(result));
    }
}#[cfg(test)]
mod tests_rug_162 {
    use super::*;
    use crate::{Ratio, FromPrimitive};

    #[test]
    fn test_from_u128() {
        let p0: u128 = 12345;

        assert_eq!(Ratio::<u8>::from_u128(p0), Some(Ratio::from_integer(p0 as u8)));
    }
}#[cfg(test)]
mod tests_rug_163 {
    use super::*;
    use crate::Ratio;
    use num_traits::FromPrimitive;

    #[test]
    fn test_from_f32() {
        let p0: f32 = 3.14;

        let result = Ratio::<u8>::from_f32(p0);
        assert_eq!(result, None);
    }
}#[cfg(test)]
mod tests_rug_164 {
    use super::*;
    use num_traits::FromPrimitive;
    use crate::Ratio;

    #[test]
    fn test_rug() {
        let p0: f64 = 3.14;

        <Ratio<u8> as FromPrimitive>::from_f64(p0);
  
    }
}#[cfg(test)]
mod tests_rug_167 {
    use super::*;
    use crate::Ratio;
    use num_traits::FromPrimitive;

    #[test]
    fn test_from_u64() {
        let p0: u64 = 100;

        <Ratio<u16> as FromPrimitive>::from_u64(p0);

    }
}#[cfg(test)]
mod tests_rug_168 {
    use super::*;
    use crate::Ratio;
    use num_traits::FromPrimitive;
    
    #[test]
    fn test_ratio_from_u128() {
        let n: u128 = 12345;

        let result = Ratio::<u16>::from_u128(n);
        // Add assertions based on your requirements
    }
}#[cfg(test)]
mod tests_rug_169 {
    use super::*;
    use crate::Ratio;
    use num_traits::FromPrimitive;

    #[test]
    fn test_from_f32() {
        let p0: f32 = 3.14159;
        
        assert_eq!(Ratio::<u16>::from_f32(p0), None);
    }
}#[cfg(test)]
mod tests_rug_170 {
    use super::*;
    use crate::Ratio;
    use num_traits::FromPrimitive;

    #[test]
    fn test_from_f64() {
        // Sample data for the f64 argument
        let p0: f64 = 3.14;

        Ratio::<u16>::from_f64(p0);
    }
}#[cfg(test)]
mod tests_rug_171 {
    use super::*;
    use num_traits::FromPrimitive;

    #[test]
    fn test_from_i64() {
        let n: i64 = 10;

        let result = <Ratio<u32> as FromPrimitive>::from_i64(n);

        assert_eq!(result, Some(Ratio::from_integer(n as u32)));
    }
}#[cfg(test)]
mod tests_rug_172 {
    use super::*;
    use crate::{Ratio, FromPrimitive};

    #[test]
    fn test_from_i128() {
        let n: i128 = 123456;
        
        let result = Ratio::<u32>::from_i128(n);

        // Add assertions here
        assert_eq!(result, Some(Ratio::new_raw(123456, 1)));
    }
}#[cfg(test)]
mod tests_rug_174 {
    use super::*;
    use num_traits::FromPrimitive;
    use crate::Ratio;

    #[test]
    fn test_from_u128() {
        let mut p0: u128 = 100;

        let result = <Ratio<u32> as FromPrimitive>::from_u128(p0);

        // Assert statements can be added here
        assert_eq!(result, Some(Ratio::new_raw(100, 1)));
    }
}#[cfg(test)]
mod tests_rug_175 {
    use super::*;
    use crate::Ratio;
    use num_traits::FromPrimitive;

    #[test]
    fn test_rug() {
        let mut p0: f32 = 3.14;

        let _result = Ratio::<u32>::from_f32(p0);
    }
}#[cfg(test)]
mod tests_rug_176 {
    use super::*;
    use crate::Ratio;
    use num_traits::FromPrimitive;

    #[test]
    fn test_rug() {
        let p0: f64 = 3.14159;
        let res = Ratio::<u32>::from_f64(p0);
        assert_eq!(res, Some(Ratio::new_raw(707065141, 224422971)));
    }
}#[cfg(test)]
mod tests_rug_177 {
    use super::*;
    use crate::Ratio;
    use num_traits::FromPrimitive;

    #[test]
    fn test_from_i64() {
        let p0: i64 = -5;

        assert_eq!(Ratio::<u64>::from_i64(p0), None);
    }
}#[cfg(test)]
mod tests_rug_178 {
    use super::*;
    use crate::Ratio;
    use num_traits::FromPrimitive;

    #[test]
    fn test_rug() {
        let p0: i128 = 100;

        let _ = Ratio::<u64>::from_i128(p0);
    }
}#[cfg(test)]
mod tests_rug_179 {
    use super::*;
    use num_traits::FromPrimitive;
    use crate::Ratio;

    #[test]
    fn test_rug() {
        let mut p0: u64 = 10;

        <Ratio<u64> as FromPrimitive>::from_u64(p0);

    }
}#[cfg(test)]
mod tests_rug_180 {
    use super::*;
    use num_traits::FromPrimitive;

    #[test]
    fn test_from_u128() {
        let p0: u128 = 1234567890;

        Ratio::<u64>::from_u128(p0);
    }
}#[cfg(test)]
mod tests_rug_181 {
    use super::*;
    use num_traits::FromPrimitive;
    use crate::Ratio;

    #[test]
    fn test_from_f32() {
        let p0: f32 = 3.14;

        let result = <Ratio<u64> as FromPrimitive>::from_f32(p0);
        assert_eq!(result, None);
    }
}#[cfg(test)]
mod tests_rug_182 {
    use super::*;
    use crate::Ratio;
    use num_traits::FromPrimitive;

    #[test]
    fn test_from_f64() {
        let p0: f64 = 3.14159;
        
        let _ = Ratio::<u64>::from_f64(p0);
    }
}#[cfg(test)]
mod tests_rug_184 {
    use super::*;
    use num_traits::FromPrimitive;
    use crate::Ratio;
    
    #[test]
    fn test_from_i128() {
        let p0: i128 = 10;
        
        <Ratio<u128> as FromPrimitive>::from_i128(p0);

    }
}#[cfg(test)]
mod tests_rug_185 {
    use super::*;
    use num_traits::FromPrimitive;
    use crate::Ratio;

    #[test]
    fn test_rug() {
        let mut p0: u64 = 10;

        <Ratio<u128> as FromPrimitive>::from_u64(p0).unwrap();
    }
}#[cfg(test)]
mod tests_rug_186 {
    use super::*;
    use num_traits::FromPrimitive;
    use crate::Ratio;

    #[test]
    fn test_from_u128() {
        let p0: u128 = 1234567890;

        let result = Ratio::<u128>::from_u128(p0);
        
        assert_eq!(result, Some(Ratio::from_integer(p0)));
    }
}
#[cfg(test)]
mod tests_rug_187 {
    use super::*;
    use crate::Ratio;
    use num_traits::FromPrimitive;

    #[test]
    fn test_from_f32() {
        let p0: f32 = 3.14159;
                
        <Ratio<u128> as num_traits::FromPrimitive>::from_f32(p0);

    }
}
#[cfg(test)]
mod tests_rug_188 {
    use super::*;
    use num_traits::FromPrimitive;
    use crate::Ratio;

    #[test]
    fn test_from_f64() {
        let p0: f64 = 3.14;

        assert_eq!(<Ratio<u128> as FromPrimitive>::from_f64(p0), Some(Ratio::<u128>::new_raw(7070651414971674, 2251799813685248)));
    }
}#[cfg(test)]
mod tests_rug_189 {
    use super::*;
    use crate::{Ratio, FromPrimitive}; // assuming num_rational is the crate name
    
    #[test]
    fn test_from_i64() {
        let n: i64 = 10;

        let result = <Ratio<usize> as FromPrimitive>::from_i64(n);

        assert_eq!(result, Some(Ratio::from_integer(n as usize)));
    }
}#[cfg(test)]
mod tests_rug_190 {
    use super::*;
    use crate::Ratio;
    use num_traits::FromPrimitive;

    #[test]
    fn test_from_i128() {
        let p0: i128 = 42;

        let result = <Ratio<usize> as FromPrimitive>::from_i128(p0);

        assert_eq!(result, Some(Ratio::new(42, 1)));
    }
}#[cfg(test)]
mod tests_rug_192 {
    use super::*;
    use crate::Ratio;
    use num_traits::FromPrimitive;

    #[test]
    fn test_from_u128() {
        // Sample data for initializing the variable
        let n: u128 = 123456789;

        let result = <Ratio<usize> as FromPrimitive>::from_u128(n);

        assert_eq!(result, Some(Ratio::from_integer(n as usize)));
    }
}#[cfg(test)]
mod tests_rug_193 {
    use super::*;
    use crate::Ratio;
    use num_traits::FromPrimitive;

    #[test]
    fn test_rug() {
        let p0: f32 = 3.14159;
        
        let result = <Ratio<usize> as FromPrimitive>::from_f32(p0);
        // Add assertions here
    }
}#[cfg(test)]
mod tests_rug_194 {
    use super::*;
    use crate::Ratio;
    use num_traits::FromPrimitive;

    #[test]
    fn test_from_f64() {
        let n = 3.14159;
        let p0: f64 = n;

        assert_eq!(Ratio::<usize>::from_f64(p0), Some(Ratio::new_raw(7074237752028445, 2251799813685248)));
    }
}#[cfg(test)]
mod tests_rug_196 {
    use super::*;
    use crate::Ratio;
    use num_traits::ToPrimitive;

    #[test]
    fn test_to_i64() {
        let p0 = Ratio::<i32>::new(5, 2);

        assert_eq!(p0.to_i64(), Some(2));
    }
}#[cfg(test)]
mod tests_rug_197 {
    use super::*;
    use crate::Ratio;
    use num_traits::ToPrimitive;

    #[test]
    fn test_to_i128() {
        let p0 = Ratio::<i16>::new(3, 2);

        p0.to_i128();
    }
}#[cfg(test)]
mod tests_rug_198 {
    use super::*;
    use crate::Ratio;
    use num_traits::ToPrimitive;

    #[test]
    fn test_rug() {
        let mut p0 = Ratio::new(10, 5);

        p0.to_u64();
    }
}#[cfg(test)]
mod tests_rug_199 {
    use super::*;
    use crate::Ratio;
    use num_traits::ToPrimitive;

    #[test]
    fn test_to_u128() {
        let p0 = Ratio::<i32>::new(10, 5);

        p0.to_u128();
    }
}#[cfg(test)]
mod tests_rug_200 {
    use super::*;
    use num_traits::ToPrimitive;
    use crate::Ratio;
    
    #[test]
    fn test_rug() {
        let mut p0: Ratio<i32> = Ratio::new(3, 2);

        p0.to_f64();

    }
}#[cfg(test)]
mod tests_rug_201 {
    use super::*;
    use crate::Bits;
    use num_bigint::BigInt;
    
    #[test]
    fn test_rug() {
        let mut p0: BigInt = BigInt::from(123456789);

        BigInt::bits(&p0);

    }
}#[cfg(test)]
mod tests_rug_202 {
    use super::*;
    use num_traits::identities::One;
    use num_traits::identities::Zero;

    #[test]
    fn test_rug() {
        let mut p0: i128 = 123;

        <i128 as Bits>::bits(&p0);
    }
}