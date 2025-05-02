#![allow(clippy::needless_range_loop)]
use crate::table::TABLE;
use core::{convert::TryInto, u64};
pub(crate) const COLS: usize = 16;
const ROUNDS: u64 = 14;
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
        column(&x, [1, 3, 5, 11, 0, 2, 4, 6]),
        column(&x, [2, 4, 6, 12, 1, 3, 5, 7]),
        column(&x, [3, 5, 7, 13, 2, 4, 6, 8]),
        column(&x, [4, 6, 8, 14, 3, 5, 7, 9]),
        column(&x, [5, 7, 9, 15, 4, 6, 8, 10]),
        column(&x, [6, 8, 10, 0, 5, 7, 9, 11]),
        column(&x, [7, 9, 11, 1, 6, 8, 10, 12]),
        column(&x, [8, 10, 12, 2, 7, 9, 11, 13]),
        column(&x, [9, 11, 13, 3, 8, 10, 12, 14]),
        column(&x, [10, 12, 14, 4, 9, 11, 13, 15]),
        column(&x, [11, 13, 15, 5, 10, 12, 14, 0]),
        column(&x, [12, 14, 0, 6, 11, 13, 15, 1]),
        column(&x, [13, 15, 1, 7, 12, 14, 0, 2]),
        column(&x, [14, 0, 2, 8, 13, 15, 1, 3]),
        column(&x, [15, 1, 3, 9, 14, 0, 2, 4]),
        column(&x, [0, 2, 4, 10, 15, 1, 3, 5]),
    ]
}
#[inline(always)]
fn rndp(mut x: [u64; COLS], r: u64) -> [u64; COLS] {
    for i in 0..COLS {
        x[i] ^= ((i as u64) << 60) ^ r;
    }
    [
        column(&x, [0, 1, 2, 3, 4, 5, 6, 11]),
        column(&x, [1, 2, 3, 4, 5, 6, 7, 12]),
        column(&x, [2, 3, 4, 5, 6, 7, 8, 13]),
        column(&x, [3, 4, 5, 6, 7, 8, 9, 14]),
        column(&x, [4, 5, 6, 7, 8, 9, 10, 15]),
        column(&x, [5, 6, 7, 8, 9, 10, 11, 0]),
        column(&x, [6, 7, 8, 9, 10, 11, 12, 1]),
        column(&x, [7, 8, 9, 10, 11, 12, 13, 2]),
        column(&x, [8, 9, 10, 11, 12, 13, 14, 3]),
        column(&x, [9, 10, 11, 12, 13, 14, 15, 4]),
        column(&x, [10, 11, 12, 13, 14, 15, 0, 5]),
        column(&x, [11, 12, 13, 14, 15, 0, 1, 6]),
        column(&x, [12, 13, 14, 15, 0, 1, 2, 7]),
        column(&x, [13, 14, 15, 0, 1, 2, 3, 8]),
        column(&x, [14, 15, 0, 1, 2, 3, 4, 9]),
        column(&x, [15, 0, 1, 2, 3, 4, 5, 10]),
    ]
}
pub(crate) fn compress(h: &mut [u64; COLS], block: &[u8; 128]) {
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
mod tests_rug_138 {
    use super::*;
    #[test]
    fn test_column() {
        let _rug_st_tests_rug_138_rrrruuuugggg_test_column = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 2;
        let rug_fuzz_4 = 3;
        let rug_fuzz_5 = 4;
        let rug_fuzz_6 = 5;
        let rug_fuzz_7 = 6;
        let rug_fuzz_8 = 7;
        let mut p0: [u64; COLS] = [rug_fuzz_0; COLS];
        let p1: [usize; 8] = [
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
            rug_fuzz_6,
            rug_fuzz_7,
            rug_fuzz_8,
        ];
        crate::compress1024::column(&p0, p1);
        let _rug_ed_tests_rug_138_rrrruuuugggg_test_column = 0;
    }
}
#[cfg(test)]
mod tests_rug_139 {
    use super::*;
    use crate::compress1024::{COLS, column};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_139_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 12345;
        let mut p0: [u64; COLS] = [rug_fuzz_0; COLS];
        let p1: u64 = rug_fuzz_1;
        crate::compress1024::rndq(p0, p1);
        let _rug_ed_tests_rug_139_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_140 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_140_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 123456789;
        let mut p0: [u64; COLS] = [rug_fuzz_0; COLS];
        let p1: u64 = rug_fuzz_1;
        crate::compress1024::rndp(p0, p1);
        let _rug_ed_tests_rug_140_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_141 {
    use super::*;
    #[test]
    fn test_compress() {
        let _rug_st_tests_rug_141_rrrruuuugggg_test_compress = 0;
        let rug_fuzz_0 = 0u64;
        let rug_fuzz_1 = 0u8;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 12345;
        let rug_fuzz_4 = 1;
        let rug_fuzz_5 = 67890;
        let rug_fuzz_6 = 0;
        let rug_fuzz_7 = 1;
        let rug_fuzz_8 = 1;
        let rug_fuzz_9 = 2;
        let mut h = [rug_fuzz_0; COLS];
        let mut block = [rug_fuzz_1; 128];
        h[rug_fuzz_2] = rug_fuzz_3;
        h[rug_fuzz_4] = rug_fuzz_5;
        block[rug_fuzz_6] = rug_fuzz_7;
        block[rug_fuzz_8] = rug_fuzz_9;
        crate::compress1024::compress(&mut h, &block);
        let _rug_ed_tests_rug_141_rrrruuuugggg_test_compress = 0;
    }
}
#[cfg(test)]
mod tests_rug_142 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_142_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0x0123456789abcdef;
        let rug_fuzz_1 = 0x0123456789abcdef;
        let rug_fuzz_2 = 0x0123456789abcdef;
        let rug_fuzz_3 = 0x0123456789abcdef;
        let rug_fuzz_4 = 0x0123456789abcdef;
        let rug_fuzz_5 = 0x0123456789abcdef;
        let rug_fuzz_6 = 0x0123456789abcdef;
        let rug_fuzz_7 = 0x0123456789abcdef;
        let rug_fuzz_8 = 0x0123456789abcdef;
        let rug_fuzz_9 = 0x0123456789abcdef;
        let rug_fuzz_10 = 0x0123456789abcdef;
        let rug_fuzz_11 = 0x0123456789abcdef;
        let rug_fuzz_12 = 0x0123456789abcdef;
        let rug_fuzz_13 = 0x0123456789abcdef;
        let rug_fuzz_14 = 0x0123456789abcdef;
        let rug_fuzz_15 = 0x0123456789abcdef;
        let mut p0: [u64; COLS] = [
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
            rug_fuzz_6,
            rug_fuzz_7,
            rug_fuzz_8,
            rug_fuzz_9,
            rug_fuzz_10,
            rug_fuzz_11,
            rug_fuzz_12,
            rug_fuzz_13,
            rug_fuzz_14,
            rug_fuzz_15,
        ];
        crate::compress1024::p(&p0);
        let _rug_ed_tests_rug_142_rrrruuuugggg_test_rug = 0;
    }
}
