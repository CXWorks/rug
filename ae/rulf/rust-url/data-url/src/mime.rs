use std::fmt::{self, Write};
use std::str::FromStr;
/// <https://mimesniff.spec.whatwg.org/#mime-type-representation>
#[derive(Debug, PartialEq, Eq)]
pub struct Mime {
    pub type_: String,
    pub subtype: String,
    /// (name, value)
    pub parameters: Vec<(String, String)>,
}
impl Mime {
    pub fn get_parameter<P>(&self, name: &P) -> Option<&str>
    where
        P: ?Sized + PartialEq<str>,
    {
        self.parameters.iter().find(|&&(ref n, _)| name == &**n).map(|&(_, ref v)| &**v)
    }
}
#[derive(Debug)]
pub struct MimeParsingError(());
/// <https://mimesniff.spec.whatwg.org/#parsing-a-mime-type>
impl FromStr for Mime {
    type Err = MimeParsingError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse(s).ok_or(MimeParsingError(()))
    }
}
fn parse(s: &str) -> Option<Mime> {
    let trimmed = s.trim_matches(ascii_whitespace);
    let (type_, rest) = split2(trimmed, '/');
    require!(only_http_token_code_points(type_) && ! type_.is_empty());
    let (subtype, rest) = split2(rest?, ';');
    let subtype = subtype.trim_end_matches(ascii_whitespace);
    require!(only_http_token_code_points(subtype) && ! subtype.is_empty());
    let mut parameters = Vec::new();
    if let Some(rest) = rest {
        parse_parameters(rest, &mut parameters)
    }
    Some(Mime {
        type_: type_.to_ascii_lowercase(),
        subtype: subtype.to_ascii_lowercase(),
        parameters,
    })
}
fn split2(s: &str, separator: char) -> (&str, Option<&str>) {
    let mut iter = s.splitn(2, separator);
    let first = iter.next().unwrap();
    (first, iter.next())
}
fn parse_parameters(s: &str, parameters: &mut Vec<(String, String)>) {
    let mut semicolon_separated = s.split(';');
    while let Some(piece) = semicolon_separated.next() {
        let piece = piece.trim_start_matches(ascii_whitespace);
        let (name, value) = split2(piece, '=');
        if name.is_empty() || !only_http_token_code_points(name)
            || contains(&parameters, name)
        {
            continue;
        }
        if let Some(value) = value {
            let value = if value.starts_with('"') {
                let max_len = value.len().saturating_sub(2);
                let mut unescaped_value = String::with_capacity(max_len);
                let mut chars = value[1..].chars();
                'until_closing_quote: loop {
                    while let Some(c) = chars.next() {
                        match c {
                            '"' => break 'until_closing_quote,
                            '\\' => unescaped_value.push(chars.next().unwrap_or('\\')),
                            _ => unescaped_value.push(c),
                        }
                    }
                    if let Some(piece) = semicolon_separated.next() {
                        unescaped_value.push(';');
                        chars = piece.chars();
                    } else {
                        break;
                    }
                }
                if !valid_value(&unescaped_value) {
                    continue;
                }
                unescaped_value
            } else {
                let value = value.trim_end_matches(ascii_whitespace);
                if !valid_value(value) {
                    continue;
                }
                value.to_owned()
            };
            parameters.push((name.to_ascii_lowercase(), value))
        }
    }
}
fn contains(parameters: &[(String, String)], name: &str) -> bool {
    parameters.iter().any(|&(ref n, _)| n == name)
}
fn valid_value(s: &str) -> bool {
    s.chars().all(|c| { matches!(c, '\t' | ' '..='~' | '\u{80}'..='\u{FF}') })
        && !s.is_empty()
}
/// <https://mimesniff.spec.whatwg.org/#serializing-a-mime-type>
impl fmt::Display for Mime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.type_)?;
        f.write_str("/")?;
        f.write_str(&self.subtype)?;
        for &(ref name, ref value) in &self.parameters {
            f.write_str(";")?;
            f.write_str(name)?;
            f.write_str("=")?;
            if only_http_token_code_points(value) {
                f.write_str(value)?
            } else {
                f.write_str("\"")?;
                for c in value.chars() {
                    if c == '"' || c == '\\' {
                        f.write_str("\\")?
                    }
                    f.write_char(c)?
                }
                f.write_str("\"")?
            }
        }
        Ok(())
    }
}
fn ascii_whitespace(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0C')
}
fn only_http_token_code_points(s: &str) -> bool {
    s.bytes().all(|byte| IS_HTTP_TOKEN[byte as usize])
}
macro_rules! byte_map {
    ($($flag:expr,)*) => {
        [$($flag != 0,)*]
    };
}
#[rustfmt::skip]
static IS_HTTP_TOKEN: [bool; 256] = byte_map![
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 1, 0, 1, 1, 1, 1, 1, 0, 0, 1, 1, 0, 1, 1, 0, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0,
];
#[cfg(test)]
mod tests_llm_16_3 {
    use crate::mime::Mime;
    use crate::mime::mime_parsing::MimeParsingError;
    use std::str::FromStr;
    #[test]
    fn test_from_str_valid_input() {
        let _rug_st_tests_llm_16_3_rrrruuuugggg_test_from_str_valid_input = 0;
        let rug_fuzz_0 = "text/plain; charset=utf-8";
        let input = rug_fuzz_0;
        let expected = Mime::from_str(input).unwrap();
        let result = <Mime as FromStr>::from_str(input).unwrap();
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_3_rrrruuuugggg_test_from_str_valid_input = 0;
    }
    #[test]
    #[should_panic]
    fn test_from_str_invalid_input() {
        let _rug_st_tests_llm_16_3_rrrruuuugggg_test_from_str_invalid_input = 0;
        let rug_fuzz_0 = "invalid_mime";
        let input = rug_fuzz_0;
        let expected = MimeParsingError(());
        let result = <Mime as FromStr>::from_str(input).unwrap();
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_3_rrrruuuugggg_test_from_str_invalid_input = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_16 {
    use super::*;
    use crate::*;
    #[test]
    fn test_get_parameter_existing() {
        let _rug_st_tests_llm_16_16_rrrruuuugggg_test_get_parameter_existing = 0;
        let rug_fuzz_0 = "text";
        let rug_fuzz_1 = "plain";
        let rug_fuzz_2 = "charset";
        let rug_fuzz_3 = "UTF-8";
        let rug_fuzz_4 = "charset";
        let mime = Mime {
            type_: rug_fuzz_0.to_string(),
            subtype: rug_fuzz_1.to_string(),
            parameters: vec![(rug_fuzz_2.to_string(), rug_fuzz_3.to_string())],
        };
        let result = mime.get_parameter(rug_fuzz_4);
        debug_assert_eq!(result, Some("UTF-8"));
        let _rug_ed_tests_llm_16_16_rrrruuuugggg_test_get_parameter_existing = 0;
    }
    #[test]
    fn test_get_parameter_non_existing() {
        let _rug_st_tests_llm_16_16_rrrruuuugggg_test_get_parameter_non_existing = 0;
        let rug_fuzz_0 = "text";
        let rug_fuzz_1 = "plain";
        let rug_fuzz_2 = "charset";
        let rug_fuzz_3 = "UTF-8";
        let rug_fuzz_4 = "format";
        let mime = Mime {
            type_: rug_fuzz_0.to_string(),
            subtype: rug_fuzz_1.to_string(),
            parameters: vec![(rug_fuzz_2.to_string(), rug_fuzz_3.to_string())],
        };
        let result = mime.get_parameter(rug_fuzz_4);
        debug_assert_eq!(result, None);
        let _rug_ed_tests_llm_16_16_rrrruuuugggg_test_get_parameter_non_existing = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_17 {
    use super::*;
    use crate::*;
    #[test]
    fn test_ascii_whitespace() {
        let _rug_st_tests_llm_16_17_rrrruuuugggg_test_ascii_whitespace = 0;
        let rug_fuzz_0 = ' ';
        let rug_fuzz_1 = '\t';
        let rug_fuzz_2 = '\n';
        let rug_fuzz_3 = '\r';
        let rug_fuzz_4 = '\x0C';
        let rug_fuzz_5 = 'a';
        let rug_fuzz_6 = '1';
        let rug_fuzz_7 = '?';
        debug_assert_eq!(ascii_whitespace(rug_fuzz_0), true);
        debug_assert_eq!(ascii_whitespace(rug_fuzz_1), true);
        debug_assert_eq!(ascii_whitespace(rug_fuzz_2), true);
        debug_assert_eq!(ascii_whitespace(rug_fuzz_3), true);
        debug_assert_eq!(ascii_whitespace(rug_fuzz_4), true);
        debug_assert_eq!(ascii_whitespace(rug_fuzz_5), false);
        debug_assert_eq!(ascii_whitespace(rug_fuzz_6), false);
        debug_assert_eq!(ascii_whitespace(rug_fuzz_7), false);
        let _rug_ed_tests_llm_16_17_rrrruuuugggg_test_ascii_whitespace = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_18 {
    use super::*;
    use crate::*;
    #[test]
    fn test_contains() {
        let _rug_st_tests_llm_16_18_rrrruuuugggg_test_contains = 0;
        let rug_fuzz_0 = "name1";
        let rug_fuzz_1 = "value1";
        let rug_fuzz_2 = "name2";
        let rug_fuzz_3 = "value2";
        let rug_fuzz_4 = "name3";
        let rug_fuzz_5 = "value3";
        let rug_fuzz_6 = "name1";
        let rug_fuzz_7 = "name2";
        let rug_fuzz_8 = "name3";
        let rug_fuzz_9 = "name4";
        let parameters: &[(String, String)] = &[
            (rug_fuzz_0.to_string(), rug_fuzz_1.to_string()),
            (rug_fuzz_2.to_string(), rug_fuzz_3.to_string()),
            (rug_fuzz_4.to_string(), rug_fuzz_5.to_string()),
        ];
        debug_assert_eq!(contains(parameters, rug_fuzz_6), true);
        debug_assert_eq!(contains(parameters, rug_fuzz_7), true);
        debug_assert_eq!(contains(parameters, rug_fuzz_8), true);
        debug_assert_eq!(contains(parameters, rug_fuzz_9), false);
        let _rug_ed_tests_llm_16_18_rrrruuuugggg_test_contains = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_19 {
    use crate::only_http_token_code_points;
    #[test]
    fn test_only_http_token_code_points() {
        let _rug_st_tests_llm_16_19_rrrruuuugggg_test_only_http_token_code_points = 0;
        let rug_fuzz_0 = "rust";
        let rug_fuzz_1 = "rust-lang";
        let rug_fuzz_2 = "123";
        let rug_fuzz_3 = "hello world";
        let rug_fuzz_4 = "HTTP";
        let rug_fuzz_5 = "HTTP/1.1";
        debug_assert_eq!(only_http_token_code_points(rug_fuzz_0), true);
        debug_assert_eq!(only_http_token_code_points(rug_fuzz_1), false);
        debug_assert_eq!(only_http_token_code_points(rug_fuzz_2), false);
        debug_assert_eq!(only_http_token_code_points(rug_fuzz_3), false);
        debug_assert_eq!(only_http_token_code_points(rug_fuzz_4), true);
        debug_assert_eq!(only_http_token_code_points(rug_fuzz_5), false);
        let _rug_ed_tests_llm_16_19_rrrruuuugggg_test_only_http_token_code_points = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_20 {
    use super::*;
    use crate::*;
    #[test]
    fn test_parse() {
        let _rug_st_tests_llm_16_20_rrrruuuugggg_test_parse = 0;
        let rug_fuzz_0 = "text/html;charset=utf-8";
        let rug_fuzz_1 = "application/json";
        let rug_fuzz_2 = "image/jpeg;quality=80";
        let rug_fuzz_3 = "invalid";
        debug_assert_eq!(
            parse(rug_fuzz_0), Some(Mime { type_ : "text".to_ascii_lowercase(), subtype :
            "html".to_ascii_lowercase(), parameters : vec![("charset".to_string(),
            "utf-8".to_string())], })
        );
        debug_assert_eq!(
            parse(rug_fuzz_1), Some(Mime { type_ : "application".to_ascii_lowercase(),
            subtype : "json".to_ascii_lowercase(), parameters : vec![], })
        );
        debug_assert_eq!(
            parse(rug_fuzz_2), Some(Mime { type_ : "image".to_ascii_lowercase(), subtype
            : "jpeg".to_ascii_lowercase(), parameters : vec![("quality".to_string(), "80"
            .to_string())], })
        );
        debug_assert_eq!(parse(rug_fuzz_3), None);
        let _rug_ed_tests_llm_16_20_rrrruuuugggg_test_parse = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_21 {
    use super::*;
    use crate::*;
    #[test]
    fn test_parse_parameters() {
        let _rug_st_tests_llm_16_21_rrrruuuugggg_test_parse_parameters = 0;
        let rug_fuzz_0 = "charset=UTF-8";
        let rug_fuzz_1 = "charset=UTF-8; q=0.5; foo=bar";
        let rug_fuzz_2 = "charset=UTF-8; q=0.5; foo=bar; bz=qux";
        let rug_fuzz_3 = "charset=UTF-8; q=0.5; foo=bar; bz=qux; extended=\"yes;no\"";
        let mut parameters: Vec<(String, String)> = Vec::new();
        parse_parameters(rug_fuzz_0, &mut parameters);
        debug_assert_eq!(parameters, vec![("charset".to_owned(), "UTF-8".to_owned())]);
        parse_parameters(rug_fuzz_1, &mut parameters);
        debug_assert_eq!(
            parameters, vec![("charset".to_owned(), "UTF-8".to_owned()), ("q".to_owned(),
            "0.5".to_owned()), ("foo".to_owned(), "bar".to_owned()),]
        );
        parse_parameters(rug_fuzz_2, &mut parameters);
        debug_assert_eq!(
            parameters, vec![("charset".to_owned(), "UTF-8".to_owned()), ("q".to_owned(),
            "0.5".to_owned()), ("foo".to_owned(), "bar".to_owned()), ("bz".to_owned(),
            "qux".to_owned()),]
        );
        parse_parameters(rug_fuzz_3, &mut parameters);
        debug_assert_eq!(
            parameters, vec![("charset".to_owned(), "UTF-8".to_owned()), ("q".to_owned(),
            "0.5".to_owned()), ("foo".to_owned(), "bar".to_owned()), ("bz".to_owned(),
            "qux".to_owned()), ("extended".to_owned(), "yes;no".to_owned()),]
        );
        let _rug_ed_tests_llm_16_21_rrrruuuugggg_test_parse_parameters = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_22 {
    use super::*;
    use crate::*;
    #[test]
    fn test_split2() {
        let _rug_st_tests_llm_16_22_rrrruuuugggg_test_split2 = 0;
        let rug_fuzz_0 = "hello,world";
        let rug_fuzz_1 = ',';
        let s = rug_fuzz_0;
        let separator = rug_fuzz_1;
        let (first, second) = split2(s, separator);
        debug_assert_eq!(first, "hello");
        debug_assert_eq!(second, Some("world"));
        let _rug_ed_tests_llm_16_22_rrrruuuugggg_test_split2 = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_23 {
    use crate::mime::valid_value;
    #[test]
    fn test_valid_value() {
        let _rug_st_tests_llm_16_23_rrrruuuugggg_test_valid_value = 0;
        let rug_fuzz_0 = "text/plain";
        let rug_fuzz_1 = "image/jpeg ";
        let rug_fuzz_2 = "application/pdf?";
        let rug_fuzz_3 = "αβγδε";
        let rug_fuzz_4 = "text/\tplain";
        let rug_fuzz_5 = "";
        debug_assert_eq!(valid_value(rug_fuzz_0), true);
        debug_assert_eq!(valid_value(rug_fuzz_1), true);
        debug_assert_eq!(valid_value(rug_fuzz_2), true);
        debug_assert_eq!(valid_value(rug_fuzz_3), true);
        debug_assert_eq!(valid_value(rug_fuzz_4), false);
        debug_assert_eq!(valid_value(rug_fuzz_5), false);
        let _rug_ed_tests_llm_16_23_rrrruuuugggg_test_valid_value = 0;
    }
}
