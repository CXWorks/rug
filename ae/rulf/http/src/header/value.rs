use bytes::{Bytes, BytesMut};
use std::convert::TryFrom;
use std::error::Error;
use std::str::FromStr;
use std::{cmp, fmt, mem, str};
use crate::header::name::HeaderName;
/// Represents an HTTP header field value.
///
/// In practice, HTTP header field values are usually valid ASCII. However, the
/// HTTP spec allows for a header value to contain opaque bytes as well. In this
/// case, the header field value is not able to be represented as a string.
///
/// To handle this, the `HeaderValue` is useable as a type and can be compared
/// with strings and implements `Debug`. A `to_str` fn is provided that returns
/// an `Err` if the header value contains non visible ascii characters.
#[derive(Clone, Hash)]
pub struct HeaderValue {
    inner: Bytes,
    is_sensitive: bool,
}
/// A possible error when converting a `HeaderValue` from a string or byte
/// slice.
pub struct InvalidHeaderValue {
    _priv: (),
}
/// A possible error when converting a `HeaderValue` to a string representation.
///
/// Header field values may contain opaque bytes, in which case it is not
/// possible to represent the value as a string.
#[derive(Debug)]
pub struct ToStrError {
    _priv: (),
}
impl HeaderValue {
    /// Convert a static string to a `HeaderValue`.
    ///
    /// This function will not perform any copying, however the string is
    /// checked to ensure that no invalid characters are present. Only visible
    /// ASCII characters (32-127) are permitted.
    ///
    /// # Panics
    ///
    /// This function panics if the argument contains invalid header value
    /// characters.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::header::HeaderValue;
    /// let val = HeaderValue::from_static("hello");
    /// assert_eq!(val, "hello");
    /// ```
    #[inline]
    pub fn from_static(src: &'static str) -> HeaderValue {
        let bytes = src.as_bytes();
        for &b in bytes {
            if !is_visible_ascii(b) {
                panic!("invalid header value");
            }
        }
        HeaderValue {
            inner: Bytes::from_static(bytes),
            is_sensitive: false,
        }
    }
    /// Attempt to convert a string to a `HeaderValue`.
    ///
    /// If the argument contains invalid header value characters, an error is
    /// returned. Only visible ASCII characters (32-127) are permitted. Use
    /// `from_bytes` to create a `HeaderValue` that includes opaque octets
    /// (128-255).
    ///
    /// This function is intended to be replaced in the future by a `TryFrom`
    /// implementation once the trait is stabilized in std.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::header::HeaderValue;
    /// let val = HeaderValue::from_str("hello").unwrap();
    /// assert_eq!(val, "hello");
    /// ```
    ///
    /// An invalid value
    ///
    /// ```
    /// # use http::header::HeaderValue;
    /// let val = HeaderValue::from_str("\n");
    /// assert!(val.is_err());
    /// ```
    #[inline]
    pub fn from_str(src: &str) -> Result<HeaderValue, InvalidHeaderValue> {
        HeaderValue::try_from_generic(src, |s| Bytes::copy_from_slice(s.as_bytes()))
    }
    /// Converts a HeaderName into a HeaderValue
    ///
    /// Since every valid HeaderName is a valid HeaderValue this is done infallibly.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::header::{HeaderValue, HeaderName};
    /// # use http::header::ACCEPT;
    /// let val = HeaderValue::from_name(ACCEPT);
    /// assert_eq!(val, HeaderValue::from_bytes(b"accept").unwrap());
    /// ```
    #[inline]
    pub fn from_name(name: HeaderName) -> HeaderValue {
        name.into()
    }
    /// Attempt to convert a byte slice to a `HeaderValue`.
    ///
    /// If the argument contains invalid header value bytes, an error is
    /// returned. Only byte values between 32 and 255 (inclusive) are permitted,
    /// excluding byte 127 (DEL).
    ///
    /// This function is intended to be replaced in the future by a `TryFrom`
    /// implementation once the trait is stabilized in std.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::header::HeaderValue;
    /// let val = HeaderValue::from_bytes(b"hello\xfa").unwrap();
    /// assert_eq!(val, &b"hello\xfa"[..]);
    /// ```
    ///
    /// An invalid value
    ///
    /// ```
    /// # use http::header::HeaderValue;
    /// let val = HeaderValue::from_bytes(b"\n");
    /// assert!(val.is_err());
    /// ```
    #[inline]
    pub fn from_bytes(src: &[u8]) -> Result<HeaderValue, InvalidHeaderValue> {
        HeaderValue::try_from_generic(src, Bytes::copy_from_slice)
    }
    /// Attempt to convert a `Bytes` buffer to a `HeaderValue`.
    ///
    /// This will try to prevent a copy if the type passed is the type used
    /// internally, and will copy the data if it is not.
    pub fn from_maybe_shared<T>(src: T) -> Result<HeaderValue, InvalidHeaderValue>
    where
        T: AsRef<[u8]> + 'static,
    {
        if_downcast_into!(T, Bytes, src, { return HeaderValue::from_shared(src); });
        HeaderValue::from_bytes(src.as_ref())
    }
    /// Convert a `Bytes` directly into a `HeaderValue` without validating.
    ///
    /// This function does NOT validate that illegal bytes are not contained
    /// within the buffer.
    pub unsafe fn from_maybe_shared_unchecked<T>(src: T) -> HeaderValue
    where
        T: AsRef<[u8]> + 'static,
    {
        if cfg!(debug_assertions) {
            match HeaderValue::from_maybe_shared(src) {
                Ok(val) => val,
                Err(_err) => {
                    panic!(
                        "HeaderValue::from_maybe_shared_unchecked() with invalid bytes"
                    );
                }
            }
        } else {
            if_downcast_into!(
                T, Bytes, src, { return HeaderValue { inner : src, is_sensitive : false,
                }; }
            );
            let src = Bytes::copy_from_slice(src.as_ref());
            HeaderValue {
                inner: src,
                is_sensitive: false,
            }
        }
    }
    fn from_shared(src: Bytes) -> Result<HeaderValue, InvalidHeaderValue> {
        HeaderValue::try_from_generic(src, std::convert::identity)
    }
    fn try_from_generic<T: AsRef<[u8]>, F: FnOnce(T) -> Bytes>(
        src: T,
        into: F,
    ) -> Result<HeaderValue, InvalidHeaderValue> {
        for &b in src.as_ref() {
            if !is_valid(b) {
                return Err(InvalidHeaderValue { _priv: () });
            }
        }
        Ok(HeaderValue {
            inner: into(src),
            is_sensitive: false,
        })
    }
    /// Yields a `&str` slice if the `HeaderValue` only contains visible ASCII
    /// chars.
    ///
    /// This function will perform a scan of the header value, checking all the
    /// characters.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::header::HeaderValue;
    /// let val = HeaderValue::from_static("hello");
    /// assert_eq!(val.to_str().unwrap(), "hello");
    /// ```
    pub fn to_str(&self) -> Result<&str, ToStrError> {
        let bytes = self.as_ref();
        for &b in bytes {
            if !is_visible_ascii(b) {
                return Err(ToStrError { _priv: () });
            }
        }
        unsafe { Ok(str::from_utf8_unchecked(bytes)) }
    }
    /// Returns the length of `self`.
    ///
    /// This length is in bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::header::HeaderValue;
    /// let val = HeaderValue::from_static("hello");
    /// assert_eq!(val.len(), 5);
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.as_ref().len()
    }
    /// Returns true if the `HeaderValue` has a length of zero bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::header::HeaderValue;
    /// let val = HeaderValue::from_static("");
    /// assert!(val.is_empty());
    ///
    /// let val = HeaderValue::from_static("hello");
    /// assert!(!val.is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Converts a `HeaderValue` to a byte slice.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::header::HeaderValue;
    /// let val = HeaderValue::from_static("hello");
    /// assert_eq!(val.as_bytes(), b"hello");
    /// ```
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        self.as_ref()
    }
    /// Mark that the header value represents sensitive information.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::header::HeaderValue;
    /// let mut val = HeaderValue::from_static("my secret");
    ///
    /// val.set_sensitive(true);
    /// assert!(val.is_sensitive());
    ///
    /// val.set_sensitive(false);
    /// assert!(!val.is_sensitive());
    /// ```
    #[inline]
    pub fn set_sensitive(&mut self, val: bool) {
        self.is_sensitive = val;
    }
    /// Returns `true` if the value represents sensitive data.
    ///
    /// Sensitive data could represent passwords or other data that should not
    /// be stored on disk or in memory. By marking header values as sensitive,
    /// components using this crate can be instructed to treat them with special
    /// care for security reasons. For example, caches can avoid storing
    /// sensitive values, and HPACK encoders used by HTTP/2.0 implementations
    /// can choose not to compress them.
    ///
    /// Additionally, sensitive values will be masked by the `Debug`
    /// implementation of `HeaderValue`.
    ///
    /// Note that sensitivity is not factored into equality or ordering.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::header::HeaderValue;
    /// let mut val = HeaderValue::from_static("my secret");
    ///
    /// val.set_sensitive(true);
    /// assert!(val.is_sensitive());
    ///
    /// val.set_sensitive(false);
    /// assert!(!val.is_sensitive());
    /// ```
    #[inline]
    pub fn is_sensitive(&self) -> bool {
        self.is_sensitive
    }
}
impl AsRef<[u8]> for HeaderValue {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.inner.as_ref()
    }
}
impl fmt::Debug for HeaderValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_sensitive {
            f.write_str("Sensitive")
        } else {
            f.write_str("\"")?;
            let mut from = 0;
            let bytes = self.as_bytes();
            for (i, &b) in bytes.iter().enumerate() {
                if !is_visible_ascii(b) || b == b'"' {
                    if from != i {
                        f.write_str(unsafe {
                            str::from_utf8_unchecked(&bytes[from..i])
                        })?;
                    }
                    if b == b'"' {
                        f.write_str("\\\"")?;
                    } else {
                        write!(f, "\\x{:x}", b)?;
                    }
                    from = i + 1;
                }
            }
            f.write_str(unsafe { str::from_utf8_unchecked(&bytes[from..]) })?;
            f.write_str("\"")
        }
    }
}
impl From<HeaderName> for HeaderValue {
    #[inline]
    fn from(h: HeaderName) -> HeaderValue {
        HeaderValue {
            inner: h.into_bytes(),
            is_sensitive: false,
        }
    }
}
macro_rules! from_integers {
    ($($name:ident : $t:ident => $max_len:expr),*) => {
        $(impl From <$t > for HeaderValue { fn from(num : $t) -> HeaderValue { let mut
        buf = if mem::size_of::< BytesMut > () - 1 < $max_len { if num as u64 >
        999_999_999_999_999_999 { BytesMut::with_capacity($max_len) } else {
        BytesMut::new() } } else { BytesMut::new() }; let _ = ::itoa::fmt(& mut buf,
        num); HeaderValue { inner : buf.freeze(), is_sensitive : false, } } } #[test] fn
        $name () { let n : $t = 55; let val = HeaderValue::from(n); assert_eq!(val, & n
        .to_string()); let n = ::std::$t ::MAX; let val = HeaderValue::from(n);
        assert_eq!(val, & n.to_string()); })*
    };
}
from_integers! {
    from_u16 : u16 => 5, from_i16 : i16 => 6, from_u32 : u32 => 10, from_i32 : i32 => 11,
    from_u64 : u64 => 20, from_i64 : i64 => 20
}
#[cfg(target_pointer_width = "16")]
from_integers! {
    from_usize : usize => 5, from_isize : isize => 6
}
#[cfg(target_pointer_width = "32")]
from_integers! {
    from_usize : usize => 10, from_isize : isize => 11
}
#[cfg(target_pointer_width = "64")]
from_integers! {
    from_usize : usize => 20, from_isize : isize => 20
}
#[cfg(test)]
mod from_header_name_tests {
    use super::*;
    use crate::header::map::HeaderMap;
    use crate::header::name;
    #[test]
    fn it_can_insert_header_name_as_header_value() {
        let mut map = HeaderMap::new();
        map.insert(name::UPGRADE, name::SEC_WEBSOCKET_PROTOCOL.into());
        map.insert(
            name::ACCEPT,
            name::HeaderName::from_bytes(b"hello-world").unwrap().into(),
        );
        assert_eq!(
            map.get(name::UPGRADE).unwrap(),
            HeaderValue::from_bytes(b"sec-websocket-protocol").unwrap()
        );
        assert_eq!(
            map.get(name::ACCEPT).unwrap(), HeaderValue::from_bytes(b"hello-world")
            .unwrap()
        );
    }
}
impl FromStr for HeaderValue {
    type Err = InvalidHeaderValue;
    #[inline]
    fn from_str(s: &str) -> Result<HeaderValue, Self::Err> {
        HeaderValue::from_str(s)
    }
}
impl<'a> From<&'a HeaderValue> for HeaderValue {
    #[inline]
    fn from(t: &'a HeaderValue) -> Self {
        t.clone()
    }
}
impl<'a> TryFrom<&'a str> for HeaderValue {
    type Error = InvalidHeaderValue;
    #[inline]
    fn try_from(t: &'a str) -> Result<Self, Self::Error> {
        t.parse()
    }
}
impl<'a> TryFrom<&'a String> for HeaderValue {
    type Error = InvalidHeaderValue;
    #[inline]
    fn try_from(s: &'a String) -> Result<Self, Self::Error> {
        Self::from_bytes(s.as_bytes())
    }
}
impl<'a> TryFrom<&'a [u8]> for HeaderValue {
    type Error = InvalidHeaderValue;
    #[inline]
    fn try_from(t: &'a [u8]) -> Result<Self, Self::Error> {
        HeaderValue::from_bytes(t)
    }
}
impl TryFrom<String> for HeaderValue {
    type Error = InvalidHeaderValue;
    #[inline]
    fn try_from(t: String) -> Result<Self, Self::Error> {
        HeaderValue::from_shared(t.into())
    }
}
impl TryFrom<Vec<u8>> for HeaderValue {
    type Error = InvalidHeaderValue;
    #[inline]
    fn try_from(vec: Vec<u8>) -> Result<Self, Self::Error> {
        HeaderValue::from_shared(vec.into())
    }
}
#[cfg(test)]
mod try_from_header_name_tests {
    use super::*;
    use crate::header::name;
    #[test]
    fn it_converts_using_try_from() {
        assert_eq!(
            HeaderValue::try_from(name::UPGRADE).unwrap(),
            HeaderValue::from_bytes(b"upgrade").unwrap()
        );
    }
}
fn is_visible_ascii(b: u8) -> bool {
    b >= 32 && b < 127 || b == b'\t'
}
#[inline]
fn is_valid(b: u8) -> bool {
    b >= 32 && b != 127 || b == b'\t'
}
impl fmt::Debug for InvalidHeaderValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("InvalidHeaderValue").finish()
    }
}
impl fmt::Display for InvalidHeaderValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("failed to parse header value")
    }
}
impl Error for InvalidHeaderValue {}
impl fmt::Display for ToStrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("failed to convert header to a str")
    }
}
impl Error for ToStrError {}
impl PartialEq for HeaderValue {
    #[inline]
    fn eq(&self, other: &HeaderValue) -> bool {
        self.inner == other.inner
    }
}
impl Eq for HeaderValue {}
impl PartialOrd for HeaderValue {
    #[inline]
    fn partial_cmp(&self, other: &HeaderValue) -> Option<cmp::Ordering> {
        self.inner.partial_cmp(&other.inner)
    }
}
impl Ord for HeaderValue {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.inner.cmp(&other.inner)
    }
}
impl PartialEq<str> for HeaderValue {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.inner == other.as_bytes()
    }
}
impl PartialEq<[u8]> for HeaderValue {
    #[inline]
    fn eq(&self, other: &[u8]) -> bool {
        self.inner == other
    }
}
impl PartialOrd<str> for HeaderValue {
    #[inline]
    fn partial_cmp(&self, other: &str) -> Option<cmp::Ordering> {
        (*self.inner).partial_cmp(other.as_bytes())
    }
}
impl PartialOrd<[u8]> for HeaderValue {
    #[inline]
    fn partial_cmp(&self, other: &[u8]) -> Option<cmp::Ordering> {
        (*self.inner).partial_cmp(other)
    }
}
impl PartialEq<HeaderValue> for str {
    #[inline]
    fn eq(&self, other: &HeaderValue) -> bool {
        *other == *self
    }
}
impl PartialEq<HeaderValue> for [u8] {
    #[inline]
    fn eq(&self, other: &HeaderValue) -> bool {
        *other == *self
    }
}
impl PartialOrd<HeaderValue> for str {
    #[inline]
    fn partial_cmp(&self, other: &HeaderValue) -> Option<cmp::Ordering> {
        self.as_bytes().partial_cmp(other.as_bytes())
    }
}
impl PartialOrd<HeaderValue> for [u8] {
    #[inline]
    fn partial_cmp(&self, other: &HeaderValue) -> Option<cmp::Ordering> {
        self.partial_cmp(other.as_bytes())
    }
}
impl PartialEq<String> for HeaderValue {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        *self == &other[..]
    }
}
impl PartialOrd<String> for HeaderValue {
    #[inline]
    fn partial_cmp(&self, other: &String) -> Option<cmp::Ordering> {
        self.inner.partial_cmp(other.as_bytes())
    }
}
impl PartialEq<HeaderValue> for String {
    #[inline]
    fn eq(&self, other: &HeaderValue) -> bool {
        *other == *self
    }
}
impl PartialOrd<HeaderValue> for String {
    #[inline]
    fn partial_cmp(&self, other: &HeaderValue) -> Option<cmp::Ordering> {
        self.as_bytes().partial_cmp(other.as_bytes())
    }
}
impl<'a> PartialEq<HeaderValue> for &'a HeaderValue {
    #[inline]
    fn eq(&self, other: &HeaderValue) -> bool {
        **self == *other
    }
}
impl<'a> PartialOrd<HeaderValue> for &'a HeaderValue {
    #[inline]
    fn partial_cmp(&self, other: &HeaderValue) -> Option<cmp::Ordering> {
        (**self).partial_cmp(other)
    }
}
impl<'a, T: ?Sized> PartialEq<&'a T> for HeaderValue
where
    HeaderValue: PartialEq<T>,
{
    #[inline]
    fn eq(&self, other: &&'a T) -> bool {
        *self == **other
    }
}
impl<'a, T: ?Sized> PartialOrd<&'a T> for HeaderValue
where
    HeaderValue: PartialOrd<T>,
{
    #[inline]
    fn partial_cmp(&self, other: &&'a T) -> Option<cmp::Ordering> {
        self.partial_cmp(*other)
    }
}
impl<'a> PartialEq<HeaderValue> for &'a str {
    #[inline]
    fn eq(&self, other: &HeaderValue) -> bool {
        *other == *self
    }
}
impl<'a> PartialOrd<HeaderValue> for &'a str {
    #[inline]
    fn partial_cmp(&self, other: &HeaderValue) -> Option<cmp::Ordering> {
        self.as_bytes().partial_cmp(other.as_bytes())
    }
}
#[test]
fn test_try_from() {
    HeaderValue::try_from(vec![127]).unwrap_err();
}
#[test]
fn test_debug() {
    let cases = &[
        ("hello", "\"hello\""),
        ("hello \"world\"", "\"hello \\\"world\\\"\""),
        ("\u{7FFF}hello", "\"\\xe7\\xbf\\xbfhello\""),
    ];
    for &(value, expected) in cases {
        let val = HeaderValue::from_bytes(value.as_bytes()).unwrap();
        let actual = format!("{:?}", val);
        assert_eq!(expected, actual);
    }
    let mut sensitive = HeaderValue::from_static("password");
    sensitive.set_sensitive(true);
    assert_eq!("Sensitive", format!("{:?}", sensitive));
}
#[cfg(test)]
mod tests_llm_16_10 {
    use super::*;
    use crate::*;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_10_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = b"hello";
        let rug_fuzz_1 = false;
        let rug_fuzz_2 = b"hello";
        let rug_fuzz_3 = false;
        let header1 = HeaderValue {
            inner: Bytes::from_static(rug_fuzz_0),
            is_sensitive: rug_fuzz_1,
        };
        let header2 = HeaderValue {
            inner: Bytes::from_static(rug_fuzz_2),
            is_sensitive: rug_fuzz_3,
        };
        debug_assert_eq!(header1.eq(& header2), true);
        let _rug_ed_tests_llm_16_10_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_11 {
    use super::*;
    use crate::*;
    #[test]
    fn test_partial_cmp() {
        let _rug_st_tests_llm_16_11_rrrruuuugggg_test_partial_cmp = 0;
        let rug_fuzz_0 = "value1";
        let rug_fuzz_1 = "value2";
        let rug_fuzz_2 = "value3";
        let header_value1 = HeaderValue::from_static(rug_fuzz_0);
        let header_value2 = HeaderValue::from_static(rug_fuzz_1);
        let header_value3 = HeaderValue::from_static(rug_fuzz_2);
        debug_assert_eq!(
            header_value1.partial_cmp(& header_value2), Some(cmp::Ordering::Less)
        );
        debug_assert_eq!(
            header_value2.partial_cmp(& header_value1), Some(cmp::Ordering::Greater)
        );
        debug_assert_eq!(
            header_value2.partial_cmp(& header_value2), Some(cmp::Ordering::Equal)
        );
        debug_assert_eq!(
            header_value1.partial_cmp(& header_value3), Some(cmp::Ordering::Less)
        );
        debug_assert_eq!(
            header_value3.partial_cmp(& header_value1), Some(cmp::Ordering::Greater)
        );
        let _rug_ed_tests_llm_16_11_rrrruuuugggg_test_partial_cmp = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_157 {
    use super::*;
    use crate::*;
    use std::cmp::Ordering;
    #[test]
    fn test_cmp() {
        let _rug_st_tests_llm_16_157_rrrruuuugggg_test_cmp = 0;
        let rug_fuzz_0 = b"value1";
        let rug_fuzz_1 = b"value2";
        let rug_fuzz_2 = false;
        let rug_fuzz_3 = false;
        let inner1: Bytes = Bytes::from_static(rug_fuzz_0);
        let inner2: Bytes = Bytes::from_static(rug_fuzz_1);
        let header_value1 = HeaderValue {
            inner: inner1,
            is_sensitive: rug_fuzz_2,
        };
        let header_value2 = HeaderValue {
            inner: inner2,
            is_sensitive: rug_fuzz_3,
        };
        let result = header_value1.cmp(&header_value2);
        debug_assert_eq!(result, Ordering::Less);
        let _rug_ed_tests_llm_16_157_rrrruuuugggg_test_cmp = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_158 {
    use super::*;
    use crate::*;
    use std::cmp::PartialEq;
    use std::convert::TryFrom;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_158_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "test";
        let rug_fuzz_1 = "test";
        let header_value: HeaderValue = HeaderValue::try_from(rug_fuzz_0).unwrap();
        let other: &str = rug_fuzz_1;
        debug_assert_eq!(header_value.eq(& other), true);
        let _rug_ed_tests_llm_16_158_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_162_llm_16_161 {
    use super::*;
    use crate::*;
    use std::str::FromStr;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_162_llm_16_161_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "hello";
        let rug_fuzz_1 = "hello";
        let rug_fuzz_2 = "world";
        let value = HeaderValue::from_str(rug_fuzz_0).unwrap();
        let other_str: &str = rug_fuzz_1;
        let other = &other_str.as_bytes();
        let result = value.eq(other);
        debug_assert_eq!(result, true);
        let other_str: &str = rug_fuzz_2;
        let other = &other_str.as_bytes();
        let result = value.eq(other);
        debug_assert_eq!(result, false);
        let _rug_ed_tests_llm_16_162_llm_16_161_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_163 {
    use super::*;
    use crate::*;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_163_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "hello";
        let rug_fuzz_1 = false;
        let rug_fuzz_2 = "hello";
        let rug_fuzz_3 = "world";
        let rug_fuzz_4 = false;
        let rug_fuzz_5 = "hello";
        let value = HeaderValue {
            inner: Bytes::from(rug_fuzz_0),
            is_sensitive: rug_fuzz_1,
        };
        debug_assert_eq!(value.eq(rug_fuzz_2), true);
        let value = HeaderValue {
            inner: Bytes::from(rug_fuzz_3),
            is_sensitive: rug_fuzz_4,
        };
        debug_assert_eq!(value.eq(rug_fuzz_5), false);
        let _rug_ed_tests_llm_16_163_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_164 {
    use super::*;
    use crate::*;
    use crate::header::value::InvalidHeaderValue;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_164_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "hello";
        let rug_fuzz_1 = "hello";
        let rug_fuzz_2 = "world";
        let value1 = HeaderValue::from_static(rug_fuzz_0);
        let value2 = HeaderValue::from_static(rug_fuzz_1);
        debug_assert_eq!(value1.eq(& value2), true);
        let value3 = HeaderValue::from_static(rug_fuzz_2);
        debug_assert_eq!(value1.eq(& value3), false);
        let _rug_ed_tests_llm_16_164_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_165 {
    use crate::header::value::HeaderValue;
    use std::cmp::Ordering;
    #[test]
    fn test_partial_cmp() {
        let _rug_st_tests_llm_16_165_rrrruuuugggg_test_partial_cmp = 0;
        let rug_fuzz_0 = "value1";
        let rug_fuzz_1 = "value2";
        let rug_fuzz_2 = "value1";
        let header_value1 = HeaderValue::from_static(rug_fuzz_0);
        let header_value2 = HeaderValue::from_static(rug_fuzz_1);
        let header_value3 = HeaderValue::from_static(rug_fuzz_2);
        let result = header_value1.partial_cmp(&&header_value2);
        debug_assert_eq!(result, Some(Ordering::Less));
        let result = header_value2.partial_cmp(&&header_value1);
        debug_assert_eq!(result, Some(Ordering::Greater));
        let result = header_value1.partial_cmp(&&header_value3);
        debug_assert_eq!(result, Some(Ordering::Equal));
        let _rug_ed_tests_llm_16_165_rrrruuuugggg_test_partial_cmp = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_169 {
    use std::cmp;
    use std::convert::TryFrom;
    use bytes::Bytes;
    use crate::header::value::HeaderValue;
    #[test]
    fn test_partial_cmp() {
        let _rug_st_tests_llm_16_169_rrrruuuugggg_test_partial_cmp = 0;
        let rug_fuzz_0 = "value1";
        let rug_fuzz_1 = "value2";
        let rug_fuzz_2 = "value3";
        let value1 = HeaderValue::try_from(rug_fuzz_0).unwrap();
        let value2 = HeaderValue::try_from(rug_fuzz_1).unwrap();
        let value3 = HeaderValue::try_from(rug_fuzz_2).unwrap();
        debug_assert_eq!(value1.partial_cmp(& value2), Some(cmp::Ordering::Less));
        debug_assert_eq!(value2.partial_cmp(& value2), Some(cmp::Ordering::Equal));
        debug_assert_eq!(value3.partial_cmp(& value2), Some(cmp::Ordering::Greater));
        let _rug_ed_tests_llm_16_169_rrrruuuugggg_test_partial_cmp = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_170 {
    use super::*;
    use crate::*;
    use std::cmp::Ordering;
    #[test]
    fn test_partial_cmp() {
        let _rug_st_tests_llm_16_170_rrrruuuugggg_test_partial_cmp = 0;
        let rug_fuzz_0 = "hello";
        let rug_fuzz_1 = "hello";
        let value = HeaderValue::from_static(rug_fuzz_0);
        debug_assert_eq!(value.partial_cmp(rug_fuzz_1), Some(Ordering::Equal));
        let _rug_ed_tests_llm_16_170_rrrruuuugggg_test_partial_cmp = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_171 {
    use super::*;
    use crate::*;
    use bytes::Bytes;
    use std::str::FromStr;
    use std::cmp::Ordering;
    #[test]
    fn test_partial_cmp() {
        let _rug_st_tests_llm_16_171_rrrruuuugggg_test_partial_cmp = 0;
        let rug_fuzz_0 = "value1";
        let rug_fuzz_1 = "value2";
        let header1 = HeaderValue::from_static(rug_fuzz_0);
        let header2 = HeaderValue::from_static(rug_fuzz_1);
        let result1 = header1.partial_cmp(&header2);
        let result2 = header2.partial_cmp(&header1);
        debug_assert_eq!(result1, Some(Ordering::Less));
        debug_assert_eq!(result2, Some(Ordering::Greater));
        let _rug_ed_tests_llm_16_171_rrrruuuugggg_test_partial_cmp = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_172 {
    use std::convert::TryFrom;
    use crate::header::HeaderValue;
    #[test]
    fn test_as_ref() {
        let _rug_st_tests_llm_16_172_rrrruuuugggg_test_as_ref = 0;
        let rug_fuzz_0 = "test";
        let value = HeaderValue::try_from(rug_fuzz_0).unwrap();
        let result = value.as_ref();
        debug_assert_eq!(result, b"test");
        let _rug_ed_tests_llm_16_172_rrrruuuugggg_test_as_ref = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_174_llm_16_173 {
    use bytes::Bytes;
    use crate::header::value::HeaderValue;
    use std::convert::TryFrom;
    use std::str::FromStr;
    #[test]
    fn test_from() {
        let _rug_st_tests_llm_16_174_llm_16_173_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = "example header";
        let header_value = HeaderValue::from_str(rug_fuzz_0).unwrap();
        let result: HeaderValue = TryFrom::try_from(&header_value).unwrap();
        let expected: HeaderValue = header_value.clone();
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_174_llm_16_173_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_180 {
    use crate::header::value::HeaderValue;
    use std::convert::TryFrom;
    #[test]
    fn test_from() {
        let _rug_st_tests_llm_16_180_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = 42;
        let rug_fuzz_1 = 999_999_999_999_999_999;
        let num: i32 = rug_fuzz_0;
        let result = <HeaderValue as std::convert::From<i32>>::from(num);
        debug_assert_eq!(result, HeaderValue::from_static("42"));
        let num: i64 = rug_fuzz_1;
        let result = <HeaderValue as std::convert::From<i64>>::from(num);
        debug_assert_eq!(result, HeaderValue::from_static("999999999999999999"));
        let _rug_ed_tests_llm_16_180_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_184_llm_16_183 {
    use crate::header::value::{HeaderValue, InvalidHeaderValue};
    use std::convert::TryFrom;
    use std::str::FromStr;
    use bytes::Bytes;
    use std::mem;
    #[test]
    fn test_from() {
        let _rug_st_tests_llm_16_184_llm_16_183_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = 100;
        let num: isize = rug_fuzz_0;
        let header: HeaderValue = HeaderValue::from(num);
        debug_assert_eq!(header.to_str().unwrap(), "100");
        let _rug_ed_tests_llm_16_184_llm_16_183_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_185 {
    use crate::header::value::HeaderValue;
    use std::convert::From;
    use bytes::Bytes;
    use bytes::BytesMut;
    use std::mem;
    use std::convert::TryFrom;
    use std::fmt;
    use std::hash::Hash;
    use std::str::FromStr;
    #[test]
    fn test_from() {
        let _rug_st_tests_llm_16_185_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = 16;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 32;
        let rug_fuzz_3 = 999_999_999_999_999_999;
        let rug_fuzz_4 = 32;
        let rug_fuzz_5 = false;
        let num: u16 = rug_fuzz_0;
        let actual: HeaderValue = <HeaderValue as std::convert::From<u16>>::from(num);
        let mut buf = if mem::size_of::<BytesMut>() - rug_fuzz_1 < rug_fuzz_2 {
            if num as u64 > rug_fuzz_3 {
                BytesMut::with_capacity(rug_fuzz_4)
            } else {
                BytesMut::new()
            }
        } else {
            BytesMut::new()
        };
        let _ = ::itoa::fmt(&mut buf, num);
        let expected: HeaderValue = HeaderValue {
            inner: buf.freeze(),
            is_sensitive: rug_fuzz_5,
        };
        debug_assert_eq!(actual, expected);
        let _rug_ed_tests_llm_16_185_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_186 {
    use super::*;
    use crate::*;
    use crate::header::HeaderValue;
    #[test]
    fn test_from() {
        let _rug_st_tests_llm_16_186_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = 123;
        let num: u32 = rug_fuzz_0;
        let expected = HeaderValue::from(num);
        let result = <header::value::HeaderValue as std::convert::From<u32>>::from(num);
        debug_assert_eq!(expected, result);
        let _rug_ed_tests_llm_16_186_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_187 {
    use super::*;
    use crate::*;
    use bytes::{Bytes, BytesMut};
    use std::convert::TryFrom;
    use std::str::FromStr;
    use std::mem;
    use ::itoa::fmt;
    use std::cmp;
    use std::hash::*;
    use std::clone::*;
    use std::fmt::*;
    #[test]
    fn test_from() {
        let _rug_st_tests_llm_16_187_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = 1234;
        let rug_fuzz_1 = b"1234";
        let rug_fuzz_2 = false;
        let rug_fuzz_3 = 999999999999999999;
        let rug_fuzz_4 = 64;
        let rug_fuzz_5 = false;
        let num1: u64 = rug_fuzz_0;
        let expected1 = HeaderValue {
            inner: Bytes::from_static(rug_fuzz_1),
            is_sensitive: rug_fuzz_2,
        };
        debug_assert_eq!(
            < HeaderValue as std::convert::From < u64 > > ::from(num1), expected1
        );
        let num2: u64 = rug_fuzz_3;
        let expected2 = HeaderValue {
            inner: BytesMut::with_capacity(rug_fuzz_4).freeze(),
            is_sensitive: rug_fuzz_5,
        };
        debug_assert_eq!(
            < HeaderValue as std::convert::From < u64 > > ::from(num2), expected2
        );
        let _rug_ed_tests_llm_16_187_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_188 {
    use super::*;
    use crate::*;
    use bytes::{Bytes, BytesMut};
    use std::mem;
    use std::str::FromStr;
    use std::convert::TryFrom;
    use std::fmt::Debug;
    use std::hash::Hash;
    use std::cmp::{Eq, Ord, PartialEq, PartialOrd};
    use std::convert::AsRef;
    #[test]
    fn test_from_func() {
        let _rug_st_tests_llm_16_188_rrrruuuugggg_test_from_func = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = b"10";
        let rug_fuzz_2 = false;
        let rug_fuzz_3 = 100_000;
        let rug_fuzz_4 = b"100000";
        let rug_fuzz_5 = false;
        let rug_fuzz_6 = 1_000_000_000_000_000;
        let rug_fuzz_7 = 19;
        let rug_fuzz_8 = false;
        let num: usize = rug_fuzz_0;
        let value = <HeaderValue as std::convert::From<usize>>::from(num);
        let expected = HeaderValue {
            inner: Bytes::from_static(rug_fuzz_1),
            is_sensitive: rug_fuzz_2,
        };
        debug_assert_eq!(value, expected);
        let num: usize = rug_fuzz_3;
        let value = <HeaderValue as std::convert::From<usize>>::from(num);
        let expected = HeaderValue {
            inner: Bytes::from_static(rug_fuzz_4),
            is_sensitive: rug_fuzz_5,
        };
        debug_assert_eq!(value, expected);
        let num: usize = rug_fuzz_6;
        let value = <HeaderValue as std::convert::From<usize>>::from(num);
        let expected = HeaderValue {
            inner: BytesMut::with_capacity(rug_fuzz_7).freeze(),
            is_sensitive: rug_fuzz_8,
        };
        debug_assert_eq!(value, expected);
        let _rug_ed_tests_llm_16_188_rrrruuuugggg_test_from_func = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_193 {
    use std::convert::TryFrom;
    use crate::header::value::HeaderValue;
    #[test]
    fn test_try_from() {
        let _rug_st_tests_llm_16_193_rrrruuuugggg_test_try_from = 0;
        let rug_fuzz_0 = "example";
        let rug_fuzz_1 = "invalid";
        let input: &str = rug_fuzz_0;
        let result = <HeaderValue as TryFrom<&str>>::try_from(input);
        debug_assert!(result.is_ok());
        let input: &str = rug_fuzz_1;
        let result = <HeaderValue as TryFrom<&str>>::try_from(input);
        debug_assert!(result.is_err());
        let _rug_ed_tests_llm_16_193_rrrruuuugggg_test_try_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_196 {
    use std::convert::TryFrom;
    use crate::header::value::HeaderValue;
    #[test]
    fn test_try_from() {
        let _rug_st_tests_llm_16_196_rrrruuuugggg_test_try_from = 0;
        let rug_fuzz_0 = b't';
        let vec: Vec<u8> = vec![rug_fuzz_0, b'e', b's', b't'];
        let result: Result<HeaderValue, _> = HeaderValue::try_from(vec);
        debug_assert!(result.is_ok());
        let _rug_ed_tests_llm_16_196_rrrruuuugggg_test_try_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_197 {
    use crate::header::value::HeaderValue;
    use std::str::FromStr;
    #[test]
    fn test_from_str() {
        let _rug_st_tests_llm_16_197_rrrruuuugggg_test_from_str = 0;
        let rug_fuzz_0 = "example_value";
        let s = rug_fuzz_0;
        let result = <HeaderValue as FromStr>::from_str(s);
        debug_assert!(result.is_ok());
        debug_assert_eq!(result.unwrap().as_bytes(), s.as_bytes());
        let _rug_ed_tests_llm_16_197_rrrruuuugggg_test_from_str = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_432 {
    use crate::header::value::HeaderValue;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_432_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = b"test";
        let rug_fuzz_1 = false;
        let rug_fuzz_2 = b"test";
        let rug_fuzz_3 = false;
        let rug_fuzz_4 = b"abc";
        let rug_fuzz_5 = false;
        let header1 = HeaderValue {
            inner: bytes::Bytes::from_static(rug_fuzz_0),
            is_sensitive: rug_fuzz_1,
        };
        let header2 = HeaderValue {
            inner: bytes::Bytes::from_static(rug_fuzz_2),
            is_sensitive: rug_fuzz_3,
        };
        let header3 = HeaderValue {
            inner: bytes::Bytes::from_static(rug_fuzz_4),
            is_sensitive: rug_fuzz_5,
        };
        debug_assert_eq!(header1.eq(& header2), true);
        debug_assert_eq!(header1.eq(& header3), false);
        let _rug_ed_tests_llm_16_432_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_433 {
    use super::*;
    use crate::*;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_433_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "hello";
        let rug_fuzz_1 = "hello";
        let rug_fuzz_2 = "world";
        let header_value1 = HeaderValue::from_static(rug_fuzz_0);
        let header_value2 = HeaderValue::from_static(rug_fuzz_1);
        let header_value3 = HeaderValue::from_static(rug_fuzz_2);
        debug_assert_eq!(header_value1.eq(& header_value2), true);
        debug_assert_eq!(header_value1.eq(& header_value3), false);
        let _rug_ed_tests_llm_16_433_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_434 {
    use super::*;
    use crate::*;
    use bytes::Bytes;
    use std::convert::TryFrom;
    use std::str::FromStr;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_434_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = b"value1";
        let rug_fuzz_1 = false;
        let rug_fuzz_2 = b"value1";
        let rug_fuzz_3 = false;
        let rug_fuzz_4 = b"value2";
        let rug_fuzz_5 = false;
        let header_value1 = HeaderValue {
            inner: Bytes::from_static(rug_fuzz_0),
            is_sensitive: rug_fuzz_1,
        };
        let header_value2 = HeaderValue {
            inner: Bytes::from_static(rug_fuzz_2),
            is_sensitive: rug_fuzz_3,
        };
        let header_value3 = HeaderValue {
            inner: Bytes::from_static(rug_fuzz_4),
            is_sensitive: rug_fuzz_5,
        };
        debug_assert_eq!(header_value1.eq(& header_value2), true);
        debug_assert_eq!(header_value1.eq(& header_value3), false);
        let _rug_ed_tests_llm_16_434_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_435 {
    use super::*;
    use crate::*;
    use crate::header::InvalidHeaderValue;
    use bytes::Bytes;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_435_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = b"hello";
        let rug_fuzz_1 = b"hello";
        let rug_fuzz_2 = false;
        let rug_fuzz_3 = false;
        let value1 = bytes::Bytes::from_static(rug_fuzz_0);
        let value2 = bytes::Bytes::from_static(rug_fuzz_1);
        let header_value1 = header::value::HeaderValue {
            inner: value1,
            is_sensitive: rug_fuzz_2,
        };
        let header_value2 = header::value::HeaderValue {
            inner: value2,
            is_sensitive: rug_fuzz_3,
        };
        debug_assert_eq!(header_value1.eq(& header_value2), true);
        let _rug_ed_tests_llm_16_435_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_436 {
    use super::*;
    use crate::*;
    use bytes::Bytes;
    use std::cmp::Ordering;
    #[test]
    fn test_partial_cmp() {
        let _rug_st_tests_llm_16_436_rrrruuuugggg_test_partial_cmp = 0;
        let rug_fuzz_0 = b"abc";
        let rug_fuzz_1 = false;
        let rug_fuzz_2 = b"def";
        let rug_fuzz_3 = false;
        let rug_fuzz_4 = b"abc";
        let rug_fuzz_5 = false;
        let value1 = HeaderValue {
            inner: Bytes::from_static(rug_fuzz_0),
            is_sensitive: rug_fuzz_1,
        };
        let value2 = HeaderValue {
            inner: Bytes::from_static(rug_fuzz_2),
            is_sensitive: rug_fuzz_3,
        };
        let value3 = HeaderValue {
            inner: Bytes::from_static(rug_fuzz_4),
            is_sensitive: rug_fuzz_5,
        };
        debug_assert_eq!(value1.partial_cmp(& value1), Some(Ordering::Equal));
        debug_assert_eq!(value1.partial_cmp(& value2), Some(Ordering::Less));
        debug_assert_eq!(value2.partial_cmp(& value1), Some(Ordering::Greater));
        debug_assert_eq!(value1.partial_cmp(& value3), Some(Ordering::Equal));
        let _rug_ed_tests_llm_16_436_rrrruuuugggg_test_partial_cmp = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_437 {
    use super::*;
    use crate::*;
    use std::cmp::Ordering;
    #[test]
    fn test_partial_cmp() {
        let _rug_st_tests_llm_16_437_rrrruuuugggg_test_partial_cmp = 0;
        let rug_fuzz_0 = b"value1";
        let rug_fuzz_1 = false;
        let rug_fuzz_2 = b"value2";
        let rug_fuzz_3 = false;
        let rug_fuzz_4 = b"value3";
        let rug_fuzz_5 = false;
        let header_value1: HeaderValue = HeaderValue {
            inner: Bytes::from_static(rug_fuzz_0),
            is_sensitive: rug_fuzz_1,
        };
        let header_value2: HeaderValue = HeaderValue {
            inner: Bytes::from_static(rug_fuzz_2),
            is_sensitive: rug_fuzz_3,
        };
        let header_value3: HeaderValue = HeaderValue {
            inner: Bytes::from_static(rug_fuzz_4),
            is_sensitive: rug_fuzz_5,
        };
        let result1: Option<Ordering> = header_value1.partial_cmp(&header_value2);
        let result2: Option<Ordering> = header_value2.partial_cmp(&header_value3);
        debug_assert_eq!(result1, Some(Ordering::Less));
        debug_assert_eq!(result2, None);
        let _rug_ed_tests_llm_16_437_rrrruuuugggg_test_partial_cmp = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_438 {
    use super::*;
    use crate::*;
    use bytes::Bytes;
    use std::cmp::Ordering;
    #[test]
    fn test_partial_cmp() {
        let _rug_st_tests_llm_16_438_rrrruuuugggg_test_partial_cmp = 0;
        let rug_fuzz_0 = "value1";
        let rug_fuzz_1 = false;
        let rug_fuzz_2 = "value2";
        let rug_fuzz_3 = false;
        let rug_fuzz_4 = "value3";
        let rug_fuzz_5 = false;
        let value1 = HeaderValue {
            inner: Bytes::from(rug_fuzz_0),
            is_sensitive: rug_fuzz_1,
        };
        let value2 = HeaderValue {
            inner: Bytes::from(rug_fuzz_2),
            is_sensitive: rug_fuzz_3,
        };
        let value3 = HeaderValue {
            inner: Bytes::from(rug_fuzz_4),
            is_sensitive: rug_fuzz_5,
        };
        debug_assert_eq!(value1.partial_cmp(& value2), Some(Ordering::Less));
        debug_assert_eq!(value2.partial_cmp(& value1), Some(Ordering::Greater));
        debug_assert_eq!(value2.partial_cmp(& value3), Some(Ordering::Less));
        debug_assert_eq!(value3.partial_cmp(& value2), Some(Ordering::Greater));
        debug_assert_eq!(value1.partial_cmp(& value3), Some(Ordering::Less));
        debug_assert_eq!(value3.partial_cmp(& value1), Some(Ordering::Greater));
        let _rug_ed_tests_llm_16_438_rrrruuuugggg_test_partial_cmp = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_439 {
    use super::*;
    use crate::*;
    #[test]
    fn test_partial_cmp() {
        let _rug_st_tests_llm_16_439_rrrruuuugggg_test_partial_cmp = 0;
        let rug_fuzz_0 = "abc";
        let rug_fuzz_1 = "def";
        let rug_fuzz_2 = "abc";
        let rug_fuzz_3 = "ghi";
        let value1 = HeaderValue::from_static(rug_fuzz_0);
        let value2 = HeaderValue::from_static(rug_fuzz_1);
        let value3 = HeaderValue::from_static(rug_fuzz_2);
        let value4 = HeaderValue::from_static(rug_fuzz_3);
        debug_assert_eq!(value1.partial_cmp(& value2), Some(cmp::Ordering::Less));
        debug_assert_eq!(value2.partial_cmp(& value1), Some(cmp::Ordering::Greater));
        debug_assert_eq!(value1.partial_cmp(& value3), Some(cmp::Ordering::Equal));
        debug_assert_eq!(value1.partial_cmp(& value4), Some(cmp::Ordering::Less));
        debug_assert_eq!(value4.partial_cmp(& value1), Some(cmp::Ordering::Greater));
        let _rug_ed_tests_llm_16_439_rrrruuuugggg_test_partial_cmp = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_440 {
    use super::*;
    use crate::*;
    use crate::header::HeaderValue;
    #[test]
    fn test_as_bytes() {
        let _rug_st_tests_llm_16_440_rrrruuuugggg_test_as_bytes = 0;
        let rug_fuzz_0 = "hello";
        let val = HeaderValue::from_static(rug_fuzz_0);
        debug_assert_eq!(val.as_bytes(), b"hello");
        let _rug_ed_tests_llm_16_440_rrrruuuugggg_test_as_bytes = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_441 {
    use crate::header::HeaderValue;
    use crate::header::InvalidHeaderValue;
    #[test]
    fn test_from_bytes_valid() {
        let _rug_st_tests_llm_16_441_rrrruuuugggg_test_from_bytes_valid = 0;
        let rug_fuzz_0 = b"hello\xfa";
        let val = HeaderValue::from_bytes(rug_fuzz_0).unwrap();
        debug_assert_eq!(val, & b"hello\xfa"[..]);
        let _rug_ed_tests_llm_16_441_rrrruuuugggg_test_from_bytes_valid = 0;
    }
    #[test]
    fn test_from_bytes_invalid() {
        let _rug_st_tests_llm_16_441_rrrruuuugggg_test_from_bytes_invalid = 0;
        let rug_fuzz_0 = b"\n";
        let val = HeaderValue::from_bytes(rug_fuzz_0);
        debug_assert!(val.is_err());
        let _rug_ed_tests_llm_16_441_rrrruuuugggg_test_from_bytes_invalid = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_442 {
    use crate::header::value::HeaderValue;
    use crate::header::value::InvalidHeaderValue;
    use bytes::Bytes;
    #[test]
    fn test_from_maybe_shared() {
        let src: &[u8] = &[b't', b'e', b's', b't'];
        let result = HeaderValue::from_maybe_shared(src);
        assert!(result.is_ok());
        let header_value = result.unwrap();
        assert_eq!(header_value.as_bytes(), & Bytes::from(src) [..]);
    }
}
#[cfg(test)]
mod tests_llm_16_443 {
    use super::*;
    use crate::*;
    #[test]
    fn test_from_maybe_shared_unchecked_with_valid_bytes() {
        let _rug_st_tests_llm_16_443_rrrruuuugggg_test_from_maybe_shared_unchecked_with_valid_bytes = 0;
        let rug_fuzz_0 = b"hello";
        let src: &'static [u8] = rug_fuzz_0;
        let result = unsafe { HeaderValue::from_maybe_shared_unchecked(src) };
        debug_assert_eq!(result.as_bytes(), src);
        let _rug_ed_tests_llm_16_443_rrrruuuugggg_test_from_maybe_shared_unchecked_with_valid_bytes = 0;
    }
    #[test]
    #[should_panic]
    fn test_from_maybe_shared_unchecked_with_invalid_bytes() {
        let _rug_st_tests_llm_16_443_rrrruuuugggg_test_from_maybe_shared_unchecked_with_invalid_bytes = 0;
        let rug_fuzz_0 = b"hello\xFA";
        let src: &'static [u8] = rug_fuzz_0;
        unsafe {
            HeaderValue::from_maybe_shared_unchecked(src);
        }
        let _rug_ed_tests_llm_16_443_rrrruuuugggg_test_from_maybe_shared_unchecked_with_invalid_bytes = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_445 {
    use crate::header::value::{HeaderValue, InvalidHeaderValue};
    use bytes::{Bytes, BytesMut, BufMut};
    #[test]
    fn test_from_shared() {
        let _rug_st_tests_llm_16_445_rrrruuuugggg_test_from_shared = 0;
        let rug_fuzz_0 = "test string";
        let src_bytes: Bytes = BytesMut::from(rug_fuzz_0).freeze().into();
        let result = HeaderValue::from_shared(src_bytes);
        debug_assert!(result.is_ok());
        let _rug_ed_tests_llm_16_445_rrrruuuugggg_test_from_shared = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_446 {
    use super::*;
    use crate::*;
    #[test]
    fn test_from_static() {
        let _rug_st_tests_llm_16_446_rrrruuuugggg_test_from_static = 0;
        let rug_fuzz_0 = "hello";
        let val = HeaderValue::from_static(rug_fuzz_0);
        debug_assert_eq!(val, "hello");
        let _rug_ed_tests_llm_16_446_rrrruuuugggg_test_from_static = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_447 {
    use crate::header::value::HeaderValue;
    use crate::header::value::InvalidHeaderValue;
    use std::convert::TryFrom;
    use bytes::Bytes;
    #[test]
    fn test_valid_header_value() {
        let _rug_st_tests_llm_16_447_rrrruuuugggg_test_valid_header_value = 0;
        let rug_fuzz_0 = "hello";
        let val = HeaderValue::from_str(rug_fuzz_0).unwrap();
        debug_assert_eq!(val, "hello");
        let _rug_ed_tests_llm_16_447_rrrruuuugggg_test_valid_header_value = 0;
    }
    #[test]
    #[should_panic]
    fn test_invalid_header_value() {
        let _rug_st_tests_llm_16_447_rrrruuuugggg_test_invalid_header_value = 0;
        let rug_fuzz_0 = "\n";
        let val = HeaderValue::from_str(rug_fuzz_0).unwrap();
        let _rug_ed_tests_llm_16_447_rrrruuuugggg_test_invalid_header_value = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_448 {
    use super::*;
    use crate::*;
    use bytes::Bytes;
    #[test]
    fn test_is_empty() {
        let _rug_st_tests_llm_16_448_rrrruuuugggg_test_is_empty = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = "hello";
        let val = HeaderValue::from_static(rug_fuzz_0);
        debug_assert!(val.is_empty());
        let val = HeaderValue::from_static(rug_fuzz_1);
        debug_assert!(! val.is_empty());
        let _rug_ed_tests_llm_16_448_rrrruuuugggg_test_is_empty = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_449 {
    use super::*;
    use crate::*;
    use crate::header::HeaderValue;
    #[test]
    fn test_is_sensitive() {
        let _rug_st_tests_llm_16_449_rrrruuuugggg_test_is_sensitive = 0;
        let rug_fuzz_0 = "my secret";
        let rug_fuzz_1 = true;
        let rug_fuzz_2 = false;
        let mut val = HeaderValue::from_static(rug_fuzz_0);
        val.set_sensitive(rug_fuzz_1);
        debug_assert!(val.is_sensitive());
        val.set_sensitive(rug_fuzz_2);
        debug_assert!(! val.is_sensitive());
        let _rug_ed_tests_llm_16_449_rrrruuuugggg_test_is_sensitive = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_450 {
    use super::*;
    use crate::*;
    use bytes::Bytes;
    #[test]
    fn test_len() {
        let _rug_st_tests_llm_16_450_rrrruuuugggg_test_len = 0;
        let rug_fuzz_0 = "hello";
        let val = HeaderValue::from_static(rug_fuzz_0);
        debug_assert_eq!(val.len(), 5);
        let _rug_ed_tests_llm_16_450_rrrruuuugggg_test_len = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_451 {
    use super::*;
    use crate::*;
    use bytes::Bytes;
    use std::str::FromStr;
    use std::convert::From;
    #[test]
    fn test_set_sensitive() {
        let _rug_st_tests_llm_16_451_rrrruuuugggg_test_set_sensitive = 0;
        let rug_fuzz_0 = "my secret";
        let rug_fuzz_1 = true;
        let rug_fuzz_2 = false;
        let mut val = HeaderValue::from_static(rug_fuzz_0);
        val.set_sensitive(rug_fuzz_1);
        debug_assert!(val.is_sensitive());
        val.set_sensitive(rug_fuzz_2);
        debug_assert!(! val.is_sensitive());
        let _rug_ed_tests_llm_16_451_rrrruuuugggg_test_set_sensitive = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_452 {
    use super::*;
    use crate::*;
    #[test]
    fn test_to_str() {
        let _rug_st_tests_llm_16_452_rrrruuuugggg_test_to_str = 0;
        let rug_fuzz_0 = "hello";
        let val = HeaderValue::from_static(rug_fuzz_0);
        debug_assert_eq!(val.to_str().unwrap(), "hello");
        let _rug_ed_tests_llm_16_452_rrrruuuugggg_test_to_str = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_457 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_visible_ascii() {
        let _rug_st_tests_llm_16_457_rrrruuuugggg_test_is_visible_ascii = 0;
        let rug_fuzz_0 = 32;
        let rug_fuzz_1 = 65;
        let rug_fuzz_2 = 126;
        let rug_fuzz_3 = 9;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 31;
        let rug_fuzz_6 = 127;
        debug_assert_eq!(is_visible_ascii(rug_fuzz_0), true);
        debug_assert_eq!(is_visible_ascii(rug_fuzz_1), true);
        debug_assert_eq!(is_visible_ascii(rug_fuzz_2), true);
        debug_assert_eq!(is_visible_ascii(rug_fuzz_3), true);
        debug_assert_eq!(is_visible_ascii(rug_fuzz_4), false);
        debug_assert_eq!(is_visible_ascii(rug_fuzz_5), false);
        debug_assert_eq!(is_visible_ascii(rug_fuzz_6), false);
        let _rug_ed_tests_llm_16_457_rrrruuuugggg_test_is_visible_ascii = 0;
    }
}
#[cfg(test)]
mod tests_rug_117 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_117_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u8 = rug_fuzz_0;
        crate::header::value::is_valid(p0);
        let _rug_ed_tests_rug_117_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_118 {
    use super::*;
    use crate::header::{HeaderName, HeaderValue, ACCEPT};
    #[test]
    fn test_from_name() {
        let _rug_st_tests_rug_118_rrrruuuugggg_test_from_name = 0;
        let mut p0: HeaderName = ACCEPT;
        let val = HeaderValue::from_name(p0);
        debug_assert_eq!(val, HeaderValue::from_bytes(b"accept").unwrap());
        let _rug_ed_tests_rug_118_rrrruuuugggg_test_from_name = 0;
    }
}
#[cfg(test)]
mod tests_rug_120 {
    use super::*;
    use crate::header::{HeaderValue, HeaderName};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_120_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "Content-Type";
        let mut p0: HeaderName = HeaderName::from_static(rug_fuzz_0);
        let res: HeaderValue = <HeaderValue as std::convert::From<HeaderName>>::from(p0);
        let _rug_ed_tests_rug_120_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_122 {
    use super::*;
    use crate::header::value::HeaderValue;
    #[test]
    fn test_header_value_from_i64() {
        let _rug_st_tests_rug_122_rrrruuuugggg_test_header_value_from_i64 = 0;
        let rug_fuzz_0 = 42;
        let p0: i64 = rug_fuzz_0;
        <HeaderValue as std::convert::From<i64>>::from(p0);
        let _rug_ed_tests_rug_122_rrrruuuugggg_test_header_value_from_i64 = 0;
    }
}
#[cfg(test)]
mod tests_rug_124 {
    use super::*;
    use crate::header::HeaderValue;
    use std::convert::TryFrom;
    #[test]
    fn test_try_from() {
        let _rug_st_tests_rug_124_rrrruuuugggg_test_try_from = 0;
        let rug_fuzz_0 = b"example_value";
        let p0: &[u8] = rug_fuzz_0;
        <HeaderValue as TryFrom<&[u8]>>::try_from(p0);
        let _rug_ed_tests_rug_124_rrrruuuugggg_test_try_from = 0;
    }
}
#[cfg(test)]
mod tests_rug_126 {
    use super::*;
    use crate::header::HeaderValue;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_126_rrrruuuugggg_sample = 0;
        let rug_fuzz_0 = "hello";
        #[cfg(test)]
        mod tests_rug_126_prepare {
            use crate::header::HeaderValue;
            #[test]
            fn sample() {
                let _rug_st_tests_rug_126_prepare_rrrruuuugggg_sample = 0;
                let rug_fuzz_0 = 0;
                let rug_fuzz_1 = "hello";
                let rug_fuzz_2 = 0;
                let _rug_st_tests_rug_126_rrrruuuugggg_sample = rug_fuzz_0;
                let rug_fuzz_0 = rug_fuzz_1;
                let mut v12: HeaderValue = HeaderValue::from_static(rug_fuzz_0);
                let _rug_ed_tests_rug_126_rrrruuuugggg_sample = rug_fuzz_2;
                let _rug_ed_tests_rug_126_prepare_rrrruuuugggg_sample = 0;
            }
        }
        let mut p0: HeaderValue = HeaderValue::from_static("hello");
        let p1: &[u8] = b"world";
        p0.eq(p1);
        let _rug_ed_tests_rug_126_rrrruuuugggg_sample = 0;
    }
}
