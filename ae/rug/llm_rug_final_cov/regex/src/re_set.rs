macro_rules! define_set {
    (
        $name:ident, $builder_mod:ident, $text_ty:ty, $as_bytes:expr,
        $(#[$doc_regexset_example:meta])*
    ) => {
        pub mod $name { use std::fmt; use std::iter; use std::slice; use std::vec; use
        crate ::error::Error; use crate ::exec::Exec; use crate
        ::re_builder::$builder_mod ::RegexSetBuilder; use crate
        ::re_trait::RegularExpression; #[doc =
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
        #[doc = " with an expression like:"] #[doc = ""] #[doc = " ```text"] #[doc =
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
        " As with the main [`Regex`][crate::Regex] type, it is cheaper to ask (1)"] #[doc
        = " instead of (2) since the matching engines can stop after the first match"]
        #[doc = " is found."] #[doc = ""] #[doc =
        " You cannot directly extract [`Match`][crate::Match] or"] #[doc =
        " [`Captures`][crate::Captures] objects from a regex set. If you need these"]
        #[doc =
        " operations, the recommended approach is to compile each pattern in the set"]
        #[doc = " independently and scan the exact same input a second time with those"]
        #[doc = " independently compiled patterns:"] #[doc = ""] #[doc = " ```rust"]
        #[doc = " use regex::{Regex, RegexSet};"] #[doc = ""] #[doc =
        " let patterns = [\"foo\", \"bar\"];"] #[doc =
        " // Both patterns will match different ranges of this string."] #[doc =
        " let text = \"barfoo\";"] #[doc = ""] #[doc =
        " // Compile a set matching any of our patterns."] #[doc =
        " let set = RegexSet::new(&patterns).unwrap();"] #[doc =
        " // Compile each pattern independently."] #[doc =
        " let regexes: Vec<_> = set.patterns().iter()"] #[doc =
        "     .map(|pat| Regex::new(pat).unwrap())"] #[doc = "     .collect();"] #[doc =
        ""] #[doc = " // Match against the whole set first and identify the individual"]
        #[doc = " // matching patterns."] #[doc =
        " let matches: Vec<&str> = set.matches(text).into_iter()"] #[doc =
        "     // Dereference the match index to get the corresponding"] #[doc =
        "     // compiled pattern."] #[doc =
        "     .map(|match_idx| &regexes[match_idx])"] #[doc =
        "     // To get match locations or any other info, we then have to search"] #[doc
        = "     // the exact same text again, using our separately-compiled pattern."]
        #[doc = "     .map(|pat| pat.find(text).unwrap().as_str())"] #[doc =
        "     .collect();"] #[doc = ""] #[doc =
        " // Matches arrive in the order the constituent patterns were declared,"] #[doc
        = " // not the order they appear in the input."] #[doc =
        " assert_eq!(vec![\"foo\", \"bar\"], matches);"] #[doc = " ```"] #[doc = ""]
        #[doc = " # Performance"] #[doc = ""] #[doc =
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
        } impl Default for RegexSet { fn default() -> Self { RegexSet::empty() } } #[doc
        = " A set of matches returned by a regex set."] #[derive(Clone, Debug)] pub
        struct SetMatches { matched_any : bool, matches : Vec < bool >, } impl SetMatches
        { #[doc = " Whether this set contains any matches."] pub fn matched_any(& self)
        -> bool { self.matched_any } #[doc =
        " Whether the regex at the given index matched."] #[doc = ""] #[doc =
        " The index for a regex is determined by its insertion order upon the"] #[doc =
        " initial construction of a `RegexSet`, starting at `0`."] #[doc = ""] #[doc =
        " # Panics"] #[doc = ""] #[doc =
        " If `regex_index` is greater than or equal to `self.len()`."] pub fn matched(&
        self, regex_index : usize) -> bool { self.matches[regex_index] } #[doc =
        " The total number of regexes in the set that created these matches."] #[doc =
        ""] #[doc =
        " **WARNING:** This always returns the same value as [`RegexSet::len`]."] #[doc =
        " In particular, it does *not* return the number of elements yielded by"] #[doc =
        " [`SetMatches::iter`]. The only way to determine the total number of"] #[doc =
        " matched regexes is to iterate over them."] pub fn len(& self) -> usize { self
        .matches.len() } #[doc =
        " Returns an iterator over indexes in the regex that matched."] #[doc = ""] #[doc
        = " This will always produces matches in ascending order of index, where"] #[doc
        = " the index corresponds to the index of the regex that matched with"] #[doc =
        " respect to its position when initially building the set."] pub fn iter(& self)
        -> SetMatchesIter <'_ > { SetMatchesIter((&* self.matches).into_iter()
        .enumerate()) } } impl IntoIterator for SetMatches { type IntoIter =
        SetMatchesIntoIter; type Item = usize; fn into_iter(self) -> Self::IntoIter {
        SetMatchesIntoIter(self.matches.into_iter().enumerate()) } } impl <'a >
        IntoIterator for &'a SetMatches { type IntoIter = SetMatchesIter <'a >; type Item
        = usize; fn into_iter(self) -> Self::IntoIter { self.iter() } } #[doc =
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
        for RegexSet { fn fmt(& self, f : & mut fmt::Formatter <'_ >) -> fmt::Result {
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
mod tests_rug_336 {
    use super::*;
    use crate::{Error, RegexSet, RegexSetBuilder};
    #[test]
    fn test_regex_set_new() {
        let _rug_st_tests_rug_336_rrrruuuugggg_test_regex_set_new = 0;
        let rug_fuzz_0 = "\\w+";
        let rug_fuzz_1 = "\\d+";
        let regexes = &[rug_fuzz_0, rug_fuzz_1];
        let result = RegexSetBuilder::new(regexes).build();
        debug_assert!(result.is_ok());
        let set: Result<RegexSet, Error> = result;
        debug_assert!(set.is_ok());
        let _rug_ed_tests_rug_336_rrrruuuugggg_test_regex_set_new = 0;
    }
}
#[cfg(test)]
mod tests_rug_337 {
    use super::*;
    use crate::RegexSet;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_337_rrrruuuugggg_test_rug = 0;
        RegexSet::empty();
        let _rug_ed_tests_rug_337_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_338 {
    use super::*;
    use crate::RegexSet;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_338_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "regex1";
        let rug_fuzz_1 = "regex2";
        let rug_fuzz_2 = "text";
        let mut p0: RegexSet = RegexSet::new(&[rug_fuzz_0, rug_fuzz_1]).unwrap();
        let p1: &str = rug_fuzz_2;
        p0.is_match(p1);
        let _rug_ed_tests_rug_338_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_339 {
    use super::*;
    use crate::RegexSet;
    #[test]
    fn test_is_match_at() {
        let _rug_st_tests_rug_339_rrrruuuugggg_test_is_match_at = 0;
        let rug_fuzz_0 = "regex1";
        let rug_fuzz_1 = "regex2";
        let rug_fuzz_2 = "sample_text";
        let rug_fuzz_3 = 0;
        let v86: RegexSet = RegexSet::new(&[rug_fuzz_0, rug_fuzz_1]).unwrap();
        let p0: &RegexSet = &v86;
        let p1: &str = rug_fuzz_2;
        let p2: usize = rug_fuzz_3;
        p0.is_match_at(p1, p2);
        let _rug_ed_tests_rug_339_rrrruuuugggg_test_is_match_at = 0;
    }
}
#[cfg(test)]
mod tests_rug_340 {
    use super::*;
    use crate::RegexSet;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_340_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "\\w+";
        let rug_fuzz_1 = "\\d+";
        let rug_fuzz_2 = "\\pL+";
        let rug_fuzz_3 = "foo";
        let rug_fuzz_4 = "bar";
        let rug_fuzz_5 = "barfoo";
        let rug_fuzz_6 = "foobar";
        let rug_fuzz_7 = "foobar";
        let set: RegexSet = RegexSet::new(
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
        let text: &str = rug_fuzz_7;
        set.matches(text);
        let _rug_ed_tests_rug_340_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_341 {
    use super::*;
    use crate::RegexSet;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_341_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "regex1";
        let rug_fuzz_1 = "regex2";
        let rug_fuzz_2 = false;
        let rug_fuzz_3 = false;
        let rug_fuzz_4 = "test_string";
        let rug_fuzz_5 = 0;
        let mut p0: RegexSet = RegexSet::new(&[rug_fuzz_0, rug_fuzz_1]).unwrap();
        let mut p1: [bool; 2] = [rug_fuzz_2, rug_fuzz_3];
        let mut p2: &str = rug_fuzz_4;
        let mut p3: usize = rug_fuzz_5;
        p0.read_matches_at(&mut p1, &p2, p3);
        let _rug_ed_tests_rug_341_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_342 {
    use super::*;
    use crate::RegexSet;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_342_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "regex1";
        let rug_fuzz_1 = "regex2";
        let v86: RegexSet = RegexSet::new(&[rug_fuzz_0, rug_fuzz_1]).unwrap();
        let p0 = &v86;
        crate::re_set::unicode::RegexSet::len(p0);
        let _rug_ed_tests_rug_342_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_343 {
    use super::*;
    use crate::RegexSet;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_343_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "regex1";
        let rug_fuzz_1 = "regex2";
        let mut p0: RegexSet = RegexSet::new(&[rug_fuzz_0, rug_fuzz_1]).unwrap();
        crate::re_set::unicode::RegexSet::is_empty(&p0);
        let _rug_ed_tests_rug_343_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_344 {
    use super::*;
    use crate::RegexSet;
    #[test]
    fn test_regex_set_patterns() {
        let _rug_st_tests_rug_344_rrrruuuugggg_test_regex_set_patterns = 0;
        let rug_fuzz_0 = r"\w+";
        let patterns = vec![
            rug_fuzz_0, r"\d+", r"\pL+", r"foo", r"bar", r"barfoo", r"foobar"
        ];
        let set = RegexSet::new(&patterns).unwrap();
        debug_assert_eq!(set.patterns(), & patterns[..]);
        let _rug_ed_tests_rug_344_rrrruuuugggg_test_regex_set_patterns = 0;
    }
}
#[cfg(test)]
mod tests_rug_359 {
    use super::*;
    use crate::{RegexSet, Error};
    #[test]
    fn test_regex_set_new() {
        let _rug_st_tests_rug_359_rrrruuuugggg_test_regex_set_new = 0;
        let rug_fuzz_0 = "\\w+";
        let exprs: Vec<&str> = vec![rug_fuzz_0, "\\d+"];
        let result = RegexSet::new(exprs.iter().cloned());
        debug_assert!(result.is_ok());
        let _rug_ed_tests_rug_359_rrrruuuugggg_test_regex_set_new = 0;
    }
}
#[cfg(test)]
mod tests_rug_360 {
    use super::*;
    use crate::RegexSet;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_360_rrrruuuugggg_test_rug = 0;
        RegexSet::empty();
        let _rug_ed_tests_rug_360_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_361 {
    use super::*;
    use crate::RegexSet;
    #[test]
    fn test_is_match() {
        let _rug_st_tests_rug_361_rrrruuuugggg_test_is_match = 0;
        let rug_fuzz_0 = r"\w+";
        let rug_fuzz_1 = r"\d+";
        let rug_fuzz_2 = "foo";
        let rug_fuzz_3 = "☃";
        let set = RegexSet::new(&[rug_fuzz_0, rug_fuzz_1]).unwrap();
        debug_assert!(set.is_match(rug_fuzz_2));
        debug_assert!(! set.is_match(rug_fuzz_3));
        let _rug_ed_tests_rug_361_rrrruuuugggg_test_is_match = 0;
    }
}
#[cfg(test)]
mod tests_rug_362 {
    use super::*;
    use crate::bytes::RegexSet;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_362_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "foo";
        let rug_fuzz_1 = b"foobar";
        let rug_fuzz_2 = 0;
        let mut p0 = RegexSet::new(vec![rug_fuzz_0]).unwrap();
        let p1: &[u8] = rug_fuzz_1;
        let p2: usize = rug_fuzz_2;
        p0.is_match_at(p1, p2);
        let _rug_ed_tests_rug_362_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_363 {
    use super::*;
    use crate::bytes::RegexSet;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_363_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = r"\w+";
        let rug_fuzz_1 = b"foobar";
        let patterns = vec![
            rug_fuzz_0, r"\d+", r"\pL+", r"foo", r"bar", r"barfoo", r"foobar"
        ];
        let regex_set = RegexSet::new(&patterns).unwrap();
        let text = rug_fuzz_1;
        regex_set.matches(text);
        let _rug_ed_tests_rug_363_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_364 {
    use super::*;
    use crate::bytes::RegexSet;
    #[test]
    fn test_read_matches_at() {
        let _rug_st_tests_rug_364_rrrruuuugggg_test_read_matches_at = 0;
        let rug_fuzz_0 = false;
        let rug_fuzz_1 = false;
        let rug_fuzz_2 = false;
        let rug_fuzz_3 = b"hello world";
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = r"hello";
        let rug_fuzz_6 = r"world";
        let mut matches = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        let text = rug_fuzz_3;
        let start = rug_fuzz_4;
        let regex_set = RegexSet::new(&[rug_fuzz_5, rug_fuzz_6]).unwrap();
        regex_set.read_matches_at(&mut matches, text, start);
        let _rug_ed_tests_rug_364_rrrruuuugggg_test_read_matches_at = 0;
    }
}
#[cfg(test)]
mod tests_rug_365 {
    use super::*;
    use crate::bytes::RegexSet;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_365_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "abc";
        let rug_fuzz_1 = "def";
        let mut p0 = RegexSet::new(&[rug_fuzz_0, rug_fuzz_1]).unwrap();
        RegexSet::len(&p0);
        let _rug_ed_tests_rug_365_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_366 {
    use super::*;
    use crate::bytes::RegexSet;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_366_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "abc";
        let rug_fuzz_1 = "def";
        let mut p0 = RegexSet::new(&[rug_fuzz_0, rug_fuzz_1]).unwrap();
        debug_assert_eq!(p0.is_empty(), false);
        let _rug_ed_tests_rug_366_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_367 {
    use super::*;
    use crate::RegexSet;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_367_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = r"\w+";
        let rug_fuzz_1 = r"\d+";
        let rug_fuzz_2 = r"\pL+";
        let rug_fuzz_3 = r"foo";
        let rug_fuzz_4 = r"bar";
        let rug_fuzz_5 = r"barfoo";
        let rug_fuzz_6 = r"foobar";
        let mut p0: RegexSet = RegexSet::new(
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
        p0.patterns();
        let _rug_ed_tests_rug_367_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_368 {
    use super::*;
    use crate::bytes::RegexSet;
    use std::default::Default;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_368_rrrruuuugggg_test_rug = 0;
        let regex_set: RegexSet = <RegexSet as Default>::default();
        let _rug_ed_tests_rug_368_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_371 {
    use super::*;
    use crate::bytes::RegexSet;
    use crate::bytes::Regex;
    use crate::bytes::SetMatches;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_371_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = r"a.*";
        let rug_fuzz_1 = r"b.*";
        let rug_fuzz_2 = b"abcdefghi";
        let set: RegexSet = RegexSet::new(&[rug_fuzz_0, rug_fuzz_1]).unwrap();
        let text = rug_fuzz_2;
        let matches: SetMatches = set.matches(text);
        debug_assert_eq!(matches.len(), 2);
        let _rug_ed_tests_rug_371_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_377 {
    use super::*;
    use crate::bytes::SetMatchesIntoIter;
    use std::iter::DoubleEndedIterator;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_377_rrrruuuugggg_test_rug = 0;
        let mut p0: SetMatchesIntoIter = unimplemented!();
        SetMatchesIntoIter::next_back(&mut p0);
        let _rug_ed_tests_rug_377_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_379 {
    use super::*;
    use crate::bytes::RegexSet;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_379_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "a";
        let rug_fuzz_1 = "b";
        let rug_fuzz_2 = b"abc";
        let set = RegexSet::new(&[rug_fuzz_0, rug_fuzz_1]).unwrap();
        let text = rug_fuzz_2;
        let matches = set.matches(text);
        let mut p0 = matches.into_iter();
        p0.size_hint();
        let _rug_ed_tests_rug_379_rrrruuuugggg_test_rug = 0;
    }
}
