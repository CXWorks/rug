//! The [`Date`] struct and its associated `impl`s.
use core::fmt;
use core::ops::{Add, Sub};
use core::time::Duration as StdDuration;
#[cfg(feature = "formatting")]
use std::io;
use crate::convert::*;
#[cfg(feature = "formatting")]
use crate::formatting::Formattable;
#[cfg(feature = "parsing")]
use crate::parsing::Parsable;
use crate::util::{days_in_year, days_in_year_month, is_leap_year, weeks_in_year};
use crate::{error, Duration, Month, PrimitiveDateTime, Time, Weekday};
/// The minimum valid year.
pub(crate) const MIN_YEAR: i32 = if cfg!(feature = "large-dates") {
    -999_999
} else {
    -9999
};
/// The maximum valid year.
pub(crate) const MAX_YEAR: i32 = if cfg!(feature = "large-dates") {
    999_999
} else {
    9999
};
/// Date in the proleptic Gregorian calendar.
///
/// By default, years between ±9999 inclusive are representable. This can be expanded to ±999,999
/// inclusive by enabling the `large-dates` crate feature. Doing so has performance implications
/// and introduces some ambiguities when parsing.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Date {
    /// Bitpacked field containing both the year and ordinal.
    value: i32,
}
impl Date {
    /// The minimum valid `Date`.
    ///
    /// The value of this may vary depending on the feature flags enabled.
    pub const MIN: Self = Self::__from_ordinal_date_unchecked(MIN_YEAR, 1);
    /// The maximum valid `Date`.
    ///
    /// The value of this may vary depending on the feature flags enabled.
    pub const MAX: Self = Self::__from_ordinal_date_unchecked(
        MAX_YEAR,
        days_in_year(MAX_YEAR),
    );
    /// Construct a `Date` from the year and ordinal values, the validity of which must be
    /// guaranteed by the caller.
    #[doc(hidden)]
    pub const fn __from_ordinal_date_unchecked(year: i32, ordinal: u16) -> Self {
        debug_assert!(year >= MIN_YEAR);
        debug_assert!(year <= MAX_YEAR);
        debug_assert!(ordinal != 0);
        debug_assert!(ordinal <= days_in_year(year));
        Self {
            value: (year << 9) | ordinal as i32,
        }
    }
    /// Attempt to create a `Date` from the year, month, and day.
    ///
    /// ```rust
    /// # use time::{Date, Month};
    /// assert!(Date::from_calendar_date(2019, Month::January, 1).is_ok());
    /// assert!(Date::from_calendar_date(2019, Month::December, 31).is_ok());
    /// ```
    ///
    /// ```rust
    /// # use time::{Date, Month};
    /// assert!(Date::from_calendar_date(2019, Month::February, 29).is_err()); // 2019 isn't a leap year.
    /// ```
    pub const fn from_calendar_date(
        year: i32,
        month: Month,
        day: u8,
    ) -> Result<Self, error::ComponentRange> {
        /// Cumulative days through the beginning of a month in both common and leap years.
        const DAYS_CUMULATIVE_COMMON_LEAP: [[u16; 12]; 2] = [
            [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334],
            [0, 31, 60, 91, 121, 152, 182, 213, 244, 274, 305, 335],
        ];
        ensure_value_in_range!(year in MIN_YEAR => MAX_YEAR);
        ensure_value_in_range!(
            day conditionally in 1 => days_in_year_month(year, month)
        );
        Ok(
            Self::__from_ordinal_date_unchecked(
                year,
                DAYS_CUMULATIVE_COMMON_LEAP[is_leap_year(year)
                    as usize][month as usize - 1] + day as u16,
            ),
        )
    }
    /// Attempt to create a `Date` from the year and ordinal day number.
    ///
    /// ```rust
    /// # use time::Date;
    /// assert!(Date::from_ordinal_date(2019, 1).is_ok());
    /// assert!(Date::from_ordinal_date(2019, 365).is_ok());
    /// ```
    ///
    /// ```rust
    /// # use time::Date;
    /// assert!(Date::from_ordinal_date(2019, 366).is_err()); // 2019 isn't a leap year.
    /// ```
    pub const fn from_ordinal_date(
        year: i32,
        ordinal: u16,
    ) -> Result<Self, error::ComponentRange> {
        ensure_value_in_range!(year in MIN_YEAR => MAX_YEAR);
        ensure_value_in_range!(ordinal conditionally in 1 => days_in_year(year));
        Ok(Self::__from_ordinal_date_unchecked(year, ordinal))
    }
    /// Attempt to create a `Date` from the ISO year, week, and weekday.
    ///
    /// ```rust
    /// # use time::{Date, Weekday::*};
    /// assert!(Date::from_iso_week_date(2019, 1, Monday).is_ok());
    /// assert!(Date::from_iso_week_date(2019, 1, Tuesday).is_ok());
    /// assert!(Date::from_iso_week_date(2020, 53, Friday).is_ok());
    /// ```
    ///
    /// ```rust
    /// # use time::{Date, Weekday::*};
    /// assert!(Date::from_iso_week_date(2019, 53, Monday).is_err()); // 2019 doesn't have 53 weeks.
    /// ```
    pub const fn from_iso_week_date(
        year: i32,
        week: u8,
        weekday: Weekday,
    ) -> Result<Self, error::ComponentRange> {
        ensure_value_in_range!(year in MIN_YEAR => MAX_YEAR);
        ensure_value_in_range!(week conditionally in 1 => weeks_in_year(year));
        let adj_year = year - 1;
        let raw = 365 * adj_year + div_floor!(adj_year, 4) - div_floor!(adj_year, 100)
            + div_floor!(adj_year, 400);
        let jan_4 = match (raw % 7) as i8 {
            -6 | 1 => 8,
            -5 | 2 => 9,
            -4 | 3 => 10,
            -3 | 4 => 4,
            -2 | 5 => 5,
            -1 | 6 => 6,
            _ => 7,
        };
        let ordinal = week as i16 * 7 + weekday.number_from_monday() as i16 - jan_4;
        Ok(
            if ordinal <= 0 {
                Self::__from_ordinal_date_unchecked(
                    year - 1,
                    (ordinal as u16).wrapping_add(days_in_year(year - 1)),
                )
            } else if ordinal > days_in_year(year) as i16 {
                Self::__from_ordinal_date_unchecked(
                    year + 1,
                    ordinal as u16 - days_in_year(year),
                )
            } else {
                Self::__from_ordinal_date_unchecked(year, ordinal as _)
            },
        )
    }
    /// Create a `Date` from the Julian day.
    ///
    /// The algorithm to perform this conversion is derived from one provided by Peter Baum; it is
    /// freely available [here](https://www.researchgate.net/publication/316558298_Date_Algorithms).
    ///
    /// ```rust
    /// # use time::Date;
    /// # use time_macros::date;
    /// assert_eq!(Date::from_julian_day(0), Ok(date!(-4713 - 11 - 24)));
    /// assert_eq!(Date::from_julian_day(2_451_545), Ok(date!(2000 - 01 - 01)));
    /// assert_eq!(Date::from_julian_day(2_458_485), Ok(date!(2019 - 01 - 01)));
    /// assert_eq!(Date::from_julian_day(2_458_849), Ok(date!(2019 - 12 - 31)));
    /// ```
    #[doc(alias = "from_julian_date")]
    pub const fn from_julian_day(
        julian_day: i32,
    ) -> Result<Self, error::ComponentRange> {
        ensure_value_in_range!(
            julian_day in Self::MIN.to_julian_day() => Self::MAX.to_julian_day()
        );
        Ok(Self::from_julian_day_unchecked(julian_day))
    }
    /// Create a `Date` from the Julian day.
    ///
    /// This does not check the validity of the provided Julian day, and as such may result in an
    /// internally invalid value.
    #[doc(alias = "from_julian_date_unchecked")]
    pub(crate) const fn from_julian_day_unchecked(julian_day: i32) -> Self {
        debug_assert!(julian_day >= Self::MIN.to_julian_day());
        debug_assert!(julian_day <= Self::MAX.to_julian_day());
        let z = julian_day - 1_721_119;
        let (mut year, mut ordinal) = if julian_day < -19_752_948
            || julian_day > 23_195_514
        {
            let g = 100 * z as i64 - 25;
            let a = (g / 3_652_425) as i32;
            let b = a - a / 4;
            let year = div_floor!(100 * b as i64 + g, 36525) as i32;
            let ordinal = (b + z - div_floor!(36525 * year as i64, 100) as i32) as _;
            (year, ordinal)
        } else {
            let g = 100 * z - 25;
            let a = g / 3_652_425;
            let b = a - a / 4;
            let year = div_floor!(100 * b + g, 36525);
            let ordinal = (b + z - div_floor!(36525 * year, 100)) as _;
            (year, ordinal)
        };
        if is_leap_year(year) {
            ordinal += 60;
            cascade!(ordinal in 1..367 => year);
        } else {
            ordinal += 59;
            cascade!(ordinal in 1..366 => year);
        }
        Self::__from_ordinal_date_unchecked(year, ordinal)
    }
    /// Get the year of the date.
    ///
    /// ```rust
    /// # use time_macros::date;
    /// assert_eq!(date!(2019 - 01 - 01).year(), 2019);
    /// assert_eq!(date!(2019 - 12 - 31).year(), 2019);
    /// assert_eq!(date!(2020 - 01 - 01).year(), 2020);
    /// ```
    pub const fn year(self) -> i32 {
        self.value >> 9
    }
    /// Get the month.
    ///
    /// ```rust
    /// # use time::Month;
    /// # use time_macros::date;
    /// assert_eq!(date!(2019 - 01 - 01).month(), Month::January);
    /// assert_eq!(date!(2019 - 12 - 31).month(), Month::December);
    /// ```
    pub const fn month(self) -> Month {
        self.month_day().0
    }
    /// Get the day of the month.
    ///
    /// The returned value will always be in the range `1..=31`.
    ///
    /// ```rust
    /// # use time_macros::date;
    /// assert_eq!(date!(2019 - 01 - 01).day(), 1);
    /// assert_eq!(date!(2019 - 12 - 31).day(), 31);
    /// ```
    pub const fn day(self) -> u8 {
        self.month_day().1
    }
    /// Get the month and day. This is more efficient than fetching the components individually.
    pub(crate) const fn month_day(self) -> (Month, u8) {
        /// The number of days up to and including the given month. Common years
        /// are first, followed by leap years.
        const CUMULATIVE_DAYS_IN_MONTH_COMMON_LEAP: [[u16; 11]; 2] = [
            [31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334],
            [31, 60, 91, 121, 152, 182, 213, 244, 274, 305, 335],
        ];
        let days = CUMULATIVE_DAYS_IN_MONTH_COMMON_LEAP[is_leap_year(self.year())
            as usize];
        let ordinal = self.ordinal();
        if ordinal > days[10] {
            (Month::December, (ordinal - days[10]) as _)
        } else if ordinal > days[9] {
            (Month::November, (ordinal - days[9]) as _)
        } else if ordinal > days[8] {
            (Month::October, (ordinal - days[8]) as _)
        } else if ordinal > days[7] {
            (Month::September, (ordinal - days[7]) as _)
        } else if ordinal > days[6] {
            (Month::August, (ordinal - days[6]) as _)
        } else if ordinal > days[5] {
            (Month::July, (ordinal - days[5]) as _)
        } else if ordinal > days[4] {
            (Month::June, (ordinal - days[4]) as _)
        } else if ordinal > days[3] {
            (Month::May, (ordinal - days[3]) as _)
        } else if ordinal > days[2] {
            (Month::April, (ordinal - days[2]) as _)
        } else if ordinal > days[1] {
            (Month::March, (ordinal - days[1]) as _)
        } else if ordinal > days[0] {
            (Month::February, (ordinal - days[0]) as _)
        } else {
            (Month::January, ordinal as _)
        }
    }
    /// Get the day of the year.
    ///
    /// The returned value will always be in the range `1..=366` (`1..=365` for common years).
    ///
    /// ```rust
    /// # use time_macros::date;
    /// assert_eq!(date!(2019 - 01 - 01).ordinal(), 1);
    /// assert_eq!(date!(2019 - 12 - 31).ordinal(), 365);
    /// ```
    pub const fn ordinal(self) -> u16 {
        (self.value & 0x1FF) as _
    }
    /// Get the ISO 8601 year and week number.
    pub(crate) const fn iso_year_week(self) -> (i32, u8) {
        let (year, ordinal) = self.to_ordinal_date();
        match ((ordinal + 10 - self.weekday().number_from_monday() as u16) / 7) as _ {
            0 => (year - 1, weeks_in_year(year - 1)),
            53 if weeks_in_year(year) == 52 => (year + 1, 1),
            week => (year, week),
        }
    }
    /// Get the ISO week number.
    ///
    /// The returned value will always be in the range `1..=53`.
    ///
    /// ```rust
    /// # use time_macros::date;
    /// assert_eq!(date!(2019 - 01 - 01).iso_week(), 1);
    /// assert_eq!(date!(2019 - 10 - 04).iso_week(), 40);
    /// assert_eq!(date!(2020 - 01 - 01).iso_week(), 1);
    /// assert_eq!(date!(2020 - 12 - 31).iso_week(), 53);
    /// assert_eq!(date!(2021 - 01 - 01).iso_week(), 53);
    /// ```
    pub const fn iso_week(self) -> u8 {
        self.iso_year_week().1
    }
    /// Get the week number where week 1 begins on the first Sunday.
    ///
    /// The returned value will always be in the range `0..=53`.
    ///
    /// ```rust
    /// # use time_macros::date;
    /// assert_eq!(date!(2019 - 01 - 01).sunday_based_week(), 0);
    /// assert_eq!(date!(2020 - 01 - 01).sunday_based_week(), 0);
    /// assert_eq!(date!(2020 - 12 - 31).sunday_based_week(), 52);
    /// assert_eq!(date!(2021 - 01 - 01).sunday_based_week(), 0);
    /// ```
    pub const fn sunday_based_week(self) -> u8 {
        ((self.ordinal() as i16 - self.weekday().number_days_from_sunday() as i16 + 6)
            / 7) as _
    }
    /// Get the week number where week 1 begins on the first Monday.
    ///
    /// The returned value will always be in the range `0..=53`.
    ///
    /// ```rust
    /// # use time_macros::date;
    /// assert_eq!(date!(2019 - 01 - 01).monday_based_week(), 0);
    /// assert_eq!(date!(2020 - 01 - 01).monday_based_week(), 0);
    /// assert_eq!(date!(2020 - 12 - 31).monday_based_week(), 52);
    /// assert_eq!(date!(2021 - 01 - 01).monday_based_week(), 0);
    /// ```
    pub const fn monday_based_week(self) -> u8 {
        ((self.ordinal() as i16 - self.weekday().number_days_from_monday() as i16 + 6)
            / 7) as _
    }
    /// Get the year, month, and day.
    ///
    /// ```rust
    /// # use time::Month;
    /// # use time_macros::date;
    /// assert_eq!(
    ///     date!(2019 - 01 - 01).to_calendar_date(),
    ///     (2019, Month::January, 1)
    /// );
    /// ```
    pub const fn to_calendar_date(self) -> (i32, Month, u8) {
        let (month, day) = self.month_day();
        (self.year(), month, day)
    }
    /// Get the year and ordinal day number.
    ///
    /// ```rust
    /// # use time_macros::date;
    /// assert_eq!(date!(2019 - 01 - 01).to_ordinal_date(), (2019, 1));
    /// ```
    pub const fn to_ordinal_date(self) -> (i32, u16) {
        (self.year(), self.ordinal())
    }
    /// Get the ISO 8601 year, week number, and weekday.
    ///
    /// ```rust
    /// # use time::Weekday::*;
    /// # use time_macros::date;
    /// assert_eq!(date!(2019 - 01 - 01).to_iso_week_date(), (2019, 1, Tuesday));
    /// assert_eq!(date!(2019 - 10 - 04).to_iso_week_date(), (2019, 40, Friday));
    /// assert_eq!(
    ///     date!(2020 - 01 - 01).to_iso_week_date(),
    ///     (2020, 1, Wednesday)
    /// );
    /// assert_eq!(
    ///     date!(2020 - 12 - 31).to_iso_week_date(),
    ///     (2020, 53, Thursday)
    /// );
    /// assert_eq!(date!(2021 - 01 - 01).to_iso_week_date(), (2020, 53, Friday));
    /// ```
    pub const fn to_iso_week_date(self) -> (i32, u8, Weekday) {
        let (year, ordinal) = self.to_ordinal_date();
        let weekday = self.weekday();
        match ((ordinal + 10 - self.weekday().number_from_monday() as u16) / 7) as _ {
            0 => (year - 1, weeks_in_year(year - 1), weekday),
            53 if weeks_in_year(year) == 52 => (year + 1, 1, weekday),
            week => (year, week, weekday),
        }
    }
    /// Get the weekday.
    ///
    /// ```rust
    /// # use time::Weekday::*;
    /// # use time_macros::date;
    /// assert_eq!(date!(2019 - 01 - 01).weekday(), Tuesday);
    /// assert_eq!(date!(2019 - 02 - 01).weekday(), Friday);
    /// assert_eq!(date!(2019 - 03 - 01).weekday(), Friday);
    /// assert_eq!(date!(2019 - 04 - 01).weekday(), Monday);
    /// assert_eq!(date!(2019 - 05 - 01).weekday(), Wednesday);
    /// assert_eq!(date!(2019 - 06 - 01).weekday(), Saturday);
    /// assert_eq!(date!(2019 - 07 - 01).weekday(), Monday);
    /// assert_eq!(date!(2019 - 08 - 01).weekday(), Thursday);
    /// assert_eq!(date!(2019 - 09 - 01).weekday(), Sunday);
    /// assert_eq!(date!(2019 - 10 - 01).weekday(), Tuesday);
    /// assert_eq!(date!(2019 - 11 - 01).weekday(), Friday);
    /// assert_eq!(date!(2019 - 12 - 01).weekday(), Sunday);
    /// ```
    pub const fn weekday(self) -> Weekday {
        match self.to_julian_day() % 7 {
            -6 | 1 => Weekday::Tuesday,
            -5 | 2 => Weekday::Wednesday,
            -4 | 3 => Weekday::Thursday,
            -3 | 4 => Weekday::Friday,
            -2 | 5 => Weekday::Saturday,
            -1 | 6 => Weekday::Sunday,
            val => {
                debug_assert!(val == 0);
                Weekday::Monday
            }
        }
    }
    /// Get the next calendar date.
    ///
    /// ```rust
    /// # use time::Date;
    /// # use time_macros::date;
    /// assert_eq!(
    ///     date!(2019 - 01 - 01).next_day(),
    ///     Some(date!(2019 - 01 - 02))
    /// );
    /// assert_eq!(
    ///     date!(2019 - 01 - 31).next_day(),
    ///     Some(date!(2019 - 02 - 01))
    /// );
    /// assert_eq!(
    ///     date!(2019 - 12 - 31).next_day(),
    ///     Some(date!(2020 - 01 - 01))
    /// );
    /// assert_eq!(Date::MAX.next_day(), None);
    /// ```
    pub const fn next_day(self) -> Option<Self> {
        if self.ordinal() == 366 || (self.ordinal() == 365 && !is_leap_year(self.year()))
        {
            if self.value == Self::MAX.value {
                None
            } else {
                Some(Self::__from_ordinal_date_unchecked(self.year() + 1, 1))
            }
        } else {
            Some(Self { value: self.value + 1 })
        }
    }
    /// Get the previous calendar date.
    ///
    /// ```rust
    /// # use time::Date;
    /// # use time_macros::date;
    /// assert_eq!(
    ///     date!(2019 - 01 - 02).previous_day(),
    ///     Some(date!(2019 - 01 - 01))
    /// );
    /// assert_eq!(
    ///     date!(2019 - 02 - 01).previous_day(),
    ///     Some(date!(2019 - 01 - 31))
    /// );
    /// assert_eq!(
    ///     date!(2020 - 01 - 01).previous_day(),
    ///     Some(date!(2019 - 12 - 31))
    /// );
    /// assert_eq!(Date::MIN.previous_day(), None);
    /// ```
    pub const fn previous_day(self) -> Option<Self> {
        if self.ordinal() != 1 {
            Some(Self { value: self.value - 1 })
        } else if self.value == Self::MIN.value {
            None
        } else {
            Some(
                Self::__from_ordinal_date_unchecked(
                    self.year() - 1,
                    days_in_year(self.year() - 1),
                ),
            )
        }
    }
    /// Get the Julian day for the date.
    ///
    /// The algorithm to perform this conversion is derived from one provided by Peter Baum; it is
    /// freely available [here](https://www.researchgate.net/publication/316558298_Date_Algorithms).
    ///
    /// ```rust
    /// # use time_macros::date;
    /// assert_eq!(date!(-4713 - 11 - 24).to_julian_day(), 0);
    /// assert_eq!(date!(2000 - 01 - 01).to_julian_day(), 2_451_545);
    /// assert_eq!(date!(2019 - 01 - 01).to_julian_day(), 2_458_485);
    /// assert_eq!(date!(2019 - 12 - 31).to_julian_day(), 2_458_849);
    /// ```
    pub const fn to_julian_day(self) -> i32 {
        let year = self.year() - 1;
        let ordinal = self.ordinal() as i32;
        ordinal + 365 * year + div_floor!(year, 4) - div_floor!(year, 100)
            + div_floor!(year, 400) + 1_721_425
    }
    /// Computes `self + duration`, returning `None` if an overflow occurred.
    ///
    /// ```rust
    /// # use time::{Date, ext::NumericalDuration};
    /// # use time_macros::date;
    /// assert_eq!(Date::MAX.checked_add(1.days()), None);
    /// assert_eq!(Date::MIN.checked_add((-2).days()), None);
    /// assert_eq!(
    ///     date!(2020 - 12 - 31).checked_add(2.days()),
    ///     Some(date!(2021 - 01 - 02))
    /// );
    /// ```
    ///
    /// # Note
    ///
    /// This function only takes whole days into account.
    ///
    /// ```rust
    /// # use time::{Date, ext::NumericalDuration};
    /// # use time_macros::date;
    /// assert_eq!(Date::MAX.checked_add(23.hours()), Some(Date::MAX));
    /// assert_eq!(Date::MIN.checked_add((-23).hours()), Some(Date::MIN));
    /// assert_eq!(
    ///     date!(2020 - 12 - 31).checked_add(23.hours()),
    ///     Some(date!(2020 - 12 - 31))
    /// );
    /// assert_eq!(
    ///     date!(2020 - 12 - 31).checked_add(47.hours()),
    ///     Some(date!(2021 - 01 - 01))
    /// );
    /// ```
    pub const fn checked_add(self, duration: Duration) -> Option<Self> {
        let whole_days = duration.whole_days();
        if whole_days < i32::MIN as i64 || whole_days > i32::MAX as i64 {
            return None;
        }
        let julian_day = const_try_opt!(
            self.to_julian_day().checked_add(whole_days as _)
        );
        if let Ok(date) = Self::from_julian_day(julian_day) { Some(date) } else { None }
    }
    /// Computes `self - duration`, returning `None` if an overflow occurred.
    ///
    /// ```
    /// # use time::{Date, ext::NumericalDuration};
    /// # use time_macros::date;
    /// assert_eq!(Date::MAX.checked_sub((-2).days()), None);
    /// assert_eq!(Date::MIN.checked_sub(1.days()), None);
    /// assert_eq!(
    ///     date!(2020 - 12 - 31).checked_sub(2.days()),
    ///     Some(date!(2020 - 12 - 29))
    /// );
    /// ```
    ///
    /// # Note
    ///
    /// This function only takes whole days into account.
    ///
    /// ```
    /// # use time::{Date, ext::NumericalDuration};
    /// # use time_macros::date;
    /// assert_eq!(Date::MAX.checked_sub((-23).hours()), Some(Date::MAX));
    /// assert_eq!(Date::MIN.checked_sub(23.hours()), Some(Date::MIN));
    /// assert_eq!(
    ///     date!(2020 - 12 - 31).checked_sub(23.hours()),
    ///     Some(date!(2020 - 12 - 31))
    /// );
    /// assert_eq!(
    ///     date!(2020 - 12 - 31).checked_sub(47.hours()),
    ///     Some(date!(2020 - 12 - 30))
    /// );
    /// ```
    pub const fn checked_sub(self, duration: Duration) -> Option<Self> {
        let whole_days = duration.whole_days();
        if whole_days < i32::MIN as i64 || whole_days > i32::MAX as i64 {
            return None;
        }
        let julian_day = const_try_opt!(
            self.to_julian_day().checked_sub(whole_days as _)
        );
        if let Ok(date) = Self::from_julian_day(julian_day) { Some(date) } else { None }
    }
    /// Computes `self + duration`, saturating value on overflow.
    ///
    /// ```rust
    /// # use time::{Date, ext::NumericalDuration};
    /// # use time_macros::date;
    /// assert_eq!(Date::MAX.saturating_add(1.days()), Date::MAX);
    /// assert_eq!(Date::MIN.saturating_add((-2).days()), Date::MIN);
    /// assert_eq!(
    ///     date!(2020 - 12 - 31).saturating_add(2.days()),
    ///     date!(2021 - 01 - 02)
    /// );
    /// ```
    ///
    /// # Note
    ///
    /// This function only takes whole days into account.
    ///
    /// ```rust
    /// # use time::ext::NumericalDuration;
    /// # use time_macros::date;
    /// assert_eq!(
    ///     date!(2020 - 12 - 31).saturating_add(23.hours()),
    ///     date!(2020 - 12 - 31)
    /// );
    /// assert_eq!(
    ///     date!(2020 - 12 - 31).saturating_add(47.hours()),
    ///     date!(2021 - 01 - 01)
    /// );
    /// ```
    pub const fn saturating_add(self, duration: Duration) -> Self {
        if let Some(datetime) = self.checked_add(duration) {
            datetime
        } else if duration.is_negative() {
            Self::MIN
        } else {
            debug_assert!(duration.is_positive());
            Self::MAX
        }
    }
    /// Computes `self - duration`, saturating value on overflow.
    ///
    /// ```
    /// # use time::{Date, ext::NumericalDuration};
    /// # use time_macros::date;
    /// assert_eq!(Date::MAX.saturating_sub((-2).days()), Date::MAX);
    /// assert_eq!(Date::MIN.saturating_sub(1.days()), Date::MIN);
    /// assert_eq!(
    ///     date!(2020 - 12 - 31).saturating_sub(2.days()),
    ///     date!(2020 - 12 - 29)
    /// );
    /// ```
    ///
    /// # Note
    ///
    /// This function only takes whole days into account.
    ///
    /// ```
    /// # use time::ext::NumericalDuration;
    /// # use time_macros::date;
    /// assert_eq!(
    ///     date!(2020 - 12 - 31).saturating_sub(23.hours()),
    ///     date!(2020 - 12 - 31)
    /// );
    /// assert_eq!(
    ///     date!(2020 - 12 - 31).saturating_sub(47.hours()),
    ///     date!(2020 - 12 - 30)
    /// );
    /// ```
    pub const fn saturating_sub(self, duration: Duration) -> Self {
        if let Some(datetime) = self.checked_sub(duration) {
            datetime
        } else if duration.is_negative() {
            Self::MAX
        } else {
            debug_assert!(duration.is_positive());
            Self::MIN
        }
    }
    /// Replace the year. The month and day will be unchanged.
    ///
    /// ```rust
    /// # use time_macros::date;
    /// assert_eq!(
    ///     date!(2022 - 02 - 18).replace_year(2019),
    ///     Ok(date!(2019 - 02 - 18))
    /// );
    /// assert!(date!(2022 - 02 - 18).replace_year(-1_000_000_000).is_err()); // -1_000_000_000 isn't a valid year
    /// assert!(date!(2022 - 02 - 18).replace_year(1_000_000_000).is_err()); // 1_000_000_000 isn't a valid year
    /// ```
    #[must_use = "This method does not mutate the original `Date`."]
    pub const fn replace_year(self, year: i32) -> Result<Self, error::ComponentRange> {
        ensure_value_in_range!(year in MIN_YEAR => MAX_YEAR);
        let ordinal = self.ordinal();
        if ordinal <= 59 {
            return Ok(Self::__from_ordinal_date_unchecked(year, ordinal));
        }
        match (is_leap_year(self.year()), is_leap_year(year)) {
            (false, false) | (true, true) => {
                Ok(Self::__from_ordinal_date_unchecked(year, ordinal))
            }
            (true, false) if ordinal == 60 => {
                Err(error::ComponentRange {
                    name: "day",
                    value: 29,
                    minimum: 1,
                    maximum: 28,
                    conditional_range: true,
                })
            }
            (false, true) => Ok(Self::__from_ordinal_date_unchecked(year, ordinal + 1)),
            (true, false) => Ok(Self::__from_ordinal_date_unchecked(year, ordinal - 1)),
        }
    }
    /// Replace the month of the year.
    ///
    /// ```rust
    /// # use time_macros::date;
    /// # use time::Month;
    /// assert_eq!(
    ///     date!(2022 - 02 - 18).replace_month(Month::January),
    ///     Ok(date!(2022 - 01 - 18))
    /// );
    /// assert!(
    ///     date!(2022 - 01 - 30)
    ///         .replace_month(Month::February)
    ///         .is_err()
    /// ); // 30 isn't a valid day in February
    /// ```
    #[must_use = "This method does not mutate the original `Date`."]
    pub const fn replace_month(
        self,
        month: Month,
    ) -> Result<Self, error::ComponentRange> {
        let (year, _, day) = self.to_calendar_date();
        Self::from_calendar_date(year, month, day)
    }
    /// Replace the day of the month.
    ///
    /// ```rust
    /// # use time_macros::date;
    /// assert_eq!(
    ///     date!(2022 - 02 - 18).replace_day(1),
    ///     Ok(date!(2022 - 02 - 01))
    /// );
    /// assert!(date!(2022 - 02 - 18).replace_day(0).is_err()); // 0 isn't a valid day
    /// assert!(date!(2022 - 02 - 18).replace_day(30).is_err()); // 30 isn't a valid day in February
    /// ```
    #[must_use = "This method does not mutate the original `Date`."]
    pub const fn replace_day(self, day: u8) -> Result<Self, error::ComponentRange> {
        if day == 0 || day >= 29 {
            ensure_value_in_range!(
                day conditionally in 1 => days_in_year_month(self.year(), self.month())
            );
        }
        Ok(
            Self::__from_ordinal_date_unchecked(
                self.year(),
                (self.ordinal() as i16 - self.day() as i16 + day as i16) as _,
            ),
        )
    }
}
/// Methods to add a [`Time`] component, resulting in a [`PrimitiveDateTime`].
impl Date {
    /// Create a [`PrimitiveDateTime`] using the existing date. The [`Time`] component will be set
    /// to midnight.
    ///
    /// ```rust
    /// # use time_macros::{date, datetime};
    /// assert_eq!(date!(1970-01-01).midnight(), datetime!(1970-01-01 0:00));
    /// ```
    pub const fn midnight(self) -> PrimitiveDateTime {
        PrimitiveDateTime::new(self, Time::MIDNIGHT)
    }
    /// Create a [`PrimitiveDateTime`] using the existing date and the provided [`Time`].
    ///
    /// ```rust
    /// # use time_macros::{date, datetime, time};
    /// assert_eq!(
    ///     date!(1970-01-01).with_time(time!(0:00)),
    ///     datetime!(1970-01-01 0:00),
    /// );
    /// ```
    pub const fn with_time(self, time: Time) -> PrimitiveDateTime {
        PrimitiveDateTime::new(self, time)
    }
    /// Attempt to create a [`PrimitiveDateTime`] using the existing date and the provided time.
    ///
    /// ```rust
    /// # use time_macros::date;
    /// assert!(date!(1970 - 01 - 01).with_hms(0, 0, 0).is_ok());
    /// assert!(date!(1970 - 01 - 01).with_hms(24, 0, 0).is_err());
    /// ```
    pub const fn with_hms(
        self,
        hour: u8,
        minute: u8,
        second: u8,
    ) -> Result<PrimitiveDateTime, error::ComponentRange> {
        Ok(
            PrimitiveDateTime::new(
                self,
                const_try!(Time::from_hms(hour, minute, second)),
            ),
        )
    }
    /// Attempt to create a [`PrimitiveDateTime`] using the existing date and the provided time.
    ///
    /// ```rust
    /// # use time_macros::date;
    /// assert!(date!(1970 - 01 - 01).with_hms_milli(0, 0, 0, 0).is_ok());
    /// assert!(date!(1970 - 01 - 01).with_hms_milli(24, 0, 0, 0).is_err());
    /// ```
    pub const fn with_hms_milli(
        self,
        hour: u8,
        minute: u8,
        second: u8,
        millisecond: u16,
    ) -> Result<PrimitiveDateTime, error::ComponentRange> {
        Ok(
            PrimitiveDateTime::new(
                self,
                const_try!(Time::from_hms_milli(hour, minute, second, millisecond)),
            ),
        )
    }
    /// Attempt to create a [`PrimitiveDateTime`] using the existing date and the provided time.
    ///
    /// ```rust
    /// # use time_macros::date;
    /// assert!(date!(1970 - 01 - 01).with_hms_micro(0, 0, 0, 0).is_ok());
    /// assert!(date!(1970 - 01 - 01).with_hms_micro(24, 0, 0, 0).is_err());
    /// ```
    pub const fn with_hms_micro(
        self,
        hour: u8,
        minute: u8,
        second: u8,
        microsecond: u32,
    ) -> Result<PrimitiveDateTime, error::ComponentRange> {
        Ok(
            PrimitiveDateTime::new(
                self,
                const_try!(Time::from_hms_micro(hour, minute, second, microsecond)),
            ),
        )
    }
    /// Attempt to create a [`PrimitiveDateTime`] using the existing date and the provided time.
    ///
    /// ```rust
    /// # use time_macros::date;
    /// assert!(date!(1970 - 01 - 01).with_hms_nano(0, 0, 0, 0).is_ok());
    /// assert!(date!(1970 - 01 - 01).with_hms_nano(24, 0, 0, 0).is_err());
    /// ```
    pub const fn with_hms_nano(
        self,
        hour: u8,
        minute: u8,
        second: u8,
        nanosecond: u32,
    ) -> Result<PrimitiveDateTime, error::ComponentRange> {
        Ok(
            PrimitiveDateTime::new(
                self,
                const_try!(Time::from_hms_nano(hour, minute, second, nanosecond)),
            ),
        )
    }
}
#[cfg(feature = "formatting")]
impl Date {
    /// Format the `Date` using the provided [format description](crate::format_description).
    pub fn format_into(
        self,
        output: &mut impl io::Write,
        format: &(impl Formattable + ?Sized),
    ) -> Result<usize, error::Format> {
        format.format_into(output, Some(self), None, None)
    }
    /// Format the `Date` using the provided [format description](crate::format_description).
    ///
    /// ```rust
    /// # use time::{format_description};
    /// # use time_macros::date;
    /// let format = format_description::parse("[year]-[month]-[day]")?;
    /// assert_eq!(date!(2020 - 01 - 02).format(&format)?, "2020-01-02");
    /// # Ok::<_, time::Error>(())
    /// ```
    pub fn format(
        self,
        format: &(impl Formattable + ?Sized),
    ) -> Result<String, error::Format> {
        format.format(Some(self), None, None)
    }
}
#[cfg(feature = "parsing")]
impl Date {
    /// Parse a `Date` from the input using the provided [format
    /// description](crate::format_description).
    ///
    /// ```rust
    /// # use time::Date;
    /// # use time_macros::{date, format_description};
    /// let format = format_description!("[year]-[month]-[day]");
    /// assert_eq!(Date::parse("2020-01-02", &format)?, date!(2020 - 01 - 02));
    /// # Ok::<_, time::Error>(())
    /// ```
    pub fn parse(
        input: &str,
        description: &(impl Parsable + ?Sized),
    ) -> Result<Self, error::Parse> {
        description.parse_date(input.as_bytes())
    }
}
impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if cfg!(feature = "large-dates") && self.year().abs() >= 10_000 {
            write!(f, "{:+}-{:02}-{:02}", self.year(), self.month() as u8, self.day())
        } else {
            write!(
                f, "{:0width$}-{:02}-{:02}", self.year(), self.month() as u8, self.day(),
                width = 4 + (self.year() < 0) as usize
            )
        }
    }
}
impl fmt::Debug for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        fmt::Display::fmt(self, f)
    }
}
impl Add<Duration> for Date {
    type Output = Self;
    fn add(self, duration: Duration) -> Self::Output {
        self.checked_add(duration).expect("overflow adding duration to date")
    }
}
impl Add<StdDuration> for Date {
    type Output = Self;
    fn add(self, duration: StdDuration) -> Self::Output {
        Self::from_julian_day(
                self.to_julian_day()
                    + (duration.as_secs() / Second.per(Day) as u64) as i32,
            )
            .expect("overflow adding duration to date")
    }
}
impl_add_assign!(Date : Duration, StdDuration);
impl Sub<Duration> for Date {
    type Output = Self;
    fn sub(self, duration: Duration) -> Self::Output {
        self.checked_sub(duration).expect("overflow subtracting duration from date")
    }
}
impl Sub<StdDuration> for Date {
    type Output = Self;
    fn sub(self, duration: StdDuration) -> Self::Output {
        Self::from_julian_day(
                self.to_julian_day()
                    - (duration.as_secs() / Second.per(Day) as u64) as i32,
            )
            .expect("overflow subtracting duration from date")
    }
}
impl_sub_assign!(Date : Duration, StdDuration);
impl Sub for Date {
    type Output = Duration;
    fn sub(self, other: Self) -> Self::Output {
        Duration::days((self.to_julian_day() - other.to_julian_day()) as _)
    }
}
#[cfg(test)]
mod tests_rug_155 {
    use super::*;
    use crate::date::Date;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_155_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 2022;
        let rug_fuzz_1 = 200;
        let mut p0: i32 = rug_fuzz_0;
        let mut p1: u16 = rug_fuzz_1;
        Date::__from_ordinal_date_unchecked(p0, p1);
        let _rug_ed_tests_rug_155_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_156 {
    use super::*;
    use crate::{Date, Month};
    #[test]
    fn test_from_calendar_date() {
        let _rug_st_tests_rug_156_rrrruuuugggg_test_from_calendar_date = 0;
        let rug_fuzz_0 = 2019;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 2019;
        let rug_fuzz_3 = 29;
        let mut p0: i32 = rug_fuzz_0;
        let mut p1: Month = Month::January;
        let mut p2: u8 = rug_fuzz_1;
        debug_assert!(Date::from_calendar_date(p0, p1, p2).is_ok());
        p1 = Month::December;
        debug_assert!(Date::from_calendar_date(p0, p1, p2).is_ok());
        p0 = rug_fuzz_2;
        p1 = Month::February;
        p2 = rug_fuzz_3;
        debug_assert!(Date::from_calendar_date(p0, p1, p2).is_err());
        let _rug_ed_tests_rug_156_rrrruuuugggg_test_from_calendar_date = 0;
    }
}
#[cfg(test)]
mod tests_rug_157 {
    use super::*;
    use crate::Date;
    #[test]
    fn test_from_ordinal_date() {
        let _rug_st_tests_rug_157_rrrruuuugggg_test_from_ordinal_date = 0;
        let rug_fuzz_0 = 2019;
        let rug_fuzz_1 = 1;
        let p0: i32 = rug_fuzz_0;
        let p1: u16 = rug_fuzz_1;
        Date::from_ordinal_date(p0, p1);
        let _rug_ed_tests_rug_157_rrrruuuugggg_test_from_ordinal_date = 0;
    }
}
#[cfg(test)]
mod tests_rug_158 {
    use super::*;
    use crate::date::{Date, Weekday};
    #[test]
    fn test_from_iso_week_date() {
        let _rug_st_tests_rug_158_rrrruuuugggg_test_from_iso_week_date = 0;
        let rug_fuzz_0 = 2019;
        let rug_fuzz_1 = 1;
        let p0: i32 = rug_fuzz_0;
        let p1: u8 = rug_fuzz_1;
        let p2: Weekday = Weekday::Monday;
        debug_assert!(Date::from_iso_week_date(p0, p1, p2).is_ok());
        let _rug_ed_tests_rug_158_rrrruuuugggg_test_from_iso_week_date = 0;
    }
}
#[cfg(test)]
mod tests_rug_159 {
    use super::*;
    use crate::{Date, date};
    #[test]
    fn test_from_julian_day() {
        let _rug_st_tests_rug_159_rrrruuuugggg_test_from_julian_day = 0;
        let rug_fuzz_0 = 0;
        let p0: i32 = rug_fuzz_0;
        Date::from_julian_day(p0);
        let _rug_ed_tests_rug_159_rrrruuuugggg_test_from_julian_day = 0;
    }
}
#[cfg(test)]
mod tests_rug_160 {
    use super::*;
    use crate::date::Date;
    #[test]
    fn test_from_julian_day_unchecked() {
        let _rug_st_tests_rug_160_rrrruuuugggg_test_from_julian_day_unchecked = 0;
        let rug_fuzz_0 = 2459432;
        let p0: i32 = rug_fuzz_0;
        Date::from_julian_day_unchecked(p0);
        let _rug_ed_tests_rug_160_rrrruuuugggg_test_from_julian_day_unchecked = 0;
    }
}
#[cfg(test)]
mod tests_rug_161 {
    use super::*;
    use crate::date::Date;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_161_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 2019;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 2019;
        let rug_fuzz_3 = 365;
        let rug_fuzz_4 = 2020;
        let rug_fuzz_5 = 1;
        let p0 = Date::__from_ordinal_date_unchecked(rug_fuzz_0, rug_fuzz_1);
        debug_assert_eq!(p0.year(), 2019);
        let p0 = Date::__from_ordinal_date_unchecked(rug_fuzz_2, rug_fuzz_3);
        debug_assert_eq!(p0.year(), 2019);
        let p0 = Date::__from_ordinal_date_unchecked(rug_fuzz_4, rug_fuzz_5);
        debug_assert_eq!(p0.year(), 2020);
        let _rug_ed_tests_rug_161_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_162 {
    use super::*;
    use crate::{Month, date};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_162_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 2019;
        let rug_fuzz_1 = 354;
        let mut p0 = date::Date::__from_ordinal_date_unchecked(rug_fuzz_0, rug_fuzz_1);
        crate::date::Date::month(p0);
        let _rug_ed_tests_rug_162_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_164 {
    use super::*;
    use crate::{Date, Month};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_164_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 2022;
        let rug_fuzz_1 = 365;
        let p0 = Date::__from_ordinal_date_unchecked(rug_fuzz_0, rug_fuzz_1);
        debug_assert_eq!(p0.month_day(), (Month::December, 31));
        let _rug_ed_tests_rug_164_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_166 {
    use super::*;
    use crate::Date;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_166_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 2022;
        let rug_fuzz_1 = 365;
        let mut p0 = Date::__from_ordinal_date_unchecked(rug_fuzz_0, rug_fuzz_1);
        Date::iso_year_week(p0);
        let _rug_ed_tests_rug_166_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_167 {
    use super::*;
    use crate::date::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_167_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 2019;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 2019;
        let rug_fuzz_3 = 40;
        let rug_fuzz_4 = 2020;
        let rug_fuzz_5 = 1;
        let rug_fuzz_6 = 2020;
        let rug_fuzz_7 = 365;
        let rug_fuzz_8 = 2021;
        let rug_fuzz_9 = 1;
        let p0 = Date::__from_ordinal_date_unchecked(rug_fuzz_0, rug_fuzz_1);
        debug_assert_eq!(p0.iso_week(), 1);
        let p0 = Date::__from_ordinal_date_unchecked(rug_fuzz_2, rug_fuzz_3);
        debug_assert_eq!(p0.iso_week(), 40);
        let p0 = Date::__from_ordinal_date_unchecked(rug_fuzz_4, rug_fuzz_5);
        debug_assert_eq!(p0.iso_week(), 1);
        let p0 = Date::__from_ordinal_date_unchecked(rug_fuzz_6, rug_fuzz_7);
        debug_assert_eq!(p0.iso_week(), 53);
        let p0 = Date::__from_ordinal_date_unchecked(rug_fuzz_8, rug_fuzz_9);
        debug_assert_eq!(p0.iso_week(), 53);
        let _rug_ed_tests_rug_167_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_170 {
    use super::*;
    use crate::date::{Date, Month};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_170_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 2019;
        let rug_fuzz_1 = 1;
        let mut p0 = Date::__from_ordinal_date_unchecked(rug_fuzz_0, rug_fuzz_1);
        debug_assert_eq!(p0.to_calendar_date(), (2019, Month::January, 1));
        let _rug_ed_tests_rug_170_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_175 {
    use super::*;
    use crate::date::Date;
    #[test]
    fn test_previous_day() {
        let _rug_st_tests_rug_175_rrrruuuugggg_test_previous_day = 0;
        let rug_fuzz_0 = 2019;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 2019;
        let rug_fuzz_3 = 33;
        let rug_fuzz_4 = 2020;
        let rug_fuzz_5 = 1;
        let p0 = Date::__from_ordinal_date_unchecked(rug_fuzz_0, rug_fuzz_1);
        debug_assert_eq!(
            p0.previous_day(), Some(Date::__from_ordinal_date_unchecked(2019, 1))
        );
        let p1 = Date::__from_ordinal_date_unchecked(rug_fuzz_2, rug_fuzz_3);
        debug_assert_eq!(
            p1.previous_day(), Some(Date::__from_ordinal_date_unchecked(2019, 32))
        );
        let p2 = Date::__from_ordinal_date_unchecked(rug_fuzz_4, rug_fuzz_5);
        debug_assert_eq!(
            p2.previous_day(), Some(Date::__from_ordinal_date_unchecked(2019, 365))
        );
        debug_assert_eq!(Date::MIN.previous_day(), None);
        let _rug_ed_tests_rug_175_rrrruuuugggg_test_previous_day = 0;
    }
}
#[cfg(test)]
mod tests_rug_178 {
    use super::*;
    use crate::ext::NumericalDuration;
    use time_macros::date;
    #[test]
    fn test_checked_sub() {
        let _rug_st_tests_rug_178_rrrruuuugggg_test_checked_sub = 0;
        let rug_fuzz_0 = 2020;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 2;
        let p0 = Date::__from_ordinal_date_unchecked(rug_fuzz_0, rug_fuzz_1);
        let p1 = (-rug_fuzz_2).days();
        debug_assert_eq!(p0.checked_sub(p1), None);
        let _rug_ed_tests_rug_178_rrrruuuugggg_test_checked_sub = 0;
    }
}
#[cfg(test)]
mod tests_rug_180 {
    use super::*;
    use crate::{Date, ext::NumericalDuration};
    use time_macros::date;
    #[test]
    fn test_saturating_sub() {
        let _rug_st_tests_rug_180_rrrruuuugggg_test_saturating_sub = 0;
        let rug_fuzz_0 = 2021;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 1;
        let mut p0 = Date::__from_ordinal_date_unchecked(rug_fuzz_0, rug_fuzz_1);
        let mut p1 = rug_fuzz_2.days();
        <Date>::saturating_sub(p0, p1);
        let _rug_ed_tests_rug_180_rrrruuuugggg_test_saturating_sub = 0;
    }
}
#[cfg(test)]
mod tests_rug_181 {
    use super::*;
    use crate::date::*;
    #[test]
    fn test_replace_year() {
        let _rug_st_tests_rug_181_rrrruuuugggg_test_replace_year = 0;
        let rug_fuzz_0 = 2022;
        let rug_fuzz_1 = 49;
        let rug_fuzz_2 = 2019;
        let rug_fuzz_3 = 1_000_000_000;
        let rug_fuzz_4 = 1_000_000_000;
        let mut p0 = Date::__from_ordinal_date_unchecked(rug_fuzz_0, rug_fuzz_1);
        let mut p1 = rug_fuzz_2;
        debug_assert_eq!(
            p0.replace_year(p1), Ok(Date::__from_ordinal_date_unchecked(2019, 49))
        );
        p1 = -rug_fuzz_3;
        debug_assert!(p0.replace_year(p1).is_err());
        p1 = rug_fuzz_4;
        debug_assert!(p0.replace_year(p1).is_err());
        let _rug_ed_tests_rug_181_rrrruuuugggg_test_replace_year = 0;
    }
}
#[cfg(test)]
mod tests_rug_184 {
    use super::*;
    use crate::date::Date;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_184_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1970;
        let rug_fuzz_1 = 1;
        let p0 = Date::__from_ordinal_date_unchecked(rug_fuzz_0, rug_fuzz_1);
        Date::midnight(p0);
        let _rug_ed_tests_rug_184_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_187 {
    use super::*;
    use crate::date::Date;
    use crate::error::ComponentRange;
    #[test]
    fn test_with_hms_milli() {
        let _rug_st_tests_rug_187_rrrruuuugggg_test_with_hms_milli = 0;
        let rug_fuzz_0 = 1970;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let p0 = Date::__from_ordinal_date_unchecked(rug_fuzz_0, rug_fuzz_1);
        let p1: u8 = rug_fuzz_2;
        let p2: u8 = rug_fuzz_3;
        let p3: u8 = rug_fuzz_4;
        let p4: u16 = rug_fuzz_5;
        debug_assert_eq!(
            p0.with_hms_milli(p1, p2, p3, p4), Ok(PrimitiveDateTime::new(p0,
            Time::from_hms_milli(p1, p2, p3, p4).unwrap()))
        );
        let _rug_ed_tests_rug_187_rrrruuuugggg_test_with_hms_milli = 0;
    }
}
#[cfg(test)]
mod tests_rug_189 {
    use super::*;
    use crate::date::*;
    use crate::error::ComponentRange;
    use crate::PrimitiveDateTime;
    use crate::Time;
    #[test]
    fn test_with_hms_nano() {
        let _rug_st_tests_rug_189_rrrruuuugggg_test_with_hms_nano = 0;
        let rug_fuzz_0 = 1970;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let mut p0 = Date::__from_ordinal_date_unchecked(rug_fuzz_0, rug_fuzz_1);
        let mut p1: u8 = rug_fuzz_2;
        let mut p2: u8 = rug_fuzz_3;
        let mut p3: u8 = rug_fuzz_4;
        let mut p4: u32 = rug_fuzz_5;
        debug_assert_eq!(
            p0.with_hms_nano(p1, p2, p3, p4), Ok(PrimitiveDateTime::new(p0,
            Time::from_hms_nano(p1, p2, p3, p4).unwrap()))
        );
        let _rug_ed_tests_rug_189_rrrruuuugggg_test_with_hms_nano = 0;
    }
}
