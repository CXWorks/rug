/// The set of user configurable options for compiling zero or more regexes.
#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub struct RegexOptions {
    pub pats: Vec<String>,
    pub size_limit: usize,
    pub dfa_size_limit: usize,
    pub nest_limit: u32,
    pub case_insensitive: bool,
    pub multi_line: bool,
    pub dot_matches_new_line: bool,
    pub swap_greed: bool,
    pub ignore_whitespace: bool,
    pub unicode: bool,
    pub octal: bool,
}
impl Default for RegexOptions {
    fn default() -> Self {
        RegexOptions {
            pats: vec![],
            size_limit: 10 * (1 << 20),
            dfa_size_limit: 2 * (1 << 20),
            nest_limit: 250,
            case_insensitive: false,
            multi_line: false,
            dot_matches_new_line: false,
            swap_greed: false,
            ignore_whitespace: false,
            unicode: true,
            octal: false,
        }
    }
}
macro_rules! define_builder {
    ($name:ident, $regex_mod:ident, $only_utf8:expr) => {
        pub mod $name { use super::RegexOptions; use error::Error; use exec::ExecBuilder;
        use $regex_mod ::Regex; #[doc =
        " A configurable builder for a regular expression."] #[doc = ""] #[doc =
        " A builder can be used to configure how the regex is built, for example, by"]
        #[doc = " setting the default flags (which can be overridden in the expression"]
        #[doc = " itself) or setting various limits."] #[derive(Debug)] pub struct
        RegexBuilder(RegexOptions); impl RegexBuilder { #[doc =
        " Create a new regular expression builder with the given pattern."] #[doc = ""]
        #[doc = " If the pattern is invalid, then an error will be returned when"] #[doc
        = " `build` is called."] pub fn new(pattern : & str) -> RegexBuilder { let mut
        builder = RegexBuilder(RegexOptions::default()); builder.0.pats.push(pattern
        .to_owned()); builder } #[doc =
        " Consume the builder and compile the regular expression."] #[doc = ""] #[doc =
        " Note that calling `as_str` on the resulting `Regex` will produce the"] #[doc =
        " pattern given to `new` verbatim. Notably, it will not incorporate any"] #[doc =
        " of the flags set on this builder."] pub fn build(& self) -> Result < Regex,
        Error > { ExecBuilder::new_options(self.0.clone()).only_utf8($only_utf8).build()
        .map(Regex::from) } #[doc =
        " Set the value for the case insensitive (`i`) flag."] #[doc = ""] #[doc =
        " When enabled, letters in the pattern will match both upper case and"] #[doc =
        " lower case variants."] pub fn case_insensitive(& mut self, yes : bool,) -> &
        mut RegexBuilder { self.0.case_insensitive = yes; self } #[doc =
        " Set the value for the multi-line matching (`m`) flag."] #[doc = ""] #[doc =
        " When enabled, `^` matches the beginning of lines and `$` matches the"] #[doc =
        " end of lines."] #[doc = ""] #[doc =
        " By default, they match beginning/end of the input."] pub fn multi_line(& mut
        self, yes : bool) -> & mut RegexBuilder { self.0.multi_line = yes; self } #[doc =
        " Set the value for the any character (`s`) flag, where in `.` matches"] #[doc =
        " anything when `s` is set and matches anything except for new line when"] #[doc
        = " it is not set (the default)."] #[doc = ""] #[doc =
        " N.B. \"matches anything\" means \"any byte\" when Unicode is disabled and"]
        #[doc = " means \"any valid UTF-8 encoding of any Unicode scalar value\" when"]
        #[doc = " Unicode is enabled."] pub fn dot_matches_new_line(& mut self, yes :
        bool,) -> & mut RegexBuilder { self.0.dot_matches_new_line = yes; self } #[doc =
        " Set the value for the greedy swap (`U`) flag."] #[doc = ""] #[doc =
        " When enabled, a pattern like `a*` is lazy (tries to find shortest"] #[doc =
        " match) and `a*?` is greedy (tries to find longest match)."] #[doc = ""] #[doc =
        " By default, `a*` is greedy and `a*?` is lazy."] pub fn swap_greed(& mut self,
        yes : bool) -> & mut RegexBuilder { self.0.swap_greed = yes; self } #[doc =
        " Set the value for the ignore whitespace (`x`) flag."] #[doc = ""] #[doc =
        " When enabled, whitespace such as new lines and spaces will be ignored"] #[doc =
        " between expressions of the pattern, and `#` can be used to start a"] #[doc =
        " comment until the next new line."] pub fn ignore_whitespace(& mut self, yes :
        bool,) -> & mut RegexBuilder { self.0.ignore_whitespace = yes; self } #[doc =
        " Set the value for the Unicode (`u`) flag."] #[doc = ""] #[doc =
        " Enabled by default. When disabled, character classes such as `\\w` only"] #[doc
        = " match ASCII word characters instead of all Unicode word characters."] pub fn
        unicode(& mut self, yes : bool) -> & mut RegexBuilder { self.0.unicode = yes;
        self } #[doc = " Whether to support octal syntax or not."] #[doc = ""] #[doc =
        " Octal syntax is a little-known way of uttering Unicode codepoints in"] #[doc =
        " a regular expression. For example, `a`, `\\x61`, `\\u0061` and"] #[doc =
        " `\\141` are all equivalent regular expressions, where the last example"] #[doc
        = " shows octal syntax."] #[doc = ""] #[doc =
        " While supporting octal syntax isn't in and of itself a problem, it does"] #[doc
        = " make good error messages harder. That is, in PCRE based regex engines,"]
        #[doc = " syntax like `\\0` invokes a backreference, which is explicitly"] #[doc
        = " unsupported in Rust's regex engine. However, many users expect it to"] #[doc
        = " be supported. Therefore, when octal support is disabled, the error"] #[doc =
        " message will explicitly mention that backreferences aren't supported."] #[doc =
        ""] #[doc = " Octal syntax is disabled by default."] pub fn octal(& mut self, yes
        : bool) -> & mut RegexBuilder { self.0.octal = yes; self } #[doc =
        " Set the approximate size limit of the compiled regular expression."] #[doc =
        ""] #[doc =
        " This roughly corresponds to the number of bytes occupied by a single"] #[doc =
        " compiled program. If the program exceeds this number, then a"] #[doc =
        " compilation error is returned."] pub fn size_limit(& mut self, limit : usize,)
        -> & mut RegexBuilder { self.0.size_limit = limit; self } #[doc =
        " Set the approximate size of the cache used by the DFA."] #[doc = ""] #[doc =
        " This roughly corresponds to the number of bytes that the DFA will"] #[doc =
        " use while searching."] #[doc = ""] #[doc =
        " Note that this is a *per thread* limit. There is no way to set a global"] #[doc
        = " limit. In particular, if a regex is used from multiple threads"] #[doc =
        " simultaneously, then each thread may use up to the number of bytes"] #[doc =
        " specified here."] pub fn dfa_size_limit(& mut self, limit : usize,) -> & mut
        RegexBuilder { self.0.dfa_size_limit = limit; self } #[doc =
        " Set the nesting limit for this parser."] #[doc = ""] #[doc =
        " The nesting limit controls how deep the abstract syntax tree is allowed"] #[doc
        = " to be. If the AST exceeds the given limit (e.g., with too many nested"] #[doc
        = " groups), then an error is returned by the parser."] #[doc = ""] #[doc =
        " The purpose of this limit is to act as a heuristic to prevent stack"] #[doc =
        " overflow for consumers that do structural induction on an `Ast` using"] #[doc =
        " explicit recursion. While this crate never does this (instead using"] #[doc =
        " constant stack space and moving the call stack to the heap), other"] #[doc =
        " crates may."] #[doc = ""] #[doc =
        " This limit is not checked until the entire Ast is parsed. Therefore,"] #[doc =
        " if callers want to put a limit on the amount of heap space used, then"] #[doc =
        " they should impose a limit on the length, in bytes, of the concrete"] #[doc =
        " pattern string. In particular, this is viable since this parser"] #[doc =
        " implementation will limit itself to heap space proportional to the"] #[doc =
        " length of the pattern string."] #[doc = ""] #[doc =
        " Note that a nest limit of `0` will return a nest limit error for most"] #[doc =
        " patterns but not all. For example, a nest limit of `0` permits `a` but"] #[doc
        = " not `ab`, since `ab` requires a concatenation, which results in a nest"]
        #[doc =
        " depth of `1`. In general, a nest limit is not something that manifests"] #[doc
        = " in an obvious way in the concrete syntax, therefore, it should not be"] #[doc
        = " used in a granular way."] pub fn nest_limit(& mut self, limit : u32) -> & mut
        RegexBuilder { self.0.nest_limit = limit; self } } }
    };
}
define_builder!(bytes, re_bytes, false);
define_builder!(unicode, re_unicode, true);
macro_rules! define_set_builder {
    ($name:ident, $regex_mod:ident, $only_utf8:expr) => {
        pub mod $name { use super::RegexOptions; use error::Error; use exec::ExecBuilder;
        use re_set::$regex_mod ::RegexSet; #[doc =
        " A configurable builder for a set of regular expressions."] #[doc = ""] #[doc =
        " A builder can be used to configure how the regexes are built, for example,"]
        #[doc =
        " by setting the default flags (which can be overridden in the expression"] #[doc
        = " itself) or setting various limits."] #[derive(Debug)] pub struct
        RegexSetBuilder(RegexOptions); impl RegexSetBuilder { #[doc =
        " Create a new regular expression builder with the given pattern."] #[doc = ""]
        #[doc = " If the pattern is invalid, then an error will be returned when"] #[doc
        = " `build` is called."] pub fn new < I, S > (patterns : I) -> RegexSetBuilder
        where S : AsRef < str >, I : IntoIterator < Item = S >, { let mut builder =
        RegexSetBuilder(RegexOptions::default()); for pat in patterns { builder.0.pats
        .push(pat.as_ref().to_owned()); } builder } #[doc =
        " Consume the builder and compile the regular expressions into a set."] pub fn
        build(& self) -> Result < RegexSet, Error > { ExecBuilder::new_options(self.0
        .clone()).only_utf8($only_utf8).build().map(RegexSet::from) } #[doc =
        " Set the value for the case insensitive (`i`) flag."] pub fn case_insensitive(&
        mut self, yes : bool,) -> & mut RegexSetBuilder { self.0.case_insensitive = yes;
        self } #[doc = " Set the value for the multi-line matching (`m`) flag."] pub fn
        multi_line(& mut self, yes : bool,) -> & mut RegexSetBuilder { self.0.multi_line
        = yes; self } #[doc =
        " Set the value for the any character (`s`) flag, where in `.` matches"] #[doc =
        " anything when `s` is set and matches anything except for new line when"] #[doc
        = " it is not set (the default)."] #[doc = ""] #[doc =
        " N.B. \"matches anything\" means \"any byte\" for `regex::bytes::RegexSet`"]
        #[doc =
        " expressions and means \"any Unicode scalar value\" for `regex::RegexSet`"]
        #[doc = " expressions."] pub fn dot_matches_new_line(& mut self, yes : bool,) ->
        & mut RegexSetBuilder { self.0.dot_matches_new_line = yes; self } #[doc =
        " Set the value for the greedy swap (`U`) flag."] pub fn swap_greed(& mut self,
        yes : bool,) -> & mut RegexSetBuilder { self.0.swap_greed = yes; self } #[doc =
        " Set the value for the ignore whitespace (`x`) flag."] pub fn
        ignore_whitespace(& mut self, yes : bool,) -> & mut RegexSetBuilder { self.0
        .ignore_whitespace = yes; self } #[doc =
        " Set the value for the Unicode (`u`) flag."] pub fn unicode(& mut self, yes :
        bool) -> & mut RegexSetBuilder { self.0.unicode = yes; self } #[doc =
        " Whether to support octal syntax or not."] #[doc = ""] #[doc =
        " Octal syntax is a little-known way of uttering Unicode codepoints in"] #[doc =
        " a regular expression. For example, `a`, `\\x61`, `\\u0061` and"] #[doc =
        " `\\141` are all equivalent regular expressions, where the last example"] #[doc
        = " shows octal syntax."] #[doc = ""] #[doc =
        " While supporting octal syntax isn't in and of itself a problem, it does"] #[doc
        = " make good error messages harder. That is, in PCRE based regex engines,"]
        #[doc = " syntax like `\\0` invokes a backreference, which is explicitly"] #[doc
        = " unsupported in Rust's regex engine. However, many users expect it to"] #[doc
        = " be supported. Therefore, when octal support is disabled, the error"] #[doc =
        " message will explicitly mention that backreferences aren't supported."] #[doc =
        ""] #[doc = " Octal syntax is disabled by default."] pub fn octal(& mut self, yes
        : bool) -> & mut RegexSetBuilder { self.0.octal = yes; self } #[doc =
        " Set the approximate size limit of the compiled regular expression."] #[doc =
        ""] #[doc =
        " This roughly corresponds to the number of bytes occupied by a single"] #[doc =
        " compiled program. If the program exceeds this number, then a"] #[doc =
        " compilation error is returned."] pub fn size_limit(& mut self, limit : usize,)
        -> & mut RegexSetBuilder { self.0.size_limit = limit; self } #[doc =
        " Set the approximate size of the cache used by the DFA."] #[doc = ""] #[doc =
        " This roughly corresponds to the number of bytes that the DFA will"] #[doc =
        " use while searching."] #[doc = ""] #[doc =
        " Note that this is a *per thread* limit. There is no way to set a global"] #[doc
        = " limit. In particular, if a regex is used from multiple threads"] #[doc =
        " simultaneously, then each thread may use up to the number of bytes"] #[doc =
        " specified here."] pub fn dfa_size_limit(& mut self, limit : usize,) -> & mut
        RegexSetBuilder { self.0.dfa_size_limit = limit; self } #[doc =
        " Set the nesting limit for this parser."] #[doc = ""] #[doc =
        " The nesting limit controls how deep the abstract syntax tree is allowed"] #[doc
        = " to be. If the AST exceeds the given limit (e.g., with too many nested"] #[doc
        = " groups), then an error is returned by the parser."] #[doc = ""] #[doc =
        " The purpose of this limit is to act as a heuristic to prevent stack"] #[doc =
        " overflow for consumers that do structural induction on an `Ast` using"] #[doc =
        " explicit recursion. While this crate never does this (instead using"] #[doc =
        " constant stack space and moving the call stack to the heap), other"] #[doc =
        " crates may."] #[doc = ""] #[doc =
        " This limit is not checked until the entire Ast is parsed. Therefore,"] #[doc =
        " if callers want to put a limit on the amount of heap space used, then"] #[doc =
        " they should impose a limit on the length, in bytes, of the concrete"] #[doc =
        " pattern string. In particular, this is viable since this parser"] #[doc =
        " implementation will limit itself to heap space proportional to the"] #[doc =
        " length of the pattern string."] #[doc = ""] #[doc =
        " Note that a nest limit of `0` will return a nest limit error for most"] #[doc =
        " patterns but not all. For example, a nest limit of `0` permits `a` but"] #[doc
        = " not `ab`, since `ab` requires a concatenation, which results in a nest"]
        #[doc =
        " depth of `1`. In general, a nest limit is not something that manifests"] #[doc
        = " in an obvious way in the concrete syntax, therefore, it should not be"] #[doc
        = " used in a granular way."] pub fn nest_limit(& mut self, limit : u32,) -> &
        mut RegexSetBuilder { self.0.nest_limit = limit; self } } }
    };
}
define_set_builder!(set_bytes, bytes, false);
define_set_builder!(set_unicode, unicode, true);
#[cfg(test)]
mod tests_llm_16_498 {
    use super::*;
    use crate::*;
    use crate::bytes::Regex;
    #[test]
    fn test_case_insensitive() {
        let _rug_st_tests_llm_16_498_rrrruuuugggg_test_case_insensitive = 0;
        let rug_fuzz_0 = "pattern";
        let rug_fuzz_1 = true;
        let rug_fuzz_2 = "Pattern";
        let mut builder = RegexBuilder::new(rug_fuzz_0);
        let regex = builder.case_insensitive(rug_fuzz_1).build().unwrap();
        debug_assert!(regex.is_match(rug_fuzz_2));
        let _rug_ed_tests_llm_16_498_rrrruuuugggg_test_case_insensitive = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_507 {
    use crate::bytes::RegexBuilder;
    #[test]
    fn test_nest_limit() {
        let _rug_st_tests_llm_16_507_rrrruuuugggg_test_nest_limit = 0;
        let rug_fuzz_0 = "pattern";
        let rug_fuzz_1 = 10;
        let mut builder = RegexBuilder::new(rug_fuzz_0);
        builder.nest_limit(rug_fuzz_1);
        let _rug_ed_tests_llm_16_507_rrrruuuugggg_test_nest_limit = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_510 {
    use super::*;
    use crate::*;
    #[test]
    fn test_octal() {
        let _rug_st_tests_llm_16_510_rrrruuuugggg_test_octal = 0;
        let rug_fuzz_0 = "pattern";
        let rug_fuzz_1 = true;
        let rug_fuzz_2 = "string";
        let rug_fuzz_3 = "pattern";
        let rug_fuzz_4 = false;
        let rug_fuzz_5 = "string";
        let mut builder = RegexBuilder::new(rug_fuzz_0);
        builder.octal(rug_fuzz_1);
        let regex = builder.build().unwrap();
        debug_assert!(regex.is_match(rug_fuzz_2));
        let mut builder = RegexBuilder::new(rug_fuzz_3);
        builder.octal(rug_fuzz_4);
        let regex = builder.build().unwrap();
        debug_assert!(regex.is_match(rug_fuzz_5));
        let _rug_ed_tests_llm_16_510_rrrruuuugggg_test_octal = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_519 {
    use crate::re_builder::set_bytes::RegexSetBuilder;
    #[test]
    fn test_case_insensitive() {
        let _rug_st_tests_llm_16_519_rrrruuuugggg_test_case_insensitive = 0;
        let rug_fuzz_0 = "abc";
        let rug_fuzz_1 = "def";
        let rug_fuzz_2 = true;
        let mut builder = RegexSetBuilder::new([rug_fuzz_0, rug_fuzz_1]);
        builder.case_insensitive(rug_fuzz_2);
        debug_assert!(builder.build().is_ok());
        let _rug_ed_tests_llm_16_519_rrrruuuugggg_test_case_insensitive = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_528 {
    use super::*;
    use crate::*;
    #[test]
    fn test_nest_limit() {
        let _rug_st_tests_llm_16_528_rrrruuuugggg_test_nest_limit = 0;
        let rug_fuzz_0 = "abc";
        let rug_fuzz_1 = 100;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 1;
        let mut builder = RegexSetBuilder::new(vec![rug_fuzz_0, "def"]);
        builder.nest_limit(rug_fuzz_1);
        let regex_set = builder.build().unwrap();
        debug_assert_eq!(regex_set.patterns().len(), 2);
        debug_assert_eq!(regex_set.patterns() [rug_fuzz_2].as_str(), "abc");
        debug_assert_eq!(regex_set.patterns() [rug_fuzz_3].as_str(), "def");
        let _rug_ed_tests_llm_16_528_rrrruuuugggg_test_nest_limit = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_529 {
    use super::*;
    use crate::*;
    use crate::RegexSet;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_529_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = "abc";
        let patterns = vec![rug_fuzz_0, "def", "ghi"];
        let builder = RegexSetBuilder::new(patterns);
        let regex_set = builder.build().unwrap();
        debug_assert_eq!(regex_set.len(), 3);
        let _rug_ed_tests_llm_16_529_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_530 {
    use super::*;
    use crate::*;
    use crate::{RegexSet, RegexSetBuilder, Error};
    #[test]
    fn test_octal() {
        let _rug_st_tests_llm_16_530_rrrruuuugggg_test_octal = 0;
        let rug_fuzz_0 = "a";
        let rug_fuzz_1 = true;
        let rug_fuzz_2 = "a";
        let rug_fuzz_3 = false;
        let mut builder = RegexSetBuilder::new([rug_fuzz_0]);
        builder.octal(rug_fuzz_1);
        let regex_set = builder.build();
        debug_assert!(regex_set.is_ok());
        let mut builder = RegexSetBuilder::new([rug_fuzz_2]);
        builder.octal(rug_fuzz_3);
        let regex_set = builder.build();
        debug_assert!(regex_set.is_ok());
        let _rug_ed_tests_llm_16_530_rrrruuuugggg_test_octal = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_539 {
    use super::*;
    use crate::*;
    use re_builder::set_unicode::RegexSetBuilder;
    #[test]
    fn test_case_insensitive() {
        let _rug_st_tests_llm_16_539_rrrruuuugggg_test_case_insensitive = 0;
        let rug_fuzz_0 = "abc";
        let rug_fuzz_1 = "def";
        let rug_fuzz_2 = true;
        let rug_fuzz_3 = "ABC";
        let rug_fuzz_4 = "Def";
        let mut builder = RegexSetBuilder::new([rug_fuzz_0, rug_fuzz_1]);
        builder.case_insensitive(rug_fuzz_2);
        let regex_set = builder.build().unwrap();
        debug_assert!(regex_set.is_match(rug_fuzz_3));
        debug_assert!(regex_set.is_match(rug_fuzz_4));
        let _rug_ed_tests_llm_16_539_rrrruuuugggg_test_case_insensitive = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_544 {
    use crate::re_builder::set_unicode::RegexSetBuilder;
    #[test]
    fn test_ignore_whitespace() {
        let _rug_st_tests_llm_16_544_rrrruuuugggg_test_ignore_whitespace = 0;
        let rug_fuzz_0 = "foo";
        let rug_fuzz_1 = true;
        let mut builder = RegexSetBuilder::new(vec![rug_fuzz_0, "bar"]);
        builder.ignore_whitespace(rug_fuzz_1);
        let regex_set = builder.build().unwrap();
        let _rug_ed_tests_llm_16_544_rrrruuuugggg_test_ignore_whitespace = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_551 {
    use super::*;
    use crate::*;
    use crate::RegexSet;
    #[test]
    fn test_octal() {
        let _rug_st_tests_llm_16_551_rrrruuuugggg_test_octal = 0;
        let rug_fuzz_0 = "abc";
        let rug_fuzz_1 = r"\141bc";
        let rug_fuzz_2 = true;
        let rug_fuzz_3 = false;
        let mut builder = RegexSetBuilder::new([rug_fuzz_0, rug_fuzz_1]);
        builder.octal(rug_fuzz_2);
        let regex_set = builder.build().unwrap();
        debug_assert_eq!(regex_set.len(), 1);
        builder.octal(rug_fuzz_3);
        let regex_set = builder.build().unwrap();
        debug_assert_eq!(regex_set.len(), 2);
        let _rug_ed_tests_llm_16_551_rrrruuuugggg_test_octal = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_552 {
    use crate::re_builder::set_unicode::RegexSetBuilder;
    #[test]
    fn test_size_limit() {
        let _rug_st_tests_llm_16_552_rrrruuuugggg_test_size_limit = 0;
        let rug_fuzz_0 = "a";
        let rug_fuzz_1 = 100;
        let mut builder = RegexSetBuilder::new(vec![rug_fuzz_0, "b", "c"]);
        builder.size_limit(rug_fuzz_1);
        let regex_set = builder.build().unwrap();
        debug_assert_eq!(regex_set.len(), 3);
        let _rug_ed_tests_llm_16_552_rrrruuuugggg_test_size_limit = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_559 {
    use super::*;
    use crate::*;
    use crate::Regex;
    #[test]
    fn test_case_insensitive() {
        let _rug_st_tests_llm_16_559_rrrruuuugggg_test_case_insensitive = 0;
        let rug_fuzz_0 = "pattern";
        let rug_fuzz_1 = true;
        let rug_fuzz_2 = "Pattern";
        let mut builder = RegexBuilder::new(rug_fuzz_0);
        builder.case_insensitive(rug_fuzz_1);
        let regex = builder.build().unwrap();
        debug_assert_eq!(regex.is_match(rug_fuzz_2), true);
        let _rug_ed_tests_llm_16_559_rrrruuuugggg_test_case_insensitive = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_568 {
    use crate::re_builder::unicode::RegexBuilder;
    #[test]
    fn test_nest_limit() {
        let _rug_st_tests_llm_16_568_rrrruuuugggg_test_nest_limit = 0;
        let rug_fuzz_0 = "pattern";
        let rug_fuzz_1 = 100;
        let mut regex_builder = RegexBuilder::new(rug_fuzz_0);
        regex_builder.nest_limit(rug_fuzz_1);
        let regex = regex_builder.build().unwrap();
        debug_assert_eq!(regex.as_str(), "pattern");
        let _rug_ed_tests_llm_16_568_rrrruuuugggg_test_nest_limit = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_575 {
    use super::*;
    use crate::*;
    #[test]
    fn test_swap_greed() {
        let _rug_st_tests_llm_16_575_rrrruuuugggg_test_swap_greed = 0;
        let rug_fuzz_0 = "pattern";
        let rug_fuzz_1 = true;
        let rug_fuzz_2 = "pattern";
        let mut builder = RegexBuilder::new(rug_fuzz_0);
        builder.swap_greed(rug_fuzz_1);
        let regex = builder.build().unwrap();
        debug_assert_eq!(regex.as_str(), "pattern");
        debug_assert_eq!(regex.is_match(rug_fuzz_2), true);
        let _rug_ed_tests_llm_16_575_rrrruuuugggg_test_swap_greed = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_576 {
    use super::*;
    use crate::*;
    #[test]
    fn test_regex_unicode() {
        let _rug_st_tests_llm_16_576_rrrruuuugggg_test_regex_unicode = 0;
        let rug_fuzz_0 = "pattern";
        let rug_fuzz_1 = true;
        let rug_fuzz_2 = "pattern";
        let rug_fuzz_3 = false;
        let mut builder = RegexBuilder::new(rug_fuzz_0);
        let result = builder.unicode(rug_fuzz_1).build();
        debug_assert!(result.is_ok());
        let mut builder = RegexBuilder::new(rug_fuzz_2);
        let result = builder.unicode(rug_fuzz_3).build();
        debug_assert!(result.is_ok());
        let _rug_ed_tests_llm_16_576_rrrruuuugggg_test_regex_unicode = 0;
    }
}
#[cfg(test)]
mod tests_rug_322 {
    use super::*;
    use crate::bytes::RegexBuilder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_322_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "abc";
        let mut p0 = rug_fuzz_0;
        RegexBuilder::new(&p0);
        let _rug_ed_tests_rug_322_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_323 {
    use super::*;
    use crate::re_builder::bytes::RegexBuilder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_323_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "pattern";
        let mut p0 = RegexBuilder::new(rug_fuzz_0);
        p0.build();
        let _rug_ed_tests_rug_323_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_324 {
    use super::*;
    use crate::bytes::RegexBuilder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_324_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "pattern";
        let rug_fuzz_1 = true;
        let mut p0 = RegexBuilder::new(rug_fuzz_0);
        let p1 = rug_fuzz_1;
        p0.dot_matches_new_line(p1);
        let _rug_ed_tests_rug_324_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_325 {
    use super::*;
    use crate::bytes::RegexBuilder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_325_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "pattern";
        let rug_fuzz_1 = true;
        let mut p0 = RegexBuilder::new(rug_fuzz_0);
        let p1 = rug_fuzz_1;
        p0.swap_greed(p1);
        let _rug_ed_tests_rug_325_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_326 {
    use super::*;
    use crate::bytes::RegexBuilder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_326_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "pattern";
        let rug_fuzz_1 = true;
        let mut p0 = RegexBuilder::new(rug_fuzz_0);
        let p1 = rug_fuzz_1;
        p0.ignore_whitespace(p1);
        let _rug_ed_tests_rug_326_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_327 {
    use super::*;
    use crate::bytes::RegexBuilder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_327_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "pattern";
        let rug_fuzz_1 = true;
        let mut p0 = RegexBuilder::new(rug_fuzz_0);
        let p1 = rug_fuzz_1;
        p0.unicode(p1);
        let _rug_ed_tests_rug_327_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_328 {
    use super::*;
    use crate::bytes::RegexBuilder;
    #[test]
    fn test_regex_builder_size_limit() {
        let _rug_st_tests_rug_328_rrrruuuugggg_test_regex_builder_size_limit = 0;
        let rug_fuzz_0 = "pattern";
        let rug_fuzz_1 = 100;
        let mut p0 = RegexBuilder::new(rug_fuzz_0);
        let p1: usize = rug_fuzz_1;
        p0.size_limit(p1);
        let _rug_ed_tests_rug_328_rrrruuuugggg_test_regex_builder_size_limit = 0;
    }
}
#[cfg(test)]
mod tests_rug_329 {
    use super::*;
    use crate::bytes::RegexBuilder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_329_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "pattern";
        let rug_fuzz_1 = 100;
        let mut p0 = RegexBuilder::new(rug_fuzz_0);
        let p1: usize = rug_fuzz_1;
        p0.dfa_size_limit(p1);
        let _rug_ed_tests_rug_329_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_330 {
    use super::*;
    use re_builder::unicode::RegexBuilder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_330_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "a pattern";
        let mut p0 = rug_fuzz_0;
        RegexBuilder::new(&p0);
        let _rug_ed_tests_rug_330_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_331 {
    use super::*;
    use re_builder::unicode::RegexBuilder;
    #[test]
    fn test_regex_builder() {
        let _rug_st_tests_rug_331_rrrruuuugggg_test_regex_builder = 0;
        let rug_fuzz_0 = "pattern";
        let rug_fuzz_1 = true;
        let rug_fuzz_2 = false;
        let rug_fuzz_3 = true;
        let rug_fuzz_4 = false;
        let rug_fuzz_5 = true;
        let rug_fuzz_6 = false;
        let rug_fuzz_7 = true;
        let rug_fuzz_8 = 1000;
        let rug_fuzz_9 = 2000;
        let rug_fuzz_10 = 3;
        let mut p0 = RegexBuilder::new(rug_fuzz_0)
            .case_insensitive(rug_fuzz_1)
            .multi_line(rug_fuzz_2)
            .dot_matches_new_line(rug_fuzz_3)
            .swap_greed(rug_fuzz_4)
            .ignore_whitespace(rug_fuzz_5)
            .unicode(rug_fuzz_6)
            .octal(rug_fuzz_7)
            .size_limit(rug_fuzz_8)
            .dfa_size_limit(rug_fuzz_9)
            .nest_limit(rug_fuzz_10)
            .build()
            .unwrap();
        let _rug_ed_tests_rug_331_rrrruuuugggg_test_regex_builder = 0;
    }
}
#[cfg(test)]
mod tests_rug_338 {
    use super::*;
    use crate::bytes::RegexSetBuilder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_338_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "pattern1";
        let rug_fuzz_1 = "pattern2";
        let mut p0 = RegexSetBuilder::new([rug_fuzz_0, rug_fuzz_1]);
        crate::re_builder::set_bytes::RegexSetBuilder::build(&mut p0).unwrap();
        let _rug_ed_tests_rug_338_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_339 {
    use super::*;
    use crate::re_builder::set_bytes::RegexSetBuilder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_339_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "pattern1";
        let rug_fuzz_1 = "pattern2";
        let rug_fuzz_2 = true;
        let mut p0 = RegexSetBuilder::new([rug_fuzz_0, rug_fuzz_1]);
        let p1: bool = rug_fuzz_2;
        p0.multi_line(p1);
        let _rug_ed_tests_rug_339_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_340 {
    use super::*;
    use crate::bytes::RegexSetBuilder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_340_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "pattern1";
        let rug_fuzz_1 = "pattern2";
        let rug_fuzz_2 = true;
        let mut p0 = RegexSetBuilder::new([rug_fuzz_0, rug_fuzz_1]);
        let p1 = rug_fuzz_2;
        p0.dot_matches_new_line(p1);
        let _rug_ed_tests_rug_340_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_341 {
    use super::*;
    use crate::re_builder::set_bytes::RegexSetBuilder;
    #[test]
    fn test_swap_greed() {
        let _rug_st_tests_rug_341_rrrruuuugggg_test_swap_greed = 0;
        let rug_fuzz_0 = "pattern1";
        let rug_fuzz_1 = "pattern2";
        let rug_fuzz_2 = true;
        let mut p0 = RegexSetBuilder::new([rug_fuzz_0, rug_fuzz_1]);
        let p1 = rug_fuzz_2;
        p0.swap_greed(p1);
        let _rug_ed_tests_rug_341_rrrruuuugggg_test_swap_greed = 0;
    }
}
#[cfg(test)]
mod tests_rug_342 {
    use super::*;
    use crate::re_builder::set_bytes::RegexSetBuilder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_342_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "pattern1";
        let rug_fuzz_1 = "pattern2";
        let rug_fuzz_2 = true;
        let mut p0 = RegexSetBuilder::new([rug_fuzz_0, rug_fuzz_1]);
        let p1 = rug_fuzz_2;
        p0.ignore_whitespace(p1);
        let _rug_ed_tests_rug_342_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_343 {
    use super::*;
    use crate::re_builder::set_bytes::RegexSetBuilder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_343_rrrruuuugggg_sample = 0;
        let rug_fuzz_0 = "pattern1";
        let rug_fuzz_1 = "pattern2";
        #[cfg(test)]
        mod tests_rug_343_prepare {
            use crate::re_builder::set_bytes::RegexSetBuilder;
            #[test]
            fn sample() {
                let _rug_st_tests_rug_343_prepare_rrrruuuugggg_sample = 0;
                let rug_fuzz_0 = 0;
                let rug_fuzz_1 = "pattern1";
                let rug_fuzz_2 = "pattern2";
                let rug_fuzz_3 = 0;
                let _rug_st_tests_rug_343_rrrruuuugggg_sample = rug_fuzz_0;
                let rug_fuzz_0 = rug_fuzz_1;
                let rug_fuzz_1 = rug_fuzz_2;
                let mut v106 = RegexSetBuilder::new([rug_fuzz_0, rug_fuzz_1]);
                let _rug_ed_tests_rug_343_rrrruuuugggg_sample = rug_fuzz_3;
                let _rug_ed_tests_rug_343_prepare_rrrruuuugggg_sample = 0;
            }
        }
        let mut p0 = RegexSetBuilder::new(["pattern1", "pattern2"]);
        let p1 = true;
        RegexSetBuilder::unicode(&mut p0, p1);
        let _rug_ed_tests_rug_343_rrrruuuugggg_sample = 0;
    }
}
#[cfg(test)]
mod tests_rug_344 {
    use super::*;
    use bytes::RegexSetBuilder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_344_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "pattern1";
        let rug_fuzz_1 = "pattern2";
        let rug_fuzz_2 = 100;
        let mut p0 = RegexSetBuilder::new([rug_fuzz_0, rug_fuzz_1]);
        let p1: usize = rug_fuzz_2;
        RegexSetBuilder::size_limit(&mut p0, p1);
        let _rug_ed_tests_rug_344_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_345 {
    use super::*;
    use crate::re_builder::set_bytes::RegexSetBuilder;
    #[test]
    fn test_dfa_size_limit() {
        let _rug_st_tests_rug_345_rrrruuuugggg_test_dfa_size_limit = 0;
        let rug_fuzz_0 = "pattern1";
        let rug_fuzz_1 = "pattern2";
        let rug_fuzz_2 = 100;
        let mut p0 = RegexSetBuilder::new([rug_fuzz_0, rug_fuzz_1]);
        let p1: usize = rug_fuzz_2;
        p0.dfa_size_limit(p1);
        let _rug_ed_tests_rug_345_rrrruuuugggg_test_dfa_size_limit = 0;
    }
}
#[cfg(test)]
mod tests_rug_346 {
    use super::*;
    use re_builder::set_unicode::RegexSetBuilder;
    use std::iter::FromIterator;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_346_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "abc";
        let p0 = vec![rug_fuzz_0.to_owned(), "def".to_owned()];
        RegexSetBuilder::new(p0.into_iter());
        let _rug_ed_tests_rug_346_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_347 {
    use super::*;
    use crate::RegexSetBuilder;
    #[test]
    fn test_build() {
        let _rug_st_tests_rug_347_rrrruuuugggg_test_build = 0;
        let rug_fuzz_0 = "pattern1";
        let mut p0 = RegexSetBuilder::new(vec![rug_fuzz_0, "pattern2"]);
        let result = p0.build();
        debug_assert!(result.is_ok());
        let _rug_ed_tests_rug_347_rrrruuuugggg_test_build = 0;
    }
}
#[cfg(test)]
mod tests_rug_348 {
    use super::*;
    use crate::RegexSetBuilder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_348_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "pattern1";
        let rug_fuzz_1 = true;
        let mut p0 = RegexSetBuilder::new(vec![rug_fuzz_0, "pattern2"]);
        let p1 = rug_fuzz_1;
        p0.multi_line(p1);
        let _rug_ed_tests_rug_348_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_349 {
    use super::*;
    use crate::RegexSetBuilder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_349_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "pattern1";
        let rug_fuzz_1 = true;
        let mut p0 = RegexSetBuilder::new(vec![rug_fuzz_0, "pattern2"]);
        let p1 = rug_fuzz_1;
        p0.dot_matches_new_line(p1);
        let _rug_ed_tests_rug_349_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_350 {
    use super::*;
    #[test]
    fn test_swap_greed() {
        let _rug_st_tests_rug_350_rrrruuuugggg_test_swap_greed = 0;
        let rug_fuzz_0 = "pattern1";
        let rug_fuzz_1 = true;
        use crate::RegexSetBuilder;
        let mut p0 = RegexSetBuilder::new(vec![rug_fuzz_0, "pattern2"]);
        let p1 = rug_fuzz_1;
        p0.swap_greed(p1);
        let _rug_ed_tests_rug_350_rrrruuuugggg_test_swap_greed = 0;
    }
}
#[cfg(test)]
mod tests_rug_351 {
    use super::*;
    use crate::RegexSetBuilder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_351_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "pattern1";
        let rug_fuzz_1 = true;
        let mut p0 = RegexSetBuilder::new(vec![rug_fuzz_0, "pattern2"]);
        let p1 = rug_fuzz_1;
        p0.unicode(p1);
        let _rug_ed_tests_rug_351_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_352 {
    use super::*;
    use crate::RegexSetBuilder;
    #[test]
    fn test_dfa_size_limit() {
        let _rug_st_tests_rug_352_rrrruuuugggg_test_dfa_size_limit = 0;
        let rug_fuzz_0 = "pattern1";
        let rug_fuzz_1 = 1000;
        let mut p0 = RegexSetBuilder::new(vec![rug_fuzz_0, "pattern2"]);
        let p1: usize = rug_fuzz_1;
        p0.dfa_size_limit(p1);
        let _rug_ed_tests_rug_352_rrrruuuugggg_test_dfa_size_limit = 0;
    }
}
#[cfg(test)]
mod tests_rug_353 {
    use super::*;
    use crate::RegexSetBuilder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_353_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "pattern1";
        let rug_fuzz_1 = 10;
        let mut p0 = RegexSetBuilder::new(vec![rug_fuzz_0, "pattern2"]);
        let mut p1 = rug_fuzz_1;
        p0.nest_limit(p1);
        let _rug_ed_tests_rug_353_rrrruuuugggg_test_rug = 0;
    }
}
