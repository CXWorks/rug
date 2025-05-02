use std::mem;
use aho_corasick::{self, packed, AhoCorasick};
use memchr::{memchr, memchr2, memchr3, memmem};
use regex_syntax::hir::literal::{Literal, Seq};
/// A prefix extracted from a compiled regular expression.
///
/// A regex prefix is a set of literal strings that *must* be matched at the
/// beginning of a regex in order for the entire regex to match. Similarly
/// for a regex suffix.
#[derive(Clone, Debug)]
pub struct LiteralSearcher {
    complete: bool,
    lcp: Memmem,
    lcs: Memmem,
    matcher: Matcher,
}
#[derive(Clone, Debug)]
enum Matcher {
    /// No literals. (Never advances through the input.)
    Empty,
    /// A set of four or more single byte literals.
    Bytes(SingleByteSet),
    /// A single substring, using vector accelerated routines when available.
    Memmem(Memmem),
    /// An Aho-Corasick automaton.
    AC { ac: AhoCorasick, lits: Vec<Literal> },
    /// A packed multiple substring searcher, using SIMD.
    ///
    /// Note that Aho-Corasick will actually use this packed searcher
    /// internally automatically, however, there is some overhead associated
    /// with going through the Aho-Corasick machinery. So using the packed
    /// searcher directly results in some gains.
    Packed { s: packed::Searcher, lits: Vec<Literal> },
}
impl LiteralSearcher {
    /// Returns a matcher that never matches and never advances the input.
    pub fn empty() -> Self {
        Self::new(Seq::infinite(), Matcher::Empty)
    }
    /// Returns a matcher for literal prefixes from the given set.
    pub fn prefixes(lits: Seq) -> Self {
        let matcher = Matcher::prefixes(&lits);
        Self::new(lits, matcher)
    }
    /// Returns a matcher for literal suffixes from the given set.
    pub fn suffixes(lits: Seq) -> Self {
        let matcher = Matcher::suffixes(&lits);
        Self::new(lits, matcher)
    }
    fn new(lits: Seq, matcher: Matcher) -> Self {
        LiteralSearcher {
            complete: lits.is_exact(),
            lcp: Memmem::new(lits.longest_common_prefix().unwrap_or(b"")),
            lcs: Memmem::new(lits.longest_common_suffix().unwrap_or(b"")),
            matcher,
        }
    }
    /// Returns true if all matches comprise the entire regular expression.
    ///
    /// This does not necessarily mean that a literal match implies a match
    /// of the regular expression. For example, the regular expression `^a`
    /// is comprised of a single complete literal `a`, but the regular
    /// expression demands that it only match at the beginning of a string.
    pub fn complete(&self) -> bool {
        self.complete && !self.is_empty()
    }
    /// Find the position of a literal in `haystack` if it exists.
    #[cfg_attr(feature = "perf-inline", inline(always))]
    pub fn find(&self, haystack: &[u8]) -> Option<(usize, usize)> {
        use self::Matcher::*;
        match self.matcher {
            Empty => Some((0, 0)),
            Bytes(ref sset) => sset.find(haystack).map(|i| (i, i + 1)),
            Memmem(ref s) => s.find(haystack).map(|i| (i, i + s.len())),
            AC { ref ac, .. } => ac.find(haystack).map(|m| (m.start(), m.end())),
            Packed { ref s, .. } => s.find(haystack).map(|m| (m.start(), m.end())),
        }
    }
    /// Like find, except matches must start at index `0`.
    pub fn find_start(&self, haystack: &[u8]) -> Option<(usize, usize)> {
        for lit in self.iter() {
            if lit.len() > haystack.len() {
                continue;
            }
            if lit == &haystack[0..lit.len()] {
                return Some((0, lit.len()));
            }
        }
        None
    }
    /// Like find, except matches must end at index `haystack.len()`.
    pub fn find_end(&self, haystack: &[u8]) -> Option<(usize, usize)> {
        for lit in self.iter() {
            if lit.len() > haystack.len() {
                continue;
            }
            if lit == &haystack[haystack.len() - lit.len()..] {
                return Some((haystack.len() - lit.len(), haystack.len()));
            }
        }
        None
    }
    /// Returns an iterator over all literals to be matched.
    pub fn iter(&self) -> LiteralIter<'_> {
        match self.matcher {
            Matcher::Empty => LiteralIter::Empty,
            Matcher::Bytes(ref sset) => LiteralIter::Bytes(&sset.dense),
            Matcher::Memmem(ref s) => LiteralIter::Single(&s.finder.needle()),
            Matcher::AC { ref lits, .. } => LiteralIter::AC(lits),
            Matcher::Packed { ref lits, .. } => LiteralIter::Packed(lits),
        }
    }
    /// Returns a matcher for the longest common prefix of this matcher.
    pub fn lcp(&self) -> &Memmem {
        &self.lcp
    }
    /// Returns a matcher for the longest common suffix of this matcher.
    pub fn lcs(&self) -> &Memmem {
        &self.lcs
    }
    /// Returns true iff this prefix is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Returns the number of prefixes in this machine.
    pub fn len(&self) -> usize {
        use self::Matcher::*;
        match self.matcher {
            Empty => 0,
            Bytes(ref sset) => sset.dense.len(),
            Memmem(_) => 1,
            AC { ref ac, .. } => ac.patterns_len(),
            Packed { ref lits, .. } => lits.len(),
        }
    }
    /// Return the approximate heap usage of literals in bytes.
    pub fn approximate_size(&self) -> usize {
        use self::Matcher::*;
        match self.matcher {
            Empty => 0,
            Bytes(ref sset) => sset.approximate_size(),
            Memmem(ref single) => single.approximate_size(),
            AC { ref ac, .. } => ac.memory_usage(),
            Packed { ref s, .. } => s.memory_usage(),
        }
    }
}
impl Matcher {
    fn prefixes(lits: &Seq) -> Self {
        let sset = SingleByteSet::prefixes(lits);
        Matcher::new(lits, sset)
    }
    fn suffixes(lits: &Seq) -> Self {
        let sset = SingleByteSet::suffixes(lits);
        Matcher::new(lits, sset)
    }
    fn new(lits: &Seq, sset: SingleByteSet) -> Self {
        if lits.is_empty() || lits.min_literal_len() == Some(0) {
            return Matcher::Empty;
        }
        let lits = match lits.literals() {
            None => return Matcher::Empty,
            Some(members) => members,
        };
        if sset.dense.len() >= 26 {
            return Matcher::Empty;
        }
        if sset.complete {
            return Matcher::Bytes(sset);
        }
        if lits.len() == 1 {
            return Matcher::Memmem(Memmem::new(lits[0].as_bytes()));
        }
        let pats: Vec<&[u8]> = lits.iter().map(|lit| lit.as_bytes()).collect();
        let is_aho_corasick_fast = sset.dense.len() <= 1 && sset.all_ascii;
        if lits.len() <= 100 && !is_aho_corasick_fast {
            let mut builder = packed::Config::new()
                .match_kind(packed::MatchKind::LeftmostFirst)
                .builder();
            if let Some(s) = builder.extend(&pats).build() {
                return Matcher::Packed {
                    s,
                    lits: lits.to_owned(),
                };
            }
        }
        let ac = AhoCorasick::builder()
            .match_kind(aho_corasick::MatchKind::LeftmostFirst)
            .kind(Some(aho_corasick::AhoCorasickKind::DFA))
            .build(&pats)
            .unwrap();
        Matcher::AC {
            ac,
            lits: lits.to_owned(),
        }
    }
}
#[derive(Debug)]
pub enum LiteralIter<'a> {
    Empty,
    Bytes(&'a [u8]),
    Single(&'a [u8]),
    AC(&'a [Literal]),
    Packed(&'a [Literal]),
}
impl<'a> Iterator for LiteralIter<'a> {
    type Item = &'a [u8];
    fn next(&mut self) -> Option<Self::Item> {
        match *self {
            LiteralIter::Empty => None,
            LiteralIter::Bytes(ref mut many) => {
                if many.is_empty() {
                    None
                } else {
                    let next = &many[0..1];
                    *many = &many[1..];
                    Some(next)
                }
            }
            LiteralIter::Single(ref mut one) => {
                if one.is_empty() {
                    None
                } else {
                    let next = &one[..];
                    *one = &[];
                    Some(next)
                }
            }
            LiteralIter::AC(ref mut lits) => {
                if lits.is_empty() {
                    None
                } else {
                    let next = &lits[0];
                    *lits = &lits[1..];
                    Some(next.as_bytes())
                }
            }
            LiteralIter::Packed(ref mut lits) => {
                if lits.is_empty() {
                    None
                } else {
                    let next = &lits[0];
                    *lits = &lits[1..];
                    Some(next.as_bytes())
                }
            }
        }
    }
}
#[derive(Clone, Debug)]
struct SingleByteSet {
    sparse: Vec<bool>,
    dense: Vec<u8>,
    complete: bool,
    all_ascii: bool,
}
impl SingleByteSet {
    fn new() -> SingleByteSet {
        SingleByteSet {
            sparse: vec![false; 256],
            dense: vec![],
            complete: true,
            all_ascii: true,
        }
    }
    fn prefixes(lits: &Seq) -> SingleByteSet {
        let mut sset = SingleByteSet::new();
        let lits = match lits.literals() {
            None => return sset,
            Some(lits) => lits,
        };
        for lit in lits.iter() {
            sset.complete = sset.complete && lit.len() == 1;
            if let Some(&b) = lit.as_bytes().get(0) {
                if !sset.sparse[b as usize] {
                    if b > 0x7F {
                        sset.all_ascii = false;
                    }
                    sset.dense.push(b);
                    sset.sparse[b as usize] = true;
                }
            }
        }
        sset
    }
    fn suffixes(lits: &Seq) -> SingleByteSet {
        let mut sset = SingleByteSet::new();
        let lits = match lits.literals() {
            None => return sset,
            Some(lits) => lits,
        };
        for lit in lits.iter() {
            sset.complete = sset.complete && lit.len() == 1;
            if let Some(&b) = lit.as_bytes().last() {
                if !sset.sparse[b as usize] {
                    if b > 0x7F {
                        sset.all_ascii = false;
                    }
                    sset.dense.push(b);
                    sset.sparse[b as usize] = true;
                }
            }
        }
        sset
    }
    /// Faster find that special cases certain sizes to use memchr.
    #[cfg_attr(feature = "perf-inline", inline(always))]
    fn find(&self, text: &[u8]) -> Option<usize> {
        match self.dense.len() {
            0 => None,
            1 => memchr(self.dense[0], text),
            2 => memchr2(self.dense[0], self.dense[1], text),
            3 => memchr3(self.dense[0], self.dense[1], self.dense[2], text),
            _ => self._find(text),
        }
    }
    /// Generic find that works on any sized set.
    fn _find(&self, haystack: &[u8]) -> Option<usize> {
        for (i, &b) in haystack.iter().enumerate() {
            if self.sparse[b as usize] {
                return Some(i);
            }
        }
        None
    }
    fn approximate_size(&self) -> usize {
        (self.dense.len() * mem::size_of::<u8>())
            + (self.sparse.len() * mem::size_of::<bool>())
    }
}
/// A simple wrapper around the memchr crate's memmem implementation.
///
/// The API this exposes mirrors the API of previous substring searchers that
/// this supplanted.
#[derive(Clone, Debug)]
pub struct Memmem {
    finder: memmem::Finder<'static>,
    char_len: usize,
}
impl Memmem {
    fn new(pat: &[u8]) -> Memmem {
        Memmem {
            finder: memmem::Finder::new(pat).into_owned(),
            char_len: char_len_lossy(pat),
        }
    }
    #[cfg_attr(feature = "perf-inline", inline(always))]
    pub fn find(&self, haystack: &[u8]) -> Option<usize> {
        self.finder.find(haystack)
    }
    #[cfg_attr(feature = "perf-inline", inline(always))]
    pub fn is_suffix(&self, text: &[u8]) -> bool {
        if text.len() < self.len() {
            return false;
        }
        &text[text.len() - self.len()..] == self.finder.needle()
    }
    pub fn len(&self) -> usize {
        self.finder.needle().len()
    }
    pub fn char_len(&self) -> usize {
        self.char_len
    }
    fn approximate_size(&self) -> usize {
        self.finder.needle().len() * mem::size_of::<u8>()
    }
}
fn char_len_lossy(bytes: &[u8]) -> usize {
    String::from_utf8_lossy(bytes).chars().count()
}
#[cfg(test)]
mod tests_rug_212 {
    use super::*;
    use crate::literal::imp::char_len_lossy;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_212_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"hello, world!";
        let p0: &[u8] = rug_fuzz_0;
        char_len_lossy(p0);
        let _rug_ed_tests_rug_212_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_213 {
    use super::*;
    use crate::literal::imp::{LiteralSearcher, Seq, Matcher};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_213_rrrruuuugggg_test_rug = 0;
        let _ = LiteralSearcher::empty();
        let _rug_ed_tests_rug_213_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_217 {
    use super::*;
    use crate::literal::imp::*;
    #[test]
    fn test_complete() {
        let _rug_st_tests_rug_217_rrrruuuugggg_test_complete = 0;
        let rug_fuzz_0 = false;
        let rug_fuzz_1 = b"abc";
        let rug_fuzz_2 = b"def";
        let rug_fuzz_3 = true;
        let rug_fuzz_4 = b"";
        let rug_fuzz_5 = b"";
        let p0 = LiteralSearcher::prefixes(Seq::infinite());
        debug_assert_eq!(p0.complete(), false);
        let p1 = LiteralSearcher::empty();
        debug_assert_eq!(p1.complete(), false);
        let p2 = LiteralSearcher::suffixes(Seq::empty());
        debug_assert_eq!(p2.complete(), false);
        let p3 = LiteralSearcher {
            complete: rug_fuzz_0,
            lcp: Memmem::new(rug_fuzz_1),
            lcs: Memmem::new(rug_fuzz_2),
            matcher: Matcher::Empty,
        };
        debug_assert_eq!(p3.complete(), false);
        let p4 = LiteralSearcher {
            complete: rug_fuzz_3,
            lcp: Memmem::new(rug_fuzz_4),
            lcs: Memmem::new(rug_fuzz_5),
            matcher: Matcher::Empty,
        };
        debug_assert_eq!(p4.complete(), false);
        let _rug_ed_tests_rug_217_rrrruuuugggg_test_complete = 0;
    }
}
#[cfg(test)]
mod tests_rug_218 {
    use super::*;
    use crate::literal::imp::{LiteralSearcher, Matcher};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_218_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"haystack";
        let mut p0 = LiteralSearcher::empty();
        let p1 = rug_fuzz_0;
        p0.find(p1);
        let _rug_ed_tests_rug_218_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_219 {
    use super::*;
    use crate::literal::imp::LiteralSearcher;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_219_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"example haystack";
        let lit_searcher: LiteralSearcher = LiteralSearcher::empty();
        let haystack: &[u8] = rug_fuzz_0;
        lit_searcher.find_start(haystack);
        let _rug_ed_tests_rug_219_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_220 {
    use super::*;
    use crate::literal::imp::LiteralSearcher;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_220_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"Example haystack data";
        let mut p0 = LiteralSearcher::empty();
        let mut p1: &[u8] = rug_fuzz_0;
        p0.find_end(p1);
        let _rug_ed_tests_rug_220_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_221 {
    use super::*;
    use crate::{literal, internal};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_221_rrrruuuugggg_test_rug = 0;
        let mut p0 = internal::LiteralSearcher::empty();
        literal::imp::LiteralSearcher::iter(&p0);
        let _rug_ed_tests_rug_221_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_222 {
    use super::*;
    use crate::literal::imp::LiteralSearcher;
    use crate::internal::LiteralSearcher as InternalLiteralSearcher;
    #[test]
    fn test_lcp() {
        let _rug_st_tests_rug_222_rrrruuuugggg_test_lcp = 0;
        let mut p0: InternalLiteralSearcher = InternalLiteralSearcher::empty();
        LiteralSearcher::lcp(&p0);
        let _rug_ed_tests_rug_222_rrrruuuugggg_test_lcp = 0;
    }
}
#[cfg(test)]
mod tests_rug_223 {
    use super::*;
    use crate::internal;
    use crate::literal::imp::LiteralSearcher;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_223_rrrruuuugggg_test_rug = 0;
        let mut p0: LiteralSearcher = internal::LiteralSearcher::empty();
        let result = p0.lcs();
        let _rug_ed_tests_rug_223_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_224 {
    use super::*;
    use crate::literal::imp::LiteralSearcher;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_224_rrrruuuugggg_test_rug = 0;
        let mut p0 = LiteralSearcher::empty();
        debug_assert_eq!(p0.is_empty(), true);
        let _rug_ed_tests_rug_224_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_225 {
    use super::*;
    use crate::literal::imp::LiteralSearcher;
    #[test]
    fn test_len() {
        let _rug_st_tests_rug_225_rrrruuuugggg_test_len = 0;
        let p0 = LiteralSearcher::empty();
        debug_assert_eq!(LiteralSearcher::len(& p0), 0);
        let _rug_ed_tests_rug_225_rrrruuuugggg_test_len = 0;
    }
}
#[cfg(test)]
mod tests_rug_226 {
    use super::*;
    use crate::internal::LiteralSearcher;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_226_rrrruuuugggg_test_rug = 0;
        let p0 = LiteralSearcher::empty();
        LiteralSearcher::approximate_size(&p0);
        let _rug_ed_tests_rug_226_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_227 {
    use super::*;
    use regex_syntax::hir::literal::Seq;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_227_rrrruuuugggg_test_rug = 0;
        let mut p0: Seq = Seq::empty();
        crate::literal::imp::Matcher::prefixes(&p0);
        let _rug_ed_tests_rug_227_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_229 {
    use super::*;
    use regex_syntax::hir::literal::Seq;
    use crate::literal::imp::{SingleByteSet, Matcher, Memmem};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let literals: Vec<&str> = vec![rug_fuzz_0, "def", "ghi"];
        let seq = Seq::new(literals);
        use crate::literal::imp::SingleByteSet;
        let mut sset = SingleByteSet::new();
        let mut p0 = &seq;
        let mut p1 = sset;
        Matcher::new(p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_231 {
    use super::*;
    use crate::literal::imp::SingleByteSet;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_231_rrrruuuugggg_test_rug = 0;
        SingleByteSet::new();
        let _rug_ed_tests_rug_231_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_234 {
    use super::*;
    use crate::literal::imp::SingleByteSet;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_234_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"sample_text";
        let mut v59 = SingleByteSet::new();
        let mut p1 = rug_fuzz_0;
        SingleByteSet::find(&v59, p1);
        let _rug_ed_tests_rug_234_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_235 {
    use super::*;
    use crate::literal::imp::SingleByteSet;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_235_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"needle";
        let mut v59 = SingleByteSet::new();
        let hay = rug_fuzz_0;
        crate::literal::imp::SingleByteSet::_find(&v59, hay);
        let _rug_ed_tests_rug_235_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_236 {
    use super::*;
    use crate::literal::imp::SingleByteSet;
    use std::mem;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_236_rrrruuuugggg_test_rug = 0;
        let mut p0 = SingleByteSet::new();
        debug_assert_eq!(p0.approximate_size(), 0);
        let _rug_ed_tests_rug_236_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_237 {
    use super::*;
    use crate::literal::imp::Memmem;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_237_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"sample_data";
        let mut p0: &[u8] = rug_fuzz_0;
        Memmem::new(p0);
        let _rug_ed_tests_rug_237_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_238 {
    use super::*;
    use crate::literal::imp::Memmem;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_238_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"sample";
        let rug_fuzz_1 = b"haystack";
        let mut p0 = Memmem::new(rug_fuzz_0);
        let p1: &[u8] = rug_fuzz_1;
        Memmem::find(&p0, p1);
        let _rug_ed_tests_rug_238_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_239_prepare {
    use crate::literal::Memmem;
    #[test]
    fn sample() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0_ext)) = <([u8; 6]) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_239 {
    use super::*;
    use crate::literal::Memmem;
    #[test]
    fn test_is_suffix() {
        let _rug_st_tests_rug_239_rrrruuuugggg_test_is_suffix = 0;
        let rug_fuzz_0 = b"sample";
        let rug_fuzz_1 = b"example";
        let mut p0 = Memmem::new(rug_fuzz_0);
        let p1: &[u8] = rug_fuzz_1;
        p0.is_suffix(p1);
        let _rug_ed_tests_rug_239_rrrruuuugggg_test_is_suffix = 0;
    }
}
#[cfg(test)]
mod tests_rug_240 {
    use super::*;
    use crate::literal::imp::Memmem;
    #[test]
    fn test_len() {
        let _rug_st_tests_rug_240_rrrruuuugggg_test_len = 0;
        let rug_fuzz_0 = b"sample";
        let mut p0 = Memmem::new(rug_fuzz_0);
        Memmem::len(&p0);
        let _rug_ed_tests_rug_240_rrrruuuugggg_test_len = 0;
    }
}
#[cfg(test)]
mod tests_rug_241 {
    use super::*;
    use crate::literal::imp::Memmem;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_241_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"sample";
        let mut v61 = Memmem::new(rug_fuzz_0);
        let p0 = &v61;
        crate::literal::imp::Memmem::char_len(p0);
        let _rug_ed_tests_rug_241_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_242 {
    use super::*;
    use crate::literal::Memmem;
    use std::mem;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_242_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"sample";
        let mut p0 = Memmem::new(rug_fuzz_0);
        crate::literal::imp::Memmem::approximate_size(&p0);
        let _rug_ed_tests_rug_242_rrrruuuugggg_test_rug = 0;
    }
}
