//! Implementation for miniz_oxide rust backend.
use std::convert::TryInto;
use std::fmt;
use miniz_oxide::deflate::core::CompressorOxide;
use miniz_oxide::inflate::stream::InflateState;
pub use miniz_oxide::*;
pub const MZ_NO_FLUSH: isize = MZFlush::None as isize;
pub const MZ_PARTIAL_FLUSH: isize = MZFlush::Partial as isize;
pub const MZ_SYNC_FLUSH: isize = MZFlush::Sync as isize;
pub const MZ_FULL_FLUSH: isize = MZFlush::Full as isize;
pub const MZ_FINISH: isize = MZFlush::Finish as isize;
use super::*;
use crate::mem;
fn format_from_bool(zlib_header: bool) -> DataFormat {
    if zlib_header { DataFormat::Zlib } else { DataFormat::Raw }
}
pub struct Inflate {
    inner: Box<InflateState>,
    total_in: u64,
    total_out: u64,
}
impl fmt::Debug for Inflate {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(
            f, "miniz_oxide inflate internal state. total_in: {}, total_out: {}", self
            .total_in, self.total_out,
        )
    }
}
impl InflateBackend for Inflate {
    fn make(zlib_header: bool, window_bits: u8) -> Self {
        assert!(
            window_bits > 8 && window_bits < 16, "window_bits must be within 9 ..= 15"
        );
        let format = format_from_bool(zlib_header);
        Inflate {
            inner: InflateState::new_boxed(format),
            total_in: 0,
            total_out: 0,
        }
    }
    fn decompress(
        &mut self,
        input: &[u8],
        output: &mut [u8],
        flush: FlushDecompress,
    ) -> Result<Status, DecompressError> {
        let flush = MZFlush::new(flush as i32).unwrap();
        let res = inflate::stream::inflate(&mut self.inner, input, output, flush);
        self.total_in += res.bytes_consumed as u64;
        self.total_out += res.bytes_written as u64;
        match res.status {
            Ok(status) => {
                match status {
                    MZStatus::Ok => Ok(Status::Ok),
                    MZStatus::StreamEnd => Ok(Status::StreamEnd),
                    MZStatus::NeedDict => {
                        mem::decompress_need_dict(
                            self.inner.decompressor().adler32().unwrap_or(0),
                        )
                    }
                }
            }
            Err(status) => {
                match status {
                    MZError::Buf => Ok(Status::BufError),
                    _ => mem::decompress_failed(),
                }
            }
        }
    }
    fn reset(&mut self, zlib_header: bool) {
        self.inner.reset(format_from_bool(zlib_header));
        self.total_in = 0;
        self.total_out = 0;
    }
}
impl Backend for Inflate {
    #[inline]
    fn total_in(&self) -> u64 {
        self.total_in
    }
    #[inline]
    fn total_out(&self) -> u64 {
        self.total_out
    }
}
pub struct Deflate {
    inner: Box<CompressorOxide>,
    total_in: u64,
    total_out: u64,
}
impl fmt::Debug for Deflate {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(
            f, "miniz_oxide deflate internal state. total_in: {}, total_out: {}", self
            .total_in, self.total_out,
        )
    }
}
impl DeflateBackend for Deflate {
    fn make(level: Compression, zlib_header: bool, window_bits: u8) -> Self {
        assert!(
            window_bits > 8 && window_bits < 16, "window_bits must be within 9 ..= 15"
        );
        debug_assert!(level.level() <= 10);
        let mut inner: Box<CompressorOxide> = Box::default();
        let format = format_from_bool(zlib_header);
        inner.set_format_and_level(format, level.level().try_into().unwrap_or(1));
        Deflate {
            inner,
            total_in: 0,
            total_out: 0,
        }
    }
    fn compress(
        &mut self,
        input: &[u8],
        output: &mut [u8],
        flush: FlushCompress,
    ) -> Result<Status, CompressError> {
        let flush = MZFlush::new(flush as i32).unwrap();
        let res = deflate::stream::deflate(&mut self.inner, input, output, flush);
        self.total_in += res.bytes_consumed as u64;
        self.total_out += res.bytes_written as u64;
        match res.status {
            Ok(status) => {
                match status {
                    MZStatus::Ok => Ok(Status::Ok),
                    MZStatus::StreamEnd => Ok(Status::StreamEnd),
                    MZStatus::NeedDict => Err(CompressError(())),
                }
            }
            Err(status) => {
                match status {
                    MZError::Buf => Ok(Status::BufError),
                    _ => Err(CompressError(())),
                }
            }
        }
    }
    fn reset(&mut self) {
        self.total_in = 0;
        self.total_out = 0;
        self.inner.reset();
    }
}
impl Backend for Deflate {
    #[inline]
    fn total_in(&self) -> u64 {
        self.total_in
    }
    #[inline]
    fn total_out(&self) -> u64 {
        self.total_out
    }
}
#[cfg(test)]
mod tests_llm_16_53 {
    use super::*;
    use crate::*;
    use crate::ffi::{Backend, DeflateBackend};
    #[test]
    fn test_make() {
        let _rug_st_tests_llm_16_53_rrrruuuugggg_test_make = 0;
        let rug_fuzz_0 = 6;
        let rug_fuzz_1 = true;
        let rug_fuzz_2 = 10;
        let level = Compression::new(rug_fuzz_0);
        let zlib_header = rug_fuzz_1;
        let window_bits = rug_fuzz_2;
        let result = Deflate::make(level, zlib_header, window_bits);
        debug_assert_eq!(result.total_in, 0);
        debug_assert_eq!(result.total_out, 0);
        let _rug_ed_tests_llm_16_53_rrrruuuugggg_test_make = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_60 {
    use super::*;
    use crate::*;
    #[test]
    fn test_decompress() {
        let _rug_st_tests_llm_16_60_rrrruuuugggg_test_decompress = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = 9;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 2;
        let rug_fuzz_4 = 3;
        let rug_fuzz_5 = 4;
        let rug_fuzz_6 = 5;
        let rug_fuzz_7 = 0u8;
        let mut inflate = Inflate::make(rug_fuzz_0, rug_fuzz_1);
        let input = &[rug_fuzz_2, rug_fuzz_3, rug_fuzz_4, rug_fuzz_5, rug_fuzz_6];
        let mut output = [rug_fuzz_7; 10];
        let flush = FlushDecompress::Finish;
        let result = inflate.decompress(input, &mut output, flush);
        debug_assert_eq!(result.unwrap(), Status::Ok);
        debug_assert_eq!(inflate.total_in, input.len() as u64);
        debug_assert_ne!(inflate.total_out, 0);
        let _rug_ed_tests_llm_16_60_rrrruuuugggg_test_decompress = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_64_llm_16_63 {
    use crate::ffi::{Inflate, InflateBackend, Backend, FlushDecompress};
    #[test]
    fn test_reset() {
        let _rug_st_tests_llm_16_64_llm_16_63_rrrruuuugggg_test_reset = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = 15;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = true;
        let mut inflate = Inflate::make(rug_fuzz_0, rug_fuzz_1);
        let input = [rug_fuzz_2; 10];
        let mut output = [rug_fuzz_3; 10];
        debug_assert_eq!(
            inflate.decompress(& input, & mut output, FlushDecompress::None).unwrap(),
            crate ::Status::Ok
        );
        inflate.reset(rug_fuzz_4);
        debug_assert_eq!(inflate.total_in(), 0);
        debug_assert_eq!(inflate.total_out(), 0);
        let _rug_ed_tests_llm_16_64_llm_16_63_rrrruuuugggg_test_reset = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_300 {
    use crate::ffi::rust::{format_from_bool, DataFormat};
    #[test]
    fn test_format_from_bool() {
        let _rug_st_tests_llm_16_300_rrrruuuugggg_test_format_from_bool = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = false;
        debug_assert_eq!(format_from_bool(rug_fuzz_0), DataFormat::Zlib);
        debug_assert_eq!(format_from_bool(rug_fuzz_1), DataFormat::Raw);
        let _rug_ed_tests_llm_16_300_rrrruuuugggg_test_format_from_bool = 0;
    }
}
#[cfg(test)]
mod tests_rug_14 {
    use super::*;
    use crate::ffi::{Inflate, InflateBackend};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_14_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = 10;
        let mut p0 = rug_fuzz_0;
        let mut p1 = rug_fuzz_1;
        <Inflate as InflateBackend>::make(p0, p1);
        let _rug_ed_tests_rug_14_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_15 {
    use super::*;
    use crate::ffi::{Backend, Inflate, InflateBackend};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_15_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = 9;
        let mut p0 = Inflate::make(rug_fuzz_0, rug_fuzz_1);
        <Inflate as Backend>::total_in(&p0);
        let _rug_ed_tests_rug_15_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_16 {
    use super::*;
    use crate::ffi::{Backend, Inflate, InflateBackend};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_16_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = 9;
        let mut p0 = Inflate::make(rug_fuzz_0, rug_fuzz_1);
        <Inflate as Backend>::total_out(&p0);
        let _rug_ed_tests_rug_16_rrrruuuugggg_test_rug = 0;
    }
}
