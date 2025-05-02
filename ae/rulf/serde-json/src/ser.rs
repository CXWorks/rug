//! Serialize a Rust data structure into JSON data.
use crate::error::{Error, ErrorCode, Result};
use crate::io;
use crate::lib::num::FpCategory;
use crate::lib::*;
use serde::ser::{self, Impossible, Serialize};
use serde::serde_if_integer128;
/// A structure for serializing Rust values into JSON.
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
        tri!(self.formatter.write_bool(& mut self.writer, value).map_err(Error::io));
        Ok(())
    }
    #[inline]
    fn serialize_i8(self, value: i8) -> Result<()> {
        tri!(self.formatter.write_i8(& mut self.writer, value).map_err(Error::io));
        Ok(())
    }
    #[inline]
    fn serialize_i16(self, value: i16) -> Result<()> {
        tri!(self.formatter.write_i16(& mut self.writer, value).map_err(Error::io));
        Ok(())
    }
    #[inline]
    fn serialize_i32(self, value: i32) -> Result<()> {
        tri!(self.formatter.write_i32(& mut self.writer, value).map_err(Error::io));
        Ok(())
    }
    #[inline]
    fn serialize_i64(self, value: i64) -> Result<()> {
        tri!(self.formatter.write_i64(& mut self.writer, value).map_err(Error::io));
        Ok(())
    }
    serde_if_integer128! {
        fn serialize_i128(self, value : i128) -> Result < () > { self.formatter
        .write_number_str(& mut self.writer, & value.to_string()).map_err(Error::io) }
    }
    #[inline]
    fn serialize_u8(self, value: u8) -> Result<()> {
        tri!(self.formatter.write_u8(& mut self.writer, value).map_err(Error::io));
        Ok(())
    }
    #[inline]
    fn serialize_u16(self, value: u16) -> Result<()> {
        tri!(self.formatter.write_u16(& mut self.writer, value).map_err(Error::io));
        Ok(())
    }
    #[inline]
    fn serialize_u32(self, value: u32) -> Result<()> {
        tri!(self.formatter.write_u32(& mut self.writer, value).map_err(Error::io));
        Ok(())
    }
    #[inline]
    fn serialize_u64(self, value: u64) -> Result<()> {
        tri!(self.formatter.write_u64(& mut self.writer, value).map_err(Error::io));
        Ok(())
    }
    serde_if_integer128! {
        fn serialize_u128(self, value : u128) -> Result < () > { self.formatter
        .write_number_str(& mut self.writer, & value.to_string()).map_err(Error::io) }
    }
    #[inline]
    fn serialize_f32(self, value: f32) -> Result<()> {
        match value.classify() {
            FpCategory::Nan | FpCategory::Infinite => {
                tri!(self.formatter.write_null(& mut self.writer).map_err(Error::io));
            }
            _ => {
                tri!(
                    self.formatter.write_f32(& mut self.writer, value).map_err(Error::io)
                );
            }
        }
        Ok(())
    }
    #[inline]
    fn serialize_f64(self, value: f64) -> Result<()> {
        match value.classify() {
            FpCategory::Nan | FpCategory::Infinite => {
                tri!(self.formatter.write_null(& mut self.writer).map_err(Error::io));
            }
            _ => {
                tri!(
                    self.formatter.write_f64(& mut self.writer, value).map_err(Error::io)
                );
            }
        }
        Ok(())
    }
    #[inline]
    fn serialize_char(self, value: char) -> Result<()> {
        let mut buf = [0; 4];
        self.serialize_str(value.encode_utf8(&mut buf))
    }
    #[inline]
    fn serialize_str(self, value: &str) -> Result<()> {
        tri!(
            format_escaped_str(& mut self.writer, & mut self.formatter, value)
            .map_err(Error::io)
        );
        Ok(())
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
        tri!(self.formatter.write_null(& mut self.writer).map_err(Error::io));
        Ok(())
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
        tri!(self.formatter.end_object(& mut self.writer).map_err(Error::io));
        Ok(())
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
        if len == Some(0) {
            tri!(self.formatter.begin_array(& mut self.writer).map_err(Error::io));
            tri!(self.formatter.end_array(& mut self.writer).map_err(Error::io));
            Ok(Compound::Map {
                ser: self,
                state: State::Empty,
            })
        } else {
            tri!(self.formatter.begin_array(& mut self.writer).map_err(Error::io));
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
        if len == Some(0) {
            tri!(self.formatter.begin_object(& mut self.writer).map_err(Error::io));
            tri!(self.formatter.end_object(& mut self.writer).map_err(Error::io));
            Ok(Compound::Map {
                ser: self,
                state: State::Empty,
            })
        } else {
            tri!(self.formatter.begin_object(& mut self.writer).map_err(Error::io));
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
        tri!(self.formatter.end_string(& mut self.writer).map_err(Error::io));
        Ok(())
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
        match *self {
            Compound::Map { ref mut ser, ref mut state } => {
                tri!(
                    ser.formatter.begin_array_value(& mut ser.writer, * state ==
                    State::First).map_err(Error::io)
                );
                *state = State::Rest;
                tri!(value.serialize(& mut ** ser));
                tri!(ser.formatter.end_array_value(& mut ser.writer).map_err(Error::io));
                Ok(())
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
                            ser.formatter.end_array(& mut ser.writer).map_err(Error::io)
                        )
                    }
                }
                Ok(())
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
                tri!(ser.formatter.end_object(& mut ser.writer).map_err(Error::io));
                Ok(())
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
        match *self {
            Compound::Map { ref mut ser, ref mut state } => {
                tri!(
                    ser.formatter.begin_object_key(& mut ser.writer, * state ==
                    State::First).map_err(Error::io)
                );
                *state = State::Rest;
                tri!(key.serialize(MapKeySerializer { ser : * ser }));
                tri!(ser.formatter.end_object_key(& mut ser.writer).map_err(Error::io));
                Ok(())
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
        match *self {
            Compound::Map { ref mut ser, .. } => {
                tri!(
                    ser.formatter.begin_object_value(& mut ser.writer).map_err(Error::io)
                );
                tri!(value.serialize(& mut ** ser));
                tri!(
                    ser.formatter.end_object_value(& mut ser.writer).map_err(Error::io)
                );
                Ok(())
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
                Ok(())
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
        match *self {
            Compound::Map { .. } => ser::SerializeMap::serialize_entry(self, key, value),
            #[cfg(feature = "arbitrary_precision")]
            Compound::Number { ref mut ser, .. } => {
                if key == crate::number::TOKEN {
                    tri!(value.serialize(NumberStrEmitter(& mut * ser)));
                    Ok(())
                } else {
                    Err(invalid_number())
                }
            }
            #[cfg(feature = "raw_value")]
            Compound::RawValue { ref mut ser, .. } => {
                if key == crate::raw::TOKEN {
                    tri!(value.serialize(RawValueStrEmitter(& mut * ser)));
                    Ok(())
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
                tri!(ser.formatter.end_object(& mut ser.writer).map_err(Error::io));
                Ok(())
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
        tri!(self.ser.formatter.end_string(& mut self.ser.writer).map_err(Error::io));
        Ok(())
    }
    fn serialize_i16(self, value: i16) -> Result<()> {
        tri!(self.ser.formatter.begin_string(& mut self.ser.writer).map_err(Error::io));
        tri!(
            self.ser.formatter.write_i16(& mut self.ser.writer, value).map_err(Error::io)
        );
        tri!(self.ser.formatter.end_string(& mut self.ser.writer).map_err(Error::io));
        Ok(())
    }
    fn serialize_i32(self, value: i32) -> Result<()> {
        tri!(self.ser.formatter.begin_string(& mut self.ser.writer).map_err(Error::io));
        tri!(
            self.ser.formatter.write_i32(& mut self.ser.writer, value).map_err(Error::io)
        );
        tri!(self.ser.formatter.end_string(& mut self.ser.writer).map_err(Error::io));
        Ok(())
    }
    fn serialize_i64(self, value: i64) -> Result<()> {
        tri!(self.ser.formatter.begin_string(& mut self.ser.writer).map_err(Error::io));
        tri!(
            self.ser.formatter.write_i64(& mut self.ser.writer, value).map_err(Error::io)
        );
        tri!(self.ser.formatter.end_string(& mut self.ser.writer).map_err(Error::io));
        Ok(())
    }
    serde_if_integer128! {
        fn serialize_i128(self, value : i128) -> Result < () > { tri!(self.ser.formatter
        .begin_string(& mut self.ser.writer).map_err(Error::io)); tri!(self.ser.formatter
        .write_number_str(& mut self.ser.writer, & value.to_string())
        .map_err(Error::io)); tri!(self.ser.formatter.end_string(& mut self.ser.writer)
        .map_err(Error::io)); Ok(()) }
    }
    fn serialize_u8(self, value: u8) -> Result<()> {
        tri!(self.ser.formatter.begin_string(& mut self.ser.writer).map_err(Error::io));
        tri!(
            self.ser.formatter.write_u8(& mut self.ser.writer, value).map_err(Error::io)
        );
        tri!(self.ser.formatter.end_string(& mut self.ser.writer).map_err(Error::io));
        Ok(())
    }
    fn serialize_u16(self, value: u16) -> Result<()> {
        tri!(self.ser.formatter.begin_string(& mut self.ser.writer).map_err(Error::io));
        tri!(
            self.ser.formatter.write_u16(& mut self.ser.writer, value).map_err(Error::io)
        );
        tri!(self.ser.formatter.end_string(& mut self.ser.writer).map_err(Error::io));
        Ok(())
    }
    fn serialize_u32(self, value: u32) -> Result<()> {
        tri!(self.ser.formatter.begin_string(& mut self.ser.writer).map_err(Error::io));
        tri!(
            self.ser.formatter.write_u32(& mut self.ser.writer, value).map_err(Error::io)
        );
        tri!(self.ser.formatter.end_string(& mut self.ser.writer).map_err(Error::io));
        Ok(())
    }
    fn serialize_u64(self, value: u64) -> Result<()> {
        tri!(self.ser.formatter.begin_string(& mut self.ser.writer).map_err(Error::io));
        tri!(
            self.ser.formatter.write_u64(& mut self.ser.writer, value).map_err(Error::io)
        );
        tri!(self.ser.formatter.end_string(& mut self.ser.writer).map_err(Error::io));
        Ok(())
    }
    serde_if_integer128! {
        fn serialize_u128(self, value : u128) -> Result < () > { tri!(self.ser.formatter
        .begin_string(& mut self.ser.writer).map_err(Error::io)); tri!(self.ser.formatter
        .write_number_str(& mut self.ser.writer, & value.to_string())
        .map_err(Error::io)); tri!(self.ser.formatter.end_string(& mut self.ser.writer)
        .map_err(Error::io)); Ok(()) }
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
    serde_if_integer128! {
        fn serialize_i128(self, _v : i128) -> Result < () > { Err(invalid_number()) }
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
    serde_if_integer128! {
        fn serialize_u128(self, _v : u128) -> Result < () > { Err(invalid_number()) }
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
    serde_if_integer128! {
        fn serialize_i128(self, _v : i128) -> Result < () > {
        Err(ser::Error::custom("expected RawValue")) }
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
    serde_if_integer128! {
        fn serialize_u128(self, _v : u128) -> Result < () > {
        Err(ser::Error::custom("expected RawValue")) }
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
        if first {
            tri!(writer.write_all(b"\n"));
        } else {
            tri!(writer.write_all(b",\n"));
        }
        tri!(indent(writer, self.current_indent, self.indent));
        Ok(())
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
        if first {
            tri!(writer.write_all(b"\n"));
        } else {
            tri!(writer.write_all(b",\n"));
        }
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
    tri!(formatter.end_string(writer));
    Ok(())
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
    if start != bytes.len() {
        tri!(formatter.write_string_fragment(writer, & value[start..]));
    }
    Ok(())
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
/// # Errors
///
/// Serialization can fail if `T`'s implementation of `Serialize` decides to
/// fail, or if `T` contains a map with non-string keys.
#[inline]
pub fn to_writer<W, T>(writer: W, value: &T) -> Result<()>
where
    W: io::Write,
    T: ?Sized + Serialize,
{
    let mut ser = Serializer::new(writer);
    tri!(value.serialize(& mut ser));
    Ok(())
}
/// Serialize the given data structure as pretty-printed JSON into the IO
/// stream.
///
/// # Errors
///
/// Serialization can fail if `T`'s implementation of `Serialize` decides to
/// fail, or if `T` contains a map with non-string keys.
#[inline]
pub fn to_writer_pretty<W, T>(writer: W, value: &T) -> Result<()>
where
    W: io::Write,
    T: ?Sized + Serialize,
{
    let mut ser = Serializer::pretty(writer);
    tri!(value.serialize(& mut ser));
    Ok(())
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
mod tests_llm_16_100_llm_16_99 {
    use super::*;
    use crate::*;
    use serde::ser::{Serializer, Serialize};
    #[test]
    fn test_serialize_i16() {
        let _rug_st_tests_llm_16_100_llm_16_99_rrrruuuugggg_test_serialize_i16 = 0;
        let rug_fuzz_0 = 42;
        let mut serializer = crate::Serializer::new(Vec::new());
        let value: i16 = rug_fuzz_0;
        let result = value.serialize(&mut serializer);
        debug_assert!(result.is_ok());
        let _rug_ed_tests_llm_16_100_llm_16_99_rrrruuuugggg_test_serialize_i16 = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_101 {
    use serde::Serialize;
    use crate::ser::{Serializer, Formatter, CompactFormatter};
    use std::io;
    use crate::Error;
    #[test]
    fn serialize_i32_test() {
        fn do_test<'a, W: io::Write>(
            mut serializer: Serializer<W, CompactFormatter>,
            value: i32,
        ) -> Result<(), Error> {
            tri!(
                serializer.formatter.write_i32(& mut serializer.writer, value)
                .map_err(Error::io)
            );
            Ok(())
        }
        let value = 42;
        let buf: &mut Vec<u8> = &mut Vec::new();
        let writer: io::Cursor<&mut Vec<u8>> = io::Cursor::new(buf);
        let serializer = Serializer::new(writer);
        let result = do_test(serializer, value);
        assert!(result.is_ok());
    }
}
#[cfg(test)]
mod tests_llm_16_419 {
    use crate::ser::{Compound, State};
    use serde::ser::SerializeSeq;
    use crate::ser::{Formatter, CompactFormatter};
    use crate::Error;
    use std::io::{self, Write};
    use serde::Serialize;
    use crate::Serializer;
    #[test]
    fn test_end() {
        let _rug_st_tests_llm_16_419_rrrruuuugggg_test_end = 0;
        let mut writer = Vec::new();
        let formatter = CompactFormatter;
        let mut ser = Serializer::with_formatter(&mut writer, formatter);
        let compound = Compound::Map {
            ser: &mut ser,
            state: State::Empty,
        };
        let result = compound.end();
        debug_assert!(result.is_ok());
        let _rug_ed_tests_llm_16_419_rrrruuuugggg_test_end = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_423_llm_16_422 {
    use super::*;
    use crate::*;
    use crate::*;
    use serde::ser::SerializeStruct;
    use crate::ser::Error;
    use std::io::Write;
    #[cfg(feature = "arbitrary_precision")]
    use crate::number::TOKEN;
    #[cfg(feature = "raw_value")]
    use crate::raw::TOKEN;
    #[cfg(feature = "arbitrary_precision")]
    fn invalid_number() -> Error {
        Error::custom("Invalid number")
    }
    #[cfg(feature = "raw_value")]
    fn invalid_raw_value() -> Error {
        Error::custom("Invalid raw value")
    }
    struct NumberStrEmitter<'a, W: 'a>(&'a mut Serializer<W, CompactFormatter>);
    struct RawValueStrEmitter<'a, W: 'a>(&'a mut Serializer<W, CompactFormatter>);
    #[test]
    fn test_end_with_map() {
        let mut serializer = Serializer::new(Vec::new());
        let compound = Compound::Map {
            ser: &mut serializer,
            state: State::First,
        };
        let result = compound.end();
        assert!(result.is_ok());
    }
    #[test]
    #[cfg(feature = "arbitrary_precision")]
    fn test_end_with_number() {
        let mut serializer = Serializer::new(Vec::new());
        let compound = Compound::Number {
            ser: &mut serializer,
        };
        let result = compound.end();
        assert!(result.is_ok());
    }
    #[test]
    #[cfg(feature = "raw_value")]
    fn test_end_with_raw_value() {
        let mut serializer = Serializer::new(Vec::new());
        let compound = Compound::RawValue {
            ser: &mut serializer,
        };
        let result = compound.end();
        assert!(result.is_ok());
    }
}
#[cfg(test)]
mod tests_llm_16_503 {
    use crate::ser::{PrettyFormatter, Formatter};
    use std::io::{self, Write};
    #[test]
    fn test_begin_array() -> io::Result<()> {
        let mut formatter = PrettyFormatter::new();
        let mut writer: Vec<u8> = Vec::new();
        formatter.begin_array(&mut writer)?;
        let expected_output = "[";
        assert_eq!(writer, expected_output.as_bytes());
        Ok(())
    }
}
#[cfg(test)]
mod tests_llm_16_508 {
    use super::*;
    use crate::*;
    use crate::ser::PrettyFormatter;
    use std::io::{self, Write};
    fn indent<W>(writer: &mut W, current_indent: usize, indent: &[u8]) -> io::Result<()>
    where
        W: ?Sized + Write,
    {
        for _ in 0..current_indent {
            writer.write_all(indent)?;
        }
        Ok(())
    }
    #[test]
    fn test_begin_object_key() -> io::Result<()> {
        let mut writer: Vec<u8> = Vec::new();
        let mut formatter = PrettyFormatter::new();
        formatter.begin_object_key(&mut writer, true)?;
        formatter.begin_object_key(&mut writer, false)?;
        Ok(())
    }
}
#[cfg(test)]
mod tests_llm_16_509 {
    use super::*;
    use crate::*;
    use std::io::Write;
    struct MockWriter(Vec<u8>);
    impl Write for MockWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    #[test]
    fn test_begin_object_value() {
        let _rug_st_tests_llm_16_509_rrrruuuugggg_test_begin_object_value = 0;
        let mut formatter = PrettyFormatter::new();
        let mut writer = MockWriter(Vec::new());
        debug_assert!(formatter.begin_object_value(& mut writer).is_ok());
        debug_assert_eq!(writer.0, b": ");
        let _rug_ed_tests_llm_16_509_rrrruuuugggg_test_begin_object_value = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_511_llm_16_510 {
    use std::io;
    use std::io::Write;
    use crate::ser::PrettyFormatter;
    use crate::ser::Formatter;
    #[test]
    fn test_end_array() {
        let _rug_st_tests_llm_16_511_llm_16_510_rrrruuuugggg_test_end_array = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = true;
        let rug_fuzz_2 = b"  ";
        let rug_fuzz_3 = b"]";
        let mut writer = Vec::new();
        let mut formatter = PrettyFormatter::new();
        formatter.current_indent = rug_fuzz_0;
        formatter.has_value = rug_fuzz_1;
        formatter.indent = rug_fuzz_2;
        let result = formatter.end_array(&mut writer);
        debug_assert!(result.is_ok());
        let expected = rug_fuzz_3;
        debug_assert_eq!(writer.as_slice(), expected);
        let _rug_ed_tests_llm_16_511_llm_16_510_rrrruuuugggg_test_end_array = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_512 {
    use crate::ser::{Formatter, PrettyFormatter};
    use std::io::{self, Write};
    #[test]
    fn test_end_array_value() {
        let _rug_st_tests_llm_16_512_rrrruuuugggg_test_end_array_value = 0;
        let mut formatter = PrettyFormatter::new();
        let mut writer = Vec::new();
        let result = formatter.end_array_value(&mut writer);
        debug_assert!(result.is_ok());
        let _rug_ed_tests_llm_16_512_rrrruuuugggg_test_end_array_value = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_514_llm_16_513 {
    use super::*;
    use crate::*;
    use crate::*;
    use std::io::Write;
    #[test]
    fn test_end_object() {
        let _rug_st_tests_llm_16_514_llm_16_513_rrrruuuugggg_test_end_object = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = true;
        let rug_fuzz_2 = b"  ";
        let mut writer = Vec::new();
        let mut formatter = PrettyFormatter::new();
        formatter.current_indent = rug_fuzz_0;
        formatter.has_value = rug_fuzz_1;
        formatter.indent = rug_fuzz_2;
        let result = formatter.end_object(&mut writer);
        debug_assert!(result.is_ok());
        debug_assert_eq!(writer, b"}");
        let _rug_ed_tests_llm_16_514_llm_16_513_rrrruuuugggg_test_end_object = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_515 {
    use super::*;
    use crate::*;
    use crate::ser::{PrettyFormatter, Formatter};
    use std::io::{self, Write};
    struct MockWriter;
    impl Write for MockWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    #[test]
    fn test_end_object_value() -> io::Result<()> {
        let mut formatter = PrettyFormatter::new();
        let mut writer = MockWriter;
        formatter.end_object_value(&mut writer)?;
        assert!(formatter.has_value);
        Ok(())
    }
}
#[cfg(test)]
mod tests_llm_16_516 {
    use std::io;
    use std::io::Write;
    use crate::ser::{Formatter, PrettyFormatter};
    #[test]
    fn test_default() {
        let mut formatter: PrettyFormatter<'static> = PrettyFormatter::default();
        let mut output = Vec::new();
        write_json(&mut formatter, &mut output).unwrap();
        let expected = b"{\n  \n}";
        assert_eq!(output, expected);
    }
    fn write_json<W: Write>(
        formatter: &mut PrettyFormatter<'static>,
        writer: &mut W,
    ) -> io::Result<()> {
        formatter.begin_object(writer)?;
        formatter.begin_object_key(writer, false)?;
        formatter.begin_object_value(writer)?;
        formatter.end_object_value(writer)?;
        formatter.end_object(writer)?;
        Ok(())
    }
}
#[cfg(test)]
mod tests_llm_16_1020 {
    use std::io::Cursor;
    use crate::ser::{Formatter, PrettyFormatter};
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_1020_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = true;
        let mut formatter: PrettyFormatter = PrettyFormatter::new();
        let mut writer = Cursor::new(Vec::new());
        let result = formatter.begin_array(&mut writer);
        debug_assert!(result.is_ok());
        let result = formatter.end_array(&mut writer);
        debug_assert!(result.is_ok());
        let result = formatter.begin_array_value(&mut writer, rug_fuzz_0);
        debug_assert!(result.is_ok());
        let result = formatter.end_array_value(&mut writer);
        debug_assert!(result.is_ok());
        let result = formatter.begin_object(&mut writer);
        debug_assert!(result.is_ok());
        let result = formatter.end_object(&mut writer);
        debug_assert!(result.is_ok());
        let result = formatter.begin_object_key(&mut writer, rug_fuzz_1);
        debug_assert!(result.is_ok());
        let result = formatter.begin_object_value(&mut writer);
        debug_assert!(result.is_ok());
        let result = formatter.end_object_value(&mut writer);
        debug_assert!(result.is_ok());
        let _rug_ed_tests_llm_16_1020_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_1021 {
    use crate::ser::{Formatter, PrettyFormatter};
    use std::io::{self, Write};
    #[test]
    fn test_with_indent() {
        let _rug_st_tests_llm_16_1021_rrrruuuugggg_test_with_indent = 0;
        let rug_fuzz_0 = b"    ";
        let indent = rug_fuzz_0;
        let formatter = PrettyFormatter::with_indent(indent);
        debug_assert_eq!(formatter.current_indent, 0);
        debug_assert_eq!(formatter.has_value, false);
        debug_assert_eq!(formatter.indent, indent);
        let _rug_ed_tests_llm_16_1021_rrrruuuugggg_test_with_indent = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_1022 {
    use crate::ser::{CompactFormatter, PrettyFormatter, Serializer};
    use std::io::{self, Write};
    #[test]
    fn test_into_inner() {
        let _rug_st_tests_llm_16_1022_rrrruuuugggg_test_into_inner = 0;
        let writer: io::Cursor<Vec<u8>> = io::Cursor::new(vec![]);
        let serializer: Serializer<io::Cursor<Vec<u8>>, CompactFormatter> = Serializer::new(
            writer,
        );
        let result: io::Cursor<Vec<u8>> = serializer.into_inner();
        let _rug_ed_tests_llm_16_1022_rrrruuuugggg_test_into_inner = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_1023 {
    use crate::ser::{Serializer, Formatter, CompactFormatter};
    #[test]
    fn test_with_formatter() {
        let _rug_st_tests_llm_16_1023_rrrruuuugggg_test_with_formatter = 0;
        let writer = std::io::stdout();
        let formatter = CompactFormatter;
        let serializer = Serializer::with_formatter(writer, formatter);
        let result = serializer.into_inner();
        let _rug_ed_tests_llm_16_1023_rrrruuuugggg_test_with_formatter = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_1025_llm_16_1024 {
    use crate::ser::{Serializer, PrettyFormatter};
    use crate::ser::Formatter;
    use std::io;
    #[test]
    fn test_pretty() {
        let _rug_st_tests_llm_16_1025_llm_16_1024_rrrruuuugggg_test_pretty = 0;
        let writer: Vec<u8> = Vec::new();
        let serializer = Serializer::<_, PrettyFormatter>::pretty(writer);
        let res: Vec<u8> = serializer.into_inner();
        debug_assert_eq!(res, Vec:: < u8 > ::new());
        let _rug_ed_tests_llm_16_1025_llm_16_1024_rrrruuuugggg_test_pretty = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_1026 {
    use super::*;
    use crate::*;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_1026_rrrruuuugggg_test_new = 0;
        let writer: Vec<u8> = Vec::new();
        let serializer = Serializer::new(writer);
        let result = serializer.into_inner();
        debug_assert_eq!(result.len(), 0);
        let _rug_ed_tests_llm_16_1026_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_1027 {
    use super::*;
    use crate::*;
    use crate::ser::format_escaped_str;
    use crate::ser::{Formatter, CompactFormatter};
    use std::io::{self, Write};
    use std::fmt::{self, Write as FmtWrite};
    #[derive(Debug, Clone)]
    struct MockFormatter;
    impl Formatter for MockFormatter {
        fn begin_string<W>(&mut self, writer: &mut W) -> io::Result<()>
        where
            W: ?Sized + io::Write,
        {
            writer.write_all(b"\"")
        }
        fn end_string<W>(&mut self, writer: &mut W) -> io::Result<()>
        where
            W: ?Sized + io::Write,
        {
            writer.write_all(b"\"")
        }
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
    }
    #[test]
    fn test_format_escaped_str() {
        let _rug_st_tests_llm_16_1027_rrrruuuugggg_test_format_escaped_str = 0;
        let rug_fuzz_0 = "hello world";
        let mut writer: Vec<u8> = Vec::new();
        let mut formatter = MockFormatter;
        let value = rug_fuzz_0;
        let result = format_escaped_str(&mut writer, &mut formatter, value);
        debug_assert!(result.is_ok());
        debug_assert_eq!(writer, b"\"hello world\"");
        let _rug_ed_tests_llm_16_1027_rrrruuuugggg_test_format_escaped_str = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_1030 {
    use crate::ser::indent;
    use std::io;
    use std::io::Write;
    use std::str;
    struct MockWriter {
        output: Vec<u8>,
    }
    impl MockWriter {
        fn new() -> MockWriter {
            MockWriter { output: Vec::new() }
        }
    }
    impl Write for MockWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.output.extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    #[test]
    fn test_indent() {
        let _rug_st_tests_llm_16_1030_rrrruuuugggg_test_indent = 0;
        let rug_fuzz_0 = b"    ";
        let rug_fuzz_1 = b"test";
        let rug_fuzz_2 = 3;
        let mut writer = MockWriter::new();
        let indent_str = rug_fuzz_0;
        let input = rug_fuzz_1;
        indent(&mut writer, rug_fuzz_2, indent_str).unwrap();
        writer.write(input).unwrap();
        debug_assert_eq!(writer.output, b"            test");
        let _rug_ed_tests_llm_16_1030_rrrruuuugggg_test_indent = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_1031 {
    use crate::ser::key_must_be_a_string;
    use crate::error::Error;
    use serde::de::Error as DeError;
    #[test]
    fn test_key_must_be_a_string() {
        let _rug_st_tests_llm_16_1031_rrrruuuugggg_test_key_must_be_a_string = 0;
        let error = key_must_be_a_string();
        debug_assert_eq!(error.classify(), crate ::error::Category::Syntax);
        let _rug_ed_tests_llm_16_1031_rrrruuuugggg_test_key_must_be_a_string = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_1032 {
    use crate::{to_string, Value, Map};
    #[test]
    fn test_to_string() {
        let _rug_st_tests_llm_16_1032_rrrruuuugggg_test_to_string = 0;
        let rug_fuzz_0 = r#"{
            "name": "John Doe",
            "age": 30,
            "city": "New York"
        }"#;
        let rug_fuzz_1 = "name";
        let rug_fuzz_2 = "John Doe";
        let rug_fuzz_3 = "age";
        let rug_fuzz_4 = 30;
        let rug_fuzz_5 = "city";
        let rug_fuzz_6 = "New York";
        let value: Value = crate::from_str(rug_fuzz_0).unwrap();
        debug_assert_eq!(
            to_string(& value).unwrap(),
            "{\"name\":\"John Doe\",\"age\":30,\"city\":\"New York\"}"
        );
        let mut map = Map::new();
        map.insert(rug_fuzz_1.into(), Value::String(rug_fuzz_2.into()));
        map.insert(rug_fuzz_3.into(), Value::Number(rug_fuzz_4.into()));
        map.insert(rug_fuzz_5.into(), Value::String(rug_fuzz_6.into()));
        debug_assert_eq!(
            to_string(& map).unwrap(),
            "{\"name\":\"John Doe\",\"age\":30,\"city\":\"New York\"}"
        );
        let _rug_ed_tests_llm_16_1032_rrrruuuugggg_test_to_string = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_1033 {
    use crate::ser::to_string_pretty;
    #[test]
    fn test_to_string_pretty() {
        let _rug_st_tests_llm_16_1033_rrrruuuugggg_test_to_string_pretty = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = "hello";
        let rug_fuzz_2 = 1;
        let value1 = rug_fuzz_0;
        let result1 = to_string_pretty(&value1).unwrap();
        debug_assert_eq!(result1, "10");
        let value2 = rug_fuzz_1;
        let result2 = to_string_pretty(&value2).unwrap();
        debug_assert_eq!(result2, "\"hello\"");
        let value3 = vec![rug_fuzz_2, 2, 3];
        let result3 = to_string_pretty(&value3).unwrap();
        debug_assert_eq!(result3, "[\n  1,\n  2,\n  3\n]");
        let value4 = crate::map::Map::new();
        let result4 = to_string_pretty(&value4).unwrap();
        debug_assert_eq!(result4, "{}");
        let value5 = crate::map::Map::new();
        let result5 = to_string_pretty(&value5).unwrap();
        debug_assert_eq!(result5, "{}");
        let _rug_ed_tests_llm_16_1033_rrrruuuugggg_test_to_string_pretty = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_1036 {
    use crate::ser::{to_vec_pretty, PrettyFormatter};
    use crate::{Map, Value};
    use std::collections::BTreeMap;
    #[test]
    fn test_to_vec_pretty() {
        let _rug_st_tests_llm_16_1036_rrrruuuugggg_test_to_vec_pretty = 0;
        let rug_fuzz_0 = "name";
        let rug_fuzz_1 = "John Doe";
        let rug_fuzz_2 = "age";
        let rug_fuzz_3 = 30;
        let rug_fuzz_4 = "is_employed";
        let rug_fuzz_5 = true;
        let rug_fuzz_6 = "{\n  \"name\": \"John Doe\",\n  \"age\": 30,\n  \"is_employed\": true\n}";
        let mut data = Map::new();
        data.insert(rug_fuzz_0.to_owned(), Value::String(rug_fuzz_1.to_owned()));
        data.insert(rug_fuzz_2.to_owned(), Value::Number(rug_fuzz_3.into()));
        data.insert(rug_fuzz_4.to_owned(), Value::Bool(rug_fuzz_5));
        let result = to_vec_pretty(&data).unwrap();
        let result_str = String::from_utf8(result).unwrap();
        let expected = String::from(rug_fuzz_6);
        debug_assert_eq!(result_str, expected);
        let _rug_ed_tests_llm_16_1036_rrrruuuugggg_test_to_vec_pretty = 0;
    }
}
#[cfg(test)]
mod tests_rug_110 {
    use super::*;
    use crate::{Number, Value, Map};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_110_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let mut v29 = Value::Object(Map::new());
        v29[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
        let p0: &Value = &v29;
        crate::ser::to_vec(p0).unwrap();
        let _rug_ed_tests_rug_110_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_112 {
    use super::*;
    use std::io::{self, Write};
    use crate::ser;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_112_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = true;
        let mut p0 = ser::CompactFormatter;
        let mut p1 = &mut Vec::new();
        let p2 = rug_fuzz_0;
        ser::Formatter::write_bool(&mut p0, &mut p1, p2).unwrap();
        let _rug_ed_tests_rug_112_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_129 {
    use super::*;
    use crate::ser::{CompactFormatter, Formatter};
    use std::fmt::Write;
    use std::io;
    #[test]
    fn test_end_array() {
        let _rug_st_tests_rug_129_rrrruuuugggg_test_end_array = 0;
        let mut p0 = CompactFormatter;
        let mut p1 = WriterFormatter {
            inner: &mut String::new(),
        };
        crate::ser::Formatter::end_array(&mut p0, &mut p1).unwrap();
        debug_assert_eq!(p1.inner, "]");
        let _rug_ed_tests_rug_129_rrrruuuugggg_test_end_array = 0;
    }
    struct WriterFormatter<'a> {
        inner: &'a mut String,
    }
    impl<'a> io::Write for WriterFormatter<'a> {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let s = std::str::from_utf8(buf).unwrap();
            self.inner.write_str(s).unwrap();
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
}
#[cfg(test)]
mod tests_rug_213 {
    use super::*;
    use crate::ser::CharEscape;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_213_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 65;
        let rug_fuzz_1 = 66;
        let p0: u8 = rug_fuzz_0;
        let p1: u8 = rug_fuzz_1;
        CharEscape::from_escape_table(p0, p1);
        let _rug_ed_tests_rug_213_rrrruuuugggg_test_rug = 0;
    }
}
