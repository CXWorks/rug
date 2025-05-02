#[cfg(
    all(
        feature = "simd-accel",
        any(
            target_feature = "sse2",
            all(target_endian = "little", target_arch = "aarch64"),
            all(target_endian = "little", target_feature = "neon")
        )
    )
)]
use simd_funcs::*;
cfg_if! {
    if #[cfg(feature = "simd-accel")] { #[allow(unused_imports)] use
    ::std::intrinsics::unlikely; #[allow(unused_imports)] use ::std::intrinsics::likely;
    } else { #[allow(dead_code)] #[inline(always)] unsafe fn unlikely(b : bool) -> bool {
    b } #[allow(dead_code)] #[inline(always)] unsafe fn likely(b : bool) -> bool { b } }
}
#[allow(dead_code)]
pub const ASCII_MASK: usize = 0x8080_8080_8080_8080u64 as usize;
#[allow(dead_code)]
pub const BASIC_LATIN_MASK: usize = 0xFF80_FF80_FF80_FF80u64 as usize;
#[allow(unused_macros)]
macro_rules! ascii_naive {
    ($name:ident, $src_unit:ty, $dst_unit:ty) => {
        #[inline(always)] pub unsafe fn $name (src : * const $src_unit, dst : * mut
        $dst_unit, len : usize,) -> Option < ($src_unit, usize) > { for i in 0..len { let
        code_unit = * (src.add(i)); if code_unit > 127 { return Some((code_unit, i)); } *
        (dst.add(i)) = code_unit as $dst_unit; } return None; }
    };
}
#[allow(unused_macros)]
macro_rules! ascii_alu {
    ($name:ident, $src_unit:ty, $dst_unit:ty, $stride_fn:ident) => {
        #[cfg_attr(feature = "cargo-clippy", allow(never_loop, cast_ptr_alignment))]
        #[inline(always)] pub unsafe fn $name (src : * const $src_unit, dst : * mut
        $dst_unit, len : usize,) -> Option < ($src_unit, usize) > { let mut offset =
        0usize; loop { let mut until_alignment = { let src_alignment = (src as usize) &
        ALU_ALIGNMENT_MASK; let dst_alignment = (dst as usize) & ALU_ALIGNMENT_MASK; if
        src_alignment != dst_alignment { break; } (ALU_ALIGNMENT - src_alignment) &
        ALU_ALIGNMENT_MASK }; if until_alignment + ALU_STRIDE_SIZE <= len { while
        until_alignment != 0 { let code_unit = * (src.add(offset)); if code_unit > 127 {
        return Some((code_unit, offset)); } * (dst.add(offset)) = code_unit as $dst_unit;
        offset += 1; until_alignment -= 1; } let len_minus_stride = len -
        ALU_STRIDE_SIZE; loop { if let Some(num_ascii) = $stride_fn (src.add(offset) as *
        const usize, dst.add(offset) as * mut usize,) { offset += num_ascii; return
        Some((* (src.add(offset)), offset)); } offset += ALU_STRIDE_SIZE; if offset >
        len_minus_stride { break; } } } break; } while offset < len { let code_unit = *
        (src.add(offset)); if code_unit > 127 { return Some((code_unit, offset)); } *
        (dst.add(offset)) = code_unit as $dst_unit; offset += 1; } None }
    };
}
#[allow(unused_macros)]
macro_rules! basic_latin_alu {
    ($name:ident, $src_unit:ty, $dst_unit:ty, $stride_fn:ident) => {
        #[cfg_attr(feature = "cargo-clippy", allow(never_loop, cast_ptr_alignment,
        cast_lossless))] #[inline(always)] pub unsafe fn $name (src : * const $src_unit,
        dst : * mut $dst_unit, len : usize,) -> Option < ($src_unit, usize) > { let mut
        offset = 0usize; loop { let mut until_alignment = { if
        ::std::mem::size_of::<$src_unit > () < ::std::mem::size_of::<$dst_unit > () { let
        src_until_alignment = (ALU_ALIGNMENT - ((src as usize) & ALU_ALIGNMENT_MASK)) &
        ALU_ALIGNMENT_MASK; if (dst.add(src_until_alignment) as usize) &
        ALU_ALIGNMENT_MASK != 0 { break; } src_until_alignment } else { let
        dst_until_alignment = (ALU_ALIGNMENT - ((dst as usize) & ALU_ALIGNMENT_MASK)) &
        ALU_ALIGNMENT_MASK; if (src.add(dst_until_alignment) as usize) &
        ALU_ALIGNMENT_MASK != 0 { break; } dst_until_alignment } }; if until_alignment +
        ALU_STRIDE_SIZE <= len { while until_alignment != 0 { let code_unit = * (src
        .add(offset)); if code_unit > 127 { return Some((code_unit, offset)); } * (dst
        .add(offset)) = code_unit as $dst_unit; offset += 1; until_alignment -= 1; } let
        len_minus_stride = len - ALU_STRIDE_SIZE; loop { if !$stride_fn (src.add(offset)
        as * const usize, dst.add(offset) as * mut usize,) { break; } offset +=
        ALU_STRIDE_SIZE; if offset > len_minus_stride { break; } } } break; } while
        offset < len { let code_unit = * (src.add(offset)); if code_unit > 127 { return
        Some((code_unit, offset)); } * (dst.add(offset)) = code_unit as $dst_unit; offset
        += 1; } None }
    };
}
#[allow(unused_macros)]
macro_rules! latin1_alu {
    ($name:ident, $src_unit:ty, $dst_unit:ty, $stride_fn:ident) => {
        #[cfg_attr(feature = "cargo-clippy", allow(never_loop, cast_ptr_alignment,
        cast_lossless))] #[inline(always)] pub unsafe fn $name (src : * const $src_unit,
        dst : * mut $dst_unit, len : usize) { let mut offset = 0usize; loop { let mut
        until_alignment = { if ::std::mem::size_of::<$src_unit > () <
        ::std::mem::size_of::<$dst_unit > () { let src_until_alignment = (ALU_ALIGNMENT -
        ((src as usize) & ALU_ALIGNMENT_MASK)) & ALU_ALIGNMENT_MASK; if (dst
        .add(src_until_alignment) as usize) & ALU_ALIGNMENT_MASK != 0 { break; }
        src_until_alignment } else { let dst_until_alignment = (ALU_ALIGNMENT - ((dst as
        usize) & ALU_ALIGNMENT_MASK)) & ALU_ALIGNMENT_MASK; if (src
        .add(dst_until_alignment) as usize) & ALU_ALIGNMENT_MASK != 0 { break; }
        dst_until_alignment } }; if until_alignment + ALU_STRIDE_SIZE <= len { while
        until_alignment != 0 { let code_unit = * (src.add(offset)); * (dst.add(offset)) =
        code_unit as $dst_unit; offset += 1; until_alignment -= 1; } let len_minus_stride
        = len - ALU_STRIDE_SIZE; loop { $stride_fn (src.add(offset) as * const usize, dst
        .add(offset) as * mut usize,); offset += ALU_STRIDE_SIZE; if offset >
        len_minus_stride { break; } } } break; } while offset < len { let code_unit = *
        (src.add(offset)); * (dst.add(offset)) = code_unit as $dst_unit; offset += 1; } }
    };
}
#[allow(unused_macros)]
macro_rules! ascii_simd_check_align {
    (
        $name:ident, $src_unit:ty, $dst_unit:ty, $stride_both_aligned:ident,
        $stride_src_aligned:ident, $stride_dst_aligned:ident,
        $stride_neither_aligned:ident
    ) => {
        #[inline(always)] pub unsafe fn $name (src : * const $src_unit, dst : * mut
        $dst_unit, len : usize,) -> Option < ($src_unit, usize) > { let mut offset =
        0usize; if SIMD_STRIDE_SIZE <= len { let len_minus_stride = len -
        SIMD_STRIDE_SIZE; let dst_masked = (dst as usize) & SIMD_ALIGNMENT_MASK; if ((src
        as usize) & SIMD_ALIGNMENT_MASK) == 0 { if dst_masked == 0 { loop { if
        !$stride_both_aligned (src.add(offset), dst.add(offset)) { break; } offset +=
        SIMD_STRIDE_SIZE; if offset > len_minus_stride { break; } } } else { loop { if
        !$stride_src_aligned (src.add(offset), dst.add(offset)) { break; } offset +=
        SIMD_STRIDE_SIZE; if offset > len_minus_stride { break; } } } } else { if
        dst_masked == 0 { loop { if !$stride_dst_aligned (src.add(offset), dst
        .add(offset)) { break; } offset += SIMD_STRIDE_SIZE; if offset > len_minus_stride
        { break; } } } else { loop { if !$stride_neither_aligned (src.add(offset), dst
        .add(offset)) { break; } offset += SIMD_STRIDE_SIZE; if offset > len_minus_stride
        { break; } } } } } while offset < len { let code_unit = * (src.add(offset)); if
        code_unit > 127 { return Some((code_unit, offset)); } * (dst.add(offset)) =
        code_unit as $dst_unit; offset += 1; } None }
    };
}
#[allow(unused_macros)]
macro_rules! ascii_simd_check_align_unrolled {
    (
        $name:ident, $src_unit:ty, $dst_unit:ty, $stride_both_aligned:ident,
        $stride_src_aligned:ident, $stride_neither_aligned:ident,
        $double_stride_both_aligned:ident, $double_stride_src_aligned:ident
    ) => {
        #[inline(always)] pub unsafe fn $name (src : * const $src_unit, dst : * mut
        $dst_unit, len : usize,) -> Option < ($src_unit, usize) > { let unit_size =
        ::std::mem::size_of::<$src_unit > (); let mut offset = 0usize; 'outer : loop { if
        SIMD_STRIDE_SIZE <= len { if !$stride_neither_aligned (src, dst) { break 'outer;
        } offset = SIMD_STRIDE_SIZE; let until_alignment = ((SIMD_ALIGNMENT - ((src
        .add(offset) as usize) & SIMD_ALIGNMENT_MASK)) & SIMD_ALIGNMENT_MASK) /
        unit_size; if until_alignment + (SIMD_STRIDE_SIZE * 3) <= len { if
        until_alignment != 0 { if !$stride_neither_aligned (src.add(offset), dst
        .add(offset)) { break; } offset += until_alignment; } let
        len_minus_stride_times_two = len - (SIMD_STRIDE_SIZE * 2); let dst_masked = (dst
        .add(offset) as usize) & SIMD_ALIGNMENT_MASK; if dst_masked == 0 { loop { if let
        Some(advance) = $double_stride_both_aligned (src.add(offset), dst.add(offset)) {
        offset += advance; let code_unit = * (src.add(offset)); return Some((code_unit,
        offset)); } offset += SIMD_STRIDE_SIZE * 2; if offset >
        len_minus_stride_times_two { break; } } if offset + SIMD_STRIDE_SIZE <= len { if
        !$stride_both_aligned (src.add(offset), dst.add(offset)) { break 'outer; } offset
        += SIMD_STRIDE_SIZE; } } else { loop { if let Some(advance) =
        $double_stride_src_aligned (src.add(offset), dst.add(offset)) { offset +=
        advance; let code_unit = * (src.add(offset)); return Some((code_unit, offset)); }
        offset += SIMD_STRIDE_SIZE * 2; if offset > len_minus_stride_times_two { break; }
        } if offset + SIMD_STRIDE_SIZE <= len { if !$stride_src_aligned (src.add(offset),
        dst.add(offset)) { break 'outer; } offset += SIMD_STRIDE_SIZE; } } } else { if
        offset + SIMD_STRIDE_SIZE <= len { if !$stride_neither_aligned (src.add(offset),
        dst.add(offset)) { break; } offset += SIMD_STRIDE_SIZE; if offset +
        SIMD_STRIDE_SIZE <= len { if !$stride_neither_aligned (src.add(offset), dst
        .add(offset)) { break; } offset += SIMD_STRIDE_SIZE; } } } } break 'outer; }
        while offset < len { let code_unit = * (src.add(offset)); if code_unit > 127 {
        return Some((code_unit, offset)); } * (dst.add(offset)) = code_unit as $dst_unit;
        offset += 1; } None }
    };
}
#[allow(unused_macros)]
macro_rules! latin1_simd_check_align {
    (
        $name:ident, $src_unit:ty, $dst_unit:ty, $stride_both_aligned:ident,
        $stride_src_aligned:ident, $stride_dst_aligned:ident,
        $stride_neither_aligned:ident
    ) => {
        #[inline(always)] pub unsafe fn $name (src : * const $src_unit, dst : * mut
        $dst_unit, len : usize) { let mut offset = 0usize; if SIMD_STRIDE_SIZE <= len {
        let len_minus_stride = len - SIMD_STRIDE_SIZE; let dst_masked = (dst as usize) &
        SIMD_ALIGNMENT_MASK; if ((src as usize) & SIMD_ALIGNMENT_MASK) == 0 { if
        dst_masked == 0 { loop { $stride_both_aligned (src.add(offset), dst.add(offset));
        offset += SIMD_STRIDE_SIZE; if offset > len_minus_stride { break; } } } else {
        loop { $stride_src_aligned (src.add(offset), dst.add(offset)); offset +=
        SIMD_STRIDE_SIZE; if offset > len_minus_stride { break; } } } } else { if
        dst_masked == 0 { loop { $stride_dst_aligned (src.add(offset), dst.add(offset));
        offset += SIMD_STRIDE_SIZE; if offset > len_minus_stride { break; } } } else {
        loop { $stride_neither_aligned (src.add(offset), dst.add(offset)); offset +=
        SIMD_STRIDE_SIZE; if offset > len_minus_stride { break; } } } } } while offset <
        len { let code_unit = * (src.add(offset)); * (dst.add(offset)) = code_unit as
        $dst_unit; offset += 1; } }
    };
}
#[allow(unused_macros)]
macro_rules! latin1_simd_check_align_unrolled {
    (
        $name:ident, $src_unit:ty, $dst_unit:ty, $stride_both_aligned:ident,
        $stride_src_aligned:ident, $stride_dst_aligned:ident,
        $stride_neither_aligned:ident
    ) => {
        #[inline(always)] pub unsafe fn $name (src : * const $src_unit, dst : * mut
        $dst_unit, len : usize) { let unit_size = ::std::mem::size_of::<$src_unit > ();
        let mut offset = 0usize; if SIMD_STRIDE_SIZE <= len { let mut until_alignment =
        ((SIMD_STRIDE_SIZE - ((src as usize) & SIMD_ALIGNMENT_MASK)) &
        SIMD_ALIGNMENT_MASK) / unit_size; while until_alignment != 0 { * (dst
        .add(offset)) = * (src.add(offset)) as $dst_unit; offset += 1; until_alignment -=
        1; } let len_minus_stride = len - SIMD_STRIDE_SIZE; if offset + SIMD_STRIDE_SIZE
        * 2 <= len { let len_minus_stride_times_two = len_minus_stride -
        SIMD_STRIDE_SIZE; if (dst.add(offset) as usize) & SIMD_ALIGNMENT_MASK == 0 { loop
        { $stride_both_aligned (src.add(offset), dst.add(offset)); offset +=
        SIMD_STRIDE_SIZE; $stride_both_aligned (src.add(offset), dst.add(offset)); offset
        += SIMD_STRIDE_SIZE; if offset > len_minus_stride_times_two { break; } } } else {
        loop { $stride_src_aligned (src.add(offset), dst.add(offset)); offset +=
        SIMD_STRIDE_SIZE; $stride_src_aligned (src.add(offset), dst.add(offset)); offset
        += SIMD_STRIDE_SIZE; if offset > len_minus_stride_times_two { break; } } } } if
        offset < len_minus_stride { $stride_src_aligned (src.add(offset), dst
        .add(offset)); offset += SIMD_STRIDE_SIZE; } } while offset < len { let code_unit
        = * (src.add(offset)); * (dst.add(offset)) = code_unit as $dst_unit; offset += 1;
        } }
    };
}
#[allow(unused_macros)]
macro_rules! ascii_simd_unalign {
    ($name:ident, $src_unit:ty, $dst_unit:ty, $stride_neither_aligned:ident) => {
        #[inline(always)] pub unsafe fn $name (src : * const $src_unit, dst : * mut
        $dst_unit, len : usize,) -> Option < ($src_unit, usize) > { let mut offset =
        0usize; if SIMD_STRIDE_SIZE <= len { let len_minus_stride = len -
        SIMD_STRIDE_SIZE; loop { if !$stride_neither_aligned (src.add(offset), dst
        .add(offset)) { break; } offset += SIMD_STRIDE_SIZE; if offset > len_minus_stride
        { break; } } } while offset < len { let code_unit = * (src.add(offset)); if
        code_unit > 127 { return Some((code_unit, offset)); } * (dst.add(offset)) =
        code_unit as $dst_unit; offset += 1; } None }
    };
}
#[allow(unused_macros)]
macro_rules! latin1_simd_unalign {
    ($name:ident, $src_unit:ty, $dst_unit:ty, $stride_neither_aligned:ident) => {
        #[inline(always)] pub unsafe fn $name (src : * const $src_unit, dst : * mut
        $dst_unit, len : usize) { let mut offset = 0usize; if SIMD_STRIDE_SIZE <= len {
        let len_minus_stride = len - SIMD_STRIDE_SIZE; loop { $stride_neither_aligned
        (src.add(offset), dst.add(offset)); offset += SIMD_STRIDE_SIZE; if offset >
        len_minus_stride { break; } } } while offset < len { let code_unit = * (src
        .add(offset)); * (dst.add(offset)) = code_unit as $dst_unit; offset += 1; } }
    };
}
#[allow(unused_macros)]
macro_rules! ascii_to_ascii_simd_stride {
    ($name:ident, $load:ident, $store:ident) => {
        #[inline(always)] pub unsafe fn $name (src : * const u8, dst : * mut u8) -> bool
        { let simd = $load (src); if ! simd_is_ascii(simd) { return false; } $store (dst,
        simd); true }
    };
}
#[allow(unused_macros)]
macro_rules! ascii_to_ascii_simd_double_stride {
    ($name:ident, $store:ident) => {
        #[inline(always)] pub unsafe fn $name (src : * const u8, dst : * mut u8) ->
        Option < usize > { let first = load16_aligned(src); let second =
        load16_aligned(src.add(SIMD_STRIDE_SIZE)); $store (dst, first); if unlikely(!
        simd_is_ascii(first | second)) { let mask_first = mask_ascii(first); if
        mask_first != 0 { return Some(mask_first.trailing_zeros() as usize); } $store
        (dst.add(SIMD_STRIDE_SIZE), second); let mask_second = mask_ascii(second); return
        Some(SIMD_STRIDE_SIZE + mask_second.trailing_zeros() as usize); } $store (dst
        .add(SIMD_STRIDE_SIZE), second); None }
    };
}
#[allow(unused_macros)]
macro_rules! ascii_to_basic_latin_simd_stride {
    ($name:ident, $load:ident, $store:ident) => {
        #[inline(always)] pub unsafe fn $name (src : * const u8, dst : * mut u16) -> bool
        { let simd = $load (src); if ! simd_is_ascii(simd) { return false; } let (first,
        second) = simd_unpack(simd); $store (dst, first); $store (dst.add(8), second);
        true }
    };
}
#[allow(unused_macros)]
macro_rules! ascii_to_basic_latin_simd_double_stride {
    ($name:ident, $store:ident) => {
        #[inline(always)] pub unsafe fn $name (src : * const u8, dst : * mut u16) ->
        Option < usize > { let first = load16_aligned(src); let second =
        load16_aligned(src.add(SIMD_STRIDE_SIZE)); let (a, b) = simd_unpack(first);
        $store (dst, a); $store (dst.add(SIMD_STRIDE_SIZE / 2), b); if unlikely(!
        simd_is_ascii(first | second)) { let mask_first = mask_ascii(first); if
        mask_first != 0 { return Some(mask_first.trailing_zeros() as usize); } let (c, d)
        = simd_unpack(second); $store (dst.add(SIMD_STRIDE_SIZE), c); $store (dst
        .add(SIMD_STRIDE_SIZE + (SIMD_STRIDE_SIZE / 2)), d); let mask_second =
        mask_ascii(second); return Some(SIMD_STRIDE_SIZE + mask_second.trailing_zeros()
        as usize); } let (c, d) = simd_unpack(second); $store (dst.add(SIMD_STRIDE_SIZE),
        c); $store (dst.add(SIMD_STRIDE_SIZE + (SIMD_STRIDE_SIZE / 2)), d); None }
    };
}
#[allow(unused_macros)]
macro_rules! unpack_simd_stride {
    ($name:ident, $load:ident, $store:ident) => {
        #[inline(always)] pub unsafe fn $name (src : * const u8, dst : * mut u16) { let
        simd = $load (src); let (first, second) = simd_unpack(simd); $store (dst, first);
        $store (dst.add(8), second); }
    };
}
#[allow(unused_macros)]
macro_rules! basic_latin_to_ascii_simd_stride {
    ($name:ident, $load:ident, $store:ident) => {
        #[inline(always)] pub unsafe fn $name (src : * const u16, dst : * mut u8) -> bool
        { let first = $load (src); let second = $load (src.add(8)); if
        simd_is_basic_latin(first | second) { $store (dst, simd_pack(first, second));
        true } else { false } }
    };
}
#[allow(unused_macros)]
macro_rules! pack_simd_stride {
    ($name:ident, $load:ident, $store:ident) => {
        #[inline(always)] pub unsafe fn $name (src : * const u16, dst : * mut u8) { let
        first = $load (src); let second = $load (src.add(8)); $store (dst,
        simd_pack(first, second)); }
    };
}
cfg_if! {
    if #[cfg(all(feature = "simd-accel", target_endian = "little", target_arch =
    "aarch64"))] { pub const SIMD_STRIDE_SIZE : usize = 16; pub const MAX_STRIDE_SIZE :
    usize = 16; pub const ALU_STRIDE_SIZE : usize = 16; pub const ALU_ALIGNMENT : usize =
    8; pub const ALU_ALIGNMENT_MASK : usize = 7;
    ascii_to_ascii_simd_stride!(ascii_to_ascii_stride_neither_aligned, load16_unaligned,
    store16_unaligned);
    ascii_to_basic_latin_simd_stride!(ascii_to_basic_latin_stride_neither_aligned,
    load16_unaligned, store8_unaligned);
    unpack_simd_stride!(unpack_stride_neither_aligned, load16_unaligned,
    store8_unaligned);
    basic_latin_to_ascii_simd_stride!(basic_latin_to_ascii_stride_neither_aligned,
    load8_unaligned, store16_unaligned); pack_simd_stride!(pack_stride_neither_aligned,
    load8_unaligned, store16_unaligned); ascii_simd_unalign!(ascii_to_ascii, u8, u8,
    ascii_to_ascii_stride_neither_aligned); ascii_simd_unalign!(ascii_to_basic_latin, u8,
    u16, ascii_to_basic_latin_stride_neither_aligned);
    ascii_simd_unalign!(basic_latin_to_ascii, u16, u8,
    basic_latin_to_ascii_stride_neither_aligned); latin1_simd_unalign!(unpack_latin1, u8,
    u16, unpack_stride_neither_aligned); latin1_simd_unalign!(pack_latin1, u16, u8,
    pack_stride_neither_aligned); } else if #[cfg(all(feature = "simd-accel",
    target_endian = "little", target_feature = "neon"))] { pub const SIMD_STRIDE_SIZE :
    usize = 16; pub const MAX_STRIDE_SIZE : usize = 16; pub const SIMD_ALIGNMENT_MASK :
    usize = 15; ascii_to_ascii_simd_stride!(ascii_to_ascii_stride_both_aligned,
    load16_aligned, store16_aligned);
    ascii_to_ascii_simd_stride!(ascii_to_ascii_stride_src_aligned, load16_aligned,
    store16_unaligned); ascii_to_ascii_simd_stride!(ascii_to_ascii_stride_dst_aligned,
    load16_unaligned, store16_aligned);
    ascii_to_ascii_simd_stride!(ascii_to_ascii_stride_neither_aligned, load16_unaligned,
    store16_unaligned);
    ascii_to_basic_latin_simd_stride!(ascii_to_basic_latin_stride_both_aligned,
    load16_aligned, store8_aligned);
    ascii_to_basic_latin_simd_stride!(ascii_to_basic_latin_stride_src_aligned,
    load16_aligned, store8_unaligned);
    ascii_to_basic_latin_simd_stride!(ascii_to_basic_latin_stride_dst_aligned,
    load16_unaligned, store8_aligned);
    ascii_to_basic_latin_simd_stride!(ascii_to_basic_latin_stride_neither_aligned,
    load16_unaligned, store8_unaligned); unpack_simd_stride!(unpack_stride_both_aligned,
    load16_aligned, store8_aligned); unpack_simd_stride!(unpack_stride_src_aligned,
    load16_aligned, store8_unaligned); unpack_simd_stride!(unpack_stride_dst_aligned,
    load16_unaligned, store8_aligned); unpack_simd_stride!(unpack_stride_neither_aligned,
    load16_unaligned, store8_unaligned);
    basic_latin_to_ascii_simd_stride!(basic_latin_to_ascii_stride_both_aligned,
    load8_aligned, store16_aligned);
    basic_latin_to_ascii_simd_stride!(basic_latin_to_ascii_stride_src_aligned,
    load8_aligned, store16_unaligned);
    basic_latin_to_ascii_simd_stride!(basic_latin_to_ascii_stride_dst_aligned,
    load8_unaligned, store16_aligned);
    basic_latin_to_ascii_simd_stride!(basic_latin_to_ascii_stride_neither_aligned,
    load8_unaligned, store16_unaligned); pack_simd_stride!(pack_stride_both_aligned,
    load8_aligned, store16_aligned); pack_simd_stride!(pack_stride_src_aligned,
    load8_aligned, store16_unaligned); pack_simd_stride!(pack_stride_dst_aligned,
    load8_unaligned, store16_aligned); pack_simd_stride!(pack_stride_neither_aligned,
    load8_unaligned, store16_unaligned); ascii_simd_check_align!(ascii_to_ascii, u8, u8,
    ascii_to_ascii_stride_both_aligned, ascii_to_ascii_stride_src_aligned,
    ascii_to_ascii_stride_dst_aligned, ascii_to_ascii_stride_neither_aligned);
    ascii_simd_check_align!(ascii_to_basic_latin, u8, u16,
    ascii_to_basic_latin_stride_both_aligned, ascii_to_basic_latin_stride_src_aligned,
    ascii_to_basic_latin_stride_dst_aligned,
    ascii_to_basic_latin_stride_neither_aligned);
    ascii_simd_check_align!(basic_latin_to_ascii, u16, u8,
    basic_latin_to_ascii_stride_both_aligned, basic_latin_to_ascii_stride_src_aligned,
    basic_latin_to_ascii_stride_dst_aligned,
    basic_latin_to_ascii_stride_neither_aligned); latin1_simd_check_align!(unpack_latin1,
    u8, u16, unpack_stride_both_aligned, unpack_stride_src_aligned,
    unpack_stride_dst_aligned, unpack_stride_neither_aligned);
    latin1_simd_check_align!(pack_latin1, u16, u8, pack_stride_both_aligned,
    pack_stride_src_aligned, pack_stride_dst_aligned, pack_stride_neither_aligned); }
    else if #[cfg(all(feature = "simd-accel", target_feature = "sse2"))] { pub const
    SIMD_STRIDE_SIZE : usize = 16; pub const SIMD_ALIGNMENT : usize = 16; pub const
    MAX_STRIDE_SIZE : usize = 16; pub const SIMD_ALIGNMENT_MASK : usize = 15;
    ascii_to_ascii_simd_double_stride!(ascii_to_ascii_simd_double_stride_both_aligned,
    store16_aligned);
    ascii_to_ascii_simd_double_stride!(ascii_to_ascii_simd_double_stride_src_aligned,
    store16_unaligned);
    ascii_to_basic_latin_simd_double_stride!(ascii_to_basic_latin_simd_double_stride_both_aligned,
    store8_aligned);
    ascii_to_basic_latin_simd_double_stride!(ascii_to_basic_latin_simd_double_stride_src_aligned,
    store8_unaligned); ascii_to_ascii_simd_stride!(ascii_to_ascii_stride_both_aligned,
    load16_aligned, store16_aligned);
    ascii_to_ascii_simd_stride!(ascii_to_ascii_stride_src_aligned, load16_aligned,
    store16_unaligned);
    ascii_to_ascii_simd_stride!(ascii_to_ascii_stride_neither_aligned, load16_unaligned,
    store16_unaligned);
    ascii_to_basic_latin_simd_stride!(ascii_to_basic_latin_stride_both_aligned,
    load16_aligned, store8_aligned);
    ascii_to_basic_latin_simd_stride!(ascii_to_basic_latin_stride_src_aligned,
    load16_aligned, store8_unaligned);
    ascii_to_basic_latin_simd_stride!(ascii_to_basic_latin_stride_neither_aligned,
    load16_unaligned, store8_unaligned); unpack_simd_stride!(unpack_stride_both_aligned,
    load16_aligned, store8_aligned); unpack_simd_stride!(unpack_stride_src_aligned,
    load16_aligned, store8_unaligned);
    basic_latin_to_ascii_simd_stride!(basic_latin_to_ascii_stride_both_aligned,
    load8_aligned, store16_aligned);
    basic_latin_to_ascii_simd_stride!(basic_latin_to_ascii_stride_src_aligned,
    load8_aligned, store16_unaligned);
    basic_latin_to_ascii_simd_stride!(basic_latin_to_ascii_stride_dst_aligned,
    load8_unaligned, store16_aligned);
    basic_latin_to_ascii_simd_stride!(basic_latin_to_ascii_stride_neither_aligned,
    load8_unaligned, store16_unaligned); pack_simd_stride!(pack_stride_both_aligned,
    load8_aligned, store16_aligned); pack_simd_stride!(pack_stride_src_aligned,
    load8_aligned, store16_unaligned); ascii_simd_check_align_unrolled!(ascii_to_ascii,
    u8, u8, ascii_to_ascii_stride_both_aligned, ascii_to_ascii_stride_src_aligned,
    ascii_to_ascii_stride_neither_aligned,
    ascii_to_ascii_simd_double_stride_both_aligned,
    ascii_to_ascii_simd_double_stride_src_aligned);
    ascii_simd_check_align_unrolled!(ascii_to_basic_latin, u8, u16,
    ascii_to_basic_latin_stride_both_aligned, ascii_to_basic_latin_stride_src_aligned,
    ascii_to_basic_latin_stride_neither_aligned,
    ascii_to_basic_latin_simd_double_stride_both_aligned,
    ascii_to_basic_latin_simd_double_stride_src_aligned);
    ascii_simd_check_align!(basic_latin_to_ascii, u16, u8,
    basic_latin_to_ascii_stride_both_aligned, basic_latin_to_ascii_stride_src_aligned,
    basic_latin_to_ascii_stride_dst_aligned,
    basic_latin_to_ascii_stride_neither_aligned);
    latin1_simd_check_align_unrolled!(unpack_latin1, u8, u16, unpack_stride_both_aligned,
    unpack_stride_src_aligned, unpack_stride_dst_aligned, unpack_stride_neither_aligned);
    latin1_simd_check_align_unrolled!(pack_latin1, u16, u8, pack_stride_both_aligned,
    pack_stride_src_aligned, pack_stride_dst_aligned, pack_stride_neither_aligned); }
    else if #[cfg(all(target_endian = "little", target_pointer_width = "64"))] { pub
    const ALU_STRIDE_SIZE : usize = 16; pub const MAX_STRIDE_SIZE : usize = 16; pub const
    ALU_ALIGNMENT : usize = 8; pub const ALU_ALIGNMENT_MASK : usize = 7;
    #[inline(always)] unsafe fn unpack_alu(word : usize, second_word : usize, dst : * mut
    usize) { let first = ((0x0000_0000_FF00_0000usize & word) << 24) |
    ((0x0000_0000_00FF_0000usize & word) << 16) | ((0x0000_0000_0000_FF00usize & word) <<
    8) | (0x0000_0000_0000_00FFusize & word); let second = ((0xFF00_0000_0000_0000usize &
    word) >> 8) | ((0x00FF_0000_0000_0000usize & word) >> 16) |
    ((0x0000_FF00_0000_0000usize & word) >> 24) | ((0x0000_00FF_0000_0000usize & word) >>
    32); let third = ((0x0000_0000_FF00_0000usize & second_word) << 24) |
    ((0x0000_0000_00FF_0000usize & second_word) << 16) | ((0x0000_0000_0000_FF00usize &
    second_word) << 8) | (0x0000_0000_0000_00FFusize & second_word); let fourth =
    ((0xFF00_0000_0000_0000usize & second_word) >> 8) | ((0x00FF_0000_0000_0000usize &
    second_word) >> 16) | ((0x0000_FF00_0000_0000usize & second_word) >> 24) |
    ((0x0000_00FF_0000_0000usize & second_word) >> 32); * dst = first; * (dst.add(1)) =
    second; * (dst.add(2)) = third; * (dst.add(3)) = fourth; } #[inline(always)] unsafe
    fn pack_alu(first : usize, second : usize, third : usize, fourth : usize, dst : * mut
    usize) { let word = ((0x00FF_0000_0000_0000usize & second) << 8) |
    ((0x0000_00FF_0000_0000usize & second) << 16) | ((0x0000_0000_00FF_0000usize &
    second) << 24) | ((0x0000_0000_0000_00FFusize & second) << 32) |
    ((0x00FF_0000_0000_0000usize & first) >> 24) | ((0x0000_00FF_0000_0000usize & first)
    >> 16) | ((0x0000_0000_00FF_0000usize & first) >> 8) | (0x0000_0000_0000_00FFusize &
    first); let second_word = ((0x00FF_0000_0000_0000usize & fourth) << 8) |
    ((0x0000_00FF_0000_0000usize & fourth) << 16) | ((0x0000_0000_00FF_0000usize &
    fourth) << 24) | ((0x0000_0000_0000_00FFusize & fourth) << 32) |
    ((0x00FF_0000_0000_0000usize & third) >> 24) | ((0x0000_00FF_0000_0000usize & third)
    >> 16) | ((0x0000_0000_00FF_0000usize & third) >> 8) | (0x0000_0000_0000_00FFusize &
    third); * dst = word; * (dst.add(1)) = second_word; } } else if
    #[cfg(all(target_endian = "little", target_pointer_width = "32"))] { pub const
    ALU_STRIDE_SIZE : usize = 8; pub const MAX_STRIDE_SIZE : usize = 8; pub const
    ALU_ALIGNMENT : usize = 4; pub const ALU_ALIGNMENT_MASK : usize = 3;
    #[inline(always)] unsafe fn unpack_alu(word : usize, second_word : usize, dst : * mut
    usize) { let first = ((0x0000_FF00usize & word) << 8) | (0x0000_00FFusize & word);
    let second = ((0xFF00_0000usize & word) >> 8) | ((0x00FF_0000usize & word) >> 16);
    let third = ((0x0000_FF00usize & second_word) << 8) | (0x0000_00FFusize &
    second_word); let fourth = ((0xFF00_0000usize & second_word) >> 8) |
    ((0x00FF_0000usize & second_word) >> 16); * dst = first; * (dst.add(1)) = second; *
    (dst.add(2)) = third; * (dst.add(3)) = fourth; } #[inline(always)] unsafe fn
    pack_alu(first : usize, second : usize, third : usize, fourth : usize, dst : * mut
    usize) { let word = ((0x00FF_0000usize & second) << 8) | ((0x0000_00FFusize & second)
    << 16) | ((0x00FF_0000usize & first) >> 8) | (0x0000_00FFusize & first); let
    second_word = ((0x00FF_0000usize & fourth) << 8) | ((0x0000_00FFusize & fourth) <<
    16) | ((0x00FF_0000usize & third) >> 8) | (0x0000_00FFusize & third); * dst = word; *
    (dst.add(1)) = second_word; } } else if #[cfg(all(target_endian = "big",
    target_pointer_width = "64"))] { pub const ALU_STRIDE_SIZE : usize = 16; pub const
    MAX_STRIDE_SIZE : usize = 16; pub const ALU_ALIGNMENT : usize = 8; pub const
    ALU_ALIGNMENT_MASK : usize = 7; #[inline(always)] unsafe fn unpack_alu(word : usize,
    second_word : usize, dst : * mut usize) { let first = ((0xFF00_0000_0000_0000usize &
    word) >> 8) | ((0x00FF_0000_0000_0000usize & word) >> 16) |
    ((0x0000_FF00_0000_0000usize & word) >> 24) | ((0x0000_00FF_0000_0000usize & word) >>
    32); let second = ((0x0000_0000_FF00_0000usize & word) << 24) |
    ((0x0000_0000_00FF_0000usize & word) << 16) | ((0x0000_0000_0000_FF00usize & word) <<
    8) | (0x0000_0000_0000_00FFusize & word); let third = ((0xFF00_0000_0000_0000usize &
    second_word) >> 8) | ((0x00FF_0000_0000_0000usize & second_word) >> 16) |
    ((0x0000_FF00_0000_0000usize & second_word) >> 24) | ((0x0000_00FF_0000_0000usize &
    second_word) >> 32); let fourth = ((0x0000_0000_FF00_0000usize & second_word) << 24)
    | ((0x0000_0000_00FF_0000usize & second_word) << 16) | ((0x0000_0000_0000_FF00usize &
    second_word) << 8) | (0x0000_0000_0000_00FFusize & second_word); * dst = first; *
    (dst.add(1)) = second; * (dst.add(2)) = third; * (dst.add(3)) = fourth; }
    #[inline(always)] unsafe fn pack_alu(first : usize, second : usize, third : usize,
    fourth : usize, dst : * mut usize) { let word = ((0x00FF0000_00000000usize & first)
    << 8) | ((0x000000FF_00000000usize & first) << 16) | ((0x00000000_00FF0000usize &
    first) << 24) | ((0x00000000_000000FFusize & first) << 32) |
    ((0x00FF0000_00000000usize & second) >> 24) | ((0x000000FF_00000000usize & second) >>
    16) | ((0x00000000_00FF0000usize & second) >> 8) | (0x00000000_000000FFusize &
    second); let second_word = ((0x00FF0000_00000000usize & third) << 8) |
    ((0x000000FF_00000000usize & third) << 16) | ((0x00000000_00FF0000usize & third) <<
    24) | ((0x00000000_000000FFusize & third) << 32) | ((0x00FF0000_00000000usize &
    fourth) >> 24) | ((0x000000FF_00000000usize & fourth) >> 16) |
    ((0x00000000_00FF0000usize & fourth) >> 8) | (0x00000000_000000FFusize & fourth); *
    dst = word; * (dst.add(1)) = second_word; } } else if #[cfg(all(target_endian =
    "big", target_pointer_width = "32"))] { pub const ALU_STRIDE_SIZE : usize = 8; pub
    const MAX_STRIDE_SIZE : usize = 8; pub const ALU_ALIGNMENT : usize = 4; pub const
    ALU_ALIGNMENT_MASK : usize = 3; #[inline(always)] unsafe fn unpack_alu(word : usize,
    second_word : usize, dst : * mut usize) { let first = ((0xFF00_0000usize & word) >>
    8) | ((0x00FF_0000usize & word) >> 16); let second = ((0x0000_FF00usize & word) << 8)
    | (0x0000_00FFusize & word); let third = ((0xFF00_0000usize & second_word) >> 8) |
    ((0x00FF_0000usize & second_word) >> 16); let fourth = ((0x0000_FF00usize &
    second_word) << 8) | (0x0000_00FFusize & second_word); * dst = first; * (dst.add(1))
    = second; * (dst.add(2)) = third; * (dst.add(3)) = fourth; } #[inline(always)] unsafe
    fn pack_alu(first : usize, second : usize, third : usize, fourth : usize, dst : * mut
    usize) { let word = ((0x00FF_0000usize & first) << 8) | ((0x0000_00FFusize & first)
    << 16) | ((0x00FF_0000usize & second) >> 8) | (0x0000_00FFusize & second); let
    second_word = ((0x00FF_0000usize & third) << 8) | ((0x0000_00FFusize & third) << 16)
    | ((0x00FF_0000usize & fourth) >> 8) | (0x0000_00FFusize & fourth); * dst = word; *
    (dst.add(1)) = second_word; } } else { ascii_naive!(ascii_to_ascii, u8, u8);
    ascii_naive!(ascii_to_basic_latin, u8, u16); ascii_naive!(basic_latin_to_ascii, u16,
    u8); }
}
cfg_if! {
    if #[cfg(target_endian = "little")] { #[allow(dead_code)] #[inline(always)] fn
    count_zeros(word : usize) -> u32 { word.trailing_zeros() } } else {
    #[allow(dead_code)] #[inline(always)] fn count_zeros(word : usize) -> u32 { word
    .leading_zeros() } }
}
cfg_if! {
    if #[cfg(all(feature = "simd-accel", target_endian = "little", target_arch =
    "disabled"))] { #[inline(always)] pub fn validate_ascii(slice : & [u8]) -> Option <
    (u8, usize) > { let src = slice.as_ptr(); let len = slice.len(); let mut offset =
    0usize; if SIMD_STRIDE_SIZE <= len { let len_minus_stride = len - SIMD_STRIDE_SIZE;
    loop { let simd = unsafe { load16_unaligned(src.add(offset)) }; if !
    simd_is_ascii(simd) { break; } offset += SIMD_STRIDE_SIZE; if offset >
    len_minus_stride { break; } } } while offset < len { let code_unit = slice[offset];
    if code_unit > 127 { return Some((code_unit, offset)); } offset += 1; } None } } else
    if #[cfg(all(feature = "simd-accel", target_feature = "sse2"))] { #[inline(always)]
    pub fn validate_ascii(slice : & [u8]) -> Option < (u8, usize) > { let src = slice
    .as_ptr(); let len = slice.len(); let mut offset = 0usize; if SIMD_STRIDE_SIZE <= len
    { let simd = unsafe { load16_unaligned(src) }; let mask = mask_ascii(simd); if mask
    != 0 { offset = mask.trailing_zeros() as usize; let non_ascii = unsafe { * src
    .add(offset) }; return Some((non_ascii, offset)); } offset = SIMD_STRIDE_SIZE; let
    until_alignment = unsafe { (SIMD_ALIGNMENT - ((src.add(offset) as usize) &
    SIMD_ALIGNMENT_MASK)) & SIMD_ALIGNMENT_MASK }; if until_alignment + (SIMD_STRIDE_SIZE
    * 3) <= len { if until_alignment != 0 { let simd = unsafe { load16_unaligned(src
    .add(offset)) }; let mask = mask_ascii(simd); if mask != 0 { offset += mask
    .trailing_zeros() as usize; let non_ascii = unsafe { * src.add(offset) }; return
    Some((non_ascii, offset)); } offset += until_alignment; } let
    len_minus_stride_times_two = len - (SIMD_STRIDE_SIZE * 2); loop { let first = unsafe
    { load16_aligned(src.add(offset)) }; let second = unsafe { load16_aligned(src
    .add(offset + SIMD_STRIDE_SIZE)) }; if ! simd_is_ascii(first | second) { let
    mask_first = mask_ascii(first); if mask_first != 0 { offset += mask_first
    .trailing_zeros() as usize; } else { let mask_second = mask_ascii(second); offset +=
    SIMD_STRIDE_SIZE + mask_second.trailing_zeros() as usize; } let non_ascii = unsafe {
    * src.add(offset) }; return Some((non_ascii, offset)); } offset += SIMD_STRIDE_SIZE *
    2; if offset > len_minus_stride_times_two { break; } } if offset + SIMD_STRIDE_SIZE
    <= len { let simd = unsafe { load16_aligned(src.add(offset)) }; let mask =
    mask_ascii(simd); if mask != 0 { offset += mask.trailing_zeros() as usize; let
    non_ascii = unsafe { * src.add(offset) }; return Some((non_ascii, offset)); } offset
    += SIMD_STRIDE_SIZE; } } else { if offset + SIMD_STRIDE_SIZE <= len { let simd =
    unsafe { load16_unaligned(src.add(offset)) }; let mask = mask_ascii(simd); if mask !=
    0 { offset += mask.trailing_zeros() as usize; let non_ascii = unsafe { * src
    .add(offset) }; return Some((non_ascii, offset)); } offset += SIMD_STRIDE_SIZE; if
    offset + SIMD_STRIDE_SIZE <= len { let simd = unsafe { load16_unaligned(src
    .add(offset)) }; let mask = mask_ascii(simd); if mask != 0 { offset += mask
    .trailing_zeros() as usize; let non_ascii = unsafe { * src.add(offset) }; return
    Some((non_ascii, offset)); } offset += SIMD_STRIDE_SIZE; } } } } while offset < len {
    let code_unit = unsafe { * (src.add(offset)) }; if code_unit > 127 { return
    Some((code_unit, offset)); } offset += 1; } None } } else { #[inline(always)] fn
    find_non_ascii(word : usize, second_word : usize) -> Option < usize > { let
    word_masked = word & ASCII_MASK; let second_masked = second_word & ASCII_MASK; if
    (word_masked | second_masked) == 0 { return None; } if word_masked != 0 { let zeros =
    count_zeros(word_masked); let num_ascii = (zeros >> 3) as usize; return
    Some(num_ascii); } let zeros = count_zeros(second_masked); let num_ascii = (zeros >>
    3) as usize; Some(ALU_ALIGNMENT + num_ascii) } #[inline(always)] unsafe fn
    validate_ascii_stride(src : * const usize) -> Option < usize > { let word = * src;
    let second_word = * (src.add(1)); find_non_ascii(word, second_word) }
    #[cfg_attr(feature = "cargo-clippy", allow(cast_ptr_alignment))] #[inline(always)]
    pub fn validate_ascii(slice : & [u8]) -> Option < (u8, usize) > { let src = slice
    .as_ptr(); let len = slice.len(); let mut offset = 0usize; let mut until_alignment =
    (ALU_ALIGNMENT - ((src as usize) & ALU_ALIGNMENT_MASK)) & ALU_ALIGNMENT_MASK; if
    until_alignment + ALU_STRIDE_SIZE <= len { while until_alignment != 0 { let code_unit
    = slice[offset]; if code_unit > 127 { return Some((code_unit, offset)); } offset +=
    1; until_alignment -= 1; } let len_minus_stride = len - ALU_STRIDE_SIZE; loop { let
    ptr = unsafe { src.add(offset) as * const usize }; if let Some(num_ascii) = unsafe {
    validate_ascii_stride(ptr) } { offset += num_ascii; return Some((unsafe { * (src
    .add(offset)) }, offset)); } offset += ALU_STRIDE_SIZE; if offset > len_minus_stride
    { break; } } } while offset < len { let code_unit = slice[offset]; if code_unit > 127
    { return Some((code_unit, offset)); } offset += 1; } None } }
}
cfg_if! {
    if #[cfg(all(feature = "simd-accel", any(target_feature = "sse2", all(target_endian =
    "little", target_arch = "aarch64"))))] {} else if #[cfg(all(feature = "simd-accel",
    target_endian = "little", target_feature = "neon"))] { pub const ALU_STRIDE_SIZE :
    usize = 8; pub const ALU_ALIGNMENT : usize = 4; pub const ALU_ALIGNMENT_MASK : usize
    = 3; } else { #[inline(always)] unsafe fn unpack_latin1_stride_alu(src : * const
    usize, dst : * mut usize) { let word = * src; let second_word = * (src.add(1));
    unpack_alu(word, second_word, dst); } #[inline(always)] unsafe fn
    pack_latin1_stride_alu(src : * const usize, dst : * mut usize) { let first = * src;
    let second = * (src.add(1)); let third = * (src.add(2)); let fourth = * (src.add(3));
    pack_alu(first, second, third, fourth, dst); } #[inline(always)] unsafe fn
    ascii_to_basic_latin_stride_alu(src : * const usize, dst : * mut usize) -> bool { let
    word = * src; let second_word = * (src.add(1)); if (word & ASCII_MASK) | (second_word
    & ASCII_MASK) != 0 { return false; } unpack_alu(word, second_word, dst); true }
    #[inline(always)] unsafe fn basic_latin_to_ascii_stride_alu(src : * const usize, dst
    : * mut usize) -> bool { let first = * src; let second = * (src.add(1)); let third =
    * (src.add(2)); let fourth = * (src.add(3)); if (first & BASIC_LATIN_MASK) | (second
    & BASIC_LATIN_MASK) | (third & BASIC_LATIN_MASK) | (fourth & BASIC_LATIN_MASK) != 0 {
    return false; } pack_alu(first, second, third, fourth, dst); true } #[inline(always)]
    unsafe fn ascii_to_ascii_stride(src : * const usize, dst : * mut usize) -> Option <
    usize > { let word = * src; let second_word = * (src.add(1)); * dst = word; * (dst
    .add(1)) = second_word; find_non_ascii(word, second_word) }
    basic_latin_alu!(ascii_to_basic_latin, u8, u16, ascii_to_basic_latin_stride_alu);
    basic_latin_alu!(basic_latin_to_ascii, u16, u8, basic_latin_to_ascii_stride_alu);
    latin1_alu!(unpack_latin1, u8, u16, unpack_latin1_stride_alu);
    latin1_alu!(pack_latin1, u16, u8, pack_latin1_stride_alu); ascii_alu!(ascii_to_ascii,
    u8, u8, ascii_to_ascii_stride); }
}
pub fn ascii_valid_up_to(bytes: &[u8]) -> usize {
    match validate_ascii(bytes) {
        None => bytes.len(),
        Some((_, num_valid)) => num_valid,
    }
}
pub fn iso_2022_jp_ascii_valid_up_to(bytes: &[u8]) -> usize {
    for (i, b_ref) in bytes.iter().enumerate() {
        let b = *b_ref;
        if b >= 0x80 || b == 0x1B || b == 0x0E || b == 0x0F {
            return i;
        }
    }
    bytes.len()
}
#[cfg(test)]
mod tests {
    use super::*;
    macro_rules! test_ascii {
        ($test_name:ident, $fn_tested:ident, $src_unit:ty, $dst_unit:ty) => {
            #[test] fn $test_name () { let mut src : Vec <$src_unit > =
            Vec::with_capacity(32); let mut dst : Vec <$dst_unit > =
            Vec::with_capacity(32); for i in 0..32 { src.clear(); dst.clear(); dst
            .resize(32, 0); for j in 0..32 { let c = if i == j { 0xAA } else { j + 0x40
            }; src.push(c as $src_unit); } match unsafe { $fn_tested (src.as_ptr(), dst
            .as_mut_ptr(), 32) } { None => unreachable!("Should always find non-ASCII"),
            Some((non_ascii, num_ascii)) => { assert_eq!(non_ascii, 0xAA);
            assert_eq!(num_ascii, i); for j in 0..i { assert_eq!(dst[j], (j + 0x40) as
            $dst_unit); } } } } }
        };
    }
    test_ascii!(test_ascii_to_ascii, ascii_to_ascii, u8, u8);
    test_ascii!(test_ascii_to_basic_latin, ascii_to_basic_latin, u8, u16);
    test_ascii!(test_basic_latin_to_ascii, basic_latin_to_ascii, u16, u8);
}
#[cfg(test)]
mod tests_rug_80 {
    use super::*;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: bool = rug_fuzz_0;
        unsafe {
            debug_assert_eq!(crate ::ascii::unlikely(p0), true);
        }
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_82 {
    use super::*;
    #[test]
    fn test_unpack_alu() {
        let mut p0: usize = 0x1234_5678_90AB_CDEF;
        let mut p1: usize = 0xFEDC_BA09_8765_4321;
        let mut p2: *mut usize = &mut 0;
        unsafe {
            crate::ascii::unpack_alu(p0, p1, p2);
        }
        assert_eq!(unsafe { * p2 }, 0x7856_3412);
        assert_eq!(unsafe { * p2.add(1) }, 0xEFCD_A980);
        assert_eq!(unsafe { * p2.add(2) }, 0xEFCD_A980);
        assert_eq!(unsafe { * p2.add(3) }, 0x2134_5678);
    }
}
#[cfg(test)]
mod tests_rug_83 {
    use super::*;
    #[test]
    fn test_rug() {
        let mut p0: usize = 0x1122_3344_5566_7788;
        let mut p1: usize = 0x8877_6655_4433_2211;
        let mut p2: usize = 0x5566_9988_1122_3344;
        let mut p3: usize = 0x4433_AA55_BB66_CC77;
        let mut p4: *mut usize = &mut 0;
        unsafe {
            crate::ascii::pack_alu(p0, p1, p2, p3, p4);
        }
    }
}
#[cfg(test)]
mod tests_rug_84 {
    use super::*;
    #[test]
    fn test_count_zeros() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: usize = rug_fuzz_0;
        crate::ascii::count_zeros(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_85 {
    use super::*;
    #[test]
    fn test_find_non_ascii() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: usize = rug_fuzz_0;
        let p1: usize = rug_fuzz_1;
        crate::ascii::find_non_ascii(p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_87 {
    use super::*;
    #[test]
    fn test_validate_ascii() {
        let _rug_st_tests_rug_87_rrrruuuugggg_test_validate_ascii = 0;
        let rug_fuzz_0 = b"Hello, World!";
        let p0: &[u8] = rug_fuzz_0;
        crate::ascii::validate_ascii(p0);
        let _rug_ed_tests_rug_87_rrrruuuugggg_test_validate_ascii = 0;
    }
}
#[cfg(test)]
mod tests_rug_88 {
    use super::*;
    use std::ptr;
    #[test]
    fn test_unpack_latin1_stride_alu() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(usize, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let src_data: [usize; 2] = [rug_fuzz_0, rug_fuzz_1];
        let mut dst_data: [usize; 1] = [rug_fuzz_2];
        let p0: *const usize = src_data.as_ptr();
        let p1: *mut usize = dst_data.as_mut_ptr();
        unsafe {
            crate::ascii::unpack_latin1_stride_alu(p0, p1);
        }
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_89 {
    use super::*;
    use std::ptr;
    #[test]
    fn test_pack_latin1_stride_alu() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5)) = <(usize, usize, usize, usize, usize, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let src_data: [usize; 4] = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3];
        let mut dst_data: [usize; 1] = [rug_fuzz_4; 1];
        let src_ptr: *const usize = src_data.as_ptr();
        let dst_ptr: *mut usize = dst_data.as_mut_ptr();
        unsafe {
            pack_latin1_stride_alu(src_ptr, dst_ptr);
        }
        debug_assert_eq!(rug_fuzz_5, 1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_90 {
    use super::*;
    use std::ptr;
    const ASCII_MASK: usize = 0;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(usize, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let src_data: [usize; 2] = [rug_fuzz_0, rug_fuzz_1];
        let mut src_ptr: *const usize = src_data.as_ptr();
        let mut dst_data: [usize; 2] = [rug_fuzz_2; 2];
        let mut dst_ptr: *mut usize = dst_data.as_mut_ptr();
        unsafe {
            ascii_to_basic_latin_stride_alu(src_ptr, dst_ptr);
        }
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_91 {
    use super::*;
    use crate::ascii::{basic_latin_to_ascii_stride_alu, BASIC_LATIN_MASK, pack_alu};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = [rug_fuzz_0; 4];
        let p1 = &mut [rug_fuzz_1; 4] as *mut usize;
        unsafe {
            basic_latin_to_ascii_stride_alu(p0.as_ptr(), p1);
        }
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_92 {
    use super::*;
    use std::ptr;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(usize, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let src_data: [usize; 2] = [rug_fuzz_0, rug_fuzz_1];
        let mut dst_data: [usize; 2] = [rug_fuzz_2; 2];
        let p0 = src_data.as_ptr();
        let p1: *mut usize = dst_data.as_mut_ptr();
        unsafe {
            debug_assert_eq!(crate ::ascii::ascii_to_ascii_stride(p0, p1), None);
        }
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_96 {
    use super::*;
    use crate::ascii;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(&str, u8, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: *const u16 = rug_fuzz_0.as_ptr() as *const u16;
        let mut dst: [u8; 10] = [rug_fuzz_1; 10];
        let p1: *mut u8 = dst.as_mut_ptr();
        let p2: usize = rug_fuzz_2;
        unsafe { ascii::pack_latin1(p0, p1, p2) };
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_98 {
    use super::*;
    #[test]
    fn test_ascii_valid_up_to() {
        let _rug_st_tests_rug_98_rrrruuuugggg_test_ascii_valid_up_to = 0;
        let rug_fuzz_0 = b"hello world";
        let p0: &[u8] = rug_fuzz_0;
        debug_assert_eq!(crate ::ascii::ascii_valid_up_to(p0), 11);
        let _rug_ed_tests_rug_98_rrrruuuugggg_test_ascii_valid_up_to = 0;
    }
}
#[cfg(test)]
mod tests_rug_99 {
    use super::*;
    #[test]
    fn test_iso_2022_jp_ascii_valid_up_to() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4)) = <(u8, u8, u8, u8, u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: &[u8] = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3, rug_fuzz_4];
        debug_assert_eq!(crate ::ascii::iso_2022_jp_ascii_valid_up_to(p0), 5);
             }
}
}
}    }
}
