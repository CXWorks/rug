use std::ffi::CString;
use std::io::prelude::*;
use std::time;
use crate::bufreader::BufReader;
use crate::Compression;
pub static FHCRC: u8 = 1 << 1;
pub static FEXTRA: u8 = 1 << 2;
pub static FNAME: u8 = 1 << 3;
pub static FCOMMENT: u8 = 1 << 4;
pub mod bufread;
pub mod read;
pub mod write;
/// A structure representing the header of a gzip stream.
///
/// The header can contain metadata about the file that was compressed, if
/// present.
#[derive(PartialEq, Clone, Debug, Default)]
pub struct GzHeader {
    extra: Option<Vec<u8>>,
    filename: Option<Vec<u8>>,
    comment: Option<Vec<u8>>,
    operating_system: u8,
    mtime: u32,
}
impl GzHeader {
    /// Returns the `filename` field of this gzip stream's header, if present.
    pub fn filename(&self) -> Option<&[u8]> {
        self.filename.as_ref().map(|s| &s[..])
    }
    /// Returns the `extra` field of this gzip stream's header, if present.
    pub fn extra(&self) -> Option<&[u8]> {
        self.extra.as_ref().map(|s| &s[..])
    }
    /// Returns the `comment` field of this gzip stream's header, if present.
    pub fn comment(&self) -> Option<&[u8]> {
        self.comment.as_ref().map(|s| &s[..])
    }
    /// Returns the `operating_system` field of this gzip stream's header.
    ///
    /// There are predefined values for various operating systems.
    /// 255 means that the value is unknown.
    pub fn operating_system(&self) -> u8 {
        self.operating_system
    }
    /// This gives the most recent modification time of the original file being compressed.
    ///
    /// The time is in Unix format, i.e., seconds since 00:00:00 GMT, Jan. 1, 1970.
    /// (Note that this may cause problems for MS-DOS and other systems that use local
    /// rather than Universal time.) If the compressed data did not come from a file,
    /// `mtime` is set to the time at which compression started.
    /// `mtime` = 0 means no time stamp is available.
    ///
    /// The usage of `mtime` is discouraged because of Year 2038 problem.
    pub fn mtime(&self) -> u32 {
        self.mtime
    }
    /// Returns the most recent modification time represented by a date-time type.
    /// Returns `None` if the value of the underlying counter is 0,
    /// indicating no time stamp is available.
    ///
    ///
    /// The time is measured as seconds since 00:00:00 GMT, Jan. 1 1970.
    /// See [`mtime`](#method.mtime) for more detail.
    pub fn mtime_as_datetime(&self) -> Option<time::SystemTime> {
        if self.mtime == 0 {
            None
        } else {
            let duration = time::Duration::new(u64::from(self.mtime), 0);
            let datetime = time::UNIX_EPOCH + duration;
            Some(datetime)
        }
    }
}
/// A builder structure to create a new gzip Encoder.
///
/// This structure controls header configuration options such as the filename.
///
/// # Examples
///
/// ```
/// use std::io::prelude::*;
/// # use std::io;
/// use std::fs::File;
/// use flate2::GzBuilder;
/// use flate2::Compression;
///
/// // GzBuilder opens a file and writes a sample string using GzBuilder pattern
///
/// # fn sample_builder() -> Result<(), io::Error> {
/// let f = File::create("examples/hello_world.gz")?;
/// let mut gz = GzBuilder::new()
///                 .filename("hello_world.txt")
///                 .comment("test file, please delete")
///                 .write(f, Compression::default());
/// gz.write_all(b"hello world")?;
/// gz.finish()?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct GzBuilder {
    extra: Option<Vec<u8>>,
    filename: Option<CString>,
    comment: Option<CString>,
    operating_system: Option<u8>,
    mtime: u32,
}
impl GzBuilder {
    /// Create a new blank builder with no header by default.
    pub fn new() -> GzBuilder {
        GzBuilder {
            extra: None,
            filename: None,
            comment: None,
            operating_system: None,
            mtime: 0,
        }
    }
    /// Configure the `mtime` field in the gzip header.
    pub fn mtime(mut self, mtime: u32) -> GzBuilder {
        self.mtime = mtime;
        self
    }
    /// Configure the `operating_system` field in the gzip header.
    pub fn operating_system(mut self, os: u8) -> GzBuilder {
        self.operating_system = Some(os);
        self
    }
    /// Configure the `extra` field in the gzip header.
    pub fn extra<T: Into<Vec<u8>>>(mut self, extra: T) -> GzBuilder {
        self.extra = Some(extra.into());
        self
    }
    /// Configure the `filename` field in the gzip header.
    ///
    /// # Panics
    ///
    /// Panics if the `filename` slice contains a zero.
    pub fn filename<T: Into<Vec<u8>>>(mut self, filename: T) -> GzBuilder {
        self.filename = Some(CString::new(filename.into()).unwrap());
        self
    }
    /// Configure the `comment` field in the gzip header.
    ///
    /// # Panics
    ///
    /// Panics if the `comment` slice contains a zero.
    pub fn comment<T: Into<Vec<u8>>>(mut self, comment: T) -> GzBuilder {
        self.comment = Some(CString::new(comment.into()).unwrap());
        self
    }
    /// Consume this builder, creating a writer encoder in the process.
    ///
    /// The data written to the returned encoder will be compressed and then
    /// written out to the supplied parameter `w`.
    pub fn write<W: Write>(self, w: W, lvl: Compression) -> write::GzEncoder<W> {
        write::gz_encoder(self.into_header(lvl), w, lvl)
    }
    /// Consume this builder, creating a reader encoder in the process.
    ///
    /// Data read from the returned encoder will be the compressed version of
    /// the data read from the given reader.
    pub fn read<R: Read>(self, r: R, lvl: Compression) -> read::GzEncoder<R> {
        read::gz_encoder(self.buf_read(BufReader::new(r), lvl))
    }
    /// Consume this builder, creating a reader encoder in the process.
    ///
    /// Data read from the returned encoder will be the compressed version of
    /// the data read from the given reader.
    pub fn buf_read<R>(self, r: R, lvl: Compression) -> bufread::GzEncoder<R>
    where
        R: BufRead,
    {
        bufread::gz_encoder(self.into_header(lvl), r, lvl)
    }
    fn into_header(self, lvl: Compression) -> Vec<u8> {
        let GzBuilder { extra, filename, comment, operating_system, mtime } = self;
        let mut flg = 0;
        let mut header = vec![0u8; 10];
        match extra {
            Some(v) => {
                flg |= FEXTRA;
                header.push((v.len() >> 0) as u8);
                header.push((v.len() >> 8) as u8);
                header.extend(v);
            }
            None => {}
        }
        match filename {
            Some(filename) => {
                flg |= FNAME;
                header.extend(filename.as_bytes_with_nul().iter().map(|x| *x));
            }
            None => {}
        }
        match comment {
            Some(comment) => {
                flg |= FCOMMENT;
                header.extend(comment.as_bytes_with_nul().iter().map(|x| *x));
            }
            None => {}
        }
        header[0] = 0x1f;
        header[1] = 0x8b;
        header[2] = 8;
        header[3] = flg;
        header[4] = (mtime >> 0) as u8;
        header[5] = (mtime >> 8) as u8;
        header[6] = (mtime >> 16) as u8;
        header[7] = (mtime >> 24) as u8;
        header[8] = if lvl.0 >= Compression::best().0 {
            2
        } else if lvl.0 <= Compression::fast().0 {
            4
        } else {
            0
        };
        header[9] = operating_system.unwrap_or(255);
        return header;
    }
}
#[cfg(test)]
mod tests {
    use std::io::prelude::*;
    use super::{read, write, GzBuilder};
    use crate::Compression;
    use rand::{thread_rng, Rng};
    #[test]
    fn roundtrip() {
        let mut e = write::GzEncoder::new(Vec::new(), Compression::default());
        e.write_all(b"foo bar baz").unwrap();
        let inner = e.finish().unwrap();
        let mut d = read::GzDecoder::new(&inner[..]);
        let mut s = String::new();
        d.read_to_string(&mut s).unwrap();
        assert_eq!(s, "foo bar baz");
    }
    #[test]
    fn roundtrip_zero() {
        let e = write::GzEncoder::new(Vec::new(), Compression::default());
        let inner = e.finish().unwrap();
        let mut d = read::GzDecoder::new(&inner[..]);
        let mut s = String::new();
        d.read_to_string(&mut s).unwrap();
        assert_eq!(s, "");
    }
    #[test]
    fn roundtrip_big() {
        let mut real = Vec::new();
        let mut w = write::GzEncoder::new(Vec::new(), Compression::default());
        let v = crate::random_bytes().take(1024).collect::<Vec<_>>();
        for _ in 0..200 {
            let to_write = &v[..thread_rng().gen_range(0, v.len())];
            real.extend(to_write.iter().map(|x| *x));
            w.write_all(to_write).unwrap();
        }
        let result = w.finish().unwrap();
        let mut r = read::GzDecoder::new(&result[..]);
        let mut v = Vec::new();
        r.read_to_end(&mut v).unwrap();
        assert!(v == real);
    }
    #[test]
    fn roundtrip_big2() {
        let v = crate::random_bytes().take(1024 * 1024).collect::<Vec<_>>();
        let mut r = read::GzDecoder::new(
            read::GzEncoder::new(&v[..], Compression::default()),
        );
        let mut res = Vec::new();
        r.read_to_end(&mut res).unwrap();
        assert!(res == v);
    }
    #[test]
    fn fields() {
        let r = vec![0, 2, 4, 6];
        let e = GzBuilder::new()
            .filename("foo.rs")
            .comment("bar")
            .extra(vec![0, 1, 2, 3])
            .read(&r[..], Compression::default());
        let mut d = read::GzDecoder::new(e);
        assert_eq!(d.header().unwrap().filename(), Some(& b"foo.rs"[..]));
        assert_eq!(d.header().unwrap().comment(), Some(& b"bar"[..]));
        assert_eq!(d.header().unwrap().extra(), Some(& b"\x00\x01\x02\x03"[..]));
        let mut res = Vec::new();
        d.read_to_end(&mut res).unwrap();
        assert_eq!(res, vec![0, 2, 4, 6]);
    }
    #[test]
    fn keep_reading_after_end() {
        let mut e = write::GzEncoder::new(Vec::new(), Compression::default());
        e.write_all(b"foo bar baz").unwrap();
        let inner = e.finish().unwrap();
        let mut d = read::GzDecoder::new(&inner[..]);
        let mut s = String::new();
        d.read_to_string(&mut s).unwrap();
        assert_eq!(s, "foo bar baz");
        d.read_to_string(&mut s).unwrap();
        assert_eq!(s, "foo bar baz");
    }
    #[test]
    fn qc_reader() {
        ::quickcheck::quickcheck(test as fn(_) -> _);
        fn test(v: Vec<u8>) -> bool {
            let r = read::GzEncoder::new(&v[..], Compression::default());
            let mut r = read::GzDecoder::new(r);
            let mut v2 = Vec::new();
            r.read_to_end(&mut v2).unwrap();
            v == v2
        }
    }
    #[test]
    fn flush_after_write() {
        let mut f = write::GzEncoder::new(Vec::new(), Compression::default());
        write!(f, "Hello world").unwrap();
        f.flush().unwrap();
    }
}
#[cfg(test)]
mod tests_llm_16_305 {
    use super::*;
    use crate::*;
    use crate::Compression;
    use std::io::prelude::*;
    use std::fs::File;
    #[test]
    fn test_extra() {
        let _rug_st_tests_llm_16_305_rrrruuuugggg_test_extra = 0;
        let rug_fuzz_0 = 1;
        let extra_data = vec![rug_fuzz_0, 2, 3, 4, 5];
        let mut builder = GzBuilder::new();
        builder = builder.extra(extra_data.clone());
        debug_assert_eq!(builder.extra, Some(extra_data));
        let _rug_ed_tests_llm_16_305_rrrruuuugggg_test_extra = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_307_llm_16_306 {
    use crate::{Compression, GzBuilder};
    use std::fs::File;
    use std::io::prelude::*;
    #[test]
    #[should_panic]
    fn test_filename_panic() {
        let _rug_st_tests_llm_16_307_llm_16_306_rrrruuuugggg_test_filename_panic = 0;
        let rug_fuzz_0 = "examples/test.gz";
        let rug_fuzz_1 = 0u8;
        let f = File::create(rug_fuzz_0).unwrap();
        let gz = GzBuilder::new()
            .filename(&[rug_fuzz_1][..])
            .write(f, Compression::default());
        let _rug_ed_tests_llm_16_307_llm_16_306_rrrruuuugggg_test_filename_panic = 0;
    }
    #[test]
    fn test_filename() {
        let _rug_st_tests_llm_16_307_llm_16_306_rrrruuuugggg_test_filename = 0;
        let rug_fuzz_0 = "examples/test.gz";
        let rug_fuzz_1 = "test.txt";
        let f = File::create(rug_fuzz_0).unwrap();
        let gz = GzBuilder::new().filename(rug_fuzz_1).write(f, Compression::default());
        let _rug_ed_tests_llm_16_307_llm_16_306_rrrruuuugggg_test_filename = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_308 {
    use super::*;
    use crate::*;
    use crate::write::GzEncoder;
    use crate::Compression;
    use std::io::prelude::*;
    use std::io::Cursor;
    #[test]
    fn test_into_header() {
        let _rug_st_tests_llm_16_308_rrrruuuugggg_test_into_header = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = "test.txt";
        let rug_fuzz_2 = "test file";
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = 12345;
        let rug_fuzz_5 = 5;
        let rug_fuzz_6 = 0;
        let rug_fuzz_7 = 1;
        let rug_fuzz_8 = 2;
        let rug_fuzz_9 = 3;
        let rug_fuzz_10 = 4;
        let rug_fuzz_11 = 5;
        let rug_fuzz_12 = 6;
        let rug_fuzz_13 = 7;
        let rug_fuzz_14 = 8;
        let rug_fuzz_15 = 9;
        let rug_fuzz_16 = 10;
        let rug_fuzz_17 = 11;
        let rug_fuzz_18 = 12;
        let rug_fuzz_19 = 13;
        let rug_fuzz_20 = 14;
        let rug_fuzz_21 = 15;
        let rug_fuzz_22 = 16;
        let rug_fuzz_23 = 17;
        let rug_fuzz_24 = 18;
        let rug_fuzz_25 = 19;
        let rug_fuzz_26 = 20;
        let rug_fuzz_27 = 21;
        let rug_fuzz_28 = 22;
        let rug_fuzz_29 = 23;
        let rug_fuzz_30 = 24;
        let builder = GzBuilder::new()
            .extra(vec![rug_fuzz_0, 2, 3, 4, 5])
            .filename(rug_fuzz_1)
            .comment(rug_fuzz_2)
            .operating_system(rug_fuzz_3)
            .mtime(rug_fuzz_4);
        let lvl = Compression::new(rug_fuzz_5);
        let header = builder.into_header(lvl);
        debug_assert_eq!(header[rug_fuzz_6], 0x1f);
        debug_assert_eq!(header[rug_fuzz_7], 0x8b);
        debug_assert_eq!(header[rug_fuzz_8], 8);
        debug_assert_eq!(header[rug_fuzz_9], 8);
        debug_assert_eq!(header[rug_fuzz_10], 0x39);
        debug_assert_eq!(header[rug_fuzz_11], 0x30);
        debug_assert_eq!(header[rug_fuzz_12], 0x00);
        debug_assert_eq!(header[rug_fuzz_13], 0x00);
        debug_assert_eq!(header[rug_fuzz_14], 0);
        debug_assert_eq!(header[rug_fuzz_15], 1);
        debug_assert_eq!(header[rug_fuzz_16], 1);
        debug_assert_eq!(header[rug_fuzz_17], 2);
        debug_assert_eq!(header[rug_fuzz_18], 3);
        debug_assert_eq!(header[rug_fuzz_19], 4);
        debug_assert_eq!(header[rug_fuzz_20], 5);
        debug_assert_eq!(header[rug_fuzz_21], 0);
        debug_assert_eq!(header[rug_fuzz_22], 0);
        debug_assert_eq!(header[rug_fuzz_23], 0);
        debug_assert_eq!(header[rug_fuzz_24], 0);
        debug_assert_eq!(header[rug_fuzz_25], 0);
        debug_assert_eq!(header[rug_fuzz_26], 0);
        debug_assert_eq!(header[rug_fuzz_27], 0);
        debug_assert_eq!(header[rug_fuzz_28], 0);
        debug_assert_eq!(header[rug_fuzz_29], 0);
        debug_assert_eq!(header[rug_fuzz_30], 0);
        let _rug_ed_tests_llm_16_308_rrrruuuugggg_test_into_header = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_311 {
    use crate::{GzBuilder, Compression};
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_311_rrrruuuugggg_test_new = 0;
        let builder = GzBuilder::new();
        debug_assert_eq!(builder.extra, None);
        debug_assert_eq!(builder.filename, None);
        debug_assert_eq!(builder.comment, None);
        debug_assert_eq!(builder.operating_system, None);
        debug_assert_eq!(builder.mtime, 0);
        let _rug_ed_tests_llm_16_311_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_312 {
    use std::io::prelude::*;
    use std::fs::File;
    use crate::{GzBuilder, Compression};
    #[test]
    fn test_operating_system() {
        let _rug_st_tests_llm_16_312_rrrruuuugggg_test_operating_system = 0;
        let rug_fuzz_0 = "test.gz";
        let rug_fuzz_1 = 5;
        let rug_fuzz_2 = b"test";
        let f = File::create(rug_fuzz_0).unwrap();
        let mut gz = GzBuilder::new()
            .operating_system(rug_fuzz_1)
            .write(f, Compression::default());
        gz.write_all(rug_fuzz_2).unwrap();
        gz.finish().unwrap();
        let _rug_ed_tests_llm_16_312_rrrruuuugggg_test_operating_system = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_315 {
    use super::*;
    use crate::*;
    use std::io::{Read, Write};
    use crate::Compression;
    #[test]
    fn test_gz_builder_write() {
        let _rug_st_tests_llm_16_315_rrrruuuugggg_test_gz_builder_write = 0;
        let rug_fuzz_0 = b"Hello, World!";
        let data = rug_fuzz_0;
        let mut buffer = Vec::new();
        let mut gz = GzBuilder::new().write(&mut buffer, Compression::default());
        gz.write_all(data).unwrap();
        gz.finish().unwrap();
        let mut result = Vec::new();
        let mut decoder = gz::read::GzDecoder::new(&buffer[..]);
        decoder.read_to_end(&mut result).unwrap();
        debug_assert_eq!(result, data);
        let _rug_ed_tests_llm_16_315_rrrruuuugggg_test_gz_builder_write = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_317_llm_16_316 {
    use super::*;
    use crate::*;
    use crate::*;
    use std::time;
    #[test]
    fn test_comment_empty() {
        let _rug_st_tests_llm_16_317_llm_16_316_rrrruuuugggg_test_comment_empty = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let header = GzHeader {
            extra: None,
            filename: None,
            comment: None,
            operating_system: rug_fuzz_0,
            mtime: rug_fuzz_1,
        };
        debug_assert_eq!(header.comment(), None);
        let _rug_ed_tests_llm_16_317_llm_16_316_rrrruuuugggg_test_comment_empty = 0;
    }
    #[test]
    fn test_comment_non_empty() {
        let _rug_st_tests_llm_16_317_llm_16_316_rrrruuuugggg_test_comment_non_empty = 0;
        let rug_fuzz_0 = 104;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let header = GzHeader {
            extra: None,
            filename: None,
            comment: Some(vec![rug_fuzz_0, 101, 108, 108, 111]),
            operating_system: rug_fuzz_1,
            mtime: rug_fuzz_2,
        };
        debug_assert_eq!(header.comment(), Some(& [104, 101, 108, 108, 111] [..]));
        let _rug_ed_tests_llm_16_317_llm_16_316_rrrruuuugggg_test_comment_non_empty = 0;
    }
    #[test]
    fn test_comment_as_datetime() {
        let _rug_st_tests_llm_16_317_llm_16_316_rrrruuuugggg_test_comment_as_datetime = 0;
        let rug_fuzz_0 = 104;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 1618641595;
        let rug_fuzz_3 = 1618641595;
        let rug_fuzz_4 = 0;
        let header = GzHeader {
            extra: None,
            filename: None,
            comment: Some(vec![rug_fuzz_0, 101, 108, 108, 111]),
            operating_system: rug_fuzz_1,
            mtime: rug_fuzz_2,
        };
        let expected = time::UNIX_EPOCH + time::Duration::new(rug_fuzz_3, rug_fuzz_4);
        debug_assert_eq!(header.mtime_as_datetime(), Some(expected));
        let _rug_ed_tests_llm_16_317_llm_16_316_rrrruuuugggg_test_comment_as_datetime = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_320 {
    use super::*;
    use crate::*;
    #[test]
    fn test_filename_returns_none_when_filename_not_present() {
        let _rug_st_tests_llm_16_320_rrrruuuugggg_test_filename_returns_none_when_filename_not_present = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let header = GzHeader {
            extra: None,
            filename: None,
            comment: None,
            operating_system: rug_fuzz_0,
            mtime: rug_fuzz_1,
        };
        debug_assert_eq!(None, header.filename());
        let _rug_ed_tests_llm_16_320_rrrruuuugggg_test_filename_returns_none_when_filename_not_present = 0;
    }
    #[test]
    fn test_filename_returns_some_filename_when_filename_present() {
        let _rug_st_tests_llm_16_320_rrrruuuugggg_test_filename_returns_some_filename_when_filename_present = 0;
        let rug_fuzz_0 = b'f';
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let filename = vec![rug_fuzz_0, b'i', b'l', b'e'];
        let header = GzHeader {
            extra: None,
            filename: Some(filename.clone()),
            comment: None,
            operating_system: rug_fuzz_1,
            mtime: rug_fuzz_2,
        };
        debug_assert_eq!(Some(filename.as_slice()), header.filename());
        let _rug_ed_tests_llm_16_320_rrrruuuugggg_test_filename_returns_some_filename_when_filename_present = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_321 {
    use crate::gz::GzHeader;
    #[test]
    fn test_mtime() {
        let _rug_st_tests_llm_16_321_rrrruuuugggg_test_mtime = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 123456;
        let gz_header = GzHeader {
            extra: None,
            filename: None,
            comment: None,
            operating_system: rug_fuzz_0,
            mtime: rug_fuzz_1,
        };
        debug_assert_eq!(gz_header.mtime(), 123456);
        let _rug_ed_tests_llm_16_321_rrrruuuugggg_test_mtime = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_322 {
    use super::*;
    use crate::*;
    use time::SystemTime;
    #[test]
    fn test_mtime_as_datetime() {
        let _rug_st_tests_llm_16_322_rrrruuuugggg_test_mtime_as_datetime = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 4;
        let rug_fuzz_2 = 7;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 1234567890;
        let rug_fuzz_5 = 1234567890;
        let rug_fuzz_6 = 0;
        let rug_fuzz_7 = 1;
        let rug_fuzz_8 = 4;
        let rug_fuzz_9 = 7;
        let rug_fuzz_10 = 0;
        let rug_fuzz_11 = 0;
        let header = GzHeader {
            extra: Some(vec![rug_fuzz_0, 2, 3]),
            filename: Some(vec![rug_fuzz_1, 5, 6]),
            comment: Some(vec![rug_fuzz_2, 8, 9]),
            operating_system: rug_fuzz_3,
            mtime: rug_fuzz_4,
        };
        let expected = Some(
            SystemTime::UNIX_EPOCH + time::Duration::new(rug_fuzz_5, rug_fuzz_6),
        );
        let result = header.mtime_as_datetime();
        debug_assert_eq!(result, expected);
        let header = GzHeader {
            extra: Some(vec![rug_fuzz_7, 2, 3]),
            filename: Some(vec![rug_fuzz_8, 5, 6]),
            comment: Some(vec![rug_fuzz_9, 8, 9]),
            operating_system: rug_fuzz_10,
            mtime: rug_fuzz_11,
        };
        let expected = None;
        let result = header.mtime_as_datetime();
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_322_rrrruuuugggg_test_mtime_as_datetime = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_323 {
    use crate::gz::GzHeader;
    #[test]
    fn test_operating_system() {
        let _rug_st_tests_llm_16_323_rrrruuuugggg_test_operating_system = 0;
        let rug_fuzz_0 = 255;
        let rug_fuzz_1 = 0;
        let header = GzHeader {
            extra: None,
            filename: None,
            comment: None,
            operating_system: rug_fuzz_0,
            mtime: rug_fuzz_1,
        };
        debug_assert_eq!(header.operating_system(), 255);
        let _rug_ed_tests_llm_16_323_rrrruuuugggg_test_operating_system = 0;
    }
}
#[cfg(test)]
mod tests_rug_142 {
    use super::*;
    use crate::gz::GzHeader;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_142_rrrruuuugggg_test_rug = 0;
        let mut p0 = GzHeader::default();
        crate::gz::GzHeader::extra(&p0);
        let _rug_ed_tests_rug_142_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_144 {
    use super::*;
    use crate::GzBuilder;
    use std::ffi::CString;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_144_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"some comment";
        let mut p0 = GzBuilder::new();
        let p1 = CString::new(rug_fuzz_0.to_vec()).unwrap();
        p0.comment(p1);
        let _rug_ed_tests_rug_144_rrrruuuugggg_test_rug = 0;
    }
}
