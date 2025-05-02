//! Processing of `data:` URLs according to the Fetch Standard:
//! <https://fetch.spec.whatwg.org/#data-urls>
//! but starting from a string rather than a parsed URL to avoid extra copies.
//!
//! ```rust
//! use data_url::{DataUrl, mime};
//!
//! let url = DataUrl::process("data:,Hello%20World!").unwrap();
//! let (body, fragment) = url.decode_to_vec().unwrap();
//!
//! assert_eq!(url.mime_type().type_, "text");
//! assert_eq!(url.mime_type().subtype, "plain");
//! assert_eq!(url.mime_type().get_parameter("charset"), Some("US-ASCII"));
//! assert_eq!(body, b"Hello World!");
//! assert!(fragment.is_none());
//! ```
#[macro_use]
extern crate matches;
macro_rules! require {
    ($condition:expr) => {
        if !$condition { return None; }
    };
}
pub mod forgiving_base64;
pub mod mime;
pub struct DataUrl<'a> {
    mime_type: mime::Mime,
    base64: bool,
    encoded_body_plus_fragment: &'a str,
}
#[derive(Debug)]
pub enum DataUrlError {
    NotADataUrl,
    NoComma,
}
impl<'a> DataUrl<'a> {
    /// <https://fetch.spec.whatwg.org/#data-url-processor>
    /// but starting from a string rather than a parsed `Url`, to avoid extra string copies.
    pub fn process(input: &'a str) -> Result<Self, DataUrlError> {
        use crate::DataUrlError::*;
        let after_colon = pretend_parse_data_url(input).ok_or(NotADataUrl)?;
        let (from_colon_to_comma, encoded_body_plus_fragment) = find_comma_before_fragment(
                after_colon,
            )
            .ok_or(NoComma)?;
        let (mime_type, base64) = parse_header(from_colon_to_comma);
        Ok(DataUrl {
            mime_type,
            base64,
            encoded_body_plus_fragment,
        })
    }
    pub fn mime_type(&self) -> &mime::Mime {
        &self.mime_type
    }
    /// Streaming-decode the data URL’s body to `write_body_bytes`,
    /// and return the URL’s fragment identifier if it has one.
    pub fn decode<F, E>(
        &self,
        write_body_bytes: F,
    ) -> Result<Option<FragmentIdentifier<'a>>, forgiving_base64::DecodeError<E>>
    where
        F: FnMut(&[u8]) -> Result<(), E>,
    {
        if self.base64 {
            decode_with_base64(self.encoded_body_plus_fragment, write_body_bytes)
        } else {
            decode_without_base64(self.encoded_body_plus_fragment, write_body_bytes)
                .map_err(forgiving_base64::DecodeError::WriteError)
        }
    }
    /// Return the decoded body, and the URL’s fragment identifier if it has one.
    pub fn decode_to_vec(
        &self,
    ) -> Result<
        (Vec<u8>, Option<FragmentIdentifier<'a>>),
        forgiving_base64::InvalidBase64,
    > {
        let mut body = Vec::new();
        let fragment = self
            .decode(|bytes| {
                body.extend_from_slice(bytes);
                Ok(())
            })?;
        Ok((body, fragment))
    }
}
/// The URL’s fragment identifier (after `#`)
pub struct FragmentIdentifier<'a>(&'a str);
impl<'a> FragmentIdentifier<'a> {
    /// Like in a parsed URL
    pub fn to_percent_encoded(&self) -> String {
        let mut string = String::new();
        for byte in self.0.bytes() {
            match byte {
                b'\t' | b'\n' | b'\r' => continue,
                b'\0'..=b' ' | b'"' | b'<' | b'>' | b'`' | b'\x7F'..=b'\xFF' => {
                    percent_encode(byte, &mut string)
                }
                _ => string.push(byte as char),
            }
        }
        string
    }
}
/// Similar to <https://url.spec.whatwg.org/#concept-basic-url-parser>
/// followed by <https://url.spec.whatwg.org/#concept-url-serializer>
///
/// * `None`: not a data URL.
///
/// * `Some(s)`: sort of the result of serialization, except:
///
///   - `data:` prefix removed
///   - The fragment is included
///   - Other components are **not** UTF-8 percent-encoded
///   - ASCII tabs and newlines in the middle are **not** removed
fn pretend_parse_data_url(input: &str) -> Option<&str> {
    let left_trimmed = input.trim_start_matches(|ch| ch <= ' ');
    let mut bytes = left_trimmed.bytes();
    {
        let mut iter = bytes
            .by_ref()
            .filter(|&byte| !matches!(byte, b'\t' | b'\n' | b'\r'));
        require!(iter.next() ?.to_ascii_lowercase() == b'd');
        require!(iter.next() ?.to_ascii_lowercase() == b'a');
        require!(iter.next() ?.to_ascii_lowercase() == b't');
        require!(iter.next() ?.to_ascii_lowercase() == b'a');
        require!(iter.next() ? == b':');
    }
    let bytes_consumed = left_trimmed.len() - bytes.len();
    let after_colon = &left_trimmed[bytes_consumed..];
    Some(after_colon.trim_end_matches(|ch| ch <= ' '))
}
fn find_comma_before_fragment(after_colon: &str) -> Option<(&str, &str)> {
    for (i, byte) in after_colon.bytes().enumerate() {
        if byte == b',' {
            return Some((&after_colon[..i], &after_colon[i + 1..]));
        }
        if byte == b'#' {
            break;
        }
    }
    None
}
fn parse_header(from_colon_to_comma: &str) -> (mime::Mime, bool) {
    let trimmed = from_colon_to_comma
        .trim_matches(|c| matches!(c, ' ' | '\t' | '\n' | '\r'));
    let without_base64_suffix = remove_base64_suffix(trimmed);
    let base64 = without_base64_suffix.is_some();
    let mime_type = without_base64_suffix.unwrap_or(trimmed);
    let mut string = String::new();
    if mime_type.starts_with(';') {
        string.push_str("text/plain")
    }
    let mut in_query = false;
    for byte in mime_type.bytes() {
        match byte {
            b'\t' | b'\n' | b'\r' => continue,
            b'\0'..=b'\x1F' | b'\x7F'..=b'\xFF' => percent_encode(byte, &mut string),
            b' ' | b'"' | b'<' | b'>' if in_query => percent_encode(byte, &mut string),
            b'?' => {
                in_query = true;
                string.push('?')
            }
            _ => string.push(byte as char),
        }
    }
    let mime_type = string
        .parse()
        .unwrap_or_else(|_| mime::Mime {
            type_: String::from("text"),
            subtype: String::from("plain"),
            parameters: vec![(String::from("charset"), String::from("US-ASCII"))],
        });
    (mime_type, base64)
}
/// None: no base64 suffix
#[allow(clippy::skip_while_next)]
fn remove_base64_suffix(s: &str) -> Option<&str> {
    let mut bytes = s.bytes();
    {
        let iter = bytes.by_ref().filter(|&byte| !matches!(byte, b'\t' | b'\n' | b'\r'));
        let mut iter = iter.rev();
        require!(iter.next() ? == b'4');
        require!(iter.next() ? == b'6');
        require!(iter.next() ?.to_ascii_lowercase() == b'e');
        require!(iter.next() ?.to_ascii_lowercase() == b's');
        require!(iter.next() ?.to_ascii_lowercase() == b'a');
        require!(iter.next() ?.to_ascii_lowercase() == b'b');
        require!(iter.skip_while(|& byte | byte == b' ').next() ? == b';');
    }
    Some(&s[..bytes.len()])
}
fn percent_encode(byte: u8, string: &mut String) {
    const HEX_UPPER: [u8; 16] = *b"0123456789ABCDEF";
    string.push('%');
    string.push(HEX_UPPER[(byte >> 4) as usize] as char);
    string.push(HEX_UPPER[(byte & 0x0f) as usize] as char);
}
/// This is <https://url.spec.whatwg.org/#string-percent-decode> while also:
///
/// * Ignoring ASCII tab or newlines
/// * Stopping at the first '#' (which indicates the start of the fragment)
///
/// Anything that would have been UTF-8 percent-encoded by the URL parser
/// would be percent-decoded here.
/// We skip that round-trip and pass it through unchanged.
fn decode_without_base64<F, E>(
    encoded_body_plus_fragment: &str,
    mut write_bytes: F,
) -> Result<Option<FragmentIdentifier<'_>>, E>
where
    F: FnMut(&[u8]) -> Result<(), E>,
{
    let bytes = encoded_body_plus_fragment.as_bytes();
    let mut slice_start = 0;
    for (i, &byte) in bytes.iter().enumerate() {
        if matches!(byte, b'%' | b'#' | b'\t' | b'\n' | b'\r') {
            if i > slice_start {
                write_bytes(&bytes[slice_start..i])?;
            }
            match byte {
                b'%' => {
                    let l = bytes.get(i + 2).and_then(|&b| (b as char).to_digit(16));
                    let h = bytes.get(i + 1).and_then(|&b| (b as char).to_digit(16));
                    if let (Some(h), Some(l)) = (h, l) {
                        let one_byte = h as u8 * 0x10 + l as u8;
                        write_bytes(&[one_byte])?;
                        slice_start = i + 3;
                    } else {}
                }
                b'#' => {
                    let fragment_start = i + 1;
                    let fragment = &encoded_body_plus_fragment[fragment_start..];
                    return Ok(Some(FragmentIdentifier(fragment)));
                }
                _ => slice_start = i + 1,
            }
        }
    }
    write_bytes(&bytes[slice_start..])?;
    Ok(None)
}
/// `decode_without_base64()` composed with
/// <https://infra.spec.whatwg.org/#isomorphic-decode> composed with
/// <https://infra.spec.whatwg.org/#forgiving-base64-decode>.
fn decode_with_base64<F, E>(
    encoded_body_plus_fragment: &str,
    write_bytes: F,
) -> Result<Option<FragmentIdentifier<'_>>, forgiving_base64::DecodeError<E>>
where
    F: FnMut(&[u8]) -> Result<(), E>,
{
    let mut decoder = forgiving_base64::Decoder::new(write_bytes);
    let fragment = decode_without_base64(
        encoded_body_plus_fragment,
        |bytes| decoder.feed(bytes),
    )?;
    decoder.finish()?;
    Ok(fragment)
}
#[cfg(test)]
mod tests_llm_16_4 {
    use super::*;
    use crate::*;
    use forgiving_base64::DecodeError;
    #[test]
    fn test_decode_with_base64() {
        let _rug_st_tests_llm_16_4_rrrruuuugggg_test_decode_with_base64 = 0;
        let rug_fuzz_0 = "text";
        let rug_fuzz_1 = "plain";
        let rug_fuzz_2 = "charset";
        let rug_fuzz_3 = "UTF-8";
        let rug_fuzz_4 = true;
        let rug_fuzz_5 = "SGVsbG8gd29ybGQh";
        let data_url = DataUrl {
            mime_type: mime::Mime {
                type_: rug_fuzz_0.to_string(),
                subtype: rug_fuzz_1.to_string(),
                parameters: vec![(rug_fuzz_2.to_string(), rug_fuzz_3.to_string())],
            },
            base64: rug_fuzz_4,
            encoded_body_plus_fragment: rug_fuzz_5,
        };
        let result = data_url
            .decode(|bytes| {
                debug_assert_eq!(bytes, "Hello world!".as_bytes());
                Ok(())
            });
        debug_assert!(result.is_ok());
        debug_assert_eq!(result.unwrap(), None);
        let _rug_ed_tests_llm_16_4_rrrruuuugggg_test_decode_with_base64 = 0;
    }
    #[test]
    fn test_decode_without_base64() {
        let _rug_st_tests_llm_16_4_rrrruuuugggg_test_decode_without_base64 = 0;
        let rug_fuzz_0 = "text";
        let rug_fuzz_1 = "plain";
        let rug_fuzz_2 = "charset";
        let rug_fuzz_3 = "UTF-8";
        let rug_fuzz_4 = false;
        let rug_fuzz_5 = "Hello world!";
        let data_url = DataUrl {
            mime_type: mime::Mime {
                type_: rug_fuzz_0.to_string(),
                subtype: rug_fuzz_1.to_string(),
                parameters: vec![(rug_fuzz_2.to_string(), rug_fuzz_3.to_string())],
            },
            base64: rug_fuzz_4,
            encoded_body_plus_fragment: rug_fuzz_5,
        };
        let result = data_url
            .decode(|bytes| {
                debug_assert_eq!(bytes, "Hello world!".as_bytes());
                Ok(())
            });
        debug_assert!(result.is_ok());
        debug_assert_eq!(result.unwrap(), None);
        let _rug_ed_tests_llm_16_4_rrrruuuugggg_test_decode_without_base64 = 0;
    }
    #[test]
    fn test_decode_to_vec() {
        let _rug_st_tests_llm_16_4_rrrruuuugggg_test_decode_to_vec = 0;
        let rug_fuzz_0 = "text";
        let rug_fuzz_1 = "plain";
        let rug_fuzz_2 = "charset";
        let rug_fuzz_3 = "UTF-8";
        let rug_fuzz_4 = false;
        let rug_fuzz_5 = "Hello world!";
        let data_url = DataUrl {
            mime_type: mime::Mime {
                type_: rug_fuzz_0.to_string(),
                subtype: rug_fuzz_1.to_string(),
                parameters: vec![(rug_fuzz_2.to_string(), rug_fuzz_3.to_string())],
            },
            base64: rug_fuzz_4,
            encoded_body_plus_fragment: rug_fuzz_5,
        };
        let result = data_url.decode_to_vec();
        debug_assert!(result.is_ok());
        debug_assert_eq!(result.unwrap(), (b"Hello world!".to_vec(), None));
        let _rug_ed_tests_llm_16_4_rrrruuuugggg_test_decode_to_vec = 0;
    }
    #[test]
    fn test_decode_to_vec_with_fragment() {
        let _rug_st_tests_llm_16_4_rrrruuuugggg_test_decode_to_vec_with_fragment = 0;
        let rug_fuzz_0 = "text";
        let rug_fuzz_1 = "plain";
        let rug_fuzz_2 = "charset";
        let rug_fuzz_3 = "UTF-8";
        let rug_fuzz_4 = false;
        let rug_fuzz_5 = "Hello world!#fragment";
        let data_url = DataUrl {
            mime_type: mime::Mime {
                type_: rug_fuzz_0.to_string(),
                subtype: rug_fuzz_1.to_string(),
                parameters: vec![(rug_fuzz_2.to_string(), rug_fuzz_3.to_string())],
            },
            base64: rug_fuzz_4,
            encoded_body_plus_fragment: rug_fuzz_5,
        };
        let result = data_url.decode_to_vec();
        debug_assert!(result.is_ok());
        debug_assert_eq!(
            result.unwrap(), (b"Hello world!".to_vec(), Some("fragment".to_string()))
        );
        let _rug_ed_tests_llm_16_4_rrrruuuugggg_test_decode_to_vec_with_fragment = 0;
    }
    #[test]
    fn test_decode_with_base64_error() {
        let _rug_st_tests_llm_16_4_rrrruuuugggg_test_decode_with_base64_error = 0;
        let rug_fuzz_0 = "text";
        let rug_fuzz_1 = "plain";
        let rug_fuzz_2 = "charset";
        let rug_fuzz_3 = "UTF-8";
        let rug_fuzz_4 = true;
        let rug_fuzz_5 = "SGVsbG8gd29ybGQh";
        let data_url = DataUrl {
            mime_type: mime::Mime {
                type_: rug_fuzz_0.to_string(),
                subtype: rug_fuzz_1.to_string(),
                parameters: vec![(rug_fuzz_2.to_string(), rug_fuzz_3.to_string())],
            },
            base64: rug_fuzz_4,
            encoded_body_plus_fragment: rug_fuzz_5,
        };
        let result = data_url.decode(|_| Err(DecodeError::InvalidCharacter));
        debug_assert!(result.is_err());
        debug_assert_eq!(result.unwrap_err(), DecodeError::InvalidCharacter);
        let _rug_ed_tests_llm_16_4_rrrruuuugggg_test_decode_with_base64_error = 0;
    }
    #[test]
    fn test_decode_without_base64_error() {
        let _rug_st_tests_llm_16_4_rrrruuuugggg_test_decode_without_base64_error = 0;
        let rug_fuzz_0 = "text";
        let rug_fuzz_1 = "plain";
        let rug_fuzz_2 = "charset";
        let rug_fuzz_3 = "UTF-8";
        let rug_fuzz_4 = false;
        let rug_fuzz_5 = "Hello world!";
        let data_url = DataUrl {
            mime_type: mime::Mime {
                type_: rug_fuzz_0.to_string(),
                subtype: rug_fuzz_1.to_string(),
                parameters: vec![(rug_fuzz_2.to_string(), rug_fuzz_3.to_string())],
            },
            base64: rug_fuzz_4,
            encoded_body_plus_fragment: rug_fuzz_5,
        };
        let result = data_url.decode(|_| Err(DecodeError::WriteError));
        debug_assert!(result.is_err());
        debug_assert_eq!(result.unwrap_err(), DecodeError::WriteError);
        let _rug_ed_tests_llm_16_4_rrrruuuugggg_test_decode_without_base64_error = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_5 {
    use super::*;
    use crate::*;
    #[test]
    fn test_decode_to_vec() {
        let _rug_st_tests_llm_16_5_rrrruuuugggg_test_decode_to_vec = 0;
        let rug_fuzz_0 = "text";
        let rug_fuzz_1 = "plain";
        let rug_fuzz_2 = false;
        let rug_fuzz_3 = "SGVsbG8gd29ybGQ=";
        let data_url = DataUrl {
            mime_type: Mime {
                type_: rug_fuzz_0.to_string(),
                subtype: rug_fuzz_1.to_string(),
                parameters: vec![],
            },
            base64: rug_fuzz_2,
            encoded_body_plus_fragment: rug_fuzz_3,
        };
        let result = data_url.decode_to_vec();
        debug_assert!(result.is_ok());
        let (body, fragment) = result.unwrap();
        debug_assert_eq!(body, b"Hello world");
        debug_assert_eq!(fragment, None);
        let _rug_ed_tests_llm_16_5_rrrruuuugggg_test_decode_to_vec = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_6 {
    use crate::data_url::DataUrl;
    use std::str::FromStr;
    #[test]
    fn test_mime_type() {
        let _rug_st_tests_llm_16_6_rrrruuuugggg_test_mime_type = 0;
        let rug_fuzz_0 = "data:text/plain;base64,SGVsbG8sIHdvcmxkIQ==";
        let rug_fuzz_1 = "text/plain";
        let data_url = DataUrl::process(rug_fuzz_0).unwrap();
        let mime_type = data_url.mime_type();
        let expected_mime_type = mime::Mime::from_str(rug_fuzz_1).unwrap();
        debug_assert_eq!(* mime_type, expected_mime_type);
        let _rug_ed_tests_llm_16_6_rrrruuuugggg_test_mime_type = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_7 {
    use super::*;
    use crate::*;
    #[test]
    fn test_process_valid_data_url() {
        let _rug_st_tests_llm_16_7_rrrruuuugggg_test_process_valid_data_url = 0;
        let rug_fuzz_0 = "data:text/plain;base64,SGVsbG8gd29ybGQ=";
        let input = rug_fuzz_0;
        let result = process(input);
        debug_assert!(result.is_ok());
        let data_url = result.unwrap();
        debug_assert_eq!(data_url.mime_type, "text/plain");
        debug_assert_eq!(data_url.base64, true);
        debug_assert_eq!(data_url.encoded_body_plus_fragment, "SGVsbG8gd29ybGQ=");
        let _rug_ed_tests_llm_16_7_rrrruuuugggg_test_process_valid_data_url = 0;
    }
    #[test]
    fn test_process_invalid_data_url() {
        let _rug_st_tests_llm_16_7_rrrruuuugggg_test_process_invalid_data_url = 0;
        let rug_fuzz_0 = "https://example.com";
        let input = rug_fuzz_0;
        let result = process(input);
        debug_assert!(result.is_err());
        debug_assert_eq!(result.unwrap_err(), DataUrlError::NotADataUrl);
        let _rug_ed_tests_llm_16_7_rrrruuuugggg_test_process_invalid_data_url = 0;
    }
    #[test]
    fn test_process_data_url_without_comma() {
        let _rug_st_tests_llm_16_7_rrrruuuugggg_test_process_data_url_without_comma = 0;
        let rug_fuzz_0 = "data:text/plain;base64";
        let input = rug_fuzz_0;
        let result = process(input);
        debug_assert!(result.is_err());
        debug_assert_eq!(result.unwrap_err(), DataUrlError::NoComma);
        let _rug_ed_tests_llm_16_7_rrrruuuugggg_test_process_data_url_without_comma = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_8 {
    use super::*;
    use crate::*;
    use url::percent_encoding::percent_encode;
    #[test]
    fn test_to_percent_encoded() {
        let _rug_st_tests_llm_16_8_rrrruuuugggg_test_to_percent_encoded = 0;
        let rug_fuzz_0 = "test#123";
        let fragment = FragmentIdentifier(rug_fuzz_0);
        let encoded = fragment.to_percent_encoded();
        debug_assert_eq!(encoded, "test#123");
        let _rug_ed_tests_llm_16_8_rrrruuuugggg_test_to_percent_encoded = 0;
    }
    #[test]
    fn test_to_percent_encoded_special_characters() {
        let _rug_st_tests_llm_16_8_rrrruuuugggg_test_to_percent_encoded_special_characters = 0;
        let rug_fuzz_0 = "test#<>&";
        let fragment = FragmentIdentifier(rug_fuzz_0);
        let encoded = fragment.to_percent_encoded();
        debug_assert_eq!(encoded, "test%23%3C%3E%26");
        let _rug_ed_tests_llm_16_8_rrrruuuugggg_test_to_percent_encoded_special_characters = 0;
    }
    #[test]
    fn test_to_percent_encoded_ascii_control_characters() {
        let _rug_st_tests_llm_16_8_rrrruuuugggg_test_to_percent_encoded_ascii_control_characters = 0;
        let rug_fuzz_0 = "test#\t\r";
        let fragment = FragmentIdentifier(rug_fuzz_0);
        let encoded = fragment.to_percent_encoded();
        debug_assert_eq!(encoded, "test#");
        let _rug_ed_tests_llm_16_8_rrrruuuugggg_test_to_percent_encoded_ascii_control_characters = 0;
    }
    #[test]
    fn test_to_percent_encoded_extended_ascii() {
        let _rug_st_tests_llm_16_8_rrrruuuugggg_test_to_percent_encoded_extended_ascii = 0;
        let rug_fuzz_0 = "test#ä";
        let fragment = FragmentIdentifier(rug_fuzz_0);
        let encoded = fragment.to_percent_encoded();
        debug_assert_eq!(encoded, "test#%C3%A4");
        let _rug_ed_tests_llm_16_8_rrrruuuugggg_test_to_percent_encoded_extended_ascii = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_9 {
    use super::*;
    use crate::*;
    #[test]
    fn test_decode_with_base64() {
        let _rug_st_tests_llm_16_9_rrrruuuugggg_test_decode_with_base64 = 0;
        let rug_fuzz_0 = "SGVsbG8gd29ybGQ#";
        let encoded_body_plus_fragment = rug_fuzz_0;
        let write_bytes = |_: &[u8]| -> Result<(), Error> { Ok(()) };
        let result = decode_with_base64(encoded_body_plus_fragment, write_bytes);
        debug_assert_eq!(result, Ok(None));
        let _rug_ed_tests_llm_16_9_rrrruuuugggg_test_decode_with_base64 = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_10 {
    use super::*;
    use crate::*;
    #[test]
    fn test_decode_without_base64() {
        let _rug_st_tests_llm_16_10_rrrruuuugggg_test_decode_without_base64 = 0;
        let rug_fuzz_0 = "Hello%20World%21%23%25%5E%2A%28%29%5B%5D%7B%7D%7E%60%3B%2F%3F%3A%40%26%3D%2B%24%2C%20%7C%3C%3E";
        let mut buffer: Vec<u8> = Vec::new();
        let result = decode_without_base64(
            rug_fuzz_0,
            |data| {
                buffer.extend_from_slice(data);
                Ok(())
            },
        );
        let expected = Ok(None);
        debug_assert_eq!(result, expected);
        debug_assert_eq!(buffer, b"Hello World!#%^*()[]{}~`;/?:@&=+$, |<>");
        let _rug_ed_tests_llm_16_10_rrrruuuugggg_test_decode_without_base64 = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_11 {
    use super::*;
    use crate::*;
    #[test]
    fn test_find_comma_before_fragment() {
        let _rug_st_tests_llm_16_11_rrrruuuugggg_test_find_comma_before_fragment = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = "#fragment";
        let rug_fuzz_2 = "data:text/plain,Hello,World";
        let rug_fuzz_3 = "data:text/plain,Hello,World#fragment";
        let rug_fuzz_4 = "data:text/plain,Hello,World,And,More,Data";
        debug_assert_eq!(find_comma_before_fragment(rug_fuzz_0), None);
        debug_assert_eq!(find_comma_before_fragment(rug_fuzz_1), None);
        debug_assert_eq!(
            find_comma_before_fragment(rug_fuzz_2), Some(("data:text/plain",
            "Hello,World"))
        );
        debug_assert_eq!(
            find_comma_before_fragment(rug_fuzz_3), Some(("data:text/plain",
            "Hello,World#fragment"))
        );
        debug_assert_eq!(
            find_comma_before_fragment(rug_fuzz_4), Some(("data:text/plain",
            "Hello,World,And,More,Data"))
        );
        let _rug_ed_tests_llm_16_11_rrrruuuugggg_test_find_comma_before_fragment = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_24 {
    use super::*;
    use crate::*;
    use mime::Mime;
    #[test]
    fn test_parse_header() {
        let _rug_st_tests_llm_16_24_rrrruuuugggg_test_parse_header = 0;
        let rug_fuzz_0 = "text/plain; charset=utf-8";
        let rug_fuzz_1 = "image/png; base64";
        let rug_fuzz_2 = "text/plain";
        let (mime_type, base64) = parse_header(rug_fuzz_0);
        debug_assert_eq!(
            mime_type, Mime { type_ : String::from("text"), subtype :
            String::from("plain"), parameters : vec![(String::from("charset"),
            String::from("utf-8"))] }
        );
        debug_assert_eq!(base64, false);
        let (mime_type, base64) = parse_header(rug_fuzz_1);
        debug_assert_eq!(
            mime_type, Mime { type_ : String::from("image"), subtype :
            String::from("png"), parameters : vec![] }
        );
        debug_assert_eq!(base64, true);
        let (mime_type, base64) = parse_header(rug_fuzz_2);
        debug_assert_eq!(
            mime_type, Mime { type_ : String::from("text"), subtype :
            String::from("plain"), parameters : vec![] }
        );
        debug_assert_eq!(base64, false);
        let _rug_ed_tests_llm_16_24_rrrruuuugggg_test_parse_header = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_25 {
    use crate::percent_encode;
    #[test]
    fn test_percent_encode() {
        let _rug_st_tests_llm_16_25_rrrruuuugggg_test_percent_encode = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 27;
        let rug_fuzz_2 = 255;
        let mut string = String::new();
        percent_encode(rug_fuzz_0, &mut string);
        debug_assert_eq!(string, "%0A");
        string = String::new();
        percent_encode(rug_fuzz_1, &mut string);
        debug_assert_eq!(string, "%1B");
        string = String::new();
        percent_encode(rug_fuzz_2, &mut string);
        debug_assert_eq!(string, "%FF");
        let _rug_ed_tests_llm_16_25_rrrruuuugggg_test_percent_encode = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_27_llm_16_26 {
    use super::*;
    use crate::*;
    use crate::*;
    #[test]
    fn test_pretend_parse_data_url() {
        let _rug_st_tests_llm_16_27_llm_16_26_rrrruuuugggg_test_pretend_parse_data_url = 0;
        let rug_fuzz_0 = "data:text/plain;charset=utf-8,Hello%20World%21#test";
        let rug_fuzz_1 = "data:;base64,SGVsbG8gV29ybGQgKlQrIQ==";
        let rug_fuzz_2 = "data:application/pdf,%25PDF-1.3%0A%25%C3%8E%C3%8F%C2%0A3%20%20%20%20%20%20%20%20%20%20%20%2018%20obj%0A%20%20%20%20%3E%3E%0A";
        let rug_fuzz_3 = "data:application/octet-stream;base64,SGVsbG8gV29ybGQgKlQrIQ==#test";
        let rug_fuzz_4 = "data:dummy";
        let rug_fuzz_5 = "data:Hello";
        debug_assert_eq!(
            pretend_parse_data_url(rug_fuzz_0),
            Some("text/plain;charset=utf-8,Hello%20World%21#test")
        );
        debug_assert_eq!(
            pretend_parse_data_url(rug_fuzz_1), Some(";base64,SGVsbG8gV29ybGQgKlQrIQ==")
        );
        debug_assert_eq!(
            pretend_parse_data_url(rug_fuzz_2),
            Some("application/pdf,%25PDF-1.3%0A%25%C3%8E%C3%8F%C2%0A3%20%20%20%20%20%20%20%20%20%20%20%2018%20obj%0A%20%20%20%20%3E%3E%0A")
        );
        debug_assert_eq!(
            pretend_parse_data_url(rug_fuzz_3),
            Some("application/octet-stream;base64,SGVsbG8gV29ybGQgKlQrIQ==#test")
        );
        debug_assert_eq!(pretend_parse_data_url(rug_fuzz_4), None);
        debug_assert_eq!(pretend_parse_data_url(rug_fuzz_5), Some("Hello"));
        let _rug_ed_tests_llm_16_27_llm_16_26_rrrruuuugggg_test_pretend_parse_data_url = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_28 {
    use super::*;
    use crate::*;
    #[test]
    fn test_remove_base64_suffix_none() {
        let _rug_st_tests_llm_16_28_rrrruuuugggg_test_remove_base64_suffix_none = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = "data:;base64,";
        let rug_fuzz_2 = "data:;64esa;";
        debug_assert_eq!(remove_base64_suffix(rug_fuzz_0), None);
        debug_assert_eq!(remove_base64_suffix(rug_fuzz_1), None);
        debug_assert_eq!(remove_base64_suffix(rug_fuzz_2), None);
        let _rug_ed_tests_llm_16_28_rrrruuuugggg_test_remove_base64_suffix_none = 0;
    }
    #[test]
    fn test_remove_base64_suffix() {
        let _rug_st_tests_llm_16_28_rrrruuuugggg_test_remove_base64_suffix = 0;
        let rug_fuzz_0 = "data:;base64";
        let rug_fuzz_1 = "data:;base64,";
        let rug_fuzz_2 = "data:;base64,z";
        let rug_fuzz_3 = "data:;base64,ze";
        let rug_fuzz_4 = "data:;base64,ze=";
        let rug_fuzz_5 = "data:;base64,ze==";
        let rug_fuzz_6 = "data:;base64,ze== ";
        let rug_fuzz_7 = "data:;base64, ze==";
        let rug_fuzz_8 = "data:;base64,  ze==  ";
        debug_assert_eq!(remove_base64_suffix(rug_fuzz_0), None);
        debug_assert_eq!(remove_base64_suffix(rug_fuzz_1), None);
        debug_assert_eq!(remove_base64_suffix(rug_fuzz_2), None);
        debug_assert_eq!(remove_base64_suffix(rug_fuzz_3), None);
        debug_assert_eq!(remove_base64_suffix(rug_fuzz_4), None);
        debug_assert_eq!(remove_base64_suffix(rug_fuzz_5), Some("data:;base64,ze=="));
        debug_assert_eq!(remove_base64_suffix(rug_fuzz_6), Some("data:;base64,ze== "));
        debug_assert_eq!(remove_base64_suffix(rug_fuzz_7), Some("data:;base64, ze=="));
        debug_assert_eq!(
            remove_base64_suffix(rug_fuzz_8), Some("data:;base64,  ze==  ")
        );
        let _rug_ed_tests_llm_16_28_rrrruuuugggg_test_remove_base64_suffix = 0;
    }
}
