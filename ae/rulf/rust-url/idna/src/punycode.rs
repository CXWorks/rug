//! Punycode ([RFC 3492](http://tools.ietf.org/html/rfc3492)) implementation.
//!
//! Since Punycode fundamentally works on unicode code points,
//! `encode` and `decode` take and return slices and vectors of `char`.
//! `encode_str` and `decode_to_string` provide convenience wrappers
//! that convert from and to Rust’s UTF-8 based `str` and `String` types.
use std::char;
use std::u32;
static BASE: u32 = 36;
static T_MIN: u32 = 1;
static T_MAX: u32 = 26;
static SKEW: u32 = 38;
static DAMP: u32 = 700;
static INITIAL_BIAS: u32 = 72;
static INITIAL_N: u32 = 0x80;
static DELIMITER: char = '-';
#[inline]
fn adapt(mut delta: u32, num_points: u32, first_time: bool) -> u32 {
    delta /= if first_time { DAMP } else { 2 };
    delta += delta / num_points;
    let mut k = 0;
    while delta > ((BASE - T_MIN) * T_MAX) / 2 {
        delta /= BASE - T_MIN;
        k += BASE;
    }
    k + (((BASE - T_MIN + 1) * delta) / (delta + SKEW))
}
/// Convert Punycode to an Unicode `String`.
///
/// This is a convenience wrapper around `decode`.
#[inline]
pub fn decode_to_string(input: &str) -> Option<String> {
    decode(input).map(|chars| chars.into_iter().collect())
}
/// Convert Punycode to Unicode.
///
/// Return None on malformed input or overflow.
/// Overflow can only happen on inputs that take more than
/// 63 encoded bytes, the DNS limit on domain name labels.
pub fn decode(input: &str) -> Option<Vec<char>> {
    let (mut output, input) = match input.rfind(DELIMITER) {
        None => (Vec::new(), input),
        Some(position) => {
            (
                input[..position].chars().collect(),
                if position > 0 { &input[position + 1..] } else { input },
            )
        }
    };
    let mut code_point = INITIAL_N;
    let mut bias = INITIAL_BIAS;
    let mut i = 0;
    let mut iter = input.bytes();
    loop {
        let previous_i = i;
        let mut weight = 1;
        let mut k = BASE;
        let mut byte = match iter.next() {
            None => break,
            Some(byte) => byte,
        };
        loop {
            let digit = match byte {
                byte @ b'0'..=b'9' => byte - b'0' + 26,
                byte @ b'A'..=b'Z' => byte - b'A',
                byte @ b'a'..=b'z' => byte - b'a',
                _ => return None,
            } as u32;
            if digit > (u32::MAX - i) / weight {
                return None;
            }
            i += digit * weight;
            let t = if k <= bias {
                T_MIN
            } else if k >= bias + T_MAX {
                T_MAX
            } else {
                k - bias
            };
            if digit < t {
                break;
            }
            if weight > u32::MAX / (BASE - t) {
                return None;
            }
            weight *= BASE - t;
            k += BASE;
            byte = match iter.next() {
                None => return None,
                Some(byte) => byte,
            };
        }
        let length = output.len() as u32;
        bias = adapt(i - previous_i, length + 1, previous_i == 0);
        if i / (length + 1) > u32::MAX - code_point {
            return None;
        }
        code_point += i / (length + 1);
        i %= length + 1;
        let c = match char::from_u32(code_point) {
            Some(c) => c,
            None => return None,
        };
        output.insert(i as usize, c);
        i += 1;
    }
    Some(output)
}
/// Convert an Unicode `str` to Punycode.
///
/// This is a convenience wrapper around `encode`.
#[inline]
pub fn encode_str(input: &str) -> Option<String> {
    let mut buf = String::with_capacity(input.len());
    encode_into(input.chars(), &mut buf).ok().map(|()| buf)
}
/// Convert Unicode to Punycode.
///
/// Return None on overflow, which can only happen on inputs that would take more than
/// 63 encoded bytes, the DNS limit on domain name labels.
pub fn encode(input: &[char]) -> Option<String> {
    let mut buf = String::with_capacity(input.len());
    encode_into(input.iter().copied(), &mut buf).ok().map(|()| buf)
}
fn encode_into<I>(input: I, output: &mut String) -> Result<(), ()>
where
    I: Iterator<Item = char> + Clone,
{
    let (mut input_length, mut basic_length) = (0, 0);
    for c in input.clone() {
        input_length += 1;
        if c.is_ascii() {
            output.push(c);
            basic_length += 1;
        }
    }
    if basic_length > 0 {
        output.push_str("-")
    }
    let mut code_point = INITIAL_N;
    let mut delta = 0;
    let mut bias = INITIAL_BIAS;
    let mut processed = basic_length;
    while processed < input_length {
        let min_code_point = input
            .clone()
            .map(|c| c as u32)
            .filter(|&c| c >= code_point)
            .min()
            .unwrap();
        if min_code_point - code_point > (u32::MAX - delta) / (processed + 1) {
            return Err(());
        }
        delta += (min_code_point - code_point) * (processed + 1);
        code_point = min_code_point;
        for c in input.clone() {
            let c = c as u32;
            if c < code_point {
                delta += 1;
                if delta == 0 {
                    return Err(());
                }
            }
            if c == code_point {
                let mut q = delta;
                let mut k = BASE;
                loop {
                    let t = if k <= bias {
                        T_MIN
                    } else if k >= bias + T_MAX {
                        T_MAX
                    } else {
                        k - bias
                    };
                    if q < t {
                        break;
                    }
                    let value = t + ((q - t) % (BASE - t));
                    output.push(value_to_digit(value));
                    q = (q - t) / (BASE - t);
                    k += BASE;
                }
                output.push(value_to_digit(q));
                bias = adapt(delta, processed + 1, processed == basic_length);
                delta = 0;
                processed += 1;
            }
        }
        delta += 1;
        code_point += 1;
    }
    Ok(())
}
#[inline]
fn value_to_digit(value: u32) -> char {
    match value {
        0..=25 => (value as u8 + b'a') as char,
        26..=35 => (value as u8 - 26 + b'0') as char,
        _ => panic!(),
    }
}
#[cfg(test)]
mod tests_llm_16_8 {
    use super::*;
    use crate::*;
    #[test]
    fn test_adapt() {
        let _rug_st_tests_llm_16_8_rrrruuuugggg_test_adapt = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = true;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 1;
        let rug_fuzz_5 = false;
        let rug_fuzz_6 = 100;
        let rug_fuzz_7 = 10;
        let rug_fuzz_8 = true;
        let rug_fuzz_9 = 200;
        let rug_fuzz_10 = 5;
        let rug_fuzz_11 = false;
        debug_assert_eq!(adapt(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2), 0);
        debug_assert_eq!(adapt(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5), 0);
        debug_assert_eq!(adapt(rug_fuzz_6, rug_fuzz_7, rug_fuzz_8), 87);
        debug_assert_eq!(adapt(rug_fuzz_9, rug_fuzz_10, rug_fuzz_11), 200);
        let _rug_ed_tests_llm_16_8_rrrruuuugggg_test_adapt = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_9 {
    use super::*;
    use crate::*;
    #[test]
    fn test_decode() {
        let _rug_st_tests_llm_16_9_rrrruuuugggg_test_decode = 0;
        let rug_fuzz_0 = "xn--bcher-kva.example";
        let rug_fuzz_1 = "bücher";
        let rug_fuzz_2 = "xn--bcher-kva";
        let rug_fuzz_3 = "bücher";
        let rug_fuzz_4 = "xn--bcher-kva.exampleoverflow";
        let rug_fuzz_5 = "xn--bcher-kva.example!@#$";
        let input1 = rug_fuzz_0;
        let output1 = Some(rug_fuzz_1.chars().collect::<Vec<char>>());
        debug_assert_eq!(decode(input1), output1);
        let input2 = rug_fuzz_2;
        let output2 = Some(rug_fuzz_3.chars().collect::<Vec<char>>());
        debug_assert_eq!(decode(input2), output2);
        let input3 = rug_fuzz_4;
        let output3: Option<Vec<char>> = None;
        debug_assert_eq!(decode(input3), output3);
        let input4 = rug_fuzz_5;
        let output4: Option<Vec<char>> = None;
        debug_assert_eq!(decode(input4), output4);
        let _rug_ed_tests_llm_16_9_rrrruuuugggg_test_decode = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_10 {
    use crate::punycode::decode_to_string;
    #[test]
    fn test_decode_to_string() {
        let _rug_st_tests_llm_16_10_rrrruuuugggg_test_decode_to_string = 0;
        let rug_fuzz_0 = "xn--4gbrim";
        let rug_fuzz_1 = "xn--t4c";
        let rug_fuzz_2 = "xn--4db9c8c";
        let rug_fuzz_3 = "xn--q9jyb4c";
        let rug_fuzz_4 = "xn--bcher-kva";
        debug_assert_eq!(
            decode_to_string(rug_fuzz_0), Some("مثال-إختبار".to_string())
        );
        debug_assert_eq!(decode_to_string(rug_fuzz_1), Some("tést".to_string()));
        debug_assert_eq!(decode_to_string(rug_fuzz_2), Some("hello".to_string()));
        debug_assert_eq!(decode_to_string(rug_fuzz_3), Some("點心".to_string()));
        debug_assert_eq!(decode_to_string(rug_fuzz_4), Some("bücher".to_string()));
        let _rug_ed_tests_llm_16_10_rrrruuuugggg_test_decode_to_string = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_11 {
    use super::*;
    use crate::*;
    #[test]
    fn test_encode() {
        let _rug_st_tests_llm_16_11_rrrruuuugggg_test_encode = 0;
        let rug_fuzz_0 = 'a';
        let rug_fuzz_1 = 'b';
        let rug_fuzz_2 = 'c';
        let rug_fuzz_3 = '☺';
        let rug_fuzz_4 = '☃';
        let rug_fuzz_5 = '☷';
        let rug_fuzz_6 = '!';
        let rug_fuzz_7 = '@';
        let rug_fuzz_8 = '#';
        let rug_fuzz_9 = '&';
        let rug_fuzz_10 = '*';
        let rug_fuzz_11 = '$';
        let rug_fuzz_12 = '%';
        let rug_fuzz_13 = '^';
        let rug_fuzz_14 = '(';
        debug_assert_eq!(
            punycode::encode(& [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2]).unwrap(), "abc"
        );
        debug_assert_eq!(
            punycode::encode(& [rug_fuzz_3, rug_fuzz_4, rug_fuzz_5]).unwrap(), "xn--74h"
        );
        debug_assert_eq!(
            punycode::encode(& [rug_fuzz_6, rug_fuzz_7, rug_fuzz_8]).unwrap(), "xn--21h"
        );
        debug_assert_eq!(
            punycode::encode(& [rug_fuzz_9, rug_fuzz_10, rug_fuzz_11]).unwrap(),
            "xn--imz"
        );
        debug_assert_eq!(
            punycode::encode(& [rug_fuzz_12, rug_fuzz_13, rug_fuzz_14]).unwrap(),
            "xn--vnu"
        );
        let _rug_ed_tests_llm_16_11_rrrruuuugggg_test_encode = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_14 {
    use crate::punycode::encode_str;
    #[test]
    fn test_encode_str() {
        let _rug_st_tests_llm_16_14_rrrruuuugggg_test_encode_str = 0;
        let rug_fuzz_0 = "example.com";
        let rug_fuzz_1 = "ünicödé.com";
        let rug_fuzz_2 = "bücher";
        let rug_fuzz_3 = "🌍.com";
        let rug_fuzz_4 = "いぬ.com";
        debug_assert_eq!(encode_str(rug_fuzz_0), Some("example.com".to_string()));
        debug_assert_eq!(encode_str(rug_fuzz_1), Some("xn--nicd-estb.com".to_string()));
        debug_assert_eq!(encode_str(rug_fuzz_2), Some("bcher-kva".to_string()));
        debug_assert_eq!(encode_str(rug_fuzz_3), Some("xn--ls8h.com".to_string()));
        debug_assert_eq!(encode_str(rug_fuzz_4), Some("xn--eckwd4c.com".to_string()));
        let _rug_ed_tests_llm_16_14_rrrruuuugggg_test_encode_str = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_15 {
    use crate::punycode::value_to_digit;
    #[test]
    fn test_value_to_digit() {
        let _rug_st_tests_llm_16_15_rrrruuuugggg_test_value_to_digit = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 25;
        let rug_fuzz_2 = 26;
        let rug_fuzz_3 = 35;
        debug_assert_eq!(value_to_digit(rug_fuzz_0), 'a');
        debug_assert_eq!(value_to_digit(rug_fuzz_1), 'z');
        debug_assert_eq!(value_to_digit(rug_fuzz_2), '0');
        debug_assert_eq!(value_to_digit(rug_fuzz_3), '9');
        let _rug_ed_tests_llm_16_15_rrrruuuugggg_test_value_to_digit = 0;
    }
}
