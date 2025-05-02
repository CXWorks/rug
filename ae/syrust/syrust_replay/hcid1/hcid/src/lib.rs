//! Holochain HCID base32 encoding utility.
//!
//! # Example
//!
//! ```
//! extern crate hcid;
//!
//! fn main() {
//!     let enc = hcid::HcidEncoding::with_kind("hcs0").unwrap();
//!     let key = enc.encode(&[0; 32]).unwrap();
//!     assert_eq!("HcSciaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", key);
//!     let buffer = enc.decode(&key).unwrap();
//!     assert_eq!([0; 32].to_vec(), buffer);
//! }
//! ```
extern crate reed_solomon;
mod error;
mod b32;
pub use error::{HcidError, HcidResult};
mod util;
use util::{b32_correct, cap_decode, cap_encode_bin, char_upper};
static HC_CODE_MAP: &'static [[u8; 2]] = &[
    [0xb2, 0xff],
    [0xb4, 0xff],
    [0xb6, 0xff],
    [0xb8, 0xff],
    [0xba, 0xff],
    [0xbc, 0xff],
    [0xbe, 0xff],
    [0xff, 0xff],
    [0xff, 0xff],
    [0xff, 0xff],
    [0xff, 0xff],
    [0xff, 0xff],
    [0xff, 0xff],
    [0xff, 0xff],
    [0xff, 0xff],
    [0xff, 0xff],
    [0xff, 0xff],
    [0xff, 0xff],
    [0xff, 0xff],
    [0xff, 0xff],
    [0xff, 0xff],
    [0xff, 0xff],
    [0xff, 0xff],
    [0xff, 0xff],
    [0xff, 0xff],
    [0xff, 0xff],
    [0xff, 0xff],
    [0xff, 0xff],
    [0xff, 0xff],
    [0xff, 0xff],
    [0xff, 0xff],
    [0xff, 0xff],
    [0xff, 0xff],
    [0xff, 0xff],
    [0xff, 0xff],
    [0xff, 0xff],
    [0xff, 0xff],
    [0xff, 0xff],
    [0xff, 0xff],
    [0xff, 0xff],
    [0xff, 0xff],
    [0xff, 0xff],
    [0xff, 0xff],
    [0xff, 0xff],
    [0xff, 0xff],
    [0xff, 0xff],
    [0x80, 0xff],
    [0x82, 0xff],
    [0x84, 0xff],
    [0x86, 0xff],
    [0x88, 0xff],
    [0x8a, 0xff],
    [0x8c, 0xff],
    [0x8e, 0xff],
    [0x90, 0xff],
    [0x92, 0xff],
    [0x94, 0xff],
    [0xff, 0xff],
    [0x96, 0xff],
    [0x98, 0xff],
    [0x9a, 0xff],
    [0x9c, 0xff],
    [0x9e, 0xff],
    [0xa0, 0xff],
    [0xa2, 0xff],
    [0xa4, 0xff],
    [0xa6, 0xff],
    [0xa8, 0xff],
    [0xaa, 0xff],
    [0xad, 0xff],
    [0xae, 0xff],
    [0xb0, 0xff],
];
/// represents an encoding configuration for hcid rendering and parsing
pub struct HcidEncodingConfig {
    /// byte count of actuall key data that will be encoded
    pub key_byte_count: usize,
    /// parity bytes that will be encoded directly into the base32 string (appended after key)
    pub base_parity_byte_count: usize,
    /// parity bytes that will be encoded in the alpha capitalization (appended after base parity)
    pub cap_parity_byte_count: usize,
    /// bytes to prefix before rendering to base32
    pub prefix: Vec<u8>,
    /// binary indication of the capitalization for prefix characters
    pub prefix_cap: Vec<u8>,
    /// how many characters are in a capitalization parity segment
    pub cap_segment_char_count: usize,
    /// how many characters long the fully rendered base32 string should be
    pub encoded_char_count: usize,
}
impl HcidEncodingConfig {
    /// create a new config given a kind token string
    ///
    /// # Example
    ///
    /// ```
    /// extern crate hcid;
    /// let hca0 = hcid::HcidEncodingConfig::new("hca0").unwrap();
    /// let hck0 = hcid::HcidEncodingConfig::new("hck0").unwrap();
    /// let hcs0 = hcid::HcidEncodingConfig::new("hcs0").unwrap();
    /// ```
    pub fn new(kind: &str) -> HcidResult<Self> {
        let kind_b = kind.as_bytes();
        if kind_b.len() != 4 || kind_b[0] != 104 || kind_b[1] != 99
            || (kind_b[3] != 48 && kind_b[3] != 49) || kind_b[2] < 51 || kind_b[2] > 122
        {
            return Err(format!("invalid kind: `{}`", kind).into());
        }
        let version = if kind_b[3] == 48 { 0 } else { 1 };
        let res = HC_CODE_MAP[(kind_b[2] - 51) as usize][version as usize];
        if res == 0xff {
            return Err(format!("invalid kind: `{}`", kind).into());
        }
        Ok(HcidEncodingConfig {
            key_byte_count: 32,
            base_parity_byte_count: 4,
            cap_parity_byte_count: 4,
            prefix: vec![0x38, res, 0x24],
            prefix_cap: b"101".to_vec(),
            cap_segment_char_count: 15,
            encoded_char_count: 63,
        })
    }
}
/// an instance that can encode / decode a particular hcid encoding configuration
pub struct HcidEncoding {
    config: HcidEncodingConfig,
    rs_enc: reed_solomon::Encoder,
    rs_dec: reed_solomon::Decoder,
}
impl HcidEncoding {
    /// create a new HcidEncoding instance from given HcidEncodingConfig
    pub fn new(config: HcidEncodingConfig) -> HcidResult<Self> {
        let rs_enc = reed_solomon::Encoder::new(
            config.base_parity_byte_count + config.cap_parity_byte_count,
        );
        let rs_dec = reed_solomon::Decoder::new(
            config.base_parity_byte_count + config.cap_parity_byte_count,
        );
        Ok(Self { config, rs_enc, rs_dec })
    }
    /// create a new config given a kind token string
    ///
    /// # Example
    ///
    /// ```
    /// extern crate hcid;
    /// let hca0 = hcid::HcidEncoding::with_kind("hca0").unwrap();
    /// let hck0 = hcid::HcidEncoding::with_kind("hck0").unwrap();
    /// let hcs0 = hcid::HcidEncoding::with_kind("hcs0").unwrap();
    /// ```
    pub fn with_kind(kind: &str) -> HcidResult<Self> {
        HcidEncoding::new(HcidEncodingConfig::new(kind)?)
    }
    /// encode a string to base32 with this instance's configuration
    pub fn encode(&self, data: &[u8]) -> HcidResult<String> {
        if data.len() != self.config.key_byte_count {
            return Err(
                HcidError(
                    String::from(
                        format!(
                            "BadDataLen:{},Expected:{}", data.len(), self.config
                            .key_byte_count
                        ),
                    ),
                ),
            );
        }
        let full_parity = self.rs_enc.encode(data);
        let cap_bytes = &full_parity[full_parity.len()
            - self.config.cap_parity_byte_count..];
        let mut base = self.config.prefix.clone();
        base.extend_from_slice(
            &full_parity[0..full_parity.len() - self.config.cap_parity_byte_count],
        );
        let mut base32 = b32::encode(&base);
        if base32.len() != self.config.encoded_char_count {
            return Err(
                HcidError(
                    String::from(
                        format!(
                            "InternalGeneratedBadLen:{},Expected:{}", base32.len(), self
                            .config.encoded_char_count
                        ),
                    ),
                ),
            );
        }
        cap_encode_bin(
            &mut base32[0..self.config.prefix_cap.len()],
            &self.config.prefix_cap,
            3,
        )?;
        for i in 0..cap_bytes.len() {
            let seg_start = self.config.prefix_cap.len()
                + (i * self.config.cap_segment_char_count);
            let seg = &mut base32[seg_start..seg_start
                + self.config.cap_segment_char_count];
            let bin = format!("{:08b}", cap_bytes[i]).into_bytes();
            cap_encode_bin(seg, &bin, 8)?;
        }
        unsafe { Ok(String::from_utf8_unchecked(base32)) }
    }
    /// decode the data from a base32 string with this instance's configuration.  Reed-Solomon can
    /// correct up to 1/2 its parity size worth of erasures (if no other errors are present).
    pub fn decode(&self, data: &str) -> HcidResult<Vec<u8>> {
        let (data, erasures) = self.pre_decode(data)?;
        if erasures.len()
            > (self.config.base_parity_byte_count + self.config.cap_parity_byte_count)
                / 2
        {
            return Err(HcidError(String::from("TooManyErrors")));
        }
        if self.pre_is_corrupt(&data, &erasures)? {
            Ok(
                self
                    .rs_dec
                    .correct(&data, Some(&erasures[..]))?[0..self.config.key_byte_count]
                    .to_vec(),
            )
        } else {
            Ok(data[0..self.config.key_byte_count].to_vec())
        }
    }
    /// a lighter-weight check to determine if a base32 string is corrupt
    pub fn is_corrupt(&self, data: &str) -> HcidResult<bool> {
        let (data, erasures) = match self.pre_decode(data) {
            Ok(v) => v,
            Err(_) => return Ok(true),
        };
        match self.pre_is_corrupt(&data, &erasures) {
            Ok(v) => Ok(v),
            Err(_) => Ok(true),
        }
    }
    /// internal helper for is_corrupt checking
    fn pre_is_corrupt(&self, data: &[u8], erasures: &[u8]) -> HcidResult<bool> {
        if erasures.len() > 0 {
            return Ok(true);
        }
        Ok(self.rs_dec.is_corrupted(&data))
    }
    /// internal helper for preparing decoding
    fn pre_decode(&self, data: &str) -> HcidResult<(Vec<u8>, Vec<u8>)> {
        if data.len() != self.config.encoded_char_count {
            return Err(
                HcidError(
                    String::from(
                        format!(
                            "BadIdLen:{},Expected:{}", data.len(), self.config
                            .encoded_char_count
                        ),
                    ),
                ),
            );
        }
        let key_base_byte_size = self.config.key_byte_count
            + self.config.base_parity_byte_count;
        let mut byte_erasures = vec![
            b'0'; key_base_byte_size + self.config.cap_parity_byte_count
        ];
        let mut char_erasures = vec![b'0'; data.len()];
        let mut data = b32_correct(data.as_bytes(), &mut char_erasures);
        let mut cap_bytes: Vec<u8> = Vec::new();
        let mut all_zro = true;
        let mut all_one = true;
        for i in 0..self.config.cap_parity_byte_count {
            let char_idx = self.config.prefix_cap.len()
                + (i * self.config.cap_segment_char_count);
            match cap_decode(
                char_idx,
                &data[char_idx..char_idx + self.config.cap_segment_char_count],
                &char_erasures,
            )? {
                None => {
                    byte_erasures[key_base_byte_size + i] = b'1';
                    cap_bytes.push(0)
                }
                Some(parity) => {
                    if all_zro && parity != 0x00_u8 {
                        all_zro = false;
                    }
                    if all_one && parity != 0xFF_u8 {
                        all_one = false;
                    }
                    cap_bytes.push(parity)
                }
            }
        }
        if all_zro || all_one {
            for i in 0..self.config.cap_parity_byte_count {
                byte_erasures[key_base_byte_size + i] = b'1';
            }
        }
        for c in data.iter_mut() {
            char_upper(c);
        }
        let mut data = b32::decode(&data)?;
        if &data[0..self.config.prefix.len()] != self.config.prefix.as_slice() {
            return Err(HcidError(String::from("PrefixMismatch")));
        }
        data.drain(0..self.config.prefix.len());
        data.append(&mut cap_bytes);
        for i in self.config.prefix_cap.len()..char_erasures.len() {
            let c = char_erasures[i];
            if c == b'1' {
                byte_erasures[(i * 5 + 0) / 8 - self.config.prefix.len()] = b'1';
                byte_erasures[(i * 5 + 4) / 8 - self.config.prefix.len()] = b'1';
            }
        }
        let mut erasures: Vec<u8> = Vec::new();
        for i in 0..byte_erasures.len() {
            if byte_erasures[i] == b'1' {
                data[i] = 0;
                erasures.push(i as u8);
            }
        }
        Ok((data, erasures))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    static TEST_HEX_1: &'static str = "0c71db50d35d760b0ea2002ff20147c7c3a8e8030d35ef28ed1adaec9e329aba";
    static TEST_ID_1: &'static str = "HcKciDds5OiogymxbnHKEabQ8iavqs8dwdVaGdJW76Vp4gx47tQDfGW4OWc9w5i";
    #[test]
    fn it_encodes_1() {
        let enc = HcidEncoding::with_kind("hck0").unwrap();
        let input = hex::decode(TEST_HEX_1.as_bytes()).unwrap();
        let id = enc.encode(&input).unwrap();
        assert_eq!(TEST_ID_1, id);
    }
    #[test]
    fn it_decodes_1() {
        let enc = HcidEncoding::with_kind("hck0").unwrap();
        let data = hex::encode(&enc.decode(TEST_ID_1).unwrap());
        assert_eq!(TEST_HEX_1, data);
    }
}
#[cfg(test)]
mod tests_rug_14 {
    use super::*;
    use crate::HcidEncodingConfig;
    #[test]
    fn test_hcid_encoding_config_new() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = rug_fuzz_0;
        HcidEncodingConfig::new(&p0).unwrap();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_15 {
    use super::*;
    use crate::{HcidEncoding, HcidEncodingConfig};
    #[test]
    fn test_hcid_encoding_new() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0 = HcidEncodingConfig::new(rug_fuzz_0).unwrap();
        let result = HcidEncoding::new(p0);
        debug_assert!(result.is_ok());
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_16 {
    use super::*;
    #[test]
    fn test_with_kind() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: &str = rug_fuzz_0;
        HcidEncoding::with_kind(p0).unwrap();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_17 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_17_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 16;
        let rug_fuzz_1 = 8;
        let rug_fuzz_2 = 4;
        let rug_fuzz_3 = b"prefix";
        let rug_fuzz_4 = 2;
        let rug_fuzz_5 = 30;
        let rug_fuzz_6 = b"PREFIX";
        let rug_fuzz_7 = b"test_data";
        let config = HcidEncodingConfig {
            key_byte_count: rug_fuzz_0,
            base_parity_byte_count: rug_fuzz_1,
            cap_parity_byte_count: rug_fuzz_2,
            prefix: rug_fuzz_3.to_vec(),
            cap_segment_char_count: rug_fuzz_4,
            encoded_char_count: rug_fuzz_5,
            prefix_cap: rug_fuzz_6.to_vec(),
        };
        let hcid_enc = HcidEncoding::new(config).unwrap();
        let data = rug_fuzz_7;
        hcid_enc.encode(data).unwrap();
        let _rug_ed_tests_rug_17_rrrruuuugggg_test_rug = 0;
    }
}
