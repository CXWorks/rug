use std::cmp;
use std::mem;
use aho_corasick::{self, packed, AhoCorasick, AhoCorasickBuilder};
use memchr::{memchr, memchr2, memchr3};
use syntax::hir::literal::{Literal, Literals};
use freqs::BYTE_FREQUENCIES;
/// A prefix extracted from a compiled regular expression.
///
/// A regex prefix is a set of literal strings that *must* be matched at the
/// beginning of a regex in order for the entire regex to match. Similarly
/// for a regex suffix.
#[derive(Clone, Debug)]
pub struct LiteralSearcher {
    complete: bool,
    lcp: FreqyPacked,
    lcs: FreqyPacked,
    matcher: Matcher,
}
#[derive(Clone, Debug)]
enum Matcher {
    /// No literals. (Never advances through the input.)
    Empty,
    /// A set of four or more single byte literals.
    Bytes(SingleByteSet),
    /// A single substring, find using memchr and frequency analysis.
    FreqyPacked(FreqyPacked),
    /// A single substring, find using Boyer-Moore.
    BoyerMoore(BoyerMooreSearch),
    /// An Aho-Corasick automaton.
    AC { ac: AhoCorasick<u32>, lits: Vec<Literal> },
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
        Self::new(Literals::empty(), Matcher::Empty)
    }
    /// Returns a matcher for literal prefixes from the given set.
    pub fn prefixes(lits: Literals) -> Self {
        let matcher = Matcher::prefixes(&lits);
        Self::new(lits, matcher)
    }
    /// Returns a matcher for literal suffixes from the given set.
    pub fn suffixes(lits: Literals) -> Self {
        let matcher = Matcher::suffixes(&lits);
        Self::new(lits, matcher)
    }
    fn new(lits: Literals, matcher: Matcher) -> Self {
        let complete = lits.all_complete();
        LiteralSearcher {
            complete: complete,
            lcp: FreqyPacked::new(lits.longest_common_prefix().to_vec()),
            lcs: FreqyPacked::new(lits.longest_common_suffix().to_vec()),
            matcher: matcher,
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
            FreqyPacked(ref s) => s.find(haystack).map(|i| (i, i + s.len())),
            BoyerMoore(ref s) => s.find(haystack).map(|i| (i, i + s.len())),
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
    pub fn iter(&self) -> LiteralIter {
        match self.matcher {
            Matcher::Empty => LiteralIter::Empty,
            Matcher::Bytes(ref sset) => LiteralIter::Bytes(&sset.dense),
            Matcher::FreqyPacked(ref s) => LiteralIter::Single(&s.pat),
            Matcher::BoyerMoore(ref s) => LiteralIter::Single(&s.pattern),
            Matcher::AC { ref lits, .. } => LiteralIter::AC(lits),
            Matcher::Packed { ref lits, .. } => LiteralIter::Packed(lits),
        }
    }
    /// Returns a matcher for the longest common prefix of this matcher.
    pub fn lcp(&self) -> &FreqyPacked {
        &self.lcp
    }
    /// Returns a matcher for the longest common suffix of this matcher.
    pub fn lcs(&self) -> &FreqyPacked {
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
            FreqyPacked(_) => 1,
            BoyerMoore(_) => 1,
            AC { ref ac, .. } => ac.pattern_count(),
            Packed { ref lits, .. } => lits.len(),
        }
    }
    /// Return the approximate heap usage of literals in bytes.
    pub fn approximate_size(&self) -> usize {
        use self::Matcher::*;
        match self.matcher {
            Empty => 0,
            Bytes(ref sset) => sset.approximate_size(),
            FreqyPacked(ref single) => single.approximate_size(),
            BoyerMoore(ref single) => single.approximate_size(),
            AC { ref ac, .. } => ac.heap_bytes(),
            Packed { ref s, .. } => s.heap_bytes(),
        }
    }
}
impl Matcher {
    fn prefixes(lits: &Literals) -> Self {
        let sset = SingleByteSet::prefixes(lits);
        Matcher::new(lits, sset)
    }
    fn suffixes(lits: &Literals) -> Self {
        let sset = SingleByteSet::suffixes(lits);
        Matcher::new(lits, sset)
    }
    fn new(lits: &Literals, sset: SingleByteSet) -> Self {
        if lits.literals().is_empty() {
            return Matcher::Empty;
        }
        if sset.dense.len() >= 26 {
            return Matcher::Empty;
        }
        if sset.complete {
            return Matcher::Bytes(sset);
        }
        if lits.literals().len() == 1 {
            let lit = lits.literals()[0].to_vec();
            if BoyerMooreSearch::should_use(lit.as_slice()) {
                return Matcher::BoyerMoore(BoyerMooreSearch::new(lit));
            } else {
                return Matcher::FreqyPacked(FreqyPacked::new(lit));
            }
        }
        let pats = lits.literals().to_owned();
        let is_aho_corasick_fast = sset.dense.len() <= 1 && sset.all_ascii;
        if lits.literals().len() <= 100 && !is_aho_corasick_fast {
            let mut builder = packed::Config::new()
                .match_kind(packed::MatchKind::LeftmostFirst)
                .builder();
            if let Some(s) = builder.extend(&pats).build() {
                return Matcher::Packed { s, lits: pats };
            }
        }
        let ac = AhoCorasickBuilder::new()
            .match_kind(aho_corasick::MatchKind::LeftmostFirst)
            .dfa(true)
            .build_with_size::<u32, _, _>(&pats)
            .unwrap();
        Matcher::AC { ac, lits: pats }
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
                    Some(&**next)
                }
            }
            LiteralIter::Packed(ref mut lits) => {
                if lits.is_empty() {
                    None
                } else {
                    let next = &lits[0];
                    *lits = &lits[1..];
                    Some(&**next)
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
    fn prefixes(lits: &Literals) -> SingleByteSet {
        let mut sset = SingleByteSet::new();
        for lit in lits.literals() {
            sset.complete = sset.complete && lit.len() == 1;
            if let Some(&b) = lit.get(0) {
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
    fn suffixes(lits: &Literals) -> SingleByteSet {
        let mut sset = SingleByteSet::new();
        for lit in lits.literals() {
            sset.complete = sset.complete && lit.len() == 1;
            if let Some(&b) = lit.get(lit.len().checked_sub(1).unwrap()) {
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
/// Provides an implementation of fast subtring search using frequency
/// analysis.
///
/// memchr is so fast that we do everything we can to keep the loop in memchr
/// for as long as possible. The easiest way to do this is to intelligently
/// pick the byte to send to memchr. The best byte is the byte that occurs
/// least frequently in the haystack. Since doing frequency analysis on the
/// haystack is far too expensive, we compute a set of fixed frequencies up
/// front and hard code them in src/freqs.rs. Frequency analysis is done via
/// scripts/frequencies.py.
#[derive(Clone, Debug)]
pub struct FreqyPacked {
    /// The pattern.
    pat: Vec<u8>,
    /// The number of Unicode characters in the pattern. This is useful for
    /// determining the effective length of a pattern when deciding which
    /// optimizations to perform. A trailing incomplete UTF-8 sequence counts
    /// as one character.
    char_len: usize,
    /// The rarest byte in the pattern, according to pre-computed frequency
    /// analysis.
    rare1: u8,
    /// The offset of the rarest byte in `pat`.
    rare1i: usize,
    /// The second rarest byte in the pattern, according to pre-computed
    /// frequency analysis. (This may be equivalent to the rarest byte.)
    ///
    /// The second rarest byte is used as a type of guard for quickly detecting
    /// a mismatch after memchr locates an instance of the rarest byte. This
    /// is a hedge against pathological cases where the pre-computed frequency
    /// analysis may be off. (But of course, does not prevent *all*
    /// pathological cases.)
    rare2: u8,
    /// The offset of the second rarest byte in `pat`.
    rare2i: usize,
}
impl FreqyPacked {
    fn new(pat: Vec<u8>) -> FreqyPacked {
        if pat.is_empty() {
            return FreqyPacked::empty();
        }
        let mut rare1 = pat[0];
        let mut rare2 = pat[0];
        for b in pat[1..].iter().cloned() {
            if freq_rank(b) < freq_rank(rare1) {
                rare1 = b;
            }
        }
        for &b in &pat {
            if rare1 == rare2 {
                rare2 = b
            } else if b != rare1 && freq_rank(b) < freq_rank(rare2) {
                rare2 = b;
            }
        }
        let rare1i = pat.iter().rposition(|&b| b == rare1).unwrap();
        let rare2i = pat.iter().rposition(|&b| b == rare2).unwrap();
        let char_len = char_len_lossy(&pat);
        FreqyPacked {
            pat: pat,
            char_len: char_len,
            rare1: rare1,
            rare1i: rare1i,
            rare2: rare2,
            rare2i: rare2i,
        }
    }
    fn empty() -> FreqyPacked {
        FreqyPacked {
            pat: vec![],
            char_len: 0,
            rare1: 0,
            rare1i: 0,
            rare2: 0,
            rare2i: 0,
        }
    }
    #[cfg_attr(feature = "perf-inline", inline(always))]
    pub fn find(&self, haystack: &[u8]) -> Option<usize> {
        let pat = &*self.pat;
        if haystack.len() < pat.len() || pat.is_empty() {
            return None;
        }
        let mut i = self.rare1i;
        while i < haystack.len() {
            i
                += match memchr(self.rare1, &haystack[i..]) {
                    None => return None,
                    Some(i) => i,
                };
            let start = i - self.rare1i;
            let end = start + pat.len();
            if end > haystack.len() {
                return None;
            }
            let aligned = &haystack[start..end];
            if aligned[self.rare2i] == self.rare2 && aligned == &*self.pat {
                return Some(start);
            }
            i += 1;
        }
        None
    }
    #[cfg_attr(feature = "perf-inline", inline(always))]
    pub fn is_suffix(&self, text: &[u8]) -> bool {
        if text.len() < self.len() {
            return false;
        }
        text[text.len() - self.len()..] == *self.pat
    }
    pub fn len(&self) -> usize {
        self.pat.len()
    }
    pub fn char_len(&self) -> usize {
        self.char_len
    }
    fn approximate_size(&self) -> usize {
        self.pat.len() * mem::size_of::<u8>()
    }
}
fn char_len_lossy(bytes: &[u8]) -> usize {
    String::from_utf8_lossy(bytes).chars().count()
}
/// An implementation of Tuned Boyer-Moore as laid out by
/// Andrew Hume and Daniel Sunday in "Fast String Searching".
/// O(n) in the size of the input.
///
/// Fast string searching algorithms come in many variations,
/// but they can generally be described in terms of three main
/// components.
///
/// The skip loop is where the string searcher wants to spend
/// as much time as possible. Exactly which character in the
/// pattern the skip loop examines varies from algorithm to
/// algorithm, but in the simplest case this loop repeated
/// looks at the last character in the pattern and jumps
/// forward in the input if it is not in the pattern.
/// Robert Boyer and J Moore called this the "fast" loop in
/// their original paper.
///
/// The match loop is responsible for actually examining the
/// whole potentially matching substring. In order to fail
/// faster, the match loop sometimes has a guard test attached.
/// The guard test uses frequency analysis of the different
/// characters in the pattern to choose the least frequency
/// occurring character and use it to find match failures
/// as quickly as possible.
///
/// The shift rule governs how the algorithm will shuffle its
/// test window in the event of a failure during the match loop.
/// Certain shift rules allow the worst-case run time of the
/// algorithm to be shown to be O(n) in the size of the input
/// rather than O(nm) in the size of the input and the size
/// of the pattern (as naive Boyer-Moore is).
///
/// "Fast String Searching", in addition to presenting a tuned
/// algorithm, provides a comprehensive taxonomy of the many
/// different flavors of string searchers. Under that taxonomy
/// TBM, the algorithm implemented here, uses an unrolled fast
/// skip loop with memchr fallback, a forward match loop with guard,
/// and the mini Sunday's delta shift rule. To unpack that you'll have to
/// read the paper.
#[derive(Clone, Debug)]
pub struct BoyerMooreSearch {
    /// The pattern we are going to look for in the haystack.
    pattern: Vec<u8>,
    /// The skip table for the skip loop.
    ///
    /// Maps the character at the end of the input
    /// to a shift.
    skip_table: Vec<usize>,
    /// The guard character (least frequently occurring char).
    guard: u8,
    /// The reverse-index of the guard character in the pattern.
    guard_reverse_idx: usize,
    /// Daniel Sunday's mini generalized delta2 shift table.
    ///
    /// We use a skip loop, so we only have to provide a shift
    /// for the skip char (last char). This is why it is a mini
    /// shift rule.
    md2_shift: usize,
}
impl BoyerMooreSearch {
    /// Create a new string searcher, performing whatever
    /// compilation steps are required.
    fn new(pattern: Vec<u8>) -> Self {
        debug_assert!(! pattern.is_empty());
        let (g, gi) = Self::select_guard(pattern.as_slice());
        let skip_table = Self::compile_skip_table(pattern.as_slice());
        let md2_shift = Self::compile_md2_shift(pattern.as_slice());
        BoyerMooreSearch {
            pattern: pattern,
            skip_table: skip_table,
            guard: g,
            guard_reverse_idx: gi,
            md2_shift: md2_shift,
        }
    }
    /// Find the pattern in `haystack`, returning the offset
    /// of the start of the first occurrence of the pattern
    /// in `haystack`.
    #[inline]
    fn find(&self, haystack: &[u8]) -> Option<usize> {
        if haystack.len() < self.pattern.len() {
            return None;
        }
        let mut window_end = self.pattern.len() - 1;
        const NUM_UNROLL: usize = 10;
        let short_circut = (NUM_UNROLL + 2) * self.pattern.len();
        if haystack.len() > short_circut {
            let backstop = haystack.len() - ((NUM_UNROLL + 1) * self.pattern.len());
            loop {
                window_end = match self.skip_loop(haystack, window_end, backstop) {
                    Some(i) => i,
                    None => return None,
                };
                if window_end >= backstop {
                    break;
                }
                if self.check_match(haystack, window_end) {
                    return Some(window_end - (self.pattern.len() - 1));
                } else {
                    let skip = self.skip_table[haystack[window_end] as usize];
                    window_end += if skip == 0 { self.md2_shift } else { skip };
                    continue;
                }
            }
        }
        while window_end < haystack.len() {
            let mut skip = self.skip_table[haystack[window_end] as usize];
            if skip == 0 {
                if self.check_match(haystack, window_end) {
                    return Some(window_end - (self.pattern.len() - 1));
                } else {
                    skip = self.md2_shift;
                }
            }
            window_end += skip;
        }
        None
    }
    fn len(&self) -> usize {
        return self.pattern.len();
    }
    /// The key heuristic behind which the BoyerMooreSearch lives.
    ///
    /// See `rust-lang/regex/issues/408`.
    ///
    /// Tuned Boyer-Moore is actually pretty slow! It turns out a handrolled
    /// platform-specific memchr routine with a bit of frequency
    /// analysis sprinkled on top actually wins most of the time.
    /// However, there are a few cases where Tuned Boyer-Moore still
    /// wins.
    ///
    /// If the haystack is random, frequency analysis doesn't help us,
    /// so Boyer-Moore will win for sufficiently large needles.
    /// Unfortunately, there is no obvious way to determine this
    /// ahead of time.
    ///
    /// If the pattern itself consists of very common characters,
    /// frequency analysis won't get us anywhere. The most extreme
    /// example of this is a pattern like `eeeeeeeeeeeeeeee`. Fortunately,
    /// this case is wholly determined by the pattern, so we can actually
    /// implement the heuristic.
    ///
    /// A third case is if the pattern is sufficiently long. The idea
    /// here is that once the pattern gets long enough the Tuned
    /// Boyer-Moore skip loop will start making strides long enough
    /// to beat the asm deep magic that is memchr.
    fn should_use(pattern: &[u8]) -> bool {
        const MIN_LEN: usize = 9;
        const MIN_CUTOFF: usize = 150;
        const MAX_CUTOFF: usize = 255;
        const LEN_CUTOFF_PROPORTION: usize = 4;
        let scaled_rank = pattern.len().wrapping_mul(LEN_CUTOFF_PROPORTION);
        let cutoff = cmp::max(
            MIN_CUTOFF,
            MAX_CUTOFF - cmp::min(MAX_CUTOFF, scaled_rank),
        );
        pattern.len() > MIN_LEN && pattern.iter().all(|c| freq_rank(*c) >= cutoff)
    }
    /// Check to see if there is a match at the given position
    #[inline]
    fn check_match(&self, haystack: &[u8], window_end: usize) -> bool {
        if haystack[window_end - self.guard_reverse_idx] != self.guard {
            return false;
        }
        let window_start = window_end - (self.pattern.len() - 1);
        for i in 0..self.pattern.len() {
            if self.pattern[i] != haystack[window_start + i] {
                return false;
            }
        }
        true
    }
    /// Skip forward according to the shift table.
    ///
    /// Returns the offset of the next occurrence
    /// of the last char in the pattern, or the none
    /// if it never reappears. If `skip_loop` hits the backstop
    /// it will leave early.
    #[inline]
    fn skip_loop(
        &self,
        haystack: &[u8],
        mut window_end: usize,
        backstop: usize,
    ) -> Option<usize> {
        let window_end_snapshot = window_end;
        let skip_of = |we: usize| -> usize { self.skip_table[haystack[we] as usize] };
        loop {
            let mut skip = skip_of(window_end);
            window_end += skip;
            skip = skip_of(window_end);
            window_end += skip;
            if skip != 0 {
                skip = skip_of(window_end);
                window_end += skip;
                skip = skip_of(window_end);
                window_end += skip;
                skip = skip_of(window_end);
                window_end += skip;
                if skip != 0 {
                    skip = skip_of(window_end);
                    window_end += skip;
                    skip = skip_of(window_end);
                    window_end += skip;
                    skip = skip_of(window_end);
                    window_end += skip;
                    if skip != 0 {
                        skip = skip_of(window_end);
                        window_end += skip;
                        skip = skip_of(window_end);
                        window_end += skip;
                        if window_end - window_end_snapshot
                            > 16 * mem::size_of::<usize>()
                        {
                            if window_end >= backstop {
                                return Some(window_end);
                            }
                            continue;
                        } else {
                            window_end = window_end
                                .checked_sub(1 + self.guard_reverse_idx)
                                .unwrap_or(0);
                            match memchr(self.guard, &haystack[window_end..]) {
                                None => return None,
                                Some(g_idx) => {
                                    return Some(window_end + g_idx + self.guard_reverse_idx);
                                }
                            }
                        }
                    }
                }
            }
            return Some(window_end);
        }
    }
    /// Compute the ufast skip table.
    fn compile_skip_table(pattern: &[u8]) -> Vec<usize> {
        let mut tab = vec![pattern.len(); 256];
        for (i, c) in pattern.iter().enumerate() {
            tab[*c as usize] = (pattern.len() - 1) - i;
        }
        tab
    }
    /// Select the guard character based off of the precomputed
    /// frequency table.
    fn select_guard(pattern: &[u8]) -> (u8, usize) {
        let mut rarest = pattern[0];
        let mut rarest_rev_idx = pattern.len() - 1;
        for (i, c) in pattern.iter().enumerate() {
            if freq_rank(*c) < freq_rank(rarest) {
                rarest = *c;
                rarest_rev_idx = (pattern.len() - 1) - i;
            }
        }
        (rarest, rarest_rev_idx)
    }
    /// If there is another occurrence of the skip
    /// char, shift to it, otherwise just shift to
    /// the next window.
    fn compile_md2_shift(pattern: &[u8]) -> usize {
        let shiftc = *pattern.last().unwrap();
        if pattern.len() == 1 {
            return 0xDEADBEAF;
        }
        let mut i = pattern.len() - 2;
        while i > 0 {
            if pattern[i] == shiftc {
                return (pattern.len() - 1) - i;
            }
            i -= 1;
        }
        pattern.len() - 1
    }
    fn approximate_size(&self) -> usize {
        (self.pattern.len() * mem::size_of::<u8>()) + (256 * mem::size_of::<usize>())
    }
}
fn freq_rank(b: u8) -> usize {
    BYTE_FREQUENCIES[b as usize] as usize
}
#[cfg(test)]
mod tests {
    use super::{BoyerMooreSearch, FreqyPacked};
    #[test]
    fn bm_find_subs() {
        let searcher = BoyerMooreSearch::new(Vec::from(&b"pattern"[..]));
        let haystack = b"I keep seeing patterns in this text";
        assert_eq!(14, searcher.find(haystack).unwrap());
    }
    #[test]
    fn bm_find_no_subs() {
        let searcher = BoyerMooreSearch::new(Vec::from(&b"pattern"[..]));
        let haystack = b"I keep seeing needles in this text";
        assert_eq!(None, searcher.find(haystack));
    }
    #[test]
    fn bm_skip_reset_bug() {
        let haystack = vec![0, 0, 0, 0, 0, 1, 1, 0];
        let needle = vec![0, 1, 1, 0];
        let searcher = BoyerMooreSearch::new(needle);
        let offset = searcher.find(haystack.as_slice()).unwrap();
        assert_eq!(4, offset);
    }
    #[test]
    fn bm_backstop_underflow_bug() {
        let haystack = vec![0, 0];
        let needle = vec![0, 0];
        let searcher = BoyerMooreSearch::new(needle);
        let offset = searcher.find(haystack.as_slice()).unwrap();
        assert_eq!(0, offset);
    }
    #[test]
    fn bm_naive_off_by_one_bug() {
        let haystack = vec![91];
        let needle = vec![91];
        let naive_offset = naive_find(&needle, &haystack).unwrap();
        assert_eq!(0, naive_offset);
    }
    #[test]
    fn bm_memchr_fallback_indexing_bug() {
        let mut haystack = vec![
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 87, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let needle = vec![1, 1, 1, 1, 32, 32, 87];
        let needle_start = haystack.len();
        haystack.extend(needle.clone());
        let searcher = BoyerMooreSearch::new(needle);
        assert_eq!(needle_start, searcher.find(haystack.as_slice()).unwrap());
    }
    #[test]
    fn bm_backstop_boundary() {
        let haystack = b"\
// aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
e_data.clone_created(entity_id, entity_to_add.entity_id);
aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
"
            .to_vec();
        let needle = b"clone_created".to_vec();
        let searcher = BoyerMooreSearch::new(needle);
        let result = searcher.find(&haystack);
        assert_eq!(Some(43), result);
    }
    #[test]
    fn bm_win_gnu_indexing_bug() {
        let haystack_raw = vec![
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0,
        ];
        let needle = vec![1, 1, 1, 1, 1, 1, 1];
        let haystack = haystack_raw.as_slice();
        BoyerMooreSearch::new(needle.clone()).find(haystack);
    }
    use quickcheck::TestResult;
    fn naive_find(needle: &[u8], haystack: &[u8]) -> Option<usize> {
        assert!(needle.len() <= haystack.len());
        for i in 0..(haystack.len() - (needle.len() - 1)) {
            if haystack[i] == needle[0] && &haystack[i..(i + needle.len())] == needle {
                return Some(i);
            }
        }
        None
    }
    quickcheck! {
        fn qc_bm_equals_nieve_find(pile1 : Vec < u8 >, pile2 : Vec < u8 >) -> TestResult
        { if pile1.len() == 0 || pile2.len() == 0 { return TestResult::discard(); } let
        (needle, haystack) = if pile1.len() < pile2.len() { (pile1, pile2.as_slice()) }
        else { (pile2, pile1.as_slice()) }; let searcher = BoyerMooreSearch::new(needle
        .clone()); TestResult::from_bool(searcher.find(haystack) == naive_find(& needle,
        haystack)) } fn qc_bm_equals_single(pile1 : Vec < u8 >, pile2 : Vec < u8 >) ->
        TestResult { if pile1.len() == 0 || pile2.len() == 0 { return
        TestResult::discard(); } let (needle, haystack) = if pile1.len() < pile2.len() {
        (pile1, pile2.as_slice()) } else { (pile2, pile1.as_slice()) }; let bm_searcher =
        BoyerMooreSearch::new(needle.clone()); let freqy_memchr =
        FreqyPacked::new(needle); TestResult::from_bool(bm_searcher.find(haystack) ==
        freqy_memchr.find(haystack)) } fn qc_bm_finds_trailing_needle(haystack_pre : Vec
        < u8 >, needle : Vec < u8 >) -> TestResult { if needle.len() == 0 { return
        TestResult::discard(); } let mut haystack = haystack_pre.clone(); let searcher =
        BoyerMooreSearch::new(needle.clone()); if haystack.len() >= needle.len() &&
        searcher.find(haystack.as_slice()).is_some() { return TestResult::discard(); }
        haystack.extend(needle.clone()); let start = haystack_pre.len()
        .checked_sub(needle.len()).unwrap_or(0); for i in 0.. (needle.len() - 1) { if
        searcher.find(& haystack[(i + start)..]).is_some() { return
        TestResult::discard(); } } TestResult::from_bool(searcher.find(haystack
        .as_slice()).map(| x | x == haystack_pre.len()).unwrap_or(false)) } fn
        qc_bm_finds_subslice(haystack : Vec < u8 >, needle_start : usize, needle_length :
        usize) -> TestResult { if haystack.len() == 0 { return TestResult::discard(); }
        let needle_start = needle_start % haystack.len(); let needle_length =
        needle_length % (haystack.len() - needle_start); if needle_length == 0 { return
        TestResult::discard(); } let needle = & haystack[needle_start.. (needle_start +
        needle_length)]; let bm_searcher = BoyerMooreSearch::new(needle.to_vec()); let
        start = naive_find(& needle, & haystack); match start { None =>
        TestResult::from_bool(false), Some(nf_start) => TestResult::from_bool(nf_start <=
        needle_start && bm_searcher.find(& haystack) == start) } } fn
        qc_bm_finds_first(needle : Vec < u8 >) -> TestResult { if needle.len() == 0 {
        return TestResult::discard(); } let mut haystack = needle.clone(); let searcher =
        BoyerMooreSearch::new(needle.clone()); haystack.extend(needle);
        TestResult::from_bool(searcher.find(haystack.as_slice()).map(| x | x == 0)
        .unwrap_or(false)) }
    }
}
#[cfg(test)]
mod tests_llm_16_406 {
    use crate::literal::imp::BoyerMooreSearch;
    #[test]
    fn test_approximate_size() {
        let _rug_st_tests_llm_16_406_rrrruuuugggg_test_approximate_size = 0;
        let rug_fuzz_0 = 97;
        let rug_fuzz_1 = 256;
        let pattern: Vec<u8> = vec![rug_fuzz_0, 98, 99, 100];
        let bm = BoyerMooreSearch::new(pattern.clone());
        let expected_size = pattern.len() * std::mem::size_of::<u8>()
            + rug_fuzz_1 * std::mem::size_of::<usize>();
        debug_assert_eq!(bm.approximate_size(), expected_size);
        let _rug_ed_tests_llm_16_406_rrrruuuugggg_test_approximate_size = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_407 {
    use super::*;
    use crate::*;
    #[test]
    fn test_check_match() {
        let _rug_st_tests_llm_16_407_rrrruuuugggg_test_check_match = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 5;
        let rug_fuzz_3 = true;
        let pattern: Vec<u8> = vec![rug_fuzz_0, 2, 3, 4];
        let bm_search = BoyerMooreSearch::new(pattern);
        let haystack: Vec<u8> = vec![rug_fuzz_1, 1, 2, 3, 4, 5];
        let window_end = rug_fuzz_2;
        let expected = rug_fuzz_3;
        let result = bm_search.check_match(&haystack, window_end);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_407_rrrruuuugggg_test_check_match = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_412 {
    use super::*;
    use crate::*;
    #[test]
    fn test_find_empty_haystack() {
        let _rug_st_tests_llm_16_412_rrrruuuugggg_test_find_empty_haystack = 0;
        let rug_fuzz_0 = 1;
        let pattern: Vec<u8> = vec![rug_fuzz_0, 2, 3];
        let bm_search = BoyerMooreSearch::new(pattern);
        let haystack: Vec<u8> = vec![];
        let result = bm_search.find(&haystack);
        debug_assert_eq!(result, None);
        let _rug_ed_tests_llm_16_412_rrrruuuugggg_test_find_empty_haystack = 0;
    }
    #[test]
    fn test_find_haystack_shorter_than_pattern() {
        let _rug_st_tests_llm_16_412_rrrruuuugggg_test_find_haystack_shorter_than_pattern = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 1;
        let pattern: Vec<u8> = vec![rug_fuzz_0, 2, 3];
        let bm_search = BoyerMooreSearch::new(pattern);
        let haystack: Vec<u8> = vec![rug_fuzz_1, 2];
        let result = bm_search.find(&haystack);
        debug_assert_eq!(result, None);
        let _rug_ed_tests_llm_16_412_rrrruuuugggg_test_find_haystack_shorter_than_pattern = 0;
    }
    #[test]
    fn test_find_haystack_same_as_pattern() {
        let _rug_st_tests_llm_16_412_rrrruuuugggg_test_find_haystack_same_as_pattern = 0;
        let rug_fuzz_0 = 1;
        let pattern: Vec<u8> = vec![rug_fuzz_0, 2, 3];
        let bm_search = BoyerMooreSearch::new(pattern.clone());
        let haystack: Vec<u8> = pattern.clone();
        let result = bm_search.find(&haystack);
        debug_assert_eq!(result, Some(0));
        let _rug_ed_tests_llm_16_412_rrrruuuugggg_test_find_haystack_same_as_pattern = 0;
    }
    #[test]
    fn test_find_haystack_contains_pattern() {
        let _rug_st_tests_llm_16_412_rrrruuuugggg_test_find_haystack_contains_pattern = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 0;
        let pattern: Vec<u8> = vec![rug_fuzz_0, 2, 3];
        let bm_search = BoyerMooreSearch::new(pattern.clone());
        let haystack: Vec<u8> = vec![rug_fuzz_1, 1, 2, 3, 4];
        let result = bm_search.find(&haystack);
        debug_assert_eq!(result, Some(1));
        let _rug_ed_tests_llm_16_412_rrrruuuugggg_test_find_haystack_contains_pattern = 0;
    }
    #[test]
    fn test_find_haystack_does_not_contains_pattern() {
        let _rug_st_tests_llm_16_412_rrrruuuugggg_test_find_haystack_does_not_contains_pattern = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 1;
        let pattern: Vec<u8> = vec![rug_fuzz_0, 2, 3];
        let bm_search = BoyerMooreSearch::new(pattern);
        let haystack: Vec<u8> = vec![rug_fuzz_1, 3, 2, 4];
        let result = bm_search.find(&haystack);
        debug_assert_eq!(result, None);
        let _rug_ed_tests_llm_16_412_rrrruuuugggg_test_find_haystack_does_not_contains_pattern = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_413 {
    use super::*;
    use crate::*;
    #[test]
    fn test_len() {
        let _rug_st_tests_llm_16_413_rrrruuuugggg_test_len = 0;
        let rug_fuzz_0 = 1;
        let searcher = BoyerMooreSearch::new(vec![rug_fuzz_0, 2, 3, 4, 5]);
        debug_assert_eq!(searcher.len(), 5);
        let _rug_ed_tests_llm_16_413_rrrruuuugggg_test_len = 0;
    }
}
#[cfg(test)]
mod unit_tests {
    use super::*;
    use crate::*;
    #[test]
    fn test_new() {
        let pattern = vec![1, 2, 3];
        let _ = BoyerMooreSearch::new(pattern);
    }
}
#[cfg(test)]
mod tests_llm_16_416_llm_16_415 {
    use crate::literal::imp::BoyerMooreSearch;
    #[test]
    fn test_select_guard() {
        let _rug_st_tests_llm_16_416_llm_16_415_rrrruuuugggg_test_select_guard = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 4;
        let rug_fuzz_4 = 5;
        let pattern: &[u8] = &[
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
        ];
        let (rarest, rarest_rev_idx) = BoyerMooreSearch::select_guard(pattern);
        debug_assert_eq!(rarest, 1);
        debug_assert_eq!(rarest_rev_idx, 4);
        let _rug_ed_tests_llm_16_416_llm_16_415_rrrruuuugggg_test_select_guard = 0;
    }
}
#[test]
fn test_should_use() {
    assert_eq!(crate ::literal::imp::BoyerMooreSearch::should_use(b"abcde"), true);
    assert_eq!(
        crate ::literal::imp::BoyerMooreSearch::should_use(b"eeeeeeeeeeeeeeee"), false
    );
    assert_eq!(crate ::literal::imp::BoyerMooreSearch::should_use(b"abcdeabcde"), true);
    assert_eq!(
        crate ::literal::imp::BoyerMooreSearch::should_use(b"abcdeeeeeee"), false
    );
    assert_eq!(
        crate ::literal::imp::BoyerMooreSearch::should_use(b"abcdeabcdeabcdeabcdeabcde"),
        true
    );
    assert_eq!(
        crate
        ::literal::imp::BoyerMooreSearch::should_use(b"abcdeabcdeabcdeabcdeeeeeee"),
        false
    );
}
#[cfg(test)]
mod tests_llm_16_419 {
    use super::*;
    use crate::*;
    #[test]
    fn test_skip_loop() {
        let _rug_st_tests_llm_16_419_rrrruuuugggg_test_skip_loop = 0;
        let rug_fuzz_0 = 97;
        let rug_fuzz_1 = 97;
        let rug_fuzz_2 = 8;
        let rug_fuzz_3 = 2;
        let pattern: Vec<u8> = vec![rug_fuzz_0, 98, 98];
        let haystack: Vec<u8> = vec![
            rug_fuzz_1, 98, 98, 99, 97, 98, 100, 98, 99, 97, 98
        ];
        let backstop: usize = rug_fuzz_2;
        let bm_search = BoyerMooreSearch::new(pattern);
        let result = bm_search.skip_loop(&haystack, rug_fuzz_3, backstop);
        debug_assert_eq!(result, Some(5));
        let _rug_ed_tests_llm_16_419_rrrruuuugggg_test_skip_loop = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_420 {
    use super::*;
    use crate::*;
    use crate::literal::imp::FreqyPacked;
    #[test]
    fn test_approximate_size() {
        let _rug_st_tests_llm_16_420_rrrruuuugggg_test_approximate_size = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 5;
        let pat = vec![rug_fuzz_0, 2, 3, 4, 5];
        let freqy_packed = FreqyPacked::new(pat);
        let actual = freqy_packed.approximate_size();
        let expected = rug_fuzz_1 * std::mem::size_of::<u8>();
        debug_assert_eq!(actual, expected);
        let _rug_ed_tests_llm_16_420_rrrruuuugggg_test_approximate_size = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_421 {
    use crate::literal::imp::FreqyPacked;
    #[test]
    fn test_char_len() {
        let _rug_st_tests_llm_16_421_rrrruuuugggg_test_char_len = 0;
        let rug_fuzz_0 = 97;
        let freqy_packed = FreqyPacked::new(vec![rug_fuzz_0, 98, 99]);
        let char_len = freqy_packed.char_len();
        debug_assert_eq!(char_len, 3);
        let _rug_ed_tests_llm_16_421_rrrruuuugggg_test_char_len = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_422 {
    use super::*;
    use crate::*;
    use std::clone::Clone;
    use std::fmt::Debug;
    #[derive(Debug, Clone)]
    struct FreqyPacked {
        pat: Vec<u8>,
        char_len: usize,
        rare1: u8,
        rare1i: usize,
        rare2: u8,
        rare2i: usize,
    }
    impl FreqyPacked {
        fn new(pat: Vec<u8>) -> FreqyPacked {
            unimplemented!()
        }
        fn empty() -> FreqyPacked {
            FreqyPacked {
                pat: vec![],
                char_len: 0,
                rare1: 0,
                rare1i: 0,
                rare2: 0,
                rare2i: 0,
            }
        }
        fn find(&self, haystack: &[u8]) -> Option<usize> {
            unimplemented!()
        }
        fn is_suffix(&self, text: &[u8]) -> bool {
            unimplemented!()
        }
        fn len(&self) -> usize {
            unimplemented!()
        }
        fn char_len(&self) -> usize {
            unimplemented!()
        }
        fn approximate_size(&self) -> usize {
            unimplemented!()
        }
    }
    #[test]
    fn test_empty() {
        let _rug_st_tests_llm_16_422_rrrruuuugggg_test_empty = 0;
        let empty_freqy_packed = FreqyPacked::empty();
        let _rug_ed_tests_llm_16_422_rrrruuuugggg_test_empty = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_423 {
    use super::*;
    use crate::*;
    #[test]
    fn test_find() {
        let _rug_st_tests_llm_16_423_rrrruuuugggg_test_find = 0;
        let rug_fuzz_0 = b'a';
        let rug_fuzz_1 = b"abcabcabcabc";
        let pattern = FreqyPacked::new(vec![rug_fuzz_0, b'b', b'c']);
        let haystack = rug_fuzz_1;
        let result = pattern.find(haystack);
        debug_assert_eq!(result, Some(0));
        let _rug_ed_tests_llm_16_423_rrrruuuugggg_test_find = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_424 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_suffix_returns_true_when_input_has_suffix() {
        let _rug_st_tests_llm_16_424_rrrruuuugggg_test_is_suffix_returns_true_when_input_has_suffix = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 0;
        let pattern = vec![rug_fuzz_0, 2, 3];
        let text = vec![rug_fuzz_1, 1, 2, 3];
        let packed = FreqyPacked::new(pattern);
        debug_assert!(packed.is_suffix(& text));
        let _rug_ed_tests_llm_16_424_rrrruuuugggg_test_is_suffix_returns_true_when_input_has_suffix = 0;
    }
    #[test]
    fn test_is_suffix_returns_false_when_input_does_not_have_suffix() {
        let _rug_st_tests_llm_16_424_rrrruuuugggg_test_is_suffix_returns_false_when_input_does_not_have_suffix = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 0;
        let pattern = vec![rug_fuzz_0, 2, 3];
        let text = vec![rug_fuzz_1, 1, 2];
        let packed = FreqyPacked::new(pattern);
        debug_assert!(! packed.is_suffix(& text));
        let _rug_ed_tests_llm_16_424_rrrruuuugggg_test_is_suffix_returns_false_when_input_does_not_have_suffix = 0;
    }
    #[test]
    fn test_is_suffix_returns_false_when_input_has_insufficient_length() {
        let _rug_st_tests_llm_16_424_rrrruuuugggg_test_is_suffix_returns_false_when_input_has_insufficient_length = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 0;
        let pattern = vec![rug_fuzz_0, 2, 3, 4];
        let text = vec![rug_fuzz_1, 1, 2, 3];
        let packed = FreqyPacked::new(pattern);
        debug_assert!(! packed.is_suffix(& text));
        let _rug_ed_tests_llm_16_424_rrrruuuugggg_test_is_suffix_returns_false_when_input_has_insufficient_length = 0;
    }
    #[test]
    fn test_is_suffix_returns_false_when_input_and_pattern_are_empty() {
        let _rug_st_tests_llm_16_424_rrrruuuugggg_test_is_suffix_returns_false_when_input_and_pattern_are_empty = 0;
        let pattern = vec![];
        let text = vec![];
        let packed = FreqyPacked::new(pattern);
        debug_assert!(! packed.is_suffix(& text));
        let _rug_ed_tests_llm_16_424_rrrruuuugggg_test_is_suffix_returns_false_when_input_and_pattern_are_empty = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_425 {
    use super::*;
    use crate::*;
    #[test]
    fn test_len() {
        let _rug_st_tests_llm_16_425_rrrruuuugggg_test_len = 0;
        let rug_fuzz_0 = 97;
        let rug_fuzz_1 = 4;
        let rug_fuzz_2 = 97;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 98;
        let rug_fuzz_5 = 1;
        let freqy_packed = FreqyPacked {
            pat: vec![rug_fuzz_0, 98, 99, 100],
            char_len: rug_fuzz_1,
            rare1: rug_fuzz_2,
            rare1i: rug_fuzz_3,
            rare2: rug_fuzz_4,
            rare2i: rug_fuzz_5,
        };
        debug_assert_eq!(freqy_packed.len(), 4);
        let _rug_ed_tests_llm_16_425_rrrruuuugggg_test_len = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_428 {
    use super::*;
    use crate::*;
    #[test]
    fn test_approximate_size() {
        let _rug_st_tests_llm_16_428_rrrruuuugggg_test_approximate_size = 0;
        let rug_fuzz_0 = false;
        let matcher = Matcher::Empty;
        let literal_searcher = LiteralSearcher {
            complete: rug_fuzz_0,
            lcp: FreqyPacked::new(vec![]),
            lcs: FreqyPacked::new(vec![]),
            matcher,
        };
        debug_assert_eq!(literal_searcher.approximate_size(), 0);
        let _rug_ed_tests_llm_16_428_rrrruuuugggg_test_approximate_size = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_459 {
    use super::*;
    use crate::*;
    #[test]
    fn test_find_empty_set() {
        let _rug_st_tests_llm_16_459_rrrruuuugggg_test_find_empty_set = 0;
        let rug_fuzz_0 = b'a';
        let rug_fuzz_1 = b'b';
        let rug_fuzz_2 = b'c';
        let rug_fuzz_3 = b'd';
        let rug_fuzz_4 = b'e';
        let sbs = SingleByteSet::new();
        let haystack = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3, rug_fuzz_4];
        let result = sbs._find(&haystack);
        debug_assert_eq!(result, None);
        let _rug_ed_tests_llm_16_459_rrrruuuugggg_test_find_empty_set = 0;
    }
    #[test]
    fn test_find_single_element_set() {
        let _rug_st_tests_llm_16_459_rrrruuuugggg_test_find_single_element_set = 0;
        let rug_fuzz_0 = 97;
        let rug_fuzz_1 = true;
        let rug_fuzz_2 = b'a';
        let rug_fuzz_3 = b'b';
        let rug_fuzz_4 = b'c';
        let rug_fuzz_5 = b'd';
        let rug_fuzz_6 = b'e';
        let mut sbs = SingleByteSet::new();
        sbs.sparse[rug_fuzz_0] = rug_fuzz_1;
        let haystack = [rug_fuzz_2, rug_fuzz_3, rug_fuzz_4, rug_fuzz_5, rug_fuzz_6];
        let result = sbs._find(&haystack);
        debug_assert_eq!(result, Some(0));
        let _rug_ed_tests_llm_16_459_rrrruuuugggg_test_find_single_element_set = 0;
    }
    #[test]
    fn test_find_multi_element_set() {
        let _rug_st_tests_llm_16_459_rrrruuuugggg_test_find_multi_element_set = 0;
        let rug_fuzz_0 = 97;
        let rug_fuzz_1 = true;
        let rug_fuzz_2 = 99;
        let rug_fuzz_3 = true;
        let rug_fuzz_4 = 101;
        let rug_fuzz_5 = true;
        let rug_fuzz_6 = b'a';
        let rug_fuzz_7 = b'b';
        let rug_fuzz_8 = b'c';
        let rug_fuzz_9 = b'd';
        let rug_fuzz_10 = b'e';
        let mut sbs = SingleByteSet::new();
        sbs.sparse[rug_fuzz_0] = rug_fuzz_1;
        sbs.sparse[rug_fuzz_2] = rug_fuzz_3;
        sbs.sparse[rug_fuzz_4] = rug_fuzz_5;
        let haystack = [rug_fuzz_6, rug_fuzz_7, rug_fuzz_8, rug_fuzz_9, rug_fuzz_10];
        let result = sbs._find(&haystack);
        debug_assert_eq!(result, Some(0));
        let _rug_ed_tests_llm_16_459_rrrruuuugggg_test_find_multi_element_set = 0;
    }
    #[test]
    fn test_find_not_found() {
        let _rug_st_tests_llm_16_459_rrrruuuugggg_test_find_not_found = 0;
        let rug_fuzz_0 = 98;
        let rug_fuzz_1 = true;
        let rug_fuzz_2 = 100;
        let rug_fuzz_3 = true;
        let rug_fuzz_4 = b'a';
        let rug_fuzz_5 = b'c';
        let rug_fuzz_6 = b'e';
        let mut sbs = SingleByteSet::new();
        sbs.sparse[rug_fuzz_0] = rug_fuzz_1;
        sbs.sparse[rug_fuzz_2] = rug_fuzz_3;
        let haystack = [rug_fuzz_4, rug_fuzz_5, rug_fuzz_6];
        let result = sbs._find(&haystack);
        debug_assert_eq!(result, None);
        let _rug_ed_tests_llm_16_459_rrrruuuugggg_test_find_not_found = 0;
    }
    #[test]
    fn test_find_duplicate_elements() {
        let _rug_st_tests_llm_16_459_rrrruuuugggg_test_find_duplicate_elements = 0;
        let rug_fuzz_0 = 98;
        let rug_fuzz_1 = true;
        let rug_fuzz_2 = b'b';
        let rug_fuzz_3 = b'b';
        let rug_fuzz_4 = b'b';
        let rug_fuzz_5 = b'b';
        let rug_fuzz_6 = b'b';
        let mut sbs = SingleByteSet::new();
        sbs.sparse[rug_fuzz_0] = rug_fuzz_1;
        let haystack = [rug_fuzz_2, rug_fuzz_3, rug_fuzz_4, rug_fuzz_5, rug_fuzz_6];
        let result = sbs._find(&haystack);
        debug_assert_eq!(result, Some(0));
        let _rug_ed_tests_llm_16_459_rrrruuuugggg_test_find_duplicate_elements = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_460 {
    use super::*;
    use crate::*;
    #[test]
    fn test_approximate_size() {
        let _rug_st_tests_llm_16_460_rrrruuuugggg_test_approximate_size = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = true;
        let rug_fuzz_2 = true;
        let sset = SingleByteSet {
            sparse: vec![false; 256],
            dense: vec![rug_fuzz_0, 2, 3],
            complete: rug_fuzz_1,
            all_ascii: rug_fuzz_2,
        };
        debug_assert_eq!(
            sset.approximate_size(), 3 * std::mem::size_of:: < u8 > () + 256 *
            std::mem::size_of:: < bool > ()
        );
        let _rug_ed_tests_llm_16_460_rrrruuuugggg_test_approximate_size = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_463 {
    use crate::literal::imp::SingleByteSet;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_463_rrrruuuugggg_test_new = 0;
        let sset = SingleByteSet::new();
        debug_assert_eq!(sset.sparse.len(), 256);
        debug_assert_eq!(sset.sparse.iter().filter(| & & b | b).count(), 0);
        debug_assert_eq!(sset.dense.len(), 0);
        debug_assert_eq!(sset.complete, true);
        debug_assert_eq!(sset.all_ascii, true);
        let _rug_ed_tests_llm_16_463_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_468 {
    use super::*;
    use crate::*;
    #[test]
    fn test_char_len_lossy() {
        let _rug_st_tests_llm_16_468_rrrruuuugggg_test_char_len_lossy = 0;
        let rug_fuzz_0 = 240;
        let rug_fuzz_1 = 159;
        let rug_fuzz_2 = 146;
        let rug_fuzz_3 = 240;
        let rug_fuzz_4 = 159;
        let rug_fuzz_5 = 146;
        let rug_fuzz_6 = 150;
        let rug_fuzz_7 = 240;
        let rug_fuzz_8 = 159;
        let rug_fuzz_9 = 146;
        let rug_fuzz_10 = 150;
        let rug_fuzz_11 = 240;
        let rug_fuzz_12 = 159;
        let rug_fuzz_13 = 146;
        let rug_fuzz_14 = 150;
        let rug_fuzz_15 = 50;
        let bytes1: [u8; 3] = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        let bytes2: [u8; 8] = [
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
            rug_fuzz_6,
            rug_fuzz_7,
            rug_fuzz_8,
            rug_fuzz_9,
            rug_fuzz_10,
        ];
        let bytes3: [u8; 5] = [
            rug_fuzz_11,
            rug_fuzz_12,
            rug_fuzz_13,
            rug_fuzz_14,
            rug_fuzz_15,
        ];
        debug_assert_eq!(char_len_lossy(& bytes1), 1);
        debug_assert_eq!(char_len_lossy(& bytes2), 2);
        debug_assert_eq!(char_len_lossy(& bytes3), 2);
        let _rug_ed_tests_llm_16_468_rrrruuuugggg_test_char_len_lossy = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_469 {
    use super::*;
    use crate::*;
    use crate::literal::imp::*;
    #[test]
    fn test_freq_rank() {
        let _rug_st_tests_llm_16_469_rrrruuuugggg_test_freq_rank = 0;
        let rug_fuzz_0 = b'A';
        let rug_fuzz_1 = b'Z';
        let rug_fuzz_2 = b'a';
        let rug_fuzz_3 = b'z';
        let rug_fuzz_4 = b'0';
        let rug_fuzz_5 = b'9';
        let rug_fuzz_6 = b'_';
        let rug_fuzz_7 = b'.';
        let rug_fuzz_8 = b' ';
        let rug_fuzz_9 = b'\n';
        let rug_fuzz_10 = b'\t';
        let rug_fuzz_11 = b'\r';
        let rug_fuzz_12 = b'\0';
        debug_assert_eq!(freq_rank(rug_fuzz_0), 0);
        debug_assert_eq!(freq_rank(rug_fuzz_1), 0);
        debug_assert_eq!(freq_rank(rug_fuzz_2), 1);
        debug_assert_eq!(freq_rank(rug_fuzz_3), 1);
        debug_assert_eq!(freq_rank(rug_fuzz_4), 2);
        debug_assert_eq!(freq_rank(rug_fuzz_5), 2);
        debug_assert_eq!(freq_rank(rug_fuzz_6), 3);
        debug_assert_eq!(freq_rank(rug_fuzz_7), 3);
        debug_assert_eq!(freq_rank(rug_fuzz_8), 3);
        debug_assert_eq!(freq_rank(rug_fuzz_9), 3);
        debug_assert_eq!(freq_rank(rug_fuzz_10), 3);
        debug_assert_eq!(freq_rank(rug_fuzz_11), 3);
        debug_assert_eq!(freq_rank(rug_fuzz_12), 3);
        let _rug_ed_tests_llm_16_469_rrrruuuugggg_test_freq_rank = 0;
    }
}
#[cfg(test)]
mod tests_rug_151 {
    use super::*;
    use crate::literal::imp::LiteralSearcher;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_151_rrrruuuugggg_test_rug = 0;
        let searcher: LiteralSearcher = LiteralSearcher::empty();
        let _rug_ed_tests_rug_151_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_155 {
    use super::*;
    use crate::literal::imp::{LiteralSearcher, Literals, Matcher, FreqyPacked};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_155_rrrruuuugggg_test_rug = 0;
        let mut p0: LiteralSearcher = LiteralSearcher::empty();
        p0.complete();
        let _rug_ed_tests_rug_155_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_156 {
    use super::*;
    use crate::literal::imp::{LiteralSearcher, Matcher};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_156_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"example haystack";
        let p0 = LiteralSearcher::empty();
        let p1: &[u8] = rug_fuzz_0;
        LiteralSearcher::find(&p0, p1);
        let _rug_ed_tests_rug_156_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_157 {
    use super::*;
    use crate::literal::imp::LiteralSearcher;
    #[test]
    fn test_find_start() {
        let _rug_st_tests_rug_157_rrrruuuugggg_test_find_start = 0;
        let rug_fuzz_0 = b"Lorem ipsum dolor sit amet";
        let searcher = LiteralSearcher::empty();
        let haystack = rug_fuzz_0;
        searcher.find_start(haystack);
        let _rug_ed_tests_rug_157_rrrruuuugggg_test_find_start = 0;
    }
}
#[cfg(test)]
mod tests_rug_158 {
    use super::*;
    use crate::literal::LiteralSearcher;
    use crate::internal::LiteralSearcher as InternalLiteralSearcher;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_158_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"abcd";
        let searcher: InternalLiteralSearcher = LiteralSearcher::empty();
        let haystack: &[u8] = rug_fuzz_0;
        searcher.find_end(haystack);
        let _rug_ed_tests_rug_158_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_160 {
    use super::*;
    use crate::internal::LiteralSearcher;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_160_rrrruuuugggg_test_rug = 0;
        let mut p0: LiteralSearcher = LiteralSearcher::empty();
        let lcp = p0.lcp();
        let _rug_ed_tests_rug_160_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_161 {
    use super::*;
    use crate::literal::imp::LiteralSearcher;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_161_rrrruuuugggg_test_rug = 0;
        let p0 = LiteralSearcher::empty();
        LiteralSearcher::lcs(&p0);
        let _rug_ed_tests_rug_161_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_162 {
    use super::*;
    use crate::internal::LiteralSearcher;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_162_rrrruuuugggg_test_rug = 0;
        let p0 = LiteralSearcher::empty();
        debug_assert_eq!(p0.is_empty(), true);
        let _rug_ed_tests_rug_162_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_163 {
    use super::*;
    use crate::literal::imp::LiteralSearcher;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_163_rrrruuuugggg_test_rug = 0;
        let mut p0: LiteralSearcher = LiteralSearcher::empty();
        p0.len();
        let _rug_ed_tests_rug_163_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_170 {
    use super::*;
    use crate::literal::imp::SingleByteSet;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_170_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"sample_text";
        let mut v53 = SingleByteSet::new();
        let text: &[u8] = rug_fuzz_0;
        SingleByteSet::find(&v53, text);
        let _rug_ed_tests_rug_170_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_171 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_171_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        use crate::literal::imp::FreqyPacked;
        debug_assert_eq!(FreqyPacked::empty().pat, vec![]);
        let mut v49: Vec<u8> = Vec::new();
        v49.push(rug_fuzz_0);
        v49.push(rug_fuzz_1);
        let _ = FreqyPacked::new(v49);
        let _rug_ed_tests_rug_171_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_172 {
    use super::*;
    use crate::literal::imp::BoyerMooreSearch;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_172_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"abracadabra";
        let p0: &[u8] = rug_fuzz_0;
        BoyerMooreSearch::compile_skip_table(p0);
        let _rug_ed_tests_rug_172_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_173 {
    use super::*;
    use crate::literal::imp::BoyerMooreSearch;
    #[test]
    fn test_compile_md2_shift() {
        let _rug_st_tests_rug_173_rrrruuuugggg_test_compile_md2_shift = 0;
        let rug_fuzz_0 = b"abcde";
        let pattern: &[u8] = rug_fuzz_0;
        BoyerMooreSearch::compile_md2_shift(pattern);
        let _rug_ed_tests_rug_173_rrrruuuugggg_test_compile_md2_shift = 0;
    }
}
