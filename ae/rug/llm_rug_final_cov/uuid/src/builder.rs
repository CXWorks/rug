//! A Builder type for [`Uuid`]s.
//!
//! [`Uuid`]: ../struct.Uuid.html
use crate::{error::*, timestamp, Bytes, Uuid, Variant, Version};
/// A builder for creating a UUID.
///
/// This type is useful if you need to mutate individual fields of a [`Uuid`]
/// while constructing it. Since the [`Uuid`] type is `Copy`, it doesn't offer
/// any methods to mutate in place. They live on the `Builder` instead.
///
/// The `Builder` type also always exposes APIs to construct [`Uuid`]s for any
/// version without needing crate features or additional dependencies. It's a
/// lower-level API than the methods on [`Uuid`].
///
/// # Examples
///
/// Creating a version 4 UUID from externally generated random bytes:
///
/// ```
/// # use uuid::{Builder, Version, Variant};
/// # let rng = || [
/// #     70, 235, 208, 238, 14, 109, 67, 201, 185, 13, 204, 195, 90,
/// # 145, 63, 62,
/// # ];
/// let random_bytes = rng();
///
/// let uuid = Builder::from_random_bytes(random_bytes).into_uuid();
///
/// assert_eq!(Some(Version::Random), uuid.get_version());
/// assert_eq!(Variant::RFC4122, uuid.get_variant());
/// ```
#[allow(missing_copy_implementations)]
#[derive(Debug)]
pub struct Builder(Uuid);
impl Uuid {
    /// The 'nil UUID' (all zeros).
    ///
    /// The nil UUID is a special form of UUID that is specified to have all
    /// 128 bits set to zero.
    ///
    /// # References
    ///
    /// * [Nil UUID in RFC4122](https://tools.ietf.org/html/rfc4122.html#section-4.1.7)
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # use uuid::Uuid;
    /// let uuid = Uuid::nil();
    ///
    /// assert_eq!(
    ///     "00000000-0000-0000-0000-000000000000",
    ///     uuid.hyphenated().to_string(),
    /// );
    /// ```
    pub const fn nil() -> Self {
        Uuid::from_bytes([0; 16])
    }
    /// The 'max UUID' (all ones).
    ///
    /// The max UUID is a special form of UUID that is specified to have all
    /// 128 bits set to one.
    ///
    /// # References
    ///
    /// * [Max UUID in Draft RFC: New UUID Formats, Version 4](https://datatracker.ietf.org/doc/html/draft-peabody-dispatch-new-uuid-format-04#section-5.4)
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # use uuid::Uuid;
    /// let uuid = Uuid::max();
    ///
    /// assert_eq!(
    ///     "ffffffff-ffff-ffff-ffff-ffffffffffff",
    ///     uuid.hyphenated().to_string(),
    /// );
    /// ```
    #[cfg(uuid_unstable)]
    pub const fn max() -> Self {
        Uuid::from_bytes([0xFF; 16])
    }
    /// Creates a UUID from four field values.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # use uuid::Uuid;
    /// let d1 = 0xa1a2a3a4;
    /// let d2 = 0xb1b2;
    /// let d3 = 0xc1c2;
    /// let d4 = [0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8];
    ///
    /// let uuid = Uuid::from_fields(d1, d2, d3, &d4);
    ///
    /// assert_eq!(
    ///     "a1a2a3a4-b1b2-c1c2-d1d2-d3d4d5d6d7d8",
    ///     uuid.hyphenated().to_string(),
    /// );
    /// ```
    pub const fn from_fields(d1: u32, d2: u16, d3: u16, d4: &[u8; 8]) -> Uuid {
        Uuid::from_bytes([
            (d1 >> 24) as u8,
            (d1 >> 16) as u8,
            (d1 >> 8) as u8,
            d1 as u8,
            (d2 >> 8) as u8,
            d2 as u8,
            (d3 >> 8) as u8,
            d3 as u8,
            d4[0],
            d4[1],
            d4[2],
            d4[3],
            d4[4],
            d4[5],
            d4[6],
            d4[7],
        ])
    }
    /// Creates a UUID from four field values in little-endian order.
    ///
    /// The bytes in the `d1`, `d2` and `d3` fields will be flipped to convert
    /// into big-endian order. This is based on the endianness of the UUID,
    /// rather than the target environment so bytes will be flipped on both
    /// big and little endian machines.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # use uuid::Uuid;
    /// let d1 = 0xa1a2a3a4;
    /// let d2 = 0xb1b2;
    /// let d3 = 0xc1c2;
    /// let d4 = [0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8];
    ///
    /// let uuid = Uuid::from_fields_le(d1, d2, d3, &d4);
    ///
    /// assert_eq!(
    ///     "a4a3a2a1-b2b1-c2c1-d1d2-d3d4d5d6d7d8",
    ///     uuid.hyphenated().to_string(),
    /// );
    /// ```
    pub const fn from_fields_le(d1: u32, d2: u16, d3: u16, d4: &[u8; 8]) -> Uuid {
        Uuid::from_bytes([
            d1 as u8,
            (d1 >> 8) as u8,
            (d1 >> 16) as u8,
            (d1 >> 24) as u8,
            (d2) as u8,
            (d2 >> 8) as u8,
            d3 as u8,
            (d3 >> 8) as u8,
            d4[0],
            d4[1],
            d4[2],
            d4[3],
            d4[4],
            d4[5],
            d4[6],
            d4[7],
        ])
    }
    /// Creates a UUID from a 128bit value.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # use uuid::Uuid;
    /// let v = 0xa1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d8u128;
    ///
    /// let uuid = Uuid::from_u128(v);
    ///
    /// assert_eq!(
    ///     "a1a2a3a4-b1b2-c1c2-d1d2-d3d4d5d6d7d8",
    ///     uuid.hyphenated().to_string(),
    /// );
    /// ```
    pub const fn from_u128(v: u128) -> Self {
        Uuid::from_bytes([
            (v >> 120) as u8,
            (v >> 112) as u8,
            (v >> 104) as u8,
            (v >> 96) as u8,
            (v >> 88) as u8,
            (v >> 80) as u8,
            (v >> 72) as u8,
            (v >> 64) as u8,
            (v >> 56) as u8,
            (v >> 48) as u8,
            (v >> 40) as u8,
            (v >> 32) as u8,
            (v >> 24) as u8,
            (v >> 16) as u8,
            (v >> 8) as u8,
            v as u8,
        ])
    }
    /// Creates a UUID from a 128bit value in little-endian order.
    ///
    /// The entire value will be flipped to convert into big-endian order.
    /// This is based on the endianness of the UUID, rather than the target
    /// environment so bytes will be flipped on both big and little endian
    /// machines.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # use uuid::Uuid;
    /// let v = 0xa1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d8u128;
    ///
    /// let uuid = Uuid::from_u128_le(v);
    ///
    /// assert_eq!(
    ///     "d8d7d6d5-d4d3-d2d1-c2c1-b2b1a4a3a2a1",
    ///     uuid.hyphenated().to_string(),
    /// );
    /// ```
    pub const fn from_u128_le(v: u128) -> Self {
        Uuid::from_bytes([
            v as u8,
            (v >> 8) as u8,
            (v >> 16) as u8,
            (v >> 24) as u8,
            (v >> 32) as u8,
            (v >> 40) as u8,
            (v >> 48) as u8,
            (v >> 56) as u8,
            (v >> 64) as u8,
            (v >> 72) as u8,
            (v >> 80) as u8,
            (v >> 88) as u8,
            (v >> 96) as u8,
            (v >> 104) as u8,
            (v >> 112) as u8,
            (v >> 120) as u8,
        ])
    }
    /// Creates a UUID from two 64bit values.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # use uuid::Uuid;
    /// let hi = 0xa1a2a3a4b1b2c1c2u64;
    /// let lo = 0xd1d2d3d4d5d6d7d8u64;
    ///
    /// let uuid = Uuid::from_u64_pair(hi, lo);
    ///
    /// assert_eq!(
    ///     "a1a2a3a4-b1b2-c1c2-d1d2-d3d4d5d6d7d8",
    ///     uuid.hyphenated().to_string(),
    /// );
    /// ```
    pub const fn from_u64_pair(high_bits: u64, low_bits: u64) -> Self {
        Uuid::from_bytes([
            (high_bits >> 56) as u8,
            (high_bits >> 48) as u8,
            (high_bits >> 40) as u8,
            (high_bits >> 32) as u8,
            (high_bits >> 24) as u8,
            (high_bits >> 16) as u8,
            (high_bits >> 8) as u8,
            high_bits as u8,
            (low_bits >> 56) as u8,
            (low_bits >> 48) as u8,
            (low_bits >> 40) as u8,
            (low_bits >> 32) as u8,
            (low_bits >> 24) as u8,
            (low_bits >> 16) as u8,
            (low_bits >> 8) as u8,
            low_bits as u8,
        ])
    }
    /// Creates a UUID using the supplied bytes.
    ///
    /// # Errors
    ///
    /// This function will return an error if `b` has any length other than 16.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # fn main() -> Result<(), uuid::Error> {
    /// # use uuid::Uuid;
    /// let bytes = [
    ///     0xa1, 0xa2, 0xa3, 0xa4,
    ///     0xb1, 0xb2,
    ///     0xc1, 0xc2,
    ///     0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8,
    /// ];
    ///
    /// let uuid = Uuid::from_slice(&bytes)?;
    ///
    /// assert_eq!(
    ///     "a1a2a3a4-b1b2-c1c2-d1d2-d3d4d5d6d7d8",
    ///     uuid.hyphenated().to_string(),
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_slice(b: &[u8]) -> Result<Uuid, Error> {
        if b.len() != 16 {
            return Err(
                Error(ErrorKind::ByteLength {
                    len: b.len(),
                }),
            );
        }
        let mut bytes: Bytes = [0; 16];
        bytes.copy_from_slice(b);
        Ok(Uuid::from_bytes(bytes))
    }
    /// Creates a UUID using the supplied bytes in little endian order.
    ///
    /// The individual fields encoded in the buffer will be flipped.
    ///
    /// # Errors
    ///
    /// This function will return an error if `b` has any length other than 16.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # fn main() -> Result<(), uuid::Error> {
    /// # use uuid::Uuid;
    /// let bytes = [
    ///     0xa1, 0xa2, 0xa3, 0xa4,
    ///     0xb1, 0xb2,
    ///     0xc1, 0xc2,
    ///     0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8,
    /// ];
    ///
    /// let uuid = Uuid::from_slice_le(&bytes)?;
    ///
    /// assert_eq!(
    ///     uuid.hyphenated().to_string(),
    ///     "a4a3a2a1-b2b1-c2c1-d1d2-d3d4d5d6d7d8"
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_slice_le(b: &[u8]) -> Result<Uuid, Error> {
        if b.len() != 16 {
            return Err(
                Error(ErrorKind::ByteLength {
                    len: b.len(),
                }),
            );
        }
        let mut bytes: Bytes = [0; 16];
        bytes.copy_from_slice(b);
        Ok(Uuid::from_bytes_le(bytes))
    }
    /// Creates a UUID using the supplied bytes.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # fn main() -> Result<(), uuid::Error> {
    /// # use uuid::Uuid;
    /// let bytes = [
    ///     0xa1, 0xa2, 0xa3, 0xa4,
    ///     0xb1, 0xb2,
    ///     0xc1, 0xc2,
    ///     0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8,
    /// ];
    ///
    /// let uuid = Uuid::from_bytes(bytes);
    ///
    /// assert_eq!(
    ///     uuid.hyphenated().to_string(),
    ///     "a1a2a3a4-b1b2-c1c2-d1d2-d3d4d5d6d7d8"
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub const fn from_bytes(bytes: Bytes) -> Uuid {
        Uuid(bytes)
    }
    /// Creates a UUID using the supplied bytes in little endian order.
    ///
    /// The individual fields encoded in the buffer will be flipped.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # fn main() -> Result<(), uuid::Error> {
    /// # use uuid::Uuid;
    /// let bytes = [
    ///     0xa1, 0xa2, 0xa3, 0xa4,
    ///     0xb1, 0xb2,
    ///     0xc1, 0xc2,
    ///     0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8,
    /// ];
    ///
    /// let uuid = Uuid::from_bytes_le(bytes);
    ///
    /// assert_eq!(
    ///     "a4a3a2a1-b2b1-c2c1-d1d2-d3d4d5d6d7d8",
    ///     uuid.hyphenated().to_string(),
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub const fn from_bytes_le(b: Bytes) -> Uuid {
        Uuid([
            b[3],
            b[2],
            b[1],
            b[0],
            b[5],
            b[4],
            b[7],
            b[6],
            b[8],
            b[9],
            b[10],
            b[11],
            b[12],
            b[13],
            b[14],
            b[15],
        ])
    }
    /// Creates a reference to a UUID from a reference to the supplied bytes.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # fn main() -> Result<(), uuid::Error> {
    /// # use uuid::Uuid;
    /// let bytes = [
    ///     0xa1, 0xa2, 0xa3, 0xa4,
    ///     0xb1, 0xb2,
    ///     0xc1, 0xc2,
    ///     0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8,
    /// ];
    ///
    /// let uuid = Uuid::from_bytes_ref(&bytes);
    ///
    /// assert_eq!(
    ///     uuid.hyphenated().to_string(),
    ///     "a1a2a3a4-b1b2-c1c2-d1d2-d3d4d5d6d7d8"
    /// );
    ///
    /// assert!(std::ptr::eq(
    ///     uuid as *const Uuid as *const u8,
    ///     &bytes as *const [u8; 16] as *const u8,
    /// ));
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_bytes_ref(bytes: &Bytes) -> &Uuid {
        unsafe { &*(bytes as *const Bytes as *const Uuid) }
    }
}
impl Builder {
    /// Creates a `Builder` using the supplied bytes.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # use uuid::Builder;
    /// let bytes = [
    ///     0xa1, 0xa2, 0xa3, 0xa4,
    ///     0xb1, 0xb2,
    ///     0xc1, 0xc2,
    ///     0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8,
    /// ];
    ///
    /// let uuid = Builder::from_bytes(bytes).into_uuid();
    ///
    /// assert_eq!(
    ///     "a1a2a3a4-b1b2-c1c2-d1d2-d3d4d5d6d7d8",
    ///     uuid.hyphenated().to_string(),
    /// );
    /// ```
    pub const fn from_bytes(b: Bytes) -> Self {
        Builder(Uuid::from_bytes(b))
    }
    /// Creates a `Builder` using the supplied bytes in little endian order.
    ///
    /// The individual fields encoded in the buffer will be flipped.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # fn main() -> Result<(), uuid::Error> {
    /// # use uuid::{Builder, Uuid};
    /// let bytes = [
    ///     0xa1, 0xa2, 0xa3, 0xa4,
    ///     0xb1, 0xb2,
    ///     0xc1, 0xc2,
    ///     0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8,
    /// ];
    ///
    /// let uuid = Builder::from_bytes_le(bytes).into_uuid();
    ///
    /// assert_eq!(
    ///     "a4a3a2a1-b2b1-c2c1-d1d2-d3d4d5d6d7d8",
    ///     uuid.hyphenated().to_string(),
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub const fn from_bytes_le(b: Bytes) -> Self {
        Builder(Uuid::from_bytes_le(b))
    }
    /// Creates a `Builder` for a version 1 UUID using the supplied timestamp and node ID.
    pub const fn from_rfc4122_timestamp(
        ticks: u64,
        counter: u16,
        node_id: &[u8; 6],
    ) -> Self {
        Builder(timestamp::encode_rfc4122_timestamp(ticks, counter, node_id))
    }
    /// Creates a `Builder` for a version 3 UUID using the supplied MD5 hashed bytes.
    pub const fn from_md5_bytes(md5_bytes: Bytes) -> Self {
        Builder(Uuid::from_bytes(md5_bytes))
            .with_variant(Variant::RFC4122)
            .with_version(Version::Md5)
    }
    /// Creates a `Builder` for a version 4 UUID using the supplied random bytes.
    ///
    /// This method assumes the bytes are already sufficiently random, it will only
    /// set the appropriate bits for the UUID version and variant.
    ///
    /// # Examples
    ///
    /// ```
    /// # use uuid::{Builder, Variant, Version};
    /// # let rng = || [
    /// #     70, 235, 208, 238, 14, 109, 67, 201, 185, 13, 204, 195, 90,
    /// # 145, 63, 62,
    /// # ];
    /// let random_bytes = rng();
    /// let uuid = Builder::from_random_bytes(random_bytes).into_uuid();
    ///
    /// assert_eq!(Some(Version::Random), uuid.get_version());
    /// assert_eq!(Variant::RFC4122, uuid.get_variant());
    /// ```
    pub const fn from_random_bytes(random_bytes: Bytes) -> Self {
        Builder(Uuid::from_bytes(random_bytes))
            .with_variant(Variant::RFC4122)
            .with_version(Version::Random)
    }
    /// Creates a `Builder` for a version 5 UUID using the supplied SHA-1 hashed bytes.
    ///
    /// This method assumes the bytes are already a SHA-1 hash, it will only set the appropriate
    /// bits for the UUID version and variant.
    pub const fn from_sha1_bytes(sha1_bytes: Bytes) -> Self {
        Builder(Uuid::from_bytes(sha1_bytes))
            .with_variant(Variant::RFC4122)
            .with_version(Version::Sha1)
    }
    /// Creates a `Builder` for a version 6 UUID using the supplied timestamp and node ID.
    ///
    /// This method will encode the ticks, counter, and node ID in a sortable UUID.
    #[cfg(uuid_unstable)]
    pub const fn from_sorted_rfc4122_timestamp(
        ticks: u64,
        counter: u16,
        node_id: &[u8; 6],
    ) -> Self {
        Builder(timestamp::encode_sorted_rfc4122_timestamp(ticks, counter, node_id))
    }
    /// Creates a `Builder` for a version 7 UUID using the supplied Unix timestamp and random bytes.
    ///
    /// This method assumes the bytes are already sufficiently random.
    ///
    /// # Examples
    ///
    /// Creating a UUID using the current system timestamp:
    ///
    /// ```
    /// # use std::convert::TryInto;
    /// use std::time::{Duration, SystemTime};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # use uuid::{Builder, Uuid, Variant, Version, Timestamp, NoContext};
    /// # let rng = || [
    /// #     70, 235, 208, 238, 14, 109, 67, 201, 185, 13
    /// # ];
    /// let ts = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
    ///
    /// let random_bytes = rng();
    ///
    /// let uuid = Builder::from_unix_timestamp_millis(ts.as_millis().try_into()?, &random_bytes).into_uuid();
    ///
    /// assert_eq!(Some(Version::SortRand), uuid.get_version());
    /// assert_eq!(Variant::RFC4122, uuid.get_variant());
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(uuid_unstable)]
    pub const fn from_unix_timestamp_millis(
        millis: u64,
        random_bytes: &[u8; 10],
    ) -> Self {
        Builder(timestamp::encode_unix_timestamp_millis(millis, random_bytes))
    }
    /// Creates a `Builder` for a version 8 UUID using the supplied user-defined bytes.
    ///
    /// This method won't interpret the given bytes in any way, except to set the appropriate
    /// bits for the UUID version and variant.
    #[cfg(uuid_unstable)]
    pub const fn from_custom_bytes(custom_bytes: Bytes) -> Self {
        Builder::from_bytes(custom_bytes)
            .with_variant(Variant::RFC4122)
            .with_version(Version::Custom)
    }
    /// Creates a `Builder` using the supplied bytes.
    ///
    /// # Errors
    ///
    /// This function will return an error if `b` has any length other than 16.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # use uuid::Builder;
    /// # fn main() -> Result<(), uuid::Error> {
    /// let bytes = [
    ///     0xa1, 0xa2, 0xa3, 0xa4,
    ///     0xb1, 0xb2,
    ///     0xc1, 0xc2,
    ///     0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8,
    /// ];
    ///
    /// let uuid = Builder::from_slice(&bytes)?.into_uuid();
    ///
    /// assert_eq!(
    ///     "a1a2a3a4-b1b2-c1c2-d1d2-d3d4d5d6d7d8",
    ///     uuid.hyphenated().to_string(),
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_slice(b: &[u8]) -> Result<Self, Error> {
        Ok(Builder(Uuid::from_slice(b)?))
    }
    /// Creates a `Builder` using the supplied bytes in little endian order.
    ///
    /// The individual fields encoded in the buffer will be flipped.
    ///
    /// # Errors
    ///
    /// This function will return an error if `b` has any length other than 16.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # use uuid::Builder;
    /// # fn main() -> Result<(), uuid::Error> {
    /// let bytes = [
    ///     0xa1, 0xa2, 0xa3, 0xa4,
    ///     0xb1, 0xb2,
    ///     0xc1, 0xc2,
    ///     0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8,
    /// ];
    ///
    /// let uuid = Builder::from_slice_le(&bytes)?.into_uuid();
    ///
    /// assert_eq!(
    ///     "a4a3a2a1-b2b1-c2c1-d1d2-d3d4d5d6d7d8",
    ///     uuid.hyphenated().to_string(),
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_slice_le(b: &[u8]) -> Result<Self, Error> {
        Ok(Builder(Uuid::from_slice_le(b)?))
    }
    /// Creates a `Builder` from four field values.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # use uuid::Builder;
    /// let d1 = 0xa1a2a3a4;
    /// let d2 = 0xb1b2;
    /// let d3 = 0xc1c2;
    /// let d4 = [0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8];
    ///
    /// let uuid = Builder::from_fields(d1, d2, d3, &d4).into_uuid();
    ///
    /// assert_eq!(
    ///     uuid.hyphenated().to_string(),
    ///     "a1a2a3a4-b1b2-c1c2-d1d2-d3d4d5d6d7d8"
    /// );
    /// ```
    pub const fn from_fields(d1: u32, d2: u16, d3: u16, d4: &[u8; 8]) -> Self {
        Builder(Uuid::from_fields(d1, d2, d3, d4))
    }
    /// Creates a `Builder` from four field values.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # use uuid::Builder;
    /// let d1 = 0xa1a2a3a4;
    /// let d2 = 0xb1b2;
    /// let d3 = 0xc1c2;
    /// let d4 = [0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8];
    ///
    /// let uuid = Builder::from_fields_le(d1, d2, d3, &d4).into_uuid();
    ///
    /// assert_eq!(
    ///     uuid.hyphenated().to_string(),
    ///     "a4a3a2a1-b2b1-c2c1-d1d2-d3d4d5d6d7d8"
    /// );
    /// ```
    pub const fn from_fields_le(d1: u32, d2: u16, d3: u16, d4: &[u8; 8]) -> Self {
        Builder(Uuid::from_fields_le(d1, d2, d3, d4))
    }
    /// Creates a `Builder` from a 128bit value.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # use uuid::Builder;
    /// let v = 0xa1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d8u128;
    ///
    /// let uuid = Builder::from_u128(v).into_uuid();
    ///
    /// assert_eq!(
    ///     "a1a2a3a4-b1b2-c1c2-d1d2-d3d4d5d6d7d8",
    ///     uuid.hyphenated().to_string(),
    /// );
    /// ```
    pub const fn from_u128(v: u128) -> Self {
        Builder(Uuid::from_u128(v))
    }
    /// Creates a UUID from a 128bit value in little-endian order.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # use uuid::Builder;
    /// let v = 0xa1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d8u128;
    ///
    /// let uuid = Builder::from_u128_le(v).into_uuid();
    ///
    /// assert_eq!(
    ///     "d8d7d6d5-d4d3-d2d1-c2c1-b2b1a4a3a2a1",
    ///     uuid.hyphenated().to_string(),
    /// );
    /// ```
    pub const fn from_u128_le(v: u128) -> Self {
        Builder(Uuid::from_u128_le(v))
    }
    /// Creates a `Builder` with an initial [`Uuid::nil`].
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # use uuid::Builder;
    /// let uuid = Builder::nil().into_uuid();
    ///
    /// assert_eq!(
    ///     "00000000-0000-0000-0000-000000000000",
    ///     uuid.hyphenated().to_string(),
    /// );
    /// ```
    pub const fn nil() -> Self {
        Builder(Uuid::nil())
    }
    /// Specifies the variant of the UUID.
    pub fn set_variant(&mut self, v: Variant) -> &mut Self {
        *self = Builder(self.0).with_variant(v);
        self
    }
    /// Specifies the variant of the UUID.
    pub const fn with_variant(mut self, v: Variant) -> Self {
        let byte = (self.0).0[8];
        (self.0)
            .0[8] = match v {
            Variant::NCS => byte & 0x7f,
            Variant::RFC4122 => (byte & 0x3f) | 0x80,
            Variant::Microsoft => (byte & 0x1f) | 0xc0,
            Variant::Future => byte | 0xe0,
        };
        self
    }
    /// Specifies the version number of the UUID.
    pub fn set_version(&mut self, v: Version) -> &mut Self {
        *self = Builder(self.0).with_version(v);
        self
    }
    /// Specifies the version number of the UUID.
    pub const fn with_version(mut self, v: Version) -> Self {
        (self.0).0[6] = ((self.0).0[6] & 0x0f) | ((v as u8) << 4);
        self
    }
    /// Get a reference to the underlying [`Uuid`].
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # use uuid::Builder;
    /// let builder = Builder::nil();
    ///
    /// let uuid1 = builder.as_uuid();
    /// let uuid2 = builder.as_uuid();
    ///
    /// assert_eq!(uuid1, uuid2);
    /// ```
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }
    /// Convert the builder into a [`Uuid`].
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # use uuid::Builder;
    /// let uuid = Builder::nil().into_uuid();
    ///
    /// assert_eq!(
    ///     uuid.hyphenated().to_string(),
    ///     "00000000-0000-0000-0000-000000000000"
    /// );
    /// ```
    pub const fn into_uuid(self) -> Uuid {
        self.0
    }
}
#[cfg(test)]
mod tests_rug_70 {
    use super::*;
    use crate::Uuid;
    #[test]
    fn test_nil() {
        let _rug_st_tests_rug_70_rrrruuuugggg_test_nil = 0;
        let rug_fuzz_0 = "00000000-0000-0000-0000-000000000000";
        let uuid = Uuid::nil();
        debug_assert_eq!(rug_fuzz_0, uuid.hyphenated().to_string());
        let _rug_ed_tests_rug_70_rrrruuuugggg_test_nil = 0;
    }
}
#[cfg(test)]
mod tests_rug_71 {
    use super::*;
    use crate::Uuid;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_71_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0xa1a2a3a4;
        let rug_fuzz_1 = 0xb1b2;
        let rug_fuzz_2 = 0xc1c2;
        let rug_fuzz_3 = 0xd1;
        let rug_fuzz_4 = 0xd2;
        let rug_fuzz_5 = 0xd3;
        let rug_fuzz_6 = 0xd4;
        let rug_fuzz_7 = 0xd5;
        let rug_fuzz_8 = 0xd6;
        let rug_fuzz_9 = 0xd7;
        let rug_fuzz_10 = 0xd8;
        let mut p0: u32 = rug_fuzz_0;
        let mut p1: u16 = rug_fuzz_1;
        let mut p2: u16 = rug_fuzz_2;
        let mut p3: [u8; 8] = [
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
            rug_fuzz_6,
            rug_fuzz_7,
            rug_fuzz_8,
            rug_fuzz_9,
            rug_fuzz_10,
        ];
        Uuid::from_fields(p0, p1, p2, &p3);
        let _rug_ed_tests_rug_71_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_72 {
    use super::*;
    use crate::Uuid;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_72_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0xa1a2a3a4;
        let rug_fuzz_1 = 0xb1b2;
        let rug_fuzz_2 = 0xc1c2;
        let rug_fuzz_3 = 0xd1;
        let rug_fuzz_4 = 0xd2;
        let rug_fuzz_5 = 0xd3;
        let rug_fuzz_6 = 0xd4;
        let rug_fuzz_7 = 0xd5;
        let rug_fuzz_8 = 0xd6;
        let rug_fuzz_9 = 0xd7;
        let rug_fuzz_10 = 0xd8;
        let mut p0: u32 = rug_fuzz_0;
        let mut p1: u16 = rug_fuzz_1;
        let mut p2: u16 = rug_fuzz_2;
        let mut p3: [u8; 8] = [
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
            rug_fuzz_6,
            rug_fuzz_7,
            rug_fuzz_8,
            rug_fuzz_9,
            rug_fuzz_10,
        ];
        <Uuid>::from_fields_le(p0, p1, p2, &p3);
        let _rug_ed_tests_rug_72_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_73 {
    use super::*;
    use crate::Uuid;
    #[test]
    fn test_from_u128() {
        let _rug_st_tests_rug_73_rrrruuuugggg_test_from_u128 = 0;
        let rug_fuzz_0 = 0xa1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d8;
        let p0: u128 = rug_fuzz_0;
        Uuid::from_u128(p0);
        let _rug_ed_tests_rug_73_rrrruuuugggg_test_from_u128 = 0;
    }
}
#[cfg(test)]
mod tests_rug_74 {
    use super::*;
    use crate::Uuid;
    #[test]
    fn test_from_u128_le() {
        let _rug_st_tests_rug_74_rrrruuuugggg_test_from_u128_le = 0;
        let rug_fuzz_0 = 0xa1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d8;
        let p0: u128 = rug_fuzz_0;
        Uuid::from_u128_le(p0);
        let _rug_ed_tests_rug_74_rrrruuuugggg_test_from_u128_le = 0;
    }
}
#[cfg(test)]
mod tests_rug_75 {
    use super::*;
    use crate::Uuid;
    #[test]
    fn test_from_u64_pair() {
        let _rug_st_tests_rug_75_rrrruuuugggg_test_from_u64_pair = 0;
        let rug_fuzz_0 = 0xa1a2a3a4b1b2c1c2;
        let rug_fuzz_1 = 0xd1d2d3d4d5d6d7d8;
        let rug_fuzz_2 = "a1a2a3a4-b1b2-c1c2-d1d2-d3d4d5d6d7d8";
        let p0: u64 = rug_fuzz_0;
        let p1: u64 = rug_fuzz_1;
        let uuid = Uuid::from_u64_pair(p0, p1);
        debug_assert_eq!(rug_fuzz_2, uuid.hyphenated().to_string());
        let _rug_ed_tests_rug_75_rrrruuuugggg_test_from_u64_pair = 0;
    }
}
#[cfg(test)]
mod tests_rug_76 {
    use super::*;
    #[test]
    fn test_from_slice() {
        let _rug_st_tests_rug_76_rrrruuuugggg_test_from_slice = 0;
        let rug_fuzz_0 = 0xa1;
        let rug_fuzz_1 = 0xa2;
        let rug_fuzz_2 = 0xa3;
        let rug_fuzz_3 = 0xa4;
        let rug_fuzz_4 = 0xb1;
        let rug_fuzz_5 = 0xb2;
        let rug_fuzz_6 = 0xc1;
        let rug_fuzz_7 = 0xc2;
        let rug_fuzz_8 = 0xd1;
        let rug_fuzz_9 = 0xd2;
        let rug_fuzz_10 = 0xd3;
        let rug_fuzz_11 = 0xd4;
        let rug_fuzz_12 = 0xd5;
        let rug_fuzz_13 = 0xd6;
        let rug_fuzz_14 = 0xd7;
        let rug_fuzz_15 = 0xd8;
        let p0: &[u8] = &[
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
        let result = <Uuid>::from_slice(p0);
        debug_assert_eq!(result.is_ok(), true);
        let _rug_ed_tests_rug_76_rrrruuuugggg_test_from_slice = 0;
    }
}
#[cfg(test)]
mod tests_rug_77 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_77_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0xa1;
        let rug_fuzz_1 = 0xa2;
        let rug_fuzz_2 = 0xa3;
        let rug_fuzz_3 = 0xa4;
        let rug_fuzz_4 = 0xb1;
        let rug_fuzz_5 = 0xb2;
        let rug_fuzz_6 = 0xc1;
        let rug_fuzz_7 = 0xc2;
        let rug_fuzz_8 = 0xd1;
        let rug_fuzz_9 = 0xd2;
        let rug_fuzz_10 = 0xd3;
        let rug_fuzz_11 = 0xd4;
        let rug_fuzz_12 = 0xd5;
        let rug_fuzz_13 = 0xd6;
        let rug_fuzz_14 = 0xd7;
        let rug_fuzz_15 = 0xd8;
        let p0: &[u8] = &[
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
        <Uuid>::from_slice_le(p0);
        let _rug_ed_tests_rug_77_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_78 {
    use super::*;
    use crate::Uuid;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_78_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0xa1;
        let rug_fuzz_1 = 0xa2;
        let rug_fuzz_2 = 0xa3;
        let rug_fuzz_3 = 0xa4;
        let rug_fuzz_4 = 0xb1;
        let rug_fuzz_5 = 0xb2;
        let rug_fuzz_6 = 0xc1;
        let rug_fuzz_7 = 0xc2;
        let rug_fuzz_8 = 0xd1;
        let rug_fuzz_9 = 0xd2;
        let rug_fuzz_10 = 0xd3;
        let rug_fuzz_11 = 0xd4;
        let rug_fuzz_12 = 0xd5;
        let rug_fuzz_13 = 0xd6;
        let rug_fuzz_14 = 0xd7;
        let rug_fuzz_15 = 0xd8;
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
        Uuid::from_bytes(p0);
        let _rug_ed_tests_rug_78_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_79 {
    use super::*;
    use crate::Uuid;
    #[test]
    fn test_from_bytes_le() {
        let _rug_st_tests_rug_79_rrrruuuugggg_test_from_bytes_le = 0;
        let rug_fuzz_0 = 0xa1;
        let rug_fuzz_1 = 0xa2;
        let rug_fuzz_2 = 0xa3;
        let rug_fuzz_3 = 0xa4;
        let rug_fuzz_4 = 0xb1;
        let rug_fuzz_5 = 0xb2;
        let rug_fuzz_6 = 0xc1;
        let rug_fuzz_7 = 0xc2;
        let rug_fuzz_8 = 0xd1;
        let rug_fuzz_9 = 0xd2;
        let rug_fuzz_10 = 0xd3;
        let rug_fuzz_11 = 0xd4;
        let rug_fuzz_12 = 0xd5;
        let rug_fuzz_13 = 0xd6;
        let rug_fuzz_14 = 0xd7;
        let rug_fuzz_15 = 0xd8;
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
        let _ = Uuid::from_bytes_le(p0);
        let _rug_ed_tests_rug_79_rrrruuuugggg_test_from_bytes_le = 0;
    }
}
#[cfg(test)]
mod tests_rug_80 {
    use super::*;
    use crate::Uuid;
    use crate::builder::Bytes;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_80_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0xa1;
        let rug_fuzz_1 = 0xa2;
        let rug_fuzz_2 = 0xa3;
        let rug_fuzz_3 = 0xa4;
        let rug_fuzz_4 = 0xb1;
        let rug_fuzz_5 = 0xb2;
        let rug_fuzz_6 = 0xc1;
        let rug_fuzz_7 = 0xc2;
        let rug_fuzz_8 = 0xd1;
        let rug_fuzz_9 = 0xd2;
        let rug_fuzz_10 = 0xd3;
        let rug_fuzz_11 = 0xd4;
        let rug_fuzz_12 = 0xd5;
        let rug_fuzz_13 = 0xd6;
        let rug_fuzz_14 = 0xd7;
        let rug_fuzz_15 = 0xd8;
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
        let bytes: &Bytes = &p0;
        <Uuid>::from_bytes_ref(bytes);
        let _rug_ed_tests_rug_80_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_81 {
    use super::*;
    use crate::{Builder, Uuid};
    #[test]
    fn test_from_bytes() {
        let _rug_st_tests_rug_81_rrrruuuugggg_test_from_bytes = 0;
        let rug_fuzz_0 = 0xa1;
        let rug_fuzz_1 = 0xa2;
        let rug_fuzz_2 = 0xa3;
        let rug_fuzz_3 = 0xa4;
        let rug_fuzz_4 = 0xb1;
        let rug_fuzz_5 = 0xb2;
        let rug_fuzz_6 = 0xc1;
        let rug_fuzz_7 = 0xc2;
        let rug_fuzz_8 = 0xd1;
        let rug_fuzz_9 = 0xd2;
        let rug_fuzz_10 = 0xd3;
        let rug_fuzz_11 = 0xd4;
        let rug_fuzz_12 = 0xd5;
        let rug_fuzz_13 = 0xd6;
        let rug_fuzz_14 = 0xd7;
        let rug_fuzz_15 = 0xd8;
        let rug_fuzz_16 = "a1a2a3a4-b1b2-c1c2-d1d2-d3d4d5d6d7d8";
        let bytes: [u8; 16] = [
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
        let uuid = Builder::from_bytes(bytes).into_uuid();
        debug_assert_eq!(rug_fuzz_16, uuid.hyphenated().to_string());
        let _rug_ed_tests_rug_81_rrrruuuugggg_test_from_bytes = 0;
    }
}
#[cfg(test)]
mod tests_rug_82 {
    use super::*;
    use crate::{Builder, Uuid};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_82_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0xa1;
        let rug_fuzz_1 = 0xa2;
        let rug_fuzz_2 = 0xa3;
        let rug_fuzz_3 = 0xa4;
        let rug_fuzz_4 = 0xb1;
        let rug_fuzz_5 = 0xb2;
        let rug_fuzz_6 = 0xc1;
        let rug_fuzz_7 = 0xc2;
        let rug_fuzz_8 = 0xd1;
        let rug_fuzz_9 = 0xd2;
        let rug_fuzz_10 = 0xd3;
        let rug_fuzz_11 = 0xd4;
        let rug_fuzz_12 = 0xd5;
        let rug_fuzz_13 = 0xd6;
        let rug_fuzz_14 = 0xd7;
        let rug_fuzz_15 = 0xd8;
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
        let _ = <Builder>::from_bytes_le(p0);
        let _rug_ed_tests_rug_82_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_83 {
    use super::*;
    use crate::builder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_83_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1609459200;
        let rug_fuzz_1 = 1234;
        let rug_fuzz_2 = 0x01;
        let rug_fuzz_3 = 0x02;
        let rug_fuzz_4 = 0x03;
        let rug_fuzz_5 = 0x04;
        let rug_fuzz_6 = 0x05;
        let rug_fuzz_7 = 0x06;
        let mut p0: u64 = rug_fuzz_0;
        let mut p1: u16 = rug_fuzz_1;
        let mut p2: [u8; 6] = [
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
            rug_fuzz_6,
            rug_fuzz_7,
        ];
        builder::Builder::from_rfc4122_timestamp(p0, p1, &p2);
        let _rug_ed_tests_rug_83_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_84 {
    use super::*;
    use crate::builder::Builder;
    #[test]
    fn test_from_md5_bytes() {
        let _rug_st_tests_rug_84_rrrruuuugggg_test_from_md5_bytes = 0;
        let rug_fuzz_0 = 0x32;
        let rug_fuzz_1 = 0x2A;
        let rug_fuzz_2 = 0xBA;
        let rug_fuzz_3 = 0x26;
        let rug_fuzz_4 = 0x93;
        let rug_fuzz_5 = 0x3F;
        let rug_fuzz_6 = 0x4A;
        let rug_fuzz_7 = 0xEC;
        let rug_fuzz_8 = 0xA5;
        let rug_fuzz_9 = 0xAA;
        let rug_fuzz_10 = 0xAC;
        let rug_fuzz_11 = 0xB9;
        let rug_fuzz_12 = 0x24;
        let rug_fuzz_13 = 0xC3;
        let rug_fuzz_14 = 0xFD;
        let rug_fuzz_15 = 0xC7;
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
        Builder::from_md5_bytes(p0);
        let _rug_ed_tests_rug_84_rrrruuuugggg_test_from_md5_bytes = 0;
    }
}
#[cfg(test)]
mod tests_rug_85 {
    use super::*;
    use crate::{Builder, Variant, Version};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_85_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 70;
        let rug_fuzz_1 = 235;
        let rug_fuzz_2 = 208;
        let rug_fuzz_3 = 238;
        let rug_fuzz_4 = 14;
        let rug_fuzz_5 = 109;
        let rug_fuzz_6 = 67;
        let rug_fuzz_7 = 201;
        let rug_fuzz_8 = 185;
        let rug_fuzz_9 = 13;
        let rug_fuzz_10 = 204;
        let rug_fuzz_11 = 195;
        let rug_fuzz_12 = 90;
        let rug_fuzz_13 = 145;
        let rug_fuzz_14 = 63;
        let rug_fuzz_15 = 62;
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
        Builder::from_random_bytes(p0);
        let _rug_ed_tests_rug_85_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_86 {
    use super::*;
    use crate::{
        Bytes, builder::{self, Builder},
        Uuid, Variant, Version,
    };
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_86_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let mut p0: [u8; 16] = [rug_fuzz_0; 16];
        <builder::Builder>::from_sha1_bytes(Bytes::from(p0));
        let _rug_ed_tests_rug_86_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_87 {
    use super::*;
    use crate::{Builder, Error};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_87_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0xa1;
        let rug_fuzz_1 = 0xa2;
        let rug_fuzz_2 = 0xa3;
        let rug_fuzz_3 = 0xa4;
        let rug_fuzz_4 = 0xb1;
        let rug_fuzz_5 = 0xb2;
        let rug_fuzz_6 = 0xc1;
        let rug_fuzz_7 = 0xc2;
        let rug_fuzz_8 = 0xd1;
        let rug_fuzz_9 = 0xd2;
        let rug_fuzz_10 = 0xd3;
        let rug_fuzz_11 = 0xd4;
        let rug_fuzz_12 = 0xd5;
        let rug_fuzz_13 = 0xd6;
        let rug_fuzz_14 = 0xd7;
        let rug_fuzz_15 = 0xd8;
        let mut p0: &[u8] = &[
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
        Builder::from_slice(p0).unwrap();
        let _rug_ed_tests_rug_87_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_88 {
    use super::*;
    use crate::{Builder, Error};
    #[test]
    fn test_from_slice_le() {
        let _rug_st_tests_rug_88_rrrruuuugggg_test_from_slice_le = 0;
        let rug_fuzz_0 = 0xa1;
        let rug_fuzz_1 = 0xa2;
        let rug_fuzz_2 = 0xa3;
        let rug_fuzz_3 = 0xa4;
        let rug_fuzz_4 = 0xb1;
        let rug_fuzz_5 = 0xb2;
        let rug_fuzz_6 = 0xc1;
        let rug_fuzz_7 = 0xc2;
        let rug_fuzz_8 = 0xd1;
        let rug_fuzz_9 = 0xd2;
        let rug_fuzz_10 = 0xd3;
        let rug_fuzz_11 = 0xd4;
        let rug_fuzz_12 = 0xd5;
        let rug_fuzz_13 = 0xd6;
        let rug_fuzz_14 = 0xd7;
        let rug_fuzz_15 = 0xd8;
        let p0: &[u8] = &[
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
        let result = Builder::from_slice_le(p0);
        debug_assert!(result.is_ok());
        let _rug_ed_tests_rug_88_rrrruuuugggg_test_from_slice_le = 0;
    }
}
#[cfg(test)]
mod tests_rug_89 {
    use super::*;
    use crate::Builder;
    #[test]
    fn test_from_fields() {
        let _rug_st_tests_rug_89_rrrruuuugggg_test_from_fields = 0;
        let rug_fuzz_0 = 0xa1a2a3a4;
        let rug_fuzz_1 = 0xb1b2;
        let rug_fuzz_2 = 0xc1c2;
        let rug_fuzz_3 = 0xd1;
        let rug_fuzz_4 = 0xd2;
        let rug_fuzz_5 = 0xd3;
        let rug_fuzz_6 = 0xd4;
        let rug_fuzz_7 = 0xd5;
        let rug_fuzz_8 = 0xd6;
        let rug_fuzz_9 = 0xd7;
        let rug_fuzz_10 = 0xd8;
        let p0: u32 = rug_fuzz_0;
        let p1: u16 = rug_fuzz_1;
        let p2: u16 = rug_fuzz_2;
        let p3: [u8; 8] = [
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
            rug_fuzz_6,
            rug_fuzz_7,
            rug_fuzz_8,
            rug_fuzz_9,
            rug_fuzz_10,
        ];
        Builder::from_fields(p0, p1, p2, &p3);
        let _rug_ed_tests_rug_89_rrrruuuugggg_test_from_fields = 0;
    }
}
#[cfg(test)]
mod tests_rug_90 {
    use super::*;
    use crate::Builder;
    #[test]
    fn test_from_fields_le() {
        let _rug_st_tests_rug_90_rrrruuuugggg_test_from_fields_le = 0;
        let rug_fuzz_0 = 0xa1a2a3a4;
        let rug_fuzz_1 = 0xb1b2;
        let rug_fuzz_2 = 0xc1c2;
        let rug_fuzz_3 = 0xd1;
        let rug_fuzz_4 = 0xd2;
        let rug_fuzz_5 = 0xd3;
        let rug_fuzz_6 = 0xd4;
        let rug_fuzz_7 = 0xd5;
        let rug_fuzz_8 = 0xd6;
        let rug_fuzz_9 = 0xd7;
        let rug_fuzz_10 = 0xd8;
        let p0: u32 = rug_fuzz_0;
        let p1: u16 = rug_fuzz_1;
        let p2: u16 = rug_fuzz_2;
        let p3: [u8; 8] = [
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
            rug_fuzz_6,
            rug_fuzz_7,
            rug_fuzz_8,
            rug_fuzz_9,
            rug_fuzz_10,
        ];
        Builder::from_fields_le(p0, p1, p2, &p3);
        let _rug_ed_tests_rug_90_rrrruuuugggg_test_from_fields_le = 0;
    }
}
#[cfg(test)]
mod tests_rug_91 {
    use super::*;
    use crate::Builder;
    #[test]
    fn test_from_u128() {
        let _rug_st_tests_rug_91_rrrruuuugggg_test_from_u128 = 0;
        let rug_fuzz_0 = 0xa1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d8;
        let p0: u128 = rug_fuzz_0;
        Builder::from_u128(p0);
        let _rug_ed_tests_rug_91_rrrruuuugggg_test_from_u128 = 0;
    }
}
#[cfg(test)]
mod tests_rug_92 {
    use super::*;
    use crate::Builder;
    #[test]
    fn test_from_u128_le() {
        let _rug_st_tests_rug_92_rrrruuuugggg_test_from_u128_le = 0;
        let rug_fuzz_0 = 0xa1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d8u128;
        let p0: u128 = rug_fuzz_0;
        Builder::from_u128_le(p0);
        let _rug_ed_tests_rug_92_rrrruuuugggg_test_from_u128_le = 0;
    }
}
#[cfg(test)]
mod tests_rug_93 {
    use super::*;
    use crate::Builder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_93_rrrruuuugggg_test_rug = 0;
        Builder::nil();
        let _rug_ed_tests_rug_93_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_94 {
    use super::*;
    use crate::{Builder, Variant};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_94_rrrruuuugggg_test_rug = 0;
        let mut p0 = Builder::nil();
        let mut p1 = Variant::RFC4122;
        p0.set_variant(p1);
        let _rug_ed_tests_rug_94_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_95 {
    use super::*;
    use crate::builder::Builder;
    use crate::Variant;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_95_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let mut p0 = Builder::from_bytes([rug_fuzz_0; 16]);
        let mut p1: Variant = Variant::RFC4122;
        Builder::with_variant(p0, p1);
        let _rug_ed_tests_rug_95_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_96 {
    use super::*;
    use crate::{Builder, Version};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_96_rrrruuuugggg_test_rug = 0;
        let mut p0 = Builder::nil();
        let mut p1 = Version::Mac;
        Builder::set_version(&mut p0, p1);
        let _rug_ed_tests_rug_96_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_97 {
    use super::*;
    use crate::{builder::Builder, Version};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_97_rrrruuuugggg_test_rug = 0;
        let mut p0 = Builder::nil();
        let p1 = Version::Mac;
        Builder::with_version(p0, p1);
        let _rug_ed_tests_rug_97_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_98 {
    use super::*;
    use crate::Builder;
    #[test]
    fn test_as_uuid() {
        let _rug_st_tests_rug_98_rrrruuuugggg_test_as_uuid = 0;
        let builder = Builder::nil();
        let uuid1 = builder.as_uuid();
        let uuid2 = builder.as_uuid();
        debug_assert_eq!(uuid1, uuid2);
        let _rug_ed_tests_rug_98_rrrruuuugggg_test_as_uuid = 0;
    }
}
#[cfg(test)]
mod tests_rug_99 {
    use super::*;
    use crate::{Builder, Uuid};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_99_rrrruuuugggg_test_rug = 0;
        let mut p0 = Builder::nil();
        let result: Uuid = p0.into_uuid();
        let _rug_ed_tests_rug_99_rrrruuuugggg_test_rug = 0;
    }
}
