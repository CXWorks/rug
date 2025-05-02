#![allow(unused)]
/// Forward a method to an inherent method or a base trait method.
macro_rules! forward {
    ($(Self:: $method:ident (self $(, $arg:ident : $ty:ty)*) -> $ret:ty;)*) => {
        $(#[inline] fn $method (self $(, $arg : $ty)*) -> $ret { Self::$method (self $(,
        $arg)*) })*
    };
    ($($base:ident :: $method:ident (self $(, $arg:ident : $ty:ty)*) -> $ret:ty;)*) => {
        $(#[inline] fn $method (self $(, $arg : $ty)*) -> $ret { < Self as $base
        >::$method (self $(, $arg)*) })*
    };
    ($($base:ident :: $method:ident ($($arg:ident : $ty:ty),*) -> $ret:ty;)*) => {
        $(#[inline] fn $method ($($arg : $ty),*) -> $ret { < Self as $base >::$method
        ($($arg),*) })*
    };
    ($($imp:path as $method:ident (self $(, $arg:ident : $ty:ty)*) -> $ret:ty;)*) => {
        $(#[inline] fn $method (self $(, $arg : $ty)*) -> $ret { $imp (self $(, $arg)*)
        })*
    };
}
macro_rules! constant {
    ($($method:ident () -> $ret:expr;)*) => {
        $(#[inline] fn $method () -> Self { $ret })*
    };
}
#[cfg(test)]
mod tests_rug_1861 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_1861_rrrruuuugggg_test_rug = 0;
        <f32 as FloatCore>::min_value();
        let _rug_ed_tests_rug_1861_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_1862 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_1862_rrrruuuugggg_test_rug = 0;
        <f32 as FloatCore>::min_positive_value();
        let _rug_ed_tests_rug_1862_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_1863 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_1863_rrrruuuugggg_test_rug = 0;
        <f32 as FloatCore>::epsilon();
        let _rug_ed_tests_rug_1863_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_1865 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        <f32 as FloatCore>::is_nan(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1866 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        p0.is_infinite();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1867 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: f32 = rug_fuzz_0;
        <f32 as FloatCore>::is_finite(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1868 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        <f32>::is_normal(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1869 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: f32 = rug_fuzz_0;
        <f32>::classify(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1870 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        <f32 as FloatCore>::floor(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1871 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        <f32 as FloatCore>::ceil(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1872 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_round() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        <f32 as FloatCore>::round(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1873 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        <f32 as FloatCore>::trunc(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1874 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        <f32 as FloatCore>::fract(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1875 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        <f32 as FloatCore>::abs(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1876 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        <f32 as FloatCore>::signum(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1877 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        <f32 as FloatCore>::is_sign_positive(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1878 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        <f32 as FloatCore>::is_sign_negative(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1879 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(f32, f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        let mut p1: f32 = rug_fuzz_1;
        <f32>::min(p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1880 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(f32, f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        let mut p1: f32 = rug_fuzz_1;
        <f32 as FloatCore>::max(p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1881 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        <f32>::recip(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1882 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(f32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        let mut p1: i32 = -rug_fuzz_1;
        <f32 as FloatCore>::powi(p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1883 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        <f32 as FloatCore>::to_degrees(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1884 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        <f32 as FloatCore>::to_radians(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1889 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_min_value() {
        let _rug_st_tests_rug_1889_rrrruuuugggg_test_min_value = 0;
        let result: f64 = <f64 as FloatCore>::min_value();
        debug_assert_eq!(result, 2.2250738585072014e-308);
        let _rug_ed_tests_rug_1889_rrrruuuugggg_test_min_value = 0;
    }
}
#[cfg(test)]
mod tests_rug_1890 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_1890_rrrruuuugggg_test_rug = 0;
        <f64 as FloatCore>::min_positive_value();
        let _rug_ed_tests_rug_1890_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_1892 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_1892_rrrruuuugggg_test_rug = 0;
        <f64 as FloatCore>::max_value();
        let _rug_ed_tests_rug_1892_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_1893 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64 as FloatCore>::is_nan(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1894 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64 as FloatCore>::is_infinite(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1895 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_is_finite() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: f64 = rug_fuzz_0;
        <f64 as FloatCore>::is_finite(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1896 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64 as FloatCore>::is_normal(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1897 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64>::classify(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1898 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_floor() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64 as FloatCore>::floor(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1899 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64 as FloatCore>::ceil(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1900 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_round() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: f64 = rug_fuzz_0;
        <f64 as FloatCore>::round(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1901 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64 as FloatCore>::trunc(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1902 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64 as FloatCore>::fract(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1903 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64 as FloatCore>::abs(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1904 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64 as FloatCore>::signum(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1905 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64 as FloatCore>::is_sign_positive(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1906 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64 as FloatCore>::is_sign_negative(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1907 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(f64, f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        let mut p1: f64 = rug_fuzz_1;
        <f64 as FloatCore>::min(p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1908 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(f64, f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        let mut p1: f64 = rug_fuzz_1;
        <f64 as FloatCore>::max(p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1909 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64 as FloatCore>::recip(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1910 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(f64, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        let mut p1: i32 = rug_fuzz_1;
        <f64 as FloatCore>::powi(p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1911 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64 as FloatCore>::to_degrees(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1912 {
    use super::*;
    use crate::float::FloatCore;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64 as FloatCore>::to_radians(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1915 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_1915_rrrruuuugggg_test_rug = 0;
        <f32 as Float>::neg_infinity();
        let _rug_ed_tests_rug_1915_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_1917 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_1917_rrrruuuugggg_test_rug = 0;
        <f32 as Float>::min_value();
        let _rug_ed_tests_rug_1917_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_1921 {
    use super::*;
    use crate::float::Float;
    #[test]
    fn test_is_nan() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: f32 = rug_fuzz_0;
        debug_assert_eq!(p0.is_nan(), false);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1922 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        <f32>::is_infinite(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1923 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        <f32>::is_finite(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1924 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        <f32>::is_normal(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1925 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        p0.classify();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1926 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        <f32 as Float>::floor(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1927 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        p0.ceil();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1928 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        p0.round();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1929 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_trunc() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        p0.trunc();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1930 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        p0.fract();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1931 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_abs() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        <f32 as Float>::abs(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1932 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        <f32 as Float>::signum(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1935 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_mul_add() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(f32, f32, f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: f32 = rug_fuzz_0;
        let p1: f32 = rug_fuzz_1;
        let p2: f32 = rug_fuzz_2;
        <f32 as Float>::mul_add(p0, p1, p2);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1936 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        <f32>::recip(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1937 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(f32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        let mut p1: i32 = rug_fuzz_1;
        <f32 as Float>::powi(p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1938 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(f32, f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        let mut p1: f32 = rug_fuzz_1;
        <f32 as Float>::powf(p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1939 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        <f32 as Float>::sqrt(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1942 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: f32 = rug_fuzz_0;
        <f32>::ln(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1943 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(f32, f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        let mut p1: f32 = rug_fuzz_1;
        p0.log(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1944 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        <f32>::log2(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1945 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        <f32>::log10(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1946 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        p0.to_degrees();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1947 {
    use super::*;
    use crate::{Float, float};
    #[test]
    fn test_to_radians() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        p0.to_radians();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1948 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_max() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(f32, f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        let mut p1: f32 = rug_fuzz_1;
        <f32 as Float>::max(p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1949 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(f32, f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        let mut p1: f32 = rug_fuzz_1;
        <f32 as Float>::min(p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1951 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(f32, f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        let mut p1: f32 = rug_fuzz_1;
        <f32 as Float>::hypot(p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1952 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        <f32>::sin(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1954 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        <f32>::tan(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1955 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        <f32>::asin(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1956 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        <f32 as Float>::acos(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1958 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(f32, f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        let mut p1: f32 = rug_fuzz_1;
        <f32 as Float>::atan2(p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1959 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        p0.sin_cos();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1960 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        <f32 as Float>::exp_m1(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1961 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_ln_1p() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        <f32>::ln_1p(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1962 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        <f32 as Float>::sinh(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1963 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        p0.cosh();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1965 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        <f32 as Float>::asinh(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1966 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        <f32 as Float>::acosh(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1967 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        <f32 as Float>::atanh(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1975 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_1975_rrrruuuugggg_test_rug = 0;
        <f64 as Float>::max_value();
        let _rug_ed_tests_rug_1975_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_1976 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = rug_fuzz_0;
        <f64>::is_nan(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1977 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64 as Float>::is_infinite(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1978 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        p0.is_finite();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1979 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64 as Float>::is_normal(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1980 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64 as Float>::classify(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1981 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64 as Float>::floor(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1982 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_ceil() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: f64 = rug_fuzz_0;
        <f64 as Float>::ceil(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1983 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        p0.round();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1984 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64 as Float>::trunc(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1985 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_fract() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64 as Float>::fract(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1987 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: f64 = rug_fuzz_0;
        <f64 as Float>::signum(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1989 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = rug_fuzz_0;
        p0.is_sign_negative();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1990 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(f64, f64, f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        let mut p1: f64 = rug_fuzz_1;
        let mut p2: f64 = rug_fuzz_2;
        <f64 as Float>::mul_add(p0, p1, p2);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1991 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64 as Float>::recip(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1992 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(f64, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        let mut p1: i32 = rug_fuzz_1;
        <f64 as Float>::powi(p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1994 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_sqrt() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: f64 = rug_fuzz_0;
        <f64>::sqrt(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1995 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_exp() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        p0.exp();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1996 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = rug_fuzz_0;
        p0.exp2();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1997 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_ln() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64 as Float>::ln(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1998 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(f64, f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        let mut p1: f64 = rug_fuzz_1;
        <f64 as Float>::log(p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1999 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64 as Float>::log2(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2000 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64 as Float>::log10(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2001 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64 as Float>::to_degrees(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2002 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64 as Float>::to_radians(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2003 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(f64, f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        let mut p1: f64 = rug_fuzz_1;
        p0.max(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2004 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(f64, f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        let mut p1: f64 = rug_fuzz_1;
        <f64 as Float>::min(p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2005 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64 as Float>::cbrt(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2006 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_hypot() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(f64, f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        let mut p1: f64 = rug_fuzz_1;
        p0.hypot(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2007 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        p0.sin();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2008 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        p0.cos();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2009 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64 as Float>::tan(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2010 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64 as Float>::asin(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2011 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        p0.acos();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2012 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64 as Float>::atan(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2013 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(f64, f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        let mut p1: f64 = rug_fuzz_1;
        <f64 as Float>::atan2(p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2014 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64>::sin_cos(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2015 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64>::exp_m1(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2016 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64 as Float>::ln_1p(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2017 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64 as Float>::sinh(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2018 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_cosh() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        p0.cosh();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2019 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64 as Float>::tanh(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2020 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64 as Float>::asinh(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2021 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        p0.acosh();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2022 {
    use super::*;
    use crate::Float;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64 as Float>::atanh(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2028 {
    use super::*;
    use crate::FloatConst;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_2028_rrrruuuugggg_test_rug = 0;
        let result: f32 = <f32 as FloatConst>::FRAC_PI_2();
        debug_assert_eq!(result, std::f32::consts::FRAC_PI_2);
        let _rug_ed_tests_rug_2028_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_2036 {
    use super::*;
    use crate::float::FloatConst;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_2036_rrrruuuugggg_test_rug = 0;
        let result: f32 = <f32 as FloatConst>::LOG2_E();
        debug_assert_eq!(result, 1.44269504);
        let _rug_ed_tests_rug_2036_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_2038 {
    use super::*;
    use crate::FloatConst;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_2038_rrrruuuugggg_test_rug = 0;
        let result: f32 = <f32 as FloatConst>::SQRT_2();
        debug_assert_eq!(result, 2.0_f32.sqrt());
        let _rug_ed_tests_rug_2038_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_2039 {
    use super::*;
    use crate::FloatConst;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_2039_rrrruuuugggg_test_rug = 0;
        let result: f32 = <f32 as FloatConst>::TAU();
        debug_assert_eq!(result, 6.2831855);
        let _rug_ed_tests_rug_2039_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_2040 {
    use super::*;
    use crate::float::FloatConst;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_2040_rrrruuuugggg_test_rug = 0;
        let result: f32 = <f32 as FloatConst>::LOG10_2();
        debug_assert_eq!(result, 0.3010299956639812);
        let result: f64 = <f64 as FloatConst>::LOG10_2();
        debug_assert_eq!(result, 0.30102999566398114);
        let _rug_ed_tests_rug_2040_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_2045 {
    use super::*;
    use crate::FloatConst;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_2045_rrrruuuugggg_test_rug = 0;
        <f64 as FloatConst>::FRAC_2_PI();
        let _rug_ed_tests_rug_2045_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_2057 {
    use super::*;
    use crate::float::FloatConst;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_2057_rrrruuuugggg_test_rug = 0;
        <f64 as FloatConst>::SQRT_2();
        let _rug_ed_tests_rug_2057_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_2058 {
    use super::*;
    use crate::FloatConst;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_2058_rrrruuuugggg_test_rug = 0;
        let tau: f64 = <f64 as FloatConst>::TAU();
        debug_assert_eq!(tau, 6.283185307179586);
        let _rug_ed_tests_rug_2058_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_2065 {
    use super::*;
    use crate::real::Real;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = rug_fuzz_0;
        debug_assert_eq!(p0.floor(), 3.0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2066 {
    use super::*;
    use crate::real::Real;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(f64, f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = rug_fuzz_0;
        debug_assert_eq!(p0.ceil(), 5.0);
        p0 = -rug_fuzz_1;
        debug_assert_eq!(p0.ceil(), - 5.0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2067 {
    use super::*;
    use crate::real::Real;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        <f32>::round(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2068 {
    use super::*;
    use crate::real::Real;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = rug_fuzz_0;
        debug_assert_eq!(p0.trunc(), 3.0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2069 {
    use super::*;
    use crate::real::Real;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = rug_fuzz_0;
        <f64>::fract(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2070 {
    use super::*;
    use crate::real::Real;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = rug_fuzz_0;
        <f64>::abs(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2071 {
    use super::*;
    use crate::real::Real;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = rug_fuzz_0;
        <f32 as Real>::signum(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2072 {
    use super::*;
    use crate::real::Real;
    use std::f64::consts;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(f64, f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = rug_fuzz_0;
        debug_assert_eq!(p0.is_sign_positive(), true);
        p0 = -rug_fuzz_1;
        debug_assert_eq!(p0.is_sign_positive(), false);
        p0 = f64::NAN;
        debug_assert_eq!(p0.is_sign_positive(), false);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2073 {
    use super::*;
    use crate::real::Real;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(f64, f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        debug_assert_eq!(f64::is_sign_negative(p0), false);
        p0 = -rug_fuzz_1;
        debug_assert_eq!(f64::is_sign_negative(p0), true);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2074 {
    use super::*;
    use crate::real::Real;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(f64, f64, f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = rug_fuzz_0;
        let mut p1 = rug_fuzz_1;
        let mut p2 = rug_fuzz_2;
        <f64 as Real>::mul_add(p0, p1, p2);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2075 {
    use super::*;
    use crate::real::Real;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64 as Real>::recip(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2076 {
    use super::*;
    use crate::real::Real;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(f64, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = rug_fuzz_0;
        let mut p1 = rug_fuzz_1;
        debug_assert_eq!(p0.powi(p1), 1764.0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2077 {
    use super::*;
    use crate::real::Real;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(f64, f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        let mut p1: f64 = rug_fuzz_1;
        <f64 as Real>::powf(p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2078 {
    use super::*;
    use crate::real::Real;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: f64 = rug_fuzz_0;
        debug_assert_eq!(< f64 as Real > ::sqrt(p0), 4.0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2080 {
    use super::*;
    use crate::real::Real;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        f64::exp2(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2081 {
    use super::*;
    use crate::real::Real;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64 as Real>::ln(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2082 {
    use super::*;
    use crate::real::Real;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(f32, f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        let mut p1: f32 = rug_fuzz_1;
        <f32 as Real>::log(p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2083 {
    use super::*;
    use crate::real::Real;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        debug_assert_eq!(< f64 as Real > ::log2(p0), 3.321928094887362);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2084 {
    use super::*;
    use crate::real::Real;
    #[test]
    fn test_log10() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: f64 = rug_fuzz_0;
        let result = f64::log10(p0);
        debug_assert_eq!(result, 1.0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2085 {
    use super::*;
    use crate::real::Real;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        p0 = p0.to_degrees();
        debug_assert_eq!(p0, 3.5_f64.to_degrees());
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2086 {
    use super::*;
    use crate::real::Real;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        let result: f32 = <f32 as Real>::to_radians(p0);
        debug_assert_eq!(result, std::f32::consts::PI);
             }
}
}
}    }
}
#[test]
fn test_rug() {
    let mut p0 = 0.0_f64;
    let mut p1 = 0.0_f64;
    p0.max(p1);
}
#[cfg(test)]
mod tests_rug_2088 {
    use super::*;
    use crate::real::Real;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(f32, f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = rug_fuzz_0;
        let mut p1 = rug_fuzz_1;
        p0.min(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2089 {
    use super::*;
    use crate::real::Real;
    #[test]
    fn test_abs_sub() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(f64, f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        let mut p1: f64 = rug_fuzz_1;
        <f64>::abs_sub(p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2090 {
    use super::*;
    use crate::real::Real;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = rug_fuzz_0;
        <f64 as Real>::cbrt(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2091 {
    use super::*;
    use crate::real::Real;
    use std::f64;
    #[test]
    fn test_hypot() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(f64, f64, f64, f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = rug_fuzz_0;
        let mut p1 = rug_fuzz_1;
        debug_assert_eq!(< f64 > ::hypot(p0, p1), 5.0);
        p0 = -rug_fuzz_2;
        p1 = rug_fuzz_3;
        debug_assert_eq!(< f64 > ::hypot(p0, p1), 9.923470214571523);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2092 {
    use super::*;
    use crate::real::Real;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        p0.sin();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2093 {
    use super::*;
    use crate::real::Real;
    #[test]
    fn test_cos() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(f32, f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        debug_assert_eq!(< f32 > ::cos(p0), 0.5403023);
        let mut p1: f64 = rug_fuzz_1;
        debug_assert_eq!(< f64 > ::cos(p1), - 0.4161468365471424);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2094 {
    use super::*;
    use crate::real::Real;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        let result = <f64 as Real>::tan(p0);
        debug_assert_eq!(result, 1.5574077246549023);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2095 {
    use super::*;
    use crate::real::Real;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        <f32 as Real>::asin(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2096 {
    use super::*;
    use crate::real::Real;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(f32, f32, f32, f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        debug_assert_eq!(p0.acos(), 1.0471976);
        p0 = -rug_fuzz_1;
        debug_assert_eq!(p0.acos(), 2.0943952);
        p0 = rug_fuzz_2;
        debug_assert_eq!(p0.acos(), 0.0);
        p0 = -rug_fuzz_3;
        debug_assert_eq!(p0.acos(), std::f32::consts::PI);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2097 {
    use super::*;
    use crate::real::Real;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5)) = <(f32, f32, f32, f64, f64, f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = rug_fuzz_0;
        debug_assert_eq!(p0.atan(), 0.4636476);
        p0 = rug_fuzz_1;
        debug_assert_eq!(p0.atan(), 0.7853982);
        p0 = -rug_fuzz_2;
        debug_assert_eq!(p0.atan(), - 0.7853982);
        let mut p1 = rug_fuzz_3;
        debug_assert_eq!(p1.atan(), 0.9827937);
        p1 = -rug_fuzz_4;
        debug_assert_eq!(p1.atan(), - 0.9827937);
        p1 = rug_fuzz_5;
        debug_assert_eq!(p1.atan(), 1.1787352619);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2098 {
    use super::*;
    use crate::real::Real;
    use std::f64;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(f64, f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: f64 = rug_fuzz_0;
        let p1: f64 = rug_fuzz_1;
        <f64 as crate::real::Real>::atan2(p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2099 {
    use super::*;
    use crate::real::Real;
    #[test]
    fn test_sin_cos() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(f32, f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        debug_assert!(p0.sin_cos() == (p0.sin(), p0.cos()));
        p0 = std::f32::consts::PI;
        debug_assert!(p0.sin_cos() == (p0.sin(), p0.cos()));
        p0 = -rug_fuzz_1;
        debug_assert!(p0.sin_cos() == (p0.sin(), p0.cos()));
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2100 {
    use super::*;
    use crate::real::Real;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        <f64>::exp_m1(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2101 {
    use super::*;
    use crate::real::Real;
    use std::f32;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(f32, f32, f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        debug_assert_eq!(p0.ln_1p(), f32::ln_1p(p0));
        p0 = -rug_fuzz_1;
        debug_assert_eq!(p0.ln_1p(), f32::ln_1p(p0));
        p0 = rug_fuzz_2;
        debug_assert_eq!(p0.ln_1p(), f32::ln_1p(p0));
        p0 = f32::NAN;
        debug_assert!(p0.ln_1p().is_nan());
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2102 {
    use super::*;
    use crate::real::Real;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        p0.sinh();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2103 {
    use super::*;
    use crate::real::Real;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f64 = rug_fuzz_0;
        p0.cosh();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2106 {
    use super::*;
    use crate::real::Real;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(f32, f64, f32, f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: f32 = rug_fuzz_0;
        p0.acosh();
        let mut p1: f64 = rug_fuzz_1;
        p1.acosh();
        let mut p2: f32 = rug_fuzz_2;
        p2.acosh();
        let mut p3: f64 = rug_fuzz_3;
        p3.acosh();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2107 {
    use super::*;
    use crate::real::Real;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(f64, f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = rug_fuzz_0;
        let result = <f64>::atanh(p0);
        let expected = rug_fuzz_1;
        debug_assert_eq!(result, expected);
             }
}
}
}    }
}
