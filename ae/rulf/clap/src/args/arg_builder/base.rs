use args::{Arg, ArgFlags, ArgSettings};
#[derive(Debug, Clone, Default)]
pub struct Base<'a, 'b>
where
    'a: 'b,
{
    pub name: &'a str,
    pub help: Option<&'b str>,
    pub long_help: Option<&'b str>,
    pub blacklist: Option<Vec<&'a str>>,
    pub settings: ArgFlags,
    pub r_unless: Option<Vec<&'a str>>,
    pub overrides: Option<Vec<&'a str>>,
    pub groups: Option<Vec<&'a str>>,
    pub requires: Option<Vec<(Option<&'b str>, &'a str)>>,
}
impl<'n, 'e> Base<'n, 'e> {
    pub fn new(name: &'n str) -> Self {
        Base {
            name: name,
            ..Default::default()
        }
    }
    pub fn set(&mut self, s: ArgSettings) {
        self.settings.set(s);
    }
    pub fn unset(&mut self, s: ArgSettings) {
        self.settings.unset(s);
    }
    pub fn is_set(&self, s: ArgSettings) -> bool {
        self.settings.is_set(s)
    }
}
impl<'n, 'e, 'z> From<&'z Arg<'n, 'e>> for Base<'n, 'e> {
    fn from(a: &'z Arg<'n, 'e>) -> Self {
        a.b.clone()
    }
}
impl<'n, 'e> PartialEq for Base<'n, 'e> {
    fn eq(&self, other: &Base<'n, 'e>) -> bool {
        self.name == other.name
    }
}
#[cfg(test)]
mod tests_llm_16_223 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_set() {
        let _rug_st_tests_llm_16_223_rrrruuuugggg_test_is_set = 0;
        let rug_fuzz_0 = "name";
        let mut base = Base::new(rug_fuzz_0);
        base.set(ArgSettings::Required);
        debug_assert_eq!(base.is_set(ArgSettings::Required), true);
        let _rug_ed_tests_llm_16_223_rrrruuuugggg_test_is_set = 0;
    }
}
#[cfg(test)]
mod tests_rug_392 {
    use super::*;
    use args::arg_builder::base::Base;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_392_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample_name";
        let p0: &str = rug_fuzz_0;
        Base::new(&p0);
        let _rug_ed_tests_rug_392_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_393 {
    use super::*;
    use crate::args::arg_builder::base::Base;
    use crate::args::settings::ArgSettings;
    use std::str::FromStr;
    #[test]
    fn test_set() {
        let _rug_st_tests_rug_393_rrrruuuugggg_test_set = 0;
        let rug_fuzz_0 = "argument_name";
        let rug_fuzz_1 = "required";
        let mut p0: Base<'_, '_> = Base::new(rug_fuzz_0);
        let mut p1: ArgSettings = ArgSettings::from_str(rug_fuzz_1).unwrap();
        p0.set(p1);
        let _rug_ed_tests_rug_393_rrrruuuugggg_test_set = 0;
    }
}
#[cfg(test)]
mod tests_rug_394 {
    use super::*;
    use crate::args::arg_builder::base::Base;
    use crate::args::settings::ArgSettings;
    use std::str::FromStr;
    #[test]
    fn test_unset() {
        let _rug_st_tests_rug_394_rrrruuuugggg_test_unset = 0;
        let rug_fuzz_0 = "argument_name";
        let rug_fuzz_1 = "required";
        let mut p0: Base<'_, '_> = Base::new(rug_fuzz_0);
        let mut p1: ArgSettings = ArgSettings::from_str(rug_fuzz_1).unwrap();
        crate::args::arg_builder::base::Base::<'_, '_>::unset(&mut p0, p1);
        let _rug_ed_tests_rug_394_rrrruuuugggg_test_unset = 0;
    }
}
#[cfg(test)]
mod tests_rug_396 {
    use super::*;
    use crate::args::arg_builder::base::Base;
    use crate::std::cmp::PartialEq;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_396_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "argument_name";
        let rug_fuzz_1 = "argument_name";
        let mut p0: Base<'_, '_> = Base::new(rug_fuzz_0);
        let mut p1: Base<'_, '_> = Base::new(rug_fuzz_1);
        p0.eq(&p1);
        let _rug_ed_tests_rug_396_rrrruuuugggg_test_rug = 0;
    }
}
