//! `debug_validate`: the five half-edge invariants.
//!
//! Construction establishes them; this checks them independently, so tests and
//! debug builds catch any later edit (cutting, repair) that breaks one. It never
//! panics on a corrupted mesh: every link is range-checked before it is followed.

use openplush_geom::epsilon::ModelTolerances;

use super::{ManifoldMesh, is_sliver, ix};
use crate::{FaceId, HalfEdgeId, MeshElement, MeshError, VertexId};

fn broken(invariant: u8, element: MeshElement) -> MeshError {
    MeshError::BrokenInvariant { invariant, element }
}

fn he(i: usize) -> MeshElement {
    MeshElement::HalfEdge(HalfEdgeId::new(i as u32))
}

impl ManifoldMesh {
    /// Checks every invariant and returns the first violation as
    /// [`MeshError::BrokenInvariant`]:
    /// 1. `next(prev(h)) == h` and `origin(next(h)) == dest(h)`, with `next`
    ///    staying in `h`'s face (`twin(twin(h)) == h` holds by construction of the
    ///    paired IDs);
    /// 2. every face's half-edges form loops of length 3, three per face;
    /// 3. each vertex's one-ring is a single fan, with at most one outgoing
    ///    boundary half-edge;
    /// 4. adjacent faces traverse their shared edge in opposite directions. With
    ///    paired twins this follows from 1 (a half-edge ends where its twin starts);
    ///    what is checked here is that an edge's two sides lie in different faces;
    /// 5. no face has area at or below `tol.area()`.
    ///
    /// Invariant 0 covers the structure around them: array lengths (located by
    /// array name), IDs in range, `f_he` pointing into its own face, and the
    /// boundary-vertex rule (`v_out` is the boundary half-edge if one exists).
    pub fn debug_validate(&self, tol: &ModelTolerances) -> Result<(), MeshError> {
        // Each check may assume the ones before it passed: shapes and ranges come
        // first so that everything after can index and follow links safely.
        self.check_storage()?;
        self.check_links_in_range()?;
        self.check_face_half_edges()?;
        self.check_links()?;
        self.check_face_loops()?;
        self.check_edge_sides()?;
        self.check_areas(tol)?;
        self.check_vertices()
    }

    fn check_storage(&self) -> Result<(), MeshError> {
        let n = self.half_edge_count();
        if !n.is_multiple_of(2) {
            return Err(broken(0, MeshElement::Storage("he_next")));
        }
        let lengths = [
            ("he_next", self.he_next.len(), n),
            ("he_prev", self.he_prev.len(), n),
            ("he_origin", self.he_origin.len(), n),
            ("he_face", self.he_face.len(), n),
            ("corner_src", self.corner_src.len(), n),
            ("corner_uv", self.corner_uv.len(), n),
            ("v_out", self.v_out.len(), self.vertex_count()),
        ];
        match lengths.iter().find(|(_, len, want)| len != want) {
            Some(&(name, _, _)) => Err(broken(0, MeshElement::Storage(name))),
            None => Ok(()),
        }
    }

    fn check_links_in_range(&self) -> Result<(), MeshError> {
        let (n, nv, nf) = (
            self.half_edge_count(),
            self.vertex_count(),
            self.face_count(),
        );
        for i in 0..n {
            let in_range = ix(self.he_next[i].index()) < n
                && ix(self.he_prev[i].index()) < n
                && ix(self.he_origin[i].index()) < nv
                && self.he_face[i].is_none_or(|f| ix(f.index()) < nf);
            if !in_range {
                return Err(broken(0, he(i)));
            }
        }
        Ok(())
    }

    fn check_face_half_edges(&self) -> Result<(), MeshError> {
        let n = self.half_edge_count();
        for (f, h) in self.f_he.iter().enumerate() {
            let face = FaceId::new(f as u32);
            if ix(h.index()) >= n || self.he_face[ix(h.index())] != Some(face) {
                return Err(broken(0, MeshElement::Face(face)));
            }
        }
        Ok(())
    }

    /// Invariant 1, per half-edge.
    fn check_links(&self) -> Result<(), MeshError> {
        for i in 0..self.half_edge_count() {
            let h = HalfEdgeId::new(i as u32);
            if self.next(self.prev(h)) != h
                || self.origin(self.next(h)) != self.dest(h)
                || self.face(self.next(h)) != self.face(h)
            {
                return Err(broken(1, he(i)));
            }
        }
        Ok(())
    }

    /// Invariant 2. Every half-edge in a face is on a loop of exactly three, and
    /// there are exactly three per face, so no face has a second (stray) loop.
    fn check_face_loops(&self) -> Result<(), MeshError> {
        let mut in_faces = 0usize;
        for i in 0..self.half_edge_count() {
            let h = HalfEdgeId::new(i as u32);
            if self.face(h).is_none() {
                continue;
            }
            in_faces += 1;
            let (h1, h2) = (self.next(h), self.next(self.next(h)));
            if self.next(h2) != h || h1 == h || h2 == h {
                return Err(broken(2, he(i)));
            }
        }
        if in_faces != 3 * self.face_count() {
            return Err(broken(2, MeshElement::Storage("he_face")));
        }
        Ok(())
    }

    /// Invariant 4: the two sides of an edge lie in different faces.
    fn check_edge_sides(&self) -> Result<(), MeshError> {
        for i in 0..self.half_edge_count() {
            let h = HalfEdgeId::new(i as u32);
            if self.face(h).is_some() && self.face(h) == self.face(h.twin()) {
                return Err(broken(4, he(i)));
            }
        }
        Ok(())
    }

    /// Invariant 5, per face.
    fn check_areas(&self, tol: &ModelTolerances) -> Result<(), MeshError> {
        for (f, &h0) in self.f_he.iter().enumerate() {
            let (h1, h2) = (self.next(h0), self.next(self.next(h0)));
            let (a, b, c) = (
                self.position(self.origin(h0)),
                self.position(self.origin(h1)),
                self.position(self.origin(h2)),
            );
            if is_sliver(a, b, c, tol) {
                return Err(broken(5, MeshElement::Face(FaceId::new(f as u32))));
            }
        }
        Ok(())
    }

    /// The boundary-vertex rule (invariant 0), then invariant 3, vertex by vertex.
    fn check_vertices(&self) -> Result<(), MeshError> {
        let (n, nv) = (self.half_edge_count(), self.vertex_count());
        let mut degree = vec![0usize; nv];
        let mut boundary_out = vec![0usize; nv];
        for i in 0..n {
            let v = ix(self.he_origin[i].index());
            degree[v] += 1;
            boundary_out[v] += usize::from(self.he_face[i].is_none());
        }
        for v in 0..nv {
            let vertex = VertexId::new(v as u32);
            let out = self.v_out[v];
            if ix(out.index()) >= n
                || self.origin(out) != vertex
                || (boundary_out[v] > 0) != self.face(out).is_none()
            {
                return Err(broken(0, MeshElement::Vertex(vertex)));
            }
            if degree[v] == 0 || boundary_out[v] > 1 || self.outgoing(vertex).count() != degree[v] {
                return Err(broken(3, MeshElement::Vertex(vertex)));
            }
        }
        Ok(())
    }
}
