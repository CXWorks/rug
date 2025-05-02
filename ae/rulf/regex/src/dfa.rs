/*!
The DFA matching engine.

A DFA provides faster matching because the engine is in exactly one state at
any point in time. In the NFA, there may be multiple active states, and
considerable CPU cycles are spent shuffling them around. In finite automata
speak, the DFA follows epsilon transitions in the regex far less than the NFA.

A DFA is a classic trade off between time and space. The NFA is slower, but
its memory requirements are typically small and predictable. The DFA is faster,
but given the right regex and the right input, the number of states in the
DFA can grow exponentially. To mitigate this space problem, we do two things:

1. We implement an *online* DFA. That is, the DFA is constructed from the NFA
   during a search. When a new state is computed, it is stored in a cache so
   that it may be reused. An important consequence of this implementation
   is that states that are never reached for a particular input are never
   computed. (This is impossible in an "offline" DFA which needs to compute
   all possible states up front.)
2. If the cache gets too big, we wipe it and continue matching.

In pathological cases, a new state can be created for every byte of input.
(e.g., The regex `(a|b)*a(a|b){20}` on a long sequence of a's and b's.)
In this case, performance regresses to slightly slower than the full NFA
simulation, in large part because the cache becomes useless. If the cache
is wiped too frequently, the DFA quits and control falls back to one of the
NFA simulations.

Because of the "lazy" nature of this DFA, the inner matching loop is
considerably more complex than one might expect out of a DFA. A number of
tricks are employed to make it fast. Tread carefully.

N.B. While this implementation is heavily commented, Russ Cox's series of
articles on regexes is strongly recommended: https://swtch.com/~rsc/regexp/
(As is the DFA implementation in RE2, which heavily influenced this
implementation.)
*/
use std::collections::HashMap;
use std::fmt;
use std::iter::repeat;
use std::mem;
use std::sync::Arc;
use exec::ProgramCache;
use prog::{Inst, Program};
use sparse::SparseSet;
/// Return true if and only if the given program can be executed by a DFA.
///
/// Generally, a DFA is always possible. A pathological case where it is not
/// possible is if the number of NFA states exceeds `u32::MAX`, in which case,
/// this function will return false.
///
/// This function will also return false if the given program has any Unicode
/// instructions (Char or Ranges) since the DFA operates on bytes only.
pub fn can_exec(insts: &Program) -> bool {
    use prog::Inst::*;
    if insts.dfa_size_limit == 0 || insts.len() > ::std::i32::MAX as usize {
        return false;
    }
    for inst in insts {
        match *inst {
            Char(_) | Ranges(_) => return false,
            EmptyLook(_) | Match(_) | Save(_) | Split(_) | Bytes(_) => {}
        }
    }
    true
}
/// A reusable cache of DFA states.
///
/// This cache is reused between multiple invocations of the same regex
/// program. (It is not shared simultaneously between threads. If there is
/// contention, then new caches are created.)
#[derive(Debug)]
pub struct Cache {
    /// Group persistent DFA related cache state together. The sparse sets
    /// listed below are used as scratch space while computing uncached states.
    inner: CacheInner,
    /// qcur and qnext are ordered sets with constant time
    /// addition/membership/clearing-whole-set and linear time iteration. They
    /// are used to manage the sets of NFA states in DFA states when computing
    /// cached DFA states. In particular, the order of the NFA states matters
    /// for leftmost-first style matching. Namely, when computing a cached
    /// state, the set of NFA states stops growing as soon as the first Match
    /// instruction is observed.
    qcur: SparseSet,
    qnext: SparseSet,
}
/// `CacheInner` is logically just a part of Cache, but groups together fields
/// that aren't passed as function parameters throughout search. (This split
/// is mostly an artifact of the borrow checker. It is happily paid.)
#[derive(Debug)]
struct CacheInner {
    /// A cache of pre-compiled DFA states, keyed by the set of NFA states
    /// and the set of empty-width flags set at the byte in the input when the
    /// state was observed.
    ///
    /// A StatePtr is effectively a `*State`, but to avoid various inconvenient
    /// things, we just pass indexes around manually. The performance impact of
    /// this is probably an instruction or two in the inner loop. However, on
    /// 64 bit, each StatePtr is half the size of a *State.
    compiled: StateMap,
    /// The transition table.
    ///
    /// The transition table is laid out in row-major order, where states are
    /// rows and the transitions for each state are columns. At a high level,
    /// given state `s` and byte `b`, the next state can be found at index
    /// `s * 256 + b`.
    ///
    /// This is, of course, a lie. A StatePtr is actually a pointer to the
    /// *start* of a row in this table. When indexing in the DFA's inner loop,
    /// this removes the need to multiply the StatePtr by the stride. Yes, it
    /// matters. This reduces the number of states we can store, but: the
    /// stride is rarely 256 since we define transitions in terms of
    /// *equivalence classes* of bytes. Each class corresponds to a set of
    /// bytes that never discriminate a distinct path through the DFA from each
    /// other.
    trans: Transitions,
    /// A set of cached start states, which are limited to the number of
    /// permutations of flags set just before the initial byte of input. (The
    /// index into this vec is a `EmptyFlags`.)
    ///
    /// N.B. A start state can be "dead" (i.e., no possible match), so we
    /// represent it with a StatePtr.
    start_states: Vec<StatePtr>,
    /// Stack scratch space used to follow epsilon transitions in the NFA.
    /// (This permits us to avoid recursion.)
    ///
    /// The maximum stack size is the number of NFA states.
    stack: Vec<InstPtr>,
    /// The total number of times this cache has been flushed by the DFA
    /// because of space constraints.
    flush_count: u64,
    /// The total heap size of the DFA's cache. We use this to determine when
    /// we should flush the cache.
    size: usize,
    /// Scratch space used when building instruction pointer lists for new
    /// states. This helps amortize allocation.
    insts_scratch_space: Vec<u8>,
}
/// The transition table.
///
/// It is laid out in row-major order, with states as rows and byte class
/// transitions as columns.
///
/// The transition table is responsible for producing valid `StatePtrs`. A
/// `StatePtr` points to the start of a particular row in this table. When
/// indexing to find the next state this allows us to avoid a multiplication
/// when computing an index into the table.
#[derive(Clone)]
struct Transitions {
    /// The table.
    table: Vec<StatePtr>,
    /// The stride.
    num_byte_classes: usize,
}
/// Fsm encapsulates the actual execution of the DFA.
#[derive(Debug)]
pub struct Fsm<'a> {
    /// prog contains the NFA instruction opcodes. DFA execution uses either
    /// the `dfa` instructions or the `dfa_reverse` instructions from
    /// `exec::ExecReadOnly`. (It never uses `ExecReadOnly.nfa`, which may have
    /// Unicode opcodes that cannot be executed by the DFA.)
    prog: &'a Program,
    /// The start state. We record it here because the pointer may change
    /// when the cache is wiped.
    start: StatePtr,
    /// The current position in the input.
    at: usize,
    /// Should we quit after seeing the first match? e.g., When the caller
    /// uses `is_match` or `shortest_match`.
    quit_after_match: bool,
    /// The last state that matched.
    ///
    /// When no match has occurred, this is set to STATE_UNKNOWN.
    ///
    /// This is only useful when matching regex sets. The last match state
    /// is useful because it contains all of the match instructions seen,
    /// thereby allowing us to enumerate which regexes in the set matched.
    last_match_si: StatePtr,
    /// The input position of the last cache flush. We use this to determine
    /// if we're thrashing in the cache too often. If so, the DFA quits so
    /// that we can fall back to the NFA algorithm.
    last_cache_flush: usize,
    /// All cached DFA information that is persisted between searches.
    cache: &'a mut CacheInner,
}
/// The result of running the DFA.
///
/// Generally, the result is either a match or not a match, but sometimes the
/// DFA runs too slowly because the cache size is too small. In that case, it
/// gives up with the intent of falling back to the NFA algorithm.
///
/// The DFA can also give up if it runs out of room to create new states, or if
/// it sees non-ASCII bytes in the presence of a Unicode word boundary.
#[derive(Clone, Debug)]
pub enum Result<T> {
    Match(T),
    NoMatch(usize),
    Quit,
}
impl<T> Result<T> {
    /// Returns true if this result corresponds to a match.
    pub fn is_match(&self) -> bool {
        match *self {
            Result::Match(_) => true,
            Result::NoMatch(_) | Result::Quit => false,
        }
    }
    /// Maps the given function onto T and returns the result.
    ///
    /// If this isn't a match, then this is a no-op.
    #[cfg(feature = "perf-literal")]
    pub fn map<U, F: FnMut(T) -> U>(self, mut f: F) -> Result<U> {
        match self {
            Result::Match(t) => Result::Match(f(t)),
            Result::NoMatch(x) => Result::NoMatch(x),
            Result::Quit => Result::Quit,
        }
    }
    /// Sets the non-match position.
    ///
    /// If this isn't a non-match, then this is a no-op.
    fn set_non_match(self, at: usize) -> Result<T> {
        match self {
            Result::NoMatch(_) => Result::NoMatch(at),
            r => r,
        }
    }
}
/// `State` is a DFA state. It contains an ordered set of NFA states (not
/// necessarily complete) and a smattering of flags.
///
/// The flags are packed into the first byte of data.
///
/// States don't carry their transitions. Instead, transitions are stored in
/// a single row-major table.
///
/// Delta encoding is used to store the instruction pointers.
/// The first instruction pointer is stored directly starting
/// at data[1], and each following pointer is stored as an offset
/// to the previous one. If a delta is in the range -127..127,
/// it is packed into a single byte; Otherwise the byte 128 (-128 as an i8)
/// is coded as a flag, followed by 4 bytes encoding the delta.
#[derive(Clone, Eq, Hash, PartialEq)]
struct State {
    data: Arc<[u8]>,
}
/// `InstPtr` is a 32 bit pointer into a sequence of opcodes (i.e., it indexes
/// an NFA state).
///
/// Throughout this library, this is usually set to `usize`, but we force a
/// `u32` here for the DFA to save on space.
type InstPtr = u32;
/// Adds ip to data using delta encoding with respect to prev.
///
/// After completion, `data` will contain `ip` and `prev` will be set to `ip`.
fn push_inst_ptr(data: &mut Vec<u8>, prev: &mut InstPtr, ip: InstPtr) {
    let delta = (ip as i32) - (*prev as i32);
    write_vari32(data, delta);
    *prev = ip;
}
struct InstPtrs<'a> {
    base: usize,
    data: &'a [u8],
}
impl<'a> Iterator for InstPtrs<'a> {
    type Item = usize;
    fn next(&mut self) -> Option<usize> {
        if self.data.is_empty() {
            return None;
        }
        let (delta, nread) = read_vari32(self.data);
        let base = self.base as i32 + delta;
        debug_assert!(base >= 0);
        debug_assert!(nread > 0);
        self.data = &self.data[nread..];
        self.base = base as usize;
        Some(self.base)
    }
}
impl State {
    fn flags(&self) -> StateFlags {
        StateFlags(self.data[0])
    }
    fn inst_ptrs(&self) -> InstPtrs {
        InstPtrs {
            base: 0,
            data: &self.data[1..],
        }
    }
}
/// `StatePtr` is a 32 bit pointer to the start of a row in the transition
/// table.
///
/// It has many special values. There are two types of special values:
/// sentinels and flags.
///
/// Sentinels corresponds to special states that carry some kind of
/// significance. There are three such states: unknown, dead and quit states.
///
/// Unknown states are states that haven't been computed yet. They indicate
/// that a transition should be filled in that points to either an existing
/// cached state or a new state altogether. In general, an unknown state means
/// "follow the NFA's epsilon transitions."
///
/// Dead states are states that can never lead to a match, no matter what
/// subsequent input is observed. This means that the DFA should quit
/// immediately and return the longest match it has found thus far.
///
/// Quit states are states that imply the DFA is not capable of matching the
/// regex correctly. Currently, this is only used when a Unicode word boundary
/// exists in the regex *and* a non-ASCII byte is observed.
///
/// The other type of state pointer is a state pointer with special flag bits.
/// There are two flags: a start flag and a match flag. The lower bits of both
/// kinds always contain a "valid" `StatePtr` (indicated by the `STATE_MAX`
/// mask).
///
/// The start flag means that the state is a start state, and therefore may be
/// subject to special prefix scanning optimizations.
///
/// The match flag means that the state is a match state, and therefore the
/// current position in the input (while searching) should be recorded.
///
/// The above exists mostly in the service of making the inner loop fast.
/// In particular, the inner *inner* loop looks something like this:
///
/// ```ignore
/// while state <= STATE_MAX and i < len(text):
///     state = state.next[i]
/// ```
///
/// This is nice because it lets us execute a lazy DFA as if it were an
/// entirely offline DFA (i.e., with very few instructions). The loop will
/// quit only when we need to examine a case that needs special attention.
type StatePtr = u32;
/// An unknown state means that the state has not been computed yet, and that
/// the only way to progress is to compute it.
const STATE_UNKNOWN: StatePtr = 1 << 31;
/// A dead state means that the state has been computed and it is known that
/// once it is entered, no future match can ever occur.
const STATE_DEAD: StatePtr = STATE_UNKNOWN + 1;
/// A quit state means that the DFA came across some input that it doesn't
/// know how to process correctly. The DFA should quit and another matching
/// engine should be run in its place.
const STATE_QUIT: StatePtr = STATE_DEAD + 1;
/// A start state is a state that the DFA can start in.
///
/// Note that start states have their lower bits set to a state pointer.
const STATE_START: StatePtr = 1 << 30;
/// A match state means that the regex has successfully matched.
///
/// Note that match states have their lower bits set to a state pointer.
const STATE_MATCH: StatePtr = 1 << 29;
/// The maximum state pointer. This is useful to mask out the "valid" state
/// pointer from a state with the "start" or "match" bits set.
///
/// It doesn't make sense to use this with unknown, dead or quit state
/// pointers, since those pointers are sentinels and never have their lower
/// bits set to anything meaningful.
const STATE_MAX: StatePtr = STATE_MATCH - 1;
/// Byte is a u8 in spirit, but a u16 in practice so that we can represent the
/// special EOF sentinel value.
#[derive(Copy, Clone, Debug)]
struct Byte(u16);
/// A set of flags for zero-width assertions.
#[derive(Clone, Copy, Eq, Debug, Default, Hash, PartialEq)]
struct EmptyFlags {
    start: bool,
    end: bool,
    start_line: bool,
    end_line: bool,
    word_boundary: bool,
    not_word_boundary: bool,
}
/// A set of flags describing various configurations of a DFA state. This is
/// represented by a `u8` so that it is compact.
#[derive(Clone, Copy, Eq, Default, Hash, PartialEq)]
struct StateFlags(u8);
impl Cache {
    /// Create new empty cache for the DFA engine.
    pub fn new(prog: &Program) -> Self {
        let num_byte_classes = (prog.byte_classes[255] as usize + 1) + 1;
        let starts = vec![STATE_UNKNOWN; 256];
        let mut cache = Cache {
            inner: CacheInner {
                compiled: StateMap::new(num_byte_classes),
                trans: Transitions::new(num_byte_classes),
                start_states: starts,
                stack: vec![],
                flush_count: 0,
                size: 0,
                insts_scratch_space: vec![],
            },
            qcur: SparseSet::new(prog.insts.len()),
            qnext: SparseSet::new(prog.insts.len()),
        };
        cache.inner.reset_size();
        cache
    }
}
impl CacheInner {
    /// Resets the cache size to account for fixed costs, such as the program
    /// and stack sizes.
    fn reset_size(&mut self) {
        self
            .size = (self.start_states.len() * mem::size_of::<StatePtr>())
            + (self.stack.len() * mem::size_of::<InstPtr>());
    }
}
impl<'a> Fsm<'a> {
    #[cfg_attr(feature = "perf-inline", inline(always))]
    pub fn forward(
        prog: &'a Program,
        cache: &ProgramCache,
        quit_after_match: bool,
        text: &[u8],
        at: usize,
    ) -> Result<usize> {
        let mut cache = cache.borrow_mut();
        let cache = &mut cache.dfa;
        let mut dfa = Fsm {
            prog: prog,
            start: 0,
            at: at,
            quit_after_match: quit_after_match,
            last_match_si: STATE_UNKNOWN,
            last_cache_flush: at,
            cache: &mut cache.inner,
        };
        let (empty_flags, state_flags) = dfa.start_flags(text, at);
        dfa
            .start = match dfa.start_state(&mut cache.qcur, empty_flags, state_flags) {
            None => return Result::Quit,
            Some(STATE_DEAD) => return Result::NoMatch(at),
            Some(si) => si,
        };
        debug_assert!(dfa.start != STATE_UNKNOWN);
        dfa.exec_at(&mut cache.qcur, &mut cache.qnext, text)
    }
    #[cfg_attr(feature = "perf-inline", inline(always))]
    pub fn reverse(
        prog: &'a Program,
        cache: &ProgramCache,
        quit_after_match: bool,
        text: &[u8],
        at: usize,
    ) -> Result<usize> {
        let mut cache = cache.borrow_mut();
        let cache = &mut cache.dfa_reverse;
        let mut dfa = Fsm {
            prog: prog,
            start: 0,
            at: at,
            quit_after_match: quit_after_match,
            last_match_si: STATE_UNKNOWN,
            last_cache_flush: at,
            cache: &mut cache.inner,
        };
        let (empty_flags, state_flags) = dfa.start_flags_reverse(text, at);
        dfa
            .start = match dfa.start_state(&mut cache.qcur, empty_flags, state_flags) {
            None => return Result::Quit,
            Some(STATE_DEAD) => return Result::NoMatch(at),
            Some(si) => si,
        };
        debug_assert!(dfa.start != STATE_UNKNOWN);
        dfa.exec_at_reverse(&mut cache.qcur, &mut cache.qnext, text)
    }
    #[cfg_attr(feature = "perf-inline", inline(always))]
    pub fn forward_many(
        prog: &'a Program,
        cache: &ProgramCache,
        matches: &mut [bool],
        text: &[u8],
        at: usize,
    ) -> Result<usize> {
        debug_assert!(matches.len() == prog.matches.len());
        let mut cache = cache.borrow_mut();
        let cache = &mut cache.dfa;
        let mut dfa = Fsm {
            prog: prog,
            start: 0,
            at: at,
            quit_after_match: false,
            last_match_si: STATE_UNKNOWN,
            last_cache_flush: at,
            cache: &mut cache.inner,
        };
        let (empty_flags, state_flags) = dfa.start_flags(text, at);
        dfa
            .start = match dfa.start_state(&mut cache.qcur, empty_flags, state_flags) {
            None => return Result::Quit,
            Some(STATE_DEAD) => return Result::NoMatch(at),
            Some(si) => si,
        };
        debug_assert!(dfa.start != STATE_UNKNOWN);
        let result = dfa.exec_at(&mut cache.qcur, &mut cache.qnext, text);
        if result.is_match() {
            if matches.len() == 1 {
                matches[0] = true;
            } else {
                debug_assert!(dfa.last_match_si != STATE_UNKNOWN);
                debug_assert!(dfa.last_match_si != STATE_DEAD);
                for ip in dfa.state(dfa.last_match_si).inst_ptrs() {
                    if let Inst::Match(slot) = dfa.prog[ip] {
                        matches[slot] = true;
                    }
                }
            }
        }
        result
    }
    /// Executes the DFA on a forward NFA.
    ///
    /// {qcur,qnext} are scratch ordered sets which may be non-empty.
    #[cfg_attr(feature = "perf-inline", inline(always))]
    fn exec_at(
        &mut self,
        qcur: &mut SparseSet,
        qnext: &mut SparseSet,
        text: &[u8],
    ) -> Result<usize> {
        debug_assert!(! self.prog.is_reverse);
        let mut result = Result::NoMatch(self.at);
        let (mut prev_si, mut next_si) = (self.start, self.start);
        let mut at = self.at;
        while at < text.len() {
            while next_si <= STATE_MAX && at < text.len() {
                prev_si = unsafe { self.next_si(next_si, text, at) };
                at += 1;
                if prev_si > STATE_MAX || at + 2 >= text.len() {
                    mem::swap(&mut prev_si, &mut next_si);
                    break;
                }
                next_si = unsafe { self.next_si(prev_si, text, at) };
                at += 1;
                if next_si > STATE_MAX {
                    break;
                }
                prev_si = unsafe { self.next_si(next_si, text, at) };
                at += 1;
                if prev_si > STATE_MAX {
                    mem::swap(&mut prev_si, &mut next_si);
                    break;
                }
                next_si = unsafe { self.next_si(prev_si, text, at) };
                at += 1;
            }
            if next_si & STATE_MATCH > 0 {
                next_si &= !STATE_MATCH;
                result = Result::Match(at - 1);
                if self.quit_after_match {
                    return result;
                }
                self.last_match_si = next_si;
                prev_si = next_si;
                if self.prog.matches.len() > 1 {
                    let state = self.state(next_si);
                    let just_matches = state
                        .inst_ptrs()
                        .all(|ip| self.prog[ip].is_match());
                    if just_matches {
                        return result;
                    }
                }
                let cur = at;
                while (next_si & !STATE_MATCH) == prev_si && at + 2 < text.len() {
                    next_si = unsafe { self.next_si(next_si & !STATE_MATCH, text, at) };
                    at += 1;
                }
                if at > cur {
                    result = Result::Match(at - 2);
                }
            } else if next_si & STATE_START > 0 {
                debug_assert!(self.has_prefix());
                next_si &= !STATE_START;
                prev_si = next_si;
                at = match self.prefix_at(text, at) {
                    None => return Result::NoMatch(text.len()),
                    Some(i) => i,
                };
            } else if next_si >= STATE_UNKNOWN {
                if next_si == STATE_QUIT {
                    return Result::Quit;
                }
                let byte = Byte::byte(text[at - 1]);
                prev_si &= STATE_MAX;
                self.at = at;
                next_si = match self.next_state(qcur, qnext, prev_si, byte) {
                    None => return Result::Quit,
                    Some(STATE_DEAD) => return result.set_non_match(at),
                    Some(si) => si,
                };
                debug_assert!(next_si != STATE_UNKNOWN);
                if next_si & STATE_MATCH > 0 {
                    next_si &= !STATE_MATCH;
                    result = Result::Match(at - 1);
                    if self.quit_after_match {
                        return result;
                    }
                    self.last_match_si = next_si;
                }
                prev_si = next_si;
            } else {
                prev_si = next_si;
            }
        }
        prev_si &= STATE_MAX;
        prev_si = match self.next_state(qcur, qnext, prev_si, Byte::eof()) {
            None => return Result::Quit,
            Some(STATE_DEAD) => return result.set_non_match(text.len()),
            Some(si) => si & !STATE_START,
        };
        debug_assert!(prev_si != STATE_UNKNOWN);
        if prev_si & STATE_MATCH > 0 {
            prev_si &= !STATE_MATCH;
            self.last_match_si = prev_si;
            result = Result::Match(text.len());
        }
        result
    }
    /// Executes the DFA on a reverse NFA.
    #[cfg_attr(feature = "perf-inline", inline(always))]
    fn exec_at_reverse(
        &mut self,
        qcur: &mut SparseSet,
        qnext: &mut SparseSet,
        text: &[u8],
    ) -> Result<usize> {
        debug_assert!(self.prog.is_reverse);
        let mut result = Result::NoMatch(self.at);
        let (mut prev_si, mut next_si) = (self.start, self.start);
        let mut at = self.at;
        while at > 0 {
            while next_si <= STATE_MAX && at > 0 {
                at -= 1;
                prev_si = unsafe { self.next_si(next_si, text, at) };
                if prev_si > STATE_MAX || at <= 4 {
                    mem::swap(&mut prev_si, &mut next_si);
                    break;
                }
                at -= 1;
                next_si = unsafe { self.next_si(prev_si, text, at) };
                if next_si > STATE_MAX {
                    break;
                }
                at -= 1;
                prev_si = unsafe { self.next_si(next_si, text, at) };
                if prev_si > STATE_MAX {
                    mem::swap(&mut prev_si, &mut next_si);
                    break;
                }
                at -= 1;
                next_si = unsafe { self.next_si(prev_si, text, at) };
            }
            if next_si & STATE_MATCH > 0 {
                next_si &= !STATE_MATCH;
                result = Result::Match(at + 1);
                if self.quit_after_match {
                    return result;
                }
                self.last_match_si = next_si;
                prev_si = next_si;
                let cur = at;
                while (next_si & !STATE_MATCH) == prev_si && at >= 2 {
                    at -= 1;
                    next_si = unsafe { self.next_si(next_si & !STATE_MATCH, text, at) };
                }
                if at < cur {
                    result = Result::Match(at + 2);
                }
            } else if next_si >= STATE_UNKNOWN {
                if next_si == STATE_QUIT {
                    return Result::Quit;
                }
                let byte = Byte::byte(text[at]);
                prev_si &= STATE_MAX;
                self.at = at;
                next_si = match self.next_state(qcur, qnext, prev_si, byte) {
                    None => return Result::Quit,
                    Some(STATE_DEAD) => return result.set_non_match(at),
                    Some(si) => si,
                };
                debug_assert!(next_si != STATE_UNKNOWN);
                if next_si & STATE_MATCH > 0 {
                    next_si &= !STATE_MATCH;
                    result = Result::Match(at + 1);
                    if self.quit_after_match {
                        return result;
                    }
                    self.last_match_si = next_si;
                }
                prev_si = next_si;
            } else {
                prev_si = next_si;
            }
        }
        prev_si = match self.next_state(qcur, qnext, prev_si, Byte::eof()) {
            None => return Result::Quit,
            Some(STATE_DEAD) => return result.set_non_match(0),
            Some(si) => si,
        };
        debug_assert!(prev_si != STATE_UNKNOWN);
        if prev_si & STATE_MATCH > 0 {
            prev_si &= !STATE_MATCH;
            self.last_match_si = prev_si;
            result = Result::Match(0);
        }
        result
    }
    /// next_si transitions to the next state, where the transition input
    /// corresponds to text[i].
    ///
    /// This elides bounds checks, and is therefore unsafe.
    #[cfg_attr(feature = "perf-inline", inline(always))]
    unsafe fn next_si(&self, si: StatePtr, text: &[u8], i: usize) -> StatePtr {
        debug_assert!(i < text.len());
        let b = *text.get_unchecked(i);
        debug_assert!((b as usize) < self.prog.byte_classes.len());
        let cls = *self.prog.byte_classes.get_unchecked(b as usize);
        self.cache.trans.next_unchecked(si, cls as usize)
    }
    /// Computes the next state given the current state and the current input
    /// byte (which may be EOF).
    ///
    /// If STATE_DEAD is returned, then there is no valid state transition.
    /// This implies that no permutation of future input can lead to a match
    /// state.
    ///
    /// STATE_UNKNOWN can never be returned.
    fn exec_byte(
        &mut self,
        qcur: &mut SparseSet,
        qnext: &mut SparseSet,
        mut si: StatePtr,
        b: Byte,
    ) -> Option<StatePtr> {
        use prog::Inst::*;
        qcur.clear();
        for ip in self.state(si).inst_ptrs() {
            qcur.insert(ip);
        }
        let is_word_last = self.state(si).flags().is_word();
        let is_word = b.is_ascii_word();
        if self.state(si).flags().has_empty() {
            let mut flags = EmptyFlags::default();
            if b.is_eof() {
                flags.end = true;
                flags.end_line = true;
            } else if b.as_byte().map_or(false, |b| b == b'\n') {
                flags.end_line = true;
            }
            if is_word_last == is_word {
                flags.not_word_boundary = true;
            } else {
                flags.word_boundary = true;
            }
            qnext.clear();
            for &ip in &*qcur {
                self.follow_epsilons(usize_to_u32(ip), qnext, flags);
            }
            mem::swap(qcur, qnext);
        }
        let mut empty_flags = EmptyFlags::default();
        let mut state_flags = StateFlags::default();
        empty_flags.start_line = b.as_byte().map_or(false, |b| b == b'\n');
        if b.is_ascii_word() {
            state_flags.set_word();
        }
        qnext.clear();
        for &ip in &*qcur {
            match self.prog[ip as usize] {
                Char(_) | Ranges(_) => unreachable!(),
                Save(_) | Split(_) | EmptyLook(_) => {}
                Match(_) => {
                    state_flags.set_match();
                    if !self.continue_past_first_match() {
                        break;
                    } else if self.prog.matches.len() > 1 && !qnext.contains(ip as usize)
                    {
                        qnext.insert(ip);
                    }
                }
                Bytes(ref inst) => {
                    if b.as_byte().map_or(false, |b| inst.matches(b)) {
                        self.follow_epsilons(inst.goto as InstPtr, qnext, empty_flags);
                    }
                }
            }
        }
        let cache = if b.is_eof() && self.prog.matches.len() > 1 {
            mem::swap(qcur, qnext);
            false
        } else {
            true
        };
        let mut next = match self.cached_state(qnext, state_flags, Some(&mut si)) {
            None => return None,
            Some(next) => next,
        };
        if (self.start & !STATE_START) == next {
            debug_assert!(! self.state(next).flags().is_match());
            next = self.start_ptr(next);
        }
        if next <= STATE_MAX && self.state(next).flags().is_match() {
            next |= STATE_MATCH;
        }
        debug_assert!(next != STATE_UNKNOWN);
        if cache {
            let cls = self.byte_class(b);
            self.cache.trans.set_next(si, cls, next);
        }
        Some(next)
    }
    /// Follows the epsilon transitions starting at (and including) `ip`. The
    /// resulting states are inserted into the ordered set `q`.
    ///
    /// Conditional epsilon transitions (i.e., empty width assertions) are only
    /// followed if they are satisfied by the given flags, which should
    /// represent the flags set at the current location in the input.
    ///
    /// If the current location corresponds to the empty string, then only the
    /// end line and/or end text flags may be set. If the current location
    /// corresponds to a real byte in the input, then only the start line
    /// and/or start text flags may be set.
    ///
    /// As an exception to the above, when finding the initial state, any of
    /// the above flags may be set:
    ///
    /// If matching starts at the beginning of the input, then start text and
    /// start line should be set. If the input is empty, then end text and end
    /// line should also be set.
    ///
    /// If matching starts after the beginning of the input, then only start
    /// line should be set if the preceding byte is `\n`. End line should never
    /// be set in this case. (Even if the following byte is a `\n`, it will
    /// be handled in a subsequent DFA state.)
    fn follow_epsilons(&mut self, ip: InstPtr, q: &mut SparseSet, flags: EmptyFlags) {
        use prog::EmptyLook::*;
        use prog::Inst::*;
        self.cache.stack.push(ip);
        while let Some(mut ip) = self.cache.stack.pop() {
            loop {
                if q.contains(ip as usize) {
                    break;
                }
                q.insert(ip as usize);
                match self.prog[ip as usize] {
                    Char(_) | Ranges(_) => unreachable!(),
                    Match(_) | Bytes(_) => {
                        break;
                    }
                    EmptyLook(ref inst) => {
                        match inst.look {
                            StartLine if flags.start_line => {
                                ip = inst.goto as InstPtr;
                            }
                            EndLine if flags.end_line => {
                                ip = inst.goto as InstPtr;
                            }
                            StartText if flags.start => {
                                ip = inst.goto as InstPtr;
                            }
                            EndText if flags.end => {
                                ip = inst.goto as InstPtr;
                            }
                            WordBoundaryAscii if flags.word_boundary => {
                                ip = inst.goto as InstPtr;
                            }
                            NotWordBoundaryAscii if flags.not_word_boundary => {
                                ip = inst.goto as InstPtr;
                            }
                            WordBoundary if flags.word_boundary => {
                                ip = inst.goto as InstPtr;
                            }
                            NotWordBoundary if flags.not_word_boundary => {
                                ip = inst.goto as InstPtr;
                            }
                            StartLine
                            | EndLine
                            | StartText
                            | EndText
                            | WordBoundaryAscii
                            | NotWordBoundaryAscii
                            | WordBoundary
                            | NotWordBoundary => {
                                break;
                            }
                        }
                    }
                    Save(ref inst) => {
                        ip = inst.goto as InstPtr;
                    }
                    Split(ref inst) => {
                        self.cache.stack.push(inst.goto2 as InstPtr);
                        ip = inst.goto1 as InstPtr;
                    }
                }
            }
        }
    }
    /// Find a previously computed state matching the given set of instructions
    /// and is_match bool.
    ///
    /// The given set of instructions should represent a single state in the
    /// NFA along with all states reachable without consuming any input.
    ///
    /// The is_match bool should be true if and only if the preceding DFA state
    /// contains an NFA matching state. The cached state produced here will
    /// then signify a match. (This enables us to delay a match by one byte,
    /// in order to account for the EOF sentinel byte.)
    ///
    /// If the cache is full, then it is wiped before caching a new state.
    ///
    /// The current state should be specified if it exists, since it will need
    /// to be preserved if the cache clears itself. (Start states are
    /// always saved, so they should not be passed here.) It takes a mutable
    /// pointer to the index because if the cache is cleared, the state's
    /// location may change.
    fn cached_state(
        &mut self,
        q: &SparseSet,
        mut state_flags: StateFlags,
        current_state: Option<&mut StatePtr>,
    ) -> Option<StatePtr> {
        let key = match self.cached_state_key(q, &mut state_flags) {
            None => return Some(STATE_DEAD),
            Some(v) => v,
        };
        if let Some(si) = self.cache.compiled.get_ptr(&key) {
            return Some(si);
        }
        if self.approximate_size() > self.prog.dfa_size_limit
            && !self.clear_cache_and_save(current_state)
        {
            return None;
        }
        self.add_state(key)
    }
    /// Produces a key suitable for describing a state in the DFA cache.
    ///
    /// The key invariant here is that equivalent keys are produced for any two
    /// sets of ordered NFA states (and toggling of whether the previous NFA
    /// states contain a match state) that do not discriminate a match for any
    /// input.
    ///
    /// Specifically, q should be an ordered set of NFA states and is_match
    /// should be true if and only if the previous NFA states contained a match
    /// state.
    fn cached_state_key(
        &mut self,
        q: &SparseSet,
        state_flags: &mut StateFlags,
    ) -> Option<State> {
        use prog::Inst::*;
        let mut insts = mem::replace(&mut self.cache.insts_scratch_space, vec![]);
        insts.clear();
        insts.push(0);
        let mut prev = 0;
        for &ip in q {
            let ip = usize_to_u32(ip);
            match self.prog[ip as usize] {
                Char(_) | Ranges(_) => unreachable!(),
                Save(_) | Split(_) => {}
                Bytes(_) => push_inst_ptr(&mut insts, &mut prev, ip),
                EmptyLook(_) => {
                    state_flags.set_empty();
                    push_inst_ptr(&mut insts, &mut prev, ip)
                }
                Match(_) => {
                    push_inst_ptr(&mut insts, &mut prev, ip);
                    if !self.continue_past_first_match() {
                        break;
                    }
                }
            }
        }
        let opt_state = if insts.len() == 1 && !state_flags.is_match() {
            None
        } else {
            let StateFlags(f) = *state_flags;
            insts[0] = f;
            Some(State { data: Arc::from(&*insts) })
        };
        self.cache.insts_scratch_space = insts;
        opt_state
    }
    /// Clears the cache, but saves and restores current_state if it is not
    /// none.
    ///
    /// The current state must be provided here in case its location in the
    /// cache changes.
    ///
    /// This returns false if the cache is not cleared and the DFA should
    /// give up.
    fn clear_cache_and_save(&mut self, current_state: Option<&mut StatePtr>) -> bool {
        if self.cache.compiled.is_empty() {
            return true;
        }
        match current_state {
            None => self.clear_cache(),
            Some(si) => {
                let cur = self.state(*si).clone();
                if !self.clear_cache() {
                    return false;
                }
                *si = self.restore_state(cur).unwrap();
                true
            }
        }
    }
    /// Wipes the state cache, but saves and restores the current start state.
    ///
    /// This returns false if the cache is not cleared and the DFA should
    /// give up.
    fn clear_cache(&mut self) -> bool {
        let nstates = self.cache.compiled.len();
        if self.cache.flush_count >= 3 && self.at >= self.last_cache_flush
            && (self.at - self.last_cache_flush) <= 10 * nstates
        {
            return false;
        }
        self.last_cache_flush = self.at;
        self.cache.flush_count += 1;
        let start = self.state(self.start & !STATE_START).clone();
        let last_match = if self.last_match_si <= STATE_MAX {
            Some(self.state(self.last_match_si).clone())
        } else {
            None
        };
        self.cache.reset_size();
        self.cache.trans.clear();
        self.cache.compiled.clear();
        for s in &mut self.cache.start_states {
            *s = STATE_UNKNOWN;
        }
        let start_ptr = self.restore_state(start).unwrap();
        self.start = self.start_ptr(start_ptr);
        if let Some(last_match) = last_match {
            self.last_match_si = self.restore_state(last_match).unwrap();
        }
        true
    }
    /// Restores the given state back into the cache, and returns a pointer
    /// to it.
    fn restore_state(&mut self, state: State) -> Option<StatePtr> {
        if let Some(si) = self.cache.compiled.get_ptr(&state) {
            return Some(si);
        }
        self.add_state(state)
    }
    /// Returns the next state given the current state si and current byte
    /// b. {qcur,qnext} are used as scratch space for storing ordered NFA
    /// states.
    ///
    /// This tries to fetch the next state from the cache, but if that fails,
    /// it computes the next state, caches it and returns a pointer to it.
    ///
    /// The pointer can be to a real state, or it can be STATE_DEAD.
    /// STATE_UNKNOWN cannot be returned.
    ///
    /// None is returned if a new state could not be allocated (i.e., the DFA
    /// ran out of space and thinks it's running too slowly).
    fn next_state(
        &mut self,
        qcur: &mut SparseSet,
        qnext: &mut SparseSet,
        si: StatePtr,
        b: Byte,
    ) -> Option<StatePtr> {
        if si == STATE_DEAD {
            return Some(STATE_DEAD);
        }
        match self.cache.trans.next(si, self.byte_class(b)) {
            STATE_UNKNOWN => self.exec_byte(qcur, qnext, si, b),
            STATE_QUIT => None,
            STATE_DEAD => Some(STATE_DEAD),
            nsi => Some(nsi),
        }
    }
    /// Computes and returns the start state, where searching begins at
    /// position `at` in `text`. If the state has already been computed,
    /// then it is pulled from the cache. If the state hasn't been cached,
    /// then it is computed, cached and a pointer to it is returned.
    ///
    /// This may return STATE_DEAD but never STATE_UNKNOWN.
    #[cfg_attr(feature = "perf-inline", inline(always))]
    fn start_state(
        &mut self,
        q: &mut SparseSet,
        empty_flags: EmptyFlags,
        state_flags: StateFlags,
    ) -> Option<StatePtr> {
        let flagi = {
            (((empty_flags.start as u8) << 0) | ((empty_flags.end as u8) << 1)
                | ((empty_flags.start_line as u8) << 2)
                | ((empty_flags.end_line as u8) << 3)
                | ((empty_flags.word_boundary as u8) << 4)
                | ((empty_flags.not_word_boundary as u8) << 5)
                | ((state_flags.is_word() as u8) << 6)) as usize
        };
        match self.cache.start_states[flagi] {
            STATE_UNKNOWN => {}
            STATE_DEAD => return Some(STATE_DEAD),
            si => return Some(si),
        }
        q.clear();
        let start = usize_to_u32(self.prog.start);
        self.follow_epsilons(start, q, empty_flags);
        let sp = match self.cached_state(q, state_flags, None) {
            None => return None,
            Some(sp) => self.start_ptr(sp),
        };
        self.cache.start_states[flagi] = sp;
        Some(sp)
    }
    /// Computes the set of starting flags for the given position in text.
    ///
    /// This should only be used when executing the DFA forwards over the
    /// input.
    fn start_flags(&self, text: &[u8], at: usize) -> (EmptyFlags, StateFlags) {
        let mut empty_flags = EmptyFlags::default();
        let mut state_flags = StateFlags::default();
        empty_flags.start = at == 0;
        empty_flags.end = text.is_empty();
        empty_flags.start_line = at == 0 || text[at - 1] == b'\n';
        empty_flags.end_line = text.is_empty();
        let is_word_last = at > 0 && Byte::byte(text[at - 1]).is_ascii_word();
        let is_word = at < text.len() && Byte::byte(text[at]).is_ascii_word();
        if is_word_last {
            state_flags.set_word();
        }
        if is_word == is_word_last {
            empty_flags.not_word_boundary = true;
        } else {
            empty_flags.word_boundary = true;
        }
        (empty_flags, state_flags)
    }
    /// Computes the set of starting flags for the given position in text.
    ///
    /// This should only be used when executing the DFA in reverse over the
    /// input.
    fn start_flags_reverse(&self, text: &[u8], at: usize) -> (EmptyFlags, StateFlags) {
        let mut empty_flags = EmptyFlags::default();
        let mut state_flags = StateFlags::default();
        empty_flags.start = at == text.len();
        empty_flags.end = text.is_empty();
        empty_flags.start_line = at == text.len() || text[at] == b'\n';
        empty_flags.end_line = text.is_empty();
        let is_word_last = at < text.len() && Byte::byte(text[at]).is_ascii_word();
        let is_word = at > 0 && Byte::byte(text[at - 1]).is_ascii_word();
        if is_word_last {
            state_flags.set_word();
        }
        if is_word == is_word_last {
            empty_flags.not_word_boundary = true;
        } else {
            empty_flags.word_boundary = true;
        }
        (empty_flags, state_flags)
    }
    /// Returns a reference to a State given a pointer to it.
    fn state(&self, si: StatePtr) -> &State {
        self.cache.compiled.get_state(si).unwrap()
    }
    /// Adds the given state to the DFA.
    ///
    /// This allocates room for transitions out of this state in
    /// self.cache.trans. The transitions can be set with the returned
    /// StatePtr.
    ///
    /// If None is returned, then the state limit was reached and the DFA
    /// should quit.
    fn add_state(&mut self, state: State) -> Option<StatePtr> {
        let si = match self.cache.trans.add() {
            None => return None,
            Some(si) => si,
        };
        if self.prog.has_unicode_word_boundary {
            for b in 128..256 {
                let cls = self.byte_class(Byte::byte(b as u8));
                self.cache.trans.set_next(si, cls, STATE_QUIT);
            }
        }
        self.cache.size
            += self.cache.trans.state_heap_size() + state.data.len()
                + (2 * mem::size_of::<State>()) + mem::size_of::<StatePtr>();
        self.cache.compiled.insert(state, si);
        debug_assert!(self.cache.compiled.len() == self.cache.trans.num_states());
        Some(si)
    }
    /// Quickly finds the next occurrence of any literal prefixes in the regex.
    /// If there are no literal prefixes, then the current position is
    /// returned. If there are literal prefixes and one could not be found,
    /// then None is returned.
    ///
    /// This should only be called when the DFA is in a start state.
    fn prefix_at(&self, text: &[u8], at: usize) -> Option<usize> {
        self.prog.prefixes.find(&text[at..]).map(|(s, _)| at + s)
    }
    /// Returns the number of byte classes required to discriminate transitions
    /// in each state.
    ///
    /// invariant: num_byte_classes() == len(State.next)
    fn num_byte_classes(&self) -> usize {
        (self.prog.byte_classes[255] as usize + 1) + 1
    }
    /// Given an input byte or the special EOF sentinel, return its
    /// corresponding byte class.
    #[cfg_attr(feature = "perf-inline", inline(always))]
    fn byte_class(&self, b: Byte) -> usize {
        match b.as_byte() {
            None => self.num_byte_classes() - 1,
            Some(b) => self.u8_class(b),
        }
    }
    /// Like byte_class, but explicitly for u8s.
    #[cfg_attr(feature = "perf-inline", inline(always))]
    fn u8_class(&self, b: u8) -> usize {
        self.prog.byte_classes[b as usize] as usize
    }
    /// Returns true if the DFA should continue searching past the first match.
    ///
    /// Leftmost first semantics in the DFA are preserved by not following NFA
    /// transitions after the first match is seen.
    ///
    /// On occasion, we want to avoid leftmost first semantics to find either
    /// the longest match (for reverse search) or all possible matches (for
    /// regex sets).
    fn continue_past_first_match(&self) -> bool {
        self.prog.is_reverse || self.prog.matches.len() > 1
    }
    /// Returns true if there is a prefix we can quickly search for.
    fn has_prefix(&self) -> bool {
        !self.prog.is_reverse && !self.prog.prefixes.is_empty()
            && !self.prog.is_anchored_start
    }
    /// Sets the STATE_START bit in the given state pointer if and only if
    /// we have a prefix to scan for.
    ///
    /// If there's no prefix, then it's a waste to treat the start state
    /// specially.
    fn start_ptr(&self, si: StatePtr) -> StatePtr {
        if self.has_prefix() { si | STATE_START } else { si }
    }
    /// Approximate size returns the approximate heap space currently used by
    /// the DFA. It is used to determine whether the DFA's state cache needs to
    /// be wiped. Namely, it is possible that for certain regexes on certain
    /// inputs, a new state could be created for every byte of input. (This is
    /// bad for memory use, so we bound it with a cache.)
    fn approximate_size(&self) -> usize {
        self.cache.size + self.prog.approximate_size()
    }
}
/// An abstraction for representing a map of states. The map supports two
/// different ways of state lookup. One is fast constant time access via a
/// state pointer. The other is a hashmap lookup based on the DFA's
/// constituent NFA states.
///
/// A DFA state internally uses an Arc such that we only need to store the
/// set of NFA states on the heap once, even though we support looking up
/// states by two different means. A more natural way to express this might
/// use raw pointers, but an Arc is safe and effectively achieves the same
/// thing.
#[derive(Debug)]
struct StateMap {
    /// The keys are not actually static but rely on always pointing to a
    /// buffer in `states` which will never be moved except when clearing
    /// the map or on drop, in which case the keys of this map will be
    /// removed before
    map: HashMap<State, StatePtr>,
    /// Our set of states. Note that `StatePtr / num_byte_classes` indexes
    /// this Vec rather than just a `StatePtr`.
    states: Vec<State>,
    /// The number of byte classes in the DFA. Used to index `states`.
    num_byte_classes: usize,
}
impl StateMap {
    fn new(num_byte_classes: usize) -> StateMap {
        StateMap {
            map: HashMap::new(),
            states: vec![],
            num_byte_classes: num_byte_classes,
        }
    }
    fn len(&self) -> usize {
        self.states.len()
    }
    fn is_empty(&self) -> bool {
        self.states.is_empty()
    }
    fn get_ptr(&self, state: &State) -> Option<StatePtr> {
        self.map.get(state).cloned()
    }
    fn get_state(&self, si: StatePtr) -> Option<&State> {
        self.states.get(si as usize / self.num_byte_classes)
    }
    fn insert(&mut self, state: State, si: StatePtr) {
        self.map.insert(state.clone(), si);
        self.states.push(state);
    }
    fn clear(&mut self) {
        self.map.clear();
        self.states.clear();
    }
}
impl Transitions {
    /// Create a new transition table.
    ///
    /// The number of byte classes corresponds to the stride. Every state will
    /// have `num_byte_classes` slots for transitions.
    fn new(num_byte_classes: usize) -> Transitions {
        Transitions {
            table: vec![],
            num_byte_classes: num_byte_classes,
        }
    }
    /// Returns the total number of states currently in this table.
    fn num_states(&self) -> usize {
        self.table.len() / self.num_byte_classes
    }
    /// Allocates room for one additional state and returns a pointer to it.
    ///
    /// If there's no more room, None is returned.
    fn add(&mut self) -> Option<StatePtr> {
        let si = self.table.len();
        if si > STATE_MAX as usize {
            return None;
        }
        self.table.extend(repeat(STATE_UNKNOWN).take(self.num_byte_classes));
        Some(usize_to_u32(si))
    }
    /// Clears the table of all states.
    fn clear(&mut self) {
        self.table.clear();
    }
    /// Sets the transition from (si, cls) to next.
    fn set_next(&mut self, si: StatePtr, cls: usize, next: StatePtr) {
        self.table[si as usize + cls] = next;
    }
    /// Returns the transition corresponding to (si, cls).
    fn next(&self, si: StatePtr, cls: usize) -> StatePtr {
        self.table[si as usize + cls]
    }
    /// The heap size, in bytes, of a single state in the transition table.
    fn state_heap_size(&self) -> usize {
        self.num_byte_classes * mem::size_of::<StatePtr>()
    }
    /// Like `next`, but uses unchecked access and is therefore unsafe.
    unsafe fn next_unchecked(&self, si: StatePtr, cls: usize) -> StatePtr {
        debug_assert!((si as usize) < self.table.len());
        debug_assert!(cls < self.num_byte_classes);
        *self.table.get_unchecked(si as usize + cls)
    }
}
impl StateFlags {
    fn is_match(&self) -> bool {
        self.0 & 0b0000000_1 > 0
    }
    fn set_match(&mut self) {
        self.0 |= 0b0000000_1;
    }
    fn is_word(&self) -> bool {
        self.0 & 0b000000_1_0 > 0
    }
    fn set_word(&mut self) {
        self.0 |= 0b000000_1_0;
    }
    fn has_empty(&self) -> bool {
        self.0 & 0b00000_1_00 > 0
    }
    fn set_empty(&mut self) {
        self.0 |= 0b00000_1_00;
    }
}
impl Byte {
    fn byte(b: u8) -> Self {
        Byte(b as u16)
    }
    fn eof() -> Self {
        Byte(256)
    }
    fn is_eof(&self) -> bool {
        self.0 == 256
    }
    fn is_ascii_word(&self) -> bool {
        let b = match self.as_byte() {
            None => return false,
            Some(b) => b,
        };
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' => true,
            _ => false,
        }
    }
    fn as_byte(&self) -> Option<u8> {
        if self.is_eof() { None } else { Some(self.0 as u8) }
    }
}
impl fmt::Debug for State {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let ips: Vec<usize> = self.inst_ptrs().collect();
        f.debug_struct("State")
            .field("flags", &self.flags())
            .field("insts", &ips)
            .finish()
    }
}
impl fmt::Debug for Transitions {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut fmtd = f.debug_map();
        for si in 0..self.num_states() {
            let s = si * self.num_byte_classes;
            let e = s + self.num_byte_classes;
            fmtd.entry(&si.to_string(), &TransitionsRow(&self.table[s..e]));
        }
        fmtd.finish()
    }
}
struct TransitionsRow<'a>(&'a [StatePtr]);
impl<'a> fmt::Debug for TransitionsRow<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut fmtd = f.debug_map();
        for (b, si) in self.0.iter().enumerate() {
            match *si {
                STATE_UNKNOWN => {}
                STATE_DEAD => {
                    fmtd.entry(&vb(b as usize), &"DEAD");
                }
                si => {
                    fmtd.entry(&vb(b as usize), &si.to_string());
                }
            }
        }
        fmtd.finish()
    }
}
impl fmt::Debug for StateFlags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("StateFlags")
            .field("is_match", &self.is_match())
            .field("is_word", &self.is_word())
            .field("has_empty", &self.has_empty())
            .finish()
    }
}
/// Helper function for formatting a byte as a nice-to-read escaped string.
fn vb(b: usize) -> String {
    use std::ascii::escape_default;
    if b > ::std::u8::MAX as usize {
        "EOF".to_owned()
    } else {
        let escaped = escape_default(b as u8).collect::<Vec<u8>>();
        String::from_utf8_lossy(&escaped).into_owned()
    }
}
fn usize_to_u32(n: usize) -> u32 {
    if (n as u64) > (::std::u32::MAX as u64) {
        panic!("BUG: {} is too big to fit into u32", n)
    }
    n as u32
}
#[allow(dead_code)]
fn show_state_ptr(si: StatePtr) -> String {
    let mut s = format!("{:?}", si & STATE_MAX);
    if si == STATE_UNKNOWN {
        s = format!("{} (unknown)", s);
    }
    if si == STATE_DEAD {
        s = format!("{} (dead)", s);
    }
    if si == STATE_QUIT {
        s = format!("{} (quit)", s);
    }
    if si & STATE_START > 0 {
        s = format!("{} (start)", s);
    }
    if si & STATE_MATCH > 0 {
        s = format!("{} (match)", s);
    }
    s
}
/// https://developers.google.com/protocol-buffers/docs/encoding#varints
fn write_vari32(data: &mut Vec<u8>, n: i32) {
    let mut un = (n as u32) << 1;
    if n < 0 {
        un = !un;
    }
    write_varu32(data, un)
}
/// https://developers.google.com/protocol-buffers/docs/encoding#varints
fn read_vari32(data: &[u8]) -> (i32, usize) {
    let (un, i) = read_varu32(data);
    let mut n = (un >> 1) as i32;
    if un & 1 != 0 {
        n = !n;
    }
    (n, i)
}
/// https://developers.google.com/protocol-buffers/docs/encoding#varints
fn write_varu32(data: &mut Vec<u8>, mut n: u32) {
    while n >= 0b1000_0000 {
        data.push((n as u8) | 0b1000_0000);
        n >>= 7;
    }
    data.push(n as u8);
}
/// https://developers.google.com/protocol-buffers/docs/encoding#varints
fn read_varu32(data: &[u8]) -> (u32, usize) {
    let mut n: u32 = 0;
    let mut shift: u32 = 0;
    for (i, &b) in data.iter().enumerate() {
        if b < 0b1000_0000 {
            return (n | ((b as u32) << shift), i + 1);
        }
        n |= ((b as u32) & 0b0111_1111) << shift;
        shift += 7;
    }
    (0, 0)
}
#[cfg(test)]
mod tests {
    extern crate rand;
    use super::{
        push_inst_ptr, read_vari32, read_varu32, write_vari32, write_varu32, State,
        StateFlags,
    };
    use quickcheck::{quickcheck, QuickCheck, StdGen};
    use std::sync::Arc;
    #[test]
    fn prop_state_encode_decode() {
        fn p(ips: Vec<u32>, flags: u8) -> bool {
            let mut data = vec![flags];
            let mut prev = 0;
            for &ip in ips.iter() {
                push_inst_ptr(&mut data, &mut prev, ip);
            }
            let state = State {
                data: Arc::from(&data[..]),
            };
            let expected: Vec<usize> = ips.into_iter().map(|ip| ip as usize).collect();
            let got: Vec<usize> = state.inst_ptrs().collect();
            expected == got && state.flags() == StateFlags(flags)
        }
        QuickCheck::new()
            .gen(StdGen::new(self::rand::thread_rng(), 10_000))
            .quickcheck(p as fn(Vec<u32>, u8) -> bool);
    }
    #[test]
    fn prop_read_write_u32() {
        fn p(n: u32) -> bool {
            let mut buf = vec![];
            write_varu32(&mut buf, n);
            let (got, nread) = read_varu32(&buf);
            nread == buf.len() && got == n
        }
        quickcheck(p as fn(u32) -> bool);
    }
    #[test]
    fn prop_read_write_i32() {
        fn p(n: i32) -> bool {
            let mut buf = vec![];
            write_vari32(&mut buf, n);
            let (got, nread) = read_vari32(&buf);
            nread == buf.len() && got == n
        }
        quickcheck(p as fn(i32) -> bool);
    }
}
#[cfg(test)]
mod tests_llm_16_32 {
    use super::*;
    use crate::*;
    use std::vec::Vec;
    use std::iter::Iterator;
    #[test]
    fn test_next() {
        let _rug_st_tests_llm_16_32_rrrruuuugggg_test_next = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 0;
        let data: Vec<u8> = vec![rug_fuzz_0, 2, 3, 4, 5];
        let mut inst_ptrs = InstPtrs {
            base: rug_fuzz_1,
            data: &data,
        };
        debug_assert_eq!(inst_ptrs.next(), Some(1));
        debug_assert_eq!(inst_ptrs.next(), Some(3));
        debug_assert_eq!(inst_ptrs.next(), Some(6));
        debug_assert_eq!(inst_ptrs.next(), Some(10));
        debug_assert_eq!(inst_ptrs.next(), Some(15));
        debug_assert_eq!(inst_ptrs.next(), None);
        let _rug_ed_tests_llm_16_32_rrrruuuugggg_test_next = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_224 {
    use super::*;
    use crate::*;
    #[test]
    fn test_as_byte_none() {
        let _rug_st_tests_llm_16_224_rrrruuuugggg_test_as_byte_none = 0;
        let byte = Byte::eof();
        debug_assert_eq!(byte.as_byte(), None);
        let _rug_ed_tests_llm_16_224_rrrruuuugggg_test_as_byte_none = 0;
    }
    #[test]
    fn test_as_byte_some() {
        let _rug_st_tests_llm_16_224_rrrruuuugggg_test_as_byte_some = 0;
        let rug_fuzz_0 = b'A';
        let byte = Byte::byte(rug_fuzz_0);
        debug_assert_eq!(byte.as_byte(), Some(b'A'));
        let _rug_ed_tests_llm_16_224_rrrruuuugggg_test_as_byte_some = 0;
    }
    #[test]
    fn test_as_byte_some_eof() {
        let _rug_st_tests_llm_16_224_rrrruuuugggg_test_as_byte_some_eof = 0;
        let rug_fuzz_0 = 255;
        let byte = Byte::byte(rug_fuzz_0);
        debug_assert_eq!(byte.as_byte(), Some(255));
        let _rug_ed_tests_llm_16_224_rrrruuuugggg_test_as_byte_some_eof = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_227 {
    use super::*;
    use crate::*;
    #[test]
    fn test_eof() {
        let _rug_st_tests_llm_16_227_rrrruuuugggg_test_eof = 0;
        let result = Byte::eof();
        debug_assert_eq!(result.0, 256);
        debug_assert!(result.is_eof());
        let _rug_ed_tests_llm_16_227_rrrruuuugggg_test_eof = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_229 {
    use crate::dfa::Byte;
    #[test]
    fn test_is_ascii_word() {
        let _rug_st_tests_llm_16_229_rrrruuuugggg_test_is_ascii_word = 0;
        let rug_fuzz_0 = b'A';
        let rug_fuzz_1 = b'z';
        let rug_fuzz_2 = b'0';
        let rug_fuzz_3 = b'9';
        let rug_fuzz_4 = b'_';
        let rug_fuzz_5 = b' ';
        let rug_fuzz_6 = b'&';
        let rug_fuzz_7 = 255;
        let byte_a = Byte::byte(rug_fuzz_0);
        debug_assert_eq!(byte_a.is_ascii_word(), true);
        let byte_z = Byte::byte(rug_fuzz_1);
        debug_assert_eq!(byte_z.is_ascii_word(), true);
        let byte_0 = Byte::byte(rug_fuzz_2);
        debug_assert_eq!(byte_0.is_ascii_word(), true);
        let byte_9 = Byte::byte(rug_fuzz_3);
        debug_assert_eq!(byte_9.is_ascii_word(), true);
        let byte_underscore = Byte::byte(rug_fuzz_4);
        debug_assert_eq!(byte_underscore.is_ascii_word(), true);
        let byte_space = Byte::byte(rug_fuzz_5);
        debug_assert_eq!(byte_space.is_ascii_word(), false);
        let byte_ampersand = Byte::byte(rug_fuzz_6);
        debug_assert_eq!(byte_ampersand.is_ascii_word(), false);
        let byte_special = Byte::byte(rug_fuzz_7);
        debug_assert_eq!(byte_special.is_ascii_word(), false);
        let byte_none = Byte::eof();
        debug_assert_eq!(byte_none.is_ascii_word(), false);
        let _rug_ed_tests_llm_16_229_rrrruuuugggg_test_is_ascii_word = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_230 {
    use super::*;
    use crate::*;
    use std::clone::Clone;
    use std::fmt::Debug;
    use std::marker::Copy;
    #[test]
    fn test_is_eof() {
        let _rug_st_tests_llm_16_230_rrrruuuugggg_test_is_eof = 0;
        let rug_fuzz_0 = b'A';
        let byte = Byte::eof();
        debug_assert!(byte.is_eof());
        let byte = Byte::byte(rug_fuzz_0);
        debug_assert!(! byte.is_eof());
        let _rug_ed_tests_llm_16_230_rrrruuuugggg_test_is_eof = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_231 {
    use super::*;
    use crate::*;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_231_rrrruuuugggg_test_new = 0;
        let dummy_prog = Program::new();
        let unit_test = dummy_prog;
        debug_assert_eq!(unit_test.len(), 0);
        let _rug_ed_tests_llm_16_231_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_232 {
    use super::*;
    use crate::*;
    use std::mem;
    #[test]
    fn test_reset_size() {
        let _rug_st_tests_llm_16_232_rrrruuuugggg_test_reset_size = 0;
        let rug_fuzz_0 = 256;
        let rug_fuzz_1 = 256;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 10;
        let rug_fuzz_5 = 20;
        let mut cache_inner = CacheInner {
            compiled: StateMap::new(rug_fuzz_0),
            trans: Transitions::new(rug_fuzz_1),
            start_states: Vec::new(),
            stack: Vec::new(),
            flush_count: rug_fuzz_2,
            size: rug_fuzz_3,
            insts_scratch_space: Vec::new(),
        };
        let state_ptr_size = mem::size_of::<StatePtr>();
        let state_count = rug_fuzz_4;
        let stack_count = rug_fuzz_5;
        cache_inner.start_states = vec![StatePtr::default(); state_count];
        cache_inner.stack = vec![InstPtr::default(); stack_count];
        let expected_size = state_count * state_ptr_size
            + stack_count * mem::size_of::<InstPtr>();
        cache_inner.reset_size();
        debug_assert_eq!(cache_inner.size, expected_size);
        let _rug_ed_tests_llm_16_232_rrrruuuugggg_test_reset_size = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_240_llm_16_239 {
    use crate::dfa::Result;
    use crate::dfa::Result::*;
    use crate::dfa::Result::{Match, NoMatch, Quit};
    fn is_match(result: &Result<i32>) -> bool {
        match result {
            Match(_) => true,
            NoMatch(_) | Quit => false,
        }
    }
    #[test]
    fn test_is_match_match() {
        let result = Match(42);
        assert_eq!(is_match(& result), true);
    }
    #[test]
    fn test_is_match_no_match() {
        let result = NoMatch(10);
        assert_eq!(is_match(& result), false);
    }
    #[test]
    fn test_is_match_quit() {
        let result = Quit;
        assert_eq!(is_match(& result), false);
    }
}
#[cfg(test)]
mod tests_llm_16_247 {
    use super::*;
    use crate::*;
    use std::sync::Arc;
    #[test]
    fn test_inst_ptrs() {
        let _rug_st_tests_llm_16_247_rrrruuuugggg_test_inst_ptrs = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 3;
        let rug_fuzz_4 = 4;
        let rug_fuzz_5 = 5;
        let data: Arc<[u8]> = Arc::from([
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
        ]);
        let state = State { data };
        let mut inst_ptrs = state.inst_ptrs();
        debug_assert_eq!(inst_ptrs.next(), Some(1));
        debug_assert_eq!(inst_ptrs.next(), Some(3));
        debug_assert_eq!(inst_ptrs.next(), Some(6));
        debug_assert_eq!(inst_ptrs.next(), None);
        let _rug_ed_tests_llm_16_247_rrrruuuugggg_test_inst_ptrs = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_248 {
    use super::*;
    use crate::*;
    #[test]
    fn test_has_empty() {
        let _rug_st_tests_llm_16_248_rrrruuuugggg_test_has_empty = 0;
        let rug_fuzz_0 = 0;
        let mut state_flags = StateFlags(rug_fuzz_0);
        debug_assert_eq!(state_flags.has_empty(), false);
        state_flags.set_empty();
        debug_assert_eq!(state_flags.has_empty(), true);
        let _rug_ed_tests_llm_16_248_rrrruuuugggg_test_has_empty = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_249 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_match_returns_true_when_bit_is_set() {
        let _rug_st_tests_llm_16_249_rrrruuuugggg_test_is_match_returns_true_when_bit_is_set = 0;
        let rug_fuzz_0 = 0b0000000_1;
        let state_flags = StateFlags(rug_fuzz_0);
        debug_assert_eq!(state_flags.is_match(), true);
        let _rug_ed_tests_llm_16_249_rrrruuuugggg_test_is_match_returns_true_when_bit_is_set = 0;
    }
    #[test]
    fn test_is_match_returns_false_when_bit_is_not_set() {
        let _rug_st_tests_llm_16_249_rrrruuuugggg_test_is_match_returns_false_when_bit_is_not_set = 0;
        let rug_fuzz_0 = 0b0000000_0;
        let state_flags = StateFlags(rug_fuzz_0);
        debug_assert_eq!(state_flags.is_match(), false);
        let _rug_ed_tests_llm_16_249_rrrruuuugggg_test_is_match_returns_false_when_bit_is_not_set = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_250 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_word_true() {
        let _rug_st_tests_llm_16_250_rrrruuuugggg_test_is_word_true = 0;
        let rug_fuzz_0 = 0b000000_1_0;
        let state_flags = StateFlags(rug_fuzz_0);
        debug_assert_eq!(state_flags.is_word(), true);
        let _rug_ed_tests_llm_16_250_rrrruuuugggg_test_is_word_true = 0;
    }
    #[test]
    fn test_is_word_false() {
        let _rug_st_tests_llm_16_250_rrrruuuugggg_test_is_word_false = 0;
        let rug_fuzz_0 = 0b0000000;
        let state_flags = StateFlags(rug_fuzz_0);
        debug_assert_eq!(state_flags.is_word(), false);
        let _rug_ed_tests_llm_16_250_rrrruuuugggg_test_is_word_false = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_251 {
    use super::*;
    use crate::*;
    use crate::dfa::StateFlags;
    #[test]
    fn test_set_empty() {
        let _rug_st_tests_llm_16_251_rrrruuuugggg_test_set_empty = 0;
        let rug_fuzz_0 = 0b0000000_0;
        let rug_fuzz_1 = 0b00000_1_00;
        let mut state = StateFlags(rug_fuzz_0);
        state.set_empty();
        debug_assert_eq!(state.0 & rug_fuzz_1, 0b00000_1_00);
        let _rug_ed_tests_llm_16_251_rrrruuuugggg_test_set_empty = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_252 {
    use super::*;
    use crate::*;
    #[test]
    fn test_set_match() {
        let _rug_st_tests_llm_16_252_rrrruuuugggg_test_set_match = 0;
        let rug_fuzz_0 = 0b0000000_0;
        let mut state_flags = StateFlags(rug_fuzz_0);
        state_flags.set_match();
        debug_assert_eq!(state_flags.is_match(), true);
        let _rug_ed_tests_llm_16_252_rrrruuuugggg_test_set_match = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_253 {
    use super::*;
    use crate::*;
    #[test]
    fn test_set_word() {
        let _rug_st_tests_llm_16_253_rrrruuuugggg_test_set_word = 0;
        let rug_fuzz_0 = 0;
        let mut flags = StateFlags(rug_fuzz_0);
        flags.set_word();
        debug_assert_eq!(flags.0, 0b000000_1_0);
        let _rug_ed_tests_llm_16_253_rrrruuuugggg_test_set_word = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_260 {
    use super::*;
    use crate::*;
    use crate::{backtrack, dfa};
    #[test]
    fn test_insert() {
        let _rug_st_tests_llm_16_260_rrrruuuugggg_test_insert = 0;
        let rug_fuzz_0 = 256;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let mut state_map = dfa::StateMap::new(rug_fuzz_0);
        let state = dfa::State {
            data: Arc::new([rug_fuzz_1; 8]),
        };
        let state_ptr = rug_fuzz_2;
        state_map.insert(state.clone(), state_ptr);
        debug_assert_eq!(state_map.len(), 1);
        debug_assert_eq!(state_map.is_empty(), false);
        debug_assert_eq!(state_map.get_ptr(& state), Some(state_ptr));
        debug_assert_eq!(state_map.get_state(state_ptr), Some(& state));
        state_map.clear();
        debug_assert_eq!(state_map.len(), 0);
        debug_assert_eq!(state_map.is_empty(), true);
        let _rug_ed_tests_llm_16_260_rrrruuuugggg_test_insert = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_267 {
    use super::*;
    use crate::*;
    use std::iter::repeat;
    #[test]
    fn test_add() {
        let mut transitions = Transitions::new(5);
        let expected_len = transitions.num_byte_classes;
        let expected_state_ptr = expected_len as StatePtr;
        let result = transitions.add();
        assert_eq!(result, Some(expected_state_ptr));
        assert_eq!(transitions.table.len(), expected_len);
        assert_eq!(
            transitions.table, repeat(STATE_UNKNOWN).take(expected_len).collect::< Vec <
            StatePtr >> ()
        );
    }
    #[test]
    fn test_add_with_full_table() {
        let mut transitions = Transitions::new(5);
        transitions.table = repeat(STATE_UNKNOWN).take(TRANSITION_TABLE_SIZE).collect();
        let result = transitions.add();
        assert_eq!(result, None);
        assert_eq!(transitions.table.len(), TRANSITION_TABLE_SIZE);
    }
    const TRANSITION_TABLE_SIZE: usize = STATE_MAX as usize + 1;
    const STATE_MAX: u32 = u32::MAX;
    const STATE_UNKNOWN: StatePtr = u32::MAX;
    type StatePtr = u32;
    fn usize_to_u32(n: usize) -> StatePtr {
        n as StatePtr
    }
    #[derive(Debug)]
    struct Transitions {
        table: Vec<StatePtr>,
        num_byte_classes: usize,
    }
    impl Transitions {
        fn new(num_byte_classes: usize) -> Transitions {
            Transitions {
                table: Vec::new(),
                num_byte_classes,
            }
        }
        fn num_states(&self) -> usize {
            self.table.len() / self.num_byte_classes
        }
        fn add(&mut self) -> Option<StatePtr> {
            let si = self.table.len();
            if si > STATE_MAX as usize {
                return None;
            }
            self.table.extend(repeat(STATE_UNKNOWN).take(self.num_byte_classes));
            Some(usize_to_u32(si))
        }
    }
}
#[cfg(test)]
mod tests_llm_16_268 {
    use crate::dfa::Transitions;
    #[test]
    fn test_clear() {
        let _rug_st_tests_llm_16_268_rrrruuuugggg_test_clear = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let mut transitions = Transitions::new(rug_fuzz_0);
        transitions.clear();
        let expected_table = vec![];
        let expected_num_byte_classes = rug_fuzz_1;
        debug_assert_eq!(transitions.table, expected_table);
        debug_assert_eq!(transitions.num_byte_classes, expected_num_byte_classes);
        let _rug_ed_tests_llm_16_268_rrrruuuugggg_test_clear = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_269 {
    use crate::dfa::Transitions;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_269_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = 10;
        let num_byte_classes = rug_fuzz_0;
        let transitions = Transitions::new(num_byte_classes);
        debug_assert_eq!(transitions.num_byte_classes, num_byte_classes);
        debug_assert_eq!(transitions.table.len(), 0);
        let _rug_ed_tests_llm_16_269_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_270 {
    use super::*;
    use crate::*;
    #[test]
    fn test_next() {
        let _rug_st_tests_llm_16_270_rrrruuuugggg_test_next = 0;
        let rug_fuzz_0 = 3;
        let rug_fuzz_1 = 1;
        let mut transitions = Transitions::new(rug_fuzz_0);
        let cls = rug_fuzz_1;
        let si = transitions.add().unwrap();
        transitions.set_next(si, cls, si);
        let next_state = transitions.next(si, cls);
        debug_assert_eq!(next_state, si);
        let _rug_ed_tests_llm_16_270_rrrruuuugggg_test_next = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_271 {
    use super::*;
    use crate::*;
    #[test]
    fn test_next_unchecked() {
        let _rug_st_tests_llm_16_271_rrrruuuugggg_test_next_unchecked = 0;
        let rug_fuzz_0 = 4;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let num_byte_classes = rug_fuzz_0;
        let mut transitions = Transitions::new(num_byte_classes);
        transitions.add();
        transitions.add();
        transitions.add();
        transitions.set_next(rug_fuzz_1, rug_fuzz_2, rug_fuzz_3);
        let si: StatePtr = rug_fuzz_4;
        let cls: usize = rug_fuzz_5;
        let result: StatePtr = unsafe { transitions.next_unchecked(si, cls) };
        debug_assert_eq!(result, 1);
        let _rug_ed_tests_llm_16_271_rrrruuuugggg_test_next_unchecked = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_272 {
    use super::*;
    use crate::*;
    use std::iter::repeat;
    #[test]
    fn test_num_states() {
        let _rug_st_tests_llm_16_272_rrrruuuugggg_test_num_states = 0;
        let rug_fuzz_0 = 3;
        let mut transitions = Transitions::new(rug_fuzz_0);
        debug_assert_eq!(transitions.num_states(), 0);
        let state1 = transitions.add();
        debug_assert_eq!(transitions.num_states(), 1);
        let state2 = transitions.add();
        debug_assert_eq!(transitions.num_states(), 2);
        let state3 = transitions.add();
        debug_assert_eq!(transitions.num_states(), 3);
        transitions.clear();
        debug_assert_eq!(transitions.num_states(), 0);
        let _rug_ed_tests_llm_16_272_rrrruuuugggg_test_num_states = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_273 {
    use super::*;
    use crate::*;
    #[test]
    fn test_set_next() {
        let _rug_st_tests_llm_16_273_rrrruuuugggg_test_set_next = 0;
        let rug_fuzz_0 = 256;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 1;
        let rug_fuzz_6 = 1;
        let rug_fuzz_7 = 1;
        let rug_fuzz_8 = 0;
        let rug_fuzz_9 = 0;
        let rug_fuzz_10 = 1;
        let rug_fuzz_11 = 1;
        let rug_fuzz_12 = 0;
        let rug_fuzz_13 = 0;
        let rug_fuzz_14 = 0;
        let rug_fuzz_15 = 0;
        let rug_fuzz_16 = 1;
        let rug_fuzz_17 = 1;
        let rug_fuzz_18 = 0;
        let rug_fuzz_19 = 1;
        let rug_fuzz_20 = 1;
        let mut transitions = Transitions::new(rug_fuzz_0);
        transitions.add();
        transitions.add();
        transitions.set_next(rug_fuzz_1, rug_fuzz_2, rug_fuzz_3);
        transitions.set_next(rug_fuzz_4, rug_fuzz_5, rug_fuzz_6);
        transitions.set_next(rug_fuzz_7, rug_fuzz_8, rug_fuzz_9);
        transitions.set_next(rug_fuzz_10, rug_fuzz_11, rug_fuzz_12);
        debug_assert_eq!(transitions.next(rug_fuzz_13, rug_fuzz_14), 1);
        debug_assert_eq!(transitions.next(rug_fuzz_15, rug_fuzz_16), 1);
        debug_assert_eq!(transitions.next(rug_fuzz_17, rug_fuzz_18), 0);
        debug_assert_eq!(transitions.next(rug_fuzz_19, rug_fuzz_20), 0);
        let _rug_ed_tests_llm_16_273_rrrruuuugggg_test_set_next = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_274 {
    use super::*;
    use crate::*;
    use std::mem;
    #[test]
    fn test_state_heap_size() {
        let _rug_st_tests_llm_16_274_rrrruuuugggg_test_state_heap_size = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let transitions = Transitions::new(rug_fuzz_0);
        let heap_size = transitions.state_heap_size();
        let expected_heap_size = rug_fuzz_1 * mem::size_of::<StatePtr>();
        debug_assert_eq!(heap_size, expected_heap_size);
        let _rug_ed_tests_llm_16_274_rrrruuuugggg_test_state_heap_size = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_277 {
    use super::*;
    use crate::*;
    #[test]
    fn test_push_inst_ptr() {
        let _rug_st_tests_llm_16_277_rrrruuuugggg_test_push_inst_ptr = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 100;
        let mut data: Vec<u8> = Vec::new();
        let mut prev: InstPtr = rug_fuzz_0;
        let ip: InstPtr = rug_fuzz_1;
        push_inst_ptr(&mut data, &mut prev, ip);
        debug_assert_eq!(data, [156, 1]);
        debug_assert_eq!(prev, 100);
        let _rug_ed_tests_llm_16_277_rrrruuuugggg_test_push_inst_ptr = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_278 {
    use super::*;
    use crate::*;
    #[test]
    fn test_read_vari32() {
        let _rug_st_tests_llm_16_278_rrrruuuugggg_test_read_vari32 = 0;
        let rug_fuzz_0 = 0x8A;
        let rug_fuzz_1 = 0x8A;
        let rug_fuzz_2 = 0x02;
        let rug_fuzz_3 = 22123;
        let rug_fuzz_4 = 3;
        let data = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        let expected = (rug_fuzz_3, rug_fuzz_4);
        debug_assert_eq!(read_vari32(data), expected);
        let _rug_ed_tests_llm_16_278_rrrruuuugggg_test_read_vari32 = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_279 {
    use super::*;
    use crate::*;
    #[test]
    fn test_read_varu32() {
        let _rug_st_tests_llm_16_279_rrrruuuugggg_test_read_varu32 = 0;
        let rug_fuzz_0 = 0b0000_0010;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 0b1010_1010;
        let rug_fuzz_4 = 0b1000_0010;
        let rug_fuzz_5 = 106;
        let rug_fuzz_6 = 2;
        let rug_fuzz_7 = 0b1010_1010;
        let rug_fuzz_8 = 0b1000_0010;
        let rug_fuzz_9 = 0b1000_0001;
        let rug_fuzz_10 = 13825;
        let rug_fuzz_11 = 3;
        let rug_fuzz_12 = 0b1010_1010;
        let rug_fuzz_13 = 0b1000_0010;
        let rug_fuzz_14 = 0b1000_0001;
        let rug_fuzz_15 = 0b0000_0000;
        let rug_fuzz_16 = 13825;
        let rug_fuzz_17 = 3;
        let rug_fuzz_18 = 0b1010_1010;
        let rug_fuzz_19 = 0b1000_0010;
        let rug_fuzz_20 = 0b1000_0001;
        let rug_fuzz_21 = 0b1000_0000;
        let rug_fuzz_22 = 13825;
        let rug_fuzz_23 = 3;
        let data: &[u8] = &[rug_fuzz_0];
        let expected: (u32, usize) = (rug_fuzz_1, rug_fuzz_2);
        debug_assert_eq!(read_varu32(data), expected);
        let data: &[u8] = &[rug_fuzz_3, rug_fuzz_4];
        let expected: (u32, usize) = (rug_fuzz_5, rug_fuzz_6);
        debug_assert_eq!(read_varu32(data), expected);
        let data: &[u8] = &[rug_fuzz_7, rug_fuzz_8, rug_fuzz_9];
        let expected: (u32, usize) = (rug_fuzz_10, rug_fuzz_11);
        debug_assert_eq!(read_varu32(data), expected);
        let data: &[u8] = &[rug_fuzz_12, rug_fuzz_13, rug_fuzz_14, rug_fuzz_15];
        let expected: (u32, usize) = (rug_fuzz_16, rug_fuzz_17);
        debug_assert_eq!(read_varu32(data), expected);
        let data: &[u8] = &[rug_fuzz_18, rug_fuzz_19, rug_fuzz_20, rug_fuzz_21];
        let expected: (u32, usize) = (rug_fuzz_22, rug_fuzz_23);
        debug_assert_eq!(read_varu32(data), expected);
        let _rug_ed_tests_llm_16_279_rrrruuuugggg_test_read_varu32 = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_282 {
    use super::*;
    use crate::*;
    use std::u32;
    #[test]
    fn test_usize_to_u32() {
        let _rug_st_tests_llm_16_282_rrrruuuugggg_test_usize_to_u32 = 0;
        let rug_fuzz_0 = 0usize;
        let rug_fuzz_1 = 1usize;
        debug_assert_eq!(usize_to_u32(rug_fuzz_0), 0u32);
        debug_assert_eq!(usize_to_u32(rug_fuzz_1), 1u32);
        debug_assert_eq!(usize_to_u32(u32::MAX as usize), u32::MAX);
        let _rug_ed_tests_llm_16_282_rrrruuuugggg_test_usize_to_u32 = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_285 {
    use crate::dfa::write_vari32;
    #[test]
    fn test_write_vari32() {
        let mut data: Vec<u8> = Vec::new();
        write_vari32(&mut data, 0);
        assert_eq!(data, vec![0x00]);
        let mut data: Vec<u8> = Vec::new();
        write_vari32(&mut data, 127);
        assert_eq!(data, vec![0xFE, 0x01]);
        let mut data: Vec<u8> = Vec::new();
        write_vari32(&mut data, -128);
        assert_eq!(data, vec![0x01]);
        let mut data: Vec<u8> = Vec::new();
        write_vari32(&mut data, -129);
        assert_eq!(data, vec![0xFF, 0x01]);
        let mut data: Vec<u8> = Vec::new();
        write_vari32(&mut data, -2147483648);
        assert_eq!(data, vec![0xFF, 0xFF, 0xFF, 0xFF, 0x0F]);
        let mut data: Vec<u8> = Vec::new();
        write_vari32(&mut data, 2147483647);
        assert_eq!(data, vec![0xFE, 0xFF, 0xFF, 0xFF, 0x0F]);
    }
}
#[cfg(test)]
mod tests_llm_16_286 {
    use crate::dfa::write_varu32;
    #[test]
    fn test_write_varu32() {
        let _rug_st_tests_llm_16_286_rrrruuuugggg_test_write_varu32 = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 127;
        let rug_fuzz_2 = 128;
        let rug_fuzz_3 = 300;
        let rug_fuzz_4 = 75263996;
        let rug_fuzz_5 = 536870911;
        let rug_fuzz_6 = 268435455;
        let mut data = Vec::new();
        write_varu32(&mut data, rug_fuzz_0);
        debug_assert_eq!(data, vec![0]);
        write_varu32(&mut data, rug_fuzz_1);
        debug_assert_eq!(data, vec![0, 127]);
        write_varu32(&mut data, rug_fuzz_2);
        debug_assert_eq!(data, vec![0, 127, 128]);
        write_varu32(&mut data, rug_fuzz_3);
        debug_assert_eq!(data, vec![0, 127, 128, 172, 2]);
        write_varu32(&mut data, rug_fuzz_4);
        debug_assert_eq!(data, vec![0, 127, 128, 172, 2, 188, 236, 116]);
        write_varu32(&mut data, rug_fuzz_5);
        debug_assert_eq!(
            data, vec![0, 127, 128, 172, 2, 188, 236, 116, 255, 255, 255, 31]
        );
        write_varu32(&mut data, rug_fuzz_6);
        debug_assert_eq!(
            data, vec![0, 127, 128, 172, 2, 188, 236, 116, 255, 255, 255, 31, 255, 255,
            255, 7]
        );
        let _rug_ed_tests_llm_16_286_rrrruuuugggg_test_write_varu32 = 0;
    }
}
#[cfg(test)]
mod tests_rug_55 {
    use super::*;
    use crate::internal::Program;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_55_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0 = Program::new();
        p0.dfa_size_limit = rug_fuzz_0;
        crate::dfa::can_exec(&p0);
        let _rug_ed_tests_rug_55_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_56 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_56_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 50;
        let p0: usize = rug_fuzz_0;
        crate::dfa::vb(p0);
        let _rug_ed_tests_rug_56_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_57 {
    use super::*;
    #[test]
    fn test_show_state_ptr() {
        let _rug_st_tests_rug_57_rrrruuuugggg_test_show_state_ptr = 0;
        let rug_fuzz_0 = 10;
        let mut p0: u32 = rug_fuzz_0;
        crate::dfa::show_state_ptr(p0);
        let _rug_ed_tests_rug_57_rrrruuuugggg_test_show_state_ptr = 0;
    }
}
#[cfg(test)]
mod tests_rug_59 {
    use super::*;
    use crate::dfa::Result;
    #[test]
    fn test_set_non_match() {
        let _rug_st_tests_rug_59_rrrruuuugggg_test_set_non_match = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 10;
        let mut p0: Result<usize> = Result::NoMatch(rug_fuzz_0);
        let p1: usize = rug_fuzz_1;
        p0.set_non_match(p1);
        let _rug_ed_tests_rug_59_rrrruuuugggg_test_set_non_match = 0;
    }
}
#[cfg(test)]
mod tests_rug_60 {
    use super::*;
    use crate::dfa::State;
    use crate::dfa::StateFlags;
    use std::sync::Arc;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_60_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let mut p0 = State {
            data: Arc::from(vec![rug_fuzz_0, 1, 2, 3, 4, 5]),
        };
        crate::dfa::State::flags(&mut p0);
        let _rug_ed_tests_rug_60_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_64 {
    use super::*;
    use crate::dfa::Fsm;
    use crate::sparse::SparseSet;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_64_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let rug_fuzz_2 = b"example_text";
        let mut p0: Fsm = unimplemented!();
        let mut p1: SparseSet = SparseSet::new(rug_fuzz_0);
        let mut p2: SparseSet = SparseSet::new(rug_fuzz_1);
        let p3: &[u8] = rug_fuzz_2;
        p0.exec_at(&mut p1, &mut p2, p3);
        let _rug_ed_tests_rug_64_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_65 {
    use super::*;
    use crate::dfa::Fsm;
    use crate::sparse::SparseSet;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_65_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let rug_fuzz_2 = b"example";
        let mut p0: Fsm<'static> = unimplemented!();
        let mut p1: SparseSet = SparseSet::new(rug_fuzz_0);
        let mut p2: SparseSet = SparseSet::new(rug_fuzz_1);
        let p3: &[u8] = rug_fuzz_2;
        p0.exec_at_reverse(&mut p1, &mut p2, p3);
        let _rug_ed_tests_rug_65_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_66 {
    use super::*;
    use crate::dfa::Fsm;
    #[test]
    fn test_next_si() {
        let _rug_st_tests_rug_66_rrrruuuugggg_test_next_si = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = b'a';
        let rug_fuzz_2 = b'b';
        let rug_fuzz_3 = b'c';
        let rug_fuzz_4 = 0;
        let p0: Fsm = unimplemented!();
        let p1: u32 = rug_fuzz_0;
        let p2: &[u8] = &[rug_fuzz_1, rug_fuzz_2, rug_fuzz_3];
        let p3: usize = rug_fuzz_4;
        unsafe {
            p0.next_si(p1, p2, p3);
        }
        let _rug_ed_tests_rug_66_rrrruuuugggg_test_next_si = 0;
    }
}
#[cfg(test)]
mod tests_rug_67 {
    use super::*;
    use crate::dfa::{Fsm, Byte};
    use crate::sparse::SparseSet;
    #[test]
    fn test_exec_byte() {
        let _rug_st_tests_rug_67_rrrruuuugggg_test_exec_byte = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let rug_fuzz_2 = b'A';
        let mut p0: Fsm<'static> = unimplemented!();
        let mut p1 = SparseSet::new(rug_fuzz_0);
        let mut p2 = SparseSet::new(rug_fuzz_1);
        let mut p3: StatePtr = unimplemented!();
        let mut p4 = Byte::byte(rug_fuzz_2);
        p0.exec_byte(&mut p1, &mut p2, p3, p4);
        let _rug_ed_tests_rug_67_rrrruuuugggg_test_exec_byte = 0;
    }
}
#[cfg(test)]
mod tests_rug_68 {
    use super::*;
    use crate::dfa::Fsm;
    use crate::dfa::EmptyFlags;
    use crate::sparse::SparseSet;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_68_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: Fsm<'static> = unimplemented!();
        let p1: u32 = unimplemented!();
        let mut v31 = SparseSet::new(rug_fuzz_0);
        let p2: &mut SparseSet = &mut v31;
        let mut v33 = EmptyFlags::default();
        let p3: EmptyFlags = v33;
        p0.follow_epsilons(p1, p2, p3);
        let _rug_ed_tests_rug_68_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_73 {
    use super::*;
    use crate::dfa::{Fsm, State};
    use std::sync::Arc;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_73_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let mut p0: Fsm<'static> = todo!();
        let p1 = State {
            data: Arc::from(vec![rug_fuzz_0, 1, 2, 3, 4, 5]),
        };
        p0.restore_state(p1);
        let _rug_ed_tests_rug_73_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_74 {
    use super::*;
    use crate::dfa;
    use crate::sparse::SparseSet;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_74_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let rug_fuzz_2 = b'A';
        let mut p0: dfa::Fsm<'static> = unimplemented!();
        let mut p1 = SparseSet::new(rug_fuzz_0);
        let mut p2 = SparseSet::new(rug_fuzz_1);
        let p3: StatePtr = unimplemented!();
        let mut p4 = Byte::byte(rug_fuzz_2);
        p0.next_state(&mut p1, &mut p2, p3, p4);
        let _rug_ed_tests_rug_74_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_75 {
    use super::*;
    use crate::dfa::{Fsm, SparseSet, EmptyFlags, StateFlags};
    #[test]
    fn test_start_state() {
        let _rug_st_tests_rug_75_rrrruuuugggg_test_start_state = 0;
        let rug_fuzz_0 = 10;
        let mut p0: Fsm<'static> = unimplemented!();
        let mut p1 = SparseSet::new(rug_fuzz_0);
        let mut p2 = EmptyFlags::default();
        let mut p3 = StateFlags::default();
        Fsm::<'static>::start_state(&mut p0, &mut p1, p2, p3);
        let _rug_ed_tests_rug_75_rrrruuuugggg_test_start_state = 0;
    }
}
#[cfg(test)]
mod tests_rug_76 {
    use super::*;
    use crate::dfa::{Fsm, EmptyFlags, StateFlags};
    #[test]
    fn test_start_flags() {
        let _rug_st_tests_rug_76_rrrruuuugggg_test_start_flags = 0;
        let mut fsm: Fsm = unimplemented!();
        let text: &[u8] = unimplemented!();
        let at: usize = unimplemented!();
        fsm.start_flags(text, at);
        let _rug_ed_tests_rug_76_rrrruuuugggg_test_start_flags = 0;
    }
}
#[cfg(test)]
mod tests_rug_77 {
    use super::*;
    use crate::dfa::{EmptyFlags, StateFlags, Fsm};
    #[test]
    fn test_start_flags_reverse() {
        let fsm: Fsm<'static> = todo!("Construct the variable fsm using dfa::Fsm<'a>");
        let text: &[u8] = todo!("Provide sample data for the text argument");
        let at: usize = todo!("Provide sample data for the at argument");
        let (empty_flags, state_flags) = fsm.start_flags_reverse(text, at);
    }
}
#[cfg(test)]
mod tests_rug_80 {
    use super::*;
    use crate::dfa::Fsm;
    #[test]
    fn test_prefix_at() {
        let _rug_st_tests_rug_80_rrrruuuugggg_test_prefix_at = 0;
        let p0: Fsm = unimplemented!();
        let p1: &[u8] = unimplemented!();
        let p2: usize = unimplemented!();
        p0.prefix_at(p1, p2);
        let _rug_ed_tests_rug_80_rrrruuuugggg_test_prefix_at = 0;
    }
}
#[cfg(test)]
mod tests_rug_82 {
    use super::*;
    use crate::dfa::{Fsm, Byte};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_82_rrrruuuugggg_test_rug = 0;
        let mut p0: Fsm<'static> = unimplemented!();
        let mut p1: Byte = unimplemented!();
        p0.byte_class(p1);
        let _rug_ed_tests_rug_82_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_83 {
    use super::*;
    use crate::dfa::Fsm;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_83_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: Fsm = unimplemented!();
        let p1: u8 = rug_fuzz_0;
        p0.u8_class(p1);
        let _rug_ed_tests_rug_83_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_88 {
    use super::*;
    use crate::dfa::StateMap;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_88_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 5;
        let num_byte_classes: usize = rug_fuzz_0;
        StateMap::new(num_byte_classes);
        let _rug_ed_tests_rug_88_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_89 {
    use super::*;
    use crate::dfa::StateMap;
    #[test]
    fn test_len() {
        let _rug_st_tests_rug_89_rrrruuuugggg_test_len = 0;
        let rug_fuzz_0 = 10;
        let mut p0 = StateMap::new(rug_fuzz_0);
        debug_assert_eq!(p0.len(), 0);
        let _rug_ed_tests_rug_89_rrrruuuugggg_test_len = 0;
    }
}
#[cfg(test)]
mod tests_rug_90 {
    use super::*;
    use crate::dfa::StateMap;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_90_rrrruuuugggg_sample = 0;
        let rug_fuzz_0 = 10;
        #[cfg(test)]
        mod tests_rug_90_prepare {
            use crate::dfa::StateMap;
            #[test]
            fn sample() {
                let _rug_st_tests_rug_90_prepare_rrrruuuugggg_sample = 0;
                let rug_fuzz_0 = 0;
                let rug_fuzz_1 = 10;
                let rug_fuzz_2 = 0;
                let _rug_st_tests_rug_90_rrrruuuugggg_sample = rug_fuzz_0;
                let rug_fuzz_0 = rug_fuzz_1;
                let mut v36 = StateMap::new(rug_fuzz_0);
                debug_assert_eq!(v36.is_empty(), true);
                let _rug_ed_tests_rug_90_rrrruuuugggg_sample = rug_fuzz_2;
                let _rug_ed_tests_rug_90_prepare_rrrruuuugggg_sample = 0;
            }
        }
        let mut p0 = StateMap::new(10);
        assert_eq!(p0.is_empty(), true);
        let _rug_ed_tests_rug_90_rrrruuuugggg_sample = 0;
    }
}
#[cfg(test)]
mod tests_rug_91 {
    use super::*;
    use crate::dfa::{StateMap, State};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_91_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 0;
        let mut p0 = StateMap::new(rug_fuzz_0);
        let v29 = State {
            data: Arc::from(vec![rug_fuzz_1, 1, 2, 3, 4, 5]),
        };
        let p1 = &v29;
        crate::dfa::StateMap::get_ptr(&p0, p1);
        let _rug_ed_tests_rug_91_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_92 {
    use super::*;
    use crate::dfa::StateMap;
    #[test]
    fn test_get_state() {
        let _rug_st_tests_rug_92_rrrruuuugggg_test_get_state = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 0;
        let mut p0 = StateMap::new(rug_fuzz_0);
        let p1: u32 = rug_fuzz_1;
        p0.get_state(p1);
        let _rug_ed_tests_rug_92_rrrruuuugggg_test_get_state = 0;
    }
}
#[cfg(test)]
mod tests_rug_93 {
    use super::*;
    use crate::dfa::StateMap;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_93_rrrruuuugggg_sample = 0;
        let rug_fuzz_0 = 10;
        #[cfg(test)]
        mod tests_rug_93_prepare {
            use crate::dfa::StateMap;
            #[test]
            fn sample() {
                let _rug_st_tests_rug_93_prepare_rrrruuuugggg_sample = 0;
                let rug_fuzz_0 = 0;
                let rug_fuzz_1 = 10;
                let rug_fuzz_2 = 0;
                let _rug_st_tests_rug_93_rrrruuuugggg_sample = rug_fuzz_0;
                let rug_fuzz_0 = rug_fuzz_1;
                let mut v36 = StateMap::new(rug_fuzz_0);
                v36.clear();
                debug_assert_eq!(v36.map.len(), 0);
                debug_assert_eq!(v36.states.len(), 0);
                let _rug_ed_tests_rug_93_rrrruuuugggg_sample = rug_fuzz_2;
                let _rug_ed_tests_rug_93_prepare_rrrruuuugggg_sample = 0;
            }
        }
        let mut p0 = StateMap::new(10);
        p0.clear();
        assert_eq!(p0.map.len(), 0);
        assert_eq!(p0.states.len(), 0);
        let _rug_ed_tests_rug_93_rrrruuuugggg_sample = 0;
    }
}
#[cfg(test)]
mod tests_rug_94 {
    use super::*;
    use crate::dfa::Byte;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_94_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 65;
        let p0: u8 = rug_fuzz_0;
        Byte::byte(p0);
        let _rug_ed_tests_rug_94_rrrruuuugggg_test_rug = 0;
    }
}
