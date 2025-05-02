//! Very basic syntax detection.
use std::borrow::Borrow;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::Arc;
use crate::config::Table;
/// The canonical identifier for a particular `LanguageDefinition`.
#[derive(
    Debug,
    Default,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord
)]
#[allow(clippy::rc_buffer)]
pub struct LanguageId(Arc<String>);
/// Describes a `LanguageDefinition`. Although these are provided by plugins,
/// they are a fundamental concept in core, used to determine things like
/// plugin activations and active user config tables.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageDefinition {
    pub name: LanguageId,
    pub extensions: Vec<String>,
    pub first_line_match: Option<String>,
    pub scope: String,
    #[serde(skip)]
    pub default_config: Option<Table>,
}
/// A repository of all loaded `LanguageDefinition`s.
#[derive(Debug, Default)]
pub struct Languages {
    named: BTreeMap<LanguageId, Arc<LanguageDefinition>>,
    extensions: HashMap<String, Arc<LanguageDefinition>>,
}
impl Languages {
    pub fn new(language_defs: &[LanguageDefinition]) -> Self {
        let mut named = BTreeMap::new();
        let mut extensions = HashMap::new();
        for lang in language_defs.iter() {
            let lang_arc = Arc::new(lang.clone());
            named.insert(lang.name.clone(), lang_arc.clone());
            for ext in &lang.extensions {
                extensions.insert(ext.clone(), lang_arc.clone());
            }
        }
        Languages { named, extensions }
    }
    pub fn language_for_path(&self, path: &Path) -> Option<Arc<LanguageDefinition>> {
        path.extension()
            .or_else(|| path.file_name())
            .and_then(|ext| self.extensions.get(ext.to_str().unwrap_or_default()))
            .map(Arc::clone)
    }
    pub fn language_for_name<S>(&self, name: S) -> Option<Arc<LanguageDefinition>>
    where
        S: AsRef<str>,
    {
        self.named.get(name.as_ref()).map(Arc::clone)
    }
    /// Returns a Vec of any `LanguageDefinition`s which exist
    /// in `self` but not `other`.
    pub fn difference(&self, other: &Languages) -> Vec<Arc<LanguageDefinition>> {
        self.named
            .iter()
            .filter(|(k, _)| !other.named.contains_key(*k))
            .map(|(_, v)| v.clone())
            .collect()
    }
    pub fn iter(&self) -> impl Iterator<Item = &Arc<LanguageDefinition>> {
        self.named.values()
    }
}
impl AsRef<str> for LanguageId {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}
impl Borrow<str> for LanguageId {
    fn borrow(&self) -> &str {
        &self.0.as_ref()
    }
}
impl<'a> From<&'a str> for LanguageId {
    fn from(src: &'a str) -> LanguageId {
        LanguageId(Arc::new(src.into()))
    }
}
#[cfg(test)]
impl LanguageDefinition {
    pub(crate) fn simple(
        name: &str,
        exts: &[&str],
        scope: &str,
        config: Option<Table>,
    ) -> Self {
        LanguageDefinition {
            name: name.into(),
            extensions: exts.iter().map(|s| (*s).into()).collect(),
            first_line_match: None,
            scope: scope.into(),
            default_config: config,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    pub fn language_for_path() {
        let ld_rust = LanguageDefinition {
            name: LanguageId::from("Rust"),
            extensions: vec![String::from("rs")],
            scope: String::from("source.rust"),
            first_line_match: None,
            default_config: None,
        };
        let ld_commit_msg = LanguageDefinition {
            name: LanguageId::from("Git Commit"),
            extensions: vec![
                String::from("COMMIT_EDITMSG"), String::from("MERGE_MSG"),
                String::from("TAG_EDITMSG"),
            ],
            scope: String::from("text.git.commit"),
            first_line_match: None,
            default_config: None,
        };
        let languages = Languages::new(&[ld_rust.clone(), ld_commit_msg.clone()]);
        assert_eq!(
            ld_rust.name, languages.language_for_path(Path::new("/path/test.rs"))
            .unwrap().name
        );
        assert_eq!(
            ld_commit_msg.name, languages
            .language_for_path(Path::new("/path/COMMIT_EDITMSG")).unwrap().name
        );
        assert_eq!(
            ld_commit_msg.name, languages.language_for_path(Path::new("/path/MERGE_MSG"))
            .unwrap().name
        );
        assert_eq!(
            ld_commit_msg.name, languages
            .language_for_path(Path::new("/path/TAG_EDITMSG")).unwrap().name
        );
    }
}
#[cfg(test)]
mod tests_llm_16_76 {
    use super::*;
    use crate::*;
    use syntax::LanguageId;
    use std::borrow::Borrow;
    #[test]
    fn test_borrow() {
        let _rug_st_tests_llm_16_76_rrrruuuugggg_test_borrow = 0;
        let rug_fuzz_0 = "rust";
        let language_id: LanguageId = LanguageId::from(rug_fuzz_0);
        let borrowed_str: &str = language_id.borrow();
        debug_assert_eq!(borrowed_str, "rust");
        let _rug_ed_tests_llm_16_76_rrrruuuugggg_test_borrow = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_77 {
    use super::*;
    use crate::*;
    #[test]
    fn test_as_ref() {
        let _rug_st_tests_llm_16_77_rrrruuuugggg_test_as_ref = 0;
        let rug_fuzz_0 = "rust";
        let language_id: LanguageId = LanguageId::from(rug_fuzz_0);
        debug_assert_eq!(language_id.as_ref(), "rust");
        let _rug_ed_tests_llm_16_77_rrrruuuugggg_test_as_ref = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_79 {
    use crate::syntax::LanguageId;
    use std::convert::From;
    #[test]
    fn test_from() {
        let _rug_st_tests_llm_16_79_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = "rust";
        let src = rug_fuzz_0;
        let result: LanguageId = From::from(src);
        debug_assert_eq!(result.as_ref(), src);
        let _rug_ed_tests_llm_16_79_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_744 {
    use super::*;
    use crate::*;
    use std::path::Path;
    use serde_json::json;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_744_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = "rust";
        let rug_fuzz_1 = "rs";
        let rug_fuzz_2 = "fn main()";
        let rug_fuzz_3 = "source.rust";
        let rug_fuzz_4 = "main.rs";
        let rug_fuzz_5 = "Language not found";
        let rug_fuzz_6 = "main.py";
        let rug_fuzz_7 = "Language not found";
        let language_defs = vec![
            LanguageDefinition { name : LanguageId::from(rug_fuzz_0), extensions :
            vec![rug_fuzz_1.to_string()], first_line_match : Some(rug_fuzz_2
            .to_string()), scope : rug_fuzz_3.to_string(), default_config : None, },
            LanguageDefinition { name : LanguageId::from("python"), extensions :
            vec!["py".to_string()], first_line_match : Some("print('Hello, world!')"
            .to_string()), scope : "source.python".to_string(), default_config : None, }
        ];
        let result = Languages::new(&language_defs);
        debug_assert_eq!(result.named.len(), 2);
        debug_assert_eq!(result.extensions.len(), 2);
        let rust_language = result
            .language_for_path(&Path::new(rug_fuzz_4))
            .expect(rug_fuzz_5);
        debug_assert_eq!(rust_language.name, LanguageId::from("rust"));
        debug_assert_eq!(rust_language.extensions, vec!["rs".to_string()]);
        debug_assert_eq!(rust_language.first_line_match, Some("fn main()".to_string()));
        debug_assert_eq!(rust_language.scope, "source.rust".to_string());
        debug_assert_eq!(rust_language.default_config, None);
        let python_language = result
            .language_for_path(&Path::new(rug_fuzz_6))
            .expect(rug_fuzz_7);
        debug_assert_eq!(python_language.name, LanguageId::from("python"));
        debug_assert_eq!(python_language.extensions, vec!["py".to_string()]);
        debug_assert_eq!(
            python_language.first_line_match, Some("print('Hello, world!')".to_string())
        );
        debug_assert_eq!(python_language.scope, "source.python".to_string());
        debug_assert_eq!(python_language.default_config, None);
        let _rug_ed_tests_llm_16_744_rrrruuuugggg_test_new = 0;
    }
}
