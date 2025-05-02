//! Parse formats used in the `format` and `parse` methods.
use crate::format::{FormatItem, Padding, Specifier};
#[cfg(not(feature = "std"))]
use alloc::{format, string::String, vec::Vec};
/// Parse the formatting string. Panics if not valid.
pub(crate) fn parse_fmt_string<'a>(s: &'a str) -> Vec<FormatItem<'a>> {
    match try_parse_fmt_string(s) {
        Ok(items) => items,
        Err(err) => panic!("{}", err),
    }
}
/// Attempt to parse the formatting string.
#[allow(clippy::too_many_lines)]
pub(crate) fn try_parse_fmt_string<'a>(
    s: &'a str,
) -> Result<Vec<FormatItem<'a>>, String> {
    let mut items = Vec::new();
    let mut literal_start = 0;
    let mut chars = s.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        /// Push the provided specifier to the list of items.
        macro_rules! push_specifier {
            ($i:ident, $specifier:expr) => {
                { literal_start = $i + 1; items.push(FormatItem::Specifier($specifier)) }
            };
        }
        if c == '%' {
            if literal_start != i {
                items.push(FormatItem::Literal(&s[literal_start..i]));
            }
            let padding = match chars.peek().map(|v| v.1) {
                Some('-') => {
                    let _ = chars.next();
                    Some(Padding::None)
                }
                Some('_') => {
                    let _ = chars.next();
                    Some(Padding::Space)
                }
                Some('0') => {
                    let _ = chars.next();
                    Some(Padding::Zero)
                }
                _ => None,
            };
            match chars.next() {
                Some((i, 'a')) => push_specifier!(i, Specifier::a),
                Some((i, 'A')) => push_specifier!(i, Specifier::A),
                Some((i, 'b')) => push_specifier!(i, Specifier::b),
                Some((i, 'B')) => push_specifier!(i, Specifier::B),
                Some((i, 'c')) => push_specifier!(i, Specifier::c),
                Some((i, 'C')) => {
                    push_specifier!(
                        i, Specifier::C { padding : padding.unwrap_or(Padding::Zero) }
                    )
                }
                Some((i, 'd')) => {
                    push_specifier!(
                        i, Specifier::d { padding : padding.unwrap_or(Padding::Zero) }
                    )
                }
                Some((i, 'D')) => push_specifier!(i, Specifier::D),
                Some((i, 'F')) => push_specifier!(i, Specifier::F),
                Some((i, 'g')) => {
                    push_specifier!(
                        i, Specifier::g { padding : padding.unwrap_or(Padding::Zero) }
                    )
                }
                Some((i, 'G')) => {
                    push_specifier!(
                        i, Specifier::G { padding : padding.unwrap_or(Padding::Zero) }
                    )
                }
                Some((i, 'H')) => {
                    push_specifier!(
                        i, Specifier::H { padding : padding.unwrap_or(Padding::Zero) }
                    )
                }
                Some((i, 'I')) => {
                    push_specifier!(
                        i, Specifier::I { padding : padding.unwrap_or(Padding::Zero) }
                    )
                }
                Some((i, 'j')) => {
                    push_specifier!(
                        i, Specifier::j { padding : padding.unwrap_or(Padding::Zero) }
                    )
                }
                Some((i, 'm')) => {
                    push_specifier!(
                        i, Specifier::m { padding : padding.unwrap_or(Padding::Zero) }
                    )
                }
                Some((i, 'M')) => {
                    push_specifier!(
                        i, Specifier::M { padding : padding.unwrap_or(Padding::Zero) }
                    )
                }
                Some((i, 'N')) => push_specifier!(i, Specifier::N),
                Some((i, 'p')) => push_specifier!(i, Specifier::p),
                Some((i, 'P')) => push_specifier!(i, Specifier::P),
                Some((i, 'r')) => push_specifier!(i, Specifier::r),
                Some((i, 'R')) => push_specifier!(i, Specifier::R),
                Some((i, 'S')) => {
                    push_specifier!(
                        i, Specifier::S { padding : padding.unwrap_or(Padding::Zero) }
                    )
                }
                Some((i, 'T')) => push_specifier!(i, Specifier::T),
                Some((i, 'u')) => push_specifier!(i, Specifier::u),
                Some((i, 'U')) => {
                    push_specifier!(
                        i, Specifier::U { padding : padding.unwrap_or(Padding::Zero) }
                    )
                }
                Some((i, 'V')) => {
                    push_specifier!(
                        i, Specifier::V { padding : padding.unwrap_or(Padding::Zero) }
                    )
                }
                Some((i, 'w')) => push_specifier!(i, Specifier::w),
                Some((i, 'W')) => {
                    push_specifier!(
                        i, Specifier::W { padding : padding.unwrap_or(Padding::Zero) }
                    )
                }
                Some((i, 'y')) => {
                    push_specifier!(
                        i, Specifier::y { padding : padding.unwrap_or(Padding::Zero) }
                    )
                }
                Some((i, 'Y')) => {
                    push_specifier!(
                        i, Specifier::Y { padding : padding.unwrap_or(Padding::Zero) }
                    )
                }
                Some((i, 'z')) => push_specifier!(i, Specifier::z),
                Some((i, '%')) => literal_start = i,
                Some((_, c)) => return Err(format!("Invalid specifier `{}`", c)),
                None => {
                    return Err(
                        String::from(
                            "Cannot end formatting with `%`. If you want a literal `%`, you must use \
                         `%%`.",
                        ),
                    );
                }
            }
        }
    }
    if literal_start < s.len() {
        items.push(FormatItem::Literal(&s[literal_start..]));
    }
    Ok(items)
}
#[cfg(test)]
mod tests_llm_16_861 {
    use super::*;
    use crate::*;
    #[test]
    fn test_try_parse_fmt_string() {
        let _rug_st_tests_llm_16_861_rrrruuuugggg_test_try_parse_fmt_string = 0;
        let rug_fuzz_0 = "%Y-%m-%d";
        let rug_fuzz_1 = "%H:%M:%S";
        let rug_fuzz_2 = "This is a literal";
        let rug_fuzz_3 = "%";
        let rug_fuzz_4 = "Invalid %d specifier";
        let result = try_parse_fmt_string(rug_fuzz_0);
        debug_assert!(result.is_ok());
        let result = try_parse_fmt_string(rug_fuzz_1);
        debug_assert!(result.is_ok());
        let result = try_parse_fmt_string(rug_fuzz_2);
        debug_assert!(result.is_ok());
        let result = try_parse_fmt_string(rug_fuzz_3);
        debug_assert!(result.is_err());
        let result = try_parse_fmt_string(rug_fuzz_4);
        debug_assert!(result.is_err());
        let _rug_ed_tests_llm_16_861_rrrruuuugggg_test_try_parse_fmt_string = 0;
    }
}
#[cfg(test)]
mod tests_rug_63 {
    use super::*;
    use crate::format::parse_items::FormatItem;
    #[test]
    fn test_parse_fmt_string() {
        let _rug_st_tests_rug_63_rrrruuuugggg_test_parse_fmt_string = 0;
        let rug_fuzz_0 = "YYYY-MM-DD";
        let p0: &str = rug_fuzz_0;
        crate::format::parse_items::parse_fmt_string(&p0);
        let _rug_ed_tests_rug_63_rrrruuuugggg_test_parse_fmt_string = 0;
    }
}
