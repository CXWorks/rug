use crate::Ratio;

use core::cmp;
use num_integer::Integer;
use num_traits::{One, Pow};

macro_rules! pow_unsigned_impl {
    (@ $exp:ty) => {
        type Output = Ratio<T>;
        #[inline]
        fn pow(self, expon: $exp) -> Ratio<T> {
            Ratio::new_raw(self.numer.pow(expon), self.denom.pow(expon))
        }
    };
    ($exp:ty) => {
        impl<T: Clone + Integer + Pow<$exp, Output = T>> Pow<$exp> for Ratio<T> {
            pow_unsigned_impl!(@ $exp);
        }
        impl<'a, T: Clone + Integer> Pow<$exp> for &'a Ratio<T>
        where
            &'a T: Pow<$exp, Output = T>,
        {
            pow_unsigned_impl!(@ $exp);
        }
        impl<'b, T: Clone + Integer + Pow<$exp, Output = T>> Pow<&'b $exp> for Ratio<T> {
            type Output = Ratio<T>;
            #[inline]
            fn pow(self, expon: &'b $exp) -> Ratio<T> {
                Pow::pow(self, *expon)
            }
        }
        impl<'a, 'b, T: Clone + Integer> Pow<&'b $exp> for &'a Ratio<T>
        where
            &'a T: Pow<$exp, Output = T>,
        {
            type Output = Ratio<T>;
            #[inline]
            fn pow(self, expon: &'b $exp) -> Ratio<T> {
                Pow::pow(self, *expon)
            }
        }
    };
}
pow_unsigned_impl!(u8);
pow_unsigned_impl!(u16);
pow_unsigned_impl!(u32);
pow_unsigned_impl!(u64);
pow_unsigned_impl!(u128);
pow_unsigned_impl!(usize);

macro_rules! pow_signed_impl {
    (@ &'b BigInt, BigUint) => {
        type Output = Ratio<T>;
        #[inline]
        fn pow(self, expon: &'b BigInt) -> Ratio<T> {
            match expon.sign() {
                Sign::NoSign => One::one(),
                Sign::Minus => {
                    Pow::pow(self, expon.magnitude()).into_recip()
                }
                Sign::Plus => Pow::pow(self, expon.magnitude()),
            }
        }
    };
    (@ $exp:ty, $unsigned:ty) => {
        type Output = Ratio<T>;
        #[inline]
        fn pow(self, expon: $exp) -> Ratio<T> {
            match expon.cmp(&0) {
                cmp::Ordering::Equal => One::one(),
                cmp::Ordering::Less => {
                    let expon = expon.wrapping_abs() as $unsigned;
                    Pow::pow(self, expon).into_recip()
                }
                cmp::Ordering::Greater => Pow::pow(self, expon as $unsigned),
            }
        }
    };
    ($exp:ty, $unsigned:ty) => {
        impl<T: Clone + Integer + Pow<$unsigned, Output = T>> Pow<$exp> for Ratio<T> {
            pow_signed_impl!(@ $exp, $unsigned);
        }
        impl<'a, T: Clone + Integer> Pow<$exp> for &'a Ratio<T>
        where
            &'a T: Pow<$unsigned, Output = T>,
        {
            pow_signed_impl!(@ $exp, $unsigned);
        }
        impl<'b, T: Clone + Integer + Pow<$unsigned, Output = T>> Pow<&'b $exp> for Ratio<T> {
            type Output = Ratio<T>;
            #[inline]
            fn pow(self, expon: &'b $exp) -> Ratio<T> {
                Pow::pow(self, *expon)
            }
        }
        impl<'a, 'b, T: Clone + Integer> Pow<&'b $exp> for &'a Ratio<T>
        where
            &'a T: Pow<$unsigned, Output = T>,
        {
            type Output = Ratio<T>;
            #[inline]
            fn pow(self, expon: &'b $exp) -> Ratio<T> {
                Pow::pow(self, *expon)
            }
        }
    };
}
pow_signed_impl!(i8, u8);
pow_signed_impl!(i16, u16);
pow_signed_impl!(i32, u32);
pow_signed_impl!(i64, u64);
pow_signed_impl!(i128, u128);
pow_signed_impl!(isize, usize);

#[cfg(feature = "num-bigint")]
mod bigint {
    use super::*;
    use num_bigint::{BigInt, BigUint, Sign};

    impl<T: Clone + Integer + for<'b> Pow<&'b BigUint, Output = T>> Pow<BigUint> for Ratio<T> {
        type Output = Ratio<T>;
        #[inline]
        fn pow(self, expon: BigUint) -> Ratio<T> {
            Pow::pow(self, &expon)
        }
    }
    impl<'a, T: Clone + Integer> Pow<BigUint> for &'a Ratio<T>
    where
        &'a T: for<'b> Pow<&'b BigUint, Output = T>,
    {
        type Output = Ratio<T>;
        #[inline]
        fn pow(self, expon: BigUint) -> Ratio<T> {
            Pow::pow(self, &expon)
        }
    }
    impl<'b, T: Clone + Integer + Pow<&'b BigUint, Output = T>> Pow<&'b BigUint> for Ratio<T> {
        pow_unsigned_impl!(@ &'b BigUint);
    }
    impl<'a, 'b, T: Clone + Integer> Pow<&'b BigUint> for &'a Ratio<T>
    where
        &'a T: Pow<&'b BigUint, Output = T>,
    {
        pow_unsigned_impl!(@ &'b BigUint);
    }

    impl<T: Clone + Integer + for<'b> Pow<&'b BigUint, Output = T>> Pow<BigInt> for Ratio<T> {
        type Output = Ratio<T>;
        #[inline]
        fn pow(self, expon: BigInt) -> Ratio<T> {
            Pow::pow(self, &expon)
        }
    }
    impl<'a, T: Clone + Integer> Pow<BigInt> for &'a Ratio<T>
    where
        &'a T: for<'b> Pow<&'b BigUint, Output = T>,
    {
        type Output = Ratio<T>;
        #[inline]
        fn pow(self, expon: BigInt) -> Ratio<T> {
            Pow::pow(self, &expon)
        }
    }
    impl<'b, T: Clone + Integer + Pow<&'b BigUint, Output = T>> Pow<&'b BigInt> for Ratio<T> {
        pow_signed_impl!(@ &'b BigInt, BigUint);
    }
    impl<'a, 'b, T: Clone + Integer> Pow<&'b BigInt> for &'a Ratio<T>
    where
        &'a T: Pow<&'b BigUint, Output = T>,
    {
        pow_signed_impl!(@ &'b BigInt, BigUint);
    }
}
#[cfg(test)]
mod tests_rug_203 {
    use super::*;
    use crate::{Ratio, Pow}; // Assuming num_rational crate is used in the project

    #[test]
    fn test_rug() {
        let mut p0: Ratio<i32> = Ratio::new_raw(5, 3); // Sample values for Ratio<T>
        let p1: u8 = 2; // Sample value for the exponent

        p0.pow(p1);
    }
}#[cfg(test)]
mod tests_rug_204 {
    use super::*;
    use crate::Ratio;
    use num_traits::Pow;
    
    #[test]
    fn test_rug() {
        let mut p0: Ratio<i32> = Ratio::new_raw(10, 2);
        let mut p1: u8 = 3;

        <&Ratio<i32>>::pow(&p0, p1);
    }
}#[cfg(test)]
mod tests_rug_205 {
    use super::*;
    use crate::Ratio;
    use num_traits::Pow;

    #[test]
    fn test_rug() {
        let mut p0: Ratio<i32> = Ratio::new_raw(3, 4);
        let mut p1: &u8 = &5;

        p0.pow(p1);
    }
}#[cfg(test)]
mod tests_rug_206 {
    use super::*;
    use num_traits::Pow;

    use crate::Ratio;

    #[test]
    fn test_rug() {
        let mut p0: Ratio<u32> = Ratio::new_raw(42, 7);
        let p1: &u8 = &5;

        <&Ratio<u32>>::pow(&p0, p1);
    }
}#[cfg(test)]
mod tests_rug_211 {
    use super::*;
    use crate::Ratio;
    use num_traits::Pow;
    
    #[test]
    fn test_rug() {
        let mut p0: Ratio<u32> = Ratio::new_raw(5, 2);
        let p1: u32 = 3;
        
        p0.pow(p1);
    }
}#[cfg(test)]
mod tests_rug_223 {
    use super::*;
    use crate::Ratio;
    use num_traits::Pow;

    #[test]
    fn test_rug() {
        let p0: Ratio<u32> = Ratio::new(10, 5);
        let p1: usize = 2;

        p0.pow(p1);
    }
}#[cfg(test)]
mod tests_rug_227 {
    use super::*;
    use crate::Ratio;
    use num_traits::identities::{One, Zero};
    use num_traits::Pow;

    #[test]
    fn test_rug() {
        let mut p0: Ratio<i32> = Ratio::new_raw(10, 3);
        let mut p1: i8 = 2;

        <Ratio<i32> as Pow<i8>>::pow(p0, p1);
    }
}#[cfg(test)]
mod tests_rug_230 {
    use super::*;
    use crate::Ratio;

    #[test]
    fn test_rug() {
        let mut p0: Ratio<i32> = Ratio::new_raw(10, 5);
        let p1: i8 = 2;
        
        p0.pow(&p1);
    }
}#[cfg(test)]
mod tests_rug_232 {
    use super::*;
    use crate::{Ratio, Pow};

    #[test]
    fn test_rug() {
        let mut p0: Ratio<i32> = Ratio::new_raw(1234, 5678);
        let p1: i16 = 3;

        p0.pow(p1);
    }
}#[cfg(test)]
mod tests_rug_235 {
    use super::*;
    use crate::{Ratio, Pow};
    use num_traits::One;

    #[test]
    fn test_rug() {
        let mut p0: Ratio<i32> = Ratio::new(1, 2);
        let p1: i32 = 3;

        p0.pow(p1);
    }
}#[cfg(test)]
mod tests_rug_236 {
    use super::*;
    use crate::{Ratio, Pow};

    #[test]
    fn test_rug() {
        let mut p0: Ratio<i32> = Ratio::new_raw(4, 3);
        let mut p1: i32 = 2;
        
        p0.pow(p1);
    }
}#[cfg(test)]
mod tests_rug_247 {
    use super::*;
    use crate::{Ratio, Pow, One};
    use std::cmp;
    
    #[test]
    fn test_rug() {
        let p0 = Ratio::new(10, 3);
        let p1: isize = -2;
        
        <Ratio<_> as Pow<_>>::pow(p0, p1);
    }
}#[cfg(test)]
mod tests_rug_249 {
    use super::*;
    use crate::{Ratio, Pow};

    #[test]
    fn test_rug() {
        let mut p0: Ratio<i32> = Ratio::new_raw(5, 2);
        let p1: isize = 3;
        
        p0.pow(&p1);
    }
}