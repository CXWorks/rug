use crate::*;
use pest::Parser;
use std::str::FromStr;
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct RangeSet {
    pub ranges: Vec<Range>,
    pub compat: Compat,
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Compat {
    Cargo,
    Npm,
}
impl RangeSet {
    fn new() -> RangeSet {
        RangeSet {
            ranges: Vec::new(),
            compat: Compat::Cargo,
        }
    }
    pub fn parse(input: &str, compat: Compat) -> Result<Self, String> {
        let range_set = match SemverParser::parse(Rule::range_set, input) {
            Ok(mut parsed) => {
                match parsed.next() {
                    Some(parsed) => parsed,
                    None => return Err(String::from("Could not parse a range set")),
                }
            }
            Err(e) => return Err(e.to_string()),
        };
        from_pair_iterator(range_set, compat)
    }
}
impl FromStr for RangeSet {
    type Err = String;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        RangeSet::parse(input, Compat::Cargo)
    }
}
/// Converts an iterator of Pairs into a RangeSet
fn from_pair_iterator(
    parsed_range_set: pest::iterators::Pair<'_, Rule>,
    compat: Compat,
) -> Result<RangeSet, String> {
    if parsed_range_set.as_rule() != Rule::range_set {
        return Err(String::from("Error parsing range set"));
    }
    let mut range_set = RangeSet::new();
    range_set.compat = compat;
    for record in parsed_range_set.into_inner() {
        match record.as_rule() {
            Rule::range => {
                range_set.ranges.push(range::from_pair_iterator(record, compat)?);
            }
            Rule::logical_or => {}
            Rule::EOI => {}
            _ => unreachable!(),
        }
    }
    Ok(range_set)
}
#[cfg(test)]
mod tests {
    use super::*;
    macro_rules! range_set_test {
        ($name:ident : $input:expr, $($x:tt)*) => {
            #[test] fn $name () { let expected_sets = vec![$($x)*]; let range_set :
            RangeSet = $input .parse().expect("parse failed"); assert_eq!(range_set
            .ranges.len(), expected_sets.len()); for it in range_set.ranges.iter()
            .zip(expected_sets.iter()) { let (ai, bi) = it; assert_eq!(ai.comparator_set
            .len(), * bi); } }
        };
    }
    macro_rules! range_set_nodecompat {
        ($name:ident : $input:expr, $($x:tt)*) => {
            #[test] fn $name () { let expected_sets = vec![$($x)*]; let range_set =
            RangeSet::parse($input, Compat::Npm).expect("parse failed");
            assert_eq!(range_set.ranges.len(), expected_sets.len()); for it in range_set
            .ranges.iter().zip(expected_sets.iter()) { let (ai, bi) = it; assert_eq!(ai
            .comparator_set.len(), * bi); } }
        };
    }
    macro_rules! should_error {
        ($($name:ident : $value:expr,)*) => {
            $(#[test] fn $name () { assert!($value .parse::< RangeSet > ().is_err()); })*
        };
    }
    range_set_test!(one_range : "=1.2.3", 1);
    range_set_test!(one_range_cargo : "1.2.3", 2);
    range_set_test!(one_range_with_space : "   =1.2.3 ", 1);
    range_set_test!(two_ranges : ">1.2.3 || =4.5.6", 1, 1);
    range_set_test!(two_ranges_with_space : " >1.2.3 || =4.5.6  ", 1, 1);
    range_set_test!(
        two_ranges_with_two_comparators : ">1.2.3 <2.3.4 || >4.5.6 <5.6.7", 2, 2
    );
    range_set_test!(caret_range : "^1.2.3", 2);
    range_set_test!(two_empty_ranges : "||", 1, 1);
    range_set_test!(two_xranges : "1.2.* || 2.*", 2, 2);
    range_set_test!(see_issue_88 : "=1.2.3+meta", 1);
    range_set_nodecompat!(node_one_range : "1.2.3", 1);
    should_error! {
        err_only_gt : ">", err_only_lt : "<", err_only_lte : "<=", err_only_gte : ">=",
        err_only_eq : "=", err_only_tilde : "~", err_only_caret : "^",
        err_leading_0_major : "01.2.3", err_leading_0_minor : "1.02.3",
        err_leading_0_patch : "1.2.03", err_hyphen_with_gt : "1.2.3 - >3.4.5",
        err_hyphen_no_2nd_version : "1.2.3 - ", err_no_pre_hyphen : "~1.2.3beta",
    }
}
#[cfg(test)]
mod tests_llm_16_103 {
    use super::*;
    use crate::*;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_103_rrrruuuugggg_test_new = 0;
        let range_set = RangeSet::new();
        debug_assert_eq!(range_set.ranges.len(), 0);
        debug_assert_eq!(range_set.compat, Compat::Cargo);
        let _rug_ed_tests_llm_16_103_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_104 {
    use super::*;
    use crate::*;
    #[test]
    fn test_parse_valid_input() {
        let _rug_st_tests_llm_16_104_rrrruuuugggg_test_parse_valid_input = 0;
        let rug_fuzz_0 = ">=1.0.0 <2.0.0 || >3.0.0";
        let input = rug_fuzz_0;
        let compat = Compat::Cargo;
        let result = RangeSet::parse(input, compat);
        debug_assert!(result.is_ok());
        let _rug_ed_tests_llm_16_104_rrrruuuugggg_test_parse_valid_input = 0;
    }
    #[test]
    fn test_parse_invalid_input() {
        let _rug_st_tests_llm_16_104_rrrruuuugggg_test_parse_invalid_input = 0;
        let rug_fuzz_0 = ">=1.0.0 <2.0.0 ||";
        let input = rug_fuzz_0;
        let compat = Compat::Cargo;
        let result = RangeSet::parse(input, compat);
        debug_assert!(result.is_err());
        let _rug_ed_tests_llm_16_104_rrrruuuugggg_test_parse_invalid_input = 0;
    }
}
#[cfg(test)]
mod tests_rug_21 {
    use super::*;
    use pest::iterators::Pair;
    use crate::{range, Compat, Rule};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_21_rrrruuuugggg_test_rug = 0;
        let mut p0: Pair<Rule> = unimplemented!();
        let mut p1: Compat = Compat::Cargo;
        range_set::from_pair_iterator(p0, p1);
        let _rug_ed_tests_rug_21_rrrruuuugggg_test_rug = 0;
    }
}
