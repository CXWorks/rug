use std::fmt;
use super::{ErrorKind, InvalidUri};
/// The port component of a URI.
pub struct Port<T> {
    port: u16,
    repr: T,
}
impl<T> Port<T> {
    /// Returns the port number as a `u16`.
    ///
    /// # Examples
    ///
    /// Port as `u16`.
    ///
    /// ```
    /// # use http::uri::Authority;
    /// let authority: Authority = "example.org:80".parse().unwrap();
    ///
    /// let port = authority.port().unwrap();
    /// assert_eq!(port.as_u16(), 80);
    /// ```
    pub fn as_u16(&self) -> u16 {
        self.port
    }
}
impl<T> Port<T>
where
    T: AsRef<str>,
{
    /// Converts a `str` to a port number.
    ///
    /// The supplied `str` must be a valid u16.
    pub(crate) fn from_str(bytes: T) -> Result<Self, InvalidUri> {
        bytes
            .as_ref()
            .parse::<u16>()
            .map(|port| Port { port, repr: bytes })
            .map_err(|_| ErrorKind::InvalidPort.into())
    }
    /// Returns the port number as a `str`.
    ///
    /// # Examples
    ///
    /// Port as `str`.
    ///
    /// ```
    /// # use http::uri::Authority;
    /// let authority: Authority = "example.org:80".parse().unwrap();
    ///
    /// let port = authority.port().unwrap();
    /// assert_eq!(port.as_str(), "80");
    /// ```
    pub fn as_str(&self) -> &str {
        self.repr.as_ref()
    }
}
impl<T> fmt::Debug for Port<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Port").field(&self.port).finish()
    }
}
impl<T> fmt::Display for Port<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.port, f)
    }
}
impl<T> From<Port<T>> for u16 {
    fn from(port: Port<T>) -> Self {
        port.as_u16()
    }
}
impl<T> AsRef<str> for Port<T>
where
    T: AsRef<str>,
{
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
impl<T, U> PartialEq<Port<U>> for Port<T> {
    fn eq(&self, other: &Port<U>) -> bool {
        self.port == other.port
    }
}
impl<T> PartialEq<u16> for Port<T> {
    fn eq(&self, other: &u16) -> bool {
        self.port == *other
    }
}
impl<T> PartialEq<Port<T>> for u16 {
    fn eq(&self, other: &Port<T>) -> bool {
        other.port == *self
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partialeq_port() {
        let port_a = Port::from_str("8080").unwrap();
        let port_b = Port::from_str("8080").unwrap();
        assert_eq!(port_a, port_b);
    }
    #[test]
    fn partialeq_port_different_reprs() {
        let port_a = Port { repr: "8081", port: 8081 };
        let port_b = Port {
            repr: String::from("8081"),
            port: 8081,
        };
        assert_eq!(port_a, port_b);
        assert_eq!(port_b, port_a);
    }
    #[test]
    fn partialeq_u16() {
        let port = Port::from_str("8080").unwrap();
        assert_eq!(port, 8080);
        assert_eq!(8080, port);
    }
    #[test]
    fn u16_from_port() {
        let port = Port::from_str("8080").unwrap();
        assert_eq!(8080, u16::from(port));
    }
}
#[cfg(test)]
mod tests_llm_16_288 {
    use super::*;
    use crate::*;
    use crate::uri::InvalidUri;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_288_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = 8080;
        let rug_fuzz_1 = "8080";
        let rug_fuzz_2 = 8080;
        let rug_fuzz_3 = 8081;
        let port: Port<String> = Port {
            port: rug_fuzz_0,
            repr: String::from(rug_fuzz_1),
        };
        let result = port.eq(&rug_fuzz_2);
        debug_assert_eq!(result, true);
        let result = port.eq(&rug_fuzz_3);
        debug_assert_eq!(result, false);
        let _rug_ed_tests_llm_16_288_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_289 {
    use super::*;
    use crate::*;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_289_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = 8080;
        let rug_fuzz_1 = "8080";
        let rug_fuzz_2 = 8080;
        let rug_fuzz_3 = 8080;
        let rug_fuzz_4 = 8081;
        let rug_fuzz_5 = 8081;
        let port1: Port<String> = Port {
            port: rug_fuzz_0,
            repr: String::from(rug_fuzz_1),
        };
        let port2: Port<u16> = Port {
            port: rug_fuzz_2,
            repr: rug_fuzz_3,
        };
        let port3: Port<u16> = Port {
            port: rug_fuzz_4,
            repr: rug_fuzz_5,
        };
        debug_assert_eq!(port1.eq(& port2), true);
        debug_assert_eq!(port1.eq(& port3), false);
        debug_assert_eq!(port2.eq(& port3), false);
        let _rug_ed_tests_llm_16_289_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_602 {
    use crate::uri::port::Port;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_602_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "80";
        let rug_fuzz_1 = "80";
        let rug_fuzz_2 = "8080";
        let port1: Port<&str> = Port::from_str(rug_fuzz_0).unwrap();
        let port2: Port<&str> = Port::from_str(rug_fuzz_1).unwrap();
        let port3: Port<&str> = Port::from_str(rug_fuzz_2).unwrap();
        debug_assert_eq!(port1.eq(& port2), true);
        debug_assert_eq!(port1.eq(& port3), false);
        let _rug_ed_tests_llm_16_602_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_603 {
    use crate::uri::port::Port;
    use std::convert::From;
    #[test]
    fn test_from() {
        let _rug_st_tests_llm_16_603_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = 8080;
        let rug_fuzz_1 = "8080";
        let port: Port<&str> = Port {
            port: rug_fuzz_0,
            repr: rug_fuzz_1,
        };
        let result: u16 = u16::from(port);
        debug_assert_eq!(result, 8080);
        let _rug_ed_tests_llm_16_603_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_604 {
    use super::*;
    use crate::*;
    use crate::uri::Authority;
    #[test]
    fn test_as_str() {
        let _rug_st_tests_llm_16_604_rrrruuuugggg_test_as_str = 0;
        let rug_fuzz_0 = "example.org:80";
        let authority: Authority = rug_fuzz_0.parse().unwrap();
        let port = authority.port().unwrap();
        debug_assert_eq!(port.as_str(), "80");
        let _rug_ed_tests_llm_16_604_rrrruuuugggg_test_as_str = 0;
    }
}
