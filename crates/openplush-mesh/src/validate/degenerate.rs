use std::collections::BTreeMap;

use openplush_geom::epsilon::ModelTolerances;

use super::report::{Duplicate, DuplicateOrientation};
use crate::halfedge::is_sliver;
use crate::{FaceId, TriangleSoup};

/// The degenerate faces, and the faces topology is judged on: every face but those
/// that repeat a vertex and the later copies of a duplicate.
pub(super) struct Degenerate {
    pub(super) zero_area: Vec<FaceId>,
    pub(super) repeated_index: Vec<FaceId>,
    pub(super) duplicates: Vec<Duplicate>,
    pub(super) kept: Vec<u32>,
}

/// `welded` holds each face's vertices after welding, in face order. Areas use the
/// soup's own positions, so a face is zero-area exactly when repair would drop it.
// INVARIANT: `welded` has one entry per soup face, and soup indices are in range.
pub(super) fn degenerate(
    soup: &TriangleSoup,
    welded: &[[u32; 3]],
    tol: &ModelTolerances,
) -> Degenerate {
    let positions = soup.positions();
    let mut out = Degenerate {
        zero_area: Vec::new(),
        repeated_index: Vec::new(),
        duplicates: Vec::new(),
        kept: Vec::new(),
    };
    let mut seen: BTreeMap<[u32; 3], u32> = BTreeMap::new();
    for (i, (&[a, b, c], tri)) in soup.triangle_indices().iter().zip(welded).enumerate() {
        // Lossless: the face count fits in u32 (`TriangleSoup::new`).
        let face = i as u32;
        let [pa, pb, pc] = [a, b, c].map(|v| positions[v as usize]);
        if is_sliver(pa, pb, pc, tol) {
            out.zero_area.push(FaceId::new(face));
        }
        let [x, y, z] = *tri;
        if x == y || y == z || z == x {
            out.repeated_index.push(FaceId::new(face));
            continue;
        }
        let mut key = *tri;
        key.sort();
        match seen.get(&key) {
            Some(&first) => out.duplicates.push(Duplicate {
                face: FaceId::new(face),
                of: FaceId::new(first),
                orientation: orientation(welded[first as usize], *tri),
            }),
            None => {
                seen.insert(key, face);
                out.kept.push(face);
            }
        }
    }
    out
}

/// Same when `b` is a rotation of `a`, so both turn the same way.
fn orientation(a: [u32; 3], b: [u32; 3]) -> DuplicateOrientation {
    let rotations = [a, [a[1], a[2], a[0]], [a[2], a[0], a[1]]];
    if rotations.contains(&b) {
        DuplicateOrientation::Same
    } else {
        DuplicateOrientation::Opposite
    }
}
