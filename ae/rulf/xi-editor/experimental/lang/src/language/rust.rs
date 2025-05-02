//! Rust language syntax analysis and highlighting.
use std::io::{stdin, Read};
use crate::parser::Parser;
use crate::peg::*;
use crate::statestack::{Context, State};
use crate::ScopeId;
/// See [this](https://github.com/sublimehq/Packages/blob/master/Rust/Rust.sublime-syntax)
/// for reference.
static ALL_SCOPES: &[&[&str]] = &[
    &["source.rust"],
    &["source.rust", "string.quoted.double.rust"],
    &["source.rust", "string.quoted.single.rust"],
    &["source.rust", "comment.line.double-slash.rust"],
    &["source.rust", "constant.character.escape.rust"],
    &["source.rust", "constant.numeric.decimal.rust"],
    &["source.rust", "invalid.illegal.rust"],
    &["source.rust", "keyword.operator.rust"],
    &["source.rust", "keyword.operator.arithmetic.rust"],
    &["source.rust", "entity.name.type.rust"],
];
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum StateEl {
    Source,
    StrQuote,
    CharQuote,
    Comment,
    CharConst,
    NumericLiteral,
    Invalid,
    Keyword,
    Operator,
    PrimType,
}
impl StateEl {
    pub fn scope_id(&self) -> ScopeId {
        match self {
            StateEl::Source => 0,
            StateEl::StrQuote => 1,
            StateEl::CharQuote => 2,
            StateEl::Comment => 3,
            StateEl::CharConst => 4,
            StateEl::NumericLiteral => 5,
            StateEl::Invalid => 6,
            StateEl::Keyword => 7,
            StateEl::Operator => 8,
            StateEl::PrimType => 9,
        }
    }
}
const RUST_KEYWORDS: &[&[u8]] = &[
    b"Self",
    b"abstract",
    b"alignof",
    b"as",
    b"become",
    b"box",
    b"break",
    b"const",
    b"continue",
    b"crate",
    b"default",
    b"do",
    b"else",
    b"enum",
    b"extern",
    b"false",
    b"final",
    b"fn",
    b"for",
    b"if",
    b"impl",
    b"in",
    b"let",
    b"loop",
    b"macro",
    b"match",
    b"mod",
    b"move",
    b"mut",
    b"offsetof",
    b"override",
    b"priv",
    b"proc",
    b"pub",
    b"pure",
    b"ref",
    b"return",
    b"self",
    b"sizeof",
    b"static",
    b"struct",
    b"super",
    b"trait",
    b"true",
    b"type",
    b"typeof",
    b"union",
    b"unsafe",
    b"unsized",
    b"use",
    b"virtual",
    b"where",
    b"while",
    b"yield",
];
const RUST_PRIM_TYPES: &[&[u8]] = &[
    b"bool",
    b"char",
    b"f32",
    b"f64",
    b"i128",
    b"i16",
    b"i32",
    b"i64",
    b"i8",
    b"isize",
    b"str",
    b"u128",
    b"u16",
    b"u32",
    b"u64",
    b"u8",
    b"usize",
];
const RUST_OPERATORS: &[&[u8]] = &[
    b"!",
    b"%=",
    b"%",
    b"&=",
    b"&&",
    b"&",
    b"*=",
    b"*",
    b"+=",
    b"+",
    b"-=",
    b"-",
    b"/=",
    b"/",
    b"<<=",
    b"<<",
    b">>=",
    b">>",
    b"^=",
    b"^",
    b"|=",
    b"||",
    b"|",
    b"==",
    b"=",
    b"..",
    b"=>",
    b"<=",
    b"<",
    b">=",
    b">",
];
pub struct RustParser {
    scope_offset: Option<u32>,
    ctx: Context<StateEl>,
}
impl RustParser {
    pub fn new() -> RustParser {
        RustParser {
            scope_offset: None,
            ctx: Context::new(),
        }
    }
    fn quoted_str(&mut self, t: &[u8], state: State) -> (usize, State, usize, State) {
        let mut i = 0;
        while i < t.len() {
            let b = t[i];
            if b == b'"' {
                return (0, state, i + 1, self.ctx.pop(state).unwrap());
            } else if b == b'\\' {
                if let Some(len) = escape.p(&t[i..]) {
                    return (i, self.ctx.push(state, StateEl::CharConst), len, state);
                } else if let Some(len)
                    = (FailIf(OneOf(b"\r\nbu")), OneChar(|_| true)).p(&t[i + 1..])
                {
                    return (i + 1, self.ctx.push(state, StateEl::Invalid), len, state);
                }
            }
            i += 1;
        }
        (0, state, i, state)
    }
}
impl Parser for RustParser {
    fn has_offset(&mut self) -> bool {
        self.scope_offset.is_some()
    }
    fn set_scope_offset(&mut self, offset: u32) {
        if !self.has_offset() {
            self.scope_offset = Some(offset);
        }
    }
    fn get_all_scopes(&self) -> Vec<Vec<String>> {
        ALL_SCOPES
            .iter()
            .map(|stack| stack.iter().map(|s| (*s).to_string()).collect::<Vec<_>>())
            .collect()
    }
    fn get_scope_id_for_state(&self, state: State) -> ScopeId {
        let offset = self.scope_offset.unwrap_or_default();
        if let Some(element) = self.ctx.tos(state) {
            element.scope_id() + offset
        } else {
            offset
        }
    }
    fn parse(&mut self, text: &str, mut state: State) -> (usize, State, usize, State) {
        let t = text.as_bytes();
        match self.ctx.tos(state) {
            Some(StateEl::Comment) => {
                for i in 0..t.len() {
                    if let Some(len) = "/*".p(&t[i..]) {
                        state = self.ctx.push(state, StateEl::Comment);
                        return (i, state, len, state);
                    } else if let Some(len) = "*/".p(&t[i..]) {
                        return (0, state, i + len, self.ctx.pop(state).unwrap());
                    }
                }
                return (0, state, t.len(), state);
            }
            Some(StateEl::StrQuote) => return self.quoted_str(t, state),
            _ => {}
        }
        let mut i = 0;
        while i < t.len() {
            let b = t[i];
            if let Some(len) = "/*".p(&t[i..]) {
                state = self.ctx.push(state, StateEl::Comment);
                return (i, state, len, state);
            } else if "//".p(&t[i..]).is_some() {
                return (i, self.ctx.push(state, StateEl::Comment), t.len(), state);
            } else if let Some(len) = numeric_literal.p(&t[i..]) {
                return (i, self.ctx.push(state, StateEl::NumericLiteral), len, state);
            } else if b == b'"' {
                state = self.ctx.push(state, StateEl::StrQuote);
                return (i, state, 1, state);
            } else if let Some(len) = char_literal.p(&t[i..]) {
                return (i, self.ctx.push(state, StateEl::CharQuote), len, state);
            } else if let Some(len) = OneOf(RUST_OPERATORS).p(&t[i..]) {
                return (i, self.ctx.push(state, StateEl::Operator), len, state);
            } else if let Some(len) = ident.p(&t[i..]) {
                if RUST_KEYWORDS.binary_search(&&t[i..i + len]).is_ok() {
                    return (i, self.ctx.push(state, StateEl::Keyword), len, state);
                } else if RUST_PRIM_TYPES.binary_search(&&t[i..i + len]).is_ok() {
                    return (i, self.ctx.push(state, StateEl::PrimType), len, state);
                } else {
                    i += len;
                    continue;
                }
            } else if let Some(len) = whitespace.p(&t[i..]) {
                return (i, self.ctx.push(state, StateEl::Source), len, state);
            }
            i += 1;
        }
        (0, self.ctx.push(state, StateEl::Source), t.len(), state)
    }
}
fn is_digit(c: u8) -> bool {
    c >= b'0' && c <= b'9'
}
fn is_hex_digit(c: u8) -> bool {
    (c >= b'0' && c <= b'9') || (c >= b'a' && c <= b'f') || (c >= b'A' && c <= b'F')
}
fn is_ident_start(c: u8) -> bool {
    (c >= b'A' && c <= b'Z') || (c >= b'a' && c <= b'z') || c == b'_'
}
fn is_ident_continue(c: u8) -> bool {
    is_ident_start(c) || is_digit(c)
}
fn ident(s: &[u8]) -> Option<usize> {
    (OneByte(is_ident_start), ZeroOrMore(OneByte(is_ident_continue))).p(s)
}
fn raw_numeric(s: &[u8]) -> Option<usize> {
    (OneByte(is_digit), ZeroOrMore(Alt(b'_', OneByte(is_digit)))).p(s)
}
fn int_suffix(s: &[u8]) -> Option<usize> {
    (Alt(b'u', b'i'), OneOf(&["8", "16", "32", "64", "128", "size"])).p(s)
}
struct OneOrMoreWithSep<P, SEP>(P, SEP);
impl<P: Peg, SEP: Peg> Peg for OneOrMoreWithSep<P, SEP> {
    fn p(&self, s: &[u8]) -> Option<usize> {
        let OneOrMoreWithSep(ref p, ref sep) = *self;
        (ZeroOrMore(Ref(sep)), Ref(p), ZeroOrMore(Alt(Ref(p), Ref(sep)))).p(s)
    }
}
fn positive_nondecimal(s: &[u8]) -> Option<usize> {
    (
        b'0',
        Alt3(
            (b'x', OneOrMoreWithSep(OneByte(is_hex_digit), b'_')),
            (b'o', OneOrMoreWithSep(Inclusive(b'0'..b'7'), b'_')),
            (b'b', OneOrMoreWithSep(Alt(b'0', b'1'), b'_')),
        ),
        Optional(int_suffix),
    )
        .p(s)
}
fn positive_decimal(s: &[u8]) -> Option<usize> {
    (
        raw_numeric,
        Alt(
            int_suffix,
            (
                Optional((b'.', FailIf(OneByte(is_ident_start)), Optional(raw_numeric))),
                Optional((Alt(b'e', b'E'), Optional(Alt(b'+', b'-')), raw_numeric)),
                Optional(Alt("f32", "f64")),
            ),
        ),
    )
        .p(s)
}
fn numeric_literal(s: &[u8]) -> Option<usize> {
    (Optional(b'-'), Alt(positive_nondecimal, positive_decimal)).p(s)
}
fn escape(s: &[u8]) -> Option<usize> {
    (
        b'\\',
        Alt3(
            OneOf(b"\\\'\"0nrt"),
            (b'x', Repeat(OneByte(is_hex_digit), 2)),
            ("u{", Repeat(OneByte(is_hex_digit), 1..7), b'}'),
        ),
    )
        .p(s)
}
fn char_literal(s: &[u8]) -> Option<usize> {
    (b'\'', Alt(OneChar(|c| c != '\\' && c != '\''), escape), b'\'').p(s)
}
fn whitespace(s: &[u8]) -> Option<usize> {
    (OneOrMore(OneOf(&[b' ', b'\t', b'\n', b'\r', 0x0B, 0x0C]))).p(s)
}
pub fn test() {
    let mut buf = String::new();
    let _ = stdin().read_to_string(&mut buf).unwrap();
    let mut c = RustParser::new();
    let mut state = State::default();
    for line in buf.lines() {
        let mut i = 0;
        while i < line.len() {
            let (prevlen, s0, len, s1) = c.parse(&line[i..], state);
            if prevlen > 0 {
                println!("{}: {:?}", & line[i..i + prevlen], state);
                i += prevlen;
            }
            println!("{}: {:?}", & line[i..i + len], s0);
            i += len;
            state = s1;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::numeric_literal;
    #[test]
    fn numeric_literals() {
        assert_eq!(Some(1), numeric_literal(b"2.f64"));
        assert_eq!(Some(6), numeric_literal(b"2.0f64"));
        assert_eq!(Some(1), numeric_literal(b"2._f64"));
        assert_eq!(Some(1), numeric_literal(b"2._0f64"));
        assert_eq!(Some(5), numeric_literal(b"1_2__"));
        assert_eq!(Some(7), numeric_literal(b"1_2__u8"));
        assert_eq!(Some(9), numeric_literal(b"1_2__u128"));
        assert_eq!(None, numeric_literal(b"_1_"));
        assert_eq!(Some(4), numeric_literal(b"0xff"));
        assert_eq!(Some(4), numeric_literal(b"0o6789"));
    }
}
#[cfg(test)]
mod tests_llm_16_40 {
    use super::*;
    use crate::*;
    use crate::parser::Parser;
    #[test]
    fn test_get_all_scopes() {
        let _rug_st_tests_llm_16_40_rrrruuuugggg_test_get_all_scopes = 0;
        let rug_fuzz_0 = "scope1";
        let rust_parser = RustParser::new();
        let all_scopes = rust_parser.get_all_scopes();
        let expected_scopes: Vec<Vec<String>> = vec![
            vec![rug_fuzz_0.to_string(), "scope2".to_string()], vec!["scope3"
            .to_string(), "scope4".to_string()]
        ];
        debug_assert_eq!(all_scopes, expected_scopes);
        let _rug_ed_tests_llm_16_40_rrrruuuugggg_test_get_all_scopes = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_43 {
    use super::*;
    use crate::*;
    use language::rust::RustParser;
    use parser::Parser;
    #[test]
    fn test_has_offset() {
        let _rug_st_tests_llm_16_43_rrrruuuugggg_test_has_offset = 0;
        let rug_fuzz_0 = 10;
        let mut parser = RustParser::new();
        debug_assert_eq!(parser.has_offset(), false);
        parser.set_scope_offset(rug_fuzz_0);
        debug_assert_eq!(parser.has_offset(), true);
        let _rug_ed_tests_llm_16_43_rrrruuuugggg_test_has_offset = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_94 {
    use super::*;
    use crate::*;
    #[test]
    fn test_scope_id() {
        let _rug_st_tests_llm_16_94_rrrruuuugggg_test_scope_id = 0;
        debug_assert_eq!(StateEl::Source.scope_id(), 0);
        debug_assert_eq!(StateEl::StrQuote.scope_id(), 1);
        debug_assert_eq!(StateEl::CharQuote.scope_id(), 2);
        debug_assert_eq!(StateEl::Comment.scope_id(), 3);
        debug_assert_eq!(StateEl::CharConst.scope_id(), 4);
        debug_assert_eq!(StateEl::NumericLiteral.scope_id(), 5);
        debug_assert_eq!(StateEl::Invalid.scope_id(), 6);
        debug_assert_eq!(StateEl::Keyword.scope_id(), 7);
        debug_assert_eq!(StateEl::Operator.scope_id(), 8);
        debug_assert_eq!(StateEl::PrimType.scope_id(), 9);
        let _rug_ed_tests_llm_16_94_rrrruuuugggg_test_scope_id = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_95 {
    use super::*;
    use crate::*;
    #[test]
    fn test_char_literal() {
        let _rug_st_tests_llm_16_95_rrrruuuugggg_test_char_literal = 0;
        let rug_fuzz_0 = b"'A'";
        let rug_fuzz_1 = b"'\\'";
        let rug_fuzz_2 = b"'\\n'";
        let rug_fuzz_3 = b"'\\u{1F600}'";
        let rug_fuzz_4 = b"'\\u{1F600";
        let rug_fuzz_5 = b"'";
        let rug_fuzz_6 = b"A'";
        debug_assert_eq!(char_literal(rug_fuzz_0), Some(3));
        debug_assert_eq!(char_literal(rug_fuzz_1), Some(4));
        debug_assert_eq!(char_literal(rug_fuzz_2), Some(4));
        debug_assert_eq!(char_literal(rug_fuzz_3), Some(12));
        debug_assert_eq!(char_literal(rug_fuzz_4), None);
        debug_assert_eq!(char_literal(rug_fuzz_5), None);
        debug_assert_eq!(char_literal(rug_fuzz_6), None);
        let _rug_ed_tests_llm_16_95_rrrruuuugggg_test_char_literal = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_96 {
    use super::*;
    use crate::*;
    #[test]
    fn test_escape() {
        let _rug_st_tests_llm_16_96_rrrruuuugggg_test_escape = 0;
        let rug_fuzz_0 = b"test";
        debug_assert_eq!(escape(rug_fuzz_0), Some(4));
        let _rug_ed_tests_llm_16_96_rrrruuuugggg_test_escape = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_97 {
    use super::*;
    use crate::*;
    use crate::language::rust::is_ident_start;
    use crate::language::rust::is_ident_continue;
    #[test]
    fn test_ident() {
        let _rug_st_tests_llm_16_97_rrrruuuugggg_test_ident = 0;
        let rug_fuzz_0 = b"string";
        let rug_fuzz_1 = b"ident_123";
        let rug_fuzz_2 = b"123";
        let rug_fuzz_3 = b"_underscore";
        let rug_fuzz_4 = b"";
        debug_assert_eq!(ident(rug_fuzz_0), Some(6));
        debug_assert_eq!(ident(rug_fuzz_1), Some(9));
        debug_assert_eq!(ident(rug_fuzz_2), None);
        debug_assert_eq!(ident(rug_fuzz_3), Some(10));
        debug_assert_eq!(ident(rug_fuzz_4), None);
        let _rug_ed_tests_llm_16_97_rrrruuuugggg_test_ident = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_99_llm_16_98 {
    use super::*;
    use crate::*;
    #[test]
    fn test_int_suffix() {
        let _rug_st_tests_llm_16_99_llm_16_98_rrrruuuugggg_test_int_suffix = 0;
        let rug_fuzz_0 = b'u';
        let rug_fuzz_1 = b'8';
        let rug_fuzz_2 = b'u';
        let rug_fuzz_3 = b'1';
        let rug_fuzz_4 = b'6';
        let rug_fuzz_5 = b'u';
        let rug_fuzz_6 = b'3';
        let rug_fuzz_7 = b'2';
        let rug_fuzz_8 = b'u';
        let rug_fuzz_9 = b'6';
        let rug_fuzz_10 = b'4';
        let rug_fuzz_11 = b'u';
        let rug_fuzz_12 = b'1';
        let rug_fuzz_13 = b'2';
        let rug_fuzz_14 = b'8';
        let rug_fuzz_15 = b'u';
        let rug_fuzz_16 = b's';
        let rug_fuzz_17 = b'i';
        let rug_fuzz_18 = b'z';
        let rug_fuzz_19 = b'e';
        let rug_fuzz_20 = b'u';
        let rug_fuzz_21 = b's';
        let rug_fuzz_22 = b'i';
        let rug_fuzz_23 = b'u';
        let rug_fuzz_24 = b's';
        let input1: &[u8] = &[rug_fuzz_0, rug_fuzz_1];
        debug_assert_eq!(int_suffix(input1), Some(2));
        let input2: &[u8] = &[rug_fuzz_2, rug_fuzz_3, rug_fuzz_4];
        debug_assert_eq!(int_suffix(input2), Some(3));
        let input3: &[u8] = &[rug_fuzz_5, rug_fuzz_6, rug_fuzz_7];
        debug_assert_eq!(int_suffix(input3), Some(3));
        let input4: &[u8] = &[rug_fuzz_8, rug_fuzz_9, rug_fuzz_10];
        debug_assert_eq!(int_suffix(input4), Some(3));
        let input5: &[u8] = &[rug_fuzz_11, rug_fuzz_12, rug_fuzz_13, rug_fuzz_14];
        debug_assert_eq!(int_suffix(input5), Some(4));
        let input6: &[u8] = &[
            rug_fuzz_15,
            rug_fuzz_16,
            rug_fuzz_17,
            rug_fuzz_18,
            rug_fuzz_19,
        ];
        debug_assert_eq!(int_suffix(input6), Some(5));
        let input7: &[u8] = &[rug_fuzz_20, rug_fuzz_21, rug_fuzz_22];
        debug_assert_eq!(int_suffix(input7), None);
        let input8: &[u8] = &[rug_fuzz_23, rug_fuzz_24];
        debug_assert_eq!(int_suffix(input8), None);
        let _rug_ed_tests_llm_16_99_llm_16_98_rrrruuuugggg_test_int_suffix = 0;
    }
}
#[test]
fn test_is_digit() {
    assert!(crate ::language::rust::is_digit(b'0'));
    assert!(crate ::language::rust::is_digit(b'9'));
    assert!(! crate ::language::rust::is_digit(b'A'));
    assert!(! crate ::language::rust::is_digit(b'Z'));
}
#[cfg(test)]
mod tests_llm_16_102 {
    use crate::language::rust::is_hex_digit;
    #[test]
    fn test_is_hex_digit() {
        let _rug_st_tests_llm_16_102_rrrruuuugggg_test_is_hex_digit = 0;
        let rug_fuzz_0 = b'0';
        let rug_fuzz_1 = b'9';
        let rug_fuzz_2 = b'a';
        let rug_fuzz_3 = b'f';
        let rug_fuzz_4 = b'A';
        let rug_fuzz_5 = b'F';
        let rug_fuzz_6 = b'g';
        let rug_fuzz_7 = b'G';
        let rug_fuzz_8 = b' ';
        let rug_fuzz_9 = b'!';
        let rug_fuzz_10 = b'@';
        debug_assert_eq!(is_hex_digit(rug_fuzz_0), true);
        debug_assert_eq!(is_hex_digit(rug_fuzz_1), true);
        debug_assert_eq!(is_hex_digit(rug_fuzz_2), true);
        debug_assert_eq!(is_hex_digit(rug_fuzz_3), true);
        debug_assert_eq!(is_hex_digit(rug_fuzz_4), true);
        debug_assert_eq!(is_hex_digit(rug_fuzz_5), true);
        debug_assert_eq!(is_hex_digit(rug_fuzz_6), false);
        debug_assert_eq!(is_hex_digit(rug_fuzz_7), false);
        debug_assert_eq!(is_hex_digit(rug_fuzz_8), false);
        debug_assert_eq!(is_hex_digit(rug_fuzz_9), false);
        debug_assert_eq!(is_hex_digit(rug_fuzz_10), false);
        let _rug_ed_tests_llm_16_102_rrrruuuugggg_test_is_hex_digit = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_104_llm_16_103 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_ident_continue() {
        let _rug_st_tests_llm_16_104_llm_16_103_rrrruuuugggg_test_is_ident_continue = 0;
        let rug_fuzz_0 = b'a';
        let rug_fuzz_1 = b'5';
        let rug_fuzz_2 = b'#';
        let rug_fuzz_3 = b'_';
        let rug_fuzz_4 = b' ';
        let rug_fuzz_5 = 255;
        let rug_fuzz_6 = 0;
        debug_assert_eq!(is_ident_continue(rug_fuzz_0), true);
        debug_assert_eq!(is_ident_continue(rug_fuzz_1), true);
        debug_assert_eq!(is_ident_continue(rug_fuzz_2), false);
        debug_assert_eq!(is_ident_continue(rug_fuzz_3), true);
        debug_assert_eq!(is_ident_continue(rug_fuzz_4), false);
        debug_assert_eq!(is_ident_continue(rug_fuzz_5), false);
        debug_assert_eq!(is_ident_continue(rug_fuzz_6), false);
        let _rug_ed_tests_llm_16_104_llm_16_103_rrrruuuugggg_test_is_ident_continue = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_106_llm_16_105 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_ident_start() {
        let _rug_st_tests_llm_16_106_llm_16_105_rrrruuuugggg_test_is_ident_start = 0;
        let rug_fuzz_0 = b'A';
        let rug_fuzz_1 = b'Z';
        let rug_fuzz_2 = b'a';
        let rug_fuzz_3 = b'z';
        let rug_fuzz_4 = b'_';
        let rug_fuzz_5 = b'0';
        let rug_fuzz_6 = b'9';
        let rug_fuzz_7 = b'$';
        let rug_fuzz_8 = b' ';
        let rug_fuzz_9 = b'\n';
        debug_assert_eq!(is_ident_start(rug_fuzz_0), true);
        debug_assert_eq!(is_ident_start(rug_fuzz_1), true);
        debug_assert_eq!(is_ident_start(rug_fuzz_2), true);
        debug_assert_eq!(is_ident_start(rug_fuzz_3), true);
        debug_assert_eq!(is_ident_start(rug_fuzz_4), true);
        debug_assert_eq!(is_ident_start(rug_fuzz_5), false);
        debug_assert_eq!(is_ident_start(rug_fuzz_6), false);
        debug_assert_eq!(is_ident_start(rug_fuzz_7), false);
        debug_assert_eq!(is_ident_start(rug_fuzz_8), false);
        debug_assert_eq!(is_ident_start(rug_fuzz_9), false);
        let _rug_ed_tests_llm_16_106_llm_16_105_rrrruuuugggg_test_is_ident_start = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_109 {
    use crate::language::rust::positive_decimal;
    #[test]
    fn test_positive_decimal() {
        let _rug_st_tests_llm_16_109_rrrruuuugggg_test_positive_decimal = 0;
        let rug_fuzz_0 = b"3.14159";
        let rug_fuzz_1 = 8;
        let input: &[u8] = rug_fuzz_0;
        let expected_output: Option<usize> = Some(rug_fuzz_1);
        debug_assert_eq!(positive_decimal(input), expected_output);
        let _rug_ed_tests_llm_16_109_rrrruuuugggg_test_positive_decimal = 0;
    }
}
#[test]
fn test_raw_numeric() {
    assert_eq!(raw_numeric(b"12345"), Some(5));
    assert_eq!(raw_numeric(b"0"), Some(1));
    assert_eq!(raw_numeric(b"123_456"), Some(7));
    assert_eq!(raw_numeric(b"abc"), None);
    assert_eq!(raw_numeric(b"_"), None);
    assert_eq!(raw_numeric(b"123.45"), None);
    assert_eq!(raw_numeric(b"12_3"), None);
    assert_eq!(raw_numeric(b"1_2_3"), None);
    assert_eq!(raw_numeric(b"12_"), None);
}
#[cfg(test)]
mod tests_llm_16_115_llm_16_114 {
    use std::io::stdin;
    use std::io::Read;
    use crate::language::rust::{RustParser, State, test};
    #[test]
    fn test_function() {
        let _rug_st_tests_llm_16_115_llm_16_114_rrrruuuugggg_test_function = 0;
        let rug_fuzz_0 = "test input string";
        let input = rug_fuzz_0;
        let mut input_bytes = Vec::new();
        stdin().read_to_end(&mut input_bytes).unwrap();
        let mut input_str = String::new();
        input_str.push_str(std::str::from_utf8(&input_bytes).unwrap());
        stdin().read_to_string(&mut input_str).unwrap();
        test();
        let _rug_ed_tests_llm_16_115_llm_16_114_rrrruuuugggg_test_function = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_116 {
    use super::*;
    use crate::*;
    #[test]
    fn test_whitespace() {
        let _rug_st_tests_llm_16_116_rrrruuuugggg_test_whitespace = 0;
        let rug_fuzz_0 = b" ";
        let rug_fuzz_1 = b"\t";
        let rug_fuzz_2 = b"\n";
        let rug_fuzz_3 = b"\r";
        let rug_fuzz_4 = 0x0B;
        let rug_fuzz_5 = 0x0C;
        let rug_fuzz_6 = b"";
        let rug_fuzz_7 = b"abc";
        let rug_fuzz_8 = b"12 ";
        let rug_fuzz_9 = b"test\t";
        debug_assert_eq!(whitespace(rug_fuzz_0), Some(1));
        debug_assert_eq!(whitespace(rug_fuzz_1), Some(1));
        debug_assert_eq!(whitespace(rug_fuzz_2), Some(1));
        debug_assert_eq!(whitespace(rug_fuzz_3), Some(1));
        debug_assert_eq!(whitespace(& [rug_fuzz_4]), Some(1));
        debug_assert_eq!(whitespace(& [rug_fuzz_5]), Some(1));
        debug_assert_eq!(whitespace(rug_fuzz_6), None);
        debug_assert_eq!(whitespace(rug_fuzz_7), None);
        debug_assert_eq!(whitespace(rug_fuzz_8), None);
        debug_assert_eq!(whitespace(rug_fuzz_9), None);
        let _rug_ed_tests_llm_16_116_rrrruuuugggg_test_whitespace = 0;
    }
}
