use crate::text::StyledGrapheme;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;
const NBSP: &str = "\u{00a0}";
/// A state machine to pack styled symbols into lines.
/// Cannot implement it as Iterator since it yields slices of the internal buffer (need streaming
/// iterators for that).
pub trait LineComposer<'a> {
    fn next_line(&mut self) -> Option<(&[StyledGrapheme<'a>], u16)>;
}
/// A state machine that wraps lines on word boundaries.
pub struct WordWrapper<'a, 'b> {
    symbols: &'b mut dyn Iterator<Item = StyledGrapheme<'a>>,
    max_line_width: u16,
    current_line: Vec<StyledGrapheme<'a>>,
    next_line: Vec<StyledGrapheme<'a>>,
    /// Removes the leading whitespace from lines
    trim: bool,
}
impl<'a, 'b> WordWrapper<'a, 'b> {
    pub fn new(
        symbols: &'b mut dyn Iterator<Item = StyledGrapheme<'a>>,
        max_line_width: u16,
        trim: bool,
    ) -> WordWrapper<'a, 'b> {
        WordWrapper {
            symbols,
            max_line_width,
            current_line: vec![],
            next_line: vec![],
            trim,
        }
    }
}
impl<'a, 'b> LineComposer<'a> for WordWrapper<'a, 'b> {
    fn next_line(&mut self) -> Option<(&[StyledGrapheme<'a>], u16)> {
        if self.max_line_width == 0 {
            return None;
        }
        std::mem::swap(&mut self.current_line, &mut self.next_line);
        self.next_line.truncate(0);
        let mut current_line_width = self
            .current_line
            .iter()
            .map(|StyledGrapheme { symbol, .. }| symbol.width() as u16)
            .sum();
        let mut symbols_to_last_word_end: usize = 0;
        let mut width_to_last_word_end: u16 = 0;
        let mut prev_whitespace = false;
        let mut symbols_exhausted = true;
        for StyledGrapheme { symbol, style } in &mut self.symbols {
            symbols_exhausted = false;
            let symbol_whitespace = symbol.chars().all(&char::is_whitespace);
            if symbol.width() as u16 > self.max_line_width
                || self.trim && symbol_whitespace && symbol != "\n"
                    && current_line_width == 0
            {
                continue;
            }
            if symbol == "\n" {
                if prev_whitespace {
                    current_line_width = width_to_last_word_end;
                    self.current_line.truncate(symbols_to_last_word_end);
                }
                break;
            }
            if symbol_whitespace && !prev_whitespace && symbol != NBSP {
                symbols_to_last_word_end = self.current_line.len();
                width_to_last_word_end = current_line_width;
            }
            self.current_line.push(StyledGrapheme { symbol, style });
            current_line_width += symbol.width() as u16;
            if current_line_width > self.max_line_width {
                let (truncate_at, truncated_width) = if symbols_to_last_word_end != 0 {
                    (symbols_to_last_word_end, width_to_last_word_end)
                } else {
                    (self.current_line.len() - 1, self.max_line_width)
                };
                {
                    let remainder = &self.current_line[truncate_at..];
                    if let Some(remainder_nonwhite)
                        = remainder
                            .iter()
                            .position(|StyledGrapheme { symbol, .. }| {
                                !symbol.chars().all(&char::is_whitespace)
                            })
                    {
                        self.next_line
                            .extend_from_slice(&remainder[remainder_nonwhite..]);
                    }
                }
                self.current_line.truncate(truncate_at);
                current_line_width = truncated_width;
                break;
            }
            prev_whitespace = symbol_whitespace;
        }
        if symbols_exhausted && self.current_line.is_empty() {
            None
        } else {
            Some((&self.current_line[..], current_line_width))
        }
    }
}
/// A state machine that truncates overhanging lines.
pub struct LineTruncator<'a, 'b> {
    symbols: &'b mut dyn Iterator<Item = StyledGrapheme<'a>>,
    max_line_width: u16,
    current_line: Vec<StyledGrapheme<'a>>,
    /// Record the offet to skip render
    horizontal_offset: u16,
}
impl<'a, 'b> LineTruncator<'a, 'b> {
    pub fn new(
        symbols: &'b mut dyn Iterator<Item = StyledGrapheme<'a>>,
        max_line_width: u16,
    ) -> LineTruncator<'a, 'b> {
        LineTruncator {
            symbols,
            max_line_width,
            horizontal_offset: 0,
            current_line: vec![],
        }
    }
    pub fn set_horizontal_offset(&mut self, horizontal_offset: u16) {
        self.horizontal_offset = horizontal_offset;
    }
}
impl<'a, 'b> LineComposer<'a> for LineTruncator<'a, 'b> {
    fn next_line(&mut self) -> Option<(&[StyledGrapheme<'a>], u16)> {
        if self.max_line_width == 0 {
            return None;
        }
        self.current_line.truncate(0);
        let mut current_line_width = 0;
        let mut skip_rest = false;
        let mut symbols_exhausted = true;
        let mut horizontal_offset = self.horizontal_offset as usize;
        for StyledGrapheme { symbol, style } in &mut self.symbols {
            symbols_exhausted = false;
            if symbol.width() as u16 > self.max_line_width {
                continue;
            }
            if symbol == "\n" {
                break;
            }
            if current_line_width + symbol.width() as u16 > self.max_line_width {
                skip_rest = true;
                break;
            }
            let symbol = if horizontal_offset == 0 {
                symbol
            } else {
                let w = symbol.width();
                if w > horizontal_offset {
                    let t = trim_offset(symbol, horizontal_offset);
                    horizontal_offset = 0;
                    t
                } else {
                    horizontal_offset -= w;
                    ""
                }
            };
            current_line_width += symbol.width() as u16;
            self.current_line.push(StyledGrapheme { symbol, style });
        }
        if skip_rest {
            for StyledGrapheme { symbol, .. } in &mut self.symbols {
                if symbol == "\n" {
                    break;
                }
            }
        }
        if symbols_exhausted && self.current_line.is_empty() {
            None
        } else {
            Some((&self.current_line[..], current_line_width))
        }
    }
}
/// This function will return a str slice which start at specified offset.
/// As src is a unicode str, start offset has to be calculated with each character.
fn trim_offset(src: &str, mut offset: usize) -> &str {
    let mut start = 0;
    for c in UnicodeSegmentation::graphemes(src, true) {
        let w = c.width();
        if w <= offset {
            offset -= w;
            start += c.len();
        } else {
            break;
        }
    }
    &src[start..]
}
#[cfg(test)]
mod test {
    use super::*;
    use unicode_segmentation::UnicodeSegmentation;
    enum Composer {
        WordWrapper { trim: bool },
        LineTruncator,
    }
    fn run_composer(
        which: Composer,
        text: &str,
        text_area_width: u16,
    ) -> (Vec<String>, Vec<u16>) {
        let style = Default::default();
        let mut styled = UnicodeSegmentation::graphemes(text, true)
            .map(|g| StyledGrapheme { symbol: g, style });
        let mut composer: Box<dyn LineComposer> = match which {
            Composer::WordWrapper { trim } => {
                Box::new(WordWrapper::new(&mut styled, text_area_width, trim))
            }
            Composer::LineTruncator => {
                Box::new(LineTruncator::new(&mut styled, text_area_width))
            }
        };
        let mut lines = vec![];
        let mut widths = vec![];
        while let Some((styled, width)) = composer.next_line() {
            let line = styled
                .iter()
                .map(|StyledGrapheme { symbol, .. }| *symbol)
                .collect::<String>();
            assert!(width <= text_area_width);
            lines.push(line);
            widths.push(width);
        }
        (lines, widths)
    }
    #[test]
    fn line_composer_one_line() {
        let width = 40;
        for i in 1..width {
            let text = "a".repeat(i);
            let (word_wrapper, _) = run_composer(
                Composer::WordWrapper {
                    trim: true,
                },
                &text,
                width as u16,
            );
            let (line_truncator, _) = run_composer(
                Composer::LineTruncator,
                &text,
                width as u16,
            );
            let expected = vec![text];
            assert_eq!(word_wrapper, expected);
            assert_eq!(line_truncator, expected);
        }
    }
    #[test]
    fn line_composer_short_lines() {
        let width = 20;
        let text = "abcdefg\nhijklmno\npabcdefg\nhijklmn\nopabcdefghijk\nlmnopabcd\n\n\nefghijklmno";
        let (word_wrapper, _) = run_composer(
            Composer::WordWrapper {
                trim: true,
            },
            text,
            width,
        );
        let (line_truncator, _) = run_composer(Composer::LineTruncator, text, width);
        let wrapped: Vec<&str> = text.split('\n').collect();
        assert_eq!(word_wrapper, wrapped);
        assert_eq!(line_truncator, wrapped);
    }
    #[test]
    fn line_composer_long_word() {
        let width = 20;
        let text = "abcdefghijklmnopabcdefghijklmnopabcdefghijklmnopabcdefghijklmno";
        let (word_wrapper, _) = run_composer(
            Composer::WordWrapper {
                trim: true,
            },
            text,
            width as u16,
        );
        let (line_truncator, _) = run_composer(
            Composer::LineTruncator,
            text,
            width as u16,
        );
        let wrapped = vec![
            & text[..width], & text[width..width * 2], & text[width * 2..width * 3], &
            text[width * 3..],
        ];
        assert_eq!(
            word_wrapper, wrapped,
            "WordWrapper should detect the line cannot be broken on word boundary and \
             break it at line width limit."
        );
        assert_eq!(line_truncator, vec![& text[..width]]);
    }
    #[test]
    fn line_composer_long_sentence() {
        let width = 20;
        let text = "abcd efghij klmnopabcd efgh ijklmnopabcdefg hijkl mnopab c d e f g h i j k l m n o";
        let text_multi_space = "abcd efghij    klmnopabcd efgh     ijklmnopabcdefg hijkl mnopab c d e f g h i j k l \
             m n o";
        let (word_wrapper_single_space, _) = run_composer(
            Composer::WordWrapper {
                trim: true,
            },
            text,
            width as u16,
        );
        let (word_wrapper_multi_space, _) = run_composer(
            Composer::WordWrapper {
                trim: true,
            },
            text_multi_space,
            width as u16,
        );
        let (line_truncator, _) = run_composer(
            Composer::LineTruncator,
            text,
            width as u16,
        );
        let word_wrapped = vec![
            "abcd efghij", "klmnopabcd efgh", "ijklmnopabcdefg", "hijkl mnopab c d e f",
            "g h i j k l m n o",
        ];
        assert_eq!(word_wrapper_single_space, word_wrapped);
        assert_eq!(word_wrapper_multi_space, word_wrapped);
        assert_eq!(line_truncator, vec![& text[..width]]);
    }
    #[test]
    fn line_composer_zero_width() {
        let width = 0;
        let text = "abcd efghij klmnopabcd efgh ijklmnopabcdefg hijkl mnopab ";
        let (word_wrapper, _) = run_composer(
            Composer::WordWrapper {
                trim: true,
            },
            text,
            width,
        );
        let (line_truncator, _) = run_composer(Composer::LineTruncator, text, width);
        let expected: Vec<&str> = Vec::new();
        assert_eq!(word_wrapper, expected);
        assert_eq!(line_truncator, expected);
    }
    #[test]
    fn line_composer_max_line_width_of_1() {
        let width = 1;
        let text = "abcd efghij klmnopabcd efgh ijklmnopabcdefg hijkl mnopab ";
        let (word_wrapper, _) = run_composer(
            Composer::WordWrapper {
                trim: true,
            },
            text,
            width,
        );
        let (line_truncator, _) = run_composer(Composer::LineTruncator, text, width);
        let expected: Vec<&str> = UnicodeSegmentation::graphemes(text, true)
            .filter(|g| g.chars().any(|c| !c.is_whitespace()))
            .collect();
        assert_eq!(word_wrapper, expected);
        assert_eq!(line_truncator, vec!["a"]);
    }
    #[test]
    fn line_composer_max_line_width_of_1_double_width_characters() {
        let width = 1;
        let text = "コンピュータ上で文字を扱う場合、典型的には文字\naaaによる通信を行う場合にその\
                    両端点では、";
        let (word_wrapper, _) = run_composer(
            Composer::WordWrapper {
                trim: true,
            },
            text,
            width,
        );
        let (line_truncator, _) = run_composer(Composer::LineTruncator, text, width);
        assert_eq!(word_wrapper, vec!["", "a", "a", "a"]);
        assert_eq!(line_truncator, vec!["", "a"]);
    }
    /// Tests WordWrapper with words some of which exceed line length and some not.
    #[test]
    fn line_composer_word_wrapper_mixed_length() {
        let width = 20;
        let text = "abcd efghij klmnopabcdefghijklmnopabcdefghijkl mnopab cdefghi j klmno";
        let (word_wrapper, _) = run_composer(
            Composer::WordWrapper {
                trim: true,
            },
            text,
            width,
        );
        assert_eq!(
            word_wrapper, vec!["abcd efghij", "klmnopabcdefghijklmn", "opabcdefghijkl",
            "mnopab cdefghi j", "klmno",]
        )
    }
    #[test]
    fn line_composer_double_width_chars() {
        let width = 20;
        let text = "コンピュータ上で文字を扱う場合、典型的には文字による通信を行う場合にその両端点\
                    では、";
        let (word_wrapper, word_wrapper_width) = run_composer(
            Composer::WordWrapper {
                trim: true,
            },
            &text,
            width,
        );
        let (line_truncator, _) = run_composer(Composer::LineTruncator, &text, width);
        assert_eq!(line_truncator, vec!["コンピュータ上で文字"]);
        let wrapped = vec![
            "コンピュータ上で文字", "を扱う場合、典型的に",
            "は文字による通信を行", "う場合にその両端点で", "は、",
        ];
        assert_eq!(word_wrapper, wrapped);
        assert_eq!(word_wrapper_width, vec![width, width, width, width, 4]);
    }
    #[test]
    fn line_composer_leading_whitespace_removal() {
        let width = 20;
        let text = "AAAAAAAAAAAAAAAAAAAA    AAA";
        let (word_wrapper, _) = run_composer(
            Composer::WordWrapper {
                trim: true,
            },
            text,
            width,
        );
        let (line_truncator, _) = run_composer(Composer::LineTruncator, text, width);
        assert_eq!(word_wrapper, vec!["AAAAAAAAAAAAAAAAAAAA", "AAA",]);
        assert_eq!(line_truncator, vec!["AAAAAAAAAAAAAAAAAAAA"]);
    }
    /// Tests truncation of leading whitespace.
    #[test]
    fn line_composer_lots_of_spaces() {
        let width = 20;
        let text = "                                                                     ";
        let (word_wrapper, _) = run_composer(
            Composer::WordWrapper {
                trim: true,
            },
            text,
            width,
        );
        let (line_truncator, _) = run_composer(Composer::LineTruncator, text, width);
        assert_eq!(word_wrapper, vec![""]);
        assert_eq!(line_truncator, vec!["                    "]);
    }
    /// Tests an input starting with a letter, folowed by spaces - some of the behaviour is
    /// incidental.
    #[test]
    fn line_composer_char_plus_lots_of_spaces() {
        let width = 20;
        let text = "a                                                                     ";
        let (word_wrapper, _) = run_composer(
            Composer::WordWrapper {
                trim: true,
            },
            text,
            width,
        );
        let (line_truncator, _) = run_composer(Composer::LineTruncator, text, width);
        assert_eq!(word_wrapper, vec!["a", ""]);
        assert_eq!(line_truncator, vec!["a                   "]);
    }
    #[test]
    fn line_composer_word_wrapper_double_width_chars_mixed_with_spaces() {
        let width = 20;
        let text = "コンピュ ータ上で文字を扱う場合、 典型的には文 字による 通信を行 う場合にその両端点では、";
        let (word_wrapper, word_wrapper_width) = run_composer(
            Composer::WordWrapper {
                trim: true,
            },
            text,
            width,
        );
        assert_eq!(
            word_wrapper, vec!["コンピュ", "ータ上で文字を扱う場",
            "合、 典型的には文", "字による 通信を行",
            "う場合にその両端点で", "は、",]
        );
        assert_eq!(word_wrapper_width, vec![8, 20, 17, 17, 20, 4]);
    }
    /// Ensure words separated by nbsp are wrapped as if they were a single one.
    #[test]
    fn line_composer_word_wrapper_nbsp() {
        let width = 20;
        let text = "AAAAAAAAAAAAAAA AAAA\u{00a0}AAA";
        let (word_wrapper, _) = run_composer(
            Composer::WordWrapper {
                trim: true,
            },
            text,
            width,
        );
        assert_eq!(word_wrapper, vec!["AAAAAAAAAAAAAAA", "AAAA\u{00a0}AAA",]);
        let text_space = text.replace("\u{00a0}", " ");
        let (word_wrapper_space, _) = run_composer(
            Composer::WordWrapper {
                trim: true,
            },
            &text_space,
            width,
        );
        assert_eq!(word_wrapper_space, vec!["AAAAAAAAAAAAAAA AAAA", "AAA",]);
    }
    #[test]
    fn line_composer_word_wrapper_preserve_indentation() {
        let width = 20;
        let text = "AAAAAAAAAAAAAAAAAAAA    AAA";
        let (word_wrapper, _) = run_composer(
            Composer::WordWrapper {
                trim: false,
            },
            text,
            width,
        );
        assert_eq!(word_wrapper, vec!["AAAAAAAAAAAAAAAAAAAA", "   AAA",]);
    }
    #[test]
    fn line_composer_word_wrapper_preserve_indentation_with_wrap() {
        let width = 10;
        let text = "AAA AAA AAAAA AA AAAAAA\n B\n  C\n   D";
        let (word_wrapper, _) = run_composer(
            Composer::WordWrapper {
                trim: false,
            },
            text,
            width,
        );
        assert_eq!(
            word_wrapper, vec!["AAA AAA", "AAAAA AA", "AAAAAA", " B", "  C", "   D"]
        );
    }
    #[test]
    fn line_composer_word_wrapper_preserve_indentation_lots_of_whitespace() {
        let width = 10;
        let text = "               4 Indent\n                 must wrap!";
        let (word_wrapper, _) = run_composer(
            Composer::WordWrapper {
                trim: false,
            },
            text,
            width,
        );
        assert_eq!(
            word_wrapper, vec!["          ", "    4", "Indent", "          ",
            "      must", "wrap!"]
        );
    }
}
#[cfg(test)]
mod tests_llm_16_374_llm_16_373 {
    use crate::widgets::reflow::trim_offset;
    use unicode_segmentation::UnicodeSegmentation;
    #[test]
    fn test_trim_offset() {
        let _rug_st_tests_llm_16_374_llm_16_373_rrrruuuugggg_test_trim_offset = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = "hello";
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = "hello";
        let rug_fuzz_5 = 2;
        let rug_fuzz_6 = "你好";
        let rug_fuzz_7 = 1;
        let rug_fuzz_8 = "你好";
        let rug_fuzz_9 = 2;
        let rug_fuzz_10 = "😀🌍";
        let rug_fuzz_11 = 0;
        let rug_fuzz_12 = "😀🌍";
        let rug_fuzz_13 = 1;
        let rug_fuzz_14 = "😀🌍";
        let rug_fuzz_15 = 2;
        debug_assert_eq!(trim_offset(rug_fuzz_0, rug_fuzz_1), "");
        debug_assert_eq!(trim_offset(rug_fuzz_2, rug_fuzz_3), "hello");
        debug_assert_eq!(trim_offset(rug_fuzz_4, rug_fuzz_5), "llo");
        debug_assert_eq!(trim_offset(rug_fuzz_6, rug_fuzz_7), "好");
        debug_assert_eq!(trim_offset(rug_fuzz_8, rug_fuzz_9), "");
        debug_assert_eq!(trim_offset(rug_fuzz_10, rug_fuzz_11), "😀🌍");
        debug_assert_eq!(trim_offset(rug_fuzz_12, rug_fuzz_13), "🌍");
        debug_assert_eq!(trim_offset(rug_fuzz_14, rug_fuzz_15), "");
        let _rug_ed_tests_llm_16_374_llm_16_373_rrrruuuugggg_test_trim_offset = 0;
    }
}
#[cfg(test)]
mod tests_rug_16 {
    use super::*;
    use crate::widgets::reflow::WordWrapper;
    use crate::text::StyledGrapheme;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_16_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = false;
        let mut p0: Box<dyn Iterator<Item = StyledGrapheme<'static>> + 'static> = Box::new(
            std::iter::empty(),
        );
        let p1: u16 = rug_fuzz_0;
        let p2: bool = rug_fuzz_1;
        WordWrapper::new(&mut p0, p1, p2);
        let _rug_ed_tests_rug_16_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_18 {
    use super::*;
    use crate::widgets::reflow::LineTruncator;
    use crate::text::StyledGrapheme;
    use std::iter;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_18_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: Box<dyn Iterator<Item = StyledGrapheme<'static>> + 'static> = Box::new(
            iter::empty(),
        );
        let p1: u16 = rug_fuzz_0;
        LineTruncator::new(&mut *p0, p1);
        let _rug_ed_tests_rug_18_rrrruuuugggg_test_rug = 0;
    }
}
