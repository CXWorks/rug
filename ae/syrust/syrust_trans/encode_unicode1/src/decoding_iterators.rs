//! Iterators that turn multiple `u8`s or `u16`s into `Utf*Char`s, but can fail.
//!
//! To be predictable, all errors consume one element each.
//!
//! The iterator adaptors produce neither offset nor element length to work
//! well with other adaptors,
//! while the slice iterators yield both to make more advanced use cases easy.
use errors::{InvalidUtf8Slice, InvalidUtf16FirstUnit, Utf16PairError};
use errors::InvalidUtf8Slice::*;
use errors::InvalidUtf8::*;
use errors::InvalidUtf8FirstByte::*;
use errors::InvalidUtf16Slice::*;
use errors::InvalidCodepoint::*;
use errors::Utf16PairError::*;
use utf8_char::Utf8Char;
use utf16_char::Utf16Char;
use traits::U16UtfExt;
extern crate core;
use self::core::borrow::Borrow;
use self::core::fmt::{self, Debug};
use self::core::iter::Chain;
use self::core::option;
/// Decodes UTF-8 characters from a byte iterator into `Utf8Char`s.
///
/// See [`IterExt::to_utf8chars()`](../trait.IterExt.html#tymethod.to_utf8chars)
/// for examples and error handling.
#[derive(Clone, Default)]
pub struct Utf8CharMerger<B: Borrow<u8>, I: Iterator<Item = B>> {
    iter: I,
    /// number of bytes that were read before an error was detected
    after_err_leftover: u8,
    /// stack because it simplifies popping.
    after_err_stack: [u8; 3],
}
impl<
    B: Borrow<u8>,
    I: Iterator<Item = B>,
    T: IntoIterator<IntoIter = I, Item = B>,
> From<T> for Utf8CharMerger<B, I> {
    fn from(t: T) -> Self {
        Utf8CharMerger {
            iter: t.into_iter(),
            after_err_leftover: 0,
            after_err_stack: [0; 3],
        }
    }
}
impl<B: Borrow<u8>, I: Iterator<Item = B>> Utf8CharMerger<B, I> {
    /// Extract the inner iterator.
    ///
    /// If the last item produced by `.next()` was an `Err`,
    /// up to three following bytes might be missing.
    /// The exact number of missing bytes for each error type should not be relied on.
    ///
    /// # Examples
    ///
    /// Three bytes swallowed:
    /// ```
    /// # use encode_unicode::IterExt;
    /// let mut merger = b"\xf4\xa1\xb2FS".iter().to_utf8chars();
    /// assert!(merger.next().unwrap().is_err());
    /// let mut inner: std::slice::Iter<u8> = merger.into_inner();
    /// assert_eq!(inner.next(), Some(&b'S')); // b'\xa1', b'\xb2' and b'F' disappeared
    /// ```
    ///
    /// All bytes present:
    /// ```
    /// # use encode_unicode::IterExt;
    /// let mut merger = b"\xb0FS".iter().to_utf8chars();
    /// assert!(merger.next().unwrap().is_err());
    /// assert_eq!(merger.into_inner().next(), Some(&b'F'));
    /// ```
    ///
    /// Two bytes missing:
    /// ```
    /// # use encode_unicode::IterExt;
    /// let mut merger = b"\xe0\x80\x80FS".iter().to_utf8chars();
    /// assert!(merger.next().unwrap().is_err());
    /// assert_eq!(merger.into_inner().next(), Some(&b'F'));
    /// ```
    pub fn into_inner(self) -> I {
        self.iter
    }
    fn save(&mut self, bytes: &[u8; 4], len: usize) {
        for &after_err in bytes[1..len].iter().rev() {
            self.after_err_stack[self.after_err_leftover as usize] = after_err;
            self.after_err_leftover += 1;
        }
    }
    /// Reads len-1 bytes into bytes[1..]
    fn extra(
        &mut self,
        bytes: &mut [u8; 4],
        len: usize,
    ) -> Result<(), InvalidUtf8Slice> {
        debug_assert_eq!(
            self.after_err_leftover, 0, "first: {:#02x}, stack: {:?}", bytes[0], self
            .after_err_stack
        );
        for i in 1..len {
            if let Some(extra) = self.iter.next() {
                let extra = *extra.borrow();
                bytes[i] = extra;
                if extra & 0b1100_0000 != 0b1000_0000 {
                    self.save(bytes, i + 1);
                    return Err(InvalidUtf8Slice::Utf8(NotAContinuationByte(i)));
                }
            } else {
                self.save(bytes, i);
                return Err(TooShort(len));
            }
        }
        Ok(())
    }
}
impl<B: Borrow<u8>, I: Iterator<Item = B>> Iterator for Utf8CharMerger<B, I> {
    type Item = Result<Utf8Char, InvalidUtf8Slice>;
    fn next(&mut self) -> Option<Self::Item> {
        let first: u8;
        if self.after_err_leftover != 0 {
            self.after_err_leftover -= 1;
            first = self.after_err_stack[self.after_err_leftover as usize];
        } else if let Some(next) = self.iter.next() {
            first = *next.borrow();
        } else {
            return None;
        }
        unsafe {
            let mut bytes = [first, 0, 0, 0];
            let ok = match first {
                0b0000_0000..=0b0111_1111 => Ok(()),
                0b1100_0010..=0b1101_1111 => self.extra(&mut bytes, 2),
                0b1110_0000..=0b1110_1111 => {
                    if let Err(e) = self.extra(&mut bytes, 3) {
                        Err(e)
                    } else if bytes[0] == 0b1110_0000 && bytes[1] <= 0b10_011111 {
                        self.save(&bytes, 3);
                        Err(Utf8(OverLong))
                    } else if bytes[0] == 0b1110_1101
                        && bytes[1] & 0b11_100000 == 0b10_100000
                    {
                        self.save(&bytes, 3);
                        Err(Codepoint(Utf16Reserved))
                    } else {
                        Ok(())
                    }
                }
                0b1111_0000..=0b1111_0100 => {
                    if let Err(e) = self.extra(&mut bytes, 4) {
                        Err(e)
                    } else if bytes[0] == 0b11110_000 && bytes[1] <= 0b10_001111 {
                        self.save(&bytes, 4);
                        Err(InvalidUtf8Slice::Utf8(OverLong))
                    } else if bytes[0] == 0b11110_100 && bytes[1] > 0b10_001111 {
                        self.save(&bytes, 4);
                        Err(InvalidUtf8Slice::Codepoint(TooHigh))
                    } else {
                        Ok(())
                    }
                }
                0b1000_0000..=0b1011_1111 => Err(Utf8(FirstByte(ContinuationByte))),
                0b1100_0000..=0b1100_0001 => Err(Utf8(OverLong)),
                0b1111_0101..=0b1111_0111 => Err(Codepoint(TooHigh)),
                0b1111_1000..=0b1111_1111 => Err(Utf8(FirstByte(TooLongSeqence))),
                _ => unreachable!("all possible byte values should be covered"),
            };
            Some(ok.map(|()| Utf8Char::from_array_unchecked(bytes)))
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (iter_min, iter_max) = self.iter.size_hint();
        let min = iter_min / 4;
        let max = iter_max
            .and_then(|max| { max.checked_add(self.after_err_leftover as usize) });
        (min, max)
    }
}
impl<B: Borrow<u8>, I: Iterator<Item = B> + Debug> Debug for Utf8CharMerger<B, I> {
    fn fmt(&self, fmtr: &mut fmt::Formatter) -> fmt::Result {
        let mut in_order = [0u8; 3];
        for i in 0..self.after_err_leftover as usize {
            in_order[i] = self.after_err_stack[self.after_err_leftover as usize - i - 1];
        }
        fmtr.debug_struct("Utf8CharMerger")
            .field("buffered", &&in_order[..self.after_err_leftover as usize])
            .field("inner", &self.iter)
            .finish()
    }
}
/// An [`Utf8CharMerger`](struct.Utf8CharMerger.html) that also produces
/// offsets and lengths, but can only iterate over slices.
///
/// See [`SliceExt::utf8char_indices()`](../trait.SliceExt.html#tymethod.utf8char_indices)
/// for examples and error handling.
#[derive(Clone, Default)]
pub struct Utf8CharDecoder<'a> {
    slice: &'a [u8],
    index: usize,
}
impl<'a> From<&'a [u8]> for Utf8CharDecoder<'a> {
    fn from(s: &[u8]) -> Utf8CharDecoder {
        Utf8CharDecoder {
            slice: s,
            index: 0,
        }
    }
}
impl<'a> Utf8CharDecoder<'a> {
    /// Extract the remainder of the source slice.
    ///
    /// # Examples
    ///
    /// Unlike `Utf8CharMerger::into_inner()`, bytes directly after an error
    /// are never swallowed:
    /// ```
    /// # use encode_unicode::SliceExt;
    /// let mut iter = b"\xf4\xa1\xb2FS".utf8char_indices();
    /// assert!(iter.next().unwrap().1.is_err());
    /// assert_eq!(iter.as_slice(), b"\xa1\xb2FS");
    /// ```
    pub fn as_slice(&self) -> &'a [u8] {
        &self.slice[self.index..]
    }
}
impl<'a> Iterator for Utf8CharDecoder<'a> {
    type Item = (usize, Result<Utf8Char, InvalidUtf8Slice>, usize);
    fn next(&mut self) -> Option<Self::Item> {
        let start = self.index;
        match Utf8Char::from_slice_start(&self.slice[self.index..]) {
            Ok((u8c, len)) => {
                self.index += len;
                Some((start, Ok(u8c), len))
            }
            Err(TooShort(1)) => None,
            Err(e) => {
                self.index += 1;
                Some((start, Err(e), 1))
            }
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let bytes = self.slice.len() - self.index;
        (bytes / 4, Some(bytes))
    }
}
impl<'a> DoubleEndedIterator for Utf8CharDecoder<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.index < self.slice.len() {
            let extras = self
                .slice
                .iter()
                .rev()
                .take_while(|&b| b & 0b1100_0000 == 0b1000_0000)
                .count();
            let starts = self.slice.len() - (extras + 1);
            match Utf8Char::from_slice_start(&self.slice[starts..]) {
                Ok((u8c, len)) if len == 1 + extras => {
                    self.slice = &self.slice[..starts];
                    Some((starts, Ok(u8c), len))
                }
                Err(e) if extras == 0 => {
                    self.slice = &self.slice[..self.slice.len() - 1];
                    Some((self.slice.len() - 1, Err(e), 1))
                }
                _ => {
                    self.slice = &self.slice[..self.slice.len() - 1];
                    Some((
                        self.slice.len() - 1,
                        Err(Utf8(FirstByte(ContinuationByte))),
                        1,
                    ))
                }
            }
        } else {
            None
        }
    }
}
impl<'a> Debug for Utf8CharDecoder<'a> {
    fn fmt(&self, fmtr: &mut fmt::Formatter) -> fmt::Result {
        write!(
            fmtr, "Utf8CharDecoder {{ bytes[{}..]: {:?} }}", self.index, self.as_slice()
        )
    }
}
/// Decodes UTF-16 characters from a `u16` iterator into `Utf16Char`s.
///
/// See [`IterExt::to_utf16chars()`](../trait.IterExt.html#tymethod.to_utf16chars)
/// for examples and error handling.
#[derive(Clone, Default)]
pub struct Utf16CharMerger<B: Borrow<u16>, I: Iterator<Item = B>> {
    iter: I,
    /// Used when a trailing surrogate was expected, the u16 can be any value.
    prev: Option<B>,
}
impl<
    B: Borrow<u16>,
    I: Iterator<Item = B>,
    T: IntoIterator<IntoIter = I, Item = B>,
> From<T> for Utf16CharMerger<B, I> {
    fn from(t: T) -> Self {
        Utf16CharMerger {
            iter: t.into_iter(),
            prev: None,
        }
    }
}
impl<B: Borrow<u16>, I: Iterator<Item = B>> Utf16CharMerger<B, I> {
    /// Extract the inner iterator.
    ///
    /// If the last item produced was an `Err`, the first unit might be missing.
    ///
    /// # Examples
    ///
    /// Unit right after an error missing
    /// ```
    /// # use encode_unicode::IterExt;
    /// # use encode_unicode::error::Utf16PairError;
    /// let mut merger = [0xd901, 'F' as u16, 'S' as u16].iter().to_utf16chars();
    /// assert_eq!(merger.next(), Some(Err(Utf16PairError::UnmatchedLeadingSurrogate)));
    /// let mut inner: std::slice::Iter<u16> = merger.into_inner();
    /// assert_eq!(inner.next(), Some('S' as u16).as_ref()); // 'F' was consumed by Utf16CharMerger
    /// ```
    ///
    /// Error that doesn't swallow any units
    /// ```
    /// # use encode_unicode::IterExt;
    /// # use encode_unicode::error::Utf16PairError;
    /// let mut merger = [0xde00, 'F' as u16, 'S' as u16].iter().to_utf16chars();
    /// assert_eq!(merger.next(), Some(Err(Utf16PairError::UnexpectedTrailingSurrogate)));
    /// let mut inner: std::slice::Iter<u16> = merger.into_inner();
    /// assert_eq!(inner.next(), Some('F' as u16).as_ref()); // not consumed
    /// ```
    pub fn into_inner(self) -> I {
        self.iter
    }
    /// Returns an iterator over the remaining units.
    /// Unlike `into_inner()` this will never drop any units.
    ///
    /// The exact type of the returned iterator should not be depended on.
    ///
    /// # Examples
    ///
    /// ```
    /// # use encode_unicode::IterExt;
    /// # use encode_unicode::error::Utf16PairError;
    /// let slice = [0xd901, 'F' as u16, 'S' as u16];
    /// let mut merger = slice.iter().to_utf16chars();
    /// assert_eq!(merger.next(), Some(Err(Utf16PairError::UnmatchedLeadingSurrogate)));
    /// let mut remaining = merger.into_remaining_units();
    /// assert_eq!(remaining.next(), Some('F' as u16).as_ref());
    /// ```
    pub fn into_remaining_units(self) -> Chain<option::IntoIter<B>, I> {
        self.prev.into_iter().chain(self.iter)
    }
}
impl<B: Borrow<u16>, I: Iterator<Item = B>> Iterator for Utf16CharMerger<B, I> {
    type Item = Result<Utf16Char, Utf16PairError>;
    fn next(&mut self) -> Option<Self::Item> {
        let first = self.prev.take().or_else(|| self.iter.next());
        first
            .map(|first| unsafe {
                match first.borrow().utf16_needs_extra_unit() {
                    Ok(false) => {
                        Ok(Utf16Char::from_array_unchecked([*first.borrow(), 0]))
                    }
                    Ok(true) => {
                        match self.iter.next() {
                            Some(second) => {
                                match second.borrow().utf16_needs_extra_unit() {
                                    Err(InvalidUtf16FirstUnit) => {
                                        Ok(
                                            Utf16Char::from_tuple_unchecked((
                                                *first.borrow(),
                                                Some(*second.borrow()),
                                            )),
                                        )
                                    }
                                    Ok(_) => {
                                        self.prev = Some(second);
                                        Err(Utf16PairError::UnmatchedLeadingSurrogate)
                                    }
                                }
                            }
                            None => Err(Utf16PairError::Incomplete),
                        }
                    }
                    Err(InvalidUtf16FirstUnit) => {
                        Err(Utf16PairError::UnexpectedTrailingSurrogate)
                    }
                }
            })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (iter_min, iter_max) = self.iter.size_hint();
        let min = iter_min / 2;
        let max = match (iter_max, &self.prev) {
            (Some(max), &Some(_)) => max.checked_add(1),
            (max, _) => max,
        };
        (min, max)
    }
}
impl<B: Borrow<u16>, I: Iterator<Item = B> + Debug> Debug for Utf16CharMerger<B, I> {
    fn fmt(&self, fmtr: &mut fmt::Formatter) -> fmt::Result {
        fmtr.debug_struct("Utf16CharMerger")
            .field("buffered", &self.prev.as_ref().map(|b| *b.borrow()))
            .field("inner", &self.iter)
            .finish()
    }
}
/// An [`Utf16CharMerger`](struct.Utf16CharMerger.html) that also produces
/// offsets and lengths, but can only iterate over slices.
///
/// See [`SliceExt::utf16char_indices()`](../trait.SliceExt.html#tymethod.utf16char_indices)
/// for examples and error handling.
#[derive(Clone, Default)]
pub struct Utf16CharDecoder<'a> {
    slice: &'a [u16],
    index: usize,
}
impl<'a> From<&'a [u16]> for Utf16CharDecoder<'a> {
    fn from(s: &'a [u16]) -> Self {
        Utf16CharDecoder {
            slice: s,
            index: 0,
        }
    }
}
impl<'a> Utf16CharDecoder<'a> {
    /// Extract the remainder of the source slice.
    ///
    /// # Examples
    ///
    /// Unlike `Utf16CharMerger::into_inner()`, the unit after an error is never swallowed:
    /// ```
    /// # use encode_unicode::SliceExt;
    /// # use encode_unicode::error::Utf16PairError;
    /// let mut iter = [0xd901, 'F' as u16, 'S' as u16].utf16char_indices();
    /// assert_eq!(iter.next(), Some((0, Err(Utf16PairError::UnmatchedLeadingSurrogate), 1)));
    /// assert_eq!(iter.as_slice(), &['F' as u16, 'S' as u16]);
    /// ```
    pub fn as_slice(&self) -> &[u16] {
        &self.slice[self.index..]
    }
}
impl<'a> Iterator for Utf16CharDecoder<'a> {
    type Item = (usize, Result<Utf16Char, Utf16PairError>, usize);
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let start = self.index;
        match Utf16Char::from_slice_start(self.as_slice()) {
            Ok((u16c, len)) => {
                self.index += len;
                Some((start, Ok(u16c), len))
            }
            Err(EmptySlice) => None,
            Err(FirstLowSurrogate) => {
                self.index += 1;
                Some((start, Err(UnexpectedTrailingSurrogate), 1))
            }
            Err(SecondNotLowSurrogate) => {
                self.index += 1;
                Some((start, Err(UnmatchedLeadingSurrogate), 1))
            }
            Err(MissingSecond) => {
                self.index = self.slice.len();
                Some((start, Err(Incomplete), 1))
            }
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let units = self.slice.len() - self.index;
        (units / 2, Some(units))
    }
}
impl<'a> Debug for Utf16CharDecoder<'a> {
    fn fmt(&self, fmtr: &mut fmt::Formatter) -> fmt::Result {
        write!(
            fmtr, "Utf16CharDecoder {{ units[{}..]: {:?} }}", self.index, self.as_slice()
        )
    }
}
#[cfg(test)]
mod tests_rug_204 {
    use super::*;
    use crate::{IterExt, decoding_iterators};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_204_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"\xf4\xa1\xb2FS";
        let mut p0 = rug_fuzz_0.iter().to_utf8chars();
        decoding_iterators::Utf8CharMerger::<_, _>::into_inner(p0);
        let _rug_ed_tests_rug_204_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_207 {
    use super::*;
    use crate::std::iter::Iterator;
    use decoding_iterators::Utf8CharMerger;
    struct MockIterator {}
    impl Iterator for MockIterator {
        type Item = u8;
        fn next(&mut self) -> Option<Self::Item> {
            Some(b'a')
        }
    }
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_207_rrrruuuugggg_test_rug = 0;
        let mut mock_iterator = MockIterator {};
        let mut utf8_char_merger: Utf8CharMerger<_, MockIterator> = Utf8CharMerger::from(
            mock_iterator,
        );
        utf8_char_merger.next();
        let _rug_ed_tests_rug_207_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_212 {
    use super::*;
    use crate::decoding_iterators::Utf8CharDecoder;
    use crate::std::iter::Iterator;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_212_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"\xCE\xBB";
        let rug_fuzz_1 = 0;
        let slice = rug_fuzz_0;
        let index = rug_fuzz_1;
        let p0 = Utf8CharDecoder { slice, index };
        p0.size_hint();
        let _rug_ed_tests_rug_212_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_215 {
    use super::*;
    use crate::IterExt;
    use crate::error::Utf16PairError;
    #[test]
    fn test_rug() {
        let mut p0 = [0xd901, 'F' as u16, 'S' as u16].iter().to_utf16chars();
        crate::decoding_iterators::Utf16CharMerger::<_, _>::into_inner(p0);
    }
}
#[cfg(test)]
mod tests_rug_216 {
    use super::*;
    use crate::IterExt;
    use crate::error::Utf16PairError;
    use std::iter::Chain;
    use std::option;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_216_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0xd901;
        let rug_fuzz_1 = 'F';
        let rug_fuzz_2 = 'S';
        let slice = [rug_fuzz_0, rug_fuzz_1 as u16, rug_fuzz_2 as u16];
        let mut merger = slice.iter().to_utf16chars();
        debug_assert_eq!(
            merger.next(), Some(Err(Utf16PairError::UnmatchedLeadingSurrogate))
        );
        let p0 = merger;
        crate::decoding_iterators::Utf16CharMerger::<_, _>::into_remaining_units(p0);
        let _rug_ed_tests_rug_216_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_219 {
    use super::*;
    use crate::decoding_iterators::Utf16CharDecoder;
    #[test]
    fn test_from_impl() {
        let p0: &[u16] = &[0x0041, 0x0042, 0x0043];
        let _result = <Utf16CharDecoder<
            'static,
        > as std::convert::From<&[u16]>>::from(p0);
    }
}
#[cfg(test)]
mod tests_rug_220 {
    use super::*;
    use crate::{SliceExt, error::Utf16PairError};
    #[test]
    fn test_rug() {
        let mut iter = [0xd901, 'F' as u16, 'S' as u16].utf16char_indices();
        assert_eq!(
            iter.next(), Some((0, Err(Utf16PairError::UnmatchedLeadingSurrogate), 1))
        );
        assert_eq!(iter.as_slice(), & ['F' as u16, 'S' as u16]);
    }
}
#[cfg(test)]
mod tests_rug_222 {
    use super::*;
    use crate::decoding_iterators::Utf16CharDecoder;
    use crate::std::iter::Iterator;
    #[test]
    fn test_size_hint() {
        let _rug_st_tests_rug_222_rrrruuuugggg_test_size_hint = 0;
        let rug_fuzz_0 = 0xDC00;
        let rug_fuzz_1 = 0xD800;
        let rug_fuzz_2 = 0xDC00;
        let rug_fuzz_3 = 0xD800;
        let rug_fuzz_4 = 0xDC01;
        let rug_fuzz_5 = 0xD800;
        let rug_fuzz_6 = 2;
        let slice = [
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
        ];
        let index = rug_fuzz_6;
        let decoder = Utf16CharDecoder {
            slice: &slice,
            index,
        };
        decoder.size_hint();
        let _rug_ed_tests_rug_222_rrrruuuugggg_test_size_hint = 0;
    }
}
