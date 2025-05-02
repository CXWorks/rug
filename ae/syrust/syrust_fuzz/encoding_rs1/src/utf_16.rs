use super::*;
use handles::*;
use variant::*;
pub struct Utf16Decoder {
    lead_surrogate: u16,
    lead_byte: Option<u8>,
    be: bool,
    pending_bmp: bool,
}
impl Utf16Decoder {
    pub fn new(big_endian: bool) -> VariantDecoder {
        VariantDecoder::Utf16(Utf16Decoder {
            lead_surrogate: 0,
            lead_byte: None,
            be: big_endian,
            pending_bmp: false,
        })
    }
    pub fn additional_from_state(&self) -> usize {
        1 + if self.lead_byte.is_some() { 1 } else { 0 }
            + if self.lead_surrogate == 0 { 0 } else { 2 }
    }
    pub fn max_utf16_buffer_length(&self, byte_length: usize) -> Option<usize> {
        checked_add(
            1,
            checked_div(byte_length.checked_add(self.additional_from_state()), 2),
        )
    }
    pub fn max_utf8_buffer_length_without_replacement(
        &self,
        byte_length: usize,
    ) -> Option<usize> {
        checked_add(
            1,
            checked_mul(
                3,
                checked_div(byte_length.checked_add(self.additional_from_state()), 2),
            ),
        )
    }
    pub fn max_utf8_buffer_length(&self, byte_length: usize) -> Option<usize> {
        checked_add(
            1,
            checked_mul(
                3,
                checked_div(byte_length.checked_add(self.additional_from_state()), 2),
            ),
        )
    }
    decoder_functions!(
        { if self.pending_bmp { match dest.check_space_bmp() { Space::Full(_) => { return
        (DecoderResult::OutputFull, 0, 0); } Space::Available(destination_handle) => {
        destination_handle.write_bmp(self.lead_surrogate); self.pending_bmp = false; self
        .lead_surrogate = 0; } } } }, { if self.lead_byte.is_none() && self
        .lead_surrogate == 0 { if let Some((read, written)) = if self.be { dest
        .copy_utf16_from::< BigEndian > (& mut source) } else { dest.copy_utf16_from::<
        LittleEndian > (& mut source) } { return (DecoderResult::Malformed(2, 0), read,
        written); } } }, { debug_assert!(! self.pending_bmp); if self.lead_surrogate != 0
        || self.lead_byte.is_some() { match dest.check_space_bmp() { Space::Full(_) => {
        return (DecoderResult::OutputFull, 0, 0); } Space::Available(_) => { if self
        .lead_surrogate != 0 { self.lead_surrogate = 0; match self.lead_byte { None => {
        return (DecoderResult::Malformed(2, 0), src_consumed, dest.written(),); } Some(_)
        => { self.lead_byte = None; return (DecoderResult::Malformed(3, 0), src_consumed,
        dest.written(),); } } } debug_assert!(self.lead_byte.is_some()); self.lead_byte =
        None; return (DecoderResult::Malformed(1, 0), src_consumed, dest.written()); } }
        } }, { match self.lead_byte { None => { self.lead_byte = Some(b); continue; }
        Some(lead) => { self.lead_byte = None; let code_unit = if self.be {
        u16::from(lead) << 8 | u16::from(b) } else { u16::from(b) << 8 | u16::from(lead)
        }; let high_bits = code_unit & 0xFC00u16; if high_bits == 0xD800u16 { if self
        .lead_surrogate != 0 { self.lead_surrogate = code_unit as u16; return
        (DecoderResult::Malformed(2, 2), unread_handle.consumed(), destination_handle
        .written(),); } self.lead_surrogate = code_unit; continue; } if high_bits ==
        0xDC00u16 { if self.lead_surrogate == 0 { return (DecoderResult::Malformed(2, 0),
        unread_handle.consumed(), destination_handle.written(),); } destination_handle
        .write_surrogate_pair(self.lead_surrogate, code_unit); self.lead_surrogate = 0;
        continue; } if self.lead_surrogate != 0 { self.lead_surrogate = code_unit; self
        .pending_bmp = true; return (DecoderResult::Malformed(2, 2), unread_handle
        .consumed(), destination_handle.written(),); } destination_handle
        .write_bmp(code_unit); continue; } } }, self, src_consumed, dest, source, b,
        destination_handle, unread_handle, check_space_astral
    );
}
#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use super::super::*;
    fn decode_utf_16le(bytes: &[u8], expect: &str) {
        decode_without_padding(UTF_16LE, bytes, expect);
    }
    fn decode_utf_16be(bytes: &[u8], expect: &str) {
        decode_without_padding(UTF_16BE, bytes, expect);
    }
    fn encode_utf_16le(string: &str, expect: &[u8]) {
        encode(UTF_16LE, string, expect);
    }
    fn encode_utf_16be(string: &str, expect: &[u8]) {
        encode(UTF_16BE, string, expect);
    }
    #[test]
    fn test_utf_16_decode() {
        decode_utf_16le(b"", "");
        decode_utf_16be(b"", "");
        decode_utf_16le(b"\x61\x00\x62\x00", "\u{0061}\u{0062}");
        decode_utf_16be(b"\x00\x61\x00\x62", "\u{0061}\u{0062}");
        decode_utf_16le(b"\xFE\xFF\x00\x61\x00\x62", "\u{0061}\u{0062}");
        decode_utf_16be(b"\xFF\xFE\x61\x00\x62\x00", "\u{0061}\u{0062}");
        decode_utf_16le(b"\x61\x00\x62", "\u{0061}\u{FFFD}");
        decode_utf_16be(b"\x00\x61\x00", "\u{0061}\u{FFFD}");
        decode_utf_16le(b"\x3D\xD8\xA9", "\u{FFFD}");
        decode_utf_16be(b"\xD8\x3D\xDC", "\u{FFFD}");
        decode_utf_16le(b"\x3D\xD8\xA9\xDC\x03\x26", "\u{1F4A9}\u{2603}");
        decode_utf_16be(b"\xD8\x3D\xDC\xA9\x26\x03", "\u{1F4A9}\u{2603}");
        decode_utf_16le(b"\xA9\xDC\x03\x26", "\u{FFFD}\u{2603}");
        decode_utf_16be(b"\xDC\xA9\x26\x03", "\u{FFFD}\u{2603}");
        decode_utf_16le(b"\x3D\xD8\x03\x26", "\u{FFFD}\u{2603}");
        decode_utf_16be(b"\xD8\x3D\x26\x03", "\u{FFFD}\u{2603}");
        let long_le = b"\x00\x00\x00\x00\x00\x00\x00\x00\x3D\xD8\xA9\xDC\x00\x00\x00\x00\x00\x00\x00\x00\x3D\xD8\x00\x00\x00\x00\x00\x00\x00\x00\xA9\xDC\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x3D\xD8\xFF\x00\x00\x00\x00\x00\x00\x00\x00\x3D\xD8\xA9\xDC\x00\x00\x00\x00\x00\x00\x00\x00\x3D\xD8\x00\x00\x00\x00\x00\x00\x00\x00\xA9\xDC\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x3D\xD8";
        let long_be = b"\x00\x00\x00\x00\x00\x00\x00\x00\xD8\x3D\xDC\xA9\x00\x00\x00\x00\x00\x00\x00\x00\xD8\x3D\x00\x00\x00\x00\x00\x00\x00\x00\xDC\xA9\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\xD8\x3D\xFF\x00\x00\x00\x00\x00\x00\x00\x00\xD8\x3D\xDC\xA9\x00\x00\x00\x00\x00\x00\x00\x00\xD8\x3D\x00\x00\x00\x00\x00\x00\x00\x00\xDC\xA9\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\xD8\x3D";
        let long_expect = "\x00\x00\x00\x00\u{1F4A9}\x00\x00\x00\x00\u{FFFD}\x00\x00\x00\x00\u{FFFD}\x00\x00\x00\x00\x00\x00\x00\x00\u{FFFD}";
        decode_utf_16le(&long_le[..long_le.len() / 2], long_expect);
        decode_utf_16be(&long_be[..long_be.len() / 2], long_expect);
        decode_utf_16le(&long_le[long_le.len() / 2 + 1..], long_expect);
        decode_utf_16be(&long_be[long_be.len() / 2 + 1..], long_expect);
    }
    #[test]
    fn test_utf_16_encode() {
        encode_utf_16be("", b"");
        encode_utf_16le("", b"");
        assert_eq!(UTF_16LE.new_encoder().encoding(), UTF_8);
        assert_eq!(UTF_16BE.new_encoder().encoding(), UTF_8);
        encode_utf_16le("\u{1F4A9}\u{2603}", "\u{1F4A9}\u{2603}".as_bytes());
        encode_utf_16be("\u{1F4A9}\u{2603}", "\u{1F4A9}\u{2603}".as_bytes());
    }
    #[test]
    fn test_utf_16be_decode_one_by_one() {
        let input = b"\x00\x61\x00\xE4\x26\x03\xD8\x3D\xDC\xA9";
        let mut output = [0u16; 20];
        let mut decoder = UTF_16BE.new_decoder();
        for b in input.chunks(1) {
            assert_eq!(b.len(), 1);
            let needed = decoder.max_utf16_buffer_length(b.len()).unwrap();
            let (result, read, _, had_errors) = decoder
                .decode_to_utf16(b, &mut output[..needed], false);
            assert_eq!(result, CoderResult::InputEmpty);
            assert_eq!(read, 1);
            assert!(! had_errors);
        }
    }
    #[test]
    fn test_utf_16le_decode_one_by_one() {
        let input = b"\x61\x00\xE4\x00\x03\x26\x3D\xD8\xA9\xDC";
        let mut output = [0u16; 20];
        let mut decoder = UTF_16LE.new_decoder();
        for b in input.chunks(1) {
            assert_eq!(b.len(), 1);
            let needed = decoder.max_utf16_buffer_length(b.len()).unwrap();
            let (result, read, _, had_errors) = decoder
                .decode_to_utf16(b, &mut output[..needed], false);
            assert_eq!(result, CoderResult::InputEmpty);
            assert_eq!(read, 1);
            assert!(! had_errors);
        }
    }
    #[test]
    fn test_utf_16be_decode_three_at_a_time() {
        let input = b"\x00\xE4\x26\x03\xD8\x3D\xDC\xA9\x00\x61\x00\xE4";
        let mut output = [0u16; 20];
        let mut decoder = UTF_16BE.new_decoder();
        for b in input.chunks(3) {
            assert_eq!(b.len(), 3);
            let needed = decoder.max_utf16_buffer_length(b.len()).unwrap();
            let (result, read, _, had_errors) = decoder
                .decode_to_utf16(b, &mut output[..needed], false);
            assert_eq!(result, CoderResult::InputEmpty);
            assert_eq!(read, b.len());
            assert!(! had_errors);
        }
    }
    #[test]
    fn test_utf_16le_decode_three_at_a_time() {
        let input = b"\xE4\x00\x03\x26\x3D\xD8\xA9\xDC\x61\x00\xE4\x00";
        let mut output = [0u16; 20];
        let mut decoder = UTF_16LE.new_decoder();
        for b in input.chunks(3) {
            assert_eq!(b.len(), 3);
            let needed = decoder.max_utf16_buffer_length(b.len()).unwrap();
            let (result, read, _, had_errors) = decoder
                .decode_to_utf16(b, &mut output[..needed], false);
            assert_eq!(result, CoderResult::InputEmpty);
            assert_eq!(read, b.len());
            assert!(! had_errors);
        }
    }
    #[test]
    fn test_utf_16le_decode_bom_prefixed_split_byte_pair() {
        let mut output = [0u16; 20];
        let mut decoder = UTF_16LE.new_decoder();
        {
            let needed = decoder.max_utf16_buffer_length(1).unwrap();
            let (result, read, written, had_errors) = decoder
                .decode_to_utf16(b"\xFF", &mut output[..needed], false);
            assert_eq!(result, CoderResult::InputEmpty);
            assert_eq!(read, 1);
            assert_eq!(written, 0);
            assert!(! had_errors);
        }
        {
            let needed = decoder.max_utf16_buffer_length(1).unwrap();
            let (result, read, written, had_errors) = decoder
                .decode_to_utf16(b"\xFD", &mut output[..needed], true);
            assert_eq!(result, CoderResult::InputEmpty);
            assert_eq!(read, 1);
            assert_eq!(written, 1);
            assert!(! had_errors);
            assert_eq!(output[0], 0xFDFF);
        }
    }
    #[test]
    fn test_utf_16be_decode_bom_prefixed_split_byte_pair() {
        let mut output = [0u16; 20];
        let mut decoder = UTF_16BE.new_decoder();
        {
            let needed = decoder.max_utf16_buffer_length(1).unwrap();
            let (result, read, written, had_errors) = decoder
                .decode_to_utf16(b"\xFE", &mut output[..needed], false);
            assert_eq!(result, CoderResult::InputEmpty);
            assert_eq!(read, 1);
            assert_eq!(written, 0);
            assert!(! had_errors);
        }
        {
            let needed = decoder.max_utf16_buffer_length(1).unwrap();
            let (result, read, written, had_errors) = decoder
                .decode_to_utf16(b"\xFD", &mut output[..needed], true);
            assert_eq!(result, CoderResult::InputEmpty);
            assert_eq!(read, 1);
            assert_eq!(written, 1);
            assert!(! had_errors);
            assert_eq!(output[0], 0xFEFD);
        }
    }
    #[test]
    fn test_utf_16le_decode_bom_prefix() {
        let mut output = [0u16; 20];
        let mut decoder = UTF_16LE.new_decoder();
        {
            let needed = decoder.max_utf16_buffer_length(1).unwrap();
            let (result, read, written, had_errors) = decoder
                .decode_to_utf16(b"\xFF", &mut output[..needed], true);
            assert_eq!(result, CoderResult::InputEmpty);
            assert_eq!(read, 1);
            assert_eq!(written, 1);
            assert!(had_errors);
            assert_eq!(output[0], 0xFFFD);
        }
    }
    #[test]
    fn test_utf_16be_decode_bom_prefix() {
        let mut output = [0u16; 20];
        let mut decoder = UTF_16BE.new_decoder();
        {
            let needed = decoder.max_utf16_buffer_length(1).unwrap();
            let (result, read, written, had_errors) = decoder
                .decode_to_utf16(b"\xFE", &mut output[..needed], true);
            assert_eq!(result, CoderResult::InputEmpty);
            assert_eq!(read, 1);
            assert_eq!(written, 1);
            assert!(had_errors);
            assert_eq!(output[0], 0xFFFD);
        }
    }
    #[test]
    fn test_utf_16le_decode_near_end() {
        let mut output = [0u8; 4];
        let mut decoder = UTF_16LE.new_decoder();
        {
            let (result, read, written, had_errors) = decoder
                .decode_to_utf8(&[0x03], &mut output[..], false);
            assert_eq!(result, CoderResult::InputEmpty);
            assert_eq!(read, 1);
            assert_eq!(written, 0);
            assert!(! had_errors);
            assert_eq!(output[0], 0x0);
        }
        {
            let (result, read, written, had_errors) = decoder
                .decode_to_utf8(&[0x26, 0x03, 0x26], &mut output[..], false);
            assert_eq!(result, CoderResult::OutputFull);
            assert_eq!(read, 1);
            assert_eq!(written, 3);
            assert!(! had_errors);
            assert_eq!(output[0], 0xE2);
            assert_eq!(output[1], 0x98);
            assert_eq!(output[2], 0x83);
            assert_eq!(output[3], 0x00);
        }
    }
    #[test]
    fn test_utf_16be_decode_near_end() {
        let mut output = [0u8; 4];
        let mut decoder = UTF_16BE.new_decoder();
        {
            let (result, read, written, had_errors) = decoder
                .decode_to_utf8(&[0x26], &mut output[..], false);
            assert_eq!(result, CoderResult::InputEmpty);
            assert_eq!(read, 1);
            assert_eq!(written, 0);
            assert!(! had_errors);
            assert_eq!(output[0], 0x0);
        }
        {
            let (result, read, written, had_errors) = decoder
                .decode_to_utf8(&[0x03, 0x26, 0x03], &mut output[..], false);
            assert_eq!(result, CoderResult::OutputFull);
            assert_eq!(read, 1);
            assert_eq!(written, 3);
            assert!(! had_errors);
            assert_eq!(output[0], 0xE2);
            assert_eq!(output[1], 0x98);
            assert_eq!(output[2], 0x83);
            assert_eq!(output[3], 0x00);
        }
    }
}
#[cfg(test)]
mod tests_rug_470 {
    use super::*;
    use crate::utf_16::{Utf16Decoder, VariantDecoder};
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: bool = rug_fuzz_0;
        Utf16Decoder::new(p0);
             }
});    }
}
#[cfg(test)]
mod tests_rug_472 {
    use super::*;
    use crate::utf_16::Utf16Decoder;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(bool, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Utf16Decoder::new(rug_fuzz_0);
        let p1: usize = rug_fuzz_1;
        debug_assert_eq!(p0.max_utf16_buffer_length(p1), Some(51));
             }
});    }
}
#[cfg(test)]
mod tests_rug_473 {
    use crate::utf_16;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(bool, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = utf_16::Utf16Decoder::new(rug_fuzz_0);
        let p1: usize = rug_fuzz_1;
        p0.max_utf8_buffer_length_without_replacement(p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_474 {
    use super::*;
    use crate::utf_16::Utf16Decoder;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(bool, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Utf16Decoder::new(rug_fuzz_0);
        let p1: usize = rug_fuzz_1;
        p0.max_utf8_buffer_length(p1);
             }
});    }
}
