//! Building a `ManifoldMesh` from a clean, consistently oriented soup (T025).
//!
//! Every check returns a typed error naming the element at fault. The order of the
//! checks is fixed, and within each check elements are visited in index order, so
//! the same input always reports the same error (Constitution V).

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

use openplush_geom::epsilon::ModelTolerances;

use super::{ManifoldMesh, is_sliver, ix};
use crate::{Counted, FaceId, HalfEdgeId, MeshError, TriangleSoup, VertexId};

/// Placeholder for a link not yet set. Every one is overwritten before the mesh is
/// returned (see the boundary-linking argument below).
const UNSET: HalfEdgeId = HalfEdgeId::new(u32::MAX);

/// One undirected edge while building.
struct EdgeSlot {
    /// Index in order of first appearance.
    id: u32,
    /// The direction the edge was first seen in; its half-edge is `2 * id`.
    from: u32,
    to: u32,
    /// Triangles using the edge, in either direction.
    uses: u32,
}

impl ManifoldMesh {
    /// Builds the half-edge mesh of `soup`, which must already be clean: no triangle
    /// repeating a vertex or at or below the area tolerance, no two triangles on the
    /// same three vertices, no unused vertices, every edge on at most two triangles,
    /// consistent winding, and a single fan at every vertex. Anything else is
    /// refused with the error for the first violation, in that order.
    ///
    /// Repair (T067) is the caller; it is crate-private so that repair stays the
    /// only way to obtain a `ManifoldMesh`.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "called by repair (T067), not yet written")
    )]
    pub(crate) fn from_oriented_triangles(
        soup: &TriangleSoup,
        tol: &ModelTolerances,
    ) -> Result<Self, MeshError> {
        let pos = soup.positions().to_vec();
        let tris = soup.triangle_indices();

        // 1. Each triangle on its own: distinct corners, area above tolerance.
        for (t, &[a, b, c]) in tris.iter().enumerate() {
            // Lossless: the soup's triangle count fits in u32.
            let triangle = FaceId::new(t as u32);
            if a == b || b == c || a == c {
                return Err(MeshError::DegenerateTriangle { triangle });
            }
            if is_sliver(pos[ix(a)], pos[ix(b)], pos[ix(c)], tol) {
                return Err(MeshError::ZeroAreaTriangle { triangle });
            }
        }

        // 1b. No two triangles on the same three vertices, in either winding.
        let mut seen: BTreeMap<[u32; 3], FaceId> = BTreeMap::new();
        for (t, tri) in tris.iter().enumerate() {
            let triangle = FaceId::new(t as u32);
            let mut key = *tri;
            key.sort_unstable();
            match seen.entry(key) {
                Entry::Occupied(first) => {
                    return Err(MeshError::DuplicateTriangle {
                        triangle,
                        first: *first.get(),
                    });
                }
                Entry::Vacant(slot) => {
                    slot.insert(triangle);
                }
            }
        }

        // 2. Every vertex is used.
        let mut used = vec![false; pos.len()];
        for &v in tris.iter().flatten() {
            used[ix(v)] = true;
        }
        if let Some(v) = used.iter().position(|&u| !u) {
            return Err(MeshError::IsolatedVertex {
                vertex: VertexId::new(v as u32),
            });
        }

        // 3. Number the edges in order of first appearance, and count their uses.
        let mut edges: BTreeMap<(u32, u32), EdgeSlot> = BTreeMap::new();
        for [a, b] in tris.iter().flat_map(corners_out) {
            let next_id = edges.len() as u32;
            let slot = edges.entry((a.min(b), a.max(b))).or_insert(EdgeSlot {
                id: next_id,
                from: a,
                to: b,
                uses: 0,
            });
            slot.uses += 1;
        }
        let mut by_id: Vec<&EdgeSlot> = edges.values().collect();
        by_id.sort_by_key(|e| e.id);
        if let Some(e) = by_id.iter().find(|e| e.uses > 2) {
            return Err(MeshError::NonManifoldEdge {
                a: VertexId::new(e.from.min(e.to)),
                b: VertexId::new(e.from.max(e.to)),
            });
        }
        let half_edges = by_id.len() * 2;
        if u32::try_from(half_edges).is_err() {
            return Err(MeshError::TooLarge {
                what: Counted::HalfEdges,
                count: half_edges,
            });
        }

        // 4. Half-edges. Edge e's first-seen direction is 2e, the other 2e + 1.
        let mut he_origin = Vec::with_capacity(half_edges);
        for e in &by_id {
            he_origin.push(VertexId::new(e.from));
            he_origin.push(VertexId::new(e.to));
        }
        let mut he_face = vec![None; half_edges];
        let mut he_next = vec![UNSET; half_edges];
        let mut he_prev = vec![UNSET; half_edges];
        let mut corner_src = vec![None; half_edges];
        let mut corner_uv = vec![None; half_edges];
        let mut f_he = Vec::with_capacity(tris.len());
        let sources = soup.corner_sources();
        let uvs = soup.corner_uvs();
        for (t, tri) in tris.iter().enumerate() {
            let face = FaceId::new(t as u32);
            let mut hs = [UNSET; 3];
            for (i, [a, b]) in corners_out(tri).into_iter().enumerate() {
                let e = &edges[&(a.min(b), a.max(b))];
                let h = HalfEdgeId::new(2 * e.id + u32::from(a != e.from));
                if he_face[ix(h.index())].is_some() {
                    // A second triangle crossing the edge in the same direction.
                    return Err(MeshError::InconsistentOrientation {
                        a: VertexId::new(a.min(b)),
                        b: VertexId::new(a.max(b)),
                    });
                }
                he_face[ix(h.index())] = Some(face);
                corner_src[ix(h.index())] = sources.get(3 * t + i).copied();
                corner_uv[ix(h.index())] = uvs.and_then(|u| u.get(3 * t + i).copied().flatten());
                hs[i] = h;
            }
            for i in 0..3 {
                he_next[ix(hs[i].index())] = hs[(i + 1) % 3];
                he_prev[ix(hs[(i + 1) % 3].index())] = hs[i];
            }
            f_he.push(hs[0]);
        }

        // 5. Boundary half-edges. Around any vertex, the triangles use as many
        //    half-edges in as out, and so do the edges, so boundary half-edges in and
        //    out balance too. A vertex with one outgoing boundary half-edge links
        //    every arriving boundary half-edge to it; more than one is a bowtie.
        let mut boundary_out = vec![None; pos.len()];
        let mut bowtie = None;
        for (i, face) in he_face.iter().enumerate() {
            if face.is_none() {
                let v = ix(he_origin[i].index());
                if boundary_out[v].is_some() {
                    bowtie = Some(bowtie.map_or(v, |w: usize| w.min(v)));
                }
                boundary_out[v] = Some(HalfEdgeId::new(i as u32));
            }
        }
        if let Some(v) = bowtie {
            return Err(MeshError::NonManifoldVertex {
                vertex: VertexId::new(v as u32),
            });
        }
        for (i, face) in he_face.iter().enumerate() {
            if face.is_none() {
                let h = HalfEdgeId::new(i as u32);
                let dest = he_origin[ix(h.twin().index())];
                // Exists by the balance argument above.
                if let Some(n) = boundary_out[ix(dest.index())] {
                    he_next[i] = n;
                    he_prev[ix(n.index())] = h;
                }
            }
        }

        // 6. Each vertex points at its boundary half-edge if it has one, else at its
        //    lowest-indexed outgoing half-edge.
        let mut v_out = boundary_out;
        for (i, &o) in he_origin.iter().enumerate() {
            let slot = &mut v_out[ix(o.index())];
            if slot.is_none() {
                *slot = Some(HalfEdgeId::new(i as u32));
            }
        }
        let v_out: Vec<HalfEdgeId> = v_out.into_iter().map(|h| h.unwrap_or(UNSET)).collect();

        let mesh = Self {
            pos,
            v_out,
            he_next,
            he_prev,
            he_origin,
            he_face,
            f_he,
            corner_src,
            corner_uv,
        };

        // 7. A single fan at every vertex: rotating from `v_out` must visit every
        //    outgoing half-edge. Two fans can share a vertex without any boundary
        //    there (two cones tip to tip), so step 5 alone does not catch this.
        let mut degree = vec![0usize; mesh.vertex_count()];
        for o in &mesh.he_origin {
            degree[ix(o.index())] += 1;
        }
        for (v, &d) in degree.iter().enumerate() {
            let vertex = VertexId::new(v as u32);
            if mesh.outgoing(vertex).count() != d {
                return Err(MeshError::NonManifoldVertex { vertex });
            }
        }

        Ok(mesh)
    }
}

/// A triangle's three directed edges, corner i to corner i + 1.
fn corners_out(&[a, b, c]: &[u32; 3]) -> [[u32; 2]; 3] {
    [[a, b], [b, c], [c, a]]
}
