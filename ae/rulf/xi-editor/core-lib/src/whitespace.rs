//! Utilities for detecting and working with indentation.
extern crate xi_rope;
use std::collections::BTreeMap;
use xi_rope::Rope;
/// An enumeration of legal indentation types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Indentation {
    Tabs,
    Spaces(usize),
}
/// A struct representing the mixed indentation error.
#[derive(Debug)]
pub struct MixedIndentError;
impl Indentation {
    /// Parses a rope for indentation settings.
    pub fn parse(rope: &Rope) -> Result<Option<Self>, MixedIndentError> {
        let lines = rope.lines_raw(..);
        let mut tabs = false;
        let mut spaces: BTreeMap<usize, usize> = BTreeMap::new();
        for line in lines {
            match Indentation::parse_line(&line) {
                Ok(Some(Indentation::Spaces(size))) => {
                    let counter = spaces.entry(size).or_insert(0);
                    *counter += 1;
                }
                Ok(Some(Indentation::Tabs)) => tabs = true,
                Ok(None) => continue,
                Err(e) => return Err(e),
            }
        }
        match (tabs, !spaces.is_empty()) {
            (true, true) => Err(MixedIndentError),
            (true, false) => Ok(Some(Indentation::Tabs)),
            (false, true) => {
                let tab_size = extract_count(spaces);
                if tab_size > 0 {
                    Ok(Some(Indentation::Spaces(tab_size)))
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }
    /// Detects the indentation on a specific line.
    /// Parses whitespace until first occurrence of something else
    pub fn parse_line(line: &str) -> Result<Option<Self>, MixedIndentError> {
        let mut spaces = 0;
        for char in line.as_bytes() {
            match char {
                b' ' => spaces += 1,
                b'\t' if spaces > 0 => return Err(MixedIndentError),
                b'\t' => return Ok(Some(Indentation::Tabs)),
                _ => break,
            }
        }
        if spaces > 0 { Ok(Some(Indentation::Spaces(spaces))) } else { Ok(None) }
    }
}
/// Uses a heuristic to calculate the greatest common denominator of most used indentation depths.
///
/// As BTreeMaps are ordered by value, using take on the iterator ensures the indentation levels
/// most frequently used in the file are extracted.
fn extract_count(spaces: BTreeMap<usize, usize>) -> usize {
    let mut take_size = 4;
    if spaces.len() < take_size {
        take_size = spaces.len();
    }
    spaces
        .iter()
        .take(take_size)
        .fold(
            0,
            |a, (b, _)| {
                let d = gcd(a, *b);
                if d == 1 { a } else { d }
            },
        )
}
/// Simple implementation to calculate greatest common divisor, based on Euclid's algorithm
fn gcd(a: usize, b: usize) -> usize {
    if a == 0 {
        b
    } else if b == 0 || a == b {
        a
    } else {
        let mut a = a;
        let mut b = b;
        while b > 0 {
            let r = a % b;
            a = b;
            b = r;
        }
        a
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gcd_calculates_correctly() {
        assert_eq!(21, gcd(1071, 462));
        assert_eq!(6, gcd(270, 192));
    }
    #[test]
    fn line_gets_two_spaces() {
        let result = Indentation::parse_line("  ");
        let expected = Indentation::Spaces(2);
        assert_eq!(result.unwrap(), Some(expected));
    }
    #[test]
    fn line_gets_tabs() {
        let result = Indentation::parse_line("\t");
        let expected = Indentation::Tabs;
        assert_eq!(result.unwrap(), Some(expected));
    }
    #[test]
    fn line_errors_mixed_indent() {
        let result = Indentation::parse_line("  \t");
        assert!(result.is_err());
    }
    #[test]
    fn rope_gets_two_spaces() {
        let result = Indentation::parse(
            &Rope::from(
                r#"
        // This is a comment
          Testing
          Indented
            Even more indented
            # Comment
            # Comment
            # Comment
        "#,
            ),
        );
        let expected = Indentation::Spaces(2);
        assert_eq!(result.unwrap(), Some(expected));
    }
    #[test]
    fn rope_gets_four_spaces() {
        let result = Indentation::parse(
            &Rope::from(
                r#"
        fn my_fun_func(&self,
                       another_arg: usize) -> Fun {
            /* Random comment describing program behavior */
            Fun::from(another_arg)
        }
        "#,
            ),
        );
        let expected = Indentation::Spaces(4);
        assert_eq!(result.unwrap(), Some(expected));
    }
    #[test]
    fn rope_returns_none() {
        let result = Indentation::parse(
            &Rope::from(
                r#"# Readme example
 1. One space.
But the majority is still 0.
"#,
            ),
        );
        assert_eq!(result.unwrap(), None);
    }
}
#[cfg(test)]
mod tests_llm_16_799 {
    use super::*;
    use crate::*;
    use std::collections::BTreeMap;
    #[test]
    fn test_extract_count() {
        let _rug_st_tests_llm_16_799_rrrruuuugggg_test_extract_count = 0;
        let rug_fuzz_0 = 2;
        let rug_fuzz_1 = 5;
        let rug_fuzz_2 = 4;
        let rug_fuzz_3 = 10;
        let rug_fuzz_4 = 6;
        let rug_fuzz_5 = 3;
        let mut spaces: BTreeMap<usize, usize> = BTreeMap::new();
        spaces.insert(rug_fuzz_0, rug_fuzz_1);
        spaces.insert(rug_fuzz_2, rug_fuzz_3);
        spaces.insert(rug_fuzz_4, rug_fuzz_5);
        debug_assert_eq!(extract_count(spaces), 2);
        let _rug_ed_tests_llm_16_799_rrrruuuugggg_test_extract_count = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_800 {
    use super::*;
    use crate::*;
    #[test]
    fn test_gcd_returns_b_when_a_is_zero() {
        let _rug_st_tests_llm_16_800_rrrruuuugggg_test_gcd_returns_b_when_a_is_zero = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 5;
        debug_assert_eq!(gcd(rug_fuzz_0, rug_fuzz_1), 5);
        let _rug_ed_tests_llm_16_800_rrrruuuugggg_test_gcd_returns_b_when_a_is_zero = 0;
    }
    #[test]
    fn test_gcd_returns_a_when_b_is_zero() {
        let _rug_st_tests_llm_16_800_rrrruuuugggg_test_gcd_returns_a_when_b_is_zero = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 0;
        debug_assert_eq!(gcd(rug_fuzz_0, rug_fuzz_1), 5);
        let _rug_ed_tests_llm_16_800_rrrruuuugggg_test_gcd_returns_a_when_b_is_zero = 0;
    }
    #[test]
    fn test_gcd_returns_a_when_a_equals_b() {
        let _rug_st_tests_llm_16_800_rrrruuuugggg_test_gcd_returns_a_when_a_equals_b = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 5;
        debug_assert_eq!(gcd(rug_fuzz_0, rug_fuzz_1), 5);
        let _rug_ed_tests_llm_16_800_rrrruuuugggg_test_gcd_returns_a_when_a_equals_b = 0;
    }
    #[test]
    fn test_gcd_returns_gcd_of_a_and_b() {
        let _rug_st_tests_llm_16_800_rrrruuuugggg_test_gcd_returns_gcd_of_a_and_b = 0;
        let rug_fuzz_0 = 12;
        let rug_fuzz_1 = 18;
        let rug_fuzz_2 = 18;
        let rug_fuzz_3 = 12;
        let rug_fuzz_4 = 17;
        let rug_fuzz_5 = 13;
        let rug_fuzz_6 = 60;
        let rug_fuzz_7 = 48;
        let rug_fuzz_8 = 48;
        let rug_fuzz_9 = 60;
        debug_assert_eq!(gcd(rug_fuzz_0, rug_fuzz_1), 6);
        debug_assert_eq!(gcd(rug_fuzz_2, rug_fuzz_3), 6);
        debug_assert_eq!(gcd(rug_fuzz_4, rug_fuzz_5), 1);
        debug_assert_eq!(gcd(rug_fuzz_6, rug_fuzz_7), 12);
        debug_assert_eq!(gcd(rug_fuzz_8, rug_fuzz_9), 12);
        let _rug_ed_tests_llm_16_800_rrrruuuugggg_test_gcd_returns_gcd_of_a_and_b = 0;
    }
}
