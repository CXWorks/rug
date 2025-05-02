use Arg;
#[derive(Debug)]
pub struct Switched<'b> {
    pub short: Option<char>,
    pub long: Option<&'b str>,
    pub aliases: Option<Vec<(&'b str, bool)>>,
    pub disp_ord: usize,
    pub unified_ord: usize,
}
impl<'e> Default for Switched<'e> {
    fn default() -> Self {
        Switched {
            short: None,
            long: None,
            aliases: None,
            disp_ord: 999,
            unified_ord: 999,
        }
    }
}
impl<'n, 'e, 'z> From<&'z Arg<'n, 'e>> for Switched<'e> {
    fn from(a: &'z Arg<'n, 'e>) -> Self {
        a.s.clone()
    }
}
impl<'e> Clone for Switched<'e> {
    fn clone(&self) -> Self {
        Switched {
            short: self.short,
            long: self.long,
            aliases: self.aliases.clone(),
            disp_ord: self.disp_ord,
            unified_ord: self.unified_ord,
        }
    }
}
#[cfg(test)]
mod tests_llm_16_145 {
    use super::*;
    use crate::*;
    use args::arg::Arg;
    #[test]
    fn test_clone() {
        let _rug_st_tests_llm_16_145_rrrruuuugggg_test_clone = 0;
        let rug_fuzz_0 = 's';
        let rug_fuzz_1 = "switch";
        let rug_fuzz_2 = "alias1";
        let rug_fuzz_3 = true;
        let rug_fuzz_4 = 1;
        let rug_fuzz_5 = 2;
        let switches = Switched {
            short: Some(rug_fuzz_0),
            long: Some(rug_fuzz_1),
            aliases: Some(vec![(rug_fuzz_2, rug_fuzz_3), ("alias2", false)]),
            disp_ord: rug_fuzz_4,
            unified_ord: rug_fuzz_5,
        };
        let cloned_switches = switches.clone();
        debug_assert_eq!(cloned_switches.short, Some('s'));
        debug_assert_eq!(cloned_switches.long, Some("switch"));
        debug_assert_eq!(
            cloned_switches.aliases, Some(vec![("alias1", true), ("alias2", false)])
        );
        debug_assert_eq!(cloned_switches.disp_ord, 1);
        debug_assert_eq!(cloned_switches.unified_ord, 2);
        let _rug_ed_tests_llm_16_145_rrrruuuugggg_test_clone = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_146 {
    use crate::args::arg_builder::switched::Switched;
    use crate::Arg;
    #[test]
    fn test_default() {
        let _rug_st_tests_llm_16_146_rrrruuuugggg_test_default = 0;
        let rug_fuzz_0 = 999;
        let rug_fuzz_1 = 999;
        let expected = Switched {
            short: None,
            long: None,
            aliases: None,
            disp_ord: rug_fuzz_0,
            unified_ord: rug_fuzz_1,
        };
        let result = Switched::default();
        debug_assert_eq!(result.short, expected.short);
        debug_assert_eq!(result.long, expected.long);
        debug_assert_eq!(result.aliases, expected.aliases);
        debug_assert_eq!(result.disp_ord, expected.disp_ord);
        debug_assert_eq!(result.unified_ord, expected.unified_ord);
        let _rug_ed_tests_llm_16_146_rrrruuuugggg_test_default = 0;
    }
}
