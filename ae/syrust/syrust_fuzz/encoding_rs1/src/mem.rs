//! Functions for converting between different in-RAM representations of text
//! and for quickly checking if the Unicode Bidirectional Algorithm can be
//! avoided.
//!
//! By using slices for output, the functions here seek to enable by-register
//! (ALU register or SIMD register as available) operations in order to
//! outperform iterator-based conversions available in the Rust standard
//! library.
//!
//! _Note:_ "Latin1" in this module refers to the Unicode range from U+0000 to
//! U+00FF, inclusive, and does not refer to the windows-1252 range. This
//! in-memory encoding is sometimes used as a storage optimization of text
//! when UTF-16 indexing and length semantics are exposed.
//!
//! The FFI binding for this module are in the
//! [encoding_c_mem crate](https://github.com/hsivonen/encoding_c_mem).
use std::borrow::Cow;
use super::in_inclusive_range16;
use super::in_inclusive_range32;
use super::in_inclusive_range8;
use super::in_range16;
use super::in_range32;
use super::DecoderResult;
use ascii::*;
use utf_8::*;
macro_rules! non_fuzz_debug_assert {
    ($($arg:tt)*) => {
        if ! cfg!(fuzzing) { debug_assert!($($arg)*); }
    };
}
cfg_if! {
    if #[cfg(feature = "simd-accel")] { use ::std::intrinsics::likely; use
    ::std::intrinsics::unlikely; } else { #[inline(always)] unsafe fn likely(b : bool) ->
    bool { b } #[inline(always)] unsafe fn unlikely(b : bool) -> bool { b } }
}
/// Classification of text as Latin1 (all code points are below U+0100),
/// left-to-right with some non-Latin1 characters or as containing at least
/// some right-to-left characters.
#[must_use]
#[derive(Debug, PartialEq, Eq)]
#[repr(C)]
pub enum Latin1Bidi {
    /// Every character is below U+0100.
    Latin1 = 0,
    /// There is at least one character that's U+0100 or higher, but there
    /// are no right-to-left characters.
    LeftToRight = 1,
    /// There is at least one right-to-left character.
    Bidi = 2,
}
#[allow(dead_code)]
const LATIN1_MASK: usize = 0xFF00_FF00_FF00_FF00u64 as usize;
#[allow(unused_macros)]
macro_rules! by_unit_check_alu {
    ($name:ident, $unit:ty, $bound:expr, $mask:ident) => {
        #[cfg_attr(feature = "cargo-clippy", allow(cast_ptr_alignment))]
        #[inline(always)] fn $name (buffer : & [$unit]) -> bool { let mut offset =
        0usize; let mut accu = 0usize; let unit_size = ::std::mem::size_of::<$unit > ();
        let len = buffer.len(); if len >= ALU_ALIGNMENT / unit_size { if buffer[0] >=
        $bound { return false; } let src = buffer.as_ptr(); let mut until_alignment =
        ((ALU_ALIGNMENT - ((src as usize) & ALU_ALIGNMENT_MASK)) & ALU_ALIGNMENT_MASK) /
        unit_size; if until_alignment + ALU_ALIGNMENT / unit_size <= len { if
        until_alignment != 0 { accu |= buffer[offset] as usize; offset += 1;
        until_alignment -= 1; while until_alignment != 0 { accu |= buffer[offset] as
        usize; offset += 1; until_alignment -= 1; } if accu >= $bound { return false; } }
        let len_minus_stride = len - ALU_ALIGNMENT / unit_size; if offset + (4 *
        (ALU_ALIGNMENT / unit_size)) <= len { let len_minus_unroll = len - (4 *
        (ALU_ALIGNMENT / unit_size)); loop { let unroll_accu = unsafe { * (src
        .add(offset) as * const usize) } | unsafe { * (src.add(offset + (ALU_ALIGNMENT /
        unit_size)) as * const usize) } | unsafe { * (src.add(offset + (2 *
        (ALU_ALIGNMENT / unit_size))) as * const usize) } | unsafe { * (src.add(offset +
        (3 * (ALU_ALIGNMENT / unit_size))) as * const usize) }; if unroll_accu & $mask !=
        0 { return false; } offset += 4 * (ALU_ALIGNMENT / unit_size); if offset >
        len_minus_unroll { break; } } } while offset <= len_minus_stride { accu |= unsafe
        { * (src.add(offset) as * const usize) }; offset += ALU_ALIGNMENT / unit_size; }
        } } for & unit in & buffer[offset..] { accu |= unit as usize; } accu & $mask == 0
        }
    };
}
#[allow(unused_macros)]
macro_rules! by_unit_check_simd {
    ($name:ident, $unit:ty, $splat:expr, $simd_ty:ty, $bound:expr, $func:ident) => {
        #[inline(always)] fn $name (buffer : & [$unit]) -> bool { let mut offset =
        0usize; let mut accu = 0usize; let unit_size = ::std::mem::size_of::<$unit > ();
        let len = buffer.len(); if len >= SIMD_STRIDE_SIZE / unit_size { if buffer[0] >=
        $bound { return false; } let src = buffer.as_ptr(); let mut until_alignment =
        ((SIMD_ALIGNMENT - ((src as usize) & SIMD_ALIGNMENT_MASK)) & SIMD_ALIGNMENT_MASK)
        / unit_size; if until_alignment + SIMD_STRIDE_SIZE / unit_size <= len { if
        until_alignment != 0 { accu |= buffer[offset] as usize; offset += 1;
        until_alignment -= 1; while until_alignment != 0 { accu |= buffer[offset] as
        usize; offset += 1; until_alignment -= 1; } if accu >= $bound { return false; } }
        let len_minus_stride = len - SIMD_STRIDE_SIZE / unit_size; if offset + (4 *
        (SIMD_STRIDE_SIZE / unit_size)) <= len { let len_minus_unroll = len - (4 *
        (SIMD_STRIDE_SIZE / unit_size)); loop { let unroll_accu = unsafe { * (src
        .add(offset) as * const $simd_ty) } | unsafe { * (src.add(offset +
        (SIMD_STRIDE_SIZE / unit_size)) as * const $simd_ty) } | unsafe { * (src
        .add(offset + (2 * (SIMD_STRIDE_SIZE / unit_size))) as * const $simd_ty) } |
        unsafe { * (src.add(offset + (3 * (SIMD_STRIDE_SIZE / unit_size))) as * const
        $simd_ty) }; if !$func (unroll_accu) { return false; } offset += 4 *
        (SIMD_STRIDE_SIZE / unit_size); if offset > len_minus_unroll { break; } } } let
        mut simd_accu = $splat; while offset <= len_minus_stride { simd_accu = simd_accu
        | unsafe { * (src.add(offset) as * const $simd_ty) }; offset += SIMD_STRIDE_SIZE
        / unit_size; } if !$func (simd_accu) { return false; } } } for & unit in &
        buffer[offset..] { accu |= unit as usize; } accu < $bound }
    };
}
cfg_if! {
    if #[cfg(all(feature = "simd-accel", any(target_feature = "sse2", all(target_endian =
    "little", target_arch = "aarch64"), all(target_endian = "little", target_feature =
    "neon"))))] { use simd_funcs::*; use packed_simd::u8x16; use packed_simd::u16x8;
    const SIMD_ALIGNMENT : usize = 16; const SIMD_ALIGNMENT_MASK : usize = 15;
    by_unit_check_simd!(is_ascii_impl, u8, u8x16::splat(0), u8x16, 0x80, simd_is_ascii);
    by_unit_check_simd!(is_basic_latin_impl, u16, u16x8::splat(0), u16x8, 0x80,
    simd_is_basic_latin); by_unit_check_simd!(is_utf16_latin1_impl, u16, u16x8::splat(0),
    u16x8, 0x100, simd_is_latin1); #[inline(always)] fn utf16_valid_up_to_impl(buffer : &
    [u16]) -> usize { let unit_size = ::std::mem::size_of::< u16 > (); let src = buffer
    .as_ptr(); let len = buffer.len(); let mut offset = 0usize; 'outer : loop { let
    until_alignment = ((SIMD_ALIGNMENT - ((unsafe { src.add(offset) } as usize) &
    SIMD_ALIGNMENT_MASK)) & SIMD_ALIGNMENT_MASK) / unit_size; if until_alignment == 0 {
    if offset + SIMD_STRIDE_SIZE / unit_size > len { break; } } else { let
    offset_plus_until_alignment = offset + until_alignment; let
    offset_plus_until_alignment_plus_one = offset_plus_until_alignment + 1; if
    offset_plus_until_alignment_plus_one + SIMD_STRIDE_SIZE / unit_size > len { break; }
    let (up_to, last_valid_low) = utf16_valid_up_to_alu(& buffer[offset
    ..offset_plus_until_alignment_plus_one]); if up_to < until_alignment { return offset
    + up_to; } if last_valid_low { offset = offset_plus_until_alignment_plus_one;
    continue; } offset = offset_plus_until_alignment; } let len_minus_stride = len -
    SIMD_STRIDE_SIZE / unit_size; 'inner : loop { let offset_plus_stride = offset +
    SIMD_STRIDE_SIZE / unit_size; if contains_surrogates(unsafe { * (src.add(offset) as *
    const u16x8) }) { if offset_plus_stride == len { break 'outer; } let
    offset_plus_stride_plus_one = offset_plus_stride + 1; let (up_to, last_valid_low) =
    utf16_valid_up_to_alu(& buffer[offset..offset_plus_stride_plus_one]); if up_to <
    SIMD_STRIDE_SIZE / unit_size { return offset + up_to; } if last_valid_low { offset =
    offset_plus_stride_plus_one; continue 'outer; } } offset = offset_plus_stride; if
    offset > len_minus_stride { break 'outer; } } } let (up_to, _) =
    utf16_valid_up_to_alu(& buffer[offset..]); offset + up_to } } else {
    by_unit_check_alu!(is_ascii_impl, u8, 0x80, ASCII_MASK);
    by_unit_check_alu!(is_basic_latin_impl, u16, 0x80, BASIC_LATIN_MASK);
    by_unit_check_alu!(is_utf16_latin1_impl, u16, 0x100, LATIN1_MASK); #[inline(always)]
    fn utf16_valid_up_to_impl(buffer : & [u16]) -> usize { let (up_to, _) =
    utf16_valid_up_to_alu(buffer); up_to } }
}
/// The second return value is true iff the last code unit of the slice was
/// reached and turned out to be a low surrogate that is part of a valid pair.
#[cfg_attr(feature = "cargo-clippy", allow(collapsible_if))]
#[inline(always)]
fn utf16_valid_up_to_alu(buffer: &[u16]) -> (usize, bool) {
    let len = buffer.len();
    if len == 0 {
        return (0, false);
    }
    let mut offset = 0usize;
    loop {
        let unit = buffer[offset];
        let next = offset + 1;
        let unit_minus_surrogate_start = unit.wrapping_sub(0xD800);
        if unit_minus_surrogate_start > (0xDFFF - 0xD800) {
            offset = next;
            if offset == len {
                return (offset, false);
            }
            continue;
        }
        if unit_minus_surrogate_start <= (0xDBFF - 0xD800) {
            if next < len {
                let second = buffer[next];
                let second_minus_low_surrogate_start = second.wrapping_sub(0xDC00);
                if second_minus_low_surrogate_start <= (0xDFFF - 0xDC00) {
                    offset = next + 1;
                    if offset == len {
                        return (offset, true);
                    }
                    continue;
                }
            }
        }
        return (offset, false);
    }
}
cfg_if! {
    if #[cfg(all(feature = "simd-accel", any(target_feature = "sse2", all(target_endian =
    "little", target_arch = "aarch64"), all(target_endian = "little", target_feature =
    "neon"))))] { #[inline(always)] fn is_str_latin1_impl(buffer : & str) -> Option <
    usize > { let mut offset = 0usize; let bytes = buffer.as_bytes(); let len = bytes
    .len(); if len >= SIMD_STRIDE_SIZE { let src = bytes.as_ptr(); let mut
    until_alignment = (SIMD_ALIGNMENT - ((src as usize) & SIMD_ALIGNMENT_MASK)) &
    SIMD_ALIGNMENT_MASK; if until_alignment + SIMD_STRIDE_SIZE <= len { while
    until_alignment != 0 { if bytes[offset] > 0xC3 { return Some(offset); } offset += 1;
    until_alignment -= 1; } let len_minus_stride = len - SIMD_STRIDE_SIZE; loop { if !
    simd_is_str_latin1(unsafe { * (src.add(offset) as * const u8x16) }) { while
    bytes[offset] & 0xC0 == 0x80 { offset += 1; } return Some(offset); } offset +=
    SIMD_STRIDE_SIZE; if offset > len_minus_stride { break; } } } } for i in offset..len
    { if bytes[i] > 0xC3 { return Some(i); } } None } } else { #[inline(always)] fn
    is_str_latin1_impl(buffer : & str) -> Option < usize > { let mut bytes = buffer
    .as_bytes(); let mut total = 0; loop { if let Some((byte, offset)) =
    validate_ascii(bytes) { total += offset; if byte > 0xC3 { return Some(total); } bytes
    = & bytes[offset + 2..]; total += 2; } else { return None; } } } }
}
#[inline(always)]
fn is_utf8_latin1_impl(buffer: &[u8]) -> Option<usize> {
    let mut bytes = buffer;
    let mut total = 0;
    loop {
        if let Some((byte, offset)) = validate_ascii(bytes) {
            total += offset;
            if in_inclusive_range8(byte, 0xC2, 0xC3) {
                let next = offset + 1;
                if next == bytes.len() {
                    return Some(total);
                }
                if bytes[next] & 0xC0 != 0x80 {
                    return Some(total);
                }
                bytes = &bytes[offset + 2..];
                total += 2;
            } else {
                return Some(total);
            }
        } else {
            return None;
        }
    }
}
cfg_if! {
    if #[cfg(all(feature = "simd-accel", any(target_feature = "sse2", all(target_endian =
    "little", target_arch = "aarch64"), all(target_endian = "little", target_feature =
    "neon"))))] { #[inline(always)] fn is_utf16_bidi_impl(buffer : & [u16]) -> bool { let
    mut offset = 0usize; let len = buffer.len(); if len >= SIMD_STRIDE_SIZE / 2 { let src
    = buffer.as_ptr(); let mut until_alignment = ((SIMD_ALIGNMENT - ((src as usize) &
    SIMD_ALIGNMENT_MASK)) & SIMD_ALIGNMENT_MASK) / 2; if until_alignment +
    (SIMD_STRIDE_SIZE / 2) <= len { while until_alignment != 0 { if
    is_utf16_code_unit_bidi(buffer[offset]) { return true; } offset += 1; until_alignment
    -= 1; } let len_minus_stride = len - (SIMD_STRIDE_SIZE / 2); loop { if
    is_u16x8_bidi(unsafe { * (src.add(offset) as * const u16x8) }) { return true; }
    offset += SIMD_STRIDE_SIZE / 2; if offset > len_minus_stride { break; } } } } for & u
    in & buffer[offset..] { if is_utf16_code_unit_bidi(u) { return true; } } false } }
    else { #[inline(always)] fn is_utf16_bidi_impl(buffer : & [u16]) -> bool { for & u in
    buffer { if is_utf16_code_unit_bidi(u) { return true; } } false } }
}
cfg_if! {
    if #[cfg(all(feature = "simd-accel", any(target_feature = "sse2", all(target_endian =
    "little", target_arch = "aarch64"), all(target_endian = "little", target_feature =
    "neon"))))] { #[inline(always)] fn check_utf16_for_latin1_and_bidi_impl(buffer : &
    [u16]) -> Latin1Bidi { let mut offset = 0usize; let len = buffer.len(); if len >=
    SIMD_STRIDE_SIZE / 2 { let src = buffer.as_ptr(); let mut until_alignment =
    ((SIMD_ALIGNMENT - ((src as usize) & SIMD_ALIGNMENT_MASK)) & SIMD_ALIGNMENT_MASK) /
    2; if until_alignment + (SIMD_STRIDE_SIZE / 2) <= len { while until_alignment != 0 {
    if buffer[offset] > 0xFF { if is_utf16_bidi_impl(& buffer[offset..]) { return
    Latin1Bidi::Bidi; } return Latin1Bidi::LeftToRight; } offset += 1; until_alignment -=
    1; } let len_minus_stride = len - (SIMD_STRIDE_SIZE / 2); loop { let mut s = unsafe {
    * (src.add(offset) as * const u16x8) }; if ! simd_is_latin1(s) { loop { if
    is_u16x8_bidi(s) { return Latin1Bidi::Bidi; } offset += SIMD_STRIDE_SIZE / 2; if
    offset > len_minus_stride { for & u in & buffer[offset..] { if
    is_utf16_code_unit_bidi(u) { return Latin1Bidi::Bidi; } } return
    Latin1Bidi::LeftToRight; } s = unsafe { * (src.add(offset) as * const u16x8) }; } }
    offset += SIMD_STRIDE_SIZE / 2; if offset > len_minus_stride { break; } } } } let mut
    iter = (& buffer[offset..]).iter(); loop { if let Some(& u) = iter.next() { if u >
    0xFF { let mut inner_u = u; loop { if is_utf16_code_unit_bidi(inner_u) { return
    Latin1Bidi::Bidi; } if let Some(& code_unit) = iter.next() { inner_u = code_unit; }
    else { return Latin1Bidi::LeftToRight; } } } } else { return Latin1Bidi::Latin1; } }
    } } else { #[cfg_attr(feature = "cargo-clippy", allow(cast_ptr_alignment))]
    #[inline(always)] fn check_utf16_for_latin1_and_bidi_impl(buffer : & [u16]) ->
    Latin1Bidi { let mut offset = 0usize; let len = buffer.len(); if len >= ALU_ALIGNMENT
    / 2 { let src = buffer.as_ptr(); let mut until_alignment = ((ALU_ALIGNMENT - ((src as
    usize) & ALU_ALIGNMENT_MASK)) & ALU_ALIGNMENT_MASK) / 2; if until_alignment +
    ALU_ALIGNMENT / 2 <= len { while until_alignment != 0 { if buffer[offset] > 0xFF { if
    is_utf16_bidi_impl(& buffer[offset..]) { return Latin1Bidi::Bidi; } return
    Latin1Bidi::LeftToRight; } offset += 1; until_alignment -= 1; } let len_minus_stride
    = len - ALU_ALIGNMENT / 2; loop { if unsafe { * (src.add(offset) as * const usize) }
    & LATIN1_MASK != 0 { if is_utf16_bidi_impl(& buffer[offset..]) { return
    Latin1Bidi::Bidi; } return Latin1Bidi::LeftToRight; } offset += ALU_ALIGNMENT / 2; if
    offset > len_minus_stride { break; } } } } let mut iter = (& buffer[offset..])
    .iter(); loop { if let Some(& u) = iter.next() { if u > 0xFF { let mut inner_u = u;
    loop { if is_utf16_code_unit_bidi(inner_u) { return Latin1Bidi::Bidi; } if let Some(&
    code_unit) = iter.next() { inner_u = code_unit; } else { return
    Latin1Bidi::LeftToRight; } } } } else { return Latin1Bidi::Latin1; } } } }
}
/// Checks whether the buffer is all-ASCII.
///
/// May read the entire buffer even if it isn't all-ASCII. (I.e. the function
/// is not guaranteed to fail fast.)
pub fn is_ascii(buffer: &[u8]) -> bool {
    is_ascii_impl(buffer)
}
/// Checks whether the buffer is all-Basic Latin (i.e. UTF-16 representing
/// only ASCII characters).
///
/// May read the entire buffer even if it isn't all-ASCII. (I.e. the function
/// is not guaranteed to fail fast.)
pub fn is_basic_latin(buffer: &[u16]) -> bool {
    is_basic_latin_impl(buffer)
}
/// Checks whether the buffer is valid UTF-8 representing only code points
/// less than or equal to U+00FF.
///
/// Fails fast. (I.e. returns before having read the whole buffer if UTF-8
/// invalidity or code points above U+00FF are discovered.
pub fn is_utf8_latin1(buffer: &[u8]) -> bool {
    is_utf8_latin1_impl(buffer).is_none()
}
/// Checks whether the buffer represents only code points less than or equal
/// to U+00FF.
///
/// Fails fast. (I.e. returns before having read the whole buffer if code
/// points above U+00FF are discovered.
pub fn is_str_latin1(buffer: &str) -> bool {
    is_str_latin1_impl(buffer).is_none()
}
/// Checks whether the buffer represents only code point less than or equal
/// to U+00FF.
///
/// May read the entire buffer even if it isn't all-Latin1. (I.e. the function
/// is not guaranteed to fail fast.)
pub fn is_utf16_latin1(buffer: &[u16]) -> bool {
    is_utf16_latin1_impl(buffer)
}
/// Checks whether a potentially-invalid UTF-8 buffer contains code points
/// that trigger right-to-left processing.
///
/// The check is done on a Unicode block basis without regard to assigned
/// vs. unassigned code points in the block. Hebrew presentation forms in
/// the Alphabetic Presentation Forms block are treated as if they formed
/// a block on their own (i.e. it treated as right-to-left). Additionally,
/// the four RIGHT-TO-LEFT FOO controls in General Punctuation are checked
/// for. Control characters that are technically bidi controls but do not
/// cause right-to-left behavior without the presence of right-to-left
/// characters or right-to-left controls are not checked for. As a special
/// case, U+FEFF is excluded from Arabic Presentation Forms-B.
///
/// Returns `true` if the input is invalid UTF-8 or the input contains an
/// RTL character. Returns `false` if the input is valid UTF-8 and contains
/// no RTL characters.
#[cfg_attr(feature = "cargo-clippy", allow(collapsible_if, cyclomatic_complexity))]
#[inline]
pub fn is_utf8_bidi(buffer: &[u8]) -> bool {
    let mut src = buffer;
    'outer: loop {
        if let Some((mut byte, mut read)) = validate_ascii(src) {
            if read + 4 <= src.len() {
                'inner: loop {
                    match byte {
                        0..=0x7F => {
                            read += 1;
                            src = &src[read..];
                            continue 'outer;
                        }
                        0xC2..=0xD5 => {
                            let second = unsafe { *(src.get_unchecked(read + 1)) };
                            if !in_inclusive_range8(second, 0x80, 0xBF) {
                                return true;
                            }
                            read += 2;
                        }
                        0xD6 => {
                            let second = unsafe { *(src.get_unchecked(read + 1)) };
                            if !in_inclusive_range8(second, 0x80, 0xBF) {
                                return true;
                            }
                            if second > 0x8F {
                                return true;
                            }
                            read += 2;
                        }
                        0xE1 | 0xE3..=0xEC | 0xEE => {
                            let second = unsafe { *(src.get_unchecked(read + 1)) };
                            let third = unsafe { *(src.get_unchecked(read + 2)) };
                            if ((UTF8_DATA.table[usize::from(second)]
                                & unsafe {
                                    *(UTF8_DATA.table.get_unchecked(byte as usize + 0x80))
                                }) | (third >> 6)) != 2
                            {
                                return true;
                            }
                            read += 3;
                        }
                        0xE2 => {
                            let second = unsafe { *(src.get_unchecked(read + 1)) };
                            let third = unsafe { *(src.get_unchecked(read + 2)) };
                            if ((UTF8_DATA.table[usize::from(second)]
                                & unsafe {
                                    *(UTF8_DATA.table.get_unchecked(byte as usize + 0x80))
                                }) | (third >> 6)) != 2
                            {
                                return true;
                            }
                            if second == 0x80 {
                                if third == 0x8F || third == 0xAB || third == 0xAE {
                                    return true;
                                }
                            } else if second == 0x81 {
                                if third == 0xA7 {
                                    return true;
                                }
                            }
                            read += 3;
                        }
                        0xEF => {
                            let second = unsafe { *(src.get_unchecked(read + 1)) };
                            let third = unsafe { *(src.get_unchecked(read + 2)) };
                            if ((UTF8_DATA.table[usize::from(second)]
                                & unsafe {
                                    *(UTF8_DATA.table.get_unchecked(byte as usize + 0x80))
                                }) | (third >> 6)) != 2
                            {
                                return true;
                            }
                            if in_inclusive_range8(second, 0xAC, 0xB7) {
                                if second == 0xAC {
                                    if third > 0x9C {
                                        return true;
                                    }
                                } else {
                                    return true;
                                }
                            } else if in_inclusive_range8(second, 0xB9, 0xBB) {
                                if second == 0xB9 {
                                    if third > 0xAF {
                                        return true;
                                    }
                                } else if second == 0xBB {
                                    if third != 0xBF {
                                        return true;
                                    }
                                } else {
                                    return true;
                                }
                            }
                            read += 3;
                        }
                        0xE0 => {
                            let second = unsafe { *(src.get_unchecked(read + 1)) };
                            let third = unsafe { *(src.get_unchecked(read + 2)) };
                            if ((UTF8_DATA.table[usize::from(second)]
                                & unsafe {
                                    *(UTF8_DATA.table.get_unchecked(byte as usize + 0x80))
                                }) | (third >> 6)) != 2
                            {
                                return true;
                            }
                            if second < 0xA4 {
                                return true;
                            }
                            read += 3;
                        }
                        0xED => {
                            let second = unsafe { *(src.get_unchecked(read + 1)) };
                            let third = unsafe { *(src.get_unchecked(read + 2)) };
                            if ((UTF8_DATA.table[usize::from(second)]
                                & unsafe {
                                    *(UTF8_DATA.table.get_unchecked(byte as usize + 0x80))
                                }) | (third >> 6)) != 2
                            {
                                return true;
                            }
                            read += 3;
                        }
                        0xF1..=0xF4 => {
                            let second = unsafe { *(src.get_unchecked(read + 1)) };
                            let third = unsafe { *(src.get_unchecked(read + 2)) };
                            let fourth = unsafe { *(src.get_unchecked(read + 3)) };
                            if (u16::from(
                                UTF8_DATA.table[usize::from(second)]
                                    & unsafe {
                                        *(UTF8_DATA.table.get_unchecked(byte as usize + 0x80))
                                    },
                            ) | u16::from(third >> 6) | (u16::from(fourth & 0xC0) << 2))
                                != 0x202
                            {
                                return true;
                            }
                            read += 4;
                        }
                        0xF0 => {
                            let second = unsafe { *(src.get_unchecked(read + 1)) };
                            let third = unsafe { *(src.get_unchecked(read + 2)) };
                            let fourth = unsafe { *(src.get_unchecked(read + 3)) };
                            if (u16::from(
                                UTF8_DATA.table[usize::from(second)]
                                    & unsafe {
                                        *(UTF8_DATA.table.get_unchecked(byte as usize + 0x80))
                                    },
                            ) | u16::from(third >> 6) | (u16::from(fourth & 0xC0) << 2))
                                != 0x202
                            {
                                return true;
                            }
                            if unsafe { unlikely(second == 0x90 || second == 0x9E) } {
                                let third = src[read + 2];
                                if third >= 0xA0 {
                                    return true;
                                }
                            }
                            read += 4;
                        }
                        _ => {
                            return true;
                        }
                    }
                    if read + 4 > src.len() {
                        if read == src.len() {
                            return false;
                        }
                        byte = src[read];
                        break 'inner;
                    }
                    byte = src[read];
                    continue 'inner;
                }
            }
            match byte {
                0..=0x7F => {
                    read += 1;
                    src = &src[read..];
                    continue 'outer;
                }
                0xC2..=0xD5 => {
                    let new_read = read + 2;
                    if new_read > src.len() {
                        return true;
                    }
                    let second = unsafe { *(src.get_unchecked(read + 1)) };
                    if !in_inclusive_range8(second, 0x80, 0xBF) {
                        return true;
                    }
                    read = new_read;
                    src = &src[read..];
                    continue 'outer;
                }
                0xD6 => {
                    let new_read = read + 2;
                    if new_read > src.len() {
                        return true;
                    }
                    let second = unsafe { *(src.get_unchecked(read + 1)) };
                    if !in_inclusive_range8(second, 0x80, 0xBF) {
                        return true;
                    }
                    if second > 0x8F {
                        return true;
                    }
                    read = new_read;
                    src = &src[read..];
                    continue 'outer;
                }
                0xE1 | 0xE3..=0xEC | 0xEE => {
                    let new_read = read + 3;
                    if new_read > src.len() {
                        return true;
                    }
                    let second = unsafe { *(src.get_unchecked(read + 1)) };
                    let third = unsafe { *(src.get_unchecked(read + 2)) };
                    if ((UTF8_DATA.table[usize::from(second)]
                        & unsafe {
                            *(UTF8_DATA.table.get_unchecked(byte as usize + 0x80))
                        }) | (third >> 6)) != 2
                    {
                        return true;
                    }
                }
                0xE2 => {
                    let new_read = read + 3;
                    if new_read > src.len() {
                        return true;
                    }
                    let second = unsafe { *(src.get_unchecked(read + 1)) };
                    let third = unsafe { *(src.get_unchecked(read + 2)) };
                    if ((UTF8_DATA.table[usize::from(second)]
                        & unsafe {
                            *(UTF8_DATA.table.get_unchecked(byte as usize + 0x80))
                        }) | (third >> 6)) != 2
                    {
                        return true;
                    }
                    if second == 0x80 {
                        if third == 0x8F || third == 0xAB || third == 0xAE {
                            return true;
                        }
                    } else if second == 0x81 {
                        if third == 0xA7 {
                            return true;
                        }
                    }
                }
                0xEF => {
                    let new_read = read + 3;
                    if new_read > src.len() {
                        return true;
                    }
                    let second = unsafe { *(src.get_unchecked(read + 1)) };
                    let third = unsafe { *(src.get_unchecked(read + 2)) };
                    if ((UTF8_DATA.table[usize::from(second)]
                        & unsafe {
                            *(UTF8_DATA.table.get_unchecked(byte as usize + 0x80))
                        }) | (third >> 6)) != 2
                    {
                        return true;
                    }
                    if in_inclusive_range8(second, 0xAC, 0xB7) {
                        if second == 0xAC {
                            if third > 0x9C {
                                return true;
                            }
                        } else {
                            return true;
                        }
                    } else if in_inclusive_range8(second, 0xB9, 0xBB) {
                        if second == 0xB9 {
                            if third > 0xAF {
                                return true;
                            }
                        } else if second == 0xBB {
                            if third != 0xBF {
                                return true;
                            }
                        } else {
                            return true;
                        }
                    }
                }
                0xE0 => {
                    let new_read = read + 3;
                    if new_read > src.len() {
                        return true;
                    }
                    let second = unsafe { *(src.get_unchecked(read + 1)) };
                    let third = unsafe { *(src.get_unchecked(read + 2)) };
                    if ((UTF8_DATA.table[usize::from(second)]
                        & unsafe {
                            *(UTF8_DATA.table.get_unchecked(byte as usize + 0x80))
                        }) | (third >> 6)) != 2
                    {
                        return true;
                    }
                    if second < 0xA4 {
                        return true;
                    }
                }
                0xED => {
                    let new_read = read + 3;
                    if new_read > src.len() {
                        return true;
                    }
                    let second = unsafe { *(src.get_unchecked(read + 1)) };
                    let third = unsafe { *(src.get_unchecked(read + 2)) };
                    if ((UTF8_DATA.table[usize::from(second)]
                        & unsafe {
                            *(UTF8_DATA.table.get_unchecked(byte as usize + 0x80))
                        }) | (third >> 6)) != 2
                    {
                        return true;
                    }
                }
                _ => {
                    return true;
                }
            }
            return false;
        } else {
            return false;
        }
    }
}
/// Checks whether a valid UTF-8 buffer contains code points that trigger
/// right-to-left processing.
///
/// The check is done on a Unicode block basis without regard to assigned
/// vs. unassigned code points in the block. Hebrew presentation forms in
/// the Alphabetic Presentation Forms block are treated as if they formed
/// a block on their own (i.e. it treated as right-to-left). Additionally,
/// the four RIGHT-TO-LEFT FOO controls in General Punctuation are checked
/// for. Control characters that are technically bidi controls but do not
/// cause right-to-left behavior without the presence of right-to-left
/// characters or right-to-left controls are not checked for. As a special
/// case, U+FEFF is excluded from Arabic Presentation Forms-B.
#[cfg_attr(feature = "cargo-clippy", allow(collapsible_if))]
#[inline]
pub fn is_str_bidi(buffer: &str) -> bool {
    let mut bytes = buffer.as_bytes();
    'outer: loop {
        if let Some((mut byte, mut read)) = validate_ascii(bytes) {
            'inner: loop {
                if byte < 0xE0 {
                    if byte >= 0x80 {
                        if unsafe { unlikely(byte >= 0xD6) } {
                            if byte == 0xD6 {
                                let second = bytes[read + 1];
                                if second > 0x8F {
                                    return true;
                                }
                            } else {
                                return true;
                            }
                        }
                        read += 2;
                    } else {
                        read += 1;
                        bytes = &bytes[read..];
                        continue 'outer;
                    }
                } else if byte < 0xF0 {
                    if unsafe {
                        unlikely(!in_inclusive_range8(byte, 0xE3, 0xEE) && byte != 0xE1)
                    } {
                        let second = bytes[read + 1];
                        if byte == 0xE0 {
                            if second < 0xA4 {
                                return true;
                            }
                        } else if byte == 0xE2 {
                            let third = bytes[read + 2];
                            if second == 0x80 {
                                if third == 0x8F || third == 0xAB || third == 0xAE {
                                    return true;
                                }
                            } else if second == 0x81 {
                                if third == 0xA7 {
                                    return true;
                                }
                            }
                        } else {
                            debug_assert_eq!(byte, 0xEF);
                            if in_inclusive_range8(second, 0xAC, 0xB7) {
                                if second == 0xAC {
                                    let third = bytes[read + 2];
                                    if third > 0x9C {
                                        return true;
                                    }
                                } else {
                                    return true;
                                }
                            } else if in_inclusive_range8(second, 0xB9, 0xBB) {
                                if second == 0xB9 {
                                    let third = bytes[read + 2];
                                    if third > 0xAF {
                                        return true;
                                    }
                                } else if second == 0xBB {
                                    let third = bytes[read + 2];
                                    if third != 0xBF {
                                        return true;
                                    }
                                } else {
                                    return true;
                                }
                            }
                        }
                    }
                    read += 3;
                } else {
                    let second = bytes[read + 1];
                    if unsafe {
                        unlikely(byte == 0xF0 && (second == 0x90 || second == 0x9E))
                    } {
                        let third = bytes[read + 2];
                        if third >= 0xA0 {
                            return true;
                        }
                    }
                    read += 4;
                }
                if read >= bytes.len() {
                    return false;
                }
                byte = bytes[read];
                continue 'inner;
            }
        } else {
            return false;
        }
    }
}
/// Checks whether a UTF-16 buffer contains code points that trigger
/// right-to-left processing.
///
/// The check is done on a Unicode block basis without regard to assigned
/// vs. unassigned code points in the block. Hebrew presentation forms in
/// the Alphabetic Presentation Forms block are treated as if they formed
/// a block on their own (i.e. it treated as right-to-left). Additionally,
/// the four RIGHT-TO-LEFT FOO controls in General Punctuation are checked
/// for. Control characters that are technically bidi controls but do not
/// cause right-to-left behavior without the presence of right-to-left
/// characters or right-to-left controls are not checked for. As a special
/// case, U+FEFF is excluded from Arabic Presentation Forms-B.
///
/// Returns `true` if the input contains an RTL character or an unpaired
/// high surrogate that could be the high half of an RTL character.
/// Returns `false` if the input contains neither RTL characters nor
/// unpaired high surrogates that could be higher halves of RTL characters.
pub fn is_utf16_bidi(buffer: &[u16]) -> bool {
    is_utf16_bidi_impl(buffer)
}
/// Checks whether a scalar value triggers right-to-left processing.
///
/// The check is done on a Unicode block basis without regard to assigned
/// vs. unassigned code points in the block. Hebrew presentation forms in
/// the Alphabetic Presentation Forms block are treated as if they formed
/// a block on their own (i.e. it treated as right-to-left). Additionally,
/// the four RIGHT-TO-LEFT FOO controls in General Punctuation are checked
/// for. Control characters that are technically bidi controls but do not
/// cause right-to-left behavior without the presence of right-to-left
/// characters or right-to-left controls are not checked for. As a special
/// case, U+FEFF is excluded from Arabic Presentation Forms-B.
#[inline(always)]
pub fn is_char_bidi(c: char) -> bool {
    let code_point = u32::from(c);
    if code_point < 0x0590 {
        return false;
    }
    if in_range32(code_point, 0x0900, 0xFB1D) {
        if in_inclusive_range32(code_point, 0x200F, 0x2067) {
            return code_point == 0x200F || code_point == 0x202B || code_point == 0x202E
                || code_point == 0x2067;
        }
        return false;
    }
    if code_point > 0x1EFFF {
        return false;
    }
    if in_range32(code_point, 0x11000, 0x1E800) {
        return false;
    }
    if in_range32(code_point, 0xFEFF, 0x10800) {
        return false;
    }
    if in_range32(code_point, 0xFE00, 0xFE70) {
        return false;
    }
    true
}
/// Checks whether a UTF-16 code unit triggers right-to-left processing.
///
/// The check is done on a Unicode block basis without regard to assigned
/// vs. unassigned code points in the block. Hebrew presentation forms in
/// the Alphabetic Presentation Forms block are treated as if they formed
/// a block on their own (i.e. it treated as right-to-left). Additionally,
/// the four RIGHT-TO-LEFT FOO controls in General Punctuation are checked
/// for. Control characters that are technically bidi controls but do not
/// cause right-to-left behavior without the presence of right-to-left
/// characters or right-to-left controls are not checked for. As a special
/// case, U+FEFF is excluded from Arabic Presentation Forms-B.
///
/// Since supplementary-plane right-to-left blocks are identifiable from the
/// high surrogate without examining the low surrogate, this function returns
/// `true` for such high surrogates making the function suitable for handling
/// supplementary-plane text without decoding surrogate pairs to scalar
/// values. Obviously, such high surrogates are then reported as right-to-left
/// even if actually unpaired.
#[inline(always)]
pub fn is_utf16_code_unit_bidi(u: u16) -> bool {
    if u < 0x0590 {
        return false;
    }
    if in_range16(u, 0x0900, 0xD802) {
        if in_inclusive_range16(u, 0x200F, 0x2067) {
            return u == 0x200F || u == 0x202B || u == 0x202E || u == 0x2067;
        }
        return false;
    }
    if in_range16(u, 0xD83C, 0xFB1D) {
        return false;
    }
    if in_range16(u, 0xD804, 0xD83A) {
        return false;
    }
    if u > 0xFEFE {
        return false;
    }
    if in_range16(u, 0xFE00, 0xFE70) {
        return false;
    }
    true
}
/// Checks whether a potentially invalid UTF-8 buffer contains code points
/// that trigger right-to-left processing or is all-Latin1.
///
/// Possibly more efficient than performing the checks separately.
///
/// Returns `Latin1Bidi::Latin1` if `is_utf8_latin1()` would return `true`.
/// Otherwise, returns `Latin1Bidi::Bidi` if `is_utf8_bidi()` would return
/// `true`. Otherwise, returns `Latin1Bidi::LeftToRight`.
pub fn check_utf8_for_latin1_and_bidi(buffer: &[u8]) -> Latin1Bidi {
    if let Some(offset) = is_utf8_latin1_impl(buffer) {
        if is_utf8_bidi(&buffer[offset..]) {
            Latin1Bidi::Bidi
        } else {
            Latin1Bidi::LeftToRight
        }
    } else {
        Latin1Bidi::Latin1
    }
}
/// Checks whether a valid UTF-8 buffer contains code points
/// that trigger right-to-left processing or is all-Latin1.
///
/// Possibly more efficient than performing the checks separately.
///
/// Returns `Latin1Bidi::Latin1` if `is_str_latin1()` would return `true`.
/// Otherwise, returns `Latin1Bidi::Bidi` if `is_str_bidi()` would return
/// `true`. Otherwise, returns `Latin1Bidi::LeftToRight`.
pub fn check_str_for_latin1_and_bidi(buffer: &str) -> Latin1Bidi {
    if let Some(offset) = is_str_latin1_impl(buffer) {
        if is_str_bidi(&buffer[offset..]) {
            Latin1Bidi::Bidi
        } else {
            Latin1Bidi::LeftToRight
        }
    } else {
        Latin1Bidi::Latin1
    }
}
/// Checks whether a potentially invalid UTF-16 buffer contains code points
/// that trigger right-to-left processing or is all-Latin1.
///
/// Possibly more efficient than performing the checks separately.
///
/// Returns `Latin1Bidi::Latin1` if `is_utf16_latin1()` would return `true`.
/// Otherwise, returns `Latin1Bidi::Bidi` if `is_utf16_bidi()` would return
/// `true`. Otherwise, returns `Latin1Bidi::LeftToRight`.
pub fn check_utf16_for_latin1_and_bidi(buffer: &[u16]) -> Latin1Bidi {
    check_utf16_for_latin1_and_bidi_impl(buffer)
}
/// Converts potentially-invalid UTF-8 to valid UTF-16 with errors replaced
/// with the REPLACEMENT CHARACTER.
///
/// The length of the destination buffer must be at least the length of the
/// source buffer _plus one_.
///
/// Returns the number of `u16`s written.
///
/// # Panics
///
/// Panics if the destination buffer is shorter than stated above.
pub fn convert_utf8_to_utf16(src: &[u8], dst: &mut [u16]) -> usize {
    assert!(dst.len() > src.len());
    let mut decoder = Utf8Decoder::new_inner();
    let mut total_read = 0usize;
    let mut total_written = 0usize;
    loop {
        let (result, read, written) = decoder
            .decode_to_utf16_raw(&src[total_read..], &mut dst[total_written..], true);
        total_read += read;
        total_written += written;
        match result {
            DecoderResult::InputEmpty => {
                return total_written;
            }
            DecoderResult::OutputFull => {
                unreachable!(
                    "The assert at the top of the function should have caught this."
                );
            }
            DecoderResult::Malformed(_, _) => {
                dst[total_written] = 0xFFFD;
                total_written += 1;
            }
        }
    }
}
/// Converts valid UTF-8 to valid UTF-16.
///
/// The length of the destination buffer must be at least the length of the
/// source buffer.
///
/// Returns the number of `u16`s written.
///
/// # Panics
///
/// Panics if the destination buffer is shorter than stated above.
pub fn convert_str_to_utf16(src: &str, dst: &mut [u16]) -> usize {
    assert!(dst.len() >= src.len(), "Destination must not be shorter than the source.");
    let bytes = src.as_bytes();
    let mut read = 0;
    let mut written = 0;
    'outer: loop {
        let mut byte = {
            let src_remaining = &bytes[read..];
            let dst_remaining = &mut dst[written..];
            let length = src_remaining.len();
            match unsafe {
                ascii_to_basic_latin(
                    src_remaining.as_ptr(),
                    dst_remaining.as_mut_ptr(),
                    length,
                )
            } {
                None => {
                    written += length;
                    return written;
                }
                Some((non_ascii, consumed)) => {
                    read += consumed;
                    written += consumed;
                    non_ascii
                }
            }
        };
        'inner: loop {
            if byte < 0xE0 {
                if byte >= 0x80 {
                    let second = unsafe { *(bytes.get_unchecked(read + 1)) };
                    let point = ((u16::from(byte) & 0x1F) << 6)
                        | (u16::from(second) & 0x3F);
                    unsafe { *(dst.get_unchecked_mut(written)) = point };
                    read += 2;
                    written += 1;
                } else {
                    unsafe { *(dst.get_unchecked_mut(written)) = u16::from(byte) };
                    read += 1;
                    written += 1;
                    continue 'outer;
                }
            } else if byte < 0xF0 {
                let second = unsafe { *(bytes.get_unchecked(read + 1)) };
                let third = unsafe { *(bytes.get_unchecked(read + 2)) };
                let point = ((u16::from(byte) & 0xF) << 12)
                    | ((u16::from(second) & 0x3F) << 6) | (u16::from(third) & 0x3F);
                unsafe { *(dst.get_unchecked_mut(written)) = point };
                read += 3;
                written += 1;
            } else {
                let second = unsafe { *(bytes.get_unchecked(read + 1)) };
                let third = unsafe { *(bytes.get_unchecked(read + 2)) };
                let fourth = unsafe { *(bytes.get_unchecked(read + 3)) };
                let point = ((u32::from(byte) & 0x7) << 18)
                    | ((u32::from(second) & 0x3F) << 12)
                    | ((u32::from(third) & 0x3F) << 6) | (u32::from(fourth) & 0x3F);
                unsafe {
                    *(dst.get_unchecked_mut(written)) = (0xD7C0 + (point >> 10)) as u16
                };
                unsafe {
                    *(dst
                        .get_unchecked_mut(
                            written + 1,
                        )) = (0xDC00 + (point & 0x3FF)) as u16
                };
                read += 4;
                written += 2;
            }
            if read >= src.len() {
                return written;
            }
            byte = bytes[read];
            continue 'inner;
        }
    }
}
/// Converts potentially-invalid UTF-8 to valid UTF-16 signaling on error.
///
/// The length of the destination buffer must be at least the length of the
/// source buffer.
///
/// Returns the number of `u16`s written or `None` if the input was invalid.
///
/// When the input was invalid, some output may have been written.
///
/// # Panics
///
/// Panics if the destination buffer is shorter than stated above.
pub fn convert_utf8_to_utf16_without_replacement(
    src: &[u8],
    dst: &mut [u16],
) -> Option<usize> {
    assert!(dst.len() >= src.len(), "Destination must not be shorter than the source.");
    let (read, written) = convert_utf8_to_utf16_up_to_invalid(src, dst);
    if read == src.len() {
        return Some(written);
    }
    None
}
/// Converts potentially-invalid UTF-16 to valid UTF-8 with errors replaced
/// with the REPLACEMENT CHARACTER with potentially insufficient output
/// space.
///
/// Returns the number of code units read and the number of bytes written.
///
/// Guarantees that the bytes in the destination beyond the number of
/// bytes claimed as written by the second item of the return tuple
/// are left unmodified.
///
/// Not all code units are read if there isn't enough output space.
///
/// Note  that this method isn't designed for general streamability but for
/// not allocating memory for the worst case up front. Specifically,
/// if the input starts with or ends with an unpaired surrogate, those are
/// replaced with the REPLACEMENT CHARACTER.
///
/// Matches the semantics of `TextEncoder.encodeInto()` from the
/// Encoding Standard.
///
/// # Safety
///
/// If you want to convert into a `&mut str`, use
/// `convert_utf16_to_str_partial()` instead of using this function
/// together with the `unsafe` method `as_bytes_mut()` on `&mut str`.
#[inline(always)]
pub fn convert_utf16_to_utf8_partial(src: &[u16], dst: &mut [u8]) -> (usize, usize) {
    let (read, written) = convert_utf16_to_utf8_partial_inner(src, dst);
    if unsafe { likely(read == src.len()) } {
        return (read, written);
    }
    let (tail_read, tail_written) = convert_utf16_to_utf8_partial_tail(
        &src[read..],
        &mut dst[written..],
    );
    (read + tail_read, written + tail_written)
}
/// Converts potentially-invalid UTF-16 to valid UTF-8 with errors replaced
/// with the REPLACEMENT CHARACTER.
///
/// The length of the destination buffer must be at least the length of the
/// source buffer times three.
///
/// Returns the number of bytes written.
///
/// # Panics
///
/// Panics if the destination buffer is shorter than stated above.
///
/// # Safety
///
/// If you want to convert into a `&mut str`, use `convert_utf16_to_str()`
/// instead of using this function together with the `unsafe` method
/// `as_bytes_mut()` on `&mut str`.
#[inline(always)]
pub fn convert_utf16_to_utf8(src: &[u16], dst: &mut [u8]) -> usize {
    assert!(dst.len() >= src.len() * 3);
    let (read, written) = convert_utf16_to_utf8_partial(src, dst);
    debug_assert_eq!(read, src.len());
    written
}
/// Converts potentially-invalid UTF-16 to valid UTF-8 with errors replaced
/// with the REPLACEMENT CHARACTER such that the validity of the output is
/// signaled using the Rust type system with potentially insufficient output
/// space.
///
/// Returns the number of code units read and the number of bytes written.
///
/// Not all code units are read if there isn't enough output space.
///
/// Note  that this method isn't designed for general streamability but for
/// not allocating memory for the worst case up front. Specifically,
/// if the input starts with or ends with an unpaired surrogate, those are
/// replaced with the REPLACEMENT CHARACTER.
pub fn convert_utf16_to_str_partial(src: &[u16], dst: &mut str) -> (usize, usize) {
    let bytes: &mut [u8] = unsafe { dst.as_bytes_mut() };
    let (read, written) = convert_utf16_to_utf8_partial(src, bytes);
    let len = bytes.len();
    let mut trail = written;
    while trail < len && ((bytes[trail] & 0xC0) == 0x80) {
        bytes[trail] = 0;
        trail += 1;
    }
    (read, written)
}
/// Converts potentially-invalid UTF-16 to valid UTF-8 with errors replaced
/// with the REPLACEMENT CHARACTER such that the validity of the output is
/// signaled using the Rust type system.
///
/// The length of the destination buffer must be at least the length of the
/// source buffer times three.
///
/// Returns the number of bytes written.
///
/// # Panics
///
/// Panics if the destination buffer is shorter than stated above.
#[inline(always)]
pub fn convert_utf16_to_str(src: &[u16], dst: &mut str) -> usize {
    assert!(dst.len() >= src.len() * 3);
    let (read, written) = convert_utf16_to_str_partial(src, dst);
    debug_assert_eq!(read, src.len());
    written
}
/// Converts bytes whose unsigned value is interpreted as Unicode code point
/// (i.e. U+0000 to U+00FF, inclusive) to UTF-16.
///
/// The length of the destination buffer must be at least the length of the
/// source buffer.
///
/// The number of `u16`s written equals the length of the source buffer.
///
/// # Panics
///
/// Panics if the destination buffer is shorter than stated above.
pub fn convert_latin1_to_utf16(src: &[u8], dst: &mut [u16]) {
    assert!(dst.len() >= src.len(), "Destination must not be shorter than the source.");
    unsafe {
        unpack_latin1(src.as_ptr(), dst.as_mut_ptr(), src.len());
    }
}
/// Converts bytes whose unsigned value is interpreted as Unicode code point
/// (i.e. U+0000 to U+00FF, inclusive) to UTF-8 with potentially insufficient
/// output space.
///
/// Returns the number of bytes read and the number of bytes written.
///
/// If the output isn't large enough, not all input is consumed.
///
/// # Safety
///
/// If you want to convert into a `&mut str`, use
/// `convert_utf16_to_str_partial()` instead of using this function
/// together with the `unsafe` method `as_bytes_mut()` on `&mut str`.
pub fn convert_latin1_to_utf8_partial(src: &[u8], dst: &mut [u8]) -> (usize, usize) {
    let src_len = src.len();
    let src_ptr = src.as_ptr();
    let dst_ptr = dst.as_mut_ptr();
    let dst_len = dst.len();
    let mut total_read = 0usize;
    let mut total_written = 0usize;
    loop {
        let src_left = src_len - total_read;
        let dst_left = dst_len - total_written;
        let min_left = ::std::cmp::min(src_left, dst_left);
        if let Some((non_ascii, consumed))
            = unsafe {
                ascii_to_ascii(
                    src_ptr.add(total_read),
                    dst_ptr.add(total_written),
                    min_left,
                )
            } {
            total_read += consumed;
            total_written += consumed;
            if total_written.checked_add(2).unwrap() > dst_len {
                return (total_read, total_written);
            }
            total_read += 1;
            dst[total_written] = (non_ascii >> 6) | 0xC0;
            total_written += 1;
            dst[total_written] = (non_ascii & 0x3F) | 0x80;
            total_written += 1;
            continue;
        }
        return (total_read + min_left, total_written + min_left);
    }
}
/// Converts bytes whose unsigned value is interpreted as Unicode code point
/// (i.e. U+0000 to U+00FF, inclusive) to UTF-8.
///
/// The length of the destination buffer must be at least the length of the
/// source buffer times two.
///
/// Returns the number of bytes written.
///
/// # Panics
///
/// Panics if the destination buffer is shorter than stated above.
///
/// # Safety
///
/// Note that this function may write garbage beyond the number of bytes
/// indicated by the return value, so using a `&mut str` interpreted as
/// `&mut [u8]` as the destination is not safe. If you want to convert into
/// a `&mut str`, use `convert_utf16_to_str()` instead of this function.
#[inline]
pub fn convert_latin1_to_utf8(src: &[u8], dst: &mut [u8]) -> usize {
    assert!(
        dst.len() >= src.len() * 2,
        "Destination must not be shorter than the source times two."
    );
    let (read, written) = convert_latin1_to_utf8_partial(src, dst);
    debug_assert_eq!(read, src.len());
    written
}
/// Converts bytes whose unsigned value is interpreted as Unicode code point
/// (i.e. U+0000 to U+00FF, inclusive) to UTF-8 such that the validity of the
/// output is signaled using the Rust type system with potentially insufficient
/// output space.
///
/// Returns the number of bytes read and the number of bytes written.
///
/// If the output isn't large enough, not all input is consumed.
#[inline]
pub fn convert_latin1_to_str_partial(src: &[u8], dst: &mut str) -> (usize, usize) {
    let bytes: &mut [u8] = unsafe { dst.as_bytes_mut() };
    let (read, written) = convert_latin1_to_utf8_partial(src, bytes);
    let len = bytes.len();
    let mut trail = written;
    let max = ::std::cmp::min(len, trail + MAX_STRIDE_SIZE);
    while trail < max {
        bytes[trail] = 0;
        trail += 1;
    }
    while trail < len && ((bytes[trail] & 0xC0) == 0x80) {
        bytes[trail] = 0;
        trail += 1;
    }
    (read, written)
}
/// Converts bytes whose unsigned value is interpreted as Unicode code point
/// (i.e. U+0000 to U+00FF, inclusive) to UTF-8 such that the validity of the
/// output is signaled using the Rust type system.
///
/// The length of the destination buffer must be at least the length of the
/// source buffer times two.
///
/// Returns the number of bytes written.
///
/// # Panics
///
/// Panics if the destination buffer is shorter than stated above.
#[inline]
pub fn convert_latin1_to_str(src: &[u8], dst: &mut str) -> usize {
    assert!(
        dst.len() >= src.len() * 2,
        "Destination must not be shorter than the source times two."
    );
    let (read, written) = convert_latin1_to_str_partial(src, dst);
    debug_assert_eq!(read, src.len());
    written
}
/// If the input is valid UTF-8 representing only Unicode code points from
/// U+0000 to U+00FF, inclusive, converts the input into output that
/// represents the value of each code point as the unsigned byte value of
/// each output byte.
///
/// If the input does not fulfill the condition stated above, this function
/// panics if debug assertions are enabled (and fuzzing isn't) and otherwise
/// does something that is memory-safe without any promises about any
/// properties of the output. In particular, callers shouldn't assume the
/// output to be the same across crate versions or CPU architectures and
/// should not assume that non-ASCII input can't map to ASCII output.
///
/// The length of the destination buffer must be at least the length of the
/// source buffer.
///
/// Returns the number of bytes written.
///
/// # Panics
///
/// Panics if the destination buffer is shorter than stated above.
///
/// If debug assertions are enabled (and not fuzzing) and the input is
/// not in the range U+0000 to U+00FF, inclusive.
pub fn convert_utf8_to_latin1_lossy(src: &[u8], dst: &mut [u8]) -> usize {
    assert!(dst.len() >= src.len(), "Destination must not be shorter than the source.");
    non_fuzz_debug_assert!(is_utf8_latin1(src));
    let src_len = src.len();
    let src_ptr = src.as_ptr();
    let dst_ptr = dst.as_mut_ptr();
    let mut total_read = 0usize;
    let mut total_written = 0usize;
    loop {
        let src_left = src_len - total_read;
        if let Some((non_ascii, consumed))
            = unsafe {
                ascii_to_ascii(
                    src_ptr.add(total_read),
                    dst_ptr.add(total_written),
                    src_left,
                )
            } {
            total_read += consumed + 1;
            total_written += consumed;
            if total_read == src_len {
                return total_written;
            }
            let trail = src[total_read];
            total_read += 1;
            dst[total_written] = ((non_ascii & 0x1F) << 6) | (trail & 0x3F);
            total_written += 1;
            continue;
        }
        return total_written + src_left;
    }
}
/// If the input is valid UTF-16 representing only Unicode code points from
/// U+0000 to U+00FF, inclusive, converts the input into output that
/// represents the value of each code point as the unsigned byte value of
/// each output byte.
///
/// If the input does not fulfill the condition stated above, does something
/// that is memory-safe without any promises about any properties of the
/// output and will probably assert in debug builds in future versions.
/// In particular, callers shouldn't assume the output to be the same across
/// crate versions or CPU architectures and should not assume that non-ASCII
/// input can't map to ASCII output.
///
/// The length of the destination buffer must be at least the length of the
/// source buffer.
///
/// The number of bytes written equals the length of the source buffer.
///
/// # Panics
///
/// Panics if the destination buffer is shorter than stated above.
///
/// (Probably in future versions if debug assertions are enabled (and not
/// fuzzing) and the input is not in the range U+0000 to U+00FF, inclusive.)
pub fn convert_utf16_to_latin1_lossy(src: &[u16], dst: &mut [u8]) {
    assert!(dst.len() >= src.len(), "Destination must not be shorter than the source.");
    unsafe {
        pack_latin1(src.as_ptr(), dst.as_mut_ptr(), src.len());
    }
}
/// Converts bytes whose unsigned value is interpreted as Unicode code point
/// (i.e. U+0000 to U+00FF, inclusive) to UTF-8.
///
/// Borrows if input is ASCII-only. Performs a single heap allocation
/// otherwise.
pub fn decode_latin1<'a>(bytes: &'a [u8]) -> Cow<'a, str> {
    let up_to = ascii_valid_up_to(bytes);
    if up_to >= bytes.len() {
        debug_assert_eq!(up_to, bytes.len());
        let s: &str = unsafe { ::std::str::from_utf8_unchecked(bytes) };
        return Cow::Borrowed(s);
    }
    let (head, tail) = bytes.split_at(up_to);
    let capacity = head.len() + tail.len() * 2;
    let mut vec = Vec::with_capacity(capacity);
    unsafe {
        vec.set_len(capacity);
    }
    (&mut vec[..up_to]).copy_from_slice(head);
    let written = convert_latin1_to_utf8(tail, &mut vec[up_to..]);
    vec.truncate(up_to + written);
    Cow::Owned(unsafe { String::from_utf8_unchecked(vec) })
}
/// If the input is valid UTF-8 representing only Unicode code points from
/// U+0000 to U+00FF, inclusive, converts the input into output that
/// represents the value of each code point as the unsigned byte value of
/// each output byte.
///
/// If the input does not fulfill the condition stated above, this function
/// panics if debug assertions are enabled (and fuzzing isn't) and otherwise
/// does something that is memory-safe without any promises about any
/// properties of the output. In particular, callers shouldn't assume the
/// output to be the same across crate versions or CPU architectures and
/// should not assume that non-ASCII input can't map to ASCII output.
///
/// Borrows if input is ASCII-only. Performs a single heap allocation
/// otherwise.
pub fn encode_latin1_lossy<'a>(string: &'a str) -> Cow<'a, [u8]> {
    let bytes = string.as_bytes();
    let up_to = ascii_valid_up_to(bytes);
    if up_to >= bytes.len() {
        debug_assert_eq!(up_to, bytes.len());
        return Cow::Borrowed(bytes);
    }
    let (head, tail) = bytes.split_at(up_to);
    let capacity = bytes.len();
    let mut vec = Vec::with_capacity(capacity);
    unsafe {
        vec.set_len(capacity);
    }
    (&mut vec[..up_to]).copy_from_slice(head);
    let written = convert_utf8_to_latin1_lossy(tail, &mut vec[up_to..]);
    vec.truncate(up_to + written);
    Cow::Owned(vec)
}
/// Returns the index of the first unpaired surrogate or, if the input is
/// valid UTF-16 in its entirety, the length of the input.
pub fn utf16_valid_up_to(buffer: &[u16]) -> usize {
    utf16_valid_up_to_impl(buffer)
}
/// Returns the index of first byte that starts an invalid byte
/// sequence or a non-Latin1 byte sequence, or the length of the
/// string if there are neither.
pub fn utf8_latin1_up_to(buffer: &[u8]) -> usize {
    is_utf8_latin1_impl(buffer).unwrap_or(buffer.len())
}
/// Returns the index of first byte that starts a non-Latin1 byte
/// sequence, or the length of the string if there are none.
pub fn str_latin1_up_to(buffer: &str) -> usize {
    is_str_latin1_impl(buffer).unwrap_or(buffer.len())
}
/// Replaces unpaired surrogates in the input with the REPLACEMENT CHARACTER.
#[inline]
pub fn ensure_utf16_validity(buffer: &mut [u16]) {
    let mut offset = 0;
    loop {
        offset += utf16_valid_up_to(&buffer[offset..]);
        if offset == buffer.len() {
            return;
        }
        buffer[offset] = 0xFFFD;
        offset += 1;
    }
}
/// Copies ASCII from source to destination up to the first non-ASCII byte
/// (or the end of the input if it is ASCII in its entirety).
///
/// The length of the destination buffer must be at least the length of the
/// source buffer.
///
/// Returns the number of bytes written.
///
/// # Panics
///
/// Panics if the destination buffer is shorter than stated above.
pub fn copy_ascii_to_ascii(src: &[u8], dst: &mut [u8]) -> usize {
    assert!(dst.len() >= src.len(), "Destination must not be shorter than the source.");
    if let Some((_, consumed))
        = unsafe { ascii_to_ascii(src.as_ptr(), dst.as_mut_ptr(), src.len()) } {
        consumed
    } else {
        src.len()
    }
}
/// Copies ASCII from source to destination zero-extending it to UTF-16 up to
/// the first non-ASCII byte (or the end of the input if it is ASCII in its
/// entirety).
///
/// The length of the destination buffer must be at least the length of the
/// source buffer.
///
/// Returns the number of `u16`s written.
///
/// # Panics
///
/// Panics if the destination buffer is shorter than stated above.
pub fn copy_ascii_to_basic_latin(src: &[u8], dst: &mut [u16]) -> usize {
    assert!(dst.len() >= src.len(), "Destination must not be shorter than the source.");
    if let Some((_, consumed))
        = unsafe { ascii_to_basic_latin(src.as_ptr(), dst.as_mut_ptr(), src.len()) } {
        consumed
    } else {
        src.len()
    }
}
/// Copies Basic Latin from source to destination narrowing it to ASCII up to
/// the first non-Basic Latin code unit (or the end of the input if it is
/// Basic Latin in its entirety).
///
/// The length of the destination buffer must be at least the length of the
/// source buffer.
///
/// Returns the number of bytes written.
///
/// # Panics
///
/// Panics if the destination buffer is shorter than stated above.
pub fn copy_basic_latin_to_ascii(src: &[u16], dst: &mut [u8]) -> usize {
    assert!(dst.len() >= src.len(), "Destination must not be shorter than the source.");
    if let Some((_, consumed))
        = unsafe { basic_latin_to_ascii(src.as_ptr(), dst.as_mut_ptr(), src.len()) } {
        consumed
    } else {
        src.len()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_is_ascii_success() {
        let mut src: Vec<u8> = Vec::with_capacity(128);
        src.resize(128, 0);
        for i in 0..src.len() {
            src[i] = i as u8;
        }
        for i in 0..src.len() {
            assert!(is_ascii(& src[i..]));
        }
    }
    #[test]
    fn test_is_ascii_fail() {
        let mut src: Vec<u8> = Vec::with_capacity(128);
        src.resize(128, 0);
        for i in 0..src.len() {
            src[i] = i as u8;
        }
        for i in 0..src.len() {
            let tail = &mut src[i..];
            for j in 0..tail.len() {
                tail[j] = 0xA0;
                assert!(! is_ascii(tail));
            }
        }
    }
    #[test]
    fn test_is_basic_latin_success() {
        let mut src: Vec<u16> = Vec::with_capacity(128);
        src.resize(128, 0);
        for i in 0..src.len() {
            src[i] = i as u16;
        }
        for i in 0..src.len() {
            assert!(is_basic_latin(& src[i..]));
        }
    }
    #[test]
    fn test_is_basic_latin_fail() {
        let mut src: Vec<u16> = Vec::with_capacity(128);
        src.resize(128, 0);
        for i in 0..src.len() {
            src[i] = i as u16;
        }
        for i in 0..src.len() {
            let tail = &mut src[i..];
            for j in 0..tail.len() {
                tail[j] = 0xA0;
                assert!(! is_basic_latin(tail));
            }
        }
    }
    #[test]
    fn test_is_utf16_latin1_success() {
        let mut src: Vec<u16> = Vec::with_capacity(256);
        src.resize(256, 0);
        for i in 0..src.len() {
            src[i] = i as u16;
        }
        for i in 0..src.len() {
            assert!(is_utf16_latin1(& src[i..]));
            assert_eq!(check_utf16_for_latin1_and_bidi(& src[i..]), Latin1Bidi::Latin1);
        }
    }
    #[test]
    fn test_is_utf16_latin1_fail() {
        let mut src: Vec<u16> = Vec::with_capacity(256);
        src.resize(256, 0);
        for i in 0..src.len() {
            src[i] = i as u16;
        }
        for i in 0..src.len() {
            let tail = &mut src[i..];
            for j in 0..tail.len() {
                tail[j] = 0x100 + j as u16;
                assert!(! is_utf16_latin1(tail));
                assert_ne!(check_utf16_for_latin1_and_bidi(tail), Latin1Bidi::Latin1);
            }
        }
    }
    #[test]
    fn test_is_str_latin1_success() {
        let mut src: Vec<u16> = Vec::with_capacity(256);
        src.resize(256, 0);
        for i in 0..src.len() {
            src[i] = i as u16;
        }
        for i in 0..src.len() {
            let s = String::from_utf16(&src[i..]).unwrap();
            assert!(is_str_latin1(& s[..]));
            assert_eq!(check_str_for_latin1_and_bidi(& s[..]), Latin1Bidi::Latin1);
        }
    }
    #[test]
    fn test_is_str_latin1_fail() {
        let mut src: Vec<u16> = Vec::with_capacity(256);
        src.resize(256, 0);
        for i in 0..src.len() {
            src[i] = i as u16;
        }
        for i in 0..src.len() {
            let tail = &mut src[i..];
            for j in 0..tail.len() {
                tail[j] = 0x100 + j as u16;
                let s = String::from_utf16(tail).unwrap();
                assert!(! is_str_latin1(& s[..]));
                assert_ne!(check_str_for_latin1_and_bidi(& s[..]), Latin1Bidi::Latin1);
            }
        }
    }
    #[test]
    fn test_is_utf8_latin1_success() {
        let mut src: Vec<u16> = Vec::with_capacity(256);
        src.resize(256, 0);
        for i in 0..src.len() {
            src[i] = i as u16;
        }
        for i in 0..src.len() {
            let s = String::from_utf16(&src[i..]).unwrap();
            assert!(is_utf8_latin1(s.as_bytes()));
            assert_eq!(check_utf8_for_latin1_and_bidi(s.as_bytes()), Latin1Bidi::Latin1);
        }
    }
    #[test]
    fn test_is_utf8_latin1_fail() {
        let mut src: Vec<u16> = Vec::with_capacity(256);
        src.resize(256, 0);
        for i in 0..src.len() {
            src[i] = i as u16;
        }
        for i in 0..src.len() {
            let tail = &mut src[i..];
            for j in 0..tail.len() {
                tail[j] = 0x100 + j as u16;
                let s = String::from_utf16(tail).unwrap();
                assert!(! is_utf8_latin1(s.as_bytes()));
                assert_ne!(
                    check_utf8_for_latin1_and_bidi(s.as_bytes()), Latin1Bidi::Latin1
                );
            }
        }
    }
    #[test]
    fn test_is_utf8_latin1_invalid() {
        assert!(! is_utf8_latin1(b"\xC3"));
        assert!(! is_utf8_latin1(b"a\xC3"));
        assert!(! is_utf8_latin1(b"\xFF"));
        assert!(! is_utf8_latin1(b"a\xFF"));
        assert!(! is_utf8_latin1(b"\xC3\xFF"));
        assert!(! is_utf8_latin1(b"a\xC3\xFF"));
    }
    #[test]
    fn test_convert_utf8_to_utf16() {
        let src = "abcdefghijklmnopqrstu\u{1F4A9}v\u{2603}w\u{00B6}xyzz";
        let mut dst: Vec<u16> = Vec::with_capacity(src.len() + 1);
        dst.resize(src.len() + 1, 0);
        let len = convert_utf8_to_utf16(src.as_bytes(), &mut dst[..]);
        dst.truncate(len);
        let reference: Vec<u16> = src.encode_utf16().collect();
        assert_eq!(dst, reference);
    }
    #[test]
    fn test_convert_str_to_utf16() {
        let src = "abcdefghijklmnopqrstu\u{1F4A9}v\u{2603}w\u{00B6}xyzz";
        let mut dst: Vec<u16> = Vec::with_capacity(src.len());
        dst.resize(src.len(), 0);
        let len = convert_str_to_utf16(src, &mut dst[..]);
        dst.truncate(len);
        let reference: Vec<u16> = src.encode_utf16().collect();
        assert_eq!(dst, reference);
    }
    #[test]
    fn test_convert_utf16_to_utf8_partial() {
        let reference = "abcdefghijklmnopqrstu\u{1F4A9}v\u{2603}w\u{00B6}xyzz";
        let src: Vec<u16> = reference.encode_utf16().collect();
        let mut dst: Vec<u8> = Vec::with_capacity(src.len() * 3 + 1);
        dst.resize(src.len() * 3 + 1, 0);
        let (read, written) = convert_utf16_to_utf8_partial(&src[..], &mut dst[..24]);
        let len = written + convert_utf16_to_utf8(&src[read..], &mut dst[written..]);
        dst.truncate(len);
        assert_eq!(dst, reference.as_bytes());
    }
    #[test]
    fn test_convert_utf16_to_utf8() {
        let reference = "abcdefghijklmnopqrstu\u{1F4A9}v\u{2603}w\u{00B6}xyzz";
        let src: Vec<u16> = reference.encode_utf16().collect();
        let mut dst: Vec<u8> = Vec::with_capacity(src.len() * 3 + 1);
        dst.resize(src.len() * 3 + 1, 0);
        let len = convert_utf16_to_utf8(&src[..], &mut dst[..]);
        dst.truncate(len);
        assert_eq!(dst, reference.as_bytes());
    }
    #[test]
    fn test_convert_latin1_to_utf16() {
        let mut src: Vec<u8> = Vec::with_capacity(256);
        src.resize(256, 0);
        let mut reference: Vec<u16> = Vec::with_capacity(256);
        reference.resize(256, 0);
        for i in 0..256 {
            src[i] = i as u8;
            reference[i] = i as u16;
        }
        let mut dst: Vec<u16> = Vec::with_capacity(src.len());
        dst.resize(src.len(), 0);
        convert_latin1_to_utf16(&src[..], &mut dst[..]);
        assert_eq!(dst, reference);
    }
    #[test]
    fn test_convert_latin1_to_utf8_partial() {
        let mut dst = [0u8, 2];
        let (read, written) = convert_latin1_to_utf8_partial(b"a\xFF", &mut dst[..]);
        assert_eq!(read, 1);
        assert_eq!(written, 1);
    }
    #[test]
    fn test_convert_latin1_to_utf8() {
        let mut src: Vec<u8> = Vec::with_capacity(256);
        src.resize(256, 0);
        let mut reference: Vec<u16> = Vec::with_capacity(256);
        reference.resize(256, 0);
        for i in 0..256 {
            src[i] = i as u8;
            reference[i] = i as u16;
        }
        let s = String::from_utf16(&reference[..]).unwrap();
        let mut dst: Vec<u8> = Vec::with_capacity(src.len() * 2);
        dst.resize(src.len() * 2, 0);
        let len = convert_latin1_to_utf8(&src[..], &mut dst[..]);
        dst.truncate(len);
        assert_eq!(& dst[..], s.as_bytes());
    }
    #[test]
    fn test_convert_utf8_to_latin1_lossy() {
        let mut reference: Vec<u8> = Vec::with_capacity(256);
        reference.resize(256, 0);
        let mut src16: Vec<u16> = Vec::with_capacity(256);
        src16.resize(256, 0);
        for i in 0..256 {
            src16[i] = i as u16;
            reference[i] = i as u8;
        }
        let src = String::from_utf16(&src16[..]).unwrap();
        let mut dst: Vec<u8> = Vec::with_capacity(src.len());
        dst.resize(src.len(), 0);
        let len = convert_utf8_to_latin1_lossy(src.as_bytes(), &mut dst[..]);
        dst.truncate(len);
        assert_eq!(dst, reference);
    }
    #[cfg(all(debug_assertions, not(fuzzing)))]
    #[test]
    #[should_panic]
    fn test_convert_utf8_to_latin1_lossy_panics() {
        let mut dst = [0u8; 16];
        let _ = convert_utf8_to_latin1_lossy("\u{100}".as_bytes(), &mut dst[..]);
    }
    #[test]
    fn test_convert_utf16_to_latin1_lossy() {
        let mut src: Vec<u16> = Vec::with_capacity(256);
        src.resize(256, 0);
        let mut reference: Vec<u8> = Vec::with_capacity(256);
        reference.resize(256, 0);
        for i in 0..256 {
            src[i] = i as u16;
            reference[i] = i as u8;
        }
        let mut dst: Vec<u8> = Vec::with_capacity(src.len());
        dst.resize(src.len(), 0);
        convert_utf16_to_latin1_lossy(&src[..], &mut dst[..]);
        assert_eq!(dst, reference);
    }
    #[test]
    fn test_convert_utf16_to_latin1_lossy_panics() {
        let mut dst = [0u8; 16];
        let _ = convert_utf16_to_latin1_lossy(&[0x0100u16], &mut dst[..]);
    }
    #[test]
    fn test_utf16_valid_up_to() {
        let valid = vec![
            0u16, 0u16, 0u16, 0u16, 0u16, 0u16, 0u16, 0u16, 0u16, 0u16, 0u16, 0u16,
            0x2603u16, 0xD83Du16, 0xDCA9u16, 0x00B6u16,
        ];
        assert_eq!(utf16_valid_up_to(& valid[..]), 16);
        let lone_high = vec![
            0u16, 0u16, 0u16, 0u16, 0u16, 0u16, 0u16, 0u16, 0u16, 0u16, 0u16, 0u16, 0u16,
            0x2603u16, 0xD83Du16, 0x00B6u16,
        ];
        assert_eq!(utf16_valid_up_to(& lone_high[..]), 14);
        let lone_low = vec![
            0u16, 0u16, 0u16, 0u16, 0u16, 0u16, 0u16, 0u16, 0u16, 0u16, 0u16, 0u16, 0u16,
            0x2603u16, 0xDCA9u16, 0x00B6u16,
        ];
        assert_eq!(utf16_valid_up_to(& lone_low[..]), 14);
        let lone_high_at_end = vec![
            0u16, 0u16, 0u16, 0u16, 0u16, 0u16, 0u16, 0u16, 0u16, 0u16, 0u16, 0u16, 0u16,
            0x2603u16, 0x00B6u16, 0xD83Du16,
        ];
        assert_eq!(utf16_valid_up_to(& lone_high_at_end[..]), 15);
    }
    #[test]
    fn test_ensure_utf16_validity() {
        let mut src = vec![
            0u16, 0xD83Du16, 0u16, 0u16, 0u16, 0xD83Du16, 0xDCA9u16, 0u16, 0u16, 0u16,
            0u16, 0u16, 0u16, 0xDCA9u16, 0u16, 0u16, 0u16, 0u16, 0u16, 0u16, 0u16, 0u16,
            0u16, 0u16, 0u16, 0u16, 0u16, 0u16, 0u16, 0u16, 0u16,
        ];
        let reference = vec![
            0u16, 0xFFFDu16, 0u16, 0u16, 0u16, 0xD83Du16, 0xDCA9u16, 0u16, 0u16, 0u16,
            0u16, 0u16, 0u16, 0xFFFDu16, 0u16, 0u16, 0u16, 0u16, 0u16, 0u16, 0u16, 0u16,
            0u16, 0u16, 0u16, 0u16, 0u16, 0u16, 0u16, 0u16, 0u16,
        ];
        ensure_utf16_validity(&mut src[..]);
        assert_eq!(src, reference);
    }
    #[test]
    fn test_is_char_bidi() {
        assert!(! is_char_bidi('a'));
        assert!(! is_char_bidi('\u{03B1}'));
        assert!(! is_char_bidi('\u{3041}'));
        assert!(! is_char_bidi('\u{1F4A9}'));
        assert!(! is_char_bidi('\u{FE00}'));
        assert!(! is_char_bidi('\u{202C}'));
        assert!(! is_char_bidi('\u{FEFF}'));
        assert!(is_char_bidi('\u{0590}'));
        assert!(is_char_bidi('\u{08FF}'));
        assert!(is_char_bidi('\u{061C}'));
        assert!(is_char_bidi('\u{FB50}'));
        assert!(is_char_bidi('\u{FDFF}'));
        assert!(is_char_bidi('\u{FE70}'));
        assert!(is_char_bidi('\u{FEFE}'));
        assert!(is_char_bidi('\u{200F}'));
        assert!(is_char_bidi('\u{202B}'));
        assert!(is_char_bidi('\u{202E}'));
        assert!(is_char_bidi('\u{2067}'));
        assert!(is_char_bidi('\u{10800}'));
        assert!(is_char_bidi('\u{10FFF}'));
        assert!(is_char_bidi('\u{1E800}'));
        assert!(is_char_bidi('\u{1EFFF}'));
    }
    #[test]
    fn test_is_utf16_code_unit_bidi() {
        assert!(! is_utf16_code_unit_bidi(0x0062));
        assert!(! is_utf16_code_unit_bidi(0x03B1));
        assert!(! is_utf16_code_unit_bidi(0x3041));
        assert!(! is_utf16_code_unit_bidi(0xD801));
        assert!(! is_utf16_code_unit_bidi(0xFE00));
        assert!(! is_utf16_code_unit_bidi(0x202C));
        assert!(! is_utf16_code_unit_bidi(0xFEFF));
        assert!(is_utf16_code_unit_bidi(0x0590));
        assert!(is_utf16_code_unit_bidi(0x08FF));
        assert!(is_utf16_code_unit_bidi(0x061C));
        assert!(is_utf16_code_unit_bidi(0xFB1D));
        assert!(is_utf16_code_unit_bidi(0xFB50));
        assert!(is_utf16_code_unit_bidi(0xFDFF));
        assert!(is_utf16_code_unit_bidi(0xFE70));
        assert!(is_utf16_code_unit_bidi(0xFEFE));
        assert!(is_utf16_code_unit_bidi(0x200F));
        assert!(is_utf16_code_unit_bidi(0x202B));
        assert!(is_utf16_code_unit_bidi(0x202E));
        assert!(is_utf16_code_unit_bidi(0x2067));
        assert!(is_utf16_code_unit_bidi(0xD802));
        assert!(is_utf16_code_unit_bidi(0xD803));
        assert!(is_utf16_code_unit_bidi(0xD83A));
        assert!(is_utf16_code_unit_bidi(0xD83B));
    }
    #[test]
    fn test_is_str_bidi() {
        assert!(! is_str_bidi("abcdefghijklmnopaabcdefghijklmnop"));
        assert!(! is_str_bidi("abcdefghijklmnop\u{03B1}abcdefghijklmnop"));
        assert!(! is_str_bidi("abcdefghijklmnop\u{3041}abcdefghijklmnop"));
        assert!(! is_str_bidi("abcdefghijklmnop\u{1F4A9}abcdefghijklmnop"));
        assert!(! is_str_bidi("abcdefghijklmnop\u{FE00}abcdefghijklmnop"));
        assert!(! is_str_bidi("abcdefghijklmnop\u{202C}abcdefghijklmnop"));
        assert!(! is_str_bidi("abcdefghijklmnop\u{FEFF}abcdefghijklmnop"));
        assert!(is_str_bidi("abcdefghijklmnop\u{0590}abcdefghijklmnop"));
        assert!(is_str_bidi("abcdefghijklmnop\u{08FF}abcdefghijklmnop"));
        assert!(is_str_bidi("abcdefghijklmnop\u{061C}abcdefghijklmnop"));
        assert!(is_str_bidi("abcdefghijklmnop\u{FB50}abcdefghijklmnop"));
        assert!(is_str_bidi("abcdefghijklmnop\u{FDFF}abcdefghijklmnop"));
        assert!(is_str_bidi("abcdefghijklmnop\u{FE70}abcdefghijklmnop"));
        assert!(is_str_bidi("abcdefghijklmnop\u{FEFE}abcdefghijklmnop"));
        assert!(is_str_bidi("abcdefghijklmnop\u{200F}abcdefghijklmnop"));
        assert!(is_str_bidi("abcdefghijklmnop\u{202B}abcdefghijklmnop"));
        assert!(is_str_bidi("abcdefghijklmnop\u{202E}abcdefghijklmnop"));
        assert!(is_str_bidi("abcdefghijklmnop\u{2067}abcdefghijklmnop"));
        assert!(is_str_bidi("abcdefghijklmnop\u{10800}abcdefghijklmnop"));
        assert!(is_str_bidi("abcdefghijklmnop\u{10FFF}abcdefghijklmnop"));
        assert!(is_str_bidi("abcdefghijklmnop\u{1E800}abcdefghijklmnop"));
        assert!(is_str_bidi("abcdefghijklmnop\u{1EFFF}abcdefghijklmnop"));
    }
    #[test]
    fn test_is_utf8_bidi() {
        assert!(! is_utf8_bidi("abcdefghijklmnopaabcdefghijklmnop".as_bytes()));
        assert!(! is_utf8_bidi("abcdefghijklmnop\u{03B1}abcdefghijklmnop".as_bytes()));
        assert!(! is_utf8_bidi("abcdefghijklmnop\u{3041}abcdefghijklmnop".as_bytes()));
        assert!(! is_utf8_bidi("abcdefghijklmnop\u{1F4A9}abcdefghijklmnop".as_bytes()));
        assert!(! is_utf8_bidi("abcdefghijklmnop\u{FE00}abcdefghijklmnop".as_bytes()));
        assert!(! is_utf8_bidi("abcdefghijklmnop\u{202C}abcdefghijklmnop".as_bytes()));
        assert!(! is_utf8_bidi("abcdefghijklmnop\u{FEFF}abcdefghijklmnop".as_bytes()));
        assert!(is_utf8_bidi("abcdefghijklmnop\u{0590}abcdefghijklmnop".as_bytes()));
        assert!(is_utf8_bidi("abcdefghijklmnop\u{08FF}abcdefghijklmnop".as_bytes()));
        assert!(is_utf8_bidi("abcdefghijklmnop\u{061C}abcdefghijklmnop".as_bytes()));
        assert!(is_utf8_bidi("abcdefghijklmnop\u{FB50}abcdefghijklmnop".as_bytes()));
        assert!(is_utf8_bidi("abcdefghijklmnop\u{FDFF}abcdefghijklmnop".as_bytes()));
        assert!(is_utf8_bidi("abcdefghijklmnop\u{FE70}abcdefghijklmnop".as_bytes()));
        assert!(is_utf8_bidi("abcdefghijklmnop\u{FEFE}abcdefghijklmnop".as_bytes()));
        assert!(is_utf8_bidi("abcdefghijklmnop\u{200F}abcdefghijklmnop".as_bytes()));
        assert!(is_utf8_bidi("abcdefghijklmnop\u{202B}abcdefghijklmnop".as_bytes()));
        assert!(is_utf8_bidi("abcdefghijklmnop\u{202E}abcdefghijklmnop".as_bytes()));
        assert!(is_utf8_bidi("abcdefghijklmnop\u{2067}abcdefghijklmnop".as_bytes()));
        assert!(is_utf8_bidi("abcdefghijklmnop\u{10800}abcdefghijklmnop".as_bytes()));
        assert!(is_utf8_bidi("abcdefghijklmnop\u{10FFF}abcdefghijklmnop".as_bytes()));
        assert!(is_utf8_bidi("abcdefghijklmnop\u{1E800}abcdefghijklmnop".as_bytes()));
        assert!(is_utf8_bidi("abcdefghijklmnop\u{1EFFF}abcdefghijklmnop".as_bytes()));
    }
    #[test]
    fn test_is_utf16_bidi() {
        assert!(
            ! is_utf16_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x0062,
            0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,])
        );
        assert!(
            ! is_utf16_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x03B1,
            0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,])
        );
        assert!(
            ! is_utf16_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x3041,
            0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,])
        );
        assert!(
            ! is_utf16_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0xD801,
            0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,])
        );
        assert!(
            ! is_utf16_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0xFE00,
            0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,])
        );
        assert!(
            ! is_utf16_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x202C,
            0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,])
        );
        assert!(
            ! is_utf16_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0xFEFF,
            0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,])
        );
        assert!(
            is_utf16_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x0590,
            0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,])
        );
        assert!(
            is_utf16_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x08FF,
            0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,])
        );
        assert!(
            is_utf16_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x061C,
            0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,])
        );
        assert!(
            is_utf16_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0xFB1D,
            0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,])
        );
        assert!(
            is_utf16_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0xFB50,
            0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,])
        );
        assert!(
            is_utf16_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0xFDFF,
            0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,])
        );
        assert!(
            is_utf16_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0xFE70,
            0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,])
        );
        assert!(
            is_utf16_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0xFEFE,
            0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,])
        );
        assert!(
            is_utf16_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x200F,
            0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,])
        );
        assert!(
            is_utf16_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x202B,
            0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,])
        );
        assert!(
            is_utf16_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x202E,
            0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,])
        );
        assert!(
            is_utf16_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x2067,
            0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,])
        );
        assert!(
            is_utf16_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0xD802,
            0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,])
        );
        assert!(
            is_utf16_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0xD803,
            0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,])
        );
        assert!(
            is_utf16_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0xD83A,
            0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,])
        );
        assert!(
            is_utf16_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0xD83B,
            0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,])
        );
        assert!(
            is_utf16_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x0590,
            0x3041, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,])
        );
    }
    #[test]
    fn test_check_str_for_latin1_and_bidi() {
        assert_ne!(
            check_str_for_latin1_and_bidi("abcdefghijklmnopaabcdefghijklmnop"),
            Latin1Bidi::Bidi
        );
        assert_ne!(
            check_str_for_latin1_and_bidi("abcdefghijklmnop\u{03B1}abcdefghijklmnop"),
            Latin1Bidi::Bidi
        );
        assert_ne!(
            check_str_for_latin1_and_bidi("abcdefghijklmnop\u{3041}abcdefghijklmnop"),
            Latin1Bidi::Bidi
        );
        assert_ne!(
            check_str_for_latin1_and_bidi("abcdefghijklmnop\u{1F4A9}abcdefghijklmnop"),
            Latin1Bidi::Bidi
        );
        assert_ne!(
            check_str_for_latin1_and_bidi("abcdefghijklmnop\u{FE00}abcdefghijklmnop"),
            Latin1Bidi::Bidi
        );
        assert_ne!(
            check_str_for_latin1_and_bidi("abcdefghijklmnop\u{202C}abcdefghijklmnop"),
            Latin1Bidi::Bidi
        );
        assert_ne!(
            check_str_for_latin1_and_bidi("abcdefghijklmnop\u{FEFF}abcdefghijklmnop"),
            Latin1Bidi::Bidi
        );
        assert_eq!(
            check_str_for_latin1_and_bidi("abcdefghijklmnop\u{0590}abcdefghijklmnop"),
            Latin1Bidi::Bidi
        );
        assert_eq!(
            check_str_for_latin1_and_bidi("abcdefghijklmnop\u{08FF}abcdefghijklmnop"),
            Latin1Bidi::Bidi
        );
        assert_eq!(
            check_str_for_latin1_and_bidi("abcdefghijklmnop\u{061C}abcdefghijklmnop"),
            Latin1Bidi::Bidi
        );
        assert_eq!(
            check_str_for_latin1_and_bidi("abcdefghijklmnop\u{FB50}abcdefghijklmnop"),
            Latin1Bidi::Bidi
        );
        assert_eq!(
            check_str_for_latin1_and_bidi("abcdefghijklmnop\u{FDFF}abcdefghijklmnop"),
            Latin1Bidi::Bidi
        );
        assert_eq!(
            check_str_for_latin1_and_bidi("abcdefghijklmnop\u{FE70}abcdefghijklmnop"),
            Latin1Bidi::Bidi
        );
        assert_eq!(
            check_str_for_latin1_and_bidi("abcdefghijklmnop\u{FEFE}abcdefghijklmnop"),
            Latin1Bidi::Bidi
        );
        assert_eq!(
            check_str_for_latin1_and_bidi("abcdefghijklmnop\u{200F}abcdefghijklmnop"),
            Latin1Bidi::Bidi
        );
        assert_eq!(
            check_str_for_latin1_and_bidi("abcdefghijklmnop\u{202B}abcdefghijklmnop"),
            Latin1Bidi::Bidi
        );
        assert_eq!(
            check_str_for_latin1_and_bidi("abcdefghijklmnop\u{202E}abcdefghijklmnop"),
            Latin1Bidi::Bidi
        );
        assert_eq!(
            check_str_for_latin1_and_bidi("abcdefghijklmnop\u{2067}abcdefghijklmnop"),
            Latin1Bidi::Bidi
        );
        assert_eq!(
            check_str_for_latin1_and_bidi("abcdefghijklmnop\u{10800}abcdefghijklmnop"),
            Latin1Bidi::Bidi
        );
        assert_eq!(
            check_str_for_latin1_and_bidi("abcdefghijklmnop\u{10FFF}abcdefghijklmnop"),
            Latin1Bidi::Bidi
        );
        assert_eq!(
            check_str_for_latin1_and_bidi("abcdefghijklmnop\u{1E800}abcdefghijklmnop"),
            Latin1Bidi::Bidi
        );
        assert_eq!(
            check_str_for_latin1_and_bidi("abcdefghijklmnop\u{1EFFF}abcdefghijklmnop"),
            Latin1Bidi::Bidi
        );
    }
    #[test]
    fn test_check_utf8_for_latin1_and_bidi() {
        assert_ne!(
            check_utf8_for_latin1_and_bidi("abcdefghijklmnopaabcdefghijklmnop"
            .as_bytes()), Latin1Bidi::Bidi
        );
        assert_ne!(
            check_utf8_for_latin1_and_bidi("abcdefghijklmnop\u{03B1}abcdefghijklmnop"
            .as_bytes()), Latin1Bidi::Bidi
        );
        assert_ne!(
            check_utf8_for_latin1_and_bidi("abcdefghijklmnop\u{3041}abcdefghijklmnop"
            .as_bytes()), Latin1Bidi::Bidi
        );
        assert_ne!(
            check_utf8_for_latin1_and_bidi("abcdefghijklmnop\u{1F4A9}abcdefghijklmnop"
            .as_bytes()), Latin1Bidi::Bidi
        );
        assert_ne!(
            check_utf8_for_latin1_and_bidi("abcdefghijklmnop\u{FE00}abcdefghijklmnop"
            .as_bytes()), Latin1Bidi::Bidi
        );
        assert_ne!(
            check_utf8_for_latin1_and_bidi("abcdefghijklmnop\u{202C}abcdefghijklmnop"
            .as_bytes()), Latin1Bidi::Bidi
        );
        assert_ne!(
            check_utf8_for_latin1_and_bidi("abcdefghijklmnop\u{FEFF}abcdefghijklmnop"
            .as_bytes()), Latin1Bidi::Bidi
        );
        assert_eq!(
            check_utf8_for_latin1_and_bidi("abcdefghijklmnop\u{0590}abcdefghijklmnop"
            .as_bytes()), Latin1Bidi::Bidi
        );
        assert_eq!(
            check_utf8_for_latin1_and_bidi("abcdefghijklmnop\u{08FF}abcdefghijklmnop"
            .as_bytes()), Latin1Bidi::Bidi
        );
        assert_eq!(
            check_utf8_for_latin1_and_bidi("abcdefghijklmnop\u{061C}abcdefghijklmnop"
            .as_bytes()), Latin1Bidi::Bidi
        );
        assert_eq!(
            check_utf8_for_latin1_and_bidi("abcdefghijklmnop\u{FB50}abcdefghijklmnop"
            .as_bytes()), Latin1Bidi::Bidi
        );
        assert_eq!(
            check_utf8_for_latin1_and_bidi("abcdefghijklmnop\u{FDFF}abcdefghijklmnop"
            .as_bytes()), Latin1Bidi::Bidi
        );
        assert_eq!(
            check_utf8_for_latin1_and_bidi("abcdefghijklmnop\u{FE70}abcdefghijklmnop"
            .as_bytes()), Latin1Bidi::Bidi
        );
        assert_eq!(
            check_utf8_for_latin1_and_bidi("abcdefghijklmnop\u{FEFE}abcdefghijklmnop"
            .as_bytes()), Latin1Bidi::Bidi
        );
        assert_eq!(
            check_utf8_for_latin1_and_bidi("abcdefghijklmnop\u{200F}abcdefghijklmnop"
            .as_bytes()), Latin1Bidi::Bidi
        );
        assert_eq!(
            check_utf8_for_latin1_and_bidi("abcdefghijklmnop\u{202B}abcdefghijklmnop"
            .as_bytes()), Latin1Bidi::Bidi
        );
        assert_eq!(
            check_utf8_for_latin1_and_bidi("abcdefghijklmnop\u{202E}abcdefghijklmnop"
            .as_bytes()), Latin1Bidi::Bidi
        );
        assert_eq!(
            check_utf8_for_latin1_and_bidi("abcdefghijklmnop\u{2067}abcdefghijklmnop"
            .as_bytes()), Latin1Bidi::Bidi
        );
        assert_eq!(
            check_utf8_for_latin1_and_bidi("abcdefghijklmnop\u{10800}abcdefghijklmnop"
            .as_bytes()), Latin1Bidi::Bidi
        );
        assert_eq!(
            check_utf8_for_latin1_and_bidi("abcdefghijklmnop\u{10FFF}abcdefghijklmnop"
            .as_bytes()), Latin1Bidi::Bidi
        );
        assert_eq!(
            check_utf8_for_latin1_and_bidi("abcdefghijklmnop\u{1E800}abcdefghijklmnop"
            .as_bytes()), Latin1Bidi::Bidi
        );
        assert_eq!(
            check_utf8_for_latin1_and_bidi("abcdefghijklmnop\u{1EFFF}abcdefghijklmnop"
            .as_bytes()), Latin1Bidi::Bidi
        );
    }
    #[test]
    fn test_check_utf16_for_latin1_and_bidi() {
        assert_ne!(
            check_utf16_for_latin1_and_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68,
            0x69, 0x0062, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,]),
            Latin1Bidi::Bidi
        );
        assert_ne!(
            check_utf16_for_latin1_and_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68,
            0x69, 0x03B1, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,]),
            Latin1Bidi::Bidi
        );
        assert_ne!(
            check_utf16_for_latin1_and_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68,
            0x69, 0x3041, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,]),
            Latin1Bidi::Bidi
        );
        assert_ne!(
            check_utf16_for_latin1_and_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68,
            0x69, 0xD801, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,]),
            Latin1Bidi::Bidi
        );
        assert_ne!(
            check_utf16_for_latin1_and_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68,
            0x69, 0xFE00, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,]),
            Latin1Bidi::Bidi
        );
        assert_ne!(
            check_utf16_for_latin1_and_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68,
            0x69, 0x202C, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,]),
            Latin1Bidi::Bidi
        );
        assert_ne!(
            check_utf16_for_latin1_and_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68,
            0x69, 0xFEFF, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,]),
            Latin1Bidi::Bidi
        );
        assert_eq!(
            check_utf16_for_latin1_and_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68,
            0x69, 0x0590, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,]),
            Latin1Bidi::Bidi
        );
        assert_eq!(
            check_utf16_for_latin1_and_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68,
            0x69, 0x08FF, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,]),
            Latin1Bidi::Bidi
        );
        assert_eq!(
            check_utf16_for_latin1_and_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68,
            0x69, 0x061C, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,]),
            Latin1Bidi::Bidi
        );
        assert_eq!(
            check_utf16_for_latin1_and_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68,
            0x69, 0xFB1D, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,]),
            Latin1Bidi::Bidi
        );
        assert_eq!(
            check_utf16_for_latin1_and_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68,
            0x69, 0xFB50, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,]),
            Latin1Bidi::Bidi
        );
        assert_eq!(
            check_utf16_for_latin1_and_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68,
            0x69, 0xFDFF, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,]),
            Latin1Bidi::Bidi
        );
        assert_eq!(
            check_utf16_for_latin1_and_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68,
            0x69, 0xFE70, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,]),
            Latin1Bidi::Bidi
        );
        assert_eq!(
            check_utf16_for_latin1_and_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68,
            0x69, 0xFEFE, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,]),
            Latin1Bidi::Bidi
        );
        assert_eq!(
            check_utf16_for_latin1_and_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68,
            0x69, 0x200F, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,]),
            Latin1Bidi::Bidi
        );
        assert_eq!(
            check_utf16_for_latin1_and_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68,
            0x69, 0x202B, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,]),
            Latin1Bidi::Bidi
        );
        assert_eq!(
            check_utf16_for_latin1_and_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68,
            0x69, 0x202E, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,]),
            Latin1Bidi::Bidi
        );
        assert_eq!(
            check_utf16_for_latin1_and_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68,
            0x69, 0x2067, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,]),
            Latin1Bidi::Bidi
        );
        assert_eq!(
            check_utf16_for_latin1_and_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68,
            0x69, 0xD802, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,]),
            Latin1Bidi::Bidi
        );
        assert_eq!(
            check_utf16_for_latin1_and_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68,
            0x69, 0xD803, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,]),
            Latin1Bidi::Bidi
        );
        assert_eq!(
            check_utf16_for_latin1_and_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68,
            0x69, 0xD83A, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,]),
            Latin1Bidi::Bidi
        );
        assert_eq!(
            check_utf16_for_latin1_and_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68,
            0x69, 0xD83B, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,]),
            Latin1Bidi::Bidi
        );
        assert_eq!(
            check_utf16_for_latin1_and_bidi(& [0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68,
            0x69, 0x0590, 0x3041, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,]),
            Latin1Bidi::Bidi
        );
    }
    #[inline(always)]
    pub fn reference_is_char_bidi(c: char) -> bool {
        match c {
            '\u{0590}'..='\u{08FF}'
            | '\u{FB1D}'..='\u{FDFF}'
            | '\u{FE70}'..='\u{FEFE}'
            | '\u{10800}'..='\u{10FFF}'
            | '\u{1E800}'..='\u{1EFFF}'
            | '\u{200F}'
            | '\u{202B}'
            | '\u{202E}'
            | '\u{2067}' => true,
            _ => false,
        }
    }
    #[inline(always)]
    pub fn reference_is_utf16_code_unit_bidi(u: u16) -> bool {
        match u {
            0x0590..=0x08FF
            | 0xFB1D..=0xFDFF
            | 0xFE70..=0xFEFE
            | 0xD802
            | 0xD803
            | 0xD83A
            | 0xD83B
            | 0x200F
            | 0x202B
            | 0x202E
            | 0x2067 => true,
            _ => false,
        }
    }
    #[test]
    fn test_is_char_bidi_thoroughly() {
        for i in 0..0xD800u32 {
            let c: char = ::std::char::from_u32(i).unwrap();
            assert_eq!(is_char_bidi(c), reference_is_char_bidi(c));
        }
        for i in 0xE000..0x110000u32 {
            let c: char = ::std::char::from_u32(i).unwrap();
            assert_eq!(is_char_bidi(c), reference_is_char_bidi(c));
        }
    }
    #[test]
    fn test_is_utf16_code_unit_bidi_thoroughly() {
        for i in 0..0x10000u32 {
            let u = i as u16;
            assert_eq!(is_utf16_code_unit_bidi(u), reference_is_utf16_code_unit_bidi(u));
        }
    }
    #[test]
    fn test_is_str_bidi_thoroughly() {
        let mut buf = [0; 4];
        for i in 0..0xD800u32 {
            let c: char = ::std::char::from_u32(i).unwrap();
            assert_eq!(
                is_str_bidi(c.encode_utf8(& mut buf[..])), reference_is_char_bidi(c)
            );
        }
        for i in 0xE000..0x110000u32 {
            let c: char = ::std::char::from_u32(i).unwrap();
            assert_eq!(
                is_str_bidi(c.encode_utf8(& mut buf[..])), reference_is_char_bidi(c)
            );
        }
    }
    #[test]
    fn test_is_utf8_bidi_thoroughly() {
        let mut buf = [0; 8];
        for i in 0..0xD800u32 {
            let c: char = ::std::char::from_u32(i).unwrap();
            let expect = reference_is_char_bidi(c);
            {
                let len = {
                    let bytes = c.encode_utf8(&mut buf[..]).as_bytes();
                    assert_eq!(is_utf8_bidi(bytes), expect);
                    bytes.len()
                };
                {
                    let tail = &mut buf[len..];
                    for b in tail.iter_mut() {
                        *b = 0;
                    }
                }
            }
            assert_eq!(is_utf8_bidi(& buf[..]), expect);
        }
        for i in 0xE000..0x110000u32 {
            let c: char = ::std::char::from_u32(i).unwrap();
            let expect = reference_is_char_bidi(c);
            {
                let len = {
                    let bytes = c.encode_utf8(&mut buf[..]).as_bytes();
                    assert_eq!(is_utf8_bidi(bytes), expect);
                    bytes.len()
                };
                {
                    let tail = &mut buf[len..];
                    for b in tail.iter_mut() {
                        *b = 0;
                    }
                }
            }
            assert_eq!(is_utf8_bidi(& buf[..]), expect);
        }
    }
    #[test]
    fn test_is_utf16_bidi_thoroughly() {
        let mut buf = [0; 32];
        for i in 0..0x10000u32 {
            let u = i as u16;
            buf[15] = u;
            assert_eq!(is_utf16_bidi(& buf[..]), reference_is_utf16_code_unit_bidi(u));
        }
    }
    #[test]
    fn test_is_utf8_bidi_edge_cases() {
        assert!(! is_utf8_bidi(b"\xD5\xBF\x61"));
        assert!(! is_utf8_bidi(b"\xD6\x80\x61"));
        assert!(! is_utf8_bidi(b"abc"));
        assert!(is_utf8_bidi(b"\xD5\xBF\xC2"));
        assert!(is_utf8_bidi(b"\xD6\x80\xC2"));
        assert!(is_utf8_bidi(b"ab\xC2"));
    }
    #[test]
    fn test_decode_latin1() {
        match decode_latin1(b"ab") {
            Cow::Borrowed(s) => {
                assert_eq!(s, "ab");
            }
            Cow::Owned(_) => {
                unreachable!("Should have borrowed");
            }
        }
        assert_eq!(decode_latin1(b"a\xE4"), "a\u{E4}");
    }
    #[test]
    fn test_encode_latin1_lossy() {
        match encode_latin1_lossy("ab") {
            Cow::Borrowed(s) => {
                assert_eq!(s, b"ab");
            }
            Cow::Owned(_) => {
                unreachable!("Should have borrowed");
            }
        }
        assert_eq!(encode_latin1_lossy("a\u{E4}"), & (b"a\xE4") [..]);
    }
    #[test]
    fn test_convert_utf8_to_utf16_without_replacement() {
        let mut buf = [0u16; 5];
        assert_eq!(
            convert_utf8_to_utf16_without_replacement(b"ab", & mut buf[..2]), Some(2)
        );
        assert_eq!(buf[0], u16::from(b'a'));
        assert_eq!(buf[1], u16::from(b'b'));
        assert_eq!(buf[2], 0);
        assert_eq!(
            convert_utf8_to_utf16_without_replacement(b"\xC3\xA4c", & mut buf[..3]),
            Some(2)
        );
        assert_eq!(buf[0], 0xE4);
        assert_eq!(buf[1], u16::from(b'c'));
        assert_eq!(buf[2], 0);
        assert_eq!(
            convert_utf8_to_utf16_without_replacement(b"\xE2\x98\x83", & mut buf[..3]),
            Some(1)
        );
        assert_eq!(buf[0], 0x2603);
        assert_eq!(buf[1], u16::from(b'c'));
        assert_eq!(buf[2], 0);
        assert_eq!(
            convert_utf8_to_utf16_without_replacement(b"\xE2\x98\x83d", & mut buf[..4]),
            Some(2)
        );
        assert_eq!(buf[0], 0x2603);
        assert_eq!(buf[1], u16::from(b'd'));
        assert_eq!(buf[2], 0);
        assert_eq!(
            convert_utf8_to_utf16_without_replacement(b"\xE2\x98\x83\xC3\xA4", & mut
            buf[..5]), Some(2)
        );
        assert_eq!(buf[0], 0x2603);
        assert_eq!(buf[1], 0xE4);
        assert_eq!(buf[2], 0);
        assert_eq!(
            convert_utf8_to_utf16_without_replacement(b"\xF0\x9F\x93\x8E", & mut
            buf[..4]), Some(2)
        );
        assert_eq!(buf[0], 0xD83D);
        assert_eq!(buf[1], 0xDCCE);
        assert_eq!(buf[2], 0);
        assert_eq!(
            convert_utf8_to_utf16_without_replacement(b"\xF0\x9F\x93\x8Ee", & mut
            buf[..5]), Some(3)
        );
        assert_eq!(buf[0], 0xD83D);
        assert_eq!(buf[1], 0xDCCE);
        assert_eq!(buf[2], u16::from(b'e'));
        assert_eq!(
            convert_utf8_to_utf16_without_replacement(b"\xF0\x9F\x93", & mut buf[..5]),
            None
        );
    }
}
#[cfg(test)]
mod tests_rug_291 {
    use super::*;
    use crate::mem;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: bool = rug_fuzz_0;
        unsafe {
            mem::likely(p0);
        }
             }
});    }
}
#[cfg(test)]
mod tests_rug_292 {
    use super::*;
    #[test]
    fn test_unlikely() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: bool = rug_fuzz_0;
        unsafe {
            let result = crate::mem::unlikely(p0);
            debug_assert_eq!(result, true);
        }
             }
});    }
}
#[cfg(test)]
mod tests_rug_293 {
    use super::*;
    use crate::mem;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_293_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"Hello, world!";
        let p0: &[u8] = rug_fuzz_0;
        mem::is_ascii_impl(p0);
        let _rug_ed_tests_rug_293_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_294 {
    use super::*;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4)) = <(u16, u16, u16, u16, u16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: [u16; 5] = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3, rug_fuzz_4];
        crate::mem::is_basic_latin_impl(&p0);
             }
});    }
}
#[cfg(test)]
mod tests_rug_295 {
    use super::*;
    use crate::mem::is_utf16_latin1_impl;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4)) = <(u16, u16, u16, u16, u16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: [u16; 5] = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3, rug_fuzz_4];
        is_utf16_latin1_impl(&p0);
             }
});    }
}
#[cfg(test)]
mod tests_rug_296 {
    use super::*;
    use crate::mem::{utf16_valid_up_to_impl, utf16_valid_up_to_alu};
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5)) = <(u16, u16, u16, u16, u16, u16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let buffer: &[u16] = &[
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
        ];
        utf16_valid_up_to_impl(buffer);
             }
});    }
}
#[cfg(test)]
mod tests_rug_297 {
    use super::*;
    #[test]
    fn test_utf16_valid_up_to_alu() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4)) = <(u16, u16, u16, u16, u16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let buffer: [u16; 5] = [
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
        ];
        let p0: &[u16] = &buffer;
        crate::mem::utf16_valid_up_to_alu(p0);
             }
});    }
}
#[cfg(test)]
mod tests_rug_298 {
    use super::*;
    #[test]
    fn test_is_str_latin1_impl() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0 = rug_fuzz_0;
        crate::mem::is_str_latin1_impl(&p0);
             }
});    }
}
#[cfg(test)]
mod tests_rug_299 {
    use super::*;
    use crate::mem::{is_utf8_latin1_impl, validate_ascii, in_inclusive_range8};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_299_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"\xC2\xA9\xC2\xA9";
        let p0: &[u8] = rug_fuzz_0;
        let result = is_utf8_latin1_impl(p0);
        debug_assert_eq!(result, Some(4));
        let _rug_ed_tests_rug_299_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_300 {
    use super::*;
    #[test]
    fn test_is_utf16_bidi_impl() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(u16, u16, u16, u16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: &[u16] = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3];
        debug_assert_eq!(crate ::mem::is_utf16_bidi_impl(p0), true);
             }
});    }
}
#[cfg(test)]
mod tests_rug_301 {
    use super::*;
    use crate::mem::{
        check_utf16_for_latin1_and_bidi_impl, Latin1Bidi, ALU_ALIGNMENT,
        ALU_ALIGNMENT_MASK, LATIN1_MASK, is_utf16_bidi_impl, is_utf16_code_unit_bidi,
    };
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5)) = <(u16, u16, u16, u16, u16, u16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: [u16; 6] = [
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
        ];
        let _ = check_utf16_for_latin1_and_bidi_impl(&p0);
             }
});    }
}
#[cfg(test)]
mod tests_rug_302 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_302_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"Hello, World!";
        let p0: &[u8] = rug_fuzz_0;
        crate::mem::is_ascii(p0);
        let _rug_ed_tests_rug_302_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_303 {
    use super::*;
    use crate::mem::is_basic_latin;
    #[test]
    fn test_is_basic_latin() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(u16, u16, u16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: [u16; 3] = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        debug_assert_eq!(is_basic_latin(& p0), true);
             }
});    }
}
#[cfg(test)]
mod tests_rug_304 {
    use super::*;
    #[test]
    fn test_is_utf8_latin1() {
        let _rug_st_tests_rug_304_rrrruuuugggg_test_is_utf8_latin1 = 0;
        let rug_fuzz_0 = b"Hello, world!";
        let p0: &[u8] = rug_fuzz_0;
        debug_assert_eq!(crate ::mem::is_utf8_latin1(p0), true);
        let _rug_ed_tests_rug_304_rrrruuuugggg_test_is_utf8_latin1 = 0;
    }
}
#[cfg(test)]
mod tests_rug_305 {
    use super::*;
    use crate::mem::is_str_latin1;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0 = rug_fuzz_0;
        debug_assert_eq!(is_str_latin1(& p0), true);
             }
});    }
}
#[cfg(test)]
mod tests_rug_306 {
    use super::*;
    #[test]
    fn test_is_utf16_latin1() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(u16, u16, u16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: &[u16] = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        crate::mem::is_utf16_latin1(p0);
             }
});    }
}
#[cfg(test)]
mod tests_rug_307 {
    use super::*;
    use crate::mem::{UTF8_DATA, in_inclusive_range8, validate_ascii};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_307_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"\x68\x65\x6C\x6C\x6F";
        let mut p0 = rug_fuzz_0;
        crate::mem::is_utf8_bidi(p0);
        let _rug_ed_tests_rug_307_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_308 {
    use super::*;
    #[test]
    fn test_is_str_bidi() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0 = rug_fuzz_0;
        crate::mem::is_str_bidi(&p0);
             }
});    }
}
#[cfg(test)]
mod tests_rug_309 {
    use super::*;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(u16, u16, u16, u16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: [u16; 4] = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3];
        debug_assert_eq!(crate ::mem::is_utf16_bidi(& p0), true);
             }
});    }
}
#[cfg(test)]
mod tests_rug_310 {
    use super::*;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(char) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: char = rug_fuzz_0;
        debug_assert_eq!(crate ::mem::is_char_bidi(p0), false);
             }
});    }
}
#[cfg(test)]
mod tests_rug_311 {
    use super::*;
    #[test]
    fn test_is_utf16_code_unit_bidi() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(u16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: u16 = rug_fuzz_0;
        debug_assert!(crate ::mem::is_utf16_code_unit_bidi(p0));
             }
});    }
}
#[cfg(test)]
mod tests_rug_313 {
    use super::*;
    use crate::{mem::check_str_for_latin1_and_bidi, mem::Latin1Bidi};
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0 = rug_fuzz_0;
        debug_assert_eq!(check_str_for_latin1_and_bidi(& p0), Latin1Bidi::LeftToRight);
             }
});    }
}
#[cfg(test)]
mod tests_rug_314 {
    use super::*;
    use crate::mem::Latin1Bidi;
    #[test]
    fn test_check_utf16_for_latin1_and_bidi() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4)) = <(u16, u16, u16, u16, u16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: [u16; 5] = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3, rug_fuzz_4];
        crate::mem::check_utf16_for_latin1_and_bidi(&p0);
             }
});    }
}
#[cfg(test)]
mod tests_rug_315 {
    use super::*;
    use crate::mem;
    #[test]
    fn test_convert_utf8_to_utf16() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1)) = <([u8; 5], u16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        let p0 = rug_fuzz_0.as_ref();
        let mut p1 = [rug_fuzz_1; 10];
        mem::convert_utf8_to_utf16(p0, &mut p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_316 {
    use super::*;
    use crate::mem::convert_str_to_utf16;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0 = rug_fuzz_0;
        let mut p1 = vec![0; p0.len()];
        convert_str_to_utf16(p0, &mut p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_317 {
    use super::*;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1)) = <([u8; 5], u16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        let p0: &[u8] = rug_fuzz_0;
        let mut p1: [u16; 5] = [rug_fuzz_1; 5];
        crate::mem::convert_utf8_to_utf16_without_replacement(p0, &mut p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_318 {
    use super::*;
    use crate::mem::convert_utf16_to_utf8_partial;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4)) = <(u16, u16, u16, u16, u16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: &[u16] = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3, rug_fuzz_4];
        let p1: &mut [u8] = &mut vec![0; 10];
        convert_utf16_to_utf8_partial(p0, p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_319 {
    use super::*;
    use crate::mem;
    #[test]
    fn test_convert_utf16_to_utf8() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(u16, u16, u16, u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: [u16; 3] = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        let mut p1: [u8; 9] = [rug_fuzz_3; 9];
        let result = mem::convert_utf16_to_utf8(&p0, &mut p1);
        debug_assert_eq!(result, 6);
             }
});    }
}
#[cfg(test)]
mod tests_rug_320 {
    use super::*;
    use crate::mem::convert_utf16_to_str_partial;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5, mut rug_fuzz_6)) = <(u16, u16, u16, u16, u16, u16, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: &[u16] = &[
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
        ];
        let mut p1: &mut str = &mut String::with_capacity(rug_fuzz_6);
        convert_utf16_to_str_partial(p0, p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_321 {
    use super::*;
    use crate::mem::convert_utf16_to_str;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5, mut rug_fuzz_6)) = <(u16, u16, u16, u16, u16, u16, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let src_data: [u16; 6] = [
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
        ];
        let src_slice = &src_data;
        let mut dst_buffer = String::with_capacity(src_slice.len() * rug_fuzz_6);
        convert_utf16_to_str(src_slice, &mut dst_buffer);
             }
});    }
}
#[cfg(test)]
mod tests_rug_322 {
    use super::*;
    use crate::mem;
    #[test]
    fn test_convert_latin1_to_utf16() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5, mut rug_fuzz_6)) = <(u8, u8, u8, u8, u8, u8, u16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: &[u8] = &[
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
        ];
        let mut p1: [u16; 6] = [rug_fuzz_6; 6];
        mem::convert_latin1_to_utf16(p0, &mut p1);
        debug_assert_eq!(p1, [0xFEFF, 0x0041, 0x0042, 0x00DF, 0x0000, 0x0000]);
             }
});    }
}
#[cfg(test)]
mod tests_rug_323 {
    use super::*;
    use crate::mem;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1)) = <([u8; 2], u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        let p0: &[u8] = rug_fuzz_0;
        let p1: &mut [u8] = &mut [rug_fuzz_1; 10];
        crate::mem::convert_latin1_to_utf8_partial(p0, p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_324 {
    use super::*;
    use crate::mem::convert_latin1_to_utf8;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1)) = <([u8; 5], u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        let p0: &[u8] = rug_fuzz_0;
        let mut p1: [u8; 10] = [rug_fuzz_1; 10];
        crate::mem::convert_latin1_to_utf8(p0, &mut p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_325 {
    use super::*;
    use crate::mem::convert_latin1_to_str_partial;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1)) = <([u8; 5], &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        let p0 = rug_fuzz_0;
        let mut p1 = rug_fuzz_1.to_string();
        convert_latin1_to_str_partial(p0, &mut p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_326 {
    use super::*;
    use crate::mem::convert_latin1_to_str;
    #[test]
    fn test_encoding_rs() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1)) = <([u8; 13], usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        let p0 = rug_fuzz_0;
        let mut p1 = String::with_capacity(p0.len() * rug_fuzz_1);
        convert_latin1_to_str(p0, &mut p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_327 {
    use super::*;
    use crate::mem::convert_utf8_to_latin1_lossy;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1)) = <([u8; 13], u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        let p0: &[u8] = rug_fuzz_0;
        let mut p1: [u8; 20] = [rug_fuzz_1; 20];
        convert_utf8_to_latin1_lossy(p0, &mut p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_328 {
    use super::*;
    use crate::mem::convert_utf16_to_latin1_lossy;
    #[test]
    fn test_convert_utf16_to_latin1_lossy() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(u16, u16, u16, u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: &[u16] = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        let mut p1: &mut [u8] = &mut [rug_fuzz_3; 3];
        convert_utf16_to_latin1_lossy(p0, p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_329 {
    use super::*;
    use std::borrow::Cow;
    #[test]
    fn test_decode_latin1() {
        let _rug_st_tests_rug_329_rrrruuuugggg_test_decode_latin1 = 0;
        let rug_fuzz_0 = b"Hello, World!";
        let bytes: &[u8] = rug_fuzz_0;
        crate::mem::decode_latin1(bytes);
        let _rug_ed_tests_rug_329_rrrruuuugggg_test_decode_latin1 = 0;
    }
}
#[cfg(test)]
mod tests_rug_330 {
    use super::*;
    use std::borrow::Cow;
    #[test]
    fn test_encode_latin1_lossy() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: &str = rug_fuzz_0;
        crate::mem::encode_latin1_lossy(&p0);
             }
});    }
}
#[cfg(test)]
mod tests_rug_331 {
    use super::*;
    use crate::mem;
    #[test]
    fn test_utf16_valid_up_to() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4)) = <(u16, u16, u16, u16, u16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0 = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3, rug_fuzz_4];
        let result = mem::utf16_valid_up_to(&p0);
        debug_assert_eq!(result, 1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_332 {
    use super::*;
    use crate::mem::utf8_latin1_up_to;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_332_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"\xF0\x90\x80\x80Hello, World!";
        let p0: &[u8] = rug_fuzz_0;
        debug_assert_eq!(utf8_latin1_up_to(p0), 0);
        let _rug_ed_tests_rug_332_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_333 {
    use super::*;
    #[test]
    fn test_str_latin1_up_to() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0 = rug_fuzz_0;
        crate::mem::str_latin1_up_to(&p0);
             }
});    }
}
#[cfg(test)]
mod tests_rug_334 {
    use super::*;
    use crate::mem::ensure_utf16_validity;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(u16, u16, u16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: &mut [u16] = &mut [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        ensure_utf16_validity(p0);
        debug_assert_eq!(& p0, & [0xDC00, 0xFFFD, 0x0020]);
             }
});    }
}
#[cfg(test)]
mod tests_rug_335 {
    use super::*;
    #[test]
    fn test_encoding_rs() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1)) = <([u8; 13], u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        let p0: &[u8] = rug_fuzz_0;
        let p1: &mut [u8] = &mut [rug_fuzz_1; 13];
        crate::mem::copy_ascii_to_ascii(p0, p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_336 {
    use super::*;
    use crate::mem;
    #[test]
    fn test_encoding_rs() {
        let _rug_st_tests_rug_336_rrrruuuugggg_test_encoding_rs = 0;
        let rug_fuzz_0 = b"Hello, world!";
        let p0: &[u8] = rug_fuzz_0;
        let mut p1: Vec<u16> = vec![0; p0.len()];
        mem::copy_ascii_to_basic_latin(p0, &mut p1);
        let _rug_ed_tests_rug_336_rrrruuuugggg_test_encoding_rs = 0;
    }
}
#[cfg(test)]
mod tests_rug_337 {
    use super::*;
    use crate::mem::copy_basic_latin_to_ascii;
    #[test]
    fn test_encoding_rs() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(u16, u16, u16, u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: &[u16] = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        let mut p1: &mut [u8] = &mut [rug_fuzz_3; 3];
        let result = copy_basic_latin_to_ascii(p0, p1);
        debug_assert_eq!(result, 3);
             }
});    }
}
