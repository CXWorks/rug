use std::fmt;
use std::mem;
use self::Error::*;
use crate::lexer::{self, Lexer, Token};
use crate::version::{Identifier, Version};
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Error<'input> {
    /// Needed more tokens for parsing, but none are available.
    UnexpectedEnd,
    /// Unexpected token.
    UnexpectedToken(Token<'input>),
    /// An error occurred in the lexer.
    Lexer(lexer::Error),
    /// More input available.
    MoreInput(Vec<Token<'input>>),
    /// Encountered empty predicate in a set of predicates.
    EmptyPredicate,
    /// Encountered an empty range.
    EmptyRange,
}
impl<'input> From<lexer::Error> for Error<'input> {
    fn from(value: lexer::Error) -> Self {
        Error::Lexer(value)
    }
}
impl<'input> fmt::Display for Error<'input> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        use self::Error::*;
        match *self {
            UnexpectedEnd => write!(fmt, "expected more input"),
            UnexpectedToken(ref token) => {
                write!(fmt, "encountered unexpected token: {:?}", token)
            }
            Lexer(ref error) => write!(fmt, "lexer error: {:?}", error),
            MoreInput(ref tokens) => {
                write!(fmt, "expected end of input, but got: {:?}", tokens)
            }
            EmptyPredicate => write!(fmt, "encountered empty predicate"),
            EmptyRange => write!(fmt, "encountered empty range"),
        }
    }
}
/// impl for backwards compatibility.
impl<'input> From<Error<'input>> for String {
    fn from(value: Error<'input>) -> Self {
        value.to_string()
    }
}
/// A recursive-descent parser for parsing version requirements.
pub struct Parser<'input> {
    /// Source of token.
    lexer: Lexer<'input>,
    /// Lookaehead.
    c1: Option<Token<'input>>,
}
impl<'input> Parser<'input> {
    /// Construct a new parser for the given input.
    pub fn new(input: &'input str) -> Result<Parser<'input>, Error<'input>> {
        let mut lexer = Lexer::new(input);
        let c1 = if let Some(c1) = lexer.next() { Some(c1?) } else { None };
        Ok(Parser { lexer, c1 })
    }
    /// Pop one token.
    #[inline(always)]
    fn pop(&mut self) -> Result<Token<'input>, Error<'input>> {
        let c1 = if let Some(c1) = self.lexer.next() { Some(c1?) } else { None };
        mem::replace(&mut self.c1, c1).ok_or_else(|| UnexpectedEnd)
    }
    /// Peek one token.
    #[inline(always)]
    fn peek(&mut self) -> Option<&Token<'input>> {
        self.c1.as_ref()
    }
    /// Skip whitespace if present.
    fn skip_whitespace(&mut self) -> Result<(), Error<'input>> {
        match self.peek() {
            Some(&Token::Whitespace(_, _)) => self.pop().map(|_| ()),
            _ => Ok(()),
        }
    }
    /// Parse a single component.
    ///
    /// Returns `None` if the component is a wildcard.
    pub fn component(&mut self) -> Result<Option<u64>, Error<'input>> {
        match self.pop()? {
            Token::Numeric(number) => Ok(Some(number)),
            ref t if t.is_wildcard() => Ok(None),
            tok => Err(UnexpectedToken(tok)),
        }
    }
    /// Parse a single numeric.
    pub fn numeric(&mut self) -> Result<u64, Error<'input>> {
        match self.pop()? {
            Token::Numeric(number) => Ok(number),
            tok => Err(UnexpectedToken(tok)),
        }
    }
    /// Optionally parse a dot, then a component.
    ///
    /// The second component of the tuple indicates if a wildcard has been encountered, and is
    /// always `false` if the first component is `Some`.
    ///
    /// If a dot is not encountered, `(None, false)` is returned.
    ///
    /// If a wildcard is encountered, `(None, true)` is returned.
    pub fn dot_component(&mut self) -> Result<(Option<u64>, bool), Error<'input>> {
        match self.peek() {
            Some(&Token::Dot) => {}
            _ => return Ok((None, false)),
        }
        self.pop()?;
        self.component().map(|n| (n, n.is_none()))
    }
    /// Parse a dot, then a numeric.
    pub fn dot_numeric(&mut self) -> Result<u64, Error<'input>> {
        match self.pop()? {
            Token::Dot => {}
            tok => return Err(UnexpectedToken(tok)),
        }
        self.numeric()
    }
    /// Parse an string identifier.
    ///
    /// Like, `foo`, or `bar`, or `beta-1`.
    pub fn identifier(&mut self) -> Result<Identifier, Error<'input>> {
        let identifier = match self.pop()? {
            Token::AlphaNumeric(identifier) => {
                Identifier::AlphaNumeric(identifier.to_string())
            }
            Token::Numeric(n) => Identifier::Numeric(n),
            tok => return Err(UnexpectedToken(tok)),
        };
        if let Some(&Token::Hyphen) = self.peek() {
            self.pop()?;
            Ok(identifier.concat("-").concat(&self.identifier()?.to_string()))
        } else {
            Ok(identifier)
        }
    }
    /// Parse all pre-release identifiers, separated by dots.
    ///
    /// Like, `abcdef.1234`.
    fn pre(&mut self) -> Result<Vec<Identifier>, Error<'input>> {
        match self.peek() {
            Some(&Token::Hyphen) => {}
            _ => return Ok(vec![]),
        }
        self.pop()?;
        self.parts()
    }
    /// Parse a dot-separated set of identifiers.
    fn parts(&mut self) -> Result<Vec<Identifier>, Error<'input>> {
        let mut parts = Vec::new();
        parts.push(self.identifier()?);
        while let Some(&Token::Dot) = self.peek() {
            self.pop()?;
            parts.push(self.identifier()?);
        }
        Ok(parts)
    }
    /// Parse optional build metadata.
    ///
    /// Like, `` (empty), or `+abcdef`.
    fn plus_build_metadata(&mut self) -> Result<Vec<Identifier>, Error<'input>> {
        match self.peek() {
            Some(&Token::Plus) => {}
            _ => return Ok(vec![]),
        }
        self.pop()?;
        self.parts()
    }
    /// Parse a version.
    ///
    /// Like, `1.0.0` or `3.0.0-beta.1`.
    pub fn version(&mut self) -> Result<Version, Error<'input>> {
        self.skip_whitespace()?;
        let major = self.numeric()?;
        let minor = self.dot_numeric()?;
        let patch = self.dot_numeric()?;
        let pre = self.pre()?;
        let build = self.plus_build_metadata()?;
        self.skip_whitespace()?;
        Ok(Version {
            major,
            minor,
            patch,
            pre,
            build,
        })
    }
    /// Check if we have reached the end of input.
    pub fn is_eof(&mut self) -> bool {
        self.c1.is_none()
    }
    /// Get the rest of the tokens in the parser.
    ///
    /// Useful for debugging.
    pub fn tail(&mut self) -> Result<Vec<Token<'input>>, Error<'input>> {
        let mut out = Vec::new();
        if let Some(t) = self.c1.take() {
            out.push(t);
        }
        while let Some(t) = self.lexer.next() {
            out.push(t?);
        }
        Ok(out)
    }
}
#[cfg(test)]
mod tests_llm_16_59 {
    use super::*;
    use crate::*;
    use crate::parser::Error;
    #[test]
    fn test_from() {
        let _rug_st_tests_llm_16_59_rrrruuuugggg_test_from = 0;
        let error: Error<'static> = Error::UnexpectedEnd;
        let result: String = From::from(error);
        debug_assert_eq!(result, "expected more input");
        let _rug_ed_tests_llm_16_59_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_60 {
    use super::*;
    use crate::*;
    #[test]
    fn test_component_numeric() {
        let _rug_st_tests_llm_16_60_rrrruuuugggg_test_component_numeric = 0;
        let rug_fuzz_0 = "123";
        let rug_fuzz_1 = "123";
        let mut lexer = Lexer::new(rug_fuzz_0);
        let mut parser = Parser::new(rug_fuzz_1).unwrap();
        debug_assert_eq!(parser.component().unwrap(), Some(123));
        debug_assert_eq!(lexer.next().unwrap().unwrap(), Token::Numeric(123));
        let _rug_ed_tests_llm_16_60_rrrruuuugggg_test_component_numeric = 0;
    }
    #[test]
    fn test_component_wildcard() {
        let _rug_st_tests_llm_16_60_rrrruuuugggg_test_component_wildcard = 0;
        let rug_fuzz_0 = "*";
        let rug_fuzz_1 = "*";
        let mut lexer = Lexer::new(rug_fuzz_0);
        let mut parser = Parser::new(rug_fuzz_1).unwrap();
        debug_assert_eq!(parser.component().unwrap(), None);
        debug_assert_eq!(lexer.next().unwrap().unwrap(), Token::Star);
        let _rug_ed_tests_llm_16_60_rrrruuuugggg_test_component_wildcard = 0;
    }
    #[test]
    fn test_component_unexpected_token() {
        let _rug_st_tests_llm_16_60_rrrruuuugggg_test_component_unexpected_token = 0;
        let rug_fuzz_0 = "abc";
        let rug_fuzz_1 = "abc";
        let mut lexer = Lexer::new(rug_fuzz_0);
        let mut parser = Parser::new(rug_fuzz_1).unwrap();
        debug_assert!(parser.component().is_err());
        debug_assert_eq!(lexer.next().unwrap().unwrap(), Token::AlphaNumeric("abc"));
        let _rug_ed_tests_llm_16_60_rrrruuuugggg_test_component_unexpected_token = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_67 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_eof() {
        let _rug_st_tests_llm_16_67_rrrruuuugggg_test_is_eof = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = "1.0.0";
        let mut parser = Parser::new(rug_fuzz_0).unwrap();
        debug_assert_eq!(parser.is_eof(), true);
        let mut parser = Parser::new(rug_fuzz_1).unwrap();
        debug_assert_eq!(parser.is_eof(), false);
        let _rug_ed_tests_llm_16_67_rrrruuuugggg_test_is_eof = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_68 {
    use super::*;
    use crate::*;
    use crate::lexer::Lexer;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_68_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = "";
        let input = rug_fuzz_0;
        let result = Parser::new(input);
        let _rug_ed_tests_llm_16_68_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_72_llm_16_71 {
    use super::*;
    use crate::*;
    use crate::lexer::Lexer;
    use crate::parser::{Identifier, Parser};
    #[test]
    fn test_parts() {
        let _rug_st_tests_llm_16_72_llm_16_71_rrrruuuugggg_test_parts = 0;
        let rug_fuzz_0 = "1.0.0-beta.1";
        let rug_fuzz_1 = "1";
        let input = rug_fuzz_0;
        let mut parser = Parser::new(input).unwrap();
        let parts = parser.parts().unwrap();
        let expected_parts = vec![
            Identifier::AlphaNumeric(rug_fuzz_1.to_string()),
            Identifier::AlphaNumeric("0".to_string()), Identifier::AlphaNumeric("beta"
            .to_string()), Identifier::AlphaNumeric("1".to_string())
        ];
        debug_assert_eq!(parts, expected_parts);
        let _rug_ed_tests_llm_16_72_llm_16_71_rrrruuuugggg_test_parts = 0;
    }
    #[test]
    fn test_parts_single_identifier() {
        let _rug_st_tests_llm_16_72_llm_16_71_rrrruuuugggg_test_parts_single_identifier = 0;
        let rug_fuzz_0 = "abcdef";
        let rug_fuzz_1 = "abcdef";
        let input = rug_fuzz_0;
        let mut parser = Parser::new(input).unwrap();
        let parts = parser.parts().unwrap();
        let expected_parts = vec![Identifier::AlphaNumeric(rug_fuzz_1.to_string())];
        debug_assert_eq!(parts, expected_parts);
        let _rug_ed_tests_llm_16_72_llm_16_71_rrrruuuugggg_test_parts_single_identifier = 0;
    }
    #[test]
    fn test_parts_empty() {
        let _rug_st_tests_llm_16_72_llm_16_71_rrrruuuugggg_test_parts_empty = 0;
        let rug_fuzz_0 = "";
        let input = rug_fuzz_0;
        let mut parser = Parser::new(input).unwrap();
        let parts = parser.parts().unwrap();
        let expected_parts: Vec<Identifier> = vec![];
        debug_assert_eq!(parts, expected_parts);
        let _rug_ed_tests_llm_16_72_llm_16_71_rrrruuuugggg_test_parts_empty = 0;
    }
    #[test]
    fn test_parts_multiple_dots() {
        let _rug_st_tests_llm_16_72_llm_16_71_rrrruuuugggg_test_parts_multiple_dots = 0;
        let rug_fuzz_0 = "1.0.0-beta.1+abcdef";
        let rug_fuzz_1 = "1";
        let input = rug_fuzz_0;
        let mut parser = Parser::new(input).unwrap();
        let parts = parser.parts().unwrap();
        let expected_parts = vec![
            Identifier::AlphaNumeric(rug_fuzz_1.to_string()),
            Identifier::AlphaNumeric("0".to_string()), Identifier::AlphaNumeric("beta"
            .to_string()), Identifier::AlphaNumeric("1".to_string())
        ];
        debug_assert_eq!(parts, expected_parts);
        let _rug_ed_tests_llm_16_72_llm_16_71_rrrruuuugggg_test_parts_multiple_dots = 0;
    }
    #[test]
    fn test_parts_extra_whitespace() {
        let _rug_st_tests_llm_16_72_llm_16_71_rrrruuuugggg_test_parts_extra_whitespace = 0;
        let rug_fuzz_0 = "  1.0.0  -  beta.1  ";
        let rug_fuzz_1 = "1";
        let input = rug_fuzz_0;
        let mut parser = Parser::new(input).unwrap();
        let parts = parser.parts().unwrap();
        let expected_parts = vec![
            Identifier::AlphaNumeric(rug_fuzz_1.to_string()),
            Identifier::AlphaNumeric("0".to_string()), Identifier::AlphaNumeric("beta"
            .to_string()), Identifier::AlphaNumeric("1".to_string())
        ];
        debug_assert_eq!(parts, expected_parts);
        let _rug_ed_tests_llm_16_72_llm_16_71_rrrruuuugggg_test_parts_extra_whitespace = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_73 {
    use super::*;
    use crate::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::parser::Error;
    use crate::lexer::Error as LexerError;
    #[test]
    fn test_peek_returns_some_token() {
        let _rug_st_tests_llm_16_73_rrrruuuugggg_test_peek_returns_some_token = 0;
        let rug_fuzz_0 = "1.0.0";
        let input = rug_fuzz_0;
        let lexer = Lexer::new(input);
        let mut parser = Parser::new(input).unwrap();
        let result = parser.peek();
        debug_assert!(result.is_some());
        let _rug_ed_tests_llm_16_73_rrrruuuugggg_test_peek_returns_some_token = 0;
    }
    #[test]
    fn test_peek_returns_none_at_end_of_input() {
        let _rug_st_tests_llm_16_73_rrrruuuugggg_test_peek_returns_none_at_end_of_input = 0;
        let rug_fuzz_0 = "1.0.0";
        let input = rug_fuzz_0;
        let lexer = Lexer::new(input);
        let mut parser = Parser::new(input).unwrap();
        let _ = parser.pop();
        let _ = parser.pop();
        let _ = parser.pop();
        let result = parser.peek();
        debug_assert!(result.is_none());
        let _rug_ed_tests_llm_16_73_rrrruuuugggg_test_peek_returns_none_at_end_of_input = 0;
    }
    #[test]
    fn test_peek_returns_correct_token() {
        let _rug_st_tests_llm_16_73_rrrruuuugggg_test_peek_returns_correct_token = 0;
        let rug_fuzz_0 = "1.0.0";
        let input = rug_fuzz_0;
        let lexer = Lexer::new(input);
        let mut parser = Parser::new(input).unwrap();
        let result = parser.peek();
        debug_assert_eq!(result, Some(& Token::Numeric(1)));
        let _rug_ed_tests_llm_16_73_rrrruuuugggg_test_peek_returns_correct_token = 0;
    }
    #[test]
    fn test_peek_returns_correct_token_after_pop() {
        let _rug_st_tests_llm_16_73_rrrruuuugggg_test_peek_returns_correct_token_after_pop = 0;
        let rug_fuzz_0 = "1.0.0";
        let input = rug_fuzz_0;
        let lexer = Lexer::new(input);
        let mut parser = Parser::new(input).unwrap();
        let _ = parser.pop();
        let result = parser.peek();
        debug_assert_eq!(result, Some(& Token::Dot));
        let _rug_ed_tests_llm_16_73_rrrruuuugggg_test_peek_returns_correct_token_after_pop = 0;
    }
    #[test]
    fn test_peek_returns_none_after_pop_at_end_of_input() {
        let _rug_st_tests_llm_16_73_rrrruuuugggg_test_peek_returns_none_after_pop_at_end_of_input = 0;
        let rug_fuzz_0 = "1.0.0";
        let input = rug_fuzz_0;
        let lexer = Lexer::new(input);
        let mut parser = Parser::new(input).unwrap();
        let _ = parser.pop();
        let _ = parser.pop();
        let _ = parser.pop();
        let result = parser.peek();
        debug_assert_eq!(result, None);
        let _rug_ed_tests_llm_16_73_rrrruuuugggg_test_peek_returns_none_after_pop_at_end_of_input = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_74 {
    use crate::parser::{Parser, Error};
    use crate::lexer::{Lexer, Token};
    use crate::parser::Identifier;
    #[test]
    fn test_plus_build_metadata_empty() {
        let _rug_st_tests_llm_16_74_rrrruuuugggg_test_plus_build_metadata_empty = 0;
        let rug_fuzz_0 = "";
        let input = rug_fuzz_0;
        let mut parser = Parser::new(input).unwrap();
        let result = parser.plus_build_metadata();
        debug_assert_eq!(result, Ok(vec![]));
        let _rug_ed_tests_llm_16_74_rrrruuuugggg_test_plus_build_metadata_empty = 0;
    }
    #[test]
    fn test_plus_build_metadata_single_identifier() {
        let _rug_st_tests_llm_16_74_rrrruuuugggg_test_plus_build_metadata_single_identifier = 0;
        let rug_fuzz_0 = "+abcdef";
        let input = rug_fuzz_0;
        let mut parser = Parser::new(input).unwrap();
        let result = parser.plus_build_metadata();
        debug_assert_eq!(
            result, Ok(vec![Identifier::AlphaNumeric("abcdef".to_string())])
        );
        let _rug_ed_tests_llm_16_74_rrrruuuugggg_test_plus_build_metadata_single_identifier = 0;
    }
    #[test]
    fn test_plus_build_metadata_multiple_identifiers() {
        let _rug_st_tests_llm_16_74_rrrruuuugggg_test_plus_build_metadata_multiple_identifiers = 0;
        let rug_fuzz_0 = "+abcdef.1234";
        let input = rug_fuzz_0;
        let mut parser = Parser::new(input).unwrap();
        let result = parser.plus_build_metadata();
        debug_assert_eq!(
            result, Ok(vec![Identifier::AlphaNumeric("abcdef".to_string()),
            Identifier::Numeric(1234)])
        );
        let _rug_ed_tests_llm_16_74_rrrruuuugggg_test_plus_build_metadata_multiple_identifiers = 0;
    }
    #[test]
    fn test_plus_build_metadata_no_plus() {
        let _rug_st_tests_llm_16_74_rrrruuuugggg_test_plus_build_metadata_no_plus = 0;
        let rug_fuzz_0 = "abcdef";
        let input = rug_fuzz_0;
        let mut parser = Parser::new(input).unwrap();
        let result = parser.plus_build_metadata();
        debug_assert_eq!(result, Ok(vec![]));
        let _rug_ed_tests_llm_16_74_rrrruuuugggg_test_plus_build_metadata_no_plus = 0;
    }
    #[test]
    fn test_plus_build_metadata_no_plus_single_identifier() {
        let _rug_st_tests_llm_16_74_rrrruuuugggg_test_plus_build_metadata_no_plus_single_identifier = 0;
        let rug_fuzz_0 = "abcdef.1234";
        let input = rug_fuzz_0;
        let mut parser = Parser::new(input).unwrap();
        let result = parser.plus_build_metadata();
        debug_assert_eq!(
            result, Ok(vec![Identifier::AlphaNumeric("abcdef".to_string()),
            Identifier::Numeric(1234)])
        );
        let _rug_ed_tests_llm_16_74_rrrruuuugggg_test_plus_build_metadata_no_plus_single_identifier = 0;
    }
    #[test]
    fn test_plus_build_metadata_no_plus_multiple_identifiers() {
        let _rug_st_tests_llm_16_74_rrrruuuugggg_test_plus_build_metadata_no_plus_multiple_identifiers = 0;
        let rug_fuzz_0 = "abcdef.1234.5678";
        let input = rug_fuzz_0;
        let mut parser = Parser::new(input).unwrap();
        let result = parser.plus_build_metadata();
        debug_assert_eq!(
            result, Ok(vec![Identifier::AlphaNumeric("abcdef".to_string()),
            Identifier::Numeric(1234), Identifier::Numeric(5678)])
        );
        let _rug_ed_tests_llm_16_74_rrrruuuugggg_test_plus_build_metadata_no_plus_multiple_identifiers = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_75 {
    use super::*;
    use crate::*;
    #[test]
    fn test_pop() {
        let _rug_st_tests_llm_16_75_rrrruuuugggg_test_pop = 0;
        let rug_fuzz_0 = "";
        let input = rug_fuzz_0;
        let mut parser = Parser::new(input).unwrap();
        let result = parser.pop();
        debug_assert_eq!(result, Err(UnexpectedEnd));
        let _rug_ed_tests_llm_16_75_rrrruuuugggg_test_pop = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_77 {
    use super::*;
    use crate::*;
    use crate::version::Identifier;
    #[test]
    fn test_pre_no_hyphen() {
        let _rug_st_tests_llm_16_77_rrrruuuugggg_test_pre_no_hyphen = 0;
        let rug_fuzz_0 = "1.0.0";
        let input = rug_fuzz_0;
        let mut parser = Parser::new(input).unwrap();
        let result = parser.pre();
        debug_assert_eq!(result, Ok(vec![]));
        let _rug_ed_tests_llm_16_77_rrrruuuugggg_test_pre_no_hyphen = 0;
    }
    #[test]
    fn test_pre_with_hyphen() {
        let _rug_st_tests_llm_16_77_rrrruuuugggg_test_pre_with_hyphen = 0;
        let rug_fuzz_0 = "-abcdef.1234";
        let input = rug_fuzz_0;
        let mut parser = Parser::new(input).unwrap();
        let result = parser.pre();
        debug_assert_eq!(
            result, Ok(vec![Identifier::AlphaNumeric("abcdef".to_string()),
            Identifier::Numeric(1234)])
        );
        let _rug_ed_tests_llm_16_77_rrrruuuugggg_test_pre_with_hyphen = 0;
    }
    #[test]
    fn test_pre_empty() {
        let _rug_st_tests_llm_16_77_rrrruuuugggg_test_pre_empty = 0;
        let rug_fuzz_0 = "-.1";
        let input = rug_fuzz_0;
        let mut parser = Parser::new(input).unwrap();
        let result = parser.pre();
        debug_assert_eq!(result, Ok(vec![]));
        let _rug_ed_tests_llm_16_77_rrrruuuugggg_test_pre_empty = 0;
    }
    #[test]
    fn test_pre_multipe_identifiers() {
        let _rug_st_tests_llm_16_77_rrrruuuugggg_test_pre_multipe_identifiers = 0;
        let rug_fuzz_0 = "-alpha.beta.1";
        let input = rug_fuzz_0;
        let mut parser = Parser::new(input).unwrap();
        let result = parser.pre();
        debug_assert_eq!(
            result, Ok(vec![Identifier::AlphaNumeric("alpha".to_string()),
            Identifier::AlphaNumeric("beta".to_string()), Identifier::Numeric(1)])
        );
        let _rug_ed_tests_llm_16_77_rrrruuuugggg_test_pre_multipe_identifiers = 0;
    }
    #[test]
    fn test_pre_with_hyphen_and_plus() {
        let _rug_st_tests_llm_16_77_rrrruuuugggg_test_pre_with_hyphen_and_plus = 0;
        let rug_fuzz_0 = "-abcdef.1234+test";
        let input = rug_fuzz_0;
        let mut parser = Parser::new(input).unwrap();
        let result = parser.pre();
        debug_assert_eq!(
            result, Ok(vec![Identifier::AlphaNumeric("abcdef".to_string()),
            Identifier::Numeric(1234)])
        );
        let _rug_ed_tests_llm_16_77_rrrruuuugggg_test_pre_with_hyphen_and_plus = 0;
    }
    #[test]
    fn test_pre_with_hyphen_and_plus_empty() {
        let _rug_st_tests_llm_16_77_rrrruuuugggg_test_pre_with_hyphen_and_plus_empty = 0;
        let rug_fuzz_0 = "-.1+test";
        let input = rug_fuzz_0;
        let mut parser = Parser::new(input).unwrap();
        let result = parser.pre();
        debug_assert_eq!(result, Ok(vec![]));
        let _rug_ed_tests_llm_16_77_rrrruuuugggg_test_pre_with_hyphen_and_plus_empty = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_78 {
    use super::*;
    use crate::*;
    #[test]
    fn test_skip_whitespace() {
        let _rug_st_tests_llm_16_78_rrrruuuugggg_test_skip_whitespace = 0;
        let rug_fuzz_0 = "  \t  \n  foo";
        let rug_fuzz_1 = "foo";
        let input = rug_fuzz_0;
        let mut parser = Parser::new(input).unwrap();
        parser.skip_whitespace().unwrap();
        let expected_tail = vec![Token::AlphaNumeric(rug_fuzz_1)];
        debug_assert_eq!(parser.tail().unwrap(), expected_tail);
        let _rug_ed_tests_llm_16_78_rrrruuuugggg_test_skip_whitespace = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_79 {
    use super::*;
    use crate::*;
    use crate::lexer::Lexer;
    #[test]
    fn test_tail() {
        let _rug_st_tests_llm_16_79_rrrruuuugggg_test_tail = 0;
        let rug_fuzz_0 = "";
        let input = rug_fuzz_0;
        let lexer = Lexer::new(input);
        let mut parser = Parser::new(input).unwrap();
        let expected = Vec::new();
        let result = parser.tail();
        debug_assert_eq!(result, Ok(expected));
        let _rug_ed_tests_llm_16_79_rrrruuuugggg_test_tail = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_81_llm_16_80 {
    use crate::parser::Parser;
    use crate::lexer::Lexer;
    use crate::parser::Error;
    use crate::parser::Token;
    use crate::parser::Version;
    use crate::parser::Identifier;
    #[test]
    fn test_version() {
        let _rug_st_tests_llm_16_81_llm_16_80_rrrruuuugggg_test_version = 0;
        let rug_fuzz_0 = "1.0.0";
        let input = rug_fuzz_0;
        let mut parser = Parser::new(input).unwrap();
        let res = parser.version();
        debug_assert_eq!(
            res, Ok(Version { major : 1, minor : 0, patch : 0, pre : vec![], build :
            vec![], })
        );
        let _rug_ed_tests_llm_16_81_llm_16_80_rrrruuuugggg_test_version = 0;
    }
    #[test]
    fn test_version_with_pre() {
        let _rug_st_tests_llm_16_81_llm_16_80_rrrruuuugggg_test_version_with_pre = 0;
        let rug_fuzz_0 = "3.0.0-beta.1";
        let input = rug_fuzz_0;
        let mut parser = Parser::new(input).unwrap();
        let res = parser.version();
        debug_assert_eq!(
            res, Ok(Version { major : 3, minor : 0, patch : 0, pre :
            vec![Identifier::AlphaNumeric("beta".to_string()), Identifier::Numeric(1)],
            build : vec![], })
        );
        let _rug_ed_tests_llm_16_81_llm_16_80_rrrruuuugggg_test_version_with_pre = 0;
    }
    #[test]
    fn test_version_with_pre_and_build() {
        let _rug_st_tests_llm_16_81_llm_16_80_rrrruuuugggg_test_version_with_pre_and_build = 0;
        let rug_fuzz_0 = "1.2.3-beta.1+build.456";
        let input = rug_fuzz_0;
        let mut parser = Parser::new(input).unwrap();
        let res = parser.version();
        debug_assert_eq!(
            res, Ok(Version { major : 1, minor : 2, patch : 3, pre :
            vec![Identifier::AlphaNumeric("beta".to_string()), Identifier::Numeric(1)],
            build : vec![Identifier::AlphaNumeric("build".to_string()),
            Identifier::Numeric(456)], })
        );
        let _rug_ed_tests_llm_16_81_llm_16_80_rrrruuuugggg_test_version_with_pre_and_build = 0;
    }
    #[test]
    fn test_version_with_whitespace() {
        let _rug_st_tests_llm_16_81_llm_16_80_rrrruuuugggg_test_version_with_whitespace = 0;
        let rug_fuzz_0 = "  1.0.0  ";
        let input = rug_fuzz_0;
        let mut parser = Parser::new(input).unwrap();
        let res = parser.version();
        debug_assert_eq!(
            res, Ok(Version { major : 1, minor : 0, patch : 0, pre : vec![], build :
            vec![], })
        );
        let _rug_ed_tests_llm_16_81_llm_16_80_rrrruuuugggg_test_version_with_whitespace = 0;
    }
    #[test]
    fn test_version_with_invalid_input() {
        let _rug_st_tests_llm_16_81_llm_16_80_rrrruuuugggg_test_version_with_invalid_input = 0;
        let rug_fuzz_0 = "1.0.0abc";
        let input = rug_fuzz_0;
        let mut parser = Parser::new(input).unwrap();
        let res = parser.version();
        debug_assert_eq!(res, Err(Error::UnexpectedToken(Token::AlphaNumeric("abc"))));
        let _rug_ed_tests_llm_16_81_llm_16_80_rrrruuuugggg_test_version_with_invalid_input = 0;
    }
}
#[cfg(test)]
mod tests_rug_31 {
    use super::*;
    use crate::parser::Parser;
    use crate::parser::Error;
    use crate::parser::Token;
    #[test]
    fn test_numeric() {
        let _rug_st_tests_rug_31_rrrruuuugggg_test_numeric = 0;
        let rug_fuzz_0 = "1.2.3";
        let rug_fuzz_1 = "abc";
        let mut p0: Parser<'static> = Parser::new(rug_fuzz_0).unwrap();
        debug_assert_eq!(Parser::numeric(& mut p0).unwrap(), 1);
        let mut p1: Parser<'static> = Parser::new(rug_fuzz_1).unwrap();
        debug_assert_eq!(
            Parser::numeric(& mut p1).unwrap_err(),
            Error::UnexpectedToken(Token::AlphaNumeric("abc"))
        );
        let _rug_ed_tests_rug_31_rrrruuuugggg_test_numeric = 0;
    }
}
