//! Management of styles.
use std::collections::{BTreeMap, HashMap, HashSet};
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::iter::FromIterator;
use std::path::{Path, PathBuf};
use serde_json::{self, Value};
use syntect::dumps::{dump_to_file, from_dump_file};
use syntect::highlighting::StyleModifier as SynStyleModifier;
use syntect::highlighting::{Color, Highlighter, Theme, ThemeSet};
use syntect::LoadingError;
pub use syntect::highlighting::ThemeSettings;
pub const N_RESERVED_STYLES: usize = 8;
const SYNTAX_PRIORITY_DEFAULT: u16 = 200;
const SYNTAX_PRIORITY_LOWEST: u16 = 0;
pub const DEFAULT_THEME: &str = "InspiredGitHub";
#[derive(Clone, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
/// A mergeable style. All values except priority are optional.
///
/// Note: A `None` value represents the absense of preference; in the case of
/// boolean options, `Some(false)` means that this style will override a lower
/// priority value in the same field.
pub struct Style {
    /// The priority of this style, in the range (0, 1000). Used to resolve
    /// conflicting fields when merging styles. The higher priority wins.
    #[serde(skip_serializing)]
    pub priority: u16,
    /// The foreground text color, in ARGB.
    pub fg_color: Option<u32>,
    /// The background text color, in ARGB.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bg_color: Option<u32>,
    /// The font-weight, in the range 100-900, interpreted like the CSS
    /// font-weight property.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub underline: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
}
impl Style {
    /// Creates a new `Style` by converting from a `Syntect::StyleModifier`.
    pub fn from_syntect_style_mod(style: &SynStyleModifier) -> Self {
        let font_style = style.font_style.map(|s| s.bits()).unwrap_or_default();
        let weight = if (font_style & 1) != 0 { Some(700) } else { None };
        let underline = if (font_style & 2) != 0 { Some(true) } else { None };
        let italic = if (font_style & 4) != 0 { Some(true) } else { None };
        Self::new(
            SYNTAX_PRIORITY_DEFAULT,
            style.foreground.map(Self::rgba_from_syntect_color),
            style.background.map(Self::rgba_from_syntect_color),
            weight,
            underline,
            italic,
        )
    }
    pub fn new<O32, O16, OB>(
        priority: u16,
        fg_color: O32,
        bg_color: O32,
        weight: O16,
        underline: OB,
        italic: OB,
    ) -> Self
    where
        O32: Into<Option<u32>>,
        O16: Into<Option<u16>>,
        OB: Into<Option<bool>>,
    {
        assert!(priority <= 1000);
        Style {
            priority,
            fg_color: fg_color.into(),
            bg_color: bg_color.into(),
            weight: weight.into(),
            underline: underline.into(),
            italic: italic.into(),
        }
    }
    /// Returns the default style for the given `Theme`.
    pub fn default_for_theme(theme: &Theme) -> Self {
        let fg = theme.settings.foreground.unwrap_or(Color::BLACK);
        Style::new(
            SYNTAX_PRIORITY_LOWEST,
            Some(Self::rgba_from_syntect_color(fg)),
            None,
            None,
            None,
            None,
        )
    }
    /// Creates a new style by combining attributes of `self` and `other`.
    /// If both styles define an attribute, the highest priority wins; `other`
    /// wins in the case of a tie.
    ///
    /// Note: when merging multiple styles, apply them in increasing priority.
    pub fn merge(&self, other: &Style) -> Style {
        let (p1, p2) = if self.priority > other.priority {
            (self, other)
        } else {
            (other, self)
        };
        Style::new(
            p1.priority,
            p1.fg_color.or(p2.fg_color),
            p1.bg_color.or(p2.bg_color),
            p1.weight.or(p2.weight),
            p1.underline.or(p2.underline),
            p1.italic.or(p2.italic),
        )
    }
    /// Encode this `Style`, setting the `id` property.
    ///
    /// Note: this should only be used when sending the `def_style` RPC.
    pub fn to_json(&self, id: usize) -> Value {
        let mut as_val = serde_json::to_value(self).expect("failed to encode style");
        as_val["id"] = id.into();
        as_val
    }
    fn rgba_from_syntect_color(color: Color) -> u32 {
        let Color { r, g, b, a } = color;
        ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
    }
}
/// A map from styles to client identifiers for a given `Theme`.
pub struct ThemeStyleMap {
    themes: ThemeSet,
    theme_name: String,
    theme: Theme,
    default_themes: Vec<String>,
    default_style: Style,
    map: HashMap<Style, usize>,
    path_map: BTreeMap<String, PathBuf>,
    styles: Vec<Style>,
    themes_dir: Option<PathBuf>,
    cache_dir: Option<PathBuf>,
    caching_enabled: bool,
}
impl ThemeStyleMap {
    pub fn new(themes_dir: Option<PathBuf>) -> ThemeStyleMap {
        let themes = ThemeSet::load_defaults();
        let theme_name = DEFAULT_THEME.to_owned();
        let theme = themes.themes.get(&theme_name).expect("missing theme").to_owned();
        let default_themes = themes.themes.keys().cloned().collect();
        let default_style = Style::default_for_theme(&theme);
        let cache_dir = None;
        let caching_enabled = true;
        ThemeStyleMap {
            themes,
            theme_name,
            theme,
            default_themes,
            default_style,
            map: HashMap::new(),
            path_map: BTreeMap::new(),
            styles: Vec::new(),
            themes_dir,
            cache_dir,
            caching_enabled,
        }
    }
    pub fn get_default_style(&self) -> &Style {
        &self.default_style
    }
    pub fn get_highlighter(&self) -> Highlighter {
        Highlighter::new(&self.theme)
    }
    pub fn get_theme_name(&self) -> &str {
        &self.theme_name
    }
    pub fn get_theme_settings(&self) -> &ThemeSettings {
        &self.theme.settings
    }
    pub fn get_theme_names(&self) -> Vec<String> {
        self.path_map.keys().chain(self.default_themes.iter()).cloned().collect()
    }
    pub fn contains_theme(&self, k: &str) -> bool {
        self.themes.themes.contains_key(k)
    }
    pub fn set_theme(&mut self, theme_name: &str) -> Result<(), &'static str> {
        match self.load_theme(theme_name) {
            Ok(()) => {
                if let Some(new_theme) = self.themes.themes.get(theme_name) {
                    self.theme = new_theme.to_owned();
                    self.theme_name = theme_name.to_owned();
                    self.default_style = Style::default_for_theme(&self.theme);
                    self.map = HashMap::new();
                    self.styles = Vec::new();
                    Ok(())
                } else {
                    Err("unknown theme")
                }
            }
            Err(e) => {
                error!(
                    "Encountered error {:?} while trying to load {:?}", e, theme_name
                );
                Err("could not load theme")
            }
        }
    }
    pub fn merge_with_default(&self, style: &Style) -> Style {
        self.default_style.merge(style)
    }
    pub fn lookup(&self, style: &Style) -> Option<usize> {
        self.map.get(style).cloned()
    }
    pub fn add(&mut self, style: &Style) -> usize {
        let result = self.styles.len() + N_RESERVED_STYLES;
        self.map.insert(style.clone(), result);
        self.styles.push(style.clone());
        result
    }
    /// Delete key and the corresponding dump file from the themes map.
    pub(crate) fn remove_theme(&mut self, path: &Path) -> Option<String> {
        validate_theme_file(path).ok()?;
        let theme_name = path.file_stem().and_then(OsStr::to_str)?;
        self.themes.themes.remove(theme_name);
        self.path_map.remove(theme_name);
        let dump_p = self.get_dump_path(theme_name)?;
        if dump_p.exists() {
            let _ = fs::remove_file(dump_p);
        }
        Some(theme_name.to_string())
    }
    /// Cache all themes names and their paths inside the given directory.
    pub(crate) fn load_theme_dir(&mut self) {
        if let Some(themes_dir) = self.themes_dir.clone() {
            match ThemeSet::discover_theme_paths(themes_dir) {
                Ok(themes) => {
                    self.caching_enabled = self.caching_enabled && self.init_cache_dir();
                    for theme_p in &themes {
                        match self.load_theme_info_from_path(theme_p) {
                            Ok(_) => {}
                            Err(e) => {
                                error!(
                                    "Encountered error {:?} loading theme at {:?}", e, theme_p
                                )
                            }
                        }
                    }
                }
                Err(e) => error!("Error loading themes dir: {:?}", e),
            }
        }
    }
    /// A wrapper around `from_dump_file`
    /// to validate the state of dump file.
    /// Invalidates if mod time of dump is less
    /// than the original one.
    fn try_load_from_dump(&self, theme_p: &Path) -> Option<(String, Theme)> {
        if !self.caching_enabled {
            return None;
        }
        let theme_name = theme_p.file_stem().and_then(OsStr::to_str)?;
        let dump_p = self.get_dump_path(theme_name)?;
        if !&dump_p.exists() {
            return None;
        }
        let mod_t = fs::metadata(&dump_p).and_then(|md| md.modified()).ok()?;
        let mod_t_orig = fs::metadata(theme_p).and_then(|md| md.modified()).ok()?;
        if mod_t >= mod_t_orig {
            from_dump_file(&dump_p).ok().map(|t| (theme_name.to_owned(), t))
        } else {
            let _ = fs::remove_file(&dump_p);
            None
        }
    }
    /// Loads a theme's name and its respective path into the theme path map.
    pub(crate) fn load_theme_info_from_path(
        &mut self,
        theme_p: &Path,
    ) -> Result<String, LoadingError> {
        validate_theme_file(theme_p)?;
        let theme_name = theme_p
            .file_stem()
            .and_then(OsStr::to_str)
            .ok_or(LoadingError::BadPath)?;
        self.path_map.insert(theme_name.to_string(), theme_p.to_path_buf());
        Ok(theme_name.to_owned())
    }
    /// Loads theme using syntect's `get_theme` fn to our `theme` path map.
    /// Stores binary dump in a file with `tmdump` extension, only if
    /// caching is enabled.
    fn load_theme(&mut self, theme_name: &str) -> Result<(), LoadingError> {
        if self.contains_theme(theme_name) && self.get_theme_name() != theme_name {
            return Ok(());
        }
        let theme_p = &self.path_map.get(theme_name).cloned();
        if let Some(theme_p) = theme_p {
            match self.try_load_from_dump(theme_p) {
                Some((dump_theme_name, dump_theme_data)) => {
                    self.insert_to_map(dump_theme_name, dump_theme_data);
                }
                None => {
                    let theme = ThemeSet::get_theme(theme_p)?;
                    if self.caching_enabled {
                        if let Some(dump_p) = self.get_dump_path(theme_name) {
                            let _ = dump_to_file(&theme, dump_p);
                        }
                    }
                    self.insert_to_map(theme_name.to_owned(), theme);
                }
            }
            Ok(())
        } else {
            Err(LoadingError::BadPath)
        }
    }
    fn insert_to_map(&mut self, k: String, v: Theme) {
        self.themes.themes.insert(k, v);
    }
    /// Returns dump's path corresponding to the given theme name.
    fn get_dump_path(&self, theme_name: &str) -> Option<PathBuf> {
        self.cache_dir.as_ref().map(|p| p.join(theme_name).with_extension("tmdump"))
    }
    /// Compare the stored file paths in `self.state`
    /// to the present ones.
    pub(crate) fn sync_dir(&mut self, dir: Option<&Path>) {
        if let Some(themes_dir) = dir {
            if let Ok(paths) = ThemeSet::discover_theme_paths(themes_dir) {
                let current_state: HashSet<PathBuf> = HashSet::from_iter(
                    paths.into_iter(),
                );
                let maintained_state: HashSet<PathBuf> = HashSet::from_iter(
                    self.path_map.values().cloned(),
                );
                let to_insert = current_state.difference(&maintained_state);
                for path in to_insert {
                    let _ = self.load_theme_info_from_path(path);
                }
                let to_remove = maintained_state.difference(&current_state);
                for path in to_remove {
                    self.remove_theme(path);
                }
            }
        }
    }
    /// Creates the cache dir returns true
    /// if it is successfully initialized or
    /// already exists.
    fn init_cache_dir(&mut self) -> bool {
        self.cache_dir = self.themes_dir.clone().map(|p| p.join("cache"));
        if let Some(ref p) = self.cache_dir {
            if p.exists() {
                return true;
            }
            fs::DirBuilder::new().create(&p).is_ok()
        } else {
            false
        }
    }
}
/// Used to remove files with extension other than `tmTheme`.
fn validate_theme_file(path: &Path) -> Result<bool, LoadingError> {
    path.extension().map(|e| e != "tmTheme").ok_or(LoadingError::BadPath)
}
impl fmt::Debug for Style {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fn fmt_color(f: &mut fmt::Formatter, c: Option<u32>) -> fmt::Result {
            if let Some(c) = c { write!(f, "#{:X}", c) } else { write!(f, "None") }
        }
        write!(f, "Style( P{}, fg: ", self.priority)?;
        fmt_color(f, self.fg_color)?;
        write!(f, " bg: ")?;
        fmt_color(f, self.bg_color)?;
        if let Some(w) = self.weight {
            write!(f, " weight {}", w)?;
        }
        if let Some(i) = self.italic {
            write!(f, " ital: {}", i)?;
        }
        if let Some(u) = self.underline {
            write!(f, " uline: {}", u)?;
        }
        write!(f, " )")
    }
}
#[cfg(test)]
mod tests_llm_16_696 {
    use super::*;
    use crate::*;
    #[test]
    fn test_merge() {
        let _rug_st_tests_llm_16_696_rrrruuuugggg_test_merge = 0;
        let rug_fuzz_0 = 100;
        let rug_fuzz_1 = 0xFF0000;
        let rug_fuzz_2 = 0x0000FF;
        let rug_fuzz_3 = 700;
        let rug_fuzz_4 = false;
        let rug_fuzz_5 = true;
        let rug_fuzz_6 = 200;
        let rug_fuzz_7 = 0x00FF00;
        let rug_fuzz_8 = 0xFF00FF;
        let rug_fuzz_9 = true;
        let rug_fuzz_10 = 200;
        let rug_fuzz_11 = 0x00FF00;
        let rug_fuzz_12 = 0xFF00FF;
        let rug_fuzz_13 = 700;
        let rug_fuzz_14 = false;
        let rug_fuzz_15 = true;
        let style1 = Style {
            priority: rug_fuzz_0,
            fg_color: Some(rug_fuzz_1),
            bg_color: Some(rug_fuzz_2),
            weight: Some(rug_fuzz_3),
            underline: Some(rug_fuzz_4),
            italic: Some(rug_fuzz_5),
        };
        let style2 = Style {
            priority: rug_fuzz_6,
            fg_color: Some(rug_fuzz_7),
            bg_color: Some(rug_fuzz_8),
            weight: None,
            underline: None,
            italic: Some(rug_fuzz_9),
        };
        let expected = Style {
            priority: rug_fuzz_10,
            fg_color: Some(rug_fuzz_11),
            bg_color: Some(rug_fuzz_12),
            weight: Some(rug_fuzz_13),
            underline: Some(rug_fuzz_14),
            italic: Some(rug_fuzz_15),
        };
        let result = style1.merge(&style2);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_696_rrrruuuugggg_test_merge = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_701 {
    use super::*;
    use crate::*;
    use serde_json::json;
    #[test]
    fn test_to_json() {
        let _rug_st_tests_llm_16_701_rrrruuuugggg_test_to_json = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 0xFF0000;
        let rug_fuzz_2 = 0x0000FF;
        let rug_fuzz_3 = 700;
        let rug_fuzz_4 = false;
        let rug_fuzz_5 = true;
        let rug_fuzz_6 = 10;
        let style = Style {
            priority: rug_fuzz_0,
            fg_color: Some(rug_fuzz_1),
            bg_color: Some(rug_fuzz_2),
            weight: Some(rug_fuzz_3),
            underline: Some(rug_fuzz_4),
            italic: Some(rug_fuzz_5),
        };
        let id = rug_fuzz_6;
        let expected = json!(
            { "priority" : 1, "fg_color" : 0xFF0000, "bg_color" : 0x0000FF, "weight" :
            700, "underline" : false, "italic" : true, "id" : 10 }
        );
        let result = style.to_json(id);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_701_rrrruuuugggg_test_to_json = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_702 {
    use super::*;
    use crate::*;
    use serde_json::json;
    #[test]
    fn test_add() {
        let _rug_st_tests_llm_16_702_rrrruuuugggg_test_add = 0;
        let rug_fuzz_0 = 100;
        let rug_fuzz_1 = 0xFF0000;
        let rug_fuzz_2 = 0x0000FF;
        let rug_fuzz_3 = 500;
        let rug_fuzz_4 = true;
        let rug_fuzz_5 = false;
        let mut style_map = ThemeStyleMap::new(None);
        let style = Style {
            priority: rug_fuzz_0,
            fg_color: Some(rug_fuzz_1),
            bg_color: Some(rug_fuzz_2),
            weight: Some(rug_fuzz_3),
            underline: Some(rug_fuzz_4),
            italic: Some(rug_fuzz_5),
        };
        let expected_result = style_map.styles.len() + N_RESERVED_STYLES;
        let result = style_map.add(&style);
        debug_assert_eq!(result, expected_result);
        debug_assert_eq!(style_map.map.get(& style).cloned(), Some(result));
        debug_assert_eq!(style_map.styles.len(), expected_result);
        debug_assert_eq!(style_map.styles[result - N_RESERVED_STYLES], style);
        let _rug_ed_tests_llm_16_702_rrrruuuugggg_test_add = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_703 {
    use super::*;
    use crate::*;
    use serde_json::{json, Value};
    #[test]
    fn test_contains_theme() {
        let _rug_st_tests_llm_16_703_rrrruuuugggg_test_contains_theme = 0;
        let rug_fuzz_0 = "theme1";
        let rug_fuzz_1 = "theme1";
        let rug_fuzz_2 = "theme2";
        let mut theme_style_map = ThemeStyleMap::new(None);
        let theme = Theme::default();
        theme_style_map.insert_to_map(rug_fuzz_0.to_owned(), theme);
        let result = theme_style_map.contains_theme(rug_fuzz_1);
        debug_assert_eq!(result, true);
        let result = theme_style_map.contains_theme(rug_fuzz_2);
        debug_assert_eq!(result, false);
        let _rug_ed_tests_llm_16_703_rrrruuuugggg_test_contains_theme = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_704 {
    use super::*;
    use crate::*;
    use serde_json::json;
    #[test]
    fn test_get_default_style() {
        let _rug_st_tests_llm_16_704_rrrruuuugggg_test_get_default_style = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let mut theme_style_map = ThemeStyleMap::new(None);
        let style = Style {
            priority: rug_fuzz_0,
            fg_color: Some(rug_fuzz_1),
            bg_color: Some(rug_fuzz_2),
            weight: None,
            underline: None,
            italic: None,
        };
        let expected_style = Style {
            priority: rug_fuzz_3,
            fg_color: Some(rug_fuzz_4),
            bg_color: Some(rug_fuzz_5),
            weight: None,
            underline: None,
            italic: None,
        };
        let default_style = theme_style_map.get_default_style();
        debug_assert_eq!(* default_style, expected_style);
        let _rug_ed_tests_llm_16_704_rrrruuuugggg_test_get_default_style = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_709 {
    use super::*;
    use crate::*;
    use serde_json;
    #[test]
    fn test_get_theme_name() {
        let _rug_st_tests_llm_16_709_rrrruuuugggg_test_get_theme_name = 0;
        let mut theme_style_map = ThemeStyleMap::new(None);
        theme_style_map.load_theme_dir();
        let theme_name = theme_style_map.get_theme_name();
        debug_assert_eq!(theme_name, "default");
        let _rug_ed_tests_llm_16_709_rrrruuuugggg_test_get_theme_name = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_714 {
    use super::*;
    use crate::*;
    use std::path::Path;
    #[test]
    fn test_init_cache_dir() {
        let _rug_st_tests_llm_16_714_rrrruuuugggg_test_init_cache_dir = 0;
        let rug_fuzz_0 = "cache";
        let mut theme_style_map = ThemeStyleMap::new(None);
        let result = theme_style_map.init_cache_dir();
        debug_assert_eq!(result, true);
        let expected_cache_dir = theme_style_map
            .themes_dir
            .clone()
            .map(|p| p.join(rug_fuzz_0));
        debug_assert_eq!(theme_style_map.cache_dir, expected_cache_dir);
        let _rug_ed_tests_llm_16_714_rrrruuuugggg_test_init_cache_dir = 0;
    }
    #[test]
    fn test_init_cache_dir_when_cache_dir_exists() {
        let _rug_st_tests_llm_16_714_rrrruuuugggg_test_init_cache_dir_when_cache_dir_exists = 0;
        let rug_fuzz_0 = "cache";
        let mut theme_style_map = ThemeStyleMap::new(None);
        let cache_dir = theme_style_map.themes_dir.clone().map(|p| p.join(rug_fuzz_0));
        if let Some(ref p) = cache_dir {
            fs::DirBuilder::new().create(&p).unwrap();
        }
        let result = theme_style_map.init_cache_dir();
        debug_assert_eq!(result, true);
        let _rug_ed_tests_llm_16_714_rrrruuuugggg_test_init_cache_dir_when_cache_dir_exists = 0;
    }
    #[test]
    fn test_init_cache_dir_when_cache_dir_does_not_exist() {
        let _rug_st_tests_llm_16_714_rrrruuuugggg_test_init_cache_dir_when_cache_dir_does_not_exist = 0;
        let rug_fuzz_0 = "cache";
        let mut theme_style_map = ThemeStyleMap::new(None);
        let cache_dir = theme_style_map.themes_dir.clone().map(|p| p.join(rug_fuzz_0));
        if let Some(ref p) = cache_dir {
            let _ = fs::remove_dir_all(&p);
        }
        let result = theme_style_map.init_cache_dir();
        debug_assert_eq!(result, true);
        let _rug_ed_tests_llm_16_714_rrrruuuugggg_test_init_cache_dir_when_cache_dir_does_not_exist = 0;
    }
    #[test]
    fn test_init_cache_dir_when_themes_dir_is_none() {
        let _rug_st_tests_llm_16_714_rrrruuuugggg_test_init_cache_dir_when_themes_dir_is_none = 0;
        let mut theme_style_map = ThemeStyleMap::new(None);
        theme_style_map.themes_dir = None;
        let result = theme_style_map.init_cache_dir();
        debug_assert_eq!(result, false);
        debug_assert_eq!(theme_style_map.cache_dir, None);
        let _rug_ed_tests_llm_16_714_rrrruuuugggg_test_init_cache_dir_when_themes_dir_is_none = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_725 {
    use crate::styles::Style;
    use crate::styles::ThemeStyleMap;
    #[test]
    fn test_merge_with_default() {
        let _rug_st_tests_llm_16_725_rrrruuuugggg_test_merge_with_default = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 100;
        let rug_fuzz_4 = true;
        let rug_fuzz_5 = true;
        let rug_fuzz_6 = 2;
        let rug_fuzz_7 = 255;
        let rug_fuzz_8 = 255;
        let rug_fuzz_9 = 700;
        let rug_fuzz_10 = true;
        let rug_fuzz_11 = false;
        let rug_fuzz_12 = 1;
        let rug_fuzz_13 = 0;
        let rug_fuzz_14 = 255;
        let rug_fuzz_15 = 100;
        let rug_fuzz_16 = true;
        let rug_fuzz_17 = true;
        let rug_fuzz_18 = 2;
        let rug_fuzz_19 = 255;
        let rug_fuzz_20 = 255;
        let rug_fuzz_21 = 700;
        let rug_fuzz_22 = true;
        let rug_fuzz_23 = false;
        let theme_style_map = ThemeStyleMap::new(None);
        let style_with_priority_1 = Style {
            priority: rug_fuzz_0,
            fg_color: Some(rug_fuzz_1),
            bg_color: Some(rug_fuzz_2),
            weight: Some(rug_fuzz_3),
            underline: Some(rug_fuzz_4),
            italic: Some(rug_fuzz_5),
        };
        let style_with_priority_2 = Style {
            priority: rug_fuzz_6,
            fg_color: Some(rug_fuzz_7),
            bg_color: Some(rug_fuzz_8),
            weight: Some(rug_fuzz_9),
            underline: Some(rug_fuzz_10),
            italic: Some(rug_fuzz_11),
        };
        let expected_result = Style {
            priority: rug_fuzz_12,
            fg_color: Some(rug_fuzz_13),
            bg_color: Some(rug_fuzz_14),
            weight: Some(rug_fuzz_15),
            underline: Some(rug_fuzz_16),
            italic: Some(rug_fuzz_17),
        };
        let actual_result = theme_style_map.merge_with_default(&style_with_priority_1);
        debug_assert_eq!(expected_result, actual_result);
        let expected_result = Style {
            priority: rug_fuzz_18,
            fg_color: Some(rug_fuzz_19),
            bg_color: Some(rug_fuzz_20),
            weight: Some(rug_fuzz_21),
            underline: Some(rug_fuzz_22),
            italic: Some(rug_fuzz_23),
        };
        let actual_result = theme_style_map.merge_with_default(&style_with_priority_2);
        debug_assert_eq!(expected_result, actual_result);
        let _rug_ed_tests_llm_16_725_rrrruuuugggg_test_merge_with_default = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_728 {
    use super::*;
    use crate::*;
    use std::path::PathBuf;
    #[test]
    fn test_remove_theme() {
        let _rug_st_tests_llm_16_728_rrrruuuugggg_test_remove_theme = 0;
        let rug_fuzz_0 = "test_theme.tmTheme";
        let mut theme_style_map = ThemeStyleMap::new(None);
        let path = Path::new(rug_fuzz_0);
        let result = theme_style_map.remove_theme(path);
        debug_assert_eq!(result, Some("test_theme".to_string()));
        let _rug_ed_tests_llm_16_728_rrrruuuugggg_test_remove_theme = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_731 {
    use super::*;
    use crate::*;
    use std::path::Path;
    #[test]
    fn test_sync_dir() {
        let _rug_st_tests_llm_16_731_rrrruuuugggg_test_sync_dir = 0;
        let rug_fuzz_0 = "theme1";
        let rug_fuzz_1 = "/path/to/theme1";
        let rug_fuzz_2 = "theme2";
        let rug_fuzz_3 = "/path/to/theme2";
        let rug_fuzz_4 = "theme3";
        let rug_fuzz_5 = "/path/to/theme3";
        let rug_fuzz_6 = "/path/to/themes";
        let rug_fuzz_7 = "/path/to/themes/theme1";
        let mut theme_style_map = ThemeStyleMap::new(None);
        theme_style_map
            .path_map
            .insert(rug_fuzz_0.to_owned(), PathBuf::from(rug_fuzz_1));
        theme_style_map
            .path_map
            .insert(rug_fuzz_2.to_owned(), PathBuf::from(rug_fuzz_3));
        theme_style_map
            .path_map
            .insert(rug_fuzz_4.to_owned(), PathBuf::from(rug_fuzz_5));
        let themes_dir = Path::new(rug_fuzz_6);
        theme_style_map.sync_dir(Some(themes_dir));
        let expected_state: HashSet<PathBuf> = vec![
            PathBuf::from(rug_fuzz_7), PathBuf::from("/path/to/themes/theme2"),
            PathBuf::from("/path/to/themes/theme3")
        ]
            .into_iter()
            .collect();
        let maintained_state: HashSet<PathBuf> = theme_style_map
            .path_map
            .values()
            .cloned()
            .collect();
        debug_assert_eq!(maintained_state, expected_state);
        let _rug_ed_tests_llm_16_731_rrrruuuugggg_test_sync_dir = 0;
    }
}
