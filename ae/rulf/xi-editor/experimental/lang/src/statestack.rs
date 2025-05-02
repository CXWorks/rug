use std::{collections::HashMap, hash::Hash};
/// An entire state stack is represented as a single integer.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct State(usize);
struct Entry<T> {
    tos: T,
    prev: State,
}
/// All states are interpreted in a context.
pub struct Context<T> {
    entries: Vec<Entry<T>>,
    next: HashMap<(State, T), State>,
}
impl<T: Clone + Hash + Eq> Context<T> {
    pub fn new() -> Context<T> {
        Context {
            entries: Vec::new(),
            next: HashMap::new(),
        }
    }
    fn entry(&self, s: State) -> Option<&Entry<T>> {
        if s.0 == 0 { None } else { Some(&self.entries[s.0 - 1]) }
    }
    /// The top of the stack for the given state.
    pub fn tos(&self, s: State) -> Option<T> {
        self.entry(s).map(|entry| entry.tos.clone())
    }
    pub fn pop(&self, s: State) -> Option<State> {
        self.entry(s).map(|entry| entry.prev)
    }
    pub fn push(&mut self, s: State, el: T) -> State {
        let entries = &mut self.entries;
        *self
            .next
            .entry((s, el.clone()))
            .or_insert_with(|| {
                entries.push(Entry { tos: el, prev: s });
                State(entries.len())
            })
    }
}
#[cfg(test)]
mod tests_llm_16_123 {
    use super::*;
    use crate::*;
    #[test]
    fn test_new_context() {
        let _rug_st_tests_llm_16_123_rrrruuuugggg_test_new_context = 0;
        let context: Context<i32> = Context::new();
        debug_assert_eq!(context.entries.len(), 0);
        debug_assert_eq!(context.next.len(), 0);
        let _rug_ed_tests_llm_16_123_rrrruuuugggg_test_new_context = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_126 {
    use super::*;
    use crate::*;
    #[derive(Clone, Eq, PartialEq, Debug, Hash, Default)]
    struct Dummy {}
    #[test]
    fn test_push() {
        let _rug_st_tests_llm_16_126_rrrruuuugggg_test_push = 0;
        let rug_fuzz_0 = 1;
        let mut context: Context<Dummy> = Context::new();
        let s = State(rug_fuzz_0);
        let el = Dummy {};
        let result = context.push(s, el);
        debug_assert_eq!(result, State(1));
        let _rug_ed_tests_llm_16_126_rrrruuuugggg_test_push = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_127 {
    use super::*;
    use crate::*;
    #[test]
    fn test_tos() {
        let _rug_st_tests_llm_16_127_rrrruuuugggg_test_tos = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 10;
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 20;
        let rug_fuzz_4 = 1;
        let rug_fuzz_5 = 30;
        let rug_fuzz_6 = 1;
        let rug_fuzz_7 = 2;
        let rug_fuzz_8 = 3;
        let rug_fuzz_9 = 2;
        let rug_fuzz_10 = 40;
        let rug_fuzz_11 = 2;
        let mut context: Context<usize> = Context::new();
        let state1 = context.push(State(rug_fuzz_0), rug_fuzz_1);
        let state2 = context.push(State(rug_fuzz_2), rug_fuzz_3);
        let state3 = context.push(State(rug_fuzz_4), rug_fuzz_5);
        debug_assert_eq!(context.tos(State(rug_fuzz_6)), Some(30));
        debug_assert_eq!(context.tos(State(rug_fuzz_7)), Some(20));
        debug_assert_eq!(context.tos(State(rug_fuzz_8)), None);
        context.push(State(rug_fuzz_9), rug_fuzz_10);
        debug_assert_eq!(context.tos(State(rug_fuzz_11)), Some(40));
        let _rug_ed_tests_llm_16_127_rrrruuuugggg_test_tos = 0;
    }
}
