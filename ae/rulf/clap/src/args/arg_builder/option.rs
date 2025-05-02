use std::ffi::{OsStr, OsString};
use std::fmt::{Display, Formatter, Result};
use std::mem;
use std::rc::Rc;
use std::result::Result as StdResult;
use args::{AnyArg, Arg, ArgSettings, Base, DispOrder, Switched, Valued};
use map::{self, VecMap};
use INTERNAL_ERROR_MSG;
#[allow(missing_debug_implementations)]
#[doc(hidden)]
#[derive(Default, Clone)]
pub struct OptBuilder<'n, 'e>
where
    'n: 'e,
{
    pub b: Base<'n, 'e>,
    pub s: Switched<'e>,
    pub v: Valued<'n, 'e>,
}
impl<'n, 'e> OptBuilder<'n, 'e> {
    pub fn new(name: &'n str) -> Self {
        OptBuilder {
            b: Base::new(name),
            ..Default::default()
        }
    }
}
impl<'n, 'e, 'z> From<&'z Arg<'n, 'e>> for OptBuilder<'n, 'e> {
    fn from(a: &'z Arg<'n, 'e>) -> Self {
        OptBuilder {
            b: Base::from(a),
            s: Switched::from(a),
            v: Valued::from(a),
        }
    }
}
impl<'n, 'e> From<Arg<'n, 'e>> for OptBuilder<'n, 'e> {
    fn from(mut a: Arg<'n, 'e>) -> Self {
        a.v.fill_in();
        OptBuilder {
            b: mem::replace(&mut a.b, Base::default()),
            s: mem::replace(&mut a.s, Switched::default()),
            v: mem::replace(&mut a.v, Valued::default()),
        }
    }
}
impl<'n, 'e> Display for OptBuilder<'n, 'e> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        debugln!("OptBuilder::fmt:{}", self.b.name);
        let sep = if self.b.is_set(ArgSettings::RequireEquals) { "=" } else { " " };
        if let Some(l) = self.s.long {
            write!(f, "--{}{}", l, sep)?;
        } else {
            write!(f, "-{}{}", self.s.short.unwrap(), sep)?;
        }
        let delim = if self.is_set(ArgSettings::RequireDelimiter) {
            self.v.val_delim.expect(INTERNAL_ERROR_MSG)
        } else {
            ' '
        };
        if let Some(ref vec) = self.v.val_names {
            let mut it = vec.iter().peekable();
            while let Some((_, val)) = it.next() {
                write!(f, "<{}>", val)?;
                if it.peek().is_some() {
                    write!(f, "{}", delim)?;
                }
            }
            let num = vec.len();
            if self.is_set(ArgSettings::Multiple) && num == 1 {
                write!(f, "...")?;
            }
        } else if let Some(num) = self.v.num_vals {
            let mut it = (0..num).peekable();
            while let Some(_) = it.next() {
                write!(f, "<{}>", self.b.name)?;
                if it.peek().is_some() {
                    write!(f, "{}", delim)?;
                }
            }
            if self.is_set(ArgSettings::Multiple) && num == 1 {
                write!(f, "...")?;
            }
        } else {
            write!(
                f, "<{}>{}", self.b.name, if self.is_set(ArgSettings::Multiple) { "..." }
                else { "" }
            )?;
        }
        Ok(())
    }
}
impl<'n, 'e> AnyArg<'n, 'e> for OptBuilder<'n, 'e> {
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
    fn has_switch(&self) -> bool {
        true
    }
    fn set(&mut self, s: ArgSettings) {
        self.b.settings.set(s)
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
        self.s.short
    }
    fn long(&self) -> Option<&'e str> {
        self.s.long
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
    fn default_val(&self) -> Option<&'e OsStr> {
        self.v.default_val
    }
    fn default_vals_ifs(
        &self,
    ) -> Option<map::Values<(&'n str, Option<&'e OsStr>, &'e OsStr)>> {
        self.v.default_vals_ifs.as_ref().map(|vm| vm.values())
    }
    fn env<'s>(&'s self) -> Option<(&'n OsStr, Option<&'s OsString>)> {
        self.v.env.as_ref().map(|&(key, ref value)| (key, value.as_ref()))
    }
    fn longest_filter(&self) -> bool {
        true
    }
    fn aliases(&self) -> Option<Vec<&'e str>> {
        if let Some(ref aliases) = self.s.aliases {
            let vis_aliases: Vec<_> = aliases
                .iter()
                .filter_map(|&(n, v)| if v { Some(n) } else { None })
                .collect();
            if vis_aliases.is_empty() { None } else { Some(vis_aliases) }
        } else {
            None
        }
    }
}
impl<'n, 'e> DispOrder for OptBuilder<'n, 'e> {
    fn disp_ord(&self) -> usize {
        self.s.disp_ord
    }
}
impl<'n, 'e> PartialEq for OptBuilder<'n, 'e> {
    fn eq(&self, other: &OptBuilder<'n, 'e>) -> bool {
        self.b == other.b
    }
}
#[cfg(test)]
mod test {
    use super::OptBuilder;
    use args::settings::ArgSettings;
    use map::VecMap;
    #[test]
    fn optbuilder_display1() {
        let mut o = OptBuilder::new("opt");
        o.s.long = Some("option");
        o.b.settings.set(ArgSettings::Multiple);
        assert_eq!(&* format!("{}", o), "--option <opt>...");
    }
    #[test]
    fn optbuilder_display2() {
        let mut v_names = VecMap::new();
        v_names.insert(0, "file");
        v_names.insert(1, "name");
        let mut o2 = OptBuilder::new("opt");
        o2.s.short = Some('o');
        o2.v.val_names = Some(v_names);
        assert_eq!(&* format!("{}", o2), "-o <file> <name>");
    }
    #[test]
    fn optbuilder_display3() {
        let mut v_names = VecMap::new();
        v_names.insert(0, "file");
        v_names.insert(1, "name");
        let mut o2 = OptBuilder::new("opt");
        o2.s.short = Some('o');
        o2.v.val_names = Some(v_names);
        o2.b.settings.set(ArgSettings::Multiple);
        assert_eq!(&* format!("{}", o2), "-o <file> <name>");
    }
    #[test]
    fn optbuilder_display_single_alias() {
        let mut o = OptBuilder::new("opt");
        o.s.long = Some("option");
        o.s.aliases = Some(vec![("als", true)]);
        assert_eq!(&* format!("{}", o), "--option <opt>");
    }
    #[test]
    fn optbuilder_display_multiple_aliases() {
        let mut o = OptBuilder::new("opt");
        o.s.long = Some("option");
        o
            .s
            .aliases = Some(
            vec![
                ("als_not_visible", false), ("als2", true), ("als3", true), ("als4",
                true),
            ],
        );
        assert_eq!(&* format!("{}", o), "--option <opt>");
    }
}
#[cfg(test)]
mod tests_llm_16_98 {
    use super::*;
    use crate::*;
    #[test]
    fn test_max_vals() {
        let _rug_st_tests_llm_16_98_rrrruuuugggg_test_max_vals = 0;
        let rug_fuzz_0 = "test";
        let opt_builder: OptBuilder<'static, 'static> = OptBuilder::new(rug_fuzz_0);
        let result = opt_builder.max_vals();
        debug_assert_eq!(result, None);
        let _rug_ed_tests_llm_16_98_rrrruuuugggg_test_max_vals = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_101 {
    use super::*;
    use crate::*;
    #[test]
    fn test_num_vals() {
        let _rug_st_tests_llm_16_101_rrrruuuugggg_test_num_vals = 0;
        let rug_fuzz_0 = "test";
        let opt_builder = OptBuilder::new(rug_fuzz_0);
        let num_vals = opt_builder.num_vals();
        debug_assert_eq!(num_vals, None);
        let _rug_ed_tests_llm_16_101_rrrruuuugggg_test_num_vals = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_111 {
    use super::*;
    use crate::*;
    use crate::args::arg_builder::base::Base;
    #[test]
    fn test_val_terminator() {
        let _rug_st_tests_llm_16_111_rrrruuuugggg_test_val_terminator = 0;
        let rug_fuzz_0 = "test";
        let base = Base::new(rug_fuzz_0);
        let opt_builder = OptBuilder {
            b: base,
            s: Switched::default(),
            v: Valued::default(),
        };
        let result = opt_builder.val_terminator();
        debug_assert_eq!(result, None);
        let _rug_ed_tests_llm_16_111_rrrruuuugggg_test_val_terminator = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_114 {
    use super::*;
    use crate::*;
    #[test]
    fn test_disp_ord() {
        let _rug_st_tests_llm_16_114_rrrruuuugggg_test_disp_ord = 0;
        let rug_fuzz_0 = "test";
        let opt_builder = OptBuilder {
            b: Base::new(rug_fuzz_0),
            s: Switched::default(),
            v: Valued::default(),
        };
        let result = opt_builder.disp_ord();
        debug_assert_eq!(result, opt_builder.s.disp_ord);
        let _rug_ed_tests_llm_16_114_rrrruuuugggg_test_disp_ord = 0;
    }
}
#[cfg(test)]
mod tests_rug_415 {
    use super::*;
    use crate::args::arg_builder::option::OptBuilder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_415_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "name";
        let p0 = rug_fuzz_0;
        OptBuilder::new(&p0);
        let _rug_ed_tests_rug_415_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_419 {
    use super::*;
    use crate::args::AnyArg;
    #[test]
    fn test_overrides() {
        let _rug_st_tests_rug_419_rrrruuuugggg_test_overrides = 0;
        let rug_fuzz_0 = "name";
        let mut p0 = OptBuilder::new(rug_fuzz_0);
        <OptBuilder<'static, 'static> as AnyArg<'static, 'static>>::overrides(&p0);
        let _rug_ed_tests_rug_419_rrrruuuugggg_test_overrides = 0;
    }
}
#[cfg(test)]
mod tests_rug_420 {
    use super::*;
    use crate::args::AnyArg;
    use crate::args::arg_builder::OptBuilder;
    #[test]
    fn test_requires() {
        let _rug_st_tests_rug_420_rrrruuuugggg_test_requires = 0;
        let rug_fuzz_0 = "option";
        let p0: OptBuilder<'static, 'static> = OptBuilder::new(rug_fuzz_0);
        p0.requires();
        let _rug_ed_tests_rug_420_rrrruuuugggg_test_requires = 0;
    }
}
#[cfg(test)]
mod tests_rug_421 {
    use super::*;
    use crate::args::{AnyArg, OptBuilder};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_421_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "test";
        let mut p0: OptBuilder<'static, 'static> = OptBuilder::new(rug_fuzz_0);
        p0.blacklist();
        let _rug_ed_tests_rug_421_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_426 {
    use super::*;
    use crate::args::AnyArg;
    use crate::args::arg_builder::option::OptBuilder;
    use crate::args::settings::ArgSettings;
    use std::str::FromStr;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_426_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "arg";
        let rug_fuzz_1 = "required";
        let mut p0: OptBuilder<'static, 'static> = OptBuilder::<
            'static,
            'static,
        >::new(rug_fuzz_0);
        let mut p1: ArgSettings = ArgSettings::from_str(rug_fuzz_1).unwrap();
        <OptBuilder<'static, 'static> as AnyArg<'static, 'static>>::set(&mut p0, p1);
        let _rug_ed_tests_rug_426_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_428 {
    use super::*;
    use crate::args::{AnyArg, OptBuilder};
    use std::result::Result;
    use std::rc::Rc;
    #[test]
    fn test_validator() {
        let _rug_st_tests_rug_428_rrrruuuugggg_test_validator = 0;
        let rug_fuzz_0 = "option";
        let mut p0: OptBuilder<'static, 'static> = OptBuilder::new(rug_fuzz_0);
        <OptBuilder<'static, 'static> as AnyArg<'static, 'static>>::validator(&p0);
        let _rug_ed_tests_rug_428_rrrruuuugggg_test_validator = 0;
    }
}
#[cfg(test)]
mod tests_rug_429 {
    use super::*;
    use crate::args::{AnyArg, arg_builder::OptBuilder};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_429_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "name";
        let mut p0: OptBuilder = OptBuilder::new(rug_fuzz_0);
        p0.validator_os();
        let _rug_ed_tests_rug_429_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_432 {
    use super::*;
    use crate::args::{AnyArg, arg_builder::option::OptBuilder};
    #[test]
    fn test_long() {
        let _rug_st_tests_rug_432_rrrruuuugggg_test_long = 0;
        let rug_fuzz_0 = "test";
        let mut p0: OptBuilder<'static, 'static> = OptBuilder::new(rug_fuzz_0);
        <OptBuilder<'static, 'static> as AnyArg<'static, 'static>>::long(&p0);
        let _rug_ed_tests_rug_432_rrrruuuugggg_test_long = 0;
    }
}
#[cfg(test)]
mod tests_rug_433 {
    use super::*;
    use crate::args::AnyArg;
    #[test]
    fn test_val_delim() {
        let _rug_st_tests_rug_433_rrrruuuugggg_test_val_delim = 0;
        let rug_fuzz_0 = "option";
        let mut p0: OptBuilder<'static, 'static> = OptBuilder::new(rug_fuzz_0);
        p0.val_delim();
        let _rug_ed_tests_rug_433_rrrruuuugggg_test_val_delim = 0;
    }
}
#[cfg(test)]
mod tests_rug_434 {
    use super::*;
    use crate::args::AnyArg;
    use crate::args::arg_builder::OptBuilder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_434_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "test";
        let mut p0: OptBuilder<'static, 'static> = OptBuilder::new(rug_fuzz_0);
        <OptBuilder<'static, 'static> as AnyArg<'static, 'static>>::takes_value(&p0);
        let _rug_ed_tests_rug_434_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_435 {
    use super::*;
    use crate::args::AnyArg;
    use crate::args::arg_builder::OptBuilder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_435_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "";
        let mut p0: OptBuilder = OptBuilder {
            b: Default::default(),
            ..OptBuilder::new(rug_fuzz_0)
        };
        <OptBuilder as AnyArg>::help(&p0);
        let _rug_ed_tests_rug_435_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_437 {
    use super::*;
    use crate::args::{AnyArg, OptBuilder};
    #[test]
    fn test_default_val() {
        let _rug_st_tests_rug_437_rrrruuuugggg_test_default_val = 0;
        let rug_fuzz_0 = "option";
        let mut p0: OptBuilder<'static, 'static> = OptBuilder::new(rug_fuzz_0);
        <OptBuilder<'static, 'static> as AnyArg<'static, 'static>>::default_val(&p0);
        let _rug_ed_tests_rug_437_rrrruuuugggg_test_default_val = 0;
    }
}
#[cfg(test)]
mod tests_rug_441 {
    use super::*;
    use crate::args::{AnyArg, OptBuilder};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_441_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "option";
        let mut p0: OptBuilder<'static, 'static> = OptBuilder::new(rug_fuzz_0);
        <OptBuilder<'static, 'static> as AnyArg<'static, 'static>>::aliases(&p0);
        let _rug_ed_tests_rug_441_rrrruuuugggg_test_rug = 0;
    }
}
