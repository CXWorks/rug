//! Getters and setters for URL components implemented per https://url.spec.whatwg.org/#api
//!
//! Unless you need to be interoperable with web browsers,
//! you probably want to use `Url` method instead.
use crate::parser::{default_port, Context, Input, Parser, SchemeType};
use crate::{Host, ParseError, Position, Url};
/// https://url.spec.whatwg.org/#dom-url-domaintoascii
pub fn domain_to_ascii(domain: &str) -> String {
    match Host::parse(domain) {
        Ok(Host::Domain(domain)) => domain,
        _ => String::new(),
    }
}
/// https://url.spec.whatwg.org/#dom-url-domaintounicode
pub fn domain_to_unicode(domain: &str) -> String {
    match Host::parse(domain) {
        Ok(Host::Domain(ref domain)) => {
            let (unicode, _errors) = idna::domain_to_unicode(domain);
            unicode
        }
        _ => String::new(),
    }
}
/// Getter for https://url.spec.whatwg.org/#dom-url-href
pub fn href(url: &Url) -> &str {
    url.as_str()
}
/// Setter for https://url.spec.whatwg.org/#dom-url-href
pub fn set_href(url: &mut Url, value: &str) -> Result<(), ParseError> {
    *url = Url::parse(value)?;
    Ok(())
}
/// Getter for https://url.spec.whatwg.org/#dom-url-origin
pub fn origin(url: &Url) -> String {
    url.origin().ascii_serialization()
}
/// Getter for https://url.spec.whatwg.org/#dom-url-protocol
#[inline]
pub fn protocol(url: &Url) -> &str {
    &url.as_str()[..url.scheme().len() + ":".len()]
}
/// Setter for https://url.spec.whatwg.org/#dom-url-protocol
pub fn set_protocol(url: &mut Url, mut new_protocol: &str) -> Result<(), ()> {
    if let Some(position) = new_protocol.find(':') {
        new_protocol = &new_protocol[..position];
    }
    url.set_scheme(new_protocol)
}
/// Getter for https://url.spec.whatwg.org/#dom-url-username
#[inline]
pub fn username(url: &Url) -> &str {
    url.username()
}
/// Setter for https://url.spec.whatwg.org/#dom-url-username
pub fn set_username(url: &mut Url, new_username: &str) -> Result<(), ()> {
    url.set_username(new_username)
}
/// Getter for https://url.spec.whatwg.org/#dom-url-password
#[inline]
pub fn password(url: &Url) -> &str {
    url.password().unwrap_or("")
}
/// Setter for https://url.spec.whatwg.org/#dom-url-password
pub fn set_password(url: &mut Url, new_password: &str) -> Result<(), ()> {
    url.set_password(if new_password.is_empty() { None } else { Some(new_password) })
}
/// Getter for https://url.spec.whatwg.org/#dom-url-host
#[inline]
pub fn host(url: &Url) -> &str {
    &url[Position::BeforeHost..Position::AfterPort]
}
/// Setter for https://url.spec.whatwg.org/#dom-url-host
pub fn set_host(url: &mut Url, new_host: &str) -> Result<(), ()> {
    if url.cannot_be_a_base() {
        return Err(());
    }
    let input = Input::no_trim(new_host);
    let host;
    let opt_port;
    {
        let scheme = url.scheme();
        let scheme_type = SchemeType::from(scheme);
        if scheme_type == SchemeType::File && new_host.is_empty() {
            url.set_host_internal(Host::Domain(String::new()), None);
            return Ok(());
        }
        if let Ok((h, remaining)) = Parser::parse_host(input, scheme_type) {
            host = h;
            opt_port = if let Some(remaining) = remaining.split_prefix(':') {
                if remaining.is_empty() {
                    None
                } else {
                    Parser::parse_port(
                            remaining,
                            || default_port(scheme),
                            Context::Setter,
                        )
                        .ok()
                        .map(|(port, _remaining)| port)
                }
            } else {
                None
            };
        } else {
            return Err(());
        }
    }
    if host == Host::Domain("".to_string()) {
        if !username(&url).is_empty() {
            return Err(());
        } else if let Some(Some(_)) = opt_port {
            return Err(());
        } else if url.port().is_some() {
            return Err(());
        }
    }
    url.set_host_internal(host, opt_port);
    Ok(())
}
/// Getter for https://url.spec.whatwg.org/#dom-url-hostname
#[inline]
pub fn hostname(url: &Url) -> &str {
    url.host_str().unwrap_or("")
}
/// Setter for https://url.spec.whatwg.org/#dom-url-hostname
pub fn set_hostname(url: &mut Url, new_hostname: &str) -> Result<(), ()> {
    if url.cannot_be_a_base() {
        return Err(());
    }
    let input = Input::no_trim(new_hostname);
    let scheme_type = SchemeType::from(url.scheme());
    if scheme_type == SchemeType::File && new_hostname.is_empty() {
        url.set_host_internal(Host::Domain(String::new()), None);
        return Ok(());
    }
    if let Ok((host, _remaining)) = Parser::parse_host(input, scheme_type) {
        if let Host::Domain(h) = &host {
            if h.is_empty() {
                if SchemeType::from(url.scheme()) == SchemeType::SpecialNotFile
                    || !port(&url).is_empty() || !url.username().is_empty()
                    || !url.password().unwrap_or(&"").is_empty()
                {
                    return Err(());
                }
            }
        }
        url.set_host_internal(host, None);
        Ok(())
    } else {
        Err(())
    }
}
/// Getter for https://url.spec.whatwg.org/#dom-url-port
#[inline]
pub fn port(url: &Url) -> &str {
    &url[Position::BeforePort..Position::AfterPort]
}
/// Setter for https://url.spec.whatwg.org/#dom-url-port
pub fn set_port(url: &mut Url, new_port: &str) -> Result<(), ()> {
    let result;
    {
        let scheme = url.scheme();
        if !url.has_host() || url.host() == Some(Host::Domain("")) || scheme == "file" {
            return Err(());
        }
        result = Parser::parse_port(
            Input::new(new_port),
            || default_port(scheme),
            Context::Setter,
        );
    }
    if let Ok((new_port, _remaining)) = result {
        url.set_port_internal(new_port);
        Ok(())
    } else {
        Err(())
    }
}
/// Getter for https://url.spec.whatwg.org/#dom-url-pathname
#[inline]
pub fn pathname(url: &Url) -> &str {
    url.path()
}
/// Setter for https://url.spec.whatwg.org/#dom-url-pathname
pub fn set_pathname(url: &mut Url, new_pathname: &str) {
    if url.cannot_be_a_base() {
        return;
    }
    if new_pathname.starts_with('/')
        || (SchemeType::from(url.scheme()).is_special()
            && new_pathname.starts_with('\\'))
    {
        url.set_path(new_pathname)
    } else {
        let mut path_to_set = String::from("/");
        path_to_set.push_str(new_pathname);
        url.set_path(&path_to_set)
    }
}
/// Getter for https://url.spec.whatwg.org/#dom-url-search
pub fn search(url: &Url) -> &str {
    trim(&url[Position::AfterPath..Position::AfterQuery])
}
/// Setter for https://url.spec.whatwg.org/#dom-url-search
pub fn set_search(url: &mut Url, new_search: &str) {
    url.set_query(
        match new_search {
            "" => None,
            _ if new_search.starts_with('?') => Some(&new_search[1..]),
            _ => Some(new_search),
        },
    )
}
/// Getter for https://url.spec.whatwg.org/#dom-url-hash
pub fn hash(url: &Url) -> &str {
    trim(&url[Position::AfterQuery..])
}
/// Setter for https://url.spec.whatwg.org/#dom-url-hash
pub fn set_hash(url: &mut Url, new_hash: &str) {
    url.set_fragment(
        match new_hash {
            "" => None,
            _ if new_hash.starts_with('#') => Some(&new_hash[1..]),
            _ => Some(new_hash),
        },
    )
}
fn trim(s: &str) -> &str {
    if s.len() == 1 { "" } else { s }
}
#[cfg(test)]
mod tests_llm_16_130 {
    use crate::quirks::domain_to_unicode;
    #[test]
    fn test_domain_to_unicode() {
        let _rug_st_tests_llm_16_130_rrrruuuugggg_test_domain_to_unicode = 0;
        let rug_fuzz_0 = "example.com";
        let rug_fuzz_1 = "xn--85x722f.com";
        let rug_fuzz_2 = "foo-bar";
        let rug_fuzz_3 = "127.0.0.1";
        debug_assert_eq!(domain_to_unicode(rug_fuzz_0), "example.com");
        debug_assert_eq!(domain_to_unicode(rug_fuzz_1), "栗林.com");
        debug_assert_eq!(domain_to_unicode(rug_fuzz_2), "");
        debug_assert_eq!(domain_to_unicode(rug_fuzz_3), "");
        let _rug_ed_tests_llm_16_130_rrrruuuugggg_test_domain_to_unicode = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_131 {
    use super::*;
    use crate::*;
    #[test]
    fn test_trim() {
        let _rug_st_tests_llm_16_131_rrrruuuugggg_test_trim = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = "a";
        let rug_fuzz_2 = "ab";
        let rug_fuzz_3 = "abc";
        debug_assert_eq!(trim(rug_fuzz_0), "");
        debug_assert_eq!(trim(rug_fuzz_1), "");
        debug_assert_eq!(trim(rug_fuzz_2), "ab");
        debug_assert_eq!(trim(rug_fuzz_3), "abc");
        let _rug_ed_tests_llm_16_131_rrrruuuugggg_test_trim = 0;
    }
}
#[cfg(test)]
mod tests_rug_44 {
    use super::*;
    #[test]
    fn test_quirks_domain_to_ascii() {
        let _rug_st_tests_rug_44_rrrruuuugggg_test_quirks_domain_to_ascii = 0;
        let rug_fuzz_0 = "http://www.example.com";
        let p0: &str = rug_fuzz_0;
        debug_assert_eq!(crate ::quirks::domain_to_ascii(& p0), "www.example.com");
        let _rug_ed_tests_rug_44_rrrruuuugggg_test_quirks_domain_to_ascii = 0;
    }
}
#[cfg(test)]
mod tests_rug_45 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_45_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https://example.com";
        let p0 = Url::parse(rug_fuzz_0).unwrap();
        crate::quirks::href(&p0);
        let _rug_ed_tests_rug_45_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_46 {
    use super::*;
    use crate::quirks::set_href;
    use crate::Url;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_46_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https://example.com";
        let rug_fuzz_1 = "/path?query=string#fragment";
        let mut p0: Url = Url::parse(rug_fuzz_0).unwrap();
        let p1: &str = rug_fuzz_1;
        set_href(&mut p0, p1).unwrap();
        let _rug_ed_tests_rug_46_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_47 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_47_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "http://www.example.com";
        let mut p0 = Url::parse(rug_fuzz_0).unwrap();
        crate::quirks::origin(&p0);
        let _rug_ed_tests_rug_47_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_48 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_protocol() {
        let _rug_st_tests_rug_48_rrrruuuugggg_test_protocol = 0;
        let rug_fuzz_0 = "https://example.com";
        let p0 = Url::parse(rug_fuzz_0).unwrap();
        debug_assert_eq!(crate ::quirks::protocol(& p0), "https:");
        let _rug_ed_tests_rug_48_rrrruuuugggg_test_protocol = 0;
    }
}
#[cfg(test)]
mod tests_rug_49 {
    use super::*;
    use crate::quirks::set_protocol;
    use crate::Url;
    #[test]
    fn test_set_protocol() {
        let _rug_st_tests_rug_49_rrrruuuugggg_test_set_protocol = 0;
        let rug_fuzz_0 = "http://example.com/path?query#fragment";
        let rug_fuzz_1 = "https";
        let mut p0 = Url::parse(rug_fuzz_0).unwrap();
        let p1 = rug_fuzz_1;
        set_protocol(&mut p0, p1).unwrap();
        let _rug_ed_tests_rug_49_rrrruuuugggg_test_set_protocol = 0;
    }
}
#[cfg(test)]
mod tests_rug_50 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_50_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https://example.com/path/to/resource?query=123#fragment";
        let mut p0: Url = Url::parse(rug_fuzz_0).unwrap();
        crate::quirks::username(&p0);
        let _rug_ed_tests_rug_50_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_51 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_51_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "http://www.example.com";
        let rug_fuzz_1 = "new_username";
        let mut p0 = Url::parse(rug_fuzz_0).unwrap();
        let p1 = rug_fuzz_1;
        crate::quirks::set_username(&mut p0, p1).unwrap();
        let _rug_ed_tests_rug_51_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_52 {
    use super::*;
    use crate::quirks::password;
    use crate::Url;
    #[test]
    fn test_password() {
        let _rug_st_tests_rug_52_rrrruuuugggg_test_password = 0;
        let rug_fuzz_0 = "https://www.example.com";
        let rug_fuzz_1 = "https://user:pass@example.com";
        let rug_fuzz_2 = "https://www.example.com";
        let rug_fuzz_3 = "password";
        let mut p0: Url = Url::parse(rug_fuzz_0).unwrap();
        debug_assert_eq!(password(& p0), "");
        let mut p1: Url = Url::parse(rug_fuzz_1).unwrap();
        debug_assert_eq!(password(& p1), "pass");
        let mut p2: Url = Url::parse(rug_fuzz_2).unwrap();
        p2.set_password(Some(rug_fuzz_3));
        debug_assert_eq!(password(& p2), "password");
        let _rug_ed_tests_rug_52_rrrruuuugggg_test_password = 0;
    }
}
#[cfg(test)]
mod tests_rug_53 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_53_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https://www.example.com";
        let rug_fuzz_1 = "password";
        let mut p0 = Url::parse(rug_fuzz_0).unwrap();
        let mut p1 = rug_fuzz_1;
        crate::quirks::set_password(&mut p0, &p1);
        let _rug_ed_tests_rug_53_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_54 {
    use super::*;
    use crate::Url;
    use crate::quirks::host;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_54_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https://example.com";
        let p0 = Url::parse(rug_fuzz_0).unwrap();
        host(&p0);
        let _rug_ed_tests_rug_54_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_55 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_55_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https://example.com";
        let rug_fuzz_1 = "newhost.com";
        let mut p0 = Url::parse(rug_fuzz_0).unwrap();
        let p1 = rug_fuzz_1;
        crate::quirks::set_host(&mut p0, &p1);
        let _rug_ed_tests_rug_55_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_56 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_56_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https://www.example.com/path/to/page.html?param1=value1&param2=value2#fragment";
        let mut p0: Url = Url::parse(rug_fuzz_0).unwrap();
        crate::quirks::hostname(&p0);
        let _rug_ed_tests_rug_56_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_57 {
    use super::*;
    use crate::quirks::{set_hostname, Url};
    #[test]
    fn test_set_hostname() {
        let _rug_st_tests_rug_57_rrrruuuugggg_test_set_hostname = 0;
        let rug_fuzz_0 = "https://example.com/path";
        let rug_fuzz_1 = "newhostname";
        let mut url = Url::parse(rug_fuzz_0).unwrap();
        let new_hostname = rug_fuzz_1;
        set_hostname(&mut url, new_hostname).unwrap();
        debug_assert_eq!(url.as_str(), "https://newhostname/path");
        let _rug_ed_tests_rug_57_rrrruuuugggg_test_set_hostname = 0;
    }
}
#[cfg(test)]
mod tests_rug_58 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_58_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "http://example.com";
        let mut p0: Url = Url::parse(rug_fuzz_0).unwrap();
        crate::quirks::port(&p0);
        let _rug_ed_tests_rug_58_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_59 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_59_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https://example.com";
        let rug_fuzz_1 = "8080";
        let mut p0 = Url::parse(rug_fuzz_0).unwrap();
        let p1 = rug_fuzz_1;
        crate::quirks::set_port(&mut p0, &p1).unwrap();
        debug_assert_eq!(p0.port(), Some(8080));
        let _rug_ed_tests_rug_59_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_60 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_60_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https://example.com/path?query#fragment";
        let mut p0: Url = Url::parse(rug_fuzz_0).unwrap();
        crate::quirks::pathname(&p0);
        let _rug_ed_tests_rug_60_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_61 {
    use super::*;
    use crate::quirks::set_pathname;
    use crate::Url;
    #[test]
    fn test_set_pathname() {
        let _rug_st_tests_rug_61_rrrruuuugggg_test_set_pathname = 0;
        let rug_fuzz_0 = "http://www.example.com";
        let rug_fuzz_1 = "/new_path";
        let mut url = Url::parse(rug_fuzz_0).unwrap();
        let new_pathname = rug_fuzz_1;
        set_pathname(&mut url, new_pathname);
        let _rug_ed_tests_rug_61_rrrruuuugggg_test_set_pathname = 0;
    }
}
#[cfg(test)]
mod tests_rug_62 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_62_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https://example.com/path?query#fragment";
        let mut p0: Url = Url::parse(rug_fuzz_0).unwrap();
        crate::quirks::search(&p0);
        let _rug_ed_tests_rug_62_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_63 {
    use super::*;
    use crate::quirks::set_search;
    use crate::Url;
    #[test]
    fn test_set_search() {
        let _rug_st_tests_rug_63_rrrruuuugggg_test_set_search = 0;
        let rug_fuzz_0 = "http://example.com/?query=value";
        let rug_fuzz_1 = "new_query=new_value";
        let mut url = Url::parse(rug_fuzz_0).unwrap();
        let new_search = rug_fuzz_1;
        set_search(&mut url, new_search);
        debug_assert_eq!(url.as_str(), "http://example.com/?new_query=new_value");
        let _rug_ed_tests_rug_63_rrrruuuugggg_test_set_search = 0;
    }
}
#[cfg(test)]
mod tests_rug_64 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_hash() {
        let _rug_st_tests_rug_64_rrrruuuugggg_test_hash = 0;
        let rug_fuzz_0 = "https://www.example.com/path?query#fragment";
        let url = Url::parse(rug_fuzz_0).unwrap();
        debug_assert_eq!(hash(& url), "fragment");
        let _rug_ed_tests_rug_64_rrrruuuugggg_test_hash = 0;
    }
}
#[cfg(test)]
mod tests_rug_65 {
    use super::*;
    use crate::Url;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_65_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https://www.example.com";
        let rug_fuzz_1 = "new_hash_value";
        let mut p0 = Url::parse(rug_fuzz_0).unwrap();
        let p1 = rug_fuzz_1;
        crate::quirks::set_hash(&mut p0, p1);
        let _rug_ed_tests_rug_65_rrrruuuugggg_test_rug = 0;
    }
}
