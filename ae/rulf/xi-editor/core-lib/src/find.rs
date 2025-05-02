//! Module for searching text.
use std::cmp::{max, min};
use std::iter;
use crate::annotations::{AnnotationRange, AnnotationSlice, AnnotationType, ToAnnotation};
use crate::line_offset::LineOffset;
use crate::selection::{InsertDrift, SelRegion, Selection};
use crate::view::View;
use crate::word_boundaries::WordCursor;
use regex::{Regex, RegexBuilder};
use xi_rope::delta::DeltaRegion;
use xi_rope::find::{find, is_multiline_regex, CaseMatching};
use xi_rope::{Cursor, Interval, LinesMetric, Metric, Rope, RopeDelta};
const REGEX_SIZE_LIMIT: usize = 1000000;
/// Information about search queries and number of matches for find
#[derive(Serialize, Deserialize, Debug)]
pub struct FindStatus {
    /// Identifier for the current search query.
    id: usize,
    /// The current search query.
    chars: Option<String>,
    /// Whether the active search is case matching.
    case_sensitive: Option<bool>,
    /// Whether the search query is considered as regular expression.
    is_regex: Option<bool>,
    /// Query only matches whole words.
    whole_words: Option<bool>,
    /// Total number of matches.
    matches: usize,
    /// Line numbers which have find results.
    lines: Vec<usize>,
}
/// Contains logic to search text
pub struct Find {
    /// Uniquely identifies this search query.
    id: usize,
    /// The occurrences, which determine the highlights, have been updated.
    hls_dirty: bool,
    /// The currently active search string.
    search_string: Option<String>,
    /// The case matching setting for the currently active search.
    case_matching: CaseMatching,
    /// The search query should be considered as regular expression.
    regex: Option<Regex>,
    /// Query matches only whole words.
    whole_words: bool,
    /// The set of all known find occurrences (highlights).
    occurrences: Selection,
}
impl Find {
    pub fn new(id: usize) -> Find {
        Find {
            id,
            hls_dirty: true,
            search_string: None,
            case_matching: CaseMatching::CaseInsensitive,
            regex: None,
            whole_words: false,
            occurrences: Selection::new(),
        }
    }
    pub fn id(&self) -> usize {
        self.id
    }
    pub fn occurrences(&self) -> &Selection {
        &self.occurrences
    }
    pub fn hls_dirty(&self) -> bool {
        self.hls_dirty
    }
    pub fn find_status(
        &self,
        view: &View,
        text: &Rope,
        matches_only: bool,
    ) -> FindStatus {
        if matches_only {
            FindStatus {
                id: self.id,
                chars: None,
                case_sensitive: None,
                is_regex: None,
                whole_words: None,
                matches: self.occurrences.len(),
                lines: Vec::new(),
            }
        } else {
            FindStatus {
                id: self.id,
                chars: self.search_string.clone(),
                case_sensitive: Some(self.case_matching == CaseMatching::Exact),
                is_regex: Some(self.regex.is_some()),
                whole_words: Some(self.whole_words),
                matches: self.occurrences.len(),
                lines: self
                    .occurrences
                    .iter()
                    .map(|o| view.offset_to_line_col(text, o.min()).0 + 1)
                    .collect(),
            }
        }
    }
    pub fn set_hls_dirty(&mut self, is_dirty: bool) {
        self.hls_dirty = is_dirty;
    }
    pub fn update_highlights(&mut self, text: &Rope, delta: &RopeDelta) {
        if self.search_string.is_some() {
            for DeltaRegion { old_offset, len, .. } in delta.iter_deletions() {
                self.occurrences.delete_range(old_offset, old_offset + len, false);
            }
            self
                .occurrences = self
                .occurrences
                .apply_delta(delta, false, InsertDrift::Default);
            for DeltaRegion { new_offset, len, .. } in delta.iter_inserts() {
                self.occurrences
                    .delete_range(new_offset.saturating_sub(1), new_offset + len, false);
            }
            let (iv, new_len) = delta.summary();
            let start = match self.occurrences.regions_in_range(0, iv.start()).last() {
                Some(reg) => reg.end,
                None => 0,
            };
            let is_multiline = LinesMetric::next(self.search_string.as_ref().unwrap(), 0)
                .is_some();
            if is_multiline || self.is_multiline_regex() {
                self.occurrences.delete_range(iv.start(), text.len(), false);
                self.update_find(text, start, text.len(), false);
            } else {
                let mut cursor = Cursor::new(&text, iv.end() + new_len);
                let end_of_line = match cursor.next::<LinesMetric>() {
                    Some(end) => end,
                    None if cursor.pos() == text.len() => cursor.pos(),
                    _ => return,
                };
                self.occurrences.delete_range(iv.start(), end_of_line, false);
                self.update_find(text, start, end_of_line, false);
            }
        }
    }
    /// Returns `true` if the search query is a multi-line regex.
    pub(crate) fn is_multiline_regex(&self) -> bool {
        self.regex.is_some() && is_multiline_regex(self.search_string.as_ref().unwrap())
    }
    /// Unsets the search and removes all highlights from the view.
    pub fn unset(&mut self) {
        self.search_string = None;
        self.occurrences = Selection::new();
        self.hls_dirty = true;
    }
    /// Sets find parameters and search query. Returns `true` if parameters have been updated.
    /// Returns `false` to indicate that parameters haven't change.
    pub(crate) fn set_find(
        &mut self,
        search_string: &str,
        case_sensitive: bool,
        is_regex: bool,
        whole_words: bool,
    ) -> bool {
        if search_string.is_empty() {
            self.unset();
        }
        let case_matching = if case_sensitive {
            CaseMatching::Exact
        } else {
            CaseMatching::CaseInsensitive
        };
        if let Some(ref s) = self.search_string {
            if s == search_string && case_matching == self.case_matching
                && self.regex.is_some() == is_regex && self.whole_words == whole_words
            {
                return false;
            }
        }
        self.unset();
        self.search_string = Some(search_string.to_string());
        self.case_matching = case_matching;
        self.whole_words = whole_words;
        self
            .regex = match is_regex {
            false => None,
            true => {
                RegexBuilder::new(search_string)
                    .size_limit(REGEX_SIZE_LIMIT)
                    .case_insensitive(case_matching == CaseMatching::CaseInsensitive)
                    .build()
                    .ok()
            }
        };
        true
    }
    /// Execute the search on the provided text in the range provided by `start` and `end`.
    pub fn update_find(
        &mut self,
        text: &Rope,
        start: usize,
        end: usize,
        include_slop: bool,
    ) {
        if self.search_string.is_none() {
            return;
        }
        let slop = if include_slop {
            self.search_string.as_ref().unwrap().len() * 2
        } else {
            0
        };
        let search_string = self.search_string.as_ref().unwrap();
        let expanded_start = max(start, slop) - slop;
        let expanded_end = min(end + slop, text.len());
        let from = text.at_or_prev_codepoint_boundary(expanded_start).unwrap_or(0);
        let to = text.at_or_next_codepoint_boundary(expanded_end).unwrap_or(text.len());
        let mut to_cursor = Cursor::new(&text, to);
        let _ = to_cursor.next_leaf();
        let sub_text = text.subseq(Interval::new(0, to_cursor.pos()));
        let mut find_cursor = Cursor::new(&sub_text, from);
        let mut raw_lines = text.lines_raw(from..to);
        while let Some(start)
            = find(
                &mut find_cursor,
                &mut raw_lines,
                self.case_matching,
                &search_string,
                self.regex.as_ref(),
            ) {
            let end = find_cursor.pos();
            if self.whole_words && !self.is_matching_whole_words(text, start, end) {
                raw_lines = text.lines_raw(find_cursor.pos()..to);
                continue;
            }
            let region = SelRegion::new(start, end);
            let (_, e) = self.occurrences.add_range_distinct(region);
            if e != end {
                find_cursor.set(e);
                raw_lines = text.lines_raw(find_cursor.pos()..to);
                continue;
            }
            if start == end {
                if end + 1 >= text.len() {
                    break;
                } else {
                    find_cursor.set(end + 1);
                }
            }
            raw_lines = text.lines_raw(find_cursor.pos()..to);
        }
        self.hls_dirty = true;
    }
    /// Return the occurrence closest to the provided selection `sel`. If searched is reversed then
    /// the occurrence closest to the start of the selection is returned. `wrapped` indicates that
    /// if the end of the text is reached the search continues from the start.
    pub fn next_occurrence(
        &self,
        text: &Rope,
        reverse: bool,
        wrapped: bool,
        sel: &Selection,
    ) -> Option<SelRegion> {
        if self.occurrences.len() == 0 {
            return None;
        }
        let (sel_start, sel_end) = match sel.last() {
            Some(last) if last.is_caret() => (last.min(), last.max()),
            Some(last) if !last.is_caret() => (last.min(), last.max() + 1),
            _ => (0, 0),
        };
        if reverse {
            let next_occurrence = match sel_start.checked_sub(1) {
                Some(search_end) => {
                    self.occurrences.regions_in_range(0, search_end).last()
                }
                None => None,
            };
            if next_occurrence.is_none() && !wrapped {
                return self
                    .occurrences
                    .regions_in_range(0, text.len())
                    .iter()
                    .cloned()
                    .filter(|o| sel.regions_in_range(o.min(), o.max()).is_empty())
                    .collect::<Vec<SelRegion>>()
                    .last()
                    .cloned();
            }
            next_occurrence.cloned()
        } else {
            let next_occurrence = self
                .occurrences
                .regions_in_range(sel_end, text.len())
                .first();
            if next_occurrence.is_none() && !wrapped {
                return self
                    .occurrences
                    .regions_in_range(0, text.len())
                    .iter()
                    .cloned()
                    .filter(|o| sel.regions_in_range(o.min(), o.max()).is_empty())
                    .collect::<Vec<SelRegion>>()
                    .first()
                    .cloned();
            }
            next_occurrence.cloned()
        }
    }
    /// Checks if the start and end of a match is matching whole words.
    fn is_matching_whole_words(&self, text: &Rope, start: usize, end: usize) -> bool {
        let mut word_end_cursor = WordCursor::new(text, end - 1);
        let mut word_start_cursor = WordCursor::new(text, start + 1);
        if let Some(start_boundary) = word_start_cursor.prev_boundary() {
            if start_boundary != start {
                return false;
            }
        }
        if let Some(end_boundary) = word_end_cursor.next_boundary() {
            if end_boundary != end {
                return false;
            }
        }
        true
    }
}
/// Implementing the `ToAnnotation` trait allows to convert finds to annotations.
impl ToAnnotation for Find {
    fn get_annotations(
        &self,
        interval: Interval,
        view: &View,
        text: &Rope,
    ) -> AnnotationSlice {
        let regions = self
            .occurrences
            .regions_in_range(interval.start(), interval.end());
        let ranges = regions
            .iter()
            .map(|region| {
                let (start_line, start_col) = view
                    .offset_to_line_col(text, region.min());
                let (end_line, end_col) = view.offset_to_line_col(text, region.max());
                AnnotationRange {
                    start_line,
                    start_col,
                    end_line,
                    end_col,
                }
            })
            .collect::<Vec<AnnotationRange>>();
        let payload = iter::repeat(json!({ "id" : self.id }))
            .take(ranges.len())
            .collect::<Vec<_>>();
        AnnotationSlice::new(AnnotationType::Find, ranges, Some(payload))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use xi_rope::DeltaBuilder;
    #[test]
    fn find() {
        let base_text = Rope::from("hello world");
        let mut find = Find::new(1);
        find.set_find("world", false, false, false);
        find.update_find(&base_text, 0, base_text.len(), false);
        assert_eq!(find.occurrences().len(), 1);
        assert_eq!(find.occurrences().first(), Some(& SelRegion::new(6, 11)));
    }
    #[test]
    fn find_whole_words() {
        let base_text = Rope::from("hello world\n many worlds");
        let mut find = Find::new(1);
        find.set_find("world", false, false, true);
        find.update_find(&base_text, 0, base_text.len(), false);
        assert_eq!(find.occurrences().len(), 1);
        assert_eq!(find.occurrences().first(), Some(& SelRegion::new(6, 11)));
    }
    #[test]
    fn find_case_sensitive() {
        let base_text = Rope::from("hello world\n HELLO WORLD");
        let mut find = Find::new(1);
        find.set_find("world", true, false, false);
        find.update_find(&base_text, 0, base_text.len(), false);
        assert_eq!(find.occurrences().len(), 1);
        assert_eq!(find.occurrences().first(), Some(& SelRegion::new(6, 11)));
    }
    #[test]
    fn find_multiline() {
        let base_text = Rope::from("hello world\n HELLO WORLD");
        let mut find = Find::new(1);
        find.set_find("hello world\n HELLO", true, false, false);
        find.update_find(&base_text, 0, base_text.len(), false);
        assert_eq!(find.occurrences().len(), 1);
        assert_eq!(find.occurrences().first(), Some(& SelRegion::new(0, 18)));
    }
    #[test]
    fn find_regex() {
        let base_text = Rope::from("hello world\n HELLO WORLD");
        let mut find = Find::new(1);
        find.set_find("hello \\w+", false, true, false);
        find.update_find(&base_text, 0, base_text.len(), false);
        assert_eq!(find.occurrences().len(), 2);
        assert_eq!(find.occurrences().first(), Some(& SelRegion::new(0, 11)));
        find.set_find("h.llo", true, true, false);
        find.update_find(&base_text, 0, base_text.len(), false);
        assert_eq!(find.occurrences().len(), 1);
        assert_eq!(find.occurrences().first(), Some(& SelRegion::new(0, 5)));
        find.set_find(".*", false, true, false);
        find.update_find(&base_text, 0, base_text.len(), false);
        assert_eq!(find.occurrences().len(), 3);
        assert_eq!(find.occurrences().first(), Some(& SelRegion::new(0, 11)));
    }
    #[test]
    fn find_regex_multiline() {
        let base_text = Rope::from("hello world\n HELLO WORLD");
        let mut find = Find::new(1);
        find.set_find("(.*\n.*)+", true, true, false);
        find.update_find(&base_text, 0, base_text.len(), false);
        assert_eq!(find.occurrences().len(), 1);
        assert_eq!(find.occurrences().first(), Some(& SelRegion::new(0, 12)));
    }
    #[test]
    fn find_multiline_regex() {
        let mut find = Find::new(1);
        find.set_find("a", true, true, false);
        assert_eq!(find.is_multiline_regex(), false);
        find.set_find(".*", true, true, false);
        assert_eq!(find.is_multiline_regex(), false);
        find.set_find("\\n", true, true, false);
        assert_eq!(find.is_multiline_regex(), true);
    }
    #[test]
    fn find_slop() {
        let base_text = Rope::from("aaa bbb aaa bbb aaa x");
        let mut find = Find::new(1);
        find.set_find("aaa", true, true, false);
        find.update_find(&base_text, 2, base_text.len(), false);
        assert_eq!(find.occurrences().len(), 2);
        assert_eq!(find.occurrences().first(), Some(& SelRegion::new(8, 11)));
        find.update_find(&base_text, 3, base_text.len(), true);
        assert_eq!(find.occurrences().len(), 3);
        assert_eq!(find.occurrences().first(), Some(& SelRegion::new(0, 3)));
    }
    #[test]
    fn find_next_occurrence() {
        let base_text = Rope::from("aaa bbb aaa bbb aaa x");
        let mut find = Find::new(1);
        find.set_find("aaa", true, true, false);
        find.update_find(&base_text, 0, base_text.len(), false);
        assert_eq!(find.occurrences().len(), 3);
        assert_eq!(
            find.next_occurrence(& base_text, false, false, & Selection::new()),
            Some(SelRegion::new(0, 3))
        );
        let mut prev_selection = Selection::new();
        prev_selection.add_region(SelRegion::new(0, 3));
        assert_eq!(
            find.next_occurrence(& base_text, false, false, & prev_selection),
            Some(SelRegion::new(8, 11))
        );
        let mut prev_selection = Selection::new();
        prev_selection.add_region(SelRegion::new(19, 19));
        assert_eq!(
            find.next_occurrence(& base_text, false, true, & prev_selection),
            Some(SelRegion::new(16, 19))
        );
        let mut prev_selection = Selection::new();
        prev_selection.add_region(SelRegion::new(20, 20));
        assert_eq!(
            find.next_occurrence(& base_text, false, false, & prev_selection),
            Some(SelRegion::new(0, 3))
        );
    }
    #[test]
    fn find_previous_occurrence() {
        let base_text = Rope::from("aaa bbb aaa bbb aaa x");
        let mut find = Find::new(1);
        find.set_find("aaa", true, true, false);
        find.update_find(&base_text, 0, base_text.len(), false);
        assert_eq!(find.occurrences().len(), 3);
        assert_eq!(
            find.next_occurrence(& base_text, true, false, & Selection::new()),
            Some(SelRegion::new(16, 19))
        );
        let mut prev_selection = Selection::new();
        prev_selection.add_region(SelRegion::new(20, 20));
        assert_eq!(
            find.next_occurrence(& base_text, true, true, & Selection::new()), None
        );
    }
    #[test]
    fn unset_find() {
        let base_text = Rope::from("aaa bbb aaa bbb aaa x");
        let mut find = Find::new(1);
        find.set_find("aaa", true, true, false);
        find.update_find(&base_text, 0, base_text.len(), false);
        assert_eq!(find.occurrences().len(), 3);
        find.unset();
        assert_eq!(find.occurrences().len(), 0);
    }
    #[test]
    fn update_find_edit() {
        let base_text = Rope::from("a b a c");
        let mut find = Find::new(1);
        find.set_find("a", false, false, false);
        find.update_find(&base_text, 0, base_text.len(), false);
        let mut builder = DeltaBuilder::new(base_text.len());
        builder.replace(0..0, "a ".into());
        assert_eq!(find.occurrences().len(), 2);
        assert_eq!(find.occurrences().first(), Some(& SelRegion::new(0, 1)));
        assert_eq!(find.occurrences().last(), Some(& SelRegion::new(4, 5)));
    }
    #[test]
    fn update_find_multiline_edit() {
        let base_text = Rope::from("x\n a\n b\n a\n c");
        let mut find = Find::new(1);
        find.set_find("a", false, false, false);
        find.update_find(&base_text, 0, base_text.len(), false);
        let mut builder = DeltaBuilder::new(base_text.len());
        builder.replace(2..2, " a\n b\n a\n".into());
        assert_eq!(find.occurrences().len(), 2);
        assert_eq!(find.occurrences().first(), Some(& SelRegion::new(3, 4)));
        assert_eq!(find.occurrences().last(), Some(& SelRegion::new(9, 10)));
    }
}
#[cfg(test)]
mod tests_llm_16_437 {
    use super::*;
    use crate::*;
    #[test]
    fn test_hls_dirty() {
        let _rug_st_tests_llm_16_437_rrrruuuugggg_test_hls_dirty = 0;
        let rug_fuzz_0 = 1;
        let find = Find::new(rug_fuzz_0);
        debug_assert_eq!(find.hls_dirty(), true);
        let _rug_ed_tests_llm_16_437_rrrruuuugggg_test_hls_dirty = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_442 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_multiline_regex() {
        let _rug_st_tests_llm_16_442_rrrruuuugggg_test_is_multiline_regex = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = "abc";
        let rug_fuzz_2 = "abc";
        let rug_fuzz_3 = "abc\ndef";
        let rug_fuzz_4 = "abc\ndef";
        let mut find = Find::new(rug_fuzz_0);
        find.search_string = Some(String::from(rug_fuzz_1));
        find.regex = Some(Regex::new(rug_fuzz_2).unwrap());
        debug_assert_eq!(find.is_multiline_regex(), false);
        find.search_string = Some(String::from(rug_fuzz_3));
        find.regex = Some(Regex::new(rug_fuzz_4).unwrap());
        debug_assert_eq!(find.is_multiline_regex(), true);
        let _rug_ed_tests_llm_16_442_rrrruuuugggg_test_is_multiline_regex = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_448 {
    use super::*;
    use crate::*;
    use find::CaseMatching;
    #[test]
    fn test_set_find() {
        let _rug_st_tests_llm_16_448_rrrruuuugggg_test_set_find = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = "";
        let rug_fuzz_2 = true;
        let rug_fuzz_3 = false;
        let rug_fuzz_4 = true;
        let rug_fuzz_5 = "test";
        let rug_fuzz_6 = true;
        let rug_fuzz_7 = false;
        let rug_fuzz_8 = true;
        let rug_fuzz_9 = "test";
        let rug_fuzz_10 = true;
        let rug_fuzz_11 = false;
        let rug_fuzz_12 = true;
        let rug_fuzz_13 = "test";
        let rug_fuzz_14 = true;
        let rug_fuzz_15 = false;
        let rug_fuzz_16 = true;
        let rug_fuzz_17 = "new";
        let rug_fuzz_18 = false;
        let rug_fuzz_19 = false;
        let rug_fuzz_20 = true;
        let mut find = Find::new(rug_fuzz_0);
        debug_assert_eq!(
            find.set_find(rug_fuzz_1, rug_fuzz_2, rug_fuzz_3, rug_fuzz_4), false
        );
        find.set_find(rug_fuzz_5, rug_fuzz_6, rug_fuzz_7, rug_fuzz_8);
        debug_assert_eq!(
            find.set_find(rug_fuzz_9, rug_fuzz_10, rug_fuzz_11, rug_fuzz_12), false
        );
        find.set_find(rug_fuzz_13, rug_fuzz_14, rug_fuzz_15, rug_fuzz_16);
        debug_assert_eq!(
            find.set_find(rug_fuzz_17, rug_fuzz_18, rug_fuzz_19, rug_fuzz_20), true
        );
        let _rug_ed_tests_llm_16_448_rrrruuuugggg_test_set_find = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_450_llm_16_449 {
    use super::*;
    use crate::*;
    #[test]
    fn test_find_set_hls_dirty() {
        let _rug_st_tests_llm_16_450_llm_16_449_rrrruuuugggg_test_find_set_hls_dirty = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = false;
        let rug_fuzz_2 = true;
        let mut find = Find::new(rug_fuzz_0);
        debug_assert_eq!(find.hls_dirty(), true);
        find.set_hls_dirty(rug_fuzz_1);
        debug_assert_eq!(find.hls_dirty(), false);
        find.set_hls_dirty(rug_fuzz_2);
        debug_assert_eq!(find.hls_dirty(), true);
        let _rug_ed_tests_llm_16_450_llm_16_449_rrrruuuugggg_test_find_set_hls_dirty = 0;
    }
}
