//! The `Format` struct and its implementations.
#[cfg(not(feature = "std"))]
use alloc::{borrow::ToOwned, string::String};
/// Various well-known formats, along with the possibility for a custom format
/// (provided either at compile-time or runtime).
#[allow(clippy::missing_docs_in_private_items)]
#[cfg_attr(__time_02_supports_non_exhaustive, non_exhaustive)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Format {
    #[cfg_attr(__time_02_docs, doc(alias = "ISO8601"))]
    Rfc3339,
    Custom(String),
    #[cfg(not(__time_02_supports_non_exhaustive))]
    #[doc(hidden)]
    __NonExhaustive,
}
impl<T: AsRef<str>> From<T> for Format {
    fn from(s: T) -> Self {
        Format::Custom(s.as_ref().to_owned())
    }
}
#[cfg(test)]
mod tests_llm_16_174 {
    use std::convert::From;
    use crate::format::format::Format;
    #[test]
    fn test_from() {
        let _rug_st_tests_llm_16_174_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = "custom_format";
        let s = rug_fuzz_0;
        let format = Format::from(s);
        debug_assert_eq!(format, Format::Custom(s.to_string()));
        let _rug_ed_tests_llm_16_174_rrrruuuugggg_test_from = 0;
    }
}
