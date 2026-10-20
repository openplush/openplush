//! Appending a patch that closes one boundary loop. New vertices, edges and faces take the
//! next IDs, so every ID the mesh had stays valid; the loop's boundary half-edges become the
//! patch's, and the patch's corners carry no provenance and no UV.

use std::collections::BTreeMap;

use openplush_geom::DVec3;

use super::{ManifoldMesh, ix};
use crate::{FaceId, HalfEdgeId, VertexId};

impl ManifoldMesh {
    /// Closes the boundary loop `rim` (its half-edges in `next` order) with `tris`, whose
    /// vertex `i` is `rim[i]`'s origin for `i < rim.len()` and `added[i - rim.len()]` after.
    /// Each triangle is wound the loop's way. The patch's faces, in `tris` order.
    ///
    /// Preconditions, which the fill establishes: every loop edge is used by exactly one
    /// triangle and every other edge of the patch by exactly two; no patch edge between two
    /// rim vertices is already an edge of the mesh; no triangle is a sliver.
    pub(crate) fn append_patch(
        &mut self,
        rim: &[HalfEdgeId],
        added: &[DVec3],
        tris: &[[usize; 3]],
    ) -> Vec<FaceId> {
        let n = rim.len();
        let first_vertex = self.pos.len();
        let vertex = |i: usize| -> VertexId {
            if i < n {
                self.he_origin[ix(rim[i].index())]
            } else {
                // Lossless: the mesh's counts fit in u32 (checked at construction).
                VertexId::new((first_vertex + i - n) as u32)
            }
        };
        let ends: Vec<VertexId> = (0..n + added.len()).map(vertex).collect();
        self.pos.extend_from_slice(added);
        // Placeholders, each overwritten below by a half-edge leaving the new vertex.
        self.v_out.extend(added.iter().map(|_| HalfEdgeId::new(0)));
        let mut new_edges: BTreeMap<(usize, usize), HalfEdgeId> = BTreeMap::new();
        let mut faces = Vec::with_capacity(tris.len());
        for &tri in tris {
            let f = FaceId::new(self.f_he.len() as u32);
            let mut ring = [HalfEdgeId::new(0); 3];
            for (k, slot) in ring.iter_mut().enumerate() {
                let (u, v) = (tri[k], tri[(k + 1) % 3]);
                *slot = if u < n && v == (u + 1) % n {
                    rim[u]
                } else {
                    self.patch_half_edge(&mut new_edges, (u, v), (ends[u], ends[v]))
                };
                if u >= n {
                    self.v_out[ix(ends[u].index())] = *slot;
                }
            }
            for k in 0..3 {
                let h = ix(ring[k].index());
                self.he_face[h] = Some(f);
                self.he_next[h] = ring[(k + 1) % 3];
                self.he_prev[h] = ring[(k + 2) % 3];
                self.corner_src[h] = None;
                self.corner_uv[h] = None;
            }
            self.f_he.push(ring[0]);
            faces.push(f);
        }
        faces
    }

    /// The half-edge from `ends.0` to `ends.1` of the patch edge between local vertices
    /// `local`, made with its twin the first time either direction is asked for.
    fn patch_half_edge(
        &mut self,
        made: &mut BTreeMap<(usize, usize), HalfEdgeId>,
        local: (usize, usize),
        ends: (VertexId, VertexId),
    ) -> HalfEdgeId {
        let key = (local.0.min(local.1), local.0.max(local.1));
        if let Some(&h) = made.get(&key) {
            return if self.he_origin[ix(h.index())] == ends.0 {
                h
            } else {
                h.twin()
            };
        }
        // Lossless: the mesh's counts fit in u32 (checked at construction).
        let h = HalfEdgeId::new(self.he_origin.len() as u32);
        for origin in [ends.0, ends.1] {
            self.he_origin.push(origin);
            self.he_face.push(None);
            self.he_next.push(h);
            self.he_prev.push(h);
            self.corner_src.push(None);
            self.corner_uv.push(None);
        }
        made.insert(key, h);
        h
    }
}
