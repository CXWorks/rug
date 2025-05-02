use std::convert::From;
use std::ffi::{OsStr, OsString};
use std::fmt::{Display, Formatter, Result};
use std::mem;
use std::rc::Rc;
use std::result::Result as StdResult;
use args::{AnyArg, ArgSettings, Base, DispOrder, Switched};
use map::{self, VecMap};
use Arg;
#[derive(Default, Clone, Debug)]
#[doc(hidden)]
pub struct FlagBuilder<'n, 'e>
where
    'n: 'e,
{
    pub b: Base<'n, 'e>,
    pub s: Switched<'e>,
}
impl<'n, 'e> FlagBuilder<'n, 'e> {
    pub fn new(name: &'n str) -> Self {
        FlagBuilder {
            b: Base::new(name),
            ..Default::default()
        }
    }
}
impl<'a, 'b, 'z> From<&'z Arg<'a, 'b>> for FlagBuilder<'a, 'b> {
    fn from(a: &'z Arg<'a, 'b>) -> Self {
        FlagBuilder {
            b: Base::from(a),
            s: Switched::from(a),
        }
    }
}
impl<'a, 'b> From<Arg<'a, 'b>> for FlagBuilder<'a, 'b> {
    fn from(mut a: Arg<'a, 'b>) -> Self {
        FlagBuilder {
            b: mem::replace(&mut a.b, Base::default()),
            s: mem::replace(&mut a.s, Switched::default()),
        }
    }
}
impl<'n, 'e> Display for FlagBuilder<'n, 'e> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        if let Some(l) = self.s.long {
            write!(f, "--{}", l)?;
        } else {
            write!(f, "-{}", self.s.short.unwrap())?;
        }
        Ok(())
    }
}
impl<'n, 'e> AnyArg<'n, 'e> for FlagBuilder<'n, 'e> {
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
    fn is_set(&self, s: ArgSettings) -> bool {
        self.b.settings.is_set(s)
    }
    fn has_switch(&self) -> bool {
        true
    }
    fn takes_value(&self) -> bool {
        false
    }
    fn set(&mut self, s: ArgSettings) {
        self.b.settings.set(s)
    }
    fn max_vals(&self) -> Option<u64> {
        None
    }
    fn val_names(&self) -> Option<&VecMap<&'e str>> {
        None
    }
    fn num_vals(&self) -> Option<u64> {
        None
    }
    fn possible_vals(&self) -> Option<&[&'e str]> {
        None
    }
    fn validator(&self) -> Option<&Rc<Fn(String) -> StdResult<(), String>>> {
        None
    }
    fn validator_os(&self) -> Option<&Rc<Fn(&OsStr) -> StdResult<(), OsString>>> {
        None
    }
    fn min_vals(&self) -> Option<u64> {
        None
    }
    fn short(&self) -> Option<char> {
        self.s.short
    }
    fn long(&self) -> Option<&'e str> {
        self.s.long
    }
    fn val_delim(&self) -> Option<char> {
        None
    }
    fn help(&self) -> Option<&'e str> {
        self.b.help
    }
    fn long_help(&self) -> Option<&'e str> {
        self.b.long_help
    }
    fn val_terminator(&self) -> Option<&'e str> {
        None
    }
    fn default_val(&self) -> Option<&'e OsStr> {
        None
    }
    fn default_vals_ifs(
        &self,
    ) -> Option<map::Values<(&'n str, Option<&'e OsStr>, &'e OsStr)>> {
        None
    }
    fn env<'s>(&'s self) -> Option<(&'n OsStr, Option<&'s OsString>)> {
        None
    }
    fn longest_filter(&self) -> bool {
        self.s.long.is_some()
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
impl<'n, 'e> DispOrder for FlagBuilder<'n, 'e> {
    fn disp_ord(&self) -> usize {
        self.s.disp_ord
    }
}
impl<'n, 'e> PartialEq for FlagBuilder<'n, 'e> {
    fn eq(&self, other: &FlagBuilder<'n, 'e>) -> bool {
        self.b == other.b
    }
}
#[cfg(test)]
mod test {
    use super::FlagBuilder;
    use args::settings::ArgSettings;
    #[test]
    fn flagbuilder_display() {
        let mut f = FlagBuilder::new("flg");
        f.b.settings.set(ArgSettings::Multiple);
        f.s.long = Some("flag");
        assert_eq!(&* format!("{}", f), "--flag");
        let mut f2 = FlagBuilder::new("flg");
        f2.s.short = Some('f');
        assert_eq!(&* format!("{}", f2), "-f");
    }
    #[test]
    fn flagbuilder_display_single_alias() {
        let mut f = FlagBuilder::new("flg");
        f.s.long = Some("flag");
        f.s.aliases = Some(vec![("als", true)]);
        assert_eq!(&* format!("{}", f), "--flag");
    }
    #[test]
    fn flagbuilder_display_multiple_aliases() {
        let mut f = FlagBuilder::new("flg");
        f.s.short = Some('f');
        f
            .s
            .aliases = Some(
            vec![("alias_not_visible", false), ("f2", true), ("f3", true), ("f4", true),],
        );
        assert_eq!(&* format!("{}", f), "-f");
    }
}
#[cfg(test)]
mod tests_llm_16_59 {
    use super::*;
    use crate::*;
    #[test]
    fn test_blacklist() {
        let _rug_st_tests_llm_16_59_rrrruuuugggg_test_blacklist = 0;
        let flag_builder: FlagBuilder = Default::default();
        let result = flag_builder.blacklist();
        debug_assert_eq!(result, None);
        let _rug_ed_tests_llm_16_59_rrrruuuugggg_test_blacklist = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_60 {
    use super::*;
    use crate::*;
    use std::ffi::OsStr;
    #[test]
    fn default_val_test() {
        let _rug_st_tests_llm_16_60_rrrruuuugggg_default_val_test = 0;
        let rug_fuzz_0 = "test_flag";
        let flag = FlagBuilder::new(rug_fuzz_0);
        let result = flag.default_val();
        debug_assert_eq!(result, None);
        let _rug_ed_tests_llm_16_60_rrrruuuugggg_default_val_test = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_61 {
    use super::*;
    use crate::*;
    #[test]
    fn test_default_vals_ifs() {
        let _rug_st_tests_llm_16_61_rrrruuuugggg_test_default_vals_ifs = 0;
        let rug_fuzz_0 = "test";
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = false;
        let flag_builder: FlagBuilder<'static, 'static> = FlagBuilder::new(rug_fuzz_0);
        let result = flag_builder.default_vals_ifs();
        match result {
            Some(map_vals) => {
                debug_assert_eq!(rug_fuzz_1, map_vals.count());
            }
            None => debug_assert!(rug_fuzz_2, "Expected Some(map_vals), got None"),
        }
        let _rug_ed_tests_llm_16_61_rrrruuuugggg_test_default_vals_ifs = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_67 {
    use super::*;
    use crate::*;
    #[test]
    fn test_long_help() {
        let _rug_st_tests_llm_16_67_rrrruuuugggg_test_long_help = 0;
        let rug_fuzz_0 = "test_flag";
        let rug_fuzz_1 = "test_flag";
        let rug_fuzz_2 = "This is a test flag";
        let flag_builder = FlagBuilder::new(rug_fuzz_0);
        debug_assert_eq!(flag_builder.long_help(), None);
        let flag_builder = FlagBuilder {
            b: Base {
                name: rug_fuzz_1,
                long_help: Some(rug_fuzz_2),
                ..Default::default()
            },
            ..Default::default()
        };
        debug_assert_eq!(flag_builder.long_help(), Some("This is a test flag"));
        let _rug_ed_tests_llm_16_67_rrrruuuugggg_test_long_help = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_69 {
    use super::*;
    use crate::*;
    use crate::args::any_arg::AnyArg;
    #[test]
    fn test_max_vals() {
        let _rug_st_tests_llm_16_69_rrrruuuugggg_test_max_vals = 0;
        let rug_fuzz_0 = "test_flag";
        let flag_builder: FlagBuilder<'static, 'static> = FlagBuilder::new(rug_fuzz_0);
        let max_vals = flag_builder.max_vals();
        debug_assert_eq!(max_vals, None);
        let _rug_ed_tests_llm_16_69_rrrruuuugggg_test_max_vals = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_71 {
    use super::*;
    use crate::*;
    #[test]
    fn test_name() {
        let _rug_st_tests_llm_16_71_rrrruuuugggg_test_name = 0;
        let rug_fuzz_0 = "test";
        let flag_builder: FlagBuilder = FlagBuilder::new(rug_fuzz_0);
        debug_assert_eq!(flag_builder.name(), "test");
        let _rug_ed_tests_llm_16_71_rrrruuuugggg_test_name = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_72 {
    use super::*;
    use crate::*;
    #[test]
    fn test_num_vals() {
        let _rug_st_tests_llm_16_72_rrrruuuugggg_test_num_vals = 0;
        let rug_fuzz_0 = "test_flag";
        let flag_builder: FlagBuilder = FlagBuilder::new(rug_fuzz_0);
        let result = flag_builder.num_vals();
        debug_assert_eq!(result, None);
        let _rug_ed_tests_llm_16_72_rrrruuuugggg_test_num_vals = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_76 {
    use super::*;
    use crate::*;
    #[test]
    fn test_requires() {
        let _rug_st_tests_llm_16_76_rrrruuuugggg_test_requires = 0;
        let rug_fuzz_0 = "test_flag";
        let flag_builder: FlagBuilder = FlagBuilder::new(rug_fuzz_0);
        let requires = flag_builder.requires();
        debug_assert_eq!(requires, None);
        let _rug_ed_tests_llm_16_76_rrrruuuugggg_test_requires = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_77 {
    use super::*;
    use crate::*;
    #[test]
    fn test_set() {
        let _rug_st_tests_llm_16_77_rrrruuuugggg_test_set = 0;
        let rug_fuzz_0 = "flag";
        let mut builder = FlagBuilder::new(rug_fuzz_0);
        let setting = ArgSettings::Required;
        builder.set(setting);
        debug_assert!(builder.b.is_set(setting));
        let _rug_ed_tests_llm_16_77_rrrruuuugggg_test_set = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_78 {
    use super::*;
    use crate::*;
    #[test]
    fn test_short() {
        let _rug_st_tests_llm_16_78_rrrruuuugggg_test_short = 0;
        let rug_fuzz_0 = "test";
        let builder = FlagBuilder::new(rug_fuzz_0);
        debug_assert_eq!(builder.short(), None);
        let _rug_ed_tests_llm_16_78_rrrruuuugggg_test_short = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_79 {
    use super::*;
    use crate::*;
    #[test]
    fn test_takes_value() {
        let _rug_st_tests_llm_16_79_rrrruuuugggg_test_takes_value = 0;
        let rug_fuzz_0 = "test";
        let flag_builder = FlagBuilder::new(rug_fuzz_0);
        debug_assert_eq!(flag_builder.takes_value(), false);
        let _rug_ed_tests_llm_16_79_rrrruuuugggg_test_takes_value = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_80 {
    use super::*;
    use crate::*;
    #[test]
    fn test_val_delim() {
        let _rug_st_tests_llm_16_80_rrrruuuugggg_test_val_delim = 0;
        let rug_fuzz_0 = "test_flag";
        let flag_builder = FlagBuilder::new(rug_fuzz_0);
        debug_assert_eq!(flag_builder.val_delim(), None);
        let _rug_ed_tests_llm_16_80_rrrruuuugggg_test_val_delim = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_82 {
    use super::*;
    use crate::*;
    use crate::args::any_arg::AnyArg;
    #[test]
    fn test_val_terminator() {
        let _rug_st_tests_llm_16_82_rrrruuuugggg_test_val_terminator = 0;
        let rug_fuzz_0 = "test";
        let flag_builder = FlagBuilder::new(rug_fuzz_0);
        let result = flag_builder.val_terminator();
        debug_assert_eq!(result, None);
        let _rug_ed_tests_llm_16_82_rrrruuuugggg_test_val_terminator = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_86 {
    use super::*;
    use crate::*;
    use crate::*;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_86_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "test1";
        let rug_fuzz_1 = "test2";
        let rug_fuzz_2 = "test1";
        let arg_builder1 = FlagBuilder::new(rug_fuzz_0);
        let arg_builder2 = FlagBuilder::new(rug_fuzz_1);
        debug_assert_eq!(arg_builder1.eq(& arg_builder2), false);
        let arg_builder3 = FlagBuilder::new(rug_fuzz_2);
        debug_assert_eq!(arg_builder1.eq(& arg_builder3), true);
        let _rug_ed_tests_llm_16_86_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_rug_397 {
    use super::*;
    use crate::args::arg_builder::flag::FlagBuilder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_397_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "example_flag_name";
        let mut p0 = rug_fuzz_0;
        FlagBuilder::new(&p0);
        let _rug_ed_tests_rug_397_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_405 {
    use super::*;
    use crate::args::AnyArg;
    use crate::args::arg_builder::FlagBuilder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_405_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "flag";
        let mut p0: FlagBuilder<'static, 'static> = FlagBuilder::new(rug_fuzz_0);
        <FlagBuilder<'static, 'static> as AnyArg<'static, 'static>>::possible_vals(&p0);
        let _rug_ed_tests_rug_405_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_408 {
    use super::*;
    use crate::args::AnyArg;
    use crate::args::arg_builder::*;
    use crate::arg_enum;
    arg_enum! {
        #[derive(Debug)] pub enum TestEnum { Option1, Option2, Option3, }
    }
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_408_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "flag";
        let mut p0: FlagBuilder<'static, 'static> = FlagBuilder::new(rug_fuzz_0);
        p0.min_vals();
        let _rug_ed_tests_rug_408_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_409 {
    use super::*;
    use crate::args::{AnyArg, arg_builder::FlagBuilder};
    #[test]
    fn test_long() {
        let _rug_st_tests_rug_409_rrrruuuugggg_test_long = 0;
        let rug_fuzz_0 = "flag";
        let mut p0: FlagBuilder = FlagBuilder::new(rug_fuzz_0);
        let result = <FlagBuilder as AnyArg>::long(&p0);
        debug_assert_eq!(result, None);
        let _rug_ed_tests_rug_409_rrrruuuugggg_test_long = 0;
    }
}
#[cfg(test)]
mod tests_rug_410 {
    use super::*;
    use crate::args::AnyArg;
    use crate::args::arg_builder::flag::FlagBuilder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_410_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "";
        let mut p0: FlagBuilder<'static, 'static> = FlagBuilder {
            b: Default::default(),
            ..FlagBuilder::<'static, 'static>::new(rug_fuzz_0)
        };
        p0.help();
        let _rug_ed_tests_rug_410_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_412 {
    use super::*;
    use crate::args::{AnyArg, arg_builder::FlagBuilder};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_412_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "flag";
        let mut p0: FlagBuilder<'static, 'static> = FlagBuilder::new(rug_fuzz_0);
        <FlagBuilder<'static, 'static> as AnyArg<'static, 'static>>::longest_filter(&p0);
        let _rug_ed_tests_rug_412_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_413 {
    use super::*;
    use crate::args::AnyArg;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_413_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "flag";
        let mut p0: FlagBuilder<'static, 'static> = FlagBuilder::new(rug_fuzz_0);
        <FlagBuilder<'static, 'static> as AnyArg<'static, 'static>>::aliases(&p0);
        let _rug_ed_tests_rug_413_rrrruuuugggg_test_rug = 0;
    }
}
