//! CBOR ([RFC 7049](http://tools.ietf.org/html/rfc7049))
//! encoder implementation.
//!
//! This module provides an `Encoder` to directly encode Rust types into
//! CBOR, and a `GenericEncoder` which encodes a `Value` into CBOR.
//!
//! # Example 1: Direct encoding
//!
//! ```
//! extern crate cbor;
//! extern crate rustc_serialize;
//!
//! use cbor::Encoder;
//! use rustc_serialize::hex::FromHex;
//! use std::io::Cursor;
//!
//! fn main() {
//!     let mut e = Encoder::new(Cursor::new(Vec::new()));
//!     e.u16(1000).unwrap();
//!     assert_eq!("1903e8".from_hex().unwrap(), e.into_writer().into_inner())
//! }
//! ```
//!
//! # Example 2: Direct encoding (indefinite string)
//!
//! ```
//! extern crate cbor;
//! extern crate rustc_serialize;
//!
//! use cbor::Encoder;
//! use rustc_serialize::hex::FromHex;
//! use std::io::Cursor;
//!
//! fn main() {
//!     let mut e = Encoder::new(Cursor::new(Vec::new()));
//!     e.text_iter(vec!["strea", "ming"].into_iter()).unwrap();
//!     let output = "7f657374726561646d696e67ff".from_hex().unwrap();
//!     assert_eq!(output, e.into_writer().into_inner())
//! }
//! ```
//!
//! # Example 3: Direct encoding (nested array)
//!
//! ```
//! extern crate cbor;
//! extern crate rustc_serialize;
//!
//! use cbor::Encoder;
//! use rustc_serialize::hex::FromHex;
//! use std::io::Cursor;
//!
//! fn main() {
//!     let mut e = Encoder::new(Cursor::new(Vec::new()));
//!     e.array(3)
//!      .and(e.u8(1))
//!      .and(e.array(2)).and(e.u8(2)).and(e.u8(3))
//!      .and(e.array(2)).and(e.u8(4)).and(e.u8(5))
//!      .unwrap();
//!     let output = "8301820203820405".from_hex().unwrap();
//!     assert_eq!(output, e.into_writer().into_inner())
//! }
//! ```
use byteorder::{BigEndian, WriteBytesExt};
use std::io;
use std::error::Error;
use std::fmt;
use types::{Tag, Type};
use value::{Bytes, Int, Key, Simple, Text, Value};
pub type EncodeResult = Result<(), EncodeError>;
#[derive(Debug)]
pub enum EncodeError {
    /// Some I/O error
    IoError(io::Error),
    /// The end of file has been encountered unexpectedly
    UnexpectedEOF,
    /// The provided `Simple` value is neither unassigned nor reserved
    InvalidSimpleValue(Simple),
    /// Certain values (e.g. `Value::Break`) are not legal to encode as
    /// independent units. Attempting to do so will trigger this error.
    InvalidValue(Value),
    /// Some other error.
    Other(Box<dyn Error + Send + Sync>),
}
impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            EncodeError::IoError(ref e) => write!(f, "EncodeError: I/O error: {}", * e),
            EncodeError::UnexpectedEOF => {
                write!(f, "EncodeError: unexpected end-of-file")
            }
            EncodeError::InvalidValue(ref v) => {
                write!(f, "EncodeError: invalid value {:?}", v)
            }
            EncodeError::InvalidSimpleValue(ref s) => {
                write!(f, "EncodeError: invalid simple value {:?}", s)
            }
            EncodeError::Other(ref e) => write!(f, "EncodeError: other error: {}", e),
        }
    }
}
impl Error for EncodeError {
    fn description(&self) -> &str {
        match *self {
            EncodeError::IoError(_) => "i/o error",
            EncodeError::UnexpectedEOF => "unexpected eof",
            EncodeError::InvalidValue(_) => "invalid value",
            EncodeError::InvalidSimpleValue(_) => "invalid simple value",
            EncodeError::Other(_) => "other error",
        }
    }
    fn cause(&self) -> Option<&Error> {
        match *self {
            EncodeError::IoError(ref e) => Some(e),
            EncodeError::Other(ref e) => Some(&**e),
            _ => None,
        }
    }
}
impl From<io::Error> for EncodeError {
    fn from(e: io::Error) -> EncodeError {
        EncodeError::IoError(e)
    }
}
/// The actual encoder type definition
pub struct Encoder<W> {
    writer: W,
}
impl<W: WriteBytesExt> Encoder<W> {
    pub fn new(w: W) -> Encoder<W> {
        Encoder { writer: w }
    }
    pub fn into_writer(self) -> W {
        self.writer
    }
    pub fn writer(&mut self) -> &mut W {
        &mut self.writer
    }
    pub fn u8(&mut self, x: u8) -> EncodeResult {
        let ref mut w = self.writer;
        match x {
            0..=23 => w.write_u8(x).map_err(From::from),
            _ => w.write_u8(24).and(w.write_u8(x)).map_err(From::from),
        }
    }
    pub fn u16(&mut self, x: u16) -> EncodeResult {
        let ref mut w = self.writer;
        match x {
            0..=23 => w.write_u8(x as u8).map_err(From::from),
            24..=0xFF => w.write_u8(24).and(w.write_u8(x as u8)).map_err(From::from),
            _ => w.write_u8(25).and(w.write_u16::<BigEndian>(x)).map_err(From::from),
        }
    }
    pub fn u32(&mut self, x: u32) -> EncodeResult {
        let ref mut w = self.writer;
        match x {
            0..=23 => w.write_u8(x as u8).map_err(From::from),
            24..=0xFF => w.write_u8(24).and(w.write_u8(x as u8)).map_err(From::from),
            0x100..=0xFFFF => {
                w
                    .write_u8(25)
                    .and(w.write_u16::<BigEndian>(x as u16))
                    .map_err(From::from)
            }
            _ => w.write_u8(26).and(w.write_u32::<BigEndian>(x)).map_err(From::from),
        }
    }
    pub fn u64(&mut self, x: u64) -> EncodeResult {
        let ref mut w = self.writer;
        match x {
            0..=23 => w.write_u8(x as u8).map_err(From::from),
            24..=0xFF => w.write_u8(24).and(w.write_u8(x as u8)).map_err(From::from),
            0x100..=0xFFFF => {
                w
                    .write_u8(25)
                    .and(w.write_u16::<BigEndian>(x as u16))
                    .map_err(From::from)
            }
            0x100000..=0xFFFFFFFF => {
                w
                    .write_u8(26)
                    .and(w.write_u32::<BigEndian>(x as u32))
                    .map_err(From::from)
            }
            _ => w.write_u8(27).and(w.write_u64::<BigEndian>(x)).map_err(From::from),
        }
    }
    pub fn i8(&mut self, x: i8) -> EncodeResult {
        if x >= 0 {
            self.u8(x as u8)
        } else {
            let ref mut w = self.writer;
            match (-1 - x) as u8 {
                n @ 0..=23 => w.write_u8(0b001_00000 | n).map_err(From::from),
                n => w.write_u8(0b001_00000 | 24).and(w.write_u8(n)).map_err(From::from),
            }
        }
    }
    pub fn i16(&mut self, x: i16) -> EncodeResult {
        if x >= 0 {
            self.u16(x as u16)
        } else {
            let ref mut w = self.writer;
            match (-1 - x) as u16 {
                n @ 0..=23 => w.write_u8(0b001_00000 | n as u8).map_err(From::from),
                n @ 24..=0xFF => {
                    w
                        .write_u8(0b001_00000 | 24)
                        .and(w.write_u8(n as u8))
                        .map_err(From::from)
                }
                n => {
                    w
                        .write_u8(0b001_00000 | 25)
                        .and(w.write_u16::<BigEndian>(n))
                        .map_err(From::from)
                }
            }
        }
    }
    pub fn i32(&mut self, x: i32) -> EncodeResult {
        if x >= 0 {
            self.u32(x as u32)
        } else {
            let ref mut w = self.writer;
            match (-1 - x) as u32 {
                n @ 0..=23 => w.write_u8(0b001_00000 | n as u8).map_err(From::from),
                n @ 24..=0xFF => {
                    w
                        .write_u8(0b001_00000 | 24)
                        .and(w.write_u8(n as u8))
                        .map_err(From::from)
                }
                n @ 0x100..=0xFFFF => {
                    w
                        .write_u8(0b001_00000 | 25)
                        .and(w.write_u16::<BigEndian>(n as u16))
                        .map_err(From::from)
                }
                n => {
                    w
                        .write_u8(0b001_00000 | 26)
                        .and(w.write_u32::<BigEndian>(n))
                        .map_err(From::from)
                }
            }
        }
    }
    pub fn i64(&mut self, x: i64) -> EncodeResult {
        if x >= 0 {
            self.u64(x as u64)
        } else {
            let ref mut w = self.writer;
            match (-1 - x) as u64 {
                n @ 0..=23 => w.write_u8(0b001_00000 | n as u8).map_err(From::from),
                n @ 24..=0xFF => {
                    w
                        .write_u8(0b001_00000 | 24)
                        .and(w.write_u8(n as u8))
                        .map_err(From::from)
                }
                n @ 0x100..=0xFFFF => {
                    w
                        .write_u8(0b001_00000 | 25)
                        .and(w.write_u16::<BigEndian>(n as u16))
                        .map_err(From::from)
                }
                n @ 0x100000..=0xFFFFFFFF => {
                    w
                        .write_u8(0b001_00000 | 26)
                        .and(w.write_u32::<BigEndian>(n as u32))
                        .map_err(From::from)
                }
                n => {
                    w
                        .write_u8(0b001_00000 | 27)
                        .and(w.write_u64::<BigEndian>(n))
                        .map_err(From::from)
                }
            }
        }
    }
    pub fn int(&mut self, x: Int) -> EncodeResult {
        match x {
            Int::Pos(v) => self.u64(v),
            Int::Neg(v) => {
                let ref mut w = self.writer;
                match v {
                    n @ 0..=23 => w.write_u8(0b001_00000 | n as u8).map_err(From::from),
                    n @ 24..=0xFF => {
                        w
                            .write_u8(0b001_00000 | 24)
                            .and(w.write_u8(n as u8))
                            .map_err(From::from)
                    }
                    n @ 0x100..=0xFFFF => {
                        w
                            .write_u8(0b001_00000 | 25)
                            .and(w.write_u16::<BigEndian>(n as u16))
                            .map_err(From::from)
                    }
                    n @ 0x100000..=0xFFFFFFFF => {
                        w
                            .write_u8(0b001_00000 | 26)
                            .and(w.write_u32::<BigEndian>(n as u32))
                            .map_err(From::from)
                    }
                    n => {
                        w
                            .write_u8(0b001_00000 | 27)
                            .and(w.write_u64::<BigEndian>(n))
                            .map_err(From::from)
                    }
                }
            }
        }
    }
    pub fn f32(&mut self, x: f32) -> EncodeResult {
        self.writer
            .write_u8(0b111_00000 | 26)
            .and(self.writer.write_f32::<BigEndian>(x))
            .map_err(From::from)
    }
    pub fn f64(&mut self, x: f64) -> EncodeResult {
        self.writer
            .write_u8(0b111_00000 | 27)
            .and(self.writer.write_f64::<BigEndian>(x))
            .map_err(From::from)
    }
    pub fn bool(&mut self, x: bool) -> EncodeResult {
        self.writer.write_u8(0b111_00000 | if x { 21 } else { 20 }).map_err(From::from)
    }
    pub fn simple(&mut self, x: Simple) -> EncodeResult {
        let ref mut w = self.writer;
        match x {
            Simple::Unassigned(n) => {
                match n {
                    0..=19 | 28..=30 => w.write_u8(0b111_00000 | n).map_err(From::from),
                    32..=255 => {
                        w
                            .write_u8(0b111_00000 | 24)
                            .and(w.write_u8(n))
                            .map_err(From::from)
                    }
                    _ => Err(EncodeError::InvalidSimpleValue(x)),
                }
            }
            Simple::Reserved(n) => {
                match n {
                    0..=31 => {
                        w
                            .write_u8(0b111_00000 | 24)
                            .and(w.write_u8(n))
                            .map_err(From::from)
                    }
                    _ => Err(EncodeError::InvalidSimpleValue(x)),
                }
            }
        }
    }
    pub fn bytes(&mut self, x: &[u8]) -> EncodeResult {
        self.type_len(Type::Bytes, x.len() as u64)
            .and(self.writer.write_all(x).map_err(From::from))
    }
    /// Indefinite byte string encoding. (RFC 7049 section 2.2.2)
    pub fn bytes_iter<'r, I: Iterator<Item = &'r [u8]>>(
        &mut self,
        iter: I,
    ) -> EncodeResult {
        self.writer.write_u8(0b010_11111)?;
        for x in iter {
            self.bytes(x)?
        }
        self.writer.write_u8(0b111_11111).map_err(From::from)
    }
    pub fn text(&mut self, x: &str) -> EncodeResult {
        self.type_len(Type::Text, x.len() as u64)
            .and(self.writer.write_all(x.as_bytes()).map_err(From::from))
    }
    /// Indefinite string encoding. (RFC 7049 section 2.2.2)
    pub fn text_iter<'r, I: Iterator<Item = &'r str>>(
        &mut self,
        iter: I,
    ) -> EncodeResult {
        self.writer.write_u8(0b011_11111)?;
        for x in iter {
            self.text(x)?
        }
        self.writer.write_u8(0b111_11111).map_err(From::from)
    }
    pub fn null(&mut self) -> EncodeResult {
        self.writer.write_u8(0b111_00000 | 22).map_err(From::from)
    }
    pub fn undefined(&mut self) -> EncodeResult {
        self.writer.write_u8(0b111_00000 | 23).map_err(From::from)
    }
    pub fn tag(&mut self, x: Tag) -> EncodeResult {
        self.type_len(Type::Tagged, x.to())
    }
    pub fn array(&mut self, len: usize) -> EncodeResult {
        self.type_len(Type::Array, len as u64)
    }
    /// Indefinite array encoding. (RFC 7049 section 2.2.1)
    pub fn array_begin(&mut self) -> EncodeResult {
        self.writer.write_u8(0b100_11111).map_err(From::from)
    }
    /// End of indefinite array encoding. (RFC 7049 section 2.2.1)
    pub fn array_end(&mut self) -> EncodeResult {
        self.writer.write_u8(0b111_11111).map_err(From::from)
    }
    pub fn object(&mut self, len: usize) -> EncodeResult {
        self.type_len(Type::Object, len as u64)
    }
    /// Indefinite object encoding. (RFC 7049 section 2.2.1)
    pub fn object_begin(&mut self) -> EncodeResult {
        self.writer.write_u8(0b101_11111).map_err(From::from)
    }
    /// End of indefinite object encoding. (RFC 7049 section 2.2.1)
    pub fn object_end(&mut self) -> EncodeResult {
        self.writer.write_u8(0b111_11111).map_err(From::from)
    }
    fn type_len(&mut self, t: Type, x: u64) -> EncodeResult {
        let ref mut w = self.writer;
        match x {
            0..=23 => w.write_u8(t.major() << 5 | x as u8).map_err(From::from),
            24..=0xFF => {
                w
                    .write_u8(t.major() << 5 | 24)
                    .and(w.write_u8(x as u8))
                    .map_err(From::from)
            }
            0x100..=0xFFFF => {
                w
                    .write_u8(t.major() << 5 | 25)
                    .and(w.write_u16::<BigEndian>(x as u16))
                    .map_err(From::from)
            }
            0x100000..=0xFFFFFFFF => {
                w
                    .write_u8(t.major() << 5 | 26)
                    .and(w.write_u32::<BigEndian>(x as u32))
                    .map_err(From::from)
            }
            _ => {
                w
                    .write_u8(t.major() << 5 | 27)
                    .and(w.write_u64::<BigEndian>(x))
                    .map_err(From::from)
            }
        }
    }
}
/// A generic encoder encodes a `Value`.
pub struct GenericEncoder<W> {
    encoder: Encoder<W>,
}
impl<W: WriteBytesExt> GenericEncoder<W> {
    pub fn new(w: W) -> GenericEncoder<W> {
        GenericEncoder {
            encoder: Encoder::new(w),
        }
    }
    pub fn from_encoder(e: Encoder<W>) -> GenericEncoder<W> {
        GenericEncoder { encoder: e }
    }
    pub fn into_inner(self) -> Encoder<W> {
        self.encoder
    }
    pub fn borrow_mut(&mut self) -> &mut Encoder<W> {
        &mut self.encoder
    }
    pub fn value(&mut self, x: &Value) -> EncodeResult {
        match *x {
            Value::Array(ref vv) => {
                self.encoder.array(vv.len())?;
                for v in vv {
                    self.value(v)?
                }
                Ok(())
            }
            Value::Bytes(Bytes::Bytes(ref bb)) => self.encoder.bytes(&bb[..]),
            Value::Bytes(Bytes::Chunks(ref bb)) => {
                self.encoder.bytes_iter(bb.iter().map(|v| &v[..]))
            }
            Value::Text(Text::Text(ref txt)) => self.encoder.text(txt),
            Value::Text(Text::Chunks(ref txt)) => {
                self.encoder.text_iter(txt.iter().map(|v| &v[..]))
            }
            Value::Map(ref m) => {
                self.encoder.object(m.len())?;
                for (k, v) in m {
                    self.key(k).and(self.value(v))?
                }
                Ok(())
            }
            Value::Tagged(t, ref val) => {
                self.encoder.tag(t)?;
                self.value(&*val)
            }
            Value::Undefined => self.encoder.undefined(),
            Value::Null => self.encoder.null(),
            Value::Simple(s) => self.encoder.simple(s),
            Value::Bool(b) => self.encoder.bool(b),
            Value::U8(n) => self.encoder.u8(n),
            Value::U16(n) => self.encoder.u16(n),
            Value::U32(n) => self.encoder.u32(n),
            Value::U64(n) => self.encoder.u64(n),
            Value::F32(n) => self.encoder.f32(n),
            Value::F64(n) => self.encoder.f64(n),
            Value::I8(n) => self.encoder.i8(n),
            Value::I16(n) => self.encoder.i16(n),
            Value::I32(n) => self.encoder.i32(n),
            Value::I64(n) => self.encoder.i64(n),
            Value::Int(n) => self.encoder.int(n),
            Value::Break => Err(EncodeError::InvalidValue(Value::Break)),
        }
    }
    fn key(&mut self, x: &Key) -> EncodeResult {
        match *x {
            Key::Bool(b) => self.encoder.bool(b),
            Key::Int(n) => self.encoder.int(n),
            Key::Bytes(Bytes::Bytes(ref bb)) => self.encoder.bytes(&bb[..]),
            Key::Bytes(Bytes::Chunks(ref bb)) => {
                self.encoder.bytes_iter(bb.iter().map(|v| &v[..]))
            }
            Key::Text(Text::Text(ref txt)) => self.encoder.text(txt),
            Key::Text(Text::Chunks(ref txt)) => {
                self.encoder.text_iter(txt.iter().map(|v| &v[..]))
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use rustc_serialize::hex::FromHex;
    use std::{f32, f64};
    use std::io::Cursor;
    use super::*;
    use types::Tag;
    use value::Simple;
    #[test]
    fn unsigned() {
        encoded("00", |mut e| e.u8(0));
        encoded("01", |mut e| e.u8(1));
        encoded("0a", |mut e| e.u8(10));
        encoded("17", |mut e| e.u8(23));
        encoded("1818", |mut e| e.u8(24));
        encoded("1819", |mut e| e.u8(25));
        encoded("1864", |mut e| e.u8(100));
        encoded("1903e8", |mut e| e.u16(1000));
        encoded("1a000f4240", |mut e| e.u32(1000000));
        encoded("1b000000e8d4a51000", |mut e| e.u64(1000000000000));
        encoded("1bffffffffffffffff", |mut e| e.u64(18446744073709551615))
    }
    #[test]
    fn signed() {
        encoded("20", |mut e| e.i8(-1));
        encoded("29", |mut e| e.i8(-10));
        encoded("3863", |mut e| e.i8(-100));
        encoded("3901f3", |mut e| e.i16(-500));
        encoded("3903e7", |mut e| e.i16(-1000));
        encoded("3a00053d89", |mut e| e.i32(-343434));
        encoded("3b000000058879da85", |mut e| e.i64(-23764523654))
    }
    #[test]
    fn bool() {
        encoded("f4", |mut e| e.bool(false));
        encoded("f5", |mut e| e.bool(true))
    }
    #[test]
    fn simple() {
        encoded("f0", |mut e| e.simple(Simple::Unassigned(16)));
        encoded("f818", |mut e| e.simple(Simple::Reserved(24)));
        encoded("f8ff", |mut e| e.simple(Simple::Unassigned(255)))
    }
    #[test]
    fn float() {
        encoded("fa47c35000", |mut e| e.f32(100000.0));
        encoded("fa7f7fffff", |mut e| e.f32(3.4028234663852886e+38));
        encoded("fbc010666666666666", |mut e| e.f64(-4.1));
        encoded("fa7f800000", |mut e| e.f32(f32::INFINITY));
        encoded("faff800000", |mut e| e.f32(-f32::INFINITY));
        encoded("fa7fc00000", |mut e| e.f32(f32::NAN));
        encoded("fb7ff0000000000000", |mut e| e.f64(f64::INFINITY));
        encoded("fbfff0000000000000", |mut e| e.f64(-f64::INFINITY));
        encoded("fb7ff8000000000000", |mut e| e.f64(f64::NAN));
    }
    #[test]
    fn bytes() {
        encoded("4401020304", |mut e| e.bytes(&vec![1, 2, 3, 4][..]));
    }
    #[test]
    fn text() {
        encoded("62c3bc", |mut e| e.text("\u{00fc}"));
        encoded(
            "781f64667364667364660d0a7364660d0a68656c6c6f0d0a736466736673646673",
            |mut e| { e.text("dfsdfsdf\r\nsdf\r\nhello\r\nsdfsfsdfs") },
        );
    }
    #[test]
    fn indefinite_text() {
        encoded(
            "7f657374726561646d696e67ff",
            |mut e| { e.text_iter(vec!["strea", "ming"].into_iter()) },
        )
    }
    #[test]
    fn indefinite_bytes() {
        encoded(
            "5f457374726561446d696e67ff",
            |mut e| {
                e.bytes_iter(vec!["strea".as_bytes(), "ming".as_bytes()].into_iter())
            },
        )
    }
    #[test]
    fn option() {
        encoded("f6", |mut e| e.null())
    }
    #[test]
    fn tagged() {
        encoded(
            "c11a514b67b0",
            |mut e| {
                e.tag(Tag::Timestamp)?;
                e.u32(1363896240)
            },
        )
    }
    #[test]
    fn array() {
        encoded(
            "83010203",
            |mut e| {
                e.array(3)?;
                e.u32(1)?;
                e.u32(2)?;
                e.u32(3)
            },
        );
        encoded(
            "8301820203820405",
            |mut e| {
                e.array(3)
                    .and(e.u8(1))
                    .and(e.array(2))
                    .and(e.u8(2))
                    .and(e.u8(3))
                    .and(e.array(2))
                    .and(e.u8(4))
                    .and(e.u8(5))
            },
        )
    }
    #[test]
    fn indefinite_array() {
        encoded(
            "9f018202039f0405ffff",
            |mut e| {
                e.array_begin()
                    .and(e.u8(1))
                    .and(e.array(2))
                    .and(e.u8(2))
                    .and(e.u8(3))
                    .and(e.array_begin())
                    .and(e.u8(4))
                    .and(e.u8(5))
                    .and(e.array_end())
                    .and(e.array_end())
            },
        );
        encoded(
            "9f01820203820405ff",
            |mut e| {
                e.array_begin()
                    .and(e.u8(1))
                    .and(e.array(2))
                    .and(e.u8(2))
                    .and(e.u8(3))
                    .and(e.array(2))
                    .and(e.u8(4))
                    .and(e.u8(5))
                    .and(e.array_end())
            },
        );
        encoded(
            "83018202039f0405ff",
            |mut e| {
                e.array(3)
                    .and(e.u8(1))
                    .and(e.array(2))
                    .and(e.u8(2))
                    .and(e.u8(3))
                    .and(e.array_begin())
                    .and(e.u8(4))
                    .and(e.u8(5))
                    .and(e.array_end())
            },
        );
        encoded(
            "83019f0203ff820405",
            |mut e| {
                e.array(3)
                    .and(e.u8(1))
                    .and(e.array_begin())
                    .and(e.u8(2))
                    .and(e.u8(3))
                    .and(e.array_end())
                    .and(e.array(2))
                    .and(e.u8(4))
                    .and(e.u8(5))
            },
        )
    }
    #[test]
    fn object() {
        encoded(
            "a26161016162820203",
            |mut e| {
                e.object(2)?;
                e.text("a").and(e.u8(1))?;
                e.text("b").and(e.array(2)).and(e.u8(2)).and(e.u8(3))
            },
        )
    }
    #[test]
    fn indefinite_object() {
        encoded(
            "bf6346756ef563416d7421ff",
            |mut e| {
                e.object_begin()
                    .and(e.text("Fun"))
                    .and(e.bool(true))
                    .and(e.text("Amt"))
                    .and(e.i8(-2))
                    .and(e.object_end())
            },
        )
    }
    fn encoded<F>(expected: &str, mut f: F)
    where
        F: FnMut(Encoder<Cursor<&mut [u8]>>) -> EncodeResult,
    {
        let mut buffer = vec![0u8; 128];
        assert!(f(Encoder::new(Cursor::new(& mut buffer[..]))).is_ok());
        assert_eq!(& expected.from_hex().unwrap() [..], & buffer[0..expected.len() / 2])
    }
}
#[cfg(test)]
mod tests_rug_122 {
    use super::*;
    use crate::std::error::Error;
    use crate::encoder::{EncodeError, Simple, Value};
    use std::io;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_122_rrrruuuugggg_test_rug = 0;
        let p0 = EncodeError::IoError(io::Error::from(io::ErrorKind::NotFound));
        EncodeError::description(&p0);
        let _rug_ed_tests_rug_122_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_123 {
    use super::*;
    use crate::encoder::EncodeError;
    use std::error::Error;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_123_rrrruuuugggg_test_rug = 0;
        let io_error = std::io::Error::from(std::io::ErrorKind::Other);
        let encode_error = EncodeError::IoError(io_error);
        let p0 = &encode_error;
        let _result = EncodeError::cause(p0);
        let _rug_ed_tests_rug_123_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_124 {
    use super::*;
    use crate::std::convert::From;
    use crate::encoder;
    use std::io::Error;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_124_rrrruuuugggg_test_rug = 0;
        let mut p0 = Error::from(std::io::ErrorKind::NotFound);
        <encoder::EncodeError>::from(p0);
        let _rug_ed_tests_rug_124_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_125 {
    use super::*;
    use std::os::unix::net::UnixStream;
    use byteorder::WriteBytesExt;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(&str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: UnixStream = UnixStream::connect(rug_fuzz_0).expect(rug_fuzz_1);
        crate::encoder::Encoder::<UnixStream>::new(p0);
             }
});    }
}
#[cfg(test)]
mod tests_rug_129 {
    use super::*;
    use crate::encoder::Encoder;
    use byteorder::{BigEndian, WriteBytesExt};
    #[test]
    fn test_u16() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(u16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut writer = Vec::new();
        let mut encoder = Encoder::new(&mut writer);
        let x: u16 = rug_fuzz_0;
        encoder.u16(x).unwrap();
        debug_assert_eq!(writer, vec![25, 0, 100]);
             }
});    }
}
#[cfg(test)]
mod tests_rug_130 {
    use super::*;
    use crate::encoder::Encoder;
    use byteorder::{BigEndian, WriteBytesExt};
    #[test]
    fn test_u32() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u32, u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut buffer: Vec<u8> = Vec::new();
        let mut encoder = Encoder::new(&mut buffer);
        let value: u32 = rug_fuzz_0;
        encoder.u32(value).unwrap();
        let expected: Vec<u8> = vec![rug_fuzz_1, 0, 1, 226, 64];
        debug_assert_eq!(buffer, expected);
             }
});    }
}
#[cfg(test)]
mod tests_rug_131 {
    use super::*;
    use byteorder::{BigEndian, WriteBytesExt};
    use crate::encoder::Encoder;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut writer: Vec<u8> = Vec::new();
        let mut encoder = Encoder::new(&mut writer);
        let mut p0 = encoder;
        let p1: u64 = rug_fuzz_0;
        p0.u64(p1).unwrap();
             }
});    }
}
#[cfg(test)]
mod tests_rug_132 {
    use super::*;
    use crate::encoder::Encoder;
    use crate::EncodeResult;
    use std::io::Cursor;
    #[test]
    fn test_i8() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(i8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut encoder = Encoder::new(Cursor::new(vec![]));
        let x: i8 = -rug_fuzz_0;
        encoder.i8(x).unwrap();
             }
});    }
}
#[cfg(test)]
mod tests_rug_133 {
    use super::*;
    use crate::encoder::{Encoder, EncodeResult};
    use byteorder::{BigEndian, WriteBytesExt};
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(i16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut writer: Vec<u8> = Vec::new();
        let mut encoder = Encoder::new(writer);
        let mut x: i16 = -rug_fuzz_0;
        encoder.i16(x).unwrap();
             }
});    }
}
#[cfg(test)]
mod tests_rug_134 {
    use super::*;
    use crate::encoder::Encoder;
    use crate::EncodeResult;
    use std::io::{Cursor, Write};
    use byteorder::{BigEndian, WriteBytesExt};
    #[test]
    fn test_i32_positive() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut cursor = Cursor::new(Vec::new());
        let mut encoder = Encoder::new(&mut cursor);
        let x: i32 = rug_fuzz_0;
        encoder.i32(x).unwrap();
        let encoded_bytes = cursor.into_inner();
        debug_assert_eq!(encoded_bytes, [0b001_01010]);
             }
});    }
    #[test]
    fn test_i32_negative_u8() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut cursor = Cursor::new(Vec::new());
        let mut encoder = Encoder::new(&mut cursor);
        let x: i32 = -rug_fuzz_0;
        encoder.i32(x).unwrap();
        let encoded_bytes = cursor.into_inner();
        debug_assert_eq!(encoded_bytes, [0b000_00001, 0b001_00111]);
             }
});    }
    #[test]
    fn test_i32_negative_u16() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut cursor = Cursor::new(Vec::new());
        let mut encoder = Encoder::new(&mut cursor);
        let x: i32 = -rug_fuzz_0;
        encoder.i32(x).unwrap();
        let encoded_bytes = cursor.into_inner();
        debug_assert_eq!(encoded_bytes, [0b000_00001, 24, 0b1000_0100]);
             }
});    }
    #[test]
    fn test_i32_negative_u32() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut cursor = Cursor::new(Vec::new());
        let mut encoder = Encoder::new(&mut cursor);
        let x: i32 = -rug_fuzz_0;
        encoder.i32(x).unwrap();
        let encoded_bytes = cursor.into_inner();
        debug_assert_eq!(encoded_bytes, [0b000_00001, 25, 0b1111_1101, 0b1001_1010]);
             }
});    }
}
#[cfg(test)]
mod tests_rug_135 {
    use super::*;
    use byteorder::{BigEndian, WriteBytesExt};
    use crate::encoder::Encoder;
    #[test]
    fn test_i64_positive() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(i64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut buf = Vec::new();
        let mut encoder = Encoder::new(&mut buf);
        let num = rug_fuzz_0;
        encoder.i64(num).unwrap();
             }
});    }
    #[test]
    fn test_i64_negative() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(i64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut buf = Vec::new();
        let mut encoder = Encoder::new(&mut buf);
        let num = -rug_fuzz_0;
        encoder.i64(num).unwrap();
             }
});    }
}
#[cfg(test)]
mod tests_rug_138 {
    use super::*;
    use byteorder::{BigEndian, WriteBytesExt};
    use crate::encoder::{Encoder, EncodeResult};
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut writer = Vec::new();
        let mut encoder = Encoder::new(&mut writer);
        let x: f64 = rug_fuzz_0;
        encoder.f64(x).unwrap();
             }
});    }
}
#[cfg(test)]
mod tests_rug_140 {
    use super::*;
    use crate::encoder::Encoder;
    use crate::value::Simple;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Encoder<Vec<u8>> = Encoder::new(Vec::new());
        let mut p1 = Simple::Unassigned(rug_fuzz_0);
        p0.simple(p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_145 {
    use super::*;
    use crate::encoder::Encoder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_145_rrrruuuugggg_test_rug = 0;
        let mut writer = Vec::new();
        let mut encoder = Encoder::new(writer);
        Encoder::<Vec<u8>>::null(&mut encoder).unwrap();
        debug_assert_eq!(encoder.into_writer(), & [0xF6]);
        let _rug_ed_tests_rug_145_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_146 {
    use super::*;
    use crate::encoder::Encoder;
    #[test]
    fn test_undefined() {
        let _rug_st_tests_rug_146_rrrruuuugggg_test_undefined = 0;
        let mut p0 = Encoder::new(Vec::new());
        Encoder::<Vec<u8>>::undefined(&mut p0).unwrap();
        debug_assert_eq!(p0.into_writer(), vec![0xFF]);
        let _rug_ed_tests_rug_146_rrrruuuugggg_test_undefined = 0;
    }
}
#[cfg(test)]
mod tests_rug_149 {
    use super::*;
    use crate::encoder::Encoder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_149_rrrruuuugggg_test_rug = 0;
        let mut writer = Vec::new();
        let mut encoder = Encoder::new(writer);
        encoder.array_begin().unwrap();
        let _rug_ed_tests_rug_149_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_150 {
    use super::*;
    use crate::encoder::Encoder;
    #[test]
    fn test_array_end() {
        let _rug_st_tests_rug_150_rrrruuuugggg_test_array_end = 0;
        let mut writer = Vec::new();
        let mut encoder = Encoder::new(&mut writer);
        Encoder::<&mut Vec<u8>>::array_end(&mut encoder).unwrap();
        debug_assert_eq!(writer, vec![0xff]);
        let _rug_ed_tests_rug_150_rrrruuuugggg_test_array_end = 0;
    }
}
#[cfg(test)]
mod tests_rug_152 {
    use super::*;
    use crate::encoder::Encoder;
    #[test]
    fn test_object_begin() {
        let _rug_st_tests_rug_152_rrrruuuugggg_test_object_begin = 0;
        let mut writer = Vec::new();
        let mut encoder = Encoder::new(&mut writer);
        encoder.object_begin().unwrap();
        debug_assert_eq!(writer, vec![0b101_11111]);
        let _rug_ed_tests_rug_152_rrrruuuugggg_test_object_begin = 0;
    }
}
#[cfg(test)]
mod tests_rug_153 {
    use super::*;
    use crate::encoder::Encoder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_153_rrrruuuugggg_test_rug = 0;
        let mut writer = Vec::new();
        let mut encoder = Encoder::new(writer);
        encoder.object_end().unwrap();
        let _rug_ed_tests_rug_153_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_155 {
    use super::*;
    use std::fs::File;
    use crate::encoder::{GenericEncoder, Encoder};
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = File::open(rug_fuzz_0).unwrap();
        GenericEncoder::<File>::new(p0);
             }
});    }
}
