//! Graph traits and graph traversals.
//!
//! ### The `Into-` Traits
//!
//! Graph traits like [`IntoNeighbors`][in] create iterators and use the same
//! pattern that `IntoIterator` does: the trait takes a reference to a graph,
//! and produces an iterator. These traits are quite composable, but with the
//! limitation that they only use shared references to graphs.
//!
//! ### Graph Traversal
//!
//! [`Dfs`](struct.Dfs.html), [`Bfs`][bfs], [`DfsPostOrder`][dfspo] and
//! [`Topo`][topo]  are basic visitors and they use “walker” methods: the
//! visitors don't hold the graph as borrowed during traversal, only for the
//! `.next()` call on the walker. They can be converted to iterators
//! through the [`Walker`][w] trait.
//!
//! There is also the callback based traversal [`depth_first_search`][dfs].
//!
//! [bfs]: struct.Bfs.html
//! [dfspo]: struct.DfsPostOrder.html
//! [topo]: struct.Topo.html
//! [dfs]: fn.depth_first_search.html
//! [w]: trait.Walker.html
//!
//! ### Other Graph Traits
//!
//! The traits are rather loosely coupled at the moment (which is intentional,
//! but will develop a bit), and there are traits missing that could be added.
//!
//! Not much is needed to be able to use the visitors on a graph. A graph
//! needs to define [`GraphBase`][gb], [`IntoNeighbors`][in] and
//! [`Visitable`][vis] as a minimum.
//!
//! [gb]: trait.GraphBase.html
//! [in]: trait.IntoNeighbors.html
//! [vis]: trait.Visitable.html
//!
pub use self::filter::*;
pub use self::reversed::*;
#[macro_use]
mod macros;
mod dfsvisit;
mod traversal;
pub use self::dfsvisit::*;
pub use self::traversal::*;
use fixedbitset::FixedBitSet;
use std::collections::HashSet;
use std::hash::{BuildHasher, Hash};
use super::{graph, EdgeType};
use crate::graph::NodeIndex;
#[cfg(feature = "graphmap")]
use crate::prelude::GraphMap;
#[cfg(feature = "stable_graph")]
use crate::prelude::StableGraph;
use crate::prelude::{Direction, Graph};
use crate::graph::Frozen;
use crate::graph::IndexType;
#[cfg(feature = "stable_graph")]
use crate::stable_graph;
#[cfg(feature = "graphmap")]
use crate::graphmap::{self, NodeTrait};
trait_template! {
    #[doc = " Base graph trait: defines the associated node identifier and"] #[doc =
    " edge identifier types."] pub trait GraphBase { @ escape[type NodeId] @ escape[type
    EdgeId] @ section nodelegate #[doc = " edge identifier"] type EdgeId : Copy +
    PartialEq; #[doc = " node identifier"] type NodeId : Copy + PartialEq; }
}
GraphBase! {
    delegate_impl[]
}
GraphBase! {
    delegate_impl[['a, G], G, &'a mut G, deref]
}
/// A copyable reference to a graph.
pub trait GraphRef: Copy + GraphBase {}
impl<'a, G> GraphRef for &'a G
where
    G: GraphBase,
{}
impl<'a, G> GraphBase for Frozen<'a, G>
where
    G: GraphBase,
{
    type NodeId = G::NodeId;
    type EdgeId = G::EdgeId;
}
#[cfg(feature = "stable_graph")]
impl<'a, N, E: 'a, Ty, Ix> IntoNeighbors for &'a StableGraph<N, E, Ty, Ix>
where
    Ty: EdgeType,
    Ix: IndexType,
{
    type Neighbors = stable_graph::Neighbors<'a, E, Ix>;
    fn neighbors(self, n: Self::NodeId) -> Self::Neighbors {
        (*self).neighbors(n)
    }
}
#[cfg(feature = "graphmap")]
impl<'a, N: 'a, E, Ty> IntoNeighbors for &'a GraphMap<N, E, Ty>
where
    N: Copy + Ord + Hash,
    Ty: EdgeType,
{
    type Neighbors = graphmap::Neighbors<'a, N, Ty>;
    fn neighbors(self, n: Self::NodeId) -> Self::Neighbors {
        self.neighbors(n)
    }
}
trait_template! {
    #[doc = " Access to the neighbors of each node"] #[doc = ""] #[doc =
    " The neighbors are, depending on the graph’s edge type:"] #[doc = ""] #[doc =
    " - `Directed`: All targets of edges from `a`."] #[doc =
    " - `Undirected`: All other endpoints of edges connected to `a`."] pub trait
    IntoNeighbors : GraphRef { @ section type type Neighbors : Iterator < Item =
    Self::NodeId >; @ section self #[doc =
    " Return an iterator of the neighbors of node `a`."] fn neighbors(self : Self, a :
    Self::NodeId) -> Self::Neighbors; }
}
IntoNeighbors! {
    delegate_impl[]
}
trait_template! {
    #[doc = " Access to the neighbors of each node, through incoming or outgoing edges."]
    #[doc = ""] #[doc =
    " Depending on the graph’s edge type, the neighbors of a given directionality"]
    #[doc = " are:"] #[doc = ""] #[doc =
    " - `Directed`, `Outgoing`: All targets of edges from `a`."] #[doc =
    " - `Directed`, `Incoming`: All sources of edges to `a`."] #[doc =
    " - `Undirected`: All other endpoints of edges connected to `a`."] pub trait
    IntoNeighborsDirected : IntoNeighbors { @ section type type NeighborsDirected :
    Iterator < Item = Self::NodeId >; @ section self fn neighbors_directed(self, n :
    Self::NodeId, d : Direction) -> Self::NeighborsDirected; }
}
impl<'a, N, E: 'a, Ty, Ix> IntoNeighbors for &'a Graph<N, E, Ty, Ix>
where
    Ty: EdgeType,
    Ix: IndexType,
{
    type Neighbors = graph::Neighbors<'a, E, Ix>;
    fn neighbors(self, n: graph::NodeIndex<Ix>) -> graph::Neighbors<'a, E, Ix> {
        Graph::neighbors(self, n)
    }
}
impl<'a, N, E: 'a, Ty, Ix> IntoNeighborsDirected for &'a Graph<N, E, Ty, Ix>
where
    Ty: EdgeType,
    Ix: IndexType,
{
    type NeighborsDirected = graph::Neighbors<'a, E, Ix>;
    fn neighbors_directed(
        self,
        n: graph::NodeIndex<Ix>,
        d: Direction,
    ) -> graph::Neighbors<'a, E, Ix> {
        Graph::neighbors_directed(self, n, d)
    }
}
#[cfg(feature = "stable_graph")]
impl<'a, N, E: 'a, Ty, Ix> IntoNeighborsDirected for &'a StableGraph<N, E, Ty, Ix>
where
    Ty: EdgeType,
    Ix: IndexType,
{
    type NeighborsDirected = stable_graph::Neighbors<'a, E, Ix>;
    fn neighbors_directed(
        self,
        n: graph::NodeIndex<Ix>,
        d: Direction,
    ) -> Self::NeighborsDirected {
        StableGraph::neighbors_directed(self, n, d)
    }
}
#[cfg(feature = "graphmap")]
impl<'a, N: 'a, E, Ty> IntoNeighborsDirected for &'a GraphMap<N, E, Ty>
where
    N: Copy + Ord + Hash,
    Ty: EdgeType,
{
    type NeighborsDirected = graphmap::NeighborsDirected<'a, N, Ty>;
    fn neighbors_directed(self, n: N, dir: Direction) -> Self::NeighborsDirected {
        self.neighbors_directed(n, dir)
    }
}
trait_template! {
    #[doc = " Access to the edges of each node."] #[doc = ""] #[doc =
    " The edges are, depending on the graph’s edge type:"] #[doc = ""] #[doc =
    " - `Directed`: All edges from `a`."] #[doc =
    " - `Undirected`: All edges connected to `a`."] #[doc = ""] #[doc =
    " This is an extended version of the trait `IntoNeighbors`; the former"] #[doc =
    " only iterates over the target node identifiers, while this trait"] #[doc =
    " yields edge references (trait [`EdgeRef`][er])."] #[doc = ""] #[doc =
    " [er]: trait.EdgeRef.html"] pub trait IntoEdges : IntoEdgeReferences + IntoNeighbors
    { @ section type type Edges : Iterator < Item = Self::EdgeRef >; @ section self fn
    edges(self, a : Self::NodeId) -> Self::Edges; }
}
IntoEdges! {
    delegate_impl[]
}
trait_template! {
    #[doc = " Access to all edges of each node, in the specified direction."] #[doc = ""]
    #[doc = " The edges are, depending on the direction and the graph’s edge type:"]
    #[doc = ""] #[doc = ""] #[doc = " - `Directed`, `Outgoing`: All edges from `a`."]
    #[doc = " - `Directed`, `Incoming`: All edges to `a`."] #[doc =
    " - `Undirected`, `Outgoing`: All edges connected to `a`, with `a` being the source of each edge."]
    #[doc =
    " - `Undirected`, `Incoming`: All edges connected to `a`, with `a` being the target of each edge."]
    #[doc = ""] #[doc =
    " This is an extended version of the trait `IntoNeighborsDirected`; the former"]
    #[doc = " only iterates over the target node identifiers, while this trait"] #[doc =
    " yields edge references (trait [`EdgeRef`][er])."] #[doc = ""] #[doc =
    " [er]: trait.EdgeRef.html"] pub trait IntoEdgesDirected : IntoEdges +
    IntoNeighborsDirected { @ section type type EdgesDirected : Iterator < Item =
    Self::EdgeRef >; @ section self fn edges_directed(self, a : Self::NodeId, dir :
    Direction) -> Self::EdgesDirected; }
}
IntoEdgesDirected! {
    delegate_impl[]
}
trait_template! {
    #[doc = " Access to the sequence of the graph’s `NodeId`s."] pub trait
    IntoNodeIdentifiers : GraphRef { @ section type type NodeIdentifiers : Iterator <
    Item = Self::NodeId >; @ section self fn node_identifiers(self) ->
    Self::NodeIdentifiers; }
}
IntoNodeIdentifiers! {
    delegate_impl[]
}
impl<'a, N, E: 'a, Ty, Ix> IntoNodeIdentifiers for &'a Graph<N, E, Ty, Ix>
where
    Ty: EdgeType,
    Ix: IndexType,
{
    type NodeIdentifiers = graph::NodeIndices<Ix>;
    fn node_identifiers(self) -> graph::NodeIndices<Ix> {
        Graph::node_indices(self)
    }
}
impl<N, E, Ty, Ix> NodeCount for Graph<N, E, Ty, Ix>
where
    Ty: EdgeType,
    Ix: IndexType,
{
    fn node_count(&self) -> usize {
        self.node_count()
    }
}
#[cfg(feature = "stable_graph")]
impl<'a, N, E: 'a, Ty, Ix> IntoNodeIdentifiers for &'a StableGraph<N, E, Ty, Ix>
where
    Ty: EdgeType,
    Ix: IndexType,
{
    type NodeIdentifiers = stable_graph::NodeIndices<'a, N, Ix>;
    fn node_identifiers(self) -> Self::NodeIdentifiers {
        StableGraph::node_indices(self)
    }
}
#[cfg(feature = "stable_graph")]
impl<N, E, Ty, Ix> NodeCount for StableGraph<N, E, Ty, Ix>
where
    Ty: EdgeType,
    Ix: IndexType,
{
    fn node_count(&self) -> usize {
        self.node_count()
    }
}
IntoNeighborsDirected! {
    delegate_impl[]
}
trait_template! {
    #[doc = " Define associated data for nodes and edges"] pub trait Data : GraphBase { @
    section type type NodeWeight; type EdgeWeight; }
}
Data! {
    delegate_impl[]
}
Data! {
    delegate_impl[['a, G], G, &'a mut G, deref]
}
/// An edge reference.
///
/// Edge references are used by traits `IntoEdges` and `IntoEdgeReferences`.
pub trait EdgeRef: Copy {
    type NodeId;
    type EdgeId;
    type Weight;
    /// The source node of the edge.
    fn source(&self) -> Self::NodeId;
    /// The target node of the edge.
    fn target(&self) -> Self::NodeId;
    /// A reference to the weight of the edge.
    fn weight(&self) -> &Self::Weight;
    /// The edge’s identifier.
    fn id(&self) -> Self::EdgeId;
}
impl<'a, N, E> EdgeRef for (N, N, &'a E)
where
    N: Copy,
{
    type NodeId = N;
    type EdgeId = (N, N);
    type Weight = E;
    fn source(&self) -> N {
        self.0
    }
    fn target(&self) -> N {
        self.1
    }
    fn weight(&self) -> &E {
        self.2
    }
    fn id(&self) -> (N, N) {
        (self.0, self.1)
    }
}
/// A node reference.
pub trait NodeRef: Copy {
    type NodeId;
    type Weight;
    fn id(&self) -> Self::NodeId;
    fn weight(&self) -> &Self::Weight;
}
trait_template! {
    #[doc = " Access to the sequence of the graph’s nodes"] pub trait
    IntoNodeReferences : Data + IntoNodeIdentifiers { @ section type type NodeRef :
    NodeRef < NodeId = Self::NodeId, Weight = Self::NodeWeight >; type NodeReferences :
    Iterator < Item = Self::NodeRef >; @ section self fn node_references(self) ->
    Self::NodeReferences; }
}
IntoNodeReferences! {
    delegate_impl[]
}
impl<Id> NodeRef for (Id, ())
where
    Id: Copy,
{
    type NodeId = Id;
    type Weight = ();
    fn id(&self) -> Self::NodeId {
        self.0
    }
    fn weight(&self) -> &Self::Weight {
        static DUMMY: () = ();
        &DUMMY
    }
}
impl<'a, Id, W> NodeRef for (Id, &'a W)
where
    Id: Copy,
{
    type NodeId = Id;
    type Weight = W;
    fn id(&self) -> Self::NodeId {
        self.0
    }
    fn weight(&self) -> &Self::Weight {
        self.1
    }
}
trait_template! {
    #[doc = " Access to the sequence of the graph’s edges"] pub trait
    IntoEdgeReferences : Data + GraphRef { @ section type type EdgeRef : EdgeRef < NodeId
    = Self::NodeId, EdgeId = Self::EdgeId, Weight = Self::EdgeWeight >; type
    EdgeReferences : Iterator < Item = Self::EdgeRef >; @ section self fn
    edge_references(self) -> Self::EdgeReferences; }
}
IntoEdgeReferences! {
    delegate_impl[]
}
#[cfg(feature = "graphmap")]
impl<N, E, Ty> Data for GraphMap<N, E, Ty>
where
    N: Copy + PartialEq,
    Ty: EdgeType,
{
    type NodeWeight = N;
    type EdgeWeight = E;
}
trait_template! {
    #[doc = " Edge kind property (directed or undirected edges)"] pub trait GraphProp :
    GraphBase { @ section type #[doc = " The kind edges in the graph."] type EdgeType :
    EdgeType; @ section nodelegate fn is_directed(& self) -> bool { < Self::EdgeType
    >::is_directed() } }
}
GraphProp! {
    delegate_impl[]
}
impl<N, E, Ty, Ix> GraphProp for Graph<N, E, Ty, Ix>
where
    Ty: EdgeType,
    Ix: IndexType,
{
    type EdgeType = Ty;
}
#[cfg(feature = "stable_graph")]
impl<N, E, Ty, Ix> GraphProp for StableGraph<N, E, Ty, Ix>
where
    Ty: EdgeType,
    Ix: IndexType,
{
    type EdgeType = Ty;
}
#[cfg(feature = "graphmap")]
impl<N, E, Ty> GraphProp for GraphMap<N, E, Ty>
where
    N: NodeTrait,
    Ty: EdgeType,
{
    type EdgeType = Ty;
}
impl<'a, N: 'a, E: 'a, Ty, Ix> IntoEdgeReferences for &'a Graph<N, E, Ty, Ix>
where
    Ty: EdgeType,
    Ix: IndexType,
{
    type EdgeRef = graph::EdgeReference<'a, E, Ix>;
    type EdgeReferences = graph::EdgeReferences<'a, E, Ix>;
    fn edge_references(self) -> Self::EdgeReferences {
        (*self).edge_references()
    }
}
trait_template! {
    #[doc = " The graph’s `NodeId`s map to indices"] pub trait NodeIndexable :
    GraphBase { @ section self #[doc =
    " Return an upper bound of the node indices in the graph"] #[doc =
    " (suitable for the size of a bitmap)."] fn node_bound(self : & Self) -> usize; #[doc
    = " Convert `a` to an integer index."] fn to_index(self : & Self, a : Self::NodeId)
    -> usize; #[doc = " Convert `i` to a node index"] fn from_index(self : & Self, i :
    usize) -> Self::NodeId; }
}
NodeIndexable! {
    delegate_impl[]
}
trait_template! {
    #[doc = " A graph with a known node count."] pub trait NodeCount : GraphBase { @
    section self fn node_count(self : & Self) -> usize; }
}
NodeCount! {
    delegate_impl[]
}
trait_template! {
    #[doc = " The graph’s `NodeId`s map to indices, in a range without holes."] #[doc =
    ""] #[doc = " The graph's node identifiers correspond to exactly the indices"] #[doc
    = " `0..self.node_bound()`."] pub trait NodeCompactIndexable : NodeIndexable +
    NodeCount {}
}
NodeCompactIndexable! {
    delegate_impl[]
}
impl<N, E, Ty, Ix> NodeIndexable for Graph<N, E, Ty, Ix>
where
    Ty: EdgeType,
    Ix: IndexType,
{
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
impl<N, E, Ty, Ix> NodeCompactIndexable for Graph<N, E, Ty, Ix>
where
    Ty: EdgeType,
    Ix: IndexType,
{}
/// A mapping for storing the visited status for NodeId `N`.
pub trait VisitMap<N> {
    /// Mark `a` as visited.
    ///
    /// Return **true** if this is the first visit, false otherwise.
    fn visit(&mut self, a: N) -> bool;
    /// Return whether `a` has been visited before.
    fn is_visited(&self, a: &N) -> bool;
}
impl<Ix> VisitMap<graph::NodeIndex<Ix>> for FixedBitSet
where
    Ix: IndexType,
{
    fn visit(&mut self, x: graph::NodeIndex<Ix>) -> bool {
        !self.put(x.index())
    }
    fn is_visited(&self, x: &graph::NodeIndex<Ix>) -> bool {
        self.contains(x.index())
    }
}
impl<Ix> VisitMap<graph::EdgeIndex<Ix>> for FixedBitSet
where
    Ix: IndexType,
{
    fn visit(&mut self, x: graph::EdgeIndex<Ix>) -> bool {
        !self.put(x.index())
    }
    fn is_visited(&self, x: &graph::EdgeIndex<Ix>) -> bool {
        self.contains(x.index())
    }
}
impl<Ix> VisitMap<Ix> for FixedBitSet
where
    Ix: IndexType,
{
    fn visit(&mut self, x: Ix) -> bool {
        !self.put(x.index())
    }
    fn is_visited(&self, x: &Ix) -> bool {
        self.contains(x.index())
    }
}
impl<N, S> VisitMap<N> for HashSet<N, S>
where
    N: Hash + Eq,
    S: BuildHasher,
{
    fn visit(&mut self, x: N) -> bool {
        self.insert(x)
    }
    fn is_visited(&self, x: &N) -> bool {
        self.contains(x)
    }
}
trait_template! {
    #[doc =
    " A graph that can create a map that tracks the visited status of its nodes."] pub
    trait Visitable : GraphBase { @ section type #[doc = " The associated map type"] type
    Map : VisitMap < Self::NodeId >; @ section self #[doc = " Create a new visitor map"]
    fn visit_map(self : & Self) -> Self::Map; #[doc =
    " Reset the visitor map (and resize to new size of graph if needed)"] fn
    reset_map(self : & Self, map : & mut Self::Map); }
}
Visitable! {
    delegate_impl[]
}
impl<N, E, Ty, Ix> GraphBase for Graph<N, E, Ty, Ix>
where
    Ix: IndexType,
{
    type NodeId = graph::NodeIndex<Ix>;
    type EdgeId = graph::EdgeIndex<Ix>;
}
impl<N, E, Ty, Ix> Visitable for Graph<N, E, Ty, Ix>
where
    Ty: EdgeType,
    Ix: IndexType,
{
    type Map = FixedBitSet;
    fn visit_map(&self) -> FixedBitSet {
        FixedBitSet::with_capacity(self.node_count())
    }
    fn reset_map(&self, map: &mut Self::Map) {
        map.clear();
        map.grow(self.node_count());
    }
}
#[cfg(feature = "stable_graph")]
impl<N, E, Ty, Ix> GraphBase for StableGraph<N, E, Ty, Ix>
where
    Ix: IndexType,
{
    type NodeId = graph::NodeIndex<Ix>;
    type EdgeId = graph::EdgeIndex<Ix>;
}
#[cfg(feature = "stable_graph")]
impl<N, E, Ty, Ix> Visitable for StableGraph<N, E, Ty, Ix>
where
    Ty: EdgeType,
    Ix: IndexType,
{
    type Map = FixedBitSet;
    fn visit_map(&self) -> FixedBitSet {
        FixedBitSet::with_capacity(self.node_bound())
    }
    fn reset_map(&self, map: &mut Self::Map) {
        map.clear();
        map.grow(self.node_bound());
    }
}
#[cfg(feature = "stable_graph")]
impl<N, E, Ty, Ix> Data for StableGraph<N, E, Ty, Ix>
where
    Ty: EdgeType,
    Ix: IndexType,
{
    type NodeWeight = N;
    type EdgeWeight = E;
}
#[cfg(feature = "graphmap")]
impl<N, E, Ty> GraphBase for GraphMap<N, E, Ty>
where
    N: Copy + PartialEq,
{
    type NodeId = N;
    type EdgeId = (N, N);
}
#[cfg(feature = "graphmap")]
impl<N, E, Ty> Visitable for GraphMap<N, E, Ty>
where
    N: Copy + Ord + Hash,
    Ty: EdgeType,
{
    type Map = HashSet<N>;
    fn visit_map(&self) -> HashSet<N> {
        HashSet::with_capacity(self.node_count())
    }
    fn reset_map(&self, map: &mut Self::Map) {
        map.clear();
    }
}
trait_template! {
    #[doc = " Create or access the adjacency matrix of a graph."] #[doc = ""] #[doc =
    " The implementor can either create an adjacency matrix, or it can return"] #[doc =
    " a placeholder if it has the needed representation internally."] pub trait
    GetAdjacencyMatrix : GraphBase { @ section type #[doc =
    " The associated adjacency matrix type"] type AdjMatrix; @ section self #[doc =
    " Create the adjacency matrix"] fn adjacency_matrix(self : & Self) ->
    Self::AdjMatrix; #[doc =
    " Return true if there is an edge from `a` to `b`, false otherwise."] #[doc = ""]
    #[doc = " Computes in O(1) time."] fn is_adjacent(self : & Self, matrix : &
    Self::AdjMatrix, a : Self::NodeId, b : Self::NodeId) -> bool; }
}
GetAdjacencyMatrix! {
    delegate_impl[]
}
#[cfg(feature = "graphmap")]
/// The `GraphMap` keeps an adjacency matrix internally.
impl<N, E, Ty> GetAdjacencyMatrix for GraphMap<N, E, Ty>
where
    N: Copy + Ord + Hash,
    Ty: EdgeType,
{
    type AdjMatrix = ();
    #[inline]
    fn adjacency_matrix(&self) {}
    #[inline]
    fn is_adjacent(&self, _: &(), a: N, b: N) -> bool {
        self.contains_edge(a, b)
    }
}
mod filter;
mod reversed;
#[cfg(test)]
mod tests_rug_544 {
    use super::*;
    use crate::visit::IntoNeighborsDirected;
    use crate::{Direction, Graph};
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Graph::<&str, &str>::default();
        let mut p1 = p0.add_node(rug_fuzz_0);
        let mut p2 = Direction::Outgoing;
        p0.neighbors_directed(p1, p2);
             }
});    }
}
#[cfg(test)]
mod tests_rug_545 {
    use super::*;
    use crate::{visit::IntoNeighborsDirected, prelude::StableGraph, graph::NodeIndex};
    use crate::Direction;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: StableGraph<(), ()> = StableGraph::new();
        let mut p1 = NodeIndex::new(rug_fuzz_0);
        let mut p2 = Direction::Outgoing;
        p0.neighbors_directed(p1, p2);
             }
});    }
}
#[cfg(test)]
mod tests_rug_548 {
    use super::*;
    use crate::visit::NodeCount;
    use crate::prelude::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_548_rrrruuuugggg_test_rug = 0;
        let mut p0: Graph<(), ()> = Graph::default();
        p0.node_count();
        let _rug_ed_tests_rug_548_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_549 {
    use super::*;
    use crate::prelude::*;
    use crate::visit::IntoNodeIdentifiers;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_549_rrrruuuugggg_test_rug = 0;
        let mut p0: StableGraph<i32, &'static str, Directed, u32> = StableGraph::new();
        p0.node_identifiers();
        let _rug_ed_tests_rug_549_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_550 {
    use super::*;
    use crate::visit::NodeCount;
    use crate::prelude::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_550_rrrruuuugggg_test_rug = 0;
        let mut g: StableGraph<i32, i32, Directed> = StableGraph::new();
        g.node_count();
        let _rug_ed_tests_rug_550_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_552 {
    use super::*;
    use crate::prelude::EdgeRef;
    #[test]
    fn test_target() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(i32, i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = (rug_fuzz_0, rug_fuzz_1, &rug_fuzz_2);
        <(i32, i32, &i32) as EdgeRef>::target(&p0);
             }
});    }
}
#[cfg(test)]
mod tests_rug_553 {
    use super::*;
    use crate::prelude::EdgeRef;
    use crate::visit::EdgeRef as VisitEdgeRef;
    #[test]
    fn test_rug() {
        let mut p0: (i32, i32, &'static &'static i32) = (1, 2, &&1);
        p0.weight();
    }
}
#[cfg(test)]
mod tests_rug_554 {
    use super::*;
    use crate::prelude::EdgeRef;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(i32, i32, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = (rug_fuzz_0, rug_fuzz_1, &rug_fuzz_2);
        p0.id();
             }
});    }
}
#[cfg(test)]
mod tests_rug_555 {
    use super::*;
    use crate::visit::NodeRef;
    use crate::visit::NodeRef as NodeRefTrait;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = (rug_fuzz_0, ());
        p0.id();
             }
});    }
}
#[cfg(test)]
mod tests_rug_558 {
    use super::*;
    use crate::visit::NodeRef;
    use crate::visit::NodeRef as NodeRefTrait;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let id = rug_fuzz_0;
        let w: i32 = rug_fuzz_1;
        let node = (id, &w);
        let p0 = &node;
        p0.weight();
             }
});    }
}
#[cfg(test)]
mod tests_rug_559 {
    use super::*;
    use crate::visit::IntoEdgeReferences;
    use crate::graph::{DiGraph, NodeIndex};
    #[test]
    fn test_edge_references() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(&str, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut graph: DiGraph<&str, &str> = DiGraph::new();
        let a = graph.add_node(rug_fuzz_0);
        let b = graph.add_node(rug_fuzz_1);
        let ab_edge = graph.add_edge(a, b, rug_fuzz_2);
        let edge_refs = <&DiGraph<
            &str,
            &str,
        > as IntoEdgeReferences>::edge_references(&graph);
        for edge_ref in edge_refs {
            debug_assert_eq!(edge_ref.source(), a);
            debug_assert_eq!(edge_ref.target(), b);
            debug_assert_eq!(edge_ref.weight(), & "Edge from A to B");
            debug_assert_eq!(edge_ref.id(), ab_edge);
        }
             }
});    }
}
#[cfg(test)]
mod tests_rug_560 {
    use super::*;
    use crate::{visit, Graph, data::DataMap};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_560_rrrruuuugggg_test_rug = 0;
        let mut p0: Graph<(), ()> = Graph::new();
        <Graph<(), (), _, _> as visit::NodeIndexable>::node_bound(&p0);
        let _rug_ed_tests_rug_560_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_562 {
    use super::*;
    use crate::visit::NodeIndexable;
    use crate::Graph;
    #[test]
    fn test_from_index() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut graph: Graph<(), ()> = Graph::new();
        let node_index: usize = rug_fuzz_0;
        graph.from_index(node_index);
             }
});    }
}
#[cfg(test)]
mod tests_rug_564 {
    use super::*;
    use crate::visit::VisitMap;
    use fixedbitset::FixedBitSet;
    use crate::graph::NodeIndex;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = FixedBitSet::with_capacity(rug_fuzz_0);
        let p1 = NodeIndex::<u32>::new(rug_fuzz_1);
        p0.is_visited(&p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_565 {
    use super::*;
    use crate::visit::VisitMap;
    use fixedbitset::FixedBitSet;
    use graph::{EdgeIndex, NodeIndex};
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = FixedBitSet::with_capacity(rug_fuzz_0);
        let p1 = EdgeIndex::<usize>::new(rug_fuzz_1);
        p0.visit(p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_568 {
    use super::*;
    use fixedbitset::FixedBitSet;
    use crate::graph::IndexType;
    use crate::visit::VisitMap;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = FixedBitSet::with_capacity(rug_fuzz_0);
        let mut p1 = rug_fuzz_1;
        <FixedBitSet as VisitMap<usize>>::is_visited(&p0, &p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_570 {
    use super::*;
    use crate::visit::VisitMap;
    use std::collections::HashSet;
    use crate::visit;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_570_rrrruuuugggg_test_rug = 0;
        let mut p0: HashSet<_, _> = HashSet::new();
        let p1 = visit::Time::default();
        p0.is_visited(&p1);
        let _rug_ed_tests_rug_570_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_573 {
    use super::*;
    use crate::visit::Visitable;
    use crate::prelude::*;
    #[test]
    fn test_visit_map() {
        let _rug_st_tests_rug_573_rrrruuuugggg_test_visit_map = 0;
        let mut graph: StableGraph<(), ()> = StableGraph::new();
        let p0 = &graph;
        p0.visit_map();
        let _rug_ed_tests_rug_573_rrrruuuugggg_test_visit_map = 0;
    }
}
#[cfg(test)]
mod tests_rug_575 {
    use super::*;
    use crate::visit::Visitable;
    use crate::prelude::*;
    #[test]
    fn test_visit_map() {
        let _rug_st_tests_rug_575_rrrruuuugggg_test_visit_map = 0;
        let mut p0 = GraphMap::<u32, u32, Directed>::new();
        <graphmap::GraphMap<u32, u32, Directed>>::visit_map(&p0);
        let _rug_ed_tests_rug_575_rrrruuuugggg_test_visit_map = 0;
    }
}
#[cfg(test)]
mod tests_rug_577 {
    use super::*;
    use crate::visit::GetAdjacencyMatrix;
    use crate::prelude::*;
    #[test]
    fn test_adjacency_matrix() {
        let _rug_st_tests_rug_577_rrrruuuugggg_test_adjacency_matrix = 0;
        let mut g = GraphMap::<i32, i32, Directed>::new();
        <graphmap::GraphMap<i32, i32, Directed>>::adjacency_matrix(&g);
        let _rug_ed_tests_rug_577_rrrruuuugggg_test_adjacency_matrix = 0;
    }
}
