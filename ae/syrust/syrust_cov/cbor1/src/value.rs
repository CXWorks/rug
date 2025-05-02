// This Source Code Form is subject to the terms of
// the Mozilla Public License, v. 2.0. If a copy of
// the MPL was not distributed with this file, You
// can obtain one at http://mozilla.org/MPL/2.0/.

//! This module defines the generic `Value` AST as well as
//! several  other types to represent CBOR values.
//! A `Cursor` can be used to deconstruct and traverse
//! a `Value`.

use std::collections::{BTreeMap, LinkedList};
use std::i64;
use types::Tag;

/// The generic CBOR representation.
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum Value {
    Array(Vec<Value>),
    Bool(bool),
    Break,
    Bytes(Bytes),
    F32(f32),
    F64(f64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    Int(Int),
    Map(BTreeMap<Key, Value>),
    Null,
    Simple(Simple),
    Tagged(Tag, Box<Value>),
    Text(Text),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    Undefined
}

/// Type to represent all possible CBOR integer values.
///
/// Since the encoding of negative integers (major type 1) follows
/// unsigned integers (major type 0), mapping negative integers
/// to `i8`, `i16`, `i32` or `i64` can result in integer overflows.
/// If all possible values should be handled, this type can be used.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Int {
    Neg(u64),
    Pos(u64)
}

impl Int {
    pub fn from_u64(n: u64) -> Int {
        Int::Pos(n)
    }

    pub fn from_i64(n: i64) -> Int {
        if n < 0 {
            Int::Neg(i64::abs(n) as u64 - 1)
        } else {
            Int::Pos(n as u64)
        }
    }

    /// Map this value to an `i64`. If the value does not
    /// fit within `[i64::MIN, i64::MAX]`, `None` is returned instead.
    pub fn i64(&self) -> Option<i64> {
        match *self {
            Int::Neg(n) if n <= i64::MAX as u64 => Some(-1 - n as i64),
            Int::Pos(n) if n <= i64::MAX as u64 => Some(n as i64),
            _ => None
        }
    }

    /// Map this value to a `u64`. If the value is negative,
    /// `None` is returned instead.
    pub fn u64(&self) -> Option<u64> {
        match *self {
            Int::Pos(n) => Some(n),
            _           => None
        }
    }
}

/// A unification of plain and indefinitly sized strings.
#[derive(Clone, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub enum Text {
    Text(String),
    Chunks(LinkedList<String>)
}

/// A unification of plain an indefinitly sized byte strings.
#[derive(Clone, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub enum Bytes {
    Bytes(Vec<u8>),
    Chunks(LinkedList<Vec<u8>>)
}

/// Most simple types (e.g. `bool` are covered elsewhere) but this
/// value captures those value ranges of CBOR type `Simple` (major 7)
/// which are either not assigned or reserved.
#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub enum Simple {
    Unassigned(u8),
    Reserved(u8)
}

/// CBOR allows heterogenous keys in objects. This enum unifies
/// all currently allowed key types.
#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum Key {
    Bool(bool),
    Bytes(Bytes),
    Int(Int),
    Text(Text)
}

impl Key {
    pub fn u64(n: u64) -> Key {
        Key::Int(Int::from_u64(n))
    }

    pub fn i64(n: i64) -> Key {
        Key::Int(Int::from_i64(n))
    }
}

/// A `Cursor` allows conventient navigation in a `Value` AST.
/// `Value`s can be converted to native Rust types if possible and
/// collections can be traversed using `at` or `get`.
pub struct Cursor<'r> {
    value: Option<&'r Value>
}

impl<'r> Cursor<'r> {
    pub fn new(v: &'r Value) -> Cursor<'r> {
        Cursor { value: Some(v) }
    }

    fn of(v: Option<&'r Value>) -> Cursor<'r> {
        Cursor { value: v }
    }

    pub fn at(&self, i: usize) -> Cursor<'r> {
        match self.value {
            Some(&Value::Array(ref a)) => Cursor::of(a.get(i)),
            _                          => Cursor::of(None)
        }
    }

    pub fn get(&self, k: Key) -> Cursor<'r> {
        match self.value {
            Some(&Value::Map(ref m)) => Cursor::of(m.get(&k)),
            _                        => Cursor::of(None)
        }
    }

    pub fn field(&self, s: &str) -> Cursor<'r> {
        self.get(Key::Text(Text::Text(String::from(s))))
    }

    pub fn value(&self) -> Option<&Value> {
        self.value
    }

    pub fn opt(&self) -> Option<Cursor<'r>> {
        match self.value {
            Some(&Value::Null) => None,
            Some(ref v)        => Some(Cursor::new(v)),
            _                  => None
        }
    }

    pub fn maybe(&self) -> Option<Cursor<'r>> {
        match self.value {
            Some(&Value::Undefined) => None,
            Some(ref v)             => Some(Cursor::new(v)),
            _                       => None
        }
    }

    pub fn bool(&self) -> Option<bool> {
        match self.value {
            Some(&Value::Bool(x)) => Some(x),
            _                     => None
        }
    }

    pub fn bytes(&self) -> Option<&Bytes> {
        match self.value {
            Some(&Value::Bytes(ref x)) => Some(x),
            _                          => None
        }
    }

    pub fn bytes_plain(&self) -> Option<&Vec<u8>> {
        match self.value {
            Some(&Value::Bytes(Bytes::Bytes(ref x))) => Some(x),
            _                                        => None
        }
    }

    pub fn bytes_chunked(&self) -> Option<&LinkedList<Vec<u8>>> {
        match self.value {
            Some(&Value::Bytes(Bytes::Chunks(ref x))) => Some(x),
            _                                         => None
        }
    }

    pub fn text(&self) -> Option<&Text> {
        match self.value {
            Some(&Value::Text(ref x)) => Some(x),
            _                         => None
        }
    }

    pub fn text_plain(&self) -> Option<&String> {
        match self.value {
            Some(&Value::Text(Text::Text(ref x))) => Some(x),
            _                                     => None
        }
    }

    pub fn text_chunked(&self) -> Option<&LinkedList<String>> {
        match self.value {
            Some(&Value::Text(Text::Chunks(ref x))) => Some(x),
            _                                       => None
        }
    }

    pub fn float32(&self) -> Option<f32> {
        match self.value {
            Some(&Value::F32(x)) => Some(x),
            _                    => None
        }
    }

    pub fn float64(&self) -> Option<f64> {
        match self.value {
            Some(&Value::F64(x)) => Some(x),
            _                    => None
        }
    }

    pub fn u8(&self) -> Option<u8> {
        match self.value {
            Some(&Value::U8(x)) => Some(x),
            _                   => None
        }
    }

    pub fn u16(&self) -> Option<u16> {
        match self.value {
            Some(&Value::U16(x)) => Some(x),
            _                    => None
        }
    }

    pub fn u32(&self) -> Option<u32> {
        match self.value {
            Some(&Value::U32(x)) => Some(x),
            _                    => None
        }
    }

    pub fn u64(&self) -> Option<u64> {
        match self.value {
            Some(&Value::U64(x)) => Some(x),
            _                    => None
        }
    }

    pub fn i8(&self) -> Option<i8> {
        match self.value {
            Some(&Value::I8(x)) => Some(x),
            _                   => None
        }
    }

    pub fn i16(&self) -> Option<i16> {
        match self.value {
            Some(&Value::I16(x)) => Some(x),
            _                    => None
        }
    }

    pub fn i32(&self) -> Option<i32> {
        match self.value {
            Some(&Value::I32(x)) => Some(x),
            _                    => None
        }
    }

    pub fn i64(&self) -> Option<i64> {
        match self.value {
            Some(&Value::I64(x)) => Some(x),
            _                    => None
        }
    }
}

/// Inspect the given `Value` which must be a `Value::Tagged` and
/// ensure that the `Tag` and type of value match according to
/// RFC 7049 section 2.4
pub fn check(value: &Value) -> bool {
    fn fun(t: Tag, b: &Value) -> bool {
        match (t, b) {
            (Tag::DateTime, &Value::Text(_))        => true,
            (Tag::Timestamp, &Value::U8(_))         => true,
            (Tag::Timestamp, &Value::U16(_))        => true,
            (Tag::Timestamp, &Value::U32(_))        => true,
            (Tag::Timestamp, &Value::U64(_))        => true,
            (Tag::Timestamp, &Value::I8(_))         => true,
            (Tag::Timestamp, &Value::I16(_))        => true,
            (Tag::Timestamp, &Value::I32(_))        => true,
            (Tag::Timestamp, &Value::I64(_))        => true,
            (Tag::Timestamp, &Value::F32(_))        => true,
            (Tag::Timestamp, &Value::F64(_))        => true,
            (Tag::Bignum, &Value::Bytes(_))         => true,
            (Tag::NegativeBignum, &Value::Bytes(_)) => true,
            (Tag::ToBase64, _)                      => true,
            (Tag::ToBase64Url, _)                   => true,
            (Tag::ToBase16, _)                      => true,
            (Tag::Cbor, &Value::Bytes(_))           => true,
            (Tag::Uri, &Value::Text(_))             => true,
            (Tag::Base64, &Value::Text(_))          => true,
            (Tag::Base64Url, &Value::Text(_))       => true,
            (Tag::Regex, &Value::Text(_))           => true,
            (Tag::Mime, &Value::Text(_))            => true,
            (Tag::CborSelf, _)                      => true,
            (Tag::Decimal, &Value::Array(ref a))
            | (Tag::Bigfloat, &Value::Array(ref a)) => {
                if a.len() != 2 {
                    return false
                }
                let is_integral = |v: &Value| {
                    match *v {
                        Value::U8(_) | Value::U16(_) | Value::U32(_) | Value::U64(_) => true,
                        Value::I8(_) | Value::I16(_) | Value::I32(_) | Value::I64(_) => true,
                        _                                                            => false
                    }
                };
                let is_bignum = |v: &Value| {
                    fun(Tag::Bignum, v) || fun(Tag::NegativeBignum, v)
                };
                let ref e = a[0];
                let ref m = a[1];
                is_integral(e) && (is_integral(m) || is_bignum(m))
            }
            (Tag::Unassigned(_), _) => true,
            _                       => false
        }
    }

    match *value {
        Value::Tagged(t, ref b) => fun(t, &*b),
        _                       => false
    }
}

#[cfg(test)]
mod tests_rug_3 {
    use super::*;
    use crate::value::Int;

    #[test]
    fn test_rug() {
        let mut p0: u64 = 42;

        Int::from_u64(p0);
    }
}
#[cfg(test)]
mod tests_rug_4 {
    use super::*;
    use value::Int;

    #[test]
    fn test_from_i64() {
        let p0: i64 = -10;

        let result = Int::from_i64(p0);

        match result {
            Int::Neg(val) => {
                assert_eq!(val, 9);
            }
            _ => panic!("Unexpected result"),
        }
    }
}#[cfg(test)]
mod tests_rug_5 {
    use super::*;
    use crate::value::Int;

    #[test]
    fn test_rug() {
        let mut p0 = Int::from_i64(100);

        let result = p0.i64();
        assert_eq!(result, Some(100));
    }
}#[cfg(test)]
mod tests_rug_6 {
    use super::*;
    use crate::value;

    #[test]
    fn test_rug() {
        let mut p0 = value::Int::from_i64(42);
        assert_eq!(p0.u64(), Some(42));
    }
}
#[cfg(test)]
mod tests_rug_7 {
    use super::*;
    use crate::value::{Key, Int};

    #[test]
    fn test_u64() {
        let mut p0: u64 = 42;

        Key::u64(p0);
    }
}
#[cfg(test)]
mod tests_rug_8 {
    use super::*;
    use crate::value::{Key, Int};

    #[test]
    fn test_i64() {
        let p0: i64 = 42;
        
        Key::i64(p0);
    }
}
#[cfg(test)]
mod tests_rug_9 {
    use super::*;
    use crate::value;

    #[test]
    fn test_rug() {
        let mut v1 = value::Value::Null;
        
        value::Cursor::<'_>::new(&v1);

    }
}
#[cfg(test)]
mod tests_rug_10 {
    use super::*;
    use crate::value::Value;

    #[test]
    fn test_of() {
        let v: Option<&Value> = Some(&Value::Null);

        crate::value::Cursor::<'static>::of(v);
    }
}#[cfg(test)]
mod tests_rug_13 {
    use super::*;
    use value::{Cursor, Key, Text};
    
    #[test]
    fn test_rug() {
        let mut p0: Cursor<'_> = unimplemented!();
        let p1 = "sample_field_name";

        p0.field(p1);

    }
}#[cfg(test)]
mod tests_rug_14 {
    use super::*;
    use crate::value::{Cursor, Value};

    #[test]
    fn test_rug() {
        let mut p0: Cursor<'static> = Cursor {
            value: Some(&Value::Null),
        };

        p0.value();

    }
}#[cfg(test)]
mod tests_rug_15 {
    use super::*;
    use value::{Value, Cursor};

    #[test]
    fn test_rug() {
        let v = Value::Null;
        let p0 = Cursor { value: Some(&v) };

        p0.opt();
    }
}#[cfg(test)]
mod tests_rug_16 {
    use super::*;
    use crate::value::{Value, Cursor}; // Assuming these are used in the test
    
    #[test]
    fn test_rug() {
        let v = Value::Undefined;
        let p0 = Cursor { value: Some(&v) };
        
        let result = p0.maybe();
        
        // Add assertions based on the expected behavior of the maybe function
        // For example, assert_eq!(result, Some(...));
    }
}
#[cfg(test)]
mod tests_rug_17 {
    use super::*;
    use crate::value::{Cursor, Value};

    #[test]
    fn test_rug() {
        let mut p0: Cursor = Cursor {
            value: Some(&Value::Bool(true)),
        };
        
        assert_eq!(p0.bool(), Some(true));
    }
}
#[cfg(test)]
mod tests_rug_19 {
    use super::*;
    use crate::value::{Value, Bytes, Cursor};

    #[test]
    fn test_rug() {
        let x_bytes = vec![0, 1, 2];
        let bytes = Bytes::Bytes(x_bytes);
        let value = Value::Bytes(bytes);
        let cursor_value = Some(&value);
        let x_cursor = Cursor { value: cursor_value };

        assert_eq!(Some(&vec![0, 1, 2]), <Cursor>::bytes_plain(&x_cursor));
    }
}#[cfg(test)]
mod tests_rug_20 {
    use super::*;
    use crate::value::Cursor;

    #[test]
    fn test_rug() {
        let mut p0: Cursor<'_> = todo!();

        p0.bytes_chunked();
    }
}#[cfg(test)]
mod tests_rug_22 {
    use super::*;
    use value::{Cursor, Value, Text};

    #[test]
    fn test_text_plain() {
        let x = String::from("sample text");
        let text = Text::Text(x);
        let value = Value::Text(text);
        let cursor = Cursor { value: Some(&value) };

        assert_eq!(cursor.text_plain(), Some(&String::from("sample text")));
    }
}#[cfg(test)]
mod tests_rug_23 {
    use super::*;

    use crate::value::{Value, Text, Cursor};
    use std::collections::LinkedList;

    #[test]
    fn test_rug() {
        let chunks: LinkedList<String> = LinkedList::new();
        let text = Text::Chunks(chunks);
        let value = Value::Text(text);
        let cursor_value = Some(&value);
        let cursor = Cursor { value: cursor_value };

        cursor.text_chunked();
    }
}#[cfg(test)]
mod tests_rug_24 {
    use super::*;
    use crate::value::{Cursor, Value};

    #[test]
    fn test_float32() {
        let p0: Cursor = Cursor {
            value: Some(&Value::F32(3.14)),
        };

        assert_eq!(p0.float32(), Some(3.14));
    }
}#[cfg(test)]
mod tests_rug_25 {
    use super::*;
    use crate::value::{Value, Cursor};

    #[test]
    fn test_float64() {
        let mut p0 = Cursor { value: Some(&Value::F64(3.14)) };

        assert_eq!(p0.float64(), Some(3.14));
    }
}#[cfg(test)]
mod tests_rug_26 {
    use super::*;
    use crate::value::{Value, Cursor};

    #[test]
    fn test_rug() {
        let mut p0: Cursor<'_> = Cursor { value: Some(&Value::U8(42)) };
        
        assert_eq!(p0.u8(), Some(42));
    }
}#[cfg(test)]
mod tests_rug_27 {
    use super::*;
    use value::Cursor;

    #[test]
    fn test_rug() {
        let value = Value::U16(42);
        let p0 = Cursor { value: Some(&value) };

        let result = Cursor::u16(&p0);
        assert_eq!(result, Some(42));
    }
}#[cfg(test)]
mod tests_rug_28 {
    use super::*;
    use crate::value::Cursor;
    
    #[test]
    fn test_u32() {
        let value = Value::U32(42);
        let cursor = Cursor{ value: Some(&value) };

        let result = cursor.u32();
        assert_eq!(result, Some(42));
    }
}#[cfg(test)]
mod tests_rug_29 {
    use super::*;
    use crate::value::Cursor;
    use crate::value::Value;

    #[test]
    fn test_rug() {
        let mut p0 = Cursor {
            value: Some(&Value::U64(42)),
        };

        assert_eq!(p0.u64(), Some(42));
    }
}#[cfg(test)]
mod tests_rug_30 {
    use super::*;
    use crate::value::{Value, Cursor};

    #[test]
    fn test_rug() {
        let mut p0: Cursor<'_> = Cursor { value: Some(&Value::I8(42)) };

        assert_eq!(p0.i8(), Some(42));
    }
}#[cfg(test)]
mod tests_rug_31 {
    use super::*;
    use crate::value::{Value, Cursor};

    #[test]
    fn test_i16() {
        let value = Value::I16(42);
        let cursor = Cursor { value: Some(&value) };

        assert_eq!(cursor.i16(), Some(42));
    }
}#[cfg(test)]
mod tests_rug_32 {
    use super::*;
    use value::{Value, Cursor};

    #[test]
    fn test_rug() {
        let mut p0: Cursor<'_> = Cursor { value: Some(&Value::I32(42)) };

        assert_eq!(p0.i32(), Some(42));
    }
}#[cfg(test)]
mod tests_rug_33 {
    use super::*;
    use crate::value::{Value, Cursor};

    #[test]
    fn test_rug() {
        let mut p0: Cursor<'_>;  

        let val: Value = Value::I64(42);
        p0 = Cursor { value: Some(&val) };

        assert_eq!(Some(42), p0.i64());
    }
}