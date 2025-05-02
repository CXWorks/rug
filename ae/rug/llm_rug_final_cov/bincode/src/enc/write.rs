//! This module contains writer-based structs and traits.
//!
//! Because `std::io::Write` is only limited to `std` and not `core`, we provide our own [Writer].
use crate::error::EncodeError;
/// Trait that indicates that a struct can be used as a destination to encode data too. This is used by [Encode]
///
/// [Encode]: ../trait.Encode.html
pub trait Writer {
    /// Write `bytes` to the underlying writer. Exactly `bytes.len()` bytes must be written, or else an error should be returned.
    fn write(&mut self, bytes: &[u8]) -> Result<(), EncodeError>;
}
impl<T: Writer> Writer for &mut T {
    #[inline]
    fn write(&mut self, bytes: &[u8]) -> Result<(), EncodeError> {
        (**self).write(bytes)
    }
}
/// A helper struct that implements `Writer` for a `&[u8]` slice.
///
/// ```
/// use bincode::enc::write::{Writer, SliceWriter};
///
/// let destination = &mut [0u8; 100];
/// let mut writer = SliceWriter::new(destination);
/// writer.write(&[1, 2, 3, 4, 5]).unwrap();
///
/// assert_eq!(writer.bytes_written(), 5);
/// assert_eq!(destination[0..6], [1, 2, 3, 4, 5, 0]);
/// ```
pub struct SliceWriter<'storage> {
    slice: &'storage mut [u8],
    original_length: usize,
}
impl<'storage> SliceWriter<'storage> {
    /// Create a new instance of `SliceWriter` with the given byte array.
    pub fn new(bytes: &'storage mut [u8]) -> SliceWriter<'storage> {
        let original = bytes.len();
        SliceWriter {
            slice: bytes,
            original_length: original,
        }
    }
    /// Return the amount of bytes written so far.
    pub fn bytes_written(&self) -> usize {
        self.original_length - self.slice.len()
    }
}
impl<'storage> Writer for SliceWriter<'storage> {
    #[inline(always)]
    fn write(&mut self, bytes: &[u8]) -> Result<(), EncodeError> {
        if bytes.len() > self.slice.len() {
            return Err(EncodeError::UnexpectedEnd);
        }
        let (a, b) = core::mem::take(&mut self.slice).split_at_mut(bytes.len());
        a.copy_from_slice(bytes);
        self.slice = b;
        Ok(())
    }
}
/// A writer that counts how many bytes were written. This is useful for e.g. pre-allocating buffers bfeore writing to them.
#[derive(Default)]
pub struct SizeWriter {
    /// the amount of bytes that were written so far
    pub bytes_written: usize,
}
impl Writer for SizeWriter {
    #[inline(always)]
    fn write(&mut self, bytes: &[u8]) -> Result<(), EncodeError> {
        self.bytes_written += bytes.len();
        Ok(())
    }
}
#[cfg(test)]
mod tests_rug_303 {
    use super::*;
    use crate::enc::write::SliceWriter;
    #[test]
    fn test_slice_writer_new() {
        let _rug_st_tests_rug_303_rrrruuuugggg_test_slice_writer_new = 0;
        let mut p0 = vec![0u8; 10];
        SliceWriter::new(&mut p0);
        let _rug_ed_tests_rug_303_rrrruuuugggg_test_slice_writer_new = 0;
    }
}
#[cfg(test)]
mod tests_rug_304 {
    use super::*;
    use crate::enc::write::SliceWriter;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_304_rrrruuuugggg_test_rug = 0;
        let mut storage: Vec<u8> = Vec::new();
        let slice_writer: SliceWriter<'_> = SliceWriter::new(&mut storage);
        let p0: &SliceWriter<'_> = &slice_writer;
        p0.bytes_written();
        let _rug_ed_tests_rug_304_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_305 {
    use super::*;
    use crate::enc::write::Writer;
    use crate::enc::write::SliceWriter;
    #[test]
    fn test_write() {
        let _rug_st_tests_rug_305_rrrruuuugggg_test_write = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let mut storage: Vec<u8> = Vec::new();
        let mut p0: SliceWriter<'_> = SliceWriter::new(&mut storage);
        let p1: &[u8] = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        p0.write(p1).unwrap();
        debug_assert_eq!(storage.as_slice(), & [1, 2, 3]);
        let _rug_ed_tests_rug_305_rrrruuuugggg_test_write = 0;
    }
}
#[cfg(test)]
mod tests_rug_306_prepare {
    use crate::enc::write::SizeWriter;
    #[test]
    fn sample() {
        let _rug_st_tests_rug_306_prepare_rrrruuuugggg_sample = 0;
        let rug_fuzz_0 = 0;
        let mut v18 = SizeWriter {
            bytes_written: rug_fuzz_0,
        };
        let _rug_ed_tests_rug_306_prepare_rrrruuuugggg_sample = 0;
    }
}
#[cfg(test)]
mod tests_rug_306 {
    use super::*;
    use crate::enc::write::Writer;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_306_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = b"sample data";
        let mut p0 = SizeWriter {
            bytes_written: rug_fuzz_0,
        };
        let p1: &[u8] = rug_fuzz_1;
        <SizeWriter as Writer>::write(&mut p0, p1).unwrap();
        let _rug_ed_tests_rug_306_rrrruuuugggg_test_rug = 0;
    }
}
