//! # Feature flags
//!
//! This crate exposes a number of features. These can be enabled or disabled as shown
//! [in Cargo's documentation](https://doc.rust-lang.org/cargo/reference/features.html). Features
//! are _disabled_ by default unless otherwise noted.
//!
//! Reliance on a given feature is always indicated alongside the item definition.
//!
//! - `std` (_enabled by default, implicitly enables `alloc`_)
//!
//!   This enables a number of features that depend on the standard library.
//!
//! - `alloc` (_enabled by default via `std`_)
//!
//!   Enables a number of features that require the ability to dynamically allocate memory.
//!
//! - `macros`
//!
//!   Enables macros that provide compile-time verification of values and intuitive syntax.
//!
//! - `formatting` (_implicitly enables `std`_)
//!
//!   Enables formatting of most structs.
//!
//! - `parsing`
//!
//!   Enables parsing of most structs.
//!
//! - `local-offset` (_implicitly enables `std`_)
//!
//!   This feature enables a number of methods that allow obtaining the system's UTC offset.
//!
//! - `large-dates`
//!
//!   By default, only years within the ±9999 range (inclusive) are supported. If you need support
//!   for years outside this range, consider enabling this feature; the supported range will be
//!   increased to ±999,999.
//!
//!   Note that enabling this feature has some costs, as it means forgoing some optimizations.
//!   Ambiguities may be introduced when parsing that would not otherwise exist.
//!
//!   If you are using this feature, **please leave a comment**
//!   [on this discussion](https://github.com/time-rs/time/discussions/306) with your use case. If
//!   there is not sufficient demand for this feature, it will be dropped in a future release.
//!
//! - `serde`
//!
//!   Enables [serde](https://docs.rs/serde) support for all types except [`Instant`].
//!
//! - `serde-human-readable` (_implicitly enables `serde`, `formatting`, and `parsing`_)
//!
//!   Allows serde representations to use a human-readable format. This is determined by the
//!   serializer, not the user. If this feature is not enabled or if the serializer requests a
//!   non-human-readable format, a format optimized for binary representation will be used.
//!
//!   Libraries should never enable this feature, as the decision of what format to use should be up
//!   to the user.
//!
//! - `serde-well-known` (_implicitly enables `serde/alloc`, `formatting`, and `parsing`_)
//!
//!   Enables support for serializing and deserializing well-known formats using serde's
//!   [`#[with]` attribute](https://serde.rs/field-attrs.html#with).
//!
//! - `rand`
//!
//!   Enables [rand](https://docs.rs/rand) support for all types.
//!
//! - `quickcheck` (_implicitly enables `alloc`_)
//!
//!   Enables [quickcheck](https://docs.rs/quickcheck) support for all types except [`Instant`].
//!
//! One pseudo-feature flag that is only available to end users is the `unsound_local_offset` cfg.
//! As the name indicates, using the feature is unsound, and [may cause unexpected segmentation
//! faults](https://github.com/time-rs/time/issues/293). Unlike other flags, this is deliberately
//! only available to end users; this is to ensure that a user doesn't have unsound behavior without
//! knowing it. To enable this behavior, you must use `RUSTFLAGS="--cfg unsound_local_offset" cargo
//! build` or similar. Note: This flag is _not tested anywhere_, including in the regular test of
//! the powerset of all feature flags. Use at your own risk. Without this flag, any method that
//! requires the local offset will return the `Err` variant.
#![feature(no_coverage)]

#![doc(html_playground_url = "https://play.rust-lang.org")]
#![cfg_attr(__time_03_docs, feature(doc_cfg, doc_auto_cfg, doc_notable_trait))]
#![cfg_attr(
    __time_03_docs,
    deny(rustdoc::broken_intra_doc_links, rustdoc::private_intra_doc_links)
)]
#![cfg_attr(not(feature = "std"), no_std)]
/*
#![deny(
    anonymous_parameters,
    clippy::all,
    const_err,
    illegal_floating_point_literal_pattern,
    late_bound_lifetime_arguments,
    path_statements,
    patterns_in_fns_without_body,
    rust_2018_idioms,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unsafe_code,
    unsafe_op_in_unsafe_fn,
    unused_extern_crates
)]
#![warn(
    clippy::dbg_macro,
    clippy::decimal_literal_representation,
    clippy::get_unwrap,
    clippy::missing_docs_in_private_items,
    clippy::nursery,
    clippy::print_stdout,
    clippy::todo,
    clippy::unimplemented,
    clippy::unnested_or_patterns,
    clippy::unwrap_in_result,
    clippy::unwrap_used,
    clippy::use_debug,
    deprecated_in_future,
    missing_copy_implementations,
    missing_debug_implementations,
    unused_qualifications,
    variant_size_differences
)]
*/
#![allow(clippy::redundant_pub_crate)]
#![doc(html_favicon_url = "https://avatars0.githubusercontent.com/u/55999857")]
#![doc(html_logo_url = "https://avatars0.githubusercontent.com/u/55999857")]
#![doc(test(attr(deny(warnings))))]

#[allow(unused_extern_crates)]
#[cfg(feature = "alloc")]
extern crate alloc;
//pub mod rusty_monitor;
use ntest::timeout;
// region: macros
/// Helper macro for easily implementing `OpAssign`.
macro_rules! __impl_assign {
    ($sym:tt $op:ident $fn:ident $target:ty : $($(#[$attr:meta])* $t:ty),+) => {$(
        #[allow(unused_qualifications)]
        $(#[$attr])*
        impl core::ops::$op<$t> for $target {
            fn $fn(&mut self, rhs: $t) {
                *self = *self $sym rhs;
            }
        }
    )+};
}

/// Implement `AddAssign` for the provided types.
macro_rules! impl_add_assign {
    ($target:ty : $($(#[$attr:meta])* $t:ty),+ $(,)?) => {
        __impl_assign!(+ AddAssign add_assign $target : $($(#[$attr])* $t),+);
    };
}

/// Implement `SubAssign` for the provided types.
macro_rules! impl_sub_assign {
    ($target:ty : $($(#[$attr:meta])* $t:ty),+ $(,)?) => {
        __impl_assign!(- SubAssign sub_assign $target : $($(#[$attr])* $t),+);
    };
}

/// Implement `MulAssign` for the provided types.
macro_rules! impl_mul_assign {
    ($target:ty : $($(#[$attr:meta])* $t:ty),+ $(,)?) => {
        __impl_assign!(* MulAssign mul_assign $target : $($(#[$attr])* $t),+);
    };
}

/// Implement `DivAssign` for the provided types.
macro_rules! impl_div_assign {
    ($target:ty : $($(#[$attr:meta])* $t:ty),+ $(,)?) => {
        __impl_assign!(/ DivAssign div_assign $target : $($(#[$attr])* $t),+);
    };
}

/// Division of integers, rounding the resulting value towards negative infinity.
macro_rules! div_floor {
    ($a:expr, $b:expr) => {{
        let _a = $a;
        let _b = $b;

        let (_quotient, _remainder) = (_a / _b, _a % _b);

        if (_remainder > 0 && _b < 0) || (_remainder < 0 && _b > 0) {
            _quotient - 1
        } else {
            _quotient
        }
    }};
}

/// Cascade an out-of-bounds value.
macro_rules! cascade {
    (@ordinal ordinal) => {};
    (@year year) => {};

    // Cascade an out-of-bounds value from "from" to "to".
    ($from:ident in $min:literal.. $max:literal => $to:tt) => {
        #[allow(unused_comparisons, unused_assignments)]
        if $from >= $max {
            $from -= $max - $min;
            $to += 1;
        } else if $from < $min {
            $from += $max - $min;
            $to -= 1;
        }
    };

    // Special case the ordinal-to-year cascade, as it has different behavior.
    ($ordinal:ident => $year:ident) => {
        // We need to actually capture the idents. Without this, macro hygiene causes errors.
        cascade!(@ordinal $ordinal);
        cascade!(@year $year);
        #[allow(unused_assignments)]
        if $ordinal > crate::util::days_in_year($year) {
            $year += 1;
            $ordinal = 1;
        } else if $ordinal == 0 {
            $year -= 1;
            $ordinal = crate::util::days_in_year($year);
        }
    };
}

/// Returns `Err(error::ComponentRange)` if the value is not in range.
macro_rules! ensure_value_in_range {
    ($value:ident in $start:expr => $end:expr) => {{
        let _start = $start;
        let _end = $end;
        #[allow(trivial_numeric_casts, unused_comparisons)]
        if $value < _start || $value > _end {
            return Err(crate::error::ComponentRange {
                name: stringify!($value),
                minimum: _start as _,
                maximum: _end as _,
                value: $value as _,
                conditional_range: false,
            });
        }
    }};

    ($value:ident conditionally in $start:expr => $end:expr) => {{
        let _start = $start;
        let _end = $end;
        #[allow(trivial_numeric_casts, unused_comparisons)]
        if $value < _start || $value > _end {
            return Err(crate::error::ComponentRange {
                name: stringify!($value),
                minimum: _start as _,
                maximum: _end as _,
                value: $value as _,
                conditional_range: true,
            });
        }
    }};
}

/// Try to unwrap an expression, returning if not possible.
///
/// This is similar to the `?` operator, but does not perform `.into()`. Because of this, it is
/// usable in `const` contexts.
macro_rules! const_try {
    ($e:expr) => {
        match $e {
            Ok(value) => value,
            Err(error) => return Err(error),
        }
    };
}

/// Try to unwrap an expression, returning if not possible.
///
/// This is similar to the `?` operator, but is usable in `const` contexts.
macro_rules! const_try_opt {
    ($e:expr) => {
        match $e {
            Some(value) => value,
            None => return None,
        }
    };
}
// endregion macros

pub mod date;
pub mod duration;
pub mod error;
pub mod ext;
#[cfg(any(feature = "formatting", feature = "parsing"))]
pub mod format_description;
#[cfg(feature = "formatting")]
pub mod formatting;
#[cfg(feature = "std")]
pub mod instant;
#[cfg(feature = "macros")]
pub mod macros;
pub mod month;
pub mod offset_date_time;
#[cfg(feature = "parsing")]
pub mod parsing;
pub mod primitive_date_time;
#[cfg(feature = "quickcheck")]
#[cfg_attr(__time_03_docs, doc(cfg(feature = "quickcheck")))]
pub mod quickcheck;
#[cfg(feature = "rand")]
#[cfg_attr(__time_03_docs, doc(cfg(feature = "rand")))]
pub mod rand;
#[cfg(feature = "serde")]
#[cfg_attr(__time_03_docs, doc(cfg(feature = "serde")))]
#[allow(missing_copy_implementations, missing_debug_implementations)]
pub mod serde;
pub mod sys;
//#[cfg(test)]
//pub mod tests;
pub mod time;
pub mod utc_offset;
pub mod util;
pub mod weekday;

pub use crate::date::Date;
pub use crate::duration::Duration;
pub use crate::error::Error;
#[cfg(feature = "std")]
pub use crate::instant::Instant;
pub use crate::month::Month;
pub use crate::offset_date_time::OffsetDateTime;
pub use crate::primitive_date_time::PrimitiveDateTime;
pub use crate::time::Time;
pub use crate::utc_offset::UtcOffset;
pub use crate::weekday::Weekday;

/// An alias for [`std::result::Result`] with a generic error from the time crate.
pub type Result<T> = core::result::Result<T, Error>;
#[cfg(test)]
mod tests_llm_16_46_llm_16_45 {
    use super::*;

use crate::*;
    use std::convert::TryFrom;
    use std::ops::DivAssign;

    #[test]
    fn test_div_assign() {
        let mut duration = Duration::new(10, 0);
        duration /= 2;
        assert_eq!(duration, Duration::new(5, 0));

        let mut duration = Duration::new(-10, 0);
        duration /= 2;
        assert_eq!(duration, Duration::new(-5, 0));

        let mut duration = Duration::new(10, 0);
        duration /= 0;
        assert_eq!(duration, Duration::new(10, 0));
    }
}#[cfg(test)]
mod tests_llm_16_47 {
    use super::*;

use crate::*;

    #[test]
    fn test_div_assign() {
        let mut duration = Duration::seconds(10);
        duration /= 2;
        assert_eq!(duration, Duration::seconds(5));

        let mut duration = Duration::seconds(10);
        duration /= 0;
        assert_eq!(duration, Duration::ZERO);
    }
}#[cfg(test)]
mod tests_llm_16_48 {
    use super::*;

use crate::*;

    #[test]
    fn test_div_assign() {
        let mut duration = Duration::seconds(10);
        duration /= 2;
        assert_eq!(duration, Duration::seconds(5));

        let mut duration = Duration::seconds(10);
        duration /= 0;
        assert_eq!(duration, Duration::seconds(0));

        let mut duration = Duration::seconds(10);
        duration /= -2;
        assert_eq!(duration, Duration::seconds(-5));
    }
}#[cfg(test)]
mod tests_llm_16_52_llm_16_51 {
    use super::*;

use crate::*;
    use std::convert::TryFrom;

    #[test]
    fn test_div_assign() {
        let mut duration = Duration::seconds(10);
        let rhs: i32 = 2;
        *(&mut duration) /= rhs;
        assert_eq!(duration, Duration::seconds(5));

        let mut duration = Duration::seconds(10);
        let rhs: i32 = 0;
        *(&mut duration) /= rhs;
        assert_eq!(duration, Duration::seconds(10));

        let mut duration = Duration::seconds(10);
        let rhs: i32 = 4;
        *(&mut duration) /= rhs;
        assert_eq!(duration, Duration::seconds(2));

        let mut duration = Duration::seconds(10);
        let rhs: i32 = -2;
        *(&mut duration) /= rhs;
        assert_eq!(duration, Duration::seconds(-5));

        let mut duration = Duration::MAX;
        let rhs: i32 = 2;
        *(&mut duration) /= rhs;
        assert_eq!(duration, Duration::MAX);

        let mut duration = Duration::MIN;
        let rhs: i32 = 2;
        *(&mut duration) /= rhs;
        assert_eq!(duration, Duration::MIN);
    }
}#[cfg(test)]
mod tests_llm_16_54 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_div_assign() {
        let mut duration = Duration::seconds(10);
        duration /= 2;
        assert_eq!(duration, Duration::seconds(5));
        
        let mut duration = Duration::seconds(5);
        duration /= -2;
        assert_eq!(duration, Duration::seconds(-2));
        
        let mut duration = Duration::seconds(10);
        duration /= 0; // division by zero
        assert_eq!(duration, Duration::seconds(10));
    }
}#[cfg(test)]
mod tests_llm_16_70 {
    use super::*;

use crate::*;

    // Test mul_assign function
    #[test]
    fn test_mul_assign() {
        let mut duration = Duration::new(10, 500_000_000);
        duration *= 2;
        assert_eq!(duration, Duration::new(21, 0));

        let mut duration = Duration::new(-10, 500_000_000);
        duration *= -2;
        assert_eq!(duration, Duration::new(21, 0));

        let mut duration = Duration::new(10, 500_000_000);
        duration *= 0;
        assert_eq!(duration, Duration::new(0, 0));

        let mut duration = Duration::new(10, 500_000_000);
        duration *= -1;
        assert_eq!(duration, Duration::new(-10, -500_000_000));

        let mut duration = Duration::MAX;
        duration *= 2;
        assert_eq!(duration, Duration::MAX);

        let mut duration = Duration::MIN;
        duration *= 2;
        assert_eq!(duration, Duration::MIN);

        let mut duration = Duration::MAX;
        duration *= -2;
        assert_eq!(duration, Duration::MIN);

        let mut duration = Duration::MIN;
        duration *= -2;
        assert_eq!(duration, Duration::MAX);
    }
}pub mod tests_llm_16_73 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_mul_assign() {
        let mut duration = Duration::seconds(5);
        duration *= 2;
        assert_eq!(duration, Duration::seconds(10));
        
        let mut duration = Duration::seconds(5);
        duration *= 0;
        assert_eq!(duration, Duration::ZERO);
    }
}#[cfg(test)]
mod tests_llm_16_75 {
    use super::*;

use crate::*;
    use crate::ext::NumericalDuration;

    #[test]
    fn test_mul_assign() {
        let mut duration = crate::Duration::new(1, 0);
        duration *= 2;
        assert_eq!(duration, crate::Duration::new(2, 0));

        let mut duration = crate::Duration::new(1, 0);
        duration *= -2;
        assert_eq!(duration, crate::Duration::new(-2, 0));

        let mut duration = crate::Duration::new(1, 0);
        duration *= 0;
        assert_eq!(duration, crate::Duration::new(0, 0));
    }
}#[cfg(test)]
mod tests_llm_16_81 {
    use super::*;

use crate::*;

    #[test]
    fn test_mul_assign() {
        let mut duration = Duration::seconds(5);
        duration *= 2;
        assert_eq!(duration, Duration::seconds(10));

        let mut duration = Duration::seconds(5);
        duration *= -2;
        assert_eq!(duration, Duration::seconds(-10));

        let mut duration = Duration::seconds(5);
        duration *= 0;
        assert_eq!(duration, Duration::seconds(0));

        let mut duration = Duration::MAX;
        duration *= 2;
        assert_eq!(duration, Duration::MAX);

        let mut duration = Duration::MIN;
        duration *= 2;
        assert_eq!(duration, Duration::MIN);

        let mut duration = Duration::MAX;
        duration *= -2;
        assert_eq!(duration, Duration::MIN);

        let mut duration = Duration::MIN;
        duration *= -2;
        assert_eq!(duration, Duration::MAX);
    }
}mod tests_llm_16_83_llm_16_82 {
    use crate::Duration;
    
    #[test]
    fn test_mul_assign() {
        let mut duration = Duration::new(2, 0);
        duration *= 3;
        assert_eq!(duration, Duration::new(6, 0));
        
        let mut duration = Duration::new(0, 500_000_000);
        duration *= 2;
        assert_eq!(duration, Duration::new(1, 0));
    }
}#[cfg(test)]
mod tests_llm_16_91_llm_16_90 {
    use super::*;

use crate::*;

    use crate::*;

    #[test]
    fn test_sub_assign() {
        let mut duration = Duration::seconds(5);
        duration -= Duration::seconds(3);
        assert_eq!(duration, Duration::seconds(2));
    }
}
#[cfg(test)]
mod tests_llm_16_157 {
    use super::*;

use crate::*;

    use crate::*;

    #[test]
    fn add_assign() {
        let mut instant = Instant::now();
        let duration = Duration::seconds(5);

        instant += duration;

        assert_eq!(instant.elapsed(), duration);
    }
}
#[cfg(test)]
mod tests_llm_16_167 {
    use super::*;

use crate::*;
    use std::cmp::Ordering;

    #[test]
    fn test_sub_assign() {
        let mut instant = Instant::now();
        let duration = Duration::seconds(2);
        instant -= duration;
        assert_eq!(instant, Instant::now() - duration);

        let mut instant = Instant::now();
        let duration = Duration::seconds(-2);
        instant -= duration;
        assert_eq!(instant, Instant::now() - duration);
    }
}#[cfg(test)]
mod tests_llm_16_348_llm_16_347 {
    use std::cmp::Ordering;
    use std::convert::TryFrom;
    use crate::ext::NumericalDuration;
    use crate::Duration;
    use crate::Instant;

    #[test]
    fn test_add_assign() {
        let mut instant = Instant::now();
        let duration = Duration::seconds(1);
        instant = instant + duration;
        assert_eq!(instant, Instant::now() + duration);
    }
}#[cfg(test)]
mod tests_rug_60 {
    use super::*;
    use std::ops::AddAssign;
    use crate::{Duration, ext::NumericalDuration};

    #[test]
    fn test_rug() {
        let mut p0 = Duration::new(5, 0);
        let p1 = std::time::Duration::new(5, 0);

        p0.add_assign(p1);
    }
}#[cfg(test)]
mod tests_rug_61 {
    use super::*;
    use crate::{Duration, ext::NumericalDuration};
    use std::time::Duration as StdDuration;

    #[test]
    fn test_rug() {
        let mut p0 = Duration::new(5, 0);
        let p1 = StdDuration::new(5, 0);
        
        <duration::Duration as std::ops::SubAssign<std::time::Duration>>::sub_assign(&mut p0, p1);
    }
}
#[cfg(test)]
mod tests_rug_70 {
    use super::*;
    use std::ops::SubAssign;
    use std::time::Instant;
    use crate::{Duration, ext::NumericalDuration};

    #[test]
    fn test_rug() {
        let mut p0 = Instant::now();
        let mut p1 = Duration::new(5, 0);
                
        <Instant>::sub_assign(&mut p0, p1);
    }
}
#[cfg(test)]
mod tests_rug_72 {
    use super::*;
    use offset_date_time::OffsetDateTime;
    use std::time::Duration;
    use std::ops::AddAssign;
    
    #[test]
    fn test_rug() {
        let mut p0 = OffsetDateTime::now_utc();
        let mut p1 = Duration::new(5, 0);
        
        <offset_date_time::OffsetDateTime>::add_assign(&mut p0, p1);
        
        // add your assertions here
    }
}#[cfg(test)]
mod tests_rug_74 {
    use super::*;
    use offset_date_time::OffsetDateTime;
    use std::ops::SubAssign;
    use std::time::Duration;
    
    #[test]
    fn test_rug() {
        let mut p0 = OffsetDateTime::now_utc();
        let mut p1 = Duration::new(5, 0);
        
        <OffsetDateTime as std::ops::SubAssign<std::time::Duration>>::sub_assign(&mut p0, p1);
    }
}#[cfg(test)]
mod tests_rug_75 {
    use super::*;
    use std::ops::AddAssign;
    use std::time::SystemTime;
    use crate::{Duration, ext::NumericalDuration};

    #[test]
    fn test_rug() {
        let mut p0: SystemTime = SystemTime::now();
        let mut p1: Duration = Duration::new(5, 0);

        <std::time::SystemTime>::add_assign(&mut p0, p1);
    }
}#[cfg(test)]
mod tests_rug_83 {
    use super::*;
    use crate::{Date, Time, Duration, ext::NumericalDuration};

    #[test]
    fn test_rug() {
        let mut p0 = Time::__from_hms_nanos_unchecked(0, 0, 0, 0);
        let mut p1 = Duration::new(5, 0);

        <time::Time as std::ops::SubAssign<duration::Duration>>::sub_assign(&mut p0, p1);
    }
}