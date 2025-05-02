macro_rules! define_set {
    (
        $name:ident, $builder_mod:ident, $text_ty:ty, $as_bytes:expr,
        $(#[$doc_regexset_example:meta])*
    ) => {
        pub mod $name { use std::fmt; use std::iter; use std::slice; use std::vec; use
        error::Error; use exec::Exec; use re_builder::$builder_mod ::RegexSetBuilder; use
        re_trait::RegularExpression; #[doc =
        " Match multiple (possibly overlapping) regular expressions in a single scan."]
        #[doc = ""] #[doc =
        " A regex set corresponds to the union of two or more regular expressions."]
        #[doc = " That is, a regex set will match text where at least one of its"] #[doc
        = " constituent regular expressions matches. A regex set as its formulated here"]
        #[doc = " provides a touch more power: it will also report *which* regular"]
        #[doc =
        " expressions in the set match. Indeed, this is the key difference between"]
        #[doc = " regex sets and a single `Regex` with many alternates, since only one"]
        #[doc = " alternate can match at a time."] #[doc = ""] #[doc =
        " For example, consider regular expressions to match email addresses and"] #[doc
        = " domains: `[a-z]+@[a-z]+\\.(com|org|net)` and `[a-z]+\\.(com|org|net)`. If a"]
        #[doc = " regex set is constructed from those regexes, then searching the text"]
        #[doc =
        " `foo@example.com` will report both regexes as matching. Of course, one"] #[doc
        = " could accomplish this by compiling each regex on its own and doing two"]
        #[doc =
        " searches over the text. The key advantage of using a regex set is that it"]
        #[doc =
        " will report the matching regexes using a *single pass through the text*."]
        #[doc =
        " If one has hundreds or thousands of regexes to match repeatedly (like a URL"]
        #[doc =
        " router for a complex web application or a user agent matcher), then a regex"]
        #[doc = " set can realize huge performance gains."] #[doc = ""] #[doc =
        " # Example"] #[doc = ""] #[doc =
        " This shows how the above two regexes (for matching email addresses and"] #[doc
        = " domains) might work:"] #[doc = ""] $(#[$doc_regexset_example])* #[doc = ""]
        #[doc =
        " Note that it would be possible to adapt the above example to using `Regex`"]
        #[doc = " with an expression like:"] #[doc = ""] #[doc = " ```ignore"] #[doc =
        " (?P<email>[a-z]+@(?P<email_domain>[a-z]+[.](com|org|net)))|(?P<domain>[a-z]+[.](com|org|net))"]
        #[doc = " ```"] #[doc = ""] #[doc =
        " After a match, one could then inspect the capture groups to figure out"] #[doc
        = " which alternates matched. The problem is that it is hard to make this"] #[doc
        = " approach scale when there are many regexes since the overlap between each"]
        #[doc = " alternate isn't always obvious to reason about."] #[doc = ""] #[doc =
        " # Limitations"] #[doc = ""] #[doc =
        " Regex sets are limited to answering the following two questions:"] #[doc = ""]
        #[doc = " 1. Does any regex in the set match?"] #[doc =
        " 2. If so, which regexes in the set match?"] #[doc = ""] #[doc =
        " As with the main `Regex` type, it is cheaper to ask (1) instead of (2)"] #[doc
        = " since the matching engines can stop after the first match is found."] #[doc =
        ""] #[doc =
        " Other features like finding the location of successive matches or their"] #[doc
        = " sub-captures aren't supported. If you need this functionality, the"] #[doc =
        " recommended approach is to compile each regex in the set independently and"]
        #[doc = " selectively match them based on which regexes in the set matched."]
        #[doc = ""] #[doc = " # Performance"] #[doc = ""] #[doc =
        " A `RegexSet` has the same performance characteristics as `Regex`. Namely,"]
        #[doc =
        " search takes `O(mn)` time, where `m` is proportional to the size of the"] #[doc
        = " regex set and `n` is proportional to the length of the search text."]
        #[derive(Clone)] pub struct RegexSet(Exec); impl RegexSet { #[doc =
        " Create a new regex set with the given regular expressions."] #[doc = ""] #[doc
        = " This takes an iterator of `S`, where `S` is something that can produce"]
        #[doc = " a `&str`. If any of the strings in the iterator are not valid regular"]
        #[doc = " expressions, then an error is returned."] #[doc = ""] #[doc =
        " # Example"] #[doc = ""] #[doc =
        " Create a new regex set from an iterator of strings:"] #[doc = ""] #[doc =
        " ```rust"] #[doc = " # use regex::RegexSet;"] #[doc =
        " let set = RegexSet::new(&[r\"\\w+\", r\"\\d+\"]).unwrap();"] #[doc =
        " assert!(set.is_match(\"foo\"));"] #[doc = " ```"] pub fn new < I, S > (exprs :
        I) -> Result < RegexSet, Error > where S : AsRef < str >, I : IntoIterator < Item
        = S > { RegexSetBuilder::new(exprs).build() } #[doc =
        " Create a new empty regex set."] #[doc = ""] #[doc = " # Example"] #[doc = ""]
        #[doc = " ```rust"] #[doc = " # use regex::RegexSet;"] #[doc =
        " let set = RegexSet::empty();"] #[doc = " assert!(set.is_empty());"] #[doc =
        " ```"] pub fn empty() -> RegexSet { RegexSetBuilder::new(& [""; 0]).build()
        .unwrap() } #[doc =
        " Returns true if and only if one of the regexes in this set matches"] #[doc =
        " the text given."] #[doc = ""] #[doc =
        " This method should be preferred if you only need to test whether any"] #[doc =
        " of the regexes in the set should match, but don't care about *which*"] #[doc =
        " regexes matched. This is because the underlying matching engine will"] #[doc =
        " quit immediately after seeing the first match instead of continuing to"] #[doc
        = " find all matches."] #[doc = ""] #[doc =
        " Note that as with searches using `Regex`, the expression is unanchored"] #[doc
        = " by default. That is, if the regex does not start with `^` or `\\A`, or"]
        #[doc = " end with `$` or `\\z`, then it is permitted to match anywhere in the"]
        #[doc = " text."] #[doc = ""] #[doc = " # Example"] #[doc = ""] #[doc =
        " Tests whether a set matches some text:"] #[doc = ""] #[doc = " ```rust"] #[doc
        = " # use regex::RegexSet;"] #[doc =
        " let set = RegexSet::new(&[r\"\\w+\", r\"\\d+\"]).unwrap();"] #[doc =
        " assert!(set.is_match(\"foo\"));"] #[doc = " assert!(!set.is_match(\"☃\"));"]
        #[doc = " ```"] pub fn is_match(& self, text : $text_ty) -> bool { self
        .is_match_at(text, 0) } #[doc =
        " Returns the same as is_match, but starts the search at the given"] #[doc =
        " offset."] #[doc = ""] #[doc =
        " The significance of the starting point is that it takes the surrounding"] #[doc
        = " context into consideration. For example, the `\\A` anchor can only"] #[doc =
        " match when `start == 0`."] #[doc(hidden)] pub fn is_match_at(& self, text :
        $text_ty, start : usize) -> bool { self.0.searcher().is_match_at($as_bytes
        (text), start) } #[doc =
        " Returns the set of regular expressions that match in the given text."] #[doc =
        ""] #[doc =
        " The set returned contains the index of each regular expression that"] #[doc =
        " matches in the given text. The index is in correspondence with the"] #[doc =
        " order of regular expressions given to `RegexSet`'s constructor."] #[doc = ""]
        #[doc = " The set can also be used to iterate over the matched indices."] #[doc =
        ""] #[doc =
        " Note that as with searches using `Regex`, the expression is unanchored"] #[doc
        = " by default. That is, if the regex does not start with `^` or `\\A`, or"]
        #[doc = " end with `$` or `\\z`, then it is permitted to match anywhere in the"]
        #[doc = " text."] #[doc = ""] #[doc = " # Example"] #[doc = ""] #[doc =
        " Tests which regular expressions match the given text:"] #[doc = ""] #[doc =
        " ```rust"] #[doc = " # use regex::RegexSet;"] #[doc =
        " let set = RegexSet::new(&["] #[doc = "     r\"\\w+\","] #[doc =
        "     r\"\\d+\","] #[doc = "     r\"\\pL+\","] #[doc = "     r\"foo\","] #[doc =
        "     r\"bar\","] #[doc = "     r\"barfoo\","] #[doc = "     r\"foobar\","] #[doc
        = " ]).unwrap();"] #[doc =
        " let matches: Vec<_> = set.matches(\"foobar\").into_iter().collect();"] #[doc =
        " assert_eq!(matches, vec![0, 2, 3, 4, 6]);"] #[doc = ""] #[doc =
        " // You can also test whether a particular regex matched:"] #[doc =
        " let matches = set.matches(\"foobar\");"] #[doc =
        " assert!(!matches.matched(5));"] #[doc = " assert!(matches.matched(6));"] #[doc
        = " ```"] pub fn matches(& self, text : $text_ty) -> SetMatches { let mut matches
        = vec![false; self.0.regex_strings().len()]; let any = self.read_matches_at(& mut
        matches, text, 0); SetMatches { matched_any : any, matches : matches, } } #[doc =
        " Returns the same as matches, but starts the search at the given"] #[doc =
        " offset and stores the matches into the slice given."] #[doc = ""] #[doc =
        " The significance of the starting point is that it takes the surrounding"] #[doc
        = " context into consideration. For example, the `\\A` anchor can only"] #[doc =
        " match when `start == 0`."] #[doc = ""] #[doc =
        " `matches` must have a length that is at least the number of regexes"] #[doc =
        " in this set."] #[doc = ""] #[doc =
        " This method returns true if and only if at least one member of"] #[doc =
        " `matches` is true after executing the set against `text`."] #[doc(hidden)] pub
        fn read_matches_at(& self, matches : & mut [bool], text : $text_ty, start :
        usize,) -> bool { self.0.searcher().many_matches_at(matches, $as_bytes (text),
        start) } #[doc = " Returns the total number of regular expressions in this set."]
        pub fn len(& self) -> usize { self.0.regex_strings().len() } #[doc =
        " Returns `true` if this set contains no regular expressions."] pub fn is_empty(&
        self) -> bool { self.0.regex_strings().is_empty() } #[doc =
        " Returns the patterns that this set will match on."] #[doc = ""] #[doc =
        " This function can be used to determine the pattern for a match. The"] #[doc =
        " slice returned has exactly as many patterns givens to this regex set,"] #[doc =
        " and the order of the slice is the same as the order of the patterns"] #[doc =
        " provided to the set."] #[doc = ""] #[doc = " # Example"] #[doc = ""] #[doc =
        " ```rust"] #[doc = " # use regex::RegexSet;"] #[doc =
        " let set = RegexSet::new(&["] #[doc = "     r\"\\w+\","] #[doc =
        "     r\"\\d+\","] #[doc = "     r\"\\pL+\","] #[doc = "     r\"foo\","] #[doc =
        "     r\"bar\","] #[doc = "     r\"barfoo\","] #[doc = "     r\"foobar\","] #[doc
        = " ]).unwrap();"] #[doc = " let matches: Vec<_> = set"] #[doc =
        "     .matches(\"foobar\")"] #[doc = "     .into_iter()"] #[doc =
        "     .map(|match_idx| &set.patterns()[match_idx])"] #[doc = "     .collect();"]
        #[doc =
        " assert_eq!(matches, vec![r\"\\w+\", r\"\\pL+\", r\"foo\", r\"bar\", r\"foobar\"]);"]
        #[doc = " ```"] pub fn patterns(& self) -> & [String] { self.0.regex_strings() }
        } #[doc = " A set of matches returned by a regex set."] #[derive(Clone, Debug)]
        pub struct SetMatches { matched_any : bool, matches : Vec < bool >, } impl
        SetMatches { #[doc = " Whether this set contains any matches."] pub fn
        matched_any(& self) -> bool { self.matched_any } #[doc =
        " Whether the regex at the given index matched."] #[doc = ""] #[doc =
        " The index for a regex is determined by its insertion order upon the"] #[doc =
        " initial construction of a `RegexSet`, starting at `0`."] #[doc = ""] #[doc =
        " # Panics"] #[doc = ""] #[doc =
        " If `regex_index` is greater than or equal to `self.len()`."] pub fn matched(&
        self, regex_index : usize) -> bool { self.matches[regex_index] } #[doc =
        " The total number of regexes in the set that created these matches."] pub fn
        len(& self) -> usize { self.matches.len() } #[doc =
        " Returns an iterator over indexes in the regex that matched."] #[doc = ""] #[doc
        = " This will always produces matches in ascending order of index, where"] #[doc
        = " the index corresponds to the index of the regex that matched with"] #[doc =
        " respect to its position when initially building the set."] pub fn iter(& self)
        -> SetMatchesIter { SetMatchesIter((&* self.matches).into_iter().enumerate()) } }
        impl IntoIterator for SetMatches { type IntoIter = SetMatchesIntoIter; type Item
        = usize; fn into_iter(self) -> Self::IntoIter { SetMatchesIntoIter(self.matches
        .into_iter().enumerate()) } } impl <'a > IntoIterator for &'a SetMatches { type
        IntoIter = SetMatchesIter <'a >; type Item = usize; fn into_iter(self) ->
        Self::IntoIter { self.iter() } } #[doc =
        " An owned iterator over the set of matches from a regex set."] #[doc = ""] #[doc
        = " This will always produces matches in ascending order of index, where the"]
        #[doc =
        " index corresponds to the index of the regex that matched with respect to"]
        #[doc = " its position when initially building the set."] #[derive(Debug)] pub
        struct SetMatchesIntoIter(iter::Enumerate < vec::IntoIter < bool >>); impl
        Iterator for SetMatchesIntoIter { type Item = usize; fn next(& mut self) ->
        Option < usize > { loop { match self.0.next() { None => return None, Some((_,
        false)) => {} Some((i, true)) => return Some(i), } } } fn size_hint(& self) ->
        (usize, Option < usize >) { self.0.size_hint() } } impl DoubleEndedIterator for
        SetMatchesIntoIter { fn next_back(& mut self) -> Option < usize > { loop { match
        self.0.next_back() { None => return None, Some((_, false)) => {} Some((i, true))
        => return Some(i), } } } } impl iter::FusedIterator for SetMatchesIntoIter {}
        #[doc = " A borrowed iterator over the set of matches from a regex set."] #[doc =
        ""] #[doc = " The lifetime `'a` refers to the lifetime of a `SetMatches` value."]
        #[doc = ""] #[doc =
        " This will always produces matches in ascending order of index, where the"]
        #[doc =
        " index corresponds to the index of the regex that matched with respect to"]
        #[doc = " its position when initially building the set."] #[derive(Clone, Debug)]
        pub struct SetMatchesIter <'a > (iter::Enumerate < slice::Iter <'a, bool >>);
        impl <'a > Iterator for SetMatchesIter <'a > { type Item = usize; fn next(& mut
        self) -> Option < usize > { loop { match self.0.next() { None => return None,
        Some((_, & false)) => {} Some((i, & true)) => return Some(i), } } } fn
        size_hint(& self) -> (usize, Option < usize >) { self.0.size_hint() } } impl <'a
        > DoubleEndedIterator for SetMatchesIter <'a > { fn next_back(& mut self) ->
        Option < usize > { loop { match self.0.next_back() { None => return None,
        Some((_, & false)) => {} Some((i, & true)) => return Some(i), } } } } impl <'a >
        iter::FusedIterator for SetMatchesIter <'a > {} #[doc(hidden)] impl From < Exec >
        for RegexSet { fn from(exec : Exec) -> Self { RegexSet(exec) } } impl fmt::Debug
        for RegexSet { fn fmt(& self, f : & mut fmt::Formatter) -> fmt::Result {
        write!(f, "RegexSet({:?})", self.0.regex_strings()) } } #[allow(dead_code)] fn
        as_bytes_str(text : & str) -> & [u8] { text.as_bytes() } #[allow(dead_code)] fn
        as_bytes_bytes(text : & [u8]) -> & [u8] { text } }
    };
}
define_set! {
    unicode, set_unicode, & str, as_bytes_str, #[doc = " ```rust"] #[doc =
    " # use regex::RegexSet;"] #[doc = " let set = RegexSet::new(&["] #[doc =
    "     r\"[a-z]+@[a-z]+\\.(com|org|net)\","] #[doc =
    "     r\"[a-z]+\\.(com|org|net)\","] #[doc = " ]).unwrap();"] #[doc = ""] #[doc =
    " // Ask whether any regexes in the set match."] #[doc =
    " assert!(set.is_match(\"foo@example.com\"));"] #[doc = ""] #[doc =
    " // Identify which regexes in the set match."] #[doc =
    " let matches: Vec<_> = set.matches(\"foo@example.com\").into_iter().collect();"]
    #[doc = " assert_eq!(vec![0, 1], matches);"] #[doc = ""] #[doc =
    " // Try again, but with text that only matches one of the regexes."] #[doc =
    " let matches: Vec<_> = set.matches(\"example.com\").into_iter().collect();"] #[doc =
    " assert_eq!(vec![1], matches);"] #[doc = ""] #[doc =
    " // Try again, but with text that doesn't match any regex in the set."] #[doc =
    " let matches: Vec<_> = set.matches(\"example\").into_iter().collect();"] #[doc =
    " assert!(matches.is_empty());"] #[doc = " ```"]
}
define_set! {
    bytes, set_bytes, & [u8], as_bytes_bytes, #[doc = " ```rust"] #[doc =
    " # use regex::bytes::RegexSet;"] #[doc = " let set = RegexSet::new(&["] #[doc =
    "     r\"[a-z]+@[a-z]+\\.(com|org|net)\","] #[doc =
    "     r\"[a-z]+\\.(com|org|net)\","] #[doc = " ]).unwrap();"] #[doc = ""] #[doc =
    " // Ask whether any regexes in the set match."] #[doc =
    " assert!(set.is_match(b\"foo@example.com\"));"] #[doc = ""] #[doc =
    " // Identify which regexes in the set match."] #[doc =
    " let matches: Vec<_> = set.matches(b\"foo@example.com\").into_iter().collect();"]
    #[doc = " assert_eq!(vec![0, 1], matches);"] #[doc = ""] #[doc =
    " // Try again, but with text that only matches one of the regexes."] #[doc =
    " let matches: Vec<_> = set.matches(b\"example.com\").into_iter().collect();"] #[doc
    = " assert_eq!(vec![1], matches);"] #[doc = ""] #[doc =
    " // Try again, but with text that doesn't match any regex in the set."] #[doc =
    " let matches: Vec<_> = set.matches(b\"example\").into_iter().collect();"] #[doc =
    " assert!(matches.is_empty());"] #[doc = " ```"]
}
#[cfg(test)]
mod tests_llm_16_641_llm_16_640 {
    use super::*;
    use crate::*;
    use crate::bytes::RegexSet;
    use crate::Error;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::fmt::{self, Debug};
    use std::ops::Deref;
    use std::rc::Rc;
    #[test]
    fn test_empty() {
        let _rug_st_tests_llm_16_641_llm_16_640_rrrruuuugggg_test_empty = 0;
        let set: RegexSet = RegexSet::empty();
        debug_assert!(set.is_empty());
        let _rug_ed_tests_llm_16_641_llm_16_640_rrrruuuugggg_test_empty = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_642 {
    use crate::re_set::bytes::RegexSet;
    #[test]
    fn test_is_empty() {
        let _rug_st_tests_llm_16_642_rrrruuuugggg_test_is_empty = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = "a";
        let rug_fuzz_2 = "b";
        let rug_fuzz_3 = "c";
        let set = RegexSet::empty();
        debug_assert!(set.is_empty());
        let set = RegexSet::new(&[rug_fuzz_0; 0]).unwrap();
        debug_assert!(set.is_empty());
        let set = RegexSet::new(&[rug_fuzz_1, rug_fuzz_2, rug_fuzz_3]).unwrap();
        debug_assert!(! set.is_empty());
        let _rug_ed_tests_llm_16_642_rrrruuuugggg_test_is_empty = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_646_llm_16_645 {
    use crate::re_set::bytes::RegexSet;
    #[test]
    fn test_is_match_at() {
        let _rug_st_tests_llm_16_646_llm_16_645_rrrruuuugggg_test_is_match_at = 0;
        let rug_fuzz_0 = r"\w+";
        let rug_fuzz_1 = r"\d+";
        let rug_fuzz_2 = r"\pL+";
        let rug_fuzz_3 = r"foo";
        let rug_fuzz_4 = r"bar";
        let rug_fuzz_5 = r"barfoo";
        let rug_fuzz_6 = r"foobar";
        let rug_fuzz_7 = "foobar";
        let rug_fuzz_8 = 0;
        let regex_set = RegexSet::new(
                &[
                    rug_fuzz_0,
                    rug_fuzz_1,
                    rug_fuzz_2,
                    rug_fuzz_3,
                    rug_fuzz_4,
                    rug_fuzz_5,
                    rug_fuzz_6,
                ],
            )
            .unwrap();
        let text = rug_fuzz_7;
        let start = rug_fuzz_8;
        let is_match = regex_set.is_match_at(text.as_bytes(), start);
        debug_assert_eq!(is_match, true);
        let _rug_ed_tests_llm_16_646_llm_16_645_rrrruuuugggg_test_is_match_at = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_650_llm_16_649 {
    use crate::re_set::bytes::RegexSet;
    #[test]
    fn test_matches() {
        let _rug_st_tests_llm_16_650_llm_16_649_rrrruuuugggg_test_matches = 0;
        let rug_fuzz_0 = r"\w+";
        let rug_fuzz_1 = r"\d+";
        let rug_fuzz_2 = r"\pL+";
        let rug_fuzz_3 = r"foo";
        let rug_fuzz_4 = r"bar";
        let rug_fuzz_5 = r"barfoo";
        let rug_fuzz_6 = r"foobar";
        let rug_fuzz_7 = b"foobar";
        let rug_fuzz_8 = b"foobar";
        let rug_fuzz_9 = 5;
        let rug_fuzz_10 = 6;
        let set = RegexSet::new(
                &[
                    rug_fuzz_0,
                    rug_fuzz_1,
                    rug_fuzz_2,
                    rug_fuzz_3,
                    rug_fuzz_4,
                    rug_fuzz_5,
                    rug_fuzz_6,
                ],
            )
            .unwrap();
        let matches: Vec<_> = set.matches(rug_fuzz_7).into_iter().collect();
        debug_assert_eq!(matches, vec![0, 2, 3, 4, 6]);
        let matches = set.matches(rug_fuzz_8);
        debug_assert!(! matches.matched(rug_fuzz_9));
        debug_assert!(matches.matched(rug_fuzz_10));
        let _rug_ed_tests_llm_16_650_llm_16_649_rrrruuuugggg_test_matches = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_651 {
    use super::*;
    use crate::*;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_651_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = "\\w+";
        let rug_fuzz_1 = "\\d+";
        let rug_fuzz_2 = "foo";
        let exprs = [rug_fuzz_0, rug_fuzz_1];
        let set = RegexSet::new(&exprs).unwrap();
        debug_assert!(set.is_match(rug_fuzz_2));
        let _rug_ed_tests_llm_16_651_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_653 {
    use crate::re_set::bytes::RegexSet;
    #[test]
    fn test_patterns() {
        let _rug_st_tests_llm_16_653_rrrruuuugggg_test_patterns = 0;
        let rug_fuzz_0 = r"\w+";
        let rug_fuzz_1 = r"\d+";
        let rug_fuzz_2 = r"\pL+";
        let rug_fuzz_3 = r"foo";
        let rug_fuzz_4 = r"bar";
        let rug_fuzz_5 = r"barfoo";
        let rug_fuzz_6 = r"foobar";
        let rug_fuzz_7 = b"foobar";
        let set = RegexSet::new(
                &[
                    rug_fuzz_0,
                    rug_fuzz_1,
                    rug_fuzz_2,
                    rug_fuzz_3,
                    rug_fuzz_4,
                    rug_fuzz_5,
                    rug_fuzz_6,
                ],
            )
            .unwrap();
        let matches: Vec<_> = set
            .matches(rug_fuzz_7)
            .into_iter()
            .map(|match_idx| &set.patterns()[match_idx])
            .collect();
        debug_assert_eq!(matches, vec![r"\w+", r"\pL+", "foo", "bar", "foobar"]);
        let _rug_ed_tests_llm_16_653_rrrruuuugggg_test_patterns = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_654 {
    use super::*;
    use crate::*;
    use crate::bytes::RegexSetBuilder;
    #[test]
    fn test_read_matches_at() {
        let _rug_st_tests_llm_16_654_rrrruuuugggg_test_read_matches_at = 0;
        let rug_fuzz_0 = r"\w+";
        let rug_fuzz_1 = r"\d+";
        let rug_fuzz_2 = r"\pL+";
        let rug_fuzz_3 = r"foo";
        let rug_fuzz_4 = r"bar";
        let rug_fuzz_5 = true;
        let rug_fuzz_6 = "Match this string";
        let rug_fuzz_7 = 0;
        let regexes = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3, rug_fuzz_4];
        let regex_set = RegexSetBuilder::new(regexes)
            .case_insensitive(rug_fuzz_5)
            .build()
            .unwrap();
        let text = rug_fuzz_6;
        let mut matches = vec![false; regex_set.len()];
        let result = regex_set
            .read_matches_at(&mut matches, text.as_bytes(), rug_fuzz_7);
        debug_assert!(result);
        debug_assert_eq!(matches, [true, false, false, false, false]);
        let _rug_ed_tests_llm_16_654_rrrruuuugggg_test_read_matches_at = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_667 {
    use super::*;
    use crate::*;
    use crate::RegexSet;
    #[test]
    fn test_empty() {
        let _rug_st_tests_llm_16_667_rrrruuuugggg_test_empty = 0;
        let set = RegexSet::empty();
        debug_assert!(set.is_empty());
        let _rug_ed_tests_llm_16_667_rrrruuuugggg_test_empty = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_668 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_empty_empty_set() {
        let _rug_st_tests_llm_16_668_rrrruuuugggg_test_is_empty_empty_set = 0;
        let regex_set = RegexSet::empty();
        debug_assert_eq!(regex_set.is_empty(), true);
        let _rug_ed_tests_llm_16_668_rrrruuuugggg_test_is_empty_empty_set = 0;
    }
    #[test]
    fn test_is_empty_non_empty_set() {
        let _rug_st_tests_llm_16_668_rrrruuuugggg_test_is_empty_non_empty_set = 0;
        let rug_fuzz_0 = "abc";
        let rug_fuzz_1 = "def";
        let regex_set = RegexSet::new(&[rug_fuzz_0, rug_fuzz_1]).unwrap();
        debug_assert_eq!(regex_set.is_empty(), false);
        let _rug_ed_tests_llm_16_668_rrrruuuugggg_test_is_empty_non_empty_set = 0;
    }
    #[test]
    fn test_is_empty_empty_set_cloned() {
        let _rug_st_tests_llm_16_668_rrrruuuugggg_test_is_empty_empty_set_cloned = 0;
        let regex_set = RegexSet::empty();
        let cloned_set = regex_set.clone();
        debug_assert_eq!(cloned_set.is_empty(), true);
        let _rug_ed_tests_llm_16_668_rrrruuuugggg_test_is_empty_empty_set_cloned = 0;
    }
    #[test]
    fn test_is_empty_non_empty_set_cloned() {
        let _rug_st_tests_llm_16_668_rrrruuuugggg_test_is_empty_non_empty_set_cloned = 0;
        let rug_fuzz_0 = "abc";
        let rug_fuzz_1 = "def";
        let regex_set = RegexSet::new(&[rug_fuzz_0, rug_fuzz_1]).unwrap();
        let cloned_set = regex_set.clone();
        debug_assert_eq!(cloned_set.is_empty(), false);
        let _rug_ed_tests_llm_16_668_rrrruuuugggg_test_is_empty_non_empty_set_cloned = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_671 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_match_at() {
        let _rug_st_tests_llm_16_671_rrrruuuugggg_test_is_match_at = 0;
        let rug_fuzz_0 = r"\w+";
        let rug_fuzz_1 = r"\d+";
        let rug_fuzz_2 = "foo";
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = "☃";
        let rug_fuzz_5 = 0;
        let set = RegexSet::new(&[rug_fuzz_0, rug_fuzz_1]).unwrap();
        debug_assert_eq!(set.is_match_at(rug_fuzz_2, rug_fuzz_3), true);
        debug_assert_eq!(set.is_match_at(rug_fuzz_4, rug_fuzz_5), false);
        let _rug_ed_tests_llm_16_671_rrrruuuugggg_test_is_match_at = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_672 {
    use crate::re_set::unicode::RegexSet;
    #[test]
    fn test_len() {
        let _rug_st_tests_llm_16_672_rrrruuuugggg_test_len = 0;
        let rug_fuzz_0 = r"\w+";
        let rug_fuzz_1 = r"\d+";
        let rug_fuzz_2 = r"\pL+";
        let rug_fuzz_3 = r"foo";
        let rug_fuzz_4 = r"bar";
        let rug_fuzz_5 = r"barfoo";
        let rug_fuzz_6 = r"foobar";
        let regex_set = RegexSet::new(
                &[
                    rug_fuzz_0,
                    rug_fuzz_1,
                    rug_fuzz_2,
                    rug_fuzz_3,
                    rug_fuzz_4,
                    rug_fuzz_5,
                    rug_fuzz_6,
                ],
            )
            .unwrap();
        let result = regex_set.len();
        debug_assert_eq!(result, 7);
        let _rug_ed_tests_llm_16_672_rrrruuuugggg_test_len = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_673 {
    use super::*;
    use crate::*;
    use crate::RegexSet;
    #[test]
    fn test_matches() {
        let _rug_st_tests_llm_16_673_rrrruuuugggg_test_matches = 0;
        let rug_fuzz_0 = r"\w+";
        let rug_fuzz_1 = r"\d+";
        let rug_fuzz_2 = r"\pL+";
        let rug_fuzz_3 = r"foo";
        let rug_fuzz_4 = r"bar";
        let rug_fuzz_5 = r"barfoo";
        let rug_fuzz_6 = r"foobar";
        let rug_fuzz_7 = "foobar";
        let rug_fuzz_8 = "foobar";
        let rug_fuzz_9 = 5;
        let rug_fuzz_10 = 6;
        let set = RegexSet::new(
                &[
                    rug_fuzz_0,
                    rug_fuzz_1,
                    rug_fuzz_2,
                    rug_fuzz_3,
                    rug_fuzz_4,
                    rug_fuzz_5,
                    rug_fuzz_6,
                ],
            )
            .unwrap();
        let matches: Vec<_> = set.matches(rug_fuzz_7).into_iter().collect();
        debug_assert_eq!(matches, vec![0, 2, 3, 4, 6]);
        let matches = set.matches(rug_fuzz_8);
        debug_assert!(! matches.matched(rug_fuzz_9));
        debug_assert!(matches.matched(rug_fuzz_10));
        let _rug_ed_tests_llm_16_673_rrrruuuugggg_test_matches = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_674 {
    use super::*;
    use crate::*;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_674_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = r"\w+";
        let rug_fuzz_1 = r"\d+";
        let rug_fuzz_2 = "foo";
        let set = RegexSet::new(&[rug_fuzz_0, rug_fuzz_1]).unwrap();
        debug_assert!(set.is_match(rug_fuzz_2));
        let _rug_ed_tests_llm_16_674_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_675 {
    use super::*;
    use crate::*;
    #[test]
    fn test_patterns() {
        let _rug_st_tests_llm_16_675_rrrruuuugggg_test_patterns = 0;
        let rug_fuzz_0 = r"\w+";
        let rug_fuzz_1 = r"\d+";
        let rug_fuzz_2 = r"\pL+";
        let rug_fuzz_3 = r"foo";
        let rug_fuzz_4 = r"bar";
        let rug_fuzz_5 = r"barfoo";
        let rug_fuzz_6 = r"foobar";
        let rug_fuzz_7 = "foobar";
        let set = RegexSet::new(
                &[
                    rug_fuzz_0,
                    rug_fuzz_1,
                    rug_fuzz_2,
                    rug_fuzz_3,
                    rug_fuzz_4,
                    rug_fuzz_5,
                    rug_fuzz_6,
                ],
            )
            .unwrap();
        let matches: Vec<_> = set
            .matches(rug_fuzz_7)
            .into_iter()
            .map(|match_idx| &set.patterns()[match_idx])
            .collect();
        debug_assert_eq!(matches, vec![r"\w+", r"\pL+", r"foo", r"bar", r"foobar"]);
        let _rug_ed_tests_llm_16_675_rrrruuuugggg_test_patterns = 0;
    }
}
#[cfg(test)]
mod tests_rug_226 {
    use super::*;
    use crate::{Error, RegexSet};
    #[test]
    fn test_regex_set_is_match() {
        let _rug_st_tests_rug_226_rrrruuuugggg_test_regex_set_is_match = 0;
        let rug_fuzz_0 = "[0-9]+";
        let rug_fuzz_1 = "[a-z]+";
        let rug_fuzz_2 = "[A-Z]+";
        let rug_fuzz_3 = "Hello World";
        let patterns = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        let regex_set = RegexSet::new(patterns).unwrap();
        let text = rug_fuzz_3;
        debug_assert!(regex_set.is_match(text));
        let _rug_ed_tests_rug_226_rrrruuuugggg_test_regex_set_is_match = 0;
    }
}
#[cfg(test)]
mod tests_rug_227 {
    use super::*;
    use crate::{Error, RegexSet, RegexSetBuilder};
    #[test]
    fn test_read_matches_at() {
        let _rug_st_tests_rug_227_rrrruuuugggg_test_read_matches_at = 0;
        let rug_fuzz_0 = "[0-9]+";
        let rug_fuzz_1 = "[a-z]+";
        let rug_fuzz_2 = "[A-Z]+";
        let rug_fuzz_3 = false;
        let rug_fuzz_4 = "abc123ABC";
        let rug_fuzz_5 = 0;
        let patterns = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        let regex_set = RegexSet::new(patterns).unwrap();
        let mut matches = [rug_fuzz_3; 3];
        let text = rug_fuzz_4;
        let start = rug_fuzz_5;
        regex_set.read_matches_at(&mut matches, text, start);
        let _rug_ed_tests_rug_227_rrrruuuugggg_test_read_matches_at = 0;
    }
}
#[cfg(test)]
mod tests_rug_234 {
    use super::*;
    use crate::std::iter::Iterator;
    use crate::re_set::unicode::SetMatchesIntoIter;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_234_rrrruuuugggg_test_rug = 0;
        let mut p0: SetMatchesIntoIter = unimplemented!();
        SetMatchesIntoIter::next(&mut p0);
        let _rug_ed_tests_rug_234_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_235 {
    use super::*;
    use crate::re_set::unicode::SetMatchesIntoIter;
    use std::iter::Iterator;
    #[test]
    fn test_size_hint() {
        let _rug_st_tests_rug_235_rrrruuuugggg_test_size_hint = 0;
        let p0: SetMatchesIntoIter = unimplemented!();
        <SetMatchesIntoIter as Iterator>::size_hint(&p0);
        let _rug_ed_tests_rug_235_rrrruuuugggg_test_size_hint = 0;
    }
}
#[cfg(test)]
mod tests_rug_236 {
    use super::*;
    use crate::std::iter::DoubleEndedIterator;
    use re_set::unicode::SetMatchesIntoIter;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_236_rrrruuuugggg_test_rug = 0;
        let mut p0: SetMatchesIntoIter = unimplemented!();
        p0.next_back();
        let _rug_ed_tests_rug_236_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_241 {
    use super::*;
    use crate::bytes::RegexSet;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_241_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = r"\w+";
        let rug_fuzz_1 = r"\d+";
        let rug_fuzz_2 = b"foo";
        let mut p0 = RegexSet::new(&[rug_fuzz_0, rug_fuzz_1]).unwrap();
        let p1: &[u8] = rug_fuzz_2;
        p0.is_match(p1);
        let _rug_ed_tests_rug_241_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_242 {
    use super::*;
    use crate::bytes::RegexSet;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_242_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = r"\w+";
        let rug_fuzz_1 = r"\d+";
        let mut p0 = RegexSet::new(&[rug_fuzz_0, rug_fuzz_1]).unwrap();
        p0.len();
        let _rug_ed_tests_rug_242_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_244 {
    use super::*;
    use crate::bytes::RegexSet;
    use crate::bytes::Regex;
    #[test]
    fn test_matched() {
        let _rug_st_tests_rug_244_rrrruuuugggg_test_matched = 0;
        let rug_fuzz_0 = r"abc";
        let rug_fuzz_1 = r"def";
        let rug_fuzz_2 = b"abcdef";
        let rug_fuzz_3 = 0;
        let set = RegexSet::new(&[rug_fuzz_0, rug_fuzz_1]).unwrap();
        let matches = set.matches(rug_fuzz_2);
        let regex_index = rug_fuzz_3;
        debug_assert_eq!(matches.matched(regex_index), true);
        let _rug_ed_tests_rug_244_rrrruuuugggg_test_matched = 0;
    }
}
#[cfg(test)]
mod tests_rug_249 {
    use super::*;
    use crate::std::iter::Iterator;
    use re_set::bytes::SetMatchesIntoIter;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_249_rrrruuuugggg_test_rug = 0;
        let mut p0: SetMatchesIntoIter = todo!();
        SetMatchesIntoIter::next(&mut p0);
        let _rug_ed_tests_rug_249_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_250 {
    use super::*;
    use crate::std::iter::Iterator;
    use re_set::bytes::SetMatchesIntoIter;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_250_rrrruuuugggg_test_rug = 0;
        let mut p0: SetMatchesIntoIter = unimplemented!();
        p0.size_hint();
        let _rug_ed_tests_rug_250_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_251 {
    use super::*;
    use crate::std::iter::DoubleEndedIterator;
    use re_set::bytes::SetMatchesIntoIter;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_251_rrrruuuugggg_test_rug = 0;
        let mut p0: SetMatchesIntoIter = unimplemented!();
        p0.next_back();
        let _rug_ed_tests_rug_251_rrrruuuugggg_test_rug = 0;
    }
}
