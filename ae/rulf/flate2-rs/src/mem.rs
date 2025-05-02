use std::error::Error;
use std::fmt;
use std::io;
use std::slice;
use crate::ffi::{self, Backend, Deflate, DeflateBackend, Inflate, InflateBackend};
use crate::Compression;
/// Raw in-memory compression stream for blocks of data.
///
/// This type is the building block for the I/O streams in the rest of this
/// crate. It requires more management than the [`Read`]/[`Write`] API but is
/// maximally flexible in terms of accepting input from any source and being
/// able to produce output to any memory location.
///
/// It is recommended to use the I/O stream adaptors over this type as they're
/// easier to use.
///
/// [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
/// [`Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
#[derive(Debug)]
pub struct Compress {
    inner: Deflate,
}
/// Raw in-memory decompression stream for blocks of data.
///
/// This type is the building block for the I/O streams in the rest of this
/// crate. It requires more management than the [`Read`]/[`Write`] API but is
/// maximally flexible in terms of accepting input from any source and being
/// able to produce output to any memory location.
///
/// It is recommended to use the I/O stream adaptors over this type as they're
/// easier to use.
///
/// [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
/// [`Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
#[derive(Debug)]
pub struct Decompress {
    inner: Inflate,
}
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
/// Values which indicate the form of flushing to be used when compressing
/// in-memory data.
pub enum FlushCompress {
    /// A typical parameter for passing to compression/decompression functions,
    /// this indicates that the underlying stream to decide how much data to
    /// accumulate before producing output in order to maximize compression.
    None = ffi::MZ_NO_FLUSH as isize,
    /// All pending output is flushed to the output buffer and the output is
    /// aligned on a byte boundary so that the decompressor can get all input
    /// data available so far.
    ///
    /// Flushing may degrade compression for some compression algorithms and so
    /// it should only be used when necessary. This will complete the current
    /// deflate block and follow it with an empty stored block.
    Sync = ffi::MZ_SYNC_FLUSH as isize,
    /// All pending output is flushed to the output buffer, but the output is
    /// not aligned to a byte boundary.
    ///
    /// All of the input data so far will be available to the decompressor (as
    /// with `Flush::Sync`. This completes the current deflate block and follows
    /// it with an empty fixed codes block that is 10 bites long, and it assures
    /// that enough bytes are output in order for the decompessor to finish the
    /// block before the empty fixed code block.
    Partial = ffi::MZ_PARTIAL_FLUSH as isize,
    /// All output is flushed as with `Flush::Sync` and the compression state is
    /// reset so decompression can restart from this point if previous
    /// compressed data has been damaged or if random access is desired.
    ///
    /// Using this option too often can seriously degrade compression.
    Full = ffi::MZ_FULL_FLUSH as isize,
    /// Pending input is processed and pending output is flushed.
    ///
    /// The return value may indicate that the stream is not yet done and more
    /// data has yet to be processed.
    Finish = ffi::MZ_FINISH as isize,
    #[doc(hidden)]
    _Nonexhaustive,
}
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
/// Values which indicate the form of flushing to be used when
/// decompressing in-memory data.
pub enum FlushDecompress {
    /// A typical parameter for passing to compression/decompression functions,
    /// this indicates that the underlying stream to decide how much data to
    /// accumulate before producing output in order to maximize compression.
    None = ffi::MZ_NO_FLUSH as isize,
    /// All pending output is flushed to the output buffer and the output is
    /// aligned on a byte boundary so that the decompressor can get all input
    /// data available so far.
    ///
    /// Flushing may degrade compression for some compression algorithms and so
    /// it should only be used when necessary. This will complete the current
    /// deflate block and follow it with an empty stored block.
    Sync = ffi::MZ_SYNC_FLUSH as isize,
    /// Pending input is processed and pending output is flushed.
    ///
    /// The return value may indicate that the stream is not yet done and more
    /// data has yet to be processed.
    Finish = ffi::MZ_FINISH as isize,
    #[doc(hidden)]
    _Nonexhaustive,
}
/// The inner state for an error when decompressing
#[derive(Debug, Default)]
pub(crate) struct DecompressErrorInner {
    pub(crate) needs_dictionary: Option<u32>,
}
/// Error returned when a decompression object finds that the input stream of
/// bytes was not a valid input stream of bytes.
#[derive(Debug)]
pub struct DecompressError(pub(crate) DecompressErrorInner);
impl DecompressError {
    /// Indicates whether decompression failed due to requiring a dictionary.
    ///
    /// The resulting integer is the Adler-32 checksum of the dictionary
    /// required.
    pub fn needs_dictionary(&self) -> Option<u32> {
        self.0.needs_dictionary
    }
}
#[inline]
pub(crate) fn decompress_failed() -> Result<Status, DecompressError> {
    Err(DecompressError(Default::default()))
}
#[inline]
pub(crate) fn decompress_need_dict(adler: u32) -> Result<Status, DecompressError> {
    Err(
        DecompressError(DecompressErrorInner {
            needs_dictionary: Some(adler),
        }),
    )
}
/// Error returned when a compression object is used incorrectly or otherwise
/// generates an error.
#[derive(Debug)]
pub struct CompressError(pub(crate) ());
/// Possible status results of compressing some data or successfully
/// decompressing a block of data.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Status {
    /// Indicates success.
    ///
    /// Means that more input may be needed but isn't available
    /// and/or there's more output to be written but the output buffer is full.
    Ok,
    /// Indicates that forward progress is not possible due to input or output
    /// buffers being empty.
    ///
    /// For compression it means the input buffer needs some more data or the
    /// output buffer needs to be freed up before trying again.
    ///
    /// For decompression this means that more input is needed to continue or
    /// the output buffer isn't large enough to contain the result. The function
    /// can be called again after fixing both.
    BufError,
    /// Indicates that all input has been consumed and all output bytes have
    /// been written. Decompression/compression should not be called again.
    ///
    /// For decompression with zlib streams the adler-32 of the decompressed
    /// data has also been verified.
    StreamEnd,
}
impl Compress {
    /// Creates a new object ready for compressing data that it's given.
    ///
    /// The `level` argument here indicates what level of compression is going
    /// to be performed, and the `zlib_header` argument indicates whether the
    /// output data should have a zlib header or not.
    pub fn new(level: Compression, zlib_header: bool) -> Compress {
        Compress {
            inner: Deflate::make(level, zlib_header, ffi::MZ_DEFAULT_WINDOW_BITS as u8),
        }
    }
    /// Creates a new object ready for compressing data that it's given.
    ///
    /// The `level` argument here indicates what level of compression is going
    /// to be performed, and the `zlib_header` argument indicates whether the
    /// output data should have a zlib header or not. The `window_bits` parameter
    /// indicates the base-2 logarithm of the sliding window size and must be
    /// between 9 and 15.
    ///
    /// # Panics
    ///
    /// If `window_bits` does not fall into the range 9 ..= 15,
    /// `new_with_window_bits` will panic.
    ///
    /// # Note
    ///
    /// This constructor is only available when the `zlib` feature is used.
    /// Other backends currently do not support custom window bits.
    #[cfg(feature = "any_zlib")]
    pub fn new_with_window_bits(
        level: Compression,
        zlib_header: bool,
        window_bits: u8,
    ) -> Compress {
        Compress {
            inner: Deflate::make(level, zlib_header, window_bits),
        }
    }
    /// Returns the total number of input bytes which have been processed by
    /// this compression object.
    pub fn total_in(&self) -> u64 {
        self.inner.total_in()
    }
    /// Returns the total number of output bytes which have been produced by
    /// this compression object.
    pub fn total_out(&self) -> u64 {
        self.inner.total_out()
    }
    /// Specifies the compression dictionary to use.
    ///
    /// Returns the Adler-32 checksum of the dictionary.
    #[cfg(feature = "any_zlib")]
    pub fn set_dictionary(&mut self, dictionary: &[u8]) -> Result<u32, CompressError> {
        let stream = &mut *self.inner.inner.stream_wrapper;
        let rc = unsafe {
            assert!(dictionary.len() < ffi::uInt::max_value() as usize);
            ffi::deflateSetDictionary(
                stream,
                dictionary.as_ptr(),
                dictionary.len() as ffi::uInt,
            )
        };
        match rc {
            ffi::MZ_STREAM_ERROR => Err(CompressError(())),
            ffi::MZ_OK => Ok(stream.adler as u32),
            c => panic!("unknown return code: {}", c),
        }
    }
    /// Quickly resets this compressor without having to reallocate anything.
    ///
    /// This is equivalent to dropping this object and then creating a new one.
    pub fn reset(&mut self) {
        self.inner.reset();
    }
    /// Dynamically updates the compression level.
    ///
    /// This can be used to switch between compression levels for different
    /// kinds of data, or it can be used in conjunction with a call to reset
    /// to reuse the compressor.
    ///
    /// This may return an error if there wasn't enough output space to complete
    /// the compression of the available input data before changing the
    /// compression level. Flushing the stream before calling this method
    /// ensures that the function will succeed on the first call.
    #[cfg(feature = "any_zlib")]
    pub fn set_level(&mut self, level: Compression) -> Result<(), CompressError> {
        use libc::c_int;
        let stream = &mut *self.inner.inner.stream_wrapper;
        let rc = unsafe {
            ffi::deflateParams(stream, level.0 as c_int, ffi::MZ_DEFAULT_STRATEGY)
        };
        match rc {
            ffi::MZ_OK => Ok(()),
            ffi::MZ_BUF_ERROR => Err(CompressError(())),
            c => panic!("unknown return code: {}", c),
        }
    }
    /// Compresses the input data into the output, consuming only as much
    /// input as needed and writing as much output as possible.
    ///
    /// The flush option can be any of the available `FlushCompress` parameters.
    ///
    /// To learn how much data was consumed or how much output was produced, use
    /// the `total_in` and `total_out` functions before/after this is called.
    pub fn compress(
        &mut self,
        input: &[u8],
        output: &mut [u8],
        flush: FlushCompress,
    ) -> Result<Status, CompressError> {
        self.inner.compress(input, output, flush)
    }
    /// Compresses the input data into the extra space of the output, consuming
    /// only as much input as needed and writing as much output as possible.
    ///
    /// This function has the same semantics as `compress`, except that the
    /// length of `vec` is managed by this function. This will not reallocate
    /// the vector provided or attempt to grow it, so space for the output must
    /// be reserved in the output vector by the caller before calling this
    /// function.
    pub fn compress_vec(
        &mut self,
        input: &[u8],
        output: &mut Vec<u8>,
        flush: FlushCompress,
    ) -> Result<Status, CompressError> {
        let cap = output.capacity();
        let len = output.len();
        unsafe {
            let before = self.total_out();
            let ret = {
                let ptr = output.as_mut_ptr().offset(len as isize);
                let out = slice::from_raw_parts_mut(ptr, cap - len);
                self.compress(input, out, flush)
            };
            output.set_len((self.total_out() - before) as usize + len);
            return ret;
        }
    }
}
impl Decompress {
    /// Creates a new object ready for decompressing data that it's given.
    ///
    /// The `zlib_header` argument indicates whether the input data is expected
    /// to have a zlib header or not.
    pub fn new(zlib_header: bool) -> Decompress {
        Decompress {
            inner: Inflate::make(zlib_header, ffi::MZ_DEFAULT_WINDOW_BITS as u8),
        }
    }
    /// Creates a new object ready for decompressing data that it's given.
    ///
    /// The `zlib_header` argument indicates whether the input data is expected
    /// to have a zlib header or not. The `window_bits` parameter indicates the
    /// base-2 logarithm of the sliding window size and must be between 9 and 15.
    ///
    /// # Panics
    ///
    /// If `window_bits` does not fall into the range 9 ..= 15,
    /// `new_with_window_bits` will panic.
    ///
    /// # Note
    ///
    /// This constructor is only available when the `zlib` feature is used.
    /// Other backends currently do not support custom window bits.
    #[cfg(feature = "any_zlib")]
    pub fn new_with_window_bits(zlib_header: bool, window_bits: u8) -> Decompress {
        Decompress {
            inner: Inflate::make(zlib_header, window_bits),
        }
    }
    /// Returns the total number of input bytes which have been processed by
    /// this decompression object.
    pub fn total_in(&self) -> u64 {
        self.inner.total_in()
    }
    /// Returns the total number of output bytes which have been produced by
    /// this decompression object.
    pub fn total_out(&self) -> u64 {
        self.inner.total_out()
    }
    /// Decompresses the input data into the output, consuming only as much
    /// input as needed and writing as much output as possible.
    ///
    /// The flush option can be any of the available `FlushDecompress` parameters.
    ///
    /// If the first call passes `FlushDecompress::Finish` it is assumed that
    /// the input and output buffers are both sized large enough to decompress
    /// the entire stream in a single call.
    ///
    /// A flush value of `FlushDecompress::Finish` indicates that there are no
    /// more source bytes available beside what's already in the input buffer,
    /// and the output buffer is large enough to hold the rest of the
    /// decompressed data.
    ///
    /// To learn how much data was consumed or how much output was produced, use
    /// the `total_in` and `total_out` functions before/after this is called.
    ///
    /// # Errors
    ///
    /// If the input data to this instance of `Decompress` is not a valid
    /// zlib/deflate stream then this function may return an instance of
    /// `DecompressError` to indicate that the stream of input bytes is corrupted.
    pub fn decompress(
        &mut self,
        input: &[u8],
        output: &mut [u8],
        flush: FlushDecompress,
    ) -> Result<Status, DecompressError> {
        self.inner.decompress(input, output, flush)
    }
    /// Decompresses the input data into the extra space in the output vector
    /// specified by `output`.
    ///
    /// This function has the same semantics as `decompress`, except that the
    /// length of `vec` is managed by this function. This will not reallocate
    /// the vector provided or attempt to grow it, so space for the output must
    /// be reserved in the output vector by the caller before calling this
    /// function.
    ///
    /// # Errors
    ///
    /// If the input data to this instance of `Decompress` is not a valid
    /// zlib/deflate stream then this function may return an instance of
    /// `DecompressError` to indicate that the stream of input bytes is corrupted.
    pub fn decompress_vec(
        &mut self,
        input: &[u8],
        output: &mut Vec<u8>,
        flush: FlushDecompress,
    ) -> Result<Status, DecompressError> {
        let cap = output.capacity();
        let len = output.len();
        unsafe {
            let before = self.total_out();
            let ret = {
                let ptr = output.as_mut_ptr().offset(len as isize);
                let out = slice::from_raw_parts_mut(ptr, cap - len);
                self.decompress(input, out, flush)
            };
            output.set_len((self.total_out() - before) as usize + len);
            return ret;
        }
    }
    /// Specifies the decompression dictionary to use.
    #[cfg(feature = "any_zlib")]
    pub fn set_dictionary(&mut self, dictionary: &[u8]) -> Result<u32, DecompressError> {
        let stream = &mut *self.inner.inner.stream_wrapper;
        let rc = unsafe {
            assert!(dictionary.len() < ffi::uInt::max_value() as usize);
            ffi::inflateSetDictionary(
                stream,
                dictionary.as_ptr(),
                dictionary.len() as ffi::uInt,
            )
        };
        match rc {
            ffi::MZ_STREAM_ERROR => Err(DecompressError(Default::default())),
            ffi::MZ_DATA_ERROR => {
                Err(
                    DecompressError(DecompressErrorInner {
                        needs_dictionary: Some(stream.adler as u32),
                    }),
                )
            }
            ffi::MZ_OK => Ok(stream.adler as u32),
            c => panic!("unknown return code: {}", c),
        }
    }
    /// Performs the equivalent of replacing this decompression state with a
    /// freshly allocated copy.
    ///
    /// This function may not allocate memory, though, and attempts to reuse any
    /// previously existing resources.
    ///
    /// The argument provided here indicates whether the reset state will
    /// attempt to decode a zlib header first or not.
    pub fn reset(&mut self, zlib_header: bool) {
        self.inner.reset(zlib_header);
    }
}
impl Error for DecompressError {}
impl From<DecompressError> for io::Error {
    fn from(data: DecompressError) -> io::Error {
        io::Error::new(io::ErrorKind::Other, data)
    }
}
impl fmt::Display for DecompressError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "deflate decompression error")
    }
}
impl Error for CompressError {}
impl From<CompressError> for io::Error {
    fn from(data: CompressError) -> io::Error {
        io::Error::new(io::ErrorKind::Other, data)
    }
}
impl fmt::Display for CompressError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "deflate decompression error")
    }
}
#[cfg(test)]
mod tests {
    use std::io::Write;
    use crate::write;
    use crate::{Compression, Decompress, FlushDecompress};
    #[cfg(feature = "any_zlib")]
    use crate::{Compress, FlushCompress};
    #[test]
    fn issue51() {
        let data = vec![
            0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0xb3, 0xc9, 0x28,
            0xc9, 0xcd, 0xb1, 0xe3, 0xe5, 0xb2, 0xc9, 0x48, 0x4d, 0x4c, 0xb1, 0xb3, 0x29,
            0xc9, 0x2c, 0xc9, 0x49, 0xb5, 0x33, 0x31, 0x30, 0x51, 0xf0, 0xcb, 0x2f, 0x51,
            0x70, 0xcb, 0x2f, 0xcd, 0x4b, 0xb1, 0xd1, 0x87, 0x08, 0xda, 0xe8, 0x83, 0x95,
            0x00, 0x95, 0x26, 0xe5, 0xa7, 0x54, 0x2a, 0x24, 0xa5, 0x27, 0xe7, 0xe7, 0xe4,
            0x17, 0xd9, 0x2a, 0x95, 0x67, 0x64, 0x96, 0xa4, 0x2a, 0x81, 0x8c, 0x48, 0x4e,
            0xcd, 0x2b, 0x49, 0x2d, 0xb2, 0xb3, 0xc9, 0x30, 0x44, 0x37, 0x01, 0x28, 0x62,
            0xa3, 0x0f, 0x95, 0x06, 0xd9, 0x05, 0x54, 0x04, 0xe5, 0xe5, 0xa5, 0x67, 0xe6,
            0x55, 0xe8, 0x1b, 0xea, 0x99, 0xe9, 0x19, 0x21, 0xab, 0xd0, 0x07, 0xd9, 0x01,
            0x32, 0x53, 0x1f, 0xea, 0x3e, 0x00, 0x94, 0x85, 0xeb, 0xe4, 0xa8, 0x00, 0x00,
            0x00,
        ];
        let mut decoded = Vec::with_capacity(data.len() * 2);
        let mut d = Decompress::new(false);
        assert!(
            d.decompress_vec(& data[10..], & mut decoded, FlushDecompress::Finish)
            .is_ok()
        );
        drop(d.decompress_vec(&[0], &mut decoded, FlushDecompress::None));
    }
    #[test]
    fn reset() {
        let string = "hello world".as_bytes();
        let mut zlib = Vec::new();
        let mut deflate = Vec::new();
        let comp = Compression::default();
        write::ZlibEncoder::new(&mut zlib, comp).write_all(string).unwrap();
        write::DeflateEncoder::new(&mut deflate, comp).write_all(string).unwrap();
        let mut dst = [0; 1024];
        let mut decoder = Decompress::new(true);
        decoder.decompress(&zlib, &mut dst, FlushDecompress::Finish).unwrap();
        assert_eq!(decoder.total_out(), string.len() as u64);
        assert!(dst.starts_with(string));
        decoder.reset(false);
        decoder.decompress(&deflate, &mut dst, FlushDecompress::Finish).unwrap();
        assert_eq!(decoder.total_out(), string.len() as u64);
        assert!(dst.starts_with(string));
    }
    #[cfg(feature = "any_zlib")]
    #[test]
    fn set_dictionary_with_zlib_header() {
        let string = "hello, hello!".as_bytes();
        let dictionary = "hello".as_bytes();
        let mut encoded = Vec::with_capacity(1024);
        let mut encoder = Compress::new(Compression::default(), true);
        let dictionary_adler = encoder.set_dictionary(&dictionary).unwrap();
        encoder.compress_vec(string, &mut encoded, FlushCompress::Finish).unwrap();
        assert_eq!(encoder.total_in(), string.len() as u64);
        assert_eq!(encoder.total_out(), encoded.len() as u64);
        let mut decoder = Decompress::new(true);
        let mut decoded = [0; 1024];
        let decompress_error = decoder
            .decompress(&encoded, &mut decoded, FlushDecompress::Finish)
            .expect_err("decompression should fail due to requiring a dictionary");
        let required_adler = decompress_error
            .needs_dictionary()
            .expect(
                "the first call to decompress should indicate a dictionary is required along with the required Adler-32 checksum",
            );
        assert_eq!(
            required_adler, dictionary_adler,
            "the Adler-32 checksum should match the value when the dictionary was set on the compressor"
        );
        let actual_adler = decoder.set_dictionary(&dictionary).unwrap();
        assert_eq!(required_adler, actual_adler);
        let total_in = decoder.total_in();
        let total_out = decoder.total_out();
        let decompress_result = decoder
            .decompress(
                &encoded[total_in as usize..],
                &mut decoded[total_out as usize..],
                FlushDecompress::Finish,
            );
        assert!(decompress_result.is_ok());
        assert_eq!(& decoded[..decoder.total_out() as usize], string);
    }
    #[cfg(feature = "any_zlib")]
    #[test]
    fn set_dictionary_raw() {
        let string = "hello, hello!".as_bytes();
        let dictionary = "hello".as_bytes();
        let mut encoded = Vec::with_capacity(1024);
        let mut encoder = Compress::new(Compression::default(), false);
        encoder.set_dictionary(&dictionary).unwrap();
        encoder.compress_vec(string, &mut encoded, FlushCompress::Finish).unwrap();
        assert_eq!(encoder.total_in(), string.len() as u64);
        assert_eq!(encoder.total_out(), encoded.len() as u64);
        let mut decoder = Decompress::new(false);
        decoder.set_dictionary(&dictionary).unwrap();
        let mut decoded = [0; 1024];
        let decompress_result = decoder
            .decompress(&encoded, &mut decoded, FlushDecompress::Finish);
        assert!(decompress_result.is_ok());
        assert_eq!(& decoded[..decoder.total_out() as usize], string);
    }
}
#[cfg(test)]
mod tests_llm_16_412 {
    use super::*;
    use crate::*;
    use crate::mem::CompressError;
    use std::io::{Error, ErrorKind};
    #[test]
    fn test_from() {
        let _rug_st_tests_llm_16_412_rrrruuuugggg_test_from = 0;
        let compress_error = CompressError(());
        let result: Error = From::from(compress_error);
        debug_assert_eq!(result.kind(), ErrorKind::Other);
        let _rug_ed_tests_llm_16_412_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_413 {
    use crate::mem::{DecompressError, DecompressErrorInner};
    use std::io::{self, Error, ErrorKind};
    #[test]
    fn test_from() {
        let _rug_st_tests_llm_16_413_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = 1234;
        let decompress_error_inner = DecompressErrorInner {
            needs_dictionary: Some(rug_fuzz_0),
        };
        let decompress_error = DecompressError(decompress_error_inner);
        let result: Error = From::from(decompress_error);
        debug_assert_eq!(result.kind(), ErrorKind::Other);
        let _rug_ed_tests_llm_16_413_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_416 {
    use super::*;
    use crate::*;
    use crate::Compression;
    #[test]
    fn test_compress_vec() {
        let _rug_st_tests_llm_16_416_rrrruuuugggg_test_compress_vec = 0;
        let rug_fuzz_0 = false;
        let rug_fuzz_1 = b"Hello, world!";
        let rug_fuzz_2 = 1024;
        let mut compressor = Compress::new(Compression::default(), rug_fuzz_0);
        let input = rug_fuzz_1;
        let mut output = Vec::with_capacity(rug_fuzz_2);
        let result = compressor.compress_vec(input, &mut output, FlushCompress::Finish);
        debug_assert!(result.is_ok());
        let _rug_ed_tests_llm_16_416_rrrruuuugggg_test_compress_vec = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_419 {
    use super::*;
    use crate::*;
    use crate::Compression;
    #[test]
    fn test_reset() {
        let _rug_st_tests_llm_16_419_rrrruuuugggg_test_reset = 0;
        let rug_fuzz_0 = false;
        let rug_fuzz_1 = b"test";
        let mut compressor = Compress::new(Compression::fast(), rug_fuzz_0);
        let input = rug_fuzz_1;
        let mut output = Vec::new();
        let flush = FlushCompress::Finish;
        let result = compressor.compress(input, &mut output, flush);
        debug_assert!(result.is_ok());
        let total_in_before = compressor.total_in();
        let total_out_before = compressor.total_out();
        compressor.reset();
        let total_in_after = compressor.total_in();
        let total_out_after = compressor.total_out();
        debug_assert_eq!(total_in_before, total_in_after);
        debug_assert_eq!(total_out_before, total_out_after);
        let _rug_ed_tests_llm_16_419_rrrruuuugggg_test_reset = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_420 {
    use crate::{Compression, FlushCompress};
    use crate::mem::Compress;
    #[test]
    fn test_total_in() {
        let _rug_st_tests_llm_16_420_rrrruuuugggg_test_total_in = 0;
        let rug_fuzz_0 = b"Hello, world!";
        let rug_fuzz_1 = false;
        let data = rug_fuzz_0;
        let mut compressor = Compress::new(Compression::fast(), rug_fuzz_1);
        let mut output = Vec::new();
        let _ = compressor.compress(data, &mut output, FlushCompress::Finish);
        let total_in = compressor.total_in();
        debug_assert_eq!(total_in, data.len() as u64);
        let _rug_ed_tests_llm_16_420_rrrruuuugggg_test_total_in = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_421 {
    use super::*;
    use crate::*;
    use crate::{Compress, Compression};
    #[test]
    fn test_total_out() {
        let _rug_st_tests_llm_16_421_rrrruuuugggg_test_total_out = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 3;
        let rug_fuzz_4 = 4;
        let rug_fuzz_5 = 5;
        let rug_fuzz_6 = 0;
        let level = Compression::default();
        let zlib_header = rug_fuzz_0;
        let mut compressor = Compress::new(level, zlib_header);
        let input = [rug_fuzz_1, rug_fuzz_2, rug_fuzz_3, rug_fuzz_4, rug_fuzz_5];
        let mut output = [rug_fuzz_6; 10];
        let flush = crate::FlushCompress::Finish;
        compressor.compress(&input, &mut output, flush).unwrap();
        debug_assert_eq!(compressor.total_out(), 10);
        let _rug_ed_tests_llm_16_421_rrrruuuugggg_test_total_out = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_425 {
    use super::*;
    use crate::*;
    use crate::Compression;
    #[test]
    fn test_decompress_vec() {
        let _rug_st_tests_llm_16_425_rrrruuuugggg_test_decompress_vec = 0;
        let rug_fuzz_0 = 120;
        let rug_fuzz_1 = true;
        let data = vec![
            rug_fuzz_0, 156, 243, 72, 205, 201, 201, 215, 81, 40, 207, 47, 202, 73, 1, 0,
            0, 255, 255
        ];
        let mut decompressor = Decompress::new(rug_fuzz_1);
        let mut output = vec![0; 20];
        let result = decompressor
            .decompress_vec(&data, &mut output, FlushDecompress::Finish);
        debug_assert_eq!(result.unwrap(), Status::StreamEnd);
        let _rug_ed_tests_llm_16_425_rrrruuuugggg_test_decompress_vec = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_429_llm_16_428 {
    use super::*;
    use crate::*;
    use crate::mem::{DecompressError, FlushDecompress, Status};
    #[test]
    fn test_reset() {
        let _rug_st_tests_llm_16_429_llm_16_428_rrrruuuugggg_test_reset = 0;
        let rug_fuzz_0 = false;
        let rug_fuzz_1 = true;
        let mut decompress = mem::Decompress::new(rug_fuzz_0);
        decompress.reset(rug_fuzz_1);
        debug_assert_eq!(decompress.total_in(), 0);
        debug_assert_eq!(decompress.total_out(), 0);
        let _rug_ed_tests_llm_16_429_llm_16_428_rrrruuuugggg_test_reset = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_431 {
    use super::*;
    use crate::*;
    use crate::{Compression, Decompress};
    #[test]
    fn test_total_in() {
        let _rug_st_tests_llm_16_431_rrrruuuugggg_test_total_in = 0;
        let rug_fuzz_0 = false;
        let rug_fuzz_1 = b"test input";
        let rug_fuzz_2 = 0;
        let mut decompressor = Decompress::new(rug_fuzz_0);
        let input = rug_fuzz_1;
        let output = &mut [rug_fuzz_2; 1024];
        decompressor.decompress(input, output, crate::FlushDecompress::None).unwrap();
        let total_in = decompressor.total_in();
        debug_assert_eq!(total_in, input.len() as u64);
        let _rug_ed_tests_llm_16_431_rrrruuuugggg_test_total_in = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_432 {
    use crate::zlib;
    use crate::Decompress;
    use crate::FlushDecompress;
    #[test]
    fn test_total_out() {
        let _rug_st_tests_llm_16_432_rrrruuuugggg_test_total_out = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = true;
        let mut input = [rug_fuzz_0; 10];
        let mut output = [rug_fuzz_1; 20];
        let mut decompress = Decompress::new(rug_fuzz_2);
        let flush = FlushDecompress::None;
        let result = decompress.decompress(&input, &mut output, flush);
        debug_assert!(result.is_ok());
        let total_out = decompress.total_out();
        debug_assert_eq!(total_out, 0u64);
        let _rug_ed_tests_llm_16_432_rrrruuuugggg_test_total_out = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_437 {
    use crate::mem::{
        decompress_need_dict, DecompressError, DecompressErrorInner, Status,
    };
    #[test]
    fn test_decompress_need_dict() {
        let _rug_st_tests_llm_16_437_rrrruuuugggg_test_decompress_need_dict = 0;
        let rug_fuzz_0 = 12345;
        let adler = rug_fuzz_0;
        let result = decompress_need_dict(adler);
        match result {
            Err(DecompressError(DecompressErrorInner { needs_dictionary })) => {
                debug_assert_eq!(needs_dictionary, Some(adler));
            }
            _ => {
                panic!(
                    "Expected Err(DecompressError(DecompressErrorInner {{ needs_dictionary }})), got {:?}",
                    result
                )
            }
        }
        let _rug_ed_tests_llm_16_437_rrrruuuugggg_test_decompress_need_dict = 0;
    }
}
#[cfg(test)]
mod tests_rug_74 {
    use super::*;
    use crate::mem::DecompressError;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_74_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 12345;
        let p0 = DecompressError(DecompressErrorInner {
            needs_dictionary: Some(rug_fuzz_0),
        });
        debug_assert_eq!(p0.needs_dictionary(), Some(12345));
        let _rug_ed_tests_rug_74_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_75 {
    use super::*;
    use crate::Compression;
    #[test]
    fn test_compression() {
        let _rug_st_tests_rug_75_rrrruuuugggg_test_compression = 0;
        let rug_fuzz_0 = true;
        let mut p0 = Compression::best();
        let p1 = rug_fuzz_0;
        crate::mem::Compress::new(p0, p1);
        let _rug_ed_tests_rug_75_rrrruuuugggg_test_compression = 0;
    }
}
#[cfg(test)]
mod tests_rug_77 {
    use super::*;
    use crate::mem::Decompress;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_77_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = true;
        let mut p0: bool = rug_fuzz_0;
        Decompress::new(p0);
        let _rug_ed_tests_rug_77_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_78 {
    use super::*;
    use crate::Decompress;
    use crate::FlushDecompress;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_78_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = false;
        let rug_fuzz_1 = 0u8;
        let rug_fuzz_2 = 1u8;
        let rug_fuzz_3 = 2u8;
        let rug_fuzz_4 = 0u8;
        let mut p0 = Decompress::new(rug_fuzz_0);
        let mut p1 = &[rug_fuzz_1, rug_fuzz_2, rug_fuzz_3];
        let mut p2 = [rug_fuzz_4; 10];
        let mut p3 = FlushDecompress::Finish;
        p0.decompress(p1, &mut p2, p3);
        let _rug_ed_tests_rug_78_rrrruuuugggg_test_rug = 0;
    }
}
