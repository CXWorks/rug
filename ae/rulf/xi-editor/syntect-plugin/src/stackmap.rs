//! StackMap is a simple nested map type (a trie) used to map `StackScope`s to
//! u32s so they can be efficiently sent to xi-core.
//!
//! For discussion of this approach, see [this
//! issue](https://github.com/google/xi-editor/issues/284).
use std::collections::HashMap;
use syntect::parsing::Scope;
#[derive(Debug, Default)]
struct Node {
    value: Option<u32>,
    children: HashMap<Scope, Node>,
}
#[derive(Debug, Default)]
/// Nested lookup table for stacks of scopes.
pub struct StackMap {
    next_id: u32,
    scopes: Node,
}
#[derive(Debug, PartialEq)]
/// Result type for `StackMap` lookups. Used to communicate to the user
/// whether or not a new identifier has been assigned, which will need to
/// be communicated to the peer.
pub enum LookupResult {
    Existing(u32),
    New(u32),
}
impl Node {
    pub fn new(value: u32) -> Self {
        Node {
            value: Some(value),
            children: HashMap::new(),
        }
    }
    fn get_value(&mut self, stack: &[Scope], next_id: u32) -> LookupResult {
        let first = stack.first().unwrap();
        if stack.len() == 1 {
            if !self.children.contains_key(first) {
                self.children.insert(first.to_owned(), Node::new(next_id));
                return LookupResult::New(next_id);
            }
            let needs_value = self.children[first].value.is_none();
            if needs_value {
                let node = self.children.get_mut(first).unwrap();
                node.value = Some(next_id);
                return LookupResult::New(next_id);
            } else {
                let value = self.children[first].value.unwrap();
                return LookupResult::Existing(value);
            }
        }
        if self.children.get(first).is_none() {
            self.children.insert(first.to_owned(), Node::default());
        }
        self.children.get_mut(first).unwrap().get_value(&stack[1..], next_id)
    }
}
impl StackMap {
    /// Returns the identifier for this stack, creating it if needed.
    pub fn get_value(&mut self, stack: &[Scope]) -> LookupResult {
        assert!(! stack.is_empty());
        let result = self.scopes.get_value(stack, self.next_id);
        if result.is_new() {
            self.next_id += 1;
        }
        result
    }
}
impl LookupResult {
    pub fn is_new(&self) -> bool {
        matches!(* self, LookupResult::New(_))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use syntect::parsing::ScopeStack;
    #[test]
    fn test_get_value() {
        let mut stackmap = StackMap::default();
        let stack = ScopeStack::from_str("text.rust.test scope.level.three").unwrap();
        assert_eq!(stack.as_slice().len(), 2);
        assert_eq!(stackmap.get_value(stack.as_slice()), LookupResult::New(0));
        assert_eq!(stackmap.get_value(stack.as_slice()), LookupResult::Existing(0));
        let stack2 = ScopeStack::from_str("text.rust.test").unwrap();
        assert_eq!(stackmap.get_value(stack2.as_slice()), LookupResult::New(1));
        assert_eq!(stack2.as_slice().len(), 1);
    }
}
#[cfg(test)]
mod tests_llm_16_68 {
    use crate::stackmap::LookupResult;
    #[test]
    fn test_is_new_existing() {
        let _rug_st_tests_llm_16_68_rrrruuuugggg_test_is_new_existing = 0;
        let rug_fuzz_0 = 123;
        let result = LookupResult::Existing(rug_fuzz_0);
        debug_assert!(! result.is_new());
        let _rug_ed_tests_llm_16_68_rrrruuuugggg_test_is_new_existing = 0;
    }
    #[test]
    fn test_is_new_new() {
        let _rug_st_tests_llm_16_68_rrrruuuugggg_test_is_new_new = 0;
        let rug_fuzz_0 = 456;
        let result = LookupResult::New(rug_fuzz_0);
        debug_assert!(result.is_new());
        let _rug_ed_tests_llm_16_68_rrrruuuugggg_test_is_new_new = 0;
    }
    #[test]
    fn test_is_new_nested_new() {
        let _rug_st_tests_llm_16_68_rrrruuuugggg_test_is_new_nested_new = 0;
        let rug_fuzz_0 = 789;
        let rug_fuzz_1 = 789;
        let result = LookupResult::New(rug_fuzz_0);
        let nested_result = LookupResult::New(rug_fuzz_1);
        debug_assert!(result.is_new());
        debug_assert!(nested_result.is_new());
        let _rug_ed_tests_llm_16_68_rrrruuuugggg_test_is_new_nested_new = 0;
    }
    #[test]
    fn test_is_new_nested_existing() {
        let _rug_st_tests_llm_16_68_rrrruuuugggg_test_is_new_nested_existing = 0;
        let rug_fuzz_0 = 789;
        let nested_result = LookupResult::Existing(rug_fuzz_0);
        debug_assert!(! nested_result.is_new());
        let _rug_ed_tests_llm_16_68_rrrruuuugggg_test_is_new_nested_existing = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_71 {
    use super::*;
    use crate::*;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_71_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = 42;
        let node = Node::new(rug_fuzz_0);
        debug_assert_eq!(node.value, Some(42));
        debug_assert_eq!(node.children.len(), 0);
        let _rug_ed_tests_llm_16_71_rrrruuuugggg_test_new = 0;
    }
}
