use exec::ProgramCache;
use input::{Input, InputAt};
use prog::{InstPtr, Program};
use re_trait::Slot;
type Bits = u32;
const BIT_SIZE: usize = 32;
const MAX_SIZE_BYTES: usize = 256 * (1 << 10);
/// Returns true iff the given regex and input should be executed by this
/// engine with reasonable memory usage.
pub fn should_exec(num_insts: usize, text_len: usize) -> bool {
    let size = ((num_insts * (text_len + 1) + BIT_SIZE - 1) / BIT_SIZE) * 4;
    size <= MAX_SIZE_BYTES
}
/// A backtracking matching engine.
#[derive(Debug)]
pub struct Bounded<'a, 'm, 'r, 's, I> {
    prog: &'r Program,
    input: I,
    matches: &'m mut [bool],
    slots: &'s mut [Slot],
    m: &'a mut Cache,
}
/// Shared cached state between multiple invocations of a backtracking engine
/// in the same thread.
#[derive(Clone, Debug)]
pub struct Cache {
    jobs: Vec<Job>,
    visited: Vec<Bits>,
}
impl Cache {
    /// Create new empty cache for the backtracking engine.
    pub fn new(_prog: &Program) -> Self {
        Cache {
            jobs: vec![],
            visited: vec![],
        }
    }
}
/// A job is an explicit unit of stack space in the backtracking engine.
///
/// The "normal" representation is a single state transition, which corresponds
/// to an NFA state and a character in the input. However, the backtracking
/// engine must keep track of old capture group values. We use the explicit
/// stack to do it.
#[derive(Clone, Copy, Debug)]
enum Job {
    Inst { ip: InstPtr, at: InputAt },
    SaveRestore { slot: usize, old_pos: Option<usize> },
}
impl<'a, 'm, 'r, 's, I: Input> Bounded<'a, 'm, 'r, 's, I> {
    /// Execute the backtracking matching engine.
    ///
    /// If there's a match, `exec` returns `true` and populates the given
    /// captures accordingly.
    pub fn exec(
        prog: &'r Program,
        cache: &ProgramCache,
        matches: &'m mut [bool],
        slots: &'s mut [Slot],
        input: I,
        start: usize,
        end: usize,
    ) -> bool {
        let mut cache = cache.borrow_mut();
        let cache = &mut cache.backtrack;
        let start = input.at(start);
        let mut b = Bounded {
            prog: prog,
            input: input,
            matches: matches,
            slots: slots,
            m: cache,
        };
        b.exec_(start, end)
    }
    /// Clears the cache such that the backtracking engine can be executed
    /// on some input of fixed length.
    fn clear(&mut self) {
        self.m.jobs.clear();
        let visited_len = (self.prog.len() * (self.input.len() + 1) + BIT_SIZE - 1)
            / BIT_SIZE;
        self.m.visited.truncate(visited_len);
        for v in &mut self.m.visited {
            *v = 0;
        }
        if visited_len > self.m.visited.len() {
            let len = self.m.visited.len();
            self.m.visited.reserve_exact(visited_len - len);
            for _ in 0..(visited_len - len) {
                self.m.visited.push(0);
            }
        }
    }
    /// Start backtracking at the given position in the input, but also look
    /// for literal prefixes.
    fn exec_(&mut self, mut at: InputAt, end: usize) -> bool {
        self.clear();
        if self.prog.is_anchored_start {
            return if !at.is_start() { false } else { self.backtrack(at) };
        }
        let mut matched = false;
        loop {
            if !self.prog.prefixes.is_empty() {
                at = match self.input.prefix_at(&self.prog.prefixes, at) {
                    None => break,
                    Some(at) => at,
                };
            }
            matched = self.backtrack(at) || matched;
            if matched && self.prog.matches.len() == 1 {
                return true;
            }
            if at.pos() >= end {
                break;
            }
            at = self.input.at(at.next_pos());
        }
        matched
    }
    /// The main backtracking loop starting at the given input position.
    fn backtrack(&mut self, start: InputAt) -> bool {
        let mut matched = false;
        self.m.jobs.push(Job::Inst { ip: 0, at: start });
        while let Some(job) = self.m.jobs.pop() {
            match job {
                Job::Inst { ip, at } => {
                    if self.step(ip, at) {
                        if self.prog.matches.len() == 1 {
                            return true;
                        }
                        matched = true;
                    }
                }
                Job::SaveRestore { slot, old_pos } => {
                    if slot < self.slots.len() {
                        self.slots[slot] = old_pos;
                    }
                }
            }
        }
        matched
    }
    fn step(&mut self, mut ip: InstPtr, mut at: InputAt) -> bool {
        use prog::Inst::*;
        loop {
            if self.has_visited(ip, at) {
                return false;
            }
            match self.prog[ip] {
                Match(slot) => {
                    if slot < self.matches.len() {
                        self.matches[slot] = true;
                    }
                    return true;
                }
                Save(ref inst) => {
                    if let Some(&old_pos) = self.slots.get(inst.slot) {
                        self.m
                            .jobs
                            .push(Job::SaveRestore {
                                slot: inst.slot,
                                old_pos: old_pos,
                            });
                        self.slots[inst.slot] = Some(at.pos());
                    }
                    ip = inst.goto;
                }
                Split(ref inst) => {
                    self.m
                        .jobs
                        .push(Job::Inst {
                            ip: inst.goto2,
                            at: at,
                        });
                    ip = inst.goto1;
                }
                EmptyLook(ref inst) => {
                    if self.input.is_empty_match(at, inst) {
                        ip = inst.goto;
                    } else {
                        return false;
                    }
                }
                Char(ref inst) => {
                    if inst.c == at.char() {
                        ip = inst.goto;
                        at = self.input.at(at.next_pos());
                    } else {
                        return false;
                    }
                }
                Ranges(ref inst) => {
                    if inst.matches(at.char()) {
                        ip = inst.goto;
                        at = self.input.at(at.next_pos());
                    } else {
                        return false;
                    }
                }
                Bytes(ref inst) => {
                    if let Some(b) = at.byte() {
                        if inst.matches(b) {
                            ip = inst.goto;
                            at = self.input.at(at.next_pos());
                            continue;
                        }
                    }
                    return false;
                }
            }
        }
    }
    fn has_visited(&mut self, ip: InstPtr, at: InputAt) -> bool {
        let k = ip * (self.input.len() + 1) + at.pos();
        let k1 = k / BIT_SIZE;
        let k2 = usize_to_u32(1 << (k & (BIT_SIZE - 1)));
        if self.m.visited[k1] & k2 == 0 {
            self.m.visited[k1] |= k2;
            false
        } else {
            true
        }
    }
}
fn usize_to_u32(n: usize) -> u32 {
    if (n as u64) > (::std::u32::MAX as u64) {
        panic!("BUG: {} is too big to fit into u32", n)
    }
    n as u32
}
#[cfg(test)]
mod tests_llm_16_192 {
    use crate::backtrack::should_exec;
    #[test]
    fn test_should_exec() {
        let _rug_st_tests_llm_16_192_rrrruuuugggg_test_should_exec = 0;
        let rug_fuzz_0 = 100;
        let rug_fuzz_1 = 100;
        let rug_fuzz_2 = 100;
        let rug_fuzz_3 = 1000;
        let rug_fuzz_4 = 1000;
        let rug_fuzz_5 = 100;
        let rug_fuzz_6 = 1000;
        let rug_fuzz_7 = 1000;
        let rug_fuzz_8 = 1000;
        let rug_fuzz_9 = 10000;
        debug_assert_eq!(should_exec(rug_fuzz_0, rug_fuzz_1), true);
        debug_assert_eq!(should_exec(rug_fuzz_2, rug_fuzz_3), true);
        debug_assert_eq!(should_exec(rug_fuzz_4, rug_fuzz_5), true);
        debug_assert_eq!(should_exec(rug_fuzz_6, rug_fuzz_7), true);
        debug_assert_eq!(should_exec(rug_fuzz_8, rug_fuzz_9), false);
        let _rug_ed_tests_llm_16_192_rrrruuuugggg_test_should_exec = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_193 {
    use super::*;
    use crate::*;
    #[test]
    fn test_usize_to_u32() {
        let _rug_st_tests_llm_16_193_rrrruuuugggg_test_usize_to_u32 = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 12345;
        debug_assert_eq!(usize_to_u32(rug_fuzz_0), 0);
        debug_assert_eq!(usize_to_u32(rug_fuzz_1), 1);
        debug_assert_eq!(usize_to_u32(rug_fuzz_2), 12345);
        let _rug_ed_tests_llm_16_193_rrrruuuugggg_test_usize_to_u32 = 0;
    }
    #[test]
    #[should_panic(expected = "BUG: 4294967296 is too big to fit into u32")]
    fn test_usize_to_u32_panic() {
        let _rug_st_tests_llm_16_193_rrrruuuugggg_test_usize_to_u32_panic = 0;
        let rug_fuzz_0 = 4294967296;
        usize_to_u32(rug_fuzz_0);
        let _rug_ed_tests_llm_16_193_rrrruuuugggg_test_usize_to_u32_panic = 0;
    }
}
#[cfg(test)]
mod tests_rug_1 {
    use super::*;
    use crate::internal::Program;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_1_rrrruuuugggg_test_rug = 0;
        let mut p0 = {
            let mut v1 = Program::new();
            v1
        };
        crate::backtrack::Cache::new(&p0);
        let _rug_ed_tests_rug_1_rrrruuuugggg_test_rug = 0;
    }
}
