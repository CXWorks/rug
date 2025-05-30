use regex_automata::DFA;
use ext_slice::ByteSlice;
use unicode::fsm::sentence_break_fwd::SENTENCE_BREAK_FWD;
use utf8;
/// An iterator over sentences in a byte string.
///
/// This iterator is typically constructed by
/// [`ByteSlice::sentences`](trait.ByteSlice.html#method.sentences).
///
/// Sentences typically include their trailing punctuation and whitespace.
///
/// Since sentences are made up of one or more codepoints, this iterator yields
/// `&str` elements. When invalid UTF-8 is encountered, replacement codepoints
/// are [substituted](index.html#handling-of-invalid-utf-8).
///
/// This iterator yields words in accordance with the default sentence boundary
/// rules specified in
/// [UAX #29](https://www.unicode.org/reports/tr29/tr29-33.html#Sentence_Boundaries).
#[derive(Clone, Debug)]
pub struct Sentences<'a> {
    bs: &'a [u8],
}
impl<'a> Sentences<'a> {
    pub(crate) fn new(bs: &'a [u8]) -> Sentences<'a> {
        Sentences { bs }
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
    /// let mut it = b"I want this. Not that. Right now.".sentences();
    ///
    /// assert_eq!(&b"I want this. Not that. Right now."[..], it.as_bytes());
    /// it.next();
    /// assert_eq!(b"Not that. Right now.", it.as_bytes());
    /// it.next();
    /// it.next();
    /// assert_eq!(b"", it.as_bytes());
    /// ```
    #[inline]
    pub fn as_bytes(&self) -> &'a [u8] {
        self.bs
    }
}
impl<'a> Iterator for Sentences<'a> {
    type Item = &'a str;
    #[inline]
    fn next(&mut self) -> Option<&'a str> {
        let (sentence, size) = decode_sentence(self.bs);
        if size == 0 {
            return None;
        }
        self.bs = &self.bs[size..];
        Some(sentence)
    }
}
/// An iterator over sentences in a byte string, along with their byte offsets.
///
/// This iterator is typically constructed by
/// [`ByteSlice::sentence_indices`](trait.ByteSlice.html#method.sentence_indices).
///
/// Sentences typically include their trailing punctuation and whitespace.
///
/// Since sentences are made up of one or more codepoints, this iterator
/// yields `&str` elements (along with their start and end byte offsets).
/// When invalid UTF-8 is encountered, replacement codepoints are
/// [substituted](index.html#handling-of-invalid-utf-8). Because of this, the
/// indices yielded by this iterator may not correspond to the length of the
/// sentence yielded with those indices. For example, when this iterator
/// encounters `\xFF` in the byte string, then it will yield a pair of indices
/// ranging over a single byte, but will provide an `&str` equivalent to
/// `"\u{FFFD}"`, which is three bytes in length. However, when given only
/// valid UTF-8, then all indices are in exact correspondence with their paired
/// word.
///
/// This iterator yields words in accordance with the default sentence boundary
/// rules specified in
/// [UAX #29](https://www.unicode.org/reports/tr29/tr29-33.html#Sentence_Boundaries).
#[derive(Clone, Debug)]
pub struct SentenceIndices<'a> {
    bs: &'a [u8],
    forward_index: usize,
}
impl<'a> SentenceIndices<'a> {
    pub(crate) fn new(bs: &'a [u8]) -> SentenceIndices<'a> {
        SentenceIndices {
            bs: bs,
            forward_index: 0,
        }
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
    /// let mut it = b"I want this. Not that. Right now.".sentence_indices();
    ///
    /// assert_eq!(&b"I want this. Not that. Right now."[..], it.as_bytes());
    /// it.next();
    /// assert_eq!(b"Not that. Right now.", it.as_bytes());
    /// it.next();
    /// it.next();
    /// assert_eq!(b"", it.as_bytes());
    /// ```
    #[inline]
    pub fn as_bytes(&self) -> &'a [u8] {
        self.bs
    }
}
impl<'a> Iterator for SentenceIndices<'a> {
    type Item = (usize, usize, &'a str);
    #[inline]
    fn next(&mut self) -> Option<(usize, usize, &'a str)> {
        let index = self.forward_index;
        let (word, size) = decode_sentence(self.bs);
        if size == 0 {
            return None;
        }
        self.bs = &self.bs[size..];
        self.forward_index += size;
        Some((index, index + size, word))
    }
}
fn decode_sentence(bs: &[u8]) -> (&str, usize) {
    if bs.is_empty() {
        ("", 0)
    } else if let Some(end) = SENTENCE_BREAK_FWD.find(bs) {
        let sentence = unsafe { bs[..end].to_str_unchecked() };
        (sentence, sentence.len())
    } else {
        const INVALID: &'static str = "\u{FFFD}";
        let (_, size) = utf8::decode_lossy(bs);
        (INVALID, size)
    }
}
#[cfg(test)]
mod tests {
    use ucd_parse::SentenceBreakTest;
    use ext_slice::ByteSlice;
    #[test]
    fn forward_ucd() {
        for (i, test) in ucdtests().into_iter().enumerate() {
            let given = test.sentences.concat();
            let got = sentences(given.as_bytes());
            assert_eq!(
                test.sentences, got,
                "\n\nsentence forward break test {} failed:\n\
                 given:    {:?}\n\
                 expected: {:?}\n\
                 got:      {:?}\n",
                i, given, strs_to_bstrs(& test.sentences), strs_to_bstrs(& got),
            );
        }
    }
    #[test]
    fn forward_additional() {
        assert_eq!(vec!["a.. ", "A"], sentences(b"a.. A"));
        assert_eq!(vec!["a.. a"], sentences(b"a.. a"));
        assert_eq!(vec!["a... ", "A"], sentences(b"a... A"));
        assert_eq!(vec!["a... a"], sentences(b"a... a"));
        assert_eq!(vec!["a...,..., a"], sentences(b"a...,..., a"));
    }
    fn sentences(bytes: &[u8]) -> Vec<&str> {
        bytes.sentences().collect()
    }
    fn strs_to_bstrs<S: AsRef<str>>(strs: &[S]) -> Vec<&[u8]> {
        strs.iter().map(|s| s.as_ref().as_bytes()).collect()
    }
    /// Return all of the UCD for sentence breaks.
    fn ucdtests() -> Vec<SentenceBreakTest> {
        const TESTDATA: &'static str = include_str!("data/SentenceBreakTest.txt");
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
mod tests_rug_325 {
    use super::*;
    use crate::unicode::sentence::{decode_sentence, SENTENCE_BREAK_FWD};
    use crate::utf8;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_325_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"Hello, world!";
        let mut p0: &[u8] = rug_fuzz_0;
        decode_sentence(p0);
        let _rug_ed_tests_rug_325_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_326 {
    use super::*;
    use unicode::sentence::Sentences;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_326_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"Hello, this is a sample sentence.";
        let p0: &[u8] = rug_fuzz_0;
        Sentences::new(p0);
        let _rug_ed_tests_rug_326_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_327 {
    use super::*;
    use crate::ByteSlice;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_327_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"I want this. Not that. Right now.";
        let text = rug_fuzz_0;
        let mut it = text.sentences();
        crate::unicode::sentence::Sentences::<'_>::as_bytes(&it);
        let _rug_ed_tests_rug_327_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_329 {
    use super::*;
    use unicode::sentence::SentenceIndices;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_329_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"Hello, World!";
        let p0: &[u8] = rug_fuzz_0;
        SentenceIndices::new(p0);
        let _rug_ed_tests_rug_329_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_330 {
    use super::*;
    use crate::ByteSlice;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_330_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"I want this. Not that. Right now.";
        let rug_fuzz_1 = b"Not that. Right now.";
        let rug_fuzz_2 = b"";
        let data = rug_fuzz_0;
        let mut it = data.sentence_indices();
        debug_assert_eq!(& data[..], it.as_bytes());
        it.next();
        debug_assert_eq!(rug_fuzz_1, it.as_bytes());
        it.next();
        it.next();
        debug_assert_eq!(rug_fuzz_2, it.as_bytes());
        let _rug_ed_tests_rug_330_rrrruuuugggg_test_rug = 0;
    }
}
