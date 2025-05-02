use std::convert::TryFrom;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::str::FromStr;
use bytes::Bytes;
use super::{ErrorKind, InvalidUri};
use crate::byte_str::ByteStr;
/// Represents the scheme component of a URI
#[derive(Clone)]
pub struct Scheme {
    pub(super) inner: Scheme2,
}
#[derive(Clone, Debug)]
pub(super) enum Scheme2<T = Box<ByteStr>> {
    None,
    Standard(Protocol),
    Other(T),
}
#[derive(Copy, Clone, Debug)]
pub(super) enum Protocol {
    Http,
    Https,
}
impl Scheme {
    /// HTTP protocol scheme
    pub const HTTP: Scheme = Scheme {
        inner: Scheme2::Standard(Protocol::Http),
    };
    /// HTTP protocol over TLS.
    pub const HTTPS: Scheme = Scheme {
        inner: Scheme2::Standard(Protocol::Https),
    };
    pub(super) fn empty() -> Self {
        Scheme { inner: Scheme2::None }
    }
    /// Return a str representation of the scheme
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::uri::*;
    /// let scheme: Scheme = "http".parse().unwrap();
    /// assert_eq!(scheme.as_str(), "http");
    /// ```
    #[inline]
    pub fn as_str(&self) -> &str {
        use self::Protocol::*;
        use self::Scheme2::*;
        match self.inner {
            Standard(Http) => "http",
            Standard(Https) => "https",
            Other(ref v) => &v[..],
            None => unreachable!(),
        }
    }
}
impl<'a> TryFrom<&'a [u8]> for Scheme {
    type Error = InvalidUri;
    #[inline]
    fn try_from(s: &'a [u8]) -> Result<Self, Self::Error> {
        use self::Scheme2::*;
        match Scheme2::parse_exact(s)? {
            None => Err(ErrorKind::InvalidScheme.into()),
            Standard(p) => Ok(Standard(p).into()),
            Other(_) => {
                let bytes = Bytes::copy_from_slice(s);
                let string = unsafe { ByteStr::from_utf8_unchecked(bytes) };
                Ok(Other(Box::new(string)).into())
            }
        }
    }
}
impl<'a> TryFrom<&'a str> for Scheme {
    type Error = InvalidUri;
    #[inline]
    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        TryFrom::try_from(s.as_bytes())
    }
}
impl FromStr for Scheme {
    type Err = InvalidUri;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        TryFrom::try_from(s)
    }
}
impl fmt::Debug for Scheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}
impl fmt::Display for Scheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
impl AsRef<str> for Scheme {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
impl PartialEq for Scheme {
    fn eq(&self, other: &Scheme) -> bool {
        use self::Protocol::*;
        use self::Scheme2::*;
        match (&self.inner, &other.inner) {
            (&Standard(Http), &Standard(Http)) => true,
            (&Standard(Https), &Standard(Https)) => true,
            (&Other(ref a), &Other(ref b)) => a.eq_ignore_ascii_case(b),
            (&None, _) | (_, &None) => unreachable!(),
            _ => false,
        }
    }
}
impl Eq for Scheme {}
/// Case-insensitive equality
///
/// # Examples
///
/// ```
/// # use http::uri::Scheme;
/// let scheme: Scheme = "HTTP".parse().unwrap();
/// assert_eq!(scheme, *"http");
/// ```
impl PartialEq<str> for Scheme {
    fn eq(&self, other: &str) -> bool {
        self.as_str().eq_ignore_ascii_case(other)
    }
}
/// Case-insensitive equality
impl PartialEq<Scheme> for str {
    fn eq(&self, other: &Scheme) -> bool {
        other == self
    }
}
/// Case-insensitive hashing
impl Hash for Scheme {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        match self.inner {
            Scheme2::None => {}
            Scheme2::Standard(Protocol::Http) => state.write_u8(1),
            Scheme2::Standard(Protocol::Https) => state.write_u8(2),
            Scheme2::Other(ref other) => {
                other.len().hash(state);
                for &b in other.as_bytes() {
                    state.write_u8(b.to_ascii_lowercase());
                }
            }
        }
    }
}
impl<T> Scheme2<T> {
    pub(super) fn is_none(&self) -> bool {
        match *self {
            Scheme2::None => true,
            _ => false,
        }
    }
}
const MAX_SCHEME_LEN: usize = 64;
const SCHEME_CHARS: [u8; 256] = [
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
    b'+',
    0,
    b'-',
    b'.',
    0,
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
    0,
    0,
    0,
    0,
    0,
    0,
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
    0,
    0,
    0,
    0,
    0,
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
impl Scheme2<usize> {
    fn parse_exact(s: &[u8]) -> Result<Scheme2<()>, InvalidUri> {
        match s {
            b"http" => Ok(Protocol::Http.into()),
            b"https" => Ok(Protocol::Https.into()),
            _ => {
                if s.len() > MAX_SCHEME_LEN {
                    return Err(ErrorKind::SchemeTooLong.into());
                }
                for &b in s {
                    match SCHEME_CHARS[b as usize] {
                        b':' => {
                            return Err(ErrorKind::InvalidScheme.into());
                        }
                        0 => {
                            return Err(ErrorKind::InvalidScheme.into());
                        }
                        _ => {}
                    }
                }
                Ok(Scheme2::Other(()))
            }
        }
    }
    pub(super) fn parse(s: &[u8]) -> Result<Scheme2<usize>, InvalidUri> {
        if s.len() >= 7 {
            if s[..7].eq_ignore_ascii_case(b"http://") {
                return Ok(Protocol::Http.into());
            }
        }
        if s.len() >= 8 {
            if s[..8].eq_ignore_ascii_case(b"https://") {
                return Ok(Protocol::Https.into());
            }
        }
        if s.len() > 3 {
            for i in 0..s.len() {
                let b = s[i];
                match SCHEME_CHARS[b as usize] {
                    b':' => {
                        if s.len() < i + 3 {
                            break;
                        }
                        if &s[i + 1..i + 3] != b"//" {
                            break;
                        }
                        if i > MAX_SCHEME_LEN {
                            return Err(ErrorKind::SchemeTooLong.into());
                        }
                        return Ok(Scheme2::Other(i));
                    }
                    0 => break,
                    _ => {}
                }
            }
        }
        Ok(Scheme2::None)
    }
}
impl Protocol {
    pub(super) fn len(&self) -> usize {
        match *self {
            Protocol::Http => 4,
            Protocol::Https => 5,
        }
    }
}
impl<T> From<Protocol> for Scheme2<T> {
    fn from(src: Protocol) -> Self {
        Scheme2::Standard(src)
    }
}
#[doc(hidden)]
impl From<Scheme2> for Scheme {
    fn from(src: Scheme2) -> Self {
        Scheme { inner: src }
    }
}
#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn scheme_eq_to_str() {
        assert_eq!(& scheme("http"), "http");
        assert_eq!(& scheme("https"), "https");
        assert_eq!(& scheme("ftp"), "ftp");
        assert_eq!(& scheme("my+funky+scheme"), "my+funky+scheme");
    }
    #[test]
    fn invalid_scheme_is_error() {
        Scheme::try_from("my_funky_scheme").expect_err("Unexpectly valid Scheme");
        Scheme::try_from([0xC0].as_ref()).expect_err("Unexpectly valid Scheme");
    }
    fn scheme(s: &str) -> Scheme {
        s.parse().expect(&format!("Invalid scheme: {}", s))
    }
}
#[cfg(test)]
mod tests_llm_16_292 {
    use super::*;
    use crate::*;
    use std::convert::TryFrom;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_292_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "http";
        let rug_fuzz_1 = "HTTP";
        let rug_fuzz_2 = "HTTPS";
        let scheme: Scheme = rug_fuzz_0.parse().unwrap();
        debug_assert_eq!(scheme.eq(rug_fuzz_1), true);
        debug_assert_eq!(scheme.eq(rug_fuzz_2), false);
        let _rug_ed_tests_llm_16_292_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_293 {
    use super::*;
    use crate::*;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_293_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "http";
        let rug_fuzz_1 = "https";
        let rug_fuzz_2 = "HTTP";
        let rug_fuzz_3 = "ftp";
        let rug_fuzz_4 = "HTTP";
        let rug_fuzz_5 = "HTTPS";
        let scheme1: Scheme = rug_fuzz_0.parse().unwrap();
        let scheme2: Scheme = rug_fuzz_1.parse().unwrap();
        let scheme3: Scheme = rug_fuzz_2.parse().unwrap();
        let scheme4: Scheme = rug_fuzz_3.parse().unwrap();
        let scheme5: Scheme = rug_fuzz_4.parse().unwrap();
        let scheme6: Scheme = rug_fuzz_5.parse().unwrap();
        let result1 = scheme1.eq(&scheme2);
        let result2 = scheme1.eq(&scheme3);
        let result3 = scheme1.eq(&scheme4);
        let result4 = scheme1.eq(&scheme5);
        let result5 = scheme1.eq(&scheme6);
        debug_assert_eq!(result1, false);
        debug_assert_eq!(result2, false);
        debug_assert_eq!(result3, false);
        debug_assert_eq!(result4, false);
        debug_assert_eq!(result5, false);
        let _rug_ed_tests_llm_16_293_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_294 {
    use std::convert::TryFrom;
    use crate::uri::scheme::{Scheme, Protocol};
    #[test]
    fn test_as_ref() {
        let _rug_st_tests_llm_16_294_rrrruuuugggg_test_as_ref = 0;
        let rug_fuzz_0 = "http";
        let rug_fuzz_1 = "https";
        let rug_fuzz_2 = "ftp";
        let scheme1: Scheme = rug_fuzz_0.parse().unwrap();
        let scheme2: Scheme = rug_fuzz_1.parse().unwrap();
        let scheme3: Scheme = rug_fuzz_2.parse().unwrap();
        debug_assert_eq!(scheme1.as_ref(), "http");
        debug_assert_eq!(scheme2.as_ref(), "https");
        debug_assert_eq!(scheme3.as_ref(), "ftp");
        let _rug_ed_tests_llm_16_294_rrrruuuugggg_test_as_ref = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_304 {
    use std::convert::TryFrom;
    use std::str::FromStr;
    #[test]
    fn test_from_str() {
        let _rug_st_tests_llm_16_304_rrrruuuugggg_test_from_str = 0;
        let rug_fuzz_0 = "http";
        let rug_fuzz_1 = "http";
        let result: Result<crate::uri::scheme::Scheme, crate::uri::InvalidUri> = Result::Ok(
            crate::uri::scheme::Scheme::try_from(rug_fuzz_0).unwrap(),
        );
        debug_assert_eq!(
            crate ::uri::scheme::Scheme::from_str(rug_fuzz_1).unwrap(), result.unwrap()
        );
        let _rug_ed_tests_llm_16_304_rrrruuuugggg_test_from_str = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_607 {
    use super::*;
    use crate::*;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_607_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "ftp";
        let scheme1: Scheme = Scheme {
            inner: Scheme2::Standard(Protocol::Http),
        };
        let scheme2: Scheme = Scheme {
            inner: Scheme2::Standard(Protocol::Http),
        };
        let scheme3: Scheme = Scheme {
            inner: Scheme2::Standard(Protocol::Https),
        };
        let scheme4: Scheme = Scheme {
            inner: Scheme2::Other(Box::new(ByteStr::from_static(rug_fuzz_0))),
        };
        let scheme5: Scheme = Scheme { inner: Scheme2::None };
        debug_assert_eq!(scheme1.eq(& scheme2), true);
        debug_assert_eq!(scheme1.eq(& scheme3), false);
        debug_assert_eq!(scheme1.eq(& scheme4), false);
        debug_assert_eq!(scheme2.eq(& scheme4), false);
        debug_assert_eq!(scheme1.eq(& scheme5), false);
        let _rug_ed_tests_llm_16_607_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_609 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_none() {
        let _rug_st_tests_llm_16_609_rrrruuuugggg_test_is_none = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = false;
        let scheme: Scheme2<Box<ByteStr>> = Scheme2::None;
        debug_assert_eq!(rug_fuzz_0, scheme.is_none());
        let scheme: Scheme2<Box<ByteStr>> = Scheme2::Standard(Protocol::Http);
        debug_assert_eq!(rug_fuzz_1, scheme.is_none());
        let _rug_ed_tests_llm_16_609_rrrruuuugggg_test_is_none = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_612 {
    use crate::uri::*;
    #[test]
    fn test_as_str() {
        let _rug_st_tests_llm_16_612_rrrruuuugggg_test_as_str = 0;
        let rug_fuzz_0 = "http";
        let rug_fuzz_1 = "https";
        let rug_fuzz_2 = "ftp";
        let scheme: Scheme = rug_fuzz_0.parse().unwrap();
        debug_assert_eq!(scheme.as_str(), "http");
        let scheme: Scheme = rug_fuzz_1.parse().unwrap();
        debug_assert_eq!(scheme.as_str(), "https");
        let scheme: Scheme = rug_fuzz_2.parse().unwrap();
        debug_assert_eq!(scheme.as_str(), "ftp");
        let _rug_ed_tests_llm_16_612_rrrruuuugggg_test_as_str = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_613 {
    use super::*;
    use crate::*;
    #[test]
    fn test_empty() {
        let _rug_st_tests_llm_16_613_rrrruuuugggg_test_empty = 0;
        let scheme = Scheme::empty();
        let expected_scheme = Scheme { inner: Scheme2::None };
        debug_assert_eq!(scheme, expected_scheme);
        let _rug_ed_tests_llm_16_613_rrrruuuugggg_test_empty = 0;
    }
}
#[cfg(test)]
mod tests_rug_244 {
    use super::*;
    use crate::uri::scheme::{Scheme, Scheme2};
    use crate::uri::InvalidUri;
    use std::convert::TryFrom;
    #[test]
    fn test_try_from() {
        let _rug_st_tests_rug_244_rrrruuuugggg_test_try_from = 0;
        let rug_fuzz_0 = b"http";
        let p0: &[u8] = rug_fuzz_0;
        let result: Result<Scheme, InvalidUri> = <Scheme as std::convert::TryFrom<
            &[u8],
        >>::try_from(p0);
        debug_assert!(result.is_ok());
        let _rug_ed_tests_rug_244_rrrruuuugggg_test_try_from = 0;
    }
}
#[cfg(test)]
mod tests_rug_247 {
    use super::*;
    use crate::uri::InvalidUri;
    use crate::uri::scheme::{Scheme2, MAX_SCHEME_LEN, SCHEME_CHARS};
    use crate::uri::ErrorKind;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_247_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"http";
        let mut p0: &[u8] = rug_fuzz_0;
        <Scheme2<usize>>::parse_exact(p0);
        let _rug_ed_tests_rug_247_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_248 {
    use super::*;
    use crate::uri::scheme::{Scheme2, InvalidUri};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_248_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"http://example.com";
        let mut p0: &[u8] = rug_fuzz_0;
        <Scheme2<usize>>::parse(p0);
        let _rug_ed_tests_rug_248_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_249 {
    use super::*;
    use crate::uri::scheme::Protocol;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_249_rrrruuuugggg_test_rug = 0;
        let p0: Protocol = Protocol::Http;
        <Protocol>::len(&p0);
        let p1: Protocol = Protocol::Https;
        <Protocol>::len(&p1);
        let _rug_ed_tests_rug_249_rrrruuuugggg_test_rug = 0;
    }
}
