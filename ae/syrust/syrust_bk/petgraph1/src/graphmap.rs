//! `GraphMap<N, E, Ty>` is a graph datastructure where node values are mapping
//! keys.

use indexmap::map::Keys;
use indexmap::map::{Iter as IndexMapIter, IterMut as IndexMapIterMut};
use indexmap::IndexMap;
use std::cmp::Ordering;
use std::fmt;
use std::hash::{self, Hash};
use std::iter::FromIterator;
use std::iter::{Cloned, DoubleEndedIterator};
use std::marker::PhantomData;
use std::ops::{Deref, Index, IndexMut};
use std::slice::Iter;

use crate::{Directed, Direction, EdgeType, Incoming, Outgoing, Undirected};

use crate::graph::node_index;
use crate::graph::Graph;
use crate::visit::{IntoEdgeReferences, IntoEdges, NodeCompactIndexable};
use crate::visit::{IntoNodeIdentifiers, IntoNodeReferences, NodeCount, NodeIndexable};
use crate::IntoWeightedEdge;

/// A `GraphMap` with undirected edges.
///
/// For example, an edge between *1* and *2* is equivalent to an edge between
/// *2* and *1*.
pub type UnGraphMap<N, E> = GraphMap<N, E, Undirected>;
/// A `GraphMap` with directed edges.
///
/// For example, an edge from *1* to *2* is distinct from an edge from *2* to
/// *1*.
pub type DiGraphMap<N, E> = GraphMap<N, E, Directed>;

/// `GraphMap<N, E, Ty>` is a graph datastructure using an associative array
/// of its node weights `N`.
///
/// It uses an combined adjacency list and sparse adjacency matrix
/// representation, using **O(|V| + |E|)** space, and allows testing for edge
/// existence in constant time.
///
/// `GraphMap` is parameterized over:
///
/// - Associated data `N` for nodes and `E` for edges, called *weights*.
/// - The node weight `N` must implement `Copy` and will be used as node
/// identifier, duplicated into several places in the data structure.
/// It must be suitable as a hash table key (implementing `Eq + Hash`).
/// The node type must also implement `Ord` so that the implementation can
/// order the pair (`a`, `b`) for an edge connecting any two nodes `a` and `b`.
/// - `E` can be of arbitrary type.
/// - Edge type `Ty` that determines whether the graph edges are directed or
/// undirected.
///
/// You can use the type aliases `UnGraphMap` and `DiGraphMap` for convenience.
///
/// `GraphMap` does not allow parallel edges, but self loops are allowed.
///
/// Depends on crate feature `graphmap` (default).
#[derive(Clone)]
pub struct GraphMap<N, E, Ty> {
    nodes: IndexMap<N, Vec<(N, CompactDirection)>>,
    edges: IndexMap<(N, N), E>,
    ty: PhantomData<Ty>,
}

impl<N: Eq + Hash + fmt::Debug, E: fmt::Debug, Ty: EdgeType> fmt::Debug for GraphMap<N, E, Ty> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.nodes.fmt(f)
    }
}

/// A trait group for `GraphMap`'s node identifier.
pub trait NodeTrait: Copy + Ord + Hash {}
impl<N> NodeTrait for N where N: Copy + Ord + Hash {}

// non-repr(usize) version of Direction
#[derive(Copy, Clone, Debug, PartialEq)]
enum CompactDirection {
    Outgoing,
    Incoming,
}

impl From<Direction> for CompactDirection {
    fn from(d: Direction) -> Self {
        match d {
            Outgoing => CompactDirection::Outgoing,
            Incoming => CompactDirection::Incoming,
        }
    }
}

impl PartialEq<Direction> for CompactDirection {
    fn eq(&self, rhs: &Direction) -> bool {
        (*self as usize) == (*rhs as usize)
    }
}

impl<N, E, Ty> GraphMap<N, E, Ty>
where
    N: NodeTrait,
    Ty: EdgeType,
{
    /// Create a new `GraphMap`
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new `GraphMap` with estimated capacity.
    pub fn with_capacity(nodes: usize, edges: usize) -> Self {
        GraphMap {
            nodes: IndexMap::with_capacity(nodes),
            edges: IndexMap::with_capacity(edges),
            ty: PhantomData,
        }
    }

    /// Return the current node and edge capacity of the graph.
    pub fn capacity(&self) -> (usize, usize) {
        (self.nodes.capacity(), self.edges.capacity())
    }

    /// Use their natural order to map the node pair (a, b) to a canonical edge id.
    #[inline]
    fn edge_key(a: N, b: N) -> (N, N) {
        if Ty::is_directed() || a <= b {
            (a, b)
        } else {
            (b, a)
        }
    }

    /// Whether the graph has directed edges.
    pub fn is_directed(&self) -> bool {
        Ty::is_directed()
    }

    /// Create a new `GraphMap` from an iterable of edges.
    ///
    /// Node values are taken directly from the list.
    /// Edge weights `E` may either be specified in the list,
    /// or they are filled with default values.
    ///
    /// Nodes are inserted automatically to match the edges.
    ///
    /// ```
    /// use petgraph::graphmap::UnGraphMap;
    ///
    /// // Create a new undirected GraphMap.
    /// // Use a type hint to have `()` be the edge weight type.
    /// let gr = UnGraphMap::<_, ()>::from_edges(&[
    ///     (0, 1), (0, 2), (0, 3),
    ///     (1, 2), (1, 3),
    ///     (2, 3),
    /// ]);
    /// ```
    pub fn from_edges<I>(iterable: I) -> Self
    where
        I: IntoIterator,
        I::Item: IntoWeightedEdge<E, NodeId = N>,
    {
        Self::from_iter(iterable)
    }

    /// Return the number of nodes in the graph.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Return the number of edges in the graph.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Remove all nodes and edges
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.edges.clear();
    }

    /// Add node `n` to the graph.
    pub fn add_node(&mut self, n: N) -> N {
        self.nodes.entry(n).or_insert(Vec::new());
        n
    }

    /// Return `true` if node `n` was removed.
    ///
    /// Computes in **O(V)** time, due to the removal of edges with other nodes.
    pub fn remove_node(&mut self, n: N) -> bool {
        let links = match self.nodes.swap_remove(&n) {
            None => return false,
            Some(sus) => sus,
        };
        for (succ, _) in links {
            // remove all successor links
            self.remove_single_edge(&succ, &n, Incoming);
            // Remove all edge values
            self.edges.swap_remove(&Self::edge_key(n, succ));
        }
        true
    }

    /// Return `true` if the node is contained in the graph.
    pub fn contains_node(&self, n: N) -> bool {
        self.nodes.contains_key(&n)
    }

    /// Add an edge connecting `a` and `b` to the graph, with associated
    /// data `weight`. For a directed graph, the edge is directed from `a`
    /// to `b`.
    ///
    /// Inserts nodes `a` and/or `b` if they aren't already part of the graph.
    ///
    /// Return `None` if the edge did not previously exist, otherwise,
    /// the associated data is updated and the old value is returned
    /// as `Some(old_weight)`.
    ///
    /// ```
    /// // Create a GraphMap with directed edges, and add one edge to it
    /// use petgraph::graphmap::DiGraphMap;
    ///
    /// let mut g = DiGraphMap::new();
    /// g.add_edge("x", "y", -1);
    /// assert_eq!(g.node_count(), 2);
    /// assert_eq!(g.edge_count(), 1);
    /// assert!(g.contains_edge("x", "y"));
    /// assert!(!g.contains_edge("y", "x"));
    /// ```
    pub fn add_edge(&mut self, a: N, b: N, weight: E) -> Option<E> {
        if let old @ Some(_) = self.edges.insert(Self::edge_key(a, b), weight) {
            old
        } else {
            // insert in the adjacency list if it's a new edge
            self.nodes
                .entry(a)
                .or_insert_with(|| Vec::with_capacity(1))
                .push((b, CompactDirection::Outgoing));
            if a != b {
                // self loops don't have the Incoming entry
                self.nodes
                    .entry(b)
                    .or_insert_with(|| Vec::with_capacity(1))
                    .push((a, CompactDirection::Incoming));
            }
            None
        }
    }

    /// Remove edge relation from a to b
    ///
    /// Return `true` if it did exist.
    fn remove_single_edge(&mut self, a: &N, b: &N, dir: Direction) -> bool {
        match self.nodes.get_mut(a) {
            None => false,
            Some(sus) => {
                if Ty::is_directed() {
                    match sus
                        .iter()
                        .position(|elt| elt == &(*b, CompactDirection::from(dir)))
                    {
                        Some(index) => {
                            sus.swap_remove(index);
                            true
                        }
                        None => false,
                    }
                } else {
                    match sus.iter().position(|elt| &elt.0 == b) {
                        Some(index) => {
                            sus.swap_remove(index);
                            true
                        }
                        None => false,
                    }
                }
            }
        }
    }

    /// Remove edge from `a` to `b` from the graph and return the edge weight.
    ///
    /// Return `None` if the edge didn't exist.
    ///
    /// ```
    /// // Create a GraphMap with undirected edges, and add and remove an edge.
    /// use petgraph::graphmap::UnGraphMap;
    ///
    /// let mut g = UnGraphMap::new();
    /// g.add_edge("x", "y", -1);
    ///
    /// let edge_data = g.remove_edge("y", "x");
    /// assert_eq!(edge_data, Some(-1));
    /// assert_eq!(g.edge_count(), 0);
    /// ```
    pub fn remove_edge(&mut self, a: N, b: N) -> Option<E> {
        let exist1 = self.remove_single_edge(&a, &b, Outgoing);
        let exist2 = if a != b {
            self.remove_single_edge(&b, &a, Incoming)
        } else {
            exist1
        };
        let weight = self.edges.remove(&Self::edge_key(a, b));
        debug_assert!(exist1 == exist2 && exist1 == weight.is_some());
        weight
    }

    /// Return `true` if the edge connecting `a` with `b` is contained in the graph.
    pub fn contains_edge(&self, a: N, b: N) -> bool {
        self.edges.contains_key(&Self::edge_key(a, b))
    }

    /// Return an iterator over the nodes of the graph.
    ///
    /// Iterator element type is `N`.
    pub fn nodes(&self) -> Nodes<N> {
        Nodes {
            iter: self.nodes.keys().cloned(),
        }
    }

    /// Return an iterator of all nodes with an edge starting from `a`.
    ///
    /// - `Directed`: Outgoing edges from `a`.
    /// - `Undirected`: All edges from or to `a`.
    ///
    /// Produces an empty iterator if the node doesn't exist.<br>
    /// Iterator element type is `N`.
    pub fn neighbors(&self, a: N) -> Neighbors<N, Ty> {
        Neighbors {
            iter: match self.nodes.get(&a) {
                Some(neigh) => neigh.iter(),
                None => [].iter(),
            },
            ty: self.ty,
        }
    }

    /// Return an iterator of all neighbors that have an edge between them and
    /// `a`, in the specified direction.
    /// If the graph's edges are undirected, this is equivalent to *.neighbors(a)*.
    ///
    /// - `Directed`, `Outgoing`: All edges from `a`.
    /// - `Directed`, `Incoming`: All edges to `a`.
    /// - `Undirected`: All edges from or to `a`.
    ///
    /// Produces an empty iterator if the node doesn't exist.<br>
    /// Iterator element type is `N`.
    pub fn neighbors_directed(&self, a: N, dir: Direction) -> NeighborsDirected<N, Ty> {
        NeighborsDirected {
            iter: match self.nodes.get(&a) {
                Some(neigh) => neigh.iter(),
                None => [].iter(),
            },
            start_node: a,
            dir,
            ty: self.ty,
        }
    }

    /// Return an iterator of target nodes with an edge starting from `a`,
    /// paired with their respective edge weights.
    ///
    /// - `Directed`: Outgoing edges from `a`.
    /// - `Undirected`: All edges from or to `a`.
    ///
    /// Produces an empty iterator if the node doesn't exist.<br>
    /// Iterator element type is `(N, &E)`.
    pub fn edges(&self, from: N) -> Edges<N, E, Ty> {
        Edges {
            from,
            iter: self.neighbors(from),
            edges: &self.edges,
        }
    }

    /// Return a reference to the edge weight connecting `a` with `b`, or
    /// `None` if the edge does not exist in the graph.
    pub fn edge_weight(&self, a: N, b: N) -> Option<&E> {
        self.edges.get(&Self::edge_key(a, b))
    }

    /// Return a mutable reference to the edge weight connecting `a` with `b`, or
    /// `None` if the edge does not exist in the graph.
    pub fn edge_weight_mut(&mut self, a: N, b: N) -> Option<&mut E> {
        self.edges.get_mut(&Self::edge_key(a, b))
    }

    /// Return an iterator over all edges of the graph with their weight in arbitrary order.
    ///
    /// Iterator element type is `(N, N, &E)`
    pub fn all_edges(&self) -> AllEdges<N, E, Ty> {
        AllEdges {
            inner: self.edges.iter(),
            ty: self.ty,
        }
    }

    /// Return an iterator over all edges of the graph in arbitrary order, with a mutable reference
    /// to their weight.
    ///
    /// Iterator element type is `(N, N, &mut E)`
    pub fn all_edges_mut(&mut self) -> AllEdgesMut<N, E, Ty> {
        AllEdgesMut {
            inner: self.edges.iter_mut(),
            ty: self.ty,
        }
    }

    /// Return a `Graph` that corresponds to this `GraphMap`.
    ///
    /// 1. Note that node and edge indices in the `Graph` have nothing in common
    ///    with the `GraphMap`s node weights `N`. The node weights `N` are used as
    ///    node weights in the resulting `Graph`, too.
    /// 2. Note that the index type is user-chosen.
    ///
    /// Computes in **O(|V| + |E|)** time (average).
    ///
    /// **Panics** if the number of nodes or edges does not fit with
    /// the resulting graph's index type.
    pub fn into_graph<Ix>(self) -> Graph<N, E, Ty, Ix>
    where
        Ix: crate::graph::IndexType,
    {
        // assuming two successive iterations of the same hashmap produce the same order
        let mut gr = Graph::with_capacity(self.node_count(), self.edge_count());
        for (&node, _) in &self.nodes {
            gr.add_node(node);
        }
        for ((a, b), edge_weight) in self.edges {
            let (ai, _, _) = self.nodes.get_full(&a).unwrap();
            let (bi, _, _) = self.nodes.get_full(&b).unwrap();
            gr.add_edge(node_index(ai), node_index(bi), edge_weight);
        }
        gr
    }
}

/// Create a new `GraphMap` from an iterable of edges.
impl<N, E, Ty, Item> FromIterator<Item> for GraphMap<N, E, Ty>
where
    Item: IntoWeightedEdge<E, NodeId = N>,
    N: NodeTrait,
    Ty: EdgeType,
{
    fn from_iter<I>(iterable: I) -> Self
    where
        I: IntoIterator<Item = Item>,
    {
        let iter = iterable.into_iter();
        let (low, _) = iter.size_hint();
        let mut g = Self::with_capacity(0, low);
        g.extend(iter);
        g
    }
}

/// Extend the graph from an iterable of edges.
///
/// Nodes are inserted automatically to match the edges.
impl<N, E, Ty, Item> Extend<Item> for GraphMap<N, E, Ty>
where
    Item: IntoWeightedEdge<E, NodeId = N>,
    N: NodeTrait,
    Ty: EdgeType,
{
    fn extend<I>(&mut self, iterable: I)
    where
        I: IntoIterator<Item = Item>,
    {
        let iter = iterable.into_iter();
        let (low, _) = iter.size_hint();
        self.edges.reserve(low);

        for elt in iter {
            let (source, target, weight) = elt.into_weighted_edge();
            self.add_edge(source, target, weight);
        }
    }
}

macro_rules! iterator_wrap {
    ($name: ident <$($typarm:tt),*> where { $($bounds: tt)* }
     item: $item: ty,
     iter: $iter: ty,
     ) => (
        pub struct $name <$($typarm),*> where $($bounds)* {
            iter: $iter,
        }
        impl<$($typarm),*> Iterator for $name <$($typarm),*>
            where $($bounds)*
        {
            type Item = $item;
            #[inline]
            fn next(&mut self) -> Option<Self::Item> {
                self.iter.next()
            }

            #[inline]
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.iter.size_hint()
            }
        }
    );
}

iterator_wrap! {
    Nodes <'a, N> where { N: 'a + NodeTrait }
    item: N,
    iter: Cloned<Keys<'a, N, Vec<(N, CompactDirection)>>>,
}

pub struct Neighbors<'a, N, Ty = Undirected>
where
    N: 'a,
    Ty: EdgeType,
{
    iter: Iter<'a, (N, CompactDirection)>,
    ty: PhantomData<Ty>,
}

impl<'a, N, Ty> Iterator for Neighbors<'a, N, Ty>
where
    N: NodeTrait,
    Ty: EdgeType,
{
    type Item = N;
    fn next(&mut self) -> Option<N> {
        if Ty::is_directed() {
            (&mut self.iter)
                .filter_map(|&(n, dir)| if dir == Outgoing { Some(n) } else { None })
                .next()
        } else {
            self.iter.next().map(|&(n, _)| n)
        }
    }
}

pub struct NeighborsDirected<'a, N, Ty>
where
    N: 'a,
    Ty: EdgeType,
{
    iter: Iter<'a, (N, CompactDirection)>,
    start_node: N,
    dir: Direction,
    ty: PhantomData<Ty>,
}

impl<'a, N, Ty> Iterator for NeighborsDirected<'a, N, Ty>
where
    N: NodeTrait,
    Ty: EdgeType,
{
    type Item = N;
    fn next(&mut self) -> Option<N> {
        if Ty::is_directed() {
            let self_dir = self.dir;
            let start_node = self.start_node;
            (&mut self.iter)
                .filter_map(move |&(n, dir)| {
                    if dir == self_dir || n == start_node {
                        Some(n)
                    } else {
                        None
                    }
                })
                .next()
        } else {
            self.iter.next().map(|&(n, _)| n)
        }
    }
}

pub struct Edges<'a, N, E: 'a, Ty>
where
    N: 'a + NodeTrait,
    Ty: EdgeType,
{
    from: N,
    edges: &'a IndexMap<(N, N), E>,
    iter: Neighbors<'a, N, Ty>,
}

impl<'a, N, E, Ty> Iterator for Edges<'a, N, E, Ty>
where
    N: 'a + NodeTrait,
    E: 'a,
    Ty: EdgeType,
{
    type Item = (N, N, &'a E);
    fn next(&mut self) -> Option<Self::Item> {
        match self.iter.next() {
            None => None,
            Some(b) => {
                let a = self.from;
                match self.edges.get(&GraphMap::<N, E, Ty>::edge_key(a, b)) {
                    None => unreachable!(),
                    Some(edge) => Some((a, b, edge)),
                }
            }
        }
    }
}

impl<'a, N: 'a, E: 'a, Ty> IntoEdgeReferences for &'a GraphMap<N, E, Ty>
where
    N: NodeTrait,
    Ty: EdgeType,
{
    type EdgeRef = (N, N, &'a E);
    type EdgeReferences = AllEdges<'a, N, E, Ty>;
    fn edge_references(self) -> Self::EdgeReferences {
        self.all_edges()
    }
}

pub struct AllEdges<'a, N, E: 'a, Ty>
where
    N: 'a + NodeTrait,
{
    inner: IndexMapIter<'a, (N, N), E>,
    ty: PhantomData<Ty>,
}

impl<'a, N, E, Ty> Iterator for AllEdges<'a, N, E, Ty>
where
    N: 'a + NodeTrait,
    E: 'a,
    Ty: EdgeType,
{
    type Item = (N, N, &'a E);
    fn next(&mut self) -> Option<Self::Item> {
        match self.inner.next() {
            None => None,
            Some((&(a, b), v)) => Some((a, b, v)),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }

    fn count(self) -> usize {
        self.inner.count()
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.inner
            .nth(n)
            .map(|(&(n1, n2), weight)| (n1, n2, weight))
    }

    fn last(self) -> Option<Self::Item> {
        self.inner
            .last()
            .map(|(&(n1, n2), weight)| (n1, n2, weight))
    }
}

impl<'a, N, E, Ty> DoubleEndedIterator for AllEdges<'a, N, E, Ty>
where
    N: 'a + NodeTrait,
    E: 'a,
    Ty: EdgeType,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner
            .next_back()
            .map(|(&(n1, n2), weight)| (n1, n2, weight))
    }
}

pub struct AllEdgesMut<'a, N, E: 'a, Ty>
where
    N: 'a + NodeTrait,
{
    inner: IndexMapIterMut<'a, (N, N), E>,
    ty: PhantomData<Ty>,
}

impl<'a, N, E, Ty> Iterator for AllEdgesMut<'a, N, E, Ty>
where
    N: 'a + NodeTrait,
    E: 'a,
    Ty: EdgeType,
{
    type Item = (N, N, &'a mut E);
    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|(&(n1, n2), weight)| (n1, n2, weight))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }

    fn count(self) -> usize {
        self.inner.count()
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.inner
            .nth(n)
            .map(|(&(n1, n2), weight)| (n1, n2, weight))
    }

    fn last(self) -> Option<Self::Item> {
        self.inner
            .last()
            .map(|(&(n1, n2), weight)| (n1, n2, weight))
    }
}

impl<'a, N, E, Ty> DoubleEndedIterator for AllEdgesMut<'a, N, E, Ty>
where
    N: 'a + NodeTrait,
    E: 'a,
    Ty: EdgeType,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner
            .next_back()
            .map(|(&(n1, n2), weight)| (n1, n2, weight))
    }
}

impl<'a, N: 'a, E: 'a, Ty> IntoEdges for &'a GraphMap<N, E, Ty>
where
    N: NodeTrait,
    Ty: EdgeType,
{
    type Edges = Edges<'a, N, E, Ty>;
    fn edges(self, a: Self::NodeId) -> Self::Edges {
        self.edges(a)
    }
}

/// Index `GraphMap` by node pairs to access edge weights.
impl<N, E, Ty> Index<(N, N)> for GraphMap<N, E, Ty>
where
    N: NodeTrait,
    Ty: EdgeType,
{
    type Output = E;
    fn index(&self, index: (N, N)) -> &E {
        let index = Self::edge_key(index.0, index.1);
        self.edge_weight(index.0, index.1)
            .expect("GraphMap::index: no such edge")
    }
}

/// Index `GraphMap` by node pairs to access edge weights.
impl<N, E, Ty> IndexMut<(N, N)> for GraphMap<N, E, Ty>
where
    N: NodeTrait,
    Ty: EdgeType,
{
    fn index_mut(&mut self, index: (N, N)) -> &mut E {
        let index = Self::edge_key(index.0, index.1);
        self.edge_weight_mut(index.0, index.1)
            .expect("GraphMap::index: no such edge")
    }
}

/// Create a new empty `GraphMap`.
impl<N, E, Ty> Default for GraphMap<N, E, Ty>
where
    N: NodeTrait,
    Ty: EdgeType,
{
    fn default() -> Self {
        GraphMap::with_capacity(0, 0)
    }
}

/// A reference that is hashed and compared by its pointer value.
///
/// `Ptr` is used for certain configurations of `GraphMap`,
/// in particular in the combination where the node type for
/// `GraphMap` is something of type for example `Ptr(&Cell<T>)`,
/// with the `Cell<T>` being `TypedArena` allocated.
pub struct Ptr<'b, T: 'b>(pub &'b T);

impl<'b, T> Copy for Ptr<'b, T> {}
impl<'b, T> Clone for Ptr<'b, T> {
    fn clone(&self) -> Self {
        *self
    }
}

fn ptr_eq<T>(a: *const T, b: *const T) -> bool {
    a == b
}

impl<'b, T> PartialEq for Ptr<'b, T> {
    /// Ptr compares by pointer equality, i.e if they point to the same value
    fn eq(&self, other: &Ptr<'b, T>) -> bool {
        ptr_eq(self.0, other.0)
    }
}

impl<'b, T> PartialOrd for Ptr<'b, T> {
    fn partial_cmp(&self, other: &Ptr<'b, T>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'b, T> Ord for Ptr<'b, T> {
    /// Ptr is ordered by pointer value, i.e. an arbitrary but stable and total order.
    fn cmp(&self, other: &Ptr<'b, T>) -> Ordering {
        let a: *const T = self.0;
        let b: *const T = other.0;
        a.cmp(&b)
    }
}

impl<'b, T> Deref for Ptr<'b, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.0
    }
}

impl<'b, T> Eq for Ptr<'b, T> {}

impl<'b, T> Hash for Ptr<'b, T> {
    fn hash<H: hash::Hasher>(&self, st: &mut H) {
        let ptr = (self.0) as *const T;
        ptr.hash(st)
    }
}

impl<'b, T: fmt::Debug> fmt::Debug for Ptr<'b, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<'a, N, E: 'a, Ty> IntoNodeIdentifiers for &'a GraphMap<N, E, Ty>
where
    N: NodeTrait,
    Ty: EdgeType,
{
    type NodeIdentifiers = NodeIdentifiers<'a, N, E, Ty>;

    fn node_identifiers(self) -> Self::NodeIdentifiers {
        NodeIdentifiers {
            iter: self.nodes.iter(),
            ty: self.ty,
            edge_ty: PhantomData,
        }
    }
}

impl<N, E, Ty> NodeCount for GraphMap<N, E, Ty>
where
    N: NodeTrait,
    Ty: EdgeType,
{
    fn node_count(&self) -> usize {
        (*self).node_count()
    }
}

pub struct NodeIdentifiers<'a, N, E: 'a, Ty>
where
    N: 'a + NodeTrait,
{
    iter: IndexMapIter<'a, N, Vec<(N, CompactDirection)>>,
    ty: PhantomData<Ty>,
    edge_ty: PhantomData<E>,
}

impl<'a, N, E, Ty> Iterator for NodeIdentifiers<'a, N, E, Ty>
where
    N: 'a + NodeTrait,
    E: 'a,
    Ty: EdgeType,
{
    type Item = N;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(&n, _)| n)
    }
}

impl<'a, N, E, Ty> IntoNodeReferences for &'a GraphMap<N, E, Ty>
where
    N: NodeTrait,
    Ty: EdgeType,
{
    type NodeRef = (N, &'a N);
    type NodeReferences = NodeReferences<'a, N, E, Ty>;
    fn node_references(self) -> Self::NodeReferences {
        NodeReferences {
            iter: self.nodes.iter(),
            ty: self.ty,
            edge_ty: PhantomData,
        }
    }
}

pub struct NodeReferences<'a, N, E: 'a, Ty>
where
    N: 'a + NodeTrait,
{
    iter: IndexMapIter<'a, N, Vec<(N, CompactDirection)>>,
    ty: PhantomData<Ty>,
    edge_ty: PhantomData<E>,
}

impl<'a, N, E, Ty> Iterator for NodeReferences<'a, N, E, Ty>
where
    N: 'a + NodeTrait,
    E: 'a,
    Ty: EdgeType,
{
    type Item = (N, &'a N);
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(n, _)| (*n, n))
    }
}

impl<N, E, Ty> NodeIndexable for GraphMap<N, E, Ty>
where
    N: NodeTrait,
    Ty: EdgeType,
{
    fn node_bound(&self) -> usize {
        self.node_count()
    }
    fn to_index(&self, ix: Self::NodeId) -> usize {
        let (i, _, _) = self.nodes.get_full(&ix).unwrap();
        i
    }
    fn from_index(&self, ix: usize) -> Self::NodeId {
        let (&key, _) = self.nodes.get_index(ix).unwrap();
        key
    }
}

impl<N, E, Ty> NodeCompactIndexable for GraphMap<N, E, Ty>
where
    N: NodeTrait,
    Ty: EdgeType,
{
}

#[cfg(test)]
mod tests_rug_307 {
    use super::*;

    #[test]
    fn test_rug() {
        let mut p0: *const i32 = &10 as *const i32;
        let mut p1: *const i32 = &20 as *const i32;

        assert_eq!(ptr_eq(p0, p1), false);
    }
}
#[cfg(test)]
mod tests_rug_311 {
    use super::*;
    use crate::graphmap::GraphMap;
    use crate::prelude::*;

    #[test]
    fn test_with_capacity() {
        let p0: usize = 10;
        let p1: usize = 5;

        GraphMap::<(), (), Directed>::with_capacity(p0, p1);
    }
}#[cfg(test)]
mod tests_rug_312 {
    use super::*;
    use crate::graphmap::GraphMap;
    use crate::prelude::*;

    #[test]
    fn test_rug() {
        let mut p0 = GraphMap::<i32, &'static str, Directed>::new();
        p0.add_node(1);
        p0.add_node(2);
        p0.add_edge(1, 2, "edge_data");
        
        assert_eq!(p0.capacity(), (2, 1));
    }
}#[cfg(test)]
mod tests_rug_314 {
    use super::*;
    use crate::prelude::*;

    #[test]
    fn test_is_directed() {
        let mut p0 = GraphMap::<i32, &str, Directed>::default();
        
        assert_eq!(p0.is_directed(), true);
    }
}#[cfg(test)]
mod tests_rug_315 {
    use super::*;
    use crate::graphmap::UnGraphMap;
    use crate::prelude::*;

    #[test]
    fn test_from_edges() {
        let mut p0 = &[
            (0, 1), (0, 2), (0, 3),
            (1, 2), (1, 3),
            (2, 3),
        ];

        let gr = UnGraphMap::<_, ()>::from_edges(p0);
    }
}#[cfg(test)]
mod tests_rug_316 {
    use super::*;
    use crate::{graphmap::GraphMap, data::Build, prelude::*};

    #[test]
    fn test_node_count() {
        let mut p0 = GraphMap::<i32, &str, Directed>::default();
        
        assert_eq!(p0.node_count(), 0);
    }
}#[cfg(test)]
mod tests_rug_317 {
    use super::*;
    use crate::prelude::*;

    #[test]
    fn test_rug() {
        let mut p0 = GraphMap::<u32, &str, Directed>::new();

        p0.add_node(1);
        p0.add_node(2);
        p0.add_edge(1, 2, "edge");

        assert_eq!(p0.edge_count(), 1);
    }
}#[cfg(test)]
mod tests_rug_318 {
    use super::*;
    use crate::graphmap::GraphMap;
    use crate::prelude::*;

    #[test]
    fn test_clear() {
        let mut p0 = GraphMap::<i32, &'static str, Directed>::with_capacity(10, 10);
        
        GraphMap::<i32, &'static str, Directed>::clear(&mut p0);
        
        assert_eq!(p0.node_count(), 0);
        assert_eq!(p0.edge_count(), 0);
    }
}#[cfg(test)]
mod tests_rug_326 {
    use super::*;
    use crate::prelude::*;

    #[test]
    fn test_nodes() {
        let mut graph = GraphMap::<i32, &str, Directed>::new();
        let node_a = graph.add_node(1);
        let node_b = graph.add_node(2);

        let result: Vec<_> = GraphMap::<i32, &str, Directed>::nodes(&graph).collect();
        
        assert_eq!(result.len(), 2);
        assert!(result.contains(&node_a));
        assert!(result.contains(&node_b));
    }
}#[cfg(test)]
mod tests_rug_327 {
    use super::*;

    use crate::prelude::*;

    #[test]
    fn test_neighbors() {
        let mut p0: GraphMap<(), (), Directed> = GraphMap::new();
        let mut p1 = ();

        p0.neighbors(p1);
    }
}use crate::prelude::*;
use crate::visit;

#[cfg(test)]
mod tests_rug_331 {
    use super::*;

    #[test]
    fn test_rug() {
        let mut p0 = GraphMap::<NodeIndex, i32, Directed>::default();
        
        let mut p1 = visit::Time::default();
        let mut p2 = visit::Time::default();

        p0.add_edge(NodeIndex::new(0), NodeIndex::new(1), 10);
        if let Some(weight) = p0.edge_weight_mut(NodeIndex::new(0), NodeIndex::new(1)) {
            *weight = 20;
        }
    }
}#[cfg(test)]
mod tests_rug_332 {
    use super::*;
    use crate::{
        prelude::*,
        graphmap::GraphMap,
    };

    #[test]
    fn test_rug() {
        let mut p0: GraphMap<NodeIndex, &str, Directed> = GraphMap::new();
        
        crate::graphmap::GraphMap::<NodeIndex, &str, Directed>::all_edges(&p0);
    }
}#[cfg(test)]
mod tests_rug_333 {
    use super::*;
    use crate::{graphmap::GraphMap, prelude::*};

    #[test]
    fn test_rug() {
        let mut p0 = GraphMap::<i32, &'static str, Directed>::new();

        p0.add_node(1);
        p0.add_node(2);
        p0.add_edge(1, 2, "edge1");

        GraphMap::<i32, &'static str, Directed>::all_edges_mut(&mut p0);
    }
}#[cfg(test)]
mod tests_rug_334 {
    use super::*;
    use crate::graph::node_index;
    use crate::prelude::*;
    use crate::Graph;
    
    #[test]
    fn test_rug() {
        let mut nodes = GraphMap::<&str, i32, Directed>::new();
        let _ = nodes.add_node("A");
        let _ = nodes.add_node("B");
        nodes.add_edge("A", "B", 100);

        nodes.into_graph::<usize>();
    }
}#[cfg(test)]
mod tests_rug_342 {
    use super::*;
    use crate::visit::IntoEdgeReferences;
    use crate::prelude::*;

    #[test]
    fn test_edge_references() {
        let mut p0 = GraphMap::<u32, &str, Directed>::default();

        <&'_ GraphMap<u32, &str, Directed> as visit::IntoEdgeReferences>::edge_references(&p0);
    }
}#[cfg(test)]
mod tests_rug_358 {
    use super::*;
    use crate::graphmap::GraphMap;
    use std::default::Default;

    #[test]
    fn test_default() {
        let graph_default: GraphMap<i32, &str, Directed> = <GraphMap<i32, &str, Directed> as Default>::default();
        // Add assertions or further test logic as needed
    }
}#[cfg(test)]
mod tests_rug_368 {
    use super::*;
    use crate::prelude::*;
    use crate::visit::IntoNodeReferences;
    
    #[test]
    fn test_node_references() {
        let mut p0 = GraphMap::<i32, &str, Directed>::new();
        let node_ref = p0.node_references();
        
        // Add assertions here
    }
}#[cfg(test)]
mod tests_rug_370 {
    use super::*;
    use crate::visit::NodeIndexable;
    use crate::prelude::*;

    #[test]
    fn test_node_bound() {
        let mut p0 = GraphMap::<u32, &'static str, Directed>::new();

        <GraphMap<u32, &'static str, Directed> as NodeIndexable>::node_bound(&p0);
    }
}