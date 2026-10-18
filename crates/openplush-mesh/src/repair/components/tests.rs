//! Component splitting, checked against hand-computed areas.

use openplush_geom::DVec3;

use super::{Component, split};
use crate::{FaceId, SourceCorner, TriangleSoup};

fn f(i: u32) -> FaceId {
    FaceId::new(i)
}

fn soup(positions: Vec<DVec3>, triangles: &[[u32; 3]]) -> TriangleSoup {
    let corners = (0..triangles.len() * 3)
        .map(|i| SourceCorner {
            primitive: 0,
            vertex: i as u32,
        })
        .collect();
    TriangleSoup::new(positions, triangles.to_vec(), corners, None).unwrap()
}

/// Right triangles in the z = 0 plane with the given legs, each on its own three vertices.
fn right_triangles(legs: &[(f64, f64)]) -> TriangleSoup {
    let mut positions = Vec::new();
    for (i, &(b, h)) in legs.iter().enumerate() {
        let x = 100.0 * i as f64;
        positions.extend([
            DVec3::new(x, 0.0, 0.0),
            DVec3::new(x + b, 0.0, 0.0),
            DVec3::new(x, h, 0.0),
        ]);
    }
    let triangles: Vec<[u32; 3]> = (0..legs.len() as u32)
        .map(|i| [3 * i, 3 * i + 1, 3 * i + 2])
        .collect();
    soup(positions, &triangles)
}

fn flags(components: &[Component]) -> Vec<bool> {
    components.iter().map(|c| c.detail_candidate).collect()
}

#[test]
fn a_soup_with_no_faces_has_no_components() {
    let s = TriangleSoup::new(
        vec![DVec3::new(0.0, 0.0, 0.0)],
        Vec::new(),
        Vec::new(),
        None,
    )
    .unwrap();
    assert!(split(&s).is_empty());
}

#[test]
fn one_component_is_never_a_detail_part() {
    let s = right_triangles(&[(2.0, 2.0)]);
    assert_eq!(
        split(&s),
        [Component {
            faces: vec![f(0)],
            detail_candidate: false
        }]
    );
}

#[test]
fn a_component_under_one_percent_of_the_area_is_a_candidate() {
    let s = right_triangles(&[(20.0, 100.0), (1.0, 1.0)]);
    let parts = split(&s);
    assert_eq!(flags(&parts), [false, true]);
}

#[test]
fn a_component_at_exactly_one_percent_is_not() {
    let s = right_triangles(&[(11.0, 18.0), (1.0, 2.0)]);
    assert_eq!(flags(&split(&s)), [false, false]);
}

#[test]
fn a_component_just_under_one_percent_is_a_candidate() {
    let s = right_triangles(&[(11.0, 18.0), (0.99, 2.0)]);
    assert_eq!(flags(&split(&s)), [false, true]);
}

#[test]
fn components_are_in_order_of_their_first_face_with_faces_ascending() {
    // Faces 0 and 2 share vertex 1; face 1 stands alone.
    let positions = (0..8)
        .map(|i| DVec3::new(f64::from(i), f64::from(i * i), 0.0))
        .collect();
    let s = soup(positions, &[[0, 1, 2], [3, 4, 5], [1, 6, 7]]);
    let faces: Vec<Vec<FaceId>> = split(&s).into_iter().map(|c| c.faces).collect();
    assert_eq!(faces, [vec![f(0), f(2)], vec![f(1)]]);
}

#[test]
fn every_part_is_a_candidate_when_each_is_under_one_percent() {
    let s = right_triangles(&vec![(2.0, 1.0); 150]);
    let parts = flags(&split(&s));
    assert_eq!(parts.len(), 150);
    assert!(parts.into_iter().all(|c| c));
}

#[test]
fn a_components_area_is_the_sum_over_all_its_faces() {
    // A: two faces of area 0.6 sharing vertex 2 (1.2 in all, 1.19% of 101.2);
    // its first face alone would be 0.6% of the total and flagged.
    let positions = vec![
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(1.2, 0.0, 0.0),
        DVec3::new(0.0, 1.0, 0.0),
        DVec3::new(0.0, 2.0, 0.0),
        DVec3::new(1.2, 2.0, 0.0),
        DVec3::new(50.0, 0.0, 0.0),
        DVec3::new(70.0, 0.0, 0.0),
        DVec3::new(50.0, 10.0, 0.0),
    ];
    let s = soup(positions, &[[0, 1, 2], [2, 3, 4], [5, 6, 7]]);
    assert_eq!(flags(&split(&s)), [false, false]);
}

#[test]
fn area_is_the_true_area_of_a_skewed_triangle_in_any_plane() {
    // Big: area 100 in the xz plane. Small: a sliver of area 0.5 in the xy plane whose
    // edge lengths multiply to about 100, so only the true area flags it.
    let positions = vec![
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(20.0, 0.0, 0.0),
        DVec3::new(0.0, 0.0, 10.0),
        DVec3::new(50.0, 0.0, 0.0),
        DVec3::new(60.0, 0.0, 0.0),
        DVec3::new(60.0, 0.1, 0.0),
    ];
    let s = soup(positions, &[[0, 1, 2], [3, 4, 5]]);
    assert_eq!(flags(&split(&s)), [false, true]);
}

#[test]
fn both_layers_of_a_sheet_count_towards_its_area() {
    // Sheet: two coincident opposite-wound triangles, area 1 each (2 in all, 1.96% of
    // 102): not a candidate. One layer alone (0.99%) would be.
    let positions = vec![
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(1.0, 0.0, 0.0),
        DVec3::new(0.0, 2.0, 0.0),
        DVec3::new(50.0, 0.0, 0.0),
        DVec3::new(70.0, 0.0, 0.0),
        DVec3::new(50.0, 10.0, 0.0),
    ];
    let s = soup(positions, &[[0, 1, 2], [0, 2, 1], [3, 4, 5]]);
    assert_eq!(flags(&split(&s)), [false, false]);
}
