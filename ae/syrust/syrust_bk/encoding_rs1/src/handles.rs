// Copyright Mozilla Foundation. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! This module provides structs that use lifetimes to couple bounds checking
//! and space availability checking and detaching those from actual slice
//! reading/writing.
//!
//! At present, the internals of the implementation are safe code, so the
//! bound checks currently also happen on read/write. Once this code works,
//! the plan is to replace the internals with unsafe code that omits the
//! bound check at the read/write time.

#[cfg(all(
    feature = "simd-accel",
    any(
        target_feature = "sse2",
        all(target_endian = "little", target_arch = "aarch64"),
        all(target_endian = "little", target_feature = "neon")
    )
))]
use simd_funcs::*;

#[cfg(all(
    feature = "simd-accel",
    any(
        target_feature = "sse2",
        all(target_endian = "little", target_arch = "aarch64"),
        all(target_endian = "little", target_feature = "neon")
    )
))]
use packed_simd::u16x8;

use super::DecoderResult;
use super::EncoderResult;
use ascii::*;
use utf_8::convert_utf8_to_utf16_up_to_invalid;
use utf_8::utf8_valid_up_to;

pub enum Space<T> {
    Available(T),
    Full(usize),
}

pub enum CopyAsciiResult<T, U> {
    Stop(T),
    GoOn(U),
}

pub enum NonAscii {
    BmpExclAscii(u16),
    Astral(char),
}

pub enum Unicode {
    Ascii(u8),
    NonAscii(NonAscii),
}

// Start UTF-16LE/BE fast path

pub trait Endian {
    const OPPOSITE_ENDIAN: bool;
}

pub struct BigEndian;

impl Endian for BigEndian {
    #[cfg(target_endian = "little")]
    const OPPOSITE_ENDIAN: bool = true;

    #[cfg(target_endian = "big")]
    const OPPOSITE_ENDIAN: bool = false;
}

pub struct LittleEndian;

impl Endian for LittleEndian {
    #[cfg(target_endian = "little")]
    const OPPOSITE_ENDIAN: bool = false;

    #[cfg(target_endian = "big")]
    const OPPOSITE_ENDIAN: bool = true;
}

#[derive(Debug, Copy, Clone)]
struct UnalignedU16Slice {
    ptr: *const u8,
    len: usize,
}

impl UnalignedU16Slice {
    #[inline(always)]
    pub unsafe fn new(ptr: *const u8, len: usize) -> UnalignedU16Slice {
        UnalignedU16Slice { ptr, len }
    }

    #[inline(always)]
    pub fn trim_last(&mut self) {
        assert!(self.len > 0);
        self.len -= 1;
    }

    #[inline(always)]
    pub fn at(&self, i: usize) -> u16 {
        assert!(i < self.len);
        unsafe {
            let mut u: u16 = ::std::mem::uninitialized();
            ::std::ptr::copy_nonoverlapping(self.ptr.add(i * 2), &mut u as *mut u16 as *mut u8, 2);
            u
        }
    }

    #[cfg(feature = "simd-accel")]
    #[inline(always)]
    pub fn simd_at(&self, i: usize) -> u16x8 {
        assert!(i + SIMD_STRIDE_SIZE / 2 <= self.len);
        let byte_index = i * 2;
        unsafe { to_u16_lanes(load16_unaligned(self.ptr.add(byte_index))) }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline(always)]
    pub fn tail(&self, from: usize) -> UnalignedU16Slice {
        // XXX the return value should be restricted not to
        // outlive self.
        assert!(from <= self.len);
        unsafe { UnalignedU16Slice::new(self.ptr.add(from * 2), self.len - from) }
    }

    #[cfg(feature = "simd-accel")]
    #[inline(always)]
    pub fn copy_bmp_to<E: Endian>(&self, other: &mut [u16]) -> Option<(u16, usize)> {
        assert!(self.len <= other.len());
        let mut offset = 0;
        if SIMD_STRIDE_SIZE / 2 <= self.len {
            let len_minus_stride = self.len - SIMD_STRIDE_SIZE / 2;
            loop {
                let mut simd = self.simd_at(offset);
                if E::OPPOSITE_ENDIAN {
                    simd = simd_byte_swap(simd);
                }
                unsafe {
                    store8_unaligned(other.as_mut_ptr().add(offset), simd);
                }
                if contains_surrogates(simd) {
                    break;
                }
                offset += SIMD_STRIDE_SIZE / 2;
                if offset > len_minus_stride {
                    break;
                }
            }
        }
        while offset < self.len {
            let unit = swap_if_opposite_endian::<E>(self.at(offset));
            other[offset] = unit;
            if super::in_range16(unit, 0xD800, 0xE000) {
                return Some((unit, offset));
            }
            offset += 1;
        }
        None
    }

    #[cfg(not(feature = "simd-accel"))]
    #[inline(always)]
    fn copy_bmp_to<E: Endian>(&self, other: &mut [u16]) -> Option<(u16, usize)> {
        assert!(self.len <= other.len());
        for (i, target) in other.iter_mut().enumerate().take(self.len) {
            let unit = swap_if_opposite_endian::<E>(self.at(i));
            *target = unit;
            if super::in_range16(unit, 0xD800, 0xE000) {
                return Some((unit, i));
            }
        }
        None
    }
}

#[inline(always)]
fn copy_unaligned_basic_latin_to_ascii_alu<E: Endian>(
    src: UnalignedU16Slice,
    dst: &mut [u8],
    offset: usize,
) -> CopyAsciiResult<usize, (u16, usize)> {
    let len = ::std::cmp::min(src.len(), dst.len());
    let mut i = 0usize;
    loop {
        if i == len {
            return CopyAsciiResult::Stop(i + offset);
        }
        let unit = swap_if_opposite_endian::<E>(src.at(i));
        if unit > 0x7F {
            return CopyAsciiResult::GoOn((unit, i + offset));
        }
        dst[i] = unit as u8;
        i += 1;
    }
}

#[inline(always)]
fn swap_if_opposite_endian<E: Endian>(unit: u16) -> u16 {
    if E::OPPOSITE_ENDIAN {
        unit.swap_bytes()
    } else {
        unit
    }
}

#[cfg(not(feature = "simd-accel"))]
#[inline(always)]
fn copy_unaligned_basic_latin_to_ascii<E: Endian>(
    src: UnalignedU16Slice,
    dst: &mut [u8],
) -> CopyAsciiResult<usize, (u16, usize)> {
    copy_unaligned_basic_latin_to_ascii_alu::<E>(src, dst, 0)
}

#[cfg(feature = "simd-accel")]
#[inline(always)]
fn copy_unaligned_basic_latin_to_ascii<E: Endian>(
    src: UnalignedU16Slice,
    dst: &mut [u8],
) -> CopyAsciiResult<usize, (u16, usize)> {
    let len = ::std::cmp::min(src.len(), dst.len());
    let mut offset = 0;
    if SIMD_STRIDE_SIZE <= len {
        let len_minus_stride = len - SIMD_STRIDE_SIZE;
        loop {
            let mut first = src.simd_at(offset);
            let mut second = src.simd_at(offset + (SIMD_STRIDE_SIZE / 2));
            if E::OPPOSITE_ENDIAN {
                first = simd_byte_swap(first);
                second = simd_byte_swap(second);
            }
            if !simd_is_basic_latin(first | second) {
                break;
            }
            let packed = simd_pack(first, second);
            unsafe {
                store16_unaligned(dst.as_mut_ptr().add(offset), packed);
            }
            offset += SIMD_STRIDE_SIZE;
            if offset > len_minus_stride {
                break;
            }
        }
    }
    copy_unaligned_basic_latin_to_ascii_alu::<E>(src.tail(offset), &mut dst[offset..], offset)
}

#[inline(always)]
fn convert_unaligned_utf16_to_utf8<E: Endian>(
    src: UnalignedU16Slice,
    dst: &mut [u8],
) -> (usize, usize, bool) {
    if dst.len() < 4 {
        return (0, 0, false);
    }
    let mut src_pos = 0usize;
    let mut dst_pos = 0usize;
    let src_len = src.len();
    let dst_len_minus_three = dst.len() - 3;
    'outer: loop {
        let mut non_ascii = match copy_unaligned_basic_latin_to_ascii::<E>(
            src.tail(src_pos),
            &mut dst[dst_pos..],
        ) {
            CopyAsciiResult::GoOn((unit, read_written)) => {
                src_pos += read_written;
                dst_pos += read_written;
                unit
            }
            CopyAsciiResult::Stop(read_written) => {
                return (src_pos + read_written, dst_pos + read_written, false);
            }
        };
        if dst_pos >= dst_len_minus_three {
            break 'outer;
        }
        // We have enough destination space to commit to
        // having read `non_ascii`.
        src_pos += 1;
        'inner: loop {
            let non_ascii_minus_surrogate_start = non_ascii.wrapping_sub(0xD800);
            if non_ascii_minus_surrogate_start > (0xDFFF - 0xD800) {
                if non_ascii < 0x800 {
                    dst[dst_pos] = ((non_ascii >> 6) | 0xC0) as u8;
                    dst_pos += 1;
                    dst[dst_pos] = ((non_ascii & 0x3F) | 0x80) as u8;
                    dst_pos += 1;
                } else {
                    dst[dst_pos] = ((non_ascii >> 12) | 0xE0) as u8;
                    dst_pos += 1;
                    dst[dst_pos] = (((non_ascii & 0xFC0) >> 6) | 0x80) as u8;
                    dst_pos += 1;
                    dst[dst_pos] = ((non_ascii & 0x3F) | 0x80) as u8;
                    dst_pos += 1;
                }
            } else if non_ascii_minus_surrogate_start <= (0xDBFF - 0xD800) {
                // high surrogate
                if src_pos < src_len {
                    let second = swap_if_opposite_endian::<E>(src.at(src_pos));
                    let second_minus_low_surrogate_start = second.wrapping_sub(0xDC00);
                    if second_minus_low_surrogate_start <= (0xDFFF - 0xDC00) {
                        // The next code unit is a low surrogate. Advance position.
                        src_pos += 1;
                        let point = (u32::from(non_ascii) << 10) + u32::from(second)
                            - (((0xD800u32 << 10) - 0x10000u32) + 0xDC00u32);

                        dst[dst_pos] = ((point >> 18) | 0xF0u32) as u8;
                        dst_pos += 1;
                        dst[dst_pos] = (((point & 0x3F000u32) >> 12) | 0x80u32) as u8;
                        dst_pos += 1;
                        dst[dst_pos] = (((point & 0xFC0u32) >> 6) | 0x80u32) as u8;
                        dst_pos += 1;
                        dst[dst_pos] = ((point & 0x3Fu32) | 0x80u32) as u8;
                        dst_pos += 1;
                    } else {
                        // The next code unit is not a low surrogate. Don't advance
                        // position and treat the high surrogate as unpaired.
                        return (src_pos, dst_pos, true);
                    }
                } else {
                    // Unpaired surrogate at the end of buffer
                    return (src_pos, dst_pos, true);
                }
            } else {
                // Unpaired low surrogate
                return (src_pos, dst_pos, true);
            }
            if dst_pos >= dst_len_minus_three || src_pos == src_len {
                break 'outer;
            }
            let unit = swap_if_opposite_endian::<E>(src.at(src_pos));
            src_pos += 1;
            if unit > 0x7F {
                non_ascii = unit;
                continue 'inner;
            }
            dst[dst_pos] = unit as u8;
            dst_pos += 1;
            continue 'outer;
        }
    }
    (src_pos, dst_pos, false)
}

// Byte source

pub struct ByteSource<'a> {
    slice: &'a [u8],
    pos: usize,
}

impl<'a> ByteSource<'a> {
    #[inline(always)]
    pub fn new(src: &[u8]) -> ByteSource {
        ByteSource { slice: src, pos: 0 }
    }
    #[inline(always)]
    pub fn check_available<'b>(&'b mut self) -> Space<ByteReadHandle<'b, 'a>> {
        if self.pos < self.slice.len() {
            Space::Available(ByteReadHandle::new(self))
        } else {
            Space::Full(self.consumed())
        }
    }
    #[inline(always)]
    fn read(&mut self) -> u8 {
        let ret = self.slice[self.pos];
        self.pos += 1;
        ret
    }
    #[inline(always)]
    fn unread(&mut self) -> usize {
        self.pos -= 1;
        self.pos
    }
    #[inline(always)]
    pub fn consumed(&self) -> usize {
        self.pos
    }
}

pub struct ByteReadHandle<'a, 'b>
where
    'b: 'a,
{
    source: &'a mut ByteSource<'b>,
}

impl<'a, 'b> ByteReadHandle<'a, 'b>
where
    'b: 'a,
{
    #[inline(always)]
    fn new(src: &'a mut ByteSource<'b>) -> ByteReadHandle<'a, 'b> {
        ByteReadHandle { source: src }
    }
    #[inline(always)]
    pub fn read(self) -> (u8, ByteUnreadHandle<'a, 'b>) {
        let byte = self.source.read();
        let handle = ByteUnreadHandle::new(self.source);
        (byte, handle)
    }
    #[inline(always)]
    pub fn consumed(&self) -> usize {
        self.source.consumed()
    }
}

pub struct ByteUnreadHandle<'a, 'b>
where
    'b: 'a,
{
    source: &'a mut ByteSource<'b>,
}

impl<'a, 'b> ByteUnreadHandle<'a, 'b>
where
    'b: 'a,
{
    #[inline(always)]
    fn new(src: &'a mut ByteSource<'b>) -> ByteUnreadHandle<'a, 'b> {
        ByteUnreadHandle { source: src }
    }
    #[inline(always)]
    pub fn unread(self) -> usize {
        self.source.unread()
    }
    #[inline(always)]
    pub fn consumed(&self) -> usize {
        self.source.consumed()
    }
    #[inline(always)]
    pub fn commit(self) -> &'a mut ByteSource<'b> {
        self.source
    }
}

// UTF-16 destination

pub struct Utf16BmpHandle<'a, 'b>
where
    'b: 'a,
{
    dest: &'a mut Utf16Destination<'b>,
}

impl<'a, 'b> Utf16BmpHandle<'a, 'b>
where
    'b: 'a,
{
    #[inline(always)]
    fn new(dst: &'a mut Utf16Destination<'b>) -> Utf16BmpHandle<'a, 'b> {
        Utf16BmpHandle { dest: dst }
    }
    #[inline(always)]
    pub fn written(&self) -> usize {
        self.dest.written()
    }
    #[inline(always)]
    pub fn write_ascii(self, ascii: u8) -> &'a mut Utf16Destination<'b> {
        self.dest.write_ascii(ascii);
        self.dest
    }
    #[inline(always)]
    pub fn write_bmp(self, bmp: u16) -> &'a mut Utf16Destination<'b> {
        self.dest.write_bmp(bmp);
        self.dest
    }
    #[inline(always)]
    pub fn write_bmp_excl_ascii(self, bmp: u16) -> &'a mut Utf16Destination<'b> {
        self.dest.write_bmp_excl_ascii(bmp);
        self.dest
    }
    #[inline(always)]
    pub fn write_mid_bmp(self, bmp: u16) -> &'a mut Utf16Destination<'b> {
        self.dest.write_mid_bmp(bmp);
        self.dest
    }
    #[inline(always)]
    pub fn write_upper_bmp(self, bmp: u16) -> &'a mut Utf16Destination<'b> {
        self.dest.write_upper_bmp(bmp);
        self.dest
    }
    #[inline(always)]
    pub fn commit(self) -> &'a mut Utf16Destination<'b> {
        self.dest
    }
}

pub struct Utf16AstralHandle<'a, 'b>
where
    'b: 'a,
{
    dest: &'a mut Utf16Destination<'b>,
}

impl<'a, 'b> Utf16AstralHandle<'a, 'b>
where
    'b: 'a,
{
    #[inline(always)]
    fn new(dst: &'a mut Utf16Destination<'b>) -> Utf16AstralHandle<'a, 'b> {
        Utf16AstralHandle { dest: dst }
    }
    #[inline(always)]
    pub fn written(&self) -> usize {
        self.dest.written()
    }
    #[inline(always)]
    pub fn write_ascii(self, ascii: u8) -> &'a mut Utf16Destination<'b> {
        self.dest.write_ascii(ascii);
        self.dest
    }
    #[inline(always)]
    pub fn write_bmp(self, bmp: u16) -> &'a mut Utf16Destination<'b> {
        self.dest.write_bmp(bmp);
        self.dest
    }
    #[inline(always)]
    pub fn write_bmp_excl_ascii(self, bmp: u16) -> &'a mut Utf16Destination<'b> {
        self.dest.write_bmp_excl_ascii(bmp);
        self.dest
    }
    #[inline(always)]
    pub fn write_upper_bmp(self, bmp: u16) -> &'a mut Utf16Destination<'b> {
        self.dest.write_upper_bmp(bmp);
        self.dest
    }
    #[inline(always)]
    pub fn write_astral(self, astral: u32) -> &'a mut Utf16Destination<'b> {
        self.dest.write_astral(astral);
        self.dest
    }
    #[inline(always)]
    pub fn write_surrogate_pair(self, high: u16, low: u16) -> &'a mut Utf16Destination<'b> {
        self.dest.write_surrogate_pair(high, low);
        self.dest
    }
    #[inline(always)]
    pub fn write_big5_combination(
        self,
        combined: u16,
        combining: u16,
    ) -> &'a mut Utf16Destination<'b> {
        self.dest.write_big5_combination(combined, combining);
        self.dest
    }
    #[inline(always)]
    pub fn commit(self) -> &'a mut Utf16Destination<'b> {
        self.dest
    }
}

pub struct Utf16Destination<'a> {
    slice: &'a mut [u16],
    pos: usize,
}

impl<'a> Utf16Destination<'a> {
    #[inline(always)]
    pub fn new(dst: &mut [u16]) -> Utf16Destination {
        Utf16Destination { slice: dst, pos: 0 }
    }
    #[inline(always)]
    pub fn check_space_bmp<'b>(&'b mut self) -> Space<Utf16BmpHandle<'b, 'a>> {
        if self.pos < self.slice.len() {
            Space::Available(Utf16BmpHandle::new(self))
        } else {
            Space::Full(self.written())
        }
    }
    #[inline(always)]
    pub fn check_space_astral<'b>(&'b mut self) -> Space<Utf16AstralHandle<'b, 'a>> {
        if self.pos + 1 < self.slice.len() {
            Space::Available(Utf16AstralHandle::new(self))
        } else {
            Space::Full(self.written())
        }
    }
    #[inline(always)]
    pub fn written(&self) -> usize {
        self.pos
    }
    #[inline(always)]
    fn write_code_unit(&mut self, u: u16) {
        unsafe {
            // OK, because we checked before handing out a handle.
            *(self.slice.get_unchecked_mut(self.pos)) = u;
        }
        self.pos += 1;
    }
    #[inline(always)]
    fn write_ascii(&mut self, ascii: u8) {
        debug_assert!(ascii < 0x80);
        self.write_code_unit(u16::from(ascii));
    }
    #[inline(always)]
    fn write_bmp(&mut self, bmp: u16) {
        self.write_code_unit(bmp);
    }
    #[inline(always)]
    fn write_bmp_excl_ascii(&mut self, bmp: u16) {
        debug_assert!(bmp >= 0x80);
        self.write_code_unit(bmp);
    }
    #[inline(always)]
    fn write_mid_bmp(&mut self, bmp: u16) {
        debug_assert!(bmp >= 0x80); // XXX
        self.write_code_unit(bmp);
    }
    #[inline(always)]
    fn write_upper_bmp(&mut self, bmp: u16) {
        debug_assert!(bmp >= 0x80);
        self.write_code_unit(bmp);
    }
    #[inline(always)]
    fn write_astral(&mut self, astral: u32) {
        debug_assert!(astral > 0xFFFF);
        debug_assert!(astral <= 0x10_FFFF);
        self.write_code_unit((0xD7C0 + (astral >> 10)) as u16);
        self.write_code_unit((0xDC00 + (astral & 0x3FF)) as u16);
    }
    #[inline(always)]
    pub fn write_surrogate_pair(&mut self, high: u16, low: u16) {
        self.write_code_unit(high);
        self.write_code_unit(low);
    }
    #[inline(always)]
    fn write_big5_combination(&mut self, combined: u16, combining: u16) {
        self.write_bmp_excl_ascii(combined);
        self.write_bmp_excl_ascii(combining);
    }
    #[inline(always)]
    pub fn copy_ascii_from_check_space_bmp<'b>(
        &'b mut self,
        source: &mut ByteSource,
    ) -> CopyAsciiResult<(DecoderResult, usize, usize), (u8, Utf16BmpHandle<'b, 'a>)> {
        let non_ascii_ret = {
            let src_remaining = &source.slice[source.pos..];
            let dst_remaining = &mut self.slice[self.pos..];
            let (pending, length) = if dst_remaining.len() < src_remaining.len() {
                (DecoderResult::OutputFull, dst_remaining.len())
            } else {
                (DecoderResult::InputEmpty, src_remaining.len())
            };
            match unsafe {
                ascii_to_basic_latin(src_remaining.as_ptr(), dst_remaining.as_mut_ptr(), length)
            } {
                None => {
                    source.pos += length;
                    self.pos += length;
                    return CopyAsciiResult::Stop((pending, source.pos, self.pos));
                }
                Some((non_ascii, consumed)) => {
                    source.pos += consumed;
                    self.pos += consumed;
                    source.pos += 1; // +1 for non_ascii
                    non_ascii
                }
            }
        };
        CopyAsciiResult::GoOn((non_ascii_ret, Utf16BmpHandle::new(self)))
    }
    #[inline(always)]
    pub fn copy_ascii_from_check_space_astral<'b>(
        &'b mut self,
        source: &mut ByteSource,
    ) -> CopyAsciiResult<(DecoderResult, usize, usize), (u8, Utf16AstralHandle<'b, 'a>)> {
        let non_ascii_ret = {
            let dst_len = self.slice.len();
            let src_remaining = &source.slice[source.pos..];
            let dst_remaining = &mut self.slice[self.pos..];
            let (pending, length) = if dst_remaining.len() < src_remaining.len() {
                (DecoderResult::OutputFull, dst_remaining.len())
            } else {
                (DecoderResult::InputEmpty, src_remaining.len())
            };
            match unsafe {
                ascii_to_basic_latin(src_remaining.as_ptr(), dst_remaining.as_mut_ptr(), length)
            } {
                None => {
                    source.pos += length;
                    self.pos += length;
                    return CopyAsciiResult::Stop((pending, source.pos, self.pos));
                }
                Some((non_ascii, consumed)) => {
                    source.pos += consumed;
                    self.pos += consumed;
                    if self.pos + 1 < dst_len {
                        source.pos += 1; // +1 for non_ascii
                        non_ascii
                    } else {
                        return CopyAsciiResult::Stop((
                            DecoderResult::OutputFull,
                            source.pos,
                            self.pos,
                        ));
                    }
                }
            }
        };
        CopyAsciiResult::GoOn((non_ascii_ret, Utf16AstralHandle::new(self)))
    }
    #[inline(always)]
    pub fn copy_utf8_up_to_invalid_from(&mut self, source: &mut ByteSource) {
        let src_remaining = &source.slice[source.pos..];
        let dst_remaining = &mut self.slice[self.pos..];
        let (read, written) = convert_utf8_to_utf16_up_to_invalid(src_remaining, dst_remaining);
        source.pos += read;
        self.pos += written;
    }
    #[inline(always)]
    pub fn copy_utf16_from<E: Endian>(
        &mut self,
        source: &mut ByteSource,
    ) -> Option<(usize, usize)> {
        let src_remaining = &source.slice[source.pos..];
        let dst_remaining = &mut self.slice[self.pos..];

        let mut src_unaligned = unsafe {
            UnalignedU16Slice::new(
                src_remaining.as_ptr(),
                ::std::cmp::min(src_remaining.len() / 2, dst_remaining.len()),
            )
        };
        if src_unaligned.len() == 0 {
            return None;
        }
        let last_unit = swap_if_opposite_endian::<E>(src_unaligned.at(src_unaligned.len() - 1));
        if super::in_range16(last_unit, 0xD800, 0xDC00) {
            // Last code unit is a high surrogate. It might
            // legitimately form a pair later, so let's not
            // include it.
            src_unaligned.trim_last();
        }
        let mut offset = 0usize;
        loop {
            if let Some((surrogate, bmp_len)) = {
                let src_left = src_unaligned.tail(offset);
                let dst_left = &mut dst_remaining[offset..src_unaligned.len()];
                src_left.copy_bmp_to::<E>(dst_left)
            } {
                offset += bmp_len; // surrogate has not been consumed yet
                let second_pos = offset + 1;
                if surrogate > 0xDBFF || second_pos == src_unaligned.len() {
                    // Unpaired surrogate
                    source.pos += second_pos * 2;
                    self.pos += offset;
                    return Some((source.pos, self.pos));
                }
                let second = swap_if_opposite_endian::<E>(src_unaligned.at(second_pos));
                if !super::in_range16(second, 0xDC00, 0xE000) {
                    // Unpaired surrogate
                    source.pos += second_pos * 2;
                    self.pos += offset;
                    return Some((source.pos, self.pos));
                }
                // `surrogate` was already speculatively written
                dst_remaining[second_pos] = second;
                offset += 2;
                continue;
            } else {
                source.pos += src_unaligned.len() * 2;
                self.pos += src_unaligned.len();
                return None;
            }
        }
    }
}

// UTF-8 destination

pub struct Utf8BmpHandle<'a, 'b>
where
    'b: 'a,
{
    dest: &'a mut Utf8Destination<'b>,
}

impl<'a, 'b> Utf8BmpHandle<'a, 'b>
where
    'b: 'a,
{
    #[inline(always)]
    fn new(dst: &'a mut Utf8Destination<'b>) -> Utf8BmpHandle<'a, 'b> {
        Utf8BmpHandle { dest: dst }
    }
    #[inline(always)]
    pub fn written(&self) -> usize {
        self.dest.written()
    }
    #[inline(always)]
    pub fn write_ascii(self, ascii: u8) -> &'a mut Utf8Destination<'b> {
        self.dest.write_ascii(ascii);
        self.dest
    }
    #[inline(always)]
    pub fn write_bmp(self, bmp: u16) -> &'a mut Utf8Destination<'b> {
        self.dest.write_bmp(bmp);
        self.dest
    }
    #[inline(always)]
    pub fn write_bmp_excl_ascii(self, bmp: u16) -> &'a mut Utf8Destination<'b> {
        self.dest.write_bmp_excl_ascii(bmp);
        self.dest
    }
    #[inline(always)]
    pub fn write_mid_bmp(self, bmp: u16) -> &'a mut Utf8Destination<'b> {
        self.dest.write_mid_bmp(bmp);
        self.dest
    }
    #[inline(always)]
    pub fn write_upper_bmp(self, bmp: u16) -> &'a mut Utf8Destination<'b> {
        self.dest.write_upper_bmp(bmp);
        self.dest
    }
    #[inline(always)]
    pub fn commit(self) -> &'a mut Utf8Destination<'b> {
        self.dest
    }
}

pub struct Utf8AstralHandle<'a, 'b>
where
    'b: 'a,
{
    dest: &'a mut Utf8Destination<'b>,
}

impl<'a, 'b> Utf8AstralHandle<'a, 'b>
where
    'b: 'a,
{
    #[inline(always)]
    fn new(dst: &'a mut Utf8Destination<'b>) -> Utf8AstralHandle<'a, 'b> {
        Utf8AstralHandle { dest: dst }
    }
    #[inline(always)]
    pub fn written(&self) -> usize {
        self.dest.written()
    }
    #[inline(always)]
    pub fn write_ascii(self, ascii: u8) -> &'a mut Utf8Destination<'b> {
        self.dest.write_ascii(ascii);
        self.dest
    }
    #[inline(always)]
    pub fn write_bmp(self, bmp: u16) -> &'a mut Utf8Destination<'b> {
        self.dest.write_bmp(bmp);
        self.dest
    }
    #[inline(always)]
    pub fn write_bmp_excl_ascii(self, bmp: u16) -> &'a mut Utf8Destination<'b> {
        self.dest.write_bmp_excl_ascii(bmp);
        self.dest
    }
    #[inline(always)]
    pub fn write_upper_bmp(self, bmp: u16) -> &'a mut Utf8Destination<'b> {
        self.dest.write_upper_bmp(bmp);
        self.dest
    }
    #[inline(always)]
    pub fn write_astral(self, astral: u32) -> &'a mut Utf8Destination<'b> {
        self.dest.write_astral(astral);
        self.dest
    }
    #[inline(always)]
    pub fn write_surrogate_pair(self, high: u16, low: u16) -> &'a mut Utf8Destination<'b> {
        self.dest.write_surrogate_pair(high, low);
        self.dest
    }
    #[inline(always)]
    pub fn write_big5_combination(
        self,
        combined: u16,
        combining: u16,
    ) -> &'a mut Utf8Destination<'b> {
        self.dest.write_big5_combination(combined, combining);
        self.dest
    }
    #[inline(always)]
    pub fn commit(self) -> &'a mut Utf8Destination<'b> {
        self.dest
    }
}

pub struct Utf8Destination<'a> {
    slice: &'a mut [u8],
    pos: usize,
}

impl<'a> Utf8Destination<'a> {
    #[inline(always)]
    pub fn new(dst: &mut [u8]) -> Utf8Destination {
        Utf8Destination { slice: dst, pos: 0 }
    }
    #[inline(always)]
    pub fn check_space_bmp<'b>(&'b mut self) -> Space<Utf8BmpHandle<'b, 'a>> {
        if self.pos + 2 < self.slice.len() {
            Space::Available(Utf8BmpHandle::new(self))
        } else {
            Space::Full(self.written())
        }
    }
    #[inline(always)]
    pub fn check_space_astral<'b>(&'b mut self) -> Space<Utf8AstralHandle<'b, 'a>> {
        if self.pos + 3 < self.slice.len() {
            Space::Available(Utf8AstralHandle::new(self))
        } else {
            Space::Full(self.written())
        }
    }
    #[inline(always)]
    pub fn written(&self) -> usize {
        self.pos
    }
    #[inline(always)]
    fn write_code_unit(&mut self, u: u8) {
        unsafe {
            // OK, because we checked before handing out a handle.
            *(self.slice.get_unchecked_mut(self.pos)) = u;
        }
        self.pos += 1;
    }
    #[inline(always)]
    fn write_ascii(&mut self, ascii: u8) {
        debug_assert!(ascii < 0x80);
        self.write_code_unit(ascii);
    }
    #[inline(always)]
    fn write_bmp(&mut self, bmp: u16) {
        if bmp < 0x80u16 {
            self.write_ascii(bmp as u8);
        } else if bmp < 0x800u16 {
            self.write_mid_bmp(bmp);
        } else {
            self.write_upper_bmp(bmp);
        }
    }
    #[inline(always)]
    fn write_mid_bmp(&mut self, mid_bmp: u16) {
        debug_assert!(mid_bmp >= 0x80);
        debug_assert!(mid_bmp < 0x800);
        self.write_code_unit(((mid_bmp >> 6) | 0xC0) as u8);
        self.write_code_unit(((mid_bmp & 0x3F) | 0x80) as u8);
    }
    #[inline(always)]
    fn write_upper_bmp(&mut self, upper_bmp: u16) {
        debug_assert!(upper_bmp >= 0x800);
        self.write_code_unit(((upper_bmp >> 12) | 0xE0) as u8);
        self.write_code_unit((((upper_bmp & 0xFC0) >> 6) | 0x80) as u8);
        self.write_code_unit(((upper_bmp & 0x3F) | 0x80) as u8);
    }
    #[inline(always)]
    fn write_bmp_excl_ascii(&mut self, bmp: u16) {
        if bmp < 0x800u16 {
            self.write_mid_bmp(bmp);
        } else {
            self.write_upper_bmp(bmp);
        }
    }
    #[inline(always)]
    fn write_astral(&mut self, astral: u32) {
        debug_assert!(astral > 0xFFFF);
        debug_assert!(astral <= 0x10_FFFF);
        self.write_code_unit(((astral >> 18) | 0xF0) as u8);
        self.write_code_unit((((astral & 0x3F000) >> 12) | 0x80) as u8);
        self.write_code_unit((((astral & 0xFC0) >> 6) | 0x80) as u8);
        self.write_code_unit(((astral & 0x3F) | 0x80) as u8);
    }
    #[inline(always)]
    pub fn write_surrogate_pair(&mut self, high: u16, low: u16) {
        self.write_astral(
            (u32::from(high) << 10) + u32::from(low)
                - (((0xD800u32 << 10) - 0x10000u32) + 0xDC00u32),
        );
    }
    #[inline(always)]
    fn write_big5_combination(&mut self, combined: u16, combining: u16) {
        self.write_mid_bmp(combined);
        self.write_mid_bmp(combining);
    }
    #[inline(always)]
    pub fn copy_ascii_from_check_space_bmp<'b>(
        &'b mut self,
        source: &mut ByteSource,
    ) -> CopyAsciiResult<(DecoderResult, usize, usize), (u8, Utf8BmpHandle<'b, 'a>)> {
        let non_ascii_ret = {
            let dst_len = self.slice.len();
            let src_remaining = &source.slice[source.pos..];
            let dst_remaining = &mut self.slice[self.pos..];
            let (pending, length) = if dst_remaining.len() < src_remaining.len() {
                (DecoderResult::OutputFull, dst_remaining.len())
            } else {
                (DecoderResult::InputEmpty, src_remaining.len())
            };
            match unsafe {
                ascii_to_ascii(src_remaining.as_ptr(), dst_remaining.as_mut_ptr(), length)
            } {
                None => {
                    source.pos += length;
                    self.pos += length;
                    return CopyAsciiResult::Stop((pending, source.pos, self.pos));
                }
                Some((non_ascii, consumed)) => {
                    source.pos += consumed;
                    self.pos += consumed;
                    if self.pos + 2 < dst_len {
                        source.pos += 1; // +1 for non_ascii
                        non_ascii
                    } else {
                        return CopyAsciiResult::Stop((
                            DecoderResult::OutputFull,
                            source.pos,
                            self.pos,
                        ));
                    }
                }
            }
        };
        CopyAsciiResult::GoOn((non_ascii_ret, Utf8BmpHandle::new(self)))
    }
    #[inline(always)]
    pub fn copy_ascii_from_check_space_astral<'b>(
        &'b mut self,
        source: &mut ByteSource,
    ) -> CopyAsciiResult<(DecoderResult, usize, usize), (u8, Utf8AstralHandle<'b, 'a>)> {
        let non_ascii_ret = {
            let dst_len = self.slice.len();
            let src_remaining = &source.slice[source.pos..];
            let dst_remaining = &mut self.slice[self.pos..];
            let (pending, length) = if dst_remaining.len() < src_remaining.len() {
                (DecoderResult::OutputFull, dst_remaining.len())
            } else {
                (DecoderResult::InputEmpty, src_remaining.len())
            };
            match unsafe {
                ascii_to_ascii(src_remaining.as_ptr(), dst_remaining.as_mut_ptr(), length)
            } {
                None => {
                    source.pos += length;
                    self.pos += length;
                    return CopyAsciiResult::Stop((pending, source.pos, self.pos));
                }
                Some((non_ascii, consumed)) => {
                    source.pos += consumed;
                    self.pos += consumed;
                    if self.pos + 3 < dst_len {
                        source.pos += 1; // +1 for non_ascii
                        non_ascii
                    } else {
                        return CopyAsciiResult::Stop((
                            DecoderResult::OutputFull,
                            source.pos,
                            self.pos,
                        ));
                    }
                }
            }
        };
        CopyAsciiResult::GoOn((non_ascii_ret, Utf8AstralHandle::new(self)))
    }
    #[inline(always)]
    pub fn copy_utf8_up_to_invalid_from(&mut self, source: &mut ByteSource) {
        let src_remaining = &source.slice[source.pos..];
        let dst_remaining = &mut self.slice[self.pos..];
        let min_len = ::std::cmp::min(src_remaining.len(), dst_remaining.len());
        // Validate first, then memcpy to let memcpy do its thing even for
        // non-ASCII. (And potentially do something better than SSE2 for ASCII.)
        let valid_len = utf8_valid_up_to(&src_remaining[..min_len]);
        (&mut dst_remaining[..valid_len]).copy_from_slice(&src_remaining[..valid_len]);
        source.pos += valid_len;
        self.pos += valid_len;
    }
    #[inline(always)]
    pub fn copy_utf16_from<E: Endian>(
        &mut self,
        source: &mut ByteSource,
    ) -> Option<(usize, usize)> {
        let src_remaining = &source.slice[source.pos..];
        let dst_remaining = &mut self.slice[self.pos..];

        let mut src_unaligned =
            unsafe { UnalignedU16Slice::new(src_remaining.as_ptr(), src_remaining.len() / 2) };
        if src_unaligned.len() == 0 {
            return None;
        }
        let mut last_unit = src_unaligned.at(src_unaligned.len() - 1);
        if E::OPPOSITE_ENDIAN {
            last_unit = last_unit.swap_bytes();
        }
        if super::in_range16(last_unit, 0xD800, 0xDC00) {
            // Last code unit is a high surrogate. It might
            // legitimately form a pair later, so let's not
            // include it.
            src_unaligned.trim_last();
        }
        let (read, written, had_error) =
            convert_unaligned_utf16_to_utf8::<E>(src_unaligned, dst_remaining);
        source.pos += read * 2;
        self.pos += written;
        if had_error {
            Some((source.pos, self.pos))
        } else {
            None
        }
    }
}

// UTF-16 source

pub struct Utf16Source<'a> {
    slice: &'a [u16],
    pos: usize,
    old_pos: usize,
}

impl<'a> Utf16Source<'a> {
    #[inline(always)]
    pub fn new(src: &[u16]) -> Utf16Source {
        Utf16Source {
            slice: src,
            pos: 0,
            old_pos: 0,
        }
    }
    #[inline(always)]
    pub fn check_available<'b>(&'b mut self) -> Space<Utf16ReadHandle<'b, 'a>> {
        if self.pos < self.slice.len() {
            Space::Available(Utf16ReadHandle::new(self))
        } else {
            Space::Full(self.consumed())
        }
    }
    #[cfg_attr(feature = "cargo-clippy", allow(collapsible_if))]
    #[inline(always)]
    fn read(&mut self) -> char {
        self.old_pos = self.pos;
        let unit = self.slice[self.pos];
        self.pos += 1;
        let unit_minus_surrogate_start = unit.wrapping_sub(0xD800);
        if unit_minus_surrogate_start > (0xDFFF - 0xD800) {
            return unsafe { ::std::char::from_u32_unchecked(u32::from(unit)) };
        }
        if unit_minus_surrogate_start <= (0xDBFF - 0xD800) {
            // high surrogate
            if self.pos < self.slice.len() {
                let second = self.slice[self.pos];
                let second_minus_low_surrogate_start = second.wrapping_sub(0xDC00);
                if second_minus_low_surrogate_start <= (0xDFFF - 0xDC00) {
                    // The next code unit is a low surrogate. Advance position.
                    self.pos += 1;
                    return unsafe {
                        ::std::char::from_u32_unchecked(
                            (u32::from(unit) << 10) + u32::from(second)
                                - (((0xD800u32 << 10) - 0x10000u32) + 0xDC00u32),
                        )
                    };
                }
                // The next code unit is not a low surrogate. Don't advance
                // position and treat the high surrogate as unpaired.
                // fall through
            }
            // Unpaired surrogate at the end of buffer, fall through
        }
        // Unpaired low surrogate
        '\u{FFFD}'
    }
    #[cfg_attr(feature = "cargo-clippy", allow(collapsible_if))]
    #[inline(always)]
    fn read_enum(&mut self) -> Unicode {
        self.old_pos = self.pos;
        let unit = self.slice[self.pos];
        self.pos += 1;
        if unit < 0x80 {
            return Unicode::Ascii(unit as u8);
        }
        let unit_minus_surrogate_start = unit.wrapping_sub(0xD800);
        if unit_minus_surrogate_start > (0xDFFF - 0xD800) {
            return Unicode::NonAscii(NonAscii::BmpExclAscii(unit));
        }
        if unit_minus_surrogate_start <= (0xDBFF - 0xD800) {
            // high surrogate
            if self.pos < self.slice.len() {
                let second = self.slice[self.pos];
                let second_minus_low_surrogate_start = second.wrapping_sub(0xDC00);
                if second_minus_low_surrogate_start <= (0xDFFF - 0xDC00) {
                    // The next code unit is a low surrogate. Advance position.
                    self.pos += 1;
                    return Unicode::NonAscii(NonAscii::Astral(unsafe {
                        ::std::char::from_u32_unchecked(
                            (u32::from(unit) << 10) + u32::from(second)
                                - (((0xD800u32 << 10) - 0x10000u32) + 0xDC00u32),
                        )
                    }));
                }
                // The next code unit is not a low surrogate. Don't advance
                // position and treat the high surrogate as unpaired.
                // fall through
            }
            // Unpaired surrogate at the end of buffer, fall through
        }
        // Unpaired low surrogate
        Unicode::NonAscii(NonAscii::BmpExclAscii(0xFFFDu16))
    }
    #[inline(always)]
    fn unread(&mut self) -> usize {
        self.pos = self.old_pos;
        self.pos
    }
    #[inline(always)]
    pub fn consumed(&self) -> usize {
        self.pos
    }
    #[inline(always)]
    pub fn copy_ascii_to_check_space_two<'b>(
        &mut self,
        dest: &'b mut ByteDestination<'a>,
    ) -> CopyAsciiResult<(EncoderResult, usize, usize), (NonAscii, ByteTwoHandle<'b, 'a>)> {
        let non_ascii_ret = {
            let dst_len = dest.slice.len();
            let src_remaining = &self.slice[self.pos..];
            let dst_remaining = &mut dest.slice[dest.pos..];
            let (pending, length) = if dst_remaining.len() < src_remaining.len() {
                (EncoderResult::OutputFull, dst_remaining.len())
            } else {
                (EncoderResult::InputEmpty, src_remaining.len())
            };
            match unsafe {
                basic_latin_to_ascii(src_remaining.as_ptr(), dst_remaining.as_mut_ptr(), length)
            } {
                None => {
                    self.pos += length;
                    dest.pos += length;
                    return CopyAsciiResult::Stop((pending, self.pos, dest.pos));
                }
                Some((non_ascii, consumed)) => {
                    self.pos += consumed;
                    dest.pos += consumed;
                    if dest.pos + 1 < dst_len {
                        self.pos += 1; // commit to reading `non_ascii`
                        let unit = non_ascii;
                        let unit_minus_surrogate_start = unit.wrapping_sub(0xD800);
                        if unit_minus_surrogate_start > (0xDFFF - 0xD800) {
                            NonAscii::BmpExclAscii(unit)
                        } else if unit_minus_surrogate_start <= (0xDBFF - 0xD800) {
                            // high surrogate
                            if self.pos < self.slice.len() {
                                let second = self.slice[self.pos];
                                let second_minus_low_surrogate_start = second.wrapping_sub(0xDC00);
                                if second_minus_low_surrogate_start <= (0xDFFF - 0xDC00) {
                                    // The next code unit is a low surrogate. Advance position.
                                    self.pos += 1;
                                    NonAscii::Astral(unsafe {
                                        ::std::char::from_u32_unchecked(
                                            (u32::from(unit) << 10) + u32::from(second)
                                                - (((0xD800u32 << 10) - 0x10000u32) + 0xDC00u32),
                                        )
                                    })
                                } else {
                                    // The next code unit is not a low surrogate. Don't advance
                                    // position and treat the high surrogate as unpaired.
                                    NonAscii::BmpExclAscii(0xFFFDu16)
                                }
                            } else {
                                // Unpaired surrogate at the end of the buffer.
                                NonAscii::BmpExclAscii(0xFFFDu16)
                            }
                        } else {
                            // Unpaired low surrogate
                            NonAscii::BmpExclAscii(0xFFFDu16)
                        }
                    } else {
                        return CopyAsciiResult::Stop((
                            EncoderResult::OutputFull,
                            self.pos,
                            dest.pos,
                        ));
                    }
                }
            }
        };
        CopyAsciiResult::GoOn((non_ascii_ret, ByteTwoHandle::new(dest)))
    }
    #[inline(always)]
    pub fn copy_ascii_to_check_space_four<'b>(
        &mut self,
        dest: &'b mut ByteDestination<'a>,
    ) -> CopyAsciiResult<(EncoderResult, usize, usize), (NonAscii, ByteFourHandle<'b, 'a>)> {
        let non_ascii_ret = {
            let dst_len = dest.slice.len();
            let src_remaining = &self.slice[self.pos..];
            let dst_remaining = &mut dest.slice[dest.pos..];
            let (pending, length) = if dst_remaining.len() < src_remaining.len() {
                (EncoderResult::OutputFull, dst_remaining.len())
            } else {
                (EncoderResult::InputEmpty, src_remaining.len())
            };
            match unsafe {
                basic_latin_to_ascii(src_remaining.as_ptr(), dst_remaining.as_mut_ptr(), length)
            } {
                None => {
                    self.pos += length;
                    dest.pos += length;
                    return CopyAsciiResult::Stop((pending, self.pos, dest.pos));
                }
                Some((non_ascii, consumed)) => {
                    self.pos += consumed;
                    dest.pos += consumed;
                    if dest.pos + 3 < dst_len {
                        self.pos += 1; // commit to reading `non_ascii`
                        let unit = non_ascii;
                        let unit_minus_surrogate_start = unit.wrapping_sub(0xD800);
                        if unit_minus_surrogate_start > (0xDFFF - 0xD800) {
                            NonAscii::BmpExclAscii(unit)
                        } else if unit_minus_surrogate_start <= (0xDBFF - 0xD800) {
                            // high surrogate
                            if self.pos == self.slice.len() {
                                // Unpaired surrogate at the end of the buffer.
                                NonAscii::BmpExclAscii(0xFFFDu16)
                            } else {
                                let second = self.slice[self.pos];
                                let second_minus_low_surrogate_start = second.wrapping_sub(0xDC00);
                                if second_minus_low_surrogate_start <= (0xDFFF - 0xDC00) {
                                    // The next code unit is a low surrogate. Advance position.
                                    self.pos += 1;
                                    NonAscii::Astral(unsafe {
                                        ::std::char::from_u32_unchecked(
                                            (u32::from(unit) << 10) + u32::from(second)
                                                - (((0xD800u32 << 10) - 0x1_0000u32) + 0xDC00u32),
                                        )
                                    })
                                } else {
                                    // The next code unit is not a low surrogate. Don't advance
                                    // position and treat the high surrogate as unpaired.
                                    NonAscii::BmpExclAscii(0xFFFDu16)
                                }
                            }
                        } else {
                            // Unpaired low surrogate
                            NonAscii::BmpExclAscii(0xFFFDu16)
                        }
                    } else {
                        return CopyAsciiResult::Stop((
                            EncoderResult::OutputFull,
                            self.pos,
                            dest.pos,
                        ));
                    }
                }
            }
        };
        CopyAsciiResult::GoOn((non_ascii_ret, ByteFourHandle::new(dest)))
    }
}

pub struct Utf16ReadHandle<'a, 'b>
where
    'b: 'a,
{
    source: &'a mut Utf16Source<'b>,
}

impl<'a, 'b> Utf16ReadHandle<'a, 'b>
where
    'b: 'a,
{
    #[inline(always)]
    fn new(src: &'a mut Utf16Source<'b>) -> Utf16ReadHandle<'a, 'b> {
        Utf16ReadHandle { source: src }
    }
    #[inline(always)]
    pub fn read(self) -> (char, Utf16UnreadHandle<'a, 'b>) {
        let character = self.source.read();
        let handle = Utf16UnreadHandle::new(self.source);
        (character, handle)
    }
    #[inline(always)]
    pub fn read_enum(self) -> (Unicode, Utf16UnreadHandle<'a, 'b>) {
        let character = self.source.read_enum();
        let handle = Utf16UnreadHandle::new(self.source);
        (character, handle)
    }
    #[inline(always)]
    pub fn consumed(&self) -> usize {
        self.source.consumed()
    }
}

pub struct Utf16UnreadHandle<'a, 'b>
where
    'b: 'a,
{
    source: &'a mut Utf16Source<'b>,
}

impl<'a, 'b> Utf16UnreadHandle<'a, 'b>
where
    'b: 'a,
{
    #[inline(always)]
    fn new(src: &'a mut Utf16Source<'b>) -> Utf16UnreadHandle<'a, 'b> {
        Utf16UnreadHandle { source: src }
    }
    #[inline(always)]
    pub fn unread(self) -> usize {
        self.source.unread()
    }
    #[inline(always)]
    pub fn consumed(&self) -> usize {
        self.source.consumed()
    }
    #[inline(always)]
    pub fn commit(self) -> &'a mut Utf16Source<'b> {
        self.source
    }
}

// UTF-8 source

pub struct Utf8Source<'a> {
    slice: &'a [u8],
    pos: usize,
    old_pos: usize,
}

impl<'a> Utf8Source<'a> {
    #[inline(always)]
    pub fn new(src: &str) -> Utf8Source {
        Utf8Source {
            slice: src.as_bytes(),
            pos: 0,
            old_pos: 0,
        }
    }
    #[inline(always)]
    pub fn check_available<'b>(&'b mut self) -> Space<Utf8ReadHandle<'b, 'a>> {
        if self.pos < self.slice.len() {
            Space::Available(Utf8ReadHandle::new(self))
        } else {
            Space::Full(self.consumed())
        }
    }
    #[inline(always)]
    fn read(&mut self) -> char {
        self.old_pos = self.pos;
        let unit = self.slice[self.pos];
        if unit < 0x80 {
            self.pos += 1;
            return char::from(unit);
        }
        if unit < 0xE0 {
            let point =
                ((u32::from(unit) & 0x1F) << 6) | (u32::from(self.slice[self.pos + 1]) & 0x3F);
            self.pos += 2;
            return unsafe { ::std::char::from_u32_unchecked(point) };
        }
        if unit < 0xF0 {
            let point = ((u32::from(unit) & 0xF) << 12)
                | ((u32::from(self.slice[self.pos + 1]) & 0x3F) << 6)
                | (u32::from(self.slice[self.pos + 2]) & 0x3F);
            self.pos += 3;
            return unsafe { ::std::char::from_u32_unchecked(point) };
        }
        let point = ((u32::from(unit) & 0x7) << 18)
            | ((u32::from(self.slice[self.pos + 1]) & 0x3F) << 12)
            | ((u32::from(self.slice[self.pos + 2]) & 0x3F) << 6)
            | (u32::from(self.slice[self.pos + 3]) & 0x3F);
        self.pos += 4;
        unsafe { ::std::char::from_u32_unchecked(point) }
    }
    #[inline(always)]
    fn read_enum(&mut self) -> Unicode {
        self.old_pos = self.pos;
        let unit = self.slice[self.pos];
        if unit < 0x80 {
            self.pos += 1;
            return Unicode::Ascii(unit);
        }
        if unit < 0xE0 {
            let point =
                ((u16::from(unit) & 0x1F) << 6) | (u16::from(self.slice[self.pos + 1]) & 0x3F);
            self.pos += 2;
            return Unicode::NonAscii(NonAscii::BmpExclAscii(point));
        }
        if unit < 0xF0 {
            let point = ((u16::from(unit) & 0xF) << 12)
                | ((u16::from(self.slice[self.pos + 1]) & 0x3F) << 6)
                | (u16::from(self.slice[self.pos + 2]) & 0x3F);
            self.pos += 3;
            return Unicode::NonAscii(NonAscii::BmpExclAscii(point));
        }
        let point = ((u32::from(unit) & 0x7) << 18)
            | ((u32::from(self.slice[self.pos + 1]) & 0x3F) << 12)
            | ((u32::from(self.slice[self.pos + 2]) & 0x3F) << 6)
            | (u32::from(self.slice[self.pos + 3]) & 0x3F);
        self.pos += 4;
        Unicode::NonAscii(NonAscii::Astral(unsafe {
            ::std::char::from_u32_unchecked(point)
        }))
    }
    #[inline(always)]
    fn unread(&mut self) -> usize {
        self.pos = self.old_pos;
        self.pos
    }
    #[inline(always)]
    pub fn consumed(&self) -> usize {
        self.pos
    }
    #[inline(always)]
    pub fn copy_ascii_to_check_space_one<'b>(
        &mut self,
        dest: &'b mut ByteDestination<'a>,
    ) -> CopyAsciiResult<(EncoderResult, usize, usize), (NonAscii, ByteOneHandle<'b, 'a>)> {
        let non_ascii_ret = {
            let src_remaining = &self.slice[self.pos..];
            let dst_remaining = &mut dest.slice[dest.pos..];
            let (pending, length) = if dst_remaining.len() < src_remaining.len() {
                (EncoderResult::OutputFull, dst_remaining.len())
            } else {
                (EncoderResult::InputEmpty, src_remaining.len())
            };
            match unsafe {
                ascii_to_ascii(src_remaining.as_ptr(), dst_remaining.as_mut_ptr(), length)
            } {
                None => {
                    self.pos += length;
                    dest.pos += length;
                    return CopyAsciiResult::Stop((pending, self.pos, dest.pos));
                }
                Some((non_ascii, consumed)) => {
                    self.pos += consumed;
                    dest.pos += consumed;
                    // We don't need to check space in destination, because
                    // `ascii_to_ascii()` already did.
                    if non_ascii < 0xE0 {
                        let point = ((u16::from(non_ascii) & 0x1F) << 6)
                            | (u16::from(self.slice[self.pos + 1]) & 0x3F);
                        self.pos += 2;
                        NonAscii::BmpExclAscii(point)
                    } else if non_ascii < 0xF0 {
                        let point = ((u16::from(non_ascii) & 0xF) << 12)
                            | ((u16::from(self.slice[self.pos + 1]) & 0x3F) << 6)
                            | (u16::from(self.slice[self.pos + 2]) & 0x3F);
                        self.pos += 3;
                        NonAscii::BmpExclAscii(point)
                    } else {
                        let point = ((u32::from(non_ascii) & 0x7) << 18)
                            | ((u32::from(self.slice[self.pos + 1]) & 0x3F) << 12)
                            | ((u32::from(self.slice[self.pos + 2]) & 0x3F) << 6)
                            | (u32::from(self.slice[self.pos + 3]) & 0x3F);
                        self.pos += 4;
                        NonAscii::Astral(unsafe { ::std::char::from_u32_unchecked(point) })
                    }
                }
            }
        };
        CopyAsciiResult::GoOn((non_ascii_ret, ByteOneHandle::new(dest)))
    }
    #[inline(always)]
    pub fn copy_ascii_to_check_space_two<'b>(
        &mut self,
        dest: &'b mut ByteDestination<'a>,
    ) -> CopyAsciiResult<(EncoderResult, usize, usize), (NonAscii, ByteTwoHandle<'b, 'a>)> {
        let non_ascii_ret = {
            let dst_len = dest.slice.len();
            let src_remaining = &self.slice[self.pos..];
            let dst_remaining = &mut dest.slice[dest.pos..];
            let (pending, length) = if dst_remaining.len() < src_remaining.len() {
                (EncoderResult::OutputFull, dst_remaining.len())
            } else {
                (EncoderResult::InputEmpty, src_remaining.len())
            };
            match unsafe {
                ascii_to_ascii(src_remaining.as_ptr(), dst_remaining.as_mut_ptr(), length)
            } {
                None => {
                    self.pos += length;
                    dest.pos += length;
                    return CopyAsciiResult::Stop((pending, self.pos, dest.pos));
                }
                Some((non_ascii, consumed)) => {
                    self.pos += consumed;
                    dest.pos += consumed;
                    if dest.pos + 1 < dst_len {
                        if non_ascii < 0xE0 {
                            let point = ((u16::from(non_ascii) & 0x1F) << 6)
                                | (u16::from(self.slice[self.pos + 1]) & 0x3F);
                            self.pos += 2;
                            NonAscii::BmpExclAscii(point)
                        } else if non_ascii < 0xF0 {
                            let point = ((u16::from(non_ascii) & 0xF) << 12)
                                | ((u16::from(self.slice[self.pos + 1]) & 0x3F) << 6)
                                | (u16::from(self.slice[self.pos + 2]) & 0x3F);
                            self.pos += 3;
                            NonAscii::BmpExclAscii(point)
                        } else {
                            let point = ((u32::from(non_ascii) & 0x7) << 18)
                                | ((u32::from(self.slice[self.pos + 1]) & 0x3F) << 12)
                                | ((u32::from(self.slice[self.pos + 2]) & 0x3F) << 6)
                                | (u32::from(self.slice[self.pos + 3]) & 0x3F);
                            self.pos += 4;
                            NonAscii::Astral(unsafe { ::std::char::from_u32_unchecked(point) })
                        }
                    } else {
                        return CopyAsciiResult::Stop((
                            EncoderResult::OutputFull,
                            self.pos,
                            dest.pos,
                        ));
                    }
                }
            }
        };
        CopyAsciiResult::GoOn((non_ascii_ret, ByteTwoHandle::new(dest)))
    }
    #[inline(always)]
    pub fn copy_ascii_to_check_space_four<'b>(
        &mut self,
        dest: &'b mut ByteDestination<'a>,
    ) -> CopyAsciiResult<(EncoderResult, usize, usize), (NonAscii, ByteFourHandle<'b, 'a>)> {
        let non_ascii_ret = {
            let dst_len = dest.slice.len();
            let src_remaining = &self.slice[self.pos..];
            let dst_remaining = &mut dest.slice[dest.pos..];
            let (pending, length) = if dst_remaining.len() < src_remaining.len() {
                (EncoderResult::OutputFull, dst_remaining.len())
            } else {
                (EncoderResult::InputEmpty, src_remaining.len())
            };
            match unsafe {
                ascii_to_ascii(src_remaining.as_ptr(), dst_remaining.as_mut_ptr(), length)
            } {
                None => {
                    self.pos += length;
                    dest.pos += length;
                    return CopyAsciiResult::Stop((pending, self.pos, dest.pos));
                }
                Some((non_ascii, consumed)) => {
                    self.pos += consumed;
                    dest.pos += consumed;
                    if dest.pos + 3 < dst_len {
                        if non_ascii < 0xE0 {
                            let point = ((u16::from(non_ascii) & 0x1F) << 6)
                                | (u16::from(self.slice[self.pos + 1]) & 0x3F);
                            self.pos += 2;
                            NonAscii::BmpExclAscii(point)
                        } else if non_ascii < 0xF0 {
                            let point = ((u16::from(non_ascii) & 0xF) << 12)
                                | ((u16::from(self.slice[self.pos + 1]) & 0x3F) << 6)
                                | (u16::from(self.slice[self.pos + 2]) & 0x3F);
                            self.pos += 3;
                            NonAscii::BmpExclAscii(point)
                        } else {
                            let point = ((u32::from(non_ascii) & 0x7) << 18)
                                | ((u32::from(self.slice[self.pos + 1]) & 0x3F) << 12)
                                | ((u32::from(self.slice[self.pos + 2]) & 0x3F) << 6)
                                | (u32::from(self.slice[self.pos + 3]) & 0x3F);
                            self.pos += 4;
                            NonAscii::Astral(unsafe { ::std::char::from_u32_unchecked(point) })
                        }
                    } else {
                        return CopyAsciiResult::Stop((
                            EncoderResult::OutputFull,
                            self.pos,
                            dest.pos,
                        ));
                    }
                }
            }
        };
        CopyAsciiResult::GoOn((non_ascii_ret, ByteFourHandle::new(dest)))
    }
}

pub struct Utf8ReadHandle<'a, 'b>
where
    'b: 'a,
{
    source: &'a mut Utf8Source<'b>,
}

impl<'a, 'b> Utf8ReadHandle<'a, 'b>
where
    'b: 'a,
{
    #[inline(always)]
    fn new(src: &'a mut Utf8Source<'b>) -> Utf8ReadHandle<'a, 'b> {
        Utf8ReadHandle { source: src }
    }
    #[inline(always)]
    pub fn read(self) -> (char, Utf8UnreadHandle<'a, 'b>) {
        let character = self.source.read();
        let handle = Utf8UnreadHandle::new(self.source);
        (character, handle)
    }
    #[inline(always)]
    pub fn read_enum(self) -> (Unicode, Utf8UnreadHandle<'a, 'b>) {
        let character = self.source.read_enum();
        let handle = Utf8UnreadHandle::new(self.source);
        (character, handle)
    }
    #[inline(always)]
    pub fn consumed(&self) -> usize {
        self.source.consumed()
    }
}

pub struct Utf8UnreadHandle<'a, 'b>
where
    'b: 'a,
{
    source: &'a mut Utf8Source<'b>,
}

impl<'a, 'b> Utf8UnreadHandle<'a, 'b>
where
    'b: 'a,
{
    #[inline(always)]
    fn new(src: &'a mut Utf8Source<'b>) -> Utf8UnreadHandle<'a, 'b> {
        Utf8UnreadHandle { source: src }
    }
    #[inline(always)]
    pub fn unread(self) -> usize {
        self.source.unread()
    }
    #[inline(always)]
    pub fn consumed(&self) -> usize {
        self.source.consumed()
    }
    #[inline(always)]
    pub fn commit(self) -> &'a mut Utf8Source<'b> {
        self.source
    }
}

// Byte destination

pub struct ByteOneHandle<'a, 'b>
where
    'b: 'a,
{
    dest: &'a mut ByteDestination<'b>,
}

impl<'a, 'b> ByteOneHandle<'a, 'b>
where
    'b: 'a,
{
    #[inline(always)]
    fn new(dst: &'a mut ByteDestination<'b>) -> ByteOneHandle<'a, 'b> {
        ByteOneHandle { dest: dst }
    }
    #[inline(always)]
    pub fn written(&self) -> usize {
        self.dest.written()
    }
    #[inline(always)]
    pub fn write_one(self, first: u8) -> &'a mut ByteDestination<'b> {
        self.dest.write_one(first);
        self.dest
    }
}

pub struct ByteTwoHandle<'a, 'b>
where
    'b: 'a,
{
    dest: &'a mut ByteDestination<'b>,
}

impl<'a, 'b> ByteTwoHandle<'a, 'b>
where
    'b: 'a,
{
    #[inline(always)]
    fn new(dst: &'a mut ByteDestination<'b>) -> ByteTwoHandle<'a, 'b> {
        ByteTwoHandle { dest: dst }
    }
    #[inline(always)]
    pub fn written(&self) -> usize {
        self.dest.written()
    }
    #[inline(always)]
    pub fn write_one(self, first: u8) -> &'a mut ByteDestination<'b> {
        self.dest.write_one(first);
        self.dest
    }
    #[inline(always)]
    pub fn write_two(self, first: u8, second: u8) -> &'a mut ByteDestination<'b> {
        self.dest.write_two(first, second);
        self.dest
    }
}

pub struct ByteThreeHandle<'a, 'b>
where
    'b: 'a,
{
    dest: &'a mut ByteDestination<'b>,
}

impl<'a, 'b> ByteThreeHandle<'a, 'b>
where
    'b: 'a,
{
    #[inline(always)]
    fn new(dst: &'a mut ByteDestination<'b>) -> ByteThreeHandle<'a, 'b> {
        ByteThreeHandle { dest: dst }
    }
    #[inline(always)]
    pub fn written(&self) -> usize {
        self.dest.written()
    }
    #[inline(always)]
    pub fn write_one(self, first: u8) -> &'a mut ByteDestination<'b> {
        self.dest.write_one(first);
        self.dest
    }
    #[inline(always)]
    pub fn write_two(self, first: u8, second: u8) -> &'a mut ByteDestination<'b> {
        self.dest.write_two(first, second);
        self.dest
    }
    #[inline(always)]
    pub fn write_three(self, first: u8, second: u8, third: u8) -> &'a mut ByteDestination<'b> {
        self.dest.write_three(first, second, third);
        self.dest
    }
    #[inline(always)]
    pub fn write_three_return_written(self, first: u8, second: u8, third: u8) -> usize {
        self.dest.write_three(first, second, third);
        self.dest.written()
    }
}

pub struct ByteFourHandle<'a, 'b>
where
    'b: 'a,
{
    dest: &'a mut ByteDestination<'b>,
}

impl<'a, 'b> ByteFourHandle<'a, 'b>
where
    'b: 'a,
{
    #[inline(always)]
    fn new(dst: &'a mut ByteDestination<'b>) -> ByteFourHandle<'a, 'b> {
        ByteFourHandle { dest: dst }
    }
    #[inline(always)]
    pub fn written(&self) -> usize {
        self.dest.written()
    }
    #[inline(always)]
    pub fn write_one(self, first: u8) -> &'a mut ByteDestination<'b> {
        self.dest.write_one(first);
        self.dest
    }
    #[inline(always)]
    pub fn write_two(self, first: u8, second: u8) -> &'a mut ByteDestination<'b> {
        self.dest.write_two(first, second);
        self.dest
    }
    #[inline(always)]
    pub fn write_four(
        self,
        first: u8,
        second: u8,
        third: u8,
        fourth: u8,
    ) -> &'a mut ByteDestination<'b> {
        self.dest.write_four(first, second, third, fourth);
        self.dest
    }
}

pub struct ByteDestination<'a> {
    slice: &'a mut [u8],
    pos: usize,
}

impl<'a> ByteDestination<'a> {
    #[inline(always)]
    pub fn new(dst: &mut [u8]) -> ByteDestination {
        ByteDestination { slice: dst, pos: 0 }
    }
    #[inline(always)]
    pub fn check_space_one<'b>(&'b mut self) -> Space<ByteOneHandle<'b, 'a>> {
        if self.pos < self.slice.len() {
            Space::Available(ByteOneHandle::new(self))
        } else {
            Space::Full(self.written())
        }
    }
    #[inline(always)]
    pub fn check_space_two<'b>(&'b mut self) -> Space<ByteTwoHandle<'b, 'a>> {
        if self.pos + 1 < self.slice.len() {
            Space::Available(ByteTwoHandle::new(self))
        } else {
            Space::Full(self.written())
        }
    }
    #[inline(always)]
    pub fn check_space_three<'b>(&'b mut self) -> Space<ByteThreeHandle<'b, 'a>> {
        if self.pos + 2 < self.slice.len() {
            Space::Available(ByteThreeHandle::new(self))
        } else {
            Space::Full(self.written())
        }
    }
    #[inline(always)]
    pub fn check_space_four<'b>(&'b mut self) -> Space<ByteFourHandle<'b, 'a>> {
        if self.pos + 3 < self.slice.len() {
            Space::Available(ByteFourHandle::new(self))
        } else {
            Space::Full(self.written())
        }
    }
    #[inline(always)]
    pub fn written(&self) -> usize {
        self.pos
    }
    #[inline(always)]
    fn write_one(&mut self, first: u8) {
        self.slice[self.pos] = first;
        self.pos += 1;
    }
    #[inline(always)]
    fn write_two(&mut self, first: u8, second: u8) {
        self.slice[self.pos] = first;
        self.slice[self.pos + 1] = second;
        self.pos += 2;
    }
    #[inline(always)]
    fn write_three(&mut self, first: u8, second: u8, third: u8) {
        self.slice[self.pos] = first;
        self.slice[self.pos + 1] = second;
        self.slice[self.pos + 2] = third;
        self.pos += 3;
    }
    #[inline(always)]
    fn write_four(&mut self, first: u8, second: u8, third: u8, fourth: u8) {
        self.slice[self.pos] = first;
        self.slice[self.pos + 1] = second;
        self.slice[self.pos + 2] = third;
        self.slice[self.pos + 3] = fourth;
        self.pos += 4;
    }
}
#[cfg(test)]
mod tests_rug_138 {
    use super::*;
    use crate::handles::{UnalignedU16Slice, CopyAsciiResult, Endian, swap_if_opposite_endian};

    #[test]
    fn test_rug() {
        let data: [u8; 8] = [0xAB, 0xCD, 0xEF, 0x12, 0x34, 0x56, 0x78, 0x90];
        let v16 = unsafe {
            UnalignedU16Slice::new(data.as_ptr(), data.len() / 2)
        };

        let mut dst: [u8; 4] = [0; 4];
        let offset: usize = 2;

        copy_unaligned_basic_latin_to_ascii_alu::<LittleEndian>(v16, &mut dst[..], offset);
    }
}
#[cfg(test)]
mod tests_rug_143 {
    use crate::handles::UnalignedU16Slice;
    use super::*;

    #[test]
    fn test_rug() {
        let mut data: [u8; 8] = [0xAB, 0xCD, 0xEF, 0x12, 0x34, 0x56, 0x78, 0x90];
        let mut p0 = unsafe {
            UnalignedU16Slice::new(data.as_ptr(), data.len() / 2)
        };

        <UnalignedU16Slice>::trim_last(&mut p0);

        assert_eq!(p0.len(), 3);
    }
}#[cfg(test)]
mod tests_rug_144 {
    use super::*;
    use crate::handles;
    
    #[test]
    fn test_rug() {
        // Create a sample data buffer
        let data: [u8; 8] = [0xAB, 0xCD, 0xEF, 0x12, 0x34, 0x56, 0x78, 0x90];
        
        // Create an initialized local variable v16 with type handles::UnalignedU16Slice
        let v16 = unsafe {
            handles::UnalignedU16Slice::new(data.as_ptr(), data.len() / 2)
        };
        
        let p0 = v16;
        let p1: usize = 2;

        let result = p0.at(p1);
    }
}#[cfg(test)]
mod tests_rug_145 {
    use super::*;
    use crate::handles;

    #[test]
    fn test_rug() {
        let mut p0: handles::UnalignedU16Slice;
        let data: [u8; 8] = [0xAB, 0xCD, 0xEF, 0x12, 0x34, 0x56, 0x78, 0x90];
        let v16 = unsafe {
            handles::UnalignedU16Slice::new(data.as_ptr(), data.len() / 2)
        };
        
        p0 = v16;

        handles::UnalignedU16Slice::len(&p0);

    }
}#[cfg(test)]
mod tests_rug_146 {
    use crate::handles::UnalignedU16Slice;

    #[test]
    fn test_tail() {
        // Sample data buffer
        let data: [u8; 8] = [0xAB, 0xCD, 0xEF, 0x12, 0x34, 0x56, 0x78, 0x90];
        
        // Initialized local variable v16 with type handles::UnalignedU16Slice
        let v16 = unsafe {
            UnalignedU16Slice::new(data.as_ptr(), data.len() / 2)
        };
        
        // Initializing usize variable for 'from' parameter
        let from: usize = 2;

        // Call the tail function with the constructed parameters     
        let result = v16.tail(from);

        // Add your assertions here
    }
}        #[cfg(test)]
        mod tests_rug_147 {
            use super::*;
            use crate::handles;

            #[test]
            fn test_rug() {
                // Prepare the first argument
                let data: [u8; 8] = [0xAB, 0xCD, 0xEF, 0x12, 0x34, 0x56, 0x78, 0x90];
                let v16 = unsafe {
                    handles::UnalignedU16Slice::new(data.as_ptr(), data.len() / 2)
                };

                // Prepare the second argument
                let mut target: [u16; 4] = [0; 4];

                let mut p1 = &mut target;
                
                crate::handles::UnalignedU16Slice::copy_bmp_to::<LittleEndian>(&v16, p1);

            }
        }#[cfg(test)]
mod tests_rug_148 {
    use super::*;
    use crate::handles::ByteSource;

    #[test]
    fn test_new_byte_source() {
        let p0: &[u8] = b"hello world";

        ByteSource::new(p0);

        // Add assertions or further tests if needed
    }
}#[cfg(test)]
mod tests_rug_150 {
    use super::*;
    use crate::handles::ByteSource;

    #[test]
    fn test_read() {
        let slice: &[u8] = &[1, 2, 3, 4, 5];
        let mut bs = ByteSource::<'_>::new(slice);

        let result = bs.read();
        assert_eq!(result, 1);
    }
}#[cfg(test)]
mod tests_rug_151 {
    use super::*;
    use crate::handles;

    #[test]
    fn test_rug() {
        let mut buffer = [0u8, 1u8, 2u8, 3u8];
        let mut p0 = handles::ByteSource::new(&buffer);

        handles::ByteSource::unread(&mut p0);
        assert_eq!(p0.pos, 3);
    }
}
#[cfg(test)]
mod tests_rug_153 {
    use super::*;
    use crate::handles::{ByteReadHandle, ByteSource};

    #[test]
    fn test_byte_read_handle_new() {
        let mut p0: ByteSource<'static> = ByteSource::new(b"test data");
        
        ByteReadHandle::new(&mut p0);
    }
}
#[cfg(test)]
mod tests_rug_154 {
    use super::*;
    use crate::handles::{ByteReadHandle, ByteUnreadHandle};

    #[test]
    fn test_rug() {
        let mut p0: ByteReadHandle<'_, '_> = unimplemented!();
        p0.read();
    }
}#[cfg(test)]
mod tests_rug_156 {
    use super::*;
    use crate::handles::{ByteSource, ByteUnreadHandle};

    #[test]
    fn test_handle_new() {
        let mut source: ByteSource<'static> = ByteSource::new(b"test data");

        ByteUnreadHandle::new(&mut source);
    }
}#[cfg(test)]
mod tests_rug_158 {
    use super::*;
    use crate::handles;

    #[test]
    fn test_consumed() {
        let mut p0: handles::ByteUnreadHandle<'_, '_> = todo!();

        p0.consumed();
    }
}#[cfg(test)]
mod tests_rug_161 {
    use super::*;
    use crate::handles::Utf16BmpHandle;
    
    #[test]
    fn test_written() {
        let mut p0: Utf16BmpHandle = unimplemented!();
        
        Utf16BmpHandle::written(&p0);
    }
}#[cfg(test)]
mod tests_rug_162 {
    use super::*;
    use crate::handles::Utf16BmpHandle;
    
    #[test]
    fn test_rug() {
        let mut p0: Utf16BmpHandle<'static, 'static> = unimplemented!();
        let p1: u8 = b'A';

        p0.write_ascii(p1);
    }
}#[cfg(test)]
mod tests_rug_164 {
    use super::*;
    use crate::handles::{Utf16BmpHandle, Utf16Destination};

    #[test]
    fn test_rug() {
        let mut p0: Utf16BmpHandle<'_, '_> = unimplemented!();
        let p1: u16 = 65;

        p0.write_bmp_excl_ascii(p1);
    }
}#[cfg(test)]
mod tests_rug_165 {
    use super::*;
    use crate::handles::Utf16BmpHandle;
    
    #[test]
    fn test_rug() {
        let mut p0: Utf16BmpHandle = unimplemented!();
        let p1: u16 = 0xABCD;
        
        p0.write_mid_bmp(p1);
    }
}#[cfg(test)]
mod tests_rug_167 {
    use super::*;
    use crate::handles::{Utf16BmpHandle, Utf16Destination};

    #[test]
    fn test_rug() {
        let mut p0: Utf16BmpHandle<'static, 'static> = unimplemented!();

        p0.commit();
    }
}#[cfg(test)]
mod tests_rug_171 {
    use super::*;
    use crate::handles::{Utf16AstralHandle, Utf16Destination};

    #[test]
    fn test_rug() {
        let mut p0: Utf16AstralHandle<'_, '_> = unimplemented!(); // Construct Utf16AstralHandle<'a, 'b> based on the given description
        let mut p1: u16 = 42; // Sample data to initialize u16
        
        p0.write_bmp(p1);

    }
}#[cfg(test)]
mod tests_rug_172 {
    use super::*;
    use crate::handles::{Utf16AstralHandle, Utf16Destination};

    #[test]
    fn test_rug() {
        let mut p0: Utf16AstralHandle<'_, '_> = unimplemented!();
        let p1: u16 = 42;

        p0.write_bmp_excl_ascii(p1);

    }
}#[cfg(test)]
mod tests_rug_173 {
    use super::*;
    use crate::handles::Utf16AstralHandle;
    use crate::handles::Utf16Destination;

    #[test]
    fn test_rug() {
        let mut p0: Utf16AstralHandle = unimplemented!("construct p0 Utf16AstralHandle");
        let p1: u16 = 0xABCD;

        p0.write_upper_bmp(p1);
    }
}#[cfg(test)]
mod tests_rug_178 {
    use super::*;
    use crate::handles::Utf16Destination;

    #[test]
    fn test_new() {
        let mut buffer: [u16; 5] = [0; 5];

        Utf16Destination::new(&mut buffer);
    }
}#[cfg(test)]
mod tests_rug_182 {
    use super::*;
    use crate::handles;

    #[test]
    fn test_rug() {
        let mut p0: handles::Utf16Destination = unimplemented!();
        let p1: u16 = 0xABCD;

        p0.write_code_unit(p1);
    }
}#[cfg(test)]
mod tests_rug_185 {
    use super::*;
    use crate::handles::Utf16Destination;

    #[test]
    fn test_rug() {
        let mut p0: Utf16Destination = todo!(); 
        let mut p1: u16 = 0x80;

        p0.write_bmp_excl_ascii(p1);
    }
}#[cfg(test)]
mod tests_rug_188 {
    use super::*;
    use crate::handles;

    #[test]
    fn test_rug() {
        let mut p0 : handles::Utf16Destination<'_> = unimplemented!();
        let mut p1 : u32 = 0x10FFFF;

        p0.write_astral(p1);
    }
}#[cfg(test)]
mod tests_rug_189 {
    use super::*;

    use crate::handles::Utf16Destination;

    #[test]
    fn test_rug() {
        let mut p0: Utf16Destination = todo!("Initialize Utf16Destination instance");

        let p1: u16 = 0xD83D;
        let p2: u16 = 0xDE07;

        p0.write_surrogate_pair(p1, p2);
    }
}#[cfg(test)]
mod tests_rug_190 {
    use super::*;
    use crate::handles::Utf16Destination;

    #[test]
    fn test_rug() {
        let mut p0: Utf16Destination = todo!(); // Construct p0 based on the description
        let p1: u16 = 0xABCD; // Sample data for the 2nd argument
        let p2: u16 = 0xDCBA; // Sample data for the 3rd argument

        p0.write_big5_combination(p1, p2);
    }
}#[cfg(test)]
mod tests_rug_191 {
    use super::*;
    use crate::handles::{Utf16Destination, ByteSource};

    #[test]
    fn test_rug() {
        let mut p0: Utf16Destination = unimplemented!(); // Construct Utf16Destination<'a> based on description
        let mut p1: ByteSource = unimplemented!(); // Construct ByteSource<'_> based on description

        p0.copy_ascii_from_check_space_bmp(&mut p1);

    }
}#[cfg(test)]
mod tests_rug_192 {
    use super::*;
    use crate::handles::{Utf16Destination, ByteSource, Utf16AstralHandle};
    
    #[test]
    fn test_rug() {
        let mut p0: Utf16Destination = unimplemented!(); // Construct the Utf16Destination variable
        let mut p1: ByteSource<'_> = unimplemented!(); // Construct the ByteSource variable
        
        p0.copy_ascii_from_check_space_astral(&mut p1);
    }
}#[cfg(test)]
mod tests_rug_194 {
    use super::*;
    use crate::handles::{Utf16Destination, ByteSource};

    #[test]
    fn test_rug() {
        let mut p0: Utf16Destination<'_> = Utf16Destination { slice: &mut [], pos: 0 };
        let mut p1: ByteSource<'_> = ByteSource { slice: &mut [], pos: 0 };

        p0.copy_utf16_from::<LittleEndian>(&mut p1);
    }
}#[cfg(test)]
mod tests_rug_199 {
    use super::*;
    use crate::handles::Utf8BmpHandle;
    
    #[test]
    fn test_rug() {
        let mut p0: Utf8BmpHandle<'_, '_> = unimplemented!();
        let p1: u16 = 65;
        
        p0.write_bmp_excl_ascii(p1);
    }
}#[cfg(test)]
mod tests_rug_205 {
    use super::*;
    use crate::handles::{Utf8AstralHandle, Utf8Destination};

    #[test]
    fn test_rug() {
        let mut p0: Utf8AstralHandle<'static, 'static> = unimplemented!();
        let p1: u8 = 65;

        p0.write_ascii(p1);

        // Add assertions here if needed
    }
}#[cfg(test)]
mod tests_rug_211 {
    use super::*;
    use crate::handles::{Utf8Destination, Utf8AstralHandle};
    
    #[test]
    fn test_rug() {
        let mut p0: Utf8AstralHandle = unimplemented!(); // Replace unimplemented!() with actual initialization
        let p1: u16 = 42;
        let p2: u16 = 23;

        p0.write_big5_combination(p1, p2);
    }
}#[cfg(test)]
mod tests_rug_213 {
    use super::*;
    use crate::handles::Utf8Destination;

    #[test]
    fn test_rug() {
        let mut p0: &mut [u8] = &mut [0; 10];

        let _ = Utf8Destination::new(p0);
    }
}#[cfg(test)]
mod tests_rug_217 {
    use super::*;
    use crate::handles::Utf8Destination;

    #[test]
    fn test_rug() {
        let mut slice = [0u8; 10];
        let pos = 0;
        let mut p0 = Utf8Destination { slice: &mut slice, pos };
        let p1: u8 = 65;

        p0.write_code_unit(p1);

        assert_eq!(p0.slice[0], 65);
        assert_eq!(p0.pos, 1);
    }
}
#[cfg(test)]
mod tests_rug_218 {
    use super::*;
    use crate::handles::Utf8Destination;

    #[test]
    fn test_rug() {
        let mut p0: Utf8Destination = unimplemented!();
        let p1: u8 = 65;

        p0.write_ascii(p1);
    }
}#[cfg(test)]
mod tests_rug_219 {
    use super::*;
    use crate::handles::Utf8Destination;

    #[test]
    fn test_rug() {
        let mut p0: Utf8Destination = unimplemented!(); // construct handles::Utf8Destination<'a> based on the description
        let p1: u16 = 0xABCD; // sample u16 value

        p0.write_bmp(p1);
    }
}#[cfg(test)]
mod tests_rug_220 {
    use super::*;
    use crate::handles::Utf8Destination;

    #[test]
    fn test_rug() {
        let mut p0: Utf8Destination<'_> = unimplemented!(); // Replace unimplemented!() with the actual initialization
        let p1: u16 = 0x1234; // Sample data for the u16 argument

        p0.write_mid_bmp(p1);
    }
}#[cfg(test)]
mod tests_rug_222 {
    use super::*;

    use crate::handles::Utf8Destination;

    #[test]
    fn test_rug() {
        let mut p0: Utf8Destination = unimplemented!(); // construct Utf8Destination type variable
        let p1: u16 = 0x800u16; // sample data to initialize u16 variable

        p0.write_bmp_excl_ascii(p1);

        // Add your assertions here based on the expected behavior of the function
    }
}#[cfg(test)]
mod tests_rug_226 {
    use super::*;
    use crate::handles::{Utf8Destination, ByteSource, DecoderResult, CopyAsciiResult, Utf8BmpHandle};
    
    #[test]
    fn test_rug() {
        let mut p0: Utf8Destination<'_> = unimplemented!(); // Construct Utf8Destination instance
        let mut p1: ByteSource<'_> = unimplemented!(); // Construct ByteSource instance

        p0.copy_ascii_from_check_space_bmp(&mut p1);
    }
}
#[cfg(test)]
mod tests_rug_230 {
    use super::*;
    use crate::handles::Utf16Source;

    #[test]
    fn test_utf16_source_new() {
        let p0: &[u16] = &[0x0048, 0x0065, 0x006C, 0x006C, 0x006F]; // Sample data

        Utf16Source::new(p0);
    }
}
#[cfg(test)]
mod tests_rug_232 {
    use super::*;
    use crate::handles;

    #[test]
    fn test_read() {
        let slice: &[u16] = &[0xD83D, 0xDE04, 0xD83D, 0xDE0A];
        let mut utf16_source = handles::Utf16Source::new(slice);

        utf16_source.read();
        utf16_source.read();
    }
}#[cfg(test)]
mod tests_rug_233 {
    use super::*;
    use crate::handles::{Utf16Source, Unicode, NonAscii};

    #[test]
    fn test_rug() {
        let slice: &[u16] = &[0x0041u16, 0xD83Du16, 0xDE80u16]; // Sample data
        let pos: usize = 0; // Sample data
        let old_pos: usize = 0; // Sample data

        let mut p0 = Utf16Source {
            slice: slice,
            pos: pos,
            old_pos: old_pos,
        };

        p0.read_enum();
    }
}
#[cfg(test)]
mod tests_rug_235 {
    use super::*;
    use crate::handles::Utf16Source;

    #[test]
    fn test_rug() {
        let mut p0: Utf16Source = unimplemented!();
        
        p0.consumed();
        
    }
}
#[cfg(test)]
mod tests_rug_240 {
    use super::*;
    use crate::handles::{Utf16ReadHandle, Unicode, Utf16UnreadHandle};

    #[test]
    fn test_read_enum() {
        let mut p0: Utf16ReadHandle<'_, '_> = unimplemented!();
                
        let (character, handle) = Utf16ReadHandle::read_enum(p0);

        // Add assertions here based on the behavior of the function
    }
}        
#[cfg(test)]
mod tests_rug_244 {
    use super::*;
    use crate::handles::Utf16UnreadHandle;

    #[test]
    fn test_rug() {
        let mut p0: Utf16UnreadHandle = unimplemented!();
        
        p0.consumed();
    }
}
#[cfg(test)]
mod tests_rug_246 {
    use super::*;
    use crate::handles::Utf8Source;

    #[test]
    fn test_new() {
        let p0 = "Hello, 你好, مرحبا";
                
        Utf8Source::new(p0);
    }
}#[cfg(test)]
mod tests_rug_249 {
    use super::*;
    use crate::handles::{Utf8Source, Unicode, NonAscii};

    #[test]
    fn test_read_enum() {
        let slice: &[u8] = &[0xE6, 0x97, 0xA5, 0xF0, 0x9F, 0x98, 0x8E]; // Sample slice
        let mut p0: Utf8Source = Utf8Source {
            slice,
            pos: 0,
            old_pos: 0,
        };

        let result = p0.read_enum();

        // Add your assertions here
    }
}#[cfg(test)]
mod tests_rug_251 {
    use super::*;
    use crate::handles::Utf8Source;

    #[test]
    fn test_rug() {
        let mut p0: Utf8Source = todo!();

        assert_eq!(p0.consumed(), 0);
    }
}#[cfg(test)]
mod tests_rug_264 {
    use super::*;
    use crate::handles::ByteOneHandle;

    #[test]
    fn test_rug() {
        let mut p0: ByteOneHandle = todo!(); 

        p0.written();
    }
}#[cfg(test)]
mod tests_rug_266 {
    use super::*;
    use crate::handles::{ByteDestination, ByteTwoHandle};

    #[test]
    fn test_rug() {
        let mut p0: ByteDestination<'_> = unimplemented!();

        ByteTwoHandle::new(&mut p0);
    }
}#[cfg(test)]
mod tests_rug_270 {
    use super::*;
    use crate::handles::{ByteThreeHandle, ByteDestination};

    #[test]
    fn test_new() {
        let mut p0: ByteDestination<'_> = unimplemented!(); // Construct p0 based on the description provided

        ByteThreeHandle::<'_, '_>::new(&mut p0);
    }
}
#[cfg(test)]
mod tests_rug_272 {
    use super::*;
    use crate::{handles::{ByteThreeHandle, ByteDestination}};

    #[test]
    fn test_rug() {
        let mut p0: ByteThreeHandle = todo!("construct the ByteThreeHandle<'a, 'b> variable");
        let p1: u8 = 65;

        p0.write_one(p1);
    }
}
#[cfg(test)]
mod tests_rug_273 {
    use super::*;
    use crate::handles::ByteThreeHandle;
    
    #[test]
    fn test_rug() {
        let mut p0: ByteThreeHandle<'static, 'static> = unimplemented!(); // Example initialization
        let p1: u8 = 10; // Sample data
        let p2: u8 = 20; // Sample data

        p0.write_two(p1, p2);
    }
}#[cfg(test)]
mod tests_rug_275 {
    use super::*;
    use crate::handles::ByteThreeHandle;

    #[test]
    fn test_rug() {
        let mut p0: ByteThreeHandle = todo!("initialize p0 with appropriate value based on the description");

        let p1: u8 = 10;
        let p2: u8 = 20;
        let p3: u8 = 30;

        p0.write_three_return_written(p1, p2, p3);
    }
}#[cfg(test)]
mod tests_rug_277 {
    use super::*;
    use crate::handles::ByteFourHandle;
    
    #[test]
    fn test_written() {
        let mut p0: ByteFourHandle = unimplemented!();
        
        p0.written();
    }
}#[cfg(test)]
mod tests_rug_278 {
    use super::*;
    use crate::handles::ByteDestination;
    use crate::handles::ByteFourHandle; 
    
    #[test]
    fn test_rug() {
        let mut p0: ByteFourHandle = unimplemented!();
        let p1: u8 = 65;

        p0.write_one(p1);

    }
}#[cfg(test)]
mod tests_rug_279 {
    use super::*;
    use crate::handles::{ByteFourHandle, ByteDestination};

    #[test]
    fn test_write_two() {
        let p0: ByteFourHandle<'_, '_> = todo!(); // Fill in based on the description
        let p1: u8 = 10; // Sample data for the first argument
        let p2: u8 = 20; // Sample data for the second argument

        p0.write_two(p1, p2); // Call the target function for testing
    }
}#[cfg(test)]
mod tests_rug_281 {
    use super::*;
    use crate::handles::ByteDestination;

    #[test]
    fn test_new() {
        let mut p0 = [0u8; 10];

        let _ = ByteDestination::new(&mut p0);
    }
}#[cfg(test)]
mod tests_rug_284 {
    use super::*;
    use crate::handles::{ByteDestination, ByteThreeHandle, Utf16Source};
    
    #[test]
    fn test_rug() {
        let mut slice: [u8; 5] = [0; 5];
        let mut dest = ByteDestination::<'static>::new(&mut slice);
        
        dest.check_space_three();
    }
}#[cfg(test)]
mod tests_rug_286 {
    use super::*;
    use crate::handles::ByteDestination;

    #[test]
    fn test_written() {
        let mut p0: ByteDestination = unimplemented!();
        assert_eq!(p0.written(), 0);
    }
}#[cfg(test)]
mod tests_rug_287 {
    use super::*;
    use crate::handles::ByteDestination;

    #[test]
    fn test_rug() {
        let mut slice = [0u8; 10];
        let mut pos = 0;
        let mut p0 = ByteDestination { slice: &mut slice, pos };

        let first_byte: u8 = 65; // Sample data

        p0.write_one(first_byte);
        
        // Add assertions here
    }
}#[cfg(test)]
mod tests_rug_288 {
    use super::*;
    use crate::handles::ByteDestination;

    #[test]
    fn test_rug() {
        let mut slice: [u8; 10] = [0; 10]; // Sample slice data
        let mut pos: usize = 0; // Sample initial position
        let mut p0 = ByteDestination { slice: &mut slice, pos };
        let p1: u8 = 65; // Sample data for first argument
        let p2: u8 = 66; // Sample data for second argument

        p0.write_two(p1, p2);
    }
}#[cfg(test)]
mod tests_rug_289 {
    use super::*;
    use crate::handles;

    #[test]
    fn test_rug() {
        let mut slice = [0u8; 6];
        let mut pos = 0;
        let mut p0 = handles::ByteDestination { slice: &mut slice, pos };
        let p1: u8 = 65; // Sample data: ASCII value of 'A'
        let p2: u8 = 66; // Sample data: ASCII value of 'B'
        let p3: u8 = 67; // Sample data: ASCII value of 'C'

        p0.write_three(p1, p2, p3);
        
        assert_eq!(p0.slice[0], 65);
        assert_eq!(p0.slice[1], 66);
        assert_eq!(p0.slice[2], 67);
    }
}#[cfg(test)]
mod tests_rug_290 {
    use super::*;
    use crate::handles::ByteDestination;

    #[test]
    fn test_write_four() {
        let slice: &mut [u8] = &mut [0; 8];
        let mut pos: usize = 0;
        let mut p0 = ByteDestination { slice, pos };
        let p1: u8 = 1;
        let p2: u8 = 2;
        let p3: u8 = 3;
        let p4: u8 = 4;

        p0.write_four(p1, p2, p3, p4);
    }
}