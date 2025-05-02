//! Representation and calculation of movement within a lineoffset.
use std::cmp::max;
use crate::line_offset::LineOffset;
use crate::selection::{HorizPos, SelRegion, Selection};
use crate::word_boundaries::WordCursor;
use xi_rope::{Cursor, LinesMetric, Rope};
/// The specification of a movement.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Movement {
    /// Move to the left by one grapheme cluster.
    Left,
    /// Move to the right by one grapheme cluster.
    Right,
    /// Move to the left by one word.
    LeftWord,
    /// Move to the right by one word.
    RightWord,
    /// Move to left end of visible line.
    LeftOfLine,
    /// Move to right end of visible line.
    RightOfLine,
    /// Move up one visible line.
    Up,
    /// Move down one visible line.
    Down,
    /// Move up one viewport height.
    UpPage,
    /// Move down one viewport height.
    DownPage,
    /// Move up to the next line that can preserve the cursor position.
    UpExactPosition,
    /// Move down to the next line that can preserve the cursor position.
    DownExactPosition,
    /// Move to the start of the text line.
    StartOfParagraph,
    /// Move to the end of the text line.
    EndOfParagraph,
    /// Move to the end of the text line, or next line if already at end.
    EndOfParagraphKill,
    /// Move to the start of the document.
    StartOfDocument,
    /// Move to the end of the document
    EndOfDocument,
}
/// Compute movement based on vertical motion by the given number of lines.
///
/// Note: in non-exceptional cases, this function preserves the `horiz`
/// field of the selection region.
fn vertical_motion(
    r: SelRegion,
    lo: &dyn LineOffset,
    text: &Rope,
    line_delta: isize,
    modify: bool,
) -> (usize, Option<HorizPos>) {
    let (col, line) = selection_position(r, lo, text, line_delta < 0, modify);
    let n_lines = lo.line_of_offset(text, text.len());
    if line_delta < 0 && (-line_delta as usize) > line {
        return (0, Some(col));
    }
    let line = if line_delta < 0 {
        line - (-line_delta as usize)
    } else {
        line.saturating_add(line_delta as usize)
    };
    if line > n_lines {
        return (text.len(), Some(col));
    }
    let new_offset = lo.line_col_to_offset(text, line, col);
    (new_offset, Some(col))
}
/// Compute movement based on vertical motion by the given number of lines skipping
/// any line that is shorter than the current cursor position.
fn vertical_motion_exact_pos(
    r: SelRegion,
    lo: &dyn LineOffset,
    text: &Rope,
    move_up: bool,
    modify: bool,
) -> (usize, Option<HorizPos>) {
    let (col, init_line) = selection_position(r, lo, text, move_up, modify);
    let n_lines = lo.line_of_offset(text, text.len());
    let mut line_length = lo.offset_of_line(text, init_line.saturating_add(1))
        - lo.offset_of_line(text, init_line);
    if move_up && init_line == 0 {
        return (lo.line_col_to_offset(text, init_line, col), Some(col));
    }
    let mut line = if move_up { init_line - 1 } else { init_line.saturating_add(1) };
    let col = if line_length < col { line_length - 1 } else { col };
    loop {
        line_length = lo.offset_of_line(text, line + 1) - lo.offset_of_line(text, line);
        if line_length > col {
            break;
        }
        if line >= n_lines || (line == 0 && move_up) {
            line = init_line;
            break;
        }
        line = if move_up { line - 1 } else { line.saturating_add(1) };
    }
    (lo.line_col_to_offset(text, line, col), Some(col))
}
/// Based on the current selection position this will return the cursor position, the current line, and the
/// total number of lines of the file.
fn selection_position(
    r: SelRegion,
    lo: &dyn LineOffset,
    text: &Rope,
    move_up: bool,
    modify: bool,
) -> (HorizPos, usize) {
    let active = if modify { r.end } else if move_up { r.min() } else { r.max() };
    let col = if let Some(col) = r.horiz {
        col
    } else {
        lo.offset_to_line_col(text, active).1
    };
    let line = lo.line_of_offset(text, active);
    (col, line)
}
/// When paging through a file, the number of lines from the previous page
/// that will also be visible in the next.
const SCROLL_OVERLAP: isize = 2;
/// Computes the actual desired amount of scrolling (generally slightly
/// less than the height of the viewport, to allow overlap).
fn scroll_height(height: usize) -> isize {
    max(height as isize - SCROLL_OVERLAP, 1)
}
/// Compute the result of movement on one selection region.
///
/// # Arguments
///
/// * `height` - viewport height
pub fn region_movement(
    m: Movement,
    r: SelRegion,
    lo: &dyn LineOffset,
    height: usize,
    text: &Rope,
    modify: bool,
) -> SelRegion {
    let (offset, horiz) = match m {
        Movement::Left => {
            if r.is_caret() || modify {
                if let Some(offset) = text.prev_grapheme_offset(r.end) {
                    (offset, None)
                } else {
                    (0, r.horiz)
                }
            } else {
                (r.min(), None)
            }
        }
        Movement::Right => {
            if r.is_caret() || modify {
                if let Some(offset) = text.next_grapheme_offset(r.end) {
                    (offset, None)
                } else {
                    (r.end, r.horiz)
                }
            } else {
                (r.max(), None)
            }
        }
        Movement::LeftWord => {
            let mut word_cursor = WordCursor::new(text, r.end);
            let offset = word_cursor.prev_boundary().unwrap_or(0);
            (offset, None)
        }
        Movement::RightWord => {
            let mut word_cursor = WordCursor::new(text, r.end);
            let offset = word_cursor.next_boundary().unwrap_or_else(|| text.len());
            (offset, None)
        }
        Movement::LeftOfLine => {
            let line = lo.line_of_offset(text, r.end);
            let offset = lo.offset_of_line(text, line);
            (offset, None)
        }
        Movement::RightOfLine => {
            let line = lo.line_of_offset(text, r.end);
            let mut offset = text.len();
            let next_line_offset = lo.offset_of_line(text, line + 1);
            if line < lo.line_of_offset(text, offset) {
                if let Some(prev) = text.prev_grapheme_offset(next_line_offset) {
                    offset = prev;
                }
            }
            (offset, None)
        }
        Movement::Up => vertical_motion(r, lo, text, -1, modify),
        Movement::Down => vertical_motion(r, lo, text, 1, modify),
        Movement::UpExactPosition => vertical_motion_exact_pos(r, lo, text, true, modify),
        Movement::DownExactPosition => {
            vertical_motion_exact_pos(r, lo, text, false, modify)
        }
        Movement::StartOfParagraph => {
            let mut cursor = Cursor::new(&text, r.end);
            let offset = cursor.prev::<LinesMetric>().unwrap_or(0);
            (offset, None)
        }
        Movement::EndOfParagraph => {
            let mut offset = r.end;
            let mut cursor = Cursor::new(&text, offset);
            if let Some(next_para_offset) = cursor.next::<LinesMetric>() {
                if cursor.is_boundary::<LinesMetric>() {
                    if let Some(eol) = text.prev_grapheme_offset(next_para_offset) {
                        offset = eol;
                    }
                } else if cursor.pos() == text.len() {
                    offset = text.len();
                }
                (offset, None)
            } else {
                (text.len(), None)
            }
        }
        Movement::EndOfParagraphKill => {
            let mut offset = r.end;
            let mut cursor = Cursor::new(&text, offset);
            if let Some(next_para_offset) = cursor.next::<LinesMetric>() {
                offset = next_para_offset;
                if cursor.is_boundary::<LinesMetric>() {
                    if let Some(eol) = text.prev_grapheme_offset(next_para_offset) {
                        if eol != r.end {
                            offset = eol;
                        }
                    }
                }
            }
            (offset, None)
        }
        Movement::UpPage => vertical_motion(r, lo, text, -scroll_height(height), modify),
        Movement::DownPage => vertical_motion(r, lo, text, scroll_height(height), modify),
        Movement::StartOfDocument => (0, None),
        Movement::EndOfDocument => (text.len(), None),
    };
    SelRegion::new(if modify { r.start } else { offset }, offset).with_horiz(horiz)
}
/// Compute a new selection by applying a movement to an existing selection.
///
/// In a multi-region selection, this function applies the movement to each
/// region in the selection, and returns the union of the results.
///
/// If `modify` is `true`, the selections are modified, otherwise the results
/// of individual region movements become carets.
///
/// # Arguments
///
/// * `height` - viewport height
pub fn selection_movement(
    m: Movement,
    s: &Selection,
    lo: &dyn LineOffset,
    height: usize,
    text: &Rope,
    modify: bool,
) -> Selection {
    let mut result = Selection::new();
    for &r in s.iter() {
        let new_region = region_movement(m, r, lo, height, text, modify);
        result.add_region(new_region);
    }
    result
}
#[cfg(test)]
mod tests_llm_16_581 {
    use super::*;
    use crate::*;
    const SCROLL_OVERLAP: isize = 5;
    #[test]
    fn test_scroll_height() {
        let _rug_st_tests_llm_16_581_rrrruuuugggg_test_scroll_height = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 20;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 0;
        debug_assert_eq!(scroll_height(rug_fuzz_0), 5);
        debug_assert_eq!(scroll_height(rug_fuzz_1), 15);
        debug_assert_eq!(scroll_height(rug_fuzz_2), 1);
        debug_assert_eq!(scroll_height(rug_fuzz_3), 1);
        let _rug_ed_tests_llm_16_581_rrrruuuugggg_test_scroll_height = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_584 {
    use crate::movement::*;
    use crate::selection::{SelRegion, Affinity};
    struct DummyLineOffset;
    impl LineOffset for DummyLineOffset {
        fn offset_to_line_col(&self, _text: &Rope, _offset: usize) -> (usize, usize) {
            unimplemented!()
        }
        fn line_of_offset(&self, _text: &Rope, _offset: usize) -> usize {
            unimplemented!()
        }
    }
    #[test]
    fn test_selection_position() {
        let _rug_st_tests_llm_16_584_rrrruuuugggg_test_selection_position = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 20;
        let rug_fuzz_2 = "Hello, World!";
        let rug_fuzz_3 = true;
        let rug_fuzz_4 = true;
        let rug_fuzz_5 = 10;
        let r = SelRegion::new(rug_fuzz_0, rug_fuzz_1);
        let lo = DummyLineOffset;
        let text = Rope::from(rug_fuzz_2);
        let move_up = rug_fuzz_3;
        let modify = rug_fuzz_4;
        let expected = (HorizPos::default(), rug_fuzz_5);
        let result = selection_position(r, &lo, &text, move_up, modify);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_584_rrrruuuugggg_test_selection_position = 0;
    }
}
