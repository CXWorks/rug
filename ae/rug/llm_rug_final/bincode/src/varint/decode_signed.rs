use crate::{
    config::Endian,
    de::read::Reader,
    error::{DecodeError, IntegerType},
};

pub fn varint_decode_i16<R: Reader>(read: &mut R, endian: Endian) -> Result<i16, DecodeError> {
    let n = super::varint_decode_u16(read, endian)
        .map_err(DecodeError::change_integer_type_to_signed)?;
    Ok(if n % 2 == 0 {
        // positive number
        (n / 2) as _
    } else {
        // negative number
        // !m * 2 + 1 = n
        // !m * 2 = n - 1
        // !m = (n - 1) / 2
        // m = !((n - 1) / 2)
        // since we have n is odd, we have floor(n / 2) = floor((n - 1) / 2)
        !(n / 2) as _
    })
}

pub fn varint_decode_i32<R: Reader>(read: &mut R, endian: Endian) -> Result<i32, DecodeError> {
    let n = super::varint_decode_u32(read, endian)
        .map_err(DecodeError::change_integer_type_to_signed)?;
    Ok(if n % 2 == 0 {
        // positive number
        (n / 2) as _
    } else {
        // negative number
        // !m * 2 + 1 = n
        // !m * 2 = n - 1
        // !m = (n - 1) / 2
        // m = !((n - 1) / 2)
        // since we have n is odd, we have floor(n / 2) = floor((n - 1) / 2)
        !(n / 2) as _
    })
}

pub fn varint_decode_i64<R: Reader>(read: &mut R, endian: Endian) -> Result<i64, DecodeError> {
    let n = super::varint_decode_u64(read, endian)
        .map_err(DecodeError::change_integer_type_to_signed)?;
    Ok(if n % 2 == 0 {
        // positive number
        (n / 2) as _
    } else {
        // negative number
        // !m * 2 + 1 = n
        // !m * 2 = n - 1
        // !m = (n - 1) / 2
        // m = !((n - 1) / 2)
        // since we have n is odd, we have floor(n / 2) = floor((n - 1) / 2)
        !(n / 2) as _
    })
}

pub fn varint_decode_i128<R: Reader>(read: &mut R, endian: Endian) -> Result<i128, DecodeError> {
    let n = super::varint_decode_u128(read, endian)
        .map_err(DecodeError::change_integer_type_to_signed)?;
    Ok(if n % 2 == 0 {
        // positive number
        (n / 2) as _
    } else {
        // negative number
        // !m * 2 + 1 = n
        // !m * 2 = n - 1
        // !m = (n - 1) / 2
        // m = !((n - 1) / 2)
        // since we have n is odd, we have floor(n / 2) = floor((n - 1) / 2)
        !(n / 2) as _
    })
}

pub fn varint_decode_isize<R: Reader>(read: &mut R, endian: Endian) -> Result<isize, DecodeError> {
    match varint_decode_i64(read, endian) {
        Ok(val) => Ok(val as isize),
        Err(DecodeError::InvalidIntegerType { found, .. }) => {
            Err(DecodeError::InvalidIntegerType {
                expected: IntegerType::Isize,
                found: found.into_signed(),
            })
        }
        Err(e) => Err(e),
    }
}
#[cfg(test)]
mod tests_rug_88 {
    use crate::varint::decode_signed::{varint_decode_i16, Endian};
    use crate::de::read::SliceReader;

    #[test]
    fn test_rug() {
        let mut p0: SliceReader<'_> = SliceReader::new(&[0u8, 1u8, 2u8]);
        let p1: Endian = Endian::Big;
        
        varint_decode_i16(&mut p0, p1);
    }
}