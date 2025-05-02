use core::char;
use core::cmp;
use core::fmt;
use core::str;
#[cfg(feature = "std")]
use std::error;
use ascii;
use bstr::BStr;
use ext_slice::ByteSlice;
const ACCEPT: usize = 12;
const REJECT: usize = 0;
/// SAFETY: The decode below function relies on the correctness of these
/// equivalence classes.
#[cfg_attr(rustfmt, rustfmt::skip)]
const CLASSES: [u8; 256] = [
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    1,
    1,
    1,
    1,
    1,
    1,
    1,
    1,
    1,
    1,
    1,
    1,
    1,
    1,
    1,
    1,
    9,
    9,
    9,
    9,
    9,
    9,
    9,
    9,
    9,
    9,
    9,
    9,
    9,
    9,
    9,
    9,
    7,
    7,
    7,
    7,
    7,
    7,
    7,
    7,
    7,
    7,
    7,
    7,
    7,
    7,
    7,
    7,
    7,
    7,
    7,
    7,
    7,
    7,
    7,
    7,
    7,
    7,
    7,
    7,
    7,
    7,
    7,
    7,
    8,
    8,
    2,
    2,
    2,
    2,
    2,
    2,
    2,
    2,
    2,
    2,
    2,
    2,
    2,
    2,
    2,
    2,
    2,
    2,
    2,
    2,
    2,
    2,
    2,
    2,
    2,
    2,
    2,
    2,
    2,
    2,
    10,
    3,
    3,
    3,
    3,
    3,
    3,
    3,
    3,
    3,
    3,
    3,
    3,
    4,
    3,
    3,
    11,
    6,
    6,
    6,
    5,
    8,
    8,
    8,
    8,
    8,
    8,
    8,
    8,
    8,
    8,
    8,
];
/// SAFETY: The decode below function relies on the correctness of this state
/// machine.
#[cfg_attr(rustfmt, rustfmt::skip)]
const STATES_FORWARD: &'static [u8] = &[
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    12,
    0,
    24,
    36,
    60,
    96,
    84,
    0,
    0,
    0,
    48,
    72,
    0,
    12,
    0,
    0,
    0,
    0,
    0,
    12,
    0,
    12,
    0,
    0,
    0,
    24,
    0,
    0,
    0,
    0,
    0,
    24,
    0,
    24,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    24,
    0,
    0,
    0,
    0,
    0,
    24,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    24,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    36,
    0,
    36,
    0,
    0,
    0,
    36,
    0,
    0,
    0,
    0,
    0,
    36,
    0,
    36,
    0,
    0,
    0,
    36,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
];
/// An iterator over Unicode scalar values in a byte string.
///
/// When invalid UTF-8 byte sequences are found, they are substituted with the
/// Unicode replacement codepoint (`U+FFFD`) using the
/// ["maximal subpart" strategy](http://www.unicode.org/review/pr-121.html).
///
/// This iterator is created by the
/// [`chars`](trait.ByteSlice.html#method.chars) method provided by the
/// [`ByteSlice`](trait.ByteSlice.html) extension trait for `&[u8]`.
#[derive(Clone, Debug)]
pub struct Chars<'a> {
    bs: &'a [u8],
}
impl<'a> Chars<'a> {
    pub(crate) fn new(bs: &'a [u8]) -> Chars<'a> {
        Chars { bs }
    }
    /// View the underlying data as a subslice of the original data.
    ///
    /// The slice returned has the same lifetime as the original slice, and so
    /// the iterator can continue to be used while this exists.
    ///
    /// # Examples
    ///
    /// ```
    /// use bstr::ByteSlice;
    ///
    /// let mut chars = b"abc".chars();
    ///
    /// assert_eq!(b"abc", chars.as_bytes());
    /// chars.next();
    /// assert_eq!(b"bc", chars.as_bytes());
    /// chars.next();
    /// chars.next();
    /// assert_eq!(b"", chars.as_bytes());
    /// ```
    #[inline]
    pub fn as_bytes(&self) -> &'a [u8] {
        self.bs
    }
}
impl<'a> Iterator for Chars<'a> {
    type Item = char;
    #[inline]
    fn next(&mut self) -> Option<char> {
        let (ch, size) = decode_lossy(self.bs);
        if size == 0 {
            return None;
        }
        self.bs = &self.bs[size..];
        Some(ch)
    }
}
impl<'a> DoubleEndedIterator for Chars<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<char> {
        let (ch, size) = decode_last_lossy(self.bs);
        if size == 0 {
            return None;
        }
        self.bs = &self.bs[..self.bs.len() - size];
        Some(ch)
    }
}
/// An iterator over Unicode scalar values in a byte string and their
/// byte index positions.
///
/// When invalid UTF-8 byte sequences are found, they are substituted with the
/// Unicode replacement codepoint (`U+FFFD`) using the
/// ["maximal subpart" strategy](http://www.unicode.org/review/pr-121.html).
///
/// Note that this is slightly different from the `CharIndices` iterator
/// provided by the standard library. Aside from working on possibly invalid
/// UTF-8, this iterator provides both the corresponding starting and ending
/// byte indices of each codepoint yielded. The ending position is necessary to
/// slice the original byte string when invalid UTF-8 bytes are converted into
/// a Unicode replacement codepoint, since a single replacement codepoint can
/// substitute anywhere from 1 to 3 invalid bytes (inclusive).
///
/// This iterator is created by the
/// [`char_indices`](trait.ByteSlice.html#method.char_indices) method provided
/// by the [`ByteSlice`](trait.ByteSlice.html) extension trait for `&[u8]`.
#[derive(Clone, Debug)]
pub struct CharIndices<'a> {
    bs: &'a [u8],
    forward_index: usize,
    reverse_index: usize,
}
impl<'a> CharIndices<'a> {
    pub(crate) fn new(bs: &'a [u8]) -> CharIndices<'a> {
        CharIndices {
            bs: bs,
            forward_index: 0,
            reverse_index: bs.len(),
        }
    }
    /// View the underlying data as a subslice of the original data.
    ///
    /// The slice returned has the same lifetime as the original slice, and so
    /// the iterator can continue to be used while this exists.
    ///
    /// # Examples
    ///
    /// ```
    /// use bstr::ByteSlice;
    ///
    /// let mut it = b"abc".char_indices();
    ///
    /// assert_eq!(b"abc", it.as_bytes());
    /// it.next();
    /// assert_eq!(b"bc", it.as_bytes());
    /// it.next();
    /// it.next();
    /// assert_eq!(b"", it.as_bytes());
    /// ```
    #[inline]
    pub fn as_bytes(&self) -> &'a [u8] {
        self.bs
    }
}
impl<'a> Iterator for CharIndices<'a> {
    type Item = (usize, usize, char);
    #[inline]
    fn next(&mut self) -> Option<(usize, usize, char)> {
        let index = self.forward_index;
        let (ch, size) = decode_lossy(self.bs);
        if size == 0 {
            return None;
        }
        self.bs = &self.bs[size..];
        self.forward_index += size;
        Some((index, index + size, ch))
    }
}
impl<'a> DoubleEndedIterator for CharIndices<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<(usize, usize, char)> {
        let (ch, size) = decode_last_lossy(self.bs);
        if size == 0 {
            return None;
        }
        self.bs = &self.bs[..self.bs.len() - size];
        self.reverse_index -= size;
        Some((self.reverse_index, self.reverse_index + size, ch))
    }
}
impl<'a> ::core::iter::FusedIterator for CharIndices<'a> {}
/// An iterator over chunks of valid UTF-8 in a byte slice.
///
/// See [`utf8_chunks`](trait.ByteSlice.html#method.utf8_chunks).
#[derive(Clone, Debug)]
pub struct Utf8Chunks<'a> {
    pub(super) bytes: &'a [u8],
}
/// A chunk of valid UTF-8, possibly followed by invalid UTF-8 bytes.
///
/// This is yielded by the
/// [`Utf8Chunks`](struct.Utf8Chunks.html)
/// iterator, which can be created via the
/// [`ByteSlice::utf8_chunks`](trait.ByteSlice.html#method.utf8_chunks)
/// method.
///
/// The `'a` lifetime parameter corresponds to the lifetime of the bytes that
/// are being iterated over.
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct Utf8Chunk<'a> {
    /// A valid UTF-8 piece, at the start, end, or between invalid UTF-8 bytes.
    ///
    /// This is empty between adjacent invalid UTF-8 byte sequences.
    valid: &'a str,
    /// A sequence of invalid UTF-8 bytes.
    ///
    /// Can only be empty in the last chunk.
    ///
    /// Should be replaced by a single unicode replacement character, if not
    /// empty.
    invalid: &'a BStr,
    /// Indicates whether the invalid sequence could've been valid if there
    /// were more bytes.
    ///
    /// Can only be true in the last chunk.
    incomplete: bool,
}
impl<'a> Utf8Chunk<'a> {
    /// Returns the (possibly empty) valid UTF-8 bytes in this chunk.
    ///
    /// This may be empty if there are consecutive sequences of invalid UTF-8
    /// bytes.
    #[inline]
    pub fn valid(&self) -> &'a str {
        self.valid
    }
    /// Returns the (possibly empty) invalid UTF-8 bytes in this chunk that
    /// immediately follow the valid UTF-8 bytes in this chunk.
    ///
    /// This is only empty when this chunk corresponds to the last chunk in
    /// the original bytes.
    ///
    /// The maximum length of this slice is 3. That is, invalid UTF-8 byte
    /// sequences greater than 1 always correspond to a valid _prefix_ of
    /// a valid UTF-8 encoded codepoint. This corresponds to the "substitution
    /// of maximal subparts" strategy that is described in more detail in the
    /// docs for the
    /// [`ByteSlice::to_str_lossy`](trait.ByteSlice.html#method.to_str_lossy)
    /// method.
    #[inline]
    pub fn invalid(&self) -> &'a [u8] {
        self.invalid.as_bytes()
    }
    /// Returns whether the invalid sequence might still become valid if more
    /// bytes are added.
    ///
    /// Returns true if the end of the input was reached unexpectedly,
    /// without encountering an unexpected byte.
    ///
    /// This can only be the case for the last chunk.
    #[inline]
    pub fn incomplete(&self) -> bool {
        self.incomplete
    }
}
impl<'a> Iterator for Utf8Chunks<'a> {
    type Item = Utf8Chunk<'a>;
    #[inline]
    fn next(&mut self) -> Option<Utf8Chunk<'a>> {
        if self.bytes.is_empty() {
            return None;
        }
        match validate(self.bytes) {
            Ok(()) => {
                let valid = self.bytes;
                self.bytes = &[];
                Some(Utf8Chunk {
                    valid: unsafe { str::from_utf8_unchecked(valid) },
                    invalid: [].as_bstr(),
                    incomplete: false,
                })
            }
            Err(e) => {
                let (valid, rest) = self.bytes.split_at(e.valid_up_to());
                let valid = unsafe { str::from_utf8_unchecked(valid) };
                let (invalid_len, incomplete) = match e.error_len() {
                    Some(n) => (n, false),
                    None => (rest.len(), true),
                };
                let (invalid, rest) = rest.split_at(invalid_len);
                self.bytes = rest;
                Some(Utf8Chunk {
                    valid,
                    invalid: invalid.as_bstr(),
                    incomplete,
                })
            }
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.bytes.is_empty() { (0, Some(0)) } else { (1, Some(self.bytes.len())) }
    }
}
impl<'a> ::core::iter::FusedIterator for Utf8Chunks<'a> {}
/// An error that occurs when UTF-8 decoding fails.
///
/// This error occurs when attempting to convert a non-UTF-8 byte
/// string to a Rust string that must be valid UTF-8. For example,
/// [`to_str`](trait.ByteSlice.html#method.to_str) is one such method.
///
/// # Example
///
/// This example shows what happens when a given byte sequence is invalid,
/// but ends with a sequence that is a possible prefix of valid UTF-8.
///
/// ```
/// use bstr::{B, ByteSlice};
///
/// let s = B(b"foobar\xF1\x80\x80");
/// let err = s.to_str().unwrap_err();
/// assert_eq!(err.valid_up_to(), 6);
/// assert_eq!(err.error_len(), None);
/// ```
///
/// This example shows what happens when a given byte sequence contains
/// invalid UTF-8.
///
/// ```
/// use bstr::ByteSlice;
///
/// let s = b"foobar\xF1\x80\x80quux";
/// let err = s.to_str().unwrap_err();
/// assert_eq!(err.valid_up_to(), 6);
/// // The error length reports the maximum number of bytes that correspond to
/// // a valid prefix of a UTF-8 encoded codepoint.
/// assert_eq!(err.error_len(), Some(3));
///
/// // In contrast to the above which contains a single invalid prefix,
/// // consider the case of multiple individal bytes that are never valid
/// // prefixes. Note how the value of error_len changes!
/// let s = b"foobar\xFF\xFFquux";
/// let err = s.to_str().unwrap_err();
/// assert_eq!(err.valid_up_to(), 6);
/// assert_eq!(err.error_len(), Some(1));
///
/// // The fact that it's an invalid prefix does not change error_len even
/// // when it immediately precedes the end of the string.
/// let s = b"foobar\xFF";
/// let err = s.to_str().unwrap_err();
/// assert_eq!(err.valid_up_to(), 6);
/// assert_eq!(err.error_len(), Some(1));
/// ```
#[derive(Debug, Eq, PartialEq)]
pub struct Utf8Error {
    valid_up_to: usize,
    error_len: Option<usize>,
}
impl Utf8Error {
    /// Returns the byte index of the position immediately following the last
    /// valid UTF-8 byte.
    ///
    /// # Example
    ///
    /// This examples shows how `valid_up_to` can be used to retrieve a
    /// possibly empty prefix that is guaranteed to be valid UTF-8:
    ///
    /// ```
    /// use bstr::ByteSlice;
    ///
    /// let s = b"foobar\xF1\x80\x80quux";
    /// let err = s.to_str().unwrap_err();
    ///
    /// // This is guaranteed to never panic.
    /// let string = s[..err.valid_up_to()].to_str().unwrap();
    /// assert_eq!(string, "foobar");
    /// ```
    #[inline]
    pub fn valid_up_to(&self) -> usize {
        self.valid_up_to
    }
    /// Returns the total number of invalid UTF-8 bytes immediately following
    /// the position returned by `valid_up_to`. This value is always at least
    /// `1`, but can be up to `3` if bytes form a valid prefix of some UTF-8
    /// encoded codepoint.
    ///
    /// If the end of the original input was found before a valid UTF-8 encoded
    /// codepoint could be completed, then this returns `None`. This is useful
    /// when processing streams, where a `None` value signals that more input
    /// might be needed.
    #[inline]
    pub fn error_len(&self) -> Option<usize> {
        self.error_len
    }
}
#[cfg(feature = "std")]
impl error::Error for Utf8Error {
    fn description(&self) -> &str {
        "invalid UTF-8"
    }
}
impl fmt::Display for Utf8Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid UTF-8 found at byte offset {}", self.valid_up_to)
    }
}
/// Returns OK if and only if the given slice is completely valid UTF-8.
///
/// If the slice isn't valid UTF-8, then an error is returned that explains
/// the first location at which invalid UTF-8 was detected.
pub fn validate(slice: &[u8]) -> Result<(), Utf8Error> {
    fn fast(slice: &[u8]) -> Result<(), Utf8Error> {
        let mut state = ACCEPT;
        let mut i = 0;
        while i < slice.len() {
            let b = slice[i];
            if state == ACCEPT && b <= 0x7F
                && slice.get(i + 1).map_or(false, |&b| b <= 0x7F)
            {
                i += ascii::first_non_ascii_byte(&slice[i..]);
                continue;
            }
            state = step(state, b);
            if state == REJECT {
                return Err(find_valid_up_to(slice, i));
            }
            i += 1;
        }
        if state != ACCEPT { Err(find_valid_up_to(slice, slice.len())) } else { Ok(()) }
    }
    #[inline(never)]
    fn find_valid_up_to(slice: &[u8], rejected_at: usize) -> Utf8Error {
        let mut backup = rejected_at.saturating_sub(1);
        while backup > 0 && !is_leading_or_invalid_utf8_byte(slice[backup]) {
            backup -= 1;
        }
        let upto = cmp::min(slice.len(), rejected_at.saturating_add(1));
        let mut err = slow(&slice[backup..upto]).unwrap_err();
        err.valid_up_to += backup;
        err
    }
    fn slow(slice: &[u8]) -> Result<(), Utf8Error> {
        let mut state = ACCEPT;
        let mut valid_up_to = 0;
        for (i, &b) in slice.iter().enumerate() {
            state = step(state, b);
            if state == ACCEPT {
                valid_up_to = i + 1;
            } else if state == REJECT {
                let error_len = Some(cmp::max(1, i - valid_up_to));
                return Err(Utf8Error {
                    valid_up_to,
                    error_len,
                });
            }
        }
        if state != ACCEPT {
            Err(Utf8Error {
                valid_up_to,
                error_len: None,
            })
        } else {
            Ok(())
        }
    }
    fn step(state: usize, b: u8) -> usize {
        let class = CLASSES[b as usize];
        unsafe { *STATES_FORWARD.get_unchecked(state + class as usize) as usize }
    }
    fast(slice)
}
/// UTF-8 decode a single Unicode scalar value from the beginning of a slice.
///
/// When successful, the corresponding Unicode scalar value is returned along
/// with the number of bytes it was encoded with. The number of bytes consumed
/// for a successful decode is always between 1 and 4, inclusive.
///
/// When unsuccessful, `None` is returned along with the number of bytes that
/// make up a maximal prefix of a valid UTF-8 code unit sequence. In this case,
/// the number of bytes consumed is always between 0 and 3, inclusive, where
/// 0 is only returned when `slice` is empty.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use bstr::decode_utf8;
///
/// // Decoding a valid codepoint.
/// let (ch, size) = decode_utf8(b"\xE2\x98\x83");
/// assert_eq!(Some('☃'), ch);
/// assert_eq!(3, size);
///
/// // Decoding an incomplete codepoint.
/// let (ch, size) = decode_utf8(b"\xE2\x98");
/// assert_eq!(None, ch);
/// assert_eq!(2, size);
/// ```
///
/// This example shows how to iterate over all codepoints in UTF-8 encoded
/// bytes, while replacing invalid UTF-8 sequences with the replacement
/// codepoint:
///
/// ```
/// use bstr::{B, decode_utf8};
///
/// let mut bytes = B(b"\xE2\x98\x83\xFF\xF0\x9D\x9E\x83\xE2\x98\x61");
/// let mut chars = vec![];
/// while !bytes.is_empty() {
///     let (ch, size) = decode_utf8(bytes);
///     bytes = &bytes[size..];
///     chars.push(ch.unwrap_or('\u{FFFD}'));
/// }
/// assert_eq!(vec!['☃', '\u{FFFD}', '𝞃', '\u{FFFD}', 'a'], chars);
/// ```
#[inline]
pub fn decode<B: AsRef<[u8]>>(slice: B) -> (Option<char>, usize) {
    let slice = slice.as_ref();
    match slice.get(0) {
        None => return (None, 0),
        Some(&b) if b <= 0x7F => return (Some(b as char), 1),
        _ => {}
    }
    let (mut state, mut cp, mut i) = (ACCEPT, 0, 0);
    while i < slice.len() {
        decode_step(&mut state, &mut cp, slice[i]);
        i += 1;
        if state == ACCEPT {
            let ch = unsafe { char::from_u32_unchecked(cp) };
            return (Some(ch), i);
        } else if state == REJECT {
            return (None, cmp::max(1, i.saturating_sub(1)));
        }
    }
    (None, i)
}
/// Lossily UTF-8 decode a single Unicode scalar value from the beginning of a
/// slice.
///
/// When successful, the corresponding Unicode scalar value is returned along
/// with the number of bytes it was encoded with. The number of bytes consumed
/// for a successful decode is always between 1 and 4, inclusive.
///
/// When unsuccessful, the Unicode replacement codepoint (`U+FFFD`) is returned
/// along with the number of bytes that make up a maximal prefix of a valid
/// UTF-8 code unit sequence. In this case, the number of bytes consumed is
/// always between 0 and 3, inclusive, where 0 is only returned when `slice` is
/// empty.
///
/// # Examples
///
/// Basic usage:
///
/// ```ignore
/// use bstr::decode_utf8_lossy;
///
/// // Decoding a valid codepoint.
/// let (ch, size) = decode_utf8_lossy(b"\xE2\x98\x83");
/// assert_eq!('☃', ch);
/// assert_eq!(3, size);
///
/// // Decoding an incomplete codepoint.
/// let (ch, size) = decode_utf8_lossy(b"\xE2\x98");
/// assert_eq!('\u{FFFD}', ch);
/// assert_eq!(2, size);
/// ```
///
/// This example shows how to iterate over all codepoints in UTF-8 encoded
/// bytes, while replacing invalid UTF-8 sequences with the replacement
/// codepoint:
///
/// ```ignore
/// use bstr::{B, decode_utf8_lossy};
///
/// let mut bytes = B(b"\xE2\x98\x83\xFF\xF0\x9D\x9E\x83\xE2\x98\x61");
/// let mut chars = vec![];
/// while !bytes.is_empty() {
///     let (ch, size) = decode_utf8_lossy(bytes);
///     bytes = &bytes[size..];
///     chars.push(ch);
/// }
/// assert_eq!(vec!['☃', '\u{FFFD}', '𝞃', '\u{FFFD}', 'a'], chars);
/// ```
#[inline]
pub fn decode_lossy<B: AsRef<[u8]>>(slice: B) -> (char, usize) {
    match decode(slice) {
        (Some(ch), size) => (ch, size),
        (None, size) => ('\u{FFFD}', size),
    }
}
/// UTF-8 decode a single Unicode scalar value from the end of a slice.
///
/// When successful, the corresponding Unicode scalar value is returned along
/// with the number of bytes it was encoded with. The number of bytes consumed
/// for a successful decode is always between 1 and 4, inclusive.
///
/// When unsuccessful, `None` is returned along with the number of bytes that
/// make up a maximal prefix of a valid UTF-8 code unit sequence. In this case,
/// the number of bytes consumed is always between 0 and 3, inclusive, where
/// 0 is only returned when `slice` is empty.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use bstr::decode_last_utf8;
///
/// // Decoding a valid codepoint.
/// let (ch, size) = decode_last_utf8(b"\xE2\x98\x83");
/// assert_eq!(Some('☃'), ch);
/// assert_eq!(3, size);
///
/// // Decoding an incomplete codepoint.
/// let (ch, size) = decode_last_utf8(b"\xE2\x98");
/// assert_eq!(None, ch);
/// assert_eq!(2, size);
/// ```
///
/// This example shows how to iterate over all codepoints in UTF-8 encoded
/// bytes in reverse, while replacing invalid UTF-8 sequences with the
/// replacement codepoint:
///
/// ```
/// use bstr::{B, decode_last_utf8};
///
/// let mut bytes = B(b"\xE2\x98\x83\xFF\xF0\x9D\x9E\x83\xE2\x98\x61");
/// let mut chars = vec![];
/// while !bytes.is_empty() {
///     let (ch, size) = decode_last_utf8(bytes);
///     bytes = &bytes[..bytes.len()-size];
///     chars.push(ch.unwrap_or('\u{FFFD}'));
/// }
/// assert_eq!(vec!['a', '\u{FFFD}', '𝞃', '\u{FFFD}', '☃'], chars);
/// ```
#[inline]
pub fn decode_last<B: AsRef<[u8]>>(slice: B) -> (Option<char>, usize) {
    let slice = slice.as_ref();
    if slice.is_empty() {
        return (None, 0);
    }
    let mut start = slice.len() - 1;
    let limit = slice.len().saturating_sub(4);
    while start > limit && !is_leading_or_invalid_utf8_byte(slice[start]) {
        start -= 1;
    }
    let (ch, size) = decode(&slice[start..]);
    if start + size != slice.len() { (None, 1) } else { (ch, size) }
}
/// Lossily UTF-8 decode a single Unicode scalar value from the end of a slice.
///
/// When successful, the corresponding Unicode scalar value is returned along
/// with the number of bytes it was encoded with. The number of bytes consumed
/// for a successful decode is always between 1 and 4, inclusive.
///
/// When unsuccessful, the Unicode replacement codepoint (`U+FFFD`) is returned
/// along with the number of bytes that make up a maximal prefix of a valid
/// UTF-8 code unit sequence. In this case, the number of bytes consumed is
/// always between 0 and 3, inclusive, where 0 is only returned when `slice` is
/// empty.
///
/// # Examples
///
/// Basic usage:
///
/// ```ignore
/// use bstr::decode_last_utf8_lossy;
///
/// // Decoding a valid codepoint.
/// let (ch, size) = decode_last_utf8_lossy(b"\xE2\x98\x83");
/// assert_eq!('☃', ch);
/// assert_eq!(3, size);
///
/// // Decoding an incomplete codepoint.
/// let (ch, size) = decode_last_utf8_lossy(b"\xE2\x98");
/// assert_eq!('\u{FFFD}', ch);
/// assert_eq!(2, size);
/// ```
///
/// This example shows how to iterate over all codepoints in UTF-8 encoded
/// bytes in reverse, while replacing invalid UTF-8 sequences with the
/// replacement codepoint:
///
/// ```ignore
/// use bstr::decode_last_utf8_lossy;
///
/// let mut bytes = B(b"\xE2\x98\x83\xFF\xF0\x9D\x9E\x83\xE2\x98\x61");
/// let mut chars = vec![];
/// while !bytes.is_empty() {
///     let (ch, size) = decode_last_utf8_lossy(bytes);
///     bytes = &bytes[..bytes.len()-size];
///     chars.push(ch);
/// }
/// assert_eq!(vec!['a', '\u{FFFD}', '𝞃', '\u{FFFD}', '☃'], chars);
/// ```
#[inline]
pub fn decode_last_lossy<B: AsRef<[u8]>>(slice: B) -> (char, usize) {
    match decode_last(slice) {
        (Some(ch), size) => (ch, size),
        (None, size) => ('\u{FFFD}', size),
    }
}
/// SAFETY: The decode function relies on state being equal to ACCEPT only if
/// cp is a valid Unicode scalar value.
#[inline]
pub fn decode_step(state: &mut usize, cp: &mut u32, b: u8) {
    let class = CLASSES[b as usize];
    if *state == ACCEPT {
        *cp = (0xFF >> class) & (b as u32);
    } else {
        *cp = (b as u32 & 0b111111) | (*cp << 6);
    }
    *state = STATES_FORWARD[*state + class as usize] as usize;
}
/// Returns true if and only if the given byte is either a valid leading UTF-8
/// byte, or is otherwise an invalid byte that can never appear anywhere in a
/// valid UTF-8 sequence.
fn is_leading_or_invalid_utf8_byte(b: u8) -> bool {
    (b & 0b1100_0000) != 0b1000_0000
}
#[cfg(test)]
mod tests {
    use std::char;
    use ext_slice::{ByteSlice, B};
    use tests::LOSSY_TESTS;
    use utf8::{self, Utf8Error};
    fn utf8e(valid_up_to: usize) -> Utf8Error {
        Utf8Error {
            valid_up_to,
            error_len: None,
        }
    }
    fn utf8e2(valid_up_to: usize, error_len: usize) -> Utf8Error {
        Utf8Error {
            valid_up_to,
            error_len: Some(error_len),
        }
    }
    #[test]
    fn validate_all_codepoints() {
        for i in 0..(0x10FFFF + 1) {
            let cp = match char::from_u32(i) {
                None => continue,
                Some(cp) => cp,
            };
            let mut buf = [0; 4];
            let s = cp.encode_utf8(&mut buf);
            assert_eq!(Ok(()), utf8::validate(s.as_bytes()));
        }
    }
    #[test]
    fn validate_multiple_codepoints() {
        assert_eq!(Ok(()), utf8::validate(b"abc"));
        assert_eq!(Ok(()), utf8::validate(b"a\xE2\x98\x83a"));
        assert_eq!(Ok(()), utf8::validate(b"a\xF0\x9D\x9C\xB7a"));
        assert_eq!(Ok(()), utf8::validate(b"\xE2\x98\x83\xF0\x9D\x9C\xB7",));
        assert_eq!(Ok(()), utf8::validate(b"a\xE2\x98\x83a\xF0\x9D\x9C\xB7a",));
        assert_eq!(Ok(()), utf8::validate(b"\xEF\xBF\xBD\xE2\x98\x83\xEF\xBF\xBD",));
    }
    #[test]
    fn validate_errors() {
        assert_eq!(Err(utf8e2(0, 1)), utf8::validate(b"\xFF"));
        assert_eq!(Err(utf8e2(1, 1)), utf8::validate(b"a\xFF"));
        assert_eq!(Err(utf8e2(2, 1)), utf8::validate(b"\xCE\xB2\xFF"));
        assert_eq!(Err(utf8e2(3, 1)), utf8::validate(b"\xE2\x98\x83\xFF"));
        assert_eq!(Err(utf8e2(4, 1)), utf8::validate(b"\xF0\x9D\x9D\xB1\xFF"));
        assert_eq!(Err(utf8e2(0, 1)), utf8::validate(b"\xCE\xF0"));
        assert_eq!(Err(utf8e2(0, 2)), utf8::validate(b"\xE2\x98\xF0"));
        assert_eq!(Err(utf8e2(0, 3)), utf8::validate(b"\xF0\x9D\x9D\xF0"));
        assert_eq!(Err(utf8e2(0, 1)), utf8::validate(b"\xF0\x82\x82\xAC"));
        assert_eq!(Err(utf8e2(1, 1)), utf8::validate(b"a\xF0\x82\x82\xAC"));
        assert_eq!(Err(utf8e2(3, 1)), utf8::validate(b"\xE2\x98\x83\xF0\x82\x82\xAC",));
        assert_eq!(Err(utf8e2(0, 1)), utf8::validate(b"\xED\xA0\x80"));
        assert_eq!(Err(utf8e2(1, 1)), utf8::validate(b"a\xED\xA0\x80"));
        assert_eq!(Err(utf8e2(3, 1)), utf8::validate(b"\xE2\x98\x83\xED\xA0\x80",));
        assert_eq!(Err(utf8e2(0, 1)), utf8::validate(b"\xCEa"));
        assert_eq!(Err(utf8e2(1, 1)), utf8::validate(b"a\xCEa"));
        assert_eq!(Err(utf8e2(3, 1)), utf8::validate(b"\xE2\x98\x83\xCE\xE2\x98\x83",));
        assert_eq!(Err(utf8e2(0, 2)), utf8::validate(b"\xE2\x98a"));
        assert_eq!(Err(utf8e2(1, 2)), utf8::validate(b"a\xE2\x98a"));
        assert_eq!(
            Err(utf8e2(3, 2)), utf8::validate(b"\xE2\x98\x83\xE2\x98\xE2\x98\x83",)
        );
        assert_eq!(Err(utf8e2(0, 3)), utf8::validate(b"\xF0\x9D\x9Ca"));
        assert_eq!(Err(utf8e2(1, 3)), utf8::validate(b"a\xF0\x9D\x9Ca"));
        assert_eq!(
            Err(utf8e2(4, 3)),
            utf8::validate(b"\xF0\x9D\x9C\xB1\xF0\x9D\x9C\xE2\x98\x83",)
        );
        assert_eq!(Err(utf8e2(6, 3)), utf8::validate(b"foobar\xF1\x80\x80quux",));
        assert_eq!(Err(utf8e(0)), utf8::validate(b"\xCE"));
        assert_eq!(Err(utf8e(1)), utf8::validate(b"a\xCE"));
        assert_eq!(Err(utf8e(3)), utf8::validate(b"\xE2\x98\x83\xCE"));
        assert_eq!(Err(utf8e(0)), utf8::validate(b"\xE2\x98"));
        assert_eq!(Err(utf8e(1)), utf8::validate(b"a\xE2\x98"));
        assert_eq!(Err(utf8e(3)), utf8::validate(b"\xE2\x98\x83\xE2\x98"));
        assert_eq!(Err(utf8e(0)), utf8::validate(b"\xF0\x9D\x9C"));
        assert_eq!(Err(utf8e(1)), utf8::validate(b"a\xF0\x9D\x9C"));
        assert_eq!(Err(utf8e(4)), utf8::validate(b"\xF0\x9D\x9C\xB1\xF0\x9D\x9C",));
        assert_eq!(
            Err(utf8e2(8, 1)), utf8::validate(b"\xe2\x98\x83\xce\xb2\xe3\x83\x84\xFF",)
        );
    }
    #[test]
    fn decode_valid() {
        fn d(mut s: &str) -> Vec<char> {
            let mut chars = vec![];
            while !s.is_empty() {
                let (ch, size) = utf8::decode(s.as_bytes());
                s = &s[size..];
                chars.push(ch.unwrap());
            }
            chars
        }
        assert_eq!(vec!['☃'], d("☃"));
        assert_eq!(vec!['☃', '☃'], d("☃☃"));
        assert_eq!(vec!['α', 'β', 'γ', 'δ', 'ε'], d("αβγδε"));
        assert_eq!(vec!['☃', '⛄', '⛇'], d("☃⛄⛇"));
        assert_eq!(
            vec!['𝗮', '𝗯', '𝗰', '𝗱', '𝗲'], d("𝗮𝗯𝗰𝗱𝗲")
        );
    }
    #[test]
    fn decode_invalid() {
        let (ch, size) = utf8::decode(b"");
        assert_eq!(None, ch);
        assert_eq!(0, size);
        let (ch, size) = utf8::decode(b"\xFF");
        assert_eq!(None, ch);
        assert_eq!(1, size);
        let (ch, size) = utf8::decode(b"\xCE\xF0");
        assert_eq!(None, ch);
        assert_eq!(1, size);
        let (ch, size) = utf8::decode(b"\xE2\x98\xF0");
        assert_eq!(None, ch);
        assert_eq!(2, size);
        let (ch, size) = utf8::decode(b"\xF0\x9D\x9D");
        assert_eq!(None, ch);
        assert_eq!(3, size);
        let (ch, size) = utf8::decode(b"\xF0\x9D\x9D\xF0");
        assert_eq!(None, ch);
        assert_eq!(3, size);
        let (ch, size) = utf8::decode(b"\xF0\x82\x82\xAC");
        assert_eq!(None, ch);
        assert_eq!(1, size);
        let (ch, size) = utf8::decode(b"\xED\xA0\x80");
        assert_eq!(None, ch);
        assert_eq!(1, size);
        let (ch, size) = utf8::decode(b"\xCEa");
        assert_eq!(None, ch);
        assert_eq!(1, size);
        let (ch, size) = utf8::decode(b"\xE2\x98a");
        assert_eq!(None, ch);
        assert_eq!(2, size);
        let (ch, size) = utf8::decode(b"\xF0\x9D\x9Ca");
        assert_eq!(None, ch);
        assert_eq!(3, size);
    }
    #[test]
    fn decode_lossy() {
        let (ch, size) = utf8::decode_lossy(b"");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(0, size);
        let (ch, size) = utf8::decode_lossy(b"\xFF");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(1, size);
        let (ch, size) = utf8::decode_lossy(b"\xCE\xF0");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(1, size);
        let (ch, size) = utf8::decode_lossy(b"\xE2\x98\xF0");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(2, size);
        let (ch, size) = utf8::decode_lossy(b"\xF0\x9D\x9D\xF0");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(3, size);
        let (ch, size) = utf8::decode_lossy(b"\xF0\x82\x82\xAC");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(1, size);
        let (ch, size) = utf8::decode_lossy(b"\xED\xA0\x80");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(1, size);
        let (ch, size) = utf8::decode_lossy(b"\xCEa");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(1, size);
        let (ch, size) = utf8::decode_lossy(b"\xE2\x98a");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(2, size);
        let (ch, size) = utf8::decode_lossy(b"\xF0\x9D\x9Ca");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(3, size);
    }
    #[test]
    fn decode_last_valid() {
        fn d(mut s: &str) -> Vec<char> {
            let mut chars = vec![];
            while !s.is_empty() {
                let (ch, size) = utf8::decode_last(s.as_bytes());
                s = &s[..s.len() - size];
                chars.push(ch.unwrap());
            }
            chars
        }
        assert_eq!(vec!['☃'], d("☃"));
        assert_eq!(vec!['☃', '☃'], d("☃☃"));
        assert_eq!(vec!['ε', 'δ', 'γ', 'β', 'α'], d("αβγδε"));
        assert_eq!(vec!['⛇', '⛄', '☃'], d("☃⛄⛇"));
        assert_eq!(
            vec!['𝗲', '𝗱', '𝗰', '𝗯', '𝗮'], d("𝗮𝗯𝗰𝗱𝗲")
        );
    }
    #[test]
    fn decode_last_invalid() {
        let (ch, size) = utf8::decode_last(b"");
        assert_eq!(None, ch);
        assert_eq!(0, size);
        let (ch, size) = utf8::decode_last(b"\xFF");
        assert_eq!(None, ch);
        assert_eq!(1, size);
        let (ch, size) = utf8::decode_last(b"\xCE\xF0");
        assert_eq!(None, ch);
        assert_eq!(1, size);
        let (ch, size) = utf8::decode_last(b"\xCE");
        assert_eq!(None, ch);
        assert_eq!(1, size);
        let (ch, size) = utf8::decode_last(b"\xE2\x98\xF0");
        assert_eq!(None, ch);
        assert_eq!(1, size);
        let (ch, size) = utf8::decode_last(b"\xE2\x98");
        assert_eq!(None, ch);
        assert_eq!(2, size);
        let (ch, size) = utf8::decode_last(b"\xF0\x9D\x9D\xF0");
        assert_eq!(None, ch);
        assert_eq!(1, size);
        let (ch, size) = utf8::decode_last(b"\xF0\x9D\x9D");
        assert_eq!(None, ch);
        assert_eq!(3, size);
        let (ch, size) = utf8::decode_last(b"\xF0\x82\x82\xAC");
        assert_eq!(None, ch);
        assert_eq!(1, size);
        let (ch, size) = utf8::decode_last(b"\xED\xA0\x80");
        assert_eq!(None, ch);
        assert_eq!(1, size);
        let (ch, size) = utf8::decode_last(b"\xED\xA0");
        assert_eq!(None, ch);
        assert_eq!(1, size);
        let (ch, size) = utf8::decode_last(b"\xED");
        assert_eq!(None, ch);
        assert_eq!(1, size);
        let (ch, size) = utf8::decode_last(b"a\xCE");
        assert_eq!(None, ch);
        assert_eq!(1, size);
        let (ch, size) = utf8::decode_last(b"a\xE2\x98");
        assert_eq!(None, ch);
        assert_eq!(2, size);
        let (ch, size) = utf8::decode_last(b"a\xF0\x9D\x9C");
        assert_eq!(None, ch);
        assert_eq!(3, size);
    }
    #[test]
    fn decode_last_lossy() {
        let (ch, size) = utf8::decode_last_lossy(b"");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(0, size);
        let (ch, size) = utf8::decode_last_lossy(b"\xFF");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(1, size);
        let (ch, size) = utf8::decode_last_lossy(b"\xCE\xF0");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(1, size);
        let (ch, size) = utf8::decode_last_lossy(b"\xCE");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(1, size);
        let (ch, size) = utf8::decode_last_lossy(b"\xE2\x98\xF0");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(1, size);
        let (ch, size) = utf8::decode_last_lossy(b"\xE2\x98");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(2, size);
        let (ch, size) = utf8::decode_last_lossy(b"\xF0\x9D\x9D\xF0");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(1, size);
        let (ch, size) = utf8::decode_last_lossy(b"\xF0\x9D\x9D");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(3, size);
        let (ch, size) = utf8::decode_last_lossy(b"\xF0\x82\x82\xAC");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(1, size);
        let (ch, size) = utf8::decode_last_lossy(b"\xED\xA0\x80");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(1, size);
        let (ch, size) = utf8::decode_last_lossy(b"\xED\xA0");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(1, size);
        let (ch, size) = utf8::decode_last_lossy(b"\xED");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(1, size);
        let (ch, size) = utf8::decode_last_lossy(b"a\xCE");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(1, size);
        let (ch, size) = utf8::decode_last_lossy(b"a\xE2\x98");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(2, size);
        let (ch, size) = utf8::decode_last_lossy(b"a\xF0\x9D\x9C");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(3, size);
    }
    #[test]
    fn chars() {
        for (i, &(expected, input)) in LOSSY_TESTS.iter().enumerate() {
            let got: String = B(input).chars().collect();
            assert_eq!(expected, got, "chars(ith: {:?}, given: {:?})", i, input,);
            let got: String = B(input).char_indices().map(|(_, _, ch)| ch).collect();
            assert_eq!(expected, got, "char_indices(ith: {:?}, given: {:?})", i, input,);
            let expected: String = expected.chars().rev().collect();
            let got: String = B(input).chars().rev().collect();
            assert_eq!(expected, got, "chars.rev(ith: {:?}, given: {:?})", i, input,);
            let got: String = B(input)
                .char_indices()
                .rev()
                .map(|(_, _, ch)| ch)
                .collect();
            assert_eq!(
                expected, got, "char_indices.rev(ith: {:?}, given: {:?})", i, input,
            );
        }
    }
    #[test]
    fn utf8_chunks() {
        let mut c = utf8::Utf8Chunks {
            bytes: b"123\xC0",
        };
        assert_eq!(
            (c.next(), c.next()), (Some(utf8::Utf8Chunk { valid : "123", invalid :
            b"\xC0".as_bstr(), incomplete : false, }), None,)
        );
        let mut c = utf8::Utf8Chunks {
            bytes: b"123\xFF\xFF",
        };
        assert_eq!(
            (c.next(), c.next(), c.next()), (Some(utf8::Utf8Chunk { valid : "123",
            invalid : b"\xFF".as_bstr(), incomplete : false, }), Some(utf8::Utf8Chunk {
            valid : "", invalid : b"\xFF".as_bstr(), incomplete : false, }), None,)
        );
        let mut c = utf8::Utf8Chunks {
            bytes: b"123\xD0",
        };
        assert_eq!(
            (c.next(), c.next()), (Some(utf8::Utf8Chunk { valid : "123", invalid :
            b"\xD0".as_bstr(), incomplete : true, }), None,)
        );
        let mut c = utf8::Utf8Chunks {
            bytes: b"123\xD0456",
        };
        assert_eq!(
            (c.next(), c.next(), c.next()), (Some(utf8::Utf8Chunk { valid : "123",
            invalid : b"\xD0".as_bstr(), incomplete : false, }), Some(utf8::Utf8Chunk {
            valid : "456", invalid : b"".as_bstr(), incomplete : false, }), None,)
        );
        let mut c = utf8::Utf8Chunks {
            bytes: b"123\xE2\x98",
        };
        assert_eq!(
            (c.next(), c.next()), (Some(utf8::Utf8Chunk { valid : "123", invalid :
            b"\xE2\x98".as_bstr(), incomplete : true, }), None,)
        );
        let mut c = utf8::Utf8Chunks {
            bytes: b"123\xF4\x8F\xBF",
        };
        assert_eq!(
            (c.next(), c.next()), (Some(utf8::Utf8Chunk { valid : "123", invalid :
            b"\xF4\x8F\xBF".as_bstr(), incomplete : true, }), None,)
        );
    }
}
#[cfg(test)]
mod tests_rug_352 {
    use super::*;
    use crate::BStr;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_352_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"\xE2\x98\x61";
        let bytes: &[u8] = rug_fuzz_0;
        let p0 = BStr::new(bytes);
        crate::utf8::decode(p0);
        let _rug_ed_tests_rug_352_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_353 {
    use super::*;
    use crate::BStr;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_353_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"\xE2\x98\x83";
        let bytes: &[u8] = rug_fuzz_0;
        let p0 = BStr::new(bytes);
        crate::utf8::decode_lossy(p0);
        let _rug_ed_tests_rug_353_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_354 {
    use super::*;
    use crate::BStr;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_354_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"\xE2\x98\x83";
        let mut bytes: &[u8] = rug_fuzz_0;
        let p0 = BStr::new(bytes);
        crate::utf8::decode_last(p0);
        let _rug_ed_tests_rug_354_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_355 {
    use super::*;
    use crate::BStr;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_355_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"\xE2\x98\x83";
        let bytes: &[u8] = rug_fuzz_0;
        let p0 = BStr::new(bytes);
        crate::utf8::decode_last_lossy(p0);
        let _rug_ed_tests_rug_355_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_356 {
    use super::*;
    use crate::utf8::{decode_step, CLASSES, ACCEPT, STATES_FORWARD};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_356_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0b11010101;
        let mut p0: usize = ACCEPT;
        let mut p1: u32 = rug_fuzz_0;
        let p2: u8 = rug_fuzz_1;
        decode_step(&mut p0, &mut p1, p2);
        let _rug_ed_tests_rug_356_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_357 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_357_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0xC0;
        let rug_fuzz_1 = 0x41;
        let rug_fuzz_2 = 0x80;
        let mut p0: u8 = rug_fuzz_0;
        debug_assert_eq!(crate ::utf8::is_leading_or_invalid_utf8_byte(p0), true);
        p0 = rug_fuzz_1;
        debug_assert_eq!(crate ::utf8::is_leading_or_invalid_utf8_byte(p0), true);
        p0 = rug_fuzz_2;
        debug_assert_eq!(crate ::utf8::is_leading_or_invalid_utf8_byte(p0), true);
        let _rug_ed_tests_rug_357_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_358 {
    use super::*;
    use utf8::Chars;
    #[test]
    fn test_chars_new() {
        let _rug_st_tests_rug_358_rrrruuuugggg_test_chars_new = 0;
        let rug_fuzz_0 = b"hello";
        let p0: &[u8] = rug_fuzz_0;
        Chars::<'_>::new(p0);
        let _rug_ed_tests_rug_358_rrrruuuugggg_test_chars_new = 0;
    }
}
#[cfg(test)]
mod tests_rug_359 {
    use super::*;
    use crate::utf8;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_359_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"abc";
        let rug_fuzz_1 = b"abc";
        let rug_fuzz_2 = b"bc";
        let rug_fuzz_3 = b"";
        let data = rug_fuzz_0;
        let mut chars = utf8::Chars::new(data);
        debug_assert_eq!(rug_fuzz_1, chars.as_bytes());
        chars.next();
        debug_assert_eq!(rug_fuzz_2, chars.as_bytes());
        chars.next();
        chars.next();
        debug_assert_eq!(rug_fuzz_3, chars.as_bytes());
        let _rug_ed_tests_rug_359_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_360 {
    use super::*;
    use crate::std::iter::Iterator;
    use crate::utf8;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_360_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"Hello, World!";
        let data = rug_fuzz_0;
        let mut p0 = utf8::Chars::new(data);
        p0.next();
        let _rug_ed_tests_rug_360_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_361 {
    use super::*;
    use crate::std::iter::DoubleEndedIterator;
    use crate::utf8;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_361_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"Hello, World!";
        let data = rug_fuzz_0;
        let mut p0 = utf8::Chars::new(data);
        <utf8::Chars<'_> as DoubleEndedIterator>::next_back(&mut p0);
        let _rug_ed_tests_rug_361_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_362 {
    use super::*;
    use utf8::CharIndices;
    #[test]
    fn test_new() {
        let _rug_st_tests_rug_362_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = b"hello";
        let p0: &[u8] = rug_fuzz_0;
        CharIndices::new(p0);
        let _rug_ed_tests_rug_362_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_rug_363 {
    use super::*;
    use crate::CharIndices;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_363_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"abc";
        let rug_fuzz_1 = b"abc";
        let rug_fuzz_2 = b"bc";
        let rug_fuzz_3 = b"";
        let data = rug_fuzz_0;
        let mut p0 = CharIndices::new(data);
        debug_assert_eq!(rug_fuzz_1, p0.as_bytes());
        p0.next();
        debug_assert_eq!(rug_fuzz_2, p0.as_bytes());
        p0.next();
        p0.next();
        debug_assert_eq!(rug_fuzz_3, p0.as_bytes());
        let _rug_ed_tests_rug_363_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_367 {
    use super::*;
    use crate::utf8::Utf8Chunk;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_367_rrrruuuugggg_test_rug = 0;
        let mut p0: Utf8Chunk<'_> = unimplemented!();
        p0.invalid();
        let _rug_ed_tests_rug_367_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_371 {
    use super::*;
    use crate::utf8;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_371_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 6;
        let rug_fuzz_1 = 3;
        let p0 = utf8::Utf8Error {
            valid_up_to: rug_fuzz_0,
            error_len: Some(rug_fuzz_1),
        };
        let result = p0.valid_up_to();
        debug_assert_eq!(result, 6);
        let _rug_ed_tests_rug_371_rrrruuuugggg_test_rug = 0;
    }
}
