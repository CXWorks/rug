use crate::host::Host;
use crate::parser::default_port;
use crate::Url;
use idna::domain_to_unicode;
use std::sync::atomic::{AtomicUsize, Ordering};
pub fn url_origin(url: &Url) -> Origin {
    let scheme = url.scheme();
    match scheme {
        "blob" => {
            let result = Url::parse(url.path());
            match result {
                Ok(ref url) => url_origin(url),
                Err(_) => Origin::new_opaque(),
            }
        }
        "ftp" | "http" | "https" | "ws" | "wss" => {
            Origin::Tuple(
                scheme.to_owned(),
                url.host().unwrap().to_owned(),
                url.port_or_known_default().unwrap(),
            )
        }
        "file" => Origin::new_opaque(),
        _ => Origin::new_opaque(),
    }
}
/// The origin of an URL
///
/// Two URLs with the same origin are considered
/// to originate from the same entity and can therefore trust
/// each other.
///
/// The origin is determined based on the scheme as follows:
///
/// - If the scheme is "blob" the origin is the origin of the
///   URL contained in the path component. If parsing fails,
///   it is an opaque origin.
/// - If the scheme is "ftp", "http", "https", "ws", or "wss",
///   then the origin is a tuple of the scheme, host, and port.
/// - If the scheme is anything else, the origin is opaque, meaning
///   the URL does not have the same origin as any other URL.
///
/// For more information see <https://url.spec.whatwg.org/#origin>
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum Origin {
    /// A globally unique identifier
    Opaque(OpaqueOrigin),
    /// Consists of the URL's scheme, host and port
    Tuple(String, Host<String>, u16),
}
impl Origin {
    /// Creates a new opaque origin that is only equal to itself.
    pub fn new_opaque() -> Origin {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        Origin::Opaque(OpaqueOrigin(COUNTER.fetch_add(1, Ordering::SeqCst)))
    }
    /// Return whether this origin is a (scheme, host, port) tuple
    /// (as opposed to an opaque origin).
    pub fn is_tuple(&self) -> bool {
        matches!(* self, Origin::Tuple(..))
    }
    /// <https://html.spec.whatwg.org/multipage/#ascii-serialisation-of-an-origin>
    pub fn ascii_serialization(&self) -> String {
        match *self {
            Origin::Opaque(_) => "null".to_owned(),
            Origin::Tuple(ref scheme, ref host, port) => {
                if default_port(scheme) == Some(port) {
                    format!("{}://{}", scheme, host)
                } else {
                    format!("{}://{}:{}", scheme, host, port)
                }
            }
        }
    }
    /// <https://html.spec.whatwg.org/multipage/#unicode-serialisation-of-an-origin>
    pub fn unicode_serialization(&self) -> String {
        match *self {
            Origin::Opaque(_) => "null".to_owned(),
            Origin::Tuple(ref scheme, ref host, port) => {
                let host = match *host {
                    Host::Domain(ref domain) => {
                        let (domain, _errors) = domain_to_unicode(domain);
                        Host::Domain(domain)
                    }
                    _ => host.clone(),
                };
                if default_port(scheme) == Some(port) {
                    format!("{}://{}", scheme, host)
                } else {
                    format!("{}://{}:{}", scheme, host, port)
                }
            }
        }
    }
}
/// Opaque identifier for URLs that have file or other schemes
#[derive(Eq, PartialEq, Hash, Clone, Debug)]
pub struct OpaqueOrigin(usize);
#[cfg(test)]
mod tests_llm_16_45 {
    use super::*;
    use crate::*;
    use std::net::Ipv4Addr;
    #[test]
    fn test_ascii_serialization_opaque() {
        let _rug_st_tests_llm_16_45_rrrruuuugggg_test_ascii_serialization_opaque = 0;
        let rug_fuzz_0 = 0;
        let origin = Origin::Opaque(OpaqueOrigin(rug_fuzz_0));
        debug_assert_eq!(origin.ascii_serialization(), "null");
        let _rug_ed_tests_llm_16_45_rrrruuuugggg_test_ascii_serialization_opaque = 0;
    }
    #[test]
    fn test_ascii_serialization_tuple() {
        let _rug_st_tests_llm_16_45_rrrruuuugggg_test_ascii_serialization_tuple = 0;
        let rug_fuzz_0 = "http";
        let rug_fuzz_1 = "example.com";
        let rug_fuzz_2 = 80;
        let scheme = rug_fuzz_0.to_owned();
        let host = Host::Domain(rug_fuzz_1.to_owned());
        let port = rug_fuzz_2;
        let origin = Origin::Tuple(scheme, host, port);
        debug_assert_eq!(origin.ascii_serialization(), "http://example.com");
        let _rug_ed_tests_llm_16_45_rrrruuuugggg_test_ascii_serialization_tuple = 0;
    }
    #[test]
    fn test_ascii_serialization_tuple_with_port() {
        let _rug_st_tests_llm_16_45_rrrruuuugggg_test_ascii_serialization_tuple_with_port = 0;
        let rug_fuzz_0 = "http";
        let rug_fuzz_1 = "example.com";
        let rug_fuzz_2 = 8080;
        let scheme = rug_fuzz_0.to_owned();
        let host = Host::Domain(rug_fuzz_1.to_owned());
        let port = rug_fuzz_2;
        let origin = Origin::Tuple(scheme, host, port);
        debug_assert_eq!(origin.ascii_serialization(), "http://example.com:8080");
        let _rug_ed_tests_llm_16_45_rrrruuuugggg_test_ascii_serialization_tuple_with_port = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_46 {
    use super::*;
    use crate::*;
    use crate::host::Host;
    #[test]
    fn test_is_tuple() {
        let _rug_st_tests_llm_16_46_rrrruuuugggg_test_is_tuple = 0;
        let rug_fuzz_0 = "http";
        let rug_fuzz_1 = "example.com";
        let rug_fuzz_2 = 80;
        let rug_fuzz_3 = 0;
        let origin_tuple = Origin::Tuple(
            rug_fuzz_0.to_string(),
            Host::Domain(rug_fuzz_1.to_string()),
            rug_fuzz_2,
        );
        let origin_opaque = Origin::Opaque(OpaqueOrigin(rug_fuzz_3));
        debug_assert_eq!(origin_tuple.is_tuple(), true);
        debug_assert_eq!(origin_opaque.is_tuple(), false);
        let _rug_ed_tests_llm_16_46_rrrruuuugggg_test_is_tuple = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_47 {
    use super::*;
    use crate::*;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    #[test]
    fn test_new_opaque() {
        let _rug_st_tests_llm_16_47_rrrruuuugggg_test_new_opaque = 0;
        let origin1 = Origin::new_opaque();
        let origin2 = Origin::new_opaque();
        debug_assert_eq!(origin1.is_tuple(), false);
        debug_assert_eq!(origin2.is_tuple(), false);
        debug_assert_eq!(origin1, origin1);
        debug_assert_eq!(origin2, origin2);
        debug_assert_ne!(origin1, origin2);
        let mut hasher1 = DefaultHasher::new();
        let mut hasher2 = DefaultHasher::new();
        origin1.hash(&mut hasher1);
        origin2.hash(&mut hasher2);
        debug_assert_ne!(hasher1.finish(), hasher2.finish());
        let _rug_ed_tests_llm_16_47_rrrruuuugggg_test_new_opaque = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_48 {
    use super::*;
    use crate::*;
    use crate::host::Host;
    use crate::origin::{Origin, OpaqueOrigin};
    #[test]
    fn test_unicode_serialization_opaque_origin() {
        let _rug_st_tests_llm_16_48_rrrruuuugggg_test_unicode_serialization_opaque_origin = 0;
        let rug_fuzz_0 = 0;
        let origin = Origin::Opaque(OpaqueOrigin(rug_fuzz_0));
        let result = origin.unicode_serialization();
        debug_assert_eq!(result, "null");
        let _rug_ed_tests_llm_16_48_rrrruuuugggg_test_unicode_serialization_opaque_origin = 0;
    }
    #[test]
    fn test_unicode_serialization_tuple_origin_with_default_port() {
        let _rug_st_tests_llm_16_48_rrrruuuugggg_test_unicode_serialization_tuple_origin_with_default_port = 0;
        let rug_fuzz_0 = "http";
        let rug_fuzz_1 = "example.com";
        let rug_fuzz_2 = 80;
        let scheme = rug_fuzz_0;
        let host = Host::Domain(rug_fuzz_1.to_owned());
        let port = rug_fuzz_2;
        let origin = Origin::Tuple(scheme.to_owned(), host, port);
        let result = origin.unicode_serialization();
        debug_assert_eq!(result, "http://example.com");
        let _rug_ed_tests_llm_16_48_rrrruuuugggg_test_unicode_serialization_tuple_origin_with_default_port = 0;
    }
    #[test]
    fn test_unicode_serialization_tuple_origin_with_custom_port() {
        let _rug_st_tests_llm_16_48_rrrruuuugggg_test_unicode_serialization_tuple_origin_with_custom_port = 0;
        let rug_fuzz_0 = "http";
        let rug_fuzz_1 = "example.com";
        let rug_fuzz_2 = 8080;
        let scheme = rug_fuzz_0;
        let host = Host::Domain(rug_fuzz_1.to_owned());
        let port = rug_fuzz_2;
        let origin = Origin::Tuple(scheme.to_owned(), host, port);
        let result = origin.unicode_serialization();
        debug_assert_eq!(result, "http://example.com:8080");
        let _rug_ed_tests_llm_16_48_rrrruuuugggg_test_unicode_serialization_tuple_origin_with_custom_port = 0;
    }
}
