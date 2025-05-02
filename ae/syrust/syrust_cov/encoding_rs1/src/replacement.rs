// Copyright Mozilla Foundation. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use super::*;
use variant::*;

pub struct ReplacementDecoder {
    emitted: bool,
}

impl ReplacementDecoder {
    pub fn new() -> VariantDecoder {
        VariantDecoder::Replacement(ReplacementDecoder { emitted: false })
    }

    pub fn max_utf16_buffer_length(&self, _u16_length: usize) -> Option<usize> {
        Some(1)
    }

    pub fn max_utf8_buffer_length_without_replacement(&self, _byte_length: usize) -> Option<usize> {
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
        // Don't err if the input stream is empty. See
        // https://github.com/whatwg/encoding/issues/33
        if self.emitted || src.is_empty() {
            (DecoderResult::InputEmpty, src.len(), 0)
        } else if dst.is_empty() {
            // Make sure there's room for the replacement character.
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
        // Don't err if the input stream is empty. See
        // https://github.com/whatwg/encoding/issues/33
        if self.emitted || src.is_empty() {
            (DecoderResult::InputEmpty, src.len(), 0)
        } else if dst.len() < 3 {
            // Make sure there's room for the replacement character.
            (DecoderResult::OutputFull, 0, 0)
        } else {
            self.emitted = true;
            (DecoderResult::Malformed(1, 0), 1, 0)
        }
    }
}

// Any copyright to the test code below this comment is dedicated to the
// Public Domain. http://creativecommons.org/publicdomain/zero/1.0/

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
        // Empty
        encode_replacement("", b"");

        assert_eq!(REPLACEMENT.new_encoder().encoding(), UTF_8);
        encode_replacement("\u{1F4A9}\u{2603}", "\u{1F4A9}\u{2603}".as_bytes());
    }
}
#[cfg(test)]
mod tests_rug_452 {
    use super::*;
    use crate::{replacement::{ReplacementDecoder, VariantDecoder}};
    
    #[test]
    fn test_rug() {
        let _decoder = ReplacementDecoder::new();
    }
}
#[cfg(test)]
mod tests_rug_453 {
    use super::*;
    use crate::replacement;

    #[test]
    fn test_rug() {
        let mut p0 = replacement::ReplacementDecoder { emitted: false };
        let p1: usize = 10;

        assert_eq!(p0.max_utf16_buffer_length(p1).unwrap(), 1);
    }
}

#[cfg(test)]
mod tests_rug_454 {
    use super::*;
    use crate::replacement;

    #[test]
    fn test_rug() {
        let mut p0 = replacement::ReplacementDecoder { emitted: false };
        let mut p1: usize = 10;

        p0.max_utf8_buffer_length_without_replacement(p1);

    }
}#[cfg(test)]
mod tests_rug_455 {
    use super::*;
    use crate::replacement;

    #[test]
    fn test_rug() {
        let mut p0 = replacement::ReplacementDecoder { emitted: false };
        let p1: usize = 10;

        p0.max_utf8_buffer_length(p1);
    }
}
#[cfg(test)]
mod tests_rug_456 {
    use super::*;
    use crate::replacement;

    #[test]
    fn test_rug() {
        let mut p0 = replacement::ReplacementDecoder { emitted: false };
        let p1 = &[0xF0, 0x9F, 0x98, 0x8A]; // Sample data
        let mut p2 = &mut [0u16; 4]; // Sample data
        let p3 = true;

        let result = p0.decode_to_utf16_raw(p1, p2, p3);

        // Add assertions based on the expected behavior of the function
        assert_eq!(result, (DecoderResult::Malformed(1, 0), 1, 0));
    }
}
#[cfg(test)]
mod tests_rug_457 {
    use super::*;
    use crate::replacement;

    #[test]
    fn test_rug() {
        let mut p0 = replacement::ReplacementDecoder { emitted: false };
        let p1 = b"Hello";
        let mut p2 = [0; 10];
        let p3 = false;

        p0.decode_to_utf8_raw(p1, &mut p2, p3);

        // Add assertions here if needed
    }
}