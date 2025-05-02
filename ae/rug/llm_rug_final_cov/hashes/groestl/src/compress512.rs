#![allow(clippy::needless_range_loop)]
use crate::table::TABLE;
use core::{convert::TryInto, u64};
pub(crate) const COLS: usize = 8;
const ROUNDS: u64 = 10;
#[inline(always)]
fn column(x: &[u64; COLS], c: [usize; 8]) -> u64 {
    let mut t = 0;
    for i in 0..8 {
        let sl = 8 * (7 - i);
        let idx = ((x[c[i]] >> sl) & 0xFF) as usize;
        t ^= TABLE[i][idx];
    }
    t
}
#[inline(always)]
fn rndq(mut x: [u64; COLS], r: u64) -> [u64; COLS] {
    for i in 0..COLS {
        x[i] ^= u64::MAX.wrapping_sub((i as u64) << 4) ^ r;
    }
    [
        column(&x, [1, 3, 5, 7, 0, 2, 4, 6]),
        column(&x, [2, 4, 6, 0, 1, 3, 5, 7]),
        column(&x, [3, 5, 7, 1, 2, 4, 6, 0]),
        column(&x, [4, 6, 0, 2, 3, 5, 7, 1]),
        column(&x, [5, 7, 1, 3, 4, 6, 0, 2]),
        column(&x, [6, 0, 2, 4, 5, 7, 1, 3]),
        column(&x, [7, 1, 3, 5, 6, 0, 2, 4]),
        column(&x, [0, 2, 4, 6, 7, 1, 3, 5]),
    ]
}
#[inline(always)]
fn rndp(mut x: [u64; COLS], r: u64) -> [u64; COLS] {
    for i in 0..COLS {
        x[i] ^= ((i as u64) << 60) ^ r;
    }
    [
        column(&x, [0, 1, 2, 3, 4, 5, 6, 7]),
        column(&x, [1, 2, 3, 4, 5, 6, 7, 0]),
        column(&x, [2, 3, 4, 5, 6, 7, 0, 1]),
        column(&x, [3, 4, 5, 6, 7, 0, 1, 2]),
        column(&x, [4, 5, 6, 7, 0, 1, 2, 3]),
        column(&x, [5, 6, 7, 0, 1, 2, 3, 4]),
        column(&x, [6, 7, 0, 1, 2, 3, 4, 5]),
        column(&x, [7, 0, 1, 2, 3, 4, 5, 6]),
    ]
}
pub(crate) fn compress(h: &mut [u64; COLS], block: &[u8; 64]) {
    let mut q = [0u64; COLS];
    for (chunk, v) in block.chunks_exact(8).zip(q.iter_mut()) {
        *v = u64::from_be_bytes(chunk.try_into().unwrap());
    }
    let mut p = [0u64; COLS];
    for i in 0..COLS {
        p[i] = h[i] ^ q[i];
    }
    for i in 0..ROUNDS {
        q = rndq(q, i);
    }
    for i in 0..ROUNDS {
        p = rndp(p, i << 56);
    }
    for i in 0..COLS {
        h[i] ^= q[i] ^ p[i];
    }
}
pub(crate) fn p(h: &[u64; COLS]) -> [u64; COLS] {
    let mut p = *h;
    for i in 0..ROUNDS {
        p = rndp(p, i << 56);
    }
    for i in 0..COLS {
        p[i] ^= h[i];
    }
    p
}
#[cfg(test)]
mod tests_rug_143 {
    use super::*;
    use crate::compress512::COLS;
    use crate::compress512::TABLE;
    #[test]
    fn test_column() {
        let _rug_st_tests_rug_143_rrrruuuugggg_test_column = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let mut p0: [u64; COLS] = [rug_fuzz_0; COLS];
        let mut p1: [usize; 8] = [rug_fuzz_1; 8];
        column(&p0, p1);
        let _rug_ed_tests_rug_143_rrrruuuugggg_test_column = 0;
    }
}
#[cfg(test)]
mod tests_rug_144 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_144_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 2;
        let rug_fuzz_4 = 3;
        let rug_fuzz_5 = 4;
        let rug_fuzz_6 = 5;
        let rug_fuzz_7 = 6;
        let rug_fuzz_8 = 7;
        let rug_fuzz_9 = 8;
        let rug_fuzz_10 = 42;
        let mut p0: [u64; 8] = [rug_fuzz_0; 8];
        let mut p1: u64 = rug_fuzz_1;
        let p0_sample: [u64; 8] = [
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
            rug_fuzz_6,
            rug_fuzz_7,
            rug_fuzz_8,
            rug_fuzz_9,
        ];
        p0.copy_from_slice(&p0_sample);
        let p1_sample: u64 = rug_fuzz_10;
        crate::compress512::rndq(p0, p1);
        let _rug_ed_tests_rug_144_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_145 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_145_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0u64;
        let rug_fuzz_1 = 0u64;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 123;
        let rug_fuzz_4 = 1;
        let rug_fuzz_5 = 456;
        let rug_fuzz_6 = 789;
        let mut p0 = [rug_fuzz_0; COLS];
        let mut p1 = rug_fuzz_1;
        p0[rug_fuzz_2] = rug_fuzz_3;
        p0[rug_fuzz_4] = rug_fuzz_5;
        p1 = rug_fuzz_6;
        crate::compress512::rndp(p0, p1);
        let _rug_ed_tests_rug_145_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_146 {
    use super::*;
    use crate::compress512::{compress, COLS, ROUNDS, rndq, rndp};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_146_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0u64;
        let rug_fuzz_1 = 0u8;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0x0123456789ABCDEF;
        let rug_fuzz_4 = 1;
        let rug_fuzz_5 = 0xFEDCBA9876543210;
        let rug_fuzz_6 = 0;
        let rug_fuzz_7 = 0x01;
        let rug_fuzz_8 = 1;
        let rug_fuzz_9 = 0x23;
        let mut h = [rug_fuzz_0; COLS];
        let mut block = [rug_fuzz_1; 64];
        h[rug_fuzz_2] = rug_fuzz_3;
        h[rug_fuzz_4] = rug_fuzz_5;
        block[rug_fuzz_6] = rug_fuzz_7;
        block[rug_fuzz_8] = rug_fuzz_9;
        compress(&mut h, &block);
        let _rug_ed_tests_rug_146_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_147 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_147_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0x0123456789ABCDEF;
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = 0xFEDCBA9876543210;
        let rug_fuzz_5 = 2;
        let rug_fuzz_6 = 0x13579BDF2468ACE0;
        let rug_fuzz_7 = 3;
        let rug_fuzz_8 = 0xECA86420BDF13579;
        let mut p0: [u64; COLS] = [rug_fuzz_0; COLS];
        p0[rug_fuzz_1] = rug_fuzz_2;
        p0[rug_fuzz_3] = rug_fuzz_4;
        p0[rug_fuzz_5] = rug_fuzz_6;
        p0[rug_fuzz_7] = rug_fuzz_8;
        crate::compress512::p(&p0);
        let _rug_ed_tests_rug_147_rrrruuuugggg_test_rug = 0;
    }
}
