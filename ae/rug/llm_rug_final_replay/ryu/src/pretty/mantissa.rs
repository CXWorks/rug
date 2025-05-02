use crate::digit_table::*;
use core::ptr;
#[cfg_attr(feature = "no-panic", inline)]
pub unsafe fn write_mantissa_long(mut output: u64, mut result: *mut u8) {
    if (output >> 32) != 0 {
        let mut output2 = (output - 100_000_000 * (output / 100_000_000)) as u32;
        output /= 100_000_000;
        let c = output2 % 10_000;
        output2 /= 10_000;
        let d = output2 % 10_000;
        let c0 = (c % 100) << 1;
        let c1 = (c / 100) << 1;
        let d0 = (d % 100) << 1;
        let d1 = (d / 100) << 1;
        ptr::copy_nonoverlapping(
            DIGIT_TABLE.as_ptr().offset(c0 as isize),
            result.offset(-2),
            2,
        );
        ptr::copy_nonoverlapping(
            DIGIT_TABLE.as_ptr().offset(c1 as isize),
            result.offset(-4),
            2,
        );
        ptr::copy_nonoverlapping(
            DIGIT_TABLE.as_ptr().offset(d0 as isize),
            result.offset(-6),
            2,
        );
        ptr::copy_nonoverlapping(
            DIGIT_TABLE.as_ptr().offset(d1 as isize),
            result.offset(-8),
            2,
        );
        result = result.offset(-8);
    }
    write_mantissa(output as u32, result);
}
#[cfg_attr(feature = "no-panic", inline)]
pub unsafe fn write_mantissa(mut output: u32, mut result: *mut u8) {
    while output >= 10_000 {
        let c = output - 10_000 * (output / 10_000);
        output /= 10_000;
        let c0 = (c % 100) << 1;
        let c1 = (c / 100) << 1;
        ptr::copy_nonoverlapping(
            DIGIT_TABLE.as_ptr().offset(c0 as isize),
            result.offset(-2),
            2,
        );
        ptr::copy_nonoverlapping(
            DIGIT_TABLE.as_ptr().offset(c1 as isize),
            result.offset(-4),
            2,
        );
        result = result.offset(-4);
    }
    if output >= 100 {
        let c = (output % 100) << 1;
        output /= 100;
        ptr::copy_nonoverlapping(
            DIGIT_TABLE.as_ptr().offset(c as isize),
            result.offset(-2),
            2,
        );
        result = result.offset(-2);
    }
    if output >= 10 {
        let c = output << 1;
        ptr::copy_nonoverlapping(
            DIGIT_TABLE.as_ptr().offset(c as isize),
            result.offset(-2),
            2,
        );
    } else {
        *result.offset(-1) = b'0' + output as u8;
    }
}
#[cfg(test)]
mod tests_rug_26 {
    use super::*;
    use crate::pretty::mantissa::write_mantissa_long;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut output: u64 = rug_fuzz_0;
        let mut result: *mut u8 = std::ptr::null_mut();
        unsafe {
            write_mantissa_long(output, result);
        }
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_27 {
    use super::*;
    use std::ptr;
    use crate::pretty::mantissa::DIGIT_TABLE;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u32, u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let output: u32 = rug_fuzz_0;
        let mut result: [u8; 10] = [rug_fuzz_1; 10];
        let p0 = output;
        let p1 = result.as_mut_ptr();
        unsafe {
            crate::pretty::mantissa::write_mantissa(p0, p1);
        }
             }
}
}
}    }
}
