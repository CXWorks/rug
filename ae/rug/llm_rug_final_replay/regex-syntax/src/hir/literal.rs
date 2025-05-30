/*!
Provides literal extraction from `Hir` expressions.

An [`Extractor`] pulls literals out of [`Hir`] expressions and returns a
[`Seq`] of [`Literal`]s.

The purpose of literal extraction is generally to provide avenues for
optimizing regex searches. The main idea is that substring searches can be an
order of magnitude faster than a regex search. Therefore, if one can execute
a substring search to find candidate match locations and only run the regex
search at those locations, then it is possible for huge improvements in
performance to be realized.

With that said, literal optimizations are generally a black art because even
though substring search is generally faster, if the number of candidates
produced is high, then it can create a lot of overhead by ping-ponging between
the substring search and the regex search.

Here are some heuristics that might be used to help increase the chances of
effective literal optimizations:

* Stick to small [`Seq`]s. If you search for too many literals, it's likely
to lead to substring search that is only a little faster than a regex search,
and thus the overhead of using literal optimizations in the first place might
make things slower overall.
* The literals in your [`Seq`] shoudn't be too short. In general, longer is
better. A sequence corresponding to single bytes that occur frequently in the
haystack, for example, is probably a bad literal optimization because it's
likely to produce many false positive candidates. Longer literals are less
likely to match, and thus probably produce fewer false positives.
* If it's possible to estimate the approximate frequency of each byte according
to some pre-computed background distribution, it is possible to compute a score
of how "good" a `Seq` is. If a `Seq` isn't good enough, you might consider
skipping the literal optimization and just use the regex engine.

(It should be noted that there are always pathological cases that can make
any kind of literal optimization be a net slower result. This is why it
might be a good idea to be conservative, or to even provide a means for
literal optimizations to be dynamically disabled if they are determined to be
ineffective according to some measure.)

You're encouraged to explore the methods on [`Seq`], which permit shrinking
the size of sequences in a preference-order preserving fashion.

Finally, note that it isn't strictly necessary to use an [`Extractor`]. Namely,
an `Extractor` only uses public APIs of the [`Seq`] and [`Literal`] types,
so it is possible to implement your own extractor. For example, for n-grams
or "inner" literals (i.e., not prefix or suffix literals). The `Extractor`
is mostly responsible for the case analysis over `Hir` expressions. Much of
the "trickier" parts are how to combine literal sequences, and that is all
implemented on [`Seq`].
*/
use core::{cmp, mem};
use alloc::{vec, vec::Vec};
use crate::hir::{self, Hir};
/// Extracts prefix or suffix literal sequences from [`Hir`] expressions.
///
/// Literal extraction is based on the following observations:
///
/// * Many regexes start with one or a small number of literals.
/// * Substring search for literals is often much faster (sometimes by an order
/// of magnitude) than a regex search.
///
/// Thus, in many cases, one can search for literals to find candidate starting
/// locations of a match, and then only run the full regex engine at each such
/// location instead of over the full haystack.
///
/// The main downside of literal extraction is that it can wind up causing a
/// search to be slower overall. For example, if there are many matches or if
/// there are many candidates that don't ultimately lead to a match, then a
/// lot of overhead will be spent in shuffing back-and-forth between substring
/// search and the regex engine. This is the fundamental reason why literal
/// optimizations for regex patterns is sometimes considered a "black art."
///
/// # Look-around assertions
///
/// Literal extraction treats all look-around assertions as-if they match every
/// empty string. So for example, the regex `\bquux\b` will yield a sequence
/// containing a single exact literal `quux`. However, not all occurrences
/// of `quux` correspond to a match a of the regex. For example, `\bquux\b`
/// does not match `ZquuxZ` anywhere because `quux` does not fall on a word
/// boundary.
///
/// In effect, if your regex contains look-around assertions, then a match of
/// an exact literal does not necessarily mean the regex overall matches. So
/// you may still need to run the regex engine in such cases to confirm the
/// match.
///
/// The precise guarantee you get from a literal sequence is: if every literal
/// in the sequence is exact and the original regex contains zero look-around
/// assertions, then a preference-order multi-substring search of those
/// literals will precisely match a preference-order search of the original
/// regex.
///
/// # Example
///
/// This shows how to extract prefixes:
///
/// ```
/// use regex_syntax::{hir::literal::{Extractor, Literal, Seq}, parse};
///
/// let hir = parse(r"(a|b|c)(x|y|z)[A-Z]+foo")?;
/// let got = Extractor::new().extract(&hir);
/// // All literals returned are "inexact" because none of them reach the
/// // match state.
/// let expected = Seq::from_iter([
///     Literal::inexact("ax"),
///     Literal::inexact("ay"),
///     Literal::inexact("az"),
///     Literal::inexact("bx"),
///     Literal::inexact("by"),
///     Literal::inexact("bz"),
///     Literal::inexact("cx"),
///     Literal::inexact("cy"),
///     Literal::inexact("cz"),
/// ]);
/// assert_eq!(expected, got);
///
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// This shows how to extract suffixes:
///
/// ```
/// use regex_syntax::{
///     hir::literal::{Extractor, ExtractKind, Literal, Seq},
///     parse,
/// };
///
/// let hir = parse(r"foo|[A-Z]+bar")?;
/// let got = Extractor::new().kind(ExtractKind::Suffix).extract(&hir);
/// // Since 'foo' gets to a match state, it is considered exact. But 'bar'
/// // does not because of the '[A-Z]+', and thus is marked inexact.
/// let expected = Seq::from_iter([
///     Literal::exact("foo"),
///     Literal::inexact("bar"),
/// ]);
/// assert_eq!(expected, got);
///
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug)]
pub struct Extractor {
    kind: ExtractKind,
    limit_class: usize,
    limit_repeat: usize,
    limit_literal_len: usize,
    limit_total: usize,
}
impl Extractor {
    /// Create a new extractor with a default configuration.
    ///
    /// The extractor can be optionally configured before calling
    /// [`Extractor::extract`] to get a literal sequence.
    pub fn new() -> Extractor {
        Extractor {
            kind: ExtractKind::Prefix,
            limit_class: 10,
            limit_repeat: 10,
            limit_literal_len: 100,
            limit_total: 250,
        }
    }
    /// Execute the extractor and return a sequence of literals.
    pub fn extract(&self, hir: &Hir) -> Seq {
        use crate::hir::HirKind::*;
        match *hir.kind() {
            Empty | Look(_) => Seq::singleton(self::Literal::exact(vec![])),
            Literal(hir::Literal(ref bytes)) => {
                let mut seq = Seq::singleton(self::Literal::exact(bytes.to_vec()));
                self.enforce_literal_len(&mut seq);
                seq
            }
            Class(hir::Class::Unicode(ref cls)) => self.extract_class_unicode(cls),
            Class(hir::Class::Bytes(ref cls)) => self.extract_class_bytes(cls),
            Repetition(ref rep) => self.extract_repetition(rep),
            Capture(hir::Capture { ref sub, .. }) => self.extract(sub),
            Concat(ref hirs) => {
                match self.kind {
                    ExtractKind::Prefix => self.extract_concat(hirs.iter()),
                    ExtractKind::Suffix => self.extract_concat(hirs.iter().rev()),
                }
            }
            Alternation(ref hirs) => self.extract_alternation(hirs.iter()),
        }
    }
    /// Set the kind of literal sequence to extract from an [`Hir`] expression.
    ///
    /// The default is to extract prefixes, but suffixes can be selected
    /// instead. The contract for prefixes is that every match of the
    /// corresponding `Hir` must start with one of the literals in the sequence
    /// returned. Moreover, the _order_ of the sequence returned corresponds to
    /// the preference order.
    ///
    /// Suffixes satisfy a similar contract in that every match of the
    /// corresponding `Hir` must end with one of the literals in the sequence
    /// returned. However, there is no guarantee that the literals are in
    /// preference order.
    ///
    /// Remember that a sequence can be infinite. For example, unless the
    /// limits are configured to be impractically large, attempting to extract
    /// prefixes (or suffixes) for the pattern `[A-Z]` will return an infinite
    /// sequence. Generally speaking, if the sequence returned is infinite,
    /// then it is presumed to be unwise to do prefix (or suffix) optimizations
    /// for the pattern.
    pub fn kind(&mut self, kind: ExtractKind) -> &mut Extractor {
        self.kind = kind;
        self
    }
    /// Configure a limit on the length of the sequence that is permitted for
    /// a character class. If a character class exceeds this limit, then the
    /// sequence returned for it is infinite.
    ///
    /// This prevents classes like `[A-Z]` or `\pL` from getting turned into
    /// huge and likely unproductive sequences of literals.
    ///
    /// # Example
    ///
    /// This example shows how this limit can be lowered to decrease the tolerance
    /// for character classes being turned into literal sequences.
    ///
    /// ```
    /// use regex_syntax::{hir::literal::{Extractor, Seq}, parse};
    ///
    /// let hir = parse(r"[0-9]")?;
    ///
    /// let got = Extractor::new().extract(&hir);
    /// let expected = Seq::new([
    ///     "0", "1", "2", "3", "4", "5", "6", "7", "8", "9",
    /// ]);
    /// assert_eq!(expected, got);
    ///
    /// // Now let's shrink the limit and see how that changes things.
    /// let got = Extractor::new().limit_class(4).extract(&hir);
    /// let expected = Seq::infinite();
    /// assert_eq!(expected, got);
    ///
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn limit_class(&mut self, limit: usize) -> &mut Extractor {
        self.limit_class = limit;
        self
    }
    /// Configure a limit on the total number of repetitions that is permitted
    /// before literal extraction is stopped.
    ///
    /// This is useful for limiting things like `(abcde){50}`, or more
    /// insidiously, `(?:){1000000000}`. This limit prevents any one single
    /// repetition from adding too much to a literal sequence.
    ///
    /// With this limit set, repetitions that exceed it will be stopped and any
    /// literals extracted up to that point will be made inexact.
    ///
    /// # Example
    ///
    /// This shows how to decrease the limit and compares it with the default.
    ///
    /// ```
    /// use regex_syntax::{hir::literal::{Extractor, Literal, Seq}, parse};
    ///
    /// let hir = parse(r"(abc){8}")?;
    ///
    /// let got = Extractor::new().extract(&hir);
    /// let expected = Seq::new(["abcabcabcabcabcabcabcabc"]);
    /// assert_eq!(expected, got);
    ///
    /// // Now let's shrink the limit and see how that changes things.
    /// let got = Extractor::new().limit_repeat(4).extract(&hir);
    /// let expected = Seq::from_iter([
    ///     Literal::inexact("abcabcabcabc"),
    /// ]);
    /// assert_eq!(expected, got);
    ///
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn limit_repeat(&mut self, limit: usize) -> &mut Extractor {
        self.limit_repeat = limit;
        self
    }
    /// Configure a limit on the maximum length of any literal in a sequence.
    ///
    /// This is useful for limiting things like `(abcde){5}{5}{5}{5}`. While
    /// each repetition or literal in that regex is small, when all the
    /// repetitions are applied, one ends up with a literal of length `5^4 =
    /// 625`.
    ///
    /// With this limit set, literals that exceed it will be made inexact and
    /// thus prevented from growing.
    ///
    /// # Example
    ///
    /// This shows how to decrease the limit and compares it with the default.
    ///
    /// ```
    /// use regex_syntax::{hir::literal::{Extractor, Literal, Seq}, parse};
    ///
    /// let hir = parse(r"(abc){2}{2}{2}")?;
    ///
    /// let got = Extractor::new().extract(&hir);
    /// let expected = Seq::new(["abcabcabcabcabcabcabcabc"]);
    /// assert_eq!(expected, got);
    ///
    /// // Now let's shrink the limit and see how that changes things.
    /// let got = Extractor::new().limit_literal_len(14).extract(&hir);
    /// let expected = Seq::from_iter([
    ///     Literal::inexact("abcabcabcabcab"),
    /// ]);
    /// assert_eq!(expected, got);
    ///
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn limit_literal_len(&mut self, limit: usize) -> &mut Extractor {
        self.limit_literal_len = limit;
        self
    }
    /// Configure a limit on the total number of literals that will be
    /// returned.
    ///
    /// This is useful as a practical measure for avoiding the creation of
    /// large sequences of literals. While the extractor will automatically
    /// handle local creations of large sequences (for example, `[A-Z]` yields
    /// an infinite sequence by default), large sequences can be created
    /// through non-local means as well.
    ///
    /// For example, `[ab]{3}{3}` would yield a sequence of length `512 = 2^9`
    /// despite each of the repetitions being small on their own. This limit
    /// thus represents a "catch all" for avoiding locally small sequences from
    /// combining into large sequences.
    ///
    /// # Example
    ///
    /// This example shows how reducing the limit will change the literal
    /// sequence returned.
    ///
    /// ```
    /// use regex_syntax::{hir::literal::{Extractor, Literal, Seq}, parse};
    ///
    /// let hir = parse(r"[ab]{2}{2}")?;
    ///
    /// let got = Extractor::new().extract(&hir);
    /// let expected = Seq::new([
    ///     "aaaa", "aaab", "aaba", "aabb",
    ///     "abaa", "abab", "abba", "abbb",
    ///     "baaa", "baab", "baba", "babb",
    ///     "bbaa", "bbab", "bbba", "bbbb",
    /// ]);
    /// assert_eq!(expected, got);
    ///
    /// // The default limit is not too big, but big enough to extract all
    /// // literals from '[ab]{2}{2}'. If we shrink the limit to less than 16,
    /// // then we'll get a truncated set. Notice that it returns a sequence of
    /// // length 4 even though our limit was 10. This is because the sequence
    /// // is difficult to increase without blowing the limit. Notice also
    /// // that every literal in the sequence is now inexact because they were
    /// // stripped of some suffix.
    /// let got = Extractor::new().limit_total(10).extract(&hir);
    /// let expected = Seq::from_iter([
    ///     Literal::inexact("aa"),
    ///     Literal::inexact("ab"),
    ///     Literal::inexact("ba"),
    ///     Literal::inexact("bb"),
    /// ]);
    /// assert_eq!(expected, got);
    ///
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn limit_total(&mut self, limit: usize) -> &mut Extractor {
        self.limit_total = limit;
        self
    }
    /// Extract a sequence from the given concatenation. Sequences from each of
    /// the child HIR expressions are combined via cross product.
    ///
    /// This short circuits once the cross product turns into a sequence
    /// containing only inexact literals.
    fn extract_concat<'a, I: Iterator<Item = &'a Hir>>(&self, it: I) -> Seq {
        let mut seq = Seq::singleton(self::Literal::exact(vec![]));
        for hir in it {
            if seq.is_inexact() {
                break;
            }
            seq = self.cross(seq, &mut self.extract(hir));
        }
        seq
    }
    /// Extract a sequence from the given alternation.
    ///
    /// This short circuits once the union turns into an infinite sequence.
    fn extract_alternation<'a, I: Iterator<Item = &'a Hir>>(&self, it: I) -> Seq {
        let mut seq = Seq::empty();
        for hir in it {
            if !seq.is_finite() {
                break;
            }
            seq = self.union(seq, &mut self.extract(hir));
        }
        seq
    }
    /// Extract a sequence of literals from the given repetition. We do our
    /// best, Some examples:
    ///
    ///   'a*'    => [inexact(a), exact("")]
    ///   'a*?'   => [exact(""), inexact(a)]
    ///   'a+'    => [inexact(a)]
    ///   'a{3}'  => [exact(aaa)]
    ///   'a{3,5} => [inexact(aaa)]
    ///
    /// The key here really is making sure we get the 'inexact' vs 'exact'
    /// attributes correct on each of the literals we add. For example, the
    /// fact that 'a*' gives us an inexact 'a' and an exact empty string means
    /// that a regex like 'ab*c' will result in [inexact(ab), exact(ac)]
    /// literals being extracted, which might actually be a better prefilter
    /// than just 'a'.
    fn extract_repetition(&self, rep: &hir::Repetition) -> Seq {
        let mut subseq = self.extract(&rep.sub);
        match *rep {
            hir::Repetition { min: 0, max, greedy, .. } => {
                if max != Some(1) {
                    subseq.make_inexact();
                }
                let mut empty = Seq::singleton(Literal::exact(vec![]));
                if !greedy {
                    mem::swap(&mut subseq, &mut empty);
                }
                self.union(subseq, &mut empty)
            }
            hir::Repetition { min, max: Some(max), .. } if min == max => {
                assert!(min > 0);
                let limit = u32::try_from(self.limit_repeat).unwrap_or(u32::MAX);
                let mut seq = Seq::singleton(Literal::exact(vec![]));
                for _ in 0..cmp::min(min, limit) {
                    if seq.is_inexact() {
                        break;
                    }
                    seq = self.cross(seq, &mut subseq.clone());
                }
                if usize::try_from(min).is_err() || min > limit {
                    seq.make_inexact();
                }
                seq
            }
            hir::Repetition { min, max: Some(max), .. } if min < max => {
                assert!(min > 0);
                let limit = u32::try_from(self.limit_repeat).unwrap_or(u32::MAX);
                let mut seq = Seq::singleton(Literal::exact(vec![]));
                for _ in 0..cmp::min(min, limit) {
                    if seq.is_inexact() {
                        break;
                    }
                    seq = self.cross(seq, &mut subseq.clone());
                }
                seq.make_inexact();
                seq
            }
            hir::Repetition { .. } => {
                subseq.make_inexact();
                subseq
            }
        }
    }
    /// Convert the given Unicode class into a sequence of literals if the
    /// class is small enough. If the class is too big, return an infinite
    /// sequence.
    fn extract_class_unicode(&self, cls: &hir::ClassUnicode) -> Seq {
        if self.class_over_limit_unicode(cls) {
            return Seq::infinite();
        }
        let mut seq = Seq::empty();
        for r in cls.iter() {
            for ch in r.start()..=r.end() {
                seq.push(Literal::from(ch));
            }
        }
        self.enforce_literal_len(&mut seq);
        seq
    }
    /// Convert the given byte class into a sequence of literals if the class
    /// is small enough. If the class is too big, return an infinite sequence.
    fn extract_class_bytes(&self, cls: &hir::ClassBytes) -> Seq {
        if self.class_over_limit_bytes(cls) {
            return Seq::infinite();
        }
        let mut seq = Seq::empty();
        for r in cls.iter() {
            for b in r.start()..=r.end() {
                seq.push(Literal::from(b));
            }
        }
        self.enforce_literal_len(&mut seq);
        seq
    }
    /// Returns true if the given Unicode class exceeds the configured limits
    /// on this extractor.
    fn class_over_limit_unicode(&self, cls: &hir::ClassUnicode) -> bool {
        let mut count = 0;
        for r in cls.iter() {
            if count > self.limit_class {
                return true;
            }
            count += r.len();
        }
        count > self.limit_class
    }
    /// Returns true if the given byte class exceeds the configured limits on
    /// this extractor.
    fn class_over_limit_bytes(&self, cls: &hir::ClassBytes) -> bool {
        let mut count = 0;
        for r in cls.iter() {
            if count > self.limit_class {
                return true;
            }
            count += r.len();
        }
        count > self.limit_class
    }
    /// Compute the cross product of the two sequences if the result would be
    /// within configured limits. Otherwise, make `seq2` infinite and cross the
    /// infinite sequence with `seq1`.
    fn cross(&self, mut seq1: Seq, seq2: &mut Seq) -> Seq {
        if seq1.max_cross_len(seq2).map_or(false, |len| len > self.limit_total) {
            seq2.make_infinite();
        }
        if let ExtractKind::Suffix = self.kind {
            seq1.cross_reverse(seq2);
        } else {
            seq1.cross_forward(seq2);
        }
        assert!(seq1.len().map_or(true, | x | x <= self.limit_total));
        self.enforce_literal_len(&mut seq1);
        seq1
    }
    /// Union the two sequences if the result would be within configured
    /// limits. Otherwise, make `seq2` infinite and union the infinite sequence
    /// with `seq1`.
    fn union(&self, mut seq1: Seq, seq2: &mut Seq) -> Seq {
        if seq1.max_union_len(seq2).map_or(false, |len| len > self.limit_total) {
            match self.kind {
                ExtractKind::Prefix => {
                    seq1.keep_first_bytes(4);
                    seq2.keep_first_bytes(4);
                }
                ExtractKind::Suffix => {
                    seq1.keep_last_bytes(4);
                    seq2.keep_last_bytes(4);
                }
            }
            seq1.dedup();
            seq2.dedup();
            if seq1.max_union_len(seq2).map_or(false, |len| len > self.limit_total) {
                seq2.make_infinite();
            }
        }
        seq1.union(seq2);
        assert!(seq1.len().map_or(true, | x | x <= self.limit_total));
        seq1
    }
    /// Applies the literal length limit to the given sequence. If none of the
    /// literals in the sequence exceed the limit, then this is a no-op.
    fn enforce_literal_len(&self, seq: &mut Seq) {
        let len = self.limit_literal_len;
        match self.kind {
            ExtractKind::Prefix => seq.keep_first_bytes(len),
            ExtractKind::Suffix => seq.keep_last_bytes(len),
        }
    }
}
impl Default for Extractor {
    fn default() -> Extractor {
        Extractor::new()
    }
}
/// The kind of literals to extract from an [`Hir`] expression.
///
/// The default extraction kind is `Prefix`.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum ExtractKind {
    /// Extracts only prefix literals from a regex.
    Prefix,
    /// Extracts only suffix literals from a regex.
    ///
    /// Note that the sequence returned by suffix literals currently may
    /// not correctly represent leftmost-first or "preference" order match
    /// semantics.
    Suffix,
}
impl ExtractKind {
    /// Returns true if this kind is the `Prefix` variant.
    pub fn is_prefix(&self) -> bool {
        matches!(* self, ExtractKind::Prefix)
    }
    /// Returns true if this kind is the `Suffix` variant.
    pub fn is_suffix(&self) -> bool {
        matches!(* self, ExtractKind::Suffix)
    }
}
impl Default for ExtractKind {
    fn default() -> ExtractKind {
        ExtractKind::Prefix
    }
}
/// A sequence of literals.
///
/// A `Seq` is very much like a set in that it represents a union of its
/// members. That is, it corresponds to a set of literals where at least one
/// must match in order for a particular [`Hir`] expression to match. (Whether
/// this corresponds to the entire `Hir` expression, a prefix of it or a suffix
/// of it depends on how the `Seq` was extracted from the `Hir`.)
///
/// It is also unlike a set in that multiple identical literals may appear,
/// and that the order of the literals in the `Seq` matters. For example, if
/// the sequence is `[sam, samwise]` and leftmost-first matching is used, then
/// `samwise` can never match and the sequence is equivalent to `[sam]`.
///
/// # States of a sequence
///
/// A `Seq` has a few different logical states to consider:
///
/// * The sequence can represent "any" literal. When this happens, the set does
/// not have a finite size. The purpose of this state is to inhibit callers
/// from making assumptions about what literals are required in order to match
/// a particular [`Hir`] expression. Generally speaking, when a set is in this
/// state, literal optimizations are inhibited. A good example of a regex that
/// will cause this sort of set to apppear is `[A-Za-z]`. The character class
/// is just too big (and also too narrow) to be usefully expanded into 52
/// different literals. (Note that the decision for when a seq should become
/// infinite is determined by the caller. A seq itself has no hard-coded
/// limits.)
/// * The sequence can be empty, in which case, it is an affirmative statement
/// that there are no literals that can match the corresponding `Hir`.
/// Consequently, the `Hir` never matches any input. For example, `[a&&b]`.
/// * The sequence can be non-empty, in which case, at least one of the
/// literals must match in order for the corresponding `Hir` to match.
///
/// # Example
///
/// This example shows how literal sequences can be simplified by stripping
/// suffixes and minimizing while maintaining preference order.
///
/// ```
/// use regex_syntax::hir::literal::{Literal, Seq};
///
/// let mut seq = Seq::new(&[
///     "farm",
///     "appliance",
///     "faraway",
///     "apple",
///     "fare",
///     "gap",
///     "applicant",
///     "applaud",
/// ]);
/// seq.keep_first_bytes(3);
/// seq.minimize_by_preference();
/// // Notice that 'far' comes before 'app', which matches the order in the
/// // original sequence. This guarantees that leftmost-first semantics are
/// // not altered by simplifying the set.
/// let expected = Seq::from_iter([
///     Literal::inexact("far"),
///     Literal::inexact("app"),
///     Literal::exact("gap"),
/// ]);
/// assert_eq!(expected, seq);
/// ```
#[derive(Clone, Eq, PartialEq)]
pub struct Seq {
    /// The members of this seq.
    ///
    /// When `None`, the seq represents all possible literals. That is, it
    /// prevents one from making assumptions about specific literals in the
    /// seq, and forces one to treat it as if any literal might be in the seq.
    ///
    /// Note that `Some(vec![])` is valid and corresponds to the empty seq of
    /// literals, i.e., a regex that can never match. For example, `[a&&b]`.
    /// It is distinct from `Some(vec![""])`, which corresponds to the seq
    /// containing an empty string, which matches at every position.
    literals: Option<Vec<Literal>>,
}
impl Seq {
    /// Returns an empty sequence.
    ///
    /// An empty sequence matches zero literals, and thus corresponds to a
    /// regex that itself can never match.
    #[inline]
    pub fn empty() -> Seq {
        Seq { literals: Some(vec![]) }
    }
    /// Returns a sequence of literals without a finite size and may contain
    /// any literal.
    ///
    /// A sequence without finite size does not reveal anything about the
    /// characteristics of the literals in its set. There are no fixed prefixes
    /// or suffixes, nor are lower or upper bounds on the length of the literals
    /// in the set known.
    ///
    /// This is useful to represent constructs in a regex that are "too big"
    /// to useful represent as a sequence of literals. For example, `[A-Za-z]`.
    /// When sequences get too big, they lose their discriminating nature and
    /// are more likely to produce false positives, which in turn makes them
    /// less likely to speed up searches.
    ///
    /// More pragmatically, for many regexes, enumerating all possible literals
    /// is itself not possible or might otherwise use too many resources. So
    /// constraining the size of sets during extraction is a practical trade
    /// off to make.
    #[inline]
    pub fn infinite() -> Seq {
        Seq { literals: None }
    }
    /// Returns a sequence containing a single literal.
    #[inline]
    pub fn singleton(lit: Literal) -> Seq {
        Seq { literals: Some(vec![lit]) }
    }
    /// Returns a sequence of exact literals from the given byte strings.
    #[inline]
    pub fn new<I, B>(it: I) -> Seq
    where
        I: IntoIterator<Item = B>,
        B: AsRef<[u8]>,
    {
        it.into_iter().map(|b| Literal::exact(b.as_ref())).collect()
    }
    /// If this is a finite sequence, return its members as a slice of
    /// literals.
    ///
    /// The slice returned may be empty, in which case, there are no literals
    /// that can match this sequence.
    #[inline]
    pub fn literals(&self) -> Option<&[Literal]> {
        self.literals.as_deref()
    }
    /// Push a literal to the end of this sequence.
    ///
    /// If this sequence is not finite, then this is a no-op.
    ///
    /// Similarly, if the most recently added item of this sequence is
    /// equivalent to the literal given, then it is not added. This reflects
    /// a `Seq`'s "set like" behavior, and represents a practical trade off.
    /// Namely, there is never any need to have two adjacent and equivalent
    /// literals in the same sequence, _and_ it is easy to detect in some
    /// cases.
    #[inline]
    pub fn push(&mut self, lit: Literal) {
        let lits = match self.literals {
            None => return,
            Some(ref mut lits) => lits,
        };
        if lits.last().map_or(false, |m| m == &lit) {
            return;
        }
        lits.push(lit);
    }
    /// Make all of the literals in this sequence inexact.
    ///
    /// This is a no-op if this sequence is not finite.
    #[inline]
    pub fn make_inexact(&mut self) {
        let lits = match self.literals {
            None => return,
            Some(ref mut lits) => lits,
        };
        for lit in lits.iter_mut() {
            lit.make_inexact();
        }
    }
    /// Converts this sequence to an infinite sequence.
    ///
    /// This is a no-op if the sequence is already infinite.
    #[inline]
    pub fn make_infinite(&mut self) {
        self.literals = None;
    }
    /// Modify this sequence to contain the cross product between it and the
    /// sequence given.
    ///
    /// The cross product only considers literals in this sequence that are
    /// exact. That is, inexact literals are not extended.
    ///
    /// The literals are always drained from `other`, even if none are used.
    /// This permits callers to reuse the sequence allocation elsewhere.
    ///
    /// If this sequence is infinite, then this is a no-op, regardless of what
    /// `other` contains (and in this case, the literals are still drained from
    /// `other`). If `other` is infinite and this sequence is finite, then this
    /// is a no-op, unless this sequence contains a zero-length literal. In
    /// which case, the infiniteness of `other` infects this sequence, and this
    /// sequence is itself made infinite.
    ///
    /// Like [`Seq::union`], this may attempt to deduplicate literals. See
    /// [`Seq::dedup`] for how deduplication deals with exact and inexact
    /// literals.
    ///
    /// # Example
    ///
    /// This example shows basic usage and how exact and inexact literals
    /// interact.
    ///
    /// ```
    /// use regex_syntax::hir::literal::{Literal, Seq};
    ///
    /// let mut seq1 = Seq::from_iter([
    ///     Literal::exact("foo"),
    ///     Literal::inexact("bar"),
    /// ]);
    /// let mut seq2 = Seq::from_iter([
    ///     Literal::inexact("quux"),
    ///     Literal::exact("baz"),
    /// ]);
    /// seq1.cross_forward(&mut seq2);
    ///
    /// // The literals are pulled out of seq2.
    /// assert_eq!(Some(0), seq2.len());
    ///
    /// let expected = Seq::from_iter([
    ///     Literal::inexact("fooquux"),
    ///     Literal::exact("foobaz"),
    ///     Literal::inexact("bar"),
    /// ]);
    /// assert_eq!(expected, seq1);
    /// ```
    ///
    /// This example shows the behavior of when `other` is an infinite
    /// sequence.
    ///
    /// ```
    /// use regex_syntax::hir::literal::{Literal, Seq};
    ///
    /// let mut seq1 = Seq::from_iter([
    ///     Literal::exact("foo"),
    ///     Literal::inexact("bar"),
    /// ]);
    /// let mut seq2 = Seq::infinite();
    /// seq1.cross_forward(&mut seq2);
    ///
    /// // When seq2 is infinite, cross product doesn't add anything, but
    /// // ensures all members of seq1 are inexact.
    /// let expected = Seq::from_iter([
    ///     Literal::inexact("foo"),
    ///     Literal::inexact("bar"),
    /// ]);
    /// assert_eq!(expected, seq1);
    /// ```
    ///
    /// This example is like the one above, but shows what happens when this
    /// sequence contains an empty string. In this case, an infinite `other`
    /// sequence infects this sequence (because the empty string means that
    /// there are no finite prefixes):
    ///
    /// ```
    /// use regex_syntax::hir::literal::{Literal, Seq};
    ///
    /// let mut seq1 = Seq::from_iter([
    ///     Literal::exact("foo"),
    ///     Literal::exact(""), // inexact provokes same behavior
    ///     Literal::inexact("bar"),
    /// ]);
    /// let mut seq2 = Seq::infinite();
    /// seq1.cross_forward(&mut seq2);
    ///
    /// // seq1 is now infinite!
    /// assert!(!seq1.is_finite());
    /// ```
    ///
    /// This example shows the behavior of this sequence is infinite.
    ///
    /// ```
    /// use regex_syntax::hir::literal::{Literal, Seq};
    ///
    /// let mut seq1 = Seq::infinite();
    /// let mut seq2 = Seq::from_iter([
    ///     Literal::exact("foo"),
    ///     Literal::inexact("bar"),
    /// ]);
    /// seq1.cross_forward(&mut seq2);
    ///
    /// // seq1 remains unchanged.
    /// assert!(!seq1.is_finite());
    /// // Even though the literals in seq2 weren't used, it was still drained.
    /// assert_eq!(Some(0), seq2.len());
    /// ```
    #[inline]
    pub fn cross_forward(&mut self, other: &mut Seq) {
        let (lits1, lits2) = match self.cross_preamble(other) {
            None => return,
            Some((lits1, lits2)) => (lits1, lits2),
        };
        let newcap = lits1.len().saturating_mul(lits2.len());
        for selflit in mem::replace(lits1, Vec::with_capacity(newcap)) {
            if !selflit.is_exact() {
                lits1.push(selflit);
                continue;
            }
            for otherlit in lits2.iter() {
                let mut newlit = Literal::exact(
                    Vec::with_capacity(selflit.len() + otherlit.len()),
                );
                newlit.extend(&selflit);
                newlit.extend(&otherlit);
                if !otherlit.is_exact() {
                    newlit.make_inexact();
                }
                lits1.push(newlit);
            }
        }
        lits2.drain(..);
        self.dedup();
    }
    /// Modify this sequence to contain the cross product between it and
    /// the sequence given, where the sequences are treated as suffixes
    /// instead of prefixes. Namely, the sequence `other` is *prepended*
    /// to `self` (as opposed to `other` being *appended* to `self` in
    /// [`Seq::cross_forward`]).
    ///
    /// The cross product only considers literals in this sequence that are
    /// exact. That is, inexact literals are not extended.
    ///
    /// The literals are always drained from `other`, even if none are used.
    /// This permits callers to reuse the sequence allocation elsewhere.
    ///
    /// If this sequence is infinite, then this is a no-op, regardless of what
    /// `other` contains (and in this case, the literals are still drained from
    /// `other`). If `other` is infinite and this sequence is finite, then this
    /// is a no-op, unless this sequence contains a zero-length literal. In
    /// which case, the infiniteness of `other` infects this sequence, and this
    /// sequence is itself made infinite.
    ///
    /// Like [`Seq::union`], this may attempt to deduplicate literals. See
    /// [`Seq::dedup`] for how deduplication deals with exact and inexact
    /// literals.
    ///
    /// # Example
    ///
    /// This example shows basic usage and how exact and inexact literals
    /// interact.
    ///
    /// ```
    /// use regex_syntax::hir::literal::{Literal, Seq};
    ///
    /// let mut seq1 = Seq::from_iter([
    ///     Literal::exact("foo"),
    ///     Literal::inexact("bar"),
    /// ]);
    /// let mut seq2 = Seq::from_iter([
    ///     Literal::inexact("quux"),
    ///     Literal::exact("baz"),
    /// ]);
    /// seq1.cross_reverse(&mut seq2);
    ///
    /// // The literals are pulled out of seq2.
    /// assert_eq!(Some(0), seq2.len());
    ///
    /// let expected = Seq::from_iter([
    ///     Literal::inexact("quuxfoo"),
    ///     Literal::inexact("bar"),
    ///     Literal::exact("bazfoo"),
    /// ]);
    /// assert_eq!(expected, seq1);
    /// ```
    ///
    /// This example shows the behavior of when `other` is an infinite
    /// sequence.
    ///
    /// ```
    /// use regex_syntax::hir::literal::{Literal, Seq};
    ///
    /// let mut seq1 = Seq::from_iter([
    ///     Literal::exact("foo"),
    ///     Literal::inexact("bar"),
    /// ]);
    /// let mut seq2 = Seq::infinite();
    /// seq1.cross_reverse(&mut seq2);
    ///
    /// // When seq2 is infinite, cross product doesn't add anything, but
    /// // ensures all members of seq1 are inexact.
    /// let expected = Seq::from_iter([
    ///     Literal::inexact("foo"),
    ///     Literal::inexact("bar"),
    /// ]);
    /// assert_eq!(expected, seq1);
    /// ```
    ///
    /// This example is like the one above, but shows what happens when this
    /// sequence contains an empty string. In this case, an infinite `other`
    /// sequence infects this sequence (because the empty string means that
    /// there are no finite suffixes):
    ///
    /// ```
    /// use regex_syntax::hir::literal::{Literal, Seq};
    ///
    /// let mut seq1 = Seq::from_iter([
    ///     Literal::exact("foo"),
    ///     Literal::exact(""), // inexact provokes same behavior
    ///     Literal::inexact("bar"),
    /// ]);
    /// let mut seq2 = Seq::infinite();
    /// seq1.cross_reverse(&mut seq2);
    ///
    /// // seq1 is now infinite!
    /// assert!(!seq1.is_finite());
    /// ```
    ///
    /// This example shows the behavior when this sequence is infinite.
    ///
    /// ```
    /// use regex_syntax::hir::literal::{Literal, Seq};
    ///
    /// let mut seq1 = Seq::infinite();
    /// let mut seq2 = Seq::from_iter([
    ///     Literal::exact("foo"),
    ///     Literal::inexact("bar"),
    /// ]);
    /// seq1.cross_reverse(&mut seq2);
    ///
    /// // seq1 remains unchanged.
    /// assert!(!seq1.is_finite());
    /// // Even though the literals in seq2 weren't used, it was still drained.
    /// assert_eq!(Some(0), seq2.len());
    /// ```
    #[inline]
    pub fn cross_reverse(&mut self, other: &mut Seq) {
        let (lits1, lits2) = match self.cross_preamble(other) {
            None => return,
            Some((lits1, lits2)) => (lits1, lits2),
        };
        let newcap = lits1.len().saturating_mul(lits2.len());
        let selflits = mem::replace(lits1, Vec::with_capacity(newcap));
        for (i, otherlit) in lits2.drain(..).enumerate() {
            for selflit in selflits.iter() {
                if !selflit.is_exact() {
                    if i == 0 {
                        lits1.push(selflit.clone());
                    }
                    continue;
                }
                let mut newlit = Literal::exact(
                    Vec::with_capacity(otherlit.len() + selflit.len()),
                );
                newlit.extend(&otherlit);
                newlit.extend(&selflit);
                if !otherlit.is_exact() {
                    newlit.make_inexact();
                }
                lits1.push(newlit);
            }
        }
        self.dedup();
    }
    /// A helper function the corresponds to the subtle preamble for both
    /// `cross_forward` and `cross_reverse`. In effect, it handles the cases
    /// of infinite sequences for both `self` and `other`, as well as ensuring
    /// that literals from `other` are drained even if they aren't used.
    fn cross_preamble<'a>(
        &'a mut self,
        other: &'a mut Seq,
    ) -> Option<(&'a mut Vec<Literal>, &'a mut Vec<Literal>)> {
        let lits2 = match other.literals {
            None => {
                if self.min_literal_len() == Some(0) {
                    *self = Seq::infinite();
                } else {
                    self.make_inexact();
                }
                return None;
            }
            Some(ref mut lits) => lits,
        };
        let lits1 = match self.literals {
            None => {
                lits2.drain(..);
                return None;
            }
            Some(ref mut lits) => lits,
        };
        Some((lits1, lits2))
    }
    /// Unions the `other` sequence into this one.
    ///
    /// The literals are always drained out of the given `other` sequence,
    /// even if they are being unioned into an infinite sequence. This permits
    /// the caller to reuse the `other` sequence in another context.
    ///
    /// Some literal deduping may be performed. If any deduping happens,
    /// any leftmost-first or "preference" order match semantics will be
    /// preserved.
    ///
    /// # Example
    ///
    /// This example shows basic usage.
    ///
    /// ```
    /// use regex_syntax::hir::literal::Seq;
    ///
    /// let mut seq1 = Seq::new(&["foo", "bar"]);
    /// let mut seq2 = Seq::new(&["bar", "quux", "foo"]);
    /// seq1.union(&mut seq2);
    ///
    /// // The literals are pulled out of seq2.
    /// assert_eq!(Some(0), seq2.len());
    ///
    /// // Adjacent literals are deduped, but non-adjacent literals may not be.
    /// assert_eq!(Seq::new(&["foo", "bar", "quux", "foo"]), seq1);
    /// ```
    ///
    /// This example shows that literals are drained from `other` even when
    /// they aren't necessarily used.
    ///
    /// ```
    /// use regex_syntax::hir::literal::Seq;
    ///
    /// let mut seq1 = Seq::infinite();
    /// // Infinite sequences have no finite length.
    /// assert_eq!(None, seq1.len());
    ///
    /// let mut seq2 = Seq::new(&["bar", "quux", "foo"]);
    /// seq1.union(&mut seq2);
    ///
    /// // seq1 is still infinite and seq2 has been drained.
    /// assert_eq!(None, seq1.len());
    /// assert_eq!(Some(0), seq2.len());
    /// ```
    #[inline]
    pub fn union(&mut self, other: &mut Seq) {
        let lits2 = match other.literals {
            None => {
                self.make_infinite();
                return;
            }
            Some(ref mut lits) => lits.drain(..),
        };
        let lits1 = match self.literals {
            None => return,
            Some(ref mut lits) => lits,
        };
        lits1.extend(lits2);
        self.dedup();
    }
    /// Unions the `other` sequence into this one by splice the `other`
    /// sequence at the position of the first zero-length literal.
    ///
    /// This is useful for preserving preference order semantics when combining
    /// two literal sequences. For example, in the regex `(a||f)+foo`, the
    /// correct preference order prefix sequence is `[a, foo, f]`.
    ///
    /// The literals are always drained out of the given `other` sequence,
    /// even if they are being unioned into an infinite sequence. This permits
    /// the caller to reuse the `other` sequence in another context. Note that
    /// the literals are drained even if no union is performed as well, i.e.,
    /// when this sequence does not contain a zero-length literal.
    ///
    /// Some literal deduping may be performed. If any deduping happens,
    /// any leftmost-first or "preference" order match semantics will be
    /// preserved.
    ///
    /// # Example
    ///
    /// This example shows basic usage.
    ///
    /// ```
    /// use regex_syntax::hir::literal::Seq;
    ///
    /// let mut seq1 = Seq::new(&["a", "", "f", ""]);
    /// let mut seq2 = Seq::new(&["foo"]);
    /// seq1.union_into_empty(&mut seq2);
    ///
    /// // The literals are pulled out of seq2.
    /// assert_eq!(Some(0), seq2.len());
    /// // 'foo' gets spliced into seq1 where the first empty string occurs.
    /// assert_eq!(Seq::new(&["a", "foo", "f"]), seq1);
    /// ```
    ///
    /// This example shows that literals are drained from `other` even when
    /// they aren't necessarily used.
    ///
    /// ```
    /// use regex_syntax::hir::literal::Seq;
    ///
    /// let mut seq1 = Seq::new(&["foo", "bar"]);
    /// let mut seq2 = Seq::new(&["bar", "quux", "foo"]);
    /// seq1.union_into_empty(&mut seq2);
    ///
    /// // seq1 has no zero length literals, so no splicing happens.
    /// assert_eq!(Seq::new(&["foo", "bar"]), seq1);
    /// // Even though no splicing happens, seq2 is still drained.
    /// assert_eq!(Some(0), seq2.len());
    /// ```
    #[inline]
    pub fn union_into_empty(&mut self, other: &mut Seq) {
        let lits2 = other.literals.as_mut().map(|lits| lits.drain(..));
        let lits1 = match self.literals {
            None => return,
            Some(ref mut lits) => lits,
        };
        let first_empty = match lits1.iter().position(|m| m.is_empty()) {
            None => return,
            Some(i) => i,
        };
        let lits2 = match lits2 {
            None => {
                self.literals = None;
                return;
            }
            Some(lits) => lits,
        };
        lits1.retain(|m| !m.is_empty());
        lits1.splice(first_empty..first_empty, lits2);
        self.dedup();
    }
    /// Deduplicate adjacent equivalent literals in this sequence.
    ///
    /// If adjacent literals are equivalent strings but one is exact and the
    /// other inexact, the inexact literal is kept and the exact one is
    /// removed.
    ///
    /// Deduping an infinite sequence is a no-op.
    ///
    /// # Example
    ///
    /// This example shows how literals that are duplicate byte strings but
    /// are not equivalent with respect to exactness are resolved.
    ///
    /// ```
    /// use regex_syntax::hir::literal::{Literal, Seq};
    ///
    /// let mut seq = Seq::from_iter([
    ///     Literal::exact("foo"),
    ///     Literal::inexact("foo"),
    /// ]);
    /// seq.dedup();
    ///
    /// assert_eq!(Seq::from_iter([Literal::inexact("foo")]), seq);
    /// ```
    #[inline]
    pub fn dedup(&mut self) {
        if let Some(ref mut lits) = self.literals {
            lits.dedup_by(|lit1, lit2| {
                if lit1.as_bytes() != lit2.as_bytes() {
                    return false;
                }
                if lit1.is_exact() != lit2.is_exact() {
                    lit1.make_inexact();
                    lit2.make_inexact();
                }
                true
            });
        }
    }
    /// Sorts this sequence of literals lexicographically.
    ///
    /// Note that if, before sorting, if a literal that is a prefix of another
    /// literal appears after it, then after sorting, the sequence will not
    /// represent the same preference order match semantics. For example,
    /// sorting the sequence `[samwise, sam]` yields the sequence `[sam,
    /// samwise]`. Under preference order semantics, the latter sequence will
    /// never match `samwise` where as the first sequence can.
    ///
    /// # Example
    ///
    /// This example shows basic usage.
    ///
    /// ```
    /// use regex_syntax::hir::literal::Seq;
    ///
    /// let mut seq = Seq::new(&["foo", "quux", "bar"]);
    /// seq.sort();
    ///
    /// assert_eq!(Seq::new(&["bar", "foo", "quux"]), seq);
    /// ```
    #[inline]
    pub fn sort(&mut self) {
        if let Some(ref mut lits) = self.literals {
            lits.sort();
        }
    }
    /// Reverses all of the literals in this sequence.
    ///
    /// The order of the sequence itself is preserved.
    ///
    /// # Example
    ///
    /// This example shows basic usage.
    ///
    /// ```
    /// use regex_syntax::hir::literal::Seq;
    ///
    /// let mut seq = Seq::new(&["oof", "rab"]);
    /// seq.reverse_literals();
    /// assert_eq!(Seq::new(&["foo", "bar"]), seq);
    /// ```
    #[inline]
    pub fn reverse_literals(&mut self) {
        if let Some(ref mut lits) = self.literals {
            for lit in lits.iter_mut() {
                lit.reverse();
            }
        }
    }
    /// Shrinks this seq to its minimal size while respecting the preference
    /// order of its literals.
    ///
    /// While this routine will remove duplicate literals from this seq, it
    /// will also remove literals that can never match in a leftmost-first or
    /// "preference order" search. Similar to [`Seq::dedup`], if a literal is
    /// deduped, then the one that remains is made inexact.
    ///
    /// This is a no-op on seqs that are empty or not finite.
    ///
    /// # Example
    ///
    /// This example shows the difference between `{sam, samwise}` and
    /// `{samwise, sam}`.
    ///
    /// ```
    /// use regex_syntax::hir::literal::{Literal, Seq};
    ///
    /// // If 'sam' comes before 'samwise' and a preference order search is
    /// // executed, then 'samwise' can never match.
    /// let mut seq = Seq::new(&["sam", "samwise"]);
    /// seq.minimize_by_preference();
    /// assert_eq!(Seq::from_iter([Literal::inexact("sam")]), seq);
    ///
    /// // But if they are reversed, then it's possible for 'samwise' to match
    /// // since it is given higher preference.
    /// let mut seq = Seq::new(&["samwise", "sam"]);
    /// seq.minimize_by_preference();
    /// assert_eq!(Seq::new(&["samwise", "sam"]), seq);
    /// ```
    ///
    /// This example shows that if an empty string is in this seq, then
    /// anything that comes after it can never match.
    ///
    /// ```
    /// use regex_syntax::hir::literal::{Literal, Seq};
    ///
    /// // An empty string is a prefix of all strings, so it automatically
    /// // inhibits any subsequent strings from matching.
    /// let mut seq = Seq::new(&["foo", "bar", "", "quux", "fox"]);
    /// seq.minimize_by_preference();
    /// let expected = Seq::from_iter([
    ///     Literal::exact("foo"),
    ///     Literal::exact("bar"),
    ///     Literal::inexact(""),
    /// ]);
    /// assert_eq!(expected, seq);
    ///
    /// // And of course, if it's at the beginning, then it makes it impossible
    /// // for anything else to match.
    /// let mut seq = Seq::new(&["", "foo", "quux", "fox"]);
    /// seq.minimize_by_preference();
    /// assert_eq!(Seq::from_iter([Literal::inexact("")]), seq);
    /// ```
    #[inline]
    pub fn minimize_by_preference(&mut self) {
        if let Some(ref mut lits) = self.literals {
            PreferenceTrie::minimize(lits, false);
        }
    }
    /// Trims all literals in this seq such that only the first `len` bytes
    /// remain. If a literal has less than or equal to `len` bytes, then it
    /// remains unchanged. Otherwise, it is trimmed and made inexact.
    ///
    /// # Example
    ///
    /// ```
    /// use regex_syntax::hir::literal::{Literal, Seq};
    ///
    /// let mut seq = Seq::new(&["a", "foo", "quux"]);
    /// seq.keep_first_bytes(2);
    ///
    /// let expected = Seq::from_iter([
    ///     Literal::exact("a"),
    ///     Literal::inexact("fo"),
    ///     Literal::inexact("qu"),
    /// ]);
    /// assert_eq!(expected, seq);
    /// ```
    #[inline]
    pub fn keep_first_bytes(&mut self, len: usize) {
        if let Some(ref mut lits) = self.literals {
            for m in lits.iter_mut() {
                m.keep_first_bytes(len);
            }
        }
    }
    /// Trims all literals in this seq such that only the last `len` bytes
    /// remain. If a literal has less than or equal to `len` bytes, then it
    /// remains unchanged. Otherwise, it is trimmed and made inexact.
    ///
    /// # Example
    ///
    /// ```
    /// use regex_syntax::hir::literal::{Literal, Seq};
    ///
    /// let mut seq = Seq::new(&["a", "foo", "quux"]);
    /// seq.keep_last_bytes(2);
    ///
    /// let expected = Seq::from_iter([
    ///     Literal::exact("a"),
    ///     Literal::inexact("oo"),
    ///     Literal::inexact("ux"),
    /// ]);
    /// assert_eq!(expected, seq);
    /// ```
    #[inline]
    pub fn keep_last_bytes(&mut self, len: usize) {
        if let Some(ref mut lits) = self.literals {
            for m in lits.iter_mut() {
                m.keep_last_bytes(len);
            }
        }
    }
    /// Returns true if this sequence is finite.
    ///
    /// When false, this sequence is infinite and must be treated as if it
    /// contains every possible literal.
    #[inline]
    pub fn is_finite(&self) -> bool {
        self.literals.is_some()
    }
    /// Returns true if and only if this sequence is finite and empty.
    ///
    /// An empty sequence never matches anything. It can only be produced by
    /// literal extraction when the corresponding regex itself cannot match.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == Some(0)
    }
    /// Returns the number of literals in this sequence if the sequence is
    /// finite. If the sequence is infinite, then `None` is returned.
    #[inline]
    pub fn len(&self) -> Option<usize> {
        self.literals.as_ref().map(|lits| lits.len())
    }
    /// Returns true if and only if all literals in this sequence are exact.
    ///
    /// This returns false if the sequence is infinite.
    #[inline]
    pub fn is_exact(&self) -> bool {
        self.literals().map_or(false, |lits| lits.iter().all(|x| x.is_exact()))
    }
    /// Returns true if and only if all literals in this sequence are inexact.
    ///
    /// This returns true if the sequence is infinite.
    #[inline]
    pub fn is_inexact(&self) -> bool {
        self.literals().map_or(true, |lits| lits.iter().all(|x| !x.is_exact()))
    }
    /// Return the maximum length of the sequence that would result from
    /// unioning `self` with `other`. If either set is infinite, then this
    /// returns `None`.
    #[inline]
    fn max_union_len(&self, other: &Seq) -> Option<usize> {
        let len1 = self.len()?;
        let len2 = other.len()?;
        Some(len1.saturating_add(len2))
    }
    /// Return the maximum length of the sequence that would result from the
    /// cross product of `self` with `other`. If either set is infinite, then
    /// this returns `None`.
    #[inline]
    fn max_cross_len(&self, other: &Seq) -> Option<usize> {
        let len1 = self.len()?;
        let len2 = other.len()?;
        Some(len1.saturating_mul(len2))
    }
    /// Returns the length of the shortest literal in this sequence.
    ///
    /// If the sequence is infinite or empty, then this returns `None`.
    #[inline]
    pub fn min_literal_len(&self) -> Option<usize> {
        self.literals.as_ref()?.iter().map(|x| x.len()).min()
    }
    /// Returns the length of the longest literal in this sequence.
    ///
    /// If the sequence is infinite or empty, then this returns `None`.
    #[inline]
    pub fn max_literal_len(&self) -> Option<usize> {
        self.literals.as_ref()?.iter().map(|x| x.len()).max()
    }
    /// Returns the longest common prefix from this seq.
    ///
    /// If the seq matches any literal or other contains no literals, then
    /// there is no meaningful prefix and this returns `None`.
    ///
    /// # Example
    ///
    /// This shows some example seqs and their longest common prefix.
    ///
    /// ```
    /// use regex_syntax::hir::literal::Seq;
    ///
    /// let seq = Seq::new(&["foo", "foobar", "fo"]);
    /// assert_eq!(Some(&b"fo"[..]), seq.longest_common_prefix());
    /// let seq = Seq::new(&["foo", "foo"]);
    /// assert_eq!(Some(&b"foo"[..]), seq.longest_common_prefix());
    /// let seq = Seq::new(&["foo", "bar"]);
    /// assert_eq!(Some(&b""[..]), seq.longest_common_prefix());
    /// let seq = Seq::new(&[""]);
    /// assert_eq!(Some(&b""[..]), seq.longest_common_prefix());
    ///
    /// let seq = Seq::infinite();
    /// assert_eq!(None, seq.longest_common_prefix());
    /// let seq = Seq::empty();
    /// assert_eq!(None, seq.longest_common_prefix());
    /// ```
    #[inline]
    pub fn longest_common_prefix(&self) -> Option<&[u8]> {
        let lits = match self.literals {
            None => return None,
            Some(ref lits) => lits,
        };
        if lits.len() == 0 {
            return None;
        }
        let base = lits[0].as_bytes();
        let mut len = base.len();
        for m in lits.iter().skip(1) {
            len = m
                .as_bytes()
                .iter()
                .zip(base[..len].iter())
                .take_while(|&(a, b)| a == b)
                .count();
            if len == 0 {
                return Some(&[]);
            }
        }
        Some(&base[..len])
    }
    /// Returns the longest common suffix from this seq.
    ///
    /// If the seq matches any literal or other contains no literals, then
    /// there is no meaningful suffix and this returns `None`.
    ///
    /// # Example
    ///
    /// This shows some example seqs and their longest common suffix.
    ///
    /// ```
    /// use regex_syntax::hir::literal::Seq;
    ///
    /// let seq = Seq::new(&["oof", "raboof", "of"]);
    /// assert_eq!(Some(&b"of"[..]), seq.longest_common_suffix());
    /// let seq = Seq::new(&["foo", "foo"]);
    /// assert_eq!(Some(&b"foo"[..]), seq.longest_common_suffix());
    /// let seq = Seq::new(&["foo", "bar"]);
    /// assert_eq!(Some(&b""[..]), seq.longest_common_suffix());
    /// let seq = Seq::new(&[""]);
    /// assert_eq!(Some(&b""[..]), seq.longest_common_suffix());
    ///
    /// let seq = Seq::infinite();
    /// assert_eq!(None, seq.longest_common_suffix());
    /// let seq = Seq::empty();
    /// assert_eq!(None, seq.longest_common_suffix());
    /// ```
    #[inline]
    pub fn longest_common_suffix(&self) -> Option<&[u8]> {
        let lits = match self.literals {
            None => return None,
            Some(ref lits) => lits,
        };
        if lits.len() == 0 {
            return None;
        }
        let base = lits[0].as_bytes();
        let mut len = base.len();
        for m in lits.iter().skip(1) {
            len = m
                .as_bytes()
                .iter()
                .rev()
                .zip(base[base.len() - len..].iter().rev())
                .take_while(|&(a, b)| a == b)
                .count();
            if len == 0 {
                return Some(&[]);
            }
        }
        Some(&base[base.len() - len..])
    }
    /// Optimizes this seq while treating its literals as prefixes and
    /// respecting the preference order of its literals.
    ///
    /// The specific way "optimization" works is meant to be an implementation
    /// detail, as it essentially represents a set of heuristics. The goal
    /// that optimization tries to accomplish is to make the literals in this
    /// set reflect inputs that will result in a more effective prefilter.
    /// Principally by reducing the false positive rate of candidates found by
    /// the literals in this sequence. That is, when a match of a literal is
    /// found, we would like it to be a strong predictor of the overall match
    /// of the regex. If it isn't, then much time will be spent starting and
    /// stopping the prefilter search and attempting to confirm the match only
    /// to have it fail.
    ///
    /// Some of those heuristics might be:
    ///
    /// * Identifying a common prefix from a larger sequence of literals, and
    /// shrinking the sequence down to that single common prefix.
    /// * Rejecting the sequence entirely if it is believed to result in very
    /// high false positive rate. When this happens, the sequence is made
    /// infinite.
    /// * Shrinking the sequence to a smaller number of literals representing
    /// prefixes, but not shrinking it so much as to make literals too short.
    /// (A sequence with very short literals, of 1 or 2 bytes, will typically
    /// result in a higher false positive rate.)
    ///
    /// Optimization should only be run once extraction is complete. Namely,
    /// optimization may make assumptions that do not compose with other
    /// operations in the middle of extraction. For example, optimization will
    /// reduce `[E(sam), E(samwise)]` to `[E(sam)]`, but such a transformation
    /// is only valid if no other extraction will occur. If other extraction
    /// may occur, then the correct transformation would be to `[I(sam)]`.
    ///
    /// The [`Seq::optimize_for_suffix_by_preference`] does the same thing, but
    /// for suffixes.
    ///
    /// # Example
    ///
    /// This shows how optimization might transform a sequence. Note that
    /// the specific behavior is not a documented guarantee. The heuristics
    /// used are an implementation detail and may change over time in semver
    /// compatible releases.
    ///
    /// ```
    /// use regex_syntax::hir::literal::{Seq, Literal};
    ///
    /// let mut seq = Seq::new(&[
    ///     "samantha",
    ///     "sam",
    ///     "samwise",
    ///     "frodo",
    /// ]);
    /// seq.optimize_for_prefix_by_preference();
    /// assert_eq!(Seq::from_iter([
    ///     Literal::exact("samantha"),
    ///     // Kept exact even though 'samwise' got pruned
    ///     // because optimization assumes literal extraction
    ///     // has finished.
    ///     Literal::exact("sam"),
    ///     Literal::exact("frodo"),
    /// ]), seq);
    /// ```
    ///
    /// # Example: optimization may make the sequence infinite
    ///
    /// If the heuristics deem that the sequence could cause a very high false
    /// positive rate, then it may make the sequence infinite, effectively
    /// disabling its use as a prefilter.
    ///
    /// ```
    /// use regex_syntax::hir::literal::{Seq, Literal};
    ///
    /// let mut seq = Seq::new(&[
    ///     "samantha",
    ///     // An empty string matches at every position,
    ///     // thus rendering the prefilter completely
    ///     // ineffective.
    ///     "",
    ///     "sam",
    ///     "samwise",
    ///     "frodo",
    /// ]);
    /// seq.optimize_for_prefix_by_preference();
    /// assert!(!seq.is_finite());
    /// ```
    ///
    /// Do note that just because there is a `" "` in the sequence, that
    /// doesn't mean the sequence will always be made infinite after it is
    /// optimized. Namely, if the sequence is considered exact (any match
    /// corresponds to an overall match of the original regex), then any match
    /// is an overall match, and so the false positive rate is always `0`.
    ///
    /// To demonstrate this, we remove `samwise` from our sequence. This
    /// results in no optimization happening and all literals remain exact.
    /// Thus the entire sequence is exact, and it is kept as-is, even though
    /// one is an ASCII space:
    ///
    /// ```
    /// use regex_syntax::hir::literal::{Seq, Literal};
    ///
    /// let mut seq = Seq::new(&[
    ///     "samantha",
    ///     " ",
    ///     "sam",
    ///     "frodo",
    /// ]);
    /// seq.optimize_for_prefix_by_preference();
    /// assert!(seq.is_finite());
    /// ```
    #[inline]
    pub fn optimize_for_prefix_by_preference(&mut self) {
        self.optimize_by_preference(true);
    }
    /// Optimizes this seq while treating its literals as suffixes and
    /// respecting the preference order of its literals.
    ///
    /// Optimization should only be run once extraction is complete.
    ///
    /// The [`Seq::optimize_for_prefix_by_preference`] does the same thing, but
    /// for prefixes. See its documentation for more explanation.
    #[inline]
    pub fn optimize_for_suffix_by_preference(&mut self) {
        self.optimize_by_preference(false);
    }
    fn optimize_by_preference(&mut self, prefix: bool) {
        let origlen = match self.len() {
            None => return,
            Some(len) => len,
        };
        if prefix {
            if let Some(ref mut lits) = self.literals {
                PreferenceTrie::minimize(lits, true);
            }
        }
        let fix = if prefix {
            self.longest_common_prefix()
        } else {
            self.longest_common_suffix()
        };
        if let Some(fix) = fix {
            if prefix && origlen > 1 && fix.len() >= 1 && fix.len() <= 3
                && rank(fix[0]) < 200
            {
                self.keep_first_bytes(1);
                self.dedup();
                return;
            }
            let isfast = self.is_exact() && self.len().map_or(false, |len| len <= 16);
            let usefix = fix.len() > 4 || (fix.len() > 1 && !isfast);
            if usefix {
                if prefix {
                    self.keep_first_bytes(fix.len());
                } else {
                    self.keep_last_bytes(fix.len());
                }
                self.dedup();
                assert_eq!(Some(1), self.len());
            }
        }
        if self.is_exact() && self.min_literal_len().map_or(false, |len| len > 0) {
            return;
        }
        const ATTEMPTS: [(usize, usize); 5] = [
            (5, 64),
            (4, 64),
            (3, 64),
            (2, 64),
            (1, 10),
        ];
        for (keep, limit) in ATTEMPTS {
            let len = match self.len() {
                None => break,
                Some(len) => len,
            };
            if len <= limit {
                break;
            }
            if prefix {
                self.keep_first_bytes(keep);
            } else {
                self.keep_last_bytes(keep);
            }
            self.minimize_by_preference();
        }
        if let Some(lits) = self.literals() {
            if lits.iter().any(|lit| lit.is_poisonous()) {
                self.make_infinite();
            }
        }
    }
}
impl core::fmt::Debug for Seq {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "Seq")?;
        if let Some(lits) = self.literals() {
            f.debug_list().entries(lits.iter()).finish()
        } else {
            write!(f, "[∅]")
        }
    }
}
impl FromIterator<Literal> for Seq {
    fn from_iter<T: IntoIterator<Item = Literal>>(it: T) -> Seq {
        let mut seq = Seq::empty();
        for literal in it {
            seq.push(literal);
        }
        seq
    }
}
/// A single literal extracted from an [`Hir`] expression.
///
/// A literal is composed of two things:
///
/// * A sequence of bytes. No guarantees with respect to UTF-8 are provided.
/// In particular, even if the regex a literal is extracted from is UTF-8, the
/// literal extracted may not be valid UTF-8. (For example, if an [`Extractor`]
/// limit resulted in trimming a literal in a way that splits a codepoint.)
/// * Whether the literal is "exact" or not. An "exact" literal means that it
/// has not been trimmed, and may continue to be extended. If a literal is
/// "exact" after visiting the entire `Hir` expression, then this implies that
/// the literal leads to a match state. (Although it doesn't necessarily imply
/// all occurrences of the literal correspond to a match of the regex, since
/// literal extraction ignores look-around assertions.)
#[derive(Clone, Eq, PartialEq, PartialOrd, Ord)]
pub struct Literal {
    bytes: Vec<u8>,
    exact: bool,
}
impl Literal {
    /// Returns a new exact literal containing the bytes given.
    #[inline]
    pub fn exact<B: Into<Vec<u8>>>(bytes: B) -> Literal {
        Literal {
            bytes: bytes.into(),
            exact: true,
        }
    }
    /// Returns a new inexact literal containing the bytes given.
    #[inline]
    pub fn inexact<B: Into<Vec<u8>>>(bytes: B) -> Literal {
        Literal {
            bytes: bytes.into(),
            exact: false,
        }
    }
    /// Returns the bytes in this literal.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
    /// Yields ownership of the bytes inside this literal.
    ///
    /// Note that this throws away whether the literal is "exact" or not.
    #[inline]
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
    /// Returns the length of this literal in bytes.
    #[inline]
    pub fn len(&self) -> usize {
        self.as_bytes().len()
    }
    /// Returns true if and only if this literal has zero bytes.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Returns true if and only if this literal is exact.
    #[inline]
    pub fn is_exact(&self) -> bool {
        self.exact
    }
    /// Marks this literal as inexact.
    ///
    /// Inexact literals can never be extended. For example,
    /// [`Seq::cross_forward`] will not extend inexact literals.
    #[inline]
    pub fn make_inexact(&mut self) {
        self.exact = false;
    }
    /// Reverse the bytes in this literal.
    #[inline]
    pub fn reverse(&mut self) {
        self.bytes.reverse();
    }
    /// Extend this literal with the literal given.
    ///
    /// If this literal is inexact, then this is a no-op.
    #[inline]
    pub fn extend(&mut self, lit: &Literal) {
        if !self.is_exact() {
            return;
        }
        self.bytes.extend_from_slice(&lit.bytes);
    }
    /// Trims this literal such that only the first `len` bytes remain. If
    /// this literal has fewer than `len` bytes, then it remains unchanged.
    /// Otherwise, the literal is marked as inexact.
    #[inline]
    pub fn keep_first_bytes(&mut self, len: usize) {
        if len >= self.len() {
            return;
        }
        self.make_inexact();
        self.bytes.truncate(len);
    }
    /// Trims this literal such that only the last `len` bytes remain. If this
    /// literal has fewer than `len` bytes, then it remains unchanged.
    /// Otherwise, the literal is marked as inexact.
    #[inline]
    pub fn keep_last_bytes(&mut self, len: usize) {
        if len >= self.len() {
            return;
        }
        self.make_inexact();
        self.bytes.drain(..self.len() - len);
    }
    /// Returns true if it is believe that this literal is likely to match very
    /// frequently, and is thus not a good candidate for a prefilter.
    fn is_poisonous(&self) -> bool {
        self.is_empty() || (self.len() == 1 && rank(self.as_bytes()[0]) >= 250)
    }
}
impl From<u8> for Literal {
    fn from(byte: u8) -> Literal {
        Literal::exact(vec![byte])
    }
}
impl From<char> for Literal {
    fn from(ch: char) -> Literal {
        use alloc::string::ToString;
        Literal::exact(ch.encode_utf8(&mut [0; 4]).to_string())
    }
}
impl AsRef<[u8]> for Literal {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}
impl core::fmt::Debug for Literal {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let tag = if self.exact { "E" } else { "I" };
        f.debug_tuple(tag).field(&crate::debug::Bytes(self.as_bytes())).finish()
    }
}
/// A "preference" trie that rejects literals that will never match when
/// executing a leftmost first or "preference" search.
///
/// For example, if 'sam' is inserted, then trying to insert 'samwise' will be
/// rejected because 'samwise' can never match since 'sam' will always take
/// priority. However, if 'samwise' is inserted first, then inserting 'sam'
/// after it is accepted. In this case, either 'samwise' or 'sam' can match in
/// a "preference" search.
///
/// Note that we only use this trie as a "set." That is, given a sequence of
/// literals, we insert each one in order. An `insert` will reject a literal
/// if a prefix of that literal already exists in the trie. Thus, to rebuild
/// the "minimal" sequence, we simply only keep literals that were successfully
/// inserted. (Since we don't need traversal, one wonders whether we can make
/// some simplifications here, but I haven't given it a ton of thought and I've
/// never seen this show up on a profile. Because of the heuristic limits
/// imposed on literal extractions, the size of the inputs here is usually
/// very small.)
#[derive(Debug, Default)]
struct PreferenceTrie {
    /// The states in this trie. The index of a state in this vector is its ID.
    states: Vec<State>,
    /// The index to allocate to the next literal added to this trie. Starts at
    /// 0 and increments by 1 for every literal successfully added to the trie.
    next_literal_index: usize,
}
/// A single state in a trie. Uses a sparse representation for its transitions.
#[derive(Debug, Default)]
struct State {
    /// Sparse representation of the transitions out of this state. Transitions
    /// are sorted by byte. There is at most one such transition for any
    /// particular byte.
    trans: Vec<(u8, usize)>,
    /// Whether this is a matching state or not. If it is, then it contains the
    /// index to the matching literal.
    literal_index: Option<usize>,
}
impl PreferenceTrie {
    /// Minimizes the given sequence of literals while preserving preference
    /// order semantics.
    ///
    /// When `keep_exact` is true, the exactness of every literal retained is
    /// kept. This is useful when dealing with a fully extracted `Seq` that
    /// only contains exact literals. In that case, we can keep all retained
    /// literals as exact because we know we'll never need to match anything
    /// after them and because any removed literals are guaranteed to never
    /// match.
    fn minimize(literals: &mut Vec<Literal>, keep_exact: bool) {
        use core::cell::RefCell;
        let trie = RefCell::new(PreferenceTrie::default());
        let mut make_inexact = vec![];
        literals
            .retain(|lit| {
                match trie.borrow_mut().insert(lit.as_bytes()) {
                    Ok(_) => true,
                    Err(i) => {
                        if !keep_exact {
                            make_inexact.push(i);
                        }
                        false
                    }
                }
            });
        for i in make_inexact {
            literals[i].make_inexact();
        }
    }
    /// Returns `Ok` if the given byte string is accepted into this trie and
    /// `Err` otherwise. The index for the success case corresponds to the
    /// index of the literal added. The index for the error case corresponds to
    /// the index of the literal already in the trie that prevented the given
    /// byte string from being added. (Which implies it is a prefix of the one
    /// given.)
    ///
    /// In short, the byte string given is accepted into the trie if and only
    /// if it is possible for it to match when executing a preference order
    /// search.
    fn insert(&mut self, bytes: &[u8]) -> Result<usize, usize> {
        let mut prev = self.root();
        if let Some(idx) = self.states[prev].literal_index {
            return Err(idx);
        }
        for &b in bytes.iter() {
            match self.states[prev].trans.binary_search_by_key(&b, |t| t.0) {
                Ok(i) => {
                    prev = self.states[prev].trans[i].1;
                    if let Some(idx) = self.states[prev].literal_index {
                        return Err(idx);
                    }
                }
                Err(i) => {
                    let next = self.create_state();
                    self.states[prev].trans.insert(i, (b, next));
                    prev = next;
                }
            }
        }
        let idx = self.next_literal_index;
        self.next_literal_index += 1;
        self.states[prev].literal_index = Some(idx);
        Ok(idx)
    }
    /// Returns the root state ID, and if it doesn't exist, creates it.
    fn root(&mut self) -> usize {
        if !self.states.is_empty() { 0 } else { self.create_state() }
    }
    /// Creates a new empty state and returns its ID.
    fn create_state(&mut self) -> usize {
        let id = self.states.len();
        self.states.push(State::default());
        id
    }
}
/// Returns the "rank" of the given byte.
///
/// The minimum rank value is `0` and the maximum rank value is `255`.
///
/// The rank of a byte is derived from a heuristic background distribution of
/// relative frequencies of bytes. The heuristic says that lower the rank of a
/// byte, the less likely that byte is to appear in any arbitrary haystack.
pub fn rank(byte: u8) -> u8 {
    crate::rank::BYTE_FREQUENCIES[usize::from(byte)]
}
#[cfg(test)]
mod tests {
    use super::*;
    fn parse(pattern: &str) -> Hir {
        crate::ParserBuilder::new().utf8(false).build().parse(pattern).unwrap()
    }
    fn prefixes(pattern: &str) -> Seq {
        Extractor::new().kind(ExtractKind::Prefix).extract(&parse(pattern))
    }
    fn suffixes(pattern: &str) -> Seq {
        Extractor::new().kind(ExtractKind::Suffix).extract(&parse(pattern))
    }
    fn e(pattern: &str) -> (Seq, Seq) {
        (prefixes(pattern), suffixes(pattern))
    }
    #[allow(non_snake_case)]
    fn E(x: &str) -> Literal {
        Literal::exact(x.as_bytes())
    }
    #[allow(non_snake_case)]
    fn I(x: &str) -> Literal {
        Literal::inexact(x.as_bytes())
    }
    fn seq<I: IntoIterator<Item = Literal>>(it: I) -> Seq {
        Seq::from_iter(it)
    }
    fn infinite() -> (Seq, Seq) {
        (Seq::infinite(), Seq::infinite())
    }
    fn inexact<I1, I2>(it1: I1, it2: I2) -> (Seq, Seq)
    where
        I1: IntoIterator<Item = Literal>,
        I2: IntoIterator<Item = Literal>,
    {
        (Seq::from_iter(it1), Seq::from_iter(it2))
    }
    fn exact<B: AsRef<[u8]>, I: IntoIterator<Item = B>>(it: I) -> (Seq, Seq) {
        let s1 = Seq::new(it);
        let s2 = s1.clone();
        (s1, s2)
    }
    fn opt<B: AsRef<[u8]>, I: IntoIterator<Item = B>>(it: I) -> (Seq, Seq) {
        let (mut p, mut s) = exact(it);
        p.optimize_for_prefix_by_preference();
        s.optimize_for_suffix_by_preference();
        (p, s)
    }
    #[test]
    fn literal() {
        assert_eq!(exact(["a"]), e("a"));
        assert_eq!(exact(["aaaaa"]), e("aaaaa"));
        assert_eq!(exact(["A", "a"]), e("(?i-u)a"));
        assert_eq!(exact(["AB", "Ab", "aB", "ab"]), e("(?i-u)ab"));
        assert_eq!(exact(["abC", "abc"]), e("ab(?i-u)c"));
        assert_eq!(exact([b"\xFF"]), e(r"(?-u:\xFF)"));
        #[cfg(feature = "unicode-case")]
        {
            assert_eq!(exact(["☃"]), e("☃"));
            assert_eq!(exact(["☃"]), e("(?i)☃"));
            assert_eq!(exact(["☃☃☃☃☃"]), e("☃☃☃☃☃"));
            assert_eq!(exact(["Δ"]), e("Δ"));
            assert_eq!(exact(["δ"]), e("δ"));
            assert_eq!(exact(["Δ", "δ"]), e("(?i)Δ"));
            assert_eq!(exact(["Δ", "δ"]), e("(?i)δ"));
            assert_eq!(exact(["S", "s", "ſ"]), e("(?i)S"));
            assert_eq!(exact(["S", "s", "ſ"]), e("(?i)s"));
            assert_eq!(exact(["S", "s", "ſ"]), e("(?i)ſ"));
        }
        let letters = "ͱͳͷΐάέήίΰαβγδεζηθικλμνξοπρςστυφχψωϊϋ";
        assert_eq!(exact([letters]), e(letters));
    }
    #[test]
    fn class() {
        assert_eq!(exact(["a", "b", "c"]), e("[abc]"));
        assert_eq!(exact(["a1b", "a2b", "a3b"]), e("a[123]b"));
        assert_eq!(exact(["δ", "ε"]), e("[εδ]"));
        #[cfg(feature = "unicode-case")]
        {
            assert_eq!(exact(["Δ", "Ε", "δ", "ε", "ϵ"]), e(r"(?i)[εδ]"));
        }
    }
    #[test]
    fn look() {
        assert_eq!(exact(["ab"]), e(r"a\Ab"));
        assert_eq!(exact(["ab"]), e(r"a\zb"));
        assert_eq!(exact(["ab"]), e(r"a(?m:^)b"));
        assert_eq!(exact(["ab"]), e(r"a(?m:$)b"));
        assert_eq!(exact(["ab"]), e(r"a\bb"));
        assert_eq!(exact(["ab"]), e(r"a\Bb"));
        assert_eq!(exact(["ab"]), e(r"a(?-u:\b)b"));
        assert_eq!(exact(["ab"]), e(r"a(?-u:\B)b"));
        assert_eq!(exact(["ab"]), e(r"^ab"));
        assert_eq!(exact(["ab"]), e(r"$ab"));
        assert_eq!(exact(["ab"]), e(r"(?m:^)ab"));
        assert_eq!(exact(["ab"]), e(r"(?m:$)ab"));
        assert_eq!(exact(["ab"]), e(r"\bab"));
        assert_eq!(exact(["ab"]), e(r"\Bab"));
        assert_eq!(exact(["ab"]), e(r"(?-u:\b)ab"));
        assert_eq!(exact(["ab"]), e(r"(?-u:\B)ab"));
        assert_eq!(exact(["ab"]), e(r"ab^"));
        assert_eq!(exact(["ab"]), e(r"ab$"));
        assert_eq!(exact(["ab"]), e(r"ab(?m:^)"));
        assert_eq!(exact(["ab"]), e(r"ab(?m:$)"));
        assert_eq!(exact(["ab"]), e(r"ab\b"));
        assert_eq!(exact(["ab"]), e(r"ab\B"));
        assert_eq!(exact(["ab"]), e(r"ab(?-u:\b)"));
        assert_eq!(exact(["ab"]), e(r"ab(?-u:\B)"));
        let expected = (seq([I("aZ"), E("ab")]), seq([I("Zb"), E("ab")]));
        assert_eq!(expected, e(r"^aZ*b"));
    }
    #[test]
    fn repetition() {
        assert_eq!(exact(["a", ""]), e(r"a?"));
        assert_eq!(exact(["", "a"]), e(r"a??"));
        assert_eq!(inexact([I("a"), E("")], [I("a"), E("")]), e(r"a*"));
        assert_eq!(inexact([E(""), I("a")], [E(""), I("a")]), e(r"a*?"));
        assert_eq!(inexact([I("a")], [I("a")]), e(r"a+"));
        assert_eq!(inexact([I("a")], [I("a")]), e(r"(a+)+"));
        assert_eq!(exact(["ab"]), e(r"aZ{0}b"));
        assert_eq!(exact(["aZb", "ab"]), e(r"aZ?b"));
        assert_eq!(exact(["ab", "aZb"]), e(r"aZ??b"));
        assert_eq!(inexact([I("aZ"), E("ab")], [I("Zb"), E("ab")]), e(r"aZ*b"));
        assert_eq!(inexact([E("ab"), I("aZ")], [E("ab"), I("Zb")]), e(r"aZ*?b"));
        assert_eq!(inexact([I("aZ")], [I("Zb")]), e(r"aZ+b"));
        assert_eq!(inexact([I("aZ")], [I("Zb")]), e(r"aZ+?b"));
        assert_eq!(exact(["aZZb"]), e(r"aZ{2}b"));
        assert_eq!(inexact([I("aZZ")], [I("ZZb")]), e(r"aZ{2,3}b"));
        assert_eq!(exact(["abc", ""]), e(r"(abc)?"));
        assert_eq!(exact(["", "abc"]), e(r"(abc)??"));
        assert_eq!(inexact([I("a"), E("b")], [I("ab"), E("b")]), e(r"a*b"));
        assert_eq!(inexact([E("b"), I("a")], [E("b"), I("ab")]), e(r"a*?b"));
        assert_eq!(inexact([I("ab")], [I("b")]), e(r"ab+"));
        assert_eq!(inexact([I("a"), I("b")], [I("b")]), e(r"a*b+"));
        assert_eq!(
            inexact([I("a"), I("b"), E("c")], [I("bc"), I("ac"), E("c")]), e(r"a*b*c")
        );
        assert_eq!(
            inexact([I("a"), I("b"), E("c")], [I("bc"), I("ac"), E("c")]),
            e(r"(a+)?(b+)?c")
        );
        assert_eq!(
            inexact([I("a"), I("b"), E("c")], [I("bc"), I("ac"), E("c")]),
            e(r"(a+|)(b+|)c")
        );
        assert_eq!(
            inexact([I("a"), I("b"), I("c"), E("")], [I("c"), I("b"), I("a"), E("")]),
            e(r"a*b*c*")
        );
        assert_eq!(inexact([I("a"), I("b"), I("c")], [I("c")]), e(r"a*b*c+"));
        assert_eq!(inexact([I("a"), I("b")], [I("bc")]), e(r"a*b+c"));
        assert_eq!(inexact([I("a"), I("b")], [I("c"), I("b")]), e(r"a*b+c*"));
        assert_eq!(inexact([I("ab"), E("a")], [I("b"), E("a")]), e(r"ab*"));
        assert_eq!(inexact([I("ab"), E("ac")], [I("bc"), E("ac")]), e(r"ab*c"));
        assert_eq!(inexact([I("ab")], [I("b")]), e(r"ab+"));
        assert_eq!(inexact([I("ab")], [I("bc")]), e(r"ab+c"));
        assert_eq!(inexact([I("z"), E("azb")], [I("zazb"), E("azb")]), e(r"z*azb"));
        let expected = exact(["aaa", "aab", "aba", "abb", "baa", "bab", "bba", "bbb"]);
        assert_eq!(expected, e(r"[ab]{3}"));
        let expected = inexact(
            [
                I("aaa"),
                I("aab"),
                I("aba"),
                I("abb"),
                I("baa"),
                I("bab"),
                I("bba"),
                I("bbb"),
            ],
            [
                I("aaa"),
                I("aab"),
                I("aba"),
                I("abb"),
                I("baa"),
                I("bab"),
                I("bba"),
                I("bbb"),
            ],
        );
        assert_eq!(expected, e(r"[ab]{3,4}"));
    }
    #[test]
    fn concat() {
        let empty: [&str; 0] = [];
        assert_eq!(exact(["abcxyz"]), e(r"abc()xyz"));
        assert_eq!(exact(["abcxyz"]), e(r"(abc)(xyz)"));
        assert_eq!(exact(["abcmnoxyz"]), e(r"abc()mno()xyz"));
        assert_eq!(exact(empty), e(r"abc[a&&b]xyz"));
        assert_eq!(exact(["abcxyz"]), e(r"abc[a&&b]*xyz"));
    }
    #[test]
    fn alternation() {
        assert_eq!(exact(["abc", "mno", "xyz"]), e(r"abc|mno|xyz"));
        assert_eq!(
            inexact([E("abc"), I("mZ"), E("mo"), E("xyz")], [E("abc"), I("Zo"), E("mo"),
            E("xyz")]), e(r"abc|mZ*o|xyz")
        );
        assert_eq!(exact(["abc", "xyz"]), e(r"abc|M[a&&b]N|xyz"));
        assert_eq!(exact(["abc", "MN", "xyz"]), e(r"abc|M[a&&b]*N|xyz"));
        assert_eq!(exact(["aaa", "aaaaa"]), e(r"(?:|aa)aaa"));
        assert_eq!(
            inexact([I("aaa"), E(""), I("aaaaa"), E("aa")], [I("aaa"), E(""), E("aa")]),
            e(r"(?:|aa)(?:aaa)*")
        );
        assert_eq!(
            inexact([E(""), I("aaa"), E("aa"), I("aaaaa")], [E(""), I("aaa"), E("aa")]),
            e(r"(?:|aa)(?:aaa)*?")
        );
        assert_eq!(
            inexact([E("a"), I("b"), E("")], [E("a"), I("b"), E("")]), e(r"a|b*")
        );
        assert_eq!(inexact([E("a"), I("b")], [E("a"), I("b")]), e(r"a|b+"));
        assert_eq!(
            inexact([I("a"), E("b"), E("c")], [I("ab"), E("b"), E("c")]), e(r"a*b|c")
        );
        assert_eq!(
            inexact([E("a"), E("b"), I("c"), E("")], [E("a"), E("b"), I("c"), E("")]),
            e(r"a|(?:b|c*)")
        );
        assert_eq!(
            inexact([I("a"), I("b"), E("c"), I("a"), I("ab"), E("c")], [I("ac"), I("bc"),
            E("c"), I("ac"), I("abc"), E("c")],), e(r"(a|b)*c|(a|ab)*c")
        );
        assert_eq!(exact(["abef", "abgh", "cdef", "cdgh"]), e(r"(ab|cd)(ef|gh)"));
        assert_eq!(
            exact(["abefij", "abefkl", "abghij", "abghkl", "cdefij", "cdefkl", "cdghij",
            "cdghkl",]), e(r"(ab|cd)(ef|gh)(ij|kl)")
        );
    }
    #[test]
    fn impossible() {
        let empty: [&str; 0] = [];
        assert_eq!(exact(empty), e(r"[a&&b]"));
        assert_eq!(exact(empty), e(r"a[a&&b]"));
        assert_eq!(exact(empty), e(r"[a&&b]b"));
        assert_eq!(exact(empty), e(r"a[a&&b]b"));
        assert_eq!(exact(["a", "b"]), e(r"a|[a&&b]|b"));
        assert_eq!(exact(["a", "b"]), e(r"a|c[a&&b]|b"));
        assert_eq!(exact(["a", "b"]), e(r"a|[a&&b]d|b"));
        assert_eq!(exact(["a", "b"]), e(r"a|c[a&&b]d|b"));
        assert_eq!(exact([""]), e(r"[a&&b]*"));
        assert_eq!(exact(["MN"]), e(r"M[a&&b]*N"));
    }
    #[test]
    fn anything() {
        assert_eq!(infinite(), e(r"."));
        assert_eq!(infinite(), e(r"(?s)."));
        assert_eq!(infinite(), e(r"[A-Za-z]"));
        assert_eq!(infinite(), e(r"[A-Z]"));
        assert_eq!(exact([""]), e(r"[A-Z]{0}"));
        assert_eq!(infinite(), e(r"[A-Z]?"));
        assert_eq!(infinite(), e(r"[A-Z]*"));
        assert_eq!(infinite(), e(r"[A-Z]+"));
        assert_eq!((seq([I("1")]), Seq::infinite()), e(r"1[A-Z]"));
        assert_eq!((seq([I("1")]), seq([I("2")])), e(r"1[A-Z]2"));
        assert_eq!((Seq::infinite(), seq([I("123")])), e(r"[A-Z]+123"));
        assert_eq!(infinite(), e(r"[A-Z]+123[A-Z]+"));
        assert_eq!(infinite(), e(r"1|[A-Z]|3"));
        assert_eq!((seq([E("1"), I("2"), E("3")]), Seq::infinite()), e(r"1|2[A-Z]|3"),);
        assert_eq!((Seq::infinite(), seq([E("1"), I("2"), E("3")])), e(r"1|[A-Z]2|3"),);
        assert_eq!(
            (seq([E("1"), I("2"), E("4")]), seq([E("1"), I("3"), E("4")])),
            e(r"1|2[A-Z]3|4"),
        );
        assert_eq!((Seq::infinite(), seq([I("2")])), e(r"(?:|1)[A-Z]2"));
        assert_eq!(inexact([I("a")], [I("z")]), e(r"a.z"));
    }
    #[test]
    fn anything_small_limits() {
        fn prefixes(pattern: &str) -> Seq {
            Extractor::new()
                .kind(ExtractKind::Prefix)
                .limit_total(10)
                .extract(&parse(pattern))
        }
        fn suffixes(pattern: &str) -> Seq {
            Extractor::new()
                .kind(ExtractKind::Suffix)
                .limit_total(10)
                .extract(&parse(pattern))
        }
        fn e(pattern: &str) -> (Seq, Seq) {
            (prefixes(pattern), suffixes(pattern))
        }
        assert_eq!(
            (seq([I("aaa"), I("aab"), I("aba"), I("abb"), I("baa"), I("bab"), I("bba"),
            I("bbb")]), seq([I("aaa"), I("aab"), I("aba"), I("abb"), I("baa"), I("bab"),
            I("bba"), I("bbb")])), e(r"[ab]{3}{3}")
        );
        assert_eq!(infinite(), e(r"ab|cd|ef|gh|ij|kl|mn|op|qr|st|uv|wx|yz"));
    }
    #[test]
    fn empty() {
        assert_eq!(exact([""]), e(r""));
        assert_eq!(exact([""]), e(r"^"));
        assert_eq!(exact([""]), e(r"$"));
        assert_eq!(exact([""]), e(r"(?m:^)"));
        assert_eq!(exact([""]), e(r"(?m:$)"));
        assert_eq!(exact([""]), e(r"\b"));
        assert_eq!(exact([""]), e(r"\B"));
        assert_eq!(exact([""]), e(r"(?-u:\b)"));
        assert_eq!(exact([""]), e(r"(?-u:\B)"));
    }
    #[test]
    fn odds_and_ends() {
        assert_eq!((Seq::infinite(), seq([I("a")])), e(r".a"));
        assert_eq!((seq([I("a")]), Seq::infinite()), e(r"a."));
        assert_eq!(infinite(), e(r"a|."));
        assert_eq!(infinite(), e(r".|a"));
        let pat = r"M[ou]'?am+[ae]r .*([AEae]l[- ])?[GKQ]h?[aeu]+([dtz][dhz]?)+af[iy]";
        let expected = inexact(
            ["Mo'am", "Moam", "Mu'am", "Muam"].map(I),
            [
                "ddafi",
                "ddafy",
                "dhafi",
                "dhafy",
                "dzafi",
                "dzafy",
                "dafi",
                "dafy",
                "tdafi",
                "tdafy",
                "thafi",
                "thafy",
                "tzafi",
                "tzafy",
                "tafi",
                "tafy",
                "zdafi",
                "zdafy",
                "zhafi",
                "zhafy",
                "zzafi",
                "zzafy",
                "zafi",
                "zafy",
            ]
                .map(I),
        );
        assert_eq!(expected, e(pat));
        assert_eq!(
            (seq(["fn is_", "fn as_"].map(I)), Seq::infinite()),
            e(r"fn is_([A-Z]+)|fn as_([A-Z]+)"),
        );
        assert_eq!(inexact([I("foo")], [I("quux")]), e(r"foo[A-Z]+bar[A-Z]+quux"));
        assert_eq!(infinite(), e(r"[A-Z]+bar[A-Z]+"));
        assert_eq!(
            exact(["Sherlock Holmes"]), e(r"(?m)^Sherlock Holmes|Sherlock Holmes$")
        );
        assert_eq!(exact(["sa", "sb"]), e(r"\bs(?:[ab])"));
    }
    #[test]
    #[cfg(feature = "unicode-case")]
    fn holmes() {
        let expected = inexact(
            ["HOL", "HOl", "HoL", "Hol", "hOL", "hOl", "hoL", "hol"].map(I),
            ["MES", "MEs", "Eſ", "MeS", "Mes", "eſ", "mES", "mEs", "meS", "mes"].map(I),
        );
        let (mut prefixes, mut suffixes) = e(r"(?i)Holmes");
        prefixes.keep_first_bytes(3);
        suffixes.keep_last_bytes(3);
        prefixes.minimize_by_preference();
        suffixes.minimize_by_preference();
        assert_eq!(expected, (prefixes, suffixes));
    }
    #[test]
    #[cfg(feature = "unicode-case")]
    fn holmes_alt() {
        let mut pre = prefixes(r"(?i)Sherlock|Holmes|Watson|Irene|Adler|John|Baker");
        assert!(pre.len().unwrap() > 0);
        pre.optimize_for_prefix_by_preference();
        assert!(pre.len().unwrap() > 0);
    }
    #[test]
    fn crazy_repeats() {
        assert_eq!(inexact([I("")], [I("")]), e(r"(?:){4294967295}"));
        assert_eq!(inexact([I("")], [I("")]), e(r"(?:){64}{64}{64}{64}{64}{64}"));
        assert_eq!(inexact([I("")], [I("")]), e(r"x{0}{4294967295}"));
        assert_eq!(inexact([I("")], [I("")]), e(r"(?:|){4294967295}"));
        assert_eq!(
            inexact([E("")], [E("")]),
            e(r"(?:){8}{8}{8}{8}{8}{8}{8}{8}{8}{8}{8}{8}{8}{8}")
        );
        let repa = "a".repeat(100);
        assert_eq!(
            inexact([I(& repa)], [I(& repa)]),
            e(r"a{8}{8}{8}{8}{8}{8}{8}{8}{8}{8}{8}{8}{8}{8}")
        );
    }
    #[test]
    fn huge() {
        let pat = r#"(?-u)
        2(?:
          [45]\d{3}|
          7(?:
            1[0-267]|
            2[0-289]|
            3[0-29]|
            4[01]|
            5[1-3]|
            6[013]|
            7[0178]|
            91
          )|
          8(?:
            0[125]|
            [139][1-6]|
            2[0157-9]|
            41|
            6[1-35]|
            7[1-5]|
            8[1-8]|
            90
          )|
          9(?:
            0[0-2]|
            1[0-4]|
            2[568]|
            3[3-6]|
            5[5-7]|
            6[0167]|
            7[15]|
            8[0146-9]
          )
        )\d{4}|
        3(?:
          12?[5-7]\d{2}|
          0(?:
            2(?:
              [025-79]\d|
              [348]\d{1,2}
            )|
            3(?:
              [2-4]\d|
              [56]\d?
            )
          )|
          2(?:
            1\d{2}|
            2(?:
              [12]\d|
              [35]\d{1,2}|
              4\d?
            )
          )|
          3(?:
            1\d{2}|
            2(?:
              [2356]\d|
              4\d{1,2}
            )
          )|
          4(?:
            1\d{2}|
            2(?:
              2\d{1,2}|
              [47]|
              5\d{2}
            )
          )|
          5(?:
            1\d{2}|
            29
          )|
          [67]1\d{2}|
          8(?:
            1\d{2}|
            2(?:
              2\d{2}|
              3|
              4\d
            )
          )
        )\d{3}|
        4(?:
          0(?:
            2(?:
              [09]\d|
              7
            )|
            33\d{2}
          )|
          1\d{3}|
          2(?:
            1\d{2}|
            2(?:
              [25]\d?|
              [348]\d|
              [67]\d{1,2}
            )
          )|
          3(?:
            1\d{2}(?:
              \d{2}
            )?|
            2(?:
              [045]\d|
              [236-9]\d{1,2}
            )|
            32\d{2}
          )|
          4(?:
            [18]\d{2}|
            2(?:
              [2-46]\d{2}|
              3
            )|
            5[25]\d{2}
          )|
          5(?:
            1\d{2}|
            2(?:
              3\d|
              5
            )
          )|
          6(?:
            [18]\d{2}|
            2(?:
              3(?:
                \d{2}
              )?|
              [46]\d{1,2}|
              5\d{2}|
              7\d
            )|
            5(?:
              3\d?|
              4\d|
              [57]\d{1,2}|
              6\d{2}|
              8
            )
          )|
          71\d{2}|
          8(?:
            [18]\d{2}|
            23\d{2}|
            54\d{2}
          )|
          9(?:
            [18]\d{2}|
            2[2-5]\d{2}|
            53\d{1,2}
          )
        )\d{3}|
        5(?:
          02[03489]\d{2}|
          1\d{2}|
          2(?:
            1\d{2}|
            2(?:
              2(?:
                \d{2}
              )?|
              [457]\d{2}
            )
          )|
          3(?:
            1\d{2}|
            2(?:
              [37](?:
                \d{2}
              )?|
              [569]\d{2}
            )
          )|
          4(?:
            1\d{2}|
            2[46]\d{2}
          )|
          5(?:
            1\d{2}|
            26\d{1,2}
          )|
          6(?:
            [18]\d{2}|
            2|
            53\d{2}
          )|
          7(?:
            1|
            24
          )\d{2}|
          8(?:
            1|
            26
          )\d{2}|
          91\d{2}
        )\d{3}|
        6(?:
          0(?:
            1\d{2}|
            2(?:
              3\d{2}|
              4\d{1,2}
            )
          )|
          2(?:
            2[2-5]\d{2}|
            5(?:
              [3-5]\d{2}|
              7
            )|
            8\d{2}
          )|
          3(?:
            1|
            2[3478]
          )\d{2}|
          4(?:
            1|
            2[34]
          )\d{2}|
          5(?:
            1|
            2[47]
          )\d{2}|
          6(?:
            [18]\d{2}|
            6(?:
              2(?:
                2\d|
                [34]\d{2}
              )|
              5(?:
                [24]\d{2}|
                3\d|
                5\d{1,2}
              )
            )
          )|
          72[2-5]\d{2}|
          8(?:
            1\d{2}|
            2[2-5]\d{2}
          )|
          9(?:
            1\d{2}|
            2[2-6]\d{2}
          )
        )\d{3}|
        7(?:
          (?:
            02|
            [3-589]1|
            6[12]|
            72[24]
          )\d{2}|
          21\d{3}|
          32
        )\d{3}|
        8(?:
          (?:
            4[12]|
            [5-7]2|
            1\d?
          )|
          (?:
            0|
            3[12]|
            [5-7]1|
            217
          )\d
        )\d{4}|
        9(?:
          [35]1|
          (?:
            [024]2|
            81
          )\d|
          (?:
            1|
            [24]1
          )\d{2}
        )\d{3}
        "#;
        let (prefixes, suffixes) = e(pat);
        assert!(! suffixes.is_finite());
        assert_eq!(Some(243), prefixes.len());
    }
    #[test]
    fn optimize() {
        let (p, s) = opt(["foobarfoobar", "foobar", "foobarzfoobar", "foobarfoobar"]);
        assert_eq!(seq([I("foobar")]), p);
        assert_eq!(seq([I("foobar")]), s);
        let (p, s) = opt(["abba", "akka", "abccba"]);
        assert_eq!(exact(["abba", "akka", "abccba"]), (p, s));
        let (p, s) = opt(["sam", "samwise"]);
        assert_eq!((seq([E("sam")]), seq([E("sam"), E("samwise")])), (p, s));
        let (p, s) = opt(["foobarfoo", "foo", "", "foozfoo", "foofoo"]);
        assert!(! p.is_finite());
        assert!(! s.is_finite());
        let mut p = seq([E("foobarfoo"), I("foo"), E(" "), E("foofoo")]);
        p.optimize_for_prefix_by_preference();
        assert!(! p.is_finite());
    }
}
#[cfg(test)]
mod tests_rug_120 {
    use super::*;
    #[test]
    fn test_rank() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: u8 = rug_fuzz_0;
        crate::hir::literal::rank(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_121 {
    use super::*;
    use crate::hir::literal::Extractor;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_121_rrrruuuugggg_test_rug = 0;
        Extractor::new();
        let _rug_ed_tests_rug_121_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_122 {
    use super::*;
    use crate::hir::{Hir, Class, ClassBytes, ClassBytesRange};
    use crate::hir::literal::{Extractor, Literal, Seq};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u8, u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Extractor::new();
        let v41 = Hir::class(
            Class::Bytes(
                ClassBytes::new(vec![ClassBytesRange::new(rug_fuzz_0, rug_fuzz_1)]),
            ),
        );
        <hir::literal::Extractor>::extract(&p0, &v41);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_123 {
    use super::*;
    use crate::hir::literal::Extractor;
    use crate::hir::literal::ExtractKind;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_123_rrrruuuugggg_test_rug = 0;
        let mut p0 = Extractor::new();
        let mut p1: ExtractKind = Default::default();
        <hir::literal::Extractor>::kind(&mut p0, p1);
        let _rug_ed_tests_rug_123_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_124 {
    use super::*;
    use crate::{
        hir::literal::{Extractor, Seq},
        parse,
    };
    #[test]
    fn test_limit_class() -> Result<(), Box<dyn std::error::Error>> {
        let mut p0 = Extractor::new();
        let p1 = 4;
        p0.limit_class(p1);
        Ok(())
    }
}
#[cfg(test)]
mod tests_rug_125 {
    use super::*;
    use crate::{hir::literal::Extractor, parse};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Extractor::new();
        let p1 = rug_fuzz_0;
        p0.limit_repeat(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_126 {
    use super::*;
    use crate::parse;
    use crate::hir::literal::{Extractor, Literal, Seq};
    use std::iter::FromIterator;
    #[test]
    fn test_limit_literal_len() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(usize, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Extractor::new();
        let p1: usize = rug_fuzz_0;
        let hir = parse(rug_fuzz_1).unwrap();
        let got = p0.limit_literal_len(p1).extract(&hir);
        let expected = Seq::from_iter([Literal::inexact(rug_fuzz_2)]);
        debug_assert_eq!(expected, got);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_127 {
    use super::*;
    use crate::parse;
    use crate::hir::literal::{Extractor, Literal, Seq};
    #[test]
    fn test_limit_total() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Extractor::new();
        let p1 = rug_fuzz_0;
        p0.limit_total(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_131 {
    use super::*;
    use crate::hir::literal::{Extractor, Seq};
    use crate::hir::{ClassUnicode, ClassUnicodeRange};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(char, char) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Extractor::new();
        let mut p1 = ClassUnicode::new(
            vec![
                ClassUnicodeRange { start : rug_fuzz_0, end : rug_fuzz_1, },
                ClassUnicodeRange { start : 'A', end : 'Z', }
            ],
        );
        <hir::literal::Extractor>::extract_class_unicode(&mut p0, &mut p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_132 {
    use super::*;
    use crate::hir::literal::{Extractor, Seq, Literal};
    use crate::hir::ClassBytes;
    #[test]
    fn test_extract_class_bytes() {
        let _rug_st_tests_rug_132_rrrruuuugggg_test_extract_class_bytes = 0;
        let mut p0 = Extractor::new();
        let mut p1 = ClassBytes::empty();
        p0.extract_class_bytes(&p1);
        let _rug_ed_tests_rug_132_rrrruuuugggg_test_extract_class_bytes = 0;
    }
}
#[cfg(test)]
mod tests_rug_133 {
    use super::*;
    use crate::hir::literal::Extractor;
    use crate::hir::{ClassUnicode, ClassUnicodeRange};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(char, char) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Extractor::new();
        let mut p1 = ClassUnicode::new(
            vec![
                ClassUnicodeRange { start : rug_fuzz_0, end : rug_fuzz_1, },
                ClassUnicodeRange { start : 'A', end : 'Z', }
            ],
        );
        debug_assert_eq!(Extractor::class_over_limit_unicode(& p0, & p1), false);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_134 {
    use super::*;
    use crate::hir::literal::Extractor;
    use crate::hir::ClassBytes;
    #[test]
    fn test_class_over_limit_bytes() {
        let _rug_st_tests_rug_134_rrrruuuugggg_test_class_over_limit_bytes = 0;
        let mut p0 = Extractor::new();
        let mut p1 = ClassBytes::empty();
        p0.class_over_limit_bytes(&p1);
        let _rug_ed_tests_rug_134_rrrruuuugggg_test_class_over_limit_bytes = 0;
    }
}
#[cfg(test)]
mod tests_rug_135 {
    use super::*;
    use crate::hir::literal::{Extractor, Seq};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5)) = <(&str, &str, &str, &str, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Extractor::new();
        let mut p1 = Seq::new([rug_fuzz_0, rug_fuzz_1, rug_fuzz_2]);
        let mut p2 = Seq::new([rug_fuzz_3, rug_fuzz_4, rug_fuzz_5]);
        p0.cross(p1, &mut p2);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_136 {
    use super::*;
    use crate::hir::literal::{Extractor, Seq};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5)) = <(&str, &str, &str, &str, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Extractor::new();
        let mut p1 = Seq::new([rug_fuzz_0, rug_fuzz_1, rug_fuzz_2]);
        let mut p2 = Seq::new([rug_fuzz_3, rug_fuzz_4, rug_fuzz_5]);
        p0.union(p1, &mut p2);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_137 {
    use super::*;
    use crate::hir::literal::{Extractor, Seq};
    #[test]
    fn test_enforce_literal_len() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(&str, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Extractor::new();
        let mut p1 = Seq::new([rug_fuzz_0, rug_fuzz_1, rug_fuzz_2]);
        Extractor::enforce_literal_len(&mut p0, &mut p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_138 {
    use super::*;
    use crate::hir::literal::Extractor;
    use std::default::Default;
    #[test]
    fn test_default() {
        let _rug_st_tests_rug_138_rrrruuuugggg_test_default = 0;
        let extractor: Extractor = Extractor::default();
        let _rug_ed_tests_rug_138_rrrruuuugggg_test_default = 0;
    }
}
#[cfg(test)]
mod tests_rug_139 {
    use super::*;
    use crate::hir::literal::ExtractKind;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_139_rrrruuuugggg_test_rug = 0;
        let mut p0: ExtractKind = Default::default();
        <ExtractKind>::is_prefix(&mut p0);
        let _rug_ed_tests_rug_139_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_140 {
    use super::*;
    use crate::hir::literal::ExtractKind;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_140_rrrruuuugggg_test_rug = 0;
        let mut p0: ExtractKind = Default::default();
        ExtractKind::is_suffix(&p0);
        let _rug_ed_tests_rug_140_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_141 {
    use super::*;
    use crate::hir::literal::ExtractKind;
    use std::default::Default;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_141_rrrruuuugggg_test_rug = 0;
        <ExtractKind as std::default::Default>::default();
        let _rug_ed_tests_rug_141_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_142 {
    use super::*;
    use crate::hir::literal::Seq;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_142_rrrruuuugggg_test_rug = 0;
        Seq::empty();
        let _rug_ed_tests_rug_142_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_143 {
    use super::*;
    use crate::hir::literal::Seq;
    #[test]
    fn test_infinite() {
        let _rug_st_tests_rug_143_rrrruuuugggg_test_infinite = 0;
        Seq::infinite();
        let _rug_ed_tests_rug_143_rrrruuuugggg_test_infinite = 0;
    }
}
#[cfg(test)]
mod tests_rug_144 {
    use super::*;
    use crate::hir::literal::{Literal, Seq};
    #[test]
    fn test_singleton() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Literal::exact(rug_fuzz_0.as_bytes().to_vec());
        Seq::singleton(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_145 {
    use super::*;
    use std::iter::once;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_145_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"hello";
        let rug_fuzz_1 = b"world";
        let mut p0: Vec<&[u8]> = Vec::new();
        p0.push(rug_fuzz_0);
        p0.push(rug_fuzz_1);
        <hir::literal::Seq>::new::<_, &[u8]>(p0);
        let _rug_ed_tests_rug_145_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_146 {
    use super::*;
    use crate::hir::literal::{Literal, Seq};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(&str, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut v88 = Seq::new([rug_fuzz_0, rug_fuzz_1, rug_fuzz_2]);
        let p0 = &v88;
        <hir::literal::Seq>::literals(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_147 {
    use super::*;
    use crate::hir::literal::{Literal, Seq};
    #[test]
    fn test_push() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(&str, &str, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Seq::new([rug_fuzz_0, rug_fuzz_1, rug_fuzz_2]);
        let mut p1 = Literal::exact(rug_fuzz_3.as_bytes().to_vec());
        p0.push(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_148_prepare {
    use crate::hir::literal::{Literal, Seq};
    #[test]
    fn sample() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(&str, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut v88 = Seq::new([rug_fuzz_0, rug_fuzz_1, rug_fuzz_2]);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_148 {
    use super::*;
    use crate::hir::literal::Seq;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(&str, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Seq::new([rug_fuzz_0, rug_fuzz_1, rug_fuzz_2]);
        <Seq>::make_inexact(&mut p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_149 {
    use super::*;
    use crate::hir::literal::{Literal, Seq};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(&str, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Seq::new([rug_fuzz_0, rug_fuzz_1, rug_fuzz_2]);
        Seq::make_infinite(&mut p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_150 {
    use super::*;
    use crate::hir::literal::{Literal, Seq};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(&str, &str, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Seq::from_iter([
            Literal::exact(rug_fuzz_0),
            Literal::inexact(rug_fuzz_1),
        ]);
        let mut p1 = Seq::from_iter([
            Literal::inexact(rug_fuzz_2),
            Literal::exact(rug_fuzz_3),
        ]);
        <hir::literal::Seq>::cross_forward(&mut p0, &mut p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_151 {
    use super::*;
    use crate::hir::literal::{Literal, Seq};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5, mut rug_fuzz_6)) = <(&str, &str, &str, &str, &str, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Seq::from_iter([
            Literal::exact(rug_fuzz_0),
            Literal::inexact(rug_fuzz_1),
        ]);
        let mut p1 = Seq::from_iter([
            Literal::inexact(rug_fuzz_2),
            Literal::exact(rug_fuzz_3),
        ]);
        <hir::literal::Seq>::cross_reverse(&mut p0, &mut p1);
        let expected = Seq::from_iter([
            Literal::inexact(rug_fuzz_4),
            Literal::inexact(rug_fuzz_5),
            Literal::exact(rug_fuzz_6),
        ]);
        debug_assert_eq!(expected, p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_152 {
    use super::*;
    use crate::hir::literal::{Literal, Seq};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5)) = <(&str, &str, &str, &str, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Seq::new([rug_fuzz_0, rug_fuzz_1, rug_fuzz_2]);
        let mut p1 = Seq::new([rug_fuzz_3, rug_fuzz_4, rug_fuzz_5]);
        <hir::literal::Seq>::cross_preamble(&mut p0, &mut p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_153 {
    use super::*;
    use crate::hir::literal::Seq;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5)) = <(&str, &str, &str, &str, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Seq::new([rug_fuzz_0, rug_fuzz_1, rug_fuzz_2]);
        let mut p1 = Seq::new([rug_fuzz_3, rug_fuzz_4, rug_fuzz_5]);
        Seq::union(&mut p0, &mut p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_154 {
    use super::*;
    use crate::hir::literal::Seq;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4)) = <(&str, &str, &str, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Seq::new([rug_fuzz_0, rug_fuzz_1, rug_fuzz_2]);
        let mut p1 = Seq::new([rug_fuzz_3, rug_fuzz_4]);
        Seq::union_into_empty(&mut p0, &mut p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_155 {
    use super::*;
    use crate::hir::literal::{Literal, Seq};
    #[test]
    fn test_dedup() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(&str, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Seq::from_iter([
            Literal::exact(rug_fuzz_0),
            Literal::inexact(rug_fuzz_1),
        ]);
        <hir::literal::Seq>::dedup(&mut p0);
        debug_assert_eq!(Seq::from_iter([Literal::inexact(rug_fuzz_2)]), p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_156 {
    use super::*;
    use crate::hir::literal::Seq;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(&str, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Seq::new(&[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2]);
        Seq::sort(&mut p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_157 {
    use super::*;
    use crate::hir::literal::{Literal, Seq};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(&str, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = {
            let mut v88 = Seq::new([rug_fuzz_0, rug_fuzz_1, rug_fuzz_2]);
            v88
        };
        <hir::literal::Seq>::reverse_literals(&mut p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_158 {
    use super::*;
    use crate::hir::literal::{Literal, Seq};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(&str, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Seq::new([rug_fuzz_0, rug_fuzz_1, rug_fuzz_2]);
        Seq::minimize_by_preference(&mut p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_159 {
    use super::*;
    use crate::hir::literal::{Literal, Seq};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(&str, &str, &str, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Seq::new([rug_fuzz_0, rug_fuzz_1, rug_fuzz_2]);
        let p1: usize = rug_fuzz_3;
        p0.keep_first_bytes(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_160 {
    use super::*;
    use crate::hir::literal::{Literal, Seq};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5, mut rug_fuzz_6)) = <(&str, &str, &str, usize, &str, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Seq::new([rug_fuzz_0, rug_fuzz_1, rug_fuzz_2]);
        let p1: usize = rug_fuzz_3;
        <hir::literal::Seq>::keep_last_bytes(&mut p0, p1);
        let expected = Seq::from_iter([
            Literal::exact(rug_fuzz_4),
            Literal::inexact(rug_fuzz_5),
            Literal::inexact(rug_fuzz_6),
        ]);
        debug_assert_eq!(expected, p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_161 {
    use super::*;
    use crate::hir::literal::Seq;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(&str, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Seq::new([rug_fuzz_0, rug_fuzz_1, rug_fuzz_2]);
        <hir::literal::Seq>::is_finite(&p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_162 {
    use super::*;
    use crate::hir::literal::{Literal, Seq};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(&str, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Seq::new([rug_fuzz_0, rug_fuzz_1, rug_fuzz_2]);
        <hir::literal::Seq>::is_empty(&p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_163 {
    use super::*;
    use crate::hir::literal::{Literal, Seq};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(&str, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Seq::new([rug_fuzz_0, rug_fuzz_1, rug_fuzz_2]);
        <hir::literal::Seq>::len(&p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_164 {
    use super::*;
    use crate::hir::literal::{Literal, Seq};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(&str, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut v88 = Seq::new([rug_fuzz_0, rug_fuzz_1, rug_fuzz_2]);
        crate::hir::literal::Seq::is_exact(&v88);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_165 {
    use super::*;
    use crate::hir::literal::Seq;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(&str, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Seq::new([rug_fuzz_0, rug_fuzz_1, rug_fuzz_2]);
        crate::hir::literal::Seq::is_inexact(&p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_166 {
    use super::*;
    use crate::hir::literal::Seq;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5)) = <(&str, &str, &str, &str, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0 = Seq::new([rug_fuzz_0, rug_fuzz_1, rug_fuzz_2]);
        let p1 = Seq::new([rug_fuzz_3, rug_fuzz_4, rug_fuzz_5]);
        p0.max_union_len(&p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_167 {
    use super::*;
    use crate::hir::literal::Seq;
    #[test]
    fn test_max_cross_len() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5)) = <(&str, &str, &str, &str, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Seq::new([rug_fuzz_0, rug_fuzz_1, rug_fuzz_2]);
        let mut p1 = Seq::new([rug_fuzz_3, rug_fuzz_4, rug_fuzz_5]);
        p0.max_cross_len(&p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_168 {
    use super::*;
    use crate::hir::literal::{Literal, Seq};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(&str, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut v88 = Seq::new([rug_fuzz_0, rug_fuzz_1, rug_fuzz_2]);
        <hir::literal::Seq>::min_literal_len(&v88);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_169 {
    use super::*;
    use crate::hir::literal::{Literal, Seq};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(&str, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut v88 = Seq::new([rug_fuzz_0, rug_fuzz_1, rug_fuzz_2]);
        <hir::literal::Seq>::max_literal_len(&v88);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_170 {
    use super::*;
    use crate::hir::literal::{Literal, Seq};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(&str, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Seq::new(&[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2]);
        <hir::literal::Seq>::longest_common_prefix(&p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_171 {
    use super::*;
    use crate::hir::literal::Seq;
    #[test]
    fn test_longest_common_suffix() {
        let _rug_st_tests_rug_171_rrrruuuugggg_test_longest_common_suffix = 0;
        let rug_fuzz_0 = "oof";
        let rug_fuzz_1 = "raboof";
        let rug_fuzz_2 = "of";
        let rug_fuzz_3 = b"of";
        let rug_fuzz_4 = "foo";
        let rug_fuzz_5 = "foo";
        let rug_fuzz_6 = b"foo";
        let rug_fuzz_7 = "foo";
        let rug_fuzz_8 = "bar";
        let rug_fuzz_9 = b"";
        let rug_fuzz_10 = "";
        let rug_fuzz_11 = b"";
        let mut p0 = Seq::new(&[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2]);
        debug_assert_eq!(Some(& rug_fuzz_3[..]), p0.longest_common_suffix());
        let mut p1 = Seq::new(&[rug_fuzz_4, rug_fuzz_5]);
        debug_assert_eq!(Some(& rug_fuzz_6[..]), p1.longest_common_suffix());
        let mut p2 = Seq::new(&[rug_fuzz_7, rug_fuzz_8]);
        debug_assert_eq!(Some(& rug_fuzz_9[..]), p2.longest_common_suffix());
        let mut p3 = Seq::new(&[rug_fuzz_10]);
        debug_assert_eq!(Some(& rug_fuzz_11[..]), p3.longest_common_suffix());
        let mut p4 = Seq::infinite();
        debug_assert_eq!(None, p4.longest_common_suffix());
        let mut p5 = Seq::empty();
        debug_assert_eq!(None, p5.longest_common_suffix());
        let _rug_ed_tests_rug_171_rrrruuuugggg_test_longest_common_suffix = 0;
    }
}
#[cfg(test)]
mod tests_rug_172 {
    use super::*;
    use crate::hir::literal::{Seq, Literal};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(&str, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Seq::new([rug_fuzz_0, rug_fuzz_1, rug_fuzz_2]);
        Seq::optimize_for_prefix_by_preference(&mut p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_173 {
    use super::*;
    use crate::hir::literal::{Literal, Seq};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(&str, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Seq::new([rug_fuzz_0, rug_fuzz_1, rug_fuzz_2]);
        <hir::literal::Seq>::optimize_for_suffix_by_preference(&mut p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_174 {
    use super::*;
    use crate::hir::literal::{Literal, Seq};
    #[test]
    fn test_optimize_by_preference() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(&str, &str, &str, bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut seq = Seq::new([rug_fuzz_0, rug_fuzz_1, rug_fuzz_2]);
        let prefix = rug_fuzz_3;
        seq.optimize_by_preference(prefix);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_176 {
    use super::*;
    use crate::hir::literal::Literal;
    #[test]
    fn test_exact() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: Vec<u8> = vec![rug_fuzz_0, b'b', b'c'];
        Literal::exact(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_177 {
    use super::*;
    use crate::hir::literal::Literal;
    #[test]
    fn test_inexact() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: Vec<u8> = vec![rug_fuzz_0, 98, 99];
        Literal::inexact(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_178 {
    use super::*;
    use crate::hir::literal::Literal;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut v90 = Literal::exact(rug_fuzz_0.as_bytes().to_vec());
        <hir::literal::Literal>::as_bytes(&v90);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_179 {
    use super::*;
    use crate::hir::literal::Literal;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Literal::exact(rug_fuzz_0.as_bytes().to_vec());
        <hir::literal::Literal>::into_bytes(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_180 {
    use super::*;
    use crate::hir::literal::Literal;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Literal::exact(rug_fuzz_0.as_bytes().to_vec());
        <hir::literal::Literal>::len(&p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_181 {
    use super::*;
    use crate::hir::literal::Literal;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Literal::exact(rug_fuzz_0.as_bytes().to_vec());
        <hir::literal::Literal>::is_empty(&p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_182 {
    use super::*;
    use crate::hir::literal::Literal;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Literal::exact(rug_fuzz_0.as_bytes().to_vec());
        <hir::literal::Literal>::is_exact(&p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_183 {
    use super::*;
    use crate::hir::literal::Literal;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Literal::exact(rug_fuzz_0.as_bytes().to_vec());
        Literal::make_inexact(&mut p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_184 {
    use super::*;
    use crate::hir::literal::Literal;
    #[test]
    fn test_reverse() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Literal::exact(rug_fuzz_0.as_bytes().to_vec());
        <Literal>::reverse(&mut p0);
        debug_assert_eq!(
            p0.bytes, vec![101, 116, 97, 100, 32, 101, 108, 112, 109, 97, 115]
        );
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_185 {
    use super::*;
    use crate::hir::literal::Literal;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(&str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Literal::exact(rug_fuzz_0.as_bytes().to_vec());
        let mut p1 = Literal::exact(rug_fuzz_1.as_bytes().to_vec());
        <hir::literal::Literal>::extend(&mut p0, &p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_186 {
    use super::*;
    use crate::hir::literal::Literal;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(&str, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Literal::exact(rug_fuzz_0.as_bytes().to_vec());
        let p1: usize = rug_fuzz_1;
        Literal::keep_first_bytes(&mut p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_187 {
    use super::*;
    use crate::hir::literal::Literal;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(&str, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Literal::exact(rug_fuzz_0.as_bytes().to_vec());
        let p1: usize = rug_fuzz_1;
        p0.keep_last_bytes(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_188 {
    use super::*;
    use crate::hir::literal::Literal;
    #[test]
    fn test_is_poisonous() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Literal::exact(rug_fuzz_0.as_bytes().to_vec());
        Literal::is_poisonous(&mut p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_189 {
    use super::*;
    use crate::hir::literal::Literal;
    use std::convert::From;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: u8 = rug_fuzz_0;
        <Literal as std::convert::From<u8>>::from(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_190 {
    use super::*;
    use alloc::string::ToString;
    use crate::hir;
    #[test]
    fn test_from() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(char) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: char = rug_fuzz_0;
        <hir::literal::Literal as std::convert::From<char>>::from(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_191 {
    use super::*;
    use crate::hir::literal::Literal;
    use crate::std::convert::AsRef;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Literal::exact(rug_fuzz_0.as_bytes().to_vec());
        <Literal as AsRef<[u8]>>::as_ref(&p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_192 {
    use super::*;
    use crate::hir::literal::Literal;
    use crate::hir::literal::PreferenceTrie;
    #[test]
    fn test_minimize() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Vec<Literal> = vec![];
        let mut p1: bool = rug_fuzz_0;
        PreferenceTrie::minimize(&mut p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_193 {
    use super::*;
    use crate::hir::literal::PreferenceTrie;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_193_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"test";
        let mut v94 = PreferenceTrie::default();
        let mut p0 = &mut v94;
        let p1 = rug_fuzz_0;
        <hir::literal::PreferenceTrie>::insert(p0, p1);
        let _rug_ed_tests_rug_193_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_194 {
    use super::*;
    use crate::hir::literal::PreferenceTrie;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_194_rrrruuuugggg_test_rug = 0;
        let mut p0 = PreferenceTrie::default();
        <hir::literal::PreferenceTrie>::root(&mut p0);
        let _rug_ed_tests_rug_194_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_195 {
    use super::*;
    use crate::hir::literal::PreferenceTrie;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_195_rrrruuuugggg_test_rug = 0;
        let mut p0 = PreferenceTrie::default();
        <hir::literal::PreferenceTrie>::create_state(&mut p0);
        let _rug_ed_tests_rug_195_rrrruuuugggg_test_rug = 0;
    }
}
