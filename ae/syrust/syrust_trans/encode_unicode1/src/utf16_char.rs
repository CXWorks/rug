use utf16_iterators::Utf16Iterator;
use traits::{CharExt, U16UtfExt};
use utf8_char::Utf8Char;
use errors::{InvalidUtf16Slice, InvalidUtf16Array, InvalidUtf16Tuple};
use errors::{NonBMPError, EmptyStrError, FromStrError};
extern crate core;
use self::core::{hash, fmt};
use self::core::cmp::Ordering;
use self::core::borrow::Borrow;
use self::core::ops::Deref;
use self::core::str::FromStr;
#[cfg(feature = "std")]
use self::core::iter::FromIterator;
#[cfg(feature = "std")]
#[allow(deprecated)]
use std::ascii::AsciiExt;
#[cfg(feature = "ascii")]
use self::core::char;
#[cfg(feature = "ascii")]
extern crate ascii;
#[cfg(feature = "ascii")]
use self::ascii::{AsciiChar, ToAsciiChar, ToAsciiCharError};
#[derive(Default)]
#[derive(PartialEq, Eq)]
#[derive(Clone, Copy)]
/// An unicode codepoint stored as UTF-16.
///
/// It can be borrowed as an `u16` slice, and has the same size as `char`.
pub struct Utf16Char {
    units: [u16; 2],
}
impl FromStr for Utf16Char {
    type Err = FromStrError;
    /// Create an `Utf16Char` from a string slice.
    /// The string must contain exactly one codepoint.
    ///
    /// # Examples
    ///
    /// ```
    /// use encode_unicode::error::FromStrError::*;
    /// use encode_unicode::Utf16Char;
    /// use std::str::FromStr;
    ///
    /// assert_eq!(Utf16Char::from_str("a"), Ok(Utf16Char::from('a')));
    /// assert_eq!(Utf16Char::from_str("🂠"), Ok(Utf16Char::from('🂠')));
    /// assert_eq!(Utf16Char::from_str(""), Err(Empty));
    /// assert_eq!(Utf16Char::from_str("ab"), Err(MultipleCodepoints));
    /// assert_eq!(Utf16Char::from_str("é"), Err(MultipleCodepoints));// 'e'+u301 combining mark
    /// ```
    fn from_str(s: &str) -> Result<Self, FromStrError> {
        match Utf16Char::from_str_start(s) {
            Ok((u16c, bytes)) if bytes == s.len() => Ok(u16c),
            Ok((_, _)) => Err(FromStrError::MultipleCodepoints),
            Err(EmptyStrError) => Err(FromStrError::Empty),
        }
    }
}
impl From<char> for Utf16Char {
    fn from(c: char) -> Self {
        let (first, second) = c.to_utf16_tuple();
        Utf16Char {
            units: [first, second.unwrap_or(0)],
        }
    }
}
impl From<Utf8Char> for Utf16Char {
    fn from(utf8: Utf8Char) -> Utf16Char {
        let (b, utf8_len) = utf8.to_array();
        match utf8_len {
            1 => {
                Utf16Char {
                    units: [b[0] as u16, 0],
                }
            }
            4 => {
                let mut first = 0xd800 - (0x01_00_00u32 >> 10) as u16;
                first += (b[0] as u16 & 0x07) << 8;
                first += (b[1] as u16 & 0x3f) << 2;
                first += (b[2] as u16 & 0x30) >> 4;
                let mut second = 0xdc00;
                second |= (b[2] as u16 & 0x0f) << 6;
                second |= b[3] as u16 & 0x3f;
                Utf16Char {
                    units: [first, second],
                }
            }
            _ => {
                let mut unit = ((b[0] as u16 & 0x1f) << 6) | (b[1] as u16 & 0x3f);
                if utf8_len == 3 {
                    unit = (unit << 6) | (b[2] as u16 & 0x3f);
                }
                Utf16Char { units: [unit, 0] }
            }
        }
    }
}
impl From<Utf16Char> for char {
    fn from(uc: Utf16Char) -> char {
        char::from_utf16_array_unchecked(uc.to_array())
    }
}
impl IntoIterator for Utf16Char {
    type Item = u16;
    type IntoIter = Utf16Iterator;
    /// Iterate over the units.
    fn into_iter(self) -> Utf16Iterator {
        Utf16Iterator::from(self)
    }
}
#[cfg(feature = "std")]
impl Extend<Utf16Char> for Vec<u16> {
    fn extend<I: IntoIterator<Item = Utf16Char>>(&mut self, iter: I) {
        let iter = iter.into_iter();
        self.reserve(iter.size_hint().0);
        for u16c in iter {
            self.push(u16c.units[0]);
            if u16c.units[1] != 0 {
                self.push(u16c.units[1]);
            }
        }
    }
}
#[cfg(feature = "std")]
impl<'a> Extend<&'a Utf16Char> for Vec<u16> {
    fn extend<I: IntoIterator<Item = &'a Utf16Char>>(&mut self, iter: I) {
        self.extend(iter.into_iter().cloned())
    }
}
#[cfg(feature = "std")]
impl FromIterator<Utf16Char> for Vec<u16> {
    fn from_iter<I: IntoIterator<Item = Utf16Char>>(iter: I) -> Self {
        let mut vec = Vec::new();
        vec.extend(iter);
        return vec;
    }
}
#[cfg(feature = "std")]
impl<'a> FromIterator<&'a Utf16Char> for Vec<u16> {
    fn from_iter<I: IntoIterator<Item = &'a Utf16Char>>(iter: I) -> Self {
        Self::from_iter(iter.into_iter().cloned())
    }
}
#[cfg(feature = "std")]
impl Extend<Utf16Char> for String {
    fn extend<I: IntoIterator<Item = Utf16Char>>(&mut self, iter: I) {
        self.extend(iter.into_iter().map(|u16c| Utf8Char::from(u16c)));
    }
}
#[cfg(feature = "std")]
impl<'a> Extend<&'a Utf16Char> for String {
    fn extend<I: IntoIterator<Item = &'a Utf16Char>>(&mut self, iter: I) {
        self.extend(iter.into_iter().cloned());
    }
}
#[cfg(feature = "std")]
impl FromIterator<Utf16Char> for String {
    fn from_iter<I: IntoIterator<Item = Utf16Char>>(iter: I) -> Self {
        let mut s = String::new();
        s.extend(iter);
        return s;
    }
}
#[cfg(feature = "std")]
impl<'a> FromIterator<&'a Utf16Char> for String {
    fn from_iter<I: IntoIterator<Item = &'a Utf16Char>>(iter: I) -> Self {
        Self::from_iter(iter.into_iter().cloned())
    }
}
impl AsRef<[u16]> for Utf16Char {
    #[inline]
    fn as_ref(&self) -> &[u16] {
        &self.units[..self.len()]
    }
}
impl Borrow<[u16]> for Utf16Char {
    #[inline]
    fn borrow(&self) -> &[u16] {
        self.as_ref()
    }
}
impl Deref for Utf16Char {
    type Target = [u16];
    #[inline]
    fn deref(&self) -> &[u16] {
        self.as_ref()
    }
}
#[cfg(feature = "std")]
#[allow(deprecated)]
impl AsciiExt for Utf16Char {
    type Owned = Self;
    fn is_ascii(&self) -> bool {
        self.units[0] < 128
    }
    fn eq_ignore_ascii_case(&self, other: &Self) -> bool {
        self.to_ascii_lowercase() == other.to_ascii_lowercase()
    }
    fn to_ascii_uppercase(&self) -> Self {
        let n = self.units[0].wrapping_sub(b'a' as u16);
        if n < 26 {
            Utf16Char {
                units: [n + b'A' as u16, 0],
            }
        } else {
            *self
        }
    }
    fn to_ascii_lowercase(&self) -> Self {
        let n = self.units[0].wrapping_sub(b'A' as u16);
        if n < 26 {
            Utf16Char {
                units: [n + b'a' as u16, 0],
            }
        } else {
            *self
        }
    }
    fn make_ascii_uppercase(&mut self) {
        *self = self.to_ascii_uppercase();
    }
    fn make_ascii_lowercase(&mut self) {
        *self = self.to_ascii_lowercase();
    }
}
#[cfg(feature = "ascii")]
/// Requires the feature "ascii".
impl From<AsciiChar> for Utf16Char {
    #[inline]
    fn from(ac: AsciiChar) -> Self {
        Utf16Char {
            units: [ac.as_byte() as u16, 0],
        }
    }
}
#[cfg(feature = "ascii")]
/// Requires the feature "ascii".
impl ToAsciiChar for Utf16Char {
    #[inline]
    fn to_ascii_char(self) -> Result<AsciiChar, ToAsciiCharError> {
        if self.is_ascii() { self.units[0] as u8 } else { 255 }.to_ascii_char()
    }
    #[inline]
    unsafe fn to_ascii_char_unchecked(self) -> AsciiChar {
        (self.units[0] as u8).to_ascii_char_unchecked()
    }
}
impl hash::Hash for Utf16Char {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.to_char().hash(state);
    }
}
impl fmt::Debug for Utf16Char {
    fn fmt(&self, fmtr: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.to_char(), fmtr)
    }
}
impl fmt::Display for Utf16Char {
    fn fmt(&self, fmtr: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&Utf8Char::from(*self), fmtr)
    }
}
impl PartialOrd for Utf16Char {
    #[inline]
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        Some(self.cmp(rhs))
    }
}
impl Ord for Utf16Char {
    #[inline]
    fn cmp(&self, rhs: &Self) -> Ordering {
        let lhs = (self.units[0] as u32, self.units[1] as u32);
        let rhs = (rhs.units[0] as u32, rhs.units[1] as u32);
        let lhs = (lhs.0 << (lhs.1 >> 12)) + lhs.1;
        let rhs = (rhs.0 << (rhs.1 >> 12)) + rhs.1;
        lhs.cmp(&rhs)
    }
}
impl PartialEq<char> for Utf16Char {
    fn eq(&self, u32c: &char) -> bool {
        *self == Utf16Char::from(*u32c)
    }
}
impl PartialEq<Utf16Char> for char {
    fn eq(&self, u16c: &Utf16Char) -> bool {
        Utf16Char::from(*self) == *u16c
    }
}
impl PartialOrd<char> for Utf16Char {
    fn partial_cmp(&self, u32c: &char) -> Option<Ordering> {
        self.partial_cmp(&Utf16Char::from(*u32c))
    }
}
impl PartialOrd<Utf16Char> for char {
    fn partial_cmp(&self, u16c: &Utf16Char) -> Option<Ordering> {
        Utf16Char::from(*self).partial_cmp(u16c)
    }
}
impl PartialEq<Utf8Char> for Utf16Char {
    fn eq(&self, u8c: &Utf8Char) -> bool {
        *self == Utf16Char::from(*u8c)
    }
}
impl PartialOrd<Utf8Char> for Utf16Char {
    fn partial_cmp(&self, u8c: &Utf8Char) -> Option<Ordering> {
        self.partial_cmp(&Utf16Char::from(*u8c))
    }
}
/// Only considers the unit equal if the codepoint of the `Utf16Char` is not
/// made up of a surrogate pair.
///
/// There is no impl in the opposite direction, as this should only be used to
/// compare `Utf16Char`s against constants.
///
/// # Examples
///
/// ```
/// # use encode_unicode::Utf16Char;
/// assert!(Utf16Char::from('6') == b'6' as u16);
/// assert!(Utf16Char::from('\u{FFFF}') == 0xffff_u16);
/// assert!(Utf16Char::from_tuple((0xd876, Some(0xdef9))).unwrap() != 0xd876_u16);
/// ```
impl PartialEq<u16> for Utf16Char {
    fn eq(&self, unit: &u16) -> bool {
        self.units[0] == *unit && self.units[1] == 0
    }
}
/// Only considers the byte equal if the codepoint of the `Utf16Char` is <= U+FF.
///
/// # Examples
///
/// ```
/// # use encode_unicode::Utf16Char;
/// assert!(Utf16Char::from('6') == b'6');
/// assert!(Utf16Char::from('\u{00FF}') == b'\xff');
/// assert!(Utf16Char::from('\u{0100}') != b'\0');
/// ```
impl PartialEq<u8> for Utf16Char {
    fn eq(&self, byte: &u8) -> bool {
        self.units[0] == *byte as u16
    }
}
#[cfg(feature = "ascii")]
/// `Utf16Char`s that are not ASCII never compare equal.
impl PartialEq<AsciiChar> for Utf16Char {
    #[inline]
    fn eq(&self, ascii: &AsciiChar) -> bool {
        self.units[0] == *ascii as u16
    }
}
#[cfg(feature = "ascii")]
/// `Utf16Char`s that are not ASCII never compare equal.
impl PartialEq<Utf16Char> for AsciiChar {
    #[inline]
    fn eq(&self, u16c: &Utf16Char) -> bool {
        *self as u16 == u16c.units[0]
    }
}
#[cfg(feature = "ascii")]
/// `Utf16Char`s that are not ASCII always compare greater.
impl PartialOrd<AsciiChar> for Utf16Char {
    #[inline]
    fn partial_cmp(&self, ascii: &AsciiChar) -> Option<Ordering> {
        self.units[0].partial_cmp(&(*ascii as u16))
    }
}
#[cfg(feature = "ascii")]
/// `Utf16Char`s that are not ASCII always compare greater.
impl PartialOrd<Utf16Char> for AsciiChar {
    #[inline]
    fn partial_cmp(&self, u16c: &Utf16Char) -> Option<Ordering> {
        (*self as u16).partial_cmp(&u16c.units[0])
    }
}
impl Utf16Char {
    /// Create an `Utf16Char` from the first codepoint in a string slice,
    /// converting from UTF-8 to UTF-16.
    ///
    /// The returned `usize` is the number of UTF-8 bytes used from the str,
    /// and not the number of UTF-16 units.
    ///
    /// Returns an error if the `str` is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use encode_unicode::Utf16Char;
    ///
    /// assert_eq!(Utf16Char::from_str_start("a"), Ok((Utf16Char::from('a'),1)));
    /// assert_eq!(Utf16Char::from_str_start("ab"), Ok((Utf16Char::from('a'),1)));
    /// assert_eq!(Utf16Char::from_str_start("🂠 "), Ok((Utf16Char::from('🂠'),4)));
    /// assert_eq!(Utf16Char::from_str_start("é"), Ok((Utf16Char::from('e'),1)));// 'e'+u301 combining mark
    /// assert!(Utf16Char::from_str_start("").is_err());
    /// ```
    pub fn from_str_start(s: &str) -> Result<(Self, usize), EmptyStrError> {
        if s.is_empty() {
            return Err(EmptyStrError);
        }
        let b = s.as_bytes();
        match b[0] {
            0..=127 => {
                let unit = b[0] as u16;
                Ok((Utf16Char { units: [unit, 0] }, 1))
            }
            0b1000_0000..=0b1101_1111 => {
                let unit = (((b[1] & 0x3f) as u16) << 0) | (((b[0] & 0x1f) as u16) << 6);
                Ok((Utf16Char { units: [unit, 0] }, 2))
            }
            0b1110_0000..=0b1110_1111 => {
                let unit = (((b[2] & 0x3f) as u16) << 0) | (((b[1] & 0x3f) as u16) << 6)
                    | (((b[0] & 0x0f) as u16) << 12);
                Ok((Utf16Char { units: [unit, 0] }, 3))
            }
            _ => {
                let second = 0xdc00 | (((b[3] & 0x3f) as u16) << 0)
                    | (((b[2] & 0x0f) as u16) << 6);
                let first = 0xd800 - (0x01_00_00u32 >> 10) as u16
                    + (((b[2] & 0x30) as u16) >> 4) + (((b[1] & 0x3f) as u16) << 2)
                    + (((b[0] & 0x07) as u16) << 8);
                Ok((
                    Utf16Char {
                        units: [first, second],
                    },
                    4,
                ))
            }
        }
    }
    /// Validate and store the first UTF-16 codepoint in the slice.
    /// Also return how many units were needed.
    pub fn from_slice_start(src: &[u16]) -> Result<(Self, usize), InvalidUtf16Slice> {
        char::from_utf16_slice_start(src)
            .map(|(_, len)| {
                let second = if len == 2 { src[1] } else { 0 };
                (
                    Utf16Char {
                        units: [src[0], second],
                    },
                    len,
                )
            })
    }
    /// Store the first UTF-16 codepoint of the slice.
    ///
    /// # Safety
    ///
    /// The slice must be non-empty and start with a valid UTF-16 codepoint.
    /// The length of the slice is never checked.
    pub unsafe fn from_slice_start_unchecked(src: &[u16]) -> (Self, usize) {
        let first = *src.get_unchecked(0);
        if first.is_utf16_leading_surrogate() {
            (
                Utf16Char {
                    units: [first, *src.get_unchecked(1)],
                },
                2,
            )
        } else {
            (Utf16Char { units: [first, 0] }, 1)
        }
    }
    /// Validate and store an UTF-16 array as returned from `char.to_utf16_array()`.
    ///
    /// # Examples
    ///
    /// ```
    /// use encode_unicode::Utf16Char;
    /// use encode_unicode::error::InvalidUtf16Array;
    ///
    /// assert_eq!(Utf16Char::from_array(['x' as u16, 'y' as u16]), Ok(Utf16Char::from('x')));
    /// assert_eq!(Utf16Char::from_array(['睷' as u16, 0]), Ok(Utf16Char::from('睷')));
    /// assert_eq!(Utf16Char::from_array([0xda6f, 0xdcde]), Ok(Utf16Char::from('\u{abcde}')));
    /// assert_eq!(Utf16Char::from_array([0xf111, 0xdbad]), Ok(Utf16Char::from('\u{f111}')));
    /// assert_eq!(Utf16Char::from_array([0xdaaf, 0xdaaf]), Err(InvalidUtf16Array::SecondIsNotTrailingSurrogate));
    /// assert_eq!(Utf16Char::from_array([0xdcac, 0x9000]), Err(InvalidUtf16Array::FirstIsTrailingSurrogate));
    /// ```
    pub fn from_array(units: [u16; 2]) -> Result<Self, InvalidUtf16Array> {
        if (units[0] & 0xf8_00) != 0xd8_00 {
            Ok(Utf16Char { units: [units[0], 0] })
        } else if units[0] < 0xdc_00 && (units[1] & 0xfc_00) == 0xdc_00 {
            Ok(Utf16Char { units: units })
        } else if units[0] < 0xdc_00 {
            Err(InvalidUtf16Array::SecondIsNotTrailingSurrogate)
        } else {
            Err(InvalidUtf16Array::FirstIsTrailingSurrogate)
        }
    }
    /// Create an `Utf16Char` from an array as returned from `char.to_utf16_array()`.
    ///
    /// # Safety
    ///
    /// The units must form a valid codepoint, and the second unit must be 0
    /// when a surrogate pair is not required.
    /// Violating this can easily lead to undefined behavior, although unlike
    /// `char` bad `Utf16Char`s simply existing is not immediately UB.
    pub unsafe fn from_array_unchecked(units: [u16; 2]) -> Self {
        Utf16Char { units: units }
    }
    /// Validate and store a UTF-16 pair as returned from `char.to_utf16_tuple()`.
    pub fn from_tuple(utf16: (u16, Option<u16>)) -> Result<Self, InvalidUtf16Tuple> {
        unsafe {
            char::from_utf16_tuple(utf16).map(|_| Self::from_tuple_unchecked(utf16))
        }
    }
    /// Create an `Utf16Char` from a tuple as returned from `char.to_utf16_tuple()`.
    ///
    /// # Safety
    ///
    /// The units must form a valid codepoint with the second being 0 when a
    /// surrogate pair is not required.
    /// Violating this can easily lead to undefined behavior.
    pub unsafe fn from_tuple_unchecked(utf16: (u16, Option<u16>)) -> Self {
        Utf16Char {
            units: [utf16.0, utf16.1.unwrap_or(0)],
        }
    }
    /// Create an `Utf16Char` from a single unit.
    ///
    /// Codepoints < '\u{1_0000}' (which fit in a `u16`) are part of the basic
    /// multilingual plane unless they are reserved for surrogate pairs.
    ///
    /// # Errors
    ///
    /// Returns `NonBMPError` if the unit is in the range `0xd800..0xe000`
    /// (which means that it's part of a surrogat pair)
    ///
    /// # Examples
    ///
    /// ```
    /// # use encode_unicode::Utf16Char;
    /// assert_eq!(Utf16Char::from_bmp(0x40).unwrap(), '@');
    /// assert_eq!(Utf16Char::from_bmp('ø' as u16).unwrap(), 'ø');
    /// assert!(Utf16Char::from_bmp(0xdddd).is_err());
    /// ```
    pub fn from_bmp(bmp_codepoint: u16) -> Result<Self, NonBMPError> {
        if bmp_codepoint & 0xf800 != 0xd800 {
            Ok(Utf16Char {
                units: [bmp_codepoint, 0],
            })
        } else {
            Err(NonBMPError)
        }
    }
    /// Create an `Utf16Char` from a single unit without checking that it's a
    /// valid codepoint on its own.
    ///
    /// # Safety
    ///
    /// The unit must be less than 0xd800 or greater than 0xdfff.
    /// In other words, not part of a surrogate pair.
    /// Violating this can easily lead to undefined behavior.
    #[inline]
    pub unsafe fn from_bmp_unchecked(bmp_codepoint: u16) -> Self {
        Utf16Char {
            units: [bmp_codepoint, 0],
        }
    }
    /// Checks that the codepoint is in the basic multilingual plane.
    ///
    /// # Examples
    /// ```
    /// # use encode_unicode::Utf16Char;
    /// assert_eq!(Utf16Char::from('e').is_bmp(), true);
    /// assert_eq!(Utf16Char::from('€').is_bmp(), true);
    /// assert_eq!(Utf16Char::from('𝔼').is_bmp(), false);
    /// ```
    #[inline]
    pub fn is_bmp(&self) -> bool {
        self.units[1] == 0
    }
    /// The number of units this character is made up of.
    ///
    /// Is either 1 or 2 and identical to `.as_char().len_utf16()`
    /// or `.as_ref().len()`.
    #[inline]
    pub fn len(self) -> usize {
        1 + (self.units[1] as usize >> 15)
    }
    /// Checks that the codepoint is an ASCII character.
    #[inline]
    pub fn is_ascii(&self) -> bool {
        self.units[0] <= 127
    }
    /// Checks that two characters are an ASCII case-insensitive match.
    ///
    /// Is equivalent to `a.to_ascii_lowercase() == b.to_ascii_lowercase()`.
    #[cfg(feature = "std")]
    pub fn eq_ignore_ascii_case(&self, other: &Self) -> bool {
        self.to_ascii_lowercase() == other.to_ascii_lowercase()
    }
    /// Converts the character to its ASCII upper case equivalent.
    ///
    /// ASCII letters 'a' to 'z' are mapped to 'A' to 'Z',
    /// but non-ASCII letters are unchanged.
    #[cfg(feature = "std")]
    pub fn to_ascii_uppercase(&self) -> Self {
        let n = self.units[0].wrapping_sub(b'a' as u16);
        if n < 26 {
            Utf16Char {
                units: [n + b'A' as u16, 0],
            }
        } else {
            *self
        }
    }
    /// Converts the character to its ASCII lower case equivalent.
    ///
    /// ASCII letters 'A' to 'Z' are mapped to 'a' to 'z',
    /// but non-ASCII letters are unchanged.
    #[cfg(feature = "std")]
    pub fn to_ascii_lowercase(&self) -> Self {
        let n = self.units[0].wrapping_sub(b'A' as u16);
        if n < 26 {
            Utf16Char {
                units: [n + b'a' as u16, 0],
            }
        } else {
            *self
        }
    }
    /// Converts the character to its ASCII upper case equivalent in-place.
    ///
    /// ASCII letters 'a' to 'z' are mapped to 'A' to 'Z',
    /// but non-ASCII letters are unchanged.
    #[cfg(feature = "std")]
    pub fn make_ascii_uppercase(&mut self) {
        *self = self.to_ascii_uppercase();
    }
    /// Converts the character to its ASCII lower case equivalent in-place.
    ///
    /// ASCII letters 'A' to 'Z' are mapped to 'a' to 'z',
    /// but non-ASCII letters are unchanged.
    #[cfg(feature = "std")]
    pub fn make_ascii_lowercase(&mut self) {
        *self = self.to_ascii_lowercase();
    }
    /// Convert from UTF-16 to UTF-32
    pub fn to_char(self) -> char {
        self.into()
    }
    /// Write the internal representation to a slice,
    /// and then returns the number of `u16`s written.
    ///
    /// # Panics
    /// Will panic the buffer is too small;
    /// You can get the required length from `.len()`,
    /// but a buffer of length two is always large enough.
    pub fn to_slice(self, dst: &mut [u16]) -> usize {
        let extra = self.units[1] as usize >> 15;
        match dst.get_mut(extra) {
            Some(first) => *first = self.units[extra],
            None => panic!("The provided buffer is too small."),
        }
        if extra != 0 {
            dst[0] = self.units[0];
        }
        extra + 1
    }
    /// Get the character represented as an array of two units.
    ///
    /// The second `u16` is zero for codepoints that fit in one unit.
    #[inline]
    pub fn to_array(self) -> [u16; 2] {
        self.units
    }
    /// The second `u16` is used for surrogate pairs.
    #[inline]
    pub fn to_tuple(self) -> (u16, Option<u16>) {
        (self.units[0], if self.units[1] == 0 { None } else { Some(self.units[1]) })
    }
}
#[cfg(test)]
mod tests_rug_149 {
    use super::*;
    use crate::error::FromStrError::*;
    use crate::Utf16Char;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_149_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "a";
        let p0: &str = rug_fuzz_0;
        debug_assert_eq!(Utf16Char::from_str(& p0), Ok(Utf16Char::from('a')));
        let _rug_ed_tests_rug_149_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_150 {
    use super::*;
    use crate::utf16_char::Utf16Char;
    use std::convert::From;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_150_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 'A';
        let p0: char = rug_fuzz_0;
        <Utf16Char as From<char>>::from(p0);
        let _rug_ed_tests_rug_150_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_152 {
    use super::*;
    use crate::Utf16Char;
    use crate::std::convert::From;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_152_rrrruuuugggg_test_rug = 0;
        let mut p0 = Utf16Char::default();
        <char>::from(p0);
        let _rug_ed_tests_rug_152_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_153 {
    use super::*;
    use crate::Utf16Char;
    use crate::std::iter::IntoIterator;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_153_rrrruuuugggg_test_rug = 0;
        let mut p0 = Utf16Char::default();
        <Utf16Char as IntoIterator>::into_iter(p0);
        let _rug_ed_tests_rug_153_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_154 {
    use super::*;
    use crate::std::iter::Extend;
    use crate::Utf16Char;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_154_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 97;
        let rug_fuzz_1 = 98;
        let mut p0: std::vec::Vec<u16> = Vec::new();
        let mut p1 = {
            let mut v7 = Utf16Char::default();
            v7.units = [rug_fuzz_0, rug_fuzz_1];
            vec![v7]
        };
        std::vec::Vec::<u16>::extend(&mut p0, p1);
        debug_assert_eq!(p0, vec![97, 98]);
        let _rug_ed_tests_rug_154_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_156 {
    use super::*;
    use crate::std::iter::FromIterator;
    use crate::Utf16Char;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_156_rrrruuuugggg_test_rug = 0;
        let mut p0 = vec![Utf16Char::default(), Utf16Char::default()];
        <std::vec::Vec<u16>>::from_iter(p0);
        let _rug_ed_tests_rug_156_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_157 {
    use super::*;
    use crate::std::iter::FromIterator;
    use crate::Utf16Char;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_157_rrrruuuugggg_test_rug = 0;
        let mut v7 = Utf16Char::default();
        let p0 = vec![& v7];
        <std::vec::Vec<u16>>::from_iter(p0);
        let _rug_ed_tests_rug_157_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_158 {
    use super::*;
    use crate::{Utf16Char, Utf8Char};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_158_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "hello";
        let mut p0 = String::from(rug_fuzz_0);
        let mut p1 = Utf16Char::default();
        <std::string::String>::extend(&mut p0, std::iter::once(p1));
        let _rug_ed_tests_rug_158_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_159 {
    use super::*;
    use crate::std::iter::Extend;
    use crate::Utf16Char;
    use std::string::String;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_159_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "Hello, ";
        let mut p0 = String::from(rug_fuzz_0);
        let mut p1 = Utf16Char::default();
        <String>::extend(&mut p0, std::iter::once(&p1));
        debug_assert_eq!(p0, "Hello, ");
        let _rug_ed_tests_rug_159_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_160 {
    use super::*;
    use crate::std::iter::FromIterator;
    use crate::Utf16Char;
    #[test]
    fn test_from_iter() {
        let _rug_st_tests_rug_160_rrrruuuugggg_test_from_iter = 0;
        let p0 = vec![Utf16Char::default(), Utf16Char::default()];
        <std::string::String>::from_iter(p0);
        let _rug_ed_tests_rug_160_rrrruuuugggg_test_from_iter = 0;
    }
}
#[cfg(test)]
mod tests_rug_161 {
    use super::*;
    use crate::utf16_char::Utf16Char;
    use crate::std::iter::FromIterator;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_161_rrrruuuugggg_test_rug = 0;
        let mut p0 = vec![Utf16Char::default()];
        <std::string::String>::from_iter(p0);
        let _rug_ed_tests_rug_161_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_162 {
    use super::*;
    use crate::Utf16Char;
    use crate::std::convert::AsRef;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_162_rrrruuuugggg_test_rug = 0;
        let mut p0 = Utf16Char::default();
        <Utf16Char as std::convert::AsRef<[u16]>>::as_ref(&p0);
        let _rug_ed_tests_rug_162_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_163 {
    use super::*;
    use crate::std::borrow::Borrow;
    use crate::Utf16Char;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_163_rrrruuuugggg_test_rug = 0;
        let mut p0 = Utf16Char::default();
        <Utf16Char as Borrow<[u16]>>::borrow(&p0);
        let _rug_ed_tests_rug_163_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_164 {
    use super::*;
    use crate::Utf16Char;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_164_rrrruuuugggg_test_rug = 0;
        let mut p0 = Utf16Char::default();
        <Utf16Char as std::ops::Deref>::deref(&p0);
        let _rug_ed_tests_rug_164_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_165 {
    use super::*;
    use crate::std::ascii::AsciiExt;
    use crate::Utf16Char;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_165_rrrruuuugggg_test_rug = 0;
        let mut p0 = Utf16Char::default();
        debug_assert_eq!(p0.is_ascii(), true);
        let _rug_ed_tests_rug_165_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_166 {
    use super::*;
    use crate::std::ascii::AsciiExt;
    use crate::Utf16Char;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_166_rrrruuuugggg_test_rug = 0;
        let mut p0 = Utf16Char::default();
        let mut p1 = Utf16Char::default();
        <Utf16Char as std::ascii::AsciiExt>::eq_ignore_ascii_case(&p0, &p1);
        let _rug_ed_tests_rug_166_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_167 {
    use super::*;
    use crate::Utf16Char;
    use crate::std::ascii::AsciiExt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_167_rrrruuuugggg_test_rug = 0;
        let mut p0 = Utf16Char::default();
        <Utf16Char as std::ascii::AsciiExt>::to_ascii_uppercase(&p0);
        let _rug_ed_tests_rug_167_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_168 {
    use super::*;
    use crate::std::ascii::AsciiExt;
    use crate::Utf16Char;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_168_rrrruuuugggg_test_rug = 0;
        let mut p0 = Utf16Char::default();
        <Utf16Char as std::ascii::AsciiExt>::to_ascii_lowercase(&p0);
        let _rug_ed_tests_rug_168_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_169 {
    use super::*;
    use crate::Utf16Char;
    use crate::std::ascii::AsciiExt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_169_rrrruuuugggg_test_rug = 0;
        let mut p0 = Utf16Char::default();
        <Utf16Char as std::ascii::AsciiExt>::make_ascii_uppercase(&mut p0);
        let _rug_ed_tests_rug_169_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_170 {
    use super::*;
    use crate::std::ascii::AsciiExt;
    use crate::Utf16Char;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_170_rrrruuuugggg_test_rug = 0;
        let mut p0 = Utf16Char::default();
        <Utf16Char as std::ascii::AsciiExt>::make_ascii_lowercase(&mut p0);
        let _rug_ed_tests_rug_170_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_171 {
    use super::*;
    use crate::std::hash::{Hash, Hasher, SipHasher};
    use crate::Utf16Char;
    #[test]
    fn test_hash() {
        let _rug_st_tests_rug_171_rrrruuuugggg_test_hash = 0;
        let mut p0 = Utf16Char::default();
        let mut p1 = SipHasher::default();
        <Utf16Char as Hash>::hash(&p0, &mut p1);
        let _rug_ed_tests_rug_171_rrrruuuugggg_test_hash = 0;
    }
}
#[cfg(test)]
mod tests_rug_172 {
    use super::*;
    use crate::std::cmp::PartialOrd;
    use crate::Utf16Char;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_172_rrrruuuugggg_test_rug = 0;
        let mut p0 = Utf16Char::default();
        let mut p1 = Utf16Char::default();
        <Utf16Char as PartialOrd>::partial_cmp(&p0, &p1);
        let _rug_ed_tests_rug_172_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_173 {
    use super::*;
    use crate::std::cmp::Ord;
    use crate::Utf16Char;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_173_rrrruuuugggg_test_rug = 0;
        let mut p0 = Utf16Char::default();
        let mut p1 = Utf16Char::default();
        <Utf16Char as Ord>::cmp(&p0, &p1);
        let _rug_ed_tests_rug_173_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_174 {
    use super::*;
    use crate::std::cmp::PartialEq;
    use crate::Utf16Char;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_174_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 'a';
        let mut p0 = Utf16Char::default();
        let mut p1: char = rug_fuzz_0;
        <Utf16Char as std::cmp::PartialEq<char>>::eq(&p0, &p1);
        let _rug_ed_tests_rug_174_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_175 {
    use super::*;
    use crate::utf16_char::Utf16Char;
    use crate::std::cmp::PartialEq;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_175_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 'a';
        let mut p0: char = rug_fuzz_0;
        let mut p1 = Utf16Char::default();
        <char>::eq(&p0, &p1);
        let _rug_ed_tests_rug_175_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_176_prepare {
    use crate::Utf16Char;
    #[test]
    fn sample() {
        let _rug_st_tests_rug_176_prepare_rrrruuuugggg_sample = 0;
        let mut v7 = Utf16Char::default();
        let _rug_ed_tests_rug_176_prepare_rrrruuuugggg_sample = 0;
    }
}
#[cfg(test)]
mod tests_rug_176 {
    use super::*;
    use crate::std::cmp::{PartialOrd, Ordering};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_176_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 'a';
        let mut p0 = Utf16Char::default();
        let p1: char = rug_fuzz_0;
        <Utf16Char as PartialOrd<char>>::partial_cmp(&p0, &p1);
        let _rug_ed_tests_rug_176_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_177 {
    use super::*;
    use crate::std::cmp::PartialOrd;
    use crate::Utf16Char;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_177_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 'a';
        let mut p0: char = rug_fuzz_0;
        let mut p1 = Utf16Char::default();
        <char>::partial_cmp(&p0, &p1);
        let _rug_ed_tests_rug_177_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_178 {
    use super::*;
    use crate::{Utf16Char, Utf8Char};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_178_rrrruuuugggg_test_rug = 0;
        let mut p0 = Utf16Char::default();
        let mut p1 = Utf8Char::default();
        <Utf16Char as std::cmp::PartialEq<Utf8Char>>::eq(&p0, &p1);
        let _rug_ed_tests_rug_178_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_179 {
    use super::*;
    use crate::std::cmp::PartialOrd;
    use crate::{Utf16Char, Utf8Char};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_179_rrrruuuugggg_test_rug = 0;
        let mut p0 = Utf16Char::default();
        let mut p1 = Utf8Char::default();
        <Utf16Char as PartialOrd<Utf8Char>>::partial_cmp(&p0, &p1);
        let _rug_ed_tests_rug_179_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_180 {
    use super::*;
    use crate::std::cmp::PartialEq;
    use crate::Utf16Char;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_180_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0xABCD;
        let mut p0 = Utf16Char::default();
        let p1: u16 = rug_fuzz_0;
        <Utf16Char as std::cmp::PartialEq<u16>>::eq(&p0, &p1);
        let _rug_ed_tests_rug_180_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_181 {
    use super::*;
    use crate::std::cmp::PartialEq;
    use crate::Utf16Char;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_181_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 65;
        let mut p0 = Utf16Char::default();
        let mut p1: u8 = rug_fuzz_0;
        debug_assert_eq!(
            < Utf16Char as std::cmp::PartialEq < u8 > > ::eq(& p0, & p1), false
        );
        let _rug_ed_tests_rug_181_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_182 {
    use super::*;
    use crate::Utf16Char;
    #[test]
    fn test_from_str_start() {
        let _rug_st_tests_rug_182_rrrruuuugggg_test_from_str_start = 0;
        let rug_fuzz_0 = "a";
        let rug_fuzz_1 = "ab";
        let rug_fuzz_2 = "🂠 ";
        let rug_fuzz_3 = "é";
        let rug_fuzz_4 = "";
        let p0 = rug_fuzz_0;
        debug_assert_eq!(Utf16Char::from_str_start(p0), Ok((Utf16Char::from('a'), 1)));
        let p1 = rug_fuzz_1;
        debug_assert_eq!(Utf16Char::from_str_start(p1), Ok((Utf16Char::from('a'), 1)));
        let p2 = rug_fuzz_2;
        debug_assert_eq!(
            Utf16Char::from_str_start(p2), Ok((Utf16Char::from('🂠'), 4))
        );
        let p3 = rug_fuzz_3;
        debug_assert_eq!(Utf16Char::from_str_start(p3), Ok((Utf16Char::from('e'), 1)));
        let p4 = rug_fuzz_4;
        debug_assert!(Utf16Char::from_str_start(p4).is_err());
        let _rug_ed_tests_rug_182_rrrruuuugggg_test_from_str_start = 0;
    }
}
#[cfg(test)]
mod tests_rug_183 {
    use super::*;
    use crate::utf16_char::{Utf16Char, InvalidUtf16Slice};
    use std::char;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_183_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0xD83D;
        let rug_fuzz_1 = 0xDE3B;
        let p0: &[u16] = &[rug_fuzz_0, rug_fuzz_1];
        Utf16Char::from_slice_start(p0).unwrap();
        let _rug_ed_tests_rug_183_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_184 {
    use super::*;
    use utf16_char::Utf16Char;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_184_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0xD83D;
        let rug_fuzz_1 = 0xDC68;
        let p0: &[u16] = &[rug_fuzz_0, rug_fuzz_1];
        unsafe {
            let _ = Utf16Char::from_slice_start_unchecked(p0);
        }
        let _rug_ed_tests_rug_184_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_185 {
    use super::*;
    use crate::Utf16Char;
    use crate::error::InvalidUtf16Array;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_185_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 'x';
        let rug_fuzz_1 = 'y';
        let mut p0: [u16; 2] = [rug_fuzz_0 as u16, rug_fuzz_1 as u16];
        debug_assert_eq!(Utf16Char::from_array(p0), Ok(Utf16Char::from('x')));
        let _rug_ed_tests_rug_185_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_186 {
    use super::*;
    use crate::utf16_char::Utf16Char;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_186_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0xD83D;
        let rug_fuzz_1 = 0xDE01;
        let mut p0: [u16; 2] = [rug_fuzz_0, rug_fuzz_1];
        unsafe {
            Utf16Char::from_array_unchecked(p0);
        }
        let _rug_ed_tests_rug_186_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_187 {
    use super::*;
    use crate::utf16_char;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_187_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 97;
        let rug_fuzz_1 = 98;
        let mut p0: (u16, std::option::Option<u16>) = (rug_fuzz_0, Some(rug_fuzz_1));
        utf16_char::Utf16Char::from_tuple(p0).unwrap();
        let _rug_ed_tests_rug_187_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_188 {
    use super::*;
    use crate::utf16_char::Utf16Char;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_188_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 'A';
        let char_val = rug_fuzz_0;
        let utf16_tuple = char_val.to_utf16_tuple();
        let p0: (u16, Option<u16>) = utf16_tuple;
        unsafe {
            Utf16Char::from_tuple_unchecked(p0);
        }
        let _rug_ed_tests_rug_188_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_189 {
    use super::*;
    use crate::Utf16Char;
    #[test]
    fn test_from_bmp() {
        let _rug_st_tests_rug_189_rrrruuuugggg_test_from_bmp = 0;
        let rug_fuzz_0 = 0x40;
        let rug_fuzz_1 = 0;
        let p0: u16 = rug_fuzz_0;
        debug_assert_eq!(Utf16Char::from_bmp(p0).unwrap().units[rug_fuzz_1], 0x40);
        let _rug_ed_tests_rug_189_rrrruuuugggg_test_from_bmp = 0;
    }
}
#[cfg(test)]
mod tests_rug_190 {
    use super::*;
    use crate::utf16_char;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_190_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0x0041;
        let p0: u16 = rug_fuzz_0;
        unsafe {
            utf16_char::Utf16Char::from_bmp_unchecked(p0);
        }
        let _rug_ed_tests_rug_190_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_191 {
    use super::*;
    use crate::Utf16Char;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_191_rrrruuuugggg_test_rug = 0;
        let mut p0 = Utf16Char::default();
        debug_assert_eq!(Utf16Char::is_bmp(& p0), true);
        let _rug_ed_tests_rug_191_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_192 {
    use super::*;
    use crate::Utf16Char;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_192_rrrruuuugggg_test_rug = 0;
        let mut p0 = Utf16Char::default();
        debug_assert_eq!(p0.len(), 1);
        let _rug_ed_tests_rug_192_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_193 {
    use super::*;
    use crate::Utf16Char;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_193_rrrruuuugggg_test_rug = 0;
        let mut p0 = Utf16Char::default();
        debug_assert_eq!(p0.is_ascii(), true);
        let _rug_ed_tests_rug_193_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_194 {
    use super::*;
    use crate::Utf16Char;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_194_rrrruuuugggg_test_rug = 0;
        let mut p0 = Utf16Char::default();
        let mut p1 = Utf16Char::default();
        debug_assert!(p0.eq_ignore_ascii_case(& p1));
        let _rug_ed_tests_rug_194_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_195 {
    use super::*;
    use crate::Utf16Char;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_195_rrrruuuugggg_test_rug = 0;
        let mut p0 = Utf16Char::default();
        p0 = p0.to_ascii_uppercase();
        let _rug_ed_tests_rug_195_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_196 {
    use super::*;
    use crate::Utf16Char;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_196_rrrruuuugggg_test_rug = 0;
        let mut p0 = Utf16Char::default();
        p0 = p0.to_ascii_lowercase();
        let _rug_ed_tests_rug_196_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_197 {
    use super::*;
    use crate::Utf16Char;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_197_rrrruuuugggg_test_rug = 0;
        let mut p0 = Utf16Char::default();
        p0.make_ascii_uppercase();
        let _rug_ed_tests_rug_197_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_198 {
    use super::*;
    use crate::Utf16Char;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_198_rrrruuuugggg_test_rug = 0;
        let mut p0 = Utf16Char::default();
        p0.make_ascii_lowercase();
        let _rug_ed_tests_rug_198_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_199 {
    use super::*;
    use crate::utf16_char::Utf16Char;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_199_rrrruuuugggg_test_rug = 0;
        let mut p0 = Utf16Char::default();
        <Utf16Char>::to_char(p0);
        let _rug_ed_tests_rug_199_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_200 {
    use super::*;
    use crate::Utf16Char;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_200_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 97;
        let rug_fuzz_1 = 98;
        let rug_fuzz_2 = 0u16;
        let mut v7 = Utf16Char::default();
        v7.units = [rug_fuzz_0, rug_fuzz_1];
        let mut buffer = [rug_fuzz_2; 2];
        debug_assert_eq!(v7.to_slice(& mut buffer), 2);
        debug_assert_eq!(buffer, [98, 97]);
        let _rug_ed_tests_rug_200_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_201 {
    use super::*;
    use crate::Utf16Char;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_201_rrrruuuugggg_test_rug = 0;
        let mut p0 = Utf16Char::default();
        debug_assert_eq!(p0.to_array(), [0u16; 2]);
        let _rug_ed_tests_rug_201_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_202 {
    use super::*;
    use crate::Utf16Char;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_202_rrrruuuugggg_test_rug = 0;
        let mut p0 = Utf16Char::default();
        debug_assert_eq!(p0.to_tuple(), (0, None));
        let _rug_ed_tests_rug_202_rrrruuuugggg_test_rug = 0;
    }
}
