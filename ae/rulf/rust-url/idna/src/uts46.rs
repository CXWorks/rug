//! [*Unicode IDNA Compatibility Processing*
//! (Unicode Technical Standard #46)](http://www.unicode.org/reports/tr46/)
use self::Mapping::*;
use crate::punycode;
use std::cmp::Ordering::{Equal, Greater, Less};
use std::{error::Error as StdError, fmt};
use unicode_bidi::{bidi_class, BidiClass};
use unicode_normalization::char::is_combining_mark;
use unicode_normalization::{is_nfc, UnicodeNormalization};
include!("uts46_mapping_table.rs");
const PUNYCODE_PREFIX: &str = "xn--";
#[derive(Debug)]
struct StringTableSlice {
    byte_start_lo: u8,
    byte_start_hi: u8,
    byte_len: u8,
}
fn decode_slice(slice: &StringTableSlice) -> &'static str {
    let lo = slice.byte_start_lo as usize;
    let hi = slice.byte_start_hi as usize;
    let start = (hi << 8) | lo;
    let len = slice.byte_len as usize;
    &STRING_TABLE[start..(start + len)]
}
#[repr(u8)]
#[derive(Debug)]
enum Mapping {
    Valid,
    Ignored,
    Mapped(StringTableSlice),
    Deviation(StringTableSlice),
    Disallowed,
    DisallowedStd3Valid,
    DisallowedStd3Mapped(StringTableSlice),
}
struct Range {
    from: char,
    to: char,
}
fn find_char(codepoint: char) -> &'static Mapping {
    let r = TABLE
        .binary_search_by(|ref range| {
            if codepoint > range.to {
                Less
            } else if codepoint < range.from {
                Greater
            } else {
                Equal
            }
        });
    r.ok()
        .map(|i| {
            const SINGLE_MARKER: u16 = 1 << 15;
            let x = INDEX_TABLE[i];
            let single = (x & SINGLE_MARKER) != 0;
            let offset = !SINGLE_MARKER & x;
            if single {
                &MAPPING_TABLE[offset as usize]
            } else {
                &MAPPING_TABLE[(offset + (codepoint as u16 - TABLE[i].from as u16))
                    as usize]
            }
        })
        .unwrap()
}
fn map_char(codepoint: char, config: Config, output: &mut String, errors: &mut Errors) {
    if let '.' | '-' | 'a'..='z' | '0'..='9' = codepoint {
        output.push(codepoint);
        return;
    }
    match *find_char(codepoint) {
        Mapping::Valid => output.push(codepoint),
        Mapping::Ignored => {}
        Mapping::Mapped(ref slice) => output.push_str(decode_slice(slice)),
        Mapping::Deviation(ref slice) => {
            if config.transitional_processing {
                output.push_str(decode_slice(slice))
            } else {
                output.push(codepoint)
            }
        }
        Mapping::Disallowed => {
            errors.disallowed_character = true;
            output.push(codepoint);
        }
        Mapping::DisallowedStd3Valid => {
            if config.use_std3_ascii_rules {
                errors.disallowed_by_std3_ascii_rules = true;
            }
            output.push(codepoint)
        }
        Mapping::DisallowedStd3Mapped(ref slice) => {
            if config.use_std3_ascii_rules {
                errors.disallowed_mapped_in_std3 = true;
            }
            output.push_str(decode_slice(slice))
        }
    }
}
fn passes_bidi(label: &str, is_bidi_domain: bool) -> bool {
    if !is_bidi_domain {
        return true;
    }
    let mut chars = label.chars();
    let first_char_class = match chars.next() {
        Some(c) => bidi_class(c),
        None => return true,
    };
    match first_char_class {
        BidiClass::L => {
            while let Some(c) = chars.next() {
                if !matches!(
                    bidi_class(c), BidiClass::L | BidiClass::EN | BidiClass::ES |
                    BidiClass::CS | BidiClass::ET | BidiClass::ON | BidiClass::BN |
                    BidiClass::NSM
                ) {
                    return false;
                }
            }
            let mut rev_chars = label.chars().rev();
            let mut last_non_nsm = rev_chars.next();
            loop {
                match last_non_nsm {
                    Some(c) if bidi_class(c) == BidiClass::NSM => {
                        last_non_nsm = rev_chars.next();
                        continue;
                    }
                    _ => {
                        break;
                    }
                }
            }
            match last_non_nsm {
                Some(
                    c,
                ) if bidi_class(c) == BidiClass::L || bidi_class(c) == BidiClass::EN => {}
                Some(_) => {
                    return false;
                }
                _ => {}
            }
        }
        BidiClass::R | BidiClass::AL => {
            let mut found_en = false;
            let mut found_an = false;
            for c in chars {
                let char_class = bidi_class(c);
                if char_class == BidiClass::EN {
                    found_en = true;
                } else if char_class == BidiClass::AN {
                    found_an = true;
                }
                if !matches!(
                    char_class, BidiClass::R | BidiClass::AL | BidiClass::AN |
                    BidiClass::EN | BidiClass::ES | BidiClass::CS | BidiClass::ET |
                    BidiClass::ON | BidiClass::BN | BidiClass::NSM
                ) {
                    return false;
                }
            }
            let mut rev_chars = label.chars().rev();
            let mut last = rev_chars.next();
            loop {
                match last {
                    Some(c) if bidi_class(c) == BidiClass::NSM => {
                        last = rev_chars.next();
                        continue;
                    }
                    _ => {
                        break;
                    }
                }
            }
            match last {
                Some(
                    c,
                ) if matches!(
                    bidi_class(c), BidiClass::R | BidiClass::AL | BidiClass::EN |
                    BidiClass::AN
                ) => {}
                _ => {
                    return false;
                }
            }
            if found_an && found_en {
                return false;
            }
        }
        _ => {
            return false;
        }
    }
    true
}
/// Check the validity criteria for the given label
///
/// V1 (NFC) and V8 (Bidi) are checked inside `processing()` to prevent doing duplicate work.
///
/// http://www.unicode.org/reports/tr46/#Validity_Criteria
fn is_valid(label: &str, config: Config) -> bool {
    let first_char = label.chars().next();
    if first_char == None {
        return true;
    }
    if config.check_hyphens && (label.starts_with('-') || label.ends_with('-')) {
        return false;
    }
    if is_combining_mark(first_char.unwrap()) {
        return false;
    }
    if label
        .chars()
        .any(|c| match *find_char(c) {
            Mapping::Valid => false,
            Mapping::Deviation(_) => config.transitional_processing,
            Mapping::DisallowedStd3Valid => config.use_std3_ascii_rules,
            _ => true,
        })
    {
        return false;
    }
    true
}
/// http://www.unicode.org/reports/tr46/#Processing
fn processing(domain: &str, config: Config) -> (String, Errors) {
    let (mut prev, mut simple, mut puny_prefix) = ('?', !domain.is_empty(), 0);
    for c in domain.chars() {
        if c == '.' {
            if prev == '-' {
                simple = false;
                break;
            }
            puny_prefix = 0;
            continue;
        } else if puny_prefix == 0 && c == '-' {
            simple = false;
            break;
        } else if puny_prefix < 5 {
            if c == ['x', 'n', '-', '-'][puny_prefix] {
                puny_prefix += 1;
                if puny_prefix == 4 {
                    simple = false;
                    break;
                }
            } else {
                puny_prefix = 5;
            }
        }
        if !c.is_ascii_lowercase() && !c.is_ascii_digit() {
            simple = false;
            break;
        }
        prev = c;
    }
    if simple {
        return (domain.to_owned(), Errors::default());
    }
    let mut errors = Errors::default();
    let mut mapped = String::with_capacity(domain.len());
    for c in domain.chars() {
        map_char(c, config, &mut mapped, &mut errors)
    }
    let mut normalized = String::with_capacity(mapped.len());
    normalized.extend(mapped.nfc());
    let mut validated = String::new();
    let non_transitional = config.transitional_processing(false);
    let (mut first, mut valid, mut has_bidi_labels) = (true, true, false);
    for label in normalized.split('.') {
        if !first {
            validated.push('.');
        }
        first = false;
        if label.starts_with(PUNYCODE_PREFIX) {
            match punycode::decode_to_string(&label[PUNYCODE_PREFIX.len()..]) {
                Some(decoded_label) => {
                    if !has_bidi_labels {
                        has_bidi_labels |= is_bidi_domain(&decoded_label);
                    }
                    if valid
                        && (!is_nfc(&decoded_label)
                            || !is_valid(&decoded_label, non_transitional))
                    {
                        valid = false;
                    }
                    validated.push_str(&decoded_label)
                }
                None => {
                    has_bidi_labels = true;
                    errors.punycode = true;
                }
            }
        } else {
            if !has_bidi_labels {
                has_bidi_labels |= is_bidi_domain(label);
            }
            valid &= is_valid(label, config);
            validated.push_str(label)
        }
    }
    for label in validated.split('.') {
        if !passes_bidi(label, has_bidi_labels) {
            valid = false;
            break;
        }
    }
    if !valid {
        errors.validity_criteria = true;
    }
    (validated, errors)
}
#[derive(Clone, Copy)]
pub struct Config {
    use_std3_ascii_rules: bool,
    transitional_processing: bool,
    verify_dns_length: bool,
    check_hyphens: bool,
}
/// The defaults are that of https://url.spec.whatwg.org/#idna
impl Default for Config {
    fn default() -> Self {
        Config {
            use_std3_ascii_rules: false,
            transitional_processing: false,
            check_hyphens: false,
            verify_dns_length: false,
        }
    }
}
impl Config {
    #[inline]
    pub fn use_std3_ascii_rules(mut self, value: bool) -> Self {
        self.use_std3_ascii_rules = value;
        self
    }
    #[inline]
    pub fn transitional_processing(mut self, value: bool) -> Self {
        self.transitional_processing = value;
        self
    }
    #[inline]
    pub fn verify_dns_length(mut self, value: bool) -> Self {
        self.verify_dns_length = value;
        self
    }
    #[inline]
    pub fn check_hyphens(mut self, value: bool) -> Self {
        self.check_hyphens = value;
        self
    }
    /// http://www.unicode.org/reports/tr46/#ToASCII
    pub fn to_ascii(self, domain: &str) -> Result<String, Errors> {
        let mut result = String::new();
        let mut first = true;
        let (domain, mut errors) = processing(domain, self);
        for label in domain.split('.') {
            if !first {
                result.push('.');
            }
            first = false;
            if label.is_ascii() {
                result.push_str(label);
            } else {
                match punycode::encode_str(label) {
                    Some(x) => {
                        result.push_str(PUNYCODE_PREFIX);
                        result.push_str(&x);
                    }
                    None => {
                        errors.punycode = true;
                    }
                }
            }
        }
        if self.verify_dns_length {
            let domain = if result.ends_with('.') {
                &result[..result.len() - 1]
            } else {
                &*result
            };
            if domain.is_empty() || domain.split('.').any(|label| label.is_empty()) {
                errors.too_short_for_dns = true;
            }
            if domain.len() > 253 || domain.split('.').any(|label| label.len() > 63) {
                errors.too_long_for_dns = true;
            }
        }
        Result::from(errors).map(|()| result)
    }
    /// http://www.unicode.org/reports/tr46/#ToUnicode
    pub fn to_unicode(self, domain: &str) -> (String, Result<(), Errors>) {
        let (domain, errors) = processing(domain, self);
        (domain, errors.into())
    }
}
fn is_bidi_domain(s: &str) -> bool {
    for c in s.chars() {
        if c.is_ascii_graphic() {
            continue;
        }
        match bidi_class(c) {
            BidiClass::R | BidiClass::AL | BidiClass::AN => return true,
            _ => {}
        }
    }
    false
}
/// Errors recorded during UTS #46 processing.
///
/// This is opaque for now, indicating what types of errors have been encountered at least once.
/// More details may be exposed in the future.
#[derive(Debug, Default)]
pub struct Errors {
    punycode: bool,
    validity_criteria: bool,
    disallowed_by_std3_ascii_rules: bool,
    disallowed_mapped_in_std3: bool,
    disallowed_character: bool,
    too_long_for_dns: bool,
    too_short_for_dns: bool,
}
impl From<Errors> for Result<(), Errors> {
    fn from(e: Errors) -> Result<(), Errors> {
        let failed = e.punycode || e.validity_criteria
            || e.disallowed_by_std3_ascii_rules || e.disallowed_mapped_in_std3
            || e.disallowed_character || e.too_long_for_dns || e.too_short_for_dns;
        if !failed { Ok(()) } else { Err(e) }
    }
}
impl StdError for Errors {}
impl fmt::Display for Errors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}
#[cfg(test)]
mod tests {
    use super::{find_char, Mapping};
    #[test]
    fn mapping_fast_path() {
        assert_matches!(find_char('-'), & Mapping::Valid);
        assert_matches!(find_char('.'), & Mapping::Valid);
        for c in &['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'] {
            assert_matches!(find_char(* c), & Mapping::Valid);
        }
        for c in &[
            'a',
            'b',
            'c',
            'd',
            'e',
            'f',
            'g',
            'h',
            'i',
            'j',
            'k',
            'l',
            'm',
            'n',
            'o',
            'p',
            'q',
            'r',
            's',
            't',
            'u',
            'v',
            'w',
            'x',
            'y',
            'z',
        ] {
            assert_matches!(find_char(* c), & Mapping::Valid);
        }
    }
}
#[cfg(test)]
mod tests_llm_16_1 {
    use super::*;
    use crate::*;
    #[test]
    fn test_default() {
        let _rug_st_tests_llm_16_1_rrrruuuugggg_test_default = 0;
        let config = <uts46::Config as std::default::Default>::default();
        debug_assert_eq!(config.use_std3_ascii_rules, false);
        debug_assert_eq!(config.transitional_processing, false);
        debug_assert_eq!(config.check_hyphens, false);
        debug_assert_eq!(config.verify_dns_length, false);
        let _rug_ed_tests_llm_16_1_rrrruuuugggg_test_default = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_18 {
    use super::*;
    use crate::*;
    #[test]
    fn test_check_hyphens() {
        let _rug_st_tests_llm_16_18_rrrruuuugggg_test_check_hyphens = 0;
        let rug_fuzz_0 = true;
        let config = Config::default().check_hyphens(rug_fuzz_0);
        debug_assert_eq!(config.check_hyphens, true);
        let _rug_ed_tests_llm_16_18_rrrruuuugggg_test_check_hyphens = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_19 {
    use super::*;
    use crate::*;
    use std::string::ToString;
    #[test]
    fn test_to_ascii() {
        let _rug_st_tests_llm_16_19_rrrruuuugggg_test_to_ascii = 0;
        let rug_fuzz_0 = "example.com";
        let config = Config::default();
        let domain = rug_fuzz_0;
        let result = config.to_ascii(domain).unwrap();
        debug_assert_eq!(result, "example.com".to_string());
        let _rug_ed_tests_llm_16_19_rrrruuuugggg_test_to_ascii = 0;
    }
    #[test]
    fn test_to_ascii_with_punycode() {
        let _rug_st_tests_llm_16_19_rrrruuuugggg_test_to_ascii_with_punycode = 0;
        let rug_fuzz_0 = "ünicode.com";
        let config = Config::default();
        let domain = rug_fuzz_0;
        let result = config.to_ascii(domain).unwrap();
        debug_assert_eq!(result, "xn--nicode-6qa.com".to_string());
        let _rug_ed_tests_llm_16_19_rrrruuuugggg_test_to_ascii_with_punycode = 0;
    }
    #[test]
    fn test_to_ascii_with_empty_string() {
        let _rug_st_tests_llm_16_19_rrrruuuugggg_test_to_ascii_with_empty_string = 0;
        let rug_fuzz_0 = "";
        let config = Config::default();
        let domain = rug_fuzz_0;
        let result = config.to_ascii(domain).unwrap();
        debug_assert_eq!(result, "".to_string());
        let _rug_ed_tests_llm_16_19_rrrruuuugggg_test_to_ascii_with_empty_string = 0;
    }
    #[test]
    fn test_to_ascii_with_invalid_label() {
        let _rug_st_tests_llm_16_19_rrrruuuugggg_test_to_ascii_with_invalid_label = 0;
        let rug_fuzz_0 = "example.comü";
        let config = Config::default();
        let domain = rug_fuzz_0;
        let result = config.to_ascii(domain).unwrap_err();
        debug_assert!(result.punycode);
        let _rug_ed_tests_llm_16_19_rrrruuuugggg_test_to_ascii_with_invalid_label = 0;
    }
    #[test]
    fn test_to_ascii_with_invalid_dns_length() {
        let _rug_st_tests_llm_16_19_rrrruuuugggg_test_to_ascii_with_invalid_dns_length = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = "example.com";
        let rug_fuzz_2 = "a";
        let rug_fuzz_3 = 250;
        let config = Config::default().verify_dns_length(rug_fuzz_0);
        let domain = rug_fuzz_1.to_string() + &rug_fuzz_2.repeat(rug_fuzz_3);
        let result = config.to_ascii(&domain).unwrap_err();
        debug_assert!(result.too_long_for_dns);
        let _rug_ed_tests_llm_16_19_rrrruuuugggg_test_to_ascii_with_invalid_dns_length = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_22 {
    use super::*;
    use crate::*;
    #[test]
    fn test_transitional_processing() {
        let _rug_st_tests_llm_16_22_rrrruuuugggg_test_transitional_processing = 0;
        let rug_fuzz_0 = true;
        let config = Config::default();
        let new_config = config.transitional_processing(rug_fuzz_0);
        debug_assert_eq!(new_config.transitional_processing, true);
        let _rug_ed_tests_llm_16_22_rrrruuuugggg_test_transitional_processing = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_23 {
    use super::*;
    use crate::*;
    #[test]
    fn test_use_std3_ascii_rules() {
        let _rug_st_tests_llm_16_23_rrrruuuugggg_test_use_std3_ascii_rules = 0;
        let rug_fuzz_0 = true;
        let config = Config::default().use_std3_ascii_rules(rug_fuzz_0);
        debug_assert_eq!(config.use_std3_ascii_rules, true);
        let _rug_ed_tests_llm_16_23_rrrruuuugggg_test_use_std3_ascii_rules = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_24 {
    use super::*;
    use crate::*;
    #[test]
    fn test_verify_dns_length() {
        let _rug_st_tests_llm_16_24_rrrruuuugggg_test_verify_dns_length = 0;
        let rug_fuzz_0 = true;
        let config = Config::default().verify_dns_length(rug_fuzz_0);
        debug_assert_eq!(config.verify_dns_length, true);
        let _rug_ed_tests_llm_16_24_rrrruuuugggg_test_verify_dns_length = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_25 {
    use crate::uts46::decode_slice;
    use crate::uts46::StringTableSlice;
    #[test]
    fn test_decode_slice() {
        let _rug_st_tests_llm_16_25_rrrruuuugggg_test_decode_slice = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let slice = StringTableSlice {
            byte_start_lo: rug_fuzz_0,
            byte_start_hi: rug_fuzz_1,
            byte_len: rug_fuzz_2,
        };
        debug_assert_eq!(decode_slice(& slice), "");
        let _rug_ed_tests_llm_16_25_rrrruuuugggg_test_decode_slice = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_28 {
    use crate::uts46::{is_bidi_domain, bidi_class, BidiClass};
    #[test]
    fn test_is_bidi_domain() {
        let _rug_st_tests_llm_16_28_rrrruuuugggg_test_is_bidi_domain = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = "abc";
        let rug_fuzz_2 = "123";
        let rug_fuzz_3 = "abc123";
        let rug_fuzz_4 = "العالم";
        let rug_fuzz_5 = "hello 世界";
        let rug_fuzz_6 = "abc 123 العالم";
        debug_assert_eq!(is_bidi_domain(rug_fuzz_0), false);
        debug_assert_eq!(is_bidi_domain(rug_fuzz_1), false);
        debug_assert_eq!(is_bidi_domain(rug_fuzz_2), false);
        debug_assert_eq!(is_bidi_domain(rug_fuzz_3), false);
        debug_assert_eq!(is_bidi_domain(rug_fuzz_4), true);
        debug_assert_eq!(is_bidi_domain(rug_fuzz_5), true);
        debug_assert_eq!(is_bidi_domain(rug_fuzz_6), true);
        let _rug_ed_tests_llm_16_28_rrrruuuugggg_test_is_bidi_domain = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_29 {
    use super::*;
    use crate::*;
    use uts46::Mapping;
    #[test]
    fn test_is_valid() {
        let _rug_st_tests_llm_16_29_rrrruuuugggg_test_is_valid = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = "example";
        let rug_fuzz_2 = "-example";
        let rug_fuzz_3 = "example-";
        let rug_fuzz_4 = "example.com";
        let rug_fuzz_5 = "◌example";
        let rug_fuzz_6 = "example";
        let rug_fuzz_7 = true;
        let rug_fuzz_8 = "ex❼mple";
        let rug_fuzz_9 = true;
        let rug_fuzz_10 = "ex!ample";
        let rug_fuzz_11 = "ex!ample";
        let config = Config::default();
        debug_assert!(is_valid(rug_fuzz_0, config));
        debug_assert!(is_valid(rug_fuzz_1, config));
        debug_assert!(! is_valid(rug_fuzz_2, config));
        debug_assert!(! is_valid(rug_fuzz_3, config));
        debug_assert!(! is_valid(rug_fuzz_4, config));
        debug_assert!(! is_valid(rug_fuzz_5, config));
        debug_assert!(is_valid(rug_fuzz_6, config));
        let config_deviation = Config::default().transitional_processing(rug_fuzz_7);
        debug_assert!(is_valid(rug_fuzz_8, config_deviation));
        let config_disallowed = Config::default().use_std3_ascii_rules(rug_fuzz_9);
        debug_assert!(! is_valid(rug_fuzz_10, config_disallowed));
        debug_assert!(is_valid(rug_fuzz_11, config));
        let _rug_ed_tests_llm_16_29_rrrruuuugggg_test_is_valid = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_32 {
    use crate::uts46::passes_bidi;
    #[test]
    fn test_passes_bidi() {
        let _rug_st_tests_llm_16_32_rrrruuuugggg_test_passes_bidi = 0;
        let rug_fuzz_0 = "abc";
        let rug_fuzz_1 = false;
        let rug_fuzz_2 = "xn--abc";
        let rug_fuzz_3 = false;
        let rug_fuzz_4 = "xn--abc";
        let rug_fuzz_5 = true;
        let rug_fuzz_6 = "abc";
        let rug_fuzz_7 = true;
        let rug_fuzz_8 = "abc";
        let rug_fuzz_9 = true;
        let rug_fuzz_10 = "xn--abc";
        let rug_fuzz_11 = true;
        let rug_fuzz_12 = "abc";
        let rug_fuzz_13 = true;
        let rug_fuzz_14 = "abc";
        let rug_fuzz_15 = true;
        let rug_fuzz_16 = "xn--abc";
        let rug_fuzz_17 = true;
        let rug_fuzz_18 = "abc";
        let rug_fuzz_19 = true;
        let rug_fuzz_20 = "abc";
        let rug_fuzz_21 = true;
        let rug_fuzz_22 = "xn--abc";
        let rug_fuzz_23 = true;
        let rug_fuzz_24 = "abc";
        let rug_fuzz_25 = true;
        let rug_fuzz_26 = "abc";
        let rug_fuzz_27 = true;
        let rug_fuzz_28 = "xn--abc";
        let rug_fuzz_29 = true;
        let rug_fuzz_30 = "abc";
        let rug_fuzz_31 = true;
        debug_assert_eq!(passes_bidi(rug_fuzz_0, rug_fuzz_1), true);
        debug_assert_eq!(passes_bidi(rug_fuzz_2, rug_fuzz_3), true);
        debug_assert_eq!(passes_bidi(rug_fuzz_4, rug_fuzz_5), false);
        debug_assert_eq!(passes_bidi(rug_fuzz_6, rug_fuzz_7), true);
        debug_assert_eq!(passes_bidi(rug_fuzz_8, rug_fuzz_9), true);
        debug_assert_eq!(passes_bidi(rug_fuzz_10, rug_fuzz_11), false);
        debug_assert_eq!(passes_bidi(rug_fuzz_12, rug_fuzz_13), true);
        debug_assert_eq!(passes_bidi(rug_fuzz_14, rug_fuzz_15), true);
        debug_assert_eq!(passes_bidi(rug_fuzz_16, rug_fuzz_17), false);
        debug_assert_eq!(passes_bidi(rug_fuzz_18, rug_fuzz_19), true);
        debug_assert_eq!(passes_bidi(rug_fuzz_20, rug_fuzz_21), true);
        debug_assert_eq!(passes_bidi(rug_fuzz_22, rug_fuzz_23), false);
        debug_assert_eq!(passes_bidi(rug_fuzz_24, rug_fuzz_25), true);
        debug_assert_eq!(passes_bidi(rug_fuzz_26, rug_fuzz_27), true);
        debug_assert_eq!(passes_bidi(rug_fuzz_28, rug_fuzz_29), false);
        debug_assert_eq!(passes_bidi(rug_fuzz_30, rug_fuzz_31), true);
        let _rug_ed_tests_llm_16_32_rrrruuuugggg_test_passes_bidi = 0;
    }
}
