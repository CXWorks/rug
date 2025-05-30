//! [![github]](https://github.com/dtolnay/itoa)&ensp;[![crates-io]](https://crates.io/crates/itoa)&ensp;[![docs-rs]](https://docs.rs/itoa)
//!
//! [github]: https://img.shields.io/badge/github-8da0cb?style=for-the-badge&labelColor=555555&logo=github
//! [crates-io]: https://img.shields.io/badge/crates.io-fc8d62?style=for-the-badge&labelColor=555555&logo=rust
//! [docs-rs]: https://img.shields.io/badge/docs.rs-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs
//!
//! <br>
//!
//! This crate provides a fast conversion of integer primitives to decimal
//! strings. The implementation comes straight from [libcore] but avoids the
//! performance penalty of going through [`core::fmt::Formatter`].
//!
//! See also [`ryu`] for printing floating point primitives.
//!
//! [libcore]: https://github.com/rust-lang/rust/blob/b8214dc6c6fc20d0a660fb5700dca9ebf51ebe89/src/libcore/fmt/num.rs#L201-L254
//! [`core::fmt::Formatter`]: https://doc.rust-lang.org/std/fmt/struct.Formatter.html
//! [`ryu`]: https://github.com/dtolnay/ryu
//!
//! # Example
//!
//! ```
//! fn main() {
//!     let mut buffer = itoa::Buffer::new();
//!     let printed = buffer.format(128u64);
//!     assert_eq!(printed, "128");
//! }
//! ```
//!
//! # Performance (lower is better)
//!
//! ![performance](https://raw.githubusercontent.com/dtolnay/itoa/master/performance.png)
#![doc(html_root_url = "https://docs.rs/itoa/1.0.6")]
#![allow(
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::must_use_candidate,
    clippy::unreadable_literal
)]
mod udiv128;
use core::mem::{self, MaybeUninit};
use core::{ptr, slice, str};
#[cfg(feature = "no-panic")]
use no_panic::no_panic;
/// A correctly sized stack allocation for the formatted integer to be written
/// into.
///
/// # Example
///
/// ```
/// let mut buffer = itoa::Buffer::new();
/// let printed = buffer.format(1234);
/// assert_eq!(printed, "1234");
/// ```
pub struct Buffer {
    bytes: [MaybeUninit<u8>; I128_MAX_LEN],
}
impl Default for Buffer {
    #[inline]
    fn default() -> Buffer {
        Buffer::new()
    }
}
impl Clone for Buffer {
    #[inline]
    fn clone(&self) -> Self {
        Buffer::new()
    }
}
impl Buffer {
    /// This is a cheap operation; you don't need to worry about reusing buffers
    /// for efficiency.
    #[inline]
    #[cfg_attr(feature = "no-panic", no_panic)]
    pub fn new() -> Buffer {
        let bytes = [MaybeUninit::<u8>::uninit(); I128_MAX_LEN];
        Buffer { bytes }
    }
    /// Print an integer into this buffer and return a reference to its string
    /// representation within the buffer.
    #[cfg_attr(feature = "no-panic", no_panic)]
    pub fn format<I: Integer>(&mut self, i: I) -> &str {
        i.write(unsafe {
            &mut *(&mut self.bytes as *mut [MaybeUninit<u8>; I128_MAX_LEN]
                as *mut <I as private::Sealed>::Buffer)
        })
    }
}
/// An integer that can be written into an [`itoa::Buffer`][Buffer].
///
/// This trait is sealed and cannot be implemented for types outside of itoa.
pub trait Integer: private::Sealed {}
mod private {
    pub trait Sealed: Copy {
        type Buffer: 'static;
        fn write(self, buf: &mut Self::Buffer) -> &str;
    }
}
const DEC_DIGITS_LUT: &[u8] = b"\
      0001020304050607080910111213141516171819\
      2021222324252627282930313233343536373839\
      4041424344454647484950515253545556575859\
      6061626364656667686970717273747576777879\
      8081828384858687888990919293949596979899";
macro_rules! impl_Integer {
    ($($max_len:expr => $t:ident),* as $conv_fn:ident) => {
        $(impl Integer for $t {} impl private::Sealed for $t { type Buffer = [MaybeUninit
        < u8 >; $max_len]; #[allow(unused_comparisons)] #[inline] #[cfg_attr(feature =
        "no-panic", no_panic)] fn write(self, buf : & mut [MaybeUninit < u8 >; $max_len])
        -> & str { let is_nonnegative = self >= 0; let mut n = if is_nonnegative { self
        as $conv_fn } else { (! (self as $conv_fn)).wrapping_add(1) }; let mut curr = buf
        .len() as isize; let buf_ptr = buf.as_mut_ptr() as * mut u8; let lut_ptr =
        DEC_DIGITS_LUT.as_ptr(); unsafe { if mem::size_of::<$t > () >= 2 { while n >=
        10000 { let rem = (n % 10000) as isize; n /= 10000; let d1 = (rem / 100) << 1;
        let d2 = (rem % 100) << 1; curr -= 4; ptr::copy_nonoverlapping(lut_ptr
        .offset(d1), buf_ptr.offset(curr), 2); ptr::copy_nonoverlapping(lut_ptr
        .offset(d2), buf_ptr.offset(curr + 2), 2); } } let mut n = n as isize; if n >=
        100 { let d1 = (n % 100) << 1; n /= 100; curr -= 2;
        ptr::copy_nonoverlapping(lut_ptr.offset(d1), buf_ptr.offset(curr), 2); } if n <
        10 { curr -= 1; * buf_ptr.offset(curr) = (n as u8) + b'0'; } else { let d1 = n <<
        1; curr -= 2; ptr::copy_nonoverlapping(lut_ptr.offset(d1), buf_ptr.offset(curr),
        2); } if ! is_nonnegative { curr -= 1; * buf_ptr.offset(curr) = b'-'; } } let len
        = buf.len() - curr as usize; let bytes = unsafe { slice::from_raw_parts(buf_ptr
        .offset(curr), len) }; unsafe { str::from_utf8_unchecked(bytes) } } })*
    };
}
const I8_MAX_LEN: usize = 4;
const U8_MAX_LEN: usize = 3;
const I16_MAX_LEN: usize = 6;
const U16_MAX_LEN: usize = 5;
const I32_MAX_LEN: usize = 11;
const U32_MAX_LEN: usize = 10;
const I64_MAX_LEN: usize = 20;
const U64_MAX_LEN: usize = 20;
impl_Integer!(
    I8_MAX_LEN => i8, U8_MAX_LEN => u8, I16_MAX_LEN => i16, U16_MAX_LEN => u16,
    I32_MAX_LEN => i32, U32_MAX_LEN => u32 as u32
);
impl_Integer!(I64_MAX_LEN => i64, U64_MAX_LEN => u64 as u64);
#[cfg(target_pointer_width = "16")]
impl_Integer!(I16_MAX_LEN => isize, U16_MAX_LEN => usize as u16);
#[cfg(target_pointer_width = "32")]
impl_Integer!(I32_MAX_LEN => isize, U32_MAX_LEN => usize as u32);
#[cfg(target_pointer_width = "64")]
impl_Integer!(I64_MAX_LEN => isize, U64_MAX_LEN => usize as u64);
macro_rules! impl_Integer128 {
    ($($max_len:expr => $t:ident),*) => {
        $(impl Integer for $t {} impl private::Sealed for $t { type Buffer = [MaybeUninit
        < u8 >; $max_len]; #[allow(unused_comparisons)] #[inline] #[cfg_attr(feature =
        "no-panic", no_panic)] fn write(self, buf : & mut [MaybeUninit < u8 >; $max_len])
        -> & str { let is_nonnegative = self >= 0; let n = if is_nonnegative { self as
        u128 } else { (! (self as u128)).wrapping_add(1) }; let mut curr = buf.len() as
        isize; let buf_ptr = buf.as_mut_ptr() as * mut u8; unsafe { let (n, rem) =
        udiv128::udivmod_1e19(n); let buf1 = buf_ptr.offset(curr - U64_MAX_LEN as isize)
        as * mut [MaybeUninit < u8 >; U64_MAX_LEN]; curr -= rem.write(& mut * buf1).len()
        as isize; if n != 0 { let target = buf.len() as isize - 19;
        ptr::write_bytes(buf_ptr.offset(target), b'0', (curr - target) as usize); curr =
        target; let (n, rem) = udiv128::udivmod_1e19(n); let buf2 = buf_ptr.offset(curr -
        U64_MAX_LEN as isize) as * mut [MaybeUninit < u8 >; U64_MAX_LEN]; curr -= rem
        .write(& mut * buf2).len() as isize; if n != 0 { let target = buf.len() as isize
        - 38; ptr::write_bytes(buf_ptr.offset(target), b'0', (curr - target) as usize);
        curr = target; curr -= 1; * buf_ptr.offset(curr) = (n as u8) + b'0'; } } if !
        is_nonnegative { curr -= 1; * buf_ptr.offset(curr) = b'-'; } let len = buf.len()
        - curr as usize; let bytes = slice::from_raw_parts(buf_ptr.offset(curr), len);
        str::from_utf8_unchecked(bytes) } } })*
    };
}
const U128_MAX_LEN: usize = 39;
const I128_MAX_LEN: usize = 40;
impl_Integer128!(I128_MAX_LEN => i128, U128_MAX_LEN => u128);
#[cfg(test)]
mod tests_rug_5 {
    use super::*;
    use crate::Buffer;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_5_rrrruuuugggg_test_rug = 0;
        Buffer::new();
        let _rug_ed_tests_rug_5_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_6 {
    use super::*;
    use crate::{Buffer, Integer, I128_MAX_LEN};
    use std::mem::MaybeUninit;
    use crate::private::Sealed;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i128) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Buffer = Buffer::new();
        let p1: i128 = rug_fuzz_0;
        p0.format(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_13 {
    use super::*;
    use crate::private::Sealed;
    use std::mem::MaybeUninit;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: i64 = rug_fuzz_0;
        let mut p1: [MaybeUninit<u8>; 20] = unsafe {
            MaybeUninit::uninit().assume_init()
        };
        p0.write(&mut p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_14 {
    use super::*;
    use crate::private::Sealed;
    #[test]
    fn test_write() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: u64 = rug_fuzz_0;
        let p1: &mut [std::mem::MaybeUninit<u8>; 20] = &mut [std::mem::MaybeUninit::uninit(); 20];
        <u64>::write(p0, p1);
             }
}
}
}    }
}
