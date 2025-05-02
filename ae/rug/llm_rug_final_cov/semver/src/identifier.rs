use crate::alloc::alloc::{alloc, dealloc, handle_alloc_error, Layout};
use core::isize;
use core::mem;
use core::num::{NonZeroU64, NonZeroUsize};
use core::ptr::{self, NonNull};
use core::slice;
use core::str;
use core::usize;
const PTR_BYTES: usize = mem::size_of::<NonNull<u8>>();
const TAIL_BYTES: usize = 8 * (PTR_BYTES < 8) as usize
    - PTR_BYTES * (PTR_BYTES < 8) as usize;
#[repr(C, align(8))]
pub(crate) struct Identifier {
    head: NonNull<u8>,
    tail: [u8; TAIL_BYTES],
}
impl Identifier {
    pub(crate) const fn empty() -> Self {
        const HEAD: NonNull<u8> = unsafe { NonNull::new_unchecked(!0 as *mut u8) };
        Identifier {
            head: HEAD,
            tail: [!0; TAIL_BYTES],
        }
    }
    pub(crate) unsafe fn new_unchecked(string: &str) -> Self {
        let len = string.len();
        debug_assert!(len <= isize::MAX as usize);
        match len as u64 {
            0 => Self::empty(),
            1..=8 => {
                let mut bytes = [0u8; mem::size_of::<Identifier>()];
                unsafe {
                    ptr::copy_nonoverlapping(string.as_ptr(), bytes.as_mut_ptr(), len)
                };
                unsafe {
                    mem::transmute::<
                        [u8; mem::size_of::<Identifier>()],
                        Identifier,
                    >(bytes)
                }
            }
            9..=0xff_ffff_ffff_ffff => {
                let size = bytes_for_varint(unsafe { NonZeroUsize::new_unchecked(len) })
                    + len;
                let align = 2;
                if mem::size_of::<usize>() < 8 {
                    let max_alloc = usize::MAX / 2 - align;
                    assert!(size <= max_alloc);
                }
                let layout = unsafe { Layout::from_size_align_unchecked(size, align) };
                let ptr = unsafe { alloc(layout) };
                if ptr.is_null() {
                    handle_alloc_error(layout);
                }
                let mut write = ptr;
                let mut varint_remaining = len;
                while varint_remaining > 0 {
                    unsafe { ptr::write(write, varint_remaining as u8 | 0x80) };
                    varint_remaining >>= 7;
                    write = unsafe { write.add(1) };
                }
                unsafe { ptr::copy_nonoverlapping(string.as_ptr(), write, len) };
                Identifier {
                    head: ptr_to_repr(ptr),
                    tail: [0; TAIL_BYTES],
                }
            }
            0x100_0000_0000_0000..=0xffff_ffff_ffff_ffff => {
                unreachable!(
                    "please refrain from storing >64 petabytes of text in semver version"
                );
            }
            #[cfg(no_exhaustive_int_match)]
            _ => unreachable!(),
        }
    }
    pub(crate) fn is_empty(&self) -> bool {
        let empty = Self::empty();
        let is_empty = self.head == empty.head && self.tail == empty.tail;
        mem::forget(empty);
        is_empty
    }
    fn is_inline(&self) -> bool {
        self.head.as_ptr() as usize >> (PTR_BYTES * 8 - 1) == 0
    }
    fn is_empty_or_inline(&self) -> bool {
        self.is_empty() || self.is_inline()
    }
    pub(crate) fn as_str(&self) -> &str {
        if self.is_empty() {
            ""
        } else if self.is_inline() {
            unsafe { inline_as_str(self) }
        } else {
            unsafe { ptr_as_str(&self.head) }
        }
    }
}
impl Clone for Identifier {
    fn clone(&self) -> Self {
        if self.is_empty_or_inline() {
            Identifier {
                head: self.head,
                tail: self.tail,
            }
        } else {
            let ptr = repr_to_ptr(self.head);
            let len = unsafe { decode_len(ptr) };
            let size = bytes_for_varint(len) + len.get();
            let align = 2;
            let layout = unsafe { Layout::from_size_align_unchecked(size, align) };
            let clone = unsafe { alloc(layout) };
            if clone.is_null() {
                handle_alloc_error(layout);
            }
            unsafe { ptr::copy_nonoverlapping(ptr, clone, size) }
            Identifier {
                head: ptr_to_repr(clone),
                tail: [0; TAIL_BYTES],
            }
        }
    }
}
impl Drop for Identifier {
    fn drop(&mut self) {
        if self.is_empty_or_inline() {
            return;
        }
        let ptr = repr_to_ptr_mut(self.head);
        let len = unsafe { decode_len(ptr) };
        let size = bytes_for_varint(len) + len.get();
        let align = 2;
        let layout = unsafe { Layout::from_size_align_unchecked(size, align) };
        unsafe { dealloc(ptr, layout) }
    }
}
impl PartialEq for Identifier {
    fn eq(&self, rhs: &Self) -> bool {
        if self.is_empty_or_inline() {
            self.head == rhs.head && self.tail == rhs.tail
        } else if rhs.is_empty_or_inline() {
            false
        } else {
            unsafe { ptr_as_str(&self.head) == ptr_as_str(&rhs.head) }
        }
    }
}
unsafe impl Send for Identifier {}
unsafe impl Sync for Identifier {}
fn ptr_to_repr(original: *mut u8) -> NonNull<u8> {
    let modified = (original as usize | 1).rotate_right(1);
    let diff = modified.wrapping_sub(original as usize);
    let modified = original.wrapping_add(diff);
    unsafe { NonNull::new_unchecked(modified) }
}
fn repr_to_ptr(modified: NonNull<u8>) -> *const u8 {
    let modified = modified.as_ptr();
    let original = (modified as usize) << 1;
    let diff = original.wrapping_sub(modified as usize);
    modified.wrapping_add(diff)
}
fn repr_to_ptr_mut(repr: NonNull<u8>) -> *mut u8 {
    repr_to_ptr(repr) as *mut u8
}
unsafe fn inline_len(repr: &Identifier) -> NonZeroUsize {
    let repr = unsafe { ptr::read(repr as *const Identifier as *const NonZeroU64) };
    #[cfg(no_nonzero_bitscan)]
    let repr = repr.get();
    #[cfg(target_endian = "little")]
    let zero_bits_on_string_end = repr.leading_zeros();
    #[cfg(target_endian = "big")]
    let zero_bits_on_string_end = repr.trailing_zeros();
    let nonzero_bytes = 8 - zero_bits_on_string_end as usize / 8;
    unsafe { NonZeroUsize::new_unchecked(nonzero_bytes) }
}
unsafe fn inline_as_str(repr: &Identifier) -> &str {
    let ptr = repr as *const Identifier as *const u8;
    let len = unsafe { inline_len(repr) }.get();
    let slice = unsafe { slice::from_raw_parts(ptr, len) };
    unsafe { str::from_utf8_unchecked(slice) }
}
unsafe fn decode_len(ptr: *const u8) -> NonZeroUsize {
    let [first, second] = unsafe { ptr::read(ptr as *const [u8; 2]) };
    if second < 0x80 {
        unsafe { NonZeroUsize::new_unchecked((first & 0x7f) as usize) }
    } else {
        return unsafe { decode_len_cold(ptr) };
        #[cold]
        #[inline(never)]
        unsafe fn decode_len_cold(mut ptr: *const u8) -> NonZeroUsize {
            let mut len = 0;
            let mut shift = 0;
            loop {
                let byte = unsafe { *ptr };
                if byte < 0x80 {
                    return unsafe { NonZeroUsize::new_unchecked(len) };
                }
                ptr = unsafe { ptr.add(1) };
                len += ((byte & 0x7f) as usize) << shift;
                shift += 7;
            }
        }
    }
}
unsafe fn ptr_as_str(repr: &NonNull<u8>) -> &str {
    let ptr = repr_to_ptr(*repr);
    let len = unsafe { decode_len(ptr) };
    let header = bytes_for_varint(len);
    let slice = unsafe { slice::from_raw_parts(ptr.add(header), len.get()) };
    unsafe { str::from_utf8_unchecked(slice) }
}
fn bytes_for_varint(len: NonZeroUsize) -> usize {
    #[cfg(no_nonzero_bitscan)]
    let len = len.get();
    let usize_bits = mem::size_of::<usize>() * 8;
    let len_bits = usize_bits - len.leading_zeros() as usize;
    (len_bits + 6) / 7
}
#[cfg(test)]
mod tests_rug_12 {
    use super::*;
    use std::ptr::null_mut;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_12_rrrruuuugggg_test_rug = 0;
        let mut p0 = null_mut();
        crate::identifier::ptr_to_repr(p0);
        let _rug_ed_tests_rug_12_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_13 {
    use std::ptr::NonNull;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_13_rrrruuuugggg_test_rug = 0;
        let mut p0: NonNull<u8> = NonNull::dangling();
        crate::identifier::repr_to_ptr(p0);
        let _rug_ed_tests_rug_13_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_14 {
    use super::*;
    use std::ptr::NonNull;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_14_rrrruuuugggg_test_rug = 0;
        let mut v9: NonNull<u8> = NonNull::dangling();
        crate::identifier::repr_to_ptr_mut(v9);
        let _rug_ed_tests_rug_14_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_15 {
    use super::*;
    use crate::identifier::Identifier;
    #[test]
    fn test_inline_len() {
        let _rug_st_tests_rug_15_rrrruuuugggg_test_inline_len = 0;
        let p0 = Identifier::default();
        unsafe { inline_len(&p0) };
        let _rug_ed_tests_rug_15_rrrruuuugggg_test_inline_len = 0;
    }
}
#[cfg(test)]
mod tests_rug_16 {
    use super::*;
    use crate::identifier::Identifier;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_16_rrrruuuugggg_test_rug = 0;
        let mut p0: &Identifier = &Identifier::default();
        unsafe {
            crate::identifier::inline_as_str(p0);
        }
        let _rug_ed_tests_rug_16_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_17 {
    use super::*;
    use std::num::NonZeroUsize;
    use std::ptr;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_17_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"Some sample data\0";
        let p0: *const u8 = rug_fuzz_0.as_ptr();
        unsafe {
            crate::identifier::decode_len(p0);
        }
        let _rug_ed_tests_rug_17_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_19 {
    use super::*;
    use std::ptr::NonNull;
    #[test]
    fn test_ptr_as_str() {
        let _rug_st_tests_rug_19_rrrruuuugggg_test_ptr_as_str = 0;
        let mut v9: NonNull<u8> = NonNull::dangling();
        unsafe {
            crate::identifier::ptr_as_str(&v9);
        }
        let _rug_ed_tests_rug_19_rrrruuuugggg_test_ptr_as_str = 0;
    }
}
#[cfg(test)]
mod tests_rug_20 {
    use super::*;
    use std::mem;
    use std::num::NonZeroUsize;
    #[test]
    fn test_bytes_for_varint() {
        let _rug_st_tests_rug_20_rrrruuuugggg_test_bytes_for_varint = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 8;
        let rug_fuzz_2 = 6;
        let rug_fuzz_3 = 7;
        let p0 = unsafe { NonZeroUsize::new_unchecked(rug_fuzz_0) };
        let usize_bits = mem::size_of::<usize>() * rug_fuzz_1;
        let len_bits = usize_bits - p0.get().leading_zeros() as usize;
        let expected = (len_bits + rug_fuzz_2) / rug_fuzz_3;
        debug_assert_eq!(crate ::identifier::bytes_for_varint(p0), expected);
        let _rug_ed_tests_rug_20_rrrruuuugggg_test_bytes_for_varint = 0;
    }
}
#[cfg(test)]
mod tests_rug_21 {
    use super::*;
    use crate::identifier::Identifier;
    use std::ptr::NonNull;
    #[test]
    fn test_empty() {
        let _rug_st_tests_rug_21_rrrruuuugggg_test_empty = 0;
        Identifier::empty();
        let _rug_ed_tests_rug_21_rrrruuuugggg_test_empty = 0;
    }
}
#[cfg(test)]
mod tests_rug_22 {
    use super::*;
    use crate::identifier::Identifier;
    use std::num::NonZeroUsize;
    use std::alloc::{alloc, Layout};
    use std::mem;
    use std::ptr;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_22_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "some_string";
        let p0 = rug_fuzz_0;
        unsafe {
            Identifier::new_unchecked(&p0);
        }
        let _rug_ed_tests_rug_22_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_23 {
    use super::*;
    use crate::identifier::Identifier;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_23_rrrruuuugggg_test_rug = 0;
        let mut p0 = Identifier::default();
        crate::identifier::Identifier::is_empty(&mut p0);
        let _rug_ed_tests_rug_23_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_24 {
    use super::*;
    use crate::identifier::Identifier;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_24_rrrruuuugggg_test_rug = 0;
        let mut p0 = Identifier::default();
        Identifier::is_inline(&p0);
        let _rug_ed_tests_rug_24_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_25 {
    use super::*;
    use crate::identifier::Identifier;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_25_rrrruuuugggg_test_rug = 0;
        let p0 = Identifier::default();
        <Identifier>::is_empty_or_inline(&p0);
        let _rug_ed_tests_rug_25_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_26 {
    use super::*;
    use crate::identifier::Identifier;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_26_rrrruuuugggg_test_rug = 0;
        let p0 = Identifier::default();
        let result = p0.as_str();
        let _rug_ed_tests_rug_26_rrrruuuugggg_test_rug = 0;
    }
}
