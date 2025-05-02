//! `ReadSlice` trait to allow efficient reading of slices without copying.
use std::error::Error;
use std::fmt;
use std::io::{self, Cursor};
/// Type which supports reading a slice of bytes.
pub trait ReadSlice {
    fn read_slice(&mut self, n: usize) -> Result<&[u8], ReadSliceError>;
}
impl ReadSlice for Cursor<Vec<u8>> {
    fn read_slice(&mut self, n: usize) -> Result<&[u8], ReadSliceError> {
        let start = self.position() as usize;
        if self.get_ref().len() - start < n {
            return Err(ReadSliceError::InsufficientData);
        }
        self.set_position((start + n) as u64);
        Ok(&self.get_ref()[start..start + n])
    }
}
impl<'r> ReadSlice for Cursor<&'r [u8]> {
    fn read_slice(&mut self, n: usize) -> Result<&[u8], ReadSliceError> {
        let start = self.position() as usize;
        if self.get_ref().len() - start < n {
            return Err(ReadSliceError::InsufficientData);
        }
        self.set_position((start + n) as u64);
        Ok(&self.get_ref()[start..start + n])
    }
}
#[derive(Debug)]
pub enum ReadSliceError {
    IoError(io::Error),
    InsufficientData,
}
impl fmt::Display for ReadSliceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            ReadSliceError::IoError(ref e) => {
                write!(f, "ReadSliceError: I/O error: {}", * e)
            }
            ReadSliceError::InsufficientData => {
                write!(f, "ReadSliceError: not enough data available")
            }
        }
    }
}
impl Error for ReadSliceError {
    fn description(&self) -> &str {
        "ReadSliceError"
    }
    fn cause(&self) -> Option<&Error> {
        match *self {
            ReadSliceError::IoError(ref e) => Some(e),
            _ => None,
        }
    }
}
#[cfg(test)]
mod tests_rug_114 {
    use super::*;
    use crate::slice::ReadSlice;
    use std::io::Cursor;
    use std::vec::Vec;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u8, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let data: Vec<u8> = vec![rug_fuzz_0, 2, 3, 4, 5];
        let cursor = Cursor::new(data);
        let mut p0 = cursor;
        let p1: usize = rug_fuzz_1;
        p0.read_slice(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_115 {
    use super::*;
    use crate::slice::ReadSlice;
    use std::io::Cursor;
    use crate::slice;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_115_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"example data for v29";
        let rug_fuzz_1 = 10;
        let mut p0: std::io::Cursor<&'static [u8]> = {
            let data: &'static [u8] = rug_fuzz_0;
            Cursor::new(data)
        };
        let p1: usize = rug_fuzz_1;
        <std::io::Cursor<&'static [u8]>>::read_slice(&mut p0, p1).unwrap();
        let _rug_ed_tests_rug_115_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_116 {
    use super::*;
    use crate::slice::ReadSliceError;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_116_rrrruuuugggg_test_rug = 0;
        let mut p0 = ReadSliceError::InsufficientData;
        debug_assert_eq!(p0.description(), "ReadSliceError");
        let _rug_ed_tests_rug_116_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_117 {
    use super::*;
    use crate::std::error::Error;
    use crate::slice::ReadSliceError;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_117_rrrruuuugggg_test_rug = 0;
        let p0 = ReadSliceError::InsufficientData;
        p0.cause();
        let _rug_ed_tests_rug_117_rrrruuuugggg_test_rug = 0;
    }
}
