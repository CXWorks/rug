#[allow(unused_macros)]
#[cfg(feature = "simd")]
macro_rules! transmute_shuffle {
    ($tmp:ident, $shuffle:ident, $vec:expr, $idx_n:expr, $idx:expr) => {
        unsafe { use crate ::simd::simdint::$shuffle; use crate ::simd::simdty::$tmp; use
        core::mem::transmute; const IDX : [u32; $idx_n] = $idx; let tmp_i : $tmp =
        transmute($vec); let tmp_o : $tmp = $shuffle (tmp_i, tmp_i, IDX);
        transmute(tmp_o) }
    };
}
#[cfg(feature = "simd")]
pub mod u32x4;
#[cfg(feature = "simd")]
pub mod u64x4;
#[cfg(not(feature = "simd"))]
macro_rules! simd_opt {
    ($vec:ident) => {
        pub mod $vec { use crate ::simd::simdty::$vec; #[inline(always)] pub fn
        rotate_right_const(vec : $vec, n : u32) -> $vec { $vec ::new(vec.0
        .rotate_right(n), vec.1.rotate_right(n), vec.2.rotate_right(n), vec.3
        .rotate_right(n),) } }
    };
}
#[cfg(not(feature = "simd"))]
simd_opt!(u32x4);
#[cfg(not(feature = "simd"))]
simd_opt!(u64x4);
#[cfg(test)]
mod tests_rug_1 {
    use super::*;
    use crate::simd::simdty::Simd4;
    use crate::simd::simd_opt::u32x4;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_1_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 2;
        let mut p0: Simd4<u32> = Simd4(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3);
        let mut p1: u32 = rug_fuzz_4;
        u32x4::rotate_right_const(p0, p1);
        let _rug_ed_tests_rug_1_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_2 {
    use super::*;
    use crate::simd::simdty::Simd4;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_2_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let mut p0: Simd4<u64> = Simd4(
            u64::default(),
            u64::default(),
            u64::default(),
            u64::default(),
        );
        let mut p1: u32 = rug_fuzz_0;
        crate::simd::simd_opt::u64x4::rotate_right_const(p0, p1);
        let _rug_ed_tests_rug_2_rrrruuuugggg_test_rug = 0;
    }
}
