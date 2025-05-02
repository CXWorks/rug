//! Serializing Rust structures into TOML.
//!
//! This module contains all the Serde support for serializing Rust structures
//! into TOML documents (as strings). Note that some top-level functions here
//! are also provided at the top of the crate.
/// Serialize the given data structure as a String of TOML.
///
/// Serialization can fail if `T`'s implementation of `Serialize` decides to
/// fail, if `T` contains a map with non-string keys, or if `T` attempts to
/// serialize an unsupported datatype such as an enum, tuple, or tuple struct.
///
/// To serialize TOML values, instead of documents, see [`ValueSerializer`].
///
/// # Examples
///
/// ```
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Config {
///     database: Database,
/// }
///
/// #[derive(Serialize)]
/// struct Database {
///     ip: String,
///     port: Vec<u16>,
///     connection_max: u32,
///     enabled: bool,
/// }
///
/// let config = Config {
///     database: Database {
///         ip: "192.168.1.1".to_string(),
///         port: vec![8001, 8002, 8003],
///         connection_max: 5000,
///         enabled: false,
///     },
/// };
///
/// let toml = toml::to_string(&config).unwrap();
/// println!("{}", toml)
/// ```
#[cfg(feature = "display")]
pub fn to_string<T: ?Sized>(value: &T) -> Result<String, Error>
where
    T: serde::ser::Serialize,
{
    let mut output = String::new();
    let serializer = Serializer::new(&mut output);
    value.serialize(serializer)?;
    Ok(output)
}
/// Serialize the given data structure as a "pretty" String of TOML.
///
/// This is identical to `to_string` except the output string has a more
/// "pretty" output. See `Serializer::pretty` for more details.
///
/// To serialize TOML values, instead of documents, see [`ValueSerializer`].
///
/// For greater customization, instead serialize to a
/// [`toml_edit::Document`](https://docs.rs/toml_edit/latest/toml_edit/struct.Document.html).
#[cfg(feature = "display")]
pub fn to_string_pretty<T: ?Sized>(value: &T) -> Result<String, Error>
where
    T: serde::ser::Serialize,
{
    let mut output = String::new();
    let serializer = Serializer::pretty(&mut output);
    value.serialize(serializer)?;
    Ok(output)
}
/// Errors that can occur when serializing a type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub(crate) inner: crate::edit::ser::Error,
}
impl Error {
    pub(crate) fn new(inner: impl std::fmt::Display) -> Self {
        Self {
            inner: crate::edit::ser::Error::Custom(inner.to_string()),
        }
    }
    #[cfg(feature = "display")]
    pub(crate) fn wrap(inner: crate::edit::ser::Error) -> Self {
        Self { inner }
    }
    pub(crate) fn unsupported_type(t: Option<&'static str>) -> Self {
        Self {
            inner: crate::edit::ser::Error::UnsupportedType(t),
        }
    }
    pub(crate) fn unsupported_none() -> Self {
        Self {
            inner: crate::edit::ser::Error::UnsupportedNone,
        }
    }
    pub(crate) fn key_not_string() -> Self {
        Self {
            inner: crate::edit::ser::Error::KeyNotString,
        }
    }
}
impl serde::ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        Error::new(msg)
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}
impl std::error::Error for Error {}
/// Serialization for TOML documents.
///
/// This structure implements serialization support for TOML to serialize an
/// arbitrary type to TOML. Note that the TOML format does not support all
/// datatypes in Rust, such as enums, tuples, and tuple structs. These types
/// will generate an error when serialized.
///
/// Currently a serializer always writes its output to an in-memory `String`,
/// which is passed in when creating the serializer itself.
///
/// To serialize TOML values, instead of documents, see [`ValueSerializer`].
#[non_exhaustive]
#[cfg(feature = "display")]
pub struct Serializer<'d> {
    dst: &'d mut String,
    settings: crate::fmt::DocumentFormatter,
}
#[cfg(feature = "display")]
impl<'d> Serializer<'d> {
    /// Creates a new serializer which will emit TOML into the buffer provided.
    ///
    /// The serializer can then be used to serialize a type after which the data
    /// will be present in `dst`.
    pub fn new(dst: &'d mut String) -> Self {
        Self {
            dst,
            settings: Default::default(),
        }
    }
    /// Apply a default "pretty" policy to the document
    ///
    /// For greater customization, instead serialize to a
    /// [`toml_edit::Document`](https://docs.rs/toml_edit/latest/toml_edit/struct.Document.html).
    pub fn pretty(dst: &'d mut String) -> Self {
        let mut ser = Serializer::new(dst);
        ser.settings.multiline_array = true;
        ser
    }
}
#[cfg(feature = "display")]
impl<'d> serde::ser::Serializer for Serializer<'d> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = SerializeDocumentArray<'d>;
    type SerializeTuple = SerializeDocumentArray<'d>;
    type SerializeTupleStruct = SerializeDocumentArray<'d>;
    type SerializeTupleVariant = SerializeDocumentArray<'d>;
    type SerializeMap = SerializeDocumentTable<'d>;
    type SerializeStruct = SerializeDocumentTable<'d>;
    type SerializeStructVariant = serde::ser::Impossible<Self::Ok, Self::Error>;
    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        write_document(
            self.dst,
            self.settings,
            toml_edit::ser::ValueSerializer::new().serialize_bool(v),
        )
    }
    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        write_document(
            self.dst,
            self.settings,
            toml_edit::ser::ValueSerializer::new().serialize_i8(v),
        )
    }
    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        write_document(
            self.dst,
            self.settings,
            toml_edit::ser::ValueSerializer::new().serialize_i16(v),
        )
    }
    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        write_document(
            self.dst,
            self.settings,
            toml_edit::ser::ValueSerializer::new().serialize_i32(v),
        )
    }
    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        write_document(
            self.dst,
            self.settings,
            toml_edit::ser::ValueSerializer::new().serialize_i64(v),
        )
    }
    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        write_document(
            self.dst,
            self.settings,
            toml_edit::ser::ValueSerializer::new().serialize_u8(v),
        )
    }
    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        write_document(
            self.dst,
            self.settings,
            toml_edit::ser::ValueSerializer::new().serialize_u16(v),
        )
    }
    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        write_document(
            self.dst,
            self.settings,
            toml_edit::ser::ValueSerializer::new().serialize_u32(v),
        )
    }
    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        write_document(
            self.dst,
            self.settings,
            toml_edit::ser::ValueSerializer::new().serialize_u64(v),
        )
    }
    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        write_document(
            self.dst,
            self.settings,
            toml_edit::ser::ValueSerializer::new().serialize_f32(v),
        )
    }
    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        write_document(
            self.dst,
            self.settings,
            toml_edit::ser::ValueSerializer::new().serialize_f64(v),
        )
    }
    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        write_document(
            self.dst,
            self.settings,
            toml_edit::ser::ValueSerializer::new().serialize_char(v),
        )
    }
    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        write_document(
            self.dst,
            self.settings,
            toml_edit::ser::ValueSerializer::new().serialize_str(v),
        )
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        write_document(
            self.dst,
            self.settings,
            toml_edit::ser::ValueSerializer::new().serialize_bytes(v),
        )
    }
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        write_document(
            self.dst,
            self.settings,
            toml_edit::ser::ValueSerializer::new().serialize_none(),
        )
    }
    fn serialize_some<T: ?Sized>(self, v: &T) -> Result<Self::Ok, Self::Error>
    where
        T: serde::ser::Serialize,
    {
        write_document(
            self.dst,
            self.settings,
            toml_edit::ser::ValueSerializer::new().serialize_some(v),
        )
    }
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        write_document(
            self.dst,
            self.settings,
            toml_edit::ser::ValueSerializer::new().serialize_unit(),
        )
    }
    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        write_document(
            self.dst,
            self.settings,
            toml_edit::ser::ValueSerializer::new().serialize_unit_struct(name),
        )
    }
    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        write_document(
            self.dst,
            self.settings,
            toml_edit::ser::ValueSerializer::new()
                .serialize_unit_variant(name, variant_index, variant),
        )
    }
    fn serialize_newtype_struct<T: ?Sized>(
        self,
        name: &'static str,
        v: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::ser::Serialize,
    {
        write_document(
            self.dst,
            self.settings,
            toml_edit::ser::ValueSerializer::new().serialize_newtype_struct(name, v),
        )
    }
    fn serialize_newtype_variant<T: ?Sized>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::ser::Serialize,
    {
        write_document(
            self.dst,
            self.settings,
            toml_edit::ser::ValueSerializer::new()
                .serialize_newtype_variant(name, variant_index, variant, value),
        )
    }
    fn serialize_seq(
        self,
        len: Option<usize>,
    ) -> Result<Self::SerializeSeq, Self::Error> {
        let ser = toml_edit::ser::ValueSerializer::new()
            .serialize_seq(len)
            .map_err(Error::wrap)?;
        let ser = SerializeDocumentArray::new(self, ser);
        Ok(ser)
    }
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        self.serialize_seq(Some(len))
    }
    fn serialize_map(
        self,
        len: Option<usize>,
    ) -> Result<Self::SerializeMap, Self::Error> {
        let ser = toml_edit::ser::ValueSerializer::new()
            .serialize_map(len)
            .map_err(Error::wrap)?;
        let ser = SerializeDocumentTable::new(self, ser);
        Ok(ser)
    }
    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.serialize_map(Some(len))
    }
    fn serialize_struct_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(Error::unsupported_type(Some(name)))
    }
}
/// Serialization for TOML [values][crate::Value].
///
/// This structure implements serialization support for TOML to serialize an
/// arbitrary type to TOML. Note that the TOML format does not support all
/// datatypes in Rust, such as enums, tuples, and tuple structs. These types
/// will generate an error when serialized.
///
/// Currently a serializer always writes its output to an in-memory `String`,
/// which is passed in when creating the serializer itself.
///
/// # Examples
///
/// ```
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Config {
///     database: Database,
/// }
///
/// #[derive(Serialize)]
/// struct Database {
///     ip: String,
///     port: Vec<u16>,
///     connection_max: u32,
///     enabled: bool,
/// }
///
/// let config = Config {
///     database: Database {
///         ip: "192.168.1.1".to_string(),
///         port: vec![8001, 8002, 8003],
///         connection_max: 5000,
///         enabled: false,
///     },
/// };
///
/// let mut value = String::new();
/// serde::Serialize::serialize(
///     &config,
///     toml::ser::ValueSerializer::new(&mut value)
/// ).unwrap();
/// println!("{}", value)
/// ```
#[non_exhaustive]
#[cfg(feature = "display")]
pub struct ValueSerializer<'d> {
    dst: &'d mut String,
}
#[cfg(feature = "display")]
impl<'d> ValueSerializer<'d> {
    /// Creates a new serializer which will emit TOML into the buffer provided.
    ///
    /// The serializer can then be used to serialize a type after which the data
    /// will be present in `dst`.
    pub fn new(dst: &'d mut String) -> Self {
        Self { dst }
    }
}
#[cfg(feature = "display")]
impl<'d> serde::ser::Serializer for ValueSerializer<'d> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = SerializeValueArray<'d>;
    type SerializeTuple = SerializeValueArray<'d>;
    type SerializeTupleStruct = SerializeValueArray<'d>;
    type SerializeTupleVariant = SerializeValueArray<'d>;
    type SerializeMap = SerializeValueTable<'d>;
    type SerializeStruct = SerializeValueTable<'d>;
    type SerializeStructVariant = serde::ser::Impossible<Self::Ok, Self::Error>;
    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        write_value(self.dst, toml_edit::ser::ValueSerializer::new().serialize_bool(v))
    }
    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        write_value(self.dst, toml_edit::ser::ValueSerializer::new().serialize_i8(v))
    }
    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        write_value(self.dst, toml_edit::ser::ValueSerializer::new().serialize_i16(v))
    }
    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        write_value(self.dst, toml_edit::ser::ValueSerializer::new().serialize_i32(v))
    }
    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        write_value(self.dst, toml_edit::ser::ValueSerializer::new().serialize_i64(v))
    }
    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        write_value(self.dst, toml_edit::ser::ValueSerializer::new().serialize_u8(v))
    }
    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        write_value(self.dst, toml_edit::ser::ValueSerializer::new().serialize_u16(v))
    }
    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        write_value(self.dst, toml_edit::ser::ValueSerializer::new().serialize_u32(v))
    }
    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        write_value(self.dst, toml_edit::ser::ValueSerializer::new().serialize_u64(v))
    }
    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        write_value(self.dst, toml_edit::ser::ValueSerializer::new().serialize_f32(v))
    }
    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        write_value(self.dst, toml_edit::ser::ValueSerializer::new().serialize_f64(v))
    }
    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        write_value(self.dst, toml_edit::ser::ValueSerializer::new().serialize_char(v))
    }
    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        write_value(self.dst, toml_edit::ser::ValueSerializer::new().serialize_str(v))
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        write_value(self.dst, toml_edit::ser::ValueSerializer::new().serialize_bytes(v))
    }
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        write_value(self.dst, toml_edit::ser::ValueSerializer::new().serialize_none())
    }
    fn serialize_some<T: ?Sized>(self, v: &T) -> Result<Self::Ok, Self::Error>
    where
        T: serde::ser::Serialize,
    {
        write_value(self.dst, toml_edit::ser::ValueSerializer::new().serialize_some(v))
    }
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        write_value(self.dst, toml_edit::ser::ValueSerializer::new().serialize_unit())
    }
    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        write_value(
            self.dst,
            toml_edit::ser::ValueSerializer::new().serialize_unit_struct(name),
        )
    }
    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        write_value(
            self.dst,
            toml_edit::ser::ValueSerializer::new()
                .serialize_unit_variant(name, variant_index, variant),
        )
    }
    fn serialize_newtype_struct<T: ?Sized>(
        self,
        name: &'static str,
        v: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::ser::Serialize,
    {
        write_value(
            self.dst,
            toml_edit::ser::ValueSerializer::new().serialize_newtype_struct(name, v),
        )
    }
    fn serialize_newtype_variant<T: ?Sized>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::ser::Serialize,
    {
        write_value(
            self.dst,
            toml_edit::ser::ValueSerializer::new()
                .serialize_newtype_variant(name, variant_index, variant, value),
        )
    }
    fn serialize_seq(
        self,
        len: Option<usize>,
    ) -> Result<Self::SerializeSeq, Self::Error> {
        let ser = toml_edit::ser::ValueSerializer::new()
            .serialize_seq(len)
            .map_err(Error::wrap)?;
        let ser = SerializeValueArray::new(self, ser);
        Ok(ser)
    }
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        self.serialize_seq(Some(len))
    }
    fn serialize_map(
        self,
        len: Option<usize>,
    ) -> Result<Self::SerializeMap, Self::Error> {
        let ser = toml_edit::ser::ValueSerializer::new()
            .serialize_map(len)
            .map_err(Error::wrap)?;
        let ser = SerializeValueTable::new(self, ser);
        Ok(ser)
    }
    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.serialize_map(Some(len))
    }
    fn serialize_struct_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(Error::unsupported_type(Some(name)))
    }
}
#[cfg(feature = "display")]
use internal::*;
#[cfg(feature = "display")]
mod internal {
    use super::*;
    use crate::fmt::DocumentFormatter;
    type InnerSerializeDocumentSeq = <toml_edit::ser::ValueSerializer as serde::Serializer>::SerializeSeq;
    #[doc(hidden)]
    pub struct SerializeDocumentArray<'d> {
        inner: InnerSerializeDocumentSeq,
        dst: &'d mut String,
        settings: DocumentFormatter,
    }
    impl<'d> SerializeDocumentArray<'d> {
        pub(crate) fn new(
            ser: Serializer<'d>,
            inner: InnerSerializeDocumentSeq,
        ) -> Self {
            Self {
                inner,
                dst: ser.dst,
                settings: ser.settings,
            }
        }
    }
    impl<'d> serde::ser::SerializeSeq for SerializeDocumentArray<'d> {
        type Ok = ();
        type Error = Error;
        fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Error>
        where
            T: serde::ser::Serialize,
        {
            self.inner.serialize_element(value).map_err(Error::wrap)
        }
        fn end(self) -> Result<Self::Ok, Self::Error> {
            write_document(self.dst, self.settings, self.inner.end())
        }
    }
    impl<'d> serde::ser::SerializeTuple for SerializeDocumentArray<'d> {
        type Ok = ();
        type Error = Error;
        fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Error>
        where
            T: serde::ser::Serialize,
        {
            self.inner.serialize_element(value).map_err(Error::wrap)
        }
        fn end(self) -> Result<Self::Ok, Self::Error> {
            write_document(self.dst, self.settings, self.inner.end())
        }
    }
    impl<'d> serde::ser::SerializeTupleVariant for SerializeDocumentArray<'d> {
        type Ok = ();
        type Error = Error;
        fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Error>
        where
            T: serde::ser::Serialize,
        {
            self.inner.serialize_field(value).map_err(Error::wrap)
        }
        fn end(self) -> Result<Self::Ok, Self::Error> {
            write_document(self.dst, self.settings, self.inner.end())
        }
    }
    impl<'d> serde::ser::SerializeTupleStruct for SerializeDocumentArray<'d> {
        type Ok = ();
        type Error = Error;
        fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Error>
        where
            T: serde::ser::Serialize,
        {
            self.inner.serialize_field(value).map_err(Error::wrap)
        }
        fn end(self) -> Result<Self::Ok, Self::Error> {
            write_document(self.dst, self.settings, self.inner.end())
        }
    }
    type InnerSerializeDocumentTable = <toml_edit::ser::ValueSerializer as serde::Serializer>::SerializeMap;
    #[doc(hidden)]
    pub struct SerializeDocumentTable<'d> {
        inner: InnerSerializeDocumentTable,
        dst: &'d mut String,
        settings: DocumentFormatter,
    }
    impl<'d> SerializeDocumentTable<'d> {
        pub(crate) fn new(
            ser: Serializer<'d>,
            inner: InnerSerializeDocumentTable,
        ) -> Self {
            Self {
                inner,
                dst: ser.dst,
                settings: ser.settings,
            }
        }
    }
    impl<'d> serde::ser::SerializeMap for SerializeDocumentTable<'d> {
        type Ok = ();
        type Error = Error;
        fn serialize_key<T: ?Sized>(&mut self, input: &T) -> Result<(), Self::Error>
        where
            T: serde::ser::Serialize,
        {
            self.inner.serialize_key(input).map_err(Error::wrap)
        }
        fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
        where
            T: serde::ser::Serialize,
        {
            self.inner.serialize_value(value).map_err(Error::wrap)
        }
        fn end(self) -> Result<Self::Ok, Self::Error> {
            write_document(self.dst, self.settings, self.inner.end())
        }
    }
    impl<'d> serde::ser::SerializeStruct for SerializeDocumentTable<'d> {
        type Ok = ();
        type Error = Error;
        fn serialize_field<T: ?Sized>(
            &mut self,
            key: &'static str,
            value: &T,
        ) -> Result<(), Self::Error>
        where
            T: serde::ser::Serialize,
        {
            self.inner.serialize_field(key, value).map_err(Error::wrap)
        }
        fn end(self) -> Result<Self::Ok, Self::Error> {
            write_document(self.dst, self.settings, self.inner.end())
        }
    }
    pub(crate) fn write_document(
        dst: &mut String,
        mut settings: DocumentFormatter,
        value: Result<toml_edit::Value, crate::edit::ser::Error>,
    ) -> Result<(), Error> {
        use std::fmt::Write;
        let value = value.map_err(Error::wrap)?;
        let mut table = match toml_edit::Item::Value(value).into_table() {
            Ok(i) => i,
            Err(_) => {
                return Err(Error::unsupported_type(None));
            }
        };
        use toml_edit::visit_mut::VisitMut as _;
        settings.visit_table_mut(&mut table);
        let doc: toml_edit::Document = table.into();
        write!(dst, "{}", doc).unwrap();
        Ok(())
    }
    type InnerSerializeValueSeq = <toml_edit::ser::ValueSerializer as serde::Serializer>::SerializeSeq;
    #[doc(hidden)]
    pub struct SerializeValueArray<'d> {
        inner: InnerSerializeValueSeq,
        dst: &'d mut String,
    }
    impl<'d> SerializeValueArray<'d> {
        pub(crate) fn new(
            ser: ValueSerializer<'d>,
            inner: InnerSerializeValueSeq,
        ) -> Self {
            Self { inner, dst: ser.dst }
        }
    }
    impl<'d> serde::ser::SerializeSeq for SerializeValueArray<'d> {
        type Ok = ();
        type Error = Error;
        fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Error>
        where
            T: serde::ser::Serialize,
        {
            self.inner.serialize_element(value).map_err(Error::wrap)
        }
        fn end(self) -> Result<Self::Ok, Self::Error> {
            write_value(self.dst, self.inner.end())
        }
    }
    impl<'d> serde::ser::SerializeTuple for SerializeValueArray<'d> {
        type Ok = ();
        type Error = Error;
        fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Error>
        where
            T: serde::ser::Serialize,
        {
            self.inner.serialize_element(value).map_err(Error::wrap)
        }
        fn end(self) -> Result<Self::Ok, Self::Error> {
            write_value(self.dst, self.inner.end())
        }
    }
    impl<'d> serde::ser::SerializeTupleVariant for SerializeValueArray<'d> {
        type Ok = ();
        type Error = Error;
        fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Error>
        where
            T: serde::ser::Serialize,
        {
            self.inner.serialize_field(value).map_err(Error::wrap)
        }
        fn end(self) -> Result<Self::Ok, Self::Error> {
            write_value(self.dst, self.inner.end())
        }
    }
    impl<'d> serde::ser::SerializeTupleStruct for SerializeValueArray<'d> {
        type Ok = ();
        type Error = Error;
        fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Error>
        where
            T: serde::ser::Serialize,
        {
            self.inner.serialize_field(value).map_err(Error::wrap)
        }
        fn end(self) -> Result<Self::Ok, Self::Error> {
            write_value(self.dst, self.inner.end())
        }
    }
    type InnerSerializeValueTable = <toml_edit::ser::ValueSerializer as serde::Serializer>::SerializeMap;
    #[doc(hidden)]
    pub struct SerializeValueTable<'d> {
        inner: InnerSerializeValueTable,
        dst: &'d mut String,
    }
    impl<'d> SerializeValueTable<'d> {
        pub(crate) fn new(
            ser: ValueSerializer<'d>,
            inner: InnerSerializeValueTable,
        ) -> Self {
            Self { inner, dst: ser.dst }
        }
    }
    impl<'d> serde::ser::SerializeMap for SerializeValueTable<'d> {
        type Ok = ();
        type Error = Error;
        fn serialize_key<T: ?Sized>(&mut self, input: &T) -> Result<(), Self::Error>
        where
            T: serde::ser::Serialize,
        {
            self.inner.serialize_key(input).map_err(Error::wrap)
        }
        fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
        where
            T: serde::ser::Serialize,
        {
            self.inner.serialize_value(value).map_err(Error::wrap)
        }
        fn end(self) -> Result<Self::Ok, Self::Error> {
            write_value(self.dst, self.inner.end())
        }
    }
    impl<'d> serde::ser::SerializeStruct for SerializeValueTable<'d> {
        type Ok = ();
        type Error = Error;
        fn serialize_field<T: ?Sized>(
            &mut self,
            key: &'static str,
            value: &T,
        ) -> Result<(), Self::Error>
        where
            T: serde::ser::Serialize,
        {
            self.inner.serialize_field(key, value).map_err(Error::wrap)
        }
        fn end(self) -> Result<Self::Ok, Self::Error> {
            write_value(self.dst, self.inner.end())
        }
    }
    pub(crate) fn write_value(
        dst: &mut String,
        value: Result<toml_edit::Value, crate::edit::ser::Error>,
    ) -> Result<(), Error> {
        use std::fmt::Write;
        let value = value.map_err(Error::wrap)?;
        write!(dst, "{}", value).unwrap();
        Ok(())
    }
}
#[cfg(test)]
mod tests_rug_57 {
    use super::*;
    use serde::{Serialize, ser::SerializeMap};
    #[derive(Serialize)]
    struct Config {
        database: Database,
    }
    #[derive(Serialize)]
    struct Database {
        ip: String,
        port: Vec<u16>,
        connection_max: u32,
        enabled: bool,
    }
    #[test]
    fn test_to_string() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(&str, u16, u32, bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let config = Config {
            database: Database {
                ip: rug_fuzz_0.to_string(),
                port: vec![rug_fuzz_1, 8002, 8003],
                connection_max: rug_fuzz_2,
                enabled: rug_fuzz_3,
            },
        };
        let toml = to_string(&config).unwrap();
        debug_assert_eq!(
            toml,
            "database.ip = \"192.168.1.1\"\n\
                         database.port = [8001, 8002, 8003]\n\
                         database.connection_max = 5000\n\
                         database.enabled = false\n"
        );
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_58 {
    use super::*;
    use serde::Serialize;
    use crate::ser::Serializer;
    use crate::map::Map;
    #[derive(Serialize)]
    struct MyData {
        name: String,
        age: u32,
        enabled: bool,
    }
    #[test]
    fn test_to_string_pretty() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(&str, u32, bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let data = MyData {
            name: String::from(rug_fuzz_0),
            age: rug_fuzz_1,
            enabled: rug_fuzz_2,
        };
        let result = to_string_pretty(&data).unwrap();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_60 {
    use super::*;
    use toml_edit::Value;
    use crate::edit::ser::Error;
    #[test]
    fn test_write_value() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = String::new();
        let p1: Result<Value, Error> = Ok(Value::from(rug_fuzz_0));
        crate::ser::internal::write_value(&mut p0, p1);
        debug_assert_eq!(p0, "42");
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_61 {
    use super::*;
    use serde::Serialize;
    use crate::ser::Error;
    #[derive(Debug, Serialize)]
    struct MyStruct {
        field1: String,
        field2: i32,
    }
    #[test]
    fn test_ser_error_new() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let inner = rug_fuzz_0;
        let error = Error::new(inner);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_63 {
    use super::*;
    use crate::ser;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_63_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "example";
        let p0: Option<&'static str> = Some(rug_fuzz_0);
        <ser::Error>::unsupported_type(p0);
        let _rug_ed_tests_rug_63_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_64 {
    use super::*;
    use crate::ser::Error;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_64_rrrruuuugggg_test_rug = 0;
        Error::unsupported_none();
        let _rug_ed_tests_rug_64_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_65 {
    use super::*;
    use crate::ser::{self, Error};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_65_rrrruuuugggg_test_rug = 0;
        Error::key_not_string();
        let _rug_ed_tests_rug_65_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_67 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_67_rrrruuuugggg_test_rug = 0;
        let mut p0 = String::new();
        let serializer = crate::ser::Serializer::new(&mut p0);
        let _rug_ed_tests_rug_67_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_68 {
    use super::*;
    use crate::ser::{Serializer, to_string_pretty};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_68_rrrruuuugggg_test_rug = 0;
        let mut p0 = String::new();
        <Serializer<'_>>::pretty(&mut p0);
        let _rug_ed_tests_rug_68_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_97 {
    use super::*;
    use std::string::String;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: String = rug_fuzz_0.to_string();
        crate::ser::ValueSerializer::new(&mut p0);
             }
}
}
}    }
}
