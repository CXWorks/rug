use crate::error::Error;
use crate::lib::str::FromStr;
use crate::lib::*;
use crate::map::Map;
use crate::number::Number;
use crate::value::Value;
use serde::de::{
    self, Deserialize, DeserializeSeed, EnumAccess, Expected, IntoDeserializer,
    MapAccess, SeqAccess, Unexpected, VariantAccess, Visitor,
};
use serde::{forward_to_deserialize_any, serde_if_integer128};
#[cfg(feature = "arbitrary_precision")]
use crate::number::NumberFromString;
impl<'de> Deserialize<'de> for Value {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ValueVisitor;
        impl<'de> Visitor<'de> for ValueVisitor {
            type Value = Value;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("any valid JSON value")
            }
            #[inline]
            fn visit_bool<E>(self, value: bool) -> Result<Value, E> {
                Ok(Value::Bool(value))
            }
            #[inline]
            fn visit_i64<E>(self, value: i64) -> Result<Value, E> {
                Ok(Value::Number(value.into()))
            }
            #[inline]
            fn visit_u64<E>(self, value: u64) -> Result<Value, E> {
                Ok(Value::Number(value.into()))
            }
            #[inline]
            fn visit_f64<E>(self, value: f64) -> Result<Value, E> {
                Ok(Number::from_f64(value).map_or(Value::Null, Value::Number))
            }
            #[cfg(any(feature = "std", feature = "alloc"))]
            #[inline]
            fn visit_str<E>(self, value: &str) -> Result<Value, E>
            where
                E: serde::de::Error,
            {
                self.visit_string(String::from(value))
            }
            #[cfg(any(feature = "std", feature = "alloc"))]
            #[inline]
            fn visit_string<E>(self, value: String) -> Result<Value, E> {
                Ok(Value::String(value))
            }
            #[inline]
            fn visit_none<E>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }
            #[inline]
            fn visit_some<D>(self, deserializer: D) -> Result<Value, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                Deserialize::deserialize(deserializer)
            }
            #[inline]
            fn visit_unit<E>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }
            #[inline]
            fn visit_seq<V>(self, mut visitor: V) -> Result<Value, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let mut vec = Vec::new();
                while let Some(elem) = tri!(visitor.next_element()) {
                    vec.push(elem);
                }
                Ok(Value::Array(vec))
            }
            #[cfg(any(feature = "std", feature = "alloc"))]
            fn visit_map<V>(self, mut visitor: V) -> Result<Value, V::Error>
            where
                V: MapAccess<'de>,
            {
                match visitor.next_key_seed(KeyClassifier)? {
                    #[cfg(feature = "arbitrary_precision")]
                    Some(KeyClass::Number) => {
                        let number: NumberFromString = visitor.next_value()?;
                        Ok(Value::Number(number.value))
                    }
                    #[cfg(feature = "raw_value")]
                    Some(KeyClass::RawValue) => {
                        let value = visitor
                            .next_value_seed(crate::raw::BoxedFromString)?;
                        crate::from_str(value.get()).map_err(de::Error::custom)
                    }
                    Some(KeyClass::Map(first_key)) => {
                        let mut values = Map::new();
                        values.insert(first_key, tri!(visitor.next_value()));
                        while let Some((key, value)) = tri!(visitor.next_entry()) {
                            values.insert(key, value);
                        }
                        Ok(Value::Object(values))
                    }
                    None => Ok(Value::Object(Map::new())),
                }
            }
        }
        deserializer.deserialize_any(ValueVisitor)
    }
}
impl FromStr for Value {
    type Err = Error;
    fn from_str(s: &str) -> Result<Value, Error> {
        super::super::de::from_str(s)
    }
}
macro_rules! deserialize_number {
    ($method:ident) => {
        #[cfg(not(feature = "arbitrary_precision"))] fn $method < V > (self, visitor : V)
        -> Result < V::Value, Error > where V : Visitor <'de >, { match self {
        Value::Number(n) => n.deserialize_any(visitor), _ => Err(self.invalid_type(&
        visitor)), } } #[cfg(feature = "arbitrary_precision")] fn $method < V > (self,
        visitor : V) -> Result < V::Value, Error > where V : Visitor <'de >, { match self
        { Value::Number(n) => n.$method (visitor), _ => self.deserialize_any(visitor), }
        }
    };
}
fn visit_array<'de, V>(array: Vec<Value>, visitor: V) -> Result<V::Value, Error>
where
    V: Visitor<'de>,
{
    let len = array.len();
    let mut deserializer = SeqDeserializer::new(array);
    let seq = tri!(visitor.visit_seq(& mut deserializer));
    let remaining = deserializer.iter.len();
    if remaining == 0 {
        Ok(seq)
    } else {
        Err(serde::de::Error::invalid_length(len, &"fewer elements in array"))
    }
}
fn visit_object<'de, V>(
    object: Map<String, Value>,
    visitor: V,
) -> Result<V::Value, Error>
where
    V: Visitor<'de>,
{
    let len = object.len();
    let mut deserializer = MapDeserializer::new(object);
    let map = tri!(visitor.visit_map(& mut deserializer));
    let remaining = deserializer.iter.len();
    if remaining == 0 {
        Ok(map)
    } else {
        Err(serde::de::Error::invalid_length(len, &"fewer elements in map"))
    }
}
impl<'de> serde::Deserializer<'de> for Value {
    type Error = Error;
    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Null => visitor.visit_unit(),
            Value::Bool(v) => visitor.visit_bool(v),
            Value::Number(n) => n.deserialize_any(visitor),
            #[cfg(any(feature = "std", feature = "alloc"))]
            Value::String(v) => visitor.visit_string(v),
            Value::Array(v) => visit_array(v, visitor),
            Value::Object(v) => visit_object(v, visitor),
        }
    }
    deserialize_number!(deserialize_i8);
    deserialize_number!(deserialize_i16);
    deserialize_number!(deserialize_i32);
    deserialize_number!(deserialize_i64);
    deserialize_number!(deserialize_u8);
    deserialize_number!(deserialize_u16);
    deserialize_number!(deserialize_u32);
    deserialize_number!(deserialize_u64);
    deserialize_number!(deserialize_f32);
    deserialize_number!(deserialize_f64);
    serde_if_integer128! {
        deserialize_number!(deserialize_i128); deserialize_number!(deserialize_u128);
    }
    #[inline]
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Null => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }
    #[inline]
    fn deserialize_enum<V>(
        self,
        _name: &str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        let (variant, value) = match self {
            Value::Object(value) => {
                let mut iter = value.into_iter();
                let (variant, value) = match iter.next() {
                    Some(v) => v,
                    None => {
                        return Err(
                            serde::de::Error::invalid_value(
                                Unexpected::Map,
                                &"map with a single key",
                            ),
                        );
                    }
                };
                if iter.next().is_some() {
                    return Err(
                        serde::de::Error::invalid_value(
                            Unexpected::Map,
                            &"map with a single key",
                        ),
                    );
                }
                (variant, Some(value))
            }
            Value::String(variant) => (variant, None),
            other => {
                return Err(
                    serde::de::Error::invalid_type(other.unexpected(), &"string or map"),
                );
            }
        };
        visitor.visit_enum(EnumDeserializer { variant, value })
    }
    #[inline]
    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        #[cfg(feature = "raw_value")]
        {
            if name == crate::raw::TOKEN {
                return visitor
                    .visit_map(crate::raw::OwnedRawDeserializer {
                        raw_value: Some(self.to_string()),
                    });
            }
        }
        let _ = name;
        visitor.visit_newtype_struct(self)
    }
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Bool(v) => visitor.visit_bool(v),
            _ => Err(self.invalid_type(&visitor)),
        }
    }
    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            #[cfg(any(feature = "std", feature = "alloc"))]
            Value::String(v) => visitor.visit_string(v),
            _ => Err(self.invalid_type(&visitor)),
        }
    }
    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_byte_buf(visitor)
    }
    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            #[cfg(any(feature = "std", feature = "alloc"))]
            Value::String(v) => visitor.visit_string(v),
            Value::Array(v) => visit_array(v, visitor),
            _ => Err(self.invalid_type(&visitor)),
        }
    }
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Null => visitor.visit_unit(),
            _ => Err(self.invalid_type(&visitor)),
        }
    }
    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Array(v) => visit_array(v, visitor),
            _ => Err(self.invalid_type(&visitor)),
        }
    }
    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }
    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }
    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Object(v) => visit_object(v, visitor),
            _ => Err(self.invalid_type(&visitor)),
        }
    }
    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Array(v) => visit_array(v, visitor),
            Value::Object(v) => visit_object(v, visitor),
            _ => Err(self.invalid_type(&visitor)),
        }
    }
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        drop(self);
        visitor.visit_unit()
    }
}
struct EnumDeserializer {
    variant: String,
    value: Option<Value>,
}
impl<'de> EnumAccess<'de> for EnumDeserializer {
    type Error = Error;
    type Variant = VariantDeserializer;
    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, VariantDeserializer), Error>
    where
        V: DeserializeSeed<'de>,
    {
        let variant = self.variant.into_deserializer();
        let visitor = VariantDeserializer {
            value: self.value,
        };
        seed.deserialize(variant).map(|v| (v, visitor))
    }
}
impl<'de> IntoDeserializer<'de, Error> for Value {
    type Deserializer = Self;
    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}
struct VariantDeserializer {
    value: Option<Value>,
}
impl<'de> VariantAccess<'de> for VariantDeserializer {
    type Error = Error;
    fn unit_variant(self) -> Result<(), Error> {
        match self.value {
            Some(value) => Deserialize::deserialize(value),
            None => Ok(()),
        }
    }
    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.value {
            Some(value) => seed.deserialize(value),
            None => {
                Err(
                    serde::de::Error::invalid_type(
                        Unexpected::UnitVariant,
                        &"newtype variant",
                    ),
                )
            }
        }
    }
    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Some(Value::Array(v)) => {
                serde::Deserializer::deserialize_any(SeqDeserializer::new(v), visitor)
            }
            Some(other) => {
                Err(serde::de::Error::invalid_type(other.unexpected(), &"tuple variant"))
            }
            None => {
                Err(
                    serde::de::Error::invalid_type(
                        Unexpected::UnitVariant,
                        &"tuple variant",
                    ),
                )
            }
        }
    }
    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Some(Value::Object(v)) => {
                serde::Deserializer::deserialize_any(MapDeserializer::new(v), visitor)
            }
            Some(other) => {
                Err(
                    serde::de::Error::invalid_type(other.unexpected(), &"struct variant"),
                )
            }
            None => {
                Err(
                    serde::de::Error::invalid_type(
                        Unexpected::UnitVariant,
                        &"struct variant",
                    ),
                )
            }
        }
    }
}
struct SeqDeserializer {
    iter: vec::IntoIter<Value>,
}
impl SeqDeserializer {
    fn new(vec: Vec<Value>) -> Self {
        SeqDeserializer {
            iter: vec.into_iter(),
        }
    }
}
impl<'de> serde::Deserializer<'de> for SeqDeserializer {
    type Error = Error;
    #[inline]
    fn deserialize_any<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        let len = self.iter.len();
        if len == 0 {
            visitor.visit_unit()
        } else {
            let ret = tri!(visitor.visit_seq(& mut self));
            let remaining = self.iter.len();
            if remaining == 0 {
                Ok(ret)
            } else {
                Err(serde::de::Error::invalid_length(len, &"fewer elements in array"))
            }
        }
    }
    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes
        byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct map struct
        enum identifier ignored_any
    }
}
impl<'de> SeqAccess<'de> for SeqDeserializer {
    type Error = Error;
    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => seed.deserialize(value).map(Some),
            None => Ok(None),
        }
    }
    fn size_hint(&self) -> Option<usize> {
        match self.iter.size_hint() {
            (lower, Some(upper)) if lower == upper => Some(upper),
            _ => None,
        }
    }
}
struct MapDeserializer {
    iter: <Map<String, Value> as IntoIterator>::IntoIter,
    value: Option<Value>,
}
impl MapDeserializer {
    fn new(map: Map<String, Value>) -> Self {
        MapDeserializer {
            iter: map.into_iter(),
            value: None,
        }
    }
}
impl<'de> MapAccess<'de> for MapDeserializer {
    type Error = Error;
    fn next_key_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.value = Some(value);
                let key_de = MapKeyDeserializer {
                    key: Cow::Owned(key),
                };
                seed.deserialize(key_de).map(Some)
            }
            None => Ok(None),
        }
    }
    fn next_value_seed<T>(&mut self, seed: T) -> Result<T::Value, Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(value) => seed.deserialize(value),
            None => Err(serde::de::Error::custom("value is missing")),
        }
    }
    fn size_hint(&self) -> Option<usize> {
        match self.iter.size_hint() {
            (lower, Some(upper)) if lower == upper => Some(upper),
            _ => None,
        }
    }
}
impl<'de> serde::Deserializer<'de> for MapDeserializer {
    type Error = Error;
    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_map(self)
    }
    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes
        byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct map struct
        enum identifier ignored_any
    }
}
macro_rules! deserialize_value_ref_number {
    ($method:ident) => {
        #[cfg(not(feature = "arbitrary_precision"))] fn $method < V > (self, visitor : V)
        -> Result < V::Value, Error > where V : Visitor <'de >, { match * self {
        Value::Number(ref n) => n.deserialize_any(visitor), _ => Err(self.invalid_type(&
        visitor)), } } #[cfg(feature = "arbitrary_precision")] fn $method < V > (self,
        visitor : V) -> Result < V::Value, Error > where V : Visitor <'de >, { match *
        self { Value::Number(ref n) => n.$method (visitor), _ => self
        .deserialize_any(visitor), } }
    };
}
fn visit_array_ref<'de, V>(array: &'de [Value], visitor: V) -> Result<V::Value, Error>
where
    V: Visitor<'de>,
{
    let len = array.len();
    let mut deserializer = SeqRefDeserializer::new(array);
    let seq = tri!(visitor.visit_seq(& mut deserializer));
    let remaining = deserializer.iter.len();
    if remaining == 0 {
        Ok(seq)
    } else {
        Err(serde::de::Error::invalid_length(len, &"fewer elements in array"))
    }
}
fn visit_object_ref<'de, V>(
    object: &'de Map<String, Value>,
    visitor: V,
) -> Result<V::Value, Error>
where
    V: Visitor<'de>,
{
    let len = object.len();
    let mut deserializer = MapRefDeserializer::new(object);
    let map = tri!(visitor.visit_map(& mut deserializer));
    let remaining = deserializer.iter.len();
    if remaining == 0 {
        Ok(map)
    } else {
        Err(serde::de::Error::invalid_length(len, &"fewer elements in map"))
    }
}
impl<'de> serde::Deserializer<'de> for &'de Value {
    type Error = Error;
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match *self {
            Value::Null => visitor.visit_unit(),
            Value::Bool(v) => visitor.visit_bool(v),
            Value::Number(ref n) => n.deserialize_any(visitor),
            Value::String(ref v) => visitor.visit_borrowed_str(v),
            Value::Array(ref v) => visit_array_ref(v, visitor),
            Value::Object(ref v) => visit_object_ref(v, visitor),
        }
    }
    deserialize_value_ref_number!(deserialize_i8);
    deserialize_value_ref_number!(deserialize_i16);
    deserialize_value_ref_number!(deserialize_i32);
    deserialize_value_ref_number!(deserialize_i64);
    deserialize_value_ref_number!(deserialize_u8);
    deserialize_value_ref_number!(deserialize_u16);
    deserialize_value_ref_number!(deserialize_u32);
    deserialize_value_ref_number!(deserialize_u64);
    deserialize_value_ref_number!(deserialize_f32);
    deserialize_value_ref_number!(deserialize_f64);
    serde_if_integer128! {
        deserialize_number!(deserialize_i128); deserialize_number!(deserialize_u128);
    }
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match *self {
            Value::Null => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }
    fn deserialize_enum<V>(
        self,
        _name: &str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        let (variant, value) = match *self {
            Value::Object(ref value) => {
                let mut iter = value.into_iter();
                let (variant, value) = match iter.next() {
                    Some(v) => v,
                    None => {
                        return Err(
                            serde::de::Error::invalid_value(
                                Unexpected::Map,
                                &"map with a single key",
                            ),
                        );
                    }
                };
                if iter.next().is_some() {
                    return Err(
                        serde::de::Error::invalid_value(
                            Unexpected::Map,
                            &"map with a single key",
                        ),
                    );
                }
                (variant, Some(value))
            }
            Value::String(ref variant) => (variant, None),
            ref other => {
                return Err(
                    serde::de::Error::invalid_type(other.unexpected(), &"string or map"),
                );
            }
        };
        visitor
            .visit_enum(EnumRefDeserializer {
                variant,
                value,
            })
    }
    #[inline]
    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        #[cfg(feature = "raw_value")]
        {
            if name == crate::raw::TOKEN {
                return visitor
                    .visit_map(crate::raw::OwnedRawDeserializer {
                        raw_value: Some(self.to_string()),
                    });
            }
        }
        let _ = name;
        visitor.visit_newtype_struct(self)
    }
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match *self {
            Value::Bool(v) => visitor.visit_bool(v),
            _ => Err(self.invalid_type(&visitor)),
        }
    }
    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match *self {
            Value::String(ref v) => visitor.visit_borrowed_str(v),
            _ => Err(self.invalid_type(&visitor)),
        }
    }
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }
    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match *self {
            Value::String(ref v) => visitor.visit_borrowed_str(v),
            Value::Array(ref v) => visit_array_ref(v, visitor),
            _ => Err(self.invalid_type(&visitor)),
        }
    }
    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match *self {
            Value::Null => visitor.visit_unit(),
            _ => Err(self.invalid_type(&visitor)),
        }
    }
    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match *self {
            Value::Array(ref v) => visit_array_ref(v, visitor),
            _ => Err(self.invalid_type(&visitor)),
        }
    }
    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }
    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }
    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match *self {
            Value::Object(ref v) => visit_object_ref(v, visitor),
            _ => Err(self.invalid_type(&visitor)),
        }
    }
    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match *self {
            Value::Array(ref v) => visit_array_ref(v, visitor),
            Value::Object(ref v) => visit_object_ref(v, visitor),
            _ => Err(self.invalid_type(&visitor)),
        }
    }
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }
}
struct EnumRefDeserializer<'de> {
    variant: &'de str,
    value: Option<&'de Value>,
}
impl<'de> EnumAccess<'de> for EnumRefDeserializer<'de> {
    type Error = Error;
    type Variant = VariantRefDeserializer<'de>;
    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Error>
    where
        V: DeserializeSeed<'de>,
    {
        let variant = self.variant.into_deserializer();
        let visitor = VariantRefDeserializer {
            value: self.value,
        };
        seed.deserialize(variant).map(|v| (v, visitor))
    }
}
struct VariantRefDeserializer<'de> {
    value: Option<&'de Value>,
}
impl<'de> VariantAccess<'de> for VariantRefDeserializer<'de> {
    type Error = Error;
    fn unit_variant(self) -> Result<(), Error> {
        match self.value {
            Some(value) => Deserialize::deserialize(value),
            None => Ok(()),
        }
    }
    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.value {
            Some(value) => seed.deserialize(value),
            None => {
                Err(
                    serde::de::Error::invalid_type(
                        Unexpected::UnitVariant,
                        &"newtype variant",
                    ),
                )
            }
        }
    }
    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Some(&Value::Array(ref v)) => {
                serde::Deserializer::deserialize_any(SeqRefDeserializer::new(v), visitor)
            }
            Some(other) => {
                Err(serde::de::Error::invalid_type(other.unexpected(), &"tuple variant"))
            }
            None => {
                Err(
                    serde::de::Error::invalid_type(
                        Unexpected::UnitVariant,
                        &"tuple variant",
                    ),
                )
            }
        }
    }
    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Some(&Value::Object(ref v)) => {
                serde::Deserializer::deserialize_any(MapRefDeserializer::new(v), visitor)
            }
            Some(other) => {
                Err(
                    serde::de::Error::invalid_type(other.unexpected(), &"struct variant"),
                )
            }
            None => {
                Err(
                    serde::de::Error::invalid_type(
                        Unexpected::UnitVariant,
                        &"struct variant",
                    ),
                )
            }
        }
    }
}
struct SeqRefDeserializer<'de> {
    iter: slice::Iter<'de, Value>,
}
impl<'de> SeqRefDeserializer<'de> {
    fn new(slice: &'de [Value]) -> Self {
        SeqRefDeserializer {
            iter: slice.iter(),
        }
    }
}
impl<'de> serde::Deserializer<'de> for SeqRefDeserializer<'de> {
    type Error = Error;
    #[inline]
    fn deserialize_any<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        let len = self.iter.len();
        if len == 0 {
            visitor.visit_unit()
        } else {
            let ret = tri!(visitor.visit_seq(& mut self));
            let remaining = self.iter.len();
            if remaining == 0 {
                Ok(ret)
            } else {
                Err(serde::de::Error::invalid_length(len, &"fewer elements in array"))
            }
        }
    }
    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes
        byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct map struct
        enum identifier ignored_any
    }
}
impl<'de> SeqAccess<'de> for SeqRefDeserializer<'de> {
    type Error = Error;
    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => seed.deserialize(value).map(Some),
            None => Ok(None),
        }
    }
    fn size_hint(&self) -> Option<usize> {
        match self.iter.size_hint() {
            (lower, Some(upper)) if lower == upper => Some(upper),
            _ => None,
        }
    }
}
struct MapRefDeserializer<'de> {
    iter: <&'de Map<String, Value> as IntoIterator>::IntoIter,
    value: Option<&'de Value>,
}
impl<'de> MapRefDeserializer<'de> {
    fn new(map: &'de Map<String, Value>) -> Self {
        MapRefDeserializer {
            iter: map.into_iter(),
            value: None,
        }
    }
}
impl<'de> MapAccess<'de> for MapRefDeserializer<'de> {
    type Error = Error;
    fn next_key_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.value = Some(value);
                let key_de = MapKeyDeserializer {
                    key: Cow::Borrowed(&**key),
                };
                seed.deserialize(key_de).map(Some)
            }
            None => Ok(None),
        }
    }
    fn next_value_seed<T>(&mut self, seed: T) -> Result<T::Value, Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(value) => seed.deserialize(value),
            None => Err(serde::de::Error::custom("value is missing")),
        }
    }
    fn size_hint(&self) -> Option<usize> {
        match self.iter.size_hint() {
            (lower, Some(upper)) if lower == upper => Some(upper),
            _ => None,
        }
    }
}
impl<'de> serde::Deserializer<'de> for MapRefDeserializer<'de> {
    type Error = Error;
    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_map(self)
    }
    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes
        byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct map struct
        enum identifier ignored_any
    }
}
struct MapKeyDeserializer<'de> {
    key: Cow<'de, str>,
}
macro_rules! deserialize_integer_key {
    ($method:ident => $visit:ident) => {
        fn $method < V > (self, visitor : V) -> Result < V::Value, Error > where V :
        Visitor <'de >, { match (self.key.parse(), self.key) { (Ok(integer), _) =>
        visitor.$visit (integer), (Err(_), Cow::Borrowed(s)) => visitor
        .visit_borrowed_str(s), #[cfg(any(feature = "std", feature = "alloc"))] (Err(_),
        Cow::Owned(s)) => visitor.visit_string(s), } }
    };
}
impl<'de> serde::Deserializer<'de> for MapKeyDeserializer<'de> {
    type Error = Error;
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        BorrowedCowStrDeserializer::new(self.key).deserialize_any(visitor)
    }
    deserialize_integer_key!(deserialize_i8 => visit_i8);
    deserialize_integer_key!(deserialize_i16 => visit_i16);
    deserialize_integer_key!(deserialize_i32 => visit_i32);
    deserialize_integer_key!(deserialize_i64 => visit_i64);
    deserialize_integer_key!(deserialize_u8 => visit_u8);
    deserialize_integer_key!(deserialize_u16 => visit_u16);
    deserialize_integer_key!(deserialize_u32 => visit_u32);
    deserialize_integer_key!(deserialize_u64 => visit_u64);
    serde_if_integer128! {
        deserialize_integer_key!(deserialize_i128 => visit_i128);
        deserialize_integer_key!(deserialize_u128 => visit_u128);
    }
    #[inline]
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_some(self)
    }
    #[inline]
    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }
    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.key.into_deserializer().deserialize_enum(name, variants, visitor)
    }
    forward_to_deserialize_any! {
        bool f32 f64 char str string bytes byte_buf unit unit_struct seq tuple
        tuple_struct map struct identifier ignored_any
    }
}
struct KeyClassifier;
enum KeyClass {
    Map(String),
    #[cfg(feature = "arbitrary_precision")]
    Number,
    #[cfg(feature = "raw_value")]
    RawValue,
}
impl<'de> DeserializeSeed<'de> for KeyClassifier {
    type Value = KeyClass;
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(self)
    }
}
impl<'de> Visitor<'de> for KeyClassifier {
    type Value = KeyClass;
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string key")
    }
    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match s {
            #[cfg(feature = "arbitrary_precision")]
            crate::number::TOKEN => Ok(KeyClass::Number),
            #[cfg(feature = "raw_value")]
            crate::raw::TOKEN => Ok(KeyClass::RawValue),
            _ => Ok(KeyClass::Map(s.to_owned())),
        }
    }
    #[cfg(any(feature = "std", feature = "alloc"))]
    fn visit_string<E>(self, s: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match s.as_str() {
            #[cfg(feature = "arbitrary_precision")]
            crate::number::TOKEN => Ok(KeyClass::Number),
            #[cfg(feature = "raw_value")]
            crate::raw::TOKEN => Ok(KeyClass::RawValue),
            _ => Ok(KeyClass::Map(s)),
        }
    }
}
impl Value {
    #[cold]
    fn invalid_type<E>(&self, exp: &dyn Expected) -> E
    where
        E: serde::de::Error,
    {
        serde::de::Error::invalid_type(self.unexpected(), exp)
    }
    #[cold]
    fn unexpected(&self) -> Unexpected {
        match *self {
            Value::Null => Unexpected::Unit,
            Value::Bool(b) => Unexpected::Bool(b),
            Value::Number(ref n) => n.unexpected(),
            Value::String(ref s) => Unexpected::Str(s),
            Value::Array(_) => Unexpected::Seq,
            Value::Object(_) => Unexpected::Map,
        }
    }
}
struct BorrowedCowStrDeserializer<'de> {
    value: Cow<'de, str>,
}
impl<'de> BorrowedCowStrDeserializer<'de> {
    fn new(value: Cow<'de, str>) -> Self {
        BorrowedCowStrDeserializer {
            value,
        }
    }
}
impl<'de> de::Deserializer<'de> for BorrowedCowStrDeserializer<'de> {
    type Error = Error;
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            Cow::Borrowed(string) => visitor.visit_borrowed_str(string),
            #[cfg(any(feature = "std", feature = "alloc"))]
            Cow::Owned(string) => visitor.visit_string(string),
        }
    }
    fn deserialize_enum<V>(
        self,
        _name: &str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_enum(self)
    }
    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes
        byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct map struct
        identifier ignored_any
    }
}
impl<'de> de::EnumAccess<'de> for BorrowedCowStrDeserializer<'de> {
    type Error = Error;
    type Variant = UnitOnly;
    fn variant_seed<T>(self, seed: T) -> Result<(T::Value, Self::Variant), Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        let value = seed.deserialize(self)?;
        Ok((value, UnitOnly))
    }
}
struct UnitOnly;
impl<'de> de::VariantAccess<'de> for UnitOnly {
    type Error = Error;
    fn unit_variant(self) -> Result<(), Error> {
        Ok(())
    }
    fn newtype_variant_seed<T>(self, _seed: T) -> Result<T::Value, Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        Err(de::Error::invalid_type(Unexpected::UnitVariant, &"newtype variant"))
    }
    fn tuple_variant<V>(self, _len: usize, _visitor: V) -> Result<V::Value, Error>
    where
        V: de::Visitor<'de>,
    {
        Err(de::Error::invalid_type(Unexpected::UnitVariant, &"tuple variant"))
    }
    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: de::Visitor<'de>,
    {
        Err(de::Error::invalid_type(Unexpected::UnitVariant, &"struct variant"))
    }
}
#[cfg(test)]
mod tests_llm_16_560_llm_16_559 {
    use serde::de::{Error, Visitor};
    use crate::value::de::{KeyClass, KeyClassifier};
    use std::fmt::Write;
    use std::fmt::Formatter;
    #[test]
    #[cfg(feature = "fmt_internals")]
    #[cfg_attr(feature = "fmt_internals", feature(fmt_internals))]
    fn test_expecting() {
        let _rug_st_tests_llm_16_560_llm_16_559_rrrruuuugggg_test_expecting = 0;
        let mut formatter = Formatter::new(&mut String::new());
        let key_classifier = KeyClassifier;
        let result = key_classifier.expecting(&mut formatter);
        debug_assert!(result.is_ok());
        let _rug_ed_tests_llm_16_560_llm_16_559_rrrruuuugggg_test_expecting = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_571 {
    use super::*;
    use crate::*;
    use crate::{Value, Map};
    #[test]
    fn test_size_hint() {
        let _rug_st_tests_llm_16_571_rrrruuuugggg_test_size_hint = 0;
        let map: Map<String, Value> = Map::new();
        let map_deserializer = MapDeserializer::new(map);
        let result = map_deserializer.size_hint();
        debug_assert_eq!(result, None);
        let _rug_ed_tests_llm_16_571_rrrruuuugggg_test_size_hint = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_599 {
    use serde::de::StdError;
    use serde::de::Visitor;
    use serde::Deserializer;
    use crate::value::de::MapKeyDeserializer;
    use crate::Error;
    use std::borrow::Cow;
    use std::fmt;
    use std::str::FromStr;
    use std::result::Result;
    #[test]
    fn test_deserialize_u8() {
        struct TestVisitor;
        impl<'de> Visitor<'de> for TestVisitor {
            type Value = u8;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("an u8")
            }
            fn visit_u8<E>(self, value: u8) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(value)
            }
        }
        let key = Cow::Borrowed("42");
        let deserializer = MapKeyDeserializer { key: key };
        let visitor = TestVisitor;
        let result: Result<u8, Error> = deserializer.deserialize_u8(visitor);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }
}
#[cfg(test)]
mod tests_llm_16_606 {
    use super::*;
    use crate::*;
    use crate::Map;
    use crate::Value;
    use serde::de::MapAccess;
    use serde::de::DeserializeSeed;
    use serde::de::Error;
    use serde::de::Visitor;
    use std::borrow::Cow;
    use std::ops::Range;
    use std::iter::FusedIterator;
    use std::iter::Filter;
    use std::slice::Iter;
    use std::slice::IterMut;
    struct MapKeyDeserializer<'de> {
        key: Cow<'de, str>,
    }
    #[test]
    fn test_size_hint() {
        let _rug_st_tests_llm_16_606_rrrruuuugggg_test_size_hint = 0;
        let map: Map<String, Value> = Map::new();
        let deserializer = MapRefDeserializer::new(&map);
        let size_hint = deserializer.size_hint();
        debug_assert_eq!(size_hint, None);
        let _rug_ed_tests_llm_16_606_rrrruuuugggg_test_size_hint = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_612_llm_16_611 {
    use crate::value::de::SeqAccess;
    use crate::value::de::SeqDeserializer;
    use crate::value::Value;
    use serde::de::{DeserializeSeed, Error, Visitor};
    #[test]
    fn test_size_hint() {
        let _rug_st_tests_llm_16_612_llm_16_611_rrrruuuugggg_test_size_hint = 0;
        let rug_fuzz_0 = "hello";
        let vec = vec![
            Value::String(String::from(rug_fuzz_0)), Value::String(String::from("world"))
        ];
        let deserializer = SeqDeserializer::new(vec);
        let hint = deserializer.size_hint();
        debug_assert_eq!(hint, Some(2));
        let _rug_ed_tests_llm_16_612_llm_16_611_rrrruuuugggg_test_size_hint = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_618_llm_16_617 {
    use crate::value::de::SeqRefDeserializer;
    use crate::value::Value;
    use serde::de::SeqAccess;
    use serde::Deserializer;
    use crate::Number;
    #[test]
    fn test_size_hint() {
        let _rug_st_tests_llm_16_618_llm_16_617_rrrruuuugggg_test_size_hint = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = 42;
        let rug_fuzz_2 = "hello";
        let seq: &[Value] = &[
            Value::Bool(rug_fuzz_0),
            Value::Number(Number::from(rug_fuzz_1)),
            Value::String(rug_fuzz_2.to_owned()),
        ];
        let de = SeqRefDeserializer::new(seq);
        let hint = de.size_hint();
        debug_assert_eq!(hint, Some(3));
        let _rug_ed_tests_llm_16_618_llm_16_617_rrrruuuugggg_test_size_hint = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_622 {
    use serde::de::Unexpected;
    use crate::value::{Map, Value};
    use crate::error::Error;
    use serde::de::{self, Visitor, MapAccess, VariantAccess, DeserializeSeed};
    struct MapVisitor;
    impl<'de> Visitor<'de> for MapVisitor {
        type Value = Map<String, Value>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a map")
        }
        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Map::new())
        }
        #[cfg(any(feature = "std", feature = "alloc"))]
        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut values = Map::new();
            while let Some((key, value)) = map.next_entry()? {
                values.insert(key, value);
            }
            Ok(values)
        }
    }
    struct UnitOnly;
    impl<'de> VariantAccess<'de> for UnitOnly {
        type Error = Error;
        fn unit_variant(self) -> Result<(), Error> {
            Ok(())
        }
        fn newtype_variant_seed<T>(self, _seed: T) -> Result<T::Value, Error>
        where
            T: DeserializeSeed<'de>,
        {
            Err(de::Error::invalid_type(Unexpected::UnitVariant, &"newtype variant"))
        }
        fn tuple_variant<V>(self, _len: usize, _visitor: V) -> Result<V::Value, Error>
        where
            V: Visitor<'de>,
        {
            Err(de::Error::invalid_type(Unexpected::UnitVariant, &"tuple variant"))
        }
        fn struct_variant<V>(
            self,
            _fields: &'static [&'static str],
            _visitor: V,
        ) -> Result<V::Value, Error>
        where
            V: Visitor<'de>,
        {
            Err(de::Error::invalid_type(Unexpected::UnitVariant, &"struct variant"))
        }
    }
    #[test]
    fn test_struct_variant() {
        let _rug_st_tests_llm_16_622_rrrruuuugggg_test_struct_variant = 0;
        let unit_only = UnitOnly;
        let fields: &'static [&'static str] = &[];
        let visitor = MapVisitor;
        let result: Result<Map<String, Value>, Error> = unit_only
            .struct_variant(fields, visitor);
        let _rug_ed_tests_llm_16_622_rrrruuuugggg_test_struct_variant = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_633 {
    use serde::de::VariantAccess;
    use crate::value::de::{Error, Value, VariantDeserializer};
    use serde::Deserialize;
    #[test]
    fn test_unit_variant() {
        let _rug_st_tests_llm_16_633_rrrruuuugggg_test_unit_variant = 0;
        let value = Some(Value::Null);
        let variant_deserializer = VariantDeserializer { value };
        let result: Result<(), Error> = variant_deserializer.unit_variant();
        debug_assert!(result.is_ok());
        debug_assert_eq!(result.unwrap(), ());
        let _rug_ed_tests_llm_16_633_rrrruuuugggg_test_unit_variant = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_641_llm_16_640 {
    use crate::value::Value;
    use serde::de::VariantAccess;
    use serde::Deserialize;
    use serde::de::Error;
    use serde::de::Unexpected;
    use serde::de::Visitor;
    use serde::de::DeserializeSeed;
    use serde::Deserializer;
    #[test]
    fn test_unit_variant() {
        let _rug_st_tests_llm_16_641_llm_16_640_rrrruuuugggg_test_unit_variant = 0;
        let value: Option<&Value> = None;
        let var = crate::value::de::VariantRefDeserializer {
            value,
        };
        let result = var.unit_variant();
        debug_assert!(result.is_ok());
        let value = Some(&Value::Null);
        let var = crate::value::de::VariantRefDeserializer {
            value,
        };
        let result = var.unit_variant();
        debug_assert!(result.is_err());
        let _rug_ed_tests_llm_16_641_llm_16_640_rrrruuuugggg_test_unit_variant = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_1042 {
    use crate::value::Value;
    use crate::error::Error;
    use crate::de::from_str;
    #[test]
    fn test_from_str() {
        let _rug_st_tests_llm_16_1042_rrrruuuugggg_test_from_str = 0;
        let rug_fuzz_0 = r#"{ "name": "John", "age": 30, "city": "New York" }"#;
        let json_str = rug_fuzz_0;
        let result: Result<Value, Error> = from_str(json_str);
        debug_assert!(result.is_ok());
        let _rug_ed_tests_llm_16_1042_rrrruuuugggg_test_from_str = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_1045 {
    use crate::error::Category;
    use crate::map::Map;
    use crate::value::Value;
    use crate::value::de::MapDeserializer;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_1045_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = "value";
        let mut map = Map::new();
        map.insert(rug_fuzz_0.to_string(), Value::String(rug_fuzz_1.to_string()));
        let deserializer = MapDeserializer::new(map);
        let _rug_ed_tests_llm_16_1045_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_rug_246 {
    use super::*;
    use serde::de::{self, Visitor};
    use serde::Deserialize;
    use crate::{Number, Value};
    struct NumberVisitor;
    impl<'de> Visitor<'de> for NumberVisitor {
        type Value = Number;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a JSON number")
        }
        #[inline]
        fn visit_i64<E>(self, value: i64) -> Result<Number, E> {
            Ok(value.into())
        }
        #[inline]
        fn visit_u64<E>(self, value: u64) -> Result<Number, E> {
            Ok(value.into())
        }
        #[inline]
        fn visit_f64<E>(self, value: f64) -> Result<Number, E>
        where
            E: de::Error,
        {
            Number::from_f64(value).ok_or_else(|| de::Error::custom("not a JSON number"))
        }
    }
    #[test]
    fn test_visit_array() {
        let _rug_st_tests_rug_246_rrrruuuugggg_test_visit_array = 0;
        let p0: Vec<Value> = Vec::<Value>::new();
        let p1 = NumberVisitor;
        crate::value::de::visit_array(p0, p1).unwrap();
        let _rug_ed_tests_rug_246_rrrruuuugggg_test_visit_array = 0;
    }
}
#[cfg(test)]
mod tests_rug_247 {
    use super::*;
    use serde::de::{self, Visitor};
    use crate::{Map, Number, Value};
    #[test]
    fn test_visit_object() {
        let mut p0: Map<String, Value> = Map::new();
        p0.insert("key1".to_string(), Value::String("value1".to_string()));
        p0.insert("key2".to_string(), Value::Number(Number::from(42)));
        struct NumberVisitor;
        impl<'de> Visitor<'de> for NumberVisitor {
            type Value = Number;
            fn expecting(
                &self,
                formatter: &mut std::fmt::Formatter,
            ) -> std::fmt::Result {
                formatter.write_str("a JSON number")
            }
            #[inline]
            fn visit_i64<E>(self, value: i64) -> Result<Number, E> {
                Ok(value.into())
            }
            #[inline]
            fn visit_u64<E>(self, value: u64) -> Result<Number, E> {
                Ok(value.into())
            }
            #[inline]
            fn visit_f64<E>(self, value: f64) -> Result<Number, E>
            where
                E: de::Error,
            {
                Number::from_f64(value)
                    .ok_or_else(|| de::Error::custom("not a JSON number"))
            }
        }
        let mut p1 = NumberVisitor;
        crate::value::de::visit_object(p0, p1);
    }
}
#[cfg(test)]
mod tests_rug_248 {
    use super::*;
    use crate::value::Value;
    use serde::de::{self, Visitor};
    use serde::Deserialize;
    use crate::Number;
    struct NumberVisitor;
    impl<'de> Visitor<'de> for NumberVisitor {
        type Value = Number;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a JSON number")
        }
        #[inline]
        fn visit_i64<E>(self, value: i64) -> Result<Number, E> {
            Ok(value.into())
        }
        #[inline]
        fn visit_u64<E>(self, value: u64) -> Result<Number, E> {
            Ok(value.into())
        }
        #[inline]
        fn visit_f64<E>(self, value: f64) -> Result<Number, E>
        where
            E: de::Error,
        {
            Number::from_f64(value).ok_or_else(|| de::Error::custom("not a JSON number"))
        }
    }
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_248_rrrruuuugggg_test_rug = 0;
        let mut p0: &[Value] = &[];
        let mut p1 = NumberVisitor;
        crate::value::de::visit_array_ref(p0, p1);
        let _rug_ed_tests_rug_248_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_266 {
    use super::*;
    use serde::de::{self, Visitor};
    use serde::Deserialize;
    use crate::{Number, Value, Map};
    struct NumberVisitor;
    impl<'de> Visitor<'de> for NumberVisitor {
        type Value = Number;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a JSON number")
        }
        #[inline]
        fn visit_i64<E>(self, value: i64) -> Result<Number, E> {
            Ok(value.into())
        }
        #[inline]
        fn visit_u64<E>(self, value: u64) -> Result<Number, E> {
            Ok(value.into())
        }
        #[inline]
        fn visit_f64<E>(self, value: f64) -> Result<Number, E>
        where
            E: de::Error,
        {
            Number::from_f64(value).ok_or_else(|| de::Error::custom("not a JSON number"))
        }
    }
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_266_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let mut v29 = Value::Object(Map::new());
        v29[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
        let mut v14 = NumberVisitor;
        <Value as serde::Deserializer<'_>>::deserialize_i32(v29, v14);
        let _rug_ed_tests_rug_266_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_267 {
    use super::*;
    use serde::de::Visitor;
    use serde::Deserialize;
    use crate::{Number, Value, Map};
    struct NumberVisitor;
    impl<'de> Visitor<'de> for NumberVisitor {
        type Value = Number;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a JSON number")
        }
        #[inline]
        fn visit_i64<E>(self, value: i64) -> Result<Number, E> {
            Ok(value.into())
        }
        #[inline]
        fn visit_u64<E>(self, value: u64) -> Result<Number, E> {
            Ok(value.into())
        }
        #[inline]
        fn visit_f64<E>(self, value: f64) -> Result<Number, E>
        where
            E: serde::de::Error,
        {
            Number::from_f64(value)
                .ok_or_else(|| serde::de::Error::custom("not a JSON number"))
        }
    }
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_267_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let mut v29 = Value::Object(Map::new());
        v29[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
        let mut v14 = NumberVisitor;
        <Value as serde::Deserializer>::deserialize_i64(v29, v14);
        let _rug_ed_tests_rug_267_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_273 {
    use super::*;
    use serde::de::{self, Visitor};
    use serde::Deserializer;
    use crate::{Number, Value, Map};
    struct NumberVisitor;
    impl<'de> Visitor<'de> for NumberVisitor {
        type Value = Number;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a JSON number")
        }
        #[inline]
        fn visit_i64<E>(self, value: i64) -> Result<Number, E> {
            Ok(value.into())
        }
        #[inline]
        fn visit_u64<E>(self, value: u64) -> Result<Number, E> {
            Ok(value.into())
        }
        #[inline]
        fn visit_f64<E>(self, value: f64) -> Result<Number, E>
        where
            E: de::Error,
        {
            Number::from_f64(value).ok_or_else(|| de::Error::custom("not a JSON number"))
        }
    }
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_273_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let mut v29 = Value::Object(Map::new());
        v29[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
        let mut v14 = NumberVisitor;
        <Value as Deserializer>::deserialize_f64(v29, v14);
        let _rug_ed_tests_rug_273_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_293 {
    use super::*;
    use crate::{Number, Value, Map};
    use serde::Deserializer;
    use serde::de::{self, Visitor};
    struct NumberVisitor;
    impl<'de> Visitor<'de> for NumberVisitor {
        type Value = Number;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a JSON number")
        }
        #[inline]
        fn visit_i64<E>(self, value: i64) -> Result<Number, E> {
            Ok(value.into())
        }
        #[inline]
        fn visit_u64<E>(self, value: u64) -> Result<Number, E> {
            Ok(value.into())
        }
        #[inline]
        fn visit_f64<E>(self, value: f64) -> Result<Number, E>
        where
            E: de::Error,
        {
            Number::from_f64(value).ok_or_else(|| de::Error::custom("not a JSON number"))
        }
    }
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_293_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let mut p0 = Value::Object(Map::new());
        p0[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
        let mut p1 = NumberVisitor;
        p0.deserialize_ignored_any(p1);
        let _rug_ed_tests_rug_293_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_295 {
    use super::*;
    use crate::de::Deserializer;
    use crate::{Number, Value, Map};
    #[test]
    fn test_into_deserializer() {
        let _rug_st_tests_rug_295_rrrruuuugggg_test_into_deserializer = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let mut v29 = Value::Object(Map::new());
        v29[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
        let p0: Value = v29;
        p0.into_deserializer();
        let _rug_ed_tests_rug_295_rrrruuuugggg_test_into_deserializer = 0;
    }
}
#[cfg(test)]
mod tests_rug_299 {
    use super::*;
    use crate::value::de::{SeqDeserializer, Value};
    use std::vec::Vec;
    #[test]
    fn test_new() {
        let _rug_st_tests_rug_299_rrrruuuugggg_test_new = 0;
        let mut p0: Vec<Value> = Vec::new();
        SeqDeserializer::new(p0);
        let _rug_ed_tests_rug_299_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_rug_308 {
    use super::*;
    use serde::de::{self, Visitor};
    use serde::Deserializer;
    use crate::{Number, Value, Map};
    struct NumberVisitor;
    impl<'de> Visitor<'de> for NumberVisitor {
        type Value = Number;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a JSON number")
        }
        #[inline]
        fn visit_i64<E>(self, value: i64) -> Result<Number, E> {
            Ok(value.into())
        }
        #[inline]
        fn visit_u64<E>(self, value: u64) -> Result<Number, E> {
            Ok(value.into())
        }
        #[inline]
        fn visit_f64<E>(self, value: f64) -> Result<Number, E>
        where
            E: de::Error,
        {
            Number::from_f64(value).ok_or_else(|| de::Error::custom("not a JSON number"))
        }
    }
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_308_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let mut p0 = Value::Object(Map::new());
        p0[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
        let mut p1 = NumberVisitor;
        p0.deserialize_i32(p1);
        let _rug_ed_tests_rug_308_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_340 {
    use super::*;
    use crate::value::Value;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_340_rrrruuuugggg_test_rug = 0;
        let mut p0: &[Value] = &[];
        crate::value::de::SeqRefDeserializer::<'static>::new(p0);
        let _rug_ed_tests_rug_340_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_343 {
    use super::*;
    use std::collections::BTreeMap;
    use crate::{Map, Value};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_343_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key1";
        let rug_fuzz_1 = 42;
        let rug_fuzz_2 = "key2";
        let rug_fuzz_3 = "value";
        let mut p0: Map<String, Value> = Map::new();
        p0.insert(String::from(rug_fuzz_0), Value::from(rug_fuzz_1));
        p0.insert(String::from(rug_fuzz_2), Value::from(rug_fuzz_3));
        crate::value::de::MapRefDeserializer::new(&p0);
        let _rug_ed_tests_rug_343_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_364 {
    use super::*;
    use crate::{Value, Number, Map};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_364_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let mut v29 = Value::Object(Map::new());
        v29[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
        let p0: Value = v29;
        p0.unexpected();
        let _rug_ed_tests_rug_364_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_365 {
    use super::*;
    use crate::value::de::BorrowedCowStrDeserializer;
    use std::borrow::Cow;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_365_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "Hello, World!";
        let mut p0: Cow<'_, str> = Cow::Borrowed(rug_fuzz_0);
        BorrowedCowStrDeserializer::<'static>::new(p0);
        let _rug_ed_tests_rug_365_rrrruuuugggg_test_rug = 0;
    }
}
