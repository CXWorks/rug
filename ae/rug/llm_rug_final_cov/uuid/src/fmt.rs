//! Adapters for alternative string formats.
use crate::{
    std::{borrow::Borrow, fmt, ptr, str},
    Uuid, Variant,
};
impl std::fmt::Debug for Uuid {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(self, f)
    }
}
impl fmt::Display for Uuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(self, f)
    }
}
impl fmt::Display for Variant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Variant::NCS => write!(f, "NCS"),
            Variant::RFC4122 => write!(f, "RFC4122"),
            Variant::Microsoft => write!(f, "Microsoft"),
            Variant::Future => write!(f, "Future"),
        }
    }
}
impl fmt::LowerHex for Uuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(self.as_hyphenated(), f)
    }
}
impl fmt::UpperHex for Uuid {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::UpperHex::fmt(self.as_hyphenated(), f)
    }
}
/// Format a [`Uuid`] as a hyphenated string, like
/// `67e55044-10b1-426f-9247-bb680e5fe0c8`.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Hyphenated(Uuid);
/// Format a [`Uuid`] as a simple string, like
/// `67e5504410b1426f9247bb680e5fe0c8`.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Simple(Uuid);
/// Format a [`Uuid`] as a URN string, like
/// `urn:uuid:67e55044-10b1-426f-9247-bb680e5fe0c8`.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Urn(Uuid);
/// Format a [`Uuid`] as a braced hyphenated string, like
/// `{67e55044-10b1-426f-9247-bb680e5fe0c8}`.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Braced(Uuid);
impl Uuid {
    /// Get a [`Hyphenated`] formatter.
    #[inline]
    pub const fn hyphenated(self) -> Hyphenated {
        Hyphenated(self)
    }
    /// Get a borrowed [`Hyphenated`] formatter.
    #[inline]
    pub fn as_hyphenated(&self) -> &Hyphenated {
        unsafe { &*(self as *const Uuid as *const Hyphenated) }
    }
    /// Get a [`Simple`] formatter.
    #[inline]
    pub const fn simple(self) -> Simple {
        Simple(self)
    }
    /// Get a borrowed [`Simple`] formatter.
    #[inline]
    pub fn as_simple(&self) -> &Simple {
        unsafe { &*(self as *const Uuid as *const Simple) }
    }
    /// Get a [`Urn`] formatter.
    #[inline]
    pub const fn urn(self) -> Urn {
        Urn(self)
    }
    /// Get a borrowed [`Urn`] formatter.
    #[inline]
    pub fn as_urn(&self) -> &Urn {
        unsafe { &*(self as *const Uuid as *const Urn) }
    }
    /// Get a [`Braced`] formatter.
    #[inline]
    pub const fn braced(self) -> Braced {
        Braced(self)
    }
    /// Get a borrowed [`Braced`] formatter.
    #[inline]
    pub fn as_braced(&self) -> &Braced {
        unsafe { &*(self as *const Uuid as *const Braced) }
    }
}
const UPPER: [u8; 16] = [
    b'0',
    b'1',
    b'2',
    b'3',
    b'4',
    b'5',
    b'6',
    b'7',
    b'8',
    b'9',
    b'A',
    b'B',
    b'C',
    b'D',
    b'E',
    b'F',
];
const LOWER: [u8; 16] = [
    b'0',
    b'1',
    b'2',
    b'3',
    b'4',
    b'5',
    b'6',
    b'7',
    b'8',
    b'9',
    b'a',
    b'b',
    b'c',
    b'd',
    b'e',
    b'f',
];
#[inline]
const fn format_simple(src: &[u8; 16], upper: bool) -> [u8; 32] {
    let lut = if upper { &UPPER } else { &LOWER };
    let mut dst = [0; 32];
    let mut i = 0;
    while i < 16 {
        let x = src[i];
        dst[i * 2] = lut[(x >> 4) as usize];
        dst[i * 2 + 1] = lut[(x & 0x0f) as usize];
        i += 1;
    }
    dst
}
#[inline]
const fn format_hyphenated(src: &[u8; 16], upper: bool) -> [u8; 36] {
    let lut = if upper { &UPPER } else { &LOWER };
    let groups = [(0, 8), (9, 13), (14, 18), (19, 23), (24, 36)];
    let mut dst = [0; 36];
    let mut group_idx = 0;
    let mut i = 0;
    while group_idx < 5 {
        let (start, end) = groups[group_idx];
        let mut j = start;
        while j < end {
            let x = src[i];
            i += 1;
            dst[j] = lut[(x >> 4) as usize];
            dst[j + 1] = lut[(x & 0x0f) as usize];
            j += 2;
        }
        if group_idx < 4 {
            dst[end] = b'-';
        }
        group_idx += 1;
    }
    dst
}
#[inline]
fn encode_simple<'b>(src: &[u8; 16], buffer: &'b mut [u8], upper: bool) -> &'b mut str {
    let buf = &mut buffer[..Simple::LENGTH];
    let dst = buf.as_mut_ptr();
    unsafe {
        ptr::write(dst.cast(), format_simple(src, upper));
        str::from_utf8_unchecked_mut(buf)
    }
}
#[inline]
fn encode_hyphenated<'b>(
    src: &[u8; 16],
    buffer: &'b mut [u8],
    upper: bool,
) -> &'b mut str {
    let buf = &mut buffer[..Hyphenated::LENGTH];
    let dst = buf.as_mut_ptr();
    unsafe {
        ptr::write(dst.cast(), format_hyphenated(src, upper));
        str::from_utf8_unchecked_mut(buf)
    }
}
#[inline]
fn encode_braced<'b>(src: &[u8; 16], buffer: &'b mut [u8], upper: bool) -> &'b mut str {
    let buf = &mut buffer[..Braced::LENGTH];
    buf[0] = b'{';
    buf[Braced::LENGTH - 1] = b'}';
    unsafe {
        let dst = buf.as_mut_ptr().add(1);
        ptr::write(dst.cast(), format_hyphenated(src, upper));
        str::from_utf8_unchecked_mut(buf)
    }
}
#[inline]
fn encode_urn<'b>(src: &[u8; 16], buffer: &'b mut [u8], upper: bool) -> &'b mut str {
    let buf = &mut buffer[..Urn::LENGTH];
    buf[..9].copy_from_slice(b"urn:uuid:");
    unsafe {
        let dst = buf.as_mut_ptr().add(9);
        ptr::write(dst.cast(), format_hyphenated(src, upper));
        str::from_utf8_unchecked_mut(buf)
    }
}
impl Hyphenated {
    /// The length of a hyphenated [`Uuid`] string.
    ///
    /// [`Uuid`]: ../struct.Uuid.html
    pub const LENGTH: usize = 36;
    /// Creates a [`Hyphenated`] from a [`Uuid`].
    ///
    /// [`Uuid`]: ../struct.Uuid.html
    /// [`Hyphenated`]: struct.Hyphenated.html
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Hyphenated(uuid)
    }
    /// Writes the [`Uuid`] as a lower-case hyphenated string to
    /// `buffer`, and returns the subslice of the buffer that contains the
    /// encoded UUID.
    ///
    /// This is slightly more efficient than using the formatting
    /// infrastructure as it avoids virtual calls, and may avoid
    /// double buffering.
    ///
    /// [`Uuid`]: ../struct.Uuid.html
    ///
    /// # Panics
    ///
    /// Panics if the buffer is not large enough: it must have length at least
    /// [`LENGTH`]. [`Uuid::encode_buffer`] can be used to get a
    /// sufficiently-large temporary buffer.
    ///
    /// [`LENGTH`]: #associatedconstant.LENGTH
    /// [`Uuid::encode_buffer`]: ../struct.Uuid.html#method.encode_buffer
    ///
    /// # Examples
    ///
    /// ```rust
    /// use uuid::Uuid;
    ///
    /// fn main() -> Result<(), uuid::Error> {
    ///     let uuid = Uuid::parse_str("936DA01f9abd4d9d80c702af85c822a8")?;
    ///
    ///     // the encoded portion is returned
    ///     assert_eq!(
    ///         uuid.hyphenated()
    ///             .encode_lower(&mut Uuid::encode_buffer()),
    ///         "936da01f-9abd-4d9d-80c7-02af85c822a8"
    ///     );
    ///
    ///     // the buffer is mutated directly, and trailing contents remains
    ///     let mut buf = [b'!'; 40];
    ///     uuid.hyphenated().encode_lower(&mut buf);
    ///     assert_eq!(
    ///         &buf as &[_],
    ///         b"936da01f-9abd-4d9d-80c7-02af85c822a8!!!!" as &[_]
    ///     );
    ///
    ///     Ok(())
    /// }
    /// ```
    /// */
    #[inline]
    pub fn encode_lower<'buf>(&self, buffer: &'buf mut [u8]) -> &'buf mut str {
        encode_hyphenated(self.0.as_bytes(), buffer, false)
    }
    /// Writes the [`Uuid`] as an upper-case hyphenated string to
    /// `buffer`, and returns the subslice of the buffer that contains the
    /// encoded UUID.
    ///
    /// This is slightly more efficient than using the formatting
    /// infrastructure as it avoids virtual calls, and may avoid
    /// double buffering.
    ///
    /// [`Uuid`]: ../struct.Uuid.html
    ///
    /// # Panics
    ///
    /// Panics if the buffer is not large enough: it must have length at least
    /// [`LENGTH`]. [`Uuid::encode_buffer`] can be used to get a
    /// sufficiently-large temporary buffer.
    ///
    /// [`LENGTH`]: #associatedconstant.LENGTH
    /// [`Uuid::encode_buffer`]: ../struct.Uuid.html#method.encode_buffer
    ///
    /// # Examples
    ///
    /// ```rust
    /// use uuid::Uuid;
    ///
    /// fn main() -> Result<(), uuid::Error> {
    ///     let uuid = Uuid::parse_str("936da01f9abd4d9d80c702af85c822a8")?;
    ///
    ///     // the encoded portion is returned
    ///     assert_eq!(
    ///         uuid.hyphenated()
    ///             .encode_upper(&mut Uuid::encode_buffer()),
    ///         "936DA01F-9ABD-4D9D-80C7-02AF85C822A8"
    ///     );
    ///
    ///     // the buffer is mutated directly, and trailing contents remains
    ///     let mut buf = [b'!'; 40];
    ///     uuid.hyphenated().encode_upper(&mut buf);
    ///     assert_eq!(
    ///         &buf as &[_],
    ///         b"936DA01F-9ABD-4D9D-80C7-02AF85C822A8!!!!" as &[_]
    ///     );
    ///
    ///     Ok(())
    /// }
    /// ```
    /// */
    #[inline]
    pub fn encode_upper<'buf>(&self, buffer: &'buf mut [u8]) -> &'buf mut str {
        encode_hyphenated(self.0.as_bytes(), buffer, true)
    }
    /// Get a reference to the underlying [`Uuid`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use uuid::Uuid;
    ///
    /// let hyphenated = Uuid::nil().hyphenated();
    /// assert_eq!(*hyphenated.as_uuid(), Uuid::nil());
    /// ```
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }
    /// Consumes the [`Hyphenated`], returning the underlying [`Uuid`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use uuid::Uuid;
    ///
    /// let hyphenated = Uuid::nil().hyphenated();
    /// assert_eq!(hyphenated.into_uuid(), Uuid::nil());
    /// ```
    pub const fn into_uuid(self) -> Uuid {
        self.0
    }
}
impl Braced {
    /// The length of a braced [`Uuid`] string.
    ///
    /// [`Uuid`]: ../struct.Uuid.html
    pub const LENGTH: usize = 38;
    /// Creates a [`Braced`] from a [`Uuid`].
    ///
    /// [`Uuid`]: ../struct.Uuid.html
    /// [`Braced`]: struct.Braced.html
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Braced(uuid)
    }
    /// Writes the [`Uuid`] as a lower-case hyphenated string surrounded by
    /// braces to `buffer`, and returns the subslice of the buffer that contains
    /// the encoded UUID.
    ///
    /// This is slightly more efficient than using the formatting
    /// infrastructure as it avoids virtual calls, and may avoid
    /// double buffering.
    ///
    /// [`Uuid`]: ../struct.Uuid.html
    ///
    /// # Panics
    ///
    /// Panics if the buffer is not large enough: it must have length at least
    /// [`LENGTH`]. [`Uuid::encode_buffer`] can be used to get a
    /// sufficiently-large temporary buffer.
    ///
    /// [`LENGTH`]: #associatedconstant.LENGTH
    /// [`Uuid::encode_buffer`]: ../struct.Uuid.html#method.encode_buffer
    ///
    /// # Examples
    ///
    /// ```rust
    /// use uuid::Uuid;
    ///
    /// fn main() -> Result<(), uuid::Error> {
    ///     let uuid = Uuid::parse_str("936DA01f9abd4d9d80c702af85c822a8")?;
    ///
    ///     // the encoded portion is returned
    ///     assert_eq!(
    ///         uuid.braced()
    ///             .encode_lower(&mut Uuid::encode_buffer()),
    ///         "{936da01f-9abd-4d9d-80c7-02af85c822a8}"
    ///     );
    ///
    ///     // the buffer is mutated directly, and trailing contents remains
    ///     let mut buf = [b'!'; 40];
    ///     uuid.braced().encode_lower(&mut buf);
    ///     assert_eq!(
    ///         &buf as &[_],
    ///         b"{936da01f-9abd-4d9d-80c7-02af85c822a8}!!" as &[_]
    ///     );
    ///
    ///     Ok(())
    /// }
    /// ```
    /// */
    #[inline]
    pub fn encode_lower<'buf>(&self, buffer: &'buf mut [u8]) -> &'buf mut str {
        encode_braced(self.0.as_bytes(), buffer, false)
    }
    /// Writes the [`Uuid`] as an upper-case hyphenated string surrounded by
    /// braces to `buffer`, and returns the subslice of the buffer that contains
    /// the encoded UUID.
    ///
    /// This is slightly more efficient than using the formatting
    /// infrastructure as it avoids virtual calls, and may avoid
    /// double buffering.
    ///
    /// [`Uuid`]: ../struct.Uuid.html
    ///
    /// # Panics
    ///
    /// Panics if the buffer is not large enough: it must have length at least
    /// [`LENGTH`]. [`Uuid::encode_buffer`] can be used to get a
    /// sufficiently-large temporary buffer.
    ///
    /// [`LENGTH`]: #associatedconstant.LENGTH
    /// [`Uuid::encode_buffer`]: ../struct.Uuid.html#method.encode_buffer
    ///
    /// # Examples
    ///
    /// ```rust
    /// use uuid::Uuid;
    ///
    /// fn main() -> Result<(), uuid::Error> {
    ///     let uuid = Uuid::parse_str("936da01f9abd4d9d80c702af85c822a8")?;
    ///
    ///     // the encoded portion is returned
    ///     assert_eq!(
    ///         uuid.braced()
    ///             .encode_upper(&mut Uuid::encode_buffer()),
    ///         "{936DA01F-9ABD-4D9D-80C7-02AF85C822A8}"
    ///     );
    ///
    ///     // the buffer is mutated directly, and trailing contents remains
    ///     let mut buf = [b'!'; 40];
    ///     uuid.braced().encode_upper(&mut buf);
    ///     assert_eq!(
    ///         &buf as &[_],
    ///         b"{936DA01F-9ABD-4D9D-80C7-02AF85C822A8}!!" as &[_]
    ///     );
    ///
    ///     Ok(())
    /// }
    /// ```
    /// */
    #[inline]
    pub fn encode_upper<'buf>(&self, buffer: &'buf mut [u8]) -> &'buf mut str {
        encode_braced(self.0.as_bytes(), buffer, true)
    }
    /// Get a reference to the underlying [`Uuid`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use uuid::Uuid;
    ///
    /// let braced = Uuid::nil().braced();
    /// assert_eq!(*braced.as_uuid(), Uuid::nil());
    /// ```
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }
    /// Consumes the [`Braced`], returning the underlying [`Uuid`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use uuid::Uuid;
    ///
    /// let braced = Uuid::nil().braced();
    /// assert_eq!(braced.into_uuid(), Uuid::nil());
    /// ```
    pub const fn into_uuid(self) -> Uuid {
        self.0
    }
}
impl Simple {
    /// The length of a simple [`Uuid`] string.
    ///
    /// [`Uuid`]: ../struct.Uuid.html
    pub const LENGTH: usize = 32;
    /// Creates a [`Simple`] from a [`Uuid`].
    ///
    /// [`Uuid`]: ../struct.Uuid.html
    /// [`Simple`]: struct.Simple.html
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Simple(uuid)
    }
    /// Writes the [`Uuid`] as a lower-case simple string to `buffer`,
    /// and returns the subslice of the buffer that contains the encoded UUID.
    ///
    /// This is slightly more efficient than using the formatting
    /// infrastructure as it avoids virtual calls, and may avoid
    /// double buffering.
    ///
    /// [`Uuid`]: ../struct.Uuid.html
    ///
    /// # Panics
    ///
    /// Panics if the buffer is not large enough: it must have length at least
    /// [`LENGTH`]. [`Uuid::encode_buffer`] can be used to get a
    /// sufficiently-large temporary buffer.
    ///
    /// [`LENGTH`]: #associatedconstant.LENGTH
    /// [`Uuid::encode_buffer`]: ../struct.Uuid.html#method.encode_buffer
    ///
    /// # Examples
    ///
    /// ```rust
    /// use uuid::Uuid;
    ///
    /// fn main() -> Result<(), uuid::Error> {
    ///     let uuid = Uuid::parse_str("936DA01f9abd4d9d80c702af85c822a8")?;
    ///
    ///     // the encoded portion is returned
    ///     assert_eq!(
    ///         uuid.simple().encode_lower(&mut Uuid::encode_buffer()),
    ///         "936da01f9abd4d9d80c702af85c822a8"
    ///     );
    ///
    ///     // the buffer is mutated directly, and trailing contents remains
    ///     let mut buf = [b'!'; 36];
    ///     assert_eq!(
    ///         uuid.simple().encode_lower(&mut buf),
    ///         "936da01f9abd4d9d80c702af85c822a8"
    ///     );
    ///     assert_eq!(
    ///         &buf as &[_],
    ///         b"936da01f9abd4d9d80c702af85c822a8!!!!" as &[_]
    ///     );
    ///
    ///     Ok(())
    /// }
    /// ```
    /// */
    #[inline]
    pub fn encode_lower<'buf>(&self, buffer: &'buf mut [u8]) -> &'buf mut str {
        encode_simple(self.0.as_bytes(), buffer, false)
    }
    /// Writes the [`Uuid`] as an upper-case simple string to `buffer`,
    /// and returns the subslice of the buffer that contains the encoded UUID.
    ///
    /// [`Uuid`]: ../struct.Uuid.html
    ///
    /// # Panics
    ///
    /// Panics if the buffer is not large enough: it must have length at least
    /// [`LENGTH`]. [`Uuid::encode_buffer`] can be used to get a
    /// sufficiently-large temporary buffer.
    ///
    /// [`LENGTH`]: #associatedconstant.LENGTH
    /// [`Uuid::encode_buffer`]: ../struct.Uuid.html#method.encode_buffer
    ///
    /// # Examples
    ///
    /// ```rust
    /// use uuid::Uuid;
    ///
    /// fn main() -> Result<(), uuid::Error> {
    ///     let uuid = Uuid::parse_str("936da01f9abd4d9d80c702af85c822a8")?;
    ///
    ///     // the encoded portion is returned
    ///     assert_eq!(
    ///         uuid.simple().encode_upper(&mut Uuid::encode_buffer()),
    ///         "936DA01F9ABD4D9D80C702AF85C822A8"
    ///     );
    ///
    ///     // the buffer is mutated directly, and trailing contents remains
    ///     let mut buf = [b'!'; 36];
    ///     assert_eq!(
    ///         uuid.simple().encode_upper(&mut buf),
    ///         "936DA01F9ABD4D9D80C702AF85C822A8"
    ///     );
    ///     assert_eq!(
    ///         &buf as &[_],
    ///         b"936DA01F9ABD4D9D80C702AF85C822A8!!!!" as &[_]
    ///     );
    ///
    ///     Ok(())
    /// }
    /// ```
    /// */
    #[inline]
    pub fn encode_upper<'buf>(&self, buffer: &'buf mut [u8]) -> &'buf mut str {
        encode_simple(self.0.as_bytes(), buffer, true)
    }
    /// Get a reference to the underlying [`Uuid`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use uuid::Uuid;
    ///
    /// let simple = Uuid::nil().simple();
    /// assert_eq!(*simple.as_uuid(), Uuid::nil());
    /// ```
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }
    /// Consumes the [`Simple`], returning the underlying [`Uuid`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use uuid::Uuid;
    ///
    /// let simple = Uuid::nil().simple();
    /// assert_eq!(simple.into_uuid(), Uuid::nil());
    /// ```
    pub const fn into_uuid(self) -> Uuid {
        self.0
    }
}
impl Urn {
    /// The length of a URN [`Uuid`] string.
    ///
    /// [`Uuid`]: ../struct.Uuid.html
    pub const LENGTH: usize = 45;
    /// Creates a [`Urn`] from a [`Uuid`].
    ///
    /// [`Uuid`]: ../struct.Uuid.html
    /// [`Urn`]: struct.Urn.html
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Urn(uuid)
    }
    /// Writes the [`Uuid`] as a lower-case URN string to
    /// `buffer`, and returns the subslice of the buffer that contains the
    /// encoded UUID.
    ///
    /// This is slightly more efficient than using the formatting
    /// infrastructure as it avoids virtual calls, and may avoid
    /// double buffering.
    ///
    /// [`Uuid`]: ../struct.Uuid.html
    ///
    /// # Panics
    ///
    /// Panics if the buffer is not large enough: it must have length at least
    /// [`LENGTH`]. [`Uuid::encode_buffer`] can be used to get a
    /// sufficiently-large temporary buffer.
    ///
    /// [`LENGTH`]: #associatedconstant.LENGTH
    /// [`Uuid::encode_buffer`]: ../struct.Uuid.html#method.encode_buffer
    ///
    /// # Examples
    ///
    /// ```rust
    /// use uuid::Uuid;
    ///
    /// fn main() -> Result<(), uuid::Error> {
    ///     let uuid = Uuid::parse_str("936DA01f9abd4d9d80c702af85c822a8")?;
    ///
    ///     // the encoded portion is returned
    ///     assert_eq!(
    ///         uuid.urn().encode_lower(&mut Uuid::encode_buffer()),
    ///         "urn:uuid:936da01f-9abd-4d9d-80c7-02af85c822a8"
    ///     );
    ///
    ///     // the buffer is mutated directly, and trailing contents remains
    ///     let mut buf = [b'!'; 49];
    ///     uuid.urn().encode_lower(&mut buf);
    ///     assert_eq!(
    ///         uuid.urn().encode_lower(&mut buf),
    ///         "urn:uuid:936da01f-9abd-4d9d-80c7-02af85c822a8"
    ///     );
    ///     assert_eq!(
    ///         &buf as &[_],
    ///         b"urn:uuid:936da01f-9abd-4d9d-80c7-02af85c822a8!!!!" as &[_]
    ///     );
    ///
    ///     Ok(())
    /// }
    /// ```
    /// */
    #[inline]
    pub fn encode_lower<'buf>(&self, buffer: &'buf mut [u8]) -> &'buf mut str {
        encode_urn(self.0.as_bytes(), buffer, false)
    }
    /// Writes the [`Uuid`] as an upper-case URN string to
    /// `buffer`, and returns the subslice of the buffer that contains the
    /// encoded UUID.
    ///
    /// This is slightly more efficient than using the formatting
    /// infrastructure as it avoids virtual calls, and may avoid
    /// double buffering.
    ///
    /// [`Uuid`]: ../struct.Uuid.html
    ///
    /// # Panics
    ///
    /// Panics if the buffer is not large enough: it must have length at least
    /// [`LENGTH`]. [`Uuid::encode_buffer`] can be used to get a
    /// sufficiently-large temporary buffer.
    ///
    /// [`LENGTH`]: #associatedconstant.LENGTH
    /// [`Uuid::encode_buffer`]: ../struct.Uuid.html#method.encode_buffer
    ///
    /// # Examples
    ///
    /// ```rust
    /// use uuid::Uuid;
    ///
    /// fn main() -> Result<(), uuid::Error> {
    ///     let uuid = Uuid::parse_str("936da01f9abd4d9d80c702af85c822a8")?;
    ///
    ///     // the encoded portion is returned
    ///     assert_eq!(
    ///         uuid.urn().encode_upper(&mut Uuid::encode_buffer()),
    ///         "urn:uuid:936DA01F-9ABD-4D9D-80C7-02AF85C822A8"
    ///     );
    ///
    ///     // the buffer is mutated directly, and trailing contents remains
    ///     let mut buf = [b'!'; 49];
    ///     assert_eq!(
    ///         uuid.urn().encode_upper(&mut buf),
    ///         "urn:uuid:936DA01F-9ABD-4D9D-80C7-02AF85C822A8"
    ///     );
    ///     assert_eq!(
    ///         &buf as &[_],
    ///         b"urn:uuid:936DA01F-9ABD-4D9D-80C7-02AF85C822A8!!!!" as &[_]
    ///     );
    ///
    ///     Ok(())
    /// }
    /// ```
    /// */
    #[inline]
    pub fn encode_upper<'buf>(&self, buffer: &'buf mut [u8]) -> &'buf mut str {
        encode_urn(self.0.as_bytes(), buffer, true)
    }
    /// Get a reference to the underlying [`Uuid`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use uuid::Uuid;
    ///
    /// let urn = Uuid::nil().urn();
    /// assert_eq!(*urn.as_uuid(), Uuid::nil());
    /// ```
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }
    /// Consumes the [`Urn`], returning the underlying [`Uuid`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use uuid::Uuid;
    ///
    /// let urn = Uuid::nil().urn();
    /// assert_eq!(urn.into_uuid(), Uuid::nil());
    /// ```
    pub const fn into_uuid(self) -> Uuid {
        self.0
    }
}
macro_rules! impl_fmt_traits {
    ($($T:ident <$($a:lifetime),*>),+) => {
        $(impl <$($a),*> fmt::Display for $T <$($a),*> { #[inline] fn fmt(& self, f : &
        mut fmt::Formatter <'_ >) -> fmt::Result { fmt::LowerHex::fmt(self, f) } } impl
        <$($a),*> fmt::LowerHex for $T <$($a),*> { fn fmt(& self, f : & mut
        fmt::Formatter <'_ >) -> fmt::Result { f.write_str(self.encode_lower(& mut [0;
        Self::LENGTH])) } } impl <$($a),*> fmt::UpperHex for $T <$($a),*> { fn fmt(&
        self, f : & mut fmt::Formatter <'_ >) -> fmt::Result { f.write_str(self
        .encode_upper(& mut [0; Self::LENGTH])) } } impl_fmt_from!($T <$($a),*>);)+
    };
}
macro_rules! impl_fmt_from {
    ($T:ident <>) => {
        impl From < Uuid > for $T { #[inline] fn from(f : Uuid) -> Self { $T (f) } } impl
        From <$T > for Uuid { #[inline] fn from(f : $T) -> Self { f.into_uuid() } } impl
        AsRef < Uuid > for $T { #[inline] fn as_ref(& self) -> & Uuid { & self.0 } } impl
        Borrow < Uuid > for $T { #[inline] fn borrow(& self) -> & Uuid { & self.0 } }
    };
    ($T:ident <$a:lifetime >) => {
        impl <$a > From <&$a Uuid > for $T <$a > { #[inline] fn from(f : &$a Uuid) ->
        Self { $T ::from_uuid_ref(f) } } impl <$a > From <$T <$a >> for &$a Uuid {
        #[inline] fn from(f : $T <$a >) -> &$a Uuid { f.0 } } impl <$a > AsRef < Uuid >
        for $T <$a > { #[inline] fn as_ref(& self) -> & Uuid { self.0 } } impl <$a >
        Borrow < Uuid > for $T <$a > { #[inline] fn borrow(& self) -> & Uuid { self.0 } }
    };
}
impl_fmt_traits! {
    Hyphenated <>, Simple <>, Urn <>, Braced <>
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hyphenated_trailing() {
        let mut buf = [b'x'; 100];
        let len = Uuid::nil().hyphenated().encode_lower(&mut buf).len();
        assert_eq!(len, super::Hyphenated::LENGTH);
        assert!(buf[len..].iter().all(| x | * x == b'x'));
    }
    #[test]
    fn hyphenated_ref_trailing() {
        let mut buf = [b'x'; 100];
        let len = Uuid::nil().as_hyphenated().encode_lower(&mut buf).len();
        assert_eq!(len, super::Hyphenated::LENGTH);
        assert!(buf[len..].iter().all(| x | * x == b'x'));
    }
    #[test]
    fn simple_trailing() {
        let mut buf = [b'x'; 100];
        let len = Uuid::nil().simple().encode_lower(&mut buf).len();
        assert_eq!(len, super::Simple::LENGTH);
        assert!(buf[len..].iter().all(| x | * x == b'x'));
    }
    #[test]
    fn simple_ref_trailing() {
        let mut buf = [b'x'; 100];
        let len = Uuid::nil().as_simple().encode_lower(&mut buf).len();
        assert_eq!(len, super::Simple::LENGTH);
        assert!(buf[len..].iter().all(| x | * x == b'x'));
    }
    #[test]
    fn urn_trailing() {
        let mut buf = [b'x'; 100];
        let len = Uuid::nil().urn().encode_lower(&mut buf).len();
        assert_eq!(len, super::Urn::LENGTH);
        assert!(buf[len..].iter().all(| x | * x == b'x'));
    }
    #[test]
    fn urn_ref_trailing() {
        let mut buf = [b'x'; 100];
        let len = Uuid::nil().as_urn().encode_lower(&mut buf).len();
        assert_eq!(len, super::Urn::LENGTH);
        assert!(buf[len..].iter().all(| x | * x == b'x'));
    }
    #[test]
    fn braced_trailing() {
        let mut buf = [b'x'; 100];
        let len = Uuid::nil().braced().encode_lower(&mut buf).len();
        assert_eq!(len, super::Braced::LENGTH);
        assert!(buf[len..].iter().all(| x | * x == b'x'));
    }
    #[test]
    fn braced_ref_trailing() {
        let mut buf = [b'x'; 100];
        let len = Uuid::nil().as_braced().encode_lower(&mut buf).len();
        assert_eq!(len, super::Braced::LENGTH);
        assert!(buf[len..].iter().all(| x | * x == b'x'));
    }
    #[test]
    #[should_panic]
    fn hyphenated_too_small() {
        Uuid::nil().hyphenated().encode_lower(&mut [0; 35]);
    }
    #[test]
    #[should_panic]
    fn simple_too_small() {
        Uuid::nil().simple().encode_lower(&mut [0; 31]);
    }
    #[test]
    #[should_panic]
    fn urn_too_small() {
        Uuid::nil().urn().encode_lower(&mut [0; 44]);
    }
    #[test]
    #[should_panic]
    fn braced_too_small() {
        Uuid::nil().braced().encode_lower(&mut [0; 37]);
    }
    #[test]
    fn hyphenated_to_inner() {
        let hyphenated = Uuid::nil().hyphenated();
        assert_eq!(Uuid::from(hyphenated), Uuid::nil());
    }
    #[test]
    fn simple_to_inner() {
        let simple = Uuid::nil().simple();
        assert_eq!(Uuid::from(simple), Uuid::nil());
    }
    #[test]
    fn urn_to_inner() {
        let urn = Uuid::nil().urn();
        assert_eq!(Uuid::from(urn), Uuid::nil());
    }
    #[test]
    fn braced_to_inner() {
        let braced = Uuid::nil().braced();
        assert_eq!(Uuid::from(braced), Uuid::nil());
    }
}
#[cfg(test)]
mod tests_rug_9 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_9_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0xABu8;
        let rug_fuzz_1 = 0xCDu8;
        let rug_fuzz_2 = 0xEFu8;
        let rug_fuzz_3 = 0x01u8;
        let rug_fuzz_4 = 0x23u8;
        let rug_fuzz_5 = 0x45u8;
        let rug_fuzz_6 = 0x67u8;
        let rug_fuzz_7 = 0x89u8;
        let rug_fuzz_8 = 0xABu8;
        let rug_fuzz_9 = 0xCDu8;
        let rug_fuzz_10 = 0xEFu8;
        let rug_fuzz_11 = 0x01u8;
        let rug_fuzz_12 = 0x23u8;
        let rug_fuzz_13 = 0x45u8;
        let rug_fuzz_14 = 0x67u8;
        let rug_fuzz_15 = 0x89u8;
        let rug_fuzz_16 = true;
        let mut p0 = &[
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
            rug_fuzz_6,
            rug_fuzz_7,
            rug_fuzz_8,
            rug_fuzz_9,
            rug_fuzz_10,
            rug_fuzz_11,
            rug_fuzz_12,
            rug_fuzz_13,
            rug_fuzz_14,
            rug_fuzz_15,
        ];
        let mut p1 = rug_fuzz_16;
        crate::fmt::format_simple(p0, p1);
        let _rug_ed_tests_rug_9_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_10 {
    use super::*;
    #[test]
    fn test_format_hyphenated() {
        let _rug_st_tests_rug_10_rrrruuuugggg_test_format_hyphenated = 0;
        let rug_fuzz_0 = 0x4a;
        let rug_fuzz_1 = 0x35;
        let rug_fuzz_2 = 0x6c;
        let rug_fuzz_3 = 0x87;
        let rug_fuzz_4 = 0xad;
        let rug_fuzz_5 = 0xec;
        let rug_fuzz_6 = 0xc6;
        let rug_fuzz_7 = 0xf5;
        let rug_fuzz_8 = 0xbb;
        let rug_fuzz_9 = 0x5c;
        let rug_fuzz_10 = 0x77;
        let rug_fuzz_11 = 0xb5;
        let rug_fuzz_12 = 0xf7;
        let rug_fuzz_13 = 0xa4;
        let rug_fuzz_14 = 0xc1;
        let rug_fuzz_15 = 0xbf;
        let rug_fuzz_16 = true;
        let p0: [u8; 16] = [
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
            rug_fuzz_6,
            rug_fuzz_7,
            rug_fuzz_8,
            rug_fuzz_9,
            rug_fuzz_10,
            rug_fuzz_11,
            rug_fuzz_12,
            rug_fuzz_13,
            rug_fuzz_14,
            rug_fuzz_15,
        ];
        let p1: bool = rug_fuzz_16;
        format_hyphenated(&p0, p1);
        let _rug_ed_tests_rug_10_rrrruuuugggg_test_format_hyphenated = 0;
    }
}
#[cfg(test)]
mod tests_rug_11 {
    use super::*;
    use crate::fmt::{format_simple, Simple};
    use std::{ptr, str};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_11_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = false;
        let mut p0: [u8; 16] = [rug_fuzz_0; 16];
        let mut p1: [u8; Simple::LENGTH] = [rug_fuzz_1; Simple::LENGTH];
        let mut p2: bool = rug_fuzz_2;
        crate::fmt::encode_simple(&mut p0, &mut p1, p2);
        let _rug_ed_tests_rug_11_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_12 {
    use super::*;
    use crate::fmt::{encode_hyphenated, Hyphenated};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_12_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = false;
        let mut p0: [u8; 16] = [rug_fuzz_0; 16];
        let mut p1: [u8; Hyphenated::LENGTH] = [rug_fuzz_1; Hyphenated::LENGTH];
        let p2: bool = rug_fuzz_2;
        encode_hyphenated(&p0, &mut p1, p2);
        let _rug_ed_tests_rug_12_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_13 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_13_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0u8;
        let rug_fuzz_1 = 0u8;
        let rug_fuzz_2 = false;
        let mut p0: [u8; 16] = [rug_fuzz_0; 16];
        let mut p1: [u8; 32] = [rug_fuzz_1; 32];
        let mut p2: bool = rug_fuzz_2;
        crate::fmt::encode_braced(&mut p0, &mut p1, p2);
        let _rug_ed_tests_rug_13_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_14 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_14_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0x6e;
        let rug_fuzz_1 = 0x5b;
        let rug_fuzz_2 = 0xce;
        let rug_fuzz_3 = 0xd1;
        let rug_fuzz_4 = 0x38;
        let rug_fuzz_5 = 0x1b;
        let rug_fuzz_6 = 0x40;
        let rug_fuzz_7 = 0x72;
        let rug_fuzz_8 = 0xa1;
        let rug_fuzz_9 = 0x3f;
        let rug_fuzz_10 = 0xa7;
        let rug_fuzz_11 = 0x23;
        let rug_fuzz_12 = 0xfa;
        let rug_fuzz_13 = 0x66;
        let rug_fuzz_14 = 0xc3;
        let rug_fuzz_15 = 0xb1;
        let rug_fuzz_16 = b' ';
        let rug_fuzz_17 = true;
        let mut p0: [u8; 16] = [
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
            rug_fuzz_6,
            rug_fuzz_7,
            rug_fuzz_8,
            rug_fuzz_9,
            rug_fuzz_10,
            rug_fuzz_11,
            rug_fuzz_12,
            rug_fuzz_13,
            rug_fuzz_14,
            rug_fuzz_15,
        ];
        let mut p1: [u8; Urn::LENGTH] = [rug_fuzz_16; Urn::LENGTH];
        let mut p2: bool = rug_fuzz_17;
        crate::fmt::encode_urn(&mut p0, &mut p1[..], p2);
        let _rug_ed_tests_rug_14_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_15 {
    use super::*;
    use crate::fmt::Hyphenated;
    use crate::Uuid;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_15_rrrruuuugggg_test_rug = 0;
        let mut p0: Uuid = Uuid::nil();
        <Uuid>::hyphenated(p0);
        let _rug_ed_tests_rug_15_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_16 {
    use super::*;
    use crate::Uuid;
    #[test]
    fn test_as_hyphenated() {
        let _rug_st_tests_rug_16_rrrruuuugggg_test_as_hyphenated = 0;
        let mut p0: Uuid;
        p0 = Uuid::nil();
        p0.as_hyphenated();
        let _rug_ed_tests_rug_16_rrrruuuugggg_test_as_hyphenated = 0;
    }
}
#[cfg(test)]
mod tests_rug_17 {
    use super::*;
    use super::super::Uuid;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_17_rrrruuuugggg_test_rug = 0;
        let mut p0: Uuid;
        p0 = Uuid::nil();
        <Uuid>::simple(p0);
        let _rug_ed_tests_rug_17_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_18 {
    use super::*;
    #[cfg(test)]
    mod tests_rug_18_prepare {
        use super::*;
        #[test]
        fn sample() {
            let _rug_st_tests_rug_18_prepare_rrrruuuugggg_sample = 0;
            let rug_fuzz_0 = 0;
            let rug_fuzz_1 = 0;
            let _rug_st_tests_rug_18_rrrruuuugggg_sample = rug_fuzz_0;
            let mut v1: Uuid;
            v1 = Uuid::nil();
            let _rug_ed_tests_rug_18_rrrruuuugggg_sample = rug_fuzz_1;
            let _rug_ed_tests_rug_18_prepare_rrrruuuugggg_sample = 0;
        }
    }
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_18_rrrruuuugggg_test_rug = 0;
        let mut p0: Uuid = Uuid::nil();
        <Uuid>::as_simple(&p0);
        let _rug_ed_tests_rug_18_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_19 {
    use super::*;
    use crate::Uuid;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_19_rrrruuuugggg_test_rug = 0;
        let mut p0: Uuid = Uuid::nil();
        <Uuid>::urn(p0);
        let _rug_ed_tests_rug_19_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_20 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_20_rrrruuuugggg_test_rug = 0;
        let mut p0: Uuid;
        p0 = Uuid::nil();
        <Uuid>::as_urn(&p0);
        let _rug_ed_tests_rug_20_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_21 {
    use super::*;
    use crate::fmt::{Braced, Uuid};
    #[test]
    fn test_braced() {
        let _rug_st_tests_rug_21_rrrruuuugggg_test_braced = 0;
        let mut p0: Uuid = Uuid::nil();
        <Uuid>::braced(p0);
        let _rug_ed_tests_rug_21_rrrruuuugggg_test_braced = 0;
    }
}
#[cfg(test)]
mod tests_rug_22 {
    use super::*;
    use crate::fmt::{Braced, Uuid};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_22_rrrruuuugggg_test_rug = 0;
        let mut p0: Uuid = Uuid::nil();
        p0.as_braced();
        let _rug_ed_tests_rug_22_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_23 {
    use super::*;
    use crate::Uuid;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_23_rrrruuuugggg_test_rug = 0;
        let mut p0: Uuid;
        p0 = Uuid::nil();
        crate::fmt::Hyphenated::from_uuid(p0);
        let _rug_ed_tests_rug_23_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_24 {
    use super::*;
    use crate::Uuid;
    use crate::fmt::Hyphenated;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_24_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let mut p0: Hyphenated = Hyphenated::from_uuid(Uuid::nil());
        let mut p1: [u8; 50] = [rug_fuzz_0; 50];
        crate::fmt::Hyphenated::encode_lower(&mut p0, &mut p1);
        let _rug_ed_tests_rug_24_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_25 {
    use super::*;
    use crate::Uuid;
    use crate::fmt::Hyphenated;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_25_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let mut p0: Hyphenated = Hyphenated::from_uuid(Uuid::nil());
        let mut p1: [u8; 40] = [rug_fuzz_0; 40];
        crate::fmt::Hyphenated::encode_upper(&mut p0, &mut p1);
        let _rug_ed_tests_rug_25_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_26 {
    use super::*;
    use crate::Uuid;
    use crate::fmt::Hyphenated;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_26_rrrruuuugggg_test_rug = 0;
        let mut p0: Hyphenated = Hyphenated::from_uuid(Uuid::nil());
        crate::fmt::Hyphenated::as_uuid(&p0);
        let _rug_ed_tests_rug_26_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_27 {
    use super::*;
    use crate::fmt::Hyphenated;
    use crate::Uuid;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_27_rrrruuuugggg_test_rug = 0;
        let mut p0: Hyphenated = Hyphenated::from_uuid(Uuid::nil());
        p0.into_uuid();
        let _rug_ed_tests_rug_27_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_29 {
    use super::*;
    use crate::Uuid;
    use crate::fmt::Braced;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_29_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let mut p0: Braced = Braced::from_uuid(Uuid::nil());
        let mut p1: [u8; 16] = [rug_fuzz_0; 16];
        crate::fmt::Braced::encode_lower(&mut p0, &mut p1);
        let _rug_ed_tests_rug_29_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_30 {
    use super::*;
    use crate::Uuid;
    use crate::fmt::Braced;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_30_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let mut p0: Braced = Braced::from_uuid(Uuid::nil());
        let mut p1: [u8; 40] = [rug_fuzz_0; 40];
        crate::fmt::Braced::encode_upper(&mut p0, &mut p1);
        let _rug_ed_tests_rug_30_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_31 {
    use super::*;
    use crate::Uuid;
    use crate::fmt::Braced;
    #[test]
    fn test_braced_as_uuid() {
        let _rug_st_tests_rug_31_rrrruuuugggg_test_braced_as_uuid = 0;
        let p0: Braced = Braced::from_uuid(Uuid::nil());
        crate::fmt::Braced::as_uuid(&p0);
        let _rug_ed_tests_rug_31_rrrruuuugggg_test_braced_as_uuid = 0;
    }
}
#[cfg(test)]
mod tests_rug_32 {
    use super::*;
    use crate::Uuid;
    use crate::fmt::Braced;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_32_rrrruuuugggg_test_rug = 0;
        let mut p0: Braced = Braced::from_uuid(Uuid::nil());
        crate::fmt::Braced::into_uuid(p0);
        let _rug_ed_tests_rug_32_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_33 {
    use super::*;
    use crate::fmt::Simple;
    use crate::Uuid;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_33_rrrruuuugggg_test_rug = 0;
        let mut p0: Uuid = Uuid::nil();
        Simple::from_uuid(p0);
        let _rug_ed_tests_rug_33_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_34 {
    use super::*;
    use crate::Uuid;
    use crate::fmt::{Simple, encode_simple};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_34_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b'!';
        let rug_fuzz_1 = false;
        let mut p0: Simple = Simple::from_uuid(Uuid::nil());
        let mut p1: [u8; 36] = [rug_fuzz_0; 36];
        encode_simple(p0.0.as_bytes(), &mut p1, rug_fuzz_1);
        let _rug_ed_tests_rug_34_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_35 {
    use super::*;
    use crate::Uuid;
    use crate::fmt;
    #[test]
    fn test_encode_upper() {
        let _rug_st_tests_rug_35_rrrruuuugggg_test_encode_upper = 0;
        let rug_fuzz_0 = 0;
        let mut p0: fmt::Simple = fmt::Simple::from_uuid(Uuid::nil());
        let mut p1: [u8; 16] = [rug_fuzz_0; 16];
        p0.encode_upper(&mut p1);
        let _rug_ed_tests_rug_35_rrrruuuugggg_test_encode_upper = 0;
    }
}
#[cfg(test)]
mod tests_rug_36 {
    use super::*;
    use crate::Uuid;
    use crate::fmt::Simple;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_36_rrrruuuugggg_test_rug = 0;
        let mut p0: Simple = Simple::from_uuid(Uuid::nil());
        crate::fmt::Simple::as_uuid(&p0);
        let _rug_ed_tests_rug_36_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_37 {
    use super::*;
    use crate::Uuid;
    use crate::fmt::Simple;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_37_rrrruuuugggg_test_rug = 0;
        let mut p0: Simple = Simple::from_uuid(Uuid::nil());
        crate::fmt::Simple::into_uuid(p0);
        let _rug_ed_tests_rug_37_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_38 {
    use super::*;
    use crate::fmt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_38_rrrruuuugggg_test_rug = 0;
        let mut p0: Uuid;
        p0 = Uuid::nil();
        fmt::Urn::from_uuid(p0);
        let _rug_ed_tests_rug_38_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_39 {
    use super::*;
    use crate::Uuid;
    use crate::fmt::Urn;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_39_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let mut p0: Urn = Urn::from_uuid(Uuid::nil());
        let mut p1: [u8; 16] = [rug_fuzz_0; 16];
        crate::fmt::Urn::encode_lower(&mut p0, &mut p1);
        let _rug_ed_tests_rug_39_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_40 {
    use super::*;
    use crate::Uuid;
    use crate::fmt::Urn;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_40_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b'!';
        let mut p0: Urn = Urn::from_uuid(Uuid::nil());
        let mut p1: [u8; 50] = [rug_fuzz_0; 50];
        <Urn>::encode_upper(&mut p0, &mut p1);
        let _rug_ed_tests_rug_40_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_41 {
    use super::*;
    use crate::Uuid;
    use crate::fmt::Urn;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_41_rrrruuuugggg_test_rug = 0;
        let mut p0: Urn = Urn::from_uuid(Uuid::nil());
        crate::fmt::Urn::as_uuid(&p0);
        let _rug_ed_tests_rug_41_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_42 {
    use super::*;
    use crate::Uuid;
    use crate::fmt::Urn;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_42_rrrruuuugggg_test_rug = 0;
        let mut p0: Urn = Urn::from_uuid(Uuid::nil());
        crate::fmt::Urn::into_uuid(p0);
        let _rug_ed_tests_rug_42_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_44 {
    use super::*;
    use crate::std::convert::From;
    use crate::Uuid;
    use crate::fmt::Hyphenated;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_44_rrrruuuugggg_test_rug = 0;
        let mut p0: Hyphenated = Hyphenated::from_uuid(Uuid::nil());
        <Uuid>::from(p0);
        let _rug_ed_tests_rug_44_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_45 {
    use super::*;
    use crate::std::convert::AsRef;
    use crate::Uuid;
    use crate::fmt::Hyphenated;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_45_rrrruuuugggg_test_rug = 0;
        let mut p0: Hyphenated = Hyphenated::from_uuid(Uuid::nil());
        p0.as_ref();
        let _rug_ed_tests_rug_45_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_48 {
    use super::*;
    use crate::std::convert::From;
    use crate::Uuid;
    use crate::fmt::Simple;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_48_rrrruuuugggg_test_rug = 0;
        let mut p0: Simple = Simple::from_uuid(Uuid::nil());
        <Uuid>::from(p0);
        let _rug_ed_tests_rug_48_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_49 {
    use super::*;
    use crate::std::convert::AsRef;
    use crate::Uuid;
    use crate::fmt::Simple;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_49_rrrruuuugggg_test_rug = 0;
        let mut p0: Simple = Simple::from_uuid(Uuid::nil());
        p0.as_ref();
        let _rug_ed_tests_rug_49_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_50 {
    use super::*;
    use crate::std::borrow::Borrow;
    use crate::Uuid;
    use crate::fmt::Simple;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_50_rrrruuuugggg_test_rug = 0;
        let mut v4: Simple = Simple::from_uuid(Uuid::nil());
        let p0: &dyn std::borrow::Borrow<Uuid> = &v4;
        p0.borrow();
        let _rug_ed_tests_rug_50_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_52 {
    use super::*;
    use crate::std::convert::From;
    use crate::fmt::Urn;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_52_rrrruuuugggg_test_rug = 0;
        let mut p0: Urn = Urn::from_uuid(Uuid::nil());
        <Uuid>::from(p0);
        let _rug_ed_tests_rug_52_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_53 {
    use super::*;
    use crate::std::convert::AsRef;
    use crate::fmt::Urn;
    use crate::Uuid;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_53_rrrruuuugggg_test_rug = 0;
        let mut p0: Urn = Urn::from_uuid(Uuid::nil());
        <Urn as AsRef<Uuid>>::as_ref(&p0);
        let _rug_ed_tests_rug_53_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_56 {
    use super::*;
    use crate::std::convert::From;
    use crate::Uuid;
    use crate::fmt::Braced;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_56_rrrruuuugggg_test_rug = 0;
        let mut p0: Braced = Braced::from_uuid(Uuid::nil());
        <Uuid>::from(p0);
        let _rug_ed_tests_rug_56_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_57 {
    use super::*;
    use crate::std::convert::AsRef;
    use crate::Uuid;
    use crate::fmt::Braced;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_57_rrrruuuugggg_sample = 0;
        #[cfg(test)]
        mod tests_rug_57_prepare {
            use crate::Uuid;
            use crate::fmt::Braced;
            #[test]
            fn sample() {
                let _rug_st_tests_rug_57_prepare_rrrruuuugggg_sample = 0;
                let rug_fuzz_0 = 0;
                let rug_fuzz_1 = 0;
                let _rug_st_tests_rug_57_rrrruuuugggg_sample = rug_fuzz_0;
                let mut v3: Braced = Braced::from_uuid(Uuid::nil());
                let _rug_ed_tests_rug_57_rrrruuuugggg_sample = rug_fuzz_1;
                let _rug_ed_tests_rug_57_prepare_rrrruuuugggg_sample = 0;
            }
        }
        let mut p0: Braced = Braced::from_uuid(Uuid::nil());
        p0.as_ref();
        let _rug_ed_tests_rug_57_rrrruuuugggg_sample = 0;
    }
}
