use super::HcidResult;
/// Pull parity bytes out that were encoded as capitalization, translate character-level erasures
/// into byte-level erasures of R-S parity symbols.  Any erasures in the capitalizataion-encoded
/// parity result in a missing byte indication.  All {char,byte}_erasures are indexed from the 0th
/// char/byte of the original full codeword, including prefix.  Returns None (erasure) or u8 value.
pub fn cap_decode(
    char_offset: usize,
    data: &[u8],
    char_erasures: &Vec<u8>,
) -> HcidResult<Option<u8>> {
    let mut bin = String::new();
    for i in 0..data.len() {
        if char_erasures[char_offset + i] == b'1' {
            bin.clear();
            break;
        }
        let c = data[i];
        if c >= b'A' && c <= b'Z' {
            bin.push('1');
        } else if c >= b'a' && c <= b'z' {
            bin.push('0');
        }
        if bin.len() >= 8 {
            break;
        }
    }
    if bin.len() < 8 { Ok(None) } else { Ok(Some(u8::from_str_radix(&bin, 2)?)) }
}
/// correct and transliteration faults
/// also note any invalid characters as erasures (character-level)
pub fn b32_correct(data: &[u8], char_erasures: &mut Vec<u8>) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    let len = data.len();
    for i in 0..len {
        out.push(
            match data[i] {
                b'0' => b'O',
                b'1' | b'l' | b'L' => b'I',
                b'2' => b'Z',
                b'A'..=b'Z' | b'a'..=b'z' | b'3'..=b'9' => data[i],
                _ => {
                    char_erasures[i] = b'1';
                    b'A'
                }
            },
        )
    }
    out
}
/// modify a character to be ascii upper-case in-place
pub fn char_lower(c: &mut u8) {
    if *c >= b'A' && *c <= b'Z' {
        *c ^= 32;
    }
}
/// modify a character to be ascii lower-case in-place
pub fn char_upper(c: &mut u8) {
    if *c >= b'a' && *c <= b'z' {
        *c ^= 32;
    }
}
/// encode `bin` into `seg` as capitalization
/// if `min` is not met, lowercase the whole thing
/// as an indication that we did not have enough alpha characters
pub fn cap_encode_bin(seg: &mut [u8], bin: &[u8], min: usize) -> HcidResult<()> {
    let mut count = 0;
    let mut bin_idx = 0;
    for c in seg.iter_mut() {
        if bin_idx >= bin.len() {
            char_lower(c);
            continue;
        }
        if (*c >= b'A' && *c <= b'Z') || (*c >= b'a' && *c <= b'z') {
            count += 1;
            if bin[bin_idx] == b'1' {
                char_upper(c);
            } else {
                char_lower(c);
            }
            bin_idx += 1;
        }
    }
    if count < min {
        for c in seg.iter_mut() {
            char_lower(c);
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests_rug_3 {
    use super::*;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1_ext, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4)) = <(usize, [u8; 13], u8, u8, u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_1 = & rug_fuzz_1_ext;
        let p0: usize = rug_fuzz_0;
        let p1: &[u8] = rug_fuzz_1;
        let mut p2: std::vec::Vec<u8> = std::vec::Vec::new();
        p2.push(rug_fuzz_2);
        p2.push(rug_fuzz_3);
        p2.push(rug_fuzz_4);
        crate::util::cap_decode(p0, p1, &p2);
             }
});    }
}
#[cfg(test)]
mod tests_rug_4 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_4_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"0l1A2z!@#";
        let p0: &[u8] = rug_fuzz_0;
        let mut p1: std::vec::Vec<u8> = std::vec::Vec::new();
        crate::util::b32_correct(p0, &mut p1);
        debug_assert_eq!(p1, b"OIZAIzAA");
        let _rug_ed_tests_rug_4_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_5 {
    use super::*;
    #[test]
    fn test_char_lower() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: u8 = rug_fuzz_0;
        crate::util::char_lower(&mut p0);
        debug_assert_eq!(p0, b'a');
             }
});    }
}
#[cfg(test)]
mod tests_rug_6 {
    use super::*;
    #[test]
    fn test_char_upper() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: u8 = rug_fuzz_0;
        crate::util::char_upper(&mut p0);
        debug_assert_eq!(p0, b'A');
             }
});    }
}
#[cfg(test)]
mod tests_rug_7 {
    use super::*;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5_ext, mut rug_fuzz_6)) = <(u8, u8, u8, u8, u8, [u8; 5], usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_5 = & rug_fuzz_5_ext;
        let mut p0 = &mut [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3, rug_fuzz_4];
        let p1 = rug_fuzz_5;
        let p2 = rug_fuzz_6;
        crate::util::cap_encode_bin(p0, p1, p2).unwrap();
             }
});    }
}
