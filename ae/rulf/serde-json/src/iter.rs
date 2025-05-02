use crate::io;
pub struct LineColIterator<I> {
    iter: I,
    /// Index of the current line. Characters in the first line of the input
    /// (before the first newline character) are in line 1.
    line: usize,
    /// Index of the current column. The first character in the input and any
    /// characters immediately following a newline character are in column 1.
    /// The column is 0 immediately after a newline character has been read.
    col: usize,
    /// Byte offset of the start of the current line. This is the sum of lenghts
    /// of all previous lines. Keeping track of things this way allows efficient
    /// computation of the current line, column, and byte offset while only
    /// updating one of the counters in `next()` in the common case.
    start_of_line: usize,
}
impl<I> LineColIterator<I>
where
    I: Iterator<Item = io::Result<u8>>,
{
    pub fn new(iter: I) -> LineColIterator<I> {
        LineColIterator {
            iter,
            line: 1,
            col: 0,
            start_of_line: 0,
        }
    }
    pub fn line(&self) -> usize {
        self.line
    }
    pub fn col(&self) -> usize {
        self.col
    }
    pub fn byte_offset(&self) -> usize {
        self.start_of_line + self.col
    }
}
impl<I> Iterator for LineColIterator<I>
where
    I: Iterator<Item = io::Result<u8>>,
{
    type Item = io::Result<u8>;
    fn next(&mut self) -> Option<io::Result<u8>> {
        match self.iter.next() {
            None => None,
            Some(Ok(b'\n')) => {
                self.start_of_line += self.col + 1;
                self.line += 1;
                self.col = 0;
                Some(Ok(b'\n'))
            }
            Some(Ok(c)) => {
                self.col += 1;
                Some(Ok(c))
            }
            Some(Err(e)) => Some(Err(e)),
        }
    }
}
#[cfg(test)]
mod tests_llm_16_902 {
    use super::*;
    use crate::*;
    use std::io;
    use crate::iter::LineColIterator;
    #[test]
    fn test_col() {
        let _rug_st_tests_llm_16_902_rrrruuuugggg_test_col = 0;
        let rug_fuzz_0 = 104;
        let data: Vec<u8> = vec![
            rug_fuzz_0, 101, 108, 108, 111, 10, 119, 111, 114, 108, 100
        ];
        let iter = LineColIterator::new(data.into_iter().map(|b| Ok(b)));
        let col = iter.col();
        debug_assert_eq!(col, 0);
        let _rug_ed_tests_llm_16_902_rrrruuuugggg_test_col = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_904_llm_16_903 {
    use super::*;
    use crate::*;
    #[test]
    fn test_line() {
        let _rug_st_tests_llm_16_904_llm_16_903_rrrruuuugggg_test_line = 0;
        let rug_fuzz_0 = b'h';
        let mut iter = vec![
            Ok(rug_fuzz_0), Ok(b'e'), Ok(b'\n'), Ok(b'l'), Ok(b'o'), Ok(b'\n')
        ]
            .into_iter();
        let mut line_iter = LineColIterator::new(&mut iter);
        debug_assert_eq!(line_iter.line(), 1);
        line_iter.next();
        debug_assert_eq!(line_iter.line(), 1);
        line_iter.next();
        debug_assert_eq!(line_iter.line(), 2);
        line_iter.next();
        debug_assert_eq!(line_iter.line(), 2);
        line_iter.next();
        debug_assert_eq!(line_iter.line(), 2);
        line_iter.next();
        debug_assert_eq!(line_iter.line(), 3);
        let _rug_ed_tests_llm_16_904_llm_16_903_rrrruuuugggg_test_line = 0;
    }
}
