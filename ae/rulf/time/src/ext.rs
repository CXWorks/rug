#![allow(trivial_numeric_casts)]
use crate::Duration;
use core::time::Duration as StdDuration;
/// Create `Duration`s from primitive and core numeric types.
///
/// This trait can be imported with `use time::prelude::*`.
///
/// Due to limitations in rustc, these methods are currently _not_ `const fn`.
/// See [RFC 2632](https://github.com/rust-lang/rfcs/pull/2632) for details.
///
/// # Examples
///
/// Basic construction of `Duration`s.
///
/// ```rust
/// # use time::{Duration, ext::NumericalDuration};
/// assert_eq!(5.nanoseconds(), Duration::nanoseconds(5));
/// assert_eq!(5.microseconds(), Duration::microseconds(5));
/// assert_eq!(5.milliseconds(), Duration::milliseconds(5));
/// assert_eq!(5.seconds(), Duration::seconds(5));
/// assert_eq!(5.minutes(), Duration::minutes(5));
/// assert_eq!(5.hours(), Duration::hours(5));
/// assert_eq!(5.days(), Duration::days(5));
/// assert_eq!(5.weeks(), Duration::weeks(5));
/// ```
///
/// Signed integers work as well!
///
/// ```rust
/// # use time::{Duration, ext::NumericalDuration};
/// assert_eq!((-5).nanoseconds(), Duration::nanoseconds(-5));
/// assert_eq!((-5).microseconds(), Duration::microseconds(-5));
/// assert_eq!((-5).milliseconds(), Duration::milliseconds(-5));
/// assert_eq!((-5).seconds(), Duration::seconds(-5));
/// assert_eq!((-5).minutes(), Duration::minutes(-5));
/// assert_eq!((-5).hours(), Duration::hours(-5));
/// assert_eq!((-5).days(), Duration::days(-5));
/// assert_eq!((-5).weeks(), Duration::weeks(-5));
/// ```
///
/// Just like any other `Duration`, they can be added, subtracted, etc.
///
/// ```rust
/// # use time::ext::NumericalDuration;
/// assert_eq!(2.seconds() + 500.milliseconds(), 2_500.milliseconds());
/// assert_eq!(2.seconds() - 500.milliseconds(), 1_500.milliseconds());
/// ```
///
/// When called on floating point values, any remainder of the floating point
/// value will be truncated. Keep in mind that floating point numbers are
/// inherently imprecise and have limited capacity.
pub trait NumericalDuration {
    /// Create a `Duration` from the number of nanoseconds.
    fn nanoseconds(self) -> Duration;
    /// Create a `Duration` from the number of microseconds.
    fn microseconds(self) -> Duration;
    /// Create a `Duration` from the number of milliseconds.
    fn milliseconds(self) -> Duration;
    /// Create a `Duration` from the number of seconds.
    fn seconds(self) -> Duration;
    /// Create a `Duration` from the number of minutes.
    fn minutes(self) -> Duration;
    /// Create a `Duration` from the number of hours.
    fn hours(self) -> Duration;
    /// Create a `Duration` from the number of days.
    fn days(self) -> Duration;
    /// Create a `Duration` from the number of weeks.
    fn weeks(self) -> Duration;
}
macro_rules! impl_numerical_duration {
    ($($type:ty),* $(,)?) => {
        $(impl NumericalDuration for $type { fn nanoseconds(self) -> Duration {
        Duration::nanoseconds(self as i64) } fn microseconds(self) -> Duration {
        Duration::microseconds(self as i64) } fn milliseconds(self) -> Duration {
        Duration::milliseconds(self as i64) } fn seconds(self) -> Duration {
        Duration::seconds(self as i64) } fn minutes(self) -> Duration {
        Duration::minutes(self as i64) } fn hours(self) -> Duration {
        Duration::hours(self as i64) } fn days(self) -> Duration { Duration::days(self as
        i64) } fn weeks(self) -> Duration { Duration::weeks(self as i64) } })*
    };
}
macro_rules! impl_numerical_duration_nonzero {
    ($($type:ty),* $(,)?) => {
        $(impl NumericalDuration for $type { fn nanoseconds(self) -> Duration {
        Duration::nanoseconds(self.get() as i64) } fn microseconds(self) -> Duration {
        Duration::microseconds(self.get() as i64) } fn milliseconds(self) -> Duration {
        Duration::milliseconds(self.get() as i64) } fn seconds(self) -> Duration {
        Duration::seconds(self.get() as i64) } fn minutes(self) -> Duration {
        Duration::minutes(self.get() as i64) } fn hours(self) -> Duration {
        Duration::hours(self.get() as i64) } fn days(self) -> Duration {
        Duration::days(self.get() as i64) } fn weeks(self) -> Duration {
        Duration::weeks(self.get() as i64) } })*
    };
}
macro_rules! impl_numerical_duration_float {
    ($($type:ty),* $(,)?) => {
        $(impl NumericalDuration for $type { fn nanoseconds(self) -> Duration {
        Duration::nanoseconds(self as i64) } fn microseconds(self) -> Duration {
        Duration::nanoseconds((self * 1_000.) as i64) } fn milliseconds(self) -> Duration
        { Duration::nanoseconds((self * 1_000_000.) as i64) } fn seconds(self) ->
        Duration { Duration::nanoseconds((self * 1_000_000_000.) as i64) } fn
        minutes(self) -> Duration { Duration::nanoseconds((self * 60_000_000_000.) as
        i64) } fn hours(self) -> Duration { Duration::nanoseconds((self *
        3_600_000_000_000.) as i64) } fn days(self) -> Duration {
        Duration::nanoseconds((self * 86_400_000_000_000.) as i64) } fn weeks(self) ->
        Duration { Duration::nanoseconds((self * 604_800_000_000_000.) as i64) } })*
    };
}
impl_numerical_duration![u8, u16, u32, i8, i16, i32, i64];
impl_numerical_duration_nonzero![
    core::num::NonZeroU8, core::num::NonZeroU16, core::num::NonZeroU32,
];
#[cfg(__time_02_nonzero_signed)]
impl_numerical_duration_nonzero![
    core::num::NonZeroI8, core::num::NonZeroI16, core::num::NonZeroI32,
    core::num::NonZeroI64,
];
impl_numerical_duration_float![f32, f64];
/// Create `std::time::Duration`s from primitive and core numeric types.
///
/// This trait can be imported (alongside others) with `use time::prelude::*`.
///
/// Due to limitations in rustc, these methods are currently _not_ `const fn`.
/// See [RFC 2632](https://github.com/rust-lang/rfcs/pull/2632) for details.
///
/// # Examples
///
/// Basic construction of `std::time::Duration`s.
///
/// ```rust
/// # use time::ext::NumericalStdDuration;
/// # use core::time::Duration;
/// assert_eq!(5.std_nanoseconds(), Duration::from_nanos(5));
/// assert_eq!(5.std_microseconds(), Duration::from_micros(5));
/// assert_eq!(5.std_milliseconds(), Duration::from_millis(5));
/// assert_eq!(5.std_seconds(), Duration::from_secs(5));
/// assert_eq!(5.std_minutes(), Duration::from_secs(5 * 60));
/// assert_eq!(5.std_hours(), Duration::from_secs(5 * 3_600));
/// assert_eq!(5.std_days(), Duration::from_secs(5 * 86_400));
/// assert_eq!(5.std_weeks(), Duration::from_secs(5 * 604_800));
/// ```
///
/// Just like any other `std::time::Duration`, they can be added, subtracted,
/// etc.
///
/// ```rust
/// # use time::ext::NumericalStdDuration;
/// assert_eq!(
///     2.std_seconds() + 500.std_milliseconds(),
///     2_500.std_milliseconds()
/// );
/// assert_eq!(
///     2.std_seconds() - 500.std_milliseconds(),
///     1_500.std_milliseconds()
/// );
/// ```
///
/// When called on floating point values, any remainder of the floating point
/// value will be truncated. Keep in mind that floating point numbers are
/// inherently imprecise and have limited capacity.
pub trait NumericalStdDuration {
    /// Create a `std::time::Duration` from the number of nanoseconds.
    fn std_nanoseconds(self) -> StdDuration;
    /// Create a `std::time::Duration` from the number of microseconds.
    fn std_microseconds(self) -> StdDuration;
    /// Create a `std::time::Duration` from the number of milliseconds.
    fn std_milliseconds(self) -> StdDuration;
    /// Create a `std::time::Duration` from the number of seconds.
    fn std_seconds(self) -> StdDuration;
    /// Create a `std::time::Duration` from the number of minutes.
    fn std_minutes(self) -> StdDuration;
    /// Create a `std::time::Duration` from the number of hours.
    fn std_hours(self) -> StdDuration;
    /// Create a `std::time::Duration` from the number of days.
    fn std_days(self) -> StdDuration;
    /// Create a `std::time::Duration` from the number of weeks.
    fn std_weeks(self) -> StdDuration;
}
macro_rules! impl_numerical_std_duration {
    ($($type:ty),* $(,)?) => {
        $(impl NumericalStdDuration for $type { fn std_nanoseconds(self) -> StdDuration {
        StdDuration::from_nanos(self as u64) } fn std_microseconds(self) -> StdDuration {
        StdDuration::from_micros(self as u64) } fn std_milliseconds(self) -> StdDuration
        { StdDuration::from_millis(self as u64) } fn std_seconds(self) -> StdDuration {
        StdDuration::from_secs(self as u64) } fn std_minutes(self) -> StdDuration {
        StdDuration::from_secs(self as u64 * 60) } fn std_hours(self) -> StdDuration {
        StdDuration::from_secs(self as u64 * 3_600) } fn std_days(self) -> StdDuration {
        StdDuration::from_secs(self as u64 * 86_400) } fn std_weeks(self) -> StdDuration
        { StdDuration::from_secs(self as u64 * 604_800) } })*
    };
}
macro_rules! impl_numerical_std_duration_nonzero {
    ($($type:ty),* $(,)?) => {
        $(impl NumericalStdDuration for $type { fn std_nanoseconds(self) -> StdDuration {
        StdDuration::from_nanos(self.get() as u64) } fn std_microseconds(self) ->
        StdDuration { StdDuration::from_micros(self.get() as u64) } fn
        std_milliseconds(self) -> StdDuration { StdDuration::from_millis(self.get() as
        u64) } fn std_seconds(self) -> StdDuration { StdDuration::from_secs(self.get() as
        u64) } fn std_minutes(self) -> StdDuration { StdDuration::from_secs(self.get() as
        u64 * 60) } fn std_hours(self) -> StdDuration { StdDuration::from_secs(self.get()
        as u64 * 3_600) } fn std_days(self) -> StdDuration { StdDuration::from_secs(self
        .get() as u64 * 86_400) } fn std_weeks(self) -> StdDuration {
        StdDuration::from_secs(self.get() as u64 * 604_800) } })*
    };
}
impl_numerical_std_duration![u8, u16, u32, u64];
impl_numerical_std_duration_nonzero![
    core::num::NonZeroU8, core::num::NonZeroU16, core::num::NonZeroU32,
    core::num::NonZeroU64,
];
/// Implement on `i32` because that's the default type for integers. This
/// performs a runtime check and panics if the value is negative.
impl NumericalStdDuration for i32 {
    fn std_nanoseconds(self) -> StdDuration {
        assert!(self >= 0);
        StdDuration::from_nanos(self as u64)
    }
    fn std_microseconds(self) -> StdDuration {
        assert!(self >= 0);
        StdDuration::from_micros(self as u64)
    }
    fn std_milliseconds(self) -> StdDuration {
        assert!(self >= 0);
        StdDuration::from_millis(self as u64)
    }
    fn std_seconds(self) -> StdDuration {
        assert!(self >= 0);
        StdDuration::from_secs(self as u64)
    }
    fn std_minutes(self) -> StdDuration {
        assert!(self >= 0);
        StdDuration::from_secs(self as u64 * 60)
    }
    fn std_hours(self) -> StdDuration {
        assert!(self >= 0);
        StdDuration::from_secs(self as u64 * 3_600)
    }
    fn std_days(self) -> StdDuration {
        assert!(self >= 0);
        StdDuration::from_secs(self as u64 * 86_400)
    }
    fn std_weeks(self) -> StdDuration {
        assert!(self >= 0);
        StdDuration::from_secs(self as u64 * 604_800)
    }
}
/// Implement on `f64` because that's the default type for floats. This performs
/// a runtime check and panics if the value is negative.
impl NumericalStdDuration for f64 {
    fn std_nanoseconds(self) -> StdDuration {
        assert!(self >= 0.);
        StdDuration::from_nanos(self as u64)
    }
    fn std_microseconds(self) -> StdDuration {
        assert!(self >= 0.);
        StdDuration::from_nanos((self * 1_000.) as u64)
    }
    fn std_milliseconds(self) -> StdDuration {
        assert!(self >= 0.);
        StdDuration::from_nanos((self * 1_000_000.) as u64)
    }
    fn std_seconds(self) -> StdDuration {
        assert!(self >= 0.);
        StdDuration::from_nanos((self * 1_000_000_000.) as u64)
    }
    fn std_minutes(self) -> StdDuration {
        assert!(self >= 0.);
        StdDuration::from_nanos((self * 60_000_000_000.) as u64)
    }
    fn std_hours(self) -> StdDuration {
        assert!(self >= 0.);
        StdDuration::from_nanos((self * 3_600_000_000_000.) as u64)
    }
    fn std_days(self) -> StdDuration {
        assert!(self >= 0.);
        StdDuration::from_nanos((self * 86_400_000_000_000.) as u64)
    }
    fn std_weeks(self) -> StdDuration {
        assert!(self >= 0.);
        StdDuration::from_nanos((self * 604_800_000_000_000.) as u64)
    }
}
/// Create `std::time::Duration`s from primitive and core numeric types. Unless
/// you are always expecting a `std::time::Duration`, you should prefer to use
/// [`NumericalStdDuration`] for clarity.
///
/// Due to limitations in rustc, these methods are currently _not_ `const fn`.
/// See [this RFC](https://github.com/rust-lang/rfcs/pull/2632) for details.
///
/// # Examples
///
/// Basic construction of `std::time::Duration`s.
///
/// ```rust
/// # use time::ext::NumericalStdDurationShort;
/// # use core::time::Duration;
/// assert_eq!(5.nanoseconds(), Duration::from_nanos(5));
/// assert_eq!(5.microseconds(), Duration::from_micros(5));
/// assert_eq!(5.milliseconds(), Duration::from_millis(5));
/// assert_eq!(5.seconds(), Duration::from_secs(5));
/// assert_eq!(5.minutes(), Duration::from_secs(5 * 60));
/// assert_eq!(5.hours(), Duration::from_secs(5 * 3_600));
/// assert_eq!(5.days(), Duration::from_secs(5 * 86_400));
/// assert_eq!(5.weeks(), Duration::from_secs(5 * 604_800));
/// ```
///
/// Just like any other `std::time::Duration`, they can be added, subtracted,
/// etc.
///
/// ```rust
/// # use time::ext::NumericalStdDurationShort;
/// assert_eq!(2.seconds() + 500.milliseconds(), 2_500.milliseconds());
/// assert_eq!(2.seconds() - 500.milliseconds(), 1_500.milliseconds());
/// ```
///
/// When called on floating point values, any remainder of the floating point
/// value will be truncated. Keep in mind that floating point numbers are
/// inherently imprecise and have limited capacity.
pub trait NumericalStdDurationShort {
    /// Create a `std::time::Duration` from the number of nanoseconds.
    fn nanoseconds(self) -> StdDuration;
    /// Create a `std::time::Duration` from the number of microseconds.
    fn microseconds(self) -> StdDuration;
    /// Create a `std::time::Duration` from the number of milliseconds.
    fn milliseconds(self) -> StdDuration;
    /// Create a `std::time::Duration` from the number of seconds.
    fn seconds(self) -> StdDuration;
    /// Create a `std::time::Duration` from the number of minutes.
    fn minutes(self) -> StdDuration;
    /// Create a `std::time::Duration` from the number of hours.
    fn hours(self) -> StdDuration;
    /// Create a `std::time::Duration` from the number of days.
    fn days(self) -> StdDuration;
    /// Create a `std::time::Duration` from the number of weeks.
    fn weeks(self) -> StdDuration;
}
impl<T: NumericalStdDuration> NumericalStdDurationShort for T {
    fn nanoseconds(self) -> StdDuration {
        <Self as NumericalStdDuration>::std_nanoseconds(self)
    }
    fn microseconds(self) -> StdDuration {
        <Self as NumericalStdDuration>::std_microseconds(self)
    }
    fn milliseconds(self) -> StdDuration {
        <Self as NumericalStdDuration>::std_milliseconds(self)
    }
    fn seconds(self) -> StdDuration {
        <Self as NumericalStdDuration>::std_seconds(self)
    }
    fn minutes(self) -> StdDuration {
        <Self as NumericalStdDuration>::std_minutes(self)
    }
    fn hours(self) -> StdDuration {
        <Self as NumericalStdDuration>::std_hours(self)
    }
    fn days(self) -> StdDuration {
        <Self as NumericalStdDuration>::std_days(self)
    }
    fn weeks(self) -> StdDuration {
        <Self as NumericalStdDuration>::std_weeks(self)
    }
}
#[cfg(test)]
mod tests_llm_16_130 {
    use super::*;
    use crate::*;
    #[test]
    fn test_days() {
        let _rug_st_tests_llm_16_130_rrrruuuugggg_test_days = 0;
        let rug_fuzz_0 = 1.5;
        let rug_fuzz_1 = 0.5;
        let rug_fuzz_2 = 0.25;
        let rug_fuzz_3 = 1.5;
        let rug_fuzz_4 = 0.5;
        let rug_fuzz_5 = 0.25;
        debug_assert_eq!(
            < f32 as super::NumericalDuration > ::days(rug_fuzz_0),
            Duration::nanoseconds(1_296_000_000_000)
        );
        debug_assert_eq!(
            < f32 as super::NumericalDuration > ::days(rug_fuzz_1),
            Duration::nanoseconds(432_000_000_000)
        );
        debug_assert_eq!(
            < f32 as super::NumericalDuration > ::days(rug_fuzz_2),
            Duration::nanoseconds(216_000_000_000)
        );
        debug_assert_eq!(
            < f32 as super::NumericalDuration > ::days(- rug_fuzz_3),
            Duration::nanoseconds(- 1_296_000_000_000)
        );
        debug_assert_eq!(
            < f32 as super::NumericalDuration > ::days(- rug_fuzz_4),
            Duration::nanoseconds(- 432_000_000_000)
        );
        debug_assert_eq!(
            < f32 as super::NumericalDuration > ::days(- rug_fuzz_5),
            Duration::nanoseconds(- 216_000_000_000)
        );
        let _rug_ed_tests_llm_16_130_rrrruuuugggg_test_days = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_135 {
    use super::*;
    use crate::*;
    #[test]
    fn test_milliseconds() {
        let _rug_st_tests_llm_16_135_rrrruuuugggg_test_milliseconds = 0;
        let rug_fuzz_0 = 500;
        let rug_fuzz_1 = 500_000_000;
        let duration = Duration::milliseconds(rug_fuzz_0);
        let expected = Duration::nanoseconds(rug_fuzz_1);
        debug_assert_eq!(duration, expected);
        let _rug_ed_tests_llm_16_135_rrrruuuugggg_test_milliseconds = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_140 {
    use crate::ext::NumericalDuration;
    use crate::duration::Duration;
    #[test]
    fn test_seconds() {
        let _rug_st_tests_llm_16_140_rrrruuuugggg_test_seconds = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 5_000_000_000;
        let duration = Duration::seconds(rug_fuzz_0);
        let expected = Duration::nanoseconds(rug_fuzz_1);
        debug_assert_eq!(duration, expected);
        let _rug_ed_tests_llm_16_140_rrrruuuugggg_test_seconds = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_144_llm_16_143 {
    use super::*;
    use crate::*;
    use crate::ext::NumericalStdDurationShort;
    #[test]
    fn test_days() {
        let _rug_st_tests_llm_16_144_llm_16_143_rrrruuuugggg_test_days = 0;
        let rug_fuzz_0 = 1_f64;
        let rug_fuzz_1 = 0_f64;
        let rug_fuzz_2 = 0.5;
        let rug_fuzz_3 = 1_f64;
        debug_assert_eq!(
            ext::NumericalStdDurationShort::days(rug_fuzz_0),
            Duration::nanoseconds(86_400_000_000_000)
        );
        debug_assert_eq!(
            ext::NumericalStdDurationShort::days(rug_fuzz_1), Duration::nanoseconds(0)
        );
        debug_assert_eq!(
            ext::NumericalStdDurationShort::days(rug_fuzz_2),
            Duration::nanoseconds(43_200_000_000_000)
        );
        debug_assert_eq!(
            ext::NumericalStdDurationShort::days((- rug_fuzz_3)), Duration::nanoseconds(-
            86_400_000_000_000)
        );
        let _rug_ed_tests_llm_16_144_llm_16_143_rrrruuuugggg_test_days = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_145 {
    use crate::Duration;
    #[test]
    fn test_hours() {
        let _rug_st_tests_llm_16_145_rrrruuuugggg_test_hours = 0;
        let rug_fuzz_0 = 2;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 0;
        let duration = Duration::hours(rug_fuzz_0);
        debug_assert_eq!(duration, Duration::nanoseconds(7_200_000_000_000));
        let duration = Duration::hours(-rug_fuzz_1);
        debug_assert_eq!(duration, Duration::nanoseconds(- 7_200_000_000_000));
        let duration = Duration::hours(rug_fuzz_2);
        debug_assert_eq!(duration, Duration::nanoseconds(0));
        let _rug_ed_tests_llm_16_145_rrrruuuugggg_test_hours = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_153_llm_16_152 {
    use super::*;
    use crate::*;
    use ext::NumericalDuration;
    use ext::NumericalStdDurationShort;
    #[test]
    fn test_nanoseconds() {
        let _rug_st_tests_llm_16_153_llm_16_152_rrrruuuugggg_test_nanoseconds = 0;
        let rug_fuzz_0 = 10.5;
        let duration = NumericalDuration::nanoseconds(rug_fuzz_0);
        debug_assert_eq!(duration.whole_nanoseconds(), 10_500_000_000);
        let _rug_ed_tests_llm_16_153_llm_16_152_rrrruuuugggg_test_nanoseconds = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_157 {
    use crate::ext::NumericalDuration;
    use crate::duration::Duration;
    #[test]
    fn test_weeks() {
        let _rug_st_tests_llm_16_157_rrrruuuugggg_test_weeks = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 1_000_000_000;
        let duration = rug_fuzz_0.weeks();
        debug_assert_eq!(duration.whole_seconds(), 604800);
        debug_assert_eq!(duration.whole_nanoseconds() % rug_fuzz_1, 0);
        let _rug_ed_tests_llm_16_157_rrrruuuugggg_test_weeks = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_183 {
    use crate::ext::NumericalDuration;
    use crate::duration::Duration;
    #[test]
    fn test_microseconds() {
        let _rug_st_tests_llm_16_183_rrrruuuugggg_test_microseconds = 0;
        let rug_fuzz_0 = 100;
        let rug_fuzz_1 = 100;
        let duration = Duration::microseconds(rug_fuzz_0);
        debug_assert_eq!(duration.whole_seconds(), 0);
        debug_assert_eq!(duration.whole_milliseconds(), 100);
        debug_assert_eq!(duration.whole_microseconds(), 100);
        debug_assert_eq!(duration.whole_nanoseconds(), 100_000);
        let duration = Duration::microseconds(-rug_fuzz_1);
        debug_assert_eq!(duration.whole_seconds(), 0);
        debug_assert_eq!(duration.whole_milliseconds(), - 100);
        debug_assert_eq!(duration.whole_microseconds(), - 100);
        debug_assert_eq!(duration.whole_nanoseconds(), - 100_000);
        let _rug_ed_tests_llm_16_183_rrrruuuugggg_test_microseconds = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_187_llm_16_186 {
    use super::*;
    use crate::*;
    use crate::ext::NumericalDuration;
    #[test]
    fn test_minutes() {
        let _rug_st_tests_llm_16_187_llm_16_186_rrrruuuugggg_test_minutes = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 300;
        let duration = <i16 as NumericalDuration>::minutes(rug_fuzz_0);
        let expected = Duration::seconds(rug_fuzz_1);
        debug_assert_eq!(duration, expected);
        let _rug_ed_tests_llm_16_187_llm_16_186_rrrruuuugggg_test_minutes = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_194 {
    use super::*;
    use crate::*;
    #[test]
    fn test_days() {
        let _rug_st_tests_llm_16_194_rrrruuuugggg_test_days = 0;
        let rug_fuzz_0 = 7;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 7;
        let duration = Duration::days(rug_fuzz_0);
        debug_assert_eq!(duration.whole_days(), 7);
        let duration = Duration::days(rug_fuzz_1);
        debug_assert_eq!(duration.whole_days(), 0);
        let duration = Duration::days(-rug_fuzz_2);
        debug_assert_eq!(duration.whole_days(), - 7);
        let _rug_ed_tests_llm_16_194_rrrruuuugggg_test_days = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_196_llm_16_195 {
    use super::*;
    use crate::*;
    use std::convert::TryInto;
    #[test]
    fn test_hours() {
        let _rug_st_tests_llm_16_196_llm_16_195_rrrruuuugggg_test_hours = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 1;
        let duration = NumericalDuration::hours(rug_fuzz_0);
        debug_assert_eq!(duration, Duration::new(1, 0));
        let negative_duration = NumericalDuration::hours(-rug_fuzz_1);
        debug_assert_eq!(negative_duration, Duration::new(- 1, 0));
        let _rug_ed_tests_llm_16_196_llm_16_195_rrrruuuugggg_test_hours = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_225 {
    use std::convert::TryFrom;
    use crate::ext;
    #[test]
    fn test_days() {
        let _rug_st_tests_llm_16_225_rrrruuuugggg_test_days = 0;
        let rug_fuzz_0 = 5;
        let duration = ext::Duration::days(rug_fuzz_0);
        debug_assert_eq!(duration.whole_days(), 5);
        let _rug_ed_tests_llm_16_225_rrrruuuugggg_test_days = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_227 {
    use super::*;
    use crate::*;
    use crate::{Duration, NumericalDuration, NumericalStdDurationShort};
    #[test]
    fn test_hours() {
        let _rug_st_tests_llm_16_227_rrrruuuugggg_test_hours = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 1;
        debug_assert_eq!(Duration::hours(rug_fuzz_0), NumericalDuration::seconds(3_600));
        debug_assert_eq!(
            Duration::hours(- rug_fuzz_1), NumericalDuration::seconds(- 3_600)
        );
        let _rug_ed_tests_llm_16_227_rrrruuuugggg_test_hours = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_238 {
    use super::*;
    use crate::*;
    use duration::*;
    #[test]
    fn test_weeks() {
        let _rug_st_tests_llm_16_238_rrrruuuugggg_test_weeks = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 1;
        debug_assert_eq!(
            < i64 as NumericalDuration > ::weeks(rug_fuzz_0), Duration::weeks(1)
        );
        debug_assert_eq!(
            < i64 as NumericalDuration > ::weeks(rug_fuzz_1), Duration::weeks(0)
        );
        debug_assert_eq!(
            < i64 as NumericalDuration > ::weeks(- rug_fuzz_2), Duration::weeks(- 1)
        );
        let _rug_ed_tests_llm_16_238_rrrruuuugggg_test_weeks = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_240_llm_16_239 {
    use super::*;
    use crate::*;
    #[test]
    fn test_days() {
        let _rug_st_tests_llm_16_240_llm_16_239_rrrruuuugggg_test_days = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 5;
        let rug_fuzz_2 = 0;
        debug_assert_eq!(Duration::days(rug_fuzz_0).whole_seconds(), 5 * 24 * 60 * 60);
        debug_assert_eq!(
            Duration::days(- rug_fuzz_1).whole_seconds(), - 5 * 24 * 60 * 60
        );
        debug_assert_eq!(Duration::days(rug_fuzz_2).whole_seconds(), 0);
        let _rug_ed_tests_llm_16_240_llm_16_239_rrrruuuugggg_test_days = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_246_llm_16_245 {
    use super::*;
    use crate::*;
    #[test]
    fn test_milliseconds() {
        let _rug_st_tests_llm_16_246_llm_16_245_rrrruuuugggg_test_milliseconds = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 1000;
        let rug_fuzz_4 = 1000;
        debug_assert_eq!(
            < i16 as ext::NumericalDuration > ::milliseconds(rug_fuzz_0),
            Duration::zero()
        );
        debug_assert_eq!(
            < i16 as ext::NumericalDuration > ::milliseconds(rug_fuzz_1),
            Duration::milliseconds(1)
        );
        debug_assert_eq!(
            < i16 as ext::NumericalDuration > ::milliseconds(- rug_fuzz_2),
            Duration::milliseconds(- 1)
        );
        debug_assert_eq!(
            < i16 as ext::NumericalDuration > ::milliseconds(rug_fuzz_3),
            Duration::seconds(1)
        );
        debug_assert_eq!(
            < i16 as ext::NumericalDuration > ::milliseconds(- rug_fuzz_4),
            Duration::seconds(- 1)
        );
        let _rug_ed_tests_llm_16_246_llm_16_245_rrrruuuugggg_test_milliseconds = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_249 {
    use super::*;
    use crate::*;
    #[test]
    fn test_nanoseconds() {
        let _rug_st_tests_llm_16_249_rrrruuuugggg_test_nanoseconds = 0;
        let rug_fuzz_0 = 100;
        let duration = <i8 as ext::NumericalDuration>::nanoseconds(-rug_fuzz_0);
        debug_assert_eq!(duration, Duration::nanoseconds(- 100));
        let _rug_ed_tests_llm_16_249_rrrruuuugggg_test_nanoseconds = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_252 {
    use super::*;
    use crate::*;
    use crate::Duration;
    #[test]
    fn test_weeks() {
        let _rug_st_tests_llm_16_252_rrrruuuugggg_test_weeks = 0;
        let rug_fuzz_0 = 2;
        let duration = Duration::weeks(rug_fuzz_0);
        debug_assert_eq!(duration, Duration::seconds(1_209_600));
        let _rug_ed_tests_llm_16_252_rrrruuuugggg_test_weeks = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_304 {
    use super::*;
    use crate::*;
    use crate::NumericalDuration;
    #[test]
    fn test_days() {
        let _rug_st_tests_llm_16_304_rrrruuuugggg_test_days = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 5;
        let duration = std::num::NonZeroI16::new(rug_fuzz_0).unwrap();
        let result: crate::Duration = <std::num::NonZeroI16 as crate::ext::NumericalDuration>::days(
                duration,
            )
            .into();
        let expected = crate::Duration::days(rug_fuzz_1);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_304_rrrruuuugggg_test_days = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_306_llm_16_305 {
    use std::num::NonZeroI16;
    use crate::ext::NumericalDuration;
    use crate::duration::Duration;
    #[test]
    fn test_hours() {
        fn hours(duration: NonZeroI16) -> Duration {
            Duration::hours(duration.get() as i64)
        }
        let hours_fn = hours as fn(NonZeroI16) -> Duration;
        let duration = NonZeroI16::new(2).unwrap();
        assert_eq!(hours_fn(duration), Duration::hours(2));
    }
}
#[cfg(test)]
mod tests_llm_16_307 {
    use crate::{Duration, NumericalDuration};
    #[test]
    fn test_microseconds() {
        let _rug_st_tests_llm_16_307_rrrruuuugggg_test_microseconds = 0;
        let rug_fuzz_0 = 10;
        let value: std::num::NonZeroI16 = std::num::NonZeroI16::new(rug_fuzz_0).unwrap();
        debug_assert_eq!(value.microseconds(), Duration::microseconds(10));
        let _rug_ed_tests_llm_16_307_rrrruuuugggg_test_microseconds = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_311_llm_16_310 {
    use super::*;
    use crate::*;
    use std::cmp::Ordering;
    use std::convert::TryFrom;
    use std::num::NonZeroI16;
    #[test]
    fn test_minutes() {
        let _rug_st_tests_llm_16_311_llm_16_310_rrrruuuugggg_test_minutes = 0;
        let rug_fuzz_0 = 10;
        let duration = NonZeroI16::new(rug_fuzz_0).unwrap();
        let result = <NonZeroI16 as ext::NumericalDuration>::minutes(duration)
            .as_seconds_f64();
        debug_assert_eq!(result, 600.0);
        let _rug_ed_tests_llm_16_311_llm_16_310_rrrruuuugggg_test_minutes = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_312 {
    use super::*;
    use crate::*;
    use std::num::NonZeroI16;
    #[test]
    fn test_nanoseconds() {
        let _rug_st_tests_llm_16_312_rrrruuuugggg_test_nanoseconds = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 5;
        let duration = NonZeroI16::new(rug_fuzz_0).unwrap().nanoseconds();
        let expected = Duration::nanoseconds(rug_fuzz_1);
        debug_assert_eq!(duration, expected);
        let _rug_ed_tests_llm_16_312_rrrruuuugggg_test_nanoseconds = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_316 {
    use super::*;
    use crate::*;
    use core::num::NonZeroI16;
    use crate::{Duration, NumericalDuration};
    #[test]
    fn test_weeks() {
        let _rug_st_tests_llm_16_316_rrrruuuugggg_test_weeks = 0;
        let rug_fuzz_0 = 2;
        let rug_fuzz_1 = 2;
        let duration = NonZeroI16::new(rug_fuzz_0).unwrap().weeks();
        let expected = Duration::weeks(rug_fuzz_1);
        debug_assert_eq!(duration, expected);
        let _rug_ed_tests_llm_16_316_rrrruuuugggg_test_weeks = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_317 {
    use std::convert::TryFrom;
    use std::num::NonZeroI32;
    use crate::ext::NumericalDuration;
    use crate::duration::Duration;
    #[test]
    fn test_days() {
        let _rug_st_tests_llm_16_317_rrrruuuugggg_test_days = 0;
        let rug_fuzz_0 = 5;
        let duration: Duration = NonZeroI32::new(rug_fuzz_0).map(|n| n.days()).unwrap();
        debug_assert_eq!(duration, Duration::days(5));
        let _rug_ed_tests_llm_16_317_rrrruuuugggg_test_days = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_319_llm_16_318 {
    use super::*;
    use crate::*;
    use core::num::NonZeroI32;
    use crate::*;
    #[test]
    fn test_hours() {
        let _rug_st_tests_llm_16_319_llm_16_318_rrrruuuugggg_test_hours = 0;
        let rug_fuzz_0 = 2;
        let rug_fuzz_1 = 2;
        let duration = NonZeroI32::new(rug_fuzz_0).unwrap().hours();
        let expected = Duration::hours(rug_fuzz_1);
        debug_assert_eq!(duration, expected);
        let _rug_ed_tests_llm_16_319_llm_16_318_rrrruuuugggg_test_hours = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_320 {
    use std::convert::TryFrom;
    use crate::ext::NumericalDuration;
    use crate::duration::Duration;
    use std::num::NonZeroI32;
    #[test]
    fn test_microseconds() {
        let _rug_st_tests_llm_16_320_rrrruuuugggg_test_microseconds = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 1000;
        let duration: Duration = NonZeroI32::new(rug_fuzz_0)
            .map(|num| num.microseconds())
            .unwrap();
        let expected: Duration = Duration::new(rug_fuzz_1, rug_fuzz_2 * rug_fuzz_3);
        debug_assert_eq!(duration, expected);
        let _rug_ed_tests_llm_16_320_rrrruuuugggg_test_microseconds = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_322 {
    use super::*;
    use crate::*;
    use std::num::NonZeroI32;
    #[test]
    fn test_milliseconds() {
        let _rug_st_tests_llm_16_322_rrrruuuugggg_test_milliseconds = 0;
        let rug_fuzz_0 = 5;
        let duration: Duration = NonZeroI32::new(rug_fuzz_0).unwrap().milliseconds();
        debug_assert_eq!(duration.whole_seconds(), 0);
        debug_assert_eq!(duration.whole_nanoseconds(), 5_000_000);
        let _rug_ed_tests_llm_16_322_rrrruuuugggg_test_milliseconds = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_323 {
    use super::*;
    use crate::*;
    use std::num::NonZeroI32;
    #[test]
    fn test_minutes() {
        let _rug_st_tests_llm_16_323_rrrruuuugggg_test_minutes = 0;
        let rug_fuzz_0 = 5;
        let duration = NonZeroI32::new(rug_fuzz_0).unwrap();
        let result = duration.minutes();
        debug_assert_eq!(result, Duration::minutes(5));
        let _rug_ed_tests_llm_16_323_rrrruuuugggg_test_minutes = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_326 {
    use std::num::NonZeroI32;
    use crate::ext::NumericalDuration;
    use crate::Duration;
    #[test]
    fn test_seconds() {
        let _rug_st_tests_llm_16_326_rrrruuuugggg_test_seconds = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let non_zero_seconds = NonZeroI32::new(rug_fuzz_0).unwrap();
        let duration = non_zero_seconds.seconds();
        debug_assert_eq!(duration, Duration::seconds(10));
        let non_zero_seconds = NonZeroI32::new(-rug_fuzz_1).unwrap();
        let duration = non_zero_seconds.seconds();
        debug_assert_eq!(duration, Duration::seconds(- 10));
        let _rug_ed_tests_llm_16_326_rrrruuuugggg_test_seconds = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_327 {
    use super::*;
    use crate::*;
    use std::num::NonZeroI32;
    #[test]
    fn test_weeks() {
        let _rug_st_tests_llm_16_327_rrrruuuugggg_test_weeks = 0;
        let rug_fuzz_0 = 2;
        let rug_fuzz_1 = 2;
        let duration = NonZeroI32::new(rug_fuzz_0).unwrap().weeks();
        let expected = Duration::weeks(rug_fuzz_1);
        debug_assert_eq!(duration, expected);
        let _rug_ed_tests_llm_16_327_rrrruuuugggg_test_weeks = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_332 {
    use std::num::NonZeroI64;
    use crate::ext::NumericalDuration;
    use crate::Duration;
    #[test]
    fn test_microseconds() {
        let _rug_st_tests_llm_16_332_rrrruuuugggg_test_microseconds = 0;
        let rug_fuzz_0 = 100;
        let rug_fuzz_1 = 100;
        let duration: Duration = NonZeroI64::new(rug_fuzz_0).unwrap().microseconds();
        let expected_duration = Duration::microseconds(rug_fuzz_1);
        debug_assert_eq!(duration, expected_duration);
        let _rug_ed_tests_llm_16_332_rrrruuuugggg_test_microseconds = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_335 {
    use std::num::NonZeroI64;
    use crate::ext::NumericalDuration;
    use crate::duration::Duration;
    #[test]
    fn test_minutes() {
        let _rug_st_tests_llm_16_335_rrrruuuugggg_test_minutes = 0;
        let rug_fuzz_0 = 5;
        let duration = NonZeroI64::new(rug_fuzz_0).unwrap().minutes();
        debug_assert_eq!(duration, Duration::minutes(5));
        let _rug_ed_tests_llm_16_335_rrrruuuugggg_test_minutes = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_339 {
    use super::*;
    use crate::*;
    use std::convert::TryInto;
    #[test]
    fn test_seconds() {
        let _rug_st_tests_llm_16_339_rrrruuuugggg_test_seconds = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 5;
        let non_zero_i64: std::num::NonZeroI64 = std::num::NonZeroI64::new(rug_fuzz_0)
            .unwrap();
        let duration: Duration = non_zero_i64.seconds();
        let expected: Duration = Duration::seconds(rug_fuzz_1);
        debug_assert_eq!(duration, expected);
        let _rug_ed_tests_llm_16_339_rrrruuuugggg_test_seconds = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_340 {
    use super::*;
    use crate::*;
    #[test]
    fn test_weeks() {
        let _rug_st_tests_llm_16_340_rrrruuuugggg_non_zero = 0;
        fn equal(duration1: Duration, duration2: Duration) {
            let _rug_st_tests_llm_16_340_rrrruuuugggg_equal = 0;
            debug_assert_eq!(duration1, duration2);
            let _rug_ed_tests_llm_16_340_rrrruuuugggg_equal = 0;
        }
        fn non_zero(duration: Duration) {
            let _rug_st_tests_llm_16_340_rrrruuuugggg_non_zero = 0;
            debug_assert_ne!(duration, Duration::zero());
            let _rug_ed_tests_llm_16_340_rrrruuuugggg_non_zero = 0;
        }
        equal(Duration::weeks(1), Duration::seconds(604800));
        equal(Duration::weeks(2), Duration::seconds(1209600));
        equal(Duration::weeks(3), Duration::seconds(1814400));
        equal(Duration::weeks(4), Duration::seconds(2419200));
        non_zero(Duration::weeks(1));
        non_zero(Duration::weeks(2));
        non_zero(Duration::weeks(3));
        non_zero(Duration::weeks(4));
        let _rug_ed_tests_llm_16_340_rrrruuuugggg_non_zero = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_342_llm_16_341 {
    use super::*;
    use crate::*;
    use std::convert::TryFrom;
    use crate::duration::Duration;
    #[test]
    fn test_days() {
        let _rug_st_tests_llm_16_342_llm_16_341_rrrruuuugggg_test_days = 0;
        let rug_fuzz_0 = 5;
        let duration = Duration::days(rug_fuzz_0);
        debug_assert_eq!(duration.whole_days(), 5);
        let _rug_ed_tests_llm_16_342_llm_16_341_rrrruuuugggg_test_days = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_345 {
    use super::*;
    use crate::*;
    use std::num::NonZeroI8;
    #[test]
    fn test_microseconds() {
        let _rug_st_tests_llm_16_345_rrrruuuugggg_test_microseconds = 0;
        let rug_fuzz_0 = 1;
        let non_zero_i8 = NonZeroI8::new(rug_fuzz_0).unwrap();
        let duration = non_zero_i8.microseconds();
        debug_assert_eq!(duration.whole_microseconds(), 1);
        let _rug_ed_tests_llm_16_345_rrrruuuugggg_test_microseconds = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_348 {
    use super::*;
    use crate::*;
    use std::num::NonZeroI8;
    #[test]
    fn test_minutes() {
        let _rug_st_tests_llm_16_348_rrrruuuugggg_test_minutes = 0;
        let rug_fuzz_0 = 10;
        let duration = Duration::minutes(rug_fuzz_0);
        debug_assert_eq!(duration, Duration::seconds(10 * 60));
        let _rug_ed_tests_llm_16_348_rrrruuuugggg_test_minutes = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_352 {
    use super::*;
    use crate::*;
    use crate::Duration;
    #[test]
    fn test_seconds() {
        let _rug_st_tests_llm_16_352_rrrruuuugggg_test_seconds = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 5;
        let non_zero_i8 = std::num::NonZeroI8::new(rug_fuzz_0).unwrap();
        let duration: Duration = non_zero_i8.seconds();
        let expected = Duration::seconds(rug_fuzz_1);
        debug_assert_eq!(duration, expected);
        let _rug_ed_tests_llm_16_352_rrrruuuugggg_test_seconds = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_357 {
    use super::*;
    use crate::*;
    #[test]
    fn test_hours() {
        let _rug_st_tests_llm_16_357_rrrruuuugggg_test_hours = 0;
        let rug_fuzz_0 = 1;
        let duration = Duration::hours(rug_fuzz_0);
        debug_assert_eq!(duration, Duration::new(1, 0));
        let _rug_ed_tests_llm_16_357_rrrruuuugggg_test_hours = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_360 {
    use std::num::NonZeroU16;
    use crate::ext::NumericalDuration;
    use crate::Duration;
    #[test]
    fn test_milliseconds() {
        let _rug_st_tests_llm_16_360_rrrruuuugggg_test_milliseconds = 0;
        let rug_fuzz_0 = 100;
        let rug_fuzz_1 = "Valid duration";
        let rug_fuzz_2 = 100;
        let duration: Duration = NonZeroU16::new(rug_fuzz_0)
            .expect(rug_fuzz_1)
            .milliseconds();
        let expected: Duration = Duration::milliseconds(rug_fuzz_2);
        debug_assert_eq!(duration, expected);
        let _rug_ed_tests_llm_16_360_rrrruuuugggg_test_milliseconds = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_399 {
    use super::*;
    use crate::*;
    use std::num::NonZeroU32;
    #[test]
    fn test_weeks() {
        let _rug_st_tests_llm_16_399_rrrruuuugggg_test_weeks = 0;
        let rug_fuzz_0 = 2;
        let duration = Duration::weeks(rug_fuzz_0);
        debug_assert_eq!(duration.whole_seconds(), 1_209_600);
        let _rug_ed_tests_llm_16_399_rrrruuuugggg_test_weeks = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_436 {
    use super::*;
    use crate::*;
    use std::convert::TryFrom;
    #[test]
    fn test_microseconds() {
        let _rug_st_tests_llm_16_436_rrrruuuugggg_test_microseconds = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 1_000_000;
        let duration = StdDuration::from_secs(rug_fuzz_0);
        let expected = Duration::microseconds(rug_fuzz_1);
        debug_assert_eq!(
            < StdDuration as TryFrom < Duration > > ::try_from(expected), Ok(duration)
        );
        let _rug_ed_tests_llm_16_436_rrrruuuugggg_test_microseconds = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_439 {
    use std::num::NonZeroU8;
    use crate::ext::NumericalDuration;
    use crate::Duration;
    #[test]
    fn test_minutes() {
        let _rug_st_tests_llm_16_439_rrrruuuugggg_test_minutes = 0;
        let rug_fuzz_0 = 1;
        let minutes: NonZeroU8 = unsafe { NonZeroU8::new_unchecked(rug_fuzz_0) };
        let duration = minutes.minutes();
        debug_assert_eq!(duration, Duration::minutes(1));
        let _rug_ed_tests_llm_16_439_rrrruuuugggg_test_minutes = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_445 {
    use super::*;
    use crate::*;
    use crate::ext::NumericalDuration;
    use crate::ext::NumericalStdDurationShort;
    use crate::Duration;
    use std::num::NonZeroU8;
    #[test]
    fn test_weeks() {
        let _rug_st_tests_llm_16_445_rrrruuuugggg_test_weeks = 0;
        let rug_fuzz_0 = 1u8;
        let rug_fuzz_1 = 2u8;
        let rug_fuzz_2 = 0u8;
        debug_assert_eq!(
            NumericalDuration::weeks(NonZeroU8::new(rug_fuzz_0).unwrap())
            .as_seconds_f64(), 604_800.0
        );
        debug_assert_eq!(
            NumericalDuration::weeks(NonZeroU8::new(rug_fuzz_1).unwrap())
            .as_seconds_f64(), 1_209_600.0
        );
        debug_assert_eq!(
            NumericalDuration::weeks(NonZeroU8::new(rug_fuzz_2).unwrap())
            .as_seconds_f64(), 0.0
        );
        let _rug_ed_tests_llm_16_445_rrrruuuugggg_test_weeks = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_490 {
    use crate::ext::NumericalDuration;
    use crate::Duration;
    #[test]
    fn test_weeks() {
        let _rug_st_tests_llm_16_490_rrrruuuugggg_test_weeks = 0;
        let rug_fuzz_0 = 2;
        let duration: Duration = rug_fuzz_0.weeks();
        debug_assert_eq!(duration, Duration::seconds(1209600));
        let _rug_ed_tests_llm_16_490_rrrruuuugggg_test_weeks = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_515 {
    use super::*;
    use crate::*;
    #[test]
    fn test_minutes() {
        let _rug_st_tests_llm_16_515_rrrruuuugggg_test_minutes = 0;
        let rug_fuzz_0 = 5;
        let duration = Duration::minutes(rug_fuzz_0);
        debug_assert_eq!(duration, Duration::seconds(300));
        let _rug_ed_tests_llm_16_515_rrrruuuugggg_test_minutes = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_520 {
    use super::*;
    use crate::*;
    use duration::Duration;
    use ext::NumericalDuration;
    #[test]
    fn test_weeks() {
        let _rug_st_tests_llm_16_520_rrrruuuugggg_test_weeks = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 100;
        debug_assert_eq!(
            < u32 as ext::NumericalDuration > ::weeks(rug_fuzz_0), Duration::weeks(1)
        );
        debug_assert_eq!(
            < u32 as ext::NumericalDuration > ::weeks(rug_fuzz_1), Duration::weeks(0)
        );
        debug_assert_eq!(
            < u32 as ext::NumericalDuration > ::weeks(rug_fuzz_2), Duration::weeks(100)
        );
        let _rug_ed_tests_llm_16_520_rrrruuuugggg_test_weeks = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_546 {
    use super::*;
    use crate::*;
    use crate::ext::NumericalStdDuration;
    use std::cmp::Ordering;
    use std::convert::TryFrom;
    #[test]
    fn test_std_minutes() {
        let _rug_st_tests_llm_16_546_rrrruuuugggg_test_std_minutes = 0;
        let rug_fuzz_0 = 180;
        let rug_fuzz_1 = 3;
        let duration = Duration::from_std(StdDuration::from_secs(rug_fuzz_0)).unwrap();
        let std_duration = <u64 as NumericalStdDuration>::std_minutes(rug_fuzz_1);
        debug_assert_eq!(std_duration, duration);
        let _rug_ed_tests_llm_16_546_rrrruuuugggg_test_std_minutes = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_554_llm_16_553 {
    use super::*;
    use crate::*;
    use crate::*;
    #[test]
    fn test_days() {
        let _rug_st_tests_llm_16_554_llm_16_553_rrrruuuugggg_test_days = 0;
        let rug_fuzz_0 = 5;
        let dur = NumericalDuration::days(rug_fuzz_0);
        debug_assert_eq!(dur, Duration::days(5));
        let _rug_ed_tests_llm_16_554_llm_16_553_rrrruuuugggg_test_days = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_556 {
    use super::*;
    use crate::*;
    use std::convert::TryInto;
    #[test]
    fn test_hours() {
        let _rug_st_tests_llm_16_556_rrrruuuugggg_test_hours = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 5;
        let duration = Duration::hours(rug_fuzz_0);
        debug_assert_eq!(duration.whole_seconds(), 5 * 3600);
        debug_assert_eq!(duration.subsec_nanoseconds(), 0);
        let duration: Duration = ext::NumericalDuration::hours(rug_fuzz_1)
            .try_into()
            .unwrap();
        debug_assert_eq!(duration.whole_seconds(), 5 * 3600);
        debug_assert_eq!(duration.subsec_nanoseconds(), 0);
        let _rug_ed_tests_llm_16_556_rrrruuuugggg_test_hours = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_562 {
    use crate::Duration;
    #[test]
    fn test_minutes() {
        let _rug_st_tests_llm_16_562_rrrruuuugggg_test_minutes = 0;
        let rug_fuzz_0 = 1;
        let duration = Duration::minutes(rug_fuzz_0);
        debug_assert_eq!(duration, Duration::seconds(60));
        let _rug_ed_tests_llm_16_562_rrrruuuugggg_test_minutes = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_563 {
    use super::*;
    use crate::*;
    #[test]
    fn test_nanoseconds() {
        let _rug_st_tests_llm_16_563_rrrruuuugggg_test_nanoseconds = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 1;
        let d1 = Duration::nanoseconds(rug_fuzz_0);
        debug_assert_eq!(d1.whole_nanoseconds(), 1);
        let d2 = Duration::nanoseconds(-rug_fuzz_1);
        debug_assert_eq!(d2.whole_nanoseconds(), - 1);
        let _rug_ed_tests_llm_16_563_rrrruuuugggg_test_nanoseconds = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_564 {
    use crate::ext::NumericalDuration;
    use crate::Duration;
    #[test]
    fn test_seconds() {
        let _rug_st_tests_llm_16_564_rrrruuuugggg_test_seconds = 0;
        let rug_fuzz_0 = 5;
        let duration = rug_fuzz_0.seconds();
        debug_assert_eq!(duration, Duration::new(5, 0));
        let _rug_ed_tests_llm_16_564_rrrruuuugggg_test_seconds = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_565 {
    use super::*;
    use crate::*;
    #[test]
    fn test_weeks() {
        let _rug_st_tests_llm_16_565_rrrruuuugggg_test_weeks = 0;
        let rug_fuzz_0 = 2;
        let rug_fuzz_1 = 2;
        let duration = <u8 as ext::NumericalDuration>::weeks(rug_fuzz_0);
        let expected = Duration::weeks(rug_fuzz_1);
        debug_assert_eq!(duration, expected);
        let _rug_ed_tests_llm_16_565_rrrruuuugggg_test_weeks = 0;
    }
}
#[cfg(test)]
mod tests_rug_140 {
    use super::*;
    use crate::ext::NumericalDuration;
    #[test]
    fn test_microseconds() {
        let _rug_st_tests_rug_140_rrrruuuugggg_test_microseconds = 0;
        let rug_fuzz_0 = 1234567890;
        let p0: i64 = rug_fuzz_0;
        p0.microseconds();
        let _rug_ed_tests_rug_140_rrrruuuugggg_test_microseconds = 0;
    }
}
#[cfg(test)]
mod tests_rug_141 {
    use super::*;
    use crate::ext::NumericalDuration;
    #[test]
    fn test_milliseconds() {
        let _rug_st_tests_rug_141_rrrruuuugggg_test_milliseconds = 0;
        let rug_fuzz_0 = 100;
        let p0: i64 = rug_fuzz_0;
        debug_assert_eq!(p0.milliseconds(), Duration::milliseconds(p0 as i64));
        let _rug_ed_tests_rug_141_rrrruuuugggg_test_milliseconds = 0;
    }
}
#[cfg(test)]
mod tests_rug_145 {
    use super::*;
    use std::num::NonZeroU8;
    use crate::ext::NumericalDuration;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_145_rrrruuuugggg_sample = 0;
        let rug_fuzz_0 = 25;
        #[cfg(test)]
        mod tests_rug_145_prepare {
            use std::num::NonZeroU8;
            #[test]
            fn sample() {
                let _rug_st_tests_rug_145_prepare_rrrruuuugggg_sample = 0;
                let rug_fuzz_0 = 0;
                let rug_fuzz_1 = 25;
                let rug_fuzz_2 = 0;
                let _rug_st_tests_rug_145_rrrruuuugggg_sample = rug_fuzz_0;
                let rug_fuzz_0 = rug_fuzz_1;
                let mut v25 = NonZeroU8::new(rug_fuzz_0).unwrap();
                let _rug_ed_tests_rug_145_rrrruuuugggg_sample = rug_fuzz_2;
                let _rug_ed_tests_rug_145_prepare_rrrruuuugggg_sample = 0;
            }
        }
        let mut p0 = NonZeroU8::new(25).unwrap();
        <std::num::NonZeroU8 as NumericalDuration>::milliseconds(p0);
        let _rug_ed_tests_rug_145_rrrruuuugggg_sample = 0;
    }
}
#[cfg(test)]
mod tests_rug_198 {
    use super::*;
    use std::time::Duration;
    #[test]
    fn test_std_nanoseconds() {
        let _rug_st_tests_rug_198_rrrruuuugggg_test_std_nanoseconds = 0;
        let rug_fuzz_0 = 100;
        let p0: u32 = rug_fuzz_0;
        let result = p0.std_nanoseconds();
        debug_assert_eq!(result, Duration::from_nanos(100));
        let _rug_ed_tests_rug_198_rrrruuuugggg_test_std_nanoseconds = 0;
    }
}
#[cfg(test)]
mod tests_rug_212 {
    use super::*;
    use std::time::Duration;
    use crate::ext::{NumericalStdDuration, StdDuration};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_212_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 2;
        let rug_fuzz_1 = 604_800;
        let p0: u64 = rug_fuzz_0;
        let result = p0.std_weeks();
        let expected = Duration::from_secs(p0 * rug_fuzz_1);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_rug_212_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_225_prepare {
    use std::num::NonZeroU16;
    #[test]
    fn sample() {
        let _rug_st_tests_rug_225_prepare_rrrruuuugggg_sample = 0;
        let rug_fuzz_0 = 1;
        let mut v31 = NonZeroU16::new(rug_fuzz_0).unwrap();
        let _rug_ed_tests_rug_225_prepare_rrrruuuugggg_sample = 0;
    }
}
#[cfg(test)]
mod tests_rug_225 {
    use super::*;
    use std::time::Duration;
    use std::num::NonZeroU16;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_225_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let mut p0 = NonZeroU16::new(rug_fuzz_0).unwrap();
        <NonZeroU16 as NumericalStdDuration>::std_minutes(p0);
        let _rug_ed_tests_rug_225_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_227 {
    use super::*;
    use std::num::NonZeroU16;
    use std::time::Duration as StdDuration;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_227_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let p0: NonZeroU16 = NonZeroU16::new(rug_fuzz_0).unwrap();
        p0.std_days();
        let _rug_ed_tests_rug_227_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_231 {
    use super::*;
    use std::num::NonZeroU32;
    use std::time::Duration as StdDuration;
    #[test]
    fn test_std_milliseconds() {
        let _rug_st_tests_rug_231_rrrruuuugggg_test_std_milliseconds = 0;
        let rug_fuzz_0 = 42;
        let p0: NonZeroU32 = NonZeroU32::new(rug_fuzz_0).unwrap();
        let result: StdDuration = p0.std_milliseconds();
        debug_assert_eq!(result, StdDuration::from_millis(42 as u64));
        let _rug_ed_tests_rug_231_rrrruuuugggg_test_std_milliseconds = 0;
    }
}
#[cfg(test)]
mod tests_rug_238 {
    use super::*;
    use std::num::NonZeroU64;
    use std::time::Duration as StdDuration;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_238_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 37;
        let mut p0 = NonZeroU64::new(rug_fuzz_0).unwrap();
        p0.std_microseconds();
        let _rug_ed_tests_rug_238_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_261 {
    use super::*;
    use crate::{NumericalStdDurationShort, ext::NumericalStdDuration};
    use std::num::NonZeroU64;
    use std::time::Duration;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_261_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 37;
        let mut p0: NonZeroU64 = NonZeroU64::new(rug_fuzz_0).unwrap();
        let result: Duration = p0.nanoseconds();
        let _rug_ed_tests_rug_261_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_262 {
    use super::*;
    use crate::{NumericalStdDurationShort, ext::NumericalStdDuration};
    use std::num::NonZeroU32;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_262_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: NonZeroU32 = NonZeroU32::new(rug_fuzz_0).unwrap();
        <std::num::NonZeroU32 as NumericalStdDurationShort>::microseconds(p0);
        let _rug_ed_tests_rug_262_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_263 {
    use super::*;
    use crate::NumericalStdDurationShort;
    use std::num::NonZeroU16;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_263_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let mut p0 = NonZeroU16::new(rug_fuzz_0).unwrap();
        <NonZeroU16 as NumericalStdDurationShort>::milliseconds(p0);
        let _rug_ed_tests_rug_263_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_264 {
    use super::*;
    use crate::{NumericalStdDurationShort, NumericalStdDuration};
    use std::num::NonZeroU32;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_264_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: NonZeroU32 = NonZeroU32::new(rug_fuzz_0).unwrap();
        <NonZeroU32 as NumericalStdDurationShort>::seconds(p0);
        let _rug_ed_tests_rug_264_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_265 {
    use super::*;
    use crate::{NumericalStdDurationShort, NumericalStdDuration};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_265_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 37;
        let mut p0 = std::num::NonZeroU64::new(rug_fuzz_0).unwrap();
        <std::num::NonZeroU64 as NumericalStdDurationShort>::minutes(p0);
        let _rug_ed_tests_rug_265_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_266 {
    use super::*;
    use crate::{ext::NumericalStdDurationShort, NumericalStdDuration};
    use std::num::NonZeroU16;
    #[test]
    fn test_hours() {
        let _rug_st_tests_rug_266_rrrruuuugggg_test_hours = 0;
        let rug_fuzz_0 = 1;
        let mut p0 = NonZeroU16::new(rug_fuzz_0).unwrap();
        <NonZeroU16 as NumericalStdDurationShort>::hours(p0);
        let _rug_ed_tests_rug_266_rrrruuuugggg_test_hours = 0;
    }
}
#[cfg(test)]
mod tests_rug_268 {
    use super::*;
    use crate::{NumericalStdDurationShort, NumericalStdDuration};
    use std::num::NonZeroU32;
    use std::time::Duration;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_268_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: NonZeroU32 = NonZeroU32::new(rug_fuzz_0).unwrap();
        let result = <NonZeroU32 as NumericalStdDurationShort>::weeks(p0);
        debug_assert_eq!(result, < NonZeroU32 as NumericalStdDuration > ::std_weeks(p0));
        let _rug_ed_tests_rug_268_rrrruuuugggg_test_rug = 0;
    }
}
