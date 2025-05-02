//! Simple parser expression generator
use std::char::from_u32;
use std::ops;
pub trait Peg {
    fn p(&self, s: &[u8]) -> Option<usize>;
}
impl<F: Fn(&[u8]) -> Option<usize>> Peg for F {
    #[inline(always)]
    fn p(&self, s: &[u8]) -> Option<usize> {
        self(s)
    }
}
pub struct OneByte<F>(pub F);
impl<F: Fn(u8) -> bool> Peg for OneByte<F> {
    #[inline(always)]
    fn p(&self, s: &[u8]) -> Option<usize> {
        if s.is_empty() || !self.0(s[0]) { None } else { Some(1) }
    }
}
impl Peg for u8 {
    #[inline(always)]
    fn p(&self, s: &[u8]) -> Option<usize> {
        OneByte(|b| b == *self).p(s)
    }
}
pub struct OneChar<F>(pub F);
fn decode_utf8(s: &[u8]) -> Option<(char, usize)> {
    if s.is_empty() {
        return None;
    }
    let b = s[0];
    if b < 0x80 {
        return Some((b as char, 1));
    } else if b >= 0xc2 && b < 0xe0 && s.len() >= 2 {
        let b2 = s[1];
        if (b2 as i8) > -0x40 {
            return None;
        }
        let cp = (u32::from(b) << 6) + u32::from(b2) - 0x3080;
        return from_u32(cp).map(|ch| (ch, 2));
    } else if b >= 0xe0 && b < 0xf0 && s.len() >= 3 {
        let b2 = s[1];
        let b3 = s[2];
        if (b2 as i8) > -0x40 || (b3 as i8) > -0x40 {
            return None;
        }
        let cp = (u32::from(b) << 12) + (u32::from(b2) << 6) + u32::from(b3) - 0xe2080;
        if cp < 0x800 {
            return None;
        }
        return from_u32(cp).map(|ch| (ch, 3));
    } else if b >= 0xf0 && b < 0xf5 && s.len() >= 4 {
        let b2 = s[1];
        let b3 = s[2];
        let b4 = s[3];
        if (b2 as i8) > -0x40 || (b3 as i8) > -0x40 || (b4 as i8) > -0x40 {
            return None;
        }
        let cp = (u32::from(b) << 18) + (u32::from(b2) << 12) + (u32::from(b3) << 6)
            + u32::from(b4) - 0x03c8_2080;
        if cp < 0x10000 {
            return None;
        }
        return from_u32(cp).map(|ch| (ch, 4));
    }
    None
}
impl<F: Fn(char) -> bool> Peg for OneChar<F> {
    #[inline(always)]
    fn p(&self, s: &[u8]) -> Option<usize> {
        if let Some((ch, len)) = decode_utf8(s) {
            if self.0(ch) {
                return Some(len);
            }
        }
        None
    }
}
fn char_helper(s: &[u8], c: char) -> Option<usize> {
    OneChar(|x| x == c).p(s)
}
impl Peg for char {
    #[inline(always)]
    fn p(&self, s: &[u8]) -> Option<usize> {
        let c = *self;
        if c <= '\x7f' { (c as u8).p(s) } else { char_helper(s, c) }
    }
}
/// Use Inclusive(a..b) to indicate an inclusive range. When a...b syntax becomes
/// stable, we'll get rid of this and switch to that.
pub struct Inclusive<T>(pub T);
impl Peg for ops::Range<u8> {
    #[inline(always)]
    fn p(&self, s: &[u8]) -> Option<usize> {
        OneByte(|x| x >= self.start && x < self.end).p(s)
    }
}
impl Peg for Inclusive<ops::Range<u8>> {
    #[inline(always)]
    fn p(&self, s: &[u8]) -> Option<usize> {
        OneByte(|x| x >= self.0.start && x <= self.0.end).p(s)
    }
}
impl<'a> Peg for &'a [u8] {
    #[inline(always)]
    fn p(&self, s: &[u8]) -> Option<usize> {
        let len = self.len();
        if s.len() >= len && &s[..len] == *self { Some(len) } else { None }
    }
}
impl<'a> Peg for &'a str {
    #[inline(always)]
    fn p(&self, s: &[u8]) -> Option<usize> {
        self.as_bytes().p(s)
    }
}
impl<P1: Peg, P2: Peg> Peg for (P1, P2) {
    #[inline(always)]
    fn p(&self, s: &[u8]) -> Option<usize> {
        self.0.p(s).and_then(|len1| self.1.p(&s[len1..]).map(|len2| len1 + len2))
    }
}
impl<P1: Peg, P2: Peg, P3: Peg> Peg for (P1, P2, P3) {
    #[inline(always)]
    fn p(&self, s: &[u8]) -> Option<usize> {
        self.0
            .p(s)
            .and_then(|len1| {
                self.1
                    .p(&s[len1..])
                    .and_then(|len2| {
                        self.2.p(&s[len1 + len2..]).map(|len3| len1 + len2 + len3)
                    })
            })
    }
}
macro_rules! impl_tuple {
    ($($p:ident $ix:ident),*) => {
        impl < $($p : Peg),* > Peg for ($($p),*) { #[inline(always)] fn p(& self, s : &
        [u8]) -> Option < usize > { let ($(ref $ix),*) = * self; let mut i = 0; $(if let
        Some(len) = $ix .p(& s[i..]) { i += len; } else { return None; })* Some(i) } }
    };
}
impl_tuple!(P1 p1, P2 p2, P3 p3, P4 p4);
/// Choice from two heterogeneous alternatives.
pub struct Alt<P1, P2>(pub P1, pub P2);
impl<P1: Peg, P2: Peg> Peg for Alt<P1, P2> {
    #[inline(always)]
    fn p(&self, s: &[u8]) -> Option<usize> {
        self.0.p(s).or_else(|| self.1.p(s))
    }
}
/// Choice from three heterogeneous alternatives.
pub struct Alt3<P1, P2, P3>(pub P1, pub P2, pub P3);
impl<P1: Peg, P2: Peg, P3: Peg> Peg for Alt3<P1, P2, P3> {
    #[inline(always)]
    fn p(&self, s: &[u8]) -> Option<usize> {
        self.0.p(s).or_else(|| self.1.p(s).or_else(|| self.2.p(s)))
    }
}
/// Choice from a homogenous slice of parsers.
pub struct OneOf<'a, P: 'a>(pub &'a [P]);
impl<'a, P: Peg> Peg for OneOf<'a, P> {
    #[inline]
    fn p(&self, s: &[u8]) -> Option<usize> {
        for p in self.0.iter() {
            if let Some(len) = p.p(s) {
                return Some(len);
            }
        }
        None
    }
}
/// Repetition with a minimum and maximum (inclusive) bound
pub struct Repeat<P, R>(pub P, pub R);
impl<P: Peg> Peg for Repeat<P, usize> {
    #[inline]
    fn p(&self, s: &[u8]) -> Option<usize> {
        let Repeat(ref p, reps) = *self;
        let mut i = 0;
        let mut count = 0;
        while count < reps {
            if let Some(len) = p.p(&s[i..]) {
                i += len;
                count += 1;
            } else {
                break;
            }
        }
        Some(i)
    }
}
impl<P: Peg> Peg for Repeat<P, ops::Range<usize>> {
    #[inline]
    fn p(&self, s: &[u8]) -> Option<usize> {
        let Repeat(ref p, ops::Range { start, end }) = *self;
        let mut i = 0;
        let mut count = 0;
        while count + 1 < end {
            if let Some(len) = p.p(&s[i..]) {
                i += len;
                count += 1;
            } else {
                break;
            }
        }
        if count >= start { Some(i) } else { None }
    }
}
impl<P: Peg> Peg for Repeat<P, ops::RangeFrom<usize>> {
    #[inline]
    fn p(&self, s: &[u8]) -> Option<usize> {
        let Repeat(ref p, ops::RangeFrom { start }) = *self;
        let mut i = 0;
        let mut count = 0;
        while let Some(len) = p.p(&s[i..]) {
            i += len;
            count += 1;
        }
        if count >= start { Some(i) } else { None }
    }
}
impl<P: Peg> Peg for Repeat<P, ops::RangeFull> {
    #[inline]
    fn p(&self, s: &[u8]) -> Option<usize> {
        ZeroOrMore(Ref(&self.0)).p(s)
    }
}
impl<P: Peg> Peg for Repeat<P, ops::RangeTo<usize>> {
    #[inline]
    fn p(&self, s: &[u8]) -> Option<usize> {
        let Repeat(ref p, ops::RangeTo { end }) = *self;
        Repeat(Ref(p), 0..end).p(s)
    }
}
pub struct Optional<P>(pub P);
impl<P: Peg> Peg for Optional<P> {
    #[inline]
    fn p(&self, s: &[u8]) -> Option<usize> {
        self.0.p(s).or(Some(0))
    }
}
#[allow(dead_code)]
pub struct OneOrMore<P>(pub P);
impl<P: Peg> Peg for OneOrMore<P> {
    #[inline]
    fn p(&self, s: &[u8]) -> Option<usize> {
        Repeat(Ref(&self.0), 1..).p(s)
    }
}
pub struct ZeroOrMore<P>(pub P);
impl<P: Peg> Peg for ZeroOrMore<P> {
    #[inline]
    fn p(&self, s: &[u8]) -> Option<usize> {
        let mut i = 0;
        while let Some(len) = self.0.p(&s[i..]) {
            i += len;
        }
        Some(i)
    }
}
/// Fail to match if the arg matches, otherwise match empty.
pub struct FailIf<P>(pub P);
impl<P: Peg> Peg for FailIf<P> {
    #[inline]
    fn p(&self, s: &[u8]) -> Option<usize> {
        match self.0.p(s) {
            Some(_) => None,
            None => Some(0),
        }
    }
}
/// A wrapper to use whenever you have a reference to a Peg object
pub struct Ref<'a, P: 'a>(pub &'a P);
impl<'a, P: Peg> Peg for Ref<'a, P> {
    #[inline]
    fn p(&self, s: &[u8]) -> Option<usize> {
        self.0.p(s)
    }
}
#[cfg(test)]
mod tests_llm_16_2 {
    use super::*;
    use crate::*;
    #[test]
    fn test_p_with_matching_prefix() {
        let _rug_st_tests_llm_16_2_rrrruuuugggg_test_p_with_matching_prefix = 0;
        let rug_fuzz_0 = b"abc";
        let rug_fuzz_1 = b"abcdef";
        let peg: &[u8] = rug_fuzz_0;
        let s: &[u8] = rug_fuzz_1;
        let result = peg.p(s);
        debug_assert_eq!(result, Some(3));
        let _rug_ed_tests_llm_16_2_rrrruuuugggg_test_p_with_matching_prefix = 0;
    }
    #[test]
    fn test_p_with_non_matching_prefix() {
        let _rug_st_tests_llm_16_2_rrrruuuugggg_test_p_with_non_matching_prefix = 0;
        let rug_fuzz_0 = b"abc";
        let rug_fuzz_1 = b"def";
        let peg: &[u8] = rug_fuzz_0;
        let s: &[u8] = rug_fuzz_1;
        let result = peg.p(s);
        debug_assert_eq!(result, None);
        let _rug_ed_tests_llm_16_2_rrrruuuugggg_test_p_with_non_matching_prefix = 0;
    }
    #[test]
    fn test_p_with_empty_string() {
        let _rug_st_tests_llm_16_2_rrrruuuugggg_test_p_with_empty_string = 0;
        let rug_fuzz_0 = b"abc";
        let rug_fuzz_1 = b"";
        let peg: &[u8] = rug_fuzz_0;
        let s: &[u8] = rug_fuzz_1;
        let result = peg.p(s);
        debug_assert_eq!(result, None);
        let _rug_ed_tests_llm_16_2_rrrruuuugggg_test_p_with_empty_string = 0;
    }
    #[test]
    fn test_p_with_short_string() {
        let _rug_st_tests_llm_16_2_rrrruuuugggg_test_p_with_short_string = 0;
        let rug_fuzz_0 = b"abc";
        let rug_fuzz_1 = b"ab";
        let peg: &[u8] = rug_fuzz_0;
        let s: &[u8] = rug_fuzz_1;
        let result = peg.p(s);
        debug_assert_eq!(result, None);
        let _rug_ed_tests_llm_16_2_rrrruuuugggg_test_p_with_short_string = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_6_llm_16_5 {
    use super::*;
    use crate::*;
    use peg::Peg;
    #[test]
    fn test_p() {
        let _rug_st_tests_llm_16_6_llm_16_5_rrrruuuugggg_test_p = 0;
        let rug_fuzz_0 = 0u8;
        struct P1;
        struct P2;
        impl Peg for P1 {
            fn p(&self, s: &[u8]) -> Option<usize> {
                unimplemented!()
            }
        }
        impl Peg for P2 {
            fn p(&self, s: &[u8]) -> Option<usize> {
                unimplemented!()
            }
        }
        let p1 = P1;
        let p2 = P2;
        let p: &dyn Peg = &(p1, p2);
        let s = &[rug_fuzz_0; 10];
        let result = p.p(s);
        debug_assert_eq!(result, None);
        let _rug_ed_tests_llm_16_6_llm_16_5_rrrruuuugggg_test_p = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_10_llm_16_9 {
    use super::*;
    use crate::*;
    struct P1;
    struct P2;
    struct P3;
    struct P4;
    impl P1 {
        #[inline(always)]
        fn p(&self, s: &[u8]) -> Option<usize> {
            None
        }
    }
    impl P2 {
        #[inline(always)]
        fn p(&self, s: &[u8]) -> Option<usize> {
            None
        }
    }
    impl P3 {
        #[inline(always)]
        fn p(&self, s: &[u8]) -> Option<usize> {
            None
        }
    }
    impl P4 {
        #[inline(always)]
        fn p(&self, s: &[u8]) -> Option<usize> {
            None
        }
    }
    trait Peg {
        fn p(&self, s: &[u8]) -> Option<usize>;
    }
    impl Peg for (P1, P2, P3, P4) {
        #[inline(always)]
        fn p(&self, s: &[u8]) -> Option<usize> {
            let (p1, p2, p3, p4) = self;
            let mut i = 0;
            if let Some(len) = p1.p(&s[i..]) {
                i += len;
            } else {
                return None;
            }
            if let Some(len) = p2.p(&s[i..]) {
                i += len;
            } else {
                return None;
            }
            if let Some(len) = p3.p(&s[i..]) {
                i += len;
            } else {
                return None;
            }
            if let Some(len) = p4.p(&s[i..]) {
                i += len;
            } else {
                return None;
            }
            Some(i)
        }
    }
    #[test]
    fn test_p() {
        let _rug_st_tests_llm_16_10_llm_16_9_rrrruuuugggg_test_p = 0;
        let rug_fuzz_0 = b"example input";
        let p1 = P1;
        let p2 = P2;
        let p3 = P3;
        let p4 = P4;
        let input = rug_fuzz_0;
        let peg = (p1, p2, p3, p4);
        let result = peg.p(input);
        debug_assert_eq!(result, Some(input.len()));
        let _rug_ed_tests_llm_16_10_llm_16_9_rrrruuuugggg_test_p = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_27 {
    use super::*;
    use crate::*;
    #[test]
    fn test_p_with_ascii_character() {
        let _rug_st_tests_llm_16_27_rrrruuuugggg_test_p_with_ascii_character = 0;
        let rug_fuzz_0 = 'a';
        let rug_fuzz_1 = b"abc";
        let c = rug_fuzz_0;
        let s = rug_fuzz_1;
        debug_assert_eq!(c.p(s), Some(1));
        let _rug_ed_tests_llm_16_27_rrrruuuugggg_test_p_with_ascii_character = 0;
    }
    #[test]
    fn test_p_with_non_ascii_character() {
        let _rug_st_tests_llm_16_27_rrrruuuugggg_test_p_with_non_ascii_character = 0;
        let rug_fuzz_0 = 'é';
        let rug_fuzz_1 = b"abc";
        let c = rug_fuzz_0;
        let s = rug_fuzz_1;
        debug_assert_eq!(c.p(s), Some(0));
        let _rug_ed_tests_llm_16_27_rrrruuuugggg_test_p_with_non_ascii_character = 0;
    }
    #[test]
    fn test_p_with_empty_input() {
        let _rug_st_tests_llm_16_27_rrrruuuugggg_test_p_with_empty_input = 0;
        let rug_fuzz_0 = 'a';
        let rug_fuzz_1 = b"";
        let c = rug_fuzz_0;
        let s = rug_fuzz_1;
        debug_assert_eq!(c.p(s), None);
        let _rug_ed_tests_llm_16_27_rrrruuuugggg_test_p_with_empty_input = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_48 {
    use super::*;
    use crate::*;
    struct MockP1 {}
    impl Peg for MockP1 {
        fn p(&self, s: &[u8]) -> Option<usize> {
            None
        }
    }
    struct MockP2 {}
    impl Peg for MockP2 {
        fn p(&self, s: &[u8]) -> Option<usize> {
            None
        }
    }
    struct MockP3 {}
    impl Peg for MockP3 {
        fn p(&self, s: &[u8]) -> Option<usize> {
            None
        }
    }
    #[test]
    fn test_p() {
        let _rug_st_tests_llm_16_48_rrrruuuugggg_test_p = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        #[allow(dead_code)]
        let alt = Alt3(MockP1 {}, MockP2 {}, MockP3 {});
        let s: &[u8] = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        let result = alt.p(s);
        debug_assert_eq!(result, None);
        let _rug_ed_tests_llm_16_48_rrrruuuugggg_test_p = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_50_llm_16_49 {
    use super::*;
    use crate::*;
    use crate::peg::Peg;
    struct P1;
    struct P2;
    impl Peg for P1 {
        fn p(&self, s: &[u8]) -> Option<usize> {
            unimplemented!()
        }
    }
    impl Peg for P2 {
        fn p(&self, s: &[u8]) -> Option<usize> {
            unimplemented!()
        }
    }
    #[test]
    fn test_p() {
        let _rug_st_tests_llm_16_50_llm_16_49_rrrruuuugggg_test_p = 0;
        let alt = Alt(P1, P2);
        let s: &[u8] = &[];
        let expected: Option<usize> = None;
        let result = alt.p(s);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_50_llm_16_49_rrrruuuugggg_test_p = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_55 {
    use super::*;
    use crate::*;
    #[test]
    fn test_p_empty_string() {
        let _rug_st_tests_llm_16_55_rrrruuuugggg_test_p_empty_string = 0;
        let rug_fuzz_0 = true;
        let one_byte = OneByte(|_c: u8| rug_fuzz_0);
        let s: [u8; 0] = [];
        debug_assert_eq!(one_byte.p(& s), None);
        let _rug_ed_tests_llm_16_55_rrrruuuugggg_test_p_empty_string = 0;
    }
    #[test]
    fn test_p_incorrect_char() {
        let _rug_st_tests_llm_16_55_rrrruuuugggg_test_p_incorrect_char = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = 65;
        let one_byte = OneByte(|_c: u8| rug_fuzz_0);
        let s: [u8; 1] = [rug_fuzz_1];
        debug_assert_eq!(one_byte.p(& s), None);
        let _rug_ed_tests_llm_16_55_rrrruuuugggg_test_p_incorrect_char = 0;
    }
    #[test]
    fn test_p_correct_char() {
        let _rug_st_tests_llm_16_55_rrrruuuugggg_test_p_correct_char = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = 97;
        let one_byte = OneByte(|_c: u8| rug_fuzz_0);
        let s: [u8; 1] = [rug_fuzz_1];
        debug_assert_eq!(one_byte.p(& s), Some(1));
        let _rug_ed_tests_llm_16_55_rrrruuuugggg_test_p_correct_char = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_56 {
    use super::*;
    use crate::*;
    #[test]
    fn test_p_returns_some() {
        let _rug_st_tests_llm_16_56_rrrruuuugggg_test_p_returns_some = 0;
        let rug_fuzz_0 = "abc";
        let char_predicate = |c: char| c.is_alphabetic();
        let one_char = OneChar(char_predicate);
        let input = rug_fuzz_0.as_bytes();
        let result = one_char.p(input);
        debug_assert_eq!(result, Some(1));
        let _rug_ed_tests_llm_16_56_rrrruuuugggg_test_p_returns_some = 0;
    }
    #[test]
    fn test_p_returns_none() {
        let _rug_st_tests_llm_16_56_rrrruuuugggg_test_p_returns_none = 0;
        let rug_fuzz_0 = "123";
        let char_predicate = |c: char| c.is_alphabetic();
        let one_char = OneChar(char_predicate);
        let input = rug_fuzz_0.as_bytes();
        let result = one_char.p(input);
        debug_assert_eq!(result, None);
        let _rug_ed_tests_llm_16_56_rrrruuuugggg_test_p_returns_none = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_60 {
    use super::*;
    use crate::*;
    #[test]
    fn test_p() {
        let _rug_st_tests_llm_16_60_rrrruuuugggg_test_p = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 2;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 1;
        let rug_fuzz_6 = 2;
        let peg = OneOrMore(rug_fuzz_0);
        let s = &[
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
            rug_fuzz_6,
        ];
        let result = peg.p(s);
        debug_assert_eq!(result, Some(0));
        let _rug_ed_tests_llm_16_60_rrrruuuugggg_test_p = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_62_llm_16_61 {
    use crate::peg::Peg;
    use crate::peg::Optional;
    #[test]
    fn test_p_returns_some() {
        let _rug_st_tests_llm_16_62_llm_16_61_rrrruuuugggg_test_p_returns_some = 0;
        let rug_fuzz_0 = "hello";
        let peg = Optional(SomePeg);
        let s = rug_fuzz_0.as_bytes();
        let result = peg.p(&s);
        debug_assert_eq!(result, Some(5));
        let _rug_ed_tests_llm_16_62_llm_16_61_rrrruuuugggg_test_p_returns_some = 0;
    }
    #[test]
    fn test_p_returns_none() {
        let _rug_st_tests_llm_16_62_llm_16_61_rrrruuuugggg_test_p_returns_none = 0;
        let rug_fuzz_0 = "hello";
        let peg = Optional(NonePeg);
        let s = rug_fuzz_0.as_bytes();
        let result = peg.p(&s);
        debug_assert_eq!(result, Some(0));
        let _rug_ed_tests_llm_16_62_llm_16_61_rrrruuuugggg_test_p_returns_none = 0;
    }
    struct SomePeg;
    impl Peg for SomePeg {
        fn p(&self, s: &[u8]) -> Option<usize> {
            Some(s.len())
        }
    }
    struct NonePeg;
    impl Peg for NonePeg {
        fn p(&self, s: &[u8]) -> Option<usize> {
            None
        }
    }
}
#[cfg(test)]
mod tests_llm_16_63 {
    use super::*;
    use crate::*;
    #[test]
    fn test_p() {
        let _rug_st_tests_llm_16_63_rrrruuuugggg_test_p = 0;
        let rug_fuzz_0 = b"test_string";
        let p: Ref<TestPeg> = Ref(&TestPeg);
        let s: &[u8] = rug_fuzz_0;
        let result = p.p(s);
        debug_assert_eq!(result, Some(11));
        let _rug_ed_tests_llm_16_63_rrrruuuugggg_test_p = 0;
    }
}
struct TestPeg;
impl Peg for TestPeg {
    fn p(&self, s: &[u8]) -> Option<usize> {
        Some(s.len())
    }
}
#[cfg(test)]
mod tests_llm_16_64 {
    use super::*;
    use crate::*;
    use std::ops::Range;
    struct MockP;
    impl Peg for MockP {
        fn p(&self, s: &[u8]) -> Option<usize> {
            Some(s.len())
        }
    }
    #[test]
    fn test_p() {
        let _rug_st_tests_llm_16_64_rrrruuuugggg_test_p = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 4;
        let rug_fuzz_2 = b"abcde";
        let rug_fuzz_3 = b"ab";
        let rug_fuzz_4 = b"a";
        let p = Repeat(
            MockP,
            Range {
                start: rug_fuzz_0,
                end: rug_fuzz_1,
            },
        );
        let s = rug_fuzz_2;
        debug_assert_eq!(p.p(s), Some(3));
        let s = rug_fuzz_3;
        debug_assert_eq!(p.p(s), Some(2));
        let s = rug_fuzz_4;
        debug_assert_eq!(p.p(s), None);
        let _rug_ed_tests_llm_16_64_rrrruuuugggg_test_p = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_65 {
    use super::*;
    use crate::*;
    #[test]
    fn test_p() {
        let _rug_st_tests_llm_16_65_rrrruuuugggg_test_p = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 5;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 10;
        let rug_fuzz_4 = 1;
        let rug_fuzz_5 = 2;
        let rug_fuzz_6 = 3;
        let rug_fuzz_7 = 4;
        let rug_fuzz_8 = 5;
        let rug_fuzz_9 = 1;
        let rug_fuzz_10 = 2;
        let rug_fuzz_11 = 3;
        let rug_fuzz_12 = 4;
        let rug_fuzz_13 = 5;
        let rug_fuzz_14 = 0;
        let rug_fuzz_15 = 5;
        let rug_fuzz_16 = 0;
        let rug_fuzz_17 = 10;
        let rug_fuzz_18 = 1;
        let rug_fuzz_19 = 2;
        let rug_fuzz_20 = 3;
        let rug_fuzz_21 = 4;
        let rug_fuzz_22 = 1;
        let rug_fuzz_23 = 2;
        let rug_fuzz_24 = 3;
        let rug_fuzz_25 = 4;
        let rug_fuzz_26 = 5;
        let rug_fuzz_27 = 0;
        let rug_fuzz_28 = 5;
        let rug_fuzz_29 = 0;
        let rug_fuzz_30 = 10;
        let rug_fuzz_31 = 1;
        let rug_fuzz_32 = 2;
        let rug_fuzz_33 = 3;
        let rug_fuzz_34 = 4;
        let rug_fuzz_35 = 1;
        let rug_fuzz_36 = 2;
        let rug_fuzz_37 = 3;
        let rug_fuzz_38 = 4;
        let r = Repeat(rug_fuzz_0..rug_fuzz_1, rug_fuzz_2..rug_fuzz_3);
        let s = [
            rug_fuzz_4,
            rug_fuzz_5,
            rug_fuzz_6,
            rug_fuzz_7,
            rug_fuzz_8,
            rug_fuzz_9,
            rug_fuzz_10,
            rug_fuzz_11,
            rug_fuzz_12,
            rug_fuzz_13,
        ];
        let result = r.p(&s);
        debug_assert_eq!(result, Some(10));
        let r = Repeat(rug_fuzz_14..rug_fuzz_15, rug_fuzz_16..rug_fuzz_17);
        let s = [
            rug_fuzz_18,
            rug_fuzz_19,
            rug_fuzz_20,
            rug_fuzz_21,
            rug_fuzz_22,
            rug_fuzz_23,
            rug_fuzz_24,
            rug_fuzz_25,
            rug_fuzz_26,
        ];
        let result = r.p(&s);
        debug_assert_eq!(result, Some(8));
        let r = Repeat(rug_fuzz_27..rug_fuzz_28, rug_fuzz_29..rug_fuzz_30);
        let s = [
            rug_fuzz_31,
            rug_fuzz_32,
            rug_fuzz_33,
            rug_fuzz_34,
            rug_fuzz_35,
            rug_fuzz_36,
            rug_fuzz_37,
            rug_fuzz_38,
        ];
        let result = r.p(&s);
        debug_assert_eq!(result, None);
        let _rug_ed_tests_llm_16_65_rrrruuuugggg_test_p = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_72 {
    use super::*;
    use crate::*;
    struct MockParser;
    impl Peg for MockParser {
        fn p(&self, s: &[u8]) -> Option<usize> {
            Some(s.len())
        }
    }
    #[test]
    fn test_p() {
        let _rug_st_tests_llm_16_72_rrrruuuugggg_test_p = 0;
        let rug_fuzz_0 = b"abcde";
        let parser = ZeroOrMore(MockParser);
        let s: &[u8] = rug_fuzz_0;
        let result = parser.p(s);
        debug_assert_eq!(result, Some(5));
        let _rug_ed_tests_llm_16_72_rrrruuuugggg_test_p = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_73 {
    use super::*;
    use crate::*;
    use peg::Peg;
    #[test]
    fn test_p() {
        let _rug_st_tests_llm_16_73_rrrruuuugggg_test_p = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 10;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 2;
        let rug_fuzz_4 = 3;
        let rug_fuzz_5 = 4;
        let rug_fuzz_6 = 5;
        let rug_fuzz_7 = 6;
        let rug_fuzz_8 = 7;
        let rug_fuzz_9 = 8;
        let rug_fuzz_10 = 9;
        let range = std::ops::Range {
            start: rug_fuzz_0,
            end: rug_fuzz_1,
        };
        let input = [
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
            rug_fuzz_6,
            rug_fuzz_7,
            rug_fuzz_8,
            rug_fuzz_9,
            rug_fuzz_10,
        ];
        debug_assert_eq!(range.p(& input), Some(0));
        let _rug_ed_tests_llm_16_73_rrrruuuugggg_test_p = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_119 {
    use super::*;
    use crate::*;
    #[test]
    fn test_char_helper() {
        let _rug_st_tests_llm_16_119_rrrruuuugggg_test_char_helper = 0;
        let rug_fuzz_0 = "hello world";
        let rug_fuzz_1 = 'l';
        let s = rug_fuzz_0.as_bytes();
        let c = rug_fuzz_1;
        let result = char_helper(s, c);
        debug_assert_eq!(result, Some(2));
        let _rug_ed_tests_llm_16_119_rrrruuuugggg_test_char_helper = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_120 {
    use super::*;
    use crate::*;
    #[test]
    fn test_decode_utf8_empty() {
        let _rug_st_tests_llm_16_120_rrrruuuugggg_test_decode_utf8_empty = 0;
        let s: &[u8] = &[];
        debug_assert_eq!(decode_utf8(s), None);
        let _rug_ed_tests_llm_16_120_rrrruuuugggg_test_decode_utf8_empty = 0;
    }
    #[test]
    fn test_decode_utf8_single_byte() {
        let _rug_st_tests_llm_16_120_rrrruuuugggg_test_decode_utf8_single_byte = 0;
        let rug_fuzz_0 = 0x41;
        let s: &[u8] = &[rug_fuzz_0];
        debug_assert_eq!(decode_utf8(s), Some(('A', 1)));
        let _rug_ed_tests_llm_16_120_rrrruuuugggg_test_decode_utf8_single_byte = 0;
    }
    #[test]
    fn test_decode_utf8_2_bytes() {
        let _rug_st_tests_llm_16_120_rrrruuuugggg_test_decode_utf8_2_bytes = 0;
        let rug_fuzz_0 = 0xc3;
        let rug_fuzz_1 = 0xa9;
        let s: &[u8] = &[rug_fuzz_0, rug_fuzz_1];
        debug_assert_eq!(decode_utf8(s), Some(('é', 2)));
        let _rug_ed_tests_llm_16_120_rrrruuuugggg_test_decode_utf8_2_bytes = 0;
    }
    #[test]
    fn test_decode_utf8_3_bytes() {
        let _rug_st_tests_llm_16_120_rrrruuuugggg_test_decode_utf8_3_bytes = 0;
        let rug_fuzz_0 = 0xe2;
        let rug_fuzz_1 = 0x82;
        let rug_fuzz_2 = 0xac;
        let s: &[u8] = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        debug_assert_eq!(decode_utf8(s), Some(('€', 3)));
        let _rug_ed_tests_llm_16_120_rrrruuuugggg_test_decode_utf8_3_bytes = 0;
    }
    #[test]
    fn test_decode_utf8_4_bytes() {
        let _rug_st_tests_llm_16_120_rrrruuuugggg_test_decode_utf8_4_bytes = 0;
        let rug_fuzz_0 = 0xf0;
        let rug_fuzz_1 = 0x9f;
        let rug_fuzz_2 = 0x8d;
        let rug_fuzz_3 = 0x80;
        let s: &[u8] = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3];
        debug_assert_eq!(decode_utf8(s), Some(('🍀', 4)));
        let _rug_ed_tests_llm_16_120_rrrruuuugggg_test_decode_utf8_4_bytes = 0;
    }
    #[test]
    fn test_decode_utf8_invalid_2_bytes() {
        let _rug_st_tests_llm_16_120_rrrruuuugggg_test_decode_utf8_invalid_2_bytes = 0;
        let rug_fuzz_0 = 0xc3;
        let rug_fuzz_1 = 0x41;
        let s: &[u8] = &[rug_fuzz_0, rug_fuzz_1];
        debug_assert_eq!(decode_utf8(s), None);
        let _rug_ed_tests_llm_16_120_rrrruuuugggg_test_decode_utf8_invalid_2_bytes = 0;
    }
    #[test]
    fn test_decode_utf8_invalid_3_bytes() {
        let _rug_st_tests_llm_16_120_rrrruuuugggg_test_decode_utf8_invalid_3_bytes = 0;
        let rug_fuzz_0 = 0xe2;
        let rug_fuzz_1 = 0x82;
        let rug_fuzz_2 = 0x41;
        let s: &[u8] = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        debug_assert_eq!(decode_utf8(s), None);
        let _rug_ed_tests_llm_16_120_rrrruuuugggg_test_decode_utf8_invalid_3_bytes = 0;
    }
    #[test]
    fn test_decode_utf8_invalid_4_bytes() {
        let _rug_st_tests_llm_16_120_rrrruuuugggg_test_decode_utf8_invalid_4_bytes = 0;
        let rug_fuzz_0 = 0xf0;
        let rug_fuzz_1 = 0x41;
        let rug_fuzz_2 = 0x8d;
        let rug_fuzz_3 = 0x80;
        let s: &[u8] = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3];
        debug_assert_eq!(decode_utf8(s), None);
        let _rug_ed_tests_llm_16_120_rrrruuuugggg_test_decode_utf8_invalid_4_bytes = 0;
    }
}
