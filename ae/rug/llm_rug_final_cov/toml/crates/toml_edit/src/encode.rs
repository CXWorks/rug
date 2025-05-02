use std::borrow::Cow;
use std::fmt::{Display, Formatter, Result, Write};
use toml_datetime::*;
use crate::document::Document;
use crate::inline_table::DEFAULT_INLINE_KEY_DECOR;
use crate::key::Key;
use crate::repr::{Formatted, Repr, ValueRepr};
use crate::table::{DEFAULT_KEY_DECOR, DEFAULT_KEY_PATH_DECOR, DEFAULT_TABLE_DECOR};
use crate::value::{
    DEFAULT_LEADING_VALUE_DECOR, DEFAULT_TRAILING_VALUE_DECOR, DEFAULT_VALUE_DECOR,
};
use crate::{Array, InlineTable, Item, Table, Value};
pub(crate) trait Encode {
    fn encode(
        &self,
        buf: &mut dyn Write,
        input: Option<&str>,
        default_decor: (&str, &str),
    ) -> Result;
}
impl Encode for Key {
    fn encode(
        &self,
        buf: &mut dyn Write,
        input: Option<&str>,
        default_decor: (&str, &str),
    ) -> Result {
        let decor = self.decor();
        decor.prefix_encode(buf, input, default_decor.0)?;
        if let Some(input) = input {
            let repr = self
                .as_repr()
                .map(Cow::Borrowed)
                .unwrap_or_else(|| Cow::Owned(self.default_repr()));
            repr.encode(buf, input)?;
        } else {
            let repr = self.display_repr();
            write!(buf, "{}", repr)?;
        };
        decor.suffix_encode(buf, input, default_decor.1)?;
        Ok(())
    }
}
impl<'k> Encode for &'k [Key] {
    fn encode(
        &self,
        buf: &mut dyn Write,
        input: Option<&str>,
        default_decor: (&str, &str),
    ) -> Result {
        for (i, key) in self.iter().enumerate() {
            let first = i == 0;
            let last = i + 1 == self.len();
            let prefix = if first { default_decor.0 } else { DEFAULT_KEY_PATH_DECOR.0 };
            let suffix = if last { default_decor.1 } else { DEFAULT_KEY_PATH_DECOR.1 };
            if !first {
                write!(buf, ".")?;
            }
            key.encode(buf, input, (prefix, suffix))?;
        }
        Ok(())
    }
}
impl<'k> Encode for &'k [&'k Key] {
    fn encode(
        &self,
        buf: &mut dyn Write,
        input: Option<&str>,
        default_decor: (&str, &str),
    ) -> Result {
        for (i, key) in self.iter().enumerate() {
            let first = i == 0;
            let last = i + 1 == self.len();
            let prefix = if first { default_decor.0 } else { DEFAULT_KEY_PATH_DECOR.0 };
            let suffix = if last { default_decor.1 } else { DEFAULT_KEY_PATH_DECOR.1 };
            if !first {
                write!(buf, ".")?;
            }
            key.encode(buf, input, (prefix, suffix))?;
        }
        Ok(())
    }
}
impl<T> Encode for Formatted<T>
where
    T: ValueRepr,
{
    fn encode(
        &self,
        buf: &mut dyn Write,
        input: Option<&str>,
        default_decor: (&str, &str),
    ) -> Result {
        let decor = self.decor();
        decor.prefix_encode(buf, input, default_decor.0)?;
        if let Some(input) = input {
            let repr = self
                .as_repr()
                .map(Cow::Borrowed)
                .unwrap_or_else(|| Cow::Owned(self.default_repr()));
            repr.encode(buf, input)?;
        } else {
            let repr = self.display_repr();
            write!(buf, "{}", repr)?;
        };
        decor.suffix_encode(buf, input, default_decor.1)?;
        Ok(())
    }
}
impl Encode for Array {
    fn encode(
        &self,
        buf: &mut dyn Write,
        input: Option<&str>,
        default_decor: (&str, &str),
    ) -> Result {
        let decor = self.decor();
        decor.prefix_encode(buf, input, default_decor.0)?;
        write!(buf, "[")?;
        for (i, elem) in self.iter().enumerate() {
            let inner_decor;
            if i == 0 {
                inner_decor = DEFAULT_LEADING_VALUE_DECOR;
            } else {
                inner_decor = DEFAULT_VALUE_DECOR;
                write!(buf, ",")?;
            }
            elem.encode(buf, input, inner_decor)?;
        }
        if self.trailing_comma() && !self.is_empty() {
            write!(buf, ",")?;
        }
        self.trailing().encode_with_default(buf, input, "")?;
        write!(buf, "]")?;
        decor.suffix_encode(buf, input, default_decor.1)?;
        Ok(())
    }
}
impl Encode for InlineTable {
    fn encode(
        &self,
        buf: &mut dyn Write,
        input: Option<&str>,
        default_decor: (&str, &str),
    ) -> Result {
        let decor = self.decor();
        decor.prefix_encode(buf, input, default_decor.0)?;
        write!(buf, "{{")?;
        self.preamble().encode_with_default(buf, input, "")?;
        let children = self.get_values();
        let len = children.len();
        for (i, (key_path, value)) in children.into_iter().enumerate() {
            if i != 0 {
                write!(buf, ",")?;
            }
            let inner_decor = if i == len - 1 {
                DEFAULT_TRAILING_VALUE_DECOR
            } else {
                DEFAULT_VALUE_DECOR
            };
            key_path.as_slice().encode(buf, input, DEFAULT_INLINE_KEY_DECOR)?;
            write!(buf, "=")?;
            value.encode(buf, input, inner_decor)?;
        }
        write!(buf, "}}")?;
        decor.suffix_encode(buf, input, default_decor.1)?;
        Ok(())
    }
}
impl Encode for Value {
    fn encode(
        &self,
        buf: &mut dyn Write,
        input: Option<&str>,
        default_decor: (&str, &str),
    ) -> Result {
        match self {
            Value::String(repr) => repr.encode(buf, input, default_decor),
            Value::Integer(repr) => repr.encode(buf, input, default_decor),
            Value::Float(repr) => repr.encode(buf, input, default_decor),
            Value::Boolean(repr) => repr.encode(buf, input, default_decor),
            Value::Datetime(repr) => repr.encode(buf, input, default_decor),
            Value::Array(array) => array.encode(buf, input, default_decor),
            Value::InlineTable(table) => table.encode(buf, input, default_decor),
        }
    }
}
impl Display for Document {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let mut path = Vec::new();
        let mut last_position = 0;
        let mut tables = Vec::new();
        visit_nested_tables(
                self.as_table(),
                &mut path,
                false,
                &mut |t, p, is_array| {
                    if let Some(pos) = t.position() {
                        last_position = pos;
                    }
                    tables.push((last_position, t, p.clone(), is_array));
                    Ok(())
                },
            )
            .unwrap();
        tables.sort_by_key(|&(id, _, _, _)| id);
        let mut first_table = true;
        for (_, table, path, is_array) in tables {
            visit_table(
                f,
                self.original.as_deref(),
                table,
                &path,
                is_array,
                &mut first_table,
            )?;
        }
        self.trailing().encode_with_default(f, self.original.as_deref(), "")
    }
}
fn visit_nested_tables<'t, F>(
    table: &'t Table,
    path: &mut Vec<Key>,
    is_array_of_tables: bool,
    callback: &mut F,
) -> Result
where
    F: FnMut(&'t Table, &Vec<Key>, bool) -> Result,
{
    if !table.is_dotted() {
        callback(table, path, is_array_of_tables)?;
    }
    for kv in table.items.values() {
        match kv.value {
            Item::Table(ref t) => {
                let mut key = kv.key.clone();
                if t.is_dotted() {
                    key.decor_mut().clear();
                }
                path.push(key);
                visit_nested_tables(t, path, false, callback)?;
                path.pop();
            }
            Item::ArrayOfTables(ref a) => {
                for t in a.iter() {
                    let key = kv.key.clone();
                    path.push(key);
                    visit_nested_tables(t, path, true, callback)?;
                    path.pop();
                }
            }
            _ => {}
        }
    }
    Ok(())
}
fn visit_table(
    buf: &mut dyn Write,
    input: Option<&str>,
    table: &Table,
    path: &[Key],
    is_array_of_tables: bool,
    first_table: &mut bool,
) -> Result {
    let children = table.get_values();
    let is_visible_std_table = !(table.implicit && children.is_empty());
    if path.is_empty() {
        if !children.is_empty() {
            *first_table = false;
        }
    } else if is_array_of_tables {
        let default_decor = if *first_table {
            *first_table = false;
            ("", DEFAULT_TABLE_DECOR.1)
        } else {
            DEFAULT_TABLE_DECOR
        };
        table.decor.prefix_encode(buf, input, default_decor.0)?;
        write!(buf, "[[")?;
        path.encode(buf, input, DEFAULT_KEY_PATH_DECOR)?;
        write!(buf, "]]")?;
        table.decor.suffix_encode(buf, input, default_decor.1)?;
        writeln!(buf)?;
    } else if is_visible_std_table {
        let default_decor = if *first_table {
            *first_table = false;
            ("", DEFAULT_TABLE_DECOR.1)
        } else {
            DEFAULT_TABLE_DECOR
        };
        table.decor.prefix_encode(buf, input, default_decor.0)?;
        write!(buf, "[")?;
        path.encode(buf, input, DEFAULT_KEY_PATH_DECOR)?;
        write!(buf, "]")?;
        table.decor.suffix_encode(buf, input, default_decor.1)?;
        writeln!(buf)?;
    }
    for (key_path, value) in children {
        key_path.as_slice().encode(buf, input, DEFAULT_KEY_DECOR)?;
        write!(buf, "=")?;
        value.encode(buf, input, DEFAULT_VALUE_DECOR)?;
        writeln!(buf)?;
    }
    Ok(())
}
impl ValueRepr for String {
    fn to_repr(&self) -> Repr {
        to_string_repr(self, None, None)
    }
}
pub(crate) fn to_string_repr(
    value: &str,
    style: Option<StringStyle>,
    literal: Option<bool>,
) -> Repr {
    let (style, literal) = match (style, literal) {
        (Some(style), Some(literal)) => (style, literal),
        (_, Some(literal)) => (infer_style(value).0, literal),
        (Some(style), _) => (style, infer_style(value).1),
        (_, _) => infer_style(value),
    };
    let mut output = String::with_capacity(value.len() * 2);
    if literal {
        output.push_str(style.literal_start());
        output.push_str(value);
        output.push_str(style.literal_end());
    } else {
        output.push_str(style.standard_start());
        for ch in value.chars() {
            match ch {
                '\u{8}' => output.push_str("\\b"),
                '\u{9}' => output.push_str("\\t"),
                '\u{a}' => {
                    match style {
                        StringStyle::NewlineTripple => output.push('\n'),
                        StringStyle::OnelineSingle => output.push_str("\\n"),
                        _ => unreachable!(),
                    }
                }
                '\u{c}' => output.push_str("\\f"),
                '\u{d}' => output.push_str("\\r"),
                '\u{22}' => output.push_str("\\\""),
                '\u{5c}' => output.push_str("\\\\"),
                c if c <= '\u{1f}' || c == '\u{7f}' => {
                    write!(output, "\\u{:04X}", ch as u32).unwrap();
                }
                ch => output.push(ch),
            }
        }
        output.push_str(style.standard_end());
    }
    Repr::new_unchecked(output)
}
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum StringStyle {
    NewlineTripple,
    OnelineTripple,
    OnelineSingle,
}
impl StringStyle {
    fn literal_start(self) -> &'static str {
        match self {
            Self::NewlineTripple => "'''\n",
            Self::OnelineTripple => "'''",
            Self::OnelineSingle => "'",
        }
    }
    fn literal_end(self) -> &'static str {
        match self {
            Self::NewlineTripple => "'''",
            Self::OnelineTripple => "'''",
            Self::OnelineSingle => "'",
        }
    }
    fn standard_start(self) -> &'static str {
        match self {
            Self::NewlineTripple => "\"\"\"\n",
            Self::OnelineTripple | Self::OnelineSingle => "\"",
        }
    }
    fn standard_end(self) -> &'static str {
        match self {
            Self::NewlineTripple => "\"\"\"",
            Self::OnelineTripple | Self::OnelineSingle => "\"",
        }
    }
}
fn infer_style(value: &str) -> (StringStyle, bool) {
    let mut out = String::with_capacity(value.len() * 2);
    let mut ty = StringStyle::OnelineSingle;
    let mut max_found_singles = 0;
    let mut found_singles = 0;
    let mut prefer_literal = false;
    let mut can_be_pretty = true;
    for ch in value.chars() {
        if can_be_pretty {
            if ch == '\'' {
                found_singles += 1;
                if found_singles >= 3 {
                    can_be_pretty = false;
                }
            } else {
                if found_singles > max_found_singles {
                    max_found_singles = found_singles;
                }
                found_singles = 0;
            }
            match ch {
                '\t' => {}
                '\\' => {
                    prefer_literal = true;
                }
                '\n' => ty = StringStyle::NewlineTripple,
                c if c <= '\u{1f}' || c == '\u{7f}' => can_be_pretty = false,
                _ => {}
            }
            out.push(ch);
        } else {
            if ch == '\n' {
                ty = StringStyle::NewlineTripple;
            }
        }
    }
    if found_singles > 0 && value.ends_with('\'') {
        can_be_pretty = false;
    }
    if !prefer_literal {
        can_be_pretty = false;
    }
    if !can_be_pretty {
        debug_assert!(ty != StringStyle::OnelineTripple);
        return (ty, false);
    }
    if found_singles > max_found_singles {
        max_found_singles = found_singles;
    }
    debug_assert!(max_found_singles < 3);
    if ty == StringStyle::OnelineSingle && max_found_singles >= 1 {
        ty = StringStyle::OnelineTripple;
    }
    (ty, true)
}
impl ValueRepr for i64 {
    fn to_repr(&self) -> Repr {
        Repr::new_unchecked(self.to_string())
    }
}
impl ValueRepr for f64 {
    fn to_repr(&self) -> Repr {
        to_f64_repr(*self)
    }
}
fn to_f64_repr(f: f64) -> Repr {
    let repr = match (f.is_sign_negative(), f.is_nan(), f == 0.0) {
        (true, true, _) => "-nan".to_owned(),
        (false, true, _) => "nan".to_owned(),
        (true, false, true) => "-0.0".to_owned(),
        (false, false, true) => "0.0".to_owned(),
        (_, false, false) => {
            if f % 1.0 == 0.0 { format!("{}.0", f) } else { format!("{}", f) }
        }
    };
    Repr::new_unchecked(repr)
}
impl ValueRepr for bool {
    fn to_repr(&self) -> Repr {
        Repr::new_unchecked(self.to_string())
    }
}
impl ValueRepr for Datetime {
    fn to_repr(&self) -> Repr {
        Repr::new_unchecked(self.to_string())
    }
}
#[cfg(test)]
mod tests_rug_427 {
    use super::*;
    use std::fmt::Write;
    use crate::{encode, Key, Table};
    #[test]
    fn test_encode() {
        let _rug_st_tests_rug_427_rrrruuuugggg_test_encode = 0;
        let rug_fuzz_0 = "input";
        let rug_fuzz_1 = "key1";
        let rug_fuzz_2 = true;
        let rug_fuzz_3 = false;
        let mut p0: &mut dyn Write = &mut String::new();
        let p1: Option<&str> = Some(rug_fuzz_0);
        let p2: Table = Default::default();
        let p3: Vec<Key> = vec![Key::from(rug_fuzz_1), Key::from("key2")];
        let p4: bool = rug_fuzz_2;
        let mut p5: bool = rug_fuzz_3;
        encode::visit_table(&mut p0, p1, &p2, &p3, p4, &mut p5);
        let _rug_ed_tests_rug_427_rrrruuuugggg_test_encode = 0;
    }
}
#[cfg(test)]
mod tests_rug_428 {
    use super::*;
    use crate::encode::{to_string_repr, Repr, StringStyle};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_428_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample_data";
        let rug_fuzz_1 = true;
        let mut p0 = rug_fuzz_0;
        let mut p1 = Some(StringStyle::OnelineSingle);
        let mut p2 = Some(rug_fuzz_1);
        to_string_repr(&p0, p1, p2);
        let _rug_ed_tests_rug_428_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_429 {
    use super::*;
    #[test]
    fn test_infer_style() {
        let _rug_st_tests_rug_429_rrrruuuugggg_test_infer_style = 0;
        let rug_fuzz_0 = "This is a sample string";
        let p0: &str = rug_fuzz_0;
        crate::encode::infer_style(&p0);
        let _rug_ed_tests_rug_429_rrrruuuugggg_test_infer_style = 0;
    }
}
#[cfg(test)]
mod tests_rug_430 {
    use super::*;
    use crate::encode::Repr;
    #[test]
    fn test_to_f64_repr() {
        let _rug_st_tests_rug_430_rrrruuuugggg_test_to_f64_repr = 0;
        let rug_fuzz_0 = 3.14;
        let mut p0: f64 = rug_fuzz_0;
        crate::encode::to_f64_repr(p0);
        let _rug_ed_tests_rug_430_rrrruuuugggg_test_to_f64_repr = 0;
    }
}
#[cfg(test)]
mod tests_rug_432 {
    use super::*;
    use std::fmt::Write;
    use crate::key::Key;
    use crate::encode::{Encode, DEFAULT_KEY_PATH_DECOR};
    #[test]
    fn test_encode() {
        let _rug_st_tests_rug_432_rrrruuuugggg_test_encode = 0;
        let rug_fuzz_0 = "key1";
        let rug_fuzz_1 = "key2";
        let rug_fuzz_2 = "key3";
        let mut buf = String::new();
        let input: Option<&str> = None;
        let default_decor = DEFAULT_KEY_PATH_DECOR;
        let keys: &[Key] = &[
            Key::from(rug_fuzz_0),
            Key::from(rug_fuzz_1),
            Key::from(rug_fuzz_2),
        ];
        keys.encode(&mut buf, input, default_decor).unwrap();
        debug_assert_eq!(buf, "key1.key2.key3");
        let _rug_ed_tests_rug_432_rrrruuuugggg_test_encode = 0;
    }
}
#[cfg(test)]
mod tests_rug_435 {
    use super::*;
    use crate::encode::Encode;
    use crate::array::Array;
    #[test]
    fn test_encode() {
        let _rug_st_tests_rug_435_rrrruuuugggg_test_encode = 0;
        let rug_fuzz_0 = "input";
        let rug_fuzz_1 = "prefix";
        let rug_fuzz_2 = "suffix";
        let mut p0: Array = Array::new();
        let mut p1: &mut dyn std::fmt::Write = &mut String::new();
        let p2: std::option::Option<&str> = Some(rug_fuzz_0);
        let p3: (&str, &str) = (rug_fuzz_1, rug_fuzz_2);
        p0.encode(p1, p2, p3).unwrap();
        let _rug_ed_tests_rug_435_rrrruuuugggg_test_encode = 0;
    }
}
#[cfg(test)]
mod tests_rug_438 {
    use super::*;
    use crate::repr::ValueRepr;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_438_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample string";
        let mut p0 = std::string::String::from(rug_fuzz_0);
        <std::string::String>::to_repr(&mut p0);
        let _rug_ed_tests_rug_438_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_439 {
    use super::*;
    use crate::encode::StringStyle;
    #[test]
    fn test_literal_start() {
        let _rug_st_tests_rug_439_rrrruuuugggg_test_literal_start = 0;
        let p0 = StringStyle::NewlineTripple;
        debug_assert_eq!(p0.literal_start(), "'''\n");
        let p1 = StringStyle::OnelineTripple;
        debug_assert_eq!(p1.literal_start(), "'''");
        let p2 = StringStyle::OnelineSingle;
        debug_assert_eq!(p2.literal_start(), "'");
        let _rug_ed_tests_rug_439_rrrruuuugggg_test_literal_start = 0;
    }
}
#[cfg(test)]
mod tests_rug_440 {
    use super::*;
    use crate::encode::StringStyle::{
        self, NewlineTripple, OnelineTripple, OnelineSingle,
    };
    #[test]
    fn test_literal_end() {
        let _rug_st_tests_rug_440_rrrruuuugggg_test_literal_end = 0;
        let p0: StringStyle = NewlineTripple;
        let result = p0.literal_end();
        debug_assert_eq!(result, "'''");
        let _rug_ed_tests_rug_440_rrrruuuugggg_test_literal_end = 0;
    }
}
#[cfg(test)]
mod tests_rug_441 {
    use super::*;
    use crate::encode::StringStyle;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_441_rrrruuuugggg_test_rug = 0;
        let p0: StringStyle = StringStyle::NewlineTripple;
        StringStyle::standard_start(p0);
        let _rug_ed_tests_rug_441_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_442 {
    use super::*;
    use crate::encode::StringStyle;
    #[test]
    fn test_standard_end() {
        let _rug_st_tests_rug_442_rrrruuuugggg_test_standard_end = 0;
        let p0: StringStyle = StringStyle::NewlineTripple;
        debug_assert_eq!(p0.standard_end(), "\"\"\"");
        let _rug_ed_tests_rug_442_rrrruuuugggg_test_standard_end = 0;
    }
}
#[cfg(test)]
mod tests_rug_443 {
    use super::*;
    use crate::repr::ValueRepr;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_443_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i64 = rug_fuzz_0;
        <i64 as ValueRepr>::to_repr(&p0);
        let _rug_ed_tests_rug_443_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_444 {
    use super::*;
    use crate::repr::ValueRepr;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_444_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3.14;
        let mut p0: f64 = rug_fuzz_0;
        p0.to_repr();
        let _rug_ed_tests_rug_444_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_445 {
    use super::*;
    use crate::repr::ValueRepr;
    #[test]
    fn test_to_repr() {
        let _rug_st_tests_rug_445_rrrruuuugggg_test_to_repr = 0;
        let rug_fuzz_0 = true;
        let mut p0: bool = rug_fuzz_0;
        p0.to_repr();
        let _rug_ed_tests_rug_445_rrrruuuugggg_test_to_repr = 0;
    }
}
