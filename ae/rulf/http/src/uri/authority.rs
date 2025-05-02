use std::convert::TryFrom;
use std::hash::{Hash, Hasher};
use std::str::FromStr;
use std::{cmp, fmt, str};
use bytes::Bytes;
use super::{ErrorKind, InvalidUri, Port, URI_CHARS};
use crate::byte_str::ByteStr;
/// Represents the authority component of a URI.
#[derive(Clone)]
pub struct Authority {
    pub(super) data: ByteStr,
}
impl Authority {
    pub(super) fn empty() -> Self {
        Authority { data: ByteStr::new() }
    }
    pub(super) fn from_shared(s: Bytes) -> Result<Self, InvalidUri> {
        create_authority(s, |s| s)
    }
    /// Attempt to convert an `Authority` from a static string.
    ///
    /// This function will not perform any copying, and the string will be
    /// checked if it is empty or contains an invalid character.
    ///
    /// # Panics
    ///
    /// This function panics if the argument contains invalid characters or
    /// is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::uri::Authority;
    /// let authority = Authority::from_static("example.com");
    /// assert_eq!(authority.host(), "example.com");
    /// ```
    pub fn from_static(src: &'static str) -> Self {
        Authority::from_shared(Bytes::from_static(src.as_bytes()))
            .expect("static str is not valid authority")
    }
    /// Attempt to convert a `Bytes` buffer to a `Authority`.
    ///
    /// This will try to prevent a copy if the type passed is the type used
    /// internally, and will copy the data if it is not.
    pub fn from_maybe_shared<T>(src: T) -> Result<Self, InvalidUri>
    where
        T: AsRef<[u8]> + 'static,
    {
        if_downcast_into!(T, Bytes, src, { return Authority::from_shared(src); });
        Authority::try_from(src.as_ref())
    }
    pub(super) fn parse(s: &[u8]) -> Result<usize, InvalidUri> {
        let mut colon_cnt = 0;
        let mut start_bracket = false;
        let mut end_bracket = false;
        let mut has_percent = false;
        let mut end = s.len();
        let mut at_sign_pos = None;
        for (i, &b) in s.iter().enumerate() {
            match URI_CHARS[b as usize] {
                b'/' | b'?' | b'#' => {
                    end = i;
                    break;
                }
                b':' => {
                    colon_cnt += 1;
                }
                b'[' => {
                    if has_percent || start_bracket {
                        return Err(ErrorKind::InvalidAuthority.into());
                    }
                    start_bracket = true;
                }
                b']' => {
                    if end_bracket {
                        return Err(ErrorKind::InvalidAuthority.into());
                    }
                    end_bracket = true;
                    colon_cnt = 0;
                    has_percent = false;
                }
                b'@' => {
                    at_sign_pos = Some(i);
                    colon_cnt = 0;
                    has_percent = false;
                }
                0 if b == b'%' => {
                    has_percent = true;
                }
                0 => {
                    return Err(ErrorKind::InvalidUriChar.into());
                }
                _ => {}
            }
        }
        if start_bracket ^ end_bracket {
            return Err(ErrorKind::InvalidAuthority.into());
        }
        if colon_cnt > 1 {
            return Err(ErrorKind::InvalidAuthority.into());
        }
        if end > 0 && at_sign_pos == Some(end - 1) {
            return Err(ErrorKind::InvalidAuthority.into());
        }
        if has_percent {
            return Err(ErrorKind::InvalidAuthority.into());
        }
        Ok(end)
    }
    fn parse_non_empty(s: &[u8]) -> Result<usize, InvalidUri> {
        if s.is_empty() {
            return Err(ErrorKind::Empty.into());
        }
        Authority::parse(s)
    }
    /// Get the host of this `Authority`.
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
    /// ```
    /// # use http::uri::*;
    /// let authority: Authority = "example.org:80".parse().unwrap();
    ///
    /// assert_eq!(authority.host(), "example.org");
    /// ```
    #[inline]
    pub fn host(&self) -> &str {
        host(self.as_str())
    }
    /// Get the port part of this `Authority`.
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
    /// Authority with port
    ///
    /// ```
    /// # use http::uri::Authority;
    /// let authority: Authority = "example.org:80".parse().unwrap();
    ///
    /// let port = authority.port().unwrap();
    /// assert_eq!(port.as_u16(), 80);
    /// assert_eq!(port.as_str(), "80");
    /// ```
    ///
    /// Authority without port
    ///
    /// ```
    /// # use http::uri::Authority;
    /// let authority: Authority = "example.org".parse().unwrap();
    ///
    /// assert!(authority.port().is_none());
    /// ```
    pub fn port(&self) -> Option<Port<&str>> {
        let bytes = self.as_str();
        bytes.rfind(":").and_then(|i| Port::from_str(&bytes[i + 1..]).ok())
    }
    /// Get the port of this `Authority` as a `u16`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::uri::Authority;
    /// let authority: Authority = "example.org:80".parse().unwrap();
    ///
    /// assert_eq!(authority.port_u16(), Some(80));
    /// ```
    pub fn port_u16(&self) -> Option<u16> {
        self.port().and_then(|p| Some(p.as_u16()))
    }
    /// Return a str representation of the authority
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.data[..]
    }
}
impl AsRef<str> for Authority {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
impl PartialEq for Authority {
    fn eq(&self, other: &Authority) -> bool {
        self.data.eq_ignore_ascii_case(&other.data)
    }
}
impl Eq for Authority {}
/// Case-insensitive equality
///
/// # Examples
///
/// ```
/// # use http::uri::Authority;
/// let authority: Authority = "HELLO.com".parse().unwrap();
/// assert_eq!(authority, "hello.coM");
/// assert_eq!("hello.com", authority);
/// ```
impl PartialEq<str> for Authority {
    fn eq(&self, other: &str) -> bool {
        self.data.eq_ignore_ascii_case(other)
    }
}
impl PartialEq<Authority> for str {
    fn eq(&self, other: &Authority) -> bool {
        self.eq_ignore_ascii_case(other.as_str())
    }
}
impl<'a> PartialEq<Authority> for &'a str {
    fn eq(&self, other: &Authority) -> bool {
        self.eq_ignore_ascii_case(other.as_str())
    }
}
impl<'a> PartialEq<&'a str> for Authority {
    fn eq(&self, other: &&'a str) -> bool {
        self.data.eq_ignore_ascii_case(other)
    }
}
impl PartialEq<String> for Authority {
    fn eq(&self, other: &String) -> bool {
        self.data.eq_ignore_ascii_case(other.as_str())
    }
}
impl PartialEq<Authority> for String {
    fn eq(&self, other: &Authority) -> bool {
        self.as_str().eq_ignore_ascii_case(other.as_str())
    }
}
/// Case-insensitive ordering
///
/// # Examples
///
/// ```
/// # use http::uri::Authority;
/// let authority: Authority = "DEF.com".parse().unwrap();
/// assert!(authority < "ghi.com");
/// assert!(authority > "abc.com");
/// ```
impl PartialOrd for Authority {
    fn partial_cmp(&self, other: &Authority) -> Option<cmp::Ordering> {
        let left = self.data.as_bytes().iter().map(|b| b.to_ascii_lowercase());
        let right = other.data.as_bytes().iter().map(|b| b.to_ascii_lowercase());
        left.partial_cmp(right)
    }
}
impl PartialOrd<str> for Authority {
    fn partial_cmp(&self, other: &str) -> Option<cmp::Ordering> {
        let left = self.data.as_bytes().iter().map(|b| b.to_ascii_lowercase());
        let right = other.as_bytes().iter().map(|b| b.to_ascii_lowercase());
        left.partial_cmp(right)
    }
}
impl PartialOrd<Authority> for str {
    fn partial_cmp(&self, other: &Authority) -> Option<cmp::Ordering> {
        let left = self.as_bytes().iter().map(|b| b.to_ascii_lowercase());
        let right = other.data.as_bytes().iter().map(|b| b.to_ascii_lowercase());
        left.partial_cmp(right)
    }
}
impl<'a> PartialOrd<Authority> for &'a str {
    fn partial_cmp(&self, other: &Authority) -> Option<cmp::Ordering> {
        let left = self.as_bytes().iter().map(|b| b.to_ascii_lowercase());
        let right = other.data.as_bytes().iter().map(|b| b.to_ascii_lowercase());
        left.partial_cmp(right)
    }
}
impl<'a> PartialOrd<&'a str> for Authority {
    fn partial_cmp(&self, other: &&'a str) -> Option<cmp::Ordering> {
        let left = self.data.as_bytes().iter().map(|b| b.to_ascii_lowercase());
        let right = other.as_bytes().iter().map(|b| b.to_ascii_lowercase());
        left.partial_cmp(right)
    }
}
impl PartialOrd<String> for Authority {
    fn partial_cmp(&self, other: &String) -> Option<cmp::Ordering> {
        let left = self.data.as_bytes().iter().map(|b| b.to_ascii_lowercase());
        let right = other.as_bytes().iter().map(|b| b.to_ascii_lowercase());
        left.partial_cmp(right)
    }
}
impl PartialOrd<Authority> for String {
    fn partial_cmp(&self, other: &Authority) -> Option<cmp::Ordering> {
        let left = self.as_bytes().iter().map(|b| b.to_ascii_lowercase());
        let right = other.data.as_bytes().iter().map(|b| b.to_ascii_lowercase());
        left.partial_cmp(right)
    }
}
/// Case-insensitive hashing
///
/// # Examples
///
/// ```
/// # use http::uri::Authority;
/// # use std::hash::{Hash, Hasher};
/// # use std::collections::hash_map::DefaultHasher;
///
/// let a: Authority = "HELLO.com".parse().unwrap();
/// let b: Authority = "hello.coM".parse().unwrap();
///
/// let mut s = DefaultHasher::new();
/// a.hash(&mut s);
/// let a = s.finish();
///
/// let mut s = DefaultHasher::new();
/// b.hash(&mut s);
/// let b = s.finish();
///
/// assert_eq!(a, b);
/// ```
impl Hash for Authority {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.data.len().hash(state);
        for &b in self.data.as_bytes() {
            state.write_u8(b.to_ascii_lowercase());
        }
    }
}
impl<'a> TryFrom<&'a [u8]> for Authority {
    type Error = InvalidUri;
    #[inline]
    fn try_from(s: &'a [u8]) -> Result<Self, Self::Error> {
        create_authority(s, |s| Bytes::copy_from_slice(s))
    }
}
impl<'a> TryFrom<&'a str> for Authority {
    type Error = InvalidUri;
    #[inline]
    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        TryFrom::try_from(s.as_bytes())
    }
}
impl FromStr for Authority {
    type Err = InvalidUri;
    fn from_str(s: &str) -> Result<Self, InvalidUri> {
        TryFrom::try_from(s)
    }
}
impl fmt::Debug for Authority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
impl fmt::Display for Authority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
fn host(auth: &str) -> &str {
    let host_port = auth
        .rsplitn(2, '@')
        .next()
        .expect("split always has at least 1 item");
    if host_port.as_bytes()[0] == b'[' {
        let i = host_port.find(']').expect("parsing should validate brackets");
        &host_port[0..i + 1]
    } else {
        host_port.split(':').next().expect("split always has at least 1 item")
    }
}
fn create_authority<B, F>(b: B, f: F) -> Result<Authority, InvalidUri>
where
    B: AsRef<[u8]>,
    F: FnOnce(B) -> Bytes,
{
    let s = b.as_ref();
    let authority_end = Authority::parse_non_empty(s)?;
    if authority_end != s.len() {
        return Err(ErrorKind::InvalidUriChar.into());
    }
    let bytes = f(b);
    Ok(Authority {
        data: unsafe { ByteStr::from_utf8_unchecked(bytes) },
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_empty_string_is_error() {
        let err = Authority::parse_non_empty(b"").unwrap_err();
        assert_eq!(err.0, ErrorKind::Empty);
    }
    #[test]
    fn equal_to_self_of_same_authority() {
        let authority1: Authority = "example.com".parse().unwrap();
        let authority2: Authority = "EXAMPLE.COM".parse().unwrap();
        assert_eq!(authority1, authority2);
        assert_eq!(authority2, authority1);
    }
    #[test]
    fn not_equal_to_self_of_different_authority() {
        let authority1: Authority = "example.com".parse().unwrap();
        let authority2: Authority = "test.com".parse().unwrap();
        assert_ne!(authority1, authority2);
        assert_ne!(authority2, authority1);
    }
    #[test]
    fn equates_with_a_str() {
        let authority: Authority = "example.com".parse().unwrap();
        assert_eq!(& authority, "EXAMPLE.com");
        assert_eq!("EXAMPLE.com", & authority);
        assert_eq!(authority, "EXAMPLE.com");
        assert_eq!("EXAMPLE.com", authority);
    }
    #[test]
    fn from_static_equates_with_a_str() {
        let authority = Authority::from_static("example.com");
        assert_eq!(authority, "example.com");
    }
    #[test]
    fn not_equal_with_a_str_of_a_different_authority() {
        let authority: Authority = "example.com".parse().unwrap();
        assert_ne!(& authority, "test.com");
        assert_ne!("test.com", & authority);
        assert_ne!(authority, "test.com");
        assert_ne!("test.com", authority);
    }
    #[test]
    fn equates_with_a_string() {
        let authority: Authority = "example.com".parse().unwrap();
        assert_eq!(authority, "EXAMPLE.com".to_string());
        assert_eq!("EXAMPLE.com".to_string(), authority);
    }
    #[test]
    fn equates_with_a_string_of_a_different_authority() {
        let authority: Authority = "example.com".parse().unwrap();
        assert_ne!(authority, "test.com".to_string());
        assert_ne!("test.com".to_string(), authority);
    }
    #[test]
    fn compares_to_self() {
        let authority1: Authority = "abc.com".parse().unwrap();
        let authority2: Authority = "def.com".parse().unwrap();
        assert!(authority1 < authority2);
        assert!(authority2 > authority1);
    }
    #[test]
    fn compares_with_a_str() {
        let authority: Authority = "def.com".parse().unwrap();
        assert!(& authority < "ghi.com");
        assert!("ghi.com" > & authority);
        assert!(& authority > "abc.com");
        assert!("abc.com" < & authority);
        assert!(authority < "ghi.com");
        assert!("ghi.com" > authority);
        assert!(authority > "abc.com");
        assert!("abc.com" < authority);
    }
    #[test]
    fn compares_with_a_string() {
        let authority: Authority = "def.com".parse().unwrap();
        assert!(authority < "ghi.com".to_string());
        assert!("ghi.com".to_string() > authority);
        assert!(authority > "abc.com".to_string());
        assert!("abc.com".to_string() < authority);
    }
    #[test]
    fn allows_percent_in_userinfo() {
        let authority_str = "a%2f:b%2f@example.com";
        let authority: Authority = authority_str.parse().unwrap();
        assert_eq!(authority, authority_str);
    }
    #[test]
    fn rejects_percent_in_hostname() {
        let err = Authority::parse_non_empty(b"example%2f.com").unwrap_err();
        assert_eq!(err.0, ErrorKind::InvalidAuthority);
        let err = Authority::parse_non_empty(b"a%2f:b%2f@example%2f.com").unwrap_err();
        assert_eq!(err.0, ErrorKind::InvalidAuthority);
    }
    #[test]
    fn allows_percent_in_ipv6_address() {
        let authority_str = "[fe80::1:2:3:4%25eth0]";
        let result: Authority = authority_str.parse().unwrap();
        assert_eq!(result, authority_str);
    }
    #[test]
    fn rejects_percent_outside_ipv6_address() {
        let err = Authority::parse_non_empty(b"1234%20[fe80::1:2:3:4]").unwrap_err();
        assert_eq!(err.0, ErrorKind::InvalidAuthority);
        let err = Authority::parse_non_empty(b"[fe80::1:2:3:4]%20").unwrap_err();
        assert_eq!(err.0, ErrorKind::InvalidAuthority);
    }
    #[test]
    fn rejects_invalid_utf8() {
        let err = Authority::try_from([0xc0u8].as_ref()).unwrap_err();
        assert_eq!(err.0, ErrorKind::InvalidUriChar);
        let err = Authority::from_shared(Bytes::from_static([0xc0u8].as_ref()))
            .unwrap_err();
        assert_eq!(err.0, ErrorKind::InvalidUriChar);
    }
    #[test]
    fn rejects_invalid_use_of_brackets() {
        let err = Authority::parse_non_empty(b"[]@[").unwrap_err();
        assert_eq!(err.0, ErrorKind::InvalidAuthority);
    }
}
#[cfg(test)]
mod tests_llm_16_249 {
    use super::*;
    use crate::*;
    use bytes::Bytes;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_249_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "example.com";
        let rug_fuzz_1 = "example.com";
        let rug_fuzz_2 = "EXAMPLE.COM";
        let rug_fuzz_3 = "example.org";
        let authority = Authority {
            data: ByteStr::from_static(rug_fuzz_0),
        };
        debug_assert_eq!(authority.eq(& rug_fuzz_1), true);
        debug_assert_eq!(authority.eq(& rug_fuzz_2), true);
        debug_assert_eq!(authority.eq(& rug_fuzz_3), false);
        let _rug_ed_tests_llm_16_249_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_250 {
    use super::*;
    use crate::*;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_250_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "example.com";
        let rug_fuzz_1 = "EXAMPLE.com";
        let authority = Authority {
            data: ByteStr::from_static(rug_fuzz_0),
        };
        let other = String::from(rug_fuzz_1);
        debug_assert_eq!(authority.eq(& other), true);
        let _rug_ed_tests_llm_16_250_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_251 {
    use super::*;
    use crate::*;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_251_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "example.com";
        let rug_fuzz_1 = "example.com";
        let rug_fuzz_2 = "EXAMPLE.COM";
        let rug_fuzz_3 = "test.com";
        let authority = Authority {
            data: ByteStr::from(rug_fuzz_0),
        };
        debug_assert_eq!(authority.eq(rug_fuzz_1), true);
        debug_assert_eq!(authority.eq(rug_fuzz_2), true);
        debug_assert_eq!(authority.eq(rug_fuzz_3), false);
        let _rug_ed_tests_llm_16_251_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_252 {
    use super::*;
    use crate::*;
    use bytes::Bytes;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_252_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "example.com";
        let rug_fuzz_1 = "example.com";
        let rug_fuzz_2 = "example.com";
        let rug_fuzz_3 = "EXAMPLE.COM";
        let rug_fuzz_4 = "example.com";
        let rug_fuzz_5 = "example.org";
        let rug_fuzz_6 = "example.com";
        let rug_fuzz_7 = "EXAMPLE.ORG";
        let rug_fuzz_8 = "example.com";
        let rug_fuzz_9 = "example.com:8080";
        let rug_fuzz_10 = "example.com:8080";
        let rug_fuzz_11 = "example.com:8080";
        let rug_fuzz_12 = "example.com:8080";
        let rug_fuzz_13 = "example.com:9090";
        let authority1 = Authority {
            data: ByteStr::from_static(rug_fuzz_0),
        };
        let authority2 = Authority {
            data: ByteStr::from_static(rug_fuzz_1),
        };
        debug_assert_eq!(authority1.eq(& authority2), true);
        let authority3 = Authority {
            data: ByteStr::from_static(rug_fuzz_2),
        };
        let authority4 = Authority {
            data: ByteStr::from_static(rug_fuzz_3),
        };
        debug_assert_eq!(authority3.eq(& authority4), true);
        let authority5 = Authority {
            data: ByteStr::from_static(rug_fuzz_4),
        };
        let authority6 = Authority {
            data: ByteStr::from_static(rug_fuzz_5),
        };
        debug_assert_eq!(authority5.eq(& authority6), false);
        let authority7 = Authority {
            data: ByteStr::from_static(rug_fuzz_6),
        };
        let authority8 = Authority {
            data: ByteStr::from_static(rug_fuzz_7),
        };
        debug_assert_eq!(authority7.eq(& authority8), false);
        let authority9 = Authority {
            data: ByteStr::from_static(rug_fuzz_8),
        };
        let authority10 = Authority {
            data: ByteStr::from_static(rug_fuzz_9),
        };
        debug_assert_eq!(authority9.eq(& authority10), false);
        let authority11 = Authority {
            data: ByteStr::from_static(rug_fuzz_10),
        };
        let authority12 = Authority {
            data: ByteStr::from_static(rug_fuzz_11),
        };
        debug_assert_eq!(authority11.eq(& authority12), true);
        let authority13 = Authority {
            data: ByteStr::from_static(rug_fuzz_12),
        };
        let authority14 = Authority {
            data: ByteStr::from_static(rug_fuzz_13),
        };
        debug_assert_eq!(authority13.eq(& authority14), false);
        let _rug_ed_tests_llm_16_252_rrrruuuugggg_test_eq = 0;
    }
    #[test]
    fn test_from_str() {
        let _rug_st_tests_llm_16_252_rrrruuuugggg_test_from_str = 0;
        let rug_fuzz_0 = "example.com:8080";
        let authority = Authority::from_str(rug_fuzz_0).unwrap();
        debug_assert_eq!(authority.as_str(), "example.com:8080");
        let _rug_ed_tests_llm_16_252_rrrruuuugggg_test_from_str = 0;
    }
    #[test]
    fn test_as_str() {
        let _rug_st_tests_llm_16_252_rrrruuuugggg_test_as_str = 0;
        let rug_fuzz_0 = "example.com:8080";
        let authority = Authority::from_str(rug_fuzz_0).unwrap();
        debug_assert_eq!(authority.as_str(), "example.com:8080");
        let _rug_ed_tests_llm_16_252_rrrruuuugggg_test_as_str = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_258 {
    use std::convert::TryFrom;
    use std::cmp;
    use bytes::Bytes;
    use crate::byte_str::ByteStr;
    use crate::uri::authority::Authority;
    #[test]
    fn test_partial_cmp() {
        let _rug_st_tests_llm_16_258_rrrruuuugggg_test_partial_cmp = 0;
        let rug_fuzz_0 = "example.com";
        let rug_fuzz_1 = "example.org";
        let authority = Authority::try_from(rug_fuzz_0).unwrap();
        let other = rug_fuzz_1;
        let result = authority.partial_cmp(other);
        let expected = Some(cmp::Ordering::Less);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_258_rrrruuuugggg_test_partial_cmp = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_259 {
    use super::*;
    use crate::*;
    use std::cmp::Ordering;
    #[test]
    fn test_partial_cmp_equal() {
        let _rug_st_tests_llm_16_259_rrrruuuugggg_test_partial_cmp_equal = 0;
        let rug_fuzz_0 = "example.com";
        let rug_fuzz_1 = "example.com";
        let authority1 = Authority {
            data: ByteStr::from_static(rug_fuzz_0),
        };
        let authority2 = Authority {
            data: ByteStr::from_static(rug_fuzz_1),
        };
        let result = authority1.partial_cmp(&authority2);
        debug_assert_eq!(result, Some(Ordering::Equal));
        let _rug_ed_tests_llm_16_259_rrrruuuugggg_test_partial_cmp_equal = 0;
    }
    #[test]
    fn test_partial_cmp_less() {
        let _rug_st_tests_llm_16_259_rrrruuuugggg_test_partial_cmp_less = 0;
        let rug_fuzz_0 = "example.com";
        let rug_fuzz_1 = "example.net";
        let authority1 = Authority {
            data: ByteStr::from_static(rug_fuzz_0),
        };
        let authority2 = Authority {
            data: ByteStr::from_static(rug_fuzz_1),
        };
        let result = authority1.partial_cmp(&authority2);
        debug_assert_eq!(result, Some(Ordering::Less));
        let _rug_ed_tests_llm_16_259_rrrruuuugggg_test_partial_cmp_less = 0;
    }
    #[test]
    fn test_partial_cmp_greater() {
        let _rug_st_tests_llm_16_259_rrrruuuugggg_test_partial_cmp_greater = 0;
        let rug_fuzz_0 = "example.net";
        let rug_fuzz_1 = "example.com";
        let authority1 = Authority {
            data: ByteStr::from_static(rug_fuzz_0),
        };
        let authority2 = Authority {
            data: ByteStr::from_static(rug_fuzz_1),
        };
        let result = authority1.partial_cmp(&authority2);
        debug_assert_eq!(result, Some(Ordering::Greater));
        let _rug_ed_tests_llm_16_259_rrrruuuugggg_test_partial_cmp_greater = 0;
    }
    #[test]
    fn test_partial_cmp_none() {
        let _rug_st_tests_llm_16_259_rrrruuuugggg_test_partial_cmp_none = 0;
        let rug_fuzz_0 = "example.com";
        let rug_fuzz_1 = "example";
        let authority1 = Authority {
            data: ByteStr::from_static(rug_fuzz_0),
        };
        let authority2 = Authority {
            data: ByteStr::from_static(rug_fuzz_1),
        };
        let result = authority1.partial_cmp(&authority2);
        debug_assert_eq!(result, None);
        let _rug_ed_tests_llm_16_259_rrrruuuugggg_test_partial_cmp_none = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_260 {
    use super::*;
    use crate::*;
    use bytes::Bytes;
    use std::convert::TryFrom;
    #[test]
    fn test_as_ref() {
        let _rug_st_tests_llm_16_260_rrrruuuugggg_test_as_ref = 0;
        let rug_fuzz_0 = "example.com";
        let authority = Authority::from_static(rug_fuzz_0);
        let result = authority.as_ref();
        debug_assert_eq!(result, "example.com");
        let _rug_ed_tests_llm_16_260_rrrruuuugggg_test_as_ref = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_261 {
    use std::convert::TryFrom;
    use crate::uri::Authority;
    #[test]
    fn test_try_from() {
        let _rug_st_tests_llm_16_261_rrrruuuugggg_test_try_from = 0;
        let rug_fuzz_0 = 97;
        let rug_fuzz_1 = 98;
        let rug_fuzz_2 = 99;
        let s: &[u8] = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        let result = Authority::try_from(s);
        debug_assert!(result.is_ok());
        let authority: Authority = result.unwrap();
        let _rug_ed_tests_llm_16_261_rrrruuuugggg_test_try_from = 0;
    }
}
#[test]
fn test_try_from() {
    use std::convert::TryInto;
    use crate::uri::authority::Authority;
    let s: &str = "example.com";
    let authority: Result<Authority, _> = TryInto::<Authority>::try_into(s);
    assert!(authority.is_ok());
    let s: &str = "example.com:8080";
    let authority: Result<Authority, _> = TryInto::<Authority>::try_into(s);
    assert!(authority.is_ok());
    let s: &str = "example.com:";
    let authority: Result<Authority, _> = TryInto::<Authority>::try_into(s);
    assert!(authority.is_err());
    let s: &str = ":8080";
    let authority: Result<Authority, _> = TryInto::<Authority>::try_into(s);
    assert!(authority.is_err());
    let s: &str = ":";
    let authority: Result<Authority, _> = TryInto::<Authority>::try_into(s);
    assert!(authority.is_err());
}
#[cfg(test)]
mod tests_llm_16_560 {
    use super::*;
    use crate::*;
    #[test]
    fn eq_returns_true_when_authorities_are_equal() {
        let _rug_st_tests_llm_16_560_rrrruuuugggg_eq_returns_true_when_authorities_are_equal = 0;
        let rug_fuzz_0 = "example.com:8080";
        let rug_fuzz_1 = "example.com:8080";
        let authority_1: Authority = rug_fuzz_0.parse().unwrap();
        let authority_2: Authority = rug_fuzz_1.parse().unwrap();
        debug_assert_eq!(authority_1.eq(& authority_2), true);
        let _rug_ed_tests_llm_16_560_rrrruuuugggg_eq_returns_true_when_authorities_are_equal = 0;
    }
    #[test]
    fn eq_returns_false_when_authorities_are_not_equal() {
        let _rug_st_tests_llm_16_560_rrrruuuugggg_eq_returns_false_when_authorities_are_not_equal = 0;
        let rug_fuzz_0 = "example.com:8080";
        let rug_fuzz_1 = "example.com:9090";
        let authority_1: Authority = rug_fuzz_0.parse().unwrap();
        let authority_2: Authority = rug_fuzz_1.parse().unwrap();
        debug_assert_eq!(authority_1.eq(& authority_2), false);
        let _rug_ed_tests_llm_16_560_rrrruuuugggg_eq_returns_false_when_authorities_are_not_equal = 0;
    }
    #[test]
    fn eq_returns_false_when_authorities_are_not_equal_ignore_case() {
        let _rug_st_tests_llm_16_560_rrrruuuugggg_eq_returns_false_when_authorities_are_not_equal_ignore_case = 0;
        let rug_fuzz_0 = "example.com:8080";
        let rug_fuzz_1 = "EXAMPLE.com:8080";
        let authority_1: Authority = rug_fuzz_0.parse().unwrap();
        let authority_2: Authority = rug_fuzz_1.parse().unwrap();
        debug_assert_eq!(authority_1.eq(& authority_2), false);
        let _rug_ed_tests_llm_16_560_rrrruuuugggg_eq_returns_false_when_authorities_are_not_equal_ignore_case = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_563 {
    use super::*;
    use crate::*;
    use crate::uri::InvalidUri;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_563_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "example.com";
        let rug_fuzz_1 = "example.com";
        let authority1 = Authority::from_static(rug_fuzz_0);
        let authority2 = Authority::from_static(rug_fuzz_1);
        let result = authority1.eq(&authority2);
        debug_assert_eq!(result, true);
        let _rug_ed_tests_llm_16_563_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_564 {
    use super::*;
    use crate::*;
    use uri::authority::{Authority, InvalidUri};
    use std::cmp::Ordering;
    #[test]
    fn test_partial_cmp() {
        let _rug_st_tests_llm_16_564_rrrruuuugggg_test_partial_cmp = 0;
        let rug_fuzz_0 = "example.com";
        let rug_fuzz_1 = "example.org";
        let authority1 = Authority {
            data: ByteStr::from_static(rug_fuzz_0),
        };
        let authority2 = Authority {
            data: ByteStr::from_static(rug_fuzz_1),
        };
        debug_assert_eq!(authority1.partial_cmp(& authority2), Some(Ordering::Less));
        let _rug_ed_tests_llm_16_564_rrrruuuugggg_test_partial_cmp = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_565 {
    use super::*;
    use crate::*;
    use std::cmp::Ordering;
    #[test]
    fn test_partial_cmp() {
        let _rug_st_tests_llm_16_565_rrrruuuugggg_test_partial_cmp = 0;
        let rug_fuzz_0 = "example.com";
        let rug_fuzz_1 = "example.org";
        let authority1 = Authority {
            data: ByteStr::from(rug_fuzz_0),
        };
        let authority2 = Authority {
            data: ByteStr::from(rug_fuzz_1),
        };
        debug_assert_eq!(authority1.partial_cmp(& authority2), Some(Ordering::Less));
        let _rug_ed_tests_llm_16_565_rrrruuuugggg_test_partial_cmp = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_566 {
    use super::*;
    use crate::*;
    use std::cmp::Ordering;
    #[test]
    fn test_partial_cmp_equal() {
        let _rug_st_tests_llm_16_566_rrrruuuugggg_test_partial_cmp_equal = 0;
        let rug_fuzz_0 = "example.com";
        let rug_fuzz_1 = "example.com";
        let authority1 = Authority::from_static(rug_fuzz_0);
        let authority2 = Authority::from_static(rug_fuzz_1);
        let result = authority1.partial_cmp(&authority2);
        debug_assert_eq!(result, Some(Ordering::Equal));
        let _rug_ed_tests_llm_16_566_rrrruuuugggg_test_partial_cmp_equal = 0;
    }
    #[test]
    fn test_partial_cmp_less() {
        let _rug_st_tests_llm_16_566_rrrruuuugggg_test_partial_cmp_less = 0;
        let rug_fuzz_0 = "example.com";
        let rug_fuzz_1 = "example.org";
        let authority1 = Authority::from_static(rug_fuzz_0);
        let authority2 = Authority::from_static(rug_fuzz_1);
        let result = authority1.partial_cmp(&authority2);
        debug_assert_eq!(result, Some(Ordering::Less));
        let _rug_ed_tests_llm_16_566_rrrruuuugggg_test_partial_cmp_less = 0;
    }
    #[test]
    fn test_partial_cmp_greater() {
        let _rug_st_tests_llm_16_566_rrrruuuugggg_test_partial_cmp_greater = 0;
        let rug_fuzz_0 = "example.org";
        let rug_fuzz_1 = "example.com";
        let authority1 = Authority::from_static(rug_fuzz_0);
        let authority2 = Authority::from_static(rug_fuzz_1);
        let result = authority1.partial_cmp(&authority2);
        debug_assert_eq!(result, Some(Ordering::Greater));
        let _rug_ed_tests_llm_16_566_rrrruuuugggg_test_partial_cmp_greater = 0;
    }
    #[test]
    fn test_partial_cmp_none() {
        let _rug_st_tests_llm_16_566_rrrruuuugggg_test_partial_cmp_none = 0;
        let rug_fuzz_0 = "example.com";
        let rug_fuzz_1 = "example";
        let authority1 = Authority::from_static(rug_fuzz_0);
        let authority2 = Authority::from_static(rug_fuzz_1);
        let result = authority1.partial_cmp(&authority2);
        debug_assert_eq!(result, None);
        let _rug_ed_tests_llm_16_566_rrrruuuugggg_test_partial_cmp_none = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_567 {
    use super::*;
    use crate::*;
    #[test]
    fn test_as_str() {
        let _rug_st_tests_llm_16_567_rrrruuuugggg_test_as_str = 0;
        let rug_fuzz_0 = "example.com";
        let authority = Authority {
            data: ByteStr::from_static(rug_fuzz_0),
        };
        let result = authority.as_str();
        debug_assert_eq!(result, "example.com");
        let _rug_ed_tests_llm_16_567_rrrruuuugggg_test_as_str = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_568 {
    use super::*;
    use crate::*;
    #[test]
    fn test_empty() {
        let _rug_st_tests_llm_16_568_rrrruuuugggg_test_empty = 0;
        let empty_authority = Authority::empty();
        debug_assert_eq!(empty_authority.as_str(), "");
        let _rug_ed_tests_llm_16_568_rrrruuuugggg_test_empty = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_570 {
    use bytes::Bytes;
    use crate::uri::authority::{Authority, InvalidUri};
    #[test]
    fn test_from_shared() {
        let _rug_st_tests_llm_16_570_rrrruuuugggg_test_from_shared = 0;
        let rug_fuzz_0 = "example.com";
        let bytes = Bytes::from(rug_fuzz_0);
        let result = Authority::from_shared(bytes);
        debug_assert!(result.is_ok());
        let _rug_ed_tests_llm_16_570_rrrruuuugggg_test_from_shared = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_571 {
    use super::*;
    use crate::*;
    #[test]
    fn test_from_static() {
        let _rug_st_tests_llm_16_571_rrrruuuugggg_test_from_static = 0;
        let rug_fuzz_0 = "example.com";
        let authority = Authority::from_static(rug_fuzz_0);
        debug_assert_eq!(authority.host(), "example.com");
        let _rug_ed_tests_llm_16_571_rrrruuuugggg_test_from_static = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_572 {
    use super::*;
    use crate::*;
    use bytes::Bytes;
    use std::convert::TryFrom;
    #[test]
    fn test_host() {
        let _rug_st_tests_llm_16_572_rrrruuuugggg_test_host = 0;
        let rug_fuzz_0 = "example.org:80";
        let authority: Authority = rug_fuzz_0.parse().unwrap();
        debug_assert_eq!(authority.host(), "example.org");
        let _rug_ed_tests_llm_16_572_rrrruuuugggg_test_host = 0;
    }
}
mod tests_llm_16_575_llm_16_574 {
    use crate::uri::authority::*;
    use crate::uri::InvalidUri;
    use crate::uri::authority::Authority;
    #[test]
    fn test_parse_non_empty() {
        let _rug_st_tests_llm_16_575_llm_16_574_rrrruuuugggg_test_parse_non_empty = 0;
        let rug_fuzz_0 = b'f';
        let rug_fuzz_1 = b'o';
        let rug_fuzz_2 = b'o';
        let empty_str: &[u8] = &[];
        debug_assert!(Authority::parse_non_empty(empty_str).is_err());
        let non_empty_str: &[u8] = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        debug_assert!(Authority::parse_non_empty(non_empty_str).is_ok());
        let _rug_ed_tests_llm_16_575_llm_16_574_rrrruuuugggg_test_parse_non_empty = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_576 {
    use super::*;
    use crate::*;
    use crate::uri::Port;
    #[test]
    fn test_port_with_port() {
        let _rug_st_tests_llm_16_576_rrrruuuugggg_test_port_with_port = 0;
        let rug_fuzz_0 = "example.org:80";
        let authority: Authority = rug_fuzz_0.parse().unwrap();
        let port = authority.port().unwrap();
        debug_assert_eq!(port.as_u16(), 80);
        debug_assert_eq!(port.as_str(), "80");
        let _rug_ed_tests_llm_16_576_rrrruuuugggg_test_port_with_port = 0;
    }
    #[test]
    fn test_port_without_port() {
        let _rug_st_tests_llm_16_576_rrrruuuugggg_test_port_without_port = 0;
        let rug_fuzz_0 = "example.org";
        let authority: Authority = rug_fuzz_0.parse().unwrap();
        debug_assert!(authority.port().is_none());
        let _rug_ed_tests_llm_16_576_rrrruuuugggg_test_port_without_port = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_579 {
    use super::*;
    use crate::*;
    #[test]
    fn test_host_with_ipv4() {
        let _rug_st_tests_llm_16_579_rrrruuuugggg_test_host_with_ipv4 = 0;
        let rug_fuzz_0 = "127.0.0.1:8080";
        debug_assert_eq!(host(rug_fuzz_0), "127.0.0.1");
        let _rug_ed_tests_llm_16_579_rrrruuuugggg_test_host_with_ipv4 = 0;
    }
    #[test]
    fn test_host_with_ipv6() {
        let _rug_st_tests_llm_16_579_rrrruuuugggg_test_host_with_ipv6 = 0;
        let rug_fuzz_0 = "[2001:0db8:85a3:0000:0000:8a2e:0370:7334]:8080";
        debug_assert_eq!(host(rug_fuzz_0), "[2001:0db8:85a3:0000:0000:8a2e:0370:7334]");
        let _rug_ed_tests_llm_16_579_rrrruuuugggg_test_host_with_ipv6 = 0;
    }
    #[test]
    fn test_host_with_username() {
        let _rug_st_tests_llm_16_579_rrrruuuugggg_test_host_with_username = 0;
        let rug_fuzz_0 = "username@example.com:8080";
        debug_assert_eq!(host(rug_fuzz_0), "example.com");
        let _rug_ed_tests_llm_16_579_rrrruuuugggg_test_host_with_username = 0;
    }
}
#[cfg(test)]
mod tests_rug_147 {
    use super::*;
    use crate::uri::authority::{ErrorKind, InvalidUri};
    #[test]
    fn test_parse() {
        let _rug_st_tests_rug_147_rrrruuuugggg_test_parse = 0;
        let rug_fuzz_0 = b"example.com";
        let mut p0: &[u8] = rug_fuzz_0;
        crate::uri::authority::Authority::parse(p0).unwrap();
        let _rug_ed_tests_rug_147_rrrruuuugggg_test_parse = 0;
    }
}
#[cfg(test)]
mod tests_rug_148 {
    use super::*;
    use crate::uri::Authority;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_148_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "example.org:80";
        let mut p0 = Authority::from_static(rug_fuzz_0);
        <Authority>::port_u16(&p0);
        let _rug_ed_tests_rug_148_rrrruuuugggg_test_rug = 0;
    }
}
