use std::io;
use std::io::prelude::*;
#[cfg(feature = "tokio")]
use futures::Poll;
#[cfg(feature = "tokio")]
use tokio_io::{AsyncRead, AsyncWrite};
use super::bufread;
use super::{GzBuilder, GzHeader};
use crate::bufreader::BufReader;
use crate::Compression;
/// A gzip streaming encoder
///
/// This structure exposes a [`Read`] interface that will read uncompressed data
/// from the underlying reader and expose the compressed version as a [`Read`]
/// interface.
///
/// [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
///
/// # Examples
///
/// ```
/// use std::io::prelude::*;
/// use std::io;
/// use flate2::Compression;
/// use flate2::read::GzEncoder;
///
/// // Return a vector containing the GZ compressed version of hello world
///
/// fn gzencode_hello_world() -> io::Result<Vec<u8>> {
///     let mut ret_vec = [0;100];
///     let bytestring = b"hello world";
///     let mut gz = GzEncoder::new(&bytestring[..], Compression::fast());
///     let count = gz.read(&mut ret_vec)?;
///     Ok(ret_vec[0..count].to_vec())
/// }
/// ```
#[derive(Debug)]
pub struct GzEncoder<R> {
    inner: bufread::GzEncoder<BufReader<R>>,
}
pub fn gz_encoder<R: Read>(inner: bufread::GzEncoder<BufReader<R>>) -> GzEncoder<R> {
    GzEncoder { inner: inner }
}
impl<R: Read> GzEncoder<R> {
    /// Creates a new encoder which will use the given compression level.
    ///
    /// The encoder is not configured specially for the emitted header. For
    /// header configuration, see the `GzBuilder` type.
    ///
    /// The data read from the stream `r` will be compressed and available
    /// through the returned reader.
    pub fn new(r: R, level: Compression) -> GzEncoder<R> {
        GzBuilder::new().read(r, level)
    }
}
impl<R> GzEncoder<R> {
    /// Acquires a reference to the underlying reader.
    pub fn get_ref(&self) -> &R {
        self.inner.get_ref().get_ref()
    }
    /// Acquires a mutable reference to the underlying reader.
    ///
    /// Note that mutation of the reader may result in surprising results if
    /// this encoder is continued to be used.
    pub fn get_mut(&mut self) -> &mut R {
        self.inner.get_mut().get_mut()
    }
    /// Returns the underlying stream, consuming this encoder
    pub fn into_inner(self) -> R {
        self.inner.into_inner().into_inner()
    }
}
impl<R: Read> Read for GzEncoder<R> {
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        self.inner.read(into)
    }
}
impl<R: Read + Write> Write for GzEncoder<R> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.get_mut().write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.get_mut().flush()
    }
}
/// A gzip streaming decoder
///
/// This structure exposes a [`Read`] interface that will consume compressed
/// data from the underlying reader and emit uncompressed data.
///
/// [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
///
/// # Examples
///
/// ```
///
/// use std::io::prelude::*;
/// use std::io;
/// # use flate2::Compression;
/// # use flate2::write::GzEncoder;
/// use flate2::read::GzDecoder;
///
/// # fn main() {
/// #    let mut e = GzEncoder::new(Vec::new(), Compression::default());
/// #    e.write_all(b"Hello World").unwrap();
/// #    let bytes = e.finish().unwrap();
/// #    println!("{}", decode_reader(bytes).unwrap());
/// # }
/// #
/// // Uncompresses a Gz Encoded vector of bytes and returns a string or error
/// // Here &[u8] implements Read
///
/// fn decode_reader(bytes: Vec<u8>) -> io::Result<String> {
///    let mut gz = GzDecoder::new(&bytes[..]);
///    let mut s = String::new();
///    gz.read_to_string(&mut s)?;
///    Ok(s)
/// }
/// ```
#[derive(Debug)]
pub struct GzDecoder<R> {
    inner: bufread::GzDecoder<BufReader<R>>,
}
impl<R: Read> GzDecoder<R> {
    /// Creates a new decoder from the given reader, immediately parsing the
    /// gzip header.
    pub fn new(r: R) -> GzDecoder<R> {
        GzDecoder {
            inner: bufread::GzDecoder::new(BufReader::new(r)),
        }
    }
}
impl<R> GzDecoder<R> {
    /// Returns the header associated with this stream, if it was valid.
    pub fn header(&self) -> Option<&GzHeader> {
        self.inner.header()
    }
    /// Acquires a reference to the underlying reader.
    pub fn get_ref(&self) -> &R {
        self.inner.get_ref().get_ref()
    }
    /// Acquires a mutable reference to the underlying stream.
    ///
    /// Note that mutation of the stream may result in surprising results if
    /// this encoder is continued to be used.
    pub fn get_mut(&mut self) -> &mut R {
        self.inner.get_mut().get_mut()
    }
    /// Consumes this decoder, returning the underlying reader.
    pub fn into_inner(self) -> R {
        self.inner.into_inner().into_inner()
    }
}
impl<R: Read> Read for GzDecoder<R> {
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        self.inner.read(into)
    }
}
#[cfg(feature = "tokio")]
impl<R: AsyncRead> AsyncRead for GzDecoder<R> {}
impl<R: Read + Write> Write for GzDecoder<R> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.get_mut().write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.get_mut().flush()
    }
}
#[cfg(feature = "tokio")]
impl<R: AsyncWrite + AsyncRead> AsyncWrite for GzDecoder<R> {
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        self.get_mut().shutdown()
    }
}
/// A gzip streaming decoder that decodes all members of a multistream
///
/// A gzip member consists of a header, compressed data and a trailer. The [gzip
/// specification](https://tools.ietf.org/html/rfc1952), however, allows multiple
/// gzip members to be joined in a single stream.  `MultiGzDecoder` will
/// decode all consecutive members while `GzDecoder` will only decompress the
/// first gzip member. The multistream format is commonly used in bioinformatics,
/// for example when using the BGZF compressed data.
///
/// This structure exposes a [`Read`] interface that will consume all gzip members
/// from the underlying reader and emit uncompressed data.
///
/// [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
///
/// # Examples
///
/// ```
/// use std::io::prelude::*;
/// use std::io;
/// # use flate2::Compression;
/// # use flate2::write::GzEncoder;
/// use flate2::read::MultiGzDecoder;
///
/// # fn main() {
/// #    let mut e = GzEncoder::new(Vec::new(), Compression::default());
/// #    e.write_all(b"Hello World").unwrap();
/// #    let bytes = e.finish().unwrap();
/// #    println!("{}", decode_reader(bytes).unwrap());
/// # }
/// #
/// // Uncompresses a Gz Encoded vector of bytes and returns a string or error
/// // Here &[u8] implements Read
///
/// fn decode_reader(bytes: Vec<u8>) -> io::Result<String> {
///    let mut gz = MultiGzDecoder::new(&bytes[..]);
///    let mut s = String::new();
///    gz.read_to_string(&mut s)?;
///    Ok(s)
/// }
/// ```
#[derive(Debug)]
pub struct MultiGzDecoder<R> {
    inner: bufread::MultiGzDecoder<BufReader<R>>,
}
impl<R: Read> MultiGzDecoder<R> {
    /// Creates a new decoder from the given reader, immediately parsing the
    /// (first) gzip header. If the gzip stream contains multiple members all will
    /// be decoded.
    pub fn new(r: R) -> MultiGzDecoder<R> {
        MultiGzDecoder {
            inner: bufread::MultiGzDecoder::new(BufReader::new(r)),
        }
    }
}
impl<R> MultiGzDecoder<R> {
    /// Returns the current header associated with this stream, if it's valid.
    pub fn header(&self) -> Option<&GzHeader> {
        self.inner.header()
    }
    /// Acquires a reference to the underlying reader.
    pub fn get_ref(&self) -> &R {
        self.inner.get_ref().get_ref()
    }
    /// Acquires a mutable reference to the underlying stream.
    ///
    /// Note that mutation of the stream may result in surprising results if
    /// this encoder is continued to be used.
    pub fn get_mut(&mut self) -> &mut R {
        self.inner.get_mut().get_mut()
    }
    /// Consumes this decoder, returning the underlying reader.
    pub fn into_inner(self) -> R {
        self.inner.into_inner().into_inner()
    }
}
impl<R: Read> Read for MultiGzDecoder<R> {
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        self.inner.read(into)
    }
}
#[cfg(feature = "tokio")]
impl<R: AsyncRead> AsyncRead for MultiGzDecoder<R> {}
impl<R: Read + Write> Write for MultiGzDecoder<R> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.get_mut().write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.get_mut().flush()
    }
}
#[cfg(feature = "tokio")]
impl<R: AsyncWrite + AsyncRead> AsyncWrite for MultiGzDecoder<R> {
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        self.get_mut().shutdown()
    }
}
#[cfg(test)]
mod tests_llm_16_96 {
    use super::*;
    use crate::*;
    use std::io::{self, Read};
    #[test]
    fn test_read() {
        let _rug_st_tests_llm_16_96_rrrruuuugggg_test_read = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 4;
        let rug_fuzz_4 = 5;
        let mut input: &[u8] = &[
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
        ];
        let mut decoder = gz::read::MultiGzDecoder::new(&mut input);
        let mut output = vec![0u8; 3];
        debug_assert_eq!(decoder.read(& mut output).unwrap(), 3);
        debug_assert_eq!(output, & [1, 2, 3]);
        let _rug_ed_tests_llm_16_96_rrrruuuugggg_test_read = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_98 {
    use super::*;
    use crate::*;
    use std::io::{self, Write};
    #[test]
    fn test_flush() -> io::Result<()> {
        let mut decoder: GzDecoder<io::Cursor<Vec<u8>>> = GzDecoder::new(
            io::Cursor::new(Vec::new()),
        );
        decoder.write_all(b"Hello, World!")?;
        let result = decoder.flush()?;
        assert_eq!(result, ());
        Ok(())
    }
}
#[cfg(test)]
mod tests_llm_16_378_llm_16_377 {
    use super::*;
    use crate::*;
    use crate::{Compression, bufread::DeflateEncoder};
    #[test]
    fn test_into_inner() {
        let _rug_st_tests_llm_16_378_llm_16_377_rrrruuuugggg_test_into_inner = 0;
        let rug_fuzz_0 = "Hello World!";
        let input = rug_fuzz_0.as_bytes();
        let level = Compression::fast();
        let encoder = DeflateEncoder::new(input, level);
        let inner = encoder.into_inner();
        let expected = input.to_vec();
        debug_assert_eq!(inner, expected);
        let _rug_ed_tests_llm_16_378_llm_16_377_rrrruuuugggg_test_into_inner = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_379 {
    use super::*;
    use crate::*;
    use crate::Compression;
    use std::io::Read;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_379_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = b"hello world";
        let rug_fuzz_1 = 31;
        let data = rug_fuzz_0;
        let mut input = std::io::Cursor::new(data);
        let level = Compression::fast();
        let mut encoder = GzEncoder::new(&mut input, level);
        let mut output = Vec::new();
        encoder.read_to_end(&mut output).unwrap();
        let expected_output = vec![
            rug_fuzz_1, 139, 8, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 9, 143, 72, 205, 201, 201, 87, 8, 201, 71, 201, 45, 202, 73, 202,
            73, 10, 200, 73, 202, 73, 75, 10, 201, 47, 202, 75, 206, 73, 201, 75, 204,
            73, 201, 73, 10, 0, 180, 149, 150, 36, 32, 34, 240, 31, 78, 124, 0, 0, 0
        ];
        debug_assert_eq!(output, expected_output);
        let _rug_ed_tests_llm_16_379_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_rug_46 {
    use super::*;
    use crate::read::GzEncoder;
    use crate::Compression;
    use std::io::{Read, Result};
    #[test]
    fn test_gz_encoder_read() {
        let _rug_st_tests_rug_46_rrrruuuugggg_test_gz_encoder_read = 0;
        let rug_fuzz_0 = 0;
        let mut p0: GzEncoder<&[u8]> = GzEncoder::new(&[], Compression::default());
        let mut p1: [u8; 10] = [rug_fuzz_0; 10];
        let result: Result<usize> = <GzEncoder<&[u8]> as Read>::read(&mut p0, &mut p1);
        let _rug_ed_tests_rug_46_rrrruuuugggg_test_gz_encoder_read = 0;
    }
}
#[cfg(test)]
mod tests_rug_50 {
    use super::*;
    use crate::read::GzDecoder;
    use std::fs::File;
    use std::io::Read;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_50_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "path/to/file.gz";
        let file = File::open(rug_fuzz_0).unwrap();
        let mut p0: GzDecoder<File> = GzDecoder::new(file);
        p0.header();
        let _rug_ed_tests_rug_50_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_51 {
    use super::*;
    use std::fs::File;
    use std::io::Read;
    use crate::read::GzDecoder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_51_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "path/to/file.gz";
        let file = File::open(rug_fuzz_0).unwrap();
        let mut p0: GzDecoder<File> = GzDecoder::new(file);
        p0.get_ref();
        let _rug_ed_tests_rug_51_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_52 {
    use super::*;
    use std::io::Read;
    use crate::read::GzDecoder;
    #[test]
    fn test_get_mut() {
        let _rug_st_tests_rug_52_rrrruuuugggg_test_get_mut = 0;
        let rug_fuzz_0 = "path/to/file.gz";
        let file = std::fs::File::open(rug_fuzz_0).unwrap();
        let mut decoder: GzDecoder<std::fs::File> = GzDecoder::new(file);
        let mut p0: &mut GzDecoder<std::fs::File> = &mut decoder;
        p0.get_mut();
        let _rug_ed_tests_rug_52_rrrruuuugggg_test_get_mut = 0;
    }
}
#[cfg(test)]
mod tests_rug_53 {
    use super::*;
    use std::fs::File;
    use crate::read::GzDecoder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_53_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "path/to/file.gz";
        let file = File::open(rug_fuzz_0).unwrap();
        let mut p0: GzDecoder<File> = GzDecoder::new(file);
        p0.into_inner();
        let _rug_ed_tests_rug_53_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_54 {
    use super::*;
    use crate::read::GzDecoder;
    use std::fs::File;
    use std::io::{self, Read};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_54_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "path/to/file.gz";
        let rug_fuzz_1 = 0u8;
        let file = File::open(rug_fuzz_0).unwrap();
        let mut p0: GzDecoder<File> = GzDecoder::new(file);
        let p1: &mut [u8] = &mut [rug_fuzz_1; 10];
        p0.read(p1).unwrap();
        let _rug_ed_tests_rug_54_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_55 {
    use super::*;
    use std::fs::File;
    use std::io::Read;
    use crate::read::GzDecoder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_55_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "path/to/file.gz";
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 3;
        let rug_fuzz_4 = 4;
        let rug_fuzz_5 = 5;
        let file = File::open(rug_fuzz_0).unwrap();
        let mut p0: GzDecoder<File> = GzDecoder::new(file);
        let p1: &[u8] = &[rug_fuzz_1, rug_fuzz_2, rug_fuzz_3, rug_fuzz_4, rug_fuzz_5];
        <GzDecoder<File> as std::io::Write>::write(&mut p0, p1).unwrap();
        let _rug_ed_tests_rug_55_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_56 {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use crate::read::GzDecoder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_56_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "path/to/file.gz";
        let file = File::open(rug_fuzz_0).unwrap();
        let mut p0: GzDecoder<File> = GzDecoder::new(file);
        <GzDecoder<File> as std::io::Write>::flush(&mut p0).unwrap();
        let _rug_ed_tests_rug_56_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_61 {
    use super::*;
    use crate::read::GzDecoder;
    #[test]
    fn test_into_inner() {
        let _rug_st_tests_rug_61_rrrruuuugggg_test_into_inner = 0;
        let mut p0: GzDecoder<&[u8]> = GzDecoder::new(&[]);
        p0.into_inner();
        let _rug_ed_tests_rug_61_rrrruuuugggg_test_into_inner = 0;
    }
}
