//! Serialize a Rust data structure into JSON data.
use crate::error::{Error, ErrorCode, Result};
use crate::io;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::{self, Display};
use core::num::FpCategory;
use serde::ser::{self, Impossible, Serialize};
/// A structure for serializing Rust values into JSON.
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub struct Serializer<W, F = CompactFormatter> {
    writer: W,
    formatter: F,
}
impl<W> Serializer<W>
where
    W: io::Write,
{
    /// Creates a new JSON serializer.
    #[inline]
    pub fn new(writer: W) -> Self {
        Serializer::with_formatter(writer, CompactFormatter)
    }
}
impl<'a, W> Serializer<W, PrettyFormatter<'a>>
where
    W: io::Write,
{
    /// Creates a new JSON pretty print serializer.
    #[inline]
    pub fn pretty(writer: W) -> Self {
        Serializer::with_formatter(writer, PrettyFormatter::new())
    }
}
impl<W, F> Serializer<W, F>
where
    W: io::Write,
    F: Formatter,
{
    /// Creates a new JSON visitor whose output will be written to the writer
    /// specified.
    #[inline]
    pub fn with_formatter(writer: W, formatter: F) -> Self {
        Serializer { writer, formatter }
    }
    /// Unwrap the `Writer` from the `Serializer`.
    #[inline]
    pub fn into_inner(self) -> W {
        self.writer
    }
}
impl<'a, W, F> ser::Serializer for &'a mut Serializer<W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Compound<'a, W, F>;
    type SerializeTuple = Compound<'a, W, F>;
    type SerializeTupleStruct = Compound<'a, W, F>;
    type SerializeTupleVariant = Compound<'a, W, F>;
    type SerializeMap = Compound<'a, W, F>;
    type SerializeStruct = Compound<'a, W, F>;
    type SerializeStructVariant = Compound<'a, W, F>;
    #[inline]
    fn serialize_bool(self, value: bool) -> Result<()> {
        self.formatter.write_bool(&mut self.writer, value).map_err(Error::io)
    }
    #[inline]
    fn serialize_i8(self, value: i8) -> Result<()> {
        self.formatter.write_i8(&mut self.writer, value).map_err(Error::io)
    }
    #[inline]
    fn serialize_i16(self, value: i16) -> Result<()> {
        self.formatter.write_i16(&mut self.writer, value).map_err(Error::io)
    }
    #[inline]
    fn serialize_i32(self, value: i32) -> Result<()> {
        self.formatter.write_i32(&mut self.writer, value).map_err(Error::io)
    }
    #[inline]
    fn serialize_i64(self, value: i64) -> Result<()> {
        self.formatter.write_i64(&mut self.writer, value).map_err(Error::io)
    }
    fn serialize_i128(self, value: i128) -> Result<()> {
        self.formatter.write_i128(&mut self.writer, value).map_err(Error::io)
    }
    #[inline]
    fn serialize_u8(self, value: u8) -> Result<()> {
        self.formatter.write_u8(&mut self.writer, value).map_err(Error::io)
    }
    #[inline]
    fn serialize_u16(self, value: u16) -> Result<()> {
        self.formatter.write_u16(&mut self.writer, value).map_err(Error::io)
    }
    #[inline]
    fn serialize_u32(self, value: u32) -> Result<()> {
        self.formatter.write_u32(&mut self.writer, value).map_err(Error::io)
    }
    #[inline]
    fn serialize_u64(self, value: u64) -> Result<()> {
        self.formatter.write_u64(&mut self.writer, value).map_err(Error::io)
    }
    fn serialize_u128(self, value: u128) -> Result<()> {
        self.formatter.write_u128(&mut self.writer, value).map_err(Error::io)
    }
    #[inline]
    fn serialize_f32(self, value: f32) -> Result<()> {
        match value.classify() {
            FpCategory::Nan | FpCategory::Infinite => {
                self.formatter.write_null(&mut self.writer).map_err(Error::io)
            }
            _ => self.formatter.write_f32(&mut self.writer, value).map_err(Error::io),
        }
    }
    #[inline]
    fn serialize_f64(self, value: f64) -> Result<()> {
        match value.classify() {
            FpCategory::Nan | FpCategory::Infinite => {
                self.formatter.write_null(&mut self.writer).map_err(Error::io)
            }
            _ => self.formatter.write_f64(&mut self.writer, value).map_err(Error::io),
        }
    }
    #[inline]
    fn serialize_char(self, value: char) -> Result<()> {
        let mut buf = [0; 4];
        self.serialize_str(value.encode_utf8(&mut buf))
    }
    #[inline]
    fn serialize_str(self, value: &str) -> Result<()> {
        format_escaped_str(&mut self.writer, &mut self.formatter, value)
            .map_err(Error::io)
    }
    #[inline]
    fn serialize_bytes(self, value: &[u8]) -> Result<()> {
        use serde::ser::SerializeSeq;
        let mut seq = tri!(self.serialize_seq(Some(value.len())));
        for byte in value {
            tri!(seq.serialize_element(byte));
        }
        seq.end()
    }
    #[inline]
    fn serialize_unit(self) -> Result<()> {
        self.formatter.write_null(&mut self.writer).map_err(Error::io)
    }
    #[inline]
    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        self.serialize_unit()
    }
    #[inline]
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<()> {
        self.serialize_str(variant)
    }
    /// Serialize newtypes without an object wrapper.
    #[inline]
    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }
    #[inline]
    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        tri!(self.formatter.begin_object(& mut self.writer).map_err(Error::io));
        tri!(
            self.formatter.begin_object_key(& mut self.writer, true).map_err(Error::io)
        );
        tri!(self.serialize_str(variant));
        tri!(self.formatter.end_object_key(& mut self.writer).map_err(Error::io));
        tri!(self.formatter.begin_object_value(& mut self.writer).map_err(Error::io));
        tri!(value.serialize(& mut * self));
        tri!(self.formatter.end_object_value(& mut self.writer).map_err(Error::io));
        self.formatter.end_object(&mut self.writer).map_err(Error::io)
    }
    #[inline]
    fn serialize_none(self) -> Result<()> {
        self.serialize_unit()
    }
    #[inline]
    fn serialize_some<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }
    #[inline]
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        tri!(self.formatter.begin_array(& mut self.writer).map_err(Error::io));
        if len == Some(0) {
            tri!(self.formatter.end_array(& mut self.writer).map_err(Error::io));
            Ok(Compound::Map {
                ser: self,
                state: State::Empty,
            })
        } else {
            Ok(Compound::Map {
                ser: self,
                state: State::First,
            })
        }
    }
    #[inline]
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(len))
    }
    #[inline]
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.serialize_seq(Some(len))
    }
    #[inline]
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        tri!(self.formatter.begin_object(& mut self.writer).map_err(Error::io));
        tri!(
            self.formatter.begin_object_key(& mut self.writer, true).map_err(Error::io)
        );
        tri!(self.serialize_str(variant));
        tri!(self.formatter.end_object_key(& mut self.writer).map_err(Error::io));
        tri!(self.formatter.begin_object_value(& mut self.writer).map_err(Error::io));
        self.serialize_seq(Some(len))
    }
    #[inline]
    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        tri!(self.formatter.begin_object(& mut self.writer).map_err(Error::io));
        if len == Some(0) {
            tri!(self.formatter.end_object(& mut self.writer).map_err(Error::io));
            Ok(Compound::Map {
                ser: self,
                state: State::Empty,
            })
        } else {
            Ok(Compound::Map {
                ser: self,
                state: State::First,
            })
        }
    }
    #[inline]
    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct> {
        match name {
            #[cfg(feature = "arbitrary_precision")]
            crate::number::TOKEN => Ok(Compound::Number { ser: self }),
            #[cfg(feature = "raw_value")]
            crate::raw::TOKEN => Ok(Compound::RawValue { ser: self }),
            _ => self.serialize_map(Some(len)),
        }
    }
    #[inline]
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        tri!(self.formatter.begin_object(& mut self.writer).map_err(Error::io));
        tri!(
            self.formatter.begin_object_key(& mut self.writer, true).map_err(Error::io)
        );
        tri!(self.serialize_str(variant));
        tri!(self.formatter.end_object_key(& mut self.writer).map_err(Error::io));
        tri!(self.formatter.begin_object_value(& mut self.writer).map_err(Error::io));
        self.serialize_map(Some(len))
    }
    fn collect_str<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + Display,
    {
        use self::fmt::Write;
        struct Adapter<'ser, W: 'ser, F: 'ser> {
            writer: &'ser mut W,
            formatter: &'ser mut F,
            error: Option<io::Error>,
        }
        impl<'ser, W, F> Write for Adapter<'ser, W, F>
        where
            W: io::Write,
            F: Formatter,
        {
            fn write_str(&mut self, s: &str) -> fmt::Result {
                debug_assert!(self.error.is_none());
                match format_escaped_str_contents(self.writer, self.formatter, s) {
                    Ok(()) => Ok(()),
                    Err(err) => {
                        self.error = Some(err);
                        Err(fmt::Error)
                    }
                }
            }
        }
        tri!(self.formatter.begin_string(& mut self.writer).map_err(Error::io));
        {
            let mut adapter = Adapter {
                writer: &mut self.writer,
                formatter: &mut self.formatter,
                error: None,
            };
            match write!(adapter, "{}", value) {
                Ok(()) => debug_assert!(adapter.error.is_none()),
                Err(fmt::Error) => {
                    return Err(
                        Error::io(adapter.error.expect("there should be an error")),
                    );
                }
            }
        }
        self.formatter.end_string(&mut self.writer).map_err(Error::io)
    }
}
#[doc(hidden)]
#[derive(Eq, PartialEq)]
pub enum State {
    Empty,
    First,
    Rest,
}
#[doc(hidden)]
pub enum Compound<'a, W: 'a, F: 'a> {
    Map { ser: &'a mut Serializer<W, F>, state: State },
    #[cfg(feature = "arbitrary_precision")]
    Number { ser: &'a mut Serializer<W, F> },
    #[cfg(feature = "raw_value")]
    RawValue { ser: &'a mut Serializer<W, F> },
}
impl<'a, W, F> ser::SerializeSeq for Compound<'a, W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = Error;
    #[inline]
    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        match self {
            Compound::Map { ser, state } => {
                tri!(
                    ser.formatter.begin_array_value(& mut ser.writer, * state ==
                    State::First).map_err(Error::io)
                );
                *state = State::Rest;
                tri!(value.serialize(& mut ** ser));
                ser.formatter.end_array_value(&mut ser.writer).map_err(Error::io)
            }
            #[cfg(feature = "arbitrary_precision")]
            Compound::Number { .. } => unreachable!(),
            #[cfg(feature = "raw_value")]
            Compound::RawValue { .. } => unreachable!(),
        }
    }
    #[inline]
    fn end(self) -> Result<()> {
        match self {
            Compound::Map { ser, state } => {
                match state {
                    State::Empty => Ok(()),
                    _ => ser.formatter.end_array(&mut ser.writer).map_err(Error::io),
                }
            }
            #[cfg(feature = "arbitrary_precision")]
            Compound::Number { .. } => unreachable!(),
            #[cfg(feature = "raw_value")]
            Compound::RawValue { .. } => unreachable!(),
        }
    }
}
impl<'a, W, F> ser::SerializeTuple for Compound<'a, W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = Error;
    #[inline]
    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }
    #[inline]
    fn end(self) -> Result<()> {
        ser::SerializeSeq::end(self)
    }
}
impl<'a, W, F> ser::SerializeTupleStruct for Compound<'a, W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = Error;
    #[inline]
    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }
    #[inline]
    fn end(self) -> Result<()> {
        ser::SerializeSeq::end(self)
    }
}
impl<'a, W, F> ser::SerializeTupleVariant for Compound<'a, W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = Error;
    #[inline]
    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }
    #[inline]
    fn end(self) -> Result<()> {
        match self {
            Compound::Map { ser, state } => {
                match state {
                    State::Empty => {}
                    _ => {
                        tri!(
                            ser.formatter.end_array(& mut ser.writer).map_err(Error::io)
                        )
                    }
                }
                tri!(
                    ser.formatter.end_object_value(& mut ser.writer).map_err(Error::io)
                );
                ser.formatter.end_object(&mut ser.writer).map_err(Error::io)
            }
            #[cfg(feature = "arbitrary_precision")]
            Compound::Number { .. } => unreachable!(),
            #[cfg(feature = "raw_value")]
            Compound::RawValue { .. } => unreachable!(),
        }
    }
}
impl<'a, W, F> ser::SerializeMap for Compound<'a, W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = Error;
    #[inline]
    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        match self {
            Compound::Map { ser, state } => {
                tri!(
                    ser.formatter.begin_object_key(& mut ser.writer, * state ==
                    State::First).map_err(Error::io)
                );
                *state = State::Rest;
                tri!(key.serialize(MapKeySerializer { ser : * ser }));
                ser.formatter.end_object_key(&mut ser.writer).map_err(Error::io)
            }
            #[cfg(feature = "arbitrary_precision")]
            Compound::Number { .. } => unreachable!(),
            #[cfg(feature = "raw_value")]
            Compound::RawValue { .. } => unreachable!(),
        }
    }
    #[inline]
    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        match self {
            Compound::Map { ser, .. } => {
                tri!(
                    ser.formatter.begin_object_value(& mut ser.writer).map_err(Error::io)
                );
                tri!(value.serialize(& mut ** ser));
                ser.formatter.end_object_value(&mut ser.writer).map_err(Error::io)
            }
            #[cfg(feature = "arbitrary_precision")]
            Compound::Number { .. } => unreachable!(),
            #[cfg(feature = "raw_value")]
            Compound::RawValue { .. } => unreachable!(),
        }
    }
    #[inline]
    fn end(self) -> Result<()> {
        match self {
            Compound::Map { ser, state } => {
                match state {
                    State::Empty => Ok(()),
                    _ => ser.formatter.end_object(&mut ser.writer).map_err(Error::io),
                }
            }
            #[cfg(feature = "arbitrary_precision")]
            Compound::Number { .. } => unreachable!(),
            #[cfg(feature = "raw_value")]
            Compound::RawValue { .. } => unreachable!(),
        }
    }
}
impl<'a, W, F> ser::SerializeStruct for Compound<'a, W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = Error;
    #[inline]
    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        match self {
            Compound::Map { .. } => ser::SerializeMap::serialize_entry(self, key, value),
            #[cfg(feature = "arbitrary_precision")]
            Compound::Number { ser, .. } => {
                if key == crate::number::TOKEN {
                    value.serialize(NumberStrEmitter(ser))
                } else {
                    Err(invalid_number())
                }
            }
            #[cfg(feature = "raw_value")]
            Compound::RawValue { ser, .. } => {
                if key == crate::raw::TOKEN {
                    value.serialize(RawValueStrEmitter(ser))
                } else {
                    Err(invalid_raw_value())
                }
            }
        }
    }
    #[inline]
    fn end(self) -> Result<()> {
        match self {
            Compound::Map { .. } => ser::SerializeMap::end(self),
            #[cfg(feature = "arbitrary_precision")]
            Compound::Number { .. } => Ok(()),
            #[cfg(feature = "raw_value")]
            Compound::RawValue { .. } => Ok(()),
        }
    }
}
impl<'a, W, F> ser::SerializeStructVariant for Compound<'a, W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = Error;
    #[inline]
    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        match *self {
            Compound::Map { .. } => {
                ser::SerializeStruct::serialize_field(self, key, value)
            }
            #[cfg(feature = "arbitrary_precision")]
            Compound::Number { .. } => unreachable!(),
            #[cfg(feature = "raw_value")]
            Compound::RawValue { .. } => unreachable!(),
        }
    }
    #[inline]
    fn end(self) -> Result<()> {
        match self {
            Compound::Map { ser, state } => {
                match state {
                    State::Empty => {}
                    _ => {
                        tri!(
                            ser.formatter.end_object(& mut ser.writer).map_err(Error::io)
                        )
                    }
                }
                tri!(
                    ser.formatter.end_object_value(& mut ser.writer).map_err(Error::io)
                );
                ser.formatter.end_object(&mut ser.writer).map_err(Error::io)
            }
            #[cfg(feature = "arbitrary_precision")]
            Compound::Number { .. } => unreachable!(),
            #[cfg(feature = "raw_value")]
            Compound::RawValue { .. } => unreachable!(),
        }
    }
}
struct MapKeySerializer<'a, W: 'a, F: 'a> {
    ser: &'a mut Serializer<W, F>,
}
#[cfg(feature = "arbitrary_precision")]
fn invalid_number() -> Error {
    Error::syntax(ErrorCode::InvalidNumber, 0, 0)
}
#[cfg(feature = "raw_value")]
fn invalid_raw_value() -> Error {
    Error::syntax(ErrorCode::ExpectedSomeValue, 0, 0)
}
fn key_must_be_a_string() -> Error {
    Error::syntax(ErrorCode::KeyMustBeAString, 0, 0)
}
impl<'a, W, F> ser::Serializer for MapKeySerializer<'a, W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = Error;
    #[inline]
    fn serialize_str(self, value: &str) -> Result<()> {
        self.ser.serialize_str(value)
    }
    #[inline]
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<()> {
        self.ser.serialize_str(variant)
    }
    #[inline]
    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }
    type SerializeSeq = Impossible<(), Error>;
    type SerializeTuple = Impossible<(), Error>;
    type SerializeTupleStruct = Impossible<(), Error>;
    type SerializeTupleVariant = Impossible<(), Error>;
    type SerializeMap = Impossible<(), Error>;
    type SerializeStruct = Impossible<(), Error>;
    type SerializeStructVariant = Impossible<(), Error>;
    fn serialize_bool(self, _value: bool) -> Result<()> {
        Err(key_must_be_a_string())
    }
    fn serialize_i8(self, value: i8) -> Result<()> {
        tri!(self.ser.formatter.begin_string(& mut self.ser.writer).map_err(Error::io));
        tri!(
            self.ser.formatter.write_i8(& mut self.ser.writer, value).map_err(Error::io)
        );
        self.ser.formatter.end_string(&mut self.ser.writer).map_err(Error::io)
    }
    fn serialize_i16(self, value: i16) -> Result<()> {
        tri!(self.ser.formatter.begin_string(& mut self.ser.writer).map_err(Error::io));
        tri!(
            self.ser.formatter.write_i16(& mut self.ser.writer, value).map_err(Error::io)
        );
        self.ser.formatter.end_string(&mut self.ser.writer).map_err(Error::io)
    }
    fn serialize_i32(self, value: i32) -> Result<()> {
        tri!(self.ser.formatter.begin_string(& mut self.ser.writer).map_err(Error::io));
        tri!(
            self.ser.formatter.write_i32(& mut self.ser.writer, value).map_err(Error::io)
        );
        self.ser.formatter.end_string(&mut self.ser.writer).map_err(Error::io)
    }
    fn serialize_i64(self, value: i64) -> Result<()> {
        tri!(self.ser.formatter.begin_string(& mut self.ser.writer).map_err(Error::io));
        tri!(
            self.ser.formatter.write_i64(& mut self.ser.writer, value).map_err(Error::io)
        );
        self.ser.formatter.end_string(&mut self.ser.writer).map_err(Error::io)
    }
    fn serialize_i128(self, value: i128) -> Result<()> {
        tri!(self.ser.formatter.begin_string(& mut self.ser.writer).map_err(Error::io));
        tri!(
            self.ser.formatter.write_i128(& mut self.ser.writer, value)
            .map_err(Error::io)
        );
        self.ser.formatter.end_string(&mut self.ser.writer).map_err(Error::io)
    }
    fn serialize_u8(self, value: u8) -> Result<()> {
        tri!(self.ser.formatter.begin_string(& mut self.ser.writer).map_err(Error::io));
        tri!(
            self.ser.formatter.write_u8(& mut self.ser.writer, value).map_err(Error::io)
        );
        self.ser.formatter.end_string(&mut self.ser.writer).map_err(Error::io)
    }
    fn serialize_u16(self, value: u16) -> Result<()> {
        tri!(self.ser.formatter.begin_string(& mut self.ser.writer).map_err(Error::io));
        tri!(
            self.ser.formatter.write_u16(& mut self.ser.writer, value).map_err(Error::io)
        );
        self.ser.formatter.end_string(&mut self.ser.writer).map_err(Error::io)
    }
    fn serialize_u32(self, value: u32) -> Result<()> {
        tri!(self.ser.formatter.begin_string(& mut self.ser.writer).map_err(Error::io));
        tri!(
            self.ser.formatter.write_u32(& mut self.ser.writer, value).map_err(Error::io)
        );
        self.ser.formatter.end_string(&mut self.ser.writer).map_err(Error::io)
    }
    fn serialize_u64(self, value: u64) -> Result<()> {
        tri!(self.ser.formatter.begin_string(& mut self.ser.writer).map_err(Error::io));
        tri!(
            self.ser.formatter.write_u64(& mut self.ser.writer, value).map_err(Error::io)
        );
        self.ser.formatter.end_string(&mut self.ser.writer).map_err(Error::io)
    }
    fn serialize_u128(self, value: u128) -> Result<()> {
        tri!(self.ser.formatter.begin_string(& mut self.ser.writer).map_err(Error::io));
        tri!(
            self.ser.formatter.write_u128(& mut self.ser.writer, value)
            .map_err(Error::io)
        );
        self.ser.formatter.end_string(&mut self.ser.writer).map_err(Error::io)
    }
    fn serialize_f32(self, _value: f32) -> Result<()> {
        Err(key_must_be_a_string())
    }
    fn serialize_f64(self, _value: f64) -> Result<()> {
        Err(key_must_be_a_string())
    }
    fn serialize_char(self, value: char) -> Result<()> {
        self.ser.serialize_str(&value.to_string())
    }
    fn serialize_bytes(self, _value: &[u8]) -> Result<()> {
        Err(key_must_be_a_string())
    }
    fn serialize_unit(self) -> Result<()> {
        Err(key_must_be_a_string())
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        Err(key_must_be_a_string())
    }
    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(key_must_be_a_string())
    }
    fn serialize_none(self) -> Result<()> {
        Err(key_must_be_a_string())
    }
    fn serialize_some<T>(self, _value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(key_must_be_a_string())
    }
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Err(key_must_be_a_string())
    }
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Err(key_must_be_a_string())
    }
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Err(key_must_be_a_string())
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Err(key_must_be_a_string())
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Err(key_must_be_a_string())
    }
    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct> {
        Err(key_must_be_a_string())
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Err(key_must_be_a_string())
    }
    fn collect_str<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + Display,
    {
        self.ser.collect_str(value)
    }
}
#[cfg(feature = "arbitrary_precision")]
struct NumberStrEmitter<'a, W: 'a + io::Write, F: 'a + Formatter>(
    &'a mut Serializer<W, F>,
);
#[cfg(feature = "arbitrary_precision")]
impl<'a, W: io::Write, F: Formatter> ser::Serializer for NumberStrEmitter<'a, W, F> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Impossible<(), Error>;
    type SerializeTuple = Impossible<(), Error>;
    type SerializeTupleStruct = Impossible<(), Error>;
    type SerializeTupleVariant = Impossible<(), Error>;
    type SerializeMap = Impossible<(), Error>;
    type SerializeStruct = Impossible<(), Error>;
    type SerializeStructVariant = Impossible<(), Error>;
    fn serialize_bool(self, _v: bool) -> Result<()> {
        Err(invalid_number())
    }
    fn serialize_i8(self, _v: i8) -> Result<()> {
        Err(invalid_number())
    }
    fn serialize_i16(self, _v: i16) -> Result<()> {
        Err(invalid_number())
    }
    fn serialize_i32(self, _v: i32) -> Result<()> {
        Err(invalid_number())
    }
    fn serialize_i64(self, _v: i64) -> Result<()> {
        Err(invalid_number())
    }
    fn serialize_i128(self, _v: i128) -> Result<()> {
        Err(invalid_number())
    }
    fn serialize_u8(self, _v: u8) -> Result<()> {
        Err(invalid_number())
    }
    fn serialize_u16(self, _v: u16) -> Result<()> {
        Err(invalid_number())
    }
    fn serialize_u32(self, _v: u32) -> Result<()> {
        Err(invalid_number())
    }
    fn serialize_u64(self, _v: u64) -> Result<()> {
        Err(invalid_number())
    }
    fn serialize_u128(self, _v: u128) -> Result<()> {
        Err(invalid_number())
    }
    fn serialize_f32(self, _v: f32) -> Result<()> {
        Err(invalid_number())
    }
    fn serialize_f64(self, _v: f64) -> Result<()> {
        Err(invalid_number())
    }
    fn serialize_char(self, _v: char) -> Result<()> {
        Err(invalid_number())
    }
    fn serialize_str(self, value: &str) -> Result<()> {
        let NumberStrEmitter(serializer) = self;
        serializer
            .formatter
            .write_number_str(&mut serializer.writer, value)
            .map_err(Error::io)
    }
    fn serialize_bytes(self, _value: &[u8]) -> Result<()> {
        Err(invalid_number())
    }
    fn serialize_none(self) -> Result<()> {
        Err(invalid_number())
    }
    fn serialize_some<T>(self, _value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(invalid_number())
    }
    fn serialize_unit(self) -> Result<()> {
        Err(invalid_number())
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        Err(invalid_number())
    }
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<()> {
        Err(invalid_number())
    }
    fn serialize_newtype_struct<T>(self, _name: &'static str, _value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(invalid_number())
    }
    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(invalid_number())
    }
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Err(invalid_number())
    }
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Err(invalid_number())
    }
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Err(invalid_number())
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Err(invalid_number())
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Err(invalid_number())
    }
    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct> {
        Err(invalid_number())
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Err(invalid_number())
    }
}
#[cfg(feature = "raw_value")]
struct RawValueStrEmitter<'a, W: 'a + io::Write, F: 'a + Formatter>(
    &'a mut Serializer<W, F>,
);
#[cfg(feature = "raw_value")]
impl<'a, W: io::Write, F: Formatter> ser::Serializer for RawValueStrEmitter<'a, W, F> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Impossible<(), Error>;
    type SerializeTuple = Impossible<(), Error>;
    type SerializeTupleStruct = Impossible<(), Error>;
    type SerializeTupleVariant = Impossible<(), Error>;
    type SerializeMap = Impossible<(), Error>;
    type SerializeStruct = Impossible<(), Error>;
    type SerializeStructVariant = Impossible<(), Error>;
    fn serialize_bool(self, _v: bool) -> Result<()> {
        Err(ser::Error::custom("expected RawValue"))
    }
    fn serialize_i8(self, _v: i8) -> Result<()> {
        Err(ser::Error::custom("expected RawValue"))
    }
    fn serialize_i16(self, _v: i16) -> Result<()> {
        Err(ser::Error::custom("expected RawValue"))
    }
    fn serialize_i32(self, _v: i32) -> Result<()> {
        Err(ser::Error::custom("expected RawValue"))
    }
    fn serialize_i64(self, _v: i64) -> Result<()> {
        Err(ser::Error::custom("expected RawValue"))
    }
    fn serialize_i128(self, _v: i128) -> Result<()> {
        Err(ser::Error::custom("expected RawValue"))
    }
    fn serialize_u8(self, _v: u8) -> Result<()> {
        Err(ser::Error::custom("expected RawValue"))
    }
    fn serialize_u16(self, _v: u16) -> Result<()> {
        Err(ser::Error::custom("expected RawValue"))
    }
    fn serialize_u32(self, _v: u32) -> Result<()> {
        Err(ser::Error::custom("expected RawValue"))
    }
    fn serialize_u64(self, _v: u64) -> Result<()> {
        Err(ser::Error::custom("expected RawValue"))
    }
    fn serialize_u128(self, _v: u128) -> Result<()> {
        Err(ser::Error::custom("expected RawValue"))
    }
    fn serialize_f32(self, _v: f32) -> Result<()> {
        Err(ser::Error::custom("expected RawValue"))
    }
    fn serialize_f64(self, _v: f64) -> Result<()> {
        Err(ser::Error::custom("expected RawValue"))
    }
    fn serialize_char(self, _v: char) -> Result<()> {
        Err(ser::Error::custom("expected RawValue"))
    }
    fn serialize_str(self, value: &str) -> Result<()> {
        let RawValueStrEmitter(serializer) = self;
        serializer
            .formatter
            .write_raw_fragment(&mut serializer.writer, value)
            .map_err(Error::io)
    }
    fn serialize_bytes(self, _value: &[u8]) -> Result<()> {
        Err(ser::Error::custom("expected RawValue"))
    }
    fn serialize_none(self) -> Result<()> {
        Err(ser::Error::custom("expected RawValue"))
    }
    fn serialize_some<T>(self, _value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(ser::Error::custom("expected RawValue"))
    }
    fn serialize_unit(self) -> Result<()> {
        Err(ser::Error::custom("expected RawValue"))
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        Err(ser::Error::custom("expected RawValue"))
    }
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<()> {
        Err(ser::Error::custom("expected RawValue"))
    }
    fn serialize_newtype_struct<T>(self, _name: &'static str, _value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(ser::Error::custom("expected RawValue"))
    }
    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(ser::Error::custom("expected RawValue"))
    }
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Err(ser::Error::custom("expected RawValue"))
    }
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Err(ser::Error::custom("expected RawValue"))
    }
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Err(ser::Error::custom("expected RawValue"))
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Err(ser::Error::custom("expected RawValue"))
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Err(ser::Error::custom("expected RawValue"))
    }
    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct> {
        Err(ser::Error::custom("expected RawValue"))
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Err(ser::Error::custom("expected RawValue"))
    }
    fn collect_str<T>(self, value: &T) -> Result<Self::Ok>
    where
        T: ?Sized + Display,
    {
        self.serialize_str(&value.to_string())
    }
}
/// Represents a character escape code in a type-safe manner.
pub enum CharEscape {
    /// An escaped quote `"`
    Quote,
    /// An escaped reverse solidus `\`
    ReverseSolidus,
    /// An escaped solidus `/`
    Solidus,
    /// An escaped backspace character (usually escaped as `\b`)
    Backspace,
    /// An escaped form feed character (usually escaped as `\f`)
    FormFeed,
    /// An escaped line feed character (usually escaped as `\n`)
    LineFeed,
    /// An escaped carriage return character (usually escaped as `\r`)
    CarriageReturn,
    /// An escaped tab character (usually escaped as `\t`)
    Tab,
    /// An escaped ASCII plane control character (usually escaped as
    /// `\u00XX` where `XX` are two hex characters)
    AsciiControl(u8),
}
impl CharEscape {
    #[inline]
    fn from_escape_table(escape: u8, byte: u8) -> CharEscape {
        match escape {
            self::BB => CharEscape::Backspace,
            self::TT => CharEscape::Tab,
            self::NN => CharEscape::LineFeed,
            self::FF => CharEscape::FormFeed,
            self::RR => CharEscape::CarriageReturn,
            self::QU => CharEscape::Quote,
            self::BS => CharEscape::ReverseSolidus,
            self::UU => CharEscape::AsciiControl(byte),
            _ => unreachable!(),
        }
    }
}
/// This trait abstracts away serializing the JSON control characters, which allows the user to
/// optionally pretty print the JSON output.
pub trait Formatter {
    /// Writes a `null` value to the specified writer.
    #[inline]
    fn write_null<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(b"null")
    }
    /// Writes a `true` or `false` value to the specified writer.
    #[inline]
    fn write_bool<W>(&mut self, writer: &mut W, value: bool) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        let s = if value { b"true" as &[u8] } else { b"false" as &[u8] };
        writer.write_all(s)
    }
    /// Writes an integer value like `-123` to the specified writer.
    #[inline]
    fn write_i8<W>(&mut self, writer: &mut W, value: i8) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        let mut buffer = itoa::Buffer::new();
        let s = buffer.format(value);
        writer.write_all(s.as_bytes())
    }
    /// Writes an integer value like `-123` to the specified writer.
    #[inline]
    fn write_i16<W>(&mut self, writer: &mut W, value: i16) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        let mut buffer = itoa::Buffer::new();
        let s = buffer.format(value);
        writer.write_all(s.as_bytes())
    }
    /// Writes an integer value like `-123` to the specified writer.
    #[inline]
    fn write_i32<W>(&mut self, writer: &mut W, value: i32) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        let mut buffer = itoa::Buffer::new();
        let s = buffer.format(value);
        writer.write_all(s.as_bytes())
    }
    /// Writes an integer value like `-123` to the specified writer.
    #[inline]
    fn write_i64<W>(&mut self, writer: &mut W, value: i64) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        let mut buffer = itoa::Buffer::new();
        let s = buffer.format(value);
        writer.write_all(s.as_bytes())
    }
    /// Writes an integer value like `-123` to the specified writer.
    #[inline]
    fn write_i128<W>(&mut self, writer: &mut W, value: i128) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        let mut buffer = itoa::Buffer::new();
        let s = buffer.format(value);
        writer.write_all(s.as_bytes())
    }
    /// Writes an integer value like `123` to the specified writer.
    #[inline]
    fn write_u8<W>(&mut self, writer: &mut W, value: u8) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        let mut buffer = itoa::Buffer::new();
        let s = buffer.format(value);
        writer.write_all(s.as_bytes())
    }
    /// Writes an integer value like `123` to the specified writer.
    #[inline]
    fn write_u16<W>(&mut self, writer: &mut W, value: u16) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        let mut buffer = itoa::Buffer::new();
        let s = buffer.format(value);
        writer.write_all(s.as_bytes())
    }
    /// Writes an integer value like `123` to the specified writer.
    #[inline]
    fn write_u32<W>(&mut self, writer: &mut W, value: u32) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        let mut buffer = itoa::Buffer::new();
        let s = buffer.format(value);
        writer.write_all(s.as_bytes())
    }
    /// Writes an integer value like `123` to the specified writer.
    #[inline]
    fn write_u64<W>(&mut self, writer: &mut W, value: u64) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        let mut buffer = itoa::Buffer::new();
        let s = buffer.format(value);
        writer.write_all(s.as_bytes())
    }
    /// Writes an integer value like `123` to the specified writer.
    #[inline]
    fn write_u128<W>(&mut self, writer: &mut W, value: u128) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        let mut buffer = itoa::Buffer::new();
        let s = buffer.format(value);
        writer.write_all(s.as_bytes())
    }
    /// Writes a floating point value like `-31.26e+12` to the specified writer.
    #[inline]
    fn write_f32<W>(&mut self, writer: &mut W, value: f32) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        let mut buffer = ryu::Buffer::new();
        let s = buffer.format_finite(value);
        writer.write_all(s.as_bytes())
    }
    /// Writes a floating point value like `-31.26e+12` to the specified writer.
    #[inline]
    fn write_f64<W>(&mut self, writer: &mut W, value: f64) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        let mut buffer = ryu::Buffer::new();
        let s = buffer.format_finite(value);
        writer.write_all(s.as_bytes())
    }
    /// Writes a number that has already been rendered to a string.
    #[inline]
    fn write_number_str<W>(&mut self, writer: &mut W, value: &str) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(value.as_bytes())
    }
    /// Called before each series of `write_string_fragment` and
    /// `write_char_escape`.  Writes a `"` to the specified writer.
    #[inline]
    fn begin_string<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(b"\"")
    }
    /// Called after each series of `write_string_fragment` and
    /// `write_char_escape`.  Writes a `"` to the specified writer.
    #[inline]
    fn end_string<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(b"\"")
    }
    /// Writes a string fragment that doesn't need any escaping to the
    /// specified writer.
    #[inline]
    fn write_string_fragment<W>(
        &mut self,
        writer: &mut W,
        fragment: &str,
    ) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(fragment.as_bytes())
    }
    /// Writes a character escape code to the specified writer.
    #[inline]
    fn write_char_escape<W>(
        &mut self,
        writer: &mut W,
        char_escape: CharEscape,
    ) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        use self::CharEscape::*;
        let s = match char_escape {
            Quote => b"\\\"",
            ReverseSolidus => b"\\\\",
            Solidus => b"\\/",
            Backspace => b"\\b",
            FormFeed => b"\\f",
            LineFeed => b"\\n",
            CarriageReturn => b"\\r",
            Tab => b"\\t",
            AsciiControl(byte) => {
                static HEX_DIGITS: [u8; 16] = *b"0123456789abcdef";
                let bytes = &[
                    b'\\',
                    b'u',
                    b'0',
                    b'0',
                    HEX_DIGITS[(byte >> 4) as usize],
                    HEX_DIGITS[(byte & 0xF) as usize],
                ];
                return writer.write_all(bytes);
            }
        };
        writer.write_all(s)
    }
    /// Called before every array.  Writes a `[` to the specified
    /// writer.
    #[inline]
    fn begin_array<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(b"[")
    }
    /// Called after every array.  Writes a `]` to the specified
    /// writer.
    #[inline]
    fn end_array<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(b"]")
    }
    /// Called before every array value.  Writes a `,` if needed to
    /// the specified writer.
    #[inline]
    fn begin_array_value<W>(&mut self, writer: &mut W, first: bool) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        if first { Ok(()) } else { writer.write_all(b",") }
    }
    /// Called after every array value.
    #[inline]
    fn end_array_value<W>(&mut self, _writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        Ok(())
    }
    /// Called before every object.  Writes a `{` to the specified
    /// writer.
    #[inline]
    fn begin_object<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(b"{")
    }
    /// Called after every object.  Writes a `}` to the specified
    /// writer.
    #[inline]
    fn end_object<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(b"}")
    }
    /// Called before every object key.
    #[inline]
    fn begin_object_key<W>(&mut self, writer: &mut W, first: bool) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        if first { Ok(()) } else { writer.write_all(b",") }
    }
    /// Called after every object key.  A `:` should be written to the
    /// specified writer by either this method or
    /// `begin_object_value`.
    #[inline]
    fn end_object_key<W>(&mut self, _writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        Ok(())
    }
    /// Called before every object value.  A `:` should be written to
    /// the specified writer by either this method or
    /// `end_object_key`.
    #[inline]
    fn begin_object_value<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(b":")
    }
    /// Called after every object value.
    #[inline]
    fn end_object_value<W>(&mut self, _writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        Ok(())
    }
    /// Writes a raw JSON fragment that doesn't need any escaping to the
    /// specified writer.
    #[inline]
    fn write_raw_fragment<W>(&mut self, writer: &mut W, fragment: &str) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(fragment.as_bytes())
    }
}
/// This structure compacts a JSON value with no extra whitespace.
#[derive(Clone, Debug)]
pub struct CompactFormatter;
impl Formatter for CompactFormatter {}
/// This structure pretty prints a JSON value to make it human readable.
#[derive(Clone, Debug)]
pub struct PrettyFormatter<'a> {
    current_indent: usize,
    has_value: bool,
    indent: &'a [u8],
}
impl<'a> PrettyFormatter<'a> {
    /// Construct a pretty printer formatter that defaults to using two spaces for indentation.
    pub fn new() -> Self {
        PrettyFormatter::with_indent(b"  ")
    }
    /// Construct a pretty printer formatter that uses the `indent` string for indentation.
    pub fn with_indent(indent: &'a [u8]) -> Self {
        PrettyFormatter {
            current_indent: 0,
            has_value: false,
            indent,
        }
    }
}
impl<'a> Default for PrettyFormatter<'a> {
    fn default() -> Self {
        PrettyFormatter::new()
    }
}
impl<'a> Formatter for PrettyFormatter<'a> {
    #[inline]
    fn begin_array<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.current_indent += 1;
        self.has_value = false;
        writer.write_all(b"[")
    }
    #[inline]
    fn end_array<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.current_indent -= 1;
        if self.has_value {
            tri!(writer.write_all(b"\n"));
            tri!(indent(writer, self.current_indent, self.indent));
        }
        writer.write_all(b"]")
    }
    #[inline]
    fn begin_array_value<W>(&mut self, writer: &mut W, first: bool) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        tri!(writer.write_all(if first { b"\n" } else { b",\n" }));
        indent(writer, self.current_indent, self.indent)
    }
    #[inline]
    fn end_array_value<W>(&mut self, _writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.has_value = true;
        Ok(())
    }
    #[inline]
    fn begin_object<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.current_indent += 1;
        self.has_value = false;
        writer.write_all(b"{")
    }
    #[inline]
    fn end_object<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.current_indent -= 1;
        if self.has_value {
            tri!(writer.write_all(b"\n"));
            tri!(indent(writer, self.current_indent, self.indent));
        }
        writer.write_all(b"}")
    }
    #[inline]
    fn begin_object_key<W>(&mut self, writer: &mut W, first: bool) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        tri!(writer.write_all(if first { b"\n" } else { b",\n" }));
        indent(writer, self.current_indent, self.indent)
    }
    #[inline]
    fn begin_object_value<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(b": ")
    }
    #[inline]
    fn end_object_value<W>(&mut self, _writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.has_value = true;
        Ok(())
    }
}
fn format_escaped_str<W, F>(
    writer: &mut W,
    formatter: &mut F,
    value: &str,
) -> io::Result<()>
where
    W: ?Sized + io::Write,
    F: ?Sized + Formatter,
{
    tri!(formatter.begin_string(writer));
    tri!(format_escaped_str_contents(writer, formatter, value));
    formatter.end_string(writer)
}
fn format_escaped_str_contents<W, F>(
    writer: &mut W,
    formatter: &mut F,
    value: &str,
) -> io::Result<()>
where
    W: ?Sized + io::Write,
    F: ?Sized + Formatter,
{
    let bytes = value.as_bytes();
    let mut start = 0;
    for (i, &byte) in bytes.iter().enumerate() {
        let escape = ESCAPE[byte as usize];
        if escape == 0 {
            continue;
        }
        if start < i {
            tri!(formatter.write_string_fragment(writer, & value[start..i]));
        }
        let char_escape = CharEscape::from_escape_table(escape, byte);
        tri!(formatter.write_char_escape(writer, char_escape));
        start = i + 1;
    }
    if start == bytes.len() {
        return Ok(());
    }
    formatter.write_string_fragment(writer, &value[start..])
}
const BB: u8 = b'b';
const TT: u8 = b't';
const NN: u8 = b'n';
const FF: u8 = b'f';
const RR: u8 = b'r';
const QU: u8 = b'"';
const BS: u8 = b'\\';
const UU: u8 = b'u';
const __: u8 = 0;
static ESCAPE: [u8; 256] = [
    UU,
    UU,
    UU,
    UU,
    UU,
    UU,
    UU,
    UU,
    BB,
    TT,
    NN,
    UU,
    FF,
    RR,
    UU,
    UU,
    UU,
    UU,
    UU,
    UU,
    UU,
    UU,
    UU,
    UU,
    UU,
    UU,
    UU,
    UU,
    UU,
    UU,
    UU,
    UU,
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
];
/// Serialize the given data structure as JSON into the IO stream.
///
/// Serialization guarantees it only feeds valid UTF-8 sequences to the writer.
///
/// # Errors
///
/// Serialization can fail if `T`'s implementation of `Serialize` decides to
/// fail, or if `T` contains a map with non-string keys.
#[inline]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub fn to_writer<W, T>(writer: W, value: &T) -> Result<()>
where
    W: io::Write,
    T: ?Sized + Serialize,
{
    let mut ser = Serializer::new(writer);
    value.serialize(&mut ser)
}
/// Serialize the given data structure as pretty-printed JSON into the IO
/// stream.
///
/// Serialization guarantees it only feeds valid UTF-8 sequences to the writer.
///
/// # Errors
///
/// Serialization can fail if `T`'s implementation of `Serialize` decides to
/// fail, or if `T` contains a map with non-string keys.
#[inline]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub fn to_writer_pretty<W, T>(writer: W, value: &T) -> Result<()>
where
    W: io::Write,
    T: ?Sized + Serialize,
{
    let mut ser = Serializer::pretty(writer);
    value.serialize(&mut ser)
}
/// Serialize the given data structure as a JSON byte vector.
///
/// # Errors
///
/// Serialization can fail if `T`'s implementation of `Serialize` decides to
/// fail, or if `T` contains a map with non-string keys.
#[inline]
pub fn to_vec<T>(value: &T) -> Result<Vec<u8>>
where
    T: ?Sized + Serialize,
{
    let mut writer = Vec::with_capacity(128);
    tri!(to_writer(& mut writer, value));
    Ok(writer)
}
/// Serialize the given data structure as a pretty-printed JSON byte vector.
///
/// # Errors
///
/// Serialization can fail if `T`'s implementation of `Serialize` decides to
/// fail, or if `T` contains a map with non-string keys.
#[inline]
pub fn to_vec_pretty<T>(value: &T) -> Result<Vec<u8>>
where
    T: ?Sized + Serialize,
{
    let mut writer = Vec::with_capacity(128);
    tri!(to_writer_pretty(& mut writer, value));
    Ok(writer)
}
/// Serialize the given data structure as a String of JSON.
///
/// # Errors
///
/// Serialization can fail if `T`'s implementation of `Serialize` decides to
/// fail, or if `T` contains a map with non-string keys.
#[inline]
pub fn to_string<T>(value: &T) -> Result<String>
where
    T: ?Sized + Serialize,
{
    let vec = tri!(to_vec(value));
    let string = unsafe { String::from_utf8_unchecked(vec) };
    Ok(string)
}
/// Serialize the given data structure as a pretty-printed String of JSON.
///
/// # Errors
///
/// Serialization can fail if `T`'s implementation of `Serialize` decides to
/// fail, or if `T` contains a map with non-string keys.
#[inline]
pub fn to_string_pretty<T>(value: &T) -> Result<String>
where
    T: ?Sized + Serialize,
{
    let vec = tri!(to_vec_pretty(value));
    let string = unsafe { String::from_utf8_unchecked(vec) };
    Ok(string)
}
fn indent<W>(wr: &mut W, n: usize, s: &[u8]) -> io::Result<()>
where
    W: ?Sized + io::Write,
{
    for _ in 0..n {
        tri!(wr.write_all(s));
    }
    Ok(())
}
#[cfg(test)]
mod tests_rug_135 {
    use super::*;
    use crate::{json, Map, Value};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_135_rrrruuuugggg_test_rug = 0;
        let mut v31 = Value::default();
        let p0: &Value = &v31;
        crate::ser::to_vec(p0).unwrap();
        let _rug_ed_tests_rug_135_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_136 {
    use super::*;
    use crate::{Map, Value};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_136_rrrruuuugggg_test_rug = 0;
        let mut p0: Map<String, Value> = Map::new();
        crate::ser::to_vec_pretty(&p0).unwrap();
        let _rug_ed_tests_rug_136_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_137 {
    use super::*;
    use crate::value::Number;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Number = Number::from_f32(rug_fuzz_0).unwrap().into();
        crate::ser::to_string(&p0).unwrap();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_138 {
    use super::*;
    use crate::{to_string_pretty, map::Map, Value};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_138_rrrruuuugggg_test_rug = 0;
        let mut v37: Map<String, Value> = Map::new();
        let p0: &Map<String, Value> = &v37;
        to_string_pretty(p0).unwrap();
        let _rug_ed_tests_rug_138_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_139 {
    use super::*;
    use crate::value::{Value, self};
    use std::io::{self, Write};
    #[test]
    fn test_indent() {
        let _rug_st_tests_rug_139_rrrruuuugggg_test_indent = 0;
        let rug_fuzz_0 = 4;
        let rug_fuzz_1 = b"\t";
        let mut buf: Vec<u8> = Vec::new();
        let n: usize = rug_fuzz_0;
        let s: &[u8] = rug_fuzz_1;
        indent(&mut buf, n, s).unwrap();
        let _rug_ed_tests_rug_139_rrrruuuugggg_test_indent = 0;
    }
}
#[cfg(test)]
mod tests_rug_142 {
    use super::*;
    use crate::ser::PrettyFormatter;
    use std::io;
    #[test]
    fn test_write_i8() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: PrettyFormatter = PrettyFormatter::default();
        let p1: &mut dyn io::Write = &mut io::stdout();
        let p2: i8 = rug_fuzz_0;
        crate::ser::Formatter::write_i8(&mut p0, p1, p2).unwrap();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_145 {
    use super::*;
    use crate::ser;
    use crate::value;
    use std::io;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: ser::PrettyFormatter = ser::PrettyFormatter::default();
        let p1: value::Value;
        let mut p2: i64 = rug_fuzz_0;
        ser::Formatter::write_i64(&mut p0, &mut io::stdout(), p2).unwrap();
             }
}
}
}    }
}
mod tests_rug_153 {
    use super::*;
    use crate::ser;
    use ryu::Buffer;
    use std::io::{self, Write};
    struct DummyWriter;
    impl Write for DummyWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    #[test]
    fn test_write_f64() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = ser::PrettyFormatter::default();
        let mut p1 = DummyWriter;
        let p2: f64 = rug_fuzz_0;
        crate::ser::Formatter::write_f64(&mut p0, &mut p1, p2).unwrap();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_154 {
    use super::*;
    use crate::ser::{Formatter, PrettyFormatter};
    use std::io::{self, Write};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: PrettyFormatter = PrettyFormatter::default();
        let mut p1: &mut dyn Write = &mut io::stdout();
        let p2: &str = rug_fuzz_0;
        p0.write_number_str(p1, &p2).unwrap();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_158 {
    use super::*;
    use crate::ser::{CompactFormatter, Formatter};
    use crate::ser::CharEscape;
    use std::io;
    #[test]
    fn test_write_char_escape() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = CompactFormatter;
        let mut p1 = &mut io::stdout();
        let p2 = CharEscape::AsciiControl(rug_fuzz_0);
        Formatter::write_char_escape(&mut p0, &mut p1, p2).unwrap();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_166 {
    use super::*;
    use crate::ser::CompactFormatter;
    use std::io;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_166_rrrruuuugggg_test_rug = 0;
        let mut p0 = CompactFormatter;
        let p1: &mut dyn io::Write = &mut io::stdout();
        p0.end_object_key(p1).unwrap();
        let _rug_ed_tests_rug_166_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_173 {
    use super::*;
    use crate::ser::Serializer;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_173_rrrruuuugggg_test_rug = 0;
        let mut p0: Serializer<Vec<u8>, _> = Serializer::new(Vec::new());
        crate::ser::Serializer::<Vec<u8>, _>::into_inner(p0);
        let _rug_ed_tests_rug_173_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_180 {
    use super::*;
    use crate::ser::Serializer;
    #[test]
    fn test_serialize_u8() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Serializer<Vec<u8>, _> = Serializer::new(Vec::new());
        let p1: u8 = rug_fuzz_0;
        <&mut Serializer<Vec<u8>, _> as serde::Serializer>::serialize_u8(&mut p0, p1)
            .unwrap();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_187 {
    use super::*;
    use crate::ser::Serializer;
    use serde::Serializer as _;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(char) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Serializer::new(Vec::new());
        let p1 = rug_fuzz_0;
        p0.serialize_char(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_202 {
    use super::*;
    use crate::{ser, Serializer};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_202_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "name";
        let rug_fuzz_1 = 10;
        let mut v39: Serializer<Vec<u8>, _> = Serializer::new(Vec::new());
        let p0 = &mut v39;
        let p1 = rug_fuzz_0;
        let p2 = rug_fuzz_1;
        <&mut ser::Serializer<
            Vec<u8>,
            _,
        > as serde::Serializer>::serialize_struct(p0, p1, p2)
            .unwrap();
        let _rug_ed_tests_rug_202_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_252 {
    use super::*;
    use crate::ser::CharEscape;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u8, u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: u8 = rug_fuzz_0;
        let mut p1: u8 = rug_fuzz_1;
        CharEscape::from_escape_table(p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_253 {
    use super::*;
    use crate::ser::PrettyFormatter;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_253_rrrruuuugggg_test_rug = 0;
        PrettyFormatter::<'static>::new();
        let _rug_ed_tests_rug_253_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_254 {
    use super::*;
    use crate::ser::PrettyFormatter;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_254_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"    ";
        let indent = rug_fuzz_0;
        PrettyFormatter::with_indent(indent);
        let _rug_ed_tests_rug_254_rrrruuuugggg_test_rug = 0;
    }
}
