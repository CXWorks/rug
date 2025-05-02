use std::io;
use std::io::prelude::*;
use std::mem;
#[cfg(feature = "tokio")]
use futures::Poll;
#[cfg(feature = "tokio")]
use tokio_io::{AsyncRead, AsyncWrite};
use crate::zio;
use crate::{Compress, Decompress};
/// A ZLIB encoder, or compressor.
///
/// This structure consumes a [`BufRead`] interface, reading uncompressed data
/// from the underlying reader, and emitting compressed data.
///
/// [`BufRead`]: https://doc.rust-lang.org/std/io/trait.BufRead.html
///
/// # Examples
///
/// ```
/// use std::io::prelude::*;
/// use flate2::Compression;
/// use flate2::bufread::ZlibEncoder;
/// use std::fs::File;
/// use std::io::BufReader;
///
/// // Use a buffered file to compress contents into a Vec<u8>
///
/// # fn open_hello_world() -> std::io::Result<Vec<u8>> {
/// let f = File::open("examples/hello_world.txt")?;
/// let b = BufReader::new(f);
/// let mut z = ZlibEncoder::new(b, Compression::fast());
/// let mut buffer = Vec::new();
/// z.read_to_end(&mut buffer)?;
/// # Ok(buffer)
/// # }
/// ```
#[derive(Debug)]
pub struct ZlibEncoder<R> {
    obj: R,
    data: Compress,
}
impl<R: BufRead> ZlibEncoder<R> {
    /// Creates a new encoder which will read uncompressed data from the given
    /// stream and emit the compressed stream.
    pub fn new(r: R, level: crate::Compression) -> ZlibEncoder<R> {
        ZlibEncoder {
            obj: r,
            data: Compress::new(level, true),
        }
    }
}
pub fn reset_encoder_data<R>(zlib: &mut ZlibEncoder<R>) {
    zlib.data.reset()
}
impl<R> ZlibEncoder<R> {
    /// Resets the state of this encoder entirely, swapping out the input
    /// stream for another.
    ///
    /// This function will reset the internal state of this encoder and replace
    /// the input stream with the one provided, returning the previous input
    /// stream. Future data read from this encoder will be the compressed
    /// version of `r`'s data.
    pub fn reset(&mut self, r: R) -> R {
        reset_encoder_data(self);
        mem::replace(&mut self.obj, r)
    }
    /// Acquires a reference to the underlying reader
    pub fn get_ref(&self) -> &R {
        &self.obj
    }
    /// Acquires a mutable reference to the underlying stream
    ///
    /// Note that mutation of the stream may result in surprising results if
    /// this encoder is continued to be used.
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.obj
    }
    /// Consumes this encoder, returning the underlying reader.
    pub fn into_inner(self) -> R {
        self.obj
    }
    /// Returns the number of bytes that have been read into this compressor.
    ///
    /// Note that not all bytes read from the underlying object may be accounted
    /// for, there may still be some active buffering.
    pub fn total_in(&self) -> u64 {
        self.data.total_in()
    }
    /// Returns the number of bytes that the compressor has produced.
    ///
    /// Note that not all bytes may have been read yet, some may still be
    /// buffered.
    pub fn total_out(&self) -> u64 {
        self.data.total_out()
    }
}
impl<R: BufRead> Read for ZlibEncoder<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        zio::read(&mut self.obj, &mut self.data, buf)
    }
}
#[cfg(feature = "tokio")]
impl<R: AsyncRead + BufRead> AsyncRead for ZlibEncoder<R> {}
impl<R: BufRead + Write> Write for ZlibEncoder<R> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.get_mut().write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.get_mut().flush()
    }
}
#[cfg(feature = "tokio")]
impl<R: AsyncWrite + BufRead> AsyncWrite for ZlibEncoder<R> {
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        self.get_mut().shutdown()
    }
}
/// A ZLIB decoder, or decompressor.
///
/// This structure consumes a [`BufRead`] interface, reading compressed data
/// from the underlying reader, and emitting uncompressed data.
///
/// [`BufRead`]: https://doc.rust-lang.org/std/io/trait.BufRead.html
///
/// # Examples
///
/// ```
/// use std::io::prelude::*;
/// use std::io;
/// # use flate2::Compression;
/// # use flate2::write::ZlibEncoder;
/// use flate2::bufread::ZlibDecoder;
///
/// # fn main() {
/// # let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
/// # e.write_all(b"Hello World").unwrap();
/// # let bytes = e.finish().unwrap();
/// # println!("{}", decode_bufreader(bytes).unwrap());
/// # }
/// #
/// // Uncompresses a Zlib Encoded vector of bytes and returns a string or error
/// // Here &[u8] implements BufRead
///
/// fn decode_bufreader(bytes: Vec<u8>) -> io::Result<String> {
///     let mut z = ZlibDecoder::new(&bytes[..]);
///     let mut s = String::new();
///     z.read_to_string(&mut s)?;
///     Ok(s)
/// }
/// ```
#[derive(Debug)]
pub struct ZlibDecoder<R> {
    obj: R,
    data: Decompress,
}
impl<R: BufRead> ZlibDecoder<R> {
    /// Creates a new decoder which will decompress data read from the given
    /// stream.
    pub fn new(r: R) -> ZlibDecoder<R> {
        ZlibDecoder {
            obj: r,
            data: Decompress::new(true),
        }
    }
}
pub fn reset_decoder_data<R>(zlib: &mut ZlibDecoder<R>) {
    zlib.data = Decompress::new(true);
}
impl<R> ZlibDecoder<R> {
    /// Resets the state of this decoder entirely, swapping out the input
    /// stream for another.
    ///
    /// This will reset the internal state of this decoder and replace the
    /// input stream with the one provided, returning the previous input
    /// stream. Future data read from this decoder will be the decompressed
    /// version of `r`'s data.
    pub fn reset(&mut self, r: R) -> R {
        reset_decoder_data(self);
        mem::replace(&mut self.obj, r)
    }
    /// Acquires a reference to the underlying stream
    pub fn get_ref(&self) -> &R {
        &self.obj
    }
    /// Acquires a mutable reference to the underlying stream
    ///
    /// Note that mutation of the stream may result in surprising results if
    /// this encoder is continued to be used.
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.obj
    }
    /// Consumes this decoder, returning the underlying reader.
    pub fn into_inner(self) -> R {
        self.obj
    }
    /// Returns the number of bytes that the decompressor has consumed.
    ///
    /// Note that this will likely be smaller than what the decompressor
    /// actually read from the underlying stream due to buffering.
    pub fn total_in(&self) -> u64 {
        self.data.total_in()
    }
    /// Returns the number of bytes that the decompressor has produced.
    pub fn total_out(&self) -> u64 {
        self.data.total_out()
    }
}
impl<R: BufRead> Read for ZlibDecoder<R> {
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        zio::read(&mut self.obj, &mut self.data, into)
    }
}
#[cfg(feature = "tokio")]
impl<R: AsyncRead + BufRead> AsyncRead for ZlibDecoder<R> {}
impl<R: BufRead + Write> Write for ZlibDecoder<R> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.get_mut().write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.get_mut().flush()
    }
}
#[cfg(feature = "tokio")]
impl<R: AsyncWrite + BufRead> AsyncWrite for ZlibDecoder<R> {
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        self.get_mut().shutdown()
    }
}
#[cfg(test)]
mod tests_llm_16_147 {
    use super::*;
    use crate::*;
    use std::io::{Read, Result};
    use std::io::Cursor;
    #[test]
    fn test_zlib_decoder_read() -> Result<()> {
        let data: &[u8] = &[
            120,
            156,
            243,
            72,
            205,
            201,
            201,
            87,
            8,
            199,
            47,
            201,
            201,
            47,
            41,
            35,
            138,
            227,
            12,
            0,
            0,
            0,
        ];
        let mut decoder = ZlibDecoder::new(Cursor::new(data));
        let mut buf = [0u8; 20];
        let result = decoder.read(&mut buf)?;
        assert_eq!(result, 20);
        assert_eq!(buf, [0u8; 20]);
        Ok(())
    }
}
#[cfg(test)]
mod tests_llm_16_152 {
    use super::*;
    use crate::*;
    use crate::Compression;
    use crate::bufread::ZlibEncoder;
    use std::io::{BufRead, Read};
    #[test]
    fn test_read() {
        let _rug_st_tests_llm_16_152_rrrruuuugggg_test_read = 0;
        let rug_fuzz_0 = b"hello world";
        let input = rug_fuzz_0;
        let compression = Compression::fast();
        let bufread = std::io::Cursor::new(input);
        let mut encoder = ZlibEncoder::new(bufread, compression);
        let mut decoded = Vec::new();
        encoder.read_to_end(&mut decoded).unwrap();
        debug_assert_eq!(input, decoded.as_slice());
        let _rug_ed_tests_llm_16_152_rrrruuuugggg_test_read = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_156_llm_16_155 {
    use super::*;
    use crate::*;
    use std::io::Write;
    use std::io::Cursor;
    #[test]
    fn test_write() {
        let _rug_st_tests_llm_16_156_llm_16_155_rrrruuuugggg_test_write = 0;
        let rug_fuzz_0 = 1;
        let mut encoder: ZlibEncoder<Cursor<Vec<u8>>> = ZlibEncoder::new(
            Cursor::new(Vec::new()),
            Compression::default(),
        );
        let data = vec![rug_fuzz_0, 2, 3, 4, 5];
        let result = encoder.write(&data[..]).unwrap();
        debug_assert_eq!(result, data.len());
        let _rug_ed_tests_llm_16_156_llm_16_155_rrrruuuugggg_test_write = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_462 {
    use super::*;
    use crate::*;
    use std::io::BufReader;
    #[test]
    fn test_into_inner() {
        let _rug_st_tests_llm_16_462_rrrruuuugggg_test_into_inner = 0;
        let rug_fuzz_0 = 120;
        let data = vec![
            rug_fuzz_0, 156, 192, 64, 12, 0, 15, 0, 2, 0, 65, 0, 229, 170, 177, 18, 75,
            123, 222, 174, 151, 151, 174, 0, 0, 0
        ];
        let reader = BufReader::new(&data[..]);
        let mut decoder = ZlibDecoder::new(reader);
        let inner = decoder.into_inner();
        debug_assert_eq!(data, inner.into_inner());
        let _rug_ed_tests_llm_16_462_rrrruuuugggg_test_into_inner = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_463 {
    use super::*;
    use crate::*;
    use std::io::Cursor;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_463_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = 0x78;
        let input = Cursor::new(
            vec![rug_fuzz_0, 0x9c, 0x08, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        );
        let decoder = ZlibDecoder::new(input);
        debug_assert_eq!(decoder.total_in(), 0);
        debug_assert_eq!(decoder.total_out(), 0);
        let _rug_ed_tests_llm_16_463_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_464 {
    use std::io::{Read, Write, BufRead};
    use crate::bufread::ZlibDecoder;
    use crate::Compression;
    use crate::write::ZlibEncoder;
    #[test]
    fn test_zlib_decoder_reset() {
        let _rug_st_tests_llm_16_464_rrrruuuugggg_test_zlib_decoder_reset = 0;
        let rug_fuzz_0 = b"Hello World";
        let rug_fuzz_1 = b"Goodbye World";
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(rug_fuzz_0).unwrap();
        let compressed_data = encoder.finish().unwrap();
        let mut decoder = ZlibDecoder::new(&compressed_data[..]);
        let new_input = rug_fuzz_1;
        let previous_input = decoder.reset(new_input);
        let mut output = Vec::new();
        decoder.read_to_end(&mut output).unwrap();
        debug_assert_eq!(previous_input, compressed_data);
        debug_assert_eq!(output, b"Goodbye World");
        let _rug_ed_tests_llm_16_464_rrrruuuugggg_test_zlib_decoder_reset = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_465 {
    use super::*;
    use crate::*;
    use std::io::BufRead;
    #[test]
    fn test_total_in() {
        let _rug_st_tests_llm_16_465_rrrruuuugggg_test_total_in = 0;
        let rug_fuzz_0 = 0;
        let data = vec![rug_fuzz_0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let decoder = ZlibDecoder::new(std::io::Cursor::new(data));
        debug_assert_eq!(decoder.total_in(), 0);
        let _rug_ed_tests_llm_16_465_rrrruuuugggg_test_total_in = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_467_llm_16_466 {
    use super::*;
    use crate::*;
    use crate::bufread::ZlibDecoder;
    use std::io::Read;
    #[test]
    fn test_total_out() {
        let _rug_st_tests_llm_16_467_llm_16_466_rrrruuuugggg_test_total_out = 0;
        let rug_fuzz_0 = 0;
        let input = vec![];
        let mut decoder = ZlibDecoder::new(&input[..]);
        let expected_output = rug_fuzz_0;
        let mut output = Vec::new();
        decoder.read_to_end(&mut output).unwrap();
        debug_assert_eq!(decoder.total_out(), expected_output);
        let _rug_ed_tests_llm_16_467_llm_16_466_rrrruuuugggg_test_total_out = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_471 {
    use super::*;
    use crate::*;
    use crate::bufread::ZlibEncoder;
    use crate::Compression;
    use std::io::{BufRead, Read, Write};
    #[test]
    fn test_get_ref() {
        let _rug_st_tests_llm_16_471_rrrruuuugggg_test_get_ref = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 4;
        let data: &[u8] = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3];
        let reader = std::io::Cursor::new(data);
        let encoder = ZlibEncoder::new(reader.clone(), Compression::default());
        let result = encoder.get_ref();
        debug_assert_eq!(result, & reader);
        let _rug_ed_tests_llm_16_471_rrrruuuugggg_test_get_ref = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_472 {
    use super::*;
    use crate::*;
    use std::io::{BufRead, Read, Write};
    #[test]
    fn test_into_inner() {
        let _rug_st_tests_llm_16_472_rrrruuuugggg_test_into_inner = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 4;
        let rug_fuzz_4 = 5;
        let input: &[u8] = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3, rug_fuzz_4];
        let reader = std::io::Cursor::new(input);
        let level = crate::Compression::default();
        let mut encoder = ZlibEncoder::new(reader, level);
        let result = encoder.into_inner();
        debug_assert_eq!(result.into_inner(), input);
        let _rug_ed_tests_llm_16_472_rrrruuuugggg_test_into_inner = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_474 {
    use super::*;
    use crate::*;
    use std::io::prelude::*;
    use std::io::BufReader;
    use crate::Compression;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_474_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = "test";
        let rug_fuzz_1 = 6;
        let r = BufReader::new(rug_fuzz_0.as_bytes());
        let level = crate::Compression::new(rug_fuzz_1);
        let encoder = crate::zlib::bufread::ZlibEncoder::new(r, level);
        let _rug_ed_tests_llm_16_474_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_475 {
    use super::*;
    use crate::*;
    use crate::Compression;
    use crate::bufread::ZlibEncoder;
    use std::fs::File;
    use std::io::{BufRead, Read};
    use std::io::BufReader;
    #[test]
    fn test_reset() {
        let _rug_st_tests_llm_16_475_rrrruuuugggg_test_reset = 0;
        let rug_fuzz_0 = "examples/hello_world.txt";
        let rug_fuzz_1 = "examples/new_file.txt";
        let file = File::open(rug_fuzz_0).unwrap();
        let reader = BufReader::new(file);
        let mut encoder = ZlibEncoder::new(reader, Compression::fast());
        let new_file = File::open(rug_fuzz_1).unwrap();
        let new_reader = BufReader::new(new_file);
        let previous_reader = encoder.reset(new_reader);
        let _rug_ed_tests_llm_16_475_rrrruuuugggg_test_reset = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_478 {
    use super::*;
    use crate::*;
    use crate::Compression;
    use crate::bufread::ZlibEncoder;
    use std::io::Read;
    use std::io::BufReader;
    #[test]
    fn test_total_out() {
        let _rug_st_tests_llm_16_478_rrrruuuugggg_test_total_out = 0;
        let rug_fuzz_0 = b"Hello, world!";
        let data = rug_fuzz_0;
        let reader = BufReader::new(data.as_ref());
        let mut encoder = ZlibEncoder::new(reader, Compression::fast());
        let mut compressed_data = Vec::new();
        encoder.read_to_end(&mut compressed_data).unwrap();
        debug_assert_eq!(encoder.total_out(), compressed_data.len() as u64);
        let _rug_ed_tests_llm_16_478_rrrruuuugggg_test_total_out = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_480 {
    use super::*;
    use crate::*;
    use std::io::{BufReader, Read};
    #[test]
    fn test_reset_decoder_data() {
        let _rug_st_tests_llm_16_480_rrrruuuugggg_test_reset_decoder_data = 0;
        let mut input: &[u8] = &[];
        let mut zlib_decoder = ZlibDecoder::new(BufReader::new(&mut input));
        reset_decoder_data(&mut zlib_decoder);
        let _rug_ed_tests_llm_16_480_rrrruuuugggg_test_reset_decoder_data = 0;
    }
}
#[cfg(test)]
mod tests_rug_100 {
    use super::*;
    use crate::bufread::ZlibDecoder;
    use std::io::BufReader;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_100_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"your_data_here";
        let mut v54: ZlibDecoder<BufReader<&[u8]>> = ZlibDecoder::new(
            BufReader::new(&rug_fuzz_0[..]),
        );
        <ZlibDecoder<BufReader<&[u8]>>>::get_ref(&v54);
        let _rug_ed_tests_rug_100_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_101 {
    use super::*;
    use crate::bufread::ZlibDecoder;
    use std::io::BufReader;
    #[test]
    fn test_get_mut() {
        let _rug_st_tests_rug_101_rrrruuuugggg_test_get_mut = 0;
        let rug_fuzz_0 = b"your_data_here";
        let mut v54: ZlibDecoder<BufReader<&[u8]>> = ZlibDecoder::new(
            BufReader::new(&rug_fuzz_0[..]),
        );
        let p0 = &mut v54;
        <ZlibDecoder<BufReader<&[u8]>>>::get_mut(p0);
        let _rug_ed_tests_rug_101_rrrruuuugggg_test_get_mut = 0;
    }
}
