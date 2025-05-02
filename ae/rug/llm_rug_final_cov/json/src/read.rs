use crate::error::{Error, ErrorCode, Result};
use alloc::vec::Vec;
use core::char;
use core::cmp;
use core::ops::Deref;
use core::str;
#[cfg(feature = "std")]
use crate::io;
#[cfg(feature = "std")]
use crate::iter::LineColIterator;
#[cfg(feature = "raw_value")]
use crate::raw::BorrowedRawDeserializer;
#[cfg(all(feature = "raw_value", feature = "std"))]
use crate::raw::OwnedRawDeserializer;
#[cfg(feature = "raw_value")]
use serde::de::Visitor;
/// Trait used by the deserializer for iterating over input. This is manually
/// "specialized" for iterating over &[u8]. Once feature(specialization) is
/// stable we can use actual specialization.
///
/// This trait is sealed and cannot be implemented for types outside of
/// `serde_json`.
pub trait Read<'de>: private::Sealed {
    #[doc(hidden)]
    fn next(&mut self) -> Result<Option<u8>>;
    #[doc(hidden)]
    fn peek(&mut self) -> Result<Option<u8>>;
    /// Only valid after a call to peek(). Discards the peeked byte.
    #[doc(hidden)]
    fn discard(&mut self);
    /// Position of the most recent call to next().
    ///
    /// The most recent call was probably next() and not peek(), but this method
    /// should try to return a sensible result if the most recent call was
    /// actually peek() because we don't always know.
    ///
    /// Only called in case of an error, so performance is not important.
    #[doc(hidden)]
    fn position(&self) -> Position;
    /// Position of the most recent call to peek().
    ///
    /// The most recent call was probably peek() and not next(), but this method
    /// should try to return a sensible result if the most recent call was
    /// actually next() because we don't always know.
    ///
    /// Only called in case of an error, so performance is not important.
    #[doc(hidden)]
    fn peek_position(&self) -> Position;
    /// Offset from the beginning of the input to the next byte that would be
    /// returned by next() or peek().
    #[doc(hidden)]
    fn byte_offset(&self) -> usize;
    /// Assumes the previous byte was a quotation mark. Parses a JSON-escaped
    /// string until the next quotation mark using the given scratch space if
    /// necessary. The scratch space is initially empty.
    #[doc(hidden)]
    fn parse_str<'s>(
        &'s mut self,
        scratch: &'s mut Vec<u8>,
    ) -> Result<Reference<'de, 's, str>>;
    /// Assumes the previous byte was a quotation mark. Parses a JSON-escaped
    /// string until the next quotation mark using the given scratch space if
    /// necessary. The scratch space is initially empty.
    ///
    /// This function returns the raw bytes in the string with escape sequences
    /// expanded but without performing unicode validation.
    #[doc(hidden)]
    fn parse_str_raw<'s>(
        &'s mut self,
        scratch: &'s mut Vec<u8>,
    ) -> Result<Reference<'de, 's, [u8]>>;
    /// Assumes the previous byte was a quotation mark. Parses a JSON-escaped
    /// string until the next quotation mark but discards the data.
    #[doc(hidden)]
    fn ignore_str(&mut self) -> Result<()>;
    /// Assumes the previous byte was a hex escape sequnce ('\u') in a string.
    /// Parses next hexadecimal sequence.
    #[doc(hidden)]
    fn decode_hex_escape(&mut self) -> Result<u16>;
    /// Switch raw buffering mode on.
    ///
    /// This is used when deserializing `RawValue`.
    #[cfg(feature = "raw_value")]
    #[doc(hidden)]
    fn begin_raw_buffering(&mut self);
    /// Switch raw buffering mode off and provides the raw buffered data to the
    /// given visitor.
    #[cfg(feature = "raw_value")]
    #[doc(hidden)]
    fn end_raw_buffering<V>(&mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>;
    /// Whether StreamDeserializer::next needs to check the failed flag. True
    /// for IoRead, false for StrRead and SliceRead which can track failure by
    /// truncating their input slice to avoid the extra check on every next
    /// call.
    #[doc(hidden)]
    const should_early_return_if_failed: bool;
    /// Mark a persistent failure of StreamDeserializer, either by setting the
    /// flag or by truncating the input data.
    #[doc(hidden)]
    fn set_failed(&mut self, failed: &mut bool);
}
pub struct Position {
    pub line: usize,
    pub column: usize,
}
pub enum Reference<'b, 'c, T>
where
    T: ?Sized + 'static,
{
    Borrowed(&'b T),
    Copied(&'c T),
}
impl<'b, 'c, T> Deref for Reference<'b, 'c, T>
where
    T: ?Sized + 'static,
{
    type Target = T;
    fn deref(&self) -> &Self::Target {
        match *self {
            Reference::Borrowed(b) => b,
            Reference::Copied(c) => c,
        }
    }
}
/// JSON input source that reads from a std::io input stream.
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub struct IoRead<R>
where
    R: io::Read,
{
    iter: LineColIterator<io::Bytes<R>>,
    /// Temporary storage of peeked byte.
    ch: Option<u8>,
    #[cfg(feature = "raw_value")]
    raw_buffer: Option<Vec<u8>>,
}
/// JSON input source that reads from a slice of bytes.
pub struct SliceRead<'a> {
    slice: &'a [u8],
    /// Index of the *next* byte that will be returned by next() or peek().
    index: usize,
    #[cfg(feature = "raw_value")]
    raw_buffering_start_index: usize,
}
/// JSON input source that reads from a UTF-8 string.
pub struct StrRead<'a> {
    delegate: SliceRead<'a>,
    #[cfg(feature = "raw_value")]
    data: &'a str,
}
mod private {
    pub trait Sealed {}
}
#[cfg(feature = "std")]
impl<R> IoRead<R>
where
    R: io::Read,
{
    /// Create a JSON input source to read from a std::io input stream.
    pub fn new(reader: R) -> Self {
        IoRead {
            iter: LineColIterator::new(reader.bytes()),
            ch: None,
            #[cfg(feature = "raw_value")]
            raw_buffer: None,
        }
    }
}
#[cfg(feature = "std")]
impl<R> private::Sealed for IoRead<R>
where
    R: io::Read,
{}
#[cfg(feature = "std")]
impl<R> IoRead<R>
where
    R: io::Read,
{
    fn parse_str_bytes<'s, T, F>(
        &'s mut self,
        scratch: &'s mut Vec<u8>,
        validate: bool,
        result: F,
    ) -> Result<T>
    where
        T: 's,
        F: FnOnce(&'s Self, &'s [u8]) -> Result<T>,
    {
        loop {
            let ch = tri!(next_or_eof(self));
            if !ESCAPE[ch as usize] {
                scratch.push(ch);
                continue;
            }
            match ch {
                b'"' => {
                    return result(self, scratch);
                }
                b'\\' => {
                    tri!(parse_escape(self, validate, scratch));
                }
                _ => {
                    if validate {
                        return error(
                            self,
                            ErrorCode::ControlCharacterWhileParsingString,
                        );
                    }
                    scratch.push(ch);
                }
            }
        }
    }
}
#[cfg(feature = "std")]
impl<'de, R> Read<'de> for IoRead<R>
where
    R: io::Read,
{
    #[inline]
    fn next(&mut self) -> Result<Option<u8>> {
        match self.ch.take() {
            Some(ch) => {
                #[cfg(feature = "raw_value")]
                {
                    if let Some(buf) = &mut self.raw_buffer {
                        buf.push(ch);
                    }
                }
                Ok(Some(ch))
            }
            None => {
                match self.iter.next() {
                    Some(Err(err)) => Err(Error::io(err)),
                    Some(Ok(ch)) => {
                        #[cfg(feature = "raw_value")]
                        {
                            if let Some(buf) = &mut self.raw_buffer {
                                buf.push(ch);
                            }
                        }
                        Ok(Some(ch))
                    }
                    None => Ok(None),
                }
            }
        }
    }
    #[inline]
    fn peek(&mut self) -> Result<Option<u8>> {
        match self.ch {
            Some(ch) => Ok(Some(ch)),
            None => {
                match self.iter.next() {
                    Some(Err(err)) => Err(Error::io(err)),
                    Some(Ok(ch)) => {
                        self.ch = Some(ch);
                        Ok(self.ch)
                    }
                    None => Ok(None),
                }
            }
        }
    }
    #[cfg(not(feature = "raw_value"))]
    #[inline]
    fn discard(&mut self) {
        self.ch = None;
    }
    #[cfg(feature = "raw_value")]
    fn discard(&mut self) {
        if let Some(ch) = self.ch.take() {
            if let Some(buf) = &mut self.raw_buffer {
                buf.push(ch);
            }
        }
    }
    fn position(&self) -> Position {
        Position {
            line: self.iter.line(),
            column: self.iter.col(),
        }
    }
    fn peek_position(&self) -> Position {
        self.position()
    }
    fn byte_offset(&self) -> usize {
        match self.ch {
            Some(_) => self.iter.byte_offset() - 1,
            None => self.iter.byte_offset(),
        }
    }
    fn parse_str<'s>(
        &'s mut self,
        scratch: &'s mut Vec<u8>,
    ) -> Result<Reference<'de, 's, str>> {
        self.parse_str_bytes(scratch, true, as_str).map(Reference::Copied)
    }
    fn parse_str_raw<'s>(
        &'s mut self,
        scratch: &'s mut Vec<u8>,
    ) -> Result<Reference<'de, 's, [u8]>> {
        self.parse_str_bytes(scratch, false, |_, bytes| Ok(bytes)).map(Reference::Copied)
    }
    fn ignore_str(&mut self) -> Result<()> {
        loop {
            let ch = tri!(next_or_eof(self));
            if !ESCAPE[ch as usize] {
                continue;
            }
            match ch {
                b'"' => {
                    return Ok(());
                }
                b'\\' => {
                    tri!(ignore_escape(self));
                }
                _ => {
                    return error(self, ErrorCode::ControlCharacterWhileParsingString);
                }
            }
        }
    }
    fn decode_hex_escape(&mut self) -> Result<u16> {
        let mut n = 0;
        for _ in 0..4 {
            match decode_hex_val(tri!(next_or_eof(self))) {
                None => return error(self, ErrorCode::InvalidEscape),
                Some(val) => {
                    n = (n << 4) + val;
                }
            }
        }
        Ok(n)
    }
    #[cfg(feature = "raw_value")]
    fn begin_raw_buffering(&mut self) {
        self.raw_buffer = Some(Vec::new());
    }
    #[cfg(feature = "raw_value")]
    fn end_raw_buffering<V>(&mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let raw = self.raw_buffer.take().unwrap();
        let raw = match String::from_utf8(raw) {
            Ok(raw) => raw,
            Err(_) => return error(self, ErrorCode::InvalidUnicodeCodePoint),
        };
        visitor
            .visit_map(OwnedRawDeserializer {
                raw_value: Some(raw),
            })
    }
    const should_early_return_if_failed: bool = true;
    #[inline]
    #[cold]
    fn set_failed(&mut self, failed: &mut bool) {
        *failed = true;
    }
}
impl<'a> SliceRead<'a> {
    /// Create a JSON input source to read from a slice of bytes.
    pub fn new(slice: &'a [u8]) -> Self {
        SliceRead {
            slice,
            index: 0,
            #[cfg(feature = "raw_value")]
            raw_buffering_start_index: 0,
        }
    }
    fn position_of_index(&self, i: usize) -> Position {
        let mut position = Position { line: 1, column: 0 };
        for ch in &self.slice[..i] {
            match *ch {
                b'\n' => {
                    position.line += 1;
                    position.column = 0;
                }
                _ => {
                    position.column += 1;
                }
            }
        }
        position
    }
    /// The big optimization here over IoRead is that if the string contains no
    /// backslash escape sequences, the returned &str is a slice of the raw JSON
    /// data so we avoid copying into the scratch space.
    fn parse_str_bytes<'s, T, F>(
        &'s mut self,
        scratch: &'s mut Vec<u8>,
        validate: bool,
        result: F,
    ) -> Result<Reference<'a, 's, T>>
    where
        T: ?Sized + 's,
        F: for<'f> FnOnce(&'s Self, &'f [u8]) -> Result<&'f T>,
    {
        let mut start = self.index;
        loop {
            while self.index < self.slice.len()
                && !ESCAPE[self.slice[self.index] as usize]
            {
                self.index += 1;
            }
            if self.index == self.slice.len() {
                return error(self, ErrorCode::EofWhileParsingString);
            }
            match self.slice[self.index] {
                b'"' => {
                    if scratch.is_empty() {
                        let borrowed = &self.slice[start..self.index];
                        self.index += 1;
                        return result(self, borrowed).map(Reference::Borrowed);
                    } else {
                        scratch.extend_from_slice(&self.slice[start..self.index]);
                        self.index += 1;
                        return result(self, scratch).map(Reference::Copied);
                    }
                }
                b'\\' => {
                    scratch.extend_from_slice(&self.slice[start..self.index]);
                    self.index += 1;
                    tri!(parse_escape(self, validate, scratch));
                    start = self.index;
                }
                _ => {
                    self.index += 1;
                    if validate {
                        return error(
                            self,
                            ErrorCode::ControlCharacterWhileParsingString,
                        );
                    }
                }
            }
        }
    }
}
impl<'a> private::Sealed for SliceRead<'a> {}
impl<'a> Read<'a> for SliceRead<'a> {
    #[inline]
    fn next(&mut self) -> Result<Option<u8>> {
        Ok(
            if self.index < self.slice.len() {
                let ch = self.slice[self.index];
                self.index += 1;
                Some(ch)
            } else {
                None
            },
        )
    }
    #[inline]
    fn peek(&mut self) -> Result<Option<u8>> {
        Ok(
            if self.index < self.slice.len() {
                Some(self.slice[self.index])
            } else {
                None
            },
        )
    }
    #[inline]
    fn discard(&mut self) {
        self.index += 1;
    }
    fn position(&self) -> Position {
        self.position_of_index(self.index)
    }
    fn peek_position(&self) -> Position {
        self.position_of_index(cmp::min(self.slice.len(), self.index + 1))
    }
    fn byte_offset(&self) -> usize {
        self.index
    }
    fn parse_str<'s>(
        &'s mut self,
        scratch: &'s mut Vec<u8>,
    ) -> Result<Reference<'a, 's, str>> {
        self.parse_str_bytes(scratch, true, as_str)
    }
    fn parse_str_raw<'s>(
        &'s mut self,
        scratch: &'s mut Vec<u8>,
    ) -> Result<Reference<'a, 's, [u8]>> {
        self.parse_str_bytes(scratch, false, |_, bytes| Ok(bytes))
    }
    fn ignore_str(&mut self) -> Result<()> {
        loop {
            while self.index < self.slice.len()
                && !ESCAPE[self.slice[self.index] as usize]
            {
                self.index += 1;
            }
            if self.index == self.slice.len() {
                return error(self, ErrorCode::EofWhileParsingString);
            }
            match self.slice[self.index] {
                b'"' => {
                    self.index += 1;
                    return Ok(());
                }
                b'\\' => {
                    self.index += 1;
                    tri!(ignore_escape(self));
                }
                _ => {
                    return error(self, ErrorCode::ControlCharacterWhileParsingString);
                }
            }
        }
    }
    fn decode_hex_escape(&mut self) -> Result<u16> {
        if self.index + 4 > self.slice.len() {
            self.index = self.slice.len();
            return error(self, ErrorCode::EofWhileParsingString);
        }
        let mut n = 0;
        for _ in 0..4 {
            let ch = decode_hex_val(self.slice[self.index]);
            self.index += 1;
            match ch {
                None => return error(self, ErrorCode::InvalidEscape),
                Some(val) => {
                    n = (n << 4) + val;
                }
            }
        }
        Ok(n)
    }
    #[cfg(feature = "raw_value")]
    fn begin_raw_buffering(&mut self) {
        self.raw_buffering_start_index = self.index;
    }
    #[cfg(feature = "raw_value")]
    fn end_raw_buffering<V>(&mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'a>,
    {
        let raw = &self.slice[self.raw_buffering_start_index..self.index];
        let raw = match str::from_utf8(raw) {
            Ok(raw) => raw,
            Err(_) => return error(self, ErrorCode::InvalidUnicodeCodePoint),
        };
        visitor
            .visit_map(BorrowedRawDeserializer {
                raw_value: Some(raw),
            })
    }
    const should_early_return_if_failed: bool = false;
    #[inline]
    #[cold]
    fn set_failed(&mut self, _failed: &mut bool) {
        self.slice = &self.slice[..self.index];
    }
}
impl<'a> StrRead<'a> {
    /// Create a JSON input source to read from a UTF-8 string.
    pub fn new(s: &'a str) -> Self {
        StrRead {
            delegate: SliceRead::new(s.as_bytes()),
            #[cfg(feature = "raw_value")]
            data: s,
        }
    }
}
impl<'a> private::Sealed for StrRead<'a> {}
impl<'a> Read<'a> for StrRead<'a> {
    #[inline]
    fn next(&mut self) -> Result<Option<u8>> {
        self.delegate.next()
    }
    #[inline]
    fn peek(&mut self) -> Result<Option<u8>> {
        self.delegate.peek()
    }
    #[inline]
    fn discard(&mut self) {
        self.delegate.discard();
    }
    fn position(&self) -> Position {
        self.delegate.position()
    }
    fn peek_position(&self) -> Position {
        self.delegate.peek_position()
    }
    fn byte_offset(&self) -> usize {
        self.delegate.byte_offset()
    }
    fn parse_str<'s>(
        &'s mut self,
        scratch: &'s mut Vec<u8>,
    ) -> Result<Reference<'a, 's, str>> {
        self.delegate
            .parse_str_bytes(
                scratch,
                true,
                |_, bytes| { Ok(unsafe { str::from_utf8_unchecked(bytes) }) },
            )
    }
    fn parse_str_raw<'s>(
        &'s mut self,
        scratch: &'s mut Vec<u8>,
    ) -> Result<Reference<'a, 's, [u8]>> {
        self.delegate.parse_str_raw(scratch)
    }
    fn ignore_str(&mut self) -> Result<()> {
        self.delegate.ignore_str()
    }
    fn decode_hex_escape(&mut self) -> Result<u16> {
        self.delegate.decode_hex_escape()
    }
    #[cfg(feature = "raw_value")]
    fn begin_raw_buffering(&mut self) {
        self.delegate.begin_raw_buffering();
    }
    #[cfg(feature = "raw_value")]
    fn end_raw_buffering<V>(&mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'a>,
    {
        let raw = &self
            .data[self.delegate.raw_buffering_start_index..self.delegate.index];
        visitor
            .visit_map(BorrowedRawDeserializer {
                raw_value: Some(raw),
            })
    }
    const should_early_return_if_failed: bool = false;
    #[inline]
    #[cold]
    fn set_failed(&mut self, failed: &mut bool) {
        self.delegate.set_failed(failed);
    }
}
impl<'a, 'de, R> private::Sealed for &'a mut R
where
    R: Read<'de>,
{}
impl<'a, 'de, R> Read<'de> for &'a mut R
where
    R: Read<'de>,
{
    fn next(&mut self) -> Result<Option<u8>> {
        R::next(self)
    }
    fn peek(&mut self) -> Result<Option<u8>> {
        R::peek(self)
    }
    fn discard(&mut self) {
        R::discard(self);
    }
    fn position(&self) -> Position {
        R::position(self)
    }
    fn peek_position(&self) -> Position {
        R::peek_position(self)
    }
    fn byte_offset(&self) -> usize {
        R::byte_offset(self)
    }
    fn parse_str<'s>(
        &'s mut self,
        scratch: &'s mut Vec<u8>,
    ) -> Result<Reference<'de, 's, str>> {
        R::parse_str(self, scratch)
    }
    fn parse_str_raw<'s>(
        &'s mut self,
        scratch: &'s mut Vec<u8>,
    ) -> Result<Reference<'de, 's, [u8]>> {
        R::parse_str_raw(self, scratch)
    }
    fn ignore_str(&mut self) -> Result<()> {
        R::ignore_str(self)
    }
    fn decode_hex_escape(&mut self) -> Result<u16> {
        R::decode_hex_escape(self)
    }
    #[cfg(feature = "raw_value")]
    fn begin_raw_buffering(&mut self) {
        R::begin_raw_buffering(self);
    }
    #[cfg(feature = "raw_value")]
    fn end_raw_buffering<V>(&mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        R::end_raw_buffering(self, visitor)
    }
    const should_early_return_if_failed: bool = R::should_early_return_if_failed;
    fn set_failed(&mut self, failed: &mut bool) {
        R::set_failed(self, failed);
    }
}
/// Marker for whether StreamDeserializer can implement FusedIterator.
pub trait Fused: private::Sealed {}
impl<'a> Fused for SliceRead<'a> {}
impl<'a> Fused for StrRead<'a> {}
static ESCAPE: [bool; 256] = {
    const CT: bool = true;
    const QU: bool = true;
    const BS: bool = true;
    const __: bool = false;
    [
        CT,
        CT,
        CT,
        CT,
        CT,
        CT,
        CT,
        CT,
        CT,
        CT,
        CT,
        CT,
        CT,
        CT,
        CT,
        CT,
        CT,
        CT,
        CT,
        CT,
        CT,
        CT,
        CT,
        CT,
        CT,
        CT,
        CT,
        CT,
        CT,
        CT,
        CT,
        CT,
        __,
        __,
        QU,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        BS,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
    ]
};
fn next_or_eof<'de, R>(read: &mut R) -> Result<u8>
where
    R: ?Sized + Read<'de>,
{
    match tri!(read.next()) {
        Some(b) => Ok(b),
        None => error(read, ErrorCode::EofWhileParsingString),
    }
}
fn peek_or_eof<'de, R>(read: &mut R) -> Result<u8>
where
    R: ?Sized + Read<'de>,
{
    match tri!(read.peek()) {
        Some(b) => Ok(b),
        None => error(read, ErrorCode::EofWhileParsingString),
    }
}
fn error<'de, R, T>(read: &R, reason: ErrorCode) -> Result<T>
where
    R: ?Sized + Read<'de>,
{
    let position = read.position();
    Err(Error::syntax(reason, position.line, position.column))
}
fn as_str<'de, 's, R: Read<'de>>(read: &R, slice: &'s [u8]) -> Result<&'s str> {
    str::from_utf8(slice).or_else(|_| error(read, ErrorCode::InvalidUnicodeCodePoint))
}
/// Parses a JSON escape sequence and appends it into the scratch space. Assumes
/// the previous byte read was a backslash.
fn parse_escape<'de, R: Read<'de>>(
    read: &mut R,
    validate: bool,
    scratch: &mut Vec<u8>,
) -> Result<()> {
    let ch = tri!(next_or_eof(read));
    match ch {
        b'"' => scratch.push(b'"'),
        b'\\' => scratch.push(b'\\'),
        b'/' => scratch.push(b'/'),
        b'b' => scratch.push(b'\x08'),
        b'f' => scratch.push(b'\x0c'),
        b'n' => scratch.push(b'\n'),
        b'r' => scratch.push(b'\r'),
        b't' => scratch.push(b'\t'),
        b'u' => {
            fn encode_surrogate(scratch: &mut Vec<u8>, n: u16) {
                scratch
                    .extend_from_slice(
                        &[
                            (n >> 12 & 0b0000_1111) as u8 | 0b1110_0000,
                            (n >> 6 & 0b0011_1111) as u8 | 0b1000_0000,
                            (n & 0b0011_1111) as u8 | 0b1000_0000,
                        ],
                    );
            }
            let c = match tri!(read.decode_hex_escape()) {
                n @ 0xDC00..=0xDFFF => {
                    return if validate {
                        error(read, ErrorCode::LoneLeadingSurrogateInHexEscape)
                    } else {
                        encode_surrogate(scratch, n);
                        Ok(())
                    };
                }
                n1 @ 0xD800..=0xDBFF => {
                    if tri!(peek_or_eof(read)) == b'\\' {
                        read.discard();
                    } else {
                        return if validate {
                            read.discard();
                            error(read, ErrorCode::UnexpectedEndOfHexEscape)
                        } else {
                            encode_surrogate(scratch, n1);
                            Ok(())
                        };
                    }
                    if tri!(peek_or_eof(read)) == b'u' {
                        read.discard();
                    } else {
                        return if validate {
                            read.discard();
                            error(read, ErrorCode::UnexpectedEndOfHexEscape)
                        } else {
                            encode_surrogate(scratch, n1);
                            parse_escape(read, validate, scratch)
                        };
                    }
                    let n2 = tri!(read.decode_hex_escape());
                    if n2 < 0xDC00 || n2 > 0xDFFF {
                        return error(read, ErrorCode::LoneLeadingSurrogateInHexEscape);
                    }
                    let n = (((n1 - 0xD800) as u32) << 10 | (n2 - 0xDC00) as u32)
                        + 0x1_0000;
                    match char::from_u32(n) {
                        Some(c) => c,
                        None => {
                            return error(read, ErrorCode::InvalidUnicodeCodePoint);
                        }
                    }
                }
                n => char::from_u32(n as u32).unwrap(),
            };
            scratch.extend_from_slice(c.encode_utf8(&mut [0_u8; 4]).as_bytes());
        }
        _ => {
            return error(read, ErrorCode::InvalidEscape);
        }
    }
    Ok(())
}
/// Parses a JSON escape sequence and discards the value. Assumes the previous
/// byte read was a backslash.
fn ignore_escape<'de, R>(read: &mut R) -> Result<()>
where
    R: ?Sized + Read<'de>,
{
    let ch = tri!(next_or_eof(read));
    match ch {
        b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't' => {}
        b'u' => {
            tri!(read.decode_hex_escape());
        }
        _ => {
            return error(read, ErrorCode::InvalidEscape);
        }
    }
    Ok(())
}
static HEX: [u8; 256] = {
    const __: u8 = 255;
    [
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        00,
        01,
        02,
        03,
        04,
        05,
        06,
        07,
        08,
        09,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        10,
        11,
        12,
        13,
        14,
        15,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        10,
        11,
        12,
        13,
        14,
        15,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
        __,
    ]
};
fn decode_hex_val(val: u8) -> Option<u16> {
    let n = HEX[val as usize] as u16;
    if n == 255 { None } else { Some(n) }
}
#[cfg(test)]
mod tests_rug_570 {
    use super::*;
    use crate::de::SliceRead;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_570_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"your_slice_here";
        let mut v1: SliceRead = SliceRead::new(rug_fuzz_0);
        crate::read::next_or_eof(&mut v1).unwrap();
        let _rug_ed_tests_rug_570_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_571 {
    use super::*;
    use crate::de::{Read, StrRead};
    #[test]
    fn test_peek_or_eof() {
        let _rug_st_tests_rug_571_rrrruuuugggg_test_peek_or_eof = 0;
        let rug_fuzz_0 = "sample string";
        let s = rug_fuzz_0;
        let mut p0 = StrRead::new(s);
        let _ = peek_or_eof(&mut p0);
        let _rug_ed_tests_rug_571_rrrruuuugggg_test_peek_or_eof = 0;
    }
}
#[cfg(test)]
mod tests_rug_573 {
    use super::*;
    use std::str;
    use crate::de;
    #[test]
    fn test_as_str() {
        let _rug_st_tests_rug_573_rrrruuuugggg_test_as_str = 0;
        let rug_fuzz_0 = "sample string";
        let rug_fuzz_1 = b"sample data";
        let s = rug_fuzz_0;
        let mut v2 = de::StrRead::new(s);
        let p0: &mut de::StrRead = &mut v2;
        let p1: &[u8] = rug_fuzz_1;
        crate::read::as_str(p0, p1).unwrap();
        let _rug_ed_tests_rug_573_rrrruuuugggg_test_as_str = 0;
    }
}
#[cfg(test)]
mod tests_rug_574 {
    use super::*;
    use crate::de::SliceRead;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_574_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"your_slice_here";
        let rug_fuzz_1 = true;
        let rug_fuzz_2 = 1;
        let mut p0: SliceRead = SliceRead::new(rug_fuzz_0);
        let p1: bool = rug_fuzz_1;
        let mut p2: std::vec::Vec<u8> = vec![rug_fuzz_2, 2, 3, 4, 5];
        crate::read::parse_escape(&mut p0, p1, &mut p2);
        let _rug_ed_tests_rug_574_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_576 {
    use super::*;
    use crate::read::StrRead;
    #[test]
    fn test_ignore_escape() {
        let _rug_st_tests_rug_576_rrrruuuugggg_test_ignore_escape = 0;
        let rug_fuzz_0 = "sample string";
        let mut p0: StrRead = StrRead::new(rug_fuzz_0);
        crate::read::ignore_escape(&mut p0).unwrap();
        let _rug_ed_tests_rug_576_rrrruuuugggg_test_ignore_escape = 0;
    }
}
#[cfg(test)]
mod tests_rug_577 {
    use super::*;
    use crate::read::HEX;
    #[test]
    fn test_decode_hex_val() {
        let _rug_st_tests_rug_577_rrrruuuugggg_test_decode_hex_val = 0;
        let rug_fuzz_0 = 10;
        let p0: u8 = rug_fuzz_0;
        debug_assert_eq!(decode_hex_val(p0), Some(HEX[10 as usize] as u16));
        let _rug_ed_tests_rug_577_rrrruuuugggg_test_decode_hex_val = 0;
    }
}
#[cfg(test)]
mod tests_rug_579 {
    use super::*;
    use std::io::{self, BufRead, Stdin, StdinLock};
    use crate::read;
    use crate::read::IoRead;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_579_rrrruuuugggg_test_rug = 0;
        let stdin: Stdin = io::stdin();
        let stdin_lock: StdinLock<'_> = stdin.lock();
        let p0: StdinLock<'_> = stdin_lock;
        <IoRead<StdinLock<'_>>>::new(p0);
        let _rug_ed_tests_rug_579_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_592 {
    use super::*;
    use crate::read::SliceRead;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_592_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"Hello, world!";
        let p0: &[u8] = rug_fuzz_0;
        SliceRead::new(p0);
        let _rug_ed_tests_rug_592_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_593 {
    use super::*;
    use crate::read::SliceRead;
    #[test]
    fn test_position_of_index() {
        let _rug_st_tests_rug_593_rrrruuuugggg_test_position_of_index = 0;
        let rug_fuzz_0 = b"Hello, World!";
        let rug_fuzz_1 = 5;
        let p0: SliceRead<'static> = SliceRead::new(rug_fuzz_0.as_ref());
        let p1: usize = rug_fuzz_1;
        <SliceRead>::position_of_index(&p0, p1);
        let _rug_ed_tests_rug_593_rrrruuuugggg_test_position_of_index = 0;
    }
}
#[cfg(test)]
mod tests_rug_595 {
    use super::*;
    use crate::de::Read;
    use crate::read::SliceRead;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_595_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"Hello, World!";
        let mut v91: SliceRead<'static> = SliceRead::new(rug_fuzz_0.as_ref());
        <SliceRead<'static> as Read<'static>>::next(&mut v91);
        let _rug_ed_tests_rug_595_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_596 {
    use super::*;
    use crate::de::Read;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_596_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"Hello, World!";
        let mut p0: SliceRead<'static> = SliceRead::new(rug_fuzz_0.as_ref());
        <SliceRead<'static> as Read<'static>>::peek(&mut p0).unwrap();
        let _rug_ed_tests_rug_596_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_597 {
    use super::*;
    use crate::de::Read;
    use crate::read::SliceRead;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_597_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"Hello, World!";
        let mut p0: SliceRead<'static> = SliceRead::new(rug_fuzz_0.as_ref());
        p0.discard();
        let _rug_ed_tests_rug_597_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_598 {
    use super::*;
    use crate::de::Read;
    use crate::de::SliceRead;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_598_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"Hello, World!";
        let mut p0: SliceRead<'static> = SliceRead::new(rug_fuzz_0.as_ref());
        <SliceRead<'static> as Read<'static>>::position(&p0);
        let _rug_ed_tests_rug_598_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_599 {
    use super::*;
    use crate::de::Read;
    use crate::read::SliceRead;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_599_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"Hello, World!";
        let slice_read: SliceRead<'static> = SliceRead::new(rug_fuzz_0.as_ref());
        <SliceRead<'static> as Read<'static>>::peek_position(&slice_read);
        let _rug_ed_tests_rug_599_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_600 {
    use super::*;
    use crate::de::Read;
    use crate::read::SliceRead;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_600_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"Hello, World!";
        let mut p0: SliceRead<'static> = SliceRead::new(rug_fuzz_0.as_ref());
        <SliceRead<'static> as Read>::byte_offset(&p0);
        let _rug_ed_tests_rug_600_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_601 {
    use super::*;
    use crate::de::Read;
    use crate::read::SliceRead;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_601_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"Hello, World!";
        let rug_fuzz_1 = 1;
        let mut v91: SliceRead<'static> = SliceRead::new(rug_fuzz_0.as_ref());
        let mut v86: Vec<u8> = vec![rug_fuzz_1, 2, 3, 4, 5];
        <SliceRead<'static> as Read<'static>>::parse_str(&mut v91, &mut v86);
        let _rug_ed_tests_rug_601_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_602 {
    use super::*;
    use crate::read::{SliceRead, Read};
    #[test]
    fn test_parse_str_raw() {
        let _rug_st_tests_rug_602_rrrruuuugggg_test_parse_str_raw = 0;
        let rug_fuzz_0 = b"Hello, World!";
        let mut p0: SliceRead<'static> = SliceRead::new(rug_fuzz_0.as_ref());
        let mut p1: Vec<u8> = Vec::new();
        <SliceRead<'static> as Read<'static>>::parse_str_raw(&mut p0, &mut p1).unwrap();
        let _rug_ed_tests_rug_602_rrrruuuugggg_test_parse_str_raw = 0;
    }
}
#[cfg(test)]
mod tests_rug_603 {
    use super::*;
    use crate::de::Read;
    use crate::read::SliceRead;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_603_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"Hello, World!";
        let slice = rug_fuzz_0.as_ref();
        let mut p0: SliceRead<'static> = SliceRead::new(slice);
        <SliceRead<'static> as Read<'static>>::ignore_str(&mut p0).unwrap();
        let _rug_ed_tests_rug_603_rrrruuuugggg_test_rug = 0;
    }
}
#[test]
fn test_rug() {
    let mut v91: SliceRead<'static> = SliceRead::new(b"Hello, World!".as_ref());
    v91.decode_hex_escape();
}
#[cfg(test)]
mod tests_rug_605 {
    use super::*;
    use crate::read::SliceRead;
    use crate::de::Read;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_605_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"Hello, World!";
        let rug_fuzz_1 = false;
        let mut p0: SliceRead<'static> = SliceRead::new(rug_fuzz_0.as_ref());
        let mut p1: bool = rug_fuzz_1;
        p0.set_failed(&mut p1);
        let _rug_ed_tests_rug_605_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_606 {
    use super::*;
    use crate::read::StrRead;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_606_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "Hello, world!";
        let p0 = rug_fuzz_0;
        StrRead::new(&p0);
        let _rug_ed_tests_rug_606_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_607 {
    use super::*;
    use crate::de::Read;
    use crate::read::StrRead;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_607_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample data";
        let mut p0: StrRead<'static> = StrRead::new(rug_fuzz_0);
        <StrRead<'static> as Read<'static>>::next(&mut p0);
        let _rug_ed_tests_rug_607_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_608 {
    use super::*;
    use crate::de::Read;
    use crate::read::StrRead;
    #[test]
    fn test_peek() {
        let _rug_st_tests_rug_608_rrrruuuugggg_test_peek = 0;
        let rug_fuzz_0 = "sample data";
        let mut p0: StrRead<'static> = StrRead::new(rug_fuzz_0);
        p0.peek();
        let _rug_ed_tests_rug_608_rrrruuuugggg_test_peek = 0;
    }
}
#[cfg(test)]
mod tests_rug_610 {
    use super::*;
    use crate::read::StrRead;
    use crate::read::Read;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_610_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample data";
        let mut p0: StrRead<'static> = StrRead::new(rug_fuzz_0);
        <StrRead<'static> as Read<'static>>::position(&p0);
        let _rug_ed_tests_rug_610_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_611 {
    use super::*;
    use crate::de::Read;
    use crate::read::{StrRead, Position};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_611_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample data";
        let mut p0: StrRead<'static> = StrRead::new(rug_fuzz_0);
        <StrRead<'static> as Read<'static>>::peek_position(&p0);
        let _rug_ed_tests_rug_611_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_612 {
    use super::*;
    use crate::read::Read;
    use crate::read::StrRead;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_612_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample data";
        let mut p0: StrRead<'static> = StrRead::new(rug_fuzz_0);
        <StrRead<'static> as Read<'static>>::byte_offset(&p0);
        let _rug_ed_tests_rug_612_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_613 {
    use super::*;
    use crate::read::{Read, StrRead};
    #[test]
    fn test_parse_str() {
        let _rug_st_tests_rug_613_rrrruuuugggg_test_parse_str = 0;
        let rug_fuzz_0 = "sample data";
        let rug_fuzz_1 = 1;
        let mut p0: StrRead<'static> = StrRead::new(rug_fuzz_0);
        let mut p1: Vec<u8> = vec![rug_fuzz_1, 2, 3, 4, 5];
        p0.parse_str(&mut p1).unwrap();
        let _rug_ed_tests_rug_613_rrrruuuugggg_test_parse_str = 0;
    }
}
#[cfg(test)]
mod tests_rug_614 {
    use super::*;
    use crate::de::Read;
    use crate::read::{StrRead, Reference};
    #[test]
    fn test_parse_str_raw() {
        let _rug_st_tests_rug_614_rrrruuuugggg_test_parse_str_raw = 0;
        let rug_fuzz_0 = "sample data";
        let rug_fuzz_1 = 1;
        let mut p0: StrRead<'static> = StrRead::new(rug_fuzz_0);
        let mut p1: Vec<u8> = vec![rug_fuzz_1, 2, 3, 4, 5];
        p0.parse_str_raw(&mut p1).unwrap();
        let _rug_ed_tests_rug_614_rrrruuuugggg_test_parse_str_raw = 0;
    }
}
#[cfg(test)]
mod tests_rug_615 {
    use super::*;
    use crate::read::{Read, StrRead};
    #[test]
    fn test_ignore_str() {
        let _rug_st_tests_rug_615_rrrruuuugggg_test_ignore_str = 0;
        let rug_fuzz_0 = "sample data";
        let mut p0: StrRead<'static> = StrRead::new(rug_fuzz_0);
        <StrRead<'static> as Read<'static>>::ignore_str(&mut p0).unwrap();
        let _rug_ed_tests_rug_615_rrrruuuugggg_test_ignore_str = 0;
    }
}
#[cfg(test)]
mod tests_rug_616 {
    use super::*;
    use crate::de::Read;
    use crate::read::StrRead;
    #[test]
    fn test_decode_hex_escape() {
        let _rug_st_tests_rug_616_rrrruuuugggg_test_decode_hex_escape = 0;
        let rug_fuzz_0 = "sample data";
        let mut p0: StrRead<'static> = StrRead::new(rug_fuzz_0);
        p0.decode_hex_escape();
        let _rug_ed_tests_rug_616_rrrruuuugggg_test_decode_hex_escape = 0;
    }
}
#[cfg(test)]
mod tests_rug_617 {
    use super::*;
    use crate::de::Read;
    use crate::read::StrRead;
    #[test]
    fn test_set_failed() {
        let _rug_st_tests_rug_617_rrrruuuugggg_test_set_failed = 0;
        let rug_fuzz_0 = "sample data";
        let rug_fuzz_1 = false;
        let mut p0: StrRead<'static> = StrRead::new(rug_fuzz_0);
        let mut p1: bool = rug_fuzz_1;
        p0.set_failed(&mut p1);
        let _rug_ed_tests_rug_617_rrrruuuugggg_test_set_failed = 0;
    }
}
#[cfg(test)]
mod tests_rug_618 {
    use super::*;
    use crate::de::Read;
    use crate::de;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_618_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample string";
        let s = rug_fuzz_0;
        let mut v2 = de::StrRead::new(s);
        let mut p0 = &mut v2;
        p0.next();
        let _rug_ed_tests_rug_618_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_619 {
    use super::*;
    use crate::de::StrRead;
    use crate::de::Read;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_619_rrrruuuugggg_sample = 0;
        let rug_fuzz_0 = "sample string";
        #[cfg(test)]
        mod tests_rug_619_prepare {
            use crate::de;
            #[test]
            fn sample() {
                let _rug_st_tests_rug_619_prepare_rrrruuuugggg_sample = 0;
                let rug_fuzz_0 = 0;
                let rug_fuzz_1 = "sample string";
                let rug_fuzz_2 = 0;
                let _rug_st_tests_rug_619_rrrruuuugggg_sample = rug_fuzz_0;
                let rug_fuzz_0 = rug_fuzz_1;
                let s = rug_fuzz_0;
                let mut v2 = de::StrRead::new(s);
                let _rug_ed_tests_rug_619_rrrruuuugggg_sample = rug_fuzz_2;
                let _rug_ed_tests_rug_619_prepare_rrrruuuugggg_sample = 0;
            }
        }
        let s = "sample string";
        let mut v2 = StrRead::new(s);
        v2.peek();
        let _rug_ed_tests_rug_619_rrrruuuugggg_sample = 0;
    }
}
#[cfg(test)]
mod tests_rug_620 {
    use super::*;
    use crate::de::Read;
    use crate::de;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_620_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample string";
        let s = rug_fuzz_0;
        let mut p0 = de::StrRead::new(s);
        p0.discard();
        let _rug_ed_tests_rug_620_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_621 {
    use super::*;
    use crate::de::Read;
    use crate::de::SliceRead;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_621_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"your_slice_here";
        let mut p0: SliceRead = SliceRead::new(rug_fuzz_0);
        p0.position();
        let _rug_ed_tests_rug_621_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_622 {
    use super::*;
    use crate::de::Read;
    use crate::de;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_622_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample string";
        let mut p0 = de::StrRead::new(rug_fuzz_0);
        p0.peek_position();
        let _rug_ed_tests_rug_622_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_623 {
    use super::*;
    use crate::de::Read;
    use crate::de::SliceRead;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_623_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"your_slice_here";
        let mut v1: SliceRead = SliceRead::new(rug_fuzz_0);
        v1.byte_offset();
        let _rug_ed_tests_rug_623_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_624 {
    use super::*;
    use crate::de::Read;
    use crate::de::SliceRead;
    use std::vec::Vec;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_624_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"your_slice_here";
        let rug_fuzz_1 = 1;
        let mut p0: SliceRead = SliceRead::new(rug_fuzz_0);
        let mut p1: Vec<u8> = vec![rug_fuzz_1, 2, 3, 4, 5];
        p0.parse_str(&mut p1);
        let _rug_ed_tests_rug_624_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_625 {
    use super::*;
    use crate::de::Read;
    use crate::de::SliceRead;
    #[test]
    fn test_parse_str_raw() {
        let _rug_st_tests_rug_625_rrrruuuugggg_test_parse_str_raw = 0;
        let rug_fuzz_0 = b"your_slice_here";
        let rug_fuzz_1 = 1;
        let mut p0: SliceRead = SliceRead::new(rug_fuzz_0);
        let mut p1: Vec<u8> = vec![rug_fuzz_1, 2, 3, 4, 5];
        p0.parse_str_raw(&mut p1).unwrap();
        let _rug_ed_tests_rug_625_rrrruuuugggg_test_parse_str_raw = 0;
    }
}
#[cfg(test)]
mod tests_rug_626 {
    use super::*;
    use crate::de::Read;
    use crate::de;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_626_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample string";
        let s = rug_fuzz_0;
        let mut p0 = de::StrRead::new(s);
        p0.ignore_str();
        let _rug_ed_tests_rug_626_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_627 {
    use super::*;
    use crate::de::Read;
    use crate::de::SliceRead;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_627_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"your_slice_here";
        let mut v1: SliceRead = SliceRead::new(rug_fuzz_0);
        v1.decode_hex_escape();
        let _rug_ed_tests_rug_627_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_628 {
    use super::*;
    use crate::de::Read;
    use crate::de::SliceRead;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_628_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"your_slice_here";
        let rug_fuzz_1 = true;
        let mut p0: SliceRead = SliceRead::new(rug_fuzz_0);
        let mut p1: bool = rug_fuzz_1;
        p0.set_failed(&mut p1);
        let _rug_ed_tests_rug_628_rrrruuuugggg_test_rug = 0;
    }
}
