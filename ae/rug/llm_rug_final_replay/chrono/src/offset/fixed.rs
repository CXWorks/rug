//! The time zone which has a fixed offset from UTC.
use core::fmt;
use core::ops::{Add, Sub};
#[cfg(feature = "rkyv")]
use rkyv::{Archive, Deserialize, Serialize};
use super::{LocalResult, Offset, TimeZone};
use crate::naive::{NaiveDate, NaiveDateTime, NaiveTime};
use crate::time_delta::TimeDelta;
use crate::DateTime;
use crate::Timelike;
/// The time zone with fixed offset, from UTC-23:59:59 to UTC+23:59:59.
///
/// Using the [`TimeZone`](./trait.TimeZone.html) methods
/// on a `FixedOffset` struct is the preferred way to construct
/// `DateTime<FixedOffset>` instances. See the [`east_opt`](#method.east_opt) and
/// [`west_opt`](#method.west_opt) methods for examples.
#[derive(PartialEq, Eq, Hash, Copy, Clone)]
#[cfg_attr(feature = "rkyv", derive(Archive, Deserialize, Serialize))]
pub struct FixedOffset {
    local_minus_utc: i32,
}
impl FixedOffset {
    /// Makes a new `FixedOffset` for the Eastern Hemisphere with given timezone difference.
    /// The negative `secs` means the Western Hemisphere.
    ///
    /// Panics on the out-of-bound `secs`.
    #[deprecated(since = "0.4.23", note = "use `east_opt()` instead")]
    #[must_use]
    pub fn east(secs: i32) -> FixedOffset {
        FixedOffset::east_opt(secs).expect("FixedOffset::east out of bounds")
    }
    /// Makes a new `FixedOffset` for the Eastern Hemisphere with given timezone difference.
    /// The negative `secs` means the Western Hemisphere.
    ///
    /// Returns `None` on the out-of-bound `secs`.
    ///
    /// # Example
    ///
    #[cfg_attr(not(feature = "std"), doc = "```ignore")]
    #[cfg_attr(feature = "std", doc = "```")]
    /// use chrono::{FixedOffset, TimeZone};
    /// let hour = 3600;
    /// let datetime = FixedOffset::east_opt(5 * hour).unwrap().ymd_opt(2016, 11, 08).unwrap()
    ///                                           .and_hms_opt(0, 0, 0).unwrap();
    /// assert_eq!(&datetime.to_rfc3339(), "2016-11-08T00:00:00+05:00")
    /// ```
    #[must_use]
    pub const fn east_opt(secs: i32) -> Option<FixedOffset> {
        if -86_400 < secs && secs < 86_400 {
            Some(FixedOffset {
                local_minus_utc: secs,
            })
        } else {
            None
        }
    }
    /// Makes a new `FixedOffset` for the Western Hemisphere with given timezone difference.
    /// The negative `secs` means the Eastern Hemisphere.
    ///
    /// Panics on the out-of-bound `secs`.
    #[deprecated(since = "0.4.23", note = "use `west_opt()` instead")]
    #[must_use]
    pub fn west(secs: i32) -> FixedOffset {
        FixedOffset::west_opt(secs).expect("FixedOffset::west out of bounds")
    }
    /// Makes a new `FixedOffset` for the Western Hemisphere with given timezone difference.
    /// The negative `secs` means the Eastern Hemisphere.
    ///
    /// Returns `None` on the out-of-bound `secs`.
    ///
    /// # Example
    ///
    #[cfg_attr(not(feature = "std"), doc = "```ignore")]
    #[cfg_attr(feature = "std", doc = "```")]
    /// use chrono::{FixedOffset, TimeZone};
    /// let hour = 3600;
    /// let datetime = FixedOffset::west_opt(5 * hour).unwrap().ymd_opt(2016, 11, 08).unwrap()
    ///                                           .and_hms_opt(0, 0, 0).unwrap();
    /// assert_eq!(&datetime.to_rfc3339(), "2016-11-08T00:00:00-05:00")
    /// ```
    #[must_use]
    pub const fn west_opt(secs: i32) -> Option<FixedOffset> {
        if -86_400 < secs && secs < 86_400 {
            Some(FixedOffset {
                local_minus_utc: -secs,
            })
        } else {
            None
        }
    }
    /// Returns the number of seconds to add to convert from UTC to the local time.
    #[inline]
    pub const fn local_minus_utc(&self) -> i32 {
        self.local_minus_utc
    }
    /// Returns the number of seconds to add to convert from the local time to UTC.
    #[inline]
    pub const fn utc_minus_local(&self) -> i32 {
        -self.local_minus_utc
    }
}
impl TimeZone for FixedOffset {
    type Offset = FixedOffset;
    fn from_offset(offset: &FixedOffset) -> FixedOffset {
        *offset
    }
    fn offset_from_local_date(&self, _local: &NaiveDate) -> LocalResult<FixedOffset> {
        LocalResult::Single(*self)
    }
    fn offset_from_local_datetime(
        &self,
        _local: &NaiveDateTime,
    ) -> LocalResult<FixedOffset> {
        LocalResult::Single(*self)
    }
    fn offset_from_utc_date(&self, _utc: &NaiveDate) -> FixedOffset {
        *self
    }
    fn offset_from_utc_datetime(&self, _utc: &NaiveDateTime) -> FixedOffset {
        *self
    }
}
impl Offset for FixedOffset {
    fn fix(&self) -> FixedOffset {
        *self
    }
}
impl fmt::Debug for FixedOffset {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let offset = self.local_minus_utc;
        let (sign, offset) = if offset < 0 { ('-', -offset) } else { ('+', offset) };
        let sec = offset.rem_euclid(60);
        let mins = offset.div_euclid(60);
        let min = mins.rem_euclid(60);
        let hour = mins.div_euclid(60);
        if sec == 0 {
            write!(f, "{}{:02}:{:02}", sign, hour, min)
        } else {
            write!(f, "{}{:02}:{:02}:{:02}", sign, hour, min, sec)
        }
    }
}
impl fmt::Display for FixedOffset {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}
#[cfg(feature = "arbitrary")]
impl arbitrary::Arbitrary<'_> for FixedOffset {
    fn arbitrary(u: &mut arbitrary::Unstructured) -> arbitrary::Result<FixedOffset> {
        let secs = u.int_in_range(-86_399..=86_399)?;
        let fixed_offset = FixedOffset::east_opt(secs)
            .expect(
                "Could not generate a valid chrono::FixedOffset. It looks like implementation of Arbitrary for FixedOffset is erroneous.",
            );
        Ok(fixed_offset)
    }
}
fn add_with_leapsecond<T>(lhs: &T, rhs: i32) -> T
where
    T: Timelike + Add<TimeDelta, Output = T>,
{
    let nanos = lhs.nanosecond();
    let lhs = lhs.with_nanosecond(0).unwrap();
    (lhs + TimeDelta::seconds(i64::from(rhs))).with_nanosecond(nanos).unwrap()
}
impl Add<FixedOffset> for NaiveTime {
    type Output = NaiveTime;
    #[inline]
    fn add(self, rhs: FixedOffset) -> NaiveTime {
        add_with_leapsecond(&self, rhs.local_minus_utc)
    }
}
impl Sub<FixedOffset> for NaiveTime {
    type Output = NaiveTime;
    #[inline]
    fn sub(self, rhs: FixedOffset) -> NaiveTime {
        add_with_leapsecond(&self, -rhs.local_minus_utc)
    }
}
impl Add<FixedOffset> for NaiveDateTime {
    type Output = NaiveDateTime;
    #[inline]
    fn add(self, rhs: FixedOffset) -> NaiveDateTime {
        add_with_leapsecond(&self, rhs.local_minus_utc)
    }
}
impl Sub<FixedOffset> for NaiveDateTime {
    type Output = NaiveDateTime;
    #[inline]
    fn sub(self, rhs: FixedOffset) -> NaiveDateTime {
        add_with_leapsecond(&self, -rhs.local_minus_utc)
    }
}
impl<Tz: TimeZone> Add<FixedOffset> for DateTime<Tz> {
    type Output = DateTime<Tz>;
    #[inline]
    fn add(self, rhs: FixedOffset) -> DateTime<Tz> {
        add_with_leapsecond(&self, rhs.local_minus_utc)
    }
}
impl<Tz: TimeZone> Sub<FixedOffset> for DateTime<Tz> {
    type Output = DateTime<Tz>;
    #[inline]
    fn sub(self, rhs: FixedOffset) -> DateTime<Tz> {
        add_with_leapsecond(&self, -rhs.local_minus_utc)
    }
}
#[cfg(test)]
mod tests {
    use super::FixedOffset;
    use crate::offset::TimeZone;
    #[test]
    fn test_date_extreme_offset() {
        assert_eq!(
            format!("{:?}", FixedOffset::east_opt(86399).unwrap().with_ymd_and_hms(2012,
            2, 29, 5, 6, 7).unwrap()), "2012-02-29T05:06:07+23:59:59".to_string()
        );
        assert_eq!(
            format!("{:?}", FixedOffset::east_opt(86399).unwrap().with_ymd_and_hms(2012,
            2, 29, 5, 6, 7).unwrap()), "2012-02-29T05:06:07+23:59:59".to_string()
        );
        assert_eq!(
            format!("{:?}", FixedOffset::west_opt(86399).unwrap().with_ymd_and_hms(2012,
            3, 4, 5, 6, 7).unwrap()), "2012-03-04T05:06:07-23:59:59".to_string()
        );
        assert_eq!(
            format!("{:?}", FixedOffset::west_opt(86399).unwrap().with_ymd_and_hms(2012,
            3, 4, 5, 6, 7).unwrap()), "2012-03-04T05:06:07-23:59:59".to_string()
        );
    }
}
#[cfg(test)]
mod tests_rug_365 {
    use super::*;
    use crate::offset::fixed::FixedOffset;
    #[test]
    fn test_east() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: i32 = rug_fuzz_0;
        FixedOffset::east(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_366 {
    use super::*;
    use crate::{FixedOffset, TimeZone};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: i32 = rug_fuzz_0 * rug_fuzz_1;
        FixedOffset::east_opt(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_367 {
    use super::*;
    use crate::offset::FixedOffset;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: i32 = rug_fuzz_0;
        FixedOffset::west(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_368 {
    use super::*;
    use crate::{FixedOffset, TimeZone};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: i32 = rug_fuzz_0 * rug_fuzz_1;
        FixedOffset::west_opt(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_369 {
    use super::*;
    use crate::offset::FixedOffset;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = FixedOffset::east(rug_fuzz_0 * rug_fuzz_1);
        <FixedOffset>::local_minus_utc(&p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_370 {
    use super::*;
    use crate::offset::FixedOffset;
    #[test]
    fn test_utc_minus_local() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0 = FixedOffset::east(rug_fuzz_0 * rug_fuzz_1);
        <FixedOffset>::utc_minus_local(&p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_372 {
    use super::*;
    use crate::TimeZone;
    use crate::offset::FixedOffset;
    #[test]
    fn test_offset_from_local_date() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4)) = <(i32, i32, i32, u32, u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = FixedOffset::east(rug_fuzz_0 * rug_fuzz_1);
        let mut p1 = NaiveDate::from_ymd(rug_fuzz_2, rug_fuzz_3, rug_fuzz_4);
        p0.offset_from_local_date(&p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_373 {
    use super::*;
    use crate::offset::fixed::FixedOffset;
    use crate::prelude::*;
    #[test]
    fn test_offset_from_local_datetime() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5, mut rug_fuzz_6, mut rug_fuzz_7)) = <(i32, i32, i32, u32, u32, u32, u32, u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = FixedOffset::east(rug_fuzz_0 * rug_fuzz_1);
        let p1 = NaiveDate::from_ymd(rug_fuzz_2, rug_fuzz_3, rug_fuzz_4)
            .and_hms(rug_fuzz_5, rug_fuzz_6, rug_fuzz_7);
        FixedOffset::offset_from_local_datetime(&p0, &p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_374 {
    use super::*;
    use crate::{offset::FixedOffset, NaiveDate, TimeZone};
    #[test]
    fn test_offset_from_utc_date() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4)) = <(i32, i32, i32, u32, u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0 = FixedOffset::east(rug_fuzz_0 * rug_fuzz_1);
        let p1 = NaiveDate::from_ymd(rug_fuzz_2, rug_fuzz_3, rug_fuzz_4);
        let result = <FixedOffset as TimeZone>::offset_from_utc_date(&p0, &p1);
        debug_assert_eq!(result, FixedOffset::east(5 * 3600));
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_376 {
    use super::*;
    use crate::Offset;
    use crate::offset::FixedOffset;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut v15 = FixedOffset::east(rug_fuzz_0 * rug_fuzz_1);
        v15.fix();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_377 {
    use super::*;
    use crate::{NaiveTime, offset::FixedOffset};
    #[test]
    fn test_add() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4)) = <(u32, u32, u32, i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = NaiveTime::from_hms(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        let mut p1 = FixedOffset::east(rug_fuzz_3 * rug_fuzz_4);
        p0.add(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_378 {
    use super::*;
    use crate::{NaiveTime, offset::FixedOffset};
    #[test]
    fn test_sub() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4)) = <(u32, u32, u32, i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0 = NaiveTime::from_hms(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        let p1 = FixedOffset::east(rug_fuzz_3 * rug_fuzz_4);
        <NaiveTime as std::ops::Sub<FixedOffset>>::sub(p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_380 {
    use super::*;
    use crate::offset::FixedOffset;
    use crate::naive::datetime::NaiveDateTime;
    use std::ops::Sub;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(i64, u32, i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: NaiveDateTime = NaiveDateTime::from_timestamp(rug_fuzz_0, rug_fuzz_1);
        let p1: FixedOffset = FixedOffset::east(rug_fuzz_2 * rug_fuzz_3);
        p0.sub(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_381 {
    use super::*;
    use crate::{DateTime, Utc};
    use crate::offset::FixedOffset;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: DateTime<Utc> = Utc::now();
        let mut p1 = FixedOffset::east(rug_fuzz_0 * rug_fuzz_1);
        <DateTime<Utc> as Add<FixedOffset>>::add(p0, p1);
             }
}
}
}    }
}
