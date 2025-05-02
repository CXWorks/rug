use core::mem;
#[cfg(any(test, not(target_arch = "x86_64")))]
const USIZE_BYTES: usize = mem::size_of::<usize>();
#[cfg(any(test, not(target_arch = "x86_64")))]
const FALLBACK_LOOP_SIZE: usize = 2 * USIZE_BYTES;
#[cfg(any(test, not(target_arch = "x86_64")))]
const ASCII_MASK_U64: u64 = 0x8080808080808080;
#[cfg(any(test, not(target_arch = "x86_64")))]
const ASCII_MASK: usize = ASCII_MASK_U64 as usize;
/// Returns the index of the first non ASCII byte in the given slice.
///
/// If slice only contains ASCII bytes, then the length of the slice is
/// returned.
pub fn first_non_ascii_byte(slice: &[u8]) -> usize {
    #[cfg(not(target_arch = "x86_64"))] { first_non_ascii_byte_fallback(slice) }
    #[cfg(target_arch = "x86_64")] { first_non_ascii_byte_sse2(slice) }
}
#[cfg(any(test, not(target_arch = "x86_64")))]
fn first_non_ascii_byte_fallback(slice: &[u8]) -> usize {
    let align = USIZE_BYTES - 1;
    let start_ptr = slice.as_ptr();
    let end_ptr = slice[slice.len()..].as_ptr();
    let mut ptr = start_ptr;
    unsafe {
        if slice.len() < USIZE_BYTES {
            return first_non_ascii_byte_slow(start_ptr, end_ptr, ptr);
        }
        let chunk = read_unaligned_usize(ptr);
        let mask = chunk & ASCII_MASK;
        if mask != 0 {
            return first_non_ascii_byte_mask(mask);
        }
        ptr = ptr_add(ptr, USIZE_BYTES - (start_ptr as usize & align));
        debug_assert!(ptr > start_ptr);
        debug_assert!(ptr_sub(end_ptr, USIZE_BYTES) >= start_ptr);
        if slice.len() >= FALLBACK_LOOP_SIZE {
            while ptr <= ptr_sub(end_ptr, FALLBACK_LOOP_SIZE) {
                debug_assert_eq!(0, (ptr as usize) % USIZE_BYTES);
                let a = *(ptr as *const usize);
                let b = *(ptr_add(ptr, USIZE_BYTES) as *const usize);
                if (a | b) & ASCII_MASK != 0 {
                    #[inline(never)]
                    unsafe fn findpos(start_ptr: *const u8, ptr: *const u8) -> usize {
                        let a = *(ptr as *const usize);
                        let b = *(ptr_add(ptr, USIZE_BYTES) as *const usize);
                        let mut at = sub(ptr, start_ptr);
                        let maska = a & ASCII_MASK;
                        if maska != 0 {
                            return at + first_non_ascii_byte_mask(maska);
                        }
                        at += USIZE_BYTES;
                        let maskb = b & ASCII_MASK;
                        debug_assert!(maskb != 0);
                        return at + first_non_ascii_byte_mask(maskb);
                    }
                    return findpos(start_ptr, ptr);
                }
                ptr = ptr_add(ptr, FALLBACK_LOOP_SIZE);
            }
        }
        first_non_ascii_byte_slow(start_ptr, end_ptr, ptr)
    }
}
#[cfg(target_arch = "x86_64")]
fn first_non_ascii_byte_sse2(slice: &[u8]) -> usize {
    use core::arch::x86_64::*;
    const VECTOR_SIZE: usize = mem::size_of::<__m128i>();
    const VECTOR_ALIGN: usize = VECTOR_SIZE - 1;
    const VECTOR_LOOP_SIZE: usize = 4 * VECTOR_SIZE;
    let start_ptr = slice.as_ptr();
    let end_ptr = slice[slice.len()..].as_ptr();
    let mut ptr = start_ptr;
    unsafe {
        if slice.len() < VECTOR_SIZE {
            return first_non_ascii_byte_slow(start_ptr, end_ptr, ptr);
        }
        let chunk = _mm_loadu_si128(ptr as *const __m128i);
        let mask = _mm_movemask_epi8(chunk);
        if mask != 0 {
            return mask.trailing_zeros() as usize;
        }
        ptr = ptr.add(VECTOR_SIZE - (start_ptr as usize & VECTOR_ALIGN));
        debug_assert!(ptr > start_ptr);
        debug_assert!(end_ptr.sub(VECTOR_SIZE) >= start_ptr);
        if slice.len() >= VECTOR_LOOP_SIZE {
            while ptr <= ptr_sub(end_ptr, VECTOR_LOOP_SIZE) {
                debug_assert_eq!(0, (ptr as usize) % VECTOR_SIZE);
                let a = _mm_load_si128(ptr as *const __m128i);
                let b = _mm_load_si128(ptr.add(VECTOR_SIZE) as *const __m128i);
                let c = _mm_load_si128(ptr.add(2 * VECTOR_SIZE) as *const __m128i);
                let d = _mm_load_si128(ptr.add(3 * VECTOR_SIZE) as *const __m128i);
                let or1 = _mm_or_si128(a, b);
                let or2 = _mm_or_si128(c, d);
                let or3 = _mm_or_si128(or1, or2);
                if _mm_movemask_epi8(or3) != 0 {
                    let mut at = sub(ptr, start_ptr);
                    let mask = _mm_movemask_epi8(a);
                    if mask != 0 {
                        return at + mask.trailing_zeros() as usize;
                    }
                    at += VECTOR_SIZE;
                    let mask = _mm_movemask_epi8(b);
                    if mask != 0 {
                        return at + mask.trailing_zeros() as usize;
                    }
                    at += VECTOR_SIZE;
                    let mask = _mm_movemask_epi8(c);
                    if mask != 0 {
                        return at + mask.trailing_zeros() as usize;
                    }
                    at += VECTOR_SIZE;
                    let mask = _mm_movemask_epi8(d);
                    debug_assert!(mask != 0);
                    return at + mask.trailing_zeros() as usize;
                }
                ptr = ptr_add(ptr, VECTOR_LOOP_SIZE);
            }
        }
        while ptr <= end_ptr.sub(VECTOR_SIZE) {
            debug_assert!(sub(end_ptr, ptr) >= VECTOR_SIZE);
            let chunk = _mm_loadu_si128(ptr as *const __m128i);
            let mask = _mm_movemask_epi8(chunk);
            if mask != 0 {
                return sub(ptr, start_ptr) + mask.trailing_zeros() as usize;
            }
            ptr = ptr.add(VECTOR_SIZE);
        }
        first_non_ascii_byte_slow(start_ptr, end_ptr, ptr)
    }
}
#[inline(always)]
unsafe fn first_non_ascii_byte_slow(
    start_ptr: *const u8,
    end_ptr: *const u8,
    mut ptr: *const u8,
) -> usize {
    debug_assert!(start_ptr <= ptr);
    debug_assert!(ptr <= end_ptr);
    while ptr < end_ptr {
        if *ptr > 0x7F {
            return sub(ptr, start_ptr);
        }
        ptr = ptr.offset(1);
    }
    sub(end_ptr, start_ptr)
}
/// Compute the position of the first ASCII byte in the given mask.
///
/// The mask should be computed by `chunk & ASCII_MASK`, where `chunk` is
/// 8 contiguous bytes of the slice being checked where *at least* one of those
/// bytes is not an ASCII byte.
///
/// The position returned is always in the inclusive range [0, 7].
#[cfg(any(test, not(target_arch = "x86_64")))]
fn first_non_ascii_byte_mask(mask: usize) -> usize {
    #[cfg(target_endian = "little")] { mask.trailing_zeros() as usize / 8 }
    #[cfg(target_endian = "big")] { mask.leading_zeros() as usize / 8 }
}
/// Increment the given pointer by the given amount.
unsafe fn ptr_add(ptr: *const u8, amt: usize) -> *const u8 {
    debug_assert!(amt < ::core::isize::MAX as usize);
    ptr.offset(amt as isize)
}
/// Decrement the given pointer by the given amount.
unsafe fn ptr_sub(ptr: *const u8, amt: usize) -> *const u8 {
    debug_assert!(amt < ::core::isize::MAX as usize);
    ptr.offset((amt as isize).wrapping_neg())
}
#[cfg(any(test, not(target_arch = "x86_64")))]
unsafe fn read_unaligned_usize(ptr: *const u8) -> usize {
    use core::ptr;
    let mut n: usize = 0;
    ptr::copy_nonoverlapping(ptr, &mut n as *mut _ as *mut u8, USIZE_BYTES);
    n
}
/// Subtract `b` from `a` and return the difference. `a` should be greater than
/// or equal to `b`.
fn sub(a: *const u8, b: *const u8) -> usize {
    debug_assert!(a >= b);
    (a as usize) - (b as usize)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn positive_fallback_forward() {
        for i in 0..517 {
            let s = "a".repeat(i);
            assert_eq!(
                i, first_non_ascii_byte_fallback(s.as_bytes()),
                "i: {:?}, len: {:?}, s: {:?}", i, s.len(), s
            );
        }
    }
    #[test]
    #[cfg(target_arch = "x86_64")]
    fn positive_sse2_forward() {
        for i in 0..517 {
            let b = "a".repeat(i).into_bytes();
            assert_eq!(b.len(), first_non_ascii_byte_sse2(& b));
        }
    }
    #[test]
    fn negative_fallback_forward() {
        for i in 0..517 {
            for align in 0..65 {
                let mut s = "a".repeat(i);
                s.push_str(
                    "☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃",
                );
                let s = s.get(align..).unwrap_or("");
                assert_eq!(
                    i.saturating_sub(align), first_non_ascii_byte_fallback(s.as_bytes()),
                    "i: {:?}, align: {:?}, len: {:?}, s: {:?}", i, align, s.len(), s
                );
            }
        }
    }
    #[test]
    #[cfg(target_arch = "x86_64")]
    fn negative_sse2_forward() {
        for i in 0..517 {
            for align in 0..65 {
                let mut s = "a".repeat(i);
                s.push_str(
                    "☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃☃",
                );
                let s = s.get(align..).unwrap_or("");
                assert_eq!(
                    i.saturating_sub(align), first_non_ascii_byte_sse2(s.as_bytes()),
                    "i: {:?}, align: {:?}, len: {:?}, s: {:?}", i, align, s.len(), s
                );
            }
        }
    }
}
#[cfg(test)]
mod tests_rug_1 {
    use super::*;
    #[test]
    fn test_first_non_ascii_byte() {
        let _rug_st_tests_rug_1_rrrruuuugggg_test_first_non_ascii_byte = 0;
        let rug_fuzz_0 = b"hello";
        let p0: &[u8] = rug_fuzz_0;
        debug_assert_eq!(crate ::ascii::first_non_ascii_byte(p0), 5);
        let _rug_ed_tests_rug_1_rrrruuuugggg_test_first_non_ascii_byte = 0;
    }
}
#[cfg(test)]
mod tests_rug_2 {
    use super::*;
    use crate::ascii::first_non_ascii_byte_sse2;
    use core::arch::x86_64::*;
    use std::arch::x86_64::*;
    use std::{mem, ptr};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5, mut rug_fuzz_6, mut rug_fuzz_7, mut rug_fuzz_8, mut rug_fuzz_9, mut rug_fuzz_10, mut rug_fuzz_11, mut rug_fuzz_12, mut rug_fuzz_13, mut rug_fuzz_14, mut rug_fuzz_15)) = <(u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let data: [u8; 16] = [
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
            rug_fuzz_6,
            rug_fuzz_7,
            rug_fuzz_8,
            rug_fuzz_9,
            rug_fuzz_10,
            rug_fuzz_11,
            rug_fuzz_12,
            rug_fuzz_13,
            rug_fuzz_14,
            rug_fuzz_15,
        ];
        let p0 = &data;
        first_non_ascii_byte_sse2(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_3 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_3_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"rust";
        let rug_fuzz_1 = b"example";
        let rug_fuzz_2 = b"code";
        let mut p0 = rug_fuzz_0.as_ptr();
        let mut p1 = rug_fuzz_1.as_ptr();
        let mut p2 = rug_fuzz_2.as_ptr();
        unsafe { crate::ascii::first_non_ascii_byte_slow(p0, p1, p2) };
        let _rug_ed_tests_rug_3_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_4 {
    use super::*;
    #[test]
    fn test_ptr_add() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1)) = <([u8; 4], usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        let ptr: *const u8 = rug_fuzz_0.as_ptr();
        let amount: usize = rug_fuzz_1;
        unsafe {
            let result = crate::ascii::ptr_add(ptr, amount);
        }
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_5 {
    use super::*;
    #[test]
    fn test_ptr_sub() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1)) = <([u8; 5], usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        let p0: *const u8 = rug_fuzz_0.as_ptr();
        let p1: usize = rug_fuzz_1;
        unsafe {
            let result = crate::ascii::ptr_sub(p0, p1);
        }
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_6 {
    use super::*;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5, mut rug_fuzz_6, mut rug_fuzz_7)) = <(u8, u8, u8, u8, u8, u8, u8, u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let a: [u8; 5] = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3, rug_fuzz_4];
        let b: [u8; 3] = [rug_fuzz_5, rug_fuzz_6, rug_fuzz_7];
        let p0: *const u8 = a.as_ptr();
        let p1: *const u8 = b.as_ptr();
        crate::ascii::sub(p0, p1);
             }
}
}
}    }
}
