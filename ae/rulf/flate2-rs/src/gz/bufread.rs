use std::cmp;
use std::io;
use std::io::prelude::*;
use std::mem;
#[cfg(feature = "tokio")]
use futures::Poll;
#[cfg(feature = "tokio")]
use tokio_io::{AsyncRead, AsyncWrite};
use super::{GzBuilder, GzHeader};
use super::{FCOMMENT, FEXTRA, FHCRC, FNAME};
use crate::crc::CrcReader;
use crate::deflate;
use crate::Compression;
fn copy(into: &mut [u8], from: &[u8], pos: &mut usize) -> usize {
    let min = cmp::min(into.len(), from.len() - *pos);
    for (slot, val) in into.iter_mut().zip(from[*pos..*pos + min].iter()) {
        *slot = *val;
    }
    *pos += min;
    return min;
}
pub(crate) fn corrupt() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "corrupt gzip stream does not have a matching checksum",
    )
}
fn bad_header() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, "invalid gzip header")
}
fn read_le_u16<R: Read>(r: &mut R) -> io::Result<u16> {
    let mut b = [0; 2];
    r.read_exact(&mut b)?;
    Ok((b[0] as u16) | ((b[1] as u16) << 8))
}
pub(crate) fn read_gz_header<R: Read>(r: &mut R) -> io::Result<GzHeader> {
    let mut crc_reader = CrcReader::new(r);
    let mut header = [0; 10];
    crc_reader.read_exact(&mut header)?;
    let id1 = header[0];
    let id2 = header[1];
    if id1 != 0x1f || id2 != 0x8b {
        return Err(bad_header());
    }
    let cm = header[2];
    if cm != 8 {
        return Err(bad_header());
    }
    let flg = header[3];
    let mtime = ((header[4] as u32) << 0) | ((header[5] as u32) << 8)
        | ((header[6] as u32) << 16) | ((header[7] as u32) << 24);
    let _xfl = header[8];
    let os = header[9];
    let extra = if flg & FEXTRA != 0 {
        let xlen = read_le_u16(&mut crc_reader)?;
        let mut extra = vec![0; xlen as usize];
        crc_reader.read_exact(&mut extra)?;
        Some(extra)
    } else {
        None
    };
    let filename = if flg & FNAME != 0 {
        let mut b = Vec::new();
        for byte in crc_reader.by_ref().bytes() {
            let byte = byte?;
            if byte == 0 {
                break;
            }
            b.push(byte);
        }
        Some(b)
    } else {
        None
    };
    let comment = if flg & FCOMMENT != 0 {
        let mut b = Vec::new();
        for byte in crc_reader.by_ref().bytes() {
            let byte = byte?;
            if byte == 0 {
                break;
            }
            b.push(byte);
        }
        Some(b)
    } else {
        None
    };
    if flg & FHCRC != 0 {
        let calced_crc = crc_reader.crc().sum() as u16;
        let stored_crc = read_le_u16(&mut crc_reader)?;
        if calced_crc != stored_crc {
            return Err(corrupt());
        }
    }
    Ok(GzHeader {
        extra: extra,
        filename: filename,
        comment: comment,
        operating_system: os,
        mtime: mtime,
    })
}
/// A gzip streaming encoder
///
/// This structure exposes a [`BufRead`] interface that will read uncompressed data
/// from the underlying reader and expose the compressed version as a [`BufRead`]
/// interface.
///
/// [`BufRead`]: https://doc.rust-lang.org/std/io/trait.BufRead.html
///
/// # Examples
///
/// ```
/// use std::io::prelude::*;
/// use std::io;
/// use flate2::Compression;
/// use flate2::bufread::GzEncoder;
/// use std::fs::File;
/// use std::io::BufReader;
///
/// // Opens sample file, compresses the contents and returns a Vector or error
/// // File wrapped in a BufReader implements BufRead
///
/// fn open_hello_world() -> io::Result<Vec<u8>> {
///     let f = File::open("examples/hello_world.txt")?;
///     let b = BufReader::new(f);
///     let mut gz = GzEncoder::new(b, Compression::fast());
///     let mut buffer = Vec::new();
///     gz.read_to_end(&mut buffer)?;
///     Ok(buffer)
/// }
/// ```
#[derive(Debug)]
pub struct GzEncoder<R> {
    inner: deflate::bufread::DeflateEncoder<CrcReader<R>>,
    header: Vec<u8>,
    pos: usize,
    eof: bool,
}
pub fn gz_encoder<R: BufRead>(header: Vec<u8>, r: R, lvl: Compression) -> GzEncoder<R> {
    let crc = CrcReader::new(r);
    GzEncoder {
        inner: deflate::bufread::DeflateEncoder::new(crc, lvl),
        header: header,
        pos: 0,
        eof: false,
    }
}
impl<R: BufRead> GzEncoder<R> {
    /// Creates a new encoder which will use the given compression level.
    ///
    /// The encoder is not configured specially for the emitted header. For
    /// header configuration, see the `GzBuilder` type.
    ///
    /// The data read from the stream `r` will be compressed and available
    /// through the returned reader.
    pub fn new(r: R, level: Compression) -> GzEncoder<R> {
        GzBuilder::new().buf_read(r, level)
    }
    fn read_footer(&mut self, into: &mut [u8]) -> io::Result<usize> {
        if self.pos == 8 {
            return Ok(0);
        }
        let crc = self.inner.get_ref().crc();
        let ref arr = [
            (crc.sum() >> 0) as u8,
            (crc.sum() >> 8) as u8,
            (crc.sum() >> 16) as u8,
            (crc.sum() >> 24) as u8,
            (crc.amount() >> 0) as u8,
            (crc.amount() >> 8) as u8,
            (crc.amount() >> 16) as u8,
            (crc.amount() >> 24) as u8,
        ];
        Ok(copy(into, arr, &mut self.pos))
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
#[inline]
fn finish(buf: &[u8; 8]) -> (u32, u32) {
    let crc = ((buf[0] as u32) << 0) | ((buf[1] as u32) << 8) | ((buf[2] as u32) << 16)
        | ((buf[3] as u32) << 24);
    let amt = ((buf[4] as u32) << 0) | ((buf[5] as u32) << 8) | ((buf[6] as u32) << 16)
        | ((buf[7] as u32) << 24);
    (crc, amt)
}
impl<R: BufRead> Read for GzEncoder<R> {
    fn read(&mut self, mut into: &mut [u8]) -> io::Result<usize> {
        let mut amt = 0;
        if self.eof {
            return self.read_footer(into);
        } else if self.pos < self.header.len() {
            amt += copy(into, &self.header, &mut self.pos);
            if amt == into.len() {
                return Ok(amt);
            }
            let tmp = into;
            into = &mut tmp[amt..];
        }
        match self.inner.read(into)? {
            0 => {
                self.eof = true;
                self.pos = 0;
                self.read_footer(into)
            }
            n => Ok(amt + n),
        }
    }
}
impl<R: BufRead + Write> Write for GzEncoder<R> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.get_mut().write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.get_mut().flush()
    }
}
/// A gzip streaming decoder
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
/// # use flate2::write::GzEncoder;
/// use flate2::bufread::GzDecoder;
///
/// # fn main() {
/// #   let mut e = GzEncoder::new(Vec::new(), Compression::default());
/// #   e.write_all(b"Hello World").unwrap();
/// #   let bytes = e.finish().unwrap();
/// #   println!("{}", decode_reader(bytes).unwrap());
/// # }
/// #
/// // Uncompresses a Gz Encoded vector of bytes and returns a string or error
/// // Here &[u8] implements BufRead
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
    inner: GzState,
    header: Option<GzHeader>,
    reader: CrcReader<deflate::bufread::DeflateDecoder<R>>,
    multi: bool,
}
#[derive(Debug)]
enum GzState {
    Header(Vec<u8>),
    Body,
    Finished(usize, [u8; 8]),
    Err(io::Error),
    End,
}
/// A small adapter which reads data originally from `buf` and then reads all
/// further data from `reader`. This will also buffer all data read from
/// `reader` into `buf` for reuse on a further call.
struct Buffer<'a, T: 'a> {
    buf: &'a mut Vec<u8>,
    buf_cur: usize,
    buf_max: usize,
    reader: &'a mut T,
}
impl<'a, T> Buffer<'a, T> {
    fn new(buf: &'a mut Vec<u8>, reader: &'a mut T) -> Buffer<'a, T> {
        Buffer {
            reader,
            buf_cur: 0,
            buf_max: buf.len(),
            buf,
        }
    }
}
impl<'a, T: Read> Read for Buffer<'a, T> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.buf_cur == self.buf_max {
            let len = self.reader.read(buf)?;
            self.buf.extend_from_slice(&buf[..len]);
            Ok(len)
        } else {
            let len = (&self.buf[self.buf_cur..self.buf_max]).read(buf)?;
            self.buf_cur += len;
            Ok(len)
        }
    }
}
impl<R: BufRead> GzDecoder<R> {
    /// Creates a new decoder from the given reader, immediately parsing the
    /// gzip header.
    pub fn new(mut r: R) -> GzDecoder<R> {
        let mut buf = Vec::with_capacity(10);
        let mut header = None;
        let result = {
            let mut reader = Buffer::new(&mut buf, &mut r);
            read_gz_header(&mut reader)
        };
        let state = match result {
            Ok(hdr) => {
                header = Some(hdr);
                GzState::Body
            }
            Err(ref err) if io::ErrorKind::WouldBlock == err.kind() => {
                GzState::Header(buf)
            }
            Err(err) => GzState::Err(err),
        };
        GzDecoder {
            inner: state,
            reader: CrcReader::new(deflate::bufread::DeflateDecoder::new(r)),
            multi: false,
            header,
        }
    }
    fn multi(mut self, flag: bool) -> GzDecoder<R> {
        self.multi = flag;
        self
    }
}
impl<R> GzDecoder<R> {
    /// Returns the header associated with this stream, if it was valid
    pub fn header(&self) -> Option<&GzHeader> {
        self.header.as_ref()
    }
    /// Acquires a reference to the underlying reader.
    pub fn get_ref(&self) -> &R {
        self.reader.get_ref().get_ref()
    }
    /// Acquires a mutable reference to the underlying stream.
    ///
    /// Note that mutation of the stream may result in surprising results if
    /// this encoder is continued to be used.
    pub fn get_mut(&mut self) -> &mut R {
        self.reader.get_mut().get_mut()
    }
    /// Consumes this decoder, returning the underlying reader.
    pub fn into_inner(self) -> R {
        self.reader.into_inner().into_inner()
    }
}
impl<R: BufRead> Read for GzDecoder<R> {
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        let GzDecoder { inner, header, reader, multi } = self;
        loop {
            *inner = match mem::replace(inner, GzState::End) {
                GzState::Header(mut buf) => {
                    let result = {
                        let mut reader = Buffer::new(
                            &mut buf,
                            reader.get_mut().get_mut(),
                        );
                        read_gz_header(&mut reader)
                    };
                    let hdr = result
                        .map_err(|err| {
                            if io::ErrorKind::WouldBlock == err.kind() {
                                *inner = GzState::Header(buf);
                            }
                            err
                        })?;
                    *header = Some(hdr);
                    GzState::Body
                }
                GzState::Body => {
                    if into.is_empty() {
                        *inner = GzState::Body;
                        return Ok(0);
                    }
                    let n = reader
                        .read(into)
                        .map_err(|err| {
                            if io::ErrorKind::WouldBlock == err.kind() {
                                *inner = GzState::Body;
                            }
                            err
                        })?;
                    match n {
                        0 => GzState::Finished(0, [0; 8]),
                        n => {
                            *inner = GzState::Body;
                            return Ok(n);
                        }
                    }
                }
                GzState::Finished(pos, mut buf) => {
                    if pos < buf.len() {
                        let n = reader
                            .get_mut()
                            .get_mut()
                            .read(&mut buf[pos..])
                            .and_then(|n| {
                                if n == 0 {
                                    Err(io::ErrorKind::UnexpectedEof.into())
                                } else {
                                    Ok(n)
                                }
                            })
                            .map_err(|err| {
                                if io::ErrorKind::WouldBlock == err.kind() {
                                    *inner = GzState::Finished(pos, buf);
                                }
                                err
                            })?;
                        GzState::Finished(pos + n, buf)
                    } else {
                        let (crc, amt) = finish(&buf);
                        if crc != reader.crc().sum() || amt != reader.crc().amount() {
                            return Err(corrupt());
                        } else if *multi {
                            let is_eof = reader
                                .get_mut()
                                .get_mut()
                                .fill_buf()
                                .map(|buf| buf.is_empty())
                                .map_err(|err| {
                                    if io::ErrorKind::WouldBlock == err.kind() {
                                        *inner = GzState::Finished(pos, buf);
                                    }
                                    err
                                })?;
                            if is_eof {
                                GzState::End
                            } else {
                                reader.reset();
                                reader.get_mut().reset_data();
                                header.take();
                                GzState::Header(Vec::with_capacity(10))
                            }
                        } else {
                            GzState::End
                        }
                    }
                }
                GzState::Err(err) => return Err(err),
                GzState::End => return Ok(0),
            };
        }
    }
}
#[cfg(feature = "tokio")]
impl<R: AsyncRead + BufRead> AsyncRead for GzDecoder<R> {}
impl<R: BufRead + Write> Write for GzDecoder<R> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.get_mut().write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.get_mut().flush()
    }
}
#[cfg(feature = "tokio")]
impl<R: AsyncWrite + BufRead> AsyncWrite for GzDecoder<R> {
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        self.get_mut().shutdown()
    }
}
/// A gzip streaming decoder that decodes all members of a multistream
///
/// A gzip member consists of a header, compressed data and a trailer. The [gzip
/// specification](https://tools.ietf.org/html/rfc1952), however, allows multiple
/// gzip members to be joined in a single stream. `MultiGzDecoder` will
/// decode all consecutive members while `GzDecoder` will only decompress
/// the first gzip member. The multistream format is commonly used in
/// bioinformatics, for example when using the BGZF compressed data.
///
/// This structure exposes a [`BufRead`] interface that will consume all gzip members
/// from the underlying reader and emit uncompressed data.
///
/// [`BufRead`]: https://doc.rust-lang.org/std/io/trait.BufRead.html
///
/// # Examples
///
/// ```
/// use std::io::prelude::*;
/// use std::io;
/// # use flate2::Compression;
/// # use flate2::write::GzEncoder;
/// use flate2::bufread::MultiGzDecoder;
///
/// # fn main() {
/// #   let mut e = GzEncoder::new(Vec::new(), Compression::default());
/// #   e.write_all(b"Hello World").unwrap();
/// #   let bytes = e.finish().unwrap();
/// #   println!("{}", decode_reader(bytes).unwrap());
/// # }
/// #
/// // Uncompresses a Gz Encoded vector of bytes and returns a string or error
/// // Here &[u8] implements BufRead
///
/// fn decode_reader(bytes: Vec<u8>) -> io::Result<String> {
///    let mut gz = MultiGzDecoder::new(&bytes[..]);
///    let mut s = String::new();
///    gz.read_to_string(&mut s)?;
///    Ok(s)
/// }
/// ```
#[derive(Debug)]
pub struct MultiGzDecoder<R>(GzDecoder<R>);
impl<R: BufRead> MultiGzDecoder<R> {
    /// Creates a new decoder from the given reader, immediately parsing the
    /// (first) gzip header. If the gzip stream contains multiple members all will
    /// be decoded.
    pub fn new(r: R) -> MultiGzDecoder<R> {
        MultiGzDecoder(GzDecoder::new(r).multi(true))
    }
}
impl<R> MultiGzDecoder<R> {
    /// Returns the current header associated with this stream, if it's valid
    pub fn header(&self) -> Option<&GzHeader> {
        self.0.header()
    }
    /// Acquires a reference to the underlying reader.
    pub fn get_ref(&self) -> &R {
        self.0.get_ref()
    }
    /// Acquires a mutable reference to the underlying stream.
    ///
    /// Note that mutation of the stream may result in surprising results if
    /// this encoder is continued to be used.
    pub fn get_mut(&mut self) -> &mut R {
        self.0.get_mut()
    }
    /// Consumes this decoder, returning the underlying reader.
    pub fn into_inner(self) -> R {
        self.0.into_inner()
    }
}
impl<R: BufRead> Read for MultiGzDecoder<R> {
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        self.0.read(into)
    }
}
#[cfg(feature = "tokio")]
impl<R: AsyncRead + BufRead> AsyncRead for MultiGzDecoder<R> {}
impl<R: BufRead + Write> Write for MultiGzDecoder<R> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.get_mut().write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.get_mut().flush()
    }
}
#[cfg(feature = "tokio")]
impl<R: AsyncWrite + BufRead> AsyncWrite for MultiGzDecoder<R> {
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        self.get_mut().shutdown()
    }
}
#[cfg(test)]
mod tests_llm_16_68_llm_16_67 {
    use super::*;
    use crate::*;
    use crate::*;
    use std::io;
    use std::io::prelude::*;
    #[test]
    fn test_gz_bufread_read() {
        let _rug_st_tests_llm_16_68_llm_16_67_rrrruuuugggg_test_gz_bufread_read = 0;
        let rug_fuzz_0 = 0x1f;
        let rug_fuzz_1 = 0x8b;
        let rug_fuzz_2 = 0x08;
        let rug_fuzz_3 = 0x00;
        let rug_fuzz_4 = 0x00;
        let rug_fuzz_5 = 0x00;
        let rug_fuzz_6 = 0x00;
        let rug_fuzz_7 = 0x00;
        let rug_fuzz_8 = 0x00;
        let rug_fuzz_9 = 0x03;
        let rug_fuzz_10 = 0x63;
        let rug_fuzz_11 = 0x60;
        let rug_fuzz_12 = 0x60;
        let rug_fuzz_13 = 0x62;
        let rug_fuzz_14 = 0x60;
        let rug_fuzz_15 = 0x60;
        let rug_fuzz_16 = 0x03;
        let rug_fuzz_17 = 0x00;
        let rug_fuzz_18 = 0x62;
        let rug_fuzz_19 = 0xec;
        let rug_fuzz_20 = 0xe5;
        let rug_fuzz_21 = 0xe0;
        let rug_fuzz_22 = 0xe5;
        let rug_fuzz_23 = 0xe5;
        let rug_fuzz_24 = 0x02;
        let rug_fuzz_25 = 0x00;
        let rug_fuzz_26 = 0x00;
        let rug_fuzz_27 = 0x00;
        let rug_fuzz_28 = 0xff;
        let rug_fuzz_29 = 0xff;
        let rug_fuzz_30 = 0x03;
        let rug_fuzz_31 = 0x00;
        let rug_fuzz_32 = 0x70;
        let rug_fuzz_33 = 0x6d;
        let rug_fuzz_34 = 0x60;
        let rug_fuzz_35 = 0x2b;
        let rug_fuzz_36 = 0x4d;
        let rug_fuzz_37 = 0x4e;
        let rug_fuzz_38 = 0x4d;
        let rug_fuzz_39 = 0x2e;
        let rug_fuzz_40 = 0xcd;
        let rug_fuzz_41 = 0x2f;
        let rug_fuzz_42 = 0x2d;
        let rug_fuzz_43 = 0x50;
        let rug_fuzz_44 = 0x28;
        let rug_fuzz_45 = 0x49;
        let rug_fuzz_46 = 0x2d;
        let rug_fuzz_47 = 0x52;
        let rug_fuzz_48 = 0xa5;
        let rug_fuzz_49 = 0xa5;
        let rug_fuzz_50 = 0xe5;
        let rug_fuzz_51 = 0x02;
        let rug_fuzz_52 = 0x00;
        let rug_fuzz_53 = 0x00;
        let rug_fuzz_54 = 0x00;
        let inner_reader: &[u8] = &[
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
            rug_fuzz_32,
            rug_fuzz_33,
            rug_fuzz_34,
            rug_fuzz_35,
            rug_fuzz_36,
            rug_fuzz_37,
            rug_fuzz_38,
            rug_fuzz_39,
            rug_fuzz_40,
            rug_fuzz_41,
            rug_fuzz_42,
            rug_fuzz_43,
            rug_fuzz_44,
            rug_fuzz_45,
            rug_fuzz_46,
            rug_fuzz_47,
            rug_fuzz_48,
            rug_fuzz_49,
            rug_fuzz_50,
            rug_fuzz_51,
            rug_fuzz_52,
            rug_fuzz_53,
            rug_fuzz_54,
        ];
        let mut decoder = GzDecoder::new(inner_reader);
        let mut buffer: Vec<u8> = vec![0; 100];
        let result = decoder.read(&mut buffer);
        debug_assert_eq!(result.unwrap(), 26);
        let _rug_ed_tests_llm_16_68_llm_16_67_rrrruuuugggg_test_gz_bufread_read = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_81 {
    use super::*;
    use crate::*;
    use crate::bufread::MultiGzDecoder;
    use crate::Compression;
    use crate::write::GzEncoder;
    #[test]
    fn test_flush() {
        let _rug_st_tests_llm_16_81_rrrruuuugggg_test_flush = 0;
        let rug_fuzz_0 = b"Hello World";
        let rug_fuzz_1 = b"Hello World";
        let mut e = GzEncoder::new(Vec::new(), Compression::default());
        e.write_all(rug_fuzz_0).unwrap();
        let bytes = e.finish().unwrap();
        let mut gz = MultiGzDecoder::new(&bytes[..]);
        let mut s = String::new();
        gz.read_to_string(&mut s).unwrap();
        let mut reference = Vec::new();
        let mut e = GzEncoder::new(&mut reference, Compression::default());
        e.write_all(rug_fuzz_1).unwrap();
        e.finish().unwrap();
        debug_assert_eq!(s, String::from_utf8_lossy(& reference));
        let _rug_ed_tests_llm_16_81_rrrruuuugggg_test_flush = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_324 {
    use super::*;
    use crate::*;
    use std::io::Read;
    struct MockReader {}
    impl Read for MockReader {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            Ok(0)
        }
    }
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_324_rrrruuuugggg_test_new = 0;
        let mut buf = vec![0; 5];
        let mut reader = MockReader {};
        let buffer = Buffer::new(&mut buf, &mut reader);
        let _rug_ed_tests_llm_16_324_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_331 {
    use super::*;
    use crate::*;
    use std::io;
    #[test]
    fn test_into_inner() {
        let _rug_st_tests_llm_16_331_rrrruuuugggg_test_into_inner = 0;
        let rug_fuzz_0 = b"compressed data";
        let data = rug_fuzz_0;
        let reader = io::Cursor::new(data);
        let mut decoder = GzDecoder::new(reader);
        let inner = decoder.into_inner();
        debug_assert_eq!(inner.into_inner(), data);
        let _rug_ed_tests_llm_16_331_rrrruuuugggg_test_into_inner = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_334 {
    use super::*;
    use crate::*;
    use std::io::{BufRead, Read, Write};
    use crate::bufread::GzDecoder;
    use crate::crc::{Crc, CrcReader};
    use crate::deflate::bufread::DeflateDecoder;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_334_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = b"test data";
        let reader: &[u8] = rug_fuzz_0;
        let mut gz_decoder = GzDecoder::new(reader);
        let mut buffer = Vec::new();
        let result = gz_decoder.read_to_end(&mut buffer);
        debug_assert_eq!(result.is_ok(), true);
        debug_assert_eq!(buffer, b"test data");
        let _rug_ed_tests_llm_16_334_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_336_llm_16_335 {
    use crate::bufread::DeflateEncoder;
    use crate::bufread::GzEncoder;
    use crate::Compression;
    use std::io::prelude::*;
    use std::io::BufReader;
    use std::io::BufWriter;
    use std::io::Cursor;
    use std::io::Write;
    #[test]
    fn test_deflate_encoder_get_mut() {
        let _rug_st_tests_llm_16_336_llm_16_335_rrrruuuugggg_test_deflate_encoder_get_mut = 0;
        let rug_fuzz_0 = b"hello world";
        let data: &[u8] = rug_fuzz_0;
        let reader = Cursor::new(data);
        let mut deflate_encoder = DeflateEncoder::new(reader, Compression::default());
        let mut buf = Vec::new();
        deflate_encoder.get_mut().read_to_end(&mut buf).unwrap();
        debug_assert_eq!(
            buf, [120, 94, 195, 177, 95, 201, 45, 128, 202, 75, 201, 204, 41, 207]
        );
        let _rug_ed_tests_llm_16_336_llm_16_335_rrrruuuugggg_test_deflate_encoder_get_mut = 0;
    }
    #[test]
    fn test_deflate_encoder_get_mut_empty() {
        let _rug_st_tests_llm_16_336_llm_16_335_rrrruuuugggg_test_deflate_encoder_get_mut_empty = 0;
        let rug_fuzz_0 = b"";
        let data: &[u8] = rug_fuzz_0;
        let reader = Cursor::new(data);
        let mut deflate_encoder = DeflateEncoder::new(reader, Compression::default());
        let mut buf = Vec::new();
        deflate_encoder.get_mut().read_to_end(&mut buf).unwrap();
        debug_assert_eq!(buf, []);
        let _rug_ed_tests_llm_16_336_llm_16_335_rrrruuuugggg_test_deflate_encoder_get_mut_empty = 0;
    }
    #[test]
    fn test_gz_encoder_get_mut() {
        let _rug_st_tests_llm_16_336_llm_16_335_rrrruuuugggg_test_gz_encoder_get_mut = 0;
        let rug_fuzz_0 = b"hello world";
        let data: &[u8] = rug_fuzz_0;
        let reader = Cursor::new(data);
        let mut gz_encoder = GzEncoder::new(reader, Compression::default());
        let mut buf = Vec::new();
        gz_encoder.get_mut().read_to_end(&mut buf).unwrap();
        debug_assert_eq!(
            buf, [31, 139, 8, 0, 0, 0, 0, 0, 0, 3, 235, 72, 205, 201, 201, 47, 202, 73,
            45, 42, 202, 201, 73, 45, 2, 0, 36, 0, 195, 162, 253, 16, 0, 0, 0]
        );
        let _rug_ed_tests_llm_16_336_llm_16_335_rrrruuuugggg_test_gz_encoder_get_mut = 0;
    }
    #[test]
    fn test_gz_encoder_get_mut_empty() {
        let _rug_st_tests_llm_16_336_llm_16_335_rrrruuuugggg_test_gz_encoder_get_mut_empty = 0;
        let rug_fuzz_0 = b"";
        let data: &[u8] = rug_fuzz_0;
        let reader = Cursor::new(data);
        let mut gz_encoder = GzEncoder::new(reader, Compression::default());
        let mut buf = Vec::new();
        gz_encoder.get_mut().read_to_end(&mut buf).unwrap();
        debug_assert_eq!(
            buf, [31, 139, 8, 0, 0, 0, 0, 0, 0, 3, 235, 72, 205, 201, 201, 47, 202, 73,
            45, 42, 202, 201, 73, 45, 2, 0, 36, 0, 0, 0, 0, 0, 16, 0, 0, 0]
        );
        let _rug_ed_tests_llm_16_336_llm_16_335_rrrruuuugggg_test_gz_encoder_get_mut_empty = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_337 {
    use super::*;
    use crate::*;
    use std::io::Cursor;
    use crate::Compression;
    use crate::bufread::DeflateEncoder;
    use crate::bufread::GzEncoder;
    #[test]
    fn test_get_ref_deflate() {
        let _rug_st_tests_llm_16_337_rrrruuuugggg_test_get_ref_deflate = 0;
        let rug_fuzz_0 = b"Hello, world!";
        let data = rug_fuzz_0;
        let cursor = Cursor::new(data);
        let deflate_encoder = DeflateEncoder::new(cursor, Compression::fast());
        let expected = deflate_encoder.get_ref() as *const _ as usize;
        let actual = deflate_encoder.get_ref() as *const _ as usize;
        debug_assert_eq!(expected, actual);
        let _rug_ed_tests_llm_16_337_rrrruuuugggg_test_get_ref_deflate = 0;
    }
    #[test]
    fn test_get_ref_gz() {
        let _rug_st_tests_llm_16_337_rrrruuuugggg_test_get_ref_gz = 0;
        let rug_fuzz_0 = b"Hello, world!";
        let data = rug_fuzz_0;
        let cursor = Cursor::new(data);
        let gz_encoder = GzEncoder::new(cursor, Compression::fast());
        let expected = gz_encoder.get_ref() as *const _ as usize;
        let actual = gz_encoder.get_ref() as *const _ as usize;
        debug_assert_eq!(expected, actual);
        let _rug_ed_tests_llm_16_337_rrrruuuugggg_test_get_ref_gz = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_338 {
    use super::*;
    use crate::*;
    use crate::{Compression, bufread::{DeflateEncoder, GzEncoder}};
    #[test]
    fn test_into_inner_deflate() {
        let _rug_st_tests_llm_16_338_rrrruuuugggg_test_into_inner_deflate = 0;
        let rug_fuzz_0 = 1;
        let data = vec![rug_fuzz_0, 2, 3, 4, 5];
        let level = Compression::fast();
        let mut encoder = DeflateEncoder::new(data.as_slice(), level);
        let result = encoder.into_inner();
        debug_assert_eq!(result, data);
        let _rug_ed_tests_llm_16_338_rrrruuuugggg_test_into_inner_deflate = 0;
    }
    #[test]
    fn test_into_inner_gz() {
        let _rug_st_tests_llm_16_338_rrrruuuugggg_test_into_inner_gz = 0;
        let rug_fuzz_0 = 1;
        let data = vec![rug_fuzz_0, 2, 3, 4, 5];
        let level = Compression::fast();
        let mut encoder = GzEncoder::new(data.as_slice(), level);
        let result = encoder.into_inner();
        debug_assert_eq!(result, data);
        let _rug_ed_tests_llm_16_338_rrrruuuugggg_test_into_inner_gz = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_353 {
    use std::io;
    use crate::gz::bufread::bad_header;
    #[test]
    fn test_bad_header() {
        let _rug_st_tests_llm_16_353_rrrruuuugggg_test_bad_header = 0;
        let err = bad_header();
        debug_assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        debug_assert_eq!(err.to_string(), "invalid gzip header");
        let _rug_ed_tests_llm_16_353_rrrruuuugggg_test_bad_header = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_354 {
    use crate::gz::bufread::copy;
    #[test]
    fn test_copy() {
        let _rug_st_tests_llm_16_354_rrrruuuugggg_test_copy = 0;
        let rug_fuzz_0 = 0u8;
        let rug_fuzz_1 = 1u8;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 5;
        let mut into = [rug_fuzz_0; 10];
        let from = [rug_fuzz_1; 5];
        let mut pos = rug_fuzz_2;
        let expected = rug_fuzz_3;
        let result = copy(&mut into, &from, &mut pos);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_354_rrrruuuugggg_test_copy = 0;
    }
}
pub enum ErrorKind {
    NotFound,
    PermissionDenied,
    ConnectionRefused,
    ConnectionReset,
    ConnectionAborted,
    NotConnected,
    AddrInUse,
    AddrNotAvailable,
    BrokenPipe,
    AlreadyExists,
    WouldBlock,
    InvalidInput,
    InvalidData,
    TimedOut,
    WriteZero,
    Interrupted,
    Other,
    UnexpectedEof,
}
#[cfg(test)]
mod tests_llm_16_356 {
    use super::*;
    use crate::*;
    #[test]
    fn test_finish() {
        let _rug_st_tests_llm_16_356_rrrruuuugggg_test_finish = 0;
        let rug_fuzz_0 = 0x12;
        let rug_fuzz_1 = 0x34;
        let rug_fuzz_2 = 0x56;
        let rug_fuzz_3 = 0x78;
        let rug_fuzz_4 = 0x9A;
        let rug_fuzz_5 = 0xBC;
        let rug_fuzz_6 = 0xDE;
        let rug_fuzz_7 = 0xF0;
        let buf: [u8; 8] = [
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
            rug_fuzz_6,
            rug_fuzz_7,
        ];
        let result = finish(&buf);
        debug_assert_eq!(result, (0x78563412, 0xF0DEBC9A));
        let _rug_ed_tests_llm_16_356_rrrruuuugggg_test_finish = 0;
    }
}
#[cfg(test)]
mod tests_rug_21 {
    use std::io::{self, Read};
    use crate::read::MultiGzDecoder;
    use crate::bufread::ZlibEncoder;
    use crate::Compression;
    use std::fs::File;
    use std::io::{BufReader, ErrorKind};
    fn read_le_u16<R: Read>(r: &mut R) -> io::Result<u16> {
        let mut b = [0; 2];
        r.read_exact(&mut b)?;
        Ok((b[0] as u16) | ((b[1] as u16) << 8))
    }
    #[test]
    fn test_rug() {
        let f = File::open("examples/hello_world.txt").unwrap();
        let b = BufReader::new(f);
        let mut v8: ZlibEncoder<_> = ZlibEncoder::new(b, Compression::fast());
        let mut p0: MultiGzDecoder<_> = MultiGzDecoder::new(v8);
        assert_eq!(read_le_u16(& mut p0).unwrap(), 0);
    }
}
#[cfg(test)]
mod tests_rug_22 {
    use super::*;
    use crate::bufread::DeflateEncoder;
    use crate::Compression;
    use std::io::{BufReader, Read};
    use std::fs::File;
    #[test]
    fn test_read_gz_header() {
        let _rug_st_tests_rug_22_rrrruuuugggg_test_read_gz_header = 0;
        let rug_fuzz_0 = "examples/hello_world.txt";
        let f = File::open(rug_fuzz_0).unwrap();
        let b = BufReader::new(f);
        let mut p0: DeflateEncoder<BufReader<File>> = DeflateEncoder::new(
            b,
            Compression::fast(),
        );
        read_gz_header(&mut p0);
        let _rug_ed_tests_rug_22_rrrruuuugggg_test_read_gz_header = 0;
    }
}
#[cfg(test)]
mod tests_rug_25 {
    use super::*;
    use std::io::{self, Write};
    use crate::bufread::{GzEncoder, GzDecoder};
    use crate::Compression;
    #[test]
    fn test_gz_encoder_read_footer() {
        let _rug_st_tests_rug_25_rrrruuuugggg_test_gz_encoder_read_footer = 0;
        let rug_fuzz_0 = 0;
        let mut input: Vec<u8> = Vec::new();
        let mut gz_encoder = GzEncoder::new(&input[..], Compression::default());
        let mut into: [u8; 8] = [rug_fuzz_0; 8];
        gz_encoder.read_footer(&mut into).unwrap();
        let _rug_ed_tests_rug_25_rrrruuuugggg_test_gz_encoder_read_footer = 0;
    }
}
#[cfg(test)]
mod tests_rug_30 {
    use super::*;
    use crate::bufread::GzDecoder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_30_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = false;
        let mut p0: GzDecoder<_> = GzDecoder::new(std::io::Cursor::new(vec![]));
        let p1: bool = rug_fuzz_0;
        p0.multi(p1);
        let _rug_ed_tests_rug_30_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_31 {
    use super::*;
    use crate::bufread::GzDecoder;
    #[test]
    fn test_header() {
        let _rug_st_tests_rug_31_rrrruuuugggg_test_header = 0;
        let mut p0: GzDecoder<_> = GzDecoder::new(std::io::Cursor::new(vec![]));
        <GzDecoder<_>>::header(&p0);
        let _rug_ed_tests_rug_31_rrrruuuugggg_test_header = 0;
    }
}
#[cfg(test)]
mod tests_rug_32 {
    use super::*;
    use crate::bufread::GzDecoder;
    use std::io::Cursor;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_32_rrrruuuugggg_test_rug = 0;
        let mut v40: GzDecoder<_> = GzDecoder::new(Cursor::new(vec![]));
        <GzDecoder<_>>::get_ref(&v40);
        let _rug_ed_tests_rug_32_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_33 {
    use super::*;
    use crate::bufread::GzDecoder;
    use std::io::Cursor;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_33_rrrruuuugggg_test_rug = 0;
        let mut p0: GzDecoder<Cursor<Vec<u8>>> = GzDecoder::new(Cursor::new(vec![]));
        <GzDecoder<Cursor<Vec<u8>>>>::get_mut(&mut p0);
        let _rug_ed_tests_rug_33_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_34 {
    use super::*;
    use crate::bufread::GzDecoder;
    use std::io::Write;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_34_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let mut p0: GzDecoder<_> = GzDecoder::new(std::io::Cursor::new(vec![]));
        let p1: &[u8] = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        <GzDecoder<_> as std::io::Write>::write(&mut p0, p1);
        let _rug_ed_tests_rug_34_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_38 {
    use super::*;
    use crate::bufread::MultiGzDecoder;
    #[test]
    fn test_get_ref() {
        let _rug_st_tests_rug_38_rrrruuuugggg_test_get_ref = 0;
        let mut p0: Vec<u8> = Vec::new();
        let mut decoder: MultiGzDecoder<&[u8]> = MultiGzDecoder::new(&p0[..]);
        decoder.get_ref();
        let _rug_ed_tests_rug_38_rrrruuuugggg_test_get_ref = 0;
    }
}
