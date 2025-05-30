use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use std::iter::FusedIterator;
use std::ops::{Index, Range};
use std::str::FromStr;
use std::sync::Arc;
use find_byte::find_byte;
use error::Error;
use exec::{Exec, ExecNoSync};
use expand::expand_bytes;
use re_builder::bytes::RegexBuilder;
use re_trait::{self, RegularExpression, SubCapturesPosIter};
/// Match represents a single match of a regex in a haystack.
///
/// The lifetime parameter `'t` refers to the lifetime of the matched text.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Match<'t> {
    text: &'t [u8],
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
    pub fn as_bytes(&self) -> &'t [u8] {
        &self.text[self.range()]
    }
    /// Creates a new match from the given haystack and byte offsets.
    #[inline]
    fn new(haystack: &'t [u8], start: usize, end: usize) -> Match<'t> {
        Match {
            text: haystack,
            start: start,
            end: end,
        }
    }
}
impl<'t> From<Match<'t>> for Range<usize> {
    fn from(m: Match<'t>) -> Range<usize> {
        m.range()
    }
}
/// A compiled regular expression for matching arbitrary bytes.
///
/// It can be used to search, split or replace text. All searching is done with
/// an implicit `.*?` at the beginning and end of an expression. To force an
/// expression to match the whole string (or a prefix or a suffix), you must
/// use an anchor like `^` or `$` (or `\A` and `\z`).
///
/// Like the `Regex` type in the parent module, matches with this regex return
/// byte offsets into the search text. **Unlike** the parent `Regex` type,
/// these byte offsets may not correspond to UTF-8 sequence boundaries since
/// the regexes in this module can match arbitrary bytes.
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
/// A constructor for Regex from an Exec.
///
/// This is hidden because Exec isn't actually part of the public API.
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
    /// Test if some text contains at least one word with exactly 13 ASCII word
    /// bytes:
    ///
    /// ```rust
    /// # extern crate regex; use regex::bytes::Regex;
    /// # fn main() {
    /// let text = b"I categorically deny having triskaidekaphobia.";
    /// assert!(Regex::new(r"\b\w{13}\b").unwrap().is_match(text));
    /// # }
    /// ```
    pub fn is_match(&self, text: &[u8]) -> bool {
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
    /// ASCII word bytes:
    ///
    /// ```rust
    /// # extern crate regex; use regex::bytes::Regex;
    /// # fn main() {
    /// let text = b"I categorically deny having triskaidekaphobia.";
    /// let mat = Regex::new(r"\b\w{13}\b").unwrap().find(text).unwrap();
    /// assert_eq!((mat.start(), mat.end()), (2, 15));
    /// # }
    /// ```
    pub fn find<'t>(&self, text: &'t [u8]) -> Option<Match<'t>> {
        self.find_at(text, 0)
    }
    /// Returns an iterator for each successive non-overlapping match in
    /// `text`, returning the start and end byte indices with respect to
    /// `text`.
    ///
    /// # Example
    ///
    /// Find the start and end location of every word with exactly 13 ASCII
    /// word bytes:
    ///
    /// ```rust
    /// # extern crate regex; use regex::bytes::Regex;
    /// # fn main() {
    /// let text = b"Retroactively relinquishing remunerations is reprehensible.";
    /// for mat in Regex::new(r"\b\w{13}\b").unwrap().find_iter(text) {
    ///     println!("{:?}", mat);
    /// }
    /// # }
    /// ```
    pub fn find_iter<'r, 't>(&'r self, text: &'t [u8]) -> Matches<'r, 't> {
        Matches(self.0.searcher().find_iter(text))
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
    /// # extern crate regex; use regex::bytes::Regex;
    /// # fn main() {
    /// let re = Regex::new(r"'([^']+)'\s+\((\d{4})\)").unwrap();
    /// let text = b"Not my favorite movie: 'Citizen Kane' (1941).";
    /// let caps = re.captures(text).unwrap();
    /// assert_eq!(caps.get(1).unwrap().as_bytes(), &b"Citizen Kane"[..]);
    /// assert_eq!(caps.get(2).unwrap().as_bytes(), &b"1941"[..]);
    /// assert_eq!(caps.get(0).unwrap().as_bytes(), &b"'Citizen Kane' (1941)"[..]);
    /// // You can also access the groups by index using the Index notation.
    /// // Note that this will panic on an invalid index.
    /// assert_eq!(&caps[1], b"Citizen Kane");
    /// assert_eq!(&caps[2], b"1941");
    /// assert_eq!(&caps[0], b"'Citizen Kane' (1941)");
    /// # }
    /// ```
    ///
    /// Note that the full match is at capture group `0`. Each subsequent
    /// capture group is indexed by the order of its opening `(`.
    ///
    /// We can make this example a bit clearer by using *named* capture groups:
    ///
    /// ```rust
    /// # extern crate regex; use regex::bytes::Regex;
    /// # fn main() {
    /// let re = Regex::new(r"'(?P<title>[^']+)'\s+\((?P<year>\d{4})\)")
    ///                .unwrap();
    /// let text = b"Not my favorite movie: 'Citizen Kane' (1941).";
    /// let caps = re.captures(text).unwrap();
    /// assert_eq!(caps.name("title").unwrap().as_bytes(), b"Citizen Kane");
    /// assert_eq!(caps.name("year").unwrap().as_bytes(), b"1941");
    /// assert_eq!(caps.get(0).unwrap().as_bytes(), &b"'Citizen Kane' (1941)"[..]);
    /// // You can also access the groups by name using the Index notation.
    /// // Note that this will panic on an invalid group name.
    /// assert_eq!(&caps["title"], b"Citizen Kane");
    /// assert_eq!(&caps["year"], b"1941");
    /// assert_eq!(&caps[0], b"'Citizen Kane' (1941)");
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
    pub fn captures<'t>(&self, text: &'t [u8]) -> Option<Captures<'t>> {
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
    /// # extern crate regex; use std::str; use regex::bytes::Regex;
    /// # fn main() {
    /// let re = Regex::new(r"'(?P<title>[^']+)'\s+\((?P<year>\d{4})\)")
    ///                .unwrap();
    /// let text = b"'Citizen Kane' (1941), 'The Wizard of Oz' (1939), 'M' (1931).";
    /// for caps in re.captures_iter(text) {
    ///     let title = str::from_utf8(&caps["title"]).unwrap();
    ///     let year = str::from_utf8(&caps["year"]).unwrap();
    ///     println!("Movie: {:?}, Released: {:?}", title, year);
    /// }
    /// // Output:
    /// // Movie: Citizen Kane, Released: 1941
    /// // Movie: The Wizard of Oz, Released: 1939
    /// // Movie: M, Released: 1931
    /// # }
    /// ```
    pub fn captures_iter<'r, 't>(&'r self, text: &'t [u8]) -> CaptureMatches<'r, 't> {
        CaptureMatches(self.0.searcher().captures_iter(text))
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
    /// # extern crate regex; use regex::bytes::Regex;
    /// # fn main() {
    /// let re = Regex::new(r"[ \t]+").unwrap();
    /// let fields: Vec<&[u8]> = re.split(b"a b \t  c\td    e").collect();
    /// assert_eq!(fields, vec![
    ///     &b"a"[..], &b"b"[..], &b"c"[..], &b"d"[..], &b"e"[..],
    /// ]);
    /// # }
    /// ```
    pub fn split<'r, 't>(&'r self, text: &'t [u8]) -> Split<'r, 't> {
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
    /// # extern crate regex; use regex::bytes::Regex;
    /// # fn main() {
    /// let re = Regex::new(r"\W+").unwrap();
    /// let fields: Vec<&[u8]> = re.splitn(b"Hey! How are you?", 3).collect();
    /// assert_eq!(fields, vec![&b"Hey"[..], &b"How"[..], &b"are you?"[..]]);
    /// # }
    /// ```
    pub fn splitn<'r, 't>(&'r self, text: &'t [u8], limit: usize) -> SplitN<'r, 't> {
        SplitN {
            splits: self.split(text),
            n: limit,
        }
    }
    /// Replaces the leftmost-first match with the replacement provided. The
    /// replacement can be a regular byte string (where `$N` and `$name` are
    /// expanded to match capture groups) or a function that takes the matches'
    /// `Captures` and returns the replaced byte string.
    ///
    /// If no match is found, then a copy of the byte string is returned
    /// unchanged.
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
    /// In typical usage, this can just be a normal byte string:
    ///
    /// ```rust
    /// # extern crate regex; use regex::bytes::Regex;
    /// # fn main() {
    /// let re = Regex::new("[^01]+").unwrap();
    /// assert_eq!(re.replace(b"1078910", &b""[..]), &b"1010"[..]);
    /// # }
    /// ```
    ///
    /// But anything satisfying the `Replacer` trait will work. For example, a
    /// closure of type `|&Captures| -> Vec<u8>` provides direct access to the
    /// captures corresponding to a match. This allows one to access capturing
    /// group matches easily:
    ///
    /// ```rust
    /// # extern crate regex; use regex::bytes::Regex;
    /// # use regex::bytes::Captures; fn main() {
    /// let re = Regex::new(r"([^,\s]+),\s+(\S+)").unwrap();
    /// let result = re.replace(b"Springsteen, Bruce", |caps: &Captures| {
    ///     let mut replacement = caps[2].to_owned();
    ///     replacement.push(b' ');
    ///     replacement.extend(&caps[1]);
    ///     replacement
    /// });
    /// assert_eq!(result, &b"Bruce Springsteen"[..]);
    /// # }
    /// ```
    ///
    /// But this is a bit cumbersome to use all the time. Instead, a simple
    /// syntax is supported that expands `$name` into the corresponding capture
    /// group. Here's the last example, but using this expansion technique
    /// with named capture groups:
    ///
    /// ```rust
    /// # extern crate regex; use regex::bytes::Regex;
    /// # fn main() {
    /// let re = Regex::new(r"(?P<last>[^,\s]+),\s+(?P<first>\S+)").unwrap();
    /// let result = re.replace(b"Springsteen, Bruce", &b"$first $last"[..]);
    /// assert_eq!(result, &b"Bruce Springsteen"[..]);
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
    /// # extern crate regex; use regex::bytes::Regex;
    /// # fn main() {
    /// let re = Regex::new(r"(?P<first>\w+)\s+(?P<second>\w+)").unwrap();
    /// let result = re.replace(b"deep fried", &b"${first}_$second"[..]);
    /// assert_eq!(result, &b"deep_fried"[..]);
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
    /// # extern crate regex; use regex::bytes::Regex;
    /// # fn main() {
    /// use regex::bytes::NoExpand;
    ///
    /// let re = Regex::new(r"(?P<last>[^,\s]+),\s+(\S+)").unwrap();
    /// let result = re.replace(b"Springsteen, Bruce", NoExpand(b"$2 $last"));
    /// assert_eq!(result, &b"$2 $last"[..]);
    /// # }
    /// ```
    pub fn replace<'t, R: Replacer>(&self, text: &'t [u8], rep: R) -> Cow<'t, [u8]> {
        self.replacen(text, 1, rep)
    }
    /// Replaces all non-overlapping matches in `text` with the replacement
    /// provided. This is the same as calling `replacen` with `limit` set to
    /// `0`.
    ///
    /// See the documentation for `replace` for details on how to access
    /// capturing group matches in the replacement text.
    pub fn replace_all<'t, R: Replacer>(&self, text: &'t [u8], rep: R) -> Cow<'t, [u8]> {
        self.replacen(text, 0, rep)
    }
    /// Replaces at most `limit` non-overlapping matches in `text` with the
    /// replacement provided. If `limit` is 0, then all non-overlapping matches
    /// are replaced.
    ///
    /// See the documentation for `replace` for details on how to access
    /// capturing group matches in the replacement text.
    pub fn replacen<'t, R: Replacer>(
        &self,
        text: &'t [u8],
        limit: usize,
        mut rep: R,
    ) -> Cow<'t, [u8]> {
        if let Some(rep) = rep.no_expansion() {
            let mut it = self.find_iter(text).enumerate().peekable();
            if it.peek().is_none() {
                return Cow::Borrowed(text);
            }
            let mut new = Vec::with_capacity(text.len());
            let mut last_match = 0;
            for (i, m) in it {
                if limit > 0 && i >= limit {
                    break;
                }
                new.extend_from_slice(&text[last_match..m.start()]);
                new.extend_from_slice(&rep);
                last_match = m.end();
            }
            new.extend_from_slice(&text[last_match..]);
            return Cow::Owned(new);
        }
        let mut it = self.captures_iter(text).enumerate().peekable();
        if it.peek().is_none() {
            return Cow::Borrowed(text);
        }
        let mut new = Vec::with_capacity(text.len());
        let mut last_match = 0;
        for (i, cap) in it {
            if limit > 0 && i >= limit {
                break;
            }
            let m = cap.get(0).unwrap();
            new.extend_from_slice(&text[last_match..m.start()]);
            rep.replace_append(&cap, &mut new);
            last_match = m.end();
        }
        new.extend_from_slice(&text[last_match..]);
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
    /// # extern crate regex; use regex::bytes::Regex;
    /// # fn main() {
    /// let text = b"aaaaa";
    /// let pos = Regex::new(r"a+").unwrap().shortest_match(text);
    /// assert_eq!(pos, Some(1));
    /// # }
    /// ```
    pub fn shortest_match(&self, text: &[u8]) -> Option<usize> {
        self.shortest_match_at(text, 0)
    }
    /// Returns the same as shortest_match, but starts the search at the given
    /// offset.
    ///
    /// The significance of the starting point is that it takes the surrounding
    /// context into consideration. For example, the `\A` anchor can only
    /// match when `start == 0`.
    pub fn shortest_match_at(&self, text: &[u8], start: usize) -> Option<usize> {
        self.0.searcher().shortest_match_at(text, start)
    }
    /// Returns the same as is_match, but starts the search at the given
    /// offset.
    ///
    /// The significance of the starting point is that it takes the surrounding
    /// context into consideration. For example, the `\A` anchor can only
    /// match when `start == 0`.
    pub fn is_match_at(&self, text: &[u8], start: usize) -> bool {
        self.shortest_match_at(text, start).is_some()
    }
    /// Returns the same as find, but starts the search at the given
    /// offset.
    ///
    /// The significance of the starting point is that it takes the surrounding
    /// context into consideration. For example, the `\A` anchor can only
    /// match when `start == 0`.
    pub fn find_at<'t>(&self, text: &'t [u8], start: usize) -> Option<Match<'t>> {
        self.0.searcher().find_at(text, start).map(|(s, e)| Match::new(text, s, e))
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
        text: &'t [u8],
    ) -> Option<Match<'t>> {
        self.captures_read_at(locs, text, 0)
    }
    /// Returns the same as `captures_read`, but starts the search at the given
    /// offset and populates the capture locations given.
    ///
    /// The significance of the starting point is that it takes the surrounding
    /// context into consideration. For example, the `\A` anchor can only
    /// match when `start == 0`.
    pub fn captures_read_at<'t>(
        &self,
        locs: &mut CaptureLocations,
        text: &'t [u8],
        start: usize,
    ) -> Option<Match<'t>> {
        self.0
            .searcher()
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
        text: &'t [u8],
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
        CaptureLocations(self.0.searcher().locations())
    }
    /// An alias for `capture_locations` to preserve backward compatibility.
    ///
    /// The `regex-capi` crate uses this method, so to avoid breaking that
    /// crate, we continue to export it as an undocumented API.
    #[doc(hidden)]
    pub fn locations(&self) -> CaptureLocations {
        CaptureLocations(self.0.searcher().locations())
    }
}
/// An iterator over all non-overlapping matches for a particular string.
///
/// The iterator yields a tuple of integers corresponding to the start and end
/// of the match. The indices are byte offsets. The iterator stops when no more
/// matches can be found.
///
/// `'r` is the lifetime of the compiled regular expression and `'t` is the
/// lifetime of the matched byte string.
#[derive(Debug)]
pub struct Matches<'r, 't>(re_trait::Matches<'t, ExecNoSync<'r>>);
impl<'r, 't> Iterator for Matches<'r, 't> {
    type Item = Match<'t>;
    fn next(&mut self) -> Option<Match<'t>> {
        let text = self.0.text();
        self.0.next().map(|(s, e)| Match::new(text, s, e))
    }
}
impl<'r, 't> FusedIterator for Matches<'r, 't> {}
/// An iterator that yields all non-overlapping capture groups matching a
/// particular regular expression.
///
/// The iterator stops when no more matches can be found.
///
/// `'r` is the lifetime of the compiled regular expression and `'t` is the
/// lifetime of the matched byte string.
#[derive(Debug)]
pub struct CaptureMatches<'r, 't>(re_trait::CaptureMatches<'t, ExecNoSync<'r>>);
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
/// Yields all substrings delimited by a regular expression match.
///
/// `'r` is the lifetime of the compiled regular expression and `'t` is the
/// lifetime of the byte string being split.
#[derive(Debug)]
pub struct Split<'r, 't> {
    finder: Matches<'r, 't>,
    last: usize,
}
impl<'r, 't> Iterator for Split<'r, 't> {
    type Item = &'t [u8];
    fn next(&mut self) -> Option<&'t [u8]> {
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
/// lifetime of the byte string being split.
#[derive(Debug)]
pub struct SplitN<'r, 't> {
    splits: Split<'r, 't>,
    n: usize,
}
impl<'r, 't> Iterator for SplitN<'r, 't> {
    type Item = &'t [u8];
    fn next(&mut self) -> Option<&'t [u8]> {
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
/// Captures represents a group of captured byte strings for a single match.
///
/// The 0th capture always corresponds to the entire match. Each subsequent
/// index corresponds to the next capture group in the regex. If a capture
/// group is named, then the matched byte string is *also* available via the
/// `name` method. (Note that the 0th capture is always unnamed and so must be
/// accessed with the `get` method.)
///
/// Positions returned from a capture group are always byte indices.
///
/// `'t` is the lifetime of the matched text.
pub struct Captures<'t> {
    text: &'t [u8],
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
    /// # use regex::bytes::Regex;
    /// let re = Regex::new(r"[a-z]+(?:([0-9]+)|([A-Z]+))").unwrap();
    /// let caps = re.captures(b"abc123").unwrap();
    ///
    /// let text1 = caps.get(1).map_or(&b""[..], |m| m.as_bytes());
    /// let text2 = caps.get(2).map_or(&b""[..], |m| m.as_bytes());
    /// assert_eq!(text1, &b"123"[..]);
    /// assert_eq!(text2, &b""[..]);
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
    /// using braces, any sequence of valid UTF-8 bytes is permitted. If the
    /// sequence does not refer to a capture group name in the corresponding
    /// regex, then it is replaced with an empty string.
    ///
    /// To write a literal `$` use `$$`.
    pub fn expand(&self, replacement: &[u8], dst: &mut Vec<u8>) {
        expand_bytes(self, replacement, dst)
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
        fn escape_bytes(bytes: &[u8]) -> String {
            let mut s = String::new();
            for &b in bytes {
                s.push_str(&escape_byte(b));
            }
            s
        }
        fn escape_byte(byte: u8) -> String {
            use std::ascii::escape_default;
            let escaped: Vec<u8> = escape_default(byte).collect();
            String::from_utf8_lossy(&escaped).into_owned()
        }
        let slot_to_name: HashMap<&usize, &String> = self
            .0
            .named_groups
            .iter()
            .map(|(a, b)| (b, a))
            .collect();
        let mut map = f.debug_map();
        for (slot, m) in self.0.locs.iter().enumerate() {
            let m = m.map(|(s, e)| escape_bytes(&self.0.text[s..e]));
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
    type Output = [u8];
    fn index(&self, i: usize) -> &[u8] {
        self.get(i)
            .map(|m| m.as_bytes())
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
    type Output = [u8];
    fn index<'a>(&'a self, name: &'i str) -> &'a [u8] {
        self.name(name)
            .map(|m| m.as_bytes())
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
/// Replacer describes types that can be used to replace matches in a byte
/// string.
///
/// In general, users of this crate shouldn't need to implement this trait,
/// since implementations are already provided for `&[u8]` and
/// `FnMut(&Captures) -> Vec<u8>` (or any `FnMut(&Captures) -> T`
/// where `T: AsRef<[u8]>`), which covers most use cases.
pub trait Replacer {
    /// Appends text to `dst` to replace the current match.
    ///
    /// The current match is represented by `caps`, which is guaranteed to
    /// have a match at capture group `0`.
    ///
    /// For example, a no-op replacement would be
    /// `dst.extend(&caps[0])`.
    fn replace_append(&mut self, caps: &Captures, dst: &mut Vec<u8>);
    /// Return a fixed unchanging replacement byte string.
    ///
    /// When doing replacements, if access to `Captures` is not needed (e.g.,
    /// the replacement byte string does not need `$` expansion), then it can
    /// be beneficial to avoid finding sub-captures.
    ///
    /// In general, this is called once for every call to `replacen`.
    fn no_expansion<'r>(&'r mut self) -> Option<Cow<'r, [u8]>> {
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
    /// use regex::bytes::{Regex, Replacer};
    ///
    /// fn replace_all_twice<R: Replacer>(
    ///     re: Regex,
    ///     src: &[u8],
    ///     mut rep: R,
    /// ) -> Vec<u8> {
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
    fn replace_append(&mut self, caps: &Captures, dst: &mut Vec<u8>) {
        self.0.replace_append(caps, dst)
    }
    fn no_expansion<'r>(&'r mut self) -> Option<Cow<'r, [u8]>> {
        self.0.no_expansion()
    }
}
impl<'a> Replacer for &'a [u8] {
    fn replace_append(&mut self, caps: &Captures, dst: &mut Vec<u8>) {
        caps.expand(*self, dst);
    }
    fn no_expansion(&mut self) -> Option<Cow<[u8]>> {
        match find_byte(b'$', *self) {
            Some(_) => None,
            None => Some(Cow::Borrowed(*self)),
        }
    }
}
impl<F, T> Replacer for F
where
    F: FnMut(&Captures) -> T,
    T: AsRef<[u8]>,
{
    fn replace_append(&mut self, caps: &Captures, dst: &mut Vec<u8>) {
        dst.extend_from_slice((*self)(caps).as_ref());
    }
}
/// `NoExpand` indicates literal byte string replacement.
///
/// It can be used with `replace` and `replace_all` to do a literal byte string
/// replacement without expanding `$name` to their corresponding capture
/// groups. This can be both convenient (to avoid escaping `$`, for example)
/// and performant (since capture groups don't need to be found).
///
/// `'t` is the lifetime of the literal text.
#[derive(Clone, Debug)]
pub struct NoExpand<'t>(pub &'t [u8]);
impl<'t> Replacer for NoExpand<'t> {
    fn replace_append(&mut self, _: &Captures, dst: &mut Vec<u8>) {
        dst.extend_from_slice(self.0);
    }
    fn no_expansion(&mut self) -> Option<Cow<[u8]>> {
        Some(Cow::Borrowed(self.0))
    }
}
#[cfg(test)]
mod tests_llm_16_101 {
    use super::*;
    use crate::*;
    #[test]
    fn test_size_hint() {
        let _rug_st_tests_llm_16_101_rrrruuuugggg_test_size_hint = 0;
        let names: Vec<Option<String>> = Vec::new();
        let capture_names = CaptureNames(names.iter());
        let (lower, upper) = capture_names.size_hint();
        debug_assert_eq!(lower, 0);
        debug_assert_eq!(upper, Some(0));
        let _rug_ed_tests_llm_16_101_rrrruuuugggg_test_size_hint = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_106 {
    use crate::re_bytes::CapturesDebug;
    use std::fmt::Debug;
    #[test]
    fn test_escape_byte() {
        fn escape_byte(byte: u8) -> String {
            use std::ascii::escape_default;
            let escaped: Vec<u8> = escape_default(byte).collect();
            String::from_utf8_lossy(&escaped).into_owned()
        }
        assert_eq!(escape_byte(b'A'), "\\x41".to_string());
        assert_eq!(escape_byte(b'\n'), "\\n".to_string());
        assert_eq!(escape_byte(b'\r'), "\\r".to_string());
        assert_eq!(escape_byte(b'\t'), "\\t".to_string());
        assert_eq!(escape_byte(b'\x01'), "\\x01".to_string());
        assert_eq!(escape_byte(b'\x7f'), "\\x7f".to_string());
    }
}
#[cfg(test)]
mod tests_llm_16_115 {
    use crate::Regex;
    use std::str::FromStr;
    #[test]
    fn test_from_str_success() {
        let _rug_st_tests_llm_16_115_rrrruuuugggg_test_from_str_success = 0;
        let rug_fuzz_0 = "a*b";
        let regex = <Regex as FromStr>::from_str(rug_fuzz_0).unwrap();
        debug_assert_eq!(regex.as_str(), "a*b");
        let _rug_ed_tests_llm_16_115_rrrruuuugggg_test_from_str_success = 0;
    }
    #[test]
    fn test_from_str_failure() {
        let _rug_st_tests_llm_16_115_rrrruuuugggg_test_from_str_failure = 0;
        let rug_fuzz_0 = "(";
        let result = <Regex as FromStr>::from_str(rug_fuzz_0);
        debug_assert!(result.is_err());
        let _rug_ed_tests_llm_16_115_rrrruuuugggg_test_from_str_failure = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_577 {
    use crate::re_bytes::{Match, Range};
    use std::convert::From;
    #[test]
    fn test_from() {
        let m = Match::new(&[1, 2, 3, 4, 5], 1, 4);
        let range: Range<usize> = From::from(m);
        assert_eq!(range.start, 1);
        assert_eq!(range.end, 4);
    }
}
#[cfg(test)]
mod tests_llm_16_595_llm_16_594 {
    use crate::re_bytes::{Match, Range};
    #[test]
    fn test_as_bytes() {
        let _rug_st_tests_llm_16_595_llm_16_594_rrrruuuugggg_test_as_bytes = 0;
        let rug_fuzz_0 = b"Lorem ipsum dolor sit amet";
        let rug_fuzz_1 = 6;
        let rug_fuzz_2 = 11;
        let rug_fuzz_3 = b"ipsum";
        let haystack = rug_fuzz_0;
        let start = rug_fuzz_1;
        let end = rug_fuzz_2;
        let match_obj = Match::new(haystack, start, end);
        let result = match_obj.as_bytes();
        let expected = rug_fuzz_3;
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_595_llm_16_594_rrrruuuugggg_test_as_bytes = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_597_llm_16_596 {
    use super::*;
    use crate::*;
    use crate::re_bytes::Match;
    #[test]
    fn test_end() {
        let _rug_st_tests_llm_16_597_llm_16_596_rrrruuuugggg_test_end = 0;
        let rug_fuzz_0 = b"abcdef";
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 4;
        let haystack = rug_fuzz_0;
        let match_obj = Match::new(haystack, rug_fuzz_1, rug_fuzz_2);
        debug_assert_eq!(match_obj.end(), 4);
        let _rug_ed_tests_llm_16_597_llm_16_596_rrrruuuugggg_test_end = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_599_llm_16_598 {
    use super::*;
    use crate::*;
    use crate::re_bytes::Match;
    use std::ops::Range;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_599_llm_16_598_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = b"hello world";
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 5;
        let haystack = rug_fuzz_0;
        let start = rug_fuzz_1;
        let end = rug_fuzz_2;
        let match_obj = Match::new(haystack, start, end);
        debug_assert_eq!(match_obj.text, haystack);
        debug_assert_eq!(match_obj.start, start);
        debug_assert_eq!(match_obj.end, end);
        let _rug_ed_tests_llm_16_599_llm_16_598_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_601 {
    use super::*;
    use crate::*;
    use std::ops::Range;
    use crate::re_bytes::Match;
    #[test]
    fn test_range() {
        let _rug_st_tests_llm_16_601_rrrruuuugggg_test_range = 0;
        let rug_fuzz_0 = b"Hello, world!";
        let rug_fuzz_1 = 7;
        let rug_fuzz_2 = 12;
        let haystack: &[u8] = rug_fuzz_0;
        let start: usize = rug_fuzz_1;
        let end: usize = rug_fuzz_2;
        let match_obj: Match<'_> = Match::new(haystack, start, end);
        let expected_range: Range<usize> = start..end;
        debug_assert_eq!(match_obj.range(), expected_range);
        let _rug_ed_tests_llm_16_601_rrrruuuugggg_test_range = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_603 {
    use super::*;
    use crate::*;
    use crate::re_bytes::Match;
    use std::ops::Range;
    #[test]
    fn test_start() {
        let _rug_st_tests_llm_16_603_rrrruuuugggg_test_start = 0;
        let rug_fuzz_0 = b"Lorem ipsum dolor sit amet";
        let rug_fuzz_1 = 6;
        let rug_fuzz_2 = 11;
        let haystack = rug_fuzz_0;
        let start = rug_fuzz_1;
        let end = rug_fuzz_2;
        let m = Match::new(haystack, start, end);
        debug_assert_eq!(m.start(), start);
        let _rug_ed_tests_llm_16_603_rrrruuuugggg_test_start = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_605 {
    use super::*;
    use crate::*;
    #[test]
    fn test_as_str() {
        let _rug_st_tests_llm_16_605_rrrruuuugggg_test_as_str = 0;
        let rug_fuzz_0 = r"(\w+)\s(\w+)";
        let regex = re_bytes::Regex::new(rug_fuzz_0).unwrap();
        debug_assert_eq!(regex.as_str(), r"(\w+)\s(\w+)");
        let _rug_ed_tests_llm_16_605_rrrruuuugggg_test_as_str = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_607_llm_16_606 {
    use super::*;
    use crate::*;
    use crate::{CaptureLocations, Regex};
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::slice;
    use std::sync::Arc;
    struct CachedGuard<T: Send>(RefCell<T>);
    #[derive(Debug)]
    struct Nfa {
        captures: Vec<Option<String>>,
        capture_name_idx: Arc<HashMap<String, usize>>,
    }
    #[derive(Debug)]
    struct Match<'t> {
        text: &'t [u8],
        start: usize,
        end: usize,
    }
    #[derive(Debug)]
    struct Captures<'t> {
        text: &'t [u8],
        locs: Vec<Option<usize>>,
        named_groups: Arc<HashMap<String, usize>>,
    }
    struct Splits<'r, 't> {
        finder: Match<'r>,
        last: usize,
        _phantom: std::marker::PhantomData<&'t [u8]>,
    }
    struct Split<'r, 't> {
        finder: Match<'r>,
        last: usize,
        _phantom: std::marker::PhantomData<&'t [u8]>,
    }
    struct SplitN<'r, 't> {
        splits: Split<'r, 't>,
        n: usize,
    }
    #[test]
    fn test_capture_locations() {
        let _rug_st_tests_llm_16_607_llm_16_606_rrrruuuugggg_test_capture_locations = 0;
        let rug_fuzz_0 = r"\d{4}-\d{2}-\d{2}";
        let regex = Regex::new(rug_fuzz_0).unwrap();
        let capture_locations = regex.capture_locations();
        debug_assert_eq!(capture_locations.len(), 0);
        let _rug_ed_tests_llm_16_607_llm_16_606_rrrruuuugggg_test_capture_locations = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_613 {
    use super::*;
    use crate::*;
    use crate::re_bytes::Regex;
    #[test]
    fn test_captures_len() {
        let _rug_st_tests_llm_16_613_rrrruuuugggg_test_captures_len = 0;
        let rug_fuzz_0 = r"(\w+)\s(\w+)\s(\w+)";
        let re = Regex::new(rug_fuzz_0).unwrap();
        debug_assert_eq!(re.captures_len(), 4);
        let _rug_ed_tests_llm_16_613_rrrruuuugggg_test_captures_len = 0;
    }
    #[test]
    fn test_captures_len_no_captures() {
        let _rug_st_tests_llm_16_613_rrrruuuugggg_test_captures_len_no_captures = 0;
        let rug_fuzz_0 = r"\d+";
        let re = Regex::new(rug_fuzz_0).unwrap();
        debug_assert_eq!(re.captures_len(), 0);
        let _rug_ed_tests_llm_16_613_rrrruuuugggg_test_captures_len_no_captures = 0;
    }
    #[test]
    fn test_captures_len_named_groups() {
        let _rug_st_tests_llm_16_613_rrrruuuugggg_test_captures_len_named_groups = 0;
        let rug_fuzz_0 = r"(?P<name>\w+)\s(?P<age>\d+)\s(?P<city>\w+)";
        let re = Regex::new(rug_fuzz_0).unwrap();
        debug_assert_eq!(re.captures_len(), 4);
        let _rug_ed_tests_llm_16_613_rrrruuuugggg_test_captures_len_named_groups = 0;
    }
    #[test]
    fn test_captures_len_empty_captures() {
        let _rug_st_tests_llm_16_613_rrrruuuugggg_test_captures_len_empty_captures = 0;
        let rug_fuzz_0 = r"(\w+)?\s(\d+)?\s(\w+)?";
        let re = Regex::new(rug_fuzz_0).unwrap();
        debug_assert_eq!(re.captures_len(), 4);
        let _rug_ed_tests_llm_16_613_rrrruuuugggg_test_captures_len_empty_captures = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_617 {
    use super::*;
    use crate::*;
    use crate::re_builder::RegexOptions;
    #[test]
    fn test_captures_read_at() {
        let _rug_st_tests_llm_16_617_rrrruuuugggg_test_captures_read_at = 0;
        let rug_fuzz_0 = r"(\d{4})-(\d{2})-(\d{2})";
        let rug_fuzz_1 = b"2022-01-01";
        let rug_fuzz_2 = 0;
        let regex = crate::re_bytes::Regex::new(rug_fuzz_0).unwrap();
        let mut locs = regex.capture_locations();
        let text = rug_fuzz_1;
        let start = rug_fuzz_2;
        let result = regex.captures_read_at(&mut locs, text, start);
        debug_assert!(result.is_some());
        let _rug_ed_tests_llm_16_617_rrrruuuugggg_test_captures_read_at = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_618 {
    use crate::bytes::Regex;
    #[test]
    fn test_find() {
        let _rug_st_tests_llm_16_618_rrrruuuugggg_test_find = 0;
        let rug_fuzz_0 = b"I categorically deny having triskaidekaphobia.";
        let rug_fuzz_1 = r"\b\w{13}\b";
        let text = rug_fuzz_0;
        let mat = Regex::new(rug_fuzz_1).unwrap().find(text).unwrap();
        debug_assert_eq!((mat.start(), mat.end()), (2, 15));
        let _rug_ed_tests_llm_16_618_rrrruuuugggg_test_find = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_620_llm_16_619 {
    use crate::re_bytes::{Match, Regex};
    #[test]
    fn test_find_at() {
        let _rug_st_tests_llm_16_620_llm_16_619_rrrruuuugggg_test_find_at = 0;
        let rug_fuzz_0 = r"\b\w{13}\b";
        let rug_fuzz_1 = b"I categorically deny having triskaidekaphobia.";
        let rug_fuzz_2 = 0;
        let regex = Regex::new(rug_fuzz_0).unwrap();
        let text = rug_fuzz_1;
        let result = regex.find_at(text, rug_fuzz_2);
        debug_assert_eq!(result, Some(Match::new(text, 2, 15)));
        let _rug_ed_tests_llm_16_620_llm_16_619_rrrruuuugggg_test_find_at = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_621 {
    use crate::re_bytes::Regex;
    #[test]
    fn test_is_match() {
        let _rug_st_tests_llm_16_621_rrrruuuugggg_test_is_match = 0;
        let rug_fuzz_0 = r"\b\w{13}\b";
        let rug_fuzz_1 = b"I categorically deny having triskaidekaphobia.";
        let regex = Regex::new(rug_fuzz_0).unwrap();
        let text = rug_fuzz_1;
        debug_assert_eq!(regex.is_match(text), true);
        let _rug_ed_tests_llm_16_621_rrrruuuugggg_test_is_match = 0;
    }
}
#[test]
fn test_is_match_at() {
    let regex = crate::Regex::new("test").unwrap();
    let text = "this is a test text";
    let start = 10;
    let result = regex.is_match_at(text, start);
    assert_eq!(result, true);
}
#[cfg(test)]
mod tests_llm_16_627 {
    use super::*;
    use crate::*;
    use crate::RegexBuilder;
    #[test]
    fn test_new_valid_regex() {
        let _rug_st_tests_llm_16_627_rrrruuuugggg_test_new_valid_regex = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = "abc";
        let rug_fuzz_2 = "foo|bar";
        debug_assert!(crate ::re_bytes::Regex::new(rug_fuzz_0).is_ok());
        debug_assert!(crate ::re_bytes::Regex::new(rug_fuzz_1).is_ok());
        debug_assert!(crate ::re_bytes::Regex::new(rug_fuzz_2).is_ok());
        let _rug_ed_tests_llm_16_627_rrrruuuugggg_test_new_valid_regex = 0;
    }
    #[test]
    fn test_new_invalid_regex() {
        let _rug_st_tests_llm_16_627_rrrruuuugggg_test_new_invalid_regex = 0;
        let rug_fuzz_0 = "(";
        let rug_fuzz_1 = "abc(";
        let rug_fuzz_2 = "foo|bar(";
        debug_assert!(crate ::re_bytes::Regex::new(rug_fuzz_0).is_err());
        debug_assert!(crate ::re_bytes::Regex::new(rug_fuzz_1).is_err());
        debug_assert!(crate ::re_bytes::Regex::new(rug_fuzz_2).is_err());
        let _rug_ed_tests_llm_16_627_rrrruuuugggg_test_new_invalid_regex = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_628 {
    use super::*;
    use crate::*;
    use crate::bytes::Regex;
    #[test]
    fn test_read_captures_at() {
        let _rug_st_tests_llm_16_628_rrrruuuugggg_test_read_captures_at = 0;
        let rug_fuzz_0 = r"\b\w{13}\b";
        let rug_fuzz_1 = b"I categorically deny having triskaidekaphobia.";
        let rug_fuzz_2 = 0;
        let regex = Regex::new(rug_fuzz_0).unwrap();
        let mut locs = regex.capture_locations();
        let text = rug_fuzz_1;
        let start = rug_fuzz_2;
        let result = regex.read_captures_at(&mut locs, text, start);
        debug_assert!(result.is_some());
        let _rug_ed_tests_llm_16_628_rrrruuuugggg_test_read_captures_at = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_633 {
    use super::*;
    use crate::*;
    use std::str::FromStr;
    use std::clone::Clone;
    use std::fmt::Debug;
    #[test]
    fn test_shortest_match() {
        let _rug_st_tests_llm_16_633_rrrruuuugggg_test_shortest_match = 0;
        let rug_fuzz_0 = b"aaaaa";
        let rug_fuzz_1 = r"a+";
        let text = rug_fuzz_0;
        let pos = re_bytes::Regex::from_str(rug_fuzz_1).unwrap().shortest_match(text);
        debug_assert_eq!(pos, Some(1));
        let _rug_ed_tests_llm_16_633_rrrruuuugggg_test_shortest_match = 0;
    }
}
#[cfg(test)]
mod tests_rug_183 {
    use super::*;
    use bytes::NoExpand;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_183_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"sample data";
        let mut p0 = NoExpand(rug_fuzz_0);
        crate::re_bytes::Replacer::no_expansion(&mut p0);
        let _rug_ed_tests_rug_183_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_184 {
    use super::*;
    use crate::bytes::{Regex, Replacer};
    #[test]
    fn test_by_ref() {
        let _rug_st_tests_rug_184_rrrruuuugggg_test_by_ref = 0;
        let rug_fuzz_0 = r"\d+";
        let rug_fuzz_1 = "12345";
        let rug_fuzz_2 = b"x";
        let mut p0 = Regex::new(rug_fuzz_0).unwrap();
        let src = rug_fuzz_1;
        let mut rep = NoExpand(rug_fuzz_2);
        let dst = p0.replace_all(src.as_bytes(), rep.by_ref());
        let _rug_ed_tests_rug_184_rrrruuuugggg_test_by_ref = 0;
    }
}
#[cfg(test)]
mod tests_rug_184_prepare {
    use crate::bytes::NoExpand;
    #[test]
    fn sample() {
        let _rug_st_tests_rug_184_prepare_rrrruuuugggg_sample = 0;
        let rug_fuzz_0 = b"sample data";
        let mut v59 = NoExpand(rug_fuzz_0);
        let _rug_ed_tests_rug_184_prepare_rrrruuuugggg_sample = 0;
    }
}
#[cfg(test)]
mod tests_rug_186 {
    use super::*;
    use crate::bytes::Regex;
    #[test]
    fn test_find_iter() {
        let _rug_st_tests_rug_186_rrrruuuugggg_test_find_iter = 0;
        let rug_fuzz_0 = r"\b\w{13}\b";
        let rug_fuzz_1 = b"Retroactively relinquishing remunerations is reprehensible.";
        let p0 = Regex::new(rug_fuzz_0).unwrap();
        let p1: &[u8] = rug_fuzz_1;
        p0.find_iter(p1);
        let _rug_ed_tests_rug_186_rrrruuuugggg_test_find_iter = 0;
    }
}
#[cfg(test)]
mod tests_rug_187 {
    use super::*;
    use crate::bytes::Regex;
    #[test]
    fn test_regex_captures() {
        let _rug_st_tests_rug_187_rrrruuuugggg_test_regex_captures = 0;
        let rug_fuzz_0 = r"'([^']+)'\s+\((\d{4})\)";
        let rug_fuzz_1 = b"Not my favorite movie: 'Citizen Kane' (1941).";
        let pattern = rug_fuzz_0;
        let regex = Regex::new(pattern).unwrap();
        let text = rug_fuzz_1;
        regex.captures(text);
        let _rug_ed_tests_rug_187_rrrruuuugggg_test_regex_captures = 0;
    }
}
#[cfg(test)]
mod tests_rug_188 {
    use super::*;
    use crate::bytes::Regex;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_188_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "your-regex-here";
        let rug_fuzz_1 = b"your-sample-data-here";
        let mut p0 = Regex::new(rug_fuzz_0).unwrap();
        let mut p1 = rug_fuzz_1;
        p0.captures_iter(p1);
        let _rug_ed_tests_rug_188_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_189 {
    use super::*;
    use crate::bytes::Regex;
    #[test]
    fn test_regex_split() {
        let _rug_st_tests_rug_189_rrrruuuugggg_test_regex_split = 0;
        let rug_fuzz_0 = r"[ \t]+";
        let rug_fuzz_1 = b"a b \t  c\td    e";
        let mut p0 = Regex::new(rug_fuzz_0).unwrap();
        let p1: &[u8] = rug_fuzz_1;
        p0.split(p1);
        let _rug_ed_tests_rug_189_rrrruuuugggg_test_regex_split = 0;
    }
}
#[cfg(test)]
mod tests_rug_190 {
    use crate::bytes::Regex;
    #[test]
    fn test_splitn() {
        let _rug_st_tests_rug_190_rrrruuuugggg_test_splitn = 0;
        let rug_fuzz_0 = r"\W+";
        let rug_fuzz_1 = b"Hey! How are you?";
        let rug_fuzz_2 = 3;
        let mut p0 = Regex::new(rug_fuzz_0).unwrap();
        let p1: &[u8] = rug_fuzz_1;
        let p2: usize = rug_fuzz_2;
        p0.splitn(p1, p2).collect::<Vec<&[u8]>>();
        let _rug_ed_tests_rug_190_rrrruuuugggg_test_splitn = 0;
    }
}
#[cfg(test)]
mod tests_rug_191 {
    use super::*;
    use crate::bytes::{Regex, NoExpand};
    #[test]
    fn test_regex_replace() {
        let _rug_st_tests_rug_191_rrrruuuugggg_test_regex_replace = 0;
        let rug_fuzz_0 = "[a-z]+";
        let rug_fuzz_1 = b"hello world";
        let rug_fuzz_2 = b"Rust";
        let regex = Regex::new(rug_fuzz_0).unwrap();
        let text = rug_fuzz_1;
        let replacement = NoExpand(rug_fuzz_2);
        let result = regex.replace(text, replacement);
        debug_assert_eq!(result, Cow::Borrowed(b"Rust world"));
        let _rug_ed_tests_rug_191_rrrruuuugggg_test_regex_replace = 0;
    }
}
#[cfg(test)]
mod tests_rug_192 {
    use super::*;
    use crate::Replacer;
    use crate::bytes::{Regex, NoExpand};
    use std::borrow::Cow;
    #[test]
    fn test_replacen() {
        let _rug_st_tests_rug_192_rrrruuuugggg_test_replacen = 0;
        let rug_fuzz_0 = "your-regex-here";
        let rug_fuzz_1 = b"your-text-here";
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = b"your-replacement-here";
        let re = Regex::new(rug_fuzz_0).unwrap();
        let text = rug_fuzz_1;
        let limit = rug_fuzz_2;
        let rep = NoExpand(rug_fuzz_3);
        re.replacen(text, limit, rep);
        let _rug_ed_tests_rug_192_rrrruuuugggg_test_replacen = 0;
    }
}
#[cfg(test)]
mod tests_rug_193 {
    use super::*;
    use re_bytes::Regex;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_193_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "your-regex-here";
        let rug_fuzz_1 = b"your-sample-data-here";
        let rug_fuzz_2 = 0;
        let mut p0 = Regex::new(rug_fuzz_0).unwrap();
        let p1: &[u8] = rug_fuzz_1;
        let p2: usize = rug_fuzz_2;
        p0.shortest_match_at(p1, p2);
        let _rug_ed_tests_rug_193_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_194 {
    use super::*;
    use crate::re_bytes::{Regex, CaptureLocations};
    #[test]
    fn test_regex_captures_read() {
        let _rug_st_tests_rug_194_rrrruuuugggg_test_regex_captures_read = 0;
        let rug_fuzz_0 = "your-regex-here";
        let rug_fuzz_1 = b"your-text-here";
        let regex = Regex::new(rug_fuzz_0).unwrap();
        let mut capture_locations = regex.capture_locations();
        let text = rug_fuzz_1;
        regex.captures_read(&mut capture_locations, text);
        let _rug_ed_tests_rug_194_rrrruuuugggg_test_regex_captures_read = 0;
    }
}
#[cfg(test)]
mod tests_rug_195 {
    use super::*;
    use crate::bytes::Regex;
    #[test]
    fn test_capture_names() {
        let _rug_st_tests_rug_195_rrrruuuugggg_test_capture_names = 0;
        let rug_fuzz_0 = "your-regex-here";
        let mut p0 = Regex::new(rug_fuzz_0).unwrap();
        p0.capture_names();
        let _rug_ed_tests_rug_195_rrrruuuugggg_test_capture_names = 0;
    }
}
#[cfg(test)]
mod tests_rug_196 {
    use super::*;
    use crate::re_bytes::Regex;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_196_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "your-regex-here";
        let mut p0 = Regex::new(rug_fuzz_0).unwrap();
        Regex::locations(&p0);
        let _rug_ed_tests_rug_196_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_198 {
    use super::*;
    use crate::bytes::CaptureMatches;
    use crate::std::iter::Iterator;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_198_rrrruuuugggg_test_rug = 0;
        let mut p0: CaptureMatches<'_, '_> = unimplemented!();
        p0.next();
        let _rug_ed_tests_rug_198_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_199 {
    use super::*;
    use crate::std::iter::Iterator;
    use crate::bytes::{Regex, Split};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_199_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = r"\d+";
        let rug_fuzz_1 = b"abc123def456";
        let re = Regex::new(rug_fuzz_0).unwrap();
        let text = rug_fuzz_1;
        let mut p0: Split = re.split(text);
        p0.next();
        let _rug_ed_tests_rug_199_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_203 {
    use super::*;
    use crate::re_bytes::CaptureNames;
    use crate::std::iter::Iterator;
    #[test]
    fn test_regex() {
        let _rug_st_tests_rug_203_rrrruuuugggg_test_regex = 0;
        let mut p0: CaptureNames<'_> = unimplemented!();
        p0.count();
        let _rug_ed_tests_rug_203_rrrruuuugggg_test_regex = 0;
    }
}
#[cfg(test)]
mod tests_rug_207 {
    use super::*;
    use crate::bytes::{Regex, Captures};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_207_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = r"[a-z]+(?:([0-9]+)|([A-Z]+))";
        let rug_fuzz_1 = b"abc123";
        let rug_fuzz_2 = 1;
        let re = Regex::new(rug_fuzz_0).unwrap();
        let caps: Captures<'_> = re.captures(rug_fuzz_1).unwrap();
        let i: usize = rug_fuzz_2;
        <Captures<'_>>::get(&caps, i);
        let _rug_ed_tests_rug_207_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_208 {
    use super::*;
    use crate::re_bytes::Captures;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_208_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample";
        let mut p0: Captures<'_> = unimplemented!();
        let mut p1: &str = rug_fuzz_0;
        Captures::<'_>::name(&p0, p1);
        let _rug_ed_tests_rug_208_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_218 {
    use super::*;
    use crate::bytes::Replacer;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_218_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"example input";
        let mut p0: &'static [u8] = rug_fuzz_0;
        <&'static [u8] as Replacer>::no_expansion(&mut p0);
        let _rug_ed_tests_rug_218_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_221 {
    use super::*;
    use crate::bytes::Replacer;
    use crate::re_bytes::NoExpand;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_221_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"Hello World";
        let mut p0: NoExpand<'static> = NoExpand(rug_fuzz_0);
        p0.no_expansion();
        let _rug_ed_tests_rug_221_rrrruuuugggg_test_rug = 0;
    }
}
