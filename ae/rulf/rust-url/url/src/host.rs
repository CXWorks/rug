use std::cmp;
use std::fmt::{self, Formatter};
use std::net::{Ipv4Addr, Ipv6Addr};
use percent_encoding::{percent_decode, utf8_percent_encode, CONTROLS};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use crate::parser::{ParseError, ParseResult};
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum HostInternal {
    None,
    Domain,
    Ipv4(Ipv4Addr),
    Ipv6(Ipv6Addr),
}
impl From<Host<String>> for HostInternal {
    fn from(host: Host<String>) -> HostInternal {
        match host {
            Host::Domain(ref s) if s.is_empty() => HostInternal::None,
            Host::Domain(_) => HostInternal::Domain,
            Host::Ipv4(address) => HostInternal::Ipv4(address),
            Host::Ipv6(address) => HostInternal::Ipv6(address),
        }
    }
}
/// The host name of an URL.
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Host<S = String> {
    /// A DNS domain name, as '.' dot-separated labels.
    /// Non-ASCII labels are encoded in punycode per IDNA if this is the host of
    /// a special URL, or percent encoded for non-special URLs. Hosts for
    /// non-special URLs are also called opaque hosts.
    Domain(S),
    /// An IPv4 address.
    /// `Url::host_str` returns the serialization of this address,
    /// as four decimal integers separated by `.` dots.
    Ipv4(Ipv4Addr),
    /// An IPv6 address.
    /// `Url::host_str` returns the serialization of that address between `[` and `]` brackets,
    /// in the format per [RFC 5952 *A Recommendation
    /// for IPv6 Address Text Representation*](https://tools.ietf.org/html/rfc5952):
    /// lowercase hexadecimal with maximal `::` compression.
    Ipv6(Ipv6Addr),
}
impl<'a> Host<&'a str> {
    /// Return a copy of `self` that owns an allocated `String` but does not borrow an `&Url`.
    pub fn to_owned(&self) -> Host<String> {
        match *self {
            Host::Domain(domain) => Host::Domain(domain.to_owned()),
            Host::Ipv4(address) => Host::Ipv4(address),
            Host::Ipv6(address) => Host::Ipv6(address),
        }
    }
}
impl Host<String> {
    /// Parse a host: either an IPv6 address in [] square brackets, or a domain.
    ///
    /// <https://url.spec.whatwg.org/#host-parsing>
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        if input.starts_with('[') {
            if !input.ends_with(']') {
                return Err(ParseError::InvalidIpv6Address);
            }
            return parse_ipv6addr(&input[1..input.len() - 1]).map(Host::Ipv6);
        }
        let domain = percent_decode(input.as_bytes()).decode_utf8_lossy();
        let domain = idna::domain_to_ascii(&domain)?;
        if domain.is_empty() {
            return Err(ParseError::EmptyHost);
        }
        let is_invalid_domain_char = |c| {
            matches!(
                c, '\0' | '\t' | '\n' | '\r' | ' ' | '#' | '%' | '/' | ':' | '<' | '>' |
                '?' | '@' | '[' | '\\' | ']' | '^'
            )
        };
        if domain.find(is_invalid_domain_char).is_some() {
            Err(ParseError::InvalidDomainCharacter)
        } else if let Some(address) = parse_ipv4addr(&domain)? {
            Ok(Host::Ipv4(address))
        } else {
            Ok(Host::Domain(domain))
        }
    }
    pub fn parse_opaque(input: &str) -> Result<Self, ParseError> {
        if input.starts_with('[') {
            if !input.ends_with(']') {
                return Err(ParseError::InvalidIpv6Address);
            }
            return parse_ipv6addr(&input[1..input.len() - 1]).map(Host::Ipv6);
        }
        let is_invalid_host_char = |c| {
            matches!(
                c, '\0' | '\t' | '\n' | '\r' | ' ' | '#' | '/' | ':' | '<' | '>' | '?' |
                '@' | '[' | '\\' | ']' | '^'
            )
        };
        if input.find(is_invalid_host_char).is_some() {
            Err(ParseError::InvalidDomainCharacter)
        } else {
            Ok(Host::Domain(utf8_percent_encode(input, CONTROLS).to_string()))
        }
    }
}
impl<S: AsRef<str>> fmt::Display for Host<S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self {
            Host::Domain(ref domain) => domain.as_ref().fmt(f),
            Host::Ipv4(ref addr) => addr.fmt(f),
            Host::Ipv6(ref addr) => {
                f.write_str("[")?;
                write_ipv6(addr, f)?;
                f.write_str("]")
            }
        }
    }
}
fn write_ipv6(addr: &Ipv6Addr, f: &mut Formatter<'_>) -> fmt::Result {
    let segments = addr.segments();
    let (compress_start, compress_end) = longest_zero_sequence(&segments);
    let mut i = 0;
    while i < 8 {
        if i == compress_start {
            f.write_str(":")?;
            if i == 0 {
                f.write_str(":")?;
            }
            if compress_end < 8 {
                i = compress_end;
            } else {
                break;
            }
        }
        write!(f, "{:x}", segments[i as usize])?;
        if i < 7 {
            f.write_str(":")?;
        }
        i += 1;
    }
    Ok(())
}
fn longest_zero_sequence(pieces: &[u16; 8]) -> (isize, isize) {
    let mut longest = -1;
    let mut longest_length = -1;
    let mut start = -1;
    macro_rules! finish_sequence {
        ($end:expr) => {
            if start >= 0 { let length = $end - start; if length > longest_length {
            longest = start; longest_length = length; } }
        };
    }
    for i in 0..8 {
        if pieces[i as usize] == 0 {
            if start < 0 {
                start = i;
            }
        } else {
            finish_sequence!(i);
            start = -1;
        }
    }
    finish_sequence!(8);
    if longest_length < 2 { (-1, -2) } else { (longest, longest + longest_length) }
}
/// <https://url.spec.whatwg.org/#ipv4-number-parser>
fn parse_ipv4number(mut input: &str) -> Result<Option<u32>, ()> {
    let mut r = 10;
    if input.starts_with("0x") || input.starts_with("0X") {
        input = &input[2..];
        r = 16;
    } else if input.len() >= 2 && input.starts_with('0') {
        input = &input[1..];
        r = 8;
    }
    let valid_number = match r {
        8 => input.chars().all(|c| c >= '0' && c <= '7'),
        10 => input.chars().all(|c| c >= '0' && c <= '9'),
        16 => {
            input
                .chars()
                .all(|c| {
                    (c >= '0' && c <= '9') || (c >= 'a' && c <= 'f')
                        || (c >= 'A' && c <= 'F')
                })
        }
        _ => false,
    };
    if !valid_number {
        return Ok(None);
    }
    if input.is_empty() {
        return Ok(Some(0));
    }
    if input.starts_with('+') {
        return Ok(None);
    }
    match u32::from_str_radix(input, r) {
        Ok(number) => Ok(Some(number)),
        Err(_) => Err(()),
    }
}
/// <https://url.spec.whatwg.org/#concept-ipv4-parser>
fn parse_ipv4addr(input: &str) -> ParseResult<Option<Ipv4Addr>> {
    if input.is_empty() {
        return Ok(None);
    }
    let mut parts: Vec<&str> = input.split('.').collect();
    if parts.last() == Some(&"") {
        parts.pop();
    }
    if parts.len() > 4 {
        return Ok(None);
    }
    let mut numbers: Vec<u32> = Vec::new();
    let mut overflow = false;
    for part in parts {
        if part == "" {
            return Ok(None);
        }
        match parse_ipv4number(part) {
            Ok(Some(n)) => numbers.push(n),
            Ok(None) => return Ok(None),
            Err(()) => overflow = true,
        };
    }
    if overflow {
        return Err(ParseError::InvalidIpv4Address);
    }
    let mut ipv4 = numbers.pop().expect("a non-empty list of numbers");
    if ipv4 > u32::max_value() >> (8 * numbers.len() as u32) {
        return Err(ParseError::InvalidIpv4Address);
    }
    if numbers.iter().any(|x| *x > 255) {
        return Err(ParseError::InvalidIpv4Address);
    }
    for (counter, n) in numbers.iter().enumerate() {
        ipv4 += n << (8 * (3 - counter as u32));
    }
    Ok(Some(Ipv4Addr::from(ipv4)))
}
/// <https://url.spec.whatwg.org/#concept-ipv6-parser>
fn parse_ipv6addr(input: &str) -> ParseResult<Ipv6Addr> {
    let input = input.as_bytes();
    let len = input.len();
    let mut is_ip_v4 = false;
    let mut pieces = [0, 0, 0, 0, 0, 0, 0, 0];
    let mut piece_pointer = 0;
    let mut compress_pointer = None;
    let mut i = 0;
    if len < 2 {
        return Err(ParseError::InvalidIpv6Address);
    }
    if input[0] == b':' {
        if input[1] != b':' {
            return Err(ParseError::InvalidIpv6Address);
        }
        i = 2;
        piece_pointer = 1;
        compress_pointer = Some(1);
    }
    while i < len {
        if piece_pointer == 8 {
            return Err(ParseError::InvalidIpv6Address);
        }
        if input[i] == b':' {
            if compress_pointer.is_some() {
                return Err(ParseError::InvalidIpv6Address);
            }
            i += 1;
            piece_pointer += 1;
            compress_pointer = Some(piece_pointer);
            continue;
        }
        let start = i;
        let end = cmp::min(len, start + 4);
        let mut value = 0u16;
        while i < end {
            match (input[i] as char).to_digit(16) {
                Some(digit) => {
                    value = value * 0x10 + digit as u16;
                    i += 1;
                }
                None => break,
            }
        }
        if i < len {
            match input[i] {
                b'.' => {
                    if i == start {
                        return Err(ParseError::InvalidIpv6Address);
                    }
                    i = start;
                    if piece_pointer > 6 {
                        return Err(ParseError::InvalidIpv6Address);
                    }
                    is_ip_v4 = true;
                }
                b':' => {
                    i += 1;
                    if i == len {
                        return Err(ParseError::InvalidIpv6Address);
                    }
                }
                _ => return Err(ParseError::InvalidIpv6Address),
            }
        }
        if is_ip_v4 {
            break;
        }
        pieces[piece_pointer] = value;
        piece_pointer += 1;
    }
    if is_ip_v4 {
        if piece_pointer > 6 {
            return Err(ParseError::InvalidIpv6Address);
        }
        let mut numbers_seen = 0;
        while i < len {
            if numbers_seen > 0 {
                if numbers_seen < 4 && (i < len && input[i] == b'.') {
                    i += 1
                } else {
                    return Err(ParseError::InvalidIpv6Address);
                }
            }
            let mut ipv4_piece = None;
            while i < len {
                let digit = match input[i] {
                    c @ b'0'..=b'9' => c - b'0',
                    _ => break,
                };
                match ipv4_piece {
                    None => ipv4_piece = Some(digit as u16),
                    Some(0) => return Err(ParseError::InvalidIpv6Address),
                    Some(ref mut v) => {
                        *v = *v * 10 + digit as u16;
                        if *v > 255 {
                            return Err(ParseError::InvalidIpv6Address);
                        }
                    }
                }
                i += 1;
            }
            pieces[piece_pointer] = if let Some(v) = ipv4_piece {
                pieces[piece_pointer] * 0x100 + v
            } else {
                return Err(ParseError::InvalidIpv6Address);
            };
            numbers_seen += 1;
            if numbers_seen == 2 || numbers_seen == 4 {
                piece_pointer += 1;
            }
        }
        if numbers_seen != 4 {
            return Err(ParseError::InvalidIpv6Address);
        }
    }
    if i < len {
        return Err(ParseError::InvalidIpv6Address);
    }
    match compress_pointer {
        Some(compress_pointer) => {
            let mut swaps = piece_pointer - compress_pointer;
            piece_pointer = 7;
            while swaps > 0 {
                pieces.swap(piece_pointer, compress_pointer + swaps - 1);
                swaps -= 1;
                piece_pointer -= 1;
            }
        }
        _ => {
            if piece_pointer != 8 {
                return Err(ParseError::InvalidIpv6Address);
            }
        }
    }
    Ok(
        Ipv6Addr::new(
            pieces[0],
            pieces[1],
            pieces[2],
            pieces[3],
            pieces[4],
            pieces[5],
            pieces[6],
            pieces[7],
        ),
    )
}
#[cfg(test)]
mod tests_llm_16_12 {
    use super::*;
    use crate::*;
    use std::net::{Ipv4Addr, Ipv6Addr};
    #[test]
    fn test_from_none() {
        let _rug_st_tests_llm_16_12_rrrruuuugggg_test_from_none = 0;
        let host = Host::Domain(String::new());
        let expected = HostInternal::None;
        let result = <host::HostInternal as std::convert::From<
            host::Host<String>,
        >>::from(host);
        debug_assert_eq!(expected, result);
        let _rug_ed_tests_llm_16_12_rrrruuuugggg_test_from_none = 0;
    }
    #[test]
    fn test_from_domain() {
        let _rug_st_tests_llm_16_12_rrrruuuugggg_test_from_domain = 0;
        let rug_fuzz_0 = "example.com";
        let domain = String::from(rug_fuzz_0);
        let host = Host::Domain(domain);
        let expected = HostInternal::Domain;
        let result = <host::HostInternal as std::convert::From<
            host::Host<String>,
        >>::from(host);
        debug_assert_eq!(expected, result);
        let _rug_ed_tests_llm_16_12_rrrruuuugggg_test_from_domain = 0;
    }
    #[test]
    fn test_from_ipv4() {
        let _rug_st_tests_llm_16_12_rrrruuuugggg_test_from_ipv4 = 0;
        let rug_fuzz_0 = 127;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 1;
        let address = Ipv4Addr::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3);
        let host = Host::Ipv4(address);
        let expected = HostInternal::Ipv4(address);
        let result = <host::HostInternal as std::convert::From<
            host::Host<String>,
        >>::from(host);
        debug_assert_eq!(expected, result);
        let _rug_ed_tests_llm_16_12_rrrruuuugggg_test_from_ipv4 = 0;
    }
    #[test]
    fn test_from_ipv6() {
        let _rug_st_tests_llm_16_12_rrrruuuugggg_test_from_ipv6 = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 0;
        let rug_fuzz_7 = 1;
        let address = Ipv6Addr::new(
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
            rug_fuzz_6,
            rug_fuzz_7,
        );
        let host = Host::Ipv6(address);
        let expected = HostInternal::Ipv6(address);
        let result = <host::HostInternal as std::convert::From<
            host::Host<String>,
        >>::from(host);
        debug_assert_eq!(expected, result);
        let _rug_ed_tests_llm_16_12_rrrruuuugggg_test_from_ipv6 = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_34 {
    use super::*;
    use crate::*;
    use std::net::{Ipv4Addr, Ipv6Addr};
    #[test]
    fn test_to_owned() {
        let _rug_st_tests_llm_16_34_rrrruuuugggg_test_to_owned = 0;
        let rug_fuzz_0 = "example.com";
        let rug_fuzz_1 = "example.com";
        let rug_fuzz_2 = 127;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 1;
        let rug_fuzz_6 = 127;
        let rug_fuzz_7 = 0;
        let rug_fuzz_8 = 0;
        let rug_fuzz_9 = 1;
        let rug_fuzz_10 = 0;
        let rug_fuzz_11 = 0;
        let rug_fuzz_12 = 0;
        let rug_fuzz_13 = 0;
        let rug_fuzz_14 = 0;
        let rug_fuzz_15 = 0;
        let rug_fuzz_16 = 0;
        let rug_fuzz_17 = 1;
        let rug_fuzz_18 = 0;
        let rug_fuzz_19 = 0;
        let rug_fuzz_20 = 0;
        let rug_fuzz_21 = 0;
        let rug_fuzz_22 = 0;
        let rug_fuzz_23 = 0;
        let rug_fuzz_24 = 0;
        let rug_fuzz_25 = 1;
        let domain: Host<&str> = Host::Domain(rug_fuzz_0);
        let domain_owned: Host<String> = Host::Domain(rug_fuzz_1.to_owned());
        debug_assert_eq!(domain.to_owned(), domain_owned);
        let ipv4: Host<&str> = Host::Ipv4(
            Ipv4Addr::new(rug_fuzz_2, rug_fuzz_3, rug_fuzz_4, rug_fuzz_5),
        );
        let ipv4_owned: Host<String> = Host::Ipv4(
            Ipv4Addr::new(rug_fuzz_6, rug_fuzz_7, rug_fuzz_8, rug_fuzz_9),
        );
        debug_assert_eq!(ipv4.to_owned(), ipv4_owned);
        let ipv6: Host<&str> = Host::Ipv6(
            Ipv6Addr::new(
                rug_fuzz_10,
                rug_fuzz_11,
                rug_fuzz_12,
                rug_fuzz_13,
                rug_fuzz_14,
                rug_fuzz_15,
                rug_fuzz_16,
                rug_fuzz_17,
            ),
        );
        let ipv6_owned: Host<String> = Host::Ipv6(
            Ipv6Addr::new(
                rug_fuzz_18,
                rug_fuzz_19,
                rug_fuzz_20,
                rug_fuzz_21,
                rug_fuzz_22,
                rug_fuzz_23,
                rug_fuzz_24,
                rug_fuzz_25,
            ),
        );
        debug_assert_eq!(ipv6.to_owned(), ipv6_owned);
        let _rug_ed_tests_llm_16_34_rrrruuuugggg_test_to_owned = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_36 {
    use super::*;
    use crate::*;
    use std::net::{Ipv6Addr, Ipv4Addr};
    #[test]
    fn test_parse() {
        let _rug_st_tests_llm_16_36_rrrruuuugggg_test_parse = 0;
        let rug_fuzz_0 = "[::1]";
        let rug_fuzz_1 = "[2001:db8::1]";
        let rug_fuzz_2 = "[::1";
        let rug_fuzz_3 = "example.com";
        let rug_fuzz_4 = "::1";
        let rug_fuzz_5 = "127.0.0.1";
        let rug_fuzz_6 = "example.com#";
        let rug_fuzz_7 = "";
        let rug_fuzz_8 = "[:::";
        let rug_fuzz_9 = "[::1]:8080";
        debug_assert_eq!(
            host::Host::parse(rug_fuzz_0), Ok(host::Host::Ipv6(Ipv6Addr::new(0, 0, 0, 0,
            0, 0, 0, 1)))
        );
        debug_assert_eq!(
            host::Host::parse(rug_fuzz_1), Ok(host::Host::Ipv6(Ipv6Addr::new(0x2001,
            0x0db8, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0001)))
        );
        debug_assert_eq!(
            host::Host::parse(rug_fuzz_2), Err(host::ParseError::InvalidIpv6Address)
        );
        debug_assert_eq!(
            host::Host::parse(rug_fuzz_3), Ok(host::Host::Domain("example.com"
            .to_string()))
        );
        debug_assert_eq!(
            host::Host::parse(rug_fuzz_4), Ok(host::Host::Ipv6(Ipv6Addr::new(0, 0, 0, 0,
            0, 0, 0, 1)))
        );
        debug_assert_eq!(
            host::Host::parse(rug_fuzz_5), Ok(host::Host::Ipv4(Ipv4Addr::new(127, 0, 0,
            1)))
        );
        debug_assert_eq!(
            host::Host::parse(rug_fuzz_6), Err(host::ParseError::InvalidDomainCharacter)
        );
        debug_assert_eq!(
            host::Host::parse(rug_fuzz_7), Err(host::ParseError::EmptyHost)
        );
        debug_assert_eq!(
            host::Host::parse(rug_fuzz_8), Err(host::ParseError::InvalidIpv6Address)
        );
        debug_assert_eq!(
            host::Host::parse(rug_fuzz_9), Ok(host::Host::Ipv6(Ipv6Addr::new(0, 0, 0, 0,
            0, 0, 0, 1)))
        );
        let _rug_ed_tests_llm_16_36_rrrruuuugggg_test_parse = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_39 {
    use super::*;
    use crate::*;
    #[test]
    fn test_longest_zero_sequence() {
        let _rug_st_tests_llm_16_39_rrrruuuugggg_test_longest_zero_sequence = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 1;
        let rug_fuzz_6 = 0;
        let rug_fuzz_7 = 1;
        let pieces: [u16; 8] = [
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
            rug_fuzz_6,
            rug_fuzz_7,
        ];
        let result = longest_zero_sequence(&pieces);
        debug_assert_eq!(result, (1, 5));
        let _rug_ed_tests_llm_16_39_rrrruuuugggg_test_longest_zero_sequence = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_40 {
    use super::*;
    use crate::*;
    use crate::host::parse_ipv4addr;
    #[test]
    fn test_parse_ipv4addr_empty() {
        let _rug_st_tests_llm_16_40_rrrruuuugggg_test_parse_ipv4addr_empty = 0;
        let rug_fuzz_0 = "";
        debug_assert_eq!(parse_ipv4addr(rug_fuzz_0), Ok(None));
        let _rug_ed_tests_llm_16_40_rrrruuuugggg_test_parse_ipv4addr_empty = 0;
    }
    #[test]
    fn test_parse_ipv4addr_valid() {
        let _rug_st_tests_llm_16_40_rrrruuuugggg_test_parse_ipv4addr_valid = 0;
        let rug_fuzz_0 = "127.0.0.1";
        let rug_fuzz_1 = "192.168.0.1";
        debug_assert_eq!(
            parse_ipv4addr(rug_fuzz_0), Ok(Some(Ipv4Addr::new(127, 0, 0, 1)))
        );
        debug_assert_eq!(
            parse_ipv4addr(rug_fuzz_1), Ok(Some(Ipv4Addr::new(192, 168, 0, 1)))
        );
        let _rug_ed_tests_llm_16_40_rrrruuuugggg_test_parse_ipv4addr_valid = 0;
    }
    #[test]
    fn test_parse_ipv4addr_invalid() {
        let _rug_st_tests_llm_16_40_rrrruuuugggg_test_parse_ipv4addr_invalid = 0;
        let rug_fuzz_0 = "1234.0.0.1";
        let rug_fuzz_1 = "256.0.0.1";
        let rug_fuzz_2 = "127.0.0";
        let rug_fuzz_3 = "127.0.0.1.0";
        let rug_fuzz_4 = "127.0.0.a";
        debug_assert_eq!(parse_ipv4addr(rug_fuzz_0), Ok(None));
        debug_assert_eq!(parse_ipv4addr(rug_fuzz_1), Ok(None));
        debug_assert_eq!(parse_ipv4addr(rug_fuzz_2), Ok(None));
        debug_assert_eq!(parse_ipv4addr(rug_fuzz_3), Ok(None));
        debug_assert_eq!(parse_ipv4addr(rug_fuzz_4), Ok(None));
        let _rug_ed_tests_llm_16_40_rrrruuuugggg_test_parse_ipv4addr_invalid = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_41 {
    use super::*;
    use crate::*;
    #[test]
    fn test_parse_ipv4number_valid_decimal() {
        let _rug_st_tests_llm_16_41_rrrruuuugggg_test_parse_ipv4number_valid_decimal = 0;
        let rug_fuzz_0 = "123";
        let rug_fuzz_1 = 123;
        let input = rug_fuzz_0;
        let expected = Ok(Some(rug_fuzz_1));
        debug_assert_eq!(parse_ipv4number(input), expected);
        let _rug_ed_tests_llm_16_41_rrrruuuugggg_test_parse_ipv4number_valid_decimal = 0;
    }
    #[test]
    fn test_parse_ipv4number_valid_octal() {
        let _rug_st_tests_llm_16_41_rrrruuuugggg_test_parse_ipv4number_valid_octal = 0;
        let rug_fuzz_0 = "0o123";
        let rug_fuzz_1 = 83;
        let input = rug_fuzz_0;
        let expected = Ok(Some(rug_fuzz_1));
        debug_assert_eq!(parse_ipv4number(input), expected);
        let _rug_ed_tests_llm_16_41_rrrruuuugggg_test_parse_ipv4number_valid_octal = 0;
    }
    #[test]
    fn test_parse_ipv4number_valid_hexadecimal() {
        let _rug_st_tests_llm_16_41_rrrruuuugggg_test_parse_ipv4number_valid_hexadecimal = 0;
        let rug_fuzz_0 = "0xabc";
        let rug_fuzz_1 = 2748;
        let input = rug_fuzz_0;
        let expected = Ok(Some(rug_fuzz_1));
        debug_assert_eq!(parse_ipv4number(input), expected);
        let _rug_ed_tests_llm_16_41_rrrruuuugggg_test_parse_ipv4number_valid_hexadecimal = 0;
    }
    #[test]
    fn test_parse_ipv4number_invalid_number() {
        let _rug_st_tests_llm_16_41_rrrruuuugggg_test_parse_ipv4number_invalid_number = 0;
        let rug_fuzz_0 = "123z";
        let input = rug_fuzz_0;
        let expected = Ok(None);
        debug_assert_eq!(parse_ipv4number(input), expected);
        let _rug_ed_tests_llm_16_41_rrrruuuugggg_test_parse_ipv4number_invalid_number = 0;
    }
    #[test]
    fn test_parse_ipv4number_empty_input() {
        let _rug_st_tests_llm_16_41_rrrruuuugggg_test_parse_ipv4number_empty_input = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = 0;
        let input = rug_fuzz_0;
        let expected = Ok(Some(rug_fuzz_1));
        debug_assert_eq!(parse_ipv4number(input), expected);
        let _rug_ed_tests_llm_16_41_rrrruuuugggg_test_parse_ipv4number_empty_input = 0;
    }
    #[test]
    fn test_parse_ipv4number_start_with_plus() {
        let _rug_st_tests_llm_16_41_rrrruuuugggg_test_parse_ipv4number_start_with_plus = 0;
        let rug_fuzz_0 = "+123";
        let input = rug_fuzz_0;
        let expected = Ok(None);
        debug_assert_eq!(parse_ipv4number(input), expected);
        let _rug_ed_tests_llm_16_41_rrrruuuugggg_test_parse_ipv4number_start_with_plus = 0;
    }
    #[test]
    fn test_parse_ipv4number_parse_error() {
        let _rug_st_tests_llm_16_41_rrrruuuugggg_test_parse_ipv4number_parse_error = 0;
        let rug_fuzz_0 = "123a";
        let input = rug_fuzz_0;
        let expected: Result<Option<u32>, ()> = Err(());
        debug_assert_eq!(parse_ipv4number(input), expected);
        let _rug_ed_tests_llm_16_41_rrrruuuugggg_test_parse_ipv4number_parse_error = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_42 {
    use super::*;
    use crate::*;
    #[test]
    fn test_parse_ipv6addr_valid() {
        let _rug_st_tests_llm_16_42_rrrruuuugggg_test_parse_ipv6addr_valid = 0;
        let rug_fuzz_0 = "2001:0db8:85a3:0000:0000:8a2e:0370:7334";
        let rug_fuzz_1 = "2001:0db8::8a2e:0370:7334";
        let rug_fuzz_2 = "::";
        let rug_fuzz_3 = "::1";
        debug_assert_eq!(
            parse_ipv6addr(rug_fuzz_0), Ok(Ipv6Addr::new(0x2001, 0x0db8, 0x85a3, 0x0000,
            0x0000, 0x8a2e, 0x0370, 0x7334))
        );
        debug_assert_eq!(
            parse_ipv6addr(rug_fuzz_1), Ok(Ipv6Addr::new(0x2001, 0x0db8, 0x0000, 0x0000,
            0x8a2e, 0x0370, 0x7334, 0x0000))
        );
        debug_assert_eq!(
            parse_ipv6addr(rug_fuzz_2), Ok(Ipv6Addr::new(0x0000, 0x0000, 0x0000, 0x0000,
            0x0000, 0x0000, 0x0000, 0x0000))
        );
        debug_assert_eq!(
            parse_ipv6addr(rug_fuzz_3), Ok(Ipv6Addr::new(0x0000, 0x0000, 0x0000, 0x0000,
            0x0000, 0x0000, 0x0000, 0x0001))
        );
        let _rug_ed_tests_llm_16_42_rrrruuuugggg_test_parse_ipv6addr_valid = 0;
    }
    #[test]
    fn test_parse_ipv6addr_invalid() {
        let _rug_st_tests_llm_16_42_rrrruuuugggg_test_parse_ipv6addr_invalid = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = ":::";
        let rug_fuzz_2 = ":1:";
        let rug_fuzz_3 = "::fffff";
        let rug_fuzz_4 = "::1:";
        let rug_fuzz_5 = ":1::1";
        let rug_fuzz_6 = "::1::";
        let rug_fuzz_7 = "2001:db8::8a2e::7334";
        let rug_fuzz_8 = "::2001:0db8:85a3::8a2e:0370:7334";
        let rug_fuzz_9 = "2001:0db8:85a3:::8a2e:0370:7334";
        let rug_fuzz_10 = "::2001:0db8:85a3:0000:0000:8a2e:0370:7334:";
        let rug_fuzz_11 = "::2001:0db8:85a3:0000:0000:8a2e:0370:7334::";
        let rug_fuzz_12 = "2001:0db8:85a3:0000:0000:8a2e:0370:7334:";
        let rug_fuzz_13 = "2001:0db8:85a3:0000:0000:8a2e:0370:7334::";
        let rug_fuzz_14 = "2001:0gb8:85a3:0000:0000:8a2e:0370:7334";
        let rug_fuzz_15 = "2001:0db8:85a3::8a2e::0370:7334";
        let rug_fuzz_16 = "2001:0db8:85a3:0000:0000:8a2e::0370:7334";
        let rug_fuzz_17 = "2001:0db8:85a3:0000:0000:8a2e:0370:7334:";
        let rug_fuzz_18 = "2001:0db8:85a3:0000:0000:8a2e:0370:7334::";
        let rug_fuzz_19 = "2001:0db8:85a3:0000:0000:8a2e:0370:7334:0000";
        let rug_fuzz_20 = "127.0.0.1";
        let rug_fuzz_21 = "2001:0db8:85a3:0000:0000:8a2e:0370:7334:1";
        debug_assert_eq!(
            parse_ipv6addr(rug_fuzz_0), Err(ParseError::InvalidIpv6Address)
        );
        debug_assert_eq!(
            parse_ipv6addr(rug_fuzz_1), Err(ParseError::InvalidIpv6Address)
        );
        debug_assert_eq!(
            parse_ipv6addr(rug_fuzz_2), Err(ParseError::InvalidIpv6Address)
        );
        debug_assert_eq!(
            parse_ipv6addr(rug_fuzz_3), Err(ParseError::InvalidIpv6Address)
        );
        debug_assert_eq!(
            parse_ipv6addr(rug_fuzz_4), Err(ParseError::InvalidIpv6Address)
        );
        debug_assert_eq!(
            parse_ipv6addr(rug_fuzz_5), Err(ParseError::InvalidIpv6Address)
        );
        debug_assert_eq!(
            parse_ipv6addr(rug_fuzz_6), Err(ParseError::InvalidIpv6Address)
        );
        debug_assert_eq!(
            parse_ipv6addr(rug_fuzz_7), Err(ParseError::InvalidIpv6Address)
        );
        debug_assert_eq!(
            parse_ipv6addr(rug_fuzz_8), Err(ParseError::InvalidIpv6Address)
        );
        debug_assert_eq!(
            parse_ipv6addr(rug_fuzz_9), Err(ParseError::InvalidIpv6Address)
        );
        debug_assert_eq!(
            parse_ipv6addr(rug_fuzz_10), Err(ParseError::InvalidIpv6Address)
        );
        debug_assert_eq!(
            parse_ipv6addr(rug_fuzz_11), Err(ParseError::InvalidIpv6Address)
        );
        debug_assert_eq!(
            parse_ipv6addr(rug_fuzz_12), Err(ParseError::InvalidIpv6Address)
        );
        debug_assert_eq!(
            parse_ipv6addr(rug_fuzz_13), Err(ParseError::InvalidIpv6Address)
        );
        debug_assert_eq!(
            parse_ipv6addr(rug_fuzz_14), Err(ParseError::InvalidIpv6Address)
        );
        debug_assert_eq!(
            parse_ipv6addr(rug_fuzz_15), Err(ParseError::InvalidIpv6Address)
        );
        debug_assert_eq!(
            parse_ipv6addr(rug_fuzz_16), Err(ParseError::InvalidIpv6Address)
        );
        debug_assert_eq!(
            parse_ipv6addr(rug_fuzz_17), Err(ParseError::InvalidIpv6Address)
        );
        debug_assert_eq!(
            parse_ipv6addr(rug_fuzz_18), Err(ParseError::InvalidIpv6Address)
        );
        debug_assert_eq!(
            parse_ipv6addr(rug_fuzz_19), Err(ParseError::InvalidIpv6Address)
        );
        debug_assert_eq!(
            parse_ipv6addr(rug_fuzz_20), Err(ParseError::InvalidIpv6Address)
        );
        debug_assert_eq!(
            parse_ipv6addr(rug_fuzz_21), Err(ParseError::InvalidIpv6Address)
        );
        let _rug_ed_tests_llm_16_42_rrrruuuugggg_test_parse_ipv6addr_invalid = 0;
    }
}
#[cfg(test)]
mod tests_rug_10 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_10_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "[::1]";
        let mut p0 = rug_fuzz_0;
        crate::host::Host::parse_opaque(&p0);
        let _rug_ed_tests_rug_10_rrrruuuugggg_test_rug = 0;
    }
}
