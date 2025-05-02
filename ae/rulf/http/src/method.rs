//! The HTTP request method
//!
//! This module contains HTTP-method related structs and errors and such. The
//! main type of this module, `Method`, is also reexported at the root of the
//! crate as `http::Method` and is intended for import through that location
//! primarily.
//!
//! # Examples
//!
//! ```
//! use http::Method;
//!
//! assert_eq!(Method::GET, Method::from_bytes(b"GET").unwrap());
//! assert!(Method::GET.is_idempotent());
//! assert_eq!(Method::POST.as_str(), "POST");
//! ```
use self::Inner::*;
use self::extension::{InlineExtension, AllocatedExtension};
use std::convert::AsRef;
use std::error::Error;
use std::str::FromStr;
use std::convert::TryFrom;
use std::{fmt, str};
/// The Request Method (VERB)
///
/// This type also contains constants for a number of common HTTP methods such
/// as GET, POST, etc.
///
/// Currently includes 8 variants representing the 8 methods defined in
/// [RFC 7230](https://tools.ietf.org/html/rfc7231#section-4.1), plus PATCH,
/// and an Extension variant for all extensions.
///
/// # Examples
///
/// ```
/// use http::Method;
///
/// assert_eq!(Method::GET, Method::from_bytes(b"GET").unwrap());
/// assert!(Method::GET.is_idempotent());
/// assert_eq!(Method::POST.as_str(), "POST");
/// ```
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Method(Inner);
/// A possible error value when converting `Method` from bytes.
pub struct InvalidMethod {
    _priv: (),
}
#[derive(Clone, PartialEq, Eq, Hash)]
enum Inner {
    Options,
    Get,
    Post,
    Put,
    Delete,
    Head,
    Trace,
    Connect,
    Patch,
    ExtensionInline(InlineExtension),
    ExtensionAllocated(AllocatedExtension),
}
impl Method {
    /// GET
    pub const GET: Method = Method(Get);
    /// POST
    pub const POST: Method = Method(Post);
    /// PUT
    pub const PUT: Method = Method(Put);
    /// DELETE
    pub const DELETE: Method = Method(Delete);
    /// HEAD
    pub const HEAD: Method = Method(Head);
    /// OPTIONS
    pub const OPTIONS: Method = Method(Options);
    /// CONNECT
    pub const CONNECT: Method = Method(Connect);
    /// PATCH
    pub const PATCH: Method = Method(Patch);
    /// TRACE
    pub const TRACE: Method = Method(Trace);
    /// Converts a slice of bytes to an HTTP method.
    pub fn from_bytes(src: &[u8]) -> Result<Method, InvalidMethod> {
        match src.len() {
            0 => Err(InvalidMethod::new()),
            3 => {
                match src {
                    b"GET" => Ok(Method(Get)),
                    b"PUT" => Ok(Method(Put)),
                    _ => Method::extension_inline(src),
                }
            }
            4 => {
                match src {
                    b"POST" => Ok(Method(Post)),
                    b"HEAD" => Ok(Method(Head)),
                    _ => Method::extension_inline(src),
                }
            }
            5 => {
                match src {
                    b"PATCH" => Ok(Method(Patch)),
                    b"TRACE" => Ok(Method(Trace)),
                    _ => Method::extension_inline(src),
                }
            }
            6 => {
                match src {
                    b"DELETE" => Ok(Method(Delete)),
                    _ => Method::extension_inline(src),
                }
            }
            7 => {
                match src {
                    b"OPTIONS" => Ok(Method(Options)),
                    b"CONNECT" => Ok(Method(Connect)),
                    _ => Method::extension_inline(src),
                }
            }
            _ => {
                if src.len() < InlineExtension::MAX {
                    Method::extension_inline(src)
                } else {
                    let allocated = AllocatedExtension::new(src)?;
                    Ok(Method(ExtensionAllocated(allocated)))
                }
            }
        }
    }
    fn extension_inline(src: &[u8]) -> Result<Method, InvalidMethod> {
        let inline = InlineExtension::new(src)?;
        Ok(Method(ExtensionInline(inline)))
    }
    /// Whether a method is considered "safe", meaning the request is
    /// essentially read-only.
    ///
    /// See [the spec](https://tools.ietf.org/html/rfc7231#section-4.2.1)
    /// for more words.
    pub fn is_safe(&self) -> bool {
        match self.0 {
            Get | Head | Options | Trace => true,
            _ => false,
        }
    }
    /// Whether a method is considered "idempotent", meaning the request has
    /// the same result if executed multiple times.
    ///
    /// See [the spec](https://tools.ietf.org/html/rfc7231#section-4.2.2) for
    /// more words.
    pub fn is_idempotent(&self) -> bool {
        match self.0 {
            Put | Delete => true,
            _ => self.is_safe(),
        }
    }
    /// Return a &str representation of the HTTP method
    #[inline]
    pub fn as_str(&self) -> &str {
        match self.0 {
            Options => "OPTIONS",
            Get => "GET",
            Post => "POST",
            Put => "PUT",
            Delete => "DELETE",
            Head => "HEAD",
            Trace => "TRACE",
            Connect => "CONNECT",
            Patch => "PATCH",
            ExtensionInline(ref inline) => inline.as_str(),
            ExtensionAllocated(ref allocated) => allocated.as_str(),
        }
    }
}
impl AsRef<str> for Method {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
impl<'a> PartialEq<&'a Method> for Method {
    #[inline]
    fn eq(&self, other: &&'a Method) -> bool {
        self == *other
    }
}
impl<'a> PartialEq<Method> for &'a Method {
    #[inline]
    fn eq(&self, other: &Method) -> bool {
        *self == other
    }
}
impl PartialEq<str> for Method {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.as_ref() == other
    }
}
impl PartialEq<Method> for str {
    #[inline]
    fn eq(&self, other: &Method) -> bool {
        self == other.as_ref()
    }
}
impl<'a> PartialEq<&'a str> for Method {
    #[inline]
    fn eq(&self, other: &&'a str) -> bool {
        self.as_ref() == *other
    }
}
impl<'a> PartialEq<Method> for &'a str {
    #[inline]
    fn eq(&self, other: &Method) -> bool {
        *self == other.as_ref()
    }
}
impl fmt::Debug for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_ref())
    }
}
impl fmt::Display for Method {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str(self.as_ref())
    }
}
impl Default for Method {
    #[inline]
    fn default() -> Method {
        Method::GET
    }
}
impl<'a> From<&'a Method> for Method {
    #[inline]
    fn from(t: &'a Method) -> Self {
        t.clone()
    }
}
impl<'a> TryFrom<&'a [u8]> for Method {
    type Error = InvalidMethod;
    #[inline]
    fn try_from(t: &'a [u8]) -> Result<Self, Self::Error> {
        Method::from_bytes(t)
    }
}
impl<'a> TryFrom<&'a str> for Method {
    type Error = InvalidMethod;
    #[inline]
    fn try_from(t: &'a str) -> Result<Self, Self::Error> {
        TryFrom::try_from(t.as_bytes())
    }
}
impl FromStr for Method {
    type Err = InvalidMethod;
    #[inline]
    fn from_str(t: &str) -> Result<Self, Self::Err> {
        TryFrom::try_from(t)
    }
}
impl InvalidMethod {
    fn new() -> InvalidMethod {
        InvalidMethod { _priv: () }
    }
}
impl fmt::Debug for InvalidMethod {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("InvalidMethod").finish()
    }
}
impl fmt::Display for InvalidMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid HTTP method")
    }
}
impl Error for InvalidMethod {}
mod extension {
    use super::InvalidMethod;
    use std::str;
    #[derive(Clone, PartialEq, Eq, Hash)]
    pub struct InlineExtension([u8; InlineExtension::MAX], u8);
    #[derive(Clone, PartialEq, Eq, Hash)]
    pub struct AllocatedExtension(Box<[u8]>);
    impl InlineExtension {
        pub const MAX: usize = 15;
        pub fn new(src: &[u8]) -> Result<InlineExtension, InvalidMethod> {
            let mut data: [u8; InlineExtension::MAX] = Default::default();
            write_checked(src, &mut data)?;
            Ok(InlineExtension(data, src.len() as u8))
        }
        pub fn as_str(&self) -> &str {
            let InlineExtension(ref data, len) = self;
            unsafe { str::from_utf8_unchecked(&data[..*len as usize]) }
        }
    }
    impl AllocatedExtension {
        pub fn new(src: &[u8]) -> Result<AllocatedExtension, InvalidMethod> {
            let mut data: Vec<u8> = vec![0; src.len()];
            write_checked(src, &mut data)?;
            Ok(AllocatedExtension(data.into_boxed_slice()))
        }
        pub fn as_str(&self) -> &str {
            unsafe { str::from_utf8_unchecked(&self.0) }
        }
    }
    const METHOD_CHARS: [u8; 256] = [
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'!',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'*',
        b'+',
        b'\0',
        b'-',
        b'.',
        b'\0',
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
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
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
        b'\0',
        b'\0',
        b'\0',
        b'^',
        b'_',
        b'`',
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
        b'\0',
        b'|',
        b'\0',
        b'~',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
        b'\0',
    ];
    fn write_checked(src: &[u8], dst: &mut [u8]) -> Result<(), InvalidMethod> {
        for (i, &b) in src.iter().enumerate() {
            let b = METHOD_CHARS[b as usize];
            if b == 0 {
                return Err(InvalidMethod::new());
            }
            dst[i] = b;
        }
        Ok(())
    }
}
#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_method_eq() {
        assert_eq!(Method::GET, Method::GET);
        assert_eq!(Method::GET, "GET");
        assert_eq!(& Method::GET, "GET");
        assert_eq!("GET", Method::GET);
        assert_eq!("GET", & Method::GET);
        assert_eq!(& Method::GET, Method::GET);
        assert_eq!(Method::GET, & Method::GET);
    }
    #[test]
    fn test_invalid_method() {
        assert!(Method::from_str("").is_err());
        assert!(Method::from_bytes(b"").is_err());
        assert!(Method::from_bytes(& [0xC0]).is_err());
        assert!(Method::from_bytes(& [0x10]).is_err());
    }
    #[test]
    fn test_is_idempotent() {
        assert!(Method::OPTIONS.is_idempotent());
        assert!(Method::GET.is_idempotent());
        assert!(Method::PUT.is_idempotent());
        assert!(Method::DELETE.is_idempotent());
        assert!(Method::HEAD.is_idempotent());
        assert!(Method::TRACE.is_idempotent());
        assert!(! Method::POST.is_idempotent());
        assert!(! Method::CONNECT.is_idempotent());
        assert!(! Method::PATCH.is_idempotent());
    }
    #[test]
    fn test_extention_method() {
        assert_eq!(Method::from_str("WOW").unwrap(), "WOW");
        assert_eq!(Method::from_str("wOw!!").unwrap(), "wOw!!");
        let long_method = "This_is_a_very_long_method.It_is_valid_but_unlikely.";
        assert_eq!(Method::from_str(& long_method).unwrap(), long_method);
    }
}
#[cfg(test)]
mod tests_llm_16_12 {
    use super::*;
    use crate::*;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_12_rrrruuuugggg_test_eq = 0;
        let method1 = Method::GET;
        let method2 = Method::GET;
        let method3 = Method::POST;
        debug_assert_eq!(method1.eq(& method2), true);
        debug_assert_eq!(method1.eq(& method3), false);
        let _rug_ed_tests_llm_16_12_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_200 {
    use super::*;
    use crate::*;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_200_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "GET";
        let rug_fuzz_1 = "POST";
        let method = Method::GET;
        debug_assert_eq!(method.eq(& rug_fuzz_0), true);
        debug_assert_eq!(method.eq(& rug_fuzz_1), false);
        let _rug_ed_tests_llm_16_200_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_201 {
    use super::*;
    use crate::*;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_201_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "GET";
        let rug_fuzz_1 = "POST";
        let rug_fuzz_2 = "";
        let method = Method::GET;
        debug_assert_eq!(method.eq(rug_fuzz_0), true);
        debug_assert_eq!(method.eq(rug_fuzz_1), false);
        debug_assert_eq!(method.eq(rug_fuzz_2), false);
        let _rug_ed_tests_llm_16_201_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_202 {
    use super::*;
    use crate::*;
    use std::convert::TryFrom;
    #[test]
    fn test_as_ref() {
        let _rug_st_tests_llm_16_202_rrrruuuugggg_test_as_ref = 0;
        let rug_fuzz_0 = b"POST";
        let rug_fuzz_1 = b"HEAD";
        let method = Method::GET;
        debug_assert_eq!(method.as_ref(), "GET");
        let method = Method::PATCH;
        debug_assert_eq!(method.as_ref(), "PATCH");
        let method = Method::from_bytes(rug_fuzz_0).unwrap();
        debug_assert_eq!(method.as_ref(), "POST");
        let method = Method::from_bytes(rug_fuzz_1).unwrap();
        debug_assert_eq!(method.as_ref(), "HEAD");
        let _rug_ed_tests_llm_16_202_rrrruuuugggg_test_as_ref = 0;
    }
    #[test]
    fn test_from_bytes() {
        let _rug_st_tests_llm_16_202_rrrruuuugggg_test_from_bytes = 0;
        let rug_fuzz_0 = b"GET";
        let rug_fuzz_1 = b"POST";
        let rug_fuzz_2 = b"PUT";
        let rug_fuzz_3 = b"DELETE";
        let rug_fuzz_4 = b"HEAD";
        let rug_fuzz_5 = b"OPTIONS";
        let rug_fuzz_6 = b"CONNECT";
        let rug_fuzz_7 = b"PATCH";
        let rug_fuzz_8 = b"TRACE";
        let rug_fuzz_9 = b"CUSTOM";
        let method = Method::from_bytes(rug_fuzz_0).unwrap();
        debug_assert_eq!(method, Method::GET);
        let method = Method::from_bytes(rug_fuzz_1).unwrap();
        debug_assert_eq!(method, Method::POST);
        let method = Method::from_bytes(rug_fuzz_2).unwrap();
        debug_assert_eq!(method, Method::PUT);
        let method = Method::from_bytes(rug_fuzz_3).unwrap();
        debug_assert_eq!(method, Method::DELETE);
        let method = Method::from_bytes(rug_fuzz_4).unwrap();
        debug_assert_eq!(method, Method::HEAD);
        let method = Method::from_bytes(rug_fuzz_5).unwrap();
        debug_assert_eq!(method, Method::OPTIONS);
        let method = Method::from_bytes(rug_fuzz_6).unwrap();
        debug_assert_eq!(method, Method::CONNECT);
        let method = Method::from_bytes(rug_fuzz_7).unwrap();
        debug_assert_eq!(method, Method::PATCH);
        let method = Method::from_bytes(rug_fuzz_8).unwrap();
        debug_assert_eq!(method, Method::TRACE);
        let method = Method::from_bytes(rug_fuzz_9).unwrap();
        debug_assert_eq!(method.as_ref(), "CUSTOM");
        let _rug_ed_tests_llm_16_202_rrrruuuugggg_test_from_bytes = 0;
    }
    #[test]
    fn test_is_safe() {
        let _rug_st_tests_llm_16_202_rrrruuuugggg_test_is_safe = 0;
        let method = Method::GET;
        debug_assert!(method.is_safe());
        let method = Method::POST;
        debug_assert!(! method.is_safe());
        let _rug_ed_tests_llm_16_202_rrrruuuugggg_test_is_safe = 0;
    }
    #[test]
    fn test_is_idempotent() {
        let _rug_st_tests_llm_16_202_rrrruuuugggg_test_is_idempotent = 0;
        let method = Method::GET;
        debug_assert!(method.is_idempotent());
        let method = Method::POST;
        debug_assert!(! method.is_idempotent());
        let method = Method::PUT;
        debug_assert!(method.is_idempotent());
        let method = Method::DELETE;
        debug_assert!(method.is_idempotent());
        let _rug_ed_tests_llm_16_202_rrrruuuugggg_test_is_idempotent = 0;
    }
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_202_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "GET";
        let rug_fuzz_1 = "POST";
        let method = Method::GET;
        debug_assert_eq!(method.eq(rug_fuzz_0), true);
        debug_assert_eq!(method.eq(rug_fuzz_1), false);
        debug_assert_eq!(method.eq(& Method::GET), true);
        let _rug_ed_tests_llm_16_202_rrrruuuugggg_test_eq = 0;
    }
    #[test]
    fn test_try_from() {
        let _rug_st_tests_llm_16_202_rrrruuuugggg_test_try_from = 0;
        let rug_fuzz_0 = "GET";
        let rug_fuzz_1 = "POST";
        let rug_fuzz_2 = "PUT";
        let rug_fuzz_3 = "DELETE";
        let rug_fuzz_4 = "HEAD";
        let rug_fuzz_5 = "OPTIONS";
        let rug_fuzz_6 = "CONNECT";
        let rug_fuzz_7 = "PATCH";
        let rug_fuzz_8 = "TRACE";
        let rug_fuzz_9 = "CUSTOM";
        let method = Method::try_from(rug_fuzz_0).unwrap();
        debug_assert_eq!(method, Method::GET);
        let method = Method::try_from(rug_fuzz_1).unwrap();
        debug_assert_eq!(method, Method::POST);
        let method = Method::try_from(rug_fuzz_2).unwrap();
        debug_assert_eq!(method, Method::PUT);
        let method = Method::try_from(rug_fuzz_3).unwrap();
        debug_assert_eq!(method, Method::DELETE);
        let method = Method::try_from(rug_fuzz_4).unwrap();
        debug_assert_eq!(method, Method::HEAD);
        let method = Method::try_from(rug_fuzz_5).unwrap();
        debug_assert_eq!(method, Method::OPTIONS);
        let method = Method::try_from(rug_fuzz_6).unwrap();
        debug_assert_eq!(method, Method::CONNECT);
        let method = Method::try_from(rug_fuzz_7).unwrap();
        debug_assert_eq!(method, Method::PATCH);
        let method = Method::try_from(rug_fuzz_8).unwrap();
        debug_assert_eq!(method, Method::TRACE);
        let method = Method::try_from(rug_fuzz_9).unwrap();
        debug_assert_eq!(method.as_ref(), "CUSTOM");
        let _rug_ed_tests_llm_16_202_rrrruuuugggg_test_try_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_209 {
    use super::*;
    use crate::*;
    use crate::method::Method;
    #[test]
    fn test_default() {
        let _rug_st_tests_llm_16_209_rrrruuuugggg_test_default = 0;
        let default_method: Method = <Method as std::default::Default>::default();
        debug_assert_eq!(default_method, Method::GET);
        let _rug_ed_tests_llm_16_209_rrrruuuugggg_test_default = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_458 {
    use super::*;
    use crate::*;
    use std::convert::TryFrom;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_458_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "GET";
        let rug_fuzz_1 = "POST";
        let rug_fuzz_2 = b"EXTENSION";
        let rug_fuzz_3 = b"EXTENSIONEXTENSIONEXTENSION";
        let method_get = Method::GET;
        let method_post = Method::POST;
        debug_assert_eq!(method_get.eq(& method_get), true);
        debug_assert_eq!(method_get.eq(& method_post), false);
        debug_assert_eq!(method_post.eq(& method_post), true);
        debug_assert_eq!(method_post.eq(& method_get), false);
        let method_get_str = Method::from_str(rug_fuzz_0).unwrap();
        let method_post_str = Method::from_str(rug_fuzz_1).unwrap();
        debug_assert_eq!(method_get.eq(& method_get_str), true);
        debug_assert_eq!(method_get_str.eq(& method_get), true);
        debug_assert_eq!(method_post.eq(& method_post_str), true);
        debug_assert_eq!(method_post_str.eq(& method_post), true);
        debug_assert_eq!(method_get_str.eq(& method_post_str), false);
        debug_assert_eq!(method_post_str.eq(& method_get_str), false);
        let method_extension_inline = Method::from_bytes(rug_fuzz_2).unwrap();
        let method_extension_allocated = Method::from_bytes(rug_fuzz_3).unwrap();
        debug_assert_eq!(method_extension_inline.eq(& method_extension_inline), true);
        debug_assert_eq!(
            method_extension_inline.eq(& method_extension_allocated), false
        );
        debug_assert_eq!(
            method_extension_allocated.eq(& method_extension_allocated), true
        );
        debug_assert_eq!(
            method_extension_allocated.eq(& method_extension_inline), false
        );
        let _rug_ed_tests_llm_16_458_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_459 {
    use super::*;
    use crate::*;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_459_rrrruuuugggg_test_eq = 0;
        let method = Method::GET;
        let other = Method::GET;
        debug_assert_eq!(method.eq(& other), true);
        let _rug_ed_tests_llm_16_459_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_462 {
    use super::*;
    use crate::*;
    #[test]
    fn test_as_str() {
        let _rug_st_tests_llm_16_462_rrrruuuugggg_test_as_str = 0;
        let method = Method::GET;
        debug_assert_eq!(method.as_str(), "GET");
        let _rug_ed_tests_llm_16_462_rrrruuuugggg_test_as_str = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_464_llm_16_463 {
    use super::*;
    use crate::*;
    use crate::method::Method;
    use crate::method::InvalidMethod;
    use crate::method::InlineExtension;
    use crate::method::ExtensionInline;
    #[test]
    fn test_extension_inline_valid() {
        let _rug_st_tests_llm_16_464_llm_16_463_rrrruuuugggg_test_extension_inline_valid = 0;
        let src: &[u8] = &[];
        let result = Method::extension_inline(src);
        debug_assert!(result.is_ok());
        let method = result.unwrap();
        debug_assert_eq!(
            method, Method(ExtensionInline(InlineExtension::new(src).unwrap()))
        );
        let _rug_ed_tests_llm_16_464_llm_16_463_rrrruuuugggg_test_extension_inline_valid = 0;
    }
    #[test]
    fn test_extension_inline_invalid() {
        let _rug_st_tests_llm_16_464_llm_16_463_rrrruuuugggg_test_extension_inline_invalid = 0;
        let src: &[u8] = &[];
        let result = Method::extension_inline(src);
        debug_assert!(result.is_err());
        let _rug_ed_tests_llm_16_464_llm_16_463_rrrruuuugggg_test_extension_inline_invalid = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_468 {
    use crate::Method;
    #[test]
    fn test_is_idempotent() {
        let _rug_st_tests_llm_16_468_rrrruuuugggg_test_is_idempotent = 0;
        let get = Method::GET;
        let post = Method::POST;
        let put = Method::PUT;
        let delete = Method::DELETE;
        debug_assert_eq!(get.is_idempotent(), true);
        debug_assert_eq!(post.is_idempotent(), false);
        debug_assert_eq!(put.is_idempotent(), true);
        debug_assert_eq!(delete.is_idempotent(), true);
        let _rug_ed_tests_llm_16_468_rrrruuuugggg_test_is_idempotent = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_470 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_safe() {
        let _rug_st_tests_llm_16_470_rrrruuuugggg_test_is_safe = 0;
        let get = Method(Inner::Get);
        debug_assert_eq!(get.is_safe(), true);
        let head = Method(Inner::Head);
        debug_assert_eq!(head.is_safe(), true);
        let options = Method(Inner::Options);
        debug_assert_eq!(options.is_safe(), true);
        let trace = Method(Inner::Trace);
        debug_assert_eq!(trace.is_safe(), true);
        let post = Method(Inner::Post);
        debug_assert_eq!(post.is_safe(), false);
        let put = Method(Inner::Put);
        debug_assert_eq!(put.is_safe(), false);
        let delete = Method(Inner::Delete);
        debug_assert_eq!(delete.is_safe(), false);
        let connect = Method(Inner::Connect);
        debug_assert_eq!(connect.is_safe(), false);
        let patch = Method(Inner::Patch);
        debug_assert_eq!(patch.is_safe(), false);
        let _rug_ed_tests_llm_16_470_rrrruuuugggg_test_is_safe = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_471 {
    use super::*;
    use crate::*;
    #[test]
    fn test_as_str() {
        let _rug_st_tests_llm_16_471_rrrruuuugggg_test_as_str = 0;
        let rug_fuzz_0 = b"test_extension";
        let extension = AllocatedExtension::new(rug_fuzz_0).unwrap();
        debug_assert_eq!(extension.as_str(), "test_extension");
        let _rug_ed_tests_llm_16_471_rrrruuuugggg_test_as_str = 0;
    }
}
#[cfg(test)]
mod tests_rug_129 {
    use super::*;
    use crate::method::{Method, InvalidMethod};
    #[test]
    fn test_from_bytes() {
        let _rug_st_tests_rug_129_rrrruuuugggg_test_from_bytes = 0;
        let rug_fuzz_0 = b"GET";
        let mut p0: &[u8] = rug_fuzz_0;
        Method::from_bytes(p0);
        let _rug_ed_tests_rug_129_rrrruuuugggg_test_from_bytes = 0;
    }
}
#[cfg(test)]
mod tests_rug_131 {
    use super::*;
    use crate::Method;
    #[test]
    fn test_from() {
        let _rug_st_tests_rug_131_rrrruuuugggg_test_from = 0;
        let mut p0: Method = Method::default();
        <Method as std::convert::From<&Method>>::from(&p0);
        let _rug_ed_tests_rug_131_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_rug_133 {
    use super::*;
    use std::convert::TryFrom;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_133_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "GET";
        let mut p0: &str = rug_fuzz_0;
        <Method as std::convert::TryFrom<&str>>::try_from(&p0);
        let _rug_ed_tests_rug_133_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_134 {
    use super::*;
    use crate::method::Method;
    use std::str::FromStr;
    #[test]
    fn test_from_str() {
        let _rug_st_tests_rug_134_rrrruuuugggg_test_from_str = 0;
        let rug_fuzz_0 = "GET";
        let p0: &str = rug_fuzz_0;
        Method::from_str(&p0);
        let _rug_ed_tests_rug_134_rrrruuuugggg_test_from_str = 0;
    }
}
#[cfg(test)]
mod tests_rug_135 {
    use super::*;
    use crate::method::InvalidMethod;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_135_rrrruuuugggg_test_rug = 0;
        InvalidMethod::new();
        let _rug_ed_tests_rug_135_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_136 {
    use super::*;
    use crate::method::extension::InlineExtension;
    #[test]
    fn test_inline_extension_new() {
        let _rug_st_tests_rug_136_rrrruuuugggg_test_inline_extension_new = 0;
        let rug_fuzz_0 = b"GET";
        let p0: &[u8] = rug_fuzz_0;
        InlineExtension::new(p0).unwrap();
        let _rug_ed_tests_rug_136_rrrruuuugggg_test_inline_extension_new = 0;
    }
}
#[cfg(test)]
mod tests_rug_137 {
    use super::*;
    use crate::method::extension::InlineExtension;
    #[test]
    fn test_as_str() {
        let _rug_st_tests_rug_137_rrrruuuugggg_test_as_str = 0;
        let rug_fuzz_0 = b"sample";
        let mut v93 = InlineExtension::new(rug_fuzz_0).unwrap();
        let p0: &InlineExtension = &v93;
        crate::method::extension::InlineExtension::as_str(p0);
        let _rug_ed_tests_rug_137_rrrruuuugggg_test_as_str = 0;
    }
}
#[cfg(test)]
mod tests_rug_138 {
    use super::*;
    use crate::method::extension::AllocatedExtension;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_138_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"sample_data";
        let p0: &[u8] = rug_fuzz_0;
        AllocatedExtension::new(p0).unwrap();
        let _rug_ed_tests_rug_138_rrrruuuugggg_test_rug = 0;
    }
}
