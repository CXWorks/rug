use std::convert::{TryFrom, TryInto};
use super::{Authority, Parts, PathAndQuery, Scheme};
use crate::Uri;
/// A builder for `Uri`s.
///
/// This type can be used to construct an instance of `Uri`
/// through a builder pattern.
#[derive(Debug)]
pub struct Builder {
    parts: Result<Parts, crate::Error>,
}
impl Builder {
    /// Creates a new default instance of `Builder` to construct a `Uri`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let uri = uri::Builder::new()
    ///     .scheme("https")
    ///     .authority("hyper.rs")
    ///     .path_and_query("/")
    ///     .build()
    ///     .unwrap();
    /// ```
    #[inline]
    pub fn new() -> Builder {
        Builder::default()
    }
    /// Set the `Scheme` for this URI.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let mut builder = uri::Builder::new();
    /// builder.scheme("https");
    /// ```
    pub fn scheme<T>(self, scheme: T) -> Self
    where
        Scheme: TryFrom<T>,
        <Scheme as TryFrom<T>>::Error: Into<crate::Error>,
    {
        self.map(move |mut parts| {
            let scheme = scheme.try_into().map_err(Into::into)?;
            parts.scheme = Some(scheme);
            Ok(parts)
        })
    }
    /// Set the `Authority` for this URI.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let uri = uri::Builder::new()
    ///     .authority("tokio.rs")
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn authority<T>(self, auth: T) -> Self
    where
        Authority: TryFrom<T>,
        <Authority as TryFrom<T>>::Error: Into<crate::Error>,
    {
        self.map(move |mut parts| {
            let auth = auth.try_into().map_err(Into::into)?;
            parts.authority = Some(auth);
            Ok(parts)
        })
    }
    /// Set the `PathAndQuery` for this URI.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let uri = uri::Builder::new()
    ///     .path_and_query("/hello?foo=bar")
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn path_and_query<T>(self, p_and_q: T) -> Self
    where
        PathAndQuery: TryFrom<T>,
        <PathAndQuery as TryFrom<T>>::Error: Into<crate::Error>,
    {
        self.map(move |mut parts| {
            let p_and_q = p_and_q.try_into().map_err(Into::into)?;
            parts.path_and_query = Some(p_and_q);
            Ok(parts)
        })
    }
    /// Consumes this builder, and tries to construct a valid `Uri` from
    /// the configured pieces.
    ///
    /// # Errors
    ///
    /// This function may return an error if any previously configured argument
    /// failed to parse or get converted to the internal representation. For
    /// example if an invalid `scheme` was specified via `scheme("!@#%/^")`
    /// the error will be returned when this function is called rather than
    /// when `scheme` was called.
    ///
    /// Additionally, the various forms of URI require certain combinations of
    /// parts to be set to be valid. If the parts don't fit into any of the
    /// valid forms of URI, a new error is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let uri = Uri::builder()
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn build(self) -> Result<Uri, crate::Error> {
        let parts = self.parts?;
        Uri::from_parts(parts).map_err(Into::into)
    }
    fn map<F>(self, func: F) -> Self
    where
        F: FnOnce(Parts) -> Result<Parts, crate::Error>,
    {
        Builder {
            parts: self.parts.and_then(func),
        }
    }
}
impl Default for Builder {
    #[inline]
    fn default() -> Builder {
        Builder {
            parts: Ok(Parts::default()),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn build_from_str() {
        let uri = Builder::new()
            .scheme(Scheme::HTTP)
            .authority("hyper.rs")
            .path_and_query("/foo?a=1")
            .build()
            .unwrap();
        assert_eq!(uri.scheme_str(), Some("http"));
        assert_eq!(uri.authority().unwrap().host(), "hyper.rs");
        assert_eq!(uri.path(), "/foo");
        assert_eq!(uri.query(), Some("a=1"));
    }
    #[test]
    fn build_from_string() {
        for i in 1..10 {
            let uri = Builder::new()
                .path_and_query(format!("/foo?a={}", i))
                .build()
                .unwrap();
            let expected_query = format!("a={}", i);
            assert_eq!(uri.path(), "/foo");
            assert_eq!(uri.query(), Some(expected_query.as_str()));
        }
    }
    #[test]
    fn build_from_string_ref() {
        for i in 1..10 {
            let p_a_q = format!("/foo?a={}", i);
            let uri = Builder::new().path_and_query(&p_a_q).build().unwrap();
            let expected_query = format!("a={}", i);
            assert_eq!(uri.path(), "/foo");
            assert_eq!(uri.query(), Some(expected_query.as_str()));
        }
    }
}
#[cfg(test)]
mod tests_llm_16_581 {
    use crate::{Uri, uri};
    #[test]
    fn test_build() {
        let _rug_st_tests_llm_16_581_rrrruuuugggg_test_build = 0;
        let uri = uri::Builder::new().build().unwrap();
        debug_assert_eq!(uri, Uri::default());
        let _rug_ed_tests_llm_16_581_rrrruuuugggg_test_build = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_582 {
    use crate::uri::builder::Builder;
    #[test]
    fn test_map() {
        let _rug_st_tests_llm_16_582_rrrruuuugggg_test_map = 0;
        let builder = Builder::new();
        let result = builder.map(|parts| { Ok(parts) });
        let _rug_ed_tests_llm_16_582_rrrruuuugggg_test_map = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_583 {
    use crate::uri::builder::Builder;
    #[test]
    fn test_builder_new() {
        let _rug_st_tests_llm_16_583_rrrruuuugggg_test_builder_new = 0;
        let builder = Builder::new();
        let _rug_ed_tests_llm_16_583_rrrruuuugggg_test_builder_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_585 {
    use super::*;
    use crate::*;
    use crate::header::HeaderValue;
    #[test]
    fn test_scheme() {
        let _rug_st_tests_llm_16_585_rrrruuuugggg_test_scheme = 0;
        let rug_fuzz_0 = "https";
        let mut builder = Builder::new();
        builder = builder.scheme(rug_fuzz_0);
        let parts = builder.parts.unwrap();
        debug_assert_eq!(parts.scheme.unwrap(), Scheme::HTTPS);
        let _rug_ed_tests_llm_16_585_rrrruuuugggg_test_scheme = 0;
    }
}
#[cfg(test)]
mod tests_rug_231 {
    use super::*;
    use crate::uri::Builder;
    #[test]
    fn test_path_and_query() {
        let _rug_st_tests_rug_231_rrrruuuugggg_test_path_and_query = 0;
        let rug_fuzz_0 = "/hello?foo=bar";
        let mut p0 = Builder::new();
        let p1: &str = rug_fuzz_0;
        p0.path_and_query(p1);
        let _rug_ed_tests_rug_231_rrrruuuugggg_test_path_and_query = 0;
    }
}
