use std::str;
use crate::find_byte::find_byte;
use crate::re_bytes;
use crate::re_unicode;
pub fn expand_str(
    caps: &re_unicode::Captures<'_>,
    mut replacement: &str,
    dst: &mut String,
) {
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
    caps: &re_bytes::Captures<'_>,
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
fn find_cap_ref(replacement: &[u8]) -> Option<CaptureRef<'_>> {
    let mut i = 0;
    let rep: &[u8] = replacement;
    if rep.len() <= 1 || rep[0] != b'$' {
        return None;
    }
    i += 1;
    if rep[i] == b'{' {
        return find_cap_ref_braced(rep, i + 1);
    }
    let mut cap_end = i;
    while rep.get(cap_end).copied().map_or(false, is_valid_cap_letter) {
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
fn find_cap_ref_braced(rep: &[u8], mut i: usize) -> Option<CaptureRef<'_>> {
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
/// Returns true if and only if the given byte is allowed in a capture name
/// written in non-brace form.
fn is_valid_cap_letter(b: u8) -> bool {
    match b {
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
    find!(find_cap_ref20, "${¾}", c!("¾", 5));
    find!(find_cap_ref21, "${¾a}", c!("¾a", 6));
    find!(find_cap_ref22, "${a¾}", c!("a¾", 6));
    find!(find_cap_ref23, "${☃}", c!("☃", 6));
    find!(find_cap_ref24, "${a☃}", c!("a☃", 7));
    find!(find_cap_ref25, "${☃a}", c!("☃a", 7));
    find!(find_cap_ref26, "${名字}", c!("名字", 9));
}
#[cfg(test)]
mod tests_rug_204 {
    use super::*;
    use crate::re_bytes::Captures;
    use std::vec::Vec;
    #[test]
    fn test_rug() {
        let caps = unimplemented!(
            "Fill in the re_bytes::Captures<'_> argument with sample data"
        );
        let replacement: &[u8] = unimplemented!(
            "Fill in the &[u8] argument with sample data"
        );
        let mut dst: Vec<u8> = Vec::new();
        crate::expand::expand_bytes(&caps, replacement, &mut dst);
    }
}
#[cfg(test)]
mod tests_rug_205 {
    use super::*;
    use crate::expand::CaptureRef;
    #[test]
    fn test_find_cap_ref() {
        let _rug_st_tests_rug_205_rrrruuuugggg_test_find_cap_ref = 0;
        let rug_fuzz_0 = b"$0";
        let p0: &[u8] = rug_fuzz_0;
        crate::expand::find_cap_ref(p0);
        let _rug_ed_tests_rug_205_rrrruuuugggg_test_find_cap_ref = 0;
    }
}
#[cfg(test)]
mod tests_rug_206 {
    use super::*;
    use crate::expand::CaptureRef;
    use std::str;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_206_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"This is a test {capture} that needs {to} be braced";
        let rug_fuzz_1 = 0;
        let rep: &[u8] = rug_fuzz_0;
        let i: usize = rug_fuzz_1;
        crate::expand::find_cap_ref_braced(rep, i);
        let _rug_ed_tests_rug_206_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_207 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_207_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b'A';
        let mut p0: u8 = rug_fuzz_0;
        crate::expand::is_valid_cap_letter(p0);
        let _rug_ed_tests_rug_207_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_209 {
    use super::*;
    use crate::expand::Ref;
    use std::convert::From;
    #[test]
    fn test_from() {
        let _rug_st_tests_rug_209_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = 42;
        let p0: usize = rug_fuzz_0;
        let result = <Ref<'static> as From<usize>>::from(p0);
        let _rug_ed_tests_rug_209_rrrruuuugggg_test_from = 0;
    }
}
