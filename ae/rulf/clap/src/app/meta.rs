#[doc(hidden)]
#[allow(missing_debug_implementations)]
#[derive(Default, Clone)]
pub struct AppMeta<'b> {
    pub name: String,
    pub bin_name: Option<String>,
    pub author: Option<&'b str>,
    pub version: Option<&'b str>,
    pub long_version: Option<&'b str>,
    pub about: Option<&'b str>,
    pub long_about: Option<&'b str>,
    pub more_help: Option<&'b str>,
    pub pre_help: Option<&'b str>,
    pub aliases: Option<Vec<(&'b str, bool)>>,
    pub usage_str: Option<&'b str>,
    pub usage: Option<String>,
    pub help_str: Option<&'b str>,
    pub disp_ord: usize,
    pub term_w: Option<usize>,
    pub max_w: Option<usize>,
    pub template: Option<&'b str>,
}
impl<'b> AppMeta<'b> {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn with_name(s: String) -> Self {
        AppMeta {
            name: s,
            disp_ord: 999,
            ..Default::default()
        }
    }
}
#[cfg(test)]
mod tests_llm_16_214 {
    use super::*;
    use crate::*;
    use crate::app::meta::AppMeta;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_214_rrrruuuugggg_test_new = 0;
        let app_meta: AppMeta = AppMeta::new();
        debug_assert_eq!(app_meta.name, "".to_string());
        debug_assert_eq!(app_meta.bin_name, None);
        debug_assert_eq!(app_meta.author, None);
        debug_assert_eq!(app_meta.version, None);
        debug_assert_eq!(app_meta.long_version, None);
        debug_assert_eq!(app_meta.about, None);
        debug_assert_eq!(app_meta.long_about, None);
        debug_assert_eq!(app_meta.more_help, None);
        debug_assert_eq!(app_meta.pre_help, None);
        debug_assert_eq!(app_meta.aliases, None);
        debug_assert_eq!(app_meta.usage_str, None);
        debug_assert_eq!(app_meta.usage, None);
        debug_assert_eq!(app_meta.help_str, None);
        debug_assert_eq!(app_meta.disp_ord, 0);
        debug_assert_eq!(app_meta.term_w, None);
        debug_assert_eq!(app_meta.max_w, None);
        debug_assert_eq!(app_meta.template, None);
        let _rug_ed_tests_llm_16_214_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_215 {
    use super::*;
    use crate::*;
    #[test]
    fn test_with_name() {
        let _rug_st_tests_llm_16_215_rrrruuuugggg_test_with_name = 0;
        let rug_fuzz_0 = "test_name";
        let name = rug_fuzz_0.to_string();
        let app_meta = AppMeta::with_name(name.clone());
        debug_assert_eq!(app_meta.name, name);
        debug_assert_eq!(app_meta.disp_ord, 999);
        debug_assert_eq!(app_meta.bin_name, None);
        debug_assert_eq!(app_meta.author, None);
        debug_assert_eq!(app_meta.version, None);
        debug_assert_eq!(app_meta.long_version, None);
        debug_assert_eq!(app_meta.about, None);
        debug_assert_eq!(app_meta.long_about, None);
        debug_assert_eq!(app_meta.more_help, None);
        debug_assert_eq!(app_meta.pre_help, None);
        debug_assert_eq!(app_meta.aliases, None);
        debug_assert_eq!(app_meta.usage_str, None);
        debug_assert_eq!(app_meta.usage, None);
        debug_assert_eq!(app_meta.help_str, None);
        debug_assert_eq!(app_meta.term_w, None);
        debug_assert_eq!(app_meta.max_w, None);
        debug_assert_eq!(app_meta.template, None);
        let _rug_ed_tests_llm_16_215_rrrruuuugggg_test_with_name = 0;
    }
}
