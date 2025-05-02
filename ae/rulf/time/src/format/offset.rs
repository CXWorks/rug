//! Formatting helpers for a `UtcOffset`.
#![allow(non_snake_case)]
use crate::{
    error,
    format::{
        parse::{try_consume_exact_digits, try_consume_first_match},
        Padding, ParsedItems,
    },
    ParseResult, UtcOffset,
};
use core::fmt::{self, Formatter};
/// UTC offset
pub(crate) fn fmt_z(f: &mut Formatter<'_>, offset: UtcOffset) -> fmt::Result {
    let offset = offset.as_duration();
    write!(
        f, "{}{:02}{:02}", if offset.is_negative() { '-' } else { '+' }, offset
        .whole_hours().abs(), (offset.whole_minutes() - 60 * offset.whole_hours()).abs()
    )
}
/// UTC offset
pub(crate) fn parse_z(items: &mut ParsedItems, s: &mut &str) -> ParseResult<()> {
    let sign = try_consume_first_match(s, [("+", 1), ("-", -1)].iter().cloned())
        .ok_or(error::Parse::InvalidOffset)?;
    let hours: i16 = try_consume_exact_digits(s, 2, Padding::Zero)
        .ok_or(error::Parse::InvalidOffset)?;
    let minutes: i16 = try_consume_exact_digits(s, 2, Padding::Zero)
        .ok_or(error::Parse::InvalidOffset)?;
    items.offset = UtcOffset::minutes(sign * (hours * 60 + minutes)).into();
    Ok(())
}
#[cfg(test)]
mod tests_rug_58 {
    use super::*;
    use std::num::NonZeroU8;
    use std::num::NonZeroU16;
    use crate::{format::offset::ParsedItems, Weekday};
    #[test]
    fn test_parse_z() {
        let _rug_st_tests_rug_58_rrrruuuugggg_test_parse_z = 0;
        let rug_fuzz_0 = 2022;
        let rug_fuzz_1 = 11;
        let rug_fuzz_2 = 25;
        let rug_fuzz_3 = "...";
        let mut items = ParsedItems::new();
        items.week_based_year = Some(rug_fuzz_0);
        items.month = Some(NonZeroU8::new(rug_fuzz_1).unwrap());
        items.day = Some(NonZeroU8::new(rug_fuzz_2).unwrap());
        items.weekday = Some(Weekday::Friday);
        let mut s = rug_fuzz_3;
        parse_z(&mut items, &mut s);
        let _rug_ed_tests_rug_58_rrrruuuugggg_test_parse_z = 0;
    }
}
