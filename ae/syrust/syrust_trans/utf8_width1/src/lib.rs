/*!
# UTF-8 Width

To determine the width of a UTF-8 character by providing its first byte.

References: https://tools.ietf.org/html/rfc3629

## Examples

```rust
extern crate utf8_width;

assert_eq!(1, utf8_width::get_width(b'1'));
assert_eq!(3, utf8_width::get_width("中".as_bytes()[0]));
```

## Benchmark

```bash
cargo bench
```

*/
pub const MIN_0_1: u8 = 0x80;
pub const MAX_0_1: u8 = 0xC1;
pub const MIN_0_2: u8 = 0xF5;
pub const MAX_0_2: u8 = 0xFF;
pub const MIN_1: u8 = 0x00;
pub const MAX_1: u8 = 0x7F;
pub const MIN_2: u8 = 0xC2;
pub const MAX_2: u8 = 0xDF;
pub const MIN_3: u8 = 0xE0;
pub const MAX_3: u8 = 0xEF;
pub const MIN_4: u8 = 0xF0;
pub const MAX_4: u8 = 0xF4;
#[inline]
pub fn is_width_1(byte: u8) -> bool {
    byte <= MAX_1
}
#[inline]
pub fn is_width_2(byte: u8) -> bool {
    MIN_2 <= byte && byte <= MAX_2
}
#[inline]
pub fn is_width_3(byte: u8) -> bool {
    MIN_3 <= byte && byte <= MAX_3
}
#[inline]
pub fn is_width_4(byte: u8) -> bool {
    MIN_4 <= byte && byte <= MAX_4
}
#[inline]
pub fn is_width_0(byte: u8) -> bool {
    MIN_0_1 <= byte && byte <= MAX_0_1 || MIN_0_2 <= byte
}
/// Given a first byte, determines how many bytes are in this UTF-8 character. If the UTF-8 character is invalid, returns `0`, otherwise returns `1` ~ `4`,
#[inline]
pub fn get_width(byte: u8) -> usize {
    if is_width_1(byte) {
        1
    } else if is_width_2(byte) {
        2
    } else if byte <= MAX_3 {
        3
    } else if byte <= MAX_4 {
        4
    } else {
        0
    }
}
#[allow(clippy::missing_safety_doc)]
/// *Assume the input first byte is from a valid UTF-8 character.* Given a first byte, determines how many bytes are in this UTF-8 character. It returns `1` ~ `4`,
#[inline]
pub unsafe fn get_width_assume_valid(byte: u8) -> usize {
    if byte <= MAX_1 {
        1
    } else if byte <= MAX_2 {
        2
    } else if byte <= MAX_3 {
        3
    } else {
        4
    }
}
#[cfg(test)]
mod tests_rug_1 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_1_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 65;
        let mut p0: u8 = rug_fuzz_0;
        debug_assert_eq!(crate ::is_width_1(p0), true);
        let _rug_ed_tests_rug_1_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_2 {
    use super::*;
    #[test]
    fn test_is_width_2() {
        let _rug_st_tests_rug_2_rrrruuuugggg_test_is_width_2 = 0;
        let rug_fuzz_0 = 0b10101010;
        let rug_fuzz_1 = true;
        let mut p0: u8 = rug_fuzz_0;
        debug_assert_eq!(rug_fuzz_1, is_width_2(p0));
        let _rug_ed_tests_rug_2_rrrruuuugggg_test_is_width_2 = 0;
    }
}
#[cfg(test)]
mod tests_rug_3 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_3_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0b11111010;
        let rug_fuzz_1 = true;
        let p0: u8 = rug_fuzz_0;
        debug_assert_eq!(rug_fuzz_1, crate ::is_width_3(p0));
        let _rug_ed_tests_rug_3_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_4 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_4_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0x20;
        let rug_fuzz_1 = true;
        let p0: u8 = rug_fuzz_0;
        debug_assert_eq!(rug_fuzz_1, crate ::is_width_4(p0));
        let _rug_ed_tests_rug_4_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_5 {
    use super::*;
    #[test]
    fn test_is_width_0() {
        let _rug_st_tests_rug_5_rrrruuuugggg_test_is_width_0 = 0;
        let rug_fuzz_0 = 32;
        let rug_fuzz_1 = true;
        let p0: u8 = rug_fuzz_0;
        debug_assert_eq!(rug_fuzz_1, crate ::is_width_0(p0));
        let _rug_ed_tests_rug_5_rrrruuuugggg_test_is_width_0 = 0;
    }
}
#[cfg(test)]
mod tests_rug_6 {
    use super::*;
    #[test]
    fn test_get_width() {
        let _rug_st_tests_rug_6_rrrruuuugggg_test_get_width = 0;
        let rug_fuzz_0 = 0b11010011;
        let p0: u8 = rug_fuzz_0;
        debug_assert_eq!(get_width(p0), 2);
        let _rug_ed_tests_rug_6_rrrruuuugggg_test_get_width = 0;
    }
}
#[cfg(test)]
mod tests_rug_7 {
    use super::*;
    #[test]
    fn test_get_width_assume_valid() {
        let _rug_st_tests_rug_7_rrrruuuugggg_test_get_width_assume_valid = 0;
        let rug_fuzz_0 = 0b0110_0001;
        let p0: u8 = rug_fuzz_0;
        debug_assert_eq!(unsafe { get_width_assume_valid(p0) }, 1);
        let _rug_ed_tests_rug_7_rrrruuuugggg_test_get_width_assume_valid = 0;
    }
}
