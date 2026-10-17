//! Vertices whose faces form more than one fan, split into one vertex per fan.

use std::collections::{BTreeMap, BTreeSet};

use crate::disjoint::DisjointSet;
use crate::{Counted, MeshError, TriangleSoup, VertexId};

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
pub(crate) struct VertexCopy {
    pub(crate) of: VertexId,
    pub(crate) copy: VertexId,
}

/// Gives every fan of faces at a vertex after the first (the one holding the lowest
/// corner) a copy of the vertex. Faces are fans when they share an edge at the
/// vertex, however many faces the edge has. Faces, their order and their corners
/// are unchanged and no vertex moves. Requires repeated-index faces gone
/// (`degenerate::remove`).
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "called by into_manifold, not yet written")
)]
pub(crate) fn split(soup: &TriangleSoup) -> Result<(TriangleSoup, Split), MeshError> {
    let tris = soup.triangle_indices();
    // Lossless: the face count fits in u32 (`TriangleSoup::new`).
    let faces: Vec<u32> = (0..tris.len() as u32).collect();
    let mut fans = DisjointSet::new(tris.len() * 3);
    for (&ends, uses) in &crate::validate::edges(tris, &faces) {
        for v in ends {
            let mut corners = uses.iter().filter_map(|u| corner(tris, u.face, v));
            if let Some(first) = corners.next() {
                corners.for_each(|c| fans.union(first, c));
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
