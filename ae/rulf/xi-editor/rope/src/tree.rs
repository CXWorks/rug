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

//! A general b-tree structure suitable for ropes and the like.

use std::cmp::{min, Ordering};
use std::marker::PhantomData;
use std::sync::Arc;

use crate::interval::{Interval, IntervalBounds};

const MIN_CHILDREN: usize = 4;
const MAX_CHILDREN: usize = 8;

pub trait NodeInfo: Clone {
    /// The type of the leaf.
    ///
    /// A given `NodeInfo` is for exactly one type of leaf. That is why
    /// the leaf type is an associated type rather than a type parameter.
    type L: Leaf;

    /// An operator that combines info from two subtrees. It is intended
    /// (but not strictly enforced) that this operator be associative and
    /// obey an identity property. In mathematical terms, the accumulate
    /// method is the operation of a monoid.
    fn accumulate(&mut self, other: &Self);

    /// A mapping from a leaf into the info type. It is intended (but
    /// not strictly enforced) that applying the accumulate method to
    /// the info derived from two leaves gives the same result as
    /// deriving the info from the concatenation of the two leaves. In
    /// mathematical terms, the compute_info method is a monoid
    /// homomorphism.
    fn compute_info(_: &Self::L) -> Self;

    /// The identity of the monoid. Need not be implemented because it
    /// can be computed from the leaf default.
    ///
    /// This is here to demonstrate that this is a monoid.
    fn identity() -> Self {
        Self::compute_info(&Self::L::default())
    }

    /// The interval covered by the first `len` base units of this node. The
    /// default impl is sufficient for most types, but interval trees may need
    /// to override it.
    fn interval(&self, len: usize) -> Interval {
        Interval::new(0, len)
    }
}

/// A trait indicating the default metric of a NodeInfo.
///
/// Adds quality of life functions to
/// Node\<N\>, where N is a DefaultMetric.
/// For example, [Node\<DefaultMetric\>.count](struct.Node.html#method.count).
pub trait DefaultMetric: NodeInfo {
    type DefaultMetric: Metric<Self>;
}

/// A trait for the leaves of trees of type [Node](struct.Node.html).
///
/// Two leafs can be concatenated using `push_maybe_split`.
pub trait Leaf: Sized + Clone + Default {
    /// Measurement of leaf in base units.
    /// A 'base unit' refers to the smallest discrete unit
    /// by which a given concrete type can be indexed.
    /// Concretely, for Rust's String type the base unit is the byte.
    fn len(&self) -> usize;

    /// Generally a minimum size requirement for leaves.
    fn is_ok_child(&self) -> bool;

    /// Combine the part `other` denoted by the `Interval` `iv` into `self`,
    /// optionly splitting off a new `Leaf` if `self` would have become too big.
    /// Returns either `None` if no splitting was needed, or `Some(rest)` if
    /// `rest` was split off.
    ///
    /// Interval is in "base units".  Generally implements a maximum size.
    ///
    /// # Invariants:
    /// - If one or the other input is empty, then no split.
    /// - If either input satisfies `is_ok_child`, then, on return, `self`
    ///   satisfies this, as does the optional split.
    fn push_maybe_split(&mut self, other: &Self, iv: Interval) -> Option<Self>;

    /// Same meaning as push_maybe_split starting from an empty
    /// leaf, but maybe can be implemented more efficiently?
    ///
    // TODO: remove if it doesn't pull its weight
    fn subseq(&self, iv: Interval) -> Self {
        let mut result = Self::default();
        if result.push_maybe_split(self, iv).is_some() {
            panic!("unexpected split");
        }
        result
    }
}

/// A b-tree node storing leaves at the bottom, and with info
/// retained at each node. It is implemented with atomic reference counting
/// and copy-on-write semantics, so an immutable clone is a very cheap
/// operation, and nodes can be shared across threads. Even so, it is
/// designed to be updated in place, with efficiency similar to a mutable
/// data structure, using uniqueness of reference count to detect when
/// this operation is safe.
///
/// When the leaf is a string, this is a rope data structure (a persistent
/// rope in functional programming jargon). However, it is not restricted
/// to strings, and it is expected to be the basis for a number of data
/// structures useful for text processing.
#[derive(Clone)]
pub struct Node<N: NodeInfo>(Arc<NodeBody<N>>);

#[derive(Clone)]
struct NodeBody<N: NodeInfo> {
    height: usize,
    len: usize,
    info: N,
    val: NodeVal<N>,
}

#[derive(Clone)]
enum NodeVal<N: NodeInfo> {
    Leaf(N::L),
    Internal(Vec<Node<N>>),
}

// also consider making Metric a newtype for usize, so type system can
// help separate metrics

/// A trait for quickly processing attributes of a
/// [NodeInfo](struct.NodeInfo.html).
///
/// For the conceptual background see the
/// [blog post, Rope science, part 2: metrics](https://github.com/google/xi-editor/blob/master/docs/docs/rope_science_02.md).
pub trait Metric<N: NodeInfo> {
    /// Return the size of the
    /// [NodeInfo::L](trait.NodeInfo.html#associatedtype.L), as measured by this
    /// metric.
    ///
    /// The usize argument is the total size/length of the node, in base units.
    ///
    /// # Examples
    /// For the [LinesMetric](../rope/struct.LinesMetric.html), this gives the number of
    /// lines in string contained in the leaf. For the
    /// [BaseMetric](../rope/struct.BaseMetric.html), this gives the size of the string
    /// in uft8 code units, that is, bytes.
    ///
    fn measure(info: &N, len: usize) -> usize;

    /// Returns the smallest offset, in base units, for an offset in measured units.
    ///
    /// # Invariants:
    ///
    /// - `from_base_units(to_base_units(x)) == x` is True for valid `x`
    fn to_base_units(l: &N::L, in_measured_units: usize) -> usize;

    /// Returns the smallest offset in measured units corresponding to an offset in base units.
    ///
    /// # Invariants:
    ///
    /// - `from_base_units(to_base_units(x)) == x` is True for valid `x`
    fn from_base_units(l: &N::L, in_base_units: usize) -> usize;

    /// Return whether the offset in base units is a boundary of this metric.
    /// If a boundary is at end of a leaf then this method must return true.
    /// However, a boundary at the beginning of a leaf is optional
    /// (the previous leaf will be queried).
    fn is_boundary(l: &N::L, offset: usize) -> bool;

    /// Returns the index of the boundary directly preceding offset,
    /// or None if no such boundary exists. Input and result are in base units.
    fn prev(l: &N::L, offset: usize) -> Option<usize>;

    /// Returns the index of the first boundary for which index > offset,
    /// or None if no such boundary exists. Input and result are in base units.
    fn next(l: &N::L, offset: usize) -> Option<usize>;

    /// Returns true if the measured units in this metric can span multiple
    /// leaves.  As an example, in a metric that measures lines in a rope, a
    /// line may start in one leaf and end in another; however in a metric
    /// measuring bytes, storage of a single byte cannot extend across leaves.
    fn can_fragment() -> bool;
}

impl<N: NodeInfo> Node<N> {
    pub fn from_leaf(l: N::L) -> Node<N> {
        let len = l.len();
        let info = N::compute_info(&l);
        Node(Arc::new(NodeBody { height: 0, len, info, val: NodeVal::Leaf(l) }))
    }

    /// Create a node from a vec of nodes.
    ///
    /// The input must satisfy the following balancing requirements:
    /// * The length of `nodes` must be <= MAX_CHILDREN and > 1.
    /// * All the nodes are the same height.
    /// * All the nodes must satisfy is_ok_child.
    fn from_nodes(nodes: Vec<Node<N>>) -> Node<N> {
        debug_assert!(nodes.len() > 1);
        debug_assert!(nodes.len() <= MAX_CHILDREN);
        let height = nodes[0].0.height + 1;
        let mut len = nodes[0].0.len;
        let mut info = nodes[0].0.info.clone();
        debug_assert!(nodes[0].is_ok_child());
        for child in &nodes[1..] {
            debug_assert_eq!(child.height() + 1, height);
            debug_assert!(child.is_ok_child());
            len += child.0.len;
            info.accumulate(&child.0.info);
        }
        Node(Arc::new(NodeBody { height, len, info, val: NodeVal::Internal(nodes) }))
    }

    pub fn len(&self) -> usize {
        self.0.len
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns `true` if these two `Node`s share the same underlying data.
    ///
    /// This is principally intended to be used by the druid crate, without needing
    /// to actually add a feature and implement druid's `Data` trait.
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }

    fn height(&self) -> usize {
        self.0.height
    }

    fn is_leaf(&self) -> bool {
        self.0.height == 0
    }

    fn interval(&self) -> Interval {
        self.0.info.interval(self.0.len)
    }

    fn get_children(&self) -> &[Node<N>] {
        if let NodeVal::Internal(ref v) = self.0.val {
            v
        } else {
            panic!("get_children called on leaf node");
        }
    }

    fn get_leaf(&self) -> &N::L {
        if let NodeVal::Leaf(ref l) = self.0.val {
            l
        } else {
            panic!("get_leaf called on internal node");
        }
    }

    /// Call a callback with a mutable reference to a leaf.
    ///
    /// This clones the leaf if the reference is shared. It also recomputes
    /// length and info after the leaf is mutated.
    fn with_leaf_mut<T>(&mut self, f: impl FnOnce(&mut N::L) -> T) -> T {
        let inner = Arc::make_mut(&mut self.0);
        if let NodeVal::Leaf(ref mut l) = inner.val {
            let result = f(l);
            inner.len = l.len();
            inner.info = N::compute_info(l);
            result
        } else {
            panic!("with_leaf_mut called on internal node");
        }
    }

    fn is_ok_child(&self) -> bool {
        match self.0.val {
            NodeVal::Leaf(ref l) => l.is_ok_child(),
            NodeVal::Internal(ref nodes) => (nodes.len() >= MIN_CHILDREN),
        }
    }

    fn merge_nodes(children1: &[Node<N>], children2: &[Node<N>]) -> Node<N> {
        let n_children = children1.len() + children2.len();
        if n_children <= MAX_CHILDREN {
            Node::from_nodes([children1, children2].concat())
        } else {
            // Note: this leans left. Splitting at midpoint is also an option
            let splitpoint = min(MAX_CHILDREN, n_children - MIN_CHILDREN);
            let mut iter = children1.iter().chain(children2.iter()).cloned();
            let left = iter.by_ref().take(splitpoint).collect();
            let right = iter.collect();
            let parent_nodes = vec![Node::from_nodes(left), Node::from_nodes(right)];
            Node::from_nodes(parent_nodes)
        }
    }

    fn merge_leaves(mut rope1: Node<N>, rope2: Node<N>) -> Node<N> {
        debug_assert!(rope1.is_leaf() && rope2.is_leaf());

        let both_ok = rope1.get_leaf().is_ok_child() && rope2.get_leaf().is_ok_child();
        if both_ok {
            return Node::from_nodes(vec![rope1, rope2]);
        }
        match {
            let node1 = Arc::make_mut(&mut rope1.0);
            let leaf2 = rope2.get_leaf();
            if let NodeVal::Leaf(ref mut leaf1) = node1.val {
                let leaf2_iv = Interval::new(0, leaf2.len());
                let new = leaf1.push_maybe_split(leaf2, leaf2_iv);
                node1.len = leaf1.len();
                node1.info = N::compute_info(leaf1);
                new
            } else {
                panic!("merge_leaves called on non-leaf");
            }
        } {
            Some(new) => Node::from_nodes(vec![rope1, Node::from_leaf(new)]),
            None => rope1,
        }
    }

    pub fn concat(rope1: Node<N>, rope2: Node<N>) -> Node<N> {
        let h1 = rope1.height();
        let h2 = rope2.height();

        match h1.cmp(&h2) {
            Ordering::Less => {
                let children2 = rope2.get_children();
                if h1 == h2 - 1 && rope1.is_ok_child() {
                    return Node::merge_nodes(&[rope1], children2);
                }
                let newrope = Node::concat(rope1, children2[0].clone());
                if newrope.height() == h2 - 1 {
                    Node::merge_nodes(&[newrope], &children2[1..])
                } else {
                    Node::merge_nodes(newrope.get_children(), &children2[1..])
                }
            }
            Ordering::Equal => {
                if rope1.is_ok_child() && rope2.is_ok_child() {
                    return Node::from_nodes(vec![rope1, rope2]);
                }
                if h1 == 0 {
                    return Node::merge_leaves(rope1, rope2);
                }
                Node::merge_nodes(rope1.get_children(), rope2.get_children())
            }
            Ordering::Greater => {
                let children1 = rope1.get_children();
                if h2 == h1 - 1 && rope2.is_ok_child() {
                    return Node::merge_nodes(children1, &[rope2]);
                }
                let lastix = children1.len() - 1;
                let newrope = Node::concat(children1[lastix].clone(), rope2);
                if newrope.height() == h1 - 1 {
                    Node::merge_nodes(&children1[..lastix], &[newrope])
                } else {
                    Node::merge_nodes(&children1[..lastix], newrope.get_children())
                }
            }
        }
    }

    pub fn measure<M: Metric<N>>(&self) -> usize {
        M::measure(&self.0.info, self.0.len)
    }

    pub(crate) fn push_subseq(&self, b: &mut TreeBuilder<N>, iv: Interval) {
        if iv.is_empty() {
            return;
        }
        if iv == self.interval() {
            b.push(self.clone());
            return;
        }
        match self.0.val {
            NodeVal::Leaf(ref l) => {
                b.push_leaf_slice(l, iv);
            }
            NodeVal::Internal(ref v) => {
                let mut offset = 0;
                for child in v {
                    if iv.is_before(offset) {
                        break;
                    }
                    let child_iv = child.interval();
                    // easier just to use signed ints?
                    let rec_iv = iv.intersect(child_iv.translate(offset)).translate_neg(offset);
                    child.push_subseq(b, rec_iv);
                    offset += child.len();
                }
            }
        }
    }

    pub fn subseq<T: IntervalBounds>(&self, iv: T) -> Node<N> {
        let iv = iv.into_interval(self.len());
        let mut b = TreeBuilder::new();
        self.push_subseq(&mut b, iv);
        b.build()
    }

    pub fn edit<T, IV>(&mut self, iv: IV, new: T)
    where
        T: Into<Node<N>>,
        IV: IntervalBounds,
    {
        let mut b = TreeBuilder::new();
        let iv = iv.into_interval(self.len());
        let self_iv = self.interval();
        self.push_subseq(&mut b, self_iv.prefix(iv));
        b.push(new.into());
        self.push_subseq(&mut b, self_iv.suffix(iv));
        *self = b.build();
    }

    // doesn't deal with endpoint, handle that specially if you need it
    pub fn convert_metrics<M1: Metric<N>, M2: Metric<N>>(&self, mut m1: usize) -> usize {
        if m1 == 0 {
            return 0;
        }
        // If M1 can fragment, then we must land on the leaf containing
        // the m1 boundary. Otherwise, we can land on the beginning of
        // the leaf immediately following the M1 boundary, which may be
        // more efficient.
        let m1_fudge = if M1::can_fragment() { 1 } else { 0 };
        let mut m2 = 0;
        let mut node = self;
        while node.height() > 0 {
            for child in node.get_children() {
                let child_m1 = child.measure::<M1>();
                if m1 < child_m1 + m1_fudge {
                    node = child;
                    break;
                }
                m2 += child.measure::<M2>();
                m1 -= child_m1;
            }
        }
        let l = node.get_leaf();
        let base = M1::to_base_units(l, m1);
        m2 + M2::from_base_units(l, base)
    }
}

impl<N: DefaultMetric> Node<N> {
    /// Measures the length of the text bounded by ``DefaultMetric::measure(offset)`` with another metric.
    ///
    /// # Examples
    /// ```
    /// use crate::xi_rope::{Rope, LinesMetric};
    ///
    /// // the default metric of Rope is BaseMetric (aka number of bytes)
    /// let my_rope = Rope::from("first line \n second line \n");
    ///
    /// // count the number of lines in my_rope
    /// let num_lines = my_rope.count::<LinesMetric>(my_rope.len());
    /// assert_eq!(2, num_lines);
    /// ```
    pub fn count<M: Metric<N>>(&self, offset: usize) -> usize {
        self.convert_metrics::<N::DefaultMetric, M>(offset)
    }

    /// Measures the length of the text bounded by ``M::measure(offset)`` with the default metric.
    ///
    /// # Examples
    /// ```
    /// use crate::xi_rope::{Rope, LinesMetric};
    ///
    /// // the default metric of Rope is BaseMetric (aka number of bytes)
    /// let my_rope = Rope::from("first line \n second line \n");
    ///
    /// // get the byte offset of the line at index 1
    /// let byte_offset = my_rope.count_base_units::<LinesMetric>(1);
    /// assert_eq!(12, byte_offset);
    /// ```
    pub fn count_base_units<M: Metric<N>>(&self, offset: usize) -> usize {
        self.convert_metrics::<M, N::DefaultMetric>(offset)
    }
}

impl<N: NodeInfo> Default for Node<N> {
    fn default() -> Node<N> {
        Node::from_leaf(N::L::default())
    }
}

/// A builder for creating new trees.
pub struct TreeBuilder<N: NodeInfo> {
    // A stack of partially built trees. These are kept in order of
    // strictly descending height, and all vectors have a length less
    // than MAX_CHILDREN and greater than zero.
    //
    // In addition, there is a balancing invariant: for each vector
    // of length greater than one, all elements satisfy `is_ok_child`.
    stack: Vec<Vec<Node<N>>>,
}

impl<N: NodeInfo> TreeBuilder<N> {
    /// A new, empty builder.
    pub fn new() -> TreeBuilder<N> {
        TreeBuilder { stack: Vec::new() }
    }

    /// Append a node to the tree being built.
    pub fn push(&mut self, mut n: Node<N>) {
        loop {
            let ord = if let Some(last) = self.stack.last() {
                last[0].height().cmp(&n.height())
            } else {
                Ordering::Greater
            };
            match ord {
                Ordering::Less => {
                    n = Node::concat(self.pop(), n);
                }
                Ordering::Equal => {
                    let tos = self.stack.last_mut().unwrap();
                    if tos.last().unwrap().is_ok_child() && n.is_ok_child() {
                        tos.push(n);
                    } else if n.height() == 0 {
                        let iv = Interval::new(0, n.len());
                        let new_leaf = tos
                            .last_mut()
                            .unwrap()
                            .with_leaf_mut(|l| l.push_maybe_split(n.get_leaf(), iv));
                        if let Some(new_leaf) = new_leaf {
                            tos.push(Node::from_leaf(new_leaf));
                        }
                    } else {
                        let last = tos.pop().unwrap();
                        let children1 = last.get_children();
                        let children2 = n.get_children();
                        let n_children = children1.len() + children2.len();
                        if n_children <= MAX_CHILDREN {
                            tos.push(Node::from_nodes([children1, children2].concat()));
                        } else {
                            // Note: this leans left. Splitting at midpoint is also an option
                            let splitpoint = min(MAX_CHILDREN, n_children - MIN_CHILDREN);
                            let mut iter = children1.iter().chain(children2.iter()).cloned();
                            let left = iter.by_ref().take(splitpoint).collect();
                            let right = iter.collect();
                            tos.push(Node::from_nodes(left));
                            tos.push(Node::from_nodes(right));
                        }
                    }
                    if tos.len() < MAX_CHILDREN {
                        break;
                    }
                    n = self.pop()
                }
                Ordering::Greater => {
                    self.stack.push(vec![n]);
                    break;
                }
            }
        }
    }

    /// Append a sequence of leaves.
    pub fn push_leaves(&mut self, leaves: impl IntoIterator<Item = N::L>) {
        for leaf in leaves.into_iter() {
            self.push(Node::from_leaf(leaf));
        }
    }

    /// Append a single leaf.
    pub fn push_leaf(&mut self, l: N::L) {
        self.push(Node::from_leaf(l))
    }

    /// Append a slice of a single leaf.
    pub fn push_leaf_slice(&mut self, l: &N::L, iv: Interval) {
        self.push(Node::from_leaf(l.subseq(iv)))
    }

    /// Build the final tree.
    ///
    /// The tree is the concatenation of all the nodes and leaves that have been pushed
    /// on the builder, in order.
    pub fn build(mut self) -> Node<N> {
        if self.stack.is_empty() {
            Node::from_leaf(N::L::default())
        } else {
            let mut n = self.pop();
            while !self.stack.is_empty() {
                n = Node::concat(self.pop(), n);
            }
            n
        }
    }

    /// Pop the last vec-of-nodes off the stack, resulting in a node.
    fn pop(&mut self) -> Node<N> {
        let nodes = self.stack.pop().unwrap();
        if nodes.len() == 1 {
            nodes.into_iter().next().unwrap()
        } else {
            Node::from_nodes(nodes)
        }
    }
}

const CURSOR_CACHE_SIZE: usize = 4;

/// A data structure for traversing boundaries in a tree.
///
/// It is designed to be efficient both for random access and for iteration. The
/// cursor itself is agnostic to which [`Metric`] is used to determine boundaries, but
/// the methods to find boundaries are parametrized on the [`Metric`].
///
/// A cursor can be valid or invalid. It is always valid when created or after
/// [`set`](#method.set) is called, and becomes invalid after [`prev`](#method.prev)
/// or [`next`](#method.next) fails to find a boundary.
///
/// [`Metric`]: struct.Metric.html
pub struct Cursor<'a, N: 'a + NodeInfo> {
    /// The tree being traversed by this cursor.
    root: &'a Node<N>,
    /// The current position of the cursor.
    ///
    /// It is always less than or equal to the tree length.
    position: usize,
    /// The cache holds the tail of the path from the root to the current leaf.
    ///
    /// Each entry is a reference to the parent node and the index of the child. It
    /// is stored bottom-up; `cache[0]` is the parent of the leaf and the index of
    /// the leaf within that parent.
    ///
    /// The main motivation for this being a fixed-size array is to keep the cursor
    /// an allocation-free data structure.
    cache: [Option<(&'a Node<N>, usize)>; CURSOR_CACHE_SIZE],
    /// The leaf containing the current position, when the cursor is valid.
    ///
    /// The position is only at the end of the leaf when it is at the end of the tree.
    leaf: Option<&'a N::L>,
    /// The offset of `leaf` within the tree.
    offset_of_leaf: usize,
}

impl<'a, N: NodeInfo> Cursor<'a, N> {
    /// Create a new cursor at the given position.
    pub fn new(n: &'a Node<N>, position: usize) -> Cursor<'a, N> {
        let mut result = Cursor {
            root: n,
            position,
            cache: [None; CURSOR_CACHE_SIZE],
            leaf: None,
            offset_of_leaf: 0,
        };
        result.descend();
        result
    }

    /// The length of the tree.
    pub fn total_len(&self) -> usize {
        self.root.len()
    }

    /// Return a reference to the root node of the tree.
    pub fn root(&self) -> &'a Node<N> {
        self.root
    }

    /// Get the current leaf of the cursor.
    ///
    /// If the cursor is valid, returns the leaf containing the current position,
    /// and the offset of the current position within the leaf. That offset is equal
    /// to the leaf length only at the end, otherwise it is less than the leaf length.
    pub fn get_leaf(&self) -> Option<(&'a N::L, usize)> {
        self.leaf.map(|l| (l, self.position - self.offset_of_leaf))
    }

    /// Set the position of the cursor.
    ///
    /// The cursor is valid after this call.
    ///
    /// Precondition: `position` is less than or equal to the length of the tree.
    pub fn set(&mut self, position: usize) {
        self.position = position;
        if let Some(l) = self.leaf {
            if self.position >= self.offset_of_leaf && self.position < self.offset_of_leaf + l.len()
            {
                return;
            }
        }
        // TODO: walk up tree to find leaf if nearby
        self.descend();
    }

    /// Get the position of the cursor.
    pub fn pos(&self) -> usize {
        self.position
    }

    /// Determine whether the current position is a boundary.
    ///
    /// Note: the beginning and end of the tree may or may not be boundaries, depending on the
    /// metric. If the metric is not `can_fragment`, then they always are.
    pub fn is_boundary<M: Metric<N>>(&mut self) -> bool {
        if self.leaf.is_none() {
            // not at a valid position
            return false;
        }
        if self.position == self.offset_of_leaf && !M::can_fragment() {
            return true;
        }
        if self.position == 0 || self.position > self.offset_of_leaf {
            return M::is_boundary(self.leaf.unwrap(), self.position - self.offset_of_leaf);
        }
        // tricky case, at beginning of leaf, need to query end of previous
        // leaf; TODO: would be nice if we could do it another way that didn't
        // make the method &mut self.
        let l = self.prev_leaf().unwrap().0;
        let result = M::is_boundary(l, l.len());
        let _ = self.next_leaf();
        result
    }

    /// Moves the cursor to the previous boundary.
    ///
    /// When there is no previous boundary, returns `None` and the cursor becomes invalid.
    ///
    /// Return value: the position of the boundary, if it exists.
    pub fn prev<M: Metric<N>>(&mut self) -> Option<usize> {
        if self.position == 0 || self.leaf.is_none() {
            self.leaf = None;
            return None;
        }
        let orig_pos = self.position;
        let offset_in_leaf = orig_pos - self.offset_of_leaf;
        if offset_in_leaf > 0 {
            let l = self.leaf.unwrap();
            if let Some(offset_in_leaf) = M::prev(l, offset_in_leaf) {
                self.position = self.offset_of_leaf + offset_in_leaf;
                return Some(self.position);
            }
        }

        // not in same leaf, need to scan backwards
        self.prev_leaf()?;
        if let Some(offset) = self.last_inside_leaf::<M>(orig_pos) {
            return Some(offset);
        }

        // Not found in previous leaf, find using measurement.
        let measure = self.measure_leaf::<M>(self.position);
        if measure == 0 {
            self.leaf = None;
            self.position = 0;
            return None;
        }
        self.descend_metric::<M>(measure);
        self.last_inside_leaf::<M>(orig_pos)
    }

    /// Moves the cursor to the next boundary.
    ///
    /// When there is no next boundary, returns `None` and the cursor becomes invalid.
    ///
    /// Return value: the position of the boundary, if it exists.
    pub fn next<M: Metric<N>>(&mut self) -> Option<usize> {
        if self.position >= self.root.len() || self.leaf.is_none() {
            self.leaf = None;
            return None;
        }

        if let Some(offset) = self.next_inside_leaf::<M>() {
            return Some(offset);
        }

        self.next_leaf()?;
        if let Some(offset) = self.next_inside_leaf::<M>() {
            return Some(offset);
        }

        // Leaf is 0-measure (otherwise would have already succeeded).
        let measure = self.measure_leaf::<M>(self.position);
        self.descend_metric::<M>(measure + 1);
        if let Some(offset) = self.next_inside_leaf::<M>() {
            return Some(offset);
        }

        // Not found, properly invalidate cursor.
        self.position = self.root.len();
        self.leaf = None;
        None
    }

    /// Returns the current position if it is a boundary in this [`Metric`],
    /// else behaves like [`next`](#method.next).
    ///
    /// [`Metric`]: struct.Metric.html
    pub fn at_or_next<M: Metric<N>>(&mut self) -> Option<usize> {
        if self.is_boundary::<M>() {
            Some(self.pos())
        } else {
            self.next::<M>()
        }
    }

    /// Returns the current position if it is a boundary in this [`Metric`],
    /// else behaves like [`prev`](#method.prev).
    ///
    /// [`Metric`]: struct.Metric.html
    pub fn at_or_prev<M: Metric<N>>(&mut self) -> Option<usize> {
        if self.is_boundary::<M>() {
            Some(self.pos())
        } else {
            self.prev::<M>()
        }
    }

    /// Returns an iterator with this cursor over the given [`Metric`].
    ///
    /// # Examples:
    ///
    /// ```
    /// # use xi_rope::{Cursor, LinesMetric, Rope};
    /// #
    /// let text: Rope = "one line\ntwo line\nred line\nblue".into();
    /// let mut cursor = Cursor::new(&text, 0);
    /// let line_offsets = cursor.iter::<LinesMetric>().collect::<Vec<_>>();
    /// assert_eq!(line_offsets, vec![9, 18, 27]);
    ///
    /// ```
    /// [`Metric`]: struct.Metric.html
    pub fn iter<'c, M: Metric<N>>(&'c mut self) -> CursorIter<'c, 'a, N, M> {
        CursorIter { cursor: self, _metric: PhantomData }
    }

    /// Tries to find the last boundary in the leaf the cursor is currently in.
    ///
    /// If the last boundary is at the end of the leaf, it is only counted if
    /// it is less than `orig_pos`.
    #[inline]
    fn last_inside_leaf<M: Metric<N>>(&mut self, orig_pos: usize) -> Option<usize> {
        let l = self.leaf.expect("inconsistent, shouldn't get here");
        let len = l.len();
        if self.offset_of_leaf + len < orig_pos && M::is_boundary(l, len) {
            let _ = self.next_leaf();
            return Some(self.position);
        }
        let offset_in_leaf = M::prev(l, len)?;
        self.position = self.offset_of_leaf + offset_in_leaf;
        Some(self.position)
    }

    /// Tries to find the next boundary in the leaf the cursor is currently in.
    #[inline]
    fn next_inside_leaf<M: Metric<N>>(&mut self) -> Option<usize> {
        let l = self.leaf.expect("inconsistent, shouldn't get here");
        let offset_in_leaf = self.position - self.offset_of_leaf;
        let offset_in_leaf = M::next(l, offset_in_leaf)?;
        if offset_in_leaf == l.len() && self.offset_of_leaf + offset_in_leaf != self.root.len() {
            let _ = self.next_leaf();
        } else {
            self.position = self.offset_of_leaf + offset_in_leaf;
        }
        Some(self.position)
    }

    /// Move to beginning of next leaf.
    ///
    /// Return value: same as [`get_leaf`](#method.get_leaf).
    pub fn next_leaf(&mut self) -> Option<(&'a N::L, usize)> {
        let leaf = self.leaf?;
        self.position = self.offset_of_leaf + leaf.len();
        for i in 0..CURSOR_CACHE_SIZE {
            if self.cache[i].is_none() {
                // this probably can't happen
                self.leaf = None;
                return None;
            }
            let (node, j) = self.cache[i].unwrap();
            if j + 1 < node.get_children().len() {
                self.cache[i] = Some((node, j + 1));
                let mut node_down = &node.get_children()[j + 1];
                for k in (0..i).rev() {
                    self.cache[k] = Some((node_down, 0));
                    node_down = &node_down.get_children()[0];
                }
                self.leaf = Some(node_down.get_leaf());
                self.offset_of_leaf = self.position;
                return self.get_leaf();
            }
        }
        if self.offset_of_leaf + self.leaf.unwrap().len() == self.root.len() {
            self.leaf = None;
            return None;
        }
        self.descend();
        self.get_leaf()
    }

    /// Move to beginning of previous leaf.
    ///
    /// Return value: same as [`get_leaf`](#method.get_leaf).
    pub fn prev_leaf(&mut self) -> Option<(&'a N::L, usize)> {
        if self.offset_of_leaf == 0 {
            self.leaf = None;
            self.position = 0;
            return None;
        }
        for i in 0..CURSOR_CACHE_SIZE {
            if self.cache[i].is_none() {
                // this probably can't happen
                self.leaf = None;
                return None;
            }
            let (node, j) = self.cache[i].unwrap();
            if j > 0 {
                self.cache[i] = Some((node, j - 1));
                let mut node_down = &node.get_children()[j - 1];
                for k in (0..i).rev() {
                    let last_ix = node_down.get_children().len() - 1;
                    self.cache[k] = Some((node_down, last_ix));
                    node_down = &node_down.get_children()[last_ix];
                }
                let leaf = node_down.get_leaf();
                self.leaf = Some(leaf);
                self.offset_of_leaf -= leaf.len();
                self.position = self.offset_of_leaf;
                return self.get_leaf();
            }
        }
        self.position = self.offset_of_leaf - 1;
        self.descend();
        self.position = self.offset_of_leaf;
        self.get_leaf()
    }

    /// Go to the leaf containing the current position.
    ///
    /// Sets `leaf` to the leaf containing `position`, and updates `cache` and
    /// `offset_of_leaf` to be consistent.
    fn descend(&mut self) {
        let mut node = self.root;
        let mut offset = 0;
        while node.height() > 0 {
            let children = node.get_children();
            let mut i = 0;
            loop {
                if i + 1 == children.len() {
                    break;
                }
                let nextoff = offset + children[i].len();
                if nextoff > self.position {
                    break;
                }
                offset = nextoff;
                i += 1;
            }
            let cache_ix = node.height() - 1;
            if cache_ix < CURSOR_CACHE_SIZE {
                self.cache[cache_ix] = Some((node, i));
            }
            node = &children[i];
        }
        self.leaf = Some(node.get_leaf());
        self.offset_of_leaf = offset;
    }

    /// Returns the measure at the beginning of the leaf containing `pos`.
    ///
    /// This method is O(log n) no matter the current cursor state.
    fn measure_leaf<M: Metric<N>>(&self, mut pos: usize) -> usize {
        let mut node = self.root;
        let mut metric = 0;
        while node.height() > 0 {
            for child in node.get_children() {
                let len = child.len();
                if pos < len {
                    node = child;
                    break;
                }
                pos -= len;
                metric += child.measure::<M>();
            }
        }
        metric
    }

    /// Find the leaf having the given measure.
    ///
    /// This function sets `self.position` to the beginning of the leaf
    /// containing the smallest offset with the given metric, and also updates
    /// state as if [`descend`](#method.descend) was called.
    ///
    /// If `measure` is greater than the measure of the whole tree, then moves
    /// to the last node.
    fn descend_metric<M: Metric<N>>(&mut self, mut measure: usize) {
        let mut node = self.root;
        let mut offset = 0;
        while node.height() > 0 {
            let children = node.get_children();
            let mut i = 0;
            loop {
                if i + 1 == children.len() {
                    break;
                }
                let child = &children[i];
                let child_m = child.measure::<M>();
                if child_m >= measure {
                    break;
                }
                offset += child.len();
                measure -= child_m;
                i += 1;
            }
            let cache_ix = node.height() - 1;
            if cache_ix < CURSOR_CACHE_SIZE {
                self.cache[cache_ix] = Some((node, i));
            }
            node = &children[i];
        }
        self.leaf = Some(node.get_leaf());
        self.position = offset;
        self.offset_of_leaf = offset;
    }
}

/// An iterator generated by a [`Cursor`], for some [`Metric`].
///
/// [`Cursor`]: struct.Cursor.html
/// [`Metric`]: struct.Metric.html
pub struct CursorIter<'c, 'a: 'c, N: 'a + NodeInfo, M: 'a + Metric<N>> {
    cursor: &'c mut Cursor<'a, N>,
    _metric: PhantomData<&'a M>,
}

impl<'c, 'a, N: NodeInfo, M: Metric<N>> Iterator for CursorIter<'c, 'a, N, M> {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        self.cursor.next::<M>()
    }
}

impl<'c, 'a, N: NodeInfo, M: Metric<N>> CursorIter<'c, 'a, N, M> {
    /// Returns the current position of the underlying [`Cursor`].
    ///
    /// [`Cursor`]: struct.Cursor.html
    pub fn pos(&self) -> usize {
        self.cursor.pos()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::rope::*;

    fn build_triangle(n: u32) -> String {
        let mut s = String::new();
        let mut line = String::new();
        for _ in 0..n {
            s += &line;
            s += "\n";
            line += "a";
        }
        s
    }

    #[test]
    fn eq_rope_with_pieces() {
        let n = 2_000;
        let s = build_triangle(n);
        let mut builder_default = TreeBuilder::new();
        let mut concat_rope = Rope::default();
        builder_default.push_str(&s);
        let mut i = 0;
        while i < s.len() {
            let j = (i + 1000).min(s.len());
            concat_rope = concat_rope + s[i..j].into();
            i = j;
        }
        let built_rope = builder_default.build();
        assert_eq!(built_rope, concat_rope);
    }

    #[test]
    fn cursor_next_triangle() {
        let n = 2_000;
        let text = Rope::from(build_triangle(n));

        let mut cursor = Cursor::new(&text, 0);
        let mut prev_offset = cursor.pos();
        for i in 1..(n + 1) as usize {
            let offset = cursor.next::<LinesMetric>().expect("arrived at the end too soon");
            assert_eq!(offset - prev_offset, i);
            prev_offset = offset;
        }
        assert_eq!(cursor.next::<LinesMetric>(), None);
    }

    #[test]
    fn node_is_empty() {
        let text = Rope::from(String::new());
        assert_eq!(text.is_empty(), true);
    }

    #[test]
    fn cursor_next_empty() {
        let text = Rope::from(String::new());
        let mut cursor = Cursor::new(&text, 0);
        assert_eq!(cursor.next::<LinesMetric>(), None);
        assert_eq!(cursor.pos(), 0);
    }

    #[test]
    fn cursor_iter() {
        let text: Rope = build_triangle(50).into();
        let mut cursor = Cursor::new(&text, 0);
        let mut manual = Vec::new();
        while let Some(nxt) = cursor.next::<LinesMetric>() {
            manual.push(nxt);
        }

        cursor.set(0);
        let auto = cursor.iter::<LinesMetric>().collect::<Vec<_>>();
        assert_eq!(manual, auto);
    }

    #[test]
    fn cursor_next_misc() {
        cursor_next_for("toto");
        cursor_next_for("toto\n");
        cursor_next_for("toto\ntata");
        cursor_next_for("歴史\n科学的");
        cursor_next_for("\n歴史\n科学的\n");
        cursor_next_for(&build_triangle(100));
    }

    fn cursor_next_for(s: &str) {
        let r = Rope::from(s.to_owned());
        for i in 0..r.len() {
            let mut c = Cursor::new(&r, i);
            let it = c.next::<LinesMetric>();
            let pos = c.pos();
            assert!(s.as_bytes()[i..pos - 1].iter().all(|c| *c != b'\n'), "missed linebreak");
            if pos < s.len() {
                assert!(it.is_some(), "must be Some(_)");
                assert!(s.as_bytes()[pos - 1] == b'\n', "not a linebreak");
            } else {
                if s.as_bytes()[s.len() - 1] == b'\n' {
                    assert!(it.is_some(), "must be Some(_)");
                } else {
                    assert!(it.is_none());
                    assert!(c.get_leaf().is_none());
                }
            }
        }
    }

    #[test]
    fn cursor_prev_misc() {
        cursor_prev_for("toto");
        cursor_prev_for("a\na\n");
        cursor_prev_for("toto\n");
        cursor_prev_for("toto\ntata");
        cursor_prev_for("歴史\n科学的");
        cursor_prev_for("\n歴史\n科学的\n");
        cursor_prev_for(&build_triangle(100));
    }

    fn cursor_prev_for(s: &str) {
        let r = Rope::from(s.to_owned());
        for i in 0..r.len() {
            let mut c = Cursor::new(&r, i);
            let it = c.prev::<LinesMetric>();
            let pos = c.pos();

            //Should countain at most one linebreak
            assert!(
                s.as_bytes()[pos..i].iter().filter(|c| **c == b'\n').count() <= 1,
                "missed linebreak"
            );

            if i == 0 && s.as_bytes()[i] == b'\n' {
                assert_eq!(pos, 0);
            }

            if pos > 0 {
                assert!(it.is_some(), "must be Some(_)");
                assert!(s.as_bytes()[pos - 1] == b'\n', "not a linebreak");
            }
        }
    }

    #[test]
    fn at_or_next() {
        let text: Rope = "this\nis\nalil\nstring".into();
        let mut cursor = Cursor::new(&text, 0);
        assert_eq!(cursor.at_or_next::<LinesMetric>(), Some(5));
        assert_eq!(cursor.at_or_next::<LinesMetric>(), Some(5));
        cursor.set(1);
        assert_eq!(cursor.at_or_next::<LinesMetric>(), Some(5));
        assert_eq!(cursor.at_or_prev::<LinesMetric>(), Some(5));
        cursor.set(6);
        assert_eq!(cursor.at_or_prev::<LinesMetric>(), Some(5));
        cursor.set(6);
        assert_eq!(cursor.at_or_next::<LinesMetric>(), Some(8));
        assert_eq!(cursor.at_or_next::<LinesMetric>(), Some(8));
    }

    #[test]
    fn next_zero_measure_large() {
        let mut text = Rope::from("a");
        for _ in 0..24 {
            text = Node::concat(text.clone(), text);
            let mut cursor = Cursor::new(&text, 0);
            assert_eq!(cursor.next::<LinesMetric>(), None);
            // Test that cursor is properly invalidated and at end of text.
            assert_eq!(cursor.get_leaf(), None);
            assert_eq!(cursor.pos(), text.len());

            cursor.set(text.len());
            assert_eq!(cursor.prev::<LinesMetric>(), None);
            // Test that cursor is properly invalidated and at beginning of text.
            assert_eq!(cursor.get_leaf(), None);
            assert_eq!(cursor.pos(), 0);
        }
    }

    #[test]
    fn prev_line_large() {
        let s: String = format!("{}{}", "\n", build_triangle(1000));
        let rope = Rope::from(s);
        let mut expected_pos = rope.len();
        let mut cursor = Cursor::new(&rope, rope.len());

        for i in (1..1001).rev() {
            expected_pos = expected_pos - i;
            assert_eq!(expected_pos, cursor.prev::<LinesMetric>().unwrap());
        }

        assert_eq!(None, cursor.prev::<LinesMetric>());
    }

    #[test]
    fn prev_line_small() {
        let empty_rope = Rope::from("\n");
        let mut cursor = Cursor::new(&empty_rope, empty_rope.len());
        assert_eq!(None, cursor.prev::<LinesMetric>());

        let rope = Rope::from("\n\n\n\n\n\n\n\n\n\n");
        cursor = Cursor::new(&rope, rope.len());
        let mut expected_pos = rope.len();
        for _ in (1..10).rev() {
            expected_pos -= 1;
            assert_eq!(expected_pos, cursor.prev::<LinesMetric>().unwrap());
        }

        assert_eq!(None, cursor.prev::<LinesMetric>());
    }

    #[test]
    fn balance_invariant() {
        let mut tb = TreeBuilder::<RopeInfo>::new();
        let leaves: Vec<String> = (0..1000).map(|i| i.to_string().into()).collect();
        tb.push_leaves(leaves);
        let tree = tb.build();
        println!("height {}", tree.height());
    }
}
#[cfg(test)]
mod tests_llm_16_417 {
    use super::*;

use crate::*;
    use crate::rope::{Rope, LinesMetric};

    #[test]
    fn test_iter() {
        let text: Rope = "one line\ntwo line\nred line\nblue".into();
        let mut cursor = Cursor::new(&text, 0);
        let line_offsets = cursor.iter::<LinesMetric>().collect::<Vec<_>>();
        assert_eq!(line_offsets, vec![9, 18, 27]);
    }
}#[cfg(test)]
mod tests_llm_16_466_llm_16_465 {
    use super::*;

use crate::*;
    use crate::breaks::{BreaksBaseMetric, BreaksInfo, BreaksLeaf};

    #[test]
    fn test_len() {
        let node = Node::<BreaksInfo>::from_leaf(BreaksLeaf::default());
        assert_eq!(node.len(), 0);
    }
}#[cfg(test)]
mod tests_llm_16_477 {
    use super::*;

use crate::*;
    use crate::interval::Interval;

    #[test]
    fn test_subseq() {
        // TODO: Create test case
    }
}#[cfg(test)]
mod tests_llm_16_486 {
    use super::*;

use crate::*;
    use breaks::BreaksBaseMetric;
    use breaks::BreaksInfo;
    use std::clone::Clone;
    use std::fmt::Debug;
    use std::marker::Copy;
    use tree::Metric;

    #[test]
    fn test_new() {
        let builder: TreeBuilder<BreaksInfo> = TreeBuilder::new();
        assert!(builder.stack.is_empty());
    }
}#[cfg(test)]
mod tests_rug_91 {
    use super::*;
    use crate::tree::NodeInfo;

    #[test]
    fn test_rug() {
        NodeInfo::<T>::identity();
    }
}
#[cfg(test)]
mod tests_rug_92 {
    use super::*;
    use rope::tree::tree::NodeInfo;
    use rope::interval::Interval;
    
    #[test]
    fn test_interval() {
        // Constructing the first argument
        // Assuming the sample data for BreaksLeaf is [0, 2, 4, 6]
        let l = BreaksLeaf {
            data: vec![0, 2, 4, 6],
        };
        let p0 = BreaksInfo::compute_info(&l);
        
        // Constructing the second argument
        let p1: usize = 5;

        
        // Calling the interval function
        NodeInfo::interval(&p0, p1);
        
        // Add your assertions here
        
    }
}
#[cfg(test)]
mod tests_rug_93 {
    use super::*;
    use crate::rope::RopeInfo;
    use crate::spans::{SpansLeaf, Span};
    use crate::interval::Interval;

    #[test]
    fn test_rug() {
        let mut v26: RopeInfo = RopeInfo::identity();
        // Modify v26 if needed

        let v58: SpansLeaf<RopeInfo> = SpansLeaf::default();
        let mut v25: Interval = Interval::new(2, 5);

        crate::tree::Leaf::subseq(&v58, v25);
    }
}#[cfg(test)]
mod tests_rug_94 {
    use super::*;
    use crate::tree;

    #[test]
    fn test_rug() {
        let mut v61: tree::StrArena = tree::StrArena::new();
        let mut p0: <N as tree::NodeInfo>::L = v61;
        
        <tree::Node<N>>::from_leaf(p0);
    }
}#[cfg(test)]
mod tests_rug_95 {
    use super::*;
    use std::vec::Vec;
    use rope::tree::{Node, MAX_CHILDREN, NodeBody, NodeVal};
    use std::sync::Arc;
                 
    #[test]
    fn test_rug() {
        let mut p0: Vec<Node<N>> = Vec::new();
        // Add sample data if necessary
                
        <tree::Node<N>>::from_nodes(p0); 
    }
}
#[cfg(test)]
mod tests_rug_96 {
    use super::*;
    use rope::tree::Node;

    #[test]
    fn test_is_empty() {
        let mut p0: Node<N> = Node::new();
        <tree::Node<N>>::is_empty(&p0);
    }
}
#[cfg(test)]
mod tests_rug_97 {
    use super::*;
    use rope::tree::Node;
    use std::sync::Arc;
    
    #[test]
    fn test_ptr_eq() {
        let p0: Node<N> = Node::new();
        let p1: Node<N> = Node::new();
        
        <tree::Node<N>>::ptr_eq(&p0, &p1);
    }
}#[cfg(test)]
mod tests_rug_98 {
    use super::*;
    use rope::tree::Node;  // Import the necessary module
    
    #[test]
    fn test_rug() {
        let mut p0: Node<N> = Node::new();

        <tree::Node<N>>::height(&p0);
        
    }
}#[cfg(test)]
mod tests_rug_99 {
    use super::*;
    use rope::tree::Node;

    #[test]
    fn test_rug() {
        let mut p0: Node<N> = Node::new();

        <tree::Node<N>>::is_leaf(&p0);
    }
}#[cfg(test)]
mod tests_rug_100 {
    use super::*;
    use rope::Interval;
    use rope::tree::Node;
    
    #[test]
    fn test_interval() {
        let mut p0: Node<N> = Node::new();

        p0.interval();
    }
}#[cfg(test)]
mod tests_rug_101 {
    use super::*;
    use rope::tree::Node;
    
    #[test]
    fn test_rug() {
        let mut p0: Node<N> = Node::new();
        <tree::Node<N>>::get_children(&p0);
    }
}
#[cfg(test)]
mod tests_rug_102 {
    use super::*;
    use rope::tree::Node as BaseNode;

    #[test]
    fn test_rug() {
        let mut p0: BaseNode<N> = BaseNode::new();

        <BaseNode<N>>::get_leaf(&p0);
    }
}        
#[cfg(test)]
mod tests_rug_103 {
    use super::*;
    use rope::tree::Node;
    use std::panic::AssertUnwindSafe;
    use crate::batch::Builder;
    
    #[test]
    fn test_rug() {
        let mut p0: Node<N> = Node::new();
        let mut p1: AssertUnwindSafe<Builder> = AssertUnwindSafe::new(Builder::new());
        
        Node::<N>::with_leaf_mut(p0, p1);
    
    }
}
        
#[cfg(test)]
mod tests_rug_104 {

    use super::*;
    use super::super::rope::tree::Leaf;
    use super::super::rope::tree::{Node, NodeVal, MIN_CHILDREN};
    
    #[test]
    fn test_rug() {
        let mut p0: Node<N> = Node::new();

        match p0.0.val {
            NodeVal::Leaf(ref l) => l.is_ok_child(),
            NodeVal::Internal(ref nodes) => (nodes.len() >= MIN_CHILDREN),
        };
    }
}
#[cfg(test)]
mod tests_rug_105 {
    use super::*;
    use rope::tree::Node;

    #[test]
    fn test_merge_nodes() {
        let mut p0: Vec<Node<N>> = Vec::new();
        let mut p1: Vec<Node<N>> = Vec::new();
        
        // Insert test data for p0 and p1 here
        
        <tree::Node<N>>::merge_nodes(&p0, &p1);
    }
}     
#[cfg(test)]
mod tests_rug_106 {
    use super::*;
    use rope::interval::Interval;
    use rope::leaf::Leaf;
    use rope::node::{Node, NodeVal};
    use std::sync::Arc;

    #[test]
    fn test_merge_leaves() {
        let mut p0 = Node::<N>::new();
        let mut p1 = Node::<N>::new();

        let leaf0 = Leaf::<N>::new();
        let leaf1 = Leaf::<N>::new();

        let sample_interval = Interval::new(0, 10);
        leaf0.push_maybe_split(leaf1, sample_interval);
        
        p0.val = NodeVal::Leaf(leaf0);

        merge_leaves(p0, p1);
    }
}
#[cfg(test)]
mod tests_rug_107 {
    use super::*;
    use crate::tree::{Node, Leaf, NodeChildren};

    #[test]
    fn test_concat() {
        let mut p0: Node<i32> = Node::new();
        let mut p1: Node<i32> = Node::new();

        // test when h1 < h2 and h1 == h2 - 1 and rope1 is an ok child
        let mut p2: Node<i32> = Node::new();
        let mut p3: Leaf<i32> = Leaf::new();
        let mut p4: Leaf<i32> = Leaf::new();
        let children2: NodeChildren<i32> = vec![p3.into(), p4.into()];
        p0.set_height(0);
        p0.set_is_ok_child(true);
        p1.set_height(1);
        p2.set_height(0);
        p2.set_is_ok_child(true);
        assert_eq!(Node::<i32>::concat(p0.clone(), p1.clone()), Node::<i32>::merge_nodes(&vec![p0.clone()], &children2[..]));
        
        // test when h1 < h2 and newrope.height() == h2 - 1
        let mut child2_1: Node<i32> = Node::new();
        child2_1.set_height(0);
        
        assert_eq!(Node::<i32>::concat(p0.clone(), child2_1), Node::<i32>::merge_nodes(&vec![p0], &children2[1..]));
        
         // test when h1 < h2 and newrope.height() != h2 - 1
         let mut newrope_3_children: Leaf<i32> = Leaf::new();
         newrope_3_children.set_height(0);
         let mut newrope_children: NodeChildren<i32> = vec![newrope_3_children.into()];
         let mut children2_1: Node<i32> = Node::new();
         let mut children2_2: Leaf<i32> = Leaf::new();
         children2_2.set_height(0);
         let children2: NodeChildren<i32> = vec![children2_1.into(), children2_2.into()];
         assert_eq!(Node::<i32>::concat(p0.clone(), children2_1.clone()), Node::<i32>::merge_nodes(&newrope_children, &children2[1..]));
        
        // test when h1 == h2 and rope1 is an ok child and rope2 is an ok child
        let mut p5: Leaf<i32> = Leaf::new();
        p5.set_height(0);
        assert_eq!(Node::<i32>::concat(p0.clone(), p5), Node::<i32>::from_nodes(vec![p0, p5]));
        
        // test when h1 == 0
        let mut p6: Leaf<i32> = Leaf::new();
        p6.set_height(0);
        assert_eq!(Node::<i32>::concat(p6, p5), Node::<i32>::merge_leaves(p6, p5));
        
        // test when h1 == h2
        let mut child1_1: Node<i32> = Node::new();
        let mut child1_2: Leaf<i32> = Leaf::new();
        child1_2.set_height(0);
        let children1: NodeChildren<i32> = vec![child1_1.into(), child1_2.into()];
        
        assert_eq!(Node::<i32>::concat(child1_1.clone(), children2_1.clone()), Node::<i32>::merge_nodes(&children1, &children2));
        
        // test when h1 > h2 and h2 == h1 - 1 and rope2 is an ok child
        let mut p7: Node<i32> = Node::new();
        let mut p8: Leaf<i32> = Leaf::new();
        let mut p9: Leaf<i32> = Leaf::new();
        let children1: NodeChildren<i32> = vec![p8.into(), p9.into()];
        p7.set_height(0);
        p7.set_is_ok_child(true);
        p7.set_is_ok_child(true);
        
        assert_eq!(Node::<i32>::concat(child1_2.clone(), child2_1.clone()), Node::<i32>::merge_nodes(&children1[..], &vec![child2_1.clone()]));
        
       // test when newrope.height() == h2 - 1
       let mut newrope_7_children: Leaf<i32> = Leaf::new();
       newrope_7_children.set_height(0);
       let mut children1_2: Node<i32> = Node::new();
       children1_2.set_height(0);
       children1_2.set_is_ok_child(true);
       let mut newrope_children: NodeChildren<i32> = vec![newrope_7_children.into()];
       let children1: NodeChildren<i32> = vec![children1[0].clone().into()];
       
      assert_eq!(Node::<i32>::concat(children1[0].clone(), child2_1.clone()), Node::<i32>::merge_nodes(&children[..lastix], &vec![child2_1.clone()]));
       
      // test when newrope.height() != h2 - 1
      assert_eq!(Node::<i32>::concat(children1[0].clone(), children2_1.clone()), Node::<i32>::merge_nodes(&children[..lastix], children2_1.get_children()));
    }
}#[cfg(test)]
        mod tests_rug_108 {
            use super::*;
            use rope::tree::Node;
            
            #[test]
            fn test_rug() {
                let mut p0: Node<N> = Node::new();

                
                <tree::Node<N>>::measure::<M>(p0);

            }
        }#[cfg(test)]
mod tests_rug_109 {
    use super::*;
    use rope::tree::Node;
    use rope::tree::TreeBuilder;
    use crate::interval::Interval;
    
    #[test]
    fn test_rug() {
        let mut p0: Node<N> = Node::new();
        let mut p1: TreeBuilder<N> = TreeBuilder::new();
        let mut p2: Interval = Interval::new(2, 5);
        
        <tree::Node<N>>::push_subseq(&p0, &mut p1, p2);
    }
}#[cfg(test)]
mod tests_rug_110 {
    use super::*;
    use rope::tree::Node;
    use std::ops::RangeFull;
    use std::sync::mpsc::sync_channel;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_rug() {
        let mut p0: Node<N> = Node::new();
        let mut p1: RangeFull = ..;
        let (sx, rx) = sync_channel::<Node<N>>(0);
        let sel = Arc::new(Mutex::new(vec![rx]));
        let mut p2 = xi_rope::rope::tree::Selected(sel);

        Node::<N>::edit(&mut p0, &mut p1, &mut p2);
    }
}#[cfg(test)]
mod tests_rug_111 {
    use super::*;
    use rope::Metric;
    use rope::tree::Node;

    #[test]
    fn test_convert_metrics() {
        let mut v63: Node<N> = Node::new();
        let mut p0: &Node<N> = &v63;
        let p1: usize = 10;

        p0.convert_metrics::<M1, M2>(p1);
    }
}#[cfg(test)]
mod tests_rug_112 {
    use super::*;
    use crate::xi_rope::{Rope, LinesMetric};
    use crate::xi_rope::tree::Node;
    
    #[test]
    fn test_rug() {
        let my_rope = Rope::from("first line \n second line \n");
        let p0 = Node::<N>::new();
        let p1: usize = 10;
        
        my_rope.count::<LinesMetric>(p0, p1);
    }
}#[cfg(test)]
mod tests_rug_113 {
    use super::*;
    use crate::xi_rope::{Rope, LinesMetric};
    use crate::xi_rope::rope::RopeTrait;  // Import the necessary traits and methods
    
    #[test]
    fn test_rug() {
        let mut p0: tree::Node<N> = tree::Node::<N>::new();  // Create the local variable p0 with type tree::Node<N> using the constructor method
        
        let p1: usize = 1;  // Initialize p1 with a sample value
        
        p0.count_base_units::<LinesMetric>(p1);
    }
}        
#[cfg(test)]
mod tests_rug_114 {
    use super::*;
    use crate::rope::node::Node;
    use crate::std::default::Default;
    
    #[test]
    fn test_default() {
        let default_node: Node<TypeN> = Node::<TypeN>::default();
        
        // Perform assertions or other checks
        
        assert_eq!(default_node, Node::<TypeN>::from_leaf(TypeL::default()));
        // Add more assertions if needed
        
    }
}
#[cfg(test)]
mod tests_rug_115 {
    use super::*;
    use rope::tree::{Node, TreeBuilder};  // Import necessary modules

    #[test]
    fn test_rug() {
        let mut p0: TreeBuilder<N> = TreeBuilder::new();
        let mut p1: Node<N> = Node::new();

        <tree::TreeBuilder<N>>::push(&mut p0, p1);
    }
}

#[cfg(test)]
mod tests_rug_116 {
    use super::*;
    use rope::tree::TreeBuilder;
    use rope::tree::Rope;
    
    #[test]
    fn test_rug() {
        let mut p0: TreeBuilder<N> = TreeBuilder::new();
        let mut p1: Option<Rope> = Some(Rope::from("sample data"));
                
        <tree::TreeBuilder<N>>::push_leaves(&mut p0, p1);

    }
}#[cfg(test)]
mod tests_rug_117 {
    use super::*;
    use crate::rope::tree;
    
    #[test]
    fn test_rug() {
        let mut v66: tree::TreeBuilder<N> = tree::TreeBuilder::new();
        let mut v61: <N as tree::NodeInfo>::L = <N as tree::NodeInfo>::L::new();
        tree::TreeBuilder::<N>::push_leaf(&mut v66, v61);
    }
}#[cfg(test)]
mod tests_rug_118 {
    use super::*;
    use rope::tree::{Node, StrArena, TreeBuilder};
    use crate::interval::Interval;

    #[test]
    fn test_push_leaf_slice() {
        let mut p0: TreeBuilder<N> = TreeBuilder::new();
        let mut p1: StrArena = StrArena::new();
        let mut p2: Interval = Interval::new(2, 5);

        <tree::TreeBuilder<N>>::push_leaf_slice(&mut p0, &p1, p2);
    }
}
#[cfg(test)]
mod tests_rug_119 {

    use super::*;
    use rope::tree::TreeBuilder;
    use rope::tree::Node;

    #[test]
    fn test_rug() {
        let mut v66: TreeBuilder<N> = TreeBuilder::new();
        let p0 = v66.build();
        
        tree::TreeBuilder::<N>::build(p0);

    }
}#[cfg(test)]
mod tests_rug_120 {
    use super::*;
    use ::rope::tree::Node;

    #[test]
    fn test_rug() {
        let mut p0: TreeBuilder<N> = TreeBuilder::new();
        <tree::TreeBuilder<N>>::pop(&mut p0);
        
        // Add assertions here
    }
}#[cfg(test)]
mod tests_rug_121 {
    use super::*;
    use rope::tree::Cursor;
    use rope::tree::Node;

    #[test]
    fn test_rug() {
        let mut p0: Node<N> = Node::new();
        let p1: usize = 10;
        
        Cursor::new(&p0, p1);
    }
}#[cfg(test)]
mod tests_rug_122 {
    use super::*;
    use crate::tree::Cursor;

    #[test]
    fn test_rug() {
        // Create a sample `Cursor`
        let mut p0: Cursor<'static, N > = Cursor::default();

        // Call the `total_len` function
        p0.total_len();
    }
}                
#[cfg(test)]
mod tests_rug_123 {
    use super::*;
    use rope::tree::Cursor;
        
    #[test]
    fn test_root() {
        
        let rope = Rope::from("") ;
        let cursor = Cursor::new(&rope, 0);
        
        Cursor::root(&cursor);
    }
}
#[cfg(test)]
mod tests_rug_124 {
    use super::*;
    use crate::tree;

    #[test]
    fn test_get_leaf() {
        let leaf = <tree::Cursor<'_, _>>::get_leaf(cursor);
        // Add your assertions here
    }
}#[cfg(test)]
mod tests_rug_125 {
    use super::*;
    use rope::tree::Cursor;

    #[test]
    fn test_rug() {
        let mut p0: Cursor<'static, usize> = Cursor {
            node: None,
            start: None,
            end: None,
        };
        let p1: usize = 10;

        p0.set(p1);

    }
}        
#[cfg(test)]
mod tests_rug_126 {
    use super::*;
    use crate::tree::Cursor;

    #[test]
    fn test_pos() {
        let position = 10; // Sample value for position
        
        let mut cursor = Cursor::new(); // Create a sample cursor
        cursor.position = position; // Set the position
        
        let result = cursor.pos(); // Call the pos() function
        
        assert_eq!(result, position); // Check if the result matches the expected position value
    }
}#[cfg(test)]
mod tests_rug_127 {
    use super::*;
    use rope::tree::{Metric, Cursor};
    use rope::Rope;
    
    #[test]
    fn test_rug() {
        let rope = Rope::from("Hello, world!");

        let mut cursor = Cursor::new(&rope);
        
        assert_eq!(cursor.is_boundary::<Metric<Rope>>(), false);
    }
}        
#[cfg(test)]
mod tests_rug_128 {
    use super::*;
    use crate::rope::RopeInfo;
    use crate::tree::{Cursor, Metric, Node};

    #[test]
    fn test_prev() {
        let info = RopeInfo::new();
        let root = Node::Leaf("Hello world".into());
        let mut cursor: Cursor<Node> = Cursor::new(&info, &root);

        assert_eq!(cursor.prev::<Metric<Node>>(), None);
    }
}

#[cfg(test)]
mod tests_rug_129 {
    use super::*;
    use crate::tree;
    
    #[test]
    fn test_rug() {
        let mut p0 = tree::Cursor {
            position: 0,
            root: Vec::new(),
            leaf: None,
            leaf_pos: 0,
        };

        <tree::Cursor<'a, N>>::next::<M>(&mut p0);
    }
}
#[cfg(test)]
mod tests_rug_130 {
    use super::*;
    use crate::rope::tree::Cursor;
    use crate::rope::Metric;

    #[test]
    fn test_rug() {
        let mut p0: Cursor<'static, Node<LeafData>> = Cursor::new(tree, offset);
        p0.at_or_next::<SomeMetric>();
    }
}#[cfg(test)]
mod tests_rug_131 {
    use super::*;
    use crate::tree::Cursor;

    #[test]
    fn test_rug() {
        let mut p0 = Cursor::<'a, N>::total_len();

        Cursor::<'a, N>::at_or_prev::<M>(&mut p0);
    }
}#[cfg(test)]
mod tests_rug_132 {
    use super::*;
    use crate::tree::{Cursor, Node};

    #[test]
    fn test_last_inside_leaf() {
        // construct variables
        let mut nodes: Vec<Node> = Vec::new();
        let leaf = Node::Leaf("example leaf".to_string());
        nodes.push(leaf);
        let cursor = Cursor::<'_, Node> {
            nodes: &nodes,
            node_idx: 0,
            offset_of_leaf: 0,
            position: 0,
            leaf: Some(&leaf),
        };
        let orig_pos = 10;

        // call the target function
        cursor.last_inside_leaf::<LengthMetric<Node>>(orig_pos);
    }
}#[cfg(test)]
mod tests_rug_133 {
    use super::*;
    use rope::tree::{Rope, Node};
    
    #[test]
    fn test_rug() {
        let data = Rope::from(vec!["abc", "def", "ghi"]);
        let root = Node::Leaf(data);
        let cursor = tree::Cursor::<'_, Node>::root(&root);
        
        let mut p0 = cursor;

        p0.next_inside_leaf::<tree::metric::RopeMetric>();
    }
}#[cfg(test)]
mod tests_rug_134 {
    use super::*;
    use rope::tree::{Cursor, Node};

    #[test]
    fn test_rug() {
        let mut p0: Cursor<Node> = Cursor::new();
        
        // Construct the necessary variables for p0
        
        Cursor::next_leaf(&mut p0);
    }
}
#[cfg(test)]
mod tests_rug_135 {
    use super::*;
    use rope::Rope;
    use rope::base::Cursor;
    
    #[test]
    fn test_rug() {
        
        // Constructing the Rope
        let mut rope = Rope::from("Hello, World!");
        
        // Creating the Cursor
        let cursor = Cursor::new(&rope, 0);
        
        // Moving to the beginning of previous leaf
        let mut p0 = cursor;
        <tree::Cursor<'a, Rope>>::prev_leaf(&mut p0);
        
    }
}
#[cfg(test)]
mod tests_rug_136 {
    use super::*;
    use crate::tree::{TreeBuilder, LeafNode};

    #[test]
    fn test_rug() {
        let mut builder = TreeBuilder::new();
        builder.push("Hello");
        builder.push(" World");
        let leaf_node = LeafNode::new(builder.build());
        let mut cursor = Cursor::new(leaf_node);

        Cursor::<'a, LeafNode>::descend(&mut cursor);
    }
}#[cfg(test)]
mod tests_rug_137 {
    use super::*;
    use rope::Rope;

    #[test]
    fn test_measure_leaf() {
        let rope = Rope::from("Hello, world!");
        let cursor = rope.cursor();
        let pos = 5;

        assert_eq!(cursor.measure_leaf::<usize>(pos), 5);
    }
}#[cfg(test)]
mod tests_rug_138 {
    use super::*;
    use rope::Cursor;
    
    #[test]
    fn test_rug() {
        let mut p0 = Cursor::new(); // Construct a new Cursor<'a, N> object
        let p1: usize = 100; // Sample value for p1
        
        p0.descend_metric::<M>(p1);
    }
}#[cfg(test)]
mod tests_rug_139 {
    use super::*;
    use crate::rope::tree::CursorIter;
    
    #[test]
    fn test_rug() {
        let mut p0: CursorIter<'_, '_, N, M> = CursorIter::new();
        
        <tree::CursorIter<'c, 'a, N, M>>::next(&mut p0);
    }
}        
#[cfg(test)]
mod tests_rug_140 {
    use super::*;
    use rope::tree::CursorIter;
    
    #[test]
    fn test_rug() {
        let mut p0: CursorIter<'_, '_, N, M> = CursorIter::new();
        
        <tree::CursorIter<'c, 'a, N, M>>::pos(p0);
    }
}
                            