use super::Value;
use crate::lib::*;
fn eq_i64(value: &Value, other: i64) -> bool {
    value.as_i64().map_or(false, |i| i == other)
}
fn eq_u64(value: &Value, other: u64) -> bool {
    value.as_u64().map_or(false, |i| i == other)
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
    eq_i64[i8 i16 i32 i64 isize] eq_u64[u8 u16 u32 u64 usize] eq_f64[f32 f64]
    eq_bool[bool]
}
#[cfg(test)]
mod tests_rug_372 {
    use super::*;
    use crate::{Number, Value, Map};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_372_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let rug_fuzz_2 = 42;
        let mut v29 = Value::Object(Map::new());
        v29[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
        let p0: &Value = &v29;
        let p1: i64 = rug_fuzz_2;
        crate::value::partial_eq::eq_i64(p0, p1);
        let _rug_ed_tests_rug_372_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_373 {
    use super::*;
    use crate::{Number, Value, Map};
    #[test]
    fn test_eq_u64() {
        let _rug_st_tests_rug_373_rrrruuuugggg_test_eq_u64 = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let rug_fuzz_2 = 42;
        let mut v29 = Value::Object(Map::new());
        v29[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
        let p0: &Value = &v29;
        let p1: u64 = rug_fuzz_2;
        crate::value::partial_eq::eq_u64(p0, p1);
        let _rug_ed_tests_rug_373_rrrruuuugggg_test_eq_u64 = 0;
    }
}
#[cfg(test)]
mod tests_rug_374 {
    use super::*;
    use crate::{Number, Value, Map};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_374_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let rug_fuzz_2 = 3.14;
        let mut p0 = {
            let mut v29 = Value::Object(Map::new());
            v29[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
            v29
        };
        let mut p1 = rug_fuzz_2;
        crate::value::partial_eq::eq_f64(&p0, p1);
        let _rug_ed_tests_rug_374_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_375 {
    use super::*;
    use crate::{Number, Value, Map};
    #[test]
    fn test_eq_bool() {
        let _rug_st_tests_rug_375_rrrruuuugggg_test_eq_bool = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let rug_fuzz_2 = true;
        let mut v29 = Value::Object(Map::new());
        v29[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
        let p0: &Value = &v29;
        let p1: bool = rug_fuzz_2;
        debug_assert_eq!(eq_bool(p0, p1), false);
        let _rug_ed_tests_rug_375_rrrruuuugggg_test_eq_bool = 0;
    }
}
#[cfg(test)]
mod tests_rug_376 {
    use super::*;
    use crate::{Number, Value, Map};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_376_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let rug_fuzz_2 = "key";
        let mut v29 = Value::Object(Map::new());
        v29[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
        let p0: &Value = &v29;
        let p1: &str = rug_fuzz_2;
        crate::value::partial_eq::eq_str(p0, &p1);
        let _rug_ed_tests_rug_376_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_380 {
    use super::*;
    use crate::{Number, Value, Map};
    #[test]
    fn test_eq() {
        let _rug_st_tests_rug_380_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "sample_string";
        let rug_fuzz_1 = "key";
        let rug_fuzz_2 = 42;
        let p0: &'static str = rug_fuzz_0;
        let mut v29 = Value::Object(Map::new());
        v29[rug_fuzz_1] = Value::Number(Number::from(rug_fuzz_2));
        let p1: &Value = &v29;
        p0.eq(p1);
        let _rug_ed_tests_rug_380_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_rug_381 {
    use super::*;
    use crate::{Value, Map};
    #[test]
    fn test_eq() {
        let _rug_st_tests_rug_381_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let rug_fuzz_2 = "example_string";
        let mut p0 = {
            let mut v29 = Value::Object(Map::new());
            v29[rug_fuzz_0] = Value::Number(rug_fuzz_1.into());
            v29
        };
        let mut p1 = String::from(rug_fuzz_2);
        p0.eq(&p1);
        let _rug_ed_tests_rug_381_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_rug_382 {
    use super::*;
    use crate::{Number, Value, Map};
    #[test]
    fn test_eq() {
        let _rug_st_tests_rug_382_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "some_string";
        let rug_fuzz_1 = "key";
        let rug_fuzz_2 = 42;
        let mut p0: std::string::String = String::from(rug_fuzz_0);
        let mut p1: Value = {
            let mut v29 = Value::Object(Map::new());
            v29[rug_fuzz_1] = Value::Number(Number::from(rug_fuzz_2));
            v29
        };
        <std::string::String>::eq(&p0, &p1);
        let _rug_ed_tests_rug_382_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_rug_387 {
    use super::*;
    use crate::Number;
    use crate::Value;
    use crate::Map;
    #[test]
    fn test_eq() {
        let _rug_st_tests_rug_387_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let rug_fuzz_2 = 42;
        let mut v29 = Value::Object(Map::new());
        v29[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
        let p0: &Value = &v29;
        let p1: &i16 = &rug_fuzz_2;
        p0.eq(p1);
        let _rug_ed_tests_rug_387_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_rug_388 {
    use super::*;
    use crate::{Number, Value, Map};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_388_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = "key";
        let rug_fuzz_2 = 42;
        let mut p0: i16 = rug_fuzz_0;
        let mut p1: Value = {
            let mut v29 = Value::Object(Map::new());
            v29[rug_fuzz_1] = Value::Number(Number::from(rug_fuzz_2));
            v29
        };
        <i16>::eq(&p0, &p1);
        let _rug_ed_tests_rug_388_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_389 {
    use super::*;
    use crate::{Number, Value, Map};
    #[test]
    fn test_eq() {
        let _rug_st_tests_rug_389_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let rug_fuzz_2 = 42;
        let mut p0 = {
            let mut v29 = Value::Object(Map::new());
            v29[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
            v29
        };
        let mut p1: i16 = rug_fuzz_2;
        p0.eq(&p1);
        let _rug_ed_tests_rug_389_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_rug_392 {
    use super::*;
    use crate::Value;
    use crate::{Number, Map};
    #[test]
    fn test_eq() {
        let _rug_st_tests_rug_392_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = 42;
        let rug_fuzz_1 = "key";
        let rug_fuzz_2 = 42;
        let mut p0: i32 = rug_fuzz_0;
        let mut v29 = Value::Object(Map::new());
        v29[rug_fuzz_1] = Value::Number(Number::from(rug_fuzz_2));
        let p1 = &v29;
        p0.eq(p1);
        let _rug_ed_tests_rug_392_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_rug_393 {
    use super::*;
    use crate::{Number, Value, Map};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_393_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let rug_fuzz_2 = 42;
        let mut p0 = {
            let mut v29 = Value::Object(Map::new());
            v29[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
            v29
        };
        let p1: i32 = rug_fuzz_2;
        p0.eq(&p1);
        let _rug_ed_tests_rug_393_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_394 {
    use super::*;
    use crate::{Number, Value, Map};
    #[test]
    fn test_eq() {
        let _rug_st_tests_rug_394_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let rug_fuzz_2 = 42;
        let mut p0 = Value::Object(Map::new());
        p0[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
        let p1: i32 = rug_fuzz_2;
        p0.eq(&p1);
        let _rug_ed_tests_rug_394_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_rug_395 {
    use super::*;
    use crate::value::Value;
    #[test]
    fn test_eq() {
        let _rug_st_tests_rug_395_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let rug_fuzz_2 = 42;
        let mut p0 = {
            use crate::{Number, Value, Map};
            let mut v29 = Value::Object(Map::new());
            v29[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
            v29
        };
        let mut p1: i64 = rug_fuzz_2;
        <Value as std::cmp::PartialEq<i64>>::eq(&p0, &p1);
        let _rug_ed_tests_rug_395_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_rug_396 {
    use super::*;
    use crate::{Number, Value, Map};
    #[test]
    fn test_eq() {
        let _rug_st_tests_rug_396_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = "key";
        let rug_fuzz_2 = 42;
        let mut p0: i64 = rug_fuzz_0;
        let mut v29 = Value::Object(Map::new());
        v29[rug_fuzz_1] = Value::Number(Number::from(rug_fuzz_2));
        let mut p1 = &v29;
        <i64>::eq(&p0, p1);
        let _rug_ed_tests_rug_396_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_rug_397 {
    use super::*;
    use crate::{Value, Number, Map};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_397_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let rug_fuzz_2 = 42;
        let mut v29 = Value::Object(Map::new());
        v29[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
        let p0: &Value = &v29;
        let p1: &i64 = &rug_fuzz_2;
        p0.eq(p1);
        let _rug_ed_tests_rug_397_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_398 {
    use super::*;
    use crate::Value;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_398_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let rug_fuzz_1 = 42;
        let mut p0 = Value::Number(rug_fuzz_0.into());
        let p1 = rug_fuzz_1;
        p0.eq(&p1);
        let _rug_ed_tests_rug_398_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_399_prepare {
    use crate::{Number, Value, Map};
    #[test]
    fn sample() {
        let _rug_st_tests_rug_399_prepare_rrrruuuugggg_sample = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let mut v29 = Value::Object(Map::new());
        v29[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
        let _rug_ed_tests_rug_399_prepare_rrrruuuugggg_sample = 0;
    }
}
#[cfg(test)]
mod tests_rug_399 {
    use super::*;
    use std::cmp::PartialEq;
    use crate::{Number, Value, Map};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_399_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let rug_fuzz_2 = 42;
        let mut p0 = Value::Object(Map::new());
        p0[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
        let p1: isize = rug_fuzz_2;
        <Value as PartialEq<isize>>::eq(&p0, &p1);
        let _rug_ed_tests_rug_399_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_400 {
    use super::*;
    use crate::{Value, Number, Map};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_400_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = "key";
        let rug_fuzz_2 = 42;
        let p0: isize = rug_fuzz_0;
        let mut v29 = Value::Object(Map::new());
        v29[rug_fuzz_1] = Value::Number(Number::from(rug_fuzz_2));
        let p1: Value = v29;
        <isize>::eq(&p0, &p1);
        let _rug_ed_tests_rug_400_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_403 {
    use super::*;
    use crate::{Number, Value, Map};
    #[test]
    fn test_eq() {
        let _rug_st_tests_rug_403_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let rug_fuzz_2 = 42;
        let mut v29 = Value::Object(Map::new());
        v29[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
        let p1: u8 = rug_fuzz_2;
        let result = v29.eq(&p1);
        let _rug_ed_tests_rug_403_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_rug_404 {
    use super::*;
    use crate::{Number, Value, Map};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_404_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = "key";
        let rug_fuzz_2 = 42;
        let mut p0: u8 = rug_fuzz_0;
        let mut v29 = Value::Object(Map::new());
        v29[rug_fuzz_1] = Value::Number(Number::from(rug_fuzz_2));
        let p1: &Value = &v29;
        <u8>::eq(&p0, p1);
        let _rug_ed_tests_rug_404_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_405 {
    use super::*;
    use crate::Value;
    use crate::Number;
    use crate::Map;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_405_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let rug_fuzz_2 = 42;
        let mut v29 = Value::Object(Map::new());
        v29[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
        let p0: &Value = &v29;
        let p1: &u8 = &rug_fuzz_2;
        p0.eq(p1);
        let _rug_ed_tests_rug_405_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_408 {
    use super::*;
    use crate::{Number, Value, Map};
    #[test]
    fn test_eq() {
        let _rug_st_tests_rug_408_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = 42;
        let rug_fuzz_1 = "key";
        let rug_fuzz_2 = 42;
        let p0: u16 = rug_fuzz_0;
        let mut v29 = Value::Object(Map::new());
        v29[rug_fuzz_1] = Value::Number(Number::from(rug_fuzz_2));
        let p1 = &v29;
        debug_assert_eq!(p0.eq(p1), true);
        let _rug_ed_tests_rug_408_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_rug_410 {
    use super::*;
    use crate::{Number, Value, Map};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_410_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let rug_fuzz_2 = 10;
        let mut p0 = {
            let mut v29 = Value::Object(Map::new());
            v29[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
            v29
        };
        let p1: u16 = rug_fuzz_2;
        p0.eq(&p1);
        let _rug_ed_tests_rug_410_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_412 {
    use super::*;
    use crate::{Number, Value, Map};
    #[test]
    fn test_eq() {
        let _rug_st_tests_rug_412_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = 42;
        let rug_fuzz_1 = "key";
        let rug_fuzz_2 = 42;
        let mut p0: u32 = rug_fuzz_0;
        let mut p1 = {
            let mut v29 = Value::Object(Map::new());
            v29[rug_fuzz_1] = Value::Number(Number::from(rug_fuzz_2));
            v29
        };
        p0.eq(&p1);
        let _rug_ed_tests_rug_412_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_rug_413 {
    use super::*;
    use crate::{Number, Value, Map};
    #[test]
    fn test_eq() {
        let _rug_st_tests_rug_413_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let rug_fuzz_2 = 42;
        let mut v = Value::Object(Map::new());
        v[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
        let p0: &Value = &v;
        let p1: &u32 = &rug_fuzz_2;
        p0.eq(p1);
        let _rug_ed_tests_rug_413_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_rug_416 {
    use super::*;
    use crate::{Number, Value, Map};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_416_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let rug_fuzz_1 = "key";
        let rug_fuzz_2 = 42;
        let p0: u64 = rug_fuzz_0;
        let mut v29 = Value::Object(Map::new());
        v29[rug_fuzz_1] = Value::Number(Number::from(rug_fuzz_2));
        let p1 = &v29;
        <u64>::eq(&p0, p1);
        let _rug_ed_tests_rug_416_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_417 {
    use super::*;
    use crate::Value;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_417_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let rug_fuzz_1 = 42;
        let mut p0 = Value::Number(rug_fuzz_0.into());
        let p1: u64 = rug_fuzz_1;
        p0.eq(&p1);
        let _rug_ed_tests_rug_417_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_418 {
    use super::*;
    use crate::{Number, Value, Map};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_418_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let rug_fuzz_2 = 42;
        let mut v29 = Value::Object(Map::new());
        v29[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
        let p0: &mut Value = &mut v29;
        let p1: &u64 = &rug_fuzz_2;
        p0.eq(p1);
        let _rug_ed_tests_rug_418_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_419 {
    use super::*;
    use crate::Value;
    #[test]
    fn test_eq() {
        let _rug_st_tests_rug_419_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let rug_fuzz_2 = 42usize;
        let mut p0 = {
            use crate::{Number, Value, Map};
            let mut v29 = Value::Object(Map::new());
            v29[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
            v29
        };
        let mut p1 = rug_fuzz_2;
        <Value as std::cmp::PartialEq<usize>>::eq(&p0, &p1);
        let _rug_ed_tests_rug_419_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_rug_420 {
    use super::*;
    use crate::{Number, Value, Map};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_420_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = "key";
        let rug_fuzz_2 = 42;
        let mut p0: usize = rug_fuzz_0;
        let mut v29 = Value::Object(Map::new());
        v29[rug_fuzz_1] = Value::Number(Number::from(rug_fuzz_2));
        let mut p1: Value = v29;
        <usize>::eq(&p0, &p1);
        let _rug_ed_tests_rug_420_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_423 {
    use super::*;
    use crate::{Number, Value, Map};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_423_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let rug_fuzz_2 = 3.14;
        let mut p0 = {
            let mut v29 = Value::Object(Map::new());
            v29[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
            v29
        };
        let mut p1: f32 = rug_fuzz_2;
        <Value as PartialEq<f32>>::eq(&p0, &p1);
        let _rug_ed_tests_rug_423_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_424 {
    use super::*;
    use crate::{Number, Value, Map};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_424_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3.14;
        let rug_fuzz_1 = "key";
        let rug_fuzz_2 = 42;
        let p0: f32 = rug_fuzz_0;
        let mut v29 = Value::Object(Map::new());
        v29[rug_fuzz_1] = Value::Number(Number::from(rug_fuzz_2));
        let p1: &Value = &v29;
        <f32>::eq(&p0, p1);
        let _rug_ed_tests_rug_424_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_425 {
    use super::*;
    use crate::{Number, Value, Map};
    #[test]
    fn test_eq() {
        let _rug_st_tests_rug_425_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let rug_fuzz_2 = 42.0;
        let mut v = Value::Object(Map::new());
        v[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
        let p0: &Value = &v;
        let p1: &f32 = &rug_fuzz_2;
        p0.eq(p1);
        let _rug_ed_tests_rug_425_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_rug_426 {
    use super::*;
    use crate::{Number, Value, Map};
    #[test]
    fn test_eq() {
        let _rug_st_tests_rug_426_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let rug_fuzz_2 = 3.14;
        let mut p0 = {
            let mut v29 = Value::Object(Map::new());
            v29[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
            v29
        };
        let p1: f32 = rug_fuzz_2;
        p0.eq(&p1);
        let _rug_ed_tests_rug_426_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_rug_427 {
    use super::*;
    use crate::{Number, Value, Map};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_427_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let rug_fuzz_2 = 3.14;
        let mut p0 = {
            let mut v29 = Value::Object(Map::new());
            v29[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
            v29
        };
        let mut p1 = rug_fuzz_2;
        p0.eq(&p1);
        let _rug_ed_tests_rug_427_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_428 {
    use super::*;
    use crate::{Number, Value, Map};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_428_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3.14;
        let rug_fuzz_1 = "key";
        let rug_fuzz_2 = 42;
        let p0: f64 = rug_fuzz_0;
        let mut v29 = Value::Object(Map::new());
        v29[rug_fuzz_1] = Value::Number(Number::from(rug_fuzz_2));
        let p1: &Value = &v29;
        f64::eq(&p0, p1);
        let _rug_ed_tests_rug_428_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_429 {
    use super::*;
    use crate::{Number, Value, Map};
    #[test]
    fn test_eq() {
        let _rug_st_tests_rug_429_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let rug_fuzz_2 = 42.0;
        let mut v29 = Value::Object(Map::new());
        v29[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
        let p0: &Value = &v29;
        let p1: &f64 = &rug_fuzz_2;
        p0.eq(p1);
        let _rug_ed_tests_rug_429_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_rug_430 {
    use super::*;
    use crate::{Number, Value, Map};
    #[test]
    fn test_eq() {
        let _rug_st_tests_rug_430_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let rug_fuzz_2 = 42.0;
        let mut p0 = {
            let mut v29 = Value::Object(Map::new());
            v29[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
            v29
        };
        let p1 = rug_fuzz_2;
        p0.eq(&p1);
        let _rug_ed_tests_rug_430_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_rug_431 {
    use super::*;
    use crate::{Value, Number, Map};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_431_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let rug_fuzz_2 = false;
        let mut p0 = {
            let mut v29 = Value::Object(Map::new());
            v29[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
            v29
        };
        let p1: bool = rug_fuzz_2;
        <Value>::eq(&p0, &p1);
        let _rug_ed_tests_rug_431_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_432 {
    use super::*;
    use crate::{Number, Value, Map};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_432_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let rug_fuzz_2 = true;
        let mut v29 = Value::Object(Map::new());
        v29[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
        let mut p0: bool = rug_fuzz_2;
        let p1: &Value = &v29;
        <bool>::eq(&p0, p1);
        let _rug_ed_tests_rug_432_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_433 {
    use super::*;
    use crate::{Number, Value, Map};
    #[test]
    fn test_eq() {
        let _rug_st_tests_rug_433_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "key";
        let rug_fuzz_1 = 42;
        let rug_fuzz_2 = true;
        let mut p0 = Value::Object(Map::new());
        p0[rug_fuzz_0] = Value::Number(Number::from(rug_fuzz_1));
        let p1: bool = rug_fuzz_2;
        p0.eq(&p1);
        let _rug_ed_tests_rug_433_rrrruuuugggg_test_eq = 0;
    }
}
