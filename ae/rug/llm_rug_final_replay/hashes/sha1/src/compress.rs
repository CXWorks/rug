use crate::{Block, BlockSizeUser, Sha1Core};
use digest::typenum::Unsigned;
cfg_if::cfg_if! {
    if #[cfg(feature = "force-soft")] { mod soft; use soft::compress as compress_inner; }
    else if #[cfg(all(feature = "asm", target_arch = "aarch64"))] { mod soft; mod
    aarch64; use aarch64::compress as compress_inner; } else if #[cfg(any(target_arch =
    "x86", target_arch = "x86_64"))] { #[cfg(not(feature = "asm"))] mod soft;
    #[cfg(feature = "asm")] mod soft { pub use sha1_asm::compress; } mod x86; use
    x86::compress as compress_inner; } else { mod soft; use soft::compress as
    compress_inner; }
}
const BLOCK_SIZE: usize = <Sha1Core as BlockSizeUser>::BlockSize::USIZE;
/// SHA-1 compression function
#[cfg_attr(docsrs, doc(cfg(feature = "compress")))]
pub fn compress(state: &mut [u32; 5], blocks: &[Block<Sha1Core>]) {
    let blocks: &[[u8; BLOCK_SIZE]] = unsafe {
        &*(blocks as *const _ as *const [[u8; BLOCK_SIZE]])
    };
    compress_inner(state, blocks);
}
#[cfg(test)]
mod tests_rug_226 {
    use super::*;
    use crate::{digest::generic_array::GenericArray, Block, Sha1Core};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u32, u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: [u32; 5] = [rug_fuzz_0; 5];
        let p1: &[[u8; 64]] = &[[rug_fuzz_1; 64]];
        crate::compress::compress(
            &mut p0,
            unsafe { &*(p1 as *const _ as *const [Block<Sha1Core>]) },
        );
             }
}
}
}    }
}
