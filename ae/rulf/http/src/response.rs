//! HTTP response types.
//!
//! This module contains structs related to HTTP responses, notably the
//! `Response` type itself as well as a builder to create responses. Typically
//! you'll import the `http::Response` type rather than reaching into this
//! module itself.
//!
//! # Examples
//!
//! Creating a `Response` to return
//!
//! ```
//! use http::{Request, Response, StatusCode};
//!
//! fn respond_to(req: Request<()>) -> http::Result<Response<()>> {
//!     let mut builder = Response::builder()
//!         .header("Foo", "Bar")
//!         .status(StatusCode::OK);
//!
//!     if req.headers().contains_key("Another-Header") {
//!         builder = builder.header("Another-Header", "Ack");
//!     }
//!
//!     builder.body(())
//! }
//! ```
//!
//! A simple 404 handler
//!
//! ```
//! use http::{Request, Response, StatusCode};
//!
//! fn not_found(_req: Request<()>) -> http::Result<Response<()>> {
//!     Response::builder()
//!         .status(StatusCode::NOT_FOUND)
//!         .body(())
//! }
//! ```
//!
//! Or otherwise inspecting the result of a request:
//!
//! ```no_run
//! use http::{Request, Response};
//!
//! fn get(url: &str) -> http::Result<Response<()>> {
//!     // ...
//! # panic!()
//! }
//!
//! let response = get("https://www.rust-lang.org/").unwrap();
//!
//! if !response.status().is_success() {
//!     panic!("failed to get a successful response status!");
//! }
//!
//! if let Some(date) = response.headers().get("Date") {
//!     // we've got a `Date` header!
//! }
//!
//! let body = response.body();
//! // ...
//! ```
use std::any::Any;
use std::convert::TryFrom;
use std::fmt;
use crate::header::{HeaderMap, HeaderName, HeaderValue};
use crate::status::StatusCode;
use crate::version::Version;
use crate::{Extensions, Result};
/// Represents an HTTP response
///
/// An HTTP response consists of a head and a potentially optional body. The body
/// component is generic, enabling arbitrary types to represent the HTTP body.
/// For example, the body could be `Vec<u8>`, a `Stream` of byte chunks, or a
/// value that has been deserialized.
///
/// Typically you'll work with responses on the client side as the result of
/// sending a `Request` and on the server you'll be generating a `Response` to
/// send back to the client.
///
/// # Examples
///
/// Creating a `Response` to return
///
/// ```
/// use http::{Request, Response, StatusCode};
///
/// fn respond_to(req: Request<()>) -> http::Result<Response<()>> {
///     let mut builder = Response::builder()
///         .header("Foo", "Bar")
///         .status(StatusCode::OK);
///
///     if req.headers().contains_key("Another-Header") {
///         builder = builder.header("Another-Header", "Ack");
///     }
///
///     builder.body(())
/// }
/// ```
///
/// A simple 404 handler
///
/// ```
/// use http::{Request, Response, StatusCode};
///
/// fn not_found(_req: Request<()>) -> http::Result<Response<()>> {
///     Response::builder()
///         .status(StatusCode::NOT_FOUND)
///         .body(())
/// }
/// ```
///
/// Or otherwise inspecting the result of a request:
///
/// ```no_run
/// use http::{Request, Response};
///
/// fn get(url: &str) -> http::Result<Response<()>> {
///     // ...
/// # panic!()
/// }
///
/// let response = get("https://www.rust-lang.org/").unwrap();
///
/// if !response.status().is_success() {
///     panic!("failed to get a successful response status!");
/// }
///
/// if let Some(date) = response.headers().get("Date") {
///     // we've got a `Date` header!
/// }
///
/// let body = response.body();
/// // ...
/// ```
///
/// Deserialize a response of bytes via json:
///
/// ```
/// # extern crate serde;
/// # extern crate serde_json;
/// # extern crate http;
/// use http::Response;
/// use serde::de;
///
/// fn deserialize<T>(req: Response<Vec<u8>>) -> serde_json::Result<Response<T>>
///     where for<'de> T: de::Deserialize<'de>,
/// {
///     let (parts, body) = req.into_parts();
///     let body = serde_json::from_slice(&body)?;
///     Ok(Response::from_parts(parts, body))
/// }
/// #
/// # fn main() {}
/// ```
///
/// Or alternatively, serialize the body of a response to json
///
/// ```
/// # extern crate serde;
/// # extern crate serde_json;
/// # extern crate http;
/// use http::Response;
/// use serde::ser;
///
/// fn serialize<T>(req: Response<T>) -> serde_json::Result<Response<Vec<u8>>>
///     where T: ser::Serialize,
/// {
///     let (parts, body) = req.into_parts();
///     let body = serde_json::to_vec(&body)?;
///     Ok(Response::from_parts(parts, body))
/// }
/// #
/// # fn main() {}
/// ```
pub struct Response<T> {
    head: Parts,
    body: T,
}
/// Component parts of an HTTP `Response`
///
/// The HTTP response head consists of a status, version, and a set of
/// header fields.
pub struct Parts {
    /// The response's status
    pub status: StatusCode,
    /// The response's version
    pub version: Version,
    /// The response's headers
    pub headers: HeaderMap<HeaderValue>,
    /// The response's extensions
    pub extensions: Extensions,
    _priv: (),
}
/// An HTTP response builder
///
/// This type can be used to construct an instance of `Response` through a
/// builder-like pattern.
#[derive(Debug)]
pub struct Builder {
    inner: Result<Parts>,
}
impl Response<()> {
    /// Creates a new builder-style object to manufacture a `Response`
    ///
    /// This method returns an instance of `Builder` which can be used to
    /// create a `Response`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let response = Response::builder()
    ///     .status(200)
    ///     .header("X-Custom-Foo", "Bar")
    ///     .body(())
    ///     .unwrap();
    /// ```
    #[inline]
    pub fn builder() -> Builder {
        Builder::new()
    }
}
impl<T> Response<T> {
    /// Creates a new blank `Response` with the body
    ///
    /// The component ports of this response will be set to their default, e.g.
    /// the ok status, no headers, etc.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let response = Response::new("hello world");
    ///
    /// assert_eq!(response.status(), StatusCode::OK);
    /// assert_eq!(*response.body(), "hello world");
    /// ```
    #[inline]
    pub fn new(body: T) -> Response<T> {
        Response {
            head: Parts::new(),
            body: body,
        }
    }
    /// Creates a new `Response` with the given head and body
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let response = Response::new("hello world");
    /// let (mut parts, body) = response.into_parts();
    ///
    /// parts.status = StatusCode::BAD_REQUEST;
    /// let response = Response::from_parts(parts, body);
    ///
    /// assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    /// assert_eq!(*response.body(), "hello world");
    /// ```
    #[inline]
    pub fn from_parts(parts: Parts, body: T) -> Response<T> {
        Response {
            head: parts,
            body: body,
        }
    }
    /// Returns the `StatusCode`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let response: Response<()> = Response::default();
    /// assert_eq!(response.status(), StatusCode::OK);
    /// ```
    #[inline]
    pub fn status(&self) -> StatusCode {
        self.head.status
    }
    /// Returns a mutable reference to the associated `StatusCode`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let mut response: Response<()> = Response::default();
    /// *response.status_mut() = StatusCode::CREATED;
    /// assert_eq!(response.status(), StatusCode::CREATED);
    /// ```
    #[inline]
    pub fn status_mut(&mut self) -> &mut StatusCode {
        &mut self.head.status
    }
    /// Returns a reference to the associated version.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let response: Response<()> = Response::default();
    /// assert_eq!(response.version(), Version::HTTP_11);
    /// ```
    #[inline]
    pub fn version(&self) -> Version {
        self.head.version
    }
    /// Returns a mutable reference to the associated version.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let mut response: Response<()> = Response::default();
    /// *response.version_mut() = Version::HTTP_2;
    /// assert_eq!(response.version(), Version::HTTP_2);
    /// ```
    #[inline]
    pub fn version_mut(&mut self) -> &mut Version {
        &mut self.head.version
    }
    /// Returns a reference to the associated header field map.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let response: Response<()> = Response::default();
    /// assert!(response.headers().is_empty());
    /// ```
    #[inline]
    pub fn headers(&self) -> &HeaderMap<HeaderValue> {
        &self.head.headers
    }
    /// Returns a mutable reference to the associated header field map.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// # use http::header::*;
    /// let mut response: Response<()> = Response::default();
    /// response.headers_mut().insert(HOST, HeaderValue::from_static("world"));
    /// assert!(!response.headers().is_empty());
    /// ```
    #[inline]
    pub fn headers_mut(&mut self) -> &mut HeaderMap<HeaderValue> {
        &mut self.head.headers
    }
    /// Returns a reference to the associated extensions.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let response: Response<()> = Response::default();
    /// assert!(response.extensions().get::<i32>().is_none());
    /// ```
    #[inline]
    pub fn extensions(&self) -> &Extensions {
        &self.head.extensions
    }
    /// Returns a mutable reference to the associated extensions.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// # use http::header::*;
    /// let mut response: Response<()> = Response::default();
    /// response.extensions_mut().insert("hello");
    /// assert_eq!(response.extensions().get(), Some(&"hello"));
    /// ```
    #[inline]
    pub fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.head.extensions
    }
    /// Returns a reference to the associated HTTP body.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let response: Response<String> = Response::default();
    /// assert!(response.body().is_empty());
    /// ```
    #[inline]
    pub fn body(&self) -> &T {
        &self.body
    }
    /// Returns a mutable reference to the associated HTTP body.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let mut response: Response<String> = Response::default();
    /// response.body_mut().push_str("hello world");
    /// assert!(!response.body().is_empty());
    /// ```
    #[inline]
    pub fn body_mut(&mut self) -> &mut T {
        &mut self.body
    }
    /// Consumes the response, returning just the body.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::Response;
    /// let response = Response::new(10);
    /// let body = response.into_body();
    /// assert_eq!(body, 10);
    /// ```
    #[inline]
    pub fn into_body(self) -> T {
        self.body
    }
    /// Consumes the response returning the head and body parts.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let response: Response<()> = Response::default();
    /// let (parts, body) = response.into_parts();
    /// assert_eq!(parts.status, StatusCode::OK);
    /// ```
    #[inline]
    pub fn into_parts(self) -> (Parts, T) {
        (self.head, self.body)
    }
    /// Consumes the response returning a new response with body mapped to the
    /// return type of the passed in function.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let response = Response::builder().body("some string").unwrap();
    /// let mapped_response: Response<&[u8]> = response.map(|b| {
    ///   assert_eq!(b, "some string");
    ///   b.as_bytes()
    /// });
    /// assert_eq!(mapped_response.body(), &"some string".as_bytes());
    /// ```
    #[inline]
    pub fn map<F, U>(self, f: F) -> Response<U>
    where
        F: FnOnce(T) -> U,
    {
        Response {
            body: f(self.body),
            head: self.head,
        }
    }
}
impl<T: Default> Default for Response<T> {
    #[inline]
    fn default() -> Response<T> {
        Response::new(T::default())
    }
}
impl<T: fmt::Debug> fmt::Debug for Response<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Response")
            .field("status", &self.status())
            .field("version", &self.version())
            .field("headers", self.headers())
            .field("body", self.body())
            .finish()
    }
}
impl Parts {
    /// Creates a new default instance of `Parts`
    fn new() -> Parts {
        Parts {
            status: StatusCode::default(),
            version: Version::default(),
            headers: HeaderMap::default(),
            extensions: Extensions::default(),
            _priv: (),
        }
    }
}
impl fmt::Debug for Parts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Parts")
            .field("status", &self.status)
            .field("version", &self.version)
            .field("headers", &self.headers)
            .finish()
    }
}
impl Builder {
    /// Creates a new default instance of `Builder` to construct either a
    /// `Head` or a `Response`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let response = response::Builder::new()
    ///     .status(200)
    ///     .body(())
    ///     .unwrap();
    /// ```
    #[inline]
    pub fn new() -> Builder {
        Builder::default()
    }
    /// Set the HTTP status for this response.
    ///
    /// This function will configure the HTTP status code of the `Response` that
    /// will be returned from `Builder::build`.
    ///
    /// By default this is `200`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let response = Response::builder()
    ///     .status(200)
    ///     .body(())
    ///     .unwrap();
    /// ```
    pub fn status<T>(self, status: T) -> Builder
    where
        StatusCode: TryFrom<T>,
        <StatusCode as TryFrom<T>>::Error: Into<crate::Error>,
    {
        self.and_then(move |mut head| {
            head.status = TryFrom::try_from(status).map_err(Into::into)?;
            Ok(head)
        })
    }
    /// Set the HTTP version for this response.
    ///
    /// This function will configure the HTTP version of the `Response` that
    /// will be returned from `Builder::build`.
    ///
    /// By default this is HTTP/1.1
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let response = Response::builder()
    ///     .version(Version::HTTP_2)
    ///     .body(())
    ///     .unwrap();
    /// ```
    pub fn version(self, version: Version) -> Builder {
        self.and_then(move |mut head| {
            head.version = version;
            Ok(head)
        })
    }
    /// Appends a header to this response builder.
    ///
    /// This function will append the provided key/value as a header to the
    /// internal `HeaderMap` being constructed. Essentially this is equivalent
    /// to calling `HeaderMap::append`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// # use http::header::HeaderValue;
    ///
    /// let response = Response::builder()
    ///     .header("Content-Type", "text/html")
    ///     .header("X-Custom-Foo", "bar")
    ///     .header("content-length", 0)
    ///     .body(())
    ///     .unwrap();
    /// ```
    pub fn header<K, V>(self, key: K, value: V) -> Builder
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<crate::Error>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<crate::Error>,
    {
        self.and_then(move |mut head| {
            let name = <HeaderName as TryFrom<K>>::try_from(key).map_err(Into::into)?;
            let value = <HeaderValue as TryFrom<V>>::try_from(value)
                .map_err(Into::into)?;
            head.headers.append(name, value);
            Ok(head)
        })
    }
    /// Get header on this response builder.
    ///
    /// When builder has error returns None.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::Response;
    /// # use http::header::HeaderValue;
    /// let res = Response::builder()
    ///     .header("Accept", "text/html")
    ///     .header("X-Custom-Foo", "bar");
    /// let headers = res.headers_ref().unwrap();
    /// assert_eq!( headers["Accept"], "text/html" );
    /// assert_eq!( headers["X-Custom-Foo"], "bar" );
    /// ```
    pub fn headers_ref(&self) -> Option<&HeaderMap<HeaderValue>> {
        self.inner.as_ref().ok().map(|h| &h.headers)
    }
    /// Get header on this response builder.
    /// when builder has error returns None
    ///
    /// # Example
    ///
    /// ```
    /// # use http::*;
    /// # use http::header::HeaderValue;
    /// # use http::response::Builder;
    /// let mut res = Response::builder();
    /// {
    ///   let headers = res.headers_mut().unwrap();
    ///   headers.insert("Accept", HeaderValue::from_static("text/html"));
    ///   headers.insert("X-Custom-Foo", HeaderValue::from_static("bar"));
    /// }
    /// let headers = res.headers_ref().unwrap();
    /// assert_eq!( headers["Accept"], "text/html" );
    /// assert_eq!( headers["X-Custom-Foo"], "bar" );
    /// ```
    pub fn headers_mut(&mut self) -> Option<&mut HeaderMap<HeaderValue>> {
        self.inner.as_mut().ok().map(|h| &mut h.headers)
    }
    /// Adds an extension to this builder
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let response = Response::builder()
    ///     .extension("My Extension")
    ///     .body(())
    ///     .unwrap();
    ///
    /// assert_eq!(response.extensions().get::<&'static str>(),
    ///            Some(&"My Extension"));
    /// ```
    pub fn extension<T>(self, extension: T) -> Builder
    where
        T: Any + Send + Sync + 'static,
    {
        self.and_then(move |mut head| {
            head.extensions.insert(extension);
            Ok(head)
        })
    }
    /// Get a reference to the extensions for this response builder.
    ///
    /// If the builder has an error, this returns `None`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::Response;
    /// let req = Response::builder().extension("My Extension").extension(5u32);
    /// let extensions = req.extensions_ref().unwrap();
    /// assert_eq!(extensions.get::<&'static str>(), Some(&"My Extension"));
    /// assert_eq!(extensions.get::<u32>(), Some(&5u32));
    /// ```
    pub fn extensions_ref(&self) -> Option<&Extensions> {
        self.inner.as_ref().ok().map(|h| &h.extensions)
    }
    /// Get a mutable reference to the extensions for this response builder.
    ///
    /// If the builder has an error, this returns `None`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::Response;
    /// let mut req = Response::builder().extension("My Extension");
    /// let mut extensions = req.extensions_mut().unwrap();
    /// assert_eq!(extensions.get::<&'static str>(), Some(&"My Extension"));
    /// extensions.insert(5u32);
    /// assert_eq!(extensions.get::<u32>(), Some(&5u32));
    /// ```
    pub fn extensions_mut(&mut self) -> Option<&mut Extensions> {
        self.inner.as_mut().ok().map(|h| &mut h.extensions)
    }
    /// "Consumes" this builder, using the provided `body` to return a
    /// constructed `Response`.
    ///
    /// # Errors
    ///
    /// This function may return an error if any previously configured argument
    /// failed to parse or get converted to the internal representation. For
    /// example if an invalid `head` was specified via `header("Foo",
    /// "Bar\r\n")` the error will be returned when this function is called
    /// rather than when `header` was called.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let response = Response::builder()
    ///     .body(())
    ///     .unwrap();
    /// ```
    pub fn body<T>(self, body: T) -> Result<Response<T>> {
        self.inner.map(move |head| { Response { head, body } })
    }
    fn and_then<F>(self, func: F) -> Self
    where
        F: FnOnce(Parts) -> Result<Parts>,
    {
        Builder {
            inner: self.inner.and_then(func),
        }
    }
}
impl Default for Builder {
    #[inline]
    fn default() -> Builder {
        Builder { inner: Ok(Parts::new()) }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_can_map_a_body_from_one_type_to_another() {
        let response = Response::builder().body("some string").unwrap();
        let mapped_response = response
            .map(|s| {
                assert_eq!(s, "some string");
                123u32
            });
        assert_eq!(mapped_response.body(), & 123u32);
    }
}
#[cfg(test)]
mod tests_llm_16_214 {
    use crate::response::{Builder, Parts};
    #[test]
    fn test_default() {
        let _rug_st_tests_llm_16_214_rrrruuuugggg_test_default = 0;
        let default_builder: Builder = Default::default();
        let expected_builder = Builder { inner: Ok(Parts::new()) };
        debug_assert_eq!(default_builder.inner.is_ok(), expected_builder.inner.is_ok());
        let _rug_ed_tests_llm_16_214_rrrruuuugggg_test_default = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_513 {
    use super::*;
    use crate::*;
    use crate::{Response, Extensions};
    use std::any::Any;
    #[test]
    fn test_extension() {
        let _rug_st_tests_llm_16_513_rrrruuuugggg_test_extension = 0;
        let rug_fuzz_0 = "My Extension";
        let response: Response<()> = Response::builder()
            .extension(rug_fuzz_0)
            .body(())
            .unwrap();
        let extensions: &Extensions = response.extensions();
        let value = extensions.get::<&'static str>().unwrap();
        debug_assert_eq!(value, & "My Extension");
        let _rug_ed_tests_llm_16_513_rrrruuuugggg_test_extension = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_514 {
    use super::*;
    use crate::*;
    use crate::{Response, Extensions};
    #[test]
    fn test_extensions_mut() {
        let _rug_st_tests_llm_16_514_rrrruuuugggg_test_extensions_mut = 0;
        let rug_fuzz_0 = "My Extension";
        let rug_fuzz_1 = 5u32;
        let mut req = Response::builder().extension(rug_fuzz_0);
        let mut extensions = req.extensions_mut().unwrap();
        debug_assert_eq!(extensions.get:: < & 'static str > (), Some(& "My Extension"));
        extensions.insert(rug_fuzz_1);
        debug_assert_eq!(extensions.get:: < u32 > (), Some(& 5u32));
        let _rug_ed_tests_llm_16_514_rrrruuuugggg_test_extensions_mut = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_515 {
    use super::*;
    use crate::*;
    use crate::Extensions;
    #[test]
    fn test_extensions_ref() {
        let _rug_st_tests_llm_16_515_rrrruuuugggg_test_extensions_ref = 0;
        let rug_fuzz_0 = "My Extension";
        let rug_fuzz_1 = 5u32;
        let req = Response::builder().extension(rug_fuzz_0).extension(rug_fuzz_1);
        let extensions = req.extensions_ref().unwrap();
        debug_assert_eq!(extensions.get:: < & 'static str > (), Some(& "My Extension"));
        debug_assert_eq!(extensions.get:: < u32 > (), Some(& 5u32));
        let _rug_ed_tests_llm_16_515_rrrruuuugggg_test_extensions_ref = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_517 {
    use super::*;
    use crate::*;
    use crate::header::HeaderValue;
    use crate::header::HeaderMap;
    #[test]
    fn test_headers_mut() {
        let _rug_st_tests_llm_16_517_rrrruuuugggg_test_headers_mut = 0;
        let rug_fuzz_0 = "Accept";
        let rug_fuzz_1 = "text/html";
        let rug_fuzz_2 = "X-Custom-Foo";
        let rug_fuzz_3 = "bar";
        let rug_fuzz_4 = "Accept";
        let rug_fuzz_5 = "X-Custom-Foo";
        let mut res = Response::builder();
        {
            let headers = res.headers_mut().unwrap();
            headers.insert(rug_fuzz_0, HeaderValue::from_static(rug_fuzz_1));
            headers.insert(rug_fuzz_2, HeaderValue::from_static(rug_fuzz_3));
        }
        let headers = res.headers_ref().unwrap();
        debug_assert_eq!(headers[rug_fuzz_4], "text/html");
        debug_assert_eq!(headers[rug_fuzz_5], "bar");
        let _rug_ed_tests_llm_16_517_rrrruuuugggg_test_headers_mut = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_521 {
    use super::*;
    use crate::*;
    use crate::*;
    use crate::version::Version;
    #[test]
    fn test_response_builder_version() {
        let _rug_st_tests_llm_16_521_rrrruuuugggg_test_response_builder_version = 0;
        let response = Response::builder().version(Version::HTTP_2).body(()).unwrap();
        debug_assert_eq!(response.head.version, Version::HTTP_2);
        let _rug_ed_tests_llm_16_521_rrrruuuugggg_test_response_builder_version = 0;
    }
}
#[cfg(test)]
mod tests_rug_206 {
    use super::*;
    use crate::response::Response;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_206_rrrruuuugggg_test_rug = 0;
        Response::<()>::builder();
        let _rug_ed_tests_rug_206_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_207 {
    use super::*;
    use crate::{Response, StatusCode};
    #[test]
    fn test_new() {
        let _rug_st_tests_rug_207_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = "hello world";
        let body = rug_fuzz_0;
        let response: Response<&str> = Response::new(body);
        debug_assert_eq!(response.status(), StatusCode::OK);
        debug_assert_eq!(* response.body(), "hello world");
        let _rug_ed_tests_rug_207_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_rug_209 {
    use super::*;
    use crate::{response::Response, StatusCode};
    #[test]
    fn test_status() {
        let _rug_st_tests_rug_209_rrrruuuugggg_test_status = 0;
        let mut p0: Response<()> = Response::default();
        debug_assert_eq!(< Response < () > > ::status(& p0), StatusCode::OK);
        let _rug_ed_tests_rug_209_rrrruuuugggg_test_status = 0;
    }
}
#[cfg(test)]
mod tests_rug_210 {
    use super::*;
    use crate::response::Response;
    use crate::StatusCode;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_210_rrrruuuugggg_test_rug = 0;
        let mut p0: Response<()> = Response::default();
        *p0.status_mut() = StatusCode::CREATED;
        debug_assert_eq!(p0.status(), StatusCode::CREATED);
        let _rug_ed_tests_rug_210_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_211 {
    use super::*;
    use crate::response::Response;
    use crate::version::Version;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_211_rrrruuuugggg_test_rug = 0;
        let mut p0: Response<()> = Response::default();
        <Response<()>>::version(&p0);
        let _rug_ed_tests_rug_211_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_212 {
    use super::*;
    use crate::response::Response;
    use crate::version::Version;
    #[test]
    fn test_response_version_mut() {
        let _rug_st_tests_rug_212_rrrruuuugggg_test_response_version_mut = 0;
        let rug_fuzz_0 = 200;
        let mut p0: Response<u32> = Response::new(rug_fuzz_0);
        Response::<u32>::version_mut(&mut p0);
        let _rug_ed_tests_rug_212_rrrruuuugggg_test_response_version_mut = 0;
    }
}
#[cfg(test)]
mod tests_rug_213 {
    use super::*;
    use crate::{response, HeaderMap, HeaderValue};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_213_rrrruuuugggg_sample = 0;
        let rug_fuzz_0 = 200;
        #[cfg(test)]
        mod tests_rug_213_prepare {
            use crate::response::Response;
            #[test]
            fn sample() {
                let _rug_st_tests_rug_213_prepare_rrrruuuugggg_sample = 0;
                let rug_fuzz_0 = 0;
                let rug_fuzz_1 = 200;
                let rug_fuzz_2 = 0;
                let _rug_st_tests_rug_213_rrrruuuugggg_sample = rug_fuzz_0;
                let rug_fuzz_0 = rug_fuzz_1;
                let mut v100: Response<u32> = Response::new(rug_fuzz_0);
                let _rug_ed_tests_rug_213_rrrruuuugggg_sample = rug_fuzz_2;
                let _rug_ed_tests_rug_213_prepare_rrrruuuugggg_sample = 0;
            }
        }
        let mut p0: Response<u32> = Response::new(200);
        <response::Response<u32>>::headers(&p0);
        let _rug_ed_tests_rug_213_rrrruuuugggg_sample = 0;
    }
}
#[cfg(test)]
mod tests_rug_214 {
    use super::*;
    use crate::response::Response;
    use crate::header::{HeaderMap, HeaderValue, HOST};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_214_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "world";
        let mut p0: Response<()> = Response::default();
        p0.headers_mut().insert(HOST, HeaderValue::from_static(rug_fuzz_0));
        crate::response::Response::<()>::headers_mut(&mut p0);
        let _rug_ed_tests_rug_214_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_215 {
    use super::*;
    use crate::Extensions;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_215_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 200;
        let mut p0: Response<u32> = Response::new(rug_fuzz_0);
        <Response<u32>>::extensions(&p0);
        let _rug_ed_tests_rug_215_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_216 {
    use super::*;
    use crate::{Extensions, response::Response};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_216_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "hello";
        let mut p0: Response<()> = Response::default();
        p0.extensions_mut().insert(rug_fuzz_0);
        debug_assert_eq!(p0.extensions().get(), Some(& "hello"));
        let _rug_ed_tests_rug_216_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_217 {
    use super::*;
    use crate::response::Response;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_217_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 200;
        let mut p0: Response<u32> = Response::new(rug_fuzz_0);
        Response::<u32>::body(&p0);
        let _rug_ed_tests_rug_217_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_218 {
    use super::*;
    use crate::response::Response;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_218_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 200;
        let mut p0: Response<u32> = Response::new(rug_fuzz_0);
        <Response<u32>>::body_mut(&mut p0);
        let _rug_ed_tests_rug_218_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_219 {
    use super::*;
    use crate::response::Response;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_219_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 200;
        let mut p0: Response<u32> = Response::new(rug_fuzz_0);
        <Response<u32>>::into_body(p0);
        let _rug_ed_tests_rug_219_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_220 {
    use super::*;
    use crate::{Response, StatusCode};
    #[test]
    fn test_into_parts() {
        let _rug_st_tests_rug_220_rrrruuuugggg_test_into_parts = 0;
        let rug_fuzz_0 = 200;
        let mut p0: Response<u32> = Response::new(rug_fuzz_0);
        let _ = <Response<u32>>::into_parts(p0);
        let _rug_ed_tests_rug_220_rrrruuuugggg_test_into_parts = 0;
    }
}
#[cfg(test)]
mod tests_rug_221 {
    use super::*;
    use crate::response::Response;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_221_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 200;
        let mut p0: Response<u32> = Response::new(rug_fuzz_0);
        let mut p1 = |b: u32| {
            debug_assert_eq!(b, 200);
            b as u64
        };
        p0.map(p1);
        let _rug_ed_tests_rug_221_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_223 {
    use super::*;
    use crate::{response::Parts, HeaderMap, StatusCode, Version};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_223_rrrruuuugggg_test_rug = 0;
        let _ = Parts::new();
        let _rug_ed_tests_rug_223_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_224 {
    use super::*;
    use crate::response::Builder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_224_rrrruuuugggg_test_rug = 0;
        Builder::new();
        let _rug_ed_tests_rug_224_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_226 {
    use super::*;
    use crate::{response, StatusCode};
    use crate::header::{HeaderName, HeaderValue};
    #[test]
    fn test_header() {
        let _rug_st_tests_rug_226_rrrruuuugggg_test_header = 0;
        let rug_fuzz_0 = "Content-Type";
        let rug_fuzz_1 = "text/html";
        let mut p0: response::Builder = response::Builder::new();
        let p1: HeaderName = HeaderName::try_from(rug_fuzz_0).unwrap();
        let p2: HeaderValue = HeaderValue::try_from(rug_fuzz_1).unwrap();
        p0.header(p1, p2);
        let _rug_ed_tests_rug_226_rrrruuuugggg_test_header = 0;
    }
}
#[cfg(test)]
mod tests_rug_227 {
    use super::*;
    use crate::{response, StatusCode};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_227_rrrruuuugggg_test_rug = 0;
        let mut p0: response::Builder = response::Builder::new();
        <response::Builder>::headers_ref(&p0);
        let _rug_ed_tests_rug_227_rrrruuuugggg_test_rug = 0;
    }
}
