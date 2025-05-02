// Copyright 2017 The xi-editor Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! A data structure for representing multi-subsets of sequences (typically strings).

use std::cmp;

// These two imports are for the `apply` method only.
use crate::interval::Interval;
use crate::tree::{Node, NodeInfo, TreeBuilder};
use std::fmt;
use std::slice;

#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
struct Segment {
    len: usize,
    count: usize,
}

/// Represents a multi-subset of a string, that is a subset where elements can
/// be included multiple times. This is represented as each element of the
/// string having a "count" which is the number of times that element is
/// included in the set.
///
/// Internally, this is stored as a list of "segments" with a length and a count.
#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Subset {
    /// Invariant, maintained by `SubsetBuilder`: all `Segment`s have non-zero
    /// length, and no `Segment` has the same count as the one before it.
    segments: Vec<Segment>,
}

#[derive(Default)]
pub struct SubsetBuilder {
    segments: Vec<Segment>,
    total_len: usize,
}

impl SubsetBuilder {
    pub fn new() -> SubsetBuilder {
        SubsetBuilder::default()
    }

    /// Intended for use with `add_range` to ensure the total length of the
    /// `Subset` corresponds to the document length.
    pub fn pad_to_len(&mut self, total_len: usize) {
        if total_len > self.total_len {
            let cur_len = self.total_len;
            self.push_segment(total_len - cur_len, 0);
        }
    }

    /// Sets the count for a given range. This method must be called with a
    /// non-empty range with `begin` not before the largest range or segment added
    /// so far. Gaps will be filled with a 0-count segment.
    pub fn add_range(&mut self, begin: usize, end: usize, count: usize) {
        assert!(begin >= self.total_len, "ranges must be added in non-decreasing order");
        // assert!(begin < end, "ranges added must be non-empty: [{},{})", begin, end);
        if begin >= end {
            return;
        }
        let len = end - begin;
        let cur_total_len = self.total_len;

        // add 0-count segment to fill any gap
        if begin > self.total_len {
            self.push_segment(begin - cur_total_len, 0);
        }

        self.push_segment(len, count);
    }

    /// Assign `count` to the next `len` elements in the string.
    /// Will panic if called with `len==0`.
    pub fn push_segment(&mut self, len: usize, count: usize) {
        assert!(len > 0, "can't push empty segment");
        self.total_len += len;

        // merge into previous segment if possible
        if let Some(last) = self.segments.last_mut() {
            if last.count == count {
                last.len += len;
                return;
            }
        }

        self.segments.push(Segment { len, count });
    }

    pub fn build(self) -> Subset {
        Subset { segments: self.segments }
    }
}

/// Determines which elements of a `Subset` a method applies to
/// based on the count of the element.
#[derive(Clone, Copy, Debug)]
pub enum CountMatcher {
    Zero,
    NonZero,
    All,
}

impl CountMatcher {
    fn matches(self, seg: &Segment) -> bool {
        match self {
            CountMatcher::Zero => (seg.count == 0),
            CountMatcher::NonZero => (seg.count != 0),
            CountMatcher::All => true,
        }
    }
}

impl Subset {
    /// Creates an empty `Subset` of a string of length `len`
    pub fn new(len: usize) -> Subset {
        let mut sb = SubsetBuilder::new();
        sb.pad_to_len(len);
        sb.build()
    }

    /// Mostly for testing.
    pub fn delete_from_string(&self, s: &str) -> String {
        let mut result = String::new();
        for (b, e) in self.range_iter(CountMatcher::Zero) {
            result.push_str(&s[b..e]);
        }
        result
    }

    // Maybe Subset should be a pure data structure and this method should
    // be a method of Node.
    /// Builds a version of `s` with all the elements in this `Subset` deleted from it.
    pub fn delete_from<N: NodeInfo>(&self, s: &Node<N>) -> Node<N> {
        let mut b = TreeBuilder::new();
        for (beg, end) in self.range_iter(CountMatcher::Zero) {
            s.push_subseq(&mut b, Interval::new(beg, end));
        }
        b.build()
    }

    /// The length of the resulting sequence after deleting this subset. A
    /// convenience alias for `self.count(CountMatcher::Zero)` to reduce
    /// thinking about what that means in the cases where the length after
    /// delete is what you want to know.
    ///
    /// `self.delete_from_string(s).len() = self.len(s.len())`
    pub fn len_after_delete(&self) -> usize {
        self.count(CountMatcher::Zero)
    }

    /// Count the total length of all the segments matching `matcher`.
    pub fn count(&self, matcher: CountMatcher) -> usize {
        self.segments.iter().filter(|seg| matcher.matches(seg)).map(|seg| seg.len).sum()
    }

    /// Convenience alias for `self.count(CountMatcher::All)`
    pub fn len(&self) -> usize {
        self.count(CountMatcher::All)
    }

    /// Determine whether the subset is empty.
    /// In this case deleting it would do nothing.
    pub fn is_empty(&self) -> bool {
        (self.segments.is_empty()) || ((self.segments.len() == 1) && (self.segments[0].count == 0))
    }

    /// Compute the union of two subsets. The count of an element in the
    /// result is the sum of the counts in the inputs.
    pub fn union(&self, other: &Subset) -> Subset {
        let mut sb = SubsetBuilder::new();
        for zseg in self.zip(other) {
            sb.push_segment(zseg.len, zseg.a_count + zseg.b_count);
        }
        sb.build()
    }

    /// Compute the difference of two subsets. The count of an element in the
    /// result is the subtraction of the counts of other from self.
    pub fn subtract(&self, other: &Subset) -> Subset {
        let mut sb = SubsetBuilder::new();
        for zseg in self.zip(other) {
            assert!(
                zseg.a_count >= zseg.b_count,
                "can't subtract {} from {}",
                zseg.a_count,
                zseg.b_count
            );
            sb.push_segment(zseg.len, zseg.a_count - zseg.b_count);
        }
        sb.build()
    }

    /// Compute the bitwise xor of two subsets, useful as a reversible
    /// difference. The count of an element in the result is the bitwise xor
    /// of the counts of the inputs. Unchanged segments will be 0.
    ///
    /// This works like set symmetric difference when all counts are 0 or 1
    /// but it extends nicely to the case of larger counts.
    pub fn bitxor(&self, other: &Subset) -> Subset {
        let mut sb = SubsetBuilder::new();
        for zseg in self.zip(other) {
            sb.push_segment(zseg.len, zseg.a_count ^ zseg.b_count);
        }
        sb.build()
    }

    /// Map the contents of `self` into the 0-regions of `other`.
    /// Precondition: `self.count(CountMatcher::All) == other.count(CountMatcher::Zero)`
    fn transform(&self, other: &Subset, union: bool) -> Subset {
        let mut sb = SubsetBuilder::new();
        let mut seg_iter = self.segments.iter();
        let mut cur_seg = Segment { len: 0, count: 0 };
        for oseg in &other.segments {
            if oseg.count > 0 {
                sb.push_segment(oseg.len, if union { oseg.count } else { 0 });
            } else {
                // fill 0-region with segments from self.
                let mut to_be_consumed = oseg.len;
                while to_be_consumed > 0 {
                    if cur_seg.len == 0 {
                        cur_seg = seg_iter
                            .next()
                            .expect("self must cover all 0-regions of other")
                            .clone();
                    }
                    // consume as much of the segment as possible and necessary
                    let to_consume = cmp::min(cur_seg.len, to_be_consumed);
                    sb.push_segment(to_consume, cur_seg.count);
                    to_be_consumed -= to_consume;
                    cur_seg.len -= to_consume;
                }
            }
        }
        assert_eq!(cur_seg.len, 0, "the 0-regions of other must be the size of self");
        assert_eq!(seg_iter.next(), None, "the 0-regions of other must be the size of self");
        sb.build()
    }

    /// Transform through coordinate transform represented by other.
    /// The equation satisfied is as follows:
    ///
    /// s1 = other.delete_from_string(s0)
    ///
    /// s2 = self.delete_from_string(s1)
    ///
    /// element in self.transform_expand(other).delete_from_string(s0) if (not in s1) or in s2
    pub fn transform_expand(&self, other: &Subset) -> Subset {
        self.transform(other, false)
    }

    /// The same as taking transform_expand and then unioning with `other`.
    pub fn transform_union(&self, other: &Subset) -> Subset {
        self.transform(other, true)
    }

    /// Transform subset through other coordinate transform, shrinking.
    /// The following equation is satisfied:
    ///
    /// C = A.transform_expand(B)
    ///
    /// B.transform_shrink(C).delete_from_string(C.delete_from_string(s)) =
    ///   A.delete_from_string(B.delete_from_string(s))
    pub fn transform_shrink(&self, other: &Subset) -> Subset {
        let mut sb = SubsetBuilder::new();
        // discard ZipSegments where the shrinking set has positive count
        for zseg in self.zip(other) {
            // TODO: should this actually do something like subtract counts?
            if zseg.b_count == 0 {
                sb.push_segment(zseg.len, zseg.a_count);
            }
        }
        sb.build()
    }

    /// Return an iterator over the ranges with a count matching the `matcher`.
    /// These will often be easier to work with than raw segments.
    pub fn range_iter(&self, matcher: CountMatcher) -> RangeIter {
        RangeIter { seg_iter: self.segments.iter(), consumed: 0, matcher }
    }

    /// Convenience alias for `self.range_iter(CountMatcher::Zero)`.
    /// Semantically iterates the ranges of the complement of this `Subset`.
    pub fn complement_iter(&self) -> RangeIter {
        self.range_iter(CountMatcher::Zero)
    }

    /// Return an iterator over `ZipSegment`s where each `ZipSegment` contains
    /// the count for both self and other in that range. The two `Subset`s
    /// must have the same total length.
    ///
    /// Each returned `ZipSegment` will differ in at least one count.
    pub fn zip<'a>(&'a self, other: &'a Subset) -> ZipIter<'a> {
        ZipIter {
            a_segs: self.segments.as_slice(),
            b_segs: other.segments.as_slice(),
            a_i: 0,
            b_i: 0,
            a_consumed: 0,
            b_consumed: 0,
            consumed: 0,
        }
    }

    /// Find the complement of this Subset. Every 0-count element will have a
    /// count of 1 and every non-zero element will have a count of 0.
    pub fn complement(&self) -> Subset {
        let mut sb = SubsetBuilder::new();
        for seg in &self.segments {
            if seg.count == 0 {
                sb.push_segment(seg.len, 1);
            } else {
                sb.push_segment(seg.len, 0);
            }
        }
        sb.build()
    }

    /// Return a `Mapper` that can be use to map coordinates in the document to coordinates
    /// in this `Subset`, but only in non-decreasing order for performance reasons.
    pub fn mapper(&self, matcher: CountMatcher) -> Mapper {
        Mapper {
            range_iter: self.range_iter(matcher),
            last_i: 0, // indices only need to be in non-decreasing order, not increasing
            cur_range: (0, 0), // will immediately try to consume next range
            subset_amount_consumed: 0,
        }
    }
}

impl fmt::Debug for Subset {
    /// Use the alternate flag (`#`) to print a more compact representation
    /// where each character represents the count of one element:
    /// '-' is 0, '#' is 1, 2-9 are digits, `+` is >9
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if f.alternate() {
            for s in &self.segments {
                let chr = if s.count == 0 {
                    '-'
                } else if s.count == 1 {
                    '#'
                } else if s.count <= 9 {
                    ((s.count as u8) + b'0') as char
                } else {
                    '+'
                };
                for _ in 0..s.len {
                    write!(f, "{}", chr)?;
                }
            }
            Ok(())
        } else {
            f.debug_tuple("Subset").field(&self.segments).finish()
        }
    }
}

pub struct RangeIter<'a> {
    seg_iter: slice::Iter<'a, Segment>,
    pub consumed: usize,
    matcher: CountMatcher,
}

impl<'a> Iterator for RangeIter<'a> {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<(usize, usize)> {
        while let Some(seg) = self.seg_iter.next() {
            self.consumed += seg.len;
            if self.matcher.matches(seg) {
                return Some((self.consumed - seg.len, self.consumed));
            }
        }
        None
    }
}

/// See `Subset::zip`
pub struct ZipIter<'a> {
    a_segs: &'a [Segment],
    b_segs: &'a [Segment],
    a_i: usize,
    b_i: usize,
    a_consumed: usize,
    b_consumed: usize,
    pub consumed: usize,
}

/// See `Subset::zip`
#[derive(Clone, Debug)]
pub struct ZipSegment {
    len: usize,
    a_count: usize,
    b_count: usize,
}

impl<'a> Iterator for ZipIter<'a> {
    type Item = ZipSegment;

    /// Consume as far as possible from `self.consumed` until reaching a
    /// segment boundary in either `Subset`, and return the resulting
    /// `ZipSegment`. Will panic if it reaches the end of one `Subset` before
    /// the other, that is when they have different total length.
    fn next(&mut self) -> Option<ZipSegment> {
        match (self.a_segs.get(self.a_i), self.b_segs.get(self.b_i)) {
            (None, None) => None,
            (None, Some(_)) | (Some(_), None) => {
                panic!("can't zip Subsets of different base lengths.")
            }
            (
                Some(&Segment { len: a_len, count: a_count }),
                Some(&Segment { len: b_len, count: b_count }),
            ) => {
                let len = match (a_len + self.a_consumed).cmp(&(b_len + self.b_consumed)) {
                    cmp::Ordering::Equal => {
                        self.a_consumed += a_len;
                        self.a_i += 1;
                        self.b_consumed += b_len;
                        self.b_i += 1;
                        self.a_consumed - self.consumed
                    }
                    cmp::Ordering::Less => {
                        self.a_consumed += a_len;
                        self.a_i += 1;
                        self.a_consumed - self.consumed
                    }
                    cmp::Ordering::Greater => {
                        self.b_consumed += b_len;
                        self.b_i += 1;
                        self.b_consumed - self.consumed
                    }
                };
                self.consumed += len;
                Some(ZipSegment { len, a_count, b_count })
            }
        }
    }
}

pub struct Mapper<'a> {
    range_iter: RangeIter<'a>,
    // Not actually necessary for computation, just for dynamic checking of invariant
    last_i: usize,
    cur_range: (usize, usize),
    pub subset_amount_consumed: usize,
}

impl<'a> Mapper<'a> {
    /// Map a coordinate in the document this subset corresponds to, to a
    /// coordinate in the subset matched by the `CountMatcher`. For example,
    /// if the Subset is a set of deletions and the matcher is
    /// `CountMatcher::NonZero`, this would map indices in the union string to
    /// indices in the tombstones string.
    ///
    /// Will return the closest coordinate in the subset if the index is not
    /// in the subset. If the coordinate is past the end of the subset it will
    /// return one more than the largest index in the subset (i.e the length).
    /// This behaviour is suitable for mapping closed-open intervals in a
    /// string to intervals in a subset of the string.
    ///
    /// In order to guarantee good performance, this method must be called
    /// with `i` values in non-decreasing order or it will panic. This allows
    /// the total cost to be O(n) where `n = max(calls,ranges)` over all times
    /// called on a single `Mapper`.
    pub fn doc_index_to_subset(&mut self, i: usize) -> usize {
        assert!(
            i >= self.last_i,
            "method must be called with i in non-decreasing order. i={}<{}=last_i",
            i,
            self.last_i
        );
        self.last_i = i;

        while i >= self.cur_range.1 {
            self.subset_amount_consumed += self.cur_range.1 - self.cur_range.0;
            self.cur_range = match self.range_iter.next() {
                Some(range) => range,
                // past the end of the subset
                None => {
                    // ensure we don't try to consume any more
                    self.cur_range = (usize::max_value(), usize::max_value());
                    return self.subset_amount_consumed;
                }
            }
        }

        if i >= self.cur_range.0 {
            let dist_in_range = i - self.cur_range.0;
            dist_in_range + self.subset_amount_consumed
        } else {
            // not in the subset
            self.subset_amount_consumed
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::multiset::*;
    use crate::test_helpers::find_deletions;

    const TEST_STR: &'static str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

    #[test]
    fn test_apply() {
        let mut sb = SubsetBuilder::new();
        for &(b, e) in &[
            (0, 1),
            (2, 4),
            (6, 11),
            (13, 14),
            (15, 18),
            (19, 23),
            (24, 26),
            (31, 32),
            (33, 35),
            (36, 37),
            (40, 44),
            (45, 48),
            (49, 51),
            (52, 57),
            (58, 59),
        ] {
            sb.add_range(b, e, 1);
        }
        sb.pad_to_len(TEST_STR.len());
        let s = sb.build();
        println!("{:?}", s);
        assert_eq!("145BCEINQRSTUWZbcdimpvxyz", s.delete_from_string(TEST_STR));
    }

    #[test]
    fn trivial() {
        let s = SubsetBuilder::new().build();
        assert!(s.is_empty());
    }

    #[test]
    fn test_find_deletions() {
        let substr = "015ABDFHJOPQVYdfgloprsuvz";
        let s = find_deletions(substr, TEST_STR);
        assert_eq!(substr, s.delete_from_string(TEST_STR));
        assert!(!s.is_empty())
    }

    #[test]
    fn test_complement() {
        let substr = "0456789DEFGHIJKLMNOPQRSTUVWXYZdefghijklmnopqrstuvw";
        let s = find_deletions(substr, TEST_STR);
        let c = s.complement();
        // deleting the complement of the deletions we found should yield the deletions
        assert_eq!("123ABCabcxyz", c.delete_from_string(TEST_STR));
    }

    #[test]
    fn test_mapper() {
        let substr = "469ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwz";
        let s = find_deletions(substr, TEST_STR);
        let mut m = s.mapper(CountMatcher::NonZero);
        // subset is {0123 5 78 xy}
        assert_eq!(0, m.doc_index_to_subset(0));
        assert_eq!(2, m.doc_index_to_subset(2));
        assert_eq!(2, m.doc_index_to_subset(2));
        assert_eq!(3, m.doc_index_to_subset(3));
        assert_eq!(4, m.doc_index_to_subset(4)); // not in subset
        assert_eq!(4, m.doc_index_to_subset(5));
        assert_eq!(5, m.doc_index_to_subset(7));
        assert_eq!(6, m.doc_index_to_subset(8));
        assert_eq!(6, m.doc_index_to_subset(8));
        assert_eq!(8, m.doc_index_to_subset(60));
        assert_eq!(9, m.doc_index_to_subset(61)); // not in subset
        assert_eq!(9, m.doc_index_to_subset(62)); // not in subset
    }

    #[test]
    #[should_panic(expected = "non-decreasing")]
    fn test_mapper_requires_non_decreasing() {
        let substr = "469ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvw";
        let s = find_deletions(substr, TEST_STR);
        let mut m = s.mapper(CountMatcher::NonZero);
        m.doc_index_to_subset(0);
        m.doc_index_to_subset(2);
        m.doc_index_to_subset(1);
    }

    #[test]
    fn union() {
        let s1 = find_deletions("024AEGHJKNQTUWXYZabcfgikqrvy", TEST_STR);
        let s2 = find_deletions("14589DEFGIKMOPQRUXZabcdefglnpsuxyz", TEST_STR);
        assert_eq!("4EGKQUXZabcfgy", s1.union(&s2).delete_from_string(TEST_STR));
    }

    fn transform_case(str1: &str, str2: &str, result: &str) {
        let s1 = find_deletions(str1, TEST_STR);
        let s2 = find_deletions(str2, str1);
        let s3 = s2.transform_expand(&s1);
        let str3 = s3.delete_from_string(TEST_STR);
        assert_eq!(result, str3);
        assert_eq!(str2, s1.transform_shrink(&s3).delete_from_string(&str3));
        assert_eq!(str2, s2.transform_union(&s1).delete_from_string(TEST_STR));
    }

    #[test]
    fn transform() {
        transform_case(
            "02345678BCDFGHKLNOPQRTUVXZbcefghjlmnopqrstwx",
            "027CDGKLOTUbcegopqrw",
            "01279ACDEGIJKLMOSTUWYabcdegikopqruvwyz",
        );
        transform_case(
            "01234678DHIKLMNOPQRUWZbcdhjostvy",
            "136KLPQZvy",
            "13569ABCEFGJKLPQSTVXYZaefgiklmnpqruvwxyz",
        );
        transform_case(
            "0125789BDEFIJKLMNPVXabdjmrstuwy",
            "12BIJVXjmrstu",
            "12346ABCGHIJOQRSTUVWXYZcefghijklmnopqrstuvxz",
        );
        transform_case(
            "12456789ABCEFGJKLMNPQRSTUVXYadefghkrtwxz",
            "15ACEFGKLPRUVYdhrtx",
            "0135ACDEFGHIKLOPRUVWYZbcdhijlmnopqrstuvxy",
        );
        transform_case(
            "0128ABCDEFGIJMNOPQXYZabcfgijkloqruvy",
            "2CEFGMZabijloruvy",
            "2345679CEFGHKLMRSTUVWZabdehijlmnoprstuvwxyz",
        );
        transform_case(
            "01245689ABCDGJKLMPQSTWXYbcdfgjlmnosvy",
            "01245ABCDJLQSWXYgsv",
            "0123457ABCDEFHIJLNOQRSUVWXYZaeghikpqrstuvwxz",
        );
    }
}
#[cfg(test)]
mod tests_llm_16_55_llm_16_54 {
    use super::*;

use crate::*;
    use crate::multiset::{CountMatcher, RangeIter};

    #[test]
    fn test_next() {
        let segments = vec![
            Segment { count: 0, len: 2 },
            Segment { count: 0, len: 3 },
            Segment { count: 1, len: 4 },
            Segment { count: 0, len: 2 },
            Segment { count: 2, len: 5 },
        ];
        let matcher = CountMatcher::Zero;
        let mut range_iter = RangeIter {
            seg_iter: segments.iter(),
            consumed: 0,
            matcher,
        };
        assert_eq!(range_iter.next(), Some((0, 2)));
        assert_eq!(range_iter.next(), Some((2, 5)));
        assert_eq!(range_iter.next(), None);
    }
}#[cfg(test)]
mod tests_llm_16_301 {
    use super::*;

use crate::*;
    use multiset::{CountMatcher, Segment};

    #[test]
    fn test_matches_zero() {
        let matcher = CountMatcher::Zero;
        let segment = Segment { len: 10, count: 0 };
        assert_eq!(matcher.matches(&segment), true);
    }

    #[test]
    fn test_matches_zero_failure() {
        let matcher = CountMatcher::Zero;
        let segment = Segment { len: 10, count: 1 };
        assert_eq!(matcher.matches(&segment), false);
    }

    #[test]
    fn test_matches_non_zero() {
        let matcher = CountMatcher::NonZero;
        let segment = Segment { len: 10, count: 1 };
        assert_eq!(matcher.matches(&segment), true);
    }

    #[test]
    fn test_matches_non_zero_failure() {
        let matcher = CountMatcher::NonZero;
        let segment = Segment { len: 10, count: 0 };
        assert_eq!(matcher.matches(&segment), false);
    }

    #[test]
    fn test_matches_all() {
        let matcher = CountMatcher::All;
        let segment = Segment { len: 10, count: 0 };
        assert_eq!(matcher.matches(&segment), true);
    }
}#[cfg(test)]
mod tests_llm_16_302 {
    use super::*;

use crate::*;
    use crate::multiset::{CountMatcher, Mapper, RangeIter};

    #[test]
    fn test_doc_index_to_subset() {
        let seg1 = Segment { count: 0, len: 5 };
        let seg2 = Segment { count: 1, len: 3 };
        let seg3 = Segment { count: 2, len: 4 };
        let segments = vec![seg1, seg2, seg3];
        let matcher = CountMatcher::NonZero;
        let range_iter = RangeIter {
            seg_iter: segments.iter(),
            consumed: 0,
            matcher,
        };
        let mut mapper = Mapper {
            range_iter,
            last_i: 0,
            cur_range: (0, 0),
            subset_amount_consumed: 0,
        };

        let result = mapper.doc_index_to_subset(3);
        assert_eq!(result, 0);
    }
}#[cfg(test)]
mod tests_llm_16_303 {
    use super::*;

use crate::*;
    use crate::multiset::Segment;
    use crate::multiset::Subset;
    
    #[test]
    fn test_bitxor() {
        let subset1 = Subset { segments: vec![
            Segment { len: 1, count: 1 },
            Segment { len: 2, count: 2 },
            Segment { len: 3, count: 3 },
        ] };
        
        let subset2 = Subset { segments: vec![
            Segment { len: 1, count: 2 },
            Segment { len: 2, count: 1 },
            Segment { len: 3, count: 3 },
        ] };
        
        let expected_subset = Subset { segments: vec![
            Segment { len: 1, count: 3 },
            Segment { len: 2, count: 3 },
            Segment { len: 3, count: 0 },
        ] };
        
        let result_subset = subset1.bitxor(&subset2);
        
        assert_eq!(result_subset, expected_subset);
    }
}#[cfg(test)]
mod tests_llm_16_304 {
    use super::*;

use crate::*;
  
    #[test]
    fn test_subset_complement() {
        let subset = Subset {
            segments: vec![
                Segment { len: 3, count: 0 },
                Segment { len: 2, count: 1 },
                Segment { len: 4, count: 2 },
            ],
        };
        let complement = subset.complement();
        assert_eq!(
            complement,
            Subset {
                segments: vec![
                    Segment { len: 3, count: 1 },
                    Segment { len: 2, count: 0 },
                    Segment { len: 4, count: 0 },
                ],
            }
        );
    }
}#[cfg(test)]
mod tests_llm_16_305 {
    use super::*;

use crate::*;
    use multiset::Segment;

    #[test]
    fn test_complement_iter() {
        let subset = Subset {
            segments: vec![
                Segment { len: 3, count: 0 },
                Segment { len: 5, count: 1 },
                Segment { len: 2, count: 0 },
            ],
        };

        let complement: Vec<(usize, usize)> = subset.complement_iter().collect();
        let expected_complement = vec![(0, 3), (10, 12)];

        assert_eq!(complement, expected_complement);
    }
}
#[cfg(test)]
mod tests_llm_16_310 {
    use super::*;

use crate::*;
    use crate::multiset::CountMatcher;

    #[test]
    fn test_delete_from_string() {
        let subset = Subset::new(10);
        let s = "hello world";
        let result = subset.delete_from_string(s);
        assert_eq!(result, "");
    }
}#[cfg(test)]
mod tests_llm_16_314 {
    use super::*;

use crate::*;
    use crate::multiset::{CountMatcher, Subset, Segment};

    #[test]
    fn test_len() {
        let subset = Subset::new(10);
        assert_eq!(subset.len(), 0);

        let mut subset = Subset::new(10);
        subset.segments.push(Segment { len: 5, count: 2 });
        assert_eq!(subset.len(), 5);

        let mut subset = Subset::new(10);
        subset.segments.push(Segment { len: 5, count: 0 });
        assert_eq!(subset.len(), 0);

        let mut subset = Subset::new(10);
        subset.segments.push(Segment { len: 2, count: 1 });
        subset.segments.push(Segment { len: 3, count: 3 });
        assert_eq!(subset.len(), 5);
    }
}#[cfg(test)]
mod tests_llm_16_315 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_len_after_delete() {
        let subset = Subset::new(5);
        let len = subset.len_after_delete();
        assert_eq!(len, 5);
    }
}#[cfg(test)]
mod tests_llm_16_316 {
    use crate::multiset::{Subset, CountMatcher};
    
    #[test]
    fn test_mapper() {
        let subset = Subset::new(10);
        let matcher = CountMatcher::NonZero;
        let mapper = subset.mapper(matcher);
        // Perform assertions
    }
}#[cfg(test)]
mod tests_llm_16_317 {
    use super::*;

use crate::*;

    #[test]
    fn test_new() {
        let subset = Subset::new(10);
        assert_eq!(subset.segments.len(), 0);
    }
}#[cfg(test)]
mod tests_llm_16_319 {
    use crate::multiset::{Subset, CountMatcher, Segment};

    #[test]
    fn test_range_iter() {
        let segments = vec![
            Segment { len: 2, count: 1 },
            Segment { len: 3, count: 0 },
            Segment { len: 1, count: 2 },
        ];
        let subset = Subset { segments };

        let iter = subset.range_iter(CountMatcher::Zero);
        let result: Vec<(usize, usize)> = iter.collect();

        assert_eq!(result, vec![(2, 5)]);
    }
}
#[cfg(test)]
mod tests_llm_16_320 {
    use super::*;

use crate::*;
    use crate::multiset::Subset;

    #[test]
    fn test_subtract() {
        let subset1 = Subset::new(5);
        let subset2 = Subset::new(5);
        let result = subset1.subtract(&subset2);
        assert_eq!(result.len(), 0);
    }
}#[cfg(test)]
mod tests_llm_16_322_llm_16_321 {
    use super::*;

use crate::*;
    use crate::multiset::{Segment, Subset, SubsetBuilder, CountMatcher};
    use crate::tree::Node;

    #[test]
    fn test_transform() {
        let self_segments = vec![Segment { len: 5, count: 0 }, Segment { len: 2, count: 1 }];
        let other_segments = vec![Segment { len: 5, count: 0 }, Segment { len: 2, count: 0 }];
        let self_subset = Subset { segments: self_segments };
        let other_subset = Subset { segments: other_segments };
        let expected_segments = vec![Segment { len: 5, count: 0 }, Segment { len: 2, count: 0 }];
        let expected_subset = Subset { segments: expected_segments };

        let result = self_subset.transform(&other_subset, false);
        assert_eq!(result, expected_subset);
    }
}#[cfg(test)]
mod tests_llm_16_324_llm_16_323 {
    use crate::multiset::{Subset, Segment};

    #[test]
    fn test_transform_expand() {
        let subset1 = Subset::new(10);
        let subset2 = Subset::new(10);
        let subset3 = subset1.transform_expand(&subset2);
        assert_eq!(subset3.len(), 10);
    }
}#[cfg(test)]
mod tests_llm_16_325 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_transform_shrink() {
        let subset1 = Subset {
            segments: vec![
                Segment { len: 3, count: 1 },
                Segment { len: 2, count: 2 },
                Segment { len: 2, count: 0 },
                Segment { len: 4, count: 1 },
            ],
        };
        let subset2 = Subset {
            segments: vec![
                Segment { len: 3, count: 1 },
                Segment { len: 2, count: 0 },
                Segment { len: 4, count: 1 },
            ],
        };
        let result = subset1.transform_shrink(&subset2);
        assert_eq!(
            result,
            Subset {
                segments: vec![
                    Segment { len: 2, count: 1 },
                    Segment { len: 4, count: 1 },
                ],
            }
        );
    }
}#[cfg(test)]
mod tests_llm_16_326 {
    use super::*;

use crate::*;
    use crate::multiset::Segment;

    #[test]
    fn test_transform_union() {
        let subset1 = Subset {
            segments: vec![Segment {
                len: 5,
                count: 1,
            }],
        };
        let subset2 = Subset {
            segments: vec![Segment {
                len: 5,
                count: 2,
            }],
        };
        let transformed = subset1.transform_union(&subset2);
        assert_eq!(transformed.segments.len(), 1);
        assert_eq!(transformed.segments[0].len, 5);
        assert_eq!(transformed.segments[0].count, 3);
    }
}#[cfg(test)]
mod tests_llm_16_330 {
    use super::*;

use crate::*;
    use std::cmp;
    use crate::multiset::{Segment, Subset};

    #[test]
    fn test_zip() {
        let segment1 = Segment { len: 5, count: 3 };
        let segment2 = Segment { len: 5, count: 2 };
        let subset1 = Subset { segments: vec![segment1] };
        let subset2 = Subset { segments: vec![segment2] };
        let mut zip_iter = subset1.zip(&subset2);

        let zip_segment = zip_iter.next().unwrap();
        assert_eq!(zip_segment.len, 5);
        assert_eq!(zip_segment.a_count, 3);
        assert_eq!(zip_segment.b_count, 2);

        assert!(zip_iter.next().is_none());
    }
}#[cfg(test)]
mod tests_llm_16_333 {
    use super::*;

use crate::*;
  
    #[test]
    fn test_build() {
        let builder = SubsetBuilder::new();
        let subset = builder.build();
        assert_eq!(subset.segments.len(), 0);
    }
}#[cfg(test)]
mod tests_llm_16_334 {
    use super::*;

use crate::*;

    #[test]
    fn test_new() {
        let subset_builder = SubsetBuilder::new();
        // Assert statements
    }
}#[cfg(test)]
mod tests_llm_16_335 {
    use super::*;

use crate::*;
   
    #[test]
    fn test_pad_to_len() {
        let mut subset_builder = SubsetBuilder::new();
        subset_builder.pad_to_len(10);
        let subset = subset_builder.build();
        assert_eq!(subset.segments.len(), 1);
        assert_eq!(subset.segments[0].len, 10);
        assert_eq!(subset.segments[0].count, 0);
    }
}#[cfg(test)]
mod tests_llm_16_336 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_push_segment() {
        let mut builder = SubsetBuilder::new();
        builder.push_segment(10, 5);

        let segment = builder.segments.pop().unwrap();
        assert_eq!(segment.len, 10);
        assert_eq!(segment.count, 5);
    }
}#[cfg(test)]
mod tests_rug_185 {
    use super::*;
    use rope::multiset::SubsetBuilder;
    
    #[test]
    fn test_add_range() {
        let mut p0: SubsetBuilder = SubsetBuilder::new();
        let p1: usize = 0; // sample data
        let p2: usize = 10; // sample data
        let p3: usize = 5; // sample data
        
        p0.add_range(p1, p2, p3);
    }
}
#[cfg(test)]
mod tests_rug_186 {
    use super::*;
    use rope::tree::Node; //import necessary module

    #[test]
    fn test_rug() {
        // Construct the first argument
        let mut p0 = multiset::Subset::new(10);

        // Construct the second argument
        let mut p1: Node<N> = Node::new();

        // Call the target function
        multiset::Subset::delete_from(&p0, &p1);
    }
}
#[cfg(test)]
mod tests_rug_187 {
    use super::*;
    use rope::multiset::{Subset, CountMatcher};
    
    #[test]
    fn test_rug() {
        let mut p0 = Subset::new(10);
        let mut p1 = CountMatcher::All;

        Subset::count(&mut p0, p1);
    }
}#[cfg(test)]
mod tests_rug_188 {
    use super::*;
    
    #[test]
    fn test_rug() {
        // Construct the variables
        let mut p0 = multiset::Subset::new(10); // create the local variable p0 with type multiset::Subset

        // Call the target function
        <multiset::Subset>::is_empty(&p0);
        
        // Assert the result if necessary
    }
}
#[cfg(test)]
mod tests_rug_189 {
    use super::*;
    use multiset::Subset;

    #[test]
    fn test_rug() {
        let mut p0 = multiset::Subset::new(10);
        let mut p1 = multiset::Subset::new(10);
        
        multiset::Subset::union(&p0, &p1);
    }
}
                        
#[cfg(test)]
mod tests_rug_190 {
    use super::*;
    use crate::std::iter::Iterator;
    use crate::rope::multiset; // import the relevant crate module
    
    #[test]
    fn test_rug() {
        let mut v81 = multiset::ZipIter::new(); // construct the multiset::ZipIter<'a> variable
        // you need to define and initialize the other required variables
        
        <multiset::ZipIter<'a> as std::iter::Iterator>::next(&mut v81); // call the target function with the constructed variables

    }
}
