use std::str;
use find_byte::find_byte;
use re_bytes;
use re_unicode;
pub fn expand_str(caps: &re_unicode::Captures, mut replacement: &str, dst: &mut String) {
    while !replacement.is_empty() {
        match find_byte(b'$', replacement.as_bytes()) {
            None => break,
            Some(i) => {
                dst.push_str(&replacement[..i]);
                replacement = &replacement[i..];
            }
        }
        if replacement.as_bytes().get(1).map_or(false, |&b| b == b'$') {
            dst.push_str("$");
            replacement = &replacement[2..];
            continue;
        }
        debug_assert!(! replacement.is_empty());
        let cap_ref = match find_cap_ref(replacement.as_bytes()) {
            Some(cap_ref) => cap_ref,
            None => {
                dst.push_str("$");
                replacement = &replacement[1..];
                continue;
            }
        };
        replacement = &replacement[cap_ref.end..];
        match cap_ref.cap {
            Ref::Number(i) => {
                dst.push_str(caps.get(i).map(|m| m.as_str()).unwrap_or(""));
            }
            Ref::Named(name) => {
                dst.push_str(caps.name(name).map(|m| m.as_str()).unwrap_or(""));
            }
        }
    }
    dst.push_str(replacement);
}
pub fn expand_bytes(
    caps: &re_bytes::Captures,
    mut replacement: &[u8],
    dst: &mut Vec<u8>,
) {
    while !replacement.is_empty() {
        match find_byte(b'$', replacement) {
            None => break,
            Some(i) => {
                dst.extend(&replacement[..i]);
                replacement = &replacement[i..];
            }
        }
        if replacement.get(1).map_or(false, |&b| b == b'$') {
            dst.push(b'$');
            replacement = &replacement[2..];
            continue;
        }
        debug_assert!(! replacement.is_empty());
        let cap_ref = match find_cap_ref(replacement) {
            Some(cap_ref) => cap_ref,
            None => {
                dst.push(b'$');
                replacement = &replacement[1..];
                continue;
            }
        };
        replacement = &replacement[cap_ref.end..];
        match cap_ref.cap {
            Ref::Number(i) => {
                dst.extend(caps.get(i).map(|m| m.as_bytes()).unwrap_or(b""));
            }
            Ref::Named(name) => {
                dst.extend(caps.name(name).map(|m| m.as_bytes()).unwrap_or(b""));
            }
        }
    }
    dst.extend(replacement);
}
/// `CaptureRef` represents a reference to a capture group inside some text.
/// The reference is either a capture group name or a number.
///
/// It is also tagged with the position in the text following the
/// capture reference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CaptureRef<'a> {
    cap: Ref<'a>,
    end: usize,
}
/// A reference to a capture group in some text.
///
/// e.g., `$2`, `$foo`, `${foo}`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Ref<'a> {
    Named(&'a str),
    Number(usize),
}
impl<'a> From<&'a str> for Ref<'a> {
    fn from(x: &'a str) -> Ref<'a> {
        Ref::Named(x)
    }
}
impl From<usize> for Ref<'static> {
    fn from(x: usize) -> Ref<'static> {
        Ref::Number(x)
    }
}
/// Parses a possible reference to a capture group name in the given text,
/// starting at the beginning of `replacement`.
///
/// If no such valid reference could be found, None is returned.
fn find_cap_ref(replacement: &[u8]) -> Option<CaptureRef> {
    let mut i = 0;
    let rep: &[u8] = replacement.as_ref();
    if rep.len() <= 1 || rep[0] != b'$' {
        return None;
    }
    i += 1;
    if rep[i] == b'{' {
        return find_cap_ref_braced(rep, i + 1);
    }
    let mut cap_end = i;
    while rep.get(cap_end).map_or(false, is_valid_cap_letter) {
        cap_end += 1;
    }
    if cap_end == i {
        return None;
    }
    let cap = str::from_utf8(&rep[i..cap_end]).expect("valid UTF-8 capture name");
    Some(CaptureRef {
        cap: match cap.parse::<u32>() {
            Ok(i) => Ref::Number(i as usize),
            Err(_) => Ref::Named(cap),
        },
        end: cap_end,
    })
}
fn find_cap_ref_braced(rep: &[u8], mut i: usize) -> Option<CaptureRef> {
    let start = i;
    while rep.get(i).map_or(false, |&b| b != b'}') {
        i += 1;
    }
    if !rep.get(i).map_or(false, |&b| b == b'}') {
        return None;
    }
    let cap = match str::from_utf8(&rep[start..i]) {
        Err(_) => return None,
        Ok(cap) => cap,
    };
    Some(CaptureRef {
        cap: match cap.parse::<u32>() {
            Ok(i) => Ref::Number(i as usize),
            Err(_) => Ref::Named(cap),
        },
        end: i + 1,
    })
}
/// Returns true if and only if the given byte is allowed in a capture name.
fn is_valid_cap_letter(b: &u8) -> bool {
    match *b {
        b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' | b'_' => true,
        _ => false,
    }
}
#[cfg(test)]
mod tests {
    use super::{find_cap_ref, CaptureRef};
    macro_rules! find {
        ($name:ident, $text:expr) => {
            #[test] fn $name () { assert_eq!(None, find_cap_ref($text .as_bytes())); }
        };
        ($name:ident, $text:expr, $capref:expr) => {
            #[test] fn $name () { assert_eq!(Some($capref), find_cap_ref($text
            .as_bytes())); }
        };
    }
    macro_rules! c {
        ($name_or_number:expr, $pos:expr) => {
            CaptureRef { cap : $name_or_number .into(), end : $pos }
        };
    }
    find!(find_cap_ref1, "$foo", c!("foo", 4));
    find!(find_cap_ref2, "${foo}", c!("foo", 6));
    find!(find_cap_ref3, "$0", c!(0, 2));
    find!(find_cap_ref4, "$5", c!(5, 2));
    find!(find_cap_ref5, "$10", c!(10, 3));
    find!(find_cap_ref6, "$42a", c!("42a", 4));
    find!(find_cap_ref7, "${42}a", c!(42, 5));
    find!(find_cap_ref8, "${42");
    find!(find_cap_ref9, "${42 ");
    find!(find_cap_ref10, " $0 ");
    find!(find_cap_ref11, "$");
    find!(find_cap_ref12, " ");
    find!(find_cap_ref13, "");
    find!(find_cap_ref14, "$1-$2", c!(1, 2));
    find!(find_cap_ref15, "$1_$2", c!("1_", 3));
    find!(find_cap_ref16, "$x-$y", c!("x", 2));
    find!(find_cap_ref17, "$x_$y", c!("x_", 3));
    find!(find_cap_ref18, "${#}", c!("#", 4));
    find!(find_cap_ref19, "${Z[}", c!("Z[", 5));
}
#[cfg(test)]
mod tests_llm_16_59 {
    use super::*;
    use crate::*;
    use expand::Ref;
    #[test]
    fn test_from() {
        let _rug_st_tests_llm_16_59_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = "foo";
        let rug_fuzz_1 = 2_usize;
        let rug_fuzz_2 = "";
        let rug_fuzz_3 = "bar";
        let rug_fuzz_4 = 5_usize;
        let rug_fuzz_5 = "";
        let str_ref: &'static str = rug_fuzz_0;
        let named_ref: Ref<'static> = Ref::from(str_ref);
        debug_assert_eq!(named_ref, Ref::Named("foo"));
        let number_ref: Ref<'static> = Ref::from(rug_fuzz_1);
        debug_assert_eq!(number_ref, Ref::Number(2));
        let empty_ref: Ref<'static> = Ref::from(rug_fuzz_2);
        debug_assert_eq!(empty_ref, Ref::Named(""));
        let str_ref: &'static str = rug_fuzz_3;
        let named_ref: Ref<'static> = Ref::from(str_ref);
        debug_assert_eq!(named_ref, Ref::Named("bar"));
        let number_ref: Ref<'static> = Ref::from(rug_fuzz_4);
        debug_assert_eq!(number_ref, Ref::Number(5));
        let empty_ref: Ref<'static> = Ref::from(rug_fuzz_5);
        debug_assert_eq!(empty_ref, Ref::Named(""));
        let _rug_ed_tests_llm_16_59_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_60 {
    use crate::expand::Ref;
    #[test]
    fn test_from() {
        let _rug_st_tests_llm_16_60_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = 5;
        let x = rug_fuzz_0;
        let result = <Ref<'static> as std::convert::From<usize>>::from(x);
        match result {
            Ref::Number(num) => debug_assert_eq!(num, x),
            _ => panic!("Expected Ref::Number"),
        }
        let _rug_ed_tests_llm_16_60_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_378 {
    use super::*;
    use crate::*;
    #[test]
    fn test_find_cap_ref() {
        let _rug_st_tests_llm_16_378_rrrruuuugggg_test_find_cap_ref = 0;
        let rug_fuzz_0 = "$group";
        let rug_fuzz_1 = "$3abc";
        let rug_fuzz_2 = "$14abc";
        let rug_fuzz_3 = "$abc";
        let rug_fuzz_4 = "$";
        let rug_fuzz_5 = "abc$def";
        let input1: &[u8] = rug_fuzz_0.as_bytes();
        debug_assert_eq!(
            find_cap_ref(input1), Some(CaptureRef { cap : Ref::Named("group"), end : 6 })
        );
        let input2: &[u8] = rug_fuzz_1.as_bytes();
        debug_assert_eq!(
            find_cap_ref(input2), Some(CaptureRef { cap : Ref::Number(3), end : 2 })
        );
        let input3: &[u8] = rug_fuzz_2.as_bytes();
        debug_assert_eq!(
            find_cap_ref(input3), Some(CaptureRef { cap : Ref::Number(14), end : 3 })
        );
        let input4: &[u8] = rug_fuzz_3.as_bytes();
        debug_assert_eq!(find_cap_ref(input4), None);
        let input5: &[u8] = rug_fuzz_4.as_bytes();
        debug_assert_eq!(find_cap_ref(input5), None);
        let input6: &[u8] = rug_fuzz_5.as_bytes();
        debug_assert_eq!(find_cap_ref(input6), None);
        let _rug_ed_tests_llm_16_378_rrrruuuugggg_test_find_cap_ref = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_379 {
    use super::*;
    use crate::*;
    #[test]
    fn test_find_cap_ref_braced() {
        let _rug_st_tests_llm_16_379_rrrruuuugggg_test_find_cap_ref_braced = 0;
        let rug_fuzz_0 = b"abc{123}";
        let rug_fuzz_1 = 3;
        let rug_fuzz_2 = b"abc{def}";
        let rug_fuzz_3 = 3;
        let rug_fuzz_4 = b"abc}";
        let rug_fuzz_5 = 3;
        let rug_fuzz_6 = b"abc{123";
        let rug_fuzz_7 = 3;
        debug_assert_eq!(
            find_cap_ref_braced(rug_fuzz_0, rug_fuzz_1), Some(CaptureRef { cap :
            Ref::Number(123), end : 8, })
        );
        debug_assert_eq!(
            find_cap_ref_braced(rug_fuzz_2, rug_fuzz_3), Some(CaptureRef { cap :
            Ref::Named("def"), end : 8, })
        );
        debug_assert_eq!(find_cap_ref_braced(rug_fuzz_4, rug_fuzz_5), None);
        debug_assert_eq!(find_cap_ref_braced(rug_fuzz_6, rug_fuzz_7), None);
        let _rug_ed_tests_llm_16_379_rrrruuuugggg_test_find_cap_ref_braced = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_380 {
    use crate::expand::is_valid_cap_letter;
    #[test]
    fn test_is_valid_cap_letter() {
        let _rug_st_tests_llm_16_380_rrrruuuugggg_test_is_valid_cap_letter = 0;
        let rug_fuzz_0 = b'0';
        let rug_fuzz_1 = b'9';
        let rug_fuzz_2 = b'a';
        let rug_fuzz_3 = b'z';
        let rug_fuzz_4 = b'A';
        let rug_fuzz_5 = b'Z';
        let rug_fuzz_6 = b'_';
        let rug_fuzz_7 = b'@';
        let rug_fuzz_8 = b' ';
        let rug_fuzz_9 = b'%';
        debug_assert_eq!(is_valid_cap_letter(& rug_fuzz_0), true);
        debug_assert_eq!(is_valid_cap_letter(& rug_fuzz_1), true);
        debug_assert_eq!(is_valid_cap_letter(& rug_fuzz_2), true);
        debug_assert_eq!(is_valid_cap_letter(& rug_fuzz_3), true);
        debug_assert_eq!(is_valid_cap_letter(& rug_fuzz_4), true);
        debug_assert_eq!(is_valid_cap_letter(& rug_fuzz_5), true);
        debug_assert_eq!(is_valid_cap_letter(& rug_fuzz_6), true);
        debug_assert_eq!(is_valid_cap_letter(& rug_fuzz_7), false);
        debug_assert_eq!(is_valid_cap_letter(& rug_fuzz_8), false);
        debug_assert_eq!(is_valid_cap_letter(& rug_fuzz_9), false);
        let _rug_ed_tests_llm_16_380_rrrruuuugggg_test_is_valid_cap_letter = 0;
    }
}
