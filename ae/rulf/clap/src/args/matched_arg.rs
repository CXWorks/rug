use std::ffi::OsString;
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct MatchedArg {
    #[doc(hidden)]
    pub occurs: u64,
    #[doc(hidden)]
    pub indices: Vec<usize>,
    #[doc(hidden)]
    pub vals: Vec<OsString>,
}
impl Default for MatchedArg {
    fn default() -> Self {
        MatchedArg {
            occurs: 1,
            indices: Vec::new(),
            vals: Vec::new(),
        }
    }
}
impl MatchedArg {
    pub fn new() -> Self {
        MatchedArg::default()
    }
}
#[cfg(test)]
mod tests_llm_16_176 {
    use super::*;
    use crate::*;
    use std::ffi::OsString;
    #[test]
    fn test_default() {
        let _rug_st_tests_llm_16_176_rrrruuuugggg_test_default = 0;
        let rug_fuzz_0 = 1;
        let default_matched_arg: MatchedArg = MatchedArg {
            occurs: rug_fuzz_0,
            indices: Vec::new(),
            vals: Vec::new(),
        };
        let tested_matched_arg: MatchedArg = MatchedArg::default();
        debug_assert_eq!(default_matched_arg.occurs, tested_matched_arg.occurs);
        debug_assert_eq!(default_matched_arg.indices, tested_matched_arg.indices);
        debug_assert_eq!(default_matched_arg.vals, tested_matched_arg.vals);
        let _rug_ed_tests_llm_16_176_rrrruuuugggg_test_default = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_247 {
    use std::ffi::OsString;
    use crate::args::matched_arg::MatchedArg;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_247_rrrruuuugggg_test_new = 0;
        let matched_arg: MatchedArg = MatchedArg::new();
        debug_assert_eq!(matched_arg.occurs, 1);
        debug_assert_eq!(matched_arg.indices, Vec:: < usize > ::new());
        debug_assert_eq!(matched_arg.vals, Vec:: < OsString > ::new());
        let _rug_ed_tests_llm_16_247_rrrruuuugggg_test_new = 0;
    }
}
