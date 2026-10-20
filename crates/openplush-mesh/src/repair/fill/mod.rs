//! Step 7's fill, as Liepa 2003: triangulate a listed loop, refine the patch to the density of
//! the edges around it, fair it, and append it to the mesh. Nothing here is silent: a loop it
//! cannot fill is a typed error naming the loop.

mod fair;
mod refine;
mod triangulate;

use std::collections::BTreeSet;

use openplush_geom::DVec3;
use openplush_geom::epsilon::ModelTolerances;

use crate::halfedge::is_sliver;
use crate::{FaceId, HalfEdgeId, LoopRef, ManifoldMesh, MeshError, VertexId};
use refine::Patch;
use triangulate::Rim;

/// The largest loop filled: the triangulation is O(n³) in time and O(n²) in memory, and a
/// scan's many-thousand-vertex hole would exhaust a wasm heap. A guess to revisit with the
/// fixture corpus.
pub(crate) const MAX_LOOP: usize = 512;

/// Fills the boundary loop `rim` (its half-edges in `next` order), named `loop_ref`, and
/// returns the faces added.
///
/// # Errors
/// `HoleTooLarge` above [`MAX_LOOP`] vertices; `HoleOnSplit` when the loop runs along a copy
/// of a split edge; `HoleNotFillable` when no triangulation avoids existing chords and
/// slivers, refinement does not settle, or fairing does not solve or leaves a sliver.
pub(crate) fn fill(
    mesh: &mut ManifoldMesh,
    loop_ref: LoopRef,
    rim: &[HalfEdgeId],
    tol: &ModelTolerances,
) -> Result<Vec<FaceId>, MeshError> {
    if rim.len() > MAX_LOOP {
        return Err(MeshError::HoleTooLarge {
            loop_ref,
            vertices: rim.len(),
        });
    }
    let copies: BTreeSet<_> = mesh.edge_copies().iter().flatten().copied().collect();
    if rim.iter().any(|h| copies.contains(&h.edge())) {
        return Err(MeshError::HoleOnSplit { loop_ref });
    }
    let unfillable = MeshError::HoleNotFillable { loop_ref };
    let vertices: Vec<VertexId> = rim.iter().map(|&h| mesh.origin(h)).collect();
    let points: Vec<DVec3> = vertices.iter().map(|&v| mesh.position(v)).collect();
    let across: Vec<DVec3> = rim.iter().map(|&h| face_normal(mesh, h.twin())).collect();
    let chords = chords(mesh, &vertices);
    let joined = |i: usize, k: usize| chords.contains(&(i, k));
    let shape = Rim {
        points: &points,
        across: &across,
        joined: &joined,
    };
    let tris = triangulate::triangulate(&shape, tol).ok_or(unfillable)?;
    let mut patch = Patch {
        sigma: vertices.iter().map(|&v| mean_edge(mesh, v)).collect(),
        pos: points,
        rim: rim.len(),
        tris,
    };
    refine::refine(&mut patch, &joined, tol).ok_or(unfillable)?;
    let outer: Vec<Vec<DVec3>> = vertices
        .iter()
        .map(|&v| outer_ring(mesh, v, &vertices))
        .collect();
    fair::fair(&mut patch, &outer).ok_or(unfillable)?;
    let flat = patch
        .tris
        .iter()
        .any(|&[a, b, c]| is_sliver(patch.pos[a], patch.pos[b], patch.pos[c], tol));
    if flat {
        return Err(unfillable);
    }
    Ok(mesh.append_patch(rim, &patch.pos[patch.rim..], &patch.tris))
}

/// The unit normal of the face `h` belongs to; `h` is a loop edge's twin, which has a face.
fn face_normal(mesh: &ManifoldMesh, h: HalfEdgeId) -> DVec3 {
    let [a, b, c] = [h, mesh.next(h), mesh.prev(h)].map(|k| mesh.position(mesh.origin(k)));
    // INVARIANT: a mesh face is not a sliver, so its normal has a direction.
    (b - a).cross(c - a).normalize().unwrap_or(DVec3::ZERO)
}

/// The pairs of loop positions `(i, k)`, `i < k`, whose vertices the mesh already joins.
fn chords(mesh: &ManifoldMesh, vertices: &[VertexId]) -> BTreeSet<(usize, usize)> {
    let position: std::collections::BTreeMap<VertexId, usize> =
        vertices.iter().enumerate().map(|(i, &v)| (v, i)).collect();
    let mut out = BTreeSet::new();
    for (i, &v) in vertices.iter().enumerate() {
        for h in mesh.outgoing(v) {
            if let Some(&k) = position.get(&mesh.dest(h)) {
                out.insert((i.min(k), i.max(k)));
            }
        }
    }
    out
}

/// The mean length of the mesh edges at `v`.
fn mean_edge(mesh: &ManifoldMesh, v: VertexId) -> f64 {
    let lengths: Vec<f64> = mesh
        .outgoing(v)
        .map(|h| (mesh.position(mesh.dest(h)) - mesh.position(v)).length())
        .collect();
    // Lossless: a vertex has far fewer than 2^53 edges.
    lengths.iter().sum::<f64>() / lengths.len().max(1) as f64
}

/// The positions of `v`'s mesh neighbours that are not on the loop.
fn outer_ring(mesh: &ManifoldMesh, v: VertexId, rim: &[VertexId]) -> Vec<DVec3> {
    let mut near: Vec<VertexId> = mesh
        .outgoing(v)
        .map(|h| mesh.dest(h))
        .filter(|w| !rim.contains(w))
        .collect();
    near.sort();
    near.dedup();
    near.into_iter().map(|w| mesh.position(w)).collect()
}

#[cfg(test)]
mod tests;
