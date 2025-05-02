//! Formatting helpers for a `Date`.
#![allow(non_snake_case)]
use crate::{
    error,
    format::{
        parse::{
            consume_padding, try_consume_digits, try_consume_exact_digits,
            try_consume_first_match,
        },
        Padding, ParseResult, ParsedItems,
    },
    Date, Weekday,
};
#[cfg(not(feature = "std"))]
use alloc::string::ToString;
use core::{
    fmt::{self, Formatter},
    num::{NonZeroU16, NonZeroU8},
};
#[allow(unused_imports)]
use standback::prelude::*;
/// Array of weekdays that corresponds to the localized values. This can be
/// zipped via an iterator to perform parsing easily.
const WEEKDAYS: [Weekday; 7] = [
    Weekday::Monday,
    Weekday::Tuesday,
    Weekday::Wednesday,
    Weekday::Thursday,
    Weekday::Friday,
    Weekday::Saturday,
    Weekday::Sunday,
];
/// Full weekday names
const WEEKDAYS_FULL: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];
/// Abbreviated weekday names
const WEEKDAYS_ABBR: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
/// Full month names
const MONTHS_FULL: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
/// Abbreviated month names
const MONTHS_ABBR: [&str; 12] = [
    "Jan",
    "Feb",
    "Mar",
    "Apr",
    "May",
    "Jun",
    "Jul",
    "Aug",
    "Sep",
    "Oct",
    "Nov",
    "Dec",
];
/// Short day of the week
pub(crate) fn fmt_a(f: &mut Formatter<'_>, date: Date) -> fmt::Result {
    f.write_str(WEEKDAYS_ABBR[date.weekday().number_days_from_monday() as usize])
}
/// Short day of the week
pub(crate) fn parse_a(items: &mut ParsedItems, s: &mut &str) -> ParseResult<()> {
    items
        .weekday = Some(
        try_consume_first_match(s, WEEKDAYS_ABBR.iter().zip(WEEKDAYS.iter().cloned()))
            .ok_or(error::Parse::InvalidDayOfWeek)?,
    );
    Ok(())
}
/// Day of the week
pub(crate) fn fmt_A(f: &mut Formatter<'_>, date: Date) -> fmt::Result {
    f.write_str(WEEKDAYS_FULL[date.weekday().number_days_from_monday() as usize])
}
/// Day of the week
pub(crate) fn parse_A(items: &mut ParsedItems, s: &mut &str) -> ParseResult<()> {
    items
        .weekday = Some(
        try_consume_first_match(s, WEEKDAYS_FULL.iter().zip(WEEKDAYS.iter().cloned()))
            .ok_or(error::Parse::InvalidDayOfWeek)?,
    );
    Ok(())
}
/// Short month name
pub(crate) fn fmt_b(f: &mut Formatter<'_>, date: Date) -> fmt::Result {
    f.write_str(MONTHS_ABBR[date.month() as usize - 1])
}
/// Short month name
pub(crate) fn parse_b(items: &mut ParsedItems, s: &mut &str) -> ParseResult<()> {
    items
        .month = Some(
        try_consume_first_match(s, MONTHS_ABBR.iter().cloned().zip(1..))
            .and_then(NonZeroU8::new)
            .ok_or(error::Parse::InvalidMonth)?,
    );
    Ok(())
}
/// Month name
pub(crate) fn fmt_B(f: &mut Formatter<'_>, date: Date) -> fmt::Result {
    f.write_str(MONTHS_FULL[date.month() as usize - 1])
}
/// Month name
pub(crate) fn parse_B(items: &mut ParsedItems, s: &mut &str) -> ParseResult<()> {
    items
        .month = Some(
        try_consume_first_match(s, MONTHS_FULL.iter().cloned().zip(1..))
            .and_then(NonZeroU8::new)
            .ok_or(error::Parse::InvalidMonth)?,
    );
    Ok(())
}
/// Year divided by 100 and truncated to integer (`00`-`999`)
pub(crate) fn fmt_C(f: &mut Formatter<'_>, date: Date, padding: Padding) -> fmt::Result {
    pad!(f, padding, 2, date.year() / 100)
}
/// Year divided by 100 and truncated to integer (`00`-`999`)
pub(crate) fn parse_C(
    items: &mut ParsedItems,
    s: &mut &str,
    padding: Padding,
) -> ParseResult<()> {
    let padding_length = consume_padding(s, padding, 1);
    items
        .year = Some(
        try_consume_digits::<i32>(s, 2 - padding_length, 3 - padding_length)
            .ok_or(error::Parse::InvalidYear)? * 100
            + items.year.unwrap_or(0).rem_euclid(100),
    );
    Ok(())
}
/// Day of the month, zero-padded (`01`-`31`)
pub(crate) fn fmt_d(f: &mut Formatter<'_>, date: Date, padding: Padding) -> fmt::Result {
    pad!(f, padding, 2, date.day())
}
/// Day of the month, zero-padded (`01`-`31`)
pub(crate) fn parse_d(
    items: &mut ParsedItems,
    s: &mut &str,
    padding: Padding,
) -> ParseResult<()> {
    items
        .day = Some(
        try_consume_exact_digits(s, 2, padding)
            .and_then(NonZeroU8::new)
            .ok_or(error::Parse::InvalidDayOfMonth)?,
    );
    Ok(())
}
/// Week-based year, last two digits (`00`-`99`)
pub(crate) fn fmt_g(f: &mut Formatter<'_>, date: Date, padding: Padding) -> fmt::Result {
    pad!(f, padding, 2, date.iso_year_week().0.rem_euclid(100))
}
/// Week-based year, last two digits (`00`-`99`)
pub(crate) fn parse_g(
    items: &mut ParsedItems,
    s: &mut &str,
    padding: Padding,
) -> ParseResult<()> {
    items
        .week_based_year = Some(
        items.week_based_year.unwrap_or(0) / 100 * 100
            + try_consume_exact_digits::<i32>(s, 2, padding)
                .ok_or(error::Parse::InvalidYear)?,
    );
    Ok(())
}
/// Week-based year
pub(crate) fn fmt_G(f: &mut Formatter<'_>, date: Date, padding: Padding) -> fmt::Result {
    let year = date.iso_year_week().0;
    if year >= 10_000 {
        f.write_str("+")?;
    }
    pad!(f, padding, 4, year)
}
/// Week-based year
pub(crate) fn parse_G(
    items: &mut ParsedItems,
    s: &mut &str,
    padding: Padding,
) -> ParseResult<()> {
    let sign = try_consume_first_match(s, [("+", 1), ("-", -1)].iter().cloned())
        .unwrap_or(1);
    consume_padding(s, padding, 4);
    items
        .week_based_year = Some(
        try_consume_digits(s, 1, 6)
            .map(|v: i32| sign * v)
            .ok_or(error::Parse::InvalidYear)?,
    );
    Ok(())
}
/// Day of the year, zero-padded to width 3 (`001`-`366`)
pub(crate) fn fmt_j(f: &mut Formatter<'_>, date: Date, padding: Padding) -> fmt::Result {
    pad!(f, padding, 3, date.ordinal())
}
/// Day of the year, zero-padded to width 3 (`001`-`366`)
pub(crate) fn parse_j(
    items: &mut ParsedItems,
    s: &mut &str,
    padding: Padding,
) -> ParseResult<()> {
    items
        .ordinal_day = Some(
        try_consume_exact_digits(s, 3, padding)
            .and_then(NonZeroU16::new)
            .ok_or(error::Parse::InvalidDayOfYear)?,
    );
    Ok(())
}
/// Month of the year, zero-padded (`01`-`12`)
pub(crate) fn fmt_m(f: &mut Formatter<'_>, date: Date, padding: Padding) -> fmt::Result {
    pad!(f, padding, 2, date.month())
}
/// Month of the year, zero-padded (`01`-`12`)
pub(crate) fn parse_m(
    items: &mut ParsedItems,
    s: &mut &str,
    padding: Padding,
) -> ParseResult<()> {
    items
        .month = Some(
        try_consume_exact_digits(s, 2, padding)
            .and_then(NonZeroU8::new)
            .ok_or(error::Parse::InvalidMonth)?,
    );
    Ok(())
}
/// ISO weekday (Monday = `1`, Sunday = `7`)
pub(crate) fn fmt_u(f: &mut Formatter<'_>, date: Date) -> fmt::Result {
    write!(f, "{}", date.weekday().iso_weekday_number())
}
/// ISO weekday (Monday = `1`, Sunday = `7`)
pub(crate) fn parse_u(items: &mut ParsedItems, s: &mut &str) -> ParseResult<()> {
    items
        .weekday = Some(
        try_consume_first_match(
                s,
                (1..).map(|d| d.to_string()).zip(WEEKDAYS.iter().cloned()),
            )
            .ok_or(error::Parse::InvalidDayOfWeek)?,
    );
    Ok(())
}
/// Sunday-based week number (`00`-`53`)
pub(crate) fn fmt_U(f: &mut Formatter<'_>, date: Date, padding: Padding) -> fmt::Result {
    pad!(f, padding, 2, date.sunday_based_week())
}
/// Sunday-based week number (`00`-`53`)
pub(crate) fn parse_U(
    items: &mut ParsedItems,
    s: &mut &str,
    padding: Padding,
) -> ParseResult<()> {
    items
        .sunday_week = Some(
        try_consume_exact_digits(s, 2, padding).ok_or(error::Parse::InvalidWeek)?,
    );
    Ok(())
}
/// ISO week number, zero-padded (`01`-`53`)
pub(crate) fn fmt_V(f: &mut Formatter<'_>, date: Date, padding: Padding) -> fmt::Result {
    pad!(f, padding, 2, date.week())
}
/// ISO week number, zero-padded (`01`-`53`)
pub(crate) fn parse_V(
    items: &mut ParsedItems,
    s: &mut &str,
    padding: Padding,
) -> ParseResult<()> {
    items
        .iso_week = Some(
        try_consume_exact_digits(s, 2, padding)
            .and_then(NonZeroU8::new)
            .ok_or(error::Parse::InvalidWeek)?,
    );
    Ok(())
}
/// Weekday number (Sunday = `0`, Saturday = `6`)
pub(crate) fn fmt_w(f: &mut Formatter<'_>, date: Date) -> fmt::Result {
    write!(f, "{}", date.weekday().number_days_from_sunday())
}
/// Weekday number (Sunday = `0`, Saturday = `6`)
pub(crate) fn parse_w(items: &mut ParsedItems, s: &mut &str) -> ParseResult<()> {
    let mut weekdays = WEEKDAYS;
    weekdays.rotate_left(1);
    items
        .weekday = Some(
        try_consume_first_match(
                s,
                (0..).map(|d: u8| d.to_string()).zip(weekdays.iter().cloned()),
            )
            .ok_or(error::Parse::InvalidDayOfWeek)?,
    );
    Ok(())
}
/// Monday-based week number (`00`-`53`)
pub(crate) fn fmt_W(f: &mut Formatter<'_>, date: Date, padding: Padding) -> fmt::Result {
    pad!(f, padding, 2, date.monday_based_week())
}
/// Monday-based week number (`00`-`53`)
pub(crate) fn parse_W(
    items: &mut ParsedItems,
    s: &mut &str,
    padding: Padding,
) -> ParseResult<()> {
    items
        .monday_week = Some(
        try_consume_exact_digits(s, 2, padding).ok_or(error::Parse::InvalidWeek)?,
    );
    Ok(())
}
/// Last two digits of year (`00`-`99`)
pub(crate) fn fmt_y(f: &mut Formatter<'_>, date: Date, padding: Padding) -> fmt::Result {
    pad!(f, padding, 2, date.year().rem_euclid(100))
}
/// Last two digits of year (`00`-`99`)
pub(crate) fn parse_y(
    items: &mut ParsedItems,
    s: &mut &str,
    padding: Padding,
) -> ParseResult<()> {
    items
        .year = Some(
        items.year.unwrap_or(0) / 100 * 100
            + try_consume_exact_digits::<i32>(s, 2, padding)
                .ok_or(error::Parse::InvalidYear)?,
    );
    Ok(())
}
/// Full year
pub(crate) fn fmt_Y(f: &mut Formatter<'_>, date: Date, padding: Padding) -> fmt::Result {
    let year = date.year();
    if year >= 10_000 {
        f.write_str("+")?;
    }
    pad!(f, padding, 4, year)
}
/// Full year
pub(crate) fn parse_Y(
    items: &mut ParsedItems,
    s: &mut &str,
    padding: Padding,
) -> ParseResult<()> {
    let (sign, max_digits) = try_consume_first_match(
            s,
            [("+", (1, 6)), ("-", (-1, 6))].iter().cloned(),
        )
        .unwrap_or((1, 4));
    consume_padding(s, padding, 3);
    items
        .year = Some(
        try_consume_digits(s, 1, max_digits)
            .map(|v: i32| sign * v)
            .ok_or(error::Parse::InvalidYear)?,
    );
    Ok(())
}
#[cfg(test)]
mod tests_llm_16_804 {
    use super::*;
    use crate::*;
    use crate::format::error;
    #[test]
    fn test_parse_B() {
        let _rug_st_tests_llm_16_804_rrrruuuugggg_test_parse_B = 0;
        let rug_fuzz_0 = "January";
        let rug_fuzz_1 = "February";
        let mut items = ParsedItems::new();
        let mut s = rug_fuzz_0;
        parse_B(&mut items, &mut s).unwrap();
        debug_assert_eq!(items.month, Some(NonZeroU8::new(1).unwrap()));
        let mut items = ParsedItems::new();
        let mut s = rug_fuzz_1;
        parse_B(&mut items, &mut s).unwrap();
        debug_assert_eq!(items.month, Some(NonZeroU8::new(2).unwrap()));
        let _rug_ed_tests_llm_16_804_rrrruuuugggg_test_parse_B = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_805 {
    use super::*;
    use crate::*;
    use crate::format::date::Padding;
    #[test]
    fn test_parse_C() {
        let _rug_st_tests_llm_16_805_rrrruuuugggg_test_parse_C = 0;
        let rug_fuzz_0 = "123";
        let mut items = ParsedItems::new();
        let mut s = rug_fuzz_0;
        let padding = Padding::None;
        let result = parse_C(&mut items, &mut s, padding);
        debug_assert_eq!(result, Ok(()));
        debug_assert_eq!(items.year, Some(123));
        let _rug_ed_tests_llm_16_805_rrrruuuugggg_test_parse_C = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_808 {
    use super::*;
    use crate::*;
    use crate::format::Padding;
    #[test]
    fn test_parse_U() {
        let _rug_st_tests_llm_16_808_rrrruuuugggg_test_parse_U = 0;
        let rug_fuzz_0 = "42";
        let mut items = ParsedItems::new();
        let mut s = rug_fuzz_0;
        let padding = Padding::None;
        parse_U(&mut items, &mut s, padding).unwrap();
        debug_assert_eq!(items.sunday_week, Some(42));
        let _rug_ed_tests_llm_16_808_rrrruuuugggg_test_parse_U = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_809 {
    use crate::format::date::parse_V;
    use crate::format::parse::ParsedItems;
    use crate::format::Padding;
    use std::num::NonZeroU8;
    use crate::error;
    use std::num::NonZeroU16;
    #[test]
    fn test_parse_V() {
        let _rug_st_tests_llm_16_809_rrrruuuugggg_test_parse_V = 0;
        let rug_fuzz_0 = "08";
        let rug_fuzz_1 = "3";
        let rug_fuzz_2 = "53";
        let rug_fuzz_3 = "100";
        let rug_fuzz_4 = "";
        let mut items = ParsedItems::new();
        let mut s = rug_fuzz_0;
        let padding = Padding::Zero;
        let result = parse_V(&mut items, &mut s, padding);
        debug_assert_eq!(result, Ok(()));
        debug_assert_eq!(items.iso_week, Some(NonZeroU8::new(8).unwrap()));
        debug_assert_eq!(s, "");
        let mut items = ParsedItems::new();
        let mut s = rug_fuzz_1;
        let padding = Padding::None;
        let result = parse_V(&mut items, &mut s, padding);
        debug_assert_eq!(result, Ok(()));
        debug_assert_eq!(items.iso_week, Some(NonZeroU8::new(3).unwrap()));
        debug_assert_eq!(s, "");
        let mut items = ParsedItems::new();
        let mut s = rug_fuzz_2;
        let padding = Padding::Zero;
        let result = parse_V(&mut items, &mut s, padding);
        debug_assert_eq!(result, Ok(()));
        debug_assert_eq!(items.iso_week, Some(NonZeroU8::new(53).unwrap()));
        debug_assert_eq!(s, "");
        let mut items = ParsedItems::new();
        let mut s = rug_fuzz_3;
        let padding = Padding::None;
        let result = parse_V(&mut items, &mut s, padding);
        debug_assert_eq!(result, Ok(()));
        debug_assert_eq!(items.iso_week, Some(NonZeroU8::new(100).unwrap()));
        debug_assert_eq!(s, "");
        let mut items = ParsedItems::new();
        let mut s = rug_fuzz_4;
        let padding = Padding::Zero;
        let result = parse_V(&mut items, &mut s, padding);
        debug_assert_eq!(result, Err(error::Parse::InvalidWeek));
        debug_assert_eq!(s, "");
        let _rug_ed_tests_llm_16_809_rrrruuuugggg_test_parse_V = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_810 {
    use super::*;
    use crate::*;
    use format::parse::ParsedItems;
    #[test]
    fn test_parse_W() {
        let _rug_st_tests_llm_16_810_rrrruuuugggg_test_parse_W = 0;
        let rug_fuzz_0 = "20";
        let mut items = ParsedItems::new();
        let mut s = rug_fuzz_0;
        let padding = Padding::Space;
        parse_W(&mut items, &mut s, padding).unwrap();
        debug_assert_eq!(items.monday_week, Some(20));
        let _rug_ed_tests_llm_16_810_rrrruuuugggg_test_parse_W = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_811 {
    use super::*;
    use crate::*;
    use crate::format::parse::ParsedItems;
    use crate::format::Padding;
    #[test]
    fn test_parse_Y() {
        let _rug_st_tests_llm_16_811_rrrruuuugggg_test_parse_Y = 0;
        let rug_fuzz_0 = "";
        let mut items = ParsedItems::new();
        let mut s = rug_fuzz_0;
        let padding = Padding::None;
        parse_Y(&mut items, &mut s, padding).unwrap();
        debug_assert_eq!(items.year, Some(0));
        let _rug_ed_tests_llm_16_811_rrrruuuugggg_test_parse_Y = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_817_llm_16_816 {
    use crate::format::date::{parse_d, Padding};
    use crate::format::parse::ParsedItems;
    use std::num::NonZeroU8;
    #[test]
    fn test_parse_d() {
        let _rug_st_tests_llm_16_817_llm_16_816_rrrruuuugggg_test_parse_d = 0;
        let rug_fuzz_0 = "05";
        let mut items = ParsedItems::new();
        let mut s = rug_fuzz_0;
        let padding = Padding::Zero;
        let result = parse_d(&mut items, &mut s, padding);
        debug_assert_eq!(result, Ok(()));
        debug_assert_eq!(items.day, Some(NonZeroU8::new(5).unwrap()));
        let _rug_ed_tests_llm_16_817_llm_16_816_rrrruuuugggg_test_parse_d = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_818 {
    use super::*;
    use crate::*;
    #[test]
    fn test_parse_g() {
        let _rug_st_tests_llm_16_818_rrrruuuugggg_test_parse_g = 0;
        let rug_fuzz_0 = "21-12-31";
        let mut items = ParsedItems::new();
        let mut s = rug_fuzz_0;
        let padding = Padding::None;
        parse_g(&mut items, &mut s, padding).unwrap();
        debug_assert_eq!(items.week_based_year, Some(2100));
        let _rug_ed_tests_llm_16_818_rrrruuuugggg_test_parse_g = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_820 {
    use super::*;
    use crate::*;
    use format::Padding;
    use std::num::NonZeroU16;
    #[test]
    fn test_parse_j() {
        let _rug_st_tests_llm_16_820_rrrruuuugggg_test_parse_j = 0;
        let rug_fuzz_0 = "123";
        let mut items = ParsedItems::new();
        let mut s = rug_fuzz_0;
        let padding = Padding::None;
        let result = parse_j(&mut items, &mut s, padding);
        debug_assert_eq!(result, Ok(()));
        debug_assert_eq!(items.ordinal_day, Some(NonZeroU16::new(123).unwrap()));
        let _rug_ed_tests_llm_16_820_rrrruuuugggg_test_parse_j = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_821 {
    use super::*;
    use crate::*;
    use crate::format::error::Parse;
    #[test]
    fn test_parse_m() {
        let _rug_st_tests_llm_16_821_rrrruuuugggg_test_parse_m = 0;
        let rug_fuzz_0 = "01";
        let rug_fuzz_1 = 1;
        let mut items = ParsedItems::new();
        let mut s = rug_fuzz_0;
        let padding = Padding::None;
        debug_assert_eq!(Ok(()), parse_m(& mut items, & mut s, padding));
        debug_assert_eq!(Some(NonZeroU8::new(rug_fuzz_1).unwrap()), items.month);
        let _rug_ed_tests_llm_16_821_rrrruuuugggg_test_parse_m = 0;
    }
    #[test]
    fn test_parse_m_invalid() {
        let _rug_st_tests_llm_16_821_rrrruuuugggg_test_parse_m_invalid = 0;
        let rug_fuzz_0 = "13";
        let mut items = ParsedItems::new();
        let mut s = rug_fuzz_0;
        let padding = Padding::None;
        debug_assert_eq!(
            Err(Parse::InvalidMonth), parse_m(& mut items, & mut s, padding)
        );
        debug_assert_eq!(None, items.month);
        let _rug_ed_tests_llm_16_821_rrrruuuugggg_test_parse_m_invalid = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_825_llm_16_824 {
    use super::*;
    use crate::*;
    use format::parse::ParsedItems;
    use format::error::Parse;
    #[test]
    fn test_parse_w() {
        let mut items = ParsedItems::new();
        let mut s: &mut &str = &mut "1";
        assert_eq!(parse_w(& mut items, & mut s), Ok(()));
    }
}
#[cfg(test)]
mod tests_llm_16_826 {
    use super::*;
    use crate::*;
    use crate::format::Padding;
    #[test]
    fn test_parse_y() {
        let _rug_st_tests_llm_16_826_rrrruuuugggg_test_parse_y = 0;
        let rug_fuzz_0 = "21";
        let mut items = ParsedItems::new();
        let mut s = rug_fuzz_0;
        let padding = Padding::None;
        debug_assert_eq!(parse_y(& mut items, & mut s, padding), Ok(()));
        debug_assert_eq!(items.year, Some(2000));
        let _rug_ed_tests_llm_16_826_rrrruuuugggg_test_parse_y = 0;
    }
}
#[cfg(test)]
mod tests_rug_36 {
    use super::*;
    use crate::{format::parse::ParsedItems, Weekday};
    #[test]
    fn test_parse_a() {
        let _rug_st_tests_rug_36_rrrruuuugggg_test_parse_a = 0;
        let rug_fuzz_0 = 2022;
        let rug_fuzz_1 = 11;
        let rug_fuzz_2 = 25;
        let rug_fuzz_3 = "Sat 01 Jan 2022";
        let mut p0 = ParsedItems::new();
        p0.week_based_year = Some(rug_fuzz_0);
        p0.month = Some(std::num::NonZeroU8::new(rug_fuzz_1).unwrap());
        p0.day = Some(std::num::NonZeroU8::new(rug_fuzz_2).unwrap());
        p0.weekday = Some(Weekday::Friday);
        let mut p1 = rug_fuzz_3;
        crate::format::date::parse_a(&mut p0, &mut p1);
        let _rug_ed_tests_rug_36_rrrruuuugggg_test_parse_a = 0;
    }
}
#[cfg(test)]
mod tests_rug_40 {
    use super::*;
    use crate::format::parse::ParsedItems;
    use std::num::NonZeroU8;
    #[test]
    fn test_parse_b() {
        let _rug_st_tests_rug_40_rrrruuuugggg_test_parse_b = 0;
        let rug_fuzz_0 = 11;
        let rug_fuzz_1 = "Nov";
        let mut items = ParsedItems::new();
        items.month = Some(NonZeroU8::new(rug_fuzz_0).unwrap());
        let mut s = rug_fuzz_1;
        parse_b(&mut items, &mut s);
        let _rug_ed_tests_rug_40_rrrruuuugggg_test_parse_b = 0;
    }
}
#[cfg(test)]
mod tests_rug_46 {
    use super::*;
    use crate::format::Padding;
    use std::num::NonZeroU8;
    use std::num::NonZeroU16;
    use crate::{format::parse::ParsedItems, Weekday};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_46_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 2022;
        let rug_fuzz_1 = 11;
        let rug_fuzz_2 = 25;
        let rug_fuzz_3 = "2022-11-25";
        let mut p0 = ParsedItems::new();
        p0.week_based_year = Some(rug_fuzz_0);
        p0.month = Some(NonZeroU8::new(rug_fuzz_1).unwrap());
        p0.day = Some(NonZeroU8::new(rug_fuzz_2).unwrap());
        p0.weekday = Some(Weekday::Friday);
        let mut p1 = rug_fuzz_3;
        let mut p2 = Padding::None;
        crate::format::date::parse_G(&mut p0, &mut p1, p2);
        let _rug_ed_tests_rug_46_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_48 {
    use super::*;
    use std::fmt::Formatter;
    use crate::format::date::fmt_m;
    use crate::format::Padding;
    use crate::Date;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_48_rrrruuuugggg_test_rug = 0;
        let mut p0: Formatter<'_> = unimplemented!();
        let p1: Date = unimplemented!();
        let mut p2: Padding = Padding::None;
        fmt_m(&mut p0, p1, p2);
        let _rug_ed_tests_rug_48_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_50 {
    use super::*;
    use crate::format::date::parse_u;
    use crate::format::parse::ParsedItems;
    use crate::Weekday;
    #[test]
    fn test_parse_u() {
        let _rug_st_tests_rug_50_rrrruuuugggg_test_parse_u = 0;
        let rug_fuzz_0 = "Monday";
        let mut p0 = ParsedItems::new();
        p0.weekday = Some(Weekday::Monday);
        let mut p1 = rug_fuzz_0;
        parse_u(&mut p0, &mut p1);
        let _rug_ed_tests_rug_50_rrrruuuugggg_test_parse_u = 0;
    }
}
