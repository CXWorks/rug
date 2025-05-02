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

//! A module for representing a set of breaks, typically used for
//! storing the result of line breaking.

use crate::interval::Interval;
use crate::tree::{DefaultMetric, Leaf, Metric, Node, NodeInfo, TreeBuilder};
use std::cmp::min;
use std::mem;

/// A set of indexes. A motivating use is storing line breaks.
pub type Breaks = Node<BreaksInfo>;

const MIN_LEAF: usize = 32;
const MAX_LEAF: usize = 64;

// Here the base units are arbitrary, but most commonly match the base units
// of the rope storing the underlying string.

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BreaksLeaf {
    /// Length, in base units.
    len: usize,
    /// Indexes, represent as offsets from the start of the leaf.
    data: Vec<usize>,
}

/// The number of breaks.
#[derive(Clone, Debug)]
pub struct BreaksInfo(usize);

impl Leaf for BreaksLeaf {
    fn len(&self) -> usize {
        self.len
    }

    fn is_ok_child(&self) -> bool {
        self.data.len() >= MIN_LEAF
    }

    fn push_maybe_split(&mut self, other: &BreaksLeaf, iv: Interval) -> Option<BreaksLeaf> {
        //eprintln!("push_maybe_split {:?} {:?} {}", self, other, iv);
        let (start, end) = iv.start_end();
        for &v in &other.data {
            if start < v && v <= end {
                self.data.push(v - start + self.len);
            }
        }
        // the min with other.len() shouldn't be needed
        self.len += min(end, other.len()) - start;

        if self.data.len() <= MAX_LEAF {
            None
        } else {
            let splitpoint = self.data.len() / 2; // number of breaks
            let splitpoint_units = self.data[splitpoint - 1];

            let mut new = self.data.split_off(splitpoint);
            for x in &mut new {
                *x -= splitpoint_units;
            }

            let new_len = self.len - splitpoint_units;
            self.len = splitpoint_units;
            Some(BreaksLeaf { len: new_len, data: new })
        }
    }
}

impl NodeInfo for BreaksInfo {
    type L = BreaksLeaf;

    fn accumulate(&mut self, other: &Self) {
        self.0 += other.0;
    }

    fn compute_info(l: &BreaksLeaf) -> BreaksInfo {
        BreaksInfo(l.data.len())
    }
}

impl DefaultMetric for BreaksInfo {
    type DefaultMetric = BreaksBaseMetric;
}

impl BreaksLeaf {
    /// Exposed for testing.
    #[doc(hidden)]
    pub fn get_data_cloned(&self) -> Vec<usize> {
        self.data.clone()
    }
}

#[derive(Copy, Clone)]
pub struct BreaksMetric(());

impl Metric<BreaksInfo> for BreaksMetric {
    fn measure(info: &BreaksInfo, _: usize) -> usize {
        info.0
    }

    fn to_base_units(l: &BreaksLeaf, in_measured_units: usize) -> usize {
        if in_measured_units > l.data.len() {
            l.len + 1
        } else if in_measured_units == 0 {
            0
        } else {
            l.data[in_measured_units - 1]
        }
    }

    fn from_base_units(l: &BreaksLeaf, in_base_units: usize) -> usize {
        match l.data.binary_search(&in_base_units) {
            Ok(n) => n + 1,
            Err(n) => n,
        }
    }

    fn is_boundary(l: &BreaksLeaf, offset: usize) -> bool {
        l.data.binary_search(&offset).is_ok()
    }

    fn prev(l: &BreaksLeaf, offset: usize) -> Option<usize> {
        for i in 0..l.data.len() {
            if offset <= l.data[i] {
                if i == 0 {
                    return None;
                } else {
                    return Some(l.data[i - 1]);
                }
            }
        }
        l.data.last().cloned()
    }

    fn next(l: &BreaksLeaf, offset: usize) -> Option<usize> {
        let n = match l.data.binary_search(&offset) {
            Ok(n) => n + 1,
            Err(n) => n,
        };

        if n == l.data.len() {
            None
        } else {
            Some(l.data[n])
        }
    }

    fn can_fragment() -> bool {
        true
    }
}

#[derive(Copy, Clone)]
pub struct BreaksBaseMetric(());

impl Metric<BreaksInfo> for BreaksBaseMetric {
    fn measure(_: &BreaksInfo, len: usize) -> usize {
        len
    }

    fn to_base_units(_: &BreaksLeaf, in_measured_units: usize) -> usize {
        in_measured_units
    }

    fn from_base_units(_: &BreaksLeaf, in_base_units: usize) -> usize {
        in_base_units
    }

    fn is_boundary(l: &BreaksLeaf, offset: usize) -> bool {
        BreaksMetric::is_boundary(l, offset)
    }

    fn prev(l: &BreaksLeaf, offset: usize) -> Option<usize> {
        BreaksMetric::prev(l, offset)
    }

    fn next(l: &BreaksLeaf, offset: usize) -> Option<usize> {
        BreaksMetric::next(l, offset)
    }

    fn can_fragment() -> bool {
        true
    }
}

// Additional functions specific to breaks

impl Breaks {
    // a length with no break, useful in edit operations; for
    // other use cases, use the builder.
    pub fn new_no_break(len: usize) -> Breaks {
        let leaf = BreaksLeaf { len, data: vec![] };
        Node::from_leaf(leaf)
    }
}

pub struct BreakBuilder {
    b: TreeBuilder<BreaksInfo>,
    leaf: BreaksLeaf,
}

impl Default for BreakBuilder {
    fn default() -> BreakBuilder {
        BreakBuilder { b: TreeBuilder::new(), leaf: BreaksLeaf::default() }
    }
}

impl BreakBuilder {
    pub fn new() -> BreakBuilder {
        BreakBuilder::default()
    }

    pub fn add_break(&mut self, len: usize) {
        if self.leaf.data.len() == MAX_LEAF {
            let leaf = mem::take(&mut self.leaf);
            self.b.push(Node::from_leaf(leaf));
        }
        self.leaf.len += len;
        self.leaf.data.push(self.leaf.len);
    }

    pub fn add_no_break(&mut self, len: usize) {
        self.leaf.len += len;
    }

    pub fn build(mut self) -> Breaks {
        self.b.push(Node::from_leaf(self.leaf));
        self.b.build()
    }
}

#[cfg(test)]
mod tests {
    use crate::breaks::{BreakBuilder, BreaksInfo, BreaksLeaf, BreaksMetric};
    use crate::interval::Interval;
    use crate::tree::{Cursor, Node};

    fn gen(n: usize) -> Node<BreaksInfo> {
        let mut node = Node::default();
        let mut b = BreakBuilder::new();
        b.add_break(10);
        let testnode = b.build();
        if n == 1 {
            return testnode;
        }
        for _ in 0..n {
            let len = node.len();
            let empty_interval_at_end = Interval::new(len, len);
            node.edit(empty_interval_at_end, testnode.clone());
        }
        node
    }

    #[test]
    fn empty() {
        let n = gen(0);
        assert_eq!(0, n.len());
    }

    #[test]
    fn fromleaf() {
        let testnode = gen(1);
        assert_eq!(10, testnode.len());
    }

    #[test]
    fn one() {
        let testleaf = BreaksLeaf { len: 10, data: vec![10] };
        let testnode = Node::<BreaksInfo>::from_leaf(testleaf.clone());
        assert_eq!(10, testnode.len());
        let mut c = Cursor::new(&testnode, 0);
        assert_eq!(c.get_leaf().unwrap().0, &testleaf);
        assert_eq!(10, c.next::<BreaksMetric>().unwrap());
        assert!(c.next::<BreaksMetric>().is_none());
        c.set(0);
        assert!(!c.is_boundary::<BreaksMetric>());
        c.set(1);
        assert!(!c.is_boundary::<BreaksMetric>());
        c.set(10);
        assert!(c.is_boundary::<BreaksMetric>());
        assert!(c.prev::<BreaksMetric>().is_none());
    }

    #[test]
    fn concat() {
        let left = gen(1);
        let right = gen(1);
        let node = Node::concat(left.clone(), right);
        assert_eq!(node.len(), 20);
        let mut c = Cursor::new(&node, 0);
        assert_eq!(10, c.next::<BreaksMetric>().unwrap());
        assert_eq!(20, c.next::<BreaksMetric>().unwrap());
        assert!(c.next::<BreaksMetric>().is_none());
    }

    #[test]
    fn larger() {
        let node = gen(100);
        assert_eq!(node.len(), 1000);
    }

    #[test]
    fn default_metric_test() {
        use super::BreaksBaseMetric;

        let breaks = gen(10);
        assert_eq!(
            breaks.convert_metrics::<BreaksBaseMetric, BreaksMetric>(5),
            breaks.count::<BreaksMetric>(5)
        );
        assert_eq!(
            breaks.convert_metrics::<BreaksMetric, BreaksBaseMetric>(7),
            breaks.count_base_units::<BreaksMetric>(7)
        );
    }
}
#[cfg(test)]
mod tests_llm_16_9_llm_16_8 {
    use super::*;

use crate::*;
    use crate::breaks::{BreaksBaseMetric, BreaksMetric, BreaksLeaf};
    use crate::tree::Metric;
    
    #[test]
    fn test_is_boundary() {
        let leaf = BreaksLeaf { len: 10, data: vec![2, 5, 8] };

        assert_eq!(BreaksMetric::is_boundary(&leaf, 1), false);
        assert_eq!(BreaksMetric::is_boundary(&leaf, 2), true);
        assert_eq!(BreaksMetric::is_boundary(&leaf, 3), false);
        assert_eq!(BreaksMetric::is_boundary(&leaf, 5), true);
        assert_eq!(BreaksMetric::is_boundary(&leaf, 9), true);
        assert_eq!(BreaksMetric::is_boundary(&leaf, 10), false);
        assert_eq!(BreaksMetric::is_boundary(&leaf, 11), false);
    }
}#[cfg(test)]
mod tests_llm_16_29_llm_16_28 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_can_fragment() {
        assert_eq!(BreaksMetric::can_fragment(), true);
    }
}#[cfg(test)]
mod tests_llm_16_30 {
    use super::*;

use crate::*;
    use breaks::BreaksInfo;
    use tree::Metric;
    
    #[test]
    fn test_from_base_units() {
        // Create a BreaksLeaf with sample data
        let leaf = BreaksLeaf {
            len: 10,
            data: vec![0, 3, 7, 10],
        };

        // Test with in_base_units = 0
        let in_base_units = 0;
        let expected_output = 0;
        let result = BreaksMetric::from_base_units(&leaf, in_base_units);
        assert_eq!(result, expected_output);

        // Test with in_base_units = 3
        let in_base_units = 3;
        let expected_output = 1;
        let result = BreaksMetric::from_base_units(&leaf, in_base_units);
        assert_eq!(result, expected_output);

        // Test with in_base_units = 7
        let in_base_units = 7;
        let expected_output = 2;
        let result = BreaksMetric::from_base_units(&leaf, in_base_units);
        assert_eq!(result, expected_output);

        // Test with in_base_units = 10
        let in_base_units = 10;
        let expected_output = 3;
        let result = BreaksMetric::from_base_units(&leaf, in_base_units);
        assert_eq!(result, expected_output);

        // Test with in_base_units = 5
        let in_base_units = 5;
        let expected_output = 2;
        let result = BreaksMetric::from_base_units(&leaf, in_base_units);
        assert_eq!(result, expected_output);

        // Test with in_base_units = 8
        let in_base_units = 8;
        let expected_output = 3;
        let result = BreaksMetric::from_base_units(&leaf, in_base_units);
        assert_eq!(result, expected_output);
    }
}#[cfg(test)]
mod tests_llm_16_32 {
    use super::*;

use crate::*;
    use crate::breaks::{BreaksLeaf, BreaksMetric, BreaksInfo};

    #[test]
    fn test_is_boundary() {
        let l = BreaksLeaf {
            len: 10,
            data: vec![2, 5, 8]
        };
        assert_eq!(breaks::BreaksMetric::is_boundary(&l, 2), true);
        assert_eq!(breaks::BreaksMetric::is_boundary(&l, 5), true);
        assert_eq!(breaks::BreaksMetric::is_boundary(&l, 8), true);
        assert_eq!(breaks::BreaksMetric::is_boundary(&l, 1), false);
        assert_eq!(breaks::BreaksMetric::is_boundary(&l, 4), false);
        assert_eq!(breaks::BreaksMetric::is_boundary(&l, 9), false);
        assert_eq!(breaks::BreaksMetric::is_boundary(&l, 10), false);
        assert_eq!(breaks::BreaksMetric::is_boundary(&l, 11), false);
    }
}#[cfg(test)]
mod tests_llm_16_138 {
    use super::*;

use crate::*;
    use breaks::{BreakBuilder, Breaks};
    use tree::{Node, NodeInfo, Leaf, Metric};

    #[test]
    fn test_new() {
        let builder: BreakBuilder = BreakBuilder::new();
    }
}#[cfg(test)]
mod tests_rug_141 {
    use super::*;
    use crate::tree::Leaf;
    use rope::breaks::BreaksLeaf;
    
    #[test]
    fn test_rug() {
        let mut p0: BreaksLeaf = BreaksLeaf { len: 10, data: vec![1, 2, 3] };

        <breaks::BreaksLeaf as tree::Leaf>::len(&p0);
    }
}        
#[cfg(test)]
mod tests_rug_142 {
    use super::*;
    use crate::tree::Leaf;
    use breaks::BreaksLeaf;
    
    #[test]
    fn test_rug() {
        let mut p0: BreaksLeaf = BreaksLeaf { len: 10, data: vec![1, 2, 3] };

        <BreaksLeaf as tree::Leaf>::is_ok_child(&p0);
    }
}
        #[cfg(test)]
mod tests_rug_143 {
    use super::*;
    use crate::tree::Leaf;
    use breaks::BreaksLeaf;
    use crate::interval::Interval;
    
    #[test]
    fn test_rug() {
        let mut p0: BreaksLeaf = BreaksLeaf { len: 10, data: vec![1, 2, 3] };
        let p1: BreaksLeaf = BreaksLeaf { len: 10, data: vec![4, 5, 6] };
        let p2: Interval = Interval::new(2, 5);
        
        p0.push_maybe_split(&p1, p2);
    }
}#[cfg(test)]
mod tests_rug_144 {
    use super::*;
    use crate::tree::NodeInfo;
    use rope::breaks::{BreaksInfo, BreaksLeaf};

    #[test]
    fn test_rug() {
        let mut p0 = {
            // Construct the breaks::BreaksInfo variable
            let l = BreaksLeaf {
                data: vec![/* fill in sample data */],
            };
            BreaksInfo::compute_info(&l)
        };

        let mut p1 = {
            // Construct the breaks::BreaksInfo variable
            let l = BreaksLeaf {
                data: vec![/* fill in sample data */],
            };
            BreaksInfo::compute_info(&l)
        };

        <breaks::BreaksInfo as tree::NodeInfo>::accumulate(&mut p0, &p1);
    }
}#[cfg(test)]
mod tests_rug_145 {
    use super::*;
    use crate::tree::NodeInfo;
    
    #[test]
    fn test_rug() {
        #[derive(Debug)]
        struct BreaksLeaf {
            len: usize,
            data: Vec<u32>,
        }
        
        impl tree::NodeInfo for BreaksLeaf {
            type Info = BreaksInfo;
    
            fn compute_info(l: &BreaksLeaf) -> BreaksInfo {
                BreaksInfo(l.data.len())
            }
        }
        
        let mut p0: BreaksLeaf = BreaksLeaf { len: 10, data: vec![1, 2, 3] };

        <BreaksLeaf as tree::NodeInfo>::compute_info(&p0);
    }
}#[cfg(test)]
mod tests_rug_146 {
    use super::*;
    use rope::breaks::BreaksLeaf;

    #[test]
    fn test_rug() {
        let mut p0: BreaksLeaf = BreaksLeaf { len: 10, data: vec![1, 2, 3] };

        <breaks::BreaksLeaf>::get_data_cloned(&p0);
    }
}
#[cfg(test)]
mod tests_rug_147 {
   use super::*;
   use rope::breaks::{BreaksInfo, BreaksMetric, BreaksLeaf};
   use crate::Metric;

   #[test]
   fn test_rug() {
      let mut p0 = breaks::BreaksInfo(0);
      let p1: usize = 10;
      
      <BreaksMetric as Metric<BreaksInfo>>::measure(&p0, p1);
   }
}
#[cfg(test)]
mod tests_rug_148 {
    use super::*;
    use rope::breaks::{BreaksMetric, BreaksInfo, BreaksLeaf};
    use rope::tree::Metric;
    
    #[test]
    fn test_rug() {
        let mut p0: BreaksLeaf = BreaksLeaf { len: 10, data: vec![1, 2, 3] };
        let mut p1: usize = 2;

        <BreaksMetric as Metric<BreaksInfo>>::to_base_units(&p0, p1);
    }
}#[cfg(test)]
mod tests_rug_149 {
    use super::*;
    use crate::Metric;
    use rope::breaks::{BreaksLeaf, BreaksMetric, BreaksInfo};
    
    #[test]
    fn test_rug() {
        let mut p0: BreaksLeaf = BreaksLeaf { len: 10, data: vec![1, 2, 3] };
        let p1: usize = 5;

        <BreaksMetric as Metric<BreaksInfo>>::prev(&p0, p1);
    }
}
#[cfg(test)]
mod tests_rug_150 {
    use super::*;
    use crate::Metric;
    use rope::breaks::{BreaksMetric, BreaksLeaf};
    
    #[test]
    fn test_rug() {
        let mut p0: BreaksLeaf = BreaksLeaf { len: 10, data: vec![1, 2, 3] };
        let p1: usize = 5;
        <BreaksMetric as Metric<BreaksLeaf>>::next(&p0, p1);
    }
}#[cfg(test)]
mod tests_rug_152 {
    use super::*;
    use crate::Metric;
    use rope::breaks::{BreaksBaseMetric, BreaksInfo, BreaksLeaf};

    #[test]
    fn test_rug() {
        let mut p0: BreaksLeaf = BreaksLeaf { len: 10, data: vec![1, 2, 3] };
        let p1: usize = 100;

        <BreaksBaseMetric as Metric<BreaksInfo>>::to_base_units(&p0, p1);
    }
}#[cfg(test)]
mod tests_rug_153 {
    use super::*;
    use crate::Metric;

    #[test]
    fn test_rug() {
        let mut p0: breaks::BreaksLeaf = breaks::BreaksLeaf { len: 10, data: vec![1, 2, 3] };
        let p1: usize = 5;

        <breaks::BreaksBaseMetric as tree::Metric<breaks::BreaksInfo>>::from_base_units(&p0, p1);
    }
}                        
#[cfg(test)]
mod tests_rug_154 {
    use super::*;
    use crate::Metric;
    use crate::breaks::{BreaksLeaf, BreaksBaseMetric};
    
    #[test]
    fn test_prev() {
        let mut p0: BreaksLeaf = BreaksLeaf { len: 10, data: vec![1, 2, 3] };
        let p1: usize = 5;

        let result = BreaksMetric::prev(&p0, p1);
        assert_eq!(result, ...);
    }
}
#[cfg(test)]
mod tests_rug_155 {
    use super::*;
    use crate::Metric;
    use rope::breaks::{BreaksLeaf, BreaksMetric, BreaksBaseMetric};

    #[test]
    fn test_rug() {
        // Prepare variables
        let mut v50: BreaksLeaf = BreaksLeaf { len: 10, data: vec![1, 2, 3] };
        let p0: &BreaksLeaf = &v50;
        let p1: usize = 5;

        // Call the target function
        <BreaksBaseMetric as Metric<BreaksInfo>>::next(p0, p1);

    }
}                          

#[cfg(test)]
mod tests_rug_156 {
    use super::*;
    use tree::Metric;
    use breaks::{BreaksBaseMetric, BreaksInfo};

    #[test]
    fn test_rug() {
        <BreaksBaseMetric as Metric<BreaksInfo>>::can_fragment();
    }
}
#[cfg(test)]
mod tests_rug_157 {
    use super::*;
    use rope::breaks::{Breaks, breaks::BreaksInfo};

    #[test]
    fn test_rug() {
        let p0: usize = 10;

        breaks::<tree::Node<BreaksInfo>>::new_no_break(p0);

    }
}#[cfg(test)]
mod tests_rug_158 {
    use super::*;
    use breaks::BreakBuilder;
    use breaks::BreaksLeaf;
    use breaks::TreeBuilder;
    #[test]
    fn test_rug() {
        <BreakBuilder as std::default::Default>::default();
    }
}#[cfg(test)]
mod tests_rug_159 {
    use super::*;
    use breaks::{BreakBuilder, Node};
    use std::mem;

    #[test]
    fn test_rug() {
        #[cfg(test)]
        mod tests_rug_159_prepare {
            #[test]
            fn sample() {
                let mut v72 = BreakBuilder::new();
            }
        }

        let mut p0 = BreakBuilder::new();
        let mut p1 = 10;

        BreakBuilder::add_break(&mut p0, p1);

    }
}#[cfg(test)]
mod tests_rug_160 {
    use super::*;
    use breaks::BreakBuilder;

    #[test]
    fn test_rug() {
        let mut p0 = BreakBuilder::new();
        let p1: usize = 10;

        BreakBuilder::add_no_break(&mut p0, p1);
    }
}
#[cfg(test)]
mod tests_rug_161 {
    use super::*;
    use crate::rope::breaks;
    
    #[test]
    fn test_rug() {
        let mut p0 = breaks::BreakBuilder::new();
        
        breaks::BreakBuilder::build(p0);
    }
}
