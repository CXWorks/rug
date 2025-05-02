//! Simple CRC bindings backed by miniz.c
use std::io;
use std::io::prelude::*;
use crc32fast::Hasher;
/// The CRC calculated by a [`CrcReader`].
///
/// [`CrcReader`]: struct.CrcReader.html
#[derive(Debug)]
pub struct Crc {
    amt: u32,
    hasher: Hasher,
}
/// A wrapper around a [`Read`] that calculates the CRC.
///
/// [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
#[derive(Debug)]
pub struct CrcReader<R> {
    inner: R,
    crc: Crc,
}
impl Crc {
    /// Create a new CRC.
    pub fn new() -> Crc {
        Crc {
            amt: 0,
            hasher: Hasher::new(),
        }
    }
    /// Returns the current crc32 checksum.
    pub fn sum(&self) -> u32 {
        self.hasher.clone().finalize()
    }
    /// The number of bytes that have been used to calculate the CRC.
    /// This value is only accurate if the amount is lower than 2<sup>32</sup>.
    pub fn amount(&self) -> u32 {
        self.amt
    }
    /// Update the CRC with the bytes in `data`.
    pub fn update(&mut self, data: &[u8]) {
        self.amt = self.amt.wrapping_add(data.len() as u32);
        self.hasher.update(data);
    }
    /// Reset the CRC.
    pub fn reset(&mut self) {
        self.amt = 0;
        self.hasher.reset();
    }
    /// Combine the CRC with the CRC for the subsequent block of bytes.
    pub fn combine(&mut self, additional_crc: &Crc) {
        self.amt += additional_crc.amt;
        self.hasher.combine(&additional_crc.hasher);
    }
}
impl<R: Read> CrcReader<R> {
    /// Create a new CrcReader.
    pub fn new(r: R) -> CrcReader<R> {
        CrcReader {
            inner: r,
            crc: Crc::new(),
        }
    }
}
impl<R> CrcReader<R> {
    /// Get the Crc for this CrcReader.
    pub fn crc(&self) -> &Crc {
        &self.crc
    }
    /// Get the reader that is wrapped by this CrcReader.
    pub fn into_inner(self) -> R {
        self.inner
    }
    /// Get the reader that is wrapped by this CrcReader by reference.
    pub fn get_ref(&self) -> &R {
        &self.inner
    }
    /// Get a mutable reference to the reader that is wrapped by this CrcReader.
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }
    /// Reset the Crc in this CrcReader.
    pub fn reset(&mut self) {
        self.crc.reset();
    }
}
impl<R: Read> Read for CrcReader<R> {
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        let amt = self.inner.read(into)?;
        self.crc.update(&into[..amt]);
        Ok(amt)
    }
}
impl<R: BufRead> BufRead for CrcReader<R> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.inner.fill_buf()
    }
    fn consume(&mut self, amt: usize) {
        if let Ok(data) = self.inner.fill_buf() {
            self.crc.update(&data[..amt]);
        }
        self.inner.consume(amt);
    }
}
/// A wrapper around a [`Write`] that calculates the CRC.
///
/// [`Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
#[derive(Debug)]
pub struct CrcWriter<W> {
    inner: W,
    crc: Crc,
}
impl<W> CrcWriter<W> {
    /// Get the Crc for this CrcWriter.
    pub fn crc(&self) -> &Crc {
        &self.crc
    }
    /// Get the writer that is wrapped by this CrcWriter.
    pub fn into_inner(self) -> W {
        self.inner
    }
    /// Get the writer that is wrapped by this CrcWriter by reference.
    pub fn get_ref(&self) -> &W {
        &self.inner
    }
    /// Get a mutable reference to the writer that is wrapped by this CrcWriter.
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.inner
    }
    /// Reset the Crc in this CrcWriter.
    pub fn reset(&mut self) {
        self.crc.reset();
    }
}
impl<W: Write> CrcWriter<W> {
    /// Create a new CrcWriter.
    pub fn new(w: W) -> CrcWriter<W> {
        CrcWriter {
            inner: w,
            crc: Crc::new(),
        }
    }
}
impl<W: Write> Write for CrcWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let amt = self.inner.write(buf)?;
        self.crc.update(&buf[..amt]);
        Ok(amt)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
#[cfg(test)]
mod tests_llm_16_6 {
    use crate::crc::{Crc, CrcReader};
    use std::io::{BufRead, Read};
    #[test]
    fn test_consume() {
        let _rug_st_tests_llm_16_6_rrrruuuugggg_test_consume = 0;
        let rug_fuzz_0 = b"hello world";
        let rug_fuzz_1 = 0x2ef0_6119;
        let data = rug_fuzz_0;
        let expected_crc = rug_fuzz_1;
        let crc = Crc::new();
        let mut reader = CrcReader::new(data as &[u8]);
        reader.fill_buf().unwrap();
        reader.consume(data.len());
        debug_assert_eq!(crc.sum(), expected_crc);
        let _rug_ed_tests_llm_16_6_rrrruuuugggg_test_consume = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_11 {
    use std::io;
    use std::io::Write;
    use crate::crc::CrcWriter;
    use crate::crc::Crc;
    #[test]
    fn test_flush() -> io::Result<()> {
        let mut writer: CrcWriter<Vec<u8>> = CrcWriter::new(Vec::new());
        writer.write_all(b"hello")?;
        writer.flush()?;
        let crc: &Crc = writer.crc();
        assert_eq!(crc.sum(), 0x3610a686);
        Ok(())
    }
}
#[cfg(test)]
mod tests_llm_16_13 {
    use super::*;
    use crate::*;
    use std::io::{Write, Error, ErrorKind};
    #[test]
    fn test_write() {
        let _rug_st_tests_llm_16_13_rrrruuuugggg_test_write = 0;
        let rug_fuzz_0 = "Hello, World!";
        let mut buffer: Vec<u8> = Vec::new();
        let mut crc_writer = CrcWriter::new(&mut buffer);
        let data = rug_fuzz_0.as_bytes();
        let result = crc_writer.write(data);
        debug_assert_eq!(result.unwrap(), data.len());
        debug_assert_eq!(crc_writer.crc().sum(), 3467524208);
        let expected_buffer: Vec<u8> = data.iter().copied().collect();
        debug_assert_eq!(buffer, expected_buffer);
        let _rug_ed_tests_llm_16_13_rrrruuuugggg_test_write = 0;
    }
    #[test]
    fn test_write_with_error() {
        let _rug_st_tests_llm_16_13_rrrruuuugggg_test_write_with_error = 0;
        let rug_fuzz_0 = "Hello, World!";
        let mut crc_writer = CrcWriter::new(ErrorWriter);
        let data = rug_fuzz_0.as_bytes();
        let result = crc_writer.write(data);
        debug_assert!(result.is_err());
        debug_assert_eq!(crc_writer.crc().sum(), 0);
        let _rug_ed_tests_llm_16_13_rrrruuuugggg_test_write_with_error = 0;
    }
    struct ErrorWriter;
    impl Write for ErrorWriter {
        fn write(&mut self, _: &[u8]) -> Result<usize, Error> {
            Err(Error::new(ErrorKind::Other, "Write error"))
        }
        fn flush(&mut self) -> Result<(), Error> {
            Ok(())
        }
    }
}
#[cfg(test)]
mod tests_llm_16_195_llm_16_194 {
    use super::*;
    use crate::*;
    use crate::*;
    use crate::crc::Crc;
    #[test]
    fn test_amount() {
        let _rug_st_tests_llm_16_195_llm_16_194_rrrruuuugggg_test_amount = 0;
        let rug_fuzz_0 = 0x61;
        let rug_fuzz_1 = 0x62;
        let rug_fuzz_2 = 0x63;
        let rug_fuzz_3 = 3;
        let mut crc = Crc::new();
        let data = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        crc.update(&data);
        debug_assert_eq!(rug_fuzz_3, crc.amount());
        let _rug_ed_tests_llm_16_195_llm_16_194_rrrruuuugggg_test_amount = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_196 {
    use super::*;
    use crate::*;
    #[test]
    fn test_combine() {
        let _rug_st_tests_llm_16_196_rrrruuuugggg_test_combine = 0;
        let rug_fuzz_0 = b"hello";
        let rug_fuzz_1 = b"world";
        let mut crc1 = Crc::new();
        crc1.update(rug_fuzz_0);
        let mut crc2 = Crc::new();
        crc2.update(rug_fuzz_1);
        crc1.combine(&crc2);
        debug_assert_eq!(crc1.amount(), 10);
        debug_assert_eq!(crc1.sum(), 1098518336);
        let _rug_ed_tests_llm_16_196_rrrruuuugggg_test_combine = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_197 {
    use super::*;
    use crate::*;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_197_rrrruuuugggg_test_new = 0;
        let crc = Crc::new();
        debug_assert_eq!(crc.amount(), 0);
        debug_assert_eq!(crc.sum(), 0);
        let _rug_ed_tests_llm_16_197_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_199_llm_16_198 {
    use crate::crc::Crc;
    #[test]
    fn test_reset() {
        let _rug_st_tests_llm_16_199_llm_16_198_rrrruuuugggg_test_reset = 0;
        let rug_fuzz_0 = b"test";
        let mut crc = Crc::new();
        crc.update(rug_fuzz_0);
        crc.reset();
        debug_assert_eq!(crc.amount(), 0);
        debug_assert_eq!(crc.sum(), 0);
        let _rug_ed_tests_llm_16_199_llm_16_198_rrrruuuugggg_test_reset = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_200 {
    use super::*;
    use crate::*;
    use std::hash::Hasher;
    #[test]
    fn test_sum() {
        let _rug_st_tests_llm_16_200_rrrruuuugggg_test_sum = 0;
        let crc = Crc::new();
        let result = crc.sum();
        debug_assert_eq!(result, 0);
        let _rug_ed_tests_llm_16_200_rrrruuuugggg_test_sum = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_201 {
    use crate::crc::Crc;
    #[test]
    fn test_update() {
        let _rug_st_tests_llm_16_201_rrrruuuugggg_test_update = 0;
        let rug_fuzz_0 = b"Hello, World!";
        let mut crc = Crc::new();
        let data = rug_fuzz_0;
        crc.update(data);
        debug_assert_eq!(crc.amount(), data.len() as u32);
        let _rug_ed_tests_llm_16_201_rrrruuuugggg_test_update = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_202 {
    use crate::crc::CrcReader;
    use std::io::Read;
    #[test]
    fn test_crc() {
        let _rug_st_tests_llm_16_202_rrrruuuugggg_test_crc = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let data = vec![rug_fuzz_0, 1, 2, 3, 4, 5];
        let mut crc_reader = CrcReader::new(&data[..]);
        let mut buffer = [rug_fuzz_1; 3];
        let _ = crc_reader.read(&mut buffer);
        let crc = crc_reader.crc().sum();
        debug_assert_eq!(crc, 13151);
        let _rug_ed_tests_llm_16_202_rrrruuuugggg_test_crc = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_205 {
    use super::*;
    use crate::*;
    use std::io::Cursor;
    #[test]
    fn test_get_ref() {
        let _rug_st_tests_llm_16_205_rrrruuuugggg_test_get_ref = 0;
        let rug_fuzz_0 = b"Hello, world!";
        let data = rug_fuzz_0;
        let crc_reader = CrcReader::new(Cursor::new(data));
        let inner = crc_reader.get_ref();
        debug_assert_eq!(inner, & Cursor::new(data));
        let _rug_ed_tests_llm_16_205_rrrruuuugggg_test_get_ref = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_208 {
    use super::*;
    use crate::*;
    use std::io::Cursor;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_208_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let data: &[u8] = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        let reader = Cursor::new(data);
        let crc_reader = crc::CrcReader::new(reader);
        debug_assert_eq!(crc_reader.get_ref().position(), 0);
        debug_assert_eq!(crc_reader.crc().amount(), 0);
        debug_assert_eq!(crc_reader.crc().sum(), 0);
        let _rug_ed_tests_llm_16_208_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_209 {
    use super::*;
    use crate::*;
    use std::io::{BufRead, Read};
    #[test]
    fn test_reset() {
        let _rug_st_tests_llm_16_209_rrrruuuugggg_test_reset = 0;
        let rug_fuzz_0 = 0;
        let data = vec![rug_fuzz_0, 1, 2, 3, 4, 5];
        let mut crc_reader = CrcReader::new(data.as_slice());
        crc_reader.reset();
        debug_assert_eq!(crc_reader.crc().amount(), 0);
        debug_assert_eq!(crc_reader.crc().sum(), 0);
        let _rug_ed_tests_llm_16_209_rrrruuuugggg_test_reset = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_210 {
    use crate::crc::{Crc, CrcWriter};
    use std::io::Write;
    #[test]
    fn test_crc() {
        let _rug_st_tests_llm_16_210_rrrruuuugggg_test_crc = 0;
        let rug_fuzz_0 = b"test";
        let mut writer: CrcWriter<Vec<u8>> = CrcWriter::new(Vec::new());
        writer.write_all(rug_fuzz_0).unwrap();
        let result = writer.crc().sum();
        debug_assert_eq!(result, 0x6FAB5C3D);
        let _rug_ed_tests_llm_16_210_rrrruuuugggg_test_crc = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_211 {
    use super::*;
    use crate::*;
    use std::io::Write;
    #[test]
    fn test_get_mut() {
        let _rug_st_tests_llm_16_211_rrrruuuugggg_test_get_mut = 0;
        let rug_fuzz_0 = b"hello";
        let mut writer: CrcWriter<Vec<u8>> = CrcWriter::new(Vec::new());
        let inner_ref = writer.get_mut();
        debug_assert_eq!(inner_ref, & mut Vec::new());
        inner_ref.write_all(rug_fuzz_0).unwrap();
        debug_assert_eq!(inner_ref, & mut Vec::from(& b"hello"[..]));
        let _rug_ed_tests_llm_16_211_rrrruuuugggg_test_get_mut = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_212 {
    use super::*;
    use crate::*;
    use std::io::Write;
    #[test]
    fn test_get_ref() {
        let _rug_st_tests_llm_16_212_rrrruuuugggg_test_get_ref = 0;
        let writer: Vec<u8> = Vec::new();
        let crc_writer = CrcWriter::new(writer);
        let reference = crc_writer.get_ref();
        let _rug_ed_tests_llm_16_212_rrrruuuugggg_test_get_ref = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_215 {
    use super::*;
    use crate::*;
    use std::io::Cursor;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_215_rrrruuuugggg_test_new = 0;
        let cursor = Cursor::new(Vec::new());
        let crc_writer = crc::CrcWriter::<Cursor<Vec<u8>>>::new(cursor);
        let crc = crc_writer.crc();
        debug_assert_eq!(crc.amount(), 0);
        debug_assert_eq!(crc.sum(), 0);
        let inner = crc_writer.into_inner();
        debug_assert_eq!(inner.into_inner(), vec![]);
        let _rug_ed_tests_llm_16_215_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_216 {
    use crate::crc::{Crc, CrcWriter};
    use std::io::Write;
    #[test]
    fn test_reset() {
        let _rug_st_tests_llm_16_216_rrrruuuugggg_test_reset = 0;
        let rug_fuzz_0 = b"Hello, world!";
        let mut data: Vec<u8> = Vec::new();
        let mut crc_writer = CrcWriter::new(&mut data);
        crc_writer.write_all(rug_fuzz_0).unwrap();
        crc_writer.reset();
        let crc = crc_writer.crc();
        debug_assert_eq!(crc.amount(), 0);
        debug_assert_eq!(crc.sum(), 0);
        let _rug_ed_tests_llm_16_216_rrrruuuugggg_test_reset = 0;
    }
}
