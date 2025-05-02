//! Data structures representing (multiple) selections and cursors.
use std::cmp::{max, min};
use std::fmt;
use std::ops::Deref;
use crate::annotations::{AnnotationRange, AnnotationSlice, AnnotationType, ToAnnotation};
use crate::index_set::remove_n_at;
use crate::line_offset::LineOffset;
use crate::view::View;
use xi_rope::{Interval, Rope, RopeDelta, Transformer};
/// A type representing horizontal measurements. This is currently in units
/// that are not very well defined except that ASCII characters count as
/// 1 each. It will change.
pub type HorizPos = usize;
/// Indicates if an edit should try to drift inside or outside nearby selections. If the selection
/// is zero width, that is, it is a caret, this value will be ignored, the equivalent of the
/// `Default` value.
#[derive(Copy, Clone)]
pub enum InsertDrift {
    /// Indicates this edit should happen within any (non-caret) selections if possible.
    Inside,
    /// Indicates this edit should happen outside any selections if possible.
    Outside,
    /// Indicates to do whatever the `after` bool says to do
    Default,
}
/// A set of zero or more selection regions, representing a selection state.
#[derive(Default, Debug, Clone)]
pub struct Selection {
    regions: Vec<SelRegion>,
}
impl Selection {
    /// Creates a new empty selection.
    pub fn new() -> Selection {
        Selection::default()
    }
    /// Creates a selection with a single region.
    pub fn new_simple(region: SelRegion) -> Selection {
        Selection { regions: vec![region] }
    }
    /// Clear the selection.
    pub fn clear(&mut self) {
        self.regions.clear();
    }
    /// Collapse all selections into a single caret.
    pub fn collapse(&mut self) {
        self.regions.truncate(1);
        self.regions[0].start = self.regions[0].end;
    }
    pub fn search(&self, offset: usize) -> usize {
        if self.regions.is_empty() || offset > self.regions.last().unwrap().max() {
            return self.regions.len();
        }
        match self.regions.binary_search_by(|r| r.max().cmp(&offset)) {
            Ok(ix) => ix,
            Err(ix) => ix,
        }
    }
    /// Add a region to the selection. This method implements merging logic.
    ///
    /// Two non-caret regions merge if their interiors intersect; merely
    /// touching at the edges does not cause a merge. A caret merges with
    /// a non-caret if it is in the interior or on either edge. Two carets
    /// merge if they are the same offset.
    ///
    /// Performance note: should be O(1) if the new region strictly comes
    /// after all the others in the selection, otherwise O(n).
    pub fn add_region(&mut self, region: SelRegion) {
        let mut ix = self.search(region.min());
        if ix == self.regions.len() {
            self.regions.push(region);
            return;
        }
        let mut region = region;
        let mut end_ix = ix;
        if self.regions[ix].min() <= region.min() {
            if self.regions[ix].should_merge(region) {
                region = region.merge_with(self.regions[ix]);
            } else {
                ix += 1;
            }
            end_ix += 1;
        }
        while end_ix < self.regions.len() && region.should_merge(self.regions[end_ix]) {
            region = region.merge_with(self.regions[end_ix]);
            end_ix += 1;
        }
        if ix == end_ix {
            self.regions.insert(ix, region);
        } else {
            self.regions[ix] = region;
            remove_n_at(&mut self.regions, ix + 1, end_ix - ix - 1);
        }
    }
    /// Gets a slice of regions that intersect the given range. Regions that
    /// merely touch the range at the edges are also included, so it is the
    /// caller's responsibility to further trim them, in particular to only
    /// display one caret in the upstream/downstream cases.
    ///
    /// Performance note: O(log n).
    pub fn regions_in_range(&self, start: usize, end: usize) -> &[SelRegion] {
        let first = self.search(start);
        let mut last = self.search(end);
        if last < self.regions.len() && self.regions[last].min() <= end {
            last += 1;
        }
        &self.regions[first..last]
    }
    /// Deletes all the regions that intersect or (if delete_adjacent = true) touch the given range.
    pub fn delete_range(&mut self, start: usize, end: usize, delete_adjacent: bool) {
        let mut first = self.search(start);
        let mut last = self.search(end);
        if first >= self.regions.len() {
            return;
        }
        if !delete_adjacent && self.regions[first].max() == start {
            first += 1;
        }
        if last < self.regions.len()
            && ((delete_adjacent && self.regions[last].min() <= end)
                || (!delete_adjacent && self.regions[last].min() < end))
        {
            last += 1;
        }
        remove_n_at(&mut self.regions, first, last - first);
    }
    /// Add a region to the selection. This method does not merge regions and does not allow
    /// ambiguous regions (regions that overlap).
    ///
    /// On ambiguous regions, the region with the lower start position wins. That is, in such a
    /// case, the new region is either not added at all, because there is an ambiguous region with
    /// a lower start position, or existing regions that intersect with the new region but do
    /// not start before the new region, are deleted.
    pub fn add_range_distinct(&mut self, region: SelRegion) -> (usize, usize) {
        let mut ix = self.search(region.min());
        if ix < self.regions.len() && self.regions[ix].max() == region.min() {
            ix += 1;
        }
        if ix < self.regions.len() {
            let occ = &self.regions[ix];
            let is_eq = occ.min() == region.min() && occ.max() == region.max();
            let is_intersect_before = region.min() >= occ.min()
                && occ.max() > region.min();
            if is_eq || is_intersect_before {
                return (occ.min(), occ.max());
            }
        }
        let mut last = self.search(region.max());
        if last < self.regions.len() && self.regions[last].min() < region.max() {
            last += 1;
        }
        remove_n_at(&mut self.regions, ix, last - ix);
        if ix == self.regions.len() {
            self.regions.push(region);
        } else {
            self.regions.insert(ix, region);
        }
        (self.regions[ix].min(), self.regions[ix].max())
    }
    /// Computes a new selection based on applying a delta to the old selection.
    ///
    /// When new text is inserted at a caret, the new caret can be either before
    /// or after the inserted text, depending on the `after` parameter.
    ///
    /// Whether or not the preceding selections are restored depends on the keep_selections
    /// value (only set to true on transpose).
    pub fn apply_delta(
        &self,
        delta: &RopeDelta,
        after: bool,
        drift: InsertDrift,
    ) -> Selection {
        let mut result = Selection::new();
        let mut transformer = Transformer::new(delta);
        for region in self.iter() {
            let is_caret = region.start == region.end;
            let is_region_forward = region.start < region.end;
            let (start_after, end_after) = match (drift, is_caret) {
                (InsertDrift::Inside, false) => (!is_region_forward, is_region_forward),
                (InsertDrift::Outside, false) => (is_region_forward, !is_region_forward),
                _ => (after, after),
            };
            let new_region = SelRegion::new(
                    transformer.transform(region.start, start_after),
                    transformer.transform(region.end, end_after),
                )
                .with_affinity(region.affinity);
            result.add_region(new_region);
        }
        result
    }
}
/// Implementing the `ToAnnotation` trait allows to convert selections to annotations.
impl ToAnnotation for Selection {
    fn get_annotations(
        &self,
        interval: Interval,
        view: &View,
        text: &Rope,
    ) -> AnnotationSlice {
        let regions = self.regions_in_range(interval.start(), interval.end());
        let ranges = regions
            .iter()
            .map(|region| {
                let (start_line, start_col) = view
                    .offset_to_line_col(text, region.min());
                let (end_line, end_col) = view.offset_to_line_col(text, region.max());
                AnnotationRange {
                    start_line,
                    start_col,
                    end_line,
                    end_col,
                }
            })
            .collect::<Vec<AnnotationRange>>();
        AnnotationSlice::new(AnnotationType::Selection, ranges, None)
    }
}
/// Implementing the Deref trait allows callers to easily test `is_empty`, iterate
/// through all ranges, etc.
impl Deref for Selection {
    type Target = [SelRegion];
    fn deref(&self) -> &[SelRegion] {
        &self.regions
    }
}
/// The "affinity" of a cursor which is sitting exactly on a line break.
///
/// We say "cursor" here rather than "caret" because (depending on presentation)
/// the front-end may draw a cursor even when the region is not a caret.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Affinity {
    /// The cursor should be displayed downstream of the line break. For
    /// example, if the buffer is "abcd", and the cursor is on a line break
    /// after "ab", it should be displayed on the second line before "cd".
    Downstream,
    /// The cursor should be displayed upstream of the line break. For
    /// example, if the buffer is "abcd", and the cursor is on a line break
    /// after "ab", it should be displayed on the previous line after "ab".
    Upstream,
}
impl Default for Affinity {
    fn default() -> Affinity {
        Affinity::Downstream
    }
}
/// A type representing a single contiguous region of a selection. We use the
/// term "caret" (sometimes also "cursor", more loosely) to refer to a selection
/// region with an empty interior. A "non-caret region" is one with a non-empty
/// interior (i.e. `start != end`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SelRegion {
    /// The inactive edge of a selection, as a byte offset. When
    /// equal to end, the selection range acts as a caret.
    pub start: usize,
    /// The active edge of a selection, as a byte offset.
    pub end: usize,
    /// A saved horizontal position (used primarily for line up/down movement).
    pub horiz: Option<HorizPos>,
    /// The affinity of the cursor.
    pub affinity: Affinity,
}
impl SelRegion {
    /// Returns a new region.
    pub fn new(start: usize, end: usize) -> Self {
        Self {
            start,
            end,
            horiz: None,
            affinity: Affinity::default(),
        }
    }
    /// Returns a new caret region (`start == end`).
    pub fn caret(pos: usize) -> Self {
        Self {
            start: pos,
            end: pos,
            horiz: None,
            affinity: Affinity::default(),
        }
    }
    /// Returns a region with the given horizontal position.
    pub fn with_horiz(self, horiz: Option<HorizPos>) -> Self {
        Self { horiz, ..self }
    }
    /// Returns a region with the given affinity.
    pub fn with_affinity(self, affinity: Affinity) -> Self {
        Self { affinity, ..self }
    }
    /// Gets the earliest offset within the region, ie the minimum of both edges.
    pub fn min(self) -> usize {
        min(self.start, self.end)
    }
    /// Gets the latest offset within the region, ie the maximum of both edges.
    pub fn max(self) -> usize {
        max(self.start, self.end)
    }
    /// Determines whether the region is a caret (ie has an empty interior).
    pub fn is_caret(self) -> bool {
        self.start == self.end
    }
    /// Determines whether the region's affinity is upstream.
    pub fn is_upstream(self) -> bool {
        self.affinity == Affinity::Upstream
    }
    fn should_merge(self, other: SelRegion) -> bool {
        other.min() < self.max()
            || ((self.is_caret() || other.is_caret()) && other.min() == self.max())
    }
    fn merge_with(self, other: SelRegion) -> SelRegion {
        let is_forward = self.end >= self.start;
        let new_min = min(self.min(), other.min());
        let new_max = max(self.max(), other.max());
        let (start, end) = if is_forward {
            (new_min, new_max)
        } else {
            (new_max, new_min)
        };
        SelRegion::new(start, end)
    }
}
impl<'a> From<&'a SelRegion> for Interval {
    fn from(src: &'a SelRegion) -> Interval {
        Interval::new(src.min(), src.max())
    }
}
impl From<Interval> for SelRegion {
    fn from(src: Interval) -> SelRegion {
        SelRegion::new(src.start, src.end)
    }
}
impl From<SelRegion> for Selection {
    fn from(region: SelRegion) -> Self {
        Self::new_simple(region)
    }
}
impl fmt::Display for Selection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.regions.len() == 1 {
            self.regions[0].fmt(f)?;
        } else {
            write!(f, "[ {}", & self.regions[0])?;
            for region in &self.regions[1..] {
                write!(f, ", {}", region)?;
            }
            write!(f, " ]")?;
        }
        Ok(())
    }
}
impl fmt::Display for SelRegion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_caret() {
            write!(f, "{}|", self.start)?;
        } else if self.start < self.end {
            write!(f, "{}..{}|", self.start, self.end)?;
        } else {
            write!(f, "|{}..{}", self.end, self.start)?;
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::{InsertDrift, SelRegion, Selection};
    use std::ops::Deref;
    use xi_rope::{DeltaBuilder, Interval};
    fn r(start: usize, end: usize) -> SelRegion {
        SelRegion::new(start, end)
    }
    #[test]
    fn empty() {
        let s = Selection::new();
        assert!(s.is_empty());
        assert_eq!(s.deref(), & []);
    }
    #[test]
    fn simple_region() {
        let s = Selection::new_simple(r(3, 5));
        assert!(! s.is_empty());
        assert_eq!(s.deref(), & [r(3, 5)]);
    }
    #[test]
    fn from_selregion() {
        let s: Selection = r(3, 5).into();
        assert!(! s.is_empty());
        assert_eq!(s.deref(), & [r(3, 5)]);
    }
    #[test]
    fn delete_range() {
        let mut s = Selection::new_simple(r(3, 5));
        s.delete_range(1, 2, true);
        assert_eq!(s.deref(), & [r(3, 5)]);
        s.delete_range(1, 3, false);
        assert_eq!(s.deref(), & [r(3, 5)]);
        s.delete_range(1, 3, true);
        assert_eq!(s.deref(), & []);
        let mut s = Selection::new_simple(r(3, 5));
        s.delete_range(5, 6, false);
        assert_eq!(s.deref(), & [r(3, 5)]);
        s.delete_range(5, 6, true);
        assert_eq!(s.deref(), & []);
        let mut s = Selection::new_simple(r(3, 5));
        s.delete_range(2, 4, false);
        assert_eq!(s.deref(), & []);
        assert_eq!(s.deref(), & []);
        let mut s = Selection::new();
        s.add_region(r(3, 5));
        s.add_region(r(7, 8));
        s.delete_range(2, 10, false);
        assert_eq!(s.deref(), & []);
    }
    #[test]
    fn simple_regions_in_range() {
        let s = Selection::new_simple(r(3, 5));
        assert_eq!(s.regions_in_range(0, 1), & []);
        assert_eq!(s.regions_in_range(0, 2), & []);
        assert_eq!(s.regions_in_range(0, 3), & [r(3, 5)]);
        assert_eq!(s.regions_in_range(0, 4), & [r(3, 5)]);
        assert_eq!(s.regions_in_range(5, 6), & [r(3, 5)]);
        assert_eq!(s.regions_in_range(6, 7), & []);
    }
    #[test]
    fn caret_regions_in_range() {
        let s = Selection::new_simple(r(4, 4));
        assert_eq!(s.regions_in_range(0, 1), & []);
        assert_eq!(s.regions_in_range(0, 2), & []);
        assert_eq!(s.regions_in_range(0, 3), & []);
        assert_eq!(s.regions_in_range(0, 4), & [r(4, 4)]);
        assert_eq!(s.regions_in_range(4, 4), & [r(4, 4)]);
        assert_eq!(s.regions_in_range(4, 5), & [r(4, 4)]);
        assert_eq!(s.regions_in_range(5, 6), & []);
    }
    #[test]
    fn merge_regions() {
        let mut s = Selection::new();
        s.add_region(r(3, 5));
        assert_eq!(s.deref(), & [r(3, 5)]);
        s.add_region(r(7, 9));
        assert_eq!(s.deref(), & [r(3, 5), r(7, 9)]);
        s.add_region(r(1, 3));
        assert_eq!(s.deref(), & [r(1, 3), r(3, 5), r(7, 9)]);
        s.add_region(r(4, 6));
        assert_eq!(s.deref(), & [r(1, 3), r(3, 6), r(7, 9)]);
        s.add_region(r(2, 8));
        assert_eq!(s.deref(), & [r(1, 9)]);
        s.add_region(r(10, 8));
        assert_eq!(s.deref(), & [r(10, 1)]);
        s.clear();
        assert_eq!(s.deref(), & []);
        s.add_region(r(1, 4));
        s.add_region(r(4, 5));
        s.add_region(r(5, 6));
        s.add_region(r(6, 9));
        assert_eq!(s.deref(), & [r(1, 4), r(4, 5), r(5, 6), r(6, 9)]);
        s.add_region(r(2, 8));
        assert_eq!(s.deref(), & [r(1, 9)]);
    }
    #[test]
    fn merge_carets() {
        let mut s = Selection::new();
        s.add_region(r(1, 1));
        assert_eq!(s.deref(), & [r(1, 1)]);
        s.add_region(r(3, 3));
        assert_eq!(s.deref(), & [r(1, 1), r(3, 3)]);
        s.add_region(r(2, 2));
        assert_eq!(s.deref(), & [r(1, 1), r(2, 2), r(3, 3)]);
        s.add_region(r(1, 1));
        assert_eq!(s.deref(), & [r(1, 1), r(2, 2), r(3, 3)]);
    }
    #[test]
    fn merge_region_caret() {
        let mut s = Selection::new();
        s.add_region(r(3, 5));
        assert_eq!(s.deref(), & [r(3, 5)]);
        s.add_region(r(3, 3));
        assert_eq!(s.deref(), & [r(3, 5)]);
        s.add_region(r(4, 4));
        assert_eq!(s.deref(), & [r(3, 5)]);
        s.add_region(r(5, 5));
        assert_eq!(s.deref(), & [r(3, 5)]);
        s.add_region(r(6, 6));
        assert_eq!(s.deref(), & [r(3, 5), r(6, 6)]);
    }
    #[test]
    fn merge_reverse() {
        let mut s = Selection::new();
        s.add_region(r(5, 3));
        assert_eq!(s.deref(), & [r(5, 3)]);
        s.add_region(r(9, 7));
        assert_eq!(s.deref(), & [r(5, 3), r(9, 7)]);
        s.add_region(r(3, 1));
        assert_eq!(s.deref(), & [r(3, 1), r(5, 3), r(9, 7)]);
        s.add_region(r(6, 4));
        assert_eq!(s.deref(), & [r(3, 1), r(6, 3), r(9, 7)]);
        s.add_region(r(8, 2));
        assert_eq!(s.deref(), & [r(9, 1)]);
    }
    #[test]
    fn apply_delta_outside_drift() {
        let mut s = Selection::new();
        s.add_region(r(0, 4));
        s.add_region(r(4, 8));
        assert_eq!(s.deref(), & [r(0, 4), r(4, 8)]);
        let mut builder = DeltaBuilder::new("texthere!".len());
        builder.replace(Interval::new(4, 4), " ".into());
        let s2 = s.apply_delta(&builder.build(), true, InsertDrift::Outside);
        assert_eq!(s2.deref(), & [r(0, 4), r(5, 9)]);
    }
    #[test]
    fn apply_delta_inside_drift() {
        let mut s = Selection::new();
        s.add_region(r(1, 2));
        assert_eq!(s.deref(), & [r(1, 2)]);
        let mut builder = DeltaBuilder::new("abc".len());
        builder.replace(Interval::new(1, 1), "b".into());
        builder.replace(Interval::new(2, 2), "b".into());
        let s2 = s.apply_delta(&builder.build(), true, InsertDrift::Inside);
        assert_eq!(s2.deref(), & [r(1, 4)]);
    }
    #[test]
    fn apply_delta_drift_ignored_for_carets() {
        let mut s = Selection::new();
        s.add_region(r(1, 1));
        assert_eq!(s.deref(), & [r(1, 1)]);
        let mut builder = DeltaBuilder::new("ab".len());
        builder.replace(Interval::new(1, 1), "b".into());
        let s2 = s.apply_delta(&builder.build(), true, InsertDrift::Inside);
        assert_eq!(s2.deref(), & [r(2, 2)]);
        let mut builder = DeltaBuilder::new("ab".len());
        builder.replace(Interval::new(1, 1), "b".into());
        let s3 = s.apply_delta(&builder.build(), false, InsertDrift::Inside);
        assert_eq!(s3.deref(), & [r(1, 1)]);
    }
    #[test]
    fn display() {
        let mut s = Selection::new();
        s.add_region(r(1, 1));
        assert_eq!(s.to_string(), "1|");
        s.add_region(r(3, 5));
        s.add_region(r(8, 6));
        assert_eq!(s.to_string(), "[ 1|, 3..5|, |6..8 ]");
    }
}
#[cfg(test)]
mod tests_llm_16_69_llm_16_68 {
    use crate::selection::Affinity;
    #[test]
    fn test_default() {
        let _rug_st_tests_llm_16_69_llm_16_68_rrrruuuugggg_test_default = 0;
        let default_affinity: Affinity = Affinity::Downstream;
        debug_assert_eq!(default_affinity, Affinity::default());
        let _rug_ed_tests_llm_16_69_llm_16_68_rrrruuuugggg_test_default = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_70 {
    use super::*;
    use crate::*;
    use selection::{Affinity, SelRegion};
    use xi_rope::Interval;
    #[test]
    fn test_from() {
        let _rug_st_tests_llm_16_70_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 5;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 5;
        let src = Interval::new(rug_fuzz_0, rug_fuzz_1);
        let actual = SelRegion::from(src);
        let expected = SelRegion::new(rug_fuzz_2, rug_fuzz_3);
        debug_assert_eq!(actual, expected);
        let _rug_ed_tests_llm_16_70_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_71 {
    use crate::selection::{Selection, SelRegion};
    use crate::selection::Affinity;
    #[test]
    fn test_from() {
        let _rug_st_tests_llm_16_71_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 5;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let region = SelRegion::new(rug_fuzz_0, rug_fuzz_1);
        let selection = Selection::from(region);
        debug_assert_eq!(selection.regions.len(), 1);
        debug_assert_eq!(selection.regions[rug_fuzz_2].start, 0);
        debug_assert_eq!(selection.regions[rug_fuzz_3].end, 5);
        debug_assert_eq!(selection.regions[rug_fuzz_4].horiz, None);
        debug_assert_eq!(selection.regions[rug_fuzz_5].affinity, Affinity::Downstream);
        let _rug_ed_tests_llm_16_71_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_72 {
    use super::*;
    use crate::*;
    #[test]
    fn test_deref() {
        let _rug_st_tests_llm_16_72_rrrruuuugggg_test_deref = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 1;
        let regions = vec![
            SelRegion::new(rug_fuzz_0, rug_fuzz_1), SelRegion::new(2, 4),
            SelRegion::new(6, 8)
        ];
        let selection = Selection { regions };
        let result = selection.deref();
        debug_assert_eq!(
            result, & [SelRegion::new(0, 1), SelRegion::new(2, 4), SelRegion::new(6, 8)]
        );
        let _rug_ed_tests_llm_16_72_rrrruuuugggg_test_deref = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_668 {
    use super::*;
    use crate::*;
    #[test]
    fn test_caret() {
        let _rug_st_tests_llm_16_668_rrrruuuugggg_test_caret = 0;
        let rug_fuzz_0 = 10;
        let pos = rug_fuzz_0;
        let region = SelRegion::caret(pos);
        debug_assert_eq!(region.start, pos);
        debug_assert_eq!(region.end, pos);
        debug_assert_eq!(region.horiz, None);
        debug_assert_eq!(region.affinity, Affinity::default());
        let _rug_ed_tests_llm_16_668_rrrruuuugggg_test_caret = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_669 {
    use crate::selection::{SelRegion, Affinity};
    #[test]
    fn test_is_caret() {
        let _rug_st_tests_llm_16_669_rrrruuuugggg_test_is_caret = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 5;
        let rug_fuzz_4 = 5;
        let rug_fuzz_5 = 5;
        let region = SelRegion::new(rug_fuzz_0, rug_fuzz_1);
        debug_assert_eq!(region.is_caret(), true);
        let region = SelRegion::new(rug_fuzz_2, rug_fuzz_3);
        debug_assert_eq!(region.is_caret(), false);
        let region = SelRegion::new(rug_fuzz_4, rug_fuzz_5);
        debug_assert_eq!(region.is_caret(), true);
        let _rug_ed_tests_llm_16_669_rrrruuuugggg_test_is_caret = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_670 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_upstream() {
        let _rug_st_tests_llm_16_670_rrrruuuugggg_test_is_upstream = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 5;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 5;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 0;
        let rug_fuzz_7 = 0;
        let region1 = SelRegion::new(rug_fuzz_0, rug_fuzz_1)
            .with_affinity(Affinity::Upstream);
        debug_assert_eq!(region1.is_upstream(), true);
        let region2 = SelRegion::new(rug_fuzz_2, rug_fuzz_3)
            .with_affinity(Affinity::Downstream);
        debug_assert_eq!(region2.is_upstream(), false);
        let region3 = SelRegion::new(rug_fuzz_4, rug_fuzz_5)
            .with_affinity(Affinity::Upstream);
        debug_assert_eq!(region3.is_upstream(), true);
        let region4 = SelRegion::new(rug_fuzz_6, rug_fuzz_7)
            .with_affinity(Affinity::Downstream);
        debug_assert_eq!(region4.is_upstream(), false);
        let _rug_ed_tests_llm_16_670_rrrruuuugggg_test_is_upstream = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_671 {
    use super::*;
    use crate::*;
    #[test]
    fn test_max() {
        let _rug_st_tests_llm_16_671_rrrruuuugggg_test_max = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 20;
        let region = SelRegion {
            start: rug_fuzz_0,
            end: rug_fuzz_1,
            horiz: None,
            affinity: Affinity::default(),
        };
        debug_assert_eq!(region.max(), 20);
        let _rug_ed_tests_llm_16_671_rrrruuuugggg_test_max = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_672 {
    use super::*;
    use crate::*;
    use selection::SelRegion;
    #[test]
    fn test_merge_with() {
        let _rug_st_tests_llm_16_672_rrrruuuugggg_test_merge_with = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 5;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 8;
        let region1 = SelRegion::new(rug_fuzz_0, rug_fuzz_1);
        let region2 = SelRegion::new(rug_fuzz_2, rug_fuzz_3);
        let merged_region = region1.merge_with(region2);
        debug_assert_eq!(merged_region.start, 0);
        debug_assert_eq!(merged_region.end, 8);
        let _rug_ed_tests_llm_16_672_rrrruuuugggg_test_merge_with = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_673 {
    use super::*;
    use crate::*;
    #[test]
    fn test_min() {
        let _rug_st_tests_llm_16_673_rrrruuuugggg_test_min = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 10;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 5;
        let rug_fuzz_4 = 5;
        let rug_fuzz_5 = 5;
        let region = SelRegion::new(rug_fuzz_0, rug_fuzz_1);
        debug_assert_eq!(region.min(), 5);
        let region = SelRegion::new(rug_fuzz_2, rug_fuzz_3);
        debug_assert_eq!(region.min(), 5);
        let region = SelRegion::new(rug_fuzz_4, rug_fuzz_5);
        debug_assert_eq!(region.min(), 5);
        let _rug_ed_tests_llm_16_673_rrrruuuugggg_test_min = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_674 {
    use super::*;
    use crate::*;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_674_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 20;
        let start = rug_fuzz_0;
        let end = rug_fuzz_1;
        let region = SelRegion::new(start, end);
        debug_assert_eq!(region.start, start);
        debug_assert_eq!(region.end, end);
        debug_assert_eq!(region.horiz, None);
        debug_assert_eq!(region.affinity, Affinity::Downstream);
        let _rug_ed_tests_llm_16_674_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_675 {
    use super::*;
    use crate::*;
    use selection::{Affinity, SelRegion};
    #[test]
    fn test_should_merge() {
        let _rug_st_tests_llm_16_675_rrrruuuugggg_test_should_merge = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 5;
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 7;
        let rug_fuzz_4 = 7;
        let rug_fuzz_5 = 10;
        let rug_fuzz_6 = 10;
        let rug_fuzz_7 = 15;
        let sel_region1 = SelRegion::new(rug_fuzz_0, rug_fuzz_1)
            .with_affinity(Affinity::Downstream);
        let sel_region2 = SelRegion::new(rug_fuzz_2, rug_fuzz_3)
            .with_affinity(Affinity::Downstream);
        let sel_region3 = SelRegion::new(rug_fuzz_4, rug_fuzz_5)
            .with_affinity(Affinity::Downstream);
        let sel_region4 = SelRegion::new(rug_fuzz_6, rug_fuzz_7)
            .with_affinity(Affinity::Downstream);
        debug_assert_eq!(sel_region1.should_merge(sel_region2), true);
        debug_assert_eq!(sel_region2.should_merge(sel_region3), true);
        debug_assert_eq!(sel_region3.should_merge(sel_region4), false);
        debug_assert_eq!(sel_region4.should_merge(sel_region1), false);
        let _rug_ed_tests_llm_16_675_rrrruuuugggg_test_should_merge = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_676 {
    use super::*;
    use crate::*;
    #[test]
    fn test_with_affinity() {
        let _rug_st_tests_llm_16_676_rrrruuuugggg_test_with_affinity = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 5;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 5;
        let rug_fuzz_4 = 10;
        let rug_fuzz_5 = 10;
        let region = SelRegion::new(rug_fuzz_0, rug_fuzz_1)
            .with_affinity(Affinity::Downstream);
        debug_assert_eq!(region.affinity, Affinity::Downstream);
        let region = SelRegion::new(rug_fuzz_2, rug_fuzz_3)
            .with_affinity(Affinity::Upstream);
        debug_assert_eq!(region.affinity, Affinity::Upstream);
        let region = SelRegion::caret(rug_fuzz_4).with_affinity(Affinity::Downstream);
        debug_assert_eq!(region.affinity, Affinity::Downstream);
        let region = SelRegion::caret(rug_fuzz_5).with_affinity(Affinity::Upstream);
        debug_assert_eq!(region.affinity, Affinity::Upstream);
        let _rug_ed_tests_llm_16_676_rrrruuuugggg_test_with_affinity = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_681 {
    use selection::{SelRegion, Affinity};
    use super::*;
    use crate::*;
    #[test]
    fn test_add_region() {
        let _rug_st_tests_llm_16_681_rrrruuuugggg_test_add_region = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 5;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 15;
        let rug_fuzz_4 = 20;
        let rug_fuzz_5 = 25;
        let rug_fuzz_6 = 30;
        let rug_fuzz_7 = 35;
        let rug_fuzz_8 = 0;
        let rug_fuzz_9 = 1;
        let rug_fuzz_10 = 2;
        let rug_fuzz_11 = 3;
        let mut selection = Selection::new();
        let region1 = SelRegion::new(rug_fuzz_0, rug_fuzz_1)
            .with_affinity(Affinity::Downstream);
        let region2 = SelRegion::new(rug_fuzz_2, rug_fuzz_3)
            .with_affinity(Affinity::Upstream);
        let region3 = SelRegion::new(rug_fuzz_4, rug_fuzz_5)
            .with_affinity(Affinity::Downstream);
        let region4 = SelRegion::new(rug_fuzz_6, rug_fuzz_7)
            .with_affinity(Affinity::Downstream);
        selection.add_region(region1);
        selection.add_region(region2);
        selection.add_region(region3);
        selection.add_region(region4);
        debug_assert_eq!(selection.regions.len(), 4);
        debug_assert_eq!(
            selection.regions[rug_fuzz_8], SelRegion::new(0, 5)
            .with_affinity(Affinity::Downstream)
        );
        debug_assert_eq!(
            selection.regions[rug_fuzz_9], SelRegion::new(10, 15)
            .with_affinity(Affinity::Upstream)
        );
        debug_assert_eq!(
            selection.regions[rug_fuzz_10], SelRegion::new(20, 25)
            .with_affinity(Affinity::Downstream)
        );
        debug_assert_eq!(
            selection.regions[rug_fuzz_11], SelRegion::new(30, 35)
            .with_affinity(Affinity::Downstream)
        );
        let _rug_ed_tests_llm_16_681_rrrruuuugggg_test_add_region = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_684 {
    use super::*;
    use crate::*;
    #[test]
    fn test_clear() {
        let _rug_st_tests_llm_16_684_rrrruuuugggg_test_clear = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 5;
        let mut selection = Selection::new_simple(
            SelRegion::new(rug_fuzz_0, rug_fuzz_1),
        );
        selection.clear();
        debug_assert_eq!(selection.len(), 0);
        let _rug_ed_tests_llm_16_684_rrrruuuugggg_test_clear = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_685 {
    use super::*;
    use crate::*;
    #[test]
    fn test_collapse() {
        let _rug_st_tests_llm_16_685_rrrruuuugggg_test_collapse = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 10;
        let rug_fuzz_2 = 20;
        let rug_fuzz_3 = 25;
        let rug_fuzz_4 = 30;
        let rug_fuzz_5 = 35;
        let rug_fuzz_6 = 0;
        let rug_fuzz_7 = 0;
        let mut selection = Selection::new_simple(
            SelRegion::new(rug_fuzz_0, rug_fuzz_1),
        );
        selection.add_region(SelRegion::new(rug_fuzz_2, rug_fuzz_3));
        selection.add_region(SelRegion::new(rug_fuzz_4, rug_fuzz_5));
        selection.collapse();
        debug_assert_eq!(selection.len(), 1);
        debug_assert_eq!(selection[rug_fuzz_6].start, 10);
        debug_assert_eq!(selection[rug_fuzz_7].end, 10);
        let _rug_ed_tests_llm_16_685_rrrruuuugggg_test_collapse = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_687 {
    use crate::selection::{Selection, SelRegion};
    #[test]
    fn test_delete_range() {
        let _rug_st_tests_llm_16_687_rrrruuuugggg_test_delete_range = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 10;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 5;
        let rug_fuzz_4 = false;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 0;
        let rug_fuzz_7 = 0;
        let rug_fuzz_8 = 10;
        let rug_fuzz_9 = 0;
        let rug_fuzz_10 = 5;
        let rug_fuzz_11 = true;
        let rug_fuzz_12 = 0;
        let rug_fuzz_13 = 0;
        let rug_fuzz_14 = 0;
        let rug_fuzz_15 = 10;
        let rug_fuzz_16 = 5;
        let rug_fuzz_17 = 10;
        let rug_fuzz_18 = false;
        let rug_fuzz_19 = 0;
        let rug_fuzz_20 = 0;
        let rug_fuzz_21 = 0;
        let rug_fuzz_22 = 10;
        let rug_fuzz_23 = 5;
        let rug_fuzz_24 = 10;
        let rug_fuzz_25 = true;
        let rug_fuzz_26 = 0;
        let rug_fuzz_27 = 0;
        let rug_fuzz_28 = 0;
        let rug_fuzz_29 = 10;
        let rug_fuzz_30 = 0;
        let rug_fuzz_31 = 10;
        let rug_fuzz_32 = false;
        let rug_fuzz_33 = 0;
        let rug_fuzz_34 = 10;
        let rug_fuzz_35 = 0;
        let rug_fuzz_36 = 10;
        let rug_fuzz_37 = true;
        let rug_fuzz_38 = 0;
        let rug_fuzz_39 = 10;
        let rug_fuzz_40 = 10;
        let rug_fuzz_41 = 20;
        let rug_fuzz_42 = false;
        let rug_fuzz_43 = 0;
        let rug_fuzz_44 = 0;
        let rug_fuzz_45 = 0;
        let rug_fuzz_46 = 10;
        let rug_fuzz_47 = 10;
        let rug_fuzz_48 = 20;
        let rug_fuzz_49 = true;
        let rug_fuzz_50 = 0;
        let rug_fuzz_51 = 0;
        let rug_fuzz_52 = 0;
        let rug_fuzz_53 = 10;
        let rug_fuzz_54 = 15;
        let rug_fuzz_55 = 20;
        let rug_fuzz_56 = false;
        let rug_fuzz_57 = 0;
        let rug_fuzz_58 = 0;
        let rug_fuzz_59 = 0;
        let rug_fuzz_60 = 10;
        let rug_fuzz_61 = 15;
        let rug_fuzz_62 = 20;
        let rug_fuzz_63 = true;
        let rug_fuzz_64 = 0;
        let rug_fuzz_65 = 0;
        let mut selection = Selection::new_simple(
            SelRegion::new(rug_fuzz_0, rug_fuzz_1),
        );
        selection.delete_range(rug_fuzz_2, rug_fuzz_3, rug_fuzz_4);
        debug_assert_eq!(selection.regions.len(), 1);
        debug_assert_eq!(selection.regions[rug_fuzz_5].start, 5);
        debug_assert_eq!(selection.regions[rug_fuzz_6].end, 10);
        let mut selection = Selection::new_simple(
            SelRegion::new(rug_fuzz_7, rug_fuzz_8),
        );
        selection.delete_range(rug_fuzz_9, rug_fuzz_10, rug_fuzz_11);
        debug_assert_eq!(selection.regions.len(), 1);
        debug_assert_eq!(selection.regions[rug_fuzz_12].start, 5);
        debug_assert_eq!(selection.regions[rug_fuzz_13].end, 10);
        let mut selection = Selection::new_simple(
            SelRegion::new(rug_fuzz_14, rug_fuzz_15),
        );
        selection.delete_range(rug_fuzz_16, rug_fuzz_17, rug_fuzz_18);
        debug_assert_eq!(selection.regions.len(), 1);
        debug_assert_eq!(selection.regions[rug_fuzz_19].start, 0);
        debug_assert_eq!(selection.regions[rug_fuzz_20].end, 5);
        let mut selection = Selection::new_simple(
            SelRegion::new(rug_fuzz_21, rug_fuzz_22),
        );
        selection.delete_range(rug_fuzz_23, rug_fuzz_24, rug_fuzz_25);
        debug_assert_eq!(selection.regions.len(), 1);
        debug_assert_eq!(selection.regions[rug_fuzz_26].start, 0);
        debug_assert_eq!(selection.regions[rug_fuzz_27].end, 5);
        let mut selection = Selection::new_simple(
            SelRegion::new(rug_fuzz_28, rug_fuzz_29),
        );
        selection.delete_range(rug_fuzz_30, rug_fuzz_31, rug_fuzz_32);
        debug_assert_eq!(selection.regions.len(), 0);
        let mut selection = Selection::new_simple(
            SelRegion::new(rug_fuzz_33, rug_fuzz_34),
        );
        selection.delete_range(rug_fuzz_35, rug_fuzz_36, rug_fuzz_37);
        debug_assert_eq!(selection.regions.len(), 0);
        let mut selection = Selection::new_simple(
            SelRegion::new(rug_fuzz_38, rug_fuzz_39),
        );
        selection.delete_range(rug_fuzz_40, rug_fuzz_41, rug_fuzz_42);
        debug_assert_eq!(selection.regions.len(), 1);
        debug_assert_eq!(selection.regions[rug_fuzz_43].start, 0);
        debug_assert_eq!(selection.regions[rug_fuzz_44].end, 10);
        let mut selection = Selection::new_simple(
            SelRegion::new(rug_fuzz_45, rug_fuzz_46),
        );
        selection.delete_range(rug_fuzz_47, rug_fuzz_48, rug_fuzz_49);
        debug_assert_eq!(selection.regions.len(), 1);
        debug_assert_eq!(selection.regions[rug_fuzz_50].start, 0);
        debug_assert_eq!(selection.regions[rug_fuzz_51].end, 10);
        let mut selection = Selection::new_simple(
            SelRegion::new(rug_fuzz_52, rug_fuzz_53),
        );
        selection.delete_range(rug_fuzz_54, rug_fuzz_55, rug_fuzz_56);
        debug_assert_eq!(selection.regions.len(), 1);
        debug_assert_eq!(selection.regions[rug_fuzz_57].start, 0);
        debug_assert_eq!(selection.regions[rug_fuzz_58].end, 10);
        let mut selection = Selection::new_simple(
            SelRegion::new(rug_fuzz_59, rug_fuzz_60),
        );
        selection.delete_range(rug_fuzz_61, rug_fuzz_62, rug_fuzz_63);
        debug_assert_eq!(selection.regions.len(), 1);
        debug_assert_eq!(selection.regions[rug_fuzz_64].start, 0);
        debug_assert_eq!(selection.regions[rug_fuzz_65].end, 10);
        let _rug_ed_tests_llm_16_687_rrrruuuugggg_test_delete_range = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_688 {
    use super::*;
    use crate::*;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_688_rrrruuuugggg_test_new = 0;
        let selection = Selection::new();
        debug_assert_eq!(selection.regions.len(), 0);
        let _rug_ed_tests_llm_16_688_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_689 {
    use super::*;
    use crate::*;
    use selection::{Affinity, SelRegion, Selection};
    #[test]
    fn test_new_simple() {
        let _rug_st_tests_llm_16_689_rrrruuuugggg_test_new_simple = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 5;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let region = SelRegion::new(rug_fuzz_0, rug_fuzz_1)
            .with_affinity(Affinity::Downstream);
        let selection = Selection::new_simple(region);
        debug_assert_eq!(selection.len(), 1);
        debug_assert_eq!(selection[rug_fuzz_2].start, 0);
        debug_assert_eq!(selection[rug_fuzz_3].end, 5);
        debug_assert_eq!(selection[rug_fuzz_4].affinity, Affinity::Downstream);
        let _rug_ed_tests_llm_16_689_rrrruuuugggg_test_new_simple = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_690 {
    use crate::selection::*;
    #[test]
    fn test_regions_in_range() {
        let _rug_st_tests_llm_16_690_rrrruuuugggg_test_regions_in_range = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 5;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 5;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 0;
        let rug_fuzz_7 = 10;
        let rug_fuzz_8 = 0;
        let rug_fuzz_9 = 0;
        let rug_fuzz_10 = 5;
        let rug_fuzz_11 = 10;
        let rug_fuzz_12 = 0;
        let rug_fuzz_13 = 5;
        let rug_fuzz_14 = 10;
        let rug_fuzz_15 = 15;
        let rug_fuzz_16 = 20;
        let rug_fuzz_17 = 25;
        let rug_fuzz_18 = 0;
        let rug_fuzz_19 = 10;
        let rug_fuzz_20 = 0;
        let rug_fuzz_21 = 0;
        let rug_fuzz_22 = 0;
        let rug_fuzz_23 = 25;
        let rug_fuzz_24 = 10;
        let rug_fuzz_25 = 20;
        let sel = Selection::new_simple(SelRegion::new(rug_fuzz_0, rug_fuzz_1));
        let result = sel.regions_in_range(rug_fuzz_2, rug_fuzz_3);
        debug_assert_eq!(result.len(), 1);
        debug_assert_eq!(result[rug_fuzz_4].start, 0);
        debug_assert_eq!(result[rug_fuzz_5].end, 5);
        let result = sel.regions_in_range(rug_fuzz_6, rug_fuzz_7);
        debug_assert_eq!(result.len(), 1);
        debug_assert_eq!(result[rug_fuzz_8].start, 0);
        debug_assert_eq!(result[rug_fuzz_9].end, 5);
        let result = sel.regions_in_range(rug_fuzz_10, rug_fuzz_11);
        debug_assert_eq!(result.len(), 0);
        let sel = Selection::new_simple(SelRegion::new(rug_fuzz_12, rug_fuzz_13));
        let sel = Selection::new_simple(SelRegion::new(rug_fuzz_14, rug_fuzz_15));
        let sel = Selection::new_simple(SelRegion::new(rug_fuzz_16, rug_fuzz_17));
        let result = sel.regions_in_range(rug_fuzz_18, rug_fuzz_19);
        debug_assert_eq!(result.len(), 1);
        debug_assert_eq!(result[rug_fuzz_20].start, 0);
        debug_assert_eq!(result[rug_fuzz_21].end, 5);
        let result = sel.regions_in_range(rug_fuzz_22, rug_fuzz_23);
        debug_assert_eq!(result.len(), 3);
        let result = sel.regions_in_range(rug_fuzz_24, rug_fuzz_25);
        debug_assert_eq!(result.len(), 0);
        let _rug_ed_tests_llm_16_690_rrrruuuugggg_test_regions_in_range = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_691 {
    use crate::selection::Selection;
    use crate::selection::SelRegion;
    #[test]
    fn test_selection_search_empty_regions() {
        let _rug_st_tests_llm_16_691_rrrruuuugggg_test_selection_search_empty_regions = 0;
        let rug_fuzz_0 = 10;
        let selection = Selection::new();
        let offset = rug_fuzz_0;
        let result = selection.search(offset);
        debug_assert_eq!(result, 0);
        let _rug_ed_tests_llm_16_691_rrrruuuugggg_test_selection_search_empty_regions = 0;
    }
    #[test]
    fn test_selection_search_offset_greater_than_last_region_max() {
        let _rug_st_tests_llm_16_691_rrrruuuugggg_test_selection_search_offset_greater_than_last_region_max = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 5;
        let rug_fuzz_2 = 10;
        let region = SelRegion::new(rug_fuzz_0, rug_fuzz_1);
        let selection = Selection::new_simple(region);
        let offset = rug_fuzz_2;
        let result = selection.search(offset);
        debug_assert_eq!(result, 1);
        let _rug_ed_tests_llm_16_691_rrrruuuugggg_test_selection_search_offset_greater_than_last_region_max = 0;
    }
    #[test]
    fn test_selection_search_offset_found() {
        let _rug_st_tests_llm_16_691_rrrruuuugggg_test_selection_search_offset_found = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 5;
        let rug_fuzz_2 = 8;
        let rug_fuzz_3 = 10;
        let rug_fuzz_4 = 7;
        let region1 = SelRegion::new(rug_fuzz_0, rug_fuzz_1);
        let region2 = SelRegion::new(rug_fuzz_2, rug_fuzz_3);
        let regions = vec![region1, region2];
        let selection = Selection { regions };
        let offset = rug_fuzz_4;
        let result = selection.search(offset);
        debug_assert_eq!(result, 1);
        let _rug_ed_tests_llm_16_691_rrrruuuugggg_test_selection_search_offset_found = 0;
    }
    #[test]
    fn test_selection_search_offset_not_found() {
        let _rug_st_tests_llm_16_691_rrrruuuugggg_test_selection_search_offset_not_found = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 5;
        let rug_fuzz_2 = 8;
        let rug_fuzz_3 = 10;
        let rug_fuzz_4 = 15;
        let region1 = SelRegion::new(rug_fuzz_0, rug_fuzz_1);
        let region2 = SelRegion::new(rug_fuzz_2, rug_fuzz_3);
        let regions = vec![region1, region2];
        let selection = Selection { regions };
        let offset = rug_fuzz_4;
        let result = selection.search(offset);
        debug_assert_eq!(result, 2);
        let _rug_ed_tests_llm_16_691_rrrruuuugggg_test_selection_search_offset_not_found = 0;
    }
}
