use super::Value;
use alloc::string::String;
fn eq_i64(value: &Value, other: i64) -> bool {
    value.as_i64().map_or(false, |i| i == other)
}
fn eq_u64(value: &Value, other: u64) -> bool {
    value.as_u64().map_or(false, |i| i == other)
}
fn eq_f32(value: &Value, other: f32) -> bool {
    match value {
        Value::Number(n) => n.as_f32().map_or(false, |i| i == other),
        _ => false,
    }
}
fn eq_f64(value: &Value, other: f64) -> bool {
    value.as_f64().map_or(false, |i| i == other)
}
fn eq_bool(value: &Value, other: bool) -> bool {
    value.as_bool().map_or(false, |i| i == other)
}
fn eq_str(value: &Value, other: &str) -> bool {
    value.as_str().map_or(false, |i| i == other)
}
impl PartialEq<str> for Value {
    fn eq(&self, other: &str) -> bool {
        eq_str(self, other)
    }
}
impl<'a> PartialEq<&'a str> for Value {
    fn eq(&self, other: &&str) -> bool {
        eq_str(self, *other)
    }
}
impl PartialEq<Value> for str {
    fn eq(&self, other: &Value) -> bool {
        eq_str(other, self)
    }
}
impl<'a> PartialEq<Value> for &'a str {
    fn eq(&self, other: &Value) -> bool {
        eq_str(other, *self)
    }
}
impl PartialEq<String> for Value {
    fn eq(&self, other: &String) -> bool {
        eq_str(self, other.as_str())
    }
}
impl PartialEq<Value> for String {
    fn eq(&self, other: &Value) -> bool {
        eq_str(other, self.as_str())
    }
}
macro_rules! partialeq_numeric {
    ($($eq:ident [$($ty:ty)*])*) => {
        $($(impl PartialEq <$ty > for Value { fn eq(& self, other : &$ty) -> bool { $eq
        (self, * other as _) } } impl PartialEq < Value > for $ty { fn eq(& self, other :
        & Value) -> bool { $eq (other, * self as _) } } impl <'a > PartialEq <$ty > for
        &'a Value { fn eq(& self, other : &$ty) -> bool { $eq (* self, * other as _) } }
        impl <'a > PartialEq <$ty > for &'a mut Value { fn eq(& self, other : &$ty) ->
        bool { $eq (* self, * other as _) } })*)*
    };
}
partialeq_numeric! {
    eq_i64[i8 i16 i32 i64 isize] eq_u64[u8 u16 u32 u64 usize] eq_f32[f32] eq_f64[f64]
    eq_bool[bool]
}
#[cfg(test)]
mod tests_rug_429 {
    use super::*;
    use crate::{json, Map, Value};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut v31 = Value::default();
        let p0: &Value = &v31;
        let p1: i64 = rug_fuzz_0;
        crate::value::partial_eq::eq_i64(p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_430 {
    use super::*;
    use crate::{json, Map, Value};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = {
            let mut v31 = Value::default();
            v31
        };
        let p1: u64 = rug_fuzz_0;
        crate::value::partial_eq::eq_u64(&p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_431 {
    use super::*;
    use crate::{json, Map, Value};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Value::default();
        let mut p1 = rug_fuzz_0;
        crate::value::partial_eq::eq_f32(&p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_432 {
    use super::*;
    use crate::{json, Map, Value};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Value::default();
        let p1 = rug_fuzz_0;
        crate::value::partial_eq::eq_f64(&p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_433 {
    use super::*;
    use crate::{json, Map, Value};
    #[test]
    fn test_eq_bool() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut v31 = Value::default();
        let p0 = &v31;
        let p1 = rug_fuzz_0;
        crate::value::partial_eq::eq_bool(p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_434 {
    use super::*;
    use crate::Value;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Value::default();
        let p1 = rug_fuzz_0;
        crate::value::partial_eq::eq_str(&p0, &p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_435 {
    use super::*;
    use crate::Value;
    #[test]
    fn test_eq() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut v31 = Value::default();
        let p1: &str = rug_fuzz_0;
        v31.eq(&p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_436 {
    use super::*;
    use crate::Value;
    #[test]
    fn test_eq() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Value::default();
        let p1: &&str = &&rug_fuzz_0;
        p0.eq(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_437 {
    use super::*;
    use crate::{json, Map, Value};
    #[test]
    fn test_eq() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(&str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: &str = rug_fuzz_0;
        let p1: Value = json!(rug_fuzz_1);
        <str>::eq(p0, &p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_438 {
    use super::*;
    use crate::{json, Map, Value};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_438_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "test_string";
        let mut p0: &'static str = rug_fuzz_0;
        let mut v31 = Value::default();
        let p1 = &v31;
        p0.eq(p1);
        let _rug_ed_tests_rug_438_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_440 {
    use super::*;
    use crate::value::partial_eq::eq_str;
    use crate::value::Value;
    use crate::{json, Map};
    #[test]
    fn test_eq() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(&str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: String = rug_fuzz_0.to_string();
        let mut v31 = Value::default();
        v31 = Value::String(rug_fuzz_1.to_string());
        let p1: &Value = &v31;
        debug_assert_eq!(p0.eq(p1), true);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_443 {
    use super::*;
    use crate::json;
    use crate::Value;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Value::default();
        let mut v31 = Value::default();
        let p1: i8 = rug_fuzz_0;
        p0.eq(&p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_445 {
    use super::*;
    use crate::map::Map;
    use crate::Value;
    #[test]
    fn test_eq() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = {
            let mut v31 = Value::default();
            v31
        };
        let p1: i16 = rug_fuzz_0;
        p0.eq(&p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_446 {
    use super::*;
    use crate::{json, Map, Value};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: i16 = rug_fuzz_0;
        let mut p1: Value = json!({ "key1" : "value1", "key2" : 2, "key3" : false });
        p0.eq(&p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_447 {
    use super::*;
    use crate::json;
    use crate::Value;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = json!({ "key1" : "value1", "key2" : 2, });
        let p1: i16 = rug_fuzz_0;
        p0.eq(&p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_448 {
    use super::*;
    use crate::{json, Map, Value};
    #[test]
    fn test_eq() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Value::default();
        let mut p1: i16 = rug_fuzz_0;
        p0.eq(&p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_449 {
    use crate::Value;
    #[test]
    fn test_eq() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Value::default();
        let mut p1 = rug_fuzz_0;
        p0.eq(&p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_450 {
    use super::*;
    use crate::json;
    use crate::Value;
    #[test]
    fn test_eq() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: i32 = rug_fuzz_0;
        let mut p1 = json!(rug_fuzz_1);
        <i32>::eq(&p0, &p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_451 {
    use super::*;
    use crate::{json, Map, Value};
    use std::cmp::PartialEq;
    #[test]
    fn test_eq() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut v31 = Value::default();
        let p0: &Value = &v31;
        let p1: &i32 = &rug_fuzz_0;
        p0.eq(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_452 {
    use super::*;
    use crate::Value;
    #[test]
    fn test_eq() {
        let _rug_st_tests_rug_452_rrrruuuugggg_sample = 0;
        #[cfg(test)]
        mod tests_rug_452_prepare {
            use super::*;
            use crate::{json, Map, Value};
            #[test]
            fn sample() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

                let _rug_st_tests_rug_452_rrrruuuugggg_sample = rug_fuzz_0;
                let mut v31 = Value::default();
                let _rug_ed_tests_rug_452_rrrruuuugggg_sample = rug_fuzz_1;
             }
}
}
}            }
        }
        let p0 = Value::default();
        let p1: i32 = 42;
        assert_eq!(p0.eq(& p1), false);
        let _rug_ed_tests_rug_452_rrrruuuugggg_sample = 0;
    }
}
#[cfg(test)]
mod tests_rug_454 {
    use super::*;
    use crate::{json, Map, Value};
    #[test]
    fn test_eq() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: i64 = rug_fuzz_0;
        let mut v31 = Value::default();
        let p1: &Value = &v31;
        <i64>::eq(&p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_456 {
    use super::*;
    use crate::Value;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Value::default();
        let p1: i64 = rug_fuzz_0;
        p0.eq(&p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_457 {
    use super::*;
    use crate::json;
    use crate::Value;
    #[test]
    fn test_eq() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(isize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Value::default();
        let p1: isize = rug_fuzz_0;
        Value::eq(&p0, &p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_459 {
    use super::*;
    use crate::json;
    use crate::Value;
    #[test]
    fn test_eq() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(isize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: Value = json!({ "key1" : "value1", "key2" : 2 });
        let p1: isize = rug_fuzz_0;
        p0.eq(&p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_460 {
    use super::*;
    use crate::{json, Map, Value};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(isize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Value::default();
        let mut v31: isize = rug_fuzz_0;
        p0.eq(&v31);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_462 {
    use super::*;
    use crate::{json, Map, Value};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: u8 = rug_fuzz_0;
        let mut v31 = Value::default();
        p0.eq(&v31);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_463 {
    use super::*;
    use crate::{json, Map, Value};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut v31 = Value::default();
        let mut p0 = &v31;
        let mut p1: u8 = rug_fuzz_0;
        p0.eq(&p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_465 {
    use super::*;
    use crate::json;
    #[test]
    fn test_eq() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Value::default();
        let p1: u16 = rug_fuzz_0;
        <Value as PartialEq<u16>>::eq(&p0, &p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_466 {
    use super::*;
    use crate::value::partial_eq;
    use crate::{json, Map, Value};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: u16 = rug_fuzz_0;
        let mut v31 = Value::default();
        let mut p1: Value = v31;
        <u16>::eq(&p0, &p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_467 {
    use super::*;
    use crate::value::Value;
    use crate::{json, Map};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut v31 = Value::default();
        let p0 = &v31;
        let p1: u16 = rug_fuzz_0;
        p0.eq(&p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_469 {
    use super::*;
    use crate::{Value, json};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Value::default();
        let mut p1: u32 = rug_fuzz_0;
        <Value>::eq(&p0, &p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_471 {
    use super::*;
    use crate::{json, Map, Value};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut v31 = Value::default();
        let mut p0 = &v31;
        let mut p1: u32 = rug_fuzz_0;
        p0.eq(&p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_473 {
    use super::*;
    use crate::Value;
    use std::cmp::PartialEq;
    #[test]
    fn test_eq() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Value::default();
        let mut p1: u64 = rug_fuzz_0;
        p0.eq(&p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_476 {
    use super::*;
    use crate::json;
    use crate::Value;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Value::default();
        let p1: u64 = rug_fuzz_0;
        p0.eq(&p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_478 {
    use super::*;
    use crate::value::partial_eq;
    use crate::{json, Map, Value};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: usize = rug_fuzz_0;
        let mut p1 = Value::default();
        <usize>::eq(&p0, &p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_479 {
    use super::*;
    use crate::{Map, Value};
    #[test]
    fn test_eq() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Value::default();
        let p1 = rug_fuzz_0;
        p0.eq(&p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_481 {
    use super::*;
    use crate::Value;
    #[test]
    fn test_eq() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0 = Value::default();
        let p1: f32 = rug_fuzz_0;
        <Value as PartialEq<f32>>::eq(&p0, &p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_482 {
    use super::*;
    use crate::{json, Map, Value};
    #[test]
    fn test_eq() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(f32, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: f32 = rug_fuzz_0;
        let p1: Value = json!(rug_fuzz_1);
        debug_assert_eq!(< f32 > ::eq(& p0, & p1), true);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_483 {
    use super::*;
    use crate::Value;
    #[test]
    fn test_eq() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0 = Value::default();
        let p1: f32 = rug_fuzz_0;
        p0.eq(&p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_485 {
    use super::*;
    use crate::{json, Map, Value};
    #[test]
    fn test_eq() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = {
            let mut v31 = Value::default();
            v31
        };
        let p1: f64 = rug_fuzz_0;
        p0.eq(&p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_486 {
    use super::*;
    use crate::{json, Map, Value};
    #[test]
    fn test_eq() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: f64 = rug_fuzz_0;
        let mut v31 = Value::default();
        let p1: &Value = &v31;
        <f64>::eq(&p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_488 {
    use super::*;
    use crate::{json, Map, Value};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Value = Value::default();
        let mut p1: f64 = rug_fuzz_0;
        p0.eq(&p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_489 {
    use super::*;
    use crate::json;
    use crate::Value;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i32, bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = {
            let mut v31 = Value::default();
            v31 = json!(rug_fuzz_0);
            v31
        };
        let mut p1 = rug_fuzz_1;
        <Value>::eq(&p0, &p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_490 {
    use super::*;
    use crate::json;
    use crate::Value;
    #[test]
    fn test_eq() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: bool = rug_fuzz_0;
        let p1: Value = json!({ "name" : "John", "age" : 30, "city" : "New York" });
        p0.eq(&p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_491 {
    use super::*;
    use crate::Value;
    #[test]
    fn test_eq() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Value::default();
        let p1: bool = rug_fuzz_0;
        p0.eq(&p1);
             }
}
}
}    }
}
