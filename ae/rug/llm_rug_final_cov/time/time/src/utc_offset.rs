//! The [`UtcOffset`] struct and its associated `impl`s.
use core::fmt;
use core::ops::Neg;
#[cfg(feature = "formatting")]
use std::io;
use crate::convert::*;
use crate::error;
#[cfg(feature = "formatting")]
use crate::formatting::Formattable;
#[cfg(feature = "parsing")]
use crate::parsing::Parsable;
#[cfg(feature = "local-offset")]
use crate::sys::local_offset_at;
#[cfg(feature = "local-offset")]
use crate::OffsetDateTime;
/// An offset from UTC.
///
/// This struct can store values up to ±23:59:59. If you need support outside this range, please
/// file an issue with your use case.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UtcOffset {
    #[allow(clippy::missing_docs_in_private_items)]
    hours: i8,
    #[allow(clippy::missing_docs_in_private_items)]
    minutes: i8,
    #[allow(clippy::missing_docs_in_private_items)]
    seconds: i8,
}
impl UtcOffset {
    /// A `UtcOffset` that is UTC.
    ///
    /// ```rust
    /// # use time::UtcOffset;
    /// # use time_macros::offset;
    /// assert_eq!(UtcOffset::UTC, offset!(UTC));
    /// ```
    pub const UTC: Self = Self::__from_hms_unchecked(0, 0, 0);
    /// Create a `UtcOffset` representing an offset of the hours, minutes, and seconds provided, the
    /// validity of which must be guaranteed by the caller. All three parameters must have the same
    /// sign.
    #[doc(hidden)]
    pub const fn __from_hms_unchecked(hours: i8, minutes: i8, seconds: i8) -> Self {
        if hours < 0 {
            debug_assert!(minutes <= 0);
            debug_assert!(seconds <= 0);
        } else if hours > 0 {
            debug_assert!(minutes >= 0);
            debug_assert!(seconds >= 0);
        }
        if minutes < 0 {
            debug_assert!(seconds <= 0);
        } else if minutes > 0 {
            debug_assert!(seconds >= 0);
        }
        debug_assert!(hours.unsigned_abs() < 24);
        debug_assert!(minutes.unsigned_abs() < Minute.per(Hour));
        debug_assert!(seconds.unsigned_abs() < Second.per(Minute));
        Self { hours, minutes, seconds }
    }
    /// Create a `UtcOffset` representing an offset by the number of hours, minutes, and seconds
    /// provided.
    ///
    /// The sign of all three components should match. If they do not, all smaller components will
    /// have their signs flipped.
    ///
    /// ```rust
    /// # use time::UtcOffset;
    /// assert_eq!(UtcOffset::from_hms(1, 2, 3)?.as_hms(), (1, 2, 3));
    /// assert_eq!(UtcOffset::from_hms(1, -2, -3)?.as_hms(), (1, 2, 3));
    /// # Ok::<_, time::Error>(())
    /// ```
    pub const fn from_hms(
        hours: i8,
        mut minutes: i8,
        mut seconds: i8,
    ) -> Result<Self, error::ComponentRange> {
        ensure_value_in_range!(hours in - 23 => 23);
        ensure_value_in_range!(
            minutes in - (Minute.per(Hour) as i8 - 1) => Minute.per(Hour) as i8 - 1
        );
        ensure_value_in_range!(
            seconds in - (Second.per(Minute) as i8 - 1) => Second.per(Minute) as i8 - 1
        );
        if (hours > 0 && minutes < 0) || (hours < 0 && minutes > 0) {
            minutes *= -1;
        }
        if (hours > 0 && seconds < 0) || (hours < 0 && seconds > 0)
            || (minutes > 0 && seconds < 0) || (minutes < 0 && seconds > 0)
        {
            seconds *= -1;
        }
        Ok(Self::__from_hms_unchecked(hours, minutes, seconds))
    }
    /// Create a `UtcOffset` representing an offset by the number of seconds provided.
    ///
    /// ```rust
    /// # use time::UtcOffset;
    /// assert_eq!(UtcOffset::from_whole_seconds(3_723)?.as_hms(), (1, 2, 3));
    /// # Ok::<_, time::Error>(())
    /// ```
    pub const fn from_whole_seconds(
        seconds: i32,
    ) -> Result<Self, error::ComponentRange> {
        ensure_value_in_range!(
            seconds in - 24 * Second.per(Hour) as i32 - 1 => 24 * Second.per(Hour) as i32
            - 1
        );
        Ok(
            Self::__from_hms_unchecked(
                (seconds / Second.per(Hour) as i32) as _,
                ((seconds % Second.per(Hour) as i32) / Minute.per(Hour) as i32) as _,
                (seconds % Second.per(Minute) as i32) as _,
            ),
        )
    }
    /// Obtain the UTC offset as its hours, minutes, and seconds. The sign of all three components
    /// will always match. A positive value indicates an offset to the east; a negative to the west.
    ///
    /// ```rust
    /// # use time_macros::offset;
    /// assert_eq!(offset!(+1:02:03).as_hms(), (1, 2, 3));
    /// assert_eq!(offset!(-1:02:03).as_hms(), (-1, -2, -3));
    /// ```
    pub const fn as_hms(self) -> (i8, i8, i8) {
        (self.hours, self.minutes, self.seconds)
    }
    /// Obtain the number of whole hours the offset is from UTC. A positive value indicates an
    /// offset to the east; a negative to the west.
    ///
    /// ```rust
    /// # use time_macros::offset;
    /// assert_eq!(offset!(+1:02:03).whole_hours(), 1);
    /// assert_eq!(offset!(-1:02:03).whole_hours(), -1);
    /// ```
    pub const fn whole_hours(self) -> i8 {
        self.hours
    }
    /// Obtain the number of whole minutes the offset is from UTC. A positive value indicates an
    /// offset to the east; a negative to the west.
    ///
    /// ```rust
    /// # use time_macros::offset;
    /// assert_eq!(offset!(+1:02:03).whole_minutes(), 62);
    /// assert_eq!(offset!(-1:02:03).whole_minutes(), -62);
    /// ```
    pub const fn whole_minutes(self) -> i16 {
        self.hours as i16 * Minute.per(Hour) as i16 + self.minutes as i16
    }
    /// Obtain the number of minutes past the hour the offset is from UTC. A positive value
    /// indicates an offset to the east; a negative to the west.
    ///
    /// ```rust
    /// # use time_macros::offset;
    /// assert_eq!(offset!(+1:02:03).minutes_past_hour(), 2);
    /// assert_eq!(offset!(-1:02:03).minutes_past_hour(), -2);
    /// ```
    pub const fn minutes_past_hour(self) -> i8 {
        self.minutes
    }
    /// Obtain the number of whole seconds the offset is from UTC. A positive value indicates an
    /// offset to the east; a negative to the west.
    ///
    /// ```rust
    /// # use time_macros::offset;
    /// assert_eq!(offset!(+1:02:03).whole_seconds(), 3723);
    /// assert_eq!(offset!(-1:02:03).whole_seconds(), -3723);
    /// ```
    pub const fn whole_seconds(self) -> i32 {
        self.hours as i32 * Second.per(Hour) as i32
            + self.minutes as i32 * Second.per(Minute) as i32 + self.seconds as i32
    }
    /// Obtain the number of seconds past the minute the offset is from UTC. A positive value
    /// indicates an offset to the east; a negative to the west.
    ///
    /// ```rust
    /// # use time_macros::offset;
    /// assert_eq!(offset!(+1:02:03).seconds_past_minute(), 3);
    /// assert_eq!(offset!(-1:02:03).seconds_past_minute(), -3);
    /// ```
    pub const fn seconds_past_minute(self) -> i8 {
        self.seconds
    }
    /// Check if the offset is exactly UTC.
    ///
    ///
    /// ```rust
    /// # use time_macros::offset;
    /// assert!(!offset!(+1:02:03).is_utc());
    /// assert!(!offset!(-1:02:03).is_utc());
    /// assert!(offset!(UTC).is_utc());
    /// ```
    pub const fn is_utc(self) -> bool {
        self.hours == 0 && self.minutes == 0 && self.seconds == 0
    }
    /// Check if the offset is positive, or east of UTC.
    ///
    /// ```rust
    /// # use time_macros::offset;
    /// assert!(offset!(+1:02:03).is_positive());
    /// assert!(!offset!(-1:02:03).is_positive());
    /// assert!(!offset!(UTC).is_positive());
    /// ```
    pub const fn is_positive(self) -> bool {
        self.hours > 0 || self.minutes > 0 || self.seconds > 0
    }
    /// Check if the offset is negative, or west of UTC.
    ///
    /// ```rust
    /// # use time_macros::offset;
    /// assert!(!offset!(+1:02:03).is_negative());
    /// assert!(offset!(-1:02:03).is_negative());
    /// assert!(!offset!(UTC).is_negative());
    /// ```
    pub const fn is_negative(self) -> bool {
        self.hours < 0 || self.minutes < 0 || self.seconds < 0
    }
    /// Attempt to obtain the system's UTC offset at a known moment in time. If the offset cannot be
    /// determined, an error is returned.
    ///
    /// ```rust
    /// # use time::{UtcOffset, OffsetDateTime};
    /// let local_offset = UtcOffset::local_offset_at(OffsetDateTime::UNIX_EPOCH);
    /// # if false {
    /// assert!(local_offset.is_ok());
    /// # }
    /// ```
    #[cfg(feature = "local-offset")]
    pub fn local_offset_at(
        datetime: OffsetDateTime,
    ) -> Result<Self, error::IndeterminateOffset> {
        local_offset_at(datetime).ok_or(error::IndeterminateOffset)
    }
    /// Attempt to obtain the system's current UTC offset. If the offset cannot be determined, an
    /// error is returned.
    ///
    /// ```rust
    /// # use time::UtcOffset;
    /// let local_offset = UtcOffset::current_local_offset();
    /// # if false {
    /// assert!(local_offset.is_ok());
    /// # }
    /// ```
    #[cfg(feature = "local-offset")]
    pub fn current_local_offset() -> Result<Self, error::IndeterminateOffset> {
        let now = OffsetDateTime::now_utc();
        local_offset_at(now).ok_or(error::IndeterminateOffset)
    }
}
#[cfg(feature = "formatting")]
impl UtcOffset {
    /// Format the `UtcOffset` using the provided [format description](crate::format_description).
    pub fn format_into(
        self,
        output: &mut impl io::Write,
        format: &(impl Formattable + ?Sized),
    ) -> Result<usize, error::Format> {
        format.format_into(output, None, None, Some(self))
    }
    /// Format the `UtcOffset` using the provided [format description](crate::format_description).
    ///
    /// ```rust
    /// # use time::format_description;
    /// # use time_macros::offset;
    /// let format = format_description::parse("[offset_hour sign:mandatory]:[offset_minute]")?;
    /// assert_eq!(offset!(+1).format(&format)?, "+01:00");
    /// # Ok::<_, time::Error>(())
    /// ```
    pub fn format(
        self,
        format: &(impl Formattable + ?Sized),
    ) -> Result<String, error::Format> {
        format.format(None, None, Some(self))
    }
}
#[cfg(feature = "parsing")]
impl UtcOffset {
    /// Parse a `UtcOffset` from the input using the provided [format
    /// description](crate::format_description).
    ///
    /// ```rust
    /// # use time::UtcOffset;
    /// # use time_macros::{offset, format_description};
    /// let format = format_description!("[offset_hour]:[offset_minute]");
    /// assert_eq!(UtcOffset::parse("-03:42", &format)?, offset!(-3:42));
    /// # Ok::<_, time::Error>(())
    /// ```
    pub fn parse(
        input: &str,
        description: &(impl Parsable + ?Sized),
    ) -> Result<Self, error::Parse> {
        description.parse_offset(input.as_bytes())
    }
}
impl fmt::Display for UtcOffset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f, "{}{:02}:{:02}:{:02}", if self.is_negative() { '-' } else { '+' }, self
            .hours.abs(), self.minutes.abs(), self.seconds.abs()
        )
    }
}
impl fmt::Debug for UtcOffset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Neg for UtcOffset {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self::__from_hms_unchecked(-self.hours, -self.minutes, -self.seconds)
    }
}
#[cfg(test)]
mod tests_rug_469 {
    use super::*;
    use crate::UtcOffset;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_469_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 5;
        let rug_fuzz_2 = 30;
        let mut p0: i8 = rug_fuzz_0;
        let mut p1: i8 = -rug_fuzz_1;
        let mut p2: i8 = rug_fuzz_2;
        UtcOffset::__from_hms_unchecked(p0, p1, p2);
        let _rug_ed_tests_rug_469_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_470 {
    use super::*;
    use crate::UtcOffset;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_470_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = "Failed to create UtcOffset from hms";
        let mut p0 = rug_fuzz_0;
        let mut p1 = rug_fuzz_1;
        let mut p2 = rug_fuzz_2;
        UtcOffset::from_hms(p0, p1, p2).expect(rug_fuzz_3);
        let _rug_ed_tests_rug_470_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_471 {
    use super::*;
    use crate::{UtcOffset, error};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_471_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3_723;
        let p0: i32 = rug_fuzz_0;
        UtcOffset::from_whole_seconds(p0).unwrap();
        let _rug_ed_tests_rug_471_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_472 {
    use super::*;
    use crate::{UtcOffset, Time};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_472_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let mut p0: UtcOffset = UtcOffset::__from_hms_unchecked(
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
        );
        p0.as_hms();
        let _rug_ed_tests_rug_472_rrrruuuugggg_test_rug = 0;
    }
}
