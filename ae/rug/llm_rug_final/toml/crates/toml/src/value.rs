//! Definition of a TOML [value][Value]

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::hash::Hash;
use std::mem::discriminant;
use std::ops;
use std::vec;

use serde::de;
use serde::de::IntoDeserializer;
use serde::ser;

use toml_datetime::__unstable as datetime;
pub use toml_datetime::{Date, Datetime, DatetimeParseError, Offset, Time};

/// Type representing a TOML array, payload of the `Value::Array` variant
pub type Array = Vec<Value>;

#[doc(no_inline)]
pub use crate::Table;

/// Representation of a TOML value.
#[derive(PartialEq, Clone, Debug)]
pub enum Value {
    /// Represents a TOML string
    String(String),
    /// Represents a TOML integer
    Integer(i64),
    /// Represents a TOML float
    Float(f64),
    /// Represents a TOML boolean
    Boolean(bool),
    /// Represents a TOML datetime
    Datetime(Datetime),
    /// Represents a TOML array
    Array(Array),
    /// Represents a TOML table
    Table(Table),
}

impl Value {
    /// Convert a `T` into `toml::Value` which is an enum that can represent
    /// any valid TOML data.
    ///
    /// This conversion can fail if `T`'s implementation of `Serialize` decides to
    /// fail, or if `T` contains a map with non-string keys.
    pub fn try_from<T>(value: T) -> Result<Value, crate::ser::Error>
    where
        T: ser::Serialize,
    {
        value.serialize(ValueSerializer)
    }

    /// Interpret a `toml::Value` as an instance of type `T`.
    ///
    /// This conversion can fail if the structure of the `Value` does not match the
    /// structure expected by `T`, for example if `T` is a struct type but the
    /// `Value` contains something other than a TOML table. It can also fail if the
    /// structure is correct but `T`'s implementation of `Deserialize` decides that
    /// something is wrong with the data, for example required struct fields are
    /// missing from the TOML map or some number is too big to fit in the expected
    /// primitive type.
    pub fn try_into<'de, T>(self) -> Result<T, crate::de::Error>
    where
        T: de::Deserialize<'de>,
    {
        de::Deserialize::deserialize(self)
    }

    /// Index into a TOML array or map. A string index can be used to access a
    /// value in a map, and a usize index can be used to access an element of an
    /// array.
    ///
    /// Returns `None` if the type of `self` does not match the type of the
    /// index, for example if the index is a string and `self` is an array or a
    /// number. Also returns `None` if the given key does not exist in the map
    /// or the given index is not within the bounds of the array.
    pub fn get<I: Index>(&self, index: I) -> Option<&Value> {
        index.index(self)
    }

    /// Mutably index into a TOML array or map. A string index can be used to
    /// access a value in a map, and a usize index can be used to access an
    /// element of an array.
    ///
    /// Returns `None` if the type of `self` does not match the type of the
    /// index, for example if the index is a string and `self` is an array or a
    /// number. Also returns `None` if the given key does not exist in the map
    /// or the given index is not within the bounds of the array.
    pub fn get_mut<I: Index>(&mut self, index: I) -> Option<&mut Value> {
        index.index_mut(self)
    }

    /// Extracts the integer value if it is an integer.
    pub fn as_integer(&self) -> Option<i64> {
        match *self {
            Value::Integer(i) => Some(i),
            _ => None,
        }
    }

    /// Tests whether this value is an integer.
    pub fn is_integer(&self) -> bool {
        self.as_integer().is_some()
    }

    /// Extracts the float value if it is a float.
    pub fn as_float(&self) -> Option<f64> {
        match *self {
            Value::Float(f) => Some(f),
            _ => None,
        }
    }

    /// Tests whether this value is a float.
    pub fn is_float(&self) -> bool {
        self.as_float().is_some()
    }

    /// Extracts the boolean value if it is a boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            Value::Boolean(b) => Some(b),
            _ => None,
        }
    }

    /// Tests whether this value is a boolean.
    pub fn is_bool(&self) -> bool {
        self.as_bool().is_some()
    }

    /// Extracts the string of this value if it is a string.
    pub fn as_str(&self) -> Option<&str> {
        match *self {
            Value::String(ref s) => Some(&**s),
            _ => None,
        }
    }

    /// Tests if this value is a string.
    pub fn is_str(&self) -> bool {
        self.as_str().is_some()
    }

    /// Extracts the datetime value if it is a datetime.
    ///
    /// Note that a parsed TOML value will only contain ISO 8601 dates. An
    /// example date is:
    ///
    /// ```notrust
    /// 1979-05-27T07:32:00Z
    /// ```
    pub fn as_datetime(&self) -> Option<&Datetime> {
        match *self {
            Value::Datetime(ref s) => Some(s),
            _ => None,
        }
    }

    /// Tests whether this value is a datetime.
    pub fn is_datetime(&self) -> bool {
        self.as_datetime().is_some()
    }

    /// Extracts the array value if it is an array.
    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match *self {
            Value::Array(ref s) => Some(s),
            _ => None,
        }
    }

    /// Extracts the array value if it is an array.
    pub fn as_array_mut(&mut self) -> Option<&mut Vec<Value>> {
        match *self {
            Value::Array(ref mut s) => Some(s),
            _ => None,
        }
    }

    /// Tests whether this value is an array.
    pub fn is_array(&self) -> bool {
        self.as_array().is_some()
    }

    /// Extracts the table value if it is a table.
    pub fn as_table(&self) -> Option<&Table> {
        match *self {
            Value::Table(ref s) => Some(s),
            _ => None,
        }
    }

    /// Extracts the table value if it is a table.
    pub fn as_table_mut(&mut self) -> Option<&mut Table> {
        match *self {
            Value::Table(ref mut s) => Some(s),
            _ => None,
        }
    }

    /// Tests whether this value is a table.
    pub fn is_table(&self) -> bool {
        self.as_table().is_some()
    }

    /// Tests whether this and another value have the same type.
    pub fn same_type(&self, other: &Value) -> bool {
        discriminant(self) == discriminant(other)
    }

    /// Returns a human-readable representation of the type of this value.
    pub fn type_str(&self) -> &'static str {
        match *self {
            Value::String(..) => "string",
            Value::Integer(..) => "integer",
            Value::Float(..) => "float",
            Value::Boolean(..) => "boolean",
            Value::Datetime(..) => "datetime",
            Value::Array(..) => "array",
            Value::Table(..) => "table",
        }
    }
}

impl<I> ops::Index<I> for Value
where
    I: Index,
{
    type Output = Value;

    fn index(&self, index: I) -> &Value {
        self.get(index).expect("index not found")
    }
}

impl<I> ops::IndexMut<I> for Value
where
    I: Index,
{
    fn index_mut(&mut self, index: I) -> &mut Value {
        self.get_mut(index).expect("index not found")
    }
}

impl<'a> From<&'a str> for Value {
    #[inline]
    fn from(val: &'a str) -> Value {
        Value::String(val.to_string())
    }
}

impl<V: Into<Value>> From<Vec<V>> for Value {
    fn from(val: Vec<V>) -> Value {
        Value::Array(val.into_iter().map(|v| v.into()).collect())
    }
}

impl<S: Into<String>, V: Into<Value>> From<BTreeMap<S, V>> for Value {
    fn from(val: BTreeMap<S, V>) -> Value {
        let table = val.into_iter().map(|(s, v)| (s.into(), v.into())).collect();

        Value::Table(table)
    }
}

impl<S: Into<String> + Hash + Eq, V: Into<Value>> From<HashMap<S, V>> for Value {
    fn from(val: HashMap<S, V>) -> Value {
        let table = val.into_iter().map(|(s, v)| (s.into(), v.into())).collect();

        Value::Table(table)
    }
}

macro_rules! impl_into_value {
    ($variant:ident : $T:ty) => {
        impl From<$T> for Value {
            #[inline]
            fn from(val: $T) -> Value {
                Value::$variant(val.into())
            }
        }
    };
}

impl_into_value!(String: String);
impl_into_value!(Integer: i64);
impl_into_value!(Integer: i32);
impl_into_value!(Integer: i8);
impl_into_value!(Integer: u8);
impl_into_value!(Integer: u32);
impl_into_value!(Float: f64);
impl_into_value!(Float: f32);
impl_into_value!(Boolean: bool);
impl_into_value!(Datetime: Datetime);
impl_into_value!(Table: Table);

/// Types that can be used to index a `toml::Value`
///
/// Currently this is implemented for `usize` to index arrays and `str` to index
/// tables.
///
/// This trait is sealed and not intended for implementation outside of the
/// `toml` crate.
pub trait Index: Sealed {
    #[doc(hidden)]
    fn index<'a>(&self, val: &'a Value) -> Option<&'a Value>;
    #[doc(hidden)]
    fn index_mut<'a>(&self, val: &'a mut Value) -> Option<&'a mut Value>;
}

/// An implementation detail that should not be implemented, this will change in
/// the future and break code otherwise.
#[doc(hidden)]
pub trait Sealed {}
impl Sealed for usize {}
impl Sealed for str {}
impl Sealed for String {}
impl<'a, T: Sealed + ?Sized> Sealed for &'a T {}

impl Index for usize {
    fn index<'a>(&self, val: &'a Value) -> Option<&'a Value> {
        match *val {
            Value::Array(ref a) => a.get(*self),
            _ => None,
        }
    }

    fn index_mut<'a>(&self, val: &'a mut Value) -> Option<&'a mut Value> {
        match *val {
            Value::Array(ref mut a) => a.get_mut(*self),
            _ => None,
        }
    }
}

impl Index for str {
    fn index<'a>(&self, val: &'a Value) -> Option<&'a Value> {
        match *val {
            Value::Table(ref a) => a.get(self),
            _ => None,
        }
    }

    fn index_mut<'a>(&self, val: &'a mut Value) -> Option<&'a mut Value> {
        match *val {
            Value::Table(ref mut a) => a.get_mut(self),
            _ => None,
        }
    }
}

impl Index for String {
    fn index<'a>(&self, val: &'a Value) -> Option<&'a Value> {
        self[..].index(val)
    }

    fn index_mut<'a>(&self, val: &'a mut Value) -> Option<&'a mut Value> {
        self[..].index_mut(val)
    }
}

impl<'s, T: ?Sized> Index for &'s T
where
    T: Index,
{
    fn index<'a>(&self, val: &'a Value) -> Option<&'a Value> {
        (**self).index(val)
    }

    fn index_mut<'a>(&self, val: &'a mut Value) -> Option<&'a mut Value> {
        (**self).index_mut(val)
    }
}

#[cfg(feature = "display")]
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use serde::Serialize as _;

        let mut output = String::new();
        let serializer = crate::ser::ValueSerializer::new(&mut output);
        self.serialize(serializer).unwrap();
        output.fmt(f)
    }
}

#[cfg(feature = "parse")]
impl std::str::FromStr for Value {
    type Err = crate::de::Error;
    fn from_str(s: &str) -> Result<Value, Self::Err> {
        crate::from_str(s)
    }
}

impl ser::Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        use serde::ser::SerializeMap;

        match *self {
            Value::String(ref s) => serializer.serialize_str(s),
            Value::Integer(i) => serializer.serialize_i64(i),
            Value::Float(f) => serializer.serialize_f64(f),
            Value::Boolean(b) => serializer.serialize_bool(b),
            Value::Datetime(ref s) => s.serialize(serializer),
            Value::Array(ref a) => a.serialize(serializer),
            Value::Table(ref t) => {
                let mut map = serializer.serialize_map(Some(t.len()))?;
                // Be sure to visit non-tables first (and also non
                // array-of-tables) as all keys must be emitted first.
                for (k, v) in t {
                    if !v.is_table() && !v.is_array()
                        || (v
                            .as_array()
                            .map(|a| !a.iter().any(|v| v.is_table()))
                            .unwrap_or(false))
                    {
                        map.serialize_entry(k, v)?;
                    }
                }
                for (k, v) in t {
                    if v.as_array()
                        .map(|a| a.iter().any(|v| v.is_table()))
                        .unwrap_or(false)
                    {
                        map.serialize_entry(k, v)?;
                    }
                }
                for (k, v) in t {
                    if v.is_table() {
                        map.serialize_entry(k, v)?;
                    }
                }
                map.end()
            }
        }
    }
}

impl<'de> de::Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct ValueVisitor;

        impl<'de> de::Visitor<'de> for ValueVisitor {
            type Value = Value;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("any valid TOML value")
            }

            fn visit_bool<E>(self, value: bool) -> Result<Value, E> {
                Ok(Value::Boolean(value))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Value, E> {
                Ok(Value::Integer(value))
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Value, E> {
                if value <= i64::max_value() as u64 {
                    Ok(Value::Integer(value as i64))
                } else {
                    Err(de::Error::custom("u64 value was too large"))
                }
            }

            fn visit_u32<E>(self, value: u32) -> Result<Value, E> {
                Ok(Value::Integer(value.into()))
            }

            fn visit_i32<E>(self, value: i32) -> Result<Value, E> {
                Ok(Value::Integer(value.into()))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Value, E> {
                Ok(Value::Float(value))
            }

            fn visit_str<E>(self, value: &str) -> Result<Value, E> {
                Ok(Value::String(value.into()))
            }

            fn visit_string<E>(self, value: String) -> Result<Value, E> {
                Ok(Value::String(value))
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Value, D::Error>
            where
                D: de::Deserializer<'de>,
            {
                de::Deserialize::deserialize(deserializer)
            }

            fn visit_seq<V>(self, mut visitor: V) -> Result<Value, V::Error>
            where
                V: de::SeqAccess<'de>,
            {
                let mut vec = Vec::new();
                while let Some(elem) = visitor.next_element()? {
                    vec.push(elem);
                }
                Ok(Value::Array(vec))
            }

            fn visit_map<V>(self, mut visitor: V) -> Result<Value, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                let mut key = String::new();
                let datetime = visitor.next_key_seed(DatetimeOrTable { key: &mut key })?;
                match datetime {
                    Some(true) => {
                        let date: datetime::DatetimeFromString = visitor.next_value()?;
                        return Ok(Value::Datetime(date.value));
                    }
                    None => return Ok(Value::Table(Table::new())),
                    Some(false) => {}
                }
                let mut map = Table::new();
                map.insert(key, visitor.next_value()?);
                while let Some(key) = visitor.next_key::<String>()? {
                    if let crate::map::Entry::Vacant(vacant) = map.entry(&key) {
                        vacant.insert(visitor.next_value()?);
                    } else {
                        let msg = format!("duplicate key: `{}`", key);
                        return Err(de::Error::custom(msg));
                    }
                }
                Ok(Value::Table(map))
            }
        }

        deserializer.deserialize_any(ValueVisitor)
    }
}

// This is wrapped by `Table` and any trait methods implemented here need to be wrapped there.
impl<'de> de::Deserializer<'de> for Value {
    type Error = crate::de::Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, crate::de::Error>
    where
        V: de::Visitor<'de>,
    {
        match self {
            Value::Boolean(v) => visitor.visit_bool(v),
            Value::Integer(n) => visitor.visit_i64(n),
            Value::Float(n) => visitor.visit_f64(n),
            Value::String(v) => visitor.visit_string(v),
            Value::Datetime(v) => visitor.visit_string(v.to_string()),
            Value::Array(v) => {
                let len = v.len();
                let mut deserializer = SeqDeserializer::new(v);
                let seq = visitor.visit_seq(&mut deserializer)?;
                let remaining = deserializer.iter.len();
                if remaining == 0 {
                    Ok(seq)
                } else {
                    Err(de::Error::invalid_length(len, &"fewer elements in array"))
                }
            }
            Value::Table(v) => {
                let len = v.len();
                let mut deserializer = MapDeserializer::new(v);
                let map = visitor.visit_map(&mut deserializer)?;
                let remaining = deserializer.iter.len();
                if remaining == 0 {
                    Ok(map)
                } else {
                    Err(de::Error::invalid_length(len, &"fewer elements in map"))
                }
            }
        }
    }

    #[inline]
    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, crate::de::Error>
    where
        V: de::Visitor<'de>,
    {
        match self {
            Value::String(variant) => visitor.visit_enum(variant.into_deserializer()),
            Value::Table(variant) => {
                use de::Error;
                if variant.is_empty() {
                    Err(crate::de::Error::custom(
                        "wanted exactly 1 element, found 0 elements",
                    ))
                } else if variant.len() != 1 {
                    Err(crate::de::Error::custom(
                        "wanted exactly 1 element, more than 1 element",
                    ))
                } else {
                    let deserializer = MapDeserializer::new(variant);
                    visitor.visit_enum(deserializer)
                }
            }
            _ => Err(de::Error::invalid_type(
                de::Unexpected::UnitVariant,
                &"string only",
            )),
        }
    }

    // `None` is interpreted as a missing field so be sure to implement `Some`
    // as a present field.
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, crate::de::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, crate::de::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    serde::forward_to_deserialize_any! {
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string unit seq
        bytes byte_buf map unit_struct tuple_struct struct
        tuple ignored_any identifier
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

impl<'de> de::SeqAccess<'de> for SeqDeserializer {
    type Error = crate::de::Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, crate::de::Error>
    where
        T: de::DeserializeSeed<'de>,
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
    iter: <Table as IntoIterator>::IntoIter,
    value: Option<(String, Value)>,
}

impl MapDeserializer {
    fn new(map: Table) -> Self {
        MapDeserializer {
            iter: map.into_iter(),
            value: None,
        }
    }
}

impl<'de> de::MapAccess<'de> for MapDeserializer {
    type Error = crate::de::Error;

    fn next_key_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, crate::de::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.value = Some((key.clone(), value));
                seed.deserialize(Value::String(key)).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<T>(&mut self, seed: T) -> Result<T::Value, crate::de::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        let (key, res) = match self.value.take() {
            Some((key, value)) => (key, seed.deserialize(value)),
            None => return Err(de::Error::custom("value is missing")),
        };
        res.map_err(|mut error| {
            error.add_key(key);
            error
        })
    }

    fn size_hint(&self) -> Option<usize> {
        match self.iter.size_hint() {
            (lower, Some(upper)) if lower == upper => Some(upper),
            _ => None,
        }
    }
}

impl<'de> de::EnumAccess<'de> for MapDeserializer {
    type Error = crate::de::Error;
    type Variant = MapEnumDeserializer;

    fn variant_seed<V>(mut self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        use de::Error;
        let (key, value) = match self.iter.next() {
            Some(pair) => pair,
            None => {
                return Err(Error::custom(
                    "expected table with exactly 1 entry, found empty table",
                ));
            }
        };

        let val = seed.deserialize(key.into_deserializer())?;

        let variant = MapEnumDeserializer::new(value);

        Ok((val, variant))
    }
}

/// Deserializes table values into enum variants.
pub(crate) struct MapEnumDeserializer {
    value: Value,
}

impl MapEnumDeserializer {
    pub(crate) fn new(value: Value) -> Self {
        MapEnumDeserializer { value }
    }
}

impl<'de> serde::de::VariantAccess<'de> for MapEnumDeserializer {
    type Error = crate::de::Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        use de::Error;
        match self.value {
            Value::Table(values) => {
                if values.is_empty() {
                    Ok(())
                } else {
                    Err(Error::custom("expected empty table"))
                }
            }
            e => Err(Error::custom(format!(
                "expected table, found {}",
                e.type_str()
            ))),
        }
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        seed.deserialize(self.value.into_deserializer())
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        use de::Error;
        match self.value {
            Value::Table(values) => {
                let tuple_values = values
                    .into_iter()
                    .enumerate()
                    .map(|(index, (key, value))| match key.parse::<usize>() {
                        Ok(key_index) if key_index == index => Ok(value),
                        Ok(_) | Err(_) => Err(Error::custom(format!(
                            "expected table key `{}`, but was `{}`",
                            index, key
                        ))),
                    })
                    // Fold all values into a `Vec`, or return the first error.
                    .fold(Ok(Vec::with_capacity(len)), |result, value_result| {
                        result.and_then(move |mut tuple_values| match value_result {
                            Ok(value) => {
                                tuple_values.push(value);
                                Ok(tuple_values)
                            }
                            // `Result<de::Value, Self::Error>` to `Result<Vec<_>, Self::Error>`
                            Err(e) => Err(e),
                        })
                    })?;

                if tuple_values.len() == len {
                    serde::de::Deserializer::deserialize_seq(
                        tuple_values.into_deserializer(),
                        visitor,
                    )
                } else {
                    Err(Error::custom(format!("expected tuple with length {}", len)))
                }
            }
            e => Err(Error::custom(format!(
                "expected table, found {}",
                e.type_str()
            ))),
        }
    }

    fn struct_variant<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        serde::de::Deserializer::deserialize_struct(
            self.value.into_deserializer(),
            "", // TODO: this should be the variant name
            fields,
            visitor,
        )
    }
}

impl<'de> de::IntoDeserializer<'de, crate::de::Error> for Value {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self {
        self
    }
}

struct ValueSerializer;

impl ser::Serializer for ValueSerializer {
    type Ok = Value;
    type Error = crate::ser::Error;

    type SerializeSeq = ValueSerializeVec;
    type SerializeTuple = ValueSerializeVec;
    type SerializeTupleStruct = ValueSerializeVec;
    type SerializeTupleVariant = ValueSerializeVec;
    type SerializeMap = ValueSerializeMap;
    type SerializeStruct = ValueSerializeMap;
    type SerializeStructVariant = ser::Impossible<Value, crate::ser::Error>;

    fn serialize_bool(self, value: bool) -> Result<Value, crate::ser::Error> {
        Ok(Value::Boolean(value))
    }

    fn serialize_i8(self, value: i8) -> Result<Value, crate::ser::Error> {
        self.serialize_i64(value.into())
    }

    fn serialize_i16(self, value: i16) -> Result<Value, crate::ser::Error> {
        self.serialize_i64(value.into())
    }

    fn serialize_i32(self, value: i32) -> Result<Value, crate::ser::Error> {
        self.serialize_i64(value.into())
    }

    fn serialize_i64(self, value: i64) -> Result<Value, crate::ser::Error> {
        Ok(Value::Integer(value))
    }

    fn serialize_u8(self, value: u8) -> Result<Value, crate::ser::Error> {
        self.serialize_i64(value.into())
    }

    fn serialize_u16(self, value: u16) -> Result<Value, crate::ser::Error> {
        self.serialize_i64(value.into())
    }

    fn serialize_u32(self, value: u32) -> Result<Value, crate::ser::Error> {
        self.serialize_i64(value.into())
    }

    fn serialize_u64(self, value: u64) -> Result<Value, crate::ser::Error> {
        if value <= i64::max_value() as u64 {
            self.serialize_i64(value as i64)
        } else {
            Err(ser::Error::custom("u64 value was too large"))
        }
    }

    fn serialize_f32(self, value: f32) -> Result<Value, crate::ser::Error> {
        self.serialize_f64(value.into())
    }

    fn serialize_f64(self, value: f64) -> Result<Value, crate::ser::Error> {
        Ok(Value::Float(value))
    }

    fn serialize_char(self, value: char) -> Result<Value, crate::ser::Error> {
        let mut s = String::new();
        s.push(value);
        self.serialize_str(&s)
    }

    fn serialize_str(self, value: &str) -> Result<Value, crate::ser::Error> {
        Ok(Value::String(value.to_owned()))
    }

    fn serialize_bytes(self, value: &[u8]) -> Result<Value, crate::ser::Error> {
        let vec = value.iter().map(|&b| Value::Integer(b.into())).collect();
        Ok(Value::Array(vec))
    }

    fn serialize_unit(self) -> Result<Value, crate::ser::Error> {
        Err(crate::ser::Error::unsupported_type(Some("unit")))
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Value, crate::ser::Error> {
        Err(crate::ser::Error::unsupported_type(Some(name)))
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Value, crate::ser::Error> {
        self.serialize_str(_variant)
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Value, crate::ser::Error>
    where
        T: ser::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Value, crate::ser::Error>
    where
        T: ser::Serialize,
    {
        let value = value.serialize(ValueSerializer)?;
        let mut table = Table::new();
        table.insert(variant.to_owned(), value);
        Ok(table.into())
    }

    fn serialize_none(self) -> Result<Value, crate::ser::Error> {
        Err(crate::ser::Error::unsupported_none())
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Value, crate::ser::Error>
    where
        T: ser::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, crate::ser::Error> {
        Ok(ValueSerializeVec {
            vec: Vec::with_capacity(len.unwrap_or(0)),
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, crate::ser::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, crate::ser::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, crate::ser::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, crate::ser::Error> {
        Ok(ValueSerializeMap {
            ser: SerializeMap {
                map: Table::new(),
                next_key: None,
            },
        })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, crate::ser::Error> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, crate::ser::Error> {
        Err(crate::ser::Error::unsupported_type(Some(name)))
    }
}

pub(crate) struct TableSerializer;

impl ser::Serializer for TableSerializer {
    type Ok = Table;
    type Error = crate::ser::Error;

    type SerializeSeq = ser::Impossible<Table, crate::ser::Error>;
    type SerializeTuple = ser::Impossible<Table, crate::ser::Error>;
    type SerializeTupleStruct = ser::Impossible<Table, crate::ser::Error>;
    type SerializeTupleVariant = ser::Impossible<Table, crate::ser::Error>;
    type SerializeMap = SerializeMap;
    type SerializeStruct = SerializeMap;
    type SerializeStructVariant = ser::Impossible<Table, crate::ser::Error>;

    fn serialize_bool(self, _value: bool) -> Result<Table, crate::ser::Error> {
        Err(crate::ser::Error::unsupported_type(None))
    }

    fn serialize_i8(self, _value: i8) -> Result<Table, crate::ser::Error> {
        Err(crate::ser::Error::unsupported_type(None))
    }

    fn serialize_i16(self, _value: i16) -> Result<Table, crate::ser::Error> {
        Err(crate::ser::Error::unsupported_type(None))
    }

    fn serialize_i32(self, _value: i32) -> Result<Table, crate::ser::Error> {
        Err(crate::ser::Error::unsupported_type(None))
    }

    fn serialize_i64(self, _value: i64) -> Result<Table, crate::ser::Error> {
        Err(crate::ser::Error::unsupported_type(None))
    }

    fn serialize_u8(self, _value: u8) -> Result<Table, crate::ser::Error> {
        Err(crate::ser::Error::unsupported_type(None))
    }

    fn serialize_u16(self, _value: u16) -> Result<Table, crate::ser::Error> {
        Err(crate::ser::Error::unsupported_type(None))
    }

    fn serialize_u32(self, _value: u32) -> Result<Table, crate::ser::Error> {
        Err(crate::ser::Error::unsupported_type(None))
    }

    fn serialize_u64(self, _value: u64) -> Result<Table, crate::ser::Error> {
        Err(crate::ser::Error::unsupported_type(None))
    }

    fn serialize_f32(self, _value: f32) -> Result<Table, crate::ser::Error> {
        Err(crate::ser::Error::unsupported_type(None))
    }

    fn serialize_f64(self, _value: f64) -> Result<Table, crate::ser::Error> {
        Err(crate::ser::Error::unsupported_type(None))
    }

    fn serialize_char(self, _value: char) -> Result<Table, crate::ser::Error> {
        Err(crate::ser::Error::unsupported_type(None))
    }

    fn serialize_str(self, _value: &str) -> Result<Table, crate::ser::Error> {
        Err(crate::ser::Error::unsupported_type(None))
    }

    fn serialize_bytes(self, _value: &[u8]) -> Result<Table, crate::ser::Error> {
        Err(crate::ser::Error::unsupported_type(None))
    }

    fn serialize_unit(self) -> Result<Table, crate::ser::Error> {
        Err(crate::ser::Error::unsupported_type(None))
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Table, crate::ser::Error> {
        Err(crate::ser::Error::unsupported_type(None))
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Table, crate::ser::Error> {
        Err(crate::ser::Error::unsupported_type(Some(name)))
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Table, crate::ser::Error>
    where
        T: ser::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Table, crate::ser::Error>
    where
        T: ser::Serialize,
    {
        let value = value.serialize(ValueSerializer)?;
        let mut table = Table::new();
        table.insert(variant.to_owned(), value);
        Ok(table)
    }

    fn serialize_none(self) -> Result<Table, crate::ser::Error> {
        Err(crate::ser::Error::unsupported_none())
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Table, crate::ser::Error>
    where
        T: ser::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, crate::ser::Error> {
        Err(crate::ser::Error::unsupported_type(None))
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, crate::ser::Error> {
        Err(crate::ser::Error::unsupported_type(None))
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, crate::ser::Error> {
        Err(crate::ser::Error::unsupported_type(Some(name)))
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, crate::ser::Error> {
        Err(crate::ser::Error::unsupported_type(Some(name)))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, crate::ser::Error> {
        Ok(SerializeMap {
            map: Table::new(),
            next_key: None,
        })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, crate::ser::Error> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, crate::ser::Error> {
        Err(crate::ser::Error::unsupported_type(Some(name)))
    }
}

struct ValueSerializeVec {
    vec: Vec<Value>,
}

impl ser::SerializeSeq for ValueSerializeVec {
    type Ok = Value;
    type Error = crate::ser::Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), crate::ser::Error>
    where
        T: ser::Serialize,
    {
        self.vec.push(Value::try_from(value)?);
        Ok(())
    }

    fn end(self) -> Result<Value, crate::ser::Error> {
        Ok(Value::Array(self.vec))
    }
}

impl ser::SerializeTuple for ValueSerializeVec {
    type Ok = Value;
    type Error = crate::ser::Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), crate::ser::Error>
    where
        T: ser::Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Value, crate::ser::Error> {
        ser::SerializeSeq::end(self)
    }
}

impl ser::SerializeTupleStruct for ValueSerializeVec {
    type Ok = Value;
    type Error = crate::ser::Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), crate::ser::Error>
    where
        T: ser::Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Value, crate::ser::Error> {
        ser::SerializeSeq::end(self)
    }
}

impl ser::SerializeTupleVariant for ValueSerializeVec {
    type Ok = Value;
    type Error = crate::ser::Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), crate::ser::Error>
    where
        T: ser::Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Value, crate::ser::Error> {
        ser::SerializeSeq::end(self)
    }
}

pub(crate) struct SerializeMap {
    map: Table,
    next_key: Option<String>,
}

impl ser::SerializeMap for SerializeMap {
    type Ok = Table;
    type Error = crate::ser::Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), crate::ser::Error>
    where
        T: ser::Serialize,
    {
        match Value::try_from(key)? {
            Value::String(s) => self.next_key = Some(s),
            _ => return Err(crate::ser::Error::key_not_string()),
        };
        Ok(())
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), crate::ser::Error>
    where
        T: ser::Serialize,
    {
        let key = self.next_key.take();
        let key = key.expect("serialize_value called before serialize_key");
        match Value::try_from(value) {
            Ok(value) => {
                self.map.insert(key, value);
            }
            Err(crate::ser::Error {
                inner: crate::edit::ser::Error::UnsupportedNone,
            }) => {}
            Err(e) => return Err(e),
        }
        Ok(())
    }

    fn end(self) -> Result<Table, crate::ser::Error> {
        Ok(self.map)
    }
}

impl ser::SerializeStruct for SerializeMap {
    type Ok = Table;
    type Error = crate::ser::Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), crate::ser::Error>
    where
        T: ser::Serialize,
    {
        ser::SerializeMap::serialize_key(self, key)?;
        ser::SerializeMap::serialize_value(self, value)
    }

    fn end(self) -> Result<Table, crate::ser::Error> {
        ser::SerializeMap::end(self)
    }
}

struct ValueSerializeMap {
    ser: SerializeMap,
}

impl ser::SerializeMap for ValueSerializeMap {
    type Ok = Value;
    type Error = crate::ser::Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), crate::ser::Error>
    where
        T: ser::Serialize,
    {
        self.ser.serialize_key(key)
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), crate::ser::Error>
    where
        T: ser::Serialize,
    {
        self.ser.serialize_value(value)
    }

    fn end(self) -> Result<Value, crate::ser::Error> {
        self.ser.end().map(Value::Table)
    }
}

impl ser::SerializeStruct for ValueSerializeMap {
    type Ok = Value;
    type Error = crate::ser::Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), crate::ser::Error>
    where
        T: ser::Serialize,
    {
        ser::SerializeMap::serialize_key(self, key)?;
        ser::SerializeMap::serialize_value(self, value)
    }

    fn end(self) -> Result<Value, crate::ser::Error> {
        ser::SerializeMap::end(self)
    }
}

struct DatetimeOrTable<'a> {
    key: &'a mut String,
}

impl<'a, 'de> de::DeserializeSeed<'de> for DatetimeOrTable<'a> {
    type Value = bool;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }
}

impl<'a, 'de> de::Visitor<'de> for DatetimeOrTable<'a> {
    type Value = bool;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a string key")
    }

    fn visit_str<E>(self, s: &str) -> Result<bool, E>
    where
        E: de::Error,
    {
        if s == datetime::FIELD {
            Ok(true)
        } else {
            self.key.push_str(s);
            Ok(false)
        }
    }

    fn visit_string<E>(self, s: String) -> Result<bool, E>
    where
        E: de::Error,
    {
        if s == datetime::FIELD {
            Ok(true)
        } else {
            *self.key = s;
            Ok(false)
        }
    }
}
#[cfg(test)]
mod tests_rug_159 {
    use super::*;
    use serde::Serialize;
    
    #[derive(Serialize)]
    struct MyStruct {
        name: String,
        age: usize,
    }
    
    #[test]
    fn test_try_from() {
        let my_struct = MyStruct {
            name: "John".to_owned(),
            age: 30,
        };
        
        let result = Value::try_from(my_struct);
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Table({
            let mut table = Table::new();
            table.insert("name".to_owned(), Value::String("John".to_owned()));
            table.insert("age".to_owned(), Value::Integer(30));
            table
        }));
    }
}
#[cfg(test)]
mod tests_rug_160 {
    use super::*;
    use crate::value::Value;
    
    #[test]
    fn test_rug() {
        let p0: Value = Value::from("test value");
        
        crate::value::Value::try_into::<Value>(p0).unwrap();

    }
}
#[cfg(test)]
mod tests_rug_161 {
    use super::*;
    use std::string::String;
    use crate::value::Value;
    use crate::de;
    
    #[test]
    fn test_get() {
        let mut p0: Value = de::from_str("123").unwrap();
        let mut p1: String = String::from("key");
        
        p0.get(p1);
    }
}#[cfg(test)]
mod tests_rug_162 {
    use super::*;
    use crate::value::Value;
    use std::string::String;
    
    #[test]
    fn test_rug() {
        let mut p0 = Value::from(5);
        let p1: String = String::from("key");
        
        p0.get_mut(p1);
    }
}
#[cfg(test)]
mod tests_rug_163 {
    use super::*;
    use crate::value::Value;

    #[test]
    fn test_rug() {
        let mut p0 = Value::from(42);
        assert_eq!(p0.as_integer(), Some(42));

        p0 = Value::from("hello");
        assert_eq!(p0.as_integer(), None);
    }
}
#[cfg(test)]
mod tests_rug_164 {
    use super::*;
    use crate::value;

    #[test]
    fn test_is_integer() {
        let p0 = value::Value::from(42);

        assert!(p0.is_integer());
    }
}                        
        #[cfg(test)]
        mod tests_rug_165 {
            use super::*;
            use crate::value::Value;
            
            #[test]
            fn test_rug() {
                let mut p0 = Value::from(3.14); 

                <Value>::as_float(&p0);

            }
        }
                            #[cfg(test)]
mod tests_rug_166 {
    use super::*;
    
    #[test]
    fn test_rug() {
        let p0 = Value::from(42.0);
        
        assert_eq!(p0.is_float(), true);
    }
}        
        #[cfg(test)]
        mod tests_rug_167 {
            use super::*;
            use crate::value::Value;
            
            #[test]
            fn test_rug() {
                let mut p0 = Value::Boolean(true);

                
                Value::as_bool(&p0);

            }
        }
                            
#[cfg(test)]
mod tests_rug_168 {
    use super::*;
    use crate::value::Value;

    #[test]
    fn test_is_bool() {
        let p0: Value = Value::from(true);
                
        assert_eq!(p0.is_bool(), true);
    }
}

#[cfg(test)]
mod tests_rug_169 {
    use super::*;
    use crate::value::Value;

    #[test]
    fn test_rug() {
        let mut p0: Value = Value::from("test");

        <Value>::as_str(&p0);
    }
}

#[cfg(test)]
mod tests_rug_170 {
    use super::*;
    use crate::ser::Serializer;
    use crate::value::{self, Value};
    use serde::{Deserialize, Serialize};
    
    #[derive(Debug, Serialize, Deserialize)]
    struct Person {
        name: String,
        age: u32,
        address: String,
    }
    
    #[test]
    fn test_is_str() {
        let p0 = Value::from("hello");
        
        assert!(p0.is_str());
        
        let p1 = Value::from(123);
        
        assert!(!p1.is_str());
        
        let p2 = Value::from(true);
        
        assert!(!p2.is_str());
        
        let p3: serde_json::Value = serde_json::from_str(r#""hello""#).unwrap();
        
        let p4 = value::Value::try_from(p3).unwrap();
        
        assert!(p4.is_str());
    }
}

#[cfg(test)]
mod tests_rug_172 {
    use super::*;
    use crate::de::Error as SerdeError;
    use crate::value;
    
    #[test]
    fn test_is_datetime() {
        let mut p0 = Value::from("2021-02-25T12:00:00Z");
        assert_eq!(p0.is_datetime(), true);
        
        let p1 = Value::from(123);
        assert_eq!(p1.is_datetime(), false);
        
        let p2 = Value::from("not a datetime");
        assert_eq!(p2.is_datetime(), false);
    }
}
#[cfg(test)]
mod tests_rug_173 {
    use super::*;
    use serde::de::DeserializeOwned;
    use crate::value::Value;

    #[test]
    fn test_rug() {
        let mut p0: Value = serde_json::from_str("{}").unwrap();

        Value::as_array(&p0);
    }
}#[cfg(test)]
mod tests_rug_174 {
    use super::*;
    use crate::value::{Value, Array};

    #[test]
    fn test_as_array_mut() {
        let mut p0: Value = Array::new().into();

        p0.as_array_mut();

    }
}
#[cfg(test)]
mod tests_rug_175 {
    use super::*;
    use crate::value::Value;

    #[test]
    fn test_is_array() {
        let mut p0 = Value::from(vec![1, 2, 3]);

        assert_eq!(p0.is_array(), true);
    }
}
#[cfg(test)]
mod tests_rug_176 {
    use super::*;
    use crate::value::{Table, Value};

    #[test]
    fn test_rug() {
        let mut p0: Value = Value::Table(Table::new());

        Value::as_table(&p0);
    }
}
#[cfg(test)]
mod tests_rug_177 {
    use super::*;
    use crate::value::{Value, Table};

    #[test]
    fn test_rug() {
        let mut p0 = Value::Table(Table::new());

        Value::as_table_mut(&mut p0);
    }
}

#[cfg(test)]
mod tests_rug_178 {
    use super::*;
    use crate::value::Value;

    #[test]
    fn test_is_table() {
        let mut p0 = Value::try_from(0).unwrap();

        Value::is_table(&p0);
    }
}
#[cfg(test)]
mod tests_rug_179 {
    use super::*;
    use crate::de::Error;
    use std::convert::TryFrom;

    #[test]
    fn test_same_type() {
        // Construct p0 and p1 variables
        let p0 = Value::from(0_i32);
        let p1 = Value::try_from(0_u32).unwrap();

        // Call the target function
        let result = Value::same_type(&p0, &p1);

        // Assert the result
        assert_eq!(result, true);
    }
}#[cfg(test)]
mod tests_rug_180 {
    use super::*;
    use crate::value::Value;

    #[test]
    fn test_type_str() {
        let p0 = Value::String(String::from("hello"));

        assert_eq!(<Value>::type_str(&p0), "string");
    }
}#[cfg(test)]
mod tests_rug_183 {
    use super::*;
    use crate::value::Value;
    
    #[test]
    fn test_from() {
        let p0: &str = "..."; // Sample input for the argument
        
        Value::from(p0);
    }
}#[cfg(test)]
mod tests_rug_185 {
    use super::*;
    use crate::value::Value;
    use std::collections::BTreeMap;
    
    #[test]
    fn test_from() {
        let mut p0: BTreeMap<String, Value> = BTreeMap::new();
        p0.insert("key1".to_string(), Value::Integer(10));
        p0.insert("key2".to_string(), Value::String("value2".to_string()));
        p0.insert("key3".to_string(), Value::Boolean(true));
        
        let _ = <Value as std::convert::From<BTreeMap<String, Value>>>::from(p0);
    }
}                        
#[cfg(test)]
mod tests_rug_187 {
    use super::*;
    use std::convert::From;
    use crate::value::Value;

    #[test]
    fn test_rug() {
        let p0: std::string::String = "Hello, World!".to_string();

        <Value as std::convert::From<std::string::String>>::from(p0);
    }
}#[cfg(test)]
mod tests_rug_188 {
    use super::*;
    use crate::value::Value;
    #[test]
    fn test_from() {
        let p0: i64 = 42;
        <Value as std::convert::From<i64>>::from(p0);
    }
}#[cfg(test)]
mod tests_rug_189 {
    use super::*;
    use crate::value::Value;
    use std::convert::From;

    #[test]
    fn test_toml_from() {
        let p0: i32 = 42;

        <Value as std::convert::From<i32>>::from(p0);
    }
}#[cfg(test)]
mod tests_rug_190 {
    use super::*;
    use crate::value::Value;

    #[test]
    fn test_from() {
        let p0: i8 = 42;

        <Value as std::convert::From<i8>>::from(p0);
    }
}#[cfg(test)]
mod tests_rug_191 {
    use super::*;
    use crate::value::Value;
    use std::convert::From;
    
    #[test]
    fn test_from() {
        let mut p0: u8 = 42;
        let result: Value = <Value as std::convert::From<u8>>::from(p0);
        // write assertion here
        // assert_eq!(result, ...);
    }
}        
        #[cfg(test)]
        mod tests_rug_192 {
            use super::*;
            use crate::value::Value;

            #[test]
            fn test_from() {
                let p0: u32 = 42;

                <Value as std::convert::From<u32>>::from(p0);

            }
        }
#[cfg(test)]
mod tests_rug_193 {
    use super::*;
    use crate::value::Value;
    use std::convert::From;

    #[test]
    fn test_from() {
        let p0: f64 = 3.14;
        
        <Value as std::convert::From<f64>>::from(p0);
    }
}#[cfg(test)]
mod tests_rug_194 {
    use super::*;
    use crate::value::Value;

    #[test]
    fn test_from() {
        let p0: f32 = 3.14;

        <Value as std::convert::From<f32>>::from(p0);
    }
}#[cfg(test)]
mod tests_rug_195 {
    use super::*;
    use crate::value::Value;
    use std::convert::From;
    
    #[test]
    fn test_from_bool_to_value() {
        let p0: bool = true;
        
        <Value as From<bool>>::from(p0);
    }
}
#[cfg(test)]
mod tests_rug_198 {
    use super::*;
    use crate::value::Index;
    use crate::value::Value;

    #[test]
    fn test_index() {
        let p0: usize = 0;
        let p1: Value = Value::Array(vec![Value::Integer(1), Value::Integer(2), Value::Integer(3)]);

        <usize as Index>::index(&p0, &p1);
    }
}
#[cfg(test)]
mod tests_rug_199 {
    use super::*;
    use crate::value::Index;
    use crate::value::Value;

    #[test]
    fn test_rug() {
        let mut p0: usize = 0;
        let mut p1: Value = Value::from(123);

        p0.index_mut(&mut p1);
    }
}
#[cfg(test)]
mod tests_rug_200 {
    use super::*;
    use crate::value::Index;
    use crate::value::Value;
    
    #[test]
    fn test_index() {
        let p0: &str = "key"; 
        let p1: Value = Value::Table(Table::new());
 
        p0.index(&p1);
    }
}
#[cfg(test)]
mod tests_rug_201 {
    use super::*;
    use crate::value::Index;
    use crate::value::Value;

    #[test]
    fn test_index_mut() {
        let mut p0: &str = "key";
        let mut p1: &mut Value = &mut Value::try_from("sample").unwrap();

        <str as Index>::index_mut(&p0, p1);
    }
}
#[cfg(test)]
mod tests_rug_202 {
    use super::*;
    use crate::value::Index;
    use crate::value::Value;
    
    #[test]
    fn test_index() {
        let mut p0 = String::from("sample_string");
        let p1 = Value::from("sample_value");
        
        <String as Index>::index(&p0, &p1);
    }
}


#[cfg(test)]
mod tests_rug_203 {
    use super::*;
    use crate::value::Index;
    use crate::value::Value;
    
    #[test]
    fn test_rug() {
        let mut p0: std::string::String = "example".to_string();
        let mut p1: Value = Value::from(10);
        
        p0.index_mut(&mut p1);
        
        // Add assertions here if needed
    }
}
                            #[cfg(test)]
mod tests_rug_204 {
    use super::*;
    use crate::value::Index;
    use crate::value::Value;
    
    #[test]
    fn test_index() {
        let p0: &std::string::String = &"test_index".to_string();
        let p1: &Value = &Value::from(10);
        
        p0.index(p1);
    }
}
#[cfg(test)]
mod tests_rug_205 {
    use super::*;
    use crate::value::Index;
    use crate::value::Value;

    #[test]
    fn test_rug() {
        let mut p0: String = String::from("example");
        let mut p1: Value = Value::from("value");

        p0.index_mut(&mut p1);
    }
}
#[cfg(test)]
mod tests_rug_225 {
    use super::*;
    use crate::value::{Value, SeqDeserializer};
    
    #[test]
    fn test_rug() {
        let p0: Vec<Value> = Vec::new();
        SeqDeserializer::new(p0);
    }
}

#[cfg(test)]
mod tests_rug_238 {
    use super::*;
    use crate::macros::IntoDeserializer;
    use crate::value::Value;
    use std::str::FromStr;

    #[test]
    fn test_rug() {
        let p0 = Value::from_str("value").unwrap();

        p0.into_deserializer();
    }
}
#[cfg(test)]
mod tests_rug_246 {
    use super::*;
    use crate::value::ValueSerializer;
    use crate::value::Value;
    use crate::ser::Error;
    use serde::ser::Serializer;

    #[test]
    fn test_rug() {
        let mut p0 = ValueSerializer;
        let mut p1: u32 = 42;

        assert_eq!(
            <ValueSerializer as Serializer>::serialize_u32(p0, p1),
            Ok(Value::Integer(42i64.into()))
        );
    }
}
#[cfg(test)]
mod tests_rug_254 {
    use super::*;
    use crate::ser::Error;
    use crate::Value;
    use serde::ser::Serializer;

    #[test]
    fn test_rug() {
        let mut p0 = ValueSerializer; // Construct ValueSerializer

        let p1: &str = "unit_struct"; // Construct `&str` value
        
        <ValueSerializer as Serializer>::serialize_unit_struct(p0, &p1).unwrap();
    }
}
#[cfg(test)]
mod tests_rug_271 {
    use super::*;
    use crate::ser::Error;
    use crate::value::{Table, TableSerializer};
    use serde::Serializer;

    #[test]
    fn test_serialize_i64() {
        let p0 = TableSerializer;
        let p1: i64 = 123;
        
        let result = p0.serialize_i64(p1);
        assert_eq!(result, Err(Error::unsupported_type(None)));
    }
}
#[cfg(test)]
mod tests_rug_292 {
    use super::*;
    use crate::ser::Error;
    use crate::value::{SerializeMap, TableSerializer};
    use serde::ser::{Serialize, Serializer};
    
    #[test]
    fn test_serialize_map() {
        let mut p0 = TableSerializer {};
        let p1 = None;
        
        let result: Result<SerializeMap, Error> = p0.serialize_map(p1);
        
        assert!(result.is_ok());
    }
}