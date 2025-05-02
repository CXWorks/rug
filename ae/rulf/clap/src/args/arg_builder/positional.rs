use std::borrow::Cow;
use std::ffi::{OsStr, OsString};
use std::fmt::{Display, Formatter, Result};
use std::mem;
use std::rc::Rc;
use std::result::Result as StdResult;
use args::{AnyArg, ArgSettings, Base, DispOrder, Valued};
use map::{self, VecMap};
use Arg;
use INTERNAL_ERROR_MSG;
#[allow(missing_debug_implementations)]
#[doc(hidden)]
#[derive(Clone, Default)]
pub struct PosBuilder<'n, 'e>
where
    'n: 'e,
{
    pub b: Base<'n, 'e>,
    pub v: Valued<'n, 'e>,
    pub index: u64,
}
impl<'n, 'e> PosBuilder<'n, 'e> {
    pub fn new(name: &'n str, idx: u64) -> Self {
        PosBuilder {
            b: Base::new(name),
            index: idx,
            ..Default::default()
        }
    }
    pub fn from_arg_ref(a: &Arg<'n, 'e>, idx: u64) -> Self {
        let mut pb = PosBuilder {
            b: Base::from(a),
            v: Valued::from(a),
            index: idx,
        };
        if a.v.max_vals.is_some() || a.v.min_vals.is_some()
            || (a.v.num_vals.is_some() && a.v.num_vals.unwrap() > 1)
        {
            pb.b.settings.set(ArgSettings::Multiple);
        }
        pb
    }
    pub fn from_arg(mut a: Arg<'n, 'e>, idx: u64) -> Self {
        if a.v.max_vals.is_some() || a.v.min_vals.is_some()
            || (a.v.num_vals.is_some() && a.v.num_vals.unwrap() > 1)
        {
            a.b.settings.set(ArgSettings::Multiple);
        }
        PosBuilder {
            b: mem::replace(&mut a.b, Base::default()),
            v: mem::replace(&mut a.v, Valued::default()),
            index: idx,
        }
    }
    pub fn multiple_str(&self) -> &str {
        let mult_vals = self.v.val_names.as_ref().map_or(true, |names| names.len() < 2);
        if self.is_set(ArgSettings::Multiple) && mult_vals { "..." } else { "" }
    }
    pub fn name_no_brackets(&self) -> Cow<str> {
        debugln!("PosBuilder::name_no_brackets;");
        let mut delim = String::new();
        delim
            .push(
                if self.is_set(ArgSettings::RequireDelimiter) {
                    self.v.val_delim.expect(INTERNAL_ERROR_MSG)
                } else {
                    ' '
                },
            );
        if let Some(ref names) = self.v.val_names {
            debugln!("PosBuilder:name_no_brackets: val_names={:#?}", names);
            if names.len() > 1 {
                Cow::Owned(
                    names
                        .values()
                        .map(|n| format!("<{}>", n))
                        .collect::<Vec<_>>()
                        .join(&*delim),
                )
            } else {
                Cow::Borrowed(names.values().next().expect(INTERNAL_ERROR_MSG))
            }
        } else {
            debugln!("PosBuilder:name_no_brackets: just name");
            Cow::Borrowed(self.b.name)
        }
    }
}
impl<'n, 'e> Display for PosBuilder<'n, 'e> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let mut delim = String::new();
        delim
            .push(
                if self.is_set(ArgSettings::RequireDelimiter) {
                    self.v.val_delim.expect(INTERNAL_ERROR_MSG)
                } else {
                    ' '
                },
            );
        if let Some(ref names) = self.v.val_names {
            write!(
                f, "{}", names.values().map(| n | format!("<{}>", n)).collect::< Vec < _
                >> ().join(&* delim)
            )?;
        } else {
            write!(f, "<{}>", self.b.name)?;
        }
        if self.b.settings.is_set(ArgSettings::Multiple)
            && (self.v.val_names.is_none()
                || self.v.val_names.as_ref().unwrap().len() == 1)
        {
            write!(f, "...")?;
        }
        Ok(())
    }
}
impl<'n, 'e> AnyArg<'n, 'e> for PosBuilder<'n, 'e> {
    fn name(&self) -> &'n str {
        self.b.name
    }
    fn overrides(&self) -> Option<&[&'e str]> {
        self.b.overrides.as_ref().map(|o| &o[..])
    }
    fn requires(&self) -> Option<&[(Option<&'e str>, &'n str)]> {
        self.b.requires.as_ref().map(|o| &o[..])
    }
    fn blacklist(&self) -> Option<&[&'e str]> {
        self.b.blacklist.as_ref().map(|o| &o[..])
    }
    fn required_unless(&self) -> Option<&[&'e str]> {
        self.b.r_unless.as_ref().map(|o| &o[..])
    }
    fn val_names(&self) -> Option<&VecMap<&'e str>> {
        self.v.val_names.as_ref()
    }
    fn is_set(&self, s: ArgSettings) -> bool {
        self.b.settings.is_set(s)
    }
    fn set(&mut self, s: ArgSettings) {
        self.b.settings.set(s)
    }
    fn has_switch(&self) -> bool {
        false
    }
    fn max_vals(&self) -> Option<u64> {
        self.v.max_vals
    }
    fn val_terminator(&self) -> Option<&'e str> {
        self.v.terminator
    }
    fn num_vals(&self) -> Option<u64> {
        self.v.num_vals
    }
    fn possible_vals(&self) -> Option<&[&'e str]> {
        self.v.possible_vals.as_ref().map(|o| &o[..])
    }
    fn validator(&self) -> Option<&Rc<Fn(String) -> StdResult<(), String>>> {
        self.v.validator.as_ref()
    }
    fn validator_os(&self) -> Option<&Rc<Fn(&OsStr) -> StdResult<(), OsString>>> {
        self.v.validator_os.as_ref()
    }
    fn min_vals(&self) -> Option<u64> {
        self.v.min_vals
    }
    fn short(&self) -> Option<char> {
        None
    }
    fn long(&self) -> Option<&'e str> {
        None
    }
    fn val_delim(&self) -> Option<char> {
        self.v.val_delim
    }
    fn takes_value(&self) -> bool {
        true
    }
    fn help(&self) -> Option<&'e str> {
        self.b.help
    }
    fn long_help(&self) -> Option<&'e str> {
        self.b.long_help
    }
    fn default_vals_ifs(
        &self,
    ) -> Option<map::Values<(&'n str, Option<&'e OsStr>, &'e OsStr)>> {
        self.v.default_vals_ifs.as_ref().map(|vm| vm.values())
    }
    fn default_val(&self) -> Option<&'e OsStr> {
        self.v.default_val
    }
    fn env<'s>(&'s self) -> Option<(&'n OsStr, Option<&'s OsString>)> {
        self.v.env.as_ref().map(|&(key, ref value)| (key, value.as_ref()))
    }
    fn longest_filter(&self) -> bool {
        true
    }
    fn aliases(&self) -> Option<Vec<&'e str>> {
        None
    }
}
impl<'n, 'e> DispOrder for PosBuilder<'n, 'e> {
    fn disp_ord(&self) -> usize {
        self.index as usize
    }
}
impl<'n, 'e> PartialEq for PosBuilder<'n, 'e> {
    fn eq(&self, other: &PosBuilder<'n, 'e>) -> bool {
        self.b == other.b
    }
}
#[cfg(test)]
mod test {
    use super::PosBuilder;
    use args::settings::ArgSettings;
    use map::VecMap;
    #[test]
    fn display_mult() {
        let mut p = PosBuilder::new("pos", 1);
        p.b.settings.set(ArgSettings::Multiple);
        assert_eq!(&* format!("{}", p), "<pos>...");
    }
    #[test]
    fn display_required() {
        let mut p2 = PosBuilder::new("pos", 1);
        p2.b.settings.set(ArgSettings::Required);
        assert_eq!(&* format!("{}", p2), "<pos>");
    }
    #[test]
    fn display_val_names() {
        let mut p2 = PosBuilder::new("pos", 1);
        let mut vm = VecMap::new();
        vm.insert(0, "file1");
        vm.insert(1, "file2");
        p2.v.val_names = Some(vm);
        assert_eq!(&* format!("{}", p2), "<file1> <file2>");
    }
    #[test]
    fn display_val_names_req() {
        let mut p2 = PosBuilder::new("pos", 1);
        p2.b.settings.set(ArgSettings::Required);
        let mut vm = VecMap::new();
        vm.insert(0, "file1");
        vm.insert(1, "file2");
        p2.v.val_names = Some(vm);
        assert_eq!(&* format!("{}", p2), "<file1> <file2>");
    }
}
#[cfg(test)]
mod tests_llm_16_120 {
    use super::*;
    use crate::*;
    use std::ffi::{OsStr, OsString};
    #[test]
    fn test_env() {
        let _rug_st_tests_llm_16_120_rrrruuuugggg_test_env = 0;
        let rug_fuzz_0 = "test";
        let rug_fuzz_1 = 1;
        let builder = PosBuilder::new(rug_fuzz_0, rug_fuzz_1);
        let env = builder.env();
        debug_assert_eq!(env, None);
        let _rug_ed_tests_llm_16_120_rrrruuuugggg_test_env = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_121 {
    use super::*;
    use crate::*;
    use crate::args::arg_builder::valued::Valued;
    use crate::args::arg_builder::positional::PosBuilder;
    use crate::args::settings::ArgFlags;
    #[test]
    fn test_has_switch() {
        let _rug_st_tests_llm_16_121_rrrruuuugggg_test_has_switch = 0;
        let rug_fuzz_0 = "test";
        let rug_fuzz_1 = "help";
        let rug_fuzz_2 = "long help";
        let rug_fuzz_3 = 0;
        let pos_builder = PosBuilder {
            b: Base {
                name: rug_fuzz_0,
                help: Some(rug_fuzz_1),
                long_help: Some(rug_fuzz_2),
                blacklist: None,
                settings: ArgFlags::new(),
                r_unless: None,
                overrides: None,
                groups: None,
                requires: None,
            },
            v: Valued {
                possible_vals: None,
                val_names: None,
                num_vals: None,
                max_vals: None,
                min_vals: None,
                validator: None,
                validator_os: None,
                val_delim: None,
                default_val: None,
                default_vals_ifs: None,
                env: None,
                terminator: None,
            },
            index: rug_fuzz_3,
        };
        let result = pos_builder.has_switch();
        debug_assert_eq!(result, false);
        let _rug_ed_tests_llm_16_121_rrrruuuugggg_test_has_switch = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_122 {
    use super::*;
    use crate::*;
    #[test]
    fn test_help() {
        let _rug_st_tests_llm_16_122_rrrruuuugggg_test_help = 0;
        let rug_fuzz_0 = "test";
        let rug_fuzz_1 = 0;
        let pos_builder = PosBuilder::new(rug_fuzz_0, rug_fuzz_1);
        debug_assert_eq!(pos_builder.help(), None);
        let _rug_ed_tests_llm_16_122_rrrruuuugggg_test_help = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_123 {
    use super::*;
    use crate::*;
    use args::any_arg::AnyArg;
    #[test]
    fn test_is_set() {
        let _rug_st_tests_llm_16_123_rrrruuuugggg_test_is_set = 0;
        let rug_fuzz_0 = "test_name";
        let rug_fuzz_1 = 0;
        let pos_builder = PosBuilder::new(rug_fuzz_0, rug_fuzz_1);
        debug_assert_eq!(pos_builder.is_set(ArgSettings::Required), false);
        debug_assert_eq!(pos_builder.is_set(ArgSettings::Multiple), false);
        let _rug_ed_tests_llm_16_123_rrrruuuugggg_test_is_set = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_128 {
    use super::*;
    use crate::*;
    #[test]
    fn test_min_vals() {
        let _rug_st_tests_llm_16_128_rrrruuuugggg_test_min_vals = 0;
        let rug_fuzz_0 = "test";
        let rug_fuzz_1 = 0;
        let name = rug_fuzz_0;
        let index = rug_fuzz_1;
        let pos_builder = PosBuilder::new(name, index);
        let min_vals = pos_builder.min_vals();
        debug_assert_eq!(min_vals, pos_builder.v.min_vals);
        let _rug_ed_tests_llm_16_128_rrrruuuugggg_test_min_vals = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_141 {
    use super::*;
    use crate::*;
    #[test]
    fn test_validator() {
        let _rug_st_tests_llm_16_141_rrrruuuugggg_test_validator = 0;
        let rug_fuzz_0 = "test";
        let rug_fuzz_1 = 0;
        let base = Base::new(rug_fuzz_0);
        let valued = Valued::default();
        let pos_builder = PosBuilder {
            b: base,
            v: valued,
            index: rug_fuzz_1,
        };
        let validator = pos_builder.validator();
        debug_assert_eq!(validator.is_none(), true);
        let _rug_ed_tests_llm_16_141_rrrruuuugggg_test_validator = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_144 {
    use super::*;
    use crate::*;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_144_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "name1";
        let rug_fuzz_1 = "name2";
        let rug_fuzz_2 = "name1";
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = "name2";
        let rug_fuzz_5 = 2;
        let b1 = Base::new(rug_fuzz_0);
        let b2 = Base::new(rug_fuzz_1);
        let pb1 = PosBuilder::new(rug_fuzz_2, rug_fuzz_3);
        let pb2 = PosBuilder::new(rug_fuzz_4, rug_fuzz_5);
        debug_assert_eq!(b1.eq(& b2), false);
        debug_assert_eq!(pb1.eq(& pb2), false);
        let _rug_ed_tests_llm_16_144_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_229 {
    use super::*;
    use crate::*;
    use args::any_arg::AnyArg;
    #[test]
    fn test_multiple_str() {
        let _rug_st_tests_llm_16_229_rrrruuuugggg_test_multiple_str = 0;
        let rug_fuzz_0 = "arg_name";
        let rug_fuzz_1 = 1;
        let mut pos_builder = PosBuilder::new(rug_fuzz_0, rug_fuzz_1);
        pos_builder.b.set(ArgSettings::Multiple);
        let result = pos_builder.multiple_str();
        debug_assert_eq!(result, "...");
        let _rug_ed_tests_llm_16_229_rrrruuuugggg_test_multiple_str = 0;
    }
    #[test]
    fn test_multiple_str_no_multiple_setting() {
        let _rug_st_tests_llm_16_229_rrrruuuugggg_test_multiple_str_no_multiple_setting = 0;
        let rug_fuzz_0 = "arg_name";
        let rug_fuzz_1 = 1;
        let pos_builder = PosBuilder::new(rug_fuzz_0, rug_fuzz_1);
        let result = pos_builder.multiple_str();
        debug_assert_eq!(result, "");
        let _rug_ed_tests_llm_16_229_rrrruuuugggg_test_multiple_str_no_multiple_setting = 0;
    }
    #[test]
    fn test_multiple_str_val_names_len() {
        let _rug_st_tests_llm_16_229_rrrruuuugggg_test_multiple_str_val_names_len = 0;
        let rug_fuzz_0 = "arg_name";
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = "val_name";
        let rug_fuzz_4 = 1;
        let rug_fuzz_5 = "val_name2";
        let mut pos_builder = PosBuilder::new(rug_fuzz_0, rug_fuzz_1);
        let mut val_names = VecMap::new();
        val_names.insert(rug_fuzz_2, rug_fuzz_3);
        val_names.insert(rug_fuzz_4, rug_fuzz_5);
        pos_builder.v.val_names = Some(val_names);
        let result = pos_builder.multiple_str();
        debug_assert_eq!(result, "...");
        let _rug_ed_tests_llm_16_229_rrrruuuugggg_test_multiple_str_val_names_len = 0;
    }
    #[test]
    fn test_multiple_str_val_names_len_no_multiple_setting() {
        let _rug_st_tests_llm_16_229_rrrruuuugggg_test_multiple_str_val_names_len_no_multiple_setting = 0;
        let rug_fuzz_0 = "arg_name";
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = "val_name";
        let rug_fuzz_4 = 1;
        let rug_fuzz_5 = "val_name2";
        let mut pos_builder = PosBuilder::new(rug_fuzz_0, rug_fuzz_1);
        let mut val_names = VecMap::new();
        val_names.insert(rug_fuzz_2, rug_fuzz_3);
        val_names.insert(rug_fuzz_4, rug_fuzz_5);
        pos_builder.v.val_names = Some(val_names);
        pos_builder.b.unset(ArgSettings::Multiple);
        let result = pos_builder.multiple_str();
        debug_assert_eq!(result, "");
        let _rug_ed_tests_llm_16_229_rrrruuuugggg_test_multiple_str_val_names_len_no_multiple_setting = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_230 {
    use std::borrow::Cow;
    #[test]
    fn test_name_no_brackets() {
        let _rug_st_tests_llm_16_230_rrrruuuugggg_test_name_no_brackets = 0;
        let rug_fuzz_0 = "test";
        let rug_fuzz_1 = 0;
        let pb = crate::args::arg_builder::positional::PosBuilder {
            b: crate::args::arg_builder::base::Base {
                name: rug_fuzz_0,
                help: None,
                long_help: None,
                blacklist: None,
                settings: crate::args::settings::ArgFlags::new(),
                r_unless: None,
                overrides: None,
                groups: None,
                requires: None,
            },
            v: crate::args::arg_builder::valued::Valued {
                possible_vals: None,
                val_names: None,
                num_vals: None,
                max_vals: None,
                min_vals: None,
                validator: None,
                validator_os: None,
                val_delim: None,
                default_val: None,
                default_vals_ifs: None,
                env: None,
                terminator: None,
            },
            index: rug_fuzz_1,
        };
        let result = pb.name_no_brackets();
        debug_assert_eq!(result, Cow::Borrowed("test"));
        let _rug_ed_tests_llm_16_230_rrrruuuugggg_test_name_no_brackets = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_231 {
    use super::*;
    use crate::*;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_231_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = "test";
        let rug_fuzz_1 = 0;
        let name = rug_fuzz_0;
        let idx = rug_fuzz_1;
        let pos_builder = PosBuilder::new(name, idx);
        debug_assert_eq!(pos_builder.b.name, name);
        debug_assert_eq!(pos_builder.index, idx);
        let _rug_ed_tests_llm_16_231_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_rug_455 {
    use super::*;
    use crate::args::{arg_builder::PosBuilder, any_arg::AnyArg};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_455_rrrruuuugggg_test_rug = 0;
        let mut p0: PosBuilder<'_, '_> = unimplemented!();
        p0.possible_vals();
        let _rug_ed_tests_rug_455_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_458 {
    use super::*;
    use crate::args::AnyArg;
    use crate::args::arg_builder::positional;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_458_rrrruuuugggg_test_rug = 0;
        let mut p0: positional::PosBuilder<'_, '_> = unimplemented!();
        <positional::PosBuilder<'_, '_> as AnyArg<'_, '_>>::long(&p0);
        let _rug_ed_tests_rug_458_rrrruuuugggg_test_rug = 0;
    }
}
