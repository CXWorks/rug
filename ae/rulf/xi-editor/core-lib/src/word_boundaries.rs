//! Segmentation of word boundaries. Note: this current implementation
//! is intended to work for code. Future work is to make it Unicode aware.
use xi_rope::{Cursor, Rope, RopeInfo};
pub struct WordCursor<'a> {
    inner: Cursor<'a, RopeInfo>,
}
impl<'a> WordCursor<'a> {
    pub fn new(text: &'a Rope, pos: usize) -> WordCursor<'a> {
        let inner = Cursor::new(text, pos);
        WordCursor { inner }
    }
    /// Get previous boundary, and set the cursor at the boundary found.
    pub fn prev_boundary(&mut self) -> Option<usize> {
        if let Some(ch) = self.inner.prev_codepoint() {
            let mut prop = get_word_property(ch);
            let mut candidate = self.inner.pos();
            while let Some(prev) = self.inner.prev_codepoint() {
                let prop_prev = get_word_property(prev);
                if classify_boundary(prop_prev, prop).is_start() {
                    break;
                }
                prop = prop_prev;
                candidate = self.inner.pos();
            }
            self.inner.set(candidate);
            return Some(candidate);
        }
        None
    }
    /// Get next boundary, and set the cursor at the boundary found.
    pub fn next_boundary(&mut self) -> Option<usize> {
        if let Some(ch) = self.inner.next_codepoint() {
            let mut prop = get_word_property(ch);
            let mut candidate = self.inner.pos();
            while let Some(next) = self.inner.next_codepoint() {
                let prop_next = get_word_property(next);
                if classify_boundary(prop, prop_next).is_end() {
                    break;
                }
                prop = prop_next;
                candidate = self.inner.pos();
            }
            self.inner.set(candidate);
            return Some(candidate);
        }
        None
    }
    /// Return the selection for the word containing the current cursor. The
    /// cursor is moved to the end of that selection.
    pub fn select_word(&mut self) -> (usize, usize) {
        let initial = self.inner.pos();
        let init_prop_after = self.inner.next_codepoint().map(get_word_property);
        self.inner.set(initial);
        let init_prop_before = self.inner.prev_codepoint().map(get_word_property);
        let mut start = initial;
        let init_boundary = if let (Some(pb), Some(pa))
            = (init_prop_before, init_prop_after) {
            classify_boundary_initial(pb, pa)
        } else {
            WordBoundary::Both
        };
        let mut prop_after = init_prop_after;
        let mut prop_before = init_prop_before;
        if prop_after.is_none() {
            start = self.inner.pos();
            prop_after = prop_before;
            prop_before = self.inner.prev_codepoint().map(get_word_property);
        }
        while let (Some(pb), Some(pa)) = (prop_before, prop_after) {
            if start == initial {
                if init_boundary.is_start() {
                    break;
                }
            } else if !init_boundary.is_boundary() {
                if classify_boundary(pb, pa).is_boundary() {
                    break;
                }
            } else if classify_boundary(pb, pa).is_start() {
                break;
            }
            start = self.inner.pos();
            prop_after = prop_before;
            prop_before = self.inner.prev_codepoint().map(get_word_property);
        }
        self.inner.set(initial);
        let mut end = initial;
        prop_after = init_prop_after;
        prop_before = init_prop_before;
        if prop_before.is_none() {
            prop_before = self.inner.next_codepoint().map(get_word_property);
            end = self.inner.pos();
            prop_after = self.inner.next_codepoint().map(get_word_property);
        }
        while let (Some(pb), Some(pa)) = (prop_before, prop_after) {
            if end == initial {
                if init_boundary.is_end() {
                    break;
                }
            } else if !init_boundary.is_boundary() {
                if classify_boundary(pb, pa).is_boundary() {
                    break;
                }
            } else if classify_boundary(pb, pa).is_end() {
                break;
            }
            end = self.inner.pos();
            prop_before = prop_after;
            prop_after = self.inner.next_codepoint().map(get_word_property);
        }
        self.inner.set(end);
        (start, end)
    }
}
#[derive(PartialEq, Eq)]
enum WordBoundary {
    Interior,
    Start,
    End,
    Both,
}
impl WordBoundary {
    fn is_start(&self) -> bool {
        *self == WordBoundary::Start || *self == WordBoundary::Both
    }
    fn is_end(&self) -> bool {
        *self == WordBoundary::End || *self == WordBoundary::Both
    }
    fn is_boundary(&self) -> bool {
        *self != WordBoundary::Interior
    }
}
fn classify_boundary(prev: WordProperty, next: WordProperty) -> WordBoundary {
    use self::WordBoundary::*;
    use self::WordProperty::*;
    match (prev, next) {
        (Lf, _) => Both,
        (_, Lf) => Both,
        (Space, Other) => Start,
        (Space, Punctuation) => Start,
        (Punctuation, Other) => Start,
        (Other, Space) => End,
        (Punctuation, Space) => End,
        (Other, Punctuation) => End,
        _ => Interior,
    }
}
fn classify_boundary_initial(prev: WordProperty, next: WordProperty) -> WordBoundary {
    use self::WordBoundary::*;
    use self::WordProperty::*;
    match (prev, next) {
        (Lf, Other) => Start,
        (Other, Lf) => End,
        (Lf, Space) => Interior,
        (Lf, Punctuation) => Interior,
        (Space, Lf) => Interior,
        (Punctuation, Lf) => Interior,
        (Space, Punctuation) => Interior,
        (Punctuation, Space) => Interior,
        _ => classify_boundary(prev, next),
    }
}
#[derive(Copy, Clone)]
enum WordProperty {
    Lf,
    Space,
    Punctuation,
    Other,
}
fn get_word_property(codepoint: char) -> WordProperty {
    if codepoint <= ' ' {
        if codepoint == '\n' {
            return WordProperty::Lf;
        }
        return WordProperty::Space;
    } else if codepoint <= '\u{3f}' {
        if (0xfc00fffe00000000u64 >> (codepoint as u32)) & 1 != 0 {
            return WordProperty::Punctuation;
        }
    } else if codepoint <= '\u{7f}' {
        if (0x7800000178000001u64 >> ((codepoint as u32) & 0x3f)) & 1 != 0 {
            return WordProperty::Punctuation;
        }
    }
    WordProperty::Other
}
#[cfg(test)]
mod tests_llm_16_812 {
    use super::*;
    use crate::*;
    use word_boundaries::WordBoundary;
    #[test]
    fn test_is_boundary() {
        let _rug_st_tests_llm_16_812_rrrruuuugggg_test_is_boundary = 0;
        debug_assert_eq!(WordBoundary::Interior.is_boundary(), false);
        debug_assert_eq!(WordBoundary::Start.is_boundary(), true);
        debug_assert_eq!(WordBoundary::End.is_boundary(), true);
        debug_assert_eq!(WordBoundary::Both.is_boundary(), true);
        let _rug_ed_tests_llm_16_812_rrrruuuugggg_test_is_boundary = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_813 {
    use crate::word_boundaries::WordBoundary;
    #[test]
    fn test_is_end() {
        let _rug_st_tests_llm_16_813_rrrruuuugggg_test_is_end = 0;
        debug_assert_eq!(WordBoundary::End.is_end(), true);
        debug_assert_eq!(WordBoundary::Both.is_end(), true);
        debug_assert_eq!(WordBoundary::Interior.is_end(), false);
        debug_assert_eq!(WordBoundary::Start.is_end(), false);
        let _rug_ed_tests_llm_16_813_rrrruuuugggg_test_is_end = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_814 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_start() {
        let _rug_st_tests_llm_16_814_rrrruuuugggg_test_is_start = 0;
        let interior = WordBoundary::Interior;
        let start = WordBoundary::Start;
        let end = WordBoundary::End;
        let both = WordBoundary::Both;
        debug_assert_eq!(interior.is_start(), false);
        debug_assert_eq!(start.is_start(), true);
        debug_assert_eq!(end.is_start(), false);
        debug_assert_eq!(both.is_start(), true);
        let _rug_ed_tests_llm_16_814_rrrruuuugggg_test_is_start = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_816_llm_16_815 {
    use crate::word_boundaries::{WordCursor, Rope};
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_816_llm_16_815_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = "Hello, world!";
        let rug_fuzz_1 = 7;
        let text = Rope::from(rug_fuzz_0);
        let pos = rug_fuzz_1;
        let word_cursor = WordCursor::new(&text, pos);
        debug_assert_eq!(word_cursor.inner.pos(), pos);
        let _rug_ed_tests_llm_16_816_llm_16_815_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_818_llm_16_817 {
    use xi_rope::Rope;
    use crate::word_boundaries::{
        get_word_property, classify_boundary, classify_boundary_initial, WordBoundary,
        WordCursor,
    };
    #[test]
    fn test_next_boundary() {
        let _rug_st_tests_llm_16_818_llm_16_817_rrrruuuugggg_test_next_boundary = 0;
        let rug_fuzz_0 = "Hello, world!";
        let rug_fuzz_1 = 0;
        let text = Rope::from(rug_fuzz_0);
        let mut cursor = WordCursor::new(&text, rug_fuzz_1);
        debug_assert_eq!(cursor.next_boundary(), Some(0));
        debug_assert_eq!(cursor.next_boundary(), Some(5));
        debug_assert_eq!(cursor.next_boundary(), Some(6));
        debug_assert_eq!(cursor.next_boundary(), Some(12));
        debug_assert_eq!(cursor.next_boundary(), Some(13));
        debug_assert_eq!(cursor.next_boundary(), Some(14));
        debug_assert_eq!(cursor.next_boundary(), Some(15));
        debug_assert_eq!(cursor.next_boundary(), Some(16));
        debug_assert_eq!(cursor.next_boundary(), Some(17));
        debug_assert_eq!(cursor.next_boundary(), None);
        let _rug_ed_tests_llm_16_818_llm_16_817_rrrruuuugggg_test_next_boundary = 0;
    }
}
