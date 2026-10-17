use openplush_geom::epsilon::ModelTolerances;

use super::report::{Duplicate, DuplicateOrientation};
use crate::halfedge::is_sliver;
use crate::repair::degenerate::Removed;
use crate::{FaceId, TriangleSoup};

/// Faces with area at or below the area tolerance, from the soup's own positions,
/// so a face is zero-area exactly when repair would stitch it out.
// INVARIANT: soup indices are in range (`TriangleSoup::new`).
pub(super) fn zero_area(soup: &TriangleSoup, tol: &ModelTolerances) -> Vec<FaceId> {
    let p = |v: u32| soup.positions()[v as usize];
    (0..)
        .zip(soup.triangle_indices())
        .filter(|&(_, &[a, b, c])| is_sliver(p(a), p(b), p(c), tol))
        .map(|(f, _)| FaceId::new(f))
        .collect()
}

/// Every copy repair drops or keeps as a sheet layer, by face.
pub(super) fn duplicates(removed: &Removed) -> Vec<Duplicate> {
    let same = removed.duplicates.iter().map(|d| Duplicate {
        face: d.face,
        of: d.of,
        orientation: DuplicateOrientation::Same,
    });
    let opposite = removed.sheets.iter().map(|&[of, face]| Duplicate {
        face,
        of,
        orientation: DuplicateOrientation::Opposite,
    });
    let mut all: Vec<Duplicate> = same.chain(opposite).collect();
    all.sort_by_key(|d| d.face);
    all
}
