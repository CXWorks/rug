use super::{SINGLE_BYTE_MAX, U128_BYTE, U16_BYTE, U32_BYTE, U64_BYTE};
use crate::{config::Endian, enc::write::Writer, error::EncodeError};
pub fn varint_encode_u16<W: Writer>(
    writer: &mut W,
    endian: Endian,
    val: u16,
) -> Result<(), EncodeError> {
    if val <= SINGLE_BYTE_MAX as _ {
        writer.write(&[val as u8])
    } else {
        writer.write(&[U16_BYTE])?;
        match endian {
            Endian::Big => writer.write(&val.to_be_bytes()),
            Endian::Little => writer.write(&val.to_le_bytes()),
        }
    }
}
pub fn varint_encode_u32<W: Writer>(
    writer: &mut W,
    endian: Endian,
    val: u32,
) -> Result<(), EncodeError> {
    if val <= SINGLE_BYTE_MAX as _ {
        writer.write(&[val as u8])
    } else if val <= u16::MAX as _ {
        writer.write(&[U16_BYTE])?;
        match endian {
            Endian::Big => writer.write(&(val as u16).to_be_bytes()),
            Endian::Little => writer.write(&(val as u16).to_le_bytes()),
        }
    } else {
        writer.write(&[U32_BYTE])?;
        match endian {
            Endian::Big => writer.write(&val.to_be_bytes()),
            Endian::Little => writer.write(&val.to_le_bytes()),
        }
    }
}
pub fn varint_encode_u64<W: Writer>(
    writer: &mut W,
    endian: Endian,
    val: u64,
) -> Result<(), EncodeError> {
    if val <= SINGLE_BYTE_MAX as _ {
        writer.write(&[val as u8])
    } else if val <= u16::MAX as _ {
        writer.write(&[U16_BYTE])?;
        match endian {
            Endian::Big => writer.write(&(val as u16).to_be_bytes()),
            Endian::Little => writer.write(&(val as u16).to_le_bytes()),
        }
    } else if val <= u32::MAX as _ {
        writer.write(&[U32_BYTE])?;
        match endian {
            Endian::Big => writer.write(&(val as u32).to_be_bytes()),
            Endian::Little => writer.write(&(val as u32).to_le_bytes()),
        }
    } else {
        writer.write(&[U64_BYTE])?;
        match endian {
            Endian::Big => writer.write(&val.to_be_bytes()),
            Endian::Little => writer.write(&val.to_le_bytes()),
        }
    }
}
pub fn varint_encode_u128<W: Writer>(
    writer: &mut W,
    endian: Endian,
    val: u128,
) -> Result<(), EncodeError> {
    if val <= SINGLE_BYTE_MAX as _ {
        writer.write(&[val as u8])
    } else if val <= u16::MAX as _ {
        writer.write(&[U16_BYTE])?;
        match endian {
            Endian::Big => writer.write(&(val as u16).to_be_bytes()),
            Endian::Little => writer.write(&(val as u16).to_le_bytes()),
        }
    } else if val <= u32::MAX as _ {
        writer.write(&[U32_BYTE])?;
        match endian {
            Endian::Big => writer.write(&(val as u32).to_be_bytes()),
            Endian::Little => writer.write(&(val as u32).to_le_bytes()),
        }
    } else if val <= u64::MAX as _ {
        writer.write(&[U64_BYTE])?;
        match endian {
            Endian::Big => writer.write(&(val as u64).to_be_bytes()),
            Endian::Little => writer.write(&(val as u64).to_le_bytes()),
        }
    } else {
        writer.write(&[U128_BYTE])?;
        match endian {
            Endian::Big => writer.write(&val.to_be_bytes()),
            Endian::Little => writer.write(&val.to_le_bytes()),
        }
    }
}
pub fn varint_encode_usize<W: Writer>(
    writer: &mut W,
    endian: Endian,
    val: usize,
) -> Result<(), EncodeError> {
    varint_encode_u64(writer, endian, val as u64)
}
#[test]
fn test_encode_u16() {
    use crate::enc::write::SliceWriter;
    let mut buffer = [0u8; 20];
    for i in 0u16..=SINGLE_BYTE_MAX as u16 {
        let mut writer = SliceWriter::new(&mut buffer);
        varint_encode_u16(&mut writer, Endian::Big, i).unwrap();
        assert_eq!(writer.bytes_written(), 1);
        assert_eq!(buffer[0] as u16, i);
        let mut writer = SliceWriter::new(&mut buffer);
        varint_encode_u16(&mut writer, Endian::Little, i).unwrap();
        assert_eq!(writer.bytes_written(), 1);
        assert_eq!(buffer[0] as u16, i);
    }
    for i in [SINGLE_BYTE_MAX as u16 + 1, 300, 500, 700, 888, 1234, u16::MAX] {
        let mut writer = SliceWriter::new(&mut buffer);
        varint_encode_u16(&mut writer, Endian::Big, i).unwrap();
        assert_eq!(writer.bytes_written(), 3);
        assert_eq!(buffer[0], U16_BYTE);
        assert_eq!(& buffer[1..3], & i.to_be_bytes());
        let mut writer = SliceWriter::new(&mut buffer);
        varint_encode_u16(&mut writer, Endian::Little, i).unwrap();
        assert_eq!(writer.bytes_written(), 3);
        assert_eq!(buffer[0], U16_BYTE);
        assert_eq!(& buffer[1..3], & i.to_le_bytes());
    }
}
#[test]
fn test_encode_u32() {
    use crate::enc::write::SliceWriter;
    let mut buffer = [0u8; 20];
    for i in 0u32..=SINGLE_BYTE_MAX as u32 {
        let mut writer = SliceWriter::new(&mut buffer);
        varint_encode_u32(&mut writer, Endian::Big, i).unwrap();
        assert_eq!(writer.bytes_written(), 1);
        assert_eq!(buffer[0] as u32, i);
        let mut writer = SliceWriter::new(&mut buffer);
        varint_encode_u32(&mut writer, Endian::Little, i).unwrap();
        assert_eq!(writer.bytes_written(), 1);
        assert_eq!(buffer[0] as u32, i);
    }
    for i in [SINGLE_BYTE_MAX as u32 + 1, 300, 500, 700, 888, 1234, u16::MAX as u32] {
        let mut writer = SliceWriter::new(&mut buffer);
        varint_encode_u32(&mut writer, Endian::Big, i).unwrap();
        assert_eq!(writer.bytes_written(), 3);
        assert_eq!(buffer[0], U16_BYTE);
        assert_eq!(& buffer[1..3], & (i as u16).to_be_bytes());
        let mut writer = SliceWriter::new(&mut buffer);
        varint_encode_u32(&mut writer, Endian::Little, i).unwrap();
        assert_eq!(writer.bytes_written(), 3);
        assert_eq!(buffer[0], U16_BYTE);
        assert_eq!(& buffer[1..3], & (i as u16).to_le_bytes());
    }
    for i in [u16::MAX as u32 + 1, 100_000, 1_000_000, u32::MAX] {
        let mut writer = SliceWriter::new(&mut buffer);
        varint_encode_u32(&mut writer, Endian::Big, i).unwrap();
        assert_eq!(writer.bytes_written(), 5);
        assert_eq!(buffer[0], U32_BYTE);
        assert_eq!(& buffer[1..5], & i.to_be_bytes());
        let mut writer = SliceWriter::new(&mut buffer);
        varint_encode_u32(&mut writer, Endian::Little, i).unwrap();
        assert_eq!(writer.bytes_written(), 5);
        assert_eq!(buffer[0], U32_BYTE);
        assert_eq!(& buffer[1..5], & i.to_le_bytes());
    }
}
#[test]
fn test_encode_u64() {
    use crate::enc::write::SliceWriter;
    let mut buffer = [0u8; 20];
    for i in 0u64..=SINGLE_BYTE_MAX as u64 {
        let mut writer = SliceWriter::new(&mut buffer);
        varint_encode_u64(&mut writer, Endian::Big, i).unwrap();
        assert_eq!(writer.bytes_written(), 1);
        assert_eq!(buffer[0] as u64, i);
        let mut writer = SliceWriter::new(&mut buffer);
        varint_encode_u64(&mut writer, Endian::Little, i).unwrap();
        assert_eq!(writer.bytes_written(), 1);
        assert_eq!(buffer[0] as u64, i);
    }
    for i in [SINGLE_BYTE_MAX as u64 + 1, 300, 500, 700, 888, 1234, u16::MAX as u64] {
        let mut writer = SliceWriter::new(&mut buffer);
        varint_encode_u64(&mut writer, Endian::Big, i).unwrap();
        assert_eq!(writer.bytes_written(), 3);
        assert_eq!(buffer[0], U16_BYTE);
        assert_eq!(& buffer[1..3], & (i as u16).to_be_bytes());
        let mut writer = SliceWriter::new(&mut buffer);
        varint_encode_u64(&mut writer, Endian::Little, i).unwrap();
        assert_eq!(writer.bytes_written(), 3);
        assert_eq!(buffer[0], U16_BYTE);
        assert_eq!(& buffer[1..3], & (i as u16).to_le_bytes());
    }
    for i in [u16::MAX as u64 + 1, 100_000, 1_000_000, u32::MAX as u64] {
        let mut writer = SliceWriter::new(&mut buffer);
        varint_encode_u64(&mut writer, Endian::Big, i).unwrap();
        assert_eq!(writer.bytes_written(), 5);
        assert_eq!(buffer[0], U32_BYTE);
        assert_eq!(& buffer[1..5], & (i as u32).to_be_bytes());
        let mut writer = SliceWriter::new(&mut buffer);
        varint_encode_u64(&mut writer, Endian::Little, i).unwrap();
        assert_eq!(writer.bytes_written(), 5);
        assert_eq!(buffer[0], U32_BYTE);
        assert_eq!(& buffer[1..5], & (i as u32).to_le_bytes());
    }
    for i in [u32::MAX as u64 + 1, 5_000_000_000, u64::MAX] {
        let mut writer = SliceWriter::new(&mut buffer);
        varint_encode_u64(&mut writer, Endian::Big, i).unwrap();
        assert_eq!(writer.bytes_written(), 9);
        assert_eq!(buffer[0], U64_BYTE);
        assert_eq!(& buffer[1..9], & i.to_be_bytes());
        let mut writer = SliceWriter::new(&mut buffer);
        varint_encode_u64(&mut writer, Endian::Little, i).unwrap();
        assert_eq!(writer.bytes_written(), 9);
        assert_eq!(buffer[0], U64_BYTE);
        assert_eq!(& buffer[1..9], & i.to_le_bytes());
    }
}
#[test]
fn test_encode_u128() {
    use crate::enc::write::SliceWriter;
    let mut buffer = [0u8; 20];
    for i in 0u128..=SINGLE_BYTE_MAX as u128 {
        let mut writer = SliceWriter::new(&mut buffer);
        varint_encode_u128(&mut writer, Endian::Big, i).unwrap();
        assert_eq!(writer.bytes_written(), 1);
        assert_eq!(buffer[0] as u128, i);
        let mut writer = SliceWriter::new(&mut buffer);
        varint_encode_u128(&mut writer, Endian::Little, i).unwrap();
        assert_eq!(writer.bytes_written(), 1);
        assert_eq!(buffer[0] as u128, i);
    }
    for i in [SINGLE_BYTE_MAX as u128 + 1, 300, 500, 700, 888, 1234, u16::MAX as u128] {
        let mut writer = SliceWriter::new(&mut buffer);
        varint_encode_u128(&mut writer, Endian::Big, i).unwrap();
        assert_eq!(writer.bytes_written(), 3);
        assert_eq!(buffer[0], U16_BYTE);
        assert_eq!(& buffer[1..3], & (i as u16).to_be_bytes());
        let mut writer = SliceWriter::new(&mut buffer);
        varint_encode_u128(&mut writer, Endian::Little, i).unwrap();
        assert_eq!(writer.bytes_written(), 3);
        assert_eq!(buffer[0], U16_BYTE);
        assert_eq!(& buffer[1..3], & (i as u16).to_le_bytes());
    }
    for i in [u16::MAX as u128 + 1, 100_000, 1_000_000, u32::MAX as u128] {
        let mut writer = SliceWriter::new(&mut buffer);
        varint_encode_u128(&mut writer, Endian::Big, i).unwrap();
        assert_eq!(writer.bytes_written(), 5);
        assert_eq!(buffer[0], U32_BYTE);
        assert_eq!(& buffer[1..5], & (i as u32).to_be_bytes());
        let mut writer = SliceWriter::new(&mut buffer);
        varint_encode_u128(&mut writer, Endian::Little, i).unwrap();
        assert_eq!(writer.bytes_written(), 5);
        assert_eq!(buffer[0], U32_BYTE);
        assert_eq!(& buffer[1..5], & (i as u32).to_le_bytes());
    }
    for i in [u32::MAX as u128 + 1, 5_000_000_000, u64::MAX as u128] {
        let mut writer = SliceWriter::new(&mut buffer);
        varint_encode_u128(&mut writer, Endian::Big, i).unwrap();
        assert_eq!(writer.bytes_written(), 9);
        assert_eq!(buffer[0], U64_BYTE);
        assert_eq!(& buffer[1..9], & (i as u64).to_be_bytes());
        let mut writer = SliceWriter::new(&mut buffer);
        varint_encode_u128(&mut writer, Endian::Little, i).unwrap();
        assert_eq!(writer.bytes_written(), 9);
        assert_eq!(buffer[0], U64_BYTE);
        assert_eq!(& buffer[1..9], & (i as u64).to_le_bytes());
    }
    for i in [u64::MAX as u128 + 1, u128::MAX] {
        let mut writer = SliceWriter::new(&mut buffer);
        varint_encode_u128(&mut writer, Endian::Big, i).unwrap();
        assert_eq!(writer.bytes_written(), 17);
        assert_eq!(buffer[0], U128_BYTE);
        assert_eq!(& buffer[1..17], & i.to_be_bytes());
        let mut writer = SliceWriter::new(&mut buffer);
        varint_encode_u128(&mut writer, Endian::Little, i).unwrap();
        assert_eq!(writer.bytes_written(), 17);
        assert_eq!(buffer[0], U128_BYTE);
        assert_eq!(& buffer[1..17], & i.to_le_bytes());
    }
}
#[cfg(test)]
mod tests_rug_109 {
    use super::*;
    use crate::features::VecWriter;
    use crate::varint::encode_unsigned::{varint_encode_u16, Endian};
    #[test]
    fn test_varint_encode_u16() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(usize, u16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = VecWriter::with_capacity(rug_fuzz_0);
        let p1 = Endian::Big;
        let p2: u16 = rug_fuzz_1;
        varint_encode_u16(&mut p0, p1, p2).unwrap();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_112 {
    use super::*;
    use crate::enc::write::SizeWriter;
    use crate::varint::encode_unsigned::{varint_encode_u128, Endian};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(usize, u128) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = SizeWriter {
            bytes_written: rug_fuzz_0,
        };
        let p1 = Endian::Big;
        let p2: u128 = rug_fuzz_1;
        varint_encode_u128(&mut p0, p1, p2).unwrap();
             }
}
}
}    }
}
