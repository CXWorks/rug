use core::convert::TryInto;
#[cfg(feature = "zeroize")]
use zeroize::{Zeroize, ZeroizeOnDrop};
const PLEN: usize = 25;
const DEFAULT_ROUND_COUNT: usize = 24;
#[derive(Clone)]
pub(crate) struct Sha3State {
    pub state: [u64; PLEN],
    round_count: usize,
}
impl Default for Sha3State {
    fn default() -> Self {
        Self {
            state: [0u64; PLEN],
            round_count: DEFAULT_ROUND_COUNT,
        }
    }
}
#[cfg(feature = "zeroize")]
impl Drop for Sha3State {
    fn drop(&mut self) {
        self.state.zeroize();
    }
}
#[cfg(feature = "zeroize")]
impl ZeroizeOnDrop for Sha3State {}
impl Sha3State {
    pub(crate) fn new(round_count: usize) -> Self {
        Self {
            state: [0u64; PLEN],
            round_count,
        }
    }
    #[inline(always)]
    pub(crate) fn absorb_block(&mut self, block: &[u8]) {
        debug_assert_eq!(block.len() % 8, 0);
        for (b, s) in block.chunks_exact(8).zip(self.state.iter_mut()) {
            *s ^= u64::from_le_bytes(b.try_into().unwrap());
        }
        keccak::p1600(&mut self.state, self.round_count);
    }
    #[inline(always)]
    pub(crate) fn as_bytes(&self, out: &mut [u8]) {
        for (o, s) in out.chunks_mut(8).zip(self.state.iter()) {
            o.copy_from_slice(&s.to_le_bytes()[..o.len()]);
        }
    }
    #[inline(always)]
    pub(crate) fn permute(&mut self) {
        keccak::p1600(&mut self.state, self.round_count);
    }
}
#[cfg(test)]
mod tests_rug_283 {
    use super::*;
    use crate::state::Sha3State;
    use std::default::Default;
    #[test]
    fn test_default() {
        let _rug_st_tests_rug_283_rrrruuuugggg_test_default = 0;
        <Sha3State as Default>::default();
        let _rug_ed_tests_rug_283_rrrruuuugggg_test_default = 0;
    }
}
#[cfg(test)]
mod tests_rug_284 {
    use super::*;
    use crate::state::Sha3State;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let round_count: usize = rug_fuzz_0;
        Sha3State::new(round_count);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_285 {
    use super::*;
    use crate::state::Sha3State;
    #[test]
    fn test_absorb_block() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5, mut rug_fuzz_6, mut rug_fuzz_7)) = <(u8, u8, u8, u8, u8, u8, u8, u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Sha3State::default();
        let p1: &[u8] = &[
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
            rug_fuzz_6,
            rug_fuzz_7,
        ];
        p0.absorb_block(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_286 {
    use super::*;
    use crate::state::Sha3State;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Sha3State::default();
        let mut p1 = [rug_fuzz_0; 64];
        Sha3State::as_bytes(&p0, &mut p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_287 {
    use super::*;
    use crate::state::Sha3State;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_287_rrrruuuugggg_test_rug = 0;
        let mut p0 = Sha3State::default();
        Sha3State::permute(&mut p0);
        let _rug_ed_tests_rug_287_rrrruuuugggg_test_rug = 0;
    }
}
