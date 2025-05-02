use regex_automata::DFA;
use unicode::fsm::whitespace_anchored_fwd::WHITESPACE_ANCHORED_FWD;
use unicode::fsm::whitespace_anchored_rev::WHITESPACE_ANCHORED_REV;
/// Return the first position of a non-whitespace character.
pub fn whitespace_len_fwd(slice: &[u8]) -> usize {
    WHITESPACE_ANCHORED_FWD.find(slice).unwrap_or(0)
}
/// Return the last position of a non-whitespace character.
pub fn whitespace_len_rev(slice: &[u8]) -> usize {
    WHITESPACE_ANCHORED_REV.rfind(slice).unwrap_or(slice.len())
}
#[cfg(test)]
mod tests_rug_332 {
    use super::*;
    use crate::unicode::whitespace::whitespace_len_fwd;
    use crate::unicode::whitespace::WHITESPACE_ANCHORED_FWD;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_332_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"  \tHello, World!";
        let mut p0: &[u8] = rug_fuzz_0;
        whitespace_len_fwd(p0);
        let _rug_ed_tests_rug_332_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_333 {
    use super::*;
    #[test]
    fn test_whitespace_len_rev() {
        let _rug_st_tests_rug_333_rrrruuuugggg_test_whitespace_len_rev = 0;
        let rug_fuzz_0 = b" \t\nHello World!";
        let slice: &[u8] = rug_fuzz_0;
        debug_assert_eq!(crate ::unicode::whitespace::whitespace_len_rev(slice), 13);
        let _rug_ed_tests_rug_333_rrrruuuugggg_test_whitespace_len_rev = 0;
    }
}
