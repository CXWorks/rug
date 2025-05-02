use crate::error;
#[cfg(feature = "std")]
use crate::Instant;
use const_fn::const_fn;
use core::{
    cmp::Ordering,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign},
    time::Duration as StdDuration,
};
use standback::convert::{TryFrom, TryInto};
#[allow(unused_imports)]
use standback::prelude::*;
/// A span of time with nanosecond precision.
///
/// Each `Duration` is composed of a whole number of seconds and a fractional
/// part represented in nanoseconds.
///
/// `Duration` implements many traits, including [`Add`], [`Sub`], [`Mul`], and
/// [`Div`], among others.
///
/// This implementation allows for negative durations, unlike
/// [`core::time::Duration`].
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(from = "crate::serde::Duration", into = "crate::serde::Duration")
)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Duration {
    /// Number of whole seconds.
    seconds: i64,
    /// Number of nanoseconds within the second. The sign always matches the
    /// `seconds` field.
    nanoseconds: i32,
}
/// The number of seconds in one minute.
const SECONDS_PER_MINUTE: i64 = 60;
/// The number of seconds in one hour.
const SECONDS_PER_HOUR: i64 = 60 * SECONDS_PER_MINUTE;
/// The number of seconds in one day.
const SECONDS_PER_DAY: i64 = 24 * SECONDS_PER_HOUR;
/// The number of seconds in one week.
const SECONDS_PER_WEEK: i64 = 7 * SECONDS_PER_DAY;
impl Duration {
    /// Equivalent to `0.seconds()`.
    ///
    /// ```rust
    /// # use time::{Duration, prelude::*};
    /// assert_eq!(Duration::zero(), 0.seconds());
    /// ```
    pub const fn zero() -> Self {
        Self::seconds(0)
    }
    /// Equivalent to `1.nanoseconds()`.
    ///
    /// ```rust
    /// # use time::{Duration, prelude::*};
    /// assert_eq!(Duration::nanosecond(), 1.nanoseconds());
    /// ```
    pub const fn nanosecond() -> Self {
        Self::nanoseconds(1)
    }
    /// Equivalent to `1.microseconds()`.
    ///
    /// ```rust
    /// # use time::{Duration, prelude::*};
    /// assert_eq!(Duration::microsecond(), 1.microseconds());
    /// ```
    pub const fn microsecond() -> Self {
        Self::microseconds(1)
    }
    /// Equivalent to `1.milliseconds()`.
    ///
    /// ```rust
    /// # use time::{Duration, prelude::*};
    /// assert_eq!(Duration::millisecond(), 1.milliseconds());
    /// ```
    pub const fn millisecond() -> Self {
        Self::milliseconds(1)
    }
    /// Equivalent to `1.seconds()`.
    ///
    /// ```rust
    /// # use time::{Duration, prelude::*};
    /// assert_eq!(Duration::second(), 1.seconds());
    /// ```
    pub const fn second() -> Self {
        Self::seconds(1)
    }
    /// Equivalent to `1.minutes()`.
    ///
    /// ```rust
    /// # use time::{Duration, prelude::*};
    /// assert_eq!(Duration::minute(), 1.minutes());
    /// ```
    pub const fn minute() -> Self {
        Self::minutes(1)
    }
    /// Equivalent to `1.hours()`.
    ///
    /// ```rust
    /// # use time::{Duration, prelude::*};
    /// assert_eq!(Duration::hour(), 1.hours());
    /// ```
    pub const fn hour() -> Self {
        Self::hours(1)
    }
    /// Equivalent to `1.days()`.
    ///
    /// ```rust
    /// # use time::{Duration, prelude::*};
    /// assert_eq!(Duration::day(), 1.days());
    /// ```
    pub const fn day() -> Self {
        Self::days(1)
    }
    /// Equivalent to `1.weeks()`.
    ///
    /// ```rust
    /// # use time::{Duration, prelude::*};
    /// assert_eq!(Duration::week(), 1.weeks());
    /// ```
    pub const fn week() -> Self {
        Self::weeks(1)
    }
    /// The maximum possible duration. Adding any positive duration to this will
    /// cause an overflow.
    ///
    /// The value returned by this method may change at any time.
    pub const fn max_value() -> Self {
        Self::new(i64::max_value(), 999_999_999)
    }
    /// The minimum possible duration. Adding any negative duration to this will
    /// cause an overflow.
    ///
    /// The value returned by this method may change at any time.
    pub const fn min_value() -> Self {
        Self::new(i64::min_value(), -999_999_999)
    }
    /// Check if a duration is exactly zero.
    ///
    /// ```rust
    /// # use time::prelude::*;
    /// assert!(0.seconds().is_zero());
    /// assert!(!1.nanoseconds().is_zero());
    /// ```
    pub const fn is_zero(self) -> bool {
        (self.seconds == 0) & (self.nanoseconds == 0)
    }
    /// Check if a duration is negative.
    ///
    /// ```rust
    /// # use time::prelude::*;
    /// assert!((-1).seconds().is_negative());
    /// assert!(!0.seconds().is_negative());
    /// assert!(!1.seconds().is_negative());
    /// ```
    pub const fn is_negative(self) -> bool {
        (self.seconds < 0) | (self.nanoseconds < 0)
    }
    /// Check if a duration is positive.
    ///
    /// ```rust
    /// # use time::{prelude::*};
    /// assert!(1.seconds().is_positive());
    /// assert!(!0.seconds().is_positive());
    /// assert!(!(-1).seconds().is_positive());
    /// ```
    pub const fn is_positive(self) -> bool {
        (self.seconds > 0) | (self.nanoseconds > 0)
    }
    /// Get the sign of the duration.
    ///
    /// ```rust
    /// # #![allow(deprecated)]
    /// # use time::{Sign, prelude::*};
    /// assert_eq!(1.seconds().sign(), Sign::Positive);
    /// assert_eq!((-1).seconds().sign(), Sign::Negative);
    /// assert_eq!(0.seconds().sign(), Sign::Zero);
    /// ```
    #[deprecated(
        since = "0.2.7",
        note = "To obtain the sign of a `Duration`, you should use the `is_positive`, \
                `is_negative`, and `is_zero` methods."
    )]
    #[allow(deprecated, clippy::missing_const_for_fn)]
    pub fn sign(self) -> crate::Sign {
        use crate::Sign::*;
        if self.nanoseconds > 0 {
            Positive
        } else if self.nanoseconds < 0 {
            Negative
        } else if self.seconds > 0 {
            Positive
        } else if self.seconds < 0 {
            Negative
        } else {
            Zero
        }
    }
    /// Get the absolute value of the duration.
    ///
    /// ```rust
    /// # use time::prelude::*;
    /// assert_eq!(1.seconds().abs(), 1.seconds());
    /// assert_eq!(0.seconds().abs(), 0.seconds());
    /// assert_eq!((-1).seconds().abs(), 1.seconds());
    /// ```
    ///
    /// This function is `const fn` when using rustc >= 1.39.
    #[const_fn("1.39")]
    pub const fn abs(self) -> Self {
        Self {
            seconds: self.seconds.abs(),
            nanoseconds: self.nanoseconds.abs(),
        }
    }
    /// Convert the existing `Duration` to a `std::time::Duration` and its sign.
    #[allow(clippy::missing_const_for_fn)]
    #[cfg(feature = "std")]
    pub(crate) fn abs_std(self) -> StdDuration {
        StdDuration::new(self.seconds.abs() as u64, self.nanoseconds.abs() as u32)
    }
    /// Create a new `Duration` with the provided seconds and nanoseconds. If
    /// nanoseconds is at least ±10<sup>9</sup>, it will wrap to the number of
    /// seconds.
    ///
    /// ```rust
    /// # use time::{Duration, prelude::*};
    /// assert_eq!(Duration::new(1, 0), 1.seconds());
    /// assert_eq!(Duration::new(-1, 0), (-1).seconds());
    /// assert_eq!(Duration::new(1, 2_000_000_000), 3.seconds());
    /// ```
    pub const fn new(seconds: i64, nanoseconds: i32) -> Self {
        Self {
            seconds: (seconds + nanoseconds as i64 / 1_000_000_000)
                + (((((seconds + nanoseconds as i64 / 1_000_000_000) > 0) as i8
                    - ((seconds + nanoseconds as i64 / 1_000_000_000) < 0) as i8) == -1)
                    & ((((nanoseconds % 1_000_000_000) > 0) as i8
                        - ((nanoseconds % 1_000_000_000) < 0) as i8) == 1)) as i64
                - (((((seconds + nanoseconds as i64 / 1_000_000_000) > 0) as i8
                    - ((seconds + nanoseconds as i64 / 1_000_000_000) < 0) as i8) == 1)
                    & ((((nanoseconds % 1_000_000_000) > 0) as i8
                        - ((nanoseconds % 1_000_000_000) < 0) as i8) == -1)) as i64,
            nanoseconds: (nanoseconds % 1_000_000_000)
                + 1_000_000_000
                    * ((((((seconds + nanoseconds as i64 / 1_000_000_000) > 0) as i8
                        - ((seconds + nanoseconds as i64 / 1_000_000_000) < 0) as i8)
                        == 1)
                        & ((((nanoseconds % 1_000_000_000) > 0) as i8
                            - ((nanoseconds % 1_000_000_000) < 0) as i8) == -1)) as i32
                        - (((((seconds + nanoseconds as i64 / 1_000_000_000) > 0) as i8
                            - ((seconds + nanoseconds as i64 / 1_000_000_000) < 0) as i8)
                            == -1)
                            & ((((nanoseconds % 1_000_000_000) > 0) as i8
                                - ((nanoseconds % 1_000_000_000) < 0) as i8) == 1)) as i32),
        }
    }
    /// Create a new `Duration` with the given number of weeks. Equivalent to
    /// `Duration::seconds(weeks * 604_800)`.
    ///
    /// ```rust
    /// # use time::{Duration, prelude::*};
    /// assert_eq!(Duration::weeks(1), 604_800.seconds());
    /// ```
    pub const fn weeks(weeks: i64) -> Self {
        Self::seconds(weeks * SECONDS_PER_WEEK)
    }
    /// Get the number of whole weeks in the duration.
    ///
    /// ```rust
    /// # use time::prelude::*;
    /// assert_eq!(1.weeks().whole_weeks(), 1);
    /// assert_eq!((-1).weeks().whole_weeks(), -1);
    /// assert_eq!(6.days().whole_weeks(), 0);
    /// assert_eq!((-6).days().whole_weeks(), 0);
    /// ```
    pub const fn whole_weeks(self) -> i64 {
        self.whole_seconds() / SECONDS_PER_WEEK
    }
    /// Create a new `Duration` with the given number of days. Equivalent to
    /// `Duration::seconds(days * 86_400)`.
    ///
    /// ```rust
    /// # use time::{Duration, prelude::*};
    /// assert_eq!(Duration::days(1), 86_400.seconds());
    /// ```
    pub const fn days(days: i64) -> Self {
        Self::seconds(days * SECONDS_PER_DAY)
    }
    /// Get the number of whole days in the duration.
    ///
    /// ```rust
    /// # use time::prelude::*;
    /// assert_eq!(1.days().whole_days(), 1);
    /// assert_eq!((-1).days().whole_days(), -1);
    /// assert_eq!(23.hours().whole_days(), 0);
    /// assert_eq!((-23).hours().whole_days(), 0);
    /// ```
    pub const fn whole_days(self) -> i64 {
        self.whole_seconds() / SECONDS_PER_DAY
    }
    /// Create a new `Duration` with the given number of hours. Equivalent to
    /// `Duration::seconds(hours * 3_600)`.
    ///
    /// ```rust
    /// # use time::{Duration, prelude::*};
    /// assert_eq!(Duration::hours(1), 3_600.seconds());
    /// ```
    pub const fn hours(hours: i64) -> Self {
        Self::seconds(hours * SECONDS_PER_HOUR)
    }
    /// Get the number of whole hours in the duration.
    ///
    /// ```rust
    /// # use time::prelude::*;
    /// assert_eq!(1.hours().whole_hours(), 1);
    /// assert_eq!((-1).hours().whole_hours(), -1);
    /// assert_eq!(59.minutes().whole_hours(), 0);
    /// assert_eq!((-59).minutes().whole_hours(), 0);
    /// ```
    pub const fn whole_hours(self) -> i64 {
        self.whole_seconds() / SECONDS_PER_HOUR
    }
    /// Create a new `Duration` with the given number of minutes. Equivalent to
    /// `Duration::seconds(minutes * 60)`.
    ///
    /// ```rust
    /// # use time::{Duration, prelude::*};
    /// assert_eq!(Duration::minutes(1), 60.seconds());
    /// ```
    pub const fn minutes(minutes: i64) -> Self {
        Self::seconds(minutes * SECONDS_PER_MINUTE)
    }
    /// Get the number of whole minutes in the duration.
    ///
    /// ```rust
    /// # use time::prelude::*;
    /// assert_eq!(1.minutes().whole_minutes(), 1);
    /// assert_eq!((-1).minutes().whole_minutes(), -1);
    /// assert_eq!(59.seconds().whole_minutes(), 0);
    /// assert_eq!((-59).seconds().whole_minutes(), 0);
    /// ```
    pub const fn whole_minutes(self) -> i64 {
        self.whole_seconds() / SECONDS_PER_MINUTE
    }
    /// Create a new `Duration` with the given number of seconds.
    ///
    /// ```rust
    /// # use time::{Duration, prelude::*};
    /// assert_eq!(Duration::seconds(1), 1_000.milliseconds());
    /// ```
    pub const fn seconds(seconds: i64) -> Self {
        Self { seconds, nanoseconds: 0 }
    }
    /// Get the number of whole seconds in the duration.
    ///
    /// ```rust
    /// # use time::prelude::*;
    /// assert_eq!(1.seconds().whole_seconds(), 1);
    /// assert_eq!((-1).seconds().whole_seconds(), -1);
    /// assert_eq!(1.minutes().whole_seconds(), 60);
    /// assert_eq!((-1).minutes().whole_seconds(), -60);
    /// ```
    pub const fn whole_seconds(self) -> i64 {
        self.seconds
    }
    /// Creates a new `Duration` from the specified number of seconds
    /// represented as `f64`.
    ///
    /// ```rust
    /// # use time::{Duration, prelude::*};
    /// assert_eq!(Duration::seconds_f64(0.5), 0.5.seconds());
    /// assert_eq!(Duration::seconds_f64(-0.5), -0.5.seconds());
    /// ```
    pub fn seconds_f64(seconds: f64) -> Self {
        Self {
            seconds: seconds as i64,
            nanoseconds: ((seconds % 1.) * 1_000_000_000.) as i32,
        }
    }
    /// Get the number of fractional seconds in the duration.
    ///
    /// ```rust
    /// # use time::prelude::*;
    /// assert_eq!(1.5.seconds().as_seconds_f64(), 1.5);
    /// assert_eq!((-1.5).seconds().as_seconds_f64(), -1.5);
    /// ```
    pub fn as_seconds_f64(self) -> f64 {
        self.seconds as f64 + self.nanoseconds as f64 / 1_000_000_000.
    }
    /// Creates a new `Duration` from the specified number of seconds
    /// represented as `f32`.
    ///
    /// ```rust
    /// # use time::{Duration, prelude::*};
    /// assert_eq!(Duration::seconds_f32(0.5), 0.5.seconds());
    /// assert_eq!(Duration::seconds_f32(-0.5), (-0.5).seconds());
    /// ```
    pub fn seconds_f32(seconds: f32) -> Self {
        Self {
            seconds: seconds as i64,
            nanoseconds: ((seconds % 1.) * 1_000_000_000.) as i32,
        }
    }
    /// Get the number of fractional seconds in the duration.
    ///
    /// ```rust
    /// # use time::prelude::*;
    /// assert_eq!(1.5.seconds().as_seconds_f32(), 1.5);
    /// assert_eq!((-1.5).seconds().as_seconds_f32(), -1.5);
    /// ```
    pub fn as_seconds_f32(self) -> f32 {
        self.seconds as f32 + self.nanoseconds as f32 / 1_000_000_000.
    }
    /// Create a new `Duration` with the given number of milliseconds.
    ///
    /// ```rust
    /// # use time::{Duration, prelude::*};
    /// assert_eq!(Duration::milliseconds(1), 1_000.microseconds());
    /// assert_eq!(Duration::milliseconds(-1), (-1_000).microseconds());
    /// ```
    pub const fn milliseconds(milliseconds: i64) -> Self {
        Self {
            seconds: milliseconds / 1_000,
            nanoseconds: ((milliseconds % 1_000) * 1_000_000) as i32,
        }
    }
    /// Get the number of whole milliseconds in the duration.
    ///
    /// ```rust
    /// # use time::prelude::*;
    /// assert_eq!(1.seconds().whole_milliseconds(), 1_000);
    /// assert_eq!((-1).seconds().whole_milliseconds(), -1_000);
    /// assert_eq!(1.milliseconds().whole_milliseconds(), 1);
    /// assert_eq!((-1).milliseconds().whole_milliseconds(), -1);
    /// ```
    pub const fn whole_milliseconds(self) -> i128 {
        self.seconds as i128 * 1_000 + self.nanoseconds as i128 / 1_000_000
    }
    /// Get the number of milliseconds past the number of whole seconds.
    ///
    /// Always in the range `-1_000..1_000`.
    ///
    /// ```rust
    /// # use time::prelude::*;
    /// assert_eq!(1.4.seconds().subsec_milliseconds(), 400);
    /// assert_eq!((-1.4).seconds().subsec_milliseconds(), -400);
    /// ```
    pub const fn subsec_milliseconds(self) -> i16 {
        (self.nanoseconds / 1_000_000) as i16
    }
    /// Create a new `Duration` with the given number of microseconds.
    ///
    /// ```rust
    /// # use time::{Duration, prelude::*};
    /// assert_eq!(Duration::microseconds(1), 1_000.nanoseconds());
    /// assert_eq!(Duration::microseconds(-1), (-1_000).nanoseconds());
    /// ```
    pub const fn microseconds(microseconds: i64) -> Self {
        Self {
            seconds: microseconds / 1_000_000,
            nanoseconds: ((microseconds % 1_000_000) * 1_000) as i32,
        }
    }
    /// Get the number of whole microseconds in the duration.
    ///
    /// ```rust
    /// # use time::prelude::*;
    /// assert_eq!(1.milliseconds().whole_microseconds(), 1_000);
    /// assert_eq!((-1).milliseconds().whole_microseconds(), -1_000);
    /// assert_eq!(1.microseconds().whole_microseconds(), 1);
    /// assert_eq!((-1).microseconds().whole_microseconds(), -1);
    /// ```
    pub const fn whole_microseconds(self) -> i128 {
        self.seconds as i128 * 1_000_000 + self.nanoseconds as i128 / 1_000
    }
    /// Get the number of microseconds past the number of whole seconds.
    ///
    /// Always in the range `-1_000_000..1_000_000`.
    ///
    /// ```rust
    /// # use time::prelude::*;
    /// assert_eq!(1.0004.seconds().subsec_microseconds(), 400);
    /// assert_eq!((-1.0004).seconds().subsec_microseconds(), -400);
    /// ```
    pub const fn subsec_microseconds(self) -> i32 {
        self.nanoseconds / 1_000
    }
    /// Create a new `Duration` with the given number of nanoseconds.
    ///
    /// ```rust
    /// # use time::{Duration, prelude::*};
    /// assert_eq!(Duration::nanoseconds(1), 1.microseconds() / 1_000);
    /// assert_eq!(Duration::nanoseconds(-1), (-1).microseconds() / 1_000);
    /// ```
    pub const fn nanoseconds(nanoseconds: i64) -> Self {
        Self {
            seconds: nanoseconds / 1_000_000_000,
            nanoseconds: (nanoseconds % 1_000_000_000) as i32,
        }
    }
    /// Create a new `Duration` with the given number of nanoseconds.
    ///
    /// As the input range cannot be fully mapped to the output, this should
    /// only be used where it's known to result in a valid value.
    pub(crate) const fn nanoseconds_i128(nanoseconds: i128) -> Self {
        Self {
            seconds: (nanoseconds / 1_000_000_000) as i64,
            nanoseconds: (nanoseconds % 1_000_000_000) as i32,
        }
    }
    /// Get the number of nanoseconds in the duration.
    ///
    /// ```rust
    /// # use time::prelude::*;
    /// assert_eq!(1.microseconds().whole_nanoseconds(), 1_000);
    /// assert_eq!((-1).microseconds().whole_nanoseconds(), -1_000);
    /// assert_eq!(1.nanoseconds().whole_nanoseconds(), 1);
    /// assert_eq!((-1).nanoseconds().whole_nanoseconds(), -1);
    /// ```
    pub const fn whole_nanoseconds(self) -> i128 {
        self.seconds as i128 * 1_000_000_000 + self.nanoseconds as i128
    }
    /// Get the number of nanoseconds past the number of whole seconds.
    ///
    /// The returned value will always be in the range
    /// `-1_000_000_000..1_000_000_000`.
    ///
    /// ```rust
    /// # use time::prelude::*;
    /// assert_eq!(1.000_000_400.seconds().subsec_nanoseconds(), 400);
    /// assert_eq!((-1.000_000_400).seconds().subsec_nanoseconds(), -400);
    /// ```
    pub const fn subsec_nanoseconds(self) -> i32 {
        self.nanoseconds
    }
    /// Computes `self + rhs`, returning `None` if an overflow occurred.
    ///
    /// ```rust
    /// # use time::{Duration, prelude::*};
    /// assert_eq!(5.seconds().checked_add(5.seconds()), Some(10.seconds()));
    /// assert_eq!(Duration::max_value().checked_add(1.nanoseconds()), None);
    /// assert_eq!((-5).seconds().checked_add(5.seconds()), Some(0.seconds()));
    /// ```
    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        let mut seconds = self.seconds.checked_add(rhs.seconds)?;
        let mut nanoseconds = self.nanoseconds + rhs.nanoseconds;
        if nanoseconds >= 1_000_000_000 || seconds < 0 && nanoseconds > 0 {
            nanoseconds -= 1_000_000_000;
            seconds = seconds.checked_add(1)?;
        } else if nanoseconds <= -1_000_000_000 || seconds > 0 && nanoseconds < 0 {
            nanoseconds += 1_000_000_000;
            seconds = seconds.checked_sub(1)?;
        }
        debug_assert_ne!(seconds.signum() * nanoseconds.signum() as i64, - 1);
        debug_assert!((- 999_999_999..1_000_000_000).contains(& nanoseconds));
        Some(Self { seconds, nanoseconds })
    }
    /// Computes `self - rhs`, returning `None` if an overflow occurred.
    ///
    /// ```rust
    /// # use time::{Duration, prelude::*};
    /// assert_eq!(5.seconds().checked_sub(5.seconds()), Some(Duration::zero()));
    /// assert_eq!(Duration::min_value().checked_sub(1.nanoseconds()), None);
    /// assert_eq!(5.seconds().checked_sub(10.seconds()), Some((-5).seconds()));
    /// ```
    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.checked_add(-rhs)
    }
    /// Computes `self * rhs`, returning `None` if an overflow occurred.
    ///
    /// ```rust
    /// # use time::{Duration, prelude::*};
    /// assert_eq!(5.seconds().checked_mul(2), Some(10.seconds()));
    /// assert_eq!(5.seconds().checked_mul(-2), Some((-10).seconds()));
    /// assert_eq!(5.seconds().checked_mul(0), Some(0.seconds()));
    /// assert_eq!(Duration::max_value().checked_mul(2), None);
    /// assert_eq!(Duration::min_value().checked_mul(2), None);
    /// ```
    pub fn checked_mul(self, rhs: i32) -> Option<Self> {
        let total_nanos = self.nanoseconds as i64 * rhs as i64;
        let extra_secs = total_nanos / 1_000_000_000;
        let nanoseconds = (total_nanos % 1_000_000_000) as i32;
        let seconds = self.seconds.checked_mul(rhs as i64)?.checked_add(extra_secs)?;
        Some(Self { seconds, nanoseconds })
    }
    /// Computes `self / rhs`, returning `None` if `rhs == 0`.
    ///
    /// ```rust
    /// # use time::prelude::*;
    /// assert_eq!(10.seconds().checked_div(2), Some(5.seconds()));
    /// assert_eq!(10.seconds().checked_div(-2), Some((-5).seconds()));
    /// assert_eq!(1.seconds().checked_div(0), None);
    /// ```
    ///
    /// This function is `const fn` when using rustc >= 1.46.
    #[const_fn("1.46")]
    pub const fn checked_div(self, rhs: i32) -> Option<Self> {
        if rhs == 0 {
            return None;
        }
        let seconds = self.seconds / (rhs as i64);
        let carry = self.seconds - seconds * (rhs as i64);
        let extra_nanos = carry * 1_000_000_000 / (rhs as i64);
        let nanoseconds = self.nanoseconds / rhs + (extra_nanos as i32);
        Some(Self { seconds, nanoseconds })
    }
    /// Runs a closure, returning the duration of time it took to run. The
    /// return value of the closure is provided in the second part of the tuple.
    #[cfg(feature = "std")]
    #[cfg_attr(__time_02_docs, doc(cfg(feature = "std")))]
    pub fn time_fn<T>(f: impl FnOnce() -> T) -> (Self, T) {
        let start = Instant::now();
        let return_value = f();
        let end = Instant::now();
        (end - start, return_value)
    }
}
/// Functions that have been renamed or had signatures changed since v0.1. As
/// such, they are deprecated.
#[cfg(feature = "deprecated")]
#[allow(clippy::missing_docs_in_private_items, clippy::missing_const_for_fn)]
impl Duration {
    #[deprecated(since = "0.2.0", note = "Use the `whole_weeks` function")]
    pub fn num_weeks(&self) -> i64 {
        self.whole_weeks()
    }
    #[deprecated(since = "0.2.0", note = "Use the `whole_days` function")]
    pub fn num_days(&self) -> i64 {
        self.whole_days()
    }
    #[deprecated(since = "0.2.0", note = "Use the `whole_hours` function")]
    pub fn num_hours(&self) -> i64 {
        self.whole_hours()
    }
    #[deprecated(since = "0.2.0", note = "Use the `whole_minutes` function")]
    pub fn num_minutes(&self) -> i64 {
        self.whole_minutes()
    }
    #[allow(clippy::missing_const_for_fn)]
    #[deprecated(since = "0.2.0", note = "Use the `whole_seconds` function")]
    pub fn num_seconds(&self) -> i64 {
        self.whole_seconds()
    }
    /// [`Duration::whole_milliseconds`] returns an `i128`, rather than
    /// panicking on overflow. To avoid panicking, this method currently limits
    /// the value to the range `i64::min_value()..=i64::max_value()`.
    #[deprecated(
        since = "0.2.0",
        note = "Use the `whole_milliseconds` function. The value is clamped between \
                `i64::min_value()` and `i64::max_value()`."
    )]
    pub fn num_milliseconds(&self) -> i64 {
        let millis = self.whole_milliseconds();
        if millis > i64::max_value() as i128 {
            return i64::max_value();
        }
        if millis < i64::min_value() as i128 {
            return i64::min_value();
        }
        millis as i64
    }
    /// [`Duration::whole_microseconds`] returns an `i128` rather than returning
    /// `None` on `i64` overflow.
    #[deprecated(since = "0.2.0", note = "Use the `whole_microseconds` function")]
    pub fn num_microseconds(&self) -> Option<i64> {
        let micros = self.whole_microseconds();
        if micros.abs() > i64::max_value() as i128 { None } else { Some(micros as i64) }
    }
    /// [`Duration::whole_nanoseconds`] returns an `i128` rather than returning
    /// `None` on `i64` overflow.
    #[deprecated(since = "0.2.0", note = "Use the `whole_nanoseconds` function")]
    pub fn num_nanoseconds(&self) -> Option<i64> {
        let nanos = self.whole_nanoseconds();
        if nanos.abs() > i64::max_value() as i128 { None } else { Some(nanos as i64) }
    }
    #[cfg(feature = "std")]
    #[deprecated(since = "0.2.0", note = "Use the `time_fn` function")]
    pub fn span<F: FnOnce()>(f: F) -> Self {
        Self::time_fn(f).0
    }
    #[allow(deprecated)]
    #[deprecated(
        since = "0.2.0",
        note = "Use `Duration::try_from(value)` or `value.try_into()`"
    )]
    pub fn from_std(std: StdDuration) -> Result<Self, error::ConversionRange> {
        std.try_into()
    }
    #[allow(deprecated)]
    #[deprecated(
        since = "0.2.0",
        note = "Use `std::time::Duration::try_from(value)` or `value.try_into()`"
    )]
    pub fn to_std(&self) -> Result<StdDuration, error::ConversionRange> {
        (*self).try_into()
    }
}
impl TryFrom<StdDuration> for Duration {
    type Error = error::ConversionRange;
    fn try_from(original: StdDuration) -> Result<Self, error::ConversionRange> {
        Ok(
            Self::new(
                original.as_secs().try_into().map_err(|_| error::ConversionRange)?,
                original.subsec_nanos().try_into().map_err(|_| error::ConversionRange)?,
            ),
        )
    }
}
impl TryFrom<Duration> for StdDuration {
    type Error = error::ConversionRange;
    fn try_from(duration: Duration) -> Result<Self, error::ConversionRange> {
        Ok(
            Self::new(
                duration.seconds.try_into().map_err(|_| error::ConversionRange)?,
                duration.nanoseconds.try_into().map_err(|_| error::ConversionRange)?,
            ),
        )
    }
}
impl Add for Duration {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        self.checked_add(rhs).expect("overflow when adding durations")
    }
}
impl Add<StdDuration> for Duration {
    type Output = Self;
    fn add(self, std_duration: StdDuration) -> Self::Output {
        self
            + Self::try_from(std_duration)
                .expect("overflow converting `std::time::Duration` to `time::Duration`")
    }
}
impl Add<Duration> for StdDuration {
    type Output = Duration;
    fn add(self, rhs: Duration) -> Self::Output {
        rhs + self
    }
}
impl AddAssign for Duration {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}
impl AddAssign<StdDuration> for Duration {
    fn add_assign(&mut self, rhs: StdDuration) {
        *self = *self + rhs;
    }
}
impl Neg for Duration {
    type Output = Self;
    fn neg(self) -> Self::Output {
        -1 * self
    }
}
impl Sub for Duration {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        self.checked_sub(rhs).expect("overflow when subtracting durations")
    }
}
impl Sub<StdDuration> for Duration {
    type Output = Self;
    fn sub(self, rhs: StdDuration) -> Self::Output {
        self
            - Self::try_from(rhs)
                .expect("overflow converting `std::time::Duration` to `time::Duration`")
    }
}
impl Sub<Duration> for StdDuration {
    type Output = Duration;
    fn sub(self, rhs: Duration) -> Self::Output {
        Duration::try_from(self)
            .expect("overflow converting `std::time::Duration` to `time::Duration`")
            - rhs
    }
}
impl SubAssign for Duration {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}
impl SubAssign<StdDuration> for Duration {
    fn sub_assign(&mut self, rhs: StdDuration) {
        *self = *self - rhs;
    }
}
impl SubAssign<Duration> for StdDuration {
    fn sub_assign(&mut self, rhs: Duration) {
        *self = (*self - rhs)
            .try_into()
            .expect(
                "Cannot represent a resulting duration in std. Try `let x = x - rhs;`, which will \
             change the type.",
            );
    }
}
macro_rules! duration_mul_div_int {
    ($($type:ty),+) => {
        $(impl Mul <$type > for Duration { type Output = Self; fn mul(self, rhs : $type)
        -> Self::Output { Self::nanoseconds_i128(self.whole_nanoseconds().checked_mul(rhs
        as i128).expect("overflow when multiplying duration")) } } impl MulAssign <$type
        > for Duration { fn mul_assign(& mut self, rhs : $type) { * self = * self * rhs;
        } } impl Mul < Duration > for $type { type Output = Duration; fn mul(self, rhs :
        Duration) -> Self::Output { rhs * self } } impl Div <$type > for Duration { type
        Output = Self; fn div(self, rhs : $type) -> Self::Output {
        Self::nanoseconds_i128(self.whole_nanoseconds() / rhs as i128) } } impl DivAssign
        <$type > for Duration { fn div_assign(& mut self, rhs : $type) { * self = * self
        / rhs; } })+
    };
}
duration_mul_div_int![i8, i16, i32, u8, u16, u32];
impl Mul<f32> for Duration {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self::Output {
        Self::seconds_f32(self.as_seconds_f32() * rhs)
    }
}
impl MulAssign<f32> for Duration {
    fn mul_assign(&mut self, rhs: f32) {
        *self = *self * rhs;
    }
}
impl Mul<Duration> for f32 {
    type Output = Duration;
    fn mul(self, rhs: Duration) -> Self::Output {
        rhs * self
    }
}
impl Mul<f64> for Duration {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self::Output {
        Self::seconds_f64(self.as_seconds_f64() * rhs)
    }
}
impl MulAssign<f64> for Duration {
    fn mul_assign(&mut self, rhs: f64) {
        *self = *self * rhs;
    }
}
impl Mul<Duration> for f64 {
    type Output = Duration;
    fn mul(self, rhs: Duration) -> Self::Output {
        rhs * self
    }
}
impl Div<f32> for Duration {
    type Output = Self;
    fn div(self, rhs: f32) -> Self::Output {
        Self::seconds_f32(self.as_seconds_f32() / rhs)
    }
}
impl DivAssign<f32> for Duration {
    fn div_assign(&mut self, rhs: f32) {
        *self = *self / rhs;
    }
}
impl Div<f64> for Duration {
    type Output = Self;
    fn div(self, rhs: f64) -> Self::Output {
        Self::seconds_f64(self.as_seconds_f64() / rhs)
    }
}
impl DivAssign<f64> for Duration {
    fn div_assign(&mut self, rhs: f64) {
        *self = *self / rhs;
    }
}
impl Div<Duration> for Duration {
    type Output = f64;
    fn div(self, rhs: Self) -> Self::Output {
        self.as_seconds_f64() / rhs.as_seconds_f64()
    }
}
impl Div<StdDuration> for Duration {
    type Output = f64;
    fn div(self, rhs: StdDuration) -> Self::Output {
        self.as_seconds_f64() / rhs.as_secs_f64()
    }
}
impl Div<Duration> for StdDuration {
    type Output = f64;
    fn div(self, rhs: Duration) -> Self::Output {
        self.as_secs_f64() / rhs.as_seconds_f64()
    }
}
impl PartialEq<StdDuration> for Duration {
    fn eq(&self, rhs: &StdDuration) -> bool {
        Ok(*self) == Self::try_from(*rhs)
    }
}
impl PartialEq<Duration> for StdDuration {
    fn eq(&self, rhs: &Duration) -> bool {
        rhs == self
    }
}
impl PartialOrd for Duration {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        Some(self.cmp(rhs))
    }
}
impl PartialOrd<StdDuration> for Duration {
    fn partial_cmp(&self, rhs: &StdDuration) -> Option<Ordering> {
        if rhs.as_secs() > i64::max_value() as u64 {
            return Some(Ordering::Less);
        }
        Some(
            self
                .seconds
                .cmp(&(rhs.as_secs() as i64))
                .then_with(|| self.nanoseconds.cmp(&(rhs.subsec_nanos() as i32))),
        )
    }
}
impl PartialOrd<Duration> for StdDuration {
    fn partial_cmp(&self, rhs: &Duration) -> Option<Ordering> {
        rhs.partial_cmp(self).map(Ordering::reverse)
    }
}
impl Ord for Duration {
    fn cmp(&self, rhs: &Self) -> Ordering {
        self.seconds
            .cmp(&rhs.seconds)
            .then_with(|| self.nanoseconds.cmp(&rhs.nanoseconds))
    }
}
#[cfg(test)]
mod tests_llm_16_32 {
    use crate::duration::{Duration, StdDuration};
    use std::convert::TryFrom;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_32_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 0;
        let duration1 = Duration::seconds(rug_fuzz_0);
        let duration2 = Duration::seconds(rug_fuzz_1);
        let std_duration = StdDuration::new(rug_fuzz_2, rug_fuzz_3);
        debug_assert_eq!(duration1.eq(& std_duration), true);
        debug_assert_eq!(duration2.eq(& std_duration), false);
        let _rug_ed_tests_llm_16_32_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_40 {
    use super::*;
    use crate::*;
    use std::ops::Add;
    #[test]
    fn test_add() {
        let _rug_st_tests_llm_16_40_rrrruuuugggg_test_add = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 1;
        let duration = Duration::new(rug_fuzz_0, rug_fuzz_1);
        let std_duration = StdDuration::from_secs(rug_fuzz_2);
        let result = duration.add(std_duration);
        debug_assert_eq!(result, Duration::new(2, 0));
        let _rug_ed_tests_llm_16_40_rrrruuuugggg_test_add = 0;
    }
}
use crate::duration::Duration as MyDuration;
use std::ops::Add as AddTrait;
#[test]
fn test_add() {
    let dur1 = MyDuration {
        seconds: 10,
        nanoseconds: 100,
    };
    let dur2 = MyDuration {
        seconds: 5,
        nanoseconds: 200,
    };
    let expected = MyDuration {
        seconds: 15,
        nanoseconds: 300,
    };
    assert_eq!(dur1.add(dur2), expected);
}
#[cfg(test)]
mod tests_llm_16_50 {
    use super::*;
    use crate::*;
    #[test]
    fn test_div() {
        let _rug_st_tests_llm_16_50_rrrruuuugggg_test_div = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 2.0;
        let rug_fuzz_2 = 0.5;
        let rug_fuzz_3 = 3;
        let duration = Duration::seconds(rug_fuzz_0);
        debug_assert_eq!(duration.div(rug_fuzz_1), duration::Duration::seconds_f64(5.0));
        debug_assert_eq!(
            duration.div(rug_fuzz_2), duration::Duration::seconds_f64(20.0)
        );
        debug_assert_eq!(
            duration.div(rug_fuzz_3), duration::Duration::seconds_f64(3.3333333333333335)
        );
        let _rug_ed_tests_llm_16_50_rrrruuuugggg_test_div = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_52 {
    use std::ops::Div;
    use std::convert::TryFrom;
    use crate::duration::Duration;
    #[test]
    fn test_div() {
        let _rug_st_tests_llm_16_52_rrrruuuugggg_test_div = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 5;
        let rug_fuzz_3 = 10;
        let rug_fuzz_4 = 2;
        let rug_fuzz_5 = 3;
        let rug_fuzz_6 = 0;
        let rug_fuzz_7 = 1;
        let rug_fuzz_8 = 10;
        let rug_fuzz_9 = 2;
        let rug_fuzz_10 = 5;
        let rug_fuzz_11 = 0;
        let duration1 = Duration::seconds(rug_fuzz_0);
        let duration2 = Duration::seconds(rug_fuzz_1);
        let duration3 = Duration::seconds(rug_fuzz_2);
        let duration4 = Duration::zero();
        let duration5 = Duration::seconds(rug_fuzz_3);
        debug_assert_eq!(Duration::div(duration1, rug_fuzz_4), duration2);
        debug_assert_eq!(Duration::div(duration1, rug_fuzz_5), duration3);
        debug_assert_eq!(Duration::div(duration1, rug_fuzz_6), duration4);
        debug_assert_eq!(Duration::div(duration1, rug_fuzz_7), duration5);
        let duration6 = Duration::zero();
        let duration7 = Duration::seconds(rug_fuzz_8);
        let duration8 = Duration::zero();
        debug_assert_eq!(duration6.checked_div(rug_fuzz_9), Some(duration6));
        debug_assert_eq!(duration7.checked_div(rug_fuzz_10), Some(duration8));
        debug_assert_eq!(duration7.checked_div(rug_fuzz_11), None);
        let _rug_ed_tests_llm_16_52_rrrruuuugggg_test_div = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_53 {
    use super::*;
    use crate::*;
    use std::convert::TryFrom;
    #[test]
    fn test_div() {
        let _rug_st_tests_llm_16_53_rrrruuuugggg_test_div = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 2;
        let rug_fuzz_4 = 500_000_000;
        let duration = Duration::new(rug_fuzz_0, rug_fuzz_1);
        let result = duration.div(rug_fuzz_2);
        let expected = Duration::new(rug_fuzz_3, rug_fuzz_4);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_53_rrrruuuugggg_test_div = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_54 {
    use super::*;
    use crate::*;
    #[test]
    fn test_div() {
        let _rug_st_tests_llm_16_54_rrrruuuugggg_test_div = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 2;
        let rug_fuzz_5 = 10;
        let rug_fuzz_6 = 0;
        let rug_fuzz_7 = 3;
        let rug_fuzz_8 = 0;
        let rug_fuzz_9 = 3;
        let duration1 = Duration::new(rug_fuzz_0, rug_fuzz_1);
        let duration2 = Duration::new(rug_fuzz_2, rug_fuzz_3);
        let result = duration1.div(rug_fuzz_4);
        debug_assert_eq!(result, Duration::new(5, 0));
        let duration3 = Duration::new(rug_fuzz_5, rug_fuzz_6);
        let duration4 = Duration::new(rug_fuzz_7, rug_fuzz_8);
        let result = duration3.div(rug_fuzz_9);
        debug_assert_eq!(result, Duration::new(3, 333_333_333));
        let _rug_ed_tests_llm_16_54_rrrruuuugggg_test_div = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_55 {
    use super::*;
    use crate::*;
    use std::convert::TryFrom;
    #[test]
    fn test_div() {
        let _rug_st_tests_llm_16_55_rrrruuuugggg_test_div = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 5;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 2.0;
        let duration = Duration::new(rug_fuzz_0, rug_fuzz_1);
        let std_duration = StdDuration::new(rug_fuzz_2, rug_fuzz_3);
        let expected = rug_fuzz_4;
        let result = duration.div(std_duration);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_55_rrrruuuugggg_test_div = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_56 {
    use super::*;
    use crate::*;
    #[test]
    fn test_div() {
        let _rug_st_tests_llm_16_56_rrrruuuugggg_test_div = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 2;
        let duration = Duration::new(rug_fuzz_0, rug_fuzz_1);
        let rhs: u16 = rug_fuzz_2;
        let result = duration.div(rhs);
        debug_assert_eq!(result, Duration::new(5, 0));
        let _rug_ed_tests_llm_16_56_rrrruuuugggg_test_div = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_58 {
    use super::*;
    use crate::*;
    #[test]
    fn test_div() {
        let _rug_st_tests_llm_16_58_rrrruuuugggg_test_div = 0;
        let rug_fuzz_0 = 500;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 2;
        let duration = Duration::new(rug_fuzz_0, rug_fuzz_1);
        let result: Duration = duration.div(rug_fuzz_2);
        debug_assert_eq!(result.whole_seconds(), 250);
        let _rug_ed_tests_llm_16_58_rrrruuuugggg_test_div = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_68 {
    use super::*;
    use crate::*;
    use crate::Duration as TimeDuration;
    #[test]
    fn test_div_assign_i16() {
        let _rug_st_tests_llm_16_68_rrrruuuugggg_test_div_assign_i16 = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 0;
        let mut duration = TimeDuration::seconds(rug_fuzz_0);
        duration.div_assign(rug_fuzz_1);
        debug_assert_eq!(duration, TimeDuration::seconds(5));
        let mut duration = TimeDuration::seconds(rug_fuzz_2);
        duration.div_assign(rug_fuzz_3);
        debug_assert_eq!(duration, TimeDuration::seconds(10));
        let _rug_ed_tests_llm_16_68_rrrruuuugggg_test_div_assign_i16 = 0;
    }
    #[test]
    fn test_div_assign_f32() {
        let _rug_st_tests_llm_16_68_rrrruuuugggg_test_div_assign_f32 = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 2.0;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 0.0;
        let mut duration = TimeDuration::seconds(rug_fuzz_0);
        duration.div_assign(rug_fuzz_1);
        debug_assert_eq!(duration, TimeDuration::seconds(5));
        let mut duration = TimeDuration::seconds(rug_fuzz_2);
        duration.div_assign(rug_fuzz_3);
        debug_assert_eq!(duration, TimeDuration::seconds(10));
        let _rug_ed_tests_llm_16_68_rrrruuuugggg_test_div_assign_f32 = 0;
    }
    #[test]
    fn test_div_assign_f64() {
        let _rug_st_tests_llm_16_68_rrrruuuugggg_test_div_assign_f64 = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 2.0;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 0.0;
        let mut duration = TimeDuration::seconds(rug_fuzz_0);
        duration.div_assign(rug_fuzz_1);
        debug_assert_eq!(duration, TimeDuration::seconds(5));
        let mut duration = TimeDuration::seconds(rug_fuzz_2);
        duration.div_assign(rug_fuzz_3);
        debug_assert_eq!(duration, TimeDuration::seconds(10));
        let _rug_ed_tests_llm_16_68_rrrruuuugggg_test_div_assign_f64 = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_78_llm_16_77 {
    use std::convert::TryFrom;
    use crate::duration::Duration;
    use std::ops::DivAssign;
    #[test]
    fn test_div_assign() {
        let _rug_st_tests_llm_16_78_llm_16_77_rrrruuuugggg_test_div_assign = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 5;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 2;
        let rug_fuzz_6 = 5;
        let rug_fuzz_7 = 0;
        let rug_fuzz_8 = 0;
        let rug_fuzz_9 = 0;
        let rug_fuzz_10 = 500_000_000;
        let rug_fuzz_11 = 2;
        let rug_fuzz_12 = 0;
        let rug_fuzz_13 = 500_000_000;
        let rug_fuzz_14 = 2;
        let rug_fuzz_15 = 0;
        let rug_fuzz_16 = 0;
        let rug_fuzz_17 = 2;
        let mut duration = Duration::new(rug_fuzz_0, rug_fuzz_1);
        duration /= rug_fuzz_2;
        debug_assert_eq!(duration, Duration::new(2, 500_000_000));
        let mut duration = Duration::new(-rug_fuzz_3, rug_fuzz_4);
        duration /= rug_fuzz_5;
        debug_assert_eq!(duration, Duration::new(- 2, - 500_000_000));
        let mut duration = Duration::new(rug_fuzz_6, rug_fuzz_7);
        duration /= rug_fuzz_8;
        debug_assert_eq!(duration, Duration::new(5, 0));
        let mut duration = Duration::new(rug_fuzz_9, rug_fuzz_10);
        duration /= rug_fuzz_11;
        debug_assert_eq!(duration, Duration::new(0, 250_000_000));
        let mut duration = Duration::new(rug_fuzz_12, rug_fuzz_13);
        duration /= -rug_fuzz_14;
        debug_assert_eq!(duration, Duration::new(0, - 250_000_000));
        let mut duration = Duration::new(rug_fuzz_15, rug_fuzz_16);
        duration /= rug_fuzz_17;
        debug_assert_eq!(duration, Duration::new(0, 0));
        let _rug_ed_tests_llm_16_78_llm_16_77_rrrruuuugggg_test_div_assign = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_88 {
    use std::convert::TryFrom;
    use crate::duration::{Duration, StdDuration, Mul};
    #[test]
    fn test_mul() {
        let _rug_st_tests_llm_16_88_rrrruuuugggg_test_mul = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 2i8;
        let duration = Duration::new(rug_fuzz_0, rug_fuzz_1);
        let result = duration.mul(rug_fuzz_2);
        debug_assert_eq!(result, Duration::new(2, 0));
        let _rug_ed_tests_llm_16_88_rrrruuuugggg_test_mul = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_93 {
    use super::*;
    use crate::*;
    #[test]
    fn test_mul() {
        let _rug_st_tests_llm_16_93_rrrruuuugggg_test_mul = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 2;
        let rug_fuzz_5 = 2;
        let duration1 = Duration::seconds(rug_fuzz_0);
        let duration2 = duration1 * rug_fuzz_1;
        debug_assert_eq!(duration2, Duration::seconds(10));
        let duration3 = duration1 * -rug_fuzz_2;
        debug_assert_eq!(duration3, Duration::seconds(- 10));
        let duration4 = duration1 * rug_fuzz_3;
        debug_assert_eq!(duration4, Duration::seconds(0));
        let duration5 = Duration::max_value();
        let result1 = duration5.checked_mul(rug_fuzz_4);
        debug_assert!(result1.is_none());
        let duration6 = Duration::min_value();
        let result2 = duration6.checked_mul(rug_fuzz_5);
        debug_assert!(result2.is_none());
        let _rug_ed_tests_llm_16_93_rrrruuuugggg_test_mul = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_94 {
    use crate::duration::Duration;
    #[test]
    fn test_mul_assign() {
        let _rug_st_tests_llm_16_94_rrrruuuugggg_test_mul_assign = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 2;
        let mut duration = Duration::seconds(rug_fuzz_0);
        duration *= rug_fuzz_1;
        debug_assert_eq!(duration, Duration::seconds(10));
        let _rug_ed_tests_llm_16_94_rrrruuuugggg_test_mul_assign = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_97 {
    use super::*;
    use crate::*;
    #[test]
    fn test_mul_assign() {
        let _rug_st_tests_llm_16_97_rrrruuuugggg_test_mul_assign = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 5;
        let rug_fuzz_3 = 2;
        let rug_fuzz_4 = 5;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 2;
        let rug_fuzz_7 = 2;
        let mut duration = Duration::seconds(rug_fuzz_0);
        duration.mul_assign(rug_fuzz_1);
        debug_assert_eq!(duration, Duration::seconds(10));
        let mut duration = Duration::seconds(rug_fuzz_2);
        duration.mul_assign(-rug_fuzz_3);
        debug_assert_eq!(duration, Duration::seconds(- 10));
        let mut duration = Duration::seconds(rug_fuzz_4);
        duration.mul_assign(rug_fuzz_5);
        debug_assert_eq!(duration, Duration::seconds(0));
        let mut duration = Duration::max_value();
        duration.mul_assign(rug_fuzz_6);
        debug_assert_eq!(duration, Duration::max_value());
        let mut duration = Duration::min_value();
        duration.mul_assign(rug_fuzz_7);
        debug_assert_eq!(duration, Duration::min_value());
        let _rug_ed_tests_llm_16_97_rrrruuuugggg_test_mul_assign = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_98 {
    use super::*;
    use crate::*;
    #[test]
    fn test_mul_assign() {
        let _rug_st_tests_llm_16_98_rrrruuuugggg_test_mul_assign = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 2;
        let mut duration = Duration::seconds(rug_fuzz_0);
        duration.mul_assign(rug_fuzz_1);
        debug_assert_eq!(duration, Duration::seconds(10));
        let _rug_ed_tests_llm_16_98_rrrruuuugggg_test_mul_assign = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_646 {
    use super::*;
    use crate::*;
    use std::cmp::Ordering;
    #[test]
    fn test_partial_cmp() {
        let _rug_st_tests_llm_16_646_rrrruuuugggg_test_partial_cmp = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 500_000_000;
        let rug_fuzz_6 = 2;
        let rug_fuzz_7 = 0;
        let duration1 = Duration::new(rug_fuzz_0, rug_fuzz_1);
        let duration2 = Duration::new(rug_fuzz_2, rug_fuzz_3);
        let duration3 = Duration::new(rug_fuzz_4, rug_fuzz_5);
        let duration4 = Duration::new(rug_fuzz_6, rug_fuzz_7);
        debug_assert_eq!(duration1.partial_cmp(& duration2), Some(Ordering::Greater));
        debug_assert_eq!(duration2.partial_cmp(& duration1), Some(Ordering::Less));
        debug_assert_eq!(duration2.partial_cmp(& duration2), Some(Ordering::Equal));
        debug_assert_eq!(duration2.partial_cmp(& duration3), Some(Ordering::Less));
        debug_assert_eq!(duration3.partial_cmp(& duration2), Some(Ordering::Greater));
        debug_assert_eq!(duration1.partial_cmp(& duration4), Some(Ordering::Less));
        debug_assert_eq!(duration4.partial_cmp(& duration1), Some(Ordering::Greater));
        let _rug_ed_tests_llm_16_646_rrrruuuugggg_test_partial_cmp = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_649 {
    use super::*;
    use crate::*;
    #[test]
    fn test_add() {
        let _rug_st_tests_llm_16_649_rrrruuuugggg_test_add = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 500_000_000;
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 3;
        let rug_fuzz_5 = 500_000_000;
        let duration_1 = Duration::new(rug_fuzz_0, rug_fuzz_1);
        let duration_2 = Duration::new(rug_fuzz_2, rug_fuzz_3);
        let expected_result = Duration::new(rug_fuzz_4, rug_fuzz_5);
        debug_assert_eq!(duration_1.add(duration_2), expected_result);
        let _rug_ed_tests_llm_16_649_rrrruuuugggg_test_add = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_650 {
    use super::*;
    use crate::*;
    #[test]
    fn test_div() {
        let _rug_st_tests_llm_16_650_rrrruuuugggg_test_div = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 2;
        let dur1 = Duration::seconds(rug_fuzz_0);
        let dur2 = Duration::seconds(rug_fuzz_1);
        let result = dur1.div(dur2);
        debug_assert_eq!(result, 2.5);
        let _rug_ed_tests_llm_16_650_rrrruuuugggg_test_div = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_660_llm_16_659 {
    use super::*;
    use crate::*;
    #[test]
    fn test_mul_duration() {
        let _rug_st_tests_llm_16_660_llm_16_659_rrrruuuugggg_test_mul_duration = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 500_000_000;
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 3;
        let rug_fuzz_4 = 500_000_000;
        let duration1 = Duration::new(rug_fuzz_0, rug_fuzz_1);
        let duration2 = rug_fuzz_2;
        let result = duration1 * duration2;
        let expected_result = Duration::new(rug_fuzz_3, rug_fuzz_4);
        debug_assert_eq!(result, expected_result);
        let _rug_ed_tests_llm_16_660_llm_16_659_rrrruuuugggg_test_mul_duration = 0;
    }
    #[test]
    fn test_mul_duration_negative() {
        let _rug_st_tests_llm_16_660_llm_16_659_rrrruuuugggg_test_mul_duration_negative = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 500_000_000;
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 3;
        let rug_fuzz_4 = 500_000_000;
        let duration1 = Duration::new(rug_fuzz_0, rug_fuzz_1);
        let duration2 = -rug_fuzz_2;
        let result = duration1 * duration2;
        let expected_result = Duration::new(-rug_fuzz_3, -rug_fuzz_4);
        debug_assert_eq!(result, expected_result);
        let _rug_ed_tests_llm_16_660_llm_16_659_rrrruuuugggg_test_mul_duration_negative = 0;
    }
    #[test]
    fn test_mul_duration_zero() {
        let _rug_st_tests_llm_16_660_llm_16_659_rrrruuuugggg_test_mul_duration_zero = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let duration1 = Duration::new(rug_fuzz_0, rug_fuzz_1);
        let duration2 = rug_fuzz_2;
        let result = duration1 * duration2;
        let expected_result = Duration::new(rug_fuzz_3, rug_fuzz_4);
        debug_assert_eq!(result, expected_result);
        let _rug_ed_tests_llm_16_660_llm_16_659_rrrruuuugggg_test_mul_duration_zero = 0;
    }
    #[test]
    fn test_mul_duration_overflow() {
        let _rug_st_tests_llm_16_660_llm_16_659_rrrruuuugggg_test_mul_duration_overflow = 0;
        let rug_fuzz_0 = 2;
        let duration1 = Duration::max_value();
        let duration2 = rug_fuzz_0;
        let result = duration1.checked_mul(duration2);
        debug_assert_eq!(result, None);
        let _rug_ed_tests_llm_16_660_llm_16_659_rrrruuuugggg_test_mul_duration_overflow = 0;
    }
    #[test]
    fn test_mul_duration_overflow_negative() {
        let _rug_st_tests_llm_16_660_llm_16_659_rrrruuuugggg_test_mul_duration_overflow_negative = 0;
        let rug_fuzz_0 = 2;
        let duration1 = Duration::min_value();
        let duration2 = rug_fuzz_0;
        let result = duration1.checked_mul(duration2);
        debug_assert_eq!(result, None);
        let _rug_ed_tests_llm_16_660_llm_16_659_rrrruuuugggg_test_mul_duration_overflow_negative = 0;
    }
    #[test]
    fn test_mul_primitive() {
        let _rug_st_tests_llm_16_660_llm_16_659_rrrruuuugggg_test_mul_primitive = 0;
        let rug_fuzz_0 = 2;
        let rug_fuzz_1 = 500_000_000;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 2;
        let rug_fuzz_4 = 500_000_000;
        let duration = Duration::new(rug_fuzz_0, rug_fuzz_1);
        let primitive = rug_fuzz_2;
        let result = duration * primitive;
        let expected_result = Duration::new(rug_fuzz_3, rug_fuzz_4);
        debug_assert_eq!(result, expected_result);
        let _rug_ed_tests_llm_16_660_llm_16_659_rrrruuuugggg_test_mul_primitive = 0;
    }
    #[test]
    fn test_mul_primitive_negative() {
        let _rug_st_tests_llm_16_660_llm_16_659_rrrruuuugggg_test_mul_primitive_negative = 0;
        let rug_fuzz_0 = 2;
        let rug_fuzz_1 = 500_000_000;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 2;
        let rug_fuzz_4 = 500_000_000;
        let duration = Duration::new(rug_fuzz_0, rug_fuzz_1);
        let primitive = -rug_fuzz_2;
        let result = duration * primitive;
        let expected_result = Duration::new(-rug_fuzz_3, -rug_fuzz_4);
        debug_assert_eq!(result, expected_result);
        let _rug_ed_tests_llm_16_660_llm_16_659_rrrruuuugggg_test_mul_primitive_negative = 0;
    }
    #[test]
    fn test_mul_primitive_zero() {
        let _rug_st_tests_llm_16_660_llm_16_659_rrrruuuugggg_test_mul_primitive_zero = 0;
        let rug_fuzz_0 = 2;
        let rug_fuzz_1 = 500_000_000;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let duration = Duration::new(rug_fuzz_0, rug_fuzz_1);
        let primitive = rug_fuzz_2;
        let result = duration * primitive;
        let expected_result = Duration::new(rug_fuzz_3, rug_fuzz_4);
        debug_assert_eq!(result, expected_result);
        let _rug_ed_tests_llm_16_660_llm_16_659_rrrruuuugggg_test_mul_primitive_zero = 0;
    }
    #[test]
    fn test_mul_primitive_overflow() {
        let _rug_st_tests_llm_16_660_llm_16_659_rrrruuuugggg_test_mul_primitive_overflow = 0;
        let rug_fuzz_0 = 2;
        let duration = Duration::max_value();
        let primitive = rug_fuzz_0;
        let result = duration.checked_mul(primitive);
        debug_assert_eq!(result, None);
        let _rug_ed_tests_llm_16_660_llm_16_659_rrrruuuugggg_test_mul_primitive_overflow = 0;
    }
    #[test]
    fn test_mul_primitive_overflow_negative() {
        let _rug_st_tests_llm_16_660_llm_16_659_rrrruuuugggg_test_mul_primitive_overflow_negative = 0;
        let rug_fuzz_0 = 2;
        let duration = Duration::min_value();
        let primitive = rug_fuzz_0;
        let result = duration.checked_mul(primitive);
        debug_assert_eq!(result, None);
        let _rug_ed_tests_llm_16_660_llm_16_659_rrrruuuugggg_test_mul_primitive_overflow_negative = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_662_llm_16_661 {
    use super::*;
    use crate::*;
    #[test]
    fn test_mul() {
        let _rug_st_tests_llm_16_662_llm_16_661_rrrruuuugggg_test_mul = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 5u16;
        let rug_fuzz_3 = 50;
        let rug_fuzz_4 = 0;
        let duration = Duration {
            seconds: rug_fuzz_0,
            nanoseconds: rug_fuzz_1,
        };
        let rhs = rug_fuzz_2;
        let expected = Duration {
            seconds: rug_fuzz_3,
            nanoseconds: rug_fuzz_4,
        };
        debug_assert_eq!(duration.mul(rhs), expected);
        let _rug_ed_tests_llm_16_662_llm_16_661_rrrruuuugggg_test_mul = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_664 {
    use super::*;
    use crate::*;
    #[test]
    fn test_mul() {
        let _rug_st_tests_llm_16_664_rrrruuuugggg_test_mul = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 500_000_000;
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 2;
        let rug_fuzz_4 = 1_000_000_000;
        let duration = Duration::new(rug_fuzz_0, rug_fuzz_1);
        let rhs = rug_fuzz_2;
        let expected = Duration::new(rug_fuzz_3, rug_fuzz_4);
        debug_assert_eq!(duration.mul(rhs), expected);
        let _rug_ed_tests_llm_16_664_rrrruuuugggg_test_mul = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_665 {
    use super::*;
    use crate::*;
    #[test]
    fn test_mul() {
        let _rug_st_tests_llm_16_665_rrrruuuugggg_test_mul = 0;
        let rug_fuzz_0 = 2;
        let rug_fuzz_1 = 500_000_000;
        let rug_fuzz_2 = 2;
        let duration = Duration::new(rug_fuzz_0, rug_fuzz_1);
        let result = duration.mul(rug_fuzz_2);
        debug_assert_eq!(result, Duration::new(5, 0));
        let _rug_ed_tests_llm_16_665_rrrruuuugggg_test_mul = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_670 {
    use super::*;
    use crate::*;
    #[test]
    fn test_abs() {
        let _rug_st_tests_llm_16_670_rrrruuuugggg_test_abs = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = 1;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 0;
        let rug_fuzz_7 = 1;
        debug_assert_eq!(Duration::zero().abs(), Duration::zero());
        debug_assert_eq!(Duration::nanosecond().abs(), Duration::nanosecond());
        debug_assert_eq!(Duration::microsecond().abs(), Duration::microsecond());
        debug_assert_eq!(Duration::millisecond().abs(), Duration::millisecond());
        debug_assert_eq!(Duration::second().abs(), Duration::second());
        debug_assert_eq!(Duration::minute().abs(), Duration::minute());
        debug_assert_eq!(Duration::hour().abs(), Duration::hour());
        debug_assert_eq!(Duration::day().abs(), Duration::day());
        debug_assert_eq!(Duration::week().abs(), Duration::week());
        debug_assert_eq!(
            Duration::new(rug_fuzz_0, rug_fuzz_1).abs(), Duration::new(1, 0)
        );
        debug_assert_eq!(
            Duration::new(rug_fuzz_2, rug_fuzz_3).abs(), Duration::new(0, 1)
        );
        debug_assert_eq!(
            Duration::new(- rug_fuzz_4, rug_fuzz_5).abs(), Duration::new(1, 0)
        );
        debug_assert_eq!(
            Duration::new(rug_fuzz_6, - rug_fuzz_7).abs(), Duration::new(0, 1)
        );
        let _rug_ed_tests_llm_16_670_rrrruuuugggg_test_abs = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_676 {
    use super::*;
    use crate::*;
    use crate::Duration;
    #[test]
    fn test_as_seconds_f64() {
        let _rug_st_tests_llm_16_676_rrrruuuugggg_test_as_seconds_f64 = 0;
        let rug_fuzz_0 = 1.5;
        let rug_fuzz_1 = 1.5;
        debug_assert_eq!(Duration::seconds_f64(rug_fuzz_0).as_seconds_f64(), 1.5);
        debug_assert_eq!(Duration::seconds_f64(- rug_fuzz_1).as_seconds_f64(), - 1.5);
        let _rug_ed_tests_llm_16_676_rrrruuuugggg_test_as_seconds_f64 = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_683 {
    use super::*;
    use crate::*;
    use crate::Duration;
    #[test]
    fn test_checked_sub() {
        let _rug_st_tests_llm_16_683_rrrruuuugggg_test_checked_sub = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 5;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 5;
        let rug_fuzz_4 = 10;
        debug_assert_eq!(
            Duration::checked_sub(Duration::seconds(rug_fuzz_0),
            Duration::seconds(rug_fuzz_1)), Some(Duration::zero())
        );
        debug_assert_eq!(
            Duration::checked_sub(Duration::min_value(),
            Duration::nanoseconds(rug_fuzz_2)), None
        );
        debug_assert_eq!(
            Duration::checked_sub(Duration::seconds(rug_fuzz_3),
            Duration::seconds(rug_fuzz_4)), Some(Duration::seconds(- 5))
        );
        let _rug_ed_tests_llm_16_683_rrrruuuugggg_test_checked_sub = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_690 {
    use super::*;
    use crate::*;
    use crate::prelude::*;
    #[test]
    fn test_hour() {
        let _rug_st_tests_llm_16_690_rrrruuuugggg_test_hour = 0;
        debug_assert_eq!(Duration::hour(), Duration::hours(1));
        let _rug_ed_tests_llm_16_690_rrrruuuugggg_test_hour = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_692_llm_16_691 {
    use super::*;
    use crate::*;
    use crate::macros::time;
    use crate::prelude::*;
    #[test]
    fn test_hours() {
        let _rug_st_tests_llm_16_692_llm_16_691_rrrruuuugggg_test_hours = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        debug_assert_eq!(Duration::hours(rug_fuzz_0), Duration::seconds(1 * 3_600));
        debug_assert_eq!(Duration::hours(rug_fuzz_1), Duration::seconds(2 * 3_600));
        let _rug_ed_tests_llm_16_692_llm_16_691_rrrruuuugggg_test_hours = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_695 {
    use super::*;
    use crate::*;
    use crate::Duration;
    #[test]
    fn test_is_positive() {
        let _rug_st_tests_llm_16_695_rrrruuuugggg_test_is_positive = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 1;
        debug_assert!(Duration::seconds(rug_fuzz_0).is_positive());
        debug_assert!(! Duration::seconds(rug_fuzz_1).is_positive());
        debug_assert!(! Duration::seconds(- rug_fuzz_2).is_positive());
        let _rug_ed_tests_llm_16_695_rrrruuuugggg_test_is_positive = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_696 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_zero() {
        let _rug_st_tests_llm_16_696_rrrruuuugggg_test_is_zero = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 1;
        debug_assert!(Duration::zero().is_zero());
        debug_assert!(Duration::seconds(rug_fuzz_0).is_zero());
        debug_assert!(Duration::nanoseconds(rug_fuzz_1).is_zero());
        debug_assert!(! Duration::seconds(rug_fuzz_2).is_zero());
        debug_assert!(! Duration::nanoseconds(rug_fuzz_3).is_zero());
        let _rug_ed_tests_llm_16_696_rrrruuuugggg_test_is_zero = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_697 {
    use super::*;
    use crate::*;
    use std::convert::TryFrom;
    #[test]
    fn test_max_value() {
        let _rug_st_tests_llm_16_697_rrrruuuugggg_test_max_value = 0;
        let max_value = Duration::max_value();
        debug_assert_eq!(max_value.seconds, i64::max_value());
        debug_assert_eq!(max_value.nanoseconds, 999_999_999);
        let _rug_ed_tests_llm_16_697_rrrruuuugggg_test_max_value = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_699 {
    use super::*;
    use crate::*;
    use crate::Duration;
    #[test]
    fn test_microsecond() {
        let _rug_st_tests_llm_16_699_rrrruuuugggg_test_microsecond = 0;
        debug_assert_eq!(Duration::microsecond(), Duration::microseconds(1));
        let _rug_ed_tests_llm_16_699_rrrruuuugggg_test_microsecond = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_700 {
    use super::*;
    use crate::*;
    #[test]
    fn test_microseconds() {
        let _rug_st_tests_llm_16_700_rrrruuuugggg_test_microseconds = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 1;
        debug_assert_eq!(
            Duration::microseconds(rug_fuzz_0), Duration::nanoseconds(1_000)
        );
        debug_assert_eq!(
            Duration::microseconds(- rug_fuzz_1), Duration::nanoseconds(- 1_000)
        );
        let _rug_ed_tests_llm_16_700_rrrruuuugggg_test_microseconds = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_703 {
    use super::*;
    use crate::*;
    use std::convert::TryFrom;
    #[test]
    fn test_milliseconds() {
        let _rug_st_tests_llm_16_703_rrrruuuugggg_test_milliseconds = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 1;
        debug_assert_eq!(
            Duration::milliseconds(rug_fuzz_0), Duration::microseconds(1_000)
        );
        debug_assert_eq!(
            Duration::milliseconds(- rug_fuzz_1), Duration::microseconds(- 1_000)
        );
        let _rug_ed_tests_llm_16_703_rrrruuuugggg_test_milliseconds = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_709_llm_16_708 {
    use super::*;
    use crate::*;
    use crate::duration::Duration;
    #[test]
    fn test_minutes() {
        let _rug_st_tests_llm_16_709_llm_16_708_rrrruuuugggg_test_minutes = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 0;
        debug_assert_eq!(Duration::minutes(rug_fuzz_0), Duration::seconds(60));
        debug_assert_eq!(Duration::minutes(rug_fuzz_1), Duration::seconds(120));
        debug_assert_eq!(Duration::minutes(- rug_fuzz_2), Duration::seconds(- 60));
        debug_assert_eq!(Duration::minutes(rug_fuzz_3), Duration::seconds(0));
        let _rug_ed_tests_llm_16_709_llm_16_708_rrrruuuugggg_test_minutes = 0;
    }
}
mod tests_llm_16_718 {
    use super::*;
    use crate::*;
    use std::convert::TryFrom;
    #[test]
    fn test_num_days() {
        let _rug_st_tests_llm_16_718_rrrruuuugggg_test_num_days = 0;
        let rug_fuzz_0 = 7;
        let rug_fuzz_1 = 7;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = 1;
        let duration = Duration::days(rug_fuzz_0);
        debug_assert_eq!(duration.num_days(), 7);
        let duration = Duration::days(-rug_fuzz_1);
        debug_assert_eq!(duration.num_days(), - 7);
        let duration = Duration::days(rug_fuzz_2);
        debug_assert_eq!(duration.num_days(), 0);
        let duration = Duration::days(rug_fuzz_3);
        debug_assert_eq!(duration.num_days(), 1);
        let duration = Duration::days(-rug_fuzz_4);
        debug_assert_eq!(duration.num_days(), - 1);
        let _rug_ed_tests_llm_16_718_rrrruuuugggg_test_num_days = 0;
    }
    #[test]
    fn test_num_days_deprecated() {
        let _rug_st_tests_llm_16_718_rrrruuuugggg_test_num_days_deprecated = 0;
        let rug_fuzz_0 = 7;
        let rug_fuzz_1 = 7;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = 1;
        let duration = Duration::days(rug_fuzz_0);
        debug_assert_eq!(duration.num_days(), duration.whole_days());
        let duration = Duration::days(-rug_fuzz_1);
        debug_assert_eq!(duration.num_days(), duration.whole_days());
        let duration = Duration::days(rug_fuzz_2);
        debug_assert_eq!(duration.num_days(), duration.whole_days());
        let duration = Duration::days(rug_fuzz_3);
        debug_assert_eq!(duration.num_days(), duration.whole_days());
        let duration = Duration::days(-rug_fuzz_4);
        debug_assert_eq!(duration.num_days(), duration.whole_days());
        let _rug_ed_tests_llm_16_718_rrrruuuugggg_test_num_days_deprecated = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_719 {
    use crate::Duration;
    #[test]
    fn test_num_hours() {
        let _rug_st_tests_llm_16_719_rrrruuuugggg_test_num_hours = 0;
        let rug_fuzz_0 = 2;
        let duration = Duration::hours(rug_fuzz_0);
        debug_assert_eq!(duration.num_hours(), 2);
        let _rug_ed_tests_llm_16_719_rrrruuuugggg_test_num_hours = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_724 {
    use super::*;
    use crate::*;
    use std::convert::TryInto;
    #[test]
    fn test_num_minutes() {
        let _rug_st_tests_llm_16_724_rrrruuuugggg_test_num_minutes = 0;
        let rug_fuzz_0 = 10;
        let duration = Duration::minutes(rug_fuzz_0);
        let result = duration.num_minutes();
        debug_assert_eq!(result, 10);
        let _rug_ed_tests_llm_16_724_rrrruuuugggg_test_num_minutes = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_727 {
    use super::*;
    use crate::*;
    #[test]
    fn test_num_seconds() {
        let _rug_st_tests_llm_16_727_rrrruuuugggg_test_num_seconds = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 500_000_000;
        let duration = Duration::new(rug_fuzz_0, rug_fuzz_1);
        debug_assert_eq!(duration.num_seconds(), 10);
        let _rug_ed_tests_llm_16_727_rrrruuuugggg_test_num_seconds = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_732 {
    use super::*;
    use crate::*;
    #[test]
    fn test_seconds() {
        let _rug_st_tests_llm_16_732_rrrruuuugggg_test_seconds = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 0;
        debug_assert_eq!(
            Duration::seconds(rug_fuzz_0), Duration { seconds : 1, nanoseconds : 0 }
        );
        debug_assert_eq!(
            Duration::seconds(- rug_fuzz_1), Duration { seconds : - 1, nanoseconds : 0 }
        );
        debug_assert_eq!(
            Duration::seconds(rug_fuzz_2), Duration { seconds : 0, nanoseconds : 0 }
        );
        let _rug_ed_tests_llm_16_732_rrrruuuugggg_test_seconds = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_733 {
    use super::*;
    use crate::*;
    #[test]
    fn test_seconds_f32() {
        let _rug_st_tests_llm_16_733_rrrruuuugggg_test_seconds_f32 = 0;
        let rug_fuzz_0 = 0.5;
        let rug_fuzz_1 = 0.5;
        debug_assert_eq!(
            Duration::seconds_f32(rug_fuzz_0), Duration::new(0, 500_000_000)
        );
        debug_assert_eq!(
            Duration::seconds_f32(- rug_fuzz_1), Duration::new(0, - 500_000_000)
        );
        let _rug_ed_tests_llm_16_733_rrrruuuugggg_test_seconds_f32 = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_734 {
    use super::*;
    use crate::*;
    use crate::duration::Duration;
    #[test]
    fn test_seconds_f64() {
        let _rug_st_tests_llm_16_734_rrrruuuugggg_test_seconds_f64 = 0;
        let rug_fuzz_0 = 0.5;
        let rug_fuzz_1 = 0.5;
        debug_assert_eq!(Duration::seconds_f64(rug_fuzz_0), Duration::new(0, 500000000));
        debug_assert_eq!(
            Duration::seconds_f64(- rug_fuzz_1), Duration::new(0, - 500000000)
        );
        let _rug_ed_tests_llm_16_734_rrrruuuugggg_test_seconds_f64 = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_736 {
    use super::*;
    use crate::*;
    use crate::ext::*;
    use crate::Sign;
    #[test]
    #[allow(deprecated)]
    fn test_duration_sign() {
        let _rug_st_tests_llm_16_736_rrrruuuugggg_test_duration_sign = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 0;
        debug_assert_eq!(
            ext::NumericalDuration::seconds(rug_fuzz_0).sign(), Sign::Positive
        );
        debug_assert_eq!(
            ext::NumericalDuration::seconds(- rug_fuzz_1).sign(), Sign::Negative
        );
        debug_assert_eq!(ext::NumericalDuration::seconds(rug_fuzz_2).sign(), Sign::Zero);
        let _rug_ed_tests_llm_16_736_rrrruuuugggg_test_duration_sign = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_740_llm_16_739 {
    use super::*;
    use crate::*;
    use crate::ext::*;
    #[test]
    fn test_subsec_microseconds() {
        let _rug_st_tests_llm_16_740_llm_16_739_rrrruuuugggg_test_subsec_microseconds = 0;
        let rug_fuzz_0 = 1.0004;
        let rug_fuzz_1 = 1.0004;
        debug_assert_eq!(
            NumericalDuration::seconds(rug_fuzz_0).subsec_microseconds(), 400
        );
        debug_assert_eq!(
            NumericalDuration::seconds(- rug_fuzz_1).subsec_microseconds(), - 400
        );
        let _rug_ed_tests_llm_16_740_llm_16_739_rrrruuuugggg_test_subsec_microseconds = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_742 {
    use super::*;
    use crate::*;
    use crate::ext::NumericalDuration;
    #[test]
    fn test_subsec_milliseconds() {
        let _rug_st_tests_llm_16_742_rrrruuuugggg_test_subsec_milliseconds = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 1_400_000_000;
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = 1_400_000_000;
        debug_assert_eq!(
            NumericalDuration::seconds(rug_fuzz_0).subsec_milliseconds(), 0
        );
        debug_assert_eq!(
            NumericalDuration::seconds(rug_fuzz_1).subsec_milliseconds(), 0
        );
        debug_assert_eq!(
            NumericalDuration::nanoseconds(rug_fuzz_2).subsec_milliseconds(), 400
        );
        debug_assert_eq!(
            NumericalDuration::seconds(- rug_fuzz_3).subsec_milliseconds(), 0
        );
        debug_assert_eq!(
            NumericalDuration::nanoseconds(- rug_fuzz_4).subsec_milliseconds(), - 400
        );
        let _rug_ed_tests_llm_16_742_rrrruuuugggg_test_subsec_milliseconds = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_751 {
    use crate::duration::Duration;
    use crate::duration::SECONDS_PER_WEEK;
    #[test]
    fn test_weeks() {
        let _rug_st_tests_llm_16_751_rrrruuuugggg_test_weeks = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = 2;
        let rug_fuzz_5 = 10;
        let rug_fuzz_6 = 0;
        debug_assert_eq!(
            Duration::weeks(rug_fuzz_0), Duration::seconds(1 * SECONDS_PER_WEEK)
        );
        debug_assert_eq!(
            Duration::weeks(rug_fuzz_1), Duration::seconds(2 * SECONDS_PER_WEEK)
        );
        debug_assert_eq!(
            Duration::weeks(rug_fuzz_2), Duration::seconds(10 * SECONDS_PER_WEEK)
        );
        debug_assert_eq!(
            Duration::weeks(- rug_fuzz_3), Duration::seconds(- 1 * SECONDS_PER_WEEK)
        );
        debug_assert_eq!(
            Duration::weeks(- rug_fuzz_4), Duration::seconds(- 2 * SECONDS_PER_WEEK)
        );
        debug_assert_eq!(
            Duration::weeks(- rug_fuzz_5), Duration::seconds(- 10 * SECONDS_PER_WEEK)
        );
        debug_assert_eq!(Duration::weeks(rug_fuzz_6), Duration::seconds(0));
        let _rug_ed_tests_llm_16_751_rrrruuuugggg_test_weeks = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_753 {
    use super::*;
    use crate::*;
    use crate::prelude::*;
    #[test]
    fn test_whole_days() {
        let _rug_st_tests_llm_16_753_rrrruuuugggg_test_whole_days = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 23;
        let rug_fuzz_3 = 23;
        debug_assert_eq!(ext::NumericalDuration::days(rug_fuzz_0).whole_days(), 1);
        debug_assert_eq!(ext::NumericalDuration::days(- rug_fuzz_1).whole_days(), - 1);
        debug_assert_eq!(ext::NumericalDuration::hours(rug_fuzz_2).whole_days(), 0);
        debug_assert_eq!(ext::NumericalDuration::hours(- rug_fuzz_3).whole_days(), 0);
        let _rug_ed_tests_llm_16_753_rrrruuuugggg_test_whole_days = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_757 {
    use super::*;
    use crate::*;
    #[test]
    fn test_whole_microseconds() {
        let _rug_st_tests_llm_16_757_rrrruuuugggg_test_whole_microseconds = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 1;
        debug_assert_eq!(Duration::milliseconds(rug_fuzz_0).whole_microseconds(), 1_000);
        debug_assert_eq!(
            Duration::milliseconds(- rug_fuzz_1).whole_microseconds(), - 1_000
        );
        debug_assert_eq!(Duration::microseconds(rug_fuzz_2).whole_microseconds(), 1);
        debug_assert_eq!(Duration::microseconds(- rug_fuzz_3).whole_microseconds(), - 1);
        let _rug_ed_tests_llm_16_757_rrrruuuugggg_test_whole_microseconds = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_768 {
    use crate::duration::Duration;
    #[test]
    fn test_zero() {
        let _rug_st_tests_llm_16_768_rrrruuuugggg_test_zero = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let expected = Duration {
            seconds: rug_fuzz_0,
            nanoseconds: rug_fuzz_1,
        };
        let result = Duration::zero();
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_768_rrrruuuugggg_test_zero = 0;
    }
}
#[cfg(test)]
mod tests_rug_269 {
    use super::*;
    use crate::{Duration, prelude::*};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_269_rrrruuuugggg_test_rug = 0;
        Duration::nanosecond();
        let _rug_ed_tests_rug_269_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_270 {
    use super::*;
    use crate::{Duration, prelude::*};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_270_rrrruuuugggg_test_rug = 0;
        Duration::millisecond();
        let _rug_ed_tests_rug_270_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_271 {
    use super::*;
    use crate::{Duration, prelude::*};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_271_rrrruuuugggg_test_rug = 0;
        Duration::second();
        let _rug_ed_tests_rug_271_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_272 {
    use super::*;
    use crate::{Duration, prelude::*};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_272_rrrruuuugggg_test_rug = 0;
        Duration::minute();
        let _rug_ed_tests_rug_272_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_273 {
    use super::*;
    use crate::{Duration, prelude::*};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_273_rrrruuuugggg_test_rug = 0;
        Duration::day();
        let _rug_ed_tests_rug_273_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_274 {
    use super::*;
    use crate::{Duration, prelude::*};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_274_rrrruuuugggg_test_rug = 0;
        Duration::week();
        let _rug_ed_tests_rug_274_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_275 {
    use super::*;
    use crate::duration::Duration;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_275_rrrruuuugggg_test_rug = 0;
        Duration::min_value();
        let _rug_ed_tests_rug_275_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_276 {
    use super::*;
    use crate::duration::Duration;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_276_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 1;
        let rug_fuzz_5 = 0;
        let mut p0 = Duration::new(-rug_fuzz_0, rug_fuzz_1);
        debug_assert_eq!(p0.is_negative(), true);
        let mut p1 = Duration::new(rug_fuzz_2, rug_fuzz_3);
        debug_assert_eq!(p1.is_negative(), false);
        let mut p2 = Duration::new(rug_fuzz_4, rug_fuzz_5);
        debug_assert_eq!(p2.is_negative(), false);
        let _rug_ed_tests_rug_276_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_277 {
    use super::*;
    use crate::duration::Duration;
    #[test]
    fn test_abs_std() {
        let _rug_st_tests_rug_277_rrrruuuugggg_test_abs_std = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 0;
        let mut p0 = Duration::new(rug_fuzz_0, rug_fuzz_1);
        crate::duration::Duration::abs_std(p0);
        let _rug_ed_tests_rug_277_rrrruuuugggg_test_abs_std = 0;
    }
}
#[cfg(test)]
mod tests_rug_278 {
    use super::*;
    use crate::Duration;
    #[test]
    fn test_duration_new() {
        let _rug_st_tests_rug_278_rrrruuuugggg_test_duration_new = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 0;
        let p0: i64 = rug_fuzz_0;
        let p1: i32 = rug_fuzz_1;
        Duration::new(p0, p1);
        let _rug_ed_tests_rug_278_rrrruuuugggg_test_duration_new = 0;
    }
}
#[cfg(test)]
mod tests_rug_279 {
    use super::*;
    use crate::duration::Duration;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_279_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 0;
        let mut p0 = Duration::new(rug_fuzz_0, rug_fuzz_1);
        Duration::whole_weeks(p0);
        let _rug_ed_tests_rug_279_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_280 {
    use super::*;
    use crate::{Duration, prelude::*};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_280_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 5;
        let p0: i64 = rug_fuzz_0;
        Duration::days(p0);
        let _rug_ed_tests_rug_280_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_281 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_whole_hours() {
        let _rug_st_tests_rug_281_rrrruuuugggg_test_whole_hours = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 59;
        let rug_fuzz_3 = 59;
        let p0 = rug_fuzz_0.hours();
        debug_assert_eq!(p0.whole_hours(), 1);
        let p1 = (-rug_fuzz_1).hours();
        debug_assert_eq!(p1.whole_hours(), - 1);
        let p2 = rug_fuzz_2.minutes();
        debug_assert_eq!(p2.whole_hours(), 0);
        let p3 = (-rug_fuzz_3).minutes();
        debug_assert_eq!(p3.whole_hours(), 0);
        let _rug_ed_tests_rug_281_rrrruuuugggg_test_whole_hours = 0;
    }
}
#[cfg(test)]
mod tests_rug_282 {
    use super::*;
    use crate::duration::Duration;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_282_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 0;
        let mut p0 = Duration::new(rug_fuzz_0, rug_fuzz_1);
        Duration::whole_minutes(p0);
        let _rug_ed_tests_rug_282_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_283 {
    use super::*;
    use crate::duration::Duration;
    #[test]
    fn test_duration_whole_seconds() {
        let _rug_st_tests_rug_283_rrrruuuugggg_test_duration_whole_seconds = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 0;
        let mut p0 = Duration::new(rug_fuzz_0, rug_fuzz_1);
        p0.whole_seconds();
        let _rug_ed_tests_rug_283_rrrruuuugggg_test_duration_whole_seconds = 0;
    }
}
#[cfg(test)]
mod tests_rug_284 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_as_seconds_f32() {
        let _rug_st_tests_rug_284_rrrruuuugggg_test_as_seconds_f32 = 0;
        let rug_fuzz_0 = 1.5;
        let rug_fuzz_1 = 1.5;
        let p0 = rug_fuzz_0.seconds();
        debug_assert_eq!(p0.as_seconds_f32(), 1.5);
        let p1 = (-rug_fuzz_1).seconds();
        debug_assert_eq!(p1.as_seconds_f32(), - 1.5);
        let _rug_ed_tests_rug_284_rrrruuuugggg_test_as_seconds_f32 = 0;
    }
}
#[cfg(test)]
mod tests_rug_285 {
    use super::*;
    use crate::duration::Duration;
    #[test]
    fn test_whole_milliseconds() {
        let _rug_st_tests_rug_285_rrrruuuugggg_test_whole_milliseconds = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 1_000_000;
        let rug_fuzz_6 = 0;
        let rug_fuzz_7 = 1_000_000;
        let mut p0 = Duration::new(rug_fuzz_0, rug_fuzz_1);
        debug_assert_eq!(p0.whole_milliseconds(), 10_000);
        let mut p1 = Duration::new(-rug_fuzz_2, rug_fuzz_3);
        debug_assert_eq!(p1.whole_milliseconds(), - 10_000);
        let mut p2 = Duration::new(rug_fuzz_4, rug_fuzz_5);
        debug_assert_eq!(p2.whole_milliseconds(), 1);
        let mut p3 = Duration::new(rug_fuzz_6, -rug_fuzz_7);
        debug_assert_eq!(p3.whole_milliseconds(), - 1);
        let _rug_ed_tests_rug_285_rrrruuuugggg_test_whole_milliseconds = 0;
    }
}
#[cfg(test)]
mod tests_rug_286 {
    use super::*;
    use crate::{Duration, prelude::*};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_286_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1_000_000;
        let rug_fuzz_1 = 1_000_000;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 1_000_000_000;
        let rug_fuzz_4 = 1_000_000_000;
        let nanoseconds = rug_fuzz_0;
        Duration::nanoseconds(nanoseconds);
        let nanoseconds_negative = -rug_fuzz_1;
        Duration::nanoseconds(nanoseconds_negative);
        let nanoseconds_zero = rug_fuzz_2;
        Duration::nanoseconds(nanoseconds_zero);
        let nanoseconds_large = rug_fuzz_3;
        Duration::nanoseconds(nanoseconds_large);
        let nanoseconds_large_negative = -rug_fuzz_4;
        Duration::nanoseconds(nanoseconds_large_negative);
        let _rug_ed_tests_rug_286_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_287 {
    use super::*;
    use crate::duration::Duration;
    #[test]
    fn test_nanoseconds_i128() {
        let _rug_st_tests_rug_287_rrrruuuugggg_test_nanoseconds_i128 = 0;
        let rug_fuzz_0 = 5000000000;
        let p0: i128 = rug_fuzz_0;
        Duration::nanoseconds_i128(p0);
        let _rug_ed_tests_rug_287_rrrruuuugggg_test_nanoseconds_i128 = 0;
    }
}
#[cfg(test)]
mod tests_rug_288 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_whole_nanoseconds() {
        let _rug_st_tests_rug_288_rrrruuuugggg_test_whole_nanoseconds = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 1;
        let p0 = rug_fuzz_0.microseconds();
        debug_assert_eq!(p0.whole_nanoseconds(), 1_000);
        let p1 = (-rug_fuzz_1).microseconds();
        debug_assert_eq!(p1.whole_nanoseconds(), - 1_000);
        let p2 = rug_fuzz_2.nanoseconds();
        debug_assert_eq!(p2.whole_nanoseconds(), 1);
        let p3 = (-rug_fuzz_3).nanoseconds();
        debug_assert_eq!(p3.whole_nanoseconds(), - 1);
        let _rug_ed_tests_rug_288_rrrruuuugggg_test_whole_nanoseconds = 0;
    }
}
#[cfg(test)]
mod tests_rug_289 {
    use super::*;
    use crate::duration::Duration;
    #[test]
    fn test_subsec_nanoseconds() {
        let _rug_st_tests_rug_289_rrrruuuugggg_test_subsec_nanoseconds = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 400_000_000;
        let rug_fuzz_4 = 1;
        let rug_fuzz_5 = 400_000_000;
        let p0 = Duration::new(rug_fuzz_0, rug_fuzz_1);
        debug_assert_eq!(p0.subsec_nanoseconds(), 0);
        let p1 = Duration::new(rug_fuzz_2, rug_fuzz_3);
        debug_assert_eq!(p1.subsec_nanoseconds(), 400_000_000);
        let p2 = Duration::new(-rug_fuzz_4, -rug_fuzz_5);
        debug_assert_eq!(p2.subsec_nanoseconds(), - 400_000_000);
        let _rug_ed_tests_rug_289_rrrruuuugggg_test_subsec_nanoseconds = 0;
    }
}
#[cfg(test)]
mod tests_rug_290 {
    use super::*;
    use crate::duration::Duration;
    #[test]
    fn test_checked_add() {
        let _rug_st_tests_rug_290_rrrruuuugggg_test_checked_add = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 20;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 1;
        let rug_fuzz_5 = 5;
        let rug_fuzz_6 = 0;
        let p0 = Duration::new(rug_fuzz_0, rug_fuzz_1);
        let p1 = Duration::new(rug_fuzz_2, rug_fuzz_3);
        debug_assert_eq!(Duration::checked_add(p0, p1), Some(Duration::new(30, 0)));
        let p2 = Duration::max_value();
        let p3 = Duration::nanoseconds(rug_fuzz_4);
        debug_assert_eq!(Duration::checked_add(p2, p3), None);
        let p4 = Duration::new(-rug_fuzz_5, rug_fuzz_6);
        debug_assert_eq!(Duration::checked_add(p4, p1), Some(Duration::zero()));
        let _rug_ed_tests_rug_290_rrrruuuugggg_test_checked_add = 0;
    }
}
#[cfg(test)]
mod tests_rug_291 {
    use super::*;
    use crate::duration::Duration;
    #[test]
    fn test_checked_mul() {
        let _rug_st_tests_rug_291_rrrruuuugggg_test_checked_mul = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 2;
        let mut p0 = Duration::new(rug_fuzz_0, rug_fuzz_1);
        let p1: i32 = rug_fuzz_2;
        debug_assert_eq!(p0.checked_mul(p1), Some(Duration::new(20, 0)));
        let _rug_ed_tests_rug_291_rrrruuuugggg_test_checked_mul = 0;
    }
}
#[cfg(test)]
mod tests_rug_292 {
    use super::*;
    use crate::duration::Duration;
    #[test]
    fn test_checked_div() {
        let _rug_st_tests_rug_292_rrrruuuugggg_test_checked_div = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 2;
        let mut p0 = Duration::new(rug_fuzz_0, rug_fuzz_1);
        let p1: i32 = rug_fuzz_2;
        Duration::checked_div(p0, p1);
        let _rug_ed_tests_rug_292_rrrruuuugggg_test_checked_div = 0;
    }
}
#[cfg(test)]
mod tests_rug_293 {
    use super::*;
    use std::time::Duration;
    use std::time::Instant;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_293_rrrruuuugggg_test_rug = 0;
        let mut p0 = || {};
        crate::duration::Duration::time_fn(p0);
        let _rug_ed_tests_rug_293_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_294 {
    use super::*;
    use crate::duration::Duration;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_294_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 0;
        let mut p0 = Duration::new(rug_fuzz_0, rug_fuzz_1);
        Duration::num_weeks(&p0);
        let _rug_ed_tests_rug_294_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_295 {
    use super::*;
    use crate::duration::Duration;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_295_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 0;
        let mut p0 = Duration::new(rug_fuzz_0, rug_fuzz_1);
        <Duration>::num_milliseconds(&p0);
        let _rug_ed_tests_rug_295_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_296 {
    use super::*;
    use crate::duration::Duration;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_296_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 0;
        let mut p0 = Duration::new(rug_fuzz_0, rug_fuzz_1);
        Duration::num_microseconds(&p0);
        let _rug_ed_tests_rug_296_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_297 {
    use super::*;
    use crate::duration::Duration;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_297_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 0;
        let mut p0 = Duration::new(rug_fuzz_0, rug_fuzz_1);
        Duration::num_nanoseconds(&p0);
        let _rug_ed_tests_rug_297_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_298 {
    use super::*;
    use crate::duration::Duration;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_298_rrrruuuugggg_test_rug = 0;
        let mut p0: fn() = || {};
        Duration::span(p0);
        let _rug_ed_tests_rug_298_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_299 {
    use super::*;
    use std::time::Duration;
    use crate::duration::Duration as TimeDuration;
    use crate::error::ConversionRange;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_299_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let mut p0: Duration = Duration::new(rug_fuzz_0, rug_fuzz_1);
        TimeDuration::from_std(p0);
        let _rug_ed_tests_rug_299_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_300 {
    use super::*;
    use crate::duration::Duration;
    #[test]
    #[allow(deprecated)]
    #[deprecated(
        since = "0.2.0",
        note = "Use `std::time::Duration::try_from(value)` or `value.try_into()`"
    )]
    fn test_to_std() {
        let _rug_st_tests_rug_300_rrrruuuugggg_test_to_std = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 0;
        let mut p0 = Duration::new(rug_fuzz_0, rug_fuzz_1);
        let result: Result<std::time::Duration, _> = p0.to_std();
        debug_assert!(result.is_ok());
        let _rug_ed_tests_rug_300_rrrruuuugggg_test_to_std = 0;
    }
}
#[cfg(test)]
mod tests_rug_304 {
    use super::*;
    use crate::duration::Duration;
    use std::ops::AddAssign;
    use std::time::Duration as StdDuration;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_304_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let mut p0 = Duration::new(rug_fuzz_0, rug_fuzz_1);
        let mut p1 = StdDuration::new(rug_fuzz_2, rug_fuzz_3);
        <Duration as std::ops::AddAssign<StdDuration>>::add_assign(&mut p0, p1);
        let _rug_ed_tests_rug_304_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_307 {
    use super::*;
    use crate::duration::Duration;
    use std::ops::Sub;
    use std::time::Duration as StdDuration;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_307_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let mut p0 = Duration::new(rug_fuzz_0, rug_fuzz_1);
        let mut p1 = StdDuration::new(rug_fuzz_2, rug_fuzz_3);
        p0.sub(p1);
        let _rug_ed_tests_rug_307_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_327 {
    use super::*;
    use crate::Duration;
    #[test]
    fn test_mul() {
        let _rug_st_tests_rug_327_rrrruuuugggg_test_mul = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 2.5;
        let mut p0 = Duration::new(rug_fuzz_0, rug_fuzz_1);
        let p1: f32 = rug_fuzz_2;
        p0.mul(p1);
        let _rug_ed_tests_rug_327_rrrruuuugggg_test_mul = 0;
    }
}
#[cfg(test)]
mod tests_rug_331 {
    use super::*;
    use crate::duration::Duration;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_331_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 2.5;
        let rug_fuzz_1 = 10;
        let rug_fuzz_2 = 0;
        let mut p0: f64 = rug_fuzz_0;
        let mut p1 = Duration::new(rug_fuzz_1, rug_fuzz_2);
        <f64>::mul(p0, p1);
        let _rug_ed_tests_rug_331_rrrruuuugggg_test_rug = 0;
    }
}
