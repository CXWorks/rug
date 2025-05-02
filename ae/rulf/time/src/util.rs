//! Utility functions.
use crate::{format::try_parse_fmt_string, internals};
#[cfg(not(feature = "std"))]
use alloc::string::String;
use const_fn::const_fn;
/// Checks if a user-provided formatting string is valid. If it isn't, a
/// description of the error is returned.
pub fn validate_format_string(s: impl AsRef<str>) -> Result<(), String> {
    try_parse_fmt_string(s.as_ref()).map(|_| ())
}
/// The number of days in a month in both common and leap years.
const DAYS_IN_MONTH_COMMON_LEAP: [[u16; 12]; 2] = [
    [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31],
    [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31],
];
/// Get the number of days in the month of a given year.
pub(crate) const fn days_in_year_month(year: i32, month: u8) -> u8 {
    DAYS_IN_MONTH_COMMON_LEAP[is_leap_year(year) as usize][month as usize - 1] as u8
}
/// Returns if the provided year is a leap year in the proleptic Gregorian
/// calendar. Uses [astronomical year numbering](https://en.wikipedia.org/wiki/Astronomical_year_numbering).
///
/// ```rust
/// # use time::is_leap_year;
/// assert!(!is_leap_year(1900));
/// assert!(is_leap_year(2000));
/// assert!(is_leap_year(2004));
/// assert!(!is_leap_year(2005));
/// assert!(!is_leap_year(2100));
/// ```
pub const fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0) & ((year % 100 != 0) | (year % 400 == 0))
}
/// Get the number of calendar days in a given year.
///
/// The returned value will always be either 365 or 366.
///
/// ```rust
/// # use time::days_in_year;
/// assert_eq!(days_in_year(1900), 365);
/// assert_eq!(days_in_year(2000), 366);
/// assert_eq!(days_in_year(2004), 366);
/// assert_eq!(days_in_year(2005), 365);
/// assert_eq!(days_in_year(2100), 365);
/// ```
pub const fn days_in_year(year: i32) -> u16 {
    365 + is_leap_year(year) as u16
}
/// Get the number of weeks in the ISO year.
///
/// The returned value will always be either 52 or 53.
///
/// ```rust
/// # use time::weeks_in_year;
/// assert_eq!(weeks_in_year(2019), 52);
/// assert_eq!(weeks_in_year(2020), 53);
/// ```
///
/// This function is `const fn` when using rustc >= 1.46.
#[const_fn("1.46")]
pub const fn weeks_in_year(year: i32) -> u8 {
    let weekday = internals::Date::from_yo_unchecked(year, 1).iso_weekday_number();
    if weekday == 4 || weekday == 3 && is_leap_year(year) { 53 } else { 52 }
}
#[cfg(test)]
mod tests_llm_16_1040 {
    use super::*;
    use crate::*;
    use crate::util::is_leap_year;
    #[test]
    fn test_days_in_year() {
        let _rug_st_tests_llm_16_1040_rrrruuuugggg_test_days_in_year = 0;
        let rug_fuzz_0 = 1900;
        let rug_fuzz_1 = 2000;
        let rug_fuzz_2 = 2004;
        let rug_fuzz_3 = 2005;
        let rug_fuzz_4 = 2100;
        debug_assert_eq!(days_in_year(rug_fuzz_0), 365);
        debug_assert_eq!(days_in_year(rug_fuzz_1), 366);
        debug_assert_eq!(days_in_year(rug_fuzz_2), 366);
        debug_assert_eq!(days_in_year(rug_fuzz_3), 365);
        debug_assert_eq!(days_in_year(rug_fuzz_4), 365);
        let _rug_ed_tests_llm_16_1040_rrrruuuugggg_test_days_in_year = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_1041 {
    use crate::util::{days_in_year_month, is_leap_year};
    #[test]
    fn test_days_in_year_month() {
        let _rug_st_tests_llm_16_1041_rrrruuuugggg_test_days_in_year_month = 0;
        let rug_fuzz_0 = 2021;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 2021;
        let rug_fuzz_3 = 2;
        let rug_fuzz_4 = 2021;
        let rug_fuzz_5 = 3;
        let rug_fuzz_6 = 2021;
        let rug_fuzz_7 = 4;
        let rug_fuzz_8 = 2021;
        let rug_fuzz_9 = 5;
        let rug_fuzz_10 = 2021;
        let rug_fuzz_11 = 6;
        let rug_fuzz_12 = 2021;
        let rug_fuzz_13 = 7;
        let rug_fuzz_14 = 2021;
        let rug_fuzz_15 = 8;
        let rug_fuzz_16 = 2021;
        let rug_fuzz_17 = 9;
        let rug_fuzz_18 = 2021;
        let rug_fuzz_19 = 10;
        let rug_fuzz_20 = 2021;
        let rug_fuzz_21 = 11;
        let rug_fuzz_22 = 2021;
        let rug_fuzz_23 = 12;
        let rug_fuzz_24 = 2020;
        let rug_fuzz_25 = 2;
        let rug_fuzz_26 = 2000;
        let rug_fuzz_27 = 2;
        let rug_fuzz_28 = 1900;
        let rug_fuzz_29 = 2;
        debug_assert_eq!(days_in_year_month(rug_fuzz_0, rug_fuzz_1), 31);
        debug_assert_eq!(days_in_year_month(rug_fuzz_2, rug_fuzz_3), 28);
        debug_assert_eq!(days_in_year_month(rug_fuzz_4, rug_fuzz_5), 31);
        debug_assert_eq!(days_in_year_month(rug_fuzz_6, rug_fuzz_7), 30);
        debug_assert_eq!(days_in_year_month(rug_fuzz_8, rug_fuzz_9), 31);
        debug_assert_eq!(days_in_year_month(rug_fuzz_10, rug_fuzz_11), 30);
        debug_assert_eq!(days_in_year_month(rug_fuzz_12, rug_fuzz_13), 31);
        debug_assert_eq!(days_in_year_month(rug_fuzz_14, rug_fuzz_15), 31);
        debug_assert_eq!(days_in_year_month(rug_fuzz_16, rug_fuzz_17), 30);
        debug_assert_eq!(days_in_year_month(rug_fuzz_18, rug_fuzz_19), 31);
        debug_assert_eq!(days_in_year_month(rug_fuzz_20, rug_fuzz_21), 30);
        debug_assert_eq!(days_in_year_month(rug_fuzz_22, rug_fuzz_23), 31);
        debug_assert_eq!(days_in_year_month(rug_fuzz_24, rug_fuzz_25), 29);
        debug_assert_eq!(days_in_year_month(rug_fuzz_26, rug_fuzz_27), 29);
        debug_assert_eq!(days_in_year_month(rug_fuzz_28, rug_fuzz_29), 28);
        let _rug_ed_tests_llm_16_1041_rrrruuuugggg_test_days_in_year_month = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_1042 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_leap_year() {
        let _rug_st_tests_llm_16_1042_rrrruuuugggg_test_is_leap_year = 0;
        let rug_fuzz_0 = 1900;
        let rug_fuzz_1 = 2000;
        let rug_fuzz_2 = 2004;
        let rug_fuzz_3 = 2005;
        let rug_fuzz_4 = 2100;
        debug_assert_eq!(is_leap_year(rug_fuzz_0), false);
        debug_assert_eq!(is_leap_year(rug_fuzz_1), true);
        debug_assert_eq!(is_leap_year(rug_fuzz_2), true);
        debug_assert_eq!(is_leap_year(rug_fuzz_3), false);
        debug_assert_eq!(is_leap_year(rug_fuzz_4), false);
        let _rug_ed_tests_llm_16_1042_rrrruuuugggg_test_is_leap_year = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_1043 {
    use super::*;
    use crate::*;
    #[test]
    fn test_validate_format_string_valid() {
        let _rug_st_tests_llm_16_1043_rrrruuuugggg_test_validate_format_string_valid = 0;
        let rug_fuzz_0 = "{}";
        let rug_fuzz_1 = "abc";
        let rug_fuzz_2 = "Hello, {}!";
        debug_assert_eq!(validate_format_string(rug_fuzz_0), Ok(()));
        debug_assert_eq!(validate_format_string(rug_fuzz_1), Ok(()));
        debug_assert_eq!(validate_format_string(rug_fuzz_2), Ok(()));
        let _rug_ed_tests_llm_16_1043_rrrruuuugggg_test_validate_format_string_valid = 0;
    }
    #[test]
    fn test_validate_format_string_invalid() {
        let _rug_st_tests_llm_16_1043_rrrruuuugggg_test_validate_format_string_invalid = 0;
        let rug_fuzz_0 = "{";
        let rug_fuzz_1 = "}";
        let rug_fuzz_2 = "{abc";
        debug_assert_eq!(
            validate_format_string(rug_fuzz_0), Err("mismatched braces".to_string())
        );
        debug_assert_eq!(
            validate_format_string(rug_fuzz_1), Err("mismatched braces".to_string())
        );
        debug_assert_eq!(
            validate_format_string(rug_fuzz_2), Err("mismatched braces".to_string())
        );
        let _rug_ed_tests_llm_16_1043_rrrruuuugggg_test_validate_format_string_invalid = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_1044 {
    use super::*;
    use crate::*;
    use crate::internals;
    #[test]
    fn test_weeks_in_year() {
        let _rug_st_tests_llm_16_1044_rrrruuuugggg_test_weeks_in_year = 0;
        let rug_fuzz_0 = 2019;
        let rug_fuzz_1 = 2020;
        debug_assert_eq!(weeks_in_year(rug_fuzz_0), 52);
        debug_assert_eq!(weeks_in_year(rug_fuzz_1), 53);
        let _rug_ed_tests_llm_16_1044_rrrruuuugggg_test_weeks_in_year = 0;
    }
}
