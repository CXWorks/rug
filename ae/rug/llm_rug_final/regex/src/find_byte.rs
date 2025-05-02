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
mod tests_rug_210 {
    use super::find_byte;

    #[test]
    fn test_find_byte() {
        let p0: u8 = 42;
        let p1: &[u8] = &[1, 2, 3, 42, 5];
        
        assert_eq!(find_byte(p0, p1), Some(3));
    }
}