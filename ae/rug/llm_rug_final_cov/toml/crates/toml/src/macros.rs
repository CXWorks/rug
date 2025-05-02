pub use serde::de::{Deserialize, IntoDeserializer};
use crate::value::{Array, Table, Value};
/// Construct a [`Table`] from TOML syntax.
///
/// ```rust
/// let cargo_toml = toml::toml! {
///     [package]
///     name = "toml"
///     version = "0.4.5"
///     authors = ["Alex Crichton <alex@alexcrichton.com>"]
///
///     [badges]
///     travis-ci = { repository = "alexcrichton/toml-rs" }
///
///     [dependencies]
///     serde = "1.0"
///
///     [dev-dependencies]
///     serde_derive = "1.0"
///     serde_json = "1.0"
/// };
///
/// println!("{:#?}", cargo_toml);
/// ```
#[macro_export]
macro_rules! toml {
    ($($toml:tt)+) => {
        { let table = $crate ::value::Table::new(); let mut root = $crate
        ::Value::Table(table); $crate ::toml_internal!(@ toplevel root[] $($toml)+);
        match root { $crate ::Value::Table(table) => table, _ => unreachable!(), } }
    };
}
#[macro_export]
#[doc(hidden)]
macro_rules! toml_internal {
    (@ toplevel $root:ident [$($path:tt)*]) => {};
    (@ toplevel $root:ident [$($path:tt)*] $($($k:tt)-+).+ = - $v:tt $($rest:tt)*) => {
        $crate ::toml_internal!(@ toplevel $root [$($path)*] $($($k)-+).+ = (-$v)
        $($rest)*);
    };
    (@ toplevel $root:ident [$($path:tt)*] $($($k:tt)-+).+ = + $v:tt $($rest:tt)*) => {
        $crate ::toml_internal!(@ toplevel $root [$($path)*] $($($k)-+).+ = ($v)
        $($rest)*);
    };
    (
        @ toplevel $root:ident [$($path:tt)*] $($($k:tt)-+).+ = $yr:tt - $mo:tt - $dhr:tt
        : $min:tt : $sec:tt . $frac:tt - $tzh:tt : $tzm:tt $($rest:tt)*
    ) => {
        $crate ::toml_internal!(@ topleveldatetime $root [$($path)*] $($($k)-+).+ = ($yr
        - $mo - $dhr : $min : $sec . $frac - $tzh : $tzm) $($rest)*);
    };
    (
        @ toplevel $root:ident [$($path:tt)*] $($($k:tt)-+).+ = $yr:tt - $mo:tt - $day:tt
        $hr:tt : $min:tt : $sec:tt . $frac:tt - $tzh:tt : $tzm:tt $($rest:tt)*
    ) => {
        $crate ::toml_internal!(@ topleveldatetime $root [$($path)*] $($($k)-+).+ = ($yr
        - $mo - $day T $hr : $min : $sec . $frac - $tzh : $tzm) $($rest)*);
    };
    (
        @ toplevel $root:ident [$($path:tt)*] $($($k:tt)-+).+ = $yr:tt - $mo:tt - $dhr:tt
        : $min:tt : $sec:tt - $tzh:tt : $tzm:tt $($rest:tt)*
    ) => {
        $crate ::toml_internal!(@ topleveldatetime $root [$($path)*] $($($k)-+).+ = ($yr
        - $mo - $dhr : $min : $sec - $tzh : $tzm) $($rest)*);
    };
    (
        @ toplevel $root:ident [$($path:tt)*] $($($k:tt)-+).+ = $yr:tt - $mo:tt - $day:tt
        $hr:tt : $min:tt : $sec:tt - $tzh:tt : $tzm:tt $($rest:tt)*
    ) => {
        $crate ::toml_internal!(@ topleveldatetime $root [$($path)*] $($($k)-+).+ = ($yr
        - $mo - $day T $hr : $min : $sec - $tzh : $tzm) $($rest)*);
    };
    (
        @ toplevel $root:ident [$($path:tt)*] $($($k:tt)-+).+ = $yr:tt - $mo:tt - $dhr:tt
        : $min:tt : $sec:tt . $frac:tt $($rest:tt)*
    ) => {
        $crate ::toml_internal!(@ topleveldatetime $root [$($path)*] $($($k)-+).+ = ($yr
        - $mo - $dhr : $min : $sec . $frac) $($rest)*);
    };
    (
        @ toplevel $root:ident [$($path:tt)*] $($($k:tt)-+).+ = $yr:tt - $mo:tt - $day:tt
        $hr:tt : $min:tt : $sec:tt . $frac:tt $($rest:tt)*
    ) => {
        $crate ::toml_internal!(@ topleveldatetime $root [$($path)*] $($($k)-+).+ = ($yr
        - $mo - $day T $hr : $min : $sec . $frac) $($rest)*);
    };
    (
        @ toplevel $root:ident [$($path:tt)*] $($($k:tt)-+).+ = $yr:tt - $mo:tt - $dhr:tt
        : $min:tt : $sec:tt $($rest:tt)*
    ) => {
        $crate ::toml_internal!(@ topleveldatetime $root [$($path)*] $($($k)-+).+ = ($yr
        - $mo - $dhr : $min : $sec) $($rest)*);
    };
    (
        @ toplevel $root:ident [$($path:tt)*] $($($k:tt)-+).+ = $yr:tt - $mo:tt - $day:tt
        $hr:tt : $min:tt : $sec:tt $($rest:tt)*
    ) => {
        $crate ::toml_internal!(@ topleveldatetime $root [$($path)*] $($($k)-+).+ = ($yr
        - $mo - $day T $hr : $min : $sec) $($rest)*);
    };
    (
        @ toplevel $root:ident [$($path:tt)*] $($($k:tt)-+).+ = $yr:tt - $mo:tt - $day:tt
        $($rest:tt)*
    ) => {
        $crate ::toml_internal!(@ topleveldatetime $root [$($path)*] $($($k)-+).+ = ($yr
        - $mo - $day) $($rest)*);
    };
    (
        @ toplevel $root:ident [$($path:tt)*] $($($k:tt)-+).+ = $hr:tt : $min:tt :
        $sec:tt . $frac:tt $($rest:tt)*
    ) => {
        $crate ::toml_internal!(@ topleveldatetime $root [$($path)*] $($($k)-+).+ = ($hr
        : $min : $sec . $frac) $($rest)*);
    };
    (
        @ toplevel $root:ident [$($path:tt)*] $($($k:tt)-+).+ = $hr:tt : $min:tt :
        $sec:tt $($rest:tt)*
    ) => {
        $crate ::toml_internal!(@ topleveldatetime $root [$($path)*] $($($k)-+).+ = ($hr
        : $min : $sec) $($rest)*);
    };
    (@ toplevel $root:ident [$($path:tt)*] $($($k:tt)-+).+ = $v:tt $($rest:tt)*) => {
        { $crate ::macros::insert_toml(& mut $root, & [$($path)* $(& concat!($("-",
        $crate ::toml_internal!(@ path $k),)+) [1..],)+], $crate ::toml_internal!(@ value
        $v)); $crate ::toml_internal!(@ toplevel $root [$($path)*] $($rest)*); }
    };
    (@ toplevel $root:ident $oldpath:tt [[$($($path:tt)-+).+]] $($rest:tt)*) => {
        $crate ::macros::push_toml(& mut $root, & [$(& concat!($("-", $crate
        ::toml_internal!(@ path $path),)+) [1..],)+]); $crate ::toml_internal!(@ toplevel
        $root [$(& concat!($("-", $crate ::toml_internal!(@ path $path),)+) [1..],)+]
        $($rest)*);
    };
    (@ toplevel $root:ident $oldpath:tt [$($($path:tt)-+).+] $($rest:tt)*) => {
        $crate ::macros::insert_toml(& mut $root, & [$(& concat!($("-", $crate
        ::toml_internal!(@ path $path),)+) [1..],)+], $crate ::Value::Table($crate
        ::value::Table::new())); $crate ::toml_internal!(@ toplevel $root [$(&
        concat!($("-", $crate ::toml_internal!(@ path $path),)+) [1..],)+] $($rest)*);
    };
    (
        @ topleveldatetime $root:ident [$($path:tt)*] $($($k:tt)-+).+ =
        ($($datetime:tt)+) $($rest:tt)*
    ) => {
        $crate ::macros::insert_toml(& mut $root, & [$($path)* $(& concat!($("-", $crate
        ::toml_internal!(@ path $k),)+) [1..],)+], $crate
        ::Value::Datetime(concat!($(stringify!($datetime)),+) .parse().unwrap())); $crate
        ::toml_internal!(@ toplevel $root [$($path)*] $($rest)*);
    };
    (@ path $ident:ident) => {
        stringify!($ident)
    };
    (@ path $quoted:tt) => {
        $quoted
    };
    (@ value { $($inline:tt)* }) => {
        { let mut table = $crate ::Value::Table($crate ::value::Table::new()); $crate
        ::toml_internal!(@ trailingcomma(@ table table) $($inline)*); table }
    };
    (@ value[$($inline:tt)*]) => {
        { let mut array = $crate ::value::Array::new(); $crate ::toml_internal!(@
        trailingcomma(@ array array) $($inline)*); $crate ::Value::Array(array) }
    };
    (@ value(- nan)) => {
        $crate ::Value::Float(-::std::f64::NAN)
    };
    (@ value(nan)) => {
        $crate ::Value::Float(::std::f64::NAN)
    };
    (@ value nan) => {
        $crate ::Value::Float(::std::f64::NAN)
    };
    (@ value(- inf)) => {
        $crate ::Value::Float(::std::f64::NEG_INFINITY)
    };
    (@ value(inf)) => {
        $crate ::Value::Float(::std::f64::INFINITY)
    };
    (@ value inf) => {
        $crate ::Value::Float(::std::f64::INFINITY)
    };
    (@ value $v:tt) => {
        { let de = $crate ::macros::IntoDeserializer::<$crate ::de::Error
        >::into_deserializer($v); <$crate ::Value as $crate ::macros::Deserialize
        >::deserialize(de).unwrap() }
    };
    (@ table $root:ident) => {};
    (@ table $root:ident $($($k:tt)-+).+ = - $v:tt, $($rest:tt)*) => {
        $crate ::toml_internal!(@ table $root $($($k)-+).+ = (-$v), $($rest)*);
    };
    (@ table $root:ident $($($k:tt)-+).+ = + $v:tt, $($rest:tt)*) => {
        $crate ::toml_internal!(@ table $root $($($k)-+).+ = ($v), $($rest)*);
    };
    (
        @ table $root:ident $($($k:tt)-+).+ = $yr:tt - $mo:tt - $dhr:tt : $min:tt :
        $sec:tt . $frac:tt - $tzh:tt : $tzm:tt, $($rest:tt)*
    ) => {
        $crate ::toml_internal!(@ tabledatetime $root $($($k)-+).+ = ($yr - $mo - $dhr :
        $min : $sec . $frac - $tzh : $tzm) $($rest)*);
    };
    (
        @ table $root:ident $($($k:tt)-+).+ = $yr:tt - $mo:tt - $day:tt $hr:tt : $min:tt
        : $sec:tt . $frac:tt - $tzh:tt : $tzm:tt, $($rest:tt)*
    ) => {
        $crate ::toml_internal!(@ tabledatetime $root $($($k)-+).+ = ($yr - $mo - $day T
        $hr : $min : $sec . $frac - $tzh : $tzm) $($rest)*);
    };
    (
        @ table $root:ident $($($k:tt)-+).+ = $yr:tt - $mo:tt - $dhr:tt : $min:tt :
        $sec:tt - $tzh:tt : $tzm:tt, $($rest:tt)*
    ) => {
        $crate ::toml_internal!(@ tabledatetime $root $($($k)-+).+ = ($yr - $mo - $dhr :
        $min : $sec - $tzh : $tzm) $($rest)*);
    };
    (
        @ table $root:ident $($($k:tt)-+).+ = $yr:tt - $mo:tt - $day:tt $hr:tt : $min:tt
        : $sec:tt - $tzh:tt : $tzm:tt, $($rest:tt)*
    ) => {
        $crate ::toml_internal!(@ tabledatetime $root $($($k)-+).+ = ($yr - $mo - $day T
        $hr : $min : $sec - $tzh : $tzm) $($rest)*);
    };
    (
        @ table $root:ident $($($k:tt)-+).+ = $yr:tt - $mo:tt - $dhr:tt : $min:tt :
        $sec:tt . $frac:tt, $($rest:tt)*
    ) => {
        $crate ::toml_internal!(@ tabledatetime $root $($($k)-+).+ = ($yr - $mo - $dhr :
        $min : $sec . $frac) $($rest)*);
    };
    (
        @ table $root:ident $($($k:tt)-+).+ = $yr:tt - $mo:tt - $day:tt $hr:tt : $min:tt
        : $sec:tt . $frac:tt, $($rest:tt)*
    ) => {
        $crate ::toml_internal!(@ tabledatetime $root $($($k)-+).+ = ($yr - $mo - $day T
        $hr : $min : $sec . $frac) $($rest)*);
    };
    (
        @ table $root:ident $($($k:tt)-+).+ = $yr:tt - $mo:tt - $dhr:tt : $min:tt :
        $sec:tt, $($rest:tt)*
    ) => {
        $crate ::toml_internal!(@ tabledatetime $root $($($k)-+).+ = ($yr - $mo - $dhr :
        $min : $sec) $($rest)*);
    };
    (
        @ table $root:ident $($($k:tt)-+).+ = $yr:tt - $mo:tt - $day:tt $hr:tt : $min:tt
        : $sec:tt, $($rest:tt)*
    ) => {
        $crate ::toml_internal!(@ tabledatetime $root $($($k)-+).+ = ($yr - $mo - $day T
        $hr : $min : $sec) $($rest)*);
    };
    (@ table $root:ident $($($k:tt)-+).+ = $yr:tt - $mo:tt - $day:tt, $($rest:tt)*) => {
        $crate ::toml_internal!(@ tabledatetime $root $($($k)-+).+ = ($yr - $mo - $day)
        $($rest)*);
    };
    (
        @ table $root:ident $($($k:tt)-+).+ = $hr:tt : $min:tt : $sec:tt . $frac:tt,
        $($rest:tt)*
    ) => {
        $crate ::toml_internal!(@ tabledatetime $root $($($k)-+).+ = ($hr : $min : $sec .
        $frac) $($rest)*);
    };
    (@ table $root:ident $($($k:tt)-+).+ = $hr:tt : $min:tt : $sec:tt, $($rest:tt)*) => {
        $crate ::toml_internal!(@ tabledatetime $root $($($k)-+).+ = ($hr : $min : $sec)
        $($rest)*);
    };
    (@ table $root:ident $($($k:tt)-+).+ = $v:tt, $($rest:tt)*) => {
        $crate ::macros::insert_toml(& mut $root, & [$(& concat!($("-", $crate
        ::toml_internal!(@ path $k),)+) [1..],)+], $crate ::toml_internal!(@ value $v));
        $crate ::toml_internal!(@ table $root $($rest)*);
    };
    (@ tabledatetime $root:ident $($($k:tt)-+).+ = ($($datetime:tt)*) $($rest:tt)*) => {
        $crate ::macros::insert_toml(& mut $root, & [$(& concat!($("-", $crate
        ::toml_internal!(@ path $k),)+) [1..],)+], $crate
        ::Value::Datetime(concat!($(stringify!($datetime)),+) .parse().unwrap())); $crate
        ::toml_internal!(@ table $root $($rest)*);
    };
    (@ array $root:ident) => {};
    (@ array $root:ident - $v:tt, $($rest:tt)*) => {
        $crate ::toml_internal!(@ array $root (-$v), $($rest)*);
    };
    (@ array $root:ident + $v:tt, $($rest:tt)*) => {
        $crate ::toml_internal!(@ array $root ($v), $($rest)*);
    };
    (
        @ array $root:ident $yr:tt - $mo:tt - $dhr:tt : $min:tt : $sec:tt . $frac:tt -
        $tzh:tt : $tzm:tt, $($rest:tt)*
    ) => {
        $crate ::toml_internal!(@ arraydatetime $root ($yr - $mo - $dhr : $min : $sec .
        $frac - $tzh : $tzm) $($rest)*);
    };
    (
        @ array $root:ident $yr:tt - $mo:tt - $day:tt $hr:tt : $min:tt : $sec:tt .
        $frac:tt - $tzh:tt : $tzm:tt, $($rest:tt)*
    ) => {
        $crate ::toml_internal!(@ arraydatetime $root ($yr - $mo - $day T $hr : $min :
        $sec . $frac - $tzh : $tzm) $($rest)*);
    };
    (
        @ array $root:ident $yr:tt - $mo:tt - $dhr:tt : $min:tt : $sec:tt - $tzh:tt :
        $tzm:tt, $($rest:tt)*
    ) => {
        $crate ::toml_internal!(@ arraydatetime $root ($yr - $mo - $dhr : $min : $sec -
        $tzh : $tzm) $($rest)*);
    };
    (
        @ array $root:ident $yr:tt - $mo:tt - $day:tt $hr:tt : $min:tt : $sec:tt -
        $tzh:tt : $tzm:tt, $($rest:tt)*
    ) => {
        $crate ::toml_internal!(@ arraydatetime $root ($yr - $mo - $day T $hr : $min :
        $sec - $tzh : $tzm) $($rest)*);
    };
    (
        @ array $root:ident $yr:tt - $mo:tt - $dhr:tt : $min:tt : $sec:tt . $frac:tt,
        $($rest:tt)*
    ) => {
        $crate ::toml_internal!(@ arraydatetime $root ($yr - $mo - $dhr : $min : $sec .
        $frac) $($rest)*);
    };
    (
        @ array $root:ident $yr:tt - $mo:tt - $day:tt $hr:tt : $min:tt : $sec:tt .
        $frac:tt, $($rest:tt)*
    ) => {
        $crate ::toml_internal!(@ arraydatetime $root ($yr - $mo - $day T $hr : $min :
        $sec . $frac) $($rest)*);
    };
    (
        @ array $root:ident $yr:tt - $mo:tt - $dhr:tt : $min:tt : $sec:tt, $($rest:tt)*
    ) => {
        $crate ::toml_internal!(@ arraydatetime $root ($yr - $mo - $dhr : $min : $sec)
        $($rest)*);
    };
    (
        @ array $root:ident $yr:tt - $mo:tt - $day:tt $hr:tt : $min:tt : $sec:tt,
        $($rest:tt)*
    ) => {
        $crate ::toml_internal!(@ arraydatetime $root ($yr - $mo - $day T $hr : $min :
        $sec) $($rest)*);
    };
    (@ array $root:ident $yr:tt - $mo:tt - $day:tt, $($rest:tt)*) => {
        $crate ::toml_internal!(@ arraydatetime $root ($yr - $mo - $day) $($rest)*);
    };
    (@ array $root:ident $hr:tt : $min:tt : $sec:tt . $frac:tt, $($rest:tt)*) => {
        $crate ::toml_internal!(@ arraydatetime $root ($hr : $min : $sec . $frac)
        $($rest)*);
    };
    (@ array $root:ident $hr:tt : $min:tt : $sec:tt, $($rest:tt)*) => {
        $crate ::toml_internal!(@ arraydatetime $root ($hr : $min : $sec) $($rest)*);
    };
    (@ array $root:ident $v:tt, $($rest:tt)*) => {
        $root .push($crate ::toml_internal!(@ value $v)); $crate ::toml_internal!(@ array
        $root $($rest)*);
    };
    (@ arraydatetime $root:ident ($($datetime:tt)*) $($rest:tt)*) => {
        $root .push($crate ::Value::Datetime(concat!($(stringify!($datetime)),+) .parse()
        .unwrap())); $crate ::toml_internal!(@ array $root $($rest)*);
    };
    (@ trailingcomma($($args:tt)*)) => {
        $crate ::toml_internal!($($args)*);
    };
    (@ trailingcomma($($args:tt)*),) => {
        $crate ::toml_internal!($($args)*,);
    };
    (@ trailingcomma($($args:tt)*) $last:tt) => {
        $crate ::toml_internal!($($args)* $last,);
    };
    (@ trailingcomma($($args:tt)*) $first:tt $($rest:tt)+) => {
        $crate ::toml_internal!(@ trailingcomma($($args)* $first) $($rest)+);
    };
}
pub fn insert_toml(root: &mut Value, path: &[&str], value: Value) {
    *traverse(root, path) = value;
}
pub fn push_toml(root: &mut Value, path: &[&str]) {
    let target = traverse(root, path);
    if !target.is_array() {
        *target = Value::Array(Array::new());
    }
    target.as_array_mut().unwrap().push(Value::Table(Table::new()));
}
fn traverse<'a>(root: &'a mut Value, path: &[&str]) -> &'a mut Value {
    let mut cur = root;
    for &key in path {
        let cur1 = cur;
        let cur2 = if cur1.is_array() {
            cur1.as_array_mut().unwrap().last_mut().unwrap()
        } else {
            cur1
        };
        if !cur2.is_table() {
            *cur2 = Value::Table(Table::new());
        }
        if !cur2.as_table().unwrap().contains_key(key) {
            let empty = Value::Table(Table::new());
            cur2.as_table_mut().unwrap().insert(key.to_owned(), empty);
        }
        cur = cur2.as_table_mut().unwrap().get_mut(key).unwrap();
    }
    cur
}
#[cfg(test)]
mod tests_rug_156 {
    use super::*;
    use crate::value::Value;
    #[test]
    fn test_insert_toml() {
        let _rug_st_tests_rug_156_rrrruuuugggg_test_insert_toml = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = "path1";
        let rug_fuzz_2 = "path2";
        let rug_fuzz_3 = "value";
        let mut p0 = Value::from(rug_fuzz_0);
        let p1 = [rug_fuzz_1, rug_fuzz_2];
        let p2 = Value::from(rug_fuzz_3);
        crate::macros::insert_toml(&mut p0, &p1, p2);
        let _rug_ed_tests_rug_156_rrrruuuugggg_test_insert_toml = 0;
    }
}
#[cfg(test)]
mod tests_rug_157 {
    use super::*;
    use crate::{value::Table, Value};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_157_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "key1";
        let rug_fuzz_1 = "key2";
        let rug_fuzz_2 = "key3";
        let mut p0 = Value::Table(Table::new());
        let p1 = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        crate::macros::push_toml(&mut p0, p1);
        let _rug_ed_tests_rug_157_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_158 {
    use super::*;
    use crate::{Table, Value};
    #[test]
    fn test_traverse() {
        let _rug_st_tests_rug_158_rrrruuuugggg_test_traverse = 0;
        let rug_fuzz_0 = "foo";
        let rug_fuzz_1 = "bar";
        let rug_fuzz_2 = "baz";
        let mut p0 = Value::Table(Table::new());
        let p1: &[&str] = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        crate::macros::traverse(&mut p0, p1);
        let _rug_ed_tests_rug_158_rrrruuuugggg_test_traverse = 0;
    }
}
