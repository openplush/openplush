//! Vertices whose faces form more than one fan, split into one vertex per fan.

use std::collections::{BTreeMap, BTreeSet};

use crate::disjoint::DisjointSet;
use crate::{Counted, FaceId, MeshError, PairedEdge, TriangleSoup, VertexId};

/// The vertices a split added, by the IDs of the soup it was given.
#[derive(Debug, Default)]
pub(crate) struct Split {
    /// Each added vertex, sorted by the copy.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "read by into_manifold, not yet written")
    )]
    pub(crate) copies: Vec<VertexCopy>,
}

/// A vertex added so one fan of faces no longer shares `of` with the others.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VertexCopy {
    /// The vertex whose fans were separated.
    pub of: VertexId,
    /// The vertex added for one fan.
    pub copy: VertexId,
}

/// Gives every fan of faces at a vertex after the first (the one holding the lowest
/// corner) a copy of the vertex. Faces are fans when they share an edge at the
/// vertex; on an edge `pairings` resolve, only the faces of one pair do, and an
/// unpaired face is a fan of its own there. Faces `pairings` do not name share the
/// edge as one group. The pairings name the soup given, so after a split that copied a
/// vertex they are stale: pair the returned soup again before building from it. Faces, their order and their corners
/// are unchanged and no vertex moves. A face that repeats a vertex is refused
/// (`degenerate::remove` drops them first).
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "called by into_manifold, not yet written")
)]
pub(crate) fn split(
    soup: &TriangleSoup,
    pairings: &[PairedEdge],
) -> Result<(TriangleSoup, Split), MeshError> {
    let tris = soup.triangle_indices();
    if let Some(f) = tris
        .iter()
        .position(|&[a, b, c]| a == b || b == c || c == a)
    {
        return Err(MeshError::DegenerateTriangle {
            // Lossless: the face count fits in u32 (`TriangleSoup::new`).
            triangle: FaceId::new(f as u32),
        });
    }
    // Lossless: the face count fits in u32 (`TriangleSoup::new`).
    let faces: Vec<u32> = (0..tris.len() as u32).collect();
    let mut fans = DisjointSet::new(tris.len() * 3);
    let copies = copies_by_edge(pairings);
    for (&ends, uses) in &crate::validate::edges(tris, &faces) {
        let on_edge: Vec<u32> = uses.iter().map(|u| u.face).collect();
        let groups = match copies.get(&ends) {
            Some(groups) => with_the_rest(groups, &on_edge),
            None => vec![on_edge],
        };
        for v in ends {
            for group in &groups {
                let mut corners = group.iter().filter_map(|&f| corner(tris, f, v));
                if let Some(first) = corners.next() {
                    corners.for_each(|c| fans.union(first, c));
                }
            }
        }
    }
    let classes: BTreeSet<(u32, u32)> = tris
        .iter()
        .flatten()
        .enumerate()
        // Lossless: the corner count is three times a u32 face count.
        .map(|(i, &v)| (v, fans.find(i as u32)))
        .collect();
    let (id_of, added) = number_fans(&classes, soup.vertex_count())?;
    let rewritten = tris
        .iter()
        .enumerate()
        .map(|(f, &t)| {
            let mut out = t;
            for (slot, v) in out.iter_mut().enumerate() {
                // Lossless: the corner count is three times a u32 face count.
                // INVARIANT: `classes` holds every corner's (vertex, fan), so it is numbered.
                *v = id_of[&(*v, fans.find((f * 3 + slot) as u32))];
            }
            out
        })
        .collect();
    let copies = added
        .iter()
        .enumerate()
        .map(|(k, &of)| VertexCopy {
            of: VertexId::new(of),
            // INVARIANT: `number_fans` checked every new ID fits in u32.
            copy: VertexId::new((soup.vertex_count() + k) as u32),
        })
        .collect();
    let rebuilt = soup.with_vertex_copies(&added).with_faces(
        rewritten,
        soup.corner_sources().to_vec(),
        soup.corner_uvs().map(<[_]>::to_vec),
    );
    Ok((rebuilt, Split { copies }))
}

/// The groups of faces that share a copy of each paired edge, by its ends.
fn copies_by_edge(pairings: &[PairedEdge]) -> BTreeMap<[u32; 2], Vec<Vec<u32>>> {
    pairings
        .iter()
        .map(|p| {
            let groups = p
                .pairs
                .iter()
                .map(|pair| pair.iter().map(|f| f.index()).collect())
                .chain(p.unpaired.map(|f| vec![f.index()]))
                .collect();
            (p.vertices.map(VertexId::index), groups)
        })
        .collect()
}

/// `groups`, and the faces of `on_edge` that no group names as one more group.
fn with_the_rest(groups: &[Vec<u32>], on_edge: &[u32]) -> Vec<Vec<u32>> {
    let rest: Vec<u32> = on_edge
        .iter()
        .copied()
        .filter(|f| !groups.iter().flatten().any(|g| g == f))
        .collect();
    let mut all = groups.to_vec();
    if !rest.is_empty() {
        all.push(rest);
    }
    all
}

/// The vertex ID of each (vertex, fan root) pair.
type Numbering = BTreeMap<(u32, u32), u32>;

/// The vertex each (vertex, fan) is renumbered to, and the vertex each added ID
/// copies, in ID order.
fn number_fans(
    classes: &BTreeSet<(u32, u32)>,
    vertex_count: usize,
) -> Result<(Numbering, Vec<u32>), MeshError> {
    let mut id_of = BTreeMap::new();
    let mut added = Vec::new();
    let mut previous = None;
    for &(v, fan) in classes {
        if previous == Some(v) {
            let count = vertex_count + added.len();
            let id = u32::try_from(count).map_err(|_| MeshError::TooLarge {
                what: Counted::Vertices,
                count,
            })?;
            id_of.insert((v, fan), id);
            added.push(v);
        } else {
            id_of.insert((v, fan), v);
        }
        previous = Some(v);
    }
    Ok((id_of, added))
}

/// The corner of face `f` at vertex `v`, as an index into the per-corner arrays.
fn corner(tris: &[[u32; 3]], f: u32, v: u32) -> Option<u32> {
    let slot = tris[f as usize].iter().position(|&x| x == v)?;
    // Lossless: the slot is below 3.
    Some(f * 3 + slot as u32)
}

#[cfg(test)]
mod tests;
