#![allow(clippy::many_single_char_names)]
use crate::consts::BLOCK_LEN;
use core::convert::TryInto;
#[inline(always)]
fn shl(v: [u32; 4], o: u32) -> [u32; 4] {
    [v[0] >> o, v[1] >> o, v[2] >> o, v[3] >> o]
}
#[inline(always)]
fn shr(v: [u32; 4], o: u32) -> [u32; 4] {
    [v[0] << o, v[1] << o, v[2] << o, v[3] << o]
}
#[inline(always)]
fn or(a: [u32; 4], b: [u32; 4]) -> [u32; 4] {
    [a[0] | b[0], a[1] | b[1], a[2] | b[2], a[3] | b[3]]
}
#[inline(always)]
fn xor(a: [u32; 4], b: [u32; 4]) -> [u32; 4] {
    [a[0] ^ b[0], a[1] ^ b[1], a[2] ^ b[2], a[3] ^ b[3]]
}
#[inline(always)]
fn add(a: [u32; 4], b: [u32; 4]) -> [u32; 4] {
    [
        a[0].wrapping_add(b[0]),
        a[1].wrapping_add(b[1]),
        a[2].wrapping_add(b[2]),
        a[3].wrapping_add(b[3]),
    ]
}
fn sha256load(v2: [u32; 4], v3: [u32; 4]) -> [u32; 4] {
    [v3[3], v2[0], v2[1], v2[2]]
}
fn sha256swap(v0: [u32; 4]) -> [u32; 4] {
    [v0[2], v0[3], v0[0], v0[1]]
}
fn sha256msg1(v0: [u32; 4], v1: [u32; 4]) -> [u32; 4] {
    #[inline]
    fn sigma0x4(x: [u32; 4]) -> [u32; 4] {
        let t1 = or(shl(x, 7), shr(x, 25));
        let t2 = or(shl(x, 18), shr(x, 14));
        let t3 = shl(x, 3);
        xor(xor(t1, t2), t3)
    }
    add(v0, sigma0x4(sha256load(v0, v1)))
}
fn sha256msg2(v4: [u32; 4], v3: [u32; 4]) -> [u32; 4] {
    macro_rules! sigma1 {
        ($a:expr) => {
            $a .rotate_right(17) ^ $a .rotate_right(19) ^ ($a >> 10)
        };
    }
    let [x3, x2, x1, x0] = v4;
    let [w15, w14, _, _] = v3;
    let w16 = x0.wrapping_add(sigma1!(w14));
    let w17 = x1.wrapping_add(sigma1!(w15));
    let w18 = x2.wrapping_add(sigma1!(w16));
    let w19 = x3.wrapping_add(sigma1!(w17));
    [w19, w18, w17, w16]
}
fn sha256_digest_round_x2(cdgh: [u32; 4], abef: [u32; 4], wk: [u32; 4]) -> [u32; 4] {
    macro_rules! big_sigma0 {
        ($a:expr) => {
            ($a .rotate_right(2) ^ $a .rotate_right(13) ^ $a .rotate_right(22))
        };
    }
    macro_rules! big_sigma1 {
        ($a:expr) => {
            ($a .rotate_right(6) ^ $a .rotate_right(11) ^ $a .rotate_right(25))
        };
    }
    macro_rules! bool3ary_202 {
        ($a:expr, $b:expr, $c:expr) => {
            $c ^ ($a & ($b ^ $c))
        };
    }
    macro_rules! bool3ary_232 {
        ($a:expr, $b:expr, $c:expr) => {
            ($a & $b) ^ ($a & $c) ^ ($b & $c)
        };
    }
    let [_, _, wk1, wk0] = wk;
    let [a0, b0, e0, f0] = abef;
    let [c0, d0, g0, h0] = cdgh;
    let x0 = big_sigma1!(e0)
        .wrapping_add(bool3ary_202!(e0, f0, g0))
        .wrapping_add(wk0)
        .wrapping_add(h0);
    let y0 = big_sigma0!(a0).wrapping_add(bool3ary_232!(a0, b0, c0));
    let (a1, b1, c1, d1, e1, f1, g1, h1) = (
        x0.wrapping_add(y0),
        a0,
        b0,
        c0,
        x0.wrapping_add(d0),
        e0,
        f0,
        g0,
    );
    let x1 = big_sigma1!(e1)
        .wrapping_add(bool3ary_202!(e1, f1, g1))
        .wrapping_add(wk1)
        .wrapping_add(h1);
    let y1 = big_sigma0!(a1).wrapping_add(bool3ary_232!(a1, b1, c1));
    let (a2, b2, _, _, e2, f2, _, _) = (
        x1.wrapping_add(y1),
        a1,
        b1,
        c1,
        x1.wrapping_add(d1),
        e1,
        f1,
        g1,
    );
    [a2, b2, e2, f2]
}
fn schedule(v0: [u32; 4], v1: [u32; 4], v2: [u32; 4], v3: [u32; 4]) -> [u32; 4] {
    let t1 = sha256msg1(v0, v1);
    let t2 = sha256load(v2, v3);
    let t3 = add(t1, t2);
    sha256msg2(t3, v3)
}
macro_rules! rounds4 {
    ($abef:ident, $cdgh:ident, $rest:expr, $i:expr) => {
        { let t1 = add($rest, crate ::consts::K32X4[$i]); $cdgh =
        sha256_digest_round_x2($cdgh, $abef, t1); let t2 = sha256swap(t1); $abef =
        sha256_digest_round_x2($abef, $cdgh, t2); }
    };
}
macro_rules! schedule_rounds4 {
    (
        $abef:ident, $cdgh:ident, $w0:expr, $w1:expr, $w2:expr, $w3:expr, $w4:expr,
        $i:expr
    ) => {
        { $w4 = schedule($w0, $w1, $w2, $w3); rounds4!($abef, $cdgh, $w4, $i); }
    };
}
/// Process a block with the SHA-256 algorithm.
fn sha256_digest_block_u32(state: &mut [u32; 8], block: &[u32; 16]) {
    let mut abef = [state[0], state[1], state[4], state[5]];
    let mut cdgh = [state[2], state[3], state[6], state[7]];
    let mut w0 = [block[3], block[2], block[1], block[0]];
    let mut w1 = [block[7], block[6], block[5], block[4]];
    let mut w2 = [block[11], block[10], block[9], block[8]];
    let mut w3 = [block[15], block[14], block[13], block[12]];
    let mut w4;
    rounds4!(abef, cdgh, w0, 0);
    rounds4!(abef, cdgh, w1, 1);
    rounds4!(abef, cdgh, w2, 2);
    rounds4!(abef, cdgh, w3, 3);
    schedule_rounds4!(abef, cdgh, w0, w1, w2, w3, w4, 4);
    schedule_rounds4!(abef, cdgh, w1, w2, w3, w4, w0, 5);
    schedule_rounds4!(abef, cdgh, w2, w3, w4, w0, w1, 6);
    schedule_rounds4!(abef, cdgh, w3, w4, w0, w1, w2, 7);
    schedule_rounds4!(abef, cdgh, w4, w0, w1, w2, w3, 8);
    schedule_rounds4!(abef, cdgh, w0, w1, w2, w3, w4, 9);
    schedule_rounds4!(abef, cdgh, w1, w2, w3, w4, w0, 10);
    schedule_rounds4!(abef, cdgh, w2, w3, w4, w0, w1, 11);
    schedule_rounds4!(abef, cdgh, w3, w4, w0, w1, w2, 12);
    schedule_rounds4!(abef, cdgh, w4, w0, w1, w2, w3, 13);
    schedule_rounds4!(abef, cdgh, w0, w1, w2, w3, w4, 14);
    schedule_rounds4!(abef, cdgh, w1, w2, w3, w4, w0, 15);
    let [a, b, e, f] = abef;
    let [c, d, g, h] = cdgh;
    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
    state[4] = state[4].wrapping_add(e);
    state[5] = state[5].wrapping_add(f);
    state[6] = state[6].wrapping_add(g);
    state[7] = state[7].wrapping_add(h);
}
pub fn compress(state: &mut [u32; 8], blocks: &[[u8; 64]]) {
    let mut block_u32 = [0u32; BLOCK_LEN];
    let mut state_cpy = *state;
    for block in blocks {
        for (o, chunk) in block_u32.iter_mut().zip(block.chunks_exact(4)) {
            *o = u32::from_be_bytes(chunk.try_into().unwrap());
        }
        sha256_digest_block_u32(&mut state_cpy, &block_u32);
    }
    *state = state_cpy;
}
#[cfg(test)]
mod tests_rug_232 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_232_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 4;
        let rug_fuzz_4 = 2;
        let mut p0: [u32; 4] = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3];
        let mut p1: u32 = rug_fuzz_4;
        crate::sha256::soft::shl(p0, p1);
        let _rug_ed_tests_rug_232_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_233 {
    use super::*;
    use crate::sha256::soft::shr;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_233_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 4;
        let rug_fuzz_4 = 10;
        let v: [u32; 4] = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3];
        let o: u32 = rug_fuzz_4;
        shr(v, o);
        let _rug_ed_tests_rug_233_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_234 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_234_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0x11111111;
        let rug_fuzz_1 = 0x22222222;
        let rug_fuzz_2 = 0x33333333;
        let rug_fuzz_3 = 0x44444444;
        let rug_fuzz_4 = 0x55555555;
        let rug_fuzz_5 = 0x66666666;
        let rug_fuzz_6 = 0x77777777;
        let rug_fuzz_7 = 0x88888888;
        let rug_fuzz_8 = 0;
        let rug_fuzz_9 = 1;
        let rug_fuzz_10 = 2;
        let rug_fuzz_11 = 3;
        let p0: [u32; 4] = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3];
        let p1: [u32; 4] = [rug_fuzz_4, rug_fuzz_5, rug_fuzz_6, rug_fuzz_7];
        let result = crate::sha256::soft::or(p0, p1);
        debug_assert_eq!(result[rug_fuzz_8], 0x55555555 | 0x11111111);
        debug_assert_eq!(result[rug_fuzz_9], 0x66666666 | 0x22222222);
        debug_assert_eq!(result[rug_fuzz_10], 0x77777777 | 0x33333333);
        debug_assert_eq!(result[rug_fuzz_11], 0x88888888 | 0x44444444);
        let _rug_ed_tests_rug_234_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_235 {
    use super::*;
    use crate::sha256::soft::xor;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_235_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0xDEADBEEF;
        let rug_fuzz_1 = 0xBAADF00D;
        let rug_fuzz_2 = 0xC0FFEE;
        let rug_fuzz_3 = 0xFEEDBEEF;
        let rug_fuzz_4 = 0xCAFEBABE;
        let rug_fuzz_5 = 0xDEADBEEF;
        let rug_fuzz_6 = 0xBAADF00D;
        let rug_fuzz_7 = 0xC0FFEE;
        let mut p0: [u32; 4] = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3];
        let mut p1: [u32; 4] = [rug_fuzz_4, rug_fuzz_5, rug_fuzz_6, rug_fuzz_7];
        xor(p0, p1);
        let _rug_ed_tests_rug_235_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_236 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_236_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 3;
        let rug_fuzz_4 = 4;
        let rug_fuzz_5 = 5;
        let rug_fuzz_6 = 6;
        let rug_fuzz_7 = 7;
        let mut p0: [u32; 4] = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3];
        let mut p1: [u32; 4] = [rug_fuzz_4, rug_fuzz_5, rug_fuzz_6, rug_fuzz_7];
        crate::sha256::soft::add(p0, p1);
        let _rug_ed_tests_rug_236_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_237 {
    use super::*;
    #[test]
    fn test_sha256load() {
        let _rug_st_tests_rug_237_rrrruuuugggg_test_sha256load = 0;
        let rug_fuzz_0 = 0x6a09e667;
        let rug_fuzz_1 = 0xbb67ae85;
        let rug_fuzz_2 = 0x3c6ef372;
        let rug_fuzz_3 = 0xa54ff53a;
        let rug_fuzz_4 = 0x510e527f;
        let rug_fuzz_5 = 0x9b05688c;
        let rug_fuzz_6 = 0x1f83d9ab;
        let rug_fuzz_7 = 0x5be0cd19;
        let mut p0: [u32; 4] = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3];
        let mut p1: [u32; 4] = [rug_fuzz_4, rug_fuzz_5, rug_fuzz_6, rug_fuzz_7];
        debug_assert_eq!(
            crate ::sha256::soft::sha256load(p0, p1), [p1[3], p0[0], p0[1], p0[2]]
        );
        let _rug_ed_tests_rug_237_rrrruuuugggg_test_sha256load = 0;
    }
}
#[cfg(test)]
mod tests_rug_238 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_238_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0x6a09e667;
        let rug_fuzz_1 = 0xbb67ae85;
        let rug_fuzz_2 = 0x3c6ef372;
        let rug_fuzz_3 = 0xa54ff53a;
        let p0: [u32; 4] = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3];
        crate::sha256::soft::sha256swap(p0);
        let _rug_ed_tests_rug_238_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_239 {
    use super::*;
    use crate::sha256::soft::{add, sha256load, shl, shr, or, xor};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_239_rrrruuuugggg_test_rug = 0;
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
        let mut p0: [u32; 4] = [rug_fuzz_0; 4];
        let mut p1: [u32; 4] = [rug_fuzz_1; 4];
        p0 = [rug_fuzz_2, rug_fuzz_3, rug_fuzz_4, rug_fuzz_5];
        p1 = [rug_fuzz_6, rug_fuzz_7, rug_fuzz_8, rug_fuzz_9];
        crate::sha256::soft::sha256msg1(p0, p1);
        let _rug_ed_tests_rug_239_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_241 {
    use super::*;
    use crate::sha256::soft::sha256msg2;
    #[test]
    fn test_sha256msg2() {
        let _rug_st_tests_rug_241_rrrruuuugggg_test_sha256msg2 = 0;
        let rug_fuzz_0 = 0x6a09e667;
        let rug_fuzz_1 = 0xbb67ae85;
        let rug_fuzz_2 = 0x3c6ef372;
        let rug_fuzz_3 = 0xa54ff53a;
        let rug_fuzz_4 = 0x510e527f;
        let rug_fuzz_5 = 0x9b05688c;
        let rug_fuzz_6 = 0x1f83d9ab;
        let rug_fuzz_7 = 0x5be0cd19;
        let rug_fuzz_8 = 0;
        let rug_fuzz_9 = 1;
        let rug_fuzz_10 = 2;
        let rug_fuzz_11 = 3;
        let v4: [u32; 4] = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3];
        let v3: [u32; 4] = [rug_fuzz_4, rug_fuzz_5, rug_fuzz_6, rug_fuzz_7];
        let result = sha256msg2(v4, v3);
        debug_assert_eq!(result[rug_fuzz_8], 0xdff5b10b);
        debug_assert_eq!(result[rug_fuzz_9], 0xf648bd4e);
        debug_assert_eq!(result[rug_fuzz_10], 0xf47c5aca);
        debug_assert_eq!(result[rug_fuzz_11], 0x1abba8db);
        let _rug_ed_tests_rug_241_rrrruuuugggg_test_sha256msg2 = 0;
    }
}
#[cfg(test)]
mod tests_rug_242 {
    use super::*;
    #[test]
    fn test_sha256_digest_round_x2() {
        let _rug_st_tests_rug_242_rrrruuuugggg_test_sha256_digest_round_x2 = 0;
        let rug_fuzz_0 = 0x6a09e667;
        let rug_fuzz_1 = 0xbb67ae85;
        let rug_fuzz_2 = 0x3c6ef372;
        let rug_fuzz_3 = 0xa54ff53a;
        let rug_fuzz_4 = 0x2a907bbb;
        let rug_fuzz_5 = 0xeb55a8e8;
        let rug_fuzz_6 = 0x14f8385a;
        let rug_fuzz_7 = 0x08de7719;
        let rug_fuzz_8 = 0xaaffffff;
        let rug_fuzz_9 = 0x3399baaa;
        let rug_fuzz_10 = 0xffeeddcc;
        let rug_fuzz_11 = 0x11223344;
        let mut p0: [u32; 4] = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3];
        let mut p1: [u32; 4] = [rug_fuzz_4, rug_fuzz_5, rug_fuzz_6, rug_fuzz_7];
        let mut p2: [u32; 4] = [rug_fuzz_8, rug_fuzz_9, rug_fuzz_10, rug_fuzz_11];
        sha256_digest_round_x2(p0, p1, p2);
        let _rug_ed_tests_rug_242_rrrruuuugggg_test_sha256_digest_round_x2 = 0;
    }
}
#[cfg(test)]
mod tests_rug_243 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_243_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let mut p0: [u32; 4] = [rug_fuzz_0; 4];
        let mut p1: [u32; 4] = [rug_fuzz_1; 4];
        let mut p2: [u32; 4] = [rug_fuzz_2; 4];
        let mut p3: [u32; 4] = [rug_fuzz_3; 4];
        crate::sha256::soft::schedule(p0, p1, p2, p3);
        let _rug_ed_tests_rug_243_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_244 {
    use super::*;
    #[test]
    fn test_sha256_digest_block_u32() {
        let _rug_st_tests_rug_244_rrrruuuugggg_test_sha256_digest_block_u32 = 0;
        let rug_fuzz_0 = 0x6a09e667;
        let rug_fuzz_1 = 0xbb67ae85;
        let rug_fuzz_2 = 0x3c6ef372;
        let rug_fuzz_3 = 0xa54ff53a;
        let rug_fuzz_4 = 0x510e527f;
        let rug_fuzz_5 = 0x9b05688c;
        let rug_fuzz_6 = 0x1f83d9ab;
        let rug_fuzz_7 = 0x5be0cd19;
        let rug_fuzz_8 = 0xf3bcc908;
        let rug_fuzz_9 = 0x6c7eae46;
        let rug_fuzz_10 = 0xb44ca70a;
        let rug_fuzz_11 = 0xb8ff64f8;
        let rug_fuzz_12 = 0xfad03212;
        let rug_fuzz_13 = 0xb0452d2e;
        let rug_fuzz_14 = 0xd5b1be16;
        let rug_fuzz_15 = 0xf7ee1122;
        let rug_fuzz_16 = 0xcce24c92;
        let rug_fuzz_17 = 0x7fc780cf;
        let rug_fuzz_18 = 0xa480f74d;
        let rug_fuzz_19 = 0xc4547c6b;
        let rug_fuzz_20 = 0xe73bf036;
        let rug_fuzz_21 = 0xc83179b1;
        let rug_fuzz_22 = 0xa50093d4;
        let rug_fuzz_23 = 0xd8a12964;
        let mut p0: [u32; 8] = [
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
            rug_fuzz_6,
            rug_fuzz_7,
        ];
        let mut p1: [u32; 16] = [
            rug_fuzz_8,
            rug_fuzz_9,
            rug_fuzz_10,
            rug_fuzz_11,
            rug_fuzz_12,
            rug_fuzz_13,
            rug_fuzz_14,
            rug_fuzz_15,
            rug_fuzz_16,
            rug_fuzz_17,
            rug_fuzz_18,
            rug_fuzz_19,
            rug_fuzz_20,
            rug_fuzz_21,
            rug_fuzz_22,
            rug_fuzz_23,
        ];
        sha256_digest_block_u32(&mut p0, &p1);
        let _rug_ed_tests_rug_244_rrrruuuugggg_test_sha256_digest_block_u32 = 0;
    }
}
#[cfg(test)]
mod tests_rug_245 {
    use super::*;
    use crate::Digest;
    #[test]
    fn test_compress() {
        let _rug_st_tests_rug_245_rrrruuuugggg_test_compress = 0;
        let rug_fuzz_0 = 0u32;
        let rug_fuzz_1 = 0u8;
        let rug_fuzz_2 = 0u8;
        let mut state = [rug_fuzz_0; 8];
        let blocks = [[rug_fuzz_1; 64], [rug_fuzz_2; 64]];
        compress(&mut state, &blocks);
        let _rug_ed_tests_rug_245_rrrruuuugggg_test_compress = 0;
    }
}
