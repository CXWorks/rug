use core::ptr;
#[cfg_attr(feature = "no-panic", inline)]
pub fn div5(x: u64) -> u64 {
    x / 5
}
#[cfg_attr(feature = "no-panic", inline)]
pub fn div10(x: u64) -> u64 {
    x / 10
}
#[cfg_attr(feature = "no-panic", inline)]
pub fn div100(x: u64) -> u64 {
    x / 100
}
#[cfg_attr(feature = "no-panic", inline)]
fn pow5_factor(mut value: u64) -> u32 {
    let mut count = 0u32;
    loop {
        debug_assert!(value != 0);
        let q = div5(value);
        let r = (value as u32).wrapping_sub(5u32.wrapping_mul(q as u32));
        if r != 0 {
            break;
        }
        value = q;
        count += 1;
    }
    count
}
#[cfg_attr(feature = "no-panic", inline)]
pub fn multiple_of_power_of_5(value: u64, p: u32) -> bool {
    pow5_factor(value) >= p
}
#[cfg_attr(feature = "no-panic", inline)]
pub fn multiple_of_power_of_2(value: u64, p: u32) -> bool {
    debug_assert!(value != 0);
    debug_assert!(p < 64);
    (value & ((1u64 << p) - 1)) == 0
}
#[cfg_attr(feature = "no-panic", inline)]
pub fn mul_shift_64(m: u64, mul: &(u64, u64), j: u32) -> u64 {
    let b0 = m as u128 * mul.0 as u128;
    let b2 = m as u128 * mul.1 as u128;
    (((b0 >> 64) + b2) >> (j - 64)) as u64
}
#[cfg_attr(feature = "no-panic", inline)]
pub unsafe fn mul_shift_all_64(
    m: u64,
    mul: &(u64, u64),
    j: u32,
    vp: *mut u64,
    vm: *mut u64,
    mm_shift: u32,
) -> u64 {
    ptr::write(vp, mul_shift_64(4 * m + 2, mul, j));
    ptr::write(vm, mul_shift_64(4 * m - 1 - mm_shift as u64, mul, j));
    mul_shift_64(4 * m, mul, j)
}
#[cfg(test)]
mod tests_rug_9 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_9_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 15;
        let mut p0: u64 = rug_fuzz_0;
        crate::d2s_intrinsics::div5(p0);
        let _rug_ed_tests_rug_9_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_10 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_10_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u64 = rug_fuzz_0;
        crate::d2s_intrinsics::div10(p0);
        let _rug_ed_tests_rug_10_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_11 {
    use super::*;
    use crate::d2s_intrinsics::div100;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_11_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 5000;
        let p0: u64 = rug_fuzz_0;
        div100(p0);
        let _rug_ed_tests_rug_11_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_12 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_12_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123456789;
        let mut value: u64 = rug_fuzz_0;
        crate::d2s_intrinsics::pow5_factor(value);
        let _rug_ed_tests_rug_12_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_13 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_13_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1234567890;
        let rug_fuzz_1 = 5;
        let mut p0: u64 = rug_fuzz_0;
        let mut p1: u32 = rug_fuzz_1;
        crate::d2s_intrinsics::multiple_of_power_of_5(p0, p1);
        let _rug_ed_tests_rug_13_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_14 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_14_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1234;
        let rug_fuzz_1 = 5;
        let value: u64 = rug_fuzz_0;
        let p: u32 = rug_fuzz_1;
        crate::d2s_intrinsics::multiple_of_power_of_2(value, p);
        let _rug_ed_tests_rug_14_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_15 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_15_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123456789;
        let rug_fuzz_1 = 987654321;
        let rug_fuzz_2 = 987654321;
        let rug_fuzz_3 = 16;
        let p0: u64 = rug_fuzz_0;
        let p1: (u64, u64) = (rug_fuzz_1, rug_fuzz_2);
        let p2: u32 = rug_fuzz_3;
        crate::d2s_intrinsics::mul_shift_64(p0, &p1, p2);
        let _rug_ed_tests_rug_15_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_16 {
    use super::*;
    use crate::d2s_intrinsics::{mul_shift_64, mul_shift_all_64};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_16_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 5;
        let rug_fuzz_4 = 3;
        let m: u64 = rug_fuzz_0;
        let mul: (u64, u64) = (rug_fuzz_1, rug_fuzz_2);
        let j: u32 = rug_fuzz_3;
        let vp: *mut u64 = std::ptr::null_mut();
        let vm: *mut u64 = std::ptr::null_mut();
        let mm_shift: u32 = rug_fuzz_4;
        unsafe {
            mul_shift_all_64(m, &mul, j, vp, vm, mm_shift);
        }
        let _rug_ed_tests_rug_16_rrrruuuugggg_test_rug = 0;
    }
}
