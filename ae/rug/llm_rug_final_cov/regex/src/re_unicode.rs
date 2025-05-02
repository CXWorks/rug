use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use std::iter::FusedIterator;
use std::ops::{Index, Range};
use std::str::FromStr;
use std::sync::Arc;
use crate::find_byte::find_byte;
use crate::error::Error;
use crate::exec::{Exec, ExecNoSyncStr};
use crate::expand::expand_str;
use crate::re_builder::unicode::RegexBuilder;
use crate::re_trait::{self, RegularExpression, SubCapturesPosIter};
/// Escapes all regular expression meta characters in `text`.
///
/// The string returned may be safely used as a literal in a regular
/// expression.
pub fn escape(text: &str) -> String {
    regex_syntax::escape(text)
}
/// Match represents a single match of a regex in a haystack.
///
/// The lifetime parameter `'t` refers to the lifetime of the matched text.
#[derive(Copy, Clone, Eq, PartialEq)]
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
    /// Returns true if and only if this match has a length of zero.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
    /// Returns the length, in bytes, of this match.
    #[inline]
    pub fn len(&self) -> usize {
        self.end - self.start
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
            start,
            end,
        }
    }
}
impl<'t> std::fmt::Debug for Match<'t> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("Match")
            .field("start", &self.start)
            .field("end", &self.end)
            .field("string", &self.as_str())
            .finish()
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
///            vec![(1, "111"), (5, "222")]);
/// assert_eq!(haystack.split(&re).collect::<Vec<_>>(), vec!["a", "b", "c"]);
/// ```
#[derive(Clone)]
pub struct Regex(Exec);
impl fmt::Display for Regex {
    /// Shows the original regular expression.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
impl fmt::Debug for Regex {
    /// Shows the original regular expression.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
    /// # use regex::Regex;
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
    /// # use regex::Regex;
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
    /// # use regex::Regex;
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
    /// # use regex::Regex;
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
    /// # use regex::Regex;
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
        self.captures_at(text, 0)
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
    /// # use regex::Regex;
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
    /// # use regex::Regex;
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
    /// # use regex::Regex;
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
    /// # use regex::Regex;
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
    /// # use regex::Regex;
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
    /// # use regex::Regex;
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
    /// # use regex::Regex;
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
    /// # use regex::Regex;
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
                new.push_str(&text[last_match..m.start()]);
                new.push_str(&rep);
                last_match = m.end();
                if limit > 0 && i >= limit - 1 {
                    break;
                }
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
            let m = cap.get(0).unwrap();
            new.push_str(&text[last_match..m.start()]);
            rep.replace_append(&cap, &mut new);
            last_match = m.end();
            if limit > 0 && i >= limit - 1 {
                break;
            }
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
    /// of the leftmost-first match that you would find via `Regex::find`.
    ///
    /// Note that it is not guaranteed that this routine finds the shortest or
    /// "earliest" possible match. Instead, the main idea of this API is that
    /// it returns the offset at the point at which the internal regex engine
    /// has determined that a match has occurred. This may vary depending on
    /// which internal regex engine is used, and thus, the offset itself may
    /// change.
    ///
    /// # Example
    ///
    /// Typically, `a+` would match the entire first sequence of `a` in some
    /// text, but `shortest_match` can give up as soon as it sees the first
    /// `a`.
    ///
    /// ```rust
    /// # use regex::Regex;
    /// # fn main() {
    /// let text = "aaaaa";
    /// let pos = Regex::new(r"a+").unwrap().shortest_match(text);
    /// assert_eq!(pos, Some(1));
    /// # }
    /// ```
    pub fn shortest_match(&self, text: &str) -> Option<usize> {
        self.shortest_match_at(text, 0)
    }
    /// Returns the same as `shortest_match`, but starts the search at the
    /// given offset.
    ///
    /// The significance of the starting point is that it takes the surrounding
    /// context into consideration. For example, the `\A` anchor can only match
    /// when `start == 0`.
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
        self.0.searcher_str().is_match_at(text, start)
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
    /// Returns the same as [`Regex::captures`], but starts the search at the
    /// given offset.
    ///
    /// The significance of the starting point is that it takes the surrounding
    /// context into consideration. For example, the `\A` anchor can only
    /// match when `start == 0`.
    pub fn captures_at<'t>(&self, text: &'t str, start: usize) -> Option<Captures<'t>> {
        let mut locs = self.capture_locations();
        self.captures_read_at(&mut locs, text, start)
            .map(move |_| Captures {
                text,
                locs: locs.0,
                named_groups: self.0.capture_name_idx().clone(),
            })
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
    pub fn capture_names(&self) -> CaptureNames<'_> {
        CaptureNames(self.0.capture_names().iter())
    }
    /// Returns the number of captures.
    pub fn captures_len(&self) -> usize {
        self.0.capture_names().len()
    }
    /// Returns the total number of capturing groups that appear in every
    /// possible match.
    ///
    /// If the number of capture groups can vary depending on the match, then
    /// this returns `None`. That is, a value is only returned when the number
    /// of matching groups is invariant or "static."
    ///
    /// Note that like [`Regex::captures_len`], this **does** include the
    /// implicit capturing group corresponding to the entire match. Therefore,
    /// when a non-None value is returned, it is guaranteed to be at least `1`.
    /// Stated differently, a return value of `Some(0)` is impossible.
    ///
    /// # Example
    ///
    /// This shows a few cases where a static number of capture groups is
    /// available and a few cases where it is not.
    ///
    /// ```
    /// use regex::Regex;
    ///
    /// let len = |pattern| {
    ///     Regex::new(pattern).map(|re| re.static_captures_len())
    /// };
    ///
    /// assert_eq!(Some(1), len("a")?);
    /// assert_eq!(Some(2), len("(a)")?);
    /// assert_eq!(Some(2), len("(a)|(b)")?);
    /// assert_eq!(Some(3), len("(a)(b)|(c)(d)")?);
    /// assert_eq!(None, len("(a)|b")?);
    /// assert_eq!(None, len("a|(b)")?);
    /// assert_eq!(None, len("(b)*")?);
    /// assert_eq!(Some(2), len("(b)+")?);
    ///
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[inline]
    pub fn static_captures_len(&self) -> Option<usize> {
        self.0.static_captures_len().map(|len| len.saturating_add(1))
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
///
/// # Example
///
/// This example shows how to create and use `CaptureLocations` in a search.
///
/// ```
/// use regex::Regex;
///
/// let re = Regex::new(r"(?<first>\w+)\s+(?<last>\w+)").unwrap();
/// let mut locs = re.capture_locations();
/// let m = re.captures_read(&mut locs, "Bruce Springsteen").unwrap();
/// assert_eq!(0..17, m.range());
/// assert_eq!(Some((0, 17)), locs.get(0));
/// assert_eq!(Some((0, 5)), locs.get(1));
/// assert_eq!(Some((6, 17)), locs.get(2));
///
/// // Asking for an invalid capture group always returns None.
/// assert_eq!(None, locs.get(3));
/// assert_eq!(None, locs.get(34973498648));
/// assert_eq!(None, locs.get(9944060567225171988));
/// ```
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
    /// Returns the total number of capture groups (even if they didn't match).
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
    /// Returns the total number of capture groups (even if they didn't match).
    ///
    /// This is always at least `1`, since every regex has at least one capture
    /// group that corresponds to the full match.
    #[inline]
    pub fn len(&self) -> usize {
        self.locs.len()
    }
}
impl<'t> fmt::Debug for Captures<'t> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Captures").field(&CapturesDebug(self)).finish()
    }
}
struct CapturesDebug<'c, 't>(&'c Captures<'t>);
impl<'c, 't> fmt::Debug for CapturesDebug<'c, 't> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
pub struct SubCaptureMatches<'c, 't> {
    caps: &'c Captures<'t>,
    it: SubCapturesPosIter<'c>,
}
impl<'c, 't> Iterator for SubCaptureMatches<'c, 't> {
    type Item = Option<Match<'t>>;
    fn next(&mut self) -> Option<Option<Match<'t>>> {
        self.it.next().map(|cap| cap.map(|(s, e)| Match::new(self.caps.text, s, e)))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.it.size_hint()
    }
    fn count(self) -> usize {
        self.it.count()
    }
}
impl<'c, 't> ExactSizeIterator for SubCaptureMatches<'c, 't> {}
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
                locs,
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
/// since implementations are already provided for `&str` along with other
/// variants of string types and `FnMut(&Captures) -> String` (or any
/// `FnMut(&Captures) -> T` where `T: AsRef<str>`), which covers most use cases.
pub trait Replacer {
    /// Appends text to `dst` to replace the current match.
    ///
    /// The current match is represented by `caps`, which is guaranteed to
    /// have a match at capture group `0`.
    ///
    /// For example, a no-op replacement would be
    /// `dst.push_str(caps.get(0).unwrap().as_str())`.
    fn replace_append(&mut self, caps: &Captures<'_>, dst: &mut String);
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
pub struct ReplacerRef<'a, R: ?Sized>(&'a mut R);
impl<'a, R: Replacer + ?Sized + 'a> Replacer for ReplacerRef<'a, R> {
    fn replace_append(&mut self, caps: &Captures<'_>, dst: &mut String) {
        self.0.replace_append(caps, dst)
    }
    fn no_expansion(&mut self) -> Option<Cow<'_, str>> {
        self.0.no_expansion()
    }
}
impl<'a> Replacer for &'a str {
    fn replace_append(&mut self, caps: &Captures<'_>, dst: &mut String) {
        caps.expand(*self, dst);
    }
    fn no_expansion(&mut self) -> Option<Cow<'_, str>> {
        no_expansion(self)
    }
}
impl<'a> Replacer for &'a String {
    fn replace_append(&mut self, caps: &Captures<'_>, dst: &mut String) {
        self.as_str().replace_append(caps, dst)
    }
    fn no_expansion(&mut self) -> Option<Cow<'_, str>> {
        no_expansion(self)
    }
}
impl Replacer for String {
    fn replace_append(&mut self, caps: &Captures<'_>, dst: &mut String) {
        self.as_str().replace_append(caps, dst)
    }
    fn no_expansion(&mut self) -> Option<Cow<'_, str>> {
        no_expansion(self)
    }
}
impl<'a> Replacer for Cow<'a, str> {
    fn replace_append(&mut self, caps: &Captures<'_>, dst: &mut String) {
        self.as_ref().replace_append(caps, dst)
    }
    fn no_expansion(&mut self) -> Option<Cow<'_, str>> {
        no_expansion(self)
    }
}
impl<'a> Replacer for &'a Cow<'a, str> {
    fn replace_append(&mut self, caps: &Captures<'_>, dst: &mut String) {
        self.as_ref().replace_append(caps, dst)
    }
    fn no_expansion(&mut self) -> Option<Cow<'_, str>> {
        no_expansion(self)
    }
}
fn no_expansion<T: AsRef<str>>(t: &T) -> Option<Cow<'_, str>> {
    let s = t.as_ref();
    match find_byte(b'$', s.as_bytes()) {
        Some(_) => None,
        None => Some(Cow::Borrowed(s)),
    }
}
impl<F, T> Replacer for F
where
    F: FnMut(&Captures<'_>) -> T,
    T: AsRef<str>,
{
    fn replace_append(&mut self, caps: &Captures<'_>, dst: &mut String) {
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
    fn replace_append(&mut self, _: &Captures<'_>, dst: &mut String) {
        dst.push_str(self.0);
    }
    fn no_expansion(&mut self) -> Option<Cow<'_, str>> {
        Some(Cow::Borrowed(self.0))
    }
}
#[cfg(test)]
mod tests_rug_382 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_382_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "Hello, World!";
        let mut p0: &str = rug_fuzz_0;
        crate::re_unicode::escape(&p0);
        let _rug_ed_tests_rug_382_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_384 {
    use super::*;
    use crate::Regex;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_384_rrrruuuugggg_test_rug = 0;
        let mut p0: std::string::String = String::new();
        crate::re_unicode::Replacer::no_expansion(&mut p0);
        let _rug_ed_tests_rug_384_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_385 {
    use super::*;
    use crate::Replacer;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_385_rrrruuuugggg_test_rug = 0;
        let mut p0: std::string::String = String::new();
        crate::re_unicode::Replacer::by_ref(&mut p0);
        let _rug_ed_tests_rug_385_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_388 {
    use super::*;
    use crate::re_unicode::Match;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_388_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = "test";
        let mut p0: Match<'static> = Match {
            start: rug_fuzz_0,
            end: rug_fuzz_1,
            text: rug_fuzz_2,
        };
        debug_assert_eq!(p0.is_empty(), true);
        let _rug_ed_tests_rug_388_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_390 {
    use super::*;
    use crate::re_unicode::Match;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_390_rrrruuuugggg_test_rug = 0;
        let mut p0: Match<'static> = unimplemented!();
        p0.range();
        let _rug_ed_tests_rug_390_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_392 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_392_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "abc";
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 3;
        let mut p0 = rug_fuzz_0;
        let mut p1 = rug_fuzz_1;
        let mut p2 = rug_fuzz_2;
        crate::re_unicode::Match::<'static>::new(&p0, p1, p2);
        let _rug_ed_tests_rug_392_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_397 {
    use super::*;
    use crate::re_unicode::{Regex, Error, RegexBuilder};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_397_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "abc";
        let p0: &str = rug_fuzz_0;
        Regex::new(&p0).unwrap();
        let _rug_ed_tests_rug_397_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_398 {
    use super::*;
    use crate::Regex;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_398_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = "";
        let mut p0: Regex = Regex::new(rug_fuzz_0).unwrap();
        let mut p1: &str = rug_fuzz_1;
        p0.is_match(&p1);
        let _rug_ed_tests_rug_398_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_399 {
    use super::*;
    use crate::Regex;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_399_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = "sample_text";
        let mut p0: Regex = Regex::new(rug_fuzz_0).unwrap();
        let mut p1: &str = rug_fuzz_1;
        p0.find(p1);
        let _rug_ed_tests_rug_399_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_400 {
    use super::*;
    use crate::Regex;
    #[test]
    fn test_regex_find_iter() {
        let _rug_st_tests_rug_400_rrrruuuugggg_test_regex_find_iter = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = "";
        let mut p0: Regex = Regex::new(rug_fuzz_0).unwrap();
        let mut p1: &str = rug_fuzz_1;
        p0.find_iter(p1);
        let _rug_ed_tests_rug_400_rrrruuuugggg_test_regex_find_iter = 0;
    }
}
#[cfg(test)]
mod tests_rug_401 {
    use super::*;
    use crate::Regex;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_401_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = "Not my favorite movie: 'Citizen Kane' (1941).";
        let mut p0: Regex = Regex::new(rug_fuzz_0).unwrap();
        let p1: &str = rug_fuzz_1;
        p0.captures(p1);
        let _rug_ed_tests_rug_401_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_402 {
    use super::*;
    use crate::Regex;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_402_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "'Citizen Kane' (1941), 'The Wizard of Oz' (1939), 'M' (1931).";
        let rug_fuzz_1 = r"'(?P<title>[^']+)'\s+\((?P<year>\d{4})\)";
        let text = rug_fuzz_0;
        let p0: Regex = Regex::new(rug_fuzz_1).unwrap();
        let p1: &str = text;
        p0.captures_iter(p1);
        let _rug_ed_tests_rug_402_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_403 {
    use super::*;
    use crate::Regex;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_403_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = "a b \t  c\td    e";
        let mut p0: Regex = Regex::new(rug_fuzz_0).unwrap();
        let p1: &str = rug_fuzz_1;
        p0.split(p1);
        let _rug_ed_tests_rug_403_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_404 {
    use super::*;
    use crate::Regex;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_404_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = "";
        let rug_fuzz_2 = 0;
        let mut p0: Regex = Regex::new(rug_fuzz_0).unwrap();
        let mut p1: &str = rug_fuzz_1;
        let mut p2: usize = rug_fuzz_2;
        p0.splitn(p1, p2);
        let _rug_ed_tests_rug_404_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_405 {
    use super::*;
    use crate::{Regex, NoExpand, Replacer};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_405_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = "sample_text";
        let rug_fuzz_2 = "sample replacement";
        let p0: Regex = Regex::new(rug_fuzz_0).unwrap();
        let p1: &str = rug_fuzz_1;
        let p2: NoExpand = NoExpand(rug_fuzz_2);
        p0.replace(p1, p2);
        let _rug_ed_tests_rug_405_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_406 {
    use super::*;
    use crate::Regex;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_406_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = "";
        let mut p0: Regex = Regex::new(rug_fuzz_0).unwrap();
        let p1: &str = rug_fuzz_1;
        let mut p2: std::string::String = String::new();
        p0.replace_all(p1, &p2);
        let _rug_ed_tests_rug_406_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_407 {
    use super::*;
    use crate::Regex;
    use std::borrow::Cow;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_407_rrrruuuugggg_sample = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = "sample data";
        let rug_fuzz_2 = "some text";
        let rug_fuzz_3 = 5;
        #[cfg(test)]
        mod tests_rug_407_re_unicode {
            use super::*;
            #[test]
            fn sample() {
                let _rug_st_tests_rug_407_re_unicode_rrrruuuugggg_sample = 0;
                let rug_fuzz_0 = 0;
                let rug_fuzz_1 = "";
                let rug_fuzz_2 = "sample data";
                let rug_fuzz_3 = "some text";
                let rug_fuzz_4 = 5;
                let rug_fuzz_5 = 0;
                let _rug_st_tests_rug_407_rrrruuuugggg_sample = rug_fuzz_0;
                let rug_fuzz_0 = rug_fuzz_1;
                let rug_fuzz_1 = rug_fuzz_2;
                let rug_fuzz_2 = rug_fuzz_3;
                let rug_fuzz_3 = rug_fuzz_4;
                let mut v96: Regex = Regex::new(rug_fuzz_0).unwrap();
                let mut v69: Cow<str> = Cow::Borrowed(rug_fuzz_1);
                let p0 = &v96;
                let p1 = rug_fuzz_2;
                let p2 = rug_fuzz_3;
                let p3 = &v69;
                p0.replacen(p1, p2, p3);
                let _rug_ed_tests_rug_407_rrrruuuugggg_sample = rug_fuzz_5;
                let _rug_ed_tests_rug_407_re_unicode_rrrruuuugggg_sample = 0;
            }
        }
        let _rug_ed_tests_rug_407_rrrruuuugggg_sample = 0;
    }
}
#[cfg(test)]
mod tests_rug_408 {
    use super::*;
    use crate::Regex;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_408_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = "sample data";
        let mut p0: Regex = Regex::new(rug_fuzz_0).unwrap();
        let p1: &str = rug_fuzz_1;
        p0.shortest_match(p1);
        let _rug_ed_tests_rug_408_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_409 {
    use super::*;
    use crate::Regex;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_409_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = "";
        let rug_fuzz_2 = 0;
        let mut p0: Regex = Regex::new(rug_fuzz_0).unwrap();
        let p1: &str = rug_fuzz_1;
        let p2: usize = rug_fuzz_2;
        p0.shortest_match_at(p1, p2);
        let _rug_ed_tests_rug_409_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_410 {
    use super::*;
    use crate::Regex;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_410_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = "test";
        let rug_fuzz_2 = 0;
        let mut p0: Regex = Regex::new(rug_fuzz_0).unwrap();
        let p1: &str = rug_fuzz_1;
        let p2: usize = rug_fuzz_2;
        p0.is_match_at(p1, p2);
        let _rug_ed_tests_rug_410_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_411 {
    use super::*;
    use crate::{Regex, Match};
    #[test]
    fn test_find_at() {
        let _rug_st_tests_rug_411_rrrruuuugggg_test_find_at = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = "";
        let rug_fuzz_2 = 0;
        let mut regex: Regex = Regex::new(rug_fuzz_0).unwrap();
        let text: &str = rug_fuzz_1;
        let start: usize = rug_fuzz_2;
        let result: Option<Match> = regex.find_at(text, start);
        debug_assert_eq!(result, None);
        let _rug_ed_tests_rug_411_rrrruuuugggg_test_find_at = 0;
    }
}
#[cfg(test)]
mod tests_rug_412 {
    use super::*;
    use crate::Regex;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_412_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = "";
        let rug_fuzz_2 = 0;
        let regex: Regex = Regex::new(rug_fuzz_0).unwrap();
        let text: &str = rug_fuzz_1;
        let start: usize = rug_fuzz_2;
        regex.captures_at(text, start);
        let _rug_ed_tests_rug_412_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_413 {
    use super::*;
    use crate::Regex;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_413_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = "test";
        let mut p0: Regex = Regex::new(rug_fuzz_0).unwrap();
        let mut p1 = p0.capture_locations();
        let mut p2 = rug_fuzz_1;
        p0.captures_read(&mut p1, &p2);
        let _rug_ed_tests_rug_413_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_414 {
    use super::*;
    use crate::{Regex, CaptureLocations};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_414_rrrruuuugggg_sample = 0;
        let rug_fuzz_0 = "";
        #[cfg(test)]
        mod tests_rug_414_prepare {
            #[test]
            fn sample() {
                let _rug_st_tests_rug_414_prepare_rrrruuuugggg_sample = 0;
                let rug_fuzz_0 = 0;
                let rug_fuzz_1 = "";
                let rug_fuzz_2 = 0;
                let _rug_st_tests_rug_414_rrrruuuugggg_sample = rug_fuzz_0;
                let rug_fuzz_0 = rug_fuzz_1;
                use crate::Regex;
                let mut p0: Regex = Regex::new(rug_fuzz_0).unwrap();
                let _rug_ed_tests_rug_414_rrrruuuugggg_sample = rug_fuzz_2;
                let _rug_ed_tests_rug_414_prepare_rrrruuuugggg_sample = 0;
            }
        }
        let mut p0: Regex = Regex::new("").unwrap();
        let mut p1: CaptureLocations = p0.capture_locations();
        let p2: &str = "test_string";
        let p3: usize = 0;
        p0.captures_read_at(&mut p1, &p2, p3);
        let _rug_ed_tests_rug_414_rrrruuuugggg_sample = 0;
    }
}
#[cfg(test)]
mod tests_rug_417 {
    use super::*;
    use crate::Regex;
    #[test]
    fn test_capture_names() {
        let _rug_st_tests_rug_417_rrrruuuugggg_test_capture_names = 0;
        let rug_fuzz_0 = "";
        let mut p0: Regex = Regex::new(rug_fuzz_0).unwrap();
        let result = p0.capture_names();
        let _rug_ed_tests_rug_417_rrrruuuugggg_test_capture_names = 0;
    }
}
#[cfg(test)]
mod tests_rug_418 {
    use super::*;
    use crate::Regex;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_418_rrrruuuugggg_sample = 0;
        let rug_fuzz_0 = "";
        #[cfg(test)]
        mod tests_rug_418_prepare {
            #[test]
            fn sample() {
                let _rug_st_tests_rug_418_prepare_rrrruuuugggg_sample = 0;
                let rug_fuzz_0 = 0;
                let rug_fuzz_1 = "";
                let rug_fuzz_2 = 0;
                let _rug_st_tests_rug_418_rrrruuuugggg_sample = rug_fuzz_0;
                let rug_fuzz_0 = rug_fuzz_1;
                use crate::Regex;
                let mut v96: Regex = Regex::new(rug_fuzz_0).unwrap();
                let _rug_ed_tests_rug_418_rrrruuuugggg_sample = rug_fuzz_2;
                let _rug_ed_tests_rug_418_prepare_rrrruuuugggg_sample = 0;
            }
        }
        let mut p0: Regex = Regex::new("").unwrap();
        crate::re_unicode::Regex::captures_len(&p0);
        let _rug_ed_tests_rug_418_rrrruuuugggg_sample = 0;
    }
}
#[cfg(test)]
mod tests_rug_419 {
    use super::*;
    use crate::Regex;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_419_rrrruuuugggg_sample = 0;
        let rug_fuzz_0 = "";
        #[cfg(test)]
        mod tests_rug_419_prepare {
            #[test]
            fn sample() {
                let _rug_st_tests_rug_419_prepare_rrrruuuugggg_sample = 0;
                let rug_fuzz_0 = 0;
                let rug_fuzz_1 = "";
                let rug_fuzz_2 = 0;
                let _rug_st_tests_rug_419_rrrruuuugggg_sample = rug_fuzz_0;
                let rug_fuzz_0 = rug_fuzz_1;
                use crate::Regex;
                let mut p0: Regex = Regex::new(rug_fuzz_0).unwrap();
                let _ = p0.static_captures_len();
                let _rug_ed_tests_rug_419_rrrruuuugggg_sample = rug_fuzz_2;
                let _rug_ed_tests_rug_419_prepare_rrrruuuugggg_sample = 0;
            }
        }
        let _rug_ed_tests_rug_419_rrrruuuugggg_sample = 0;
    }
}
#[cfg(test)]
mod tests_rug_420 {
    use super::*;
    use crate::Regex;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_420_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "";
        let mut v96: Regex = Regex::new(rug_fuzz_0).unwrap();
        let mut p0 = v96.capture_locations();
        crate::re_unicode::Regex::capture_locations(&v96);
        let _rug_ed_tests_rug_420_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_421 {
    use crate::{Regex, RegexBuilder};
    use crate::re_unicode::CaptureLocations;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_421_rrrruuuugggg_sample = 0;
        let rug_fuzz_0 = "";
        #[cfg(test)]
        mod tests_rug_421_prepare {
            #[test]
            fn sample() {
                let _rug_st_tests_rug_421_prepare_rrrruuuugggg_sample = 0;
                let rug_fuzz_0 = 0;
                let rug_fuzz_1 = "";
                let rug_fuzz_2 = 0;
                let _rug_st_tests_rug_421_rrrruuuugggg_sample = rug_fuzz_0;
                let rug_fuzz_0 = rug_fuzz_1;
                use crate::Regex;
                let v96: Regex = Regex::new(rug_fuzz_0).unwrap();
                let _rug_ed_tests_rug_421_rrrruuuugggg_sample = rug_fuzz_2;
                let _rug_ed_tests_rug_421_prepare_rrrruuuugggg_sample = 0;
            }
        }
        let p0: Regex = RegexBuilder::new("").build().unwrap();
        p0.locations();
        let _rug_ed_tests_rug_421_rrrruuuugggg_sample = 0;
    }
}
#[cfg(test)]
mod tests_rug_431 {
    use super::*;
    use crate::{Regex, Captures, Match};
    #[test]
    fn test_get() {
        let _rug_st_tests_rug_431_rrrruuuugggg_test_get = 0;
        let rug_fuzz_0 = r"[a-z]+(?:([0-9]+)|([A-Z]+))";
        let rug_fuzz_1 = "abc123";
        let rug_fuzz_2 = 1;
        let re = Regex::new(rug_fuzz_0).unwrap();
        let caps: Captures<'_> = re.captures(rug_fuzz_1).unwrap();
        let i = rug_fuzz_2;
        let result: Option<Match<'_>> = caps.get(i);
        let _rug_ed_tests_rug_431_rrrruuuugggg_test_get = 0;
    }
}
#[test]
fn test_rug() {
    let pattern = Regex::new(r"pattern").unwrap();
    let text = "text";
    let mut p0: CaptureMatches<'_, '_> = pattern.captures_iter(text);
    p0.next();
}
#[cfg(test)]
mod tests_rug_446 {
    use super::*;
    use crate::Replacer;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_446_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "some_str";
        let mut p0: &'static str = rug_fuzz_0;
        p0.no_expansion();
        let _rug_ed_tests_rug_446_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_450 {
    use super::*;
    use crate::Replacer;
    use std::borrow::Cow;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_450_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "some sample string";
        let mut p0: String = String::from(rug_fuzz_0);
        p0.no_expansion();
        let _rug_ed_tests_rug_450_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_452 {
    use super::*;
    use crate::Replacer;
    use std::borrow::Cow;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_452_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample data";
        let mut p0: Cow<'static, str> = Cow::Borrowed(rug_fuzz_0);
        p0.no_expansion();
        let _rug_ed_tests_rug_452_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_454 {
    use super::*;
    use crate::Replacer;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_454_rrrruuuugggg_sample = 0;
        let rug_fuzz_0 = "sample data";
        #[cfg(test)]
        mod tests_rug_454_prepare {
            use std::borrow::Cow;
            #[test]
            fn sample() {
                let _rug_st_tests_rug_454_prepare_rrrruuuugggg_sample = 0;
                let rug_fuzz_0 = 0;
                let rug_fuzz_1 = "sample data";
                let rug_fuzz_2 = 0;
                let _rug_st_tests_rug_454_rrrruuuugggg_sample = rug_fuzz_0;
                let rug_fuzz_0 = rug_fuzz_1;
                let mut p0: std::borrow::Cow<'_, str> = std::borrow::Cow::Borrowed(
                    rug_fuzz_0,
                );
                let _rug_ed_tests_rug_454_rrrruuuugggg_sample = rug_fuzz_2;
                let _rug_ed_tests_rug_454_prepare_rrrruuuugggg_sample = 0;
            }
        }
        let mut p0: std::borrow::Cow<'_, str> = std::borrow::Cow::Borrowed(
            "sample data",
        );
        p0.no_expansion();
        let _rug_ed_tests_rug_454_rrrruuuugggg_sample = 0;
    }
}
#[cfg(test)]
mod tests_rug_457 {
    use super::*;
    use crate::NoExpand;
    use crate::Replacer;
    use std::borrow::Cow;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_457_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "example text";
        let s0 = rug_fuzz_0;
        let mut p0: NoExpand<'_> = NoExpand(s0);
        let result = p0.no_expansion();
        debug_assert_eq!(result, Some(Cow::Borrowed(s0)));
        let _rug_ed_tests_rug_457_rrrruuuugggg_test_rug = 0;
    }
}
