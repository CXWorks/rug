pub trait _StrExt {
    fn _is_char_boundary(&self, index: usize) -> bool;
}
impl _StrExt for str {
    #[inline]
    fn _is_char_boundary(&self, index: usize) -> bool {
        if index == self.len() {
            return true;
        }
        match self.as_bytes().get(index) {
            None => false,
            Some(&b) => b < 128 || b >= 192,
        }
    }
}
#[cfg(test)]
mod tests_llm_16_194 {
    use crate::strext::_StrExt;
    #[test]
    fn test_is_char_boundary() {
        let _rug_st_tests_llm_16_194_rrrruuuugggg_test_is_char_boundary = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = true;
        let rug_fuzz_2 = true;
        let rug_fuzz_3 = false;
        let rug_fuzz_4 = false;
        debug_assert_eq!(rug_fuzz_0, < str as _StrExt > ::_is_char_boundary("hello", 0));
        debug_assert_eq!(rug_fuzz_1, < str as _StrExt > ::_is_char_boundary("hello", 5));
        debug_assert_eq!(rug_fuzz_2, < str as _StrExt > ::_is_char_boundary("", 0));
        debug_assert_eq!(rug_fuzz_3, < str as _StrExt > ::_is_char_boundary("hello", 6));
        debug_assert_eq!(
            rug_fuzz_4, < str as _StrExt > ::_is_char_boundary("hello", 10)
        );
        let _rug_ed_tests_llm_16_194_rrrruuuugggg_test_is_char_boundary = 0;
    }
}
