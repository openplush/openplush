//! The half-edge `ManifoldMesh`.
//!
//! Index arenas in struct-of-arrays layout, with newtype IDs. Twins are paired
//! indices, boundary half-edges are explicit (`face = None`), and `prev` is stored.
//! Per-corner attributes (provenance, UVs) live on half-edges, because a vertex on a
//! UV seam has one value per corner.
//!
//! **Preconditions on every accessor**: an ID passed in must come from this mesh.
//! IDs are never user input; they come out of the mesh itself, so an out-of-range ID
//! is a caller bug and indexing panics on it, like a slice.

mod build;
mod copies;
mod validate;

use openplush_geom::epsilon::{AREA_RELATIVE_SQ, ModelTolerances};
use openplush_geom::{DVec2, DVec3};

use crate::{FaceId, HalfEdgeId, SourceCorner, SourceMaterials, VertexId};

/// A triangle mesh satisfying the five invariants of `debug_validate`: a
/// consistently oriented 2-manifold, possibly with boundary, with no degenerate or
/// zero-area face and no isolated vertex.
///
/// There is no public constructor. Repair is the only producer, so
/// unrepaired data cannot reach the flattener by construction (Constitution VII).
#[derive(Debug, Clone, PartialEq)]
pub struct ManifoldMesh {
    // per vertex
    pos: Vec<DVec3>,
    /// Boundary vertices point at their (unique) outgoing boundary half-edge.
    v_out: Vec<HalfEdgeId>,
    // per half-edge
    he_next: Vec<HalfEdgeId>,
    he_prev: Vec<HalfEdgeId>,
    he_origin: Vec<VertexId>,
    he_face: Vec<Option<FaceId>>,
    // per face
    f_he: Vec<HalfEdgeId>,
    // per corner, stored per half-edge (`None` on boundary half-edges)
    corner_src: Vec<Option<SourceCorner>>,
    corner_uv: Vec<Option<DVec2>>,
    // file-level, moved unchanged from the soup
    materials: SourceMaterials,
    extra_uv_sets: u32,
}

/// True if triangle (a, b, c) has area at or below `tol.area()`. The edge vectors
/// are divided by the diagonal first and the result compared with the relative
/// threshold, so the cross product stays near 1 and cannot overflow at any
/// diagonal `ModelTolerances` accepts. (Unscaled, |e1 x e2|^2 overflows above a
/// diagonal of about 2.6e82, and an overflowed area reads as "large enough".)
pub(crate) fn is_sliver(a: DVec3, b: DVec3, c: DVec3, tol: &ModelTolerances) -> bool {
    let inv = 1.0 / tol.diagonal();
    0.5 * ((b - a) * inv).cross((c - a) * inv).length() <= AREA_RELATIVE_SQ
}

#[inline]
fn ix(i: u32) -> usize {
    // Lossless: usize is at least 32 bits on every supported target.
    i as usize
}

impl ManifoldMesh {
    /// Number of vertices.
    pub fn vertex_count(&self) -> usize {
        self.pos.len()
    }

    /// Number of undirected edges.
    pub fn edge_count(&self) -> usize {
        self.he_next.len() / 2
    }

    /// Number of half-edges, boundary ones included: twice the edge count.
    pub fn half_edge_count(&self) -> usize {
        self.he_next.len()
    }

    /// Number of faces.
    pub fn face_count(&self) -> usize {
        self.f_he.len()
    }

    /// Position of `v`.
    pub fn position(&self, v: VertexId) -> DVec3 {
        self.pos[ix(v.index())]
    }

    /// An outgoing half-edge of `v`. For a boundary vertex it is the outgoing
    /// boundary half-edge, so one-ring iteration has a deterministic start.
    pub fn vertex_out(&self, v: VertexId) -> HalfEdgeId {
        self.v_out[ix(v.index())]
    }

    /// True if `v` lies on the boundary. O(1) by the boundary-vertex rule.
    pub fn is_boundary_vertex(&self, v: VertexId) -> bool {
        self.face(self.vertex_out(v)).is_none()
    }

    /// The next half-edge around `h`'s face or boundary loop.
    pub fn next(&self, h: HalfEdgeId) -> HalfEdgeId {
        self.he_next[ix(h.index())]
    }

    /// The previous half-edge around `h`'s face or boundary loop.
    pub fn prev(&self, h: HalfEdgeId) -> HalfEdgeId {
        self.he_prev[ix(h.index())]
    }

    /// The vertex `h` leaves.
    pub fn origin(&self, h: HalfEdgeId) -> VertexId {
        self.he_origin[ix(h.index())]
    }

    /// The vertex `h` arrives at.
    pub fn dest(&self, h: HalfEdgeId) -> VertexId {
        self.origin(h.twin())
    }

    /// The face to the left of `h`, or `None` for a boundary half-edge.
    pub fn face(&self, h: HalfEdgeId) -> Option<FaceId> {
        self.he_face[ix(h.index())]
    }

    /// The half-edge leaving corner 0 of `f`: it starts at the face's first vertex
    /// in the input's winding.
    pub fn face_half_edge(&self, f: FaceId) -> HalfEdgeId {
        self.f_he[ix(f.index())]
    }

    /// Provenance of the corner at `h`'s origin in `h`'s face; `None` for a
    /// boundary half-edge.
    pub fn corner_source(&self, h: HalfEdgeId) -> Option<SourceCorner> {
        self.corner_src[ix(h.index())]
    }

    /// UV of the corner at `h`'s origin in `h`'s face; `None` for a boundary
    /// half-edge or a corner the source gave no UV.
    pub fn corner_uv(&self, h: HalfEdgeId) -> Option<DVec2> {
        self.corner_uv[ix(h.index())]
    }

    /// The source file's materials, unchanged by repair.
    pub fn materials(&self) -> &SourceMaterials {
        &self.materials
    }

    /// UV sets the source file has beyond the one read.
    pub fn extra_uv_sets(&self) -> u32 {
        self.extra_uv_sets
    }

    /// The half-edges leaving `v`, each exactly once, starting at
    /// [`vertex_out`](Self::vertex_out) and turning counter-clockwise (seen from
    /// the side the counter-clockwise faces face). At a boundary vertex that is the
    /// boundary half-edge first, then the interior ones in order across the fan.
    pub fn outgoing(&self, v: VertexId) -> Outgoing<'_> {
        let start = self.vertex_out(v);
        Outgoing {
            mesh: self,
            start,
            current: Some(start),
            // Bound the walk even on a corrupted mesh (see `debug_validate`).
            remaining: self.half_edge_count(),
        }
    }

    /// Every boundary loop, each as its half-edges in `next` order. Loops are
    /// ordered by their lowest half-edge index, and each starts there, so the result
    /// is deterministic. Empty for a closed mesh.
    pub fn boundary_loops(&self) -> Vec<Vec<HalfEdgeId>> {
        let n = self.half_edge_count();
        let mut seen = vec![false; n];
        let mut loops = Vec::new();
        for (i, face) in self.he_face.iter().enumerate() {
            if face.is_some() || seen[i] {
                continue;
            }
            // Lossless: the half-edge count fits in u32 (checked at construction).
            let start = HalfEdgeId::new(i as u32);
            let mut ring = Vec::new();
            let mut h = start;
            // Bounded, so a corrupted `next` cannot loop forever.
            for _ in 0..n {
                seen[ix(h.index())] = true;
                ring.push(h);
                h = self.next(h);
                if h == start {
                    break;
                }
            }
            loops.push(ring);
        }
        loops
    }

    /// Test-only: break invariant 1 at `h` so `debug_validate` can be shown to
    /// catch it.
    #[cfg(test)]
    pub(crate) fn corrupt_next_for_test(&mut self, h: HalfEdgeId) {
        let skip = self.next(self.next(h));
        self.he_next[ix(h.index())] = skip;
    }
}

/// Iterator over a vertex's outgoing half-edges; see [`ManifoldMesh::outgoing`].
#[derive(Debug, Clone)]
pub struct Outgoing<'a> {
    mesh: &'a ManifoldMesh,
    start: HalfEdgeId,
    current: Option<HalfEdgeId>,
    remaining: usize,
}

impl Iterator for Outgoing<'_> {
    type Item = HalfEdgeId;

    fn next(&mut self) -> Option<HalfEdgeId> {
        let h = self.current?;
        self.remaining = self.remaining.checked_sub(1)?;
        // Rotate: the half-edge arriving before `h` in its loop, reversed, leaves
        // the same vertex. Works across the boundary because boundary half-edges
        // have `next`/`prev` too.
        let n = self.mesh.prev(h).twin();
        self.current = (n != self.start).then_some(n);
        Some(h)
    }
}

#[cfg(test)]
mod paired_tests;
#[cfg(test)]
mod tests;

// Native only: proptest's dependencies don't build for wasm32. Two attributes,
// not cfg(all(..)), so clippy still recognises a test module.
#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod proptests;
