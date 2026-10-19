//! Charts: the regions of faces the seams cut the mesh into.

use openplush_mesh::{FaceId, HalfEdgeId, ManifoldMesh};

use crate::{ChartId, SeamSet, SeamsError};

/// The faces reachable from each other without crossing a seam.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chart {
    id: ChartId,
    faces: Vec<FaceId>,
}

impl Chart {
    /// Its lowest `FaceId`.
    pub fn id(&self) -> ChartId {
        self.id
    }

    /// Its faces, in ascending order.
    pub fn faces(&self) -> &[FaceId] {
        &self.faces
    }
}

/// Every chart of `seams` on `mesh`, in ID order. A dart, or any seam that does not separate
/// faces, slits a chart rather than splitting it.
///
/// # Errors
/// `UnknownEdge`, naming the lowest, when a seam is on an edge `mesh` does not have.
pub fn charts(mesh: &ManifoldMesh, seams: &SeamSet) -> Result<Vec<Chart>, SeamsError> {
    seams.check_edges(mesh)?;
    let mut seen = vec![false; mesh.face_count()];
    let mut charts = Vec::new();
    // Lossless: the face count fits in u32 (checked when the mesh is built).
    for first in (0..mesh.face_count() as u32).map(FaceId::new) {
        if seen[first.index() as usize] {
            continue;
        }
        seen[first.index() as usize] = true;
        let mut faces = vec![first];
        let mut i = 0;
        while let Some(&f) = faces.get(i) {
            i += 1;
            for h in sides(mesh, f) {
                if seams.get(h.edge()).is_some() {
                    continue;
                }
                if let Some(g) = mesh.face(h.twin())
                    && !std::mem::replace(&mut seen[g.index() as usize], true)
                {
                    faces.push(g);
                }
            }
        }
        faces.sort();
        // Faces are tried in ascending order, so a chart's first face is its lowest.
        charts.push(Chart {
            id: ChartId::new(first.index()),
            faces,
        });
    }
    Ok(charts)
}

/// Whether a half-edge of `chart` is cut: a seam runs along it or the face across it is not in
/// the chart.
pub(crate) fn cut_test<'a>(
    mesh: &'a ManifoldMesh,
    seams: &'a SeamSet,
    chart: &Chart,
) -> impl Fn(HalfEdgeId) -> bool + 'a {
    let mut inside = vec![false; mesh.face_count()];
    for f in chart.faces() {
        inside[f.index() as usize] = true;
    }
    move |h| {
        seams.get(h.edge()).is_some()
            || !mesh
                .face(h.twin())
                .is_some_and(|g| inside[g.index() as usize])
    }
}

/// `f`'s half-edges in its corner order.
pub(crate) fn sides(mesh: &ManifoldMesh, f: FaceId) -> [HalfEdgeId; 3] {
    let h = mesh.face_half_edge(f);
    [h, mesh.next(h), mesh.prev(h)]
}
