use std::collections::BTreeMap;

use super::Edges;
use crate::disjoint::DisjointSet;
use crate::{FaceId, VertexId};

/// One-face edges chained into loops in their face's direction, split wherever a
/// vertex repeats, so a boundary pinched at a vertex gives one loop per side whatever
/// the vertex labels. Each loop starts at its smallest vertex; loops are sorted.
pub(super) fn boundary_loops(edges: &Edges) -> Vec<Vec<VertexId>> {
    let mut next: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for (&[a, b], uses) in edges {
        if let [only] = uses.as_slice() {
            let (from, to) = if only.forward { (a, b) } else { (b, a) };
            next.entry(from).or_default().push(to);
        }
    }
    let mut loops = Vec::new();
    while let Some((&start, _)) = next.iter().find(|(_, t)| !t.is_empty()) {
        let mut walk = vec![start];
        let mut at = start;
        while let Some(to) = next
            .get_mut(&at)
            .filter(|t| !t.is_empty())
            .map(|t| t.remove(0))
        {
            walk.push(to);
            if to == start {
                break;
            }
            at = to;
        }
        split_at_repeats(&walk, &mut loops);
    }
    loops.sort();
    loops
}

/// Cuts a walk into simple loops at each repeated vertex, each rotated to start at its
/// smallest vertex. A walk that never returns to its start is kept as one open chain.
fn split_at_repeats(walk: &[u32], loops: &mut Vec<Vec<VertexId>>) {
    let mut stack: Vec<u32> = Vec::new();
    for &v in walk {
        if let Some(at) = stack.iter().position(|&s| s == v) {
            let mut cycle: Vec<u32> = stack.split_off(at);
            let low = cycle
                .iter()
                .enumerate()
                .min_by_key(|&(_, x)| x)
                .map_or(0, |(i, _)| i);
            cycle.rotate_left(low);
            loops.push(cycle.into_iter().map(VertexId::new).collect());
        }
        stack.push(v);
    }
    // What remains is the start alone after a closed walk, or an open chain.
    if stack.len() > 1 {
        loops.push(stack.into_iter().map(VertexId::new).collect());
    }
}

/// Kept faces grouped by shared vertices, components in order of their first face.
pub(crate) fn components(vertex_count: usize, tris: &[[u32; 3]], kept: &[u32]) -> Vec<Vec<FaceId>> {
    let mut set = DisjointSet::new(vertex_count);
    for &face in kept {
        // INVARIANT: kept faces index `tris`, and their vertices are below `vertex_count`.
        let [a, b, c] = tris[face as usize];
        set.union(a, b);
        set.union(b, c);
    }
    let mut index: BTreeMap<u32, usize> = BTreeMap::new();
    let mut out: Vec<Vec<FaceId>> = Vec::new();
    for &face in kept {
        // INVARIANT: as above.
        let root = set.find(tris[face as usize][0]);
        let slot = *index.entry(root).or_insert_with(|| {
            out.push(Vec::new());
            out.len() - 1
        });
        out[slot].push(FaceId::new(face));
    }
    out
}
