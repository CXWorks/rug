use memchr::{memchr, memchr2, memchr3, memrchr, memrchr2, memrchr3};
mod scalar;
#[inline]
fn build_table(byteset: &[u8]) -> [u8; 256] {
    let mut table = [0u8; 256];
    for &b in byteset {
        table[b as usize] = 1;
    }
    table
}
#[inline]
pub(crate) fn find(haystack: &[u8], byteset: &[u8]) -> Option<usize> {
    match byteset.len() {
        0 => return None,
        1 => memchr(byteset[0], haystack),
        2 => memchr2(byteset[0], byteset[1], haystack),
        3 => memchr3(byteset[0], byteset[1], byteset[2], haystack),
        _ => {
            let table = build_table(byteset);
            scalar::forward_search_bytes(haystack, |b| table[b as usize] != 0)
        }
    }
}
#[inline]
pub(crate) fn rfind(haystack: &[u8], byteset: &[u8]) -> Option<usize> {
    match byteset.len() {
        0 => return None,
        1 => memrchr(byteset[0], haystack),
        2 => memrchr2(byteset[0], byteset[1], haystack),
        3 => memrchr3(byteset[0], byteset[1], byteset[2], haystack),
        _ => {
            let table = build_table(byteset);
            scalar::reverse_search_bytes(haystack, |b| table[b as usize] != 0)
        }
    }
}
#[inline]
pub(crate) fn find_not(haystack: &[u8], byteset: &[u8]) -> Option<usize> {
    if haystack.is_empty() {
        return None;
    }
    match byteset.len() {
        0 => return Some(0),
        1 => scalar::inv_memchr(byteset[0], haystack),
        2 => {
            scalar::forward_search_bytes(
                haystack,
                |b| { b != byteset[0] && b != byteset[1] },
            )
        }
        3 => {
            scalar::forward_search_bytes(
                haystack,
                |b| { b != byteset[0] && b != byteset[1] && b != byteset[2] },
            )
        }
        _ => {
            let table = build_table(byteset);
            scalar::forward_search_bytes(haystack, |b| table[b as usize] == 0)
        }
    }
}
#[inline]
pub(crate) fn rfind_not(haystack: &[u8], byteset: &[u8]) -> Option<usize> {
    if haystack.is_empty() {
        return None;
    }
    match byteset.len() {
        0 => return Some(haystack.len() - 1),
        1 => scalar::inv_memrchr(byteset[0], haystack),
        2 => {
            scalar::reverse_search_bytes(
                haystack,
                |b| { b != byteset[0] && b != byteset[1] },
            )
        }
        3 => {
            scalar::reverse_search_bytes(
                haystack,
                |b| { b != byteset[0] && b != byteset[1] && b != byteset[2] },
            )
        }
        _ => {
            let table = build_table(byteset);
            scalar::reverse_search_bytes(haystack, |b| table[b as usize] == 0)
        }
    }
}
#[cfg(test)]
mod tests {
    quickcheck! {
        fn qc_byteset_forward_matches_naive(haystack : Vec < u8 >, needles : Vec < u8 >)
        -> bool { super::find(& haystack, & needles) == haystack.iter().position(| b |
        needles.contains(b)) } fn qc_byteset_backwards_matches_naive(haystack : Vec < u8
        >, needles : Vec < u8 >) -> bool { super::rfind(& haystack, & needles) ==
        haystack.iter().rposition(| b | needles.contains(b)) } fn
        qc_byteset_forward_not_matches_naive(haystack : Vec < u8 >, needles : Vec < u8 >)
        -> bool { super::find_not(& haystack, & needles) == haystack.iter().position(| b
        | ! needles.contains(b)) } fn qc_byteset_backwards_not_matches_naive(haystack :
        Vec < u8 >, needles : Vec < u8 >) -> bool { super::rfind_not(& haystack, &
        needles) == haystack.iter().rposition(| b | ! needles.contains(b)) }
    }
}
#[cfg(test)]
mod tests_rug_16 {
    use super::*;
    #[test]
    fn test_build_table() {
        let _rug_st_tests_rug_16_rrrruuuugggg_test_build_table = 0;
        let rug_fuzz_0 = b"abcde";
        let rug_fuzz_1 = b'a';
        let rug_fuzz_2 = b'b';
        let rug_fuzz_3 = b'c';
        let rug_fuzz_4 = b'd';
        let rug_fuzz_5 = b'e';
        let rug_fuzz_6 = 0;
        let rug_fuzz_7 = 256;
        let p0: &[u8] = rug_fuzz_0;
        let result = build_table(p0);
        debug_assert_eq!(result[rug_fuzz_1 as usize], 1);
        debug_assert_eq!(result[rug_fuzz_2 as usize], 1);
        debug_assert_eq!(result[rug_fuzz_3 as usize], 1);
        debug_assert_eq!(result[rug_fuzz_4 as usize], 1);
        debug_assert_eq!(result[rug_fuzz_5 as usize], 1);
        for i in rug_fuzz_6..rug_fuzz_7 {
            if p0.contains(&(i as u8)) {
                debug_assert_eq!(result[i], 1);
            } else {
                debug_assert_eq!(result[i], 0);
            }
        }
        let _rug_ed_tests_rug_16_rrrruuuugggg_test_build_table = 0;
    }
}
#[cfg(test)]
mod tests_rug_17 {
    use super::*;
    use crate::byteset::{
        find, build_table, scalar::forward_search_bytes, memchr, memchr2, memchr3,
    };
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_17_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"example haystack data";
        let rug_fuzz_1 = b"set";
        let p0: &[u8] = rug_fuzz_0;
        let p1: &[u8] = rug_fuzz_1;
        find(p0, p1);
        let _rug_ed_tests_rug_17_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_18 {
    use super::*;
    use crate::byteset;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_18_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"example_haystack_data";
        let rug_fuzz_1 = b"set";
        let p0: &[u8] = rug_fuzz_0;
        let p1: &[u8] = rug_fuzz_1;
        byteset::rfind(p0, p1);
        let _rug_ed_tests_rug_18_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_19 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_19_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"hello world";
        let rug_fuzz_1 = b"abc";
        let p0: &[u8] = rug_fuzz_0;
        let p1: &[u8] = rug_fuzz_1;
        crate::byteset::find_not(p0, p1);
        let _rug_ed_tests_rug_19_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_20 {
    use super::*;
    use crate::byteset::scalar;
    use crate::byteset::build_table;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_20_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"examplehaystackdata";
        let rug_fuzz_1 = b"set";
        let p0: &[u8] = rug_fuzz_0;
        let p1: &[u8] = rug_fuzz_1;
        crate::byteset::rfind_not(p0, p1);
        let _rug_ed_tests_rug_20_rrrruuuugggg_test_rug = 0;
    }
}
