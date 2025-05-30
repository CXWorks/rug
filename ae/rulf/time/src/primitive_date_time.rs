use crate::{
    format::parse::{parse, ParsedItems},
    internals, Date, DeferredFormat, Duration, OffsetDateTime, ParseResult, Time,
    UtcOffset, Weekday,
};
#[cfg(not(feature = "std"))]
use alloc::{borrow::ToOwned, string::{String, ToString}};
use const_fn::const_fn;
#[cfg(feature = "std")]
use core::convert::From;
use core::{
    cmp::Ordering, fmt::{self, Display},
    ops::{Add, AddAssign, Sub, SubAssign},
    time::Duration as StdDuration,
};
#[cfg(feature = "std")]
use standback::convert::TryFrom;
#[cfg(feature = "serde")]
use standback::convert::TryInto;
#[allow(unused_imports)]
use standback::prelude::*;
#[cfg(feature = "std")]
use std::time::SystemTime;
/// Combined date and time.
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(into = "crate::serde::PrimitiveDateTime"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PrimitiveDateTime {
    #[allow(clippy::missing_docs_in_private_items)]
    pub(crate) date: Date,
    #[allow(clippy::missing_docs_in_private_items)]
    pub(crate) time: Time,
}
#[cfg(feature = "serde")]
impl<'a> serde::Deserialize<'a> for PrimitiveDateTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        crate::serde::PrimitiveDateTime::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}
impl PrimitiveDateTime {
    /// Create a new `PrimitiveDateTime` from the provided `Date` and `Time`.
    ///
    /// ```rust
    /// # use time::{PrimitiveDateTime, time, date};
    /// assert_eq!(
    ///     PrimitiveDateTime::new(date!(2019-01-01), time!(0:00)),
    ///     date!(2019-01-01).midnight(),
    /// );
    /// ```
    pub const fn new(date: Date, time: Time) -> Self {
        Self { date, time }
    }
    /// Create a new `PrimitiveDateTime` with the current date and time (UTC).
    ///
    /// ```rust
    /// # #![allow(deprecated)]
    /// # use time::PrimitiveDateTime;
    /// assert!(PrimitiveDateTime::now().year() >= 2019);
    /// ```
    #[cfg(feature = "std")]
    #[cfg_attr(__time_02_docs, doc(cfg(feature = "std")))]
    #[deprecated(
        since = "0.2.7",
        note = "This method returns a value that assumes an offset of UTC."
    )]
    pub fn now() -> Self {
        SystemTime::now().into()
    }
    /// Midnight, 1 January, 1970 (UTC).
    ///
    /// ```rust
    /// # #![allow(deprecated)]
    /// # use time::{PrimitiveDateTime, date};
    /// assert_eq!(
    ///     PrimitiveDateTime::unix_epoch(),
    ///     date!(1970-01-01).midnight()
    /// );
    /// ```
    #[deprecated(since = "0.2.7", note = "This method assumes an offset of UTC.")]
    pub const fn unix_epoch() -> Self {
        Self {
            date: internals::Date::from_yo_unchecked(1970, 1),
            time: Time::midnight(),
        }
    }
    /// Create a `PrimitiveDateTime` from the provided [Unix timestamp](https://en.wikipedia.org/wiki/Unix_time).
    ///
    /// ```rust
    /// # #![allow(deprecated)]
    /// # use time::{date, PrimitiveDateTime};
    /// assert_eq!(
    ///     PrimitiveDateTime::from_unix_timestamp(0),
    ///     PrimitiveDateTime::unix_epoch()
    /// );
    /// assert_eq!(
    ///     PrimitiveDateTime::from_unix_timestamp(1_546_300_800),
    ///     date!(2019-01-01).midnight(),
    /// );
    /// ```
    #[deprecated(
        since = "0.2.7",
        note = "This method returns a value that assumes an offset of UTC."
    )]
    #[allow(deprecated)]
    pub fn from_unix_timestamp(timestamp: i64) -> Self {
        Self::unix_epoch() + Duration::seconds(timestamp)
    }
    /// Get the [Unix timestamp](https://en.wikipedia.org/wiki/Unix_time)
    /// representing the `PrimitiveDateTime`.
    ///
    /// ```rust
    /// # #![allow(deprecated)]
    /// # use time::{date, PrimitiveDateTime};
    /// assert_eq!(PrimitiveDateTime::unix_epoch().timestamp(), 0);
    /// assert_eq!(date!(2019-01-01).midnight().timestamp(), 1_546_300_800);
    /// ```
    #[allow(deprecated)]
    #[deprecated(since = "0.2.7", note = "This method assumes an offset of UTC.")]
    pub fn timestamp(self) -> i64 {
        let days = (self.date.julian_day()
            - internals::Date::from_yo_unchecked(1970, 1).julian_day()) * 86_400;
        let hours = self.hour() as i64 * 3_600;
        let minutes = self.minute() as i64 * 60;
        let seconds = self.second() as i64;
        days + hours + minutes + seconds
    }
    /// Get the `Date` component of the `PrimitiveDateTime`.
    ///
    /// ```rust
    /// # use time::date;
    /// assert_eq!(
    ///     date!(2019-01-01).midnight().date(),
    ///     date!(2019-01-01)
    /// );
    /// ```
    pub const fn date(self) -> Date {
        self.date
    }
    /// Get the `Time` component of the `PrimitiveDateTime`.
    ///
    /// ```rust
    /// # use time::{date, time};
    /// assert_eq!(date!(2019-01-01).midnight().time(), time!(0:00));
    pub const fn time(self) -> Time {
        self.time
    }
    /// Get the year of the date.
    ///
    /// ```rust
    /// # use time::date;
    /// assert_eq!(date!(2019-01-01).midnight().year(), 2019);
    /// assert_eq!(date!(2019-12-31).midnight().year(), 2019);
    /// assert_eq!(date!(2020-01-01).midnight().year(), 2020);
    /// ```
    ///
    /// This function is `const fn` when using rustc >= 1.46.
    #[const_fn("1.46")]
    pub const fn year(self) -> i32 {
        self.date().year()
    }
    /// Get the month of the date. If fetching both the month and day, it is
    /// more efficient to use [`PrimitiveDateTime::month_day`].
    ///
    /// The returned value will always be in the range `1..=12`.
    ///
    /// ```rust
    /// # use time::date;
    /// assert_eq!(date!(2019-01-01).midnight().month(), 1);
    /// assert_eq!(date!(2019-12-31).midnight().month(), 12);
    /// ```
    ///
    /// This function is `const fn` when using rustc >= 1.46.
    #[const_fn("1.46")]
    pub const fn month(self) -> u8 {
        self.date().month()
    }
    /// Get the day of the date.  If fetching both the month and day, it is
    /// more efficient to use [`PrimitiveDateTime::month_day`].
    ///
    /// The returned value will always be in the range `1..=31`.
    ///
    /// ```rust
    /// # use time::date;
    /// assert_eq!(date!(2019-1-1).midnight().day(), 1);
    /// assert_eq!(date!(2019-12-31).midnight().day(), 31);
    /// ```
    ///
    /// This function is `const fn` when using rustc >= 1.46.
    #[const_fn("1.46")]
    pub const fn day(self) -> u8 {
        self.date().day()
    }
    /// Get the month and day of the date. This is more efficient than fetching
    /// the components individually.
    ///
    /// The month component will always be in the range `1..=12`;
    /// the day component in `1..=31`.
    ///
    /// ```rust
    /// # use time::date;
    /// assert_eq!(date!(2019-01-01).midnight().month_day(), (1, 1));
    /// assert_eq!(date!(2019-12-31).midnight().month_day(), (12, 31));
    /// ```
    ///
    /// This function is `const fn` when using rustc >= 1.46.
    #[const_fn("1.46")]
    pub const fn month_day(self) -> (u8, u8) {
        self.date().month_day()
    }
    /// Get the day of the year.
    ///
    /// The returned value will always be in the range `1..=366` (`1..=365` for
    /// common years).
    ///
    /// ```rust
    /// # use time::date;
    /// assert_eq!(date!(2019-01-01).midnight().ordinal(), 1);
    /// assert_eq!(date!(2019-12-31).midnight().ordinal(), 365);
    /// ```
    ///
    /// This function is `const fn` when using rustc >= 1.46.
    #[const_fn("1.46")]
    pub const fn ordinal(self) -> u16 {
        self.date().ordinal()
    }
    /// Get the ISO 8601 year and week number.
    ///
    /// ```rust
    /// # use time::date;
    /// assert_eq!(date!(2019-01-01).midnight().iso_year_week(), (2019, 1));
    /// assert_eq!(date!(2019-10-04).midnight().iso_year_week(), (2019, 40));
    /// assert_eq!(date!(2020-01-01).midnight().iso_year_week(), (2020, 1));
    /// assert_eq!(date!(2020-12-31).midnight().iso_year_week(), (2020, 53));
    /// assert_eq!(date!(2021-01-01).midnight().iso_year_week(), (2020, 53));
    /// ```
    ///
    /// This function is `const fn` when using rustc >= 1.46.
    #[const_fn("1.46")]
    pub const fn iso_year_week(self) -> (i32, u8) {
        self.date().iso_year_week()
    }
    /// Get the ISO week number.
    ///
    /// The returned value will always be in the range `1..=53`.
    ///
    /// ```rust
    /// # use time::date;
    /// assert_eq!(date!(2019-01-01).midnight().week(), 1);
    /// assert_eq!(date!(2019-10-04).midnight().week(), 40);
    /// assert_eq!(date!(2020-01-01).midnight().week(), 1);
    /// assert_eq!(date!(2020-12-31).midnight().week(), 53);
    /// assert_eq!(date!(2021-01-01).midnight().week(), 53);
    /// ```
    ///
    /// This function is `const fn` when using rustc >= 1.46.
    #[const_fn("1.46")]
    pub const fn week(self) -> u8 {
        self.date().week()
    }
    /// Get the week number where week 1 begins on the first Sunday.
    ///
    /// The returned value will always be in the range `0..=53`.
    ///
    /// ```rust
    /// # use time::date;
    /// assert_eq!(date!(2019-01-01).midnight().sunday_based_week(), 0);
    /// assert_eq!(date!(2020-01-01).midnight().sunday_based_week(), 0);
    /// assert_eq!(date!(2020-12-31).midnight().sunday_based_week(), 52);
    /// assert_eq!(date!(2021-01-01).midnight().sunday_based_week(), 0);
    /// ```
    ///
    /// This function is `const fn` when using rustc >= 1.46.
    #[const_fn("1.46")]
    pub const fn sunday_based_week(self) -> u8 {
        self.date().sunday_based_week()
    }
    /// Get the week number where week 1 begins on the first Monday.
    ///
    /// The returned value will always be in the range `0..=53`.
    ///
    /// ```rust
    /// # use time::date;
    /// assert_eq!(date!(2019-01-01).midnight().monday_based_week(), 0);
    /// assert_eq!(date!(2020-01-01).midnight().monday_based_week(), 0);
    /// assert_eq!(date!(2020-12-31).midnight().monday_based_week(), 52);
    /// assert_eq!(date!(2021-01-01).midnight().monday_based_week(), 0);
    /// ```
    ///
    /// This function is `const fn` when using rustc >= 1.46.
    #[const_fn("1.46")]
    pub const fn monday_based_week(self) -> u8 {
        self.date().monday_based_week()
    }
    /// Get the weekday.
    ///
    /// This current uses [Zeller's congruence](https://en.wikipedia.org/wiki/Zeller%27s_congruence)
    /// internally.
    ///
    /// ```rust
    /// # use time::{date, Weekday::*};
    /// assert_eq!(date!(2019-01-01).midnight().weekday(), Tuesday);
    /// assert_eq!(date!(2019-02-01).midnight().weekday(), Friday);
    /// assert_eq!(date!(2019-03-01).midnight().weekday(), Friday);
    /// assert_eq!(date!(2019-04-01).midnight().weekday(), Monday);
    /// assert_eq!(date!(2019-05-01).midnight().weekday(), Wednesday);
    /// assert_eq!(date!(2019-06-01).midnight().weekday(), Saturday);
    /// assert_eq!(date!(2019-07-01).midnight().weekday(), Monday);
    /// assert_eq!(date!(2019-08-01).midnight().weekday(), Thursday);
    /// assert_eq!(date!(2019-09-01).midnight().weekday(), Sunday);
    /// assert_eq!(date!(2019-10-01).midnight().weekday(), Tuesday);
    /// assert_eq!(date!(2019-11-01).midnight().weekday(), Friday);
    /// assert_eq!(date!(2019-12-01).midnight().weekday(), Sunday);
    /// ```
    pub fn weekday(self) -> Weekday {
        self.date().weekday()
    }
    /// Get the clock hour.
    ///
    /// The returned value will always be in the range `0..24`.
    ///
    /// ```rust
    /// # use time::{date, time};
    /// assert_eq!(date!(2019-01-01).midnight().hour(), 0);
    /// assert_eq!(date!(2019-01-01).with_time(time!(23:59:59)).hour(), 23);
    /// ```
    pub const fn hour(self) -> u8 {
        self.time().hour()
    }
    /// Get the minute within the hour.
    ///
    /// The returned value will always be in the range `0..60`.
    ///
    /// ```rust
    /// # use time::{date, time};
    /// assert_eq!(date!(2019-01-01).midnight().minute(), 0);
    /// assert_eq!(date!(2019-01-01).with_time(time!(23:59:59)).minute(), 59);
    /// ```
    pub const fn minute(self) -> u8 {
        self.time().minute()
    }
    /// Get the second within the minute.
    ///
    /// The returned value will always be in the range `0..60`.
    ///
    /// ```rust
    /// # use time::{date, time};
    /// assert_eq!(date!(2019-01-01).midnight().second(), 0);
    /// assert_eq!(date!(2019-01-01).with_time(time!(23:59:59)).second(), 59);
    /// ```
    pub const fn second(self) -> u8 {
        self.time().second()
    }
    /// Get the milliseconds within the second.
    ///
    /// The returned value will always be in the range `0..1_000`.
    ///
    /// ```rust
    /// # use time::{date, time};
    /// assert_eq!(date!(2019-01-01).midnight().millisecond(), 0);
    /// assert_eq!(date!(2019-01-01).with_time(time!(23:59:59.999)).millisecond(), 999);
    /// ```
    pub const fn millisecond(self) -> u16 {
        self.time().millisecond()
    }
    /// Get the microseconds within the second.
    ///
    /// The returned value will always be in the range `0..1_000_000`.
    ///
    /// ```rust
    /// # use time::{date, time};
    /// assert_eq!(date!(2019-01-01).midnight().microsecond(), 0);
    /// assert_eq!(date!(2019-01-01).with_time(time!(23:59:59.999_999)).microsecond(), 999_999);
    /// ```
    pub const fn microsecond(self) -> u32 {
        self.time().microsecond()
    }
    /// Get the nanoseconds within the second.
    ///
    /// The returned value will always be in the range `0..1_000_000_000`.
    ///
    /// ```rust
    /// # use time::{date, time};
    /// assert_eq!(date!(2019-01-01).midnight().nanosecond(), 0);
    /// assert_eq!(
    ///     date!(2019-01-01).with_time(time!(23:59:59.999_999_999)).nanosecond(),
    ///     999_999_999,
    /// );
    /// ```
    pub const fn nanosecond(self) -> u32 {
        self.time().nanosecond()
    }
    /// Assuming that the existing `PrimitiveDateTime` represents a moment in
    /// the UTC, return an `OffsetDateTime` with the provided `UtcOffset`.
    ///
    /// ```rust
    /// # #![allow(deprecated)]
    /// # use time::{date, offset};
    /// assert_eq!(
    ///     date!(2019-01-01).midnight().using_offset(offset!(UTC)).unix_timestamp(),
    ///     1_546_300_800,
    /// );
    /// assert_eq!(
    ///     date!(2019-01-01).midnight().using_offset(offset!(-1)).unix_timestamp(),
    ///     1_546_300_800,
    /// );
    /// ```
    ///
    /// This function is the same as calling `.assume_utc().to_offset(offset)`.
    #[deprecated(
        since = "0.2.7",
        note = "Due to behavior not clear by its name alone, it is preferred to use \
                `.assume_utc().to_offset(offset)`. This has the same behavior and can be used in \
                `const` contexts."
    )]
    pub const fn using_offset(self, offset: UtcOffset) -> OffsetDateTime {
        self.assume_utc().to_offset(offset)
    }
    /// Assuming that the existing `PrimitiveDateTime` represents a moment in
    /// the provided `UtcOffset`, return an `OffsetDateTime`.
    ///
    /// ```rust
    /// # use time::{date, offset};
    /// assert_eq!(
    ///     date!(2019-01-01).midnight().assume_offset(offset!(UTC)).unix_timestamp(),
    ///     1_546_300_800,
    /// );
    /// assert_eq!(
    ///     date!(2019-01-01).midnight().assume_offset(offset!(-1)).unix_timestamp(),
    ///     1_546_304_400,
    /// );
    /// ```
    pub fn assume_offset(self, offset: UtcOffset) -> OffsetDateTime {
        OffsetDateTime::new_assuming_offset(self, offset)
    }
    /// Assuming that the existing `PrimitiveDateTime` represents a moment in
    /// the UTC, return an `OffsetDateTime`.
    ///
    /// ```rust
    /// # use time::date;
    /// assert_eq!(
    ///     date!(2019-01-01).midnight().assume_utc().unix_timestamp(),
    ///     1_546_300_800,
    /// );
    /// ```
    ///
    /// This function is the same as calling `.assume_offset(offset!(UTC))`,
    /// except it is usable in `const` contexts.
    pub const fn assume_utc(self) -> OffsetDateTime {
        OffsetDateTime::new_assuming_utc(self)
    }
}
/// Methods that allow formatting the `PrimitiveDateTime`.
impl PrimitiveDateTime {
    /// Format the `PrimitiveDateTime` using the provided string.
    ///
    /// ```rust
    /// # use time::date;
    /// assert_eq!(
    ///     date!(2019-01-02).midnight().format("%F %r"),
    ///     "2019-01-02 12:00:00 am"
    /// );
    /// ```
    pub fn format(self, format: impl AsRef<str>) -> String {
        self.lazy_format(format).to_string()
    }
    /// Format the `PrimitiveDateTime` using the provided string.
    ///
    /// ```rust
    /// # use time::date;
    /// assert_eq!(
    ///     date!(2019-01-02).midnight().lazy_format("%F %r").to_string(),
    ///     "2019-01-02 12:00:00 am"
    /// );
    /// ```
    pub fn lazy_format(self, format: impl AsRef<str>) -> impl Display {
        DeferredFormat::new(format.as_ref())
            .with_date(self.date())
            .with_time(self.time())
            .to_owned()
    }
    /// Attempt to parse a `PrimitiveDateTime` using the provided string.
    ///
    /// ```rust
    /// # use time::{date, PrimitiveDateTime, time};
    /// assert_eq!(
    ///     PrimitiveDateTime::parse("2019-01-02 00:00:00", "%F %T"),
    ///     Ok(date!(2019-01-02).midnight()),
    /// );
    /// assert_eq!(
    ///     PrimitiveDateTime::parse("2019-002 23:59:59", "%Y-%j %T"),
    ///     Ok(date!(2019-002).with_time(time!(23:59:59)))
    /// );
    /// assert_eq!(
    ///     PrimitiveDateTime::parse("2019-W01-3 12:00:00 pm", "%G-W%V-%u %r"),
    ///     Ok(date!(2019-W01-3).with_time(time!(12:00))),
    /// );
    /// ```
    pub fn parse(s: impl AsRef<str>, format: impl AsRef<str>) -> ParseResult<Self> {
        Self::try_from_parsed_items(parse(s.as_ref(), &format.into())?)
    }
    /// Given the items already parsed, attempt to create a `PrimitiveDateTime`.
    pub(crate) fn try_from_parsed_items(items: ParsedItems) -> ParseResult<Self> {
        Ok(Self {
            date: Date::try_from_parsed_items(items)?,
            time: Time::try_from_parsed_items(items)?,
        })
    }
}
impl Display for PrimitiveDateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.date(), self.time())
    }
}
impl Add<Duration> for PrimitiveDateTime {
    type Output = Self;
    fn add(self, duration: Duration) -> Self::Output {
        let nanos = self.time.nanoseconds_since_midnight() as i64
            + (duration.whole_nanoseconds() % 86_400_000_000_000) as i64;
        let date_modifier = if nanos < 0 {
            Duration::days(-1)
        } else if nanos >= 86_400_000_000_000 {
            Duration::day()
        } else {
            Duration::zero()
        };
        Self::new(self.date + duration + date_modifier, self.time + duration)
    }
}
impl Add<StdDuration> for PrimitiveDateTime {
    type Output = Self;
    fn add(self, duration: StdDuration) -> Self::Output {
        let nanos = self.time.nanoseconds_since_midnight()
            + (duration.as_nanos() % 86_400_000_000_000) as u64;
        let date_modifier = if nanos >= 86_400_000_000_000 {
            Duration::day()
        } else {
            Duration::zero()
        };
        Self::new(self.date + duration + date_modifier, self.time + duration)
    }
}
impl AddAssign<Duration> for PrimitiveDateTime {
    fn add_assign(&mut self, duration: Duration) {
        *self = *self + duration;
    }
}
impl AddAssign<StdDuration> for PrimitiveDateTime {
    fn add_assign(&mut self, duration: StdDuration) {
        *self = *self + duration;
    }
}
impl Sub<Duration> for PrimitiveDateTime {
    type Output = Self;
    fn sub(self, duration: Duration) -> Self::Output {
        self + -duration
    }
}
impl Sub<StdDuration> for PrimitiveDateTime {
    type Output = Self;
    fn sub(self, duration: StdDuration) -> Self::Output {
        let nanos = self.time.nanoseconds_since_midnight() as i64
            - (duration.as_nanos() % 86_400_000_000_000) as i64;
        let date_modifier = if nanos < 0 {
            Duration::days(-1)
        } else {
            Duration::zero()
        };
        Self::new(self.date - duration + date_modifier, self.time - duration)
    }
}
impl SubAssign<Duration> for PrimitiveDateTime {
    fn sub_assign(&mut self, duration: Duration) {
        *self = *self - duration;
    }
}
impl SubAssign<StdDuration> for PrimitiveDateTime {
    fn sub_assign(&mut self, duration: StdDuration) {
        *self = *self - duration;
    }
}
impl Sub<PrimitiveDateTime> for PrimitiveDateTime {
    type Output = Duration;
    fn sub(self, rhs: Self) -> Self::Output {
        (self.date - rhs.date) + (self.time - rhs.time)
    }
}
/// Deprecated since v0.2.7, as it assumes an offset of UTC.
#[cfg(feature = "std")]
impl Sub<SystemTime> for PrimitiveDateTime {
    type Output = Duration;
    fn sub(self, rhs: SystemTime) -> Self::Output {
        self - Self::from(rhs)
    }
}
/// Deprecated since v0.2.7, as it assumes an offset of UTC.
#[cfg(feature = "std")]
impl Sub<PrimitiveDateTime> for SystemTime {
    type Output = Duration;
    fn sub(self, rhs: PrimitiveDateTime) -> Self::Output {
        PrimitiveDateTime::from(self) - rhs
    }
}
impl PartialOrd for PrimitiveDateTime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
/// Deprecated since v0.2.7, as it assumes an offset of UTC.
#[cfg(feature = "std")]
impl PartialEq<SystemTime> for PrimitiveDateTime {
    fn eq(&self, rhs: &SystemTime) -> bool {
        self == &Self::from(*rhs)
    }
}
/// Deprecated since v0.2.7, as it assumes an offset of UTC.
#[cfg(feature = "std")]
impl PartialEq<PrimitiveDateTime> for SystemTime {
    fn eq(&self, rhs: &PrimitiveDateTime) -> bool {
        &PrimitiveDateTime::from(*self) == rhs
    }
}
/// Deprecated since v0.2.7, as it assumes an offset of UTC.
#[cfg(feature = "std")]
impl PartialOrd<SystemTime> for PrimitiveDateTime {
    fn partial_cmp(&self, other: &SystemTime) -> Option<Ordering> {
        self.partial_cmp(&Self::from(*other))
    }
}
/// Deprecated since v0.2.7, as it assumes an offset of UTC.
#[cfg(feature = "std")]
impl PartialOrd<PrimitiveDateTime> for SystemTime {
    fn partial_cmp(&self, other: &PrimitiveDateTime) -> Option<Ordering> {
        PrimitiveDateTime::from(*self).partial_cmp(other)
    }
}
impl Ord for PrimitiveDateTime {
    fn cmp(&self, other: &Self) -> Ordering {
        self.date.cmp(&other.date).then_with(|| self.time.cmp(&other.time))
    }
}
/// Deprecated since v0.2.7, as it returns a value that assumes an offset of UTC.
#[cfg(feature = "std")]
#[allow(deprecated)]
impl From<SystemTime> for PrimitiveDateTime {
    fn from(system_time: SystemTime) -> Self {
        let duration = match system_time.duration_since(SystemTime::UNIX_EPOCH) {
            Ok(duration) => {
                Duration::try_from(duration)
                    .expect(
                        "overflow converting `std::time::Duration` to `time::Duration`",
                    )
            }
            Err(err) => {
                -Duration::try_from(err.duration())
                    .expect(
                        "overflow converting `std::time::Duration` to `time::Duration`",
                    )
            }
        };
        Self::unix_epoch() + duration
    }
}
/// Deprecated since v0.2.7, as it assumes an offset of UTC.
#[cfg(feature = "std")]
#[allow(deprecated)]
impl From<PrimitiveDateTime> for SystemTime {
    fn from(datetime: PrimitiveDateTime) -> Self {
        let duration = datetime - PrimitiveDateTime::unix_epoch();
        if duration.is_zero() {
            Self::UNIX_EPOCH
        } else if duration.is_positive() {
            Self::UNIX_EPOCH + duration.abs_std()
        } else {
            Self::UNIX_EPOCH - duration.abs_std()
        }
    }
}
#[cfg(test)]
mod tests_rug_429 {
    use super::*;
    #[test]
    fn test_now() {
        let _rug_st_tests_rug_429_rrrruuuugggg_test_now = 0;
        let now = PrimitiveDateTime::now();
        let _rug_ed_tests_rug_429_rrrruuuugggg_test_now = 0;
    }
}
#[cfg(test)]
mod tests_rug_430 {
    use super::*;
    use crate::primitive_date_time::PrimitiveDateTime;
    use crate::{internals::Date, Time, macros::date};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_430_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1970;
        let rug_fuzz_1 = 1;
        let expected_date = Date::from_yo_unchecked(rug_fuzz_0, rug_fuzz_1);
        let expected_time = Time::midnight();
        let result = PrimitiveDateTime::unix_epoch();
        debug_assert_eq!(result.date, expected_date);
        debug_assert_eq!(result.time, expected_time);
        let _rug_ed_tests_rug_430_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_431 {
    use super::*;
    use crate::{PrimitiveDateTime, Date};
    #[test]
    fn test_primitive_date_time_from_unix_timestamp() {
        let _rug_st_tests_rug_431_rrrruuuugggg_test_primitive_date_time_from_unix_timestamp = 0;
        let rug_fuzz_0 = 0;
        let p0: i64 = rug_fuzz_0;
        PrimitiveDateTime::from_unix_timestamp(p0);
        let _rug_ed_tests_rug_431_rrrruuuugggg_test_primitive_date_time_from_unix_timestamp = 0;
    }
}
#[cfg(test)]
mod tests_rug_432 {
    use super::*;
    use crate::{PrimitiveDateTime, OffsetDateTime};
    #[test]
    fn test_timestamp() {
        let _rug_st_tests_rug_432_rrrruuuugggg_test_timestamp = 0;
        let rug_fuzz_0 = 1_546_300_800;
        let p0 = PrimitiveDateTime::unix_epoch();
        debug_assert_eq!(p0.timestamp(), 0);
        let p1 = PrimitiveDateTime::from_unix_timestamp(rug_fuzz_0);
        debug_assert_eq!(p1.timestamp(), 1_546_300_800);
        let _rug_ed_tests_rug_432_rrrruuuugggg_test_timestamp = 0;
    }
}
#[cfg(test)]
mod tests_rug_433 {
    use super::*;
    use crate::{Date, PrimitiveDateTime};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_433_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let mut p0 = PrimitiveDateTime::from_unix_timestamp(rug_fuzz_0);
        PrimitiveDateTime::date(p0);
        let _rug_ed_tests_rug_433_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_434 {
    use super::*;
    use crate::{PrimitiveDateTime, Time};
    #[test]
    fn test_time() {
        let _rug_st_tests_rug_434_rrrruuuugggg_test_time = 0;
        let rug_fuzz_0 = 0;
        let p0 = PrimitiveDateTime::from_unix_timestamp(rug_fuzz_0);
        <PrimitiveDateTime>::time(p0);
        let _rug_ed_tests_rug_434_rrrruuuugggg_test_time = 0;
    }
}
#[cfg(test)]
mod tests_rug_436 {
    use super::*;
    use crate::PrimitiveDateTime;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_436_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let mut p0: PrimitiveDateTime = PrimitiveDateTime::from_unix_timestamp(
            rug_fuzz_0,
        );
        PrimitiveDateTime::month(p0);
        let _rug_ed_tests_rug_436_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_440 {
    use super::*;
    use crate::PrimitiveDateTime;
    #[test]
    fn test_iso_year_week() {
        let _rug_st_tests_rug_440_rrrruuuugggg_test_iso_year_week = 0;
        let rug_fuzz_0 = 1546300800;
        let p0 = PrimitiveDateTime::from_unix_timestamp(rug_fuzz_0);
        debug_assert_eq!(p0.iso_year_week(), (2019, 1));
        let _rug_ed_tests_rug_440_rrrruuuugggg_test_iso_year_week = 0;
    }
}
#[cfg(test)]
mod tests_rug_441 {
    use super::*;
    use crate::{date, PrimitiveDateTime};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_441_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1609459200;
        let p0 = PrimitiveDateTime::from_unix_timestamp(rug_fuzz_0);
        p0.week();
        let _rug_ed_tests_rug_441_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_442 {
    use super::*;
    use crate::offset_date_time::OffsetDateTime;
    use crate::date::Date;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_442_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1611187200;
        let p0 = PrimitiveDateTime::from_unix_timestamp(rug_fuzz_0);
        p0.sunday_based_week();
        let _rug_ed_tests_rug_442_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_443 {
    use super::*;
    use crate::PrimitiveDateTime;
    #[test]
    fn test_monday_based_week() {
        let _rug_st_tests_rug_443_rrrruuuugggg_test_monday_based_week = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 1577836800;
        let rug_fuzz_2 = 1609372800;
        let rug_fuzz_3 = 1609459200;
        let p0 = PrimitiveDateTime::from_unix_timestamp(rug_fuzz_0);
        debug_assert_eq!(p0.monday_based_week(), 0);
        let p1 = PrimitiveDateTime::from_unix_timestamp(rug_fuzz_1);
        debug_assert_eq!(p1.monday_based_week(), 0);
        let p2 = PrimitiveDateTime::from_unix_timestamp(rug_fuzz_2);
        debug_assert_eq!(p2.monday_based_week(), 52);
        let p3 = PrimitiveDateTime::from_unix_timestamp(rug_fuzz_3);
        debug_assert_eq!(p3.monday_based_week(), 0);
        let _rug_ed_tests_rug_443_rrrruuuugggg_test_monday_based_week = 0;
    }
}
#[cfg(test)]
mod tests_rug_444 {
    use super::*;
    use crate::PrimitiveDateTime;
    use crate::Weekday;
    #[test]
    fn test_weekday() {
        let _rug_st_tests_rug_444_rrrruuuugggg_test_weekday = 0;
        let p0 = PrimitiveDateTime::unix_epoch();
        debug_assert_eq!(p0.weekday(), Weekday::Thursday);
        let _rug_ed_tests_rug_444_rrrruuuugggg_test_weekday = 0;
    }
}
#[cfg(test)]
mod tests_rug_447 {
    use super::*;
    use crate::{date, time, PrimitiveDateTime};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_447_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let mut p0 = PrimitiveDateTime::from_unix_timestamp(rug_fuzz_0);
        PrimitiveDateTime::second(p0);
        let _rug_ed_tests_rug_447_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_451 {
    use super::*;
    use crate::*;
    #[test]
    fn test_using_offset() {
        let _rug_st_tests_rug_451_rrrruuuugggg_test_using_offset = 0;
        let rug_fuzz_0 = 0;
        let p0 = PrimitiveDateTime::from_unix_timestamp(rug_fuzz_0);
        let p1 = UtcOffset::UTC;
        let result = PrimitiveDateTime::using_offset(p0, p1);
        let _rug_ed_tests_rug_451_rrrruuuugggg_test_using_offset = 0;
    }
}
#[cfg(test)]
mod tests_rug_452 {
    use super::*;
    use crate::{PrimitiveDateTime, UtcOffset};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_452_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 15;
        let mut p0 = PrimitiveDateTime::from_unix_timestamp(rug_fuzz_0);
        let mut p1 = UtcOffset::east_minutes(rug_fuzz_1);
        PrimitiveDateTime::assume_offset(p0, p1);
        let _rug_ed_tests_rug_452_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_453 {
    use super::*;
    use crate::primitive_date_time::PrimitiveDateTime;
    use crate::OffsetDateTime;
    #[test]
    fn test_assume_utc() {
        let _rug_st_tests_rug_453_rrrruuuugggg_test_assume_utc = 0;
        let rug_fuzz_0 = 1_546_300_800;
        let p0: PrimitiveDateTime = PrimitiveDateTime::from_unix_timestamp(rug_fuzz_0);
        let result: OffsetDateTime = p0.assume_utc();
        let _rug_ed_tests_rug_453_rrrruuuugggg_test_assume_utc = 0;
    }
}
#[cfg(test)]
mod tests_rug_457 {
    use super::*;
    use std::num::NonZeroU8;
    use std::num::NonZeroU16;
    use crate::{format::parse::ParsedItems, Date, Time, Weekday};
    #[test]
    fn test_try_from_parsed_items() {
        let _rug_st_tests_rug_457_rrrruuuugggg_test_try_from_parsed_items = 0;
        let rug_fuzz_0 = 2022;
        let rug_fuzz_1 = 11;
        let rug_fuzz_2 = 25;
        let mut p0 = ParsedItems::new();
        p0.week_based_year = Some(rug_fuzz_0);
        p0.month = Some(NonZeroU8::new(rug_fuzz_1).unwrap());
        p0.day = Some(NonZeroU8::new(rug_fuzz_2).unwrap());
        p0.weekday = Some(Weekday::Friday);
        let result = PrimitiveDateTime::try_from_parsed_items(p0);
        debug_assert!(result.is_ok());
        let _rug_ed_tests_rug_457_rrrruuuugggg_test_try_from_parsed_items = 0;
    }
}
#[cfg(test)]
mod tests_rug_458 {
    use super::*;
    use crate::{primitive_date_time::PrimitiveDateTime, duration::Duration};
    #[test]
    fn test_add() {
        let _rug_st_tests_rug_458_rrrruuuugggg_test_add = 0;
        let rug_fuzz_0 = 1609459200;
        let rug_fuzz_1 = 86400;
        let rug_fuzz_2 = 0;
        let p0 = PrimitiveDateTime::from_unix_timestamp(rug_fuzz_0);
        let p1 = Duration::new(rug_fuzz_1, rug_fuzz_2);
        let result = p0.add(p1);
        debug_assert_eq!(result, PrimitiveDateTime::from_unix_timestamp(1609545600));
        let _rug_ed_tests_rug_458_rrrruuuugggg_test_add = 0;
    }
}
#[cfg(test)]
mod tests_rug_461 {
    use super::*;
    use std::ops::AddAssign;
    use crate::{PrimitiveDateTime, Duration};
    #[test]
    fn test_add_assign() {
        let _rug_st_tests_rug_461_rrrruuuugggg_test_add_assign = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let mut p0 = PrimitiveDateTime::from_unix_timestamp(rug_fuzz_0);
        let mut p1 = Duration::new(rug_fuzz_1, rug_fuzz_2);
        p0.add_assign(p1);
        let _rug_ed_tests_rug_461_rrrruuuugggg_test_add_assign = 0;
    }
}
#[cfg(test)]
mod tests_rug_462 {
    use super::*;
    use crate::duration::Duration;
    #[test]
    fn test_sub() {
        let _rug_st_tests_rug_462_rrrruuuugggg_test_sub = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 10;
        let rug_fuzz_2 = 0;
        let mut p0 = PrimitiveDateTime::from_unix_timestamp(rug_fuzz_0);
        let mut p1 = Duration::new(rug_fuzz_1, rug_fuzz_2);
        p0.sub(p1);
        let _rug_ed_tests_rug_462_rrrruuuugggg_test_sub = 0;
    }
}
#[cfg(test)]
mod tests_rug_463 {
    use super::*;
    use crate::PrimitiveDateTime;
    use crate::Duration;
    use std::time::Duration as StdDuration;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_463_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let mut p0 = PrimitiveDateTime::unix_epoch();
        let mut p1 = Duration::new(rug_fuzz_0, rug_fuzz_1);
        p0.sub(p1);
        let _rug_ed_tests_rug_463_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_465 {
    use super::*;
    use crate::primitive_date_time::PrimitiveDateTime;
    use std::time::Duration;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_465_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 1;
        let mut p0 = PrimitiveDateTime::from_unix_timestamp(rug_fuzz_0);
        let mut p1 = Duration::from_secs(rug_fuzz_1);
        p0.sub_assign(p1);
        let _rug_ed_tests_rug_465_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_468 {
    use super::*;
    use crate::primitive_date_time::PrimitiveDateTime;
    use std::time::SystemTime;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_468_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1609459200;
        let mut p0: SystemTime = SystemTime::now();
        let mut p1: PrimitiveDateTime = PrimitiveDateTime::from_unix_timestamp(
            rug_fuzz_0,
        );
        <std::time::SystemTime>::sub(p0, p1);
        let _rug_ed_tests_rug_468_rrrruuuugggg_test_rug = 0;
    }
}
