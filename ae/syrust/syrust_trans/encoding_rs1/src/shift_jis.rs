use super::*;
use data::*;
use handles::*;
use variant::*;
use super::in_inclusive_range;
use super::in_inclusive_range16;
pub struct ShiftJisDecoder {
    lead: Option<u8>,
}
impl ShiftJisDecoder {
    pub fn new() -> VariantDecoder {
        VariantDecoder::ShiftJis(ShiftJisDecoder { lead: None })
    }
    pub fn in_neutral_state(&self) -> bool {
        self.lead.is_none()
    }
    fn plus_one_if_lead(&self, byte_length: usize) -> Option<usize> {
        byte_length
            .checked_add(
                match self.lead {
                    None => 0,
                    Some(_) => 1,
                },
            )
    }
    pub fn max_utf16_buffer_length(&self, byte_length: usize) -> Option<usize> {
        self.plus_one_if_lead(byte_length)
    }
    pub fn max_utf8_buffer_length_without_replacement(
        &self,
        byte_length: usize,
    ) -> Option<usize> {
        self.max_utf8_buffer_length(byte_length)
    }
    pub fn max_utf8_buffer_length(&self, byte_length: usize) -> Option<usize> {
        checked_mul(3, self.plus_one_if_lead(byte_length))
    }
    ascii_compatible_two_byte_decoder_functions!(
        { let mut non_ascii_minus_offset = non_ascii.wrapping_sub(0x81); if
        non_ascii_minus_offset > (0x9F - 0x81) { let non_ascii_minus_range_start =
        non_ascii.wrapping_sub(0xE0); if non_ascii_minus_range_start > (0xFC - 0xE0) {
        let non_ascii_minus_half_with_katakana_start = non_ascii.wrapping_sub(0xA1); if
        non_ascii_minus_half_with_katakana_start > (0xDF - 0xA1) { if non_ascii == 0x80 {
        handle.write_mid_bmp(0x80); continue 'outermost; } return
        (DecoderResult::Malformed(1, 0), source.consumed(), handle.written()); } handle
        .write_upper_bmp(0xFF61 + u16::from(non_ascii_minus_half_with_katakana_start));
        continue 'outermost; } non_ascii_minus_offset = non_ascii - 0xC1; }
        non_ascii_minus_offset }, { let trail_minus_hiragana = byte.wrapping_sub(0x9F);
        if lead_minus_offset == 0x01 && trail_minus_hiragana < 0x53 { handle
        .write_upper_bmp(0x3041 + u16::from(trail_minus_hiragana)) } else { let mut
        trail_minus_offset = byte.wrapping_sub(0x40); if trail_minus_offset > (0x7E -
        0x40) { let trail_minus_range_start = byte.wrapping_sub(0x80); if
        trail_minus_range_start > (0xFC - 0x80) { if byte < 0x80 { return
        (DecoderResult::Malformed(1, 0), unread_handle_trail.unread(), handle.written());
        } return (DecoderResult::Malformed(2, 0), unread_handle_trail.consumed(), handle
        .written()); } trail_minus_offset = byte - 0x41; } if lead_minus_offset == 0x02
        && trail_minus_offset < 0x56 { handle.write_upper_bmp(0x30A1 +
        u16::from(trail_minus_offset)) } else { let pointer = lead_minus_offset as usize
        * 188usize + trail_minus_offset as usize; let level1_pointer = pointer
        .wrapping_sub(1410); if level1_pointer < JIS0208_LEVEL1_KANJI.len() { handle
        .write_upper_bmp(JIS0208_LEVEL1_KANJI[level1_pointer]) } else { let
        level2_pointer = pointer.wrapping_sub(4418); if level2_pointer <
        JIS0208_LEVEL2_AND_ADDITIONAL_KANJI.len() { handle
        .write_upper_bmp(JIS0208_LEVEL2_AND_ADDITIONAL_KANJI[level2_pointer]) } else {
        let upper_ibm_pointer = pointer.wrapping_sub(10744); if upper_ibm_pointer <
        IBM_KANJI.len() { handle.write_upper_bmp(IBM_KANJI[upper_ibm_pointer]) } else {
        let lower_ibm_pointer = pointer.wrapping_sub(8272); if lower_ibm_pointer <
        IBM_KANJI.len() { handle.write_upper_bmp(IBM_KANJI[lower_ibm_pointer]) } else if
        in_inclusive_range(pointer, 8836, 10715) { handle.write_upper_bmp((0xE000 - 8836
        + pointer) as u16) } else if let Some(bmp) = jis0208_symbol_decode(pointer) {
        handle.write_bmp_excl_ascii(bmp) } else if let Some(bmp) =
        jis0208_range_decode(pointer) { handle.write_bmp_excl_ascii(bmp) } else { if byte
        < 0x80 { return (DecoderResult::Malformed(1, 0), unread_handle_trail.unread(),
        handle.written()); } return (DecoderResult::Malformed(2, 0), unread_handle_trail
        .consumed(), handle.written()); } } } } } } }, self, non_ascii, byte,
        lead_minus_offset, unread_handle_trail, source, handle, 'outermost,
        copy_ascii_from_check_space_bmp, check_space_bmp, false
    );
}
#[cfg(feature = "fast-kanji-encode")]
#[inline(always)]
fn encode_kanji(bmp: u16) -> Option<(u8, u8)> {
    jis0208_kanji_shift_jis_encode(bmp)
}
#[cfg(not(feature = "fast-kanji-encode"))]
#[inline(always)]
fn encode_kanji(bmp: u16) -> Option<(u8, u8)> {
    if let Some((lead, trail)) = jis0208_level1_kanji_shift_jis_encode(bmp) {
        return Some((lead, trail));
    }
    let pointer = if 0x4EDD == bmp {
        23
    } else if let Some(pos) = jis0208_level2_and_additional_kanji_encode(bmp) {
        4418 + pos
    } else if let Some(pos) = position(&IBM_KANJI[..], bmp) {
        10744 + pos
    } else {
        return None;
    };
    let lead = pointer / 188;
    let lead_offset = if lead < 0x1F { 0x81usize } else { 0xC1usize };
    let trail = pointer % 188;
    let trail_offset = if trail < 0x3F { 0x40usize } else { 0x41usize };
    Some(((lead + lead_offset) as u8, (trail + trail_offset) as u8))
}
pub struct ShiftJisEncoder;
impl ShiftJisEncoder {
    pub fn new(encoding: &'static Encoding) -> Encoder {
        Encoder::new(encoding, VariantEncoder::ShiftJis(ShiftJisEncoder))
    }
    pub fn max_buffer_length_from_utf16_without_replacement(
        &self,
        u16_length: usize,
    ) -> Option<usize> {
        u16_length.checked_mul(2)
    }
    pub fn max_buffer_length_from_utf8_without_replacement(
        &self,
        byte_length: usize,
    ) -> Option<usize> {
        byte_length.checked_add(1)
    }
    ascii_compatible_bmp_encoder_functions!(
        { let bmp_minus_hiragana = bmp.wrapping_sub(0x3041); if bmp_minus_hiragana < 0x53
        { handle.write_two(0x82, 0x9F + bmp_minus_hiragana as u8) } else if
        in_inclusive_range16(bmp, 0x4E00, 0x9FA0) { if let Some((lead, trail)) =
        encode_kanji(bmp) { handle.write_two(lead, trail) } else { return
        (EncoderResult::unmappable_from_bmp(bmp), source.consumed(), handle.written(),);
        } } else { let bmp_minus_katakana = bmp.wrapping_sub(0x30A1); if
        bmp_minus_katakana < 0x56 { let trail_offset = if bmp_minus_katakana < 0x3F {
        0x40 } else { 0x41 }; handle.write_two(0x83, (trail_offset + bmp_minus_katakana)
        as u8) } else { let bmp_minus_space = bmp.wrapping_sub(0x3000); if
        bmp_minus_space < 3 { handle.write_two(0x81, 0x40 + bmp_minus_space as u8) } else
        if bmp == 0xA5 { handle.write_one(0x5Cu8) } else if bmp == 0x80 { handle
        .write_one(0x80u8) } else if bmp == 0x203E { handle.write_one(0x7Eu8) } else if
        in_inclusive_range16(bmp, 0xFF61, 0xFF9F) { handle.write_one((bmp - (0xFF61 -
        0xA1)) as u8) } else if bmp == 0x2212 { handle.write_two(0x81u8, 0x7Cu8) } else {
        let bmp_minus_roman = bmp.wrapping_sub(0x2170); let pointer = if bmp_minus_roman
        <= (0x2179 - 0x2170) { 10716 + bmp_minus_roman as usize } else if let
        Some(pointer) = jis0208_range_encode(bmp) { pointer } else if
        in_inclusive_range16(bmp, 0xFA0E, 0xFA2D) || bmp == 0xF929 || bmp == 0xF9DC { let
        pos = position(& IBM_KANJI[..], bmp).unwrap(); 10744 + pos } else if let
        Some(pointer) = jis0208_symbol_encode(bmp) { pointer } else { return
        (EncoderResult::unmappable_from_bmp(bmp), source.consumed(), handle.written(),);
        }; let lead = pointer / 188; let lead_offset = if lead < 0x1F { 0x81usize } else
        { 0xC1usize }; let trail = pointer % 188; let trail_offset = if trail < 0x3F {
        0x40usize } else { 0x41usize }; handle.write_two((lead + lead_offset) as u8,
        (trail + trail_offset) as u8) } } } }, bmp, self, source, handle,
        copy_ascii_to_check_space_two, check_space_two, false
    );
}
#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use super::super::*;
    fn decode_shift_jis(bytes: &[u8], expect: &str) {
        decode(SHIFT_JIS, bytes, expect);
    }
    fn encode_shift_jis(string: &str, expect: &[u8]) {
        encode(SHIFT_JIS, string, expect);
    }
    #[test]
    fn test_shift_jis_decode() {
        decode_shift_jis(b"", &"");
        decode_shift_jis(b"\x61\x62", "\u{0061}\u{0062}");
        decode_shift_jis(b"\xA1", "\u{FF61}");
        decode_shift_jis(b"\xDF", "\u{FF9F}");
        decode_shift_jis(b"\xA0", "\u{FFFD}");
        decode_shift_jis(b"\xE0", "\u{FFFD}");
        decode_shift_jis(b"\xA0+", "\u{FFFD}+");
        decode_shift_jis(b"\xE0+", "\u{FFFD}+");
        decode_shift_jis(b"\xF0\x40", "\u{E000}");
        decode_shift_jis(b"\xF9\xFC", "\u{E757}");
        decode_shift_jis(b"\xEF\xFC", "\u{FFFD}");
        decode_shift_jis(b"\xFA\x40", "\u{2170}");
        decode_shift_jis(b"\x81\x40", "\u{3000}");
        decode_shift_jis(b"\x81\x3F", "\u{FFFD}?");
        decode_shift_jis(b"\xEE\xFC", "\u{FF02}");
        decode_shift_jis(b"\xEE\xFD", "\u{FFFD}");
        decode_shift_jis(b"\xFA\x40", "\u{2170}");
        decode_shift_jis(b"\xFA\x3F", "\u{FFFD}?");
        decode_shift_jis(b"\xFC\x4B", "\u{9ED1}");
        decode_shift_jis(b"\xFC\x4C", "\u{FFFD}L");
    }
    #[test]
    fn test_shift_jis_encode() {
        encode_shift_jis("", b"");
        encode_shift_jis("\u{0061}\u{0062}", b"\x61\x62");
        encode_shift_jis("\u{0080}", b"\x80");
        encode_shift_jis("\u{00A5}", b"\x5C");
        encode_shift_jis("\u{203E}", b"\x7E");
        encode_shift_jis("\u{2212}", b"\x81\x7C");
        encode_shift_jis("\u{FF61}", b"\xA1");
        encode_shift_jis("\u{FF9F}", b"\xDF");
        encode_shift_jis("\u{E000}", b"&#57344;");
        encode_shift_jis("\u{E757}", b"&#59223;");
        encode_shift_jis("\u{02D8}", b"&#728;");
        encode_shift_jis("\u{3000}", b"\x81\x40");
        encode_shift_jis("\u{FF02}", b"\xFA\x57");
        encode_shift_jis("\u{2170}", b"\xFA\x40");
        encode_shift_jis("\u{9ED1}", b"\xFC\x4B");
    }
    #[test]
    fn test_shift_jis_decode_all() {
        let input = include_bytes!("test_data/shift_jis_in.txt");
        let expectation = include_str!("test_data/shift_jis_in_ref.txt");
        let (cow, had_errors) = SHIFT_JIS.decode_without_bom_handling(input);
        assert!(had_errors, "Should have had errors.");
        assert_eq!(& cow[..], expectation);
    }
    #[test]
    fn test_shift_jis_encode_all() {
        let input = include_str!("test_data/shift_jis_out.txt");
        let expectation = include_bytes!("test_data/shift_jis_out_ref.txt");
        let (cow, encoding, had_errors) = SHIFT_JIS.encode(input);
        assert!(! had_errors, "Should not have had errors.");
        assert_eq!(encoding, SHIFT_JIS);
        assert_eq!(& cow[..], & expectation[..]);
    }
    #[test]
    fn test_shift_jis_half_width_katakana_length() {
        let mut output = [0u8; 20];
        let mut decoder = SHIFT_JIS.new_decoder();
        {
            let needed = decoder.max_utf8_buffer_length_without_replacement(1).unwrap();
            let (result, read, written) = decoder
                .decode_to_utf8_without_replacement(
                    b"\xA1",
                    &mut output[..needed],
                    true,
                );
            assert_eq!(result, DecoderResult::InputEmpty);
            assert_eq!(read, 1);
            assert_eq!(written, 3);
            assert_eq!(output[0], 0xEF);
            assert_eq!(output[1], 0xBD);
            assert_eq!(output[2], 0xA1);
        }
    }
}
#[cfg(test)]
mod tests_rug_52 {
    use super::*;
    use crate::shift_jis::{
        jis0208_level1_kanji_shift_jis_encode,
        jis0208_level2_and_additional_kanji_encode, position, IBM_KANJI,
    };
    #[test]
    fn test_encode_kanji() {
        let _rug_st_tests_rug_52_rrrruuuugggg_test_encode_kanji = 0;
        let rug_fuzz_0 = 0x4EDD;
        let p0: u16 = rug_fuzz_0;
        debug_assert_eq!(encode_kanji(p0), Some((0x88, 0x9F)));
        let _rug_ed_tests_rug_52_rrrruuuugggg_test_encode_kanji = 0;
    }
}
#[cfg(test)]
mod tests_rug_53 {
    use super::*;
    use crate::{Decoder, VariantDecoder};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_53_rrrruuuugggg_test_rug = 0;
        ShiftJisDecoder::new();
        let _rug_ed_tests_rug_53_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_54 {
    use super::*;
    use shift_jis::ShiftJisDecoder;
    #[test]
    fn test_in_neutral_state() {
        let _rug_st_tests_rug_54_rrrruuuugggg_test_in_neutral_state = 0;
        let p0 = ShiftJisDecoder { lead: None };
        let result = p0.in_neutral_state();
        debug_assert_eq!(result, true);
        let _rug_ed_tests_rug_54_rrrruuuugggg_test_in_neutral_state = 0;
    }
}
#[cfg(test)]
mod tests_rug_56 {
    use super::*;
    use crate::shift_jis;
    #[test]
    fn test_max_utf16_buffer_length() {
        let _rug_st_tests_rug_56_rrrruuuugggg_test_max_utf16_buffer_length = 0;
        let rug_fuzz_0 = 10;
        let p0 = shift_jis::ShiftJisDecoder::new();
        let p1: usize = rug_fuzz_0;
        p0.max_utf16_buffer_length(p1);
        let _rug_ed_tests_rug_56_rrrruuuugggg_test_max_utf16_buffer_length = 0;
    }
}
#[cfg(test)]
mod tests_rug_59 {
    use super::*;
    use crate::Encoding;
    #[test]
    fn test_shift_jis_encoder_new() {
        let _rug_st_tests_rug_59_rrrruuuugggg_test_shift_jis_encoder_new = 0;
        let rug_fuzz_0 = b"euc-jp";
        let rug_fuzz_1 = "Encoding not found";
        let v3 = Encoding::for_label(rug_fuzz_0).expect(rug_fuzz_1);
        let p0 = v3;
        shift_jis::ShiftJisEncoder::new(p0);
        let _rug_ed_tests_rug_59_rrrruuuugggg_test_shift_jis_encoder_new = 0;
    }
}
#[cfg(test)]
mod tests_rug_60 {
    use super::*;
    use crate::{Encoding, Encoder, shift_jis};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_60_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 5;
        let p0 = &shift_jis::ShiftJisEncoder;
        let p1: usize = rug_fuzz_0;
        p0.max_buffer_length_from_utf16_without_replacement(p1);
        let _rug_ed_tests_rug_60_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_61 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_61_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let p0 = shift_jis::ShiftJisEncoder;
        let p1: usize = rug_fuzz_0;
        debug_assert_eq!(
            p0.max_buffer_length_from_utf8_without_replacement(p1), Some(11)
        );
        let _rug_ed_tests_rug_61_rrrruuuugggg_test_rug = 0;
    }
}
