#[cfg(target_arch = "x86")]
use core::arch::x86 as arch;
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64 as arch;
#[derive(Clone)]
pub struct State {
    state: u32,
}
impl State {
    #[cfg(not(feature = "std"))]
    pub fn new(state: u32) -> Option<Self> {
        if cfg!(target_feature = "pclmulqdq") && cfg!(target_feature = "sse2")
            && cfg!(target_feature = "sse4.1")
        {
            Some(Self { state })
        } else {
            None
        }
    }
    #[cfg(feature = "std")]
    pub fn new(state: u32) -> Option<Self> {
        if is_x86_feature_detected!("pclmulqdq") && is_x86_feature_detected!("sse2")
            && is_x86_feature_detected!("sse4.1")
        {
            Some(Self { state })
        } else {
            None
        }
    }
    pub fn update(&mut self, buf: &[u8]) {
        self.state = unsafe { calculate(self.state, buf) };
    }
    pub fn finalize(self) -> u32 {
        self.state
    }
    pub fn reset(&mut self) {
        self.state = 0;
    }
    pub fn combine(&mut self, other: u32, amount: u64) {
        self.state = ::combine::combine(self.state, other, amount);
    }
}
const K1: i64 = 0x154442bd4;
const K2: i64 = 0x1c6e41596;
const K3: i64 = 0x1751997d0;
const K4: i64 = 0x0ccaa009e;
const K5: i64 = 0x163cd6124;
const K6: i64 = 0x1db710640;
const P_X: i64 = 0x1DB710641;
const U_PRIME: i64 = 0x1F7011641;
#[cfg(feature = "std")]
unsafe fn debug(s: &str, a: arch::__m128i) -> arch::__m128i {
    if false {
        union A {
            a: arch::__m128i,
            b: [u8; 16],
        }
        let x = A { a }.b;
        print!(" {:20} | ", s);
        for x in x.iter() {
            print!("{:02x} ", x);
        }
        println!();
    }
    return a;
}
#[cfg(not(feature = "std"))]
unsafe fn debug(_s: &str, a: arch::__m128i) -> arch::__m128i {
    a
}
#[target_feature(enable = "pclmulqdq", enable = "sse2", enable = "sse4.1")]
unsafe fn calculate(crc: u32, mut data: &[u8]) -> u32 {
    if data.len() < 128 {
        return ::baseline::update_fast_16(crc, data);
    }
    let mut x3 = get(&mut data);
    let mut x2 = get(&mut data);
    let mut x1 = get(&mut data);
    let mut x0 = get(&mut data);
    x3 = arch::_mm_xor_si128(x3, arch::_mm_cvtsi32_si128(!crc as i32));
    let k1k2 = arch::_mm_set_epi64x(K2, K1);
    while data.len() >= 64 {
        x3 = reduce128(x3, get(&mut data), k1k2);
        x2 = reduce128(x2, get(&mut data), k1k2);
        x1 = reduce128(x1, get(&mut data), k1k2);
        x0 = reduce128(x0, get(&mut data), k1k2);
    }
    let k3k4 = arch::_mm_set_epi64x(K4, K3);
    let mut x = reduce128(x3, x2, k3k4);
    x = reduce128(x, x1, k3k4);
    x = reduce128(x, x0, k3k4);
    while data.len() >= 16 {
        x = reduce128(x, get(&mut data), k3k4);
    }
    debug("128 > 64 init", x);
    drop(K6);
    let x = arch::_mm_xor_si128(
        arch::_mm_clmulepi64_si128(x, k3k4, 0x10),
        arch::_mm_srli_si128(x, 8),
    );
    let x = arch::_mm_xor_si128(
        arch::_mm_clmulepi64_si128(
            arch::_mm_and_si128(x, arch::_mm_set_epi32(0, 0, 0, !0)),
            arch::_mm_set_epi64x(0, K5),
            0x00,
        ),
        arch::_mm_srli_si128(x, 4),
    );
    debug("128 > 64 xx", x);
    let pu = arch::_mm_set_epi64x(U_PRIME, P_X);
    let t1 = arch::_mm_clmulepi64_si128(
        arch::_mm_and_si128(x, arch::_mm_set_epi32(0, 0, 0, !0)),
        pu,
        0x10,
    );
    let t2 = arch::_mm_clmulepi64_si128(
        arch::_mm_and_si128(t1, arch::_mm_set_epi32(0, 0, 0, !0)),
        pu,
        0x00,
    );
    let c = arch::_mm_extract_epi32(arch::_mm_xor_si128(x, t2), 1) as u32;
    if !data.is_empty() { ::baseline::update_fast_16(!c, data) } else { !c }
}
unsafe fn reduce128(
    a: arch::__m128i,
    b: arch::__m128i,
    keys: arch::__m128i,
) -> arch::__m128i {
    let t1 = arch::_mm_clmulepi64_si128(a, keys, 0x00);
    let t2 = arch::_mm_clmulepi64_si128(a, keys, 0x11);
    arch::_mm_xor_si128(arch::_mm_xor_si128(b, t1), t2)
}
unsafe fn get(a: &mut &[u8]) -> arch::__m128i {
    debug_assert!(a.len() >= 16);
    let r = arch::_mm_loadu_si128(a.as_ptr() as *const arch::__m128i);
    *a = &a[16..];
    return r;
}
#[cfg(test)]
mod test {
    quickcheck! {
        fn check_against_baseline(init : u32, chunks : Vec < (Vec < u8 >, usize) >) ->
        bool { let mut baseline = super::super::super::baseline::State::new(init); let
        mut pclmulqdq = super::State::new(init).expect("not supported"); for (chunk, mut
        offset) in chunks { offset &= 0xF; if chunk.len() <= offset { baseline.update(&
        chunk); pclmulqdq.update(& chunk); } else { baseline.update(& chunk[offset..]);
        pclmulqdq.update(& chunk[offset..]); } } pclmulqdq.finalize() == baseline
        .finalize() }
    }
}
#[cfg(test)]
mod tests_rug_12 {
    use super::*;
    #[test]
    fn test_calculate() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5, mut rug_fuzz_6, mut rug_fuzz_7, mut rug_fuzz_8, mut rug_fuzz_9, mut rug_fuzz_10)) = <(u32, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: u32 = rug_fuzz_0;
        let mut p1: [u8; 10] = [
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
        ];
        unsafe { calculate(p0, &mut p1) };
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_14 {
    use super::*;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5, mut rug_fuzz_6, mut rug_fuzz_7, mut rug_fuzz_8, mut rug_fuzz_9, mut rug_fuzz_10, mut rug_fuzz_11, mut rug_fuzz_12, mut rug_fuzz_13, mut rug_fuzz_14, mut rug_fuzz_15)) = <(u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: &[u8] = &[
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
        unsafe {
            crate::specialized::pclmulqdq::get(&mut p0);
        }
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_15 {
    use super::*;
    use specialized::pclmulqdq::State;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let state: u32 = rug_fuzz_0;
        State::new(state);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_16 {
    use super::*;
    use crate::specialized::pclmulqdq::State;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(u32, u8, u8, u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = State::new(rug_fuzz_0).unwrap();
        let p1: &[u8] = &[rug_fuzz_1, rug_fuzz_2, rug_fuzz_3];
        p0.update(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_17 {
    use super::*;
    use crate::specialized::pclmulqdq::State;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: State = State { state: rug_fuzz_0 };
        let result = p0.finalize();
        debug_assert_eq!(result, 0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_18 {
    use super::*;
    use crate::specialized::State;
    #[test]
    fn test_reset() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = State::new(rug_fuzz_0).unwrap();
        State::reset(&mut p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_19 {
    use super::*;
    use crate::combine;
    use crate::specialized::pclmulqdq::State;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(u32, u32, u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = State::new(rug_fuzz_0).unwrap();
        let p1: u32 = rug_fuzz_1;
        let p2: u64 = rug_fuzz_2;
        p0.combine(p1, p2);
             }
}
}
}    }
}
