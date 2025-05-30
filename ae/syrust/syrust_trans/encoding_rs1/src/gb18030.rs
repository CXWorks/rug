use super::*;
use data::*;
use handles::*;
use variant::*;
use super::in_inclusive_range16;
use super::in_range16;
enum Gb18030Pending {
    None,
    One(u8),
    Two(u8, u8),
    Three(u8, u8, u8),
}
impl Gb18030Pending {
    fn is_none(&self) -> bool {
        match *self {
            Gb18030Pending::None => true,
            _ => false,
        }
    }
    fn count(&self) -> usize {
        match *self {
            Gb18030Pending::None => 0,
            Gb18030Pending::One(_) => 1,
            Gb18030Pending::Two(_, _) => 2,
            Gb18030Pending::Three(_, _, _) => 3,
        }
    }
}
pub struct Gb18030Decoder {
    first: Option<u8>,
    second: Option<u8>,
    third: Option<u8>,
    pending: Gb18030Pending,
    pending_ascii: Option<u8>,
}
impl Gb18030Decoder {
    pub fn new() -> VariantDecoder {
        VariantDecoder::Gb18030(Gb18030Decoder {
            first: None,
            second: None,
            third: None,
            pending: Gb18030Pending::None,
            pending_ascii: None,
        })
    }
    pub fn in_neutral_state(&self) -> bool {
        self.first.is_none() && self.second.is_none() && self.third.is_none()
            && self.pending.is_none() && self.pending_ascii.is_none()
    }
    fn extra_from_state(&self, byte_length: usize) -> Option<usize> {
        byte_length
            .checked_add(
                self.pending.count()
                    + match self.first {
                        None => 0,
                        Some(_) => 1,
                    }
                    + match self.second {
                        None => 0,
                        Some(_) => 1,
                    }
                    + match self.third {
                        None => 0,
                        Some(_) => 1,
                    }
                    + match self.pending_ascii {
                        None => 0,
                        Some(_) => 1,
                    },
            )
    }
    pub fn max_utf16_buffer_length(&self, byte_length: usize) -> Option<usize> {
        checked_add(1, self.extra_from_state(byte_length))
    }
    pub fn max_utf8_buffer_length_without_replacement(
        &self,
        byte_length: usize,
    ) -> Option<usize> {
        self.max_utf8_buffer_length(byte_length)
    }
    pub fn max_utf8_buffer_length(&self, byte_length: usize) -> Option<usize> {
        checked_add(1, checked_mul(3, self.extra_from_state(byte_length)))
    }
    gb18030_decoder_functions!(
        { let non_ascii_minus_offset = non_ascii.wrapping_sub(0x81); if
        non_ascii_minus_offset > (0xFE - 0x81) { if non_ascii == 0x80 { handle
        .write_upper_bmp(0x20ACu16); continue 'outermost; } return
        (DecoderResult::Malformed(1, 0), source.consumed(), handle.written()); }
        non_ascii_minus_offset }, { if first_minus_offset >= 0x20 { let
        trail_minus_offset = second.wrapping_sub(0xA1); if trail_minus_offset <= (0xFE -
        0xA1) { let hanzi_lead = first_minus_offset.wrapping_sub(0x2F); if hanzi_lead <
        (0x77 - 0x2F) { let hanzi_pointer = mul_94(hanzi_lead) + trail_minus_offset as
        usize; let upper_bmp = GB2312_HANZI[hanzi_pointer]; handle
        .write_upper_bmp(upper_bmp) } else if first_minus_offset == 0x20 { let bmp =
        GB2312_SYMBOLS[trail_minus_offset as usize]; handle.write_bmp_excl_ascii(bmp) }
        else if first_minus_offset == 0x25 && ((trail_minus_offset.wrapping_sub(63) as
        usize) < GB2312_SYMBOLS_AFTER_GREEK.len()) { handle
        .write_bmp_excl_ascii(GB2312_SYMBOLS_AFTER_GREEK[trail_minus_offset
        .wrapping_sub(63) as usize]) } else if first_minus_offset == 0x27 &&
        (trail_minus_offset as usize) < GB2312_PINYIN.len() { handle
        .write_bmp_excl_ascii(GB2312_PINYIN[trail_minus_offset as usize]) } else if
        first_minus_offset > 0x76 { let pua = (0xE234 + mul_94(first_minus_offset - 0x77)
        + trail_minus_offset as usize) as u16; handle.write_upper_bmp(pua) } else { let
        bmp = gb2312_other_decode((mul_94(first_minus_offset - 0x21) +
        (trail_minus_offset as usize)) as u16); handle.write_bmp_excl_ascii(bmp) } } else
        { let mut trail_minus_offset = second.wrapping_sub(0x40); if trail_minus_offset >
        (0x7E - 0x40) { let trail_minus_range_start = second.wrapping_sub(0x80); if
        trail_minus_range_start > (0xA0 - 0x80) { if second < 0x80 { return
        (DecoderResult::Malformed(1, 0), unread_handle_second.unread(), handle
        .written()); } return (DecoderResult::Malformed(2, 0), unread_handle_second
        .consumed(), handle.written()); } trail_minus_offset = second - 0x41; } let
        left_lead = first_minus_offset - 0x20; let left_pointer = left_lead as usize *
        (190 - 94) + trail_minus_offset as usize; let gbk_left_ideograph_pointer =
        left_pointer.wrapping_sub((0x29 - 0x20) * (190 - 94)); if
        gbk_left_ideograph_pointer < (((0x7D - 0x29) * (190 - 94)) - 5) { let upper_bmp =
        gbk_left_ideograph_decode(gbk_left_ideograph_pointer as u16); handle
        .write_upper_bmp(upper_bmp) } else if left_pointer < ((0x29 - 0x20) * (190 - 94))
        { let bmp = gbk_other_decode(left_pointer as u16); handle
        .write_bmp_excl_ascii(bmp) } else { let bottom_pointer = left_pointer - (((0x7D -
        0x20) * (190 - 94)) - 5); let upper_bmp = GBK_BOTTOM[bottom_pointer]; handle
        .write_upper_bmp(upper_bmp) } } } else { let mut trail_minus_offset = second
        .wrapping_sub(0x40); if trail_minus_offset > (0x7E - 0x40) { let
        trail_minus_range_start = second.wrapping_sub(0x80); if trail_minus_range_start >
        (0xFE - 0x80) { if second < 0x80 { return (DecoderResult::Malformed(1, 0),
        unread_handle_second.unread(), handle.written()); } return
        (DecoderResult::Malformed(2, 0), unread_handle_second.consumed(), handle
        .written()); } trail_minus_offset = second - 0x41; } let pointer =
        first_minus_offset as usize * 190usize + trail_minus_offset as usize; let
        upper_bmp = gbk_top_ideograph_decode(pointer as u16); handle
        .write_upper_bmp(upper_bmp) } }, { let third_minus_offset = third
        .wrapping_sub(0x81); if third_minus_offset > (0xFE - 0x81) { self.pending_ascii =
        Some(second_minus_offset + 0x30); return (DecoderResult::Malformed(1, 1),
        unread_handle_third.unread(), handle.written()); } third_minus_offset }, { let
        fourth_minus_offset = fourth.wrapping_sub(0x30); if fourth_minus_offset > (0x39 -
        0x30) { self.pending_ascii = Some(second_minus_offset + 0x30); self.pending =
        Gb18030Pending::One(third_minus_offset); return (DecoderResult::Malformed(1, 2),
        unread_handle_fourth.unread(), handle.written()); } let pointer =
        (first_minus_offset as usize * (10 * 126 * 10)) + (second_minus_offset as usize *
        (10 * 126)) + (third_minus_offset as usize * 10) + fourth_minus_offset as usize;
        if pointer <= 39419 { if pointer == 7457 { handle.write_upper_bmp(0xE7C7) } else
        { handle.write_bmp_excl_ascii(gb18030_range_decode(pointer as u16)) } } else if
        pointer >= 189_000 && pointer <= 1_237_575 { handle.write_astral((pointer -
        (189_000usize - 0x1_0000usize)) as u32) } else { return
        (DecoderResult::Malformed(4, 0), unread_handle_fourth.consumed(), handle
        .written()); } }, self, non_ascii, first_minus_offset, second,
        second_minus_offset, unread_handle_second, third, third_minus_offset,
        unread_handle_third, fourth, fourth_minus_offset, unread_handle_fourth, source,
        handle, 'outermost
    );
}
fn gbk_encode_non_unified(bmp: u16) -> Option<(usize, usize)> {
    if in_inclusive_range16(bmp, 0x2014, 0x3017)
        || in_inclusive_range16(bmp, 0xFF04, 0xFFE1)
    {
        if let Some(pos) = position(&GB2312_SYMBOLS[..], bmp) {
            return Some((0xA1, pos + 0xA1));
        }
    }
    if in_range16(bmp, 0x3400, 0x4E00) {
        return position(&GBK_BOTTOM[21..100], bmp)
            .map(|pos| {
                (0xFE, pos + if pos < (0x3F - 16) { 0x40 + 16 } else { 0x41 + 16 })
            });
    }
    if in_range16(bmp, 0xF900, 0xFB00) {
        return position(&GBK_BOTTOM[0..21], bmp)
            .map(|pos| {
                if pos < 5 {
                    (0xFD, pos + (190 - 94 - 5 + 0x41))
                } else {
                    (0xFE, pos + (0x40 - 5))
                }
            });
    }
    if bmp < 0x02CA {
        if in_range16(bmp, 0x00E0, 0x0262) && bmp != 0x00F7 {
            if let Some(pos) = position(&GB2312_PINYIN[..], bmp) {
                return Some((0xA8, pos + 0xA1));
            }
        } else if in_inclusive_range16(bmp, 0x00A4, 0x00F7)
            || in_inclusive_range16(bmp, 0x02C7, 0x02C9)
        {
            if let Some(pos) = position(&GB2312_SYMBOLS[3..(0xAC - 0x60)], bmp) {
                return Some((0xA1, pos + 0xA1 + 3));
            }
        }
        return None;
    }
    if bmp >= 0xE794 {
        if let Some(pos) = position(&GB2312_SYMBOLS_AFTER_GREEK[..], bmp) {
            return Some((0xA6, pos + (0x9F - 0x60 + 0xA1)));
        }
    } else if bmp == 0x1E3F {
        return Some((0xA8, 0x7B - 0x60 + 0xA1));
    } else if in_range16(bmp, 0xA000, 0xD800) {
        return None;
    }
    if let Some(other_pointer) = gb2312_other_encode(bmp) {
        let other_lead = other_pointer as usize / 94;
        let other_trail = other_pointer as usize % 94;
        return Some((0xA2 + other_lead, 0xA1 + other_trail));
    }
    if in_range16(bmp, 0x02DA, 0x2010) {
        return None;
    }
    if let Some(other_pointer) = gbk_other_encode(bmp) {
        let other_lead = other_pointer as usize / (190 - 94);
        let other_trail = other_pointer as usize % (190 - 94);
        let offset = if other_trail < 0x3F { 0x40 } else { 0x41 };
        return Some((other_lead + (0x81 + 0x20), other_trail + offset));
    }
    if in_inclusive_range16(bmp, 0x2E81, 0x2ECA)
        || in_inclusive_range16(bmp, 0xE816, 0xE864)
    {
        if let Some(pos) = position(&GBK_BOTTOM[21..], bmp) {
            let trail = pos + 16;
            let offset = if trail < 0x3F { 0x40 } else { 0x41 };
            return Some((0xFE, trail + offset));
        }
    }
    let bmp_minus_gb2312_bottom_pua = bmp.wrapping_sub(0xE234);
    if bmp_minus_gb2312_bottom_pua <= (0xE4C5 - 0xE234) {
        let pua_lead = bmp_minus_gb2312_bottom_pua as usize / 94;
        let pua_trail = bmp_minus_gb2312_bottom_pua as usize % 94;
        return Some((0x81 + 0x77 + pua_lead, 0xA1 + pua_trail));
    }
    let bmp_minus_pua_between_hanzi = bmp.wrapping_sub(0xE810);
    if bmp_minus_pua_between_hanzi < 5 {
        return Some((0x81 + 0x56, 0xFF - 5 + bmp_minus_pua_between_hanzi as usize));
    }
    None
}
#[cfg(not(feature = "fast-gb-hanzi-encode"))]
#[inline(always)]
fn encode_hanzi(bmp: u16, _: u16) -> (u8, u8) {
    if let Some((lead, trail)) = gb2312_level1_hanzi_encode(bmp) {
        (lead, trail)
    } else if let Some(hanzi_pointer) = gb2312_level2_hanzi_encode(bmp) {
        let hanzi_lead = (hanzi_pointer / 94) + (0xD8);
        let hanzi_trail = (hanzi_pointer % 94) + 0xA1;
        (hanzi_lead as u8, hanzi_trail as u8)
    } else {
        let (lead, gbk_trail) = if bmp < 0x72DC {
            let pointer = gbk_top_ideograph_encode(bmp) as usize;
            let lead = (pointer / 190) + 0x81;
            let gbk_trail = pointer % 190;
            (lead, gbk_trail)
        } else {
            let gbk_left_ideograph_pointer = gbk_left_ideograph_encode(bmp) as usize;
            let lead = (gbk_left_ideograph_pointer / (190 - 94)) + (0x81 + 0x29);
            let gbk_trail = gbk_left_ideograph_pointer % (190 - 94);
            (lead, gbk_trail)
        };
        let offset = if gbk_trail < 0x3F { 0x40 } else { 0x41 };
        (lead as u8, (gbk_trail + offset) as u8)
    }
}
#[cfg(feature = "fast-gb-hanzi-encode")]
#[inline(always)]
fn encode_hanzi(_: u16, bmp_minus_unified_start: u16) -> (u8, u8) {
    gbk_hanzi_encode(bmp_minus_unified_start)
}
pub struct Gb18030Encoder {
    extended: bool,
}
impl Gb18030Encoder {
    pub fn new(encoding: &'static Encoding, extended_range: bool) -> Encoder {
        Encoder::new(
            encoding,
            VariantEncoder::Gb18030(Gb18030Encoder {
                extended: extended_range,
            }),
        )
    }
    pub fn max_buffer_length_from_utf16_without_replacement(
        &self,
        u16_length: usize,
    ) -> Option<usize> {
        if self.extended {
            u16_length.checked_mul(4)
        } else {
            checked_add(2, u16_length.checked_mul(2))
        }
    }
    pub fn max_buffer_length_from_utf8_without_replacement(
        &self,
        byte_length: usize,
    ) -> Option<usize> {
        if self.extended {
            checked_add(2, byte_length.checked_mul(2))
        } else {
            byte_length.checked_add(3)
        }
    }
    ascii_compatible_encoder_functions!(
        { let bmp_minus_unified_start = bmp.wrapping_sub(0x4E00); if
        bmp_minus_unified_start < (0x9FA6 - 0x4E00) { let (lead, trail) =
        encode_hanzi(bmp, bmp_minus_unified_start); handle.write_two(lead, trail) } else
        if bmp == 0xE5E5 { return (EncoderResult::unmappable_from_bmp(bmp), source
        .consumed(), handle.written(),); } else if bmp == 0x20AC && ! self.extended {
        handle.write_one(0x80u8) } else { match gbk_encode_non_unified(bmp) { Some((lead,
        trail)) => handle.write_two(lead as u8, trail as u8), None => { if ! self
        .extended { return (EncoderResult::unmappable_from_bmp(bmp), source.consumed(),
        handle.written(),); } let range_pointer = gb18030_range_encode(bmp); let first =
        range_pointer / (10 * 126 * 10); let rem_first = range_pointer % (10 * 126 * 10);
        let second = rem_first / (10 * 126); let rem_second = rem_first % (10 * 126); let
        third = rem_second / 10; let fourth = rem_second % 10; handle.write_four((first +
        0x81) as u8, (second + 0x30) as u8, (third + 0x81) as u8, (fourth + 0x30) as u8,)
        } } } }, { if ! self.extended { return (EncoderResult::Unmappable(astral), source
        .consumed(), handle.written(),); } let range_pointer = astral as usize +
        (189_000usize - 0x1_0000usize); let first = range_pointer / (10 * 126 * 10); let
        rem_first = range_pointer % (10 * 126 * 10); let second = rem_first / (10 * 126);
        let rem_second = rem_first % (10 * 126); let third = rem_second / 10; let fourth
        = rem_second % 10; handle.write_four((first + 0x81) as u8, (second + 0x30) as u8,
        (third + 0x81) as u8, (fourth + 0x30) as u8,) }, bmp, astral, self, source,
        handle, copy_ascii_to_check_space_four, check_space_four, false
    );
}
#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use super::super::*;
    fn decode_gb18030(bytes: &[u8], expect: &str) {
        decode(GB18030, bytes, expect);
    }
    fn encode_gb18030(string: &str, expect: &[u8]) {
        encode(GB18030, string, expect);
    }
    fn encode_gbk(string: &str, expect: &[u8]) {
        encode(GBK, string, expect);
    }
    #[test]
    fn test_gb18030_decode() {
        decode_gb18030(b"", &"");
        decode_gb18030(b"\x61\x62", "\u{0061}\u{0062}");
        decode_gb18030(b"\x80", "\u{20AC}");
        decode_gb18030(b"\xA2\xE3", "\u{20AC}");
        decode_gb18030(b"\x81\x40", "\u{4E02}");
        decode_gb18030(b"\x81\x7E", "\u{4E8A}");
        decode_gb18030(b"\x81\x7F", "\u{FFFD}\u{007F}");
        decode_gb18030(b"\x81\x80", "\u{4E90}");
        decode_gb18030(b"\x81\xFE", "\u{4FA2}");
        decode_gb18030(b"\xFE\x40", "\u{FA0C}");
        decode_gb18030(b"\xFE\x7E", "\u{E843}");
        decode_gb18030(b"\xFE\x7F", "\u{FFFD}\u{007F}");
        decode_gb18030(b"\xFE\x80", "\u{4723}");
        decode_gb18030(b"\xFE\xFE", "\u{E4C5}");
        decode_gb18030(b"\xA3\xA0", "\u{3000}");
        decode_gb18030(b"\xA1\xA1", "\u{3000}");
        decode_gb18030(b"\xFF\x40", "\u{FFFD}\u{0040}");
        decode_gb18030(b"\xE3\xFF\x9A\x33", "\u{FFFD}\u{FFFD}");
        decode_gb18030(b"\xFF\x32\x9A\x33", "\u{FFFD}\u{0032}\u{FFFD}");
        decode_gb18030(b"\xFF\x40\x00", "\u{FFFD}\u{0040}\u{0000}");
        decode_gb18030(b"\xE3\xFF\x9A\x33\x00", "\u{FFFD}\u{FFFD}\u{0033}\u{0000}");
        decode_gb18030(
            b"\xFF\x32\x9A\x33\x00",
            "\u{FFFD}\u{0032}\u{FFFD}\u{0033}\u{0000}",
        );
        decode_gb18030(b"\x81\x30\x81\x30", "\u{0080}");
        decode_gb18030(b"\x81\x35\xF4\x37", "\u{E7C7}");
        decode_gb18030(b"\x81\x37\xA3\x30", "\u{2603}");
        decode_gb18030(b"\x94\x39\xDA\x33", "\u{1F4A9}");
        decode_gb18030(b"\xE3\x32\x9A\x35", "\u{10FFFF}");
        decode_gb18030(b"\xE3\x32\x9A\x36\x81\x30", "\u{FFFD}\u{FFFD}");
        decode_gb18030(b"\xE3\x32\x9A\x36\x81\x40", "\u{FFFD}\u{4E02}");
        decode_gb18030(b"\xE3\x32\x9A", "\u{FFFD}");
        decode_gb18030(b"\xE3\x32\x9A\x00", "\u{FFFD}\u{0032}\u{FFFD}\u{0000}");
    }
    #[test]
    fn test_gb18030_encode() {
        encode_gb18030("", b"");
        encode_gb18030("\u{0061}\u{0062}", b"\x61\x62");
        encode_gb18030("\u{20AC}", b"\xA2\xE3");
        encode_gb18030("\u{4E02}", b"\x81\x40");
        encode_gb18030("\u{4E8A}", b"\x81\x7E");
        encode_gb18030("\u{4E90}", b"\x81\x80");
        encode_gb18030("\u{4FA2}", b"\x81\xFE");
        encode_gb18030("\u{FA0C}", b"\xFE\x40");
        encode_gb18030("\u{E843}", b"\xFE\x7E");
        encode_gb18030("\u{4723}", b"\xFE\x80");
        encode_gb18030("\u{E4C5}", b"\xFE\xFE");
        encode_gb18030("\u{E5E5}", b"&#58853;");
        encode_gb18030("\u{3000}", b"\xA1\xA1");
        encode_gb18030("\u{0080}", b"\x81\x30\x81\x30");
        encode_gb18030("\u{E7C7}", b"\x81\x35\xF4\x37");
        encode_gb18030("\u{2603}", b"\x81\x37\xA3\x30");
        encode_gb18030("\u{1F4A9}", b"\x94\x39\xDA\x33");
        encode_gb18030("\u{10FFFF}", b"\xE3\x32\x9A\x35");
        encode_gb18030("\u{00F7}", b"\xA1\xC2");
    }
    #[test]
    fn test_gbk_encode() {
        encode_gbk("", b"");
        encode_gbk("\u{0061}\u{0062}", b"\x61\x62");
        encode_gbk("\u{20AC}", b"\x80");
        encode_gbk("\u{4E02}", b"\x81\x40");
        encode_gbk("\u{4E8A}", b"\x81\x7E");
        encode_gbk("\u{4E90}", b"\x81\x80");
        encode_gbk("\u{4FA2}", b"\x81\xFE");
        encode_gbk("\u{FA0C}", b"\xFE\x40");
        encode_gbk("\u{E843}", b"\xFE\x7E");
        encode_gbk("\u{4723}", b"\xFE\x80");
        encode_gbk("\u{E4C5}", b"\xFE\xFE");
        encode_gbk("\u{E5E5}", b"&#58853;");
        encode_gbk("\u{3000}", b"\xA1\xA1");
        encode_gbk("\u{0080}", b"&#128;");
        encode_gbk("\u{E7C7}", b"&#59335;");
        encode_gbk("\u{2603}", b"&#9731;");
        encode_gbk("\u{1F4A9}", b"&#128169;");
        encode_gbk("\u{10FFFF}", b"&#1114111;");
        encode_gbk("\u{00F7}", b"\xA1\xC2");
    }
    #[test]
    fn test_gb18030_decode_all() {
        let input = include_bytes!("test_data/gb18030_in.txt");
        let expectation = include_str!("test_data/gb18030_in_ref.txt");
        let (cow, had_errors) = GB18030.decode_without_bom_handling(input);
        assert!(! had_errors, "Should not have had errors.");
        assert_eq!(& cow[..], expectation);
    }
    #[test]
    fn test_gb18030_encode_all() {
        let input = include_str!("test_data/gb18030_out.txt");
        let expectation = include_bytes!("test_data/gb18030_out_ref.txt");
        let (cow, encoding, had_errors) = GB18030.encode(input);
        assert!(! had_errors, "Should not have had errors.");
        assert_eq!(encoding, GB18030);
        assert_eq!(& cow[..], & expectation[..]);
    }
    #[test]
    fn test_gb18030_encode_from_utf16_max_length() {
        let mut output = [0u8; 20];
        let mut encoder = GB18030.new_encoder();
        {
            let needed = encoder
                .max_buffer_length_from_utf16_without_replacement(1)
                .unwrap();
            let (result, read, written) = encoder
                .encode_from_utf16_without_replacement(
                    &[0x3000],
                    &mut output[..needed],
                    true,
                );
            assert_eq!(result, EncoderResult::InputEmpty);
            assert_eq!(read, 1);
            assert_eq!(written, 2);
            assert_eq!(output[0], 0xA1);
            assert_eq!(output[1], 0xA1);
        }
    }
}
#[cfg(test)]
mod tests_rug_25 {
    use super::*;
    #[test]
    fn test_gbk_encode_non_unified() {
        let _rug_st_tests_rug_25_rrrruuuugggg_test_gbk_encode_non_unified = 0;
        let rug_fuzz_0 = 0x2015;
        let p0: u16 = rug_fuzz_0;
        crate::gb18030::gbk_encode_non_unified(p0);
        let _rug_ed_tests_rug_25_rrrruuuugggg_test_gbk_encode_non_unified = 0;
    }
}
#[cfg(test)]
mod tests_rug_26 {
    use super::*;
    use crate::gb18030::{
        encode_hanzi, gb2312_level1_hanzi_encode, gb2312_level2_hanzi_encode,
        gbk_top_ideograph_encode, gbk_left_ideograph_encode,
    };
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_26_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0x4E00;
        let rug_fuzz_1 = 0x0000;
        let mut p0: u16 = rug_fuzz_0;
        let mut p1: u16 = rug_fuzz_1;
        encode_hanzi(p0, p1);
        let _rug_ed_tests_rug_26_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_27 {
    use super::*;
    use gb18030::Gb18030Pending;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_27_rrrruuuugggg_test_rug = 0;
        let p0: Gb18030Pending = Gb18030Pending::None;
        debug_assert_eq!(p0.is_none(), true);
        let _rug_ed_tests_rug_27_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_28 {
    use super::*;
    use crate::gb18030::Gb18030Pending;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_28_rrrruuuugggg_test_rug = 0;
        let mut p0 = Gb18030Pending::None;
        Gb18030Pending::count(&p0);
        let _rug_ed_tests_rug_28_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_29 {
    use super::*;
    use crate::gb18030;
    #[test]
    fn test_new() {
        let _rug_st_tests_rug_29_rrrruuuugggg_test_new = 0;
        let _ = gb18030::Gb18030Decoder::new();
        let _rug_ed_tests_rug_29_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_rug_32 {
    use super::*;
    use crate::gb18030;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_32_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 16;
        let mut p0 = gb18030::Gb18030Decoder::new();
        let p1: usize = rug_fuzz_0;
        p0.max_utf16_buffer_length(p1);
        let _rug_ed_tests_rug_32_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_33 {
    use super::*;
    use crate::gb18030;
    #[test]
    fn test_max_utf8_buffer_length_without_replacement() {
        let _rug_st_tests_rug_33_rrrruuuugggg_test_max_utf8_buffer_length_without_replacement = 0;
        let rug_fuzz_0 = 10;
        let mut decoder = gb18030::Gb18030Decoder::new();
        let byte_length: usize = rug_fuzz_0;
        decoder.max_utf8_buffer_length_without_replacement(byte_length);
        let _rug_ed_tests_rug_33_rrrruuuugggg_test_max_utf8_buffer_length_without_replacement = 0;
    }
}
#[cfg(test)]
mod tests_rug_34 {
    use crate::gb18030::Gb18030Decoder;
    use crate::checked_add;
    use crate::checked_mul;
    #[test]
    fn test_max_utf8_buffer_length() {
        let _rug_st_tests_rug_34_rrrruuuugggg_test_max_utf8_buffer_length = 0;
        let rug_fuzz_0 = 10;
        let mut p0 = Gb18030Decoder::new();
        let p1: usize = rug_fuzz_0;
        p0.max_utf8_buffer_length(p1);
        let _rug_ed_tests_rug_34_rrrruuuugggg_test_max_utf8_buffer_length = 0;
    }
}
use crate::{Encoding, Encoder};
#[cfg(test)]
mod tests_rug_35 {
    use super::*;
    use crate::gb18030;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_35_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"euc-jp";
        let rug_fuzz_1 = "Encoding not found";
        let rug_fuzz_2 = true;
        let v3 = Encoding::for_label(rug_fuzz_0).expect(rug_fuzz_1);
        let extended_range = rug_fuzz_2;
        gb18030::Gb18030Encoder::new(&v3, extended_range);
        let _rug_ed_tests_rug_35_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_36 {
    use super::*;
    #[test]
    fn test_max_buffer_length_from_utf16_without_replacement() {
        let _rug_st_tests_rug_36_rrrruuuugggg_test_max_buffer_length_from_utf16_without_replacement = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = 10;
        let rug_fuzz_2 = 40;
        let encoder = gb18030::Gb18030Encoder {
            extended: rug_fuzz_0,
        };
        let u16_length: usize = rug_fuzz_1;
        let result = encoder
            .max_buffer_length_from_utf16_without_replacement(u16_length);
        debug_assert_eq!(Some(rug_fuzz_2), result);
        let _rug_ed_tests_rug_36_rrrruuuugggg_test_max_buffer_length_from_utf16_without_replacement = 0;
    }
}
#[cfg(test)]
mod tests_rug_37 {
    use super::*;
    use crate::gb18030;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_37_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = 10;
        let mut p0 = gb18030::Gb18030Encoder {
            extended: rug_fuzz_0,
        };
        let mut p1: usize = rug_fuzz_1;
        p0.max_buffer_length_from_utf8_without_replacement(p1);
        let _rug_ed_tests_rug_37_rrrruuuugggg_test_rug = 0;
    }
}
