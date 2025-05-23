//! ISO 8601 calendar date with time zone.
#![allow(deprecated)]
#[cfg(any(feature = "alloc", feature = "std", test))]
use core::borrow::Borrow;
use core::cmp::Ordering;
use core::ops::{Add, AddAssign, Sub, SubAssign};
use core::{fmt, hash};
#[cfg(feature = "rkyv")]
use rkyv::{Archive, Deserialize, Serialize};
#[cfg(feature = "unstable-locales")]
use crate::format::Locale;
#[cfg(any(feature = "alloc", feature = "std", test))]
use crate::format::{DelayedFormat, Item, StrftimeItems};
use crate::naive::{IsoWeek, NaiveDate, NaiveTime};
use crate::offset::{TimeZone, Utc};
use crate::time_delta::TimeDelta;
use crate::DateTime;
use crate::{Datelike, Weekday};
/// ISO 8601 calendar date with time zone.
///
/// You almost certainly want to be using a [`NaiveDate`] instead of this type.
///
/// This type primarily exists to aid in the construction of DateTimes that
/// have a timezone by way of the [`TimeZone`] datelike constructors (e.g.
/// [`TimeZone::ymd`]).
///
/// This type should be considered ambiguous at best, due to the inherent lack
/// of precision required for the time zone resolution.
///
/// There are some guarantees on the usage of `Date<Tz>`:
///
/// - If properly constructed via [`TimeZone::ymd`] and others without an error,
///   the corresponding local date should exist for at least a moment.
///   (It may still have a gap from the offset changes.)
///
/// - The `TimeZone` is free to assign *any* [`Offset`](crate::offset::Offset) to the
///   local date, as long as that offset did occur in given day.
///
///   For example, if `2015-03-08T01:59-08:00` is followed by `2015-03-08T03:00-07:00`,
///   it may produce either `2015-03-08-08:00` or `2015-03-08-07:00`
///   but *not* `2015-03-08+00:00` and others.
///
/// - Once constructed as a full `DateTime`, [`DateTime::date`] and other associated
///   methods should return those for the original `Date`. For example, if `dt =
///   tz.ymd_opt(y,m,d).unwrap().hms(h,n,s)` were valid, `dt.date() == tz.ymd_opt(y,m,d).unwrap()`.
///
/// - The date is timezone-agnostic up to one day (i.e. practically always),
///   so the local date and UTC date should be equal for most cases
///   even though the raw calculation between `NaiveDate` and `Duration` may not.
#[deprecated(since = "0.4.23", note = "Use `NaiveDate` or `DateTime<Tz>` instead")]
#[derive(Clone)]
#[cfg_attr(feature = "rkyv", derive(Archive, Deserialize, Serialize))]
pub struct Date<Tz: TimeZone> {
    date: NaiveDate,
    offset: Tz::Offset,
}
/// The minimum possible `Date`.
#[allow(deprecated)]
#[deprecated(since = "0.4.20", note = "Use Date::MIN_UTC instead")]
pub const MIN_DATE: Date<Utc> = Date::<Utc>::MIN_UTC;
/// The maximum possible `Date`.
#[allow(deprecated)]
#[deprecated(since = "0.4.20", note = "Use Date::MAX_UTC instead")]
pub const MAX_DATE: Date<Utc> = Date::<Utc>::MAX_UTC;
impl<Tz: TimeZone> Date<Tz> {
    /// Makes a new `Date` with given *UTC* date and offset.
    /// The local date should be constructed via the `TimeZone` trait.
    #[inline]
    #[must_use]
    pub fn from_utc(date: NaiveDate, offset: Tz::Offset) -> Date<Tz> {
        Date { date, offset }
    }
    /// Makes a new `DateTime` from the current date and given `NaiveTime`.
    /// The offset in the current date is preserved.
    ///
    /// Panics on invalid datetime.
    #[inline]
    #[must_use]
    pub fn and_time(&self, time: NaiveTime) -> Option<DateTime<Tz>> {
        let localdt = self.naive_local().and_time(time);
        self.timezone().from_local_datetime(&localdt).single()
    }
    /// Makes a new `DateTime` from the current date, hour, minute and second.
    /// The offset in the current date is preserved.
    ///
    /// Panics on invalid hour, minute and/or second.
    #[deprecated(since = "0.4.23", note = "Use and_hms_opt() instead")]
    #[inline]
    #[must_use]
    pub fn and_hms(&self, hour: u32, min: u32, sec: u32) -> DateTime<Tz> {
        self.and_hms_opt(hour, min, sec).expect("invalid time")
    }
    /// Makes a new `DateTime` from the current date, hour, minute and second.
    /// The offset in the current date is preserved.
    ///
    /// Returns `None` on invalid hour, minute and/or second.
    #[inline]
    #[must_use]
    pub fn and_hms_opt(&self, hour: u32, min: u32, sec: u32) -> Option<DateTime<Tz>> {
        NaiveTime::from_hms_opt(hour, min, sec).and_then(|time| self.and_time(time))
    }
    /// Makes a new `DateTime` from the current date, hour, minute, second and millisecond.
    /// The millisecond part can exceed 1,000 in order to represent the leap second.
    /// The offset in the current date is preserved.
    ///
    /// Panics on invalid hour, minute, second and/or millisecond.
    #[deprecated(since = "0.4.23", note = "Use and_hms_milli_opt() instead")]
    #[inline]
    #[must_use]
    pub fn and_hms_milli(
        &self,
        hour: u32,
        min: u32,
        sec: u32,
        milli: u32,
    ) -> DateTime<Tz> {
        self.and_hms_milli_opt(hour, min, sec, milli).expect("invalid time")
    }
    /// Makes a new `DateTime` from the current date, hour, minute, second and millisecond.
    /// The millisecond part can exceed 1,000 in order to represent the leap second.
    /// The offset in the current date is preserved.
    ///
    /// Returns `None` on invalid hour, minute, second and/or millisecond.
    #[inline]
    #[must_use]
    pub fn and_hms_milli_opt(
        &self,
        hour: u32,
        min: u32,
        sec: u32,
        milli: u32,
    ) -> Option<DateTime<Tz>> {
        NaiveTime::from_hms_milli_opt(hour, min, sec, milli)
            .and_then(|time| self.and_time(time))
    }
    /// Makes a new `DateTime` from the current date, hour, minute, second and microsecond.
    /// The microsecond part can exceed 1,000,000 in order to represent the leap second.
    /// The offset in the current date is preserved.
    ///
    /// Panics on invalid hour, minute, second and/or microsecond.
    #[deprecated(since = "0.4.23", note = "Use and_hms_micro_opt() instead")]
    #[inline]
    #[must_use]
    pub fn and_hms_micro(
        &self,
        hour: u32,
        min: u32,
        sec: u32,
        micro: u32,
    ) -> DateTime<Tz> {
        self.and_hms_micro_opt(hour, min, sec, micro).expect("invalid time")
    }
    /// Makes a new `DateTime` from the current date, hour, minute, second and microsecond.
    /// The microsecond part can exceed 1,000,000 in order to represent the leap second.
    /// The offset in the current date is preserved.
    ///
    /// Returns `None` on invalid hour, minute, second and/or microsecond.
    #[inline]
    #[must_use]
    pub fn and_hms_micro_opt(
        &self,
        hour: u32,
        min: u32,
        sec: u32,
        micro: u32,
    ) -> Option<DateTime<Tz>> {
        NaiveTime::from_hms_micro_opt(hour, min, sec, micro)
            .and_then(|time| self.and_time(time))
    }
    /// Makes a new `DateTime` from the current date, hour, minute, second and nanosecond.
    /// The nanosecond part can exceed 1,000,000,000 in order to represent the leap second.
    /// The offset in the current date is preserved.
    ///
    /// Panics on invalid hour, minute, second and/or nanosecond.
    #[deprecated(since = "0.4.23", note = "Use and_hms_nano_opt() instead")]
    #[inline]
    #[must_use]
    pub fn and_hms_nano(
        &self,
        hour: u32,
        min: u32,
        sec: u32,
        nano: u32,
    ) -> DateTime<Tz> {
        self.and_hms_nano_opt(hour, min, sec, nano).expect("invalid time")
    }
    /// Makes a new `DateTime` from the current date, hour, minute, second and nanosecond.
    /// The nanosecond part can exceed 1,000,000,000 in order to represent the leap second.
    /// The offset in the current date is preserved.
    ///
    /// Returns `None` on invalid hour, minute, second and/or nanosecond.
    #[inline]
    #[must_use]
    pub fn and_hms_nano_opt(
        &self,
        hour: u32,
        min: u32,
        sec: u32,
        nano: u32,
    ) -> Option<DateTime<Tz>> {
        NaiveTime::from_hms_nano_opt(hour, min, sec, nano)
            .and_then(|time| self.and_time(time))
    }
    /// Makes a new `Date` for the next date.
    ///
    /// Panics when `self` is the last representable date.
    #[deprecated(since = "0.4.23", note = "Use succ_opt() instead")]
    #[inline]
    #[must_use]
    pub fn succ(&self) -> Date<Tz> {
        self.succ_opt().expect("out of bound")
    }
    /// Makes a new `Date` for the next date.
    ///
    /// Returns `None` when `self` is the last representable date.
    #[inline]
    #[must_use]
    pub fn succ_opt(&self) -> Option<Date<Tz>> {
        self.date.succ_opt().map(|date| Date::from_utc(date, self.offset.clone()))
    }
    /// Makes a new `Date` for the prior date.
    ///
    /// Panics when `self` is the first representable date.
    #[deprecated(since = "0.4.23", note = "Use pred_opt() instead")]
    #[inline]
    #[must_use]
    pub fn pred(&self) -> Date<Tz> {
        self.pred_opt().expect("out of bound")
    }
    /// Makes a new `Date` for the prior date.
    ///
    /// Returns `None` when `self` is the first representable date.
    #[inline]
    #[must_use]
    pub fn pred_opt(&self) -> Option<Date<Tz>> {
        self.date.pred_opt().map(|date| Date::from_utc(date, self.offset.clone()))
    }
    /// Retrieves an associated offset from UTC.
    #[inline]
    #[must_use]
    pub fn offset(&self) -> &Tz::Offset {
        &self.offset
    }
    /// Retrieves an associated time zone.
    #[inline]
    #[must_use]
    pub fn timezone(&self) -> Tz {
        TimeZone::from_offset(&self.offset)
    }
    /// Changes the associated time zone.
    /// This does not change the actual `Date` (but will change the string representation).
    #[inline]
    #[must_use]
    pub fn with_timezone<Tz2: TimeZone>(&self, tz: &Tz2) -> Date<Tz2> {
        tz.from_utc_date(&self.date)
    }
    /// Adds given `Duration` to the current date.
    ///
    /// Returns `None` when it will result in overflow.
    #[inline]
    #[must_use]
    pub fn checked_add_signed(self, rhs: TimeDelta) -> Option<Date<Tz>> {
        let date = self.date.checked_add_signed(rhs)?;
        Some(Date { date, offset: self.offset })
    }
    /// Subtracts given `Duration` from the current date.
    ///
    /// Returns `None` when it will result in overflow.
    #[inline]
    #[must_use]
    pub fn checked_sub_signed(self, rhs: TimeDelta) -> Option<Date<Tz>> {
        let date = self.date.checked_sub_signed(rhs)?;
        Some(Date { date, offset: self.offset })
    }
    /// Subtracts another `Date` from the current date.
    /// Returns a `Duration` of integral numbers.
    ///
    /// This does not overflow or underflow at all,
    /// as all possible output fits in the range of `Duration`.
    #[inline]
    #[must_use]
    pub fn signed_duration_since<Tz2: TimeZone>(self, rhs: Date<Tz2>) -> TimeDelta {
        self.date.signed_duration_since(rhs.date)
    }
    /// Returns a view to the naive UTC date.
    #[inline]
    #[must_use]
    pub fn naive_utc(&self) -> NaiveDate {
        self.date
    }
    /// Returns a view to the naive local date.
    ///
    /// This is technically the same as [`naive_utc`](#method.naive_utc)
    /// because the offset is restricted to never exceed one day,
    /// but provided for the consistency.
    #[inline]
    #[must_use]
    pub fn naive_local(&self) -> NaiveDate {
        self.date
    }
    /// Returns the number of whole years from the given `base` until `self`.
    #[must_use]
    pub fn years_since(&self, base: Self) -> Option<u32> {
        self.date.years_since(base.date)
    }
    /// The minimum possible `Date`.
    pub const MIN_UTC: Date<Utc> = Date {
        date: NaiveDate::MIN,
        offset: Utc,
    };
    /// The maximum possible `Date`.
    pub const MAX_UTC: Date<Utc> = Date {
        date: NaiveDate::MAX,
        offset: Utc,
    };
}
/// Maps the local date to other date with given conversion function.
fn map_local<Tz: TimeZone, F>(d: &Date<Tz>, mut f: F) -> Option<Date<Tz>>
where
    F: FnMut(NaiveDate) -> Option<NaiveDate>,
{
    f(d.naive_local()).and_then(|date| d.timezone().from_local_date(&date).single())
}
impl<Tz: TimeZone> Date<Tz>
where
    Tz::Offset: fmt::Display,
{
    /// Formats the date with the specified formatting items.
    #[cfg(any(feature = "alloc", feature = "std", test))]
    #[cfg_attr(docsrs, doc(cfg(any(feature = "alloc", feature = "std"))))]
    #[inline]
    #[must_use]
    pub fn format_with_items<'a, I, B>(&self, items: I) -> DelayedFormat<I>
    where
        I: Iterator<Item = B> + Clone,
        B: Borrow<Item<'a>>,
    {
        DelayedFormat::new_with_offset(
            Some(self.naive_local()),
            None,
            &self.offset,
            items,
        )
    }
    /// Formats the date with the specified format string.
    /// See the [`crate::format::strftime`] module
    /// on the supported escape sequences.
    #[cfg(any(feature = "alloc", feature = "std", test))]
    #[cfg_attr(docsrs, doc(cfg(any(feature = "alloc", feature = "std"))))]
    #[inline]
    #[must_use]
    pub fn format<'a>(&self, fmt: &'a str) -> DelayedFormat<StrftimeItems<'a>> {
        self.format_with_items(StrftimeItems::new(fmt))
    }
    /// Formats the date with the specified formatting items and locale.
    #[cfg(feature = "unstable-locales")]
    #[cfg_attr(docsrs, doc(cfg(feature = "unstable-locales")))]
    #[inline]
    #[must_use]
    pub fn format_localized_with_items<'a, I, B>(
        &self,
        items: I,
        locale: Locale,
    ) -> DelayedFormat<I>
    where
        I: Iterator<Item = B> + Clone,
        B: Borrow<Item<'a>>,
    {
        DelayedFormat::new_with_offset_and_locale(
            Some(self.naive_local()),
            None,
            &self.offset,
            items,
            locale,
        )
    }
    /// Formats the date with the specified format string and locale.
    /// See the [`crate::format::strftime`] module
    /// on the supported escape sequences.
    #[cfg(feature = "unstable-locales")]
    #[cfg_attr(docsrs, doc(cfg(feature = "unstable-locales")))]
    #[inline]
    #[must_use]
    pub fn format_localized<'a>(
        &self,
        fmt: &'a str,
        locale: Locale,
    ) -> DelayedFormat<StrftimeItems<'a>> {
        self.format_localized_with_items(
            StrftimeItems::new_with_locale(fmt, locale),
            locale,
        )
    }
}
impl<Tz: TimeZone> Datelike for Date<Tz> {
    #[inline]
    fn year(&self) -> i32 {
        self.naive_local().year()
    }
    #[inline]
    fn month(&self) -> u32 {
        self.naive_local().month()
    }
    #[inline]
    fn month0(&self) -> u32 {
        self.naive_local().month0()
    }
    #[inline]
    fn day(&self) -> u32 {
        self.naive_local().day()
    }
    #[inline]
    fn day0(&self) -> u32 {
        self.naive_local().day0()
    }
    #[inline]
    fn ordinal(&self) -> u32 {
        self.naive_local().ordinal()
    }
    #[inline]
    fn ordinal0(&self) -> u32 {
        self.naive_local().ordinal0()
    }
    #[inline]
    fn weekday(&self) -> Weekday {
        self.naive_local().weekday()
    }
    #[inline]
    fn iso_week(&self) -> IsoWeek {
        self.naive_local().iso_week()
    }
    #[inline]
    fn with_year(&self, year: i32) -> Option<Date<Tz>> {
        map_local(self, |date| date.with_year(year))
    }
    #[inline]
    fn with_month(&self, month: u32) -> Option<Date<Tz>> {
        map_local(self, |date| date.with_month(month))
    }
    #[inline]
    fn with_month0(&self, month0: u32) -> Option<Date<Tz>> {
        map_local(self, |date| date.with_month0(month0))
    }
    #[inline]
    fn with_day(&self, day: u32) -> Option<Date<Tz>> {
        map_local(self, |date| date.with_day(day))
    }
    #[inline]
    fn with_day0(&self, day0: u32) -> Option<Date<Tz>> {
        map_local(self, |date| date.with_day0(day0))
    }
    #[inline]
    fn with_ordinal(&self, ordinal: u32) -> Option<Date<Tz>> {
        map_local(self, |date| date.with_ordinal(ordinal))
    }
    #[inline]
    fn with_ordinal0(&self, ordinal0: u32) -> Option<Date<Tz>> {
        map_local(self, |date| date.with_ordinal0(ordinal0))
    }
}
impl<Tz: TimeZone> Copy for Date<Tz>
where
    <Tz as TimeZone>::Offset: Copy,
{}
unsafe impl<Tz: TimeZone> Send for Date<Tz>
where
    <Tz as TimeZone>::Offset: Send,
{}
impl<Tz: TimeZone, Tz2: TimeZone> PartialEq<Date<Tz2>> for Date<Tz> {
    fn eq(&self, other: &Date<Tz2>) -> bool {
        self.date == other.date
    }
}
impl<Tz: TimeZone> Eq for Date<Tz> {}
impl<Tz: TimeZone> PartialOrd for Date<Tz> {
    fn partial_cmp(&self, other: &Date<Tz>) -> Option<Ordering> {
        self.date.partial_cmp(&other.date)
    }
}
impl<Tz: TimeZone> Ord for Date<Tz> {
    fn cmp(&self, other: &Date<Tz>) -> Ordering {
        self.date.cmp(&other.date)
    }
}
impl<Tz: TimeZone> hash::Hash for Date<Tz> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.date.hash(state)
    }
}
impl<Tz: TimeZone> Add<TimeDelta> for Date<Tz> {
    type Output = Date<Tz>;
    #[inline]
    fn add(self, rhs: TimeDelta) -> Date<Tz> {
        self.checked_add_signed(rhs).expect("`Date + Duration` overflowed")
    }
}
impl<Tz: TimeZone> AddAssign<TimeDelta> for Date<Tz> {
    #[inline]
    fn add_assign(&mut self, rhs: TimeDelta) {
        self
            .date = self
            .date
            .checked_add_signed(rhs)
            .expect("`Date + Duration` overflowed");
    }
}
impl<Tz: TimeZone> Sub<TimeDelta> for Date<Tz> {
    type Output = Date<Tz>;
    #[inline]
    fn sub(self, rhs: TimeDelta) -> Date<Tz> {
        self.checked_sub_signed(rhs).expect("`Date - Duration` overflowed")
    }
}
impl<Tz: TimeZone> SubAssign<TimeDelta> for Date<Tz> {
    #[inline]
    fn sub_assign(&mut self, rhs: TimeDelta) {
        self
            .date = self
            .date
            .checked_sub_signed(rhs)
            .expect("`Date - Duration` overflowed");
    }
}
impl<Tz: TimeZone> Sub<Date<Tz>> for Date<Tz> {
    type Output = TimeDelta;
    #[inline]
    fn sub(self, rhs: Date<Tz>) -> TimeDelta {
        self.signed_duration_since(rhs)
    }
}
impl<Tz: TimeZone> fmt::Debug for Date<Tz> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.naive_local().fmt(f)?;
        self.offset.fmt(f)
    }
}
impl<Tz: TimeZone> fmt::Display for Date<Tz>
where
    Tz::Offset: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.naive_local().fmt(f)?;
        self.offset.fmt(f)
    }
}
#[cfg(feature = "arbitrary")]
impl<'a, Tz> arbitrary::Arbitrary<'a> for Date<Tz>
where
    Tz: TimeZone,
    <Tz as TimeZone>::Offset: arbitrary::Arbitrary<'a>,
{
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Date<Tz>> {
        let date = NaiveDate::arbitrary(u)?;
        let offset = <Tz as TimeZone>::Offset::arbitrary(u)?;
        Ok(Date::from_utc(date, offset))
    }
}
#[cfg(test)]
mod tests {
    use super::Date;
    use crate::time_delta::TimeDelta;
    use crate::{FixedOffset, NaiveDate, Utc};
    #[cfg(feature = "clock")]
    use crate::offset::{Local, TimeZone};
    #[test]
    #[cfg(feature = "clock")]
    fn test_years_elapsed() {
        const WEEKS_PER_YEAR: f32 = 52.1775;
        let one_year_ago = Utc::today()
            - TimeDelta::weeks((WEEKS_PER_YEAR * 1.5).ceil() as i64);
        let two_year_ago = Utc::today()
            - TimeDelta::weeks((WEEKS_PER_YEAR * 2.5).ceil() as i64);
        assert_eq!(Utc::today().years_since(one_year_ago), Some(1));
        assert_eq!(Utc::today().years_since(two_year_ago), Some(2));
        let future = Utc::today() + TimeDelta::weeks(12);
        assert_eq!(Utc::today().years_since(future), None);
    }
    #[test]
    fn test_date_add_assign() {
        let naivedate = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
        let date = Date::<Utc>::from_utc(naivedate, Utc);
        let mut date_add = date;
        date_add += TimeDelta::days(5);
        assert_eq!(date_add, date + TimeDelta::days(5));
        let timezone = FixedOffset::east_opt(60 * 60).unwrap();
        let date = date.with_timezone(&timezone);
        let date_add = date_add.with_timezone(&timezone);
        assert_eq!(date_add, date + TimeDelta::days(5));
        let timezone = FixedOffset::west_opt(2 * 60 * 60).unwrap();
        let date = date.with_timezone(&timezone);
        let date_add = date_add.with_timezone(&timezone);
        assert_eq!(date_add, date + TimeDelta::days(5));
    }
    #[test]
    #[cfg(feature = "clock")]
    fn test_date_add_assign_local() {
        let naivedate = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
        let date = Local.from_utc_date(&naivedate);
        let mut date_add = date;
        date_add += TimeDelta::days(5);
        assert_eq!(date_add, date + TimeDelta::days(5));
    }
    #[test]
    fn test_date_sub_assign() {
        let naivedate = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
        let date = Date::<Utc>::from_utc(naivedate, Utc);
        let mut date_sub = date;
        date_sub -= TimeDelta::days(5);
        assert_eq!(date_sub, date - TimeDelta::days(5));
        let timezone = FixedOffset::east_opt(60 * 60).unwrap();
        let date = date.with_timezone(&timezone);
        let date_sub = date_sub.with_timezone(&timezone);
        assert_eq!(date_sub, date - TimeDelta::days(5));
        let timezone = FixedOffset::west_opt(2 * 60 * 60).unwrap();
        let date = date.with_timezone(&timezone);
        let date_sub = date_sub.with_timezone(&timezone);
        assert_eq!(date_sub, date - TimeDelta::days(5));
    }
    #[test]
    #[cfg(feature = "clock")]
    fn test_date_sub_assign_local() {
        let naivedate = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
        let date = Local.from_utc_date(&naivedate);
        let mut date_sub = date;
        date_sub -= TimeDelta::days(5);
        assert_eq!(date_sub, date - TimeDelta::days(5));
    }
}
#[cfg(test)]
mod tests_rug_36 {
    use super::*;
    use crate::{NaiveDate, Date, Local, TimeZone};
    #[test]
    fn test_map_local() {
        let _rug_st_tests_rug_36_rrrruuuugggg_test_map_local = 0;
        let mut p0: Date<Local> = Local::today();
        let mut p1: fn(NaiveDate) -> Option<NaiveDate> = |date| Some(date);
        map_local(&p0, p1);
        let _rug_ed_tests_rug_36_rrrruuuugggg_test_map_local = 0;
    }
}
#[cfg(test)]
mod tests_rug_38 {
    use super::*;
    use crate::{Date, DateTime, Local, NaiveDate, NaiveTime, TimeZone};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_38_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 12;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let p0: Date<Local> = Local::today();
        let p1 = NaiveTime::from_hms(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        let _ = Date::<Local>::and_time(&p0, p1);
        let _rug_ed_tests_rug_38_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_39 {
    use super::*;
    use crate::{Date, Local, DateTime, TimeZone};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_39_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 12;
        let rug_fuzz_1 = 30;
        let rug_fuzz_2 = 45;
        let mut p0: Date<Local> = Local::today();
        let mut p1: u32 = rug_fuzz_0;
        let mut p2: u32 = rug_fuzz_1;
        let mut p3: u32 = rug_fuzz_2;
        <Date<Local>>::and_hms(&p0, p1, p2, p3);
        let _rug_ed_tests_rug_39_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_40 {
    use super::*;
    use crate::Date;
    use crate::offset::TimeZone;
    use crate::offset::Local;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_40_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 30;
        let rug_fuzz_2 = 5;
        let date: Date<Local> = Local::today();
        let hour: u32 = rug_fuzz_0;
        let minute: u32 = rug_fuzz_1;
        let second: u32 = rug_fuzz_2;
        date.and_hms_opt(hour, minute, second);
        let _rug_ed_tests_rug_40_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_41 {
    use super::*;
    use crate::{Date, DateTime, Local, TimeZone};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_41_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 12;
        let rug_fuzz_1 = 30;
        let rug_fuzz_2 = 45;
        let rug_fuzz_3 = 500;
        let mut p0: Date<Local> = Local::now().date();
        let mut p1: u32 = rug_fuzz_0;
        let mut p2: u32 = rug_fuzz_1;
        let mut p3: u32 = rug_fuzz_2;
        let mut p4: u32 = rug_fuzz_3;
        p0.and_hms_milli(p1, p2, p3, p4);
        let _rug_ed_tests_rug_41_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_42 {
    use super::*;
    use crate::offset::TimeZone;
    use crate::naive::NaiveDate;
    use crate::{DateTime, Local};
    #[test]
    fn test_and_hms_milli_opt() {
        let _rug_st_tests_rug_42_rrrruuuugggg_test_and_hms_milli_opt = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 30;
        let rug_fuzz_2 = 45;
        let rug_fuzz_3 = 500;
        let date = Local::now().date();
        let hour: u32 = rug_fuzz_0;
        let min: u32 = rug_fuzz_1;
        let sec: u32 = rug_fuzz_2;
        let milli: u32 = rug_fuzz_3;
        date.and_hms_milli_opt(hour, min, sec, milli);
        let _rug_ed_tests_rug_42_rrrruuuugggg_test_and_hms_milli_opt = 0;
    }
}
#[cfg(test)]
mod tests_rug_43 {
    use super::*;
    use crate::{Date, Local, TimeZone};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_43_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 12;
        let rug_fuzz_1 = 30;
        let rug_fuzz_2 = 45;
        let rug_fuzz_3 = 123456;
        let mut p0: Date<Local> = Local::today();
        let mut p1: u32 = rug_fuzz_0;
        let mut p2: u32 = rug_fuzz_1;
        let mut p3: u32 = rug_fuzz_2;
        let mut p4: u32 = rug_fuzz_3;
        p0.and_hms_micro(p1, p2, p3, p4);
        let _rug_ed_tests_rug_43_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_44 {
    use super::*;
    use crate::{Date, DateTime, Datelike, Local, NaiveTime, Timelike, TimeZone};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_44_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 12;
        let rug_fuzz_1 = 34;
        let rug_fuzz_2 = 56;
        let rug_fuzz_3 = 789;
        let date: Date<Local> = Local::now().date();
        let hour: u32 = rug_fuzz_0;
        let min: u32 = rug_fuzz_1;
        let sec: u32 = rug_fuzz_2;
        let micro: u32 = rug_fuzz_3;
        date.and_hms_micro_opt(hour, min, sec, micro);
        let _rug_ed_tests_rug_44_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_45 {
    use super::*;
    use crate::{DateTime, NaiveDate, NaiveTime, Utc};
    #[test]
    fn test_and_hms_nano() {
        let _rug_st_tests_rug_45_rrrruuuugggg_test_and_hms_nano = 0;
        let rug_fuzz_0 = 2022;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 12;
        let rug_fuzz_4 = 34;
        let rug_fuzz_5 = 56;
        let rug_fuzz_6 = 789;
        let date: Date<Utc> = Date::from_utc(
            NaiveDate::from_ymd(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2),
            Utc,
        );
        let hour: u32 = rug_fuzz_3;
        let min: u32 = rug_fuzz_4;
        let sec: u32 = rug_fuzz_5;
        let nano: u32 = rug_fuzz_6;
        date.and_hms_nano(hour, min, sec, nano);
        let _rug_ed_tests_rug_45_rrrruuuugggg_test_and_hms_nano = 0;
    }
}
#[cfg(test)]
mod tests_rug_46 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_46_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 12;
        let rug_fuzz_1 = 34;
        let rug_fuzz_2 = 56;
        let rug_fuzz_3 = 789;
        let date: Date<Local> = Local::now().date();
        let hour: u32 = rug_fuzz_0;
        let min: u32 = rug_fuzz_1;
        let sec: u32 = rug_fuzz_2;
        let nano: u32 = rug_fuzz_3;
        date.and_hms_nano_opt(hour, min, sec, nano);
        let _rug_ed_tests_rug_46_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_47 {
    use super::*;
    use crate::{Date, FixedOffset, TimeZone};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_47_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 2022;
        let rug_fuzz_2 = 12;
        let rug_fuzz_3 = 31;
        let mut p0: Date<FixedOffset> = FixedOffset::east(rug_fuzz_0)
            .ymd(rug_fuzz_1, rug_fuzz_2, rug_fuzz_3);
        Date::<FixedOffset>::succ(&p0);
        let _rug_ed_tests_rug_47_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_48 {
    use super::*;
    use crate::{Date, TimeZone, Utc};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_48_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 2022;
        let rug_fuzz_1 = 12;
        let rug_fuzz_2 = 31;
        let p0: Date<Utc> = Utc.ymd(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        Date::<Utc>::succ_opt(&p0);
        let _rug_ed_tests_rug_48_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_50 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_50_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3600;
        let rug_fuzz_1 = 2022;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 1;
        let tz = FixedOffset::east(rug_fuzz_0);
        let offset = tz.clone();
        let date = Date::<
            FixedOffset,
        >::from_utc(NaiveDate::from_ymd(rug_fuzz_1, rug_fuzz_2, rug_fuzz_3), offset);
        date.pred_opt();
        let _rug_ed_tests_rug_50_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_51 {
    use super::*;
    use crate::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Offset, TimeZone, Utc};
    #[test]
    fn test_offset() {
        let _rug_st_tests_rug_51_rrrruuuugggg_test_offset = 0;
        let mut p0: DateTime<Utc> = Utc::now();
        let result = p0.offset();
        let _rug_ed_tests_rug_51_rrrruuuugggg_test_offset = 0;
    }
}
#[cfg(test)]
mod tests_rug_52 {
    use super::*;
    use crate::{Date, TimeZone, Local};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_52_rrrruuuugggg_test_rug = 0;
        let mut p0: Date<Local> = Local::today();
        Date::<Local>::timezone(&p0);
        let _rug_ed_tests_rug_52_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_56 {
    use super::*;
    use crate::{Date, TimeZone, Utc};
    #[test]
    fn test_signed_duration_since() {
        let _rug_st_tests_rug_56_rrrruuuugggg_test_signed_duration_since = 0;
        let mut p0: Date<Utc> = Utc::today();
        let mut p1: Date<Utc> = Utc::today();
        p0.signed_duration_since(p1);
        let _rug_ed_tests_rug_56_rrrruuuugggg_test_signed_duration_since = 0;
    }
}
#[cfg(test)]
mod tests_rug_57 {
    use super::*;
    use crate::{Date, NaiveDate, Utc};
    #[test]
    fn test_naive_utc() {
        let _rug_st_tests_rug_57_rrrruuuugggg_test_naive_utc = 0;
        let rug_fuzz_0 = 2022;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 1;
        let p0: Date<Utc> = Date::from_utc(
            NaiveDate::from_ymd(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2),
            Utc,
        );
        <Date<Utc>>::naive_utc(&p0);
        let _rug_ed_tests_rug_57_rrrruuuugggg_test_naive_utc = 0;
    }
}
#[cfg(test)]
mod tests_rug_58 {
    use super::*;
    use crate::{Date, TimeZone, NaiveDate, Utc};
    #[test]
    fn test_naive_local() {
        let _rug_st_tests_rug_58_rrrruuuugggg_test_naive_local = 0;
        let rug_fuzz_0 = 2022;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 1;
        let tz = Utc;
        let date: NaiveDate = NaiveDate::from_ymd(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        let dt: Date<Utc> = tz.from_local_date(&date).unwrap().with_timezone(&Utc);
        let result: NaiveDate = dt.naive_local();
        debug_assert_eq!(result, date);
        let _rug_ed_tests_rug_58_rrrruuuugggg_test_naive_local = 0;
    }
}
#[cfg(test)]
mod tests_rug_59 {
    use super::*;
    use crate::{Date, Local};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_59_rrrruuuugggg_test_rug = 0;
        let mut p0: Date<Local> = Local::now().date();
        let mut p1: Date<Local> = Local::now().date();
        p0.years_since(p1);
        let _rug_ed_tests_rug_59_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_60 {
    use super::*;
    use crate::format::strftime::StrftimeItems;
    use crate::format::DelayedFormat;
    use crate::{Date, Local};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_60_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "your strftime format string";
        let p0: Date<Local> = Local::today();
        let p1: StrftimeItems = StrftimeItems::new(rug_fuzz_0);
        Date::<Local>::format_with_items(&p0, p1);
        let _rug_ed_tests_rug_60_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_61 {
    use super::*;
    use crate::{Date, Timelike, Utc};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_61_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "%Y-%m-%d %H:%M:%S";
        let p0: Date<Utc> = Utc::now().date();
        let p1: &str = rug_fuzz_0;
        p0.format(p1);
        let _rug_ed_tests_rug_61_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_62 {
    use super::*;
    use crate::{Date, NaiveDate, Utc};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_62_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 2022;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 1;
        let p0: Date<Utc> = Date::from_utc(
            NaiveDate::from_ymd(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2),
            Utc,
        );
        p0.year();
        let _rug_ed_tests_rug_62_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_64 {
    use super::*;
    use crate::Datelike;
    use crate::{Date, Utc};
    #[test]
    fn test_month0() {
        let _rug_st_tests_rug_64_rrrruuuugggg_test_month0 = 0;
        let rug_fuzz_0 = 2022;
        let rug_fuzz_1 = 3;
        let rug_fuzz_2 = 11;
        let mut p0: Date<Utc> = Date::from_utc(
            NaiveDate::from_ymd(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2),
            Utc,
        );
        p0.month0();
        let _rug_ed_tests_rug_64_rrrruuuugggg_test_month0 = 0;
    }
}
#[cfg(test)]
mod tests_rug_65 {
    use super::*;
    use crate::Datelike;
    use crate::{Date, TimeZone, Local};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_65_rrrruuuugggg_test_rug = 0;
        let mut p0: Date<Local> = Local::now().date();
        p0.day();
        let _rug_ed_tests_rug_65_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_66 {
    use super::*;
    use crate::{Date, Datelike, DateTime, Local, NaiveDate, TimeZone};
    #[test]
    fn test_day0() {
        let _rug_st_tests_rug_66_rrrruuuugggg_test_day0 = 0;
        let local: DateTime<Local> = Local::now();
        let date: Date<Local> = local.date();
        date.day0();
        let _rug_ed_tests_rug_66_rrrruuuugggg_test_day0 = 0;
    }
}
#[cfg(test)]
mod tests_rug_67 {
    use super::*;
    use crate::Datelike;
    use crate::Date;
    use crate::Local;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_67_rrrruuuugggg_test_rug = 0;
        let mut p0: Date<Local> = Local::now().date();
        p0.ordinal();
        let _rug_ed_tests_rug_67_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_68 {
    use super::*;
    use crate::{Date, Datelike, Local};
    #[test]
    fn test_ordinal0() {
        let _rug_st_tests_rug_68_rrrruuuugggg_test_ordinal0 = 0;
        let mut p0: Date<Local> = Local::now().date();
        <Date<Local> as Datelike>::ordinal0(&p0);
        let _rug_ed_tests_rug_68_rrrruuuugggg_test_ordinal0 = 0;
    }
}
#[cfg(test)]
mod tests_rug_71 {
    use super::*;
    use crate::{Date, Datelike, TimeZone, Utc};
    #[test]
    fn test_with_year() {
        let _rug_st_tests_rug_71_rrrruuuugggg_test_with_year = 0;
        let rug_fuzz_0 = 2022;
        let mut p0: Date<Utc> = Utc::today();
        let p1: i32 = rug_fuzz_0;
        p0.with_year(p1);
        let _rug_ed_tests_rug_71_rrrruuuugggg_test_with_year = 0;
    }
}
#[cfg(test)]
mod tests_rug_72 {
    use super::*;
    use crate::{Date, Datelike, Local, LocalResult};
    #[test]
    fn test_with_month() {
        let _rug_st_tests_rug_72_rrrruuuugggg_test_with_month = 0;
        let rug_fuzz_0 = 10;
        let p0: Date<Local> = Local::now().date();
        let p1: u32 = rug_fuzz_0;
        p0.with_month(p1);
        let _rug_ed_tests_rug_72_rrrruuuugggg_test_with_month = 0;
    }
}
#[cfg(test)]
mod tests_rug_73 {
    use super::*;
    use crate::Datelike;
    use crate::Date;
    use crate::Local;
    #[test]
    fn test_with_month0() {
        let _rug_st_tests_rug_73_rrrruuuugggg_test_with_month0 = 0;
        let rug_fuzz_0 = 11;
        let mut p0: Date<Local> = Local::now().date();
        let mut p1: u32 = rug_fuzz_0;
        p0.with_month0(p1);
        let _rug_ed_tests_rug_73_rrrruuuugggg_test_with_month0 = 0;
    }
}
#[cfg(test)]
mod tests_rug_74 {
    use super::*;
    use crate::Datelike;
    use crate::{Date, TimeZone, Utc};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_74_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 2022;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 15;
        let mut p0: Date<Utc> = Utc.ymd(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        let p1: u32 = rug_fuzz_3;
        p0.with_day(p1);
        let _rug_ed_tests_rug_74_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_76 {
    use super::*;
    use crate::{Date, Datelike};
    use crate::{DateTime, Utc, TimeZone};
    #[test]
    fn test_with_ordinal() {
        let _rug_st_tests_rug_76_rrrruuuugggg_test_with_ordinal = 0;
        let rug_fuzz_0 = 2459549;
        let p0: DateTime<Utc> = Utc::now();
        let p1: u32 = rug_fuzz_0;
        p0.with_ordinal(p1);
        let _rug_ed_tests_rug_76_rrrruuuugggg_test_with_ordinal = 0;
    }
}
#[cfg(test)]
mod tests_rug_78 {
    use super::*;
    use crate::{Date, TimeZone, Utc};
    #[test]
    fn test_eq() {
        let _rug_st_tests_rug_78_rrrruuuugggg_test_eq = 0;
        let mut p0: Date<Utc> = Utc::today();
        let mut p1: Date<Utc> = Utc::today();
        debug_assert_eq!(
            < Date < Utc > as std::cmp::PartialEq < Date < Utc > > > ::eq(& p0, & p1),
            true
        );
        let _rug_ed_tests_rug_78_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_rug_80 {
    use super::*;
    use crate::{Date, TimeZone, Utc};
    use std::cmp::Ordering;
    #[test]
    fn test_cmp() {
        let _rug_st_tests_rug_80_rrrruuuugggg_test_cmp = 0;
        let rug_fuzz_0 = 2022;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 2022;
        let rug_fuzz_4 = 12;
        let rug_fuzz_5 = 31;
        let tz = Utc;
        let p0: Date<Utc> = tz.ymd(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        let p1: Date<Utc> = tz.ymd(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5);
        debug_assert_eq!(p0.cmp(& p1), Ordering::Less);
        debug_assert_eq!(p1.cmp(& p0), Ordering::Greater);
        debug_assert_eq!(p0.cmp(& p0), Ordering::Equal);
        let _rug_ed_tests_rug_80_rrrruuuugggg_test_cmp = 0;
    }
}
#[cfg(test)]
mod tests_rug_86 {
    use super::*;
    use crate::{Date, TimeZone, Utc};
    use std::ops::Sub;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_86_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 2020;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 2020;
        let rug_fuzz_4 = 1;
        let rug_fuzz_5 = 2;
        let p0: Date<Utc> = Utc.ymd(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        let p1: Date<Utc> = Utc.ymd(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5);
        <Date<Utc> as Sub>::sub(p0, p1);
        let _rug_ed_tests_rug_86_rrrruuuugggg_test_rug = 0;
    }
}
