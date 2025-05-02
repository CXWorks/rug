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
        let _rug_st_tests_rug_39_rrrruuuugggg_test_as_bytes = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 4;
        let rug_fuzz_4 = 5;
        let mut p0: [u8; 5] = [
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
        ];
        p0.as_bytes();
        let _rug_ed_tests_rug_39_rrrruuuugggg_test_as_bytes = 0;
    }
}
#[cfg(test)]
mod tests_rug_40 {
    use super::*;
    use crate::as_bytes::AsBytes;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_40_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 4;
        let mut p0: [u32; 4] = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3];
        <[u32]>::as_mut_bytes(&mut p0);
        let _rug_ed_tests_rug_40_rrrruuuugggg_test_rug = 0;
    }
}
