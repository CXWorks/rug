/*!

rust-url is an implementation of the [URL Standard](http://url.spec.whatwg.org/)
for the [Rust](http://rust-lang.org/) programming language.


# URL parsing and data structures

First, URL parsing may fail for various reasons and therefore returns a `Result`.

```
use url::{Url, ParseError};

assert!(Url::parse("http://[:::1]") == Err(ParseError::InvalidIpv6Address))
```

Let’s parse a valid URL and look at its components.

```
use url::{Url, Host, Position};
# use url::ParseError;
# fn run() -> Result<(), ParseError> {
let issue_list_url = Url::parse(
    "https://github.com/rust-lang/rust/issues?labels=E-easy&state=open"
)?;


assert!(issue_list_url.scheme() == "https");
assert!(issue_list_url.username() == "");
assert!(issue_list_url.password() == None);
assert!(issue_list_url.host_str() == Some("github.com"));
assert!(issue_list_url.host() == Some(Host::Domain("github.com")));
assert!(issue_list_url.port() == None);
assert!(issue_list_url.path() == "/rust-lang/rust/issues");
assert!(issue_list_url.path_segments().map(|c| c.collect::<Vec<_>>()) ==
        Some(vec!["rust-lang", "rust", "issues"]));
assert!(issue_list_url.query() == Some("labels=E-easy&state=open"));
assert!(&issue_list_url[Position::BeforePath..] == "/rust-lang/rust/issues?labels=E-easy&state=open");
assert!(issue_list_url.fragment() == None);
assert!(!issue_list_url.cannot_be_a_base());
# Ok(())
# }
# run().unwrap();
```

Some URLs are said to be *cannot-be-a-base*:
they don’t have a username, password, host, or port,
and their "path" is an arbitrary string rather than slash-separated segments:

```
use url::Url;
# use url::ParseError;

# fn run() -> Result<(), ParseError> {
let data_url = Url::parse("data:text/plain,Hello?World#")?;

assert!(data_url.cannot_be_a_base());
assert!(data_url.scheme() == "data");
assert!(data_url.path() == "text/plain,Hello");
assert!(data_url.path_segments().is_none());
assert!(data_url.query() == Some("World"));
assert!(data_url.fragment() == Some(""));
# Ok(())
# }
# run().unwrap();
```


# Base URL

Many contexts allow URL *references* that can be relative to a *base URL*:

```html
<link rel="stylesheet" href="../main.css">
```

Since parsed URLs are absolute, giving a base is required for parsing relative URLs:

```
use url::{Url, ParseError};

assert!(Url::parse("../main.css") == Err(ParseError::RelativeUrlWithoutBase))
```

Use the `join` method on an `Url` to use it as a base URL:

```
use url::Url;
# use url::ParseError;

# fn run() -> Result<(), ParseError> {
let this_document = Url::parse("http://servo.github.io/rust-url/url/index.html")?;
let css_url = this_document.join("../main.css")?;
assert_eq!(css_url.as_str(), "http://servo.github.io/rust-url/main.css");
# Ok(())
# }
# run().unwrap();
*/
#![doc(html_root_url = "https://docs.rs/url/2.1.1")]
#[macro_use]
extern crate matches;
pub use form_urlencoded;
#[cfg(feature = "serde")]
extern crate serde;
use crate::host::HostInternal;
use crate::parser::{to_u32, Context, Parser, SchemeType, PATH_SEGMENT, USERINFO};
use percent_encoding::{percent_decode, percent_encode, utf8_percent_encode};
use std::borrow::Borrow;
use std::cmp;
#[cfg(feature = "serde")]
use std::error::Error;
use std::fmt::{self, Write};
use std::hash;
use std::io;
use std::mem;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::ops::{Range, RangeFrom, RangeTo};
use std::path::{Path, PathBuf};
use std::str;
use std::convert::TryFrom;
pub use crate::host::Host;
pub use crate::origin::{OpaqueOrigin, Origin};
pub use crate::parser::{ParseError, SyntaxViolation};
pub use crate::path_segments::PathSegmentsMut;
pub use crate::slicing::Position;
pub use form_urlencoded::EncodingOverride;
mod host;
mod origin;
mod parser;
mod path_segments;
mod slicing;
#[doc(hidden)]
pub mod quirks;
/// A parsed URL record.
#[derive(Clone)]
pub struct Url {
    /// Syntax in pseudo-BNF:
    ///
    ///   url = scheme ":" [ hierarchical | non-hierarchical ] [ "?" query ]? [ "#" fragment ]?
    ///   non-hierarchical = non-hierarchical-path
    ///   non-hierarchical-path = /* Does not start with "/" */
    ///   hierarchical = authority? hierarchical-path
    ///   authority = "//" userinfo? host [ ":" port ]?
    ///   userinfo = username [ ":" password ]? "@"
    ///   hierarchical-path = [ "/" path-segment ]+
    serialization: String,
    scheme_end: u32,
    username_end: u32,
    host_start: u32,
    host_end: u32,
    host: HostInternal,
    port: Option<u16>,
    path_start: u32,
    query_start: Option<u32>,
    fragment_start: Option<u32>,
}
/// Full configuration for the URL parser.
#[derive(Copy, Clone)]
pub struct ParseOptions<'a> {
    base_url: Option<&'a Url>,
    encoding_override: EncodingOverride<'a>,
    violation_fn: Option<&'a dyn Fn(SyntaxViolation)>,
}
impl<'a> ParseOptions<'a> {
    /// Change the base URL
    pub fn base_url(mut self, new: Option<&'a Url>) -> Self {
        self.base_url = new;
        self
    }
    /// Override the character encoding of query strings.
    /// This is a legacy concept only relevant for HTML.
    pub fn encoding_override(mut self, new: EncodingOverride<'a>) -> Self {
        self.encoding_override = new;
        self
    }
    /// Call the provided function or closure for a non-fatal `SyntaxViolation`
    /// when it occurs during parsing. Note that since the provided function is
    /// `Fn`, the caller might need to utilize _interior mutability_, such as with
    /// a `RefCell`, to collect the violations.
    ///
    /// ## Example
    /// ```
    /// use std::cell::RefCell;
    /// use url::{Url, SyntaxViolation};
    /// # use url::ParseError;
    /// # fn run() -> Result<(), url::ParseError> {
    /// let violations = RefCell::new(Vec::new());
    /// let url = Url::options()
    ///     .syntax_violation_callback(Some(&|v| violations.borrow_mut().push(v)))
    ///     .parse("https:////example.com")?;
    /// assert_eq!(url.as_str(), "https://example.com/");
    /// assert_eq!(violations.into_inner(),
    ///            vec!(SyntaxViolation::ExpectedDoubleSlash));
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    pub fn syntax_violation_callback(
        mut self,
        new: Option<&'a dyn Fn(SyntaxViolation)>,
    ) -> Self {
        self.violation_fn = new;
        self
    }
    /// Parse an URL string with the configuration so far.
    pub fn parse(self, input: &str) -> Result<Url, crate::ParseError> {
        Parser {
            serialization: String::with_capacity(input.len()),
            base_url: self.base_url,
            query_encoding_override: self.encoding_override,
            violation_fn: self.violation_fn,
            context: Context::UrlParser,
        }
            .parse_url(input)
    }
}
impl Url {
    /// Parse an absolute URL from a string.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use url::Url;
    /// # use url::ParseError;
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let url = Url::parse("https://example.net")?;
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// If the function can not parse an absolute URL from the given string,
    /// a [`ParseError`] variant will be returned.
    ///
    /// [`ParseError`]: enum.ParseError.html
    #[inline]
    pub fn parse(input: &str) -> Result<Url, crate::ParseError> {
        Url::options().parse(input)
    }
    /// Parse an absolute URL from a string and add params to its query string.
    ///
    /// Existing params are not removed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use url::Url;
    /// # use url::ParseError;
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let url = Url::parse_with_params("https://example.net?dont=clobberme",
    ///                                  &[("lang", "rust"), ("browser", "servo")])?;
    /// assert_eq!("https://example.net/?dont=clobberme&lang=rust&browser=servo", url.as_str());
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// If the function can not parse an absolute URL from the given string,
    /// a [`ParseError`] variant will be returned.
    ///
    /// [`ParseError`]: enum.ParseError.html
    #[inline]
    pub fn parse_with_params<I, K, V>(
        input: &str,
        iter: I,
    ) -> Result<Url, crate::ParseError>
    where
        I: IntoIterator,
        I::Item: Borrow<(K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let mut url = Url::options().parse(input);
        if let Ok(ref mut url) = url {
            url.query_pairs_mut().extend_pairs(iter);
        }
        url
    }
    /// Parse a string as an URL, with this URL as the base URL.
    ///
    /// Note: a trailing slash is significant.
    /// Without it, the last path component is considered to be a “file” name
    /// to be removed to get at the “directory” that is used as the base:
    ///
    /// # Examples
    ///
    /// ```rust
    /// use url::Url;
    /// # use url::ParseError;
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let base = Url::parse("https://example.net/a/b.html")?;
    /// let url = base.join("c.png")?;
    /// assert_eq!(url.as_str(), "https://example.net/a/c.png");  // Not /a/b.html/c.png
    ///
    /// let base = Url::parse("https://example.net/a/b/")?;
    /// let url = base.join("c.png")?;
    /// assert_eq!(url.as_str(), "https://example.net/a/b/c.png");
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// If the function can not parse an URL from the given string
    /// with this URL as the base URL, a [`ParseError`] variant will be returned.
    ///
    /// [`ParseError`]: enum.ParseError.html
    #[inline]
    pub fn join(&self, input: &str) -> Result<Url, crate::ParseError> {
        Url::options().base_url(Some(self)).parse(input)
    }
    /// Return a default `ParseOptions` that can fully configure the URL parser.
    ///
    /// # Examples
    ///
    /// Get default `ParseOptions`, then change base url
    ///
    /// ```rust
    /// use url::Url;
    /// # use url::ParseError;
    /// # fn run() -> Result<(), ParseError> {
    /// let options = Url::options();
    /// let api = Url::parse("https://api.example.com")?;
    /// let base_url = options.base_url(Some(&api));
    /// let version_url = base_url.parse("version.json")?;
    /// assert_eq!(version_url.as_str(), "https://api.example.com/version.json");
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    pub fn options<'a>() -> ParseOptions<'a> {
        ParseOptions {
            base_url: None,
            encoding_override: None,
            violation_fn: None,
        }
    }
    /// Return the serialization of this URL.
    ///
    /// This is fast since that serialization is already stored in the `Url` struct.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use url::Url;
    /// # use url::ParseError;
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let url_str = "https://example.net/";
    /// let url = Url::parse(url_str)?;
    /// assert_eq!(url.as_str(), url_str);
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.serialization
    }
    /// Return the serialization of this URL.
    ///
    /// This consumes the `Url` and takes ownership of the `String` stored in it.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use url::Url;
    /// # use url::ParseError;
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let url_str = "https://example.net/";
    /// let url = Url::parse(url_str)?;
    /// assert_eq!(url.into_string(), url_str);
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    #[inline]
    pub fn into_string(self) -> String {
        self.serialization
    }
    /// For internal testing, not part of the public API.
    ///
    /// Methods of the `Url` struct assume a number of invariants.
    /// This checks each of these invariants and panic if one is not met.
    /// This is for testing rust-url itself.
    #[doc(hidden)]
    pub fn check_invariants(&self) -> Result<(), String> {
        macro_rules! assert {
            ($x:expr) => {
                if !$x { return Err(format!("!( {} ) for URL {:?}", stringify!($x), self
                .serialization)); }
            };
        }
        macro_rules! assert_eq {
            ($a:expr, $b:expr) => {
                { let a = $a; let b = $b; if a != b { return
                Err(format!("{:?} != {:?} ({} != {}) for URL {:?}", a, b, stringify!($a),
                stringify!($b), self.serialization)) } }
            };
        }
        assert!(self.scheme_end >= 1);
        assert!(matches!(self.byte_at(0), b'a'..= b'z' | b'A'..= b'Z'));
        assert!(
            self.slice(1..self.scheme_end).chars().all(| c | matches!(c, 'a'..='z' | 'A'
            ..='Z' | '0'..='9' | '+' | '-' | '.'))
        );
        assert_eq!(self.byte_at(self.scheme_end), b':');
        if self.slice(self.scheme_end + 1..).starts_with("//") {
            if self.username_end != self.serialization.len() as u32 {
                match self.byte_at(self.username_end) {
                    b':' => {
                        assert!(self.host_start >= self.username_end + 2);
                        assert_eq!(self.byte_at(self.host_start - 1), b'@');
                    }
                    b'@' => assert!(self.host_start == self.username_end + 1),
                    _ => assert_eq!(self.username_end, self.scheme_end + 3),
                }
            }
            assert!(self.host_start >= self.username_end);
            assert!(self.host_end >= self.host_start);
            let host_str = self.slice(self.host_start..self.host_end);
            match self.host {
                HostInternal::None => assert_eq!(host_str, ""),
                HostInternal::Ipv4(address) => assert_eq!(host_str, address.to_string()),
                HostInternal::Ipv6(address) => {
                    let h: Host<String> = Host::Ipv6(address);
                    assert_eq!(host_str, h.to_string())
                }
                HostInternal::Domain => {
                    if SchemeType::from(self.scheme()).is_special() {
                        assert!(! host_str.is_empty())
                    }
                }
            }
            if self.path_start == self.host_end {
                assert_eq!(self.port, None);
            } else {
                assert_eq!(self.byte_at(self.host_end), b':');
                let port_str = self.slice(self.host_end + 1..self.path_start);
                assert_eq!(
                    self.port, Some(port_str.parse::< u16 > ()
                    .expect("Couldn't parse port?"))
                );
            }
            assert!(
                self.path_start as usize == self.serialization.len() || matches!(self
                .byte_at(self.path_start), b'/' | b'#' | b'?')
            );
        } else {
            assert_eq!(self.username_end, self.scheme_end + 1);
            assert_eq!(self.host_start, self.scheme_end + 1);
            assert_eq!(self.host_end, self.scheme_end + 1);
            assert_eq!(self.host, HostInternal::None);
            assert_eq!(self.port, None);
            assert_eq!(self.path_start, self.scheme_end + 1);
        }
        if let Some(start) = self.query_start {
            assert!(start >= self.path_start);
            assert_eq!(self.byte_at(start), b'?');
        }
        if let Some(start) = self.fragment_start {
            assert!(start >= self.path_start);
            assert_eq!(self.byte_at(start), b'#');
        }
        if let (Some(query_start), Some(fragment_start))
            = (self.query_start, self.fragment_start) {
            assert!(fragment_start > query_start);
        }
        let other = Url::parse(self.as_str()).expect("Failed to parse myself?");
        assert_eq!(& self.serialization, & other.serialization);
        assert_eq!(self.scheme_end, other.scheme_end);
        assert_eq!(self.username_end, other.username_end);
        assert_eq!(self.host_start, other.host_start);
        assert_eq!(self.host_end, other.host_end);
        assert!(
            self.host == other.host || (self.host_str(), other.host_str()) == (None,
            Some(""))
        );
        assert_eq!(self.port, other.port);
        assert_eq!(self.path_start, other.path_start);
        assert_eq!(self.query_start, other.query_start);
        assert_eq!(self.fragment_start, other.fragment_start);
        Ok(())
    }
    /// Return the origin of this URL (<https://url.spec.whatwg.org/#origin>)
    ///
    /// Note: this returns an opaque origin for `file:` URLs, which causes
    /// `url.origin() != url.origin()`.
    ///
    /// # Examples
    ///
    /// URL with `ftp` scheme:
    ///
    /// ```rust
    /// use url::{Host, Origin, Url};
    /// # use url::ParseError;
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let url = Url::parse("ftp://example.com/foo")?;
    /// assert_eq!(url.origin(),
    ///            Origin::Tuple("ftp".into(),
    ///                          Host::Domain("example.com".into()),
    ///                          21));
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    ///
    /// URL with `blob` scheme:
    ///
    /// ```rust
    /// use url::{Host, Origin, Url};
    /// # use url::ParseError;
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let url = Url::parse("blob:https://example.com/foo")?;
    /// assert_eq!(url.origin(),
    ///            Origin::Tuple("https".into(),
    ///                          Host::Domain("example.com".into()),
    ///                          443));
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    ///
    /// URL with `file` scheme:
    ///
    /// ```rust
    /// use url::{Host, Origin, Url};
    /// # use url::ParseError;
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let url = Url::parse("file:///tmp/foo")?;
    /// assert!(!url.origin().is_tuple());
    ///
    /// let other_url = Url::parse("file:///tmp/foo")?;
    /// assert!(url.origin() != other_url.origin());
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    ///
    /// URL with other scheme:
    ///
    /// ```rust
    /// use url::{Host, Origin, Url};
    /// # use url::ParseError;
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let url = Url::parse("foo:bar")?;
    /// assert!(!url.origin().is_tuple());
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    #[inline]
    pub fn origin(&self) -> Origin {
        origin::url_origin(self)
    }
    /// Return the scheme of this URL, lower-cased, as an ASCII string without the ':' delimiter.
    ///
    /// # Examples
    ///
    /// ```
    /// use url::Url;
    /// # use url::ParseError;
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let url = Url::parse("file:///tmp/foo")?;
    /// assert_eq!(url.scheme(), "file");
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    #[inline]
    pub fn scheme(&self) -> &str {
        self.slice(..self.scheme_end)
    }
    /// Return whether the URL has an 'authority',
    /// which can contain a username, password, host, and port number.
    ///
    /// URLs that do *not* are either path-only like `unix:/run/foo.socket`
    /// or cannot-be-a-base like `data:text/plain,Stuff`.
    ///
    /// # Examples
    ///
    /// ```
    /// use url::Url;
    /// # use url::ParseError;
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let url = Url::parse("ftp://rms@example.com")?;
    /// assert!(url.has_authority());
    ///
    /// let url = Url::parse("unix:/run/foo.socket")?;
    /// assert!(!url.has_authority());
    ///
    /// let url = Url::parse("data:text/plain,Stuff")?;
    /// assert!(!url.has_authority());
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    #[inline]
    pub fn has_authority(&self) -> bool {
        debug_assert!(self.byte_at(self.scheme_end) == b':');
        self.slice(self.scheme_end..).starts_with("://")
    }
    /// Return whether this URL is a cannot-be-a-base URL,
    /// meaning that parsing a relative URL string with this URL as the base will return an error.
    ///
    /// This is the case if the scheme and `:` delimiter are not followed by a `/` slash,
    /// as is typically the case of `data:` and `mailto:` URLs.
    ///
    /// # Examples
    ///
    /// ```
    /// use url::Url;
    /// # use url::ParseError;
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let url = Url::parse("ftp://rms@example.com")?;
    /// assert!(!url.cannot_be_a_base());
    ///
    /// let url = Url::parse("unix:/run/foo.socket")?;
    /// assert!(!url.cannot_be_a_base());
    ///
    /// let url = Url::parse("data:text/plain,Stuff")?;
    /// assert!(url.cannot_be_a_base());
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    #[inline]
    pub fn cannot_be_a_base(&self) -> bool {
        !self.slice(self.scheme_end + 1..).starts_with('/')
    }
    /// Return the username for this URL (typically the empty string)
    /// as a percent-encoded ASCII string.
    ///
    /// # Examples
    ///
    /// ```
    /// use url::Url;
    /// # use url::ParseError;
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let url = Url::parse("ftp://rms@example.com")?;
    /// assert_eq!(url.username(), "rms");
    ///
    /// let url = Url::parse("ftp://:secret123@example.com")?;
    /// assert_eq!(url.username(), "");
    ///
    /// let url = Url::parse("https://example.com")?;
    /// assert_eq!(url.username(), "");
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    pub fn username(&self) -> &str {
        if self.has_authority() {
            self.slice(self.scheme_end + ("://".len() as u32)..self.username_end)
        } else {
            ""
        }
    }
    /// Return the password for this URL, if any, as a percent-encoded ASCII string.
    ///
    /// # Examples
    ///
    /// ```
    /// use url::Url;
    /// # use url::ParseError;
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let url = Url::parse("ftp://rms:secret123@example.com")?;
    /// assert_eq!(url.password(), Some("secret123"));
    ///
    /// let url = Url::parse("ftp://:secret123@example.com")?;
    /// assert_eq!(url.password(), Some("secret123"));
    ///
    /// let url = Url::parse("ftp://rms@example.com")?;
    /// assert_eq!(url.password(), None);
    ///
    /// let url = Url::parse("https://example.com")?;
    /// assert_eq!(url.password(), None);
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    pub fn password(&self) -> Option<&str> {
        if self.has_authority() && self.username_end != self.serialization.len() as u32
            && self.byte_at(self.username_end) == b':'
        {
            debug_assert!(self.byte_at(self.host_start - 1) == b'@');
            Some(self.slice(self.username_end + 1..self.host_start - 1))
        } else {
            None
        }
    }
    /// Equivalent to `url.host().is_some()`.
    ///
    /// # Examples
    ///
    /// ```
    /// use url::Url;
    /// # use url::ParseError;
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let url = Url::parse("ftp://rms@example.com")?;
    /// assert!(url.has_host());
    ///
    /// let url = Url::parse("unix:/run/foo.socket")?;
    /// assert!(!url.has_host());
    ///
    /// let url = Url::parse("data:text/plain,Stuff")?;
    /// assert!(!url.has_host());
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    pub fn has_host(&self) -> bool {
        !matches!(self.host, HostInternal::None)
    }
    /// Return the string representation of the host (domain or IP address) for this URL, if any.
    ///
    /// Non-ASCII domains are punycode-encoded per IDNA.
    /// IPv6 addresses are given between `[` and `]` brackets.
    ///
    /// Cannot-be-a-base URLs (typical of `data:` and `mailto:`) and some `file:` URLs
    /// don’t have a host.
    ///
    /// See also the `host` method.
    ///
    /// # Examples
    ///
    /// ```
    /// use url::Url;
    /// # use url::ParseError;
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let url = Url::parse("https://127.0.0.1/index.html")?;
    /// assert_eq!(url.host_str(), Some("127.0.0.1"));
    ///
    /// let url = Url::parse("ftp://rms@example.com")?;
    /// assert_eq!(url.host_str(), Some("example.com"));
    ///
    /// let url = Url::parse("unix:/run/foo.socket")?;
    /// assert_eq!(url.host_str(), None);
    ///
    /// let url = Url::parse("data:text/plain,Stuff")?;
    /// assert_eq!(url.host_str(), None);
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    pub fn host_str(&self) -> Option<&str> {
        if self.has_host() {
            Some(self.slice(self.host_start..self.host_end))
        } else {
            None
        }
    }
    /// Return the parsed representation of the host for this URL.
    /// Non-ASCII domain labels are punycode-encoded per IDNA.
    ///
    /// Cannot-be-a-base URLs (typical of `data:` and `mailto:`) and some `file:` URLs
    /// don’t have a host.
    ///
    /// See also the `host_str` method.
    ///
    /// # Examples
    ///
    /// ```
    /// use url::Url;
    /// # use url::ParseError;
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let url = Url::parse("https://127.0.0.1/index.html")?;
    /// assert!(url.host().is_some());
    ///
    /// let url = Url::parse("ftp://rms@example.com")?;
    /// assert!(url.host().is_some());
    ///
    /// let url = Url::parse("unix:/run/foo.socket")?;
    /// assert!(url.host().is_none());
    ///
    /// let url = Url::parse("data:text/plain,Stuff")?;
    /// assert!(url.host().is_none());
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    pub fn host(&self) -> Option<Host<&str>> {
        match self.host {
            HostInternal::None => None,
            HostInternal::Domain => {
                Some(Host::Domain(self.slice(self.host_start..self.host_end)))
            }
            HostInternal::Ipv4(address) => Some(Host::Ipv4(address)),
            HostInternal::Ipv6(address) => Some(Host::Ipv6(address)),
        }
    }
    /// If this URL has a host and it is a domain name (not an IP address), return it.
    ///
    /// # Examples
    ///
    /// ```
    /// use url::Url;
    /// # use url::ParseError;
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let url = Url::parse("https://127.0.0.1/")?;
    /// assert_eq!(url.domain(), None);
    ///
    /// let url = Url::parse("mailto:rms@example.net")?;
    /// assert_eq!(url.domain(), None);
    ///
    /// let url = Url::parse("https://example.com/")?;
    /// assert_eq!(url.domain(), Some("example.com"));
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    pub fn domain(&self) -> Option<&str> {
        match self.host {
            HostInternal::Domain => Some(self.slice(self.host_start..self.host_end)),
            _ => None,
        }
    }
    /// Return the port number for this URL, if any.
    ///
    /// Note that default port numbers are never reflected by the serialization,
    /// use the `port_or_known_default()` method if you want a default port number returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use url::Url;
    /// # use url::ParseError;
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let url = Url::parse("https://example.com")?;
    /// assert_eq!(url.port(), None);
    ///
    /// let url = Url::parse("https://example.com:443/")?;
    /// assert_eq!(url.port(), None);
    ///
    /// let url = Url::parse("ssh://example.com:22")?;
    /// assert_eq!(url.port(), Some(22));
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    #[inline]
    pub fn port(&self) -> Option<u16> {
        self.port
    }
    /// Return the port number for this URL, or the default port number if it is known.
    ///
    /// This method only knows the default port number
    /// of the `http`, `https`, `ws`, `wss` and `ftp` schemes.
    ///
    /// For URLs in these schemes, this method always returns `Some(_)`.
    /// For other schemes, it is the same as `Url::port()`.
    ///
    /// # Examples
    ///
    /// ```
    /// use url::Url;
    /// # use url::ParseError;
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let url = Url::parse("foo://example.com")?;
    /// assert_eq!(url.port_or_known_default(), None);
    ///
    /// let url = Url::parse("foo://example.com:1456")?;
    /// assert_eq!(url.port_or_known_default(), Some(1456));
    ///
    /// let url = Url::parse("https://example.com")?;
    /// assert_eq!(url.port_or_known_default(), Some(443));
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    #[inline]
    pub fn port_or_known_default(&self) -> Option<u16> {
        self.port.or_else(|| parser::default_port(self.scheme()))
    }
    /// Resolve a URL’s host and port number to `SocketAddr`.
    ///
    /// If the URL has the default port number of a scheme that is unknown to this library,
    /// `default_port_number` provides an opportunity to provide the actual port number.
    /// In non-example code this should be implemented either simply as `|| None`,
    /// or by matching on the URL’s `.scheme()`.
    ///
    /// If the host is a domain, it is resolved using the standard library’s DNS support.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let url = url::Url::parse("https://example.net/").unwrap();
    /// let addrs = url.socket_addrs(|| None).unwrap();
    /// std::net::TcpStream::connect(&*addrs)
    /// # ;
    /// ```
    ///
    /// ```
    /// /// With application-specific known default port numbers
    /// fn socket_addrs(url: url::Url) -> std::io::Result<Vec<std::net::SocketAddr>> {
    ///     url.socket_addrs(|| match url.scheme() {
    ///         "socks5" | "socks5h" => Some(1080),
    ///         _ => None,
    ///     })
    /// }
    /// ```
    pub fn socket_addrs(
        &self,
        default_port_number: impl Fn() -> Option<u16>,
    ) -> io::Result<Vec<SocketAddr>> {
        fn io_result<T>(opt: Option<T>, message: &str) -> io::Result<T> {
            opt.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, message))
        }
        let host = io_result(self.host(), "No host name in the URL")?;
        let port = io_result(
            self.port_or_known_default().or_else(default_port_number),
            "No port number in the URL",
        )?;
        Ok(
            match host {
                Host::Domain(domain) => (domain, port).to_socket_addrs()?.collect(),
                Host::Ipv4(ip) => vec![(ip, port).into()],
                Host::Ipv6(ip) => vec![(ip, port).into()],
            },
        )
    }
    /// Return the path for this URL, as a percent-encoded ASCII string.
    /// For cannot-be-a-base URLs, this is an arbitrary string that doesn’t start with '/'.
    /// For other URLs, this starts with a '/' slash
    /// and continues with slash-separated path segments.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use url::{Url, ParseError};
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let url = Url::parse("https://example.com/api/versions?page=2")?;
    /// assert_eq!(url.path(), "/api/versions");
    ///
    /// let url = Url::parse("https://example.com")?;
    /// assert_eq!(url.path(), "/");
    ///
    /// let url = Url::parse("https://example.com/countries/việt nam")?;
    /// assert_eq!(url.path(), "/countries/vi%E1%BB%87t%20nam");
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    pub fn path(&self) -> &str {
        match (self.query_start, self.fragment_start) {
            (None, None) => self.slice(self.path_start..),
            (Some(next_component_start), _) | (None, Some(next_component_start)) => {
                self.slice(self.path_start..next_component_start)
            }
        }
    }
    /// Unless this URL is cannot-be-a-base,
    /// return an iterator of '/' slash-separated path segments,
    /// each as a percent-encoded ASCII string.
    ///
    /// Return `None` for cannot-be-a-base URLs.
    ///
    /// When `Some` is returned, the iterator always contains at least one string
    /// (which may be empty).
    ///
    /// # Examples
    ///
    /// ```
    /// use url::Url;
    /// # use std::error::Error;
    ///
    /// # fn run() -> Result<(), Box<dyn Error>> {
    /// let url = Url::parse("https://example.com/foo/bar")?;
    /// let mut path_segments = url.path_segments().ok_or_else(|| "cannot be base")?;
    /// assert_eq!(path_segments.next(), Some("foo"));
    /// assert_eq!(path_segments.next(), Some("bar"));
    /// assert_eq!(path_segments.next(), None);
    ///
    /// let url = Url::parse("https://example.com")?;
    /// let mut path_segments = url.path_segments().ok_or_else(|| "cannot be base")?;
    /// assert_eq!(path_segments.next(), Some(""));
    /// assert_eq!(path_segments.next(), None);
    ///
    /// let url = Url::parse("data:text/plain,HelloWorld")?;
    /// assert!(url.path_segments().is_none());
    ///
    /// let url = Url::parse("https://example.com/countries/việt nam")?;
    /// let mut path_segments = url.path_segments().ok_or_else(|| "cannot be base")?;
    /// assert_eq!(path_segments.next(), Some("countries"));
    /// assert_eq!(path_segments.next(), Some("vi%E1%BB%87t%20nam"));
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    pub fn path_segments(&self) -> Option<str::Split<'_, char>> {
        let path = self.path();
        if path.starts_with('/') { Some(path[1..].split('/')) } else { None }
    }
    /// Return this URL’s query string, if any, as a percent-encoded ASCII string.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use url::Url;
    /// # use url::ParseError;
    ///
    /// fn run() -> Result<(), ParseError> {
    /// let url = Url::parse("https://example.com/products?page=2")?;
    /// let query = url.query();
    /// assert_eq!(query, Some("page=2"));
    ///
    /// let url = Url::parse("https://example.com/products")?;
    /// let query = url.query();
    /// assert!(query.is_none());
    ///
    /// let url = Url::parse("https://example.com/?country=español")?;
    /// let query = url.query();
    /// assert_eq!(query, Some("country=espa%C3%B1ol"));
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    pub fn query(&self) -> Option<&str> {
        match (self.query_start, self.fragment_start) {
            (None, _) => None,
            (Some(query_start), None) => {
                debug_assert!(self.byte_at(query_start) == b'?');
                Some(self.slice(query_start + 1..))
            }
            (Some(query_start), Some(fragment_start)) => {
                debug_assert!(self.byte_at(query_start) == b'?');
                Some(self.slice(query_start + 1..fragment_start))
            }
        }
    }
    /// Parse the URL’s query string, if any, as `application/x-www-form-urlencoded`
    /// and return an iterator of (key, value) pairs.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::borrow::Cow;
    ///
    /// use url::Url;
    /// # use url::ParseError;
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let url = Url::parse("https://example.com/products?page=2&sort=desc")?;
    /// let mut pairs = url.query_pairs();
    ///
    /// assert_eq!(pairs.count(), 2);
    ///
    /// assert_eq!(pairs.next(), Some((Cow::Borrowed("page"), Cow::Borrowed("2"))));
    /// assert_eq!(pairs.next(), Some((Cow::Borrowed("sort"), Cow::Borrowed("desc"))));
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    ///
    #[inline]
    pub fn query_pairs(&self) -> form_urlencoded::Parse<'_> {
        form_urlencoded::parse(self.query().unwrap_or("").as_bytes())
    }
    /// Return this URL’s fragment identifier, if any.
    ///
    /// A fragment is the part of the URL after the `#` symbol.
    /// The fragment is optional and, if present, contains a fragment identifier
    /// that identifies a secondary resource, such as a section heading
    /// of a document.
    ///
    /// In HTML, the fragment identifier is usually the id attribute of a an element
    /// that is scrolled to on load. Browsers typically will not send the fragment portion
    /// of a URL to the server.
    ///
    /// **Note:** the parser did *not* percent-encode this component,
    /// but the input may have been percent-encoded already.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use url::Url;
    /// # use url::ParseError;
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let url = Url::parse("https://example.com/data.csv#row=4")?;
    ///
    /// assert_eq!(url.fragment(), Some("row=4"));
    ///
    /// let url = Url::parse("https://example.com/data.csv#cell=4,1-6,2")?;
    ///
    /// assert_eq!(url.fragment(), Some("cell=4,1-6,2"));
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    pub fn fragment(&self) -> Option<&str> {
        self.fragment_start
            .map(|start| {
                debug_assert!(self.byte_at(start) == b'#');
                self.slice(start + 1..)
            })
    }
    fn mutate<F: FnOnce(&mut Parser<'_>) -> R, R>(&mut self, f: F) -> R {
        let mut parser = Parser::for_setter(
            mem::replace(&mut self.serialization, String::new()),
        );
        let result = f(&mut parser);
        self.serialization = parser.serialization;
        result
    }
    /// Change this URL’s fragment identifier.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use url::Url;
    /// # use url::ParseError;
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let mut url = Url::parse("https://example.com/data.csv")?;
    /// assert_eq!(url.as_str(), "https://example.com/data.csv");
    /// url.set_fragment(Some("cell=4,1-6,2"));
    /// assert_eq!(url.as_str(), "https://example.com/data.csv#cell=4,1-6,2");
    /// assert_eq!(url.fragment(), Some("cell=4,1-6,2"));
    ///
    /// url.set_fragment(None);
    /// assert_eq!(url.as_str(), "https://example.com/data.csv");
    /// assert!(url.fragment().is_none());
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    pub fn set_fragment(&mut self, fragment: Option<&str>) {
        if let Some(start) = self.fragment_start {
            debug_assert!(self.byte_at(start) == b'#');
            self.serialization.truncate(start as usize);
        }
        if let Some(input) = fragment {
            self.fragment_start = Some(to_u32(self.serialization.len()).unwrap());
            self.serialization.push('#');
            self.mutate(|parser| parser.parse_fragment(parser::Input::no_trim(input)))
        } else {
            self.fragment_start = None
        }
    }
    fn take_fragment(&mut self) -> Option<String> {
        self.fragment_start
            .take()
            .map(|start| {
                debug_assert!(self.byte_at(start) == b'#');
                let fragment = self.slice(start + 1..).to_owned();
                self.serialization.truncate(start as usize);
                fragment
            })
    }
    fn restore_already_parsed_fragment(&mut self, fragment: Option<String>) {
        if let Some(ref fragment) = fragment {
            assert!(self.fragment_start.is_none());
            self.fragment_start = Some(to_u32(self.serialization.len()).unwrap());
            self.serialization.push('#');
            self.serialization.push_str(fragment);
        }
    }
    /// Change this URL’s query string.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use url::Url;
    /// # use url::ParseError;
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let mut url = Url::parse("https://example.com/products")?;
    /// assert_eq!(url.as_str(), "https://example.com/products");
    ///
    /// url.set_query(Some("page=2"));
    /// assert_eq!(url.as_str(), "https://example.com/products?page=2");
    /// assert_eq!(url.query(), Some("page=2"));
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    pub fn set_query(&mut self, query: Option<&str>) {
        let fragment = self.take_fragment();
        if let Some(start) = self.query_start.take() {
            debug_assert!(self.byte_at(start) == b'?');
            self.serialization.truncate(start as usize);
        }
        if let Some(input) = query {
            self.query_start = Some(to_u32(self.serialization.len()).unwrap());
            self.serialization.push('?');
            let scheme_type = SchemeType::from(self.scheme());
            let scheme_end = self.scheme_end;
            self.mutate(|parser| {
                let vfn = parser.violation_fn;
                parser
                    .parse_query(
                        scheme_type,
                        scheme_end,
                        parser::Input::trim_tab_and_newlines(input, vfn),
                    )
            });
        }
        self.restore_already_parsed_fragment(fragment);
    }
    /// Manipulate this URL’s query string, viewed as a sequence of name/value pairs
    /// in `application/x-www-form-urlencoded` syntax.
    ///
    /// The return value has a method-chaining API:
    ///
    /// ```rust
    /// # use url::{Url, ParseError};
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let mut url = Url::parse("https://example.net?lang=fr#nav")?;
    /// assert_eq!(url.query(), Some("lang=fr"));
    ///
    /// url.query_pairs_mut().append_pair("foo", "bar");
    /// assert_eq!(url.query(), Some("lang=fr&foo=bar"));
    /// assert_eq!(url.as_str(), "https://example.net/?lang=fr&foo=bar#nav");
    ///
    /// url.query_pairs_mut()
    ///     .clear()
    ///     .append_pair("foo", "bar & baz")
    ///     .append_pair("saisons", "\u{00C9}t\u{00E9}+hiver");
    /// assert_eq!(url.query(), Some("foo=bar+%26+baz&saisons=%C3%89t%C3%A9%2Bhiver"));
    /// assert_eq!(url.as_str(),
    ///            "https://example.net/?foo=bar+%26+baz&saisons=%C3%89t%C3%A9%2Bhiver#nav");
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    ///
    /// Note: `url.query_pairs_mut().clear();` is equivalent to `url.set_query(Some(""))`,
    /// not `url.set_query(None)`.
    ///
    /// The state of `Url` is unspecified if this return value is leaked without being dropped.
    pub fn query_pairs_mut(&mut self) -> form_urlencoded::Serializer<'_, UrlQuery<'_>> {
        let fragment = self.take_fragment();
        let query_start;
        if let Some(start) = self.query_start {
            debug_assert!(self.byte_at(start) == b'?');
            query_start = start as usize;
        } else {
            query_start = self.serialization.len();
            self.query_start = Some(to_u32(query_start).unwrap());
            self.serialization.push('?');
        }
        let query = UrlQuery {
            url: Some(self),
            fragment,
        };
        form_urlencoded::Serializer::for_suffix(query, query_start + "?".len())
    }
    fn take_after_path(&mut self) -> String {
        match (self.query_start, self.fragment_start) {
            (Some(i), _) | (None, Some(i)) => {
                let after_path = self.slice(i..).to_owned();
                self.serialization.truncate(i as usize);
                after_path
            }
            (None, None) => String::new(),
        }
    }
    /// Change this URL’s path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use url::Url;
    /// # use url::ParseError;
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let mut url = Url::parse("https://example.com")?;
    /// url.set_path("api/comments");
    /// assert_eq!(url.as_str(), "https://example.com/api/comments");
    /// assert_eq!(url.path(), "/api/comments");
    ///
    /// let mut url = Url::parse("https://example.com/api")?;
    /// url.set_path("data/report.csv");
    /// assert_eq!(url.as_str(), "https://example.com/data/report.csv");
    /// assert_eq!(url.path(), "/data/report.csv");
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    pub fn set_path(&mut self, mut path: &str) {
        let after_path = self.take_after_path();
        let old_after_path_pos = to_u32(self.serialization.len()).unwrap();
        let cannot_be_a_base = self.cannot_be_a_base();
        let scheme_type = SchemeType::from(self.scheme());
        self.serialization.truncate(self.path_start as usize);
        self.mutate(|parser| {
            if cannot_be_a_base {
                if path.starts_with('/') {
                    parser.serialization.push_str("%2F");
                    path = &path[1..];
                }
                parser.parse_cannot_be_a_base_path(parser::Input::new(path));
            } else {
                let mut has_host = true;
                parser
                    .parse_path_start(
                        scheme_type,
                        &mut has_host,
                        parser::Input::new(path),
                    );
            }
        });
        self.restore_after_path(old_after_path_pos, &after_path);
    }
    /// Return an object with methods to manipulate this URL’s path segments.
    ///
    /// Return `Err(())` if this URL is cannot-be-a-base.
    pub fn path_segments_mut(&mut self) -> Result<PathSegmentsMut<'_>, ()> {
        if self.cannot_be_a_base() { Err(()) } else { Ok(path_segments::new(self)) }
    }
    fn restore_after_path(&mut self, old_after_path_position: u32, after_path: &str) {
        let new_after_path_position = to_u32(self.serialization.len()).unwrap();
        let adjust = |index: &mut u32| {
            *index -= old_after_path_position;
            *index += new_after_path_position;
        };
        if let Some(ref mut index) = self.query_start {
            adjust(index)
        }
        if let Some(ref mut index) = self.fragment_start {
            adjust(index)
        }
        self.serialization.push_str(after_path)
    }
    /// Change this URL’s port number.
    ///
    /// Note that default port numbers are not reflected in the serialization.
    ///
    /// If this URL is cannot-be-a-base, does not have a host, or has the `file` scheme;
    /// do nothing and return `Err`.
    ///
    /// # Examples
    ///
    /// ```
    /// use url::Url;
    /// # use std::error::Error;
    ///
    /// # fn run() -> Result<(), Box<dyn Error>> {
    /// let mut url = Url::parse("ssh://example.net:2048/")?;
    ///
    /// url.set_port(Some(4096)).map_err(|_| "cannot be base")?;
    /// assert_eq!(url.as_str(), "ssh://example.net:4096/");
    ///
    /// url.set_port(None).map_err(|_| "cannot be base")?;
    /// assert_eq!(url.as_str(), "ssh://example.net/");
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    ///
    /// Known default port numbers are not reflected:
    ///
    /// ```rust
    /// use url::Url;
    /// # use std::error::Error;
    ///
    /// # fn run() -> Result<(), Box<dyn Error>> {
    /// let mut url = Url::parse("https://example.org/")?;
    ///
    /// url.set_port(Some(443)).map_err(|_| "cannot be base")?;
    /// assert!(url.port().is_none());
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    ///
    /// Cannot set port for cannot-be-a-base URLs:
    ///
    /// ```
    /// use url::Url;
    /// # use url::ParseError;
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let mut url = Url::parse("mailto:rms@example.net")?;
    ///
    /// let result = url.set_port(Some(80));
    /// assert!(result.is_err());
    ///
    /// let result = url.set_port(None);
    /// assert!(result.is_err());
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    pub fn set_port(&mut self, mut port: Option<u16>) -> Result<(), ()> {
        if !self.has_host() || self.host() == Some(Host::Domain(""))
            || self.scheme() == "file"
        {
            return Err(());
        }
        if port.is_some() && port == parser::default_port(self.scheme()) {
            port = None;
        }
        self.set_port_internal(port);
        Ok(())
    }
    fn set_port_internal(&mut self, port: Option<u16>) {
        match (self.port, port) {
            (None, None) => {}
            (Some(_), None) => {
                self.serialization
                    .drain(self.host_end as usize..self.path_start as usize);
                let offset = self.path_start - self.host_end;
                self.path_start = self.host_end;
                if let Some(ref mut index) = self.query_start {
                    *index -= offset;
                }
                if let Some(ref mut index) = self.fragment_start {
                    *index -= offset;
                }
            }
            (Some(old), Some(new)) if old == new => {}
            (_, Some(new)) => {
                let path_and_after = self.slice(self.path_start..).to_owned();
                self.serialization.truncate(self.host_end as usize);
                write!(& mut self.serialization, ":{}", new).unwrap();
                let old_path_start = self.path_start;
                let new_path_start = to_u32(self.serialization.len()).unwrap();
                self.path_start = new_path_start;
                let adjust = |index: &mut u32| {
                    *index -= old_path_start;
                    *index += new_path_start;
                };
                if let Some(ref mut index) = self.query_start {
                    adjust(index)
                }
                if let Some(ref mut index) = self.fragment_start {
                    adjust(index)
                }
                self.serialization.push_str(&path_and_after);
            }
        }
        self.port = port;
    }
    /// Change this URL’s host.
    ///
    /// Removing the host (calling this with `None`)
    /// will also remove any username, password, and port number.
    ///
    /// # Examples
    ///
    /// Change host:
    ///
    /// ```
    /// use url::Url;
    /// # use url::ParseError;
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let mut url = Url::parse("https://example.net")?;
    /// let result = url.set_host(Some("rust-lang.org"));
    /// assert!(result.is_ok());
    /// assert_eq!(url.as_str(), "https://rust-lang.org/");
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    ///
    /// Remove host:
    ///
    /// ```
    /// use url::Url;
    /// # use url::ParseError;
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let mut url = Url::parse("foo://example.net")?;
    /// let result = url.set_host(None);
    /// assert!(result.is_ok());
    /// assert_eq!(url.as_str(), "foo:/");
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    ///
    /// Cannot remove host for 'special' schemes (e.g. `http`):
    ///
    /// ```
    /// use url::Url;
    /// # use url::ParseError;
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let mut url = Url::parse("https://example.net")?;
    /// let result = url.set_host(None);
    /// assert!(result.is_err());
    /// assert_eq!(url.as_str(), "https://example.net/");
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    ///
    /// Cannot change or remove host for cannot-be-a-base URLs:
    ///
    /// ```
    /// use url::Url;
    /// # use url::ParseError;
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let mut url = Url::parse("mailto:rms@example.net")?;
    ///
    /// let result = url.set_host(Some("rust-lang.org"));
    /// assert!(result.is_err());
    /// assert_eq!(url.as_str(), "mailto:rms@example.net");
    ///
    /// let result = url.set_host(None);
    /// assert!(result.is_err());
    /// assert_eq!(url.as_str(), "mailto:rms@example.net");
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// If this URL is cannot-be-a-base or there is an error parsing the given `host`,
    /// a [`ParseError`] variant will be returned.
    ///
    /// [`ParseError`]: enum.ParseError.html
    pub fn set_host(&mut self, host: Option<&str>) -> Result<(), ParseError> {
        if self.cannot_be_a_base() {
            return Err(ParseError::SetHostOnCannotBeABaseUrl);
        }
        if let Some(host) = host {
            if host == "" && SchemeType::from(self.scheme()).is_special() {
                return Err(ParseError::EmptyHost);
            }
            let mut host_substr = host;
            if !host.starts_with('[') || !host.ends_with(']') {
                match host.find(':') {
                    Some(0) => {
                        return Err(ParseError::InvalidDomainCharacter);
                    }
                    Some(colon_index) => {
                        host_substr = &host[..colon_index];
                    }
                    None => {}
                }
            }
            if SchemeType::from(self.scheme()).is_special() {
                self.set_host_internal(Host::parse(host_substr)?, None);
            } else {
                self.set_host_internal(Host::parse_opaque(host_substr)?, None);
            }
        } else if self.has_host() {
            let scheme_type = SchemeType::from(self.scheme());
            if scheme_type.is_special() {
                return Err(ParseError::EmptyHost);
            } else if self.serialization.len() == self.path_start as usize {
                self.serialization.push('/');
            }
            debug_assert!(self.byte_at(self.scheme_end) == b':');
            debug_assert!(self.byte_at(self.path_start) == b'/');
            let new_path_start = self.scheme_end + 1;
            self.serialization.drain(new_path_start as usize..self.path_start as usize);
            let offset = self.path_start - new_path_start;
            self.path_start = new_path_start;
            self.username_end = new_path_start;
            self.host_start = new_path_start;
            self.host_end = new_path_start;
            self.port = None;
            if let Some(ref mut index) = self.query_start {
                *index -= offset;
            }
            if let Some(ref mut index) = self.fragment_start {
                *index -= offset;
            }
        }
        Ok(())
    }
    /// opt_new_port: None means leave unchanged, Some(None) means remove any port number.
    fn set_host_internal(
        &mut self,
        host: Host<String>,
        opt_new_port: Option<Option<u16>>,
    ) {
        let old_suffix_pos = if opt_new_port.is_some() {
            self.path_start
        } else {
            self.host_end
        };
        let suffix = self.slice(old_suffix_pos..).to_owned();
        self.serialization.truncate(self.host_start as usize);
        if !self.has_authority() {
            debug_assert!(self.slice(self.scheme_end..self.host_start) == ":");
            debug_assert!(self.username_end == self.host_start);
            self.serialization.push('/');
            self.serialization.push('/');
            self.username_end += 2;
            self.host_start += 2;
        }
        write!(& mut self.serialization, "{}", host).unwrap();
        self.host_end = to_u32(self.serialization.len()).unwrap();
        self.host = host.into();
        if let Some(new_port) = opt_new_port {
            self.port = new_port;
            if let Some(port) = new_port {
                write!(& mut self.serialization, ":{}", port).unwrap();
            }
        }
        let new_suffix_pos = to_u32(self.serialization.len()).unwrap();
        self.serialization.push_str(&suffix);
        let adjust = |index: &mut u32| {
            *index -= old_suffix_pos;
            *index += new_suffix_pos;
        };
        adjust(&mut self.path_start);
        if let Some(ref mut index) = self.query_start {
            adjust(index)
        }
        if let Some(ref mut index) = self.fragment_start {
            adjust(index)
        }
    }
    /// Change this URL’s host to the given IP address.
    ///
    /// If this URL is cannot-be-a-base, do nothing and return `Err`.
    ///
    /// Compared to `Url::set_host`, this skips the host parser.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use url::{Url, ParseError};
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let mut url = Url::parse("http://example.com")?;
    /// url.set_ip_host("127.0.0.1".parse().unwrap());
    /// assert_eq!(url.host_str(), Some("127.0.0.1"));
    /// assert_eq!(url.as_str(), "http://127.0.0.1/");
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    ///
    /// Cannot change URL's from mailto(cannot-be-base) to ip:
    ///
    /// ```rust
    /// use url::{Url, ParseError};
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let mut url = Url::parse("mailto:rms@example.com")?;
    /// let result = url.set_ip_host("127.0.0.1".parse().unwrap());
    ///
    /// assert_eq!(url.as_str(), "mailto:rms@example.com");
    /// assert!(result.is_err());
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    ///
    pub fn set_ip_host(&mut self, address: IpAddr) -> Result<(), ()> {
        if self.cannot_be_a_base() {
            return Err(());
        }
        let address = match address {
            IpAddr::V4(address) => Host::Ipv4(address),
            IpAddr::V6(address) => Host::Ipv6(address),
        };
        self.set_host_internal(address, None);
        Ok(())
    }
    /// Change this URL’s password.
    ///
    /// If this URL is cannot-be-a-base or does not have a host, do nothing and return `Err`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use url::{Url, ParseError};
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let mut url = Url::parse("mailto:rmz@example.com")?;
    /// let result = url.set_password(Some("secret_password"));
    /// assert!(result.is_err());
    ///
    /// let mut url = Url::parse("ftp://user1:secret1@example.com")?;
    /// let result = url.set_password(Some("secret_password"));
    /// assert_eq!(url.password(), Some("secret_password"));
    ///
    /// let mut url = Url::parse("ftp://user2:@example.com")?;
    /// let result = url.set_password(Some("secret2"));
    /// assert!(result.is_ok());
    /// assert_eq!(url.password(), Some("secret2"));
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    pub fn set_password(&mut self, password: Option<&str>) -> Result<(), ()> {
        if !self.has_host() || self.host() == Some(Host::Domain(""))
            || self.scheme() == "file"
        {
            return Err(());
        }
        if let Some(password) = password {
            let host_and_after = self.slice(self.host_start..).to_owned();
            self.serialization.truncate(self.username_end as usize);
            self.serialization.push(':');
            self.serialization.extend(utf8_percent_encode(password, USERINFO));
            self.serialization.push('@');
            let old_host_start = self.host_start;
            let new_host_start = to_u32(self.serialization.len()).unwrap();
            let adjust = |index: &mut u32| {
                *index -= old_host_start;
                *index += new_host_start;
            };
            self.host_start = new_host_start;
            adjust(&mut self.host_end);
            adjust(&mut self.path_start);
            if let Some(ref mut index) = self.query_start {
                adjust(index)
            }
            if let Some(ref mut index) = self.fragment_start {
                adjust(index)
            }
            self.serialization.push_str(&host_and_after);
        } else if self.byte_at(self.username_end) == b':' {
            let has_username_or_password = self.byte_at(self.host_start - 1) == b'@';
            debug_assert!(has_username_or_password);
            let username_start = self.scheme_end + 3;
            let empty_username = username_start == self.username_end;
            let start = self.username_end;
            let end = if empty_username { self.host_start } else { self.host_start - 1 };
            self.serialization.drain(start as usize..end as usize);
            let offset = end - start;
            self.host_start -= offset;
            self.host_end -= offset;
            self.path_start -= offset;
            if let Some(ref mut index) = self.query_start {
                *index -= offset;
            }
            if let Some(ref mut index) = self.fragment_start {
                *index -= offset;
            }
        }
        Ok(())
    }
    /// Change this URL’s username.
    ///
    /// If this URL is cannot-be-a-base or does not have a host, do nothing and return `Err`.
    /// # Examples
    ///
    /// Cannot setup username from mailto(cannot-be-base)
    ///
    /// ```rust
    /// use url::{Url, ParseError};
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let mut url = Url::parse("mailto:rmz@example.com")?;
    /// let result = url.set_username("user1");
    /// assert_eq!(url.as_str(), "mailto:rmz@example.com");
    /// assert!(result.is_err());
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    ///
    /// Setup username to user1
    ///
    /// ```rust
    /// use url::{Url, ParseError};
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let mut url = Url::parse("ftp://:secre1@example.com/")?;
    /// let result = url.set_username("user1");
    /// assert!(result.is_ok());
    /// assert_eq!(url.username(), "user1");
    /// assert_eq!(url.as_str(), "ftp://user1:secre1@example.com/");
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    pub fn set_username(&mut self, username: &str) -> Result<(), ()> {
        if !self.has_host() || self.host() == Some(Host::Domain(""))
            || self.scheme() == "file"
        {
            return Err(());
        }
        let username_start = self.scheme_end + 3;
        debug_assert!(self.slice(self.scheme_end..username_start) == "://");
        if self.slice(username_start..self.username_end) == username {
            return Ok(());
        }
        let after_username = self.slice(self.username_end..).to_owned();
        self.serialization.truncate(username_start as usize);
        self.serialization.extend(utf8_percent_encode(username, USERINFO));
        let mut removed_bytes = self.username_end;
        self.username_end = to_u32(self.serialization.len()).unwrap();
        let mut added_bytes = self.username_end;
        let new_username_is_empty = self.username_end == username_start;
        match (new_username_is_empty, after_username.chars().next()) {
            (true, Some('@')) => {
                removed_bytes += 1;
                self.serialization.push_str(&after_username[1..]);
            }
            (false, Some('@')) | (_, Some(':')) | (true, _) => {
                self.serialization.push_str(&after_username);
            }
            (false, _) => {
                added_bytes += 1;
                self.serialization.push('@');
                self.serialization.push_str(&after_username);
            }
        }
        let adjust = |index: &mut u32| {
            *index -= removed_bytes;
            *index += added_bytes;
        };
        adjust(&mut self.host_start);
        adjust(&mut self.host_end);
        adjust(&mut self.path_start);
        if let Some(ref mut index) = self.query_start {
            adjust(index)
        }
        if let Some(ref mut index) = self.fragment_start {
            adjust(index)
        }
        Ok(())
    }
    /// Change this URL’s scheme.
    ///
    /// Do nothing and return `Err` under the following circumstances:
    ///
    /// * If the new scheme is not in `[a-zA-Z][a-zA-Z0-9+.-]+`
    /// * If this URL is cannot-be-a-base and the new scheme is one of
    ///   `http`, `https`, `ws`, `wss` or `ftp`
    /// * If either the old or new scheme is `http`, `https`, `ws`,
    ///   `wss` or `ftp` and the other is not one of these
    /// * If the new scheme is `file` and this URL includes credentials
    ///   or has a non-null port
    /// * If this URL's scheme is `file` and its host is empty or null
    ///
    /// See also [the URL specification's section on legal scheme state
    /// overrides](https://url.spec.whatwg.org/#scheme-state).
    ///
    /// # Examples
    ///
    /// Change the URL’s scheme from `https` to `foo`:
    ///
    /// ```
    /// use url::Url;
    /// # use url::ParseError;
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let mut url = Url::parse("https://example.net")?;
    /// let result = url.set_scheme("http");
    /// assert_eq!(url.as_str(), "http://example.net/");
    /// assert!(result.is_ok());
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    /// Change the URL’s scheme from `foo` to `bar`:
    ///
    /// ```
    /// use url::Url;
    /// # use url::ParseError;
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let mut url = Url::parse("foo://example.net")?;
    /// let result = url.set_scheme("bar");
    /// assert_eq!(url.as_str(), "bar://example.net");
    /// assert!(result.is_ok());
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    ///
    /// Cannot change URL’s scheme from `https` to `foõ`:
    ///
    /// ```
    /// use url::Url;
    /// # use url::ParseError;
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let mut url = Url::parse("https://example.net")?;
    /// let result = url.set_scheme("foõ");
    /// assert_eq!(url.as_str(), "https://example.net/");
    /// assert!(result.is_err());
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    ///
    /// Cannot change URL’s scheme from `mailto` (cannot-be-a-base) to `https`:
    ///
    /// ```
    /// use url::Url;
    /// # use url::ParseError;
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let mut url = Url::parse("mailto:rms@example.net")?;
    /// let result = url.set_scheme("https");
    /// assert_eq!(url.as_str(), "mailto:rms@example.net");
    /// assert!(result.is_err());
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    /// Cannot change the URL’s scheme from `foo` to `https`:
    ///
    /// ```
    /// use url::Url;
    /// # use url::ParseError;
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let mut url = Url::parse("foo://example.net")?;
    /// let result = url.set_scheme("https");
    /// assert_eq!(url.as_str(), "foo://example.net");
    /// assert!(result.is_err());
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    /// Cannot change the URL’s scheme from `http` to `foo`:
    ///
    /// ```
    /// use url::Url;
    /// # use url::ParseError;
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let mut url = Url::parse("http://example.net")?;
    /// let result = url.set_scheme("foo");
    /// assert_eq!(url.as_str(), "http://example.net/");
    /// assert!(result.is_err());
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    pub fn set_scheme(&mut self, scheme: &str) -> Result<(), ()> {
        let mut parser = Parser::for_setter(String::new());
        let remaining = parser.parse_scheme(parser::Input::new(scheme))?;
        let new_scheme_type = SchemeType::from(&parser.serialization);
        let old_scheme_type = SchemeType::from(self.scheme());
        if (new_scheme_type.is_special() && !old_scheme_type.is_special())
            || (!new_scheme_type.is_special() && old_scheme_type.is_special())
            || (new_scheme_type.is_file() && self.has_authority())
        {
            return Err(());
        }
        if !remaining.is_empty() || (!self.has_host() && new_scheme_type.is_special()) {
            return Err(());
        }
        let old_scheme_end = self.scheme_end;
        let new_scheme_end = to_u32(parser.serialization.len()).unwrap();
        let adjust = |index: &mut u32| {
            *index -= old_scheme_end;
            *index += new_scheme_end;
        };
        self.scheme_end = new_scheme_end;
        adjust(&mut self.username_end);
        adjust(&mut self.host_start);
        adjust(&mut self.host_end);
        adjust(&mut self.path_start);
        if let Some(ref mut index) = self.query_start {
            adjust(index)
        }
        if let Some(ref mut index) = self.fragment_start {
            adjust(index)
        }
        parser.serialization.push_str(self.slice(old_scheme_end..));
        self.serialization = parser.serialization;
        let previous_port = self.port();
        let _ = self.set_port(previous_port);
        Ok(())
    }
    /// Convert a file name as `std::path::Path` into an URL in the `file` scheme.
    ///
    /// This returns `Err` if the given path is not absolute or,
    /// on Windows, if the prefix is not a disk prefix (e.g. `C:`) or a UNC prefix (`\\`).
    ///
    /// # Examples
    ///
    /// On Unix-like platforms:
    ///
    /// ```
    /// # if cfg!(unix) {
    /// use url::Url;
    ///
    /// # fn run() -> Result<(), ()> {
    /// let url = Url::from_file_path("/tmp/foo.txt")?;
    /// assert_eq!(url.as_str(), "file:///tmp/foo.txt");
    ///
    /// let url = Url::from_file_path("../foo.txt");
    /// assert!(url.is_err());
    ///
    /// let url = Url::from_file_path("https://google.com/");
    /// assert!(url.is_err());
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// # }
    /// ```
    #[cfg(any(unix, windows, target_os = "redox"))]
    pub fn from_file_path<P: AsRef<Path>>(path: P) -> Result<Url, ()> {
        let mut serialization = "file://".to_owned();
        let host_start = serialization.len() as u32;
        let (host_end, host) = path_to_file_url_segments(
            path.as_ref(),
            &mut serialization,
        )?;
        Ok(Url {
            serialization,
            scheme_end: "file".len() as u32,
            username_end: host_start,
            host_start,
            host_end,
            host,
            port: None,
            path_start: host_end,
            query_start: None,
            fragment_start: None,
        })
    }
    /// Convert a directory name as `std::path::Path` into an URL in the `file` scheme.
    ///
    /// This returns `Err` if the given path is not absolute or,
    /// on Windows, if the prefix is not a disk prefix (e.g. `C:`) or a UNC prefix (`\\`).
    ///
    /// Compared to `from_file_path`, this ensure that URL’s the path has a trailing slash
    /// so that the entire path is considered when using this URL as a base URL.
    ///
    /// For example:
    ///
    /// * `"index.html"` parsed with `Url::from_directory_path(Path::new("/var/www"))`
    ///   as the base URL is `file:///var/www/index.html`
    /// * `"index.html"` parsed with `Url::from_file_path(Path::new("/var/www"))`
    ///   as the base URL is `file:///var/index.html`, which might not be what was intended.
    ///
    /// Note that `std::path` does not consider trailing slashes significant
    /// and usually does not include them (e.g. in `Path::parent()`).
    #[cfg(any(unix, windows, target_os = "redox"))]
    pub fn from_directory_path<P: AsRef<Path>>(path: P) -> Result<Url, ()> {
        let mut url = Url::from_file_path(path)?;
        if !url.serialization.ends_with('/') {
            url.serialization.push('/')
        }
        Ok(url)
    }
    /// Serialize with Serde using the internal representation of the `Url` struct.
    ///
    /// The corresponding `deserialize_internal` method sacrifices some invariant-checking
    /// for speed, compared to the `Deserialize` trait impl.
    ///
    /// This method is only available if the `serde` Cargo feature is enabled.
    #[cfg(feature = "serde")]
    #[deny(unused)]
    pub fn serialize_internal<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::Serialize;
        let Url {
            ref serialization,
            ref scheme_end,
            ref username_end,
            ref host_start,
            ref host_end,
            ref host,
            ref port,
            ref path_start,
            ref query_start,
            ref fragment_start,
        } = *self;
        (
            serialization,
            scheme_end,
            username_end,
            host_start,
            host_end,
            host,
            port,
            path_start,
            query_start,
            fragment_start,
        )
            .serialize(serializer)
    }
    /// Serialize with Serde using the internal representation of the `Url` struct.
    ///
    /// The corresponding `deserialize_internal` method sacrifices some invariant-checking
    /// for speed, compared to the `Deserialize` trait impl.
    ///
    /// This method is only available if the `serde` Cargo feature is enabled.
    #[cfg(feature = "serde")]
    #[deny(unused)]
    pub fn deserialize_internal<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{Deserialize, Error, Unexpected};
        let (
            serialization,
            scheme_end,
            username_end,
            host_start,
            host_end,
            host,
            port,
            path_start,
            query_start,
            fragment_start,
        ) = Deserialize::deserialize(deserializer)?;
        let url = Url {
            serialization,
            scheme_end,
            username_end,
            host_start,
            host_end,
            host,
            port,
            path_start,
            query_start,
            fragment_start,
        };
        if cfg!(debug_assertions) {
            url.check_invariants()
                .map_err(|reason| {
                    let reason: &str = &reason;
                    Error::invalid_value(Unexpected::Other("value"), &reason)
                })?
        }
        Ok(url)
    }
    /// Assuming the URL is in the `file` scheme or similar,
    /// convert its path to an absolute `std::path::Path`.
    ///
    /// **Note:** This does not actually check the URL’s `scheme`,
    /// and may give nonsensical results for other schemes.
    /// It is the user’s responsibility to check the URL’s scheme before calling this.
    ///
    /// ```
    /// # use url::Url;
    /// # let url = Url::parse("file:///etc/passwd").unwrap();
    /// let path = url.to_file_path();
    /// ```
    ///
    /// Returns `Err` if the host is neither empty nor `"localhost"` (except on Windows, where
    /// `file:` URLs may have a non-local host),
    /// or if `Path::new_opt()` returns `None`.
    /// (That is, if the percent-decoded path contains a NUL byte or,
    /// for a Windows path, is not UTF-8.)
    #[inline]
    #[cfg(any(unix, windows, target_os = "redox"))]
    pub fn to_file_path(&self) -> Result<PathBuf, ()> {
        if let Some(segments) = self.path_segments() {
            let host = match self.host() {
                None | Some(Host::Domain("localhost")) => None,
                Some(_) if cfg!(windows) && self.scheme() == "file" => {
                    Some(
                        &self
                            .serialization[self.host_start
                            as usize..self.host_end as usize],
                    )
                }
                _ => return Err(()),
            };
            return file_url_segments_to_pathbuf(host, segments);
        }
        Err(())
    }
    #[inline]
    fn slice<R>(&self, range: R) -> &str
    where
        R: RangeArg,
    {
        range.slice_of(&self.serialization)
    }
    #[inline]
    fn byte_at(&self, i: u32) -> u8 {
        self.serialization.as_bytes()[i as usize]
    }
}
/// Parse a string as an URL, without a base URL or encoding override.
impl str::FromStr for Url {
    type Err = ParseError;
    #[inline]
    fn from_str(input: &str) -> Result<Url, crate::ParseError> {
        Url::parse(input)
    }
}
impl<'a> TryFrom<&'a str> for Url {
    type Error = ParseError;
    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        Url::parse(s)
    }
}
/// Display the serialization of this URL.
impl fmt::Display for Url {
    #[inline]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.serialization, formatter)
    }
}
/// Debug the serialization of this URL.
impl fmt::Debug for Url {
    #[inline]
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter
            .debug_struct("Url")
            .field("scheme", &self.scheme())
            .field("host", &self.host())
            .field("port", &self.port())
            .field("path", &self.path())
            .field("query", &self.query())
            .field("fragment", &self.fragment())
            .finish()
    }
}
/// URLs compare like their serialization.
impl Eq for Url {}
/// URLs compare like their serialization.
impl PartialEq for Url {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.serialization == other.serialization
    }
}
/// URLs compare like their serialization.
impl Ord for Url {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.serialization.cmp(&other.serialization)
    }
}
/// URLs compare like their serialization.
impl PartialOrd for Url {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.serialization.partial_cmp(&other.serialization)
    }
}
/// URLs hash like their serialization.
impl hash::Hash for Url {
    #[inline]
    fn hash<H>(&self, state: &mut H)
    where
        H: hash::Hasher,
    {
        hash::Hash::hash(&self.serialization, state)
    }
}
/// Return the serialization of this URL.
impl AsRef<str> for Url {
    #[inline]
    fn as_ref(&self) -> &str {
        &self.serialization
    }
}
trait RangeArg {
    fn slice_of<'a>(&self, s: &'a str) -> &'a str;
}
impl RangeArg for Range<u32> {
    #[inline]
    fn slice_of<'a>(&self, s: &'a str) -> &'a str {
        &s[self.start as usize..self.end as usize]
    }
}
impl RangeArg for RangeFrom<u32> {
    #[inline]
    fn slice_of<'a>(&self, s: &'a str) -> &'a str {
        &s[self.start as usize..]
    }
}
impl RangeArg for RangeTo<u32> {
    #[inline]
    fn slice_of<'a>(&self, s: &'a str) -> &'a str {
        &s[..self.end as usize]
    }
}
/// Serializes this URL into a `serde` stream.
///
/// This implementation is only available if the `serde` Cargo feature is enabled.
#[cfg(feature = "serde")]
impl serde::Serialize for Url {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}
/// Deserializes this URL from a `serde` stream.
///
/// This implementation is only available if the `serde` Cargo feature is enabled.
#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Url {
    fn deserialize<D>(deserializer: D) -> Result<Url, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{Error, Unexpected, Visitor};
        struct UrlVisitor;
        impl<'de> Visitor<'de> for UrlVisitor {
            type Value = Url;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string representing an URL")
            }
            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Url::parse(s)
                    .map_err(|err| Error::invalid_value(
                        Unexpected::Str(s),
                        &err.description(),
                    ))
            }
        }
        deserializer.deserialize_str(UrlVisitor)
    }
}
#[cfg(any(unix, target_os = "redox"))]
fn path_to_file_url_segments(
    path: &Path,
    serialization: &mut String,
) -> Result<(u32, HostInternal), ()> {
    use std::os::unix::prelude::OsStrExt;
    if !path.is_absolute() {
        return Err(());
    }
    let host_end = to_u32(serialization.len()).unwrap();
    let mut empty = true;
    for component in path.components().skip(1) {
        empty = false;
        serialization.push('/');
        serialization
            .extend(percent_encode(component.as_os_str().as_bytes(), PATH_SEGMENT));
    }
    if empty {
        serialization.push('/');
    }
    Ok((host_end, HostInternal::None))
}
#[cfg(windows)]
fn path_to_file_url_segments(
    path: &Path,
    serialization: &mut String,
) -> Result<(u32, HostInternal), ()> {
    path_to_file_url_segments_windows(path, serialization)
}
#[cfg_attr(not(windows), allow(dead_code))]
fn path_to_file_url_segments_windows(
    path: &Path,
    serialization: &mut String,
) -> Result<(u32, HostInternal), ()> {
    use std::path::{Component, Prefix};
    if !path.is_absolute() {
        return Err(());
    }
    let mut components = path.components();
    let host_start = serialization.len() + 1;
    let host_end;
    let host_internal;
    match components.next() {
        Some(Component::Prefix(ref p)) => {
            match p.kind() {
                Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => {
                    host_end = to_u32(serialization.len()).unwrap();
                    host_internal = HostInternal::None;
                    serialization.push('/');
                    serialization.push(letter as char);
                    serialization.push(':');
                }
                Prefix::UNC(server, share) | Prefix::VerbatimUNC(server, share) => {
                    let host = Host::parse(server.to_str().ok_or(())?).map_err(|_| ())?;
                    write!(serialization, "{}", host).unwrap();
                    host_end = to_u32(serialization.len()).unwrap();
                    host_internal = host.into();
                    serialization.push('/');
                    let share = share.to_str().ok_or(())?;
                    serialization.extend(percent_encode(share.as_bytes(), PATH_SEGMENT));
                }
                _ => return Err(()),
            }
        }
        _ => return Err(()),
    }
    let mut path_only_has_prefix = true;
    for component in components {
        if component == Component::RootDir {
            continue;
        }
        path_only_has_prefix = false;
        let component = component.as_os_str().to_str().ok_or(())?;
        serialization.push('/');
        serialization.extend(percent_encode(component.as_bytes(), PATH_SEGMENT));
    }
    if serialization.len() > host_start
        && parser::is_windows_drive_letter(&serialization[host_start..])
        && path_only_has_prefix
    {
        serialization.push('/');
    }
    Ok((host_end, host_internal))
}
#[cfg(any(unix, target_os = "redox"))]
fn file_url_segments_to_pathbuf(
    host: Option<&str>,
    segments: str::Split<'_, char>,
) -> Result<PathBuf, ()> {
    use std::ffi::OsStr;
    use std::os::unix::prelude::OsStrExt;
    if host.is_some() {
        return Err(());
    }
    let mut bytes = if cfg!(target_os = "redox") {
        b"file:".to_vec()
    } else {
        Vec::new()
    };
    for segment in segments {
        bytes.push(b'/');
        bytes.extend(percent_decode(segment.as_bytes()));
    }
    if bytes.len() > 2 && matches!(bytes[bytes.len() - 2], b'a'..= b'z' | b'A'..= b'Z')
        && matches!(bytes[bytes.len() - 1], b':' | b'|')
    {
        bytes.push(b'/');
    }
    let os_str = OsStr::from_bytes(&bytes);
    let path = PathBuf::from(os_str);
    debug_assert!(
        path.is_absolute(), "to_file_path() failed to produce an absolute Path"
    );
    Ok(path)
}
#[cfg(windows)]
fn file_url_segments_to_pathbuf(
    host: Option<&str>,
    segments: str::Split<char>,
) -> Result<PathBuf, ()> {
    file_url_segments_to_pathbuf_windows(host, segments)
}
#[cfg_attr(not(windows), allow(dead_code))]
fn file_url_segments_to_pathbuf_windows(
    host: Option<&str>,
    mut segments: str::Split<'_, char>,
) -> Result<PathBuf, ()> {
    let mut string = if let Some(host) = host {
        r"\\".to_owned() + host
    } else {
        let first = segments.next().ok_or(())?;
        match first.len() {
            2 => {
                if !first.starts_with(parser::ascii_alpha) || first.as_bytes()[1] != b':'
                {
                    return Err(());
                }
                first.to_owned()
            }
            4 => {
                if !first.starts_with(parser::ascii_alpha) {
                    return Err(());
                }
                let bytes = first.as_bytes();
                if bytes[1] != b'%' || bytes[2] != b'3'
                    || (bytes[3] != b'a' && bytes[3] != b'A')
                {
                    return Err(());
                }
                first[0..1].to_owned() + ":"
            }
            _ => return Err(()),
        }
    };
    for segment in segments {
        string.push('\\');
        match String::from_utf8(percent_decode(segment.as_bytes()).collect()) {
            Ok(s) => string.push_str(&s),
            Err(..) => return Err(()),
        }
    }
    let path = PathBuf::from(string);
    debug_assert!(
        path.is_absolute(), "to_file_path() failed to produce an absolute Path"
    );
    Ok(path)
}
/// Implementation detail of `Url::query_pairs_mut`. Typically not used directly.
#[derive(Debug)]
pub struct UrlQuery<'a> {
    url: Option<&'a mut Url>,
    fragment: Option<String>,
}
impl<'a> form_urlencoded::Target for UrlQuery<'a> {
    fn as_mut_string(&mut self) -> &mut String {
        &mut self.url.as_mut().unwrap().serialization
    }
    fn finish(mut self) -> &'a mut Url {
        let url = self.url.take().unwrap();
        url.restore_already_parsed_fragment(self.fragment.take());
        url
    }
    type Finished = &'a mut Url;
}
impl<'a> Drop for UrlQuery<'a> {
    fn drop(&mut self) {
        if let Some(url) = self.url.take() {
            url.restore_already_parsed_fragment(self.fragment.take())
        }
    }
}
#[cfg(test)]
mod tests_llm_16_5 {
    use std::convert::TryFrom;
    use crate::Url;
    #[test]
    fn test_try_from() {
        let _rug_st_tests_llm_16_5_rrrruuuugggg_test_try_from = 0;
        let rug_fuzz_0 = "https://www.example.com";
        let url_str = rug_fuzz_0;
        let url = Url::try_from(url_str).unwrap();
        debug_assert_eq!(url.as_str(), url_str);
        let _rug_ed_tests_llm_16_5_rrrruuuugggg_test_try_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_6 {
    use crate::Url;
    use std::str::FromStr;
    use crate::ParseError;
    #[test]
    fn test_from_str() {
        let _rug_st_tests_llm_16_6_rrrruuuugggg_test_from_str = 0;
        let rug_fuzz_0 = "https://example.com";
        let input = rug_fuzz_0;
        let url = Url::from_str(input).unwrap();
        debug_assert_eq!(url.as_str(), input);
        let _rug_ed_tests_llm_16_6_rrrruuuugggg_test_from_str = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_9 {
    use super::*;
    use crate::*;
    #[test]
    fn test_drop() {
        let _rug_st_tests_llm_16_9_rrrruuuugggg_test_drop = 0;
        let rug_fuzz_0 = "https://example.com/path?param1=value1&param2=value2#fragment";
        let mut url = Url::parse(rug_fuzz_0).unwrap();
        {
            let mut url_query = url.query_pairs_mut();
        }
        let _rug_ed_tests_llm_16_9_rrrruuuugggg_test_drop = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_17 {
    use std::ops::Range;
    use crate::RangeArg;
    #[test]
    fn test_slice_of() {
        let _rug_st_tests_llm_16_17_rrrruuuugggg_test_slice_of = 0;
        let rug_fuzz_0 = 2;
        let rug_fuzz_1 = 5;
        let rug_fuzz_2 = "Hello, world!";
        let range = Range {
            start: rug_fuzz_0,
            end: rug_fuzz_1,
        };
        let s = rug_fuzz_2;
        let result = range.slice_of(s);
        debug_assert_eq!(result, "llo");
        let _rug_ed_tests_llm_16_17_rrrruuuugggg_test_slice_of = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_18 {
    use std::ops::RangeFrom;
    use crate::RangeArg;
    #[test]
    fn test_slice_of() {
        let _rug_st_tests_llm_16_18_rrrruuuugggg_test_slice_of = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = "1234567890";
        let range = RangeFrom::<u32> {
            start: rug_fuzz_0,
        };
        let s = rug_fuzz_1;
        let result = range.slice_of(s);
        debug_assert_eq!(result, "67890");
        let _rug_ed_tests_llm_16_18_rrrruuuugggg_test_slice_of = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_21 {
    use super::*;
    use crate::*;
    #[test]
    fn test_base_url() {
        let _rug_st_tests_llm_16_21_rrrruuuugggg_test_base_url = 0;
        let rug_fuzz_0 = "https://example.com";
        let mut options = ParseOptions::<'static> {
            base_url: None,
            encoding_override: EncodingOverride::None,
            violation_fn: None,
        };
        let new_url = Url::parse(rug_fuzz_0).unwrap();
        options = options.base_url(Some(&new_url));
        debug_assert_eq!(options.base_url, Some(& new_url));
        let _rug_ed_tests_llm_16_21_rrrruuuugggg_test_base_url = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_26 {
    use crate::{Url, SyntaxViolation};
    use std::cell::RefCell;
    #[test]
    fn test_syntax_violation_callback() {
        let _rug_st_tests_llm_16_26_rrrruuuugggg_test_syntax_violation_callback = 0;
        let rug_fuzz_0 = "https:////example.com";
        let violations = RefCell::new(Vec::new());
        let url = Url::options()
            .syntax_violation_callback(Some(&|v| violations.borrow_mut().push(v)))
            .parse(rug_fuzz_0)
            .unwrap();
        debug_assert_eq!(url.as_str(), "https://example.com/");
        debug_assert_eq!(
            violations.into_inner(), vec![SyntaxViolation::ExpectedDoubleSlash]
        );
        let _rug_ed_tests_llm_16_26_rrrruuuugggg_test_syntax_violation_callback = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_27 {
    use super::*;
    use crate::*;
    #[test]
    fn test_options() {
        let _rug_st_tests_llm_16_27_rrrruuuugggg_test_options = 0;
        let rug_fuzz_0 = "https://api.example.com";
        let rug_fuzz_1 = "version.json";
        let options = Url::options();
        let api = Url::parse(rug_fuzz_0).unwrap();
        let base_url = options.base_url(Some(&api));
        let version_url = base_url.parse(rug_fuzz_1).unwrap();
        debug_assert_eq!(version_url.as_str(), "https://api.example.com/version.json");
        let _rug_ed_tests_llm_16_27_rrrruuuugggg_test_options = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_32 {
    use super::*;
    use crate::*;
    #[test]
    #[cfg(any(unix, target_os = "redox"))]
    fn test_file_url_segments_to_pathbuf() {
        let _rug_st_tests_llm_16_32_rrrruuuugggg_test_file_url_segments_to_pathbuf = 0;
        let rug_fuzz_0 = "path/to/file";
        let rug_fuzz_1 = '/';
        let result = file_url_segments_to_pathbuf(None, rug_fuzz_0.split(rug_fuzz_1));
        debug_assert_eq!(result, Ok(PathBuf::from("file:/path/to/file")));
        let _rug_ed_tests_llm_16_32_rrrruuuugggg_test_file_url_segments_to_pathbuf = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_33 {
    use super::*;
    use crate::*;
    use std::path::PathBuf;
    #[test]
    fn test_file_url_segments_to_pathbuf_windows() {
        let _rug_st_tests_llm_16_33_rrrruuuugggg_test_file_url_segments_to_pathbuf_windows = 0;
        let rug_fuzz_0 = "example.com";
        let rug_fuzz_1 = "A%3A%41\\path\\to\\file";
        let rug_fuzz_2 = '\\';
        let rug_fuzz_3 = r"\\example.com\A:\path\to\file";
        let host = Some(rug_fuzz_0);
        let segments = rug_fuzz_1.split(rug_fuzz_2);
        let result = file_url_segments_to_pathbuf_windows(host, segments);
        let expected = PathBuf::from(rug_fuzz_3);
        debug_assert_eq!(result, Ok(expected));
        let _rug_ed_tests_llm_16_33_rrrruuuugggg_test_file_url_segments_to_pathbuf_windows = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_125 {
    use super::*;
    use crate::*;
    #[test]
    fn test_path_to_file_url_segments_absolute_path() {
        let _rug_st_tests_llm_16_125_rrrruuuugggg_test_path_to_file_url_segments_absolute_path = 0;
        let rug_fuzz_0 = "/path/to/file.txt";
        let mut serialization = String::new();
        let path = Path::new(rug_fuzz_0);
        let result = path_to_file_url_segments(path, &mut serialization);
        debug_assert!(result.is_ok());
        debug_assert_eq!(serialization, "/path/to/file.txt");
        let _rug_ed_tests_llm_16_125_rrrruuuugggg_test_path_to_file_url_segments_absolute_path = 0;
    }
    #[test]
    fn test_path_to_file_url_segments_relative_path() {
        let _rug_st_tests_llm_16_125_rrrruuuugggg_test_path_to_file_url_segments_relative_path = 0;
        let rug_fuzz_0 = "path/to/file.txt";
        let mut serialization = String::new();
        let path = Path::new(rug_fuzz_0);
        let result = path_to_file_url_segments(path, &mut serialization);
        debug_assert!(result.is_err());
        debug_assert_eq!(serialization, "");
        let _rug_ed_tests_llm_16_125_rrrruuuugggg_test_path_to_file_url_segments_relative_path = 0;
    }
}
#[cfg(test)]
mod tests_rug_67 {
    use super::*;
    use std::path::Path;
    use std::string::String;
    #[test]
    fn test_path_to_file_url_segments_windows() {
        let _rug_st_tests_rug_67_rrrruuuugggg_test_path_to_file_url_segments_windows = 0;
        let rug_fuzz_0 = "/path/to/file";
        let rug_fuzz_1 = "Failed to path_to_file_url_segments_windows";
        let p0 = Path::new(rug_fuzz_0);
        let mut p1 = String::new();
        crate::path_to_file_url_segments_windows(p0, &mut p1).expect(rug_fuzz_1);
        let _rug_ed_tests_rug_67_rrrruuuugggg_test_path_to_file_url_segments_windows = 0;
    }
}
#[cfg(test)]
mod tests_rug_69 {
    use super::*;
    use crate::{ParseOptions, ParseError, Url};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_69_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https://www.example.com";
        let mut p0: ParseOptions<'static> = ParseOptions {
            base_url: None,
            encoding_override: None,
            violation_fn: None,
        };
        let p1: &str = rug_fuzz_0;
        let result: Result<Url, ParseError> = p0.parse(p1);
        debug_assert_eq!(result.is_ok(), true);
        let _rug_ed_tests_rug_69_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_70 {
    use super::*;
    #[test]
    fn test_url_parse() {
        let _rug_st_tests_rug_70_rrrruuuugggg_test_url_parse = 0;
        let rug_fuzz_0 = "https://example.net";
        let p0 = rug_fuzz_0;
        <Url>::parse(&p0).unwrap();
        let _rug_ed_tests_rug_70_rrrruuuugggg_test_url_parse = 0;
    }
}
#[cfg(test)]
mod tests_rug_71 {
    use super::*;
    use std::collections::HashSet;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_71_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https://example.net?dont=clobberme";
        let rug_fuzz_1 = "lang";
        let rug_fuzz_2 = "rust";
        let rug_fuzz_3 = "browser";
        let rug_fuzz_4 = "servo";
        let mut p0 = rug_fuzz_0;
        let mut p1 = HashSet::new();
        p1.insert((rug_fuzz_1, rug_fuzz_2));
        p1.insert((rug_fuzz_3, rug_fuzz_4));
        crate::Url::parse_with_params(&p0, &p1);
        let _rug_ed_tests_rug_71_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_72 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_join() {
        let _rug_st_tests_rug_72_rrrruuuugggg_test_join = 0;
        let rug_fuzz_0 = "https://example.net/a/b.html";
        let rug_fuzz_1 = "c.png";
        let mut p0: Url = Url::parse(rug_fuzz_0).unwrap();
        let p1: &str = rug_fuzz_1;
        p0.join(p1).unwrap();
        let _rug_ed_tests_rug_72_rrrruuuugggg_test_join = 0;
    }
}
#[cfg(test)]
mod tests_rug_73 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_73_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https://example.net/";
        let mut p0: Url = Url::parse(rug_fuzz_0).unwrap();
        <Url>::as_str(&p0);
        let _rug_ed_tests_rug_73_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_74 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_74_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https://example.net/";
        let mut p0: Url = Url::parse(rug_fuzz_0).unwrap();
        debug_assert_eq!(< Url > ::into_string(p0), "https://example.net/");
        let _rug_ed_tests_rug_74_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_75 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_75_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https://www.example.com/path?query#fragment";
        let mut p0: Url = Url::parse(rug_fuzz_0).unwrap();
        p0.check_invariants().unwrap();
        let _rug_ed_tests_rug_75_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_76 {
    use super::*;
    use crate::{Host, Origin, Url};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_76_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "ftp://example.com/foo";
        let rug_fuzz_1 = "blob:https://example.com/foo";
        let rug_fuzz_2 = "file:///tmp/foo";
        let rug_fuzz_3 = "file:///tmp/foo";
        let rug_fuzz_4 = "foo:bar";
        let mut p0 = Url::parse(rug_fuzz_0).unwrap();
        debug_assert_eq!(
            < Url > ::origin(& p0), Origin::Tuple("ftp".into(),
            Host::Domain("example.com".into()), 21)
        );
        let mut p0 = Url::parse(rug_fuzz_1).unwrap();
        debug_assert_eq!(
            < Url > ::origin(& p0), Origin::Tuple("https".into(),
            Host::Domain("example.com".into()), 443)
        );
        let mut p0 = Url::parse(rug_fuzz_2).unwrap();
        debug_assert!(! < Url > ::origin(& p0).is_tuple());
        let mut other_url = Url::parse(rug_fuzz_3).unwrap();
        debug_assert!(< Url > ::origin(& p0) != < Url > ::origin(& other_url));
        let mut p0 = Url::parse(rug_fuzz_4).unwrap();
        debug_assert!(! < Url > ::origin(& p0).is_tuple());
        let _rug_ed_tests_rug_76_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_78 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_78_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "ftp://example.com";
        let mut p0: Url = Url::parse(rug_fuzz_0).unwrap();
        debug_assert_eq!(p0.has_authority(), true);
        let _rug_ed_tests_rug_78_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_79 {
    use super::*;
    use crate::Url as Url_origin;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_79_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "data:text/plain,Stuff";
        let mut p0: Url_origin = Url_origin::parse(rug_fuzz_0).unwrap();
        <Url_origin>::cannot_be_a_base(&p0);
        let _rug_ed_tests_rug_79_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_80 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_80_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "ftp://rms@example.com";
        let mut p0 = Url::parse(rug_fuzz_0).unwrap();
        debug_assert_eq!(p0.username(), "rms");
        let _rug_ed_tests_rug_80_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_81 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_password() {
        let _rug_st_tests_rug_81_rrrruuuugggg_test_password = 0;
        let rug_fuzz_0 = "ftp://rms:secret123@example.com";
        let rug_fuzz_1 = "ftp://:secret123@example.com";
        let rug_fuzz_2 = "ftp://rms@example.com";
        let rug_fuzz_3 = "https://example.com";
        let url = Url::parse(rug_fuzz_0).unwrap();
        debug_assert_eq!(url.password(), Some("secret123"));
        let url = Url::parse(rug_fuzz_1).unwrap();
        debug_assert_eq!(url.password(), Some("secret123"));
        let url = Url::parse(rug_fuzz_2).unwrap();
        debug_assert_eq!(url.password(), None);
        let url = Url::parse(rug_fuzz_3).unwrap();
        debug_assert_eq!(url.password(), None);
        let _rug_ed_tests_rug_81_rrrruuuugggg_test_password = 0;
    }
}
#[cfg(test)]
mod tests_rug_82 {
    use super::*;
    use crate::{Url, ParseError};
    use crate::HostInternal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_82_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "ftp://rms@example.com";
        let rug_fuzz_1 = "unix:/run/foo.socket";
        let rug_fuzz_2 = "data:text/plain,Stuff";
        let mut p0 = Url::parse(rug_fuzz_0).unwrap();
        debug_assert!(Url::has_host(& p0));
        p0 = Url::parse(rug_fuzz_1).unwrap();
        debug_assert!(! Url::has_host(& p0));
        p0 = Url::parse(rug_fuzz_2).unwrap();
        debug_assert!(! Url::has_host(& p0));
        let _rug_ed_tests_rug_82_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_83 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_83_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https://127.0.0.1/index.html";
        let rug_fuzz_1 = "ftp://rms@example.com";
        let rug_fuzz_2 = "unix:/run/foo.socket";
        let rug_fuzz_3 = "data:text/plain,Stuff";
        let mut p0 = Url::parse(rug_fuzz_0).unwrap();
        debug_assert_eq!(p0.host_str(), Some("127.0.0.1"));
        let mut p0 = Url::parse(rug_fuzz_1).unwrap();
        debug_assert_eq!(p0.host_str(), Some("example.com"));
        let mut p0 = Url::parse(rug_fuzz_2).unwrap();
        debug_assert_eq!(p0.host_str(), None);
        let mut p0 = Url::parse(rug_fuzz_3).unwrap();
        debug_assert_eq!(p0.host_str(), None);
        let _rug_ed_tests_rug_83_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_84 {
    use super::*;
    use crate::Url;
    use crate::Host;
    use crate::ParseError;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_84_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https://127.0.0.1/index.html";
        let rug_fuzz_1 = "ftp://rms@example.com";
        let rug_fuzz_2 = "unix:/run/foo.socket";
        let rug_fuzz_3 = "data:text/plain,Stuff";
        let mut p0 = Url::parse(rug_fuzz_0).unwrap();
        debug_assert!(p0.host().is_some());
        let mut p1 = Url::parse(rug_fuzz_1).unwrap();
        debug_assert!(p1.host().is_some());
        let mut p2 = Url::parse(rug_fuzz_2).unwrap();
        debug_assert!(p2.host().is_none());
        let mut p3 = Url::parse(rug_fuzz_3).unwrap();
        debug_assert!(p3.host().is_none());
        let _rug_ed_tests_rug_84_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_85 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_url_domain() {
        let _rug_st_tests_rug_85_rrrruuuugggg_test_url_domain = 0;
        let rug_fuzz_0 = "https://127.0.0.1/";
        let rug_fuzz_1 = "mailto:rms@example.net";
        let rug_fuzz_2 = "https://example.com/";
        let p0: Url = Url::parse(rug_fuzz_0).unwrap();
        debug_assert_eq!(p0.domain(), None);
        let p1: Url = Url::parse(rug_fuzz_1).unwrap();
        debug_assert_eq!(p1.domain(), None);
        let p2: Url = Url::parse(rug_fuzz_2).unwrap();
        debug_assert_eq!(p2.domain(), Some("example.com"));
        let _rug_ed_tests_rug_85_rrrruuuugggg_test_url_domain = 0;
    }
}
#[cfg(test)]
mod tests_rug_86 {
    use super::*;
    use crate::Url;
    use crate::ParseError;
    #[test]
    fn test_port() -> Result<(), ParseError> {
        let mut p0 = Url::parse("https://example.com")?;
        assert_eq!(< Url >::port(& p0), None);
        let mut p0 = Url::parse("https://example.com:443/")?;
        assert_eq!(< Url >::port(& p0), None);
        let mut p0 = Url::parse("ssh://example.com:22")?;
        assert_eq!(< Url >::port(& p0), Some(22));
        Ok(())
    }
}
#[cfg(test)]
mod tests_rug_87 {
    use super::*;
    use crate::Url;
    use crate::ParseError;
    use crate::parser;
    #[test]
    fn test_rug() -> Result<(), ParseError> {
        let url = Url::parse("foo://example.com")?;
        assert_eq!(url.port_or_known_default(), None);
        let url = Url::parse("foo://example.com:1456")?;
        assert_eq!(url.port_or_known_default(), Some(1456));
        let url = Url::parse("https://example.com")?;
        assert_eq!(url.port_or_known_default(), Some(443));
        Ok(())
    }
}
#[cfg(test)]
mod tests_rug_88 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_socket_addrs() {
        let _rug_st_tests_rug_88_rrrruuuugggg_test_socket_addrs = 0;
        let rug_fuzz_0 = "https://example.net/";
        let url: Url = Url::parse(rug_fuzz_0).unwrap();
        let default_port_number = || None;
        let addrs = url.socket_addrs(default_port_number).unwrap();
        std::net::TcpStream::connect(&*addrs);
        let _rug_ed_tests_rug_88_rrrruuuugggg_test_socket_addrs = 0;
    }
}
#[cfg(test)]
mod tests_rug_89 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_89_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https://example.com/api/versions?page=2";
        let rug_fuzz_1 = "https://example.com";
        let rug_fuzz_2 = "https://example.com/countries/việt nam";
        let mut p0 = Url::parse(rug_fuzz_0).unwrap();
        debug_assert_eq!(p0.path(), "/api/versions");
        p0 = Url::parse(rug_fuzz_1).unwrap();
        debug_assert_eq!(p0.path(), "/");
        p0 = Url::parse(rug_fuzz_2).unwrap();
        debug_assert_eq!(p0.path(), "/countries/vi%E1%BB%87t%20nam");
        let _rug_ed_tests_rug_89_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_90 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_90_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https://example.com/foo/bar";
        let mut p0: Url = Url::parse(rug_fuzz_0).unwrap();
        p0.path_segments();
        let _rug_ed_tests_rug_90_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_91 {
    use super::*;
    use crate::Url;
    use crate::ParseError;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_91_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https://example.com/products?page=2";
        let rug_fuzz_1 = "https://example.com/products";
        let rug_fuzz_2 = "https://example.com/?country=espa%C3%B1ol";
        let mut p0 = Url::parse(rug_fuzz_0).unwrap();
        debug_assert_eq!(p0.query(), Some("page=2"));
        let mut p0 = Url::parse(rug_fuzz_1).unwrap();
        debug_assert!(p0.query().is_none());
        let mut p0 = Url::parse(rug_fuzz_2).unwrap();
        debug_assert_eq!(p0.query(), Some("country=espa%C3%B1ol"));
        let _rug_ed_tests_rug_91_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_92 {
    use super::*;
    use crate::Url;
    use std::borrow::Cow;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_92_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https://example.com/products?page=2&sort=desc";
        let url = Url::parse(rug_fuzz_0).unwrap();
        let mut pairs = url.query_pairs();
        debug_assert_eq!(pairs.count(), 2);
        debug_assert_eq!(
            pairs.next(), Some((Cow::Borrowed("page"), Cow::Borrowed("2")))
        );
        debug_assert_eq!(
            pairs.next(), Some((Cow::Borrowed("sort"), Cow::Borrowed("desc")))
        );
        let _rug_ed_tests_rug_92_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_93 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_93_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https://example.com/data.csv#row=4";
        let rug_fuzz_1 = "https://example.com/data.csv#cell=4,1-6,2";
        let mut p0: Url = Url::parse(rug_fuzz_0).unwrap();
        let result = p0.fragment();
        debug_assert_eq!(result, Some("row=4"));
        let mut p1: Url = Url::parse(rug_fuzz_1).unwrap();
        let result = p1.fragment();
        debug_assert_eq!(result, Some("cell=4,1-6,2"));
        let _rug_ed_tests_rug_93_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_95 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_95_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https://example.com/data.csv";
        let rug_fuzz_1 = "cell=4,1-6,2";
        let mut p0: Url = Url::try_from(rug_fuzz_0).unwrap();
        let p1: Option<&str> = Some(rug_fuzz_1);
        p0.set_fragment(p1);
        debug_assert_eq!(p0.as_str(), "https://example.com/data.csv#cell=4,1-6,2");
        debug_assert_eq!(p0.fragment(), Some("cell=4,1-6,2"));
        p0.set_fragment(None);
        debug_assert_eq!(p0.as_str(), "https://example.com/data.csv");
        debug_assert!(p0.fragment().is_none());
        let _rug_ed_tests_rug_95_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_96 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_96_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https://www.example.com/#fragment";
        let mut p0: Url = Url::parse(rug_fuzz_0).unwrap();
        p0.take_fragment();
        let _rug_ed_tests_rug_96_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_97 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_restore_already_parsed_fragment() {
        let _rug_st_tests_rug_97_rrrruuuugggg_test_restore_already_parsed_fragment = 0;
        let rug_fuzz_0 = "https://example.com/path/to/file";
        let rug_fuzz_1 = "section";
        let mut p0 = Url::parse(rug_fuzz_0).unwrap();
        let p1 = Some(rug_fuzz_1.to_string());
        p0.restore_already_parsed_fragment(p1);
        let _rug_ed_tests_rug_97_rrrruuuugggg_test_restore_already_parsed_fragment = 0;
    }
}
#[cfg(test)]
mod tests_rug_98 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_98_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https://example.com/products";
        let rug_fuzz_1 = "page=2";
        let mut p0: Url = Url::parse(rug_fuzz_0).unwrap();
        let p1: Option<&str> = Some(rug_fuzz_1);
        p0.set_query(p1);
        debug_assert_eq!(p0.as_str(), "https://example.com/products?page=2");
        debug_assert_eq!(p0.query(), Some("page=2"));
        let _rug_ed_tests_rug_98_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_99 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_query_pairs_mut() {
        let _rug_st_tests_rug_99_rrrruuuugggg_test_query_pairs_mut = 0;
        let rug_fuzz_0 = "https://example.net?lang=fr#nav";
        let rug_fuzz_1 = "foo";
        let rug_fuzz_2 = "bar";
        let rug_fuzz_3 = "foo";
        let rug_fuzz_4 = "bar & baz";
        let rug_fuzz_5 = "saisons";
        let rug_fuzz_6 = "\u{00C9}t\u{00E9}+hiver";
        let mut url = Url::parse(rug_fuzz_0).unwrap();
        url.query_pairs_mut().append_pair(rug_fuzz_1, rug_fuzz_2);
        debug_assert_eq!(url.query(), Some("lang=fr&foo=bar"));
        debug_assert_eq!(url.as_str(), "https://example.net/?lang=fr&foo=bar#nav");
        url.query_pairs_mut()
            .clear()
            .append_pair(rug_fuzz_3, rug_fuzz_4)
            .append_pair(rug_fuzz_5, rug_fuzz_6);
        debug_assert_eq!(
            url.query(), Some("foo=bar+%26+baz&saisons=%C3%89t%C3%A9%2Bhiver")
        );
        debug_assert_eq!(
            url.as_str(),
            "https://example.net/?foo=bar+%26+baz&saisons=%C3%89t%C3%A9%2Bhiver#nav"
        );
        let _rug_ed_tests_rug_99_rrrruuuugggg_test_query_pairs_mut = 0;
    }
}
#[cfg(test)]
mod tests_rug_100 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_100_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https://example.com/path?query#fragment";
        let mut p0 = Url::parse(rug_fuzz_0).unwrap();
        Url::take_after_path(&mut p0);
        let _rug_ed_tests_rug_100_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_101 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_set_path() {
        let _rug_st_tests_rug_101_rrrruuuugggg_test_set_path = 0;
        let rug_fuzz_0 = "https://example.com";
        let rug_fuzz_1 = "api/comments";
        let rug_fuzz_2 = "https://example.com/api";
        let rug_fuzz_3 = "data/report.csv";
        let mut url = Url::parse(rug_fuzz_0).unwrap();
        let path = rug_fuzz_1;
        url.set_path(path);
        debug_assert_eq!(url.as_str(), "https://example.com/api/comments");
        debug_assert_eq!(url.path(), "/api/comments");
        let mut url = Url::parse(rug_fuzz_2).unwrap();
        let path = rug_fuzz_3;
        url.set_path(path);
        debug_assert_eq!(url.as_str(), "https://example.com/data/report.csv");
        debug_assert_eq!(url.path(), "/data/report.csv");
        let _rug_ed_tests_rug_101_rrrruuuugggg_test_set_path = 0;
    }
}
#[cfg(test)]
mod tests_rug_102 {
    use super::*;
    use crate::Url;
    use crate::path_segments::PathSegmentsMut;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_102_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https://www.example.com/path/to/resource";
        let mut p0 = Url::parse(rug_fuzz_0).unwrap();
        let result = p0.path_segments_mut();
        debug_assert!(result.is_ok());
        let _rug_ed_tests_rug_102_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_103 {
    use super::*;
    #[test]
    fn test_restore_after_path() {
        let _rug_st_tests_rug_103_rrrruuuugggg_test_restore_after_path = 0;
        let rug_fuzz_0 = "https://example.com/oldpath";
        let rug_fuzz_1 = 12;
        let rug_fuzz_2 = "newpath";
        let mut url = <Url>::parse(rug_fuzz_0).unwrap();
        let old_after_path_position = rug_fuzz_1;
        let after_path = rug_fuzz_2;
        url.restore_after_path(old_after_path_position, after_path);
        let _rug_ed_tests_rug_103_rrrruuuugggg_test_restore_after_path = 0;
    }
}
#[cfg(test)]
mod tests_rug_104 {
    use super::*;
    use crate::{Url, parser};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_104_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "ssh://example.net:2048/";
        let rug_fuzz_1 = 4096;
        let rug_fuzz_2 = "ssh://example.net:2048/";
        let rug_fuzz_3 = "https://example.org/";
        let rug_fuzz_4 = 443;
        let rug_fuzz_5 = "cannot be base";
        let mut p0: Url = Url::parse(rug_fuzz_0).unwrap();
        let mut p1: Option<u16> = Some(rug_fuzz_1);
        <Url>::set_port(&mut p0, p1).unwrap();
        debug_assert_eq!(p0.as_str(), "ssh://example.net:4096/");
        let mut p0: Url = Url::parse(rug_fuzz_2).unwrap();
        let mut p1: Option<u16> = None;
        <Url>::set_port(&mut p0, p1).unwrap();
        debug_assert_eq!(p0.as_str(), "ssh://example.net/");
        let mut p0: Url = Url::parse(rug_fuzz_3).unwrap();
        let mut p1: Option<u16> = Some(rug_fuzz_4);
        <Url>::set_port(&mut p0, p1).map_err(|_| rug_fuzz_5).unwrap();
        debug_assert!(p0.port().is_none());
        let _rug_ed_tests_rug_104_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_105 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_105_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https://example.com/path";
        let rug_fuzz_1 = 8080;
        let mut p0: Url = Url::parse(rug_fuzz_0).unwrap();
        let mut p1: Option<u16> = Some(rug_fuzz_1);
        <Url>::set_port_internal(&mut p0, p1);
        let _rug_ed_tests_rug_105_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_106 {
    use super::*;
    use crate::Url;
    use crate::ParseError;
    #[test]
    fn test_set_host() {
        let _rug_st_tests_rug_106_rrrruuuugggg_test_set_host = 0;
        let rug_fuzz_0 = "https://example.net";
        let rug_fuzz_1 = "rust-lang.org";
        let rug_fuzz_2 = "foo://example.net";
        let rug_fuzz_3 = "https://example.net";
        let rug_fuzz_4 = "mailto:rms@example.net";
        let rug_fuzz_5 = "rust-lang.org";
        let mut url = Url::parse(rug_fuzz_0).unwrap();
        let result = url.set_host(Some(rug_fuzz_1));
        debug_assert!(result.is_ok());
        debug_assert_eq!(url.as_str(), "https://rust-lang.org/");
        let mut url = Url::parse(rug_fuzz_2).unwrap();
        let result = url.set_host(None);
        debug_assert!(result.is_ok());
        debug_assert_eq!(url.as_str(), "foo:/");
        let mut url = Url::parse(rug_fuzz_3).unwrap();
        let result = url.set_host(None);
        debug_assert!(result.is_err());
        debug_assert_eq!(url.as_str(), "https://example.net/");
        let mut url = Url::parse(rug_fuzz_4).unwrap();
        let result = url.set_host(Some(rug_fuzz_5));
        debug_assert!(result.is_err());
        debug_assert_eq!(url.as_str(), "mailto:rms@example.net");
        let result = url.set_host(None);
        debug_assert!(result.is_err());
        debug_assert_eq!(url.as_str(), "mailto:rms@example.net");
        let _rug_ed_tests_rug_106_rrrruuuugggg_test_set_host = 0;
    }
}
#[cfg(test)]
mod tests_rug_107 {
    use super::*;
    use crate::Url;
    use crate::Host;
    #[test]
    fn test_set_host_internal() {
        let _rug_st_tests_rug_107_rrrruuuugggg_test_set_host_internal = 0;
        let rug_fuzz_0 = "http://example.com/path?query#fragment";
        let rug_fuzz_1 = "newexample.com";
        let rug_fuzz_2 = 8080;
        let mut p0: Url = Url::parse(rug_fuzz_0).unwrap();
        let mut p1: Host<String> = Host::parse(rug_fuzz_1).unwrap();
        let mut p2: Option<Option<u16>> = Some(Some(rug_fuzz_2));
        p0.set_host_internal(p1, p2);
        debug_assert_eq!(p0.as_str(), "http://newexample.com:8080/path?query#fragment");
        let _rug_ed_tests_rug_107_rrrruuuugggg_test_set_host_internal = 0;
    }
}
#[cfg(test)]
mod tests_rug_108 {
    use super::*;
    use crate::Url;
    use std::net::IpAddr;
    #[test]
    fn test_set_ip_host() {
        let _rug_st_tests_rug_108_rrrruuuugggg_test_set_ip_host = 0;
        let rug_fuzz_0 = "http://example.com";
        let rug_fuzz_1 = "127.0.0.1";
        let mut url: Url = Url::parse(rug_fuzz_0).unwrap();
        let address: IpAddr = rug_fuzz_1.parse().unwrap();
        url.set_ip_host(address).unwrap();
        debug_assert_eq!(url.host_str(), Some("127.0.0.1"));
        debug_assert_eq!(url.as_str(), "http://127.0.0.1/");
        let _rug_ed_tests_rug_108_rrrruuuugggg_test_set_ip_host = 0;
    }
    #[test]
    fn test_set_ip_host_cannot_be_base() {
        let _rug_st_tests_rug_108_rrrruuuugggg_test_set_ip_host_cannot_be_base = 0;
        let rug_fuzz_0 = "mailto:rms@example.com";
        let rug_fuzz_1 = "127.0.0.1";
        let mut url: Url = Url::parse(rug_fuzz_0).unwrap();
        let address: IpAddr = rug_fuzz_1.parse().unwrap();
        let result = url.set_ip_host(address);
        debug_assert_eq!(url.as_str(), "mailto:rms@example.com");
        debug_assert!(result.is_err());
        let _rug_ed_tests_rug_108_rrrruuuugggg_test_set_ip_host_cannot_be_base = 0;
    }
}
#[cfg(test)]
mod tests_rug_109 {
    use super::*;
    use crate::{Url, ParseError};
    #[test]
    fn test_set_password() {
        let _rug_st_tests_rug_109_rrrruuuugggg_test_set_password = 0;
        let rug_fuzz_0 = "mailto:rmz@example.com";
        let rug_fuzz_1 = "secret_password";
        let rug_fuzz_2 = "ftp://user1:secret1@example.com";
        let rug_fuzz_3 = "secret_password";
        let rug_fuzz_4 = "ftp://user2:@example.com";
        let rug_fuzz_5 = "secret2";
        let mut url = Url::parse(rug_fuzz_0).unwrap();
        let result = url.set_password(Some(rug_fuzz_1));
        debug_assert!(result.is_err());
        let mut url = Url::parse(rug_fuzz_2).unwrap();
        let result = url.set_password(Some(rug_fuzz_3));
        debug_assert_eq!(url.password(), Some("secret_password"));
        let mut url = Url::parse(rug_fuzz_4).unwrap();
        let result = url.set_password(Some(rug_fuzz_5));
        debug_assert!(result.is_ok());
        debug_assert_eq!(url.password(), Some("secret2"));
        let _rug_ed_tests_rug_109_rrrruuuugggg_test_set_password = 0;
    }
}
#[cfg(test)]
mod tests_rug_110 {
    use super::*;
    use crate::{Url, ParseError};
    #[test]
    fn test_set_username() {
        let _rug_st_tests_rug_110_rrrruuuugggg_test_set_username = 0;
        let rug_fuzz_0 = "ftp://:secre1@example.com/";
        let rug_fuzz_1 = "user1";
        let mut url = Url::parse(rug_fuzz_0).unwrap();
        let result = url.set_username(rug_fuzz_1);
        debug_assert!(result.is_ok());
        debug_assert_eq!(url.username(), "user1");
        debug_assert_eq!(url.as_str(), "ftp://user1:secre1@example.com/");
        let _rug_ed_tests_rug_110_rrrruuuugggg_test_set_username = 0;
    }
}
#[cfg(test)]
mod tests_rug_111 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_url() {
        let _rug_st_tests_rug_111_rrrruuuugggg_test_url = 0;
        let rug_fuzz_0 = "https://example.net";
        let rug_fuzz_1 = "http";
        let rug_fuzz_2 = "foo://example.net";
        let rug_fuzz_3 = "bar";
        let rug_fuzz_4 = "https://example.net";
        let rug_fuzz_5 = "foõ";
        let rug_fuzz_6 = "mailto:rms@example.net";
        let rug_fuzz_7 = "https";
        let rug_fuzz_8 = "foo://example.net";
        let rug_fuzz_9 = "https";
        let rug_fuzz_10 = "http://example.net";
        let rug_fuzz_11 = "foo";
        let mut url = Url::parse(rug_fuzz_0).unwrap();
        url.set_scheme(rug_fuzz_1);
        debug_assert_eq!(url.as_str(), "http://example.net/");
        let mut url = Url::parse(rug_fuzz_2).unwrap();
        url.set_scheme(rug_fuzz_3);
        debug_assert_eq!(url.as_str(), "bar://example.net");
        let mut url = Url::parse(rug_fuzz_4).unwrap();
        let result = url.set_scheme(rug_fuzz_5);
        debug_assert_eq!(url.as_str(), "https://example.net/");
        debug_assert!(result.is_err());
        let mut url = Url::parse(rug_fuzz_6).unwrap();
        let result = url.set_scheme(rug_fuzz_7);
        debug_assert_eq!(url.as_str(), "mailto:rms@example.net");
        debug_assert!(result.is_err());
        let mut url = Url::parse(rug_fuzz_8).unwrap();
        let result = url.set_scheme(rug_fuzz_9);
        debug_assert_eq!(url.as_str(), "foo://example.net");
        debug_assert!(result.is_err());
        let mut url = Url::parse(rug_fuzz_10).unwrap();
        let result = url.set_scheme(rug_fuzz_11);
        debug_assert_eq!(url.as_str(), "http://example.net/");
        debug_assert!(result.is_err());
        let _rug_ed_tests_rug_111_rrrruuuugggg_test_url = 0;
    }
}
#[cfg(test)]
mod tests_rug_114 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_to_file_path() {
        let _rug_st_tests_rug_114_rrrruuuugggg_test_to_file_path = 0;
        let rug_fuzz_0 = "file:///etc/passwd";
        let url = Url::parse(rug_fuzz_0).unwrap();
        let path = url.to_file_path();
        debug_assert_eq!(path, Ok(PathBuf::from("/etc/passwd")));
        let _rug_ed_tests_rug_114_rrrruuuugggg_test_to_file_path = 0;
    }
}
#[cfg(test)]
mod tests_rug_116 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_116_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https://www.example.com/path?query#fragment";
        let rug_fuzz_1 = 10;
        let url: Url = Url::parse(rug_fuzz_0).unwrap();
        let index: u32 = rug_fuzz_1;
        url.byte_at(index);
        let _rug_ed_tests_rug_116_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_121 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_121_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https://www.example.com/";
        let mut p0: Url = Url::parse(rug_fuzz_0).unwrap();
        p0.as_ref();
        let _rug_ed_tests_rug_121_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_122 {
    use super::*;
    use crate::RangeArg;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_122_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 100;
        let rug_fuzz_1 = "hello world";
        let mut p0: std::ops::RangeTo<u32> = ..rug_fuzz_0;
        let mut p1: &str = rug_fuzz_1;
        <std::ops::RangeTo<u32> as RangeArg>::slice_of(&p0, &p1);
        let _rug_ed_tests_rug_122_rrrruuuugggg_test_rug = 0;
    }
}
