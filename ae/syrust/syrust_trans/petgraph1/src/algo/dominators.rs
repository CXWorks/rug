//! Compute dominators of a control-flow graph.
//!
//! # The Dominance Relation
//!
//! In a directed graph with a root node **R**, a node **A** is said to *dominate* a
//! node **B** iff every path from **R** to **B** contains **A**.
//!
//! The node **A** is said to *strictly dominate* the node **B** iff **A** dominates
//! **B** and **A ≠ B**.
//!
//! The node **A** is said to be the *immediate dominator* of a node **B** iff it
//! strictly dominates **B** and there does not exist any node **C** where **A**
//! dominates **C** and **C** dominates **B**.
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use crate::visit::{DfsPostOrder, GraphBase, IntoNeighbors, Visitable, Walker};
/// The dominance relation for some graph and root.
#[derive(Debug, Clone)]
pub struct Dominators<N>
where
    N: Copy + Eq + Hash,
{
    root: N,
    dominators: HashMap<N, N>,
}
impl<N> Dominators<N>
where
    N: Copy + Eq + Hash,
{
    /// Get the root node used to construct these dominance relations.
    pub fn root(&self) -> N {
        self.root
    }
    /// Get the immediate dominator of the given node.
    ///
    /// Returns `None` for any node that is not reachable from the root, and for
    /// the root itself.
    pub fn immediate_dominator(&self, node: N) -> Option<N> {
        if node == self.root { None } else { self.dominators.get(&node).cloned() }
    }
    /// Iterate over the given node's strict dominators.
    ///
    /// If the given node is not reachable from the root, then `None` is
    /// returned.
    pub fn strict_dominators(&self, node: N) -> Option<DominatorsIter<N>> {
        if self.dominators.contains_key(&node) {
            Some(DominatorsIter {
                dominators: self,
                node: self.immediate_dominator(node),
            })
        } else {
            None
        }
    }
    /// Iterate over all of the given node's dominators (including the given
    /// node itself).
    ///
    /// If the given node is not reachable from the root, then `None` is
    /// returned.
    pub fn dominators(&self, node: N) -> Option<DominatorsIter<N>> {
        if self.dominators.contains_key(&node) {
            Some(DominatorsIter {
                dominators: self,
                node: Some(node),
            })
        } else {
            None
        }
    }
}
/// Iterator for a node's dominators.
pub struct DominatorsIter<'a, N>
where
    N: 'a + Copy + Eq + Hash,
{
    dominators: &'a Dominators<N>,
    node: Option<N>,
}
impl<'a, N> Iterator for DominatorsIter<'a, N>
where
    N: 'a + Copy + Eq + Hash,
{
    type Item = N;
    fn next(&mut self) -> Option<Self::Item> {
        let next = self.node.take();
        if let Some(next) = next {
            self.node = self.dominators.immediate_dominator(next);
        }
        next
    }
}
/// The undefined dominator sentinel, for when we have not yet discovered a
/// node's dominator.
const UNDEFINED: usize = ::std::usize::MAX;
/// This is an implementation of the engineered ["Simple, Fast Dominance
/// Algorithm"][0] discovered by Cooper et al.
///
/// This algorithm is **O(|V|²)**, and therefore has slower theoretical running time
/// than the Lengauer-Tarjan algorithm (which is **O(|E| log |V|)**. However,
/// Cooper et al found it to be faster in practice on control flow graphs of up
/// to ~30,000 vertices.
///
/// [0]: http://www.cs.rice.edu/~keith/EMBED/dom.pdf
pub fn simple_fast<G>(graph: G, root: G::NodeId) -> Dominators<G::NodeId>
where
    G: IntoNeighbors + Visitable,
    <G as GraphBase>::NodeId: Eq + Hash,
{
    let (post_order, predecessor_sets) = simple_fast_post_order(graph, root);
    let length = post_order.len();
    debug_assert!(length > 0);
    debug_assert!(post_order.last() == Some(& root));
    let node_to_post_order_idx: HashMap<_, _> = post_order
        .iter()
        .enumerate()
        .map(|(idx, &node)| (node, idx))
        .collect();
    let idx_to_predecessor_vec = predecessor_sets_to_idx_vecs(
        &post_order,
        &node_to_post_order_idx,
        predecessor_sets,
    );
    let mut dominators = vec![UNDEFINED; length];
    dominators[length - 1] = length - 1;
    let mut changed = true;
    while changed {
        changed = false;
        for idx in (0..length - 1).rev() {
            debug_assert!(post_order[idx] != root);
            let new_idom_idx = {
                let mut predecessors = idx_to_predecessor_vec[idx]
                    .iter()
                    .filter(|&&p| dominators[p] != UNDEFINED);
                let new_idom_idx = predecessors
                    .next()
                    .expect(
                        "Because the root is initialized to dominate itself, and is the \
                     first node in every path, there must exist a predecessor to this \
                     node that also has a dominator",
                    );
                predecessors
                    .fold(
                        *new_idom_idx,
                        |new_idom_idx, &predecessor_idx| {
                            intersect(&dominators, new_idom_idx, predecessor_idx)
                        },
                    )
            };
            debug_assert!(new_idom_idx < length);
            if new_idom_idx != dominators[idx] {
                dominators[idx] = new_idom_idx;
                changed = true;
            }
        }
    }
    debug_assert!(! dominators.iter().any(|& dom | dom == UNDEFINED));
    Dominators {
        root,
        dominators: dominators
            .into_iter()
            .enumerate()
            .map(|(idx, dom_idx)| (post_order[idx], post_order[dom_idx]))
            .collect(),
    }
}
fn intersect(dominators: &[usize], mut finger1: usize, mut finger2: usize) -> usize {
    loop {
        match finger1.cmp(&finger2) {
            Ordering::Less => finger1 = dominators[finger1],
            Ordering::Greater => finger2 = dominators[finger2],
            Ordering::Equal => return finger1,
        }
    }
}
fn predecessor_sets_to_idx_vecs<N>(
    post_order: &[N],
    node_to_post_order_idx: &HashMap<N, usize>,
    mut predecessor_sets: HashMap<N, HashSet<N>>,
) -> Vec<Vec<usize>>
where
    N: Copy + Eq + Hash,
{
    post_order
        .iter()
        .map(|node| {
            predecessor_sets
                .remove(node)
                .map(|predecessors| {
                    predecessors
                        .into_iter()
                        .map(|p| *node_to_post_order_idx.get(&p).unwrap())
                        .collect()
                })
                .unwrap_or_else(Vec::new)
        })
        .collect()
}
fn simple_fast_post_order<G>(
    graph: G,
    root: G::NodeId,
) -> (Vec<G::NodeId>, HashMap<G::NodeId, HashSet<G::NodeId>>)
where
    G: IntoNeighbors + Visitable,
    <G as GraphBase>::NodeId: Eq + Hash,
{
    let mut post_order = vec![];
    let mut predecessor_sets = HashMap::new();
    for node in DfsPostOrder::new(graph, root).iter(graph) {
        post_order.push(node);
        for successor in graph.neighbors(node) {
            predecessor_sets.entry(successor).or_insert_with(HashSet::new).insert(node);
        }
    }
    (post_order, predecessor_sets)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_iter_dominators() {
        let doms: Dominators<u32> = Dominators {
            root: 0,
            dominators: [(2, 1), (1, 0), (0, 0)].iter().cloned().collect(),
        };
        let all_doms: Vec<_> = doms.dominators(2).unwrap().collect();
        assert_eq!(vec![2, 1, 0], all_doms);
        assert_eq!(None::< () >, doms.dominators(99).map(| _ | unreachable!()));
        let strict_doms: Vec<_> = doms.strict_dominators(2).unwrap().collect();
        assert_eq!(vec![1, 0], strict_doms);
        assert_eq!(None::< () >, doms.strict_dominators(99).map(| _ | unreachable!()));
    }
}
#[cfg(test)]
mod tests_rug_45 {
    use super::*;
    use crate::algo::dominators::intersect;
    use std::cmp::Ordering;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_45_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 3;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 2;
        let rug_fuzz_6 = 4;
        let mut p0 = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3, rug_fuzz_4];
        let mut p1 = rug_fuzz_5;
        let mut p2 = rug_fuzz_6;
        intersect(&p0, p1, p2);
        let _rug_ed_tests_rug_45_rrrruuuugggg_test_rug = 0;
    }
}
