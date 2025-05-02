#[cfg(feature = "std")]
use crate::PrimitiveDateTime;
use crate::{
    error, format::{parse, parse::AmPm, ParsedItems},
    DeferredFormat, Duration, ParseResult,
};
#[cfg(not(feature = "std"))]
use alloc::{borrow::ToOwned, string::{String, ToString}};
use const_fn::const_fn;
use core::{
    cmp::Ordering, fmt::{self, Display},
    num::NonZeroU8, ops::{Add, AddAssign, Sub, SubAssign},
    time::Duration as StdDuration,
};
use standback::convert::TryFrom;
#[allow(unused_imports)]
use standback::prelude::*;
/// The number of nanoseconds in one day.
pub(crate) const NANOS_PER_DAY: u64 = 24 * 60 * 60 * 1_000_000_000;
/// The clock time within a given date. Nanosecond precision.
///
/// All minutes are assumed to have exactly 60 seconds; no attempt is made to
/// handle leap seconds (either positive or negative).
///
/// When comparing two `Time`s, they are assumed to be in the same calendar
/// date.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(from = "crate::serde::Time", into = "crate::serde::Time")
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Time {
    #[allow(clippy::missing_docs_in_private_items)]
    pub(crate) hour: u8,
    #[allow(clippy::missing_docs_in_private_items)]
    pub(crate) minute: u8,
    #[allow(clippy::missing_docs_in_private_items)]
    pub(crate) second: u8,
    #[allow(clippy::missing_docs_in_private_items)]
    pub(crate) nanosecond: u32,
}
impl Time {
    /// Create a `Time` that is exactly midnight.
    ///
    /// ```rust
    /// # use time::{Time, time};
    /// assert_eq!(Time::midnight(), time!(0:00));
    /// ```
    pub const fn midnight() -> Self {
        Time {
            hour: 0,
            minute: 0,
            second: 0,
            nanosecond: 0,
        }
    }
    /// Create a `Time` from the hour, minute, and second.
    ///
    /// ```rust
    /// # #![allow(deprecated)]
    /// # use time::Time;
    /// let time = Time::from_hms(1, 2, 3);
    /// assert_eq!(time.hour(), 1);
    /// assert_eq!(time.minute(), 2);
    /// assert_eq!(time.second(), 3);
    /// assert_eq!(time.nanosecond(), 0);
    /// ```
    ///
    /// Panics if any component is not valid.
    ///
    /// ```rust,should_panic
    /// # #![allow(deprecated)]
    /// # use time::Time;
    /// Time::from_hms(24, 0, 0); // 24 isn't a valid hour.
    /// ```
    ///
    /// ```rust,should_panic
    /// # #![allow(deprecated)]
    /// # use time::Time;
    /// Time::from_hms(0, 60, 0); // 60 isn't a valid minute.
    /// ```
    ///
    /// ```rust,should_panic
    /// # #![allow(deprecated)]
    /// # use time::Time;
    /// Time::from_hms(0, 0, 60); // 60 isn't a valid second.
    /// ```
    #[cfg(feature = "panicking-api")]
    #[cfg_attr(__time_02_docs, doc(cfg(feature = "panicking-api")))]
    #[deprecated(
        since = "0.2.3",
        note = "For times knowable at compile-time, use the `time!` macro. For situations where a \
                value isn't known, use `Time::try_from_hms`."
    )]
    pub fn from_hms(hour: u8, minute: u8, second: u8) -> Self {
        assert_value_in_range!(hour in 0 => 23);
        assert_value_in_range!(minute in 0 => 59);
        assert_value_in_range!(second in 0 => 59);
        Self {
            hour,
            minute,
            second,
            nanosecond: 0,
        }
    }
    /// Attempt to create a `Time` from the hour, minute, and second.
    ///
    /// ```rust
    /// # use time::Time;
    /// assert!(Time::try_from_hms(1, 2, 3).is_ok());
    /// ```
    ///
    /// Returns `None` if any component is not valid.
    ///
    /// ```rust
    /// # use time::Time;
    /// assert!(Time::try_from_hms(24, 0, 0).is_err()); // 24 isn't a valid hour.
    /// assert!(Time::try_from_hms(0, 60, 0).is_err()); // 60 isn't a valid minute.
    /// assert!(Time::try_from_hms(0, 0, 60).is_err()); // 60 isn't a valid second.
    /// ```
    ///
    /// This function is `const fn` when using rustc >= 1.46.
    #[const_fn("1.46")]
    pub const fn try_from_hms(
        hour: u8,
        minute: u8,
        second: u8,
    ) -> Result<Self, error::ComponentRange> {
        ensure_value_in_range!(hour in 0 => 23);
        ensure_value_in_range!(minute in 0 => 59);
        ensure_value_in_range!(second in 0 => 59);
        Ok(Self {
            hour,
            minute,
            second,
            nanosecond: 0,
        })
    }
    /// Create a `Time` from the hour, minute, second, and millisecond.
    ///
    /// ```rust
    /// # #![allow(deprecated)]
    /// # use time::Time;
    /// let time = Time::from_hms_milli(1, 2, 3, 4);
    /// assert_eq!(time.hour(), 1);
    /// assert_eq!(time.minute(), 2);
    /// assert_eq!(time.second(), 3);
    /// assert_eq!(time.millisecond(), 4);
    /// assert_eq!(time.nanosecond(), 4_000_000);
    /// ```
    ///
    /// Panics if any component is not valid.
    ///
    /// ```rust,should_panic
    /// # #![allow(deprecated)]
    /// # use time::Time;
    /// Time::from_hms_milli(24, 0, 0, 0); // 24 isn't a valid hour.
    /// ```
    ///
    /// ```rust,should_panic
    /// # #![allow(deprecated)]
    /// # use time::Time;
    /// Time::from_hms_milli(0, 60, 0, 0); // 60 isn't a valid minute.
    /// ```
    ///
    /// ```rust,should_panic
    /// # #![allow(deprecated)]
    /// # use time::Time;
    /// Time::from_hms_milli(0, 0, 60, 0); // 60 isn't a valid second.
    /// ```
    ///
    /// ```rust,should_panic
    /// # #![allow(deprecated)]
    /// # use time::Time;
    /// Time::from_hms_milli(0, 0, 0, 1_000); // 1_000 isn't a valid millisecond.
    /// ```
    #[cfg(feature = "panicking-api")]
    #[cfg_attr(__time_02_docs, doc(cfg(feature = "panicking-api")))]
    #[deprecated(
        since = "0.2.3",
        note = "For times knowable at compile-time, use the `time!` macro. For situations where a \
                value isn't known, use `Time::try_from_hms_milli`."
    )]
    pub fn from_hms_milli(hour: u8, minute: u8, second: u8, millisecond: u16) -> Self {
        assert_value_in_range!(hour in 0 => 23);
        assert_value_in_range!(minute in 0 => 59);
        assert_value_in_range!(second in 0 => 59);
        assert_value_in_range!(millisecond in 0 => 999);
        Self {
            hour,
            minute,
            second,
            nanosecond: millisecond as u32 * 1_000_000,
        }
    }
    /// Attempt to create a `Time` from the hour, minute, second, and millisecond.
    ///
    /// ```rust
    /// # use time::Time;
    /// assert!(Time::try_from_hms_milli(1, 2, 3, 4).is_ok());
    /// ```
    ///
    /// Returns `None` if any component is not valid.
    ///
    /// ```rust
    /// # use time::Time;
    /// assert!(Time::try_from_hms_milli(24, 0, 0, 0).is_err()); // 24 isn't a valid hour.
    /// assert!(Time::try_from_hms_milli(0, 60, 0, 0).is_err()); // 60 isn't a valid minute.
    /// assert!(Time::try_from_hms_milli(0, 0, 60, 0).is_err()); // 60 isn't a valid second.
    /// assert!(Time::try_from_hms_milli(0, 0, 0, 1_000).is_err()); // 1_000 isn't a valid millisecond.
    /// ```
    ///
    /// This function is `const fn` when using rustc >= 1.46.
    #[const_fn("1.46")]
    pub const fn try_from_hms_milli(
        hour: u8,
        minute: u8,
        second: u8,
        millisecond: u16,
    ) -> Result<Self, error::ComponentRange> {
        ensure_value_in_range!(hour in 0 => 23);
        ensure_value_in_range!(minute in 0 => 59);
        ensure_value_in_range!(second in 0 => 59);
        ensure_value_in_range!(millisecond in 0 => 999);
        Ok(Self {
            hour,
            minute,
            second,
            nanosecond: millisecond as u32 * 1_000_000,
        })
    }
    /// Create a `Time` from the hour, minute, second, and microsecond.
    ///
    /// ```rust
    /// # #![allow(deprecated)]
    /// # use time::Time;
    /// let time = Time::from_hms_micro(1, 2, 3, 4);
    /// assert_eq!(time.hour(), 1);
    /// assert_eq!(time.minute(), 2);
    /// assert_eq!(time.second(), 3);
    /// assert_eq!(time.microsecond(), 4);
    /// assert_eq!(time.nanosecond(), 4_000);
    /// ```
    ///
    /// Panics if any component is not valid.
    ///
    /// ```rust,should_panic
    /// # #![allow(deprecated)]
    /// # use time::Time;
    /// Time::from_hms_micro(24, 0, 0, 0); // 24 isn't a valid hour.
    /// ```
    ///
    /// ```rust,should_panic
    /// # #![allow(deprecated)]
    /// # use time::Time;
    /// Time::from_hms_micro(0, 60, 0, 0); // 60 isn't a valid minute.
    /// ```
    ///
    /// ```rust,should_panic
    /// # #![allow(deprecated)]
    /// # use time::Time;
    /// Time::from_hms_micro(0, 0, 60, 0); // 60 isn't a valid second.
    /// ```
    ///
    /// ```rust,should_panic
    /// # #![allow(deprecated)]
    /// # use time::Time;
    /// Time::from_hms_micro(0, 0, 0, 1_000_000); // 1_000_000 isn't a valid microsecond.
    /// ```
    #[cfg(feature = "panicking-api")]
    #[cfg_attr(__time_02_docs, doc(cfg(feature = "panicking-api")))]
    #[deprecated(
        since = "0.2.3",
        note = "For times knowable at compile-time, use the `time!` macro. For situations where a \
                value isn't known, use `Time::try_from_hms_micro`."
    )]
    pub fn from_hms_micro(hour: u8, minute: u8, second: u8, microsecond: u32) -> Self {
        assert_value_in_range!(hour in 0 => 23);
        assert_value_in_range!(minute in 0 => 59);
        assert_value_in_range!(second in 0 => 59);
        assert_value_in_range!(microsecond in 0 => 999_999);
        Self {
            hour,
            minute,
            second,
            nanosecond: microsecond * 1_000,
        }
    }
    /// Attempt to create a `Time` from the hour, minute, second, and microsecond.
    ///
    /// ```rust
    /// # use time::Time;
    /// assert!(Time::try_from_hms_micro(1, 2, 3, 4).is_ok());
    /// ```
    ///
    /// Returns `None` if any component is not valid.
    ///
    /// ```rust
    /// # use time::Time;
    /// assert!(Time::try_from_hms_micro(24, 0, 0, 0).is_err()); // 24 isn't a valid hour.
    /// assert!(Time::try_from_hms_micro(0, 60, 0, 0).is_err()); // 60 isn't a valid minute.
    /// assert!(Time::try_from_hms_micro(0, 0, 60, 0).is_err()); // 60 isn't a valid second.
    /// assert!(Time::try_from_hms_micro(0, 0, 0, 1_000_000).is_err()); // 1_000_000 isn't a valid microsecond.
    /// ```
    ///
    /// This function is `const fn` when using rustc >= 1.46.
    #[const_fn("1.46")]
    pub const fn try_from_hms_micro(
        hour: u8,
        minute: u8,
        second: u8,
        microsecond: u32,
    ) -> Result<Self, error::ComponentRange> {
        ensure_value_in_range!(hour in 0 => 23);
        ensure_value_in_range!(minute in 0 => 59);
        ensure_value_in_range!(second in 0 => 59);
        ensure_value_in_range!(microsecond in 0 => 999_999);
        Ok(Self {
            hour,
            minute,
            second,
            nanosecond: microsecond * 1_000,
        })
    }
    /// Create a `Time` from the hour, minute, second, and nanosecond.
    ///
    /// ```rust
    /// # #![allow(deprecated)]
    /// # use time::Time;
    /// let time = Time::from_hms_nano(1, 2, 3, 4);
    /// assert_eq!(time.hour(), 1);
    /// assert_eq!(time.minute(), 2);
    /// assert_eq!(time.second(), 3);
    /// assert_eq!(time.nanosecond(), 4);
    /// ```
    ///
    /// Panics if any component is not valid.
    ///
    /// ```rust,should_panic
    /// # #![allow(deprecated)]
    /// # use time::Time;
    /// Time::from_hms_nano(24, 0, 0, 0); // 24 isn't a valid hour.
    /// ```
    ///
    /// ```rust,should_panic
    /// # #![allow(deprecated)]
    /// # use time::Time;
    /// Time::from_hms_nano(0, 60, 0, 0); // 60 isn't a valid minute.
    /// ```
    ///
    /// ```rust,should_panic
    /// # #![allow(deprecated)]
    /// # use time::Time;
    /// Time::from_hms_nano(0, 0, 60, 0); // 60 isn't a valid second.
    /// ```
    ///
    /// ```rust,should_panic
    /// # #![allow(deprecated)]
    /// # use time::Time;
    /// Time::from_hms_nano(0, 0, 0, 1_000_000_000); // 1_000_000_000 isn't a valid nanosecond.
    /// ```
    #[cfg(feature = "panicking-api")]
    #[cfg_attr(__time_02_docs, doc(cfg(feature = "panicking-api")))]
    #[deprecated(
        since = "0.2.3",
        note = "For times knowable at compile-time, use the `time!` macro. For situations where a \
                value isn't known, use `Time::try_from_hms_nano`."
    )]
    pub fn from_hms_nano(hour: u8, minute: u8, second: u8, nanosecond: u32) -> Self {
        assert_value_in_range!(hour in 0 => 23);
        assert_value_in_range!(minute in 0 => 59);
        assert_value_in_range!(second in 0 => 59);
        assert_value_in_range!(nanosecond in 0 => 999_999_999);
        Self {
            hour,
            minute,
            second,
            nanosecond,
        }
    }
    /// Attempt to create a `Time` from the hour, minute, second, and nanosecond.
    ///
    /// ```rust
    /// # use time::Time;
    /// assert!(Time::try_from_hms_nano(1, 2, 3, 4).is_ok());
    /// ```
    ///
    /// Returns `None` if any component is not valid.
    ///
    /// ```rust
    /// # use time::Time;
    /// assert!(Time::try_from_hms_nano(24, 0, 0, 0).is_err()); // 24 isn't a valid hour.
    /// assert!(Time::try_from_hms_nano(0, 60, 0, 0).is_err()); // 60 isn't a valid minute.
    /// assert!(Time::try_from_hms_nano(0, 0, 60, 0).is_err()); // 60 isn't a valid second.
    /// assert!(Time::try_from_hms_nano(0, 0, 0, 1_000_000_000).is_err()); // 1_000_000_000 isn't a valid nanosecond.
    /// ```
    ///
    /// This function is `const fn` when using rustc >= 1.46.
    #[const_fn("1.46")]
    pub const fn try_from_hms_nano(
        hour: u8,
        minute: u8,
        second: u8,
        nanosecond: u32,
    ) -> Result<Self, error::ComponentRange> {
        ensure_value_in_range!(hour in 0 => 23);
        ensure_value_in_range!(minute in 0 => 59);
        ensure_value_in_range!(second in 0 => 59);
        ensure_value_in_range!(nanosecond in 0 => 999_999_999);
        Ok(Self {
            hour,
            minute,
            second,
            nanosecond,
        })
    }
    /// Create a `Time` representing the current time (UTC).
    ///
    /// ```rust
    /// # #![allow(deprecated)]
    /// # use time::Time;
    /// println!("{:?}", Time::now());
    /// ```
    #[cfg(feature = "std")]
    #[cfg_attr(__time_02_docs, doc(cfg(feature = "std")))]
    #[deprecated(
        since = "0.2.7",
        note = "This method returns a value that assumes an offset of UTC."
    )]
    #[allow(deprecated)]
    pub fn now() -> Self {
        PrimitiveDateTime::now().time()
    }
    /// Get the clock hour.
    ///
    /// The returned value will always be in the range `0..24`.
    ///
    /// ```rust
    /// # use time::time;
    /// assert_eq!(time!(0:00:00).hour(), 0);
    /// assert_eq!(time!(23:59:59).hour(), 23);
    /// ```
    pub const fn hour(self) -> u8 {
        self.hour
    }
    /// Get the minute within the hour.
    ///
    /// The returned value will always be in the range `0..60`.
    ///
    /// ```rust
    /// # use time::time;
    /// assert_eq!(time!(0:00:00).minute(), 0);
    /// assert_eq!(time!(23:59:59).minute(), 59);
    /// ```
    pub const fn minute(self) -> u8 {
        self.minute
    }
    /// Get the second within the minute.
    ///
    /// The returned value will always be in the range `0..60`.
    ///
    /// ```rust
    /// # use time::time;
    /// assert_eq!(time!(0:00:00).second(), 0);
    /// assert_eq!(time!(23:59:59).second(), 59);
    /// ```
    pub const fn second(self) -> u8 {
        self.second
    }
    /// Get the milliseconds within the second.
    ///
    /// The returned value will always be in the range `0..1_000`.
    ///
    /// ```rust
    /// # use time::time;
    /// assert_eq!(time!(0:00).millisecond(), 0);
    /// assert_eq!(time!(23:59:59.999).millisecond(), 999);
    /// ```
    pub const fn millisecond(self) -> u16 {
        (self.nanosecond() / 1_000_000) as u16
    }
    /// Get the microseconds within the second.
    ///
    /// The returned value will always be in the range `0..1_000_000`.
    ///
    /// ```rust
    /// # use time::time;
    /// assert_eq!(time!(0:00).microsecond(), 0);
    /// assert_eq!(time!(23:59:59.999_999).microsecond(), 999_999);
    /// ```
    pub const fn microsecond(self) -> u32 {
        self.nanosecond() / 1_000
    }
    /// Get the nanoseconds within the second.
    ///
    /// The returned value will always be in the range `0..1_000_000_000`.
    ///
    /// ```rust
    /// # use time::time;
    /// assert_eq!(time!(0:00).nanosecond(), 0);
    /// assert_eq!(time!(23:59:59.999_999_999).nanosecond(), 999_999_999);
    /// ```
    pub const fn nanosecond(self) -> u32 {
        self.nanosecond
    }
    /// Get the number of nanoseconds since midnight.
    pub(crate) const fn nanoseconds_since_midnight(self) -> u64 {
        self.hour() as u64 * 60 * 60 * 1_000_000_000
            + self.minute() as u64 * 60 * 1_000_000_000
            + self.second() as u64 * 1_000_000_000 + self.nanosecond() as u64
    }
    /// Create a `Time` from the number of nanoseconds since midnight.
    pub(crate) const fn from_nanoseconds_since_midnight(nanosecond: u64) -> Self {
        Self {
            hour: (nanosecond / 1_000_000_000 / 60 / 60 % 24) as u8,
            minute: (nanosecond / 1_000_000_000 / 60 % 60) as u8,
            second: (nanosecond / 1_000_000_000 % 60) as u8,
            nanosecond: (nanosecond % 1_000_000_000) as u32,
        }
    }
}
/// Methods that allow formatting the `Time`.
impl Time {
    /// Format the `Time` using the provided string.
    ///
    /// ```rust
    /// # use time::time;
    /// assert_eq!(time!(0:00).format("%r"), "12:00:00 am");
    /// ```
    pub fn format(self, format: impl AsRef<str>) -> String {
        self.lazy_format(format).to_string()
    }
    /// Format the `Time` using the provided string.
    ///
    /// ```rust
    /// # use time::time;
    /// assert_eq!(time!(0:00).lazy_format("%r").to_string(), "12:00:00 am");
    /// ```
    pub fn lazy_format(self, format: impl AsRef<str>) -> impl Display {
        DeferredFormat::new(format.as_ref()).with_time(self).to_owned()
    }
    /// Attempt to parse a `Time` using the provided string.
    ///
    /// ```rust
    /// # use time::{Time, time};
    /// assert_eq!(
    ///     Time::parse("0:00:00", "%T"),
    ///     Ok(time!(0:00))
    /// );
    /// assert_eq!(
    ///     Time::parse("23:59:59", "%T"),
    ///     Ok(time!(23:59:59))
    /// );
    /// assert_eq!(
    ///     Time::parse("12:00:00 am", "%r"),
    ///     Ok(time!(0:00))
    /// );
    /// assert_eq!(
    ///     Time::parse("12:00:00 pm", "%r"),
    ///     Ok(time!(12:00))
    /// );
    /// assert_eq!(
    ///     Time::parse("11:59:59 pm", "%r"),
    ///     Ok(time!(23:59:59))
    /// );
    /// ```
    pub fn parse(s: impl AsRef<str>, format: impl AsRef<str>) -> ParseResult<Self> {
        Self::try_from_parsed_items(parse(s.as_ref(), &format.into())?)
    }
    /// Given the items already parsed, attempt to create a `Time`.
    pub(crate) fn try_from_parsed_items(items: ParsedItems) -> ParseResult<Self> {
        macro_rules! items {
            ($($item:ident),* $(,)?) => {
                ParsedItems { $($item : Some($item)),*, .. }
            };
        }
        /// Convert a 12-hour time to a 24-hour time.
        #[allow(clippy::missing_const_for_fn)]
        fn hour_12_to_24(hour: NonZeroU8, am_pm: AmPm) -> u8 {
            use AmPm::{AM, PM};
            match (hour.get(), am_pm) {
                (12, AM) => 0,
                (12, PM) => 12,
                (h, AM) => h,
                (h, PM) => h + 12,
            }
        }
        match items {
            items!(hour_24, minute, second, nanosecond) => {
                Self::try_from_hms_nano(hour_24, minute, second, nanosecond)
                    .map_err(Into::into)
            }
            items!(hour_12, minute, second, nanosecond, am_pm) => {
                Self::try_from_hms_nano(
                        hour_12_to_24(hour_12, am_pm),
                        minute,
                        second,
                        nanosecond,
                    )
                    .map_err(Into::into)
            }
            items!(hour_24, minute, second) => {
                Self::try_from_hms(hour_24, minute, second).map_err(Into::into)
            }
            items!(hour_12, minute, second, am_pm) => {
                Self::try_from_hms(hour_12_to_24(hour_12, am_pm), minute, second)
                    .map_err(Into::into)
            }
            items!(hour_24, minute) => {
                Self::try_from_hms(hour_24, minute, 0).map_err(Into::into)
            }
            items!(hour_12, minute, am_pm) => {
                Self::try_from_hms(hour_12_to_24(hour_12, am_pm), minute, 0)
                    .map_err(Into::into)
            }
            items!(hour_24) => Self::try_from_hms(hour_24, 0, 0).map_err(Into::into),
            items!(hour_12, am_pm) => {
                Self::try_from_hms(hour_12_to_24(hour_12, am_pm), 0, 0)
                    .map_err(Into::into)
            }
            _ => Err(error::Parse::InsufficientInformation),
        }
    }
}
impl Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use crate::format::{time, Padding};
        time::fmt_H(f, *self, Padding::None)?;
        f.write_str(":")?;
        time::fmt_M(f, *self, Padding::Zero)?;
        if self.second != 0 || self.nanosecond != 0 {
            f.write_str(":")?;
            time::fmt_S(f, *self, Padding::Zero)?;
        }
        if self.nanosecond != 0 {
            f.write_str(".")?;
            if self.nanosecond % 1_000_000 == 0 {
                write!(f, "{:03}", self.nanosecond / 1_000_000)?;
            } else if self.nanosecond % 1_000 == 0 {
                write!(f, "{:06}", self.nanosecond / 1_000)?;
            } else {
                write!(f, "{:09}", self.nanosecond)?;
            }
        }
        Ok(())
    }
}
impl Add<Duration> for Time {
    type Output = Self;
    /// Add the sub-day time of the `Duration` to the `Time`. Wraps on overflow.
    ///
    /// ```rust
    /// # use time::prelude::*;
    /// # use time_macros::time;
    /// assert_eq!(time!(12:00) + 2.hours(), time!(14:00));
    /// assert_eq!(time!(0:00:01) + (-2).seconds(), time!(23:59:59));
    /// ```
    fn add(self, duration: Duration) -> Self::Output {
        Self::from_nanoseconds_since_midnight(
            self.nanoseconds_since_midnight()
                + duration.whole_nanoseconds().rem_euclid(NANOS_PER_DAY as i128) as u64,
        )
    }
}
impl Add<StdDuration> for Time {
    type Output = Self;
    /// Add the sub-day time of the `std::time::Duration` to the `Time`. Wraps
    /// on overflow.
    ///
    /// ```rust
    /// # use time::prelude::*;
    /// # use time_macros::time;
    /// assert_eq!(time!(12:00) + 2.std_hours(), time!(14:00));
    /// assert_eq!(time!(23:59:59) + 2.std_seconds(), time!(0:00:01));
    /// ```
    fn add(self, duration: StdDuration) -> Self::Output {
        self
            + Duration::try_from(duration)
                .expect("overflow converting `core::time::Duration` to `time::Duration`")
    }
}
impl AddAssign<Duration> for Time {
    /// Add the sub-day time of the `Duration` to the existing `Time`. Wraps on
    /// overflow.
    ///
    /// ```rust
    /// # use time::prelude::*;
    /// # use time_macros::time;
    /// let mut time = time!(12:00);
    /// time += 2.hours();
    /// assert_eq!(time, time!(14:00));
    ///
    /// let mut time = time!(0:00:01);
    /// time += (-2).seconds();
    /// assert_eq!(time, time!(23:59:59));
    /// ```
    fn add_assign(&mut self, duration: Duration) {
        *self = *self + duration;
    }
}
impl AddAssign<StdDuration> for Time {
    /// Add the sub-day time of the `std::time::Duration` to the existing
    /// `Time`. Wraps on overflow.
    ///
    /// ```rust
    /// # use time::prelude::*;
    /// # use time_macros::time;
    /// let mut time = time!(12:00);
    /// time += 2.std_hours();
    /// assert_eq!(time, time!(14:00));
    ///
    /// let mut time = time!(23:59:59);
    /// time += 2.std_seconds();
    /// assert_eq!(time, time!(0:00:01));
    /// ```
    fn add_assign(&mut self, duration: StdDuration) {
        *self = *self + duration;
    }
}
impl Sub<Duration> for Time {
    type Output = Self;
    /// Subtract the sub-day time of the `Duration` from the `Time`. Wraps on
    /// overflow.
    ///
    /// ```rust
    /// # use time::prelude::*;
    /// # use time_macros::time;
    /// assert_eq!(
    ///     time!(14:00) - 2.hours(),
    ///     time!(12:00)
    /// );
    /// assert_eq!(
    ///     time!(23:59:59) - (-2).seconds(),
    ///     time!(0:00:01)
    /// );
    /// ```
    fn sub(self, duration: Duration) -> Self::Output {
        self + -duration
    }
}
impl Sub<StdDuration> for Time {
    type Output = Self;
    /// Subtract the sub-day time of the `std::time::Duration` from the `Time`.
    /// Wraps on overflow.
    ///
    /// ```rust
    /// # use time::prelude::*;
    /// # use time_macros::time;
    /// assert_eq!(time!(14:00) - 2.std_hours(), time!(12:00));
    /// assert_eq!(time!(0:00:01) - 2.std_seconds(), time!(23:59:59));
    /// ```
    fn sub(self, duration: StdDuration) -> Self::Output {
        self
            - Duration::try_from(duration)
                .expect("overflow converting `core::time::Duration` to `time::Duration`")
    }
}
impl SubAssign<Duration> for Time {
    /// Subtract the sub-day time of the `Duration` from the existing `Time`.
    /// Wraps on overflow.
    ///
    /// ```rust
    /// # use time::prelude::*;
    /// # use time_macros::time;
    /// let mut time = time!(14:00);
    /// time -= 2.hours();
    /// assert_eq!(time, time!(12:00));
    ///
    /// let mut time = time!(23:59:59);
    /// time -= (-2).seconds();
    /// assert_eq!(time, time!(0:00:01));
    /// ```
    fn sub_assign(&mut self, duration: Duration) {
        *self = *self - duration;
    }
}
impl SubAssign<StdDuration> for Time {
    /// Subtract the sub-day time of the `std::time::Duration` from the existing
    /// `Time`. Wraps on overflow.
    ///
    /// ```rust
    /// # use time::prelude::*;
    /// # use time_macros::time;
    /// let mut time = time!(14:00);
    /// time -= 2.std_hours();
    /// assert_eq!(time, time!(12:00));
    ///
    /// let mut time = time!(0:00:01);
    /// time -= 2.std_seconds();
    /// assert_eq!(time, time!(23:59:59));
    /// ```
    fn sub_assign(&mut self, duration: StdDuration) {
        *self = *self - duration;
    }
}
impl Sub<Time> for Time {
    type Output = Duration;
    /// Subtract two `Time`s, returning the `Duration` between. This assumes
    /// both `Time`s are in the same calendar day.
    ///
    /// ```rust
    /// # use time::prelude::*;
    /// # use time_macros::time;
    /// assert_eq!(time!(0:00) - time!(0:00), 0.seconds());
    /// assert_eq!(time!(1:00) - time!(0:00), 1.hours());
    /// assert_eq!(time!(0:00) - time!(1:00), (-1).hours());
    /// assert_eq!(time!(0:00) - time!(23:00), (-23).hours());
    /// ```
    fn sub(self, rhs: Self) -> Self::Output {
        Duration::nanoseconds(
            self.nanoseconds_since_midnight() as i64
                - rhs.nanoseconds_since_midnight() as i64,
        )
    }
}
impl PartialOrd for Time {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.cmp(other).into()
    }
}
impl Ord for Time {
    fn cmp(&self, other: &Self) -> Ordering {
        self.hour
            .cmp(&other.hour)
            .then_with(|| self.minute.cmp(&other.minute))
            .then_with(|| self.second.cmp(&other.second))
            .then_with(|| self.nanosecond.cmp(&other.nanosecond))
    }
}
#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn nanoseconds_since_midnight() -> crate::Result<()> {
        let time = Time::midnight();
        assert_eq!(time.nanoseconds_since_midnight(), 0);
        assert_eq!(Time::from_nanoseconds_since_midnight(0), time);
        let time = Time::try_from_hms_nano(23, 59, 59, 999_999_999)?;
        assert_eq!(time.nanoseconds_since_midnight(), NANOS_PER_DAY - 1);
        assert_eq!(Time::from_nanoseconds_since_midnight(NANOS_PER_DAY - 1), time);
        Ok(())
    }
}
#[cfg(test)]
mod tests_llm_16_463 {
    use super::*;
    use crate::*;
    use std::cmp::Ordering;
    #[test]
    fn test_cmp() {
        let _rug_st_tests_llm_16_463_rrrruuuugggg_test_cmp = 0;
        let rug_fuzz_0 = 12;
        let rug_fuzz_1 = 30;
        let rug_fuzz_2 = 45;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 12;
        let rug_fuzz_5 = 30;
        let rug_fuzz_6 = 45;
        let rug_fuzz_7 = 0;
        let rug_fuzz_8 = 12;
        let rug_fuzz_9 = 30;
        let rug_fuzz_10 = 30;
        let rug_fuzz_11 = 0;
        let rug_fuzz_12 = 12;
        let rug_fuzz_13 = 45;
        let rug_fuzz_14 = 0;
        let rug_fuzz_15 = 0;
        let rug_fuzz_16 = 13;
        let rug_fuzz_17 = 0;
        let rug_fuzz_18 = 0;
        let rug_fuzz_19 = 0;
        let time1 = Time {
            hour: rug_fuzz_0,
            minute: rug_fuzz_1,
            second: rug_fuzz_2,
            nanosecond: rug_fuzz_3,
        };
        let time2 = Time {
            hour: rug_fuzz_4,
            minute: rug_fuzz_5,
            second: rug_fuzz_6,
            nanosecond: rug_fuzz_7,
        };
        let time3 = Time {
            hour: rug_fuzz_8,
            minute: rug_fuzz_9,
            second: rug_fuzz_10,
            nanosecond: rug_fuzz_11,
        };
        let time4 = Time {
            hour: rug_fuzz_12,
            minute: rug_fuzz_13,
            second: rug_fuzz_14,
            nanosecond: rug_fuzz_15,
        };
        let time5 = Time {
            hour: rug_fuzz_16,
            minute: rug_fuzz_17,
            second: rug_fuzz_18,
            nanosecond: rug_fuzz_19,
        };
        debug_assert_eq!(time1.cmp(& time2), Ordering::Equal);
        debug_assert_eq!(time1.cmp(& time3), Ordering::Greater);
        debug_assert_eq!(time1.cmp(& time4), Ordering::Less);
        debug_assert_eq!(time1.cmp(& time5), Ordering::Less);
        debug_assert_eq!(time3.cmp(& time4), Ordering::Less);
        debug_assert_eq!(time4.cmp(& time5), Ordering::Less);
        let _rug_ed_tests_llm_16_463_rrrruuuugggg_test_cmp = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_464 {
    use super::*;
    use crate::*;
    use std::cmp::Ordering;
    #[test]
    fn test_partial_cmp() {
        let _rug_st_tests_llm_16_464_rrrruuuugggg_test_partial_cmp = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 30;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 2;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 0;
        let rug_fuzz_7 = 0;
        let rug_fuzz_8 = 1;
        let rug_fuzz_9 = 30;
        let rug_fuzz_10 = 0;
        let rug_fuzz_11 = 0;
        let rug_fuzz_12 = 0;
        let rug_fuzz_13 = 59;
        let rug_fuzz_14 = 0;
        let rug_fuzz_15 = 0;
        let rug_fuzz_16 = 2;
        let rug_fuzz_17 = 0;
        let rug_fuzz_18 = 0;
        let rug_fuzz_19 = 0;
        let time1 = Time {
            hour: rug_fuzz_0,
            minute: rug_fuzz_1,
            second: rug_fuzz_2,
            nanosecond: rug_fuzz_3,
        };
        let time2 = Time {
            hour: rug_fuzz_4,
            minute: rug_fuzz_5,
            second: rug_fuzz_6,
            nanosecond: rug_fuzz_7,
        };
        let time3 = Time {
            hour: rug_fuzz_8,
            minute: rug_fuzz_9,
            second: rug_fuzz_10,
            nanosecond: rug_fuzz_11,
        };
        let time4 = Time {
            hour: rug_fuzz_12,
            minute: rug_fuzz_13,
            second: rug_fuzz_14,
            nanosecond: rug_fuzz_15,
        };
        let time5 = Time {
            hour: rug_fuzz_16,
            minute: rug_fuzz_17,
            second: rug_fuzz_18,
            nanosecond: rug_fuzz_19,
        };
        debug_assert_eq!(time1.partial_cmp(& time2), Some(Ordering::Less));
        debug_assert_eq!(time2.partial_cmp(& time1), Some(Ordering::Greater));
        debug_assert_eq!(time1.partial_cmp(& time3), Some(Ordering::Equal));
        debug_assert_eq!(time4.partial_cmp(& time5), Some(Ordering::Less));
        let _rug_ed_tests_llm_16_464_rrrruuuugggg_test_partial_cmp = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_988 {
    use super::*;
    use crate::*;
    use core::convert::TryFrom;
    #[test]
    fn test_format() {
        let _rug_st_tests_llm_16_988_rrrruuuugggg_test_format = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = "%T";
        let rug_fuzz_4 = 12;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 0;
        let rug_fuzz_7 = "%T";
        let rug_fuzz_8 = 23;
        let rug_fuzz_9 = 59;
        let rug_fuzz_10 = 59;
        let rug_fuzz_11 = "%T";
        debug_assert_eq!(
            Time::try_from_hms(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2).unwrap()
            .format(rug_fuzz_3), "00:00:00"
        );
        debug_assert_eq!(
            Time::try_from_hms(rug_fuzz_4, rug_fuzz_5, rug_fuzz_6).unwrap()
            .format(rug_fuzz_7), "12:00:00"
        );
        debug_assert_eq!(
            Time::try_from_hms(rug_fuzz_8, rug_fuzz_9, rug_fuzz_10).unwrap()
            .format(rug_fuzz_11), "23:59:59"
        );
        let _rug_ed_tests_llm_16_988_rrrruuuugggg_test_format = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_989 {
    use super::*;
    use crate::*;
    #[test]
    fn test_from_nanoseconds_since_midnight() {
        let _rug_st_tests_llm_16_989_rrrruuuugggg_test_from_nanoseconds_since_midnight = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 1_000_000_000;
        let rug_fuzz_6 = 0;
        let rug_fuzz_7 = 0;
        let rug_fuzz_8 = 1;
        let rug_fuzz_9 = 0;
        let rug_fuzz_10 = 3_600_000_000_000;
        let rug_fuzz_11 = 1;
        let rug_fuzz_12 = 0;
        let rug_fuzz_13 = 0;
        let rug_fuzz_14 = 0;
        let rug_fuzz_15 = 3_600_000_000_001;
        let rug_fuzz_16 = 1;
        let rug_fuzz_17 = 0;
        let rug_fuzz_18 = 0;
        let rug_fuzz_19 = 1;
        let rug_fuzz_20 = 86_400_000_000_000;
        let rug_fuzz_21 = 0;
        let rug_fuzz_22 = 0;
        let rug_fuzz_23 = 0;
        let rug_fuzz_24 = 0;
        let nanoseconds = rug_fuzz_0;
        let expected = Time {
            hour: rug_fuzz_1,
            minute: rug_fuzz_2,
            second: rug_fuzz_3,
            nanosecond: rug_fuzz_4,
        };
        let actual = Time::from_nanoseconds_since_midnight(nanoseconds);
        debug_assert_eq!(expected, actual);
        let nanoseconds = rug_fuzz_5;
        let expected = Time {
            hour: rug_fuzz_6,
            minute: rug_fuzz_7,
            second: rug_fuzz_8,
            nanosecond: rug_fuzz_9,
        };
        let actual = Time::from_nanoseconds_since_midnight(nanoseconds);
        debug_assert_eq!(expected, actual);
        let nanoseconds = rug_fuzz_10;
        let expected = Time {
            hour: rug_fuzz_11,
            minute: rug_fuzz_12,
            second: rug_fuzz_13,
            nanosecond: rug_fuzz_14,
        };
        let actual = Time::from_nanoseconds_since_midnight(nanoseconds);
        debug_assert_eq!(expected, actual);
        let nanoseconds = rug_fuzz_15;
        let expected = Time {
            hour: rug_fuzz_16,
            minute: rug_fuzz_17,
            second: rug_fuzz_18,
            nanosecond: rug_fuzz_19,
        };
        let actual = Time::from_nanoseconds_since_midnight(nanoseconds);
        debug_assert_eq!(expected, actual);
        let nanoseconds = rug_fuzz_20;
        let expected = Time {
            hour: rug_fuzz_21,
            minute: rug_fuzz_22,
            second: rug_fuzz_23,
            nanosecond: rug_fuzz_24,
        };
        let actual = Time::from_nanoseconds_since_midnight(nanoseconds);
        debug_assert_eq!(expected, actual);
        let _rug_ed_tests_llm_16_989_rrrruuuugggg_test_from_nanoseconds_since_midnight = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_996 {
    use super::*;
    use crate::*;
    #[test]
    fn test_midnight() {
        let _rug_st_tests_llm_16_996_rrrruuuugggg_test_midnight = 0;
        debug_assert_eq!(
            Time::midnight(), Time { hour : 0, minute : 0, second : 0, nanosecond : 0, }
        );
        let _rug_ed_tests_llm_16_996_rrrruuuugggg_test_midnight = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_998 {
    use super::*;
    use crate::*;
    #[test]
    fn test_millisecond() {
        let _rug_st_tests_llm_16_998_rrrruuuugggg_test_millisecond = 0;
        let rug_fuzz_0 = 23;
        let rug_fuzz_1 = 59;
        let rug_fuzz_2 = 59;
        use Time;
        debug_assert_eq!(Time::midnight().millisecond(), 0);
        debug_assert_eq!(
            Time::try_from_hms(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2).unwrap()
            .millisecond(), 999
        );
        let _rug_ed_tests_llm_16_998_rrrruuuugggg_test_millisecond = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_1000 {
    use super::*;
    use crate::*;
    use crate::time_mod::AmPm::AM;
    use core::num::NonZeroU8;
    #[test]
    fn test_minute() {
        let _rug_st_tests_llm_16_1000_rrrruuuugggg_test_minute = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 23;
        let rug_fuzz_4 = 59;
        let rug_fuzz_5 = 59;
        debug_assert_eq!(
            Time::try_from_hms(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2).unwrap().minute(), 0
        );
        debug_assert_eq!(
            Time::try_from_hms(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5).unwrap().minute(), 59
        );
        let _rug_ed_tests_llm_16_1000_rrrruuuugggg_test_minute = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_1011 {
    use crate::time_mod::Time;
    #[test]
    fn test_try_from_hms_valid() {
        let _rug_st_tests_llm_16_1011_rrrruuuugggg_test_try_from_hms_valid = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        debug_assert!(Time::try_from_hms(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2).is_ok());
        let _rug_ed_tests_llm_16_1011_rrrruuuugggg_test_try_from_hms_valid = 0;
    }
    #[test]
    fn test_try_from_hms_invalid_hour() {
        let _rug_st_tests_llm_16_1011_rrrruuuugggg_test_try_from_hms_invalid_hour = 0;
        let rug_fuzz_0 = 24;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        debug_assert!(Time::try_from_hms(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2).is_err());
        let _rug_ed_tests_llm_16_1011_rrrruuuugggg_test_try_from_hms_invalid_hour = 0;
    }
    #[test]
    fn test_try_from_hms_invalid_minute() {
        let _rug_st_tests_llm_16_1011_rrrruuuugggg_test_try_from_hms_invalid_minute = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 60;
        let rug_fuzz_2 = 0;
        debug_assert!(Time::try_from_hms(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2).is_err());
        let _rug_ed_tests_llm_16_1011_rrrruuuugggg_test_try_from_hms_invalid_minute = 0;
    }
    #[test]
    fn test_try_from_hms_invalid_second() {
        let _rug_st_tests_llm_16_1011_rrrruuuugggg_test_try_from_hms_invalid_second = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 60;
        debug_assert!(Time::try_from_hms(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2).is_err());
        let _rug_ed_tests_llm_16_1011_rrrruuuugggg_test_try_from_hms_invalid_second = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_1014 {
    use super::*;
    use crate::*;
    #[test]
    fn test_try_from_hms_milli_ok() {
        let _rug_st_tests_llm_16_1014_rrrruuuugggg_test_try_from_hms_milli_ok = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 4;
        debug_assert!(
            Time::try_from_hms_milli(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3)
            .is_ok()
        );
        let _rug_ed_tests_llm_16_1014_rrrruuuugggg_test_try_from_hms_milli_ok = 0;
    }
    #[test]
    fn test_try_from_hms_milli_err_hour() {
        let _rug_st_tests_llm_16_1014_rrrruuuugggg_test_try_from_hms_milli_err_hour = 0;
        let rug_fuzz_0 = 24;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        debug_assert!(
            Time::try_from_hms_milli(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3)
            .is_err()
        );
        let _rug_ed_tests_llm_16_1014_rrrruuuugggg_test_try_from_hms_milli_err_hour = 0;
    }
    #[test]
    fn test_try_from_hms_milli_err_minute() {
        let _rug_st_tests_llm_16_1014_rrrruuuugggg_test_try_from_hms_milli_err_minute = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 60;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        debug_assert!(
            Time::try_from_hms_milli(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3)
            .is_err()
        );
        let _rug_ed_tests_llm_16_1014_rrrruuuugggg_test_try_from_hms_milli_err_minute = 0;
    }
    #[test]
    fn test_try_from_hms_milli_err_second() {
        let _rug_st_tests_llm_16_1014_rrrruuuugggg_test_try_from_hms_milli_err_second = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 60;
        let rug_fuzz_3 = 0;
        debug_assert!(
            Time::try_from_hms_milli(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3)
            .is_err()
        );
        let _rug_ed_tests_llm_16_1014_rrrruuuugggg_test_try_from_hms_milli_err_second = 0;
    }
    #[test]
    fn test_try_from_hms_milli_err_millisecond() {
        let _rug_st_tests_llm_16_1014_rrrruuuugggg_test_try_from_hms_milli_err_millisecond = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 1_000;
        debug_assert!(
            Time::try_from_hms_milli(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3)
            .is_err()
        );
        let _rug_ed_tests_llm_16_1014_rrrruuuugggg_test_try_from_hms_milli_err_millisecond = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_1015 {
    use crate::time_mod::{Time, error};
    #[test]
    fn test_try_from_hms_nano() {
        let _rug_st_tests_llm_16_1015_rrrruuuugggg_test_try_from_hms_nano = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 4;
        let rug_fuzz_4 = 24;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 0;
        let rug_fuzz_7 = 0;
        let rug_fuzz_8 = 0;
        let rug_fuzz_9 = 60;
        let rug_fuzz_10 = 0;
        let rug_fuzz_11 = 0;
        let rug_fuzz_12 = 0;
        let rug_fuzz_13 = 0;
        let rug_fuzz_14 = 60;
        let rug_fuzz_15 = 0;
        let rug_fuzz_16 = 0;
        let rug_fuzz_17 = 0;
        let rug_fuzz_18 = 0;
        let rug_fuzz_19 = 1_000_000_000;
        debug_assert!(
            Time::try_from_hms_nano(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3)
            .is_ok()
        );
        debug_assert!(
            Time::try_from_hms_nano(rug_fuzz_4, rug_fuzz_5, rug_fuzz_6, rug_fuzz_7)
            .is_err()
        );
        debug_assert!(
            Time::try_from_hms_nano(rug_fuzz_8, rug_fuzz_9, rug_fuzz_10, rug_fuzz_11)
            .is_err()
        );
        debug_assert!(
            Time::try_from_hms_nano(rug_fuzz_12, rug_fuzz_13, rug_fuzz_14, rug_fuzz_15)
            .is_err()
        );
        debug_assert!(
            Time::try_from_hms_nano(rug_fuzz_16, rug_fuzz_17, rug_fuzz_18, rug_fuzz_19)
            .is_err()
        );
        let _rug_ed_tests_llm_16_1015_rrrruuuugggg_test_try_from_hms_nano = 0;
    }
}
#[cfg(test)]
mod tests_rug_80 {
    use super::*;
    use crate::Time;
    use crate::error::ComponentRange;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_80_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 4;
        let mut p0: u8 = rug_fuzz_0;
        let mut p1: u8 = rug_fuzz_1;
        let mut p2: u8 = rug_fuzz_2;
        let mut p3: u32 = rug_fuzz_3;
        Time::try_from_hms_micro(p0, p1, p2, p3);
        let _rug_ed_tests_rug_80_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_81 {
    use super::*;
    use crate::Time;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_81_rrrruuuugggg_test_rug = 0;
        Time::now();
        let _rug_ed_tests_rug_81_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_82 {
    use super::*;
    use ::std::convert::TryFrom;
    use crate::Time;
    #[test]
    fn test_hour() {
        let _rug_st_tests_rug_82_rrrruuuugggg_test_hour = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 23;
        let rug_fuzz_4 = 59;
        let rug_fuzz_5 = 59;
        let mut p0 = Time::try_from_hms(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2).unwrap();
        debug_assert_eq!(< Time > ::hour(p0), 0);
        let mut p1 = Time::try_from_hms(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5).unwrap();
        debug_assert_eq!(< Time > ::hour(p1), 23);
        let _rug_ed_tests_rug_82_rrrruuuugggg_test_hour = 0;
    }
}
#[cfg(test)]
mod tests_rug_83 {
    use super::*;
    use crate::Time;
    use std::convert::TryFrom;
    #[test]
    fn test_second() {
        let _rug_st_tests_rug_83_rrrruuuugggg_test_second = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 23;
        let rug_fuzz_4 = 59;
        let rug_fuzz_5 = 59;
        let mut p0 = Time::try_from_hms(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2).unwrap();
        debug_assert_eq!(< Time > ::second(p0), 0);
        let mut p1 = Time::try_from_hms(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5).unwrap();
        debug_assert_eq!(< Time > ::second(p1), 59);
        let _rug_ed_tests_rug_83_rrrruuuugggg_test_second = 0;
    }
}
#[cfg(test)]
mod tests_rug_84 {
    use super::*;
    use crate::time_mod::Time;
    use ::std::convert::TryFrom;
    #[test]
    fn test_microsecond() {
        let _rug_st_tests_rug_84_rrrruuuugggg_test_microsecond = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 23;
        let rug_fuzz_4 = 59;
        let rug_fuzz_5 = 59;
        let rug_fuzz_6 = 12;
        let rug_fuzz_7 = 34;
        let rug_fuzz_8 = 56;
        let rug_fuzz_9 = 11;
        let rug_fuzz_10 = 11;
        let rug_fuzz_11 = 11;
        let mut p0 = Time::try_from_hms(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2).unwrap();
        debug_assert_eq!(p0.microsecond(), 0);
        p0 = Time::try_from_hms(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5).unwrap();
        debug_assert_eq!(p0.microsecond(), 999_999);
        p0 = Time::try_from_hms(rug_fuzz_6, rug_fuzz_7, rug_fuzz_8).unwrap();
        debug_assert_eq!(p0.microsecond(), 559);
        p0 = Time::try_from_hms(rug_fuzz_9, rug_fuzz_10, rug_fuzz_11).unwrap();
        debug_assert_eq!(p0.microsecond(), 111);
        let _rug_ed_tests_rug_84_rrrruuuugggg_test_microsecond = 0;
    }
}
#[cfg(test)]
mod tests_rug_85 {
    use super::*;
    use crate::time_mod::Time;
    use std::convert::TryFrom;
    #[test]
    fn test_nanosecond() {
        let _rug_st_tests_rug_85_rrrruuuugggg_test_nanosecond = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 23;
        let rug_fuzz_4 = 59;
        let rug_fuzz_5 = 59;
        let rug_fuzz_6 = 23;
        let rug_fuzz_7 = 59;
        let rug_fuzz_8 = 59;
        let rug_fuzz_9 = 999_999_999;
        let p0 = Time::try_from_hms(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2).unwrap();
        debug_assert_eq!(p0.nanosecond(), 0);
        let p1 = Time::try_from_hms(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5).unwrap();
        debug_assert_eq!(p1.nanosecond(), 0);
        let p2 = Time::try_from_hms_nano(rug_fuzz_6, rug_fuzz_7, rug_fuzz_8, rug_fuzz_9)
            .unwrap();
        debug_assert_eq!(p2.nanosecond(), 999_999_999);
        let _rug_ed_tests_rug_85_rrrruuuugggg_test_nanosecond = 0;
    }
}
#[cfg(test)]
mod tests_rug_86 {
    use super::*;
    use crate::Time;
    use std::convert::TryFrom;
    #[test]
    fn test_nanoseconds_since_midnight() {
        let _rug_st_tests_rug_86_rrrruuuugggg_test_nanoseconds_since_midnight = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let p0 = Time::try_from_hms(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2).unwrap();
        let result = p0.nanoseconds_since_midnight();
        debug_assert_eq!(result, 3723000000000);
        let _rug_ed_tests_rug_86_rrrruuuugggg_test_nanoseconds_since_midnight = 0;
    }
}
#[cfg(test)]
mod tests_rug_90 {
    use super::*;
    use crate::prelude::*;
    use crate::macros::time;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_90_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 10;
        let rug_fuzz_4 = 0;
        let mut p0 = Time::try_from_hms(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2).unwrap();
        let mut p1 = Duration::new(rug_fuzz_3, rug_fuzz_4);
        p0.add(p1);
        let _rug_ed_tests_rug_90_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_91 {
    use super::*;
    use crate::time_mod::Time;
    use std::time::Duration;
    use std::convert::TryFrom;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_91_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let mut p0 = Time::try_from_hms(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2).unwrap();
        let mut p1 = Duration::new(rug_fuzz_3, rug_fuzz_4);
        <Time as std::ops::Add<Duration>>::add(p0, p1);
        let _rug_ed_tests_rug_91_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_96 {
    use super::*;
    use crate::duration::Duration;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_96_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 10;
        let rug_fuzz_4 = 0;
        let mut p0 = crate::time_mod::Time::try_from_hms(
                rug_fuzz_0,
                rug_fuzz_1,
                rug_fuzz_2,
            )
            .unwrap();
        let mut p1 = Duration::new(rug_fuzz_3, rug_fuzz_4);
        p0.sub_assign(p1);
        let _rug_ed_tests_rug_96_rrrruuuugggg_test_rug = 0;
    }
}
