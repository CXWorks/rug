//! A DEFLATE-based stream compression/decompression library
//!
//! This library provides support for compression and decompression of
//! DEFLATE-based streams:
//!
//! * the DEFLATE format itself
//! * the zlib format
//! * gzip
//!
//! These three formats are all closely related and largely only differ in their
//! headers/footers. This crate has three types in each submodule for dealing
//! with these three formats.
//!
//! # Implementation
//!
//! In addition to supporting three formats, this crate supports three different
//! backends, controlled through this crate's features:
//!
//! * `default`, or `rust_backend` - this implementation uses the `miniz_oxide`
//!   crate which is a port of `miniz.c` (below) to Rust. This feature does not
//!   require a C compiler and only requires Rust code.
//!
//! * `miniz-sys` - when enabled this feature will enable this crate to instead
//!   use `miniz.c`, distributed with `miniz-sys`, to implement
//!   compression/decompression.
//!
//! * `zlib` - finally, this feature will enable linking against the `libz`
//!   library, typically found on most Linux systems by default. If the library
//!   isn't found to already be on the system it will be compiled from source
//!   (this is a C library).
//!
//! There's various tradeoffs associated with each implementation, but in
//! general you probably won't have to tweak the defaults. The default choice is
//! selected to avoid the need for a C compiler at build time. The `miniz-sys`
//! feature is largely a historical artifact at this point and is unlikely to be
//! needed, and `zlib` is often useful if you're already using `zlib` for other
//! C dependencies. The compression ratios and performance of each of these
//! feature should be roughly comparable, but you'll likely want to run your own
//! tests if you're curious about the performance.
//!
//! # Organization
//!
//! This crate consists mainly of three modules, [`read`], [`write`], and
//! [`bufread`]. Each module contains a number of types used to encode and
//! decode various streams of data.
//!
//! All types in the [`write`] module work on instances of [`Write`][write],
//! whereas all types in the [`read`] module work on instances of
//! [`Read`][read] and [`bufread`] works with [`BufRead`][bufread]. If you
//! are decoding directly from a `&[u8]`, use the [`bufread`] types.
//!
//! ```
//! use flate2::write::GzEncoder;
//! use flate2::Compression;
//! use std::io;
//! use std::io::prelude::*;
//!
//! # fn main() { let _ = run(); }
//! # fn run() -> io::Result<()> {
//! let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
//! encoder.write_all(b"Example")?;
//! # Ok(())
//! # }
//! ```
//!
//!
//! Other various types are provided at the top-level of the crate for
//! management and dealing with encoders/decoders. Also note that types which
//! operate over a specific trait often implement the mirroring trait as well.
//! For example a `flate2::read::DeflateDecoder<T>` *also* implements the
//! `Write` trait if `T: Write`. That is, the "dual trait" is forwarded directly
//! to the underlying object if available.
//!
//! [`read`]: read/index.html
//! [`bufread`]: bufread/index.html
//! [`write`]: write/index.html
//! [read]: https://doc.rust-lang.org/std/io/trait.Read.html
//! [write]: https://doc.rust-lang.org/std/io/trait.Write.html
//! [bufread]: https://doc.rust-lang.org/std/io/trait.BufRead.html
//!
//! # Async I/O
//!
//! This crate optionally can support async I/O streams with the [Tokio stack] via
//! the `tokio` feature of this crate:
//!
//! [Tokio stack]: https://tokio.rs/
//!
//! ```toml
//! flate2 = { version = "0.2", features = ["tokio"] }
//! ```
//!
//! All methods are internally capable of working with streams that may return
//! [`ErrorKind::WouldBlock`] when they're not ready to perform the particular
//! operation.
//!
//! [`ErrorKind::WouldBlock`]: https://doc.rust-lang.org/std/io/enum.ErrorKind.html
//!
//! Note that care needs to be taken when using these objects, however. The
//! Tokio runtime, in particular, requires that data is fully flushed before
//! dropping streams. For compatibility with blocking streams all streams are
//! flushed/written when they are dropped, and this is not always a suitable
//! time to perform I/O. If I/O streams are flushed before drop, however, then
//! these operations will be a noop.
#![doc(html_root_url = "https://docs.rs/flate2/0.2")]
#![allow(trivial_numeric_casts)]
pub use crate::crc::{Crc, CrcReader, CrcWriter};
pub use crate::gz::GzBuilder;
pub use crate::gz::GzHeader;
pub use crate::mem::{Compress, CompressError, Decompress, DecompressError, Status};
pub use crate::mem::{FlushCompress, FlushDecompress};
mod bufreader;
mod crc;
mod deflate;
mod ffi;
mod gz;
mod mem;
mod zio;
mod zlib;
/// Types which operate over [`Read`] streams, both encoders and decoders for
/// various formats.
///
/// [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
pub mod read {
    pub use crate::deflate::read::DeflateDecoder;
    pub use crate::deflate::read::DeflateEncoder;
    pub use crate::gz::read::GzDecoder;
    pub use crate::gz::read::GzEncoder;
    pub use crate::gz::read::MultiGzDecoder;
    pub use crate::zlib::read::ZlibDecoder;
    pub use crate::zlib::read::ZlibEncoder;
}
/// Types which operate over [`Write`] streams, both encoders and decoders for
/// various formats.
///
/// [`Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
pub mod write {
    pub use crate::deflate::write::DeflateDecoder;
    pub use crate::deflate::write::DeflateEncoder;
    pub use crate::gz::write::GzDecoder;
    pub use crate::gz::write::GzEncoder;
    pub use crate::zlib::write::ZlibDecoder;
    pub use crate::zlib::write::ZlibEncoder;
}
/// Types which operate over [`BufRead`] streams, both encoders and decoders for
/// various formats.
///
/// [`BufRead`]: https://doc.rust-lang.org/std/io/trait.BufRead.html
pub mod bufread {
    pub use crate::deflate::bufread::DeflateDecoder;
    pub use crate::deflate::bufread::DeflateEncoder;
    pub use crate::gz::bufread::GzDecoder;
    pub use crate::gz::bufread::GzEncoder;
    pub use crate::gz::bufread::MultiGzDecoder;
    pub use crate::zlib::bufread::ZlibDecoder;
    pub use crate::zlib::bufread::ZlibEncoder;
}
fn _assert_send_sync() {
    fn _assert_send_sync<T: Send + Sync>() {}
    _assert_send_sync::<read::DeflateEncoder<&[u8]>>();
    _assert_send_sync::<read::DeflateDecoder<&[u8]>>();
    _assert_send_sync::<read::ZlibEncoder<&[u8]>>();
    _assert_send_sync::<read::ZlibDecoder<&[u8]>>();
    _assert_send_sync::<read::GzEncoder<&[u8]>>();
    _assert_send_sync::<read::GzDecoder<&[u8]>>();
    _assert_send_sync::<read::MultiGzDecoder<&[u8]>>();
    _assert_send_sync::<write::DeflateEncoder<Vec<u8>>>();
    _assert_send_sync::<write::DeflateDecoder<Vec<u8>>>();
    _assert_send_sync::<write::ZlibEncoder<Vec<u8>>>();
    _assert_send_sync::<write::ZlibDecoder<Vec<u8>>>();
    _assert_send_sync::<write::GzEncoder<Vec<u8>>>();
    _assert_send_sync::<write::GzDecoder<Vec<u8>>>();
}
/// When compressing data, the compression level can be specified by a value in
/// this enum.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Compression(u32);
impl Compression {
    /// Creates a new description of the compression level with an explicitly
    /// specified integer.
    ///
    /// The integer here is typically on a scale of 0-9 where 0 means "no
    /// compression" and 9 means "take as long as you'd like".
    pub const fn new(level: u32) -> Compression {
        Compression(level)
    }
    /// No compression is to be performed, this may actually inflate data
    /// slightly when encoding.
    pub const fn none() -> Compression {
        Compression(0)
    }
    /// Optimize for the best speed of encoding.
    pub const fn fast() -> Compression {
        Compression(1)
    }
    /// Optimize for the size of data being encoded.
    pub const fn best() -> Compression {
        Compression(9)
    }
    /// Returns an integer representing the compression level, typically on a
    /// scale of 0-9
    pub fn level(&self) -> u32 {
        self.0
    }
}
impl Default for Compression {
    fn default() -> Compression {
        Compression(6)
    }
}
#[cfg(test)]
fn random_bytes() -> impl Iterator<Item = u8> {
    use rand::Rng;
    use std::iter;
    iter::repeat(()).map(|_| rand::thread_rng().gen())
}
#[cfg(test)]
mod tests_llm_16_1 {
    use crate::Compression;
    use std::default::Default;
    #[test]
    fn test_default() {
        let _rug_st_tests_llm_16_1_rrrruuuugggg_test_default = 0;
        let result: Compression = <Compression as Default>::default();
        debug_assert_eq!(result.level(), 6);
        let _rug_ed_tests_llm_16_1_rrrruuuugggg_test_default = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_177 {
    use crate::Compression;
    #[test]
    fn test_best() {
        let _rug_st_tests_llm_16_177_rrrruuuugggg_test_best = 0;
        let best = Compression::best();
        debug_assert_eq!(best.level(), 9);
        let _rug_ed_tests_llm_16_177_rrrruuuugggg_test_best = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_178 {
    use crate::Compression;
    #[test]
    fn test_fast() {
        let _rug_st_tests_llm_16_178_rrrruuuugggg_test_fast = 0;
        let compression = Compression::fast();
        debug_assert_eq!(compression.level(), 1);
        let _rug_ed_tests_llm_16_178_rrrruuuugggg_test_fast = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_179 {
    use crate::Compression;
    #[test]
    fn test_level() {
        let _rug_st_tests_llm_16_179_rrrruuuugggg_test_level = 0;
        let rug_fuzz_0 = 6;
        let compression = Compression::new(rug_fuzz_0);
        debug_assert_eq!(compression.level(), 6);
        let _rug_ed_tests_llm_16_179_rrrruuuugggg_test_level = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_180 {
    use super::*;
    use crate::*;
    use std::cmp::PartialEq;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_180_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = 6;
        let compression = Compression::new(rug_fuzz_0);
        debug_assert_eq!(compression.level(), 6);
        let _rug_ed_tests_llm_16_180_rrrruuuugggg_test_new = 0;
    }
    #[test]
    fn test_none() {
        let _rug_st_tests_llm_16_180_rrrruuuugggg_test_none = 0;
        let compression = Compression::none();
        debug_assert_eq!(compression.level(), 0);
        let _rug_ed_tests_llm_16_180_rrrruuuugggg_test_none = 0;
    }
    #[test]
    fn test_fast() {
        let _rug_st_tests_llm_16_180_rrrruuuugggg_test_fast = 0;
        let compression = Compression::fast();
        debug_assert_eq!(compression.level(), 1);
        let _rug_ed_tests_llm_16_180_rrrruuuugggg_test_fast = 0;
    }
    #[test]
    fn test_best() {
        let _rug_st_tests_llm_16_180_rrrruuuugggg_test_best = 0;
        let compression = Compression::best();
        debug_assert_eq!(compression.level(), 9);
        let _rug_ed_tests_llm_16_180_rrrruuuugggg_test_best = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_181 {
    use crate::Compression;
    #[test]
    fn test_none() {
        let _rug_st_tests_llm_16_181_rrrruuuugggg_test_none = 0;
        let compression = Compression::none();
        debug_assert_eq!(compression.level(), 0);
        let _rug_ed_tests_llm_16_181_rrrruuuugggg_test_none = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_182 {
    use crate::read;
    use crate::write;
    #[test]
    fn test_assert_send_sync() {
        let _rug_st_tests_llm_16_182_rrrruuuugggg__assert_send_sync = 0;
        fn _assert_send_sync<T: Send + Sync>() {
            let _rug_st_tests_llm_16_182_rrrruuuugggg__assert_send_sync = 0;
            let _rug_ed_tests_llm_16_182_rrrruuuugggg__assert_send_sync = 0;
        }
        _assert_send_sync::<read::DeflateEncoder<&[u8]>>();
        _assert_send_sync::<read::DeflateDecoder<&[u8]>>();
        _assert_send_sync::<read::ZlibEncoder<&[u8]>>();
        _assert_send_sync::<read::ZlibDecoder<&[u8]>>();
        _assert_send_sync::<read::GzEncoder<&[u8]>>();
        _assert_send_sync::<read::GzDecoder<&[u8]>>();
        _assert_send_sync::<read::MultiGzDecoder<&[u8]>>();
        _assert_send_sync::<write::DeflateEncoder<Vec<u8>>>();
        _assert_send_sync::<write::DeflateDecoder<Vec<u8>>>();
        _assert_send_sync::<write::ZlibEncoder<Vec<u8>>>();
        _assert_send_sync::<write::ZlibDecoder<Vec<u8>>>();
        _assert_send_sync::<write::GzEncoder<Vec<u8>>>();
        _assert_send_sync::<write::GzDecoder<Vec<u8>>>();
        let _rug_ed_tests_llm_16_182_rrrruuuugggg__assert_send_sync = 0;
    }
}
