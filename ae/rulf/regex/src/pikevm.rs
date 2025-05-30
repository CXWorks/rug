use std::mem;
use exec::ProgramCache;
use input::{Input, InputAt};
use prog::{InstPtr, Program};
use re_trait::Slot;
use sparse::SparseSet;
/// An NFA simulation matching engine.
#[derive(Debug)]
pub struct Fsm<'r, I> {
    /// The sequence of opcodes (among other things) that is actually executed.
    ///
    /// The program may be byte oriented or Unicode codepoint oriented.
    prog: &'r Program,
    /// An explicit stack used for following epsilon transitions. (This is
    /// borrowed from the cache.)
    stack: &'r mut Vec<FollowEpsilon>,
    /// The input to search.
    input: I,
}
/// A cached allocation that can be reused on each execution.
#[derive(Clone, Debug)]
pub struct Cache {
    /// A pair of ordered sets for tracking NFA states.
    clist: Threads,
    nlist: Threads,
    /// An explicit stack used for following epsilon transitions.
    stack: Vec<FollowEpsilon>,
}
/// An ordered set of NFA states and their captures.
#[derive(Clone, Debug)]
struct Threads {
    /// An ordered set of opcodes (each opcode is an NFA state).
    set: SparseSet,
    /// Captures for every NFA state.
    ///
    /// It is stored in row-major order, where the columns are the capture
    /// slots and the rows are the states.
    caps: Vec<Slot>,
    /// The number of capture slots stored per thread. (Every capture has
    /// two slots.)
    slots_per_thread: usize,
}
/// A representation of an explicit stack frame when following epsilon
/// transitions. This is used to avoid recursion.
#[derive(Clone, Debug)]
enum FollowEpsilon {
    /// Follow transitions at the given instruction pointer.
    IP(InstPtr),
    /// Restore the capture slot with the given position in the input.
    Capture { slot: usize, pos: Slot },
}
impl Cache {
    /// Create a new allocation used by the NFA machine to record execution
    /// and captures.
    pub fn new(_prog: &Program) -> Self {
        Cache {
            clist: Threads::new(),
            nlist: Threads::new(),
            stack: vec![],
        }
    }
}
impl<'r, I: Input> Fsm<'r, I> {
    /// Execute the NFA matching engine.
    ///
    /// If there's a match, `exec` returns `true` and populates the given
    /// captures accordingly.
    pub fn exec(
        prog: &'r Program,
        cache: &ProgramCache,
        matches: &mut [bool],
        slots: &mut [Slot],
        quit_after_match: bool,
        input: I,
        start: usize,
        end: usize,
    ) -> bool {
        let mut cache = cache.borrow_mut();
        let cache = &mut cache.pikevm;
        cache.clist.resize(prog.len(), prog.captures.len());
        cache.nlist.resize(prog.len(), prog.captures.len());
        let at = input.at(start);
        Fsm {
            prog: prog,
            stack: &mut cache.stack,
            input: input,
        }
            .exec_(
                &mut cache.clist,
                &mut cache.nlist,
                matches,
                slots,
                quit_after_match,
                at,
                end,
            )
    }
    fn exec_(
        &mut self,
        mut clist: &mut Threads,
        mut nlist: &mut Threads,
        matches: &mut [bool],
        slots: &mut [Slot],
        quit_after_match: bool,
        mut at: InputAt,
        end: usize,
    ) -> bool {
        let mut matched = false;
        let mut all_matched = false;
        clist.set.clear();
        nlist.set.clear();
        'LOOP: loop {
            if clist.set.is_empty() {
                if (matched && matches.len() <= 1) || all_matched
                    || (!at.is_start() && self.prog.is_anchored_start)
                {
                    break;
                }
                if !self.prog.prefixes.is_empty() {
                    at = match self.input.prefix_at(&self.prog.prefixes, at) {
                        None => break,
                        Some(at) => at,
                    };
                }
            }
            if clist.set.is_empty() || (!self.prog.is_anchored_start && !all_matched) {
                self.add(&mut clist, slots, 0, at);
            }
            let at_next = self.input.at(at.next_pos());
            for i in 0..clist.set.len() {
                let ip = clist.set[i];
                if self.step(&mut nlist, matches, slots, clist.caps(ip), ip, at, at_next)
                {
                    matched = true;
                    all_matched = all_matched || matches.iter().all(|&b| b);
                    if quit_after_match {
                        break 'LOOP;
                    }
                    if self.prog.matches.len() == 1 {
                        break;
                    }
                }
            }
            if at.pos() >= end {
                break;
            }
            at = at_next;
            mem::swap(clist, nlist);
            nlist.set.clear();
        }
        matched
    }
    /// Step through the input, one token (byte or codepoint) at a time.
    ///
    /// nlist is the set of states that will be processed on the next token
    /// in the input.
    ///
    /// caps is the set of captures passed by the caller of the NFA. They are
    /// written to only when a match state is visited.
    ///
    /// thread_caps is the set of captures set for the current NFA state, ip.
    ///
    /// at and at_next are the current and next positions in the input. at or
    /// at_next may be EOF.
    fn step(
        &mut self,
        nlist: &mut Threads,
        matches: &mut [bool],
        slots: &mut [Slot],
        thread_caps: &mut [Option<usize>],
        ip: usize,
        at: InputAt,
        at_next: InputAt,
    ) -> bool {
        use prog::Inst::*;
        match self.prog[ip] {
            Match(match_slot) => {
                if match_slot < matches.len() {
                    matches[match_slot] = true;
                }
                for (slot, val) in slots.iter_mut().zip(thread_caps.iter()) {
                    *slot = *val;
                }
                true
            }
            Char(ref inst) => {
                if inst.c == at.char() {
                    self.add(nlist, thread_caps, inst.goto, at_next);
                }
                false
            }
            Ranges(ref inst) => {
                if inst.matches(at.char()) {
                    self.add(nlist, thread_caps, inst.goto, at_next);
                }
                false
            }
            Bytes(ref inst) => {
                if let Some(b) = at.byte() {
                    if inst.matches(b) {
                        self.add(nlist, thread_caps, inst.goto, at_next);
                    }
                }
                false
            }
            EmptyLook(_) | Save(_) | Split(_) => false,
        }
    }
    /// Follows epsilon transitions and adds them for processing to nlist,
    /// starting at and including ip.
    fn add(
        &mut self,
        nlist: &mut Threads,
        thread_caps: &mut [Option<usize>],
        ip: usize,
        at: InputAt,
    ) {
        self.stack.push(FollowEpsilon::IP(ip));
        while let Some(frame) = self.stack.pop() {
            match frame {
                FollowEpsilon::IP(ip) => {
                    self.add_step(nlist, thread_caps, ip, at);
                }
                FollowEpsilon::Capture { slot, pos } => {
                    thread_caps[slot] = pos;
                }
            }
        }
    }
    /// A helper function for add that avoids excessive pushing to the stack.
    fn add_step(
        &mut self,
        nlist: &mut Threads,
        thread_caps: &mut [Option<usize>],
        mut ip: usize,
        at: InputAt,
    ) {
        use prog::Inst::*;
        loop {
            if nlist.set.contains(ip) {
                return;
            }
            nlist.set.insert(ip);
            match self.prog[ip] {
                EmptyLook(ref inst) => {
                    if self.input.is_empty_match(at, inst) {
                        ip = inst.goto;
                    }
                }
                Save(ref inst) => {
                    if inst.slot < thread_caps.len() {
                        self.stack
                            .push(FollowEpsilon::Capture {
                                slot: inst.slot,
                                pos: thread_caps[inst.slot],
                            });
                        thread_caps[inst.slot] = Some(at.pos());
                    }
                    ip = inst.goto;
                }
                Split(ref inst) => {
                    self.stack.push(FollowEpsilon::IP(inst.goto2));
                    ip = inst.goto1;
                }
                Match(_) | Char(_) | Ranges(_) | Bytes(_) => {
                    let t = &mut nlist.caps(ip);
                    for (slot, val) in t.iter_mut().zip(thread_caps.iter()) {
                        *slot = *val;
                    }
                    return;
                }
            }
        }
    }
}
impl Threads {
    fn new() -> Self {
        Threads {
            set: SparseSet::new(0),
            caps: vec![],
            slots_per_thread: 0,
        }
    }
    fn resize(&mut self, num_insts: usize, ncaps: usize) {
        if num_insts == self.set.capacity() {
            return;
        }
        self.slots_per_thread = ncaps * 2;
        self.set = SparseSet::new(num_insts);
        self.caps = vec![None; self.slots_per_thread * num_insts];
    }
    fn caps(&mut self, pc: usize) -> &mut [Option<usize>] {
        let i = pc * self.slots_per_thread;
        &mut self.caps[i..i + self.slots_per_thread]
    }
}
#[cfg(test)]
mod tests_llm_16_470 {
    use super::*;
    use crate::*;
    #[test]
    fn test_pikevm_cache_new() {
        let _rug_st_tests_llm_16_470_rrrruuuugggg_test_pikevm_cache_new = 0;
        let prog = Program::new();
        let cache = Cache::new(&prog);
        debug_assert_eq!(cache.clist.set.len(), 0);
        debug_assert_eq!(cache.nlist.set.len(), 0);
        debug_assert_eq!(cache.stack.len(), 0);
        let _rug_ed_tests_llm_16_470_rrrruuuugggg_test_pikevm_cache_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_477 {
    use super::*;
    use crate::*;
    #[test]
    fn test_caps() {
        let _rug_st_tests_llm_16_477_rrrruuuugggg_test_caps = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 3;
        let rug_fuzz_2 = 2;
        let mut threads = Threads::new();
        threads.resize(rug_fuzz_0, rug_fuzz_1);
        let caps = threads.caps(rug_fuzz_2);
        debug_assert_eq!(caps.len(), 6);
        let expected = [None, None, None, None, None, None];
        debug_assert_eq!(caps, & expected);
        let _rug_ed_tests_llm_16_477_rrrruuuugggg_test_caps = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_478 {
    use crate::pikevm;
    #[test]
    fn test_new_threads() {
        let _rug_st_tests_llm_16_478_rrrruuuugggg_test_new_threads = 0;
        let threads = pikevm::Threads::new();
        debug_assert!(threads.set.is_empty());
        debug_assert!(threads.caps.is_empty());
        debug_assert_eq!(threads.slots_per_thread, 0);
        let _rug_ed_tests_llm_16_478_rrrruuuugggg_test_new_threads = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_479 {
    use super::*;
    use crate::*;
    #[test]
    fn test_resize() {
        let _rug_st_tests_llm_16_479_rrrruuuugggg_test_resize = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 3;
        let mut threads = Threads::new();
        threads.resize(rug_fuzz_0, rug_fuzz_1);
        debug_assert_eq!(threads.set.capacity(), 5);
        debug_assert_eq!(threads.slots_per_thread, 6);
        debug_assert_eq!(threads.caps.len(), 5 * 6);
        let _rug_ed_tests_llm_16_479_rrrruuuugggg_test_resize = 0;
    }
}
