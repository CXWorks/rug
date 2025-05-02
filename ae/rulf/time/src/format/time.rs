//! Formatting helpers for a `Time`.
#![allow(non_snake_case)]
use crate::{
    error,
    format::{
        parse::{try_consume_exact_digits, try_consume_first_match, AmPm::{AM, PM}},
        Padding, ParseResult, ParsedItems,
    },
    Time,
};
use core::{
    fmt::{self, Formatter},
    num::NonZeroU8,
};
#[allow(unused_imports)]
use standback::prelude::*;
/// Hour in 24h format (`00`-`23`)
pub(crate) fn fmt_H(f: &mut Formatter<'_>, time: Time, padding: Padding) -> fmt::Result {
    pad!(f, padding, 2, time.hour())
}
/// Hour in 24h format (`00`-`23`)
pub(crate) fn parse_H(
    items: &mut ParsedItems,
    s: &mut &str,
    padding: Padding,
) -> ParseResult<()> {
    items
        .hour_24 = Some(
        try_consume_exact_digits(s, 2, padding).ok_or(error::Parse::InvalidHour)?,
    );
    Ok(())
}
/// Hour in 12h format (`01`-`12`)
pub(crate) fn fmt_I(f: &mut Formatter<'_>, time: Time, padding: Padding) -> fmt::Result {
    pad!(f, padding, 2, (time.hour() as i8 - 1).rem_euclid(12) + 1)
}
/// Hour in 12h format (`01`-`12`)
pub(crate) fn parse_I(
    items: &mut ParsedItems,
    s: &mut &str,
    padding: Padding,
) -> ParseResult<()> {
    items
        .hour_12 = Some(
        try_consume_exact_digits(s, 2, padding)
            .and_then(NonZeroU8::new)
            .ok_or(error::Parse::InvalidHour)?,
    );
    Ok(())
}
/// Minutes, zero-padded (`00`-`59`)
pub(crate) fn fmt_M(f: &mut Formatter<'_>, time: Time, padding: Padding) -> fmt::Result {
    pad!(f, padding, 2, time.minute())
}
/// Minutes, zero-added (`00`-`59`)
pub(crate) fn parse_M(
    items: &mut ParsedItems,
    s: &mut &str,
    padding: Padding,
) -> ParseResult<()> {
    items
        .minute = Some(
        try_consume_exact_digits(s, 2, padding).ok_or(error::Parse::InvalidMinute)?,
    );
    Ok(())
}
/// Subsecond nanoseconds. Always 9 digits
pub(crate) fn fmt_N(f: &mut Formatter<'_>, time: Time) -> fmt::Result {
    write!(f, "{:09}", time.nanosecond)
}
/// Subsecond nanoseconds. Always 9 digits
pub(crate) fn parse_N(items: &mut ParsedItems, s: &mut &str) -> ParseResult<()> {
    items
        .nanosecond = Some(
        try_consume_exact_digits(s, 9, Padding::None)
            .ok_or(error::Parse::InvalidNanosecond)?,
    );
    Ok(())
}
/// am/pm
pub(crate) fn fmt_p(f: &mut Formatter<'_>, time: Time) -> fmt::Result {
    if time.hour() < 12 { f.write_str("am") } else { f.write_str("pm") }
}
/// am/pm
pub(crate) fn parse_p(items: &mut ParsedItems, s: &mut &str) -> ParseResult<()> {
    items
        .am_pm = Some(
        try_consume_first_match(s, [("am", AM), ("pm", PM)].iter().cloned())
            .ok_or(error::Parse::InvalidAmPm)?,
    );
    Ok(())
}
/// AM/PM
pub(crate) fn fmt_P(f: &mut Formatter<'_>, time: Time) -> fmt::Result {
    if time.hour() < 12 { f.write_str("AM") } else { f.write_str("PM") }
}
/// AM/PM
pub(crate) fn parse_P(items: &mut ParsedItems, s: &mut &str) -> ParseResult<()> {
    items
        .am_pm = Some(
        try_consume_first_match(s, [("AM", AM), ("PM", PM)].iter().cloned())
            .ok_or(error::Parse::InvalidAmPm)?,
    );
    Ok(())
}
/// Seconds, zero-padded (`00`-`59`)
pub(crate) fn fmt_S(f: &mut Formatter<'_>, time: Time, padding: Padding) -> fmt::Result {
    pad!(f, padding, 2, time.second())
}
/// Seconds, zero-added (`00`-`59`)
pub(crate) fn parse_S(
    items: &mut ParsedItems,
    s: &mut &str,
    padding: Padding,
) -> ParseResult<()> {
    items
        .second = Some(
        try_consume_exact_digits(s, 2, padding).ok_or(error::Parse::InvalidSecond)?,
    );
    Ok(())
}
#[cfg(test)]
mod tests_llm_16_876 {
    use super::*;
    use crate::*;
    use format::parse::ParsedItems;
    use format::Padding;
    use std::num::NonZeroU8;
    use std::num::NonZeroU16;
    use crate::Weekday;
    #[test]
    fn test_parse_H() {
        let _rug_st_tests_llm_16_876_rrrruuuugggg_test_parse_H = 0;
        let rug_fuzz_0 = "23";
        let padding = Padding::None;
        let mut items = ParsedItems::new();
        let mut s = rug_fuzz_0;
        let result = parse_H(&mut items, &mut s, padding);
        debug_assert_eq!(result, Ok(()));
        debug_assert_eq!(items.hour_24, Some(23));
        let _rug_ed_tests_llm_16_876_rrrruuuugggg_test_parse_H = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_881 {
    use super::*;
    use crate::*;
    use crate::error::Parse;
    #[test]
    fn test_parse_N() {
        let _rug_st_tests_llm_16_881_rrrruuuugggg_test_parse_N = 0;
        let rug_fuzz_0 = "123456789";
        let rug_fuzz_1 = "12345678";
        let rug_fuzz_2 = "1234567890";
        let mut items = ParsedItems::new();
        let mut s = rug_fuzz_0;
        debug_assert_eq!(parse_N(& mut items, & mut s), Ok(()));
        debug_assert_eq!(items.nanosecond, Some(123456789));
        let mut items = ParsedItems::new();
        let mut s = rug_fuzz_1;
        debug_assert_eq!(parse_N(& mut items, & mut s), Err(Parse::InvalidNanosecond));
        debug_assert_eq!(items.nanosecond, None);
        let mut items = ParsedItems::new();
        let mut s = rug_fuzz_2;
        debug_assert_eq!(parse_N(& mut items, & mut s), Err(Parse::InvalidNanosecond));
        debug_assert_eq!(items.nanosecond, None);
        let _rug_ed_tests_llm_16_881_rrrruuuugggg_test_parse_N = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_882 {
    use super::*;
    use crate::*;
    use crate::error::Parse;
    #[test]
    fn test_parse_P() {
        let _rug_st_tests_llm_16_882_rrrruuuugggg_test_parse_P = 0;
        let rug_fuzz_0 = "AM";
        let rug_fuzz_1 = "PM";
        let rug_fuzz_2 = "A";
        let rug_fuzz_3 = "AM";
        let rug_fuzz_4 = 1;
        let rug_fuzz_5 = "";
        let mut items = ParsedItems::new();
        let mut s = rug_fuzz_0;
        debug_assert_eq!(parse_P(& mut items, & mut s), Ok(()));
        debug_assert_eq!(items.am_pm, Some(AM));
        debug_assert_eq!(s, "");
        let mut items = ParsedItems::new();
        let mut s = rug_fuzz_1;
        debug_assert_eq!(parse_P(& mut items, & mut s), Ok(()));
        debug_assert_eq!(items.am_pm, Some(PM));
        debug_assert_eq!(s, "");
        let mut items = ParsedItems::new();
        let mut s = rug_fuzz_2;
        debug_assert_eq!(parse_P(& mut items, & mut s), Err(Parse::InvalidAmPm));
        debug_assert_eq!(items.am_pm, None);
        debug_assert_eq!(s, "A");
        let mut items = ParsedItems::new();
        let mut s = rug_fuzz_3;
        s = &s[..rug_fuzz_4];
        debug_assert_eq!(parse_P(& mut items, & mut s), Err(Parse::InvalidAmPm));
        debug_assert_eq!(items.am_pm, None);
        debug_assert_eq!(s, "M");
        let mut items = ParsedItems::new();
        let mut s = rug_fuzz_5;
        debug_assert_eq!(parse_P(& mut items, & mut s), Err(Parse::InvalidAmPm));
        debug_assert_eq!(items.am_pm, None);
        debug_assert_eq!(s, "");
        let _rug_ed_tests_llm_16_882_rrrruuuugggg_test_parse_P = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_885 {
    use super::*;
    use crate::*;
    use crate::error::Parse;
    #[test]
    fn test_parse_p_valid_am() {
        let _rug_st_tests_llm_16_885_rrrruuuugggg_test_parse_p_valid_am = 0;
        let rug_fuzz_0 = "am";
        let mut items = ParsedItems::new();
        let mut s = rug_fuzz_0;
        let result = parse_p(&mut items, &mut s);
        debug_assert_eq!(result, Ok(()));
        debug_assert_eq!(items.am_pm, Some(AM));
        let _rug_ed_tests_llm_16_885_rrrruuuugggg_test_parse_p_valid_am = 0;
    }
    #[test]
    fn test_parse_p_valid_pm() {
        let _rug_st_tests_llm_16_885_rrrruuuugggg_test_parse_p_valid_pm = 0;
        let rug_fuzz_0 = "pm";
        let mut items = ParsedItems::new();
        let mut s = rug_fuzz_0;
        let result = parse_p(&mut items, &mut s);
        debug_assert_eq!(result, Ok(()));
        debug_assert_eq!(items.am_pm, Some(PM));
        let _rug_ed_tests_llm_16_885_rrrruuuugggg_test_parse_p_valid_pm = 0;
    }
    #[test]
    fn test_parse_p_invalid() {
        let _rug_st_tests_llm_16_885_rrrruuuugggg_test_parse_p_invalid = 0;
        let rug_fuzz_0 = "invalid";
        let mut items = ParsedItems::new();
        let mut s = rug_fuzz_0;
        let result = parse_p(&mut items, &mut s);
        debug_assert_eq!(result, Err(Parse::InvalidAmPm));
        debug_assert_eq!(items.am_pm, None);
        let _rug_ed_tests_llm_16_885_rrrruuuugggg_test_parse_p_invalid = 0;
    }
}
#[cfg(test)]
mod tests_rug_65 {
    use super::*;
    use crate::{
        format::parse::ParsedItems, format::Padding, Weekday, error::Parse,
        error::Parse::*,
    };
    use std::num::NonZeroU8;
    #[test]
    fn test_parse_I() {
        let _rug_st_tests_rug_65_rrrruuuugggg_test_parse_I = 0;
        let rug_fuzz_0 = 2022;
        let rug_fuzz_1 = 11;
        let rug_fuzz_2 = 25;
        let rug_fuzz_3 = "01:23:45";
        let mut items = ParsedItems::new();
        items.week_based_year = Some(rug_fuzz_0);
        items.month = Some(NonZeroU8::new(rug_fuzz_1).unwrap());
        items.day = Some(NonZeroU8::new(rug_fuzz_2).unwrap());
        items.weekday = Some(Weekday::Friday);
        let mut s = rug_fuzz_3;
        let padding = Padding::None;
        debug_assert_eq!(parse_I(& mut items, & mut s, padding), Ok(()));
        let _rug_ed_tests_rug_65_rrrruuuugggg_test_parse_I = 0;
    }
}
#[cfg(test)]
mod tests_rug_67 {
    use super::*;
    use crate::{format::parse::ParsedItems, Weekday};
    #[test]
    fn test_parse_M() {
        let _rug_st_tests_rug_67_rrrruuuugggg_test_parse_M = 0;
        let rug_fuzz_0 = 2022;
        let rug_fuzz_1 = 11;
        let rug_fuzz_2 = 25;
        let rug_fuzz_3 = "30";
        let mut items = ParsedItems::new();
        items.week_based_year = Some(rug_fuzz_0);
        items.month = Some(std::num::NonZeroU8::new(rug_fuzz_1).unwrap());
        items.day = Some(std::num::NonZeroU8::new(rug_fuzz_2).unwrap());
        items.weekday = Some(Weekday::Friday);
        let mut s: &str = rug_fuzz_3;
        let padding = Padding::Zero;
        parse_M(&mut items, &mut s, padding);
        let _rug_ed_tests_rug_67_rrrruuuugggg_test_parse_M = 0;
    }
}
#[cfg(test)]
mod tests_rug_72 {
    use super::*;
    use std::num::NonZeroU8;
    use crate::{
        format::{parse::ParsedItems, Padding},
        Weekday,
    };
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_72_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 2022;
        let rug_fuzz_1 = 11;
        let rug_fuzz_2 = 25;
        let rug_fuzz_3 = "sample string data";
        let mut p0 = ParsedItems::new();
        p0.week_based_year = Some(rug_fuzz_0);
        p0.month = Some(NonZeroU8::new(rug_fuzz_1).unwrap());
        p0.day = Some(NonZeroU8::new(rug_fuzz_2).unwrap());
        p0.weekday = Some(Weekday::Friday);
        let mut p1 = rug_fuzz_3;
        let mut p2 = Padding::None;
        crate::format::time::parse_S(&mut p0, &mut p1, p2);
        let _rug_ed_tests_rug_72_rrrruuuugggg_test_rug = 0;
    }
}
