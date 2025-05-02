use bytes::Bytes;
use std::{ops, str};
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) struct ByteStr {
    bytes: Bytes,
}
impl ByteStr {
    #[inline]
    pub fn new() -> ByteStr {
        ByteStr { bytes: Bytes::new() }
    }
    #[inline]
    pub fn from_static(val: &'static str) -> ByteStr {
        ByteStr {
            bytes: Bytes::from_static(val.as_bytes()),
        }
    }
    #[inline]
    /// ## Panics
    /// In a debug build this will panic if `bytes` is not valid UTF-8.
    ///
    /// ## Safety
    /// `bytes` must contain valid UTF-8. In a release build it is undefined
    /// behaviour to call this with `bytes` that is not valid UTF-8.
    pub unsafe fn from_utf8_unchecked(bytes: Bytes) -> ByteStr {
        if cfg!(debug_assertions) {
            match str::from_utf8(&bytes) {
                Ok(_) => {}
                Err(err) => {
                    panic!(
                        "ByteStr::from_utf8_unchecked() with invalid bytes; error = {}, bytes = {:?}",
                        err, bytes
                    )
                }
            }
        }
        ByteStr { bytes: bytes }
    }
}
impl ops::Deref for ByteStr {
    type Target = str;
    #[inline]
    fn deref(&self) -> &str {
        let b: &[u8] = self.bytes.as_ref();
        unsafe { str::from_utf8_unchecked(b) }
    }
}
impl From<String> for ByteStr {
    #[inline]
    fn from(src: String) -> ByteStr {
        ByteStr { bytes: Bytes::from(src) }
    }
}
impl<'a> From<&'a str> for ByteStr {
    #[inline]
    fn from(src: &'a str) -> ByteStr {
        ByteStr {
            bytes: Bytes::copy_from_slice(src.as_bytes()),
        }
    }
}
impl From<ByteStr> for Bytes {
    fn from(src: ByteStr) -> Self {
        src.bytes
    }
}
#[cfg(test)]
mod tests_llm_16_34 {
    use super::*;
    use crate::*;
    #[test]
    fn test_from() {
        let _rug_st_tests_llm_16_34_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = "Hello, World!";
        let rug_fuzz_1 = "Hello, World!";
        let src = String::from(rug_fuzz_0);
        let result: ByteStr = src.into();
        let expected = ByteStr {
            bytes: Bytes::from(rug_fuzz_1),
        };
        debug_assert_eq!(result.bytes, expected.bytes);
        let _rug_ed_tests_llm_16_34_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_36 {
    use super::*;
    use crate::*;
    use std::ops::Deref;
    #[test]
    fn test_deref() {
        let _rug_st_tests_llm_16_36_rrrruuuugggg_test_deref = 0;
        let rug_fuzz_0 = "test";
        let byte_str = ByteStr::from(rug_fuzz_0);
        let result = byte_str.deref();
        debug_assert_eq!(result, "test");
        let _rug_ed_tests_llm_16_36_rrrruuuugggg_test_deref = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_314 {
    use super::*;
    use crate::*;
    use crate::byte_str::ByteStr;
    use bytes::Bytes;
    use std::mem;
    #[test]
    fn test_from() {
        let _rug_st_tests_llm_16_314_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = "hello world";
        let src = ByteStr::from(rug_fuzz_0);
        let result: Bytes = byte_str::ByteStr::from(src).into();
        debug_assert_eq!(result, Bytes::from("hello world"));
        let _rug_ed_tests_llm_16_314_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_315 {
    use super::*;
    use crate::*;
    #[test]
    fn test_from_static() {
        let _rug_st_tests_llm_16_315_rrrruuuugggg_test_from_static = 0;
        let rug_fuzz_0 = "hello";
        let val: &'static str = rug_fuzz_0;
        let result = ByteStr::from_static(val);
        debug_assert_eq!(result, ByteStr { bytes : Bytes::from_static(val.as_bytes()) });
        let _rug_ed_tests_llm_16_315_rrrruuuugggg_test_from_static = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_316 {
    use super::*;
    use crate::*;
    use bytes::Bytes;
    #[test]
    #[should_panic(expected = "ByteStr::from_utf8_unchecked() with invalid bytes")]
    fn test_from_utf8_unchecked_panics_invalid_bytes() {
        let bytes = Bytes::from_static(&[0xC3, 0x28]);
        unsafe {
            let _ = ByteStr::from_utf8_unchecked(bytes);
        }
    }
    #[test]
    #[cfg(debug_assertions)]
    fn test_from_utf8_unchecked_valid_debug() {
        let bytes = Bytes::from_static("hello".as_bytes());
        unsafe {
            let _ = ByteStr::from_utf8_unchecked(bytes);
        }
    }
    #[test]
    #[cfg(not(debug_assertions))]
    fn test_from_utf8_unchecked_valid_release() {
        let bytes = Bytes::from_static("hello".as_bytes());
        unsafe {
            let _ = ByteStr::from_utf8_unchecked(bytes);
        }
    }
}
#[cfg(test)]
mod tests_llm_16_317 {
    use crate::byte_str::ByteStr;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_317_rrrruuuugggg_test_new = 0;
        let byte_str = ByteStr::new();
        debug_assert_eq!(byte_str.len(), 0);
        let _rug_ed_tests_llm_16_317_rrrruuuugggg_test_new = 0;
    }
}
