//! HTTP request types.
//!
//! This module contains structs related to HTTP requests, notably the
//! `Request` type itself as well as a builder to create requests. Typically
//! you'll import the `http::Request` type rather than reaching into this
//! module itself.
//!
//! # Examples
//!
//! Creating a `Request` to send
//!
//! ```no_run
//! use http::{Request, Response};
//!
//! let mut request = Request::builder()
//!     .uri("https://www.rust-lang.org/")
//!     .header("User-Agent", "my-awesome-agent/1.0");
//!
//! if needs_awesome_header() {
//!     request = request.header("Awesome", "yes");
//! }
//!
//! let response = send(request.body(()).unwrap());
//!
//! # fn needs_awesome_header() -> bool {
//! #     true
//! # }
//! #
//! fn send(req: Request<()>) -> Response<()> {
//!     // ...
//! # panic!()
//! }
//! ```
//!
//! Inspecting a request to see what was sent.
//!
//! ```
//! use http::{Request, Response, StatusCode};
//!
//! fn respond_to(req: Request<()>) -> http::Result<Response<()>> {
//!     if req.uri() != "/awesome-url" {
//!         return Response::builder()
//!             .status(StatusCode::NOT_FOUND)
//!             .body(())
//!     }
//!
//!     let has_awesome_header = req.headers().contains_key("Awesome");
//!     let body = req.body();
//!
//!     // ...
//! # panic!()
//! }
//! ```
use std::any::Any;
use std::convert::TryFrom;
use std::fmt;
use crate::header::{HeaderMap, HeaderName, HeaderValue};
use crate::method::Method;
use crate::version::Version;
use crate::{Extensions, Result, Uri};
/// Represents an HTTP request.
///
/// An HTTP request consists of a head and a potentially optional body. The body
/// component is generic, enabling arbitrary types to represent the HTTP body.
/// For example, the body could be `Vec<u8>`, a `Stream` of byte chunks, or a
/// value that has been deserialized.
///
/// # Examples
///
/// Creating a `Request` to send
///
/// ```no_run
/// use http::{Request, Response};
///
/// let mut request = Request::builder()
///     .uri("https://www.rust-lang.org/")
///     .header("User-Agent", "my-awesome-agent/1.0");
///
/// if needs_awesome_header() {
///     request = request.header("Awesome", "yes");
/// }
///
/// let response = send(request.body(()).unwrap());
///
/// # fn needs_awesome_header() -> bool {
/// #     true
/// # }
/// #
/// fn send(req: Request<()>) -> Response<()> {
///     // ...
/// # panic!()
/// }
/// ```
///
/// Inspecting a request to see what was sent.
///
/// ```
/// use http::{Request, Response, StatusCode};
///
/// fn respond_to(req: Request<()>) -> http::Result<Response<()>> {
///     if req.uri() != "/awesome-url" {
///         return Response::builder()
///             .status(StatusCode::NOT_FOUND)
///             .body(())
///     }
///
///     let has_awesome_header = req.headers().contains_key("Awesome");
///     let body = req.body();
///
///     // ...
/// # panic!()
/// }
/// ```
///
/// Deserialize a request of bytes via json:
///
/// ```
/// # extern crate serde;
/// # extern crate serde_json;
/// # extern crate http;
/// use http::Request;
/// use serde::de;
///
/// fn deserialize<T>(req: Request<Vec<u8>>) -> serde_json::Result<Request<T>>
///     where for<'de> T: de::Deserialize<'de>,
/// {
///     let (parts, body) = req.into_parts();
///     let body = serde_json::from_slice(&body)?;
///     Ok(Request::from_parts(parts, body))
/// }
/// #
/// # fn main() {}
/// ```
///
/// Or alternatively, serialize the body of a request to json
///
/// ```
/// # extern crate serde;
/// # extern crate serde_json;
/// # extern crate http;
/// use http::Request;
/// use serde::ser;
///
/// fn serialize<T>(req: Request<T>) -> serde_json::Result<Request<Vec<u8>>>
///     where T: ser::Serialize,
/// {
///     let (parts, body) = req.into_parts();
///     let body = serde_json::to_vec(&body)?;
///     Ok(Request::from_parts(parts, body))
/// }
/// #
/// # fn main() {}
/// ```
pub struct Request<T> {
    head: Parts,
    body: T,
}
/// Component parts of an HTTP `Request`
///
/// The HTTP request head consists of a method, uri, version, and a set of
/// header fields.
pub struct Parts {
    /// The request's method
    pub method: Method,
    /// The request's URI
    pub uri: Uri,
    /// The request's version
    pub version: Version,
    /// The request's headers
    pub headers: HeaderMap<HeaderValue>,
    /// The request's extensions
    pub extensions: Extensions,
    _priv: (),
}
/// An HTTP request builder
///
/// This type can be used to construct an instance or `Request`
/// through a builder-like pattern.
#[derive(Debug)]
pub struct Builder {
    inner: Result<Parts>,
}
impl Request<()> {
    /// Creates a new builder-style object to manufacture a `Request`
    ///
    /// This method returns an instance of `Builder` which can be used to
    /// create a `Request`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let request = Request::builder()
    ///     .method("GET")
    ///     .uri("https://www.rust-lang.org/")
    ///     .header("X-Custom-Foo", "Bar")
    ///     .body(())
    ///     .unwrap();
    /// ```
    #[inline]
    pub fn builder() -> Builder {
        Builder::new()
    }
    /// Creates a new `Builder` initialized with a GET method and the given URI.
    ///
    /// This method returns an instance of `Builder` which can be used to
    /// create a `Request`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::*;
    ///
    /// let request = Request::get("https://www.rust-lang.org/")
    ///     .body(())
    ///     .unwrap();
    /// ```
    pub fn get<T>(uri: T) -> Builder
    where
        Uri: TryFrom<T>,
        <Uri as TryFrom<T>>::Error: Into<crate::Error>,
    {
        Builder::new().method(Method::GET).uri(uri)
    }
    /// Creates a new `Builder` initialized with a PUT method and the given URI.
    ///
    /// This method returns an instance of `Builder` which can be used to
    /// create a `Request`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::*;
    ///
    /// let request = Request::put("https://www.rust-lang.org/")
    ///     .body(())
    ///     .unwrap();
    /// ```
    pub fn put<T>(uri: T) -> Builder
    where
        Uri: TryFrom<T>,
        <Uri as TryFrom<T>>::Error: Into<crate::Error>,
    {
        Builder::new().method(Method::PUT).uri(uri)
    }
    /// Creates a new `Builder` initialized with a POST method and the given URI.
    ///
    /// This method returns an instance of `Builder` which can be used to
    /// create a `Request`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::*;
    ///
    /// let request = Request::post("https://www.rust-lang.org/")
    ///     .body(())
    ///     .unwrap();
    /// ```
    pub fn post<T>(uri: T) -> Builder
    where
        Uri: TryFrom<T>,
        <Uri as TryFrom<T>>::Error: Into<crate::Error>,
    {
        Builder::new().method(Method::POST).uri(uri)
    }
    /// Creates a new `Builder` initialized with a DELETE method and the given URI.
    ///
    /// This method returns an instance of `Builder` which can be used to
    /// create a `Request`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::*;
    ///
    /// let request = Request::delete("https://www.rust-lang.org/")
    ///     .body(())
    ///     .unwrap();
    /// ```
    pub fn delete<T>(uri: T) -> Builder
    where
        Uri: TryFrom<T>,
        <Uri as TryFrom<T>>::Error: Into<crate::Error>,
    {
        Builder::new().method(Method::DELETE).uri(uri)
    }
    /// Creates a new `Builder` initialized with an OPTIONS method and the given URI.
    ///
    /// This method returns an instance of `Builder` which can be used to
    /// create a `Request`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::*;
    ///
    /// let request = Request::options("https://www.rust-lang.org/")
    ///     .body(())
    ///     .unwrap();
    /// # assert_eq!(*request.method(), Method::OPTIONS);
    /// ```
    pub fn options<T>(uri: T) -> Builder
    where
        Uri: TryFrom<T>,
        <Uri as TryFrom<T>>::Error: Into<crate::Error>,
    {
        Builder::new().method(Method::OPTIONS).uri(uri)
    }
    /// Creates a new `Builder` initialized with a HEAD method and the given URI.
    ///
    /// This method returns an instance of `Builder` which can be used to
    /// create a `Request`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::*;
    ///
    /// let request = Request::head("https://www.rust-lang.org/")
    ///     .body(())
    ///     .unwrap();
    /// ```
    pub fn head<T>(uri: T) -> Builder
    where
        Uri: TryFrom<T>,
        <Uri as TryFrom<T>>::Error: Into<crate::Error>,
    {
        Builder::new().method(Method::HEAD).uri(uri)
    }
    /// Creates a new `Builder` initialized with a CONNECT method and the given URI.
    ///
    /// This method returns an instance of `Builder` which can be used to
    /// create a `Request`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::*;
    ///
    /// let request = Request::connect("https://www.rust-lang.org/")
    ///     .body(())
    ///     .unwrap();
    /// ```
    pub fn connect<T>(uri: T) -> Builder
    where
        Uri: TryFrom<T>,
        <Uri as TryFrom<T>>::Error: Into<crate::Error>,
    {
        Builder::new().method(Method::CONNECT).uri(uri)
    }
    /// Creates a new `Builder` initialized with a PATCH method and the given URI.
    ///
    /// This method returns an instance of `Builder` which can be used to
    /// create a `Request`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::*;
    ///
    /// let request = Request::patch("https://www.rust-lang.org/")
    ///     .body(())
    ///     .unwrap();
    /// ```
    pub fn patch<T>(uri: T) -> Builder
    where
        Uri: TryFrom<T>,
        <Uri as TryFrom<T>>::Error: Into<crate::Error>,
    {
        Builder::new().method(Method::PATCH).uri(uri)
    }
    /// Creates a new `Builder` initialized with a TRACE method and the given URI.
    ///
    /// This method returns an instance of `Builder` which can be used to
    /// create a `Request`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::*;
    ///
    /// let request = Request::trace("https://www.rust-lang.org/")
    ///     .body(())
    ///     .unwrap();
    /// ```
    pub fn trace<T>(uri: T) -> Builder
    where
        Uri: TryFrom<T>,
        <Uri as TryFrom<T>>::Error: Into<crate::Error>,
    {
        Builder::new().method(Method::TRACE).uri(uri)
    }
}
impl<T> Request<T> {
    /// Creates a new blank `Request` with the body
    ///
    /// The component parts of this request will be set to their default, e.g.
    /// the GET method, no headers, etc.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let request = Request::new("hello world");
    ///
    /// assert_eq!(*request.method(), Method::GET);
    /// assert_eq!(*request.body(), "hello world");
    /// ```
    #[inline]
    pub fn new(body: T) -> Request<T> {
        Request {
            head: Parts::new(),
            body: body,
        }
    }
    /// Creates a new `Request` with the given components parts and body.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let request = Request::new("hello world");
    /// let (mut parts, body) = request.into_parts();
    /// parts.method = Method::POST;
    ///
    /// let request = Request::from_parts(parts, body);
    /// ```
    #[inline]
    pub fn from_parts(parts: Parts, body: T) -> Request<T> {
        Request { head: parts, body: body }
    }
    /// Returns a reference to the associated HTTP method.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let request: Request<()> = Request::default();
    /// assert_eq!(*request.method(), Method::GET);
    /// ```
    #[inline]
    pub fn method(&self) -> &Method {
        &self.head.method
    }
    /// Returns a mutable reference to the associated HTTP method.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let mut request: Request<()> = Request::default();
    /// *request.method_mut() = Method::PUT;
    /// assert_eq!(*request.method(), Method::PUT);
    /// ```
    #[inline]
    pub fn method_mut(&mut self) -> &mut Method {
        &mut self.head.method
    }
    /// Returns a reference to the associated URI.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let request: Request<()> = Request::default();
    /// assert_eq!(*request.uri(), *"/");
    /// ```
    #[inline]
    pub fn uri(&self) -> &Uri {
        &self.head.uri
    }
    /// Returns a mutable reference to the associated URI.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let mut request: Request<()> = Request::default();
    /// *request.uri_mut() = "/hello".parse().unwrap();
    /// assert_eq!(*request.uri(), *"/hello");
    /// ```
    #[inline]
    pub fn uri_mut(&mut self) -> &mut Uri {
        &mut self.head.uri
    }
    /// Returns the associated version.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let request: Request<()> = Request::default();
    /// assert_eq!(request.version(), Version::HTTP_11);
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
    /// let mut request: Request<()> = Request::default();
    /// *request.version_mut() = Version::HTTP_2;
    /// assert_eq!(request.version(), Version::HTTP_2);
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
    /// let request: Request<()> = Request::default();
    /// assert!(request.headers().is_empty());
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
    /// let mut request: Request<()> = Request::default();
    /// request.headers_mut().insert(HOST, HeaderValue::from_static("world"));
    /// assert!(!request.headers().is_empty());
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
    /// let request: Request<()> = Request::default();
    /// assert!(request.extensions().get::<i32>().is_none());
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
    /// let mut request: Request<()> = Request::default();
    /// request.extensions_mut().insert("hello");
    /// assert_eq!(request.extensions().get(), Some(&"hello"));
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
    /// let request: Request<String> = Request::default();
    /// assert!(request.body().is_empty());
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
    /// let mut request: Request<String> = Request::default();
    /// request.body_mut().push_str("hello world");
    /// assert!(!request.body().is_empty());
    /// ```
    #[inline]
    pub fn body_mut(&mut self) -> &mut T {
        &mut self.body
    }
    /// Consumes the request, returning just the body.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::Request;
    /// let request = Request::new(10);
    /// let body = request.into_body();
    /// assert_eq!(body, 10);
    /// ```
    #[inline]
    pub fn into_body(self) -> T {
        self.body
    }
    /// Consumes the request returning the head and body parts.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let request = Request::new(());
    /// let (parts, body) = request.into_parts();
    /// assert_eq!(parts.method, Method::GET);
    /// ```
    #[inline]
    pub fn into_parts(self) -> (Parts, T) {
        (self.head, self.body)
    }
    /// Consumes the request returning a new request with body mapped to the
    /// return type of the passed in function.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let request = Request::builder().body("some string").unwrap();
    /// let mapped_request: Request<&[u8]> = request.map(|b| {
    ///   assert_eq!(b, "some string");
    ///   b.as_bytes()
    /// });
    /// assert_eq!(mapped_request.body(), &"some string".as_bytes());
    /// ```
    #[inline]
    pub fn map<F, U>(self, f: F) -> Request<U>
    where
        F: FnOnce(T) -> U,
    {
        Request {
            body: f(self.body),
            head: self.head,
        }
    }
}
impl<T: Default> Default for Request<T> {
    fn default() -> Request<T> {
        Request::new(T::default())
    }
}
impl<T: fmt::Debug> fmt::Debug for Request<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Request")
            .field("method", self.method())
            .field("uri", self.uri())
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
            method: Method::default(),
            uri: Uri::default(),
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
            .field("method", &self.method)
            .field("uri", &self.uri)
            .field("version", &self.version)
            .field("headers", &self.headers)
            .finish()
    }
}
impl Builder {
    /// Creates a new default instance of `Builder` to construct a `Request`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let req = request::Builder::new()
    ///     .method("POST")
    ///     .body(())
    ///     .unwrap();
    /// ```
    #[inline]
    pub fn new() -> Builder {
        Builder::default()
    }
    /// Set the HTTP method for this request.
    ///
    /// This function will configure the HTTP method of the `Request` that will
    /// be returned from `Builder::build`.
    ///
    /// By default this is `GET`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let req = Request::builder()
    ///     .method("POST")
    ///     .body(())
    ///     .unwrap();
    /// ```
    pub fn method<T>(self, method: T) -> Builder
    where
        Method: TryFrom<T>,
        <Method as TryFrom<T>>::Error: Into<crate::Error>,
    {
        self.and_then(move |mut head| {
            let method = TryFrom::try_from(method).map_err(Into::into)?;
            head.method = method;
            Ok(head)
        })
    }
    /// Get the HTTP Method for this request.
    ///
    /// By default this is `GET`. If builder has error, returns None.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let mut req = Request::builder();
    /// assert_eq!(req.method_ref(),Some(&Method::GET));
    ///
    /// req = req.method("POST");
    /// assert_eq!(req.method_ref(),Some(&Method::POST));
    /// ```
    pub fn method_ref(&self) -> Option<&Method> {
        self.inner.as_ref().ok().map(|h| &h.method)
    }
    /// Set the URI for this request.
    ///
    /// This function will configure the URI of the `Request` that will
    /// be returned from `Builder::build`.
    ///
    /// By default this is `/`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let req = Request::builder()
    ///     .uri("https://www.rust-lang.org/")
    ///     .body(())
    ///     .unwrap();
    /// ```
    pub fn uri<T>(self, uri: T) -> Builder
    where
        Uri: TryFrom<T>,
        <Uri as TryFrom<T>>::Error: Into<crate::Error>,
    {
        self.and_then(move |mut head| {
            head.uri = TryFrom::try_from(uri).map_err(Into::into)?;
            Ok(head)
        })
    }
    /// Get the URI for this request
    ///
    /// By default this is `/`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let mut req = Request::builder();
    /// assert_eq!(req.uri_ref().unwrap(), "/" );
    ///
    /// req = req.uri("https://www.rust-lang.org/");
    /// assert_eq!(req.uri_ref().unwrap(), "https://www.rust-lang.org/" );
    /// ```
    pub fn uri_ref(&self) -> Option<&Uri> {
        self.inner.as_ref().ok().map(|h| &h.uri)
    }
    /// Set the HTTP version for this request.
    ///
    /// This function will configure the HTTP version of the `Request` that
    /// will be returned from `Builder::build`.
    ///
    /// By default this is HTTP/1.1
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let req = Request::builder()
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
    /// Appends a header to this request builder.
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
    /// let req = Request::builder()
    ///     .header("Accept", "text/html")
    ///     .header("X-Custom-Foo", "bar")
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
    /// Get header on this request builder.
    /// when builder has error returns None
    ///
    /// # Example
    ///
    /// ```
    /// # use http::Request;
    /// let req = Request::builder()
    ///     .header("Accept", "text/html")
    ///     .header("X-Custom-Foo", "bar");
    /// let headers = req.headers_ref().unwrap();
    /// assert_eq!( headers["Accept"], "text/html" );
    /// assert_eq!( headers["X-Custom-Foo"], "bar" );
    /// ```
    pub fn headers_ref(&self) -> Option<&HeaderMap<HeaderValue>> {
        self.inner.as_ref().ok().map(|h| &h.headers)
    }
    /// Get headers on this request builder.
    ///
    /// When builder has error returns None.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::{header::HeaderValue, Request};
    /// let mut req = Request::builder();
    /// {
    ///   let headers = req.headers_mut().unwrap();
    ///   headers.insert("Accept", HeaderValue::from_static("text/html"));
    ///   headers.insert("X-Custom-Foo", HeaderValue::from_static("bar"));
    /// }
    /// let headers = req.headers_ref().unwrap();
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
    /// let req = Request::builder()
    ///     .extension("My Extension")
    ///     .body(())
    ///     .unwrap();
    ///
    /// assert_eq!(req.extensions().get::<&'static str>(),
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
    /// Get a reference to the extensions for this request builder.
    ///
    /// If the builder has an error, this returns `None`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::Request;
    /// let req = Request::builder().extension("My Extension").extension(5u32);
    /// let extensions = req.extensions_ref().unwrap();
    /// assert_eq!(extensions.get::<&'static str>(), Some(&"My Extension"));
    /// assert_eq!(extensions.get::<u32>(), Some(&5u32));
    /// ```
    pub fn extensions_ref(&self) -> Option<&Extensions> {
        self.inner.as_ref().ok().map(|h| &h.extensions)
    }
    /// Get a mutable reference to the extensions for this request builder.
    ///
    /// If the builder has an error, this returns `None`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::Request;
    /// let mut req = Request::builder().extension("My Extension");
    /// let mut extensions = req.extensions_mut().unwrap();
    /// assert_eq!(extensions.get::<&'static str>(), Some(&"My Extension"));
    /// extensions.insert(5u32);
    /// assert_eq!(extensions.get::<u32>(), Some(&5u32));
    /// ```
    pub fn extensions_mut(&mut self) -> Option<&mut Extensions> {
        self.inner.as_mut().ok().map(|h| &mut h.extensions)
    }
    /// "Consumes" this builder, using the provided `body` to return a
    /// constructed `Request`.
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
    /// let request = Request::builder()
    ///     .body(())
    ///     .unwrap();
    /// ```
    pub fn body<T>(self, body: T) -> Result<Request<T>> {
        self.inner.map(move |head| { Request { head, body } })
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
        let request = Request::builder().body("some string").unwrap();
        let mapped_request = request
            .map(|s| {
                assert_eq!(s, "some string");
                123u32
            });
        assert_eq!(mapped_request.body(), & 123u32);
    }
}
#[cfg(test)]
mod tests_llm_16_484 {
    use super::*;
    use crate::*;
    use crate::{Method, Request, Version, header::{HeaderMap, HeaderValue}};
    #[test]
    fn test_extension() {
        let _rug_st_tests_llm_16_484_rrrruuuugggg_test_extension = 0;
        let rug_fuzz_0 = "My Extension";
        let rug_fuzz_1 = 5u32;
        let rug_fuzz_2 = 1;
        let req = Request::builder()
            .extension(rug_fuzz_0)
            .extension(rug_fuzz_1)
            .extension(vec![rug_fuzz_2, 2, 3])
            .body(())
            .unwrap();
        let extensions = req.extensions();
        debug_assert_eq!(extensions.get:: < & 'static str > (), Some(& "My Extension"));
        debug_assert_eq!(extensions.get:: < u32 > (), Some(& 5u32));
        debug_assert_eq!(extensions.get:: < Vec < u8 > > (), Some(& vec![1, 2, 3]));
        let _rug_ed_tests_llm_16_484_rrrruuuugggg_test_extension = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_485 {
    use crate::{Request, Extensions};
    use std::any::Any;
    use std::sync::Mutex;
    #[test]
    fn test_extensions_mut() {
        let _rug_st_tests_llm_16_485_rrrruuuugggg_test_extensions_mut = 0;
        let rug_fuzz_0 = "My Extension";
        let rug_fuzz_1 = 5u32;
        let mut req = Request::builder().extension(rug_fuzz_0);
        let mut extensions = req.extensions_mut().unwrap();
        debug_assert_eq!(extensions.get:: < & 'static str > (), Some(& "My Extension"));
        extensions.insert(rug_fuzz_1);
        debug_assert_eq!(extensions.get:: < u32 > (), Some(& 5u32));
        let _rug_ed_tests_llm_16_485_rrrruuuugggg_test_extensions_mut = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_486 {
    use super::*;
    use crate::*;
    use crate::{Request, Extensions};
    #[test]
    fn test_extensions_ref() {
        let _rug_st_tests_llm_16_486_rrrruuuugggg_test_extensions_ref = 0;
        let rug_fuzz_0 = "My Extension";
        let rug_fuzz_1 = 5u32;
        let req = Request::builder().extension(rug_fuzz_0).extension(rug_fuzz_1);
        let extensions = req.extensions_ref().unwrap();
        debug_assert_eq!(extensions.get:: < & 'static str > (), Some(& "My Extension"));
        debug_assert_eq!(extensions.get:: < u32 > (), Some(& 5u32));
        let _rug_ed_tests_llm_16_486_rrrruuuugggg_test_extensions_ref = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_488 {
    use super::*;
    use crate::*;
    use crate::header::HeaderValue;
    use crate::Request;
    #[test]
    fn test_headers_mut() {
        let _rug_st_tests_llm_16_488_rrrruuuugggg_test_headers_mut = 0;
        let rug_fuzz_0 = "Accept";
        let rug_fuzz_1 = "text/html";
        let rug_fuzz_2 = "X-Custom-Foo";
        let rug_fuzz_3 = "bar";
        let rug_fuzz_4 = "Accept";
        let rug_fuzz_5 = "X-Custom-Foo";
        let mut req = Request::builder();
        {
            let headers = req.headers_mut().unwrap();
            headers.insert(rug_fuzz_0, HeaderValue::from_static(rug_fuzz_1));
            headers.insert(rug_fuzz_2, HeaderValue::from_static(rug_fuzz_3));
        }
        let headers = req.headers_ref().unwrap();
        debug_assert_eq!(headers[rug_fuzz_4], "text/html");
        debug_assert_eq!(headers[rug_fuzz_5], "bar");
        let _rug_ed_tests_llm_16_488_rrrruuuugggg_test_headers_mut = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_492 {
    use super::*;
    use crate::*;
    use crate::*;
    #[test]
    fn test_method_ref() {
        let _rug_st_tests_llm_16_492_rrrruuuugggg_test_method_ref = 0;
        let rug_fuzz_0 = "POST";
        let mut req = Request::builder();
        debug_assert_eq!(req.method_ref(), Some(& Method::GET));
        req = req.method(rug_fuzz_0);
        debug_assert_eq!(req.method_ref(), Some(& Method::POST));
        let _rug_ed_tests_llm_16_492_rrrruuuugggg_test_method_ref = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_496 {
    use super::*;
    use crate::*;
    use crate::{Request, Uri};
    #[test]
    fn test_uri_ref_default() {
        let _rug_st_tests_llm_16_496_rrrruuuugggg_test_uri_ref_default = 0;
        let req = Request::builder();
        debug_assert_eq!(req.uri_ref(), Some(& Uri::from_static("/")));
        let _rug_ed_tests_llm_16_496_rrrruuuugggg_test_uri_ref_default = 0;
    }
    #[test]
    fn test_uri_ref_custom() {
        let _rug_st_tests_llm_16_496_rrrruuuugggg_test_uri_ref_custom = 0;
        let rug_fuzz_0 = "https://www.rust-lang.org/";
        let req = Request::builder().uri(rug_fuzz_0);
        debug_assert_eq!(
            req.uri_ref(), Some(& Uri::from_static("https://www.rust-lang.org/"))
        );
        let _rug_ed_tests_llm_16_496_rrrruuuugggg_test_uri_ref_custom = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_497 {
    use super::*;
    use crate::*;
    use crate::{Request, Version};
    #[test]
    fn test_version() {
        let _rug_st_tests_llm_16_497_rrrruuuugggg_test_version = 0;
        let req = Request::builder().version(Version::HTTP_2).body(()).unwrap();
        debug_assert_eq!(req.version(), Version::HTTP_2);
        let _rug_ed_tests_llm_16_497_rrrruuuugggg_test_version = 0;
    }
}
pub mod test_request {
    use crate::header::{HeaderValue, CONTENT_TYPE, CONTENT_LENGTH};
    use crate::{Request, Method};
    use std::convert::TryInto;
    #[test]
    fn test_delete_request_builder() {
        let request = Request::delete("https://www.rust-lang.org/").body(()).unwrap();
        assert_eq!(request.method(), Method::DELETE);
    }
}
#[cfg(test)]
mod tests_llm_16_502 {
    use super::*;
    use crate::*;
    use crate::header::*;
    #[test]
    fn test_get() {
        let _rug_st_tests_llm_16_502_rrrruuuugggg_test_get = 0;
        let rug_fuzz_0 = "https://www.example.com/";
        let request = Request::get(rug_fuzz_0).body(()).unwrap();
        debug_assert_eq!(request.method(), & Method::GET);
        debug_assert_eq!(request.uri(), & "https://www.example.com/");
        let _rug_ed_tests_llm_16_502_rrrruuuugggg_test_get = 0;
    }
    #[test]
    fn test_headers() {
        let _rug_st_tests_llm_16_502_rrrruuuugggg_test_headers = 0;
        let rug_fuzz_0 = "https://www.example.com/";
        let rug_fuzz_1 = "Content-Type";
        let rug_fuzz_2 = "application/json";
        let rug_fuzz_3 = "Authorization";
        let rug_fuzz_4 = "Bearer asdf1234";
        let rug_fuzz_5 = "X-Custom";
        let rug_fuzz_6 = "foo";
        let rug_fuzz_7 = "Content-Type";
        let rug_fuzz_8 = "Authorization";
        let rug_fuzz_9 = "X-Custom";
        let request = Request::get(rug_fuzz_0)
            .header(rug_fuzz_1, rug_fuzz_2)
            .header(rug_fuzz_3, rug_fuzz_4)
            .header(rug_fuzz_5, rug_fuzz_6)
            .body(())
            .unwrap();
        let headers = request.headers();
        debug_assert_eq!(headers.get(rug_fuzz_7).unwrap(), "application/json");
        debug_assert_eq!(headers.get(rug_fuzz_8).unwrap(), "Bearer asdf1234");
        debug_assert_eq!(headers.get(rug_fuzz_9).unwrap(), "foo");
        let _rug_ed_tests_llm_16_502_rrrruuuugggg_test_headers = 0;
    }
    #[test]
    fn test_extensions() {
        let _rug_st_tests_llm_16_502_rrrruuuugggg_test_extensions = 0;
        let rug_fuzz_0 = "https://www.example.com/";
        let rug_fuzz_1 = "extension_data";
        let request = Request::get(rug_fuzz_0).extension(rug_fuzz_1).body(()).unwrap();
        let extensions = request.extensions();
        debug_assert_eq!(extensions.get:: < & str > ().unwrap(), & "extension_data");
        let _rug_ed_tests_llm_16_502_rrrruuuugggg_test_extensions = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_505 {
    use super::*;
    use crate::*;
    use crate::request::Request;
    #[test]
    fn test_patch() {
        let _rug_st_tests_llm_16_505_rrrruuuugggg_test_patch = 0;
        let rug_fuzz_0 = "https://www.rust-lang.org/";
        let uri: &'static str = rug_fuzz_0;
        let expected_method = crate::Method::PATCH;
        let request = Request::patch(uri).body(()).unwrap();
        debug_assert_eq!(request.method(), & expected_method);
        debug_assert_eq!(request.uri().to_string(), uri);
        let _rug_ed_tests_llm_16_505_rrrruuuugggg_test_patch = 0;
    }
}
#[cfg(test)]
mod tests_rug_172 {
    use super::*;
    use crate::Request;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_172_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "GET";
        let rug_fuzz_1 = "https://www.rust-lang.org/";
        let rug_fuzz_2 = "X-Custom-Foo";
        let rug_fuzz_3 = "Bar";
        let request = <Request<()>>::builder()
            .method(rug_fuzz_0)
            .uri(rug_fuzz_1)
            .header(rug_fuzz_2, rug_fuzz_3)
            .body(())
            .unwrap();
        let _rug_ed_tests_rug_172_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_173 {
    use super::*;
    use crate::{Request, Method};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_173_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https://www.rust-lang.org/";
        let p0: &str = rug_fuzz_0;
        Request::<()>::put(p0);
        let _rug_ed_tests_rug_173_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_174 {
    use super::*;
    use crate::{Request, Method};
    #[test]
    fn test_post() {
        let _rug_st_tests_rug_174_rrrruuuugggg_test_post = 0;
        let rug_fuzz_0 = "https://www.rust-lang.org/";
        let uri: &'static str = rug_fuzz_0;
        let p0 = uri;
        Request::<()>::post(p0);
        let _rug_ed_tests_rug_174_rrrruuuugggg_test_post = 0;
    }
}
#[cfg(test)]
mod tests_rug_175 {
    use super::*;
    use crate::{Request, Uri, Error, Method};
    #[test]
    fn test_options() {
        let _rug_st_tests_rug_175_rrrruuuugggg_test_options = 0;
        let rug_fuzz_0 = "https://www.rust-lang.org/";
        let p0: &str = rug_fuzz_0;
        Request::<()>::options::<&str>(p0);
        let _rug_ed_tests_rug_175_rrrruuuugggg_test_options = 0;
    }
}
#[cfg(test)]
mod tests_rug_176 {
    use super::*;
    use crate::{Request, Method};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_176_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https://www.rust-lang.org/";
        let p0: &str = rug_fuzz_0;
        Request::<()>::head(p0);
        let _rug_ed_tests_rug_176_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_177 {
    use super::*;
    use crate::{Request, Method};
    #[test]
    fn test_connect() {
        let _rug_st_tests_rug_177_rrrruuuugggg_test_connect = 0;
        let rug_fuzz_0 = "https://www.rust-lang.org/";
        let p0: &str = rug_fuzz_0;
        let request = Request::<()>::connect::<&str>(p0).body(()).unwrap();
        debug_assert_eq!(request.method(), Method::CONNECT);
        debug_assert_eq!(request.uri().to_string(), p0.to_string());
        let _rug_ed_tests_rug_177_rrrruuuugggg_test_connect = 0;
    }
}
#[cfg(test)]
mod tests_rug_178 {
    use super::*;
    use crate::{Request, Uri};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_178_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https://www.rust-lang.org/";
        let mut p0: &str = rug_fuzz_0;
        Request::<()>::trace::<&str>(p0);
        let _rug_ed_tests_rug_178_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_179 {
    use super::*;
    use crate::{Request, Method};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_179_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "hello world";
        let mut p0 = rug_fuzz_0;
        let request: Request<&str> = Request::new(p0);
        debug_assert_eq!(* request.method(), Method::GET);
        debug_assert_eq!(* request.body(), "hello world");
        let _rug_ed_tests_rug_179_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_181 {
    use super::*;
    use crate::request::Request;
    use crate::Method;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_181_rrrruuuugggg_test_rug = 0;
        let mut v97: Request<()> = Request::new(());
        <Request<()>>::method(&v97);
        let _rug_ed_tests_rug_181_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_182 {
    use super::*;
    use crate::request::Request;
    use crate::Method;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_182_rrrruuuugggg_test_rug = 0;
        let mut p0: Request<()> = Request::default();
        *p0.method_mut() = Method::PUT;
        debug_assert_eq!(* p0.method(), Method::PUT);
        let _rug_ed_tests_rug_182_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_183 {
    use super::*;
    use crate::request::Request;
    use crate::Uri;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_183_rrrruuuugggg_test_rug = 0;
        let mut p0: Request<()> = Request::default();
        crate::request::Request::<()>::uri(&p0);
        let _rug_ed_tests_rug_183_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_184 {
    use super::*;
    use crate::request::Request;
    use crate::{Uri, Method, Version};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_184_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "/hello";
        let mut p0: Request<()> = Request::default();
        *p0.method_mut() = Method::POST;
        *p0.version_mut() = Version::HTTP_11;
        *p0.uri_mut() = rug_fuzz_0.parse().unwrap();
        let _rug_ed_tests_rug_184_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_185 {
    use super::*;
    use crate::request::Request;
    use crate::Version;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_185_rrrruuuugggg_test_rug = 0;
        let mut p0: Request<()> = Request::new(());
        <Request<()>>::version(&p0);
        let _rug_ed_tests_rug_185_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_186 {
    use super::*;
    use crate::request::Request;
    use crate::Version;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_186_rrrruuuugggg_test_rug = 0;
        let mut p0: Request<()> = Request::default();
        *p0.version_mut() = Version::HTTP_2;
        <Request<_>>::version_mut(&mut p0);
        let _rug_ed_tests_rug_186_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_187 {
    use super::*;
    use crate::request::Request;
    use crate::{HeaderMap, HeaderValue};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_187_rrrruuuugggg_test_rug = 0;
        let mut v97: Request<Vec<u8>> = Request::new(Vec::new());
        let p0: &Request<Vec<u8>> = &v97;
        let _ = Request::<Vec<u8>>::headers(p0);
        let _rug_ed_tests_rug_187_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_188 {
    use super::*;
    use crate::header::*;
    use crate::request::Request;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_188_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "world";
        let mut p0: Request<()> = Request::default();
        p0.headers_mut().insert(HOST, HeaderValue::from_static(rug_fuzz_0));
        debug_assert!(! p0.headers().is_empty());
        let _rug_ed_tests_rug_188_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_189 {
    use super::*;
    use crate::Extensions;
    use crate::request::Request;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_189_rrrruuuugggg_test_rug = 0;
        let mut p0: Request<()> = Request::default();
        let result = Request::<()>::extensions(&p0);
        let _rug_ed_tests_rug_189_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_190 {
    use super::*;
    use crate::extensions::Extensions;
    use crate::request::Request;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_190_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "hello";
        let mut p0: Request<()> = Request::default();
        p0.extensions_mut().insert(rug_fuzz_0);
        debug_assert_eq!(p0.extensions().get(), Some(& "hello"));
        let _rug_ed_tests_rug_190_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_191 {
    use super::*;
    use crate::request::Request;
    use crate::{Method, Uri, Version};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_191_rrrruuuugggg_test_rug = 0;
        let mut p0: Request<Vec<u8>> = Request::new(Vec::new());
        <Request<Vec<u8>>>::body(&p0);
        let _rug_ed_tests_rug_191_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_192 {
    use super::*;
    use crate::request::Request;
    use crate::{Method, Uri, Version};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_192_rrrruuuugggg_test_rug = 0;
        let mut p0: Request<Vec<u8>> = Request::new(Vec::new());
        <Request<Vec<u8>>>::body_mut(&mut p0);
        let _rug_ed_tests_rug_192_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_193 {
    use super::*;
    use crate::request::Request;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_193_rrrruuuugggg_test_rug = 0;
        let mut p0: Request<Vec<u8>> = Request::new(Vec::new());
        Request::<Vec<u8>>::into_body(p0);
        let _rug_ed_tests_rug_193_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_194 {
    use super::*;
    use crate::{Method, Version};
    use crate::request::Request;
    #[test]
    fn test_into_parts() {
        let _rug_st_tests_rug_194_rrrruuuugggg_test_into_parts = 0;
        let mut p0: Request<()> = Request::new(());
        let (parts, body) = p0.into_parts();
        debug_assert_eq!(parts.method, Method::GET);
        let _rug_ed_tests_rug_194_rrrruuuugggg_test_into_parts = 0;
    }
}
#[cfg(test)]
mod tests_rug_195 {
    use super::*;
    use crate::request::Request;
    use crate::Method;
    use crate::Uri;
    use crate::Version;
    #[test]
    fn test_map() {
        let _rug_st_tests_rug_195_rrrruuuugggg_test_map = 0;
        let mut p0: Request<Vec<u8>> = Request::new(Vec::new());
        let p1 = |b: Vec<u8>| -> Vec<u8> {
            debug_assert_eq!(b, Vec:: < u8 > ::new());
            b
        };
        p0.map(p1);
        let _rug_ed_tests_rug_195_rrrruuuugggg_test_map = 0;
    }
}
#[cfg(test)]
mod tests_rug_197 {
    use super::*;
    use crate::{Method, Uri, Version, HeaderMap, Extensions};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_197_rrrruuuugggg_test_rug = 0;
        let _ = <Method>::default();
        let _ = <Uri>::default();
        let _ = <Version>::default();
        let _ = <HeaderMap>::default();
        let _ = <Extensions>::default();
        let _ = Parts::new();
        let _rug_ed_tests_rug_197_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_198 {
    use super::*;
    use crate::request;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_198_rrrruuuugggg_test_rug = 0;
        request::Builder::new();
        let _rug_ed_tests_rug_198_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_199 {
    use super::*;
    use crate::request::Builder;
    #[test]
    fn test_method() {
        let _rug_st_tests_rug_199_rrrruuuugggg_test_method = 0;
        let rug_fuzz_0 = "POST";
        let mut p0: Builder = Builder::new();
        let p1: &str = rug_fuzz_0;
        p0.method(p1);
        let _rug_ed_tests_rug_199_rrrruuuugggg_test_method = 0;
    }
}
#[cfg(test)]
mod tests_rug_200 {
    use super::*;
    use crate::request::Builder;
    use crate::Uri;
    use std::convert::TryFrom;
    use crate::Error;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_200_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https://www.rust-lang.org/";
        let mut p0: Builder = Builder::new();
        let mut p1: &str = rug_fuzz_0;
        p0.uri(p1);
        let _rug_ed_tests_rug_200_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_201 {
    use super::*;
    use crate::request::Builder;
    use crate::header::{HeaderName, HeaderValue};
    use std::convert::TryFrom;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_201_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "Accept";
        let rug_fuzz_1 = "text/html";
        let mut p0: Builder = Builder::new();
        let p1: &str = rug_fuzz_0;
        let p2: &str = rug_fuzz_1;
        p0.header::<&str, &str>(p1, p2);
        let _rug_ed_tests_rug_201_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_202 {
    use super::*;
    use crate::Request;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_202_rrrruuuugggg_sample = 0;
        use crate::HeaderMap;
        #[cfg(test)]
        mod tests_rug_202_prepare {
            #[test]
            fn sample() {
                let _rug_st_tests_rug_202_prepare_rrrruuuugggg_sample = 0;
                let rug_fuzz_0 = 0;
                let rug_fuzz_1 = 0;
                let _rug_st_tests_rug_202_rrrruuuugggg_sample = rug_fuzz_0;
                use crate::request::Builder;
                let mut v29: Builder = Builder::new();
                let _rug_ed_tests_rug_202_rrrruuuugggg_sample = rug_fuzz_1;
                let _rug_ed_tests_rug_202_prepare_rrrruuuugggg_sample = 0;
            }
        }
        let req = Request::builder()
            .header("Accept", "text/html")
            .header("X-Custom-Foo", "bar");
        let headers = req.headers_ref().unwrap();
        assert_eq!(headers["Accept"], "text/html");
        assert_eq!(headers["X-Custom-Foo"], "bar");
        let _rug_ed_tests_rug_202_rrrruuuugggg_sample = 0;
    }
}
#[cfg(test)]
mod tests_rug_203 {
    use super::*;
    use crate::request::Builder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_203_rrrruuuugggg_test_rug = 0;
        let mut p0: Builder = Builder::new();
        let p1: () = ();
        p0.body(p1);
        let _rug_ed_tests_rug_203_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_204 {
    use super::*;
    use crate::request::Builder;
    use crate::request::Parts;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_204_rrrruuuugggg_test_rug = 0;
        let mut p0: Builder = Builder::new();
        let p1: fn(Parts) -> Result<Parts> = |parts| Ok(parts);
        p0.and_then(p1);
        let _rug_ed_tests_rug_204_rrrruuuugggg_test_rug = 0;
    }
}
