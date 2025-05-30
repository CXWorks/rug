use regex_automata::DFA;

use ext_slice::ByteSlice;
use unicode::fsm::simple_word_fwd::SIMPLE_WORD_FWD;
use unicode::fsm::word_break_fwd::WORD_BREAK_FWD;
use utf8;

/// An iterator over words in a byte string.
///
/// This iterator is typically constructed by
/// [`ByteSlice::words`](trait.ByteSlice.html#method.words).
///
/// This is similar to the [`WordsWithBreaks`](struct.WordsWithBreaks.html)
/// iterator, except it only returns elements that contain a "word" character.
/// A word character is defined by UTS #18 (Annex C) to be the combination
/// of the `Alphabetic` and `Join_Control` properties, along with the
/// `Decimal_Number`, `Mark` and `Connector_Punctuation` general categories.
///
/// Since words are made up of one or more codepoints, this iterator yields
/// `&str` elements. When invalid UTF-8 is encountered, replacement codepoints
/// are [substituted](index.html#handling-of-invalid-utf-8).
///
/// This iterator yields words in accordance with the default word boundary
/// rules specified in
/// [UAX #29](https://www.unicode.org/reports/tr29/tr29-33.html#Word_Boundaries).
/// In particular, this may not be suitable for Japanese and Chinese scripts
/// that do not use spaces between words.
#[derive(Clone, Debug)]
pub struct Words<'a>(WordsWithBreaks<'a>);

impl<'a> Words<'a> {
    pub(crate) fn new(bs: &'a [u8]) -> Words<'a> {
        Words(WordsWithBreaks::new(bs))
    }

    /// View the underlying data as a subslice of the original data.
    ///
    /// The slice returned has the same lifetime as the original slice, and so
    /// the iterator can continue to be used while this exists.
    ///
    /// # Examples
    ///
    /// ```
    /// use bstr::ByteSlice;
    ///
    /// let mut it = b"foo bar baz".words();
    ///
    /// assert_eq!(b"foo bar baz", it.as_bytes());
    /// it.next();
    /// it.next();
    /// assert_eq!(b" baz", it.as_bytes());
    /// it.next();
    /// assert_eq!(b"", it.as_bytes());
    /// ```
    #[inline]
    pub fn as_bytes(&self) -> &'a [u8] {
        self.0.as_bytes()
    }
}

impl<'a> Iterator for Words<'a> {
    type Item = &'a str;

    #[inline]
    fn next(&mut self) -> Option<&'a str> {
        while let Some(word) = self.0.next() {
            if SIMPLE_WORD_FWD.is_match(word.as_bytes()) {
                return Some(word);
            }
        }
        None
    }
}

/// An iterator over words in a byte string and their byte index positions.
///
/// This iterator is typically constructed by
/// [`ByteSlice::word_indices`](trait.ByteSlice.html#method.word_indices).
///
/// This is similar to the
/// [`WordsWithBreakIndices`](struct.WordsWithBreakIndices.html) iterator,
/// except it only returns elements that contain a "word" character. A
/// word character is defined by UTS #18 (Annex C) to be the combination
/// of the `Alphabetic` and `Join_Control` properties, along with the
/// `Decimal_Number`, `Mark` and `Connector_Punctuation` general categories.
///
/// Since words are made up of one or more codepoints, this iterator
/// yields `&str` elements (along with their start and end byte offsets).
/// When invalid UTF-8 is encountered, replacement codepoints are
/// [substituted](index.html#handling-of-invalid-utf-8). Because of this, the
/// indices yielded by this iterator may not correspond to the length of the
/// word yielded with those indices. For example, when this iterator encounters
/// `\xFF` in the byte string, then it will yield a pair of indices ranging
/// over a single byte, but will provide an `&str` equivalent to `"\u{FFFD}"`,
/// which is three bytes in length. However, when given only valid UTF-8, then
/// all indices are in exact correspondence with their paired word.
///
/// This iterator yields words in accordance with the default word boundary
/// rules specified in
/// [UAX #29](https://www.unicode.org/reports/tr29/tr29-33.html#Word_Boundaries).
/// In particular, this may not be suitable for Japanese and Chinese scripts
/// that do not use spaces between words.
#[derive(Clone, Debug)]
pub struct WordIndices<'a>(WordsWithBreakIndices<'a>);

impl<'a> WordIndices<'a> {
    pub(crate) fn new(bs: &'a [u8]) -> WordIndices<'a> {
        WordIndices(WordsWithBreakIndices::new(bs))
    }

    /// View the underlying data as a subslice of the original data.
    ///
    /// The slice returned has the same lifetime as the original slice, and so
    /// the iterator can continue to be used while this exists.
    ///
    /// # Examples
    ///
    /// ```
    /// use bstr::ByteSlice;
    ///
    /// let mut it = b"foo bar baz".word_indices();
    ///
    /// assert_eq!(b"foo bar baz", it.as_bytes());
    /// it.next();
    /// it.next();
    /// assert_eq!(b" baz", it.as_bytes());
    /// it.next();
    /// it.next();
    /// assert_eq!(b"", it.as_bytes());
    /// ```
    #[inline]
    pub fn as_bytes(&self) -> &'a [u8] {
        self.0.as_bytes()
    }
}

impl<'a> Iterator for WordIndices<'a> {
    type Item = (usize, usize, &'a str);

    #[inline]
    fn next(&mut self) -> Option<(usize, usize, &'a str)> {
        while let Some((start, end, word)) = self.0.next() {
            if SIMPLE_WORD_FWD.is_match(word.as_bytes()) {
                return Some((start, end, word));
            }
        }
        None
    }
}

/// An iterator over all word breaks in a byte string.
///
/// This iterator is typically constructed by
/// [`ByteSlice::words_with_breaks`](trait.ByteSlice.html#method.words_with_breaks).
///
/// This iterator yields not only all words, but the content that comes between
/// words. In particular, if all elements yielded by this iterator are
/// concatenated, then the result is the original string (subject to Unicode
/// replacement codepoint substitutions).
///
/// Since words are made up of one or more codepoints, this iterator yields
/// `&str` elements. When invalid UTF-8 is encountered, replacement codepoints
/// are [substituted](index.html#handling-of-invalid-utf-8).
///
/// This iterator yields words in accordance with the default word boundary
/// rules specified in
/// [UAX #29](https://www.unicode.org/reports/tr29/tr29-33.html#Word_Boundaries).
/// In particular, this may not be suitable for Japanese and Chinese scripts
/// that do not use spaces between words.
#[derive(Clone, Debug)]
pub struct WordsWithBreaks<'a> {
    bs: &'a [u8],
}

impl<'a> WordsWithBreaks<'a> {
    pub(crate) fn new(bs: &'a [u8]) -> WordsWithBreaks<'a> {
        WordsWithBreaks { bs }
    }

    /// View the underlying data as a subslice of the original data.
    ///
    /// The slice returned has the same lifetime as the original slice, and so
    /// the iterator can continue to be used while this exists.
    ///
    /// # Examples
    ///
    /// ```
    /// use bstr::ByteSlice;
    ///
    /// let mut it = b"foo bar baz".words_with_breaks();
    ///
    /// assert_eq!(b"foo bar baz", it.as_bytes());
    /// it.next();
    /// assert_eq!(b" bar baz", it.as_bytes());
    /// it.next();
    /// it.next();
    /// assert_eq!(b" baz", it.as_bytes());
    /// it.next();
    /// it.next();
    /// assert_eq!(b"", it.as_bytes());
    /// ```
    #[inline]
    pub fn as_bytes(&self) -> &'a [u8] {
        self.bs
    }
}

impl<'a> Iterator for WordsWithBreaks<'a> {
    type Item = &'a str;

    #[inline]
    fn next(&mut self) -> Option<&'a str> {
        let (word, size) = decode_word(self.bs);
        if size == 0 {
            return None;
        }
        self.bs = &self.bs[size..];
        Some(word)
    }
}

/// An iterator over all word breaks in a byte string, along with their byte
/// index positions.
///
/// This iterator is typically constructed by
/// [`ByteSlice::words_with_break_indices`](trait.ByteSlice.html#method.words_with_break_indices).
///
/// This iterator yields not only all words, but the content that comes between
/// words. In particular, if all elements yielded by this iterator are
/// concatenated, then the result is the original string (subject to Unicode
/// replacement codepoint substitutions).
///
/// Since words are made up of one or more codepoints, this iterator
/// yields `&str` elements (along with their start and end byte offsets).
/// When invalid UTF-8 is encountered, replacement codepoints are
/// [substituted](index.html#handling-of-invalid-utf-8). Because of this, the
/// indices yielded by this iterator may not correspond to the length of the
/// word yielded with those indices. For example, when this iterator encounters
/// `\xFF` in the byte string, then it will yield a pair of indices ranging
/// over a single byte, but will provide an `&str` equivalent to `"\u{FFFD}"`,
/// which is three bytes in length. However, when given only valid UTF-8, then
/// all indices are in exact correspondence with their paired word.
///
/// This iterator yields words in accordance with the default word boundary
/// rules specified in
/// [UAX #29](https://www.unicode.org/reports/tr29/tr29-33.html#Word_Boundaries).
/// In particular, this may not be suitable for Japanese and Chinese scripts
/// that do not use spaces between words.
#[derive(Clone, Debug)]
pub struct WordsWithBreakIndices<'a> {
    bs: &'a [u8],
    forward_index: usize,
}

impl<'a> WordsWithBreakIndices<'a> {
    pub(crate) fn new(bs: &'a [u8]) -> WordsWithBreakIndices<'a> {
        WordsWithBreakIndices { bs: bs, forward_index: 0 }
    }

    /// View the underlying data as a subslice of the original data.
    ///
    /// The slice returned has the same lifetime as the original slice, and so
    /// the iterator can continue to be used while this exists.
    ///
    /// # Examples
    ///
    /// ```
    /// use bstr::ByteSlice;
    ///
    /// let mut it = b"foo bar baz".words_with_break_indices();
    ///
    /// assert_eq!(b"foo bar baz", it.as_bytes());
    /// it.next();
    /// assert_eq!(b" bar baz", it.as_bytes());
    /// it.next();
    /// it.next();
    /// assert_eq!(b" baz", it.as_bytes());
    /// it.next();
    /// it.next();
    /// assert_eq!(b"", it.as_bytes());
    /// ```
    #[inline]
    pub fn as_bytes(&self) -> &'a [u8] {
        self.bs
    }
}

impl<'a> Iterator for WordsWithBreakIndices<'a> {
    type Item = (usize, usize, &'a str);

    #[inline]
    fn next(&mut self) -> Option<(usize, usize, &'a str)> {
        let index = self.forward_index;
        let (word, size) = decode_word(self.bs);
        if size == 0 {
            return None;
        }
        self.bs = &self.bs[size..];
        self.forward_index += size;
        Some((index, index + size, word))
    }
}

fn decode_word(bs: &[u8]) -> (&str, usize) {
    if bs.is_empty() {
        ("", 0)
    } else if let Some(end) = WORD_BREAK_FWD.find(bs) {
        // Safe because a match can only occur for valid UTF-8.
        let word = unsafe { bs[..end].to_str_unchecked() };
        (word, word.len())
    } else {
        const INVALID: &'static str = "\u{FFFD}";
        // No match on non-empty bytes implies we found invalid UTF-8.
        let (_, size) = utf8::decode_lossy(bs);
        (INVALID, size)
    }
}

#[cfg(test)]
mod tests {
    use ucd_parse::WordBreakTest;

    use ext_slice::ByteSlice;

    #[test]
    fn forward_ucd() {
        for (i, test) in ucdtests().into_iter().enumerate() {
            let given = test.words.concat();
            let got = words(given.as_bytes());
            assert_eq!(
                test.words,
                got,
                "\n\nword forward break test {} failed:\n\
                 given:    {:?}\n\
                 expected: {:?}\n\
                 got:      {:?}\n",
                i,
                given,
                strs_to_bstrs(&test.words),
                strs_to_bstrs(&got),
            );
        }
    }

    // Some additional tests that don't seem to be covered by the UCD tests.
    //
    // It's pretty amazing that the UCD tests miss these cases. I only found
    // them by running this crate's segmenter and ICU's segmenter on the same
    // text and comparing the output.
    #[test]
    fn forward_additional() {
        assert_eq!(vec!["a", ".", "  ", "Y"], words(b"a.  Y"));
        assert_eq!(vec!["r", ".", "  ", "Yo"], words(b"r.  Yo"));
        assert_eq!(
            vec!["whatsoever", ".", "  ", "You", " ", "may"],
            words(b"whatsoever.  You may")
        );
        assert_eq!(
            vec!["21stcentury'syesterday"],
            words(b"21stcentury'syesterday")
        );

        assert_eq!(vec!["Bonta_", "'", "s"], words(b"Bonta_'s"));
        assert_eq!(vec!["_vhat's"], words(b"_vhat's"));
        assert_eq!(vec!["__on'anima"], words(b"__on'anima"));
        assert_eq!(vec!["123_", "'", "4"], words(b"123_'4"));
        assert_eq!(vec!["_123'4"], words(b"_123'4"));
        assert_eq!(vec!["__12'345"], words(b"__12'345"));

        assert_eq!(
            vec!["tomorrowat4", ":", "00", ","],
            words(b"tomorrowat4:00,")
        );
        assert_eq!(vec!["RS1", "'", "s"], words(b"RS1's"));
        assert_eq!(vec!["X38"], words(b"X38"));

        assert_eq!(vec!["4abc", ":", "00", ","], words(b"4abc:00,"));
        assert_eq!(vec!["12S", "'", "1"], words(b"12S'1"));
        assert_eq!(vec!["1XY"], words(b"1XY"));

        assert_eq!(vec!["\u{FEFF}", "Ты"], words("\u{FEFF}Ты".as_bytes()));
    }

    fn words(bytes: &[u8]) -> Vec<&str> {
        bytes.words_with_breaks().collect()
    }

    fn strs_to_bstrs<S: AsRef<str>>(strs: &[S]) -> Vec<&[u8]> {
        strs.iter().map(|s| s.as_ref().as_bytes()).collect()
    }

    /// Return all of the UCD for word breaks.
    fn ucdtests() -> Vec<WordBreakTest> {
        const TESTDATA: &'static str = include_str!("data/WordBreakTest.txt");

        let mut tests = vec![];
        for mut line in TESTDATA.lines() {
            line = line.trim();
            if line.starts_with("#") || line.contains("surrogate") {
                continue;
            }
            tests.push(line.parse().unwrap());
        }
        tests
    }
}

#[cfg(test)]
mod tests_rug_335 {
    use super::*;
    use crate::unicode::word::{Words, WordsWithBreaks};

    #[test]
    fn test_new() {
        let p0: &[u8] = b"hello world";
        
        Words::<'_>::new(p0);
    }
}
#[cfg(test)]
mod tests_rug_336 {
    use super::*;

    use crate::ByteSlice;

    #[test]
    fn test_as_bytes() {
        let mut p0 = b"foo bar baz".words();

        assert_eq!(b"foo bar baz", p0.as_bytes());
        p0.next();
        p0.next();
        assert_eq!(b" baz", p0.as_bytes());
        p0.next();
        assert_eq!(b"", p0.as_bytes());
    }
}#[cfg(test)]
mod tests_rug_338 {
    use super::*;
    use unicode::word::{WordIndices, WordsWithBreakIndices};

    #[test]
    fn test_new() {
        let p0: &[u8] = b"Hello, World!";

        WordIndices::new(p0);
    }
}#[cfg(test)]
mod tests_rug_340 {
    use super::*;
    use crate::unicode::word::WordIndices;
    use crate::std::iter::Iterator;
    
    #[test]
    fn test_rug() {
        let mut p0: WordIndices = unimplemented!();
        
        p0.next();
    }
}#[cfg(test)]
mod tests_rug_341 {
    use super::*;
    use unicode::word::WordsWithBreaks;

    #[test]
    fn test_new() {
        let data: &'static [u8] = b"Hello, world!";
        let p0 = data;

        WordsWithBreaks::new(p0);
    }
}#[cfg(test)]
mod tests_rug_342 {
    use super::*;
    use crate::ByteSlice;

    #[test]
    fn test_as_bytes() {
        let data = b"foo bar baz";
        let mut it = data.words_with_breaks();
        let p0 = &it;

        assert_eq!(b"foo bar baz", it.as_bytes());
        it.next();
        assert_eq!(b" bar baz", it.as_bytes());
        it.next();
        it.next();
        assert_eq!(b" baz", it.as_bytes());
        it.next();
        it.next();
        assert_eq!(b"", it.as_bytes());
    }
}#[cfg(test)]
mod tests_rug_343 {
    use super::*;
    use unicode::word::WordsWithBreaks;
    use crate::std::iter::Iterator;
    
    #[test]
    fn test_rug() {
        let bs: &[u8] = b"hello_world";
        let words_with_breaks = WordsWithBreaks::new(bs);
        let mut p0: WordsWithBreaks<'_> = words_with_breaks;

        assert_eq!(p0.next(), Some("hello"));
        assert_eq!(p0.next(), Some("world"));
        assert_eq!(p0.next(), None);
    }
}
#[cfg(test)]
mod tests_rug_344 {
    use super::*;
    use unicode::word::WordsWithBreakIndices;

    #[test]
    fn test_new() {
        let p0: &[u8] = b"hello world";

        WordsWithBreakIndices::<'_>::new(p0);

    }
}
#[cfg(test)]
mod tests_rug_345 {
    use super::*;
    use crate::ByteSlice;
    use unicode::word::WordsWithBreakIndices;

    #[test]
    fn test_rug() {
        let data = b"foo bar baz";
        let mut it = data.words_with_break_indices();

        let p0 = &it;

        assert_eq!(b"foo bar baz", <WordsWithBreakIndices<'_>>::as_bytes(p0));
    }
}#[cfg(test)]
mod tests_rug_346 {
    use super::*;
    use crate::std::iter::Iterator;
    use unicode::word::WordsWithBreakIndices;

    #[test]
    fn test_rug() {
        let bs = "Hello Rustacean";
        let mut words = WordsWithBreakIndices::<'_>::new(bs.as_bytes());
        
        words.next();
    }
}