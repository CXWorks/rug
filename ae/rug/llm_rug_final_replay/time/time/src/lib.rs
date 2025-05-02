//! # Feature flags
//!
//! This crate exposes a number of features. These can be enabled or disabled as shown
//! [in Cargo's documentation](https://doc.rust-lang.org/cargo/reference/features.html). Features
//! are _disabled_ by default unless otherwise noted.
//!
//! Reliance on a given feature is always indicated alongside the item definition.
//!
//! - `std` (_enabled by default, implicitly enables `alloc`_)
//!
//!   This enables a number of features that depend on the standard library.
//!
//! - `alloc` (_enabled by default via `std`_)
//!
//!   Enables a number of features that require the ability to dynamically allocate memory.
//!
//! - `macros`
//!
//!   Enables macros that provide compile-time verification of values and intuitive syntax.
//!
//! - `formatting` (_implicitly enables `std`_)
//!
//!   Enables formatting of most structs.
//!
//! - `parsing`
//!
//!   Enables parsing of most structs.
//!
//! - `local-offset` (_implicitly enables `std`_)
//!
//!   This feature enables a number of methods that allow obtaining the system's UTC offset.
//!
//! - `large-dates`
//!
//!   By default, only years within the ±9999 range (inclusive) are supported. If you need support
//!   for years outside this range, consider enabling this feature; the supported range will be
//!   increased to ±999,999.
//!
//!   Note that enabling this feature has some costs, as it means forgoing some optimizations.
//!   Ambiguities may be introduced when parsing that would not otherwise exist.
//!
//! - `serde`
//!
//!   Enables [serde](https://docs.rs/serde) support for all types except [`Instant`].
//!
//! - `serde-human-readable` (_implicitly enables `serde`, `formatting`, and `parsing`_)
//!
//!   Allows serde representations to use a human-readable format. This is determined by the
//!   serializer, not the user. If this feature is not enabled or if the serializer requests a
//!   non-human-readable format, a format optimized for binary representation will be used.
//!
//!   Libraries should never enable this feature, as the decision of what format to use should be up
//!   to the user.
//!
//! - `serde-well-known` (_implicitly enables `serde-human-readable`_)
//!
//!   _This feature flag is deprecated and will be removed in a future breaking release. Use the
//!   `serde-human-readable` feature instead._
//!
//!   Enables support for serializing and deserializing well-known formats using serde's
//!   [`#[with]` attribute](https://serde.rs/field-attrs.html#with).
//!
//! - `rand`
//!
//!   Enables [rand](https://docs.rs/rand) support for all types.
//!
//! - `quickcheck` (_implicitly enables `alloc`_)
//!
//!   Enables [quickcheck](https://docs.rs/quickcheck) support for all types except [`Instant`].
//!
//! - `wasm-bindgen`
//!
//!   Enables [wasm-bindgen](https://github.com/rustwasm/wasm-bindgen) support for converting
//!   [JavaScript dates](https://rustwasm.github.io/wasm-bindgen/api/js_sys/struct.Date.html), as
//!   well as obtaining the UTC offset from JavaScript.
#![doc(html_playground_url = "https://play.rust-lang.org")]
#![cfg_attr(__time_03_docs, feature(doc_auto_cfg, doc_notable_trait))]
#![cfg_attr(coverage_nightly, feature(no_coverage))]
#![cfg_attr(not(feature = "std"), no_std)]
#![warn(
    clippy::dbg_macro,
    clippy::decimal_literal_representation,
    clippy::get_unwrap,
    clippy::missing_docs_in_private_items,
    clippy::nursery,
    clippy::print_stdout,
    clippy::todo,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnested_or_patterns,
    clippy::unwrap_in_result,
    clippy::unwrap_used,
    clippy::use_debug,
    deprecated_in_future,
    missing_copy_implementations,
    missing_debug_implementations,
    unused_qualifications,
    variant_size_differences
)]
#![allow(
    clippy::redundant_pub_crate,
    clippy::option_if_let_else,
    clippy::unused_peekable,
    clippy::std_instead_of_core,
)]
#![doc(html_favicon_url = "https://avatars0.githubusercontent.com/u/55999857")]
#![doc(html_logo_url = "https://avatars0.githubusercontent.com/u/55999857")]
#[allow(unused_extern_crates)]
#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(unsound_local_offset)]
compile_error!(
    "The `unsound_local_offset` flag was removed in time 0.3.18. If you need this functionality, \
     see the `time::util::local_offset::set_soundness` function."
);
/// Helper macro for easily implementing `OpAssign`.
macro_rules! __impl_assign {
    ($sym:tt $op:ident $fn:ident $target:ty : $($(#[$attr:meta])* $t:ty),+) => {
        $(#[allow(unused_qualifications)] $(#[$attr])* impl core::ops::$op <$t > for
        $target { fn $fn (& mut self, rhs : $t) { * self = * self $sym rhs; } })+
    };
}
/// Implement `AddAssign` for the provided types.
macro_rules! impl_add_assign {
    ($target:ty : $($(#[$attr:meta])* $t:ty),+ $(,)?) => {
        __impl_assign!(+ AddAssign add_assign $target : $($(#[$attr])* $t),+);
    };
}
/// Implement `SubAssign` for the provided types.
macro_rules! impl_sub_assign {
    ($target:ty : $($(#[$attr:meta])* $t:ty),+ $(,)?) => {
        __impl_assign!(- SubAssign sub_assign $target : $($(#[$attr])* $t),+);
    };
}
/// Implement `MulAssign` for the provided types.
macro_rules! impl_mul_assign {
    ($target:ty : $($(#[$attr:meta])* $t:ty),+ $(,)?) => {
        __impl_assign!(* MulAssign mul_assign $target : $($(#[$attr])* $t),+);
    };
}
/// Implement `DivAssign` for the provided types.
macro_rules! impl_div_assign {
    ($target:ty : $($(#[$attr:meta])* $t:ty),+ $(,)?) => {
        __impl_assign!(/ DivAssign div_assign $target : $($(#[$attr])* $t),+);
    };
}
/// Division of integers, rounding the resulting value towards negative infinity.
macro_rules! div_floor {
    ($a:expr, $b:expr) => {
        { let _a = $a; let _b = $b; let (_quotient, _remainder) = (_a / _b, _a % _b); if
        (_remainder > 0 && _b < 0) || (_remainder < 0 && _b > 0) { _quotient - 1 } else {
        _quotient } }
    };
}
/// Cascade an out-of-bounds value.
macro_rules! cascade {
    (@ ordinal ordinal) => {};
    (@ year year) => {};
    ($from:ident in $min:literal .. $max:expr => $to:tt) => {
        #[allow(unused_comparisons, unused_assignments)] let min = $min; let max = $max;
        if $from >= max { $from -= max - min; $to += 1; } else if $from < min { $from +=
        max - min; $to -= 1; }
    };
    ($ordinal:ident => $year:ident) => {
        cascade!(@ ordinal $ordinal); cascade!(@ year $year);
        #[allow(unused_assignments)] if $ordinal > crate ::util::days_in_year($year) as
        i16 { $ordinal -= crate ::util::days_in_year($year) as i16; $year += 1; } else if
        $ordinal < 1 { $year -= 1; $ordinal += crate ::util::days_in_year($year) as i16;
        }
    };
}
/// Returns `Err(error::ComponentRange)` if the value is not in range.
macro_rules! ensure_value_in_range {
    ($value:ident in $start:expr => $end:expr) => {
        { let _start = $start; let _end = $end; #[allow(trivial_numeric_casts,
        unused_comparisons)] if $value < _start || $value > _end { return Err(crate
        ::error::ComponentRange { name : stringify!($value), minimum : _start as _,
        maximum : _end as _, value : $value as _, conditional_range : false, }); } }
    };
    ($value:ident conditionally in $start:expr => $end:expr) => {
        { let _start = $start; let _end = $end; #[allow(trivial_numeric_casts,
        unused_comparisons)] if $value < _start || $value > _end { return Err(crate
        ::error::ComponentRange { name : stringify!($value), minimum : _start as _,
        maximum : _end as _, value : $value as _, conditional_range : true, }); } }
    };
}
/// Try to unwrap an expression, returning if not possible.
///
/// This is similar to the `?` operator, but does not perform `.into()`. Because of this, it is
/// usable in `const` contexts.
macro_rules! const_try {
    ($e:expr) => {
        match $e { Ok(value) => value, Err(error) => return Err(error), }
    };
}
/// Try to unwrap an expression, returning if not possible.
///
/// This is similar to the `?` operator, but is usable in `const` contexts.
macro_rules! const_try_opt {
    ($e:expr) => {
        match $e { Some(value) => value, None => return None, }
    };
}
/// Try to unwrap an expression, panicking if not possible.
///
/// This is similar to `$e.expect($message)`, but is usable in `const` contexts.
macro_rules! expect_opt {
    ($e:expr, $message:literal) => {
        match $e { Some(value) => value, None => crate ::expect_failed($message), }
    };
}
/// `unreachable!()`, but better.
macro_rules! bug {
    () => {
        compile_error!("provide an error message to help fix a possible bug")
    };
    ($descr:literal $($rest:tt)?) => {
        panic!(concat!("internal error: ", $descr) $($rest)?)
    };
}
mod date;
mod date_time;
mod duration;
pub mod error;
pub mod ext;
#[cfg(any(feature = "formatting", feature = "parsing"))]
pub mod format_description;
#[cfg(feature = "formatting")]
pub mod formatting;
#[cfg(feature = "std")]
mod instant;
#[cfg(feature = "macros")]
pub mod macros;
mod month;
mod offset_date_time;
#[cfg(feature = "parsing")]
pub mod parsing;
mod primitive_date_time;
#[cfg(feature = "quickcheck")]
mod quickcheck;
#[cfg(feature = "rand")]
mod rand;
#[cfg(feature = "serde")]
#[allow(missing_copy_implementations, missing_debug_implementations)]
pub mod serde;
mod sys;
#[cfg(test)]
mod tests;
mod time;
mod utc_offset;
pub mod util;
mod weekday;
use time_core::convert;
pub use crate::date::Date;
use crate::date_time::DateTime;
pub use crate::duration::Duration;
pub use crate::error::Error;
#[cfg(feature = "std")]
pub use crate::instant::Instant;
pub use crate::month::Month;
pub use crate::offset_date_time::OffsetDateTime;
pub use crate::primitive_date_time::PrimitiveDateTime;
pub use crate::time::Time;
pub use crate::utc_offset::UtcOffset;
pub use crate::weekday::Weekday;
/// An alias for [`std::result::Result`] with a generic error from the time crate.
pub type Result<T> = core::result::Result<T, Error>;
/// This is a separate function to reduce the code size of `expect_opt!`.
#[inline(never)]
#[cold]
#[track_caller]
const fn expect_failed(message: &str) -> ! {
    panic!("{}", message)
}
#[cfg(test)]
mod tests_rug_86 {
    use super::*;
    #[test]
    fn test_expect_failed() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: &str = rug_fuzz_0;
        expect_failed(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_93 {
    use super::*;
    use std::ops::AddAssign;
    use duration::Duration;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(i64, i32, i64, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Duration::new(rug_fuzz_0, rug_fuzz_1);
        let mut p1 = Duration::new(rug_fuzz_2, rug_fuzz_3);
        p0.add_assign(p1);
        debug_assert_eq!(p0, Duration::new(1, 0));
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_94 {
    use super::*;
    use std::ops::AddAssign;
    #[test]
    fn test_add_assign() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(i64, i32, u64, u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = duration::Duration::new(rug_fuzz_0, rug_fuzz_1);
        let p1 = std::time::Duration::new(rug_fuzz_2, rug_fuzz_3);
        <duration::Duration as std::ops::AddAssign<
            std::time::Duration,
        >>::add_assign(&mut p0, p1);
        debug_assert_eq!(p0, duration::Duration::new(3, 0));
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_96 {
    use super::*;
    use std::ops::SubAssign;
    use std::time::Duration as StdDuration;
    use duration::Duration;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(i64, i32, u64, u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Duration::new(rug_fuzz_0, rug_fuzz_1);
        let mut p1 = StdDuration::new(rug_fuzz_2, rug_fuzz_3);
        <Duration as SubAssign<StdDuration>>::sub_assign(&mut p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_97 {
    use super::*;
    use std::ops::MulAssign;
    use duration::Duration;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(i64, i32, i8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Duration::new(rug_fuzz_0, rug_fuzz_1);
        let p1: i8 = rug_fuzz_2;
        <Duration as MulAssign<i8>>::mul_assign(&mut p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_98 {
    use super::*;
    use std::ops::MulAssign;
    use crate::duration::Duration;
    #[test]
    fn test_duration_mul_assign() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(i64, i32, i16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Duration::new(rug_fuzz_0, rug_fuzz_1);
        let p1: i16 = rug_fuzz_2;
        p0.mul_assign(p1);
        debug_assert_eq!(p0, Duration::new(5, 0));
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_99 {
    use super::*;
    use crate::Duration;
    use std::ops::MulAssign;
    #[test]
    fn test_mul_assign() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i64, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Duration::seconds(rug_fuzz_0);
        let p1: i32 = rug_fuzz_1;
        p0.mul_assign(p1);
        debug_assert_eq!(p0, Duration::seconds(50));
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_101 {
    use super::*;
    use std::ops::MulAssign;
    use crate::{Duration, ext::NumericalDuration};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i64, u16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Duration::seconds(rug_fuzz_0);
        let mut p1 = rug_fuzz_1;
        p0.mul_assign(p1);
        debug_assert_eq!(p0, Duration::seconds(50));
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_105 {
    use super::*;
    use std::ops::DivAssign;
    use crate::Duration;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i64, i8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Duration::seconds(rug_fuzz_0);
        let mut p1 = rug_fuzz_1;
        p0.div_assign(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_108 {
    use super::*;
    use std::ops::DivAssign;
    use crate::date_time::DateTime;
    use crate::duration::Duration;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(i64, i32, u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Duration::new(rug_fuzz_0, rug_fuzz_1);
        let mut p1: u8 = rug_fuzz_2;
        <Duration as DivAssign<u8>>::div_assign(&mut p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_114 {
    use super::*;
    use std::ops::AddAssign;
    use instant::Instant;
    use std::time::Duration;
    #[test]
    fn test_add_assign() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Instant::now();
        let p1 = Duration::from_secs(rug_fuzz_0);
        p0.add_assign(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_117 {
    use super::*;
    use instant::Instant;
    use std::ops::SubAssign;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Instant::now();
        let mut p1 = std::time::Duration::from_secs(rug_fuzz_0);
        <Instant as std::ops::SubAssign<std::time::Duration>>::sub_assign(&mut p0, p1);
             }
}
}
}    }
}
