use crate::de::ParserNumber;
use crate::error::Error;
use crate::lib::*;
use serde::de::{self, Unexpected, Visitor};
use serde::{
    forward_to_deserialize_any, serde_if_integer128, Deserialize, Deserializer,
    Serialize, Serializer,
};
#[cfg(feature = "arbitrary_precision")]
use crate::error::ErrorCode;
#[cfg(feature = "arbitrary_precision")]
use serde::de::{IntoDeserializer, MapAccess};
#[cfg(feature = "arbitrary_precision")]
pub(crate) const TOKEN: &str = "$serde_json::private::Number";
/// Represents a JSON number, whether integer or floating point.
#[derive(Clone, Eq, PartialEq)]
pub struct Number {
    n: N,
}
#[cfg(not(feature = "arbitrary_precision"))]
#[derive(Copy, Clone, PartialEq)]
enum N {
    PosInt(u64),
    /// Always less than zero.
    NegInt(i64),
    /// Always finite.
    Float(f64),
}
#[cfg(not(feature = "arbitrary_precision"))]
impl Eq for N {}
#[cfg(feature = "arbitrary_precision")]
type N = String;
impl Number {
    /// Returns true if the `Number` is an integer between `i64::MIN` and
    /// `i64::MAX`.
    ///
    /// For any Number on which `is_i64` returns true, `as_i64` is guaranteed to
    /// return the integer value.
    ///
    /// ```
    /// # use serde_json::json;
    /// #
    /// let big = i64::max_value() as u64 + 10;
    /// let v = json!({ "a": 64, "b": big, "c": 256.0 });
    ///
    /// assert!(v["a"].is_i64());
    ///
    /// // Greater than i64::MAX.
    /// assert!(!v["b"].is_i64());
    ///
    /// // Numbers with a decimal point are not considered integers.
    /// assert!(!v["c"].is_i64());
    /// ```
    #[inline]
    pub fn is_i64(&self) -> bool {
        #[cfg(not(feature = "arbitrary_precision"))]
        match self.n {
            N::PosInt(v) => v <= i64::max_value() as u64,
            N::NegInt(_) => true,
            N::Float(_) => false,
        }
        #[cfg(feature = "arbitrary_precision")] self.as_i64().is_some()
    }
    /// Returns true if the `Number` is an integer between zero and `u64::MAX`.
    ///
    /// For any Number on which `is_u64` returns true, `as_u64` is guaranteed to
    /// return the integer value.
    ///
    /// ```
    /// # use serde_json::json;
    /// #
    /// let v = json!({ "a": 64, "b": -64, "c": 256.0 });
    ///
    /// assert!(v["a"].is_u64());
    ///
    /// // Negative integer.
    /// assert!(!v["b"].is_u64());
    ///
    /// // Numbers with a decimal point are not considered integers.
    /// assert!(!v["c"].is_u64());
    /// ```
    #[inline]
    pub fn is_u64(&self) -> bool {
        #[cfg(not(feature = "arbitrary_precision"))]
        match self.n {
            N::PosInt(_) => true,
            N::NegInt(_) | N::Float(_) => false,
        }
        #[cfg(feature = "arbitrary_precision")] self.as_u64().is_some()
    }
    /// Returns true if the `Number` can be represented by f64.
    ///
    /// For any Number on which `is_f64` returns true, `as_f64` is guaranteed to
    /// return the floating point value.
    ///
    /// Currently this function returns true if and only if both `is_i64` and
    /// `is_u64` return false but this is not a guarantee in the future.
    ///
    /// ```
    /// # use serde_json::json;
    /// #
    /// let v = json!({ "a": 256.0, "b": 64, "c": -64 });
    ///
    /// assert!(v["a"].is_f64());
    ///
    /// // Integers.
    /// assert!(!v["b"].is_f64());
    /// assert!(!v["c"].is_f64());
    /// ```
    #[inline]
    pub fn is_f64(&self) -> bool {
        #[cfg(not(feature = "arbitrary_precision"))]
        match self.n {
            N::Float(_) => true,
            N::PosInt(_) | N::NegInt(_) => false,
        }
        #[cfg(feature = "arbitrary_precision")]
        {
            for c in self.n.chars() {
                if c == '.' || c == 'e' || c == 'E' {
                    return self.n.parse::<f64>().ok().map_or(false, |f| f.is_finite());
                }
            }
            false
        }
    }
    /// If the `Number` is an integer, represent it as i64 if possible. Returns
    /// None otherwise.
    ///
    /// ```
    /// # use serde_json::json;
    /// #
    /// let big = i64::max_value() as u64 + 10;
    /// let v = json!({ "a": 64, "b": big, "c": 256.0 });
    ///
    /// assert_eq!(v["a"].as_i64(), Some(64));
    /// assert_eq!(v["b"].as_i64(), None);
    /// assert_eq!(v["c"].as_i64(), None);
    /// ```
    #[inline]
    pub fn as_i64(&self) -> Option<i64> {
        #[cfg(not(feature = "arbitrary_precision"))]
        match self.n {
            N::PosInt(n) => {
                if n <= i64::max_value() as u64 { Some(n as i64) } else { None }
            }
            N::NegInt(n) => Some(n),
            N::Float(_) => None,
        }
        #[cfg(feature = "arbitrary_precision")] self.n.parse().ok()
    }
    /// If the `Number` is an integer, represent it as u64 if possible. Returns
    /// None otherwise.
    ///
    /// ```
    /// # use serde_json::json;
    /// #
    /// let v = json!({ "a": 64, "b": -64, "c": 256.0 });
    ///
    /// assert_eq!(v["a"].as_u64(), Some(64));
    /// assert_eq!(v["b"].as_u64(), None);
    /// assert_eq!(v["c"].as_u64(), None);
    /// ```
    #[inline]
    pub fn as_u64(&self) -> Option<u64> {
        #[cfg(not(feature = "arbitrary_precision"))]
        match self.n {
            N::PosInt(n) => Some(n),
            N::NegInt(_) | N::Float(_) => None,
        }
        #[cfg(feature = "arbitrary_precision")] self.n.parse().ok()
    }
    /// Represents the number as f64 if possible. Returns None otherwise.
    ///
    /// ```
    /// # use serde_json::json;
    /// #
    /// let v = json!({ "a": 256.0, "b": 64, "c": -64 });
    ///
    /// assert_eq!(v["a"].as_f64(), Some(256.0));
    /// assert_eq!(v["b"].as_f64(), Some(64.0));
    /// assert_eq!(v["c"].as_f64(), Some(-64.0));
    /// ```
    #[inline]
    pub fn as_f64(&self) -> Option<f64> {
        #[cfg(not(feature = "arbitrary_precision"))]
        match self.n {
            N::PosInt(n) => Some(n as f64),
            N::NegInt(n) => Some(n as f64),
            N::Float(n) => Some(n),
        }
        #[cfg(feature = "arbitrary_precision")]
        self.n.parse::<f64>().ok().filter(|float| float.is_finite())
    }
    /// Converts a finite `f64` to a `Number`. Infinite or NaN values are not JSON
    /// numbers.
    ///
    /// ```
    /// # use std::f64;
    /// #
    /// # use serde_json::Number;
    /// #
    /// assert!(Number::from_f64(256.0).is_some());
    ///
    /// assert!(Number::from_f64(f64::NAN).is_none());
    /// ```
    #[inline]
    pub fn from_f64(f: f64) -> Option<Number> {
        if f.is_finite() {
            let n = {
                #[cfg(not(feature = "arbitrary_precision"))] { N::Float(f) }
                #[cfg(feature = "arbitrary_precision")]
                { ryu::Buffer::new().format_finite(f).to_owned() }
            };
            Some(Number { n })
        } else {
            None
        }
    }
    #[cfg(feature = "arbitrary_precision")]
    /// Not public API. Only tests use this.
    #[doc(hidden)]
    #[inline]
    pub fn from_string_unchecked(n: String) -> Self {
        Number { n }
    }
}
impl fmt::Display for Number {
    #[cfg(not(feature = "arbitrary_precision"))]
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self.n {
            N::PosInt(u) => Display::fmt(&u, formatter),
            N::NegInt(i) => Display::fmt(&i, formatter),
            N::Float(f) => Display::fmt(&f, formatter),
        }
    }
    #[cfg(feature = "arbitrary_precision")]
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.n, formatter)
    }
}
impl Debug for Number {
    #[cfg(not(feature = "arbitrary_precision"))]
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let mut debug = formatter.debug_tuple("Number");
        match self.n {
            N::PosInt(i) => {
                debug.field(&i);
            }
            N::NegInt(i) => {
                debug.field(&i);
            }
            N::Float(f) => {
                debug.field(&f);
            }
        }
        debug.finish()
    }
    #[cfg(feature = "arbitrary_precision")]
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.debug_tuple("Number").field(&format_args!("{}", self.n)).finish()
    }
}
impl Serialize for Number {
    #[cfg(not(feature = "arbitrary_precision"))]
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.n {
            N::PosInt(u) => serializer.serialize_u64(u),
            N::NegInt(i) => serializer.serialize_i64(i),
            N::Float(f) => serializer.serialize_f64(f),
        }
    }
    #[cfg(feature = "arbitrary_precision")]
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct(TOKEN, 1)?;
        s.serialize_field(TOKEN, &self.n)?;
        s.end()
    }
}
impl<'de> Deserialize<'de> for Number {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Number, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct NumberVisitor;
        impl<'de> Visitor<'de> for NumberVisitor {
            type Value = Number;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
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
            #[cfg(feature = "arbitrary_precision")]
            #[inline]
            fn visit_map<V>(self, mut visitor: V) -> Result<Number, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                let value = visitor.next_key::<NumberKey>()?;
                if value.is_none() {
                    return Err(de::Error::invalid_type(Unexpected::Map, &self));
                }
                let v: NumberFromString = visitor.next_value()?;
                Ok(v.value)
            }
        }
        deserializer.deserialize_any(NumberVisitor)
    }
}
#[cfg(feature = "arbitrary_precision")]
struct NumberKey;
#[cfg(feature = "arbitrary_precision")]
impl<'de> de::Deserialize<'de> for NumberKey {
    fn deserialize<D>(deserializer: D) -> Result<NumberKey, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct FieldVisitor;
        impl<'de> de::Visitor<'de> for FieldVisitor {
            type Value = ();
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a valid number field")
            }
            fn visit_str<E>(self, s: &str) -> Result<(), E>
            where
                E: de::Error,
            {
                if s == TOKEN {
                    Ok(())
                } else {
                    Err(de::Error::custom("expected field with custom name"))
                }
            }
        }
        deserializer.deserialize_identifier(FieldVisitor)?;
        Ok(NumberKey)
    }
}
#[cfg(feature = "arbitrary_precision")]
pub struct NumberFromString {
    pub value: Number,
}
#[cfg(feature = "arbitrary_precision")]
impl<'de> de::Deserialize<'de> for NumberFromString {
    fn deserialize<D>(deserializer: D) -> Result<NumberFromString, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> de::Visitor<'de> for Visitor {
            type Value = NumberFromString;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("string containing a number")
            }
            fn visit_str<E>(self, s: &str) -> Result<NumberFromString, E>
            where
                E: de::Error,
            {
                let n = tri!(s.parse().map_err(de::Error::custom));
                Ok(NumberFromString { value: n })
            }
        }
        deserializer.deserialize_str(Visitor)
    }
}
#[cfg(feature = "arbitrary_precision")]
fn invalid_number() -> Error {
    Error::syntax(ErrorCode::InvalidNumber, 0, 0)
}
macro_rules! deserialize_any {
    (@ expand[$($num_string:tt)*]) => {
        #[cfg(not(feature = "arbitrary_precision"))] #[inline] fn deserialize_any < V >
        (self, visitor : V) -> Result < V::Value, Error > where V : Visitor <'de >, {
        match self.n { N::PosInt(u) => visitor.visit_u64(u), N::NegInt(i) => visitor
        .visit_i64(i), N::Float(f) => visitor.visit_f64(f), } } #[cfg(feature =
        "arbitrary_precision")] #[inline] fn deserialize_any < V > (self, visitor : V) ->
        Result < V::Value, Error > where V : Visitor <'de > { if let Some(u) = self
        .as_u64() { return visitor.visit_u64(u); } else if let Some(i) = self.as_i64() {
        return visitor.visit_i64(i); } else if let Some(f) = self.as_f64() { if
        ryu::Buffer::new().format_finite(f) == self.n || f.to_string() == self.n { return
        visitor.visit_f64(f); } } visitor.visit_map(NumberDeserializer { number :
        Some(self.$($num_string)*), }) }
    };
    (owned) => {
        deserialize_any!(@ expand[n]);
    };
    (ref) => {
        deserialize_any!(@ expand[n.clone()]);
    };
}
macro_rules! deserialize_number {
    ($deserialize:ident => $visit:ident) => {
        #[cfg(not(feature = "arbitrary_precision"))] fn $deserialize < V > (self, visitor
        : V) -> Result < V::Value, Error > where V : Visitor <'de >, { self
        .deserialize_any(visitor) } #[cfg(feature = "arbitrary_precision")] fn
        $deserialize < V > (self, visitor : V) -> Result < V::Value, Error > where V :
        de::Visitor <'de >, { visitor.$visit (self.n.parse().map_err(| _ |
        invalid_number()) ?) }
    };
}
impl<'de> Deserializer<'de> for Number {
    type Error = Error;
    deserialize_any!(owned);
    deserialize_number!(deserialize_i8 => visit_i8);
    deserialize_number!(deserialize_i16 => visit_i16);
    deserialize_number!(deserialize_i32 => visit_i32);
    deserialize_number!(deserialize_i64 => visit_i64);
    deserialize_number!(deserialize_u8 => visit_u8);
    deserialize_number!(deserialize_u16 => visit_u16);
    deserialize_number!(deserialize_u32 => visit_u32);
    deserialize_number!(deserialize_u64 => visit_u64);
    deserialize_number!(deserialize_f32 => visit_f32);
    deserialize_number!(deserialize_f64 => visit_f64);
    serde_if_integer128! {
        deserialize_number!(deserialize_i128 => visit_i128);
        deserialize_number!(deserialize_u128 => visit_u128);
    }
    forward_to_deserialize_any! {
        bool char str string bytes byte_buf option unit unit_struct newtype_struct seq
        tuple tuple_struct map struct enum identifier ignored_any
    }
}
impl<'de, 'a> Deserializer<'de> for &'a Number {
    type Error = Error;
    deserialize_any!(ref);
    deserialize_number!(deserialize_i8 => visit_i8);
    deserialize_number!(deserialize_i16 => visit_i16);
    deserialize_number!(deserialize_i32 => visit_i32);
    deserialize_number!(deserialize_i64 => visit_i64);
    deserialize_number!(deserialize_u8 => visit_u8);
    deserialize_number!(deserialize_u16 => visit_u16);
    deserialize_number!(deserialize_u32 => visit_u32);
    deserialize_number!(deserialize_u64 => visit_u64);
    deserialize_number!(deserialize_f32 => visit_f32);
    deserialize_number!(deserialize_f64 => visit_f64);
    serde_if_integer128! {
        deserialize_number!(deserialize_i128 => visit_i128);
        deserialize_number!(deserialize_u128 => visit_u128);
    }
    forward_to_deserialize_any! {
        bool char str string bytes byte_buf option unit unit_struct newtype_struct seq
        tuple tuple_struct map struct enum identifier ignored_any
    }
}
#[cfg(feature = "arbitrary_precision")]
pub(crate) struct NumberDeserializer {
    pub number: Option<String>,
}
#[cfg(feature = "arbitrary_precision")]
impl<'de> MapAccess<'de> for NumberDeserializer {
    type Error = Error;
    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        if self.number.is_none() {
            return Ok(None);
        }
        seed.deserialize(NumberFieldDeserializer).map(Some)
    }
    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        seed.deserialize(self.number.take().unwrap().into_deserializer())
    }
}
#[cfg(feature = "arbitrary_precision")]
struct NumberFieldDeserializer;
#[cfg(feature = "arbitrary_precision")]
impl<'de> Deserializer<'de> for NumberFieldDeserializer {
    type Error = Error;
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_borrowed_str(TOKEN)
    }
    forward_to_deserialize_any! {
        bool u8 u16 u32 u64 u128 i8 i16 i32 i64 i128 f32 f64 char str string seq bytes
        byte_buf map struct option unit newtype_struct ignored_any unit_struct
        tuple_struct tuple enum identifier
    }
}
impl From<ParserNumber> for Number {
    fn from(value: ParserNumber) -> Self {
        let n = match value {
            ParserNumber::F64(f) => {
                #[cfg(not(feature = "arbitrary_precision"))] { N::Float(f) }
                #[cfg(feature = "arbitrary_precision")] { f.to_string() }
            }
            ParserNumber::U64(u) => {
                #[cfg(not(feature = "arbitrary_precision"))] { N::PosInt(u) }
                #[cfg(feature = "arbitrary_precision")] { u.to_string() }
            }
            ParserNumber::I64(i) => {
                #[cfg(not(feature = "arbitrary_precision"))] { N::NegInt(i) }
                #[cfg(feature = "arbitrary_precision")] { i.to_string() }
            }
            #[cfg(feature = "arbitrary_precision")]
            ParserNumber::String(s) => s,
        };
        Number { n }
    }
}
macro_rules! impl_from_unsigned {
    ($($ty:ty),*) => {
        $(impl From <$ty > for Number { #[inline] fn from(u : $ty) -> Self { let n = {
        #[cfg(not(feature = "arbitrary_precision"))] { N::PosInt(u as u64) }
        #[cfg(feature = "arbitrary_precision")] { itoa::Buffer::new().format(u)
        .to_owned() } }; Number { n } } })*
    };
}
macro_rules! impl_from_signed {
    ($($ty:ty),*) => {
        $(impl From <$ty > for Number { #[inline] fn from(i : $ty) -> Self { let n = {
        #[cfg(not(feature = "arbitrary_precision"))] { if i < 0 { N::NegInt(i as i64) }
        else { N::PosInt(i as u64) } } #[cfg(feature = "arbitrary_precision")] {
        itoa::Buffer::new().format(i).to_owned() } }; Number { n } } })*
    };
}
impl_from_unsigned!(u8, u16, u32, u64, usize);
impl_from_signed!(i8, i16, i32, i64, isize);
#[cfg(feature = "arbitrary_precision")]
serde_if_integer128! {
    impl From < i128 > for Number { fn from(i : i128) -> Self { Number { n : i
    .to_string() } } } impl From < u128 > for Number { fn from(u : u128) -> Self { Number
    { n : u.to_string() } } }
}
impl Number {
    #[cfg(not(feature = "arbitrary_precision"))]
    #[cold]
    pub(crate) fn unexpected(&self) -> Unexpected {
        match self.n {
            N::PosInt(u) => Unexpected::Unsigned(u),
            N::NegInt(i) => Unexpected::Signed(i),
            N::Float(f) => Unexpected::Float(f),
        }
    }
    #[cfg(feature = "arbitrary_precision")]
    #[cold]
    pub(crate) fn unexpected(&self) -> Unexpected {
        Unexpected::Other("number")
    }
}
#[cfg(test)]
mod tests_llm_16_168_llm_16_167 {
    use crate::{Number, Error, from_str, Value, Map};
    #[test]
    fn test_deserialize_u64() -> Result<(), Error> {
        let input = r#"{"a": 42, "b": 3.14, "c": "hello"}"#;
        let value: Map<String, Value> = from_str(input)?;
        let number: Number = value["a"].as_u64().unwrap().into();
        assert_eq!(number, Number::from(42u64));
        let number: Number = value["b"].as_u64().unwrap().into();
        assert_eq!(number, Number::from(3u64));
        let number: Number = value["c"].as_u64().unwrap().into();
        assert_eq!(number, Number::from(0u64));
        Ok(())
    }
}
#[cfg(test)]
mod tests_llm_16_314 {
    use serde::Deserialize;
    use crate::{Map, Number, Value};
    #[test]
    fn test_deserialize_i128() {
        let _rug_st_tests_llm_16_314_rrrruuuugggg_test_deserialize_i128 = 0;
        let value: Value = Value::Object(Map::new());
        let result: Result<Number, _> = Number::deserialize(&value);
        debug_assert!(result.is_err());
        let _rug_ed_tests_llm_16_314_rrrruuuugggg_test_deserialize_i128 = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_324_llm_16_323 {
    use serde::de::Deserialize;
    use crate::error::Error;
    use crate::map::Map;
    use crate::value::Value;
    use crate::number::Number;
    use std::str::FromStr;
    #[test]
    fn test_deserialize_u128() {
        let _rug_st_tests_llm_16_324_llm_16_323_rrrruuuugggg_test_deserialize_u128 = 0;
        let rug_fuzz_0 = r#"{
            "a": 123456789012345678901234567890,
            "b": "123456789012345678901234567890",
            "c": 2.5,
            "d": 9223372036854775809
        }"#;
        let rug_fuzz_1 = "123456789012345678901234567890";
        let rug_fuzz_2 = "a";
        let rug_fuzz_3 = "b";
        let rug_fuzz_4 = "c";
        let rug_fuzz_5 = "d";
        let json = rug_fuzz_0;
        let value: Map<String, Value> = crate::from_str(json).unwrap();
        let expected_a = Some(Number::from_str(rug_fuzz_1).unwrap());
        let expected_b = None;
        let expected_c = None;
        let expected_d = None;
        let actual_a = value
            .get(rug_fuzz_2)
            .and_then(|v| v.as_str().and_then(|s| Some(Number::from_str(s).unwrap())));
        let actual_b = value
            .get(rug_fuzz_3)
            .and_then(|v| v.as_str().and_then(|s| Some(Number::from_str(s).unwrap())));
        let actual_c = value
            .get(rug_fuzz_4)
            .and_then(|v| v.as_u64().map(|u| Number::from(u)));
        let actual_d = value
            .get(rug_fuzz_5)
            .and_then(|v| v.as_i64().map(|i| Number::from(i as u64)));
        debug_assert_eq!(expected_a, actual_a);
        debug_assert_eq!(expected_b, actual_b);
        debug_assert_eq!(expected_c, actual_c);
        debug_assert_eq!(expected_d, actual_d);
        let _rug_ed_tests_llm_16_324_llm_16_323_rrrruuuugggg_test_deserialize_u128 = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_340 {
    use crate::number::Number;
    use crate::number::N;
    use serde::Deserialize;
    use serde::de::{self, Deserializer, Visitor};
    use std::fmt;
    use std::marker::PhantomData;
    use std::result;
    use std::str::FromStr;
    #[test]
    fn test_number_from_i32() {
        let _rug_st_tests_llm_16_340_rrrruuuugggg_test_number_from_i32 = 0;
        let rug_fuzz_0 = 42_i32;
        let num: Number = Number::from(rug_fuzz_0);
        debug_assert_eq!(num, Number { n : N::PosInt(42) });
        let _rug_ed_tests_llm_16_340_rrrruuuugggg_test_number_from_i32 = 0;
    }
    #[test]
    fn test_number_from_i32_negative() {
        let _rug_st_tests_llm_16_340_rrrruuuugggg_test_number_from_i32_negative = 0;
        let rug_fuzz_0 = 42_i32;
        let num: Number = Number::from(-rug_fuzz_0);
        debug_assert_eq!(num, Number { n : N::NegInt(- 42) });
        let _rug_ed_tests_llm_16_340_rrrruuuugggg_test_number_from_i32_negative = 0;
    }
    #[cfg(feature = "arbitrary_precision")]
    #[test]
    fn test_number_from_i32_arbitrary_precision() {
        let _rug_st_tests_llm_16_340_rrrruuuugggg_test_number_from_i32_arbitrary_precision = 0;
        let rug_fuzz_0 = 42_i32;
        let num: Number = Number::from(rug_fuzz_0);
        debug_assert_eq!(num, Number { n : N::Float(42.0) });
        let _rug_ed_tests_llm_16_340_rrrruuuugggg_test_number_from_i32_arbitrary_precision = 0;
    }
    #[cfg(feature = "arbitrary_precision")]
    #[test]
    fn test_number_from_i32_negative_arbitrary_precision() {
        let _rug_st_tests_llm_16_340_rrrruuuugggg_test_number_from_i32_negative_arbitrary_precision = 0;
        let rug_fuzz_0 = 42_i32;
        let num: Number = Number::from(-rug_fuzz_0);
        debug_assert_eq!(num, Number { n : N::Float(- 42.0) });
        let _rug_ed_tests_llm_16_340_rrrruuuugggg_test_number_from_i32_negative_arbitrary_precision = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_344_llm_16_343 {
    use super::*;
    use crate::*;
    use crate::Number;
    #[test]
    fn test_from_i8_positive() {
        let _rug_st_tests_llm_16_344_llm_16_343_rrrruuuugggg_test_from_i8_positive = 0;
        let rug_fuzz_0 = 42;
        let i: i8 = rug_fuzz_0;
        let expected: Number = i.into();
        let result: Number = Number::from(i);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_344_llm_16_343_rrrruuuugggg_test_from_i8_positive = 0;
    }
    #[test]
    fn test_from_i8_negative() {
        let _rug_st_tests_llm_16_344_llm_16_343_rrrruuuugggg_test_from_i8_negative = 0;
        let rug_fuzz_0 = 42;
        let i: i8 = -rug_fuzz_0;
        let expected: Number = i.into();
        let result: Number = Number::from(i);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_344_llm_16_343_rrrruuuugggg_test_from_i8_negative = 0;
    }
    #[test]
    #[cfg(not(feature = "arbitrary_precision"))]
    fn test_from_i8_max_value() {
        let _rug_st_tests_llm_16_344_llm_16_343_rrrruuuugggg_test_from_i8_max_value = 0;
        let i: i8 = i8::MAX;
        let expected: Number = i.into();
        let result: Number = Number::from(i);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_344_llm_16_343_rrrruuuugggg_test_from_i8_max_value = 0;
    }
    #[test]
    #[cfg(not(feature = "arbitrary_precision"))]
    fn test_from_i8_min_value() {
        let _rug_st_tests_llm_16_344_llm_16_343_rrrruuuugggg_test_from_i8_min_value = 0;
        let i: i8 = i8::MIN;
        let expected: Number = i.into();
        let result: Number = Number::from(i);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_344_llm_16_343_rrrruuuugggg_test_from_i8_min_value = 0;
    }
    #[test]
    #[cfg(feature = "arbitrary_precision")]
    fn test_from_i8_arbitrary_precision() {
        let _rug_st_tests_llm_16_344_llm_16_343_rrrruuuugggg_test_from_i8_arbitrary_precision = 0;
        let rug_fuzz_0 = 42;
        let rug_fuzz_1 = "42";
        let i: i8 = rug_fuzz_0;
        let expected: Number = Number::from_string_unchecked(rug_fuzz_1.to_string());
        let result: Number = Number::from(i);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_344_llm_16_343_rrrruuuugggg_test_from_i8_arbitrary_precision = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_346_llm_16_345 {
    use crate::number::{Number, N};
    #[test]
    fn test_from() {
        let _rug_st_tests_llm_16_346_llm_16_345_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        debug_assert_eq!(
            < Number as std::convert::From < isize > > ::from(rug_fuzz_0), Number { n :
            N::PosInt(10) }
        );
        debug_assert_eq!(
            < Number as std::convert::From < isize > > ::from(- rug_fuzz_1), Number { n :
            N::NegInt(- 10) }
        );
        let _rug_ed_tests_llm_16_346_llm_16_345_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_349 {
    use super::*;
    use crate::*;
    #[test]
    fn test_from() {
        let _rug_st_tests_llm_16_349_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = 42;
        let u: u32 = rug_fuzz_0;
        let result: Number = Number::from(u);
        let expected: Number = Number { n: N::PosInt(u as u64) };
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_349_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_355 {
    use super::*;
    use crate::*;
    use serde::{Serialize, Deserialize};
    #[test]
    fn test_from() {
        let _rug_st_tests_llm_16_355_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = 42;
        let rug_fuzz_1 = 42;
        let u: u64 = rug_fuzz_0;
        let expected_result: Number = Number { n: N::PosInt(rug_fuzz_1) };
        let result: Number = From::from(u);
        debug_assert_eq!(result, expected_result);
        let _rug_ed_tests_llm_16_355_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_932 {
    use crate::json;
    #[test]
    fn test_as_f64() {
        let _rug_st_tests_llm_16_932_rrrruuuugggg_test_as_f64 = 0;
        let rug_fuzz_0 = "a";
        let rug_fuzz_1 = "b";
        let rug_fuzz_2 = "c";
        let v = json!({ "a" : 256.0, "b" : 64, "c" : - 64 });
        debug_assert_eq!(v[rug_fuzz_0].as_f64(), Some(256.0));
        debug_assert_eq!(v[rug_fuzz_1].as_f64(), Some(64.0));
        debug_assert_eq!(v[rug_fuzz_2].as_f64(), Some(- 64.0));
        let _rug_ed_tests_llm_16_932_rrrruuuugggg_test_as_f64 = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_933 {
    use super::*;
    use crate::*;
    use crate::json;
    #[test]
    fn test_as_i64() {
        let _rug_st_tests_llm_16_933_rrrruuuugggg_test_as_i64 = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = "a";
        let rug_fuzz_2 = "b";
        let rug_fuzz_3 = "c";
        let big = i64::max_value() as u64 + rug_fuzz_0;
        let v = json!({ "a" : 64, "b" : big, "c" : 256.0 });
        debug_assert_eq!(v[rug_fuzz_1].as_i64(), Some(64));
        debug_assert_eq!(v[rug_fuzz_2].as_i64(), None);
        debug_assert_eq!(v[rug_fuzz_3].as_i64(), None);
        let _rug_ed_tests_llm_16_933_rrrruuuugggg_test_as_i64 = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_934 {
    use crate::json;
    use crate::Number;
    #[test]
    fn test_as_u64() {
        let _rug_st_tests_llm_16_934_rrrruuuugggg_test_as_u64 = 0;
        let rug_fuzz_0 = "a";
        let rug_fuzz_1 = "b";
        let rug_fuzz_2 = "c";
        let v = json!({ "a" : 64, "b" : - 64, "c" : 256.0 });
        debug_assert_eq!(v[rug_fuzz_0].as_u64(), Some(64));
        debug_assert_eq!(v[rug_fuzz_1].as_u64(), None);
        debug_assert_eq!(v[rug_fuzz_2].as_u64(), None);
        let _rug_ed_tests_llm_16_934_rrrruuuugggg_test_as_u64 = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_935 {
    use super::*;
    use crate::*;
    use crate::Number;
    #[test]
    fn test_from_f64_with_finite_number_should_return_some() {
        let _rug_st_tests_llm_16_935_rrrruuuugggg_test_from_f64_with_finite_number_should_return_some = 0;
        let rug_fuzz_0 = 256.0;
        debug_assert!(Number::from_f64(rug_fuzz_0).is_some());
        let _rug_ed_tests_llm_16_935_rrrruuuugggg_test_from_f64_with_finite_number_should_return_some = 0;
    }
    #[test]
    fn test_from_f64_with_nan_should_return_none() {
        let _rug_st_tests_llm_16_935_rrrruuuugggg_test_from_f64_with_nan_should_return_none = 0;
        debug_assert!(Number::from_f64(std::f64::NAN).is_none());
        let _rug_ed_tests_llm_16_935_rrrruuuugggg_test_from_f64_with_nan_should_return_none = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_936 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_f64() {
        let number = Number { n: N::Float(10.0) };
        assert_eq!(number.is_f64(), true);
        let number = Number { n: N::PosInt(10) };
        assert_eq!(number.is_f64(), false);
        let number = Number { n: N::NegInt(-10) };
        assert_eq!(number.is_f64(), false);
        #[cfg(feature = "arbitrary_precision")]
        {
            let number = Number { n: N::Float(f64::NAN) };
            assert_eq!(number.is_f64(), false);
            let number = Number {
                n: N::Float(f64::INFINITY),
            };
            assert_eq!(number.is_f64(), false);
            let number = Number {
                n: N::Float(f64::NEG_INFINITY),
            };
            assert_eq!(number.is_f64(), false);
            let number = Number { n: N::Float(10.0) };
            assert_eq!(number.is_f64(), true);
            let number = Number { n: N::Float(10.5) };
            assert_eq!(number.is_f64(), true);
            let number = Number { n: N::PosInt(10) };
            assert_eq!(number.is_f64(), false);
            let number = Number { n: N::NegInt(-10) };
            assert_eq!(number.is_f64(), false);
            let number = Number { n: N::Float(10e10) };
            assert_eq!(number.is_f64(), true);
            let number = Number { n: N::Float(10e-10) };
            assert_eq!(number.is_f64(), true);
            let number = Number { n: N::Float(10e1000) };
            assert_eq!(number.is_f64(), false);
        }
    }
}
#[cfg(test)]
mod tests_llm_16_937 {
    use crate::json;
    use crate::number::{Number, N};
    #[test]
    fn test_is_i64() {
        let _rug_st_tests_llm_16_937_rrrruuuugggg_test_is_i64 = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = "a";
        let rug_fuzz_2 = "b";
        let rug_fuzz_3 = "c";
        let big = i64::max_value() as u64 + rug_fuzz_0;
        let v = json!({ "a" : 64, "b" : big, "c" : 256.0 });
        debug_assert!(v[rug_fuzz_1].is_i64());
        debug_assert!(! v[rug_fuzz_2].is_i64());
        debug_assert!(! v[rug_fuzz_3].is_i64());
        let _rug_ed_tests_llm_16_937_rrrruuuugggg_test_is_i64 = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_938 {
    use crate::{json, Number};
    #[test]
    fn test_is_u64() {
        let _rug_st_tests_llm_16_938_rrrruuuugggg_test_is_u64 = 0;
        let rug_fuzz_0 = "a";
        let rug_fuzz_1 = "b";
        let rug_fuzz_2 = "c";
        let v = json!({ "a" : 64, "b" : - 64, "c" : 256.0 });
        debug_assert!(v[rug_fuzz_0].is_u64());
        debug_assert!(! v[rug_fuzz_1].is_u64());
        debug_assert!(! v[rug_fuzz_2].is_u64());
        let _rug_ed_tests_llm_16_938_rrrruuuugggg_test_is_u64 = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_939 {
    use super::*;
    use crate::*;
    #[test]
    fn test_unexpected_pos_int() {
        let _rug_st_tests_llm_16_939_rrrruuuugggg_test_unexpected_pos_int = 0;
        let rug_fuzz_0 = 10;
        let num = Number { n: N::PosInt(rug_fuzz_0) };
        let result = num.unexpected();
        debug_assert_eq!(result, Unexpected::Unsigned(10));
        let _rug_ed_tests_llm_16_939_rrrruuuugggg_test_unexpected_pos_int = 0;
    }
    #[test]
    fn test_unexpected_neg_int() {
        let _rug_st_tests_llm_16_939_rrrruuuugggg_test_unexpected_neg_int = 0;
        let rug_fuzz_0 = 10;
        let num = Number {
            n: N::NegInt(-rug_fuzz_0),
        };
        let result = num.unexpected();
        debug_assert_eq!(result, Unexpected::Signed(- 10));
        let _rug_ed_tests_llm_16_939_rrrruuuugggg_test_unexpected_neg_int = 0;
    }
    #[test]
    fn test_unexpected_float() {
        let _rug_st_tests_llm_16_939_rrrruuuugggg_test_unexpected_float = 0;
        let rug_fuzz_0 = 3.14;
        let num = Number { n: N::Float(rug_fuzz_0) };
        let result = num.unexpected();
        debug_assert_eq!(result, Unexpected::Float(3.14));
        let _rug_ed_tests_llm_16_939_rrrruuuugggg_test_unexpected_float = 0;
    }
}
#[cfg(test)]
mod tests_rug_622 {
    use super::*;
    use serde::de::{self, Visitor};
    use serde::Deserialize;
    use crate::{Number, value};
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
        let _rug_st_tests_rug_622_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123;
        let mut p0: value::Number = rug_fuzz_0.into();
        let mut p1 = NumberVisitor;
        p0.deserialize_f32(p1);
        let _rug_ed_tests_rug_622_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_625 {
    use super::*;
    use crate::Number;
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
        let _rug_st_tests_rug_625_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123;
        let mut p0: Number = rug_fuzz_0.into();
        let mut p1 = NumberVisitor;
        p0.deserialize_i8(p1);
        let _rug_ed_tests_rug_625_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_636 {
    use super::*;
    use crate::number::Number;
    use std::convert::From;
    #[test]
    fn test_number() {
        let _rug_st_tests_rug_636_rrrruuuugggg_test_number = 0;
        let rug_fuzz_0 = 10;
        let mut p0: u8 = rug_fuzz_0;
        Number::from(p0);
        let _rug_ed_tests_rug_636_rrrruuuugggg_test_number = 0;
    }
}
#[cfg(test)]
mod tests_rug_637 {
    use super::*;
    use crate::number::Number;
    use std::convert::From;
    #[test]
    fn test_from() {
        let _rug_st_tests_rug_637_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = 42;
        let p0: u16 = rug_fuzz_0;
        <Number as std::convert::From<u16>>::from(p0);
        let _rug_ed_tests_rug_637_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_rug_638 {
    use super::*;
    use crate::Number;
    #[test]
    fn test_from() {
        let _rug_st_tests_rug_638_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = 42;
        let p0: u64 = rug_fuzz_0;
        let result = <Number as std::convert::From<u64>>::from(p0);
        debug_assert_eq!(Number { n : N::PosInt(p0 as u64) }, result);
        let _rug_ed_tests_rug_638_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_rug_639 {
    use super::*;
    use crate::number::Number;
    use std::convert::From;
    #[test]
    fn test_from() {
        let _rug_st_tests_rug_639_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = 42;
        let p0: i16 = rug_fuzz_0;
        <Number as std::convert::From<i16>>::from(p0);
        let _rug_ed_tests_rug_639_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_rug_640 {
    use super::*;
    use crate::number::Number;
    use std::convert::From;
    #[test]
    fn test_from() {
        let _rug_st_tests_rug_640_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = 42;
        let p0: i64 = rug_fuzz_0;
        <Number as std::convert::From<i64>>::from(p0);
        let _rug_ed_tests_rug_640_rrrruuuugggg_test_from = 0;
    }
}
