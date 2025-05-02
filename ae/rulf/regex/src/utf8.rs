/// A few elementary UTF-8 encoding and decoding functions used by the matching
/// engines.
///
/// In an ideal world, the matching engines operate on `&str` and we can just
/// lean on the standard library for all our UTF-8 needs. However, to support
/// byte based regexes (that can match on arbitrary bytes which may contain
/// UTF-8), we need to be capable of searching and decoding UTF-8 on a `&[u8]`.
/// The standard library doesn't really recognize this use case, so we have
/// to build it out ourselves.
///
/// Should this be factored out into a separate crate? It seems independently
/// useful. There are other crates that already exist (e.g., `utf-8`) that have
/// overlapping use cases. Not sure what to do.
use std::char;
const TAG_CONT: u8 = 0b1000_0000;
const TAG_TWO: u8 = 0b1100_0000;
const TAG_THREE: u8 = 0b1110_0000;
const TAG_FOUR: u8 = 0b1111_0000;
/// Returns the smallest possible index of the next valid UTF-8 sequence
/// starting after `i`.
pub fn next_utf8(text: &[u8], i: usize) -> usize {
    let b = match text.get(i) {
        None => return i + 1,
        Some(&b) => b,
    };
    let inc = if b <= 0x7F {
        1
    } else if b <= 0b110_11111 {
        2
    } else if b <= 0b1110_1111 {
        3
    } else {
        4
    };
    i + inc
}
/// Decode a single UTF-8 sequence into a single Unicode codepoint from `src`.
///
/// If no valid UTF-8 sequence could be found, then `None` is returned.
/// Otherwise, the decoded codepoint and the number of bytes read is returned.
/// The number of bytes read (for a valid UTF-8 sequence) is guaranteed to be
/// 1, 2, 3 or 4.
///
/// Note that a UTF-8 sequence is invalid if it is incorrect UTF-8, encodes a
/// codepoint that is out of range (surrogate codepoints are out of range) or
/// is not the shortest possible UTF-8 sequence for that codepoint.
#[inline]
pub fn decode_utf8(src: &[u8]) -> Option<(char, usize)> {
    let b0 = match src.get(0) {
        None => return None,
        Some(&b) if b <= 0x7F => return Some((b as char, 1)),
        Some(&b) => b,
    };
    match b0 {
        0b110_00000..=0b110_11111 => {
            if src.len() < 2 {
                return None;
            }
            let b1 = src[1];
            if 0b11_000000 & b1 != TAG_CONT {
                return None;
            }
            let cp = ((b0 & !TAG_TWO) as u32) << 6 | ((b1 & !TAG_CONT) as u32);
            match cp {
                0x80..=0x7FF => char::from_u32(cp).map(|cp| (cp, 2)),
                _ => None,
            }
        }
        0b1110_0000..=0b1110_1111 => {
            if src.len() < 3 {
                return None;
            }
            let (b1, b2) = (src[1], src[2]);
            if 0b11_000000 & b1 != TAG_CONT {
                return None;
            }
            if 0b11_000000 & b2 != TAG_CONT {
                return None;
            }
            let cp = ((b0 & !TAG_THREE) as u32) << 12 | ((b1 & !TAG_CONT) as u32) << 6
                | ((b2 & !TAG_CONT) as u32);
            match cp {
                0x800..=0xFFFF => char::from_u32(cp).map(|cp| (cp, 3)),
                _ => None,
            }
        }
        0b11110_000..=0b11110_111 => {
            if src.len() < 4 {
                return None;
            }
            let (b1, b2, b3) = (src[1], src[2], src[3]);
            if 0b11_000000 & b1 != TAG_CONT {
                return None;
            }
            if 0b11_000000 & b2 != TAG_CONT {
                return None;
            }
            if 0b11_000000 & b3 != TAG_CONT {
                return None;
            }
            let cp = ((b0 & !TAG_FOUR) as u32) << 18 | ((b1 & !TAG_CONT) as u32) << 12
                | ((b2 & !TAG_CONT) as u32) << 6 | ((b3 & !TAG_CONT) as u32);
            match cp {
                0x10000..=0x10FFFF => char::from_u32(cp).map(|cp| (cp, 4)),
                _ => None,
            }
        }
        _ => None,
    }
}
/// Like `decode_utf8`, but decodes the last UTF-8 sequence in `src` instead
/// of the first.
pub fn decode_last_utf8(src: &[u8]) -> Option<(char, usize)> {
    if src.is_empty() {
        return None;
    }
    let mut start = src.len() - 1;
    if src[start] <= 0x7F {
        return Some((src[start] as char, 1));
    }
    while start > src.len().saturating_sub(4) {
        start -= 1;
        if is_start_byte(src[start]) {
            break;
        }
    }
    match decode_utf8(&src[start..]) {
        None => None,
        Some((_, n)) if n < src.len() - start => None,
        Some((cp, n)) => Some((cp, n)),
    }
}
fn is_start_byte(b: u8) -> bool {
    b & 0b11_000000 != 0b1_0000000
}
#[cfg(test)]
mod tests {
    use std::str;
    use quickcheck::quickcheck;
    use super::{decode_last_utf8, decode_utf8, TAG_CONT, TAG_FOUR, TAG_THREE, TAG_TWO};
    #[test]
    fn prop_roundtrip() {
        fn p(given_cp: char) -> bool {
            let mut tmp = [0; 4];
            let encoded_len = given_cp.encode_utf8(&mut tmp).len();
            let (got_cp, got_len) = decode_utf8(&tmp[..encoded_len]).unwrap();
            encoded_len == got_len && given_cp == got_cp
        }
        quickcheck(p as fn(char) -> bool)
    }
    #[test]
    fn prop_roundtrip_last() {
        fn p(given_cp: char) -> bool {
            let mut tmp = [0; 4];
            let encoded_len = given_cp.encode_utf8(&mut tmp).len();
            let (got_cp, got_len) = decode_last_utf8(&tmp[..encoded_len]).unwrap();
            encoded_len == got_len && given_cp == got_cp
        }
        quickcheck(p as fn(char) -> bool)
    }
    #[test]
    fn prop_encode_matches_std() {
        fn p(cp: char) -> bool {
            let mut got = [0; 4];
            let n = cp.encode_utf8(&mut got).len();
            let expected = cp.to_string();
            &got[..n] == expected.as_bytes()
        }
        quickcheck(p as fn(char) -> bool)
    }
    #[test]
    fn prop_decode_matches_std() {
        fn p(given_cp: char) -> bool {
            let mut tmp = [0; 4];
            let n = given_cp.encode_utf8(&mut tmp).len();
            let (got_cp, _) = decode_utf8(&tmp[..n]).unwrap();
            let expected_cp = str::from_utf8(&tmp[..n]).unwrap().chars().next().unwrap();
            got_cp == expected_cp
        }
        quickcheck(p as fn(char) -> bool)
    }
    #[test]
    fn prop_decode_last_matches_std() {
        fn p(given_cp: char) -> bool {
            let mut tmp = [0; 4];
            let n = given_cp.encode_utf8(&mut tmp).len();
            let (got_cp, _) = decode_last_utf8(&tmp[..n]).unwrap();
            let expected_cp = str::from_utf8(&tmp[..n])
                .unwrap()
                .chars()
                .rev()
                .next()
                .unwrap();
            got_cp == expected_cp
        }
        quickcheck(p as fn(char) -> bool)
    }
    #[test]
    fn reject_invalid() {
        assert_eq!(decode_utf8(& [0xFF]), None);
        assert_eq!(decode_utf8(& [0xED, 0xA0, 0x81]), None);
        assert_eq!(decode_utf8(& [0xD4, 0xC2]), None);
        assert_eq!(decode_utf8(& [0xC3]), None);
        assert_eq!(decode_utf8(& [0xEF, 0xBF]), None);
        assert_eq!(decode_utf8(& [0xF4, 0x8F, 0xBF]), None);
        assert_eq!(decode_utf8(& [TAG_TWO, TAG_CONT | b'a']), None);
        assert_eq!(decode_utf8(& [TAG_THREE, TAG_CONT, TAG_CONT | b'a']), None);
        assert_eq!(
            decode_utf8(& [TAG_FOUR, TAG_CONT, TAG_CONT, TAG_CONT | b'a',]), None
        );
    }
    #[test]
    fn reject_invalid_last() {
        assert_eq!(decode_last_utf8(& [0xFF]), None);
        assert_eq!(decode_last_utf8(& [0xED, 0xA0, 0x81]), None);
        assert_eq!(decode_last_utf8(& [0xC3]), None);
        assert_eq!(decode_last_utf8(& [0xEF, 0xBF]), None);
        assert_eq!(decode_last_utf8(& [0xF4, 0x8F, 0xBF]), None);
        assert_eq!(decode_last_utf8(& [TAG_TWO, TAG_CONT | b'a']), None);
        assert_eq!(decode_last_utf8(& [TAG_THREE, TAG_CONT, TAG_CONT | b'a',]), None);
        assert_eq!(
            decode_last_utf8(& [TAG_FOUR, TAG_CONT, TAG_CONT, TAG_CONT | b'a',]), None
        );
    }
}
#[cfg(test)]
mod tests_llm_16_752 {
    use super::*;
    use crate::*;
    use crate::utf8::decode_utf8;
    #[test]
    fn test_decode_last_utf8() {
        let _rug_st_tests_llm_16_752_rrrruuuugggg_test_decode_last_utf8 = 0;
        let rug_fuzz_0 = 195;
        let rug_fuzz_1 = 160;
        let rug_fuzz_2 = 195;
        let rug_fuzz_3 = 175;
        let rug_fuzz_4 = 195;
        let rug_fuzz_5 = 180;
        let rug_fuzz_6 = 195;
        let rug_fuzz_7 = 170;
        let rug_fuzz_8 = 195;
        let rug_fuzz_9 = 160;
        let rug_fuzz_10 = 195;
        let rug_fuzz_11 = 175;
        let rug_fuzz_12 = 0;
        let rug_fuzz_13 = 0xF0;
        let rug_fuzz_14 = 0x9F;
        let rug_fuzz_15 = 0x98;
        let rug_fuzz_16 = 0x81;
        let rug_fuzz_17 = 0xF0;
        let rug_fuzz_18 = 0x9F;
        let rug_fuzz_19 = 0x98;
        let rug_fuzz_20 = 0x81;
        let rug_fuzz_21 = 0xE2;
        let rug_fuzz_22 = 0x82;
        let rug_fuzz_23 = 0xF0;
        let rug_fuzz_24 = 0x9F;
        let rug_fuzz_25 = 0x98;
        let rug_fuzz_26 = 0x81;
        let rug_fuzz_27 = 0xE2;
        let rug_fuzz_28 = 0x82;
        let rug_fuzz_29 = 0xAC;
        let rug_fuzz_30 = 0xF0;
        let rug_fuzz_31 = 0x9F;
        let rug_fuzz_32 = 0x98;
        let rug_fuzz_33 = 197;
        let rug_fuzz_34 = 191;
        debug_assert_eq!(decode_last_utf8(& []), None);
        let src2 = [
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
            rug_fuzz_6,
            rug_fuzz_7,
        ];
        debug_assert_eq!(decode_last_utf8(& src2), Some(('ª', 2)));
        let src3 = [rug_fuzz_8, rug_fuzz_9, rug_fuzz_10, rug_fuzz_11, rug_fuzz_12];
        debug_assert_eq!(decode_last_utf8(& src3), Some(('ª', 2)));
        let src4 = [rug_fuzz_13, rug_fuzz_14, rug_fuzz_15, rug_fuzz_16];
        debug_assert_eq!(decode_last_utf8(& src4), Some(('😁', 4)));
        let src5 = [
            rug_fuzz_17,
            rug_fuzz_18,
            rug_fuzz_19,
            rug_fuzz_20,
            rug_fuzz_21,
            rug_fuzz_22,
        ];
        debug_assert_eq!(decode_last_utf8(& src5), Some(('‚', 2)));
        let src6 = [
            rug_fuzz_23,
            rug_fuzz_24,
            rug_fuzz_25,
            rug_fuzz_26,
            rug_fuzz_27,
            rug_fuzz_28,
            rug_fuzz_29,
        ];
        debug_assert_eq!(decode_last_utf8(& src6), Some(('€', 3)));
        let src7 = [rug_fuzz_30, rug_fuzz_31, rug_fuzz_32];
        debug_assert_eq!(decode_last_utf8(& src7), None);
        let src8 = [rug_fuzz_33, rug_fuzz_34];
        debug_assert_eq!(decode_last_utf8(& src8), Some(('¿', 2)));
        let _rug_ed_tests_llm_16_752_rrrruuuugggg_test_decode_last_utf8 = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_753 {
    use super::*;
    use crate::*;
    #[test]
    fn test_decode_utf8() {
        let _rug_st_tests_llm_16_753_rrrruuuugggg_test_decode_utf8 = 0;
        let rug_fuzz_0 = 0x41;
        let rug_fuzz_1 = 0x80;
        let rug_fuzz_2 = 0xC2;
        let rug_fuzz_3 = 0xA2;
        let rug_fuzz_4 = 0xC2;
        let rug_fuzz_5 = 0xC2;
        let rug_fuzz_6 = 0x41;
        let rug_fuzz_7 = 0xE2;
        let rug_fuzz_8 = 0x82;
        let rug_fuzz_9 = 0xAC;
        let rug_fuzz_10 = 0xE2;
        let rug_fuzz_11 = 0x82;
        let rug_fuzz_12 = 0xE2;
        let rug_fuzz_13 = 0x82;
        let rug_fuzz_14 = 0x41;
        let rug_fuzz_15 = 0xF0;
        let rug_fuzz_16 = 0x9F;
        let rug_fuzz_17 = 0x8E;
        let rug_fuzz_18 = 0x80;
        let rug_fuzz_19 = 0xF0;
        let rug_fuzz_20 = 0x9F;
        let rug_fuzz_21 = 0x8E;
        let rug_fuzz_22 = 0xF0;
        let rug_fuzz_23 = 0x9F;
        let rug_fuzz_24 = 0x8E;
        let rug_fuzz_25 = 0x41;
        let rug_fuzz_26 = 0xF4;
        let rug_fuzz_27 = 0x8F;
        let rug_fuzz_28 = 0x8F;
        let rug_fuzz_29 = 0x8F;
        debug_assert_eq!(decode_utf8(& []), None);
        debug_assert_eq!(decode_utf8(& [rug_fuzz_0]), Some(('A', 1)));
        debug_assert_eq!(decode_utf8(& [rug_fuzz_1]), None);
        debug_assert_eq!(decode_utf8(& [rug_fuzz_2, rug_fuzz_3]), Some(('¢', 2)));
        debug_assert_eq!(decode_utf8(& [rug_fuzz_4]), None);
        debug_assert_eq!(decode_utf8(& [rug_fuzz_5, rug_fuzz_6]), None);
        debug_assert_eq!(
            decode_utf8(& [rug_fuzz_7, rug_fuzz_8, rug_fuzz_9]), Some(('€', 3))
        );
        debug_assert_eq!(decode_utf8(& [rug_fuzz_10, rug_fuzz_11]), None);
        debug_assert_eq!(decode_utf8(& [rug_fuzz_12, rug_fuzz_13, rug_fuzz_14]), None);
        debug_assert_eq!(
            decode_utf8(& [rug_fuzz_15, rug_fuzz_16, rug_fuzz_17, rug_fuzz_18]),
            Some(('🎀', 4))
        );
        debug_assert_eq!(decode_utf8(& [rug_fuzz_19, rug_fuzz_20, rug_fuzz_21]), None);
        debug_assert_eq!(
            decode_utf8(& [rug_fuzz_22, rug_fuzz_23, rug_fuzz_24, rug_fuzz_25]), None
        );
        debug_assert_eq!(
            decode_utf8(& [rug_fuzz_26, rug_fuzz_27, rug_fuzz_28, rug_fuzz_29]), None
        );
        let _rug_ed_tests_llm_16_753_rrrruuuugggg_test_decode_utf8 = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_754 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_start_byte() {
        let _rug_st_tests_llm_16_754_rrrruuuugggg_test_is_start_byte = 0;
        let rug_fuzz_0 = 0b0100_0000;
        let rug_fuzz_1 = 0b1000_0000;
        let rug_fuzz_2 = 0b1100_0000;
        let rug_fuzz_3 = 0b1110_0000;
        let rug_fuzz_4 = 0b1111_0000;
        debug_assert_eq!(is_start_byte(rug_fuzz_0), true);
        debug_assert_eq!(is_start_byte(rug_fuzz_1), false);
        debug_assert_eq!(is_start_byte(rug_fuzz_2), false);
        debug_assert_eq!(is_start_byte(rug_fuzz_3), false);
        debug_assert_eq!(is_start_byte(rug_fuzz_4), false);
        let _rug_ed_tests_llm_16_754_rrrruuuugggg_test_is_start_byte = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_756 {
    use super::*;
    use crate::*;
    use crate::utf8::next_utf8;
    #[test]
    fn test_next_utf8() {
        let _rug_st_tests_llm_16_756_rrrruuuugggg_test_next_utf8 = 0;
        let rug_fuzz_0 = b"hello";
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = b"hello";
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = b"hello";
        let rug_fuzz_5 = 2;
        let rug_fuzz_6 = b"hello";
        let rug_fuzz_7 = 3;
        let rug_fuzz_8 = b"hello";
        let rug_fuzz_9 = 4;
        let rug_fuzz_10 = b"hello";
        let rug_fuzz_11 = 5;
        let rug_fuzz_12 = b"hello";
        let rug_fuzz_13 = 6;
        let rug_fuzz_14 = b"\xF0\x9F\x8C\x8D hello";
        let rug_fuzz_15 = 0;
        let rug_fuzz_16 = b"\xF0\x9F\x8C\x8D hello";
        let rug_fuzz_17 = 1;
        let rug_fuzz_18 = b"\xF0\x9F\x8C\x8D hello";
        let rug_fuzz_19 = 2;
        let rug_fuzz_20 = b"\xF0\x9F\x8C\x8D hello";
        let rug_fuzz_21 = 3;
        let rug_fuzz_22 = b"\xF0\x9F\x8C\x8D hello";
        let rug_fuzz_23 = 4;
        let rug_fuzz_24 = b"\xF0\x9F\x8C\x8D hello";
        let rug_fuzz_25 = 5;
        let rug_fuzz_26 = b"\xF0\x9F\x8C\x8D hello";
        let rug_fuzz_27 = 6;
        let rug_fuzz_28 = b"\xF0\x9F\x8C\x8D hello";
        let rug_fuzz_29 = 7;
        let rug_fuzz_30 = b"\xF0\x9F\x8C\x8D hello";
        let rug_fuzz_31 = 8;
        let rug_fuzz_32 = b"\xF0\x9F\x8C\x8D hello";
        let rug_fuzz_33 = 9;
        let rug_fuzz_34 = b"\xF0\x9F\x8C\x8D hello";
        let rug_fuzz_35 = 10;
        debug_assert_eq!(next_utf8(rug_fuzz_0, rug_fuzz_1), 1);
        debug_assert_eq!(next_utf8(rug_fuzz_2, rug_fuzz_3), 2);
        debug_assert_eq!(next_utf8(rug_fuzz_4, rug_fuzz_5), 3);
        debug_assert_eq!(next_utf8(rug_fuzz_6, rug_fuzz_7), 4);
        debug_assert_eq!(next_utf8(rug_fuzz_8, rug_fuzz_9), 5);
        debug_assert_eq!(next_utf8(rug_fuzz_10, rug_fuzz_11), 6);
        debug_assert_eq!(next_utf8(rug_fuzz_12, rug_fuzz_13), 6);
        debug_assert_eq!(next_utf8(rug_fuzz_14, rug_fuzz_15), 4);
        debug_assert_eq!(next_utf8(rug_fuzz_16, rug_fuzz_17), 5);
        debug_assert_eq!(next_utf8(rug_fuzz_18, rug_fuzz_19), 6);
        debug_assert_eq!(next_utf8(rug_fuzz_20, rug_fuzz_21), 7);
        debug_assert_eq!(next_utf8(rug_fuzz_22, rug_fuzz_23), 8);
        debug_assert_eq!(next_utf8(rug_fuzz_24, rug_fuzz_25), 8);
        debug_assert_eq!(next_utf8(rug_fuzz_26, rug_fuzz_27), 8);
        debug_assert_eq!(next_utf8(rug_fuzz_28, rug_fuzz_29), 8);
        debug_assert_eq!(next_utf8(rug_fuzz_30, rug_fuzz_31), 9);
        debug_assert_eq!(next_utf8(rug_fuzz_32, rug_fuzz_33), 10);
        debug_assert_eq!(next_utf8(rug_fuzz_34, rug_fuzz_35), 10);
        let _rug_ed_tests_llm_16_756_rrrruuuugggg_test_next_utf8 = 0;
    }
}
