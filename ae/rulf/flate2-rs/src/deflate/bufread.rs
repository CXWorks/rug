use std::io;
use std::io::prelude::*;
use std::mem;
#[cfg(feature = "tokio")]
use futures::Poll;
#[cfg(feature = "tokio")]
use tokio_io::{AsyncRead, AsyncWrite};
use crate::zio;
use crate::{Compress, Decompress};
/// A DEFLATE encoder, or compressor.
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
/// use std::io;
/// use flate2::Compression;
/// use flate2::bufread::DeflateEncoder;
/// use std::fs::File;
/// use std::io::BufReader;
///
/// # fn main() {
/// #    println!("{:?}", open_hello_world().unwrap());
/// # }
/// #
/// // Opens sample file, compresses the contents and returns a Vector
/// fn open_hello_world() -> io::Result<Vec<u8>> {
///    let f = File::open("examples/hello_world.txt")?;
///    let b = BufReader::new(f);
///    let mut deflater = DeflateEncoder::new(b, Compression::fast());
///    let mut buffer = Vec::new();
///    deflater.read_to_end(&mut buffer)?;
///    Ok(buffer)
/// }
/// ```
#[derive(Debug)]
pub struct DeflateEncoder<R> {
    obj: R,
    data: Compress,
}
impl<R: BufRead> DeflateEncoder<R> {
    /// Creates a new encoder which will read uncompressed data from the given
    /// stream and emit the compressed stream.
    pub fn new(r: R, level: crate::Compression) -> DeflateEncoder<R> {
        DeflateEncoder {
            obj: r,
            data: Compress::new(level, false),
        }
    }
}
pub fn reset_encoder_data<R>(zlib: &mut DeflateEncoder<R>) {
    zlib.data.reset();
}
impl<R> DeflateEncoder<R> {
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
impl<R: BufRead> Read for DeflateEncoder<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        zio::read(&mut self.obj, &mut self.data, buf)
    }
}
#[cfg(feature = "tokio")]
impl<R: AsyncRead + BufRead> AsyncRead for DeflateEncoder<R> {}
impl<W: BufRead + Write> Write for DeflateEncoder<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.get_mut().write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.get_mut().flush()
    }
}
#[cfg(feature = "tokio")]
impl<R: AsyncWrite + BufRead> AsyncWrite for DeflateEncoder<R> {
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        self.get_mut().shutdown()
    }
}
/// A DEFLATE decoder, or decompressor.
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
/// # use flate2::write::DeflateEncoder;
/// use flate2::bufread::DeflateDecoder;
///
/// # fn main() {
/// #    let mut e = DeflateEncoder::new(Vec::new(), Compression::default());
/// #    e.write_all(b"Hello World").unwrap();
/// #    let bytes = e.finish().unwrap();
/// #    println!("{}", decode_reader(bytes).unwrap());
/// # }
/// // Uncompresses a Deflate Encoded vector of bytes and returns a string or error
/// // Here &[u8] implements Read
/// fn decode_reader(bytes: Vec<u8>) -> io::Result<String> {
///    let mut deflater = DeflateDecoder::new(&bytes[..]);
///    let mut s = String::new();
///    deflater.read_to_string(&mut s)?;
///    Ok(s)
/// }
/// ```
#[derive(Debug)]
pub struct DeflateDecoder<R> {
    obj: R,
    data: Decompress,
}
pub fn reset_decoder_data<R>(zlib: &mut DeflateDecoder<R>) {
    zlib.data = Decompress::new(false);
}
impl<R: BufRead> DeflateDecoder<R> {
    /// Creates a new decoder which will decompress data read from the given
    /// stream.
    pub fn new(r: R) -> DeflateDecoder<R> {
        DeflateDecoder {
            obj: r,
            data: Decompress::new(false),
        }
    }
}
impl<R> DeflateDecoder<R> {
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
    /// Resets the state of this decoder's data
    ///
    /// This will reset the internal state of this decoder. It will continue
    /// reading from the same stream.
    pub fn reset_data(&mut self) {
        reset_decoder_data(self);
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
impl<R: BufRead> Read for DeflateDecoder<R> {
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        zio::read(&mut self.obj, &mut self.data, into)
    }
}
#[cfg(feature = "tokio")]
impl<R: AsyncRead + BufRead> AsyncRead for DeflateDecoder<R> {}
impl<W: BufRead + Write> Write for DeflateDecoder<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.get_mut().write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.get_mut().flush()
    }
}
#[cfg(feature = "tokio")]
impl<R: AsyncWrite + BufRead> AsyncWrite for DeflateDecoder<R> {
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        self.get_mut().shutdown()
    }
}
#[cfg(test)]
mod tests_llm_16_15_llm_16_14 {
    use super::*;
    use crate::*;
    use std::io::{self, Read};
    use crate::bufread::DeflateDecoder;
    #[test]
    fn test_read() {
        let _rug_st_tests_llm_16_15_llm_16_14_rrrruuuugggg_test_read = 0;
        let rug_fuzz_0 = 0x78;
        let rug_fuzz_1 = 0x9c;
        let rug_fuzz_2 = 0x01;
        let rug_fuzz_3 = 0x00;
        let rug_fuzz_4 = 0x00;
        let rug_fuzz_5 = 0xff;
        let rug_fuzz_6 = 0xff;
        let rug_fuzz_7 = 0xff;
        let rug_fuzz_8 = 0xff;
        let rug_fuzz_9 = 0x00;
        let rug_fuzz_10 = 0x00;
        let rug_fuzz_11 = 0;
        let input: &[u8] = &[
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
            rug_fuzz_6,
            rug_fuzz_7,
            rug_fuzz_8,
            rug_fuzz_9,
            rug_fuzz_10,
        ];
        let mut decoder = DeflateDecoder::new(input);
        let mut output = [rug_fuzz_11; 8];
        let result = decoder.read(&mut output);
        debug_assert_eq!(result.unwrap(), 8);
        debug_assert_eq!(output, [0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00]);
        let _rug_ed_tests_llm_16_15_llm_16_14_rrrruuuugggg_test_read = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_20 {
    use super::*;
    use crate::*;
    use std::io::Read;
    use crate::{Compression, bufread::DeflateEncoder};
    #[test]
    fn test_read() {
        let _rug_st_tests_llm_16_20_rrrruuuugggg_test_read = 0;
        let rug_fuzz_0 = b"Hello, world!";
        let input = rug_fuzz_0;
        let mut compressed = Vec::new();
        let mut encoder = DeflateEncoder::new(&input[..], Compression::default());
        encoder.read_to_end(&mut compressed).unwrap();
        let mut decompressed = Vec::new();
        let mut decoder = DeflateDecoder::new(&compressed[..]);
        decoder.read_to_end(&mut decompressed).unwrap();
        debug_assert_eq!(input, decompressed.as_slice());
        let _rug_ed_tests_llm_16_20_rrrruuuugggg_test_read = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_221 {
    use super::*;
    use crate::*;
    use std::io::BufReader;
    use crate::Compression;
    use crate::write::DeflateEncoder;
    #[test]
    fn test_into_inner() {
        let _rug_st_tests_llm_16_221_rrrruuuugggg_test_into_inner = 0;
        let rug_fuzz_0 = b"Hello World";
        let mut e = DeflateEncoder::new(Vec::new(), Compression::default());
        e.write_all(rug_fuzz_0).unwrap();
        let bytes = e.finish().unwrap();
        let reader = BufReader::new(&bytes[..]);
        let mut decoder = DeflateDecoder::new(reader);
        let inner = decoder.into_inner();
        let _rug_ed_tests_llm_16_221_rrrruuuugggg_test_into_inner = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_223 {
    use crate::bufread::DeflateDecoder;
    use crate::Compression;
    use std::io::prelude::*;
    use std::io;
    #[test]
    fn test_deflate_decoder_new() {
        let _rug_st_tests_llm_16_223_rrrruuuugggg_test_deflate_decoder_new = 0;
        let rug_fuzz_0 = 1;
        let input = vec![rug_fuzz_0, 2, 3, 4, 5, 6, 7, 8];
        let mut decoder = DeflateDecoder::new(input.as_slice());
        let mut output = Vec::new();
        decoder.read_to_end(&mut output).unwrap();
        debug_assert_eq!(input, output);
        let _rug_ed_tests_llm_16_223_rrrruuuugggg_test_deflate_decoder_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_227 {
    use super::*;
    use crate::*;
    use std::io::prelude::*;
    use std::io;
    use crate::bufread::DeflateDecoder;
    #[test]
    fn test_reset_data() {
        let _rug_st_tests_llm_16_227_rrrruuuugggg_test_reset_data = 0;
        let rug_fuzz_0 = b"Hello World";
        let mut deflater = DeflateDecoder::new(io::Cursor::new(Vec::new()));
        deflater.write_all(rug_fuzz_0).unwrap();
        deflater.reset_data();
        let mut s = String::new();
        deflater.read_to_string(&mut s).unwrap();
        debug_assert_eq!(s, "Hello World");
        let _rug_ed_tests_llm_16_227_rrrruuuugggg_test_reset_data = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_230 {
    use crate::bufread::DeflateDecoder;
    use crate::Compression;
    use crate::write::DeflateEncoder;
    use std::io::prelude::*;
    use std::io;
    #[test]
    fn test_total_out() {
        let _rug_st_tests_llm_16_230_rrrruuuugggg_test_total_out = 0;
        let rug_fuzz_0 = b"Hello World";
        let mut e = DeflateEncoder::new(Vec::new(), Compression::default());
        e.write_all(rug_fuzz_0).unwrap();
        let bytes = e.finish().unwrap();
        let mut deflater = DeflateDecoder::new(&bytes[..]);
        let mut s = String::new();
        deflater.read_to_string(&mut s).unwrap();
        debug_assert_eq!(deflater.total_out(), s.len() as u64);
        let _rug_ed_tests_llm_16_230_rrrruuuugggg_test_total_out = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_233 {
    use crate::Compression;
    use crate::bufread::DeflateEncoder;
    use std::io::BufRead;
    use std::io::Read;
    use std::fs::File;
    use std::io::BufReader;
    use crate::Compress;
    #[test]
    fn test_get_ref() {
        let _rug_st_tests_llm_16_233_rrrruuuugggg_test_get_ref = 0;
        let rug_fuzz_0 = "examples/hello_world.txt";
        let f = File::open(rug_fuzz_0).unwrap();
        let b = BufReader::new(f);
        let deflater = DeflateEncoder::new(b, Compression::fast());
        let reader = deflater.get_ref();
        let _rug_ed_tests_llm_16_233_rrrruuuugggg_test_get_ref = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_237 {
    use crate::bufread::DeflateEncoder;
    use crate::Compression;
    use std::io::Read;
    #[test]
    fn test_deflate_encoder_new() {
        let _rug_st_tests_llm_16_237_rrrruuuugggg_test_deflate_encoder_new = 0;
        let rug_fuzz_0 = 1;
        let data = vec![rug_fuzz_0, 2, 3, 4, 5];
        let level = Compression::fast();
        let reader = std::io::Cursor::new(data);
        let mut encoder = DeflateEncoder::new(reader, level);
        let mut buf = vec![0; 10];
        encoder.read_exact(&mut buf).unwrap();
        debug_assert_eq!(buf, vec![120, 156, 1, 2, 3, 4, 5, 0, 0, 0]);
        let _rug_ed_tests_llm_16_237_rrrruuuugggg_test_deflate_encoder_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_240 {
    use super::*;
    use crate::*;
    use crate::{Compression, bufread::DeflateEncoder};
    use std::io::{BufRead, Cursor};
    #[test]
    fn test_total_in() {
        let _rug_st_tests_llm_16_240_rrrruuuugggg_test_total_in = 0;
        let rug_fuzz_0 = b"Hello, world!";
        let data = rug_fuzz_0;
        let cursor = Cursor::new(data);
        let deflater = DeflateEncoder::new(cursor, Compression::fast());
        let total_in = deflater.total_in();
        debug_assert_eq!(total_in, data.len() as u64);
        let _rug_ed_tests_llm_16_240_rrrruuuugggg_test_total_in = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_241 {
    use crate::bufread::DeflateEncoder;
    use crate::Compression;
    use std::fs::File;
    use std::io::BufReader;
    use std::io::Read;
    use std::io;
    #[test]
    fn total_out_returns_correct_value() {
        let _rug_st_tests_llm_16_241_rrrruuuugggg_total_out_returns_correct_value = 0;
        let rug_fuzz_0 = "examples/hello_world.txt";
        let f = File::open(rug_fuzz_0).unwrap();
        let b = BufReader::new(f);
        let mut deflater = DeflateEncoder::new(b, Compression::fast());
        let mut buffer = Vec::new();
        deflater.read_to_end(&mut buffer).unwrap();
        let total_out = deflater.total_out();
        debug_assert_eq!(total_out, buffer.len() as u64);
        let _rug_ed_tests_llm_16_241_rrrruuuugggg_total_out_returns_correct_value = 0;
    }
}
#[cfg(test)]
mod tests_rug_1 {
    use super::*;
    use std::io::Read;
    use crate::Compression;
    use crate::bufread::{DeflateEncoder, ZlibEncoder};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_1_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"Hello, world!";
        let data = rug_fuzz_0;
        let reader = &data[..];
        let mut p0: DeflateEncoder<&[u8]> = DeflateEncoder::new(
            reader,
            Compression::default(),
        );
        crate::deflate::bufread::reset_encoder_data(&mut p0);
        let _rug_ed_tests_rug_1_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_2 {
    use super::*;
    use crate::bufread::DeflateDecoder;
    use std::io::Cursor;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_2_rrrruuuugggg_test_rug = 0;
        let mut p0: DeflateDecoder<Cursor<Vec<u8>>> = DeflateDecoder::new(
            Cursor::new(vec![]),
        );
        crate::deflate::bufread::reset_decoder_data(&mut p0);
        let _rug_ed_tests_rug_2_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_4 {
    use super::*;
    use crate::bufread::DeflateEncoder;
    use crate::Compression;
    #[test]
    fn test_get_mut() {
        let _rug_st_tests_rug_4_rrrruuuugggg_test_get_mut = 0;
        let rug_fuzz_0 = b"Hello, world!";
        let data = rug_fuzz_0;
        let reader = &data[..];
        let mut v1 = DeflateEncoder::new(reader, Compression::default());
        let p0: &mut DeflateEncoder<_> = &mut v1;
        DeflateEncoder::<_>::get_mut(p0);
        let _rug_ed_tests_rug_4_rrrruuuugggg_test_get_mut = 0;
    }
}
#[cfg(test)]
mod tests_rug_5 {
    use super::*;
    use std::io::Read;
    use crate::Compression;
    use crate::bufread::{DeflateEncoder, ZlibEncoder};
    #[test]
    fn test_into_inner() {
        let _rug_st_tests_rug_5_rrrruuuugggg_test_into_inner = 0;
        let rug_fuzz_0 = b"Hello, world!";
        let data = rug_fuzz_0;
        let reader = &data[..];
        let mut p0: DeflateEncoder<&[u8]> = DeflateEncoder::new(
            reader,
            Compression::default(),
        );
        p0.into_inner();
        let _rug_ed_tests_rug_5_rrrruuuugggg_test_into_inner = 0;
    }
}
#[cfg(test)]
mod tests_rug_9 {
    use super::*;
    use crate::bufread::DeflateDecoder;
    use std::io::Cursor;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_9_rrrruuuugggg_test_rug = 0;
        let mut p0: DeflateDecoder<Cursor<Vec<u8>>> = DeflateDecoder::new(
            Cursor::new(vec![]),
        );
        <DeflateDecoder<Cursor<Vec<u8>>>>::get_ref(&p0);
        let _rug_ed_tests_rug_9_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_10 {
    use super::*;
    use crate::bufread::DeflateDecoder;
    use std::io::Cursor;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_10_rrrruuuugggg_test_rug = 0;
        let mut p0: DeflateDecoder<Cursor<Vec<u8>>> = DeflateDecoder::new(
            Cursor::new(vec![]),
        );
        <DeflateDecoder<Cursor<Vec<u8>>>>::get_mut(&mut p0);
        let _rug_ed_tests_rug_10_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_11 {
    use super::*;
    use crate::bufread::DeflateDecoder;
    use std::io::Cursor;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_11_rrrruuuugggg_test_rug = 0;
        let mut p0: DeflateDecoder<Cursor<Vec<u8>>> = DeflateDecoder::new(
            Cursor::new(vec![]),
        );
        <DeflateDecoder<Cursor<Vec<u8>>>>::total_in(&p0);
        let _rug_ed_tests_rug_11_rrrruuuugggg_test_rug = 0;
    }
}
