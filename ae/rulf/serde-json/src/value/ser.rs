use crate::error::{Error, ErrorCode, Result};
use crate::lib::*;
use crate::map::Map;
use crate::number::Number;
use crate::value::{to_value, Value};
use serde::ser::{Impossible, Serialize};
#[cfg(feature = "arbitrary_precision")]
use serde::serde_if_integer128;
impl Serialize for Value {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> result::Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        match *self {
            Value::Null => serializer.serialize_unit(),
            Value::Bool(b) => serializer.serialize_bool(b),
            Value::Number(ref n) => n.serialize(serializer),
            Value::String(ref s) => serializer.serialize_str(s),
            Value::Array(ref v) => v.serialize(serializer),
            #[cfg(any(feature = "std", feature = "alloc"))]
            Value::Object(ref m) => {
                use serde::ser::SerializeMap;
                let mut map = tri!(serializer.serialize_map(Some(m.len())));
                for (k, v) in m {
                    tri!(map.serialize_entry(k, v));
                }
                map.end()
            }
        }
    }
}
/// Serializer whose output is a `Value`.
///
/// This is the serializer that backs [`serde_json::to_value`][crate::to_value].
/// Unlike the main serde_json serializer which goes from some serializable
/// value of type `T` to JSON text, this one goes from `T` to
/// `serde_json::Value`.
///
/// The `to_value` function is implementable as:
///
/// ```
/// use serde::Serialize;
/// use serde_json::{Error, Value};
///
/// pub fn to_value<T>(input: T) -> Result<Value, Error>
/// where
///     T: Serialize,
/// {
///     input.serialize(serde_json::value::Serializer)
/// }
/// ```
pub struct Serializer;
impl serde::Serializer for Serializer {
    type Ok = Value;
    type Error = Error;
    type SerializeSeq = SerializeVec;
    type SerializeTuple = SerializeVec;
    type SerializeTupleStruct = SerializeVec;
    type SerializeTupleVariant = SerializeTupleVariant;
    type SerializeMap = SerializeMap;
    type SerializeStruct = SerializeMap;
    type SerializeStructVariant = SerializeStructVariant;
    #[inline]
    fn serialize_bool(self, value: bool) -> Result<Value> {
        Ok(Value::Bool(value))
    }
    #[inline]
    fn serialize_i8(self, value: i8) -> Result<Value> {
        self.serialize_i64(value as i64)
    }
    #[inline]
    fn serialize_i16(self, value: i16) -> Result<Value> {
        self.serialize_i64(value as i64)
    }
    #[inline]
    fn serialize_i32(self, value: i32) -> Result<Value> {
        self.serialize_i64(value as i64)
    }
    fn serialize_i64(self, value: i64) -> Result<Value> {
        Ok(Value::Number(value.into()))
    }
    #[cfg(feature = "arbitrary_precision")]
    serde_if_integer128! {
        fn serialize_i128(self, value : i128) -> Result < Value > {
        Ok(Value::Number(value.into())) }
    }
    #[inline]
    fn serialize_u8(self, value: u8) -> Result<Value> {
        self.serialize_u64(value as u64)
    }
    #[inline]
    fn serialize_u16(self, value: u16) -> Result<Value> {
        self.serialize_u64(value as u64)
    }
    #[inline]
    fn serialize_u32(self, value: u32) -> Result<Value> {
        self.serialize_u64(value as u64)
    }
    #[inline]
    fn serialize_u64(self, value: u64) -> Result<Value> {
        Ok(Value::Number(value.into()))
    }
    #[cfg(feature = "arbitrary_precision")]
    serde_if_integer128! {
        fn serialize_u128(self, value : u128) -> Result < Value > {
        Ok(Value::Number(value.into())) }
    }
    #[inline]
    fn serialize_f32(self, value: f32) -> Result<Value> {
        self.serialize_f64(value as f64)
    }
    #[inline]
    fn serialize_f64(self, value: f64) -> Result<Value> {
        Ok(Number::from_f64(value).map_or(Value::Null, Value::Number))
    }
    #[inline]
    fn serialize_char(self, value: char) -> Result<Value> {
        let mut s = String::new();
        s.push(value);
        Ok(Value::String(s))
    }
    #[inline]
    fn serialize_str(self, value: &str) -> Result<Value> {
        Ok(Value::String(value.to_owned()))
    }
    fn serialize_bytes(self, value: &[u8]) -> Result<Value> {
        let vec = value.iter().map(|&b| Value::Number(b.into())).collect();
        Ok(Value::Array(vec))
    }
    #[inline]
    fn serialize_unit(self) -> Result<Value> {
        Ok(Value::Null)
    }
    #[inline]
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Value> {
        self.serialize_unit()
    }
    #[inline]
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Value> {
        self.serialize_str(variant)
    }
    #[inline]
    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<Value>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Value>
    where
        T: ?Sized + Serialize,
    {
        let mut values = Map::new();
        values.insert(String::from(variant), tri!(to_value(& value)));
        Ok(Value::Object(values))
    }
    #[inline]
    fn serialize_none(self) -> Result<Value> {
        self.serialize_unit()
    }
    #[inline]
    fn serialize_some<T>(self, value: &T) -> Result<Value>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        Ok(SerializeVec {
            vec: Vec::with_capacity(len.unwrap_or(0)),
        })
    }
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Ok(SerializeTupleVariant {
            name: String::from(variant),
            vec: Vec::with_capacity(len),
        })
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(SerializeMap::Map {
            map: Map::new(),
            next_key: None,
        })
    }
    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct> {
        match name {
            #[cfg(feature = "arbitrary_precision")]
            crate::number::TOKEN => {
                Ok(SerializeMap::Number {
                    out_value: None,
                })
            }
            #[cfg(feature = "raw_value")]
            crate::raw::TOKEN => {
                Ok(SerializeMap::RawValue {
                    out_value: None,
                })
            }
            _ => self.serialize_map(Some(len)),
        }
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Ok(SerializeStructVariant {
            name: String::from(variant),
            map: Map::new(),
        })
    }
    fn collect_str<T: ?Sized>(self, value: &T) -> Result<Value>
    where
        T: Display,
    {
        Ok(Value::String(value.to_string()))
    }
}
pub struct SerializeVec {
    vec: Vec<Value>,
}
pub struct SerializeTupleVariant {
    name: String,
    vec: Vec<Value>,
}
pub enum SerializeMap {
    Map { map: Map<String, Value>, next_key: Option<String> },
    #[cfg(feature = "arbitrary_precision")]
    Number { out_value: Option<Value> },
    #[cfg(feature = "raw_value")]
    RawValue { out_value: Option<Value> },
}
pub struct SerializeStructVariant {
    name: String,
    map: Map<String, Value>,
}
impl serde::ser::SerializeSeq for SerializeVec {
    type Ok = Value;
    type Error = Error;
    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.vec.push(tri!(to_value(& value)));
        Ok(())
    }
    fn end(self) -> Result<Value> {
        Ok(Value::Array(self.vec))
    }
}
impl serde::ser::SerializeTuple for SerializeVec {
    type Ok = Value;
    type Error = Error;
    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        serde::ser::SerializeSeq::serialize_element(self, value)
    }
    fn end(self) -> Result<Value> {
        serde::ser::SerializeSeq::end(self)
    }
}
impl serde::ser::SerializeTupleStruct for SerializeVec {
    type Ok = Value;
    type Error = Error;
    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        serde::ser::SerializeSeq::serialize_element(self, value)
    }
    fn end(self) -> Result<Value> {
        serde::ser::SerializeSeq::end(self)
    }
}
impl serde::ser::SerializeTupleVariant for SerializeTupleVariant {
    type Ok = Value;
    type Error = Error;
    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.vec.push(tri!(to_value(& value)));
        Ok(())
    }
    fn end(self) -> Result<Value> {
        let mut object = Map::new();
        object.insert(self.name, Value::Array(self.vec));
        Ok(Value::Object(object))
    }
}
impl serde::ser::SerializeMap for SerializeMap {
    type Ok = Value;
    type Error = Error;
    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        match *self {
            SerializeMap::Map { ref mut next_key, .. } => {
                *next_key = Some(tri!(key.serialize(MapKeySerializer)));
                Ok(())
            }
            #[cfg(feature = "arbitrary_precision")]
            SerializeMap::Number { .. } => unreachable!(),
            #[cfg(feature = "raw_value")]
            SerializeMap::RawValue { .. } => unreachable!(),
        }
    }
    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        match *self {
            SerializeMap::Map { ref mut map, ref mut next_key } => {
                let key = next_key.take();
                let key = key.expect("serialize_value called before serialize_key");
                map.insert(key, tri!(to_value(& value)));
                Ok(())
            }
            #[cfg(feature = "arbitrary_precision")]
            SerializeMap::Number { .. } => unreachable!(),
            #[cfg(feature = "raw_value")]
            SerializeMap::RawValue { .. } => unreachable!(),
        }
    }
    fn end(self) -> Result<Value> {
        match self {
            SerializeMap::Map { map, .. } => Ok(Value::Object(map)),
            #[cfg(feature = "arbitrary_precision")]
            SerializeMap::Number { .. } => unreachable!(),
            #[cfg(feature = "raw_value")]
            SerializeMap::RawValue { .. } => unreachable!(),
        }
    }
}
struct MapKeySerializer;
fn key_must_be_a_string() -> Error {
    Error::syntax(ErrorCode::KeyMustBeAString, 0, 0)
}
impl serde::Serializer for MapKeySerializer {
    type Ok = String;
    type Error = Error;
    type SerializeSeq = Impossible<String, Error>;
    type SerializeTuple = Impossible<String, Error>;
    type SerializeTupleStruct = Impossible<String, Error>;
    type SerializeTupleVariant = Impossible<String, Error>;
    type SerializeMap = Impossible<String, Error>;
    type SerializeStruct = Impossible<String, Error>;
    type SerializeStructVariant = Impossible<String, Error>;
    #[inline]
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<String> {
        Ok(variant.to_owned())
    }
    #[inline]
    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<String>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }
    fn serialize_bool(self, _value: bool) -> Result<String> {
        Err(key_must_be_a_string())
    }
    fn serialize_i8(self, value: i8) -> Result<String> {
        Ok(value.to_string())
    }
    fn serialize_i16(self, value: i16) -> Result<String> {
        Ok(value.to_string())
    }
    fn serialize_i32(self, value: i32) -> Result<String> {
        Ok(value.to_string())
    }
    fn serialize_i64(self, value: i64) -> Result<String> {
        Ok(value.to_string())
    }
    fn serialize_u8(self, value: u8) -> Result<String> {
        Ok(value.to_string())
    }
    fn serialize_u16(self, value: u16) -> Result<String> {
        Ok(value.to_string())
    }
    fn serialize_u32(self, value: u32) -> Result<String> {
        Ok(value.to_string())
    }
    fn serialize_u64(self, value: u64) -> Result<String> {
        Ok(value.to_string())
    }
    fn serialize_f32(self, _value: f32) -> Result<String> {
        Err(key_must_be_a_string())
    }
    fn serialize_f64(self, _value: f64) -> Result<String> {
        Err(key_must_be_a_string())
    }
    #[inline]
    fn serialize_char(self, value: char) -> Result<String> {
        Ok({
            let mut s = String::new();
            s.push(value);
            s
        })
    }
    #[inline]
    fn serialize_str(self, value: &str) -> Result<String> {
        Ok(value.to_owned())
    }
    fn serialize_bytes(self, _value: &[u8]) -> Result<String> {
        Err(key_must_be_a_string())
    }
    fn serialize_unit(self) -> Result<String> {
        Err(key_must_be_a_string())
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<String> {
        Err(key_must_be_a_string())
    }
    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<String>
    where
        T: ?Sized + Serialize,
    {
        Err(key_must_be_a_string())
    }
    fn serialize_none(self) -> Result<String> {
        Err(key_must_be_a_string())
    }
    fn serialize_some<T>(self, _value: &T) -> Result<String>
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
    fn collect_str<T: ?Sized>(self, value: &T) -> Result<String>
    where
        T: Display,
    {
        Ok(value.to_string())
    }
}
impl serde::ser::SerializeStruct for SerializeMap {
    type Ok = Value;
    type Error = Error;
    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        match *self {
            SerializeMap::Map { .. } => {
                serde::ser::SerializeMap::serialize_entry(self, key, value)
            }
            #[cfg(feature = "arbitrary_precision")]
            SerializeMap::Number { ref mut out_value } => {
                if key == crate::number::TOKEN {
                    *out_value = Some(value.serialize(NumberValueEmitter)?);
                    Ok(())
                } else {
                    Err(invalid_number())
                }
            }
            #[cfg(feature = "raw_value")]
            SerializeMap::RawValue { ref mut out_value } => {
                if key == crate::raw::TOKEN {
                    *out_value = Some(value.serialize(RawValueEmitter)?);
                    Ok(())
                } else {
                    Err(invalid_raw_value())
                }
            }
        }
    }
    fn end(self) -> Result<Value> {
        match self {
            SerializeMap::Map { .. } => serde::ser::SerializeMap::end(self),
            #[cfg(feature = "arbitrary_precision")]
            SerializeMap::Number { out_value, .. } => {
                Ok(out_value.expect("number value was not emitted"))
            }
            #[cfg(feature = "raw_value")]
            SerializeMap::RawValue { out_value, .. } => {
                Ok(out_value.expect("raw value was not emitted"))
            }
        }
    }
}
impl serde::ser::SerializeStructVariant for SerializeStructVariant {
    type Ok = Value;
    type Error = Error;
    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.map.insert(String::from(key), tri!(to_value(& value)));
        Ok(())
    }
    fn end(self) -> Result<Value> {
        let mut object = Map::new();
        object.insert(self.name, Value::Object(self.map));
        Ok(Value::Object(object))
    }
}
#[cfg(feature = "arbitrary_precision")]
struct NumberValueEmitter;
#[cfg(feature = "arbitrary_precision")]
fn invalid_number() -> Error {
    Error::syntax(ErrorCode::InvalidNumber, 0, 0)
}
#[cfg(feature = "arbitrary_precision")]
impl serde::ser::Serializer for NumberValueEmitter {
    type Ok = Value;
    type Error = Error;
    type SerializeSeq = Impossible<Value, Error>;
    type SerializeTuple = Impossible<Value, Error>;
    type SerializeTupleStruct = Impossible<Value, Error>;
    type SerializeTupleVariant = Impossible<Value, Error>;
    type SerializeMap = Impossible<Value, Error>;
    type SerializeStruct = Impossible<Value, Error>;
    type SerializeStructVariant = Impossible<Value, Error>;
    fn serialize_bool(self, _v: bool) -> Result<Value> {
        Err(invalid_number())
    }
    fn serialize_i8(self, _v: i8) -> Result<Value> {
        Err(invalid_number())
    }
    fn serialize_i16(self, _v: i16) -> Result<Value> {
        Err(invalid_number())
    }
    fn serialize_i32(self, _v: i32) -> Result<Value> {
        Err(invalid_number())
    }
    fn serialize_i64(self, _v: i64) -> Result<Value> {
        Err(invalid_number())
    }
    fn serialize_u8(self, _v: u8) -> Result<Value> {
        Err(invalid_number())
    }
    fn serialize_u16(self, _v: u16) -> Result<Value> {
        Err(invalid_number())
    }
    fn serialize_u32(self, _v: u32) -> Result<Value> {
        Err(invalid_number())
    }
    fn serialize_u64(self, _v: u64) -> Result<Value> {
        Err(invalid_number())
    }
    fn serialize_f32(self, _v: f32) -> Result<Value> {
        Err(invalid_number())
    }
    fn serialize_f64(self, _v: f64) -> Result<Value> {
        Err(invalid_number())
    }
    fn serialize_char(self, _v: char) -> Result<Value> {
        Err(invalid_number())
    }
    fn serialize_str(self, value: &str) -> Result<Value> {
        let n = tri!(value.to_owned().parse());
        Ok(Value::Number(n))
    }
    fn serialize_bytes(self, _value: &[u8]) -> Result<Value> {
        Err(invalid_number())
    }
    fn serialize_none(self) -> Result<Value> {
        Err(invalid_number())
    }
    fn serialize_some<T>(self, _value: &T) -> Result<Value>
    where
        T: ?Sized + Serialize,
    {
        Err(invalid_number())
    }
    fn serialize_unit(self) -> Result<Value> {
        Err(invalid_number())
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Value> {
        Err(invalid_number())
    }
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Value> {
        Err(invalid_number())
    }
    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        _value: &T,
    ) -> Result<Value>
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
    ) -> Result<Value>
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
struct RawValueEmitter;
#[cfg(feature = "raw_value")]
fn invalid_raw_value() -> Error {
    Error::syntax(ErrorCode::ExpectedSomeValue, 0, 0)
}
#[cfg(feature = "raw_value")]
impl serde::ser::Serializer for RawValueEmitter {
    type Ok = Value;
    type Error = Error;
    type SerializeSeq = Impossible<Value, Error>;
    type SerializeTuple = Impossible<Value, Error>;
    type SerializeTupleStruct = Impossible<Value, Error>;
    type SerializeTupleVariant = Impossible<Value, Error>;
    type SerializeMap = Impossible<Value, Error>;
    type SerializeStruct = Impossible<Value, Error>;
    type SerializeStructVariant = Impossible<Value, Error>;
    fn serialize_bool(self, _v: bool) -> Result<Value> {
        Err(invalid_raw_value())
    }
    fn serialize_i8(self, _v: i8) -> Result<Value> {
        Err(invalid_raw_value())
    }
    fn serialize_i16(self, _v: i16) -> Result<Value> {
        Err(invalid_raw_value())
    }
    fn serialize_i32(self, _v: i32) -> Result<Value> {
        Err(invalid_raw_value())
    }
    fn serialize_i64(self, _v: i64) -> Result<Value> {
        Err(invalid_raw_value())
    }
    fn serialize_u8(self, _v: u8) -> Result<Value> {
        Err(invalid_raw_value())
    }
    fn serialize_u16(self, _v: u16) -> Result<Value> {
        Err(invalid_raw_value())
    }
    fn serialize_u32(self, _v: u32) -> Result<Value> {
        Err(invalid_raw_value())
    }
    fn serialize_u64(self, _v: u64) -> Result<Value> {
        Err(invalid_raw_value())
    }
    fn serialize_f32(self, _v: f32) -> Result<Value> {
        Err(invalid_raw_value())
    }
    fn serialize_f64(self, _v: f64) -> Result<Value> {
        Err(invalid_raw_value())
    }
    fn serialize_char(self, _v: char) -> Result<Value> {
        Err(invalid_raw_value())
    }
    fn serialize_str(self, value: &str) -> Result<Value> {
        crate::from_str(value)
    }
    fn serialize_bytes(self, _value: &[u8]) -> Result<Value> {
        Err(invalid_raw_value())
    }
    fn serialize_none(self) -> Result<Value> {
        Err(invalid_raw_value())
    }
    fn serialize_some<T>(self, _value: &T) -> Result<Value>
    where
        T: ?Sized + Serialize,
    {
        Err(invalid_raw_value())
    }
    fn serialize_unit(self) -> Result<Value> {
        Err(invalid_raw_value())
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Value> {
        Err(invalid_raw_value())
    }
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Value> {
        Err(invalid_raw_value())
    }
    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        _value: &T,
    ) -> Result<Value>
    where
        T: ?Sized + Serialize,
    {
        Err(invalid_raw_value())
    }
    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Value>
    where
        T: ?Sized + Serialize,
    {
        Err(invalid_raw_value())
    }
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Err(invalid_raw_value())
    }
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Err(invalid_raw_value())
    }
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Err(invalid_raw_value())
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Err(invalid_raw_value())
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Err(invalid_raw_value())
    }
    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct> {
        Err(invalid_raw_value())
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Err(invalid_raw_value())
    }
}
#[cfg(test)]
mod tests_llm_16_644 {
    use super::*;
    use crate::*;
    use serde::{Serialize, Serializer};
    #[test]
    fn serialize_bool_should_return_error() {
        let _rug_st_tests_llm_16_644_rrrruuuugggg_serialize_bool_should_return_error = 0;
        let rug_fuzz_0 = true;
        let serializer = MapKeySerializer;
        let value = rug_fuzz_0;
        let result = serializer.serialize_bool(value);
        debug_assert!(result.is_err());
        let _rug_ed_tests_llm_16_644_rrrruuuugggg_serialize_bool_should_return_error = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_655 {
    use super::*;
    use crate::*;
    use serde::ser::{Serialize, Serializer};
    #[test]
    fn test_serialize_i32() {
        let _rug_st_tests_llm_16_655_rrrruuuugggg_test_serialize_i32 = 0;
        let rug_fuzz_0 = 42;
        let rug_fuzz_1 = "42";
        let serializer = MapKeySerializer;
        let value = rug_fuzz_0;
        let result = serializer.serialize_i32(value).unwrap();
        let expected = rug_fuzz_1.to_string();
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_655_rrrruuuugggg_test_serialize_i32 = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_676 {
    use super::*;
    use crate::*;
    use serde::ser::Error;
    use serde::ser::Impossible;
    use serde::ser::Serialize;
    use serde::ser::Serializer;
    use serde::ser::SerializeStructVariant;
    use crate::Value;
    #[test]
    fn test_serialize_struct_variant() {
        let _rug_st_tests_llm_16_676_rrrruuuugggg_test_serialize_struct_variant = 0;
        let rug_fuzz_0 = "MyStruct";
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = "Variant";
        let rug_fuzz_3 = 5;
        let serializer = MapKeySerializer;
        let name = rug_fuzz_0;
        let variant_index = rug_fuzz_1;
        let variant = rug_fuzz_2;
        let len = rug_fuzz_3;
        let result = serializer
            .serialize_struct_variant(name, variant_index, variant, len);
        debug_assert!(result.is_err());
        let _rug_ed_tests_llm_16_676_rrrruuuugggg_test_serialize_struct_variant = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_680_llm_16_679 {
    use serde::ser::{Serialize, Serializer};
    use crate::value::ser::{MapKeySerializer, Impossible, key_must_be_a_string};
    use crate::Error;
    #[test]
    fn test_serialize_tuple_struct() {
        let _rug_st_tests_llm_16_680_llm_16_679_rrrruuuugggg_test_serialize_tuple_struct = 0;
        let rug_fuzz_0 = "TupleStruct";
        let rug_fuzz_1 = 3;
        let serializer = MapKeySerializer;
        let name = rug_fuzz_0;
        let len = rug_fuzz_1;
        let result = serializer.serialize_tuple_struct(name, len);
        debug_assert!(matches!(result, Err(_)));
        let _rug_ed_tests_llm_16_680_llm_16_679_rrrruuuugggg_test_serialize_tuple_struct = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_689 {
    use serde::Serializer;
    use crate::Error;
    use crate::value::ser::MapKeySerializer;
    #[test]
    fn test_serialize_u8() {
        let _rug_st_tests_llm_16_689_rrrruuuugggg_test_serialize_u8 = 0;
        let rug_fuzz_0 = 42_u8;
        let serializer = MapKeySerializer {};
        let value = rug_fuzz_0;
        let result = serializer.serialize_u8(value).unwrap();
        debug_assert_eq!(result, value.to_string());
        let _rug_ed_tests_llm_16_689_rrrruuuugggg_test_serialize_u8 = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_691_llm_16_690 {
    use super::*;
    use crate::*;
    use crate::*;
    use serde::Serializer;
    #[test]
    fn test_serialize_unit() {
        let _rug_st_tests_llm_16_691_llm_16_690_rrrruuuugggg_test_serialize_unit = 0;
        let serializer = MapKeySerializer;
        let result = serializer.serialize_unit();
        debug_assert!(result.is_err());
        let _rug_ed_tests_llm_16_691_llm_16_690_rrrruuuugggg_test_serialize_unit = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_703_llm_16_702 {
    use crate::value::ser::{SerializeMap, Map, Error, Value};
    use serde::ser::SerializeStruct;
    #[test]
    fn test_end() {
        let _rug_st_tests_llm_16_703_llm_16_702_rrrruuuugggg_test_end = 0;
        let map = Map::new();
        let serialize_map = SerializeMap::Map {
            map: map,
            next_key: None,
        };
        let result = serialize_map.end();
        debug_assert!(result.is_err());
        #[cfg(feature = "arbitrary_precision")]
        let serialize_map = SerializeMap::Number {
            out_value: None,
        };
        #[cfg(feature = "arbitrary_precision")]
        let result = serialize_map.end();
        #[cfg(feature = "arbitrary_precision")] debug_assert!(result.is_err());
        #[cfg(feature = "raw_value")]
        let serialize_map = SerializeMap::RawValue {
            out_value: None,
        };
        #[cfg(feature = "raw_value")]
        let result = serialize_map.end();
        #[cfg(feature = "raw_value")] debug_assert!(result.is_err());
        let _rug_ed_tests_llm_16_703_llm_16_702_rrrruuuugggg_test_end = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_717 {
    use serde::ser::SerializeSeq;
    use crate::value::ser::SerializeVec;
    use serde::Serialize;
    use crate::Value;
    use crate::Error;
    #[test]
    fn test_end() {
        let _rug_st_tests_llm_16_717_rrrruuuugggg_test_end = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 1;
        let mut ser = SerializeVec { vec: vec![] };
        ser.serialize_element(&rug_fuzz_0).unwrap();
        ser.serialize_element(&rug_fuzz_1).unwrap();
        ser.serialize_element(&rug_fuzz_2).unwrap();
        let result = ser.end().unwrap();
        let expected = crate::Value::Array(
            vec![
                Value::Number(rug_fuzz_3.into()), Value::Number(2.into()),
                Value::Number(3.into())
            ],
        );
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_717_rrrruuuugggg_test_end = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_726 {
    use crate::{to_value, Value};
    #[test]
    fn test_serialize_bool() {
        let _rug_st_tests_llm_16_726_rrrruuuugggg_test_serialize_bool = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = false;
        let serializer = to_value(rug_fuzz_0).unwrap();
        debug_assert_eq!(serializer, Value::Bool(true));
        let serializer = to_value(rug_fuzz_1).unwrap();
        debug_assert_eq!(serializer, Value::Bool(false));
        let _rug_ed_tests_llm_16_726_rrrruuuugggg_test_serialize_bool = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_732_llm_16_731 {
    use super::*;
    use crate::*;
    use crate::{value::ser::Serializer, Number, Value};
    #[test]
    fn test_serialize_f32() {
        let _rug_st_tests_llm_16_732_llm_16_731_rrrruuuugggg_test_serialize_f32 = 0;
        let rug_fuzz_0 = 3.14_f32;
        let val = rug_fuzz_0;
        let expected_result = Value::Number(Number::from_f64(val as f64).unwrap());
        let serializer = Serializer;
        let result = <value::ser::Serializer as serde::Serializer>::serialize_f32(
            serializer,
            val,
        );
        debug_assert_eq!(result.unwrap(), expected_result);
        let _rug_ed_tests_llm_16_732_llm_16_731_rrrruuuugggg_test_serialize_f32 = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_740_llm_16_739 {
    use crate::value::ser::Serializer;
    use serde::Serializer as SerdeSerializer;
    use crate::Value;
    #[test]
    fn test_serialize_i64() {
        let _rug_st_tests_llm_16_740_llm_16_739_rrrruuuugggg_test_serialize_i64 = 0;
        let rug_fuzz_0 = 42;
        let serializer = Serializer;
        let value = rug_fuzz_0;
        let result = serializer.serialize_i64(value).unwrap();
        debug_assert_eq!(result, Value::Number(value.into()));
        let _rug_ed_tests_llm_16_740_llm_16_739_rrrruuuugggg_test_serialize_i64 = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_772 {
    use crate::value::ser::Serializer;
    use crate::Value;
    use serde::Serializer as Ser;
    use serde::Serialize;
    use crate::error::Error;
    #[test]
    fn serialize_u64_test() -> Result<(), Error> {
        let value = Serializer.serialize_u64(123456789)?;
        let expected_result = Value::Number(123456789.into());
        assert_eq!(value, expected_result);
        Ok(())
    }
}
#[cfg(test)]
mod tests_rug_439 {
    use super::*;
    use crate::value::Serializer;
    use serde::Serializer as _;
    #[test]
    fn test_serialize_i32() {
        let _rug_st_tests_rug_439_rrrruuuugggg_test_serialize_i32 = 0;
        let rug_fuzz_0 = 42;
        let mut p0 = Serializer;
        let p1: i32 = rug_fuzz_0;
        p0.serialize_i32(p1).unwrap();
        let _rug_ed_tests_rug_439_rrrruuuugggg_test_serialize_i32 = 0;
    }
}
#[cfg(test)]
mod tests_rug_440 {
    use super::*;
    use crate::value::ser::Serializer;
    use serde::ser::Serializer as _;
    #[test]
    fn test_serialize_u8() {
        let _rug_st_tests_rug_440_rrrruuuugggg_test_serialize_u8 = 0;
        let rug_fuzz_0 = 42;
        let mut p0 = Serializer;
        let p1: u8 = rug_fuzz_0;
        p0.serialize_u8(p1).unwrap();
        let _rug_ed_tests_rug_440_rrrruuuugggg_test_serialize_u8 = 0;
    }
}
#[cfg(test)]
mod tests_rug_441 {
    use super::*;
    use crate::value::Serializer;
    use serde::Serializer as _;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_441_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0 = Serializer;
        let p1: u16 = rug_fuzz_0;
        p0.serialize_u16(p1);
        let _rug_ed_tests_rug_441_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_442 {
    use super::*;
    use crate::value::Serializer;
    use serde::Serializer as SerSerializer;
    use crate::error::Result;
    use crate::value::Value;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_442_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0 = Serializer;
        let p1: u32 = rug_fuzz_0;
        p0.serialize_u32(p1);
        let _rug_ed_tests_rug_442_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_454 {
    use super::*;
    use crate::value::Serializer;
    use serde::Serializer as _;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_454_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0 = Serializer;
        let mut p1: Option<usize> = Some(rug_fuzz_0);
        p0.serialize_seq(p1);
        let _rug_ed_tests_rug_454_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_465 {
    use super::*;
    use serde::ser::{SerializeSeq, SerializeTupleStruct};
    use crate::{Number, Value, Map};
    use crate::value::ser::SerializeVec;
    #[test]
    fn test_serialize_field() {
        let _rug_st_tests_rug_465_rrrruuuugggg_test_serialize_field = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let mut v75 = SerializeVec { vec: Vec::new() };
        let mut v29 = Value::Object(Map::new());
        v29[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
        let mut p0: SerializeVec = SerializeVec { vec: Vec::new() };
        let p1: &Value = &v29;
        p0.serialize_field(p1).unwrap();
        let _rug_ed_tests_rug_465_rrrruuuugggg_test_serialize_field = 0;
    }
}
#[cfg(test)]
mod tests_rug_466 {
    use super::*;
    use crate::Value;
    use crate::value::ser::SerializeVec;
    use serde::ser::SerializeSeq;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_466_rrrruuuugggg_test_rug = 0;
        let mut v75 = SerializeVec { vec: Vec::new() };
        let result: Result<Value> = SerializeSeq::end(v75);
        debug_assert!(result.is_ok());
        let _rug_ed_tests_rug_466_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_471 {
    use super::*;
    use serde::Serializer;
    use crate::value::ser::MapKeySerializer;
    #[test]
    fn test_serialize_unit_variant() {
        let _rug_st_tests_rug_471_rrrruuuugggg_test_serialize_unit_variant = 0;
        let rug_fuzz_0 = "name";
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = "variant";
        let mut p0: MapKeySerializer = MapKeySerializer;
        let p1: &'static str = rug_fuzz_0;
        let p2: u32 = rug_fuzz_1;
        let p3: &'static str = rug_fuzz_2;
        debug_assert_eq!(
            p0.serialize_unit_variant(p1, p2, p3).unwrap(), String::from("variant")
        );
        let _rug_ed_tests_rug_471_rrrruuuugggg_test_serialize_unit_variant = 0;
    }
}
#[cfg(test)]
mod tests_rug_475 {
    use super::*;
    use serde::Serializer;
    use crate::value::ser::MapKeySerializer;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_475_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut v74: MapKeySerializer = MapKeySerializer;
        let p0 = v74;
        let p1: i64 = rug_fuzz_0;
        debug_assert_eq!(p0.serialize_i64(p1).unwrap(), "42".to_string());
        let _rug_ed_tests_rug_475_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_476 {
    use super::*;
    use serde::Serializer;
    use crate::value::ser::MapKeySerializer;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_476_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: MapKeySerializer = MapKeySerializer;
        let p1: u16 = rug_fuzz_0;
        p0.serialize_u16(p1).unwrap();
        let _rug_ed_tests_rug_476_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_481 {
    use super::*;
    use serde::Serializer;
    use crate::value::ser::MapKeySerializer;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_481_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 'a';
        let mut p0: MapKeySerializer = MapKeySerializer;
        let mut p1: char = rug_fuzz_0;
        p0.serialize_char(p1);
        let _rug_ed_tests_rug_481_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_482 {
    use super::*;
    use serde::Serializer;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_482_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample_data";
        use serde::ser::SerializeMap;
        use crate::value::ser::MapKeySerializer;
        let mut p0: MapKeySerializer = MapKeySerializer;
        let p1: &str = rug_fuzz_0;
        p0.serialize_str(&p1);
        let _rug_ed_tests_rug_482_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_486 {
    use super::*;
    use serde::Serializer;
    use crate::value::ser::MapKeySerializer;
    #[test]
    fn test_serialize_none() {
        let _rug_st_tests_rug_486_rrrruuuugggg_test_serialize_none = 0;
        let mut p0: MapKeySerializer = MapKeySerializer;
        let result = <MapKeySerializer as Serializer>::serialize_none(p0);
        debug_assert!(result.is_err());
        let _rug_ed_tests_rug_486_rrrruuuugggg_test_serialize_none = 0;
    }
}
#[cfg(test)]
mod tests_rug_488 {
    use super::*;
    use serde::Serializer;
    use crate::value::ser::MapKeySerializer;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_488_rrrruuuugggg_test_rug = 0;
        let mut p0: MapKeySerializer = MapKeySerializer;
        let mut p1: Option<usize> = None;
        p0.serialize_seq(p1);
        let _rug_ed_tests_rug_488_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_490 {
    use super::*;
    use serde::Serializer;
    use crate::value::ser::MapKeySerializer;
    #[test]
    fn test_serialize_tuple_variant() {
        let _rug_st_tests_rug_490_rrrruuuugggg_test_serialize_tuple_variant = 0;
        let rug_fuzz_0 = "variant_name";
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = "variant_value";
        let rug_fuzz_3 = 2;
        let mut p0: MapKeySerializer = MapKeySerializer;
        let p1: &str = rug_fuzz_0;
        let p2: u32 = rug_fuzz_1;
        let p3: &str = rug_fuzz_2;
        let p4: usize = rug_fuzz_3;
        let result = p0.serialize_tuple_variant(&p1, p2, &p3, p4);
        debug_assert!(result.is_err());
        let _rug_ed_tests_rug_490_rrrruuuugggg_test_serialize_tuple_variant = 0;
    }
}
