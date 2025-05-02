use std::char;
use std::cmp::Ordering;
use std::fmt;
use std::ops;
use std::u32;
use syntax;
use literal::LiteralSearcher;
use prog::InstEmptyLook;
use utf8::{decode_last_utf8, decode_utf8};
/// Represents a location in the input.
#[derive(Clone, Copy, Debug)]
pub struct InputAt {
    pos: usize,
    c: Char,
    byte: Option<u8>,
    len: usize,
}
impl InputAt {
    /// Returns true iff this position is at the beginning of the input.
    pub fn is_start(&self) -> bool {
        self.pos == 0
    }
    /// Returns true iff this position is past the end of the input.
    pub fn is_end(&self) -> bool {
        self.c.is_none() && self.byte.is_none()
    }
    /// Returns the character at this position.
    ///
    /// If this position is just before or after the input, then an absent
    /// character is returned.
    pub fn char(&self) -> Char {
        self.c
    }
    /// Returns the byte at this position.
    pub fn byte(&self) -> Option<u8> {
        self.byte
    }
    /// Returns the UTF-8 width of the character at this position.
    pub fn len(&self) -> usize {
        self.len
    }
    /// Returns whether the UTF-8 width of the character at this position
    /// is zero.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    /// Returns the byte offset of this position.
    pub fn pos(&self) -> usize {
        self.pos
    }
    /// Returns the byte offset of the next position in the input.
    pub fn next_pos(&self) -> usize {
        self.pos + self.len
    }
}
/// An abstraction over input used in the matching engines.
pub trait Input: fmt::Debug {
    /// Return an encoding of the position at byte offset `i`.
    fn at(&self, i: usize) -> InputAt;
    /// Return the Unicode character occurring next to `at`.
    ///
    /// If no such character could be decoded, then `Char` is absent.
    fn next_char(&self, at: InputAt) -> Char;
    /// Return the Unicode character occurring previous to `at`.
    ///
    /// If no such character could be decoded, then `Char` is absent.
    fn previous_char(&self, at: InputAt) -> Char;
    /// Return true if the given empty width instruction matches at the
    /// input position given.
    fn is_empty_match(&self, at: InputAt, empty: &InstEmptyLook) -> bool;
    /// Scan the input for a matching prefix.
    fn prefix_at(&self, prefixes: &LiteralSearcher, at: InputAt) -> Option<InputAt>;
    /// The number of bytes in the input.
    fn len(&self) -> usize;
    /// Whether the input is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Return the given input as a sequence of bytes.
    fn as_bytes(&self) -> &[u8];
}
impl<'a, T: Input> Input for &'a T {
    fn at(&self, i: usize) -> InputAt {
        (**self).at(i)
    }
    fn next_char(&self, at: InputAt) -> Char {
        (**self).next_char(at)
    }
    fn previous_char(&self, at: InputAt) -> Char {
        (**self).previous_char(at)
    }
    fn is_empty_match(&self, at: InputAt, empty: &InstEmptyLook) -> bool {
        (**self).is_empty_match(at, empty)
    }
    fn prefix_at(&self, prefixes: &LiteralSearcher, at: InputAt) -> Option<InputAt> {
        (**self).prefix_at(prefixes, at)
    }
    fn len(&self) -> usize {
        (**self).len()
    }
    fn as_bytes(&self) -> &[u8] {
        (**self).as_bytes()
    }
}
/// An input reader over characters.
#[derive(Clone, Copy, Debug)]
pub struct CharInput<'t>(&'t [u8]);
impl<'t> CharInput<'t> {
    /// Return a new character input reader for the given string.
    pub fn new(s: &'t [u8]) -> CharInput<'t> {
        CharInput(s)
    }
}
impl<'t> ops::Deref for CharInput<'t> {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        self.0
    }
}
impl<'t> Input for CharInput<'t> {
    fn at(&self, i: usize) -> InputAt {
        if i >= self.len() {
            InputAt {
                pos: self.len(),
                c: None.into(),
                byte: None,
                len: 0,
            }
        } else {
            let c = decode_utf8(&self[i..]).map(|(c, _)| c).into();
            InputAt {
                pos: i,
                c: c,
                byte: None,
                len: c.len_utf8(),
            }
        }
    }
    fn next_char(&self, at: InputAt) -> Char {
        at.char()
    }
    fn previous_char(&self, at: InputAt) -> Char {
        decode_last_utf8(&self[..at.pos()]).map(|(c, _)| c).into()
    }
    fn is_empty_match(&self, at: InputAt, empty: &InstEmptyLook) -> bool {
        use prog::EmptyLook::*;
        match empty.look {
            StartLine => {
                let c = self.previous_char(at);
                at.pos() == 0 || c == '\n'
            }
            EndLine => {
                let c = self.next_char(at);
                at.pos() == self.len() || c == '\n'
            }
            StartText => at.pos() == 0,
            EndText => at.pos() == self.len(),
            WordBoundary => {
                let (c1, c2) = (self.previous_char(at), self.next_char(at));
                c1.is_word_char() != c2.is_word_char()
            }
            NotWordBoundary => {
                let (c1, c2) = (self.previous_char(at), self.next_char(at));
                c1.is_word_char() == c2.is_word_char()
            }
            WordBoundaryAscii => {
                let (c1, c2) = (self.previous_char(at), self.next_char(at));
                c1.is_word_byte() != c2.is_word_byte()
            }
            NotWordBoundaryAscii => {
                let (c1, c2) = (self.previous_char(at), self.next_char(at));
                c1.is_word_byte() == c2.is_word_byte()
            }
        }
    }
    fn prefix_at(&self, prefixes: &LiteralSearcher, at: InputAt) -> Option<InputAt> {
        prefixes.find(&self[at.pos()..]).map(|(s, _)| self.at(at.pos() + s))
    }
    fn len(&self) -> usize {
        self.0.len()
    }
    fn as_bytes(&self) -> &[u8] {
        self.0
    }
}
/// An input reader over bytes.
#[derive(Clone, Copy, Debug)]
pub struct ByteInput<'t> {
    text: &'t [u8],
    only_utf8: bool,
}
impl<'t> ByteInput<'t> {
    /// Return a new byte-based input reader for the given string.
    pub fn new(text: &'t [u8], only_utf8: bool) -> ByteInput<'t> {
        ByteInput {
            text: text,
            only_utf8: only_utf8,
        }
    }
}
impl<'t> ops::Deref for ByteInput<'t> {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        self.text
    }
}
impl<'t> Input for ByteInput<'t> {
    fn at(&self, i: usize) -> InputAt {
        if i >= self.len() {
            InputAt {
                pos: self.len(),
                c: None.into(),
                byte: None,
                len: 0,
            }
        } else {
            InputAt {
                pos: i,
                c: None.into(),
                byte: self.get(i).cloned(),
                len: 1,
            }
        }
    }
    fn next_char(&self, at: InputAt) -> Char {
        decode_utf8(&self[at.pos()..]).map(|(c, _)| c).into()
    }
    fn previous_char(&self, at: InputAt) -> Char {
        decode_last_utf8(&self[..at.pos()]).map(|(c, _)| c).into()
    }
    fn is_empty_match(&self, at: InputAt, empty: &InstEmptyLook) -> bool {
        use prog::EmptyLook::*;
        match empty.look {
            StartLine => {
                let c = self.previous_char(at);
                at.pos() == 0 || c == '\n'
            }
            EndLine => {
                let c = self.next_char(at);
                at.pos() == self.len() || c == '\n'
            }
            StartText => at.pos() == 0,
            EndText => at.pos() == self.len(),
            WordBoundary => {
                let (c1, c2) = (self.previous_char(at), self.next_char(at));
                c1.is_word_char() != c2.is_word_char()
            }
            NotWordBoundary => {
                let (c1, c2) = (self.previous_char(at), self.next_char(at));
                c1.is_word_char() == c2.is_word_char()
            }
            WordBoundaryAscii => {
                let (c1, c2) = (self.previous_char(at), self.next_char(at));
                if self.only_utf8 {
                    if c1.is_none() && !at.is_start() {
                        return false;
                    }
                    if c2.is_none() && !at.is_end() {
                        return false;
                    }
                }
                c1.is_word_byte() != c2.is_word_byte()
            }
            NotWordBoundaryAscii => {
                let (c1, c2) = (self.previous_char(at), self.next_char(at));
                if self.only_utf8 {
                    if c1.is_none() && !at.is_start() {
                        return false;
                    }
                    if c2.is_none() && !at.is_end() {
                        return false;
                    }
                }
                c1.is_word_byte() == c2.is_word_byte()
            }
        }
    }
    fn prefix_at(&self, prefixes: &LiteralSearcher, at: InputAt) -> Option<InputAt> {
        prefixes.find(&self[at.pos()..]).map(|(s, _)| self.at(at.pos() + s))
    }
    fn len(&self) -> usize {
        self.text.len()
    }
    fn as_bytes(&self) -> &[u8] {
        self.text
    }
}
/// An inline representation of `Option<char>`.
///
/// This eliminates the need to do case analysis on `Option<char>` to determine
/// ordinality with other characters.
///
/// (The `Option<char>` is not related to encoding. Instead, it is used in the
/// matching engines to represent the beginning and ending boundaries of the
/// search text.)
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Char(u32);
impl fmt::Debug for Char {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match char::from_u32(self.0) {
            None => write!(f, "Empty"),
            Some(c) => write!(f, "{:?}", c),
        }
    }
}
impl Char {
    /// Returns true iff the character is absent.
    #[inline]
    pub fn is_none(self) -> bool {
        self.0 == u32::MAX
    }
    /// Returns the length of the character's UTF-8 encoding.
    ///
    /// If the character is absent, then `1` is returned.
    #[inline]
    pub fn len_utf8(self) -> usize {
        char::from_u32(self.0).map_or(1, |c| c.len_utf8())
    }
    /// Returns true iff the character is a word character.
    ///
    /// If the character is absent, then false is returned.
    pub fn is_word_char(self) -> bool {
        char::from_u32(self.0).map_or(false, syntax::is_word_character)
    }
    /// Returns true iff the byte is a word byte.
    ///
    /// If the byte is absent, then false is returned.
    pub fn is_word_byte(self) -> bool {
        match char::from_u32(self.0) {
            Some(c) if c <= '\u{7F}' => syntax::is_word_byte(c as u8),
            None | Some(_) => false,
        }
    }
}
impl From<char> for Char {
    fn from(c: char) -> Char {
        Char(c as u32)
    }
}
impl From<Option<char>> for Char {
    fn from(c: Option<char>) -> Char {
        c.map_or(Char(u32::MAX), |c| c.into())
    }
}
impl PartialEq<char> for Char {
    #[inline]
    fn eq(&self, other: &char) -> bool {
        self.0 == *other as u32
    }
}
impl PartialEq<Char> for char {
    #[inline]
    fn eq(&self, other: &Char) -> bool {
        *self as u32 == other.0
    }
}
impl PartialOrd<char> for Char {
    #[inline]
    fn partial_cmp(&self, other: &char) -> Option<Ordering> {
        self.0.partial_cmp(&(*other as u32))
    }
}
impl PartialOrd<Char> for char {
    #[inline]
    fn partial_cmp(&self, other: &Char) -> Option<Ordering> {
        (*self as u32).partial_cmp(&other.0)
    }
}
#[cfg(test)]
mod tests_llm_16_1 {
    use super::*;
    use crate::*;
    #[test]
    fn test_as_bytes() {
        let _rug_st_tests_llm_16_1_rrrruuuugggg_test_as_bytes = 0;
        let _rug_ed_tests_llm_16_1_rrrruuuugggg_test_as_bytes = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_61 {
    use super::*;
    use crate::*;
    use crate::input::{Input, ByteInput};
    #[test]
    fn test_as_bytes() {
        let _rug_st_tests_llm_16_61_rrrruuuugggg_test_as_bytes = 0;
        let rug_fuzz_0 = "Hello, World!";
        let rug_fuzz_1 = true;
        let text = rug_fuzz_0.as_bytes();
        let input = ByteInput::new(text, rug_fuzz_1);
        debug_assert_eq!(input.as_bytes(), text);
        let _rug_ed_tests_llm_16_61_rrrruuuugggg_test_as_bytes = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_62 {
    use crate::input::{Char, Input, InputAt, ByteInput};
    #[test]
    fn test_at() {
        let _rug_st_tests_llm_16_62_rrrruuuugggg_test_at = 0;
        let rug_fuzz_0 = 97;
        let rug_fuzz_1 = 98;
        let rug_fuzz_2 = 99;
        let rug_fuzz_3 = 100;
        let rug_fuzz_4 = 101;
        let rug_fuzz_5 = false;
        let rug_fuzz_6 = 0;
        let rug_fuzz_7 = 1;
        let rug_fuzz_8 = 5;
        let input_text: &[u8] = &[
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
        ];
        let input = ByteInput::new(input_text, rug_fuzz_5);
        let at = input.at(rug_fuzz_6);
        debug_assert_eq!(at.is_start(), true);
        debug_assert_eq!(at.is_end(), false);
        debug_assert_eq!(at.char(), Char::from(Some('a')));
        debug_assert_eq!(at.byte(), Some(97));
        debug_assert_eq!(at.len(), 1);
        debug_assert_eq!(at.is_empty(), false);
        debug_assert_eq!(at.pos(), 0);
        debug_assert_eq!(at.next_pos(), 1);
        let at = input.at(rug_fuzz_7);
        debug_assert_eq!(at.is_start(), false);
        debug_assert_eq!(at.is_end(), false);
        debug_assert_eq!(at.char(), Char::from(Some('b')));
        debug_assert_eq!(at.byte(), Some(98));
        debug_assert_eq!(at.len(), 1);
        debug_assert_eq!(at.is_empty(), false);
        debug_assert_eq!(at.pos(), 1);
        debug_assert_eq!(at.next_pos(), 2);
        let at = input.at(rug_fuzz_8);
        debug_assert_eq!(at.is_start(), false);
        debug_assert_eq!(at.is_end(), true);
        debug_assert_eq!(at.char(), Char::from(None));
        debug_assert_eq!(at.byte(), None);
        debug_assert_eq!(at.len(), 0);
        debug_assert_eq!(at.is_empty(), true);
        debug_assert_eq!(at.pos(), 5);
        debug_assert_eq!(at.next_pos(), 5);
        let _rug_ed_tests_llm_16_62_rrrruuuugggg_test_at = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_65 {
    use super::*;
    use crate::*;
    use crate::input::{ByteInput, Input};
    #[test]
    fn test_len() {
        let _rug_st_tests_llm_16_65_rrrruuuugggg_test_len = 0;
        let rug_fuzz_0 = b"example";
        let rug_fuzz_1 = false;
        let input = ByteInput::new(rug_fuzz_0, rug_fuzz_1);
        debug_assert_eq!(input.len(), 7);
        let _rug_ed_tests_llm_16_65_rrrruuuugggg_test_len = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_66 {
    use super::*;
    use crate::*;
    use crate::input::{Input, ByteInput, Char, InputAt};
    #[test]
    fn test_next_char() {
        let _rug_st_tests_llm_16_66_rrrruuuugggg_test_next_char = 0;
        let rug_fuzz_0 = "Hello, world!";
        let rug_fuzz_1 = true;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let text = rug_fuzz_0;
        let input = ByteInput::new(text.as_bytes(), rug_fuzz_1);
        let at = InputAt {
            pos: rug_fuzz_2,
            c: Char::from(None),
            byte: None,
            len: rug_fuzz_3,
        };
        let result = input.next_char(at);
        debug_assert_eq!(result, Char::from('H'));
        let _rug_ed_tests_llm_16_66_rrrruuuugggg_test_next_char = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_70_llm_16_69 {
    use crate::input::{ByteInput, Char, Input, InputAt};
    #[test]
    fn test_previous_char() {
        let _rug_st_tests_llm_16_70_llm_16_69_rrrruuuugggg_test_previous_char = 0;
        let rug_fuzz_0 = 104;
        let rug_fuzz_1 = 101;
        let rug_fuzz_2 = 108;
        let rug_fuzz_3 = 108;
        let rug_fuzz_4 = 111;
        let rug_fuzz_5 = true;
        let rug_fuzz_6 = 4;
        let text: &[u8] = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3, rug_fuzz_4];
        let input = ByteInput::new(text, rug_fuzz_5);
        let at = input.at(rug_fuzz_6);
        let result = input.previous_char(at);
        debug_assert_eq!(result, Char::from('o'));
        let _rug_ed_tests_llm_16_70_llm_16_69_rrrruuuugggg_test_previous_char = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_71 {
    use crate::input::{ByteInput, Input};
    use std::ops::Deref;
    #[test]
    fn test_deref() {
        let _rug_st_tests_llm_16_71_rrrruuuugggg_test_deref = 0;
        let rug_fuzz_0 = b"Hello, World!";
        let rug_fuzz_1 = false;
        let input = ByteInput::new(rug_fuzz_0, rug_fuzz_1);
        let dereferenced = input.deref();
        debug_assert_eq!(dereferenced, b"Hello, World!");
        let _rug_ed_tests_llm_16_71_rrrruuuugggg_test_deref = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_72 {
    use super::*;
    use crate::*;
    use std::cmp::Ordering;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_72_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = 'a';
        let rug_fuzz_1 = 'b';
        let rug_fuzz_2 = 'a';
        let rug_fuzz_3 = 'a';
        let rug_fuzz_4 = 'b';
        let rug_fuzz_5 = 'c';
        let rug_fuzz_6 = 'b';
        let char1 = Char::from(Some(rug_fuzz_0));
        let char2 = Char::from(Some(rug_fuzz_1));
        let char3 = Char::from(None);
        let char4 = Char::from(Some(rug_fuzz_2));
        debug_assert_eq!(char1.eq(& rug_fuzz_3), true);
        debug_assert_eq!(char2.eq(& rug_fuzz_4), true);
        debug_assert_eq!(char3.eq(& rug_fuzz_5), false);
        debug_assert_eq!(char4.eq(& rug_fuzz_6), false);
        let _rug_ed_tests_llm_16_72_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_73 {
    use std::cmp::Ordering;
    use crate::input::Char;
    #[test]
    fn test_partial_cmp() {
        let _rug_st_tests_llm_16_73_rrrruuuugggg_test_partial_cmp = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 'a';
        let rug_fuzz_4 = 'b';
        let rug_fuzz_5 = 'a';
        let rug_fuzz_6 = 'c';
        let rug_fuzz_7 = 'b';
        let rug_fuzz_8 = 'b';
        let char1 = Char(rug_fuzz_0);
        let char2 = Char(rug_fuzz_1);
        let char3 = Char(rug_fuzz_2);
        debug_assert_eq!(char1.partial_cmp(& rug_fuzz_3), Some(Ordering::Less));
        debug_assert_eq!(char2.partial_cmp(& rug_fuzz_4), Some(Ordering::Less));
        debug_assert_eq!(char2.partial_cmp(& rug_fuzz_5), Some(Ordering::Equal));
        debug_assert_eq!(char2.partial_cmp(& rug_fuzz_6), Some(Ordering::Greater));
        debug_assert_eq!(char2.partial_cmp(& rug_fuzz_7), Some(Ordering::Equal));
        debug_assert_eq!(char3.partial_cmp(& rug_fuzz_8), Some(Ordering::Equal));
        let _rug_ed_tests_llm_16_73_rrrruuuugggg_test_partial_cmp = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_78 {
    use super::*;
    use crate::*;
    #[test]
    fn test_as_bytes() {
        let _rug_st_tests_llm_16_78_rrrruuuugggg_test_as_bytes = 0;
        let rug_fuzz_0 = b"abcdef";
        let rug_fuzz_1 = b"abcdef";
        let input = CharInput::new(rug_fuzz_0);
        let expected = rug_fuzz_1;
        let result = input.as_bytes();
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_78_rrrruuuugggg_test_as_bytes = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_79 {
    use crate::input::{Char, CharInput, Input, InputAt};
    #[test]
    fn test_at() {
        let _rug_st_tests_llm_16_79_rrrruuuugggg_test_at = 0;
        let rug_fuzz_0 = b"abcdef";
        let rug_fuzz_1 = 2;
        let input = CharInput::new(rug_fuzz_0);
        let at = input.at(rug_fuzz_1);
        debug_assert_eq!(at.char(), Char::from('c'));
        debug_assert_eq!(at.byte(), Some(b'c'));
        debug_assert_eq!(at.len(), 1);
        debug_assert_eq!(at.is_empty(), false);
        debug_assert_eq!(at.pos(), 2);
        debug_assert_eq!(at.next_pos(), 3);
        let _rug_ed_tests_llm_16_79_rrrruuuugggg_test_at = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_80 {
    use super::*;
    use crate::*;
    use prog::{EmptyLook, InstEmptyLook};
    use input::{Char, CharInput, Input, InputAt};
    #[test]
    fn test_is_empty_match() {
        let _rug_st_tests_llm_16_80_rrrruuuugggg_test_is_empty_match = 0;
        let rug_fuzz_0 = b"test";
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 't';
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = 0;
        let input = CharInput::new(rug_fuzz_0);
        let at = InputAt {
            pos: rug_fuzz_1,
            c: Char::from(rug_fuzz_2),
            byte: None,
            len: rug_fuzz_3,
        };
        let empty = InstEmptyLook {
            goto: rug_fuzz_4,
            look: EmptyLook::StartLine,
        };
        let result = input.is_empty_match(at, &empty);
        debug_assert_eq!(result, false);
        let _rug_ed_tests_llm_16_80_rrrruuuugggg_test_is_empty_match = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_81 {
    use super::*;
    use crate::*;
    use crate::input::{CharInput, Input};
    #[test]
    fn len_returns_correct_length() {
        let _rug_st_tests_llm_16_81_rrrruuuugggg_len_returns_correct_length = 0;
        let rug_fuzz_0 = b"Some input data";
        let input = CharInput::new(rug_fuzz_0);
        debug_assert_eq!(input.len(), 16);
        let _rug_ed_tests_llm_16_81_rrrruuuugggg_len_returns_correct_length = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_82 {
    use super::*;
    use crate::*;
    use crate::input::Input;
    #[test]
    fn test_next_char() {
        let _rug_st_tests_llm_16_82_rrrruuuugggg_test_next_char = 0;
        let rug_fuzz_0 = b"Hello, World!";
        let rug_fuzz_1 = 0;
        let input = CharInput::new(rug_fuzz_0);
        let at = input.at(rug_fuzz_1);
        let result = input.next_char(at);
        debug_assert_eq!(result, Char(b'H' as u32));
        let _rug_ed_tests_llm_16_82_rrrruuuugggg_test_next_char = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_86 {
    use crate::input::{Char, CharInput, Input, InputAt};
    #[test]
    fn test_previous_char() {
        let _rug_st_tests_llm_16_86_rrrruuuugggg_test_previous_char = 0;
        let rug_fuzz_0 = b"hello";
        let rug_fuzz_1 = 4;
        let rug_fuzz_2 = 'e';
        let input = CharInput::new(rug_fuzz_0);
        let at = input.at(rug_fuzz_1);
        let expected = Char::from(Some(rug_fuzz_2));
        let result = input.previous_char(at);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_86_rrrruuuugggg_test_previous_char = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_87 {
    use crate::input::CharInput;
    use std::ops::Deref;
    #[test]
    fn test_deref() {
        let _rug_st_tests_llm_16_87_rrrruuuugggg_test_deref = 0;
        let rug_fuzz_0 = b"testing";
        let input = CharInput::new(rug_fuzz_0);
        let result = <CharInput as Deref>::deref(&input);
        debug_assert_eq!(result, b"testing");
        let _rug_ed_tests_llm_16_87_rrrruuuugggg_test_deref = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_385 {
    use super::*;
    use crate::*;
    use std::cmp::Ordering;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_385_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = 65;
        let rug_fuzz_1 = 65;
        let rug_fuzz_2 = 66;
        let char_1 = Char(rug_fuzz_0);
        let char_2 = Char(rug_fuzz_1);
        let char_3 = Char(rug_fuzz_2);
        let char_4 = Char(u32::MAX);
        debug_assert_eq!(char_1.eq(& char_2), true);
        debug_assert_eq!(char_1.eq(& char_3), false);
        debug_assert_eq!(char_1.eq(& char_4), false);
        debug_assert_eq!(char_2.eq(& char_1), true);
        debug_assert_eq!(char_2.eq(& char_3), false);
        debug_assert_eq!(char_2.eq(& char_4), false);
        debug_assert_eq!(char_3.eq(& char_1), false);
        debug_assert_eq!(char_3.eq(& char_2), false);
        debug_assert_eq!(char_3.eq(& char_4), false);
        debug_assert_eq!(char_4.eq(& char_1), false);
        debug_assert_eq!(char_4.eq(& char_2), false);
        debug_assert_eq!(char_4.eq(& char_3), false);
        let _rug_ed_tests_llm_16_385_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_386 {
    use crate::input::Char;
    use std::cmp::Ordering;
    #[test]
    fn test_partial_cmp() {
        let _rug_st_tests_llm_16_386_rrrruuuugggg_test_partial_cmp = 0;
        let rug_fuzz_0 = 'a';
        let rug_fuzz_1 = 'b';
        let rug_fuzz_2 = 'a';
        let rug_fuzz_3 = 'c';
        let char1 = Char::from(rug_fuzz_0);
        let char2 = Char::from(rug_fuzz_1);
        let char3 = Char::from(rug_fuzz_2);
        let char4 = Char::from(rug_fuzz_3);
        debug_assert_eq!(char1.partial_cmp(& char2), Some(Ordering::Less));
        debug_assert_eq!(char2.partial_cmp(& char1), Some(Ordering::Greater));
        debug_assert_eq!(char1.partial_cmp(& char3), Some(Ordering::Equal));
        debug_assert_eq!(char3.partial_cmp(& char4), Some(Ordering::Less));
        let _rug_ed_tests_llm_16_386_rrrruuuugggg_test_partial_cmp = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_387 {
    use super::*;
    use crate::*;
    use crate::input::{Input, Char, InputAt, InstEmptyLook, LiteralSearcher};
    use std::ops;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_387_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = 97;
        let rug_fuzz_1 = 98;
        let rug_fuzz_2 = 99;
        let rug_fuzz_3 = false;
        let text: &[u8] = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        let only_utf8: bool = rug_fuzz_3;
        let result = ByteInput::new(text, only_utf8);
        debug_assert_eq!(result.text, & [97, 98, 99]);
        debug_assert_eq!(result.only_utf8, false);
        let _rug_ed_tests_llm_16_387_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_388 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_none() {
        let _rug_st_tests_llm_16_388_rrrruuuugggg_test_is_none = 0;
        let rug_fuzz_0 = 10;
        let char_with_max_u32 = Char(u32::MAX);
        debug_assert!(char_with_max_u32.is_none());
        let char_with_non_max_u32 = Char(rug_fuzz_0);
        debug_assert!(! char_with_non_max_u32.is_none());
        let _rug_ed_tests_llm_16_388_rrrruuuugggg_test_is_none = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_391 {
    use crate::input::Char;
    #[test]
    fn test_is_word_char_with_word_character() {
        let _rug_st_tests_llm_16_391_rrrruuuugggg_test_is_word_char_with_word_character = 0;
        let rug_fuzz_0 = 'a';
        let c = Char::from(rug_fuzz_0);
        debug_assert_eq!(c.is_word_char(), true);
        let _rug_ed_tests_llm_16_391_rrrruuuugggg_test_is_word_char_with_word_character = 0;
    }
    #[test]
    fn test_is_word_char_with_non_word_character() {
        let _rug_st_tests_llm_16_391_rrrruuuugggg_test_is_word_char_with_non_word_character = 0;
        let rug_fuzz_0 = ' ';
        let c = Char::from(rug_fuzz_0);
        debug_assert_eq!(c.is_word_char(), false);
        let _rug_ed_tests_llm_16_391_rrrruuuugggg_test_is_word_char_with_non_word_character = 0;
    }
    #[test]
    fn test_is_word_char_with_absent_character() {
        let _rug_st_tests_llm_16_391_rrrruuuugggg_test_is_word_char_with_absent_character = 0;
        let c = Char::from(None);
        debug_assert_eq!(c.is_word_char(), false);
        let _rug_ed_tests_llm_16_391_rrrruuuugggg_test_is_word_char_with_absent_character = 0;
    }
    #[test]
    fn test_is_word_byte_with_word_byte() {
        let _rug_st_tests_llm_16_391_rrrruuuugggg_test_is_word_byte_with_word_byte = 0;
        let rug_fuzz_0 = '\u{007}';
        let c = Char::from(rug_fuzz_0);
        debug_assert_eq!(c.is_word_byte(), true);
        let _rug_ed_tests_llm_16_391_rrrruuuugggg_test_is_word_byte_with_word_byte = 0;
    }
    #[test]
    fn test_is_word_byte_with_non_word_byte() {
        let _rug_st_tests_llm_16_391_rrrruuuugggg_test_is_word_byte_with_non_word_byte = 0;
        let rug_fuzz_0 = '\u{300}';
        let c = Char::from(rug_fuzz_0);
        debug_assert_eq!(c.is_word_byte(), false);
        let _rug_ed_tests_llm_16_391_rrrruuuugggg_test_is_word_byte_with_non_word_byte = 0;
    }
    #[test]
    fn test_is_word_byte_with_absent_byte() {
        let _rug_st_tests_llm_16_391_rrrruuuugggg_test_is_word_byte_with_absent_byte = 0;
        let c = Char::from(None);
        debug_assert_eq!(c.is_word_byte(), false);
        let _rug_ed_tests_llm_16_391_rrrruuuugggg_test_is_word_byte_with_absent_byte = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_393 {
    use super::*;
    use crate::*;
    #[test]
    fn test_len_utf8_returns_length_of_utf8_encoding() {
        let _rug_st_tests_llm_16_393_rrrruuuugggg_test_len_utf8_returns_length_of_utf8_encoding = 0;
        let rug_fuzz_0 = 'A';
        let rug_fuzz_1 = 'Ω';
        let rug_fuzz_2 = '😀';
        let rug_fuzz_3 = '\u{1F600}';
        let char1 = Char::from(rug_fuzz_0);
        debug_assert_eq!(char1.len_utf8(), 1);
        let char2 = Char::from(rug_fuzz_1);
        debug_assert_eq!(char2.len_utf8(), 2);
        let char3 = Char::from(rug_fuzz_2);
        debug_assert_eq!(char3.len_utf8(), 4);
        let char4 = Char::from(rug_fuzz_3);
        debug_assert_eq!(char4.len_utf8(), 4);
        let _rug_ed_tests_llm_16_393_rrrruuuugggg_test_len_utf8_returns_length_of_utf8_encoding = 0;
    }
    #[test]
    fn test_len_utf8_returns_1_when_character_is_absent() {
        let _rug_st_tests_llm_16_393_rrrruuuugggg_test_len_utf8_returns_1_when_character_is_absent = 0;
        let rug_fuzz_0 = 1;
        let char1 = Char(u32::MAX);
        debug_assert_eq!(char1.len_utf8(), 1);
        let char2 = Char(u32::MAX - rug_fuzz_0);
        debug_assert_eq!(char2.len_utf8(), 1);
        let _rug_ed_tests_llm_16_393_rrrruuuugggg_test_len_utf8_returns_1_when_character_is_absent = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_398 {
    use super::*;
    use crate::*;
    #[test]
    fn test_byte_returns_some_byte() {
        let _rug_st_tests_llm_16_398_rrrruuuugggg_test_byte_returns_some_byte = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 'a';
        let rug_fuzz_2 = b'a';
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = b'1';
        let rug_fuzz_5 = b'!';
        let mut input_at = InputAt {
            pos: rug_fuzz_0,
            c: Char::from(Some(rug_fuzz_1)),
            byte: Some(rug_fuzz_2),
            len: rug_fuzz_3,
        };
        debug_assert_eq!(input_at.byte(), Some(b'a'));
        input_at.byte = Some(rug_fuzz_4);
        debug_assert_eq!(input_at.byte(), Some(b'1'));
        input_at.byte = Some(rug_fuzz_5);
        debug_assert_eq!(input_at.byte(), Some(b'!'));
        let _rug_ed_tests_llm_16_398_rrrruuuugggg_test_byte_returns_some_byte = 0;
    }
    #[test]
    fn test_byte_returns_none() {
        let _rug_st_tests_llm_16_398_rrrruuuugggg_test_byte_returns_none = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 'a';
        let rug_fuzz_2 = 1;
        let input_at = InputAt {
            pos: rug_fuzz_0,
            c: Char::from(Some(rug_fuzz_1)),
            byte: None,
            len: rug_fuzz_2,
        };
        debug_assert_eq!(input_at.byte(), None);
        let _rug_ed_tests_llm_16_398_rrrruuuugggg_test_byte_returns_none = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_399 {
    use super::*;
    use crate::*;
    use std::cmp::Ordering;
    #[test]
    fn test_char() {
        let _rug_st_tests_llm_16_399_rrrruuuugggg_test_char = 0;
        let rug_fuzz_0 = 65;
        let c = Char(rug_fuzz_0);
        debug_assert_eq!(c.is_none(), false);
        debug_assert_eq!(c.len_utf8(), 1);
        debug_assert_eq!(c.is_word_char(), false);
        debug_assert_eq!(c.is_word_byte(), false);
        let c = Char(u32::MAX);
        debug_assert_eq!(c.is_none(), true);
        debug_assert_eq!(c.len_utf8(), 1);
        debug_assert_eq!(c.is_word_char(), false);
        debug_assert_eq!(c.is_word_byte(), false);
        let _rug_ed_tests_llm_16_399_rrrruuuugggg_test_char = 0;
    }
    #[test]
    fn test_input_at() {
        let _rug_st_tests_llm_16_399_rrrruuuugggg_test_input_at = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 65;
        let rug_fuzz_2 = 65;
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = 5;
        let rug_fuzz_5 = 0;
        let input = InputAt {
            pos: rug_fuzz_0,
            c: Char(rug_fuzz_1),
            byte: Some(rug_fuzz_2),
            len: rug_fuzz_3,
        };
        debug_assert_eq!(input.is_start(), true);
        debug_assert_eq!(input.is_end(), false);
        debug_assert_eq!(input.char(), Char(65));
        debug_assert_eq!(input.byte(), Some(65));
        debug_assert_eq!(input.len(), 1);
        debug_assert_eq!(input.is_empty(), false);
        debug_assert_eq!(input.pos(), 0);
        debug_assert_eq!(input.next_pos(), 1);
        let input = InputAt {
            pos: rug_fuzz_4,
            c: Char(u32::MAX),
            byte: None,
            len: rug_fuzz_5,
        };
        debug_assert_eq!(input.is_start(), false);
        debug_assert_eq!(input.is_end(), true);
        debug_assert_eq!(input.char(), Char(u32::MAX));
        debug_assert_eq!(input.byte(), None);
        debug_assert_eq!(input.len(), 0);
        debug_assert_eq!(input.is_empty(), true);
        debug_assert_eq!(input.pos(), 5);
        debug_assert_eq!(input.next_pos(), 5);
        let _rug_ed_tests_llm_16_399_rrrruuuugggg_test_input_at = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_400 {
    use crate::input::Char;
    use crate::input::InputAt;
    #[test]
    fn test_is_empty() {
        let _rug_st_tests_llm_16_400_rrrruuuugggg_test_is_empty = 0;
        let rug_fuzz_0 = 'a';
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = b'a';
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = 1;
        let rug_fuzz_5 = 0;
        let c = Char::from(rug_fuzz_0);
        let input = InputAt {
            pos: rug_fuzz_1,
            c,
            byte: Some(rug_fuzz_2),
            len: rug_fuzz_3,
        };
        debug_assert_eq!(input.is_empty(), false);
        let c = Char::from(None::<char>);
        let input = InputAt {
            pos: rug_fuzz_4,
            c,
            byte: None,
            len: rug_fuzz_5,
        };
        debug_assert_eq!(input.is_empty(), true);
        let _rug_ed_tests_llm_16_400_rrrruuuugggg_test_is_empty = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_401 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_end_none() {
        let _rug_st_tests_llm_16_401_rrrruuuugggg_test_is_end_none = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let input = InputAt {
            pos: rug_fuzz_0,
            c: Char(u32::MAX),
            byte: None,
            len: rug_fuzz_1,
        };
        debug_assert_eq!(input.is_end(), true);
        let _rug_ed_tests_llm_16_401_rrrruuuugggg_test_is_end_none = 0;
    }
    #[test]
    fn test_is_end_some() {
        let _rug_st_tests_llm_16_401_rrrruuuugggg_test_is_end_some = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 97;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 1;
        let input = InputAt {
            pos: rug_fuzz_0,
            c: Char(rug_fuzz_1),
            byte: Some(rug_fuzz_2),
            len: rug_fuzz_3,
        };
        debug_assert_eq!(input.is_end(), false);
        let _rug_ed_tests_llm_16_401_rrrruuuugggg_test_is_end_some = 0;
    }
    #[test]
    fn test_is_end_none_byte() {
        let _rug_st_tests_llm_16_401_rrrruuuugggg_test_is_end_none_byte = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 1;
        let input = InputAt {
            pos: rug_fuzz_0,
            c: Char(u32::MAX),
            byte: Some(rug_fuzz_1),
            len: rug_fuzz_2,
        };
        debug_assert_eq!(input.is_end(), false);
        let _rug_ed_tests_llm_16_401_rrrruuuugggg_test_is_end_none_byte = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_402 {
    use super::*;
    use crate::*;
    use crate::input::{Char, InputAt};
    #[test]
    fn test_is_start() {
        let _rug_st_tests_llm_16_402_rrrruuuugggg_test_is_start = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 'a';
        let rug_fuzz_6 = b'a';
        let rug_fuzz_7 = 1;
        let rug_fuzz_8 = 1;
        let rug_fuzz_9 = 'a';
        let rug_fuzz_10 = b'a';
        let rug_fuzz_11 = 1;
        let input = InputAt {
            pos: rug_fuzz_0,
            c: Char::from(None),
            byte: None,
            len: rug_fuzz_1,
        };
        debug_assert_eq!(input.is_start(), true);
        let input = InputAt {
            pos: rug_fuzz_2,
            c: Char::from(None),
            byte: None,
            len: rug_fuzz_3,
        };
        debug_assert_eq!(input.is_start(), false);
        let input = InputAt {
            pos: rug_fuzz_4,
            c: Char::from(rug_fuzz_5),
            byte: Some(rug_fuzz_6),
            len: rug_fuzz_7,
        };
        debug_assert_eq!(input.is_start(), true);
        let input = InputAt {
            pos: rug_fuzz_8,
            c: Char::from(rug_fuzz_9),
            byte: Some(rug_fuzz_10),
            len: rug_fuzz_11,
        };
        debug_assert_eq!(input.is_start(), false);
        let _rug_ed_tests_llm_16_402_rrrruuuugggg_test_is_start = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_403 {
    use super::*;
    use crate::*;
    use crate::input::{Char, InputAt};
    #[test]
    fn test_len() {
        let _rug_st_tests_llm_16_403_rrrruuuugggg_test_len = 0;
        let rug_fuzz_0 = 'a';
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = b'a';
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let char = Char::from(rug_fuzz_0);
        let input_at = InputAt {
            pos: rug_fuzz_1,
            c: char,
            byte: Some(rug_fuzz_2),
            len: rug_fuzz_3,
        };
        debug_assert_eq!(input_at.len(), 1);
        let char = Char::from(None);
        let input_at = InputAt {
            pos: rug_fuzz_4,
            c: char,
            byte: None,
            len: rug_fuzz_5,
        };
        debug_assert_eq!(input_at.len(), 0);
        let _rug_ed_tests_llm_16_403_rrrruuuugggg_test_len = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_404 {
    use super::*;
    use crate::*;
    #[test]
    fn test_next_pos() {
        let _rug_st_tests_llm_16_404_rrrruuuugggg_test_next_pos = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 'a';
        let rug_fuzz_2 = b'a';
        let rug_fuzz_3 = 1;
        let input = input::InputAt {
            pos: rug_fuzz_0,
            c: input::Char::from(Some(rug_fuzz_1)),
            byte: Some(rug_fuzz_2),
            len: rug_fuzz_3,
        };
        debug_assert_eq!(input.next_pos(), 1);
        let _rug_ed_tests_llm_16_404_rrrruuuugggg_test_next_pos = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_405 {
    use super::*;
    use crate::*;
    #[test]
    fn test_pos() {
        let _rug_st_tests_llm_16_405_rrrruuuugggg_test_pos = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 5;
        let input = input::InputAt {
            pos: rug_fuzz_0,
            c: input::Char(rug_fuzz_1),
            byte: Some(rug_fuzz_2),
            len: rug_fuzz_3,
        };
        debug_assert_eq!(input.pos(), 10);
        let _rug_ed_tests_llm_16_405_rrrruuuugggg_test_pos = 0;
    }
}
#[cfg(test)]
mod tests_rug_290 {
    use super::*;
    use crate::input::{Input, CharInput};
    #[test]
    fn test_is_empty() {
        let _rug_st_tests_rug_290_rrrruuuugggg_test_is_empty = 0;
        let rug_fuzz_0 = b"sample data";
        let s = rug_fuzz_0;
        let input: CharInput = CharInput::new(s);
        let result = Input::is_empty(&input);
        debug_assert_eq!(result, false);
        let _rug_ed_tests_rug_290_rrrruuuugggg_test_is_empty = 0;
    }
}
#[cfg(test)]
mod tests_rug_291 {
    use super::*;
    use crate::internal::Input;
    use crate::input::ByteInput;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_291_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"sample text";
        let rug_fuzz_1 = true;
        let rug_fuzz_2 = 5;
        let mut p0 = ByteInput::new(rug_fuzz_0, rug_fuzz_1);
        let mut p1: usize = rug_fuzz_2;
        p0.at(p1);
        let _rug_ed_tests_rug_291_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_296 {
    use super::*;
    use crate::internal::Input;
    use crate::internal::CharInput;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_296_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"sample data";
        let s = rug_fuzz_0;
        let p0: CharInput = CharInput::new(s);
        p0.len();
        let _rug_ed_tests_rug_296_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_297 {
    use super::*;
    use crate::input::CharInput;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_297_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"Hello, world!";
        let p0: &[u8] = rug_fuzz_0;
        CharInput::new(p0);
        let _rug_ed_tests_rug_297_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_301 {
    use super::*;
    use crate::input::Char;
    use crate::syntax;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_301_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 'a';
        let mut v56 = Char::from(Some(rug_fuzz_0));
        let p0 = v56;
        <Char>::is_word_byte(p0);
        let _rug_ed_tests_rug_301_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_303 {
    use super::*;
    use crate::input::Char;
    use std::convert::From;
    #[test]
    fn test_from() {
        let _rug_st_tests_rug_303_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = 'a';
        let p0: Option<char> = Some(rug_fuzz_0);
        <Char as From<Option<char>>>::from(p0);
        let _rug_ed_tests_rug_303_rrrruuuugggg_test_from = 0;
    }
}
