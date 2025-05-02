use crate::InternalString;
/// Opaque string storage for raw TOML; internal to `toml_edit`
#[derive(PartialEq, Eq, Clone, Hash)]
pub struct RawString(RawStringInner);
#[derive(PartialEq, Eq, Clone, Hash)]
enum RawStringInner {
    Empty,
    Explicit(InternalString),
    Spanned(std::ops::Range<usize>),
}
impl RawString {
    pub(crate) fn with_span(span: std::ops::Range<usize>) -> Self {
        if span.start == span.end {
            RawString(RawStringInner::Empty)
        } else {
            RawString(RawStringInner::Spanned(span))
        }
    }
    /// Access the underlying string
    pub fn as_str(&self) -> Option<&str> {
        match &self.0 {
            RawStringInner::Empty => Some(""),
            RawStringInner::Explicit(s) => Some(s.as_str()),
            RawStringInner::Spanned(_) => None,
        }
    }
    pub(crate) fn to_str<'s>(&'s self, input: &'s str) -> &'s str {
        match &self.0 {
            RawStringInner::Empty => "",
            RawStringInner::Explicit(s) => s.as_str(),
            RawStringInner::Spanned(span) => {
                input
                    .get(span.clone())
                    .unwrap_or_else(|| {
                        panic!(
                            "span {:?} should be in input:\n```\n{}\n```", span, input
                        )
                    })
            }
        }
    }
    pub(crate) fn to_str_with_default<'s>(
        &'s self,
        input: Option<&'s str>,
        default: &'s str,
    ) -> &'s str {
        match &self.0 {
            RawStringInner::Empty => "",
            RawStringInner::Explicit(s) => s.as_str(),
            RawStringInner::Spanned(span) => {
                if let Some(input) = input {
                    input
                        .get(span.clone())
                        .unwrap_or_else(|| {
                            panic!(
                                "span {:?} should be in input:\n```\n{}\n```", span, input
                            )
                        })
                } else {
                    default
                }
            }
        }
    }
    /// Access the underlying span
    pub(crate) fn span(&self) -> Option<std::ops::Range<usize>> {
        match &self.0 {
            RawStringInner::Empty => None,
            RawStringInner::Explicit(_) => None,
            RawStringInner::Spanned(span) => Some(span.clone()),
        }
    }
    pub(crate) fn despan(&mut self, input: &str) {
        match &self.0 {
            RawStringInner::Empty => {}
            RawStringInner::Explicit(_) => {}
            RawStringInner::Spanned(span) => {
                *self = Self::from(
                    input
                        .get(span.clone())
                        .unwrap_or_else(|| {
                            panic!(
                                "span {:?} should be in input:\n```\n{}\n```", span, input
                            )
                        }),
                );
            }
        }
    }
    pub(crate) fn encode(
        &self,
        buf: &mut dyn std::fmt::Write,
        input: &str,
    ) -> std::fmt::Result {
        let raw = self.to_str(input);
        for part in raw.split('\r') {
            write!(buf, "{}", part)?;
        }
        Ok(())
    }
    pub(crate) fn encode_with_default(
        &self,
        buf: &mut dyn std::fmt::Write,
        input: Option<&str>,
        default: &str,
    ) -> std::fmt::Result {
        let raw = self.to_str_with_default(input, default);
        for part in raw.split('\r') {
            write!(buf, "{}", part)?;
        }
        Ok(())
    }
}
impl Default for RawString {
    fn default() -> Self {
        Self(RawStringInner::Empty)
    }
}
impl std::fmt::Debug for RawString {
    #[inline]
    fn fmt(
        &self,
        formatter: &mut std::fmt::Formatter<'_>,
    ) -> Result<(), std::fmt::Error> {
        match &self.0 {
            RawStringInner::Empty => write!(formatter, "empty"),
            RawStringInner::Explicit(s) => write!(formatter, "{:?}", s),
            RawStringInner::Spanned(s) => write!(formatter, "{:?}", s),
        }
    }
}
impl From<&str> for RawString {
    #[inline]
    fn from(s: &str) -> Self {
        if s.is_empty() {
            Self(RawStringInner::Empty)
        } else {
            InternalString::from(s).into()
        }
    }
}
impl From<String> for RawString {
    #[inline]
    fn from(s: String) -> Self {
        if s.is_empty() {
            Self(RawStringInner::Empty)
        } else {
            InternalString::from(s).into()
        }
    }
}
impl From<&String> for RawString {
    #[inline]
    fn from(s: &String) -> Self {
        if s.is_empty() {
            Self(RawStringInner::Empty)
        } else {
            InternalString::from(s).into()
        }
    }
}
impl From<InternalString> for RawString {
    #[inline]
    fn from(inner: InternalString) -> Self {
        Self(RawStringInner::Explicit(inner))
    }
}
impl From<&InternalString> for RawString {
    #[inline]
    fn from(s: &InternalString) -> Self {
        if s.is_empty() {
            Self(RawStringInner::Empty)
        } else {
            InternalString::from(s).into()
        }
    }
}
impl From<Box<str>> for RawString {
    #[inline]
    fn from(s: Box<str>) -> Self {
        if s.is_empty() {
            Self(RawStringInner::Empty)
        } else {
            InternalString::from(s).into()
        }
    }
}
#[cfg(test)]
mod tests_rug_939 {
    use super::*;
    use std::ops::Range;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_939_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 5;
        let mut p0: Range<usize> = rug_fuzz_0..rug_fuzz_1;
        crate::raw_string::RawString::with_span(p0);
        let _rug_ed_tests_rug_939_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_940 {
    use super::*;
    use crate::raw_string::RawString;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_940_rrrruuuugggg_test_rug = 0;
        let p0 = RawString::default();
        RawString::as_str(&p0);
        let _rug_ed_tests_rug_940_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_941 {
    use super::*;
    use crate::raw_string::{RawString, RawStringInner};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_941_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "Some sample data";
        let p0 = RawString(RawStringInner::Empty);
        let p1 = rug_fuzz_0;
        RawString::to_str(&p0, p1);
        let _rug_ed_tests_rug_941_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_942 {
    use super::*;
    use crate::raw_string::RawString;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_942_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = "";
        let mut p0 = RawString::default();
        let mut p1: Option<&str> = Some(rug_fuzz_0);
        let mut p2 = rug_fuzz_1;
        RawString::to_str_with_default(&p0, p1, &p2);
        let _rug_ed_tests_rug_942_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_943 {
    use super::*;
    use crate::parser::inline_table;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_943_rrrruuuugggg_test_rug = 0;
        let mut p0: RawString = RawString::default();
        RawString::span(&p0);
        let _rug_ed_tests_rug_943_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_944 {
    use super::*;
    use crate::raw_string::RawString;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_944_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample input";
        let mut p0 = RawString::default();
        let p1 = rug_fuzz_0;
        RawString::despan(&mut p0, p1);
        let _rug_ed_tests_rug_944_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_945 {
    use super::*;
    use crate::raw_string::RawString;
    use std::fmt::Write;
    #[test]
    fn test_raw_string_encode() {
        let _rug_st_tests_rug_945_rrrruuuugggg_test_raw_string_encode = 0;
        let rug_fuzz_0 = "Sample input string";
        let mut buf = String::new();
        let input = rug_fuzz_0;
        let raw_string = RawString::default();
        raw_string.encode(&mut buf, input).unwrap();
        debug_assert_eq!(buf, "Sample input string");
        let _rug_ed_tests_rug_945_rrrruuuugggg_test_raw_string_encode = 0;
    }
}
#[cfg(test)]
mod tests_rug_946 {
    use super::*;
    use crate::raw_string::RawString;
    use std::fmt::Write;
    #[test]
    fn test_encode_with_default() {
        let _rug_st_tests_rug_946_rrrruuuugggg_test_encode_with_default = 0;
        let rug_fuzz_0 = "input";
        let rug_fuzz_1 = "default";
        let mut p0 = RawString::default();
        let mut p1 = String::new();
        let mut p2 = Some(rug_fuzz_0);
        let mut p3 = rug_fuzz_1;
        crate::raw_string::RawString::encode_with_default(
                &mut p0,
                &mut p1,
                p2.as_ref().map(|s| s.as_ref()),
                &p3,
            )
            .unwrap();
        let _rug_ed_tests_rug_946_rrrruuuugggg_test_encode_with_default = 0;
    }
}
#[cfg(test)]
mod tests_rug_947 {
    use super::*;
    use crate::raw_string::RawString;
    use std::default::Default;
    #[test]
    fn test_raw_string_default() {
        let _rug_st_tests_rug_947_rrrruuuugggg_test_raw_string_default = 0;
        let raw_string: RawString = RawString::default();
        debug_assert_eq!(raw_string, RawString(RawStringInner::Empty));
        let _rug_ed_tests_rug_947_rrrruuuugggg_test_raw_string_default = 0;
    }
}
