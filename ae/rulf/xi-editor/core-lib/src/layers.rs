//! Handles syntax highlighting and other styling.
//!
//! Plugins provide syntax highlighting information in the form of 'scopes'.
//! Scope information originating from any number of plugins can be resolved
//! into styles using a theme, augmented with additional style definitions.
use std::collections::{BTreeMap, HashMap, HashSet};
use syntect::highlighting::StyleModifier;
use syntect::parsing::Scope;
use xi_rope::spans::{Spans, SpansBuilder};
use xi_rope::{Interval, RopeDelta};
use xi_trace::trace_block;
use crate::plugins::PluginPid;
use crate::styles::{Style, ThemeStyleMap};
/// A collection of layers containing scope information.
#[derive(Default)]
pub struct Layers {
    layers: BTreeMap<PluginPid, ScopeLayer>,
    deleted: HashSet<PluginPid>,
    merged: Spans<Style>,
}
/// A collection of scope spans from a single source.
pub struct ScopeLayer {
    stack_lookup: Vec<Vec<Scope>>,
    style_lookup: Vec<Style>,
    /// style state of existing scope spans, so we can more efficiently
    /// compute styles of child spans.
    style_cache: HashMap<Vec<Scope>, StyleModifier>,
    /// Human readable scope names, for debugging
    scope_spans: Spans<u32>,
    style_spans: Spans<Style>,
}
impl Layers {
    pub fn get_merged(&self) -> &Spans<Style> {
        &self.merged
    }
    /// Adds the provided scopes to the layer's lookup table.
    pub fn add_scopes(
        &mut self,
        layer: PluginPid,
        scopes: Vec<Vec<String>>,
        style_map: &ThemeStyleMap,
    ) {
        let _t = trace_block("Layers::AddScopes", &["core"]);
        if self.create_if_missing(layer).is_err() {
            return;
        }
        self.layers.get_mut(&layer).unwrap().add_scopes(scopes, style_map);
    }
    /// Applies the delta to all layers, inserting empty intervals
    /// for any regions inserted in the delta.
    ///
    /// This is useful for clearing spans, and for updating spans
    /// as edits occur.
    pub fn update_all(&mut self, delta: &RopeDelta) {
        self.merged.apply_shape(delta);
        for layer in self.layers.values_mut() {
            layer.blank_scopes(delta);
        }
        let (iv, _len) = delta.summary();
        self.resolve_styles(iv);
    }
    /// Updates the scope spans for a given layer.
    pub fn update_layer(&mut self, layer: PluginPid, iv: Interval, spans: Spans<u32>) {
        if self.create_if_missing(layer).is_err() {
            return;
        }
        self.layers.get_mut(&layer).unwrap().update_scopes(iv, &spans);
        self.resolve_styles(iv);
    }
    /// Removes a given layer. This will remove all styles derived from
    /// that layer's scopes.
    pub fn remove_layer(&mut self, layer: PluginPid) -> Option<ScopeLayer> {
        self.deleted.insert(layer);
        let layer = self.layers.remove(&layer);
        if layer.is_some() {
            let iv_all = Interval::new(0, self.merged.len());
            self.merged = SpansBuilder::new(self.merged.len()).build();
            self.resolve_styles(iv_all);
        }
        layer
    }
    pub fn theme_changed(&mut self, style_map: &ThemeStyleMap) {
        for layer in self.layers.values_mut() {
            layer.theme_changed(style_map);
        }
        self.merged = SpansBuilder::new(self.merged.len()).build();
        let iv_all = Interval::new(0, self.merged.len());
        self.resolve_styles(iv_all);
    }
    /// Resolves styles from all layers for the given interval, updating
    /// the master style spans.
    fn resolve_styles(&mut self, iv: Interval) {
        if self.layers.is_empty() {
            return;
        }
        let mut layer_iter = self.layers.values();
        let mut resolved = layer_iter.next().unwrap().style_spans.subseq(iv);
        for other in layer_iter {
            let spans = other.style_spans.subseq(iv);
            assert_eq!(resolved.len(), spans.len());
            resolved = resolved
                .merge(
                    &spans,
                    |a, b| match b {
                        Some(b) => a.merge(b),
                        None => a.to_owned(),
                    },
                );
        }
        self.merged.edit(iv, resolved);
    }
    /// Prints scopes and style information for the given `Interval`.
    pub fn debug_print_spans(&self, iv: Interval) {
        for (id, layer) in &self.layers {
            let spans = layer.scope_spans.subseq(iv);
            let styles = layer.style_spans.subseq(iv);
            if spans.iter().next().is_some() {
                info!("scopes for layer {:?}:", id);
                for (iv, val) in spans.iter() {
                    info!("{}: {:?}", iv, layer.stack_lookup[* val as usize]);
                }
                info!("styles:");
                for (iv, val) in styles.iter() {
                    info!("{}: {:?}", iv, val);
                }
            }
        }
    }
    /// Returns an `Err` if this layer has been deleted; the caller should return.
    fn create_if_missing(&mut self, layer_id: PluginPid) -> Result<(), ()> {
        if self.deleted.contains(&layer_id) {
            return Err(());
        }
        if !self.layers.contains_key(&layer_id) {
            self.layers.insert(layer_id, ScopeLayer::new(self.merged.len()));
        }
        Ok(())
    }
}
impl Default for ScopeLayer {
    fn default() -> Self {
        ScopeLayer {
            stack_lookup: Vec::new(),
            style_lookup: Vec::new(),
            style_cache: HashMap::new(),
            scope_spans: Spans::default(),
            style_spans: Spans::default(),
        }
    }
}
impl ScopeLayer {
    pub fn new(len: usize) -> Self {
        ScopeLayer {
            stack_lookup: Vec::new(),
            style_lookup: Vec::new(),
            style_cache: HashMap::new(),
            scope_spans: SpansBuilder::new(len).build(),
            style_spans: SpansBuilder::new(len).build(),
        }
    }
    fn theme_changed(&mut self, style_map: &ThemeStyleMap) {
        let cur_stacks = self.stack_lookup.clone();
        self.style_lookup = self.styles_for_stacks(&cur_stacks, style_map);
        let iv_all = Interval::new(0, self.style_spans.len());
        self.style_spans = SpansBuilder::new(self.style_spans.len()).build();
        let scopes = self.scope_spans.clone();
        self.update_styles(iv_all, &scopes)
    }
    fn add_scopes(&mut self, scopes: Vec<Vec<String>>, style_map: &ThemeStyleMap) {
        let mut stacks = Vec::with_capacity(scopes.len());
        for stack in scopes {
            let scopes = stack
                .iter()
                .map(|s| Scope::new(&s))
                .filter(|result| match *result {
                    Err(ref err) => {
                        warn!(
                            "failed to resolve scope {}\nErr: {:?}", & stack.join(" "),
                            err
                        );
                        false
                    }
                    _ => true,
                })
                .map(|s| s.unwrap())
                .collect::<Vec<_>>();
            stacks.push(scopes);
        }
        let mut new_styles = self.styles_for_stacks(stacks.as_slice(), style_map);
        self.stack_lookup.append(&mut stacks);
        self.style_lookup.append(&mut new_styles);
    }
    fn styles_for_stacks(
        &mut self,
        stacks: &[Vec<Scope>],
        style_map: &ThemeStyleMap,
    ) -> Vec<Style> {
        let highlighter = style_map.get_highlighter();
        let mut new_styles = Vec::new();
        for stack in stacks {
            let mut last_style: Option<StyleModifier> = None;
            let mut upper_bound_of_last = stack.len() as usize;
            for i in 0..stack.len() - 1 {
                let prev_range = 0..stack.len() - (i + 1);
                if let Some(s) = self.style_cache.get(&stack[prev_range]) {
                    last_style = Some(*s);
                    upper_bound_of_last = stack.len() - (i + 1);
                    break;
                }
            }
            let mut base_style_mod = last_style.unwrap_or_default();
            for i in upper_bound_of_last..stack.len() {
                let style_mod = highlighter.style_mod_for_stack(&stack[0..=i]);
                base_style_mod = base_style_mod.apply(style_mod);
            }
            let style = Style::from_syntect_style_mod(&base_style_mod);
            self.style_cache.insert(stack.clone(), base_style_mod);
            new_styles.push(style);
        }
        new_styles
    }
    fn update_scopes(&mut self, iv: Interval, spans: &Spans<u32>) {
        self.scope_spans.edit(iv, spans.to_owned());
        self.update_styles(iv, spans);
    }
    /// Applies `delta`, which is presumed to contain empty spans.
    /// This is only used when we receive an edit, to adjust current span
    /// positions.
    fn blank_scopes(&mut self, delta: &RopeDelta) {
        self.style_spans.apply_shape(delta);
        self.scope_spans.apply_shape(delta);
    }
    /// Updates `self.style_spans`, mapping scopes to styles and combining
    /// adjacent and equal spans.
    fn update_styles(&mut self, iv: Interval, spans: &Spans<u32>) {
        let mut sb = SpansBuilder::new(spans.len());
        let mut spans_iter = spans.iter();
        let mut prev = spans_iter.next();
        {
            let style_eq = |i1: &u32, i2: &u32| {
                self.style_lookup[*i1 as usize] == self.style_lookup[*i2 as usize]
            };
            while let Some((p_iv, p_val)) = prev {
                match spans_iter.next() {
                    Some(
                        (n_iv, n_val),
                    ) if n_iv.start() == p_iv.end() && style_eq(p_val, n_val) => {
                        prev = Some((p_iv.union(n_iv), p_val));
                    }
                    other => {
                        sb.add_span(p_iv, self.style_lookup[*p_val as usize].to_owned());
                        prev = other;
                    }
                }
            }
        }
        self.style_spans.edit(iv, sb.build());
    }
}
#[cfg(test)]
mod tests_llm_16_468 {
    use super::*;
    use crate::*;
    use serde_json;
    #[test]
    fn test_debug_print_spans() {
        let _rug_st_tests_llm_16_468_rrrruuuugggg_test_debug_print_spans = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 10;
        let layers: Layers = Default::default();
        let iv = Interval::new(rug_fuzz_0, rug_fuzz_1);
        layers.debug_print_spans(iv);
        let _rug_ed_tests_llm_16_468_rrrruuuugggg_test_debug_print_spans = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_471 {
    use super::*;
    use crate::*;
    use crate::annotations::AnnotationRange;
    use crate::layers::Layers;
    use crate::plugins::PluginPid;
    use crate::styles::Style;
    #[test]
    fn test_remove_layer() {
        let _rug_st_tests_llm_16_471_rrrruuuugggg_test_remove_layer = 0;
        let rug_fuzz_0 = 1;
        let mut layers = Layers::default();
        let layer = PluginPid(rug_fuzz_0);
        let result = layers.remove_layer(layer);
        debug_assert!(layers.deleted.contains(& layer));
        debug_assert!(layers.layers.get(& layer).is_none());
        debug_assert!(result.is_none());
        let _rug_ed_tests_llm_16_471_rrrruuuugggg_test_remove_layer = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_474 {
    use super::*;
    use crate::*;
    #[test]
    fn test_theme_changed() {
        let _rug_st_tests_llm_16_474_rrrruuuugggg_test_theme_changed = 0;
        let mut layers = Layers::default();
        let style_map = ThemeStyleMap::new(None);
        layers.theme_changed(&style_map);
        let _rug_ed_tests_llm_16_474_rrrruuuugggg_test_theme_changed = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_486 {
    use super::*;
    use crate::*;
    use serde_json::json;
    use serde_json::Value;
    #[test]
    fn test_theme_changed() {
        let _rug_st_tests_llm_16_486_rrrruuuugggg_test_theme_changed = 0;
        let mut scope_layer = ScopeLayer::default();
        let style_map = ThemeStyleMap::new(None);
        scope_layer.theme_changed(&style_map);
        let expected_style_lookup = Vec::<Style>::new();
        debug_assert_eq!(scope_layer.style_lookup, expected_style_lookup);
        let _rug_ed_tests_llm_16_486_rrrruuuugggg_test_theme_changed = 0;
    }
}
