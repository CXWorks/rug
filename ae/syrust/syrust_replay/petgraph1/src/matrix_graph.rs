//! `MatrixGraph<N, E, Ty, NullN, NullE, Ix>` is a graph datastructure backed by an adjacency matrix.
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};
use std::cmp;
use std::mem;
use indexmap::IndexSet;
use fixedbitset::FixedBitSet;
use crate::{Directed, Direction, EdgeType, IntoWeightedEdge, Outgoing, Undirected};
use crate::graph::NodeIndex as GraphNodeIndex;
use crate::visit::{
    Data, GetAdjacencyMatrix, GraphBase, GraphProp, IntoEdgeReferences, IntoEdges,
    IntoNeighbors, IntoNeighborsDirected, IntoNodeIdentifiers, IntoNodeReferences,
    NodeCompactIndexable, NodeCount, NodeIndexable, Visitable,
};
use crate::data::Build;
pub use crate::graph::IndexType;
type DefaultIx = u16;
/// Node identifier.
pub type NodeIndex<Ix = DefaultIx> = GraphNodeIndex<Ix>;
mod private {
    pub trait Sealed {}
    impl<T> Sealed for super::NotZero<T> {}
    impl<T> Sealed for Option<T> {}
}
/// Wrapper trait for an `Option`, allowing user-defined structs to be input as containers when
/// defining a null element.
///
/// Note: this trait is currently *sealed* and cannot be implemented for types outside this crate.
pub trait Nullable: Default + Into<
        Option<<Self as Nullable>::Wrapped>,
    > + private::Sealed {
    #[doc(hidden)]
    type Wrapped;
    #[doc(hidden)]
    fn new(value: Self::Wrapped) -> Self;
    #[doc(hidden)]
    fn as_ref(&self) -> Option<&Self::Wrapped>;
    #[doc(hidden)]
    fn as_mut(&mut self) -> Option<&mut Self::Wrapped>;
    #[doc(hidden)]
    fn is_null(&self) -> bool {
        self.as_ref().is_none()
    }
}
impl<T> Nullable for Option<T> {
    type Wrapped = T;
    fn new(value: T) -> Self {
        Some(value)
    }
    fn as_ref(&self) -> Option<&Self::Wrapped> {
        self.as_ref()
    }
    fn as_mut(&mut self) -> Option<&mut Self::Wrapped> {
        self.as_mut()
    }
}
/// `NotZero` is used to optimize the memory usage of edge weights `E` in a
/// [`MatrixGraph`](struct.MatrixGraph.html), replacing the default `Option<E>` sentinel.
///
/// Pre-requisite: edge weight should implement [`Zero`](trait.Zero.html).
///
/// Note that if you're already using the standard non-zero types (such as `NonZeroU32`), you don't
/// have to use this wrapper and can leave the default `Null` type argument.
pub struct NotZero<T>(T);
impl<T: Zero> Default for NotZero<T> {
    fn default() -> Self {
        NotZero(T::zero())
    }
}
impl<T: Zero> Nullable for NotZero<T> {
    type Wrapped = T;
    fn new(value: T) -> Self {
        assert!(! value.is_zero());
        NotZero(value)
    }
    fn is_null(&self) -> bool {
        self.0.is_zero()
    }
    fn as_ref(&self) -> Option<&Self::Wrapped> {
        if !self.is_null() { Some(&self.0) } else { None }
    }
    fn as_mut(&mut self) -> Option<&mut Self::Wrapped> {
        if !self.is_null() { Some(&mut self.0) } else { None }
    }
}
impl<T: Zero> Into<Option<T>> for NotZero<T> {
    fn into(self) -> Option<T> {
        if !self.is_null() { Some(self.0) } else { None }
    }
}
/// Base trait for types that can be wrapped in a [`NotZero`](struct.NotZero.html).
///
/// Implementors must provide a singleton object that will be used to mark empty edges in a
/// [`MatrixGraph`](struct.MatrixGraph.html).
///
/// Note that this trait is already implemented for the base numeric types.
pub trait Zero {
    /// Return the singleton object which can be used as a sentinel value.
    fn zero() -> Self;
    /// Return true if `self` is equal to the sentinel value.
    fn is_zero(&self) -> bool;
}
macro_rules! not_zero_impl {
    ($t:ty,$z:expr) => {
        impl Zero for $t { fn zero() -> Self { $z as $t } fn is_zero(& self) -> bool {
        self == & Self::zero() } }
    };
}
macro_rules! not_zero_impls {
    ($($t:ty),*) => {
        $(not_zero_impl!($t, 0);)*
    };
}
not_zero_impls!(u8, u16, u32, u64, usize);
not_zero_impls!(i8, i16, i32, i64, isize);
not_zero_impls!(f32, f64);
/// Short version of `NodeIndex::new` (with Ix = `DefaultIx`)
#[inline]
pub fn node_index(ax: usize) -> NodeIndex {
    NodeIndex::new(ax)
}
/// `MatrixGraph<N, E, Ty, Null>` is a graph datastructure using an adjacency matrix
/// representation.
///
/// `MatrixGraph` is parameterized over:
///
/// - Associated data `N` for nodes and `E` for edges, called *weights*.
///   The associated data can be of arbitrary type.
/// - Edge type `Ty` that determines whether the graph edges are directed or undirected.
/// - Nullable type `Null`, which denotes the edges' presence (defaults to `Option<E>`). You may
///   specify [`NotZero<E>`](struct.NotZero.html) if you want to use a sentinel value (such as 0)
///   to mark the absence of an edge.
/// - Index type `Ix` that sets the maximum size for the graph (defaults to `DefaultIx`).
///
/// The graph uses **O(|V^2|)** space, with fast edge insertion & amortized node insertion, as well
/// as efficient graph search and graph algorithms on dense graphs.
///
/// This graph is backed by a flattened 2D array. For undirected graphs, only the lower triangular
/// matrix is stored. Since the backing array stores edge weights, it is recommended to box large
/// edge weights.
#[derive(Clone)]
pub struct MatrixGraph<
    N,
    E,
    Ty = Directed,
    Null: Nullable<Wrapped = E> = Option<E>,
    Ix = DefaultIx,
> {
    node_adjacencies: Vec<Null>,
    node_capacity: usize,
    nodes: IdStorage<N>,
    nb_edges: usize,
    ty: PhantomData<Ty>,
    ix: PhantomData<Ix>,
}
/// A `MatrixGraph` with directed edges.
pub type DiMatrix<N, E, Null = Option<E>, Ix = DefaultIx> = MatrixGraph<
    N,
    E,
    Directed,
    Null,
    Ix,
>;
/// A `MatrixGraph` with undirected edges.
pub type UnMatrix<N, E, Null = Option<E>, Ix = DefaultIx> = MatrixGraph<
    N,
    E,
    Undirected,
    Null,
    Ix,
>;
impl<
    N,
    E,
    Ty: EdgeType,
    Null: Nullable<Wrapped = E>,
    Ix: IndexType,
> MatrixGraph<N, E, Ty, Null, Ix> {
    /// Create a new `MatrixGraph` with estimated capacity for nodes.
    pub fn with_capacity(node_capacity: usize) -> Self {
        let mut m = Self {
            node_adjacencies: vec![],
            node_capacity: 0,
            nodes: IdStorage::with_capacity(node_capacity),
            nb_edges: 0,
            ty: PhantomData,
            ix: PhantomData,
        };
        debug_assert!(node_capacity <= < Ix as IndexType >::max().index());
        m.extend_capacity_for_node(NodeIndex::new(node_capacity));
        m
    }
    #[inline]
    fn to_edge_position(&self, a: NodeIndex<Ix>, b: NodeIndex<Ix>) -> usize {
        to_linearized_matrix_position::<Ty>(a.index(), b.index(), self.node_capacity)
    }
    /// Remove all nodes and edges.
    pub fn clear(&mut self) {
        for edge in self.node_adjacencies.iter_mut() {
            *edge = Default::default();
        }
        self.nodes.clear();
        self.nb_edges = 0;
    }
    /// Return the number of nodes (vertices) in the graph.
    ///
    /// Computes in **O(1)** time.
    #[inline]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
    /// Return the number of edges in the graph.
    ///
    /// Computes in **O(1)** time.
    #[inline]
    pub fn edge_count(&self) -> usize {
        self.nb_edges
    }
    /// Return whether the graph has directed edges or not.
    #[inline]
    pub fn is_directed(&self) -> bool {
        Ty::is_directed()
    }
    /// Add a node (also called vertex) with associated data `weight` to the graph.
    ///
    /// Computes in **O(1)** time.
    ///
    /// Return the index of the new node.
    ///
    /// **Panics** if the MatrixGraph is at the maximum number of nodes for its index type.
    pub fn add_node(&mut self, weight: N) -> NodeIndex<Ix> {
        NodeIndex::new(self.nodes.add(weight))
    }
    /// Remove `a` from the graph.
    ///
    /// Computes in **O(V)** time, due to the removal of edges with other nodes.
    ///
    /// **Panics** if the node `a` does not exist.
    pub fn remove_node(&mut self, a: NodeIndex<Ix>) -> N {
        for id in self.nodes.iter_ids() {
            let position = self.to_edge_position(a, NodeIndex::new(id));
            self.node_adjacencies[position] = Default::default();
            if Ty::is_directed() {
                let position = self.to_edge_position(NodeIndex::new(id), a);
                self.node_adjacencies[position] = Default::default();
            }
        }
        self.nodes.remove(a.index())
    }
    #[inline]
    fn extend_capacity_for_node(&mut self, min_node: NodeIndex<Ix>) {
        self
            .node_capacity = extend_linearized_matrix::<
            Ty,
            _,
        >(&mut self.node_adjacencies, self.node_capacity, min_node.index());
    }
    #[inline]
    fn extend_capacity_for_edge(&mut self, a: NodeIndex<Ix>, b: NodeIndex<Ix>) {
        let min_node = cmp::max(a, b);
        if min_node.index() >= self.node_capacity {
            self.extend_capacity_for_node(min_node);
        }
    }
    /// Update the edge from `a` to `b` to the graph, with its associated data `weight`.
    ///
    /// Return the previous data, if any.
    ///
    /// Computes in **O(1)** time, best case.
    /// Computes in **O(|V|^2)** time, worst case (matrix needs to be re-allocated).
    ///
    /// **Panics** if any of the nodes don't exist.
    pub fn update_edge(
        &mut self,
        a: NodeIndex<Ix>,
        b: NodeIndex<Ix>,
        weight: E,
    ) -> Option<E> {
        self.extend_capacity_for_edge(a, b);
        let p = self.to_edge_position(a, b);
        let old_weight = mem::replace(&mut self.node_adjacencies[p], Null::new(weight));
        if old_weight.is_null() {
            self.nb_edges += 1;
        }
        old_weight.into()
    }
    /// Add an edge from `a` to `b` to the graph, with its associated
    /// data `weight`.
    ///
    /// Return the index of the new edge.
    ///
    /// Computes in **O(1)** time, best case.
    /// Computes in **O(|V|^2)** time, worst case (matrix needs to be re-allocated).
    ///
    /// **Panics** if any of the nodes don't exist.
    /// **Panics** if an edge already exists from `a` to `b`.
    ///
    /// **Note:** `MatrixGraph` does not allow adding parallel (“duplicate”) edges. If you want to avoid
    /// this, use [`.update_edge(a, b, weight)`](#method.update_edge) instead.
    pub fn add_edge(&mut self, a: NodeIndex<Ix>, b: NodeIndex<Ix>, weight: E) {
        let old_edge_id = self.update_edge(a, b, weight);
        assert!(old_edge_id.is_none());
    }
    /// Remove the edge from `a` to `b` to the graph.
    ///
    /// **Panics** if any of the nodes don't exist.
    /// **Panics** if no edge exists between `a` and `b`.
    pub fn remove_edge(&mut self, a: NodeIndex<Ix>, b: NodeIndex<Ix>) -> E {
        let p = self.to_edge_position(a, b);
        let old_weight = mem::replace(&mut self.node_adjacencies[p], Default::default())
            .into()
            .unwrap();
        let old_weight: Option<_> = old_weight.into();
        self.nb_edges -= 1;
        old_weight.unwrap()
    }
    /// Return true if there is an edge between `a` and `b`.
    ///
    /// **Panics** if any of the nodes don't exist.
    pub fn has_edge(&self, a: NodeIndex<Ix>, b: NodeIndex<Ix>) -> bool {
        let p = self.to_edge_position(a, b);
        !self.node_adjacencies[p].is_null()
    }
    /// Access the weight for node `a`.
    ///
    /// Also available with indexing syntax: `&graph[a]`.
    ///
    /// **Panics** if the node doesn't exist.
    pub fn node_weight(&self, a: NodeIndex<Ix>) -> &N {
        &self.nodes[a.index()]
    }
    /// Access the weight for node `a`, mutably.
    ///
    /// Also available with indexing syntax: `&mut graph[a]`.
    ///
    /// **Panics** if the node doesn't exist.
    pub fn node_weight_mut(&mut self, a: NodeIndex<Ix>) -> &mut N {
        &mut self.nodes[a.index()]
    }
    /// Access the weight for edge `e`.
    ///
    /// Also available with indexing syntax: `&graph[e]`.
    ///
    /// **Panics** if no edge exists between `a` and `b`.
    pub fn edge_weight(&self, a: NodeIndex<Ix>, b: NodeIndex<Ix>) -> &E {
        let p = self.to_edge_position(a, b);
        self.node_adjacencies[p].as_ref().unwrap()
    }
    /// Access the weight for edge `e`, mutably.
    ///
    /// Also available with indexing syntax: `&mut graph[e]`.
    ///
    /// **Panics** if no edge exists between `a` and `b`.
    pub fn edge_weight_mut(&mut self, a: NodeIndex<Ix>, b: NodeIndex<Ix>) -> &mut E {
        let p = self.to_edge_position(a, b);
        self.node_adjacencies[p].as_mut().unwrap()
    }
    /// Return an iterator of all nodes with an edge starting from `a`.
    ///
    /// - `Directed`: Outgoing edges from `a`.
    /// - `Undirected`: All edges from or to `a`.
    ///
    /// Produces an empty iterator if the node doesn't exist.<br>
    /// Iterator element type is [`NodeIndex<Ix>`](../graph/struct.NodeIndex.html).
    pub fn neighbors(&self, a: NodeIndex<Ix>) -> Neighbors<Ty, Null, Ix> {
        Neighbors(
            Edges::on_columns(a.index(), &self.node_adjacencies, self.node_capacity),
        )
    }
    /// Return an iterator of all edges of `a`.
    ///
    /// - `Directed`: Outgoing edges from `a`.
    /// - `Undirected`: All edges connected to `a`.
    ///
    /// Produces an empty iterator if the node doesn't exist.<br>
    /// Iterator element type is [`Edges<E, Ix>`](../graph/struct.Edges.html).
    pub fn edges(&self, a: NodeIndex<Ix>) -> Edges<Ty, Null, Ix> {
        Edges::on_columns(a.index(), &self.node_adjacencies, self.node_capacity)
    }
    /// Create a new `MatrixGraph` from an iterable of edges.
    ///
    /// Node weights `N` are set to default values.
    /// Edge weights `E` may either be specified in the list,
    /// or they are filled with default values.
    ///
    /// Nodes are inserted automatically to match the edges.
    ///
    /// ```
    /// use petgraph::matrix_graph::MatrixGraph;
    ///
    /// let gr = MatrixGraph::<(), i32>::from_edges(&[
    ///     (0, 1), (0, 2), (0, 3),
    ///     (1, 2), (1, 3),
    ///     (2, 3),
    /// ]);
    /// ```
    pub fn from_edges<I>(iterable: I) -> Self
    where
        I: IntoIterator,
        I::Item: IntoWeightedEdge<E>,
        <I::Item as IntoWeightedEdge<E>>::NodeId: Into<NodeIndex<Ix>>,
        N: Default,
    {
        let mut g = Self::default();
        g.extend_with_edges(iterable);
        g
    }
    /// Extend the graph from an iterable of edges.
    ///
    /// Node weights `N` are set to default values.
    /// Edge weights `E` may either be specified in the list,
    /// or they are filled with default values.
    ///
    /// Nodes are inserted automatically to match the edges.
    pub fn extend_with_edges<I>(&mut self, iterable: I)
    where
        I: IntoIterator,
        I::Item: IntoWeightedEdge<E>,
        <I::Item as IntoWeightedEdge<E>>::NodeId: Into<NodeIndex<Ix>>,
        N: Default,
    {
        for elt in iterable {
            let (source, target, weight) = elt.into_weighted_edge();
            let (source, target) = (source.into(), target.into());
            let nx = cmp::max(source, target);
            while nx.index() >= self.node_count() {
                self.add_node(N::default());
            }
            self.add_edge(source, target, weight);
        }
    }
}
impl<
    N,
    E,
    Null: Nullable<Wrapped = E>,
    Ix: IndexType,
> MatrixGraph<N, E, Directed, Null, Ix> {
    /// Return an iterator of all neighbors that have an edge between them and
    /// `a`, in the specified direction.
    /// If the graph's edges are undirected, this is equivalent to *.neighbors(a)*.
    ///
    /// - `Outgoing`: All edges from `a`.
    /// - `Incoming`: All edges to `a`.
    ///
    /// Produces an empty iterator if the node doesn't exist.<br>
    /// Iterator element type is [`NodeIndex<Ix>`](../graph/struct.NodeIndex.html).
    pub fn neighbors_directed(
        &self,
        a: NodeIndex<Ix>,
        d: Direction,
    ) -> Neighbors<Directed, Null, Ix> {
        if d == Outgoing {
            self.neighbors(a)
        } else {
            Neighbors(
                Edges::on_rows(a.index(), &self.node_adjacencies, self.node_capacity),
            )
        }
    }
    /// Return an iterator of all edges of `a`, in the specified direction.
    ///
    /// - `Outgoing`: All edges from `a`.
    /// - `Incoming`: All edges to `a`.
    ///
    /// Produces an empty iterator if the node `a` doesn't exist.<br>
    /// Iterator element type is [`EdgeReference<E, Ix>`](../graph/struct.EdgeReference.html).
    pub fn edges_directed(
        &self,
        a: NodeIndex<Ix>,
        d: Direction,
    ) -> Edges<Directed, Null, Ix> {
        if d == Outgoing {
            self.edges(a)
        } else {
            Edges::on_rows(a.index(), &self.node_adjacencies, self.node_capacity)
        }
    }
}
/// Iterator over the node identifiers of a graph.
///
/// Created from a call to [`.node_identifiers()`][1] on a [`MatrixGraph`][2].
///
/// [1]: ../visit/trait.IntoNodeIdentifiers.html#tymethod.node_identifiers
/// [2]: struct.MatrixGraph.html
pub struct NodeIdentifiers<'a, Ix> {
    iter: IdIterator<'a>,
    ix: PhantomData<Ix>,
}
impl<'a, Ix: IndexType> NodeIdentifiers<'a, Ix> {
    fn new(iter: IdIterator<'a>) -> Self {
        Self { iter, ix: PhantomData }
    }
}
impl<'a, Ix: IndexType> Iterator for NodeIdentifiers<'a, Ix> {
    type Item = NodeIndex<Ix>;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(NodeIndex::new)
    }
}
/// Iterator over all nodes of a graph.
///
/// Created from a call to [`.node_references()`][1] on a [`MatrixGraph`][2].
///
/// [1]: ../visit/trait.IntoNodeReferences.html#tymethod.node_references
/// [2]: struct.MatrixGraph.html
pub struct NodeReferences<'a, N: 'a, Ix> {
    nodes: &'a IdStorage<N>,
    iter: IdIterator<'a>,
    ix: PhantomData<Ix>,
}
impl<'a, N: 'a, Ix> NodeReferences<'a, N, Ix> {
    fn new(nodes: &'a IdStorage<N>) -> Self {
        NodeReferences {
            nodes,
            iter: nodes.iter_ids(),
            ix: PhantomData,
        }
    }
}
impl<'a, N: 'a, Ix: IndexType> Iterator for NodeReferences<'a, N, Ix> {
    type Item = (NodeIndex<Ix>, &'a N);
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|i| (NodeIndex::new(i), &self.nodes[i]))
    }
}
/// Iterator over all edges of a graph.
///
/// Created from a call to [`.edge_references()`][1] on a [`MatrixGraph`][2].
///
/// [1]: ../visit/trait.IntoEdgeReferences.html#tymethod.edge_references
/// [2]: struct.MatrixGraph.html
pub struct EdgeReferences<'a, Ty: EdgeType, Null: 'a + Nullable, Ix> {
    row: usize,
    column: usize,
    node_adjacencies: &'a [Null],
    node_capacity: usize,
    ty: PhantomData<Ty>,
    ix: PhantomData<Ix>,
}
impl<'a, Ty: EdgeType, Null: 'a + Nullable, Ix> EdgeReferences<'a, Ty, Null, Ix> {
    fn new(node_adjacencies: &'a [Null], node_capacity: usize) -> Self {
        EdgeReferences {
            row: 0,
            column: 0,
            node_adjacencies,
            node_capacity,
            ty: PhantomData,
            ix: PhantomData,
        }
    }
}
impl<'a, Ty: EdgeType, Null: Nullable, Ix: IndexType> Iterator
for EdgeReferences<'a, Ty, Null, Ix> {
    type Item = (NodeIndex<Ix>, NodeIndex<Ix>, &'a Null::Wrapped);
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (row, column) = (self.row, self.column);
            if row >= self.node_capacity {
                return None;
            }
            self.column += 1;
            let max_column_len = if !Ty::is_directed() {
                row + 1
            } else {
                self.node_capacity
            };
            if self.column >= max_column_len {
                self.column = 0;
                self.row += 1;
            }
            let p = to_linearized_matrix_position::<Ty>(row, column, self.node_capacity);
            if let Some(e) = self.node_adjacencies[p].as_ref() {
                return Some((NodeIndex::new(row), NodeIndex::new(column), e));
            }
        }
    }
}
/// Iterator over the neighbors of a node.
///
/// Iterator element type is `NodeIndex<Ix>`.
///
/// Created with [`.neighbors()`][1], [`.neighbors_directed()`][2].
///
/// [1]: struct.MatrixGraph.html#method.neighbors
/// [2]: struct.MatrixGraph.html#method.neighbors_directed
pub struct Neighbors<'a, Ty: EdgeType, Null: 'a + Nullable, Ix>(Edges<'a, Ty, Null, Ix>);
impl<'a, Ty: EdgeType, Null: Nullable, Ix: IndexType> Iterator
for Neighbors<'a, Ty, Null, Ix> {
    type Item = NodeIndex<Ix>;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(_, b, _)| b)
    }
}
enum NeighborIterDirection {
    Rows,
    Columns,
}
/// Iterator over the edges of from or to a node
///
/// Created with [`.edges()`][1], [`.edges_directed()`][2].
///
/// [1]: struct.MatrixGraph.html#method.edges
/// [2]: struct.MatrixGraph.html#method.edges_directed
pub struct Edges<'a, Ty: EdgeType, Null: 'a + Nullable, Ix> {
    iter_direction: NeighborIterDirection,
    node_adjacencies: &'a [Null],
    node_capacity: usize,
    row: usize,
    column: usize,
    ty: PhantomData<Ty>,
    ix: PhantomData<Ix>,
}
impl<'a, Ty: EdgeType, Null: 'a + Nullable, Ix> Edges<'a, Ty, Null, Ix> {
    fn on_columns(
        row: usize,
        node_adjacencies: &'a [Null],
        node_capacity: usize,
    ) -> Self {
        Edges {
            iter_direction: NeighborIterDirection::Columns,
            node_adjacencies,
            node_capacity,
            row,
            column: 0,
            ty: PhantomData,
            ix: PhantomData,
        }
    }
    fn on_rows(
        column: usize,
        node_adjacencies: &'a [Null],
        node_capacity: usize,
    ) -> Self {
        Edges {
            iter_direction: NeighborIterDirection::Rows,
            node_adjacencies,
            node_capacity,
            row: 0,
            column,
            ty: PhantomData,
            ix: PhantomData,
        }
    }
}
impl<'a, Ty: EdgeType, Null: Nullable, Ix: IndexType> Iterator
for Edges<'a, Ty, Null, Ix> {
    type Item = (NodeIndex<Ix>, NodeIndex<Ix>, &'a Null::Wrapped);
    fn next(&mut self) -> Option<Self::Item> {
        use self::NeighborIterDirection::*;
        loop {
            let (row, column) = (self.row, self.column);
            if row >= self.node_capacity || column >= self.node_capacity {
                return None;
            }
            match self.iter_direction {
                Rows => self.row += 1,
                Columns => self.column += 1,
            }
            let p = to_linearized_matrix_position::<Ty>(row, column, self.node_capacity);
            if let Some(e) = self.node_adjacencies[p].as_ref() {
                let (a, b) = match self.iter_direction {
                    Rows => (column, row),
                    Columns => (row, column),
                };
                return Some((NodeIndex::new(a), NodeIndex::new(b), e));
            }
        }
    }
}
#[inline]
fn to_linearized_matrix_position<Ty: EdgeType>(
    row: usize,
    column: usize,
    width: usize,
) -> usize {
    if Ty::is_directed() {
        to_flat_square_matrix_position(row, column, width)
    } else {
        to_lower_triangular_matrix_position(row, column)
    }
}
#[inline]
fn extend_linearized_matrix<Ty: EdgeType, T: Default>(
    node_adjacencies: &mut Vec<T>,
    old_node_capacity: usize,
    min_node_capacity: usize,
) -> usize {
    if Ty::is_directed() {
        extend_flat_square_matrix(node_adjacencies, old_node_capacity, min_node_capacity)
    } else {
        extend_lower_triangular_matrix(node_adjacencies, min_node_capacity)
    }
}
#[inline]
fn to_flat_square_matrix_position(row: usize, column: usize, width: usize) -> usize {
    row * width + column
}
#[inline]
fn extend_flat_square_matrix<T: Default>(
    node_adjacencies: &mut Vec<T>,
    old_node_capacity: usize,
    min_node_capacity: usize,
) -> usize {
    let min_node_capacity = (min_node_capacity + 1).next_power_of_two();
    const MIN_CAPACITY: usize = 4;
    let new_node_capacity = cmp::max(min_node_capacity, MIN_CAPACITY);
    let mut new_node_adjacencies = vec![];
    ensure_len(&mut new_node_adjacencies, new_node_capacity.pow(2));
    for c in 0..old_node_capacity {
        let pos = c * old_node_capacity;
        let new_pos = c * new_node_capacity;
        let mut old = &mut node_adjacencies[pos..pos + old_node_capacity];
        let mut new = &mut new_node_adjacencies[new_pos..new_pos + old_node_capacity];
        mem::swap(&mut old, &mut new);
    }
    mem::swap(node_adjacencies, &mut new_node_adjacencies);
    new_node_capacity
}
#[inline]
fn to_lower_triangular_matrix_position(row: usize, column: usize) -> usize {
    let (row, column) = if row > column { (row, column) } else { (column, row) };
    (row * (row + 1)) / 2 + column
}
#[inline]
fn extend_lower_triangular_matrix<T: Default>(
    node_adjacencies: &mut Vec<T>,
    new_node_capacity: usize,
) -> usize {
    let max_pos = to_lower_triangular_matrix_position(
        new_node_capacity,
        new_node_capacity,
    );
    ensure_len(node_adjacencies, max_pos + 1);
    new_node_capacity + 1
}
/// Grow a Vec by appending the type's default value until the `size` is reached.
fn ensure_len<T: Default>(v: &mut Vec<T>, size: usize) {
    if let Some(n) = size.checked_sub(v.len()) {
        v.reserve(n);
        for _ in 0..n {
            v.push(T::default());
        }
    }
}
#[derive(Clone)]
struct IdStorage<T> {
    elements: Vec<Option<T>>,
    upper_bound: usize,
    removed_ids: IndexSet<usize>,
}
impl<T> IdStorage<T> {
    fn with_capacity(capacity: usize) -> Self {
        IdStorage {
            elements: Vec::with_capacity(capacity),
            upper_bound: 0,
            removed_ids: IndexSet::new(),
        }
    }
    fn add(&mut self, element: T) -> usize {
        let id = if let Some(id) = self.removed_ids.pop() {
            id
        } else {
            let id = self.upper_bound;
            self.upper_bound += 1;
            ensure_len(&mut self.elements, id + 1);
            id
        };
        self.elements[id] = Some(element);
        id
    }
    fn remove(&mut self, id: usize) -> T {
        let data = self.elements[id].take().unwrap();
        if self.upper_bound - id == 1 {
            self.upper_bound -= 1;
        } else {
            self.removed_ids.insert(id);
        }
        data
    }
    fn clear(&mut self) {
        self.upper_bound = 0;
        self.elements.clear();
        self.removed_ids.clear();
    }
    #[inline]
    fn len(&self) -> usize {
        self.upper_bound - self.removed_ids.len()
    }
    fn iter_ids(&self) -> IdIterator {
        IdIterator {
            upper_bound: self.upper_bound,
            removed_ids: &self.removed_ids,
            current: None,
        }
    }
}
impl<T> Index<usize> for IdStorage<T> {
    type Output = T;
    fn index(&self, index: usize) -> &T {
        self.elements[index].as_ref().unwrap()
    }
}
impl<T> IndexMut<usize> for IdStorage<T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        self.elements[index].as_mut().unwrap()
    }
}
struct IdIterator<'a> {
    upper_bound: usize,
    removed_ids: &'a IndexSet<usize>,
    current: Option<usize>,
}
impl<'a> Iterator for IdIterator<'a> {
    type Item = usize;
    fn next(&mut self) -> Option<Self::Item> {
        let current = {
            if self.current.is_none() {
                self.current = Some(0);
                self.current.as_mut().unwrap()
            } else {
                let current = self.current.as_mut().unwrap();
                *current += 1;
                current
            }
        };
        while self.removed_ids.contains(current) && *current < self.upper_bound {
            *current += 1;
        }
        if *current < self.upper_bound { Some(*current) } else { None }
    }
}
/// Create a new empty `MatrixGraph`.
impl<N, E, Ty: EdgeType, Null: Nullable<Wrapped = E>, Ix: IndexType> Default
for MatrixGraph<N, E, Ty, Null, Ix> {
    fn default() -> Self {
        Self::with_capacity(0)
    }
}
impl<N, E> MatrixGraph<N, E, Directed> {
    /// Create a new `MatrixGraph` with directed edges.
    ///
    /// This is a convenience method. Use `MatrixGraph::with_capacity` or `MatrixGraph::default` for
    /// a constructor that is generic in all the type parameters of `MatrixGraph`.
    pub fn new() -> Self {
        MatrixGraph::default()
    }
}
impl<N, E> MatrixGraph<N, E, Undirected> {
    /// Create a new `MatrixGraph` with undirected edges.
    ///
    /// This is a convenience method. Use `MatrixGraph::with_capacity` or `MatrixGraph::default` for
    /// a constructor that is generic in all the type parameters of `MatrixGraph`.
    pub fn new_undirected() -> Self {
        MatrixGraph::default()
    }
}
/// Index the `MatrixGraph` by `NodeIndex` to access node weights.
///
/// **Panics** if the node doesn't exist.
impl<N, E, Ty: EdgeType, Null: Nullable<Wrapped = E>, Ix: IndexType> Index<NodeIndex<Ix>>
for MatrixGraph<N, E, Ty, Null, Ix> {
    type Output = N;
    fn index(&self, ax: NodeIndex<Ix>) -> &N {
        self.node_weight(ax)
    }
}
/// Index the `MatrixGraph` by `NodeIndex` to access node weights.
///
/// **Panics** if the node doesn't exist.
impl<
    N,
    E,
    Ty: EdgeType,
    Null: Nullable<Wrapped = E>,
    Ix: IndexType,
> IndexMut<NodeIndex<Ix>> for MatrixGraph<N, E, Ty, Null, Ix> {
    fn index_mut(&mut self, ax: NodeIndex<Ix>) -> &mut N {
        self.node_weight_mut(ax)
    }
}
impl<N, E, Ty: EdgeType, Null: Nullable<Wrapped = E>, Ix: IndexType> NodeCount
for MatrixGraph<N, E, Ty, Null, Ix> {
    fn node_count(&self) -> usize {
        MatrixGraph::node_count(self)
    }
}
/// Index the `MatrixGraph` by `NodeIndex` pair to access edge weights.
///
/// Also available with indexing syntax: `&graph[e]`.
///
/// **Panics** if no edge exists between `a` and `b`.
impl<
    N,
    E,
    Ty: EdgeType,
    Null: Nullable<Wrapped = E>,
    Ix: IndexType,
> Index<(NodeIndex<Ix>, NodeIndex<Ix>)> for MatrixGraph<N, E, Ty, Null, Ix> {
    type Output = E;
    fn index(&self, (ax, bx): (NodeIndex<Ix>, NodeIndex<Ix>)) -> &E {
        self.edge_weight(ax, bx)
    }
}
/// Index the `MatrixGraph` by `NodeIndex` pair to access edge weights.
///
/// Also available with indexing syntax: `&mut graph[e]`.
///
/// **Panics** if no edge exists between `a` and `b`.
impl<
    N,
    E,
    Ty: EdgeType,
    Null: Nullable<Wrapped = E>,
    Ix: IndexType,
> IndexMut<(NodeIndex<Ix>, NodeIndex<Ix>)> for MatrixGraph<N, E, Ty, Null, Ix> {
    fn index_mut(&mut self, (ax, bx): (NodeIndex<Ix>, NodeIndex<Ix>)) -> &mut E {
        self.edge_weight_mut(ax, bx)
    }
}
impl<N, E, Ty: EdgeType, Null: Nullable<Wrapped = E>, Ix: IndexType> GetAdjacencyMatrix
for MatrixGraph<N, E, Ty, Null, Ix> {
    type AdjMatrix = ();
    fn adjacency_matrix(&self) -> Self::AdjMatrix {}
    fn is_adjacent(
        &self,
        _: &Self::AdjMatrix,
        a: NodeIndex<Ix>,
        b: NodeIndex<Ix>,
    ) -> bool {
        MatrixGraph::has_edge(self, a, b)
    }
}
impl<N, E, Ty: EdgeType, Null: Nullable<Wrapped = E>, Ix: IndexType> Visitable
for MatrixGraph<N, E, Ty, Null, Ix> {
    type Map = FixedBitSet;
    fn visit_map(&self) -> FixedBitSet {
        FixedBitSet::with_capacity(self.node_count())
    }
    fn reset_map(&self, map: &mut Self::Map) {
        map.clear();
        map.grow(self.node_count());
    }
}
impl<N, E, Ty: EdgeType, Null: Nullable<Wrapped = E>, Ix: IndexType> GraphBase
for MatrixGraph<N, E, Ty, Null, Ix> {
    type NodeId = NodeIndex<Ix>;
    type EdgeId = (NodeIndex<Ix>, NodeIndex<Ix>);
}
impl<N, E, Ty: EdgeType, Null: Nullable<Wrapped = E>, Ix: IndexType> GraphProp
for MatrixGraph<N, E, Ty, Null, Ix> {
    type EdgeType = Ty;
}
impl<N, E, Ty: EdgeType, Null: Nullable<Wrapped = E>, Ix: IndexType> Data
for MatrixGraph<N, E, Ty, Null, Ix> {
    type NodeWeight = N;
    type EdgeWeight = E;
}
impl<
    'a,
    N,
    E: 'a,
    Ty: EdgeType,
    Null: Nullable<Wrapped = E>,
    Ix: IndexType,
> IntoNodeIdentifiers for &'a MatrixGraph<N, E, Ty, Null, Ix> {
    type NodeIdentifiers = NodeIdentifiers<'a, Ix>;
    fn node_identifiers(self) -> Self::NodeIdentifiers {
        NodeIdentifiers::new(self.nodes.iter_ids())
    }
}
impl<
    'a,
    N,
    E: 'a,
    Ty: EdgeType,
    Null: Nullable<Wrapped = E>,
    Ix: IndexType,
> IntoNeighbors for &'a MatrixGraph<N, E, Ty, Null, Ix> {
    type Neighbors = Neighbors<'a, Ty, Null, Ix>;
    fn neighbors(self, a: NodeIndex<Ix>) -> Self::Neighbors {
        MatrixGraph::neighbors(self, a)
    }
}
impl<'a, N, E: 'a, Null: Nullable<Wrapped = E>, Ix: IndexType> IntoNeighborsDirected
for &'a MatrixGraph<N, E, Directed, Null, Ix> {
    type NeighborsDirected = Neighbors<'a, Directed, Null, Ix>;
    fn neighbors_directed(
        self,
        a: NodeIndex<Ix>,
        d: Direction,
    ) -> Self::NeighborsDirected {
        MatrixGraph::neighbors_directed(self, a, d)
    }
}
impl<
    'a,
    N,
    E,
    Ty: EdgeType,
    Null: Nullable<Wrapped = E>,
    Ix: IndexType,
> IntoNodeReferences for &'a MatrixGraph<N, E, Ty, Null, Ix> {
    type NodeRef = (NodeIndex<Ix>, &'a N);
    type NodeReferences = NodeReferences<'a, N, Ix>;
    fn node_references(self) -> Self::NodeReferences {
        NodeReferences::new(&self.nodes)
    }
}
impl<
    'a,
    N,
    E,
    Ty: EdgeType,
    Null: Nullable<Wrapped = E>,
    Ix: IndexType,
> IntoEdgeReferences for &'a MatrixGraph<N, E, Ty, Null, Ix> {
    type EdgeRef = (NodeIndex<Ix>, NodeIndex<Ix>, &'a E);
    type EdgeReferences = EdgeReferences<'a, Ty, Null, Ix>;
    fn edge_references(self) -> Self::EdgeReferences {
        EdgeReferences::new(&self.node_adjacencies, self.node_capacity)
    }
}
impl<'a, N, E, Ty: EdgeType, Null: Nullable<Wrapped = E>, Ix: IndexType> IntoEdges
for &'a MatrixGraph<N, E, Ty, Null, Ix> {
    type Edges = Edges<'a, Ty, Null, Ix>;
    fn edges(self, a: Self::NodeId) -> Self::Edges {
        MatrixGraph::edges(self, a)
    }
}
impl<N, E, Ty: EdgeType, Null: Nullable<Wrapped = E>, Ix: IndexType> NodeIndexable
for MatrixGraph<N, E, Ty, Null, Ix> {
    fn node_bound(&self) -> usize {
        self.node_count()
    }
    fn to_index(&self, ix: NodeIndex<Ix>) -> usize {
        ix.index()
    }
    fn from_index(&self, ix: usize) -> Self::NodeId {
        NodeIndex::new(ix)
    }
}
impl<N, E, Ty: EdgeType, Null: Nullable<Wrapped = E>, Ix: IndexType> NodeCompactIndexable
for MatrixGraph<N, E, Ty, Null, Ix> {}
impl<N, E, Ty: EdgeType, Null: Nullable<Wrapped = E>, Ix: IndexType> Build
for MatrixGraph<N, E, Ty, Null, Ix> {
    fn add_node(&mut self, weight: Self::NodeWeight) -> Self::NodeId {
        self.add_node(weight)
    }
    fn add_edge(
        &mut self,
        a: Self::NodeId,
        b: Self::NodeId,
        weight: Self::EdgeWeight,
    ) -> Option<Self::EdgeId> {
        if !self.has_edge(a, b) {
            MatrixGraph::update_edge(self, a, b, weight);
            Some((a, b))
        } else {
            None
        }
    }
    fn update_edge(
        &mut self,
        a: Self::NodeId,
        b: Self::NodeId,
        weight: Self::EdgeWeight,
    ) -> Self::EdgeId {
        MatrixGraph::update_edge(self, a, b, weight);
        (a, b)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Incoming, Outgoing};
    #[test]
    fn test_new() {
        let g = MatrixGraph::<i32, i32>::new();
        assert_eq!(g.node_count(), 0);
        assert_eq!(g.edge_count(), 0);
    }
    #[test]
    fn test_default() {
        let g = MatrixGraph::<i32, i32>::default();
        assert_eq!(g.node_count(), 0);
        assert_eq!(g.edge_count(), 0);
    }
    #[test]
    fn test_with_capacity() {
        let g = MatrixGraph::<i32, i32>::with_capacity(10);
        assert_eq!(g.node_count(), 0);
        assert_eq!(g.edge_count(), 0);
    }
    #[test]
    fn test_node_indexing() {
        let mut g: MatrixGraph<char, ()> = MatrixGraph::new();
        let a = g.add_node('a');
        let b = g.add_node('b');
        assert_eq!(g.node_count(), 2);
        assert_eq!(g.edge_count(), 0);
        assert_eq!(g[a], 'a');
        assert_eq!(g[b], 'b');
    }
    #[test]
    fn test_remove_node() {
        let mut g: MatrixGraph<char, ()> = MatrixGraph::new();
        let a = g.add_node('a');
        g.remove_node(a);
        assert_eq!(g.node_count(), 0);
        assert_eq!(g.edge_count(), 0);
    }
    #[test]
    fn test_add_edge() {
        let mut g = MatrixGraph::new();
        let a = g.add_node('a');
        let b = g.add_node('b');
        let c = g.add_node('c');
        g.add_edge(a, b, ());
        g.add_edge(b, c, ());
        assert_eq!(g.node_count(), 3);
        assert_eq!(g.edge_count(), 2);
    }
    #[test]
    fn test_add_edge_with_weights() {
        let mut g = MatrixGraph::new();
        let a = g.add_node('a');
        let b = g.add_node('b');
        let c = g.add_node('c');
        g.add_edge(a, b, true);
        g.add_edge(b, c, false);
        assert_eq!(* g.edge_weight(a, b), true);
        assert_eq!(* g.edge_weight(b, c), false);
    }
    #[test]
    fn test_add_edge_with_weights_undirected() {
        let mut g = MatrixGraph::new_undirected();
        let a = g.add_node('a');
        let b = g.add_node('b');
        let c = g.add_node('c');
        let d = g.add_node('d');
        g.add_edge(a, b, "ab");
        g.add_edge(a, a, "aa");
        g.add_edge(b, c, "bc");
        g.add_edge(d, d, "dd");
        assert_eq!(* g.edge_weight(a, b), "ab");
        assert_eq!(* g.edge_weight(b, c), "bc");
    }
    /// Shorthand for `.collect::<Vec<_>>()`
    trait IntoVec<T> {
        fn into_vec(self) -> Vec<T>;
    }
    impl<It, T> IntoVec<T> for It
    where
        It: Iterator<Item = T>,
    {
        fn into_vec(self) -> Vec<T> {
            self.collect()
        }
    }
    #[test]
    fn test_clear() {
        let mut g = MatrixGraph::new();
        let a = g.add_node('a');
        let b = g.add_node('b');
        let c = g.add_node('c');
        assert_eq!(g.node_count(), 3);
        g.add_edge(a, b, ());
        g.add_edge(b, c, ());
        g.add_edge(c, a, ());
        assert_eq!(g.edge_count(), 3);
        g.clear();
        assert_eq!(g.node_count(), 0);
        assert_eq!(g.edge_count(), 0);
        let a = g.add_node('a');
        let b = g.add_node('b');
        let c = g.add_node('c');
        assert_eq!(g.node_count(), 3);
        assert_eq!(g.edge_count(), 0);
        assert_eq!(g.neighbors_directed(a, Incoming).into_vec(), vec![]);
        assert_eq!(g.neighbors_directed(b, Incoming).into_vec(), vec![]);
        assert_eq!(g.neighbors_directed(c, Incoming).into_vec(), vec![]);
        assert_eq!(g.neighbors_directed(a, Outgoing).into_vec(), vec![]);
        assert_eq!(g.neighbors_directed(b, Outgoing).into_vec(), vec![]);
        assert_eq!(g.neighbors_directed(c, Outgoing).into_vec(), vec![]);
    }
    #[test]
    fn test_clear_undirected() {
        let mut g = MatrixGraph::new_undirected();
        let a = g.add_node('a');
        let b = g.add_node('b');
        let c = g.add_node('c');
        assert_eq!(g.node_count(), 3);
        g.add_edge(a, b, ());
        g.add_edge(b, c, ());
        g.add_edge(c, a, ());
        assert_eq!(g.edge_count(), 3);
        g.clear();
        assert_eq!(g.node_count(), 0);
        assert_eq!(g.edge_count(), 0);
        let a = g.add_node('a');
        let b = g.add_node('b');
        let c = g.add_node('c');
        assert_eq!(g.node_count(), 3);
        assert_eq!(g.edge_count(), 0);
        assert_eq!(g.neighbors(a).into_vec(), vec![]);
        assert_eq!(g.neighbors(b).into_vec(), vec![]);
        assert_eq!(g.neighbors(c).into_vec(), vec![]);
    }
    /// Helper trait for always sorting before testing.
    trait IntoSortedVec<T> {
        fn into_sorted_vec(self) -> Vec<T>;
    }
    impl<It, T> IntoSortedVec<T> for It
    where
        It: Iterator<Item = T>,
        T: Ord,
    {
        fn into_sorted_vec(self) -> Vec<T> {
            let mut v: Vec<T> = self.collect();
            v.sort();
            v
        }
    }
    /// Helper macro for always sorting before testing.
    macro_rules! sorted_vec {
        ($($x:expr),*) => {
            { let mut v = vec![$($x,)*]; v.sort(); v }
        };
    }
    #[test]
    fn test_neighbors() {
        let mut g = MatrixGraph::new();
        let a = g.add_node('a');
        let b = g.add_node('b');
        let c = g.add_node('c');
        g.add_edge(a, b, ());
        g.add_edge(a, c, ());
        let a_neighbors = g.neighbors(a).into_sorted_vec();
        assert_eq!(a_neighbors, sorted_vec![b, c]);
        let b_neighbors = g.neighbors(b).into_sorted_vec();
        assert_eq!(b_neighbors, vec![]);
        let c_neighbors = g.neighbors(c).into_sorted_vec();
        assert_eq!(c_neighbors, vec![]);
    }
    #[test]
    fn test_neighbors_undirected() {
        let mut g = MatrixGraph::new_undirected();
        let a = g.add_node('a');
        let b = g.add_node('b');
        let c = g.add_node('c');
        g.add_edge(a, b, ());
        g.add_edge(a, c, ());
        let a_neighbors = g.neighbors(a).into_sorted_vec();
        assert_eq!(a_neighbors, sorted_vec![b, c]);
        let b_neighbors = g.neighbors(b).into_sorted_vec();
        assert_eq!(b_neighbors, sorted_vec![a]);
        let c_neighbors = g.neighbors(c).into_sorted_vec();
        assert_eq!(c_neighbors, sorted_vec![a]);
    }
    #[test]
    fn test_remove_node_and_edges() {
        let mut g = MatrixGraph::new();
        let a = g.add_node('a');
        let b = g.add_node('b');
        let c = g.add_node('c');
        g.add_edge(a, b, ());
        g.add_edge(b, c, ());
        g.add_edge(c, a, ());
        g.remove_node(b);
        assert_eq!(g.node_count(), 2);
        let a_neighbors = g.neighbors(a).into_sorted_vec();
        assert_eq!(a_neighbors, vec![]);
        let c_neighbors = g.neighbors(c).into_sorted_vec();
        assert_eq!(c_neighbors, vec![a]);
    }
    #[test]
    fn test_remove_node_and_edges_undirected() {
        let mut g = UnMatrix::new_undirected();
        let a = g.add_node('a');
        let b = g.add_node('b');
        let c = g.add_node('c');
        g.add_edge(a, b, ());
        g.add_edge(b, c, ());
        g.add_edge(c, a, ());
        g.remove_node(a);
        assert_eq!(g.node_count(), 2);
        let b_neighbors = g.neighbors(b).into_sorted_vec();
        assert_eq!(b_neighbors, vec![c]);
        let c_neighbors = g.neighbors(c).into_sorted_vec();
        assert_eq!(c_neighbors, vec![b]);
    }
    #[test]
    fn test_node_identifiers() {
        let mut g = MatrixGraph::new();
        let a = g.add_node('a');
        let b = g.add_node('b');
        let c = g.add_node('c');
        let d = g.add_node('c');
        g.add_edge(a, b, ());
        g.add_edge(a, c, ());
        let node_ids = g.node_identifiers().into_sorted_vec();
        assert_eq!(node_ids, sorted_vec![a, b, c, d]);
    }
    #[test]
    fn test_edges_directed() {
        let g: MatrixGraph<char, bool> = MatrixGraph::from_edges(
            &[(0, 5), (0, 2), (0, 3), (0, 1), (1, 3), (2, 3), (2, 4), (4, 0), (6, 6)],
        );
        assert_eq!(g.edges_directed(node_index(0), Outgoing).count(), 4);
        assert_eq!(g.edges_directed(node_index(1), Outgoing).count(), 1);
        assert_eq!(g.edges_directed(node_index(2), Outgoing).count(), 2);
        assert_eq!(g.edges_directed(node_index(3), Outgoing).count(), 0);
        assert_eq!(g.edges_directed(node_index(4), Outgoing).count(), 1);
        assert_eq!(g.edges_directed(node_index(5), Outgoing).count(), 0);
        assert_eq!(g.edges_directed(node_index(6), Outgoing).count(), 1);
        assert_eq!(g.edges_directed(node_index(0), Incoming).count(), 1);
        assert_eq!(g.edges_directed(node_index(1), Incoming).count(), 1);
        assert_eq!(g.edges_directed(node_index(2), Incoming).count(), 1);
        assert_eq!(g.edges_directed(node_index(3), Incoming).count(), 3);
        assert_eq!(g.edges_directed(node_index(4), Incoming).count(), 1);
        assert_eq!(g.edges_directed(node_index(5), Incoming).count(), 1);
        assert_eq!(g.edges_directed(node_index(6), Incoming).count(), 1);
    }
    #[test]
    fn test_edges_undirected() {
        let g: UnMatrix<char, bool> = UnMatrix::from_edges(
            &[(0, 5), (0, 2), (0, 3), (0, 1), (1, 3), (2, 3), (2, 4), (4, 0), (6, 6)],
        );
        assert_eq!(g.edges(node_index(0)).count(), 5);
        assert_eq!(g.edges(node_index(1)).count(), 2);
        assert_eq!(g.edges(node_index(2)).count(), 3);
        assert_eq!(g.edges(node_index(3)).count(), 3);
        assert_eq!(g.edges(node_index(4)).count(), 2);
        assert_eq!(g.edges(node_index(5)).count(), 1);
        assert_eq!(g.edges(node_index(6)).count(), 1);
    }
    #[test]
    fn test_edges_of_absent_node_is_empty_iterator() {
        let g: MatrixGraph<char, bool> = MatrixGraph::new();
        assert_eq!(g.edges(node_index(0)).count(), 0);
    }
    #[test]
    fn test_neighbors_of_absent_node_is_empty_iterator() {
        let g: MatrixGraph<char, bool> = MatrixGraph::new();
        assert_eq!(g.neighbors(node_index(0)).count(), 0);
    }
    #[test]
    fn test_edge_references() {
        let g: MatrixGraph<char, bool> = MatrixGraph::from_edges(
            &[(0, 5), (0, 2), (0, 3), (0, 1), (1, 3), (2, 3), (2, 4), (4, 0), (6, 6)],
        );
        assert_eq!(g.edge_references().count(), 9);
    }
    #[test]
    fn test_edge_references_undirected() {
        let g: UnMatrix<char, bool> = UnMatrix::from_edges(
            &[(0, 5), (0, 2), (0, 3), (0, 1), (1, 3), (2, 3), (2, 4), (4, 0), (6, 6)],
        );
        assert_eq!(g.edge_references().count(), 9);
    }
    #[test]
    fn test_id_storage() {
        use super::IdStorage;
        let mut storage: IdStorage<char> = IdStorage::with_capacity(0);
        let a = storage.add('a');
        let b = storage.add('b');
        let c = storage.add('c');
        assert!(a < b && b < c);
        assert_eq!(storage.iter_ids().into_vec(), vec![a, b, c]);
        storage.remove(b);
        let bb = storage.add('B');
        assert_eq!(b, bb);
        assert_eq!(storage.iter_ids().into_vec(), vec![a, b, c]);
    }
    #[test]
    fn test_not_zero() {
        let mut g: MatrixGraph<(), i32, Directed, NotZero<i32>> = MatrixGraph::default();
        let a = g.add_node(());
        let b = g.add_node(());
        assert!(! g.has_edge(a, b));
        assert_eq!(g.edge_count(), 0);
        g.add_edge(a, b, 12);
        assert!(g.has_edge(a, b));
        assert_eq!(g.edge_count(), 1);
        assert_eq!(g.edge_weight(a, b), & 12);
        g.remove_edge(a, b);
        assert!(! g.has_edge(a, b));
        assert_eq!(g.edge_count(), 0);
    }
    #[test]
    #[should_panic]
    fn test_not_zero_asserted() {
        let mut g: MatrixGraph<(), i32, Directed, NotZero<i32>> = MatrixGraph::default();
        let a = g.add_node(());
        let b = g.add_node(());
        g.add_edge(a, b, 0);
    }
    #[test]
    fn test_not_zero_float() {
        let mut g: MatrixGraph<(), f32, Directed, NotZero<f32>> = MatrixGraph::default();
        let a = g.add_node(());
        let b = g.add_node(());
        assert!(! g.has_edge(a, b));
        assert_eq!(g.edge_count(), 0);
        g.add_edge(a, b, 12.);
        assert!(g.has_edge(a, b));
        assert_eq!(g.edge_count(), 1);
        assert_eq!(g.edge_weight(a, b), & 12.);
        g.remove_edge(a, b);
        assert!(! g.has_edge(a, b));
        assert_eq!(g.edge_count(), 0);
    }
}
#[cfg(test)]
mod tests_rug_387 {
    use super::*;
    use crate::graph::NodeIndex;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: usize = rug_fuzz_0;
        crate::matrix_graph::node_index(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_388 {
    use super::*;
    use crate::matrix_graph::{
        to_linearized_matrix_position, to_flat_square_matrix_position,
        to_lower_triangular_matrix_position,
    };
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(usize, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: usize = rug_fuzz_0;
        let mut p1: usize = rug_fuzz_1;
        let mut p2: usize = rug_fuzz_2;
        to_linearized_matrix_position::<Directed>(p0, p1, p2);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_389 {
    use super::*;
    use crate::matrix_graph::{
        extend_linearized_matrix, extend_flat_square_matrix,
        extend_lower_triangular_matrix,
    };
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Vec<i32> = Vec::new();
        let p1: usize = rug_fuzz_0;
        let p2: usize = rug_fuzz_1;
        extend_linearized_matrix::<Directed, i32>(&mut p0, p1, p2);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_390 {
    use super::*;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(usize, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: usize = rug_fuzz_0;
        let mut p1: usize = rug_fuzz_1;
        let mut p2: usize = rug_fuzz_2;
        crate::matrix_graph::to_flat_square_matrix_position(p0, p1, p2);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_391 {
    use super::*;
    use std::cmp;
    #[test]
    fn test_extend_flat_square_matrix() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: std::vec::Vec<i32> = std::vec::Vec::new();
        let p1: usize = rug_fuzz_0;
        let p2: usize = rug_fuzz_1;
        crate::matrix_graph::extend_flat_square_matrix(&mut p0, p1, p2);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_392 {
    use super::*;
    use crate::matrix_graph::to_lower_triangular_matrix_position;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: usize = rug_fuzz_0;
        let mut p1: usize = rug_fuzz_1;
        to_lower_triangular_matrix_position(p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_393 {
    use super::*;
    use crate::matrix_graph::{
        extend_lower_triangular_matrix, to_lower_triangular_matrix_position, ensure_len,
    };
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: std::vec::Vec<i32> = std::vec::Vec::new();
        let p1: usize = rug_fuzz_0;
        extend_lower_triangular_matrix(&mut p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_394 {
    use super::*;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: std::vec::Vec<i32> = std::vec::Vec::new();
        let p1: usize = rug_fuzz_0;
        crate::matrix_graph::ensure_len(&mut p0, p1);
        debug_assert_eq!(p0.len(), 10);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_395 {
    use super::*;
    use crate::matrix_graph::{NotZero, Zero, Nullable};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_395_rrrruuuugggg_test_rug = 0;
        let p0: NotZero<u32> = NotZero::default();
        debug_assert_eq!(p0.is_null(), false);
        let _rug_ed_tests_rug_395_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_396 {
    use super::*;
    use crate::matrix_graph::Nullable;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: i32 = rug_fuzz_0;
        <std::option::Option<i32>>::new(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_399 {
    use super::*;
    use crate::matrix_graph::NotZero;
    use std::default::Default;
    #[test]
    fn test_default() {
        let _rug_st_tests_rug_399_rrrruuuugggg_test_default = 0;
        let default_value: NotZero<u32> = <NotZero<u32> as Default>::default();
        let _rug_ed_tests_rug_399_rrrruuuugggg_test_default = 0;
    }
}
#[cfg(test)]
mod tests_rug_400 {
    use super::*;
    use crate::matrix_graph::{Nullable, NotZero, Zero};
    #[test]
    fn test_rug() {
        struct ConcreteType;
        impl Zero for ConcreteType {
            fn zero() -> Self {
                ConcreteType
            }
            fn is_zero(&self) -> bool {
                false
            }
        }
        let p0 = ConcreteType;
        <NotZero<ConcreteType>>::new(p0);
    }
}
#[cfg(test)]
mod tests_rug_401 {
    use super::*;
    use crate::matrix_graph::Nullable;
    use crate::matrix_graph::NotZero;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = NotZero::<i32>(rug_fuzz_0);
        debug_assert_eq!(< NotZero < i32 > as Nullable > ::is_null(& p0), false);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_402 {
    use super::*;
    use crate::matrix_graph::{NotZero, Nullable};
    #[test]
    fn test_as_ref() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0 = NotZero(rug_fuzz_0);
        p0.as_ref();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_404 {
    use super::*;
    use crate::matrix_graph::{NotZero, Nullable};
    use std::convert::Into;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let val = rug_fuzz_0;
        let p0 = NotZero(val);
        let result: Option<i32> = NotZero::<i32>::into(p0);
        debug_assert_eq!(result, Some(val));
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_405 {
    use super::*;
    use crate::matrix_graph::Zero;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_405_rrrruuuugggg_test_rug = 0;
        <u8 as crate::matrix_graph::Zero>::zero();
        let _rug_ed_tests_rug_405_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_406 {
    use super::*;
    use crate::matrix_graph::Zero;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: u8 = rug_fuzz_0;
        debug_assert_eq!(< u8 as Zero > ::is_zero(& p0), true);
             }
}
}
}    }
}
use crate::matrix_graph;
#[cfg(test)]
mod tests_rug_407 {
    use super::*;
    use crate::matrix_graph::Zero;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_407_rrrruuuugggg_test_rug = 0;
        let result: u16 = <u16 as matrix_graph::Zero>::zero();
        debug_assert_eq!(result, 0);
        let _rug_ed_tests_rug_407_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_408 {
    use super::*;
    use crate::matrix_graph::Zero;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: u16 = rug_fuzz_0;
        debug_assert!(< u16 as matrix_graph::Zero > ::is_zero(& p0));
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_409 {
    use super::*;
    use crate::matrix_graph::Zero;
    use crate::matrix_graph;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_409_rrrruuuugggg_test_rug = 0;
        <u32 as matrix_graph::Zero>::zero();
        let _rug_ed_tests_rug_409_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_410 {
    use super::*;
    use crate::matrix_graph::Zero;
    #[test]
    fn test_is_zero() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: u32 = rug_fuzz_0;
        debug_assert_eq!(p0.is_zero(), true);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_411 {
    use super::*;
    use crate::matrix_graph::Zero;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_411_rrrruuuugggg_test_rug = 0;
        debug_assert_eq!(< u64 as matrix_graph::Zero > ::zero(), 0);
        let _rug_ed_tests_rug_411_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_412 {
    use super::*;
    use crate::matrix_graph::Zero;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: u64 = rug_fuzz_0;
        debug_assert_eq!(< u64 as matrix_graph::Zero > ::is_zero(& p0), true);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_413 {
    use super::*;
    use crate::matrix_graph::Zero;
    use crate::matrix_graph;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_413_rrrruuuugggg_test_rug = 0;
        <usize as matrix_graph::Zero>::zero();
        let _rug_ed_tests_rug_413_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_414 {
    use super::*;
    use crate::matrix_graph::Zero;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: usize = rug_fuzz_0;
        debug_assert_eq!(< usize as matrix_graph::Zero > ::is_zero(& p0), false);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_415 {
    use super::*;
    use crate::matrix_graph::Zero;
    #[test]
    fn test_zero() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let z: i8 = rug_fuzz_0;
        let result = <i8 as Zero>::zero();
        debug_assert_eq!(result, z);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_416 {
    use super::*;
    use crate::matrix_graph::Zero;
    use crate::matrix_graph;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: i8 = rug_fuzz_0;
        debug_assert_eq!(< i8 as matrix_graph::Zero > ::is_zero(& p0), true);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_417 {
    use super::*;
    use crate::matrix_graph::Zero;
    use crate::matrix_graph;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_417_rrrruuuugggg_test_rug = 0;
        <i16 as matrix_graph::Zero>::zero();
        let _rug_ed_tests_rug_417_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_418 {
    use super::*;
    use crate::matrix_graph::Zero;
    #[test]
    fn test_is_zero() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: i16 = rug_fuzz_0;
        debug_assert_eq!(< i16 as matrix_graph::Zero > ::is_zero(& p0), true);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_419 {
    use super::*;
    use crate::matrix_graph::Zero;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let z: i32 = rug_fuzz_0;
        let t: i32 = rug_fuzz_1;
        <i32 as matrix_graph::Zero>::zero();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_420 {
    use super::*;
    use crate::matrix_graph::Zero;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: i32 = rug_fuzz_0;
        debug_assert_eq!(p0.is_zero(), true);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_421 {
    use super::*;
    use crate::matrix_graph::Zero;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_421_rrrruuuugggg_test_rug = 0;
        let result = <i64 as matrix_graph::Zero>::zero();
        debug_assert_eq!(result, 0);
        let _rug_ed_tests_rug_421_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_422 {
    use super::*;
    use crate::matrix_graph::Zero;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: i64 = rug_fuzz_0;
        debug_assert!(< i64 as crate ::matrix_graph::Zero > ::is_zero(& p0));
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_423 {
    use super::*;
    use crate::matrix_graph::Zero;
    use crate::matrix_graph;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_423_rrrruuuugggg_test_rug = 0;
        <isize as matrix_graph::Zero>::zero();
        let _rug_ed_tests_rug_423_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_424 {
    use super::*;
    use crate::matrix_graph::Zero;
    #[test]
    fn test_is_zero() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(isize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: isize = rug_fuzz_0;
        debug_assert_eq!(p0.is_zero(), true);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_425 {
    use super::*;
    use crate::matrix_graph::Zero;
    #[test]
    fn test_zero_f32() {
        let _rug_st_tests_rug_425_rrrruuuugggg_test_zero_f32 = 0;
        let result: f32 = f32::zero();
        debug_assert_eq!(result, 0.0);
        let _rug_ed_tests_rug_425_rrrruuuugggg_test_zero_f32 = 0;
    }
}
#[cfg(test)]
mod tests_rug_426 {
    use super::*;
    use crate::matrix_graph::Zero;
    #[test]
    fn test_is_zero() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: f32 = rug_fuzz_0;
        debug_assert_eq!(< f32 as Zero > ::is_zero(& p0), true);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_427 {
    use super::*;
    use crate::matrix_graph::Zero;
    use crate::graph::NodeIndex;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_427_rrrruuuugggg_test_rug = 0;
        let result: f64 = f64::zero();
        debug_assert_eq!(result, 0.0);
        let _rug_ed_tests_rug_427_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_428 {
    use super::*;
    use crate::matrix_graph::Zero;
    #[test]
    fn test_is_zero() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: f64 = rug_fuzz_0;
        debug_assert_eq!(< f64 as Zero > ::is_zero(& p0), true);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_454 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_454_rrrruuuugggg_test_rug = 0;
        let g = MatrixGraph::<(), ()>::new();
        let mut node_identifiers = g.node_identifiers();
        let mut p0 = node_identifiers;
        p0.next();
        let _rug_ed_tests_rug_454_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_463 {
    use super::*;
    use crate::matrix_graph::IdStorage;
    use crate::prelude::*;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: usize = rug_fuzz_0;
        IdStorage::<usize>::with_capacity(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_470 {
    use super::*;
    use crate::matrix_graph::IdStorage;
    use std::ops::IndexMut;
    #[test]
    fn test_index_mut() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(usize, i32, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut storage = IdStorage::<i32>::with_capacity(rug_fuzz_0);
        storage.add(rug_fuzz_1);
        let index = rug_fuzz_2;
        storage.index_mut(index);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_472 {
    use super::*;
    use crate::graph::{DefaultIx, NodeIndex};
    use crate::visit::{EdgeRef, IntoEdgeReferences};
    use crate::{Directed, Graph};
    #[test]
    fn test_default() {
        let _rug_st_tests_rug_472_rrrruuuugggg_test_default = 0;
        let graph: Graph<(), (), Directed, DefaultIx> = Default::default();
        let _rug_ed_tests_rug_472_rrrruuuugggg_test_default = 0;
    }
}
#[cfg(test)]
mod tests_rug_473 {
    use super::*;
    use crate::matrix_graph::MatrixGraph;
    #[test]
    fn test_new() {
        let _rug_st_tests_rug_473_rrrruuuugggg_test_new = 0;
        let graph: MatrixGraph<i32, &str> = MatrixGraph::new();
        let _rug_ed_tests_rug_473_rrrruuuugggg_test_new = 0;
    }
}
