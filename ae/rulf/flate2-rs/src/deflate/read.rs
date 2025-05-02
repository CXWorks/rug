use std::io;
use std::io::prelude::*;
#[cfg(feature = "tokio")]
use futures::Poll;
#[cfg(feature = "tokio")]
use tokio_io::{AsyncRead, AsyncWrite};
use super::bufread;
use crate::bufreader::BufReader;
/// A DEFLATE encoder, or compressor.
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
/// use std::io;
/// use flate2::Compression;
/// use flate2::read::DeflateEncoder;
///
/// # fn main() {
/// #    println!("{:?}", deflateencoder_read_hello_world().unwrap());
/// # }
/// #
/// // Return a vector containing the Deflate compressed version of hello world
/// fn deflateencoder_read_hello_world() -> io::Result<Vec<u8>> {
///    let mut ret_vec = [0;100];
///    let c = b"hello world";
///    let mut deflater = DeflateEncoder::new(&c[..], Compression::fast());
///    let count = deflater.read(&mut ret_vec)?;
///    Ok(ret_vec[0..count].to_vec())
/// }
/// ```
#[derive(Debug)]
pub struct DeflateEncoder<R> {
    inner: bufread::DeflateEncoder<BufReader<R>>,
}
impl<R: Read> DeflateEncoder<R> {
    /// Creates a new encoder which will read uncompressed data from the given
    /// stream and emit the compressed stream.
    pub fn new(r: R, level: crate::Compression) -> DeflateEncoder<R> {
        DeflateEncoder {
            inner: bufread::DeflateEncoder::new(BufReader::new(r), level),
        }
    }
}
impl<R> DeflateEncoder<R> {
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
    /// Acquires a reference to the underlying reader
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
impl<R: Read> Read for DeflateEncoder<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}
#[cfg(feature = "tokio")]
impl<R: AsyncRead> AsyncRead for DeflateEncoder<R> {}
impl<W: Read + Write> Write for DeflateEncoder<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.get_mut().write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.get_mut().flush()
    }
}
#[cfg(feature = "tokio")]
impl<R: AsyncRead + AsyncWrite> AsyncWrite for DeflateEncoder<R> {
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        self.get_mut().shutdown()
    }
}
/// A DEFLATE decoder, or decompressor.
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
/// # use flate2::write::DeflateEncoder;
/// use flate2::read::DeflateDecoder;
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
    inner: bufread::DeflateDecoder<BufReader<R>>,
}
impl<R: Read> DeflateDecoder<R> {
    /// Creates a new decoder which will decompress data read from the given
    /// stream.
    pub fn new(r: R) -> DeflateDecoder<R> {
        DeflateDecoder::new_with_buf(r, vec![0; 32 * 1024])
    }
    /// Same as `new`, but the intermediate buffer for data is specified.
    ///
    /// Note that the capacity of the intermediate buffer is never increased,
    /// and it is recommended for it to be large.
    pub fn new_with_buf(r: R, buf: Vec<u8>) -> DeflateDecoder<R> {
        DeflateDecoder {
            inner: bufread::DeflateDecoder::new(BufReader::with_buf(buf, r)),
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
impl<R: Read> Read for DeflateDecoder<R> {
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        self.inner.read(into)
    }
}
#[cfg(feature = "tokio")]
impl<R: AsyncRead> AsyncRead for DeflateDecoder<R> {}
impl<W: Read + Write> Write for DeflateDecoder<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.get_mut().write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.get_mut().flush()
    }
}
#[cfg(feature = "tokio")]
impl<R: AsyncWrite + AsyncRead> AsyncWrite for DeflateDecoder<R> {
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        self.get_mut().shutdown()
    }
}
#[cfg(test)]
mod tests_llm_16_25 {
    use super::*;
    use crate::*;
    use std::io::Read;
    use std::io::Write;
    use crate::bufread::DeflateDecoder;
    #[test]
    fn test_read() {
        let _rug_st_tests_llm_16_25_rrrruuuugggg_test_read = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 4;
        let rug_fuzz_4 = 5;
        let rug_fuzz_5 = 0u8;
        let input: &[u8] = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3, rug_fuzz_4];
        let mut decoder = DeflateDecoder::new(input);
        let mut output = [rug_fuzz_5; 5];
        let result = decoder.read(&mut output);
        debug_assert_eq!(result.unwrap(), 5);
        debug_assert_eq!(output, [1, 2, 3, 4, 5]);
        let _rug_ed_tests_llm_16_25_rrrruuuugggg_test_read = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_31 {
    use super::*;
    use crate::*;
    use std::io::{self, Read};
    use crate::Compression;
    #[test]
    fn test_read() -> io::Result<()> {
        let buf = [1, 2, 3, 4, 5];
        let mut encoder = DeflateEncoder::new(&buf[..], Compression::fast());
        let mut output = [0; 5];
        let result = encoder.read(&mut output)?;
        assert_eq!(result, 5);
        assert_eq!(output, buf);
        Ok(())
    }
}
#[cfg(test)]
mod tests_llm_16_247 {
    use super::*;
    use crate::*;
    use std::io::BufRead;
    #[test]
    fn test_get_mut() {
        let _rug_st_tests_llm_16_247_rrrruuuugggg_test_get_mut = 0;
        let rug_fuzz_0 = 0u8;
        let mut data: &[u8] = &[rug_fuzz_0; 0];
        let mut decoder = DeflateDecoder::new_with_buf(&mut data, vec![0; 32 * 1024]);
        let _ = decoder.get_mut();
        let _rug_ed_tests_llm_16_247_rrrruuuugggg_test_get_mut = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_252 {
    use super::*;
    use crate::*;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_252_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let reader: &[u8] = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        let decoder = DeflateDecoder::<&[u8]>::new(reader);
        let _rug_ed_tests_llm_16_252_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_257 {
    use super::*;
    use crate::*;
    use std::io::{BufRead, Cursor};
    #[test]
    fn test_total_in() {
        let _rug_st_tests_llm_16_257_rrrruuuugggg_test_total_in = 0;
        let rug_fuzz_0 = b"hello world";
        let rug_fuzz_1 = 0;
        let data = rug_fuzz_0;
        let cursor = Cursor::new(data);
        let decoder = DeflateDecoder::new(cursor);
        debug_assert_eq!(rug_fuzz_1, decoder.total_in());
        let _rug_ed_tests_llm_16_257_rrrruuuugggg_test_total_in = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_271_llm_16_270 {
    use super::*;
    use crate::*;
    use crate::*;
    use crate::{bufread::DeflateEncoder, Compression};
    use std::io::{BufRead, Read};
    #[test]
    fn test_total_in() {
        let _rug_st_tests_llm_16_271_llm_16_270_rrrruuuugggg_test_total_in = 0;
        let rug_fuzz_0 = "hello world";
        let input = rug_fuzz_0.as_bytes();
        let mut encoder = DeflateEncoder::new(input, Compression::fast());
        let mut buffer = Vec::new();
        encoder.read_to_end(&mut buffer).unwrap();
        let total_in = encoder.total_in();
        debug_assert_eq!(total_in, 11);
        let _rug_ed_tests_llm_16_271_llm_16_270_rrrruuuugggg_test_total_in = 0;
    }
}
#[cfg(test)]
mod tests_rug_113 {
    use std::io::prelude::*;
    use crate::bufread::ZlibEncoder;
    use crate::Compression;
    use std::fs::File;
    use std::io::BufReader;
    use crate::read::DeflateEncoder;
    #[test]
    fn test_deflate_encoder_new() {
        let _rug_st_tests_rug_113_rrrruuuugggg_test_deflate_encoder_new = 0;
        let rug_fuzz_0 = "examples/hello_world.txt";
        let f = File::open(rug_fuzz_0).unwrap();
        let b = BufReader::new(f);
        let mut p0: ZlibEncoder<BufReader<File>> = ZlibEncoder::new(
            b,
            Compression::fast(),
        );
        let mut p1: Compression = Compression::best();
        DeflateEncoder::<ZlibEncoder<BufReader<File>>>::new(p0, p1);
        let _rug_ed_tests_rug_113_rrrruuuugggg_test_deflate_encoder_new = 0;
    }
}
#[cfg(test)]
mod tests_rug_114 {
    use super::*;
    use crate::Compression;
    use crate::read::DeflateEncoder;
    use std::fs::File;
    #[test]
    fn test_reset() {
        let _rug_st_tests_rug_114_rrrruuuugggg_test_reset = 0;
        let rug_fuzz_0 = "path/to/your/file.txt";
        let rug_fuzz_1 = "path/to/your/new_file.txt";
        let file = File::open(rug_fuzz_0).unwrap();
        let mut encoder = DeflateEncoder::new(file, Compression::default());
        let mut new_file = File::open(rug_fuzz_1).unwrap();
        encoder.reset(new_file);
        let _rug_ed_tests_rug_114_rrrruuuugggg_test_reset = 0;
    }
}
#[cfg(test)]
mod tests_rug_115 {
    use super::*;
    use crate::{Compression, read::DeflateEncoder};
    use std::fs::File;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_115_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "path/to/your/file.txt";
        let file = File::open(rug_fuzz_0).unwrap();
        let mut v58 = DeflateEncoder::new(file, Compression::default());
        <DeflateEncoder<File>>::get_ref(&v58);
        let _rug_ed_tests_rug_115_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_116 {
    use crate::Compression;
    use crate::read::DeflateEncoder;
    use std::fs::File;
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_116_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "path/to/your/file.txt";
        let file = File::open(rug_fuzz_0).unwrap();
        let mut p0 = DeflateEncoder::new(file, Compression::default());
        p0.get_mut();
        let _rug_ed_tests_rug_116_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_117 {
    use super::*;
    use std::fs::File;
    use crate::Compression;
    use crate::read::DeflateEncoder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_117_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "path/to/your/file.txt";
        let file = File::open(rug_fuzz_0).unwrap();
        let mut p0: DeflateEncoder<File> = DeflateEncoder::new(
            file,
            Compression::default(),
        );
        crate::deflate::read::DeflateEncoder::<File>::into_inner(p0);
        let _rug_ed_tests_rug_117_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_118 {
    use crate::Compression;
    use crate::read::DeflateEncoder;
    use std::fs::File;
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_118_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "path/to/your/file.txt";
        let file = File::open(rug_fuzz_0).unwrap();
        let mut p0 = DeflateEncoder::new(file, Compression::default());
        p0.total_out();
        let _rug_ed_tests_rug_118_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_119 {
    use super::*;
    use crate::write::DeflateEncoder;
    use crate::Compression;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_119_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let mut p0 = DeflateEncoder::new(Vec::new(), Compression::default());
        let p1: &[u8] = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        p0.write(p1);
        let _rug_ed_tests_rug_119_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_122 {
    use super::*;
    use crate::read::DeflateDecoder;
    use std::io::Read;
    #[test]
    fn test_reset() {
        let _rug_st_tests_rug_122_rrrruuuugggg_test_reset = 0;
        let r: &[u8] = &[];
        let mut p0: DeflateDecoder<&[u8]> = DeflateDecoder::new(r);
        let p1: &[u8] = &[];
        p0.reset(p1);
        let _rug_ed_tests_rug_122_rrrruuuugggg_test_reset = 0;
    }
}
#[cfg(test)]
mod tests_rug_123 {
    use super::*;
    use crate::read::DeflateDecoder;
    use std::io::Read;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_123_rrrruuuugggg_test_rug = 0;
        let r: &[u8] = &[];
        let mut v60 = DeflateDecoder::new(r);
        let p0 = &v60;
        crate::deflate::read::DeflateDecoder::<&'_ [u8]>::get_ref(p0);
        let _rug_ed_tests_rug_123_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_124 {
    use super::*;
    use crate::read::DeflateDecoder;
    use std::io::Read;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_124_rrrruuuugggg_test_rug = 0;
        let r: &[u8] = &[];
        let mut v60 = DeflateDecoder::new(r);
        let p0: DeflateDecoder<&[u8]> = v60;
        DeflateDecoder::<&[u8]>::into_inner(p0);
        let _rug_ed_tests_rug_124_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_125 {
    use super::*;
    use crate::read::DeflateDecoder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_125_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let r: &[u8] = &[];
        let mut v60 = DeflateDecoder::new(r);
        let p0 = &v60;
        debug_assert_eq!(rug_fuzz_0, p0.total_out());
        let _rug_ed_tests_rug_125_rrrruuuugggg_test_rug = 0;
    }
}
