//! Seams read from the source model: the edges where its UV map is cut.

use std::collections::BTreeMap;

use openplush_geom::DVec2;
use openplush_geom::epsilon::UV_CUT;
use openplush_mesh::{EdgeId, FaceId, ManifoldMesh};

use crate::{Proposal, ProposalKind, SeamKind, SeamReason, SeamSet, SeamsError};

/// The source model's UV cuts that are not seams yet, and how many faces had no UVs to read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UvCuts {
    edges: Vec<EdgeId>,
    faces_without_uv: u32,
}

impl UvCuts {
    /// The cut edges, ascending.
    pub fn edges(&self) -> &[EdgeId] {
        &self.edges
    }

    /// Faces with a corner that has no UV: their edges are never cuts.
    pub fn faces_without_uv(&self) -> u32 {
        self.faces_without_uv
    }

    /// The cuts as one proposal of [`SeamKind::Cut`] seams for [`SeamReason::SourceModel`], or
    /// `None` when there are none.
    ///
    /// # Errors
    /// Whatever [`Proposal::new`] refuses, which cannot arise: it refuses only an empty edit.
    pub fn proposal(&self) -> Result<Option<Proposal>, SeamsError> {
        if self.edges.is_empty() {
            return Ok(None);
        }
        let add: BTreeMap<EdgeId, SeamKind> =
            self.edges.iter().map(|&e| (e, SeamKind::Cut)).collect();
        // INVARIANT: an imported cut is an ordinary seam. A dart would have to end inside a
        // piece and an opening is left unsewn, and the file says neither.
        debug_assert!(
            add.values()
                .all(|k| !matches!(k, SeamKind::Dart | SeamKind::Opening)),
            "a UV cut proposed as a dart or an opening"
        );
        let count = add.len();
        let edges = if count == 1 { "edge" } else { "edges" };
        let explanation = format!(
            "Seams along the {count} {edges} where the model's UV map is cut, as marked in the \
             modelling tool."
        );
        let kind = ProposalKind::Seam {
            reason: SeamReason::SourceModel,
        };
        Proposal::new(kind, add, explanation).map(Some)
    }
}

/// Every interior edge of `mesh`, not already in `seams`, where the corner UVs on its two sides
/// differ by more than [`UV_CUT`] in either coordinate at either endpoint. An edge is tested
/// only when both its faces have a UV at every corner; a boundary edge is never a cut. A NaN
/// difference reads as a cut, so a broken UV shows up rather than vanishing.
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
    let bare = with_uv.iter().filter(|&&ok| !ok).count();
    Ok(UvCuts {
        edges,
        // Lossless: a mesh's face count fits in u32, as its IDs do.
        faces_without_uv: bare as u32,
    })
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
