//! A sample plugin, intended as an illustration and a template for plugin
//! developers.
extern crate xi_core_lib as xi_core;
extern crate xi_plugin_lib;
extern crate xi_rope;
use std::path::Path;
use crate::xi_core::ConfigTable;
use xi_plugin_lib::{mainloop, ChunkCache, Error, Plugin, View};
use xi_rope::delta::Builder as EditBuilder;
use xi_rope::interval::Interval;
use xi_rope::rope::RopeDelta;
/// A type that implements the `Plugin` trait, and interacts with xi-core.
///
/// Currently, this plugin has a single noteworthy behaviour,
/// intended to demonstrate how to edit a document; when the plugin is active,
/// and the user inserts an exclamation mark, the plugin will capitalize the
/// preceding word.
struct SamplePlugin;
impl Plugin for SamplePlugin {
    type Cache = ChunkCache;
    fn new_view(&mut self, view: &mut View<Self::Cache>) {
        eprintln!("new view {}", view.get_id());
    }
    fn did_close(&mut self, view: &View<Self::Cache>) {
        eprintln!("close view {}", view.get_id());
    }
    fn did_save(&mut self, view: &mut View<Self::Cache>, _old: Option<&Path>) {
        eprintln!("saved view {}", view.get_id());
    }
    fn config_changed(&mut self, _view: &mut View<Self::Cache>, _changes: &ConfigTable) {}
    fn update(
        &mut self,
        view: &mut View<Self::Cache>,
        delta: Option<&RopeDelta>,
        _edit_type: String,
        _author: String,
    ) {
        if let Some(delta) = delta {
            let (iv, _) = delta.summary();
            let text: String = delta
                .as_simple_insert()
                .map(String::from)
                .unwrap_or_default();
            if text == "!" {
                let _ = self.capitalize_word(view, iv.end());
            }
        }
    }
}
impl SamplePlugin {
    /// Uppercases the word preceding `end_offset`.
    fn capitalize_word(
        &self,
        view: &mut View<ChunkCache>,
        end_offset: usize,
    ) -> Result<(), Error> {
        let line_nb = view.line_of_offset(end_offset)?;
        let line_start = view.offset_of_line(line_nb)?;
        let mut cur_utf8_ix = 0;
        let mut word_start = 0;
        for c in view.get_line(line_nb)?.chars() {
            if c.is_whitespace() {
                word_start = cur_utf8_ix;
            }
            cur_utf8_ix += c.len_utf8();
            if line_start + cur_utf8_ix == end_offset {
                break;
            }
        }
        let new_text = view
            .get_line(line_nb)?[word_start..end_offset - line_start]
            .to_uppercase();
        let buf_size = view.get_buf_size();
        let mut builder = EditBuilder::new(buf_size);
        let iv = Interval::new(line_start + word_start, end_offset);
        builder.replace(iv, new_text.into());
        view.edit(builder.build(), 0, false, true, "sample".into());
        Ok(())
    }
}
fn main() {
    let mut plugin = SamplePlugin;
    mainloop(&mut plugin).unwrap();
}
#[cfg(test)]
mod tests_llm_16_13 {
    use super::*;
    use crate::*;
    #[test]
    fn test_main() {
        let _rug_st_tests_llm_16_13_rrrruuuugggg_test_main = 0;
        let mut plugin = SamplePlugin;
        mainloop(&mut plugin).unwrap();
        let _rug_ed_tests_llm_16_13_rrrruuuugggg_test_main = 0;
    }
}
