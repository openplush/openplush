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
