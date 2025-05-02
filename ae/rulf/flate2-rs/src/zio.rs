use std::io;
use std::io::prelude::*;
use std::mem;
use crate::{
    Compress, Decompress, DecompressError, FlushCompress, FlushDecompress, Status,
};
#[derive(Debug)]
pub struct Writer<W: Write, D: Ops> {
    obj: Option<W>,
    pub data: D,
    buf: Vec<u8>,
}
pub trait Ops {
    type Flush: Flush;
    fn total_in(&self) -> u64;
    fn total_out(&self) -> u64;
    fn run(
        &mut self,
        input: &[u8],
        output: &mut [u8],
        flush: Self::Flush,
    ) -> Result<Status, DecompressError>;
    fn run_vec(
        &mut self,
        input: &[u8],
        output: &mut Vec<u8>,
        flush: Self::Flush,
    ) -> Result<Status, DecompressError>;
}
impl Ops for Compress {
    type Flush = FlushCompress;
    fn total_in(&self) -> u64 {
        self.total_in()
    }
    fn total_out(&self) -> u64 {
        self.total_out()
    }
    fn run(
        &mut self,
        input: &[u8],
        output: &mut [u8],
        flush: FlushCompress,
    ) -> Result<Status, DecompressError> {
        Ok(self.compress(input, output, flush).unwrap())
    }
    fn run_vec(
        &mut self,
        input: &[u8],
        output: &mut Vec<u8>,
        flush: FlushCompress,
    ) -> Result<Status, DecompressError> {
        Ok(self.compress_vec(input, output, flush).unwrap())
    }
}
impl Ops for Decompress {
    type Flush = FlushDecompress;
    fn total_in(&self) -> u64 {
        self.total_in()
    }
    fn total_out(&self) -> u64 {
        self.total_out()
    }
    fn run(
        &mut self,
        input: &[u8],
        output: &mut [u8],
        flush: FlushDecompress,
    ) -> Result<Status, DecompressError> {
        self.decompress(input, output, flush)
    }
    fn run_vec(
        &mut self,
        input: &[u8],
        output: &mut Vec<u8>,
        flush: FlushDecompress,
    ) -> Result<Status, DecompressError> {
        self.decompress_vec(input, output, flush)
    }
}
pub trait Flush {
    fn none() -> Self;
    fn sync() -> Self;
    fn finish() -> Self;
}
impl Flush for FlushCompress {
    fn none() -> Self {
        FlushCompress::None
    }
    fn sync() -> Self {
        FlushCompress::Sync
    }
    fn finish() -> Self {
        FlushCompress::Finish
    }
}
impl Flush for FlushDecompress {
    fn none() -> Self {
        FlushDecompress::None
    }
    fn sync() -> Self {
        FlushDecompress::Sync
    }
    fn finish() -> Self {
        FlushDecompress::Finish
    }
}
pub fn read<R, D>(obj: &mut R, data: &mut D, dst: &mut [u8]) -> io::Result<usize>
where
    R: BufRead,
    D: Ops,
{
    loop {
        let (read, consumed, ret, eof);
        {
            let input = obj.fill_buf()?;
            eof = input.is_empty();
            let before_out = data.total_out();
            let before_in = data.total_in();
            let flush = if eof { D::Flush::finish() } else { D::Flush::none() };
            ret = data.run(input, dst, flush);
            read = (data.total_out() - before_out) as usize;
            consumed = (data.total_in() - before_in) as usize;
        }
        obj.consume(consumed);
        match ret {
            Ok(Status::Ok)
            | Ok(Status::BufError) if read == 0 && !eof && dst.len() > 0 => continue,
            Ok(Status::Ok) | Ok(Status::BufError) | Ok(Status::StreamEnd) => {
                return Ok(read);
            }
            Err(..) => {
                return Err(
                    io::Error::new(io::ErrorKind::InvalidInput, "corrupt deflate stream"),
                );
            }
        }
    }
}
impl<W: Write, D: Ops> Writer<W, D> {
    pub fn new(w: W, d: D) -> Writer<W, D> {
        Writer {
            obj: Some(w),
            data: d,
            buf: Vec::with_capacity(32 * 1024),
        }
    }
    pub fn finish(&mut self) -> io::Result<()> {
        loop {
            self.dump()?;
            let before = self.data.total_out();
            self.data.run_vec(&[], &mut self.buf, D::Flush::finish())?;
            if before == self.data.total_out() {
                return Ok(());
            }
        }
    }
    pub fn replace(&mut self, w: W) -> W {
        self.buf.truncate(0);
        mem::replace(self.get_mut(), w)
    }
    pub fn get_ref(&self) -> &W {
        self.obj.as_ref().unwrap()
    }
    pub fn get_mut(&mut self) -> &mut W {
        self.obj.as_mut().unwrap()
    }
    pub fn take_inner(&mut self) -> W {
        self.obj.take().unwrap()
    }
    pub fn is_present(&self) -> bool {
        self.obj.is_some()
    }
    pub(crate) fn write_with_status(
        &mut self,
        buf: &[u8],
    ) -> io::Result<(usize, Status)> {
        loop {
            self.dump()?;
            let before_in = self.data.total_in();
            let ret = self.data.run_vec(buf, &mut self.buf, D::Flush::none());
            let written = (self.data.total_in() - before_in) as usize;
            let is_stream_end = match ret {
                Ok(Status::StreamEnd) => true,
                _ => false,
            };
            if buf.len() > 0 && written == 0 && ret.is_ok() && !is_stream_end {
                continue;
            }
            return match ret {
                Ok(st) => {
                    match st {
                        Status::Ok | Status::BufError | Status::StreamEnd => {
                            Ok((written, st))
                        }
                    }
                }
                Err(..) => {
                    Err(
                        io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "corrupt deflate stream",
                        ),
                    )
                }
            };
        }
    }
    fn dump(&mut self) -> io::Result<()> {
        while self.buf.len() > 0 {
            let n = self.obj.as_mut().unwrap().write(&self.buf)?;
            if n == 0 {
                return Err(io::ErrorKind::WriteZero.into());
            }
            self.buf.drain(..n);
        }
        Ok(())
    }
}
impl<W: Write, D: Ops> Write for Writer<W, D> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.write_with_status(buf).map(|res| res.0)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.data.run_vec(&[], &mut self.buf, D::Flush::sync()).unwrap();
        loop {
            self.dump()?;
            let before = self.data.total_out();
            self.data.run_vec(&[], &mut self.buf, D::Flush::none()).unwrap();
            if before == self.data.total_out() {
                break;
            }
        }
        self.obj.as_mut().unwrap().flush()
    }
}
impl<W: Write, D: Ops> Drop for Writer<W, D> {
    fn drop(&mut self) {
        if self.obj.is_some() {
            let _ = self.finish();
        }
    }
}
#[cfg(test)]
mod tests_llm_16_118_llm_16_117 {
    use super::*;
    use crate::*;
    use crate::mem::Compress;
    use crate::zio::{Ops, FlushCompress};
    use std::io::{Error, ErrorKind};
    #[test]
    fn test_run() {
        let _rug_st_tests_llm_16_118_llm_16_117_rrrruuuugggg_test_run = 0;
        let rug_fuzz_0 = 0u8;
        let rug_fuzz_1 = 0u8;
        let rug_fuzz_2 = false;
        let mut input = [rug_fuzz_0; 10];
        let mut output = [rug_fuzz_1; 10];
        let mut flush = FlushCompress::None;
        let result = Compress::run(
                &mut Compress::new(Compression::default(), rug_fuzz_2),
                &input,
                &mut output,
                flush,
            )
            .unwrap();
        debug_assert_eq!(result, Status::Ok);
        let _rug_ed_tests_llm_16_118_llm_16_117_rrrruuuugggg_test_run = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_122 {
    use super::*;
    use crate::*;
    use crate::zio::Ops;
    #[test]
    fn test_total_in() {
        let _rug_st_tests_llm_16_122_rrrruuuugggg_test_total_in = 0;
        let rug_fuzz_0 = false;
        let rug_fuzz_1 = b"Hello, world!";
        let mut compress = Compress::new(Compression::fast(), rug_fuzz_0);
        debug_assert_eq!(compress.total_in(), 0);
        let input = rug_fuzz_1;
        let mut output = vec![0; 100];
        compress.compress(input, &mut output, FlushCompress::Finish).unwrap();
        debug_assert_eq!(compress.total_in(), input.len() as u64);
        let _rug_ed_tests_llm_16_122_rrrruuuugggg_test_total_in = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_125 {
    use super::*;
    use crate::*;
    use crate::mem::FlushDecompress;
    #[test]
    fn test_run() {
        let _rug_st_tests_llm_16_125_rrrruuuugggg_test_run = 0;
        let rug_fuzz_0 = 0u8;
        let rug_fuzz_1 = 0u8;
        let rug_fuzz_2 = false;
        let mut input = [rug_fuzz_0; 10];
        let mut output = [rug_fuzz_1; 10];
        let mut decompress = Decompress::new(rug_fuzz_2);
        let flush = FlushDecompress::None;
        let result = decompress.run(&input, &mut output, flush);
        debug_assert!(result.is_ok());
        let _rug_ed_tests_llm_16_125_rrrruuuugggg_test_run = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_129_llm_16_128 {
    use super::*;
    use crate::*;
    use crate::Decompress;
    use crate::DecompressError;
    use crate::FlushDecompress;
    use crate::Status;
    #[test]
    fn test_total_in() {
        let _rug_st_tests_llm_16_129_llm_16_128_rrrruuuugggg_test_total_in = 0;
        let rug_fuzz_0 = true;
        let mut decompress = Decompress::new(rug_fuzz_0);
        let total_in = decompress.total_in();
        debug_assert_eq!(total_in, 0);
        let _rug_ed_tests_llm_16_129_llm_16_128_rrrruuuugggg_test_total_in = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_132 {
    use crate::zio::{Flush, FlushCompress};
    #[test]
    fn test_finish() {
        let _rug_st_tests_llm_16_132_rrrruuuugggg_test_finish = 0;
        let result = FlushCompress::finish();
        debug_assert_eq!(result, FlushCompress::Finish);
        let _rug_ed_tests_llm_16_132_rrrruuuugggg_test_finish = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_134 {
    use super::*;
    use crate::*;
    use crate::zio::Flush;
    #[test]
    fn test_none() {
        let _rug_st_tests_llm_16_134_rrrruuuugggg_test_none = 0;
        let result = <FlushCompress as Flush>::none();
        debug_assert_eq!(result, FlushCompress::None);
        let _rug_ed_tests_llm_16_134_rrrruuuugggg_test_none = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_135 {
    use crate::zio::Flush;
    use crate::zio::FlushCompress;
    #[test]
    fn test_sync() {
        let _rug_st_tests_llm_16_135_rrrruuuugggg_test_sync = 0;
        let flush_compress: FlushCompress = FlushCompress::sync();
        debug_assert_eq!(flush_compress, FlushCompress::Sync);
        let _rug_ed_tests_llm_16_135_rrrruuuugggg_test_sync = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_138 {
    use crate::zio::Flush;
    use crate::zio::FlushDecompress;
    #[test]
    fn test_none() {
        let _rug_st_tests_llm_16_138_rrrruuuugggg_test_none = 0;
        let result = FlushDecompress::none();
        debug_assert_eq!(result, FlushDecompress::None);
        let _rug_ed_tests_llm_16_138_rrrruuuugggg_test_none = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_140_llm_16_139 {
    use super::*;
    use crate::*;
    use crate::FlushDecompress;
    #[test]
    fn test_sync() {
        let _rug_st_tests_llm_16_140_llm_16_139_rrrruuuugggg_test_sync = 0;
        let result = <FlushDecompress as zio::Flush>::sync();
        debug_assert_eq!(result, FlushDecompress::Sync);
        let _rug_ed_tests_llm_16_140_llm_16_139_rrrruuuugggg_test_sync = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_449_llm_16_448 {
    use super::*;
    use crate::*;
    use std::io::Write;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_449_llm_16_448_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = 6;
        let rug_fuzz_1 = false;
        let w = Vec::new();
        let d = Compress::new(Compression(rug_fuzz_0), rug_fuzz_1);
        let writer = Writer::new(w, d);
        let _rug_ed_tests_llm_16_449_llm_16_448_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_rug_82 {
    use super::*;
    use crate::{Decompress, zio::Ops};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_82_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = false;
        let mut p0 = Decompress::new(rug_fuzz_0);
        p0.total_out();
        let _rug_ed_tests_rug_82_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_83 {
    use super::*;
    use crate::zio::Ops;
    use crate::{Decompress, FlushDecompress};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_83_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = false;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 2;
        let rug_fuzz_4 = 3;
        let rug_fuzz_5 = 4;
        let mut p0 = Decompress::new(rug_fuzz_0);
        let p1: &[u8] = &[rug_fuzz_1, rug_fuzz_2, rug_fuzz_3, rug_fuzz_4, rug_fuzz_5];
        let mut p2 = Vec::new();
        let p3 = FlushDecompress::finish();
        p0.run_vec(p1, &mut p2, p3);
        let _rug_ed_tests_rug_83_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_84 {
    use super::*;
    use crate::zio::Flush;
    use crate::mem::FlushDecompress;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_84_rrrruuuugggg_test_rug = 0;
        FlushDecompress::finish();
        let _rug_ed_tests_rug_84_rrrruuuugggg_test_rug = 0;
    }
}
