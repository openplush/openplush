//! Shortest edge paths over a chart cut open along its seams: the searches that find the cuts
//! closing a chart into a disk.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

use openplush_mesh::{EdgeId, HalfEdgeId, ManifoldMesh};

use crate::chart::{cut_test, sides};
use crate::{Chart, ChartMesh, SeamSet};

/// An edge of the cut-open chart with a chart face on each side, so cutting along it changes the
/// chart.
struct Inner {
    ends: [usize; 2],
    faces: [usize; 2],
    length: f64,
}

/// A chart cut open along its seams, as a graph of its inner edges and boundary loops.
pub(crate) struct CutOpen {
    open: ChartMesh,
    inner: BTreeMap<EdgeId, Inner>,
    adjacent: Vec<Vec<(usize, EdgeId)>>,
    /// Each boundary loop's local vertices, the loops in order of their lowest half-edge.
    loops: Vec<Vec<usize>>,
}

/// Shortest-path distances and the edge each vertex was reached by.
struct Tree {
    dist: Vec<f64>,
    parent: Vec<Option<(usize, EdgeId)>>,
}

impl CutOpen {
    /// `chart` cut open along `seams`.
    pub(crate) fn new(mesh: &ManifoldMesh, seams: &SeamSet, chart: &Chart) -> Self {
        let open = ChartMesh::new(mesh, seams, chart);
        let cut = cut_test(mesh, seams, chart);
        let mut halves: BTreeMap<EdgeId, Vec<(usize, [usize; 2])>> = BTreeMap::new();
        let mut boundary = BTreeMap::new();
        for (k, &(f, corners)) in open.faces().iter().enumerate() {
            for (i, h) in sides(mesh, f).into_iter().enumerate() {
                let ends = [i, (i + 1) % 3].map(|j| corners[j].index() as usize);
                if cut(h) {
                    boundary.insert(h, ends);
                } else {
                    halves.entry(h.edge()).or_default().push((k, ends));
                }
            }
        }
        let xyz = open.xyz();
        let inner: BTreeMap<EdgeId, Inner> = halves
            .into_iter()
            .filter_map(|(edge, sides)| match sides[..] {
                [(k, ends), (other, _)] => Some((
                    edge,
                    Inner {
                        ends,
                        faces: [k, other],
                        length: (xyz[ends[1]] - xyz[ends[0]]).length(),
                    },
                )),
                _ => None,
            })
            .collect();
        let mut adjacent = vec![Vec::new(); xyz.len()];
        for (&edge, e) in &inner {
            adjacent[e.ends[0]].push((e.ends[1], edge));
            adjacent[e.ends[1]].push((e.ends[0], edge));
        }
        let loops = loops(&boundary, xyz.len());
        Self {
            open,
            inner,
            adjacent,
            loops,
        }
    }

    /// The shortest path between the two vertices of largest angle defect, for a closed chart.
    pub(crate) fn between_sharpest(&self) -> Option<Vec<EdgeId>> {
        let defect = self.angle_defects();
        let mut order: Vec<usize> = (0..defect.len()).collect();
        order.sort_by(|&a, &b| defect[b].total_cmp(&defect[a]).then(a.cmp(&b)));
        let (&from, &to) = (order.first()?, order.get(1)?);
        let (tree, hit) = self.search(&[from], |v| v == to);
        hit.map(|to| tree.path_to(to).collect())
    }

    /// Shortest paths that join every boundary loop into one: starting from the first loop, the
    /// path from the loops joined so far to the nearest other loop, until one remains.
    pub(crate) fn joining_loops(&self) -> Option<Vec<EdgeId>> {
        let mut loop_of = vec![None; self.adjacent.len()];
        for (l, members) in self.loops.iter().enumerate() {
            members.iter().for_each(|&v| loop_of[v] = Some(l));
        }
        let mut joined = vec![false; self.adjacent.len()];
        let mut done = vec![false; self.loops.len()];
        let mut cuts = Vec::new();
        let mut next = self.loops.first().map(|_| 0);
        while let Some(l) = next {
            done[l] = true;
            self.loops[l].iter().for_each(|&v| joined[v] = true);
            if done.iter().all(|&d| d) {
                break;
            }
            let sources: Vec<usize> = (0..joined.len()).filter(|&v| joined[v]).collect();
            let (tree, hit) = self.search(&sources, |v| loop_of[v].is_some_and(|l| !done[l]));
            let hit = hit?;
            tree.vertices_to(hit).for_each(|v| joined[v] = true);
            cuts.extend(tree.path_to(hit));
            next = loop_of[hit];
        }
        Some(cuts)
    }

    /// Tree-cotree handle loops (Eppstein 2003, with Erickson & Whittlesey 2005's greedy choice):
    /// a shortest-path tree from the boundary, or from the lowest mesh vertex when there is none;
    /// a maximum spanning tree of the dual over the other inner edges, each weighted by the length
    /// of the loop it closes through the tree; and the loops the edges left over close.
    pub(crate) fn handle_loops(&self) -> Option<Vec<EdgeId>> {
        let n = self.adjacent.len();
        let roots: Vec<usize> = if self.loops.is_empty() {
            let lowest = (0..n).min_by_key(|&v| self.open.vtx_to_global()[v])?;
            vec![lowest]
        } else {
            self.loops
                .iter()
                .flatten()
                .copied()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect()
        };
        let (tree, _) = self.search(&roots, |_| false);
        if tree.dist.iter().any(|d| d.is_infinite()) {
            return None;
        }
        let in_tree: BTreeSet<EdgeId> = tree.parent.iter().flatten().map(|&(_, e)| e).collect();
        let mut rest: Vec<(f64, EdgeId)> = self
            .inner
            .iter()
            .filter(|(edge, _)| !in_tree.contains(edge))
            .map(|(&edge, e)| (tree.dist[e.ends[0]] + tree.dist[e.ends[1]] + e.length, edge))
            .collect();
        rest.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        let mut dual = Faces::new(self.open.faces().len());
        let mut cuts = Vec::new();
        for (_, edge) in rest {
            let e = &self.inner[&edge];
            if !dual.union(e.faces[0], e.faces[1]) {
                cuts.push(edge);
                cuts.extend(tree.loop_through(e.ends));
            }
        }
        Some(cuts)
    }

    /// 2π minus the sum of the corner angles at each local vertex.
    fn angle_defects(&self) -> Vec<f64> {
        let xyz = self.open.xyz();
        let mut defect = vec![std::f64::consts::TAU; xyz.len()];
        for (_, corners) in self.open.faces() {
            let c = corners.map(|v| v.index() as usize);
            for i in 0..3 {
                let (a, b) = (
                    xyz[c[(i + 1) % 3]] - xyz[c[i]],
                    xyz[c[(i + 2) % 3]] - xyz[c[i]],
                );
                defect[c[i]] -= libm::atan2(a.cross(b).length(), a.dot(b));
            }
        }
        defect
    }

    /// Dijkstra over inner edges from `sources`, each at distance 0, until `stop` holds for the
    /// vertex taken next, which is returned. Ties go to the lower local vertex, then the lower
    /// edge.
    fn search(&self, sources: &[usize], stop: impl Fn(usize) -> bool) -> (Tree, Option<usize>) {
        let n = self.adjacent.len();
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
            for &(w, edge) in &self.adjacent[vertex] {
                let d = dist + self.inner[&edge].length;
                if d < tree.dist[w] {
                    tree.dist[w] = d;
                    tree.parent[w] = Some((vertex, edge));
                    heap.push(Entry { dist: d, vertex: w });
                }
            }
        }
        (tree, None)
    }
}

impl Tree {
    /// The vertices from `v` up to its root, `v` first.
    fn vertices_to(&self, v: usize) -> impl Iterator<Item = usize> + '_ {
        std::iter::successors(Some(v), |&u| self.parent[u].map(|(p, _)| p))
    }

    /// The edges from `v` up to its root.
    fn path_to(&self, v: usize) -> impl Iterator<Item = EdgeId> + '_ {
        self.vertices_to(v)
            .filter_map(|u| self.parent[u].map(|(_, e)| e))
    }

    /// The tree edges of the loop an edge between `ends` closes: each end's path up to where the
    /// two meet, or to their roots when they never do.
    fn loop_through(&self, ends: [usize; 2]) -> Vec<EdgeId> {
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

/// Union-find over a chart's faces, for the dual spanning tree.
struct Faces(Vec<usize>);

impl Faces {
    fn new(n: usize) -> Self {
        Self((0..n).collect())
    }

    fn root(&mut self, mut f: usize) -> usize {
        while self.0[f] != f {
            self.0[f] = self.0[self.0[f]];
            f = self.0[f];
        }
        f
    }

    /// Joins the sets of `a` and `b`; false when they were already one.
    fn union(&mut self, a: usize, b: usize) -> bool {
        let (a, b) = (self.root(a), self.root(b));
        self.0[a.max(b)] = a.min(b);
        a != b
    }
}

/// Each boundary loop's vertices, following each cut half-edge to the one leaving its end; the
/// loops in order of their lowest half-edge.
fn loops(boundary: &BTreeMap<HalfEdgeId, [usize; 2]>, n: usize) -> Vec<Vec<usize>> {
    let mut out = vec![None; n];
    for ends in boundary.values() {
        out[ends[0]] = Some(ends[1]);
    }
    let mut seen = vec![false; n];
    let mut loops = Vec::new();
    for ends in boundary.values() {
        let mut members = Vec::new();
        let mut v = ends[0];
        while !std::mem::replace(&mut seen[v], true) {
            members.push(v);
            match out[v] {
                Some(w) => v = w,
                None => break,
            }
        }
        if !members.is_empty() {
            loops.push(members);
        }
    }
    loops
}
