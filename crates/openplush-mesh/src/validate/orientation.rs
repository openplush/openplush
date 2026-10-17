use std::collections::{BTreeMap, BTreeSet, VecDeque};

use openplush_geom::DVec3;

use super::Edges;
use crate::disjoint::DisjointSet;
use crate::{FaceId, VertexId};

/// Two-face edges whose faces traverse them the same way.
pub(super) fn conflicts(edges: &Edges) -> Vec<[VertexId; 2]> {
    edges
        .iter()
        .filter(|(_, uses)| matches!(uses.as_slice(), [p, q] if p.forward == q.forward))
        .map(|(&[a, b], _)| [VertexId::new(a), VertexId::new(b)])
        .collect()
}

/// Two-face edges where the flips a breadth-first walk assigns (from each
/// component's first face, across two-face edges) contradict each other.
pub(super) fn non_orientable(edges: &Edges, tris: &[[u32; 3]], kept: &[u32]) -> Vec<[VertexId; 2]> {
    // flip[face]: None until reached; Some(true) if the face must turn over to agree.
    let mut flip: Vec<Option<bool>> = vec![None; tris.len()];
    let mut bad: BTreeSet<[u32; 2]> = BTreeSet::new();
    for &seed in kept {
        // INVARIANT: kept faces index `tris` and `flip`.
        if flip[seed as usize].is_some() {
            continue;
        }
        flip[seed as usize] = Some(false);
        let mut queue = VecDeque::from([seed]);
        while let Some(face) = queue.pop_front() {
            let mine = flip[face as usize] == Some(true);
            for key in edge_keys(tris[face as usize]) {
                let Some([p, q]) = edges.get(&key).map(Vec::as_slice).and_then(pair) else {
                    continue;
                };
                let (here, there) = if p.face == face { (p, q) } else { (q, p) };
                // Faces agree when they traverse a shared edge in opposite directions.
                let wanted = mine ^ (here.forward == there.forward);
                match flip[there.face as usize] {
                    None => {
                        flip[there.face as usize] = Some(wanted);
                        queue.push_back(there.face);
                    }
                    Some(have) if have != wanted => {
                        bad.insert(key);
                    }
                    Some(_) => {}
                }
            }
        }
    }
    bad.into_iter()
        .map(|[a, b]| [VertexId::new(a), VertexId::new(b)])
        .collect()
}

fn pair<T: Copy>(uses: &[T]) -> Option<[T; 2]> {
    match uses {
        [p, q] => Some([*p, *q]),
        _ => None,
    }
}

/// Shells, faces joined across two-face edges, that are closed (each of their edges
/// used by exactly two of their own faces) and whose signed volume is not positive.
/// Judging shells rather than components catches an inside-out shell that touches the
/// rest of the mesh only at a vertex or a non-manifold edge. Positions are taken
/// relative to the shell's first vertex and divided by the diagonal, so a tiny model's
/// volume does not underflow to zero.
pub(super) fn inside_out(
    edges: &Edges,
    tris: &[[u32; 3]],
    kept: &[u32],
    positions: &[DVec3],
    diagonal: f64,
) -> Vec<Vec<FaceId>> {
    shells(edges, tris, kept)
        .into_iter()
        .filter(|shell| closed(tris, shell) && volume(tris, shell, positions, diagonal) <= 0.0)
        .map(|shell| shell.into_iter().map(FaceId::new).collect())
        .collect()
}

/// Kept faces joined across two-face edges, in order of each shell's first face.
fn shells(edges: &Edges, tris: &[[u32; 3]], kept: &[u32]) -> Vec<Vec<u32>> {
    let mut set = DisjointSet::new(tris.len());
    for uses in edges.values() {
        if let [p, q] = uses.as_slice() {
            set.union(p.face, q.face);
        }
    }
    let mut index: BTreeMap<u32, usize> = BTreeMap::new();
    let mut out: Vec<Vec<u32>> = Vec::new();
    for &face in kept {
        let slot = *index.entry(set.find(face)).or_insert_with(|| {
            out.push(Vec::new());
            out.len() - 1
        });
        out[slot].push(face);
    }
    out
}

// INVARIANT (here and in `volume`): shell faces index `tris`, and their vertices index
// `positions`.
fn closed(tris: &[[u32; 3]], shell: &[u32]) -> bool {
    let mut uses: BTreeMap<[u32; 2], u32> = BTreeMap::new();
    for &face in shell {
        for key in edge_keys(tris[face as usize]) {
            *uses.entry(key).or_default() += 1;
        }
    }
    uses.values().all(|&n| n == 2)
}

fn volume(tris: &[[u32; 3]], shell: &[u32], positions: &[DVec3], diagonal: f64) -> f64 {
    let Some(&first) = shell.first() else {
        return 0.0;
    };
    let origin = positions[tris[first as usize][0] as usize];
    let local = |v: u32| (positions[v as usize] - origin) * (1.0 / diagonal);
    shell
        .iter()
        .map(|&f| {
            let [a, b, c] = tris[f as usize].map(local);
            a.dot(b.cross(c))
        })
        .sum()
}

/// The three edge keys of a triangle, smaller vertex first.
pub(super) fn edge_keys([a, b, c]: [u32; 3]) -> [[u32; 2]; 3] {
    [[a, b], [b, c], [c, a]].map(|[x, y]| if x < y { [x, y] } else { [y, x] })
}
