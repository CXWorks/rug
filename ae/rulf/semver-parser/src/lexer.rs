//! Lexer for semver ranges.
//!
//! Breaks a string of input into an iterator of tokens that can be used with a parser.
//!
//! This should be used with the [`parser`] module.
//!
//! [`parser`]: ../parser/index.html
//!
//! # Examples
//!
//! Example without errors:
//!
//! ```rust
//! use semver_parser::lexer::{Lexer, Token};
//!
//! let mut l = Lexer::new("foo 123 *");
//!
//! assert_eq!(Some(Ok(Token::AlphaNumeric("foo"))), l.next());
//! assert_eq!(Some(Ok(Token::Whitespace(3, 4))), l.next());
//! assert_eq!(Some(Ok(Token::Numeric(123))), l.next());
//! assert_eq!(Some(Ok(Token::Whitespace(7, 8))), l.next());
//! assert_eq!(Some(Ok(Token::Star)), l.next());
//! assert_eq!(None, l.next());
//! ```
//!
//! Example with error:
//!
//! ```rust
//! use semver_parser::lexer::{Lexer, Token, Error};
//!
//! let mut l = Lexer::new("foo / *");
//!
//! assert_eq!(Some(Ok(Token::AlphaNumeric("foo"))), l.next());
//! assert_eq!(Some(Ok(Token::Whitespace(3, 4))), l.next());
//! assert_eq!(Some(Err(Error::UnexpectedChar('/'))), l.next());
//! ```
use self::Error::*;
use self::Token::*;
use std::str;
macro_rules! scan_while {
    ($slf:expr, $start:expr, $first:pat $(| $rest:pat)*) => {
        { let mut __end = $start; loop { if let Some((idx, c)) = $slf .one() { __end =
        idx; match c { $first $(| $rest)* => $slf .step(), _ => break, } continue; } else
        { __end = $slf .input.len(); } break; } __end }
    };
}
/// Semver tokens.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Token<'input> {
    /// `=`
    Eq,
    /// `>`
    Gt,
    /// `<`
    Lt,
    /// `<=`
    LtEq,
    /// `>=`
    GtEq,
    /// '^`
    Caret,
    /// '~`
    Tilde,
    /// '*`
    Star,
    /// `.`
    Dot,
    /// `,`
    Comma,
    /// `-`
    Hyphen,
    /// `+`
    Plus,
    /// '||'
    Or,
    /// any number of whitespace (`\t\r\n `) and its span.
    Whitespace(usize, usize),
    /// Numeric component, like `0` or `42`.
    Numeric(u64),
    /// Alphanumeric component, like `alpha1` or `79deadbe`.
    AlphaNumeric(&'input str),
}
impl<'input> Token<'input> {
    /// Check if the current token is a whitespace token.
    pub fn is_whitespace(&self) -> bool {
        match *self {
            Whitespace(..) => true,
            _ => false,
        }
    }
    /// Check if the current token is a wildcard token.
    pub fn is_wildcard(&self) -> bool {
        match *self {
            Star | AlphaNumeric("X") | AlphaNumeric("x") => true,
            _ => false,
        }
    }
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Error {
    /// Unexpected character.
    UnexpectedChar(char),
}
/// Lexer for semver tokens belonging to a range.
#[derive(Debug)]
pub struct Lexer<'input> {
    input: &'input str,
    chars: str::CharIndices<'input>,
    c1: Option<(usize, char)>,
    c2: Option<(usize, char)>,
}
impl<'input> Lexer<'input> {
    /// Construct a new lexer for the given input.
    pub fn new(input: &str) -> Lexer {
        let mut chars = input.char_indices();
        let c1 = chars.next();
        let c2 = chars.next();
        Lexer { input, chars, c1, c2 }
    }
    /// Shift all lookahead storage by one.
    fn step(&mut self) {
        self.c1 = self.c2;
        self.c2 = self.chars.next();
    }
    fn step_n(&mut self, n: usize) {
        for _ in 0..n {
            self.step();
        }
    }
    /// Access the one character, or set it if it is not set.
    fn one(&mut self) -> Option<(usize, char)> {
        self.c1
    }
    /// Access two characters.
    fn two(&mut self) -> Option<(usize, char, char)> {
        self.c1.and_then(|(start, c1)| self.c2.map(|(_, c2)| (start, c1, c2)))
    }
    /// Consume a component.
    ///
    /// A component can either be an alphanumeric or numeric.
    /// Does not permit leading zeroes if numeric.
    fn component(&mut self, start: usize) -> Result<Token<'input>, Error> {
        let end = scan_while!(self, start, '0'..='9' | 'A'..='Z' | 'a'..='z');
        let input = &self.input[start..end];
        let mut it = input.chars();
        let (a, b) = (it.next(), it.next());
        if a == Some('0') && b.is_none() {
            return Ok(Numeric(0));
        }
        if a != Some('0') {
            if let Ok(numeric) = input.parse::<u64>() {
                return Ok(Numeric(numeric));
            }
        }
        Ok(AlphaNumeric(input))
    }
    /// Consume whitespace.
    fn whitespace(&mut self, start: usize) -> Result<Token<'input>, Error> {
        let end = scan_while!(self, start, ' ' | '\t' | '\n' | '\r');
        Ok(Whitespace(start, end))
    }
}
impl<'input> Iterator for Lexer<'input> {
    type Item = Result<Token<'input>, Error>;
    fn next(&mut self) -> Option<Self::Item> {
        #[allow(clippy::never_loop)]
        loop {
            if let Some((_, a, b)) = self.two() {
                let two = match (a, b) {
                    ('<', '=') => Some(LtEq),
                    ('>', '=') => Some(GtEq),
                    ('|', '|') => Some(Or),
                    _ => None,
                };
                if let Some(two) = two {
                    self.step_n(2);
                    return Some(Ok(two));
                }
            }
            if let Some((start, c)) = self.one() {
                let tok = match c {
                    ' ' | '\t' | '\n' | '\r' => {
                        self.step();
                        return Some(self.whitespace(start));
                    }
                    '=' => Eq,
                    '>' => Gt,
                    '<' => Lt,
                    '^' => Caret,
                    '~' => Tilde,
                    '*' => Star,
                    '.' => Dot,
                    ',' => Comma,
                    '-' => Hyphen,
                    '+' => Plus,
                    '0'..='9' | 'a'..='z' | 'A'..='Z' => {
                        self.step();
                        return Some(self.component(start));
                    }
                    c => return Some(Err(UnexpectedChar(c))),
                };
                self.step();
                return Some(Ok(tok));
            }
            return None;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn lex(input: &str) -> Vec<Token> {
        Lexer::new(input).map(Result::unwrap).collect::<Vec<_>>()
    }
    #[test]
    pub fn simple_tokens() {
        assert_eq!(
            lex("=><<=>=^~*.,-+||"), vec![Eq, Gt, Lt, LtEq, GtEq, Caret, Tilde, Star,
            Dot, Comma, Hyphen, Plus, Or,]
        );
    }
    #[test]
    pub fn whitespace() {
        assert_eq!(
            lex("  foo \t\n\rbar"), vec![Whitespace(0, 2), AlphaNumeric("foo"),
            Whitespace(5, 9), AlphaNumeric("bar"),]
        );
    }
    #[test]
    pub fn components() {
        assert_eq!(lex("42"), vec![Numeric(42)]);
        assert_eq!(lex("0"), vec![Numeric(0)]);
        assert_eq!(lex("01"), vec![AlphaNumeric("01")]);
        assert_eq!(lex("01"), vec![AlphaNumeric("01")]);
        assert_eq!(lex("5885644aa"), vec![AlphaNumeric("5885644aa")]);
        assert_eq!(lex("beta2"), vec![AlphaNumeric("beta2")]);
        assert_eq!(lex("beta.2"), vec![AlphaNumeric("beta"), Dot, Numeric(2)]);
    }
    #[test]
    pub fn is_wildcard() {
        assert_eq!(Star.is_wildcard(), true);
        assert_eq!(AlphaNumeric("x").is_wildcard(), true);
        assert_eq!(AlphaNumeric("X").is_wildcard(), true);
        assert_eq!(AlphaNumeric("other").is_wildcard(), false);
    }
    #[test]
    pub fn empty() {
        assert_eq!(lex(""), vec![]);
    }
    #[test]
    pub fn numeric_all_numbers() {
        let expected: Vec<Token> = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]
            .into_iter()
            .map(Numeric)
            .collect::<Vec<_>>();
        let actual: Vec<_> = lex("0 1 2 3 4 5 6 7 8 9")
            .into_iter()
            .filter(|t| !t.is_whitespace())
            .collect();
        assert_eq!(actual, expected);
    }
}
#[cfg(test)]
mod tests_llm_16_51_llm_16_50 {
    use super::*;
    use crate::*;
    use crate::lexer::{Token, Error};
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_51_llm_16_50_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = "1.0.0-alpha";
        let input = rug_fuzz_0;
        let lexer = Lexer::new(input);
        let expected_input = input;
        let expected_chars: std::str::CharIndices<'_> = input.char_indices();
        let expected_c1 = expected_chars.clone().next();
        let expected_c2 = expected_chars.clone().next();
        debug_assert_eq!(lexer.input, expected_input);
        debug_assert_eq!(
            lexer.chars.collect:: < Vec < _ > > (), expected_chars.collect:: < Vec < _ >
            > ()
        );
        debug_assert_eq!(lexer.c1, expected_c1);
        debug_assert_eq!(lexer.c2, expected_c2);
        let _rug_ed_tests_llm_16_51_llm_16_50_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_52 {
    use super::*;
    use crate::*;
    #[test]
    fn test_one() {
        let _rug_st_tests_llm_16_52_rrrruuuugggg_test_one = 0;
        let rug_fuzz_0 = "1.2.3";
        let rug_fuzz_1 = 2;
        let mut lexer = Lexer::new(rug_fuzz_0);
        debug_assert_eq!(lexer.one(), Some((0, '1')));
        lexer.step();
        debug_assert_eq!(lexer.one(), Some((1, '.')));
        lexer.step_n(rug_fuzz_1);
        debug_assert_eq!(lexer.one(), None);
        lexer.step();
        debug_assert_eq!(lexer.one(), Some((3, '3')));
        let _rug_ed_tests_llm_16_52_rrrruuuugggg_test_one = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_53 {
    use super::*;
    use crate::*;
    #[test]
    fn test_lexer_step() {
        let _rug_st_tests_llm_16_53_rrrruuuugggg_test_lexer_step = 0;
        let rug_fuzz_0 = "test";
        let mut lexer = Lexer::new(rug_fuzz_0);
        lexer.step();
        debug_assert_eq!(lexer.c1, Some((0, 't')));
        debug_assert_eq!(lexer.c2, Some((1, 'e')));
        let _rug_ed_tests_llm_16_53_rrrruuuugggg_test_lexer_step = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_54 {
    use super::*;
    use crate::*;
    use crate::lexer::Token;
    #[test]
    fn test_step_n() {
        let _rug_st_tests_llm_16_54_rrrruuuugggg_test_step_n = 0;
        let rug_fuzz_0 = "=1.2.3";
        let rug_fuzz_1 = 3;
        let input = rug_fuzz_0;
        let mut lexer = Lexer::new(input);
        lexer.step_n(rug_fuzz_1);
        debug_assert_eq!(lexer.next(), Some(Ok(Token::Eq)));
        debug_assert_eq!(lexer.next(), Some(Ok(Token::Numeric(1))));
        debug_assert_eq!(lexer.next(), Some(Ok(Token::Dot)));
        debug_assert_eq!(lexer.next(), Some(Ok(Token::Numeric(2))));
        debug_assert_eq!(lexer.next(), Some(Ok(Token::Dot)));
        debug_assert_eq!(lexer.next(), Some(Ok(Token::Numeric(3))));
        debug_assert_eq!(lexer.next(), None);
        let _rug_ed_tests_llm_16_54_rrrruuuugggg_test_step_n = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_55 {
    use super::*;
    use crate::*;
    #[test]
    fn test_two() {
        let _rug_st_tests_llm_16_55_rrrruuuugggg_test_two = 0;
        let rug_fuzz_0 = "input";
        let mut lexer = Lexer::new(rug_fuzz_0);
        let result = lexer.two();
        debug_assert_eq!(result, None);
        let _rug_ed_tests_llm_16_55_rrrruuuugggg_test_two = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_56 {
    use super::*;
    use crate::*;
    use crate::lexer::*;
    use crate::lexer::Token::*;
    #[test]
    fn test_whitespace() {
        let _rug_st_tests_llm_16_56_rrrruuuugggg_test_whitespace = 0;
        let rug_fuzz_0 = "   \t \n \r";
        let rug_fuzz_1 = 0;
        let mut lexer = Lexer::new(rug_fuzz_0);
        let token = lexer.whitespace(rug_fuzz_1).unwrap();
        debug_assert_eq!(token, Whitespace(0, 9));
        let _rug_ed_tests_llm_16_56_rrrruuuugggg_test_whitespace = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_57 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_whitespace_true() {
        let _rug_st_tests_llm_16_57_rrrruuuugggg_test_is_whitespace_true = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 5;
        let token = Token::Whitespace(rug_fuzz_0, rug_fuzz_1);
        debug_assert!(token.is_whitespace());
        let _rug_ed_tests_llm_16_57_rrrruuuugggg_test_is_whitespace_true = 0;
    }
    #[test]
    fn test_is_whitespace_false() {
        let _rug_st_tests_llm_16_57_rrrruuuugggg_test_is_whitespace_false = 0;
        let token = Token::Eq;
        debug_assert!(! token.is_whitespace());
        let _rug_ed_tests_llm_16_57_rrrruuuugggg_test_is_whitespace_false = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_58 {
    use super::*;
    use crate::*;
    use lexer::Token;
    #[test]
    fn test_is_wildcard_star() {
        let _rug_st_tests_llm_16_58_rrrruuuugggg_test_is_wildcard_star = 0;
        let token = Token::Star;
        debug_assert!(token.is_wildcard());
        let _rug_ed_tests_llm_16_58_rrrruuuugggg_test_is_wildcard_star = 0;
    }
    #[test]
    fn test_is_wildcard_x() {
        let _rug_st_tests_llm_16_58_rrrruuuugggg_test_is_wildcard_x = 0;
        let rug_fuzz_0 = "X";
        let token = Token::AlphaNumeric(rug_fuzz_0);
        debug_assert!(token.is_wildcard());
        let _rug_ed_tests_llm_16_58_rrrruuuugggg_test_is_wildcard_x = 0;
    }
    #[test]
    fn test_is_wildcard_lowercase_x() {
        let _rug_st_tests_llm_16_58_rrrruuuugggg_test_is_wildcard_lowercase_x = 0;
        let rug_fuzz_0 = "x";
        let token = Token::AlphaNumeric(rug_fuzz_0);
        debug_assert!(token.is_wildcard());
        let _rug_ed_tests_llm_16_58_rrrruuuugggg_test_is_wildcard_lowercase_x = 0;
    }
    #[test]
    fn test_is_wildcard_eq() {
        let _rug_st_tests_llm_16_58_rrrruuuugggg_test_is_wildcard_eq = 0;
        let token = Token::Eq;
        debug_assert!(! token.is_wildcard());
        let _rug_ed_tests_llm_16_58_rrrruuuugggg_test_is_wildcard_eq = 0;
    }
    #[test]
    fn test_is_wildcard_gt() {
        let _rug_st_tests_llm_16_58_rrrruuuugggg_test_is_wildcard_gt = 0;
        let token = Token::Gt;
        debug_assert!(! token.is_wildcard());
        let _rug_ed_tests_llm_16_58_rrrruuuugggg_test_is_wildcard_gt = 0;
    }
    #[test]
    fn test_is_wildcard_lt() {
        let _rug_st_tests_llm_16_58_rrrruuuugggg_test_is_wildcard_lt = 0;
        let token = Token::Lt;
        debug_assert!(! token.is_wildcard());
        let _rug_ed_tests_llm_16_58_rrrruuuugggg_test_is_wildcard_lt = 0;
    }
    #[test]
    fn test_is_wildcard_lteq() {
        let _rug_st_tests_llm_16_58_rrrruuuugggg_test_is_wildcard_lteq = 0;
        let token = Token::LtEq;
        debug_assert!(! token.is_wildcard());
        let _rug_ed_tests_llm_16_58_rrrruuuugggg_test_is_wildcard_lteq = 0;
    }
    #[test]
    fn test_is_wildcard_gteq() {
        let _rug_st_tests_llm_16_58_rrrruuuugggg_test_is_wildcard_gteq = 0;
        let token = Token::GtEq;
        debug_assert!(! token.is_wildcard());
        let _rug_ed_tests_llm_16_58_rrrruuuugggg_test_is_wildcard_gteq = 0;
    }
    #[test]
    fn test_is_wildcard_caret() {
        let _rug_st_tests_llm_16_58_rrrruuuugggg_test_is_wildcard_caret = 0;
        let token = Token::Caret;
        debug_assert!(! token.is_wildcard());
        let _rug_ed_tests_llm_16_58_rrrruuuugggg_test_is_wildcard_caret = 0;
    }
    #[test]
    fn test_is_wildcard_tilde() {
        let _rug_st_tests_llm_16_58_rrrruuuugggg_test_is_wildcard_tilde = 0;
        let token = Token::Tilde;
        debug_assert!(! token.is_wildcard());
        let _rug_ed_tests_llm_16_58_rrrruuuugggg_test_is_wildcard_tilde = 0;
    }
    #[test]
    fn test_is_wildcard_dot() {
        let _rug_st_tests_llm_16_58_rrrruuuugggg_test_is_wildcard_dot = 0;
        let token = Token::Dot;
        debug_assert!(! token.is_wildcard());
        let _rug_ed_tests_llm_16_58_rrrruuuugggg_test_is_wildcard_dot = 0;
    }
    #[test]
    fn test_is_wildcard_comma() {
        let _rug_st_tests_llm_16_58_rrrruuuugggg_test_is_wildcard_comma = 0;
        let token = Token::Comma;
        debug_assert!(! token.is_wildcard());
        let _rug_ed_tests_llm_16_58_rrrruuuugggg_test_is_wildcard_comma = 0;
    }
    #[test]
    fn test_is_wildcard_hyphen() {
        let _rug_st_tests_llm_16_58_rrrruuuugggg_test_is_wildcard_hyphen = 0;
        let token = Token::Hyphen;
        debug_assert!(! token.is_wildcard());
        let _rug_ed_tests_llm_16_58_rrrruuuugggg_test_is_wildcard_hyphen = 0;
    }
    #[test]
    fn test_is_wildcard_plus() {
        let _rug_st_tests_llm_16_58_rrrruuuugggg_test_is_wildcard_plus = 0;
        let token = Token::Plus;
        debug_assert!(! token.is_wildcard());
        let _rug_ed_tests_llm_16_58_rrrruuuugggg_test_is_wildcard_plus = 0;
    }
    #[test]
    fn test_is_wildcard_or() {
        let _rug_st_tests_llm_16_58_rrrruuuugggg_test_is_wildcard_or = 0;
        let token = Token::Or;
        debug_assert!(! token.is_wildcard());
        let _rug_ed_tests_llm_16_58_rrrruuuugggg_test_is_wildcard_or = 0;
    }
    #[test]
    fn test_is_wildcard_whitespace() {
        let _rug_st_tests_llm_16_58_rrrruuuugggg_test_is_wildcard_whitespace = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 5;
        let token = Token::Whitespace(rug_fuzz_0, rug_fuzz_1);
        debug_assert!(! token.is_wildcard());
        let _rug_ed_tests_llm_16_58_rrrruuuugggg_test_is_wildcard_whitespace = 0;
    }
    #[test]
    fn test_is_wildcard_numeric() {
        let _rug_st_tests_llm_16_58_rrrruuuugggg_test_is_wildcard_numeric = 0;
        let rug_fuzz_0 = 42;
        let token = Token::Numeric(rug_fuzz_0);
        debug_assert!(! token.is_wildcard());
        let _rug_ed_tests_llm_16_58_rrrruuuugggg_test_is_wildcard_numeric = 0;
    }
    #[test]
    fn test_is_wildcard_alphanumeric() {
        let _rug_st_tests_llm_16_58_rrrruuuugggg_test_is_wildcard_alphanumeric = 0;
        let rug_fuzz_0 = "test";
        let token = Token::AlphaNumeric(rug_fuzz_0);
        debug_assert!(! token.is_wildcard());
        let _rug_ed_tests_llm_16_58_rrrruuuugggg_test_is_wildcard_alphanumeric = 0;
    }
}
#[cfg(test)]
mod tests_rug_28 {
    use super::*;
    use crate::lexer;
    #[test]
    fn test_component() {
        let _rug_st_tests_rug_28_rrrruuuugggg_test_component = 0;
        let rug_fuzz_0 = "1.2.3";
        let rug_fuzz_1 = 0;
        let mut p0: lexer::Lexer<'static> = lexer::Lexer::new(rug_fuzz_0);
        let p1: usize = rug_fuzz_1;
        p0.component(p1);
        let _rug_ed_tests_rug_28_rrrruuuugggg_test_component = 0;
    }
}
#[cfg(test)]
mod tests_rug_29 {
    use super::*;
    use crate::lexer;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_29_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "1.2.3";
        let mut v9: lexer::Lexer<'static> = lexer::Lexer::new(rug_fuzz_0);
        lexer::Lexer::next(&mut v9);
        let _rug_ed_tests_rug_29_rrrruuuugggg_test_rug = 0;
    }
}
