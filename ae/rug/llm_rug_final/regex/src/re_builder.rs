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
        pub mod $name {
            use super::RegexOptions;
            use crate::error::Error;
            use crate::exec::ExecBuilder;

            use crate::$regex_mod::Regex;

            /// A configurable builder for a regular expression.
            ///
            /// A builder can be used to configure how the regex is built, for example, by
            /// setting the default flags (which can be overridden in the expression
            /// itself) or setting various limits.
            #[derive(Debug)]
            pub struct RegexBuilder(RegexOptions);

            impl RegexBuilder {
                /// Create a new regular expression builder with the given pattern.
                ///
                /// If the pattern is invalid, then an error will be returned when
                /// `build` is called.
                pub fn new(pattern: &str) -> RegexBuilder {
                    let mut builder = RegexBuilder(RegexOptions::default());
                    builder.0.pats.push(pattern.to_owned());
                    builder
                }

                /// Consume the builder and compile the regular expression.
                ///
                /// Note that calling `as_str` on the resulting `Regex` will produce the
                /// pattern given to `new` verbatim. Notably, it will not incorporate any
                /// of the flags set on this builder.
                pub fn build(&self) -> Result<Regex, Error> {
                    ExecBuilder::new_options(self.0.clone())
                        .only_utf8($only_utf8)
                        .build()
                        .map(Regex::from)
                }

                /// Set the value for the case insensitive (`i`) flag.
                ///
                /// When enabled, letters in the pattern will match both upper case and
                /// lower case variants.
                pub fn case_insensitive(
                    &mut self,
                    yes: bool,
                ) -> &mut RegexBuilder {
                    self.0.case_insensitive = yes;
                    self
                }

                /// Set the value for the multi-line matching (`m`) flag.
                ///
                /// When enabled, `^` matches the beginning of lines and `$` matches the
                /// end of lines.
                ///
                /// By default, they match beginning/end of the input.
                pub fn multi_line(&mut self, yes: bool) -> &mut RegexBuilder {
                    self.0.multi_line = yes;
                    self
                }

                /// Set the value for the any character (`s`) flag, where in `.` matches
                /// anything when `s` is set and matches anything except for new line when
                /// it is not set (the default).
                ///
                /// N.B. "matches anything" means "any byte" when Unicode is disabled and
                /// means "any valid UTF-8 encoding of any Unicode scalar value" when
                /// Unicode is enabled.
                pub fn dot_matches_new_line(
                    &mut self,
                    yes: bool,
                ) -> &mut RegexBuilder {
                    self.0.dot_matches_new_line = yes;
                    self
                }

                /// Set the value for the greedy swap (`U`) flag.
                ///
                /// When enabled, a pattern like `a*` is lazy (tries to find shortest
                /// match) and `a*?` is greedy (tries to find longest match).
                ///
                /// By default, `a*` is greedy and `a*?` is lazy.
                pub fn swap_greed(&mut self, yes: bool) -> &mut RegexBuilder {
                    self.0.swap_greed = yes;
                    self
                }

                /// Set the value for the ignore whitespace (`x`) flag.
                ///
                /// When enabled, whitespace such as new lines and spaces will be ignored
                /// between expressions of the pattern, and `#` can be used to start a
                /// comment until the next new line.
                pub fn ignore_whitespace(
                    &mut self,
                    yes: bool,
                ) -> &mut RegexBuilder {
                    self.0.ignore_whitespace = yes;
                    self
                }

                /// Set the value for the Unicode (`u`) flag.
                ///
                /// Enabled by default. When disabled, character classes such as `\w` only
                /// match ASCII word characters instead of all Unicode word characters.
                pub fn unicode(&mut self, yes: bool) -> &mut RegexBuilder {
                    self.0.unicode = yes;
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
                pub fn octal(&mut self, yes: bool) -> &mut RegexBuilder {
                    self.0.octal = yes;
                    self
                }

                /// Set the approximate size limit of the compiled regular expression.
                ///
                /// This roughly corresponds to the number of bytes occupied by a single
                /// compiled program. If the program exceeds this number, then a
                /// compilation error is returned.
                pub fn size_limit(
                    &mut self,
                    limit: usize,
                ) -> &mut RegexBuilder {
                    self.0.size_limit = limit;
                    self
                }

                /// Set the approximate size of the cache used by the DFA.
                ///
                /// This roughly corresponds to the number of bytes that the DFA will
                /// use while searching.
                ///
                /// Note that this is a *per thread* limit. There is no way to set a global
                /// limit. In particular, if a regex is used from multiple threads
                /// simultaneously, then each thread may use up to the number of bytes
                /// specified here.
                pub fn dfa_size_limit(
                    &mut self,
                    limit: usize,
                ) -> &mut RegexBuilder {
                    self.0.dfa_size_limit = limit;
                    self
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
                /// length of the pattern string.
                ///
                /// Note that a nest limit of `0` will return a nest limit error for most
                /// patterns but not all. For example, a nest limit of `0` permits `a` but
                /// not `ab`, since `ab` requires a concatenation, which results in a nest
                /// depth of `1`. In general, a nest limit is not something that manifests
                /// in an obvious way in the concrete syntax, therefore, it should not be
                /// used in a granular way.
                pub fn nest_limit(&mut self, limit: u32) -> &mut RegexBuilder {
                    self.0.nest_limit = limit;
                    self
                }
            }
        }
    };
}

define_builder!(bytes, re_bytes, false);
define_builder!(unicode, re_unicode, true);

macro_rules! define_set_builder {
    ($name:ident, $regex_mod:ident, $only_utf8:expr) => {
        pub mod $name {
            use super::RegexOptions;
            use crate::error::Error;
            use crate::exec::ExecBuilder;

            use crate::re_set::$regex_mod::RegexSet;

            /// A configurable builder for a set of regular expressions.
            ///
            /// A builder can be used to configure how the regexes are built, for example,
            /// by setting the default flags (which can be overridden in the expression
            /// itself) or setting various limits.
            #[derive(Debug)]
            pub struct RegexSetBuilder(RegexOptions);

            impl RegexSetBuilder {
                /// Create a new regular expression builder with the given pattern.
                ///
                /// If the pattern is invalid, then an error will be returned when
                /// `build` is called.
                pub fn new<I, S>(patterns: I) -> RegexSetBuilder
                where
                    S: AsRef<str>,
                    I: IntoIterator<Item = S>,
                {
                    let mut builder = RegexSetBuilder(RegexOptions::default());
                    for pat in patterns {
                        builder.0.pats.push(pat.as_ref().to_owned());
                    }
                    builder
                }

                /// Consume the builder and compile the regular expressions into a set.
                pub fn build(&self) -> Result<RegexSet, Error> {
                    ExecBuilder::new_options(self.0.clone())
                        .only_utf8($only_utf8)
                        .build()
                        .map(RegexSet::from)
                }

                /// Set the value for the case insensitive (`i`) flag.
                pub fn case_insensitive(
                    &mut self,
                    yes: bool,
                ) -> &mut RegexSetBuilder {
                    self.0.case_insensitive = yes;
                    self
                }

                /// Set the value for the multi-line matching (`m`) flag.
                pub fn multi_line(
                    &mut self,
                    yes: bool,
                ) -> &mut RegexSetBuilder {
                    self.0.multi_line = yes;
                    self
                }

                /// Set the value for the any character (`s`) flag, where in `.` matches
                /// anything when `s` is set and matches anything except for new line when
                /// it is not set (the default).
                ///
                /// N.B. "matches anything" means "any byte" for `regex::bytes::RegexSet`
                /// expressions and means "any Unicode scalar value" for `regex::RegexSet`
                /// expressions.
                pub fn dot_matches_new_line(
                    &mut self,
                    yes: bool,
                ) -> &mut RegexSetBuilder {
                    self.0.dot_matches_new_line = yes;
                    self
                }

                /// Set the value for the greedy swap (`U`) flag.
                pub fn swap_greed(
                    &mut self,
                    yes: bool,
                ) -> &mut RegexSetBuilder {
                    self.0.swap_greed = yes;
                    self
                }

                /// Set the value for the ignore whitespace (`x`) flag.
                pub fn ignore_whitespace(
                    &mut self,
                    yes: bool,
                ) -> &mut RegexSetBuilder {
                    self.0.ignore_whitespace = yes;
                    self
                }

                /// Set the value for the Unicode (`u`) flag.
                pub fn unicode(&mut self, yes: bool) -> &mut RegexSetBuilder {
                    self.0.unicode = yes;
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
                pub fn octal(&mut self, yes: bool) -> &mut RegexSetBuilder {
                    self.0.octal = yes;
                    self
                }

                /// Set the approximate size limit of the compiled regular expression.
                ///
                /// This roughly corresponds to the number of bytes occupied by a single
                /// compiled program. If the program exceeds this number, then a
                /// compilation error is returned.
                pub fn size_limit(
                    &mut self,
                    limit: usize,
                ) -> &mut RegexSetBuilder {
                    self.0.size_limit = limit;
                    self
                }

                /// Set the approximate size of the cache used by the DFA.
                ///
                /// This roughly corresponds to the number of bytes that the DFA will
                /// use while searching.
                ///
                /// Note that this is a *per thread* limit. There is no way to set a global
                /// limit. In particular, if a regex is used from multiple threads
                /// simultaneously, then each thread may use up to the number of bytes
                /// specified here.
                pub fn dfa_size_limit(
                    &mut self,
                    limit: usize,
                ) -> &mut RegexSetBuilder {
                    self.0.dfa_size_limit = limit;
                    self
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
                /// length of the pattern string.
                ///
                /// Note that a nest limit of `0` will return a nest limit error for most
                /// patterns but not all. For example, a nest limit of `0` permits `a` but
                /// not `ab`, since `ab` requires a concatenation, which results in a nest
                /// depth of `1`. In general, a nest limit is not something that manifests
                /// in an obvious way in the concrete syntax, therefore, it should not be
                /// used in a granular way.
                pub fn nest_limit(
                    &mut self,
                    limit: u32,
                ) -> &mut RegexSetBuilder {
                    self.0.nest_limit = limit;
                    self
                }
            }
        }
    };
}

define_set_builder!(set_bytes, bytes, false);
define_set_builder!(set_unicode, unicode, true);
#[cfg(test)]
mod tests_rug_540 {
    use super::*;
    use crate::re_builder::RegexOptions;
    
    #[test]
    fn test_rug() {
        <RegexOptions as std::default::Default>::default();
    }
}#[cfg(test)]
mod tests_rug_541 {
    use super::*;

    #[test]
    fn test_rug() {
        let mut p0 = "abc";

        crate::re_builder::bytes::RegexBuilder::new(&p0);

    }
}
#[cfg(test)]
mod tests_rug_542 {
    use super::*;
    use crate::re_builder::bytes::RegexBuilder;

    #[test]
    fn test_rug() {
        let p0 = RegexBuilder::new("pattern")
            .case_insensitive(true)
            .multi_line(false)
            .dot_matches_new_line(true)
            .swap_greed(true)
            .ignore_whitespace(false)
            .unicode(true)
            .octal(false)
            .size_limit(1000)
            .dfa_size_limit(1000)
            .nest_limit(10)
            .build();

        assert!(p0.is_ok());
    }
}#[cfg(test)]
mod tests_rug_553 {
    use super::*;
    use crate::re_builder::unicode::RegexBuilder;

    #[test]
    fn test_rug() {
        let p0: &str = "abcd";
        
        RegexBuilder::new(p0);
    }
}
#[cfg(test)]
mod tests_rug_554 {
    use super::*;
    use crate::{Error, Regex, RegexBuilder};

    #[test]
    fn test_rug() {
        let mut p0 = RegexBuilder::new("pattern");

        p0.build().unwrap();
    }
}
#[cfg(test)]
mod tests_rug_555 {
    use super::*;
    use crate::RegexBuilder;
    
    #[test]
    fn test_rug() {
        let mut p0 = RegexBuilder::new("pattern");
        let mut p1 = true;

        p0.case_insensitive(p1);
    }
}#[cfg(test)]
mod tests_rug_556 {
    use crate::re_builder::unicode::RegexBuilder;

    #[test]
    fn test_multi_line() {
        let mut p0 = RegexBuilder::new("pattern");
        let p1 = true;

        p0.multi_line(p1);
    }
}#[cfg(test)]
mod tests_rug_557 {
    use super::*;
    use crate::RegexBuilder;

    #[test]
    fn test_rug() {
        let mut p0 = RegexBuilder::new("pattern");
        let p1 = true;

        p0.dot_matches_new_line(p1);
    }
}#[cfg(test)]
mod tests_rug_558 {
    use super::*;
    use crate::RegexBuilder;
    
    #[test]
    fn test_swap_greed() {
        let mut p0 = RegexBuilder::new("pattern");
        let p1: bool = true;
        
        p0.swap_greed(p1);
    }
}#[cfg(test)]
mod tests_rug_559 {
    use super::*;
    use crate::RegexBuilder;
    
    #[test]
    fn test_rug() {
        let mut p0 = RegexBuilder::new("pattern");
        let p1 = true;

        p0.ignore_whitespace(p1);
    }
}#[cfg(test)]
mod tests_rug_560 {
    use super::*;

    #[test]
    fn test_rug() {
        let mut p0 = crate::re_builder::unicode::RegexBuilder::new("pattern");
        let p1 = true;

        p0.unicode(p1);

    }
}#[cfg(test)]
mod tests_rug_561 {
    use super::*;
    use crate::RegexBuilder;
  
    #[test]
    fn test_rug() {
        let mut p0 = RegexBuilder::new("pattern");
        let p1 = true;

        p0.octal(p1);

    }
}#[cfg(test)]
mod tests_rug_562 {
    use super::*;
    use crate::re_builder;
    
    #[test]
    fn test_rug() {
        #[cfg(test)]
        mod tests_rug_562_prepare {
            use crate::RegexBuilder;
          
            #[test]
            fn sample() {
                let mut v127 = RegexBuilder::new("pattern");
            }
        }
        
        let mut p0 = re_builder::unicode::RegexBuilder::new("pattern");
        let p1: usize = 100;
        
        <re_builder::unicode::RegexBuilder>::size_limit(&mut p0, p1);
    }
}#[cfg(test)]
mod tests_rug_563 {
    use super::*;
    use crate::RegexBuilder;
    
    #[test]
    fn test_dfa_size_limit() {
        let mut p0 = RegexBuilder::new("pattern");
        let p1: usize = 1000;
        
        p0.dfa_size_limit(p1);
    }
}#[cfg(test)]
mod tests_rug_564 {
    use super::*;
    use crate::RegexBuilder;

    #[test]
    fn test_rug() {
        let mut p0 = RegexBuilder::new("pattern");
        let p1: u32 = 10;

        p0.nest_limit(p1);
    }
}#[cfg(test)]
mod tests_rug_566 {
    use super::*;
    use crate::bytes::RegexSetBuilder;

    #[test]
    fn test_regex_set_builder() {
        let p0: RegexSetBuilder = RegexSetBuilder::new(["abc", "def"]);
        p0.build().unwrap();
    }
}#[cfg(test)]
mod tests_rug_574 {
    use super::*;
    use crate::bytes::RegexSetBuilder;

    #[test]
    fn test_rug() {
        let mut p0 = RegexSetBuilder::new(["abc", "def"]);
        let p1: usize = 100;

        p0.size_limit(p1);
    }
}#[cfg(test)]
mod tests_rug_575 {
    use super::*;
    use crate::bytes::RegexSetBuilder;
    
    #[test]
    fn test_dfa_size_limit() {
        let mut p0 = RegexSetBuilder::new(["abc", "def"]);
        let p1: usize = 100;
        
        p0.dfa_size_limit(p1);
    }
}                        
        #[cfg(test)]
        mod tests_rug_577 {
            use super::*;
            use crate::RegexSetBuilder;
            #[test]
            fn test_rug() {
                let p0: Vec<String> = vec!["abc".to_owned(), "def".to_owned()];
                RegexSetBuilder::new(p0);
            }
        }
                            
#[cfg(test)]
mod tests_rug_578 {

    use super::*;
    use crate::{RegexSetBuilder, RegexSet, Error};

    #[test]
    fn test_rug() {
        let mut p0 = RegexSetBuilder::new(["pattern1", "pattern2"]);

        let _ = p0.build().map(RegexSet::from);

    }
}#[cfg(test)]
mod tests_rug_579 {
    use super::*;
    use crate::RegexSetBuilder;
    
    #[test]
    fn test_rug() {
        let mut p0 = RegexSetBuilder::new(["pattern1", "pattern2"]);
        let p1: bool = false;
        
        p0.case_insensitive(p1);

    }
}
#[cfg(test)]
mod tests_rug_580 {
    use super::*;
    use crate::RegexSetBuilder;
    
    #[test]
    fn test_rug() {
        let mut v129 = RegexSetBuilder::new(["pattern1", "pattern2"]);
        let mut p0 = &mut v129;
        let p1 = true;
        
        p0.multi_line(p1);
    }
}#[cfg(test)]
mod tests_rug_581 {
    use super::*;
    use crate::RegexSetBuilder;
    
    #[test]
    fn test_rug() {
        let mut p0 = RegexSetBuilder::new(["pattern1", "pattern2"]);
        let p1 = true;
        
        p0.dot_matches_new_line(p1);
    }
}#[cfg(test)]
mod tests_rug_582 {
    use super::*;
    use crate::RegexSetBuilder;
    #[test]
    fn test_rug() {
        let mut p0 = RegexSetBuilder::new(["pattern1", "pattern2"]);
        let p1: bool = true;
        p0.swap_greed(p1);

    }
}#[cfg(test)]
mod tests_rug_583 {
    use super::*;
    use crate::RegexSetBuilder;
    
    #[test]
    fn test_rug() {
        let mut p0 = RegexSetBuilder::new(["pattern1", "pattern2"]);
        let p1 = true;

        p0.ignore_whitespace(p1);

        // Add assertions or any other test logic here
    }
}#[cfg(test)]
    mod tests_rug_584 {
        use super::*;
        use crate::RegexSetBuilder;
        
        #[test]
        fn test_rug() {
            let mut p0 = RegexSetBuilder::new(["pattern1", "pattern2"]);
            let p1 = true;
                    
            p0.unicode(p1);

        }
    }
#[cfg(test)]
mod tests_rug_585 {
    use super::*;
    use crate::RegexSetBuilder;
    
    #[test]
    fn test_rug() {
        let mut p0 = RegexSetBuilder::new(["pattern1", "pattern2"]);
        let p1 = true;

        p0.octal(p1);

    }
}#[cfg(test)]
mod tests_rug_586 {
    use super::*;
    use crate::RegexSetBuilder;

    #[test]
    fn test_rug() {
        // Sample code to construct p0
        let mut v129 = RegexSetBuilder::new(["pattern1", "pattern2"]);

        // Sample value for p1
        let limit: usize = 100;

        // Call the target function
        v129.size_limit(limit);
    }
}#[cfg(test)]
mod tests_rug_587 {
    use super::*;
    use crate::RegexSetBuilder;
    
    #[test]
    fn test_regex_builder() {
        let mut p0 = RegexSetBuilder::new(["pattern1", "pattern2"]);
        let p1: usize = 100;
        
        p0.dfa_size_limit(p1);

    }
}#[cfg(test)]
mod tests_rug_588 {
    use crate::RegexSetBuilder;

    #[test]
    fn test_rug() {
        let mut p0 = RegexSetBuilder::new(["pattern1", "pattern2"]);
        let p1: u32 = 10;

        p0.nest_limit(p1);
    }
}