/// Searches for the given needle in the given haystack.
///
/// If the perf-literal feature is enabled, then this uses the super optimized
/// memchr crate. Otherwise, it uses the naive byte-at-a-time implementation.
pub fn find_byte(needle: u8, haystack: &[u8]) -> Option<usize> {
    #[cfg(not(feature = "perf-literal"))]
    fn imp(needle: u8, haystack: &[u8]) -> Option<usize> {
        haystack.iter().position(|&b| b == needle)
    }
    #[cfg(feature = "perf-literal")]
    fn imp(needle: u8, haystack: &[u8]) -> Option<usize> {
        use memchr::memchr;
        memchr(needle, haystack)
    }
    imp(needle, haystack)
}
#[cfg(test)]
mod tests_rug_149 {
    use super::*;
    #[test]
    fn test_find_byte() {
        let _rug_st_tests_rug_149_rrrruuuugggg_test_find_byte = 0;
        let rug_fuzz_0 = 65;
        let rug_fuzz_1 = b"ABC";
        let p0: u8 = rug_fuzz_0;
        let p1: &[u8] = rug_fuzz_1;
        find_byte(p0, p1);
        let _rug_ed_tests_rug_149_rrrruuuugggg_test_find_byte = 0;
    }
}
