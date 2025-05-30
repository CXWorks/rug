use core::num::Wrapping;
use core::{f32, f64};
use core::{i128, i16, i32, i64, i8, isize};
use core::{u128, u16, u32, u64, u8, usize};

/// Numbers which have upper and lower bounds
pub trait Bounded {
    // FIXME (#5527): These should be associated constants
    /// Returns the smallest finite number this type can represent
    fn min_value() -> Self;
    /// Returns the largest finite number this type can represent
    fn max_value() -> Self;
}

/// Numbers which have lower bounds
pub trait LowerBounded {
    /// Returns the smallest finite number this type can represent
    fn min_value() -> Self;
}

// FIXME: With a major version bump, this should be a supertrait instead
impl<T: Bounded> LowerBounded for T {
    fn min_value() -> T {
        Bounded::min_value()
    }
}

/// Numbers which have upper bounds
pub trait UpperBounded {
    /// Returns the largest finite number this type can represent
    fn max_value() -> Self;
}

// FIXME: With a major version bump, this should be a supertrait instead
impl<T: Bounded> UpperBounded for T {
    fn max_value() -> T {
        Bounded::max_value()
    }
}

macro_rules! bounded_impl {
    ($t:ty, $min:expr, $max:expr) => {
        impl Bounded for $t {
            #[inline]
            fn min_value() -> $t {
                $min
            }

            #[inline]
            fn max_value() -> $t {
                $max
            }
        }
    };
}

bounded_impl!(usize, usize::MIN, usize::MAX);
bounded_impl!(u8, u8::MIN, u8::MAX);
bounded_impl!(u16, u16::MIN, u16::MAX);
bounded_impl!(u32, u32::MIN, u32::MAX);
bounded_impl!(u64, u64::MIN, u64::MAX);
bounded_impl!(u128, u128::MIN, u128::MAX);

bounded_impl!(isize, isize::MIN, isize::MAX);
bounded_impl!(i8, i8::MIN, i8::MAX);
bounded_impl!(i16, i16::MIN, i16::MAX);
bounded_impl!(i32, i32::MIN, i32::MAX);
bounded_impl!(i64, i64::MIN, i64::MAX);
bounded_impl!(i128, i128::MIN, i128::MAX);

impl<T: Bounded> Bounded for Wrapping<T> {
    fn min_value() -> Self {
        Wrapping(T::min_value())
    }
    fn max_value() -> Self {
        Wrapping(T::max_value())
    }
}

bounded_impl!(f32, f32::MIN, f32::MAX);

macro_rules! for_each_tuple_ {
    ( $m:ident !! ) => (
        $m! { }
    );
    ( $m:ident !! $h:ident, $($t:ident,)* ) => (
        $m! { $h $($t)* }
        for_each_tuple_! { $m !! $($t,)* }
    );
}
macro_rules! for_each_tuple {
    ($m:ident) => {
        for_each_tuple_! { $m !! A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, }
    };
}

macro_rules! bounded_tuple {
    ( $($name:ident)* ) => (
        impl<$($name: Bounded,)*> Bounded for ($($name,)*) {
            #[inline]
            fn min_value() -> Self {
                ($($name::min_value(),)*)
            }
            #[inline]
            fn max_value() -> Self {
                ($($name::max_value(),)*)
            }
        }
    );
}

for_each_tuple!(bounded_tuple);
bounded_impl!(f64, f64::MIN, f64::MAX);

#[test]
fn wrapping_bounded() {
    macro_rules! test_wrapping_bounded {
        ($($t:ty)+) => {
            $(
                assert_eq!(<Wrapping<$t> as Bounded>::min_value().0, <$t>::min_value());
                assert_eq!(<Wrapping<$t> as Bounded>::max_value().0, <$t>::max_value());
            )+
        };
    }

    test_wrapping_bounded!(usize u8 u16 u32 u64 isize i8 i16 i32 i64);
}

#[test]
fn wrapping_bounded_i128() {
    macro_rules! test_wrapping_bounded {
        ($($t:ty)+) => {
            $(
                assert_eq!(<Wrapping<$t> as Bounded>::min_value().0, <$t>::min_value());
                assert_eq!(<Wrapping<$t> as Bounded>::max_value().0, <$t>::max_value());
            )+
        };
    }

    test_wrapping_bounded!(u128 i128);
}

#[test]
fn wrapping_is_bounded() {
    fn require_bounded<T: Bounded>(_: &T) {}
    require_bounded(&Wrapping(42_u32));
    require_bounded(&Wrapping(-42));
}
#[cfg(test)]
mod tests_rug_1433 {
    use super::*;
    use crate::bounds::Bounded;
    
    #[test]
    fn test_rug() {
        <u8 as Bounded>::min_value();
    }
}#[cfg(test)]
mod tests_rug_1437 {
    use super::*;
    use crate::Bounded;

    #[test]
    fn test_rug() {
        <u32 as Bounded>::min_value();
    }
}#[cfg(test)]
mod tests_rug_1443 {
    use super::*;
    use crate::Bounded;

    #[test]
    fn test_rug() {
        let result: isize = <isize as Bounded>::min_value();
        assert_eq!(result, isize::min_value());
    }
}
#[cfg(test)]
mod tests_rug_1444 {
    use super::*;
    use crate::Bounded;
    
    #[test]
    fn test_isize() {
        let max_value: isize = <isize as Bounded>::max_value();
        assert_eq!(max_value, isize::MAX);
    }
    
    #[test]
    fn test_i8() {
        let max_value: i8 = <i8 as Bounded>::max_value();
        assert_eq!(max_value, i8::MAX);
    }
    
    #[test]
    fn test_i16() {
        let max_value: i16 = <i16 as Bounded>::max_value();
        assert_eq!(max_value, i16::MAX);
    }
    
    #[test]
    fn test_i32() {
        let max_value: i32 = <i32 as Bounded>::max_value();
        assert_eq!(max_value, i32::MAX);
    }
    
    #[test]
    fn test_i64() {
        let max_value: i64 = <i64 as Bounded>::max_value();
        assert_eq!(max_value, i64::MAX);
    }
    
}
#[cfg(test)]
mod tests_rug_1447 {
    use super::*;
    use crate::{Bounded};

    #[test]
    fn test_rug() {
        let min_value = <i16 as Bounded>::min_value();
        assert_eq!(min_value, -32768);
    }
}
#[cfg(test)]
mod tests_rug_1449 {
    use super::*;

    use crate::bounds::Bounded;

    #[test]
    fn test_rug() {
        let val: i32 = Bounded::min_value();
    }
}
#[cfg(test)]
mod tests_rug_1453 {
    use super::*;
    use crate::bounds::Bounded;

    #[test]
    fn test_rug() {
        let result: i128 = <i128 as Bounded>::min_value();

        // add assertions here
    }
}
#[cfg(test)]
mod tests_rug_1455 {
    use super::*;
    use crate::Bounded;
    use std::num::Wrapping;

    #[test]
    fn test_rug() {
        let min_value: Wrapping<i32> = <Wrapping<i32> as Bounded>::min_value();
    }
}
#[cfg(test)]
mod tests_rug_1478 {
    use super::*;
    use crate::Bounded;

    #[test]
    fn test_rug() {
        <(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32) as Bounded>::max_value();
    }
}
#[cfg(test)]
mod tests_rug_1483 {
    use super::*;
    use crate::{Bounded};
    
    #[test]
    fn test_rug() {
        <(i8, i16, i32, i64, i128, isize, u8, u16) as Bounded>::min_value();
    }
}
#[cfg(test)]
mod tests_rug_1499 {
    use super::*;
    use crate::Bounded;
    #[test]
    fn test_rug() {
        <() as Bounded>::min_value();
    }
}#[cfg(test)]
mod tests_rug_1502 {
    use super::*;
    use crate::Bounded;

    #[test]
    fn test_rug() {
        <f64 as Bounded>::max_value();
    }
}