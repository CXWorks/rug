use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use std::iter::FusedIterator;
use std::ops::{Index, Range};
use std::str::FromStr;
use std::sync::Arc;
use crate::find_byte::find_byte;
use crate::error::Error;
use crate::exec::{Exec, ExecNoSync};
use crate::expand::expand_bytes;
use crate::re_builder::bytes::RegexBuilder;
use crate::re_trait::{self, RegularExpression, SubCapturesPosIter};
/// Match represents a single match of a regex in a haystack.
///
/// The lifetime parameter `'t` refers to the lifetime of the matched text.
#[derive(Copy, Clone, Eq, PartialEq)]
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
    pub fn as_bytes(&self) -> &'t [u8] {
        &self.text[self.range()]
    }
    /// Creates a new match from the given haystack and byte offsets.
    #[inline]
    fn new(haystack: &'t [u8], start: usize, end: usize) -> Match<'t> {
        Match {
            text: haystack,
            start,
            end,
        }
    }
}
impl<'t> std::fmt::Debug for Match<'t> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut fmt = f.debug_struct("Match");
        fmt.field("start", &self.start).field("end", &self.end);
        if let Ok(s) = std::str::from_utf8(self.as_bytes()) {
            fmt.field("bytes", &s);
        } else {
            fmt.field("bytes", &self.as_bytes());
        }
        fmt.finish()
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
    /// # use regex::bytes::Regex;
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
    /// # use regex::bytes::Regex;
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
    /// # use regex::bytes::Regex;
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
    /// # use regex::bytes::Regex;
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
    /// # use regex::bytes::Regex;
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
    /// # use std::str; use regex::bytes::Regex;
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
    /// # use regex::bytes::Regex;
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
    /// # use regex::bytes::Regex;
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
    /// # use regex::bytes::Regex;
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
    /// # use regex::bytes::Regex;
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
    /// # use regex::bytes::Regex;
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
    /// # use regex::bytes::Regex;
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
    /// # use regex::bytes::Regex;
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
                new.extend_from_slice(&text[last_match..m.start()]);
                new.extend_from_slice(&rep);
                last_match = m.end();
                if limit > 0 && i >= limit - 1 {
                    break;
                }
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
            let m = cap.get(0).unwrap();
            new.extend_from_slice(&text[last_match..m.start()]);
            rep.replace_append(&cap, &mut new);
            last_match = m.end();
            if limit > 0 && i >= limit - 1 {
                break;
            }
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
    /// # use regex::bytes::Regex;
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
        self.0.searcher().is_match_at(text, start)
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
    /// Returns the same as [`Regex::captures`], but starts the search at the
    /// given offset.
    ///
    /// The significance of the starting point is that it takes the surrounding
    /// context into consideration. For example, the `\A` anchor can only
    /// match when `start == 0`.
    pub fn captures_at<'t>(&self, text: &'t [u8], start: usize) -> Option<Captures<'t>> {
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
    /// use regex::bytes::Regex;
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
                locs,
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
///
/// # Example
///
/// This example shows how to create and use `CaptureLocations` in a search.
///
/// ```
/// use regex::bytes::Regex;
///
/// let re = Regex::new(r"(?<first>\w+)\s+(?<last>\w+)").unwrap();
/// let mut locs = re.capture_locations();
/// let m = re.captures_read(&mut locs, b"Bruce Springsteen").unwrap();
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
pub struct SubCaptureMatches<'c, 't> {
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
/// since implementations are already provided for `&[u8]` along with other
/// variants of bytes types and `FnMut(&Captures) -> Vec<u8>` (or any
/// `FnMut(&Captures) -> T` where `T: AsRef<[u8]>`), which covers most use cases.
pub trait Replacer {
    /// Appends text to `dst` to replace the current match.
    ///
    /// The current match is represented by `caps`, which is guaranteed to
    /// have a match at capture group `0`.
    ///
    /// For example, a no-op replacement would be
    /// `dst.extend(&caps[0])`.
    fn replace_append(&mut self, caps: &Captures<'_>, dst: &mut Vec<u8>);
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
pub struct ReplacerRef<'a, R: ?Sized>(&'a mut R);
impl<'a, R: Replacer + ?Sized + 'a> Replacer for ReplacerRef<'a, R> {
    fn replace_append(&mut self, caps: &Captures<'_>, dst: &mut Vec<u8>) {
        self.0.replace_append(caps, dst)
    }
    fn no_expansion<'r>(&'r mut self) -> Option<Cow<'r, [u8]>> {
        self.0.no_expansion()
    }
}
impl<'a> Replacer for &'a [u8] {
    fn replace_append(&mut self, caps: &Captures<'_>, dst: &mut Vec<u8>) {
        caps.expand(*self, dst);
    }
    fn no_expansion(&mut self) -> Option<Cow<'_, [u8]>> {
        no_expansion(self)
    }
}
impl<'a> Replacer for &'a Vec<u8> {
    fn replace_append(&mut self, caps: &Captures<'_>, dst: &mut Vec<u8>) {
        caps.expand(*self, dst);
    }
    fn no_expansion(&mut self) -> Option<Cow<'_, [u8]>> {
        no_expansion(self)
    }
}
impl Replacer for Vec<u8> {
    fn replace_append(&mut self, caps: &Captures<'_>, dst: &mut Vec<u8>) {
        caps.expand(self, dst);
    }
    fn no_expansion(&mut self) -> Option<Cow<'_, [u8]>> {
        no_expansion(self)
    }
}
impl<'a> Replacer for Cow<'a, [u8]> {
    fn replace_append(&mut self, caps: &Captures<'_>, dst: &mut Vec<u8>) {
        caps.expand(self.as_ref(), dst);
    }
    fn no_expansion(&mut self) -> Option<Cow<'_, [u8]>> {
        no_expansion(self)
    }
}
impl<'a> Replacer for &'a Cow<'a, [u8]> {
    fn replace_append(&mut self, caps: &Captures<'_>, dst: &mut Vec<u8>) {
        caps.expand(self.as_ref(), dst);
    }
    fn no_expansion(&mut self) -> Option<Cow<'_, [u8]>> {
        no_expansion(self)
    }
}
fn no_expansion<T: AsRef<[u8]>>(t: &T) -> Option<Cow<'_, [u8]>> {
    let s = t.as_ref();
    match find_byte(b'$', s) {
        Some(_) => None,
        None => Some(Cow::Borrowed(s)),
    }
}
impl<F, T> Replacer for F
where
    F: FnMut(&Captures<'_>) -> T,
    T: AsRef<[u8]>,
{
    fn replace_append(&mut self, caps: &Captures<'_>, dst: &mut Vec<u8>) {
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
    fn replace_append(&mut self, _: &Captures<'_>, dst: &mut Vec<u8>) {
        dst.extend_from_slice(self.0);
    }
    fn no_expansion(&mut self) -> Option<Cow<'_, [u8]>> {
        Some(Cow::Borrowed(self.0))
    }
}
#[cfg(test)]
mod tests_rug_260 {
    use super::*;
    use std::borrow::Cow;
    #[test]
    fn test_no_expansion() {
        let _rug_st_tests_rug_260_rrrruuuugggg_test_no_expansion = 0;
        let rug_fuzz_0 = b"Hello, World!";
        let rug_fuzz_1 = b"$Hello, World!";
        let p0: &[u8] = rug_fuzz_0;
        let result = no_expansion(&p0);
        debug_assert_eq!(result, Some(Cow::Borrowed(p0)));
        let p1: &[u8] = rug_fuzz_1;
        let result = no_expansion(&p1);
        debug_assert_eq!(result, None);
        let _rug_ed_tests_rug_260_rrrruuuugggg_test_no_expansion = 0;
    }
}
#[cfg(test)]
mod tests_rug_261 {
    use super::*;
    use crate::re_bytes::Replacer;
    use std::borrow::Cow;
    use std::vec::Vec;
    #[test]
    fn test_no_expansion() {
        let _rug_st_tests_rug_261_rrrruuuugggg_test_no_expansion = 0;
        let mut p0: Vec<u8> = Vec::new();
        Replacer::no_expansion(&mut p0);
        let _rug_ed_tests_rug_261_rrrruuuugggg_test_no_expansion = 0;
    }
}
#[cfg(test)]
mod tests_rug_266 {
    use super::*;
    use crate::re_bytes::Match;
    use crate::bytes::Match as BytesMatch;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_266_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"example";
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 7;
        let p0: BytesMatch<'static> = BytesMatch {
            text: rug_fuzz_0,
            start: rug_fuzz_1,
            end: rug_fuzz_2,
        };
        Match::<'_>::len(&p0);
        let _rug_ed_tests_rug_266_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_269 {
    use super::*;
    use crate::re_bytes::Match;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_269_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"abcdef";
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 4;
        let mut p0: &'static [u8] = rug_fuzz_0;
        let mut p1: usize = rug_fuzz_1;
        let mut p2: usize = rug_fuzz_2;
        Match::<'static>::new(p0, p1, p2);
        let _rug_ed_tests_rug_269_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_273 {
    use super::*;
    use crate::{Regex, Error};
    #[test]
    fn test_regex_new() {
        let _rug_st_tests_rug_273_rrrruuuugggg_test_regex_new = 0;
        let rug_fuzz_0 = "abc";
        let p0: &str = rug_fuzz_0;
        let _ = Regex::new(&p0);
        let _rug_ed_tests_rug_273_rrrruuuugggg_test_regex_new = 0;
    }
}
#[cfg(test)]
mod tests_rug_274 {
    use super::*;
    use crate::bytes::Regex;
    #[test]
    fn test_regex() {
        let _rug_st_tests_rug_274_rrrruuuugggg_test_regex = 0;
        let rug_fuzz_0 = r"\b\w{13}\b";
        let rug_fuzz_1 = b"I categorically deny having triskaidekaphobia.";
        let p0: Regex = Regex::new(rug_fuzz_0).unwrap();
        let p1: &[u8] = rug_fuzz_1;
        let result = p0.is_match(p1);
        debug_assert!(result);
        let _rug_ed_tests_rug_274_rrrruuuugggg_test_regex = 0;
    }
}
#[cfg(test)]
mod tests_rug_275 {
    use super::*;
    use crate::bytes::Regex as BytesRegex;
    #[test]
    fn test_find() {
        let _rug_st_tests_rug_275_rrrruuuugggg_test_find = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = b"sample text";
        let mut p0: BytesRegex = Regex::new(rug_fuzz_0).unwrap();
        let p1: &[u8] = rug_fuzz_1;
        p0.find(p1);
        let _rug_ed_tests_rug_275_rrrruuuugggg_test_find = 0;
    }
}
#[cfg(test)]
mod tests_rug_276 {
    use super::*;
    use crate::bytes::Regex;
    #[test]
    fn test_find_iter() {
        let _rug_st_tests_rug_276_rrrruuuugggg_test_find_iter = 0;
        let rug_fuzz_0 = b"Retroactively relinquishing remunerations is reprehensible.";
        let rug_fuzz_1 = r"\b\w{13}\b";
        let text = rug_fuzz_0;
        let regex = Regex::new(rug_fuzz_1).unwrap();
        let result = regex.find_iter(text);
        let _rug_ed_tests_rug_276_rrrruuuugggg_test_find_iter = 0;
    }
}
#[cfg(test)]
mod tests_rug_277 {
    use super::*;
    use crate::bytes::Regex;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_277_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = b"Not my favorite movie: 'Citizen Kane' (1941).";
        let mut p0: Regex = Regex::new(rug_fuzz_0).unwrap();
        let p1: &[u8] = rug_fuzz_1;
        p0.captures(p1);
        let _rug_ed_tests_rug_277_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_278 {
    use super::*;
    use crate::bytes::Regex as BytesRegex;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_278_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = b"'Citizen Kane' (1941), 'The Wizard of Oz' (1939), 'M' (1931).";
        let mut p0: BytesRegex = BytesRegex::new(rug_fuzz_0).unwrap();
        let p1: &[u8] = rug_fuzz_1;
        p0.captures_iter(p1);
        let _rug_ed_tests_rug_278_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_279 {
    use super::*;
    use crate::bytes::Regex;
    #[test]
    fn test_regex_split() {
        let _rug_st_tests_rug_279_rrrruuuugggg_test_regex_split = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = b"a b \t  c\td    e";
        let mut p0: Regex = Regex::new(rug_fuzz_0).unwrap();
        let p1: &[u8] = rug_fuzz_1;
        p0.split(p1);
        let _rug_ed_tests_rug_279_rrrruuuugggg_test_regex_split = 0;
    }
}
#[cfg(test)]
mod tests_rug_280 {
    use crate::bytes::Regex;
    #[test]
    fn test_splitn() {
        let _rug_st_tests_rug_280_rrrruuuugggg_test_splitn = 0;
        let rug_fuzz_0 = r"\W+";
        let rug_fuzz_1 = b"Hey! How are you?";
        let rug_fuzz_2 = 3;
        let p0 = Regex::new(rug_fuzz_0).unwrap();
        let p1: &[u8] = rug_fuzz_1;
        let p2: usize = rug_fuzz_2;
        p0.splitn(p1, p2);
        let _rug_ed_tests_rug_280_rrrruuuugggg_test_splitn = 0;
    }
}
#[cfg(test)]
mod tests_rug_281 {
    use crate::bytes::{Regex, NoExpand};
    use std::borrow::Cow;
    #[test]
    fn test_regex_replace() {
        let _rug_st_tests_rug_281_rrrruuuugggg_test_regex_replace = 0;
        let rug_fuzz_0 = b"Hello, world!";
        let rug_fuzz_1 = r"world";
        let rug_fuzz_2 = b"Rust";
        let text: &[u8] = rug_fuzz_0;
        let re = Regex::new(rug_fuzz_1).unwrap();
        let rep = NoExpand(&rug_fuzz_2[..]);
        let result = Regex::replace(&re, text, rep);
        debug_assert_eq!(result, Cow::Borrowed(& b"Hello, Rust!"[..]));
        let _rug_ed_tests_rug_281_rrrruuuugggg_test_regex_replace = 0;
    }
}
#[cfg(test)]
mod tests_rug_282 {
    use crate::bytes::Regex;
    use crate::bytes::NoExpand;
    #[test]
    fn test_replace_all() {
        let _rug_st_tests_rug_282_rrrruuuugggg_test_replace_all = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = b"sample text";
        let rug_fuzz_2 = b"replacement text";
        let mut p0 = Regex::new(rug_fuzz_0).unwrap();
        let p1: &[u8] = rug_fuzz_1;
        let mut p2 = NoExpand(rug_fuzz_2);
        p0.replace_all(p1, p2);
        let _rug_ed_tests_rug_282_rrrruuuugggg_test_replace_all = 0;
    }
}
#[cfg(test)]
mod tests_rug_283 {
    use crate::bytes::Regex;
    use std::borrow::Cow;
    #[test]
    fn test_replacen() {
        let _rug_st_tests_rug_283_rrrruuuugggg_test_replacen = 0;
        let rug_fuzz_0 = b"Lorem ipsum dolor sit amet";
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = b"***";
        let rug_fuzz_3 = r"\b\w+\b";
        let text: &[u8] = rug_fuzz_0;
        let limit: usize = rug_fuzz_1;
        let rep: Vec<u8> = rug_fuzz_2.to_vec();
        let regex: Regex = Regex::new(rug_fuzz_3).unwrap();
        let result: Cow<[u8]> = regex.replacen(text, limit, rep);
        debug_assert_eq!(result, Cow::Borrowed(b"*** ipsum *** amet"));
        let _rug_ed_tests_rug_283_rrrruuuugggg_test_replacen = 0;
    }
}
#[cfg(test)]
mod tests_rug_284 {
    use super::*;
    use crate::bytes::Regex;
    #[test]
    fn test_shortest_match() {
        let _rug_st_tests_rug_284_rrrruuuugggg_test_shortest_match = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = b"sample";
        let mut p0: Regex = Regex::new(rug_fuzz_0).unwrap();
        let p1: &[u8] = rug_fuzz_1;
        p0.shortest_match(p1);
        let _rug_ed_tests_rug_284_rrrruuuugggg_test_shortest_match = 0;
    }
}
#[cfg(test)]
mod tests_rug_285 {
    use super::*;
    use crate::bytes::Regex as BytesRegex;
    #[test]
    fn test_shortest_match_at() {
        let _rug_st_tests_rug_285_rrrruuuugggg_test_shortest_match_at = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = b"sample";
        let rug_fuzz_2 = 0;
        let mut p0 = Regex::new(rug_fuzz_0).unwrap();
        let p1 = rug_fuzz_1;
        let p2 = rug_fuzz_2;
        p0.shortest_match_at(p1, p2);
        let _rug_ed_tests_rug_285_rrrruuuugggg_test_shortest_match_at = 0;
    }
}
#[cfg(test)]
mod tests_rug_286 {
    use super::*;
    use crate::bytes::Regex as BytesRegex;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_286_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = b"test_data";
        let rug_fuzz_2 = 0;
        let mut p0: BytesRegex = BytesRegex::new(rug_fuzz_0).unwrap();
        let mut p1: &[u8] = rug_fuzz_1;
        let mut p2: usize = rug_fuzz_2;
        p0.is_match_at(p1, p2);
        let _rug_ed_tests_rug_286_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_287 {
    use super::*;
    use crate::bytes::Regex as BytesRegex;
    #[test]
    fn test_find_at() {
        let _rug_st_tests_rug_287_rrrruuuugggg_test_find_at = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = b"This is a sample text";
        let rug_fuzz_2 = 10;
        let mut p0: BytesRegex = BytesRegex::new(rug_fuzz_0).unwrap();
        let p1: &[u8] = rug_fuzz_1;
        let p2: usize = rug_fuzz_2;
        p0.find_at(p1, p2);
        let _rug_ed_tests_rug_287_rrrruuuugggg_test_find_at = 0;
    }
}
#[cfg(test)]
mod tests_rug_288 {
    use super::*;
    use crate::bytes::Regex;
    #[test]
    fn test_regex_captures_at() {
        let _rug_st_tests_rug_288_rrrruuuugggg_test_regex_captures_at = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = b"some_sample_text";
        let rug_fuzz_2 = 5;
        let p0: Regex = Regex::new(rug_fuzz_0).unwrap();
        let p1: &[u8] = rug_fuzz_1;
        let p2: usize = rug_fuzz_2;
        p0.captures_at(p1, p2);
        let _rug_ed_tests_rug_288_rrrruuuugggg_test_regex_captures_at = 0;
    }
}
#[cfg(test)]
mod tests_rug_289 {
    use super::*;
    use crate::bytes::Regex;
    use crate::bytes::Regex as BytesRegex;
    use crate::bytes::CaptureLocations;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_289_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = b"sample";
        let mut p0: BytesRegex = BytesRegex::new(rug_fuzz_0).unwrap();
        let mut p1: CaptureLocations = p0.capture_locations();
        let mut p2: &[u8] = rug_fuzz_1;
        Regex::captures_read(&p0, &mut p1, &p2);
        let _rug_ed_tests_rug_289_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_290 {
    use super::*;
    use crate::bytes::Regex as BytesRegex;
    use crate::bytes::CaptureLocations;
    #[test]
    fn test_captures_read_at() {
        let _rug_st_tests_rug_290_rrrruuuugggg_test_captures_read_at = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = b"Sample Text";
        let rug_fuzz_2 = 0;
        let regex: BytesRegex = BytesRegex::new(rug_fuzz_0).unwrap();
        let mut capture_locs: CaptureLocations = regex.capture_locations();
        let text: &[u8] = rug_fuzz_1;
        let start_offset: usize = rug_fuzz_2;
        let result = regex.captures_read_at(&mut capture_locs, text, start_offset);
        let _rug_ed_tests_rug_290_rrrruuuugggg_test_captures_read_at = 0;
    }
}
#[cfg(test)]
mod tests_rug_291 {
    use super::*;
    use crate::bytes::{Regex, CaptureLocations};
    #[test]
    fn test_read_captures_at() {
        let _rug_st_tests_rug_291_rrrruuuugggg_test_read_captures_at = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = b"sample";
        let rug_fuzz_2 = 0;
        let regex: Regex = Regex::new(rug_fuzz_0).unwrap();
        let mut locs: CaptureLocations = regex.capture_locations();
        let text: &[u8] = rug_fuzz_1;
        let start: usize = rug_fuzz_2;
        Regex::read_captures_at(&regex, &mut locs, text, start);
        let _rug_ed_tests_rug_291_rrrruuuugggg_test_read_captures_at = 0;
    }
}
#[cfg(test)]
mod tests_rug_292 {
    use super::*;
    use crate::bytes::Regex;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_292_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "";
        let mut p0: Regex = Regex::new(rug_fuzz_0).unwrap();
        crate::re_bytes::Regex::as_str(&p0);
        let _rug_ed_tests_rug_292_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_293 {
    use super::*;
    use crate::bytes::Regex as BytesRegex;
    #[test]
    fn test_capture_names() {
        let _rug_st_tests_rug_293_rrrruuuugggg_test_capture_names = 0;
        let rug_fuzz_0 = "";
        let p0: BytesRegex = BytesRegex::new(rug_fuzz_0).unwrap();
        p0.capture_names();
        let _rug_ed_tests_rug_293_rrrruuuugggg_test_capture_names = 0;
    }
}
#[cfg(test)]
mod tests_rug_294 {
    use super::*;
    use crate::bytes::Regex as BytesRegex;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_294_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "";
        let mut p0: BytesRegex = BytesRegex::new(rug_fuzz_0).unwrap();
        crate::re_bytes::Regex::captures_len(&p0);
        let _rug_ed_tests_rug_294_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_295 {
    use super::*;
    use crate::bytes::Regex;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_295_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "";
        let mut p0: Regex = Regex::new(rug_fuzz_0).unwrap();
        <Regex>::static_captures_len(&p0);
        let _rug_ed_tests_rug_295_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_296 {
    use super::*;
    use crate::bytes::Regex;
    #[test]
    fn test_capture_locations() {
        let _rug_st_tests_rug_296_rrrruuuugggg_test_capture_locations = 0;
        let rug_fuzz_0 = "";
        let v72: Regex = Regex::new(rug_fuzz_0).unwrap();
        let p0 = v72.capture_locations();
        let _rug_ed_tests_rug_296_rrrruuuugggg_test_capture_locations = 0;
    }
}
#[cfg(test)]
mod tests_rug_297 {
    use super::*;
    use crate::bytes::Regex as BytesRegex;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_297_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "";
        let v72: BytesRegex = BytesRegex::new(rug_fuzz_0).unwrap();
        let p0 = v72;
        crate::re_bytes::Regex::locations(&p0);
        let _rug_ed_tests_rug_297_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_309 {
    use super::*;
    use crate::bytes::Captures;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_309_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let mut p0: Captures<'static> = todo!();
        let p1: usize = rug_fuzz_0;
        p0.get(p1);
        let _rug_ed_tests_rug_309_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_310 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_310_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample_name";
        use crate::bytes::Captures;
        let mut p0: Captures<'static> = todo!();
        let p1: &str = rug_fuzz_0;
        p0.name(p1);
        let _rug_ed_tests_rug_310_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_311 {
    use super::*;
    use crate::bytes::Captures;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_311_rrrruuuugggg_test_rug = 0;
        let mut p0: Captures<'static> = todo!();
        p0.iter();
        let _rug_ed_tests_rug_311_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_312 {
    use super::*;
    use crate::bytes::Captures;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_312_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"replacement";
        let mut p0: Captures<'static> = todo!();
        let p1: &[u8] = rug_fuzz_0;
        let mut p2: Vec<u8> = Vec::new();
        crate::re_bytes::Captures::<'static>::expand(&mut p0, p1, &mut p2);
        let _rug_ed_tests_rug_312_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_313 {
    use super::*;
    use crate::bytes::Captures;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_313_rrrruuuugggg_test_rug = 0;
        let mut p0: Captures<'static> = todo!();
        p0.len();
        let _rug_ed_tests_rug_313_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_316 {
    use super::*;
    use crate::bytes::SubCaptureMatches;
    use crate::bytes::Match;
    #[test]
    fn test_regex_next() {
        let _rug_st_tests_rug_316_rrrruuuugggg_test_regex_next = 0;
        let mut v80: SubCaptureMatches<'_, '_> = unimplemented!();
        v80.next();
        let _rug_ed_tests_rug_316_rrrruuuugggg_test_regex_next = 0;
    }
}
#[cfg(test)]
mod tests_rug_320 {
    use super::*;
    use crate::bytes::Replacer;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_320_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"sample data";
        let mut p0: &'static [u8] = rug_fuzz_0;
        <&[u8] as Replacer>::no_expansion(&mut p0);
        let _rug_ed_tests_rug_320_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_321 {
    use super::*;
    use crate::bytes::Replacer;
    use crate::bytes::Captures;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_321_rrrruuuugggg_test_rug = 0;
        let mut p0: std::vec::Vec<u8> = std::vec::Vec::new();
        let p1: Captures<'_> = unimplemented!();
        let mut p2: std::vec::Vec<u8> = std::vec::Vec::new();
        p0.replace_append(&p1, &mut p2);
        let _rug_ed_tests_rug_321_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_322 {
    use super::*;
    use crate::bytes::Replacer;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_322_rrrruuuugggg_test_rug = 0;
        let mut p0: &mut std::vec::Vec<u8> = &mut std::vec::Vec::new();
        p0.no_expansion();
        let _rug_ed_tests_rug_322_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_323 {
    use super::*;
    use crate::bytes::Replacer;
    use crate::expand::expand_bytes;
    use std::vec::Vec;
    #[test]
    fn test_rug() {
        let mut p0: Vec<u8> = Vec::new();
        let p1: Captures<'_> = expand_bytes_Captures();
        let mut p2: Vec<u8> = Vec::new();
        p0.replace_append(&p1, &mut p2);
    }
    #[allow(dead_code)]
    fn expand_bytes_Captures<'a>() -> Captures<'a> {
        unimplemented!()
    }
}
#[cfg(test)]
mod tests_rug_324 {
    use super::*;
    use crate::bytes::Replacer;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_324_rrrruuuugggg_test_rug = 0;
        let mut v27: std::vec::Vec<u8> = std::vec::Vec::new();
        let mut p0: &mut std::vec::Vec<u8> = &mut v27;
        <std::vec::Vec<u8> as crate::bytes::Replacer>::no_expansion(p0);
        let _rug_ed_tests_rug_324_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_326 {
    use super::*;
    use crate::bytes::Replacer;
    use std::borrow::Cow;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_326_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"sample data";
        let mut p0: Cow<'_, [u8]> = Cow::Borrowed(rug_fuzz_0);
        <Cow<'_, [u8]> as Replacer>::no_expansion(&mut p0);
        let _rug_ed_tests_rug_326_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_328 {
    use super::*;
    use crate::bytes::Replacer;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_328_rrrruuuugggg_sample = 0;
        let rug_fuzz_0 = b"sample data";
        #[cfg(test)]
        mod tests_rug_328_prepare {
            use std::borrow::Cow;
            use crate::bytes::Regex;
            #[test]
            fn sample() {
                let _rug_st_tests_rug_328_prepare_rrrruuuugggg_sample = 0;
                let rug_fuzz_0 = 0;
                let rug_fuzz_1 = b"sample data";
                let rug_fuzz_2 = 0;
                let _rug_st_tests_rug_328_rrrruuuugggg_sample = rug_fuzz_0;
                let rug_fuzz_0 = rug_fuzz_1;
                let mut v82: Cow<'static, [u8]> = Cow::Borrowed(rug_fuzz_0);
                let _rug_ed_tests_rug_328_rrrruuuugggg_sample = rug_fuzz_2;
                let _rug_ed_tests_rug_328_prepare_rrrruuuugggg_sample = 0;
            }
        }
        let mut p0: std::borrow::Cow<'static, [u8]> = Cow::Borrowed(b"sample data");
        p0.no_expansion();
        let _rug_ed_tests_rug_328_rrrruuuugggg_sample = 0;
    }
}
#[cfg(test)]
mod tests_rug_331 {
    use super::*;
    use crate::bytes::NoExpand;
    use crate::bytes::Replacer;
    use std::borrow::Cow;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_331_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"Hello";
        let rug_fuzz_1 = true;
        let mut p0: NoExpand<'static> = NoExpand(rug_fuzz_0);
        p0.no_expansion();
        debug_assert!(rug_fuzz_1);
        let _rug_ed_tests_rug_331_rrrruuuugggg_test_rug = 0;
    }
}
