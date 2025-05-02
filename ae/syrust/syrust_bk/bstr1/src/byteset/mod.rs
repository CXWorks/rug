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
        2 => scalar::forward_search_bytes(haystack, |b| {
            b != byteset[0] && b != byteset[1]
        }),
        3 => scalar::forward_search_bytes(haystack, |b| {
            b != byteset[0] && b != byteset[1] && b != byteset[2]
        }),
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
        2 => scalar::reverse_search_bytes(haystack, |b| {
            b != byteset[0] && b != byteset[1]
        }),
        3 => scalar::reverse_search_bytes(haystack, |b| {
            b != byteset[0] && b != byteset[1] && b != byteset[2]
        }),
        _ => {
            let table = build_table(byteset);
            scalar::reverse_search_bytes(haystack, |b| table[b as usize] == 0)
        }
    }
}

#[cfg(test)]
mod tests {

    quickcheck! {
        fn qc_byteset_forward_matches_naive(
            haystack: Vec<u8>,
            needles: Vec<u8>
        ) -> bool {
            super::find(&haystack, &needles)
                == haystack.iter().position(|b| needles.contains(b))
        }
        fn qc_byteset_backwards_matches_naive(
            haystack: Vec<u8>,
            needles: Vec<u8>
        ) -> bool {
            super::rfind(&haystack, &needles)
                == haystack.iter().rposition(|b| needles.contains(b))
        }
        fn qc_byteset_forward_not_matches_naive(
            haystack: Vec<u8>,
            needles: Vec<u8>
        ) -> bool {
            super::find_not(&haystack, &needles)
                == haystack.iter().position(|b| !needles.contains(b))
        }
        fn qc_byteset_backwards_not_matches_naive(
            haystack: Vec<u8>,
            needles: Vec<u8>
        ) -> bool {
            super::rfind_not(&haystack, &needles)
                == haystack.iter().rposition(|b| !needles.contains(b))
        }
    }
}
#[cfg(test)]
mod tests_rug_16 {
    use super::*;

    #[test]
    fn test_build_table() {
        let p0: &[u8] = b"abcde";
        
        let result = build_table(p0);
        
        assert_eq!(result[b'a' as usize], 1);
        assert_eq!(result[b'b' as usize], 1);
        assert_eq!(result[b'c' as usize], 1);
        assert_eq!(result[b'd' as usize], 1);
        assert_eq!(result[b'e' as usize], 1);
        
        for i in 0..256 {
            if p0.contains(&(i as u8)) {
                assert_eq!(result[i], 1);
            } else {
                assert_eq!(result[i], 0);
            }
        }
    }
}#[cfg(test)]
mod tests_rug_17 {
    use super::*;
    use crate::byteset::{find, build_table, scalar::forward_search_bytes, memchr, memchr2, memchr3};

    #[test]
    fn test_rug() {
        let p0: &[u8] = b"example haystack data";
        let p1: &[u8] = b"set";

        find(p0, p1);

    }
}
#[cfg(test)]
mod tests_rug_18 {
    use super::*;
    use crate::byteset;
    
    #[test]
    fn test_rug() {
        let p0: &[u8] = b"example_haystack_data";
        let p1: &[u8] = b"set";
        
        byteset::rfind(p0, p1);
    }
}
#[cfg(test)]
mod tests_rug_19 {
    use super::*;

    #[test]
    fn test_rug() {
        let p0: &[u8] = b"hello world";
        let p1: &[u8] = b"abc";

        crate::byteset::find_not(p0, p1);
    }
}#[cfg(test)]
mod tests_rug_20 {
    use super::*;
    use crate::byteset::scalar;
    use crate::byteset::build_table;

    #[test]
    fn test_rug() {
        let p0: &[u8] = b"examplehaystackdata";
        let p1: &[u8] = b"set";

        crate::byteset::rfind_not(p0, p1);

    }
}