use regex_automata::DFA;
use ext_slice::ByteSlice;
use unicode::fsm::grapheme_break_fwd::GRAPHEME_BREAK_FWD;
use unicode::fsm::grapheme_break_rev::GRAPHEME_BREAK_REV;
use unicode::fsm::regional_indicator_rev::REGIONAL_INDICATOR_REV;
use utf8;
/// An iterator over grapheme clusters in a byte string.
///
/// This iterator is typically constructed by
/// [`ByteSlice::graphemes`](trait.ByteSlice.html#method.graphemes).
///
/// Unicode defines a grapheme cluster as an *approximation* to a single user
/// visible character. A grapheme cluster, or just "grapheme," is made up of
/// one or more codepoints. For end user oriented tasks, one should generally
/// prefer using graphemes instead of [`Chars`](struct.Chars.html), which
/// always yields one codepoint at a time.
///
/// Since graphemes are made up of one or more codepoints, this iterator yields
/// `&str` elements. When invalid UTF-8 is encountered, replacement codepoints
/// are [substituted](index.html#handling-of-invalid-utf-8).
///
/// This iterator can be used in reverse. When reversed, exactly the same
/// set of grapheme clusters are yielded, but in reverse order.
///
/// This iterator only yields *extended* grapheme clusters, in accordance with
/// [UAX #29](https://www.unicode.org/reports/tr29/tr29-33.html#Grapheme_Cluster_Boundaries).
#[derive(Clone, Debug)]
pub struct Graphemes<'a> {
    bs: &'a [u8],
}
impl<'a> Graphemes<'a> {
    pub(crate) fn new(bs: &'a [u8]) -> Graphemes<'a> {
        Graphemes { bs }
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
    /// let mut it = b"abc".graphemes();
    ///
    /// assert_eq!(b"abc", it.as_bytes());
    /// it.next();
    /// assert_eq!(b"bc", it.as_bytes());
    /// it.next();
    /// it.next();
    /// assert_eq!(b"", it.as_bytes());
    /// ```
    #[inline]
    pub fn as_bytes(&self) -> &'a [u8] {
        self.bs
    }
}
impl<'a> Iterator for Graphemes<'a> {
    type Item = &'a str;
    #[inline]
    fn next(&mut self) -> Option<&'a str> {
        let (grapheme, size) = decode_grapheme(self.bs);
        if size == 0 {
            return None;
        }
        self.bs = &self.bs[size..];
        Some(grapheme)
    }
}
impl<'a> DoubleEndedIterator for Graphemes<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a str> {
        let (grapheme, size) = decode_last_grapheme(self.bs);
        if size == 0 {
            return None;
        }
        self.bs = &self.bs[..self.bs.len() - size];
        Some(grapheme)
    }
}
/// An iterator over grapheme clusters in a byte string and their byte index
/// positions.
///
/// This iterator is typically constructed by
/// [`ByteSlice::grapheme_indices`](trait.ByteSlice.html#method.grapheme_indices).
///
/// Unicode defines a grapheme cluster as an *approximation* to a single user
/// visible character. A grapheme cluster, or just "grapheme," is made up of
/// one or more codepoints. For end user oriented tasks, one should generally
/// prefer using graphemes instead of [`Chars`](struct.Chars.html), which
/// always yields one codepoint at a time.
///
/// Since graphemes are made up of one or more codepoints, this iterator
/// yields `&str` elements (along with their start and end byte offsets).
/// When invalid UTF-8 is encountered, replacement codepoints are
/// [substituted](index.html#handling-of-invalid-utf-8). Because of this, the
/// indices yielded by this iterator may not correspond to the length of the
/// grapheme cluster yielded with those indices. For example, when this
/// iterator encounters `\xFF` in the byte string, then it will yield a pair
/// of indices ranging over a single byte, but will provide an `&str`
/// equivalent to `"\u{FFFD}"`, which is three bytes in length. However, when
/// given only valid UTF-8, then all indices are in exact correspondence with
/// their paired grapheme cluster.
///
/// This iterator can be used in reverse. When reversed, exactly the same
/// set of grapheme clusters are yielded, but in reverse order.
///
/// This iterator only yields *extended* grapheme clusters, in accordance with
/// [UAX #29](https://www.unicode.org/reports/tr29/tr29-33.html#Grapheme_Cluster_Boundaries).
#[derive(Clone, Debug)]
pub struct GraphemeIndices<'a> {
    bs: &'a [u8],
    forward_index: usize,
    reverse_index: usize,
}
impl<'a> GraphemeIndices<'a> {
    pub(crate) fn new(bs: &'a [u8]) -> GraphemeIndices<'a> {
        GraphemeIndices {
            bs: bs,
            forward_index: 0,
            reverse_index: bs.len(),
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
    /// let mut it = b"abc".grapheme_indices();
    ///
    /// assert_eq!(b"abc", it.as_bytes());
    /// it.next();
    /// assert_eq!(b"bc", it.as_bytes());
    /// it.next();
    /// it.next();
    /// assert_eq!(b"", it.as_bytes());
    /// ```
    #[inline]
    pub fn as_bytes(&self) -> &'a [u8] {
        self.bs
    }
}
impl<'a> Iterator for GraphemeIndices<'a> {
    type Item = (usize, usize, &'a str);
    #[inline]
    fn next(&mut self) -> Option<(usize, usize, &'a str)> {
        let index = self.forward_index;
        let (grapheme, size) = decode_grapheme(self.bs);
        if size == 0 {
            return None;
        }
        self.bs = &self.bs[size..];
        self.forward_index += size;
        Some((index, index + size, grapheme))
    }
}
impl<'a> DoubleEndedIterator for GraphemeIndices<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<(usize, usize, &'a str)> {
        let (grapheme, size) = decode_last_grapheme(self.bs);
        if size == 0 {
            return None;
        }
        self.bs = &self.bs[..self.bs.len() - size];
        self.reverse_index -= size;
        Some((self.reverse_index, self.reverse_index + size, grapheme))
    }
}
/// Decode a grapheme from the given byte string.
///
/// This returns the resulting grapheme (which may be a Unicode replacement
/// codepoint if invalid UTF-8 was found), along with the number of bytes
/// decoded in the byte string. The number of bytes decoded may not be the
/// same as the length of grapheme in the case where invalid UTF-8 is found.
pub fn decode_grapheme(bs: &[u8]) -> (&str, usize) {
    if bs.is_empty() {
        ("", 0)
    } else if let Some(end) = GRAPHEME_BREAK_FWD.find(bs) {
        let grapheme = unsafe { bs[..end].to_str_unchecked() };
        (grapheme, grapheme.len())
    } else {
        const INVALID: &'static str = "\u{FFFD}";
        let (_, size) = utf8::decode_lossy(bs);
        (INVALID, size)
    }
}
fn decode_last_grapheme(bs: &[u8]) -> (&str, usize) {
    if bs.is_empty() {
        ("", 0)
    } else if let Some(mut start) = GRAPHEME_BREAK_REV.rfind(bs) {
        start = adjust_rev_for_regional_indicator(bs, start);
        let grapheme = unsafe { bs[start..].to_str_unchecked() };
        (grapheme, grapheme.len())
    } else {
        const INVALID: &'static str = "\u{FFFD}";
        let (_, size) = utf8::decode_last_lossy(bs);
        (INVALID, size)
    }
}
/// Return the correct offset for the next grapheme decoded at the end of the
/// given byte string, where `i` is the initial guess. In particular,
/// `&bs[i..]` represents the candidate grapheme.
///
/// `i` is returned by this function in all cases except when `&bs[i..]` is
/// a pair of regional indicator codepoints. In that case, if an odd number of
/// additional regional indicator codepoints precedes `i`, then `i` is
/// adjusted such that it points to only a single regional indicator.
///
/// This "fixing" is necessary to handle the requirement that a break cannot
/// occur between regional indicators where it would cause an odd number of
/// regional indicators to exist before the break from the *start* of the
/// string. A reverse regex cannot detect this case easily without look-around.
fn adjust_rev_for_regional_indicator(mut bs: &[u8], i: usize) -> usize {
    if bs.len() - i != 8 {
        return i;
    }
    let mut count = 0;
    while let Some(start) = REGIONAL_INDICATOR_REV.rfind(bs) {
        bs = &bs[..start];
        count += 1;
    }
    if count % 2 == 0 { i } else { i + 4 }
}
#[cfg(test)]
mod tests {
    use ucd_parse::GraphemeClusterBreakTest;
    use super::*;
    use ext_slice::ByteSlice;
    use tests::LOSSY_TESTS;
    #[test]
    fn forward_ucd() {
        for (i, test) in ucdtests().into_iter().enumerate() {
            let given = test.grapheme_clusters.concat();
            let got: Vec<String> = Graphemes::new(given.as_bytes())
                .map(|cluster| cluster.to_string())
                .collect();
            assert_eq!(
                test.grapheme_clusters, got,
                "\ngrapheme forward break test {} failed:\n\
                 given:    {:?}\n\
                 expected: {:?}\n\
                 got:      {:?}\n",
                i, uniescape(& given), uniescape_vec(& test.grapheme_clusters),
                uniescape_vec(& got),
            );
        }
    }
    #[test]
    fn reverse_ucd() {
        for (i, test) in ucdtests().into_iter().enumerate() {
            let given = test.grapheme_clusters.concat();
            let mut got: Vec<String> = Graphemes::new(given.as_bytes())
                .rev()
                .map(|cluster| cluster.to_string())
                .collect();
            got.reverse();
            assert_eq!(
                test.grapheme_clusters, got,
                "\n\ngrapheme reverse break test {} failed:\n\
                 given:    {:?}\n\
                 expected: {:?}\n\
                 got:      {:?}\n",
                i, uniescape(& given), uniescape_vec(& test.grapheme_clusters),
                uniescape_vec(& got),
            );
        }
    }
    #[test]
    fn forward_lossy() {
        for &(expected, input) in LOSSY_TESTS {
            let got = Graphemes::new(input.as_bytes()).collect::<String>();
            assert_eq!(expected, got);
        }
    }
    #[test]
    fn reverse_lossy() {
        for &(expected, input) in LOSSY_TESTS {
            let expected: String = expected.chars().rev().collect();
            let got = Graphemes::new(input.as_bytes()).rev().collect::<String>();
            assert_eq!(expected, got);
        }
    }
    fn uniescape(s: &str) -> String {
        s.chars().flat_map(|c| c.escape_unicode()).collect::<String>()
    }
    fn uniescape_vec(strs: &[String]) -> Vec<String> {
        strs.iter().map(|s| uniescape(s)).collect()
    }
    /// Return all of the UCD for grapheme breaks.
    fn ucdtests() -> Vec<GraphemeClusterBreakTest> {
        const TESTDATA: &'static str = include_str!("data/GraphemeBreakTest.txt");
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
mod tests_rug_314 {
    use super::*;
    use crate::unicode::grapheme::{decode_grapheme, GRAPHEME_BREAK_FWD};
    use crate::utf8;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_314_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"hello";
        let mut p0: &[u8] = rug_fuzz_0;
        decode_grapheme(p0);
        let _rug_ed_tests_rug_314_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_315 {
    use super::*;
    use crate::unicode::grapheme::{
        decode_last_grapheme, GRAPHEME_BREAK_REV, adjust_rev_for_regional_indicator, utf8,
    };
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_315_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"Hello World!";
        let p0: &[u8] = rug_fuzz_0;
        decode_last_grapheme(p0);
        let _rug_ed_tests_rug_315_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_316 {
    use super::*;
    use crate::unicode::grapheme::REGIONAL_INDICATOR_REV;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_316_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"\xf0\x9f\x87\xa9\xf0\x9f\x87\xaa";
        let rug_fuzz_1 = 4;
        let p0: &[u8] = rug_fuzz_0;
        let p1: usize = rug_fuzz_1;
        crate::unicode::grapheme::adjust_rev_for_regional_indicator(p0, p1);
        let _rug_ed_tests_rug_316_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_317 {
    use super::*;
    use unicode::grapheme::Graphemes;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_317_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"hello world";
        let p0: &[u8] = rug_fuzz_0;
        Graphemes::new(p0);
        let _rug_ed_tests_rug_317_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_318 {
    use super::*;
    use crate::ByteSlice;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_318_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"abc";
        let rug_fuzz_1 = b"bc";
        let rug_fuzz_2 = b"";
        let data = rug_fuzz_0;
        let mut it = data.graphemes();
        debug_assert_eq!(data, it.as_bytes());
        it.next();
        debug_assert_eq!(rug_fuzz_1, it.as_bytes());
        it.next();
        it.next();
        debug_assert_eq!(rug_fuzz_2, it.as_bytes());
        let _rug_ed_tests_rug_318_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_319 {
    use crate::unicode::grapheme::Graphemes;
    use crate::unicode::decode_grapheme;
    use crate::std::iter::Iterator;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_319_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"hello";
        let bs_sample: &[u8] = rug_fuzz_0;
        let mut p0 = Graphemes::<'_>::new(bs_sample);
        p0.next();
        let _rug_ed_tests_rug_319_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_321 {
    use super::*;
    use unicode::grapheme::GraphemeIndices;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_321_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"hello";
        let mut p0: &[u8] = rug_fuzz_0;
        let _ = GraphemeIndices::<'static>::new(p0);
        let _rug_ed_tests_rug_321_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_322 {
    use super::*;
    use crate::ByteSlice;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_322_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"abc";
        let s = rug_fuzz_0;
        let mut it = s.grapheme_indices();
        crate::unicode::grapheme::GraphemeIndices::<'_>::as_bytes(&it);
        let _rug_ed_tests_rug_322_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_323 {
    use super::*;
    use unicode::grapheme::GraphemeIndices;
    use std::iter::Iterator;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_323_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "héllô";
        let bs = rug_fuzz_0.as_bytes();
        let mut grapheme_indices = GraphemeIndices::<'_>::new(bs);
        let mut p0 = &mut grapheme_indices;
        p0.next();
        let _rug_ed_tests_rug_323_rrrruuuugggg_test_rug = 0;
    }
}
