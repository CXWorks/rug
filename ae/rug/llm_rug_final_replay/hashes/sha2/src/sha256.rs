use digest::{generic_array::GenericArray, typenum::U64};
cfg_if::cfg_if! {
    if #[cfg(feature = "force-soft")] { mod soft; use soft::compress; } else if
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))] { #[cfg(not(feature =
    "asm"))] mod soft; #[cfg(feature = "asm")] mod soft { pub (crate) use
    sha2_asm::compress256 as compress; } mod x86; use x86::compress; } else if
    #[cfg(all(feature = "asm", target_arch = "aarch64"))] { mod soft; mod aarch64; use
    aarch64::compress; } else { mod soft; use soft::compress; }
}
/// Raw SHA-256 compression function.
///
/// This is a low-level "hazmat" API which provides direct access to the core
/// functionality of SHA-256.
#[cfg_attr(docsrs, doc(cfg(feature = "compress")))]
pub fn compress256(state: &mut [u32; 8], blocks: &[GenericArray<u8, U64>]) {
    let p = blocks.as_ptr() as *const [u8; 64];
    let blocks = unsafe { core::slice::from_raw_parts(p, blocks.len()) };
    compress(state, blocks)
}
#[cfg(test)]
mod tests_rug_249 {
    use super::*;
    use crate::{Sha256, Digest};
    use digest::generic_array::GenericArray;
    use digest::typenum::{U64, B0, B1, UInt, UTerm};
    #[test]
    fn test_compress256() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut state: [u32; 8] = [rug_fuzz_0; 8];
        let blocks: [GenericArray<u8, U64>; 1] = [GenericArray::<u8, U64>::default()];
        compress256(&mut state, &blocks);
             }
}
}
}    }
}
