use crate::error::{Error, ErrorCode, Result};
use crate::lib::ops::Deref;
use crate::lib::*;
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
                    tri!(parse_escape(self, scratch));
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
                    if let Some(ref mut buf) = self.raw_buffer {
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
                            if let Some(ref mut buf) = self.raw_buffer {
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
            if let Some(ref mut buf) = self.raw_buffer {
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
        let raw = String::from_utf8(raw).unwrap();
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
                    tri!(parse_escape(self, scratch));
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
        let raw = str::from_utf8(raw).unwrap();
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
        self.delegate.begin_raw_buffering()
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
        R::discard(self)
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
        R::begin_raw_buffering(self)
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
        R::set_failed(self, failed)
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
fn parse_escape<'de, R: Read<'de>>(read: &mut R, scratch: &mut Vec<u8>) -> Result<()> {
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
            let c = match tri!(read.decode_hex_escape()) {
                0xDC00..=0xDFFF => {
                    return error(read, ErrorCode::LoneLeadingSurrogateInHexEscape);
                }
                n1 @ 0xD800..=0xDBFF => {
                    if tri!(next_or_eof(read)) != b'\\' {
                        return error(read, ErrorCode::UnexpectedEndOfHexEscape);
                    }
                    if tri!(next_or_eof(read)) != b'u' {
                        return error(read, ErrorCode::UnexpectedEndOfHexEscape);
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
                n => {
                    match char::from_u32(n as u32) {
                        Some(c) => c,
                        None => {
                            return error(read, ErrorCode::InvalidUnicodeCodePoint);
                        }
                    }
                }
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
            let n = match tri!(read.decode_hex_escape()) {
                0xDC00..=0xDFFF => {
                    return error(read, ErrorCode::LoneLeadingSurrogateInHexEscape);
                }
                n1 @ 0xD800..=0xDBFF => {
                    if tri!(next_or_eof(read)) != b'\\' {
                        return error(read, ErrorCode::UnexpectedEndOfHexEscape);
                    }
                    if tri!(next_or_eof(read)) != b'u' {
                        return error(read, ErrorCode::UnexpectedEndOfHexEscape);
                    }
                    let n2 = tri!(read.decode_hex_escape());
                    if n2 < 0xDC00 || n2 > 0xDFFF {
                        return error(read, ErrorCode::LoneLeadingSurrogateInHexEscape);
                    }
                    (((n1 - 0xD800) as u32) << 10 | (n2 - 0xDC00) as u32) + 0x1_0000
                }
                n => n as u32,
            };
            if char::from_u32(n).is_none() {
                return error(read, ErrorCode::InvalidUnicodeCodePoint);
            }
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
use crate::read;
#[cfg(test)]
mod tests_llm_16_6 {
    use super::*;
    use crate::*;
    #[test]
    fn test_discard() {
        let _rug_st_tests_llm_16_6_rrrruuuugggg_test_discard = 0;
        let rug_fuzz_0 = b"";
        let mut input: &[u8] = rug_fuzz_0;
        let mut reader = read::IoRead::new(&mut input);
        reader.discard();
        let _rug_ed_tests_llm_16_6_rrrruuuugggg_test_discard = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_7 {
    use crate::read::Read;
    use crate::read::Result;
    #[test]
    fn test_ignore_str() -> Result<()> {
        Ok(())
    }
}
#[cfg(test)]
mod tests_llm_16_16 {
    use crate::read::Position;
    #[test]
    fn test_peek_position() {
        let _rug_st_tests_llm_16_16_rrrruuuugggg_test_peek_position = 0;
        let _rug_ed_tests_llm_16_16_rrrruuuugggg_test_peek_position = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_361_llm_16_360 {
    use super::*;
    use crate::*;
    use crate::read::{Read, IoRead};
    #[test]
    fn test_discard() {
        let _rug_st_tests_llm_16_361_llm_16_360_rrrruuuugggg_test_discard = 0;
        let rug_fuzz_0 = b"test";
        let mut reader: IoRead<std::io::Cursor<&[u8]>> = IoRead::new(
            std::io::Cursor::new(rug_fuzz_0),
        );
        reader.discard();
        debug_assert_eq!(reader.peek().unwrap(), None);
        let _rug_ed_tests_llm_16_361_llm_16_360_rrrruuuugggg_test_discard = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_373_llm_16_372 {
    use crate::read::Position;
    use crate::read::IoRead;
    use crate::read::Read;
    use std::io::Read as StdRead;
    #[test]
    fn test_peek_position() {
        let _rug_st_tests_llm_16_373_llm_16_372_rrrruuuugggg_test_peek_position = 0;
        let rug_fuzz_0 = b"Hello World";
        let mut reader: IoRead<&[u8]> = IoRead::new(rug_fuzz_0);
        let position: Position = <IoRead<&[u8]> as Read>::peek_position(&mut reader);
        debug_assert_eq!(position.line, 1);
        debug_assert_eq!(position.column, 0);
        let _rug_ed_tests_llm_16_373_llm_16_372_rrrruuuugggg_test_peek_position = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_376 {
    use crate::read::IoRead;
    use crate::read::Read;
    use std::io;
    #[test]
    fn test_set_failed() {
        let _rug_st_tests_llm_16_376_rrrruuuugggg_test_set_failed = 0;
        let rug_fuzz_0 = false;
        let rug_fuzz_1 = true;
        let mut failed = rug_fuzz_0;
        let mut reader: IoRead<io::Empty> = IoRead::new(io::empty());
        reader.set_failed(&mut failed);
        debug_assert_eq!(rug_fuzz_1, failed);
        let _rug_ed_tests_llm_16_376_rrrruuuugggg_test_set_failed = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_377 {
    use crate::read::Reference;
    #[test]
    fn test_deref_borrowed() {
        let _rug_st_tests_llm_16_377_rrrruuuugggg_test_deref_borrowed = 0;
        let rug_fuzz_0 = 10;
        let value = rug_fuzz_0;
        let reference = Reference::Borrowed(&value);
        debug_assert_eq!(* reference, value);
        let _rug_ed_tests_llm_16_377_rrrruuuugggg_test_deref_borrowed = 0;
    }
    #[test]
    fn test_deref_copied() {
        let _rug_st_tests_llm_16_377_rrrruuuugggg_test_deref_copied = 0;
        let rug_fuzz_0 = 20;
        let value = rug_fuzz_0;
        let reference = Reference::Copied(&value);
        debug_assert_eq!(* reference, value);
        let _rug_ed_tests_llm_16_377_rrrruuuugggg_test_deref_copied = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_378 {
    use super::*;
    use crate::*;
    #[test]
    fn test_byte_offset() {
        let _rug_st_tests_llm_16_378_rrrruuuugggg_test_byte_offset = 0;
        let rug_fuzz_0 = b'a';
        let rug_fuzz_1 = b'b';
        let rug_fuzz_2 = b'c';
        let rug_fuzz_3 = b'd';
        let rug_fuzz_4 = b'e';
        let slice = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3, rug_fuzz_4];
        let reader = SliceRead::new(&slice);
        debug_assert_eq!(reader.byte_offset(), 0);
        let _rug_ed_tests_llm_16_378_rrrruuuugggg_test_byte_offset = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_381 {
    use super::*;
    use crate::*;
    #[test]
    fn test_discard() {
        let _rug_st_tests_llm_16_381_rrrruuuugggg_test_discard = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let mut slice = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        let mut slice_read = SliceRead::new(&mut slice);
        slice_read.discard();
        debug_assert_eq!(slice_read.index, 1);
        let _rug_ed_tests_llm_16_381_rrrruuuugggg_test_discard = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_390 {
    use crate::read::Read;
    use crate::read::SliceRead;
    #[test]
    fn test_peek() {
        let _rug_st_tests_llm_16_390_rrrruuuugggg_test_peek = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 4;
        let input = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3];
        let mut reader = SliceRead::new(input);
        debug_assert_eq!(reader.peek().unwrap(), Some(1));
        debug_assert_eq!(reader.peek().unwrap(), Some(1));
        debug_assert_eq!(reader.next().unwrap(), Some(1));
        debug_assert_eq!(reader.peek().unwrap(), Some(2));
        debug_assert_eq!(reader.next().unwrap(), Some(2));
        debug_assert_eq!(reader.peek().unwrap(), Some(3));
        debug_assert_eq!(reader.next().unwrap(), Some(3));
        debug_assert_eq!(reader.peek().unwrap(), Some(4));
        debug_assert_eq!(reader.next().unwrap(), Some(4));
        debug_assert_eq!(reader.peek().unwrap(), None);
        debug_assert_eq!(reader.next().unwrap(), None);
        let _rug_ed_tests_llm_16_390_rrrruuuugggg_test_peek = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_395 {
    use super::*;
    use crate::*;
    use crate::read::{Position, read::Read};
    #[test]
    fn test_set_failed() {
        let _rug_st_tests_llm_16_395_rrrruuuugggg_test_set_failed = 0;
        let rug_fuzz_0 = false;
        let rug_fuzz_1 = b"test";
        let mut failed = rug_fuzz_0;
        let mut slice_read = SliceRead::new(rug_fuzz_1);
        slice_read.set_failed(&mut failed);
        debug_assert_eq!(slice_read.slice, b"");
        let _rug_ed_tests_llm_16_395_rrrruuuugggg_test_set_failed = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_396 {
    use super::*;
    use crate::*;
    use crate::read::Read;
    #[test]
    fn test_byte_offset() {
        let _rug_st_tests_llm_16_396_rrrruuuugggg_test_byte_offset = 0;
        let rug_fuzz_0 = b'{';
        let rug_fuzz_1 = b'"';
        let rug_fuzz_2 = b'n';
        let rug_fuzz_3 = b'a';
        let rug_fuzz_4 = b'm';
        let rug_fuzz_5 = b'e';
        let rug_fuzz_6 = b'"';
        let rug_fuzz_7 = b':';
        let rug_fuzz_8 = b'"';
        let rug_fuzz_9 = b'J';
        let rug_fuzz_10 = b'o';
        let rug_fuzz_11 = b'h';
        let rug_fuzz_12 = b'n';
        let rug_fuzz_13 = b'!';
        let rug_fuzz_14 = b'"';
        let rug_fuzz_15 = b'}';
        let slice = [
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
        let mut reader = SliceRead::new(&slice);
        debug_assert_eq!(reader.byte_offset(), 0);
        reader.next().unwrap().unwrap();
        reader.next().unwrap().unwrap();
        debug_assert_eq!(reader.byte_offset(), 2);
        reader.discard();
        debug_assert_eq!(reader.byte_offset(), 3);
        reader.discard();
        reader.discard();
        reader.discard();
        reader.discard();
        debug_assert_eq!(reader.byte_offset(), 7);
        let _rug_ed_tests_llm_16_396_rrrruuuugggg_test_byte_offset = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_398_llm_16_397 {
    use crate::read::SliceRead;
    use crate::read::Read;
    #[test]
    fn test_decode_hex_escape() {
        let _rug_st_tests_llm_16_398_llm_16_397_rrrruuuugggg_test_decode_hex_escape = 0;
        let rug_fuzz_0 = b"abcd";
        let mut reader = SliceRead::new(rug_fuzz_0);
        let result = reader.decode_hex_escape();
        debug_assert_eq!(result.unwrap(), 43981);
        let _rug_ed_tests_llm_16_398_llm_16_397_rrrruuuugggg_test_decode_hex_escape = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_399 {
    use crate::read::{Read, SliceRead, StrRead};
    #[test]
    fn test_discard_slice_read() {
        let _rug_st_tests_llm_16_399_rrrruuuugggg_test_discard_slice_read = 0;
        let rug_fuzz_0 = b"abc";
        let mut slice_read = SliceRead::new(rug_fuzz_0);
        slice_read.discard();
        debug_assert_eq!(slice_read.index, 1);
        let _rug_ed_tests_llm_16_399_rrrruuuugggg_test_discard_slice_read = 0;
    }
    #[test]
    fn test_discard_str_read() {
        let _rug_st_tests_llm_16_399_rrrruuuugggg_test_discard_str_read = 0;
        let rug_fuzz_0 = "abc";
        let mut str_read = StrRead::new(rug_fuzz_0);
        str_read.discard();
        debug_assert_eq!(str_read.delegate.index, 1);
        let _rug_ed_tests_llm_16_399_rrrruuuugggg_test_discard_str_read = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_400 {
    use super::*;
    use crate::*;
    #[test]
    fn test_ignore_str() {
        let _rug_st_tests_llm_16_400_rrrruuuugggg_test_ignore_str = 0;
        let rug_fuzz_0 = r#"{"key1":"value1", "key2":"value2", "key3":"value3"}"#;
        let rug_fuzz_1 = r#"{"key1\":\"value1\", "key2":"value2\", "key3\":\"value3"}"#;
        let rug_fuzz_2 = r#"{"key1":"value1", "key2":"\", "key3":"value3"}"#;
        let mut reader = StrRead::new(rug_fuzz_0);
        let result = reader.ignore_str();
        debug_assert_eq!(result.is_ok(), true);
        let mut reader = StrRead::new(rug_fuzz_1);
        let result = reader.ignore_str();
        debug_assert_eq!(result.is_err(), true);
        let mut reader = StrRead::new(rug_fuzz_2);
        let result = reader.ignore_str();
        debug_assert_eq!(result.is_err(), true);
        let _rug_ed_tests_llm_16_400_rrrruuuugggg_test_ignore_str = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_401 {
    use super::*;
    use crate::*;
    use crate::error::Result;
    use crate::read::Read;
    #[test]
    fn test_next() -> Result<()> {
        let mut reader = StrRead::new("Hello, World!");
        let result = reader.next()?;
        assert_eq!(result, Some(b'H'));
        Ok(())
    }
}
#[cfg(test)]
mod tests_llm_16_406 {
    use crate::read::Read;
    use crate::read::SliceRead;
    use crate::read::StrRead;
    use crate::error::Result;
    #[test]
    fn test_peek() -> Result<()> {
        let json = r#""Hello, World!""#;
        let mut reader = StrRead::new(json);
        let peeked = reader.peek()?;
        assert_eq!(peeked, Some(b'H'));
        Ok(())
    }
}
#[cfg(test)]
mod tests_llm_16_409 {
    use super::*;
    use crate::*;
    use serde::de::IntoDeserializer;
    #[test]
    fn test_position() {
        let _rug_st_tests_llm_16_409_rrrruuuugggg_test_position = 0;
        let rug_fuzz_0 = r#"{"name":"John","age":30,"city":"New York"}"#;
        let input = rug_fuzz_0;
        let mut reader = StrRead::new(input);
        let position = reader.position();
        debug_assert_eq!(position.line, 1);
        debug_assert_eq!(position.column, 0);
        let _rug_ed_tests_llm_16_409_rrrruuuugggg_test_position = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_944 {
    use super::*;
    use crate::*;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_944_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let slice = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        let reader = SliceRead::new(slice);
        debug_assert_eq!(reader.slice.len(), slice.len());
        debug_assert_eq!(reader.index, 0);
        let _rug_ed_tests_llm_16_944_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_951 {
    use crate::read::as_str;
    use crate::read::{IoRead, Read};
    #[test]
    fn test_as_str() {
        let _rug_st_tests_llm_16_951_rrrruuuugggg_test_as_str = 0;
        let rug_fuzz_0 = b"hello";
        let rug_fuzz_1 = 0xC3;
        let rug_fuzz_2 = 0x28;
        let input: &[u8] = rug_fuzz_0;
        let read = IoRead::new(input);
        let result = as_str(&read, input);
        debug_assert!(result.is_ok());
        debug_assert_eq!(result.unwrap(), "hello");
        let input: &[u8] = &[rug_fuzz_1, rug_fuzz_2];
        let read = IoRead::new(input);
        let result = as_str(&read, input);
        debug_assert!(result.is_err());
        let _rug_ed_tests_llm_16_951_rrrruuuugggg_test_as_str = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_952 {
    use crate::read::decode_hex_val;
    #[test]
    fn test_decode_hex_val() {
        let _rug_st_tests_llm_16_952_rrrruuuugggg_test_decode_hex_val = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 9;
        let rug_fuzz_3 = 10;
        let rug_fuzz_4 = 15;
        let rug_fuzz_5 = 16;
        let rug_fuzz_6 = 255;
        debug_assert_eq!(decode_hex_val(rug_fuzz_0), Some(0));
        debug_assert_eq!(decode_hex_val(rug_fuzz_1), Some(1));
        debug_assert_eq!(decode_hex_val(rug_fuzz_2), Some(9));
        debug_assert_eq!(decode_hex_val(rug_fuzz_3), Some(10));
        debug_assert_eq!(decode_hex_val(rug_fuzz_4), Some(15));
        debug_assert_eq!(decode_hex_val(rug_fuzz_5), Some(16));
        debug_assert_eq!(decode_hex_val(rug_fuzz_6), None);
        let _rug_ed_tests_llm_16_952_rrrruuuugggg_test_decode_hex_val = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_960_llm_16_959 {
    use crate::read::parse_escape;
    use crate::read::Position;
    use crate::read::Read;
    use crate::read::ErrorCode;
    use crate::read::Error;
    use crate::read::Result;
    use std::io;
    use crate::read::private::Sealed;
    struct MockReader {
        data: Vec<u8>,
        index: usize,
    }
    impl MockReader {
        fn new(data: Vec<u8>) -> MockReader {
            MockReader { data, index: 0 }
        }
    }
    impl Sealed for MockReader {}
    impl<'de> Read<'de> for MockReader {
        fn next(&mut self) -> Result<Option<u8>> {
            if self.index >= self.data.len() {
                Ok(None)
            } else {
                let ch = self.data[self.index];
                self.index += 1;
                Ok(Some(ch))
            }
        }
        fn peek(&mut self) -> Result<Option<u8>> {
            if self.index >= self.data.len() {
                Ok(None)
            } else {
                Ok(Some(self.data[self.index]))
            }
        }
        fn discard(&mut self) {}
        fn position(&self) -> Position {
            Position { line: 1, column: 1 }
        }
        fn peek_position(&self) -> Position {
            Position { line: 1, column: 1 }
        }
        fn byte_offset(&self) -> usize {
            self.index
        }
        fn parse_str<'s>(
            &'s mut self,
            _scratch: &'s mut Vec<u8>,
        ) -> Result<crate::read::Reference<'de, 's, str>> {
            Ok(crate::read::Reference::Borrowed(""))
        }
        fn parse_str_raw<'s>(
            &'s mut self,
            _scratch: &'s mut Vec<u8>,
        ) -> Result<crate::read::Reference<'de, 's, [u8]>> {
            Ok(crate::read::Reference::Borrowed(&[]))
        }
        fn ignore_str(&mut self) -> Result<()> {
            Ok(())
        }
        fn decode_hex_escape(&mut self) -> Result<u16> {
            Ok(0)
        }
        #[cfg(feature = "raw_value")]
        fn begin_raw_buffering(&mut self) {}
        #[cfg(feature = "raw_value")]
        fn end_raw_buffering<V>(&mut self, _visitor: V) -> Result<V::Value>
        where
            V: serde::de::Visitor<'de>,
        {
            Err(Error::Syntax(ErrorCode::Custom("Unexpected end of hex escape")))
        }
        const should_early_return_if_failed: bool = false;
        fn set_failed(&mut self, _failed: &mut bool) {}
    }
    #[test]
    fn test_parse_escape() {
        let _rug_st_tests_llm_16_960_llm_16_959_rrrruuuugggg_test_parse_escape = 0;
        let rug_fuzz_0 = b'\\';
        let mut reader = MockReader::new(vec![rug_fuzz_0, b'n']);
        let mut scratch = Vec::new();
        debug_assert!(parse_escape(& mut reader, & mut scratch).is_ok());
        let _rug_ed_tests_llm_16_960_llm_16_959_rrrruuuugggg_test_parse_escape = 0;
    }
}
#[cfg(test)]
mod tests_rug_497 {
    use super::*;
    use crate::de::{SliceRead, Read};
    #[test]
    fn test_next_or_eof() {
        let _rug_st_tests_rug_497_rrrruuuugggg_test_next_or_eof = 0;
        let mut p0: SliceRead<'_> = SliceRead::new(&[]);
        crate::read::next_or_eof(&mut p0);
        let _rug_ed_tests_rug_497_rrrruuuugggg_test_next_or_eof = 0;
    }
}
#[cfg(test)]
mod tests_rug_498 {
    use super::*;
    use crate::error::{ErrorCode, Error};
    use crate::de::{Read, SliceRead};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_498_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "Sample Error Message";
        let mut p0: &mut SliceRead = &mut SliceRead::new(&[]);
        let p1: ErrorCode = ErrorCode::Message(Box::<str>::from(rug_fuzz_0));
        let result: Result<()> = read::error(p0, p1);
        debug_assert_eq!(result.is_err(), true);
        let _rug_ed_tests_rug_498_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_499 {
    use super::*;
    use crate::de::{StrRead, Read};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_499_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "";
        let mut p0: &mut StrRead = &mut StrRead::new(rug_fuzz_0);
        crate::read::ignore_escape(p0).unwrap();
        let _rug_ed_tests_rug_499_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_500 {
    use super::*;
    use crate::read::IoRead;
    use std::io::{self, Read};
    use std::net::TcpStream;
    use std::process::ChildStdout;
    struct MockReader;
    impl Read for MockReader {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            unimplemented!()
        }
    }
    #[test]
    fn test_new_with_tcp_stream() {
        let _rug_st_tests_rug_500_rrrruuuugggg_test_new_with_tcp_stream = 0;
        let p0: TcpStream = unimplemented!();
        let _: IoRead<TcpStream> = IoRead::<TcpStream>::new(p0);
        let _rug_ed_tests_rug_500_rrrruuuugggg_test_new_with_tcp_stream = 0;
    }
    #[test]
    fn test_new_with_child_stdout() {
        let _rug_st_tests_rug_500_rrrruuuugggg_test_new_with_child_stdout = 0;
        let p0: ChildStdout = unimplemented!();
        let _: IoRead<ChildStdout> = IoRead::<ChildStdout>::new(p0);
        let _rug_ed_tests_rug_500_rrrruuuugggg_test_new_with_child_stdout = 0;
    }
}
#[cfg(test)]
mod tests_rug_510 {
    use super::*;
    use crate::read::SliceRead;
    #[test]
    fn test_position_of_index() {
        let _rug_st_tests_rug_510_rrrruuuugggg_test_position_of_index = 0;
        let rug_fuzz_0 = b"sample data";
        let rug_fuzz_1 = 5;
        let mut p0: SliceRead<'static> = SliceRead::new(rug_fuzz_0);
        let p1: usize = rug_fuzz_1;
        p0.position_of_index(p1);
        let _rug_ed_tests_rug_510_rrrruuuugggg_test_position_of_index = 0;
    }
}
#[cfg(test)]
mod tests_rug_512 {
    use super::*;
    use crate::de::Read;
    use crate::read::SliceRead;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_512_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"sample data";
        let mut p0: SliceRead<'static> = SliceRead::new(rug_fuzz_0);
        <SliceRead<'static> as Read<'static>>::next(&mut p0);
        let _rug_ed_tests_rug_512_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_513 {
    use super::*;
    use crate::de::Read;
    use crate::read::SliceRead;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_513_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"sample data";
        let mut p0: SliceRead<'static> = SliceRead::new(rug_fuzz_0);
        <read::SliceRead<'_>>::position(&p0);
        let _rug_ed_tests_rug_513_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_514 {
    use super::*;
    use crate::de::Read;
    use crate::read::SliceRead;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_514_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"sample data";
        let mut p0: SliceRead<'static> = SliceRead::new(rug_fuzz_0);
        <read::SliceRead<'static> as read::Read<'static>>::peek_position(&p0);
        let _rug_ed_tests_rug_514_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_515 {
    use super::*;
    use crate::read::Read;
    use std::vec::Vec;
    #[test]
    fn test_parse_str() {
        let _rug_st_tests_rug_515_rrrruuuugggg_test_parse_str = 0;
        let rug_fuzz_0 = b"sample data";
        let mut p0: read::SliceRead<'_> = read::SliceRead::new(rug_fuzz_0);
        let mut p1: Vec<u8> = Vec::new();
        p0.parse_str(&mut p1);
        let _rug_ed_tests_rug_515_rrrruuuugggg_test_parse_str = 0;
    }
}
#[cfg(test)]
mod tests_rug_516 {
    use super::*;
    use crate::de::Read;
    use crate::read::{SliceRead, IoRead};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_516_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"sample data";
        let mut v86: SliceRead<'static> = SliceRead::new(rug_fuzz_0);
        let mut p0 = &mut v86;
        let mut p1: Vec<u8> = Vec::new();
        <SliceRead<'_>>::parse_str_raw(p0, &mut p1).unwrap();
        let _rug_ed_tests_rug_516_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_517 {
    use super::*;
    use crate::de::Read;
    use crate::read::SliceRead;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_517_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"sample data";
        let mut v86: SliceRead<'static> = SliceRead::new(rug_fuzz_0);
        <read::SliceRead<'static> as read::Read<'static>>::ignore_str(&mut v86).unwrap();
        let _rug_ed_tests_rug_517_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_518 {
    use super::*;
    use crate::de::Read;
    use crate::read::SliceRead;
    #[test]
    fn test_decode_hex_escape() {
        let _rug_st_tests_rug_518_rrrruuuugggg_test_decode_hex_escape = 0;
        let rug_fuzz_0 = b"sample data";
        let rug_fuzz_1 = 2;
        let slice = rug_fuzz_0;
        let mut slice_read: SliceRead<'_> = SliceRead::new(slice);
        slice_read.index = rug_fuzz_1;
        let result = <SliceRead<'_>>::decode_hex_escape(&mut slice_read);
        debug_assert!(result.is_ok());
        let _rug_ed_tests_rug_518_rrrruuuugggg_test_decode_hex_escape = 0;
    }
}
#[cfg(test)]
mod tests_rug_519 {
    use super::*;
    use crate::read::StrRead;
    use crate::read::SliceRead;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_519_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "{\"key\": \"value\"}";
        let p0 = rug_fuzz_0;
        StrRead::new(&p0);
        let _rug_ed_tests_rug_519_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_520 {
    use super::*;
    use crate::de::Read;
    use crate::de::StrRead;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_520_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = r#"{"name":"John","age":30,"city":"New York"}"#;
        let mut p0: StrRead<'static> = StrRead::new(rug_fuzz_0);
        p0.peek_position();
        let _rug_ed_tests_rug_520_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_521 {
    use super::*;
    use crate::de::Read;
    use crate::de::{StrRead, Read as DeRead};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_521_rrrruuuugggg_test_rug = 0;
        let mut p0: StrRead<'static> = unimplemented!();
        let mut p1: Vec<u8> = unimplemented!();
        <StrRead<'static> as DeRead>::parse_str(&mut p0, &mut p1).unwrap();
        let _rug_ed_tests_rug_521_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_522 {
    use super::*;
    use crate::de::Read;
    use crate::de::StrRead;
    use std::vec::Vec;
    #[test]
    fn test_parse_str_raw() {
        let _rug_st_tests_rug_522_rrrruuuugggg_test_parse_str_raw = 0;
        let mut p0: StrRead<'static> = unimplemented!();
        let mut p1: Vec<u8> = unimplemented!();
        p0.parse_str_raw(&mut p1).unwrap();
        let _rug_ed_tests_rug_522_rrrruuuugggg_test_parse_str_raw = 0;
    }
}
#[cfg(test)]
mod tests_rug_524 {
    use super::*;
    use crate::de::Read;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_524_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "";
        let mut p0: StrRead = StrRead::new(rug_fuzz_0);
        let result: Result<Option<u8>> = p0.next();
        let _rug_ed_tests_rug_524_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_525 {
    use super::*;
    use crate::de::{StrRead, Read};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_525_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "";
        let mut p0: StrRead = StrRead::new(rug_fuzz_0);
        p0.peek();
        let _rug_ed_tests_rug_525_rrrruuuugggg_test_rug = 0;
    }
}
#[test]
fn test_position() {
    #[cfg(test)]
    mod tests_rug_526_prepare {
        use crate::de::{Read, StrRead};
        #[test]
        fn sample() {
            let _rug_st_tests_rug_526_prepare_rrrruuuugggg_sample = 0;
            let rug_fuzz_0 = "";
            let mut v1: StrRead = StrRead::new(rug_fuzz_0);
            let _rug_ed_tests_rug_526_prepare_rrrruuuugggg_sample = 0;
        }
    }
    let mut p0: StrRead = StrRead::new("");
    p0.position();
}
#[cfg(test)]
mod tests_rug_527 {
    use super::*;
    use crate::de::Read;
    use crate::de::SliceRead;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_527_rrrruuuugggg_test_rug = 0;
        let mut v2: SliceRead = SliceRead::new(&[]);
        let mut p0: &mut SliceRead = &mut v2;
        p0.byte_offset();
        let _rug_ed_tests_rug_527_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_528 {
    use super::*;
    use crate::de::Read;
    use crate::read::{StrRead, IoRead};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_528_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "";
        let mut p0: StrRead = StrRead::new(rug_fuzz_0);
        let mut p1: Vec<u8> = Vec::new();
        p0.parse_str(&mut p1);
        let _rug_ed_tests_rug_528_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_529 {
    use super::*;
    use crate::de::Read;
    use crate::de::StrRead;
    use std::vec::Vec;
    #[test]
    fn test_parse_str_raw() {
        let _rug_st_tests_rug_529_rrrruuuugggg_test_parse_str_raw = 0;
        let rug_fuzz_0 = "";
        let mut p0: StrRead = StrRead::new(rug_fuzz_0);
        let mut p1: Vec<u8> = Vec::new();
        p0.parse_str_raw(&mut p1).unwrap();
        let _rug_ed_tests_rug_529_rrrruuuugggg_test_parse_str_raw = 0;
    }
}
#[cfg(test)]
mod tests_rug_530 {
    use super::*;
    use crate::de::Read;
    use crate::de::SliceRead;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_530_rrrruuuugggg_test_rug = 0;
        let mut p0: SliceRead = SliceRead::new(&[]);
        p0.decode_hex_escape();
        let _rug_ed_tests_rug_530_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_531 {
    use super::*;
    use crate::de::Read;
    use crate::de::SliceRead;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_531_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = true;
        let mut p0: SliceRead = SliceRead::new(&[]);
        let mut p1: bool = rug_fuzz_0;
        p0.set_failed(&mut p1);
        let _rug_ed_tests_rug_531_rrrruuuugggg_test_rug = 0;
    }
}
