use core::mem;
use core::slice;
#[allow(clippy::missing_safety_doc)]
pub unsafe trait Safe {}
pub trait AsBytes {
    fn as_bytes(&self) -> &[u8];
    fn as_mut_bytes(&mut self) -> &mut [u8];
}
impl<T: Safe> AsBytes for [T] {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(
                self.as_ptr() as *const u8,
                self.len() * mem::size_of::<T>(),
            )
        }
    }
    #[inline]
    fn as_mut_bytes(&mut self) -> &mut [u8] {
        unsafe {
            slice::from_raw_parts_mut(
                self.as_mut_ptr() as *mut u8,
                self.len() * mem::size_of::<T>(),
            )
        }
    }
}
unsafe impl Safe for u8 {}
unsafe impl Safe for u16 {}
unsafe impl Safe for u32 {}
unsafe impl Safe for u64 {}
unsafe impl Safe for i8 {}
unsafe impl Safe for i16 {}
unsafe impl Safe for i32 {}
unsafe impl Safe for i64 {}
#[cfg(test)]
mod tests_rug_39 {
    use super::*;
    use crate::as_bytes::AsBytes;
    #[test]
    fn test_as_bytes() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4)) = <(u8, u8, u8, u8, u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: [u8; 5] = [
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
        ];
        p0.as_bytes();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_40 {
    use super::*;
    use crate::as_bytes::AsBytes;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(u32, u32, u32, u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: [u32; 4] = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3];
        <[u32]>::as_mut_bytes(&mut p0);
             }
}
}
}    }
}
