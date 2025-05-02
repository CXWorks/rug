//! The UTC (Coordinated Universal Time) time zone.
use core::fmt;
#[cfg(
    all(
        feature = "clock",
        not(
            all(
                target_arch = "wasm32",
                feature = "wasmbind",
                not(any(target_os = "emscripten", target_os = "wasi"))
            )
        )
    )
)]
use std::time::{SystemTime, UNIX_EPOCH};
#[cfg(feature = "rkyv")]
use rkyv::{Archive, Deserialize, Serialize};
use super::{FixedOffset, LocalResult, Offset, TimeZone};
use crate::naive::{NaiveDate, NaiveDateTime};
#[cfg(feature = "clock")]
#[allow(deprecated)]
use crate::{Date, DateTime};
/// The UTC time zone. This is the most efficient time zone when you don't need the local time.
/// It is also used as an offset (which is also a dummy type).
///
/// Using the [`TimeZone`](./trait.TimeZone.html) methods
/// on the UTC struct is the preferred way to construct `DateTime<Utc>`
/// instances.
///
/// # Example
///
/// ```
/// use chrono::{DateTime, TimeZone, NaiveDateTime, Utc};
///
/// let dt = DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(61, 0), Utc);
///
/// assert_eq!(Utc.timestamp(61, 0), dt);
/// assert_eq!(Utc.with_ymd_and_hms(1970, 1, 1, 0, 1, 1).unwrap(), dt);
/// ```
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "rkyv", derive(Archive, Deserialize, Serialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct Utc;
#[cfg(feature = "clock")]
#[cfg_attr(docsrs, doc(cfg(feature = "clock")))]
impl Utc {
    /// Returns a `Date` which corresponds to the current date.
    #[deprecated(
        since = "0.4.23",
        note = "use `Utc::now()` instead, potentially with `.date_naive()`"
    )]
    #[allow(deprecated)]
    #[must_use]
    pub fn today() -> Date<Utc> {
        Utc::now().date()
    }
    /// Returns a `DateTime` which corresponds to the current date and time.
    #[cfg(
        not(
            all(
                target_arch = "wasm32",
                feature = "wasmbind",
                not(any(target_os = "emscripten", target_os = "wasi"))
            )
        )
    )]
    #[must_use]
    pub fn now() -> DateTime<Utc> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before Unix epoch");
        let naive = NaiveDateTime::from_timestamp_opt(
                now.as_secs() as i64,
                now.subsec_nanos(),
            )
            .unwrap();
        DateTime::from_utc(naive, Utc)
    }
    /// Returns a `DateTime` which corresponds to the current date and time.
    #[cfg(
        all(
            target_arch = "wasm32",
            feature = "wasmbind",
            not(any(target_os = "emscripten", target_os = "wasi"))
        )
    )]
    #[must_use]
    pub fn now() -> DateTime<Utc> {
        let now = js_sys::Date::new_0();
        DateTime::<Utc>::from(now)
    }
}
impl TimeZone for Utc {
    type Offset = Utc;
    fn from_offset(_state: &Utc) -> Utc {
        Utc
    }
    fn offset_from_local_date(&self, _local: &NaiveDate) -> LocalResult<Utc> {
        LocalResult::Single(Utc)
    }
    fn offset_from_local_datetime(&self, _local: &NaiveDateTime) -> LocalResult<Utc> {
        LocalResult::Single(Utc)
    }
    fn offset_from_utc_date(&self, _utc: &NaiveDate) -> Utc {
        Utc
    }
    fn offset_from_utc_datetime(&self, _utc: &NaiveDateTime) -> Utc {
        Utc
    }
}
impl Offset for Utc {
    fn fix(&self) -> FixedOffset {
        FixedOffset::east_opt(0).unwrap()
    }
}
impl fmt::Debug for Utc {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Z")
    }
}
impl fmt::Display for Utc {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "UTC")
    }
}
#[cfg(test)]
mod tests_rug_611 {
    use super::*;
    use crate::offset::utc::Utc;
    use crate::{NaiveDate, Date};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_611_rrrruuuugggg_test_rug = 0;
        let date: Date<Utc> = Utc::today();
        debug_assert_eq!(date, Utc::now().date());
        let _rug_ed_tests_rug_611_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_612 {
    use super::*;
    use crate::{DateTime, NaiveDateTime, TimeZone, Utc};
    use std::time::{SystemTime, UNIX_EPOCH};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_612_rrrruuuugggg_test_rug = 0;
        Utc::now();
        let _rug_ed_tests_rug_612_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_614 {
    use super::*;
    use crate::{Utc, TimeZone, NaiveDate, LocalResult};
    #[test]
    fn test_offset_from_local_date() {
        let _rug_st_tests_rug_614_rrrruuuugggg_test_offset_from_local_date = 0;
        let rug_fuzz_0 = 2021;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 1;
        let mut p0: Utc = Utc;
        let mut p1: NaiveDate = NaiveDate::from_ymd(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        p0.offset_from_local_date(&p1);
        let _rug_ed_tests_rug_614_rrrruuuugggg_test_offset_from_local_date = 0;
    }
}
#[cfg(test)]
mod tests_rug_615 {
    use super::*;
    use crate::TimeZone;
    use crate::{Utc, NaiveDateTime, LocalResult};
    #[test]
    fn test_offset_from_local_datetime() {
        let _rug_st_tests_rug_615_rrrruuuugggg_test_offset_from_local_datetime = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let p0: Utc = Utc;
        let p1: NaiveDateTime = NaiveDateTime::from_timestamp(rug_fuzz_0, rug_fuzz_1);
        debug_assert_eq!(p0.offset_from_local_datetime(& p1), LocalResult::Single(Utc));
        let _rug_ed_tests_rug_615_rrrruuuugggg_test_offset_from_local_datetime = 0;
    }
}
#[cfg(test)]
mod tests_rug_616 {
    use super::*;
    use crate::{Utc, TimeZone, NaiveDate};
    #[test]
    fn test_offset_from_utc_date() {
        let _rug_st_tests_rug_616_rrrruuuugggg_test_offset_from_utc_date = 0;
        let rug_fuzz_0 = 2021;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 1;
        let p0: Utc = Utc;
        let p1: NaiveDate = NaiveDate::from_ymd(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        Utc::offset_from_utc_date(&p0, &p1);
        let _rug_ed_tests_rug_616_rrrruuuugggg_test_offset_from_utc_date = 0;
    }
}
#[cfg(test)]
mod tests_rug_617 {
    use super::*;
    use crate::TimeZone;
    use crate::{Utc, NaiveDateTime};
    #[test]
    fn test_offset_from_utc_datetime() {
        let _rug_st_tests_rug_617_rrrruuuugggg_test_offset_from_utc_datetime = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let mut p0: Utc = Utc;
        let mut p1: NaiveDateTime = NaiveDateTime::from_timestamp(
            rug_fuzz_0,
            rug_fuzz_1,
        );
        p0.offset_from_utc_datetime(&p1);
        let _rug_ed_tests_rug_617_rrrruuuugggg_test_offset_from_utc_datetime = 0;
    }
}
#[cfg(test)]
mod tests_rug_618 {
    use super::*;
    use crate::Offset;
    use crate::FixedOffset;
    use crate::offset::utc::Utc;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_618_rrrruuuugggg_test_rug = 0;
        let mut p0: Utc = Utc;
        p0.fix();
        let _rug_ed_tests_rug_618_rrrruuuugggg_test_rug = 0;
    }
}
