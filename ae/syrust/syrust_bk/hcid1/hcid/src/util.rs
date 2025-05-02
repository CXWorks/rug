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

    // iterate over input data
    for i in 0..data.len() {
        if char_erasures[char_offset + i] == b'1' {
            // If char is known to be lost, parity byte will be marked as an erasure
            bin.clear();
            break;
        }

        let c = data[i];

        // is alpha
        if c >= b'A' && c <= b'Z' {
            // uppercase = bit on
            bin.push('1');
        } else if c >= b'a' && c <= b'z' {
            // lowercase = bit off
            bin.push('0');
        }

        // we have our 8 bits! proceed
        if bin.len() >= 8 {
            break;
        }
    }

    // We did not get a full byte iff we don't have 8 bits.  Later (when the caller has all decoded
    // parity), it should be determined if they are all 1's/0's, indicating capitalization is lost.
    if bin.len() < 8 {
        Ok(None)
    } else {
        Ok(Some(u8::from_str_radix(&bin, 2)?))
    }
}

/// correct and transliteration faults
/// also note any invalid characters as erasures (character-level)
pub fn b32_correct(data: &[u8], char_erasures: &mut Vec<u8>) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();

    let len = data.len();
    for i in 0..len {
        out.push(match data[i] {
            b'0' => b'O',
            b'1' | b'l' | b'L' => b'I',
            b'2' => b'Z',
            b'A'..=b'Z' | b'a'..=b'z' | b'3'..=b'9' => data[i],
            _ => {
                // we cannot translate this character
                // mark it as an erasure... see if we can continue
                char_erasures[i] = b'1';
                b'A'
            }
        })
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
        // is alpha
        if (*c >= b'A' && *c <= b'Z') || (*c >= b'a' && *c <= b'z') {
            count += 1;
            // is 1
            if bin[bin_idx] == b'1' {
                // the bit is on: uppercase
                char_upper(c);
            } else {
                // the bit is off: lowercase
                char_lower(c);
            }
            bin_idx += 1;
        }
    }
    if count < min {
        // not enough alpha characters to encode the min
        // mark everything as lower
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
        let p0: usize = 0;
        let p1: &[u8] = b"Hello, World!";
        
        let mut p2: std::vec::Vec<u8> = std::vec::Vec::new();
        p2.push(0);
        p2.push(1);
        p2.push(0);
        
        crate::util::cap_decode(p0, p1, &p2);

    }
}

#[cfg(test)]
mod tests_rug_4 {
    use super::*;

    #[test]
    fn test_rug() {
        let p0: &[u8] = b"0l1A2z!@#";
        let mut p1: std::vec::Vec<u8> = std::vec::Vec::new();
        
        crate::util::b32_correct(p0, &mut p1);

        assert_eq!(p1, b"OIZAIzAA");
    }
}
#[cfg(test)]
mod tests_rug_5 {
    use super::*;
    
    #[test]
    fn test_char_lower() {
        let mut p0: u8 = b'A';

        crate::util::char_lower(&mut p0);
        assert_eq!(p0, b'a');
    }
}#[cfg(test)]
mod tests_rug_6 {
    use super::*;

    #[test]
    fn test_char_upper() {
        let mut p0: u8 = b'a';

        crate::util::char_upper(&mut p0);
        assert_eq!(p0, b'A');
    }
}
#[cfg(test)]
mod tests_rug_7 {
    use super::*;
    
    #[test]
    fn test_rug() {
        let mut p0 = &mut [b'A', b'B', b'C', b'D', b'E']; // Sample data
        let p1 = b"10100"; // Sample data
        let p2 = 3; // Sample data
        
        crate::util::cap_encode_bin(p0, p1, p2).unwrap();
        
        // Add assertions here based on test cases
    }
}
