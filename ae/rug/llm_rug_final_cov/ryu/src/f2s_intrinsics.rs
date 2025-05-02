use crate::d2s;
pub const FLOAT_POW5_INV_BITCOUNT: i32 = d2s::DOUBLE_POW5_INV_BITCOUNT - 64;
pub const FLOAT_POW5_BITCOUNT: i32 = d2s::DOUBLE_POW5_BITCOUNT - 64;
#[cfg_attr(feature = "no-panic", inline)]
fn pow5factor_32(mut value: u32) -> u32 {
    let mut count = 0u32;
    loop {
        debug_assert!(value != 0);
        let q = value / 5;
        let r = value % 5;
        if r != 0 {
            break;
        }
        value = q;
        count += 1;
    }
    count
}
#[cfg_attr(feature = "no-panic", inline)]
pub fn multiple_of_power_of_5_32(value: u32, p: u32) -> bool {
    pow5factor_32(value) >= p
}
#[cfg_attr(feature = "no-panic", inline)]
pub fn multiple_of_power_of_2_32(value: u32, p: u32) -> bool {
    (value & ((1u32 << p) - 1)) == 0
}
#[cfg_attr(feature = "no-panic", inline)]
fn mul_shift_32(m: u32, factor: u64, shift: i32) -> u32 {
    debug_assert!(shift > 32);
    let factor_lo = factor as u32;
    let factor_hi = (factor >> 32) as u32;
    let bits0 = m as u64 * factor_lo as u64;
    let bits1 = m as u64 * factor_hi as u64;
    let sum = (bits0 >> 32) + bits1;
    let shifted_sum = sum >> (shift - 32);
    debug_assert!(shifted_sum <= u32::max_value() as u64);
    shifted_sum as u32
}
#[cfg_attr(feature = "no-panic", inline)]
pub fn mul_pow5_inv_div_pow2(m: u32, q: u32, j: i32) -> u32 {
    #[cfg(feature = "small")]
    {
        let pow5 = unsafe { d2s::compute_inv_pow5(q) };
        mul_shift_32(m, pow5.1 + 1, j)
    }
    #[cfg(not(feature = "small"))]
    {
        debug_assert!(q < d2s::DOUBLE_POW5_INV_SPLIT.len() as u32);
        unsafe {
            mul_shift_32(
                m,
                d2s::DOUBLE_POW5_INV_SPLIT.get_unchecked(q as usize).1 + 1,
                j,
            )
        }
    }
}
#[cfg_attr(feature = "no-panic", inline)]
pub fn mul_pow5_div_pow2(m: u32, i: u32, j: i32) -> u32 {
    #[cfg(feature = "small")]
    {
        let pow5 = unsafe { d2s::compute_pow5(i) };
        mul_shift_32(m, pow5.1, j)
    }
    #[cfg(not(feature = "small"))]
    {
        debug_assert!(i < d2s::DOUBLE_POW5_SPLIT.len() as u32);
        unsafe { mul_shift_32(m, d2s::DOUBLE_POW5_SPLIT.get_unchecked(i as usize).1, j) }
    }
}
#[cfg(test)]
mod tests_rug_18 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_18_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 12345;
        let mut p0: u32 = rug_fuzz_0;
        crate::f2s_intrinsics::pow5factor_32(p0);
        let _rug_ed_tests_rug_18_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_19 {
    use super::*;
    use crate::f2s_intrinsics::pow5factor_32;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_19_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 12345;
        let rug_fuzz_1 = 3;
        let value: u32 = rug_fuzz_0;
        let p: u32 = rug_fuzz_1;
        debug_assert_eq!(multiple_of_power_of_5_32(value, p), pow5factor_32(value) >= p);
        let _rug_ed_tests_rug_19_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_20 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_20_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 3;
        let mut p0: u32 = rug_fuzz_0;
        let mut p1: u32 = rug_fuzz_1;
        crate::f2s_intrinsics::multiple_of_power_of_2_32(p0, p1);
        let _rug_ed_tests_rug_20_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_21 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_21_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1234;
        let rug_fuzz_1 = 5678;
        let rug_fuzz_2 = 16;
        let mut p0: u32 = rug_fuzz_0;
        let mut p1: u64 = rug_fuzz_1;
        let mut p2: i32 = -rug_fuzz_2;
        crate::f2s_intrinsics::mul_shift_32(p0, p1, p2);
        let _rug_ed_tests_rug_21_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_22 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_22_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123;
        let rug_fuzz_1 = 456;
        let rug_fuzz_2 = 789;
        let mut p0: u32 = rug_fuzz_0;
        let mut p1: u32 = rug_fuzz_1;
        let mut p2: i32 = -rug_fuzz_2;
        crate::f2s_intrinsics::mul_pow5_inv_div_pow2(p0, p1, p2);
        let _rug_ed_tests_rug_22_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_23 {
    use super::*;
    use crate::f2s_intrinsics::mul_pow5_div_pow2;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_23_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123;
        let rug_fuzz_1 = 456;
        let rug_fuzz_2 = 789;
        let mut p0: u32 = rug_fuzz_0;
        let mut p1: u32 = rug_fuzz_1;
        let mut p2: i32 = -rug_fuzz_2;
        mul_pow5_div_pow2(p0, p1, p2);
        let _rug_ed_tests_rug_23_rrrruuuugggg_test_rug = 0;
    }
}
