//! The [`Duration`] struct and its associated `impl`s.

use core::cmp::Ordering;
use core::convert::{TryFrom, TryInto};
use core::fmt;
use core::iter::Sum;
use core::ops::{Add, Div, Mul, Neg, Sub, SubAssign};
use core::time::Duration as StdDuration;

use crate::error;
#[cfg(feature = "std")]
use crate::Instant;

/// By explicitly inserting this enum where padding is expected, the compiler is able to better
/// perform niche value optimization.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum Padding {
    #[allow(clippy::missing_docs_in_private_items)]
    Optimize,
}

impl Default for Padding {
    fn default() -> Self {
        Self::Optimize
    }
}

/// A span of time with nanosecond precision.
///
/// Each `Duration` is composed of a whole number of seconds and a fractional part represented in
/// nanoseconds.
///
/// This implementation allows for negative durations, unlike [`core::time::Duration`].
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Duration {
    /// Number of whole seconds.
    seconds: i64,
    /// Number of nanoseconds within the second. The sign always matches the `seconds` field.
    nanoseconds: i32, // always -10^9 < nanoseconds < 10^9
    #[allow(clippy::missing_docs_in_private_items)]
    padding: Padding,
}

impl fmt::Debug for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Duration")
            .field("seconds", &self.seconds)
            .field("nanoseconds", &self.nanoseconds)
            .finish()
    }
}

impl Duration {
    // region: constants
    /// Equivalent to `0.seconds()`.
    ///
    /// ```rust
    /// # use time::{Duration, ext::NumericalDuration};
    /// assert_eq!(Duration::ZERO, 0.seconds());
    /// ```
    pub const ZERO: Self = Self::seconds(0);

    /// Equivalent to `1.nanoseconds()`.
    ///
    /// ```rust
    /// # use time::{Duration, ext::NumericalDuration};
    /// assert_eq!(Duration::NANOSECOND, 1.nanoseconds());
    /// ```
    pub const NANOSECOND: Self = Self::nanoseconds(1);

    /// Equivalent to `1.microseconds()`.
    ///
    /// ```rust
    /// # use time::{Duration, ext::NumericalDuration};
    /// assert_eq!(Duration::MICROSECOND, 1.microseconds());
    /// ```
    pub const MICROSECOND: Self = Self::microseconds(1);

    /// Equivalent to `1.milliseconds()`.
    ///
    /// ```rust
    /// # use time::{Duration, ext::NumericalDuration};
    /// assert_eq!(Duration::MILLISECOND, 1.milliseconds());
    /// ```
    pub const MILLISECOND: Self = Self::milliseconds(1);

    /// Equivalent to `1.seconds()`.
    ///
    /// ```rust
    /// # use time::{Duration, ext::NumericalDuration};
    /// assert_eq!(Duration::SECOND, 1.seconds());
    /// ```
    pub const SECOND: Self = Self::seconds(1);

    /// Equivalent to `1.minutes()`.
    ///
    /// ```rust
    /// # use time::{Duration, ext::NumericalDuration};
    /// assert_eq!(Duration::MINUTE, 1.minutes());
    /// ```
    pub const MINUTE: Self = Self::minutes(1);

    /// Equivalent to `1.hours()`.
    ///
    /// ```rust
    /// # use time::{Duration, ext::NumericalDuration};
    /// assert_eq!(Duration::HOUR, 1.hours());
    /// ```
    pub const HOUR: Self = Self::hours(1);

    /// Equivalent to `1.days()`.
    ///
    /// ```rust
    /// # use time::{Duration, ext::NumericalDuration};
    /// assert_eq!(Duration::DAY, 1.days());
    /// ```
    pub const DAY: Self = Self::days(1);

    /// Equivalent to `1.weeks()`.
    ///
    /// ```rust
    /// # use time::{Duration, ext::NumericalDuration};
    /// assert_eq!(Duration::WEEK, 1.weeks());
    /// ```
    pub const WEEK: Self = Self::weeks(1);

    /// The minimum possible duration. Adding any negative duration to this will cause an overflow.
    pub const MIN: Self = Self::new_unchecked(i64::MIN, -999_999_999);

    /// The maximum possible duration. Adding any positive duration to this will cause an overflow.
    pub const MAX: Self = Self::new_unchecked(i64::MAX, 999_999_999);
    // endregion constants

    // region: is_{sign}
    /// Check if a duration is exactly zero.
    ///
    /// ```rust
    /// # use time::ext::NumericalDuration;
    /// assert!(0.seconds().is_zero());
    /// assert!(!1.nanoseconds().is_zero());
    /// ```
    pub const fn is_zero(self) -> bool {
        self.seconds == 0 && self.nanoseconds == 0
    }

    /// Check if a duration is negative.
    ///
    /// ```rust
    /// # use time::ext::NumericalDuration;
    /// assert!((-1).seconds().is_negative());
    /// assert!(!0.seconds().is_negative());
    /// assert!(!1.seconds().is_negative());
    /// ```
    pub const fn is_negative(self) -> bool {
        self.seconds < 0 || self.nanoseconds < 0
    }

    /// Check if a duration is positive.
    ///
    /// ```rust
    /// # use time::ext::NumericalDuration;
    /// assert!(1.seconds().is_positive());
    /// assert!(!0.seconds().is_positive());
    /// assert!(!(-1).seconds().is_positive());
    /// ```
    pub const fn is_positive(self) -> bool {
        self.seconds > 0 || self.nanoseconds > 0
    }
    // endregion is_{sign}

    // region: abs
    /// Get the absolute value of the duration.
    ///
    /// This method saturates the returned value if it would otherwise overflow.
    ///
    /// ```rust
    /// # use time::ext::NumericalDuration;
    /// assert_eq!(1.seconds().abs(), 1.seconds());
    /// assert_eq!(0.seconds().abs(), 0.seconds());
    /// assert_eq!((-1).seconds().abs(), 1.seconds());
    /// ```
    pub const fn abs(self) -> Self {
        Self::new_unchecked(self.seconds.saturating_abs(), self.nanoseconds.abs())
    }

    /// Convert the existing `Duration` to a `std::time::Duration` and its sign. This doesn't
    /// actually require the standard library, but is currently only used when it's enabled.
    #[allow(clippy::missing_const_for_fn)] // false positive
    #[cfg(feature = "std")]
    pub(crate) fn abs_std(self) -> StdDuration {
        StdDuration::new(self.seconds.unsigned_abs(), self.nanoseconds.unsigned_abs())
    }
    // endregion abs

    // region: constructors
    /// Create a new `Duration` without checking the validity of the components.
    pub(crate) const fn new_unchecked(seconds: i64, nanoseconds: i32) -> Self {
        Self {
            seconds,
            nanoseconds,
            padding: Padding::Optimize,
        }
    }

    /// Create a new `Duration` with the provided seconds and nanoseconds. If nanoseconds is at
    /// least ±10<sup>9</sup>, it will wrap to the number of seconds.
    ///
    /// ```rust
    /// # use time::{Duration, ext::NumericalDuration};
    /// assert_eq!(Duration::new(1, 0), 1.seconds());
    /// assert_eq!(Duration::new(-1, 0), (-1).seconds());
    /// assert_eq!(Duration::new(1, 2_000_000_000), 3.seconds());
    /// ```
    pub const fn new(mut seconds: i64, mut nanoseconds: i32) -> Self {
        seconds += nanoseconds as i64 / 1_000_000_000;
        nanoseconds %= 1_000_000_000;

        if seconds > 0 && nanoseconds < 0 {
            seconds -= 1;
            nanoseconds += 1_000_000_000;
        } else if seconds < 0 && nanoseconds > 0 {
            seconds += 1;
            nanoseconds -= 1_000_000_000;
        }

        Self::new_unchecked(seconds, nanoseconds)
    }

    /// Create a new `Duration` with the given number of weeks. Equivalent to
    /// `Duration::seconds(weeks * 604_800)`.
    ///
    /// ```rust
    /// # use time::{Duration, ext::NumericalDuration};
    /// assert_eq!(Duration::weeks(1), 604_800.seconds());
    /// ```
    pub const fn weeks(weeks: i64) -> Self {
        Self::seconds(weeks * 604_800)
    }

    /// Create a new `Duration` with the given number of days. Equivalent to
    /// `Duration::seconds(days * 86_400)`.
    ///
    /// ```rust
    /// # use time::{Duration, ext::NumericalDuration};
    /// assert_eq!(Duration::days(1), 86_400.seconds());
    /// ```
    pub const fn days(days: i64) -> Self {
        Self::seconds(days * 86_400)
    }

    /// Create a new `Duration` with the given number of hours. Equivalent to
    /// `Duration::seconds(hours * 3_600)`.
    ///
    /// ```rust
    /// # use time::{Duration, ext::NumericalDuration};
    /// assert_eq!(Duration::hours(1), 3_600.seconds());
    /// ```
    pub const fn hours(hours: i64) -> Self {
        Self::seconds(hours * 3_600)
    }

    /// Create a new `Duration` with the given number of minutes. Equivalent to
    /// `Duration::seconds(minutes * 60)`.
    ///
    /// ```rust
    /// # use time::{Duration, ext::NumericalDuration};
    /// assert_eq!(Duration::minutes(1), 60.seconds());
    /// ```
    pub const fn minutes(minutes: i64) -> Self {
        Self::seconds(minutes * 60)
    }

    /// Create a new `Duration` with the given number of seconds.
    ///
    /// ```rust
    /// # use time::{Duration, ext::NumericalDuration};
    /// assert_eq!(Duration::seconds(1), 1_000.milliseconds());
    /// ```
    pub const fn seconds(seconds: i64) -> Self {
        Self::new_unchecked(seconds, 0)
    }

    /// Creates a new `Duration` from the specified number of seconds represented as `f64`.
    ///
    /// ```rust
    /// # use time::{Duration, ext::NumericalDuration};
    /// assert_eq!(Duration::seconds_f64(0.5), 0.5.seconds());
    /// assert_eq!(Duration::seconds_f64(-0.5), -0.5.seconds());
    /// ```
    pub fn seconds_f64(seconds: f64) -> Self {
        Self::new_unchecked(seconds as _, ((seconds % 1.) * 1_000_000_000.) as _)
    }

    /// Creates a new `Duration` from the specified number of seconds represented as `f32`.
    ///
    /// ```rust
    /// # use time::{Duration, ext::NumericalDuration};
    /// assert_eq!(Duration::seconds_f32(0.5), 0.5.seconds());
    /// assert_eq!(Duration::seconds_f32(-0.5), (-0.5).seconds());
    /// ```
    pub fn seconds_f32(seconds: f32) -> Self {
        Self::new_unchecked(seconds as _, ((seconds % 1.) * 1_000_000_000.) as _)
    }

    /// Create a new `Duration` with the given number of milliseconds.
    ///
    /// ```rust
    /// # use time::{Duration, ext::NumericalDuration};
    /// assert_eq!(Duration::milliseconds(1), 1_000.microseconds());
    /// assert_eq!(Duration::milliseconds(-1), (-1_000).microseconds());
    /// ```
    pub const fn milliseconds(milliseconds: i64) -> Self {
        Self::new_unchecked(
            milliseconds / 1_000,
            ((milliseconds % 1_000) * 1_000_000) as _,
        )
    }

    /// Create a new `Duration` with the given number of microseconds.
    ///
    /// ```rust
    /// # use time::{Duration, ext::NumericalDuration};
    /// assert_eq!(Duration::microseconds(1), 1_000.nanoseconds());
    /// assert_eq!(Duration::microseconds(-1), (-1_000).nanoseconds());
    /// ```
    pub const fn microseconds(microseconds: i64) -> Self {
        Self::new_unchecked(
            microseconds / 1_000_000,
            ((microseconds % 1_000_000) * 1_000) as _,
        )
    }

    /// Create a new `Duration` with the given number of nanoseconds.
    ///
    /// ```rust
    /// # use time::{Duration, ext::NumericalDuration};
    /// assert_eq!(Duration::nanoseconds(1), 1.microseconds() / 1_000);
    /// assert_eq!(Duration::nanoseconds(-1), (-1).microseconds() / 1_000);
    /// ```
    pub const fn nanoseconds(nanoseconds: i64) -> Self {
        Self::new_unchecked(
            nanoseconds / 1_000_000_000,
            (nanoseconds % 1_000_000_000) as _,
        )
    }

    /// Create a new `Duration` with the given number of nanoseconds.
    ///
    /// As the input range cannot be fully mapped to the output, this should only be used where it's
    /// known to result in a valid value.
    pub(crate) const fn nanoseconds_i128(nanoseconds: i128) -> Self {
        Self::new_unchecked(
            (nanoseconds / 1_000_000_000) as _,
            (nanoseconds % 1_000_000_000) as _,
        )
    }
    // endregion constructors

    // region: getters
    /// Get the number of whole weeks in the duration.
    ///
    /// ```rust
    /// # use time::ext::NumericalDuration;
    /// assert_eq!(1.weeks().whole_weeks(), 1);
    /// assert_eq!((-1).weeks().whole_weeks(), -1);
    /// assert_eq!(6.days().whole_weeks(), 0);
    /// assert_eq!((-6).days().whole_weeks(), 0);
    /// ```
    pub const fn whole_weeks(self) -> i64 {
        self.whole_seconds() / 604_800
    }

    /// Get the number of whole days in the duration.
    ///
    /// ```rust
    /// # use time::ext::NumericalDuration;
    /// assert_eq!(1.days().whole_days(), 1);
    /// assert_eq!((-1).days().whole_days(), -1);
    /// assert_eq!(23.hours().whole_days(), 0);
    /// assert_eq!((-23).hours().whole_days(), 0);
    /// ```
    pub const fn whole_days(self) -> i64 {
        self.whole_seconds() / 86_400
    }

    /// Get the number of whole hours in the duration.
    ///
    /// ```rust
    /// # use time::ext::NumericalDuration;
    /// assert_eq!(1.hours().whole_hours(), 1);
    /// assert_eq!((-1).hours().whole_hours(), -1);
    /// assert_eq!(59.minutes().whole_hours(), 0);
    /// assert_eq!((-59).minutes().whole_hours(), 0);
    /// ```
    pub const fn whole_hours(self) -> i64 {
        self.whole_seconds() / 3_600
    }

    /// Get the number of whole minutes in the duration.
    ///
    /// ```rust
    /// # use time::ext::NumericalDuration;
    /// assert_eq!(1.minutes().whole_minutes(), 1);
    /// assert_eq!((-1).minutes().whole_minutes(), -1);
    /// assert_eq!(59.seconds().whole_minutes(), 0);
    /// assert_eq!((-59).seconds().whole_minutes(), 0);
    /// ```
    pub const fn whole_minutes(self) -> i64 {
        self.whole_seconds() / 60
    }

    /// Get the number of whole seconds in the duration.
    ///
    /// ```rust
    /// # use time::ext::NumericalDuration;
    /// assert_eq!(1.seconds().whole_seconds(), 1);
    /// assert_eq!((-1).seconds().whole_seconds(), -1);
    /// assert_eq!(1.minutes().whole_seconds(), 60);
    /// assert_eq!((-1).minutes().whole_seconds(), -60);
    /// ```
    pub const fn whole_seconds(self) -> i64 {
        self.seconds
    }

    /// Get the number of fractional seconds in the duration.
    ///
    /// ```rust
    /// # use time::ext::NumericalDuration;
    /// assert_eq!(1.5.seconds().as_seconds_f64(), 1.5);
    /// assert_eq!((-1.5).seconds().as_seconds_f64(), -1.5);
    /// ```
    pub fn as_seconds_f64(self) -> f64 {
        self.seconds as f64 + self.nanoseconds as f64 / 1_000_000_000.
    }

    /// Get the number of fractional seconds in the duration.
    ///
    /// ```rust
    /// # use time::ext::NumericalDuration;
    /// assert_eq!(1.5.seconds().as_seconds_f32(), 1.5);
    /// assert_eq!((-1.5).seconds().as_seconds_f32(), -1.5);
    /// ```
    pub fn as_seconds_f32(self) -> f32 {
        self.seconds as f32 + self.nanoseconds as f32 / 1_000_000_000.
    }

    /// Get the number of whole milliseconds in the duration.
    ///
    /// ```rust
    /// # use time::ext::NumericalDuration;
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
    /// # use time::ext::NumericalDuration;
    /// assert_eq!(1.4.seconds().subsec_milliseconds(), 400);
    /// assert_eq!((-1.4).seconds().subsec_milliseconds(), -400);
    /// ```
    // Allow the lint, as the value is guaranteed to be less than 1000.
    pub const fn subsec_milliseconds(self) -> i16 {
        (self.nanoseconds / 1_000_000) as _
    }

    /// Get the number of whole microseconds in the duration.
    ///
    /// ```rust
    /// # use time::ext::NumericalDuration;
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
    /// # use time::ext::NumericalDuration;
    /// assert_eq!(1.0004.seconds().subsec_microseconds(), 400);
    /// assert_eq!((-1.0004).seconds().subsec_microseconds(), -400);
    /// ```
    pub const fn subsec_microseconds(self) -> i32 {
        self.nanoseconds / 1_000
    }

    /// Get the number of nanoseconds in the duration.
    ///
    /// ```rust
    /// # use time::ext::NumericalDuration;
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
    /// The returned value will always be in the range `-1_000_000_000..1_000_000_000`.
    ///
    /// ```rust
    /// # use time::ext::NumericalDuration;
    /// assert_eq!(1.000_000_400.seconds().subsec_nanoseconds(), 400);
    /// assert_eq!((-1.000_000_400).seconds().subsec_nanoseconds(), -400);
    /// ```
    pub const fn subsec_nanoseconds(self) -> i32 {
        self.nanoseconds
    }
    // endregion getters

    // region: checked arithmetic
    /// Computes `self + rhs`, returning `None` if an overflow occurred.
    ///
    /// ```rust
    /// # use time::{Duration, ext::NumericalDuration};
    /// assert_eq!(5.seconds().checked_add(5.seconds()), Some(10.seconds()));
    /// assert_eq!(Duration::MAX.checked_add(1.nanoseconds()), None);
    /// assert_eq!((-5).seconds().checked_add(5.seconds()), Some(0.seconds()));
    /// ```
    pub const fn checked_add(self, rhs: Self) -> Option<Self> {
        let mut seconds = const_try_opt!(self.seconds.checked_add(rhs.seconds));
        let mut nanoseconds = self.nanoseconds + rhs.nanoseconds;

        if nanoseconds >= 1_000_000_000 || seconds < 0 && nanoseconds > 0 {
            nanoseconds -= 1_000_000_000;
            seconds = const_try_opt!(seconds.checked_add(1));
        } else if nanoseconds <= -1_000_000_000 || seconds > 0 && nanoseconds < 0 {
            nanoseconds += 1_000_000_000;
            seconds = const_try_opt!(seconds.checked_sub(1));
        }

        Some(Self::new_unchecked(seconds, nanoseconds))
    }

    /// Computes `self - rhs`, returning `None` if an overflow occurred.
    ///
    /// ```rust
    /// # use time::{Duration, ext::NumericalDuration};
    /// assert_eq!(5.seconds().checked_sub(5.seconds()), Some(Duration::ZERO));
    /// assert_eq!(Duration::MIN.checked_sub(1.nanoseconds()), None);
    /// assert_eq!(5.seconds().checked_sub(10.seconds()), Some((-5).seconds()));
    /// ```
    pub const fn checked_sub(self, rhs: Self) -> Option<Self> {
        let mut seconds = const_try_opt!(self.seconds.checked_sub(rhs.seconds));
        let mut nanoseconds = self.nanoseconds - rhs.nanoseconds;

        if nanoseconds >= 1_000_000_000 || seconds < 0 && nanoseconds > 0 {
            nanoseconds -= 1_000_000_000;
            seconds = const_try_opt!(seconds.checked_add(1));
        } else if nanoseconds <= -1_000_000_000 || seconds > 0 && nanoseconds < 0 {
            nanoseconds += 1_000_000_000;
            seconds = const_try_opt!(seconds.checked_sub(1));
        }

        Some(Self::new_unchecked(seconds, nanoseconds))
    }

    /// Computes `self * rhs`, returning `None` if an overflow occurred.
    ///
    /// ```rust
    /// # use time::{Duration, ext::NumericalDuration};
    /// assert_eq!(5.seconds().checked_mul(2), Some(10.seconds()));
    /// assert_eq!(5.seconds().checked_mul(-2), Some((-10).seconds()));
    /// assert_eq!(5.seconds().checked_mul(0), Some(0.seconds()));
    /// assert_eq!(Duration::MAX.checked_mul(2), None);
    /// assert_eq!(Duration::MIN.checked_mul(2), None);
    /// ```
    pub const fn checked_mul(self, rhs: i32) -> Option<Self> {
        // Multiply nanoseconds as i64, because it cannot overflow that way.
        let total_nanos = self.nanoseconds as i64 * rhs as i64;
        let extra_secs = total_nanos / 1_000_000_000;
        let nanoseconds = (total_nanos % 1_000_000_000) as _;
        let seconds = const_try_opt!(
            const_try_opt!(self.seconds.checked_mul(rhs as _)).checked_add(extra_secs)
        );

        Some(Self::new_unchecked(seconds, nanoseconds))
    }

    /// Computes `self / rhs`, returning `None` if `rhs == 0` or if the result would overflow.
    ///
    /// ```rust
    /// # use time::ext::NumericalDuration;
    /// assert_eq!(10.seconds().checked_div(2), Some(5.seconds()));
    /// assert_eq!(10.seconds().checked_div(-2), Some((-5).seconds()));
    /// assert_eq!(1.seconds().checked_div(0), None);
    /// ```
    pub const fn checked_div(self, rhs: i32) -> Option<Self> {
        let seconds = const_try_opt!(self.seconds.checked_div(rhs as i64));
        let carry = self.seconds - seconds * (rhs as i64);
        let extra_nanos = const_try_opt!((carry * 1_000_000_000).checked_div(rhs as i64));
        let nanoseconds = const_try_opt!(self.nanoseconds.checked_div(rhs)) + (extra_nanos as i32);

        Some(Self::new_unchecked(seconds, nanoseconds))
    }
    // endregion checked arithmetic

    // region: saturating arithmetic
    /// Computes `self + rhs`, saturating if an overflow occurred.
    ///
    /// ```rust
    /// # use time::{Duration, ext::NumericalDuration};
    /// assert_eq!(5.seconds().saturating_add(5.seconds()), 10.seconds());
    /// assert_eq!(Duration::MAX.saturating_add(1.nanoseconds()), Duration::MAX);
    /// assert_eq!(
    ///     Duration::MIN.saturating_add((-1).nanoseconds()),
    ///     Duration::MIN
    /// );
    /// assert_eq!((-5).seconds().saturating_add(5.seconds()), Duration::ZERO);
    /// ```
    pub const fn saturating_add(self, rhs: Self) -> Self {
        let (mut seconds, overflow) = self.seconds.overflowing_add(rhs.seconds);
        if overflow {
            if self.seconds > 0 {
                return Self::MAX;
            }
            return Self::MIN;
        }
        let mut nanoseconds = self.nanoseconds + rhs.nanoseconds;

        if nanoseconds >= 1_000_000_000 || seconds < 0 && nanoseconds > 0 {
            nanoseconds -= 1_000_000_000;
            seconds = match seconds.checked_add(1) {
                Some(seconds) => seconds,
                None => return Self::MAX,
            };
        } else if nanoseconds <= -1_000_000_000 || seconds > 0 && nanoseconds < 0 {
            nanoseconds += 1_000_000_000;
            seconds = match seconds.checked_sub(1) {
                Some(seconds) => seconds,
                None => return Self::MIN,
            };
        }

        Self::new_unchecked(seconds, nanoseconds)
    }

    /// Computes `self - rhs`, saturating if an overflow occurred.
    ///
    /// ```rust
    /// # use time::{Duration, ext::NumericalDuration};
    /// assert_eq!(5.seconds().saturating_sub(5.seconds()), Duration::ZERO);
    /// assert_eq!(Duration::MIN.saturating_sub(1.nanoseconds()), Duration::MIN);
    /// assert_eq!(
    ///     Duration::MAX.saturating_sub((-1).nanoseconds()),
    ///     Duration::MAX
    /// );
    /// assert_eq!(5.seconds().saturating_sub(10.seconds()), (-5).seconds());
    /// ```
    pub const fn saturating_sub(self, rhs: Self) -> Self {
        let (mut seconds, overflow) = self.seconds.overflowing_sub(rhs.seconds);
        if overflow {
            if self.seconds > 0 {
                return Self::MAX;
            }
            return Self::MIN;
        }
        let mut nanoseconds = self.nanoseconds - rhs.nanoseconds;

        if nanoseconds >= 1_000_000_000 || seconds < 0 && nanoseconds > 0 {
            nanoseconds -= 1_000_000_000;
            seconds = match seconds.checked_add(1) {
                Some(seconds) => seconds,
                None => return Self::MAX,
            };
        } else if nanoseconds <= -1_000_000_000 || seconds > 0 && nanoseconds < 0 {
            nanoseconds += 1_000_000_000;
            seconds = match seconds.checked_sub(1) {
                Some(seconds) => seconds,
                None => return Self::MIN,
            };
        }

        Self::new_unchecked(seconds, nanoseconds)
    }

    /// Computes `self * rhs`, saturating if an overflow occurred.
    ///
    /// ```rust
    /// # use time::{Duration, ext::NumericalDuration};
    /// assert_eq!(5.seconds().saturating_mul(2), 10.seconds());
    /// assert_eq!(5.seconds().saturating_mul(-2), (-10).seconds());
    /// assert_eq!(5.seconds().saturating_mul(0), Duration::ZERO);
    /// assert_eq!(Duration::MAX.saturating_mul(2), Duration::MAX);
    /// assert_eq!(Duration::MIN.saturating_mul(2), Duration::MIN);
    /// assert_eq!(Duration::MAX.saturating_mul(-2), Duration::MIN);
    /// assert_eq!(Duration::MIN.saturating_mul(-2), Duration::MAX);
    /// ```
    pub const fn saturating_mul(self, rhs: i32) -> Self {
        // Multiply nanoseconds as i64, because it cannot overflow that way.
        let total_nanos = self.nanoseconds as i64 * rhs as i64;
        let extra_secs = total_nanos / 1_000_000_000;
        let nanoseconds = (total_nanos % 1_000_000_000) as _;
        let (seconds, overflow1) = self.seconds.overflowing_mul(rhs as _);
        if overflow1 {
            if self.seconds > 0 && rhs > 0 || self.seconds < 0 && rhs < 0 {
                return Self::MAX;
            }
            return Self::MIN;
        }
        let (seconds, overflow2) = seconds.overflowing_add(extra_secs);
        if overflow2 {
            if self.seconds > 0 && rhs > 0 {
                return Self::MAX;
            }
            return Self::MIN;
        }

        Self::new_unchecked(seconds, nanoseconds)
    }
    // endregion saturating arithmetic

    /// Runs a closure, returning the duration of time it took to run. The return value of the
    /// closure is provided in the second part of the tuple.
    #[cfg(feature = "std")]
    pub fn time_fn<T>(f: impl FnOnce() -> T) -> (Self, T) {
        let start = Instant::now();
        let return_value = f();
        let end = Instant::now();

        (end - start, return_value)
    }
}

// region: trait impls
impl TryFrom<StdDuration> for Duration {
    type Error = error::ConversionRange;

    fn try_from(original: StdDuration) -> Result<Self, error::ConversionRange> {
        Ok(Self::new(
            original
                .as_secs()
                .try_into()
                .map_err(|_| error::ConversionRange)?,
            original.subsec_nanos() as _,
        ))
    }
}

impl TryFrom<Duration> for StdDuration {
    type Error = error::ConversionRange;

    fn try_from(duration: Duration) -> Result<Self, error::ConversionRange> {
        Ok(Self::new(
            duration
                .seconds
                .try_into()
                .map_err(|_| error::ConversionRange)?,
            duration
                .nanoseconds
                .try_into()
                .map_err(|_| error::ConversionRange)?,
        ))
    }
}

impl Add for Duration {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        self.checked_add(rhs)
            .expect("overflow when adding durations")
    }
}

impl Add<StdDuration> for Duration {
    type Output = Self;

    fn add(self, std_duration: StdDuration) -> Self::Output {
        self + Self::try_from(std_duration)
            .expect("overflow converting `std::time::Duration` to `time::Duration`")
    }
}

impl Add<Duration> for StdDuration {
    type Output = Duration;

    fn add(self, rhs: Duration) -> Self::Output {
        rhs + self
    }
}

impl_add_assign!(Duration: Duration, StdDuration);

impl Neg for Duration {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new_unchecked(-self.seconds, -self.nanoseconds)
    }
}

impl Sub for Duration {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        self.checked_sub(rhs)
            .expect("overflow when subtracting durations")
    }
}

impl Sub<StdDuration> for Duration {
    type Output = Self;

    fn sub(self, rhs: StdDuration) -> Self::Output {
        self - Self::try_from(rhs)
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

impl_sub_assign!(Duration: Duration, StdDuration);

impl SubAssign<Duration> for StdDuration {
    fn sub_assign(&mut self, rhs: Duration) {
        *self = (*self - rhs).try_into().expect(
            "Cannot represent a resulting duration in std. Try `let x = x - rhs;`, which will \
             change the type.",
        );
    }
}

/// Implement `Mul` (reflexively) and `Div` for `Duration` for various types.
macro_rules! duration_mul_div_int {
    ($($type:ty),+) => {$(
        impl Mul<$type> for Duration {
            type Output = Self;

            fn mul(self, rhs: $type) -> Self::Output {
                Self::nanoseconds_i128(
                    self.whole_nanoseconds()
                        .checked_mul(rhs as _)
                        .expect("overflow when multiplying duration")
                )
            }
        }

        impl Mul<Duration> for $type {
            type Output = Duration;

            fn mul(self, rhs: Duration) -> Self::Output {
                rhs * self
            }
        }

        impl Div<$type> for Duration {
            type Output = Self;

            fn div(self, rhs: $type) -> Self::Output {
                Self::nanoseconds_i128(self.whole_nanoseconds() / rhs as i128)
            }
        }
    )+};
}
duration_mul_div_int![i8, i16, i32, u8, u16, u32];

impl Mul<f32> for Duration {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::seconds_f32(self.as_seconds_f32() * rhs)
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

impl Mul<Duration> for f64 {
    type Output = Duration;

    fn mul(self, rhs: Duration) -> Self::Output {
        rhs * self
    }
}

impl_mul_assign!(Duration: i8, i16, i32, u8, u16, u32, f32, f64);

impl Div<f32> for Duration {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self::seconds_f32(self.as_seconds_f32() / rhs)
    }
}

impl Div<f64> for Duration {
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        Self::seconds_f64(self.as_seconds_f64() / rhs)
    }
}

impl_div_assign!(Duration: i8, i16, i32, u8, u16, u32, f32, f64);

impl Div for Duration {
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

impl PartialOrd<StdDuration> for Duration {
    fn partial_cmp(&self, rhs: &StdDuration) -> Option<Ordering> {
        if rhs.as_secs() > i64::MAX as _ {
            return Some(Ordering::Less);
        }

        Some(
            self.seconds
                .cmp(&(rhs.as_secs() as _))
                .then_with(|| self.nanoseconds.cmp(&(rhs.subsec_nanos() as _))),
        )
    }
}

impl PartialOrd<Duration> for StdDuration {
    fn partial_cmp(&self, rhs: &Duration) -> Option<Ordering> {
        rhs.partial_cmp(self).map(Ordering::reverse)
    }
}

impl Sum for Duration {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.reduce(|a, b| a + b).unwrap_or_default()
    }
}

impl<'a> Sum<&'a Self> for Duration {
    fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        iter.copied().sum()
    }
}
// endregion trait impls

#[cfg(test)]
mod tests_llm_16_13 {
    use super::*;

use crate::*;
    use std::cmp::Ordering;

    #[test]
    fn test_partial_cmp() {
        let duration_1 = Duration::new(1, 0);
        let duration_2 = Duration::new(2, 0);
        let std_duration_1 = StdDuration::new(1, 0);
        let std_duration_2 = StdDuration::new(2, 0);

        assert_eq!(
            duration_1.partial_cmp(&std_duration_1),
            Some(Ordering::Equal)
        );
        assert_eq!(
            duration_1.partial_cmp(&std_duration_2),
            Some(Ordering::Equal)
        );
        assert_eq!(
            duration_2.partial_cmp(&std_duration_1),
            Some(Ordering::Equal)
        );
        assert_eq!(
            duration_2.partial_cmp(&std_duration_2),
            Some(Ordering::Equal)
        );
    }
}#[cfg(test)]
mod tests_llm_16_19 {
    use super::*;

use crate::*;

    #[test]
    fn test_sum() {
        let durations = vec![Duration::seconds(1), Duration::seconds(2), Duration::seconds(3)];
        let expected = Duration::seconds(6);
        let result: Duration = durations.into_iter().sum();
        assert_eq!(result, expected);
    }
}#[cfg(test)]
mod tests_llm_16_28 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_div() {
        assert_eq!(
            <duration::Duration as std::ops::Div<f32>>::div(Duration::seconds(10), 2.0),
            Duration::seconds_f32(5.0)
        );
        assert_eq!(
            <duration::Duration as std::ops::Div<f32>>::div(Duration::seconds(5), 0.5),
            Duration::seconds_f32(10.0)
        );
        assert_eq!(
            <duration::Duration as std::ops::Div<f32>>::div(Duration::seconds(0), 2.0),
            Duration::ZERO
        );
        assert_eq!(
            <duration::Duration as std::ops::Div<f32>>::div(Duration::seconds(10), 0.0),
            Duration::ZERO
        );
    }
}#[cfg(test)]
mod tests_llm_16_29 {
    use super::*;

use crate::*;
    use std::ops::Div;

    #[test]
    fn test_div_duration_by_f64() {
        let duration = Duration::seconds(10);
        let rhs = 2.5;
        let expected = Duration::seconds(4);
        let result = duration.div(rhs);
        assert_eq!(result, expected);
    }
}#[cfg(test)]
mod tests_llm_16_31_llm_16_30 {
    use super::*;

use crate::*;


    #[test]
    fn div_test() {
        let duration1 = Duration::seconds(10);
        let duration2 = Duration::seconds(2);
        let result: f64 = <Duration as std::ops::Div<Duration>>::div(duration1, duration2);
        let expected_result = Duration::seconds(5);
        assert_eq!(Duration::nanoseconds_i128(result as i128), expected_result);
    }
}#[cfg(test)]
mod tests_llm_16_41 {
    use std::cmp::Ordering;
    use std::convert::TryFrom;
    use crate::ext::NumericalDuration;
    use crate::Duration;

    #[test]
    fn test_div() {
        let duration1 = Duration::seconds(10);
        let duration2 = Duration::seconds(2);
        let result = duration1 / duration2;
        assert_eq!(result, 5.0);
    }
}#[cfg(test)]
mod tests_llm_16_43 {
    use super::*;

use crate::*;
    use std::convert::TryInto;

    #[test]
    fn test_div() {
        // Test case 1
        let duration1 = Duration::new(10, 0);
        let rhs1: u8 = 2;
        let expected1 = Duration::nanoseconds_i128(10_000_000_000 / rhs1 as i128);
        let result1 = duration1.div(rhs1);
        assert_eq!(result1, expected1);

        // Test case 2
        let duration2 = Duration::new(5, 0);
        let rhs2: u8 = 1;
        let expected2 = Duration::nanoseconds_i128(5_000_000_000 / rhs2 as i128);
        let result2 = duration2.div(rhs2);
        assert_eq!(result2, expected2);
    }
}#[cfg(test)]
mod tests_llm_16_44 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_div() {
        let duration1 = Duration::seconds(10);
        let duration2 = Duration::seconds(2);
        assert_eq!(duration1.div(duration2), 5.0);
    }
}#[cfg(test)]
mod tests_llm_16_57 {
    use super::*;

use crate::*;
    use std::cmp::Ordering;

    #[test]
    fn test_mul() {
        let duration = Duration::seconds(2);
        let result = duration.mul(1.5);
        let expected = Duration::seconds_f64(2.0 * 1.5);
        assert_eq!(result, expected);
    }
}#[cfg(test)]
mod tests_llm_16_61_llm_16_60 {
    use super::*;

use crate::*;

    #[test]
    fn test_mul() {
        let duration = Duration::new(10, 0);
        let result = <Duration as std::ops::Mul<i16>>::mul(duration, 5);
        assert_eq!(result, Duration::new(50, 0));

        let duration = Duration::new(86400, 0);
        let result = <Duration as std::ops::Mul<i16>>::mul(duration, -2);
        assert_eq!(result, Duration::new(-172800, 0));

        let duration = Duration::new(1, 0);
        let result = <Duration as std::ops::Mul<i16>>::mul(duration, 0);
        assert_eq!(result, Duration::new(0, 0));

        let duration = Duration::MAX;
        let result = <Duration as std::ops::Mul<i16>>::mul(duration, 2);
        assert_eq!(result, Duration::MAX);

        let duration = Duration::MIN;
        let result = <Duration as std::ops::Mul<i16>>::mul(duration, 2);
        assert_eq!(result, Duration::MIN);
    }
}#[cfg(test)]
mod tests_llm_16_63_llm_16_62 {
    use crate::duration::Duration;
    use std::ops::Mul;
    use crate::ext::NumericalDuration;

    #[test]
    fn test_mul() {
        let duration = Duration::new(10, 500_000_000);
        let result = <Duration as Mul<i32>>::mul(duration, 2);
        assert_eq!(result.seconds, 21);
        assert_eq!(result.nanoseconds, 0);
    }
}#[cfg(test)]
mod tests_llm_16_64 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_mul() {
        let duration = Duration::seconds(10);
        let result = duration.mul(2);
        let expected = Duration::seconds(20);
        
        assert_eq!(result, expected);
    }
}#[cfg(test)]
mod tests_llm_16_65 {
    use super::*;

use crate::*;
    use std::convert::TryFrom;

    #[test]
    fn test_mul() {
        let duration = Duration::new(2, 500_000_000);
        let result = Duration::try_from(duration * 4).unwrap();
        assert_eq!(result.seconds, 10);
        assert_eq!(result.nanoseconds, 2_000_000_000);
    }

    #[test]
    #[should_panic(expected = "overflow when multiplying duration")]
    fn test_mul_panic() {
        let duration = Duration::new(9_000_000_000, 0);
        let _ = duration * 4;
    }
}#[cfg(test)]
mod tests_llm_16_66 {
    use super::*;

use crate::*;
    use std::convert::TryFrom;

    #[test]
    fn test_mul() {
        let duration = Duration::new(2, 500_000_000);
        let rhs = 3;
        let expected = Duration::new(7, 500_000_000);
        let result = duration.mul(rhs);
        assert_eq!(result, expected);
    }
}#[cfg(test)]
mod tests_llm_16_84 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_neg() {
        let duration = Duration::new(1, 500_000_000);
        let neg_duration = -duration;
        assert_eq!(neg_duration.seconds, -1);
        assert_eq!(neg_duration.nanoseconds, -500_000_000);
    }
}#[cfg(test)]
mod tests_llm_16_87 {
    use super::*;

use crate::*;

    #[test]
    fn test_sub() {
        let duration1 = Duration::new(1, 0);
        let duration2 = Duration::new(0, 500_000_000);
        let result = duration1.sub(duration2);
        assert_eq!(result, Duration::new(0, 500_000_000));
    }
}#[cfg(test)]
mod tests_llm_16_92 {
    use crate::duration::Padding;
    
    #[test]
    fn test_default() {
        let default_padding: Padding = Default::default();
        assert_eq!(default_padding, Padding::Optimize);
    }
}#[cfg(test)]
mod tests_llm_16_266_llm_16_265 {
    use super::*;

use crate::*;

    #[test]
    fn test_mul() {
        let duration = Duration::new(5, 0);
        let rhs = 2;
        assert_eq!(duration.mul(rhs), Duration::new(10, 0));
    }
}#[cfg(test)]
mod tests_llm_16_268_llm_16_267 {
    use super::*;

use crate::*;
    use duration::Duration;

    #[test]
    fn test_mul() {
        let duration = Duration::seconds(10);
        let rhs = 2;
        let expected = Duration::seconds(20);
        assert_eq!(duration * rhs, expected);
    }
}#[cfg(test)]
mod tests_llm_16_274_llm_16_273 {
    use crate::duration::*;

    #[test]
    fn test_mul() {
        let duration1 = Duration::new(5, 100);
        let duration2 = Duration::new(2, 200);
        assert_eq!(duration1 * 2, Duration::new(10, 200));
        assert_eq!(duration2 * 2, Duration::new(4, 400));

        let duration3 = Duration::new(10, 500);
        let duration4 = Duration::new(0, 800);
        assert_eq!(duration3 * 0, Duration::new(0, 0));
        assert_eq!(duration4 * 5, Duration::new(0, 4000));
    }
}#[cfg(test)]
mod tests_llm_16_280 {
    use std::convert::TryInto;
    use crate::duration::Duration;
    use crate::ext::NumericalDuration;

    #[test]
    fn test_sub_assign() {
        let mut duration = Duration::seconds(10);
        duration -= Duration::seconds(5);
        let expected = Duration::seconds(5);
        assert_eq!(duration, expected);
    }
}#[cfg(test)]
mod tests_llm_16_289 {
  use crate::{
    duration::{Duration, Padding},
    ext::NumericalDuration,
  };
  use std::convert::TryFrom;

  #[test]
  fn test_checked_add() {
    assert_eq!(
      5.seconds().checked_add(5.seconds()),
      Some(10.seconds())
    );
    assert_eq!(
      Duration::MAX.checked_add(1.nanoseconds()),
      None
    );
    assert_eq!(
      (-5).seconds().checked_add(5.seconds()),
      Some(0.seconds())
    );
  }
}#[cfg(test)]
mod tests_llm_16_290 {
    use crate::duration::{Duration, Padding};
    use std::convert::TryFrom;

    #[test]
    fn test_checked_div() {
        assert_eq!(
            Duration::new_unchecked(10, 0).checked_div(2),
            Some(Duration::new_unchecked(5, 0))
        );
        assert_eq!(
            Duration::new_unchecked(10, 0).checked_div(-2),
            Some(Duration::new_unchecked(-5, 0))
        );
        assert_eq!(Duration::new_unchecked(1, 0).checked_div(0), None);
    }
}#[cfg(test)]
mod tests_llm_16_291 {
    use super::*;

use crate::*;
    use std::convert::TryInto;

    #[test]
    fn test_checked_mul() {
        assert_eq!(
            Duration::seconds(5).checked_mul(2),
            Some(Duration::seconds(10))
        );
        assert_eq!(
            Duration::seconds(5).checked_mul(-2),
            Some(Duration::seconds(-10))
        );
        assert_eq!(Duration::seconds(5).checked_mul(0), Some(Duration::seconds(0)));
        assert_eq!(Duration::MAX.checked_mul(2), None);
        assert_eq!(Duration::MIN.checked_mul(2), None);
    }
}#[cfg(test)]
mod tests_llm_16_296 {
    use super::*;

use crate::*;
    use crate::duration::Duration;
    use crate::duration::Padding;

    #[test]
    fn test_hours() {
        assert_eq!(Duration::hours(1), Duration {
            seconds: 1 * 3_600,
            nanoseconds: 0,
            padding: Padding::Optimize
        });
    }
}#[cfg(test)]
mod tests_llm_16_297 {
    use super::*;

use crate::*;
    use duration::Duration;

    #[test]
    fn test_is_negative() {
        assert_eq!(Duration::new(-1, 0).is_negative(), true);
        assert_eq!(Duration::new(-1, 1).is_negative(), true);
        assert_eq!(Duration::new(0, -1).is_negative(), true);
        assert_eq!(Duration::new(-1, -1).is_negative(), true);
        assert_eq!(Duration::new(0, 0).is_negative(), false);
        assert_eq!(Duration::new(1, 0).is_negative(), false);
        assert_eq!(Duration::new(1, 1).is_negative(), false);
    }
}#[cfg(test)]
mod tests_llm_16_298 {
    use crate::duration::*;

    #[test]
    fn test_is_positive() {
        assert!(Duration::new(1, 0).is_positive());
        assert!(Duration::new(0, 1).is_positive());
        assert!(Duration::new(1, 1).is_positive());
        assert!(!Duration::new(0, 0).is_positive());
        assert!(!Duration::new(-1, 0).is_positive());
        assert!(!Duration::new(0, -1).is_positive());
        assert!(!Duration::new(-1, -1).is_positive());
    }
}#[cfg(test)]
mod tests_llm_16_299 {
    use super::*;

use crate::*;
    use crate::ext::NumericalDuration;

    #[test]
    fn test_is_zero() {
        assert!(Duration::new(0, 0).is_zero());
        assert!(Duration::new(0, 1).is_zero());
        assert!(!Duration::new(1, 0).is_zero());
        assert!(!Duration::new(-1, 0).is_zero());
    }
}#[cfg(test)]
mod tests_llm_16_300 {
    use super::*;

use crate::*;

    #[test]
    fn test_microseconds() {
        assert_eq!(Duration::microseconds(1), Duration::nanoseconds(1_000));
        assert_eq!(Duration::microseconds(-1), Duration::nanoseconds(-1_000));
    }
}#[cfg(test)]
mod tests_llm_16_301 {
    use super::*;

use crate::*;
    use std::convert::TryFrom;

    #[test]
    fn test_milliseconds() {
        assert_eq!(
            Duration::milliseconds(1),
            Duration::new_unchecked(0, 1_000_000)
        );
        assert_eq!(
            Duration::milliseconds(-1),
            Duration::new_unchecked(0, -1_000_000)
        );
        assert_eq!(
            Duration::milliseconds(0),
            Duration::new_unchecked(0, 0)
        );
    }
}#[cfg(test)]
mod tests_llm_16_302 {
    use super::*;

use crate::*;

    #[test]
    fn test_minutes() {
        assert_eq!(Duration::minutes(1), Duration::seconds(60));
        assert_eq!(Duration::minutes(-1), Duration::seconds(-60));
        assert_eq!(Duration::minutes(0), Duration::seconds(0));
    }
}#[cfg(test)]
mod tests_llm_16_303 {
    use super::*;

use crate::*;
    use crate::duration::Padding;
    
    #[test]
    fn test_nanoseconds() {
        let duration = Duration::nanoseconds(1);
        assert_eq!(duration.seconds, 0);
        assert_eq!(duration.nanoseconds, 1);
        assert_eq!(duration.padding, Padding::Optimize);
        
        let duration = Duration::nanoseconds(-1);
        assert_eq!(duration.seconds, 0);
        assert_eq!(duration.nanoseconds, -1);
        assert_eq!(duration.padding, Padding::Optimize);
    }
}#[cfg(test)]
mod tests_llm_16_308 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_new_unchecked() {
        let duration = Duration::new_unchecked(1, 500_000_000);
        assert_eq!(duration.seconds, 1);
        assert_eq!(duration.nanoseconds, 500_000_000);
        assert_eq!(duration.padding, Padding::Optimize);
    }
}#[cfg(test)]
mod tests_llm_16_310_llm_16_309 {
    use super::*;

use crate::*;
    use crate::ext::NumericalDuration;

    #[test]
    fn test_saturating_add() {
        assert_eq!(Duration::seconds(5).saturating_add(Duration::seconds(5)), Duration::seconds(10));
        assert_eq!(Duration::MAX.saturating_add(Duration::nanoseconds(1)), Duration::MAX);
        assert_eq!(Duration::MIN.saturating_add(Duration::nanoseconds(-1)), Duration::MIN);
        assert_eq!(Duration::seconds(-5).saturating_add(Duration::seconds(5)), Duration::ZERO);
    }
}#[cfg(test)]
mod tests_llm_16_317 {
    use crate::duration::{Duration, Padding};

    #[test]
    fn test_seconds_f32() {
        assert_eq!(Duration::seconds_f32(0.5), Duration::new_unchecked(0, 500_000_000));
        assert_eq!(Duration::seconds_f32(-0.5), Duration::new_unchecked(0, -500_000_000));
    }
}#[cfg(test)]
mod tests_llm_16_318 {
    use super::*;

use crate::*;
    use crate::ext::NumericalDuration;

    #[test]
    fn test_seconds_f64() {
        assert_eq!(Duration::seconds_f64(0.5), 0.5.seconds());
        assert_eq!(Duration::seconds_f64(-0.5), -0.5.seconds());
    }
}#[cfg(test)]
mod tests_llm_16_319 {
    use super::*;

use crate::*;
    use crate::ext::NumericalDuration;

    #[test]
    fn test_subsec_microseconds() {
        assert_eq!(1.0004.seconds().subsec_microseconds(), 400);
        assert_eq!((-1.0004).seconds().subsec_microseconds(), -400);
    }
}#[cfg(test)]
mod tests_llm_16_320 {
    use super::*;

use crate::*;
    use std::convert::TryFrom;

    #[test]
    fn test_subsec_milliseconds() {
        assert_eq!(Duration::new(1, 400_000_000).subsec_milliseconds(), 400);
        assert_eq!(Duration::new(-1, -400_000_000).subsec_milliseconds(), -400);
    }
}#[cfg(test)]
mod tests_llm_16_321 {
    use super::*;

use crate::*;
    use crate::ext::NumericalDuration;

    #[test]
    fn test_subsec_nanoseconds() {
        assert_eq!(1.000_000_400.seconds().subsec_nanoseconds(), 400);
        assert_eq!((-1.000_000_400).seconds().subsec_nanoseconds(), -400);
    }
}#[cfg(test)]
mod tests_llm_16_326 {
    use super::*;

use crate::*;
    use crate::ext::NumericalDuration;
    
    #[test]
    fn test_whole_days() {
        assert_eq!(1.days().whole_days(), 1);
        assert_eq!((-1).days().whole_days(), -1);
        assert_eq!(23.hours().whole_days(), 0);
        assert_eq!((-23).hours().whole_days(), 0);
    }
}#[cfg(test)]
mod tests_llm_16_327 {
    use super::*;

use crate::*;
    use crate::duration;
    use crate::ext::NumericalDuration;

    #[test]
    fn test_whole_hours() {
        assert_eq!(1.hours().whole_hours(), 1);
        assert_eq!((-1).hours().whole_hours(), -1);
        assert_eq!(59.minutes().whole_hours(), 0);
        assert_eq!((-59).minutes().whole_hours(), 0);
    }
}#[cfg(test)]
mod tests_llm_16_328 {
    use super::*;

use crate::*;
    use crate::ext::NumericalDuration;

    #[test]
    fn test_whole_microseconds() {
        assert_eq!(1.milliseconds().whole_microseconds(), 1_000);
        assert_eq!((-1).milliseconds().whole_microseconds(), -1_000);
        assert_eq!(1.microseconds().whole_microseconds(), 1);
        assert_eq!((-1).microseconds().whole_microseconds(), -1);
    }
}#[cfg(test)]
mod tests_llm_16_329 {
    use super::*;

use crate::*;
    use crate::ext::NumericalDuration;

    #[test]
    fn test_whole_milliseconds_positive_seconds() {
        assert_eq!(1.seconds().whole_milliseconds(), 1_000);
        assert_eq!(2_386.seconds().whole_milliseconds(), 2_386_000);
    }

    #[test]
    fn test_whole_milliseconds_negative_seconds() {
        assert_eq!((-1).seconds().whole_milliseconds(), -1_000);
        assert_eq!((-2_386).seconds().whole_milliseconds(), -2_386_000);
    }

    #[test]
    fn test_whole_milliseconds_milliseconds() {
        assert_eq!(1.milliseconds().whole_milliseconds(), 1);
        assert_eq!(56.milliseconds().whole_milliseconds(), 56);
    }

    #[test]
    fn test_whole_milliseconds_negative_milliseconds() {
        assert_eq!((-1).milliseconds().whole_milliseconds(), -1);
        assert_eq!((-56).milliseconds().whole_milliseconds(), -56);
    }
}#[cfg(test)]
mod tests_llm_16_330 {
    use super::*;

use crate::*;
    use crate::ext::NumericalDuration;
    
    #[test]
    fn test_whole_minutes() {
        assert_eq!(1.minutes().whole_minutes(), 1);
        assert_eq!((-1).minutes().whole_minutes(), -1);
        assert_eq!(59.seconds().whole_minutes(), 0);
        assert_eq!((-59).seconds().whole_minutes(), 0);
    }
}#[cfg(test)]
mod tests_llm_16_331 {
    use super::*;

use crate::*;
    use std::convert::TryFrom;

    #[test]
    fn test_whole_nanoseconds() {
        assert_eq!(Duration::new(0, 0).whole_nanoseconds(), 0);
        assert_eq!(Duration::new(1, 0).whole_nanoseconds(), 1_000_000_000);
        assert_eq!(Duration::new(1, 1_000_000_000 - 1).whole_nanoseconds(), 1_999_999_999);
        assert_eq!(Duration::new(1, -1_000_000_000 + 1).whole_nanoseconds(), 1);
        assert_eq!(Duration::new(1, -1_000_000_000).whole_nanoseconds(), -1_000_000_000);
        assert_eq!(Duration::new(0, -1_000_000_000).whole_nanoseconds(), -1_000_000_000);
        assert_eq!(Duration::new(-1, 0).whole_nanoseconds(), -1_000_000_000);
        assert_eq!(Duration::new(-1, 1_000_000_000 - 1).whole_nanoseconds(), -1);
        assert_eq!(Duration::new(-1, -1_000_000_000 + 1).whole_nanoseconds(), -1_999_999_999);
        assert_eq!(Duration::new(-1, -1_000_000_000).whole_nanoseconds(), -1_000_000_000);
        assert_eq!(Duration::new(-1, -1_000_000_000 - 1).whole_nanoseconds(), -1_000_000_000 - 1);
    }
}#[cfg(test)]
mod tests_llm_16_334 {
    use super::*;

use crate::*;
    use crate::ext::NumericalDuration;

    #[test]
    fn test_whole_weeks() {
        assert_eq!(1.weeks().whole_weeks(), 1);
        assert_eq!((-1).weeks().whole_weeks(), -1);
        assert_eq!(6.days().whole_weeks(), 0);
        assert_eq!((-6).days().whole_weeks(), 0);
    }
}#[cfg(test)]
mod tests_rug_85 {
    use super::*;
    use crate::{Duration, ext::NumericalDuration};
    
    #[test]
    fn test_rug() {
        let mut p0 = Duration::new(5, 0);
        
        Duration::abs(p0);
    }
}#[cfg(test)]
mod tests_rug_86 {
    use super::*;
    use crate::{Duration, ext::NumericalDuration};

    #[test]
    fn test_rug() {
        let mut v3 = Duration::new(5, 0);
        crate::duration::Duration::abs_std(v3);

    }
}
#[cfg(test)]
mod tests_rug_87 {
    use super::*;
    use crate::{Duration, ext::NumericalDuration};

    #[test]
    fn test_rug() {
        let mut p0: i64 = 1;
        let mut p1: i32 = 2_000_000_000;

        <Duration>::new(p0, p1);
    }
}
#[cfg(test)]
mod tests_rug_88 {
    use super::*;
    use crate::{Duration, ext::NumericalDuration};

    #[test]
    fn test_weeks() {
        let p0: i64 = 2;  // Sample data
        Duration::weeks(p0);
    }
}#[cfg(test)]
mod tests_rug_89 {
    use super::*;
    use crate::{Duration, ext::NumericalDuration};

    #[test]
    fn test_days() {
        let p0: i64 = 10;
        
        Duration::days(p0);
    }
}#[cfg(test)]
mod tests_rug_90 {
    use super::*;
    use crate::{Duration, ext::NumericalDuration};

    #[test]
    fn test_rug() {
        let p0: i64 = 1;

        Duration::seconds(p0);
    }
}                        
#[cfg(test)]
mod tests_rug_91 {
    use super::*;
    use crate::duration::Duration;
    
    #[test]
    fn test_nanoseconds_i128() {
        let nanoseconds = 123456789i128;
        Duration::nanoseconds_i128(nanoseconds);
    }
}#[cfg(test)]
mod tests_rug_92 {
    use super::*;
    use crate::{Duration, ext::NumericalDuration};

    #[test]
    fn test_rug() {
        let mut p0 = Duration::new(5, 0);

        p0.whole_seconds();
    }
}#[cfg(test)]
mod tests_rug_93 {
    use super::*;
    use crate::{Duration, ext::NumericalDuration};

    #[test]
    fn test_rug() {
        let mut p0 = Duration::new(5, 0);

        p0.as_seconds_f64();
    }
}#[cfg(test)]
mod tests_rug_94 {
    use super::*;
    use crate::{Duration, ext::NumericalDuration};

    #[test]
    fn test_rug() {
        let mut p0 = Duration::new(5, 0);

        crate::duration::Duration::as_seconds_f32(p0);
    }
}#[cfg(test)]
mod tests_rug_95 {
    use super::*;
    use crate::{Duration, ext::NumericalDuration};

    #[test]
    fn test_checked_sub() {
        let mut p0 = Duration::new(5, 0);
        let mut p1 = Duration::new(5, 0);

        Duration::checked_sub(p0, p1);
    }
}#[cfg(test)]
mod tests_rug_96 {

    use super::*;
    use crate::{Duration};

    #[test]
    fn test_saturating_sub() {
        let mut p0 = Duration::new(5, 0);
        let mut p1 = Duration::new(5, 0);
        
        crate::duration::Duration::saturating_sub(p0, p1);
    }
}#[cfg(test)]
mod tests_rug_97 {
    use super::*;
    use crate::{Duration, ext::NumericalDuration};

    #[test]
    fn test_rug() {
        let mut p0 = Duration::new(5, 0);
        let p1 = 2;

        Duration::saturating_mul(p0, p1);
    }
}        
#[cfg(test)]

mod tests_rug_98 { 
    use super::*;
    use std::boxed::Box;
    use std::time::{Duration, Instant};
    
    #[test]
    fn test_rug() {
        let mut p0: Box<dyn FnOnce() -> i32> = Box::new(|| {
            // Your closure code here
            42
        });
        
        let _ = crate::duration::Duration::time_fn(p0);
    }
    
}
#[cfg(test)]
mod tests_rug_101 {
    use super::*;
    use crate::{Duration, ext::NumericalDuration};

    #[test]
    fn test_rug() {
        let mut p0 = Duration::new(5, 0);
        let mut p1 = Duration::new(3, 0);
        
        p0.add(p1);
    }
}         
#[cfg(test)]
mod tests_rug_104 {
    use super::*;
    use crate::{Duration, ext::NumericalDuration};

    #[test]
    fn test_sub() {
        let mut p0 = Duration::new(5, 0);
        let p1 = std::time::Duration::new(5, 0);
        
        p0.sub(p1);
    }
}
                            #[cfg(test)]
mod tests_rug_107 {
    use super::*;
    use crate::{Duration, ext::NumericalDuration};

    #[test]
    fn test_rug() {
        let mut p0 = Duration::new(5, 0);
        let mut p1: i8 = 2;

        p0.div(p1);
    }
}#[cfg(test)]
mod tests_rug_113 {
    use super::*;
    use crate::duration::Duration;
    
    #[test]
    fn test_rug() {
        let mut p1 = Duration::new(5, 0);
        let p0: f32 = 3.14;
        
        <f32>::mul(p0, p1);
    }
}#[cfg(test)]
mod tests_rug_114 {
    use super::*;
    use crate::{Duration, ext::NumericalDuration};

    #[test]
    fn test_rug() {
        let mut p0 = Duration::new(5, 0);
        let mut p1: f64 = 2.5;

        p0.mul(p1);
    }
}
#[cfg(test)]
mod tests_rug_120 {
    use super::*;
    use std::cmp::PartialOrd;
    use std::cmp::Ordering;
    use std::time::Duration;
    
    #[test]
    fn test_rug() {
        let v5 = Duration::new(5, 0); // Sample code to construct Duration

        let mut v3 = Duration::new(5, 0); // Sample code to construct duration::Duration

        v3.partial_cmp(&v5);
       
    }
}
