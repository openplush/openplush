use std::collections::BTreeMap;

use crate::{FaceId, TriangleSoup};

/// What the degenerate step removed or flagged, by the IDs of the soup it was given.
#[derive(Debug, Default)]
pub(crate) struct Removed {
    pub(crate) repeated_index: Vec<FaceId>,
    pub(crate) duplicates: Vec<DroppedCopy>,
    /// Each pair of opposite-orientation copies kept as the two layers of a sheet,
    /// the lower face ID first, sorted by it.
    pub(crate) sheets: Vec<[FaceId; 2]>,
    /// The input face each face of the returned soup came from, in its order.
    pub(crate) origin: Vec<FaceId>,
}

/// A face dropped because it repeats the earlier face `of` with the same winding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DroppedCopy {
    /// The face dropped.
    pub face: FaceId,
    /// The earlier face it repeats.
    pub of: FaceId,
}

/// Drops faces that repeat a vertex and every copy of an earlier face wound the
/// same way, keeping the lowest face ID. Two copies wound opposite ways are both
/// kept, as a sheet.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "called by into_manifold, not yet written")
)]
pub(crate) fn remove(soup: &TriangleSoup) -> (TriangleSoup, Removed) {
    let removed = classify(soup.triangle_indices());
    let kept: Vec<u32> = removed.origin.iter().map(|f| f.index()).collect();
    (soup.retaining(&kept), removed)
}

/// What [`remove`] drops and keeps from these faces, without building the soup.
pub(crate) fn classify(triangles: &[[u32; 3]]) -> Removed {
    let mut removed = Removed::default();
    // Rotated to start at the smallest vertex, so copies wound the same way share
    // a key and copies wound opposite ways do not.
    let mut first: BTreeMap<[u32; 3], u32> = BTreeMap::new();
    for (i, &tri) in triangles.iter().enumerate() {
        // Lossless: the face count fits in u32 (`TriangleSoup::new`).
        let face = i as u32;
        let [a, b, c] = tri;
        if a == b || b == c || c == a {
            removed.repeated_index.push(FaceId::new(face));
            continue;
        }
        let key = rotated(tri);
        if let Some(&of) = first.get(&key) {
            removed.duplicates.push(DroppedCopy {
                face: FaceId::new(face),
                of: FaceId::new(of),
            });
            continue;
        }
        if let Some(&layer) = first.get(&rotated([a, c, b])) {
            removed.sheets.push([FaceId::new(layer), FaceId::new(face)]);
        }
        first.insert(key, face);
        removed.origin.push(FaceId::new(face));
    }
    removed.sheets.sort_by_key(|&[layer, _]| layer);
    removed
}

fn rotated([a, b, c]: [u32; 3]) -> [u32; 3] {
    if a < b && a < c {
        [a, b, c]
    } else if b < c {
        [b, c, a]
    } else {
        [c, a, b]
    }
}

#[cfg(test)]
mod tests {
    use openplush_geom::{DVec2, DVec3};

    use super::{DroppedCopy, remove};
    use crate::{FaceId, SourceCorner, TriangleSoup};

    fn soup(triangles: Vec<[u32; 3]>) -> TriangleSoup {
        let positions = vec![
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(1.0, 0.0, 0.0),
            DVec3::new(1.0, 1.0, 0.0),
            DVec3::new(0.0, 1.0, 0.0),
        ];
        let corners = (0..triangles.len() * 3)
            .map(|i| SourceCorner {
                primitive: 0,
                vertex: i as u32,
            })
            .collect();
        let uvs = (0..triangles.len() * 3)
            .map(|i| Some(DVec2::new(i as f64, 0.5)))
            .collect();
        TriangleSoup::new(positions, triangles, corners, Some(uvs)).unwrap()
    }

    fn f(i: u32) -> FaceId {
        FaceId::new(i)
    }

    /// The corner records of `faces` of `s`, in that order.
    fn corners_of(s: &TriangleSoup, faces: &[u32]) -> (Vec<SourceCorner>, Vec<Option<DVec2>>) {
        let slots = || {
            faces
                .iter()
                .flat_map(|&t| (0..3).map(move |c| (t * 3) as usize + c))
        };
        let src = slots().map(|i| s.corner_sources()[i]).collect();
        let uv = slots().map(|i| s.corner_uvs().unwrap()[i]).collect();
        (src, uv)
    }

    #[test]
    fn drops_a_face_that_repeats_a_vertex_and_keeps_the_rest_with_their_corners() {
        let s = soup(vec![[0, 1, 2], [1, 1, 3], [0, 2, 3], [3, 0, 3], [2, 3, 3]]);
        let (kept, removed) = remove(&s);
        assert_eq!(kept.triangle_indices(), [[0, 1, 2], [0, 2, 3]]);
        let (src, uv) = corners_of(&s, &[0, 2]);
        assert_eq!(kept.corner_sources(), src);
        assert_eq!(kept.corner_uvs().unwrap(), uv);
        assert_eq!(kept.positions(), s.positions());
        assert_eq!(removed.repeated_index, [f(1), f(3), f(4)]);
        assert_eq!(removed.origin, [f(0), f(2)]);
        assert!(removed.duplicates.is_empty());
        assert!(removed.sheets.is_empty());
    }

    #[test]
    fn keeps_the_lowest_id_of_same_orientation_copies() {
        let s = soup(vec![[0, 1, 2], [1, 2, 0], [0, 2, 3], [0, 1, 2]]);
        let (kept, removed) = remove(&s);
        assert_eq!(kept.triangle_indices(), [[0, 1, 2], [0, 2, 3]]);
        assert_eq!(
            removed.duplicates,
            [
                DroppedCopy {
                    face: f(1),
                    of: f(0)
                },
                DroppedCopy {
                    face: f(3),
                    of: f(0)
                }
            ]
        );
        assert!(removed.sheets.is_empty());
    }

    #[test]
    fn keeps_both_layers_of_a_sheet_and_flags_them() {
        let s = soup(vec![[0, 1, 2], [2, 1, 0], [1, 0, 2], [1, 2, 0]]);
        let (kept, removed) = remove(&s);
        assert_eq!(kept.triangle_indices(), [[0, 1, 2], [2, 1, 0]]);
        assert_eq!(removed.sheets, [[f(0), f(1)]]);
        assert_eq!(removed.origin, [f(0), f(1)]);
        assert_eq!(
            removed.duplicates,
            [
                DroppedCopy {
                    face: f(2),
                    of: f(1)
                },
                DroppedCopy {
                    face: f(3),
                    of: f(0)
                }
            ]
        );
    }

    #[test]
    fn names_faces_by_input_id_and_maps_each_kept_face_to_its_origin() {
        let s = soup(vec![[0, 0, 1], [0, 1, 2], [3, 2, 1], [2, 1, 0]]);
        let (kept, removed) = remove(&s);
        assert_eq!(kept.triangle_indices(), [[0, 1, 2], [3, 2, 1], [2, 1, 0]]);
        assert_eq!(removed.sheets, [[f(1), f(3)]]);
        assert_eq!(removed.origin, [f(1), f(2), f(3)]);
    }

    #[test]
    fn keeps_every_face_of_a_two_layer_square() {
        let s = soup(vec![[0, 1, 2], [0, 2, 3], [0, 3, 2], [2, 1, 0]]);
        let (kept, removed) = remove(&s);
        assert_eq!(kept, s);
        assert_eq!(removed.sheets, [[f(0), f(3)], [f(1), f(2)]]);
        assert!(removed.duplicates.is_empty());
    }

    #[test]
    fn a_soup_with_nothing_to_remove_comes_back_unchanged() {
        let s = soup(vec![[0, 1, 2], [0, 2, 3]]);
        let (kept, removed) = remove(&s);
        assert_eq!(kept, s);
        assert!(removed.repeated_index.is_empty());
        assert!(removed.duplicates.is_empty());
        assert!(removed.sheets.is_empty());
    }
}
