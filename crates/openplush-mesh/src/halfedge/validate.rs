//! `debug_validate` (T026): the five invariants of data-model section 2.
//!
//! Construction establishes them; this checks them independently, so tests and
//! debug builds catch any later edit (cutting, repair) that breaks one. It never
//! panics on a corrupted mesh: every link is range-checked before it is followed.

use openplush_geom::epsilon::ModelTolerances;

use super::{ManifoldMesh, ix};
use crate::{FaceId, HalfEdgeId, MeshElement, MeshError, VertexId};

fn broken(invariant: u8, element: MeshElement) -> MeshError {
    MeshError::BrokenInvariant { invariant, element }
}

impl ManifoldMesh {
    /// Checks every invariant and returns the first violation as
    /// [`MeshError::BrokenInvariant`]:
    /// 1. `next(prev(h)) == h`, `twin(twin(h)) == h`, `origin(next(h)) == origin(twin(h))`;
    /// 2. interior face loops have length 3;
    /// 3. each vertex's one-ring is a single fan;
    /// 4. adjacent faces traverse their shared edge in opposite directions;
    /// 5. no face has area at or below `tol.area()`.
    ///
    /// Invariant 0 covers the structure around them: array lengths, IDs in range,
    /// and the boundary-vertex rule (`v_out` is the boundary half-edge if one exists).
    pub fn debug_validate(&self, tol: &ModelTolerances) -> Result<(), MeshError> {
        let n = self.half_edge_count();
        let nv = self.vertex_count();
        let nf = self.face_count();
        let he = |i: usize| MeshElement::HalfEdge(HalfEdgeId::new(i as u32));

        // 0. Shapes and ranges, so everything below can index safely.
        let lengths_ok = n.is_multiple_of(2)
            && [self.he_prev.len(), self.he_origin.len(), self.he_face.len()]
                .iter()
                .chain([self.corner_src.len(), self.corner_uv.len()].iter())
                .all(|&l| l == n)
            && self.v_out.len() == nv;
        if !lengths_ok {
            return Err(broken(0, MeshElement::HalfEdge(HalfEdgeId::new(0))));
        }
        for i in 0..n {
            let in_range = ix(self.he_next[i].index()) < n
                && ix(self.he_prev[i].index()) < n
                && ix(self.he_origin[i].index()) < nv
                && self.he_face[i].is_none_or(|f| ix(f.index()) < nf);
            if !in_range {
                return Err(broken(0, he(i)));
            }
        }
        for (f, h) in self.f_he.iter().enumerate() {
            if ix(h.index()) >= n || self.he_face[ix(h.index())] != Some(FaceId::new(f as u32)) {
                return Err(broken(0, MeshElement::Face(FaceId::new(f as u32))));
            }
        }

        // 1 and 4, per half-edge.
        for i in 0..n {
            let h = HalfEdgeId::new(i as u32);
            if self.next(self.prev(h)) != h || h.twin().twin() != h {
                return Err(broken(1, he(i)));
            }
            if self.origin(self.next(h)) != self.dest(h) {
                return Err(broken(1, he(i)));
            }
            if self.face(self.next(h)) != self.face(h) {
                return Err(broken(1, he(i)));
            }
            // 4: twins point opposite ways (by 1, h ends where its twin starts, and
            // vice versa) and, between two faces, belong to different faces.
            let twin_face = self.face(h.twin());
            if self.origin(h) == self.dest(h)
                || (self.face(h).is_some() && self.face(h) == twin_face)
            {
                return Err(broken(4, he(i)));
            }
        }

        // 2 and 5, per face.
        for (f, &h0) in self.f_he.iter().enumerate() {
            let face = MeshElement::Face(FaceId::new(f as u32));
            let (h1, h2) = (self.next(h0), self.next(self.next(h0)));
            if self.next(h2) != h0 || h1 == h0 || h2 == h0 {
                return Err(broken(2, face));
            }
            let (a, b, c) = (
                self.position(self.origin(h0)),
                self.position(self.origin(h1)),
                self.position(self.origin(h2)),
            );
            if 0.5 * (b - a).cross(c - a).length() <= tol.area() {
                return Err(broken(5, face));
            }
        }

        // 3 and the boundary-vertex rule, per vertex.
        let mut degree = vec![0usize; nv];
        let mut has_boundary = vec![false; nv];
        for i in 0..n {
            let v = ix(self.he_origin[i].index());
            degree[v] += 1;
            has_boundary[v] |= self.he_face[i].is_none();
        }
        for v in 0..nv {
            let vertex = VertexId::new(v as u32);
            let out = self.v_out[v];
            if ix(out.index()) >= n
                || self.origin(out) != vertex
                || has_boundary[v] != self.face(out).is_none()
            {
                return Err(broken(0, MeshElement::Vertex(vertex)));
            }
            if degree[v] == 0 || self.outgoing(vertex).count() != degree[v] {
                return Err(broken(3, MeshElement::Vertex(vertex)));
            }
        }
        Ok(())
    }
}
