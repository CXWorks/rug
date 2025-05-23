use crate::parser::{self, to_u32, SchemeType};
use crate::Url;
use std::str;
/// Exposes methods to manipulate the path of an URL that is not cannot-be-base.
///
/// The path always starts with a `/` slash, and is made of slash-separated segments.
/// There is always at least one segment (which may be the empty string).
///
/// Examples:
///
/// ```rust
/// use url::Url;
/// # use std::error::Error;
///
/// # fn run() -> Result<(), Box<dyn Error>> {
/// let mut url = Url::parse("mailto:me@example.com")?;
/// assert!(url.path_segments_mut().is_err());
///
/// let mut url = Url::parse("http://example.net/foo/index.html")?;
/// url.path_segments_mut().map_err(|_| "cannot be base")?
///     .pop().push("img").push("2/100%.png");
/// assert_eq!(url.as_str(), "http://example.net/foo/img/2%2F100%25.png");
/// # Ok(())
/// # }
/// # run().unwrap();
/// ```
#[derive(Debug)]
pub struct PathSegmentsMut<'a> {
    url: &'a mut Url,
    after_first_slash: usize,
    after_path: String,
    old_after_path_position: u32,
}
pub fn new(url: &mut Url) -> PathSegmentsMut<'_> {
    let after_path = url.take_after_path();
    let old_after_path_position = to_u32(url.serialization.len()).unwrap();
    if SchemeType::from(url.scheme()).is_special() {
        debug_assert!(url.byte_at(url.path_start) == b'/');
    } else {
        debug_assert!(
            url.serialization.len() == url.path_start as usize || url.byte_at(url
            .path_start) == b'/'
        );
    }
    PathSegmentsMut {
        after_first_slash: url.path_start as usize + "/".len(),
        url,
        old_after_path_position,
        after_path,
    }
}
impl<'a> Drop for PathSegmentsMut<'a> {
    fn drop(&mut self) {
        self.url.restore_after_path(self.old_after_path_position, &self.after_path)
    }
}
impl<'a> PathSegmentsMut<'a> {
    /// Remove all segments in the path, leaving the minimal `url.path() == "/"`.
    ///
    /// Returns `&mut Self` so that method calls can be chained.
    ///
    /// Example:
    ///
    /// ```rust
    /// use url::Url;
    /// # use std::error::Error;
    ///
    /// # fn run() -> Result<(), Box<dyn Error>> {
    /// let mut url = Url::parse("https://github.com/servo/rust-url/")?;
    /// url.path_segments_mut().map_err(|_| "cannot be base")?
    ///     .clear().push("logout");
    /// assert_eq!(url.as_str(), "https://github.com/logout");
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    pub fn clear(&mut self) -> &mut Self {
        self.url.serialization.truncate(self.after_first_slash);
        self
    }
    /// Remove the last segment of this URL’s path if it is empty,
    /// except if these was only one segment to begin with.
    ///
    /// In other words, remove one path trailing slash, if any,
    /// unless it is also the initial slash (so this does nothing if `url.path() == "/")`.
    ///
    /// Returns `&mut Self` so that method calls can be chained.
    ///
    /// Example:
    ///
    /// ```rust
    /// use url::Url;
    /// # use std::error::Error;
    ///
    /// # fn run() -> Result<(), Box<dyn Error>> {
    /// let mut url = Url::parse("https://github.com/servo/rust-url/")?;
    /// url.path_segments_mut().map_err(|_| "cannot be base")?
    ///     .push("pulls");
    /// assert_eq!(url.as_str(), "https://github.com/servo/rust-url//pulls");
    ///
    /// let mut url = Url::parse("https://github.com/servo/rust-url/")?;
    /// url.path_segments_mut().map_err(|_| "cannot be base")?
    ///     .pop_if_empty().push("pulls");
    /// assert_eq!(url.as_str(), "https://github.com/servo/rust-url/pulls");
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    pub fn pop_if_empty(&mut self) -> &mut Self {
        if self.url.serialization[self.after_first_slash..].ends_with('/') {
            self.url.serialization.pop();
        }
        self
    }
    /// Remove the last segment of this URL’s path.
    ///
    /// If the path only has one segment, make it empty such that `url.path() == "/"`.
    ///
    /// Returns `&mut Self` so that method calls can be chained.
    pub fn pop(&mut self) -> &mut Self {
        let last_slash = self
            .url
            .serialization[self.after_first_slash..]
            .rfind('/')
            .unwrap_or(0);
        self.url.serialization.truncate(self.after_first_slash + last_slash);
        self
    }
    /// Append the given segment at the end of this URL’s path.
    ///
    /// See the documentation for `.extend()`.
    ///
    /// Returns `&mut Self` so that method calls can be chained.
    pub fn push(&mut self, segment: &str) -> &mut Self {
        self.extend(Some(segment))
    }
    /// Append each segment from the given iterator at the end of this URL’s path.
    ///
    /// Each segment is percent-encoded like in `Url::parse` or `Url::join`,
    /// except that `%` and `/` characters are also encoded (to `%25` and `%2F`).
    /// This is unlike `Url::parse` where `%` is left as-is in case some of the input
    /// is already percent-encoded, and `/` denotes a path segment separator.)
    ///
    /// Note that, in addition to slashes between new segments,
    /// this always adds a slash between the existing path and the new segments
    /// *except* if the existing path is `"/"`.
    /// If the previous last segment was empty (if the path had a trailing slash)
    /// the path after `.extend()` will contain two consecutive slashes.
    /// If that is undesired, call `.pop_if_empty()` first.
    ///
    /// To obtain a behavior similar to `Url::join`, call `.pop()` unconditionally first.
    ///
    /// Returns `&mut Self` so that method calls can be chained.
    ///
    /// Example:
    ///
    /// ```rust
    /// use url::Url;
    /// # use std::error::Error;
    ///
    /// # fn run() -> Result<(), Box<dyn Error>> {
    /// let mut url = Url::parse("https://github.com/")?;
    /// let org = "servo";
    /// let repo = "rust-url";
    /// let issue_number = "188";
    /// url.path_segments_mut().map_err(|_| "cannot be base")?
    ///     .extend(&[org, repo, "issues", issue_number]);
    /// assert_eq!(url.as_str(), "https://github.com/servo/rust-url/issues/188");
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    ///
    /// In order to make sure that parsing the serialization of an URL gives the same URL,
    /// a segment is ignored if it is `"."` or `".."`:
    ///
    /// ```rust
    /// use url::Url;
    /// # use std::error::Error;
    ///
    /// # fn run() -> Result<(), Box<dyn Error>> {
    /// let mut url = Url::parse("https://github.com/servo")?;
    /// url.path_segments_mut().map_err(|_| "cannot be base")?
    ///     .extend(&["..", "rust-url", ".", "pulls"]);
    /// assert_eq!(url.as_str(), "https://github.com/servo/rust-url/pulls");
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    pub fn extend<I>(&mut self, segments: I) -> &mut Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let scheme_type = SchemeType::from(self.url.scheme());
        let path_start = self.url.path_start as usize;
        self.url
            .mutate(|parser| {
                parser.context = parser::Context::PathSegmentSetter;
                for segment in segments {
                    let segment = segment.as_ref();
                    if matches!(segment, "." | "..") {
                        continue;
                    }
                    if parser.serialization.len() > path_start + 1
                        || parser.serialization.len() == path_start
                    {
                        parser.serialization.push('/');
                    }
                    let mut has_host = true;
                    parser
                        .parse_path(
                            scheme_type,
                            &mut has_host,
                            path_start,
                            parser::Input::new(segment),
                        );
                }
            });
        self
    }
}
#[cfg(test)]
mod tests_rug_37 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_new() {
        let _rug_st_tests_rug_37_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = "https://www.example.com/path/to/resource?param=value#fragment";
        let mut url = Url::parse(rug_fuzz_0).unwrap();
        crate::path_segments::new(&mut url);
        let _rug_ed_tests_rug_37_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_rug_39 {
    use super::*;
    use crate::Url;
    use std::error::Error;
    #[test]
    fn test_clear() {
        let _rug_st_tests_rug_39_rrrruuuugggg_test_clear = 0;
        let rug_fuzz_0 = "https://github.com/servo/rust-url/";
        let rug_fuzz_1 = "cannot be base";
        let rug_fuzz_2 = "logout";
        let mut url = Url::parse(rug_fuzz_0).unwrap();
        url.path_segments_mut()
            .map_err(|_| rug_fuzz_1)
            .unwrap()
            .clear()
            .push(rug_fuzz_2);
        debug_assert_eq!(url.as_str(), "https://github.com/logout");
        let _rug_ed_tests_rug_39_rrrruuuugggg_test_clear = 0;
    }
}
#[cfg(test)]
mod tests_rug_41 {
    use super::*;
    use crate::{Url, path_segments::PathSegmentsMut};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_41_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https://example.com/foo/bar/baz";
        let mut url = Url::parse(rug_fuzz_0).unwrap();
        let mut p0: PathSegmentsMut = url.path_segments_mut().unwrap();
        p0.pop();
        let _rug_ed_tests_rug_41_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_43 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_extend() {
        let _rug_st_tests_rug_43_rrrruuuugggg_test_extend = 0;
        let rug_fuzz_0 = "https://github.com/";
        let rug_fuzz_1 = "servo";
        let rug_fuzz_2 = "rust-url";
        let rug_fuzz_3 = "issues";
        let rug_fuzz_4 = "188";
        let mut url = Url::parse(rug_fuzz_0).unwrap();
        let segments = &[rug_fuzz_1, rug_fuzz_2, rug_fuzz_3, rug_fuzz_4];
        url.path_segments_mut().unwrap().extend(segments);
        debug_assert_eq!(url.as_str(), "https://github.com/servo/rust-url/issues/188");
        let _rug_ed_tests_rug_43_rrrruuuugggg_test_extend = 0;
    }
    #[test]
    fn test_extend_ignore_segments() {
        let _rug_st_tests_rug_43_rrrruuuugggg_test_extend_ignore_segments = 0;
        let rug_fuzz_0 = "https://github.com/servo";
        let rug_fuzz_1 = "..";
        let rug_fuzz_2 = "rust-url";
        let rug_fuzz_3 = ".";
        let rug_fuzz_4 = "pulls";
        let mut url = Url::parse(rug_fuzz_0).unwrap();
        let segments = &[rug_fuzz_1, rug_fuzz_2, rug_fuzz_3, rug_fuzz_4];
        url.path_segments_mut().unwrap().extend(segments);
        debug_assert_eq!(url.as_str(), "https://github.com/servo/rust-url/pulls");
        let _rug_ed_tests_rug_43_rrrruuuugggg_test_extend_ignore_segments = 0;
    }
}
