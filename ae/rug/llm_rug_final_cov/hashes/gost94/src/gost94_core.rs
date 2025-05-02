#![allow(clippy::many_single_char_names)]
use core::{convert::TryInto, fmt};
use digest::{
    block_buffer::Eager,
    core_api::{
        AlgorithmName, Block as TBlock, BlockSizeUser, Buffer, BufferKindUser,
        FixedOutputCore, OutputSizeUser, Reset, UpdateCore,
    },
    typenum::{Unsigned, U32},
    HashMarker, Output,
};
use crate::params::{Block, Gost94Params, SBox};
const C: Block = [
    0x00,
    0xff,
    0x00,
    0xff,
    0x00,
    0xff,
    0x00,
    0xff,
    0xff,
    0x00,
    0xff,
    0x00,
    0xff,
    0x00,
    0xff,
    0x00,
    0x00,
    0xff,
    0xff,
    0x00,
    0xff,
    0x00,
    0x00,
    0xff,
    0xff,
    0x00,
    0x00,
    0x00,
    0xff,
    0xff,
    0x00,
    0xff,
];
fn sbox(a: u32, s: &SBox) -> u32 {
    let mut v = 0;
    #[allow(clippy::needless_range_loop)]
    for i in 0..8 {
        let shft = 4 * i;
        let k = ((a & (0b1111u32 << shft)) >> shft) as usize;
        v += u32::from(s[i][k]) << shft;
    }
    v
}
fn g(a: u32, k: u32, s: &SBox) -> u32 {
    sbox(a.wrapping_add(k), s).rotate_left(11)
}
#[allow(clippy::needless_range_loop)]
fn encrypt(msg: &mut [u8], key: Block, sbox: &SBox) {
    let mut k = [0u32; 8];
    let mut a = u32::from_le_bytes(msg[0..4].try_into().unwrap());
    let mut b = u32::from_le_bytes(msg[4..8].try_into().unwrap());
    for (o, chunk) in k.iter_mut().zip(key.chunks_exact(4)) {
        *o = u32::from_le_bytes(chunk.try_into().unwrap());
    }
    for _ in 0..3 {
        for i in 0..8 {
            let t = b ^ g(a, k[i], sbox);
            b = a;
            a = t;
        }
    }
    for i in (0..8).rev() {
        let t = b ^ g(a, k[i], sbox);
        b = a;
        a = t;
    }
    msg[0..4].copy_from_slice(&b.to_le_bytes());
    msg[4..8].copy_from_slice(&a.to_le_bytes());
}
fn x(a: &Block, b: &Block) -> Block {
    let mut out = Block::default();
    for i in 0..32 {
        out[i] = a[i] ^ b[i];
    }
    out
}
fn x_mut(a: &mut Block, b: &Block) {
    for i in 0..32 {
        a[i] ^= b[i];
    }
}
fn a(x: Block) -> Block {
    let mut out = Block::default();
    out[..24].clone_from_slice(&x[8..]);
    for i in 0..8 {
        out[24 + i] = x[i] ^ x[i + 8];
    }
    out
}
fn p(y: Block) -> Block {
    let mut out = Block::default();
    for i in 0..4 {
        for k in 0..8 {
            out[i + 4 * k] = y[8 * i + k];
        }
    }
    out
}
fn psi(block: &mut Block) {
    let mut out = Block::default();
    out[..30].copy_from_slice(&block[2..]);
    out[30..].copy_from_slice(&block[..2]);
    out[30] ^= block[2];
    out[31] ^= block[3];
    out[30] ^= block[4];
    out[31] ^= block[5];
    out[30] ^= block[6];
    out[31] ^= block[7];
    out[30] ^= block[24];
    out[31] ^= block[25];
    out[30] ^= block[30];
    out[31] ^= block[31];
    block.copy_from_slice(&out);
}
#[inline(always)]
fn adc(a: &mut u64, b: u64, carry: &mut u64) {
    let ret = (*a as u128) + (b as u128) + (*carry as u128);
    *a = ret as u64;
    *carry = (ret >> 64) as u64;
}
/// Core GOST94 algorithm generic over parameters.
#[derive(Clone)]
pub struct Gost94Core<P: Gost94Params> {
    h: Block,
    n: [u64; 4],
    sigma: [u64; 4],
    _m: core::marker::PhantomData<P>,
}
impl<P: Gost94Params> Gost94Core<P> {
    fn shuffle(&mut self, m: &Block, s: &Block) {
        let mut res = Block::default();
        res.copy_from_slice(s);
        for _ in 0..12 {
            psi(&mut res);
        }
        x_mut(&mut res, m);
        psi(&mut res);
        x_mut(&mut self.h, &res);
        for _ in 0..61 {
            psi(&mut self.h);
        }
    }
    fn f(&mut self, m: &Block) {
        let mut s = Block::default();
        s.copy_from_slice(&self.h);
        let k = p(x(&self.h, m));
        encrypt(&mut s[0..8], k, &P::S_BOX);
        let u = a(self.h);
        let v = a(a(*m));
        let k = p(x(&u, &v));
        encrypt(&mut s[8..16], k, &P::S_BOX);
        let mut u = a(u);
        x_mut(&mut u, &C);
        let v = a(a(v));
        let k = p(x(&u, &v));
        encrypt(&mut s[16..24], k, &P::S_BOX);
        let u = a(u);
        let v = a(a(v));
        let k = p(x(&u, &v));
        encrypt(&mut s[24..32], k, &P::S_BOX);
        self.shuffle(m, &s);
    }
    fn update_sigma(&mut self, m: &Block) {
        let mut carry = 0;
        for (a, chunk) in self.sigma.iter_mut().zip(m.chunks_exact(8)) {
            let b = u64::from_le_bytes(chunk.try_into().unwrap());
            adc(a, b, &mut carry);
        }
    }
    fn update_n(&mut self, len: usize) {
        let mut carry = 0;
        adc(&mut self.n[0], (len as u64) << 3, &mut carry);
        adc(&mut self.n[1], (len as u64) >> 61, &mut carry);
        adc(&mut self.n[2], 0, &mut carry);
        adc(&mut self.n[3], 0, &mut carry);
    }
    #[inline(always)]
    fn compress(&mut self, block: &[u8; 32]) {
        self.f(block);
        self.update_sigma(block);
    }
}
impl<P: Gost94Params> HashMarker for Gost94Core<P> {}
impl<P: Gost94Params> BlockSizeUser for Gost94Core<P> {
    type BlockSize = U32;
}
impl<P: Gost94Params> BufferKindUser for Gost94Core<P> {
    type BufferKind = Eager;
}
impl<P: Gost94Params> OutputSizeUser for Gost94Core<P> {
    type OutputSize = U32;
}
impl<P: Gost94Params> UpdateCore for Gost94Core<P> {
    #[inline]
    fn update_blocks(&mut self, blocks: &[TBlock<Self>]) {
        let len = Self::BlockSize::USIZE * blocks.len();
        self.update_n(len);
        blocks.iter().for_each(|b| self.compress(b.as_ref()));
    }
}
impl<P: Gost94Params> FixedOutputCore for Gost94Core<P> {
    #[inline]
    fn finalize_fixed_core(
        &mut self,
        buffer: &mut Buffer<Self>,
        out: &mut Output<Self>,
    ) {
        if buffer.get_pos() != 0 {
            self.update_n(buffer.get_pos());
            self.compress(buffer.pad_with_zeros().as_ref());
        }
        let mut buf = Block::default();
        for (o, v) in buf.chunks_exact_mut(8).zip(self.n.iter()) {
            o.copy_from_slice(&v.to_le_bytes());
        }
        self.f(&buf);
        for (o, v) in buf.chunks_exact_mut(8).zip(self.sigma.iter()) {
            o.copy_from_slice(&v.to_le_bytes());
        }
        self.f(&buf);
        out.copy_from_slice(&self.h);
    }
}
impl<P: Gost94Params> Default for Gost94Core<P> {
    #[inline]
    fn default() -> Self {
        Self {
            h: P::H0,
            n: Default::default(),
            sigma: Default::default(),
            _m: Default::default(),
        }
    }
}
impl<P: Gost94Params> Reset for Gost94Core<P> {
    #[inline]
    fn reset(&mut self) {
        *self = Default::default();
    }
}
impl<P: Gost94Params> AlgorithmName for Gost94Core<P> {
    fn write_alg_name(f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(P::NAME)
    }
}
impl<P: Gost94Params> fmt::Debug for Gost94Core<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.write_str(P::NAME)?;
        f.write_str("Core { .. }")
    }
}
#[cfg(test)]
mod tests_rug_121 {
    use super::*;
    use crate::gost94_core::{Block, SBox};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_121_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let mut p0: [u8; 8] = [rug_fuzz_0; 8];
        let mut p1: Block = [rug_fuzz_1; 32];
        let mut p2: SBox = [[rug_fuzz_2; 16]; 8];
        crate::gost94_core::encrypt(&mut p0, p1, &p2);
        let _rug_ed_tests_rug_121_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_122 {
    use super::*;
    use crate::gost94_core::Block;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_122_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0u8;
        let rug_fuzz_1 = 255u8;
        let p0: Block = [rug_fuzz_0; 32];
        let p1: Block = [rug_fuzz_1; 32];
        let _ = crate::gost94_core::x(&p0, &p1);
        let _rug_ed_tests_rug_122_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_123 {
    use super::*;
    use crate::gost94_core::Block;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_123_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 1;
        let mut p0: [u8; 32] = [rug_fuzz_0; 32];
        let p1: [u8; 32] = [rug_fuzz_1; 32];
        crate::gost94_core::x_mut(&mut p0, &p1);
        let _rug_ed_tests_rug_123_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_126 {
    use super::*;
    use crate::gost94_core::Block;
    #[test]
    fn test_psi() {
        let _rug_st_tests_rug_126_rrrruuuugggg_test_psi = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0x00;
        let rug_fuzz_2 = 0x01;
        let rug_fuzz_3 = 0x02;
        let rug_fuzz_4 = 0x03;
        let rug_fuzz_5 = 0x04;
        let rug_fuzz_6 = 0x05;
        let rug_fuzz_7 = 0x06;
        let rug_fuzz_8 = 0x07;
        let rug_fuzz_9 = 0x08;
        let rug_fuzz_10 = 0x09;
        let rug_fuzz_11 = 0x10;
        let rug_fuzz_12 = 0x11;
        let rug_fuzz_13 = 0x12;
        let rug_fuzz_14 = 0x13;
        let rug_fuzz_15 = 0x14;
        let rug_fuzz_16 = 0x15;
        let mut p0: [u8; 32] = [rug_fuzz_0; 32];
        p0.copy_from_slice(
            &[
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
                rug_fuzz_16,
            ],
        );
        crate::gost94_core::psi(&mut p0);
        let _rug_ed_tests_rug_126_rrrruuuugggg_test_psi = 0;
    }
}
#[cfg(test)]
mod tests_rug_127 {
    use super::*;
    #[test]
    fn test_adc() {
        let _rug_st_tests_rug_127_rrrruuuugggg_test_adc = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 5;
        let rug_fuzz_2 = 0;
        let mut p0: u64 = rug_fuzz_0;
        let p1: u64 = rug_fuzz_1;
        let mut p2: u64 = rug_fuzz_2;
        crate::gost94_core::adc(&mut p0, p1, &mut p2);
        debug_assert_eq!(p0, 15);
        debug_assert_eq!(p2, 0);
        let _rug_ed_tests_rug_127_rrrruuuugggg_test_adc = 0;
    }
}
