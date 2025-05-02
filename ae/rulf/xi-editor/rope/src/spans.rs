// Copyright 2016 The xi-editor Authors.
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

//! A module for representing spans (in an interval tree), useful for rich text
//! annotations. It is parameterized over a data type, so can be used for
//! storing different annotations.

use std::fmt;
use std::marker::PhantomData;
use std::mem;

use crate::delta::{Delta, DeltaElement, Transformer};
use crate::interval::{Interval, IntervalBounds};
use crate::tree::{Cursor, Leaf, Node, NodeInfo, TreeBuilder};

const MIN_LEAF: usize = 32;
const MAX_LEAF: usize = 64;

pub type Spans<T> = Node<SpansInfo<T>>;

#[derive(Clone)]
pub struct Span<T: Clone> {
    iv: Interval,
    data: T,
}

#[derive(Clone)]
pub struct SpansLeaf<T: Clone> {
    len: usize, // measured in base units
    spans: Vec<Span<T>>,
}

// It would be preferable to derive Default.
// This would however require T to implement Default due to an issue in Rust.
// See: https://github.com/rust-lang/rust/issues/26925
impl<T: Clone> Default for SpansLeaf<T> {
    fn default() -> Self {
        SpansLeaf { len: 0, spans: vec![] }
    }
}

#[derive(Clone)]
pub struct SpansInfo<T> {
    n_spans: usize,
    iv: Interval,
    phantom: PhantomData<T>,
}

impl<T: Clone> Leaf for SpansLeaf<T> {
    fn len(&self) -> usize {
        self.len
    }

    fn is_ok_child(&self) -> bool {
        self.spans.len() >= MIN_LEAF
    }

    fn push_maybe_split(&mut self, other: &Self, iv: Interval) -> Option<Self> {
        let iv_start = iv.start();
        for span in &other.spans {
            let span_iv = span.iv.intersect(iv).translate_neg(iv_start).translate(self.len);

            if !span_iv.is_empty() {
                self.spans.push(Span { iv: span_iv, data: span.data.clone() });
            }
        }
        self.len += iv.size();

        if self.spans.len() <= MAX_LEAF {
            None
        } else {
            let splitpoint = self.spans.len() / 2; // number of spans
            let splitpoint_units = self.spans[splitpoint].iv.start();
            let mut new = self.spans.split_off(splitpoint);
            for span in &mut new {
                span.iv = span.iv.translate_neg(splitpoint_units);
            }
            let new_len = self.len - splitpoint_units;
            self.len = splitpoint_units;
            Some(SpansLeaf { len: new_len, spans: new })
        }
    }
}

impl<T: Clone> NodeInfo for SpansInfo<T> {
    type L = SpansLeaf<T>;

    fn accumulate(&mut self, other: &Self) {
        self.n_spans += other.n_spans;
        self.iv = self.iv.union(other.iv);
    }

    fn compute_info(l: &SpansLeaf<T>) -> Self {
        let mut iv = Interval::new(0, 0); // should be Interval::default?
        for span in &l.spans {
            iv = iv.union(span.iv);
        }
        SpansInfo { n_spans: l.spans.len(), iv, phantom: PhantomData }
    }
}

pub struct SpansBuilder<T: Clone> {
    b: TreeBuilder<SpansInfo<T>>,
    leaf: SpansLeaf<T>,
    len: usize,
    total_len: usize,
}

impl<T: Clone> SpansBuilder<T> {
    pub fn new(total_len: usize) -> Self {
        SpansBuilder { b: TreeBuilder::new(), leaf: SpansLeaf::default(), len: 0, total_len }
    }

    // Precondition: spans must be added in nondecreasing start order.
    // Maybe take Span struct instead of separate iv, data args?
    pub fn add_span<IV: IntervalBounds>(&mut self, iv: IV, data: T) {
        let iv = iv.into_interval(self.total_len);
        if self.leaf.spans.len() == MAX_LEAF {
            let mut leaf = mem::take(&mut self.leaf);
            leaf.len = iv.start() - self.len;
            self.len = iv.start();
            self.b.push(Node::from_leaf(leaf));
        }
        self.leaf.spans.push(Span { iv: iv.translate_neg(self.len), data })
    }

    // Would make slightly more implementation sense to take total_len as an argument
    // here, but that's not quite the usual builder pattern.
    pub fn build(mut self) -> Spans<T> {
        self.leaf.len = self.total_len - self.len;
        self.b.push(Node::from_leaf(self.leaf));
        self.b.build()
    }
}

pub struct SpanIter<'a, T: 'a + Clone> {
    cursor: Cursor<'a, SpansInfo<T>>,
    ix: usize,
}

impl<T: Clone> Spans<T> {
    /// Perform operational transformation on a spans object intended to be edited into
    /// a sequence at the given offset.

    // Note: this implementation is not efficient for very large Spans objects, as it
    // traverses all spans linearly. A more sophisticated approach would be to traverse
    // the tree, and only delve into subtrees that are transformed.
    pub fn transform<N: NodeInfo>(
        &self,
        base_start: usize,
        base_end: usize,
        xform: &mut Transformer<N>,
    ) -> Self {
        // TODO: maybe should take base as an Interval and figure out "after" from that
        let new_start = xform.transform(base_start, false);
        let new_end = xform.transform(base_end, true);
        let mut builder = SpansBuilder::new(new_end - new_start);
        for (iv, data) in self.iter() {
            let start = xform.transform(iv.start() + base_start, false) - new_start;
            let end = xform.transform(iv.end() + base_start, false) - new_start;
            if start < end {
                let iv = Interval::new(start, end);
                // TODO: could imagine using a move iterator and avoiding clone, but it's not easy.
                builder.add_span(iv, data.clone());
            }
        }
        builder.build()
    }

    /// Creates a new Spans instance by merging spans from `other` with `self`,
    /// using a closure to transform values.
    ///
    /// New spans are created from non-overlapping regions of existing spans,
    /// and by combining overlapping regions into new spans. In all cases,
    /// new values are generated by calling a closure that transforms the
    /// value of the existing span or spans.
    ///
    /// # Panics
    ///
    /// Panics if `self` and `other` have different lengths.
    ///
    pub fn merge<F, O>(&self, other: &Self, mut f: F) -> Spans<O>
    where
        F: FnMut(&T, Option<&T>) -> O,
        O: Clone,
    {
        //TODO: confirm that this is sensible behaviour
        assert_eq!(self.len(), other.len());
        let mut sb = SpansBuilder::new(self.len());

        // red/blue is just a better name than one/two or me/other
        let mut iter_red = self.iter();
        let mut iter_blue = other.iter();

        let mut next_red = iter_red.next();
        let mut next_blue = iter_blue.next();

        loop {
            // exit conditions:
            if next_red.is_none() && next_blue.is_none() {
                // all merged.
                break;
            } else if next_red.is_none() != next_blue.is_none() {
                // one side is exhausted; append remaining items from other side.
                let iter = if next_red.is_some() { iter_red } else { iter_blue };
                // add this item
                let (iv, val) = next_red.or(next_blue).unwrap();
                sb.add_span(iv, f(val, None));

                for (iv, val) in iter {
                    sb.add_span(iv, f(val, None))
                }
                break;
            }

            // body:
            let (mut red_iv, red_val) = next_red.unwrap();
            let (mut blue_iv, blue_val) = next_blue.unwrap();

            if red_iv.intersect(blue_iv).is_empty() {
                // spans do not overlap. Add the leading span & advance that iter.
                if red_iv.is_before(blue_iv.start()) {
                    sb.add_span(red_iv, f(red_val, None));
                    next_red = iter_red.next();
                } else {
                    sb.add_span(blue_iv, f(blue_val, None));
                    next_blue = iter_blue.next();
                }
                continue;
            }
            assert!(!red_iv.intersect(blue_iv).is_empty());

            // if these two spans do not share a start point, create a new span from
            // the prefix of the leading span.
            use std::cmp::Ordering;

            match red_iv.start().cmp(&blue_iv.start()) {
                Ordering::Less => {
                    let iv = red_iv.prefix(blue_iv);
                    sb.add_span(iv, f(red_val, None));
                    red_iv = red_iv.suffix(iv);
                }
                Ordering::Greater => {
                    let iv = blue_iv.prefix(red_iv);
                    sb.add_span(iv, f(blue_val, None));
                    blue_iv = blue_iv.suffix(iv);
                }
                Ordering::Equal => {}
            }

            assert!(red_iv.start() == blue_iv.start());
            // create a new span by merging the overlapping regions.
            let iv = red_iv.intersect(blue_iv);
            assert!(!iv.is_empty());
            sb.add_span(iv, f(red_val, Some(blue_val)));

            // if an old span was consumed by this new span, advance
            // else reuse remaining span (set next_red/blue) for the next loop iteration
            red_iv = red_iv.suffix(iv);
            blue_iv = blue_iv.suffix(iv);
            assert!(red_iv.is_empty() || blue_iv.is_empty());

            if red_iv.is_empty() {
                next_red = iter_red.next();
            } else {
                next_red = Some((red_iv, red_val));
            }

            if blue_iv.is_empty() {
                next_blue = iter_blue.next();
            } else {
                next_blue = Some((blue_iv, blue_val));
            }
        }
        sb.build()
    }

    // possible future: an iterator that takes an interval, so results are the same as
    // taking a subseq on the spans object. Would require specialized Cursor.
    pub fn iter(&self) -> SpanIter<T> {
        SpanIter { cursor: Cursor::new(self, 0), ix: 0 }
    }

    /// Applies a generic delta to `self`, inserting empty spans for any
    /// added regions.
    ///
    /// This is intended to be used to keep spans up to date with a `Rope`
    /// as edits occur.
    pub fn apply_shape<M: NodeInfo>(&mut self, delta: &Delta<M>) {
        let mut b = TreeBuilder::new();
        for elem in &delta.els {
            match *elem {
                DeltaElement::Copy(beg, end) => b.push(self.subseq(Interval::new(beg, end))),
                DeltaElement::Insert(ref n) => b.push(SpansBuilder::new(n.len()).build()),
            }
        }
        *self = b.build();
    }

    /// Deletes all spans that intersect with `interval` and that come after.
    pub fn delete_after(&mut self, interval: Interval) {
        let mut builder = SpansBuilder::new(self.len());

        for (iv, data) in self.iter() {
            // check if spans overlaps with interval
            if iv.intersect(interval).is_empty() {
                // keep the ones that are not overlapping
                builder.add_span(iv, data.clone());
            } else {
                // all remaining spans are invalid
                break;
            }
        }
        *self = builder.build();
    }
}

impl<T: Clone + fmt::Debug> fmt::Debug for Spans<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let strs =
            self.iter().map(|(iv, val)| format!("{}: {:?}", iv, val)).collect::<Vec<String>>();
        write!(f, "len: {}\nspans:\n\t{}", self.len(), &strs.join("\n\t"))
    }
}

impl<'a, T: Clone> Iterator for SpanIter<'a, T> {
    type Item = (Interval, &'a T);

    fn next(&mut self) -> Option<(Interval, &'a T)> {
        if let Some((leaf, start_pos)) = self.cursor.get_leaf() {
            if leaf.spans.is_empty() {
                return None;
            }
            let leaf_start = self.cursor.pos() - start_pos;
            let span = &leaf.spans[self.ix];
            self.ix += 1;
            if self.ix == leaf.spans.len() {
                let _ = self.cursor.next_leaf();
                self.ix = 0;
            }
            return Some((span.iv.translate(leaf_start), &span.data));
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]

    fn test_merge() {
        // merging 1 1 1 1 1 1 1 1 1 16
        // with    2 2 4 4     8 8
        // ==      3 3 5 5 1 1 9 9 1 16
        let mut sb = SpansBuilder::new(10);
        sb.add_span(Interval::new(0, 9), 1u32);
        sb.add_span(Interval::new(9, 10), 16);
        let red = sb.build();

        let mut sb = SpansBuilder::new(10);
        sb.add_span(Interval::new(0, 2), 2);
        sb.add_span(Interval::new(2, 4), 4);
        sb.add_span(Interval::new(6, 8), 8);
        let blue = sb.build();
        let merged = red.merge(&blue, |r, b| b.map(|b| b + r).unwrap_or(*r));

        let mut merged_iter = merged.iter();
        let (iv, val) = merged_iter.next().unwrap();
        assert_eq!(iv, Interval::new(0, 2));
        assert_eq!(*val, 3);

        let (iv, val) = merged_iter.next().unwrap();
        assert_eq!(iv, Interval::new(2, 4));
        assert_eq!(*val, 5);

        let (iv, val) = merged_iter.next().unwrap();
        assert_eq!(iv, Interval::new(4, 6));
        assert_eq!(*val, 1);

        let (iv, val) = merged_iter.next().unwrap();
        assert_eq!(iv, Interval::new(6, 8));
        assert_eq!(*val, 9);

        let (iv, val) = merged_iter.next().unwrap();
        assert_eq!(iv, Interval::new(8, 9));
        assert_eq!(*val, 1);

        let (iv, val) = merged_iter.next().unwrap();
        assert_eq!(iv, Interval::new(9, 10));
        assert_eq!(*val, 16);

        assert!(merged_iter.next().is_none());
    }

    #[test]
    fn test_merge_2() {
        // 1 1 1   4 4
        //   2 2 2 2     8 9
        let mut sb = SpansBuilder::new(9);
        sb.add_span(Interval::new(0, 3), 1);
        sb.add_span(Interval::new(4, 6), 4);
        let blue = sb.build();

        let mut sb = SpansBuilder::new(9);
        sb.add_span(Interval::new(1, 5), 2);
        sb.add_span(Interval::new(7, 8), 8);
        sb.add_span(Interval::new(8, 9), 9);
        let red = sb.build();

        let merged = red.merge(&blue, |r, b| b.map(|b| b + r).unwrap_or(*r));

        let mut merged_iter = merged.iter();
        let (iv, val) = merged_iter.next().unwrap();
        assert_eq!(iv, Interval::new(0, 1));
        assert_eq!(*val, 1);

        let (iv, val) = merged_iter.next().unwrap();
        assert_eq!(iv, Interval::new(1, 3));
        assert_eq!(*val, 3);

        let (iv, val) = merged_iter.next().unwrap();
        assert_eq!(iv, Interval::new(3, 4));
        assert_eq!(*val, 2);

        let (iv, val) = merged_iter.next().unwrap();
        assert_eq!(iv, Interval::new(4, 5));
        assert_eq!(*val, 6);

        let (iv, val) = merged_iter.next().unwrap();
        assert_eq!(iv, Interval::new(5, 6));
        assert_eq!(*val, 4);

        let (iv, val) = merged_iter.next().unwrap();
        assert_eq!(iv, Interval::new(7, 8));
        assert_eq!(*val, 8);

        let (iv, val) = merged_iter.next().unwrap();
        assert_eq!(iv, Interval::new(8, 9));
        assert_eq!(*val, 9);

        assert!(merged_iter.next().is_none());
    }

    #[test]
    fn test_delete_after() {
        let mut sb = SpansBuilder::new(11);
        sb.add_span(Interval::new(1, 2), 2);
        sb.add_span(Interval::new(3, 5), 8);
        sb.add_span(Interval::new(6, 8), 9);
        sb.add_span(Interval::new(9, 10), 1);
        sb.add_span(Interval::new(10, 11), 1);
        let mut spans = sb.build();

        spans.delete_after(Interval::new(4, 7));

        assert_eq!(spans.iter().count(), 1);

        let (iv, val) = spans.iter().next().unwrap();
        assert_eq!(iv, Interval::new(1, 2));
        assert_eq!(*val, 2);
    }

    #[test]
    fn delete_after_big_at_start() {
        let mut sb = SpansBuilder::new(10);
        sb.add_span(0..10, 0);

        let mut spans = sb.build();
        assert_eq!(spans.iter().count(), 1);

        spans.delete_after(Interval::new(1, 2));
        assert_eq!(spans.iter().count(), 0);
    }

    #[test]
    fn delete_after_big_and_small() {
        let mut sb = SpansBuilder::new(10);
        sb.add_span(0..10, 0);
        sb.add_span(3..10, 1);

        let mut spans = sb.build();
        assert_eq!(spans.iter().count(), 2);

        spans.delete_after(Interval::new(1, 2));
        assert_eq!(spans.iter().count(), 0);
    }

    #[test]
    fn delete_after_empty() {
        let mut sb = SpansBuilder::new(10);
        sb.add_span(0..3, 0);

        let mut spans = sb.build();
        assert_eq!(spans.iter().count(), 1);

        spans.delete_after(Interval::new(5, 7));
        assert_eq!(spans.iter().count(), 1);
    }
}
#[cfg(test)]
mod tests_llm_16_116_llm_16_115 {
    use std::cmp::{max, min};
    use std::default::Default;
    use std::marker::PhantomData;
    use std::ops::{Range, RangeInclusive, RangeTo, RangeToInclusive};
    use std::fmt;
    
    // Define the Interval struct
    #[derive(Clone, Eq, PartialEq, Debug)]
    pub struct Interval {
        pub start: usize,
        pub end: usize,
    }
    
    impl Interval {
        pub fn new(start: usize, end: usize) -> Interval {
            debug_assert!(start <= end);
            Interval { start, end }
        }
    
        pub fn union(&self, other: &Interval) -> Interval {
            let start = min(self.start, other.start);
            let end = max(self.end, other.end);
            Interval { start, end }
        }
    }
    
    // Define the SpansLeaf struct
    #[derive(Clone, Default)]
    struct SpansLeaf<T: Clone> {
        len: usize,
        spans: Vec<Span<T>>,
    }
    
    // Define the Span struct
    #[derive(Clone)]
    struct Span<T> {
        iv: Interval,
        data: T,
    }
    
    // Define the SpansInfo struct
    struct SpansInfo<T> {
        n_spans: usize,
        iv: Interval,
        phantom: PhantomData<T>,
    }
    
    // Define the NodeInfo trait
    trait NodeInfo {
        type L;
    
        fn accumulate(&mut self, other: &Self);
        fn compute_info(l: &Self::L) -> Self;
    }
    
    // Implement the NodeInfo trait for the SpansInfo struct
    impl<T: Clone> NodeInfo for SpansInfo<T> {
        type L = SpansLeaf<T>;
    
        fn accumulate(&mut self, other: &Self) {
            self.n_spans += other.n_spans;
            self.iv = self.iv.union(&other.iv);
        }
    
        fn compute_info(l: &SpansLeaf<T>) -> Self {
            let mut iv = Interval::new(0, 0);
            for span in &l.spans {
                iv = iv.union(&span.iv);
            }
            SpansInfo {
                n_spans: l.spans.len(),
                iv,
                phantom: PhantomData,
            }
        }
    }
    
    // Unit test for compute_info
    #[test]
    fn test_compute_info() {
        let l = SpansLeaf {
            len: 0,
            spans: vec![
                Span {
                    iv: Interval::new(1, 3),
                    data: "span1".to_string(),
                },
                Span {
                    iv: Interval::new(5, 7),
                    data: "span2".to_string(),
                },
            ],
        };
    
        let result = SpansInfo::compute_info(&l);
    
        assert_eq!(result.n_spans, 2);
        assert_eq!(result.iv.start, 1);
        assert_eq!(result.iv.end, 7);
    }
}#[cfg(test)]
mod tests_llm_16_119 {
    use super::*;

use crate::*;

    const MIN_LEAF: usize = 0; // define the value of MIN_LEAF here
    const MAX_LEAF: usize = 0; // define the value of MAX_LEAF here

    #[test]
    fn test_is_ok_child() {
        let spans_leaf = SpansLeaf::<i32>::default();
        assert_eq!(spans_leaf.is_ok_child(), false);
    }
}#[cfg(test)]
mod tests_llm_16_120 {
    use super::*;

use crate::*;
    use crate::breaks;
    use crate::spans;

    #[test]
    fn test_len() {
        let spans_leaf: spans::SpansLeaf<i32> = spans::SpansLeaf::default();
        let result = <spans::SpansLeaf<i32> as tree::Leaf>::len(&spans_leaf);
        assert_eq!(result, 0);
    }
}
#[cfg(test)]
mod tests_rug_191 {
    use super::*;
    use crate::rope::src::spans::{SpansLeaf, Spans};
    use std::default::Default;
    
    #[test]
    fn test_default() {
        let result: SpansLeaf<T> = <SpansLeaf<T> as Default>::default();
        
        // add your assertions here
    }
}
#[cfg(test)]
mod tests_rug_192 {
    use super::*;
    use rope::tree::Leaf;
    use rope::spans::{SpansLeaf, Span};
    use crate::interval::Interval;

    #[test]
    fn test_push_maybe_split() {
        let mut p0: SpansLeaf<T> = SpansLeaf::new();
        let mut p1: SpansLeaf<T> = SpansLeaf::new();
        let p2: Interval = Interval::new(2, 5);

        p0.push_maybe_split(&p1, p2);
    }
}#[cfg(test)]
mod tests_rug_193 {
    use super::*;
    use crate::tree::NodeInfo;
    use crate::rope::spans::SpansInfo;

    #[test]
    fn test_rug() {
        let mut p0: SpansInfo<T> = SpansInfo::new();
        let mut p1: SpansInfo<T> = SpansInfo::new();

        <spans::SpansInfo<T> as tree::NodeInfo>::accumulate(&mut p0, &p1);
    }
}        
#[cfg(test)]
mod tests_rug_194 {
    use super::*;
    
    #[test]
    fn test_rug() {
        let mut p0: usize = 10; // sample value
                
        spans::SpansBuilder::<T>::new(p0);
    }
}        
#[cfg(test)]
        mod tests_rug_195 {
            use super::*;
            use rope::spans::{SpansBuilder, span::Span};
            use rope::breaks::{BreaksLeaf};
            use std::mem;
            use rope::node::Node;
            use rope::{MAX_LEAF};
            
            #[test]
            fn test_rug() {
                // construct SpansBuilder
                let mut v84: SpansBuilder<T> = SpansBuilder::new();
                // construct RangeFull
                let mut v28: std::ops::RangeFull = ..;
                // construct BreaksLeaf
                let mut v50: BreaksLeaf = BreaksLeaf { len: 10, data: vec![1, 2, 3] };
                
                // call add_span function
                <spans::SpansBuilder<T>>::add_span(&mut v84, v28, v50);

            }
        }
        #[cfg(test)]
mod tests_rug_196 {
    use super::*;
    use crate::spans::SpansBuilder;

    #[test]
    fn test_rug() {
        let mut p0: SpansBuilder<T> = SpansBuilder::new();
        // Other operations on p0 can be performed here
        
        <spans::SpansBuilder<T>>::build(p0);
    }
}#[cfg(test)]
mod tests_rug_197 {
    use super::*;
    use crate::tree::Node;
    use crate::spans::{SpansInfo, Interval};
    use crate::delta::Transformer;

    #[test]
    fn test_rug() {
        type T = u32;
        let mut p0: Node<SpansInfo<T>> = Node::Internal { left: None, right: None, height: 0, len: 0 };
        let p1: usize = 10; // Sample value, replace with actual value
        let p2: usize = 20; // Sample value, replace with actual value
        let mut p3: Transformer<'_, N> = Transformer::new(); // replace N with the desired type

        <tree::Node<spans::SpansInfo<T>>>::transform(&p0, p1, p2, &mut p3);

    }
}#[cfg(test)]
mod tests_rug_198 {
    use super::*;
    use crate::tree::Node;
    use crate::spans::{SpansInfo, Interval};
    use core::const_closure::ConstFnMutClosure;

    type T = u32; // Replace T with the desired type

    #[test]
    fn test_merge() {
        let mut p0: Node<SpansInfo<T>> = Node::Internal { left: None, right: None, height: 0, len: 0 };
        let mut p1: Node<SpansInfo<T>> = Node::Internal { left: None, right: None, height: 0, len: 0 };
        let mut p2 = ConstFnMutClosure::<(&mut Node<SpansInfo<T>>, Option<&Node<SpansInfo<T>>>), T>::new(|val, other| {
            // implementation of the closure goes here
            // use `val` and `other` as references to manipulate and access the desired types
            // return a value of type T
        });

        spans::<Node<SpansInfo<T>>>::merge(&p0, &p1, &mut p2);
    }
}
#[cfg(test)]
mod tests_rug_199 {
    use super::*;
    use crate::tree::Node;
    use crate::spans::{SpansInfo, Interval};
    
    type T = u32; // Replace T with the desired type
    
    #[test]
    fn test_rug() {
        let mut v85: Node<SpansInfo<T>> = Node::Internal { left: None, right: None, height: 0, len: 0 };
        <tree::Node<spans::SpansInfo<T>>>::iter(&v85);
    }
}

#[cfg(test)]
mod tests_rug_200 {
    use super::*;
    use crate::tree::Node;
    use crate::spans::{SpansInfo, SpansBuilder, Interval};
    use crate::delta::{Delta, DeltaElement};
    
    type T = u32; // Replace T with the desired type
    
    #[test]
    fn test_rug() {
        let mut p0: Node<SpansInfo<T>> = Node::Internal { left: None, right: None, height: 0, len: 0 };
        
        let mut p1: Delta<Node<SpansInfo<T>>> = Delta::default();
        
        <tree::Node<spans::SpansInfo<T>>>::apply_shape(&mut p0, &p1);
    }
}
#[cfg(test)]
mod tests_rug_201 {
    use super::*;
    use crate::tree::Node;
    use crate::spans::{SpansInfo, SpansBuilder, Interval};

    #[test]
    fn test_rug() {
        type T = u32;

        let mut v85: Node<SpansInfo<T>> = Node::Internal { left: None, right: None, height: 0, len: 0 };
        let mut v25: Interval = Interval::new(2, 5);

        <tree::Node<spans::SpansInfo<T>>>::delete_after(&mut v85, v25);

        // Add assertions here
    }
}                        
#[cfg(test)]
mod tests_rug_202 {
    use super::*;
    use crate::spans::SpanIter;
    
    #[test]
    fn test_rug() {
        let mut p0: SpanIter<'_, T> = unimplemented!();
        
        SpanIter::next(&mut p0);
        
    }
}