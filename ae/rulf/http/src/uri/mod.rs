//! URI component of request and response lines
//!
//! This module primarily contains the `Uri` type which is a component of all
//! HTTP requests and also reexports this type at the root of the crate. A URI
//! is not always a "full URL" in the sense of something you'd type into a web
//! browser, but HTTP requests may only have paths on servers but may have full
//! schemes and hostnames on clients.
//!
//! # Examples
//!
//! ```
//! use http::Uri;
//!
//! let uri = "/foo/bar?baz".parse::<Uri>().unwrap();
//! assert_eq!(uri.path(), "/foo/bar");
//! assert_eq!(uri.query(), Some("baz"));
//! assert_eq!(uri.host(), None);
//!
//! let uri = "https://www.rust-lang.org/install.html".parse::<Uri>().unwrap();
//! assert_eq!(uri.scheme_str(), Some("https"));
//! assert_eq!(uri.host(), Some("www.rust-lang.org"));
//! assert_eq!(uri.path(), "/install.html");
//! ```
use crate::byte_str::ByteStr;
use std::convert::TryFrom;
use bytes::Bytes;
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::str::{self, FromStr};
use std::{fmt, u16, u8};
use self::scheme::Scheme2;
pub use self::authority::Authority;
pub use self::builder::Builder;
pub use self::path::PathAndQuery;
pub use self::port::Port;
pub use self::scheme::Scheme;
mod authority;
mod builder;
mod path;
mod port;
mod scheme;
#[cfg(test)]
mod tests;
/// The URI component of a request.
///
/// For HTTP 1, this is included as part of the request line. From Section 5.3,
/// Request Target:
///
/// > Once an inbound connection is obtained, the client sends an HTTP
/// > request message (Section 3) with a request-target derived from the
/// > target URI.  There are four distinct formats for the request-target,
/// > depending on both the method being requested and whether the request
/// > is to a proxy.
/// >
/// > ```notrust
/// > request-target = origin-form
/// >                / absolute-form
/// >                / authority-form
/// >                / asterisk-form
/// > ```
///
/// The URI is structured as follows:
///
/// ```notrust
/// abc://username:password@example.com:123/path/data?key=value&key2=value2#fragid1
/// |-|   |-------------------------------||--------| |-------------------| |-----|
///  |                  |                       |               |              |
/// scheme          authority                 path            query         fragment
/// ```
///
/// For HTTP 2.0, the URI is encoded using pseudoheaders.
///
/// # Examples
///
/// ```
/// use http::Uri;
///
/// let uri = "/foo/bar?baz".parse::<Uri>().unwrap();
/// assert_eq!(uri.path(), "/foo/bar");
/// assert_eq!(uri.query(), Some("baz"));
/// assert_eq!(uri.host(), None);
///
/// let uri = "https://www.rust-lang.org/install.html".parse::<Uri>().unwrap();
/// assert_eq!(uri.scheme_str(), Some("https"));
/// assert_eq!(uri.host(), Some("www.rust-lang.org"));
/// assert_eq!(uri.path(), "/install.html");
/// ```
#[derive(Clone)]
pub struct Uri {
    scheme: Scheme,
    authority: Authority,
    path_and_query: PathAndQuery,
}
/// The various parts of a URI.
///
/// This struct is used to provide to and retrieve from a URI.
#[derive(Debug, Default)]
pub struct Parts {
    /// The scheme component of a URI
    pub scheme: Option<Scheme>,
    /// The authority component of a URI
    pub authority: Option<Authority>,
    /// The origin-form component of a URI
    pub path_and_query: Option<PathAndQuery>,
    /// Allow extending in the future
    _priv: (),
}
/// An error resulting from a failed attempt to construct a URI.
#[derive(Debug)]
pub struct InvalidUri(ErrorKind);
/// An error resulting from a failed attempt to construct a URI.
#[derive(Debug)]
pub struct InvalidUriParts(InvalidUri);
#[derive(Debug, Eq, PartialEq)]
enum ErrorKind {
    InvalidUriChar,
    InvalidScheme,
    InvalidAuthority,
    InvalidPort,
    InvalidFormat,
    SchemeMissing,
    AuthorityMissing,
    PathAndQueryMissing,
    TooLong,
    Empty,
    SchemeTooLong,
}
const MAX_LEN: usize = (u16::MAX - 1) as usize;
const URI_CHARS: [u8; 256] = [
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    b'!',
    0,
    b'#',
    b'$',
    0,
    b'&',
    b'\'',
    b'(',
    b')',
    b'*',
    b'+',
    b',',
    b'-',
    b'.',
    b'/',
    b'0',
    b'1',
    b'2',
    b'3',
    b'4',
    b'5',
    b'6',
    b'7',
    b'8',
    b'9',
    b':',
    b';',
    0,
    b'=',
    0,
    b'?',
    b'@',
    b'A',
    b'B',
    b'C',
    b'D',
    b'E',
    b'F',
    b'G',
    b'H',
    b'I',
    b'J',
    b'K',
    b'L',
    b'M',
    b'N',
    b'O',
    b'P',
    b'Q',
    b'R',
    b'S',
    b'T',
    b'U',
    b'V',
    b'W',
    b'X',
    b'Y',
    b'Z',
    b'[',
    0,
    b']',
    0,
    b'_',
    0,
    b'a',
    b'b',
    b'c',
    b'd',
    b'e',
    b'f',
    b'g',
    b'h',
    b'i',
    b'j',
    b'k',
    b'l',
    b'm',
    b'n',
    b'o',
    b'p',
    b'q',
    b'r',
    b's',
    b't',
    b'u',
    b'v',
    b'w',
    b'x',
    b'y',
    b'z',
    0,
    0,
    0,
    b'~',
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
];
impl Uri {
    /// Creates a new builder-style object to manufacture a `Uri`.
    ///
    /// This method returns an instance of `Builder` which can be usd to
    /// create a `Uri`.
    ///
    /// # Examples
    ///
    /// ```
    /// use http::Uri;
    ///
    /// let uri = Uri::builder()
    ///     .scheme("https")
    ///     .authority("hyper.rs")
    ///     .path_and_query("/")
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn builder() -> Builder {
        Builder::new()
    }
    /// Attempt to convert a `Uri` from `Parts`
    pub fn from_parts(src: Parts) -> Result<Uri, InvalidUriParts> {
        if src.scheme.is_some() {
            if src.authority.is_none() {
                return Err(ErrorKind::AuthorityMissing.into());
            }
            if src.path_and_query.is_none() {
                return Err(ErrorKind::PathAndQueryMissing.into());
            }
        } else {
            if src.authority.is_some() && src.path_and_query.is_some() {
                return Err(ErrorKind::SchemeMissing.into());
            }
        }
        let scheme = match src.scheme {
            Some(scheme) => scheme,
            None => Scheme { inner: Scheme2::None },
        };
        let authority = match src.authority {
            Some(authority) => authority,
            None => Authority::empty(),
        };
        let path_and_query = match src.path_and_query {
            Some(path_and_query) => path_and_query,
            None => PathAndQuery::empty(),
        };
        Ok(Uri {
            scheme: scheme,
            authority: authority,
            path_and_query: path_and_query,
        })
    }
    /// Attempt to convert a `Bytes` buffer to a `Uri`.
    ///
    /// This will try to prevent a copy if the type passed is the type used
    /// internally, and will copy the data if it is not.
    pub fn from_maybe_shared<T>(src: T) -> Result<Self, InvalidUri>
    where
        T: AsRef<[u8]> + 'static,
    {
        if_downcast_into!(T, Bytes, src, { return Uri::from_shared(src); });
        Uri::try_from(src.as_ref())
    }
    fn from_shared(s: Bytes) -> Result<Uri, InvalidUri> {
        use self::ErrorKind::*;
        if s.len() > MAX_LEN {
            return Err(TooLong.into());
        }
        match s.len() {
            0 => {
                return Err(Empty.into());
            }
            1 => {
                match s[0] {
                    b'/' => {
                        return Ok(Uri {
                            scheme: Scheme::empty(),
                            authority: Authority::empty(),
                            path_and_query: PathAndQuery::slash(),
                        });
                    }
                    b'*' => {
                        return Ok(Uri {
                            scheme: Scheme::empty(),
                            authority: Authority::empty(),
                            path_and_query: PathAndQuery::star(),
                        });
                    }
                    _ => {
                        let authority = Authority::from_shared(s)?;
                        return Ok(Uri {
                            scheme: Scheme::empty(),
                            authority: authority,
                            path_and_query: PathAndQuery::empty(),
                        });
                    }
                }
            }
            _ => {}
        }
        if s[0] == b'/' {
            return Ok(Uri {
                scheme: Scheme::empty(),
                authority: Authority::empty(),
                path_and_query: PathAndQuery::from_shared(s)?,
            });
        }
        parse_full(s)
    }
    /// Convert a `Uri` from a static string.
    ///
    /// This function will not perform any copying, however the string is
    /// checked to ensure that it is valid.
    ///
    /// # Panics
    ///
    /// This function panics if the argument is an invalid URI.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::uri::Uri;
    /// let uri = Uri::from_static("http://example.com/foo");
    ///
    /// assert_eq!(uri.host().unwrap(), "example.com");
    /// assert_eq!(uri.path(), "/foo");
    /// ```
    pub fn from_static(src: &'static str) -> Self {
        let s = Bytes::from_static(src.as_bytes());
        match Uri::from_shared(s) {
            Ok(uri) => uri,
            Err(e) => panic!("static str is not valid URI: {}", e),
        }
    }
    /// Convert a `Uri` into `Parts`.
    ///
    /// # Note
    ///
    /// This is just an inherent method providing the same functionality as
    /// `let parts: Parts = uri.into()`
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::uri::*;
    /// let uri: Uri = "/foo".parse().unwrap();
    ///
    /// let parts = uri.into_parts();
    ///
    /// assert_eq!(parts.path_and_query.unwrap(), "/foo");
    ///
    /// assert!(parts.scheme.is_none());
    /// assert!(parts.authority.is_none());
    /// ```
    #[inline]
    pub fn into_parts(self) -> Parts {
        self.into()
    }
    /// Returns the path & query components of the Uri
    #[inline]
    pub fn path_and_query(&self) -> Option<&PathAndQuery> {
        if !self.scheme.inner.is_none() || self.authority.data.is_empty() {
            Some(&self.path_and_query)
        } else {
            None
        }
    }
    /// Get the path of this `Uri`.
    ///
    /// Both relative and absolute URIs contain a path component, though it
    /// might be the empty string. The path component is **case sensitive**.
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
    /// A relative URI
    ///
    /// ```
    /// # use http::Uri;
    ///
    /// let uri: Uri = "/hello/world".parse().unwrap();
    ///
    /// assert_eq!(uri.path(), "/hello/world");
    /// ```
    ///
    /// An absolute URI
    ///
    /// ```
    /// # use http::Uri;
    /// let uri: Uri = "http://example.org/hello/world".parse().unwrap();
    ///
    /// assert_eq!(uri.path(), "/hello/world");
    /// ```
    #[inline]
    pub fn path(&self) -> &str {
        if self.has_path() { self.path_and_query.path() } else { "" }
    }
    /// Get the scheme of this `Uri`.
    ///
    /// The URI scheme refers to a specification for assigning identifiers
    /// within that scheme. Only absolute URIs contain a scheme component, but
    /// not all absolute URIs will contain a scheme component.  Although scheme
    /// names are case-insensitive, the canonical form is lowercase.
    ///
    /// ```notrust
    /// abc://username:password@example.com:123/path/data?key=value&key2=value2#fragid1
    /// |-|
    ///  |
    /// scheme
    /// ```
    ///
    /// # Examples
    ///
    /// Absolute URI
    ///
    /// ```
    /// use http::uri::{Scheme, Uri};
    ///
    /// let uri: Uri = "http://example.org/hello/world".parse().unwrap();
    ///
    /// assert_eq!(uri.scheme(), Some(&Scheme::HTTP));
    /// ```
    ///
    ///
    /// Relative URI
    ///
    /// ```
    /// # use http::Uri;
    /// let uri: Uri = "/hello/world".parse().unwrap();
    ///
    /// assert!(uri.scheme().is_none());
    /// ```
    #[inline]
    pub fn scheme(&self) -> Option<&Scheme> {
        if self.scheme.inner.is_none() { None } else { Some(&self.scheme) }
    }
    /// Get the scheme of this `Uri` as a `&str`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::Uri;
    /// let uri: Uri = "http://example.org/hello/world".parse().unwrap();
    ///
    /// assert_eq!(uri.scheme_str(), Some("http"));
    /// ```
    #[inline]
    pub fn scheme_str(&self) -> Option<&str> {
        if self.scheme.inner.is_none() { None } else { Some(self.scheme.as_str()) }
    }
    /// Get the authority of this `Uri`.
    ///
    /// The authority is a hierarchical element for naming authority such that
    /// the remainder of the URI is delegated to that authority. For HTTP, the
    /// authority consists of the host and port. The host portion of the
    /// authority is **case-insensitive**.
    ///
    /// The authority also includes a `username:password` component, however
    /// the use of this is deprecated and should be avoided.
    ///
    /// ```notrust
    /// abc://username:password@example.com:123/path/data?key=value&key2=value2#fragid1
    ///       |-------------------------------|
    ///                     |
    ///                 authority
    /// ```
    ///
    /// This function will be renamed to `authority` in the next semver release.
    ///
    /// # Examples
    ///
    /// Absolute URI
    ///
    /// ```
    /// # use http::Uri;
    /// let uri: Uri = "http://example.org:80/hello/world".parse().unwrap();
    ///
    /// assert_eq!(uri.authority().map(|a| a.as_str()), Some("example.org:80"));
    /// ```
    ///
    ///
    /// Relative URI
    ///
    /// ```
    /// # use http::Uri;
    /// let uri: Uri = "/hello/world".parse().unwrap();
    ///
    /// assert!(uri.authority().is_none());
    /// ```
    #[inline]
    pub fn authority(&self) -> Option<&Authority> {
        if self.authority.data.is_empty() { None } else { Some(&self.authority) }
    }
    /// Get the host of this `Uri`.
    ///
    /// The host subcomponent of authority is identified by an IP literal
    /// encapsulated within square brackets, an IPv4 address in dotted- decimal
    /// form, or a registered name.  The host subcomponent is **case-insensitive**.
    ///
    /// ```notrust
    /// abc://username:password@example.com:123/path/data?key=value&key2=value2#fragid1
    ///                         |---------|
    ///                              |
    ///                             host
    /// ```
    ///
    /// # Examples
    ///
    /// Absolute URI
    ///
    /// ```
    /// # use http::Uri;
    /// let uri: Uri = "http://example.org:80/hello/world".parse().unwrap();
    ///
    /// assert_eq!(uri.host(), Some("example.org"));
    /// ```
    ///
    ///
    /// Relative URI
    ///
    /// ```
    /// # use http::Uri;
    /// let uri: Uri = "/hello/world".parse().unwrap();
    ///
    /// assert!(uri.host().is_none());
    /// ```
    #[inline]
    pub fn host(&self) -> Option<&str> {
        self.authority().map(|a| a.host())
    }
    /// Get the port part of this `Uri`.
    ///
    /// The port subcomponent of authority is designated by an optional port
    /// number following the host and delimited from it by a single colon (":")
    /// character. It can be turned into a decimal port number with the `as_u16`
    /// method or as a `str` with the `as_str` method.
    ///
    /// ```notrust
    /// abc://username:password@example.com:123/path/data?key=value&key2=value2#fragid1
    ///                                     |-|
    ///                                      |
    ///                                     port
    /// ```
    ///
    /// # Examples
    ///
    /// Absolute URI with port
    ///
    /// ```
    /// # use http::Uri;
    /// let uri: Uri = "http://example.org:80/hello/world".parse().unwrap();
    ///
    /// let port = uri.port().unwrap();
    /// assert_eq!(port.as_u16(), 80);
    /// ```
    ///
    /// Absolute URI without port
    ///
    /// ```
    /// # use http::Uri;
    /// let uri: Uri = "http://example.org/hello/world".parse().unwrap();
    ///
    /// assert!(uri.port().is_none());
    /// ```
    ///
    /// Relative URI
    ///
    /// ```
    /// # use http::Uri;
    /// let uri: Uri = "/hello/world".parse().unwrap();
    ///
    /// assert!(uri.port().is_none());
    /// ```
    pub fn port(&self) -> Option<Port<&str>> {
        self.authority().and_then(|a| a.port())
    }
    /// Get the port of this `Uri` as a `u16`.
    ///
    ///
    /// # Example
    ///
    /// ```
    /// # use http::{Uri, uri::Port};
    /// let uri: Uri = "http://example.org:80/hello/world".parse().unwrap();
    ///
    /// assert_eq!(uri.port_u16(), Some(80));
    /// ```
    pub fn port_u16(&self) -> Option<u16> {
        self.port().and_then(|p| Some(p.as_u16()))
    }
    /// Get the query string of this `Uri`, starting after the `?`.
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
    /// Absolute URI
    ///
    /// ```
    /// # use http::Uri;
    /// let uri: Uri = "http://example.org/hello/world?key=value".parse().unwrap();
    ///
    /// assert_eq!(uri.query(), Some("key=value"));
    /// ```
    ///
    /// Relative URI with a query string component
    ///
    /// ```
    /// # use http::Uri;
    /// let uri: Uri = "/hello/world?key=value&foo=bar".parse().unwrap();
    ///
    /// assert_eq!(uri.query(), Some("key=value&foo=bar"));
    /// ```
    ///
    /// Relative URI without a query string component
    ///
    /// ```
    /// # use http::Uri;
    /// let uri: Uri = "/hello/world".parse().unwrap();
    ///
    /// assert!(uri.query().is_none());
    /// ```
    #[inline]
    pub fn query(&self) -> Option<&str> {
        self.path_and_query.query()
    }
    fn has_path(&self) -> bool {
        !self.path_and_query.data.is_empty() || !self.scheme.inner.is_none()
    }
}
impl<'a> TryFrom<&'a [u8]> for Uri {
    type Error = InvalidUri;
    #[inline]
    fn try_from(t: &'a [u8]) -> Result<Self, Self::Error> {
        Uri::from_shared(Bytes::copy_from_slice(t))
    }
}
impl<'a> TryFrom<&'a str> for Uri {
    type Error = InvalidUri;
    #[inline]
    fn try_from(t: &'a str) -> Result<Self, Self::Error> {
        t.parse()
    }
}
impl<'a> TryFrom<&'a String> for Uri {
    type Error = InvalidUri;
    #[inline]
    fn try_from(t: &'a String) -> Result<Self, Self::Error> {
        t.parse()
    }
}
impl TryFrom<String> for Uri {
    type Error = InvalidUri;
    #[inline]
    fn try_from(t: String) -> Result<Self, Self::Error> {
        Uri::from_shared(Bytes::from(t))
    }
}
impl TryFrom<Parts> for Uri {
    type Error = InvalidUriParts;
    #[inline]
    fn try_from(src: Parts) -> Result<Self, Self::Error> {
        Uri::from_parts(src)
    }
}
impl<'a> TryFrom<&'a Uri> for Uri {
    type Error = crate::Error;
    #[inline]
    fn try_from(src: &'a Uri) -> Result<Self, Self::Error> {
        Ok(src.clone())
    }
}
/// Convert a `Uri` from parts
///
/// # Examples
///
/// Relative URI
///
/// ```
/// # use http::uri::*;
/// let mut parts = Parts::default();
/// parts.path_and_query = Some("/foo".parse().unwrap());
///
/// let uri = Uri::from_parts(parts).unwrap();
///
/// assert_eq!(uri.path(), "/foo");
///
/// assert!(uri.scheme().is_none());
/// assert!(uri.authority().is_none());
/// ```
///
/// Absolute URI
///
/// ```
/// # use http::uri::*;
/// let mut parts = Parts::default();
/// parts.scheme = Some("http".parse().unwrap());
/// parts.authority = Some("foo.com".parse().unwrap());
/// parts.path_and_query = Some("/foo".parse().unwrap());
///
/// let uri = Uri::from_parts(parts).unwrap();
///
/// assert_eq!(uri.scheme().unwrap().as_str(), "http");
/// assert_eq!(uri.authority().unwrap(), "foo.com");
/// assert_eq!(uri.path(), "/foo");
/// ```
impl From<Uri> for Parts {
    fn from(src: Uri) -> Self {
        let path_and_query = if src.has_path() {
            Some(src.path_and_query)
        } else {
            None
        };
        let scheme = match src.scheme.inner {
            Scheme2::None => None,
            _ => Some(src.scheme),
        };
        let authority = if src.authority.data.is_empty() {
            None
        } else {
            Some(src.authority)
        };
        Parts {
            scheme: scheme,
            authority: authority,
            path_and_query: path_and_query,
            _priv: (),
        }
    }
}
fn parse_full(mut s: Bytes) -> Result<Uri, InvalidUri> {
    let scheme = match Scheme2::parse(&s[..])? {
        Scheme2::None => Scheme2::None,
        Scheme2::Standard(p) => {
            let _ = s.split_to(p.len() + 3);
            Scheme2::Standard(p)
        }
        Scheme2::Other(n) => {
            let mut scheme = s.split_to(n + 3);
            let _ = scheme.split_off(n);
            let val = unsafe { ByteStr::from_utf8_unchecked(scheme) };
            Scheme2::Other(Box::new(val))
        }
    };
    let authority_end = Authority::parse(&s[..])?;
    if scheme.is_none() {
        if authority_end != s.len() {
            return Err(ErrorKind::InvalidFormat.into());
        }
        let authority = Authority {
            data: unsafe { ByteStr::from_utf8_unchecked(s) },
        };
        return Ok(Uri {
            scheme: scheme.into(),
            authority: authority,
            path_and_query: PathAndQuery::empty(),
        });
    }
    if authority_end == 0 {
        return Err(ErrorKind::InvalidFormat.into());
    }
    let authority = s.split_to(authority_end);
    let authority = Authority {
        data: unsafe { ByteStr::from_utf8_unchecked(authority) },
    };
    Ok(Uri {
        scheme: scheme.into(),
        authority: authority,
        path_and_query: PathAndQuery::from_shared(s)?,
    })
}
impl FromStr for Uri {
    type Err = InvalidUri;
    #[inline]
    fn from_str(s: &str) -> Result<Uri, InvalidUri> {
        Uri::try_from(s.as_bytes())
    }
}
impl PartialEq for Uri {
    fn eq(&self, other: &Uri) -> bool {
        if self.scheme() != other.scheme() {
            return false;
        }
        if self.authority() != other.authority() {
            return false;
        }
        if self.path() != other.path() {
            return false;
        }
        if self.query() != other.query() {
            return false;
        }
        true
    }
}
impl PartialEq<str> for Uri {
    fn eq(&self, other: &str) -> bool {
        let mut other = other.as_bytes();
        let mut absolute = false;
        if let Some(scheme) = self.scheme() {
            let scheme = scheme.as_str().as_bytes();
            absolute = true;
            if other.len() < scheme.len() + 3 {
                return false;
            }
            if !scheme.eq_ignore_ascii_case(&other[..scheme.len()]) {
                return false;
            }
            other = &other[scheme.len()..];
            if &other[..3] != b"://" {
                return false;
            }
            other = &other[3..];
        }
        if let Some(auth) = self.authority() {
            let len = auth.data.len();
            absolute = true;
            if other.len() < len {
                return false;
            }
            if !auth.data.as_bytes().eq_ignore_ascii_case(&other[..len]) {
                return false;
            }
            other = &other[len..];
        }
        let path = self.path();
        if other.len() < path.len() || path.as_bytes() != &other[..path.len()] {
            if absolute && path == "/" {} else {
                return false;
            }
        } else {
            other = &other[path.len()..];
        }
        if let Some(query) = self.query() {
            if other.len() == 0 {
                return query.len() == 0;
            }
            if other[0] != b'?' {
                return false;
            }
            other = &other[1..];
            if other.len() < query.len() {
                return false;
            }
            if query.as_bytes() != &other[..query.len()] {
                return false;
            }
            other = &other[query.len()..];
        }
        other.is_empty() || other[0] == b'#'
    }
}
impl PartialEq<Uri> for str {
    fn eq(&self, uri: &Uri) -> bool {
        uri == self
    }
}
impl<'a> PartialEq<&'a str> for Uri {
    fn eq(&self, other: &&'a str) -> bool {
        self == *other
    }
}
impl<'a> PartialEq<Uri> for &'a str {
    fn eq(&self, uri: &Uri) -> bool {
        uri == *self
    }
}
impl Eq for Uri {}
/// Returns a `Uri` representing `/`
impl Default for Uri {
    #[inline]
    fn default() -> Uri {
        Uri {
            scheme: Scheme::empty(),
            authority: Authority::empty(),
            path_and_query: PathAndQuery::slash(),
        }
    }
}
impl fmt::Display for Uri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(scheme) = self.scheme() {
            write!(f, "{}://", scheme)?;
        }
        if let Some(authority) = self.authority() {
            write!(f, "{}", authority)?;
        }
        write!(f, "{}", self.path())?;
        if let Some(query) = self.query() {
            write!(f, "?{}", query)?;
        }
        Ok(())
    }
}
impl fmt::Debug for Uri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl From<ErrorKind> for InvalidUri {
    fn from(src: ErrorKind) -> InvalidUri {
        InvalidUri(src)
    }
}
impl From<ErrorKind> for InvalidUriParts {
    fn from(src: ErrorKind) -> InvalidUriParts {
        InvalidUriParts(src.into())
    }
}
impl InvalidUri {
    fn s(&self) -> &str {
        match self.0 {
            ErrorKind::InvalidUriChar => "invalid uri character",
            ErrorKind::InvalidScheme => "invalid scheme",
            ErrorKind::InvalidAuthority => "invalid authority",
            ErrorKind::InvalidPort => "invalid port",
            ErrorKind::InvalidFormat => "invalid format",
            ErrorKind::SchemeMissing => "scheme missing",
            ErrorKind::AuthorityMissing => "authority missing",
            ErrorKind::PathAndQueryMissing => "path missing",
            ErrorKind::TooLong => "uri too long",
            ErrorKind::Empty => "empty string",
            ErrorKind::SchemeTooLong => "scheme too long",
        }
    }
}
impl fmt::Display for InvalidUri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.s().fmt(f)
    }
}
impl Error for InvalidUri {}
impl fmt::Display for InvalidUriParts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl Error for InvalidUriParts {}
impl Hash for Uri {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        if !self.scheme.inner.is_none() {
            self.scheme.hash(state);
            state.write_u8(0xff);
        }
        if let Some(auth) = self.authority() {
            auth.hash(state);
        }
        Hash::hash_slice(self.path().as_bytes(), state);
        if let Some(query) = self.query() {
            b'?'.hash(state);
            Hash::hash_slice(query.as_bytes(), state);
        }
    }
}
#[cfg(test)]
mod tests_llm_16_235 {
    use super::*;
    use crate::*;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_235_rrrruuuugggg_test_eq = 0;
        let _rug_ed_tests_llm_16_235_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_236 {
    use super::*;
    use crate::*;
    use crate::uri::Authority;
    use crate::Uri;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_236_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "http://example.com/hello/world";
        let rug_fuzz_1 = "http://example.com/hello/world";
        let uri1 = Uri::from_static(rug_fuzz_0);
        let uri2 = Uri::from_static(rug_fuzz_1);
        debug_assert_eq!(uri1.eq(& uri2), true);
        let _rug_ed_tests_llm_16_236_rrrruuuugggg_test_eq = 0;
    }
    #[test]
    fn test_eq_none() {
        let _rug_st_tests_llm_16_236_rrrruuuugggg_test_eq_none = 0;
        let uri1 = Uri::default();
        let uri2 = Uri::default();
        debug_assert_eq!(uri1.eq(& uri2), true);
        let _rug_ed_tests_llm_16_236_rrrruuuugggg_test_eq_none = 0;
    }
    #[test]
    fn test_eq_scheme() {
        let _rug_st_tests_llm_16_236_rrrruuuugggg_test_eq_scheme = 0;
        let rug_fuzz_0 = "http://example.com/hello/world";
        let rug_fuzz_1 = "https://example.com/hello/world";
        let uri1 = Uri::from_static(rug_fuzz_0);
        let uri2 = Uri::from_static(rug_fuzz_1);
        debug_assert_eq!(uri1.eq(& uri2), false);
        let _rug_ed_tests_llm_16_236_rrrruuuugggg_test_eq_scheme = 0;
    }
    #[test]
    fn test_eq_authority() {
        let _rug_st_tests_llm_16_236_rrrruuuugggg_test_eq_authority = 0;
        let rug_fuzz_0 = "http://example1.com/hello/world";
        let rug_fuzz_1 = "http://example2.com/hello/world";
        let uri1 = Uri::from_static(rug_fuzz_0);
        let uri2 = Uri::from_static(rug_fuzz_1);
        debug_assert_eq!(uri1.eq(& uri2), false);
        let _rug_ed_tests_llm_16_236_rrrruuuugggg_test_eq_authority = 0;
    }
    #[test]
    fn test_eq_path() {
        let _rug_st_tests_llm_16_236_rrrruuuugggg_test_eq_path = 0;
        let rug_fuzz_0 = "http://example.com/hello/world1";
        let rug_fuzz_1 = "http://example.com/hello/world2";
        let uri1 = Uri::from_static(rug_fuzz_0);
        let uri2 = Uri::from_static(rug_fuzz_1);
        debug_assert_eq!(uri1.eq(& uri2), false);
        let _rug_ed_tests_llm_16_236_rrrruuuugggg_test_eq_path = 0;
    }
    #[test]
    fn test_eq_query() {
        let _rug_st_tests_llm_16_236_rrrruuuugggg_test_eq_query = 0;
        let rug_fuzz_0 = "http://example.com/hello/world?key1=value1";
        let rug_fuzz_1 = "http://example.com/hello/world?key2=value2";
        let uri1 = Uri::from_static(rug_fuzz_0);
        let uri2 = Uri::from_static(rug_fuzz_1);
        debug_assert_eq!(uri1.eq(& uri2), false);
        let _rug_ed_tests_llm_16_236_rrrruuuugggg_test_eq_query = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_238_llm_16_237 {
    use std::convert::TryFrom;
    use crate::uri::{Uri, InvalidUri};
    use bytes::Bytes;
    #[test]
    fn test_try_from() {
        let _rug_st_tests_llm_16_238_llm_16_237_rrrruuuugggg_test_try_from = 0;
        let rug_fuzz_0 = b"https://www.example.com";
        let input: &[u8] = rug_fuzz_0;
        let result: Result<Uri, InvalidUri> = Uri::try_from(input);
        debug_assert!(result.is_ok());
        debug_assert_eq!(
            result.unwrap(), Uri::from_shared(Bytes::copy_from_slice(input)).unwrap()
        );
        let _rug_ed_tests_llm_16_238_llm_16_237_rrrruuuugggg_test_try_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_239 {
    use std::error::Error;
    use std::convert::TryFrom;
    use std::string::String;
    use crate::uri::Uri;
    #[test]
    fn test_try_from() {
        let _rug_st_tests_llm_16_239_rrrruuuugggg_test_try_from = 0;
        let rug_fuzz_0 = "https://www.example.com";
        let uri_string: &'static str = rug_fuzz_0;
        let uri = Uri::try_from(&String::from(uri_string)).unwrap();
        debug_assert_eq!(uri, Uri::from_static(uri_string));
        let _rug_ed_tests_llm_16_239_rrrruuuugggg_test_try_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_240 {
    use std::convert::TryFrom;
    use crate::uri::Uri;
    #[test]
    fn test_try_from() {
        let _rug_st_tests_llm_16_240_rrrruuuugggg_test_try_from = 0;
        let rug_fuzz_0 = "http://example.com";
        let uri_str = rug_fuzz_0;
        let uri: Result<Uri, _> = <Uri as TryFrom<&str>>::try_from(uri_str);
        debug_assert!(uri.is_ok());
        let _rug_ed_tests_llm_16_240_rrrruuuugggg_test_try_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_241 {
    use super::*;
    use crate::*;
    use crate::uri::Uri;
    use std::convert::TryFrom;
    #[test]
    fn test_try_from_uri() {
        let _rug_st_tests_llm_16_241_rrrruuuugggg_test_try_from_uri = 0;
        let rug_fuzz_0 = "http";
        let rug_fuzz_1 = "example.com";
        let rug_fuzz_2 = "/test";
        let uri = Uri::builder()
            .scheme(rug_fuzz_0)
            .authority(rug_fuzz_1)
            .path_and_query(rug_fuzz_2)
            .build()
            .unwrap();
        let result = <Uri as std::convert::TryFrom<&Uri>>::try_from(&uri);
        debug_assert_eq!(result.unwrap().scheme_str().unwrap(), "http");
        let _rug_ed_tests_llm_16_241_rrrruuuugggg_test_try_from_uri = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_242 {
    use crate::uri::Uri;
    use std::convert::TryFrom;
    use bytes::Bytes;
    #[test]
    fn test_try_from() {
        let _rug_st_tests_llm_16_242_rrrruuuugggg_test_try_from = 0;
        let rug_fuzz_0 = "http://example.com";
        let uri_string = String::from(rug_fuzz_0);
        let result = Uri::try_from(uri_string);
        debug_assert_eq!(result.is_ok(), true);
        let uri = result.unwrap();
        debug_assert_eq!(uri.scheme_str(), Some("http"));
        debug_assert_eq!(uri.host(), Some("example.com"));
        let _rug_ed_tests_llm_16_242_rrrruuuugggg_test_try_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_541 {
    use crate::uri::*;
    #[test]
    fn test_eq_function() {
        let _rug_st_tests_llm_16_541_rrrruuuugggg_test_eq_function = 0;
        let rug_fuzz_0 = "http://example.org/path?key=value";
        let uri: Uri = rug_fuzz_0.parse().unwrap();
        let eq = uri.eq(&uri);
        debug_assert_eq!(eq, true);
        let _rug_ed_tests_llm_16_541_rrrruuuugggg_test_eq_function = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_542 {
    use super::*;
    use crate::*;
    #[test]
    fn test_s() {
        let _rug_st_tests_llm_16_542_rrrruuuugggg_test_s = 0;
        let error_kind = ErrorKind::InvalidUriChar;
        let invalid_uri = InvalidUri(error_kind);
        debug_assert_eq!(invalid_uri.s(), "invalid uri character");
        let error_kind = ErrorKind::InvalidScheme;
        let invalid_uri = InvalidUri(error_kind);
        debug_assert_eq!(invalid_uri.s(), "invalid scheme");
        let error_kind = ErrorKind::InvalidAuthority;
        let invalid_uri = InvalidUri(error_kind);
        debug_assert_eq!(invalid_uri.s(), "invalid authority");
        let error_kind = ErrorKind::InvalidPort;
        let invalid_uri = InvalidUri(error_kind);
        debug_assert_eq!(invalid_uri.s(), "invalid port");
        let error_kind = ErrorKind::InvalidFormat;
        let invalid_uri = InvalidUri(error_kind);
        debug_assert_eq!(invalid_uri.s(), "invalid format");
        let error_kind = ErrorKind::SchemeMissing;
        let invalid_uri = InvalidUri(error_kind);
        debug_assert_eq!(invalid_uri.s(), "scheme missing");
        let error_kind = ErrorKind::AuthorityMissing;
        let invalid_uri = InvalidUri(error_kind);
        debug_assert_eq!(invalid_uri.s(), "authority missing");
        let error_kind = ErrorKind::PathAndQueryMissing;
        let invalid_uri = InvalidUri(error_kind);
        debug_assert_eq!(invalid_uri.s(), "path missing");
        let error_kind = ErrorKind::TooLong;
        let invalid_uri = InvalidUri(error_kind);
        debug_assert_eq!(invalid_uri.s(), "uri too long");
        let error_kind = ErrorKind::Empty;
        let invalid_uri = InvalidUri(error_kind);
        debug_assert_eq!(invalid_uri.s(), "empty string");
        let error_kind = ErrorKind::SchemeTooLong;
        let invalid_uri = InvalidUri(error_kind);
        debug_assert_eq!(invalid_uri.s(), "scheme too long");
        let _rug_ed_tests_llm_16_542_rrrruuuugggg_test_s = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_544 {
    use crate::Uri;
    #[test]
    fn test_builder() {
        let _rug_st_tests_llm_16_544_rrrruuuugggg_test_builder = 0;
        let rug_fuzz_0 = "https";
        let rug_fuzz_1 = "hyper.rs";
        let rug_fuzz_2 = "/";
        let uri = Uri::builder()
            .scheme(rug_fuzz_0)
            .authority(rug_fuzz_1)
            .path_and_query(rug_fuzz_2)
            .build()
            .unwrap();
        let _rug_ed_tests_llm_16_544_rrrruuuugggg_test_builder = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_548 {
    use bytes::Bytes;
    use crate::uri::{Authority, InvalidUri, PathAndQuery, Scheme, Uri};
    #[test]
    fn test_from_shared_empty_string() {
        let _rug_st_tests_llm_16_548_rrrruuuugggg_test_from_shared_empty_string = 0;
        let rug_fuzz_0 = "";
        let s = Bytes::from(rug_fuzz_0);
        let result = Uri::from_shared(s);
        debug_assert!(result.is_err());
        debug_assert_eq!(result.unwrap_err().to_string(), "provided URI is empty");
        let _rug_ed_tests_llm_16_548_rrrruuuugggg_test_from_shared_empty_string = 0;
    }
    #[test]
    fn test_from_shared_slash() {
        let _rug_st_tests_llm_16_548_rrrruuuugggg_test_from_shared_slash = 0;
        let rug_fuzz_0 = "/";
        let s = Bytes::from(rug_fuzz_0);
        let result = Uri::from_shared(s);
        debug_assert!(result.is_ok());
        let uri = result.unwrap();
        debug_assert_eq!(uri.scheme, Scheme::empty());
        debug_assert_eq!(uri.authority, Authority::empty());
        debug_assert_eq!(uri.path_and_query, PathAndQuery::slash());
        let _rug_ed_tests_llm_16_548_rrrruuuugggg_test_from_shared_slash = 0;
    }
    #[test]
    fn test_from_shared_star() {
        let _rug_st_tests_llm_16_548_rrrruuuugggg_test_from_shared_star = 0;
        let rug_fuzz_0 = "*";
        let s = Bytes::from(rug_fuzz_0);
        let result = Uri::from_shared(s);
        debug_assert!(result.is_ok());
        let uri = result.unwrap();
        debug_assert_eq!(uri.scheme, Scheme::empty());
        debug_assert_eq!(uri.authority, Authority::empty());
        debug_assert_eq!(uri.path_and_query, PathAndQuery::star());
        let _rug_ed_tests_llm_16_548_rrrruuuugggg_test_from_shared_star = 0;
    }
    #[test]
    fn test_from_shared_authority() {
        let _rug_st_tests_llm_16_548_rrrruuuugggg_test_from_shared_authority = 0;
        let rug_fuzz_0 = "example.com";
        let s = Bytes::from(rug_fuzz_0);
        let result = Uri::from_shared(s);
        debug_assert!(result.is_ok());
        let uri = result.unwrap();
        debug_assert_eq!(uri.scheme, Scheme::empty());
        debug_assert_eq!(
            uri.authority, Authority::from_shared("example.com".into()).unwrap()
        );
        debug_assert_eq!(uri.path_and_query, PathAndQuery::empty());
        let _rug_ed_tests_llm_16_548_rrrruuuugggg_test_from_shared_authority = 0;
    }
    #[test]
    fn test_from_shared_starting_with_slash() {
        let _rug_st_tests_llm_16_548_rrrruuuugggg_test_from_shared_starting_with_slash = 0;
        let rug_fuzz_0 = "/path?query";
        let s = Bytes::from(rug_fuzz_0);
        let result = Uri::from_shared(s);
        debug_assert!(result.is_ok());
        let uri = result.unwrap();
        debug_assert_eq!(uri.scheme, Scheme::empty());
        debug_assert_eq!(uri.authority, Authority::empty());
        debug_assert_eq!(
            uri.path_and_query, PathAndQuery::from_shared("/path?query".into()).unwrap()
        );
        let _rug_ed_tests_llm_16_548_rrrruuuugggg_test_from_shared_starting_with_slash = 0;
    }
}
#[test]
fn test_from_static() {
    assert_eq!(
        Uri::from_static("http://example.com/foo").host().unwrap(), "example.com"
    );
    assert_eq!(Uri::from_static("http://example.com/foo").path(), "/foo");
}
#[cfg(test)]
mod tests_llm_16_550 {
    use super::*;
    use crate::*;
    #[test]
    fn test_has_path_with_path_and_query_empty() {
        let _rug_st_tests_llm_16_550_rrrruuuugggg_test_has_path_with_path_and_query_empty = 0;
        let rug_fuzz_0 = "example.com";
        let uri = Uri {
            scheme: Scheme::HTTP,
            authority: Authority::from_static(rug_fuzz_0),
            path_and_query: PathAndQuery::empty(),
        };
        debug_assert_eq!(uri.has_path(), false);
        let _rug_ed_tests_llm_16_550_rrrruuuugggg_test_has_path_with_path_and_query_empty = 0;
    }
    #[test]
    fn test_has_path_with_path_and_query_not_empty() {
        let _rug_st_tests_llm_16_550_rrrruuuugggg_test_has_path_with_path_and_query_not_empty = 0;
        let rug_fuzz_0 = "example.com";
        let rug_fuzz_1 = "/path";
        let uri = Uri {
            scheme: Scheme::HTTP,
            authority: Authority::from_static(rug_fuzz_0),
            path_and_query: PathAndQuery::from_static(rug_fuzz_1),
        };
        debug_assert_eq!(uri.has_path(), true);
        let _rug_ed_tests_llm_16_550_rrrruuuugggg_test_has_path_with_path_and_query_not_empty = 0;
    }
    #[test]
    fn test_has_path_with_scheme_not_empty_and_path_and_query_empty() {
        let _rug_st_tests_llm_16_550_rrrruuuugggg_test_has_path_with_scheme_not_empty_and_path_and_query_empty = 0;
        let uri = Uri {
            scheme: Scheme::HTTP,
            authority: Authority::empty(),
            path_and_query: PathAndQuery::empty(),
        };
        debug_assert_eq!(uri.has_path(), true);
        let _rug_ed_tests_llm_16_550_rrrruuuugggg_test_has_path_with_scheme_not_empty_and_path_and_query_empty = 0;
    }
    #[test]
    fn test_has_path_with_scheme_not_empty_and_path_and_query_not_empty() {
        let _rug_st_tests_llm_16_550_rrrruuuugggg_test_has_path_with_scheme_not_empty_and_path_and_query_not_empty = 0;
        let rug_fuzz_0 = "/path";
        let uri = Uri {
            scheme: Scheme::HTTP,
            authority: Authority::empty(),
            path_and_query: PathAndQuery::from_static(rug_fuzz_0),
        };
        debug_assert_eq!(uri.has_path(), true);
        let _rug_ed_tests_llm_16_550_rrrruuuugggg_test_has_path_with_scheme_not_empty_and_path_and_query_not_empty = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_551 {
    use super::*;
    use crate::*;
    use crate::uri::scheme::Protocol::*;
    #[test]
    fn test_host_with_absolute_uri() {
        let _rug_st_tests_llm_16_551_rrrruuuugggg_test_host_with_absolute_uri = 0;
        let rug_fuzz_0 = "http://example.org:80/hello/world";
        let uri: Uri = rug_fuzz_0.parse().unwrap();
        debug_assert_eq!(uri.host(), Some("example.org"));
        let _rug_ed_tests_llm_16_551_rrrruuuugggg_test_host_with_absolute_uri = 0;
    }
    #[test]
    fn test_host_with_relative_uri() {
        let _rug_st_tests_llm_16_551_rrrruuuugggg_test_host_with_relative_uri = 0;
        let rug_fuzz_0 = "/hello/world";
        let uri: Uri = rug_fuzz_0.parse().unwrap();
        debug_assert!(uri.host().is_none());
        let _rug_ed_tests_llm_16_551_rrrruuuugggg_test_host_with_relative_uri = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_553 {
    use crate::uri::Uri;
    #[test]
    fn test_path() {
        let _rug_st_tests_llm_16_553_rrrruuuugggg_test_path = 0;
        let rug_fuzz_0 = "/hello/world";
        let uri: Uri = rug_fuzz_0.parse().unwrap();
        debug_assert_eq!(uri.path(), "/hello/world");
        let _rug_ed_tests_llm_16_553_rrrruuuugggg_test_path = 0;
    }
    #[test]
    fn test_path_empty() {
        let _rug_st_tests_llm_16_553_rrrruuuugggg_test_path_empty = 0;
        let uri: Uri = Uri::default();
        debug_assert_eq!(uri.path(), "");
        let _rug_ed_tests_llm_16_553_rrrruuuugggg_test_path_empty = 0;
    }
    #[test]
    fn test_path_absolute() {
        let _rug_st_tests_llm_16_553_rrrruuuugggg_test_path_absolute = 0;
        let rug_fuzz_0 = "http://example.org/hello/world";
        let uri: Uri = rug_fuzz_0.parse().unwrap();
        debug_assert_eq!(uri.path(), "/hello/world");
        let _rug_ed_tests_llm_16_553_rrrruuuugggg_test_path_absolute = 0;
    }
    #[test]
    fn test_path_query() {
        let _rug_st_tests_llm_16_553_rrrruuuugggg_test_path_query = 0;
        let rug_fuzz_0 = "/hello/world?key=value";
        let uri: Uri = rug_fuzz_0.parse().unwrap();
        debug_assert_eq!(uri.path(), "/hello/world");
        let _rug_ed_tests_llm_16_553_rrrruuuugggg_test_path_query = 0;
    }
    #[test]
    fn test_path_query_fragment() {
        let _rug_st_tests_llm_16_553_rrrruuuugggg_test_path_query_fragment = 0;
        let rug_fuzz_0 = "/hello/world?key=value#fragment";
        let uri: Uri = rug_fuzz_0.parse().unwrap();
        debug_assert_eq!(uri.path(), "/hello/world");
        let _rug_ed_tests_llm_16_553_rrrruuuugggg_test_path_query_fragment = 0;
    }
    #[test]
    fn test_path_star() {
        let _rug_st_tests_llm_16_553_rrrruuuugggg_test_path_star = 0;
        let rug_fuzz_0 = "*";
        let uri: Uri = rug_fuzz_0.parse().unwrap();
        debug_assert_eq!(uri.path(), "*");
        let _rug_ed_tests_llm_16_553_rrrruuuugggg_test_path_star = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_554 {
    use super::*;
    use crate::*;
    #[test]
    fn test_path_and_query() {
        let _rug_st_tests_llm_16_554_rrrruuuugggg_test_path_and_query = 0;
        let rug_fuzz_0 = "http://example.com:8080/hello/world?key=value";
        let rug_fuzz_1 = "/hello/world?key=value";
        let uri: Uri = rug_fuzz_0.parse().unwrap();
        let expected = Some(rug_fuzz_1);
        let actual = uri.path_and_query().map(|pq| pq.as_str());
        debug_assert_eq!(expected, actual);
        let _rug_ed_tests_llm_16_554_rrrruuuugggg_test_path_and_query = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_555 {
    use super::*;
    use crate::*;
    #[test]
    fn test_port_with_port() {
        let _rug_st_tests_llm_16_555_rrrruuuugggg_test_port_with_port = 0;
        let rug_fuzz_0 = "http://example.org:80/hello/world";
        let uri: Uri = rug_fuzz_0.parse().unwrap();
        let port = uri.port().unwrap();
        debug_assert_eq!(port.as_u16(), 80);
        let _rug_ed_tests_llm_16_555_rrrruuuugggg_test_port_with_port = 0;
    }
    #[test]
    fn test_port_without_port() {
        let _rug_st_tests_llm_16_555_rrrruuuugggg_test_port_without_port = 0;
        let rug_fuzz_0 = "http://example.org/hello/world";
        let uri: Uri = rug_fuzz_0.parse().unwrap();
        debug_assert!(uri.port().is_none());
        let _rug_ed_tests_llm_16_555_rrrruuuugggg_test_port_without_port = 0;
    }
    #[test]
    fn test_port_with_relative_uri() {
        let _rug_st_tests_llm_16_555_rrrruuuugggg_test_port_with_relative_uri = 0;
        let rug_fuzz_0 = "/hello/world";
        let uri: Uri = rug_fuzz_0.parse().unwrap();
        debug_assert!(uri.port().is_none());
        let _rug_ed_tests_llm_16_555_rrrruuuugggg_test_port_with_relative_uri = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_557 {
    use crate::uri::{Uri, Scheme, Authority, PathAndQuery};
    #[test]
    fn test_query_absolute_uri() {
        let _rug_st_tests_llm_16_557_rrrruuuugggg_test_query_absolute_uri = 0;
        let rug_fuzz_0 = "http://example.org/hello/world?key=value";
        let uri: Uri = rug_fuzz_0.parse().unwrap();
        debug_assert_eq!(uri.query(), Some("key=value"));
        let _rug_ed_tests_llm_16_557_rrrruuuugggg_test_query_absolute_uri = 0;
    }
    #[test]
    fn test_query_relative_uri_with_query() {
        let _rug_st_tests_llm_16_557_rrrruuuugggg_test_query_relative_uri_with_query = 0;
        let rug_fuzz_0 = "/hello/world?key=value&foo=bar";
        let uri: Uri = rug_fuzz_0.parse().unwrap();
        debug_assert_eq!(uri.query(), Some("key=value&foo=bar"));
        let _rug_ed_tests_llm_16_557_rrrruuuugggg_test_query_relative_uri_with_query = 0;
    }
    #[test]
    fn test_query_relative_uri_without_query() {
        let _rug_st_tests_llm_16_557_rrrruuuugggg_test_query_relative_uri_without_query = 0;
        let rug_fuzz_0 = "/hello/world";
        let uri: Uri = rug_fuzz_0.parse().unwrap();
        debug_assert!(uri.query().is_none());
        let _rug_ed_tests_llm_16_557_rrrruuuugggg_test_query_relative_uri_without_query = 0;
    }
    #[test]
    fn test_path_absolute_uri() {
        let _rug_st_tests_llm_16_557_rrrruuuugggg_test_path_absolute_uri = 0;
        let rug_fuzz_0 = "http://example.org/hello/world";
        let uri: Uri = rug_fuzz_0.parse().unwrap();
        debug_assert_eq!(uri.path(), "/hello/world");
        let _rug_ed_tests_llm_16_557_rrrruuuugggg_test_path_absolute_uri = 0;
    }
    #[test]
    fn test_path_relative_uri_with_path() {
        let _rug_st_tests_llm_16_557_rrrruuuugggg_test_path_relative_uri_with_path = 0;
        let rug_fuzz_0 = "/hello/world?key=value&foo=bar";
        let uri: Uri = rug_fuzz_0.parse().unwrap();
        debug_assert_eq!(uri.path(), "/hello/world");
        let _rug_ed_tests_llm_16_557_rrrruuuugggg_test_path_relative_uri_with_path = 0;
    }
    #[test]
    fn test_path_relative_uri_without_path() {
        let _rug_st_tests_llm_16_557_rrrruuuugggg_test_path_relative_uri_without_path = 0;
        let rug_fuzz_0 = "/";
        let uri: Uri = rug_fuzz_0.parse().unwrap();
        debug_assert_eq!(uri.path(), "/");
        let _rug_ed_tests_llm_16_557_rrrruuuugggg_test_path_relative_uri_without_path = 0;
    }
    #[test]
    fn test_authority_absolute_uri_with_authority() {
        let _rug_st_tests_llm_16_557_rrrruuuugggg_test_authority_absolute_uri_with_authority = 0;
        let rug_fuzz_0 = "http://example.org:8080/hello/world?key=value&foo=bar";
        let uri: Uri = rug_fuzz_0.parse().unwrap();
        debug_assert_eq!(uri.authority().unwrap().as_str(), "example.org:8080");
        let _rug_ed_tests_llm_16_557_rrrruuuugggg_test_authority_absolute_uri_with_authority = 0;
    }
    #[test]
    fn test_authority_absolute_uri_without_authority() {
        let _rug_st_tests_llm_16_557_rrrruuuugggg_test_authority_absolute_uri_without_authority = 0;
        let rug_fuzz_0 = "http:///hello/world?key=value&foo=bar";
        let uri: Uri = rug_fuzz_0.parse().unwrap();
        debug_assert!(uri.authority().is_none());
        let _rug_ed_tests_llm_16_557_rrrruuuugggg_test_authority_absolute_uri_without_authority = 0;
    }
    #[test]
    fn test_host_absolute_uri_with_host() {
        let _rug_st_tests_llm_16_557_rrrruuuugggg_test_host_absolute_uri_with_host = 0;
        let rug_fuzz_0 = "http://example.org:8080/hello/world?key=value&foo=bar";
        let uri: Uri = rug_fuzz_0.parse().unwrap();
        debug_assert_eq!(uri.host().unwrap(), "example.org");
        let _rug_ed_tests_llm_16_557_rrrruuuugggg_test_host_absolute_uri_with_host = 0;
    }
    #[test]
    fn test_host_absolute_uri_without_host() {
        let _rug_st_tests_llm_16_557_rrrruuuugggg_test_host_absolute_uri_without_host = 0;
        let rug_fuzz_0 = "http:///hello/world?key=value&foo=bar";
        let uri: Uri = rug_fuzz_0.parse().unwrap();
        debug_assert!(uri.host().is_none());
        let _rug_ed_tests_llm_16_557_rrrruuuugggg_test_host_absolute_uri_without_host = 0;
    }
    #[test]
    fn test_port_absolute_uri_with_port() {
        let _rug_st_tests_llm_16_557_rrrruuuugggg_test_port_absolute_uri_with_port = 0;
        let rug_fuzz_0 = "http://example.org:8080/hello/world?key=value&foo=bar";
        let uri: Uri = rug_fuzz_0.parse().unwrap();
        debug_assert_eq!(uri.port().map(| p | p.as_u16()), Some(8080));
        let _rug_ed_tests_llm_16_557_rrrruuuugggg_test_port_absolute_uri_with_port = 0;
    }
    #[test]
    fn test_port_absolute_uri_without_port() {
        let _rug_st_tests_llm_16_557_rrrruuuugggg_test_port_absolute_uri_without_port = 0;
        let rug_fuzz_0 = "http://example.org/hello/world?key=value&foo=bar";
        let uri: Uri = rug_fuzz_0.parse().unwrap();
        debug_assert!(uri.port().is_none());
        let _rug_ed_tests_llm_16_557_rrrruuuugggg_test_port_absolute_uri_without_port = 0;
    }
    #[test]
    fn test_query_path_and_query_with_query() {
        let _rug_st_tests_llm_16_557_rrrruuuugggg_test_query_path_and_query_with_query = 0;
        let rug_fuzz_0 = "/hello/world?key=value&foo=bar";
        let path_and_query: PathAndQuery = rug_fuzz_0.parse().unwrap();
        debug_assert_eq!(path_and_query.query(), Some("key=value&foo=bar"));
        let _rug_ed_tests_llm_16_557_rrrruuuugggg_test_query_path_and_query_with_query = 0;
    }
    #[test]
    fn test_query_path_and_query_without_query() {
        let _rug_st_tests_llm_16_557_rrrruuuugggg_test_query_path_and_query_without_query = 0;
        let rug_fuzz_0 = "/hello/world";
        let path_and_query: PathAndQuery = rug_fuzz_0.parse().unwrap();
        debug_assert!(path_and_query.query().is_none());
        let _rug_ed_tests_llm_16_557_rrrruuuugggg_test_query_path_and_query_without_query = 0;
    }
    #[test]
    fn test_as_str_path_and_query_with_query() {
        let _rug_st_tests_llm_16_557_rrrruuuugggg_test_as_str_path_and_query_with_query = 0;
        let rug_fuzz_0 = "/hello/world?key=value&foo=bar";
        let path_and_query: PathAndQuery = rug_fuzz_0.parse().unwrap();
        debug_assert_eq!(path_and_query.as_str(), "/hello/world?key=value&foo=bar");
        let _rug_ed_tests_llm_16_557_rrrruuuugggg_test_as_str_path_and_query_with_query = 0;
    }
    #[test]
    fn test_as_str_path_and_query_without_query() {
        let _rug_st_tests_llm_16_557_rrrruuuugggg_test_as_str_path_and_query_without_query = 0;
        let rug_fuzz_0 = "/hello/world";
        let path_and_query: PathAndQuery = rug_fuzz_0.parse().unwrap();
        debug_assert_eq!(path_and_query.as_str(), "/hello/world");
        let _rug_ed_tests_llm_16_557_rrrruuuugggg_test_as_str_path_and_query_without_query = 0;
    }
    #[test]
    fn test_as_str_scheme_http() {
        let _rug_st_tests_llm_16_557_rrrruuuugggg_test_as_str_scheme_http = 0;
        let rug_fuzz_0 = "http";
        let scheme: Scheme = rug_fuzz_0.parse().unwrap();
        debug_assert_eq!(scheme.as_str(), "http");
        let _rug_ed_tests_llm_16_557_rrrruuuugggg_test_as_str_scheme_http = 0;
    }
    #[test]
    fn test_as_str_scheme_https() {
        let _rug_st_tests_llm_16_557_rrrruuuugggg_test_as_str_scheme_https = 0;
        let rug_fuzz_0 = "https";
        let scheme: Scheme = rug_fuzz_0.parse().unwrap();
        debug_assert_eq!(scheme.as_str(), "https");
        let _rug_ed_tests_llm_16_557_rrrruuuugggg_test_as_str_scheme_https = 0;
    }
}
#[test]
fn test_scheme() {
    use crate::uri::{Scheme, Uri};
    let uri: Uri = "http://example.org/hello/world".parse().unwrap();
    assert_eq!(uri.scheme(), Some(& Scheme::HTTP));
    let uri: Uri = "/hello/world".parse().unwrap();
    assert!(uri.scheme().is_none());
}
#[cfg(test)]
mod tests_llm_16_559 {
    use super::*;
    use crate::*;
    #[test]
    fn test_scheme_str_none() {
        let _rug_st_tests_llm_16_559_rrrruuuugggg_test_scheme_str_none = 0;
        let uri = Uri {
            scheme: Scheme::empty(),
            authority: Authority::empty(),
            path_and_query: PathAndQuery::empty(),
        };
        debug_assert_eq!(uri.scheme_str(), None);
        let _rug_ed_tests_llm_16_559_rrrruuuugggg_test_scheme_str_none = 0;
    }
    #[test]
    fn test_scheme_str_http() {
        let _rug_st_tests_llm_16_559_rrrruuuugggg_test_scheme_str_http = 0;
        let uri = Uri {
            scheme: Scheme::HTTP,
            authority: Authority::empty(),
            path_and_query: PathAndQuery::empty(),
        };
        debug_assert_eq!(uri.scheme_str(), Some("http"));
        let _rug_ed_tests_llm_16_559_rrrruuuugggg_test_scheme_str_http = 0;
    }
    #[test]
    fn test_scheme_str_https() {
        let _rug_st_tests_llm_16_559_rrrruuuugggg_test_scheme_str_https = 0;
        let uri = Uri {
            scheme: Scheme::HTTPS,
            authority: Authority::empty(),
            path_and_query: PathAndQuery::empty(),
        };
        debug_assert_eq!(uri.scheme_str(), Some("https"));
        let _rug_ed_tests_llm_16_559_rrrruuuugggg_test_scheme_str_https = 0;
    }
    #[test]
    fn test_scheme_str_other() {
        let _rug_st_tests_llm_16_559_rrrruuuugggg_test_scheme_str_other = 0;
        let rug_fuzz_0 = "other";
        let uri = Uri {
            scheme: Scheme {
                inner: Scheme2::Other(Box::new(ByteStr::from_static(rug_fuzz_0))),
            },
            authority: Authority::empty(),
            path_and_query: PathAndQuery::empty(),
        };
        debug_assert_eq!(uri.scheme_str(), Some("other"));
        let _rug_ed_tests_llm_16_559_rrrruuuugggg_test_scheme_str_other = 0;
    }
}
#[cfg(test)]
mod tests_rug_154 {
    use super::*;
    use bytes::Bytes;
    #[test]
    fn test_parse_full() {
        let _rug_st_tests_rug_154_rrrruuuugggg_test_parse_full = 0;
        let rug_fuzz_0 = "example";
        let mut p0: Bytes = Bytes::from(rug_fuzz_0.as_bytes());
        crate::uri::parse_full(p0);
        let _rug_ed_tests_rug_154_rrrruuuugggg_test_parse_full = 0;
    }
}
#[cfg(test)]
mod tests_rug_155 {
    use super::*;
    use crate::uri::{
        Uri, Parts, Scheme, Scheme2, Authority, PathAndQuery, InvalidUriParts, ErrorKind,
    };
    #[test]
    fn test_from_parts() {
        let _rug_st_tests_rug_155_rrrruuuugggg_test_from_parts = 0;
        let mut p0 = Parts::default();
        p0.scheme = Some(Scheme { inner: Scheme2::None });
        p0.authority = Some(Authority::empty());
        p0.path_and_query = Some(PathAndQuery::empty());
        let result = Uri::from_parts(p0);
        debug_assert!(result.is_ok());
        let _rug_ed_tests_rug_155_rrrruuuugggg_test_from_parts = 0;
    }
}
#[cfg(test)]
mod tests_rug_156 {
    use super::*;
    use crate::header::HeaderValue;
    use crate::uri::InvalidUri;
    use bytes::Bytes;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_156_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "hello";
        let mut p0: HeaderValue = HeaderValue::from_static(rug_fuzz_0);
        <Uri>::from_maybe_shared(p0);
        let _rug_ed_tests_rug_156_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_157 {
    use super::*;
    use crate::Uri;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_157_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https";
        let rug_fuzz_1 = "example.com";
        let rug_fuzz_2 = "/foo";
        let mut p0: Uri = Uri::builder()
            .scheme(rug_fuzz_0)
            .authority(rug_fuzz_1)
            .path_and_query(rug_fuzz_2)
            .build()
            .unwrap();
        crate::uri::Uri::into_parts(p0);
        let _rug_ed_tests_rug_157_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_158 {
    use super::*;
    use crate::Uri;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_158_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https";
        let rug_fuzz_1 = "example.com";
        let rug_fuzz_2 = "/foo";
        let mut p0: Uri = Uri::builder()
            .scheme(rug_fuzz_0)
            .authority(rug_fuzz_1)
            .path_and_query(rug_fuzz_2)
            .build()
            .unwrap();
        p0.authority();
        let _rug_ed_tests_rug_158_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_159 {
    use super::*;
    use crate::Uri;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_159_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https";
        let rug_fuzz_1 = "example.com";
        let rug_fuzz_2 = "/foo";
        let mut p0: Uri = Uri::builder()
            .scheme(rug_fuzz_0)
            .authority(rug_fuzz_1)
            .path_and_query(rug_fuzz_2)
            .build()
            .unwrap();
        p0.port_u16();
        let _rug_ed_tests_rug_159_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_160 {
    use super::*;
    use crate::uri::{Parts, Uri};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_160_rrrruuuugggg_test_rug = 0;
        let mut p0 = Parts::default();
        <Uri as std::convert::TryFrom<Parts>>::try_from(p0);
        let _rug_ed_tests_rug_160_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_163 {
    use super::*;
    use crate::Uri;
    use std::cmp::PartialEq;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_163_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https";
        let rug_fuzz_1 = "example.com";
        let rug_fuzz_2 = "/foo";
        let rug_fuzz_3 = "https";
        let rug_fuzz_4 = "example.com";
        let rug_fuzz_5 = "/foo";
        let p0: Uri = Uri::builder()
            .scheme(rug_fuzz_0)
            .authority(rug_fuzz_1)
            .path_and_query(rug_fuzz_2)
            .build()
            .unwrap();
        let p1: Uri = Uri::builder()
            .scheme(rug_fuzz_3)
            .authority(rug_fuzz_4)
            .path_and_query(rug_fuzz_5)
            .build()
            .unwrap();
        <Uri as PartialEq>::eq(&p0, &p1);
        let _rug_ed_tests_rug_163_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_164 {
    #[allow(unused_imports)]
    use super::*;
    use std::cmp::PartialEq;
    #[allow(unused_imports)]
    use crate::Uri;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_164_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https";
        let rug_fuzz_1 = "example.com";
        let rug_fuzz_2 = "/foo";
        let rug_fuzz_3 = "https://example.com/foo";
        let mut p0: Uri = Uri::builder()
            .scheme(rug_fuzz_0)
            .authority(rug_fuzz_1)
            .path_and_query(rug_fuzz_2)
            .build()
            .unwrap();
        let mut p1: &str = rug_fuzz_3;
        <Uri as std::cmp::PartialEq<&str>>::eq(&p0, &p1);
        let _rug_ed_tests_rug_164_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_166 {
    use super::*;
    use crate::uri::{Uri, Scheme, Authority, PathAndQuery};
    use std::default::Default;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_166_rrrruuuugggg_test_rug = 0;
        let uri: Uri = <Uri as Default>::default();
        let _rug_ed_tests_rug_166_rrrruuuugggg_test_rug = 0;
    }
}
