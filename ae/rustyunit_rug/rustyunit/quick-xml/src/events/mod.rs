//! Defines zero-copy XML events used throughout this library.
//!
//! A XML event often represents part of a XML element.
//! They occur both during reading and writing and are
//! usually used with the stream-oriented API.
//!
//! For example, the XML element
//! ```xml
//! <name attr="value">Inner text</name>
//! ```
//! consists of the three events `Start`, `Text` and `End`.
//! They can also represent other parts in an XML document like the
//! XML declaration. Each Event usually contains further information,
//! like the tag name, the attribute or the inner text.
//!
//! See [`Event`] for a list of all possible events.
//!
//! # Reading
//! When reading a XML stream, the events are emitted by
//! [`Reader::read_event`]. You must listen
//! for the different types of events you are interested in.
//!
//! See [`Reader`] for further information.
//!
//! # Writing
//! When writing the XML document, you must create the XML element
//! by constructing the events it consists of and pass them to the writer
//! sequentially.
//!
//! See [`Writer`] for further information.
//!
//! [`Reader::read_event`]: ../reader/struct.Reader.html#method.read_event
//! [`Reader`]: ../reader/struct.Reader.html
//! [`Writer`]: ../writer/struct.Writer.html
//! [`Event`]: enum.Event.html

pub mod attributes;

#[cfg(feature = "encoding_rs")]
use encoding_rs::Encoding;
use std::{borrow::Cow, collections::HashMap, io::BufRead, ops::Deref, str::from_utf8};

use crate::escape::{do_unescape, escape, partial_escape};
use crate::utils::write_cow_string;
use crate::{errors::Error, errors::Result, reader::Reader};
use attributes::{Attribute, Attributes};

#[cfg(feature = "serialize")]
use crate::escape::EscapeError;

use memchr;

/// Opening tag data (`Event::Start`), with optional attributes.
///
/// `<name attr="value">`.
///
/// The name can be accessed using the [`name`], [`local_name`] or [`unescaped`] methods. An
/// iterator over the attributes is returned by the [`attributes`] method.
///
/// [`name`]: #method.name
/// [`local_name`]: #method.local_name
/// [`unescaped`]: #method.unescaped
/// [`attributes`]: #method.attributes
#[derive(Clone, Eq, PartialEq)]
pub struct BytesStart<'a> {
    /// content of the element, before any utf8 conversion
    buf: Cow<'a, [u8]>,
    /// end of the element name, the name starts at that the start of `buf`
    name_len: usize,
}

impl<'a> BytesStart<'a> {
    /// Creates a new `BytesStart` from the given content (name + attributes).
    ///
    /// # Warning
    ///
    /// `&content[..name_len]` is not checked to be a valid name

    pub fn borrowed(content: &'a [u8], name_len: usize) -> Self {
        BytesStart {
            buf: Cow::Borrowed(content),
            name_len,
        }
    }

    /// Creates a new `BytesStart` from the given name.
    ///
    /// # Warning
    ///
    /// `&content` is not checked to be a valid name

    pub fn borrowed_name(name: &'a [u8]) -> BytesStart<'a> {
        Self::borrowed(name, name.len())
    }

    /// Creates a new `BytesStart` from the given content (name + attributes)
    ///
    /// Owns its contents.

    pub fn owned<C: Into<Vec<u8>>>(content: C, name_len: usize) -> BytesStart<'static> {
        BytesStart {
            buf: Cow::Owned(content.into()),
            name_len,
        }
    }

    /// Creates a new `BytesStart` from the given name
    ///
    /// Owns its contents.

    pub fn owned_name<C: Into<Vec<u8>>>(name: C) -> BytesStart<'static> {
        let content = name.into();
        BytesStart {
            name_len: content.len(),
            buf: Cow::Owned(content),
        }
    }

    /// Converts the event into an owned event.
    pub fn into_owned(self) -> BytesStart<'static> {
        Self::owned(self.buf.into_owned(), self.name_len)
    }

    /// Converts the event into an owned event without taking ownership of Event
    pub fn to_owned(&self) -> BytesStart<'static> {
        Self::owned(self.buf.to_owned(), self.name_len)
    }

    /// Converts the event into a borrowed event. Most useful when paired with [`to_end`].
    ///
    /// # Example
    ///
    /// ```rust
    /// # use quick_xml::{Error, Writer};
    /// use quick_xml::events::{BytesStart, Event};
    ///
    /// struct SomeStruct<'a> {
    ///     attrs: BytesStart<'a>,
    ///     // ...
    /// }
    /// # impl<'a> SomeStruct<'a> {
    /// # fn example(&self) -> Result<(), Error> {
    /// # let mut writer = Writer::new(Vec::new());
    ///
    /// writer.write_event(Event::Start(self.attrs.to_borrowed()))?;
    /// // ...
    /// writer.write_event(Event::End(self.attrs.to_end()))?;
    /// # Ok(())
    /// # }}
    /// ```
    ///
    /// [`to_end`]: #method.to_end
    pub fn to_borrowed(&self) -> BytesStart {
        BytesStart::borrowed(&self.buf, self.name_len)
    }

    /// Creates new paired close tag
    pub fn to_end(&self) -> BytesEnd {
        BytesEnd::borrowed(self.name())
    }

    /// Gets the undecoded raw tag name as a `&[u8]`.

    pub fn name(&self) -> &[u8] {
        &self.buf[..self.name_len]
    }

    /// Gets the undecoded raw local tag name (excluding namespace) as a `&[u8]`.
    ///
    /// All content up to and including the first `:` character is removed from the tag name.

    pub fn local_name(&self) -> &[u8] {
        let name = self.name();
        memchr::memchr(b':', name).map_or(name, |i| &name[i + 1..])
    }

    /// Gets the unescaped tag name.
    ///
    /// XML escape sequences like "`&lt;`" will be replaced by their unescaped characters like
    /// "`<`".
    ///
    /// See also [`unescaped_with_custom_entities()`](#method.unescaped_with_custom_entities)

    pub fn unescaped(&self) -> Result<Cow<[u8]>> {
        self.make_unescaped(None)
    }

    /// Gets the unescaped tag name, using custom entities.
    ///
    /// XML escape sequences like "`&lt;`" will be replaced by their unescaped characters like
    /// "`<`".
    /// Additional entities can be provided in `custom_entities`.
    ///
    /// # Pre-condition
    ///
    /// The keys and values of `custom_entities`, if any, must be valid UTF-8.
    ///
    /// See also [`unescaped()`](#method.unescaped)

    pub fn unescaped_with_custom_entities<'s>(
        &'s self,
        custom_entities: &HashMap<Vec<u8>, Vec<u8>>,
    ) -> Result<Cow<'s, [u8]>> {
        self.make_unescaped(Some(custom_entities))
    }


    fn make_unescaped<'s>(
        &'s self,
        custom_entities: Option<&HashMap<Vec<u8>, Vec<u8>>>,
    ) -> Result<Cow<'s, [u8]>> {
        do_unescape(&*self.buf, custom_entities).map_err(Error::EscapeError)
    }

    /// Returns the unescaped and decoded string value.
    ///
    /// This allocates a `String` in all cases. For performance reasons it might be a better idea to
    /// instead use one of:
    ///
    /// * [`unescaped()`], as it doesn't allocate when no escape sequences are used.
    /// * [`Reader::decode()`], as it only allocates when the decoding can't be performed otherwise.
    ///
    /// [`unescaped()`]: #method.unescaped
    /// [`Reader::decode()`]: ../reader/struct.Reader.html#method.decode

    pub fn unescape_and_decode<B: BufRead>(&self, reader: &Reader<B>) -> Result<String> {
        self.do_unescape_and_decode_with_custom_entities(reader, None)
    }

    /// Returns the unescaped and decoded string value with custom entities.
    ///
    /// This allocates a `String` in all cases. For performance reasons it might be a better idea to
    /// instead use one of:
    ///
    /// * [`unescaped_with_custom_entities()`], as it doesn't allocate when no escape sequences are used.
    /// * [`Reader::decode()`], as it only allocates when the decoding can't be performed otherwise.
    ///
    /// [`unescaped_with_custom_entities()`]: #method.unescaped_with_custom_entities
    /// [`Reader::decode()`]: ../reader/struct.Reader.html#method.decode
    ///
    /// # Pre-condition
    ///
    /// The keys and values of `custom_entities`, if any, must be valid UTF-8.

    pub fn unescape_and_decode_with_custom_entities<B: BufRead>(
        &self,
        reader: &Reader<B>,
        custom_entities: &HashMap<Vec<u8>, Vec<u8>>,
    ) -> Result<String> {
        self.do_unescape_and_decode_with_custom_entities(reader, Some(custom_entities))
    }

    #[cfg(feature = "encoding")]

    fn do_unescape_and_decode_with_custom_entities<B: BufRead>(
        &self,
        reader: &Reader<B>,
        custom_entities: Option<&HashMap<Vec<u8>, Vec<u8>>>,
    ) -> Result<String> {
        let decoded = reader.decode(&*self);
        let unescaped =
            do_unescape(decoded.as_bytes(), custom_entities).map_err(Error::EscapeError)?;
        String::from_utf8(unescaped.into_owned()).map_err(|e| Error::Utf8(e.utf8_error()))
    }

    #[cfg(not(feature = "encoding"))]

    fn do_unescape_and_decode_with_custom_entities<B: BufRead>(
        &self,
        reader: &Reader<B>,
        custom_entities: Option<&HashMap<Vec<u8>, Vec<u8>>>,
    ) -> Result<String> {
        let decoded = reader.decode(&*self)?;
        let unescaped =
            do_unescape(decoded.as_bytes(), custom_entities).map_err(Error::EscapeError)?;
        String::from_utf8(unescaped.into_owned()).map_err(|e| Error::Utf8(e.utf8_error()))
    }

    /// Edit the name of the BytesStart in-place
    ///
    /// # Warning
    ///
    /// `name` is not checked to be a valid name
    pub fn set_name(&mut self, name: &[u8]) -> &mut BytesStart<'a> {
        let bytes = self.buf.to_mut();
        bytes.splice(..self.name_len, name.iter().cloned());
        self.name_len = name.len();
        self
    }
}

/// Attribute-related methods
impl<'a> BytesStart<'a> {
    /// Consumes `self` and yield a new `BytesStart` with additional attributes from an iterator.
    ///
    /// The yielded items must be convertible to [`Attribute`] using `Into`.
    pub fn with_attributes<'b, I>(mut self, attributes: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<Attribute<'b>>,
    {
        self.extend_attributes(attributes);
        self
    }

    /// Add additional attributes to this tag using an iterator.
    ///
    /// The yielded items must be convertible to [`Attribute`] using `Into`.
    pub fn extend_attributes<'b, I>(&mut self, attributes: I) -> &mut BytesStart<'a>
    where
        I: IntoIterator,
        I::Item: Into<Attribute<'b>>,
    {
        for attr in attributes {
            self.push_attribute(attr);
        }
        self
    }

    /// Adds an attribute to this element.
    pub fn push_attribute<'b, A>(&mut self, attr: A)
    where
        A: Into<Attribute<'b>>,
    {
        let a = attr.into();
        let bytes = self.buf.to_mut();
        bytes.push(b' ');
        bytes.extend_from_slice(a.key);
        bytes.extend_from_slice(b"=\"");
        bytes.extend_from_slice(&*a.value);
        bytes.push(b'"');
    }

    /// Remove all attributes from the ByteStart
    pub fn clear_attributes(&mut self) -> &mut BytesStart<'a> {
        self.buf.to_mut().truncate(self.name_len);
        self
    }

    /// Returns an iterator over the attributes of this tag.
    pub fn attributes(&self) -> Attributes {
        Attributes::new(&self.buf, self.name_len)
    }

    /// Returns an iterator over the HTML-like attributes of this tag (no mandatory quotes or `=`).
    pub fn html_attributes(&self) -> Attributes {
        Attributes::html(self, self.name_len)
    }

    /// Gets the undecoded raw string with the attributes of this tag as a `&[u8]`,
    /// including the whitespace after the tag name if there is any.

    pub fn attributes_raw(&self) -> &[u8] {
        &self.buf[self.name_len..]
    }

    /// Try to get an attribute
    pub fn try_get_attribute<N: AsRef<[u8]> + Sized>(
        &'a self,
        attr_name: N,
    ) -> Result<Option<Attribute<'a>>> {
        for a in self.attributes() {
            let a = a?;
            if a.key == attr_name.as_ref() {
                return Ok(Some(a));
            }
        }
        Ok(None)
    }
}

impl<'a> std::fmt::Debug for BytesStart<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "BytesStart {{ buf: ")?;
        write_cow_string(f, &self.buf)?;
        write!(f, ", name_len: {} }}", self.name_len)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

/// An XML declaration (`Event::Decl`).
///
/// [W3C XML 1.1 Prolog and Document Type Declaration](http://w3.org/TR/xml11/#sec-prolog-dtd)
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BytesDecl<'a> {
    element: BytesStart<'a>,
}

impl<'a> BytesDecl<'a> {
    /// Creates a `BytesDecl` from a `BytesStart`
    pub fn from_start(start: BytesStart<'a>) -> BytesDecl<'a> {
        BytesDecl { element: start }
    }

    /// Gets xml version, excluding quotes (`'` or `"`).
    ///
    /// According to the [grammar], the version *must* be the first thing in the declaration.
    /// This method tries to extract the first thing in the declaration and return it.
    /// In case of multiple attributes value of the first one is returned.
    ///
    /// If version is missed in the declaration, or the first thing is not a version,
    /// [`Error::XmlDeclWithoutVersion`] will be returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::borrow::Cow;
    /// use quick_xml::Error;
    /// use quick_xml::events::{BytesDecl, BytesStart};
    ///
    /// // <?xml version='1.1'?>
    /// let decl = BytesDecl::from_start(BytesStart::borrowed(b" version='1.1'", 0));
    /// assert_eq!(
    ///     decl.version().unwrap(),
    ///     Cow::Borrowed(b"1.1".as_ref())
    /// );
    ///
    /// // <?xml version='1.0' version='1.1'?>
    /// let decl = BytesDecl::from_start(BytesStart::borrowed(b" version='1.0' version='1.1'", 0));
    /// assert_eq!(
    ///     decl.version().unwrap(),
    ///     Cow::Borrowed(b"1.0".as_ref())
    /// );
    ///
    /// // <?xml encoding='utf-8'?>
    /// let decl = BytesDecl::from_start(BytesStart::borrowed(b" encoding='utf-8'", 0));
    /// match decl.version() {
    ///     Err(Error::XmlDeclWithoutVersion(Some(key))) => assert_eq!(key, "encoding".to_string()),
    ///     _ => assert!(false),
    /// }
    ///
    /// // <?xml encoding='utf-8' version='1.1'?>
    /// let decl = BytesDecl::from_start(BytesStart::borrowed(b" encoding='utf-8' version='1.1'", 0));
    /// match decl.version() {
    ///     Err(Error::XmlDeclWithoutVersion(Some(key))) => assert_eq!(key, "encoding".to_string()),
    ///     _ => assert!(false),
    /// }
    ///
    /// // <?xml?>
    /// let decl = BytesDecl::from_start(BytesStart::borrowed(b"", 0));
    /// match decl.version() {
    ///     Err(Error::XmlDeclWithoutVersion(None)) => {},
    ///     _ => assert!(false),
    /// }
    /// ```
    ///
    /// [grammar]: https://www.w3.org/TR/xml11/#NT-XMLDecl
    pub fn version(&self) -> Result<Cow<[u8]>> {
        // The version *must* be the first thing in the declaration.
        match self.element.attributes().with_checks(false).next() {
            Some(Ok(a)) if a.key == b"version" => Ok(a.value),
            // first attribute was not "version"
            Some(Ok(a)) => {
                let found = from_utf8(a.key).map_err(Error::Utf8)?.to_string();
                Err(Error::XmlDeclWithoutVersion(Some(found)))
            }
            // error parsing attributes
            Some(Err(e)) => Err(e.into()),
            // no attributes
            None => Err(Error::XmlDeclWithoutVersion(None)),
        }
    }

    /// Gets xml encoding, excluding quotes (`'` or `"`).
    ///
    /// Although according to the [grammar] encoding must appear before `"standalone"`
    /// and after `"version"`, this method does not check that. The first occurrence
    /// of the attribute will be returned even if there are several. Also, method does
    /// not restrict symbols that can forming the encoding, so the returned encoding
    /// name may not correspond to the grammar.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::borrow::Cow;
    /// use quick_xml::Error;
    /// use quick_xml::events::{BytesDecl, BytesStart};
    ///
    /// // <?xml version='1.1'?>
    /// let decl = BytesDecl::from_start(BytesStart::borrowed(b" version='1.1'", 0));
    /// assert!(decl.encoding().is_none());
    ///
    /// // <?xml encoding='utf-8'?>
    /// let decl = BytesDecl::from_start(BytesStart::borrowed(b" encoding='utf-8'", 0));
    /// match decl.encoding() {
    ///     Some(Ok(Cow::Borrowed(encoding))) => assert_eq!(encoding, b"utf-8"),
    ///     _ => assert!(false),
    /// }
    ///
    /// // <?xml encoding='something_WRONG' encoding='utf-8'?>
    /// let decl = BytesDecl::from_start(BytesStart::borrowed(b" encoding='something_WRONG' encoding='utf-8'", 0));
    /// match decl.encoding() {
    ///     Some(Ok(Cow::Borrowed(encoding))) => assert_eq!(encoding, b"something_WRONG"),
    ///     _ => assert!(false),
    /// }
    /// ```
    ///
    /// [grammar]: https://www.w3.org/TR/xml11/#NT-XMLDecl
    pub fn encoding(&self) -> Option<Result<Cow<[u8]>>> {
        self.element
            .try_get_attribute("encoding")
            .map(|a| a.map(|a| a.value))
            .transpose()
    }

    /// Gets xml standalone, excluding quotes (`'` or `"`).
    ///
    /// Although according to the [grammar] standalone flag must appear after `"version"`
    /// and `"encoding"`, this method does not check that. The first occurrence of the
    /// attribute will be returned even if there are several. Also, method does not
    /// restrict symbols that can forming the value, so the returned flag name may not
    /// correspond to the grammar.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::borrow::Cow;
    /// use quick_xml::Error;
    /// use quick_xml::events::{BytesDecl, BytesStart};
    ///
    /// // <?xml version='1.1'?>
    /// let decl = BytesDecl::from_start(BytesStart::borrowed(b" version='1.1'", 0));
    /// assert!(decl.standalone().is_none());
    ///
    /// // <?xml standalone='yes'?>
    /// let decl = BytesDecl::from_start(BytesStart::borrowed(b" standalone='yes'", 0));
    /// match decl.standalone() {
    ///     Some(Ok(Cow::Borrowed(encoding))) => assert_eq!(encoding, b"yes"),
    ///     _ => assert!(false),
    /// }
    ///
    /// // <?xml standalone='something_WRONG' encoding='utf-8'?>
    /// let decl = BytesDecl::from_start(BytesStart::borrowed(b" standalone='something_WRONG' encoding='utf-8'", 0));
    /// match decl.standalone() {
    ///     Some(Ok(Cow::Borrowed(flag))) => assert_eq!(flag, b"something_WRONG"),
    ///     _ => assert!(false),
    /// }
    /// ```
    ///
    /// [grammar]: https://www.w3.org/TR/xml11/#NT-XMLDecl
    pub fn standalone(&self) -> Option<Result<Cow<[u8]>>> {
        self.element
            .try_get_attribute("standalone")
            .map(|a| a.map(|a| a.value))
            .transpose()
    }

    /// Constructs a new `XmlDecl` from the (mandatory) _version_ (should be `1.0` or `1.1`),
    /// the optional _encoding_ (e.g., `UTF-8`) and the optional _standalone_ (`yes` or `no`)
    /// attribute.
    ///
    /// Does not escape any of its inputs. Always uses double quotes to wrap the attribute values.
    /// The caller is responsible for escaping attribute values. Shouldn't usually be relevant since
    /// the double quote character is not allowed in any of the attribute values.
    pub fn new(
        version: &[u8],
        encoding: Option<&[u8]>,
        standalone: Option<&[u8]>,
    ) -> BytesDecl<'static> {
        // Compute length of the buffer based on supplied attributes
        // ' encoding=""'   => 12
        let encoding_attr_len = if let Some(xs) = encoding {
            12 + xs.len()
        } else {
            0
        };
        // ' standalone=""' => 14
        let standalone_attr_len = if let Some(xs) = standalone {
            14 + xs.len()
        } else {
            0
        };
        // 'xml version=""' => 14
        let mut buf = Vec::with_capacity(14 + encoding_attr_len + standalone_attr_len);

        buf.extend_from_slice(b"xml version=\"");
        buf.extend_from_slice(version);

        if let Some(encoding_val) = encoding {
            buf.extend_from_slice(b"\" encoding=\"");
            buf.extend_from_slice(encoding_val);
        }

        if let Some(standalone_val) = standalone {
            buf.extend_from_slice(b"\" standalone=\"");
            buf.extend_from_slice(standalone_val);
        }
        buf.push(b'"');

        BytesDecl {
            element: BytesStart::owned(buf, 3),
        }
    }

    /// Gets the decoder struct
    #[cfg(feature = "encoding_rs")]
    pub fn encoder(&self) -> Option<&'static Encoding> {
        self.encoding()
            .and_then(|e| e.ok())
            .and_then(|e| Encoding::for_label(&*e))
    }

    /// Converts the event into an owned event.
    pub fn into_owned(self) -> BytesDecl<'static> {
        BytesDecl {
            element: self.element.into_owned(),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

/// A struct to manage `Event::End` events
#[derive(Clone, Eq, PartialEq)]
pub struct BytesEnd<'a> {
    name: Cow<'a, [u8]>,
}

impl<'a> BytesEnd<'a> {
    /// Creates a new `BytesEnd` borrowing a slice

    pub fn borrowed(name: &'a [u8]) -> BytesEnd<'a> {
        BytesEnd {
            name: Cow::Borrowed(name),
        }
    }

    /// Creates a new `BytesEnd` owning its name

    pub fn owned(name: Vec<u8>) -> BytesEnd<'static> {
        BytesEnd {
            name: Cow::Owned(name),
        }
    }

    /// Converts the event into an owned event.
    pub fn into_owned(self) -> BytesEnd<'static> {
        BytesEnd {
            name: Cow::Owned(self.name.into_owned()),
        }
    }

    /// Gets `BytesEnd` event name

    pub fn name(&self) -> &[u8] {
        &*self.name
    }

    /// local name (excluding namespace) as &[u8] (without eventual attributes)
    /// returns the name() with any leading namespace removed (all content up to
    /// and including the first ':' character)

    pub fn local_name(&self) -> &[u8] {
        if let Some(i) = self.name().iter().position(|b| *b == b':') {
            &self.name()[i + 1..]
        } else {
            self.name()
        }
    }
}

impl<'a> std::fmt::Debug for BytesEnd<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "BytesEnd {{ name: ")?;
        write_cow_string(f, &self.name)?;
        write!(f, " }}")
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

/// Data from various events (most notably, `Event::Text`) that stored in XML
/// in escaped form. Internally data is stored in escaped form
#[derive(Clone, Eq, PartialEq)]
pub struct BytesText<'a> {
    // Invariant: The content is always escaped.
    content: Cow<'a, [u8]>,
}

impl<'a> BytesText<'a> {
    /// Creates a new `BytesText` from an escaped byte sequence.

    pub fn from_escaped<C: Into<Cow<'a, [u8]>>>(content: C) -> Self {
        Self {
            content: content.into(),
        }
    }

    /// Creates a new `BytesText` from a byte sequence. The byte sequence is
    /// expected not to be escaped.

    pub fn from_plain(content: &'a [u8]) -> Self {
        Self {
            content: escape(content),
        }
    }

    /// Creates a new `BytesText` from an escaped string.

    pub fn from_escaped_str<C: Into<Cow<'a, str>>>(content: C) -> Self {
        Self::from_escaped(match content.into() {
            Cow::Owned(o) => Cow::Owned(o.into_bytes()),
            Cow::Borrowed(b) => Cow::Borrowed(b.as_bytes()),
        })
    }

    /// Creates a new `BytesText` from a string. The string is expected not to
    /// be escaped.

    pub fn from_plain_str(content: &'a str) -> Self {
        Self::from_plain(content.as_bytes())
    }

    /// Ensures that all data is owned to extend the object's lifetime if
    /// necessary.

    pub fn into_owned(self) -> BytesText<'static> {
        BytesText {
            content: self.content.into_owned().into(),
        }
    }

    /// Extracts the inner `Cow` from the `BytesText` event container.

    pub fn into_inner(self) -> Cow<'a, [u8]> {
        self.content
    }

    /// Returns unescaped version of the text content, that can be written
    /// as CDATA in XML
    #[cfg(feature = "serialize")]
    pub(crate) fn unescape(self) -> std::result::Result<BytesCData<'a>, EscapeError> {
        //TODO: need to think about better API instead of dozens similar functions
        // Maybe use builder pattern. After that expose function as public API
        //FIXME: need to take into account entities defined in the document
        Ok(BytesCData::new(match do_unescape(&self.content, None)? {
            Cow::Borrowed(_) => self.content,
            Cow::Owned(unescaped) => Cow::Owned(unescaped),
        }))
    }

    /// gets escaped content
    ///
    /// Searches for '&' into content and try to escape the coded character if possible
    /// returns Malformed error with index within element if '&' is not followed by ';'
    ///
    /// See also [`unescaped_with_custom_entities()`](#method.unescaped_with_custom_entities)
    pub fn unescaped(&self) -> Result<Cow<[u8]>> {
        self.make_unescaped(None)
    }

    /// gets escaped content with custom entities
    ///
    /// Searches for '&' into content and try to escape the coded character if possible
    /// returns Malformed error with index within element if '&' is not followed by ';'
    /// Additional entities can be provided in `custom_entities`.
    ///
    /// # Pre-condition
    ///
    /// The keys and values of `custom_entities`, if any, must be valid UTF-8.
    ///
    /// See also [`unescaped()`](#method.unescaped)
    pub fn unescaped_with_custom_entities<'s>(
        &'s self,
        custom_entities: &HashMap<Vec<u8>, Vec<u8>>,
    ) -> Result<Cow<'s, [u8]>> {
        self.make_unescaped(Some(custom_entities))
    }

    fn make_unescaped<'s>(
        &'s self,
        custom_entities: Option<&HashMap<Vec<u8>, Vec<u8>>>,
    ) -> Result<Cow<'s, [u8]>> {
        do_unescape(self, custom_entities).map_err(Error::EscapeError)
    }

    /// helper method to unescape then decode self using the reader encoding
    /// but without BOM (Byte order mark)
    ///
    /// for performance reasons (could avoid allocating a `String`),
    /// it might be wiser to manually use
    /// 1. BytesText::unescaped()
    /// 2. Reader::decode(...)
    #[cfg(feature = "encoding")]
    pub fn unescape_and_decode_without_bom<B: BufRead>(
        &self,
        reader: &mut Reader<B>,
    ) -> Result<String> {
        self.do_unescape_and_decode_without_bom(reader, None)
    }

    /// helper method to unescape then decode self using the reader encoding
    /// but without BOM (Byte order mark)
    ///
    /// for performance reasons (could avoid allocating a `String`),
    /// it might be wiser to manually use
    /// 1. BytesText::unescaped()
    /// 2. Reader::decode(...)
    #[cfg(not(feature = "encoding"))]
    pub fn unescape_and_decode_without_bom<B: BufRead>(
        &self,
        reader: &Reader<B>,
    ) -> Result<String> {
        self.do_unescape_and_decode_without_bom(reader, None)
    }

    /// helper method to unescape then decode self using the reader encoding with custom entities
    /// but without BOM (Byte order mark)
    ///
    /// for performance reasons (could avoid allocating a `String`),
    /// it might be wiser to manually use
    /// 1. BytesText::unescaped()
    /// 2. Reader::decode(...)
    ///
    /// # Pre-condition
    ///
    /// The keys and values of `custom_entities`, if any, must be valid UTF-8.
    #[cfg(feature = "encoding")]
    pub fn unescape_and_decode_without_bom_with_custom_entities<B: BufRead>(
        &self,
        reader: &mut Reader<B>,
        custom_entities: &HashMap<Vec<u8>, Vec<u8>>,
    ) -> Result<String> {
        self.do_unescape_and_decode_without_bom(reader, Some(custom_entities))
    }

    /// helper method to unescape then decode self using the reader encoding with custom entities
    /// but without BOM (Byte order mark)
    ///
    /// for performance reasons (could avoid allocating a `String`),
    /// it might be wiser to manually use
    /// 1. BytesText::unescaped()
    /// 2. Reader::decode(...)
    ///
    /// # Pre-condition
    ///
    /// The keys and values of `custom_entities`, if any, must be valid UTF-8.
    #[cfg(not(feature = "encoding"))]
    pub fn unescape_and_decode_without_bom_with_custom_entities<B: BufRead>(
        &self,
        reader: &Reader<B>,
        custom_entities: &HashMap<Vec<u8>, Vec<u8>>,
    ) -> Result<String> {
        self.do_unescape_and_decode_without_bom(reader, Some(custom_entities))
    }

    #[cfg(feature = "encoding")]
    fn do_unescape_and_decode_without_bom<B: BufRead>(
        &self,
        reader: &mut Reader<B>,
        custom_entities: Option<&HashMap<Vec<u8>, Vec<u8>>>,
    ) -> Result<String> {
        let decoded = reader.decode_without_bom(&*self);
        let unescaped =
            do_unescape(decoded.as_bytes(), custom_entities).map_err(Error::EscapeError)?;
        String::from_utf8(unescaped.into_owned()).map_err(|e| Error::Utf8(e.utf8_error()))
    }

    #[cfg(not(feature = "encoding"))]
    fn do_unescape_and_decode_without_bom<B: BufRead>(
        &self,
        reader: &Reader<B>,
        custom_entities: Option<&HashMap<Vec<u8>, Vec<u8>>>,
    ) -> Result<String> {
        let decoded = reader.decode_without_bom(&*self)?;
        let unescaped =
            do_unescape(decoded.as_bytes(), custom_entities).map_err(Error::EscapeError)?;
        String::from_utf8(unescaped.into_owned()).map_err(|e| Error::Utf8(e.utf8_error()))
    }

    /// helper method to unescape then decode self using the reader encoding
    ///
    /// for performance reasons (could avoid allocating a `String`),
    /// it might be wiser to manually use
    /// 1. BytesText::unescaped()
    /// 2. Reader::decode(...)
    pub fn unescape_and_decode<B: BufRead>(&self, reader: &Reader<B>) -> Result<String> {
        self.do_unescape_and_decode_with_custom_entities(reader, None)
    }

    /// helper method to unescape then decode self using the reader encoding with custom entities
    ///
    /// for performance reasons (could avoid allocating a `String`),
    /// it might be wiser to manually use
    /// 1. BytesText::unescaped()
    /// 2. Reader::decode(...)
    ///
    /// # Pre-condition
    ///
    /// The keys and values of `custom_entities`, if any, must be valid UTF-8.
    pub fn unescape_and_decode_with_custom_entities<B: BufRead>(
        &self,
        reader: &Reader<B>,
        custom_entities: &HashMap<Vec<u8>, Vec<u8>>,
    ) -> Result<String> {
        self.do_unescape_and_decode_with_custom_entities(reader, Some(custom_entities))
    }

    #[cfg(feature = "encoding")]
    fn do_unescape_and_decode_with_custom_entities<B: BufRead>(
        &self,
        reader: &Reader<B>,
        custom_entities: Option<&HashMap<Vec<u8>, Vec<u8>>>,
    ) -> Result<String> {
        let decoded = reader.decode(&*self);
        let unescaped =
            do_unescape(decoded.as_bytes(), custom_entities).map_err(Error::EscapeError)?;
        String::from_utf8(unescaped.into_owned()).map_err(|e| Error::Utf8(e.utf8_error()))
    }

    #[cfg(not(feature = "encoding"))]
    fn do_unescape_and_decode_with_custom_entities<B: BufRead>(
        &self,
        reader: &Reader<B>,
        custom_entities: Option<&HashMap<Vec<u8>, Vec<u8>>>,
    ) -> Result<String> {
        let decoded = reader.decode(&*self)?;
        let unescaped =
            do_unescape(decoded.as_bytes(), custom_entities).map_err(Error::EscapeError)?;
        String::from_utf8(unescaped.into_owned()).map_err(|e| Error::Utf8(e.utf8_error()))
    }

    /// Gets escaped content.
    pub fn escaped(&self) -> &[u8] {
        self.content.as_ref()
    }
}

impl<'a> std::fmt::Debug for BytesText<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "BytesText {{ content: ")?;
        write_cow_string(f, &self.content)?;
        write!(f, " }}")
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

/// CDATA content contains unescaped data from the reader. If you want to write them as a text,
/// [convert](Self::escape) it to [`BytesText`]
#[derive(Clone, Eq, PartialEq)]
pub struct BytesCData<'a> {
    content: Cow<'a, [u8]>,
}

impl<'a> BytesCData<'a> {
    /// Creates a new `BytesCData` from a byte sequence.

    pub fn new<C: Into<Cow<'a, [u8]>>>(content: C) -> Self {
        Self {
            content: content.into(),
        }
    }

    /// Creates a new `BytesCData` from a string

    pub fn from_str(content: &'a str) -> Self {
        Self::new(content.as_bytes())
    }

    /// Ensures that all data is owned to extend the object's lifetime if
    /// necessary.

    pub fn into_owned(self) -> BytesCData<'static> {
        BytesCData {
            content: self.content.into_owned().into(),
        }
    }

    /// Extracts the inner `Cow` from the `BytesCData` event container.

    pub fn into_inner(self) -> Cow<'a, [u8]> {
        self.content
    }

    /// Converts this CDATA content to an escaped version, that can be written
    /// as an usual text in XML.
    ///
    /// This function performs following replacements:
    ///
    /// | Character | Replacement
    /// |-----------|------------
    /// | `<`       | `&lt;`
    /// | `>`       | `&gt;`
    /// | `&`       | `&amp;`
    /// | `'`       | `&apos;`
    /// | `"`       | `&quot;`
    pub fn escape(self) -> BytesText<'a> {
        BytesText::from_escaped(match escape(&self.content) {
            Cow::Borrowed(_) => self.content,
            Cow::Owned(escaped) => Cow::Owned(escaped),
        })
    }

    /// Converts this CDATA content to an escaped version, that can be written
    /// as an usual text in XML.
    ///
    /// In XML text content, it is allowed (though not recommended) to leave
    /// the quote special characters `"` and `'` unescaped.
    ///
    /// This function performs following replacements:
    ///
    /// | Character | Replacement
    /// |-----------|------------
    /// | `<`       | `&lt;`
    /// | `>`       | `&gt;`
    /// | `&`       | `&amp;`
    pub fn partial_escape(self) -> BytesText<'a> {
        BytesText::from_escaped(match partial_escape(&self.content) {
            Cow::Borrowed(_) => self.content,
            Cow::Owned(escaped) => Cow::Owned(escaped),
        })
    }

    /// Gets content of this text buffer in the specified encoding
    #[cfg(feature = "serialize")]
    pub(crate) fn decode(&self, decoder: crate::reader::Decoder) -> Result<Cow<'a, str>> {
        Ok(match &self.content {
            Cow::Borrowed(bytes) => {
                #[cfg(feature = "encoding")]
                {
                    decoder.decode(bytes)
                }
                #[cfg(not(feature = "encoding"))]
                {
                    decoder.decode(bytes)?.into()
                }
            }
            Cow::Owned(bytes) => {
                #[cfg(feature = "encoding")]
                let decoded = decoder.decode(bytes).into_owned();

                #[cfg(not(feature = "encoding"))]
                let decoded = decoder.decode(bytes)?.to_string();

                decoded.into()
            }
        })
    }
}

impl<'a> std::fmt::Debug for BytesCData<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "BytesCData {{ content: ")?;
        write_cow_string(f, &self.content)?;
        write!(f, " }}")
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

/// Event emitted by [`Reader::read_event`].
///
/// [`Reader::read_event`]: ../reader/struct.Reader.html#method.read_event
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Event<'a> {
    /// Start tag (with attributes) `<tag attr="value">`.
    Start(BytesStart<'a>),
    /// End tag `</tag>`.
    End(BytesEnd<'a>),
    /// Empty element tag (with attributes) `<tag attr="value" />`.
    Empty(BytesStart<'a>),
    /// Character data between `Start` and `End` element.
    Text(BytesText<'a>),
    /// Comment `<!-- ... -->`.
    Comment(BytesText<'a>),
    /// CData `<![CDATA[...]]>`.
    CData(BytesCData<'a>),
    /// XML declaration `<?xml ...?>`.
    Decl(BytesDecl<'a>),
    /// Processing instruction `<?...?>`.
    PI(BytesText<'a>),
    /// Doctype `<!DOCTYPE ...>`.
    DocType(BytesText<'a>),
    /// End of XML document.
    Eof,
}

impl<'a> Event<'a> {
    /// Converts the event to an owned version, untied to the lifetime of
    /// buffer used when reading but incurring a new, separate allocation.
    pub fn into_owned(self) -> Event<'static> {
        match self {
            Event::Start(e) => Event::Start(e.into_owned()),
            Event::End(e) => Event::End(e.into_owned()),
            Event::Empty(e) => Event::Empty(e.into_owned()),
            Event::Text(e) => Event::Text(e.into_owned()),
            Event::Comment(e) => Event::Comment(e.into_owned()),
            Event::CData(e) => Event::CData(e.into_owned()),
            Event::Decl(e) => Event::Decl(e.into_owned()),
            Event::PI(e) => Event::PI(e.into_owned()),
            Event::DocType(e) => Event::DocType(e.into_owned()),
            Event::Eof => Event::Eof,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

impl<'a> Deref for BytesStart<'a> {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &*self.buf
    }
}

impl<'a> Deref for BytesDecl<'a> {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &*self.element
    }
}

impl<'a> Deref for BytesEnd<'a> {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &*self.name
    }
}

impl<'a> Deref for BytesText<'a> {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &*self.content
    }
}

impl<'a> Deref for BytesCData<'a> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &*self.content
    }
}

impl<'a> Deref for Event<'a> {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        match *self {
            Event::Start(ref e) | Event::Empty(ref e) => &*e,
            Event::End(ref e) => &*e,
            Event::Text(ref e) => &*e,
            Event::Decl(ref e) => &*e,
            Event::PI(ref e) => &*e,
            Event::CData(ref e) => &*e,
            Event::Comment(ref e) => &*e,
            Event::DocType(ref e) => &*e,
            Event::Eof => &[],
        }
    }
}

impl<'a> AsRef<Event<'a>> for Event<'a> {
    fn as_ref(&self) -> &Event<'a> {
        self
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn local_name() {
        use std::str::from_utf8;
        let xml = r#"
            <foo:bus attr='bar'>foobusbar</foo:bus>
            <foo: attr='bar'>foobusbar</foo:>
            <:foo attr='bar'>foobusbar</:foo>
            <foo:bus:baz attr='bar'>foobusbar</foo:bus:baz>
            "#;
        let mut rdr = Reader::from_str(xml);
        let mut buf = Vec::new();
        let mut parsed_local_names = Vec::new();
        loop {
            match rdr.read_event(&mut buf).expect("unable to read xml event") {
                Event::Start(ref e) => parsed_local_names.push(
                    from_utf8(e.local_name())
                        .expect("unable to build str from local_name")
                        .to_string(),
                ),
                Event::End(ref e) => parsed_local_names.push(
                    from_utf8(e.local_name())
                        .expect("unable to build str from local_name")
                        .to_string(),
                ),
                Event::Eof => break,
                _ => {}
            }
        }
        assert_eq!(parsed_local_names[0], "bus".to_string());
        assert_eq!(parsed_local_names[1], "bus".to_string());
        assert_eq!(parsed_local_names[2], "".to_string());
        assert_eq!(parsed_local_names[3], "".to_string());
        assert_eq!(parsed_local_names[4], "foo".to_string());
        assert_eq!(parsed_local_names[5], "foo".to_string());
        assert_eq!(parsed_local_names[6], "bus:baz".to_string());
        assert_eq!(parsed_local_names[7], "bus:baz".to_string());
    }

    #[test]
    fn bytestart_create() {
        let b = BytesStart::owned_name("test");
        assert_eq!(b.len(), 4);
        assert_eq!(b.name(), b"test");
    }

    #[test]
    fn bytestart_set_name() {
        let mut b = BytesStart::owned_name("test");
        assert_eq!(b.len(), 4);
        assert_eq!(b.name(), b"test");
        assert_eq!(b.attributes_raw(), b"");
        b.push_attribute(("x", "a"));
        assert_eq!(b.len(), 10);
        assert_eq!(b.attributes_raw(), b" x=\"a\"");
        b.set_name(b"g");
        assert_eq!(b.len(), 7);
        assert_eq!(b.name(), b"g");
    }

    #[test]
    fn bytestart_clear_attributes() {
        let mut b = BytesStart::owned_name("test");
        b.push_attribute(("x", "y\"z"));
        b.push_attribute(("x", "y\"z"));
        b.clear_attributes();
        assert!(b.attributes().next().is_none());
        assert_eq!(b.len(), 4);
        assert_eq!(b.name(), b"test");
    }
}
#[cfg(test)]
mod tests_llm_16_39 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_deref() {
        let name: Cow<[u8]> = Cow::Borrowed(b"test");
        let bytes_end = BytesEnd { name };

        let result = bytes_end.deref();

        assert_eq!(result, b"test");
    }
}#[cfg(test)]
mod tests_llm_16_40 {
    use super::*;

use crate::*;
    use std::borrow::Cow;

    #[test]
    fn test_deref() {
        let buf: &[u8] = &[1, 2, 3];
        let start = BytesStart {
            buf: Cow::Borrowed(buf),
            name_len: buf.len(),
        };
        let result: &[u8] = start.deref();
        assert_eq!(result, buf);
    }
}#[cfg(test)]
mod tests_llm_16_43 {
    use super::*;

use crate::*;

    #[test]
    fn test_as_ref() {
        let event = Event::Start(BytesStart::borrowed_name(b"tag"));
        assert_eq!(event.as_ref(), &event);
    }
}#[cfg(test)]
mod tests_llm_16_77 {
    use crate::events::BytesCData;

    #[test]
    fn test_from_str() {
        let content = "Hello, World!";
        let cdata = BytesCData::from_str(content);
        assert_eq!(cdata.into_inner(), content.as_bytes());
    }
}#[cfg(test)]
mod tests_llm_16_78 {
    use crate::events::BytesCData;
    use std::borrow::Cow;

    #[test]
    fn test_into_inner() {
        let content: Cow<[u8]> = Cow::Borrowed(&[1, 2, 3, 4]); // Modify the content as needed
        let cdata = BytesCData { content };

        let result = cdata.into_inner();

        // Add your assertions here
    }
}#[cfg(test)]
mod tests_llm_16_82_llm_16_81 {
    use super::*;

use crate::*;
    use std::borrow::Cow;

    #[test]
    fn test_new() {
        let cdata = BytesCData::new(Cow::Borrowed("content".as_bytes()));
        assert_eq!(cdata.content, "content".as_bytes());
    }
}#[cfg(test)]
mod tests_llm_16_86_llm_16_85 {
    use std::borrow::Cow;
    use crate::Error;
    use crate::events::{BytesDecl, BytesStart};

    #[test]
    fn test_encoding() {
        let decl = BytesDecl::from_start(BytesStart::borrowed(b" version='1.1'", 0));
        assert!(decl.encoding().is_none());

        let decl = BytesDecl::from_start(BytesStart::borrowed(b" encoding='utf-8'", 0));
        match decl.encoding() {
            Some(Ok(Cow::Borrowed(encoding))) => assert_eq!(encoding, b"utf-8"),
            _ => assert!(false),
        }

        let decl = BytesDecl::from_start(BytesStart::borrowed(b" encoding='something_WRONG' encoding='utf-8'", 0));
        match decl.encoding() {
            Some(Ok(Cow::Borrowed(encoding))) => assert_eq!(encoding, b"something_WRONG"),
            _ => assert!(false),
        }
    }
}#[cfg(test)]
mod tests_llm_16_88_llm_16_87 {
    use super::*;

use crate::*;
    use crate::events::{BytesStart, BytesDecl};
    
    #[test]
    fn test_from_start() {
        let start = BytesStart::borrowed(b"<name attr=\"value\">", 0);
        let decl = BytesDecl::from_start(start);
        // Add your assertions here
    }
}#[cfg(test)]
mod tests_llm_16_89 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_into_owned() {
        // create a BytesDecl<'a> instance
        let element = BytesStart::borrowed(b"decl", 0);
        let decl = BytesDecl::from_start(element);
        
        // call into_owned() method
        let owned_decl = decl.into_owned();
        
        // assert the result
        assert_eq!(owned_decl, BytesDecl { element: BytesStart::borrowed(b"decl", 0) });
    }
}#[cfg(test)]
mod tests_llm_16_92 {
    use super::*;

use crate::*;
    use std::borrow::Cow;

    #[test]
    fn test_standalone_no_decl() {
        let decl = BytesDecl::from_start(BytesStart::borrowed(b"", 0));
        assert!(decl.standalone().is_none());
    }

    #[test]
    fn test_standalone_yes() {
        let decl = BytesDecl::from_start(BytesStart::borrowed(b" standalone='yes'", 0));
        match decl.standalone() {
            Some(Ok(Cow::Borrowed(flag))) => assert_eq!(flag, b"yes"),
            _ => assert!(false),
        }
    }

    #[test]
    fn test_standalone_custom() {
        let decl = BytesDecl::from_start(BytesStart::borrowed(b" standalone='something_WRONG' encoding='utf-8'", 0));
        match decl.standalone() {
            Some(Ok(Cow::Borrowed(flag))) => assert_eq!(flag, b"something_WRONG"),
            _ => assert!(false),
        }
    }
}#[cfg(test)]
mod tests_llm_16_94_llm_16_93 {
    use crate::events::*;
    use crate::Error;
    use std::borrow::Cow;

    #[test]
    fn test_version() {
        let decl = BytesDecl::from_start(BytesStart::borrowed(b" version='1.1'", 0));
        assert_eq!(decl.version().unwrap(), Cow::Borrowed(b"1.1".as_ref()));

        let decl = BytesDecl::from_start(BytesStart::borrowed(b" version='1.0' version='1.1'", 0));
        assert_eq!(decl.version().unwrap(), Cow::Borrowed(b"1.0".as_ref()));

        let decl = BytesDecl::from_start(BytesStart::borrowed(b" encoding='utf-8'", 0));
        match decl.version() {
            Err(Error::XmlDeclWithoutVersion(Some(key))) => assert_eq!(key, "encoding".to_string()),
            _ => assert!(false),
        }

        let decl = BytesDecl::from_start(BytesStart::borrowed(b" encoding='utf-8' version='1.1'", 0));
        match decl.version() {
            Err(Error::XmlDeclWithoutVersion(Some(key))) => assert_eq!(key, "encoding".to_string()),
            _ => assert!(false),
        }

        let decl = BytesDecl::from_start(BytesStart::borrowed(b"", 0));
        match decl.version() {
            Err(Error::XmlDeclWithoutVersion(None)) => {},
            _ => assert!(false),
        }
    }
}#[cfg(test)]
mod tests_llm_16_96_llm_16_95 {
    use super::*;

use crate::*;
    use crate::events::BytesEnd;

    #[test]
    fn test_borrowed() {
        let name = b"testing";
        let bytes_end = BytesEnd::borrowed(name);
        assert_eq!(bytes_end.name(), name);
    }
}#[cfg(test)]
mod tests_llm_16_102_llm_16_101 {
    use crate::events::BytesEnd;
  
    #[test]
    fn test_name() {
        let bytes_end = BytesEnd::borrowed(b"example");
        assert_eq!(bytes_end.name(), b"example");
    }
}#[cfg(test)]
mod tests_llm_16_103 {
    use super::*;

use crate::*;
    use std::borrow::Cow;
    
    #[test]
    fn test_owned() {
        let name: Vec<u8> = vec![1, 2, 3, 4];
        let bytes_end: BytesEnd<'static> = BytesEnd::owned(name);
        
        assert_eq!(bytes_end.name(), &[1, 2, 3, 4]);
        assert_eq!(bytes_end.local_name(), &[1, 2, 3, 4]);
    }
}#[cfg(test)]
mod tests_llm_16_108 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_borrowed() {
        let content = b"<tag attr=\"value\">";
        let name_len = 3;
        let start = BytesStart::borrowed(content, name_len);
        
        assert_eq!(start.buf, Cow::Borrowed(content));
        assert_eq!(start.name_len, name_len);
    }
}#[cfg(test)]
mod tests_llm_16_109 {
    use crate::events::BytesStart;
    use std::borrow::Cow;

    #[test]
    fn test_borrowed_name() {
        let name: &[u8] = b"test_name";
        let result = BytesStart::borrowed_name(name);
        let expected = BytesStart {
            buf: Cow::Borrowed(name),
            name_len: name.len(),
        };
        assert_eq!(result.buf, expected.buf);
        assert_eq!(result.name_len, expected.name_len);
    }
}#[cfg(test)]
mod tests_llm_16_158 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_from_escaped() {
        let content: Vec<u8> = vec![65, 66, 67];
        let bytes_text = BytesText::from_escaped(content);
        assert_eq!(&*bytes_text, &[65, 66, 67]);
    }
}#[cfg(test)]
mod tests_llm_16_159 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_from_escaped_str() {
        let text: BytesText = BytesText::from_escaped_str("escaped text");
        
        assert_eq!(text.content, Cow::Borrowed(&[101, 115, 99, 97, 112, 101, 100, 32, 116, 101, 120, 116]));
    }
}#[cfg(test)]
mod tests_llm_16_165 {
    use super::*;

use crate::*;

    #[test]
    fn test_into_inner() {
        let text = BytesText::from_plain(b"Hello, world!");
        let inner = text.into_inner();
        assert_eq!(inner.as_ref(), b"Hello, world!");
    }
}#[cfg(test)]
mod tests_llm_16_170 {
    use super::*;

use crate::*;
    use std::io::BufReader;
    
    #[test]
    fn test_unescape_and_decode() {
        let xml = r#"<tag1 att1 = "test">Test</tag1>"#;
        let reader = Reader::from_str(xml);
        let text = BytesText::from_plain_str("Test");
        let result = text.unescape_and_decode(&reader);
        assert_eq!(result.unwrap(), "Test");
    }
}#[cfg(test)]
mod tests_llm_16_179_llm_16_178 {
    use super::*;

use crate::*;
    use std::collections::HashMap;

    #[test]
    fn test_unescaped_with_custom_entities() {
        let input_text = BytesText::from_plain(b"Hello &amp; World");
        let mut custom_entities = HashMap::new();
        custom_entities.insert(b"amp".to_vec(), b"&".to_vec());

        let result = input_text.unescaped_with_custom_entities(&custom_entities).unwrap();
        let expected = Cow::Borrowed(b"Hello & World");

        assert_eq!(result, expected);
    }
}
#[cfg(test)]
mod tests_rug_78 {
    use super::*;
    use std::borrow::Cow;
    use crate::events::BytesStart;
    
    #[test]
    fn test_rug() {
        let mut p0: Cow<[u8]> = Cow::Borrowed(b"example content");
        let p1: usize = 10;
        
        BytesStart::<'static>::owned(p0, p1);

    }
}
#[cfg(test)]
mod tests_rug_80 {
    use super::*;
    use crate::events::BytesStart;

    #[test]
    fn test_into_owned() {
        let buf = vec![b'<', b'e', b'l', b'e', b'm', b'e', b'n', b't'];
        let name_len = 8;
        let start: BytesStart<'_> = BytesStart::borrowed(&buf[..], name_len);

        let owned_start: BytesStart<'static> = start.into_owned();
        assert_eq!(owned_start.name(), buf);
    }
}#[cfg(test)]
mod tests_rug_81 {
    use super::*;
    use crate::events::BytesStart;

    #[test]
    fn test_to_owned() {
        let mut p0: BytesStart<'static> = BytesStart::owned(Vec::new(), 0);

        BytesStart::to_owned(&p0);
    }
}#[cfg(test)]
mod tests_rug_82 {
    use super::*;
    use crate::events::{BytesStart, Event};
    
    #[test]
    fn test_to_borrowed() {
        let buf: &[u8] = b"<tag attr=\"value\">";
        let name_len: usize = 3;
        let attrs = BytesStart::owned(buf.to_vec(), name_len);

        let borrowed_attrs = attrs.to_borrowed();
        
        // Assert statements
    }
}#[cfg(test)]
mod tests_rug_83 {
    use super::*;
    use crate::events::BytesStart;

    #[test]
    fn test_rug() {
        let mut p0: BytesStart = BytesStart::borrowed_name(b"tag_name");

        p0.to_end();
    }
}        
#[cfg(test)]
mod tests_rug_84 {
    use super::*;
    use crate::events::BytesStart;
    
    #[test]
    fn test_rug() {
        let buf: Vec<u8> = vec![b'<', b'e', b'l', b'e', b'm', b'e', b'n', b't', b'>'];
        let name_len: usize = 9;
        let bytes_start = BytesStart::owned(buf, name_len);
                
        bytes_start.name();
    }
}
        #[cfg(test)]
mod tests_rug_85 {
    use super::*;
    use crate::events::BytesStart;

    #[test]
    fn test_local_name() {
        let name: &[u8] = b"test:tag";
        let p0: BytesStart = BytesStart::borrowed(name, name.len());

        p0.local_name();
    }
}#[cfg(test)]
mod tests_rug_86 {
    use super::*;
    use crate::events::BytesStart;

    #[test]
    fn test_unescaped() {
        let mut p0: BytesStart<'static> = BytesStart::borrowed_name(b"tag_name");
        let result = p0.unescaped();
        assert!(result.is_ok());
    }
}#[cfg(test)]
mod tests_rug_87 {
    use super::*;
    use std::collections::HashMap;
    use crate::events;

    #[test]
    fn test_unescaped_with_custom_entities() {
        let p0: events::BytesStart<'static> = events::BytesStart::borrowed_name(b"tag_name");
        let mut p1: HashMap<Vec<u8>, Vec<u8>> = HashMap::new();
        // Add sample data to the HashMap if necessary

        let result = p0.unescaped_with_custom_entities(&p1);
        
        // Add assertions for the result if needed
        // assert_eq!(result, ...);
    }
}#[cfg(test)]
mod tests_rug_89 {
    use super::*;
    use crate::{events, reader};

    #[test]
    fn test_rug() {
        let mut p0: events::BytesStart<'static> = events::BytesStart::borrowed_name(b"element");
        let mut p1: reader::Reader<&[u8]> = reader::Reader::from_reader(b"<element>test</element>" as &[u8]);

        p0.unescape_and_decode(&p1).unwrap();
    }
}#[cfg(test)]
mod tests_rug_92 {
    use super::*;
    use crate::events::BytesStart;

    #[test]
    fn test_set_name() {
        let mut p0: BytesStart = todo!();  // Fill in the appropriate value
        let p1: &[u8] = todo!();  // Fill in the appropriate value

        p0.set_name(p1);
    }
}        
#[cfg(test)]
mod tests_rug_96 {
    use super::*;
    use crate::events::BytesStart;
    
    #[test]
    fn test_rug() {
        let mut p0: BytesStart<'static> = BytesStart::borrowed(b"tag", 3);
        p0.push_attribute(("attr1", "value1"));
        p0.push_attribute(("attr2", "value2"));
        p0.push_attribute(("attr3", "value3"));
        
        BytesStart::clear_attributes(&mut p0);
        
        // Add assertions here
    }
}#[cfg(test)]
mod tests_rug_97 {
    use super::*;
    use crate::events::BytesStart;
    
    #[test]
    fn test_attributes() {
        let buf: &[u8] = b"<tag attribute1=\"value1\" attribute2=\"value2\"></tag>";
        let name_len: usize = 3;
        let start: BytesStart<'static> = BytesStart::borrowed(buf, name_len);
        
        BytesStart::attributes(&start);
    }
}

#[cfg(test)]
mod tests_rug_98{
    use super::*;
    use crate::events::BytesStart;

    #[test]
    fn test_html_attributes() {
        let name = b"div";
        let p0: BytesStart = BytesStart::borrowed(name, name.len());
        
        <BytesStart<'_>>::html_attributes(&p0);
    }
}
   
   #[cfg(test)]
   mod tests_rug_99 {
       use super::*;
       use crate::events::BytesStart;
       
       #[test]
       fn test_attributes_raw() {
           let buf: &[u8] = b"<tag attr1=\"value1\" attr2=\"value2\">content</tag>";
           let name_len = 5;
           let start = BytesStart::borrowed(buf, name_len);
           
           assert_eq!(start.attributes_raw(), &buf[name_len..]);
       }
   }
   #[cfg(test)]
mod tests_rug_100 {
    use super::*;
    use crate::events::{self, BytesStart};
    
    #[test]
    fn test_try_get_attribute() {
        let mut p0: events::BytesStart<'static> = BytesStart::borrowed_name(b"tag");
        let mut p1: &[u8] = b"attr_name";
        
        p0.try_get_attribute(p1);
    }
}#[cfg(test)]
mod tests_rug_101 {
    use super::*;
    use crate::events::{BytesDecl, BytesStart};
    use std::option::Option;

    #[test]
    fn test_new() {
        let version: &[u8] = b"1.0";
        let encoding: Option<&[u8]> = Some(b"UTF-8");
        let standalone: Option<&[u8]> = Some(b"no");

        BytesDecl::new(version, encoding, standalone);
    }
}#[cfg(test)]
mod tests_rug_102 {
    use super::*;
    use crate::events::BytesEnd;

    #[test]
    fn test_rug() {
        #[cfg(test)]
        mod tests_rug_102_prepare {
            use crate::events::BytesEnd;

            #[test]
            fn sample() {
                let mut v45: BytesEnd = BytesEnd::owned(Vec::new());
            }
        }
        
        let mut p0: BytesEnd<'static> = BytesEnd::owned(Vec::new());
        
        crate::events::BytesEnd::<'static>::into_owned(p0);

    }
}                        
#[cfg(test)]
mod tests_rug_103 {
    use super::*;
    use crate::events::BytesEnd;
    
    #[test]
    fn test_local_name() {
        // sample code to construct the variable
        let mut p0: BytesEnd<'static> = BytesEnd::owned(Vec::new());

        // calling the target function
        p0.local_name();
    }
}#[cfg(test)]
mod tests_rug_104 {
    use super::*;
    use crate::events::BytesText;

    #[test]
    fn test_rug() {
        let p0: &[u8] = b"Hello, World!";
        
        BytesText::<'static>::from_plain(p0);
    }
}#[cfg(test)]
mod tests_rug_105 {
    use super::*;
    use crate::events::BytesText;

    #[test]
    fn test_from_plain_str() {
        let p0: &str = "Sample content";

        BytesText::from_plain_str(p0);
    }
}
#[cfg(test)]
mod tests_rug_106 {
    use super::*;
    use crate::events;
    
    #[test]
    fn test_rug() {
        let mut p0: events::BytesText<'static> = events::BytesText::from_plain_str("Sample text").into();

        <events::BytesText<'static>>::into_owned(p0);
    }
}
#[cfg(test)]
mod tests_rug_107 {
    use super::*;
    use crate::events;
    
    #[test]
    fn test_rug() {
        let mut p0: events::BytesText<'static> = events::BytesText::from_plain_str("Sample text");
        
        p0.unescaped();

    }
}#[cfg(test)]
mod tests_rug_109 {
    use super::*;
    use crate::events;
    use crate::reader::Reader;
    use std::io::Cursor;

    #[test]
    fn test_rug() {
        let p0: events::BytesText<'static> = events::BytesText::from_plain_str("Sample text");
        let p1: Reader<Cursor<&[u8]>> = Reader::from_reader(Cursor::new(b""));
        
        p0.unescape_and_decode_without_bom(&p1);
    }
}#[cfg(test)]
mod tests_rug_110 {
    use super::*;
    use crate::events::{BytesText, Event};
    use crate::reader::Reader;
    use std::collections::HashMap;
    
    #[test]
    fn test_rug() {
        let mut p0: BytesText<'static> = BytesText::from_plain_str("Sample text");
        let mut p1: Reader<&[u8]> = Reader::from_str("<xml></xml>");
        let mut p2: HashMap<Vec<u8>, Vec<u8>> = HashMap::new();
    
        p0.unescape_and_decode_without_bom_with_custom_entities(&p1, &p2);
    }
}#[cfg(test)]
mod tests_rug_112 {
    use super::*;
    use crate::events::{BytesText, Event};
    use crate::Reader;
    use std::collections::HashMap;
    use std::io::BufReader;

    #[test]
    fn test_rug() {
        let mut p0: BytesText<'static> = BytesText::from_plain_str("Sample text");
        let p1: Reader<BufReader<&[u8]>> = Reader::from_reader(BufReader::new(b"<root></root>"));
        let mut p2: HashMap<Vec<u8>, Vec<u8>> = HashMap::new();

        BytesText::unescape_and_decode_with_custom_entities(&p0, &p1, &p2).unwrap();
    }
}#[cfg(test)]
mod tests_rug_113 {
    use super::*;
    use crate::events;
    use crate::reader::Reader;
    use crate::escape::do_unescape;
    use crate::Error;
    use std::collections::HashMap;
    use std::io::BufRead;

    #[test]
    fn test_rug() {
        let mut v46: events::BytesText<'static> = events::BytesText::from_plain_str("Sample text");
        let mut v83: Option<&HashMap<Vec<u8>, Vec<u8>>> = Some(&HashMap::new());

        let reader = Reader::from_str("");
        let custom_entities: Option<&HashMap<Vec<u8>, Vec<u8>>> = None;

        v46.do_unescape_and_decode_with_custom_entities(&reader, Some(&HashMap::<Vec<u8>, Vec<u8>>::new()));
    }
}#[cfg(test)]
mod tests_rug_114 {
    use super::*;
    use crate::events;
    
    #[test]
    fn test_rug() {
        let mut v46: events::BytesText<'static> = events::BytesText::from_plain_str("Sample text");
        
        <events::BytesText<'static>>::escaped(&v46);
    }
}#[cfg(test)]
mod tests_rug_115 {
    use super::*;
    use crate::events::BytesCData;

    #[test]
    fn test_rug() {
        let mut p0: BytesCData<'static> = BytesCData {
            content: Cow::Borrowed(b"example content"),
        };

        BytesCData::<'static>::into_owned(p0);
    }
}#[cfg(test)]
    mod tests_rug_117 {
        use super::*;
        use crate::events::{BytesCData, BytesText};

        #[test]
        fn test_partial_escape() {
            let p0: BytesCData = todo!("Construct the BytesCData instance");

            let result: BytesText = p0.partial_escape();

            // Add assertions for the result
        }
    }#[cfg(test)]
mod tests_rug_120 {
    use super::*;
    use crate::events;

    #[test]
    fn test_rug() {
        let mut p0: events::BytesText<'static> = events::BytesText::from_plain_str("Sample text");
        <events::BytesText<'static> as std::ops::Deref>::deref(&p0);
    }
}