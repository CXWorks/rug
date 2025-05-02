use std::convert::TryFrom;
use std::str::FromStr;
use std::{cmp, fmt, str};
use bytes::Bytes;
use super::{ErrorKind, InvalidUri};
use crate::byte_str::ByteStr;
/// Represents the path component of a URI
#[derive(Clone)]
pub struct PathAndQuery {
    pub(super) data: ByteStr,
    pub(super) query: u16,
}
const NONE: u16 = ::std::u16::MAX;
impl PathAndQuery {
    pub(super) fn from_shared(mut src: Bytes) -> Result<Self, InvalidUri> {
        let mut query = NONE;
        let mut fragment = None;
        {
            let mut iter = src.as_ref().iter().enumerate();
            for (i, &b) in &mut iter {
                match b {
                    b'?' => {
                        debug_assert_eq!(query, NONE);
                        query = i as u16;
                        break;
                    }
                    b'#' => {
                        fragment = Some(i);
                        break;
                    }
                    0x21
                    | 0x24..=0x3B
                    | 0x3D
                    | 0x40..=0x5F
                    | 0x61..=0x7A
                    | 0x7C
                    | 0x7E => {}
                    _ => return Err(ErrorKind::InvalidUriChar.into()),
                }
            }
            if query != NONE {
                for (i, &b) in iter {
                    match b {
                        0x21 | 0x24..=0x3B | 0x3D | 0x3F..=0x7E => {}
                        b'#' => {
                            fragment = Some(i);
                            break;
                        }
                        _ => return Err(ErrorKind::InvalidUriChar.into()),
                    }
                }
            }
        }
        if let Some(i) = fragment {
            src.truncate(i);
        }
        Ok(PathAndQuery {
            data: unsafe { ByteStr::from_utf8_unchecked(src) },
            query: query,
        })
    }
    /// Convert a `PathAndQuery` from a static string.
    ///
    /// This function will not perform any copying, however the string is
    /// checked to ensure that it is valid.
    ///
    /// # Panics
    ///
    /// This function panics if the argument is an invalid path and query.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::uri::*;
    /// let v = PathAndQuery::from_static("/hello?world");
    ///
    /// assert_eq!(v.path(), "/hello");
    /// assert_eq!(v.query(), Some("world"));
    /// ```
    #[inline]
    pub fn from_static(src: &'static str) -> Self {
        let src = Bytes::from_static(src.as_bytes());
        PathAndQuery::from_shared(src).unwrap()
    }
    /// Attempt to convert a `Bytes` buffer to a `PathAndQuery`.
    ///
    /// This will try to prevent a copy if the type passed is the type used
    /// internally, and will copy the data if it is not.
    pub fn from_maybe_shared<T>(src: T) -> Result<Self, InvalidUri>
    where
        T: AsRef<[u8]> + 'static,
    {
        if_downcast_into!(T, Bytes, src, { return PathAndQuery::from_shared(src); });
        PathAndQuery::try_from(src.as_ref())
    }
    pub(super) fn empty() -> Self {
        PathAndQuery {
            data: ByteStr::new(),
            query: NONE,
        }
    }
    pub(super) fn slash() -> Self {
        PathAndQuery {
            data: ByteStr::from_static("/"),
            query: NONE,
        }
    }
    pub(super) fn star() -> Self {
        PathAndQuery {
            data: ByteStr::from_static("*"),
            query: NONE,
        }
    }
    /// Returns the path component
    ///
    /// The path component is **case sensitive**.
    ///
    /// ```notrust
    /// abc://username:password@example.com:123/path/data?key=value&key2=value2#fragid1
    ///                                        |--------|
    ///                                             |
    ///                                           path
    /// ```
    ///
    /// If the URI is `*` then the path component is equal to `*`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::uri::*;
    ///
    /// let path_and_query: PathAndQuery = "/hello/world".parse().unwrap();
    ///
    /// assert_eq!(path_and_query.path(), "/hello/world");
    /// ```
    #[inline]
    pub fn path(&self) -> &str {
        let ret = if self.query == NONE {
            &self.data[..]
        } else {
            &self.data[..self.query as usize]
        };
        if ret.is_empty() {
            return "/";
        }
        ret
    }
    /// Returns the query string component
    ///
    /// The query component contains non-hierarchical data that, along with data
    /// in the path component, serves to identify a resource within the scope of
    /// the URI's scheme and naming authority (if any). The query component is
    /// indicated by the first question mark ("?") character and terminated by a
    /// number sign ("#") character or by the end of the URI.
    ///
    /// ```notrust
    /// abc://username:password@example.com:123/path/data?key=value&key2=value2#fragid1
    ///                                                   |-------------------|
    ///                                                             |
    ///                                                           query
    /// ```
    ///
    /// # Examples
    ///
    /// With a query string component
    ///
    /// ```
    /// # use http::uri::*;
    /// let path_and_query: PathAndQuery = "/hello/world?key=value&foo=bar".parse().unwrap();
    ///
    /// assert_eq!(path_and_query.query(), Some("key=value&foo=bar"));
    /// ```
    ///
    /// Without a query string component
    ///
    /// ```
    /// # use http::uri::*;
    /// let path_and_query: PathAndQuery = "/hello/world".parse().unwrap();
    ///
    /// assert!(path_and_query.query().is_none());
    /// ```
    #[inline]
    pub fn query(&self) -> Option<&str> {
        if self.query == NONE {
            None
        } else {
            let i = self.query + 1;
            Some(&self.data[i as usize..])
        }
    }
    /// Returns the path and query as a string component.
    ///
    /// # Examples
    ///
    /// With a query string component
    ///
    /// ```
    /// # use http::uri::*;
    /// let path_and_query: PathAndQuery = "/hello/world?key=value&foo=bar".parse().unwrap();
    ///
    /// assert_eq!(path_and_query.as_str(), "/hello/world?key=value&foo=bar");
    /// ```
    ///
    /// Without a query string component
    ///
    /// ```
    /// # use http::uri::*;
    /// let path_and_query: PathAndQuery = "/hello/world".parse().unwrap();
    ///
    /// assert_eq!(path_and_query.as_str(), "/hello/world");
    /// ```
    #[inline]
    pub fn as_str(&self) -> &str {
        let ret = &self.data[..];
        if ret.is_empty() {
            return "/";
        }
        ret
    }
}
impl<'a> TryFrom<&'a [u8]> for PathAndQuery {
    type Error = InvalidUri;
    #[inline]
    fn try_from(s: &'a [u8]) -> Result<Self, Self::Error> {
        PathAndQuery::from_shared(Bytes::copy_from_slice(s))
    }
}
impl<'a> TryFrom<&'a str> for PathAndQuery {
    type Error = InvalidUri;
    #[inline]
    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        TryFrom::try_from(s.as_bytes())
    }
}
impl TryFrom<String> for PathAndQuery {
    type Error = InvalidUri;
    #[inline]
    fn try_from(s: String) -> Result<Self, Self::Error> {
        TryFrom::try_from(s.as_bytes())
    }
}
impl TryFrom<&String> for PathAndQuery {
    type Error = InvalidUri;
    #[inline]
    fn try_from(s: &String) -> Result<Self, Self::Error> {
        TryFrom::try_from(s.as_bytes())
    }
}
impl FromStr for PathAndQuery {
    type Err = InvalidUri;
    #[inline]
    fn from_str(s: &str) -> Result<Self, InvalidUri> {
        TryFrom::try_from(s)
    }
}
impl fmt::Debug for PathAndQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl fmt::Display for PathAndQuery {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.data.is_empty() {
            match self.data.as_bytes()[0] {
                b'/' | b'*' => write!(fmt, "{}", & self.data[..]),
                _ => write!(fmt, "/{}", & self.data[..]),
            }
        } else {
            write!(fmt, "/")
        }
    }
}
impl PartialEq for PathAndQuery {
    #[inline]
    fn eq(&self, other: &PathAndQuery) -> bool {
        self.data == other.data
    }
}
impl Eq for PathAndQuery {}
impl PartialEq<str> for PathAndQuery {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}
impl<'a> PartialEq<PathAndQuery> for &'a str {
    #[inline]
    fn eq(&self, other: &PathAndQuery) -> bool {
        self == &other.as_str()
    }
}
impl<'a> PartialEq<&'a str> for PathAndQuery {
    #[inline]
    fn eq(&self, other: &&'a str) -> bool {
        self.as_str() == *other
    }
}
impl PartialEq<PathAndQuery> for str {
    #[inline]
    fn eq(&self, other: &PathAndQuery) -> bool {
        self == other.as_str()
    }
}
impl PartialEq<String> for PathAndQuery {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other.as_str()
    }
}
impl PartialEq<PathAndQuery> for String {
    #[inline]
    fn eq(&self, other: &PathAndQuery) -> bool {
        self.as_str() == other.as_str()
    }
}
impl PartialOrd for PathAndQuery {
    #[inline]
    fn partial_cmp(&self, other: &PathAndQuery) -> Option<cmp::Ordering> {
        self.as_str().partial_cmp(other.as_str())
    }
}
impl PartialOrd<str> for PathAndQuery {
    #[inline]
    fn partial_cmp(&self, other: &str) -> Option<cmp::Ordering> {
        self.as_str().partial_cmp(other)
    }
}
impl PartialOrd<PathAndQuery> for str {
    #[inline]
    fn partial_cmp(&self, other: &PathAndQuery) -> Option<cmp::Ordering> {
        self.partial_cmp(other.as_str())
    }
}
impl<'a> PartialOrd<&'a str> for PathAndQuery {
    #[inline]
    fn partial_cmp(&self, other: &&'a str) -> Option<cmp::Ordering> {
        self.as_str().partial_cmp(*other)
    }
}
impl<'a> PartialOrd<PathAndQuery> for &'a str {
    #[inline]
    fn partial_cmp(&self, other: &PathAndQuery) -> Option<cmp::Ordering> {
        self.partial_cmp(&other.as_str())
    }
}
impl PartialOrd<String> for PathAndQuery {
    #[inline]
    fn partial_cmp(&self, other: &String) -> Option<cmp::Ordering> {
        self.as_str().partial_cmp(other.as_str())
    }
}
impl PartialOrd<PathAndQuery> for String {
    #[inline]
    fn partial_cmp(&self, other: &PathAndQuery) -> Option<cmp::Ordering> {
        self.as_str().partial_cmp(other.as_str())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn equal_to_self_of_same_path() {
        let p1: PathAndQuery = "/hello/world&foo=bar".parse().unwrap();
        let p2: PathAndQuery = "/hello/world&foo=bar".parse().unwrap();
        assert_eq!(p1, p2);
        assert_eq!(p2, p1);
    }
    #[test]
    fn not_equal_to_self_of_different_path() {
        let p1: PathAndQuery = "/hello/world&foo=bar".parse().unwrap();
        let p2: PathAndQuery = "/world&foo=bar".parse().unwrap();
        assert_ne!(p1, p2);
        assert_ne!(p2, p1);
    }
    #[test]
    fn equates_with_a_str() {
        let path_and_query: PathAndQuery = "/hello/world&foo=bar".parse().unwrap();
        assert_eq!(& path_and_query, "/hello/world&foo=bar");
        assert_eq!("/hello/world&foo=bar", & path_and_query);
        assert_eq!(path_and_query, "/hello/world&foo=bar");
        assert_eq!("/hello/world&foo=bar", path_and_query);
    }
    #[test]
    fn not_equal_with_a_str_of_a_different_path() {
        let path_and_query: PathAndQuery = "/hello/world&foo=bar".parse().unwrap();
        assert_ne!(& path_and_query, "/hello&foo=bar");
        assert_ne!("/hello&foo=bar", & path_and_query);
        assert_ne!(path_and_query, "/hello&foo=bar");
        assert_ne!("/hello&foo=bar", path_and_query);
    }
    #[test]
    fn equates_with_a_string() {
        let path_and_query: PathAndQuery = "/hello/world&foo=bar".parse().unwrap();
        assert_eq!(path_and_query, "/hello/world&foo=bar".to_string());
        assert_eq!("/hello/world&foo=bar".to_string(), path_and_query);
    }
    #[test]
    fn not_equal_with_a_string_of_a_different_path() {
        let path_and_query: PathAndQuery = "/hello/world&foo=bar".parse().unwrap();
        assert_ne!(path_and_query, "/hello&foo=bar".to_string());
        assert_ne!("/hello&foo=bar".to_string(), path_and_query);
    }
    #[test]
    fn compares_to_self() {
        let p1: PathAndQuery = "/a/world&foo=bar".parse().unwrap();
        let p2: PathAndQuery = "/b/world&foo=bar".parse().unwrap();
        assert!(p1 < p2);
        assert!(p2 > p1);
    }
    #[test]
    fn compares_with_a_str() {
        let path_and_query: PathAndQuery = "/b/world&foo=bar".parse().unwrap();
        assert!(& path_and_query < "/c/world&foo=bar");
        assert!("/c/world&foo=bar" > & path_and_query);
        assert!(& path_and_query > "/a/world&foo=bar");
        assert!("/a/world&foo=bar" < & path_and_query);
        assert!(path_and_query < "/c/world&foo=bar");
        assert!("/c/world&foo=bar" > path_and_query);
        assert!(path_and_query > "/a/world&foo=bar");
        assert!("/a/world&foo=bar" < path_and_query);
    }
    #[test]
    fn compares_with_a_string() {
        let path_and_query: PathAndQuery = "/b/world&foo=bar".parse().unwrap();
        assert!(path_and_query < "/c/world&foo=bar".to_string());
        assert!("/c/world&foo=bar".to_string() > path_and_query);
        assert!(path_and_query > "/a/world&foo=bar".to_string());
        assert!("/a/world&foo=bar".to_string() < path_and_query);
    }
    #[test]
    fn ignores_valid_percent_encodings() {
        assert_eq!("/a%20b", pq("/a%20b?r=1").path());
        assert_eq!("qr=%31", pq("/a/b?qr=%31").query().unwrap());
    }
    #[test]
    fn ignores_invalid_percent_encodings() {
        assert_eq!("/a%%b", pq("/a%%b?r=1").path());
        assert_eq!("/aaa%", pq("/aaa%").path());
        assert_eq!("/aaa%", pq("/aaa%?r=1").path());
        assert_eq!("/aa%2", pq("/aa%2").path());
        assert_eq!("/aa%2", pq("/aa%2?r=1").path());
        assert_eq!("qr=%3", pq("/a/b?qr=%3").query().unwrap());
    }
    fn pq(s: &str) -> PathAndQuery {
        s.parse().expect(&format!("parsing {}", s))
    }
}
#[cfg(test)]
mod tests_llm_16_270 {
    use super::*;
    use crate::*;
    use crate::uri::path::PathAndQuery;
    use std::cmp::PartialEq;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_270_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "/hello/world";
        let rug_fuzz_1 = "test";
        let path = PathAndQuery::from_static(rug_fuzz_0);
        let other = rug_fuzz_1;
        debug_assert_eq!(path.eq(other), false);
        let _rug_ed_tests_llm_16_270_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_273 {
    use super::*;
    use crate::*;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_273_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "/test";
        let rug_fuzz_1 = "/test";
        let rug_fuzz_2 = "/hello";
        let path = PathAndQuery::from_static(rug_fuzz_0);
        debug_assert!(path.eq(rug_fuzz_1));
        debug_assert!(! path.eq(rug_fuzz_2));
        let _rug_ed_tests_llm_16_273_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_274 {
    use super::*;
    use crate::*;
    use crate::uri::InvalidUri;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_274_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "/hello";
        let rug_fuzz_1 = "/hello";
        let rug_fuzz_2 = "/world";
        let rug_fuzz_3 = "/hello";
        let rug_fuzz_4 = 5;
        let path1 = PathAndQuery {
            data: ByteStr::from_static(rug_fuzz_0),
            query: NONE,
        };
        let path2 = PathAndQuery {
            data: ByteStr::from_static(rug_fuzz_1),
            query: NONE,
        };
        let path3 = PathAndQuery {
            data: ByteStr::from_static(rug_fuzz_2),
            query: NONE,
        };
        let path4 = PathAndQuery {
            data: ByteStr::from_static(rug_fuzz_3),
            query: rug_fuzz_4,
        };
        debug_assert_eq!(path1.eq(& path2), true);
        debug_assert_eq!(path1.eq(& path3), false);
        debug_assert_eq!(path1.eq(& path4), false);
        let _rug_ed_tests_llm_16_274_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_276 {
    use super::*;
    use crate::*;
    use std::cmp::Ordering;
    use crate::uri::path::PathAndQuery;
    use crate::byte_str::ByteStr;
    use crate::uri::InvalidUri;
    use std::str::FromStr;
    use std::convert::TryFrom;
    #[test]
    fn test_partial_cmp() {
        let _rug_st_tests_llm_16_276_rrrruuuugggg_test_partial_cmp = 0;
        let rug_fuzz_0 = "/abc";
        let rug_fuzz_1 = "/def";
        let path_and_query: PathAndQuery = PathAndQuery::from_static(rug_fuzz_0);
        let other = rug_fuzz_1;
        let result = path_and_query.partial_cmp(&PathAndQuery::from_str(other).unwrap());
        let expected = Some(Ordering::Less);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_276_rrrruuuugggg_test_partial_cmp = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_277 {
    use super::*;
    use crate::*;
    use std::cmp::Ordering;
    #[test]
    fn test_partial_cmp() {
        let _rug_st_tests_llm_16_277_rrrruuuugggg_test_partial_cmp = 0;
        let rug_fuzz_0 = "/hello/world";
        let rug_fuzz_1 = "/hello/universe";
        let path: PathAndQuery = rug_fuzz_0.parse().unwrap();
        let other: String = rug_fuzz_1.to_string();
        let result: Option<Ordering> = path.partial_cmp(&other);
        debug_assert_eq!(result, Some(Ordering::Greater));
        let _rug_ed_tests_llm_16_277_rrrruuuugggg_test_partial_cmp = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_278 {
    use super::*;
    use crate::*;
    use std::cmp::Ordering;
    #[test]
    fn test_partial_cmp() {
        let _rug_st_tests_llm_16_278_rrrruuuugggg_test_partial_cmp = 0;
        let rug_fuzz_0 = "/hello/world";
        let rug_fuzz_1 = "/hello";
        let rug_fuzz_2 = "/hello/world";
        let rug_fuzz_3 = "/hello/world";
        let rug_fuzz_4 = "/hello/worlds";
        let path_a = PathAndQuery::from_static(rug_fuzz_0);
        let path_b = PathAndQuery::from_static(rug_fuzz_1);
        let path_c = PathAndQuery::from_static(rug_fuzz_2);
        let path_d = PathAndQuery::from_static(rug_fuzz_3);
        let path_e = PathAndQuery::from_static(rug_fuzz_4);
        debug_assert_eq!(path_a.partial_cmp(& path_b), Some(Ordering::Greater));
        debug_assert_eq!(path_a.partial_cmp(& path_c), Some(Ordering::Equal));
        debug_assert_eq!(path_a.partial_cmp(& path_d), Some(Ordering::Equal));
        debug_assert_eq!(path_a.partial_cmp(& path_e), Some(Ordering::Less));
        let _rug_ed_tests_llm_16_278_rrrruuuugggg_test_partial_cmp = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_279 {
    use super::*;
    use crate::*;
    use std::cmp::Ordering;
    #[test]
    fn test_partial_cmp() {
        let _rug_st_tests_llm_16_279_rrrruuuugggg_test_partial_cmp = 0;
        let rug_fuzz_0 = "/hello";
        let rug_fuzz_1 = "/world";
        let rug_fuzz_2 = "/hello";
        let rug_fuzz_3 = "/hello";
        let rug_fuzz_4 = 5;
        let rug_fuzz_5 = "/hello";
        let rug_fuzz_6 = 5;
        let rug_fuzz_7 = "/foo";
        let rug_fuzz_8 = 5;
        let rug_fuzz_9 = "/bar";
        let rug_fuzz_10 = 5;
        let path1 = PathAndQuery {
            data: ByteStr::from_static(rug_fuzz_0),
            query: NONE,
        };
        let path2 = PathAndQuery {
            data: ByteStr::from_static(rug_fuzz_1),
            query: NONE,
        };
        let path3 = PathAndQuery {
            data: ByteStr::from_static(rug_fuzz_2),
            query: NONE,
        };
        let path4 = PathAndQuery {
            data: ByteStr::from_static(rug_fuzz_3),
            query: rug_fuzz_4,
        };
        let path5 = PathAndQuery {
            data: ByteStr::from_static(rug_fuzz_5),
            query: rug_fuzz_6,
        };
        let path6 = PathAndQuery {
            data: ByteStr::from_static(rug_fuzz_7),
            query: rug_fuzz_8,
        };
        let path7 = PathAndQuery {
            data: ByteStr::from_static(rug_fuzz_9),
            query: rug_fuzz_10,
        };
        debug_assert_eq!(path1.partial_cmp(& path2), Some(Ordering::Less));
        debug_assert_eq!(path2.partial_cmp(& path1), Some(Ordering::Greater));
        debug_assert_eq!(path1.partial_cmp(& path3), Some(Ordering::Equal));
        debug_assert_eq!(path1.partial_cmp(& path4), Some(Ordering::Equal));
        debug_assert_eq!(path4.partial_cmp(& path5), Some(Ordering::Equal));
        debug_assert_eq!(path4.partial_cmp(& path6), Some(Ordering::Less));
        debug_assert_eq!(path6.partial_cmp(& path4), Some(Ordering::Greater));
        debug_assert_eq!(path6.partial_cmp(& path7), Some(Ordering::Less));
        debug_assert_eq!(path7.partial_cmp(& path6), Some(Ordering::Greater));
        let _rug_ed_tests_llm_16_279_rrrruuuugggg_test_partial_cmp = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_283 {
    use std::convert::TryFrom;
    use crate::uri::path::PathAndQuery;
    #[test]
    fn test_try_from() {
        let _rug_st_tests_llm_16_283_rrrruuuugggg_test_try_from = 0;
        let rug_fuzz_0 = "/path?query";
        let input = rug_fuzz_0;
        let expected = PathAndQuery::try_from(input).unwrap();
        let result = PathAndQuery::try_from(input).unwrap();
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_283_rrrruuuugggg_test_try_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_284 {
    use std::convert::TryInto;
    use std::string::String;
    use crate::uri::path::PathAndQuery;
    #[test]
    fn test_try_from() {
        let _rug_st_tests_llm_16_284_rrrruuuugggg_test_try_from = 0;
        let rug_fuzz_0 = "example";
        let s = rug_fuzz_0.to_string();
        let result: Result<PathAndQuery, _> = TryInto::<PathAndQuery>::try_into(&s);
        debug_assert!(result.is_ok());
        let _rug_ed_tests_llm_16_284_rrrruuuugggg_test_try_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_287 {
    use crate::uri::path::PathAndQuery;
    use std::str::FromStr;
    #[test]
    fn test_from_str() {
        let _rug_st_tests_llm_16_287_rrrruuuugggg_test_from_str = 0;
        let rug_fuzz_0 = "/path?query";
        let s = rug_fuzz_0;
        let result = PathAndQuery::from_str(s);
        debug_assert!(result.is_ok());
        let _rug_ed_tests_llm_16_287_rrrruuuugggg_test_from_str = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_587 {
    use super::*;
    use crate::*;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_587_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "/hello/world";
        let rug_fuzz_1 = "/hello/world";
        let path_and_query = PathAndQuery {
            data: ByteStr::from_static(rug_fuzz_0),
            query: NONE,
        };
        let other = PathAndQuery {
            data: ByteStr::from_static(rug_fuzz_1),
            query: NONE,
        };
        debug_assert!(path_and_query.eq(& other));
        let _rug_ed_tests_llm_16_587_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_588 {
    use super::*;
    use crate::*;
    use std::convert::TryFrom;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_588_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "/hello/world";
        let rug_fuzz_1 = "/hello/world";
        let rug_fuzz_2 = "/hello/world?key=value";
        let path1 = PathAndQuery::try_from(rug_fuzz_0).unwrap();
        let path2 = PathAndQuery::try_from(rug_fuzz_1).unwrap();
        let path3 = PathAndQuery::try_from(rug_fuzz_2).unwrap();
        debug_assert_eq!(path1.eq(& path2), true);
        debug_assert_eq!(path1.eq(& path3), false);
        let _rug_ed_tests_llm_16_588_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_589 {
    use super::*;
    use crate::*;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_589_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "/hello";
        let rug_fuzz_1 = "/hello";
        let path_and_query = PathAndQuery {
            data: ByteStr::from_static(rug_fuzz_0),
            query: NONE,
        };
        let path_and_query2 = PathAndQuery {
            data: ByteStr::from_static(rug_fuzz_1),
            query: NONE,
        };
        debug_assert_eq!(path_and_query.eq(& path_and_query2), true);
        let _rug_ed_tests_llm_16_589_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_590 {
    use super::*;
    use crate::*;
    use std::cmp::Ordering;
    #[test]
    fn test_partial_cmp() {
        let _rug_st_tests_llm_16_590_rrrruuuugggg_test_partial_cmp = 0;
        let rug_fuzz_0 = "/hello/world";
        let rug_fuzz_1 = "/hello/world";
        let rug_fuzz_2 = "/hello/world";
        let rug_fuzz_3 = "/hello";
        let rug_fuzz_4 = "/";
        let rug_fuzz_5 = "/hello";
        let rug_fuzz_6 = "/";
        let rug_fuzz_7 = "/";
        let rug_fuzz_8 = "/hello?world";
        let rug_fuzz_9 = "/hello?query";
        let rug_fuzz_10 = "/hello?world";
        let rug_fuzz_11 = "/hello";
        let rug_fuzz_12 = "/hello";
        let rug_fuzz_13 = "/hello?query";
        let rug_fuzz_14 = "/hello/world";
        let rug_fuzz_15 = "/world/hello";
        let rug_fuzz_16 = "/world/hello";
        let rug_fuzz_17 = "/hello/world";
        let path1 = PathAndQuery::from_static(rug_fuzz_0);
        let path2 = PathAndQuery::from_static(rug_fuzz_1);
        debug_assert_eq!(path1.partial_cmp(& path2), Some(Ordering::Equal));
        let path1 = PathAndQuery::from_static(rug_fuzz_2);
        let path2 = PathAndQuery::from_static(rug_fuzz_3);
        debug_assert_eq!(path1.partial_cmp(& path2), Some(Ordering::Greater));
        let path1 = PathAndQuery::from_static(rug_fuzz_4);
        let path2 = PathAndQuery::from_static(rug_fuzz_5);
        debug_assert_eq!(path1.partial_cmp(& path2), Some(Ordering::Less));
        let path1 = PathAndQuery::from_static(rug_fuzz_6);
        let path2 = PathAndQuery::from_static(rug_fuzz_7);
        debug_assert_eq!(path1.partial_cmp(& path2), Some(Ordering::Equal));
        let path1 = PathAndQuery::from_static(rug_fuzz_8);
        let path2 = PathAndQuery::from_static(rug_fuzz_9);
        debug_assert_eq!(path1.partial_cmp(& path2), Some(Ordering::Equal));
        let path1 = PathAndQuery::from_static(rug_fuzz_10);
        let path2 = PathAndQuery::from_static(rug_fuzz_11);
        debug_assert_eq!(path1.partial_cmp(& path2), Some(Ordering::Greater));
        let path1 = PathAndQuery::from_static(rug_fuzz_12);
        let path2 = PathAndQuery::from_static(rug_fuzz_13);
        debug_assert_eq!(path1.partial_cmp(& path2), Some(Ordering::Less));
        let path1 = PathAndQuery::from_static(rug_fuzz_14);
        let path2 = PathAndQuery::from_static(rug_fuzz_15);
        debug_assert_eq!(path1.partial_cmp(& path2), Some(Ordering::Greater));
        let path1 = PathAndQuery::from_static(rug_fuzz_16);
        let path2 = PathAndQuery::from_static(rug_fuzz_17);
        debug_assert_eq!(path1.partial_cmp(& path2), Some(Ordering::Less));
        let _rug_ed_tests_llm_16_590_rrrruuuugggg_test_partial_cmp = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_591 {
    use super::*;
    use crate::*;
    use std::cmp::Ordering;
    #[test]
    fn test_partial_cmp() {
        let _rug_st_tests_llm_16_591_rrrruuuugggg_test_partial_cmp = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let path1 = PathAndQuery {
            data: ByteStr::new(),
            query: rug_fuzz_0,
        };
        let path2 = PathAndQuery {
            data: ByteStr::new(),
            query: rug_fuzz_1,
        };
        let result = path1.partial_cmp(&path2);
        debug_assert_eq!(result, Some(Ordering::Equal));
        let _rug_ed_tests_llm_16_591_rrrruuuugggg_test_partial_cmp = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_593 {
    use super::*;
    use crate::*;
    use crate::uri::*;
    #[test]
    fn test_as_str_with_query() {
        let _rug_st_tests_llm_16_593_rrrruuuugggg_test_as_str_with_query = 0;
        let rug_fuzz_0 = "/hello/world?key=value&foo=bar";
        let path_and_query: PathAndQuery = rug_fuzz_0.parse().unwrap();
        debug_assert_eq!(path_and_query.as_str(), "/hello/world?key=value&foo=bar");
        let _rug_ed_tests_llm_16_593_rrrruuuugggg_test_as_str_with_query = 0;
    }
    #[test]
    fn test_as_str_without_query() {
        let _rug_st_tests_llm_16_593_rrrruuuugggg_test_as_str_without_query = 0;
        let rug_fuzz_0 = "/hello/world";
        let path_and_query: PathAndQuery = rug_fuzz_0.parse().unwrap();
        debug_assert_eq!(path_and_query.as_str(), "/hello/world");
        let _rug_ed_tests_llm_16_593_rrrruuuugggg_test_as_str_without_query = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_597 {
    use super::*;
    use crate::*;
    use crate::uri::*;
    #[test]
    fn test_from_static() {
        let _rug_st_tests_llm_16_597_rrrruuuugggg_test_from_static = 0;
        let rug_fuzz_0 = "/hello?world";
        let v = PathAndQuery::from_static(rug_fuzz_0);
        debug_assert_eq!(v.path(), "/hello");
        debug_assert_eq!(v.query(), Some("world"));
        let _rug_ed_tests_llm_16_597_rrrruuuugggg_test_from_static = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_599 {
    use super::*;
    use crate::*;
    use bytes::Bytes;
    #[test]
    fn test_query_with_query_string() {
        let _rug_st_tests_llm_16_599_rrrruuuugggg_test_query_with_query_string = 0;
        let rug_fuzz_0 = "/hello/world?key=value&foo=bar";
        let rug_fuzz_1 = 11;
        let path_and_query: PathAndQuery = PathAndQuery {
            data: ByteStr::from_static(rug_fuzz_0),
            query: rug_fuzz_1,
        };
        debug_assert_eq!(path_and_query.query(), Some("key=value&foo=bar"));
        let _rug_ed_tests_llm_16_599_rrrruuuugggg_test_query_with_query_string = 0;
    }
    #[test]
    fn test_query_without_query_string() {
        let _rug_st_tests_llm_16_599_rrrruuuugggg_test_query_without_query_string = 0;
        let rug_fuzz_0 = "/hello/world";
        let path_and_query: PathAndQuery = PathAndQuery {
            data: ByteStr::from_static(rug_fuzz_0),
            query: NONE,
        };
        debug_assert!(path_and_query.query().is_none());
        let _rug_ed_tests_llm_16_599_rrrruuuugggg_test_query_without_query_string = 0;
    }
    #[test]
    fn test_query_with_empty_path() {
        let _rug_st_tests_llm_16_599_rrrruuuugggg_test_query_with_empty_path = 0;
        let rug_fuzz_0 = "";
        let path_and_query: PathAndQuery = PathAndQuery {
            data: ByteStr::from_static(rug_fuzz_0),
            query: NONE,
        };
        debug_assert!(path_and_query.query().is_none());
        let _rug_ed_tests_llm_16_599_rrrruuuugggg_test_query_with_empty_path = 0;
    }
    #[test]
    fn test_query_with_empty_query() {
        let _rug_st_tests_llm_16_599_rrrruuuugggg_test_query_with_empty_query = 0;
        let rug_fuzz_0 = "/hello/world?";
        let rug_fuzz_1 = 11;
        let path_and_query: PathAndQuery = PathAndQuery {
            data: ByteStr::from_static(rug_fuzz_0),
            query: rug_fuzz_1,
        };
        debug_assert_eq!(path_and_query.query(), Some(""));
        let _rug_ed_tests_llm_16_599_rrrruuuugggg_test_query_with_empty_query = 0;
    }
    #[test]
    fn test_query_with_hash() {
        let _rug_st_tests_llm_16_599_rrrruuuugggg_test_query_with_hash = 0;
        let rug_fuzz_0 = "/hello/world?key=hello#value";
        let rug_fuzz_1 = 11;
        let path_and_query: PathAndQuery = PathAndQuery {
            data: ByteStr::from_static(rug_fuzz_0),
            query: rug_fuzz_1,
        };
        debug_assert_eq!(path_and_query.query(), Some("key=hello"));
        let _rug_ed_tests_llm_16_599_rrrruuuugggg_test_query_with_hash = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_600 {
    use super::*;
    use crate::*;
    #[test]
    fn test_slash() {
        let _rug_st_tests_llm_16_600_rrrruuuugggg_test_slash = 0;
        let path = PathAndQuery::slash();
        debug_assert_eq!(path.data, ByteStr::from_static("/"));
        debug_assert_eq!(path.query, NONE);
        let _rug_ed_tests_llm_16_600_rrrruuuugggg_test_slash = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_601 {
    use super::*;
    use crate::*;
    #[test]
    fn test_star() {
        let _rug_st_tests_llm_16_601_rrrruuuugggg_test_star = 0;
        let result = PathAndQuery::star();
        debug_assert_eq!(result.data, ByteStr::from_static("*"));
        debug_assert_eq!(result.query, NONE);
        let _rug_ed_tests_llm_16_601_rrrruuuugggg_test_star = 0;
    }
}
#[cfg(test)]
mod tests_rug_233 {
    use super::*;
    use bytes::Bytes;
    #[test]
    fn test_from_shared() {
        let _rug_st_tests_rug_233_rrrruuuugggg_test_from_shared = 0;
        let rug_fuzz_0 = "example";
        let mut v96: Bytes = Bytes::from(rug_fuzz_0.as_bytes());
        let p0: Bytes = v96.clone();
        PathAndQuery::from_shared(p0);
        let _rug_ed_tests_rug_233_rrrruuuugggg_test_from_shared = 0;
    }
}
#[cfg(test)]
mod tests_rug_234 {
    use super::*;
    use crate::header::HeaderValue;
    #[test]
    fn test_from_maybe_shared() {
        let _rug_st_tests_rug_234_rrrruuuugggg_test_from_maybe_shared = 0;
        let rug_fuzz_0 = "hello";
        let mut p0: HeaderValue = HeaderValue::from_static(rug_fuzz_0);
        crate::uri::path::PathAndQuery::from_maybe_shared(p0);
        let _rug_ed_tests_rug_234_rrrruuuugggg_test_from_maybe_shared = 0;
    }
}
#[cfg(test)]
mod tests_rug_235 {
    use super::*;
    #[test]
    fn test_empty() {
        let _rug_st_tests_rug_235_rrrruuuugggg_test_empty = 0;
        PathAndQuery::empty();
        let _rug_ed_tests_rug_235_rrrruuuugggg_test_empty = 0;
    }
}
#[cfg(test)]
mod tests_rug_236 {
    use super::*;
    use crate::uri::PathAndQuery;
    #[test]
    fn test_path() {
        let _rug_st_tests_rug_236_rrrruuuugggg_test_path = 0;
        let rug_fuzz_0 = "/hello/world";
        let mut p0: PathAndQuery = PathAndQuery::from_static(rug_fuzz_0);
        debug_assert_eq!(p0.path(), "/hello/world");
        let _rug_ed_tests_rug_236_rrrruuuugggg_test_path = 0;
    }
}
