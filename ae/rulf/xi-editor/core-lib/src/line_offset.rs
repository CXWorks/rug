#![allow(clippy::range_plus_one)]
use std::ops::Range;
use xi_rope::Rope;
use crate::linewrap::Lines;
use crate::selection::SelRegion;
/// A trait from which lines and columns in a document can be calculated
/// into offsets inside a rope an vice versa.
pub trait LineOffset {
    /// Returns the byte offset corresponding to the given line.
    fn offset_of_line(&self, text: &Rope, line: usize) -> usize {
        text.offset_of_line(line)
    }
    /// Returns the visible line number containing the given offset.
    fn line_of_offset(&self, text: &Rope, offset: usize) -> usize {
        text.line_of_offset(offset)
    }
    fn offset_to_line_col(&self, text: &Rope, offset: usize) -> (usize, usize) {
        let line = self.line_of_offset(text, offset);
        (line, offset - self.offset_of_line(text, line))
    }
    fn line_col_to_offset(&self, text: &Rope, line: usize, col: usize) -> usize {
        let mut offset = self.offset_of_line(text, line).saturating_add(col);
        if offset >= text.len() {
            offset = text.len();
            if self.line_of_offset(text, offset) <= line {
                return offset;
            }
        } else {
            offset = text.prev_grapheme_offset(offset + 1).unwrap();
        }
        let next_line_offset = self.offset_of_line(text, line + 1);
        if offset >= next_line_offset {
            if let Some(prev) = text.prev_grapheme_offset(next_line_offset) {
                offset = prev;
            }
        }
        offset
    }
    /// Get the line range of a selected region.
    fn get_line_range(&self, text: &Rope, region: &SelRegion) -> Range<usize> {
        let (first_line, _) = self.offset_to_line_col(text, region.min());
        let (mut last_line, last_col) = self.offset_to_line_col(text, region.max());
        if last_col == 0 && last_line > first_line {
            last_line -= 1;
        }
        first_line..(last_line + 1)
    }
}
/// A struct from which the default definitions for `offset_of_line`
/// and `line_of_offset` can be accessed, and think in logical lines.
pub struct LogicalLines;
impl LineOffset for LogicalLines {}
impl LineOffset for xi_rope::breaks::Breaks {
    fn offset_of_line(&self, _text: &Rope, line: usize) -> usize {
        self.count_base_units::<xi_rope::breaks::BreaksMetric>(line)
    }
    fn line_of_offset(&self, text: &Rope, offset: usize) -> usize {
        let offset = offset.min(text.len());
        self.count::<xi_rope::breaks::BreaksMetric>(offset)
    }
}
impl LineOffset for Lines {
    fn offset_of_line(&self, text: &Rope, line: usize) -> usize {
        self.offset_of_visual_line(text, line)
    }
    fn line_of_offset(&self, text: &Rope, offset: usize) -> usize {
        self.visual_line_of_offset(text, offset)
    }
}
#[cfg(test)]
mod tests_llm_16_41 {
    use super::*;
    use crate::*;
    use crate::line_offset::LineOffset;
    use crate::linewrap::*;
    use xi_rope::Rope;
    #[test]
    fn test_offset_of_line() {
        let _rug_st_tests_llm_16_41_rrrruuuugggg_test_offset_of_line = 0;
        let rug_fuzz_0 = "Hello\nWorld\nRust";
        let rug_fuzz_1 = 1;
        let text = Rope::from(rug_fuzz_0);
        let mut lines = Lines::default();
        lines.set_wrap_width(&text, WrapWidth::None);
        let offset = lines.offset_of_line(&text, rug_fuzz_1);
        debug_assert_eq!(offset, 6);
        let _rug_ed_tests_llm_16_41_rrrruuuugggg_test_offset_of_line = 0;
    }
}
