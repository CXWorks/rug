use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;
#[cfg(feature = "perf-literal")]
use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use syntax::hir::literal::Literals;
use syntax::hir::Hir;
use syntax::ParserBuilder;
use backtrack;
use cache::{Cached, CachedGuard};
use compile::Compiler;
#[cfg(feature = "perf-dfa")]
use dfa;
use error::Error;
use input::{ByteInput, CharInput};
use literal::LiteralSearcher;
use pikevm;
use prog::Program;
use re_builder::RegexOptions;
use re_bytes;
use re_set;
use re_trait::{Locations, RegularExpression, Slot};
use re_unicode;
use utf8::next_utf8;
/// `Exec` manages the execution of a regular expression.
///
/// In particular, this manages the various compiled forms of a single regular
/// expression and the choice of which matching engine to use to execute a
/// regular expression.
#[derive(Debug)]
pub struct Exec {
    /// All read only state.
    ro: Arc<ExecReadOnly>,
    /// Caches for the various matching engines.
    cache: Cached<ProgramCache>,
}
/// `ExecNoSync` is like `Exec`, except it embeds a reference to a cache. This
/// means it is no longer Sync, but we can now avoid the overhead of
/// synchronization to fetch the cache.
#[derive(Debug)]
pub struct ExecNoSync<'c> {
    /// All read only state.
    ro: &'c Arc<ExecReadOnly>,
    /// Caches for the various matching engines.
    cache: CachedGuard<'c, ProgramCache>,
}
/// `ExecNoSyncStr` is like `ExecNoSync`, but matches on &str instead of &[u8].
#[derive(Debug)]
pub struct ExecNoSyncStr<'c>(ExecNoSync<'c>);
/// `ExecReadOnly` comprises all read only state for a regex. Namely, all such
/// state is determined at compile time and never changes during search.
#[derive(Debug)]
struct ExecReadOnly {
    /// The original regular expressions given by the caller to compile.
    res: Vec<String>,
    /// A compiled program that is used in the NFA simulation and backtracking.
    /// It can be byte-based or Unicode codepoint based.
    ///
    /// N.B. It is not possibly to make this byte-based from the public API.
    /// It is only used for testing byte based programs in the NFA simulations.
    nfa: Program,
    /// A compiled byte based program for DFA execution. This is only used
    /// if a DFA can be executed. (Currently, only word boundary assertions are
    /// not supported.) Note that this program contains an embedded `.*?`
    /// preceding the first capture group, unless the regex is anchored at the
    /// beginning.
    dfa: Program,
    /// The same as above, except the program is reversed (and there is no
    /// preceding `.*?`). This is used by the DFA to find the starting location
    /// of matches.
    dfa_reverse: Program,
    /// A set of suffix literals extracted from the regex.
    ///
    /// Prefix literals are stored on the `Program`, since they are used inside
    /// the matching engines.
    suffixes: LiteralSearcher,
    /// An Aho-Corasick automaton with leftmost-first match semantics.
    ///
    /// This is only set when the entire regex is a simple unanchored
    /// alternation of literals. We could probably use it more circumstances,
    /// but this is already hacky enough in this architecture.
    ///
    /// N.B. We use u32 as a state ID representation under the assumption that
    /// if we were to exhaust the ID space, we probably would have long
    /// surpassed the compilation size limit.
    #[cfg(feature = "perf-literal")]
    ac: Option<AhoCorasick<u32>>,
    /// match_type encodes as much upfront knowledge about how we're going to
    /// execute a search as possible.
    match_type: MatchType,
}
/// Facilitates the construction of an executor by exposing various knobs
/// to control how a regex is executed and what kinds of resources it's
/// permitted to use.
#[allow(missing_debug_implementations)]
pub struct ExecBuilder {
    options: RegexOptions,
    match_type: Option<MatchType>,
    bytes: bool,
    only_utf8: bool,
}
/// Parsed represents a set of parsed regular expressions and their detected
/// literals.
struct Parsed {
    exprs: Vec<Hir>,
    prefixes: Literals,
    suffixes: Literals,
    bytes: bool,
}
impl ExecBuilder {
    /// Create a regex execution builder.
    ///
    /// This uses default settings for everything except the regex itself,
    /// which must be provided. Further knobs can be set by calling methods,
    /// and then finally, `build` to actually create the executor.
    pub fn new(re: &str) -> Self {
        Self::new_many(&[re])
    }
    /// Like new, but compiles the union of the given regular expressions.
    ///
    /// Note that when compiling 2 or more regular expressions, capture groups
    /// are completely unsupported. (This means both `find` and `captures`
    /// wont work.)
    pub fn new_many<I, S>(res: I) -> Self
    where
        S: AsRef<str>,
        I: IntoIterator<Item = S>,
    {
        let mut opts = RegexOptions::default();
        opts.pats = res.into_iter().map(|s| s.as_ref().to_owned()).collect();
        Self::new_options(opts)
    }
    /// Create a regex execution builder.
    pub fn new_options(opts: RegexOptions) -> Self {
        ExecBuilder {
            options: opts,
            match_type: None,
            bytes: false,
            only_utf8: true,
        }
    }
    /// Set the matching engine to be automatically determined.
    ///
    /// This is the default state and will apply whatever optimizations are
    /// possible, such as running a DFA.
    ///
    /// This overrides whatever was previously set via the `nfa` or
    /// `bounded_backtracking` methods.
    pub fn automatic(mut self) -> Self {
        self.match_type = None;
        self
    }
    /// Sets the matching engine to use the NFA algorithm no matter what
    /// optimizations are possible.
    ///
    /// This overrides whatever was previously set via the `automatic` or
    /// `bounded_backtracking` methods.
    pub fn nfa(mut self) -> Self {
        self.match_type = Some(MatchType::Nfa(MatchNfaType::PikeVM));
        self
    }
    /// Sets the matching engine to use a bounded backtracking engine no
    /// matter what optimizations are possible.
    ///
    /// One must use this with care, since the bounded backtracking engine
    /// uses memory proportion to `len(regex) * len(text)`.
    ///
    /// This overrides whatever was previously set via the `automatic` or
    /// `nfa` methods.
    pub fn bounded_backtracking(mut self) -> Self {
        self.match_type = Some(MatchType::Nfa(MatchNfaType::Backtrack));
        self
    }
    /// Compiles byte based programs for use with the NFA matching engines.
    ///
    /// By default, the NFA engines match on Unicode scalar values. They can
    /// be made to use byte based programs instead. In general, the byte based
    /// programs are slower because of a less efficient encoding of character
    /// classes.
    ///
    /// Note that this does not impact DFA matching engines, which always
    /// execute on bytes.
    pub fn bytes(mut self, yes: bool) -> Self {
        self.bytes = yes;
        self
    }
    /// When disabled, the program compiled may match arbitrary bytes.
    ///
    /// When enabled (the default), all compiled programs exclusively match
    /// valid UTF-8 bytes.
    pub fn only_utf8(mut self, yes: bool) -> Self {
        self.only_utf8 = yes;
        self
    }
    /// Set the Unicode flag.
    pub fn unicode(mut self, yes: bool) -> Self {
        self.options.unicode = yes;
        self
    }
    /// Parse the current set of patterns into their AST and extract literals.
    fn parse(&self) -> Result<Parsed, Error> {
        let mut exprs = Vec::with_capacity(self.options.pats.len());
        let mut prefixes = Some(Literals::empty());
        let mut suffixes = Some(Literals::empty());
        let mut bytes = false;
        let is_set = self.options.pats.len() > 1;
        for pat in &self.options.pats {
            let mut parser = ParserBuilder::new()
                .octal(self.options.octal)
                .case_insensitive(self.options.case_insensitive)
                .multi_line(self.options.multi_line)
                .dot_matches_new_line(self.options.dot_matches_new_line)
                .swap_greed(self.options.swap_greed)
                .ignore_whitespace(self.options.ignore_whitespace)
                .unicode(self.options.unicode)
                .allow_invalid_utf8(!self.only_utf8)
                .nest_limit(self.options.nest_limit)
                .build();
            let expr = parser.parse(pat).map_err(|e| Error::Syntax(e.to_string()))?;
            bytes = bytes || !expr.is_always_utf8();
            if cfg!(feature = "perf-literal") {
                if !expr.is_anchored_start() && expr.is_any_anchored_start() {
                    prefixes = None;
                } else if is_set && expr.is_anchored_start() {
                    prefixes = None;
                }
                prefixes = prefixes
                    .and_then(|mut prefixes| {
                        if !prefixes.union_prefixes(&expr) {
                            None
                        } else {
                            Some(prefixes)
                        }
                    });
                if !expr.is_anchored_end() && expr.is_any_anchored_end() {
                    suffixes = None;
                } else if is_set && expr.is_anchored_end() {
                    suffixes = None;
                }
                suffixes = suffixes
                    .and_then(|mut suffixes| {
                        if !suffixes.union_suffixes(&expr) {
                            None
                        } else {
                            Some(suffixes)
                        }
                    });
            }
            exprs.push(expr);
        }
        Ok(Parsed {
            exprs: exprs,
            prefixes: prefixes.unwrap_or_else(Literals::empty),
            suffixes: suffixes.unwrap_or_else(Literals::empty),
            bytes: bytes,
        })
    }
    /// Build an executor that can run a regular expression.
    pub fn build(self) -> Result<Exec, Error> {
        if self.options.pats.is_empty() {
            let ro = Arc::new(ExecReadOnly {
                res: vec![],
                nfa: Program::new(),
                dfa: Program::new(),
                dfa_reverse: Program::new(),
                suffixes: LiteralSearcher::empty(),
                #[cfg(feature = "perf-literal")]
                ac: None,
                match_type: MatchType::Nothing,
            });
            return Ok(Exec {
                ro: ro,
                cache: Cached::new(),
            });
        }
        let parsed = self.parse()?;
        let mut nfa = Compiler::new()
            .size_limit(self.options.size_limit)
            .bytes(self.bytes || parsed.bytes)
            .only_utf8(self.only_utf8)
            .compile(&parsed.exprs)?;
        let mut dfa = Compiler::new()
            .size_limit(self.options.size_limit)
            .dfa(true)
            .only_utf8(self.only_utf8)
            .compile(&parsed.exprs)?;
        let mut dfa_reverse = Compiler::new()
            .size_limit(self.options.size_limit)
            .dfa(true)
            .only_utf8(self.only_utf8)
            .reverse(true)
            .compile(&parsed.exprs)?;
        #[cfg(feature = "perf-literal")]
        let ac = self.build_aho_corasick(&parsed);
        nfa.prefixes = LiteralSearcher::prefixes(parsed.prefixes);
        dfa.prefixes = nfa.prefixes.clone();
        dfa.dfa_size_limit = self.options.dfa_size_limit;
        dfa_reverse.dfa_size_limit = self.options.dfa_size_limit;
        let mut ro = ExecReadOnly {
            res: self.options.pats,
            nfa: nfa,
            dfa: dfa,
            dfa_reverse: dfa_reverse,
            suffixes: LiteralSearcher::suffixes(parsed.suffixes),
            #[cfg(feature = "perf-literal")]
            ac: ac,
            match_type: MatchType::Nothing,
        };
        ro.match_type = ro.choose_match_type(self.match_type);
        let ro = Arc::new(ro);
        Ok(Exec {
            ro: ro,
            cache: Cached::new(),
        })
    }
    #[cfg(feature = "perf-literal")]
    fn build_aho_corasick(&self, parsed: &Parsed) -> Option<AhoCorasick<u32>> {
        if parsed.exprs.len() != 1 {
            return None;
        }
        let lits = match alternation_literals(&parsed.exprs[0]) {
            None => return None,
            Some(lits) => lits,
        };
        if lits.len() <= 32 {
            return None;
        }
        Some(
            AhoCorasickBuilder::new()
                .match_kind(MatchKind::LeftmostFirst)
                .auto_configure(&lits)
                .byte_classes(true)
                .build_with_size::<u32, _, _>(&lits)
                .expect("AC automaton too big"),
        )
    }
}
impl<'c> RegularExpression for ExecNoSyncStr<'c> {
    type Text = str;
    fn slots_len(&self) -> usize {
        self.0.slots_len()
    }
    fn next_after_empty(&self, text: &str, i: usize) -> usize {
        next_utf8(text.as_bytes(), i)
    }
    #[cfg_attr(feature = "perf-inline", inline(always))]
    fn shortest_match_at(&self, text: &str, start: usize) -> Option<usize> {
        self.0.shortest_match_at(text.as_bytes(), start)
    }
    #[cfg_attr(feature = "perf-inline", inline(always))]
    fn is_match_at(&self, text: &str, start: usize) -> bool {
        self.0.is_match_at(text.as_bytes(), start)
    }
    #[cfg_attr(feature = "perf-inline", inline(always))]
    fn find_at(&self, text: &str, start: usize) -> Option<(usize, usize)> {
        self.0.find_at(text.as_bytes(), start)
    }
    #[cfg_attr(feature = "perf-inline", inline(always))]
    fn captures_read_at(
        &self,
        locs: &mut Locations,
        text: &str,
        start: usize,
    ) -> Option<(usize, usize)> {
        self.0.captures_read_at(locs, text.as_bytes(), start)
    }
}
impl<'c> RegularExpression for ExecNoSync<'c> {
    type Text = [u8];
    /// Returns the number of capture slots in the regular expression. (There
    /// are two slots for every capture group, corresponding to possibly empty
    /// start and end locations of the capture.)
    fn slots_len(&self) -> usize {
        self.ro.nfa.captures.len() * 2
    }
    fn next_after_empty(&self, _text: &[u8], i: usize) -> usize {
        i + 1
    }
    /// Returns the end of a match location, possibly occurring before the
    /// end location of the correct leftmost-first match.
    #[cfg_attr(feature = "perf-inline", inline(always))]
    fn shortest_match_at(&self, text: &[u8], start: usize) -> Option<usize> {
        if !self.is_anchor_end_match(text) {
            return None;
        }
        match self.ro.match_type {
            #[cfg(feature = "perf-literal")]
            MatchType::Literal(ty) => self.find_literals(ty, text, start).map(|(_, e)| e),
            #[cfg(feature = "perf-dfa")]
            MatchType::Dfa | MatchType::DfaMany => {
                match self.shortest_dfa(text, start) {
                    dfa::Result::Match(end) => Some(end),
                    dfa::Result::NoMatch(_) => None,
                    dfa::Result::Quit => self.shortest_nfa(text, start),
                }
            }
            #[cfg(feature = "perf-dfa")]
            MatchType::DfaAnchoredReverse => {
                match dfa::Fsm::reverse(
                    &self.ro.dfa_reverse,
                    self.cache.value(),
                    true,
                    &text[start..],
                    text.len(),
                ) {
                    dfa::Result::Match(_) => Some(text.len()),
                    dfa::Result::NoMatch(_) => None,
                    dfa::Result::Quit => self.shortest_nfa(text, start),
                }
            }
            #[cfg(all(feature = "perf-dfa", feature = "perf-literal"))]
            MatchType::DfaSuffix => {
                match self.shortest_dfa_reverse_suffix(text, start) {
                    dfa::Result::Match(e) => Some(e),
                    dfa::Result::NoMatch(_) => None,
                    dfa::Result::Quit => self.shortest_nfa(text, start),
                }
            }
            MatchType::Nfa(ty) => self.shortest_nfa_type(ty, text, start),
            MatchType::Nothing => None,
        }
    }
    /// Returns true if and only if the regex matches text.
    ///
    /// For single regular expressions, this is equivalent to calling
    /// shortest_match(...).is_some().
    #[cfg_attr(feature = "perf-inline", inline(always))]
    fn is_match_at(&self, text: &[u8], start: usize) -> bool {
        if !self.is_anchor_end_match(text) {
            return false;
        }
        match self.ro.match_type {
            #[cfg(feature = "perf-literal")]
            MatchType::Literal(ty) => self.find_literals(ty, text, start).is_some(),
            #[cfg(feature = "perf-dfa")]
            MatchType::Dfa | MatchType::DfaMany => {
                match self.shortest_dfa(text, start) {
                    dfa::Result::Match(_) => true,
                    dfa::Result::NoMatch(_) => false,
                    dfa::Result::Quit => self.match_nfa(text, start),
                }
            }
            #[cfg(feature = "perf-dfa")]
            MatchType::DfaAnchoredReverse => {
                match dfa::Fsm::reverse(
                    &self.ro.dfa_reverse,
                    self.cache.value(),
                    true,
                    &text[start..],
                    text.len(),
                ) {
                    dfa::Result::Match(_) => true,
                    dfa::Result::NoMatch(_) => false,
                    dfa::Result::Quit => self.match_nfa(text, start),
                }
            }
            #[cfg(all(feature = "perf-dfa", feature = "perf-literal"))]
            MatchType::DfaSuffix => {
                match self.shortest_dfa_reverse_suffix(text, start) {
                    dfa::Result::Match(_) => true,
                    dfa::Result::NoMatch(_) => false,
                    dfa::Result::Quit => self.match_nfa(text, start),
                }
            }
            MatchType::Nfa(ty) => self.match_nfa_type(ty, text, start),
            MatchType::Nothing => false,
        }
    }
    /// Finds the start and end location of the leftmost-first match, starting
    /// at the given location.
    #[cfg_attr(feature = "perf-inline", inline(always))]
    fn find_at(&self, text: &[u8], start: usize) -> Option<(usize, usize)> {
        if !self.is_anchor_end_match(text) {
            return None;
        }
        match self.ro.match_type {
            #[cfg(feature = "perf-literal")]
            MatchType::Literal(ty) => self.find_literals(ty, text, start),
            #[cfg(feature = "perf-dfa")]
            MatchType::Dfa => {
                match self.find_dfa_forward(text, start) {
                    dfa::Result::Match((s, e)) => Some((s, e)),
                    dfa::Result::NoMatch(_) => None,
                    dfa::Result::Quit => self.find_nfa(MatchNfaType::Auto, text, start),
                }
            }
            #[cfg(feature = "perf-dfa")]
            MatchType::DfaAnchoredReverse => {
                match self.find_dfa_anchored_reverse(text, start) {
                    dfa::Result::Match((s, e)) => Some((s, e)),
                    dfa::Result::NoMatch(_) => None,
                    dfa::Result::Quit => self.find_nfa(MatchNfaType::Auto, text, start),
                }
            }
            #[cfg(all(feature = "perf-dfa", feature = "perf-literal"))]
            MatchType::DfaSuffix => {
                match self.find_dfa_reverse_suffix(text, start) {
                    dfa::Result::Match((s, e)) => Some((s, e)),
                    dfa::Result::NoMatch(_) => None,
                    dfa::Result::Quit => self.find_nfa(MatchNfaType::Auto, text, start),
                }
            }
            MatchType::Nfa(ty) => self.find_nfa(ty, text, start),
            MatchType::Nothing => None,
            #[cfg(feature = "perf-dfa")]
            MatchType::DfaMany => unreachable!("BUG: RegexSet cannot be used with find"),
        }
    }
    /// Finds the start and end location of the leftmost-first match and also
    /// fills in all matching capture groups.
    ///
    /// The number of capture slots given should be equal to the total number
    /// of capture slots in the compiled program.
    ///
    /// Note that the first two slots always correspond to the start and end
    /// locations of the overall match.
    fn captures_read_at(
        &self,
        locs: &mut Locations,
        text: &[u8],
        start: usize,
    ) -> Option<(usize, usize)> {
        let slots = locs.as_slots();
        for slot in slots.iter_mut() {
            *slot = None;
        }
        match slots.len() {
            0 => return self.find_at(text, start),
            2 => {
                return self
                    .find_at(text, start)
                    .map(|(s, e)| {
                        slots[0] = Some(s);
                        slots[1] = Some(e);
                        (s, e)
                    });
            }
            _ => {}
        }
        if !self.is_anchor_end_match(text) {
            return None;
        }
        match self.ro.match_type {
            #[cfg(feature = "perf-literal")]
            MatchType::Literal(ty) => {
                self
                    .find_literals(ty, text, start)
                    .and_then(|(s, e)| {
                        self.captures_nfa_type(MatchNfaType::Auto, slots, text, s, e)
                    })
            }
            #[cfg(feature = "perf-dfa")]
            MatchType::Dfa => {
                if self.ro.nfa.is_anchored_start {
                    self.captures_nfa(slots, text, start)
                } else {
                    match self.find_dfa_forward(text, start) {
                        dfa::Result::Match((s, e)) => {
                            self.captures_nfa_type(MatchNfaType::Auto, slots, text, s, e)
                        }
                        dfa::Result::NoMatch(_) => None,
                        dfa::Result::Quit => self.captures_nfa(slots, text, start),
                    }
                }
            }
            #[cfg(feature = "perf-dfa")]
            MatchType::DfaAnchoredReverse => {
                match self.find_dfa_anchored_reverse(text, start) {
                    dfa::Result::Match((s, e)) => {
                        self.captures_nfa_type(MatchNfaType::Auto, slots, text, s, e)
                    }
                    dfa::Result::NoMatch(_) => None,
                    dfa::Result::Quit => self.captures_nfa(slots, text, start),
                }
            }
            #[cfg(all(feature = "perf-dfa", feature = "perf-literal"))]
            MatchType::DfaSuffix => {
                match self.find_dfa_reverse_suffix(text, start) {
                    dfa::Result::Match((s, e)) => {
                        self.captures_nfa_type(MatchNfaType::Auto, slots, text, s, e)
                    }
                    dfa::Result::NoMatch(_) => None,
                    dfa::Result::Quit => self.captures_nfa(slots, text, start),
                }
            }
            MatchType::Nfa(ty) => {
                self.captures_nfa_type(ty, slots, text, start, text.len())
            }
            MatchType::Nothing => None,
            #[cfg(feature = "perf-dfa")]
            MatchType::DfaMany => {
                unreachable!("BUG: RegexSet cannot be used with captures")
            }
        }
    }
}
impl<'c> ExecNoSync<'c> {
    /// Finds the leftmost-first match using only literal search.
    #[cfg(feature = "perf-literal")]
    #[cfg_attr(feature = "perf-inline", inline(always))]
    fn find_literals(
        &self,
        ty: MatchLiteralType,
        text: &[u8],
        start: usize,
    ) -> Option<(usize, usize)> {
        use self::MatchLiteralType::*;
        match ty {
            Unanchored => {
                let lits = &self.ro.nfa.prefixes;
                lits.find(&text[start..]).map(|(s, e)| (start + s, start + e))
            }
            AnchoredStart => {
                let lits = &self.ro.nfa.prefixes;
                if start == 0 || !self.ro.nfa.is_anchored_start {
                    lits.find_start(&text[start..]).map(|(s, e)| (start + s, start + e))
                } else {
                    None
                }
            }
            AnchoredEnd => {
                let lits = &self.ro.suffixes;
                lits.find_end(&text[start..]).map(|(s, e)| (start + s, start + e))
            }
            AhoCorasick => {
                self
                    .ro
                    .ac
                    .as_ref()
                    .unwrap()
                    .find(&text[start..])
                    .map(|m| (start + m.start(), start + m.end()))
            }
        }
    }
    /// Finds the leftmost-first match (start and end) using only the DFA.
    ///
    /// If the result returned indicates that the DFA quit, then another
    /// matching engine should be used.
    #[cfg(feature = "perf-dfa")]
    #[cfg_attr(feature = "perf-inline", inline(always))]
    fn find_dfa_forward(
        &self,
        text: &[u8],
        start: usize,
    ) -> dfa::Result<(usize, usize)> {
        use dfa::Result::*;
        let end = match dfa::Fsm::forward(
            &self.ro.dfa,
            self.cache.value(),
            false,
            text,
            start,
        ) {
            NoMatch(i) => return NoMatch(i),
            Quit => return Quit,
            Match(end) if start == end => return Match((start, start)),
            Match(end) => end,
        };
        match dfa::Fsm::reverse(
            &self.ro.dfa_reverse,
            self.cache.value(),
            false,
            &text[start..],
            end - start,
        ) {
            Match(s) => Match((start + s, end)),
            NoMatch(i) => NoMatch(i),
            Quit => Quit,
        }
    }
    /// Finds the leftmost-first match (start and end) using only the DFA,
    /// but assumes the regex is anchored at the end and therefore starts at
    /// the end of the regex and matches in reverse.
    ///
    /// If the result returned indicates that the DFA quit, then another
    /// matching engine should be used.
    #[cfg(feature = "perf-dfa")]
    #[cfg_attr(feature = "perf-inline", inline(always))]
    fn find_dfa_anchored_reverse(
        &self,
        text: &[u8],
        start: usize,
    ) -> dfa::Result<(usize, usize)> {
        use dfa::Result::*;
        match dfa::Fsm::reverse(
            &self.ro.dfa_reverse,
            self.cache.value(),
            false,
            &text[start..],
            text.len() - start,
        ) {
            Match(s) => Match((start + s, text.len())),
            NoMatch(i) => NoMatch(i),
            Quit => Quit,
        }
    }
    /// Finds the end of the shortest match using only the DFA.
    #[cfg(feature = "perf-dfa")]
    #[cfg_attr(feature = "perf-inline", inline(always))]
    fn shortest_dfa(&self, text: &[u8], start: usize) -> dfa::Result<usize> {
        dfa::Fsm::forward(&self.ro.dfa, self.cache.value(), true, text, start)
    }
    /// Finds the end of the shortest match using only the DFA by scanning for
    /// suffix literals.
    #[cfg(all(feature = "perf-dfa", feature = "perf-literal"))]
    #[cfg_attr(feature = "perf-inline", inline(always))]
    fn shortest_dfa_reverse_suffix(
        &self,
        text: &[u8],
        start: usize,
    ) -> dfa::Result<usize> {
        match self.exec_dfa_reverse_suffix(text, start) {
            None => self.shortest_dfa(text, start),
            Some(r) => r.map(|(_, end)| end),
        }
    }
    /// Finds the end of the shortest match using only the DFA by scanning for
    /// suffix literals. It also reports the start of the match.
    ///
    /// Note that if None is returned, then the optimization gave up to avoid
    /// worst case quadratic behavior. A forward scanning DFA should be tried
    /// next.
    ///
    /// If a match is returned and the full leftmost-first match is desired,
    /// then a forward scan starting from the beginning of the match must be
    /// done.
    ///
    /// If the result returned indicates that the DFA quit, then another
    /// matching engine should be used.
    #[cfg(all(feature = "perf-dfa", feature = "perf-literal"))]
    #[cfg_attr(feature = "perf-inline", inline(always))]
    fn exec_dfa_reverse_suffix(
        &self,
        text: &[u8],
        original_start: usize,
    ) -> Option<dfa::Result<(usize, usize)>> {
        use dfa::Result::*;
        let lcs = self.ro.suffixes.lcs();
        debug_assert!(lcs.len() >= 1);
        let mut start = original_start;
        let mut end = start;
        let mut last_literal = start;
        while end <= text.len() {
            last_literal
                += match lcs.find(&text[last_literal..]) {
                    None => return Some(NoMatch(text.len())),
                    Some(i) => i,
                };
            end = last_literal + lcs.len();
            match dfa::Fsm::reverse(
                &self.ro.dfa_reverse,
                self.cache.value(),
                false,
                &text[start..end],
                end - start,
            ) {
                Match(0) | NoMatch(0) => return None,
                Match(i) => return Some(Match((start + i, end))),
                NoMatch(i) => {
                    start += i;
                    last_literal += 1;
                    continue;
                }
                Quit => return Some(Quit),
            };
        }
        Some(NoMatch(text.len()))
    }
    /// Finds the leftmost-first match (start and end) using only the DFA
    /// by scanning for suffix literals.
    ///
    /// If the result returned indicates that the DFA quit, then another
    /// matching engine should be used.
    #[cfg(all(feature = "perf-dfa", feature = "perf-literal"))]
    #[cfg_attr(feature = "perf-inline", inline(always))]
    fn find_dfa_reverse_suffix(
        &self,
        text: &[u8],
        start: usize,
    ) -> dfa::Result<(usize, usize)> {
        use dfa::Result::*;
        let match_start = match self.exec_dfa_reverse_suffix(text, start) {
            None => return self.find_dfa_forward(text, start),
            Some(Match((start, _))) => start,
            Some(r) => return r,
        };
        match dfa::Fsm::forward(
            &self.ro.dfa,
            self.cache.value(),
            false,
            text,
            match_start,
        ) {
            NoMatch(_) => panic!("BUG: reverse match implies forward match"),
            Quit => Quit,
            Match(e) => Match((match_start, e)),
        }
    }
    /// Executes the NFA engine to return whether there is a match or not.
    ///
    /// Ideally, we could use shortest_nfa(...).is_some() and get the same
    /// performance characteristics, but regex sets don't have captures, which
    /// shortest_nfa depends on.
    #[cfg(feature = "perf-dfa")]
    fn match_nfa(&self, text: &[u8], start: usize) -> bool {
        self.match_nfa_type(MatchNfaType::Auto, text, start)
    }
    /// Like match_nfa, but allows specification of the type of NFA engine.
    fn match_nfa_type(&self, ty: MatchNfaType, text: &[u8], start: usize) -> bool {
        self.exec_nfa(ty, &mut [false], &mut [], true, false, text, start, text.len())
    }
    /// Finds the shortest match using an NFA.
    #[cfg(feature = "perf-dfa")]
    fn shortest_nfa(&self, text: &[u8], start: usize) -> Option<usize> {
        self.shortest_nfa_type(MatchNfaType::Auto, text, start)
    }
    /// Like shortest_nfa, but allows specification of the type of NFA engine.
    fn shortest_nfa_type(
        &self,
        ty: MatchNfaType,
        text: &[u8],
        start: usize,
    ) -> Option<usize> {
        let mut slots = [None, None];
        if self
            .exec_nfa(ty, &mut [false], &mut slots, true, true, text, start, text.len())
        {
            slots[1]
        } else {
            None
        }
    }
    /// Like find, but executes an NFA engine.
    fn find_nfa(
        &self,
        ty: MatchNfaType,
        text: &[u8],
        start: usize,
    ) -> Option<(usize, usize)> {
        let mut slots = [None, None];
        if self
            .exec_nfa(
                ty,
                &mut [false],
                &mut slots,
                false,
                false,
                text,
                start,
                text.len(),
            )
        {
            match (slots[0], slots[1]) {
                (Some(s), Some(e)) => Some((s, e)),
                _ => None,
            }
        } else {
            None
        }
    }
    /// Like find_nfa, but fills in captures.
    ///
    /// `slots` should have length equal to `2 * nfa.captures.len()`.
    #[cfg(feature = "perf-dfa")]
    fn captures_nfa(
        &self,
        slots: &mut [Slot],
        text: &[u8],
        start: usize,
    ) -> Option<(usize, usize)> {
        self.captures_nfa_type(MatchNfaType::Auto, slots, text, start, text.len())
    }
    /// Like captures_nfa, but allows specification of type of NFA engine.
    fn captures_nfa_type(
        &self,
        ty: MatchNfaType,
        slots: &mut [Slot],
        text: &[u8],
        start: usize,
        end: usize,
    ) -> Option<(usize, usize)> {
        if self.exec_nfa(ty, &mut [false], slots, false, false, text, start, end) {
            match (slots[0], slots[1]) {
                (Some(s), Some(e)) => Some((s, e)),
                _ => None,
            }
        } else {
            None
        }
    }
    fn exec_nfa(
        &self,
        mut ty: MatchNfaType,
        matches: &mut [bool],
        slots: &mut [Slot],
        quit_after_match: bool,
        quit_after_match_with_pos: bool,
        text: &[u8],
        start: usize,
        end: usize,
    ) -> bool {
        use self::MatchNfaType::*;
        if let Auto = ty {
            if backtrack::should_exec(self.ro.nfa.len(), text.len()) {
                ty = Backtrack;
            } else {
                ty = PikeVM;
            }
        }
        if quit_after_match_with_pos || ty == PikeVM {
            self.exec_pikevm(matches, slots, quit_after_match, text, start, end)
        } else {
            self.exec_backtrack(matches, slots, text, start, end)
        }
    }
    /// Always run the NFA algorithm.
    fn exec_pikevm(
        &self,
        matches: &mut [bool],
        slots: &mut [Slot],
        quit_after_match: bool,
        text: &[u8],
        start: usize,
        end: usize,
    ) -> bool {
        if self.ro.nfa.uses_bytes() {
            pikevm::Fsm::exec(
                &self.ro.nfa,
                self.cache.value(),
                matches,
                slots,
                quit_after_match,
                ByteInput::new(text, self.ro.nfa.only_utf8),
                start,
                end,
            )
        } else {
            pikevm::Fsm::exec(
                &self.ro.nfa,
                self.cache.value(),
                matches,
                slots,
                quit_after_match,
                CharInput::new(text),
                start,
                end,
            )
        }
    }
    /// Always runs the NFA using bounded backtracking.
    fn exec_backtrack(
        &self,
        matches: &mut [bool],
        slots: &mut [Slot],
        text: &[u8],
        start: usize,
        end: usize,
    ) -> bool {
        if self.ro.nfa.uses_bytes() {
            backtrack::Bounded::exec(
                &self.ro.nfa,
                self.cache.value(),
                matches,
                slots,
                ByteInput::new(text, self.ro.nfa.only_utf8),
                start,
                end,
            )
        } else {
            backtrack::Bounded::exec(
                &self.ro.nfa,
                self.cache.value(),
                matches,
                slots,
                CharInput::new(text),
                start,
                end,
            )
        }
    }
    /// Finds which regular expressions match the given text.
    ///
    /// `matches` should have length equal to the number of regexes being
    /// searched.
    ///
    /// This is only useful when one wants to know which regexes in a set
    /// match some text.
    pub fn many_matches_at(
        &self,
        matches: &mut [bool],
        text: &[u8],
        start: usize,
    ) -> bool {
        use self::MatchType::*;
        if !self.is_anchor_end_match(text) {
            return false;
        }
        match self.ro.match_type {
            #[cfg(feature = "perf-literal")]
            Literal(ty) => {
                debug_assert_eq!(matches.len(), 1);
                matches[0] = self.find_literals(ty, text, start).is_some();
                matches[0]
            }
            #[cfg(feature = "perf-dfa")]
            Dfa | DfaAnchoredReverse | DfaMany => {
                match dfa::Fsm::forward_many(
                    &self.ro.dfa,
                    self.cache.value(),
                    matches,
                    text,
                    start,
                ) {
                    dfa::Result::Match(_) => true,
                    dfa::Result::NoMatch(_) => false,
                    dfa::Result::Quit => {
                        self
                            .exec_nfa(
                                MatchNfaType::Auto,
                                matches,
                                &mut [],
                                false,
                                false,
                                text,
                                start,
                                text.len(),
                            )
                    }
                }
            }
            #[cfg(all(feature = "perf-dfa", feature = "perf-literal"))]
            DfaSuffix => {
                match dfa::Fsm::forward_many(
                    &self.ro.dfa,
                    self.cache.value(),
                    matches,
                    text,
                    start,
                ) {
                    dfa::Result::Match(_) => true,
                    dfa::Result::NoMatch(_) => false,
                    dfa::Result::Quit => {
                        self
                            .exec_nfa(
                                MatchNfaType::Auto,
                                matches,
                                &mut [],
                                false,
                                false,
                                text,
                                start,
                                text.len(),
                            )
                    }
                }
            }
            Nfa(ty) => {
                self
                    .exec_nfa(
                        ty,
                        matches,
                        &mut [],
                        false,
                        false,
                        text,
                        start,
                        text.len(),
                    )
            }
            Nothing => false,
        }
    }
    #[cfg_attr(feature = "perf-inline", inline(always))]
    fn is_anchor_end_match(&self, text: &[u8]) -> bool {
        #[cfg(not(feature = "perf-literal"))]
        fn imp(_: &ExecReadOnly, _: &[u8]) -> bool {
            true
        }
        #[cfg(feature = "perf-literal")]
        fn imp(ro: &ExecReadOnly, text: &[u8]) -> bool {
            if text.len() > (1 << 20) && ro.nfa.is_anchored_end {
                let lcs = ro.suffixes.lcs();
                if lcs.len() >= 1 && !lcs.is_suffix(text) {
                    return false;
                }
            }
            true
        }
        imp(&self.ro, text)
    }
    pub fn capture_name_idx(&self) -> &Arc<HashMap<String, usize>> {
        &self.ro.nfa.capture_name_idx
    }
}
impl<'c> ExecNoSyncStr<'c> {
    pub fn capture_name_idx(&self) -> &Arc<HashMap<String, usize>> {
        self.0.capture_name_idx()
    }
}
impl Exec {
    /// Get a searcher that isn't Sync.
    #[cfg_attr(feature = "perf-inline", inline(always))]
    pub fn searcher(&self) -> ExecNoSync {
        let create = || RefCell::new(ProgramCacheInner::new(&self.ro));
        ExecNoSync {
            ro: &self.ro,
            cache: self.cache.get_or(create),
        }
    }
    /// Get a searcher that isn't Sync and can match on &str.
    #[cfg_attr(feature = "perf-inline", inline(always))]
    pub fn searcher_str(&self) -> ExecNoSyncStr {
        ExecNoSyncStr(self.searcher())
    }
    /// Build a Regex from this executor.
    pub fn into_regex(self) -> re_unicode::Regex {
        re_unicode::Regex::from(self)
    }
    /// Build a RegexSet from this executor.
    pub fn into_regex_set(self) -> re_set::unicode::RegexSet {
        re_set::unicode::RegexSet::from(self)
    }
    /// Build a Regex from this executor that can match arbitrary bytes.
    pub fn into_byte_regex(self) -> re_bytes::Regex {
        re_bytes::Regex::from(self)
    }
    /// Build a RegexSet from this executor that can match arbitrary bytes.
    pub fn into_byte_regex_set(self) -> re_set::bytes::RegexSet {
        re_set::bytes::RegexSet::from(self)
    }
    /// The original regular expressions given by the caller that were
    /// compiled.
    pub fn regex_strings(&self) -> &[String] {
        &self.ro.res
    }
    /// Return a slice of capture names.
    ///
    /// Any capture that isn't named is None.
    pub fn capture_names(&self) -> &[Option<String>] {
        &self.ro.nfa.captures
    }
    /// Return a reference to named groups mapping (from group name to
    /// group position).
    pub fn capture_name_idx(&self) -> &Arc<HashMap<String, usize>> {
        &self.ro.nfa.capture_name_idx
    }
}
impl Clone for Exec {
    fn clone(&self) -> Exec {
        Exec {
            ro: self.ro.clone(),
            cache: Cached::new(),
        }
    }
}
impl ExecReadOnly {
    fn choose_match_type(&self, hint: Option<MatchType>) -> MatchType {
        if let Some(MatchType::Nfa(_)) = hint {
            return hint.unwrap();
        }
        if self.nfa.insts.is_empty() {
            return MatchType::Nothing;
        }
        if let Some(literalty) = self.choose_literal_match_type() {
            return literalty;
        }
        if let Some(dfaty) = self.choose_dfa_match_type() {
            return dfaty;
        }
        MatchType::Nfa(MatchNfaType::Auto)
    }
    /// If a plain literal scan can be used, then a corresponding literal
    /// search type is returned.
    fn choose_literal_match_type(&self) -> Option<MatchType> {
        #[cfg(not(feature = "perf-literal"))]
        fn imp(_: &ExecReadOnly) -> Option<MatchType> {
            None
        }
        #[cfg(feature = "perf-literal")]
        fn imp(ro: &ExecReadOnly) -> Option<MatchType> {
            if ro.res.len() != 1 {
                return None;
            }
            if ro.ac.is_some() {
                return Some(MatchType::Literal(MatchLiteralType::AhoCorasick));
            }
            if ro.nfa.prefixes.complete() {
                return if ro.nfa.is_anchored_start {
                    Some(MatchType::Literal(MatchLiteralType::AnchoredStart))
                } else {
                    Some(MatchType::Literal(MatchLiteralType::Unanchored))
                };
            }
            if ro.suffixes.complete() {
                return if ro.nfa.is_anchored_end {
                    Some(MatchType::Literal(MatchLiteralType::AnchoredEnd))
                } else {
                    Some(MatchType::Literal(MatchLiteralType::Unanchored))
                };
            }
            None
        }
        imp(self)
    }
    /// If a DFA scan can be used, then choose the appropriate DFA strategy.
    fn choose_dfa_match_type(&self) -> Option<MatchType> {
        #[cfg(not(feature = "perf-dfa"))]
        fn imp(_: &ExecReadOnly) -> Option<MatchType> {
            None
        }
        #[cfg(feature = "perf-dfa")]
        fn imp(ro: &ExecReadOnly) -> Option<MatchType> {
            if !dfa::can_exec(&ro.dfa) {
                return None;
            }
            if ro.res.len() >= 2 {
                return Some(MatchType::DfaMany);
            }
            if !ro.nfa.is_anchored_start && ro.nfa.is_anchored_end {
                return Some(MatchType::DfaAnchoredReverse);
            }
            #[cfg(feature = "perf-literal")]
            {
                if ro.should_suffix_scan() {
                    return Some(MatchType::DfaSuffix);
                }
            }
            Some(MatchType::Dfa)
        }
        imp(self)
    }
    /// Returns true if the program is amenable to suffix scanning.
    ///
    /// When this is true, as a heuristic, we assume it is OK to quickly scan
    /// for suffix literals and then do a *reverse* DFA match from any matches
    /// produced by the literal scan. (And then followed by a forward DFA
    /// search, since the previously found suffix literal maybe not actually be
    /// the end of a match.)
    ///
    /// This is a bit of a specialized optimization, but can result in pretty
    /// big performance wins if 1) there are no prefix literals and 2) the
    /// suffix literals are pretty rare in the text. (1) is obviously easy to
    /// account for but (2) is harder. As a proxy, we assume that longer
    /// strings are generally rarer, so we only enable this optimization when
    /// we have a meaty suffix.
    #[cfg(all(feature = "perf-dfa", feature = "perf-literal"))]
    fn should_suffix_scan(&self) -> bool {
        if self.suffixes.is_empty() {
            return false;
        }
        let lcs_len = self.suffixes.lcs().char_len();
        lcs_len >= 3 && lcs_len > self.dfa.prefixes.lcp().char_len()
    }
}
#[derive(Clone, Copy, Debug)]
enum MatchType {
    /// A single or multiple literal search. This is only used when the regex
    /// can be decomposed into a literal search.
    #[cfg(feature = "perf-literal")]
    Literal(MatchLiteralType),
    /// A normal DFA search.
    #[cfg(feature = "perf-dfa")]
    Dfa,
    /// A reverse DFA search starting from the end of a haystack.
    #[cfg(feature = "perf-dfa")]
    DfaAnchoredReverse,
    /// A reverse DFA search with suffix literal scanning.
    #[cfg(all(feature = "perf-dfa", feature = "perf-literal"))]
    DfaSuffix,
    /// Use the DFA on two or more regular expressions.
    #[cfg(feature = "perf-dfa")]
    DfaMany,
    /// An NFA variant.
    Nfa(MatchNfaType),
    /// No match is ever possible, so don't ever try to search.
    Nothing,
}
#[derive(Clone, Copy, Debug)]
#[cfg(feature = "perf-literal")]
enum MatchLiteralType {
    /// Match literals anywhere in text.
    Unanchored,
    /// Match literals only at the start of text.
    AnchoredStart,
    /// Match literals only at the end of text.
    AnchoredEnd,
    /// Use an Aho-Corasick automaton. This requires `ac` to be Some on
    /// ExecReadOnly.
    AhoCorasick,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MatchNfaType {
    /// Choose between Backtrack and PikeVM.
    Auto,
    /// NFA bounded backtracking.
    ///
    /// (This is only set by tests, since it never makes sense to always want
    /// backtracking.)
    Backtrack,
    /// The Pike VM.
    ///
    /// (This is only set by tests, since it never makes sense to always want
    /// the Pike VM.)
    PikeVM,
}
/// `ProgramCache` maintains reusable allocations for each matching engine
/// available to a particular program.
pub type ProgramCache = RefCell<ProgramCacheInner>;
#[derive(Debug)]
pub struct ProgramCacheInner {
    pub pikevm: pikevm::Cache,
    pub backtrack: backtrack::Cache,
    #[cfg(feature = "perf-dfa")]
    pub dfa: dfa::Cache,
    #[cfg(feature = "perf-dfa")]
    pub dfa_reverse: dfa::Cache,
}
impl ProgramCacheInner {
    fn new(ro: &ExecReadOnly) -> Self {
        ProgramCacheInner {
            pikevm: pikevm::Cache::new(&ro.nfa),
            backtrack: backtrack::Cache::new(&ro.nfa),
            #[cfg(feature = "perf-dfa")]
            dfa: dfa::Cache::new(&ro.dfa),
            #[cfg(feature = "perf-dfa")]
            dfa_reverse: dfa::Cache::new(&ro.dfa_reverse),
        }
    }
}
/// Alternation literals checks if the given HIR is a simple alternation of
/// literals, and if so, returns them. Otherwise, this returns None.
#[cfg(feature = "perf-literal")]
fn alternation_literals(expr: &Hir) -> Option<Vec<Vec<u8>>> {
    use syntax::hir::{HirKind, Literal};
    if !expr.is_alternation_literal() {
        return None;
    }
    let alts = match *expr.kind() {
        HirKind::Alternation(ref alts) => alts,
        _ => return None,
    };
    let extendlit = |lit: &Literal, dst: &mut Vec<u8>| match *lit {
        Literal::Unicode(c) => {
            let mut buf = [0; 4];
            dst.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
        }
        Literal::Byte(b) => {
            dst.push(b);
        }
    };
    let mut lits = vec![];
    for alt in alts {
        let mut lit = vec![];
        match *alt.kind() {
            HirKind::Literal(ref x) => extendlit(x, &mut lit),
            HirKind::Concat(ref exprs) => {
                for e in exprs {
                    match *e.kind() {
                        HirKind::Literal(ref x) => extendlit(x, &mut lit),
                        _ => unreachable!("expected literal, got {:?}", e),
                    }
                }
            }
            _ => unreachable!("expected literal or concat, got {:?}", alt),
        }
        lits.push(lit);
    }
    Some(lits)
}
#[cfg(test)]
mod test {
    #[test]
    fn uppercut_s_backtracking_bytes_default_bytes_mismatch() {
        use internal::ExecBuilder;
        let backtrack_bytes_re = ExecBuilder::new("^S")
            .bounded_backtracking()
            .only_utf8(false)
            .build()
            .map(|exec| exec.into_byte_regex())
            .map_err(|err| format!("{}", err))
            .unwrap();
        let default_bytes_re = ExecBuilder::new("^S")
            .only_utf8(false)
            .build()
            .map(|exec| exec.into_byte_regex())
            .map_err(|err| format!("{}", err))
            .unwrap();
        let input = vec![83, 83];
        let s1 = backtrack_bytes_re.split(&input);
        let s2 = default_bytes_re.split(&input);
        for (chunk1, chunk2) in s1.zip(s2) {
            assert_eq!(chunk1, chunk2);
        }
    }
    #[test]
    fn unicode_lit_star_backtracking_utf8bytes_default_utf8bytes_mismatch() {
        use internal::ExecBuilder;
        let backtrack_bytes_re = ExecBuilder::new(r"^(?u:\*)")
            .bounded_backtracking()
            .bytes(true)
            .build()
            .map(|exec| exec.into_regex())
            .map_err(|err| format!("{}", err))
            .unwrap();
        let default_bytes_re = ExecBuilder::new(r"^(?u:\*)")
            .bytes(true)
            .build()
            .map(|exec| exec.into_regex())
            .map_err(|err| format!("{}", err))
            .unwrap();
        let input = "**";
        let s1 = backtrack_bytes_re.split(input);
        let s2 = default_bytes_re.split(input);
        for (chunk1, chunk2) in s1.zip(s2) {
            assert_eq!(chunk1, chunk2);
        }
    }
}
#[cfg(test)]
mod tests_llm_16_309 {
    use super::*;
    use crate::*;
    use crate::Regex;
    #[test]
    fn test_build() {
        let _rug_st_tests_llm_16_309_rrrruuuugggg_test_build = 0;
        let rug_fuzz_0 = r"ab+c[de]*fg";
        let rug_fuzz_1 = true;
        let rug_fuzz_2 = false;
        let rug_fuzz_3 = true;
        let re = Regex::new(rug_fuzz_0).unwrap();
        let builder = ExecBuilder::new(re.as_str())
            .unicode(rug_fuzz_1)
            .bytes(rug_fuzz_2)
            .only_utf8(rug_fuzz_3)
            .automatic();
        let result = builder.build();
        debug_assert!(result.is_ok());
        let _rug_ed_tests_llm_16_309_rrrruuuugggg_test_build = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_312 {
    use super::*;
    use crate::*;
    use crate::exec::{ExecBuilder, Parsed};
    #[test]
    fn test_bytes() {
        let _rug_st_tests_llm_16_312_rrrruuuugggg_test_bytes = 0;
        let rug_fuzz_0 = true;
        let mut exec_builder = ExecBuilder::new_options(RegexOptions::default());
        let bytes = rug_fuzz_0;
        let result = exec_builder.bytes(bytes).build();
        let _rug_ed_tests_llm_16_312_rrrruuuugggg_test_bytes = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_314 {
    use super::*;
    use crate::*;
    #[test]
    fn test_exec_builder_new() {
        let _rug_st_tests_llm_16_314_rrrruuuugggg_test_exec_builder_new = 0;
        let rug_fuzz_0 = "test_regex";
        let re = rug_fuzz_0;
        let exec_builder = exec::ExecBuilder::new(re);
        debug_assert_eq!(exec_builder.options.pats, vec![re.to_owned()]);
        debug_assert_eq!(exec_builder.options.size_limit, 10 * (1 << 20));
        debug_assert_eq!(exec_builder.options.dfa_size_limit, 2 * (1 << 20));
        debug_assert_eq!(exec_builder.options.nest_limit, 250);
        debug_assert_eq!(exec_builder.options.case_insensitive, false);
        debug_assert_eq!(exec_builder.options.multi_line, false);
        debug_assert_eq!(exec_builder.options.dot_matches_new_line, false);
        debug_assert_eq!(exec_builder.options.swap_greed, false);
        debug_assert_eq!(exec_builder.options.ignore_whitespace, false);
        debug_assert_eq!(exec_builder.options.unicode, true);
        debug_assert_eq!(exec_builder.options.octal, false);
        debug_assert!(exec_builder.match_type.is_none());
        debug_assert_eq!(exec_builder.bytes, false);
        debug_assert_eq!(exec_builder.only_utf8, true);
        let _rug_ed_tests_llm_16_314_rrrruuuugggg_test_exec_builder_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_317 {
    use crate::exec::{ExecBuilder, RegexOptions};
    #[test]
    fn test_new_options() {
        let _rug_st_tests_llm_16_317_rrrruuuugggg_test_new_options = 0;
        let opts = RegexOptions::default();
        let builder = ExecBuilder::new_options(opts);
        let _rug_ed_tests_llm_16_317_rrrruuuugggg_test_new_options = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_318 {
    use super::*;
    use crate::*;
    #[test]
    fn test_nfa() {
        let _rug_st_tests_llm_16_318_rrrruuuugggg_test_nfa = 0;
        let rug_fuzz_0 = "regex";
        let builder = ExecBuilder::new(rug_fuzz_0);
        let result = builder.nfa();
        let _rug_ed_tests_llm_16_318_rrrruuuugggg_test_nfa = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_320 {
    use super::*;
    use crate::*;
    use crate::re_builder::RegexOptions;
    #[test]
    fn test_only_utf8() {
        let _rug_st_tests_llm_16_320_rrrruuuugggg_test_only_utf8 = 0;
        let rug_fuzz_0 = "(abc|def)";
        let rug_fuzz_1 = false;
        let re = rug_fuzz_0;
        let mut builder = ExecBuilder::new(re);
        let expected = rug_fuzz_1;
        let actual = builder.only_utf8(expected).only_utf8;
        debug_assert_eq!(actual, expected);
        let _rug_ed_tests_llm_16_320_rrrruuuugggg_test_only_utf8 = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_321 {
    use super::*;
    use crate::*;
    #[test]
    fn test_parse() {
        let _rug_st_tests_llm_16_321_rrrruuuugggg_test_parse = 0;
        let rug_fuzz_0 = "abc";
        let rug_fuzz_1 = 10;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 20;
        let rug_fuzz_4 = 2;
        let rug_fuzz_5 = 1;
        let rug_fuzz_6 = 20;
        let rug_fuzz_7 = 250;
        let rug_fuzz_8 = false;
        let rug_fuzz_9 = false;
        let rug_fuzz_10 = false;
        let rug_fuzz_11 = false;
        let rug_fuzz_12 = false;
        let rug_fuzz_13 = true;
        let rug_fuzz_14 = false;
        let options = RegexOptions {
            pats: vec![rug_fuzz_0.to_owned(), "def".to_owned()],
            size_limit: rug_fuzz_1 * (rug_fuzz_2 << rug_fuzz_3),
            dfa_size_limit: rug_fuzz_4 * (rug_fuzz_5 << rug_fuzz_6),
            nest_limit: rug_fuzz_7,
            case_insensitive: rug_fuzz_8,
            multi_line: rug_fuzz_9,
            dot_matches_new_line: rug_fuzz_10,
            swap_greed: rug_fuzz_11,
            ignore_whitespace: rug_fuzz_12,
            unicode: rug_fuzz_13,
            octal: rug_fuzz_14,
        };
        let exec_builder = ExecBuilder::new_options(options);
        let result = exec_builder.parse();
        debug_assert!(result.is_ok());
        let _rug_ed_tests_llm_16_321_rrrruuuugggg_test_parse = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_323 {
    use super::*;
    use crate::*;
    use re_builder::RegexOptions;
    #[test]
    fn test_unicode() {
        let _rug_st_tests_llm_16_323_rrrruuuugggg_test_unicode = 0;
        let rug_fuzz_0 = true;
        let options = RegexOptions::default();
        let builder = ExecBuilder::new_options(options);
        let result = builder.unicode(rug_fuzz_0);
        debug_assert!(result.options.unicode);
        let _rug_ed_tests_llm_16_323_rrrruuuugggg_test_unicode = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_324 {
    use super::*;
    use crate::*;
    #[test]
    fn test_capture_name_idx() {
        let _rug_st_tests_llm_16_324_rrrruuuugggg_test_capture_name_idx = 0;
        let _rug_ed_tests_llm_16_324_rrrruuuugggg_test_capture_name_idx = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_372 {
    use super::*;
    use crate::*;
    #[test]
    fn test_exec_program_cache_inner_new() {
        let _rug_st_tests_llm_16_372_rrrruuuugggg_test_exec_program_cache_inner_new = 0;
        let ro = ExecReadOnly {
            res: vec![],
            nfa: Program::new(),
            dfa: Program::new(),
            dfa_reverse: Program::new(),
            suffixes: LiteralSearcher::empty(),
            #[cfg(feature = "perf-literal")]
            ac: None,
            match_type: MatchType::Nothing,
        };
        let _ = ProgramCacheInner::new(&ro);
        let _rug_ed_tests_llm_16_372_rrrruuuugggg_test_exec_program_cache_inner_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_373 {
    use super::*;
    use crate::*;
    #[test]
    #[cfg(feature = "perf-literal")]
    fn test_alternation_literals() {
        let _rug_st_tests_llm_16_373_rrrruuuugggg_test_alternation_literals = 0;
        let rug_fuzz_0 = 'a';
        let rug_fuzz_1 = 'b';
        let rug_fuzz_2 = 'c';
        let rug_fuzz_3 = b"a";
        use syntax::hir::*;
        let literal1 = Hir::literal(Literal::Unicode(rug_fuzz_0));
        let literal2 = Hir::literal(Literal::Unicode(rug_fuzz_1));
        let literal3 = Hir::literal(Literal::Unicode(rug_fuzz_2));
        let alternation = Hir::alternation(vec![literal1, literal2, literal3]);
        let result = alternation_literals(&alternation);
        debug_assert!(result.is_some());
        let expected = Some(vec![rug_fuzz_3.to_vec(), b"b".to_vec(), b"c".to_vec()]);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_373_rrrruuuugggg_test_alternation_literals = 0;
    }
}
#[cfg(test)]
mod tests_rug_99 {
    use super::*;
    use crate::internal::ExecBuilder;
    #[test]
    fn test_regex() {
        let _rug_st_tests_rug_99_rrrruuuugggg_test_regex = 0;
        let rug_fuzz_0 = "regex pattern";
        let mut p0 = ExecBuilder::new(rug_fuzz_0);
        p0.automatic();
        let _rug_ed_tests_rug_99_rrrruuuugggg_test_regex = 0;
    }
}
#[cfg(test)]
mod tests_rug_100 {
    use super::*;
    use crate::internal::ExecBuilder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_100_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "regex pattern";
        let mut p0 = ExecBuilder::new(rug_fuzz_0);
        crate::exec::ExecBuilder::bounded_backtracking(p0);
        let _rug_ed_tests_rug_100_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_104 {
    use super::*;
    use crate::re_trait::RegularExpression;
    use crate::exec::ExecNoSyncStr;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_104_rrrruuuugggg_test_rug = 0;
        let mut p0: ExecNoSyncStr<'static> = unimplemented!();
        let mut p1: &str = unimplemented!();
        let mut p2: usize = unimplemented!();
        <ExecNoSyncStr<'static> as RegularExpression>::shortest_match_at(&p0, p1, p2);
        let _rug_ed_tests_rug_104_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_110 {
    use super::*;
    use crate::re_trait::RegularExpression;
    use crate::exec::ExecNoSync;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_110_rrrruuuugggg_test_rug = 0;
        let mut p0: ExecNoSync<'static> = unimplemented!();
        let p1: &[u8] = unimplemented!();
        let p2: usize = unimplemented!();
        <ExecNoSync<'static> as RegularExpression>::shortest_match_at(&mut p0, p1, p2);
        let _rug_ed_tests_rug_110_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_111 {
    use super::*;
    use crate::re_trait::RegularExpression;
    use crate::exec::{ExecNoSync, MatchType};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_111_rrrruuuugggg_test_rug = 0;
        let mut p0: ExecNoSync<'static> = unimplemented!();
        let mut p1: &[u8] = unimplemented!();
        let mut p2: usize = unimplemented!();
        <ExecNoSync<'static> as RegularExpression>::is_match_at(&p0, p1, p2);
        let _rug_ed_tests_rug_111_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_114 {
    use super::*;
    use crate::exec::{ExecNoSync, MatchLiteralType};
    #[test]
    fn test_find_literals() {
        let _rug_st_tests_rug_114_rrrruuuugggg_test_find_literals = 0;
        let mut p0: ExecNoSync<'_> = unimplemented!();
        let mut p1: MatchLiteralType = MatchLiteralType::Unanchored;
        let mut p2: &[u8] = unimplemented!();
        let mut p3: usize = unimplemented!();
        p0.find_literals(p1, p2, p3);
        let _rug_ed_tests_rug_114_rrrruuuugggg_test_find_literals = 0;
    }
}
#[cfg(test)]
mod tests_rug_118 {
    use super::*;
    use crate::exec::ExecNoSync;
    use crate::dfa;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_118_rrrruuuugggg_test_rug = 0;
        let mut p0: ExecNoSync<'_> = unimplemented!();
        let p1: &[u8] = unimplemented!();
        let p2: usize = unimplemented!();
        ExecNoSync::shortest_dfa_reverse_suffix(&mut p0, p1, p2);
        let _rug_ed_tests_rug_118_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_120 {
    use super::*;
    use crate::exec::ExecNoSync;
    use crate::dfa::Result;
    use crate::dfa::Fsm;
    #[test]
    fn test_find_dfa_reverse_suffix() {
        let exec_no_sync: ExecNoSync<'static> = todo!(
            "Construct ExecNoSync<'c> sample here"
        );
        let text: &[u8] = todo!("Construct sample &[u8] here");
        let start: usize = todo!("Construct sample usize here");
        let result: Result<(usize, usize)> = exec_no_sync
            .find_dfa_reverse_suffix(text, start);
    }
}
#[cfg(test)]
mod tests_rug_121 {
    use super::*;
    use crate::exec::ExecNoSync;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_121_rrrruuuugggg_test_rug = 0;
        let mut p0: ExecNoSync = unimplemented!();
        let p1: &[u8] = unimplemented!();
        let p2: usize = unimplemented!();
        p0.match_nfa(p1, p2);
        let _rug_ed_tests_rug_121_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_123 {
    use super::*;
    use crate::exec::ExecNoSync;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_123_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"example";
        let rug_fuzz_1 = 4;
        let mut p0: ExecNoSync<'static> = unimplemented!();
        let p1: &[u8] = rug_fuzz_0;
        let p2: usize = rug_fuzz_1;
        p0.shortest_nfa(p1, p2);
        let _rug_ed_tests_rug_123_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_125 {
    use super::*;
    use crate::exec::{ExecNoSync, MatchNfaType, Slot};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_125_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"Test input";
        let rug_fuzz_1 = 0;
        let mut p0: ExecNoSync<'static> = unimplemented!();
        let mut p1: &mut [Slot] = unimplemented!();
        let p2: &[u8] = rug_fuzz_0;
        let p3: usize = rug_fuzz_1;
        p0.captures_nfa(p1, p2, p3);
        let _rug_ed_tests_rug_125_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_127 {
    use super::*;
    use crate::exec::{ExecNoSync, MatchNfaType};
    use std::option::Option;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_127_rrrruuuugggg_sample = 0;
        let rug_fuzz_0 = 42;
        let mut p0: ExecNoSync<'_> = unimplemented!();
        let mut p1: MatchNfaType = {
            #[cfg(test)]
            mod tests_rug_127_prepare {
                use super::*;
                #[test]
                fn sample() {
                    let _rug_st_tests_rug_127_prepare_rrrruuuugggg_sample = 0;
                    let rug_fuzz_0 = 0;
                    let rug_fuzz_1 = 0;
                    let rug_fuzz_2 = 0;
                    let rug_fuzz_3 = 0;
                    let _rug_st_tests_rug_127_prepare_rrrruuuugggg_sample = rug_fuzz_0;
                    let rug_fuzz_0 = rug_fuzz_1;
                    let rug_fuzz_1 = rug_fuzz_2;
                    let _rug_st_tests_rug_127_rrrruuuugggg_sample = rug_fuzz_0;
                    let mut v44 = MatchNfaType::Auto;
                    let _rug_ed_tests_rug_127_rrrruuuugggg_sample = rug_fuzz_1;
                    let _rug_ed_tests_rug_127_prepare_rrrruuuugggg_sample = rug_fuzz_3;
                    let _rug_ed_tests_rug_127_prepare_rrrruuuugggg_sample = 0;
                }
            }
            MatchNfaType::Auto
        };
        let mut p2: &mut [bool] = unimplemented!();
        let mut p3: &mut [Option<usize>] = {
            #[cfg(test)]
            mod tests_rug_127_prepare {
                use super::*;
                #[test]
                fn sample() {
                    let _rug_st_tests_rug_127_prepare_rrrruuuugggg_sample = 0;
                    let rug_fuzz_0 = 0;
                    let rug_fuzz_1 = 0;
                    let rug_fuzz_2 = 42;
                    let rug_fuzz_3 = 0;
                    let rug_fuzz_4 = 0;
                    let _rug_st_tests_rug_127_prepare_rrrruuuugggg_sample = rug_fuzz_0;
                    let rug_fuzz_0 = rug_fuzz_1;
                    let rug_fuzz_1 = rug_fuzz_2;
                    let rug_fuzz_2 = rug_fuzz_3;
                    let _rug_st_tests_rug_127_rrrruuuugggg_sample = rug_fuzz_0;
                    let rug_fuzz_0 = rug_fuzz_1;
                    let mut v3: Option<usize> = Some(rug_fuzz_0);
                    let _rug_ed_tests_rug_127_rrrruuuugggg_sample = rug_fuzz_2;
                    let _rug_ed_tests_rug_127_prepare_rrrruuuugggg_sample = rug_fuzz_4;
                    let _rug_ed_tests_rug_127_prepare_rrrruuuugggg_sample = 0;
                }
            }
            unimplemented!()
        };
        let mut p4: bool = unimplemented!();
        let mut p5: bool = unimplemented!();
        let mut p6: &[u8] = unimplemented!();
        let mut p7: usize = unimplemented!();
        let mut p8: usize = unimplemented!();
        p0.exec_nfa(p1, p2, p3, p4, p5, p6, p7, p8);
        let _rug_ed_tests_rug_127_rrrruuuugggg_sample = 0;
    }
}
#[cfg(test)]
mod tests_rug_134 {
    use super::*;
    use crate::internal::Exec;
    use crate::internal::ExecBuilder;
    use crate::RegexBuilder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_134_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "pattern";
        let p0 = ExecBuilder::new(rug_fuzz_0).build().unwrap();
        p0.searcher_str();
        let _rug_ed_tests_rug_134_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_138 {
    use super::*;
    use crate::exec::{Exec, ExecBuilder};
    use crate::re_set::bytes::RegexSet;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_138_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "pattern";
        let mut p0: Exec = ExecBuilder::new(rug_fuzz_0).build().unwrap();
        Exec::into_byte_regex_set(p0);
        let _rug_ed_tests_rug_138_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_145 {
    use super::*;
    use crate::exec::ExecReadOnly;
    #[test]
    fn test_rug() {
        let mut p0: ExecReadOnly = unimplemented!("Construct the ExecReadOnly object");
        p0.choose_dfa_match_type();
    }
}
#[cfg(test)]
mod tests_rug_146 {
    use super::*;
    use crate::exec::ExecReadOnly;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_146_rrrruuuugggg_test_rug = 0;
        let mut p0: ExecReadOnly = unimplemented!();
        ExecReadOnly::should_suffix_scan(&p0);
        let _rug_ed_tests_rug_146_rrrruuuugggg_test_rug = 0;
    }
}
