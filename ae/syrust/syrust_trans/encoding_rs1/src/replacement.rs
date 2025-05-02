use super::*;
use variant::*;
pub struct ReplacementDecoder {
    emitted: bool,
}
impl ReplacementDecoder {
    pub fn new() -> VariantDecoder {
        VariantDecoder::Replacement(ReplacementDecoder {
            emitted: false,
        })
    }
    pub fn max_utf16_buffer_length(&self, _u16_length: usize) -> Option<usize> {
        Some(1)
    }
    pub fn max_utf8_buffer_length_without_replacement(
        &self,
        _byte_length: usize,
    ) -> Option<usize> {
        Some(3)
    }
    pub fn max_utf8_buffer_length(&self, _byte_length: usize) -> Option<usize> {
        Some(3)
    }
    pub fn decode_to_utf16_raw(
        &mut self,
        src: &[u8],
        dst: &mut [u16],
        _last: bool,
    ) -> (DecoderResult, usize, usize) {
        if self.emitted || src.is_empty() {
            (DecoderResult::InputEmpty, src.len(), 0)
        } else if dst.is_empty() {
            (DecoderResult::OutputFull, 0, 0)
        } else {
            self.emitted = true;
            (DecoderResult::Malformed(1, 0), 1, 0)
        }
    }
    pub fn decode_to_utf8_raw(
        &mut self,
        src: &[u8],
        dst: &mut [u8],
        _last: bool,
    ) -> (DecoderResult, usize, usize) {
        if self.emitted || src.is_empty() {
            (DecoderResult::InputEmpty, src.len(), 0)
        } else if dst.len() < 3 {
            (DecoderResult::OutputFull, 0, 0)
        } else {
            self.emitted = true;
            (DecoderResult::Malformed(1, 0), 1, 0)
        }
    }
}
#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use super::super::*;
    fn decode_replacement(bytes: &[u8], expect: &str) {
        decode_without_padding(REPLACEMENT, bytes, expect);
    }
    fn encode_replacement(string: &str, expect: &[u8]) {
        encode(REPLACEMENT, string, expect);
    }
    #[test]
    fn test_replacement_decode() {
        decode_replacement(b"", "");
        decode_replacement(b"A", "\u{FFFD}");
        decode_replacement(b"AB", "\u{FFFD}");
    }
    #[test]
    fn test_replacement_encode() {
        encode_replacement("", b"");
        assert_eq!(REPLACEMENT.new_encoder().encoding(), UTF_8);
        encode_replacement("\u{1F4A9}\u{2603}", "\u{1F4A9}\u{2603}".as_bytes());
    }
}
#[cfg(test)]
mod tests_rug_452 {
    use super::*;
    use crate::replacement::{ReplacementDecoder, VariantDecoder};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_452_rrrruuuugggg_test_rug = 0;
        let _decoder = ReplacementDecoder::new();
        let _rug_ed_tests_rug_452_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_453 {
    use super::*;
    use crate::replacement;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_453_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = false;
        let rug_fuzz_1 = 10;
        let mut p0 = replacement::ReplacementDecoder {
            emitted: rug_fuzz_0,
        };
        let p1: usize = rug_fuzz_1;
        debug_assert_eq!(p0.max_utf16_buffer_length(p1).unwrap(), 1);
        let _rug_ed_tests_rug_453_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_454 {
    use super::*;
    use crate::replacement;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_454_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = false;
        let rug_fuzz_1 = 10;
        let mut p0 = replacement::ReplacementDecoder {
            emitted: rug_fuzz_0,
        };
        let mut p1: usize = rug_fuzz_1;
        p0.max_utf8_buffer_length_without_replacement(p1);
        let _rug_ed_tests_rug_454_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_455 {
    use super::*;
    use crate::replacement;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_455_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = false;
        let rug_fuzz_1 = 10;
        let mut p0 = replacement::ReplacementDecoder {
            emitted: rug_fuzz_0,
        };
        let p1: usize = rug_fuzz_1;
        p0.max_utf8_buffer_length(p1);
        let _rug_ed_tests_rug_455_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_456 {
    use super::*;
    use crate::replacement;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_456_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = false;
        let rug_fuzz_1 = 0xF0;
        let rug_fuzz_2 = 0x9F;
        let rug_fuzz_3 = 0x98;
        let rug_fuzz_4 = 0x8A;
        let rug_fuzz_5 = 0u16;
        let rug_fuzz_6 = true;
        let mut p0 = replacement::ReplacementDecoder {
            emitted: rug_fuzz_0,
        };
        let p1 = &[rug_fuzz_1, rug_fuzz_2, rug_fuzz_3, rug_fuzz_4];
        let mut p2 = &mut [rug_fuzz_5; 4];
        let p3 = rug_fuzz_6;
        let result = p0.decode_to_utf16_raw(p1, p2, p3);
        debug_assert_eq!(result, (DecoderResult::Malformed(1, 0), 1, 0));
        let _rug_ed_tests_rug_456_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_457 {
    use super::*;
    use crate::replacement;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_457_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = false;
        let rug_fuzz_1 = b"Hello";
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = false;
        let mut p0 = replacement::ReplacementDecoder {
            emitted: rug_fuzz_0,
        };
        let p1 = rug_fuzz_1;
        let mut p2 = [rug_fuzz_2; 10];
        let p3 = rug_fuzz_3;
        p0.decode_to_utf8_raw(p1, &mut p2, p3);
        let _rug_ed_tests_rug_457_rrrruuuugggg_test_rug = 0;
    }
}
