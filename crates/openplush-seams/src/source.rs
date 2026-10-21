//! Seams read from the source model: the edges where its UV map is cut or its material changes.

use std::collections::BTreeMap;

use openplush_geom::DVec2;
use openplush_geom::epsilon::UV_CUT;
use openplush_mesh::{EdgeId, FaceId, ManifoldMesh, MaterialIndex, VertexId};

use crate::{Proposal, ProposalKind, SeamKind, SeamReason, SeamSet, SeamsError};

/// At a projection point every incident edge is a cut, so each face round it is its own island
/// there. Hand-marked layouts rarely bring five seams to one vertex, while a pole fan on a
/// 12-segment sphere has twelve faces, and a false hint costs only one sentence of advice.
const PROJECTION_MIN_FACES: usize = 5;

/// The source model's UV cuts and material boundaries that are not seams yet, how many faces
/// had no UVs to read, and where the UVs look like an automatic projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UvCuts {
    edges: Vec<EdgeId>,
    material_boundaries: Vec<EdgeId>,
    faces_without_uv: u32,
    projection_points: Vec<VertexId>,
}

impl UvCuts {
    /// The cut edges, ascending.
    pub fn edges(&self) -> &[EdgeId] {
        &self.edges
    }

    /// Interior edges, ascending, whose two faces both come from the file and differ in
    /// material, a face with no material differing from one with a material. An edge beside a
    /// face repair added is never one, so a filled hole makes no seam.
    pub fn material_boundaries(&self) -> &[EdgeId] {
        &self.material_boundaries
    }

    /// Edges that are both a UV cut and a material boundary.
    pub fn shared_edges(&self) -> u32 {
        let shared = self
            .material_boundaries
            .iter()
            .filter(|e| self.edges.binary_search(e).is_ok())
            .count();
        // Lossless: a mesh's edge count fits in u32, as its IDs do.
        shared as u32
    }

    /// Faces with a corner that has no UV: their edges are never cuts.
    pub fn faces_without_uv(&self) -> u32 {
        self.faces_without_uv
    }

    /// Interior vertices, ascending, with at least five faces round them, all with UVs, whose
    /// corner UVs there differ pairwise as a cut's do. An automatic projection leaves these at a
    /// sphere's poles; so does a hand-marked layout where five or more seams meet. They change
    /// no cut.
    pub fn projection_points(&self) -> &[VertexId] {
        &self.projection_points
    }

    /// The cuts and boundaries as one proposal for [`SeamReason::SourceModel`], or `None` when
    /// there are none: each boundary a [`SeamKind::FabricBoundary`] seam, including one that is
    /// also a cut, and every other cut a [`SeamKind::Cut`] seam.
    ///
    /// # Errors
    /// Whatever [`Proposal::new`] refuses, which cannot arise: it refuses only an empty edit.
    pub fn proposal(&self) -> Result<Option<Proposal>, SeamsError> {
        let mut add: BTreeMap<EdgeId, SeamKind> =
            self.edges.iter().map(|&e| (e, SeamKind::Cut)).collect();
        // The more specific kind wins; both are ordinary seams to the flattener.
        add.extend(
            self.material_boundaries
                .iter()
                .map(|&e| (e, SeamKind::FabricBoundary)),
        );
        if add.is_empty() {
            return Ok(None);
        }
        // INVARIANT: an imported cut or boundary is an ordinary seam. A dart would have to end
        // inside a piece and an opening is left unsewn, and the file says neither.
        debug_assert!(
            add.values()
                .all(|k| !matches!(k, SeamKind::Dart | SeamKind::Opening)),
            "a source seam proposed as a dart or an opening"
        );
        let explanation = explain(&add);
        let kind = ProposalKind::Seam {
            reason: SeamReason::SourceModel,
        };
        Proposal::new(kind, add, explanation).map(Some)
    }
}

fn explain(add: &BTreeMap<EdgeId, SeamKind>) -> String {
    let count = |kind: SeamKind| add.values().filter(|&&k| k == kind).count();
    let edges = |n: usize| if n == 1 { "edge" } else { "edges" };
    let (cuts, boundaries) = (count(SeamKind::Cut), count(SeamKind::FabricBoundary));
    let cut = format!(
        "the {cuts} {} where the model's UV map is cut, as marked in the modelling tool",
        edges(cuts)
    );
    let between = edges(boundaries);
    match (cuts, boundaries) {
        (_, 0) => format!("Seams along {cut}."),
        (0, _) => format!("Seams along the {boundaries} {between} between the model's materials."),
        _ => format!("Seams along {cut}, and the {boundaries} {between} between its materials."),
    }
}

/// The source model's UV cuts and material boundaries, plus the projection points where its UVs
/// look auto-projected.
///
/// A UV cut is an interior edge, not already in `seams`, where the corner UVs on its two sides
/// differ by more than [`UV_CUT`] in either coordinate at either endpoint. An edge is tested
/// only when both its faces have a UV at every corner; a boundary edge is never a cut. A NaN
/// difference reads as a cut, so a broken UV shows up rather than vanishing. A material boundary
/// is likewise left out when it is already in `seams`.
///
/// # Errors
/// `UnknownEdge`, naming the lowest, when `seams` has a seam on an edge `mesh` lacks.
pub fn uv_cuts(mesh: &ManifoldMesh, seams: &SeamSet) -> Result<UvCuts, SeamsError> {
    seams.check_edges(mesh)?;
    let with_uv: Vec<bool> = (0..mesh.face_count() as u32)
        .map(|f| has_uvs(mesh, FaceId::new(f)))
        .collect();
    let edges = (0..mesh.edge_count() as u32)
        .map(EdgeId::new)
        .filter(|&e| seams.get(e).is_none() && is_cut(mesh, &with_uv, e))
        .collect();
    let materials: Vec<Option<Option<MaterialIndex>>> = (0..mesh.face_count() as u32)
        .map(|f| source_material(mesh, FaceId::new(f)))
        .collect();
    let material_boundaries = (0..mesh.edge_count() as u32)
        .map(EdgeId::new)
        .filter(|&e| seams.get(e).is_none() && is_material_boundary(mesh, &materials, e))
        .collect();
    let bare = with_uv.iter().filter(|&&ok| !ok).count();
    let projection_points = (0..mesh.vertex_count() as u32)
        .map(VertexId::new)
        .filter(|&v| is_projection_point(mesh, &with_uv, v))
        .collect();
    Ok(UvCuts {
        edges,
        material_boundaries,
        // Lossless: a mesh's face count fits in u32, as its IDs do.
        faces_without_uv: bare as u32,
        projection_points,
    })
}

fn is_projection_point(mesh: &ManifoldMesh, with_uv: &[bool], v: VertexId) -> bool {
    // A boundary vertex's fan includes its boundary half-edge, which has no face.
    let corners: Option<Vec<DVec2>> = mesh
        .outgoing(v)
        .map(|h| {
            let f = mesh.face(h)?;
            with_uv[f.index() as usize]
                .then(|| mesh.corner_uv(h))
                .flatten()
        })
        .collect();
    corners.is_some_and(|uvs| {
        uvs.len() >= PROJECTION_MIN_FACES
            && uvs
                .iter()
                .enumerate()
                .all(|(i, &a)| uvs[i + 1..].iter().all(|&b| differ(a, b)))
    })
}

/// The material of `f`'s first corner's primitive, or `None` for a face repair added, which has
/// no source corner.
fn source_material(mesh: &ManifoldMesh, f: FaceId) -> Option<Option<MaterialIndex>> {
    mesh.corner_source(mesh.face_half_edge(f))
        .map(|c| mesh.materials().material(c.primitive))
}

fn is_material_boundary(
    mesh: &ManifoldMesh,
    materials: &[Option<Option<MaterialIndex>>],
    edge: EdgeId,
) -> bool {
    let h = edge.half_edge();
    let (Some(left), Some(right)) = (mesh.face(h), mesh.face(h.twin())) else {
        return false;
    };
    match (
        materials[left.index() as usize],
        materials[right.index() as usize],
    ) {
        (Some(a), Some(b)) => a != b,
        _ => false,
    }
}

fn has_uvs(mesh: &ManifoldMesh, f: FaceId) -> bool {
    let h = mesh.face_half_edge(f);
    [h, mesh.next(h), mesh.prev(h)]
        .into_iter()
        .all(|c| mesh.corner_uv(c).is_some())
}

fn is_cut(mesh: &ManifoldMesh, with_uv: &[bool], edge: EdgeId) -> bool {
    let h = edge.half_edge();
    let t = h.twin();
    let (Some(left), Some(right)) = (mesh.face(h), mesh.face(t)) else {
        return false;
    };
    if !(with_uv[left.index() as usize] && with_uv[right.index() as usize]) {
        return false;
    }
    // A corner's UV sits on the half-edge leaving it, so at `h`'s origin the two sides are `h`
    // and `next(t)`, and at its end `next(h)` and `t`.
    [(h, mesh.next(t)), (mesh.next(h), t)]
        .into_iter()
        .any(|(a, b)| {
            mesh.corner_uv(a)
                .zip(mesh.corner_uv(b))
                .is_some_and(|(a, b)| differ(a, b))
        })
}

fn differ(a: DVec2, b: DVec2) -> bool {
    let apart = |d: f64| d.is_nan() || d.abs() > UV_CUT;
    apart(a.x - b.x) || apart(a.y - b.y)
}
