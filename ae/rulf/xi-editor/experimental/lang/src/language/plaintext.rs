use crate::parser::Parser;
use crate::statestack::{Context, State};
use crate::ScopeId;
const PLAINTEXT_SOURCE_SCOPE: &[&str] = &["source.plaintext"];
pub struct PlaintextParser {
    scope_offset: Option<u32>,
    ctx: Context<()>,
}
impl PlaintextParser {
    pub fn new() -> PlaintextParser {
        PlaintextParser {
            scope_offset: None,
            ctx: Context::new(),
        }
    }
}
impl Parser for PlaintextParser {
    fn has_offset(&mut self) -> bool {
        self.scope_offset.is_some()
    }
    fn set_scope_offset(&mut self, offset: u32) {
        if !self.has_offset() {
            self.scope_offset = Some(offset);
        }
    }
    fn get_all_scopes(&self) -> Vec<Vec<String>> {
        vec![PLAINTEXT_SOURCE_SCOPE.iter().map(| it | (* it).to_string()).collect()]
    }
    fn get_scope_id_for_state(&self, _state: State) -> ScopeId {
        self.scope_offset.unwrap_or_default()
    }
    fn parse(&mut self, text: &str, state: State) -> (usize, State, usize, State) {
        (0, self.ctx.push(state, ()), text.as_bytes().len(), state)
    }
}
#[cfg(test)]
mod tests_llm_16_29 {
    use super::*;
    use crate::*;
    use crate::language::plaintext::PlaintextParser;
    use crate::parser::Parser;
    use crate::statestack::State;
    const PLAINTEXT_SOURCE_SCOPE: &[&str] = &["source.plaintext"];
    #[test]
    fn test_get_all_scopes() {
        let _rug_st_tests_llm_16_29_rrrruuuugggg_test_get_all_scopes = 0;
        let parser = PlaintextParser::new();
        let result = parser.get_all_scopes();
        let expected: Vec<Vec<String>> = vec![
            PLAINTEXT_SOURCE_SCOPE.iter().map(| & it | it.to_string()).collect()
        ];
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_29_rrrruuuugggg_test_get_all_scopes = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_33 {
    use super::*;
    use crate::*;
    use crate::language::plaintext::PlaintextParser;
    use crate::parser::Parser;
    #[test]
    fn test_has_offset() {
        let _rug_st_tests_llm_16_33_rrrruuuugggg_test_has_offset = 0;
        let rug_fuzz_0 = 10;
        let mut parser: PlaintextParser = PlaintextParser::new();
        debug_assert_eq!(parser.has_offset(), false);
        parser.set_scope_offset(rug_fuzz_0);
        debug_assert_eq!(parser.has_offset(), true);
        let _rug_ed_tests_llm_16_33_rrrruuuugggg_test_has_offset = 0;
    }
}
