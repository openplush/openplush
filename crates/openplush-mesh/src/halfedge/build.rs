//! Building a `ManifoldMesh` from a clean, consistently oriented soup.
//!
//! Every check returns a typed error naming the element at fault. The order of the
//! checks is fixed, and within each check elements are visited in index order, so
//! the same input always reports the same error (Constitution V).

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

use openplush_geom::DVec2;
use openplush_geom::epsilon::ModelTolerances;

use super::copies::{Copies, corners_out, cut_copies, edge_copies, edge_key};
use super::{ManifoldMesh, is_sliver, ix};
use crate::repair::orientation::Conflict;
use crate::{
    Counted, FaceId, HalfEdgeId, MeshError, PairedEdge, SourceCorner, TriangleSoup, VertexId,
};

/// Placeholder for a link not yet set. Every one is overwritten before the mesh is
/// returned (see the balance argument on `link_boundary`).
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

/// The numbered edges, keyed by (lower, higher) vertex and copy.
type Edges = BTreeMap<(u32, u32, u32), EdgeSlot>;

/// The half-edge arrays, each face linked into a loop; boundary half-edges are
/// not linked yet.
struct Linked {
    he_origin: Vec<VertexId>,
    he_face: Vec<Option<FaceId>>,
    he_next: Vec<HalfEdgeId>,
    he_prev: Vec<HalfEdgeId>,
    f_he: Vec<HalfEdgeId>,
    corner_src: Vec<Option<SourceCorner>>,
    corner_uv: Vec<Option<DVec2>>,
}

impl ManifoldMesh {
    /// Builds the half-edge mesh of `soup`, which must already be clean: no triangle
    /// repeating a vertex or at or below the area tolerance, no two triangles on the
    /// same three vertices, no unused vertices, every edge on at most two triangles,
    /// consistent winding, and a single fan at every vertex. Anything else is
    /// refused with the error for the first violation, in that order.
    ///
    /// Repair is the caller; it is crate-private so that repair stays the
    /// only way to obtain a `ManifoldMesh`.
    #[cfg(test)]
    pub(crate) fn from_oriented_triangles(
        soup: &TriangleSoup,
        tol: &ModelTolerances,
    ) -> Result<Self, MeshError> {
        Self::from_paired_triangles(soup, tol, &[], &[], &[])
    }

    /// As [`Self::from_oriented_triangles`], for a soup with edges on more than two
    /// triangles that `pairings` resolve: each pair of faces gets its own copy of the
    /// edge, even where the copies join the same two vertices, and a face left
    /// unpaired gets a copy of its own. `sheets` are the pairs of coincident faces
    /// (lower face ID first) that are the two layers of a double-sided sheet, which the duplicate check
    /// lets through. A face the pairings do not name shares the first copy. Each of
    /// `conflicts` is a two-face edge cut into two boundary copies, one per face.
    pub(crate) fn from_paired_triangles(
        soup: &TriangleSoup,
        tol: &ModelTolerances,
        pairings: &[PairedEdge],
        sheets: &[[FaceId; 2]],
        conflicts: &[Conflict],
    ) -> Result<Self, MeshError> {
        refuse_bad_triangles(soup, tol)?;
        refuse_duplicates(soup.triangle_indices(), sheets)?;
        let mut copies = edge_copies(soup.triangle_indices(), pairings)?;
        cut_copies(soup.triangle_indices(), conflicts, &mut copies)?;
        refuse_isolated(soup)?;
        let edges = number_edges(soup.triangle_indices(), &copies)?;
        let mut linked = link_faces(soup, &edges, &copies)?;
        let boundary_out = link_boundary(&mut linked, soup.positions().len())?;
        let mesh = Self {
            pos: soup.positions().to_vec(),
            v_out: vertex_out(boundary_out, &linked.he_origin),
            he_next: linked.he_next,
            he_prev: linked.he_prev,
            he_origin: linked.he_origin,
            he_face: linked.he_face,
            f_he: linked.f_he,
            corner_src: linked.corner_src,
            corner_uv: linked.corner_uv,
            materials: soup.materials().clone(),
            extra_uv_sets: soup.extra_uv_sets(),
            edge_copies: Vec::new(),
        };
        mesh.refuse_split_fans()?;
        if cfg!(debug_assertions) {
            mesh.debug_validate(tol)?;
        }
        Ok(mesh)
    }

    /// Refuses a vertex whose outgoing half-edges form more than one fan.
    ///
    /// Rotating from `v_out` must visit every outgoing half-edge. Two fans can share
    /// a vertex without any boundary there (two cones tip to tip), so the bowtie
    /// check in `link_boundary` alone does not catch this.
    fn refuse_split_fans(&self) -> Result<(), MeshError> {
        let mut degree = vec![0usize; self.vertex_count()];
        for o in &self.he_origin {
            degree[ix(o.index())] += 1;
        }
        for (v, &d) in degree.iter().enumerate() {
            let vertex = VertexId::new(v as u32);
            if self.outgoing(vertex).count() != d {
                return Err(MeshError::NonManifoldVertex { vertex });
            }
        }
        Ok(())
    }
}

/// Refuses a triangle that repeats a corner or has area at or below tolerance.
fn refuse_bad_triangles(soup: &TriangleSoup, tol: &ModelTolerances) -> Result<(), MeshError> {
    let pos = soup.positions();
    for (t, &[a, b, c]) in soup.triangle_indices().iter().enumerate() {
        // Lossless: the soup's triangle count fits in u32.
        let triangle = FaceId::new(t as u32);
        if a == b || b == c || a == c {
            return Err(MeshError::DegenerateTriangle { triangle });
        }
        if is_sliver(pos[ix(a)], pos[ix(b)], pos[ix(c)], tol) {
            return Err(MeshError::ZeroAreaTriangle { triangle });
        }
    }
    Ok(())
}

/// Refuses two triangles on the same three vertices, in either winding, unless they are
/// the two layers of a sheet.
fn refuse_duplicates(tris: &[[u32; 3]], sheets: &[[FaceId; 2]]) -> Result<(), MeshError> {
    let mut seen: BTreeMap<[u32; 3], FaceId> = BTreeMap::new();
    for (t, tri) in tris.iter().enumerate() {
        let triangle = FaceId::new(t as u32);
        let mut key = *tri;
        key.sort();
        match seen.entry(key) {
            Entry::Occupied(first) => {
                let first = *first.get();
                if !sheets.contains(&[first, triangle]) {
                    return Err(MeshError::DuplicateTriangle { triangle, first });
                }
            }
            Entry::Vacant(slot) => {
                slot.insert(triangle);
            }
        }
    }
    Ok(())
}

/// Refuses the lowest vertex no triangle uses.
fn refuse_isolated(soup: &TriangleSoup) -> Result<(), MeshError> {
    let mut used = vec![false; soup.positions().len()];
    for &v in soup.triangle_indices().iter().flatten() {
        used[ix(v)] = true;
    }
    match used.iter().position(|&u| !u) {
        Some(v) => Err(MeshError::IsolatedVertex {
            vertex: VertexId::new(v as u32),
        }),
        None => Ok(()),
    }
}

/// Numbers the edges in order of first appearance and counts their uses, refusing
/// an edge on more than two triangles or too many half-edges to index.
fn number_edges(tris: &[[u32; 3]], copies: &Copies) -> Result<Edges, MeshError> {
    let mut edges = Edges::new();
    for (t, [a, b]) in tris
        .iter()
        .enumerate()
        .flat_map(|(t, tri)| corners_out(tri).into_iter().map(move |ab| (t, ab)))
    {
        let next_id = edges.len() as u32;
        let slot = edges.entry(edge_key(copies, t, a, b)).or_insert(EdgeSlot {
            id: next_id,
            from: a,
            to: b,
            uses: 0,
        });
        slot.uses += 1;
    }
    if let Some(e) = in_id_order(&edges).iter().find(|e| e.uses > 2) {
        return Err(MeshError::NonManifoldEdge {
            a: VertexId::new(e.from.min(e.to)),
            b: VertexId::new(e.from.max(e.to)),
        });
    }
    let half_edges = edges.len() * 2;
    if u32::try_from(half_edges).is_err() {
        return Err(MeshError::TooLarge {
            what: Counted::HalfEdges,
            count: half_edges,
        });
    }
    Ok(edges)
}

fn in_id_order(edges: &Edges) -> Vec<&EdgeSlot> {
    let mut by_id: Vec<&EdgeSlot> = edges.values().collect();
    by_id.sort_by_key(|e| e.id);
    by_id
}

/// Creates the half-edges and links each face, refusing a second triangle that
/// crosses an edge in the same direction. Edge e's first-seen direction is 2e, the
/// other 2e + 1.
fn link_faces(soup: &TriangleSoup, edges: &Edges, copies: &Copies) -> Result<Linked, MeshError> {
    let tris = soup.triangle_indices();
    let half_edges = edges.len() * 2;
    let mut l = Linked {
        he_origin: in_id_order(edges)
            .iter()
            .flat_map(|e| [VertexId::new(e.from), VertexId::new(e.to)])
            .collect(),
        he_face: vec![None; half_edges],
        he_next: vec![UNSET; half_edges],
        he_prev: vec![UNSET; half_edges],
        f_he: Vec::with_capacity(tris.len()),
        corner_src: vec![None; half_edges],
        corner_uv: vec![None; half_edges],
    };
    let sources = soup.corner_sources();
    let uvs = soup.corner_uvs();
    for (t, tri) in tris.iter().enumerate() {
        let face = FaceId::new(t as u32);
        let mut hs = [UNSET; 3];
        for (i, [a, b]) in corners_out(tri).into_iter().enumerate() {
            // INVARIANT: `number_edges` keyed every directed edge of every triangle.
            let e = &edges[&edge_key(copies, t, a, b)];
            let h = HalfEdgeId::new(2 * e.id + u32::from(a != e.from));
            if l.he_face[ix(h.index())].is_some() {
                return Err(MeshError::InconsistentOrientation {
                    a: VertexId::new(a.min(b)),
                    b: VertexId::new(a.max(b)),
                });
            }
            l.he_face[ix(h.index())] = Some(face);
            l.corner_src[ix(h.index())] = sources.get(3 * t + i).copied();
            l.corner_uv[ix(h.index())] = uvs.and_then(|u| u.get(3 * t + i).copied().flatten());
            hs[i] = h;
        }
        for i in 0..3 {
            l.he_next[ix(hs[i].index())] = hs[(i + 1) % 3];
            l.he_prev[ix(hs[(i + 1) % 3].index())] = hs[i];
        }
        l.f_he.push(hs[0]);
    }
    Ok(l)
}

/// Links the boundary half-edges into loops and returns each vertex's outgoing
/// boundary half-edge, refusing the lowest vertex with more than one (a bowtie).
///
/// Around any vertex, the triangles use as many half-edges in as out, and so do
/// the edges, so boundary half-edges in and out balance too. A vertex with one
/// outgoing boundary half-edge links every arriving boundary half-edge to it.
fn link_boundary(
    l: &mut Linked,
    vertex_count: usize,
) -> Result<Vec<Option<HalfEdgeId>>, MeshError> {
    let mut boundary_out = vec![None; vertex_count];
    let mut bowtie = None;
    for (i, face) in l.he_face.iter().enumerate() {
        if face.is_none() {
            let v = ix(l.he_origin[i].index());
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
    for (i, face) in l.he_face.iter().enumerate() {
        if face.is_none() {
            let h = HalfEdgeId::new(i as u32);
            let dest = l.he_origin[ix(h.twin().index())];
            // Exists by the balance argument above.
            if let Some(n) = boundary_out[ix(dest.index())] {
                l.he_next[i] = n;
                l.he_prev[ix(n.index())] = h;
            }
        }
    }
    Ok(boundary_out)
}

/// Each vertex's `v_out`: its boundary half-edge if it has one, else its
/// lowest-indexed outgoing half-edge.
fn vertex_out(boundary_out: Vec<Option<HalfEdgeId>>, he_origin: &[VertexId]) -> Vec<HalfEdgeId> {
    let mut v_out = boundary_out;
    for (i, &o) in he_origin.iter().enumerate() {
        let slot = &mut v_out[ix(o.index())];
        if slot.is_none() {
            *slot = Some(HalfEdgeId::new(i as u32));
        }
    }
    v_out.into_iter().map(|h| h.unwrap_or(UNSET)).collect()
}
