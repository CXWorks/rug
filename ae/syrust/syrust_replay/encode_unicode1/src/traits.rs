/* Copyright 2016 The encode_unicode Developers
 *
 * Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
 * http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
 * http://opensource.org/licenses/MIT>, at your option. This file may not be
 * copied, modified, or distributed except according to those terms.
 */

#![allow(unused_unsafe)]// explicit unsafe{} blocks in unsafe functions are a good thing.

use utf8_char::Utf8Char;
use utf16_char::Utf16Char;
use utf8_iterators::*;
use utf16_iterators::*;
use decoding_iterators::*;
use error::*;
extern crate core;
use self::core::{char, u32, mem};
use self::core::ops::{Not, Index, RangeFull};
use self::core::borrow::Borrow;
#[cfg(feature="ascii")]
extern crate ascii;
#[cfg(feature="ascii")]
use self::ascii::AsciiStr;

// TODO better docs and tests

/// Methods for working with `u8`s as UTF-8 bytes.
pub trait U8UtfExt {
    /// How many more bytes will you need to complete this codepoint?
    ///
    /// # Errors
    ///
    /// An error is returned if the byte is not a valid start of an UTF-8
    /// codepoint:
    ///
    /// * `128..192`: ContinuationByte
    /// * `248..`: TooLongSequence
    ///
    /// Values in 244..248 represent a too high codepoint, but do not cause an
    /// error.
    fn extra_utf8_bytes(self) -> Result<usize,InvalidUtf8FirstByte>;

    /// How many more bytes will you need to complete this codepoint?
    ///
    /// This function assumes that the byte is a valid UTF-8 start, and might
    /// return any value otherwise. (but the function is pure and safe to call
    /// with any value).
    fn extra_utf8_bytes_unchecked(self) -> usize;
}

impl U8UtfExt for u8 {
    #[inline]
    fn extra_utf8_bytes(self) -> Result<usize,InvalidUtf8FirstByte> {
        use error::InvalidUtf8FirstByte::{ContinuationByte,TooLongSeqence};
        // the bit twiddling is explained in extra_utf8_bytes_unchecked()
        if self < 128 {
            return Ok(0);
        }
        match ((self as u32)<<25).not().leading_zeros() {
            n @ 1...3 => Ok(n as usize),
            0 => Err(ContinuationByte),
            _ => Err(TooLongSeqence),
        }
    }
    #[inline]
    fn extra_utf8_bytes_unchecked(self) -> usize {
        // For fun I've optimized this function (for x86 instruction count):
        // The most straightforward implementation, that lets the compiler do
        // the optimizing:
        //match self {
        //    0b0000_0000...0b0111_1111 => 0,
        //    0b1100_0010...0b1101_1111 => 1,
        //    0b1110_0000...0b1110_1111 => 2,
        //    0b1111_0000...0b1111_0100 => 3,
        //                _             => whatever()
        //}
        // Using `unsafe{self::core::hint::unreachable_unchecked()}` for the
        // "don't care" case is a terrible idea: while having the function
        // non-deterministically return whatever happens to be in a register
        // MIGHT be acceptable, it permits the function to not `ret`urn at all,
        // but let execution fall through to whatever comes after it in the
        // binary! (in other words completely UB).
        // Currently unreachable_unchecked() might trap too,
        // which is certainly not what we want.
        // I also think `unsafe{mem::unitialized()}` is much more likely to
        // explicitly produce whatever happens to be in a register than tell
        // the compiler it can ignore this branch but needs to produce a value.
        //
        // From the bit patterns we see that for non-ASCII values the result is
        // (number of leading set bits) - 1
        // The standard library doesn't have a method for counting leading ones,
        // but it has leading_zeros(), which can be used after inverting.
        // This function can therefore be reduced to the one-liner
        //`self.not().leading_zeros().saturating_sub(1) as usize`, which would
        // be branchless for architectures with instructions for
        // leading_zeros() and saturating_sub().

        // Shortest version as long as ASCII-ness can be predicted: (especially
        // if the BSR instruction which leading_zeros() uses is microcoded or
        // doesn't exist)
        // u8.leading_zeros() would cast to a bigger type internally, so that's
        // free. compensating by shifting left by 24 before inverting lets the
        // compiler know that the value passed to leading_zeros() is not zero,
        // for which BSR's output is undefined and leading_zeros() normally has
        // special case with a branch.
        // Shifting one bit too many left acts as a saturating_sub(1).
        if self<128 {0} else {((self as u32)<<25).not().leading_zeros() as usize}

        // Branchless but longer version: (9 instructions)
        // It's tempting to try (self|0x80).not().leading_zeros().wrapping_sub(1),
        // but that produces high lengths for ASCII values 0b01xx_xxxx.
        // If we could somehow (branchlessy) clear that bit for ASCII values...
        // We can by masking with the value shifted right with sign extension!
        // (any nonzero number of bits in range works)
        //let extended = self as i8 as i32;
        //let ascii_cleared = (extended<<25) & (extended>>25);
        //ascii_cleared.not().leading_zeros() as usize

        // cmov version: (7 instructions)
        //(((self as u32)<<24).not().leading_zeros() as usize).saturating_sub(1)
    }
}


/// Methods for working with `u16`s as UTF-16 units.
pub trait U16UtfExt {
    /// Will you need an extra unit to complete this codepoint?
    ///
    /// Returns `Err` for trailing surrogates, `Ok(true)` for leading surrogates,
    /// and `Ok(false)` for others.
    fn utf16_needs_extra_unit(self) -> Result<bool,InvalidUtf16FirstUnit>;

    /// Does this `u16` need another `u16` to complete a codepoint?
    /// Returns `(self & 0xfc00) == 0xd800`
    ///
    /// Is basically an unchecked variant of `utf16_needs_extra_unit()`.
    fn is_utf16_leading_surrogate(self) -> bool;
}
impl U16UtfExt for u16 {
    #[inline]
    fn utf16_needs_extra_unit(self) -> Result<bool,InvalidUtf16FirstUnit> {
        match self {
            // https://en.wikipedia.org/wiki/UTF-16#U.2B10000_to_U.2B10FFFF
            0x00_00...0xd7_ff | 0xe0_00...0xff_ff => Ok(false),
            0xd8_00...0xdb_ff => Ok(true),
                    _         => Err(InvalidUtf16FirstUnit)
        }
    }
    #[inline]
    fn is_utf16_leading_surrogate(self) -> bool {
        (self & 0xfc00) == 0xd800// Clear the ten content bytes of a surrogate,
                                 // and see if it's a leading surrogate.
    }
}




/// Extension trait for `char` that adds methods for converting to and from UTF-8 or UTF-16.
pub trait CharExt: Sized {
    /// Get the UTF-8 representation of this codepoint.
    ///
    /// `Utf8Char` is to `[u8;4]` what `char` is to `u32`:
    /// a restricted type that cannot be mutated internally.
    fn to_utf8(self) -> Utf8Char;

    /// Get the UTF-16 representation of this codepoint.
    ///
    /// `Utf16Char` is to `[u16;2]` what `char` is to `u32`:
    /// a restricted type that cannot be mutated internally.
    fn to_utf16(self) -> Utf16Char;

    /// Iterate over or [read](https://doc.rust-lang.org/std/io/trait.Read.html)
    /// the one to four bytes in the UTF-8 representation of this codepoint.
    ///
    /// An identical alternative to the unstable `char.encode_utf8()`.
    /// That method somehow still exist on stable, so I have to use a different name.
    fn iter_utf8_bytes(self) -> Utf8Iterator;

    /// Iterate over the one or two units in the UTF-16 representation of this codepoint.
    ///
    /// An identical alternative to the unstable `char.encode_utf16()`.
    /// That method somehow still exist on stable, so I have to use a different name.
    fn iter_utf16_units(self) -> Utf16Iterator;


    /// Convert this char to an UTF-8 array, and also return how many bytes of
    /// the array are used,
    ///
    /// The returned array is left-aligned with unused bytes set to zero.
    fn to_utf8_array(self) -> ([u8; 4], usize);

    /// Convert this `char` to UTF-16.
    ///
    /// The second element is non-zero when a surrogate pair is required.
    ///
    /// # Examples
    ///
    /// ```
    /// use encode_unicode::CharExt;
    ///
    /// assert_eq!('@'.to_utf16_array(), ['@' as u16, 0]);
    /// assert_eq!('睷'.to_utf16_array(), ['睷' as u16, 0]);
    /// assert_eq!('\u{abcde}'.to_utf16_array(), [0xda6f, 0xdcde]);
    /// ```
    fn to_utf16_array(self) -> [u16; 2];

    /// Convert this `char` to UTF-16.
    /// The second item is `Some` if a surrogate pair is required.
    ///
    /// # Examples
    ///
    /// ```
    /// use encode_unicode::CharExt;
    ///
    /// assert_eq!('@'.to_utf16_tuple(), ('@' as u16, None));
    /// assert_eq!('睷'.to_utf16_tuple(), ('睷' as u16, None));
    /// assert_eq!('\u{abcde}'.to_utf16_tuple(), (0xda6f, Some(0xdcde)));
    /// ```
    fn to_utf16_tuple(self) -> (u16, Option<u16>);



    /// Create a `char` from the start of an UTF-8 slice,
    /// and also return how many bytes were used.
    ///
    /// # Errors
    ///
    /// Returns an `Err` if the slice is empty, doesn't start with a valid
    /// UTF-8 sequence or is too short for the sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use encode_unicode::CharExt;
    /// use encode_unicode::error::InvalidUtf8Slice::*;
    /// use encode_unicode::error::InvalidUtf8::*;
    ///
    /// assert_eq!(char::from_utf8_slice_start(&[b'A', b'B', b'C']), Ok(('A',1)));
    /// assert_eq!(char::from_utf8_slice_start(&[0xdd, 0xbb]), Ok(('\u{77b}',2)));
    ///
    /// assert_eq!(char::from_utf8_slice_start(&[]), Err(TooShort(1)));
    /// assert_eq!(char::from_utf8_slice_start(&[0xf0, 0x99]), Err(TooShort(4)));
    /// assert_eq!(char::from_utf8_slice_start(&[0xee, b'F', 0x80]), Err(Utf8(NotAContinuationByte(1))));
    /// assert_eq!(char::from_utf8_slice_start(&[0xee, 0x99, 0x0f]), Err(Utf8(NotAContinuationByte(2))));
    /// ```
    fn from_utf8_slice_start(src: &[u8]) -> Result<(Self,usize),InvalidUtf8Slice>;

    /// Create a `char` from the start of an UTF-16 slice,
    /// and also return how many units were used.
    ///
    /// If you want to continue after an error, continue with the next `u16` unit.
    fn from_utf16_slice_start(src: &[u16]) -> Result<(Self,usize), InvalidUtf16Slice>;


    /// Convert an UTF-8 sequence as returned from `.to_utf8_array()` into a `char`
    ///
    /// The codepoint must start at the first byte, and leftover bytes are ignored.
    ///
    /// # Errors
    ///
    /// Returns an `Err` if the array doesn't start with a valid UTF-8 sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use encode_unicode::CharExt;
    /// use encode_unicode::error::InvalidUtf8Array::*;
    /// use encode_unicode::error::InvalidUtf8::*;
    /// use encode_unicode::error::InvalidCodepoint::*;
    ///
    /// assert_eq!(char::from_utf8_array([b'A', 0, 0, 0]), Ok('A'));
    /// assert_eq!(char::from_utf8_array([0xf4, 0x8b, 0xbb, 0xbb]), Ok('\u{10befb}'));
    /// assert_eq!(char::from_utf8_array([b'A', b'B', b'C', b'D']), Ok('A'));
    /// assert_eq!(char::from_utf8_array([0, 0, 0xcc, 0xbb]), Ok('\0'));
    ///
    /// assert_eq!(char::from_utf8_array([0xef, b'F', 0x80, 0x80]), Err(Utf8(NotAContinuationByte(1))));
    /// assert_eq!(char::from_utf8_array([0xc1, 0x80, 0, 0]), Err(Utf8(OverLong)));
    /// assert_eq!(char::from_utf8_array([0xf7, 0xaa, 0x99, 0x88]), Err(Codepoint(TooHigh)));
    /// ```
    fn from_utf8_array(utf8: [u8; 4]) -> Result<Self,InvalidUtf8Array>;

    /// Convert a UTF-16 pair as returned from `.to_utf16_array()` into a `char`.
    ///
    /// The second element is ignored when not required.
    ///
    /// # Examples
    ///
    /// ```
    /// use encode_unicode::CharExt;
    /// use encode_unicode::error::InvalidUtf16Array;
    ///
    /// assert_eq!(char::from_utf16_array(['x' as u16, 'y' as u16]), Ok('x'));
    /// assert_eq!(char::from_utf16_array(['睷' as u16, 0]), Ok('睷'));
    /// assert_eq!(char::from_utf16_array([0xda6f, 0xdcde]), Ok('\u{abcde}'));
    /// assert_eq!(char::from_utf16_array([0xf111, 0xdbad]), Ok('\u{f111}'));
    /// assert_eq!(char::from_utf16_array([0xdaaf, 0xdaaf]), Err(InvalidUtf16Array::SecondIsNotTrailingSurrogate));
    /// assert_eq!(char::from_utf16_array([0xdcac, 0x9000]), Err(InvalidUtf16Array::FirstIsTrailingSurrogate));
    /// ```
    fn from_utf16_array(utf16: [u16; 2]) -> Result<Self, InvalidUtf16Array>;

    /// Convert a UTF-16 pair as returned from `.to_utf16_tuple()` into a `char`.
    fn from_utf16_tuple(utf16: (u16, Option<u16>)) -> Result<Self, InvalidUtf16Tuple>;


    /// Convert an UTF-8 sequence into a char.
    ///
    /// The length of the slice is taken as length of the sequence;
    /// it should be 1,2,3 or 4.
    ///
    /// # Safety
    ///
    /// The slice must contain exactly one, valid, UTF-8 sequence.
    ///
    /// Passing a slice that produces an invalid codepoint is always undefined
    /// behavior; Later checks that the codepoint is valid can be removed
    /// by the compiler.
    ///
    /// # Panics
    ///
    /// If the slice is empty
    unsafe fn from_utf8_exact_slice_unchecked(src: &[u8]) -> Self;

    /// Convert a UTF-16 array as returned from `.to_utf16_array()` into a
    /// `char`.
    ///
    /// This function is safe because it avoids creating invalid codepoints,
    /// but the returned value might not be what one expectedd.
    ///
    /// # Examples
    ///
    /// ```
    /// use encode_unicode::CharExt;
    ///
    /// // starts with a trailing surrogate - converted as if it was a valid
    /// // surrogate pair anyway.
    /// assert_eq!(char::from_utf16_array_unchecked([0xdbad, 0xf19e]), '\u{fb59e}');
    /// // missing trailing surrogate - ditto
    /// assert_eq!(char::from_utf16_array_unchecked([0xd802, 0]), '\u{10800}');
    /// ```
    fn from_utf16_array_unchecked(utf16: [u16;2]) -> Self;

    /// Convert a UTF-16 tuple as returned from `.to_utf16_tuple()` into a `char`.
    unsafe fn from_utf16_tuple_unchecked(utf16: (u16, Option<u16>)) -> Self;


    /// Produces more detailed errors than `char::from_u32()`
    ///
    /// # Errors
    ///
    /// This function will return an error if
    ///
    /// * the value is greater than 0x10ffff
    /// * the value is between 0xd800 and 0xdfff (inclusive)
    ///
    /// # Examples
    ///
    /// ```
    /// use encode_unicode::CharExt;
    /// use encode_unicode::error::InvalidCodepoint;
    ///
    /// assert_eq!(char::from_u32_detailed(0x41), Ok('A'));
    /// assert_eq!(char::from_u32_detailed(0x40_00_00), Err(InvalidCodepoint::TooHigh));
    /// assert_eq!(char::from_u32_detailed(0xd951), Err(InvalidCodepoint::Utf16Reserved));
    /// assert_eq!(char::from_u32_detailed(0xdddd), Err(InvalidCodepoint::Utf16Reserved));
    /// assert_eq!(char::from_u32_detailed(0xdd), Ok('Ý'));
    /// assert_eq!(char::from_u32_detailed(0x1f331), Ok('🌱'));
    /// ```
    fn from_u32_detailed(c: u32) -> Result<Self,InvalidCodepoint>;
}



impl CharExt for char {
      /////////
     //UTF-8//
    /////////

    fn to_utf8(self) -> Utf8Char {
        self.into()
    }
    fn iter_utf8_bytes(self) -> Utf8Iterator {
        self.to_utf8().into_iter()
    }

    fn to_utf8_array(self) -> ([u8; 4], usize) {
        let len = self.len_utf8();
        let mut c = self as u32;
        if len == 1 {// ASCII, the common case
            ([c as u8, 0, 0, 0],  1)
        } else {
            let mut parts = 0;// convert to 6-bit bytes
                        parts |= c & 0x3f;  c>>=6;
            parts<<=8;  parts |= c & 0x3f;  c>>=6;
            parts<<=8;  parts |= c & 0x3f;  c>>=6;
            parts<<=8;  parts |= c & 0x3f;
            parts |= 0x80_80_80_80;// set the most significant bit
            parts >>= 8*(4-len);// right-align bytes
            // Now, unused bytes are zero, (which matters for Utf8Char.eq())
            // and the rest are 0b10xx_xxxx

            // set header on first byte
            parts |= (0xff_00u32 >> len)  &  0xff;// store length
            parts &= Not::not(1u32 << 7-len);// clear the next bit after it

            let bytes: [u8; 4] = unsafe{ mem::transmute(u32::from_le(parts)) };
            (bytes, len)
        }
    }


    fn from_utf8_slice_start(src: &[u8]) -> Result<(Self,usize),InvalidUtf8Slice> {
        use errors::InvalidUtf8::*;
        use errors::InvalidUtf8Slice::*;
        let first = match src.first() {
            Some(first) => *first,
            None => return Err(TooShort(1)),
        };
        let bytes = match first.extra_utf8_bytes() {
            Err(e)    => return Err(Utf8(FirstByte(e))),
            Ok(0)     => return Ok((first as char, 1)),
            Ok(extra) if extra >= src.len()
                      => return Err(TooShort(extra+1)),
            Ok(extra) => &src[..extra+1],
        };
        if let Some(i) = bytes.iter().skip(1).position(|&b| (b >> 6) != 0b10 ) {
            Err(Utf8(NotAContinuationByte(i+1)))
        } else if overlong(bytes[0], bytes[1]) {
            Err(Utf8(OverLong))
        } else {
            match char::from_u32_detailed(merge_nonascii_unchecked_utf8(bytes)) {
                Ok(c) => Ok((c, bytes.len())),
                Err(e) => Err(Codepoint(e)),
            }
        }
    }

    fn from_utf8_array(utf8: [u8; 4]) -> Result<Self,InvalidUtf8Array> {
        use errors::InvalidUtf8::*;
        use errors::InvalidUtf8Array::*;
        let src = match utf8[0].extra_utf8_bytes() {
            Err(error) => return Err(Utf8(FirstByte(error))),
            Ok(0)      => return Ok(utf8[0] as char),
            Ok(extra)  => &utf8[..extra+1],
        };
        if let Some(i) = src[1..].iter().position(|&b| (b >> 6) != 0b10 ) {
            Err(Utf8(NotAContinuationByte(i+1)))
        } else if overlong(utf8[0], utf8[1]) {
            Err(Utf8(OverLong))
        } else {
            char::from_u32_detailed(merge_nonascii_unchecked_utf8(src))
                 .map_err(|e| Codepoint(e) )
        }
    }

    unsafe fn from_utf8_exact_slice_unchecked(src: &[u8]) -> Self {
        if src.len() == 1 {
            src[0] as char
        } else {
            char::from_u32_unchecked(merge_nonascii_unchecked_utf8(src))
        }
    }



      //////////
     //UTF-16//
    //////////

    fn to_utf16(self) -> Utf16Char {
        Utf16Char::from(self)
    }
    fn iter_utf16_units(self) -> Utf16Iterator {
        self.to_utf16().into_iter()
    }

    fn to_utf16_array(self) -> [u16;2] {
        let (first, second) = self.to_utf16_tuple();
        [first, second.unwrap_or(0)]
    }
    fn to_utf16_tuple(self) -> (u16, Option<u16>) {
        if self <= '\u{ffff}' {// single
            (self as u16, None)
        } else {// double
            let c = self as u32 - 0x_01_00_00;
            let high = 0x_d8_00 + (c >> 10);
            let low = 0x_dc_00 + (c & 0x_03_ff);
            (high as u16,  Some(low as u16))
        }
    }


    fn from_utf16_slice_start(src: &[u16]) -> Result<(Self,usize), InvalidUtf16Slice> {
        use errors::InvalidUtf16Slice::*;
        unsafe {match (src.get(0), src.get(1)) {
            (Some(&u @ 0x00_00...0xd7_ff), _) |
            (Some(&u @ 0xe0_00...0xff_ff), _)
                => Ok((char::from_u32_unchecked(u as u32), 1)),
            (Some(&0xdc_00...0xdf_ff), _) => Err(FirstLowSurrogate),
            (None, _) => Err(EmptySlice),
            (Some(&f @ 0xd8_00...0xdb_ff), Some(&s @ 0xdc_00...0xdf_ff))
                => Ok((char::from_utf16_tuple_unchecked((f, Some(s))), 2)),
            (Some(&0xd8_00...0xdb_ff), Some(_)) => Err(SecondNotLowSurrogate),
            (Some(&0xd8_00...0xdb_ff), None) => Err(MissingSecond),
            (Some(_), _) => unreachable!()
        }}
    }

    fn from_utf16_array(utf16: [u16;2]) -> Result<Self, InvalidUtf16Array> {
        use errors::InvalidUtf16Array::*;
        if let Some(c) = char::from_u32(utf16[0] as u32) {
            Ok(c) // single
        } else if utf16[0] < 0xdc_00  &&  utf16[1] & 0xfc_00 == 0xdc_00 {
            // correct surrogate pair
            Ok(combine_surrogates(utf16[0], utf16[1]))
        } else if utf16[0] < 0xdc_00 {
            Err(SecondIsNotTrailingSurrogate)
        } else {
            Err(FirstIsTrailingSurrogate)
        }
    }
    fn from_utf16_tuple(utf16: (u16, Option<u16>)) -> Result<Self, InvalidUtf16Tuple> {
        use errors::InvalidUtf16Tuple::*;
        unsafe{ match utf16 {
            (0x00_00...0xd7_ff, None) | // single
            (0xe0_00...0xff_ff, None) | // single
            (0xd8_00...0xdb_ff, Some(0xdc_00...0xdf_ff)) // correct surrogate
                => Ok(char::from_utf16_tuple_unchecked(utf16)),
            (0xd8_00...0xdb_ff, Some(_)) => Err(InvalidSecond),
            (0xd8_00...0xdb_ff, None   ) => Err(MissingSecond),
            (0xdc_00...0xdf_ff,    _   ) => Err(FirstIsTrailingSurrogate),
            (        _        , Some(_)) => Err(SuperfluousSecond),
            (        _        , None   ) => unreachable!()
        }}
    }

    fn from_utf16_array_unchecked(utf16: [u16;2]) -> Self {
        // treat any array with a surrogate value in [0] as a surrogate because
        // combine_surrogates() is safe.
        // `(utf16[0] & 0xf800) == 0xd80` might not be quite as fast as
        // `utf16[1] != 0`, but avoiding the potential for UB is worth it
        // since the conversion isn't zero-cost in either case.
        char::from_u32(utf16[0] as u32)
            .unwrap_or_else(|| combine_surrogates(utf16[0], utf16[1]) )
    }
    unsafe fn from_utf16_tuple_unchecked(utf16: (u16, Option<u16>)) -> Self {
        match utf16.1 {
            Some(second) => combine_surrogates(utf16.0, second),
            None         => char::from_u32_unchecked(utf16.0 as u32)
        }
    }


    fn from_u32_detailed(c: u32) -> Result<Self,InvalidCodepoint> {
        match char::from_u32(c) {
            Some(c) => Ok(c),
            None if c > 0x10_ff_ff => Err(InvalidCodepoint::TooHigh),
            None => Err(InvalidCodepoint::Utf16Reserved),
        }
    }
}

// Adapted from https://www.cl.cam.ac.uk/~mgk25/ucs/utf8_check.c
fn overlong(first: u8, second: u8) -> bool {
    if first < 0x80 {
        false
    } else if (first & 0xe0) == 0xc0 {
        (first & 0xfe) == 0xc0
    } else if (first & 0xf0) == 0xe0 {
        first == 0xe0 && (second & 0xe0) == 0x80
    } else {
        first == 0xf0 && (second & 0xf0) == 0x80
    }
}

/// Decodes the codepoint represented by a multi-byte UTF-8 sequence.
///
/// Does not check that the codepoint is valid,
/// and returns `u32` because casting invalid codepoints to `char` is insta UB.
fn merge_nonascii_unchecked_utf8(src: &[u8]) -> u32 {
    let mut c = src[0] as u32 & (0x7f >> src.len());
    for b in &src[1..] {
        c = (c << 6)  |  (b & 0b0011_1111) as u32;
    }
    c
}

/// Create a `char` from a leading and a trailing surrogate.
///
/// This function is safe because it ignores the six most significant bits of
/// each arguments and always produces a codepoint in 0x01_00_00..=0x10_ff_ff.
fn combine_surrogates(first: u16,  second: u16) -> char {
    unsafe {
        let high = (first & 0x_03_ff) as u32;
        let low = (second & 0x_03_ff) as u32;
        let c = ((high << 10) | low) + 0x_01_00_00; // no, the constant can't be or'd in
        char::from_u32_unchecked(c)
    }
}



/// Adds `.utf8chars()` and `.utf16chars()` iterator constructors to `&str`.
pub trait StrExt: AsRef<str> {
    /// Equivalent to `.chars()` but produces `Utf8Char`s.
    fn utf8chars(&self) -> Utf8Chars;
    /// Equivalent to `.chars()` but produces `Utf16Char`s.
    fn utf16chars(&self) -> Utf16Chars;
    /// Equivalent to `.char_indices()` but produces `Utf8Char`s.
    fn utf8char_indices(&self) -> Utf8CharIndices;
    /// Equivalent to `.char_indices()` but produces `Utf16Char`s.
    fn utf16char_indices(&self) -> Utf16CharIndices;
}

impl StrExt for str {
    fn utf8chars(&self) -> Utf8Chars {
        Utf8Chars::from(self)
    }
    fn utf16chars(&self) -> Utf16Chars {
        Utf16Chars::from(self)
    }
    fn utf8char_indices(&self) -> Utf8CharIndices {
        Utf8CharIndices::from(self)
    }
    fn utf16char_indices(&self) -> Utf16CharIndices {
        Utf16CharIndices::from(self)
    }
}

#[cfg(feature="ascii")]
impl StrExt for AsciiStr {
    fn utf8chars(&self) -> Utf8Chars {
        Utf8Chars::from(self.as_str())
    }
    fn utf16chars(&self) -> Utf16Chars {
        Utf16Chars::from(self.as_str())
    }
    fn utf8char_indices(&self) -> Utf8CharIndices {
        Utf8CharIndices::from(self.as_str())
    }
    fn utf16char_indices(&self) -> Utf16CharIndices {
        Utf16CharIndices::from(self.as_str())
    }
}



/// Iterator methods that convert between `u8`s and `Utf8Char` or `u16`s and `Utf16Char`
///
/// All the iterator adapters also accept iterators that produce references of
/// the type they convert from.
pub trait IterExt: Iterator+Sized {
    /// Converts an iterator of `Utf8Char`s or `&Utf8Char`s to an iterator of
    /// `u8`s.
    ///
    /// Has the same effect as `.flat_map()` or `.flatten()`, but the returned
    /// iterator is ~40% faster.
    ///
    /// The iterator also implements `Read`
    /// (when the `std` feature isn't disabled).  
    /// Reading will never produce an error, and calls to `.read()` and `.next()`
    /// can be mixed.
    ///
    /// The exact number of bytes cannot be known in advance, but `size_hint()`
    /// gives the possible range.
    /// (min: all remaining characters are ASCII, max: all require four bytes)
    ///
    /// # Examples
    ///
    /// From iterator of values:
    ///
    /// ```
    /// use encode_unicode::{IterExt, StrExt};
    ///
    /// let iterator = "foo".utf8chars();
    /// let mut bytes = [0; 4];
    /// for (u,dst) in iterator.to_bytes().zip(&mut bytes) {*dst=u;}
    /// assert_eq!(&bytes, b"foo\0");
    /// ```
    ///
    /// From iterator of references:
    ///
    #[cfg_attr(feature="std", doc=" ```")]
    #[cfg_attr(not(feature="std"), doc=" ```no_compile")]
    /// use encode_unicode::{IterExt, StrExt, Utf8Char};
    ///
    /// let chars: Vec<Utf8Char> = "💣 bomb 💣".utf8chars().collect();
    /// let bytes: Vec<u8> = chars.iter().to_bytes().collect();
    /// let flat_map: Vec<u8> = chars.iter().flat_map(|u8c| *u8c ).collect();
    /// assert_eq!(bytes, flat_map);
    /// ```
    ///
    /// `Read`ing from it:
    ///
    #[cfg_attr(feature="std", doc=" ```")]
    #[cfg_attr(not(feature="std"), doc=" ```no_compile")]
    /// use encode_unicode::{IterExt, StrExt};
    /// use std::io::Read;
    ///
    /// let s = "Ååh‽";
    /// assert_eq!(s.len(), 8);
    /// let mut buf = [b'E'; 9];
    /// let mut reader = s.utf8chars().to_bytes();
    /// assert_eq!(reader.read(&mut buf[..]).unwrap(), 8);
    /// assert_eq!(reader.read(&mut buf[..]).unwrap(), 0);
    /// assert_eq!(&buf[..8], s.as_bytes());
    /// assert_eq!(buf[8], b'E');
    /// ```
    fn to_bytes(self) -> Utf8CharSplitter<Self::Item,Self> where Self::Item: Borrow<Utf8Char>;

    /// Converts an iterator of `Utf16Char` (or `&Utf16Char`) to an iterator of
    /// `u16`s.
    ///
    /// Has the same effect as `.flat_map()` or `.flatten()`, but the returned
    /// iterator is about twice as fast.
    ///
    /// The exact number of units cannot be known in advance, but `size_hint()`
    /// gives the possible range.
    ///
    /// # Examples
    ///
    /// From iterator of values:
    ///
    /// ```
    /// use encode_unicode::{IterExt, StrExt};
    ///
    /// let iterator = "foo".utf16chars();
    /// let mut units = [0; 4];
    /// for (u,dst) in iterator.to_units().zip(&mut units) {*dst=u;}
    ///
    /// assert_eq!(units, ['f' as u16, 'o' as u16, 'o' as u16, 0]);
    /// ```
    ///
    /// From iterator of references:
    ///
    #[cfg_attr(feature="std", doc=" ```")]
    #[cfg_attr(not(feature="std"), doc=" ```no_compile")]
    /// use encode_unicode::{IterExt, StrExt, Utf16Char};
    ///
    /// // (💣 takes two units)
    /// let chars: Vec<Utf16Char> = "💣 bomb 💣".utf16chars().collect();
    /// let units: Vec<u16> = chars.iter().to_units().collect();
    /// let flat_map: Vec<u16> = chars.iter().flat_map(|u16c| *u16c ).collect();
    ///
    /// assert_eq!(units, flat_map);
    /// ```
    fn to_units(self) -> Utf16CharSplitter<Self::Item,Self> where Self::Item: Borrow<Utf16Char>;

    /// Decodes bytes as UTF-8 and groups them into `Utf8Char`s
    ///
    /// When errors (invalid values or sequences) are encountered,
    /// it continues with the byte right after the start of the error sequence.  
    /// This is neither the most intelligent choiche (sometimes it is guaranteed to
    ///  produce another error), nor the easiest to implement, but I believe it to
    /// be the most predictable.
    /// It also means that ASCII characters are never hidden by errors.
    ///
    /// # Examples
    ///
    /// Replace all errors with u+FFFD REPLACEMENT_CHARACTER:
    /// ```
    /// use encode_unicode::{Utf8Char, IterExt};
    ///
    /// let mut buf = [b'\0'; 255];
    /// let len = b"foo\xCFbar".iter()
    ///     .to_utf8chars()
    ///     .flat_map(|r| r.unwrap_or(Utf8Char::from('\u{FFFD}')).into_iter() )
    ///     .zip(&mut buf[..])
    ///     .map(|(byte, dst)| *dst = byte )
    ///     .count();
    ///
    /// assert_eq!(&buf[..len], "foo\u{FFFD}bar".as_bytes());
    /// ```
    ///
    /// Collect everything up until the first error into a string:
    #[cfg_attr(feature="std", doc=" ```")]
    #[cfg_attr(not(feature="std"), doc=" ```no_compile")]
    /// use encode_unicode::iterator::Utf8CharMerger;
    /// let mut good = String::new();
    /// for r in Utf8CharMerger::from(b"foo\xcc\xbbbar\xcc\xddbaz") {
    ///     if let Ok(uc) = r {
    ///         good.push_str(uc.as_str());
    ///     } else {
    ///         break;
    ///     }
    /// }
    /// assert_eq!(good, "foo̻bar");
    /// ```
    ///
    /// Abort decoding on error:
    #[cfg_attr(feature="std", doc=" ```")]
    #[cfg_attr(not(feature="std"), doc=" ```no_compile")]
    /// use encode_unicode::{IterExt, Utf8Char};
    /// use encode_unicode::error::{InvalidUtf8Slice, InvalidUtf8};
    ///
    /// let result = b"ab\0\xe0\xbc\xa9 \xf3\x80\x77".iter()
    ///     .to_utf8chars()
    ///     .collect::<Result<String,InvalidUtf8Slice>>();
    ///
    /// assert_eq!(result, Err(InvalidUtf8Slice::Utf8(InvalidUtf8::NotAContinuationByte(2))));
    /// ```
    fn to_utf8chars(self) -> Utf8CharMerger<Self::Item,Self> where Self::Item: Borrow<u8>;

    /// Decodes bytes as UTF-16 and groups them into `Utf16Char`s
    ///
    /// When errors (unmatched leading surrogates or unexpected trailing surrogates)
    /// are encountered, an error is produced for every unit.
    ///
    /// # Examples
    ///
    /// Replace errors with '�':
    #[cfg_attr(feature="std", doc=" ```")]
    #[cfg_attr(not(feature="std"), doc=" ```no_compile")]
    /// use encode_unicode::{IterExt, Utf16Char};
    ///
    /// let slice = &['a' as u16, 0xdf00, 0xd83c, 0xdca0][..];
    /// let string = slice.iter()
    ///     .to_utf16chars()
    ///     .map(|r| r.unwrap_or(Utf16Char::from('\u{fffd}')) ) // REPLACEMENT_CHARACTER
    ///     .collect::<String>();
    ///
    /// assert_eq!(string, "a�🂠");
    /// ```
    ///
    /// ```
    /// use encode_unicode::{IterExt, Utf16Char};
    /// use encode_unicode::error::Utf16PairError::*;
    ///
    /// let slice = [0xdcba, 0xdeff, 0xd8be, 0xdeee, 'Y' as u16, 0xdab1, 0xdab1];
    /// let mut iter = slice.iter().to_utf16chars();
    /// assert_eq!(iter.size_hint(), (3, Some(7)));
    /// assert_eq!(iter.next(), Some(Err(UnexpectedTrailingSurrogate)));
    /// assert_eq!(iter.next(), Some(Err(UnexpectedTrailingSurrogate)));
    /// assert_eq!(iter.next(), Some(Ok(Utf16Char::from('\u{3faee}'))));
    /// assert_eq!(iter.next(), Some(Ok(Utf16Char::from('Y'))));
    /// assert_eq!(iter.next(), Some(Err(UnmatchedLeadingSurrogate)));
    /// assert_eq!(iter.next(), Some(Err(Incomplete)));
    /// assert_eq!(iter.into_remaining_units().next(), None);
    /// ```
    ///
    /// Search for a codepoint and return the codepoint index of the first match:
    /// ```
    /// use encode_unicode::{IterExt, Utf16Char};
    ///
    /// let position = [0xd875, 0xdd4f, '≈' as u16, '2' as u16].iter()
    ///     .to_utf16chars()
    ///     .position(|r| r == Ok(Utf16Char::from('≈')) );
    ///
    /// assert_eq!(position, Some(1));
    /// ```
    fn to_utf16chars(self) -> Utf16CharMerger<Self::Item,Self> where Self::Item: Borrow<u16>;
}

impl<I:Iterator> IterExt for I {
    fn to_bytes(self) -> Utf8CharSplitter<Self::Item,Self> where Self::Item: Borrow<Utf8Char> {
        iter_bytes(self)
    }
    fn to_units(self) -> Utf16CharSplitter<Self::Item,Self> where Self::Item: Borrow<Utf16Char> {
        iter_units(self)
    }
    fn to_utf8chars(self) -> Utf8CharMerger<Self::Item,Self> where Self::Item: Borrow<u8> {
        Utf8CharMerger::from(self)
    }
    fn to_utf16chars(self) -> Utf16CharMerger<Self::Item,Self> where Self::Item: Borrow<u16> {
        Utf16CharMerger::from(self)
    }
}


/// Methods for iterating over `u8` and `u16` slices as UTF-8 or UTF-16 characters.
///
/// The iterators are slightly faster than the similar methods in [`IterExt`](trait.IterExt.html)
/// because they con "push back" items for free after errors and don't need a
/// separate buffer that must be checked on every call to `.next()`.
pub trait SliceExt: Index<RangeFull> {
    /// Decode `u8` slices as UTF-8 and iterate over the codepoints as `Utf8Char`s,
    ///
    /// # Examples
    ///
    /// Get the index and error type of the first error:
    #[cfg_attr(feature="std", doc=" ```")]
    #[cfg_attr(not(feature="std"), doc=" ```no_compile")]
    /// use encode_unicode::{SliceExt, Utf8Char};
    /// use encode_unicode::error::InvalidUtf8Slice;
    ///
    /// let slice = b"ab\0\xe0\xbc\xa9 \xf3\x80\x77";
    /// let result = slice.utf8char_indices()
    ///     .map(|(offset,r,length)| r.map_err(|e| (offset,e,length) ) )
    ///     .collect::<Result<String,(usize,InvalidUtf8Slice,usize)>>();
    ///
    /// assert_eq!(result, Err((7, InvalidUtf8Slice::TooShort(4), 1)));
    /// ```
    ///
    /// ```
    /// use encode_unicode::{SliceExt, Utf8Char};
    /// use std::error::Error;
    ///
    /// let slice = b"\xf0\xbf\xbf\xbfXY\xdd\xbb\xe1\x80\x99quux123";
    /// let mut fixed_size = [Utf8Char::default(); 8];
    /// for (cp_i, (byte_index, r, _)) in slice.utf8char_indices().enumerate().take(8) {
    ///     match r {
    ///         Ok(u8c) => fixed_size[cp_i] = u8c,
    ///         Err(e) => panic!("Invalid codepoint at index {} ({})", cp_i, e.description()),
    ///     }
    /// }
    /// let chars = ['\u{3ffff}', 'X', 'Y', '\u{77b}', '\u{1019}', 'q', 'u', 'u'];
    /// assert_eq!(fixed_size, chars);
    /// ```
    ///
    #[cfg_attr(feature="std", doc=" ```")]
    #[cfg_attr(not(feature="std"), doc=" ```no_compile")]
    /// use encode_unicode::{SliceExt, Utf8Char};
    /// use encode_unicode::error::InvalidUtf8Slice::*;
    /// use encode_unicode::error::{InvalidUtf8, InvalidUtf8FirstByte, InvalidCodepoint};
    ///
    /// let bytes = b"\xfa-\xf4\x8f\xee\xa1\x8f-\xed\xa9\x87\xf0\xcc\xbb";
    /// let mut errors = Vec::new();
    /// let mut lengths = Vec::new();
    /// let mut string = String::new();
    /// for (offset,result,length) in bytes.utf8char_indices() {
    ///     lengths.push((offset,length));
    ///     let c = result.unwrap_or_else(|error| {
    ///         errors.push((offset,error));
    ///         Utf8Char::from('\u{fffd}') // replacement character
    ///     });
    ///     string.push_str(c.as_str());
    /// }
    ///
    /// assert_eq!(string, "�-��\u{e84f}-����\u{33b}");
    /// assert_eq!(lengths, [(0,1), (1,1), (2,1), (3,1), (4,3), (7,1),
    ///                      (8,1), (9,1), (10,1), (11,1), (12,2)]);
    /// assert_eq!(errors, [
    ///     ( 0, Utf8(InvalidUtf8::FirstByte(InvalidUtf8FirstByte::TooLongSeqence))),
    ///     ( 2, Utf8(InvalidUtf8::NotAContinuationByte(2))),
    ///     ( 3, Utf8(InvalidUtf8::FirstByte(InvalidUtf8FirstByte::ContinuationByte))),
    ///     ( 8, Codepoint(InvalidCodepoint::Utf16Reserved)),
    ///     ( 9, Utf8(InvalidUtf8::FirstByte(InvalidUtf8FirstByte::ContinuationByte))),
    ///     (10, Utf8(InvalidUtf8::FirstByte(InvalidUtf8FirstByte::ContinuationByte))),
    ///     (11, TooShort(4)), // (but it was not the last element returned!)
    /// ]);
    /// ```
    fn utf8char_indices(&self) -> Utf8CharDecoder where Self::Output: Borrow<[u8]>;


    /// Decode `u16` slices as UTF-16 and iterate over the codepoints as `Utf16Char`s,
    ///
    /// The iterator produces `(usize,Result<Utf16Char,Utf16Error>,usize)`,
    /// and the slice is validated as you go.
    ///
    /// The first `usize` contains the offset from the start of the slice and
    /// the last `usize` contains the length of the codepoint or error.
    /// The length is either 1 or 2, and always 1 for errors.
    ///
    /// # Examples
    ///
    #[cfg_attr(feature="std", doc=" ```")]
    #[cfg_attr(not(feature="std"), doc=" ```no_compile")]
    /// use encode_unicode::{SliceExt, Utf8Char};
    ///
    /// let slice = &['a' as u16, 0xdf00, 0xd83c, 0xdca0][..];
    /// let mut errors = Vec::new();
    /// let string = slice.utf16char_indices().map(|(offset,r,_)| match r {
    ///     Ok(u16c) => Utf8Char::from(u16c),
    ///     Err(_) => {
    ///         errors.push(offset);
    ///         Utf8Char::from('\u{fffd}') // REPLACEMENT_CHARACTER
    ///     }
    /// }).collect::<String>();
    ///
    /// assert_eq!(string, "a�🂠");
    /// assert_eq!(errors, [1]);
    /// ```
    ///
    /// Search for a codepoint and return its unit and codepoint index.
    /// ```
    /// use encode_unicode::{SliceExt, Utf16Char};
    ///
    /// let slice = [0xd875,/*'𝕏'*/ 0xdd4f, '≈' as u16, '2' as u16];
    /// let position = slice.utf16char_indices()
    ///     .enumerate()
    ///     .find(|&(_,(_,r,_))| r == Ok(Utf16Char::from('≈')) )
    ///     .map(|(codepoint, (offset, _, _))| (codepoint, offset) );
    ///
    /// assert_eq!(position, Some((1,2)));
    /// ```
    ///
    /// Error types:
    /// ```
    /// use encode_unicode::{SliceExt, Utf16Char};
    /// use encode_unicode::error::Utf16PairError::*;
    ///
    /// let slice = [0xdcba, 0xdeff, 0xd8be, 0xdeee, 'λ' as u16, 0xdab1, 0xdab1];
    /// let mut iter = slice.utf16char_indices();
    /// assert_eq!(iter.next(), Some((0, Err(UnexpectedTrailingSurrogate), 1)));
    /// assert_eq!(iter.next(), Some((1, Err(UnexpectedTrailingSurrogate), 1)));
    /// assert_eq!(iter.next(), Some((2, Ok(Utf16Char::from('\u{3faee}')), 2)));
    /// assert_eq!(iter.next(), Some((4, Ok(Utf16Char::from('λ')), 1)));
    /// assert_eq!(iter.next(), Some((5, Err(UnmatchedLeadingSurrogate), 1)));
    /// assert_eq!(iter.next(), Some((6, Err(Incomplete), 1)));
    /// assert_eq!(iter.next(), None);
    /// assert_eq!(iter.as_slice(), [])
    /// ```
    fn utf16char_indices(&self) -> Utf16CharDecoder where Self::Output: Borrow<[u16]>;
}

impl<S: ?Sized+Index<RangeFull>> SliceExt for S {
    fn utf8char_indices(&self) -> Utf8CharDecoder where Self::Output: Borrow<[u8]> {
        Utf8CharDecoder::from(self[..].borrow())
    }
    fn utf16char_indices(&self) -> Utf16CharDecoder where Self::Output: Borrow<[u16]> {
        Utf16CharDecoder::from(self[..].borrow())
    }
}
#[cfg(test)]
mod tests_rug_1 {
    use super::*;
    
    #[test]
    fn test_overlong() {
        let mut p0: u8 = 0xc0;
        let mut p1: u8 = 0x80;

        assert_eq!(crate::traits::overlong(p0, p1), true);
    }
}#[cfg(test)]
mod tests_rug_2 {
    use super::*;

    #[test]
    fn test_rug() {
        let p0: &[u8] = &[0b1100_0010, 0b1011_0010];

        crate::traits::merge_nonascii_unchecked_utf8(p0);
    }
}#[cfg(test)]
mod tests_rug_3 {
    use super::*;
    
    #[test]
    fn test_combine_surrogates() {
        let mut p0: u16 = 0xD83D;
        let mut p1: u16 = 0xDC69;

        crate::traits::combine_surrogates(p0, p1);
    }
}#[cfg(test)]
mod tests_rug_4 {
    use super::*;
    use crate::U8UtfExt;
    use crate::error::InvalidUtf8FirstByte;

    #[test]
    fn test_extra_utf8_bytes() {
        let mut p0: u8 = 198;

        assert_eq!(p0.extra_utf8_bytes(), Ok(2));
    }
}#[cfg(test)]
mod tests_rug_5 {
    use super::*;
    use crate::U8UtfExt;

    #[test]
    fn test_rug() {
        let mut p0: u8 = 0b1101_0010;

        <u8 as U8UtfExt>::extra_utf8_bytes_unchecked(p0);
    }
}
#[cfg(test)]
mod tests_rug_6 {
    use super::*;
    use crate::U16UtfExt;

    #[test]
    fn test_utf16_needs_extra_unit() {
        let mut p0: u16 = 0x1234;

        assert_eq!(p0.utf16_needs_extra_unit().unwrap(), false);
    }
}
#[cfg(test)]
mod tests_rug_7 {
    use super::*;
    use crate::traits::U16UtfExt;
    
    #[test]
    fn test_rug() {
        let mut p0: u16 = 0xd801;

        assert_eq!(p0.is_utf16_leading_surrogate(), false);
    }
}#[cfg(test)]
mod tests_rug_8 {
    use super::*;
    use crate::CharExt;

    #[test]
    fn test_to_utf8() {
        let p0: char = 'a';

        <char as CharExt>::to_utf8(p0);
    }
}
#[cfg(test)]
mod tests_rug_9 {
    use super::*;
    use crate::CharExt;
    use crate::Utf8Iterator;

    #[test]
    fn test_rug() {
        let p0: char = 'a';

        <char as CharExt>::iter_utf8_bytes(p0);
    }
}
#[cfg(test)]
mod tests_rug_10 {
    use super::*;
    use crate::CharExt;

    #[test]
    fn test_rug() {
        // Sample data for initializing the argument
        let p0: char = '€';
        
        <char>::to_utf8_array(p0);
    }
}#[cfg(test)]
mod tests_rug_11 {
    use super::*;
    use crate::CharExt;
    use crate::errors::InvalidUtf8;
    use crate::errors::InvalidUtf8Slice;
    
    #[test]
    fn test_rug() {
        let mut p0: &[u8] = b"Hello";

        <char>::from_utf8_slice_start(p0);

    }
}#[cfg(test)]
mod tests_rug_12 {
    use super::*;
    use crate::{CharExt, errors::InvalidUtf8::*};

    #[test]
    fn test_rug() {
        let mut p0: [u8; 4] = [0xE2, 0x82, 0xAC, 0x00]; // Sample UTF-8 data for testing

        <char as CharExt>::from_utf8_array(p0).unwrap();

    }
}#[cfg(test)]
mod tests_rug_14 {
    use super::*;
    use crate::CharExt;

    #[test]
    fn test_rug() {
        let mut p0: char = 'A';

        <char>::to_utf16(p0);
    }
}
#[cfg(test)]
mod tests_rug_15 {
    use super::*;
    use crate::{CharExt, Utf16Iterator};

    #[test]
    fn test_iter_utf16_units() {
        let p0: char = 'A';

        p0.iter_utf16_units();
    }
}
#[cfg(test)]
mod tests_rug_16 {
    use super::*;
    use crate::CharExt;

    #[test]
    fn test_to_utf16_array() {
        let mut p0: char = 'A';

        <char>::to_utf16_array(p0);
    }
}#[cfg(test)]
mod tests_rug_17 {
    use super::*;
    use crate::CharExt;

    #[test]
    fn test_rug() {
        let mut p0: char = '😊';

        <char>::to_utf16_tuple(p0);
    }
}#[cfg(test)]
mod tests_rug_19 {
    use super::*;
    use crate::CharExt;
    use crate::errors::InvalidUtf16Array;

    #[test]
    fn test_from_utf16_array() {
        let utf16_input: [u16; 2] = [0xd801, 0xdc01];

        assert_eq!(<char as CharExt>::from_utf16_array(utf16_input), Ok('𐐁'));
    }
}#[cfg(test)]
mod tests_rug_20 {
    use super::*;
    use crate::{CharExt, errors::InvalidUtf16Tuple};

    use std::option::Option;

    #[test]
    fn test_rug() {
        let mut p0: (u16, Option<u16>) = (0x0041, None);

        <char>::from_utf16_tuple(p0).unwrap();

    }
}#[cfg(test)]
mod tests_rug_21 {
    use super::*;
    use crate::CharExt;

    #[test]
    fn test_from_utf16_array_unchecked() {
        let p0: [u16; 2] = [0x0041, 0x0042];

        <char as CharExt>::from_utf16_array_unchecked(p0);
    }
}#[cfg(test)]
mod tests_rug_24 {
    use super::*;
    use crate::StrExt;

    #[test]
    fn test_rug() {
        let mut p0: &str = "Hello, 世界!";

        p0.utf8chars();
    }
}#[cfg(test)]
mod tests_rug_25 {
    use super::*;
    use crate::traits::StrExt; // assuming the traits module is defined in the crate

    #[test]
    fn test_utf16chars() {
        let p0: &str = "Hello, 世界!"; // Sample data for &str

        p0.utf16chars();
    }
}#[cfg(test)]
mod tests_rug_29 {
    use super::*;
    use crate::IterExt;
    use crate::iterator::Utf16Iterator;
    use crate::Utf16Char;

    #[test]
    fn test_rug() {
        let mut v10 = Utf16Iterator::from('A');
        let result: Vec<u16> = v10.collect();
        assert_eq!(result, vec![65, 0]);
        let splitted: Utf16Iterator = Utf16Char::from('A').into();
        let result2: Vec<u16> = splitted.collect();
        assert_eq!(result2, vec![65, 0]);
    }
}
#[cfg(test)]
mod tests_rug_31 {
    use super::*;
    use crate::IterExt;
    use crate::iterator::Utf16Iterator;

    #[test]
    fn test_rug() {
        let mut p0 = Utf16Iterator::from('A');
                
        p0.to_utf16chars();

    }
}
