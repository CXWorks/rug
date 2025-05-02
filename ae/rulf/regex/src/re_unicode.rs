use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use std::iter::FusedIterator;
use std::ops::{Index, Range};
use std::str::FromStr;
use std::sync::Arc;
use find_byte::find_byte;
use syntax;
use error::Error;
use exec::{Exec, ExecNoSyncStr};
use expand::expand_str;
use re_builder::unicode::RegexBuilder;
use re_trait::{self, RegularExpression, SubCapturesPosIter};
/// Escapes all regular expression meta characters in `text`.
///
/// The string returned may be safely used as a literal in a regular
/// expression.
pub fn escape(text: &str) -> String {
    syntax::escape(text)
}
/// Match represents a single match of a regex in a haystack.
///
/// The lifetime parameter `'t` refers to the lifetime of the matched text.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Match<'t> {
    text: &'t str,
    start: usize,
    end: usize,
}
impl<'t> Match<'t> {
    /// Returns the starting byte offset of the match in the haystack.
    #[inline]
    pub fn start(&self) -> usize {
        self.start
    }
    /// Returns the ending byte offset of the match in the haystack.
    #[inline]
    pub fn end(&self) -> usize {
        self.end
    }
    /// Returns the range over the starting and ending byte offsets of the
    /// match in the haystack.
    #[inline]
    pub fn range(&self) -> Range<usize> {
        self.start..self.end
    }
    /// Returns the matched text.
    #[inline]
    pub fn as_str(&self) -> &'t str {
        &self.text[self.range()]
    }
    /// Creates a new match from the given haystack and byte offsets.
    #[inline]
    fn new(haystack: &'t str, start: usize, end: usize) -> Match<'t> {
        Match {
            text: haystack,
            start: start,
            end: end,
        }
    }
}
impl<'t> From<Match<'t>> for &'t str {
    fn from(m: Match<'t>) -> &'t str {
        m.as_str()
    }
}
impl<'t> From<Match<'t>> for Range<usize> {
    fn from(m: Match<'t>) -> Range<usize> {
        m.range()
    }
}
/// A compiled regular expression for matching Unicode strings.
///
/// It is represented as either a sequence of bytecode instructions (dynamic)
/// or as a specialized Rust function (native). It can be used to search, split
/// or replace text. All searching is done with an implicit `.*?` at the
/// beginning and end of an expression. To force an expression to match the
/// whole string (or a prefix or a suffix), you must use an anchor like `^` or
/// `$` (or `\A` and `\z`).
///
/// While this crate will handle Unicode strings (whether in the regular
/// expression or in the search text), all positions returned are **byte
/// indices**. Every byte index is guaranteed to be at a Unicode code point
/// boundary.
///
/// The lifetimes `'r` and `'t` in this crate correspond to the lifetime of a
/// compiled regular expression and text to search, respectively.
///
/// The only methods that allocate new strings are the string replacement
/// methods. All other methods (searching and splitting) return borrowed
/// pointers into the string given.
///
/// # Examples
///
/// Find the location of a US phone number:
///
/// ```rust
/// # use regex::Regex;
/// let re = Regex::new("[0-9]{3}-[0-9]{3}-[0-9]{4}").unwrap();
/// let mat = re.find("phone: 111-222-3333").unwrap();
/// assert_eq!((mat.start(), mat.end()), (7, 19));
/// ```
///
/// # Using the `std::str::pattern` methods with `Regex`
///
/// > **Note**: This section requires that this crate is compiled with the
/// > `pattern` Cargo feature enabled, which **requires nightly Rust**.
///
/// Since `Regex` implements `Pattern`, you can use regexes with methods
/// defined on `&str`. For example, `is_match`, `find`, `find_iter`
/// and `split` can be replaced with `str::contains`, `str::find`,
/// `str::match_indices` and `str::split`.
///
/// Here are some examples:
///
/// ```rust,ignore
/// # use regex::Regex;
/// let re = Regex::new(r"\d+").unwrap();
/// let haystack = "a111b222c";
///
/// assert!(haystack.contains(&re));
/// assert_eq!(haystack.find(&re), Some(1));
/// assert_eq!(haystack.match_indices(&re).collect::<Vec<_>>(),
///            vec![(1, 4), (5, 8)]);
/// assert_eq!(haystack.split(&re).collect::<Vec<_>>(), vec!["a", "b", "c"]);
/// ```
#[derive(Clone)]
pub struct Regex(Exec);
impl fmt::Display for Regex {
    /// Shows the original regular expression.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
impl fmt::Debug for Regex {
    /// Shows the original regular expression.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
#[doc(hidden)]
impl From<Exec> for Regex {
    fn from(exec: Exec) -> Regex {
        Regex(exec)
    }
}
impl FromStr for Regex {
    type Err = Error;
    /// Attempts to parse a string into a regular expression
    fn from_str(s: &str) -> Result<Regex, Error> {
        Regex::new(s)
    }
}
/// Core regular expression methods.
impl Regex {
    /// Compiles a regular expression. Once compiled, it can be used repeatedly
    /// to search, split or replace text in a string.
    ///
    /// If an invalid expression is given, then an error is returned.
    pub fn new(re: &str) -> Result<Regex, Error> {
        RegexBuilder::new(re).build()
    }
    /// Returns true if and only if there is a match for the regex in the
    /// string given.
    ///
    /// It is recommended to use this method if all you need to do is test
    /// a match, since the underlying matching engine may be able to do less
    /// work.
    ///
    /// # Example
    ///
    /// Test if some text contains at least one word with exactly 13
    /// Unicode word characters:
    ///
    /// ```rust
    /// # extern crate regex; use regex::Regex;
    /// # fn main() {
    /// let text = "I categorically deny having triskaidekaphobia.";
    /// assert!(Regex::new(r"\b\w{13}\b").unwrap().is_match(text));
    /// # }
    /// ```
    pub fn is_match(&self, text: &str) -> bool {
        self.is_match_at(text, 0)
    }
    /// Returns the start and end byte range of the leftmost-first match in
    /// `text`. If no match exists, then `None` is returned.
    ///
    /// Note that this should only be used if you want to discover the position
    /// of the match. Testing the existence of a match is faster if you use
    /// `is_match`.
    ///
    /// # Example
    ///
    /// Find the start and end location of the first word with exactly 13
    /// Unicode word characters:
    ///
    /// ```rust
    /// # extern crate regex; use regex::Regex;
    /// # fn main() {
    /// let text = "I categorically deny having triskaidekaphobia.";
    /// let mat = Regex::new(r"\b\w{13}\b").unwrap().find(text).unwrap();
    /// assert_eq!(mat.start(), 2);
    /// assert_eq!(mat.end(), 15);
    /// # }
    /// ```
    pub fn find<'t>(&self, text: &'t str) -> Option<Match<'t>> {
        self.find_at(text, 0)
    }
    /// Returns an iterator for each successive non-overlapping match in
    /// `text`, returning the start and end byte indices with respect to
    /// `text`.
    ///
    /// # Example
    ///
    /// Find the start and end location of every word with exactly 13 Unicode
    /// word characters:
    ///
    /// ```rust
    /// # extern crate regex; use regex::Regex;
    /// # fn main() {
    /// let text = "Retroactively relinquishing remunerations is reprehensible.";
    /// for mat in Regex::new(r"\b\w{13}\b").unwrap().find_iter(text) {
    ///     println!("{:?}", mat);
    /// }
    /// # }
    /// ```
    pub fn find_iter<'r, 't>(&'r self, text: &'t str) -> Matches<'r, 't> {
        Matches(self.0.searcher_str().find_iter(text))
    }
    /// Returns the capture groups corresponding to the leftmost-first
    /// match in `text`. Capture group `0` always corresponds to the entire
    /// match. If no match is found, then `None` is returned.
    ///
    /// You should only use `captures` if you need access to the location of
    /// capturing group matches. Otherwise, `find` is faster for discovering
    /// the location of the overall match.
    ///
    /// # Examples
    ///
    /// Say you have some text with movie names and their release years,
    /// like "'Citizen Kane' (1941)". It'd be nice if we could search for text
    /// looking like that, while also extracting the movie name and its release
    /// year separately.
    ///
    /// ```rust
    /// # extern crate regex; use regex::Regex;
    /// # fn main() {
    /// let re = Regex::new(r"'([^']+)'\s+\((\d{4})\)").unwrap();
    /// let text = "Not my favorite movie: 'Citizen Kane' (1941).";
    /// let caps = re.captures(text).unwrap();
    /// assert_eq!(caps.get(1).unwrap().as_str(), "Citizen Kane");
    /// assert_eq!(caps.get(2).unwrap().as_str(), "1941");
    /// assert_eq!(caps.get(0).unwrap().as_str(), "'Citizen Kane' (1941)");
    /// // You can also access the groups by index using the Index notation.
    /// // Note that this will panic on an invalid index.
    /// assert_eq!(&caps[1], "Citizen Kane");
    /// assert_eq!(&caps[2], "1941");
    /// assert_eq!(&caps[0], "'Citizen Kane' (1941)");
    /// # }
    /// ```
    ///
    /// Note that the full match is at capture group `0`. Each subsequent
    /// capture group is indexed by the order of its opening `(`.
    ///
    /// We can make this example a bit clearer by using *named* capture groups:
    ///
    /// ```rust
    /// # extern crate regex; use regex::Regex;
    /// # fn main() {
    /// let re = Regex::new(r"'(?P<title>[^']+)'\s+\((?P<year>\d{4})\)")
    ///                .unwrap();
    /// let text = "Not my favorite movie: 'Citizen Kane' (1941).";
    /// let caps = re.captures(text).unwrap();
    /// assert_eq!(caps.name("title").unwrap().as_str(), "Citizen Kane");
    /// assert_eq!(caps.name("year").unwrap().as_str(), "1941");
    /// assert_eq!(caps.get(0).unwrap().as_str(), "'Citizen Kane' (1941)");
    /// // You can also access the groups by name using the Index notation.
    /// // Note that this will panic on an invalid group name.
    /// assert_eq!(&caps["title"], "Citizen Kane");
    /// assert_eq!(&caps["year"], "1941");
    /// assert_eq!(&caps[0], "'Citizen Kane' (1941)");
    ///
    /// # }
    /// ```
    ///
    /// Here we name the capture groups, which we can access with the `name`
    /// method or the `Index` notation with a `&str`. Note that the named
    /// capture groups are still accessible with `get` or the `Index` notation
    /// with a `usize`.
    ///
    /// The `0`th capture group is always unnamed, so it must always be
    /// accessed with `get(0)` or `[0]`.
    pub fn captures<'t>(&self, text: &'t str) -> Option<Captures<'t>> {
        let mut locs = self.capture_locations();
        self.captures_read_at(&mut locs, text, 0)
            .map(move |_| Captures {
                text: text,
                locs: locs.0,
                named_groups: self.0.capture_name_idx().clone(),
            })
    }
    /// Returns an iterator over all the non-overlapping capture groups matched
    /// in `text`. This is operationally the same as `find_iter`, except it
    /// yields information about capturing group matches.
    ///
    /// # Example
    ///
    /// We can use this to find all movie titles and their release years in
    /// some text, where the movie is formatted like "'Title' (xxxx)":
    ///
    /// ```rust
    /// # extern crate regex; use regex::Regex;
    /// # fn main() {
    /// let re = Regex::new(r"'(?P<title>[^']+)'\s+\((?P<year>\d{4})\)")
    ///                .unwrap();
    /// let text = "'Citizen Kane' (1941), 'The Wizard of Oz' (1939), 'M' (1931).";
    /// for caps in re.captures_iter(text) {
    ///     println!("Movie: {:?}, Released: {:?}",
    ///              &caps["title"], &caps["year"]);
    /// }
    /// // Output:
    /// // Movie: Citizen Kane, Released: 1941
    /// // Movie: The Wizard of Oz, Released: 1939
    /// // Movie: M, Released: 1931
    /// # }
    /// ```
    pub fn captures_iter<'r, 't>(&'r self, text: &'t str) -> CaptureMatches<'r, 't> {
        CaptureMatches(self.0.searcher_str().captures_iter(text))
    }
    /// Returns an iterator of substrings of `text` delimited by a match of the
    /// regular expression. Namely, each element of the iterator corresponds to
    /// text that *isn't* matched by the regular expression.
    ///
    /// This method will *not* copy the text given.
    ///
    /// # Example
    ///
    /// To split a string delimited by arbitrary amounts of spaces or tabs:
    ///
    /// ```rust
    /// # extern crate regex; use regex::Regex;
    /// # fn main() {
    /// let re = Regex::new(r"[ \t]+").unwrap();
    /// let fields: Vec<&str> = re.split("a b \t  c\td    e").collect();
    /// assert_eq!(fields, vec!["a", "b", "c", "d", "e"]);
    /// # }
    /// ```
    pub fn split<'r, 't>(&'r self, text: &'t str) -> Split<'r, 't> {
        Split {
            finder: self.find_iter(text),
            last: 0,
        }
    }
    /// Returns an iterator of at most `limit` substrings of `text` delimited
    /// by a match of the regular expression. (A `limit` of `0` will return no
    /// substrings.) Namely, each element of the iterator corresponds to text
    /// that *isn't* matched by the regular expression. The remainder of the
    /// string that is not split will be the last element in the iterator.
    ///
    /// This method will *not* copy the text given.
    ///
    /// # Example
    ///
    /// Get the first two words in some text:
    ///
    /// ```rust
    /// # extern crate regex; use regex::Regex;
    /// # fn main() {
    /// let re = Regex::new(r"\W+").unwrap();
    /// let fields: Vec<&str> = re.splitn("Hey! How are you?", 3).collect();
    /// assert_eq!(fields, vec!("Hey", "How", "are you?"));
    /// # }
    /// ```
    pub fn splitn<'r, 't>(&'r self, text: &'t str, limit: usize) -> SplitN<'r, 't> {
        SplitN {
            splits: self.split(text),
            n: limit,
        }
    }
    /// Replaces the leftmost-first match with the replacement provided.
    /// The replacement can be a regular string (where `$N` and `$name` are
    /// expanded to match capture groups) or a function that takes the matches'
    /// `Captures` and returns the replaced string.
    ///
    /// If no match is found, then a copy of the string is returned unchanged.
    ///
    /// # Replacement string syntax
    ///
    /// All instances of `$name` in the replacement text is replaced with the
    /// corresponding capture group `name`.
    ///
    /// `name` may be an integer corresponding to the index of the
    /// capture group (counted by order of opening parenthesis where `0` is the
    /// entire match) or it can be a name (consisting of letters, digits or
    /// underscores) corresponding to a named capture group.
    ///
    /// If `name` isn't a valid capture group (whether the name doesn't exist
    /// or isn't a valid index), then it is replaced with the empty string.
    ///
    /// The longest possible name is used. e.g., `$1a` looks up the capture
    /// group named `1a` and not the capture group at index `1`. To exert more
    /// precise control over the name, use braces, e.g., `${1}a`.
    ///
    /// To write a literal `$` use `$$`.
    ///
    /// # Examples
    ///
    /// Note that this function is polymorphic with respect to the replacement.
    /// In typical usage, this can just be a normal string:
    ///
    /// ```rust
    /// # extern crate regex; use regex::Regex;
    /// # fn main() {
    /// let re = Regex::new("[^01]+").unwrap();
    /// assert_eq!(re.replace("1078910", ""), "1010");
    /// # }
    /// ```
    ///
    /// But anything satisfying the `Replacer` trait will work. For example,
    /// a closure of type `|&Captures| -> String` provides direct access to the
    /// captures corresponding to a match. This allows one to access
    /// capturing group matches easily:
    ///
    /// ```rust
    /// # extern crate regex; use regex::Regex;
    /// # use regex::Captures; fn main() {
    /// let re = Regex::new(r"([^,\s]+),\s+(\S+)").unwrap();
    /// let result = re.replace("Springsteen, Bruce", |caps: &Captures| {
    ///     format!("{} {}", &caps[2], &caps[1])
    /// });
    /// assert_eq!(result, "Bruce Springsteen");
    /// # }
    /// ```
    ///
    /// But this is a bit cumbersome to use all the time. Instead, a simple
    /// syntax is supported that expands `$name` into the corresponding capture
    /// group. Here's the last example, but using this expansion technique
    /// with named capture groups:
    ///
    /// ```rust
    /// # extern crate regex; use regex::Regex;
    /// # fn main() {
    /// let re = Regex::new(r"(?P<last>[^,\s]+),\s+(?P<first>\S+)").unwrap();
    /// let result = re.replace("Springsteen, Bruce", "$first $last");
    /// assert_eq!(result, "Bruce Springsteen");
    /// # }
    /// ```
    ///
    /// Note that using `$2` instead of `$first` or `$1` instead of `$last`
    /// would produce the same result. To write a literal `$` use `$$`.
    ///
    /// Sometimes the replacement string requires use of curly braces to
    /// delineate a capture group replacement and surrounding literal text.
    /// For example, if we wanted to join two words together with an
    /// underscore:
    ///
    /// ```rust
    /// # extern crate regex; use regex::Regex;
    /// # fn main() {
    /// let re = Regex::new(r"(?P<first>\w+)\s+(?P<second>\w+)").unwrap();
    /// let result = re.replace("deep fried", "${first}_$second");
    /// assert_eq!(result, "deep_fried");
    /// # }
    /// ```
    ///
    /// Without the curly braces, the capture group name `first_` would be
    /// used, and since it doesn't exist, it would be replaced with the empty
    /// string.
    ///
    /// Finally, sometimes you just want to replace a literal string with no
    /// regard for capturing group expansion. This can be done by wrapping a
    /// byte string with `NoExpand`:
    ///
    /// ```rust
    /// # extern crate regex; use regex::Regex;
    /// # fn main() {
    /// use regex::NoExpand;
    ///
    /// let re = Regex::new(r"(?P<last>[^,\s]+),\s+(\S+)").unwrap();
    /// let result = re.replace("Springsteen, Bruce", NoExpand("$2 $last"));
    /// assert_eq!(result, "$2 $last");
    /// # }
    /// ```
    pub fn replace<'t, R: Replacer>(&self, text: &'t str, rep: R) -> Cow<'t, str> {
        self.replacen(text, 1, rep)
    }
    /// Replaces all non-overlapping matches in `text` with the replacement
    /// provided. This is the same as calling `replacen` with `limit` set to
    /// `0`.
    ///
    /// See the documentation for `replace` for details on how to access
    /// capturing group matches in the replacement string.
    pub fn replace_all<'t, R: Replacer>(&self, text: &'t str, rep: R) -> Cow<'t, str> {
        self.replacen(text, 0, rep)
    }
    /// Replaces at most `limit` non-overlapping matches in `text` with the
    /// replacement provided. If `limit` is 0, then all non-overlapping matches
    /// are replaced.
    ///
    /// See the documentation for `replace` for details on how to access
    /// capturing group matches in the replacement string.
    pub fn replacen<'t, R: Replacer>(
        &self,
        text: &'t str,
        limit: usize,
        mut rep: R,
    ) -> Cow<'t, str> {
        if let Some(rep) = rep.no_expansion() {
            let mut it = self.find_iter(text).enumerate().peekable();
            if it.peek().is_none() {
                return Cow::Borrowed(text);
            }
            let mut new = String::with_capacity(text.len());
            let mut last_match = 0;
            for (i, m) in it {
                if limit > 0 && i >= limit {
                    break;
                }
                new.push_str(&text[last_match..m.start()]);
                new.push_str(&rep);
                last_match = m.end();
            }
            new.push_str(&text[last_match..]);
            return Cow::Owned(new);
        }
        let mut it = self.captures_iter(text).enumerate().peekable();
        if it.peek().is_none() {
            return Cow::Borrowed(text);
        }
        let mut new = String::with_capacity(text.len());
        let mut last_match = 0;
        for (i, cap) in it {
            if limit > 0 && i >= limit {
                break;
            }
            let m = cap.get(0).unwrap();
            new.push_str(&text[last_match..m.start()]);
            rep.replace_append(&cap, &mut new);
            last_match = m.end();
        }
        new.push_str(&text[last_match..]);
        Cow::Owned(new)
    }
}
/// Advanced or "lower level" search methods.
impl Regex {
    /// Returns the end location of a match in the text given.
    ///
    /// This method may have the same performance characteristics as
    /// `is_match`, except it provides an end location for a match. In
    /// particular, the location returned *may be shorter* than the proper end
    /// of the leftmost-first match.
    ///
    /// # Example
    ///
    /// Typically, `a+` would match the entire first sequence of `a` in some
    /// text, but `shortest_match` can give up as soon as it sees the first
    /// `a`.
    ///
    /// ```rust
    /// # extern crate regex; use regex::Regex;
    /// # fn main() {
    /// let text = "aaaaa";
    /// let pos = Regex::new(r"a+").unwrap().shortest_match(text);
    /// assert_eq!(pos, Some(1));
    /// # }
    /// ```
    pub fn shortest_match(&self, text: &str) -> Option<usize> {
        self.shortest_match_at(text, 0)
    }
    /// Returns the same as shortest_match, but starts the search at the given
    /// offset.
    ///
    /// The significance of the starting point is that it takes the surrounding
    /// context into consideration. For example, the `\A` anchor can only
    /// match when `start == 0`.
    pub fn shortest_match_at(&self, text: &str, start: usize) -> Option<usize> {
        self.0.searcher_str().shortest_match_at(text, start)
    }
    /// Returns the same as is_match, but starts the search at the given
    /// offset.
    ///
    /// The significance of the starting point is that it takes the surrounding
    /// context into consideration. For example, the `\A` anchor can only
    /// match when `start == 0`.
    pub fn is_match_at(&self, text: &str, start: usize) -> bool {
        self.shortest_match_at(text, start).is_some()
    }
    /// Returns the same as find, but starts the search at the given
    /// offset.
    ///
    /// The significance of the starting point is that it takes the surrounding
    /// context into consideration. For example, the `\A` anchor can only
    /// match when `start == 0`.
    pub fn find_at<'t>(&self, text: &'t str, start: usize) -> Option<Match<'t>> {
        self.0.searcher_str().find_at(text, start).map(|(s, e)| Match::new(text, s, e))
    }
    /// This is like `captures`, but uses
    /// [`CaptureLocations`](struct.CaptureLocations.html)
    /// instead of
    /// [`Captures`](struct.Captures.html) in order to amortize allocations.
    ///
    /// To create a `CaptureLocations` value, use the
    /// `Regex::capture_locations` method.
    ///
    /// This returns the overall match if this was successful, which is always
    /// equivalence to the `0`th capture group.
    pub fn captures_read<'t>(
        &self,
        locs: &mut CaptureLocations,
        text: &'t str,
    ) -> Option<Match<'t>> {
        self.captures_read_at(locs, text, 0)
    }
    /// Returns the same as captures, but starts the search at the given
    /// offset and populates the capture locations given.
    ///
    /// The significance of the starting point is that it takes the surrounding
    /// context into consideration. For example, the `\A` anchor can only
    /// match when `start == 0`.
    pub fn captures_read_at<'t>(
        &self,
        locs: &mut CaptureLocations,
        text: &'t str,
        start: usize,
    ) -> Option<Match<'t>> {
        self.0
            .searcher_str()
            .captures_read_at(&mut locs.0, text, start)
            .map(|(s, e)| Match::new(text, s, e))
    }
    /// An undocumented alias for `captures_read_at`.
    ///
    /// The `regex-capi` crate previously used this routine, so to avoid
    /// breaking that crate, we continue to provide the name as an undocumented
    /// alias.
    #[doc(hidden)]
    pub fn read_captures_at<'t>(
        &self,
        locs: &mut CaptureLocations,
        text: &'t str,
        start: usize,
    ) -> Option<Match<'t>> {
        self.captures_read_at(locs, text, start)
    }
}
/// Auxiliary methods.
impl Regex {
    /// Returns the original string of this regex.
    pub fn as_str(&self) -> &str {
        &self.0.regex_strings()[0]
    }
    /// Returns an iterator over the capture names.
    pub fn capture_names(&self) -> CaptureNames {
        CaptureNames(self.0.capture_names().iter())
    }
    /// Returns the number of captures.
    pub fn captures_len(&self) -> usize {
        self.0.capture_names().len()
    }
    /// Returns an empty set of capture locations that can be reused in
    /// multiple calls to `captures_read` or `captures_read_at`.
    pub fn capture_locations(&self) -> CaptureLocations {
        CaptureLocations(self.0.searcher_str().locations())
    }
    /// An alias for `capture_locations` to preserve backward compatibility.
    ///
    /// The `regex-capi` crate uses this method, so to avoid breaking that
    /// crate, we continue to export it as an undocumented API.
    #[doc(hidden)]
    pub fn locations(&self) -> CaptureLocations {
        CaptureLocations(self.0.searcher_str().locations())
    }
}
/// An iterator over the names of all possible captures.
///
/// `None` indicates an unnamed capture; the first element (capture 0, the
/// whole matched region) is always unnamed.
///
/// `'r` is the lifetime of the compiled regular expression.
#[derive(Clone, Debug)]
pub struct CaptureNames<'r>(::std::slice::Iter<'r, Option<String>>);
impl<'r> Iterator for CaptureNames<'r> {
    type Item = Option<&'r str>;
    fn next(&mut self) -> Option<Option<&'r str>> {
        self.0.next().as_ref().map(|slot| slot.as_ref().map(|name| name.as_ref()))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
    fn count(self) -> usize {
        self.0.count()
    }
}
impl<'r> ExactSizeIterator for CaptureNames<'r> {}
impl<'r> FusedIterator for CaptureNames<'r> {}
/// Yields all substrings delimited by a regular expression match.
///
/// `'r` is the lifetime of the compiled regular expression and `'t` is the
/// lifetime of the string being split.
#[derive(Debug)]
pub struct Split<'r, 't> {
    finder: Matches<'r, 't>,
    last: usize,
}
impl<'r, 't> Iterator for Split<'r, 't> {
    type Item = &'t str;
    fn next(&mut self) -> Option<&'t str> {
        let text = self.finder.0.text();
        match self.finder.next() {
            None => {
                if self.last > text.len() {
                    None
                } else {
                    let s = &text[self.last..];
                    self.last = text.len() + 1;
                    Some(s)
                }
            }
            Some(m) => {
                let matched = &text[self.last..m.start()];
                self.last = m.end();
                Some(matched)
            }
        }
    }
}
impl<'r, 't> FusedIterator for Split<'r, 't> {}
/// Yields at most `N` substrings delimited by a regular expression match.
///
/// The last substring will be whatever remains after splitting.
///
/// `'r` is the lifetime of the compiled regular expression and `'t` is the
/// lifetime of the string being split.
#[derive(Debug)]
pub struct SplitN<'r, 't> {
    splits: Split<'r, 't>,
    n: usize,
}
impl<'r, 't> Iterator for SplitN<'r, 't> {
    type Item = &'t str;
    fn next(&mut self) -> Option<&'t str> {
        if self.n == 0 {
            return None;
        }
        self.n -= 1;
        if self.n > 0 {
            return self.splits.next();
        }
        let text = self.splits.finder.0.text();
        if self.splits.last > text.len() {
            None
        } else {
            Some(&text[self.splits.last..])
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.n))
    }
}
impl<'r, 't> FusedIterator for SplitN<'r, 't> {}
/// CaptureLocations is a low level representation of the raw offsets of each
/// submatch.
///
/// You can think of this as a lower level
/// [`Captures`](struct.Captures.html), where this type does not support
/// named capturing groups directly and it does not borrow the text that these
/// offsets were matched on.
///
/// Primarily, this type is useful when using the lower level `Regex` APIs
/// such as `read_captures`, which permits amortizing the allocation in which
/// capture match locations are stored.
///
/// In order to build a value of this type, you'll need to call the
/// `capture_locations` method on the `Regex` being used to execute the search.
/// The value returned can then be reused in subsequent searches.
#[derive(Clone, Debug)]
pub struct CaptureLocations(re_trait::Locations);
/// A type alias for `CaptureLocations` for backwards compatibility.
///
/// Previously, we exported `CaptureLocations` as `Locations` in an
/// undocumented API. To prevent breaking that code (e.g., in `regex-capi`),
/// we continue re-exporting the same undocumented API.
#[doc(hidden)]
pub type Locations = CaptureLocations;
impl CaptureLocations {
    /// Returns the start and end positions of the Nth capture group. Returns
    /// `None` if `i` is not a valid capture group or if the capture group did
    /// not match anything. The positions returned are *always* byte indices
    /// with respect to the original string matched.
    #[inline]
    pub fn get(&self, i: usize) -> Option<(usize, usize)> {
        self.0.pos(i)
    }
    /// Returns the total number of capturing groups.
    ///
    /// This is always at least `1` since every regex has at least `1`
    /// capturing group that corresponds to the entire match.
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }
    /// An alias for the `get` method for backwards compatibility.
    ///
    /// Previously, we exported `get` as `pos` in an undocumented API. To
    /// prevent breaking that code (e.g., in `regex-capi`), we continue
    /// re-exporting the same undocumented API.
    #[doc(hidden)]
    #[inline]
    pub fn pos(&self, i: usize) -> Option<(usize, usize)> {
        self.get(i)
    }
}
/// Captures represents a group of captured strings for a single match.
///
/// The 0th capture always corresponds to the entire match. Each subsequent
/// index corresponds to the next capture group in the regex. If a capture
/// group is named, then the matched string is *also* available via the `name`
/// method. (Note that the 0th capture is always unnamed and so must be
/// accessed with the `get` method.)
///
/// Positions returned from a capture group are always byte indices.
///
/// `'t` is the lifetime of the matched text.
pub struct Captures<'t> {
    text: &'t str,
    locs: re_trait::Locations,
    named_groups: Arc<HashMap<String, usize>>,
}
impl<'t> Captures<'t> {
    /// Returns the match associated with the capture group at index `i`. If
    /// `i` does not correspond to a capture group, or if the capture group
    /// did not participate in the match, then `None` is returned.
    ///
    /// # Examples
    ///
    /// Get the text of the match with a default of an empty string if this
    /// group didn't participate in the match:
    ///
    /// ```rust
    /// # use regex::Regex;
    /// let re = Regex::new(r"[a-z]+(?:([0-9]+)|([A-Z]+))").unwrap();
    /// let caps = re.captures("abc123").unwrap();
    ///
    /// let text1 = caps.get(1).map_or("", |m| m.as_str());
    /// let text2 = caps.get(2).map_or("", |m| m.as_str());
    /// assert_eq!(text1, "123");
    /// assert_eq!(text2, "");
    /// ```
    pub fn get(&self, i: usize) -> Option<Match<'t>> {
        self.locs.pos(i).map(|(s, e)| Match::new(self.text, s, e))
    }
    /// Returns the match for the capture group named `name`. If `name` isn't a
    /// valid capture group or didn't match anything, then `None` is returned.
    pub fn name(&self, name: &str) -> Option<Match<'t>> {
        self.named_groups.get(name).and_then(|&i| self.get(i))
    }
    /// An iterator that yields all capturing matches in the order in which
    /// they appear in the regex. If a particular capture group didn't
    /// participate in the match, then `None` is yielded for that capture.
    ///
    /// The first match always corresponds to the overall match of the regex.
    pub fn iter<'c>(&'c self) -> SubCaptureMatches<'c, 't> {
        SubCaptureMatches {
            caps: self,
            it: self.locs.iter(),
        }
    }
    /// Expands all instances of `$name` in `replacement` to the corresponding
    /// capture group `name`, and writes them to the `dst` buffer given.
    ///
    /// `name` may be an integer corresponding to the index of the capture
    /// group (counted by order of opening parenthesis where `0` is the
    /// entire match) or it can be a name (consisting of letters, digits or
    /// underscores) corresponding to a named capture group.
    ///
    /// If `name` isn't a valid capture group (whether the name doesn't exist
    /// or isn't a valid index), then it is replaced with the empty string.
    ///
    /// The longest possible name consisting of the characters `[_0-9A-Za-z]`
    /// is used. e.g., `$1a` looks up the capture group named `1a` and not the
    /// capture group at index `1`. To exert more precise control over the
    /// name, or to refer to a capture group name that uses characters outside
    /// of `[_0-9A-Za-z]`, use braces, e.g., `${1}a` or `${foo[bar].baz}`. When
    /// using braces, any sequence of characters is permitted. If the sequence
    /// does not refer to a capture group name in the corresponding regex, then
    /// it is replaced with an empty string.
    ///
    /// To write a literal `$` use `$$`.
    pub fn expand(&self, replacement: &str, dst: &mut String) {
        expand_str(self, replacement, dst)
    }
    /// Returns the number of captured groups.
    ///
    /// This is always at least `1`, since every regex has at least one capture
    /// group that corresponds to the full match.
    #[inline]
    pub fn len(&self) -> usize {
        self.locs.len()
    }
}
impl<'t> fmt::Debug for Captures<'t> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("Captures").field(&CapturesDebug(self)).finish()
    }
}
struct CapturesDebug<'c, 't: 'c>(&'c Captures<'t>);
impl<'c, 't> fmt::Debug for CapturesDebug<'c, 't> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let slot_to_name: HashMap<&usize, &String> = self
            .0
            .named_groups
            .iter()
            .map(|(a, b)| (b, a))
            .collect();
        let mut map = f.debug_map();
        for (slot, m) in self.0.locs.iter().enumerate() {
            let m = m.map(|(s, e)| &self.0.text[s..e]);
            if let Some(name) = slot_to_name.get(&slot) {
                map.entry(&name, &m);
            } else {
                map.entry(&slot, &m);
            }
        }
        map.finish()
    }
}
/// Get a group by index.
///
/// `'t` is the lifetime of the matched text.
///
/// The text can't outlive the `Captures` object if this method is
/// used, because of how `Index` is defined (normally `a[i]` is part
/// of `a` and can't outlive it); to do that, use `get()` instead.
///
/// # Panics
///
/// If there is no group at the given index.
impl<'t> Index<usize> for Captures<'t> {
    type Output = str;
    fn index(&self, i: usize) -> &str {
        self.get(i)
            .map(|m| m.as_str())
            .unwrap_or_else(|| panic!("no group at index '{}'", i))
    }
}
/// Get a group by name.
///
/// `'t` is the lifetime of the matched text and `'i` is the lifetime
/// of the group name (the index).
///
/// The text can't outlive the `Captures` object if this method is
/// used, because of how `Index` is defined (normally `a[i]` is part
/// of `a` and can't outlive it); to do that, use `name` instead.
///
/// # Panics
///
/// If there is no group named by the given value.
impl<'t, 'i> Index<&'i str> for Captures<'t> {
    type Output = str;
    fn index<'a>(&'a self, name: &'i str) -> &'a str {
        self.name(name)
            .map(|m| m.as_str())
            .unwrap_or_else(|| panic!("no group named '{}'", name))
    }
}
/// An iterator that yields all capturing matches in the order in which they
/// appear in the regex.
///
/// If a particular capture group didn't participate in the match, then `None`
/// is yielded for that capture. The first match always corresponds to the
/// overall match of the regex.
///
/// The lifetime `'c` corresponds to the lifetime of the `Captures` value, and
/// the lifetime `'t` corresponds to the originally matched text.
#[derive(Clone, Debug)]
pub struct SubCaptureMatches<'c, 't: 'c> {
    caps: &'c Captures<'t>,
    it: SubCapturesPosIter<'c>,
}
impl<'c, 't> Iterator for SubCaptureMatches<'c, 't> {
    type Item = Option<Match<'t>>;
    fn next(&mut self) -> Option<Option<Match<'t>>> {
        self.it.next().map(|cap| cap.map(|(s, e)| Match::new(self.caps.text, s, e)))
    }
}
impl<'c, 't> FusedIterator for SubCaptureMatches<'c, 't> {}
/// An iterator that yields all non-overlapping capture groups matching a
/// particular regular expression.
///
/// The iterator stops when no more matches can be found.
///
/// `'r` is the lifetime of the compiled regular expression and `'t` is the
/// lifetime of the matched string.
#[derive(Debug)]
pub struct CaptureMatches<'r, 't>(re_trait::CaptureMatches<'t, ExecNoSyncStr<'r>>);
impl<'r, 't> Iterator for CaptureMatches<'r, 't> {
    type Item = Captures<'t>;
    fn next(&mut self) -> Option<Captures<'t>> {
        self.0
            .next()
            .map(|locs| Captures {
                text: self.0.text(),
                locs: locs,
                named_groups: self.0.regex().capture_name_idx().clone(),
            })
    }
}
impl<'r, 't> FusedIterator for CaptureMatches<'r, 't> {}
/// An iterator over all non-overlapping matches for a particular string.
///
/// The iterator yields a `Match` value. The iterator stops when no more
/// matches can be found.
///
/// `'r` is the lifetime of the compiled regular expression and `'t` is the
/// lifetime of the matched string.
#[derive(Debug)]
pub struct Matches<'r, 't>(re_trait::Matches<'t, ExecNoSyncStr<'r>>);
impl<'r, 't> Iterator for Matches<'r, 't> {
    type Item = Match<'t>;
    fn next(&mut self) -> Option<Match<'t>> {
        let text = self.0.text();
        self.0.next().map(|(s, e)| Match::new(text, s, e))
    }
}
impl<'r, 't> FusedIterator for Matches<'r, 't> {}
/// Replacer describes types that can be used to replace matches in a string.
///
/// In general, users of this crate shouldn't need to implement this trait,
/// since implementations are already provided for `&str` and
/// `FnMut(&Captures) -> String` (or any `FnMut(&Captures) -> T`
/// where `T: AsRef<str>`), which covers most use cases.
pub trait Replacer {
    /// Appends text to `dst` to replace the current match.
    ///
    /// The current match is represented by `caps`, which is guaranteed to
    /// have a match at capture group `0`.
    ///
    /// For example, a no-op replacement would be
    /// `dst.push_str(caps.get(0).unwrap().as_str())`.
    fn replace_append(&mut self, caps: &Captures, dst: &mut String);
    /// Return a fixed unchanging replacement string.
    ///
    /// When doing replacements, if access to `Captures` is not needed (e.g.,
    /// the replacement byte string does not need `$` expansion), then it can
    /// be beneficial to avoid finding sub-captures.
    ///
    /// In general, this is called once for every call to `replacen`.
    fn no_expansion<'r>(&'r mut self) -> Option<Cow<'r, str>> {
        None
    }
    /// Return a `Replacer` that borrows and wraps this `Replacer`.
    ///
    /// This is useful when you want to take a generic `Replacer` (which might
    /// not be cloneable) and use it without consuming it, so it can be used
    /// more than once.
    ///
    /// # Example
    ///
    /// ```
    /// use regex::{Regex, Replacer};
    ///
    /// fn replace_all_twice<R: Replacer>(
    ///     re: Regex,
    ///     src: &str,
    ///     mut rep: R,
    /// ) -> String {
    ///     let dst = re.replace_all(src, rep.by_ref());
    ///     let dst = re.replace_all(&dst, rep.by_ref());
    ///     dst.into_owned()
    /// }
    /// ```
    fn by_ref<'r>(&'r mut self) -> ReplacerRef<'r, Self> {
        ReplacerRef(self)
    }
}
/// By-reference adaptor for a `Replacer`
///
/// Returned by [`Replacer::by_ref`](trait.Replacer.html#method.by_ref).
#[derive(Debug)]
pub struct ReplacerRef<'a, R: ?Sized + 'a>(&'a mut R);
impl<'a, R: Replacer + ?Sized + 'a> Replacer for ReplacerRef<'a, R> {
    fn replace_append(&mut self, caps: &Captures, dst: &mut String) {
        self.0.replace_append(caps, dst)
    }
    fn no_expansion(&mut self) -> Option<Cow<str>> {
        self.0.no_expansion()
    }
}
impl<'a> Replacer for &'a str {
    fn replace_append(&mut self, caps: &Captures, dst: &mut String) {
        caps.expand(*self, dst);
    }
    fn no_expansion(&mut self) -> Option<Cow<str>> {
        match find_byte(b'$', self.as_bytes()) {
            Some(_) => None,
            None => Some(Cow::Borrowed(*self)),
        }
    }
}
impl<F, T> Replacer for F
where
    F: FnMut(&Captures) -> T,
    T: AsRef<str>,
{
    fn replace_append(&mut self, caps: &Captures, dst: &mut String) {
        dst.push_str((*self)(caps).as_ref());
    }
}
/// `NoExpand` indicates literal string replacement.
///
/// It can be used with `replace` and `replace_all` to do a literal string
/// replacement without expanding `$name` to their corresponding capture
/// groups. This can be both convenient (to avoid escaping `$`, for example)
/// and performant (since capture groups don't need to be found).
///
/// `'t` is the lifetime of the literal text.
#[derive(Clone, Debug)]
pub struct NoExpand<'t>(pub &'t str);
impl<'t> Replacer for NoExpand<'t> {
    fn replace_append(&mut self, _: &Captures, dst: &mut String) {
        dst.push_str(self.0);
    }
    fn no_expansion(&mut self) -> Option<Cow<str>> {
        Some(Cow::Borrowed(self.0))
    }
}
#[cfg(test)]
mod tests_llm_16_25 {
    use crate::re_unicode::Replacer;
    use std::borrow::Cow;
    #[test]
    fn test_no_expansion() {
        let _rug_st_tests_llm_16_25_rrrruuuugggg_test_no_expansion = 0;
        let rug_fuzz_0 = "abc";
        let mut replacer = rug_fuzz_0;
        let result = replacer.no_expansion();
        debug_assert_eq!(result.is_none(), true);
        debug_assert_eq!(result.unwrap(), Cow::Borrowed("abc"));
        let _rug_ed_tests_llm_16_25_rrrruuuugggg_test_no_expansion = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_166 {
    use std::borrow::Cow;
    use crate::re_unicode::{NoExpand, Replacer};
    #[test]
    fn test_no_expansion() {
        let _rug_st_tests_llm_16_166_rrrruuuugggg_test_no_expansion = 0;
        let rug_fuzz_0 = "literal text";
        let mut replacer = NoExpand(rug_fuzz_0);
        let mut dst = String::new();
        let result = replacer.no_expansion();
        debug_assert_eq!(result, Some(Cow::Borrowed("literal text")));
        let _rug_ed_tests_llm_16_166_rrrruuuugggg_test_no_expansion = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_171 {
    use crate::{Regex, Error};
    #[test]
    fn test_from_str() {
        let _rug_st_tests_llm_16_171_rrrruuuugggg_test_from_str = 0;
        let rug_fuzz_0 = "";
        let result: Result<Regex, Error> = <Regex as std::str::FromStr>::from_str(
            rug_fuzz_0,
        );
        debug_assert!(result.is_err());
        let _rug_ed_tests_llm_16_171_rrrruuuugggg_test_from_str = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_701_llm_16_700 {
    use std::ops::Range;
    use crate::re_unicode::Match;
    #[test]
    fn test_from() {
        let _rug_st_tests_llm_16_701_llm_16_700_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = "abcd";
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 3;
        let match_obj = Match::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        let range_obj: Range<usize> = match_obj.range().into();
        debug_assert_eq!(range_obj.start, 1);
        debug_assert_eq!(range_obj.end, 3);
        let _rug_ed_tests_llm_16_701_llm_16_700_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_710 {
    use super::*;
    use crate::*;
    use crate::Regex;
    #[test]
    fn test_get() {
        let _rug_st_tests_llm_16_710_rrrruuuugggg_test_get = 0;
        let rug_fuzz_0 = r"[a-z]+(?:([0-9]+)|([A-Z]+))";
        let rug_fuzz_1 = "abc123";
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = "";
        let rug_fuzz_4 = 2;
        let rug_fuzz_5 = "";
        let re = Regex::new(rug_fuzz_0).unwrap();
        let caps = re.captures(rug_fuzz_1).unwrap();
        let text1 = caps.get(rug_fuzz_2).map_or(rug_fuzz_3, |m| m.as_str());
        let text2 = caps.get(rug_fuzz_4).map_or(rug_fuzz_5, |m| m.as_str());
        debug_assert_eq!(text1, "123");
        debug_assert_eq!(text2, "");
        let _rug_ed_tests_llm_16_710_rrrruuuugggg_test_get = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_712_llm_16_711 {
    use std::collections::HashMap;
    use std::sync::Arc;
    use crate::re_trait;
    use crate::Match;
    #[test]
    fn test_iter() {
        let _rug_st_tests_llm_16_712_llm_16_711_rrrruuuugggg_test_iter = 0;
        let rug_fuzz_0 = "abc123";
        let rug_fuzz_1 = r"[a-z]+(?:([0-9]+)|([A-Z]+))";
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 6;
        let text = rug_fuzz_0;
        let re = crate::Regex::new(rug_fuzz_1).unwrap();
        let caps = re.captures(text).unwrap();
        let iter_result: Vec<Option<Match>> = caps.iter().collect();
        let expected: Vec<Option<Match>> = vec![
            Some(Match::new(text, rug_fuzz_2, rug_fuzz_3)), Some(Match::new(text, 3, 6)),
            None
        ];
        debug_assert_eq!(iter_result, expected);
        let _rug_ed_tests_llm_16_712_llm_16_711_rrrruuuugggg_test_iter = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_717 {
    use super::*;
    use crate::*;
    #[test]
    fn test_as_str() {
        let _rug_st_tests_llm_16_717_rrrruuuugggg_test_as_str = 0;
        let rug_fuzz_0 = "Hello, world!";
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 5;
        let text = rug_fuzz_0;
        let match_text = Match::new(text, rug_fuzz_1, rug_fuzz_2);
        debug_assert_eq!(match_text.as_str(), "Hello");
        let _rug_ed_tests_llm_16_717_rrrruuuugggg_test_as_str = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_718 {
    use std::ops::Range;
    use crate::re_unicode::Match;
    #[test]
    fn test_end() {
        let _rug_st_tests_llm_16_718_rrrruuuugggg_test_end = 0;
        let rug_fuzz_0 = "Hello, world!";
        let rug_fuzz_1 = 7;
        let rug_fuzz_2 = 12;
        let haystack = rug_fuzz_0;
        let start = rug_fuzz_1;
        let end = rug_fuzz_2;
        let matched = Match::new(haystack, start, end);
        debug_assert_eq!(matched.end(), end);
        let _rug_ed_tests_llm_16_718_rrrruuuugggg_test_end = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_719 {
    use std::ops::Range;
    use crate::re_unicode::Match;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_719_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = "Lorem ipsum dolor sit amet";
        let rug_fuzz_1 = 6;
        let rug_fuzz_2 = 11;
        let haystack = rug_fuzz_0;
        let start = rug_fuzz_1;
        let end = rug_fuzz_2;
        let m = Match::new(haystack, start, end);
        debug_assert_eq!(m.text, haystack);
        debug_assert_eq!(m.start, start);
        debug_assert_eq!(m.end, end);
        let _rug_ed_tests_llm_16_719_rrrruuuugggg_test_new = 0;
    }
    #[test]
    fn test_start() {
        let _rug_st_tests_llm_16_719_rrrruuuugggg_test_start = 0;
        let rug_fuzz_0 = "Lorem ipsum dolor sit amet";
        let rug_fuzz_1 = 6;
        let rug_fuzz_2 = 11;
        let haystack = rug_fuzz_0;
        let start = rug_fuzz_1;
        let end = rug_fuzz_2;
        let m = Match::new(haystack, start, end);
        debug_assert_eq!(m.start(), start);
        let _rug_ed_tests_llm_16_719_rrrruuuugggg_test_start = 0;
    }
    #[test]
    fn test_end() {
        let _rug_st_tests_llm_16_719_rrrruuuugggg_test_end = 0;
        let rug_fuzz_0 = "Lorem ipsum dolor sit amet";
        let rug_fuzz_1 = 6;
        let rug_fuzz_2 = 11;
        let haystack = rug_fuzz_0;
        let start = rug_fuzz_1;
        let end = rug_fuzz_2;
        let m = Match::new(haystack, start, end);
        debug_assert_eq!(m.end(), end);
        let _rug_ed_tests_llm_16_719_rrrruuuugggg_test_end = 0;
    }
    #[test]
    fn test_range() {
        let _rug_st_tests_llm_16_719_rrrruuuugggg_test_range = 0;
        let rug_fuzz_0 = "Lorem ipsum dolor sit amet";
        let rug_fuzz_1 = 6;
        let rug_fuzz_2 = 11;
        let haystack = rug_fuzz_0;
        let start = rug_fuzz_1;
        let end = rug_fuzz_2;
        let m = Match::new(haystack, start, end);
        let expected_range = start..end;
        debug_assert_eq!(m.range(), expected_range);
        let _rug_ed_tests_llm_16_719_rrrruuuugggg_test_range = 0;
    }
    #[test]
    fn test_as_str() {
        let _rug_st_tests_llm_16_719_rrrruuuugggg_test_as_str = 0;
        let rug_fuzz_0 = "Lorem ipsum dolor sit amet";
        let rug_fuzz_1 = 6;
        let rug_fuzz_2 = 11;
        let haystack = rug_fuzz_0;
        let start = rug_fuzz_1;
        let end = rug_fuzz_2;
        let m = Match::new(haystack, start, end);
        let expected_str = &haystack[start..end];
        debug_assert_eq!(m.as_str(), expected_str);
        let _rug_ed_tests_llm_16_719_rrrruuuugggg_test_as_str = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_720 {
    use super::*;
    use crate::*;
    use std::ops::Range;
    #[test]
    fn test_range() {
        let _rug_st_tests_llm_16_720_rrrruuuugggg_test_range = 0;
        let rug_fuzz_0 = "hello world";
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 5;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 5;
        let match_obj = re_unicode::Match::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        let expected_range: Range<usize> = rug_fuzz_3..rug_fuzz_4;
        debug_assert_eq!(match_obj.range(), expected_range);
        let _rug_ed_tests_llm_16_720_rrrruuuugggg_test_range = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_721 {
    use super::*;
    use crate::*;
    #[test]
    fn test_start() {
        let _rug_st_tests_llm_16_721_rrrruuuugggg_test_start = 0;
        let rug_fuzz_0 = "Hello, world!";
        let rug_fuzz_1 = 7;
        let rug_fuzz_2 = 12;
        let haystack = rug_fuzz_0;
        let start = rug_fuzz_1;
        let end = rug_fuzz_2;
        let match_obj = re_unicode::Match::new(haystack, start, end);
        debug_assert_eq!(match_obj.start(), start);
        let _rug_ed_tests_llm_16_721_rrrruuuugggg_test_start = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_722 {
    use super::*;
    use crate::*;
    #[test]
    fn test_as_str() {
        let _rug_st_tests_llm_16_722_rrrruuuugggg_test_as_str = 0;
        let rug_fuzz_0 = r"\b\w{3}\b";
        let regex = Regex::new(rug_fuzz_0).unwrap();
        debug_assert_eq!(regex.as_str(), r"\b\w{3}\b");
        let _rug_ed_tests_llm_16_722_rrrruuuugggg_test_as_str = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_724 {
    use super::*;
    use crate::*;
    #[test]
    fn test_capture_names() {
        let _rug_st_tests_llm_16_724_rrrruuuugggg_test_capture_names = 0;
        let rug_fuzz_0 = r"(\d+):(\d+)";
        let regex = Regex::new(rug_fuzz_0).unwrap();
        let capture_names = regex.capture_names();
        let expected = vec![None, None];
        let actual: Vec<Option<&str>> = capture_names.collect();
        debug_assert_eq!(actual, expected);
        let _rug_ed_tests_llm_16_724_rrrruuuugggg_test_capture_names = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_725 {
    use crate::re_unicode::*;
    #[test]
    fn test_captures() {
        let _rug_st_tests_llm_16_725_rrrruuuugggg_test_captures = 0;
        let rug_fuzz_0 = r"'([^']+)'\s+\((\d{4})\)";
        let rug_fuzz_1 = "Not my favorite movie: 'Citizen Kane' (1941).";
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 2;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 1;
        let rug_fuzz_6 = 2;
        let rug_fuzz_7 = 0;
        let rug_fuzz_8 = r"'(?P<title>[^']+)'\s+\((?P<year>\d{4})\)";
        let rug_fuzz_9 = "Not my favorite movie: 'Citizen Kane' (1941).";
        let rug_fuzz_10 = "title";
        let rug_fuzz_11 = "year";
        let rug_fuzz_12 = 0;
        let rug_fuzz_13 = "title";
        let rug_fuzz_14 = "year";
        let rug_fuzz_15 = 0;
        let regex_str = rug_fuzz_0;
        let regex = Regex::new(regex_str).unwrap();
        let text = rug_fuzz_1;
        let caps = regex.captures(text).unwrap();
        debug_assert_eq!(caps.get(rug_fuzz_2).unwrap().as_str(), "Citizen Kane");
        debug_assert_eq!(caps.get(rug_fuzz_3).unwrap().as_str(), "1941");
        debug_assert_eq!(
            caps.get(rug_fuzz_4).unwrap().as_str(), "'Citizen Kane' (1941)"
        );
        debug_assert_eq!(& caps[rug_fuzz_5], "Citizen Kane");
        debug_assert_eq!(& caps[rug_fuzz_6], "1941");
        debug_assert_eq!(& caps[rug_fuzz_7], "'Citizen Kane' (1941)");
        let regex_str = rug_fuzz_8;
        let regex = Regex::new(regex_str).unwrap();
        let text = rug_fuzz_9;
        let caps = regex.captures(text).unwrap();
        debug_assert_eq!(caps.name(rug_fuzz_10).unwrap().as_str(), "Citizen Kane");
        debug_assert_eq!(caps.name(rug_fuzz_11).unwrap().as_str(), "1941");
        debug_assert_eq!(
            caps.get(rug_fuzz_12).unwrap().as_str(), "'Citizen Kane' (1941)"
        );
        debug_assert_eq!(& caps[rug_fuzz_13], "Citizen Kane");
        debug_assert_eq!(& caps[rug_fuzz_14], "1941");
        debug_assert_eq!(& caps[rug_fuzz_15], "'Citizen Kane' (1941)");
        let _rug_ed_tests_llm_16_725_rrrruuuugggg_test_captures = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_726 {
    use super::*;
    use crate::*;
    use crate::Regex;
    #[test]
    fn test_captures_len() {
        let _rug_st_tests_llm_16_726_rrrruuuugggg_test_captures_len = 0;
        let rug_fuzz_0 = r"[0-9]{3}-[0-9]{3}-[0-9]{4}";
        let rug_fuzz_1 = r"(\w+),(\w+)";
        let regex = Regex::new(rug_fuzz_0).unwrap();
        debug_assert_eq!(regex.captures_len(), 0);
        let regex = Regex::new(rug_fuzz_1).unwrap();
        debug_assert_eq!(regex.captures_len(), 3);
        let _rug_ed_tests_llm_16_726_rrrruuuugggg_test_captures_len = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_727 {
    use super::*;
    use crate::*;
    use std::fmt::Debug;
    fn assert_send<T: Send>() {
        let _rug_st_tests_llm_16_727_rrrruuuugggg_assert_send = 0;
        let _rug_ed_tests_llm_16_727_rrrruuuugggg_assert_send = 0;
    }
    fn assert_clone<T: Clone>() {
        let _rug_st_tests_llm_16_727_rrrruuuugggg_assert_clone = 0;
        let _rug_ed_tests_llm_16_727_rrrruuuugggg_assert_clone = 0;
    }
    fn assert_debug<T: Debug>() {
        let _rug_st_tests_llm_16_727_rrrruuuugggg_assert_debug = 0;
        let _rug_ed_tests_llm_16_727_rrrruuuugggg_assert_debug = 0;
    }
    #[test]
    fn test_captures_read() {
        let _rug_st_tests_llm_16_727_rrrruuuugggg_test_captures_read = 0;
        let rug_fuzz_0 = r"\d+";
        let rug_fuzz_1 = "hello 123 world 456";
        assert_send::<Regex>();
        assert_clone::<Regex>();
        assert_debug::<Regex>();
        let regex = Regex::new(rug_fuzz_0).unwrap();
        let mut locs = regex.capture_locations();
        let text = rug_fuzz_1;
        let result = regex.captures_read(&mut locs, text);
        debug_assert_eq!(result, Some(Match::new(text, 6, 9)));
        let _rug_ed_tests_llm_16_727_rrrruuuugggg_test_captures_read = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_729 {
    use crate::Regex;
    #[test]
    fn test_find() {
        let _rug_st_tests_llm_16_729_rrrruuuugggg_test_find = 0;
        let rug_fuzz_0 = "I categorically deny having triskaidekaphobia.";
        let rug_fuzz_1 = r"\b\w{13}\b";
        let text = rug_fuzz_0;
        let mat = Regex::new(rug_fuzz_1).unwrap().find(text).unwrap();
        debug_assert_eq!(mat.start(), 2);
        debug_assert_eq!(mat.end(), 15);
        let _rug_ed_tests_llm_16_729_rrrruuuugggg_test_find = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_731 {
    use super::*;
    use crate::*;
    #[test]
    fn is_match_test() {
        let _rug_st_tests_llm_16_731_rrrruuuugggg_is_match_test = 0;
        let rug_fuzz_0 = r"\b\w{13}\b";
        let rug_fuzz_1 = "I categorically deny having triskaidekaphobia.";
        let regex = Regex::new(rug_fuzz_0).unwrap();
        let text = rug_fuzz_1;
        debug_assert!(regex.is_match(text));
        let _rug_ed_tests_llm_16_731_rrrruuuugggg_is_match_test = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_732 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_match_at() {
        let _rug_st_tests_llm_16_732_rrrruuuugggg_test_is_match_at = 0;
        let rug_fuzz_0 = r"\d+";
        let rug_fuzz_1 = "1234";
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = "abc";
        let rug_fuzz_4 = 0;
        let regex = Regex::new(rug_fuzz_0).unwrap();
        debug_assert!(regex.is_match_at(rug_fuzz_1, rug_fuzz_2));
        debug_assert!(! regex.is_match_at(rug_fuzz_3, rug_fuzz_4));
        let _rug_ed_tests_llm_16_732_rrrruuuugggg_test_is_match_at = 0;
    }
    #[test]
    fn test_is_match_at_start() {
        let _rug_st_tests_llm_16_732_rrrruuuugggg_test_is_match_at_start = 0;
        let rug_fuzz_0 = r"\d+";
        let rug_fuzz_1 = "1234";
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = "1234";
        let rug_fuzz_4 = 1;
        let regex = Regex::new(rug_fuzz_0).unwrap();
        debug_assert!(regex.is_match_at(rug_fuzz_1, rug_fuzz_2));
        debug_assert!(! regex.is_match_at(rug_fuzz_3, rug_fuzz_4));
        let _rug_ed_tests_llm_16_732_rrrruuuugggg_test_is_match_at_start = 0;
    }
    #[test]
    fn test_is_match_at_end() {
        let _rug_st_tests_llm_16_732_rrrruuuugggg_test_is_match_at_end = 0;
        let rug_fuzz_0 = r"\d+";
        let rug_fuzz_1 = "1234";
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = "1234";
        let rug_fuzz_4 = 4;
        let regex = Regex::new(rug_fuzz_0).unwrap();
        debug_assert!(regex.is_match_at(rug_fuzz_1, rug_fuzz_2));
        debug_assert!(! regex.is_match_at(rug_fuzz_3, rug_fuzz_4));
        let _rug_ed_tests_llm_16_732_rrrruuuugggg_test_is_match_at_end = 0;
    }
    #[test]
    fn test_is_match_at_middle() {
        let _rug_st_tests_llm_16_732_rrrruuuugggg_test_is_match_at_middle = 0;
        let rug_fuzz_0 = r"\d+";
        let rug_fuzz_1 = "abcd1234efgh";
        let rug_fuzz_2 = 4;
        let rug_fuzz_3 = "abcd1234efgh";
        let rug_fuzz_4 = 8;
        let regex = Regex::new(rug_fuzz_0).unwrap();
        debug_assert!(regex.is_match_at(rug_fuzz_1, rug_fuzz_2));
        debug_assert!(! regex.is_match_at(rug_fuzz_3, rug_fuzz_4));
        let _rug_ed_tests_llm_16_732_rrrruuuugggg_test_is_match_at_middle = 0;
    }
    #[test]
    fn test_is_match_at_no_match() {
        let _rug_st_tests_llm_16_732_rrrruuuugggg_test_is_match_at_no_match = 0;
        let rug_fuzz_0 = r"\d+";
        let rug_fuzz_1 = "abcd";
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = "abcd";
        let rug_fuzz_4 = 4;
        let regex = Regex::new(rug_fuzz_0).unwrap();
        debug_assert!(! regex.is_match_at(rug_fuzz_1, rug_fuzz_2));
        debug_assert!(! regex.is_match_at(rug_fuzz_3, rug_fuzz_4));
        let _rug_ed_tests_llm_16_732_rrrruuuugggg_test_is_match_at_no_match = 0;
    }
    #[test]
    fn test_is_match_at_empty() {
        let _rug_st_tests_llm_16_732_rrrruuuugggg_test_is_match_at_empty = 0;
        let rug_fuzz_0 = r"\d+";
        let rug_fuzz_1 = "";
        let rug_fuzz_2 = 0;
        let regex = Regex::new(rug_fuzz_0).unwrap();
        debug_assert!(! regex.is_match_at(rug_fuzz_1, rug_fuzz_2));
        let _rug_ed_tests_llm_16_732_rrrruuuugggg_test_is_match_at_empty = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_733 {
    use super::*;
    use crate::*;
    #[test]
    fn test_regex_locations() {
        let _rug_st_tests_llm_16_733_rrrruuuugggg_test_regex_locations = 0;
        let rug_fuzz_0 = r"\d+";
        let rug_fuzz_1 = "abc123def456ghi";
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let regex = Regex::new(rug_fuzz_0).unwrap();
        let locations = regex.locations();
        let text = rug_fuzz_1;
        let match_start = locations.get(rug_fuzz_2).unwrap().0;
        let match_end = locations.get(rug_fuzz_3).unwrap().1;
        debug_assert_eq!((match_start, match_end), (3, 6));
        let _rug_ed_tests_llm_16_733_rrrruuuugggg_test_regex_locations = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_734 {
    use crate::{Regex, Error};
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_734_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = "abc";
        let rug_fuzz_1 = "ab(c";
        let regex = Regex::new(rug_fuzz_0);
        debug_assert!(regex.is_ok());
        let regex_error = Regex::new(rug_fuzz_1);
        debug_assert!(regex_error.is_err());
        let regex_error_msg = regex_error.unwrap_err().to_string();
        debug_assert_eq!(regex_error_msg, "missing )");
        let _rug_ed_tests_llm_16_734_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_736 {
    use super::*;
    use crate::*;
    use crate::{Captures, NoExpand};
    #[test]
    fn test_replace() {
        let _rug_st_tests_llm_16_736_rrrruuuugggg_test_replace = 0;
        let rug_fuzz_0 = "[^01]+";
        let rug_fuzz_1 = "1078910";
        let rug_fuzz_2 = "";
        let re = Regex::new(rug_fuzz_0).unwrap();
        debug_assert_eq!(re.replace(rug_fuzz_1, rug_fuzz_2), "1010");
        let _rug_ed_tests_llm_16_736_rrrruuuugggg_test_replace = 0;
    }
    #[test]
    fn test_replace_closure() {
        let _rug_st_tests_llm_16_736_rrrruuuugggg_test_replace_closure = 0;
        let rug_fuzz_0 = r"([^,\s]+),\s+(\S+)";
        let rug_fuzz_1 = "Springsteen, Bruce";
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 1;
        let re = Regex::new(rug_fuzz_0).unwrap();
        let result = re
            .replace(
                rug_fuzz_1,
                |caps: &Captures| {
                    format!("{} {}", & caps[rug_fuzz_2], & caps[rug_fuzz_3])
                },
            );
        debug_assert_eq!(result, "Bruce Springsteen");
        let _rug_ed_tests_llm_16_736_rrrruuuugggg_test_replace_closure = 0;
    }
    #[test]
    fn test_replace_named_capture() {
        let _rug_st_tests_llm_16_736_rrrruuuugggg_test_replace_named_capture = 0;
        let rug_fuzz_0 = r"(?P<last>[^,\s]+),\s+(?P<first>\S+)";
        let rug_fuzz_1 = "Springsteen, Bruce";
        let rug_fuzz_2 = "$first $last";
        let re = Regex::new(rug_fuzz_0).unwrap();
        let result = re.replace(rug_fuzz_1, rug_fuzz_2);
        debug_assert_eq!(result, "Bruce Springsteen");
        let _rug_ed_tests_llm_16_736_rrrruuuugggg_test_replace_named_capture = 0;
    }
    #[test]
    fn test_replace_literal_dollar() {
        let _rug_st_tests_llm_16_736_rrrruuuugggg_test_replace_literal_dollar = 0;
        let rug_fuzz_0 = r"(?P<last>[^,\s]+),\s+(\S+)";
        let rug_fuzz_1 = "Springsteen, Bruce";
        let rug_fuzz_2 = "$2 $last";
        let re = Regex::new(rug_fuzz_0).unwrap();
        let result = re.replace(rug_fuzz_1, NoExpand(rug_fuzz_2));
        debug_assert_eq!(result, "$2 $last");
        let _rug_ed_tests_llm_16_736_rrrruuuugggg_test_replace_literal_dollar = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_738 {
    use super::*;
    use crate::*;
    #[test]
    fn test_replacen() {
        let _rug_st_tests_llm_16_738_rrrruuuugggg_test_replacen = 0;
        let rug_fuzz_0 = r"a+";
        let rug_fuzz_1 = "aaaaa";
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = "";
        let rug_fuzz_4 = r"[^01]+";
        let rug_fuzz_5 = "1078910";
        let rug_fuzz_6 = 1;
        let rug_fuzz_7 = "";
        let rug_fuzz_8 = r"[^01]+";
        let rug_fuzz_9 = "1078910";
        let rug_fuzz_10 = 1;
        let rug_fuzz_11 = "1";
        let rug_fuzz_12 = r"[^01]+";
        let rug_fuzz_13 = "1078910";
        let rug_fuzz_14 = 3;
        let rug_fuzz_15 = "1";
        let rug_fuzz_16 = r"a+";
        let rug_fuzz_17 = "aaaaa";
        let rug_fuzz_18 = 1;
        let rug_fuzz_19 = "b";
        let rug_fuzz_20 = r"a+";
        let rug_fuzz_21 = "aaaaa";
        let rug_fuzz_22 = 3;
        let rug_fuzz_23 = "b";
        let rug_fuzz_24 = r"[^01]+";
        let rug_fuzz_25 = "1078910";
        let rug_fuzz_26 = 3;
        let rug_fuzz_27 = 0;
        debug_assert_eq!(
            Regex::new(rug_fuzz_0).unwrap().replacen(rug_fuzz_1, rug_fuzz_2, rug_fuzz_3),
            "aaaaa"
        );
        debug_assert_eq!(
            Regex::new(rug_fuzz_4).unwrap().replacen(rug_fuzz_5, rug_fuzz_6, rug_fuzz_7),
            "1010"
        );
        debug_assert_eq!(
            Regex::new(rug_fuzz_8).unwrap().replacen(rug_fuzz_9, rug_fuzz_10,
            rug_fuzz_11), "1"
        );
        debug_assert_eq!(
            Regex::new(rug_fuzz_12).unwrap().replacen(rug_fuzz_13, rug_fuzz_14,
            rug_fuzz_15), "1010"
        );
        debug_assert_eq!(
            Regex::new(rug_fuzz_16).unwrap().replacen(rug_fuzz_17, rug_fuzz_18,
            rug_fuzz_19), "baaa"
        );
        debug_assert_eq!(
            Regex::new(rug_fuzz_20).unwrap().replacen(rug_fuzz_21, rug_fuzz_22,
            rug_fuzz_23), "ba"
        );
        debug_assert_eq!(
            Regex::new(rug_fuzz_24).unwrap().replacen(rug_fuzz_25, rug_fuzz_26, | caps :
            & Captures | { format!("{}b", & caps[rug_fuzz_27]) }), "10b78910"
        );
        let _rug_ed_tests_llm_16_738_rrrruuuugggg_test_replacen = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_739 {
    use crate::Regex;
    #[test]
    fn test_shortest_match() {
        let _rug_st_tests_llm_16_739_rrrruuuugggg_test_shortest_match = 0;
        let rug_fuzz_0 = "aaaaa";
        let rug_fuzz_1 = r"a+";
        let text = rug_fuzz_0;
        let pos = Regex::new(rug_fuzz_1).unwrap().shortest_match(text);
        debug_assert_eq!(pos, Some(1));
        let _rug_ed_tests_llm_16_739_rrrruuuugggg_test_shortest_match = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_740 {
    use super::*;
    use crate::*;
    #[test]
    fn test_shortest_match_at() {
        let _rug_st_tests_llm_16_740_rrrruuuugggg_test_shortest_match_at = 0;
        let rug_fuzz_0 = "\\d{3}-\\d{3}-\\d{4}";
        let rug_fuzz_1 = "phone: 111-222-3333";
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = "phone: 111-222-3333";
        let rug_fuzz_4 = 7;
        let rug_fuzz_5 = "phone: 111-222-3333";
        let rug_fuzz_6 = 14;
        let rug_fuzz_7 = "phone: 111-222-3333";
        let rug_fuzz_8 = 20;
        let re = Regex::new(rug_fuzz_0).unwrap();
        debug_assert_eq!(re.shortest_match_at(rug_fuzz_1, rug_fuzz_2), Some(12));
        debug_assert_eq!(re.shortest_match_at(rug_fuzz_3, rug_fuzz_4), Some(12));
        debug_assert_eq!(re.shortest_match_at(rug_fuzz_5, rug_fuzz_6), Some(15));
        debug_assert_eq!(re.shortest_match_at(rug_fuzz_7, rug_fuzz_8), None);
        let _rug_ed_tests_llm_16_740_rrrruuuugggg_test_shortest_match_at = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_741 {
    use super::*;
    use crate::*;
    use crate::{Regex, Replacer};
    #[test]
    fn test_by_ref() {
        let _rug_st_tests_llm_16_741_rrrruuuugggg_test_by_ref = 0;
        let rug_fuzz_0 = r"\d+";
        let rug_fuzz_1 = "I have 10 apples and 20 oranges";
        let re = Regex::new(rug_fuzz_0).unwrap();
        let src = rug_fuzz_1;
        let mut rep = SimpleReplacer;
        let dst = re.replace_all(src, rep.by_ref());
        debug_assert_eq!(dst, "I have  apples and  oranges");
        let dst = re.replace_all(&dst, rep.by_ref());
        debug_assert_eq!(dst, "I have  apples and  oranges");
        let _rug_ed_tests_llm_16_741_rrrruuuugggg_test_by_ref = 0;
    }
    struct SimpleReplacer;
    impl Replacer for SimpleReplacer {
        fn replace_append(&mut self, caps: &crate::Captures, dst: &mut String) {
            dst.push_str(" ");
        }
        fn no_expansion(&mut self) -> Option<std::borrow::Cow<str>> {
            Some("".into())
        }
    }
}
#[cfg(test)]
mod tests_llm_16_744 {
    use crate::re_unicode::escape;
    #[test]
    fn test_escape() {
        let _rug_st_tests_llm_16_744_rrrruuuugggg_test_escape = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = "abc";
        let rug_fuzz_2 = ".*+?^$\\()[]{}|";
        let rug_fuzz_3 = "foo.bar";
        let rug_fuzz_4 = "foo(bar)";
        let rug_fuzz_5 = "foo[bar]";
        debug_assert_eq!(escape(rug_fuzz_0), "");
        debug_assert_eq!(escape(rug_fuzz_1), "abc");
        debug_assert_eq!(escape(rug_fuzz_2), "\\Q.*+?^$\\()[]{}|\\E");
        debug_assert_eq!(escape(rug_fuzz_3), "foo\\.bar");
        debug_assert_eq!(escape(rug_fuzz_4), "foo\\(bar\\)");
        debug_assert_eq!(escape(rug_fuzz_5), "foo\\[bar\\]");
        let _rug_ed_tests_llm_16_744_rrrruuuugggg_test_escape = 0;
    }
}
#[cfg(test)]
mod tests_rug_256 {
    use super::*;
    use crate::NoExpand;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_256_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "text";
        let mut p0 = NoExpand(rug_fuzz_0);
        crate::re_unicode::Replacer::no_expansion(&mut p0);
        let _rug_ed_tests_rug_256_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_259 {
    use super::*;
    use crate::Regex;
    #[test]
    fn test_regex() {
        let _rug_st_tests_rug_259_rrrruuuugggg_test_regex = 0;
        let rug_fuzz_0 = "your_regex_here";
        let rug_fuzz_1 = "your_text_here";
        let mut p0 = Regex::new(rug_fuzz_0).unwrap();
        let p1 = rug_fuzz_1;
        p0.find_iter(p1);
        let _rug_ed_tests_rug_259_rrrruuuugggg_test_regex = 0;
    }
}
#[cfg(test)]
mod tests_rug_260 {
    use super::*;
    use crate::Regex;
    #[test]
    fn test_regex_captures_iter() {
        let _rug_st_tests_rug_260_rrrruuuugggg_test_regex_captures_iter = 0;
        let rug_fuzz_0 = r"'(?P<title>[^']+)'\s+\((?P<year>\d{4})\)";
        let rug_fuzz_1 = "'Citizen Kane' (1941), 'The Wizard of Oz' (1939), 'M' (1931).";
        let mut p0 = Regex::new(rug_fuzz_0).unwrap();
        let p1 = rug_fuzz_1;
        p0.captures_iter(p1);
        let _rug_ed_tests_rug_260_rrrruuuugggg_test_regex_captures_iter = 0;
    }
}
#[cfg(test)]
mod tests_rug_261 {
    use super::*;
    use crate::Regex;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_261_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = r"[ \t]+";
        let rug_fuzz_1 = "a b \t  c\td    e";
        let mut p0 = Regex::new(rug_fuzz_0).unwrap();
        let p1 = rug_fuzz_1;
        p0.split(&p1);
        let _rug_ed_tests_rug_261_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_262 {
    use super::*;
    use crate::Regex;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_262_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = r"\W+";
        let rug_fuzz_1 = "Hey! How are you?";
        let rug_fuzz_2 = 3;
        let mut p0 = Regex::new(rug_fuzz_0).unwrap();
        let mut p1 = rug_fuzz_1;
        let mut p2 = rug_fuzz_2;
        p0.splitn(p1, p2);
        let _rug_ed_tests_rug_262_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_263 {
    use super::*;
    use crate::{Regex, NoExpand};
    #[test]
    fn test_replace_all() {
        let _rug_st_tests_rug_263_rrrruuuugggg_test_replace_all = 0;
        let rug_fuzz_0 = "your_regex_here";
        let rug_fuzz_1 = "your_sample_text_here";
        let rug_fuzz_2 = "text";
        let mut p0 = Regex::new(rug_fuzz_0).unwrap();
        let p1 = rug_fuzz_1;
        let mut p2 = NoExpand(rug_fuzz_2);
        p0.replace_all(p1, p2);
        let _rug_ed_tests_rug_263_rrrruuuugggg_test_replace_all = 0;
    }
}
#[cfg(test)]
mod tests_rug_264 {
    use super::*;
    use crate::Regex;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_264_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "your_regex_here";
        let rug_fuzz_1 = "your_text_here";
        let rug_fuzz_2 = 0;
        let mut p0 = Regex::new(rug_fuzz_0).unwrap();
        let p1 = rug_fuzz_1;
        let p2 = rug_fuzz_2;
        p0.find_at(p1, p2);
        let _rug_ed_tests_rug_264_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_265 {
    use crate::{Regex, Captures};
    use re_unicode::{self, Match};
    #[test]
    fn test_captures_read_at() {
        let _rug_st_tests_rug_265_rrrruuuugggg_test_captures_read_at = 0;
        let rug_fuzz_0 = "your_regex_here";
        let rug_fuzz_1 = "your_text_here";
        let rug_fuzz_2 = 0;
        let regex = Regex::new(rug_fuzz_0).unwrap();
        let mut locs = regex.capture_locations();
        let text = rug_fuzz_1;
        let start = rug_fuzz_2;
        let result = regex.captures_read_at(&mut locs, text, start);
        debug_assert_eq!(result, Some(Match::new(text, 0, 0)));
        let _rug_ed_tests_rug_265_rrrruuuugggg_test_captures_read_at = 0;
    }
}
#[cfg(test)]
mod tests_rug_267 {
    use super::*;
    use crate::Regex;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_267_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "your_regex_here";
        let mut p0 = Regex::new(rug_fuzz_0).unwrap();
        let _ = crate::re_unicode::Regex::capture_locations(&p0);
        let _rug_ed_tests_rug_267_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_271 {
    use super::*;
    use crate::std::iter::Iterator;
    use crate::re_unicode;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_271_rrrruuuugggg_test_rug = 0;
        let mut p0: re_unicode::Split<'_, '_> = unimplemented!();
        p0.next();
        let _rug_ed_tests_rug_271_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_278 {
    use super::*;
    use crate::re_unicode::Captures;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_278_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "replacement string";
        let mut p0: Captures = unimplemented!();
        let p1: &str = rug_fuzz_0;
        let mut p2: String = unimplemented!();
        p0.expand(p1, &mut p2);
        let _rug_ed_tests_rug_278_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_280 {
    use super::*;
    use crate::{Captures, Regex};
    #[test]
    fn test_index() {
        let _rug_st_tests_rug_280_rrrruuuugggg_test_index = 0;
        let rug_fuzz_0 = "abc 123";
        let rug_fuzz_1 = r"([a-z]+) (\d+)";
        let rug_fuzz_2 = 1;
        let text = rug_fuzz_0;
        let regex = Regex::new(rug_fuzz_1).unwrap();
        let captures = regex.captures(text).unwrap();
        let result = captures.index(rug_fuzz_2);
        debug_assert_eq!(result, "abc");
        let _rug_ed_tests_rug_280_rrrruuuugggg_test_index = 0;
    }
}
