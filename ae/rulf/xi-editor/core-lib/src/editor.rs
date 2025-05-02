use std::borrow::{Borrow, Cow};
use std::cmp::min;
use std::collections::BTreeSet;
use serde_json::Value;
use xi_rope::diff::{Diff, LineHashDiff};
use xi_rope::engine::{Engine, RevId, RevToken};
use xi_rope::rope::count_newlines;
use xi_rope::spans::SpansBuilder;
use xi_rope::{DeltaBuilder, Interval, LinesMetric, Rope, RopeDelta, Transformer};
use xi_trace::{trace_block, trace_payload};
use crate::annotations::{AnnotationType, Annotations};
use crate::config::BufferItems;
use crate::edit_ops::{self, IndentDirection};
use crate::edit_types::BufferEvent;
use crate::event_context::MAX_SIZE_LIMIT;
use crate::layers::Layers;
use crate::line_offset::{LineOffset, LogicalLines};
use crate::movement::Movement;
use crate::plugins::rpc::{DataSpan, GetDataResponse, PluginEdit, ScopeSpan, TextUnit};
use crate::plugins::PluginId;
use crate::rpc::SelectionModifier;
use crate::selection::{InsertDrift, SelRegion, Selection};
use crate::styles::ThemeStyleMap;
use crate::view::{Replace, View};
#[cfg(not(feature = "ledger"))]
pub struct SyncStore;
#[cfg(feature = "ledger")]
use fuchsia::sync::SyncStore;
const MAX_UNDOS: usize = 20;
pub struct Editor {
    /// The contents of the buffer.
    text: Rope,
    /// The CRDT engine, which tracks edit history and manages concurrent edits.
    engine: Engine,
    /// The most recent revision.
    last_rev_id: RevId,
    /// The revision of the last save.
    pristine_rev_id: RevId,
    undo_group_id: usize,
    /// Undo groups that may still be toggled
    live_undos: Vec<usize>,
    /// The index of the current undo; subsequent undos are currently 'undone'
    /// (but may be redone)
    cur_undo: usize,
    /// undo groups that are undone
    undos: BTreeSet<usize>,
    /// undo groups that are no longer live and should be gc'ed
    gc_undos: BTreeSet<usize>,
    force_undo_group: bool,
    this_edit_type: EditType,
    last_edit_type: EditType,
    revs_in_flight: usize,
    /// Used only on Fuchsia for syncing
    #[allow(dead_code)]
    sync_store: Option<SyncStore>,
    #[allow(dead_code)]
    last_synced_rev: RevId,
    layers: Layers,
}
impl Editor {
    /// Creates a new `Editor` with a new empty buffer.
    pub fn new() -> Editor {
        Self::with_text("")
    }
    /// Creates a new `Editor`, loading text into a new buffer.
    pub fn with_text<T: Into<Rope>>(text: T) -> Editor {
        let engine = Engine::new(text.into());
        let buffer = engine.get_head().clone();
        let last_rev_id = engine.get_head_rev_id();
        Editor {
            text: buffer,
            engine,
            last_rev_id,
            pristine_rev_id: last_rev_id,
            undo_group_id: 1,
            live_undos: vec![0],
            cur_undo: 1,
            undos: BTreeSet::new(),
            gc_undos: BTreeSet::new(),
            force_undo_group: false,
            last_edit_type: EditType::Other,
            this_edit_type: EditType::Other,
            layers: Layers::default(),
            revs_in_flight: 0,
            sync_store: None,
            last_synced_rev: last_rev_id,
        }
    }
    pub(crate) fn get_buffer(&self) -> &Rope {
        &self.text
    }
    pub(crate) fn get_layers(&self) -> &Layers {
        &self.layers
    }
    pub(crate) fn get_layers_mut(&mut self) -> &mut Layers {
        &mut self.layers
    }
    pub(crate) fn get_head_rev_token(&self) -> u64 {
        self.engine.get_head_rev_id().token()
    }
    pub(crate) fn get_edit_type(&self) -> EditType {
        self.this_edit_type
    }
    pub(crate) fn get_active_undo_group(&self) -> usize {
        *self.live_undos.last().unwrap_or(&0)
    }
    pub(crate) fn update_edit_type(&mut self) {
        self.last_edit_type = self.this_edit_type;
        self.this_edit_type = EditType::Other;
    }
    pub(crate) fn set_pristine(&mut self) {
        self.pristine_rev_id = self.engine.get_head_rev_id();
    }
    pub(crate) fn is_pristine(&self) -> bool {
        self.engine
            .is_equivalent_revision(self.pristine_rev_id, self.engine.get_head_rev_id())
    }
    /// Set whether or not edits are forced into the same undo group rather than being split by
    /// their EditType.
    ///
    /// This is used for things such as recording playback, where you don't want the
    /// individual events to be undoable, but instead the entire playback should be.
    pub(crate) fn set_force_undo_group(&mut self, force_undo_group: bool) {
        trace_payload(
            "Editor::set_force_undo_group",
            &["core"],
            force_undo_group.to_string(),
        );
        self.force_undo_group = force_undo_group;
    }
    /// Sets this Editor's contents to `text`, preserving undo state and cursor
    /// position when possible.
    pub fn reload(&mut self, text: Rope) {
        let delta = LineHashDiff::compute_delta(self.get_buffer(), &text);
        self.add_delta(delta);
        self.set_pristine();
    }
    pub fn increment_revs_in_flight(&mut self) {
        self.revs_in_flight += 1;
    }
    pub fn dec_revs_in_flight(&mut self) {
        self.revs_in_flight -= 1;
        self.gc_undos();
    }
    /// Applies a delta to the text, and updates undo state.
    ///
    /// Records the delta into the CRDT engine so that it can be undone. Also
    /// contains the logic for merging edits into the same undo group. At call
    /// time, self.this_edit_type should be set appropriately.
    ///
    /// This method can be called multiple times, accumulating deltas that will
    /// be committed at once with `commit_delta`. Note that it does not update
    /// the views. Thus, view-associated state such as the selection and line
    /// breaks are to be considered invalid after this method, until the
    /// `commit_delta` call.
    fn add_delta(&mut self, delta: RopeDelta) {
        let head_rev_id = self.engine.get_head_rev_id();
        let undo_group = self.calculate_undo_group();
        self.last_edit_type = self.this_edit_type;
        let priority = 0x10000;
        self.engine.edit_rev(priority, undo_group, head_rev_id.token(), delta);
        self.text = self.engine.get_head().clone();
    }
    pub(crate) fn calculate_undo_group(&mut self) -> usize {
        let has_undos = !self.live_undos.is_empty();
        let force_undo_group = self.force_undo_group;
        let is_unbroken_group = !self
            .this_edit_type
            .breaks_undo_group(self.last_edit_type);
        if has_undos && (force_undo_group || is_unbroken_group) {
            *self.live_undos.last().unwrap()
        } else {
            let undo_group = self.undo_group_id;
            self.gc_undos.extend(&self.live_undos[self.cur_undo..]);
            self.live_undos.truncate(self.cur_undo);
            self.live_undos.push(undo_group);
            if self.live_undos.len() <= MAX_UNDOS {
                self.cur_undo += 1;
            } else {
                self.gc_undos.insert(self.live_undos.remove(0));
            }
            self.undo_group_id += 1;
            undo_group
        }
    }
    /// generates a delta from a plugin's response and applies it to the buffer.
    pub fn apply_plugin_edit(&mut self, edit: PluginEdit) {
        let _t = trace_block("Editor::apply_plugin_edit", &["core"]);
        let PluginEdit { rev, delta, priority, undo_group, .. } = edit;
        let priority = priority as usize;
        let undo_group = undo_group.unwrap_or_else(|| self.calculate_undo_group());
        match self.engine.try_edit_rev(priority, undo_group, rev, delta) {
            Err(e) => error!("Error applying plugin edit: {}", e),
            Ok(_) => self.text = self.engine.get_head().clone(),
        };
    }
    /// Commits the current delta. If the buffer has changed, returns
    /// a 3-tuple containing the delta representing the changes, the previous
    /// buffer, and an `InsertDrift` enum describing the correct selection update
    /// behaviour.
    pub(crate) fn commit_delta(&mut self) -> Option<(RopeDelta, Rope, InsertDrift)> {
        let _t = trace_block("Editor::commit_delta", &["core"]);
        if self.engine.get_head_rev_id() == self.last_rev_id {
            return None;
        }
        let last_token = self.last_rev_id.token();
        let delta = self
            .engine
            .try_delta_rev_head(last_token)
            .expect("last_rev not found");
        let last_text = self.engine.get_rev(last_token).expect("last_rev not found");
        let drift = match self.this_edit_type {
            EditType::Transpose => InsertDrift::Inside,
            EditType::Surround => InsertDrift::Outside,
            _ => InsertDrift::Default,
        };
        self.layers.update_all(&delta);
        self.last_rev_id = self.engine.get_head_rev_id();
        self.sync_state_changed();
        Some((delta, last_text, drift))
    }
    /// Attempts to find the delta from head for the given `RevToken`. Returns
    /// `None` if the revision is not found, so this result should be checked if
    /// the revision is coming from a plugin.
    pub(crate) fn delta_rev_head(&self, target_rev_id: RevToken) -> Option<RopeDelta> {
        self.engine.try_delta_rev_head(target_rev_id).ok()
    }
    #[cfg(not(target_os = "fuchsia"))]
    fn gc_undos(&mut self) {
        if self.revs_in_flight == 0 && !self.gc_undos.is_empty() {
            self.engine.gc(&self.gc_undos);
            self.undos = &self.undos - &self.gc_undos;
            self.gc_undos.clear();
        }
    }
    #[cfg(target_os = "fuchsia")]
    fn gc_undos(&mut self) {}
    pub fn merge_new_state(&mut self, new_engine: Engine) {
        self.engine.merge(&new_engine);
        self.text = self.engine.get_head().clone();
        self.undo_group_id = self.engine.max_undo_group_id() + 1;
        self.last_synced_rev = self.engine.get_head_rev_id();
        self.commit_delta();
    }
    /// See `Engine::set_session_id`. Only useful for Fuchsia sync.
    pub fn set_session_id(&mut self, session: (u64, u32)) {
        self.engine.set_session_id(session);
    }
    #[cfg(feature = "ledger")]
    pub fn set_sync_store(&mut self, sync_store: SyncStore) {
        self.sync_store = Some(sync_store);
    }
    #[cfg(not(feature = "ledger"))]
    pub fn sync_state_changed(&mut self) {}
    #[cfg(feature = "ledger")]
    pub fn sync_state_changed(&mut self) {
        if let Some(sync_store) = self.sync_store.as_mut() {
            if self.last_synced_rev != self.engine.get_head_rev_id() {
                self.last_synced_rev = self.engine.get_head_rev_id();
                sync_store.state_changed();
            }
        }
    }
    #[cfg(feature = "ledger")]
    pub fn transaction_ready(&mut self) {
        if let Some(sync_store) = self.sync_store.as_mut() {
            sync_store.commit_transaction(&self.engine);
        }
    }
    fn do_insert(&mut self, view: &View, config: &BufferItems, chars: &str) {
        let pair_search = config.surrounding_pairs.iter().find(|pair| pair.0 == chars);
        let caret_exists = view.sel_regions().iter().any(|region| region.is_caret());
        if let (Some(pair), false) = (pair_search, caret_exists) {
            self.this_edit_type = EditType::Surround;
            self.add_delta(
                edit_ops::surround(
                    &self.text,
                    view.sel_regions(),
                    pair.0.to_string(),
                    pair.1.to_string(),
                ),
            );
        } else {
            self.this_edit_type = EditType::InsertChars;
            self.add_delta(edit_ops::insert(&self.text, view.sel_regions(), chars));
        }
    }
    fn do_paste(&mut self, view: &View, chars: &str) {
        if view.sel_regions().len() == 1
            || view.sel_regions().len() != count_lines(chars)
        {
            self.add_delta(edit_ops::insert(&self.text, view.sel_regions(), chars));
        } else {
            let mut builder = DeltaBuilder::new(self.text.len());
            for (sel, line) in view.sel_regions().iter().zip(chars.lines()) {
                let iv = Interval::new(sel.min(), sel.max());
                builder.replace(iv, line.into());
            }
            self.add_delta(builder.build());
        }
    }
    pub(crate) fn do_cut(&mut self, view: &mut View) -> Value {
        let result = self.do_copy(view);
        let delta = edit_ops::delete_sel_regions(&self.text, &view.sel_regions());
        if !delta.is_identity() {
            self.this_edit_type = EditType::Delete;
            self.add_delta(delta);
        }
        result
    }
    pub(crate) fn do_copy(&self, view: &View) -> Value {
        if let Some(val)
            = edit_ops::extract_sel_regions(&self.text, view.sel_regions()) {
            Value::String(val.into_owned())
        } else {
            Value::Null
        }
    }
    fn do_undo(&mut self) {
        if self.cur_undo > 1 {
            self.cur_undo -= 1;
            assert!(self.undos.insert(self.live_undos[self.cur_undo]));
            self.this_edit_type = EditType::Undo;
            self.update_undos();
        }
    }
    fn do_redo(&mut self) {
        if self.cur_undo < self.live_undos.len() {
            assert!(self.undos.remove(& self.live_undos[self.cur_undo]));
            self.cur_undo += 1;
            self.this_edit_type = EditType::Redo;
            self.update_undos();
        }
    }
    fn update_undos(&mut self) {
        self.engine.undo(self.undos.clone());
        self.text = self.engine.get_head().clone();
    }
    fn do_replace(&mut self, view: &mut View, replace_all: bool) {
        if let Some(Replace { chars, .. }) = view.get_replace() {
            let mut old_selection = Selection::new();
            for &region in view.sel_regions() {
                old_selection.add_region(region);
            }
            view.collapse_selections(&self.text);
            if replace_all {
                view.do_find_all(&self.text);
            } else {
                view.do_find_next(
                    &self.text,
                    false,
                    true,
                    true,
                    &SelectionModifier::Set,
                );
            }
            match last_selection_region(view.sel_regions()) {
                Some(_) => {
                    self
                        .add_delta(
                            edit_ops::insert(&self.text, view.sel_regions(), chars),
                        )
                }
                None => return,
            };
        }
    }
    fn do_delete_by_movement(
        &mut self,
        view: &View,
        movement: Movement,
        save: bool,
        kill_ring: &mut Rope,
    ) {
        let (delta, rope) = edit_ops::delete_by_movement(
            &self.text,
            view.sel_regions(),
            view.get_lines(),
            movement,
            view.scroll_height(),
            save,
        );
        if let Some(rope) = rope {
            *kill_ring = rope;
        }
        if !delta.is_identity() {
            self.this_edit_type = EditType::Delete;
            self.add_delta(delta);
        }
    }
    fn do_delete_backward(&mut self, view: &View, config: &BufferItems) {
        let delta = edit_ops::delete_backward(&self.text, view.sel_regions(), config);
        if !delta.is_identity() {
            self.this_edit_type = EditType::Delete;
            self.add_delta(delta);
        }
    }
    fn do_transpose(&mut self, view: &View) {
        let delta = edit_ops::transpose(&self.text, view.sel_regions());
        if !delta.is_identity() {
            self.this_edit_type = EditType::Transpose;
            self.add_delta(delta);
        }
    }
    fn do_transform_text<F: Fn(&str) -> String>(
        &mut self,
        view: &View,
        transform_function: F,
    ) {
        let delta = edit_ops::transform_text(
            &self.text,
            view.sel_regions(),
            transform_function,
        );
        if !delta.is_identity() {
            self.this_edit_type = EditType::Other;
            self.add_delta(delta);
        }
    }
    fn do_capitalize_text(&mut self, view: &mut View) {
        let (delta, final_selection) = edit_ops::capitalize_text(
            &self.text,
            view.sel_regions(),
        );
        if !delta.is_identity() {
            self.this_edit_type = EditType::Other;
            self.add_delta(delta);
        }
        view.collapse_selections(&self.text);
        view.set_selection(&self.text, final_selection);
    }
    fn do_modify_indent(
        &mut self,
        view: &View,
        config: &BufferItems,
        direction: IndentDirection,
    ) {
        let delta = edit_ops::modify_indent(
            &self.text,
            view.sel_regions(),
            config,
            direction,
        );
        self.add_delta(delta);
        self
            .this_edit_type = match direction {
            IndentDirection::In => EditType::InsertChars,
            IndentDirection::Out => EditType::Delete,
        };
    }
    fn do_insert_newline(&mut self, view: &View, config: &BufferItems) {
        let delta = edit_ops::insert_newline(&self.text, view.sel_regions(), config);
        self.add_delta(delta);
        self.this_edit_type = EditType::InsertNewline;
    }
    fn do_insert_tab(&mut self, view: &View, config: &BufferItems) {
        let regions = view.sel_regions();
        let delta = edit_ops::insert_tab(&self.text, regions, config);
        let condition = regions
            .first()
            .map(|x| LogicalLines.get_line_range(&self.text, x).len() > 1)
            .unwrap_or(false);
        self.add_delta(delta);
        self
            .this_edit_type = if regions.len() > 1 || condition {
            EditType::Indent
        } else {
            EditType::InsertChars
        };
    }
    fn do_yank(&mut self, view: &View, kill_ring: &Rope) {
        let delta = edit_ops::insert(&self.text, view.sel_regions(), kill_ring.clone());
        self.add_delta(delta);
    }
    fn do_duplicate_line(&mut self, view: &View, config: &BufferItems) {
        let delta = edit_ops::duplicate_line(&self.text, view.sel_regions(), config);
        self.add_delta(delta);
        self.this_edit_type = EditType::Other;
    }
    fn do_change_number<F: Fn(i128) -> Option<i128>>(
        &mut self,
        view: &View,
        transform_function: F,
    ) {
        let delta = edit_ops::change_number(
            &self.text,
            view.sel_regions(),
            transform_function,
        );
        if !delta.is_identity() {
            self.this_edit_type = EditType::Other;
            self.add_delta(delta);
        }
    }
    pub(crate) fn do_edit(
        &mut self,
        view: &mut View,
        kill_ring: &mut Rope,
        config: &BufferItems,
        cmd: BufferEvent,
    ) {
        use self::BufferEvent::*;
        match cmd {
            Delete { movement, kill } => {
                self.do_delete_by_movement(view, movement, kill, kill_ring)
            }
            Backspace => self.do_delete_backward(view, config),
            Transpose => self.do_transpose(view),
            Undo => self.do_undo(),
            Redo => self.do_redo(),
            Uppercase => self.do_transform_text(view, |s| s.to_uppercase()),
            Lowercase => self.do_transform_text(view, |s| s.to_lowercase()),
            Capitalize => self.do_capitalize_text(view),
            Indent => self.do_modify_indent(view, config, IndentDirection::In),
            Outdent => self.do_modify_indent(view, config, IndentDirection::Out),
            InsertNewline => self.do_insert_newline(view, config),
            InsertTab => self.do_insert_tab(view, config),
            Insert(chars) => self.do_insert(view, config, &chars),
            Paste(chars) => self.do_paste(view, &chars),
            Yank => self.do_yank(view, kill_ring),
            ReplaceNext => self.do_replace(view, false),
            ReplaceAll => self.do_replace(view, true),
            DuplicateLine => self.do_duplicate_line(view, config),
            IncreaseNumber => self.do_change_number(view, |s| s.checked_add(1)),
            DecreaseNumber => self.do_change_number(view, |s| s.checked_sub(1)),
        }
    }
    pub fn theme_changed(&mut self, style_map: &ThemeStyleMap) {
        self.layers.theme_changed(style_map);
    }
    pub fn plugin_n_lines(&self) -> usize {
        self.text.measure::<LinesMetric>() + 1
    }
    pub fn update_spans(
        &mut self,
        view: &mut View,
        plugin: PluginId,
        start: usize,
        len: usize,
        spans: Vec<ScopeSpan>,
        rev: RevToken,
    ) {
        let _t = trace_block("Editor::update_spans", &["core"]);
        let mut start = start;
        let mut end_offset = start + len;
        let mut sb = SpansBuilder::new(len);
        for span in spans {
            sb.add_span(Interval::new(span.start, span.end), span.scope_id);
        }
        let mut spans = sb.build();
        if rev != self.engine.get_head_rev_id().token() {
            if let Ok(delta) = self.engine.try_delta_rev_head(rev) {
                let mut transformer = Transformer::new(&delta);
                let new_start = transformer.transform(start, false);
                if !transformer.interval_untouched(Interval::new(start, end_offset)) {
                    spans = spans.transform(start, end_offset, &mut transformer);
                }
                start = new_start;
                end_offset = transformer.transform(end_offset, true);
            } else {
                error!("Revision {} not found", rev);
            }
        }
        let iv = Interval::new(start, end_offset);
        self.layers.update_layer(plugin, iv, spans);
        view.invalidate_styles(&self.text, start, end_offset);
    }
    pub fn update_annotations(
        &mut self,
        view: &mut View,
        plugin: PluginId,
        start: usize,
        len: usize,
        annotation_spans: Vec<DataSpan>,
        annotation_type: AnnotationType,
        rev: RevToken,
    ) {
        let _t = trace_block("Editor::update_annotations", &["core"]);
        let mut start = start;
        let mut end_offset = start + len;
        let mut sb = SpansBuilder::new(len);
        for span in annotation_spans {
            sb.add_span(Interval::new(span.start, span.end), span.data);
        }
        let mut spans = sb.build();
        if rev != self.engine.get_head_rev_id().token() {
            if let Ok(delta) = self.engine.try_delta_rev_head(rev) {
                let mut transformer = Transformer::new(&delta);
                let new_start = transformer.transform(start, false);
                if !transformer.interval_untouched(Interval::new(start, end_offset)) {
                    spans = spans.transform(start, end_offset, &mut transformer);
                }
                start = new_start;
                end_offset = transformer.transform(end_offset, true);
            } else {
                error!("Revision {} not found", rev);
            }
        }
        let iv = Interval::new(start, end_offset);
        view.update_annotations(
            plugin,
            iv,
            Annotations {
                items: spans,
                annotation_type,
            },
        );
    }
    pub(crate) fn get_rev(&self, rev: RevToken) -> Option<Cow<Rope>> {
        let text_cow = if rev == self.engine.get_head_rev_id().token() {
            Cow::Borrowed(&self.text)
        } else {
            match self.engine.get_rev(rev) {
                None => return None,
                Some(text) => Cow::Owned(text),
            }
        };
        Some(text_cow)
    }
    pub fn plugin_get_data(
        &self,
        start: usize,
        unit: TextUnit,
        max_size: usize,
        rev: RevToken,
    ) -> Option<GetDataResponse> {
        let _t = trace_block("Editor::plugin_get_data", &["core"]);
        let text_cow = self.get_rev(rev)?;
        let text = &text_cow;
        let offset = unit.resolve_offset(text.borrow(), start)?;
        let max_size = min(max_size, MAX_SIZE_LIMIT);
        let mut end_off = offset.saturating_add(max_size);
        if end_off >= text.len() {
            end_off = text.len();
        } else {
            end_off = text.prev_codepoint_offset(end_off + 1).unwrap();
        }
        let chunk = text.slice_to_cow(offset..end_off).into_owned();
        let first_line = text.line_of_offset(offset);
        let first_line_offset = offset - text.offset_of_line(first_line);
        Some(GetDataResponse {
            chunk,
            offset,
            first_line,
            first_line_offset,
        })
    }
}
#[derive(PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EditType {
    /// A catchall for edits that don't fit elsewhere, and which should
    /// always have their own undo groups; used for things like cut/copy/paste.
    Other,
    /// An insert from the keyboard/IME (not a paste or a yank).
    #[serde(rename = "insert")]
    InsertChars,
    #[serde(rename = "newline")]
    InsertNewline,
    /// An indentation adjustment.
    Indent,
    Delete,
    Undo,
    Redo,
    Transpose,
    Surround,
}
impl EditType {
    /// Checks whether a new undo group should be created between two edits.
    fn breaks_undo_group(self, previous: EditType) -> bool {
        self == EditType::Other || self == EditType::Transpose || self != previous
    }
}
fn last_selection_region(regions: &[SelRegion]) -> Option<&SelRegion> {
    for region in regions.iter().rev() {
        if !region.is_caret() {
            return Some(region);
        }
    }
    None
}
/// Counts the number of lines in the string, not including any trailing newline.
fn count_lines(s: &str) -> usize {
    let mut newlines = count_newlines(s);
    if s.as_bytes().last() == Some(&0xa) {
        newlines -= 1;
    }
    1 + newlines
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plugin_edit() {
        let base_text = "hello";
        let mut editor = Editor::with_text(base_text);
        let mut builder = DeltaBuilder::new(base_text.len());
        builder.replace(0..0, "s".into());
        let delta = builder.build();
        let rev = editor.get_head_rev_token();
        let edit_one = PluginEdit {
            rev,
            delta,
            priority: 55,
            after_cursor: false,
            undo_group: None,
            author: "plugin_one".into(),
        };
        editor.apply_plugin_edit(edit_one.clone());
        editor.apply_plugin_edit(edit_one);
        assert_eq!(editor.get_buffer().to_string(), "sshello");
    }
}
#[cfg(test)]
mod tests_llm_16_288 {
    use crate::editor::EditType;
    #[test]
    fn test_breaks_undo_group() {
        let _rug_st_tests_llm_16_288_rrrruuuugggg_test_breaks_undo_group = 0;
        let previous = EditType::InsertChars;
        debug_assert_eq!(EditType::Other.breaks_undo_group(previous), true);
        debug_assert_eq!(EditType::Other.breaks_undo_group(EditType::Other), true);
        debug_assert_eq!(EditType::InsertNewline.breaks_undo_group(previous), true);
        debug_assert_eq!(EditType::Indent.breaks_undo_group(previous), true);
        debug_assert_eq!(EditType::Delete.breaks_undo_group(previous), true);
        debug_assert_eq!(EditType::Undo.breaks_undo_group(previous), true);
        debug_assert_eq!(EditType::Redo.breaks_undo_group(previous), true);
        debug_assert_eq!(EditType::Transpose.breaks_undo_group(previous), true);
        debug_assert_eq!(EditType::Surround.breaks_undo_group(previous), true);
        debug_assert_eq!(EditType::InsertChars.breaks_undo_group(previous), false);
        let _rug_ed_tests_llm_16_288_rrrruuuugggg_test_breaks_undo_group = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_293 {
    use super::*;
    use crate::*;
    #[test]
    fn test_calculate_undo_group() {
        let _rug_st_tests_llm_16_293_rrrruuuugggg_test_calculate_undo_group = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = true;
        let rug_fuzz_3 = false;
        let rug_fuzz_4 = false;
        let mut editor = Editor::new();
        let result = editor.calculate_undo_group();
        debug_assert_eq!(result, 0);
        editor.live_undos.push(rug_fuzz_0);
        editor.cur_undo = rug_fuzz_1;
        let result = editor.calculate_undo_group();
        debug_assert_eq!(result, 1);
        editor.force_undo_group = rug_fuzz_2;
        let result = editor.calculate_undo_group();
        debug_assert_eq!(result, 1);
        editor.force_undo_group = rug_fuzz_3;
        editor.this_edit_type = EditType::Indent;
        editor.last_edit_type = EditType::InsertChars;
        let result = editor.calculate_undo_group();
        debug_assert_eq!(result, 1);
        editor.force_undo_group = rug_fuzz_4;
        editor.this_edit_type = EditType::Indent;
        editor.last_edit_type = EditType::Indent;
        let result = editor.calculate_undo_group();
        debug_assert_eq!(result, 0);
        let new_undo_group = editor.undo_group_id;
        let result = editor.calculate_undo_group();
        debug_assert_eq!(result, new_undo_group);
        let _rug_ed_tests_llm_16_293_rrrruuuugggg_test_calculate_undo_group = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_302 {
    use super::*;
    use crate::*;
    #[derive(Debug)]
    struct AnnotationRange {
        start_line: usize,
        start_col: usize,
        end_line: usize,
        end_col: usize,
    }
    #[derive(Debug, PartialEq)]
    enum EditType {
        Other,
        InsertChars,
        InsertNewline,
        Indent,
        Delete,
        Undo,
        Redo,
        Transpose,
        Surround,
    }
    #[derive(Debug)]
    struct Layers {
        themes: Vec<String>,
    }
    #[derive(Debug)]
    struct Editor {
        cur_undo: usize,
        undos: Vec<usize>,
        live_undos: Vec<usize>,
        this_edit_type: EditType,
        last_edit_type: EditType,
        layers: Layers,
    }
    impl AnnotationRange {
        fn new(
            start_line: usize,
            start_col: usize,
            end_line: usize,
            end_col: usize,
        ) -> Self {
            Self {
                start_line,
                start_col,
                end_line,
                end_col,
            }
        }
    }
    impl EditType {
        fn breaks_undo_group(self, previous: Self) -> bool {
            self == EditType::Other || self == EditType::Transpose || self != previous
        }
    }
    impl Layers {
        fn new(themes: Vec<String>) -> Self {
            Self { themes }
        }
    }
    impl Editor {
        fn new() -> Self {
            Self {
                cur_undo: 0,
                undos: Vec::new(),
                live_undos: Vec::new(),
                this_edit_type: EditType::Other,
                last_edit_type: EditType::Other,
                layers: Layers::new(vec!["theme1".to_string(), "theme2".to_string()]),
            }
        }
        fn do_undo(&mut self) {
            if self.cur_undo > 1 {
                self.cur_undo -= 1;
                self.undos.remove(self.cur_undo);
                self.this_edit_type = EditType::Undo;
                self.update_undos();
            }
        }
        fn update_undos(&mut self) {}
    }
    #[cfg(test)]
    #[test]
    fn test_do_undo() {
        let _rug_st_tests_llm_16_302_rrrruuuugggg_test_do_undo = 0;
        let rug_fuzz_0 = 2;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 3;
        let mut editor = Editor::new();
        editor.cur_undo = rug_fuzz_0;
        editor.live_undos = vec![rug_fuzz_1, 2, 3];
        editor.undos = vec![rug_fuzz_2, 4, 5];
        editor.this_edit_type = EditType::Other;
        editor.last_edit_type = EditType::Surround;
        editor.do_undo();
        debug_assert_eq!(editor.cur_undo, 1);
        debug_assert_eq!(editor.undos, vec![3, 1, 4, 5]);
        debug_assert_eq!(editor.this_edit_type, EditType::Undo);
        let _rug_ed_tests_llm_16_302_rrrruuuugggg_test_do_undo = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_311 {
    use super::*;
    use crate::*;
    use serde_json;
    #[test]
    fn test_get_layers() {
        let _rug_st_tests_llm_16_311_rrrruuuugggg_test_get_layers = 0;
        let mut editor = Editor::new();
        let result = editor.get_layers();
        let expected_result = &editor.layers;
        debug_assert_eq!(result as * const Layers, expected_result as * const Layers);
        let _rug_ed_tests_llm_16_311_rrrruuuugggg_test_get_layers = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_322 {
    use super::*;
    use crate::*;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_322_rrrruuuugggg_test_new = 0;
        let editor = Editor::new();
        debug_assert_eq!(editor.get_buffer().len(), 0);
        debug_assert_eq!(editor.get_layers().get_merged().len(), 0);
        let _rug_ed_tests_llm_16_322_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_323 {
    use super::*;
    use crate::*;
    #[test]
    fn test_plugin_get_data() {
        let _rug_st_tests_llm_16_323_rrrruuuugggg_test_plugin_get_data = 0;
        let _rug_ed_tests_llm_16_323_rrrruuuugggg_test_plugin_get_data = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_324 {
    use super::*;
    use crate::*;
    #[test]
    fn test_plugin_n_lines() {
        let _rug_st_tests_llm_16_324_rrrruuuugggg_test_plugin_n_lines = 0;
        let editor = Editor::new();
        debug_assert_eq!(editor.plugin_n_lines(), 1);
        let _rug_ed_tests_llm_16_324_rrrruuuugggg_test_plugin_n_lines = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_327 {
    use super::*;
    use crate::*;
    #[test]
    fn test_set_force_undo_group() {
        let _rug_st_tests_llm_16_327_rrrruuuugggg_test_set_force_undo_group = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = false;
        let mut editor = Editor::new();
        editor.set_force_undo_group(rug_fuzz_0);
        debug_assert_eq!(editor.force_undo_group, true);
        editor.set_force_undo_group(rug_fuzz_1);
        debug_assert_eq!(editor.force_undo_group, false);
        let _rug_ed_tests_llm_16_327_rrrruuuugggg_test_set_force_undo_group = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_328 {
    use super::*;
    use crate::*;
    #[test]
    fn test_set_pristine() {
        let _rug_st_tests_llm_16_328_rrrruuuugggg_test_set_pristine = 0;
        let rug_fuzz_0 = true;
        let mut editor = Editor::new();
        editor.set_force_undo_group(rug_fuzz_0);
        editor.set_pristine();
        debug_assert_eq!(editor.pristine_rev_id, editor.engine.get_head_rev_id());
        let _rug_ed_tests_llm_16_328_rrrruuuugggg_test_set_pristine = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_331 {
    use super::*;
    use crate::*;
    #[test]
    fn test_sync_state_changed() {
        let _rug_st_tests_llm_16_331_rrrruuuugggg_test_sync_state_changed = 0;
        let mut editor = Editor::new();
        editor.sync_state_changed();
        let _rug_ed_tests_llm_16_331_rrrruuuugggg_test_sync_state_changed = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_333 {
    use super::*;
    use crate::*;
    use serde_json;
    #[test]
    fn test_update_edit_type() {
        let _rug_st_tests_llm_16_333_rrrruuuugggg_test_update_edit_type = 0;
        let mut editor = Editor::new();
        editor.this_edit_type = EditType::InsertChars;
        editor.update_edit_type();
        debug_assert_eq!(editor.last_edit_type, EditType::InsertChars);
        debug_assert_eq!(editor.this_edit_type, EditType::Other);
        let _rug_ed_tests_llm_16_333_rrrruuuugggg_test_update_edit_type = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_338 {
    use super::*;
    use crate::*;
    #[test]
    fn test_count_lines_empty_string() {
        let _rug_st_tests_llm_16_338_rrrruuuugggg_test_count_lines_empty_string = 0;
        let rug_fuzz_0 = "";
        let s = rug_fuzz_0;
        let result = count_lines(s);
        debug_assert_eq!(result, 0);
        let _rug_ed_tests_llm_16_338_rrrruuuugggg_test_count_lines_empty_string = 0;
    }
    #[test]
    fn test_count_lines_single_line() {
        let _rug_st_tests_llm_16_338_rrrruuuugggg_test_count_lines_single_line = 0;
        let rug_fuzz_0 = "This is a single line";
        let s = rug_fuzz_0;
        let result = count_lines(s);
        debug_assert_eq!(result, 1);
        let _rug_ed_tests_llm_16_338_rrrruuuugggg_test_count_lines_single_line = 0;
    }
    #[test]
    fn test_count_lines_multiple_lines() {
        let _rug_st_tests_llm_16_338_rrrruuuugggg_test_count_lines_multiple_lines = 0;
        let rug_fuzz_0 = "This is\na\nmulti-line\nstring";
        let s = rug_fuzz_0;
        let result = count_lines(s);
        debug_assert_eq!(result, 4);
        let _rug_ed_tests_llm_16_338_rrrruuuugggg_test_count_lines_multiple_lines = 0;
    }
    #[test]
    fn test_count_lines_with_trailing_newline() {
        let _rug_st_tests_llm_16_338_rrrruuuugggg_test_count_lines_with_trailing_newline = 0;
        let rug_fuzz_0 = "This is\na\nmulti-line\nstring\n";
        let s = rug_fuzz_0;
        let result = count_lines(s);
        debug_assert_eq!(result, 4);
        let _rug_ed_tests_llm_16_338_rrrruuuugggg_test_count_lines_with_trailing_newline = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_339 {
    use super::*;
    use crate::*;
    use crate::selection::{SelRegion, Affinity};
    #[test]
    fn test_last_selection_region() {
        let _rug_st_tests_llm_16_339_rrrruuuugggg_test_last_selection_region = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 1;
        let regions = vec![
            SelRegion::new(rug_fuzz_0, rug_fuzz_1).with_affinity(Affinity::Upstream),
            SelRegion::new(2, 3).with_affinity(Affinity::Downstream), SelRegion::new(4,
            5).with_affinity(Affinity::Upstream)
        ];
        debug_assert_eq!(last_selection_region(& regions).unwrap().start, 2);
        debug_assert_eq!(last_selection_region(& regions).unwrap().end, 3);
        let _rug_ed_tests_llm_16_339_rrrruuuugggg_test_last_selection_region = 0;
    }
}
