use std::cmp;
use std::io;
use std::io::prelude::*;
use std::mem;
pub struct BufReader<R> {
    inner: R,
    buf: Box<[u8]>,
    pos: usize,
    cap: usize,
}
impl<R> ::std::fmt::Debug for BufReader<R>
where
    R: ::std::fmt::Debug,
{
    fn fmt(&self, fmt: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        fmt.debug_struct("BufReader")
            .field("reader", &self.inner)
            .field("buffer", &format_args!("{}/{}", self.cap - self.pos, self.buf.len()))
            .finish()
    }
}
impl<R: Read> BufReader<R> {
    pub fn new(inner: R) -> BufReader<R> {
        BufReader::with_buf(vec![0; 32 * 1024], inner)
    }
    pub fn with_buf(buf: Vec<u8>, inner: R) -> BufReader<R> {
        BufReader {
            inner: inner,
            buf: buf.into_boxed_slice(),
            pos: 0,
            cap: 0,
        }
    }
}
impl<R> BufReader<R> {
    pub fn get_ref(&self) -> &R {
        &self.inner
    }
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }
    pub fn into_inner(self) -> R {
        self.inner
    }
    pub fn reset(&mut self, inner: R) -> R {
        self.pos = 0;
        self.cap = 0;
        mem::replace(&mut self.inner, inner)
    }
}
impl<R: Read> Read for BufReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.pos == self.cap && buf.len() >= self.buf.len() {
            return self.inner.read(buf);
        }
        let nread = {
            let mut rem = self.fill_buf()?;
            rem.read(buf)?
        };
        self.consume(nread);
        Ok(nread)
    }
}
impl<R: Read> BufRead for BufReader<R> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if self.pos == self.cap {
            self.cap = self.inner.read(&mut self.buf)?;
            self.pos = 0;
        }
        Ok(&self.buf[self.pos..self.cap])
    }
    fn consume(&mut self, amt: usize) {
        self.pos = cmp::min(self.pos + amt, self.cap);
    }
}
#[cfg(test)]
mod tests_llm_16_3_llm_16_2 {
    use crate::bufreader::BufReader;
    use std::io::BufRead;
    use std::io::Read;
    use std::io;
    #[test]
    fn test_consume() {
        let _rug_st_tests_llm_16_3_llm_16_2_rrrruuuugggg_test_consume = 0;
        let rug_fuzz_0 = b"hello world";
        let rug_fuzz_1 = 5;
        let mut reader = BufReader::new(&rug_fuzz_0[..]);
        <BufReader<&[u8]> as BufRead>::consume(&mut reader, rug_fuzz_1);
        debug_assert_eq!(reader.pos, 5);
        debug_assert_eq!(reader.cap, 11);
        let _rug_ed_tests_llm_16_3_llm_16_2_rrrruuuugggg_test_consume = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_4 {
    use std::io::{self, BufRead, Read};
    use crate::bufreader::BufReader;
    #[test]
    fn test_fill_buf() {
        let mut reader: BufReader<&[u8]> = BufReader::with_buf(
            vec![1, 2, 3, 4, 5],
            &[1, 2, 3, 4, 5],
        );
        let buf = reader.fill_buf().unwrap();
        assert_eq!(buf, & [1, 2, 3, 4, 5]);
    }
}
#[cfg(test)]
mod tests_llm_16_5 {
    use std::io::{BufRead, Cursor, Read};
    use crate::bufreader::BufReader;
    #[test]
    fn test_read() {
        let _rug_st_tests_llm_16_5_rrrruuuugggg_test_read = 0;
        let rug_fuzz_0 = b"Hello, World!";
        let rug_fuzz_1 = 0u8;
        let data: &[u8] = rug_fuzz_0;
        let cursor = Cursor::new(data.to_vec());
        let mut buf_reader = BufReader::new(cursor);
        let mut buf = [rug_fuzz_1; 5];
        let result = buf_reader.read(&mut buf);
        debug_assert_eq!(result.unwrap(), 5);
        debug_assert_eq!(& buf, b"Hello");
        let _rug_ed_tests_llm_16_5_rrrruuuugggg_test_read = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_187 {
    use super::*;
    use crate::*;
    #[test]
    fn test_get_ref() {
        let _rug_st_tests_llm_16_187_rrrruuuugggg_test_get_ref = 0;
        let rug_fuzz_0 = b"Hello, World!";
        let reader: BufReader<&[u8]> = BufReader::new(rug_fuzz_0.as_ref());
        let ref_inner = reader.get_ref();
        debug_assert_eq!(ref_inner, & b"Hello, World!".as_ref());
        let _rug_ed_tests_llm_16_187_rrrruuuugggg_test_get_ref = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_190 {
    use super::*;
    use crate::*;
    use std::io::Read;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_190_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 4;
        let rug_fuzz_4 = 5;
        let inner: &[u8] = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3, rug_fuzz_4];
        let reader = BufReader::new(inner);
        let buf = reader.buf;
        debug_assert_eq!(buf.len(), 32768);
        let pos = reader.pos;
        debug_assert_eq!(pos, 0);
        let cap = reader.cap;
        debug_assert_eq!(cap, 0);
        let _rug_ed_tests_llm_16_190_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_193 {
    use super::*;
    use crate::*;
    use std::io::Read;
    #[test]
    fn test_with_buf() {
        let _rug_st_tests_llm_16_193_rrrruuuugggg_test_with_buf = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 6;
        let rug_fuzz_2 = 7;
        let rug_fuzz_3 = 8;
        let buf: Vec<u8> = vec![rug_fuzz_0, 2, 3, 4, 5];
        let inner: &[u8] = &[rug_fuzz_1, rug_fuzz_2, rug_fuzz_3];
        let br = BufReader::with_buf(buf, inner as &[u8]);
        debug_assert_eq!(br.inner, inner as & [u8]);
        debug_assert_eq!(br.buf, vec![1, 2, 3, 4, 5] .into_boxed_slice());
        debug_assert_eq!(br.pos, 0);
        debug_assert_eq!(br.cap, 0);
        let _rug_ed_tests_llm_16_193_rrrruuuugggg_test_with_buf = 0;
    }
}
#[cfg(test)]
mod tests_rug_105_prepare {
    use crate::bufreader::BufReader;
    use std::io::Read;
    #[test]
    fn sample() {
        let _rug_st_tests_rug_105_prepare_rrrruuuugggg_sample = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let mut data: Vec<u8> = Vec::new();
        let mut v: &[u8] = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        let v55 = BufReader::new(v);
        let _rug_ed_tests_rug_105_prepare_rrrruuuugggg_sample = 0;
    }
}
#[cfg(test)]
mod tests_rug_105 {
    use super::*;
    use crate::bufreader::BufReader;
    #[test]
    fn test_rug() {
        let mut p0: BufReader<&[u8]> = BufReader::new(&[1, 2, 3]);
        crate::bufreader::BufReader::<&[u8]>::get_mut(&mut p0);
    }
}
#[cfg(test)]
mod tests_rug_106 {
    use super::*;
    use crate::bufreader::BufReader;
    use std::io::Read;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_106_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let mut data: Vec<u8> = Vec::new();
        let mut v: &[u8] = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        let p0 = BufReader::new(v);
        let result = p0.into_inner();
        let _rug_ed_tests_rug_106_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_107 {
    use super::*;
    use crate::bufreader::BufReader;
    use std::io::Read;
    #[test]
    fn test_reset() {
        let _rug_st_tests_rug_107_rrrruuuugggg_test_reset = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 4;
        let rug_fuzz_4 = 5;
        let rug_fuzz_5 = 6;
        let mut data: Vec<u8> = Vec::new();
        let mut v: &[u8] = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        let mut br: BufReader<&[u8]> = BufReader::new(v);
        let inner: &[u8] = &[rug_fuzz_3, rug_fuzz_4, rug_fuzz_5];
        debug_assert_eq!(br.reset(inner), & [1, 2, 3]);
        debug_assert_eq!(br.pos, 0);
        debug_assert_eq!(br.cap, 0);
        let _rug_ed_tests_rug_107_rrrruuuugggg_test_reset = 0;
    }
}
