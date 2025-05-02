use std::io;
use std::io::prelude::*;
#[cfg(feature = "tokio")]
use futures::Poll;
#[cfg(feature = "tokio")]
use tokio_io::{AsyncRead, AsyncWrite};
use crate::zio;
use crate::{Compress, Decompress};
/// A ZLIB encoder, or compressor.
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
/// use flate2::write::ZlibEncoder;
///
/// // Vec<u8> implements Write, assigning the compressed bytes of sample string
///
/// # fn zlib_encoding() -> std::io::Result<()> {
/// let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
/// e.write_all(b"Hello World")?;
/// let compressed = e.finish()?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct ZlibEncoder<W: Write> {
    inner: zio::Writer<W, Compress>,
}
impl<W: Write> ZlibEncoder<W> {
    /// Creates a new encoder which will write compressed data to the stream
    /// given at the given compression level.
    ///
    /// When this encoder is dropped or unwrapped the final pieces of data will
    /// be flushed.
    pub fn new(w: W, level: crate::Compression) -> ZlibEncoder<W> {
        ZlibEncoder {
            inner: zio::Writer::new(w, Compress::new(level, true)),
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
    /// output stream before swapping out the two output streams.
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
impl<W: Write> Write for ZlibEncoder<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
#[cfg(feature = "tokio")]
impl<W: AsyncWrite> AsyncWrite for ZlibEncoder<W> {
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        self.try_finish()?;
        self.get_mut().shutdown()
    }
}
impl<W: Read + Write> Read for ZlibEncoder<W> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.get_mut().read(buf)
    }
}
#[cfg(feature = "tokio")]
impl<W: AsyncRead + AsyncWrite> AsyncRead for ZlibEncoder<W> {}
/// A ZLIB decoder, or decompressor.
///
/// This structure implements a [`Write`] and will emit a stream of decompressed
/// data when fed a stream of compressed data.
///
/// [`Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
///
/// # Examples
///
/// ```
/// use std::io::prelude::*;
/// use std::io;
/// # use flate2::Compression;
/// # use flate2::write::ZlibEncoder;
/// use flate2::write::ZlibDecoder;
///
/// # fn main() {
/// #    let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
/// #    e.write_all(b"Hello World").unwrap();
/// #    let bytes = e.finish().unwrap();
/// #    println!("{}", decode_reader(bytes).unwrap());
/// # }
/// #
/// // Uncompresses a Zlib Encoded vector of bytes and returns a string or error
/// // Here Vec<u8> implements Write
///
/// fn decode_reader(bytes: Vec<u8>) -> io::Result<String> {
///    let mut writer = Vec::new();
///    let mut z = ZlibDecoder::new(writer);
///    z.write_all(&bytes[..])?;
///    writer = z.finish()?;
///    let return_string = String::from_utf8(writer).expect("String parsing error");
///    Ok(return_string)
/// }
/// ```
#[derive(Debug)]
pub struct ZlibDecoder<W: Write> {
    inner: zio::Writer<W, Decompress>,
}
impl<W: Write> ZlibDecoder<W> {
    /// Creates a new decoder which will write uncompressed data to the stream.
    ///
    /// When this decoder is dropped or unwrapped the final pieces of data will
    /// be flushed.
    pub fn new(w: W) -> ZlibDecoder<W> {
        ZlibDecoder {
            inner: zio::Writer::new(w, Decompress::new(true)),
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
    /// This will reset the internal state of this decoder and replace the
    /// output stream with the one provided, returning the previous output
    /// stream. Future data written to this decoder will be decompressed into
    /// the output stream `w`.
    ///
    /// # Errors
    ///
    /// This function will perform I/O to complete this stream, and any I/O
    /// errors which occur will be returned from this function.
    pub fn reset(&mut self, w: W) -> io::Result<W> {
        self.inner.finish()?;
        self.inner.data = Decompress::new(true);
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
impl<W: Write> Write for ZlibDecoder<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
#[cfg(feature = "tokio")]
impl<W: AsyncWrite> AsyncWrite for ZlibDecoder<W> {
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        self.inner.finish()?;
        self.inner.get_mut().shutdown()
    }
}
impl<W: Read + Write> Read for ZlibDecoder<W> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.get_mut().read(buf)
    }
}
#[cfg(feature = "tokio")]
impl<W: AsyncRead + AsyncWrite> AsyncRead for ZlibDecoder<W> {}
#[cfg(test)]
mod tests_llm_16_175 {
    use super::*;
    use crate::*;
    #[test]
    fn test_flush() {
        let _rug_st_tests_llm_16_175_rrrruuuugggg_test_flush = 0;
        let rug_fuzz_0 = b"Hello World";
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(rug_fuzz_0).unwrap();
        let result = encoder.flush();
        debug_assert!(result.is_ok());
        let _rug_ed_tests_llm_16_175_rrrruuuugggg_test_flush = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_176 {
    use super::*;
    use crate::*;
    use std::io::Write;
    #[test]
    fn test_write() {
        let _rug_st_tests_llm_16_176_rrrruuuugggg_test_write = 0;
        let rug_fuzz_0 = "Hello, World!";
        let mut buf: Vec<u8> = Vec::new();
        let mut encoder = ZlibEncoder::new(&mut buf, Compression::default());
        let data = rug_fuzz_0;
        encoder.write(data.as_bytes()).unwrap();
        encoder.finish().unwrap();
        debug_assert_eq!(
            buf, vec![120, 156, 202, 72, 205, 201, 201, 87, 8, 207, 47, 202, 73, 1, 0, 0,
            255, 255]
        );
        let _rug_ed_tests_llm_16_176_rrrruuuugggg_test_write = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_509 {
    use super::*;
    use crate::*;
    use std::io::prelude::*;
    use std::io;
    use crate::write::ZlibEncoder;
    #[test]
    fn test_finish() -> io::Result<()> {
        let mut e = ZlibEncoder::new(Vec::new(), crate::Compression::default());
        e.write_all(b"Hello World")?;
        let bytes = e.finish()?;
        let mut d = zlib::write::ZlibDecoder::new(Vec::new());
        d.write_all(&bytes)?;
        let result = d.finish()?;
        assert_eq!(result, b"Hello World");
        Ok(())
    }
}
#[cfg(test)]
mod tests_llm_16_515_llm_16_514 {
    use super::*;
    use crate::*;
    use std::io::prelude::*;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_515_llm_16_514_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 3;
        let input: &[u8] = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3];
        let mut buffer: Vec<u8> = Vec::new();
        {
            let mut decoder = ZlibDecoder::new(&mut buffer);
            decoder.write_all(input).unwrap();
            decoder.flush().unwrap();
        }
        debug_assert_eq!(buffer, input);
        let _rug_ed_tests_llm_16_515_llm_16_514_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_521 {
    use crate::write::ZlibDecoder;
    use std::io::Write;
    #[test]
    fn test_total_out() {
        let _rug_st_tests_llm_16_521_rrrruuuugggg_test_total_out = 0;
        let rug_fuzz_0 = 0x78;
        let rug_fuzz_1 = 0x01;
        let rug_fuzz_2 = 0x00;
        let rug_fuzz_3 = 0x00;
        let rug_fuzz_4 = 0x00;
        let rug_fuzz_5 = 0x00;
        let rug_fuzz_6 = 0x00;
        let rug_fuzz_7 = 0x00;
        let rug_fuzz_8 = 0x00;
        let rug_fuzz_9 = 0x03;
        let rug_fuzz_10 = 0x00;
        let rug_fuzz_11 = 0x00;
        let rug_fuzz_12 = 0x00;
        let rug_fuzz_13 = 0x00;
        let rug_fuzz_14 = 0x01;
        let rug_fuzz_15 = 0x00;
        let rug_fuzz_16 = 0x78;
        let rug_fuzz_17 = 0x01;
        let rug_fuzz_18 = 0x00;
        let rug_fuzz_19 = 0x00;
        let rug_fuzz_20 = 0x00;
        let rug_fuzz_21 = 0x00;
        let rug_fuzz_22 = 0x00;
        let rug_fuzz_23 = 0x00;
        let rug_fuzz_24 = 0x00;
        let rug_fuzz_25 = 0x03;
        let rug_fuzz_26 = 0x00;
        let rug_fuzz_27 = 0x00;
        let rug_fuzz_28 = 0x00;
        let rug_fuzz_29 = 0x00;
        let rug_fuzz_30 = 0x01;
        let rug_fuzz_31 = 0x00;
        let mut buffer = Vec::new();
        let mut decoder = ZlibDecoder::new(&mut buffer);
        decoder
            .write(
                &[
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
                    rug_fuzz_11,
                    rug_fuzz_12,
                    rug_fuzz_13,
                    rug_fuzz_14,
                    rug_fuzz_15,
                ],
            )
            .unwrap();
        decoder
            .write(
                &[
                    rug_fuzz_16,
                    rug_fuzz_17,
                    rug_fuzz_18,
                    rug_fuzz_19,
                    rug_fuzz_20,
                    rug_fuzz_21,
                    rug_fuzz_22,
                    rug_fuzz_23,
                    rug_fuzz_24,
                    rug_fuzz_25,
                    rug_fuzz_26,
                    rug_fuzz_27,
                    rug_fuzz_28,
                    rug_fuzz_29,
                    rug_fuzz_30,
                    rug_fuzz_31,
                ],
            )
            .unwrap();
        debug_assert_eq!(decoder.total_out(), 32);
        let _rug_ed_tests_llm_16_521_rrrruuuugggg_test_total_out = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_522 {
    use std::io::{self, Write};
    use crate::write::ZlibDecoder;
    #[test]
    fn test_try_finish() -> io::Result<()> {
        let mut buf: Vec<u8> = Vec::new();
        let mut decoder = ZlibDecoder::new(&mut buf);
        decoder.try_finish()?;
        Ok(())
    }
}
#[cfg(test)]
mod tests_llm_16_523 {
    use super::*;
    use crate::*;
    use std::io::Write;
    #[test]
    fn test_finish() {
        let _rug_st_tests_llm_16_523_rrrruuuugggg_test_finish = 0;
        let rug_fuzz_0 = b"Hello, World!";
        let mut encoder: ZlibEncoder<Vec<u8>> = ZlibEncoder::new(
            Vec::new(),
            Compression::default(),
        );
        let input = rug_fuzz_0;
        let _ = encoder.write_all(input);
        let result = encoder.finish();
        debug_assert!(result.is_ok());
        let _rug_ed_tests_llm_16_523_rrrruuuugggg_test_finish = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_524 {
    use crate::Compression;
    use crate::read::ZlibDecoder;
    use crate::write::ZlibEncoder;
    use std::io::{Read, Write};
    #[test]
    fn test_flush_finish() {
        let _rug_st_tests_llm_16_524_rrrruuuugggg_test_flush_finish = 0;
        let rug_fuzz_0 = b"Hello, world!";
        let data = rug_fuzz_0;
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data).unwrap();
        let compressed = encoder.flush_finish().unwrap();
        let mut decoder = ZlibDecoder::new(compressed.as_slice());
        let mut output = Vec::new();
        decoder.read_to_end(&mut output).unwrap();
        debug_assert_eq!(output.as_slice(), data);
        let _rug_ed_tests_llm_16_524_rrrruuuugggg_test_flush_finish = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_529 {
    use super::*;
    use crate::*;
    use std::io::Write;
    #[test]
    fn test_zlib_encoder_new() {
        let _rug_st_tests_llm_16_529_rrrruuuugggg_test_zlib_encoder_new = 0;
        let writer = Vec::new();
        let level = Compression::default();
        let encoder = ZlibEncoder::new(writer, level);
        debug_assert_eq!(encoder.total_in(), 0);
        debug_assert_eq!(encoder.total_out(), 0);
        let _rug_ed_tests_llm_16_529_rrrruuuugggg_test_zlib_encoder_new = 0;
    }
    #[test]
    fn test_zlib_encoder_write() {
        let _rug_st_tests_llm_16_529_rrrruuuugggg_test_zlib_encoder_write = 0;
        let rug_fuzz_0 = b"test data";
        let writer = Vec::new();
        let level = Compression::default();
        let mut encoder = ZlibEncoder::new(writer, level);
        let data = rug_fuzz_0;
        let result = encoder.write(data);
        debug_assert_eq!(result.is_ok(), true);
        let _rug_ed_tests_llm_16_529_rrrruuuugggg_test_zlib_encoder_write = 0;
    }
    #[test]
    fn test_zlib_encoder_flush() {
        let _rug_st_tests_llm_16_529_rrrruuuugggg_test_zlib_encoder_flush = 0;
        let rug_fuzz_0 = b"test data";
        let writer = Vec::new();
        let level = Compression::default();
        let mut encoder = ZlibEncoder::new(writer, level);
        let data = rug_fuzz_0;
        debug_assert_eq!(encoder.write(data).is_ok(), true);
        debug_assert_eq!(encoder.flush().is_ok(), true);
        let _rug_ed_tests_llm_16_529_rrrruuuugggg_test_zlib_encoder_flush = 0;
    }
    #[test]
    fn test_zlib_encoder_try_finish() {
        let _rug_st_tests_llm_16_529_rrrruuuugggg_test_zlib_encoder_try_finish = 0;
        let rug_fuzz_0 = b"test data";
        let writer = Vec::new();
        let level = Compression::default();
        let mut encoder = ZlibEncoder::new(writer, level);
        let data = rug_fuzz_0;
        debug_assert_eq!(encoder.write(data).is_ok(), true);
        debug_assert_eq!(encoder.try_finish().is_ok(), true);
        let _rug_ed_tests_llm_16_529_rrrruuuugggg_test_zlib_encoder_try_finish = 0;
    }
    #[test]
    fn test_zlib_encoder_finish() {
        let _rug_st_tests_llm_16_529_rrrruuuugggg_test_zlib_encoder_finish = 0;
        let rug_fuzz_0 = b"test data";
        let writer = Vec::new();
        let level = Compression::default();
        let mut encoder = ZlibEncoder::new(writer, level);
        let data = rug_fuzz_0;
        debug_assert_eq!(encoder.write(data).is_ok(), true);
        let result = encoder.finish();
        debug_assert_eq!(result.is_ok(), true);
        let _rug_ed_tests_llm_16_529_rrrruuuugggg_test_zlib_encoder_finish = 0;
    }
    #[test]
    fn test_zlib_encoder_flush_finish() {
        let _rug_st_tests_llm_16_529_rrrruuuugggg_test_zlib_encoder_flush_finish = 0;
        let rug_fuzz_0 = b"test data";
        let writer = Vec::new();
        let level = Compression::default();
        let mut encoder = ZlibEncoder::new(writer, level);
        let data = rug_fuzz_0;
        debug_assert_eq!(encoder.write(data).is_ok(), true);
        let result = encoder.flush_finish();
        debug_assert_eq!(result.is_ok(), true);
        let _rug_ed_tests_llm_16_529_rrrruuuugggg_test_zlib_encoder_flush_finish = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_532 {
    use crate::Compression;
    use crate::write::ZlibEncoder;
    use std::io::prelude::*;
    use std::io::Write;
    fn compress_string(data: &str) -> Vec<u8> {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data.as_bytes()).unwrap();
        encoder.finish().unwrap()
    }
    #[test]
    fn test_compress_string() {
        let data = "Hello World";
        let compressed_data = compress_string(data);
    }
}
#[cfg(test)]
mod tests_llm_16_535 {
    use std::io::Write;
    use crate::Compression;
    use crate::write::ZlibEncoder;
    #[test]
    fn test_try_finish() {
        let _rug_st_tests_llm_16_535_rrrruuuugggg_test_try_finish = 0;
        let rug_fuzz_0 = b"Hello World";
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(rug_fuzz_0).unwrap();
        let result = encoder.try_finish();
        debug_assert!(result.is_ok());
        let _rug_ed_tests_llm_16_535_rrrruuuugggg_test_try_finish = 0;
    }
}
#[cfg(test)]
mod tests_rug_159 {
    use super::*;
    use std::io::Cursor;
    use crate::write::ZlibEncoder;
    use crate::Compression;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_159_rrrruuuugggg_test_rug = 0;
        let mut p0: ZlibEncoder<Cursor<Vec<u8>>> = ZlibEncoder::new(
            Cursor::new(Vec::<u8>::new()),
            Compression::default(),
        );
        <ZlibEncoder<Cursor<Vec<u8>>>>::get_ref(&p0);
        let _rug_ed_tests_rug_159_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_162 {
    use super::*;
    use crate::write::ZlibEncoder;
    use crate::Compression;
    use std::io::Write;
    #[test]
    fn test_total_out() {
        let _rug_st_tests_rug_162_rrrruuuugggg_test_total_out = 0;
        let mut p0 = ZlibEncoder::new(Vec::<u8>::new(), Compression::default());
        p0.total_out();
        let _rug_ed_tests_rug_162_rrrruuuugggg_test_total_out = 0;
    }
}
#[cfg(test)]
mod tests_rug_164 {
    use super::*;
    use crate::write::ZlibDecoder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_164_rrrruuuugggg_test_rug = 0;
        let mut p0: ZlibDecoder<Vec<u8>> = ZlibDecoder::new(Vec::new());
        crate::zlib::write::ZlibDecoder::<Vec<u8>>::get_ref(&p0);
        let _rug_ed_tests_rug_164_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_165 {
    use super::*;
    use crate::write::ZlibDecoder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_165_rrrruuuugggg_test_rug = 0;
        let mut v72: ZlibDecoder<Vec<u8>> = ZlibDecoder::new(Vec::new());
        <ZlibDecoder<Vec<u8>>>::get_mut(&mut v72);
        let _rug_ed_tests_rug_165_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_167 {
    use super::*;
    use crate::write::ZlibDecoder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_167_rrrruuuugggg_test_rug = 0;
        let mut p0: ZlibDecoder<Vec<u8>> = ZlibDecoder::new(Vec::new());
        <ZlibDecoder<Vec<u8>>>::total_in(&p0);
        let _rug_ed_tests_rug_167_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_169 {
    use super::*;
    use crate::write::ZlibDecoder;
    use std::io::Write;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_169_rrrruuuugggg_test_rug = 0;
        let mut p0: ZlibDecoder<Vec<u8>> = ZlibDecoder::new(Vec::new());
        p0.flush().unwrap();
        let _rug_ed_tests_rug_169_rrrruuuugggg_test_rug = 0;
    }
}
