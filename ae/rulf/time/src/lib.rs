//! [![GitHub time-rs/time](https://img.shields.io/badge/GitHub-time--rs%2Ftime-9b88bb?logo=github&style=for-the-badge)](https://github.com/time-rs/time)
//! ![license MIT or Apache-2.0](https://img.shields.io/badge/license-MIT%20or%20Apache--2.0-779a6b?style=for-the-badge)
//! [![minimum rustc 1.32.0](https://img.shields.io/badge/minimum%20rustc-1.32.0-c18170?logo=rust&style=for-the-badge)](https://www.whatrustisit.com)
//!
//! # Feature flags
//!
//! This crate exposes a number of features. These can be enabled or disabled as
//! shown [in Cargo's documentation](https://doc.rust-lang.org/cargo/reference/features.html).
//! Features are _disabled_ by default unless otherwise noted.
//!
//! Reliance on a given feature is always indicated alongside the item
//! definition.
//!
//! - `std` (_enabled by default_)
//!
//!   This enables a number of features that depend on the standard library.
//!   [`Instant`] is the primary item that requires this feature, though some
//!   others methods may rely on [`Instant`] internally.
//!
//!   This crate currently requires a global allocator be present even if this
//!   feature is disabled.
//!
//! - `serde`
//!
//!   Enables [serde](https://docs.rs/serde) support for all types.
//!
//! - `rand`
//!
//!   Enables [rand](https://docs.rs/rand) support for all types.
//!
//! - `deprecated` (_enabled by default_)
//!
//!   Allows using certain deprecated functions from time 0.1.
//!
//! - `panicking-api`
//!
//!   Non-panicking APIs are provided, and should generally be preferred.
//!   However, there are some situations where avoiding `.unwrap()` may be
//!   desired. Generally speaking, macros should be used in these situations.
//!   Library authors should avoid using this feature.
//!
//! # Formatting
//!
//! Time's formatting behavior is based on `strftime` in C, though it is
//! explicitly _not_ compatible. Specifiers may be missing, added, or have
//! different behavior than in C. As such, you should use the table below, which
//! is an up-to-date reference on what each specifier does.
//!
//! <style>
//! summary, details:not([open]) { cursor: pointer; }
//! summary { display: list-item; }
//! summary::marker { content: '▶ '; }
//! details[open] summary::marker { content: '▼ '; }
//! </style>
//!
//! <details>
//! <summary>Specifiers</summary>
//!
//! | Specifier | Replaced by                                                            | Example                    |
//! |-----------|------------------------------------------------------------------------|----------------------------|
//! | `%a`      | Abbreviated weekday name                                               | `Thu`                      |
//! | `%A`      | Full weekday name                                                      | `Thursday`                 |
//! | `%b`      | Abbreviated month name                                                 | `Aug`                      |
//! | `%B`      | Full month name                                                        | `August`                   |
//! | `%c`      | Date and time representation, equivalent to `%a %b %-d %-H:%M:%S %-Y`  | `Thu Aug 23 14:55:02 2001` |
//! | `%C`      | Year divided by 100 and truncated to integer (`00`-`99`)               | `20`                       |
//! | `%d`      | Day of the month, zero-padded (`01`-`31`)                              | `23`                       |
//! | `%D`      | Short MM/DD/YY date, equivalent to `%-m/%d/%y`                         | `8/23/01`                  |
//! | `%F`      | Short YYYY-MM-DD date, equivalent to `%-Y-%m-%d`                       | `2001-08-23`               |
//! | `%g`      | Week-based year, last two digits (`00`-`99`)                           | `01`                       |
//! | `%G`      | Week-based year                                                        | `2001`                     |
//! | `%H`      | Hour in 24h format (`00`-`23`)                                         | `14`                       |
//! | `%I`      | Hour in 12h format (`01`-`12`)                                         | `02`                       |
//! | `%j`      | Day of the year (`001`-`366`)                                          | `235`                      |
//! | `%m`      | Month as a decimal number (`01`-`12`)                                  | `08`                       |
//! | `%M`      | Minute (`00`-`59`)                                                     | `55`                       |
//! | `%N`      | Subsecond nanoseconds. Always 9 digits                                 | `012345678`                |
//! | `%p`      | `am` or `pm` designation                                               | `pm`                       |
//! | `%P`      | `AM` or `PM` designation                                               | `PM`                       |
//! | `%r`      | 12-hour clock time, equivalent to `%-I:%M:%S %p`                       | `2:55:02 pm`               |
//! | `%R`      | 24-hour HH:MM time, equivalent to `%-H:%M`                             | `14:55`                    |
//! | `%S`      | Second (`00`-`59`)                                                     | `02`                       |
//! | `%T`      | 24-hour clock time with seconds, equivalent to `%-H:%M:%S`             | `14:55:02`                 |
//! | `%u`      | ISO 8601 weekday as number with Monday as 1 (`1`-`7`)                  | `4`                        |
//! | `%U`      | Week number with the first Sunday as the start of week one (`00`-`53`) | `33`                       |
//! | `%V`      | ISO 8601 week number (`01`-`53`)                                       | `34`                       |
//! | `%w`      | Weekday as a decimal number with Sunday as 0 (`0`-`6`)                 | `4`                        |
//! | `%W`      | Week number with the first Monday as the start of week one (`00`-`53`) | `34`                       |
//! | `%y`      | Year, last two digits (`00`-`99`)                                      | `01`                       |
//! | `%Y`      | Full year, including `+` if ≥10,000                                    | `2001`                     |
//! | `%z`      | ISO 8601 offset from UTC in timezone (+HHMM)                           | `+0100`                    |
//! | `%%`      | Literal `%`                                                            | `%`                        |
//!
//! </details>
//!
//! ## Modifiers
//!
//! All specifiers that are strictly numerical have modifiers for formatting.
//! Adding a modifier to a non-supporting specifier is a no-op.
//!
//! <!-- rust-lang/rust#65613 -->
//! <style>.docblock code { white-space: pre-wrap; }</style>
//!
//! | Modifier         | Behavior        | Example       |
//! |------------------|-----------------|---------------|
//! | `-` (dash)       | No padding      | `%-d` => `5`  |
//! | `_` (underscore) | Pad with spaces | `%_d` => ` 5` |
//! | `0`              | Pad with zeros  | `%0d` => `05` |
#![cfg_attr(__time_02_docs, feature(doc_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::enum_glob_use,
    clippy::inline_always,
    clippy::missing_errors_doc,
    clippy::module_name_repetitions,
    clippy::must_use_candidate,
    clippy::redundant_pub_crate,
    clippy::suspicious_arithmetic_impl,
    clippy::suspicious_op_assign_impl,
    clippy::use_self,
    clippy::wildcard_imports,
    clippy::zero_prefixed_literal,
    unstable_name_collisions
)]
#![cfg_attr(
    test,
    allow(clippy::cognitive_complexity, clippy::similar_names, clippy::too_many_lines)
)]
#![doc(html_favicon_url = "https://avatars0.githubusercontent.com/u/55999857")]
#![doc(html_logo_url = "https://avatars0.githubusercontent.com/u/55999857")]
#![doc(test(attr(deny(warnings))))]
#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(feature = "panicking-api")]
#[cfg_attr(__time_02_docs, doc(cfg(feature = "panicking-api")))]
macro_rules! format_conditional {
    ($conditional:ident) => {
        { #[cfg(not(feature = "std"))] use alloc::format;
        format!(concat!(stringify!($conditional), "={}"), $conditional) }
    };
    ($first_conditional:ident, $($conditional:ident),*) => {
        { #[cfg(not(feature = "std"))] use alloc:: { format, string::String }; let mut s
        = String::new(); s.push_str(& format_conditional!($first_conditional)); $(s
        .push_str(& format!(concat!(", ", stringify!($conditional), "={}"),
        $conditional));)* s }
    };
}
/// Panic if the value is not in range.
#[cfg(feature = "panicking-api")]
#[cfg_attr(__time_02_docs, doc(cfg(feature = "panicking-api")))]
macro_rules! assert_value_in_range {
    ($value:ident in $start:expr => $end:expr) => {
        { #[allow(unused_imports)] use standback::prelude::*; if ! ($start ..=$end)
        .contains(&$value) { panic!(concat!(stringify!($value),
        " must be in the range {}..={} (was {})"), $start, $end, $value,); } }
    };
    ($value:ident in $start:expr => $end:expr, given $($conditional:ident),+ $(,)?) => {
        { #[allow(unused_imports)] use standback::prelude::*; if ! ($start ..=$end)
        .contains(&$value) { panic!(concat!(stringify!($value),
        " must be in the range {}..={} given{} (was {})"), $start, $end, &
        format_conditional!($($conditional),+), $value,); } }
    };
}
/// Returns `Err(error::ComponentRange)` if the value is not in range.
macro_rules! ensure_value_in_range {
    ($value:ident in $start:expr => $end:expr) => {
        { #![allow(trivial_numeric_casts, unused_comparisons)] if $value < $start ||
        $value > $end { return Err(crate ::error::ComponentRange { name :
        stringify!($value), minimum : $start as i64, maximum : $end as i64, value :
        $value as i64, conditional_range : false,
        #[cfg(not(__time_02_supports_non_exhaustive))] __non_exhaustive : (), }); } }
    };
    ($value:ident conditionally in $start:expr => $end:expr) => {
        { #![allow(trivial_numeric_casts, unused_comparisons)] if $value < $start ||
        $value > $end { return Err(crate ::error::ComponentRange { name :
        stringify!($value), minimum : $start as i64, maximum : $end as i64, value :
        $value as i64, conditional_range : true,
        #[cfg(not(__time_02_supports_non_exhaustive))] __non_exhaustive : (), }); } }
    };
}
/// Try to unwrap an expression, returning if not possible.
///
/// This is similar to the `?` operator, but does not perform `.into()`. Because
/// of this, it is usable in `const` contexts.
macro_rules! const_try {
    ($e:expr) => {
        match $e { Ok(value) => value, Err(error) => return Err(error), }
    };
}
/// The `Date` struct and its associated `impl`s.
mod date;
/// The `Duration` struct and its associated `impl`s.
mod duration;
/// Various error types returned by methods in the time crate.
pub mod error;
/// Extension traits.
pub mod ext;
mod format;
/// The `Instant` struct and its associated `impl`s.
#[cfg(feature = "std")]
mod instant;
pub mod internals;
/// The `OffsetDateTime` struct and its associated `impl`s.
mod offset_date_time;
/// The `PrimitiveDateTime` struct and its associated `impl`s.
mod primitive_date_time;
#[cfg(feature = "rand")]
mod rand;
#[cfg(feature = "serde")]
#[allow(missing_copy_implementations, missing_debug_implementations)]
pub mod serde;
/// The `Sign` struct and its associated `impl`s.
mod sign;
/// The `Time` struct and its associated `impl`s.
mod time_mod;
/// The `UtcOffset` struct and its associated `impl`s.
mod utc_offset;
pub mod util;
/// Days of the week.
mod weekday;
pub use date::Date;
pub use duration::Duration;
pub use error::Error;
#[deprecated(since = "0.2.23", note = "Errors have been moved to the `error` module.")]
pub use error::{
    ComponentRange as ComponentRangeError, ConversionRange as ConversionRangeError,
    IndeterminateOffset as IndeterminateOffsetError, Parse as ParseError,
};
#[deprecated(
    since = "0.2.23",
    note = "Extension traits have been moved to the `ext` module."
)]
pub use ext::{NumericalDuration, NumericalStdDuration, NumericalStdDurationShort};
pub(crate) use format::DeferredFormat;
pub use format::Format;
use format::ParseResult;
#[cfg(feature = "std")]
pub use instant::Instant;
#[deprecated(since = "0.2.23", note = "Macros have been moved to the `macros` module.")]
pub use macros::{date, offset, time};
pub use offset_date_time::OffsetDateTime;
pub use primitive_date_time::PrimitiveDateTime;
#[allow(deprecated)]
pub use sign::Sign;
#[allow(unused_imports)]
use standback::prelude::*;
pub use time_mod::Time;
pub use utc_offset::UtcOffset;
#[deprecated(
    since = "0.2.23",
    note = "This function has been moved to the `util` module."
)]
pub use util::{days_in_year, is_leap_year, validate_format_string, weeks_in_year};
pub use weekday::Weekday;
/// An alias for `Result` with a generic error from the time crate.
pub type Result<T> = core::result::Result<T, Error>;
/// A collection of imports that are widely useful.
///
/// Unlike the standard library, this must be explicitly imported:
///
/// ```rust,no_run
/// # #[allow(unused_imports)]
/// use time::prelude::*;
/// ```
///
/// The prelude may grow in minor releases. Any removals will only occur in
/// major releases.
pub mod prelude {
    #[cfg(not(__time_02_use_trait_as_underscore))]
    pub use crate::ext::{NumericalDuration, NumericalStdDuration};
    #[cfg(__time_02_use_trait_as_underscore)]
    pub use crate::ext::{NumericalDuration as _, NumericalStdDuration as _};
    pub use time_macros::{date, offset, time};
}
#[allow(clippy::missing_docs_in_private_items)]
mod private {
    use super::*;
    macro_rules! parsable {
        ($($type:ty),* $(,)?) => {
            $(impl Parsable for $type { fn parse(s : impl AsRef < str >, format : impl
            AsRef < str >) -> ParseResult < Self > { Self::parse(s, format) } })*
        };
    }
    pub trait Parsable: Sized {
        fn parse(s: impl AsRef<str>, format: impl AsRef<str>) -> ParseResult<Self>;
    }
    parsable![Time, Date, UtcOffset, PrimitiveDateTime, OffsetDateTime];
}
/// Parse any parsable type from the time crate.
///
/// This is identical to calling `T::parse(s, format)`, but allows the use of
/// type inference where possible.
///
/// ```rust
/// use time::Time;
///
/// #[derive(Debug)]
/// struct Foo(Time);
///
/// fn main() -> time::Result<()> {
///     // We don't need to tell the compiler what type we need!
///     let foo = Foo(time::parse("14:55:02", "%T")?);
///     println!("{:?}", foo);
///     Ok(())
/// }
/// ```
pub fn parse<T: private::Parsable>(
    s: impl AsRef<str>,
    format: impl AsRef<str>,
) -> ParseResult<T> {
    private::Parsable::parse(s, format)
}
/// Macros to statically construct values that are known to be valid.
pub mod macros {
    /// Construct a [`Date`](crate::Date) with a statically known value.
    ///
    /// The resulting expression can be used in `const` or `static` declarations.
    ///
    /// Three formats are supported: year-week-weekday, year-ordinal, and
    /// year-month-day.
    ///
    /// ```rust
    /// # use time::{Date, macros::date, Weekday::*};
    /// # fn main() -> time::Result<()> {
    /// assert_eq!(date!(2020-W01-3), Date::try_from_iso_ywd(2020, 1, Wednesday)?);
    /// assert_eq!(date!(2020-001), Date::try_from_yo(2020, 1)?);
    /// assert_eq!(date!(2020-01-01), Date::try_from_ymd(2020, 1, 1)?);
    /// # Ok(())
    /// # }
    /// ```
    pub use time_macros::date;
    /// Construct a [`UtcOffset`](crate::UtcOffset) with a statically known value.
    ///
    /// The resulting expression can be used in `const` or `static` declarations.
    ///
    /// A sign and the hour must be provided; minutes and seconds default to zero.
    /// `UTC` (both uppercase and lowercase) is also allowed.
    ///
    /// ```rust
    /// # use time::{macros::offset, UtcOffset};
    /// assert_eq!(offset!(UTC), UtcOffset::hours(0));
    /// assert_eq!(offset!(utc), UtcOffset::hours(0));
    /// assert_eq!(offset!(+0), UtcOffset::hours(0));
    /// assert_eq!(offset!(+1), UtcOffset::hours(1));
    /// assert_eq!(offset!(-1), UtcOffset::hours(-1));
    /// assert_eq!(offset!(+1:30), UtcOffset::minutes(90));
    /// assert_eq!(offset!(-1:30), UtcOffset::minutes(-90));
    /// assert_eq!(offset!(+1:30:59), UtcOffset::seconds(5459));
    /// assert_eq!(offset!(-1:30:59), UtcOffset::seconds(-5459));
    /// assert_eq!(offset!(+23:59:59), UtcOffset::seconds(86_399));
    /// assert_eq!(offset!(-23:59:59), UtcOffset::seconds(-86_399));
    /// ```
    pub use time_macros::offset;
    /// Construct a [`Time`](crate::Time) with a statically known value.
    ///
    /// The resulting expression can be used in `const` or `static` declarations.
    ///
    /// Hours and minutes must be provided, while seconds defaults to zero. AM/PM is
    /// allowed (either uppercase or lowercase). Any number of subsecond digits may
    /// be provided (though any past nine will be discarded).
    ///
    /// All components are validated at compile-time. An error will be raised if any
    /// value is invalid.
    ///
    /// ```rust
    /// # use time::{Time, macros::time};
    /// # fn main() -> time::Result<()> {
    /// assert_eq!(time!(0:00), Time::try_from_hms(0, 0, 0)?);
    /// assert_eq!(time!(1:02:03), Time::try_from_hms(1, 2, 3)?);
    /// assert_eq!(time!(1:02:03.004_005_006), Time::try_from_hms_nano(1, 2, 3, 4_005_006)?);
    /// assert_eq!(time!(12:00 am), Time::try_from_hms(0, 0, 0)?);
    /// assert_eq!(time!(1:02:03 am), Time::try_from_hms(1, 2, 3)?);
    /// assert_eq!(time!(1:02:03.004_005_006 am), Time::try_from_hms_nano(1, 2, 3, 4_005_006)?);
    /// assert_eq!(time!(12:00 pm), Time::try_from_hms(12, 0, 0)?);
    /// assert_eq!(time!(1:02:03 pm), Time::try_from_hms(13, 2, 3)?);
    /// assert_eq!(time!(1:02:03.004_005_006 pm), Time::try_from_hms_nano(13, 2, 3, 4_005_006)?);
    /// # Ok(())
    /// # }
    /// ```
    pub use time_macros::time;
}
#[cfg(all(feature = "std", feature = "deprecated"))]
#[allow(clippy::missing_docs_in_private_items)]
#[deprecated(since = "0.2.0", note = "Use `Instant`")]
pub type PreciseTime = Instant;
#[cfg(all(feature = "std", feature = "deprecated"))]
#[allow(clippy::missing_docs_in_private_items)]
#[deprecated(since = "0.2.0", note = "Use `Instant`")]
pub type SteadyTime = Instant;
#[cfg(all(feature = "std", feature = "deprecated"))]
#[allow(clippy::missing_docs_in_private_items)]
#[deprecated(
    since = "0.2.0",
    note = "Use `OffsetDateTime::now() - OffsetDateTime::unix_epoch()` to get a `Duration` since \
            a known epoch."
)]
pub fn precise_time_ns() -> u64 {
    use standback::convert::TryInto;
    use std::time::SystemTime;
    (SystemTime::now().duration_since(SystemTime::UNIX_EPOCH))
        .expect("System clock was before 1970.")
        .as_nanos()
        .try_into()
        .expect("This function will be removed long before this is an issue.")
}
#[cfg(all(feature = "std", feature = "deprecated"))]
#[allow(clippy::missing_docs_in_private_items)]
#[deprecated(
    since = "0.2.0",
    note = "Use `OffsetDateTime::now() - OffsetDateTime::unix_epoch()` to get a `Duration` since \
            a known epoch."
)]
pub fn precise_time_s() -> f64 {
    use std::time::SystemTime;
    (SystemTime::now().duration_since(SystemTime::UNIX_EPOCH))
        .expect("System clock was before 1970.")
        .as_secs_f64()
}
#[cfg(test)]
mod tests_llm_16_17 {
    use super::*;
    use crate::*;
    use crate::date::Date;
    use crate::private::Parsable;
    #[test]
    fn test_parse() {
        let _rug_st_tests_llm_16_17_rrrruuuugggg_test_parse = 0;
        let rug_fuzz_0 = "2022-01-01";
        let rug_fuzz_1 = "%Y-%m-%d";
        let s = rug_fuzz_0;
        let format = rug_fuzz_1;
        let result = Date::parse(s, format);
        debug_assert_eq!(result.is_ok(), true);
        let _rug_ed_tests_llm_16_17_rrrruuuugggg_test_parse = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_462 {
    use super::*;
    use crate::*;
    #[test]
    fn test_parse() {
        let _rug_st_tests_llm_16_462_rrrruuuugggg_test_parse = 0;
        let rug_fuzz_0 = "2022-01-01 12:00:00";
        let rug_fuzz_1 = "%Y-%m-%d %H:%M:%S";
        let s = rug_fuzz_0;
        let format = rug_fuzz_1;
        let result = <time_mod::Time as private::Parsable>::parse(s, format);
        debug_assert!(result.is_ok());
        let _rug_ed_tests_llm_16_462_rrrruuuugggg_test_parse = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_939 {
    use crate::precise_time_s;
    #[test]
    fn test_precise_time_s() {
        let _rug_st_tests_llm_16_939_rrrruuuugggg_test_precise_time_s = 0;
        let rug_fuzz_0 = 0.0;
        let now = precise_time_s();
        debug_assert!(
            now >= rug_fuzz_0, "Returned time should be greater than or equal to 0.0"
        );
        let _rug_ed_tests_llm_16_939_rrrruuuugggg_test_precise_time_s = 0;
    }
}
#[cfg(test)]
mod tests_rug_103 {
    use super::*;
    use crate::Date;
    #[test]
    fn test_parse() {
        let _rug_st_tests_rug_103_rrrruuuugggg_test_parse = 0;
        let rug_fuzz_0 = "14:55:02";
        let rug_fuzz_1 = "%T";
        let p0: &str = rug_fuzz_0;
        let p1: &str = rug_fuzz_1;
        parse::<Date>(p0, p1);
        let _rug_ed_tests_rug_103_rrrruuuugggg_test_parse = 0;
    }
}
#[cfg(test)]
mod tests_rug_104 {
    use super::*;
    use standback::convert::TryInto;
    use std::time::SystemTime;
    #[test]
    fn test_precise_time_ns() {
        let _rug_st_tests_rug_104_rrrruuuugggg_test_precise_time_ns = 0;
        let rug_fuzz_0 = "System clock was before 1970.";
        let rug_fuzz_1 = "This function will be removed long before this is an issue.";
        let now = SystemTime::now();
        let unix_epoch = SystemTime::UNIX_EPOCH;
        let duration = now.duration_since(unix_epoch).expect(rug_fuzz_0);
        let nanos = duration.as_nanos();
        let result: u64 = nanos.try_into().expect(rug_fuzz_1);
        debug_assert_eq!(result, precise_time_ns());
        let _rug_ed_tests_rug_104_rrrruuuugggg_test_precise_time_ns = 0;
    }
}
#[cfg(test)]
mod tests_rug_106 {
    use super::*;
    use crate::private::Parsable;
    use std::rc::Rc;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_106_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "input_string";
        let rug_fuzz_1 = "format_string";
        let p0: Rc<str> = Rc::from(rug_fuzz_0);
        let p1: Rc<str> = Rc::from(rug_fuzz_1);
        <primitive_date_time::PrimitiveDateTime as private::Parsable>::parse(p0, p1);
        let _rug_ed_tests_rug_106_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_107 {
    use super::*;
    use crate::private::Parsable;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_107_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "input_string";
        let rug_fuzz_1 = "format_string";
        let mut p0: Box<str> = Box::from(rug_fuzz_0);
        let mut p1: Box<str> = Box::from(rug_fuzz_1);
        <offset_date_time::OffsetDateTime as private::Parsable>::parse(p0, p1);
        let _rug_ed_tests_rug_107_rrrruuuugggg_test_rug = 0;
    }
}
