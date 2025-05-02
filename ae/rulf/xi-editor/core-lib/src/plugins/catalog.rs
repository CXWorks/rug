//! Keeping track of available plugins.
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use super::{PluginDescription, PluginName};
use crate::config::table_from_toml_str;
use crate::syntax::Languages;
/// A catalog of all available plugins.
#[derive(Debug, Clone, Default)]
pub struct PluginCatalog {
    items: HashMap<PluginName, Arc<PluginDescription>>,
    locations: HashMap<PathBuf, Arc<PluginDescription>>,
}
/// Errors that can occur while trying to load a plugin.
#[derive(Debug)]
pub enum PluginLoadError {
    Io(io::Error),
    /// Malformed manifest
    Parse(toml::de::Error),
}
#[allow(dead_code)]
impl<'a> PluginCatalog {
    /// Loads any plugins discovered in these paths, replacing any existing
    /// plugins.
    pub fn reload_from_paths(&mut self, paths: &[PathBuf]) {
        self.items.clear();
        self.locations.clear();
        self.load_from_paths(paths);
    }
    /// Loads plugins from paths and adds them to existing plugins.
    pub fn load_from_paths(&mut self, paths: &[PathBuf]) {
        let all_manifests = find_all_manifests(paths);
        for manifest_path in &all_manifests {
            match load_manifest(manifest_path) {
                Err(e) => warn!("error loading plugin {:?}", e),
                Ok(manifest) => {
                    info!("loaded {}", manifest.name);
                    let manifest = Arc::new(manifest);
                    self.items.insert(manifest.name.clone(), manifest.clone());
                    self.locations.insert(manifest_path.clone(), manifest);
                }
            }
        }
    }
    pub fn make_languages_map(&self) -> Languages {
        let all_langs = self
            .items
            .values()
            .flat_map(|plug| plug.languages.iter().cloned())
            .collect::<Vec<_>>();
        Languages::new(all_langs.as_slice())
    }
    /// Returns an iterator over all plugins in the catalog, in arbitrary order.
    pub fn iter(&'a self) -> impl Iterator<Item = Arc<PluginDescription>> + 'a {
        self.items.values().cloned()
    }
    /// Returns an iterator over all plugin names in the catalog,
    /// in arbitrary order.
    pub fn iter_names(&'a self) -> impl Iterator<Item = &'a PluginName> {
        self.items.keys()
    }
    /// Returns the plugin located at the provided file path.
    pub fn get_from_path(&self, path: &PathBuf) -> Option<Arc<PluginDescription>> {
        self.items
            .values()
            .find(|&v| v.exec_path.to_str().unwrap().contains(path.to_str().unwrap()))
            .cloned()
    }
    /// Returns a reference to the named plugin if it exists in the catalog.
    pub fn get_named(&self, plugin_name: &str) -> Option<Arc<PluginDescription>> {
        self.items.get(plugin_name).map(Arc::clone)
    }
    /// Removes the named plugin.
    pub fn remove_named(&mut self, plugin_name: &str) {
        self.items.remove(plugin_name);
    }
}
fn find_all_manifests(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut manifest_paths = Vec::new();
    for path in paths.iter() {
        let manif_path = path.join("manifest.toml");
        if manif_path.exists() {
            manifest_paths.push(manif_path);
            continue;
        }
        let result = path
            .read_dir()
            .map(|dir| {
                dir.flat_map(|item| item.map(|p| p.path()).ok())
                    .map(|dir| dir.join("manifest.toml"))
                    .filter(|f| f.exists())
                    .for_each(|f| manifest_paths.push(f))
            });
        if let Err(e) = result {
            error!("error reading plugin path {:?}, {:?}", path, e);
        }
    }
    manifest_paths
}
fn load_manifest(path: &Path) -> Result<PluginDescription, PluginLoadError> {
    let mut file = fs::File::open(&path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let mut manifest: PluginDescription = toml::from_str(&contents)?;
    if manifest.exec_path.starts_with("./") {
        manifest
            .exec_path = path.parent().unwrap().join(manifest.exec_path).canonicalize()?;
    }
    for lang in &mut manifest.languages {
        let lang_config_path = path
            .parent()
            .unwrap()
            .join(&lang.name.as_ref())
            .with_extension("toml");
        if !lang_config_path.exists() {
            continue;
        }
        let lang_defaults = fs::read_to_string(&lang_config_path)?;
        let lang_defaults = table_from_toml_str(&lang_defaults)?;
        lang.default_config = Some(lang_defaults);
    }
    Ok(manifest)
}
impl From<io::Error> for PluginLoadError {
    fn from(err: io::Error) -> PluginLoadError {
        PluginLoadError::Io(err)
    }
}
impl From<toml::de::Error> for PluginLoadError {
    fn from(err: toml::de::Error) -> PluginLoadError {
        PluginLoadError::Parse(err)
    }
}
#[cfg(test)]
mod tests_llm_16_47 {
    use std::io;
    use toml;
    use crate::plugins::catalog::PluginLoadError;
    #[test]
    fn test_from() {
        let _rug_st_tests_llm_16_47_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = "Some error";
        let err = io::Error::new(io::ErrorKind::Other, rug_fuzz_0);
        let result = PluginLoadError::from(err);
        match result {
            PluginLoadError::Io(err) => {
                debug_assert_eq!(err.kind(), io::ErrorKind::Other)
            }
            _ => panic!("Unexpected error variant"),
        }
        let _rug_ed_tests_llm_16_47_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_619 {
    use super::*;
    use crate::*;
    use std::collections::HashMap;
    #[test]
    fn test_iter_names() {
        let _rug_st_tests_llm_16_619_rrrruuuugggg_test_iter_names = 0;
        let catalog = PluginCatalog {
            items: HashMap::new(),
            locations: HashMap::new(),
        };
        let names: Vec<&PluginName> = catalog.iter_names().collect();
        debug_assert_eq!(names.len(), 0);
        let _rug_ed_tests_llm_16_619_rrrruuuugggg_test_iter_names = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_624 {
    use super::*;
    use crate::*;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::Arc;
    use crate::annotations::AnnotationRange;
    use serde::{Deserialize, Serialize};
    #[test]
    fn test_reload_from_paths() {
        let _rug_st_tests_llm_16_624_rrrruuuugggg_test_reload_from_paths = 0;
        let rug_fuzz_0 = "path1";
        let mut plugin_catalog = PluginCatalog {
            items: HashMap::new(),
            locations: HashMap::new(),
        };
        let paths: Vec<PathBuf> = vec![rug_fuzz_0.into(), "path2".into()];
        plugin_catalog.reload_from_paths(&paths);
        debug_assert!(plugin_catalog.items.is_empty());
        debug_assert!(plugin_catalog.locations.is_empty());
        let _rug_ed_tests_llm_16_624_rrrruuuugggg_test_reload_from_paths = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_627 {
    use super::*;
    use crate::*;
    #[test]
    fn test_find_all_manifests() {
        let _rug_st_tests_llm_16_627_rrrruuuugggg_test_find_all_manifests = 0;
        let rug_fuzz_0 = "path1";
        let rug_fuzz_1 = "path2";
        let mut paths = Vec::new();
        paths.push(PathBuf::from(rug_fuzz_0));
        paths.push(PathBuf::from(rug_fuzz_1));
        let result = find_all_manifests(&paths);
        let _rug_ed_tests_llm_16_627_rrrruuuugggg_test_find_all_manifests = 0;
    }
}
