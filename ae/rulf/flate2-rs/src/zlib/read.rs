use std::io;
use std::io::prelude::*;
#[cfg(feature = "tokio")]
use futures::Poll;
#[cfg(feature = "tokio")]
use tokio_io::{AsyncRead, AsyncWrite};
use super::bufread;
use crate::bufreader::BufReader;
/// A ZLIB encoder, or compressor.
///
/// This structure implements a [`Read`] interface and will read uncompressed
/// data from an underlying stream and emit a stream of compressed data.
///
/// [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
///
/// # Examples
///
/// ```
/// use std::io::prelude::*;
/// use flate2::Compression;
/// use flate2::read::ZlibEncoder;
/// use std::fs::File;
///
/// // Open example file and compress the contents using Read interface
///
/// # fn open_hello_world() -> std::io::Result<Vec<u8>> {
/// let f = File::open("examples/hello_world.txt")?;
/// let mut z = ZlibEncoder::new(f, Compression::fast());
/// let mut buffer = [0;50];
/// let byte_count = z.read(&mut buffer)?;
/// # Ok(buffer[0..byte_count].to_vec())
/// # }
/// ```
#[derive(Debug)]
pub struct ZlibEncoder<R> {
    inner: bufread::ZlibEncoder<BufReader<R>>,
}
impl<R: Read> ZlibEncoder<R> {
    /// Creates a new encoder which will read uncompressed data from the given
    /// stream and emit the compressed stream.
    pub fn new(r: R, level: crate::Compression) -> ZlibEncoder<R> {
        ZlibEncoder {
            inner: bufread::ZlibEncoder::new(BufReader::new(r), level),
        }
    }
}
impl<R> ZlibEncoder<R> {
    /// Resets the state of this encoder entirely, swapping out the input
    /// stream for another.
    ///
    /// This function will reset the internal state of this encoder and replace
    /// the input stream with the one provided, returning the previous input
    /// stream. Future data read from this encoder will be the compressed
    /// version of `r`'s data.
    ///
    /// Note that there may be currently buffered data when this function is
    /// called, and in that case the buffered data is discarded.
    pub fn reset(&mut self, r: R) -> R {
        super::bufread::reset_encoder_data(&mut self.inner);
        self.inner.get_mut().reset(r)
    }
    /// Acquires a reference to the underlying stream
    pub fn get_ref(&self) -> &R {
        self.inner.get_ref().get_ref()
    }
    /// Acquires a mutable reference to the underlying stream
    ///
    /// Note that mutation of the stream may result in surprising results if
    /// this encoder is continued to be used.
    pub fn get_mut(&mut self) -> &mut R {
        self.inner.get_mut().get_mut()
    }
    /// Consumes this encoder, returning the underlying reader.
    ///
    /// Note that there may be buffered bytes which are not re-acquired as part
    /// of this transition. It's recommended to only call this function after
    /// EOF has been reached.
    pub fn into_inner(self) -> R {
        self.inner.into_inner().into_inner()
    }
    /// Returns the number of bytes that have been read into this compressor.
    ///
    /// Note that not all bytes read from the underlying object may be accounted
    /// for, there may still be some active buffering.
    pub fn total_in(&self) -> u64 {
        self.inner.total_in()
    }
    /// Returns the number of bytes that the compressor has produced.
    ///
    /// Note that not all bytes may have been read yet, some may still be
    /// buffered.
    pub fn total_out(&self) -> u64 {
        self.inner.total_out()
    }
}
impl<R: Read> Read for ZlibEncoder<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}
#[cfg(feature = "tokio")]
impl<R: AsyncRead> AsyncRead for ZlibEncoder<R> {}
impl<W: Read + Write> Write for ZlibEncoder<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.get_mut().write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.get_mut().flush()
    }
}
#[cfg(feature = "tokio")]
impl<R: AsyncRead + AsyncWrite> AsyncWrite for ZlibEncoder<R> {
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        self.get_mut().shutdown()
    }
}
/// A ZLIB decoder, or decompressor.
///
/// This structure implements a [`Read`] interface and takes a stream of
/// compressed data as input, providing the decompressed data when read from.
///
/// [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
///
/// # Examples
///
/// ```
/// use std::io::prelude::*;
/// use std::io;
/// # use flate2::Compression;
/// # use flate2::write::ZlibEncoder;
/// use flate2::read::ZlibDecoder;
///
/// # fn main() {
/// # let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
/// # e.write_all(b"Hello World").unwrap();
/// # let bytes = e.finish().unwrap();
/// # println!("{}", decode_reader(bytes).unwrap());
/// # }
/// #
/// // Uncompresses a Zlib Encoded vector of bytes and returns a string or error
/// // Here &[u8] implements Read
///
/// fn decode_reader(bytes: Vec<u8>) -> io::Result<String> {
///     let mut z = ZlibDecoder::new(&bytes[..]);
///     let mut s = String::new();
///     z.read_to_string(&mut s)?;
///     Ok(s)
/// }
/// ```
#[derive(Debug)]
pub struct ZlibDecoder<R> {
    inner: bufread::ZlibDecoder<BufReader<R>>,
}
impl<R: Read> ZlibDecoder<R> {
    /// Creates a new decoder which will decompress data read from the given
    /// stream.
    pub fn new(r: R) -> ZlibDecoder<R> {
        ZlibDecoder::new_with_buf(r, vec![0; 32 * 1024])
    }
    /// Same as `new`, but the intermediate buffer for data is specified.
    ///
    /// Note that the specified buffer will only be used up to its current
    /// length. The buffer's capacity will also not grow over time.
    pub fn new_with_buf(r: R, buf: Vec<u8>) -> ZlibDecoder<R> {
        ZlibDecoder {
            inner: bufread::ZlibDecoder::new(BufReader::with_buf(buf, r)),
        }
    }
}
impl<R> ZlibDecoder<R> {
    /// Resets the state of this decoder entirely, swapping out the input
    /// stream for another.
    ///
    /// This will reset the internal state of this decoder and replace the
    /// input stream with the one provided, returning the previous input
    /// stream. Future data read from this decoder will be the decompressed
    /// version of `r`'s data.
    ///
    /// Note that there may be currently buffered data when this function is
    /// called, and in that case the buffered data is discarded.
    pub fn reset(&mut self, r: R) -> R {
        super::bufread::reset_decoder_data(&mut self.inner);
        self.inner.get_mut().reset(r)
    }
    /// Acquires a reference to the underlying stream
    pub fn get_ref(&self) -> &R {
        self.inner.get_ref().get_ref()
    }
    /// Acquires a mutable reference to the underlying stream
    ///
    /// Note that mutation of the stream may result in surprising results if
    /// this encoder is continued to be used.
    pub fn get_mut(&mut self) -> &mut R {
        self.inner.get_mut().get_mut()
    }
    /// Consumes this decoder, returning the underlying reader.
    ///
    /// Note that there may be buffered bytes which are not re-acquired as part
    /// of this transition. It's recommended to only call this function after
    /// EOF has been reached.
    pub fn into_inner(self) -> R {
        self.inner.into_inner().into_inner()
    }
    /// Returns the number of bytes that the decompressor has consumed.
    ///
    /// Note that this will likely be smaller than what the decompressor
    /// actually read from the underlying stream due to buffering.
    pub fn total_in(&self) -> u64 {
        self.inner.total_in()
    }
    /// Returns the number of bytes that the decompressor has produced.
    pub fn total_out(&self) -> u64 {
        self.inner.total_out()
    }
}
impl<R: Read> Read for ZlibDecoder<R> {
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        self.inner.read(into)
    }
}
#[cfg(feature = "tokio")]
impl<R: AsyncRead> AsyncRead for ZlibDecoder<R> {}
impl<R: Read + Write> Write for ZlibDecoder<R> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.get_mut().write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.get_mut().flush()
    }
}
#[cfg(feature = "tokio")]
impl<R: AsyncWrite + AsyncRead> AsyncWrite for ZlibDecoder<R> {
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        self.get_mut().shutdown()
    }
}
#[cfg(test)]
mod tests_llm_16_158_llm_16_157 {
    use super::*;
    use crate::*;
    use crate::bufread::ZlibDecoder;
    use std::io::{Read, Result};
    use std::io::Cursor;
    #[test]
    fn test_read() {
        let _rug_st_tests_llm_16_158_llm_16_157_rrrruuuugggg_test_read = 0;
        let rug_fuzz_0 = 0;
        let mut input: Vec<u8> = Vec::new();
        let mut output: Vec<u8> = Vec::new();
        let mut cursor = Cursor::new(input);
        let mut decoder = ZlibDecoder::new(&mut cursor);
        let mut buffer = [rug_fuzz_0; 1024];
        let result: Result<usize> = decoder.read(&mut buffer);
        debug_assert!(result.is_ok());
        let _rug_ed_tests_llm_16_158_llm_16_157_rrrruuuugggg_test_read = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_163 {
    use super::*;
    use crate::*;
    use std::io::{Error, ErrorKind, Read};
    #[test]
    fn test_read() {
        struct MockReader;
        impl Read for MockReader {
            fn read(&mut self, _: &mut [u8]) -> Result<usize, Error> {
                Err(Error::new(ErrorKind::Other, "Error"))
            }
        }
        let mut encoder = ZlibEncoder::new(MockReader, Compression::fast());
        let mut buf = [0u8; 100];
        let result = encoder.read(&mut buf);
    }
}
#[cfg(test)]
mod tests_llm_16_489 {
    use super::*;
    use crate::*;
    use std::io::Cursor;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_489_rrrruuuugggg_test_new = 0;
        let data = Cursor::new(vec![]);
        let decoder = ZlibDecoder::new(data);
        debug_assert_eq!(decoder.total_in(), 0);
        debug_assert_eq!(decoder.total_out(), 0);
        let _rug_ed_tests_llm_16_489_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_493_llm_16_492 {
    use super::*;
    use crate::*;
    use crate::*;
    use crate::read::ZlibDecoder;
    use crate::write::ZlibEncoder;
    use std::io::prelude::*;
    use std::io::Cursor;
    #[test]
    fn test_reset() {
        let _rug_st_tests_llm_16_493_llm_16_492_rrrruuuugggg_test_reset = 0;
        let rug_fuzz_0 = b"Hello, world!";
        let data = rug_fuzz_0;
        let mut encoder = ZlibEncoder::new(Vec::new(), crate::Compression::default());
        encoder.write_all(data).unwrap();
        let compressed_data = encoder.finish().unwrap();
        let mut decoder = ZlibDecoder::new(Cursor::new(compressed_data.clone()));
        let mut output = Vec::new();
        decoder.read_to_end(&mut output).unwrap();
        decoder.reset(Cursor::new(compressed_data));
        let mut reset_output = Vec::new();
        decoder.read_to_end(&mut reset_output).unwrap();
        debug_assert_eq!(output, reset_output);
        let _rug_ed_tests_llm_16_493_llm_16_492_rrrruuuugggg_test_reset = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_494 {
    use crate::read::ZlibDecoder;
    use crate::Compression;
    use std::io::prelude::*;
    #[test]
    fn test_total_in() {
        let _rug_st_tests_llm_16_494_rrrruuuugggg_test_total_in = 0;
        let rug_fuzz_0 = 0x78;
        let rug_fuzz_1 = 0x9C;
        let rug_fuzz_2 = 0x03;
        let rug_fuzz_3 = 0x00;
        let rug_fuzz_4 = 0x00;
        let rug_fuzz_5 = 0x00;
        let rug_fuzz_6 = 0x00;
        let rug_fuzz_7 = 0x01;
        let data: &[u8] = &[
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
            rug_fuzz_6,
            rug_fuzz_7,
        ];
        let mut decoder = ZlibDecoder::new(data);
        decoder.read_to_end(&mut Vec::new()).unwrap();
        debug_assert_eq!(decoder.total_in(), 8);
        let _rug_ed_tests_llm_16_494_rrrruuuugggg_test_total_in = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_496 {
    use crate::bufread::ZlibDecoder;
    use crate::Compression;
    use std::io::prelude::*;
    #[test]
    fn test_total_out() {
        let _rug_st_tests_llm_16_496_rrrruuuugggg_test_total_out = 0;
        let rug_fuzz_0 = b"\x78\x9c\xcb\x48\xcd\xc9\xc9\x07\x00\x06\x2c\x02\x46";
        let data = rug_fuzz_0;
        let mut decoder = ZlibDecoder::new(&data[..]);
        let mut buffer = Vec::new();
        decoder.read_to_end(&mut buffer).unwrap();
        debug_assert_eq!(decoder.total_out(), buffer.len() as u64);
        let _rug_ed_tests_llm_16_496_rrrruuuugggg_test_total_out = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_499 {
    use crate::bufread::ZlibEncoder;
    use crate::Compression;
    use std::io::prelude::*;
    use std::io::BufReader;
    use std::fs::File;
    #[test]
    fn test_get_ref() {
        let _rug_st_tests_llm_16_499_rrrruuuugggg_test_get_ref = 0;
        let rug_fuzz_0 = "examples/hello_world.txt";
        let file = File::open(rug_fuzz_0).unwrap();
        let reader = BufReader::new(file);
        let mut encoder = ZlibEncoder::new(reader, Compression::fast());
        let mut buffer = Vec::new();
        encoder.read_to_end(&mut buffer).unwrap();
        let reference = encoder.get_ref();
        let _rug_ed_tests_llm_16_499_rrrruuuugggg_test_get_ref = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_502 {
    use super::*;
    use crate::*;
    use crate::Compression;
    use std::fs::File;
    use std::io::BufReader;
    use std::io::Read;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_502_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = "examples/hello_world.txt";
        let file = File::open(rug_fuzz_0).unwrap();
        let reader = BufReader::new(file);
        let level = Compression::fast();
        let _ = ZlibEncoder::new(reader, level);
        let _rug_ed_tests_llm_16_502_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_508 {
    use crate::Compression;
    use crate::read::ZlibEncoder;
    use std::io::Read;
    use std::fs::File;
    #[test]
    fn test_total_out() {
        let _rug_st_tests_llm_16_508_rrrruuuugggg_test_total_out = 0;
        let rug_fuzz_0 = "examples/hello_world.txt";
        let rug_fuzz_1 = 0;
        let f = File::open(rug_fuzz_0).unwrap();
        let mut z = ZlibEncoder::new(f, Compression::fast());
        let mut buffer = [rug_fuzz_1; 50];
        let byte_count = z.read(&mut buffer).unwrap();
        let total_out = z.total_out();
        debug_assert_eq!(byte_count, total_out as usize);
        let _rug_ed_tests_llm_16_508_rrrruuuugggg_test_total_out = 0;
    }
}
#[cfg(test)]
mod tests_rug_148 {
    use super::*;
    use crate::read::{DeflateEncoder, ZlibEncoder};
    use std::io::Cursor;
    #[test]
    fn test_zlib_encoder_get_mut() {
        let _rug_st_tests_rug_148_rrrruuuugggg_test_zlib_encoder_get_mut = 0;
        let mut data = Cursor::new(vec![]);
        let mut p0: ZlibEncoder<Cursor<Vec<u8>>> = ZlibEncoder::new(
            data,
            Default::default(),
        );
        p0.get_mut();
        let _rug_ed_tests_rug_148_rrrruuuugggg_test_zlib_encoder_get_mut = 0;
    }
}
#[cfg(test)]
mod tests_rug_151 {
    use super::*;
    use crate::write::ZlibEncoder;
    use crate::Compression;
    use std::io::Write;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_151_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 4;
        let rug_fuzz_4 = 5;
        let mut p0: ZlibEncoder<Vec<u8>> = ZlibEncoder::new(
            Vec::new(),
            Compression::default(),
        );
        let p1: &[u8] = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3, rug_fuzz_4];
        <ZlibEncoder<Vec<u8>> as Write>::write(&mut p0, p1);
        let _rug_ed_tests_rug_151_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_154 {
    use super::*;
    use crate::read::ZlibDecoder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_154_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0u8;
        let mut v70: ZlibDecoder<&[u8]> = ZlibDecoder::new(&[rug_fuzz_0; 0]);
        <ZlibDecoder<&[u8]>>::get_ref(&v70);
        let _rug_ed_tests_rug_154_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_155 {
    use super::*;
    use crate::read::ZlibDecoder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_155_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0u8;
        let mut p0: ZlibDecoder<&[u8]> = ZlibDecoder::new(&[rug_fuzz_0; 0]);
        let result = p0.get_mut();
        let _rug_ed_tests_rug_155_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_156 {
    use super::*;
    use crate::read::ZlibDecoder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_156_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0u8;
        let mut p0: ZlibDecoder<&[u8]> = ZlibDecoder::new(&[rug_fuzz_0; 0]);
        <ZlibDecoder<&[u8]>>::into_inner(p0);
        let _rug_ed_tests_rug_156_rrrruuuugggg_test_rug = 0;
    }
}
