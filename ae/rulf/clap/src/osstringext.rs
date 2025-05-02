use std::ffi::OsStr;
#[cfg(not(any(target_os = "windows", target_arch = "wasm32")))]
use std::os::unix::ffi::OsStrExt;
#[cfg(any(target_os = "windows", target_arch = "wasm32"))]
use INVALID_UTF8;
#[cfg(any(target_os = "windows", target_arch = "wasm32"))]
pub trait OsStrExt3 {
    fn from_bytes(b: &[u8]) -> &Self;
    fn as_bytes(&self) -> &[u8];
}
#[doc(hidden)]
pub trait OsStrExt2 {
    fn starts_with(&self, s: &[u8]) -> bool;
    fn split_at_byte(&self, b: u8) -> (&OsStr, &OsStr);
    fn split_at(&self, i: usize) -> (&OsStr, &OsStr);
    fn trim_left_matches(&self, b: u8) -> &OsStr;
    fn contains_byte(&self, b: u8) -> bool;
    fn split(&self, b: u8) -> OsSplit;
}
#[cfg(target_os = "windows")]
fn windows_osstr_starts_with(osstr: &OsStr, prefix: &[u8]) -> bool {
    use std::os::windows::ffi::OsStrExt;
    let prefix_str = if let Ok(s) = std::str::from_utf8(prefix) {
        s
    } else {
        return false;
    };
    let mut osstr_units = osstr.encode_wide();
    let mut prefix_units = prefix_str.encode_utf16();
    loop {
        match (osstr_units.next(), prefix_units.next()) {
            (Some(o), Some(p)) if o == p => continue,
            (_, None) => return true,
            _ => return false,
        }
    }
}
#[test]
#[cfg(target_os = "windows")]
fn test_windows_osstr_starts_with() {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    fn from_ascii(ascii: &[u8]) -> OsString {
        let u16_vec: Vec<u16> = ascii.iter().map(|&c| c as u16).collect();
        OsString::from_wide(&u16_vec)
    }
    assert!(windows_osstr_starts_with(& from_ascii(b"abcdef"), b"abc"));
    assert!(windows_osstr_starts_with(& from_ascii(b"abcdef"), b"abcdef"));
    assert!(! windows_osstr_starts_with(& from_ascii(b"abcdef"), b"def"));
    assert!(! windows_osstr_starts_with(& from_ascii(b"abc"), b"abcd"));
    assert!(! windows_osstr_starts_with(& from_ascii(b"\xff"), b"\xff"));
    let surrogate_char: u16 = 0xDC00;
    let mut invalid_unicode = OsString::from_wide(
        &['a' as u16, 'b' as u16, 'c' as u16, surrogate_char],
    );
    assert!(
        invalid_unicode.to_str().is_none(),
        "This string is invalid Unicode, and conversion to &str should fail.",
    );
    assert!(windows_osstr_starts_with(& invalid_unicode, b"abc"));
    assert!(! windows_osstr_starts_with(& invalid_unicode, b"abcd"));
}
#[cfg(any(target_os = "windows", target_arch = "wasm32"))]
impl OsStrExt3 for OsStr {
    fn from_bytes(b: &[u8]) -> &Self {
        use std::mem;
        unsafe { mem::transmute(b) }
    }
    fn as_bytes(&self) -> &[u8] {
        self.to_str().map(|s| s.as_bytes()).expect(INVALID_UTF8)
    }
}
impl OsStrExt2 for OsStr {
    fn starts_with(&self, s: &[u8]) -> bool {
        #[cfg(target_os = "windows")]
        {
            return windows_osstr_starts_with(self, s);
        }
        self.as_bytes().starts_with(s)
    }
    fn contains_byte(&self, byte: u8) -> bool {
        for b in self.as_bytes() {
            if b == &byte {
                return true;
            }
        }
        false
    }
    fn split_at_byte(&self, byte: u8) -> (&OsStr, &OsStr) {
        for (i, b) in self.as_bytes().iter().enumerate() {
            if b == &byte {
                return (
                    OsStr::from_bytes(&self.as_bytes()[..i]),
                    OsStr::from_bytes(&self.as_bytes()[i + 1..]),
                );
            }
        }
        (&*self, OsStr::from_bytes(&self.as_bytes()[self.len()..self.len()]))
    }
    fn trim_left_matches(&self, byte: u8) -> &OsStr {
        let mut found = false;
        for (i, b) in self.as_bytes().iter().enumerate() {
            if b != &byte {
                return OsStr::from_bytes(&self.as_bytes()[i..]);
            } else {
                found = true;
            }
        }
        if found {
            return OsStr::from_bytes(&self.as_bytes()[self.len()..]);
        }
        &*self
    }
    fn split_at(&self, i: usize) -> (&OsStr, &OsStr) {
        (
            OsStr::from_bytes(&self.as_bytes()[..i]),
            OsStr::from_bytes(&self.as_bytes()[i..]),
        )
    }
    fn split(&self, b: u8) -> OsSplit {
        OsSplit {
            sep: b,
            val: self.as_bytes(),
            pos: 0,
        }
    }
}
#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct OsSplit<'a> {
    sep: u8,
    val: &'a [u8],
    pos: usize,
}
impl<'a> Iterator for OsSplit<'a> {
    type Item = &'a OsStr;
    fn next(&mut self) -> Option<&'a OsStr> {
        debugln!("OsSplit::next: self={:?}", self);
        if self.pos == self.val.len() {
            return None;
        }
        let start = self.pos;
        for b in &self.val[start..] {
            self.pos += 1;
            if *b == self.sep {
                return Some(OsStr::from_bytes(&self.val[start..self.pos - 1]));
            }
        }
        Some(OsStr::from_bytes(&self.val[start..]))
    }
}
#[cfg(test)]
mod tests_llm_16_186 {
    use std::ffi::OsStr;
    use crate::osstringext::OsSplit;
    #[test]
    fn test_next() {
        let _rug_st_tests_llm_16_186_rrrruuuugggg_test_next = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 65;
        let rug_fuzz_2 = 66;
        let rug_fuzz_3 = 10;
        let rug_fuzz_4 = 67;
        let rug_fuzz_5 = 68;
        let rug_fuzz_6 = 0;
        let sep: u8 = rug_fuzz_0;
        let val: &[u8] = &[rug_fuzz_1, rug_fuzz_2, rug_fuzz_3, rug_fuzz_4, rug_fuzz_5];
        let mut os_split = OsSplit {
            sep: sep,
            val: val,
            pos: rug_fuzz_6,
        };
        debug_assert_eq!(os_split.next(), Some(OsStr::new("AB")));
        debug_assert_eq!(os_split.next(), Some(OsStr::new("CD")));
        debug_assert_eq!(os_split.next(), None);
        let _rug_ed_tests_llm_16_186_rrrruuuugggg_test_next = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_189 {
    use super::*;
    use crate::*;
    #[test]
    fn test_os_split_next() {
        let _rug_st_tests_llm_16_189_rrrruuuugggg_test_os_split_next = 0;
        let rug_fuzz_0 = "test,string";
        let rug_fuzz_1 = b',';
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = "test";
        let os_str = OsStr::new(rug_fuzz_0);
        let os_split = OsSplit {
            sep: rug_fuzz_1,
            val: os_str.as_bytes(),
            pos: rug_fuzz_2,
        };
        let result: Vec<&OsStr> = os_split.collect();
        let expected: Vec<&OsStr> = vec![OsStr::new(rug_fuzz_3), OsStr::new("string")];
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_189_rrrruuuugggg_test_os_split_next = 0;
    }
    #[test]
    fn test_os_str_split() {
        let _rug_st_tests_llm_16_189_rrrruuuugggg_test_os_str_split = 0;
        let rug_fuzz_0 = "test,string";
        let rug_fuzz_1 = b',';
        let rug_fuzz_2 = "test";
        let os_str = OsStr::new(rug_fuzz_0);
        let result = os_str.split(rug_fuzz_1);
        let expected: Vec<&OsStr> = vec![OsStr::new(rug_fuzz_2), OsStr::new("string")];
        let result: Vec<&OsStr> = result.collect();
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_189_rrrruuuugggg_test_os_str_split = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_190 {
    use std::ffi::OsStr;
    use osstringext::OsStrExt2;
    #[test]
    fn test_split_at() {
        let _rug_st_tests_llm_16_190_rrrruuuugggg_test_split_at = 0;
        let rug_fuzz_0 = "HelloWorld";
        let rug_fuzz_1 = 5;
        let os_str = OsStr::new(rug_fuzz_0);
        let (left, right) = <OsStr as OsStrExt2>::split_at(&os_str, rug_fuzz_1);
        debug_assert_eq!(left, OsStr::new("Hello"));
        debug_assert_eq!(right, OsStr::new("World"));
        let _rug_ed_tests_llm_16_190_rrrruuuugggg_test_split_at = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_191 {
    use super::*;
    use crate::*;
    #[test]
    fn test_split_at_byte() {
        let _rug_st_tests_llm_16_191_rrrruuuugggg_test_split_at_byte = 0;
        let rug_fuzz_0 = "hello";
        let rug_fuzz_1 = b'l';
        let rug_fuzz_2 = "hello";
        let rug_fuzz_3 = b'x';
        let rug_fuzz_4 = "abc";
        let rug_fuzz_5 = b'c';
        let rug_fuzz_6 = "abc";
        let rug_fuzz_7 = b'a';
        let os_str = std::ffi::OsStr::new(rug_fuzz_0);
        let (left, right) = os_str.split_at_byte(rug_fuzz_1);
        debug_assert_eq!(left, std::ffi::OsStr::new("he"));
        debug_assert_eq!(right, std::ffi::OsStr::new("lo"));
        let os_str = std::ffi::OsStr::new(rug_fuzz_2);
        let (left, right) = os_str.split_at_byte(rug_fuzz_3);
        debug_assert_eq!(left, std::ffi::OsStr::new("hello"));
        debug_assert_eq!(right, std::ffi::OsStr::new(""));
        let os_str = std::ffi::OsStr::new(rug_fuzz_4);
        let (left, right) = os_str.split_at_byte(rug_fuzz_5);
        debug_assert_eq!(left, std::ffi::OsStr::new("ab"));
        debug_assert_eq!(right, std::ffi::OsStr::new(""));
        let os_str = std::ffi::OsStr::new(rug_fuzz_6);
        let (left, right) = os_str.split_at_byte(rug_fuzz_7);
        debug_assert_eq!(left, std::ffi::OsStr::new(""));
        debug_assert_eq!(right, std::ffi::OsStr::new("bc"));
        let _rug_ed_tests_llm_16_191_rrrruuuugggg_test_split_at_byte = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_192 {
    use std::ffi::OsStr;
    use crate::osstringext::OsStrExt2;
    #[test]
    fn test_starts_with() {
        let _rug_st_tests_llm_16_192_rrrruuuugggg_test_starts_with = 0;
        let rug_fuzz_0 = "hello";
        let rug_fuzz_1 = b"hello";
        let rug_fuzz_2 = b"world";
        let os_str = OsStr::new(rug_fuzz_0);
        debug_assert_eq!(os_str.starts_with(rug_fuzz_1), true);
        debug_assert_eq!(os_str.starts_with(rug_fuzz_2), false);
        let _rug_ed_tests_llm_16_192_rrrruuuugggg_test_starts_with = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_193 {
    use super::*;
    use crate::*;
    use std::ffi::OsStr;
    #[test]
    fn test_trim_left_matches() {
        let _rug_st_tests_llm_16_193_rrrruuuugggg_test_trim_left_matches = 0;
        let rug_fuzz_0 = "abca";
        let rug_fuzz_1 = b'a';
        let rug_fuzz_2 = "bca";
        let os_str = OsStr::new(rug_fuzz_0);
        let byte = rug_fuzz_1;
        let result = os_str.trim_left_matches(byte);
        let expected = OsStr::new(rug_fuzz_2);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_193_rrrruuuugggg_test_trim_left_matches = 0;
    }
}
#[cfg(test)]
mod tests_rug_222 {
    use super::*;
    use crate::osstringext::OsStrExt2;
    use std::ffi::OsStr;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_222_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample";
        let rug_fuzz_1 = 65;
        let mut p0 = OsStr::new(rug_fuzz_0);
        let mut p1: u8 = rug_fuzz_1;
        <std::ffi::OsStr>::contains_byte(p0, p1);
        let _rug_ed_tests_rug_222_rrrruuuugggg_test_rug = 0;
    }
}
