use std::cmp;
use std::io;
use std::io::prelude::*;
#[cfg(feature = "tokio")]
use futures::Poll;
#[cfg(feature = "tokio")]
use tokio_io::{AsyncRead, AsyncWrite};
use super::bufread::{corrupt, read_gz_header};
use super::{GzBuilder, GzHeader};
use crate::crc::{Crc, CrcWriter};
use crate::zio;
use crate::{Compress, Compression, Decompress, Status};
/// A gzip streaming encoder
///
/// This structure exposes a [`Write`] interface that will emit compressed data
/// to the underlying writer `W`.
///
/// [`Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
///
/// # Examples
///
/// ```
/// use std::io::prelude::*;
/// use flate2::Compression;
/// use flate2::write::GzEncoder;
///
/// // Vec<u8> implements Write to print the compressed bytes of sample string
/// # fn main() {
///
/// let mut e = GzEncoder::new(Vec::new(), Compression::default());
/// e.write_all(b"Hello World").unwrap();
/// println!("{:?}", e.finish().unwrap());
/// # }
/// ```
#[derive(Debug)]
pub struct GzEncoder<W: Write> {
    inner: zio::Writer<W, Compress>,
    crc: Crc,
    crc_bytes_written: usize,
    header: Vec<u8>,
}
pub fn gz_encoder<W: Write>(header: Vec<u8>, w: W, lvl: Compression) -> GzEncoder<W> {
    GzEncoder {
        inner: zio::Writer::new(w, Compress::new(lvl, false)),
        crc: Crc::new(),
        header: header,
        crc_bytes_written: 0,
    }
}
impl<W: Write> GzEncoder<W> {
    /// Creates a new encoder which will use the given compression level.
    ///
    /// The encoder is not configured specially for the emitted header. For
    /// header configuration, see the `GzBuilder` type.
    ///
    /// The data written to the returned encoder will be compressed and then
    /// written to the stream `w`.
    pub fn new(w: W, level: Compression) -> GzEncoder<W> {
        GzBuilder::new().write(w, level)
    }
    /// Acquires a reference to the underlying writer.
    pub fn get_ref(&self) -> &W {
        self.inner.get_ref()
    }
    /// Acquires a mutable reference to the underlying writer.
    ///
    /// Note that mutation of the writer may result in surprising results if
    /// this encoder is continued to be used.
    pub fn get_mut(&mut self) -> &mut W {
        self.inner.get_mut()
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
        self.write_header()?;
        self.inner.finish()?;
        while self.crc_bytes_written < 8 {
            let (sum, amt) = (self.crc.sum() as u32, self.crc.amount());
            let buf = [
                (sum >> 0) as u8,
                (sum >> 8) as u8,
                (sum >> 16) as u8,
                (sum >> 24) as u8,
                (amt >> 0) as u8,
                (amt >> 8) as u8,
                (amt >> 16) as u8,
                (amt >> 24) as u8,
            ];
            let inner = self.inner.get_mut();
            let n = inner.write(&buf[self.crc_bytes_written..])?;
            self.crc_bytes_written += n;
        }
        Ok(())
    }
    /// Finish encoding this stream, returning the underlying writer once the
    /// encoding is done.
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
        self.try_finish()?;
        Ok(self.inner.take_inner())
    }
    fn write_header(&mut self) -> io::Result<()> {
        while self.header.len() > 0 {
            let n = self.inner.get_mut().write(&self.header)?;
            self.header.drain(..n);
        }
        Ok(())
    }
}
impl<W: Write> Write for GzEncoder<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        assert_eq!(self.crc_bytes_written, 0);
        self.write_header()?;
        let n = self.inner.write(buf)?;
        self.crc.update(&buf[..n]);
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        assert_eq!(self.crc_bytes_written, 0);
        self.write_header()?;
        self.inner.flush()
    }
}
#[cfg(feature = "tokio")]
impl<W: AsyncWrite> AsyncWrite for GzEncoder<W> {
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        self.try_finish()?;
        self.get_mut().shutdown()
    }
}
impl<R: Read + Write> Read for GzEncoder<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.get_mut().read(buf)
    }
}
#[cfg(feature = "tokio")]
impl<R: AsyncRead + AsyncWrite> AsyncRead for GzEncoder<R> {}
impl<W: Write> Drop for GzEncoder<W> {
    fn drop(&mut self) {
        if self.inner.is_present() {
            let _ = self.try_finish();
        }
    }
}
/// A gzip streaming decoder
///
/// This structure exposes a [`Write`] interface that will emit compressed data
/// to the underlying writer `W`.
///
/// [`Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
///
/// # Examples
///
/// ```
/// use std::io::prelude::*;
/// use std::io;
/// use flate2::Compression;
/// use flate2::write::{GzEncoder, GzDecoder};
///
/// # fn main() {
/// #    let mut e = GzEncoder::new(Vec::new(), Compression::default());
/// #    e.write(b"Hello World").unwrap();
/// #    let bytes = e.finish().unwrap();
/// #    assert_eq!("Hello World", decode_writer(bytes).unwrap());
/// # }
/// // Uncompresses a gzip encoded vector of bytes and returns a string or error
/// // Here Vec<u8> implements Write
/// fn decode_writer(bytes: Vec<u8>) -> io::Result<String> {
///    let mut writer = Vec::new();
///    let mut decoder = GzDecoder::new(writer);
///    decoder.write_all(&bytes[..])?;
///    writer = decoder.finish()?;
///    let return_string = String::from_utf8(writer).expect("String parsing error");
///    Ok(return_string)
/// }
/// ```
#[derive(Debug)]
pub struct GzDecoder<W: Write> {
    inner: zio::Writer<CrcWriter<W>, Decompress>,
    crc_bytes: Vec<u8>,
    header: Option<GzHeader>,
    header_buf: Vec<u8>,
}
const CRC_BYTES_LEN: usize = 8;
impl<W: Write> GzDecoder<W> {
    /// Creates a new decoder which will write uncompressed data to the stream.
    ///
    /// When this encoder is dropped or unwrapped the final pieces of data will
    /// be flushed.
    pub fn new(w: W) -> GzDecoder<W> {
        GzDecoder {
            inner: zio::Writer::new(CrcWriter::new(w), Decompress::new(false)),
            crc_bytes: Vec::with_capacity(CRC_BYTES_LEN),
            header: None,
            header_buf: Vec::new(),
        }
    }
    /// Returns the header associated with this stream.
    pub fn header(&self) -> Option<&GzHeader> {
        self.header.as_ref()
    }
    /// Acquires a reference to the underlying writer.
    pub fn get_ref(&self) -> &W {
        self.inner.get_ref().get_ref()
    }
    /// Acquires a mutable reference to the underlying writer.
    ///
    /// Note that mutating the output/input state of the stream may corrupt this
    /// object, so care must be taken when using this method.
    pub fn get_mut(&mut self) -> &mut W {
        self.inner.get_mut().get_mut()
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
        self.finish_and_check_crc()?;
        Ok(())
    }
    /// Consumes this decoder, flushing the output stream.
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
        self.finish_and_check_crc()?;
        Ok(self.inner.take_inner().into_inner())
    }
    fn finish_and_check_crc(&mut self) -> io::Result<()> {
        self.inner.finish()?;
        if self.crc_bytes.len() != 8 {
            return Err(corrupt());
        }
        let crc = ((self.crc_bytes[0] as u32) << 0) | ((self.crc_bytes[1] as u32) << 8)
            | ((self.crc_bytes[2] as u32) << 16) | ((self.crc_bytes[3] as u32) << 24);
        let amt = ((self.crc_bytes[4] as u32) << 0) | ((self.crc_bytes[5] as u32) << 8)
            | ((self.crc_bytes[6] as u32) << 16) | ((self.crc_bytes[7] as u32) << 24);
        if crc != self.inner.get_ref().crc().sum() as u32 {
            return Err(corrupt());
        }
        if amt != self.inner.get_ref().crc().amount() {
            return Err(corrupt());
        }
        Ok(())
    }
}
struct Counter<T: Read> {
    inner: T,
    pos: usize,
}
impl<T: Read> Read for Counter<T> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let pos = self.inner.read(buf)?;
        self.pos += pos;
        Ok(pos)
    }
}
impl<W: Write> Write for GzDecoder<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.header.is_none() {
            let (res, pos) = {
                let mut counter = Counter {
                    inner: self.header_buf.chain(buf),
                    pos: 0,
                };
                let res = read_gz_header(&mut counter);
                (res, counter.pos)
            };
            match res {
                Err(err) => {
                    if err.kind() == io::ErrorKind::UnexpectedEof {
                        self.header_buf.extend(buf);
                        Ok(buf.len())
                    } else {
                        Err(err)
                    }
                }
                Ok(header) => {
                    self.header = Some(header);
                    let pos = pos - self.header_buf.len();
                    self.header_buf.truncate(0);
                    Ok(pos)
                }
            }
        } else {
            let (n, status) = self.inner.write_with_status(buf)?;
            if status == Status::StreamEnd {
                if n < buf.len() && self.crc_bytes.len() < 8 {
                    let remaining = buf.len() - n;
                    let crc_bytes = cmp::min(
                        remaining,
                        CRC_BYTES_LEN - self.crc_bytes.len(),
                    );
                    self.crc_bytes.extend(&buf[n..n + crc_bytes]);
                    return Ok(n + crc_bytes);
                }
            }
            Ok(n)
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
#[cfg(feature = "tokio")]
impl<W: AsyncWrite> AsyncWrite for GzDecoder<W> {
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        self.try_finish()?;
        self.inner.get_mut().get_mut().shutdown()
    }
}
impl<W: Read + Write> Read for GzDecoder<W> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.get_mut().get_mut().read(buf)
    }
}
#[cfg(feature = "tokio")]
impl<W: AsyncRead + AsyncWrite> AsyncRead for GzDecoder<W> {}
#[cfg(test)]
mod tests {
    use super::*;
    const STR: &'static str = "Hello World Hello World Hello World Hello World Hello World \
                               Hello World Hello World Hello World Hello World Hello World \
                               Hello World Hello World Hello World Hello World Hello World \
                               Hello World Hello World Hello World Hello World Hello World \
                               Hello World Hello World Hello World Hello World Hello World";
    #[test]
    fn decode_writer_one_chunk() {
        let mut e = GzEncoder::new(Vec::new(), Compression::default());
        e.write(STR.as_ref()).unwrap();
        let bytes = e.finish().unwrap();
        let mut writer = Vec::new();
        let mut decoder = GzDecoder::new(writer);
        let n = decoder.write(&bytes[..]).unwrap();
        decoder.write(&bytes[n..]).unwrap();
        decoder.try_finish().unwrap();
        writer = decoder.finish().unwrap();
        let return_string = String::from_utf8(writer).expect("String parsing error");
        assert_eq!(return_string, STR);
    }
    #[test]
    fn decode_writer_partial_header() {
        let mut e = GzEncoder::new(Vec::new(), Compression::default());
        e.write(STR.as_ref()).unwrap();
        let bytes = e.finish().unwrap();
        let mut writer = Vec::new();
        let mut decoder = GzDecoder::new(writer);
        assert_eq!(decoder.write(& bytes[..5]).unwrap(), 5);
        let n = decoder.write(&bytes[5..]).unwrap();
        if n < bytes.len() - 5 {
            decoder.write(&bytes[n + 5..]).unwrap();
        }
        writer = decoder.finish().unwrap();
        let return_string = String::from_utf8(writer).expect("String parsing error");
        assert_eq!(return_string, STR);
    }
    #[test]
    fn decode_writer_exact_header() {
        let mut e = GzEncoder::new(Vec::new(), Compression::default());
        e.write(STR.as_ref()).unwrap();
        let bytes = e.finish().unwrap();
        let mut writer = Vec::new();
        let mut decoder = GzDecoder::new(writer);
        assert_eq!(decoder.write(& bytes[..10]).unwrap(), 10);
        decoder.write(&bytes[10..]).unwrap();
        writer = decoder.finish().unwrap();
        let return_string = String::from_utf8(writer).expect("String parsing error");
        assert_eq!(return_string, STR);
    }
    #[test]
    fn decode_writer_partial_crc() {
        let mut e = GzEncoder::new(Vec::new(), Compression::default());
        e.write(STR.as_ref()).unwrap();
        let bytes = e.finish().unwrap();
        let mut writer = Vec::new();
        let mut decoder = GzDecoder::new(writer);
        let l = bytes.len() - 5;
        let n = decoder.write(&bytes[..l]).unwrap();
        decoder.write(&bytes[n..]).unwrap();
        writer = decoder.finish().unwrap();
        let return_string = String::from_utf8(writer).expect("String parsing error");
        assert_eq!(return_string, STR);
    }
}
#[cfg(test)]
mod tests_llm_16_104_llm_16_103 {
    use std::io::prelude::*;
    use std::io::{self, Cursor};
    use crate::Compression;
    use crate::{write::GzDecoder, read::MultiGzDecoder};
    use crate::write::GzEncoder;
    const TEST_DATA: &[u8] = b"Hello, World!";
    #[test]
    fn test_gz_encode_decode() {
        let _rug_st_tests_llm_16_104_llm_16_103_rrrruuuugggg_test_gz_encode_decode = 0;
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(TEST_DATA).unwrap();
        let compressed_data = encoder.finish().unwrap();
        let mut decoder = GzDecoder::new(Cursor::new(compressed_data));
        let mut decompressed_data = Vec::new();
        decoder.read_to_end(&mut decompressed_data).unwrap();
        debug_assert_eq!(decompressed_data, TEST_DATA);
        let _rug_ed_tests_llm_16_104_llm_16_103_rrrruuuugggg_test_gz_encode_decode = 0;
    }
    #[test]
    fn test_multi_gz_decode() {
        let _rug_st_tests_llm_16_104_llm_16_103_rrrruuuugggg_test_multi_gz_decode = 0;
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(TEST_DATA).unwrap();
        let compressed_data = encoder.finish().unwrap();
        let mut multi_decoder = MultiGzDecoder::new(Cursor::new(compressed_data));
        let mut decompressed_data = Vec::new();
        multi_decoder.read_to_end(&mut decompressed_data).unwrap();
        debug_assert_eq!(decompressed_data, TEST_DATA);
        let _rug_ed_tests_llm_16_104_llm_16_103_rrrruuuugggg_test_multi_gz_decode = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_106 {
    use super::*;
    use crate::*;
    use std::io::{self, Write};
    use crate::write::ZlibEncoder;
    #[test]
    fn test_flush() {
        let _rug_st_tests_llm_16_106_rrrruuuugggg_test_flush = 0;
        let rug_fuzz_0 = b"Hello, World!";
        let mut zlib_encoder = ZlibEncoder::new(
            Vec::new(),
            crate::Compression::default(),
        );
        zlib_encoder.write_all(rug_fuzz_0).unwrap();
        let result = zlib_encoder.flush();
        debug_assert!(result.is_ok());
        let _rug_ed_tests_llm_16_106_rrrruuuugggg_test_flush = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_112 {
    use super::*;
    use crate::*;
    use std::io::Write;
    #[test]
    fn test_flush() {
        let _rug_st_tests_llm_16_112_rrrruuuugggg_test_flush = 0;
        let rug_fuzz_0 = b"Hello World";
        let mut w = Vec::new();
        {
            let mut encoder = gz::write::GzEncoder::new(&mut w, Compression::default());
            encoder.write_all(rug_fuzz_0).unwrap();
            encoder.flush().unwrap();
        }
        let result = w.as_slice();
        debug_assert_eq!(
            result, & [31, 139, 8, 0, 0, 0, 0, 0, 0, 255, 72, 205, 201, 201, 215, 81,
            208, 47, 202, 73, 1, 0, 0, 0]
        );
        let _rug_ed_tests_llm_16_112_rrrruuuugggg_test_flush = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_398 {
    use super::*;
    use crate::*;
    use std::io::Write;
    #[test]
    fn test_get_ref() {
        let _rug_st_tests_llm_16_398_rrrruuuugggg_test_get_ref = 0;
        let mut writer = Vec::new();
        let mut gz = gz::write::GzDecoder::new(&mut writer);
        gz.get_ref();
        let _rug_ed_tests_llm_16_398_rrrruuuugggg_test_get_ref = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_401 {
    use super::*;
    use crate::*;
    use std::io::Write;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_401_rrrruuuugggg_test_new = 0;
        let writer = Vec::new();
        let decoder = GzDecoder::new(writer);
        let _rug_ed_tests_llm_16_401_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_402 {
    use std::io::{self, Write};
    use crate::{Compression, read::ZlibDecoder, write::ZlibEncoder};
    use crate::write::GzEncoder;
    #[test]
    fn test_try_finish() {
        let _rug_st_tests_llm_16_402_rrrruuuugggg_test_try_finish = 0;
        let rug_fuzz_0 = b"Hello, world!";
        let mut buffer: Vec<u8> = Vec::new();
        let mut encoder = GzEncoder::new(&mut buffer[..], Compression::default());
        encoder.write_all(rug_fuzz_0).unwrap();
        encoder.try_finish().unwrap();
        let _rug_ed_tests_llm_16_402_rrrruuuugggg_test_try_finish = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_403 {
    use super::*;
    use crate::*;
    use std::io::Cursor;
    #[test]
    fn test_finish() {
        let _rug_st_tests_llm_16_403_rrrruuuugggg_test_finish = 0;
        let rug_fuzz_0 = b"Hello World";
        let data = rug_fuzz_0;
        let mut gz_encoder = GzEncoder::new(Vec::new(), Compression::default());
        gz_encoder.write_all(data).unwrap();
        let compressed_data = gz_encoder.finish().unwrap();
        let mut gz_decoder = GzDecoder::new(Cursor::new(compressed_data));
        let mut decompressed_data = Vec::new();
        gz_decoder.read_to_end(&mut decompressed_data).unwrap();
        debug_assert_eq!(decompressed_data, data);
        let _rug_ed_tests_llm_16_403_rrrruuuugggg_test_finish = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_404 {
    use super::*;
    use crate::*;
    #[test]
    fn test_get_mut() {
        let _rug_st_tests_llm_16_404_rrrruuuugggg_test_get_mut = 0;
        let rug_fuzz_0 = b"test";
        let rug_fuzz_1 = b" continued";
        let mut writer = Vec::new();
        let mut gz_encoder = GzEncoder::new(
            CrcWriter::new(&mut writer),
            Compression::default(),
        );
        gz_encoder.write_all(rug_fuzz_0).unwrap();
        let w = gz_encoder.get_mut();
        debug_assert_eq!(w.write(rug_fuzz_1).unwrap(), 9);
        let _rug_ed_tests_llm_16_404_rrrruuuugggg_test_get_mut = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_405 {
    use super::*;
    use crate::*;
    #[test]
    fn test_get_ref() {
        let _rug_st_tests_llm_16_405_rrrruuuugggg_test_get_ref = 0;
        let data = Vec::new();
        let writer = CrcWriter::new(data);
        let gz_encoder = GzEncoder::new(writer, Compression::default());
        let result = gz_encoder.get_ref();
        let _rug_ed_tests_llm_16_405_rrrruuuugggg_test_get_ref = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_406 {
    use super::*;
    use crate::*;
    use std::io::Write;
    use crate::Compression;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_406_rrrruuuugggg_test_new = 0;
        let w: Vec<u8> = vec![];
        let level = Compression::default();
        let result = GzEncoder::new(w, level);
        debug_assert_eq!(result.get_ref().len(), 0);
        let _rug_ed_tests_llm_16_406_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_410 {
    use crate::gz::write::{GzEncoder, GzBuilder};
    use crate::Compression;
    use std::io::{self, Write};
    #[test]
    fn test_write_header() {
        let _rug_st_tests_llm_16_410_rrrruuuugggg_test_write_header = 0;
        struct MockWriter;
        impl Write for MockWriter {
            fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                Ok(buf.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let mut encoder = GzBuilder::new().write(MockWriter, Compression::default());
        let result = encoder.write_header();
        debug_assert!(result.is_ok());
        let _rug_ed_tests_llm_16_410_rrrruuuugggg_test_write_header = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_411 {
    use super::*;
    use crate::*;
    use std::io::Write;
    #[test]
    fn test_gz_encoder() {
        let _rug_st_tests_llm_16_411_rrrruuuugggg_test_gz_encoder = 0;
        let rug_fuzz_0 = 0x1f;
        let rug_fuzz_1 = 6;
        let header = vec![
            rug_fuzz_0, 0x8b, 0x08, 0x08, 0x3f, 0x63, 0x4d, 0x5a, 0x00, 0x03
        ];
        let mut output = Vec::new();
        let level = Compression::new(rug_fuzz_1);
        let _ = gz_encoder(header, &mut output, level);
        debug_assert_eq!(output.len(), 0);
        let _rug_ed_tests_llm_16_411_rrrruuuugggg_test_gz_encoder = 0;
    }
}
#[cfg(test)]
mod tests_rug_63 {
    use super::*;
    use crate::write::GzEncoder;
    use crate::Compression;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_63_rrrruuuugggg_test_rug = 0;
        let mut p0: GzEncoder::<Vec<u8>> = GzEncoder::new(
            Vec::new(),
            Compression::default(),
        );
        GzEncoder::<Vec<u8>>::try_finish(&mut p0).unwrap();
        let _rug_ed_tests_rug_63_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_64 {
    use super::*;
    use crate::{write::GzEncoder, Compression};
    use std::io::Write;
    #[test]
    fn test_write() {
        let _rug_st_tests_rug_64_rrrruuuugggg_test_write = 0;
        let rug_fuzz_0 = b"Hello, world!";
        let mut p0: GzEncoder<Vec<u8>> = GzEncoder::new(
            Vec::new(),
            Compression::default(),
        );
        let p1: &[u8] = rug_fuzz_0;
        p0.write(p1).unwrap();
        let _rug_ed_tests_rug_64_rrrruuuugggg_test_write = 0;
    }
}
#[cfg(test)]
mod tests_rug_67 {
    use super::*;
    use crate::write::GzDecoder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_67_rrrruuuugggg_test_rug = 0;
        let mut v48: GzDecoder<Vec<u8>> = GzDecoder::new(Vec::new());
        crate::gz::write::GzDecoder::<Vec<u8>>::header(&v48);
        let _rug_ed_tests_rug_67_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_68 {
    use super::*;
    use crate::write::GzDecoder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_68_rrrruuuugggg_test_rug = 0;
        let mut v48: GzDecoder<Vec<u8>> = GzDecoder::new(Vec::new());
        let mut p0: &mut GzDecoder<Vec<u8>> = &mut v48;
        <GzDecoder<Vec<u8>>>::get_mut(p0);
        let _rug_ed_tests_rug_68_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_69 {
    use super::*;
    use crate::write::GzDecoder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_69_rrrruuuugggg_test_rug = 0;
        let mut v48: GzDecoder<Vec<u8>> = GzDecoder::new(Vec::new());
        let p0 = v48;
        p0.finish().unwrap();
        let _rug_ed_tests_rug_69_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_70 {
    use super::*;
    use crate::write::GzDecoder;
    use std::io;
    #[test]
    fn test_finish_and_check_crc() {
        let _rug_st_tests_rug_70_rrrruuuugggg_test_finish_and_check_crc = 0;
        let mut p0: GzDecoder<Vec<u8>> = GzDecoder::new(Vec::new());
        p0.finish_and_check_crc().unwrap();
        let _rug_ed_tests_rug_70_rrrruuuugggg_test_finish_and_check_crc = 0;
    }
}
#[cfg(test)]
mod tests_rug_72 {
    use super::*;
    use crate::write::GzDecoder;
    use std::io::Write;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_72_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"Hello, World!";
        let rug_fuzz_1 = "Failed to write";
        let mut p0: GzDecoder<Vec<u8>> = GzDecoder::new(Vec::new());
        let p1: &[u8] = rug_fuzz_0;
        p0.write(p1).expect(rug_fuzz_1);
        let _rug_ed_tests_rug_72_rrrruuuugggg_test_rug = 0;
    }
}
