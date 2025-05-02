const GF2_DIM: usize = 32;
fn gf2_matrix_times(mat: &[u32; GF2_DIM], mut vec: u32) -> u32 {
    let mut sum = 0;
    let mut idx = 0;
    while vec > 0 {
        if vec & 1 == 1 {
            sum ^= mat[idx];
        }
        vec >>= 1;
        idx += 1;
    }
    return sum;
}
fn gf2_matrix_square(square: &mut [u32; GF2_DIM], mat: &[u32; GF2_DIM]) {
    for n in 0..GF2_DIM {
        square[n] = gf2_matrix_times(mat, mat[n]);
    }
}
pub(crate) fn combine(mut crc1: u32, crc2: u32, mut len2: u64) -> u32 {
    let mut row: u32;
    let mut even = [0u32; GF2_DIM];
    let mut odd = [0u32; GF2_DIM];
    if len2 <= 0 {
        return crc1;
    }
    odd[0] = 0xedb88320;
    row = 1;
    for n in 1..GF2_DIM {
        odd[n] = row;
        row <<= 1;
    }
    gf2_matrix_square(&mut even, &odd);
    gf2_matrix_square(&mut odd, &even);
    loop {
        gf2_matrix_square(&mut even, &odd);
        if len2 & 1 == 1 {
            crc1 = gf2_matrix_times(&even, crc1);
        }
        len2 >>= 1;
        if len2 == 0 {
            break;
        }
        gf2_matrix_square(&mut odd, &even);
        if len2 & 1 == 1 {
            crc1 = gf2_matrix_times(&odd, crc1);
        }
        len2 >>= 1;
        if len2 == 0 {
            break;
        }
    }
    crc1 ^= crc2;
    return crc1;
}
#[cfg(test)]
mod tests_rug_8 {
    use super::*;
    use crate::combine;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u32, u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mat: [u32; GF2_DIM] = [rug_fuzz_0; GF2_DIM];
        let vec: u32 = rug_fuzz_1;
        combine::gf2_matrix_times(&mat, vec);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_9 {
    use super::*;
    use crate::combine::{GF2_DIM, gf2_matrix_times};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u32, u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: [u32; GF2_DIM] = [rug_fuzz_0; GF2_DIM];
        let p1: [u32; GF2_DIM] = [rug_fuzz_1; GF2_DIM];
        gf2_matrix_square(&mut p0, &p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_10 {
    use super::*;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(u32, u32, u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: u32 = rug_fuzz_0;
        let mut p1: u32 = rug_fuzz_1;
        let mut p2: u64 = rug_fuzz_2;
        combine(p0, p1, p2);
             }
}
}
}    }
}
