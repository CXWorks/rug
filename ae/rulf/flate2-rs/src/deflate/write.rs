use std::io;
use std::io::prelude::*;
#[cfg(feature = "tokio")]
use futures::Poll;
#[cfg(feature = "tokio")]
use tokio_io::{AsyncRead, AsyncWrite};
use crate::zio;
use crate::{Compress, Decompress};
/// A DEFLATE encoder, or compressor.
///
/// This structure implements a [`Write`] interface and takes a stream of
/// uncompressed data, writing the compressed data to the wrapped writer.
///
/// [`Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
///
/// # Examples
///
/// ```
/// use std::io::prelude::*;
/// use flate2::Compression;
/// use flate2::write::DeflateEncoder;
///
/// // Vec<u8> implements Write to print the compressed bytes of sample string
/// # fn main() {
///
/// let mut e = DeflateEncoder::new(Vec::new(), Compression::default());
/// e.write_all(b"Hello World").unwrap();
/// println!("{:?}", e.finish().unwrap());
/// # }
/// ```
#[derive(Debug)]
pub struct DeflateEncoder<W: Write> {
    inner: zio::Writer<W, Compress>,
}
impl<W: Write> DeflateEncoder<W> {
    /// Creates a new encoder which will write compressed data to the stream
    /// given at the given compression level.
    ///
    /// When this encoder is dropped or unwrapped the final pieces of data will
    /// be flushed.
    pub fn new(w: W, level: crate::Compression) -> DeflateEncoder<W> {
        DeflateEncoder {
            inner: zio::Writer::new(w, Compress::new(level, false)),
        }
    }
    /// Acquires a reference to the underlying writer.
    pub fn get_ref(&self) -> &W {
        self.inner.get_ref()
    }
    /// Acquires a mutable reference to the underlying writer.
    ///
    /// Note that mutating the output/input state of the stream may corrupt this
    /// object, so care must be taken when using this method.
    pub fn get_mut(&mut self) -> &mut W {
        self.inner.get_mut()
    }
    /// Resets the state of this encoder entirely, swapping out the output
    /// stream for another.
    ///
    /// This function will finish encoding the current stream into the current
    /// output stream before swapping out the two output streams. If the stream
    /// cannot be finished an error is returned.
    ///
    /// After the current stream has been finished, this will reset the internal
    /// state of this encoder and replace the output stream with the one
    /// provided, returning the previous output stream. Future data written to
    /// this encoder will be the compressed into the stream `w` provided.
    ///
    /// # Errors
    ///
    /// This function will perform I/O to complete this stream, and any I/O
    /// errors which occur will be returned from this function.
    pub fn reset(&mut self, w: W) -> io::Result<W> {
        self.inner.finish()?;
        self.inner.data.reset();
        Ok(self.inner.replace(w))
    }
    /// Attempt to finish this output stream, writing out final chunks of data.
    ///
    /// Note that this function can only be used once data has finished being
    /// written to the output stream. After this function is called then further
    /// calls to `write` may result in a panic.
    ///
    /// # Panics
    ///
    /// Attempts to write data to this stream may result in a panic after this
    /// function is called.
    ///
    /// # Errors
    ///
    /// This function will perform I/O to complete this stream, and any I/O
    /// errors which occur will be returned from this function.
    pub fn try_finish(&mut self) -> io::Result<()> {
        self.inner.finish()
    }
    /// Consumes this encoder, flushing the output stream.
    ///
    /// This will flush the underlying data stream, close off the compressed
    /// stream and, if successful, return the contained writer.
    ///
    /// Note that this function may not be suitable to call in a situation where
    /// the underlying stream is an asynchronous I/O stream. To finish a stream
    /// the `try_finish` (or `shutdown`) method should be used instead. To
    /// re-acquire ownership of a stream it is safe to call this method after
    /// `try_finish` or `shutdown` has returned `Ok`.
    ///
    /// # Errors
    ///
    /// This function will perform I/O to complete this stream, and any I/O
    /// errors which occur will be returned from this function.
    pub fn finish(mut self) -> io::Result<W> {
        self.inner.finish()?;
        Ok(self.inner.take_inner())
    }
    /// Consumes this encoder, flushing the output stream.
    ///
    /// This will flush the underlying data stream and then return the contained
    /// writer if the flush succeeded.
    /// The compressed stream will not closed but only flushed. This
    /// means that obtained byte array can by extended by another deflated
    /// stream. To close the stream add the two bytes 0x3 and 0x0.
    ///
    /// # Errors
    ///
    /// This function will perform I/O to complete this stream, and any I/O
    /// errors which occur will be returned from this function.
    pub fn flush_finish(mut self) -> io::Result<W> {
        self.inner.flush()?;
        Ok(self.inner.take_inner())
    }
    /// Returns the number of bytes that have been written to this compresor.
    ///
    /// Note that not all bytes written to this object may be accounted for,
    /// there may still be some active buffering.
    pub fn total_in(&self) -> u64 {
        self.inner.data.total_in()
    }
    /// Returns the number of bytes that the compressor has produced.
    ///
    /// Note that not all bytes may have been written yet, some may still be
    /// buffered.
    pub fn total_out(&self) -> u64 {
        self.inner.data.total_out()
    }
}
impl<W: Write> Write for DeflateEncoder<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
#[cfg(feature = "tokio")]
impl<W: AsyncWrite> AsyncWrite for DeflateEncoder<W> {
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        self.inner.finish()?;
        self.inner.get_mut().shutdown()
    }
}
impl<W: Read + Write> Read for DeflateEncoder<W> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.get_mut().read(buf)
    }
}
#[cfg(feature = "tokio")]
impl<W: AsyncRead + AsyncWrite> AsyncRead for DeflateEncoder<W> {}
/// A DEFLATE decoder, or decompressor.
///
/// This structure implements a [`Write`] and will emit a stream of decompressed
/// data when fed a stream of compressed data.
///
/// [`Write`]: https://doc.rust-lang.org/std/io/trait.Read.html
///
/// # Examples
///
/// ```
/// use std::io::prelude::*;
/// use std::io;
/// # use flate2::Compression;
/// # use flate2::write::DeflateEncoder;
/// use flate2::write::DeflateDecoder;
///
/// # fn main() {
/// #    let mut e = DeflateEncoder::new(Vec::new(), Compression::default());
/// #    e.write_all(b"Hello World").unwrap();
/// #    let bytes = e.finish().unwrap();
/// #    println!("{}", decode_writer(bytes).unwrap());
/// # }
/// // Uncompresses a Deflate Encoded vector of bytes and returns a string or error
/// // Here Vec<u8> implements Write
/// fn decode_writer(bytes: Vec<u8>) -> io::Result<String> {
///    let mut writer = Vec::new();
///    let mut deflater = DeflateDecoder::new(writer);
///    deflater.write_all(&bytes[..])?;
///    writer = deflater.finish()?;
///    let return_string = String::from_utf8(writer).expect("String parsing error");
///    Ok(return_string)
/// }
/// ```
#[derive(Debug)]
pub struct DeflateDecoder<W: Write> {
    inner: zio::Writer<W, Decompress>,
}
impl<W: Write> DeflateDecoder<W> {
    /// Creates a new decoder which will write uncompressed data to the stream.
    ///
    /// When this encoder is dropped or unwrapped the final pieces of data will
    /// be flushed.
    pub fn new(w: W) -> DeflateDecoder<W> {
        DeflateDecoder {
            inner: zio::Writer::new(w, Decompress::new(false)),
        }
    }
    /// Acquires a reference to the underlying writer.
    pub fn get_ref(&self) -> &W {
        self.inner.get_ref()
    }
    /// Acquires a mutable reference to the underlying writer.
    ///
    /// Note that mutating the output/input state of the stream may corrupt this
    /// object, so care must be taken when using this method.
    pub fn get_mut(&mut self) -> &mut W {
        self.inner.get_mut()
    }
    /// Resets the state of this decoder entirely, swapping out the output
    /// stream for another.
    ///
    /// This function will finish encoding the current stream into the current
    /// output stream before swapping out the two output streams.
    ///
    /// This will then reset the internal state of this decoder and replace the
    /// output stream with the one provided, returning the previous output
    /// stream. Future data written to this decoder will be decompressed into
    /// the output stream `w`.
    ///
    /// # Errors
    ///
    /// This function will perform I/O to finish the stream, and if that I/O
    /// returns an error then that will be returned from this function.
    pub fn reset(&mut self, w: W) -> io::Result<W> {
        self.inner.finish()?;
        self.inner.data = Decompress::new(false);
        Ok(self.inner.replace(w))
    }
    /// Attempt to finish this output stream, writing out final chunks of data.
    ///
    /// Note that this function can only be used once data has finished being
    /// written to the output stream. After this function is called then further
    /// calls to `write` may result in a panic.
    ///
    /// # Panics
    ///
    /// Attempts to write data to this stream may result in a panic after this
    /// function is called.
    ///
    /// # Errors
    ///
    /// This function will perform I/O to finish the stream, returning any
    /// errors which happen.
    pub fn try_finish(&mut self) -> io::Result<()> {
        self.inner.finish()
    }
    /// Consumes this encoder, flushing the output stream.
    ///
    /// This will flush the underlying data stream and then return the contained
    /// writer if the flush succeeded.
    ///
    /// Note that this function may not be suitable to call in a situation where
    /// the underlying stream is an asynchronous I/O stream. To finish a stream
    /// the `try_finish` (or `shutdown`) method should be used instead. To
    /// re-acquire ownership of a stream it is safe to call this method after
    /// `try_finish` or `shutdown` has returned `Ok`.
    ///
    /// # Errors
    ///
    /// This function will perform I/O to complete this stream, and any I/O
    /// errors which occur will be returned from this function.
    pub fn finish(mut self) -> io::Result<W> {
        self.inner.finish()?;
        Ok(self.inner.take_inner())
    }
    /// Returns the number of bytes that the decompressor has consumed for
    /// decompression.
    ///
    /// Note that this will likely be smaller than the number of bytes
    /// successfully written to this stream due to internal buffering.
    pub fn total_in(&self) -> u64 {
        self.inner.data.total_in()
    }
    /// Returns the number of bytes that the decompressor has written to its
    /// output stream.
    pub fn total_out(&self) -> u64 {
        self.inner.data.total_out()
    }
}
impl<W: Write> Write for DeflateDecoder<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
#[cfg(feature = "tokio")]
impl<W: AsyncWrite> AsyncWrite for DeflateDecoder<W> {
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        self.inner.finish()?;
        self.inner.get_mut().shutdown()
    }
}
impl<W: Read + Write> Read for DeflateDecoder<W> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.get_mut().read(buf)
    }
}
#[cfg(feature = "tokio")]
impl<W: AsyncRead + AsyncWrite> AsyncRead for DeflateDecoder<W> {}
#[cfg(test)]
mod tests_llm_16_46 {
    use super::*;
    use crate::*;
    use std::io::Write;
    #[test]
    fn test_write() {
        let _rug_st_tests_llm_16_46_rrrruuuugggg_test_write = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 4;
        let buf: &[u8] = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3];
        let mut inner = std::io::Cursor::new(Vec::new());
        let mut crc_writer = CrcWriter::new(&mut inner);
        crc_writer.write(buf).unwrap();
        let crc = crc_writer.crc().sum();
        debug_assert_eq!(crc, 0x13582b4c);
        let _rug_ed_tests_llm_16_46_rrrruuuugggg_test_write = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_274 {
    use crate::write::DeflateDecoder;
    use std::io::prelude::*;
    #[test]
    fn test_finish() {
        let _rug_st_tests_llm_16_274_rrrruuuugggg_test_finish = 0;
        let rug_fuzz_0 = b"test data";
        let mut encoder = DeflateDecoder::new(Vec::new());
        let data = rug_fuzz_0;
        encoder.write_all(data).unwrap();
        let result = encoder.finish();
        debug_assert!(result.is_ok());
        let _rug_ed_tests_llm_16_274_rrrruuuugggg_test_finish = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_279 {
    use super::*;
    use crate::*;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_279_rrrruuuugggg_test_new = 0;
        let w = Vec::new();
        let decoder = DeflateDecoder::new(w);
        let _rug_ed_tests_llm_16_279_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_286 {
    use super::*;
    use crate::*;
    use std::io::{self, Write};
    #[test]
    fn test_try_finish() -> io::Result<()> {
        let mut writer = Vec::new();
        let mut deflate = deflate::write::DeflateDecoder::new(writer);
        deflate.write_all(b"Hello, World!")?;
        deflate.try_finish()?;
        Ok(())
    }
}
#[cfg(test)]
mod tests_llm_16_289 {
    use crate::write::DeflateEncoder;
    use crate::Compression;
    use std::io::prelude::*;
    use std::io::Cursor;
    #[test]
    fn test_flush_finish() {
        let _rug_st_tests_llm_16_289_rrrruuuugggg_test_flush_finish = 0;
        let rug_fuzz_0 = b"Hello World";
        let data = rug_fuzz_0;
        let mut output = Vec::new();
        {
            let mut encoder = DeflateEncoder::new(&mut output, Compression::default());
            encoder.write_all(data).unwrap();
            encoder.flush_finish().unwrap();
        }
        debug_assert_eq!(
            & output, & [120, 156, 75, 75, 2, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0]
        );
        let _rug_ed_tests_llm_16_289_rrrruuuugggg_test_flush_finish = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_292 {
    use super::*;
    use crate::*;
    use std::io::{self, Write};
    #[test]
    fn test_get_ref() {
        let _rug_st_tests_llm_16_292_rrrruuuugggg_test_get_ref = 0;
        struct DummyWriter;
        impl Write for DummyWriter {
            fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                Ok(buf.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let writer = DummyWriter;
        let deflate_encoder = DeflateEncoder::new(writer, Compression::default());
        let deflate_encoder_ref = deflate_encoder.get_ref();
        let _rug_ed_tests_llm_16_292_rrrruuuugggg_test_get_ref = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_293 {
    use super::*;
    use crate::*;
    use std::io::Write;
    use crate::Compression;
    #[test]
    fn test_deflate_encoder_new() {
        let _rug_st_tests_llm_16_293_rrrruuuugggg_test_deflate_encoder_new = 0;
        let data: Vec<u8> = Vec::new();
        let level = Compression::default();
        let encoder = DeflateEncoder::new(data, level);
        debug_assert_eq!(encoder.total_in(), 0);
        debug_assert_eq!(encoder.total_out(), 0);
        let _rug_ed_tests_llm_16_293_rrrruuuugggg_test_deflate_encoder_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_296 {
    use crate::write::DeflateEncoder;
    use crate::Compression;
    use std::io::Write;
    #[test]
    fn test_total_in() {
        let _rug_st_tests_llm_16_296_rrrruuuugggg_test_total_in = 0;
        let rug_fuzz_0 = b"Hello, world!";
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(rug_fuzz_0).unwrap();
        debug_assert_eq!(encoder.total_in(), 13);
        let _rug_ed_tests_llm_16_296_rrrruuuugggg_test_total_in = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_299 {
    use super::*;
    use crate::*;
    use std::io::{self, Write};
    #[test]
    fn test_try_finish() -> io::Result<()> {
        let mut buffer: Vec<u8> = Vec::new();
        let mut encoder = DeflateEncoder::new(
            CrcWriter::new(&mut buffer),
            Compression::default(),
        );
        encoder.write_all(b"Hello, world!")?;
        encoder.try_finish()?;
        Ok(())
    }
}
#[cfg(test)]
mod tests_rug_129 {
    use super::*;
    use crate::write::{DeflateEncoder, ZlibEncoder};
    use crate::Compression;
    use std::io::{self, Write};
    #[test]
    fn test_reset() {
        let _rug_st_tests_rug_129_rrrruuuugggg_test_reset = 0;
        let mut p0: DeflateEncoder<Vec<u8>> = DeflateEncoder::new(
            Vec::new(),
            Compression::default(),
        );
        let mut p1: Vec<u8> = Vec::new();
        let result: io::Result<Vec<u8>> = p0.reset(p1);
        debug_assert_eq!(result.is_ok(), true);
        let _rug_ed_tests_rug_129_rrrruuuugggg_test_reset = 0;
    }
}
#[cfg(test)]
mod tests_rug_130 {
    use super::*;
    use crate::write::{DeflateEncoder, ZlibEncoder};
    use crate::Compression;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_130_rrrruuuugggg_test_rug = 0;
        let mut p0: DeflateEncoder<Vec<u8>> = DeflateEncoder::new(
            Vec::new(),
            Compression::default(),
        );
        p0.finish().unwrap();
        let _rug_ed_tests_rug_130_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_131 {
    use super::*;
    use std::io::Write;
    use crate::write::DeflateEncoder;
    use crate::Compression;
    #[test]
    fn test_total_out() {
        let _rug_st_tests_rug_131_rrrruuuugggg_test_total_out = 0;
        let mut p0: Vec<u8> = Vec::new();
        let mut encoder: DeflateEncoder<Vec<u8>> = DeflateEncoder::new(
            p0,
            Compression::default(),
        );
        let result = encoder.total_out();
        debug_assert_eq!(result, 0);
        let _rug_ed_tests_rug_131_rrrruuuugggg_test_total_out = 0;
    }
}
#[cfg(test)]
mod tests_rug_135 {
    use super::*;
    use crate::write::DeflateDecoder;
    #[test]
    fn test_get_mut() {
        let _rug_st_tests_rug_135_rrrruuuugggg_test_get_mut = 0;
        let mut p0: DeflateDecoder<Vec<u8>> = DeflateDecoder::new(Vec::new());
        p0.get_mut();
        let _rug_ed_tests_rug_135_rrrruuuugggg_test_get_mut = 0;
    }
}
#[cfg(test)]
mod tests_rug_138 {
    use super::*;
    use crate::deflate::write::DeflateDecoder;
    use std::io::Write;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_138_rrrruuuugggg_test_rug = 0;
        let mut p0 = Vec::new();
        let mut p1 = DeflateDecoder::<Vec<u8>>::new(p0);
        p1.total_out();
        let _rug_ed_tests_rug_138_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_139 {
    use super::*;
    use crate::write::{DeflateDecoder, DeflateEncoder};
    use std::io::Write;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_139_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 3;
        let rug_fuzz_4 = 4;
        let mut p0: DeflateDecoder<Vec<u8>> = DeflateDecoder::new(Vec::new());
        let mut p1: &[u8] = &[
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
        ];
        p0.write(p1).unwrap();
        let _rug_ed_tests_rug_139_rrrruuuugggg_test_rug = 0;
    }
}
