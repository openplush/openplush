//! The fold check: a chart whose surface lies back over itself is refused before flattening, since
//! it flattens to garbage and the flattener would only report that as inverted faces.

use openplush_geom::DVec3;
use openplush_geom::epsilon::FOLD_ANGLE;
use openplush_mesh::{EdgeId, FaceId, ManifoldMesh};

use crate::chart::{cut_test, sides};
use crate::{Chart, SeamSet, SeamsError};

/// `Ok` unless two faces of `chart`, from `charts(mesh, seams)`, share an edge no seam cuts and
/// have normals within `FOLD_ANGLE` of antiparallel. A fold along a seam is two pieces, not a fold.
///
/// # Errors
/// `FoldedChart`, naming the lowest such edge.
pub fn check_fold(mesh: &ManifoldMesh, seams: &SeamSet, chart: &Chart) -> Result<(), SeamsError> {
    let cut = cut_test(mesh, seams, chart);
    let limit = -libm::cos(FOLD_ANGLE);
    let mut folded: Option<EdgeId> = None;
    for &f in chart.faces() {
        let n = normal(mesh, f);
        for h in sides(mesh, f) {
            if cut(h) {
                continue;
            }
            // INVARIANT: an uncut half-edge has a chart face across it.
            let Some(g) = mesh.face(h.twin()) else {
                continue;
            };
            let m = normal(mesh, g);
            // Scaled by both lengths instead of normalising, so there is no division; a face with
            // no area (repair's to reject) compares as unfolded rather than as NaN.
            if n.dot(m) < limit * n.length() * m.length() {
                let edge = h.edge();
                folded = Some(folded.map_or(edge, |low| low.min(edge)));
            }
        }
    }
    match folded {
        Some(edge) => Err(SeamsError::FoldedChart {
            chart: chart.id(),
            edge,
        }),
        None => Ok(()),
    }
}

/// `f`'s normal, scaled by twice its area.
fn normal(mesh: &ManifoldMesh, f: FaceId) -> DVec3 {
    let [a, b, c] = sides(mesh, f).map(|h| mesh.position(mesh.origin(h)));
    let ab = DVec3::new(b.x - a.x, b.y - a.y, b.z - a.z);
    let ac = DVec3::new(c.x - a.x, c.y - a.y, c.z - a.z);
    ab.cross(ac)
}
