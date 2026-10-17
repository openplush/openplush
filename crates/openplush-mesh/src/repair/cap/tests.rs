//! Cap stitching, checked against area, crack and sliver oracles.

use openplush_geom::epsilon::ModelTolerances;
use openplush_geom::{DVec2, DVec3};

use super::stitch;
use crate::halfedge::is_sliver;
use crate::{FaceId, SourceCorner, TriangleSoup};

/// a, b, m halfway between them, x above and y below the line ab, a point
/// off the plane, a quarter of the way from a to b, and halfway from b to x.
fn points() -> Vec<DVec3> {
    vec![
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(2.0, 0.0, 0.0),
        DVec3::new(1.0, 0.0, 0.0),
        DVec3::new(1.0, 1.0, 0.0),
        DVec3::new(1.0, -1.0, 0.0),
        DVec3::new(1.0, 1.0, 1.0),
        DVec3::new(0.5, 0.0, 0.0),
        DVec3::new(1.5, 0.5, 0.0),
    ]
}

/// Corner `i` has source vertex `i` and UV (i, 0.5), or no UV when `uv` is
/// false.
fn soup_with(triangles: &[[u32; 3]], uv: bool) -> TriangleSoup {
    let n = triangles.len() * 3;
    let corners = (0..n)
        .map(|i| SourceCorner {
            primitive: 0,
            vertex: i as u32,
        })
        .collect();
    let uvs = uv.then(|| (0..n).map(|i| Some(DVec2::new(i as f64, 0.5))).collect());
    TriangleSoup::new(points(), triangles.to_vec(), corners, uvs).unwrap()
}

fn soup(triangles: &[[u32; 3]]) -> TriangleSoup {
    soup_with(triangles, true)
}

fn tol(s: &TriangleSoup) -> ModelTolerances {
    ModelTolerances::for_diagonal(s.bbox_diagonal().unwrap()).unwrap()
}

fn f(i: u32) -> FaceId {
    FaceId::new(i)
}

fn src(vertex: u32) -> SourceCorner {
    SourceCorner {
        primitive: 0,
        vertex,
    }
}

fn uv(u: f64) -> Option<DVec2> {
    Some(DVec2::new(u, 0.5))
}

fn area(s: &TriangleSoup) -> f64 {
    s.triangle_indices()
        .iter()
        .map(|t| {
            let [a, b, c] = t.map(|v| s.positions()[v as usize]);
            0.5 * (b - a).cross(c - a).length()
        })
        .sum()
}

/// The checks every stitched fixture here passes: area kept, no zero-area
/// face left, and every edge on the line ab paired with its reverse.
fn assert_stitched(before: &TriangleSoup, after: &TriangleSoup) {
    let t = tol(before);
    assert!((area(after) - area(before)).abs() < 1e-12);
    let p = |v: u32| after.positions()[v as usize];
    for &[a, b, c] in after.triangle_indices() {
        assert!(!is_sliver(p(a), p(b), p(c), &t), "{a} {b} {c}");
    }
    let directed: Vec<[u32; 2]> = after
        .triangle_indices()
        .iter()
        .flat_map(|&[a, b, c]| [[a, b], [b, c], [c, a]])
        .collect();
    let below: Vec<[u32; 2]> = directed
        .iter()
        .copied()
        .filter(|&[a, b]| p(a).y == 0.0 && p(b).y == 0.0)
        .collect();
    let rim = |[a, b]: [u32; 2]| [a, b] == [1, 0] || [a, b] == [0, 1];
    for [a, b] in below.into_iter().filter(|&e| !rim(e)) {
        assert!(directed.contains(&[b, a]), "edge {a}-{b} is unpaired");
    }
}

/// x's face above ab, the cap (b, a, m) below it, and the two faces under the
/// cap that already meet at m.
const STRIP: [[u32; 3]; 4] = [[0, 1, 3], [1, 0, 2], [0, 4, 2], [2, 4, 1]];

#[test]
fn re_triangulates_the_face_across_a_caps_longest_edge_and_drops_the_cap() {
    let s = soup(&STRIP);
    let (out, stitched) = stitch(&s, &tol(&s));
    assert_eq!(
        out.triangle_indices(),
        [[3, 0, 2], [2, 1, 3], [0, 4, 2], [2, 4, 1]]
    );
    assert_eq!(stitched.caps, [f(1)]);
    assert_eq!(stitched.retriangulated, [f(0)]);
    assert_eq!(stitched.origin, [f(0), f(0), f(2), f(3)]);
    assert_eq!(out.positions(), s.positions());
    assert_stitched(&s, &out);
}

#[test]
fn a_new_corner_takes_the_caps_record_and_the_uv_along_the_faces_edge() {
    let s = soup(&STRIP);
    let (out, _) = stitch(&s, &tol(&s));
    // Face 0's corners are 0, 1, 2 (vertices a, b, x); the cap's corner at m is
    // corner 5.
    assert_eq!(
        out.corner_sources()[..6],
        [src(2), src(0), src(5), src(5), src(1), src(2)]
    );
    assert_eq!(
        out.corner_uvs().unwrap()[..6],
        [uv(2.0), uv(0.0), uv(0.5), uv(0.5), uv(1.0), uv(2.0)]
    );
    assert_eq!(out.corner_sources()[6..], s.corner_sources()[6..]);
}

#[test]
fn interpolates_the_uv_at_the_vertexs_place_along_the_edge() {
    let s = soup(&[[0, 1, 3], [1, 0, 6]]);
    let (out, _) = stitch(&s, &tol(&s));
    assert_eq!(out.triangle_indices(), [[3, 0, 6], [6, 1, 3]]);
    assert_eq!(out.corner_uvs().unwrap()[2], uv(0.25));
    assert_eq!(out.corner_uvs().unwrap()[3], uv(0.25));
}

#[test]
fn a_new_corner_has_no_uv_if_either_end_of_its_edge_has_none() {
    let mut uvs: Vec<_> = (0..6).map(|i| uv(i as f64)).collect();
    uvs[1] = None;
    let corners = (0..6).map(src).collect();
    let s = TriangleSoup::new(points(), vec![[0, 1, 3], [1, 0, 2]], corners, Some(uvs)).unwrap();
    let (out, _) = stitch(&s, &tol(&s));
    assert_eq!(out.corner_uvs().unwrap()[2], None);
    assert_eq!(out.corner_uvs().unwrap()[3], None);
}

#[test]
fn a_soup_without_uvs_stays_without() {
    let s = soup_with(&[[0, 1, 3], [1, 0, 2]], false);
    let (out, _) = stitch(&s, &tol(&s));
    assert_eq!(out.triangle_indices(), [[3, 0, 2], [2, 1, 3]]);
    assert_eq!(out.corner_uvs(), None);
}

#[test]
fn finds_the_longest_edge_at_any_corner_and_in_either_direction() {
    let s = soup(&[[0, 1, 3], [0, 2, 1], [0, 4, 2], [2, 4, 1]]);
    let (out, _) = stitch(&s, &tol(&s));
    assert_eq!(out.triangle_indices()[..2], [[3, 0, 2], [2, 1, 3]]);
    let s = soup(&[[1, 0, 2], [1, 0, 3]]);
    let (out, stitched) = stitch(&s, &tol(&s));
    assert_eq!(out.triangle_indices(), [[3, 1, 2], [2, 0, 3]]);
    assert_eq!(stitched.retriangulated, [f(1)]);
}

#[test]
fn a_cap_with_no_face_across_its_longest_edge_is_just_dropped() {
    let s = soup(&[[1, 0, 2], [0, 4, 2], [2, 4, 1]]);
    let (out, stitched) = stitch(&s, &tol(&s));
    assert_eq!(out.triangle_indices(), [[0, 4, 2], [2, 4, 1]]);
    assert_eq!(stitched.caps, [f(0)]);
    assert!(stitched.retriangulated.is_empty());
    assert_eq!(stitched.origin, [f(1), f(2)]);
}

#[test]
fn both_layers_of_a_cap_sheet_are_dropped() {
    let s = soup(&[[1, 0, 2], [0, 1, 2]]);
    let (out, stitched) = stitch(&s, &tol(&s));
    assert_eq!(out.triangle_count(), 0);
    assert_eq!(stitched.caps, [f(0), f(1)]);
}

#[test]
fn re_triangulates_every_face_of_a_fan_on_the_longest_edge() {
    let s = soup(&[[0, 1, 3], [1, 0, 2], [0, 1, 5]]);
    let (out, stitched) = stitch(&s, &tol(&s));
    assert_eq!(
        out.triangle_indices(),
        [[3, 0, 2], [2, 1, 3], [5, 0, 2], [2, 1, 5]]
    );
    assert_eq!(stitched.retriangulated, [f(0), f(2)]);
    assert_eq!(stitched.origin, [f(0), f(0), f(2), f(2)]);
}

#[test]
fn follows_a_cap_whose_longest_edge_is_part_of_another_caps() {
    // The cap (m, a, q) lies on the half a-m of the cap (b, a, m).
    let s = soup(&[
        [0, 1, 3],
        [1, 0, 2],
        [2, 0, 6],
        [0, 4, 6],
        [6, 4, 2],
        [2, 4, 1],
    ]);
    let (out, stitched) = stitch(&s, &tol(&s));
    assert_eq!(
        out.triangle_indices()[..3],
        [[3, 0, 6], [3, 6, 2], [2, 1, 3]]
    );
    assert_eq!(stitched.caps, [f(1), f(2)]);
    assert_stitched(&s, &out);
}

#[test]
fn never_leaves_three_vertices_of_one_edge_in_a_triangle() {
    // Vertices on two edges of face (a, b, x): m on a-b, and the cap (x, b, w)
    // puts w on b-x.
    let s = soup(&[[0, 1, 3], [1, 0, 2], [3, 1, 7]]);
    let (out, stitched) = stitch(&s, &tol(&s));
    assert_eq!(out.triangle_indices(), [[3, 0, 2], [2, 1, 7], [2, 7, 3]]);
    assert_eq!(stitched.caps, [f(1), f(2)]);
    assert_eq!(stitched.retriangulated, [f(0)]);
}

#[test]
fn puts_two_caps_on_one_edge_into_every_face_across_it_in_order() {
    let s = soup(&[[1, 0, 2], [1, 0, 6], [0, 1, 3], [0, 1, 5]]);
    let (out, stitched) = stitch(&s, &tol(&s));
    assert_eq!(
        out.triangle_indices(),
        [
            [3, 0, 6],
            [3, 6, 2],
            [2, 1, 3],
            [5, 0, 6],
            [5, 6, 2],
            [2, 1, 5]
        ]
    );
    assert_eq!(stitched.caps, [f(0), f(1)]);
    assert_eq!(stitched.retriangulated, [f(2), f(3)]);
}

#[test]
fn leaves_faces_that_repeat_a_vertex_and_a_soup_with_no_cap_alone() {
    let s = soup(&[[0, 1, 3], [0, 0, 1], [1, 0, 0]]);
    let (out, stitched) = stitch(&s, &tol(&s));
    assert_eq!(out, s);
    assert!(stitched.caps.is_empty() && stitched.retriangulated.is_empty());
    assert_eq!(stitched.origin, [f(0), f(1), f(2)]);
}

#[test]
fn two_caps_meeting_at_one_place_along_an_edge_leave_no_crack() {
    // Two caps on a-b whose middle vertices are at the same place along it, one
    // each side; faces above and below a-b.
    let positions = vec![
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(6.7e-6, 0.0, 0.0),
        DVec3::new(1.0, 1.0, 0.0),
        DVec3::new(1.0, -1.0, 0.0),
        DVec3::new(3.35e-6, 1.4e-6, 0.0),
        DVec3::new(3.35e-6, -1.4e-6, 0.0),
    ];
    let triangles = vec![[0, 1, 2], [1, 0, 3], [1, 0, 4], [1, 0, 5]];
    let corners = (0..12).map(src).collect();
    let s = TriangleSoup::new(positions, triangles, corners, None).unwrap();
    let (out, stitched) = stitch(&s, &tol(&s));
    assert_eq!(stitched.caps, [f(2), f(3)]);
    let directed: Vec<[u32; 2]> = out
        .triangle_indices()
        .iter()
        .flat_map(|&[a, b, c]| [[a, b], [b, c], [c, a]])
        .collect();
    for &[a, b] in directed.iter().filter(|e| e.iter().any(|&v| v >= 4)) {
        assert!(directed.contains(&[b, a]), "edge {a}-{b} is unpaired");
    }
}
