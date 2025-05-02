use super::{write::Writer, Encode, Encoder};
use crate::{
    config::{Endian, IntEncoding, InternalEndianConfig, InternalIntEncodingConfig},
    error::EncodeError,
};
use core::{
    cell::{Cell, RefCell},
    marker::PhantomData,
    num::{
        NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroIsize,
        NonZeroU128, NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8, NonZeroUsize,
    },
    ops::{Bound, Range, RangeInclusive},
    time::Duration,
};
impl Encode for () {
    fn encode<E: Encoder>(&self, _: &mut E) -> Result<(), EncodeError> {
        Ok(())
    }
}
impl<T> Encode for PhantomData<T> {
    fn encode<E: Encoder>(&self, _: &mut E) -> Result<(), EncodeError> {
        Ok(())
    }
}
impl Encode for bool {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        u8::from(*self).encode(encoder)
    }
}
impl Encode for u8 {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        encoder.writer().write(&[*self])
    }
}
impl Encode for NonZeroU8 {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        self.get().encode(encoder)
    }
}
impl Encode for u16 {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        match E::C::INT_ENCODING {
            IntEncoding::Variable => {
                crate::varint::varint_encode_u16(encoder.writer(), E::C::ENDIAN, *self)
            }
            IntEncoding::Fixed => {
                match E::C::ENDIAN {
                    Endian::Big => encoder.writer().write(&self.to_be_bytes()),
                    Endian::Little => encoder.writer().write(&self.to_le_bytes()),
                }
            }
        }
    }
}
impl Encode for NonZeroU16 {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        self.get().encode(encoder)
    }
}
impl Encode for u32 {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        match E::C::INT_ENCODING {
            IntEncoding::Variable => {
                crate::varint::varint_encode_u32(encoder.writer(), E::C::ENDIAN, *self)
            }
            IntEncoding::Fixed => {
                match E::C::ENDIAN {
                    Endian::Big => encoder.writer().write(&self.to_be_bytes()),
                    Endian::Little => encoder.writer().write(&self.to_le_bytes()),
                }
            }
        }
    }
}
impl Encode for NonZeroU32 {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        self.get().encode(encoder)
    }
}
impl Encode for u64 {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        match E::C::INT_ENCODING {
            IntEncoding::Variable => {
                crate::varint::varint_encode_u64(encoder.writer(), E::C::ENDIAN, *self)
            }
            IntEncoding::Fixed => {
                match E::C::ENDIAN {
                    Endian::Big => encoder.writer().write(&self.to_be_bytes()),
                    Endian::Little => encoder.writer().write(&self.to_le_bytes()),
                }
            }
        }
    }
}
impl Encode for NonZeroU64 {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        self.get().encode(encoder)
    }
}
impl Encode for u128 {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        match E::C::INT_ENCODING {
            IntEncoding::Variable => {
                crate::varint::varint_encode_u128(encoder.writer(), E::C::ENDIAN, *self)
            }
            IntEncoding::Fixed => {
                match E::C::ENDIAN {
                    Endian::Big => encoder.writer().write(&self.to_be_bytes()),
                    Endian::Little => encoder.writer().write(&self.to_le_bytes()),
                }
            }
        }
    }
}
impl Encode for NonZeroU128 {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        self.get().encode(encoder)
    }
}
impl Encode for usize {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        match E::C::INT_ENCODING {
            IntEncoding::Variable => {
                crate::varint::varint_encode_usize(encoder.writer(), E::C::ENDIAN, *self)
            }
            IntEncoding::Fixed => {
                match E::C::ENDIAN {
                    Endian::Big => encoder.writer().write(&(*self as u64).to_be_bytes()),
                    Endian::Little => {
                        encoder.writer().write(&(*self as u64).to_le_bytes())
                    }
                }
            }
        }
    }
}
impl Encode for NonZeroUsize {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        self.get().encode(encoder)
    }
}
impl Encode for i8 {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        encoder.writer().write(&[*self as u8])
    }
}
impl Encode for NonZeroI8 {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        self.get().encode(encoder)
    }
}
impl Encode for i16 {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        match E::C::INT_ENCODING {
            IntEncoding::Variable => {
                crate::varint::varint_encode_i16(encoder.writer(), E::C::ENDIAN, *self)
            }
            IntEncoding::Fixed => {
                match E::C::ENDIAN {
                    Endian::Big => encoder.writer().write(&self.to_be_bytes()),
                    Endian::Little => encoder.writer().write(&self.to_le_bytes()),
                }
            }
        }
    }
}
impl Encode for NonZeroI16 {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        self.get().encode(encoder)
    }
}
impl Encode for i32 {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        match E::C::INT_ENCODING {
            IntEncoding::Variable => {
                crate::varint::varint_encode_i32(encoder.writer(), E::C::ENDIAN, *self)
            }
            IntEncoding::Fixed => {
                match E::C::ENDIAN {
                    Endian::Big => encoder.writer().write(&self.to_be_bytes()),
                    Endian::Little => encoder.writer().write(&self.to_le_bytes()),
                }
            }
        }
    }
}
impl Encode for NonZeroI32 {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        self.get().encode(encoder)
    }
}
impl Encode for i64 {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        match E::C::INT_ENCODING {
            IntEncoding::Variable => {
                crate::varint::varint_encode_i64(encoder.writer(), E::C::ENDIAN, *self)
            }
            IntEncoding::Fixed => {
                match E::C::ENDIAN {
                    Endian::Big => encoder.writer().write(&self.to_be_bytes()),
                    Endian::Little => encoder.writer().write(&self.to_le_bytes()),
                }
            }
        }
    }
}
impl Encode for NonZeroI64 {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        self.get().encode(encoder)
    }
}
impl Encode for i128 {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        match E::C::INT_ENCODING {
            IntEncoding::Variable => {
                crate::varint::varint_encode_i128(encoder.writer(), E::C::ENDIAN, *self)
            }
            IntEncoding::Fixed => {
                match E::C::ENDIAN {
                    Endian::Big => encoder.writer().write(&self.to_be_bytes()),
                    Endian::Little => encoder.writer().write(&self.to_le_bytes()),
                }
            }
        }
    }
}
impl Encode for NonZeroI128 {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        self.get().encode(encoder)
    }
}
impl Encode for isize {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        match E::C::INT_ENCODING {
            IntEncoding::Variable => {
                crate::varint::varint_encode_isize(encoder.writer(), E::C::ENDIAN, *self)
            }
            IntEncoding::Fixed => {
                match E::C::ENDIAN {
                    Endian::Big => encoder.writer().write(&(*self as i64).to_be_bytes()),
                    Endian::Little => {
                        encoder.writer().write(&(*self as i64).to_le_bytes())
                    }
                }
            }
        }
    }
}
impl Encode for NonZeroIsize {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        self.get().encode(encoder)
    }
}
impl Encode for f32 {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        match E::C::ENDIAN {
            Endian::Big => encoder.writer().write(&self.to_be_bytes()),
            Endian::Little => encoder.writer().write(&self.to_le_bytes()),
        }
    }
}
impl Encode for f64 {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        match E::C::ENDIAN {
            Endian::Big => encoder.writer().write(&self.to_be_bytes()),
            Endian::Little => encoder.writer().write(&self.to_le_bytes()),
        }
    }
}
impl Encode for char {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        encode_utf8(encoder.writer(), *self)
    }
}
impl<T> Encode for [T]
where
    T: Encode + 'static,
{
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        super::encode_slice_len(encoder, self.len())?;
        if core::any::TypeId::of::<T>() == core::any::TypeId::of::<u8>() {
            let t: &[u8] = unsafe { core::mem::transmute(self) };
            encoder.writer().write(t)?;
            return Ok(());
        }
        for item in self {
            item.encode(encoder)?;
        }
        Ok(())
    }
}
const TAG_CONT: u8 = 0b1000_0000;
const TAG_TWO_B: u8 = 0b1100_0000;
const TAG_THREE_B: u8 = 0b1110_0000;
const TAG_FOUR_B: u8 = 0b1111_0000;
const MAX_ONE_B: u32 = 0x80;
const MAX_TWO_B: u32 = 0x800;
const MAX_THREE_B: u32 = 0x10000;
fn encode_utf8(writer: &mut impl Writer, c: char) -> Result<(), EncodeError> {
    let code = c as u32;
    if code < MAX_ONE_B {
        writer.write(&[c as u8])
    } else if code < MAX_TWO_B {
        let mut buf = [0u8; 2];
        buf[0] = (code >> 6 & 0x1F) as u8 | TAG_TWO_B;
        buf[1] = (code & 0x3F) as u8 | TAG_CONT;
        writer.write(&buf)
    } else if code < MAX_THREE_B {
        let mut buf = [0u8; 3];
        buf[0] = (code >> 12 & 0x0F) as u8 | TAG_THREE_B;
        buf[1] = (code >> 6 & 0x3F) as u8 | TAG_CONT;
        buf[2] = (code & 0x3F) as u8 | TAG_CONT;
        writer.write(&buf)
    } else {
        let mut buf = [0u8; 4];
        buf[0] = (code >> 18 & 0x07) as u8 | TAG_FOUR_B;
        buf[1] = (code >> 12 & 0x3F) as u8 | TAG_CONT;
        buf[2] = (code >> 6 & 0x3F) as u8 | TAG_CONT;
        buf[3] = (code & 0x3F) as u8 | TAG_CONT;
        writer.write(&buf)
    }
}
impl Encode for str {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        self.as_bytes().encode(encoder)
    }
}
impl<T, const N: usize> Encode for [T; N]
where
    T: Encode,
{
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        for item in self.iter() {
            item.encode(encoder)?;
        }
        Ok(())
    }
}
impl<T> Encode for Option<T>
where
    T: Encode,
{
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        super::encode_option_variant(encoder, self)?;
        if let Some(val) = self {
            val.encode(encoder)?;
        }
        Ok(())
    }
}
impl<T, U> Encode for Result<T, U>
where
    T: Encode,
    U: Encode,
{
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        match self {
            Ok(val) => {
                0u32.encode(encoder)?;
                val.encode(encoder)
            }
            Err(err) => {
                1u32.encode(encoder)?;
                err.encode(encoder)
            }
        }
    }
}
impl<T> Encode for Cell<T>
where
    T: Encode + Copy,
{
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        T::encode(&self.get(), encoder)
    }
}
impl<T> Encode for RefCell<T>
where
    T: Encode + ?Sized,
{
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        let borrow_guard = self
            .try_borrow()
            .map_err(|e| EncodeError::RefCellAlreadyBorrowed {
                inner: e,
                type_name: core::any::type_name::<RefCell<T>>(),
            })?;
        T::encode(&borrow_guard, encoder)
    }
}
impl Encode for Duration {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        self.as_secs().encode(encoder)?;
        self.subsec_nanos().encode(encoder)?;
        Ok(())
    }
}
impl<T> Encode for Range<T>
where
    T: Encode,
{
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        self.start.encode(encoder)?;
        self.end.encode(encoder)?;
        Ok(())
    }
}
impl<T> Encode for RangeInclusive<T>
where
    T: Encode,
{
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        self.start().encode(encoder)?;
        self.end().encode(encoder)?;
        Ok(())
    }
}
impl<T> Encode for Bound<T>
where
    T: Encode,
{
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        match self {
            Self::Unbounded => {
                0u32.encode(encoder)?;
            }
            Self::Included(val) => {
                1u32.encode(encoder)?;
                val.encode(encoder)?;
            }
            Self::Excluded(val) => {
                2u32.encode(encoder)?;
                val.encode(encoder)?;
            }
        }
        Ok(())
    }
}
impl<'a, T> Encode for &'a T
where
    T: Encode + ?Sized,
{
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        T::encode(self, encoder)
    }
}
#[cfg(test)]
mod tests_rug_240 {
    use super::*;
    use crate::enc::impls::{
        Writer, encode_utf8, EncodeError, TAG_TWO_B, TAG_CONT, TAG_THREE_B, TAG_FOUR_B,
    };
    use crate::features::IoWriter;
    use std::vec::Vec;
    #[test]
    fn test_encode_utf8() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(char) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Vec<u8> = Vec::new();
        let mut p1: char = rug_fuzz_0;
        let mut writer: IoWriter<Vec<u8>> = IoWriter::new(&mut p0);
        encode_utf8(&mut writer, p1).unwrap();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_241 {
    use super::*;
    use crate::enc::EncoderImpl;
    use crate::features::IoWriter;
    use crate::config::{Configuration, BigEndian, Fixint, Limit};
    use std::vec::Vec;
    #[test]
    fn test_encode() {
        let _rug_st_tests_rug_241_rrrruuuugggg_test_encode = 0;
        let mut writer: Vec<u8> = Vec::new();
        let mut v16: IoWriter<Vec<u8>> = IoWriter::new(&mut writer);
        let config = Configuration::<BigEndian, Fixint, Limit<100>>::default();
        let mut v19: EncoderImpl<
            IoWriter<Vec<u8>>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = EncoderImpl::new(v16, config);
        let p0: () = ();
        let p1 = &mut v19;
        p0.encode(p1).unwrap();
        let _rug_ed_tests_rug_241_rrrruuuugggg_test_encode = 0;
    }
}
#[cfg(test)]
mod tests_rug_242 {
    use super::*;
    use crate::enc::{Encode, Encoder, EncodeError};
    use crate::enc::EncoderImpl;
    use crate::features::IoWriter;
    use crate::config::{Configuration, BigEndian, Fixint, Limit};
    use std::marker::PhantomData;
    use std::vec::Vec;
    #[test]
    fn test_encode() {
        let _rug_st_tests_rug_242_rrrruuuugggg_test_encode = 0;
        let mut writer: Vec<u8> = Vec::new();
        let mut v16: IoWriter<Vec<u8>> = IoWriter::new(&mut writer);
        let config = Configuration::<BigEndian, Fixint, Limit<100>>::default();
        let mut v19: EncoderImpl<
            IoWriter<Vec<u8>>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = EncoderImpl::new(v16, config);
        let p0: PhantomData<u32> = PhantomData;
        let p1: &mut EncoderImpl<
            IoWriter<Vec<u8>>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = &mut v19;
        p0.encode(p1);
        let _rug_ed_tests_rug_242_rrrruuuugggg_test_encode = 0;
    }
}
#[cfg(test)]
mod tests_rug_243 {
    use super::*;
    use crate::enc::Encoder;
    use crate::enc::EncodeError;
    use crate::enc::EncoderImpl;
    use crate::features::IoWriter;
    use crate::config::{Configuration, BigEndian, Fixint, Limit};
    use std::vec::Vec;
    #[test]
    fn test_encode() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: bool = rug_fuzz_0;
        let mut writer: Vec<u8> = Vec::new();
        let mut v16: IoWriter<Vec<u8>> = IoWriter::new(&mut writer);
        let config = Configuration::<BigEndian, Fixint, Limit<100>>::default();
        let mut v19: EncoderImpl<
            IoWriter<Vec<u8>>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = EncoderImpl::new(v16, config);
        p0.encode(&mut v19).unwrap();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_244 {
    use super::*;
    use crate::Encode;
    use crate::enc::{Encoder, EncodeError};
    use crate::enc::EncoderImpl;
    use crate::features::IoWriter;
    use crate::config::{Configuration, BigEndian, Fixint, Limit};
    use std::vec::Vec;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: u8 = rug_fuzz_0;
        let mut writer: Vec<u8> = Vec::new();
        let mut v16: IoWriter<Vec<u8>> = IoWriter::new(&mut writer);
        let config = Configuration::<BigEndian, Fixint, Limit<100>>::default();
        let mut p1: EncoderImpl<
            IoWriter<Vec<u8>>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = EncoderImpl::new(v16, config);
        <u8>::encode(&p0, &mut p1).unwrap();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_245 {
    use super::*;
    use crate::Encode;
    use crate::enc::encoder::EncoderImpl;
    use crate::features::IoWriter;
    use crate::config::{Configuration, BigEndian, Fixint, Limit};
    use std::vec::Vec;
    use std::num::NonZeroU8;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: NonZeroU8 = NonZeroU8::new(rug_fuzz_0).unwrap();
        let mut writer: Vec<u8> = Vec::new();
        let mut v16: IoWriter<Vec<u8>> = IoWriter::new(&mut writer);
        let config = Configuration::<BigEndian, Fixint, Limit<100>>::default();
        let mut p1: EncoderImpl<
            IoWriter<Vec<u8>>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = EncoderImpl::new(v16, config);
        p0.encode(&mut p1).unwrap();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_247 {
    use super::*;
    use crate::enc::{Encode, EncodeError};
    use crate::enc::EncoderImpl;
    use crate::features::IoWriter;
    use crate::config::{Configuration, BigEndian, Fixint, Limit};
    use std::num::NonZeroU16;
    use std::vec::Vec;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = NonZeroU16::new(rug_fuzz_0).unwrap();
        let mut writer: Vec<u8> = Vec::new();
        let mut p1: IoWriter<Vec<u8>> = IoWriter::new(&mut writer);
        let config = Configuration::<BigEndian, Fixint, Limit<100>>::default();
        let mut encoder_impl: EncoderImpl<
            IoWriter<Vec<u8>>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = EncoderImpl::new(p1, config);
        p0.encode(&mut encoder_impl).unwrap();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_248 {
    use super::*;
    use crate::enc::EncoderImpl;
    use crate::features::IoWriter;
    use crate::config::{Configuration, BigEndian, Fixint, Limit};
    use std::vec::Vec;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: u32 = rug_fuzz_0;
        let mut writer: Vec<u8> = Vec::new();
        let mut v16: IoWriter<Vec<u8>> = IoWriter::new(&mut writer);
        let config = Configuration::<BigEndian, Fixint, Limit<100>>::default();
        let mut p1: EncoderImpl<
            IoWriter<Vec<u8>>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = EncoderImpl::new(v16, config);
        <u32>::encode(&p0, &mut p1).unwrap();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_249 {
    use super::*;
    use crate::enc::Encoder;
    use crate::Encode;
    use crate::enc::EncoderImpl;
    use crate::features::IoWriter;
    use crate::config::{Configuration, BigEndian, Fixint, Limit};
    use std::vec::Vec;
    use std::num::NonZeroU32;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: NonZeroU32 = NonZeroU32::new(rug_fuzz_0).unwrap();
        let mut writer: Vec<u8> = Vec::new();
        let mut v16: IoWriter<Vec<u8>> = IoWriter::new(&mut writer);
        let config = Configuration::<BigEndian, Fixint, Limit<100>>::default();
        let mut p1: EncoderImpl<
            IoWriter<Vec<u8>>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = EncoderImpl::new(v16, config);
        <std::num::NonZeroU32>::encode(&mut p0, &mut p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_250 {
    use super::*;
    use crate::enc::EncoderImpl;
    use crate::features::IoWriter;
    use crate::config::{Configuration, BigEndian, Fixint, Limit};
    use crate::Encode;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: u64 = rug_fuzz_0;
        let mut writer: Vec<u8> = Vec::new();
        let mut v16: IoWriter<Vec<u8>> = IoWriter::new(&mut writer);
        let config = Configuration::<BigEndian, Fixint, Limit<100>>::default();
        let mut p1: EncoderImpl<
            IoWriter<Vec<u8>>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = EncoderImpl::new(v16, config);
        p0.encode(&mut p1).unwrap();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_251 {
    use super::*;
    use crate::Encode;
    use crate::enc::EncoderImpl;
    use crate::features::IoWriter;
    use crate::config::{Configuration, BigEndian, Fixint, Limit};
    use std::vec::Vec;
    use std::num::NonZeroU64;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = NonZeroU64::new(rug_fuzz_0).unwrap();
        let mut writer: Vec<u8> = Vec::new();
        let mut v16: IoWriter<Vec<u8>> = IoWriter::new(&mut writer);
        let config = Configuration::<BigEndian, Fixint, Limit<100>>::default();
        let mut p1: EncoderImpl<
            IoWriter<Vec<u8>>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = EncoderImpl::new(v16, config);
        <std::num::NonZeroU64>::encode(&p0, &mut p1).unwrap();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_252 {
    use super::*;
    use crate::enc::EncoderImpl;
    use crate::features::IoWriter;
    use crate::config::{Configuration, BigEndian, Fixint, Limit};
    use std::vec::Vec;
    use crate::Encode;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u128) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: u128 = rug_fuzz_0;
        let mut writer: Vec<u8> = Vec::new();
        let mut v16: IoWriter<Vec<u8>> = IoWriter::new(&mut writer);
        let config = Configuration::<BigEndian, Fixint, Limit<100>>::default();
        let mut p1: EncoderImpl<
            IoWriter<Vec<u8>>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = EncoderImpl::new(v16, config);
        p0.encode(&mut p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_253 {
    use super::*;
    use crate::Encode;
    use crate::enc::EncoderImpl;
    use crate::features::IoWriter;
    use crate::config::{Configuration, BigEndian, Fixint, Limit};
    use std::num::NonZeroU128;
    use std::vec::Vec;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u128) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = NonZeroU128::new(rug_fuzz_0).unwrap();
        let mut writer: Vec<u8> = Vec::new();
        let mut p1: IoWriter<Vec<u8>> = IoWriter::new(&mut writer);
        let config = Configuration::<BigEndian, Fixint, Limit<100>>::default();
        let mut encoder: EncoderImpl<
            IoWriter<Vec<u8>>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = EncoderImpl::new(p1, config);
        <std::num::NonZeroU128>::encode(&p0, &mut encoder).unwrap();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_254 {
    use super::*;
    use crate::enc::EncoderImpl;
    use crate::features::IoWriter;
    use crate::config::{Configuration, BigEndian, Fixint, Limit};
    use std::vec::Vec;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut writer: Vec<u8> = Vec::new();
        let mut v16: IoWriter<Vec<u8>> = IoWriter::new(&mut writer);
        let config = Configuration::<BigEndian, Fixint, Limit<100>>::default();
        let mut v19: EncoderImpl<
            IoWriter<Vec<u8>>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = EncoderImpl::new(v16, config);
        let p0: usize = rug_fuzz_0;
        let p1: &mut EncoderImpl<
            IoWriter<Vec<u8>>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = &mut v19;
        p0.encode(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_256 {
    use super::*;
    use crate::enc::{Encoder, EncodeError};
    use crate::enc::EncoderImpl;
    use crate::features::IoWriter;
    use crate::config::{Configuration, BigEndian, Fixint, Limit};
    use std::vec::Vec;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut writer: Vec<u8> = Vec::new();
        let mut v16: IoWriter<Vec<u8>> = IoWriter::new(&mut writer);
        let config = Configuration::<BigEndian, Fixint, Limit<100>>::default();
        let mut v19: EncoderImpl<
            IoWriter<Vec<u8>>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = EncoderImpl::new(v16, config);
        let p0: i8 = rug_fuzz_0;
        let p1: &mut EncoderImpl<
            IoWriter<Vec<u8>>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = &mut v19;
        p0.encode(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_257 {
    use super::*;
    use crate::enc::EncoderImpl;
    use crate::features::IoWriter;
    use crate::config::{Configuration, BigEndian, Fixint, Limit};
    use std::num::NonZeroI8;
    #[test]
    fn test_encode() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i8, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = NonZeroI8::new(rug_fuzz_0).expect(rug_fuzz_1);
        let mut writer: Vec<u8> = Vec::new();
        let mut p1: IoWriter<Vec<u8>> = IoWriter::new(&mut writer);
        let config = Configuration::<BigEndian, Fixint, Limit<100>>::default();
        let mut encoder: EncoderImpl<
            IoWriter<Vec<u8>>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = EncoderImpl::new(p1, config);
        p0.encode(&mut encoder);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_258 {
    use super::*;
    use crate::enc::EncoderImpl;
    use crate::enc::Encoder;
    use crate::features::IoWriter;
    use crate::config::{Configuration, BigEndian, Fixint, Limit};
    use std::vec::Vec;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: i16 = -rug_fuzz_0;
        let mut writer: Vec<u8> = Vec::new();
        let mut v16: IoWriter<Vec<u8>> = IoWriter::new(&mut writer);
        let config = Configuration::<BigEndian, Fixint, Limit<100>>::default();
        let mut p1: EncoderImpl<
            IoWriter<Vec<u8>>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = EncoderImpl::new(v16, config);
        p0.encode(&mut p1).unwrap();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_259 {
    use super::*;
    use crate::Encode;
    use std::num::NonZeroI16;
    use crate::enc::EncoderImpl;
    use crate::features::IoWriter;
    use crate::config::{Configuration, BigEndian, Fixint, Limit};
    use std::vec::Vec;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut v107: NonZeroI16 = NonZeroI16::new(rug_fuzz_0).unwrap();
        let mut writer: Vec<u8> = Vec::new();
        let mut v16: IoWriter<Vec<u8>> = IoWriter::new(&mut writer);
        let config = Configuration::<BigEndian, Fixint, Limit<100>>::default();
        let mut v19: EncoderImpl<
            IoWriter<Vec<u8>>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = EncoderImpl::new(v16, config);
        <std::num::NonZeroI16>::encode(&mut v107, &mut v19);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_261 {
    use super::*;
    use crate::{
        Encode, enc::{Encoder, EncodeError},
        enc::EncoderImpl, features::IoWriter,
        config::{Configuration, BigEndian, Fixint, Limit},
    };
    use std::num::NonZeroI32;
    use std::vec::Vec;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = NonZeroI32::new(rug_fuzz_0).unwrap();
        let mut writer: Vec<u8> = Vec::new();
        let mut v16: IoWriter<Vec<u8>> = IoWriter::new(&mut writer);
        let config = Configuration::<BigEndian, Fixint, Limit<100>>::default();
        let mut p1: EncoderImpl<
            IoWriter<Vec<u8>>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = EncoderImpl::new(v16, config);
        <std::num::NonZeroI32>::encode(&p0, &mut p1).unwrap();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_263 {
    use super::*;
    use crate::enc::{Encode, EncodeError};
    use crate::enc::Encoder;
    use std::num::NonZeroI64;
    use crate::enc::encoder::EncoderImpl;
    use crate::features::IoWriter;
    use crate::config::{Configuration, BigEndian, Fixint, Limit};
    use std::vec::Vec;
    #[test]
    fn test_encode() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i64, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = NonZeroI64::new(rug_fuzz_0).unwrap();
        let mut writer: Vec<u8> = Vec::new();
        let mut v16: IoWriter<Vec<u8>> = IoWriter::new(&mut writer);
        let config = Configuration::<BigEndian, Fixint, Limit<100>>::default();
        let mut p1: EncoderImpl<
            IoWriter<Vec<u8>>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = EncoderImpl::new(v16, config);
        <std::num::NonZeroI64 as Encode>::encode(&p0, &mut p1).unwrap();
        debug_assert_eq!(writer[rug_fuzz_1], 42);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_264 {
    use super::*;
    use crate::enc::EncoderImpl;
    use crate::features::IoWriter;
    use crate::config::{Configuration, BigEndian, Fixint, Limit};
    use crate::enc::{Encode, EncodeError};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i128) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: i128 = rug_fuzz_0;
        let mut writer: Vec<u8> = Vec::new();
        let mut v16: IoWriter<Vec<u8>> = IoWriter::new(&mut writer);
        let config = Configuration::<BigEndian, Fixint, Limit<100>>::default();
        let mut p1: EncoderImpl<
            IoWriter<Vec<u8>>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = EncoderImpl::new(v16, config);
        p0.encode(&mut p1).unwrap();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_265 {
    use super::*;
    use crate::enc::{Encode, Encoder};
    use crate::enc::encoder::EncoderImpl;
    use crate::features::IoWriter;
    use crate::config::{Configuration, BigEndian, Fixint, Limit};
    use std::vec::Vec;
    use std::num::NonZeroI128;
    #[test]
    fn test_encode() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i128) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = NonZeroI128::new(rug_fuzz_0).unwrap();
        let mut writer: Vec<u8> = Vec::new();
        let mut v16: IoWriter<Vec<u8>> = IoWriter::new(&mut writer);
        let config = Configuration::<BigEndian, Fixint, Limit<100>>::default();
        let mut p1: EncoderImpl<
            IoWriter<Vec<u8>>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = EncoderImpl::new(v16, config);
        <std::num::NonZeroI128>::encode(&p0, &mut p1).unwrap();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_267 {
    use super::*;
    use crate::Encode;
    use crate::enc::EncoderImpl;
    use crate::features::IoWriter;
    use crate::config::{Configuration, BigEndian, Fixint, Limit};
    use std::num::NonZeroIsize;
    use std::vec::Vec;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(isize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = NonZeroIsize::new(rug_fuzz_0).unwrap();
        let mut writer: Vec<u8> = Vec::new();
        let mut v16: IoWriter<Vec<u8>> = IoWriter::new(&mut writer);
        let config = Configuration::<BigEndian, Fixint, Limit<100>>::default();
        let mut p1: EncoderImpl<
            IoWriter<Vec<u8>>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = EncoderImpl::new(v16, config);
        <std::num::NonZeroIsize>::encode(&p0, &mut p1).unwrap();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_268 {
    use super::*;
    use crate::Encode;
    use crate::enc::EncoderImpl;
    use crate::features::IoWriter;
    use crate::config::{Configuration, BigEndian, Fixint, Limit};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: f32 = rug_fuzz_0;
        let mut writer: Vec<u8> = Vec::new();
        let mut v16: IoWriter<Vec<u8>> = IoWriter::new(&mut writer);
        let config = Configuration::<BigEndian, Fixint, Limit<100>>::default();
        let mut p1: EncoderImpl<
            IoWriter<Vec<u8>>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = EncoderImpl::new(v16, config);
        <f32>::encode(&p0, &mut p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_272 {
    use super::*;
    use crate::Encode;
    use crate::enc::{Encoder, EncodeError};
    use crate::enc::EncoderImpl;
    use crate::features::IoWriter;
    use crate::config::{Configuration, BigEndian, Fixint, Limit};
    use std::vec::Vec;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut writer: Vec<u8> = Vec::new();
        let mut v16: IoWriter<Vec<u8>> = IoWriter::new(&mut writer);
        let config = Configuration::<BigEndian, Fixint, Limit<100>>::default();
        let mut v19: EncoderImpl<
            IoWriter<Vec<u8>>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = EncoderImpl::new(v16, config);
        let p0: &str = rug_fuzz_0;
        let p1 = &mut v19;
        <str>::encode(&p0, p1).unwrap();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_273 {
    use super::*;
    use crate::enc::EncoderImpl;
    use crate::features::IoWriter;
    use crate::config::{Configuration, BigEndian, Fixint, Limit};
    use std::vec::Vec;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4)) = <(i32, i32, i32, i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: [i32; 5] = [
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
        ];
        let mut writer: Vec<u8> = Vec::new();
        let mut v16: IoWriter<Vec<u8>> = IoWriter::new(&mut writer);
        let config = Configuration::<BigEndian, Fixint, Limit<100>>::default();
        let mut p1: EncoderImpl<
            IoWriter<Vec<u8>>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = EncoderImpl::new(v16, config);
        <[i32; 5]>::encode(&p0, &mut p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_274 {
    use super::*;
    use crate::enc::{Encode, EncodeError};
    use crate::enc::encoder::EncoderImpl;
    use crate::features::IoWriter;
    use crate::config::{Configuration, BigEndian, Fixint, Limit};
    use std::vec::Vec;
    #[test]
    fn test_encode() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut writer: Vec<u8> = Vec::new();
        let mut v16: IoWriter<Vec<u8>> = IoWriter::new(&mut writer);
        let config = Configuration::<BigEndian, Fixint, Limit<100>>::default();
        let mut encoder: EncoderImpl<
            IoWriter<Vec<u8>>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = EncoderImpl::new(v16, config);
        let p0: Option<i32> = Some(rug_fuzz_0);
        p0.encode(&mut encoder).unwrap();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_277 {
    use super::*;
    use crate::Encode;
    use crate::enc::impls::EncodeError;
    use std::cell::RefCell;
    use crate::enc::EncoderImpl;
    use crate::features::IoWriter;
    use crate::config::{Configuration, BigEndian, Fixint, Limit};
    #[test]
    fn test_encode() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: RefCell<i32> = RefCell::new(rug_fuzz_0);
        let mut writer: Vec<u8> = Vec::new();
        let mut p1: EncoderImpl<
            IoWriter<Vec<u8>>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = EncoderImpl::new(
            IoWriter::new(&mut writer),
            Configuration::<BigEndian, Fixint, Limit<100>>::default(),
        );
        p0.encode(&mut p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_278 {
    use super::*;
    use crate::Encode;
    use crate::enc::EncoderImpl;
    use crate::features::IoWriter;
    use crate::config::{Configuration, BigEndian, Fixint, Limit};
    use std::time::Duration;
    use std::vec::Vec;
    #[test]
    fn test_encode() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u64, u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Duration = Duration::new(rug_fuzz_0, rug_fuzz_1);
        let mut writer: Vec<u8> = Vec::new();
        let mut p1: EncoderImpl<
            IoWriter<Vec<u8>>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = EncoderImpl::new(
            IoWriter::new(&mut writer),
            Configuration::<BigEndian, Fixint, Limit<100>>::default(),
        );
        <std::time::Duration as crate::Encode>::encode(&p0, &mut p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_280 {
    use super::*;
    use crate::Encode;
    use std::ops::RangeInclusive;
    use crate::enc::EncoderImpl;
    use crate::features::IoWriter;
    use crate::config::{Configuration, BigEndian, Fixint, Limit};
    use std::vec::Vec;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: RangeInclusive<i32> = rug_fuzz_0..=rug_fuzz_1;
        let mut writer: Vec<u8> = Vec::new();
        let mut p1: EncoderImpl<
            IoWriter<Vec<u8>>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = EncoderImpl::new(
            IoWriter::new(&mut writer),
            Configuration::<BigEndian, Fixint, Limit<100>>::default(),
        );
        p0.encode(&mut p1).unwrap();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_282 {
    use super::*;
    use crate::enc::{Encode, Encoder};
    use crate::enc::encoder::EncoderImpl;
    use crate::features::IoWriter;
    use crate::config::{Configuration, BigEndian, Fixint, Limit};
    use std::vec::Vec;
    use std::marker::PhantomData;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_282_rrrruuuugggg_test_rug = 0;
        let mut phantom: PhantomData<()> = PhantomData;
        let mut writer: Vec<u8> = Vec::new();
        let mut io_writer: IoWriter<Vec<u8>> = IoWriter::new(&mut writer);
        let config = Configuration::<BigEndian, Fixint, Limit<100>>::default();
        let mut encoder_impl: EncoderImpl<
            IoWriter<Vec<u8>>,
            Configuration<BigEndian, Fixint, Limit<100>>,
        > = EncoderImpl::new(io_writer, config);
        phantom.encode(&mut encoder_impl).unwrap();
        let _rug_ed_tests_rug_282_rrrruuuugggg_test_rug = 0;
    }
}
