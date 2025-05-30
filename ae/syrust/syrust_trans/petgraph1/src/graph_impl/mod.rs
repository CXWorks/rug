use std::cmp;
use std::fmt;
use std::hash::Hash;
use std::iter;
use std::marker::PhantomData;
use std::mem::size_of;
use std::ops::{Index, IndexMut, Range};
use std::slice;
use crate::{
    Directed, Direction, EdgeType, Incoming, IntoWeightedEdge, Outgoing, Undirected,
};
use crate::iter_format::{DebugMap, IterFormatExt, NoPretty};
use crate::util::enumerate;
use crate::visit::EdgeRef;
use crate::visit::{IntoEdges, IntoEdgesDirected, IntoNodeReferences};
#[cfg(feature = "serde-1")]
mod serialization;
/// The default integer type for graph indices.
/// `u32` is the default to reduce the size of the graph's data and improve
/// performance in the common case.
///
/// Used for node and edge indices in `Graph` and `StableGraph`, used
/// for node indices in `Csr`.
pub type DefaultIx = u32;
/// Trait for the unsigned integer type used for node and edge indices.
///
/// Marked `unsafe` because: the trait must faithfully preserve
/// and convert index values.
pub unsafe trait IndexType: Copy + Default + Hash + Ord + fmt::Debug + 'static {
    fn new(x: usize) -> Self;
    fn index(&self) -> usize;
    fn max() -> Self;
}
unsafe impl IndexType for usize {
    #[inline(always)]
    fn new(x: usize) -> Self {
        x
    }
    #[inline(always)]
    fn index(&self) -> Self {
        *self
    }
    #[inline(always)]
    fn max() -> Self {
        ::std::usize::MAX
    }
}
unsafe impl IndexType for u32 {
    #[inline(always)]
    fn new(x: usize) -> Self {
        x as u32
    }
    #[inline(always)]
    fn index(&self) -> usize {
        *self as usize
    }
    #[inline(always)]
    fn max() -> Self {
        ::std::u32::MAX
    }
}
unsafe impl IndexType for u16 {
    #[inline(always)]
    fn new(x: usize) -> Self {
        x as u16
    }
    #[inline(always)]
    fn index(&self) -> usize {
        *self as usize
    }
    #[inline(always)]
    fn max() -> Self {
        ::std::u16::MAX
    }
}
unsafe impl IndexType for u8 {
    #[inline(always)]
    fn new(x: usize) -> Self {
        x as u8
    }
    #[inline(always)]
    fn index(&self) -> usize {
        *self as usize
    }
    #[inline(always)]
    fn max() -> Self {
        ::std::u8::MAX
    }
}
/// Node identifier.
#[derive(Copy, Clone, Default, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct NodeIndex<Ix = DefaultIx>(Ix);
impl<Ix: IndexType> NodeIndex<Ix> {
    #[inline]
    pub fn new(x: usize) -> Self {
        NodeIndex(IndexType::new(x))
    }
    #[inline]
    pub fn index(self) -> usize {
        self.0.index()
    }
    #[inline]
    pub fn end() -> Self {
        NodeIndex(IndexType::max())
    }
    fn _into_edge(self) -> EdgeIndex<Ix> {
        EdgeIndex(self.0)
    }
}
impl<Ix: IndexType> From<Ix> for NodeIndex<Ix> {
    fn from(ix: Ix) -> Self {
        NodeIndex(ix)
    }
}
impl<Ix: fmt::Debug> fmt::Debug for NodeIndex<Ix> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "NodeIndex({:?})", self.0)
    }
}
/// Short version of `NodeIndex::new`
pub fn node_index<Ix: IndexType>(index: usize) -> NodeIndex<Ix> {
    NodeIndex::new(index)
}
/// Short version of `EdgeIndex::new`
pub fn edge_index<Ix: IndexType>(index: usize) -> EdgeIndex<Ix> {
    EdgeIndex::new(index)
}
/// Edge identifier.
#[derive(Copy, Clone, Default, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct EdgeIndex<Ix = DefaultIx>(Ix);
impl<Ix: IndexType> EdgeIndex<Ix> {
    #[inline]
    pub fn new(x: usize) -> Self {
        EdgeIndex(IndexType::new(x))
    }
    #[inline]
    pub fn index(self) -> usize {
        self.0.index()
    }
    /// An invalid `EdgeIndex` used to denote absence of an edge, for example
    /// to end an adjacency list.
    #[inline]
    pub fn end() -> Self {
        EdgeIndex(IndexType::max())
    }
    fn _into_node(self) -> NodeIndex<Ix> {
        NodeIndex(self.0)
    }
}
impl<Ix: IndexType> From<Ix> for EdgeIndex<Ix> {
    fn from(ix: Ix) -> Self {
        EdgeIndex(ix)
    }
}
impl<Ix: fmt::Debug> fmt::Debug for EdgeIndex<Ix> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "EdgeIndex({:?})", self.0)
    }
}
const DIRECTIONS: [Direction; 2] = [Outgoing, Incoming];
/// The graph's node type.
#[derive(Debug)]
pub struct Node<N, Ix = DefaultIx> {
    /// Associated node data.
    pub weight: N,
    /// Next edge in outgoing and incoming edge lists.
    next: [EdgeIndex<Ix>; 2],
}
impl<E, Ix> Clone for Node<E, Ix>
where
    E: Clone,
    Ix: Copy,
{
    clone_fields!(Node, weight, next,);
}
impl<N, Ix: IndexType> Node<N, Ix> {
    /// Accessor for data structure internals: the first edge in the given direction.
    pub fn next_edge(&self, dir: Direction) -> EdgeIndex<Ix> {
        self.next[dir.index()]
    }
}
/// The graph's edge type.
#[derive(Debug)]
pub struct Edge<E, Ix = DefaultIx> {
    /// Associated edge data.
    pub weight: E,
    /// Next edge in outgoing and incoming edge lists.
    next: [EdgeIndex<Ix>; 2],
    /// Start and End node index
    node: [NodeIndex<Ix>; 2],
}
impl<E, Ix> Clone for Edge<E, Ix>
where
    E: Clone,
    Ix: Copy,
{
    clone_fields!(Edge, weight, next, node,);
}
impl<E, Ix: IndexType> Edge<E, Ix> {
    /// Accessor for data structure internals: the next edge for the given direction.
    pub fn next_edge(&self, dir: Direction) -> EdgeIndex<Ix> {
        self.next[dir.index()]
    }
    /// Return the source node index.
    pub fn source(&self) -> NodeIndex<Ix> {
        self.node[0]
    }
    /// Return the target node index.
    pub fn target(&self) -> NodeIndex<Ix> {
        self.node[1]
    }
}
/// `Graph<N, E, Ty, Ix>` is a graph datastructure using an adjacency list representation.
///
/// `Graph` is parameterized over:
///
/// - Associated data `N` for nodes and `E` for edges, called *weights*.
///   The associated data can be of arbitrary type.
/// - Edge type `Ty` that determines whether the graph edges are directed or undirected.
/// - Index type `Ix`, which determines the maximum size of the graph.
///
/// The `Graph` is a regular Rust collection and is `Send` and `Sync` (as long
/// as associated data `N` and `E` are).
///
/// The graph uses **O(|V| + |E|)** space, and allows fast node and edge insert,
/// efficient graph search and graph algorithms.
/// It implements **O(e')** edge lookup and edge and node removals, where **e'**
/// is some local measure of edge count.
/// Based on the graph datastructure used in rustc.
///
/// Here's an example of building a graph with directed edges, and below
/// an illustration of how it could be rendered with graphviz (see
/// [`Dot`](../dot/struct.Dot.html)):
///
/// ```
/// use petgraph::Graph;
///
/// let mut deps = Graph::<&str, &str>::new();
/// let pg = deps.add_node("petgraph");
/// let fb = deps.add_node("fixedbitset");
/// let qc = deps.add_node("quickcheck");
/// let rand = deps.add_node("rand");
/// let libc = deps.add_node("libc");
/// deps.extend_with_edges(&[
///     (pg, fb), (pg, qc),
///     (qc, rand), (rand, libc), (qc, libc),
/// ]);
/// ```
///
/// ![graph-example](https://bluss.github.io/ndarray/images/graph-example.svg)
///
/// ### Graph Indices
///
/// The graph maintains indices for nodes and edges, and node and edge
/// weights may be accessed mutably. Indices range in a compact interval, for
/// example for *n* nodes indices are 0 to *n* - 1 inclusive.
///
/// `NodeIndex` and `EdgeIndex` are types that act as references to nodes and edges,
/// but these are only stable across certain operations:
///
/// * **Removing nodes or edges may shift other indices.** Removing a node will
/// force the last node to shift its index to take its place. Similarly,
/// removing an edge shifts the index of the last edge.
/// * Adding nodes or edges keeps indices stable.
///
/// The `Ix` parameter is `u32` by default. The goal is that you can ignore this parameter
/// completely unless you need a very big graph -- then you can use `usize`.
///
/// * The fact that the node and edge indices in the graph each are numbered in compact
/// intervals (from 0 to *n* - 1 for *n* nodes) simplifies some graph algorithms.
///
/// * You can select graph index integer type after the size of the graph. A smaller
/// size may have better performance.
///
/// * Using indices allows mutation while traversing the graph, see `Dfs`,
/// and `.neighbors(a).detach()`.
///
/// * You can create several graphs using the equal node indices but with
/// differing weights or differing edges.
///
/// * Indices don't allow as much compile time checking as references.
///
pub struct Graph<N, E, Ty = Directed, Ix = DefaultIx> {
    nodes: Vec<Node<N, Ix>>,
    edges: Vec<Edge<E, Ix>>,
    ty: PhantomData<Ty>,
}
/// A `Graph` with directed edges.
///
/// For example, an edge from *1* to *2* is distinct from an edge from *2* to
/// *1*.
pub type DiGraph<N, E, Ix = DefaultIx> = Graph<N, E, Directed, Ix>;
/// A `Graph` with undirected edges.
///
/// For example, an edge between *1* and *2* is equivalent to an edge between
/// *2* and *1*.
pub type UnGraph<N, E, Ix = DefaultIx> = Graph<N, E, Undirected, Ix>;
/// The resulting cloned graph has the same graph indices as `self`.
impl<N, E, Ty, Ix: IndexType> Clone for Graph<N, E, Ty, Ix>
where
    N: Clone,
    E: Clone,
{
    fn clone(&self) -> Self {
        Graph {
            nodes: self.nodes.clone(),
            edges: self.edges.clone(),
            ty: self.ty,
        }
    }
    fn clone_from(&mut self, rhs: &Self) {
        self.nodes.clone_from(&rhs.nodes);
        self.edges.clone_from(&rhs.edges);
        self.ty = rhs.ty;
    }
}
impl<N, E, Ty, Ix> fmt::Debug for Graph<N, E, Ty, Ix>
where
    N: fmt::Debug,
    E: fmt::Debug,
    Ty: EdgeType,
    Ix: IndexType,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let etype = if self.is_directed() { "Directed" } else { "Undirected" };
        let mut fmt_struct = f.debug_struct("Graph");
        fmt_struct.field("Ty", &etype);
        fmt_struct.field("node_count", &self.node_count());
        fmt_struct.field("edge_count", &self.edge_count());
        if self.edge_count() > 0 {
            fmt_struct
                .field(
                    "edges",
                    &self
                        .edges
                        .iter()
                        .map(|e| NoPretty((e.source().index(), e.target().index())))
                        .format(", "),
                );
        }
        if size_of::<N>() != 0 {
            fmt_struct
                .field(
                    "node weights",
                    &DebugMap(|| self.nodes.iter().map(|n| &n.weight).enumerate()),
                );
        }
        if size_of::<E>() != 0 {
            fmt_struct
                .field(
                    "edge weights",
                    &DebugMap(|| self.edges.iter().map(|n| &n.weight).enumerate()),
                );
        }
        fmt_struct.finish()
    }
}
enum Pair<T> {
    Both(T, T),
    One(T),
    None,
}
use std::cmp::max;
/// Get mutable references at index `a` and `b`.
fn index_twice<T>(slc: &mut [T], a: usize, b: usize) -> Pair<&mut T> {
    if max(a, b) >= slc.len() {
        Pair::None
    } else if a == b {
        Pair::One(&mut slc[max(a, b)])
    } else {
        unsafe {
            let ar = &mut *(slc.get_unchecked_mut(a) as *mut _);
            let br = &mut *(slc.get_unchecked_mut(b) as *mut _);
            Pair::Both(ar, br)
        }
    }
}
impl<N, E> Graph<N, E, Directed> {
    /// Create a new `Graph` with directed edges.
    ///
    /// This is a convenience method. Use `Graph::with_capacity` or `Graph::default` for
    /// a constructor that is generic in all the type parameters of `Graph`.
    pub fn new() -> Self {
        Graph {
            nodes: Vec::new(),
            edges: Vec::new(),
            ty: PhantomData,
        }
    }
}
impl<N, E> Graph<N, E, Undirected> {
    /// Create a new `Graph` with undirected edges.
    ///
    /// This is a convenience method. Use `Graph::with_capacity` or `Graph::default` for
    /// a constructor that is generic in all the type parameters of `Graph`.
    pub fn new_undirected() -> Self {
        Graph {
            nodes: Vec::new(),
            edges: Vec::new(),
            ty: PhantomData,
        }
    }
}
impl<N, E, Ty, Ix> Graph<N, E, Ty, Ix>
where
    Ty: EdgeType,
    Ix: IndexType,
{
    /// Create a new `Graph` with estimated capacity.
    pub fn with_capacity(nodes: usize, edges: usize) -> Self {
        Graph {
            nodes: Vec::with_capacity(nodes),
            edges: Vec::with_capacity(edges),
            ty: PhantomData,
        }
    }
    /// Return the number of nodes (vertices) in the graph.
    ///
    /// Computes in **O(1)** time.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
    /// Return the number of edges in the graph.
    ///
    /// Computes in **O(1)** time.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
    /// Whether the graph has directed edges or not.
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
    /// **Panics** if the Graph is at the maximum number of nodes for its index
    /// type (N/A if usize).
    pub fn add_node(&mut self, weight: N) -> NodeIndex<Ix> {
        let node = Node {
            weight,
            next: [EdgeIndex::end(), EdgeIndex::end()],
        };
        let node_idx = NodeIndex::new(self.nodes.len());
        assert!(
            < Ix as IndexType >::max().index() == ! 0 || NodeIndex::end() != node_idx
        );
        self.nodes.push(node);
        node_idx
    }
    /// Access the weight for node `a`.
    ///
    /// Also available with indexing syntax: `&graph[a]`.
    pub fn node_weight(&self, a: NodeIndex<Ix>) -> Option<&N> {
        self.nodes.get(a.index()).map(|n| &n.weight)
    }
    /// Access the weight for node `a`, mutably.
    ///
    /// Also available with indexing syntax: `&mut graph[a]`.
    pub fn node_weight_mut(&mut self, a: NodeIndex<Ix>) -> Option<&mut N> {
        self.nodes.get_mut(a.index()).map(|n| &mut n.weight)
    }
    /// Add an edge from `a` to `b` to the graph, with its associated
    /// data `weight`.
    ///
    /// Return the index of the new edge.
    ///
    /// Computes in **O(1)** time.
    ///
    /// **Panics** if any of the nodes don't exist.<br>
    /// **Panics** if the Graph is at the maximum number of edges for its index
    /// type (N/A if usize).
    ///
    /// **Note:** `Graph` allows adding parallel (“duplicate”) edges. If you want
    /// to avoid this, use [`.update_edge(a, b, weight)`](#method.update_edge) instead.
    pub fn add_edge(
        &mut self,
        a: NodeIndex<Ix>,
        b: NodeIndex<Ix>,
        weight: E,
    ) -> EdgeIndex<Ix> {
        let edge_idx = EdgeIndex::new(self.edges.len());
        assert!(
            < Ix as IndexType >::max().index() == ! 0 || EdgeIndex::end() != edge_idx
        );
        let mut edge = Edge {
            weight,
            node: [a, b],
            next: [EdgeIndex::end(); 2],
        };
        match index_twice(&mut self.nodes, a.index(), b.index()) {
            Pair::None => panic!("Graph::add_edge: node indices out of bounds"),
            Pair::One(an) => {
                edge.next = an.next;
                an.next[0] = edge_idx;
                an.next[1] = edge_idx;
            }
            Pair::Both(an, bn) => {
                edge.next = [an.next[0], bn.next[1]];
                an.next[0] = edge_idx;
                bn.next[1] = edge_idx;
            }
        }
        self.edges.push(edge);
        edge_idx
    }
    /// Add or update an edge from `a` to `b`.
    /// If the edge already exists, its weight is updated.
    ///
    /// Return the index of the affected edge.
    ///
    /// Computes in **O(e')** time, where **e'** is the number of edges
    /// connected to `a` (and `b`, if the graph edges are undirected).
    ///
    /// **Panics** if any of the nodes don't exist.
    pub fn update_edge(
        &mut self,
        a: NodeIndex<Ix>,
        b: NodeIndex<Ix>,
        weight: E,
    ) -> EdgeIndex<Ix> {
        if let Some(ix) = self.find_edge(a, b) {
            if let Some(ed) = self.edge_weight_mut(ix) {
                *ed = weight;
                return ix;
            }
        }
        self.add_edge(a, b, weight)
    }
    /// Access the weight for edge `e`.
    ///
    /// Also available with indexing syntax: `&graph[e]`.
    pub fn edge_weight(&self, e: EdgeIndex<Ix>) -> Option<&E> {
        self.edges.get(e.index()).map(|ed| &ed.weight)
    }
    /// Access the weight for edge `e`, mutably.
    ///
    /// Also available with indexing syntax: `&mut graph[e]`.
    pub fn edge_weight_mut(&mut self, e: EdgeIndex<Ix>) -> Option<&mut E> {
        self.edges.get_mut(e.index()).map(|ed| &mut ed.weight)
    }
    /// Access the source and target nodes for `e`.
    pub fn edge_endpoints(
        &self,
        e: EdgeIndex<Ix>,
    ) -> Option<(NodeIndex<Ix>, NodeIndex<Ix>)> {
        self.edges.get(e.index()).map(|ed| (ed.source(), ed.target()))
    }
    /// Remove `a` from the graph if it exists, and return its weight.
    /// If it doesn't exist in the graph, return `None`.
    ///
    /// Apart from `a`, this invalidates the last node index in the graph
    /// (that node will adopt the removed node index). Edge indices are
    /// invalidated as they would be following the removal of each edge
    /// with an endpoint in `a`.
    ///
    /// Computes in **O(e')** time, where **e'** is the number of affected
    /// edges, including *n* calls to `.remove_edge()` where *n* is the number
    /// of edges with an endpoint in `a`, and including the edges with an
    /// endpoint in the displaced node.
    pub fn remove_node(&mut self, a: NodeIndex<Ix>) -> Option<N> {
        self.nodes.get(a.index())?;
        for d in &DIRECTIONS {
            let k = d.index();
            loop {
                let next = self.nodes[a.index()].next[k];
                if next == EdgeIndex::end() {
                    break;
                }
                let ret = self.remove_edge(next);
                debug_assert!(ret.is_some());
                let _ = ret;
            }
        }
        let node = self.nodes.swap_remove(a.index());
        let swap_edges = match self.nodes.get(a.index()) {
            None => return Some(node.weight),
            Some(ed) => ed.next,
        };
        let old_index = NodeIndex::new(self.nodes.len());
        let new_index = a;
        for &d in &DIRECTIONS {
            let k = d.index();
            let mut edges = edges_walker_mut(&mut self.edges, swap_edges[k], d);
            while let Some(curedge) = edges.next_edge() {
                debug_assert!(curedge.node[k] == old_index);
                curedge.node[k] = new_index;
            }
        }
        Some(node.weight)
    }
    /// For edge `e` with endpoints `edge_node`, replace links to it,
    /// with links to `edge_next`.
    fn change_edge_links(
        &mut self,
        edge_node: [NodeIndex<Ix>; 2],
        e: EdgeIndex<Ix>,
        edge_next: [EdgeIndex<Ix>; 2],
    ) {
        for &d in &DIRECTIONS {
            let k = d.index();
            let node = match self.nodes.get_mut(edge_node[k].index()) {
                Some(r) => r,
                None => {
                    debug_assert!(
                        false, "Edge's endpoint dir={:?} index={:?} not found", d,
                        edge_node[k]
                    );
                    return;
                }
            };
            let fst = node.next[k];
            if fst == e {
                node.next[k] = edge_next[k];
            } else {
                let mut edges = edges_walker_mut(&mut self.edges, fst, d);
                while let Some(curedge) = edges.next_edge() {
                    if curedge.next[k] == e {
                        curedge.next[k] = edge_next[k];
                        break;
                    }
                }
            }
        }
    }
    /// Remove an edge and return its edge weight, or `None` if it didn't exist.
    ///
    /// Apart from `e`, this invalidates the last edge index in the graph
    /// (that edge will adopt the removed edge index).
    ///
    /// Computes in **O(e')** time, where **e'** is the size of four particular edge lists, for
    /// the vertices of `e` and the vertices of another affected edge.
    pub fn remove_edge(&mut self, e: EdgeIndex<Ix>) -> Option<E> {
        let (edge_node, edge_next) = match self.edges.get(e.index()) {
            None => return None,
            Some(x) => (x.node, x.next),
        };
        self.change_edge_links(edge_node, e, edge_next);
        self.remove_edge_adjust_indices(e)
    }
    fn remove_edge_adjust_indices(&mut self, e: EdgeIndex<Ix>) -> Option<E> {
        let edge = self.edges.swap_remove(e.index());
        let swap = match self.edges.get(e.index()) {
            None => return Some(edge.weight),
            Some(ed) => ed.node,
        };
        let swapped_e = EdgeIndex::new(self.edges.len());
        self.change_edge_links(swap, swapped_e, [e, e]);
        Some(edge.weight)
    }
    /// Return an iterator of all nodes with an edge starting from `a`.
    ///
    /// - `Directed`: Outgoing edges from `a`.
    /// - `Undirected`: All edges from or to `a`.
    ///
    /// Produces an empty iterator if the node doesn't exist.<br>
    /// Iterator element type is `NodeIndex<Ix>`.
    ///
    /// Use [`.neighbors(a).detach()`][1] to get a neighbor walker that does
    /// not borrow from the graph.
    ///
    /// [1]: struct.Neighbors.html#method.detach
    pub fn neighbors(&self, a: NodeIndex<Ix>) -> Neighbors<E, Ix> {
        self.neighbors_directed(a, Outgoing)
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
    /// Iterator element type is `NodeIndex<Ix>`.
    ///
    /// For a `Directed` graph, neighbors are listed in reverse order of their
    /// addition to the graph, so the most recently added edge's neighbor is
    /// listed first. The order in an `Undirected` graph is arbitrary.
    ///
    /// Use [`.neighbors_directed(a, dir).detach()`][1] to get a neighbor walker that does
    /// not borrow from the graph.
    ///
    /// [1]: struct.Neighbors.html#method.detach
    pub fn neighbors_directed(
        &self,
        a: NodeIndex<Ix>,
        dir: Direction,
    ) -> Neighbors<E, Ix> {
        let mut iter = self.neighbors_undirected(a);
        if self.is_directed() {
            let k = dir.index();
            iter.next[1 - k] = EdgeIndex::end();
            iter.skip_start = NodeIndex::end();
        }
        iter
    }
    /// Return an iterator of all neighbors that have an edge between them and
    /// `a`, in either direction.
    /// If the graph's edges are undirected, this is equivalent to *.neighbors(a)*.
    ///
    /// - `Directed` and `Undirected`: All edges from or to `a`.
    ///
    /// Produces an empty iterator if the node doesn't exist.<br>
    /// Iterator element type is `NodeIndex<Ix>`.
    ///
    /// Use [`.neighbors_undirected(a).detach()`][1] to get a neighbor walker that does
    /// not borrow from the graph.
    ///
    /// [1]: struct.Neighbors.html#method.detach
    ///
    pub fn neighbors_undirected(&self, a: NodeIndex<Ix>) -> Neighbors<E, Ix> {
        Neighbors {
            skip_start: a,
            edges: &self.edges,
            next: match self.nodes.get(a.index()) {
                None => [EdgeIndex::end(), EdgeIndex::end()],
                Some(n) => n.next,
            },
        }
    }
    /// Return an iterator of all edges of `a`.
    ///
    /// - `Directed`: Outgoing edges from `a`.
    /// - `Undirected`: All edges connected to `a`.
    ///
    /// Produces an empty iterator if the node doesn't exist.<br>
    /// Iterator element type is `EdgeReference<E, Ix>`.
    pub fn edges(&self, a: NodeIndex<Ix>) -> Edges<E, Ty, Ix> {
        self.edges_directed(a, Outgoing)
    }
    /// Return an iterator of all edges of `a`, in the specified direction.
    ///
    /// - `Directed`, `Outgoing`: All edges from `a`.
    /// - `Directed`, `Incoming`: All edges to `a`.
    /// - `Undirected`, `Outgoing`: All edges connected to `a`, with `a` being the source of each
    ///   edge.
    /// - `Undirected`, `Incoming`: All edges connected to `a`, with `a` being the target of each
    ///   edge.
    ///
    /// Produces an empty iterator if the node `a` doesn't exist.<br>
    /// Iterator element type is `EdgeReference<E, Ix>`.
    pub fn edges_directed(&self, a: NodeIndex<Ix>, dir: Direction) -> Edges<E, Ty, Ix> {
        Edges {
            skip_start: a,
            edges: &self.edges,
            direction: dir,
            next: match self.nodes.get(a.index()) {
                None => [EdgeIndex::end(), EdgeIndex::end()],
                Some(n) => n.next,
            },
            ty: PhantomData,
        }
    }
    /// Return an iterator over all the edges connecting `a` and `b`.
    ///
    /// - `Directed`: Outgoing edges from `a`.
    /// - `Undirected`: All edges connected to `a`.
    ///
    /// Iterator element type is `EdgeReference<E, Ix>`.
    pub fn edges_connecting(
        &self,
        a: NodeIndex<Ix>,
        b: NodeIndex<Ix>,
    ) -> EdgesConnecting<E, Ty, Ix> {
        EdgesConnecting {
            target_node: b,
            edges: self.edges_directed(a, Direction::Outgoing),
            ty: PhantomData,
        }
    }
    /// Lookup if there is an edge from `a` to `b`.
    ///
    /// Computes in **O(e')** time, where **e'** is the number of edges
    /// connected to `a` (and `b`, if the graph edges are undirected).
    pub fn contains_edge(&self, a: NodeIndex<Ix>, b: NodeIndex<Ix>) -> bool {
        self.find_edge(a, b).is_some()
    }
    /// Lookup an edge from `a` to `b`.
    ///
    /// Computes in **O(e')** time, where **e'** is the number of edges
    /// connected to `a` (and `b`, if the graph edges are undirected).
    pub fn find_edge(
        &self,
        a: NodeIndex<Ix>,
        b: NodeIndex<Ix>,
    ) -> Option<EdgeIndex<Ix>> {
        if !self.is_directed() {
            self.find_edge_undirected(a, b).map(|(ix, _)| ix)
        } else {
            match self.nodes.get(a.index()) {
                None => None,
                Some(node) => self.find_edge_directed_from_node(node, b),
            }
        }
    }
    fn find_edge_directed_from_node(
        &self,
        node: &Node<N, Ix>,
        b: NodeIndex<Ix>,
    ) -> Option<EdgeIndex<Ix>> {
        let mut edix = node.next[0];
        while let Some(edge) = self.edges.get(edix.index()) {
            if edge.node[1] == b {
                return Some(edix);
            }
            edix = edge.next[0];
        }
        None
    }
    /// Lookup an edge between `a` and `b`, in either direction.
    ///
    /// If the graph is undirected, then this is equivalent to `.find_edge()`.
    ///
    /// Return the edge index and its directionality, with `Outgoing` meaning
    /// from `a` to `b` and `Incoming` the reverse,
    /// or `None` if the edge does not exist.
    pub fn find_edge_undirected(
        &self,
        a: NodeIndex<Ix>,
        b: NodeIndex<Ix>,
    ) -> Option<(EdgeIndex<Ix>, Direction)> {
        match self.nodes.get(a.index()) {
            None => None,
            Some(node) => self.find_edge_undirected_from_node(node, b),
        }
    }
    fn find_edge_undirected_from_node(
        &self,
        node: &Node<N, Ix>,
        b: NodeIndex<Ix>,
    ) -> Option<(EdgeIndex<Ix>, Direction)> {
        for &d in &DIRECTIONS {
            let k = d.index();
            let mut edix = node.next[k];
            while let Some(edge) = self.edges.get(edix.index()) {
                if edge.node[1 - k] == b {
                    return Some((edix, d));
                }
                edix = edge.next[k];
            }
        }
        None
    }
    /// Return an iterator over either the nodes without edges to them
    /// (`Incoming`) or from them (`Outgoing`).
    ///
    /// An *internal* node has both incoming and outgoing edges.
    /// The nodes in `.externals(Incoming)` are the source nodes and
    /// `.externals(Outgoing)` are the sinks of the graph.
    ///
    /// For a graph with undirected edges, both the sinks and the sources are
    /// just the nodes without edges.
    ///
    /// The whole iteration computes in **O(|V|)** time.
    pub fn externals(&self, dir: Direction) -> Externals<N, Ty, Ix> {
        Externals {
            iter: self.nodes.iter().enumerate(),
            dir,
            ty: PhantomData,
        }
    }
    /// Return an iterator over the node indices of the graph.
    ///
    /// For example, in a rare case where a graph algorithm were not applicable,
    /// the following code will iterate through all nodes to find a
    /// specific index:
    ///
    /// ```
    /// # use petgraph::Graph;
    /// # let mut g = Graph::<&str, i32>::new();
    /// # g.add_node("book");
    /// let index = g.node_indices().find(|i| g[*i] == "book").unwrap();
    /// ```
    pub fn node_indices(&self) -> NodeIndices<Ix> {
        NodeIndices {
            r: 0..self.node_count(),
            ty: PhantomData,
        }
    }
    /// Return an iterator yielding mutable access to all node weights.
    ///
    /// The order in which weights are yielded matches the order of their
    /// node indices.
    pub fn node_weights_mut(&mut self) -> NodeWeightsMut<N, Ix> {
        NodeWeightsMut {
            nodes: self.nodes.iter_mut(),
        }
    }
    /// Return an iterator over the edge indices of the graph
    pub fn edge_indices(&self) -> EdgeIndices<Ix> {
        EdgeIndices {
            r: 0..self.edge_count(),
            ty: PhantomData,
        }
    }
    /// Create an iterator over all edges, in indexed order.
    ///
    /// Iterator element type is `EdgeReference<E, Ix>`.
    pub fn edge_references(&self) -> EdgeReferences<E, Ix> {
        EdgeReferences {
            iter: self.edges.iter().enumerate(),
        }
    }
    /// Return an iterator yielding mutable access to all edge weights.
    ///
    /// The order in which weights are yielded matches the order of their
    /// edge indices.
    pub fn edge_weights_mut(&mut self) -> EdgeWeightsMut<E, Ix> {
        EdgeWeightsMut {
            edges: self.edges.iter_mut(),
        }
    }
    /// Access the internal node array.
    pub fn raw_nodes(&self) -> &[Node<N, Ix>] {
        &self.nodes
    }
    /// Access the internal edge array.
    pub fn raw_edges(&self) -> &[Edge<E, Ix>] {
        &self.edges
    }
    /// Convert the graph into a vector of Nodes and a vector of Edges
    pub fn into_nodes_edges(self) -> (Vec<Node<N, Ix>>, Vec<Edge<E, Ix>>) {
        (self.nodes, self.edges)
    }
    /// Accessor for data structure internals: the first edge in the given direction.
    pub fn first_edge(&self, a: NodeIndex<Ix>, dir: Direction) -> Option<EdgeIndex<Ix>> {
        match self.nodes.get(a.index()) {
            None => None,
            Some(node) => {
                let edix = node.next[dir.index()];
                if edix == EdgeIndex::end() { None } else { Some(edix) }
            }
        }
    }
    /// Accessor for data structure internals: the next edge for the given direction.
    pub fn next_edge(&self, e: EdgeIndex<Ix>, dir: Direction) -> Option<EdgeIndex<Ix>> {
        match self.edges.get(e.index()) {
            None => None,
            Some(node) => {
                let edix = node.next[dir.index()];
                if edix == EdgeIndex::end() { None } else { Some(edix) }
            }
        }
    }
    /// Index the `Graph` by two indices, any combination of
    /// node or edge indices is fine.
    ///
    /// **Panics** if the indices are equal or if they are out of bounds.
    ///
    /// ```
    /// use petgraph::{Graph, Incoming};
    /// use petgraph::visit::Dfs;
    ///
    /// let mut gr = Graph::new();
    /// let a = gr.add_node(0.);
    /// let b = gr.add_node(0.);
    /// let c = gr.add_node(0.);
    /// gr.add_edge(a, b, 3.);
    /// gr.add_edge(b, c, 2.);
    /// gr.add_edge(c, b, 1.);
    ///
    /// // walk the graph and sum incoming edges into the node weight
    /// let mut dfs = Dfs::new(&gr, a);
    /// while let Some(node) = dfs.next(&gr) {
    ///     // use a walker -- a detached neighbors iterator
    ///     let mut edges = gr.neighbors_directed(node, Incoming).detach();
    ///     while let Some(edge) = edges.next_edge(&gr) {
    ///         let (nw, ew) = gr.index_twice_mut(node, edge);
    ///         *nw += *ew;
    ///     }
    /// }
    ///
    /// // check the result
    /// assert_eq!(gr[a], 0.);
    /// assert_eq!(gr[b], 4.);
    /// assert_eq!(gr[c], 2.);
    /// ```
    pub fn index_twice_mut<T, U>(
        &mut self,
        i: T,
        j: U,
    ) -> (&mut <Self as Index<T>>::Output, &mut <Self as Index<U>>::Output)
    where
        Self: IndexMut<T> + IndexMut<U>,
        T: GraphIndex,
        U: GraphIndex,
    {
        assert!(T::is_node_index() != U::is_node_index() || i.index() != j.index());
        unsafe {
            let self_mut = self as *mut _;
            (
                <Self as IndexMut<T>>::index_mut(&mut *self_mut, i),
                <Self as IndexMut<U>>::index_mut(&mut *self_mut, j),
            )
        }
    }
    /// Reverse the direction of all edges
    pub fn reverse(&mut self) {
        for edge in &mut self.edges {
            edge.node.swap(0, 1);
            edge.next.swap(0, 1);
        }
        for node in &mut self.nodes {
            node.next.swap(0, 1);
        }
    }
    /// Remove all nodes and edges
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.edges.clear();
    }
    /// Remove all edges
    pub fn clear_edges(&mut self) {
        self.edges.clear();
        for node in &mut self.nodes {
            node.next = [EdgeIndex::end(), EdgeIndex::end()];
        }
    }
    /// Return the current node and edge capacity of the graph.
    pub fn capacity(&self) -> (usize, usize) {
        (self.nodes.capacity(), self.edges.capacity())
    }
    /// Reserves capacity for at least `additional` more nodes to be inserted in
    /// the graph. Graph may reserve more space to avoid frequent reallocations.
    ///
    /// **Panics** if the new capacity overflows `usize`.
    pub fn reserve_nodes(&mut self, additional: usize) {
        self.nodes.reserve(additional);
    }
    /// Reserves capacity for at least `additional` more edges to be inserted in
    /// the graph. Graph may reserve more space to avoid frequent reallocations.
    ///
    /// **Panics** if the new capacity overflows `usize`.
    pub fn reserve_edges(&mut self, additional: usize) {
        self.edges.reserve(additional);
    }
    /// Reserves the minimum capacity for exactly `additional` more nodes to be
    /// inserted in the graph. Does nothing if the capacity is already
    /// sufficient.
    ///
    /// Prefer `reserve_nodes` if future insertions are expected.
    ///
    /// **Panics** if the new capacity overflows `usize`.
    pub fn reserve_exact_nodes(&mut self, additional: usize) {
        self.nodes.reserve_exact(additional);
    }
    /// Reserves the minimum capacity for exactly `additional` more edges to be
    /// inserted in the graph.
    /// Does nothing if the capacity is already sufficient.
    ///
    /// Prefer `reserve_edges` if future insertions are expected.
    ///
    /// **Panics** if the new capacity overflows `usize`.
    pub fn reserve_exact_edges(&mut self, additional: usize) {
        self.edges.reserve_exact(additional);
    }
    /// Shrinks the capacity of the underlying nodes collection as much as possible.
    pub fn shrink_to_fit_nodes(&mut self) {
        self.nodes.shrink_to_fit();
    }
    /// Shrinks the capacity of the underlying edges collection as much as possible.
    pub fn shrink_to_fit_edges(&mut self) {
        self.edges.shrink_to_fit();
    }
    /// Shrinks the capacity of the graph as much as possible.
    pub fn shrink_to_fit(&mut self) {
        self.nodes.shrink_to_fit();
        self.edges.shrink_to_fit();
    }
    /// Keep all nodes that return `true` from the `visit` closure,
    /// remove the others.
    ///
    /// `visit` is provided a proxy reference to the graph, so that
    /// the graph can be walked and associated data modified.
    ///
    /// The order nodes are visited is not specified.
    pub fn retain_nodes<F>(&mut self, mut visit: F)
    where
        F: FnMut(Frozen<Self>, NodeIndex<Ix>) -> bool,
    {
        for index in self.node_indices().rev() {
            if !visit(Frozen(self), index) {
                let ret = self.remove_node(index);
                debug_assert!(ret.is_some());
                let _ = ret;
            }
        }
    }
    /// Keep all edges that return `true` from the `visit` closure,
    /// remove the others.
    ///
    /// `visit` is provided a proxy reference to the graph, so that
    /// the graph can be walked and associated data modified.
    ///
    /// The order edges are visited is not specified.
    pub fn retain_edges<F>(&mut self, mut visit: F)
    where
        F: FnMut(Frozen<Self>, EdgeIndex<Ix>) -> bool,
    {
        for index in self.edge_indices().rev() {
            if !visit(Frozen(self), index) {
                let ret = self.remove_edge(index);
                debug_assert!(ret.is_some());
                let _ = ret;
            }
        }
    }
    /// Create a new `Graph` from an iterable of edges.
    ///
    /// Node weights `N` are set to default values.
    /// Edge weights `E` may either be specified in the list,
    /// or they are filled with default values.
    ///
    /// Nodes are inserted automatically to match the edges.
    ///
    /// ```
    /// use petgraph::Graph;
    ///
    /// let gr = Graph::<(), i32>::from_edges(&[
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
        let mut g = Self::with_capacity(0, 0);
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
        let iter = iterable.into_iter();
        let (low, _) = iter.size_hint();
        self.edges.reserve(low);
        for elt in iter {
            let (source, target, weight) = elt.into_weighted_edge();
            let (source, target) = (source.into(), target.into());
            let nx = cmp::max(source, target);
            while nx.index() >= self.node_count() {
                self.add_node(N::default());
            }
            self.add_edge(source, target, weight);
        }
    }
    /// Create a new `Graph` by mapping node and
    /// edge weights to new values.
    ///
    /// The resulting graph has the same structure and the same
    /// graph indices as `self`.
    pub fn map<'a, F, G, N2, E2>(
        &'a self,
        mut node_map: F,
        mut edge_map: G,
    ) -> Graph<N2, E2, Ty, Ix>
    where
        F: FnMut(NodeIndex<Ix>, &'a N) -> N2,
        G: FnMut(EdgeIndex<Ix>, &'a E) -> E2,
    {
        let mut g = Graph::with_capacity(self.node_count(), self.edge_count());
        g.nodes
            .extend(
                enumerate(&self.nodes)
                    .map(|(i, node)| Node {
                        weight: node_map(NodeIndex::new(i), &node.weight),
                        next: node.next,
                    }),
            );
        g.edges
            .extend(
                enumerate(&self.edges)
                    .map(|(i, edge)| Edge {
                        weight: edge_map(EdgeIndex::new(i), &edge.weight),
                        next: edge.next,
                        node: edge.node,
                    }),
            );
        g
    }
    /// Create a new `Graph` by mapping nodes and edges.
    /// A node or edge may be mapped to `None` to exclude it from
    /// the resulting graph.
    ///
    /// Nodes are mapped first with the `node_map` closure, then
    /// `edge_map` is called for the edges that have not had any endpoint
    /// removed.
    ///
    /// The resulting graph has the structure of a subgraph of the original graph.
    /// If no nodes are removed, the resulting graph has compatible node
    /// indices; if neither nodes nor edges are removed, the result has
    /// the same graph indices as `self`.
    pub fn filter_map<'a, F, G, N2, E2>(
        &'a self,
        mut node_map: F,
        mut edge_map: G,
    ) -> Graph<N2, E2, Ty, Ix>
    where
        F: FnMut(NodeIndex<Ix>, &'a N) -> Option<N2>,
        G: FnMut(EdgeIndex<Ix>, &'a E) -> Option<E2>,
    {
        let mut g = Graph::with_capacity(0, 0);
        let mut node_index_map = vec![NodeIndex::end(); self.node_count()];
        for (i, node) in enumerate(&self.nodes) {
            if let Some(nw) = node_map(NodeIndex::new(i), &node.weight) {
                node_index_map[i] = g.add_node(nw);
            }
        }
        for (i, edge) in enumerate(&self.edges) {
            let source = node_index_map[edge.source().index()];
            let target = node_index_map[edge.target().index()];
            if source != NodeIndex::end() && target != NodeIndex::end() {
                if let Some(ew) = edge_map(EdgeIndex::new(i), &edge.weight) {
                    g.add_edge(source, target, ew);
                }
            }
        }
        g
    }
    /// Convert the graph into either undirected or directed. No edge adjustments
    /// are done, so you may want to go over the result to remove or add edges.
    ///
    /// Computes in **O(1)** time.
    pub fn into_edge_type<NewTy>(self) -> Graph<N, E, NewTy, Ix>
    where
        NewTy: EdgeType,
    {
        Graph {
            nodes: self.nodes,
            edges: self.edges,
            ty: PhantomData,
        }
    }
    #[cfg(feature = "serde-1")]
    /// Fix up node and edge links after deserialization
    fn link_edges(&mut self) -> Result<(), NodeIndex<Ix>> {
        for (edge_index, edge) in enumerate(&mut self.edges) {
            let a = edge.source();
            let b = edge.target();
            let edge_idx = EdgeIndex::new(edge_index);
            match index_twice(&mut self.nodes, a.index(), b.index()) {
                Pair::None => return Err(if a > b { a } else { b }),
                Pair::One(an) => {
                    edge.next = an.next;
                    an.next[0] = edge_idx;
                    an.next[1] = edge_idx;
                }
                Pair::Both(an, bn) => {
                    edge.next = [an.next[0], bn.next[1]];
                    an.next[0] = edge_idx;
                    bn.next[1] = edge_idx;
                }
            }
        }
        Ok(())
    }
}
/// An iterator over either the nodes without edges to them or from them.
pub struct Externals<'a, N: 'a, Ty, Ix: IndexType = DefaultIx> {
    iter: iter::Enumerate<slice::Iter<'a, Node<N, Ix>>>,
    dir: Direction,
    ty: PhantomData<Ty>,
}
impl<'a, N: 'a, Ty, Ix> Iterator for Externals<'a, N, Ty, Ix>
where
    Ty: EdgeType,
    Ix: IndexType,
{
    type Item = NodeIndex<Ix>;
    fn next(&mut self) -> Option<NodeIndex<Ix>> {
        let k = self.dir.index();
        loop {
            match self.iter.next() {
                None => return None,
                Some((index, node)) => {
                    if node.next[k] == EdgeIndex::end()
                        && (Ty::is_directed() || node.next[1 - k] == EdgeIndex::end())
                    {
                        return Some(NodeIndex::new(index));
                    } else {
                        continue;
                    }
                }
            }
        }
    }
}
/// Iterator over the neighbors of a node.
///
/// Iterator element type is `NodeIndex<Ix>`.
///
/// Created with [`.neighbors()`][1], [`.neighbors_directed()`][2] or
/// [`.neighbors_undirected()`][3].
///
/// [1]: struct.Graph.html#method.neighbors
/// [2]: struct.Graph.html#method.neighbors_directed
/// [3]: struct.Graph.html#method.neighbors_undirected
pub struct Neighbors<'a, E: 'a, Ix: 'a = DefaultIx> {
    /// starting node to skip over
    skip_start: NodeIndex<Ix>,
    edges: &'a [Edge<E, Ix>],
    next: [EdgeIndex<Ix>; 2],
}
impl<'a, E, Ix> Iterator for Neighbors<'a, E, Ix>
where
    Ix: IndexType,
{
    type Item = NodeIndex<Ix>;
    fn next(&mut self) -> Option<NodeIndex<Ix>> {
        match self.edges.get(self.next[0].index()) {
            None => {}
            Some(edge) => {
                self.next[0] = edge.next[0];
                return Some(edge.node[1]);
            }
        }
        while let Some(edge) = self.edges.get(self.next[1].index()) {
            self.next[1] = edge.next[1];
            if edge.node[0] != self.skip_start {
                return Some(edge.node[0]);
            }
        }
        None
    }
}
impl<'a, E, Ix> Clone for Neighbors<'a, E, Ix>
where
    Ix: IndexType,
{
    clone_fields!(Neighbors, skip_start, edges, next,);
}
impl<'a, E, Ix> Neighbors<'a, E, Ix>
where
    Ix: IndexType,
{
    /// Return a “walker” object that can be used to step through the
    /// neighbors and edges from the origin node.
    ///
    /// Note: The walker does not borrow from the graph, this is to allow mixing
    /// edge walking with mutating the graph's weights.
    pub fn detach(&self) -> WalkNeighbors<Ix> {
        WalkNeighbors {
            skip_start: self.skip_start,
            next: self.next,
        }
    }
}
struct EdgesWalkerMut<'a, E: 'a, Ix: IndexType = DefaultIx> {
    edges: &'a mut [Edge<E, Ix>],
    next: EdgeIndex<Ix>,
    dir: Direction,
}
fn edges_walker_mut<E, Ix>(
    edges: &mut [Edge<E, Ix>],
    next: EdgeIndex<Ix>,
    dir: Direction,
) -> EdgesWalkerMut<E, Ix>
where
    Ix: IndexType,
{
    EdgesWalkerMut { edges, next, dir }
}
impl<'a, E, Ix> EdgesWalkerMut<'a, E, Ix>
where
    Ix: IndexType,
{
    fn next_edge(&mut self) -> Option<&mut Edge<E, Ix>> {
        self.next().map(|t| t.1)
    }
    fn next(&mut self) -> Option<(EdgeIndex<Ix>, &mut Edge<E, Ix>)> {
        let this_index = self.next;
        let k = self.dir.index();
        match self.edges.get_mut(self.next.index()) {
            None => None,
            Some(edge) => {
                self.next = edge.next[k];
                Some((this_index, edge))
            }
        }
    }
}
impl<'a, N, E, Ty, Ix> IntoEdges for &'a Graph<N, E, Ty, Ix>
where
    Ty: EdgeType,
    Ix: IndexType,
{
    type Edges = Edges<'a, E, Ty, Ix>;
    fn edges(self, a: Self::NodeId) -> Self::Edges {
        self.edges(a)
    }
}
impl<'a, N, E, Ty, Ix> IntoEdgesDirected for &'a Graph<N, E, Ty, Ix>
where
    Ty: EdgeType,
    Ix: IndexType,
{
    type EdgesDirected = Edges<'a, E, Ty, Ix>;
    fn edges_directed(self, a: Self::NodeId, dir: Direction) -> Self::EdgesDirected {
        self.edges_directed(a, dir)
    }
}
/// Iterator over the edges of from or to a node
pub struct Edges<'a, E: 'a, Ty, Ix: 'a = DefaultIx>
where
    Ty: EdgeType,
    Ix: IndexType,
{
    /// starting node to skip over
    skip_start: NodeIndex<Ix>,
    edges: &'a [Edge<E, Ix>],
    /// Next edge to visit.
    next: [EdgeIndex<Ix>; 2],
    /// For directed graphs: the direction to iterate in
    /// For undirected graphs: the direction of edges
    direction: Direction,
    ty: PhantomData<Ty>,
}
impl<'a, E, Ty, Ix> Iterator for Edges<'a, E, Ty, Ix>
where
    Ty: EdgeType,
    Ix: IndexType,
{
    type Item = EdgeReference<'a, E, Ix>;
    fn next(&mut self) -> Option<Self::Item> {
        let (iterate_over, reverse) = if Ty::is_directed() {
            (Some(self.direction), None)
        } else {
            (None, Some(self.direction.opposite()))
        };
        if iterate_over.unwrap_or(Outgoing) == Outgoing {
            let i = self.next[0].index();
            if let Some(Edge { node, weight, next }) = self.edges.get(i) {
                self.next[0] = next[0];
                return Some(EdgeReference {
                    index: edge_index(i),
                    node: if reverse == Some(Outgoing) {
                        swap_pair(*node)
                    } else {
                        *node
                    },
                    weight,
                });
            }
        }
        if iterate_over.unwrap_or(Incoming) == Incoming {
            while let Some(Edge { node, weight, next })
                = self.edges.get(self.next[1].index())
            {
                let edge_index = self.next[1];
                self.next[1] = next[1];
                if iterate_over.is_none() && node[0] == self.skip_start {
                    continue;
                }
                return Some(EdgeReference {
                    index: edge_index,
                    node: if reverse == Some(Incoming) {
                        swap_pair(*node)
                    } else {
                        *node
                    },
                    weight,
                });
            }
        }
        None
    }
}
/// Iterator over the multiple directed edges connecting a source node to a target node
pub struct EdgesConnecting<'a, E: 'a, Ty, Ix: 'a = DefaultIx>
where
    Ty: EdgeType,
    Ix: IndexType,
{
    target_node: NodeIndex<Ix>,
    edges: Edges<'a, E, Ty, Ix>,
    ty: PhantomData<Ty>,
}
impl<'a, E, Ty, Ix> Iterator for EdgesConnecting<'a, E, Ty, Ix>
where
    Ty: EdgeType,
    Ix: IndexType,
{
    type Item = EdgeReference<'a, E, Ix>;
    fn next(&mut self) -> Option<EdgeReference<'a, E, Ix>> {
        while let Some(edge) = self.edges.next() {
            if edge.node[1] == self.target_node {
                return Some(edge);
            }
        }
        None
    }
}
fn swap_pair<T>(mut x: [T; 2]) -> [T; 2] {
    x.swap(0, 1);
    x
}
impl<'a, E, Ty, Ix> Clone for Edges<'a, E, Ty, Ix>
where
    Ix: IndexType,
    Ty: EdgeType,
{
    fn clone(&self) -> Self {
        Edges {
            skip_start: self.skip_start,
            edges: self.edges,
            next: self.next,
            direction: self.direction,
            ty: self.ty,
        }
    }
}
/// Iterator yielding mutable access to all node weights.
pub struct NodeWeightsMut<'a, N: 'a, Ix: IndexType = DefaultIx> {
    nodes: ::std::slice::IterMut<'a, Node<N, Ix>>,
}
impl<'a, N, Ix> Iterator for NodeWeightsMut<'a, N, Ix>
where
    Ix: IndexType,
{
    type Item = &'a mut N;
    fn next(&mut self) -> Option<&'a mut N> {
        self.nodes.next().map(|node| &mut node.weight)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.nodes.size_hint()
    }
}
/// Iterator yielding mutable access to all edge weights.
pub struct EdgeWeightsMut<'a, E: 'a, Ix: IndexType = DefaultIx> {
    edges: ::std::slice::IterMut<'a, Edge<E, Ix>>,
}
impl<'a, E, Ix> Iterator for EdgeWeightsMut<'a, E, Ix>
where
    Ix: IndexType,
{
    type Item = &'a mut E;
    fn next(&mut self) -> Option<&'a mut E> {
        self.edges.next().map(|edge| &mut edge.weight)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.edges.size_hint()
    }
}
/// Index the `Graph` by `NodeIndex` to access node weights.
///
/// **Panics** if the node doesn't exist.
impl<N, E, Ty, Ix> Index<NodeIndex<Ix>> for Graph<N, E, Ty, Ix>
where
    Ty: EdgeType,
    Ix: IndexType,
{
    type Output = N;
    fn index(&self, index: NodeIndex<Ix>) -> &N {
        &self.nodes[index.index()].weight
    }
}
/// Index the `Graph` by `NodeIndex` to access node weights.
///
/// **Panics** if the node doesn't exist.
impl<N, E, Ty, Ix> IndexMut<NodeIndex<Ix>> for Graph<N, E, Ty, Ix>
where
    Ty: EdgeType,
    Ix: IndexType,
{
    fn index_mut(&mut self, index: NodeIndex<Ix>) -> &mut N {
        &mut self.nodes[index.index()].weight
    }
}
/// Index the `Graph` by `EdgeIndex` to access edge weights.
///
/// **Panics** if the edge doesn't exist.
impl<N, E, Ty, Ix> Index<EdgeIndex<Ix>> for Graph<N, E, Ty, Ix>
where
    Ty: EdgeType,
    Ix: IndexType,
{
    type Output = E;
    fn index(&self, index: EdgeIndex<Ix>) -> &E {
        &self.edges[index.index()].weight
    }
}
/// Index the `Graph` by `EdgeIndex` to access edge weights.
///
/// **Panics** if the edge doesn't exist.
impl<N, E, Ty, Ix> IndexMut<EdgeIndex<Ix>> for Graph<N, E, Ty, Ix>
where
    Ty: EdgeType,
    Ix: IndexType,
{
    fn index_mut(&mut self, index: EdgeIndex<Ix>) -> &mut E {
        &mut self.edges[index.index()].weight
    }
}
/// Create a new empty `Graph`.
impl<N, E, Ty, Ix> Default for Graph<N, E, Ty, Ix>
where
    Ty: EdgeType,
    Ix: IndexType,
{
    fn default() -> Self {
        Self::with_capacity(0, 0)
    }
}
/// A  `GraphIndex` is a node or edge index.
pub trait GraphIndex: Copy {
    #[doc(hidden)]
    fn index(&self) -> usize;
    #[doc(hidden)]
    fn is_node_index() -> bool;
}
impl<Ix: IndexType> GraphIndex for NodeIndex<Ix> {
    #[inline]
    fn index(&self) -> usize {
        NodeIndex::index(*self)
    }
    #[inline]
    fn is_node_index() -> bool {
        true
    }
}
impl<Ix: IndexType> GraphIndex for EdgeIndex<Ix> {
    #[inline]
    fn index(&self) -> usize {
        EdgeIndex::index(*self)
    }
    #[inline]
    fn is_node_index() -> bool {
        false
    }
}
/// A “walker” object that can be used to step through the edge list of a node.
///
/// Created with [`.detach()`](struct.Neighbors.html#method.detach).
///
/// The walker does not borrow from the graph, so it lets you step through
/// neighbors or incident edges while also mutating graph weights, as
/// in the following example:
///
/// ```
/// use petgraph::{Graph, Incoming};
/// use petgraph::visit::Dfs;
///
/// let mut gr = Graph::new();
/// let a = gr.add_node(0.);
/// let b = gr.add_node(0.);
/// let c = gr.add_node(0.);
/// gr.add_edge(a, b, 3.);
/// gr.add_edge(b, c, 2.);
/// gr.add_edge(c, b, 1.);
///
/// // step through the graph and sum incoming edges into the node weight
/// let mut dfs = Dfs::new(&gr, a);
/// while let Some(node) = dfs.next(&gr) {
///     // use a detached neighbors walker
///     let mut edges = gr.neighbors_directed(node, Incoming).detach();
///     while let Some(edge) = edges.next_edge(&gr) {
///         gr[node] += gr[edge];
///     }
/// }
///
/// // check the result
/// assert_eq!(gr[a], 0.);
/// assert_eq!(gr[b], 4.);
/// assert_eq!(gr[c], 2.);
/// ```
pub struct WalkNeighbors<Ix> {
    skip_start: NodeIndex<Ix>,
    next: [EdgeIndex<Ix>; 2],
}
impl<Ix> Clone for WalkNeighbors<Ix>
where
    Ix: IndexType,
{
    fn clone(&self) -> Self {
        WalkNeighbors {
            skip_start: self.skip_start,
            next: self.next,
        }
    }
}
impl<Ix: IndexType> WalkNeighbors<Ix> {
    /// Step to the next edge and its endpoint node in the walk for graph `g`.
    ///
    /// The next node indices are always the others than the starting point
    /// where the `WalkNeighbors` value was created.
    /// For an `Outgoing` walk, the target nodes,
    /// for an `Incoming` walk, the source nodes of the edge.
    pub fn next<N, E, Ty: EdgeType>(
        &mut self,
        g: &Graph<N, E, Ty, Ix>,
    ) -> Option<(EdgeIndex<Ix>, NodeIndex<Ix>)> {
        match g.edges.get(self.next[0].index()) {
            None => {}
            Some(edge) => {
                let ed = self.next[0];
                self.next[0] = edge.next[0];
                return Some((ed, edge.node[1]));
            }
        }
        while let Some(edge) = g.edges.get(self.next[1].index()) {
            let ed = self.next[1];
            self.next[1] = edge.next[1];
            if edge.node[0] != self.skip_start {
                return Some((ed, edge.node[0]));
            }
        }
        None
    }
    pub fn next_node<N, E, Ty: EdgeType>(
        &mut self,
        g: &Graph<N, E, Ty, Ix>,
    ) -> Option<NodeIndex<Ix>> {
        self.next(g).map(|t| t.1)
    }
    pub fn next_edge<N, E, Ty: EdgeType>(
        &mut self,
        g: &Graph<N, E, Ty, Ix>,
    ) -> Option<EdgeIndex<Ix>> {
        self.next(g).map(|t| t.0)
    }
}
/// Iterator over the node indices of a graph.
#[derive(Clone, Debug)]
pub struct NodeIndices<Ix = DefaultIx> {
    r: Range<usize>,
    ty: PhantomData<fn() -> Ix>,
}
impl<Ix: IndexType> Iterator for NodeIndices<Ix> {
    type Item = NodeIndex<Ix>;
    fn next(&mut self) -> Option<Self::Item> {
        self.r.next().map(node_index)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.r.size_hint()
    }
}
impl<Ix: IndexType> DoubleEndedIterator for NodeIndices<Ix> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.r.next_back().map(node_index)
    }
}
impl<Ix: IndexType> ExactSizeIterator for NodeIndices<Ix> {}
/// Iterator over the edge indices of a graph.
#[derive(Clone, Debug)]
pub struct EdgeIndices<Ix = DefaultIx> {
    r: Range<usize>,
    ty: PhantomData<fn() -> Ix>,
}
impl<Ix: IndexType> Iterator for EdgeIndices<Ix> {
    type Item = EdgeIndex<Ix>;
    fn next(&mut self) -> Option<Self::Item> {
        self.r.next().map(edge_index)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.r.size_hint()
    }
}
impl<Ix: IndexType> DoubleEndedIterator for EdgeIndices<Ix> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.r.next_back().map(edge_index)
    }
}
impl<Ix: IndexType> ExactSizeIterator for EdgeIndices<Ix> {}
/// Reference to a `Graph` edge.
#[derive(Debug)]
pub struct EdgeReference<'a, E: 'a, Ix = DefaultIx> {
    index: EdgeIndex<Ix>,
    node: [NodeIndex<Ix>; 2],
    weight: &'a E,
}
impl<'a, E, Ix: IndexType> Clone for EdgeReference<'a, E, Ix> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<'a, E, Ix: IndexType> Copy for EdgeReference<'a, E, Ix> {}
impl<'a, E, Ix: IndexType> PartialEq for EdgeReference<'a, E, Ix>
where
    E: PartialEq,
{
    fn eq(&self, rhs: &Self) -> bool {
        self.index == rhs.index && self.weight == rhs.weight
    }
}
impl<'a, N, E, Ty, Ix> IntoNodeReferences for &'a Graph<N, E, Ty, Ix>
where
    Ty: EdgeType,
    Ix: IndexType,
{
    type NodeRef = (NodeIndex<Ix>, &'a N);
    type NodeReferences = NodeReferences<'a, N, Ix>;
    fn node_references(self) -> Self::NodeReferences {
        NodeReferences {
            iter: self.nodes.iter().enumerate(),
        }
    }
}
/// Iterator over all nodes of a graph.
pub struct NodeReferences<'a, N: 'a, Ix: IndexType = DefaultIx> {
    iter: iter::Enumerate<slice::Iter<'a, Node<N, Ix>>>,
}
impl<'a, N, Ix> Iterator for NodeReferences<'a, N, Ix>
where
    Ix: IndexType,
{
    type Item = (NodeIndex<Ix>, &'a N);
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(i, node)| (node_index(i), &node.weight))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}
impl<'a, N, Ix> DoubleEndedIterator for NodeReferences<'a, N, Ix>
where
    Ix: IndexType,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back().map(|(i, node)| (node_index(i), &node.weight))
    }
}
impl<'a, N, Ix> ExactSizeIterator for NodeReferences<'a, N, Ix>
where
    Ix: IndexType,
{}
impl<'a, Ix, E> EdgeReference<'a, E, Ix>
where
    Ix: IndexType,
{
    /// Access the edge’s weight.
    ///
    /// **NOTE** that this method offers a longer lifetime
    /// than the trait (unfortunately they don't match yet).
    pub fn weight(&self) -> &'a E {
        self.weight
    }
}
impl<'a, Ix, E> EdgeRef for EdgeReference<'a, E, Ix>
where
    Ix: IndexType,
{
    type NodeId = NodeIndex<Ix>;
    type EdgeId = EdgeIndex<Ix>;
    type Weight = E;
    fn source(&self) -> Self::NodeId {
        self.node[0]
    }
    fn target(&self) -> Self::NodeId {
        self.node[1]
    }
    fn weight(&self) -> &E {
        self.weight
    }
    fn id(&self) -> Self::EdgeId {
        self.index
    }
}
/// Iterator over all edges of a graph.
pub struct EdgeReferences<'a, E: 'a, Ix: IndexType = DefaultIx> {
    iter: iter::Enumerate<slice::Iter<'a, Edge<E, Ix>>>,
}
impl<'a, E, Ix> Iterator for EdgeReferences<'a, E, Ix>
where
    Ix: IndexType,
{
    type Item = EdgeReference<'a, E, Ix>;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|(i, edge)| EdgeReference {
                index: edge_index(i),
                node: edge.node,
                weight: &edge.weight,
            })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}
impl<'a, E, Ix> DoubleEndedIterator for EdgeReferences<'a, E, Ix>
where
    Ix: IndexType,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter
            .next_back()
            .map(|(i, edge)| EdgeReference {
                index: edge_index(i),
                node: edge.node,
                weight: &edge.weight,
            })
    }
}
impl<'a, E, Ix> ExactSizeIterator for EdgeReferences<'a, E, Ix>
where
    Ix: IndexType,
{}
mod frozen;
#[cfg(feature = "stable_graph")]
pub mod stable_graph;
/// `Frozen` is a graph wrapper.
///
/// The `Frozen` only allows shared access (read-only) to the
/// underlying graph `G`, but it allows mutable access to its
/// node and edge weights.
///
/// This is used to ensure immutability of the graph's structure
/// while permitting weights to be both read and written.
///
/// See indexing implementations and the traits `Data` and `DataMap`
/// for read-write access to the graph's weights.
pub struct Frozen<'a, G: 'a>(&'a mut G);
#[cfg(test)]
mod tests_rug_80 {
    use super::*;
    use crate::graph::{IndexType, NodeIndex};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_80_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 5;
        let p0: usize = rug_fuzz_0;
        crate::graph_impl::node_index::<usize>(p0);
        let _rug_ed_tests_rug_80_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_81 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_edge_index() {
        let _rug_st_tests_rug_81_rrrruuuugggg_test_edge_index = 0;
        let rug_fuzz_0 = 42;
        let p0: usize = rug_fuzz_0;
        crate::graph_impl::edge_index::<DefaultIx>(p0);
        let _rug_ed_tests_rug_81_rrrruuuugggg_test_edge_index = 0;
    }
}
#[cfg(test)]
mod tests_rug_83 {
    use super::*;
    use crate::Direction;
    use crate::graph_impl::{Edge, EdgeIndex};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_83_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let mut p0: Vec<Edge<i32, u32>> = Vec::new();
        let mut p1: EdgeIndex<u32> = EdgeIndex::new(rug_fuzz_0);
        let mut p2 = Direction::Outgoing;
        crate::graph_impl::edges_walker_mut(&mut p0, p1, p2);
        let _rug_ed_tests_rug_83_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_84 {
    use super::*;
    use crate::graph_impl::swap_pair;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_84_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let mut p0: [i32; 2] = [rug_fuzz_0, rug_fuzz_1];
        swap_pair(p0);
        let _rug_ed_tests_rug_84_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_85 {
    use super::*;
    use crate::graph::IndexType;
    #[test]
    fn test_new() {
        let _rug_st_tests_rug_85_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = 42;
        let mut p0: usize = rug_fuzz_0;
        let result = usize::new(p0);
        debug_assert_eq!(result, 42);
        let _rug_ed_tests_rug_85_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_rug_86 {
    use super::*;
    use crate::graph::IndexType;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_86_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 5;
        let mut p0: usize = rug_fuzz_0;
        debug_assert_eq!(p0.index(), 5);
        let _rug_ed_tests_rug_86_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_87 {
    use super::*;
    use crate::graph_impl::IndexType;
    #[test]
    fn test_max() {
        let _rug_st_tests_rug_87_rrrruuuugggg_test_max = 0;
        let result: usize = <usize as IndexType>::max();
        debug_assert_eq!(result, usize::MAX);
        let _rug_ed_tests_rug_87_rrrruuuugggg_test_max = 0;
    }
}
#[cfg(test)]
mod tests_rug_88 {
    use super::*;
    use crate::graph::IndexType;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_88_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 5;
        let mut p0: usize = rug_fuzz_0;
        <u32>::new(p0);
        let _rug_ed_tests_rug_88_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_89 {
    use super::*;
    use crate::graph::IndexType;
    #[test]
    fn test_index() {
        let _rug_st_tests_rug_89_rrrruuuugggg_test_index = 0;
        let rug_fuzz_0 = 5;
        let p0: u32 = rug_fuzz_0;
        debug_assert_eq!(< u32 as IndexType > ::index(& p0), 5usize);
        let _rug_ed_tests_rug_89_rrrruuuugggg_test_index = 0;
    }
}
#[cfg(test)]
mod tests_rug_90 {
    use super::*;
    use crate::graph_impl::IndexType;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_90_rrrruuuugggg_test_rug = 0;
        let result = <u32 as IndexType>::max();
        debug_assert_eq!(result, ::std::u32::MAX);
        let _rug_ed_tests_rug_90_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_91 {
    use super::*;
    use crate::graph::IndexType;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_91_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: usize = rug_fuzz_0;
        <u16 as IndexType>::new(p0);
        let _rug_ed_tests_rug_91_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_92 {
    use super::*;
    use crate::graph_impl::IndexType;
    use crate::graph_impl;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_92_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u16 = rug_fuzz_0;
        <u16 as graph_impl::IndexType>::index(&p0);
        let _rug_ed_tests_rug_92_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_93 {
    use super::*;
    use crate::graph_impl::IndexType;
    #[test]
    fn test_max() {
        let _rug_st_tests_rug_93_rrrruuuugggg_test_max = 0;
        let max_val: u16 = <u16 as IndexType>::max();
        debug_assert_eq!(max_val, std::u16::MAX);
        let _rug_ed_tests_rug_93_rrrruuuugggg_test_max = 0;
    }
}
#[cfg(test)]
mod tests_rug_94 {
    use super::*;
    use crate::graph::IndexType;
    #[test]
    fn test_new() {
        let _rug_st_tests_rug_94_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = 10;
        let p0: usize = rug_fuzz_0;
        <u8 as IndexType>::new(p0);
        let _rug_ed_tests_rug_94_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_rug_95 {
    use super::*;
    use crate::graph::IndexType;
    #[test]
    fn test_index() {
        let _rug_st_tests_rug_95_rrrruuuugggg_test_index = 0;
        let rug_fuzz_0 = 5;
        let mut p0: u8 = rug_fuzz_0;
        <u8 as IndexType>::index(&p0);
        let _rug_ed_tests_rug_95_rrrruuuugggg_test_index = 0;
    }
}
#[cfg(test)]
mod tests_rug_96 {
    use super::*;
    use crate::graph_impl::IndexType;
    #[test]
    fn test_max() {
        let _rug_st_tests_rug_96_rrrruuuugggg_test_max = 0;
        let max_value: u8 = <u8 as IndexType>::max();
        debug_assert_eq!(max_value, std::u8::MAX);
        let _rug_ed_tests_rug_96_rrrruuuugggg_test_max = 0;
    }
}
#[cfg(test)]
mod tests_rug_97 {
    use super::*;
    use crate::graph::NodeIndex;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_97_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 5;
        let mut p0: usize = rug_fuzz_0;
        NodeIndex::<usize>::new(p0);
        let _rug_ed_tests_rug_97_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_99 {
    use super::*;
    use crate::graph::NodeIndex;
    use crate::graph::IndexType;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_99_rrrruuuugggg_test_rug = 0;
        NodeIndex::<usize>::end();
        let _rug_ed_tests_rug_99_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_113 {
    use super::*;
    use crate::graph::{node_index, DiGraph, Graph, NodeIndex};
    use crate::visit::{EdgeRef, IntoEdgeReferences};
    #[test]
    fn test_new() {
        let _rug_st_tests_rug_113_rrrruuuugggg_test_new = 0;
        let graph: Graph<&str, &str> = Graph::<&str, &str>::new();
        debug_assert_eq!(graph.node_count(), 0);
        debug_assert_eq!(graph.edge_count(), 0);
        let _rug_ed_tests_rug_113_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_rug_116 {
    use super::*;
    use crate::graph::{Graph, NodeIndex};
    #[test]
    fn test_node_count() {
        let _rug_st_tests_rug_116_rrrruuuugggg_test_node_count = 0;
        let rug_fuzz_0 = "A";
        let rug_fuzz_1 = "B";
        let mut graph: Graph<&str, &str> = Graph::new();
        let node_a = graph.add_node(rug_fuzz_0);
        let node_b = graph.add_node(rug_fuzz_1);
        debug_assert_eq!(graph.node_count(), 2);
        let _rug_ed_tests_rug_116_rrrruuuugggg_test_node_count = 0;
    }
}
#[cfg(test)]
mod tests_rug_117 {
    use super::*;
    use crate::graph::{node_index, DiGraph, NodeIndex};
    #[test]
    fn test_edge_count() {
        let _rug_st_tests_rug_117_rrrruuuugggg_test_edge_count = 0;
        let rug_fuzz_0 = "Node A";
        let rug_fuzz_1 = "Node B";
        let rug_fuzz_2 = "Edge";
        let mut graph: DiGraph<&str, &str> = DiGraph::new();
        let node_a = graph.add_node(rug_fuzz_0);
        let node_b = graph.add_node(rug_fuzz_1);
        graph.add_edge(node_a, node_b, rug_fuzz_2);
        debug_assert_eq!(graph.edge_count(), 1);
        let _rug_ed_tests_rug_117_rrrruuuugggg_test_edge_count = 0;
    }
}
#[cfg(test)]
mod tests_rug_118 {
    use super::*;
    use crate::graph::{DiGraph, NodeIndex};
    use crate::visit::EdgeRef;
    #[test]
    fn test_is_directed() {
        let _rug_st_tests_rug_118_rrrruuuugggg_test_is_directed = 0;
        let rug_fuzz_0 = "A";
        let rug_fuzz_1 = "B";
        let rug_fuzz_2 = "AB";
        let rug_fuzz_3 = "A";
        let rug_fuzz_4 = "B";
        let rug_fuzz_5 = "AB";
        let mut graph: DiGraph<&str, &str> = DiGraph::new();
        let a = graph.add_node(rug_fuzz_0);
        let b = graph.add_node(rug_fuzz_1);
        graph.add_edge(a, b, rug_fuzz_2);
        debug_assert_eq!(graph.is_directed(), true);
        let mut graph: DiGraph<&str, &str> = DiGraph::new();
        let a = graph.add_node(rug_fuzz_3);
        let b = graph.add_node(rug_fuzz_4);
        graph.add_edge(a, b, rug_fuzz_5);
        debug_assert_eq!(graph.is_directed(), false);
        let _rug_ed_tests_rug_118_rrrruuuugggg_test_is_directed = 0;
    }
}
#[cfg(test)]
mod tests_rug_123 {
    use super::*;
    use crate::graph::{Graph, NodeIndex, EdgeIndex};
    use crate::data::{DataMap, DataMapMut};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_123_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut graph: Graph<(), _, _, _> = Graph::new();
        let node1 = graph.add_node(());
        let node2 = graph.add_node(());
        let weight = rug_fuzz_0;
        graph.update_edge(node1, node2, weight);
        let _rug_ed_tests_rug_123_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_124 {
    use super::*;
    use crate::graph::{Graph, node_index, EdgeIndex};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_124_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let mut p0 = Graph::<u32, &str, Directed>::new();
        let p1 = EdgeIndex::<u32>::new(rug_fuzz_0);
        p0.edge_weight(p1);
        let _rug_ed_tests_rug_124_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_126 {
    use super::*;
    use crate::graph::{Graph, node_index};
    use crate::data::DataMap;
    use crate::prelude::{NodeIndex, EdgeIndex};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_126_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "Node A";
        let rug_fuzz_1 = "Node B";
        let rug_fuzz_2 = 1;
        let mut graph = Graph::<_, _, _, _>::new();
        let node_a = graph.add_node(rug_fuzz_0);
        let node_b = graph.add_node(rug_fuzz_1);
        let edge_ab = graph.add_edge(node_a, node_b, rug_fuzz_2);
        debug_assert_eq!(Some((node_a, node_b)), graph.edge_endpoints(edge_ab));
        let _rug_ed_tests_rug_126_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_135 {
    use super::*;
    use crate::{Direction, EdgeDirection, graph::{DiGraph, NodeIndex}};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_135_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let mut p0: DiGraph<(), ()> = DiGraph::new();
        let mut p1: NodeIndex = NodeIndex::new(rug_fuzz_0);
        let mut p2: EdgeDirection = Direction::Outgoing;
        p0.edges_directed(p1, p2);
        let _rug_ed_tests_rug_135_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_137 {
    use super::*;
    use crate::graph::IndexType;
    use crate::graph::{NodeIndex, Graph};
    #[test]
    fn test_contains_edge() {
        let _rug_st_tests_rug_137_rrrruuuugggg_test_contains_edge = 0;
        let rug_fuzz_0 = "Node A";
        let rug_fuzz_1 = "Node B";
        let rug_fuzz_2 = 10;
        let mut g = Graph::<_, _, Directed>::new();
        let a = g.add_node(rug_fuzz_0);
        let b = g.add_node(rug_fuzz_1);
        g.add_edge(a, b, rug_fuzz_2);
        debug_assert_eq!(g.contains_edge(a, b), true);
        debug_assert_eq!(g.contains_edge(b, a), false);
        let _rug_ed_tests_rug_137_rrrruuuugggg_test_contains_edge = 0;
    }
}
#[cfg(test)]
mod tests_rug_143 {
    use super::*;
    use crate::{graph::node_index, Graph};
    #[test]
    fn test_node_indices() {
        let _rug_st_tests_rug_143_rrrruuuugggg_test_node_indices = 0;
        let rug_fuzz_0 = "book";
        let rug_fuzz_1 = "book";
        let mut graph = Graph::<&str, i32>::new();
        let node_book = graph.add_node(rug_fuzz_0);
        let index = graph.node_indices().find(|i| graph[*i] == rug_fuzz_1).unwrap();
        debug_assert_eq!(index, node_book);
        let _rug_ed_tests_rug_143_rrrruuuugggg_test_node_indices = 0;
    }
}
#[cfg(test)]
mod tests_rug_144 {
    use super::*;
    use crate::graph::{Graph, NodeWeightsMut};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_144_rrrruuuugggg_test_rug = 0;
        let mut g = Graph::<i32, (), Directed>::new();
        Graph::<i32, (), Directed>::node_weights_mut(&mut g);
        let _rug_ed_tests_rug_144_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_145 {
    use super::*;
    use crate::graph::{DiGraph, EdgeIndices};
    use std::marker::PhantomData;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_145_rrrruuuugggg_test_rug = 0;
        let mut p0: DiGraph<&str, &str> = DiGraph::new();
        p0.edge_indices();
        let _rug_ed_tests_rug_145_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_147 {
    use super::*;
    use crate::graph::node_index;
    use crate::graph::DiGraph;
    use crate::data::{DataMap, DataMapMut};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_147_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "Node A";
        let rug_fuzz_1 = "Node B";
        let rug_fuzz_2 = "Node C";
        let rug_fuzz_3 = "Edge 1";
        let rug_fuzz_4 = "Edge 2";
        let mut graph: DiGraph<&str, &str> = DiGraph::new();
        let a = graph.add_node(rug_fuzz_0);
        let b = graph.add_node(rug_fuzz_1);
        let c = graph.add_node(rug_fuzz_2);
        graph.add_edge(a, b, rug_fuzz_3);
        graph.add_edge(b, c, rug_fuzz_4);
        let p0 = &mut graph;
        p0.edge_weights_mut();
        let _rug_ed_tests_rug_147_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_148 {
    use super::*;
    use crate::{
        graph::{Node, Graph},
        data::DataMap,
    };
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_148_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut graph: Graph<i32, &str> = Graph::new();
        let node = graph.add_node(rug_fuzz_0);
        graph.raw_nodes();
        let _rug_ed_tests_rug_148_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_149 {
    use crate::graph::{DiGraph, Graph, NodeIndex};
    use crate::data::{DataMap, DataMapMut};
    use crate::prelude::EdgeIndex;
    use super::*;
    #[test]
    fn test_raw_edges() {
        let _rug_st_tests_rug_149_rrrruuuugggg_test_raw_edges = 0;
        let mut graph = Graph::<(), ()>::new();
        let node_a = graph.add_node(());
        let node_b = graph.add_node(());
        let edge_ab = graph.add_edge(node_a, node_b, ());
        graph.raw_edges();
        let _rug_ed_tests_rug_149_rrrruuuugggg_test_raw_edges = 0;
    }
}
#[cfg(test)]
mod tests_rug_150 {
    use super::*;
    use crate::graph::{DiGraph, node_index, EdgeIndex};
    #[test]
    fn test_into_nodes_edges() {
        let _rug_st_tests_rug_150_rrrruuuugggg_test_into_nodes_edges = 0;
        let rug_fuzz_0 = "Node A";
        let rug_fuzz_1 = "Node B";
        let rug_fuzz_2 = "Edge AB";
        let mut p0: DiGraph<&str, &str> = DiGraph::new();
        let node_a = p0.add_node(rug_fuzz_0);
        let node_b = p0.add_node(rug_fuzz_1);
        let edge_ab = p0.add_edge(node_a, node_b, rug_fuzz_2);
        let (nodes, edges) = p0.into_nodes_edges();
        debug_assert_eq!(nodes.len(), 2);
        debug_assert_eq!(edges.len(), 1);
        let _rug_ed_tests_rug_150_rrrruuuugggg_test_into_nodes_edges = 0;
    }
}
#[cfg(test)]
mod tests_rug_154 {
    use super::*;
    use crate::graph::{Graph, NodeIndex, EdgeIndex};
    use crate::graph::IndexType;
    use crate::visit::NodeIndexable;
    use crate::data::{DataMap, DataMapMut};
    #[test]
    fn test_reverse() {
        let _rug_st_tests_rug_154_rrrruuuugggg_test_reverse = 0;
        let mut graph: Graph<(), (), Directed, u32> = Graph::new();
        let node_a = graph.add_node(());
        let node_b = graph.add_node(());
        let edge_ab = graph.add_edge(node_a, node_b, ());
        <Graph<(), (), Directed, u32>>::reverse(&mut graph);
        debug_assert_eq!(graph.edge_endpoints(edge_ab).unwrap(), (node_b, node_a));
        let _rug_ed_tests_rug_154_rrrruuuugggg_test_reverse = 0;
    }
}
#[cfg(test)]
mod tests_rug_155 {
    use super::*;
    use crate::graph::{Graph, NodeIndex};
    use crate::visit::NodeIndexable;
    #[test]
    fn test_clear() {
        let _rug_st_tests_rug_155_rrrruuuugggg_test_clear = 0;
        let rug_fuzz_0 = "Node 1";
        let rug_fuzz_1 = "Node 2";
        let rug_fuzz_2 = "Edge 1-2";
        let mut graph: Graph<&str, &str> = Graph::new();
        let n1 = graph.add_node(rug_fuzz_0);
        let n2 = graph.add_node(rug_fuzz_1);
        graph.add_edge(n1, n2, rug_fuzz_2);
        debug_assert_eq!(graph.node_count(), 2);
        debug_assert_eq!(graph.edge_count(), 1);
        graph.clear();
        debug_assert_eq!(graph.node_count(), 0);
        debug_assert_eq!(graph.edge_count(), 0);
        let _rug_ed_tests_rug_155_rrrruuuugggg_test_clear = 0;
    }
}
#[cfg(test)]
mod tests_rug_156 {
    use crate::prelude::*;
    #[test]
    fn test_clear_edges() {
        let _rug_st_tests_rug_156_rrrruuuugggg_test_clear_edges = 0;
        let mut graph: Graph<(), (), Undirected> = Graph::default();
        graph.clear_edges();
        let _rug_ed_tests_rug_156_rrrruuuugggg_test_clear_edges = 0;
    }
}
#[cfg(test)]
mod tests_rug_157 {
    use super::*;
    use crate::graph::node_index;
    use crate::prelude::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_157_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 20;
        let rug_fuzz_2 = "Node 1";
        let rug_fuzz_3 = "Node 2";
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 1;
        let rug_fuzz_6 = 100;
        let mut p0 = Graph::<
            _,
            _,
            Directed,
            usize,
        >::with_capacity(rug_fuzz_0, rug_fuzz_1);
        p0.add_node(rug_fuzz_2);
        p0.add_node(rug_fuzz_3);
        p0.add_edge(node_index(rug_fuzz_4), node_index(rug_fuzz_5), rug_fuzz_6);
        debug_assert_eq!(p0.capacity(), (10, 20));
        let _rug_ed_tests_rug_157_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_158 {
    use super::*;
    use crate::graph::{Graph, node_index};
    #[test]
    fn test_reserve_nodes() {
        let _rug_st_tests_rug_158_rrrruuuugggg_test_reserve_nodes = 0;
        let rug_fuzz_0 = 10;
        let mut graph: Graph<(), ()> = Graph::new();
        let additional = rug_fuzz_0;
        graph.reserve_nodes(additional);
        let _rug_ed_tests_rug_158_rrrruuuugggg_test_reserve_nodes = 0;
    }
}
#[cfg(test)]
mod tests_rug_159 {
    use super::*;
    use crate::graph::{Graph, NodeIndex};
    #[test]
    fn test_reserve_edges() {
        let _rug_st_tests_rug_159_rrrruuuugggg_test_reserve_edges = 0;
        let rug_fuzz_0 = 10;
        let mut graph = Graph::<&str, &str>::new();
        let additional = rug_fuzz_0;
        graph.reserve_edges(additional);
        let _rug_ed_tests_rug_159_rrrruuuugggg_test_reserve_edges = 0;
    }
}
#[cfg(test)]
mod tests_rug_160 {
    use super::*;
    use crate::graph::node_index;
    use crate::graph::Graph;
    #[test]
    fn test_reserve_exact_nodes() {
        let _rug_st_tests_rug_160_rrrruuuugggg_test_reserve_exact_nodes = 0;
        let rug_fuzz_0 = 5;
        let mut g: Graph<(), ()> = Graph::new();
        let additional = rug_fuzz_0;
        g.reserve_exact_nodes(additional);
        let _rug_ed_tests_rug_160_rrrruuuugggg_test_reserve_exact_nodes = 0;
    }
}
#[cfg(test)]
mod tests_rug_161 {
    use super::*;
    use crate::graph::{Graph, node_index};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_161_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut graph: Graph<&str, &str> = Graph::new();
        let additional = rug_fuzz_0;
        graph.reserve_exact_edges(additional);
        let _rug_ed_tests_rug_161_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_162 {
    use super::*;
    use crate::graph::{Graph, NodeIndex};
    #[test]
    fn test_shrink_to_fit_nodes() {
        let _rug_st_tests_rug_162_rrrruuuugggg_test_shrink_to_fit_nodes = 0;
        let rug_fuzz_0 = "Node A";
        let rug_fuzz_1 = "Node B";
        let mut graph: Graph<&str, &str> = Graph::new();
        let node_a = graph.add_node(rug_fuzz_0);
        let node_b = graph.add_node(rug_fuzz_1);
        graph.shrink_to_fit_nodes();
        let _rug_ed_tests_rug_162_rrrruuuugggg_test_shrink_to_fit_nodes = 0;
    }
}
#[cfg(test)]
mod tests_rug_163 {
    use super::*;
    use crate::graph::{Graph, NodeIndex};
    #[test]
    fn test_shrink_to_fit_edges() {
        let _rug_st_tests_rug_163_rrrruuuugggg_test_shrink_to_fit_edges = 0;
        let rug_fuzz_0 = "A";
        let rug_fuzz_1 = "edge1";
        let mut graph: Graph<&str, &str> = Graph::new();
        let node = graph.add_node(rug_fuzz_0);
        let edge = graph.add_edge(node, node, rug_fuzz_1);
        graph.shrink_to_fit_edges();
        let _rug_ed_tests_rug_163_rrrruuuugggg_test_shrink_to_fit_edges = 0;
    }
}
#[cfg(test)]
mod tests_rug_164 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_164_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 1;
        let mut p0: Graph<(), ()> = Graph::new();
        p0.add_node(());
        p0.add_node(());
        p0.add_edge(NodeIndex::new(rug_fuzz_0), NodeIndex::new(rug_fuzz_1), ());
        p0.shrink_to_fit();
        let _rug_ed_tests_rug_164_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_167 {
    use super::*;
    use crate::graph::{NodeIndex, Graph};
    use crate::visit::{NodeIndexable, IntoNodeIdentifiers};
    #[test]
    fn test_from_edges() {
        let _rug_st_tests_rug_167_rrrruuuugggg_test_from_edges = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 2;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 3;
        let rug_fuzz_6 = 1;
        let rug_fuzz_7 = 2;
        let rug_fuzz_8 = 1;
        let rug_fuzz_9 = 3;
        let mut p0 = std::collections::HashSet::new();
        p0.insert((rug_fuzz_0, rug_fuzz_1));
        p0.insert((rug_fuzz_2, rug_fuzz_3));
        p0.insert((rug_fuzz_4, rug_fuzz_5));
        p0.insert((rug_fuzz_6, rug_fuzz_7));
        p0.insert((rug_fuzz_8, rug_fuzz_9));
        let gr = <Graph<(), i32>>::from_edges(&p0);
        let _rug_ed_tests_rug_167_rrrruuuugggg_test_from_edges = 0;
    }
}
#[cfg(test)]
mod tests_rug_168 {
    use super::*;
    use crate::prelude::*;
    use crate::graph::{node_index, NodeIndex};
    use std::collections::LinkedList;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_168_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 1;
        let mut p0: Graph<(), (), Directed, u32> = Graph::new();
        let mut p1: LinkedList<(NodeIndex<u32>, NodeIndex<u32>, ())> = LinkedList::new();
        let source_node = node_index(rug_fuzz_0);
        let target_node = node_index(rug_fuzz_1);
        p1.push_back((source_node, target_node, ()));
        p0.extend_with_edges(p1);
        let _rug_ed_tests_rug_168_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_177 {
    use super::*;
    use crate::visit::IntoEdges;
    use crate::graph::{Graph, NodeIndex};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_177_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 100;
        let mut p0: Graph<i32, &str, Directed, u32> = Graph::new();
        let p1: NodeIndex<u32> = p0.add_node(rug_fuzz_0);
        p0.edges(p1);
        let _rug_ed_tests_rug_177_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_178 {
    use super::*;
    use crate::{Direction, Graph, visit::{IntoEdgesDirected, IntoEdges}};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_178_rrrruuuugggg_test_rug = 0;
        let mut graph: Graph<(), ()> = Graph::new();
        let node_a = graph.add_node(());
        let node_b = graph.add_node(());
        let direction = Direction::Outgoing;
        graph.edges_directed(node_a, direction);
        let _rug_ed_tests_rug_178_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_187 {
    use super::*;
    use crate::graph::node_index;
    use crate::data::DataMap;
    use crate::graph::{Graph, NodeIndex};
    #[test]
    fn test_index_mut() {
        let _rug_st_tests_rug_187_rrrruuuugggg_test_index_mut = 0;
        let rug_fuzz_0 = 0;
        let mut graph: Graph<i32, &'static str> = Graph::new();
        let node_index = node_index(rug_fuzz_0);
        graph.index_mut(node_index);
        let _rug_ed_tests_rug_187_rrrruuuugggg_test_index_mut = 0;
    }
}
#[cfg(test)]
mod tests_rug_190 {
    use super::*;
    use crate::graph::NodeIndex;
    use crate::graph::IndexType;
    use crate::prelude::*;
    #[test]
    fn test_default() {
        let _rug_st_tests_rug_190_rrrruuuugggg_test_default = 0;
        let default_graph: Graph<i32, i32> = <Graph<i32, i32>>::default();
        debug_assert_eq!(default_graph.node_count(), 0);
        debug_assert_eq!(default_graph.edge_count(), 0);
        let _rug_ed_tests_rug_190_rrrruuuugggg_test_default = 0;
    }
}
#[cfg(test)]
mod tests_rug_191 {
    use super::*;
    use crate::graph::GraphIndex;
    use crate::graph::{NodeIndex, IndexType};
    #[test]
    fn test_index() {
        let _rug_st_tests_rug_191_rrrruuuugggg_test_index = 0;
        let rug_fuzz_0 = 0;
        let mut p0: NodeIndex<usize> = NodeIndex::new(rug_fuzz_0);
        p0.index();
        let _rug_ed_tests_rug_191_rrrruuuugggg_test_index = 0;
    }
}
#[cfg(test)]
mod tests_rug_192 {
    use super::*;
    use crate::graph::{GraphIndex, NodeIndex};
    #[test]
    fn test_is_node_index() {
        let _rug_st_tests_rug_192_rrrruuuugggg_test_is_node_index = 0;
        let result = NodeIndex::<u32>::is_node_index();
        debug_assert_eq!(result, true);
        let _rug_ed_tests_rug_192_rrrruuuugggg_test_is_node_index = 0;
    }
}
#[cfg(test)]
mod tests_rug_193 {
    use super::*;
    use crate::graph::{GraphIndex, EdgeIndex};
    use crate::graph::EdgeIndex as petgraph_EdgeIndex;
    use crate::algo::DfsSpace;
    use crate::visit::{Dfs, IntoNeighbors};
    use crate::data::{Build, FromElements};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_193_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let mut p0: petgraph_EdgeIndex<u32> = petgraph_EdgeIndex::new(rug_fuzz_0);
        p0.index();
        let _rug_ed_tests_rug_193_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_202 {
    use super::*;
    use crate::graph::EdgeIndices;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_202_rrrruuuugggg_test_rug = 0;
        let mut p0: EdgeIndices<usize> = unimplemented!();
        p0.next();
        let _rug_ed_tests_rug_202_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_207 {
    use super::*;
    use crate::{visit, Graph};
    #[test]
    fn test_node_references() {
        let _rug_st_tests_rug_207_rrrruuuugggg_test_node_references = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 20;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 1;
        let mut graph: Graph<i32, &'static str> = Graph::new();
        let node_a = graph.add_node(rug_fuzz_0);
        let node_b = graph.add_node(rug_fuzz_1);
        let node_references = <&Graph<
            i32,
            &'static str,
        > as visit::IntoNodeReferences>::node_references(&graph);
        for (node, &index) in node_references {
            debug_assert!(node == node_a || node == node_b);
            debug_assert!(index == rug_fuzz_2 || index == rug_fuzz_3);
        }
        let _rug_ed_tests_rug_207_rrrruuuugggg_test_node_references = 0;
    }
}
#[cfg(test)]
mod tests_rug_210 {
    use super::*;
    use crate::{graph, graph_impl};
    use crate::prelude::*;
    use std::iter::DoubleEndedIterator;
    #[test]
    fn test_next_back() {
        let _rug_st_tests_rug_210_rrrruuuugggg_test_next_back = 0;
        let node_references: graph_impl::NodeReferences<'_, u32, usize> = unimplemented!(

        );
        let result = <graph_impl::NodeReferences<
            '_,
            u32,
            usize,
        > as DoubleEndedIterator>::next_back(&mut node_references);
        let _rug_ed_tests_rug_210_rrrruuuugggg_test_next_back = 0;
    }
}
#[cfg(test)]
mod tests_rug_218 {
    use super::*;
    use crate::graph::EdgeReferences;
    use crate::graph::edge_index;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_218_rrrruuuugggg_test_rug = 0;
        let mut p0: EdgeReferences<'_, String, usize> = unimplemented!();
        p0.next_back();
        let _rug_ed_tests_rug_218_rrrruuuugggg_test_rug = 0;
    }
}
