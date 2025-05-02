use core::cmp;
use core::usize;
#[cfg(target_pointer_width = "32")]
const USIZE_BYTES: usize = 4;
#[cfg(target_pointer_width = "64")]
const USIZE_BYTES: usize = 8;
const LOOP_SIZE: usize = 2 * USIZE_BYTES;
/// Repeat the given byte into a word size number. That is, every 8 bits
/// is equivalent to the given byte. For example, if `b` is `\x4E` or
/// `01001110` in binary, then the returned value on a 32-bit system would be:
/// `01001110_01001110_01001110_01001110`.
#[inline(always)]
fn repeat_byte(b: u8) -> usize {
    (b as usize) * (usize::MAX / 255)
}
pub fn inv_memchr(n1: u8, haystack: &[u8]) -> Option<usize> {
    let vn1 = repeat_byte(n1);
    let confirm = |byte| byte != n1;
    let loop_size = cmp::min(LOOP_SIZE, haystack.len());
    let align = USIZE_BYTES - 1;
    let start_ptr = haystack.as_ptr();
    let end_ptr = haystack[haystack.len()..].as_ptr();
    let mut ptr = start_ptr;
    unsafe {
        if haystack.len() < USIZE_BYTES {
            return forward_search(start_ptr, end_ptr, ptr, confirm);
        }
        let chunk = read_unaligned_usize(ptr);
        if (chunk ^ vn1) != 0 {
            return forward_search(start_ptr, end_ptr, ptr, confirm);
        }
        ptr = ptr.add(USIZE_BYTES - (start_ptr as usize & align));
        debug_assert!(ptr > start_ptr);
        debug_assert!(end_ptr.sub(USIZE_BYTES) >= start_ptr);
        while loop_size == LOOP_SIZE && ptr <= end_ptr.sub(loop_size) {
            debug_assert_eq!(0, (ptr as usize) % USIZE_BYTES);
            let a = *(ptr as *const usize);
            let b = *(ptr.add(USIZE_BYTES) as *const usize);
            let eqa = (a ^ vn1) != 0;
            let eqb = (b ^ vn1) != 0;
            if eqa || eqb {
                break;
            }
            ptr = ptr.add(LOOP_SIZE);
        }
        forward_search(start_ptr, end_ptr, ptr, confirm)
    }
}
/// Return the last index not matching the byte `x` in `text`.
pub fn inv_memrchr(n1: u8, haystack: &[u8]) -> Option<usize> {
    let vn1 = repeat_byte(n1);
    let confirm = |byte| byte != n1;
    let loop_size = cmp::min(LOOP_SIZE, haystack.len());
    let align = USIZE_BYTES - 1;
    let start_ptr = haystack.as_ptr();
    let end_ptr = haystack[haystack.len()..].as_ptr();
    let mut ptr = end_ptr;
    unsafe {
        if haystack.len() < USIZE_BYTES {
            return reverse_search(start_ptr, end_ptr, ptr, confirm);
        }
        let chunk = read_unaligned_usize(ptr.sub(USIZE_BYTES));
        if (chunk ^ vn1) != 0 {
            return reverse_search(start_ptr, end_ptr, ptr, confirm);
        }
        ptr = (end_ptr as usize & !align) as *const u8;
        debug_assert!(start_ptr <= ptr && ptr <= end_ptr);
        while loop_size == LOOP_SIZE && ptr >= start_ptr.add(loop_size) {
            debug_assert_eq!(0, (ptr as usize) % USIZE_BYTES);
            let a = *(ptr.sub(2 * USIZE_BYTES) as *const usize);
            let b = *(ptr.sub(1 * USIZE_BYTES) as *const usize);
            let eqa = (a ^ vn1) != 0;
            let eqb = (b ^ vn1) != 0;
            if eqa || eqb {
                break;
            }
            ptr = ptr.sub(loop_size);
        }
        reverse_search(start_ptr, end_ptr, ptr, confirm)
    }
}
#[inline(always)]
unsafe fn forward_search<F: Fn(u8) -> bool>(
    start_ptr: *const u8,
    end_ptr: *const u8,
    mut ptr: *const u8,
    confirm: F,
) -> Option<usize> {
    debug_assert!(start_ptr <= ptr);
    debug_assert!(ptr <= end_ptr);
    while ptr < end_ptr {
        if confirm(*ptr) {
            return Some(sub(ptr, start_ptr));
        }
        ptr = ptr.offset(1);
    }
    None
}
#[inline(always)]
unsafe fn reverse_search<F: Fn(u8) -> bool>(
    start_ptr: *const u8,
    end_ptr: *const u8,
    mut ptr: *const u8,
    confirm: F,
) -> Option<usize> {
    debug_assert!(start_ptr <= ptr);
    debug_assert!(ptr <= end_ptr);
    while ptr > start_ptr {
        ptr = ptr.offset(-1);
        if confirm(*ptr) {
            return Some(sub(ptr, start_ptr));
        }
    }
    None
}
unsafe fn read_unaligned_usize(ptr: *const u8) -> usize {
    (ptr as *const usize).read_unaligned()
}
/// Subtract `b` from `a` and return the difference. `a` should be greater than
/// or equal to `b`.
fn sub(a: *const u8, b: *const u8) -> usize {
    debug_assert!(a >= b);
    (a as usize) - (b as usize)
}
/// Safe wrapper around `forward_search`
#[inline]
pub(crate) fn forward_search_bytes<F: Fn(u8) -> bool>(
    s: &[u8],
    confirm: F,
) -> Option<usize> {
    unsafe {
        let start = s.as_ptr();
        let end = start.add(s.len());
        forward_search(start, end, start, confirm)
    }
}
/// Safe wrapper around `reverse_search`
#[inline]
pub(crate) fn reverse_search_bytes<F: Fn(u8) -> bool>(
    s: &[u8],
    confirm: F,
) -> Option<usize> {
    unsafe {
        let start = s.as_ptr();
        let end = start.add(s.len());
        reverse_search(start, end, end, confirm)
    }
}
#[cfg(test)]
mod tests {
    use super::{inv_memchr, inv_memrchr};
    const TESTS: &[(&[u8], u8, usize, usize)] = &[
        (b"z", b'a', 0, 0),
        (b"zz", b'a', 0, 1),
        (b"aza", b'a', 1, 1),
        (b"zaz", b'a', 0, 2),
        (b"zza", b'a', 0, 1),
        (b"zaa", b'a', 0, 0),
        (b"zzz", b'a', 0, 2),
    ];
    type TestCase = (Vec<u8>, u8, Option<(usize, usize)>);
    fn build_tests() -> Vec<TestCase> {
        let mut result = vec![];
        for &(search, byte, fwd_pos, rev_pos) in TESTS {
            result.push((search.to_vec(), byte, Some((fwd_pos, rev_pos))));
            for i in 1..515 {
                let mut suffixed: Vec<u8> = search.into();
                suffixed.extend(std::iter::repeat(byte).take(i));
                result.push((suffixed, byte, Some((fwd_pos, rev_pos))));
                let mut prefixed: Vec<u8> = std::iter::repeat(byte).take(i).collect();
                prefixed.extend(search);
                result.push((prefixed, byte, Some((fwd_pos + i, rev_pos + i))));
                let mut surrounded: Vec<u8> = std::iter::repeat(byte).take(i).collect();
                surrounded.extend(search);
                surrounded.extend(std::iter::repeat(byte).take(i));
                result.push((surrounded, byte, Some((fwd_pos + i, rev_pos + i))));
            }
        }
        for i in 0..515 {
            result.push((std::iter::repeat(b'\0').take(i).collect(), b'\0', None));
        }
        result
    }
    #[test]
    fn test_inv_memchr() {
        use {ByteSlice, B};
        for (search, byte, matching) in build_tests() {
            assert_eq!(
                inv_memchr(byte, & search), matching.map(| m | m.0),
                "inv_memchr when searching for {:?} in {:?}", byte as char, B(& search)
                .as_bstr(),
            );
            assert_eq!(
                inv_memrchr(byte, & search), matching.map(| m | m.1),
                "inv_memrchr when searching for {:?} in {:?}", byte as char, B(& search)
                .as_bstr(),
            );
            for offset in 1..130 {
                if offset >= search.len() {
                    break;
                }
                if let Some((f, r)) = matching {
                    if offset > f || offset > r {
                        break;
                    }
                }
                let realigned = &search[offset..];
                let forward_pos = matching.map(|m| m.0 - offset);
                let reverse_pos = matching.map(|m| m.1 - offset);
                assert_eq!(
                    inv_memchr(byte, & realigned), forward_pos,
                    "inv_memchr when searching (realigned by {}) for {:?} in {:?}",
                    offset, byte as char, realigned.as_bstr(),
                );
                assert_eq!(
                    inv_memrchr(byte, & realigned), reverse_pos,
                    "inv_memrchr when searching (realigned by {}) for {:?} in {:?}",
                    offset, byte as char, realigned.as_bstr(),
                );
            }
        }
    }
}
#[cfg(test)]
mod tests_rug_7 {
    use super::*;
    #[test]
    fn test_repeat_byte() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: u8 = rug_fuzz_0;
        debug_assert_eq!(crate ::byteset::scalar::repeat_byte(p0), 0x4E4E4E4E);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_8 {
    use super::*;
    use crate::byteset::scalar::{
        inv_memchr, repeat_byte, LOOP_SIZE, USIZE_BYTES, forward_search,
        read_unaligned_usize,
    };
    use std::{cmp, mem};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_8_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let rug_fuzz_1 = b"hello world";
        let p0: u8 = rug_fuzz_0;
        let p1: &[u8] = rug_fuzz_1;
        inv_memchr(p0, p1);
        let _rug_ed_tests_rug_8_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_9 {
    use super::*;
    use crate::byteset::scalar::{inv_memrchr, repeat_byte, reverse_search};
    use std::{cmp, mem};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_9_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 65;
        let rug_fuzz_1 = b"abcdefg";
        let p0: u8 = rug_fuzz_0;
        let p1: &[u8] = rug_fuzz_1;
        inv_memrchr(p0, p1);
        let _rug_ed_tests_rug_9_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_12 {
    use super::*;
    use crate::byteset::scalar::read_unaligned_usize;
    #[test]
    fn test_read_unaligned_usize() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5, mut rug_fuzz_6, mut rug_fuzz_7)) = <(u8, u8, u8, u8, u8, u8, u8, u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let data: [u8; 8] = [
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
            rug_fuzz_6,
            rug_fuzz_7,
        ];
        let ptr = data.as_ptr();
        unsafe {
            read_unaligned_usize(ptr);
        }
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_13 {
    use super::*;
    #[test]
    fn test_sub() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u8, u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let a: u8 = rug_fuzz_0;
        let b: u8 = rug_fuzz_1;
        let p0: *const u8 = &a;
        let p1: *const u8 = &b;
        debug_assert_eq!(crate ::byteset::scalar::sub(p0, p1), 5);
             }
}
}
}    }
}
