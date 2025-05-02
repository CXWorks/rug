//! Functions for editing ropes.
use std::borrow::Cow;
use std::collections::BTreeSet;
use xi_rope::{Cursor, DeltaBuilder, Interval, LinesMetric, Rope, RopeDelta};
use crate::backspace::offset_for_delete_backwards;
use crate::config::BufferItems;
use crate::line_offset::{LineOffset, LogicalLines};
use crate::linewrap::Lines;
use crate::movement::{region_movement, Movement};
use crate::selection::{SelRegion, Selection};
use crate::word_boundaries::WordCursor;
#[derive(Debug, Copy, Clone)]
pub enum IndentDirection {
    In,
    Out,
}
/// Replaces the selection with the text `T`.
pub fn insert<T: Into<Rope>>(base: &Rope, regions: &[SelRegion], text: T) -> RopeDelta {
    let rope = text.into();
    let mut builder = DeltaBuilder::new(base.len());
    for region in regions {
        let iv = Interval::new(region.min(), region.max());
        builder.replace(iv, rope.clone());
    }
    builder.build()
}
/// Leaves the current selection untouched, but surrounds it with two insertions.
pub fn surround<BT, AT>(
    base: &Rope,
    regions: &[SelRegion],
    before_text: BT,
    after_text: AT,
) -> RopeDelta
where
    BT: Into<Rope>,
    AT: Into<Rope>,
{
    let mut builder = DeltaBuilder::new(base.len());
    let before_rope = before_text.into();
    let after_rope = after_text.into();
    for region in regions {
        let before_iv = Interval::new(region.min(), region.min());
        builder.replace(before_iv, before_rope.clone());
        let after_iv = Interval::new(region.max(), region.max());
        builder.replace(after_iv, after_rope.clone());
    }
    builder.build()
}
pub fn duplicate_line(
    base: &Rope,
    regions: &[SelRegion],
    config: &BufferItems,
) -> RopeDelta {
    let mut builder = DeltaBuilder::new(base.len());
    let mut to_duplicate = BTreeSet::new();
    for region in regions {
        let (first_line, _) = LogicalLines.offset_to_line_col(base, region.min());
        let line_start = LogicalLines.offset_of_line(base, first_line);
        let mut cursor = match region.is_caret() {
            true => Cursor::new(base, line_start),
            false => {
                let (last_line, _) = LogicalLines.offset_to_line_col(base, region.max());
                let line_end = LogicalLines.offset_of_line(base, last_line);
                Cursor::new(base, line_end)
            }
        };
        if let Some(line_end) = cursor.next::<LinesMetric>() {
            to_duplicate.insert((line_start, line_end));
        }
    }
    for (start, end) in to_duplicate {
        let iv = Interval::new(start, start);
        builder.replace(iv, base.slice(start..end));
        if end == base.len() {
            builder.replace(iv, Rope::from(&config.line_ending))
        }
    }
    builder.build()
}
/// Used when the user presses the backspace key. If no delta is returned, then nothing changes.
pub fn delete_backward(
    base: &Rope,
    regions: &[SelRegion],
    config: &BufferItems,
) -> RopeDelta {
    let mut builder = DeltaBuilder::new(base.len());
    for region in regions {
        let start = offset_for_delete_backwards(&region, base, &config);
        let iv = Interval::new(start, region.max());
        if !iv.is_empty() {
            builder.delete(iv);
        }
    }
    builder.build()
}
/// Common logic for a number of delete methods. For each region in the
/// selection, if the selection is a caret, delete the region between
/// the caret and the movement applied to the caret, otherwise delete
/// the region.
///
/// If `save` is set, the tuple will contain a rope with the deleted text.
///
/// # Arguments
///
/// * `height` - viewport height
pub(crate) fn delete_by_movement(
    base: &Rope,
    regions: &[SelRegion],
    lines: &Lines,
    movement: Movement,
    height: usize,
    save: bool,
) -> (RopeDelta, Option<Rope>) {
    let mut deletions = Selection::new();
    for &r in regions {
        if r.is_caret() {
            let new_region = region_movement(movement, r, lines, height, base, true);
            deletions.add_region(new_region);
        } else {
            deletions.add_region(r);
        }
    }
    let kill_ring = if save {
        let saved = extract_sel_regions(base, &deletions).unwrap_or_default();
        Some(Rope::from(saved))
    } else {
        None
    };
    (delete_sel_regions(base, &deletions), kill_ring)
}
/// Deletes the given regions.
pub(crate) fn delete_sel_regions(base: &Rope, sel_regions: &[SelRegion]) -> RopeDelta {
    let mut builder = DeltaBuilder::new(base.len());
    for region in sel_regions {
        let iv = Interval::new(region.min(), region.max());
        if !iv.is_empty() {
            builder.delete(iv);
        }
    }
    builder.build()
}
/// Extracts non-caret selection regions into a string,
/// joining multiple regions with newlines.
pub(crate) fn extract_sel_regions<'a>(
    base: &'a Rope,
    sel_regions: &[SelRegion],
) -> Option<Cow<'a, str>> {
    let mut saved = None;
    for region in sel_regions {
        if !region.is_caret() {
            let val = base.slice_to_cow(region);
            match saved {
                None => saved = Some(val),
                Some(ref mut s) => {
                    s.to_mut().push('\n');
                    s.to_mut().push_str(&val);
                }
            }
        }
    }
    saved
}
pub fn insert_newline(
    base: &Rope,
    regions: &[SelRegion],
    config: &BufferItems,
) -> RopeDelta {
    insert(base, regions, &config.line_ending)
}
pub fn insert_tab(
    base: &Rope,
    regions: &[SelRegion],
    config: &BufferItems,
) -> RopeDelta {
    let mut builder = DeltaBuilder::new(base.len());
    let const_tab_text = get_tab_text(config, None);
    for region in regions {
        let line_range = LogicalLines.get_line_range(base, region);
        if line_range.len() > 1 {
            for line in line_range {
                let offset = LogicalLines.line_col_to_offset(base, line, 0);
                let iv = Interval::new(offset, offset);
                builder.replace(iv, Rope::from(const_tab_text));
            }
        } else {
            let (_, col) = LogicalLines.offset_to_line_col(base, region.start);
            let mut tab_size = config.tab_size;
            tab_size = tab_size - (col % tab_size);
            let tab_text = get_tab_text(config, Some(tab_size));
            let iv = Interval::new(region.min(), region.max());
            builder.replace(iv, Rope::from(tab_text));
        }
    }
    builder.build()
}
/// Indents or outdents lines based on selection and user's tab settings.
/// Uses a BTreeSet to holds the collection of lines to modify.
/// Preserves cursor position and current selection as much as possible.
/// Tries to have behavior consistent with other editors like Atom,
/// Sublime and VSCode, with non-caret selections not being modified.
pub fn modify_indent(
    base: &Rope,
    regions: &[SelRegion],
    config: &BufferItems,
    direction: IndentDirection,
) -> RopeDelta {
    let mut lines = BTreeSet::new();
    let tab_text = get_tab_text(config, None);
    for region in regions {
        let line_range = LogicalLines.get_line_range(base, region);
        for line in line_range {
            lines.insert(line);
        }
    }
    match direction {
        IndentDirection::In => indent(base, lines, tab_text),
        IndentDirection::Out => outdent(base, lines, tab_text),
    }
}
fn indent(base: &Rope, lines: BTreeSet<usize>, tab_text: &str) -> RopeDelta {
    let mut builder = DeltaBuilder::new(base.len());
    for line in lines {
        let offset = LogicalLines.line_col_to_offset(base, line, 0);
        let interval = Interval::new(offset, offset);
        builder.replace(interval, Rope::from(tab_text));
    }
    builder.build()
}
fn outdent(base: &Rope, lines: BTreeSet<usize>, tab_text: &str) -> RopeDelta {
    let mut builder = DeltaBuilder::new(base.len());
    for line in lines {
        let offset = LogicalLines.line_col_to_offset(base, line, 0);
        let tab_offset = LogicalLines.line_col_to_offset(base, line, tab_text.len());
        let interval = Interval::new(offset, tab_offset);
        let leading_slice = base.slice_to_cow(interval.start()..interval.end());
        if leading_slice == tab_text {
            builder.delete(interval);
        } else if let Some(first_char_col)
            = leading_slice.find(|c: char| !c.is_whitespace())
        {
            let first_char_offset = LogicalLines
                .line_col_to_offset(base, line, first_char_col);
            let interval = Interval::new(offset, first_char_offset);
            builder.delete(interval);
        }
    }
    builder.build()
}
pub fn transpose(base: &Rope, regions: &[SelRegion]) -> RopeDelta {
    let mut builder = DeltaBuilder::new(base.len());
    let mut last = 0;
    let mut optional_previous_selection: Option<(Interval, Rope)> = last_selection_region(
            regions,
        )
        .map(|&region| sel_region_to_interval_and_rope(base, region));
    for &region in regions {
        if region.is_caret() {
            let mut middle = region.end;
            let mut start = base.prev_grapheme_offset(middle).unwrap_or(0);
            let mut end = base.next_grapheme_offset(middle).unwrap_or(middle);
            if start >= last {
                let end_line_offset = LogicalLines
                    .offset_of_line(base, LogicalLines.line_of_offset(base, end));
                if (end == middle || end == end_line_offset) && end != base.len() {
                    middle = start;
                    start = base.prev_grapheme_offset(middle).unwrap_or(0);
                    end = middle.wrapping_add(1);
                }
                let interval = Interval::new(start, end);
                let before = base.slice_to_cow(start..middle);
                let after = base.slice_to_cow(middle..end);
                let swapped: String = [after, before].concat();
                builder.replace(interval, Rope::from(swapped));
                last = end;
            }
        } else if let Some(previous_selection) = optional_previous_selection {
            let current_interval = sel_region_to_interval_and_rope(base, region);
            builder.replace(current_interval.0, previous_selection.1);
            optional_previous_selection = Some(current_interval);
        }
    }
    builder.build()
}
pub fn transform_text<F: Fn(&str) -> String>(
    base: &Rope,
    regions: &[SelRegion],
    transform_function: F,
) -> RopeDelta {
    let mut builder = DeltaBuilder::new(base.len());
    for region in regions {
        let selected_text = base.slice_to_cow(region);
        let interval = Interval::new(region.min(), region.max());
        builder.replace(interval, Rope::from(transform_function(&selected_text)));
    }
    builder.build()
}
/// Changes the number(s) under the cursor(s) with the `transform_function`.
/// If there is a number next to or on the beginning of the region, then
/// this number will be replaced with the result of `transform_function` and
/// the cursor will be placed at the end of the number.
/// Some Examples with a increment `transform_function`:
///
/// "|1234" -> "1235|"
/// "12|34" -> "1235|"
/// "-|12" -> "-11|"
/// "another number is 123|]" -> "another number is 124"
///
/// This function also works fine with multiple regions.
pub fn change_number<F: Fn(i128) -> Option<i128>>(
    base: &Rope,
    regions: &[SelRegion],
    transform_function: F,
) -> RopeDelta {
    let mut builder = DeltaBuilder::new(base.len());
    for region in regions {
        let mut cursor = WordCursor::new(base, region.end);
        let (mut start, end) = cursor.select_word();
        if start > 0 && base.byte_at(start - 1) == (b'-') {
            start -= 1;
        }
        let word = base.slice_to_cow(start..end);
        if let Some(number) = word.parse::<i128>().ok().and_then(&transform_function) {
            let interval = Interval::new(start, end);
            builder.replace(interval, Rope::from(number.to_string()));
        }
    }
    builder.build()
}
pub fn capitalize_text(base: &Rope, regions: &[SelRegion]) -> (RopeDelta, Selection) {
    let mut builder = DeltaBuilder::new(base.len());
    let mut final_selection = Selection::new();
    for &region in regions {
        final_selection.add_region(SelRegion::new(region.max(), region.max()));
        let mut word_cursor = WordCursor::new(base, region.min());
        loop {
            let (start, end) = word_cursor.select_word();
            if start < end {
                let interval = Interval::new(start, end);
                let word = base.slice_to_cow(start..end);
                let (first_char, rest) = word.split_at(1);
                let capitalized_text = [first_char.to_uppercase(), rest.to_lowercase()]
                    .concat();
                builder.replace(interval, Rope::from(capitalized_text));
            }
            if word_cursor.next_boundary().is_none() || end > region.max() {
                break;
            }
        }
    }
    (builder.build(), final_selection)
}
fn sel_region_to_interval_and_rope(base: &Rope, region: SelRegion) -> (Interval, Rope) {
    let as_interval = Interval::new(region.min(), region.max());
    let interval_rope = base.subseq(as_interval);
    (as_interval, interval_rope)
}
fn last_selection_region(regions: &[SelRegion]) -> Option<&SelRegion> {
    for region in regions.iter().rev() {
        if !region.is_caret() {
            return Some(region);
        }
    }
    None
}
fn get_tab_text(config: &BufferItems, tab_size: Option<usize>) -> &'static str {
    let tab_size = tab_size.unwrap_or(config.tab_size);
    let tab_text = if config.translate_tabs_to_spaces {
        n_spaces(tab_size)
    } else {
        "\t"
    };
    tab_text
}
fn n_spaces(n: usize) -> &'static str {
    let spaces = "                                ";
    assert!(n <= spaces.len());
    &spaces[..n]
}
#[cfg(test)]
mod tests_llm_16_264 {
    use crate::edit_ops::{extract_sel_regions, SelRegion, Rope};
    use crate::selection::Affinity;
    use std::borrow::Cow;
    fn create_sel_regions() -> Vec<SelRegion> {
        vec![
            SelRegion::new(0, 5).with_affinity(Affinity::Downstream), SelRegion::new(7,
            12).with_affinity(Affinity::Upstream), SelRegion::new(0, 12)
            .with_affinity(Affinity::Downstream),
        ]
    }
    #[test]
    fn test_extract_sel_regions() {
        let rope = Rope::from("Hello, world!");
        let sel_regions = create_sel_regions();
        let expected = Some(Cow::Borrowed("Hello\nworld!"));
        let result = extract_sel_regions(&rope, &sel_regions);
        assert_eq!(result, expected);
    }
}
#[cfg(test)]
mod tests_llm_16_265 {
    use super::*;
    use crate::*;
    use serde_json;
    #[test]
    fn test_get_tab_text() {
        let _rug_st_tests_llm_16_265_rrrruuuugggg_test_get_tab_text = 0;
        let rug_fuzz_0 = "\n";
        let rug_fuzz_1 = 4;
        let rug_fuzz_2 = true;
        let rug_fuzz_3 = false;
        let rug_fuzz_4 = "Arial";
        let rug_fuzz_5 = 12.0;
        let rug_fuzz_6 = true;
        let rug_fuzz_7 = false;
        let rug_fuzz_8 = 80;
        let rug_fuzz_9 = true;
        let rug_fuzz_10 = false;
        let rug_fuzz_11 = "(";
        let rug_fuzz_12 = ")";
        let rug_fuzz_13 = true;
        let config = BufferItems {
            line_ending: rug_fuzz_0.to_owned(),
            tab_size: rug_fuzz_1,
            translate_tabs_to_spaces: rug_fuzz_2,
            use_tab_stops: rug_fuzz_3,
            font_face: rug_fuzz_4.to_owned(),
            font_size: rug_fuzz_5,
            auto_indent: rug_fuzz_6,
            scroll_past_end: rug_fuzz_7,
            wrap_width: rug_fuzz_8,
            word_wrap: rug_fuzz_9,
            autodetect_whitespace: rug_fuzz_10,
            surrounding_pairs: vec![
                (rug_fuzz_11.to_owned(), rug_fuzz_12.to_owned()), ("{".to_owned(), "}"
                .to_owned())
            ],
            save_with_newline: rug_fuzz_13,
        };
        let tab_text = get_tab_text(&config, None);
        debug_assert_eq!(tab_text, "    ");
        let _rug_ed_tests_llm_16_265_rrrruuuugggg_test_get_tab_text = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_270 {
    use super::*;
    use crate::*;
    #[test]
    fn test_insert_newline() {
        let _rug_st_tests_llm_16_270_rrrruuuugggg_test_insert_newline = 0;
        let rug_fuzz_0 = "hello world";
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 5;
        let rug_fuzz_3 = "\n";
        let rug_fuzz_4 = 4;
        let rug_fuzz_5 = true;
        let rug_fuzz_6 = false;
        let rug_fuzz_7 = "Arial";
        let rug_fuzz_8 = 12.0;
        let rug_fuzz_9 = true;
        let rug_fuzz_10 = false;
        let rug_fuzz_11 = 80;
        let rug_fuzz_12 = true;
        let rug_fuzz_13 = true;
        let rug_fuzz_14 = "(";
        let rug_fuzz_15 = ")";
        let rug_fuzz_16 = true;
        let base = Rope::from(rug_fuzz_0);
        let regions = vec![SelRegion::new(rug_fuzz_1, rug_fuzz_2)];
        let config = BufferItems {
            line_ending: String::from(rug_fuzz_3),
            tab_size: rug_fuzz_4,
            translate_tabs_to_spaces: rug_fuzz_5,
            use_tab_stops: rug_fuzz_6,
            font_face: String::from(rug_fuzz_7),
            font_size: rug_fuzz_8,
            auto_indent: rug_fuzz_9,
            scroll_past_end: rug_fuzz_10,
            wrap_width: rug_fuzz_11,
            word_wrap: rug_fuzz_12,
            autodetect_whitespace: rug_fuzz_13,
            surrounding_pairs: vec![
                (rug_fuzz_14.to_string(), rug_fuzz_15.to_string()), ("{".to_string(), "}"
                .to_string())
            ],
            save_with_newline: rug_fuzz_16,
        };
        let result = insert_newline(&base, &regions, &config);
        let _rug_ed_tests_llm_16_270_rrrruuuugggg_test_insert_newline = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_277 {
    use super::*;
    use crate::*;
    #[test]
    fn test_n_spaces() {
        let _rug_st_tests_llm_16_277_rrrruuuugggg_test_n_spaces = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 5;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 20;
        let rug_fuzz_4 = 50;
        debug_assert_eq!(n_spaces(rug_fuzz_0), "");
        debug_assert_eq!(n_spaces(rug_fuzz_1), "     ");
        debug_assert_eq!(n_spaces(rug_fuzz_2), "          ");
        debug_assert_eq!(n_spaces(rug_fuzz_3), "                    ");
        debug_assert_eq!(
            n_spaces(rug_fuzz_4), "                                                  "
        );
        let _rug_ed_tests_llm_16_277_rrrruuuugggg_test_n_spaces = 0;
    }
}
