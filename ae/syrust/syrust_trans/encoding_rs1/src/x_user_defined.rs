use super::*;
use handles::*;
use variant::*;
cfg_if! {
    if #[cfg(feature = "simd-accel")] { use simd_funcs::*; use packed_simd::u16x8;
    #[inline(always)] fn shift_upper(unpacked : u16x8) -> u16x8 { let highest_ascii =
    u16x8::splat(0x7F); unpacked + unpacked.gt(highest_ascii)
    .select(u16x8::splat(0xF700), u16x8::splat(0)) } } else {}
}
pub struct UserDefinedDecoder;
impl UserDefinedDecoder {
    pub fn new() -> VariantDecoder {
        VariantDecoder::UserDefined(UserDefinedDecoder)
    }
    pub fn max_utf16_buffer_length(&self, byte_length: usize) -> Option<usize> {
        Some(byte_length)
    }
    pub fn max_utf8_buffer_length_without_replacement(
        &self,
        byte_length: usize,
    ) -> Option<usize> {
        byte_length.checked_mul(3)
    }
    pub fn max_utf8_buffer_length(&self, byte_length: usize) -> Option<usize> {
        byte_length.checked_mul(3)
    }
    decoder_function!(
        {}, {}, {}, { if b < 0x80 { destination_handle.write_ascii(b); continue; }
        destination_handle.write_upper_bmp(u16::from(b) + 0xF700); continue; }, self,
        src_consumed, dest, source, b, destination_handle, _unread_handle,
        check_space_bmp, decode_to_utf8_raw, u8, Utf8Destination
    );
    #[cfg(not(feature = "simd-accel"))]
    pub fn decode_to_utf16_raw(
        &mut self,
        src: &[u8],
        dst: &mut [u16],
        _last: bool,
    ) -> (DecoderResult, usize, usize) {
        let (pending, length) = if dst.len() < src.len() {
            (DecoderResult::OutputFull, dst.len())
        } else {
            (DecoderResult::InputEmpty, src.len())
        };
        let src_trim = &src[..length];
        let dst_trim = &mut dst[..length];
        src_trim
            .iter()
            .zip(dst_trim.iter_mut())
            .for_each(|(from, to)| {
                *to = {
                    let unit = *from;
                    if unit < 0x80 { u16::from(unit) } else { u16::from(unit) + 0xF700 }
                }
            });
        (pending, length, length)
    }
    #[cfg(feature = "simd-accel")]
    pub fn decode_to_utf16_raw(
        &mut self,
        src: &[u8],
        dst: &mut [u16],
        _last: bool,
    ) -> (DecoderResult, usize, usize) {
        let (pending, length) = if dst.len() < src.len() {
            (DecoderResult::OutputFull, dst.len())
        } else {
            (DecoderResult::InputEmpty, src.len())
        };
        let tail_start = length & !0xF;
        let simd_iterations = length >> 4;
        let src_ptr = src.as_ptr();
        let dst_ptr = dst.as_mut_ptr();
        for i in 0..simd_iterations {
            let input = unsafe { load16_unaligned(src_ptr.add(i * 16)) };
            let (first, second) = simd_unpack(input);
            unsafe {
                store8_unaligned(dst_ptr.add(i * 16), shift_upper(first));
                store8_unaligned(dst_ptr.add((i * 16) + 8), shift_upper(second));
            }
        }
        let src_tail = &src[tail_start..length];
        let dst_tail = &mut dst[tail_start..length];
        src_tail
            .iter()
            .zip(dst_tail.iter_mut())
            .for_each(|(from, to)| {
                *to = {
                    let unit = *from;
                    if unit < 0x80 { u16::from(unit) } else { u16::from(unit) + 0xF700 }
                }
            });
        (pending, length, length)
    }
}
pub struct UserDefinedEncoder;
impl UserDefinedEncoder {
    pub fn new(encoding: &'static Encoding) -> Encoder {
        Encoder::new(encoding, VariantEncoder::UserDefined(UserDefinedEncoder))
    }
    pub fn max_buffer_length_from_utf16_without_replacement(
        &self,
        u16_length: usize,
    ) -> Option<usize> {
        Some(u16_length)
    }
    pub fn max_buffer_length_from_utf8_without_replacement(
        &self,
        byte_length: usize,
    ) -> Option<usize> {
        Some(byte_length)
    }
    encoder_functions!(
        {}, { if c <= '\u{7F}' { destination_handle.write_one(c as u8); continue; } if c
        < '\u{F780}' || c > '\u{F7FF}' { return (EncoderResult::Unmappable(c),
        unread_handle.consumed(), destination_handle.written(),); } destination_handle
        .write_one((u32::from(c) - 0xF700) as u8); continue; }, self, src_consumed,
        source, dest, c, destination_handle, unread_handle, check_space_one
    );
}
#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use super::super::*;
    fn decode_x_user_defined(bytes: &[u8], expect: &str) {
        decode(X_USER_DEFINED, bytes, expect);
    }
    fn encode_x_user_defined(string: &str, expect: &[u8]) {
        encode(X_USER_DEFINED, string, expect);
    }
    #[test]
    fn test_x_user_defined_decode() {
        decode_x_user_defined(b"", "");
        decode_x_user_defined(b"\x61\x62", "\u{0061}\u{0062}");
        decode_x_user_defined(b"\x80\xFF", "\u{F780}\u{F7FF}");
        decode_x_user_defined(
            b"\x80\xFF\x61\x62\x80\xFF\x61\x62\x80\xFF\x61\x62\x80\xFF\x61\x62\x80\xFF\x61\x62",
            "\u{F780}\u{F7FF}\u{0061}\u{0062}\u{F780}\u{F7FF}\u{0061}\u{0062}\u{F780}\u{F7FF}\u{0061}\u{0062}\u{F780}\u{F7FF}\u{0061}\u{0062}\u{F780}\u{F7FF}\u{0061}\u{0062}",
        );
    }
    #[test]
    fn test_x_user_defined_encode() {
        encode_x_user_defined("", b"");
        encode_x_user_defined("\u{0061}\u{0062}", b"\x61\x62");
        encode_x_user_defined("\u{F780}\u{F7FF}", b"\x80\xFF");
        encode_x_user_defined("\u{F77F}\u{F800}", b"&#63359;&#63488;");
    }
    #[test]
    fn test_x_user_defined_from_two_low_surrogates() {
        let expectation = b"&#65533;&#65533;";
        let mut output = [0u8; 40];
        let mut encoder = X_USER_DEFINED.new_encoder();
        let (result, read, written, had_errors) = encoder
            .encode_from_utf16(&[0xDC00u16, 0xDEDEu16], &mut output[..], true);
        assert_eq!(result, CoderResult::InputEmpty);
        assert_eq!(read, 2);
        assert_eq!(written, expectation.len());
        assert!(had_errors);
        assert_eq!(& output[..written], expectation);
    }
}
#[cfg(test)]
mod tests_rug_475 {
    use super::*;
    use crate::x_user_defined::UserDefinedDecoder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_475_rrrruuuugggg_test_rug = 0;
        UserDefinedDecoder::new();
        let _rug_ed_tests_rug_475_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_476 {
    use super::*;
    use x_user_defined::UserDefinedDecoder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_476_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0 = UserDefinedDecoder::new();
        let p1: usize = rug_fuzz_0;
        let result = p0.max_utf16_buffer_length(p1);
        debug_assert_eq!(result, Some(10));
        let _rug_ed_tests_rug_476_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_477 {
    use super::*;
    use x_user_defined::UserDefinedDecoder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_477_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0 = UserDefinedDecoder::new();
        let p1: usize = rug_fuzz_0;
        p0.max_utf8_buffer_length_without_replacement(p1);
        let _rug_ed_tests_rug_477_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_478 {
    use super::*;
    use x_user_defined::UserDefinedDecoder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_478_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0 = UserDefinedDecoder::new();
        let p1: usize = rug_fuzz_0;
        let result = p0.max_utf8_buffer_length(p1);
        debug_assert_eq!(result, Some(30));
        let _rug_ed_tests_rug_478_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_479 {
    use super::*;
    use crate::x_user_defined::{UserDefinedDecoder, DecoderResult};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_479_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"Hello";
        let rug_fuzz_1 = true;
        let mut p0 = UserDefinedDecoder::new();
        let p1 = rug_fuzz_0.as_ref();
        let mut p2 = vec![0u16; 10];
        let p3 = rug_fuzz_1;
        let result = p0.decode_to_utf16_raw(p1, &mut p2, p3);
        debug_assert_eq!(result.0, DecoderResult::InputEmpty);
        debug_assert_eq!(result.1, 5);
        debug_assert_eq!(result.2, 5);
        let _rug_ed_tests_rug_479_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_480 {
    use super::*;
    use crate::Encoding;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_480_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"euc-jp";
        let rug_fuzz_1 = "Encoding not found";
        let v3 = Encoding::for_label(rug_fuzz_0).expect(rug_fuzz_1);
        x_user_defined::UserDefinedEncoder::new(&v3);
        let _rug_ed_tests_rug_480_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_481 {
    use super::*;
    use crate::x_user_defined::UserDefinedEncoder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_481_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0 = x_user_defined::UserDefinedEncoder {
        };
        let mut p1: usize = rug_fuzz_0;
        p0.max_buffer_length_from_utf16_without_replacement(p1);
        let _rug_ed_tests_rug_481_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_482 {
    use super::*;
    use crate::x_user_defined;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_482_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0 = x_user_defined::UserDefinedEncoder;
        let mut p1: usize = rug_fuzz_0;
        <x_user_defined::UserDefinedEncoder>::max_buffer_length_from_utf8_without_replacement(
            &p0,
            p1,
        );
        let _rug_ed_tests_rug_482_rrrruuuugggg_test_rug = 0;
    }
}
