use ast;
use hir;
use Result;
/// A builder for a regular expression parser.
///
/// This builder permits modifying configuration options for the parser.
///
/// This type combines the builder options for both the
/// [AST `ParserBuilder`](ast/parse/struct.ParserBuilder.html)
/// and the
/// [HIR `TranslatorBuilder`](hir/translate/struct.TranslatorBuilder.html).
#[derive(Clone, Debug, Default)]
pub struct ParserBuilder {
    ast: ast::parse::ParserBuilder,
    hir: hir::translate::TranslatorBuilder,
}
impl ParserBuilder {
    /// Create a new parser builder with a default configuration.
    pub fn new() -> ParserBuilder {
        ParserBuilder::default()
    }
    /// Build a parser from this configuration with the given pattern.
    pub fn build(&self) -> Parser {
        Parser {
            ast: self.ast.build(),
            hir: self.hir.build(),
        }
    }
    /// Set the nesting limit for this parser.
    ///
    /// The nesting limit controls how deep the abstract syntax tree is allowed
    /// to be. If the AST exceeds the given limit (e.g., with too many nested
    /// groups), then an error is returned by the parser.
    ///
    /// The purpose of this limit is to act as a heuristic to prevent stack
    /// overflow for consumers that do structural induction on an `Ast` using
    /// explicit recursion. While this crate never does this (instead using
    /// constant stack space and moving the call stack to the heap), other
    /// crates may.
    ///
    /// This limit is not checked until the entire Ast is parsed. Therefore,
    /// if callers want to put a limit on the amount of heap space used, then
    /// they should impose a limit on the length, in bytes, of the concrete
    /// pattern string. In particular, this is viable since this parser
    /// implementation will limit itself to heap space proportional to the
    /// lenth of the pattern string.
    ///
    /// Note that a nest limit of `0` will return a nest limit error for most
    /// patterns but not all. For example, a nest limit of `0` permits `a` but
    /// not `ab`, since `ab` requires a concatenation, which results in a nest
    /// depth of `1`. In general, a nest limit is not something that manifests
    /// in an obvious way in the concrete syntax, therefore, it should not be
    /// used in a granular way.
    pub fn nest_limit(&mut self, limit: u32) -> &mut ParserBuilder {
        self.ast.nest_limit(limit);
        self
    }
    /// Whether to support octal syntax or not.
    ///
    /// Octal syntax is a little-known way of uttering Unicode codepoints in
    /// a regular expression. For example, `a`, `\x61`, `\u0061` and
    /// `\141` are all equivalent regular expressions, where the last example
    /// shows octal syntax.
    ///
    /// While supporting octal syntax isn't in and of itself a problem, it does
    /// make good error messages harder. That is, in PCRE based regex engines,
    /// syntax like `\0` invokes a backreference, which is explicitly
    /// unsupported in Rust's regex engine. However, many users expect it to
    /// be supported. Therefore, when octal support is disabled, the error
    /// message will explicitly mention that backreferences aren't supported.
    ///
    /// Octal syntax is disabled by default.
    pub fn octal(&mut self, yes: bool) -> &mut ParserBuilder {
        self.ast.octal(yes);
        self
    }
    /// When enabled, the parser will permit the construction of a regular
    /// expression that may match invalid UTF-8.
    ///
    /// When disabled (the default), the parser is guaranteed to produce
    /// an expression that will only ever match valid UTF-8 (otherwise, the
    /// parser will return an error).
    ///
    /// Perhaps surprisingly, when invalid UTF-8 isn't allowed, a negated ASCII
    /// word boundary (uttered as `(?-u:\B)` in the concrete syntax) will cause
    /// the parser to return an error. Namely, a negated ASCII word boundary
    /// can result in matching positions that aren't valid UTF-8 boundaries.
    pub fn allow_invalid_utf8(&mut self, yes: bool) -> &mut ParserBuilder {
        self.hir.allow_invalid_utf8(yes);
        self
    }
    /// Enable verbose mode in the regular expression.
    ///
    /// When enabled, verbose mode permits insigificant whitespace in many
    /// places in the regular expression, as well as comments. Comments are
    /// started using `#` and continue until the end of the line.
    ///
    /// By default, this is disabled. It may be selectively enabled in the
    /// regular expression by using the `x` flag regardless of this setting.
    pub fn ignore_whitespace(&mut self, yes: bool) -> &mut ParserBuilder {
        self.ast.ignore_whitespace(yes);
        self
    }
    /// Enable or disable the case insensitive flag by default.
    ///
    /// By default this is disabled. It may alternatively be selectively
    /// enabled in the regular expression itself via the `i` flag.
    pub fn case_insensitive(&mut self, yes: bool) -> &mut ParserBuilder {
        self.hir.case_insensitive(yes);
        self
    }
    /// Enable or disable the multi-line matching flag by default.
    ///
    /// By default this is disabled. It may alternatively be selectively
    /// enabled in the regular expression itself via the `m` flag.
    pub fn multi_line(&mut self, yes: bool) -> &mut ParserBuilder {
        self.hir.multi_line(yes);
        self
    }
    /// Enable or disable the "dot matches any character" flag by default.
    ///
    /// By default this is disabled. It may alternatively be selectively
    /// enabled in the regular expression itself via the `s` flag.
    pub fn dot_matches_new_line(&mut self, yes: bool) -> &mut ParserBuilder {
        self.hir.dot_matches_new_line(yes);
        self
    }
    /// Enable or disable the "swap greed" flag by default.
    ///
    /// By default this is disabled. It may alternatively be selectively
    /// enabled in the regular expression itself via the `U` flag.
    pub fn swap_greed(&mut self, yes: bool) -> &mut ParserBuilder {
        self.hir.swap_greed(yes);
        self
    }
    /// Enable or disable the Unicode flag (`u`) by default.
    ///
    /// By default this is **enabled**. It may alternatively be selectively
    /// disabled in the regular expression itself via the `u` flag.
    ///
    /// Note that unless `allow_invalid_utf8` is enabled (it's disabled by
    /// default), a regular expression will fail to parse if Unicode mode is
    /// disabled and a sub-expression could possibly match invalid UTF-8.
    pub fn unicode(&mut self, yes: bool) -> &mut ParserBuilder {
        self.hir.unicode(yes);
        self
    }
}
/// A convenience parser for regular expressions.
///
/// This parser takes as input a regular expression pattern string (the
/// "concrete syntax") and returns a high-level intermediate representation
/// (the HIR) suitable for most types of analysis. In particular, this parser
/// hides the intermediate state of producing an AST (the "abstract syntax").
/// The AST is itself far more complex than the HIR, so this parser serves as a
/// convenience for never having to deal with it at all.
///
/// If callers have more fine grained use cases that need an AST, then please
/// see the [`ast::parse`](ast/parse/index.html) module.
///
/// A `Parser` can be configured in more detail via a
/// [`ParserBuilder`](struct.ParserBuilder.html).
#[derive(Clone, Debug)]
pub struct Parser {
    ast: ast::parse::Parser,
    hir: hir::translate::Translator,
}
impl Parser {
    /// Create a new parser with a default configuration.
    ///
    /// The parser can be run with `parse` method. The parse method returns
    /// a high level intermediate representation of the given regular
    /// expression.
    ///
    /// To set configuration options on the parser, use
    /// [`ParserBuilder`](struct.ParserBuilder.html).
    pub fn new() -> Parser {
        ParserBuilder::new().build()
    }
    /// Parse the regular expression into a high level intermediate
    /// representation.
    pub fn parse(&mut self, pattern: &str) -> Result<hir::Hir> {
        let ast = self.ast.parse(pattern)?;
        let hir = self.hir.translate(pattern, &ast)?;
        Ok(hir)
    }
}
#[cfg(test)]
mod tests_llm_16_588 {
    use super::*;
    use crate::*;
    #[test]
    fn test_case_insensitive() {
        let _rug_st_tests_llm_16_588_rrrruuuugggg_test_case_insensitive = 0;
        let rug_fuzz_0 = true;
        let mut builder = ParserBuilder::new();
        builder.case_insensitive(rug_fuzz_0);
        let parser = builder.build();
        let _rug_ed_tests_llm_16_588_rrrruuuugggg_test_case_insensitive = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_590 {
    use crate::parser::ParserBuilder;
    #[test]
    fn test_dot_matches_new_line() {
        let _rug_st_tests_llm_16_590_rrrruuuugggg_test_dot_matches_new_line = 0;
        let rug_fuzz_0 = true;
        let mut builder = ParserBuilder::new();
        let result = builder.dot_matches_new_line(rug_fuzz_0);
        debug_assert_eq!(
            result as * mut ParserBuilder, & mut builder as * mut ParserBuilder
        );
        let _rug_ed_tests_llm_16_590_rrrruuuugggg_test_dot_matches_new_line = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_604 {
    use super::*;
    use crate::*;
    use crate::ParserBuilder;
    #[test]
    fn test_unicode() {
        let _rug_st_tests_llm_16_604_rrrruuuugggg_test_unicode = 0;
        let rug_fuzz_0 = true;
        let mut builder = ParserBuilder::new();
        let result = builder.unicode(rug_fuzz_0);
        debug_assert_eq!(
            result as * mut ParserBuilder, & mut builder as * mut ParserBuilder
        );
        let _rug_ed_tests_llm_16_604_rrrruuuugggg_test_unicode = 0;
    }
}
#[cfg(test)]
mod tests_rug_324 {
    use super::*;
    use crate::ParserBuilder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_324_rrrruuuugggg_test_rug = 0;
        ParserBuilder::new();
        let _rug_ed_tests_rug_324_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_325 {
    use super::*;
    use crate::{ParserBuilder, Parser};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_325_rrrruuuugggg_test_rug = 0;
        let mut p0: ParserBuilder = ParserBuilder::new();
        p0.build();
        let _rug_ed_tests_rug_325_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_326_prepare {
    use crate::ParserBuilder;
    #[test]
    fn sample() {
        let _rug_st_tests_rug_326_prepare_rrrruuuugggg_sample = 0;
        let mut v104: ParserBuilder = ParserBuilder::new();
        let _rug_ed_tests_rug_326_prepare_rrrruuuugggg_sample = 0;
    }
}
#[cfg(test)]
mod tests_rug_326 {
    use super::*;
    use crate::ParserBuilder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_326_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: ParserBuilder = ParserBuilder::new();
        let mut p1: u32 = rug_fuzz_0;
        p0.nest_limit(p1);
        let _rug_ed_tests_rug_326_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_327 {
    use super::*;
    use crate::ParserBuilder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_327_rrrruuuugggg_sample = 0;
        #[cfg(test)]
        mod tests_rug_327_prepare {
            use crate::ParserBuilder;
            #[test]
            fn sample() {
                let _rug_st_tests_rug_327_prepare_rrrruuuugggg_sample = 0;
                let rug_fuzz_0 = 0;
                let rug_fuzz_1 = 0;
                let _rug_st_tests_rug_327_rrrruuuugggg_sample = rug_fuzz_0;
                let mut v104: ParserBuilder = ParserBuilder::new();
                let _rug_ed_tests_rug_327_rrrruuuugggg_sample = rug_fuzz_1;
                let _rug_ed_tests_rug_327_prepare_rrrruuuugggg_sample = 0;
            }
        }
        let mut p0: ParserBuilder = ParserBuilder::new();
        let p1: bool = true;
        p0.octal(p1);
        let _rug_ed_tests_rug_327_rrrruuuugggg_sample = 0;
    }
}
#[cfg(test)]
mod tests_rug_328 {
    use super::*;
    use crate::ParserBuilder;
    #[test]
    fn test_allow_invalid_utf8() {
        let _rug_st_tests_rug_328_rrrruuuugggg_test_allow_invalid_utf8 = 0;
        let rug_fuzz_0 = true;
        let mut p0: ParserBuilder = ParserBuilder::new();
        let p1: bool = rug_fuzz_0;
        p0.allow_invalid_utf8(p1);
        let _rug_ed_tests_rug_328_rrrruuuugggg_test_allow_invalid_utf8 = 0;
    }
}
#[cfg(test)]
mod tests_rug_329 {
    use super::*;
    use crate::ParserBuilder;
    #[test]
    fn test_ignore_whitespace() {
        let _rug_st_tests_rug_329_rrrruuuugggg_test_ignore_whitespace = 0;
        let rug_fuzz_0 = true;
        let mut p0: ParserBuilder = ParserBuilder::new();
        let p1: bool = rug_fuzz_0;
        p0.ignore_whitespace(p1);
        let _rug_ed_tests_rug_329_rrrruuuugggg_test_ignore_whitespace = 0;
    }
}
#[cfg(test)]
mod tests_rug_330 {
    use super::*;
    use crate::ParserBuilder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_330_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = true;
        let mut p0: ParserBuilder = ParserBuilder::new();
        let p1: bool = rug_fuzz_0;
        p0.multi_line(p1);
        let _rug_ed_tests_rug_330_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_331 {
    use super::*;
    use crate::ParserBuilder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_331_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = true;
        let mut p0: ParserBuilder = ParserBuilder::new();
        let p1: bool = rug_fuzz_0;
        p0.swap_greed(p1);
        let _rug_ed_tests_rug_331_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_332 {
    use super::*;
    use crate::parser::{Parser, ParserBuilder};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_332_rrrruuuugggg_test_rug = 0;
        Parser::new();
        let _rug_ed_tests_rug_332_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_333 {
    use super::*;
    use crate::{Parser, hir::Hir};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_333_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "test_pattern";
        let mut p0: Parser = Parser::new();
        let mut p1: &str = rug_fuzz_0;
        let result: Result<Hir> = p0.parse(p1);
        let _rug_ed_tests_rug_333_rrrruuuugggg_test_rug = 0;
    }
}
