use super::*;
use data::*;
use handles::*;
use variant::*;
use super::in_inclusive_range32;
pub struct Big5Decoder {
    lead: Option<u8>,
}
impl Big5Decoder {
    pub fn new() -> VariantDecoder {
        VariantDecoder::Big5(Big5Decoder { lead: None })
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
        checked_add(1, self.plus_one_if_lead(byte_length))
    }
    pub fn max_utf8_buffer_length_without_replacement(
        &self,
        byte_length: usize,
    ) -> Option<usize> {
        checked_add(2, checked_mul(2, self.plus_one_if_lead(byte_length)))
    }
    pub fn max_utf8_buffer_length(&self, byte_length: usize) -> Option<usize> {
        checked_add(3, checked_mul(3, self.plus_one_if_lead(byte_length)))
    }
    ascii_compatible_two_byte_decoder_functions!(
        { let non_ascii_minus_offset = non_ascii.wrapping_sub(0x81); if
        non_ascii_minus_offset > (0xFE - 0x81) { return (DecoderResult::Malformed(1, 0),
        source.consumed(), handle.written()); } non_ascii_minus_offset }, { let mut
        trail_minus_offset = byte.wrapping_sub(0x40); if trail_minus_offset > (0x7E -
        0x40) { let trail_minus_range_start = byte.wrapping_sub(0xA1); if
        trail_minus_range_start > (0xFE - 0xA1) { if byte < 0x80 { return
        (DecoderResult::Malformed(1, 0), unread_handle_trail.unread(), handle.written());
        } return (DecoderResult::Malformed(2, 0), unread_handle_trail.consumed(), handle
        .written()); } trail_minus_offset = byte - 0x62; } let pointer =
        lead_minus_offset as usize * 157usize + trail_minus_offset as usize; let
        rebased_pointer = pointer.wrapping_sub(942); let low_bits =
        big5_low_bits(rebased_pointer); if low_bits == 0 { match pointer { 1133 => {
        handle.write_big5_combination(0x00CAu16, 0x0304u16) } 1135 => { handle
        .write_big5_combination(0x00CAu16, 0x030Cu16) } 1164 => { handle
        .write_big5_combination(0x00EAu16, 0x0304u16) } 1166 => { handle
        .write_big5_combination(0x00EAu16, 0x030Cu16) } _ => { if byte < 0x80 { return
        (DecoderResult::Malformed(1, 0), unread_handle_trail.unread(), handle.written());
        } return (DecoderResult::Malformed(2, 0), unread_handle_trail.consumed(), handle
        .written()); } } } else if big5_is_astral(rebased_pointer) { handle
        .write_astral(u32::from(low_bits) | 0x20000u32) } else { handle
        .write_bmp_excl_ascii(low_bits) } }, self, non_ascii, byte, lead_minus_offset,
        unread_handle_trail, source, handle, 'outermost,
        copy_ascii_from_check_space_astral, check_space_astral, false
    );
}
pub struct Big5Encoder;
impl Big5Encoder {
    pub fn new(encoding: &'static Encoding) -> Encoder {
        Encoder::new(encoding, VariantEncoder::Big5(Big5Encoder))
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
    ascii_compatible_encoder_functions!(
        { if let Some((lead, trail)) = big5_level1_hanzi_encode(bmp) { handle
        .write_two(lead, trail) } else { let pointer = if let Some(pointer) =
        big5_box_encode(bmp) { pointer } else if let Some(pointer) =
        big5_other_encode(bmp) { pointer } else { return
        (EncoderResult::unmappable_from_bmp(bmp), source.consumed(), handle.written(),);
        }; let lead = pointer / 157 + 0x81; let remainder = pointer % 157; let trail = if
        remainder < 0x3F { remainder + 0x40 } else { remainder + 0x62 }; handle
        .write_two(lead as u8, trail as u8) } }, { if in_inclusive_range32(astral as u32,
        0x2008A, 0x2F8A6) { if let Some(rebased_pointer) = big5_astral_encode(astral as
        u16) { let lead = rebased_pointer / 157 + 0x87; let remainder = rebased_pointer %
        157; let trail = if remainder < 0x3F { remainder + 0x40 } else { remainder + 0x62
        }; handle.write_two(lead as u8, trail as u8) } else { return
        (EncoderResult::Unmappable(astral), source.consumed(), handle.written(),); } }
        else { return (EncoderResult::Unmappable(astral), source.consumed(), handle
        .written(),); } }, bmp, astral, self, source, handle,
        copy_ascii_to_check_space_two, check_space_two, false
    );
}
#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use super::super::*;
    fn decode_big5(bytes: &[u8], expect: &str) {
        decode(BIG5, bytes, expect);
    }
    fn encode_big5(string: &str, expect: &[u8]) {
        encode(BIG5, string, expect);
    }
    #[test]
    fn test_big5_decode() {
        decode_big5(b"", &"");
        decode_big5(&[0x61u8, 0x62u8], &"\u{0061}\u{0062}");
        decode_big5(&[0x87u8, 0x40u8], &"\u{43F0}");
        decode_big5(&[0xFEu8, 0xFEu8], &"\u{79D4}");
        decode_big5(&[0xFEu8, 0xFDu8], &"\u{2910D}");
        decode_big5(&[0x88u8, 0x62u8], &"\u{00CA}\u{0304}");
        decode_big5(&[0x88u8, 0x64u8], &"\u{00CA}\u{030C}");
        decode_big5(&[0x88u8, 0x66u8], &"\u{00CA}");
        decode_big5(&[0x88u8, 0xA3u8], &"\u{00EA}\u{0304}");
        decode_big5(&[0x88u8, 0xA5u8], &"\u{00EA}\u{030C}");
        decode_big5(&[0x88u8, 0xA7u8], &"\u{00EA}");
        decode_big5(&[0x99u8, 0xD4u8], &"\u{8991}");
        decode_big5(&[0x99u8, 0xD5u8], &"\u{27967}");
        decode_big5(&[0x99u8, 0xD6u8], &"\u{8A29}");
        decode_big5(&[0x61u8, 0x87u8, 0x40u8, 0x62u8], &"\u{0061}\u{43F0}\u{0062}");
        decode_big5(&[0x61u8, 0xFEu8, 0xFEu8, 0x62u8], &"\u{0061}\u{79D4}\u{0062}");
        decode_big5(&[0x61u8, 0xFEu8, 0xFDu8, 0x62u8], &"\u{0061}\u{2910D}\u{0062}");
        decode_big5(
            &[0x61u8, 0x88u8, 0x62u8, 0x62u8],
            &"\u{0061}\u{00CA}\u{0304}\u{0062}",
        );
        decode_big5(
            &[0x61u8, 0x88u8, 0x64u8, 0x62u8],
            &"\u{0061}\u{00CA}\u{030C}\u{0062}",
        );
        decode_big5(&[0x61u8, 0x88u8, 0x66u8, 0x62u8], &"\u{0061}\u{00CA}\u{0062}");
        decode_big5(
            &[0x61u8, 0x88u8, 0xA3u8, 0x62u8],
            &"\u{0061}\u{00EA}\u{0304}\u{0062}",
        );
        decode_big5(
            &[0x61u8, 0x88u8, 0xA5u8, 0x62u8],
            &"\u{0061}\u{00EA}\u{030C}\u{0062}",
        );
        decode_big5(&[0x61u8, 0x88u8, 0xA7u8, 0x62u8], &"\u{0061}\u{00EA}\u{0062}");
        decode_big5(&[0x61u8, 0x99u8, 0xD4u8, 0x62u8], &"\u{0061}\u{8991}\u{0062}");
        decode_big5(&[0x61u8, 0x99u8, 0xD5u8, 0x62u8], &"\u{0061}\u{27967}\u{0062}");
        decode_big5(&[0x61u8, 0x99u8, 0xD6u8, 0x62u8], &"\u{0061}\u{8A29}\u{0062}");
        decode_big5(&[0x80u8, 0x61u8], &"\u{FFFD}\u{0061}");
        decode_big5(&[0xFFu8, 0x61u8], &"\u{FFFD}\u{0061}");
        decode_big5(&[0xFEu8, 0x39u8], &"\u{FFFD}\u{0039}");
        decode_big5(&[0x87u8, 0x66u8], &"\u{FFFD}\u{0066}");
        decode_big5(&[0x81u8, 0x40u8], &"\u{FFFD}\u{0040}");
        decode_big5(&[0x61u8, 0x81u8], &"\u{0061}\u{FFFD}");
    }
    #[test]
    fn test_big5_encode() {
        encode_big5("", b"");
        encode_big5("\u{0061}\u{0062}", b"\x61\x62");
        encode_big5("\u{9EA6}\u{0061}", b"&#40614;\x61");
        encode_big5("\u{2626B}\u{0061}", b"&#156267;\x61");
        encode_big5("\u{3000}", b"\xA1\x40");
        encode_big5("\u{20AC}", b"\xA3\xE1");
        encode_big5("\u{4E00}", b"\xA4\x40");
        encode_big5("\u{27607}", b"\xC8\xA4");
        encode_big5("\u{FFE2}", b"\xC8\xCD");
        encode_big5("\u{79D4}", b"\xFE\xFE");
        encode_big5("\u{2603}\u{0061}", b"&#9731;\x61");
        encode_big5("\u{203B5}", b"\xFD\x6A");
        encode_big5("\u{25605}", b"\xFE\x46");
        encode_big5("\u{2550}", b"\xF9\xF9");
    }
    #[test]
    fn test_big5_decode_all() {
        let input = include_bytes!("test_data/big5_in.txt");
        let expectation = include_str!("test_data/big5_in_ref.txt");
        let (cow, had_errors) = BIG5.decode_without_bom_handling(input);
        assert!(had_errors, "Should have had errors.");
        assert_eq!(& cow[..], expectation);
    }
    #[test]
    fn test_big5_encode_all() {
        let input = include_str!("test_data/big5_out.txt");
        let expectation = include_bytes!("test_data/big5_out_ref.txt");
        let (cow, encoding, had_errors) = BIG5.encode(input);
        assert!(! had_errors, "Should not have had errors.");
        assert_eq!(encoding, BIG5);
        assert_eq!(& cow[..], & expectation[..]);
    }
    #[test]
    fn test_big5_encode_from_two_low_surrogates() {
        let expectation = b"&#65533;&#65533;";
        let mut output = [0u8; 40];
        let mut encoder = BIG5.new_encoder();
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
mod tests_rug_401 {
    use super::*;
    use crate::big5::{Big5Decoder, VariantDecoder};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_401_rrrruuuugggg_test_rug = 0;
        <Big5Decoder>::new();
        let _rug_ed_tests_rug_401_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_404 {
    use super::*;
    use crate::big5;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = big5::Big5Decoder::new();
        let p1: usize = rug_fuzz_0;
        p0.max_utf16_buffer_length(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_405_prepare {
    use crate::big5;
    #[test]
    fn sample() {
        let _rug_st_tests_rug_405_prepare_rrrruuuugggg_sample = 0;
        let mut v53 = big5::Big5Decoder::new();
        let _rug_ed_tests_rug_405_prepare_rrrruuuugggg_sample = 0;
    }
}
#[cfg(test)]
mod tests_rug_405 {
    use crate::big5::Big5Decoder;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Big5Decoder::new();
        let p1: usize = rug_fuzz_0;
        p0.max_utf8_buffer_length_without_replacement(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_406 {
    use super::*;
    use crate::big5;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut decoder = big5::Big5Decoder::new();
        let byte_length: usize = rug_fuzz_0;
        decoder.max_utf8_buffer_length(byte_length);
             }
}
}
}    }
}
use crate::Encoding;
use crate::big5;
#[cfg(test)]
mod tests_rug_407 {
    use super::*;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1)) = <([u8; 6], &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        let v3 = Encoding::for_label(rug_fuzz_0).expect(rug_fuzz_1);
        let mut p0 = &v3;
        big5::Big5Encoder::new(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_408 {
    use super::*;
    use crate::{big5, Encoding};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0 = big5::Big5Encoder;
        let p1: usize = rug_fuzz_0;
        p0.max_buffer_length_from_utf16_without_replacement(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_409 {
    use super::*;
    use crate::big5::Big5Encoder;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Big5Encoder {};
        let mut p1: usize = rug_fuzz_0;
        debug_assert_eq!(
            p0.max_buffer_length_from_utf8_without_replacement(p1), Some(11)
        );
             }
}
}
}    }
}
