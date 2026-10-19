//! Dijkstra over an adjacency list, shared by every shortest-edge-path search in the crate.

use std::cmp::Ordering;
use std::collections::{BTreeSet, BinaryHeap};

use openplush_mesh::EdgeId;

/// Shortest-path distances and the edge each vertex was reached by.
pub(crate) struct Tree {
    pub(crate) dist: Vec<f64>,
    pub(crate) parent: Vec<Option<(usize, EdgeId)>>,
}

/// Dijkstra over `adjacent` (each vertex's neighbours with the edge to each) from `sources`, each
/// at distance 0, with `length` the cost of an edge, until `stop` holds for the vertex taken
/// next, which is returned. Ties go to the lower vertex, then the first edge listed.
pub(crate) fn search(
    adjacent: &[Vec<(usize, EdgeId)>],
    length: impl Fn(EdgeId) -> f64,
    sources: &[usize],
    stop: impl Fn(usize) -> bool,
) -> (Tree, Option<usize>) {
    let n = adjacent.len();
    let mut tree = Tree {
        dist: vec![f64::INFINITY; n],
        parent: vec![None; n],
    };
    let mut heap = BinaryHeap::new();
    for &s in sources {
        tree.dist[s] = 0.0;
        heap.push(Entry {
            dist: 0.0,
            vertex: s,
        });
    }
    let mut settled = vec![false; n];
    while let Some(Entry { dist, vertex }) = heap.pop() {
        if std::mem::replace(&mut settled[vertex], true) {
            continue;
        }
        if stop(vertex) {
            return (tree, Some(vertex));
        }
        for &(w, edge) in &adjacent[vertex] {
            let d = dist + length(edge);
            if d < tree.dist[w] {
                tree.dist[w] = d;
                tree.parent[w] = Some((vertex, edge));
                heap.push(Entry { dist: d, vertex: w });
            }
        }
    }
    (tree, None)
}

impl Tree {
    /// The vertices from `v` up to its root, `v` first.
    pub(crate) fn vertices_to(&self, v: usize) -> impl Iterator<Item = usize> + '_ {
        std::iter::successors(Some(v), |&u| self.parent[u].map(|(p, _)| p))
    }

    /// The edges from `v` up to its root.
    pub(crate) fn path_to(&self, v: usize) -> impl Iterator<Item = EdgeId> + '_ {
        self.vertices_to(v)
            .filter_map(|u| self.parent[u].map(|(_, e)| e))
    }

    /// The tree edges of the loop an edge between `ends` closes: each end's path up to where the
    /// two meet, or to their roots when they never do.
    pub(crate) fn loop_through(&self, ends: [usize; 2]) -> Vec<EdgeId> {
        let above: BTreeSet<usize> = self.vertices_to(ends[0]).collect();
        let meet = self.vertices_to(ends[1]).find(|v| above.contains(v));
        let up = |v: usize| {
            self.vertices_to(v)
                .take_while(move |&u| Some(u) != meet)
                .filter_map(|u| self.parent[u].map(|(_, e)| e))
        };
        up(ends[0]).chain(up(ends[1])).collect()
    }
}

/// A heap entry ordered so the nearest vertex, then the lowest, pops first.
struct Entry {
    dist: f64,
    vertex: usize,
}

impl Ord for Entry {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .dist
            .total_cmp(&self.dist)
            .then(other.vertex.cmp(&self.vertex))
    }
}

impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Entry {}
