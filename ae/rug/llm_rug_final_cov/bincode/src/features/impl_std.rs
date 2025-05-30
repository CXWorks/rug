use crate::{
    config::Config,
    de::{read::Reader, BorrowDecode, BorrowDecoder, Decode, Decoder, DecoderImpl},
    enc::{write::Writer, Encode, Encoder, EncoderImpl},
    error::{DecodeError, EncodeError},
    impl_borrow_decode,
};
use core::time::Duration;
use std::{
    collections::{HashMap, HashSet},
    ffi::{CStr, CString},
    hash::Hash, io::Read,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
    path::{Path, PathBuf},
    sync::{Mutex, RwLock},
    time::SystemTime,
};
/// Decode type `D` from the given reader with the given `Config`. The reader can be any type that implements `std::io::Read`, e.g. `std::fs::File`.
///
/// See the [config] module for more information about config options.
///
/// [config]: config/index.html
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub fn decode_from_std_read<D: Decode, C: Config, R: std::io::Read>(
    src: &mut R,
    config: C,
) -> Result<D, DecodeError> {
    let reader = IoReader::new(src);
    let mut decoder = DecoderImpl::<_, C>::new(reader, config);
    D::decode(&mut decoder)
}
pub(crate) struct IoReader<R> {
    reader: R,
}
impl<R> IoReader<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }
}
impl<R> Reader for IoReader<R>
where
    R: std::io::Read,
{
    #[inline(always)]
    fn read(&mut self, bytes: &mut [u8]) -> Result<(), DecodeError> {
        self.reader
            .read_exact(bytes)
            .map_err(|inner| DecodeError::Io {
                inner,
                additional: bytes.len(),
            })
    }
}
impl<R> Reader for std::io::BufReader<R>
where
    R: std::io::Read,
{
    fn read(&mut self, bytes: &mut [u8]) -> Result<(), DecodeError> {
        self.read_exact(bytes)
            .map_err(|inner| DecodeError::Io {
                inner,
                additional: bytes.len(),
            })
    }
    #[inline]
    fn peek_read(&mut self, n: usize) -> Option<&[u8]> {
        self.buffer().get(..n)
    }
    #[inline]
    fn consume(&mut self, n: usize) {
        <Self as std::io::BufRead>::consume(self, n);
    }
}
/// Encode the given value into any type that implements `std::io::Write`, e.g. `std::fs::File`, with the given `Config`.
/// See the [config] module for more information.
/// Returns the amount of bytes written.
///
/// [config]: config/index.html
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub fn encode_into_std_write<E: Encode, C: Config, W: std::io::Write>(
    val: E,
    dst: &mut W,
    config: C,
) -> Result<usize, EncodeError> {
    let writer = IoWriter::new(dst);
    let mut encoder = EncoderImpl::<_, C>::new(writer, config);
    val.encode(&mut encoder)?;
    Ok(encoder.into_writer().bytes_written())
}
pub(crate) struct IoWriter<'a, W: std::io::Write> {
    writer: &'a mut W,
    bytes_written: usize,
}
impl<'a, W: std::io::Write> IoWriter<'a, W> {
    pub fn new(writer: &'a mut W) -> Self {
        Self { writer, bytes_written: 0 }
    }
    pub fn bytes_written(&self) -> usize {
        self.bytes_written
    }
}
impl<'storage, W: std::io::Write> Writer for IoWriter<'storage, W> {
    #[inline(always)]
    fn write(&mut self, bytes: &[u8]) -> Result<(), EncodeError> {
        self.writer
            .write_all(bytes)
            .map_err(|inner| EncodeError::Io {
                inner,
                index: self.bytes_written,
            })?;
        self.bytes_written += bytes.len();
        Ok(())
    }
}
impl<'a> Encode for &'a CStr {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        self.to_bytes().encode(encoder)
    }
}
impl Encode for CString {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        self.as_bytes().encode(encoder)
    }
}
impl Decode for CString {
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        let vec = std::vec::Vec::decode(decoder)?;
        CString::new(vec)
            .map_err(|inner| DecodeError::CStringNulError {
                position: inner.nul_position(),
            })
    }
}
impl_borrow_decode!(CString);
impl<T> Encode for Mutex<T>
where
    T: Encode,
{
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        let t = self
            .lock()
            .map_err(|_| EncodeError::LockFailed {
                type_name: core::any::type_name::<Mutex<T>>(),
            })?;
        t.encode(encoder)
    }
}
impl<T> Decode for Mutex<T>
where
    T: Decode,
{
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        let t = T::decode(decoder)?;
        Ok(Mutex::new(t))
    }
}
impl<'de, T> BorrowDecode<'de> for Mutex<T>
where
    T: BorrowDecode<'de>,
{
    fn borrow_decode<D: BorrowDecoder<'de>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        let t = T::borrow_decode(decoder)?;
        Ok(Mutex::new(t))
    }
}
impl<T> Encode for RwLock<T>
where
    T: Encode,
{
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        let t = self
            .read()
            .map_err(|_| EncodeError::LockFailed {
                type_name: core::any::type_name::<RwLock<T>>(),
            })?;
        t.encode(encoder)
    }
}
impl<T> Decode for RwLock<T>
where
    T: Decode,
{
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        let t = T::decode(decoder)?;
        Ok(RwLock::new(t))
    }
}
impl<'de, T> BorrowDecode<'de> for RwLock<T>
where
    T: BorrowDecode<'de>,
{
    fn borrow_decode<D: BorrowDecoder<'de>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        let t = T::borrow_decode(decoder)?;
        Ok(RwLock::new(t))
    }
}
impl Encode for SystemTime {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        let duration = self
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| {
                EncodeError::InvalidSystemTime {
                    inner: e,
                    time: std::boxed::Box::new(*self),
                }
            })?;
        duration.encode(encoder)
    }
}
impl Decode for SystemTime {
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        let duration = Duration::decode(decoder)?;
        match SystemTime::UNIX_EPOCH.checked_add(duration) {
            Some(t) => Ok(t),
            None => {
                Err(DecodeError::InvalidSystemTime {
                    duration,
                })
            }
        }
    }
}
impl_borrow_decode!(SystemTime);
impl Encode for &'_ Path {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        match self.to_str() {
            Some(str) => str.encode(encoder),
            None => Err(EncodeError::InvalidPathCharacters),
        }
    }
}
impl<'de> BorrowDecode<'de> for &'de Path {
    fn borrow_decode<D: BorrowDecoder<'de>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        let str = <&'de str>::borrow_decode(decoder)?;
        Ok(Path::new(str))
    }
}
impl Encode for PathBuf {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        self.as_path().encode(encoder)
    }
}
impl Decode for PathBuf {
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        let string = std::string::String::decode(decoder)?;
        Ok(string.into())
    }
}
impl_borrow_decode!(PathBuf);
impl Encode for IpAddr {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        match self {
            IpAddr::V4(v4) => {
                0u32.encode(encoder)?;
                v4.encode(encoder)
            }
            IpAddr::V6(v6) => {
                1u32.encode(encoder)?;
                v6.encode(encoder)
            }
        }
    }
}
impl Decode for IpAddr {
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        match u32::decode(decoder)? {
            0 => Ok(IpAddr::V4(Ipv4Addr::decode(decoder)?)),
            1 => Ok(IpAddr::V6(Ipv6Addr::decode(decoder)?)),
            found => {
                Err(DecodeError::UnexpectedVariant {
                    allowed: &crate::error::AllowedEnumVariants::Range {
                        min: 0,
                        max: 1,
                    },
                    found,
                    type_name: core::any::type_name::<IpAddr>(),
                })
            }
        }
    }
}
impl_borrow_decode!(IpAddr);
impl Encode for Ipv4Addr {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        encoder.writer().write(&self.octets())
    }
}
impl Decode for Ipv4Addr {
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        let mut buff = [0u8; 4];
        decoder.reader().read(&mut buff)?;
        Ok(Self::from(buff))
    }
}
impl_borrow_decode!(Ipv4Addr);
impl Encode for Ipv6Addr {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        encoder.writer().write(&self.octets())
    }
}
impl Decode for Ipv6Addr {
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        let mut buff = [0u8; 16];
        decoder.reader().read(&mut buff)?;
        Ok(Self::from(buff))
    }
}
impl_borrow_decode!(Ipv6Addr);
impl Encode for SocketAddr {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        match self {
            SocketAddr::V4(v4) => {
                0u32.encode(encoder)?;
                v4.encode(encoder)
            }
            SocketAddr::V6(v6) => {
                1u32.encode(encoder)?;
                v6.encode(encoder)
            }
        }
    }
}
impl Decode for SocketAddr {
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        match u32::decode(decoder)? {
            0 => Ok(SocketAddr::V4(SocketAddrV4::decode(decoder)?)),
            1 => Ok(SocketAddr::V6(SocketAddrV6::decode(decoder)?)),
            found => {
                Err(DecodeError::UnexpectedVariant {
                    allowed: &crate::error::AllowedEnumVariants::Range {
                        min: 0,
                        max: 1,
                    },
                    found,
                    type_name: core::any::type_name::<SocketAddr>(),
                })
            }
        }
    }
}
impl_borrow_decode!(SocketAddr);
impl Encode for SocketAddrV4 {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        self.ip().encode(encoder)?;
        self.port().encode(encoder)
    }
}
impl Decode for SocketAddrV4 {
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        let ip = Ipv4Addr::decode(decoder)?;
        let port = u16::decode(decoder)?;
        Ok(Self::new(ip, port))
    }
}
impl_borrow_decode!(SocketAddrV4);
impl Encode for SocketAddrV6 {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        self.ip().encode(encoder)?;
        self.port().encode(encoder)
    }
}
impl Decode for SocketAddrV6 {
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        let ip = Ipv6Addr::decode(decoder)?;
        let port = u16::decode(decoder)?;
        Ok(Self::new(ip, port, 0, 0))
    }
}
impl_borrow_decode!(SocketAddrV6);
impl std::error::Error for EncodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::RefCellAlreadyBorrowed { inner, .. } => Some(inner),
            Self::Io { inner, .. } => Some(inner),
            Self::InvalidSystemTime { inner, .. } => Some(inner),
            _ => None,
        }
    }
}
impl std::error::Error for DecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Utf8 { inner } => Some(inner),
            _ => None,
        }
    }
}
impl<K, V, S> Encode for HashMap<K, V, S>
where
    K: Encode,
    V: Encode,
{
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        crate::enc::encode_slice_len(encoder, self.len())?;
        for (k, v) in self.iter() {
            Encode::encode(k, encoder)?;
            Encode::encode(v, encoder)?;
        }
        Ok(())
    }
}
impl<K, V, S> Decode for HashMap<K, V, S>
where
    K: Decode + Eq + std::hash::Hash,
    V: Decode,
    S: std::hash::BuildHasher + Default,
{
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        let len = crate::de::decode_slice_len(decoder)?;
        decoder.claim_container_read::<(K, V)>(len)?;
        let hash_builder: S = Default::default();
        let mut map = HashMap::with_capacity_and_hasher(len, hash_builder);
        for _ in 0..len {
            decoder.unclaim_bytes_read(core::mem::size_of::<(K, V)>());
            let k = K::decode(decoder)?;
            let v = V::decode(decoder)?;
            map.insert(k, v);
        }
        Ok(map)
    }
}
impl<'de, K, V, S> BorrowDecode<'de> for HashMap<K, V, S>
where
    K: BorrowDecode<'de> + Eq + std::hash::Hash,
    V: BorrowDecode<'de>,
    S: std::hash::BuildHasher + Default,
{
    fn borrow_decode<D: BorrowDecoder<'de>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        let len = crate::de::decode_slice_len(decoder)?;
        decoder.claim_container_read::<(K, V)>(len)?;
        let hash_builder: S = Default::default();
        let mut map = HashMap::with_capacity_and_hasher(len, hash_builder);
        for _ in 0..len {
            decoder.unclaim_bytes_read(core::mem::size_of::<(K, V)>());
            let k = K::borrow_decode(decoder)?;
            let v = V::borrow_decode(decoder)?;
            map.insert(k, v);
        }
        Ok(map)
    }
}
impl<T, S> Decode for HashSet<T, S>
where
    T: Decode + Eq + Hash,
    S: std::hash::BuildHasher + Default,
{
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        let len = crate::de::decode_slice_len(decoder)?;
        decoder.claim_container_read::<T>(len)?;
        let hash_builder: S = Default::default();
        let mut map: HashSet<T, S> = HashSet::with_capacity_and_hasher(
            len,
            hash_builder,
        );
        for _ in 0..len {
            decoder.unclaim_bytes_read(core::mem::size_of::<T>());
            let key = T::decode(decoder)?;
            map.insert(key);
        }
        Ok(map)
    }
}
impl<'de, T, S> BorrowDecode<'de> for HashSet<T, S>
where
    T: BorrowDecode<'de> + Eq + Hash,
    S: std::hash::BuildHasher + Default,
{
    fn borrow_decode<D: BorrowDecoder<'de>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        let len = crate::de::decode_slice_len(decoder)?;
        decoder.claim_container_read::<T>(len)?;
        let mut map = HashSet::with_capacity_and_hasher(len, S::default());
        for _ in 0..len {
            decoder.unclaim_bytes_read(core::mem::size_of::<T>());
            let key = T::borrow_decode(decoder)?;
            map.insert(key);
        }
        Ok(map)
    }
}
impl<T, S> Encode for HashSet<T, S>
where
    T: Encode,
{
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        crate::enc::encode_slice_len(encoder, self.len())?;
        for item in self.iter() {
            item.encode(encoder)?;
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests_rug_45 {
    use super::*;
    use std::io::Cursor;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_45_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let mut p0: Cursor<Vec<u8>> = Cursor::new(vec![rug_fuzz_0, 2, 3]);
        crate::features::impl_std::IoReader::<Cursor<Vec<u8>>>::new(p0);
        let _rug_ed_tests_rug_45_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_48 {
    use super::*;
    use crate::de::read::Reader;
    use std::io::BufReader;
    use std::fs::File;
    #[test]
    fn test_peek_read() {
        let _rug_st_tests_rug_48_rrrruuuugggg_test_peek_read = 0;
        let rug_fuzz_0 = "path/to/your_file";
        let rug_fuzz_1 = 10;
        let file = File::open(rug_fuzz_0).unwrap();
        let mut p0: BufReader<File> = BufReader::new(file);
        let p1: usize = rug_fuzz_1;
        <std::io::BufReader<File>>::peek_read(&mut p0, p1);
        let _rug_ed_tests_rug_48_rrrruuuugggg_test_peek_read = 0;
    }
}
#[cfg(test)]
mod tests_rug_49 {
    use super::*;
    use crate::de::read::Reader;
    use std::fs::File;
    use std::io::BufReader;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_49_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "path/to/your_file";
        let rug_fuzz_1 = 10;
        let file = File::open(rug_fuzz_0).unwrap();
        let mut p0: BufReader<File> = BufReader::new(file);
        let p1: usize = rug_fuzz_1;
        <std::io::BufReader<File> as std::io::BufRead>::consume(&mut p0, p1);
        let _rug_ed_tests_rug_49_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_50 {
    use super::*;
    use crate::features::impl_std::IoWriter;
    use std::io::Cursor;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_50_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b'A';
        let rug_fuzz_1 = b'B';
        let rug_fuzz_2 = b'C';
        let mut buf: Vec<u8> = Vec::new();
        buf.push(rug_fuzz_0);
        buf.push(rug_fuzz_1);
        buf.push(rug_fuzz_2);
        let mut p0 = Cursor::new(&mut buf[..]);
        IoWriter::<Cursor<&mut [u8]>>::new(&mut p0);
        let _rug_ed_tests_rug_50_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_53 {
    use super::*;
    use crate::Encode;
    use crate::enc::EncoderImpl;
    use crate::features::IoWriter;
    use crate::config::{Configuration, BigEndian, Fixint, Limit};
    use std::vec::Vec;
    use std::ffi::CStr;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_53_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"Hello, world!\0";
        let rug_fuzz_1 = "Failed to create CStr";
        let v48 = CStr::from_bytes_with_nul(rug_fuzz_0).expect(rug_fuzz_1);
        let mut writer: Vec<u8> = Vec::new();
        let mut v16: IoWriter<Vec<u8>> = IoWriter::new(&mut writer);
        let config = Configuration::<BigEndian, Fixint, Limit<100>>::default();
        let mut v19: EncoderImpl<
            IoWriter<Vec<u8>>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = EncoderImpl::new(v16, config);
        v48.encode(&mut v19);
        let _rug_ed_tests_rug_53_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_62 {
    use super::*;
    use crate::Encode;
    use crate::enc::EncoderImpl;
    use crate::features::IoWriter;
    use crate::config::{Configuration, BigEndian, Fixint, Limit};
    use std::time::SystemTime;
    use std::vec::Vec;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_62_rrrruuuugggg_test_rug = 0;
        let mut p0: SystemTime = SystemTime::now();
        let mut writer: Vec<u8> = Vec::new();
        let mut p1: EncoderImpl<
            IoWriter<Vec<u8>>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = {
            let mut v16: IoWriter<Vec<u8>> = IoWriter::new(&mut writer);
            let config = Configuration::<BigEndian, Fixint, Limit<100>>::default();
            EncoderImpl::new(v16, config)
        };
        p0.encode(&mut p1);
        let _rug_ed_tests_rug_62_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_63 {
    use super::*;
    use crate::de::DecoderImpl;
    use crate::de::read::SliceReader;
    use crate::config::{Configuration, BigEndian, Fixint, Limit};
    use std::time::{SystemTime, Duration};
    use crate::Decode;
    #[test]
    fn test_decode() {
        let mut p0: DecoderImpl<
            SliceReader<'_>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = DecoderImpl::new(
            SliceReader::new(&[0u8, 1u8, 2u8]),
            Configuration::<BigEndian, Fixint, Limit<100>>::default(),
        );
        <std::time::SystemTime>::decode(&mut p0);
    }
}
#[cfg(test)]
mod tests_rug_68 {
    use super::*;
    use crate::{
        enc::{Encoder, EncoderImpl},
        features::IoWriter, config::{Configuration, BigEndian, Fixint, Limit},
    };
    use std::vec::Vec;
    use std::net::IpAddr;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_68_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "192.0.2.1";
        let mut p0: IpAddr = rug_fuzz_0.parse().unwrap();
        let mut writer: Vec<u8> = Vec::new();
        let mut p1: EncoderImpl<
            IoWriter<Vec<u8>>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = {
            let v16: IoWriter<Vec<u8>> = IoWriter::new(&mut writer);
            let config = Configuration::<BigEndian, Fixint, Limit<100>>::default();
            EncoderImpl::new(v16, config)
        };
        p0.encode(&mut p1);
        let _rug_ed_tests_rug_68_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_69 {
    use super::*;
    use crate::Decode;
    use crate::de::DecoderImpl;
    use crate::de::read::SliceReader;
    use crate::config::{Configuration, BigEndian, Fixint, Limit};
    #[test]
    fn test_decode_ipaddr() {
        let mut p0: DecoderImpl<
            SliceReader<'_>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = DecoderImpl::new(
            SliceReader::new(&[0u8]),
            Configuration::<BigEndian, Fixint, Limit<100>>::default(),
        );
        <std::net::IpAddr>::decode(&mut p0);
    }
}
#[cfg(test)]
mod tests_rug_70 {
    use super::*;
    use crate::enc::{Encode, Encoder};
    use crate::enc::EncoderImpl;
    use crate::features::IoWriter;
    use crate::config::{Configuration, BigEndian, Fixint, Limit};
    use std::net::Ipv4Addr;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_70_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 127;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 1;
        let mut p0: std::net::Ipv4Addr = Ipv4Addr::new(
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
        );
        let mut writer: Vec<u8> = Vec::new();
        let mut p1: EncoderImpl<
            IoWriter<Vec<u8>>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = {
            let mut writer: IoWriter<Vec<u8>> = IoWriter::new(&mut writer);
            let config = Configuration::<BigEndian, Fixint, Limit<100>>::default();
            EncoderImpl::new(writer, config)
        };
        <std::net::Ipv4Addr>::encode(&p0, &mut p1).unwrap();
        let _rug_ed_tests_rug_70_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_71 {
    use super::*;
    use crate::de::Decode;
    use crate::de::DecoderImpl;
    use crate::de::read::SliceReader;
    use crate::config::{Configuration, BigEndian, Fixint, Limit};
    #[test]
    fn test_rug() {
        let mut p0: DecoderImpl<
            SliceReader<'_>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = DecoderImpl::new(
            SliceReader::new(&[0u8, 1u8, 2u8]),
            Configuration::<BigEndian, Fixint, Limit<100>>::default(),
        );
        <std::net::Ipv4Addr>::decode(&mut p0);
    }
}
#[cfg(test)]
mod tests_rug_73 {
    use super::*;
    use crate::Decode;
    use crate::de::DecoderImpl;
    use crate::de::read::SliceReader;
    use crate::config::{Configuration, BigEndian, Fixint, Limit};
    #[test]
    fn test_decode_ipv6_addr() {
        let mut p0: DecoderImpl<
            SliceReader<'_>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = DecoderImpl::new(
            SliceReader::new(&[0u8; 16]),
            Configuration::<BigEndian, Fixint, Limit<100>>::default(),
        );
        <std::net::Ipv6Addr>::decode(&mut p0).unwrap();
    }
}
#[cfg(test)]
mod tests_rug_74 {
    use super::*;
    use crate::enc::Encoder;
    use crate::enc::EncoderImpl;
    use crate::config::{Configuration, BigEndian, Fixint, Limit};
    use crate::features::IoWriter;
    use std::vec::Vec;
    use std::net::{SocketAddr, IpAddr, Ipv4Addr};
    #[test]
    fn test_encode() {
        let _rug_st_tests_rug_74_rrrruuuugggg_test_encode = 0;
        let rug_fuzz_0 = 127;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = 8080;
        let mut p0: SocketAddr = SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3)),
            rug_fuzz_4,
        );
        let mut writer: Vec<u8> = Vec::new();
        let mut p1: EncoderImpl<
            IoWriter<Vec<u8>>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = {
            let writer = IoWriter::new(&mut writer);
            let config = Configuration::<BigEndian, Fixint, Limit<100>>::default();
            EncoderImpl::<
                IoWriter<Vec<u8>>,
                Configuration<BigEndian, Fixint, Limit<100>>,
            >::new(writer, config)
        };
        p0.encode(&mut p1);
        let _rug_ed_tests_rug_74_rrrruuuugggg_test_encode = 0;
    }
}
#[cfg(test)]
mod tests_rug_75 {
    use super::*;
    use crate::{Decode, de::DecoderImpl, de::read::SliceReader};
    use crate::config::{Configuration, BigEndian, Fixint, Limit};
    #[test]
    fn test_rug() {
        let mut p0: DecoderImpl<
            SliceReader<'_>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = DecoderImpl::new(
            SliceReader::new(&[0u8, 1u8, 2u8]),
            Configuration::<BigEndian, Fixint, Limit<100>>::default(),
        );
        <std::net::SocketAddr>::decode(&mut p0);
    }
}
#[cfg(test)]
mod tests_rug_79 {
    use super::*;
    use crate::Decode;
    use crate::de::DecoderImpl;
    use crate::de::read::SliceReader;
    use crate::config::{Configuration, BigEndian, Fixint, Limit};
    #[test]
    fn test_rug() {
        let mut p0: DecoderImpl<
            SliceReader<'_>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = DecoderImpl::new(
            SliceReader::new(&[0u8, 1u8, 2u8]),
            Configuration::<BigEndian, Fixint, Limit<100>>::default(),
        );
        <std::net::SocketAddrV6>::decode(&mut p0);
    }
}
#[cfg(test)]
mod tests_rug_80 {
    use super::*;
    use crate::std::error::Error;
    use crate::error::EncodeError;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_80_rrrruuuugggg_test_rug = 0;
        let mut p0 = EncodeError::UnexpectedEnd;
        <EncodeError as std::error::Error>::source(&p0);
        let _rug_ed_tests_rug_80_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_81 {
    use super::*;
    use crate::std::error::Error;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_81_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        use crate::error::DecodeError;
        let mut v69: DecodeError = DecodeError::UnexpectedEnd {
            additional: rug_fuzz_0,
        };
        let mut p0: &DecodeError = &v69;
        <DecodeError as std::error::Error>::source(p0);
        let _rug_ed_tests_rug_81_rrrruuuugggg_test_rug = 0;
    }
}
