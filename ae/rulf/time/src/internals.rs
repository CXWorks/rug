//! This module and its contents are not subject to stability guarantees and
//! should not be relied upon.
//!
//! These methods either exist to reduce duplication in code elsewhere or are
//! public only for usage in macros. The reasoning for a method's existence is
//! generally documented alongside the method.
//!
//! Failure to ensure that parameters to the contained functions are in range
//! will likely result in invalid behavior.
#![doc(hidden)]
#![allow(missing_debug_implementations, missing_copy_implementations)]
use crate::{days_in_year, is_leap_year, Weekday};
use const_fn::const_fn;
pub struct Time;
impl Time {
    /// Create a `Time` from its components.
    pub const fn from_hms_nanos_unchecked(
        hour: u8,
        minute: u8,
        second: u8,
        nanosecond: u32,
    ) -> crate::Time {
        crate::Time {
            hour,
            minute,
            second,
            nanosecond,
        }
    }
}
pub struct Date;
impl Date {
    pub const fn from_yo_unchecked(year: i32, ordinal: u16) -> crate::Date {
        crate::Date {
            value: (year << 9) | ordinal as i32,
        }
    }
    pub(crate) const fn from_ymd_unchecked(
        year: i32,
        month: u8,
        day: u8,
    ) -> crate::Date {
        /// Cumulative days through the beginning of a month in both common and
        /// leap years.
        const DAYS_CUMULATIVE_COMMON_LEAP: [[u16; 12]; 2] = [
            [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334],
            [0, 31, 60, 91, 121, 152, 182, 213, 244, 274, 305, 335],
        ];
        Date::from_yo_unchecked(
            year,
            DAYS_CUMULATIVE_COMMON_LEAP[is_leap_year(year) as usize][month as usize - 1]
                + day as u16,
        )
    }
    #[const_fn("1.46")]
    pub(crate) const fn from_iso_ywd_unchecked(
        year: i32,
        week: u8,
        weekday: Weekday,
    ) -> crate::Date {
        let (ordinal, overflow) = (week as u16 * 7 + weekday.iso_weekday_number() as u16)
            .overflowing_sub(jan_weekday(year, 4) as u16 + 4);
        if overflow || ordinal == 0 {
            return Self::from_yo_unchecked(
                year - 1,
                ordinal.wrapping_add(days_in_year(year - 1)),
            );
        }
        let days_in_cur_year = days_in_year(year);
        if ordinal > days_in_cur_year {
            Self::from_yo_unchecked(year + 1, ordinal - days_in_cur_year)
        } else {
            Self::from_yo_unchecked(year, ordinal)
        }
    }
}
/// Obtain the ISO weekday number of a day in January.
#[const_fn("1.46")]
pub(crate) const fn jan_weekday(year: i32, ordinal: i32) -> u8 {
    let adj_year = year - 1;
    let rem = (ordinal + adj_year + adj_year / 4 - adj_year / 100 + adj_year / 400 + 6)
        % 7;
    if rem < 0 { (rem + 7) as u8 } else { rem as u8 }
}
#[cfg(test)]
mod tests_llm_16_914_llm_16_913 {
    use super::*;
    use crate::*;
    use crate::Weekday;
    #[test]
    fn test_from_iso_ywd_unchecked() {
        let _rug_st_tests_llm_16_914_llm_16_913_rrrruuuugggg_test_from_iso_ywd_unchecked = 0;
        let rug_fuzz_0 = 2019;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 2019;
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = 2020;
        let rug_fuzz_5 = 53;
        debug_assert_eq!(
            crate ::internals::Date::from_iso_ywd_unchecked(rug_fuzz_0, rug_fuzz_1,
            Weekday::Monday), crate ::Date::try_from_iso_ywd(2019, 1, Weekday::Monday)
            .unwrap()
        );
        debug_assert_eq!(
            crate ::internals::Date::from_iso_ywd_unchecked(rug_fuzz_2, rug_fuzz_3,
            Weekday::Tuesday), crate ::Date::try_from_iso_ywd(2019, 1, Weekday::Tuesday)
            .unwrap()
        );
        debug_assert_eq!(
            crate ::internals::Date::from_iso_ywd_unchecked(rug_fuzz_4, rug_fuzz_5,
            Weekday::Friday), crate ::Date::try_from_iso_ywd(2020, 53, Weekday::Friday)
            .unwrap()
        );
        let _rug_ed_tests_llm_16_914_llm_16_913_rrrruuuugggg_test_from_iso_ywd_unchecked = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_921 {
    use crate::internals::jan_weekday;
    #[test]
    fn test_jan_weekday() {
        let _rug_st_tests_llm_16_921_rrrruuuugggg_test_jan_weekday = 0;
        let rug_fuzz_0 = 2022;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 2023;
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = 2024;
        let rug_fuzz_5 = 1;
        let rug_fuzz_6 = 2025;
        let rug_fuzz_7 = 1;
        let rug_fuzz_8 = 2026;
        let rug_fuzz_9 = 1;
        debug_assert_eq!(jan_weekday(rug_fuzz_0, rug_fuzz_1), 6);
        debug_assert_eq!(jan_weekday(rug_fuzz_2, rug_fuzz_3), 0);
        debug_assert_eq!(jan_weekday(rug_fuzz_4, rug_fuzz_5), 1);
        debug_assert_eq!(jan_weekday(rug_fuzz_6, rug_fuzz_7), 2);
        debug_assert_eq!(jan_weekday(rug_fuzz_8, rug_fuzz_9), 3);
        let _rug_ed_tests_llm_16_921_rrrruuuugggg_test_jan_weekday = 0;
    }
}
#[cfg(test)]
mod tests_rug_76 {
    use super::*;
    use crate::internals;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_76_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 8;
        let rug_fuzz_1 = 30;
        let rug_fuzz_2 = 45;
        let rug_fuzz_3 = 500_000_000;
        let mut p0 = rug_fuzz_0;
        let mut p1 = rug_fuzz_1;
        let mut p2 = rug_fuzz_2;
        let mut p3 = rug_fuzz_3;
        internals::Time::from_hms_nanos_unchecked(p0, p1, p2, p3);
        let _rug_ed_tests_rug_76_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_77 {
    use super::*;
    use crate::internals::Date;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_77_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 2022;
        let rug_fuzz_1 = 365;
        let p0: i32 = rug_fuzz_0;
        let p1: u16 = rug_fuzz_1;
        Date::from_yo_unchecked(p0, p1);
        let _rug_ed_tests_rug_77_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_78 {
    use super::*;
    use crate::internals::Date;
    #[test]
    fn test_from_ymd_unchecked() {
        let _rug_st_tests_rug_78_rrrruuuugggg_test_from_ymd_unchecked = 0;
        let rug_fuzz_0 = 2021;
        let rug_fuzz_1 = 10;
        let rug_fuzz_2 = 25;
        let p0: i32 = rug_fuzz_0;
        let p1: u8 = rug_fuzz_1;
        let p2: u8 = rug_fuzz_2;
        Date::from_ymd_unchecked(p0, p1, p2);
        let _rug_ed_tests_rug_78_rrrruuuugggg_test_from_ymd_unchecked = 0;
    }
}
