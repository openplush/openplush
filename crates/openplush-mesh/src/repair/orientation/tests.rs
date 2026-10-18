//! Orientation, checked against `validate` as the oracle for agreement and volume.

use openplush_geom::epsilon::ModelTolerances;
use openplush_geom::{DVec2, DVec3};

use super::orient;
use crate::{FaceId, MeshError, SourceCorner, TriangleSoup, VertexId, validate};

fn f(i: u32) -> FaceId {
    FaceId::new(i)
}

fn v(i: u32) -> VertexId {
    VertexId::new(i)
}

fn p(x: f64, y: f64, z: f64) -> DVec3 {
    DVec3::new(x, y, z)
}

/// Corner `i` has source vertex `i` and UV (i, 0.5).
fn soup(positions: Vec<DVec3>, triangles: &[[u32; 3]]) -> TriangleSoup {
    let corners = (0..triangles.len() * 3)
        .map(|i| SourceCorner {
            primitive: 0,
            vertex: i as u32,
        })
        .collect();
    let uvs = (0..triangles.len() * 3)
        .map(|i| Some(DVec2::new(i as f64, 0.5)))
        .collect();
    TriangleSoup::new(positions, triangles.to_vec(), corners, Some(uvs)).unwrap()
}

fn tol(s: &TriangleSoup) -> ModelTolerances {
    ModelTolerances::for_diagonal(s.bbox_diagonal().unwrap()).unwrap()
}

fn oriented(s: &TriangleSoup) -> (TriangleSoup, super::Oriented) {
    orient(s, &tol(s)).unwrap()
}

fn reversed(triangles: &[[u32; 3]]) -> Vec<[u32; 3]> {
    triangles.iter().map(|&[a, b, c]| [a, c, b]).collect()
}

/// Faces 0, 1, 2 in a strip, all facing +z; the middle one is the largest.
fn strip() -> (Vec<DVec3>, Vec<[u32; 3]>) {
    let positions = vec![
        p(0.0, 0.0, 0.0),
        p(1.0, 0.0, 0.0),
        p(0.0, 1.0, 0.0),
        p(4.0, 4.0, 0.0),
        p(1.0, 2.5, 0.0),
    ];
    (positions, vec![[0, 1, 2], [1, 3, 2], [2, 3, 4]])
}

/// Four faces wound outward; face 3 is the largest.
fn tetrahedron() -> (Vec<DVec3>, Vec<[u32; 3]>) {
    let positions = vec![
        p(0.0, 0.0, 0.0),
        p(1.0, 0.0, 0.0),
        p(0.0, 1.0, 0.0),
        p(0.0, 0.0, 1.0),
    ];
    (positions, vec![[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]])
}

fn assert_agrees_and_encloses(s: &TriangleSoup) {
    let report = validate(s);
    assert!(report.winding_conflicts().is_empty());
    assert!(report.inside_out().is_empty());
}

#[test]
fn a_consistently_wound_surface_is_left_alone() {
    let (pos, tris) = strip();
    let s = soup(pos, &tris);
    let (out, done) = oriented(&s);
    assert_eq!(out, s);
    assert!(done.flipped.is_empty());
    assert!(done.conflicts.is_empty());
}

#[test]
fn faces_are_turned_to_agree_with_the_largest_face() {
    let (pos, tris) = strip();
    let s = soup(pos.clone(), &[[0, 2, 1], [1, 3, 2], [2, 4, 3]]);
    let (out, done) = oriented(&s);
    assert_eq!(out.triangle_indices(), tris);
    assert_eq!(done.flipped, [f(0), f(2)]);
    assert!(done.conflicts.is_empty());
    assert!(validate(&out).winding_conflicts().is_empty());
}

#[test]
fn a_surface_wound_consistently_but_inward_is_not_judged_by_volume() {
    let (pos, tris) = strip();
    let s = soup(pos, &reversed(&tris));
    let (out, done) = oriented(&s);
    assert_eq!(out, s);
    assert!(done.flipped.is_empty());
}

#[test]
fn a_flipped_face_keeps_each_vertex_with_its_corner_record() {
    let (pos, _) = strip();
    let s = soup(pos, &[[0, 2, 1], [1, 3, 2], [2, 4, 3]]);
    let (out, done) = oriented(&s);
    assert_eq!(done.flipped, [f(0), f(2)]);
    for face in 0..3 {
        let before = s.triangle(f(face)).unwrap();
        let after = out.triangle(f(face)).unwrap();
        for (slot, vertex) in after.into_iter().enumerate() {
            let was = before.iter().position(|&b| b == vertex).unwrap();
            assert_eq!(
                out.corner_source(f(face), slot),
                s.corner_source(f(face), was)
            );
            assert_eq!(out.corner_uv(f(face), slot), s.corner_uv(f(face), was));
        }
    }
}

#[test]
fn a_flipped_face_of_a_soup_without_uvs_still_swaps_its_provenance() {
    let (pos, _) = strip();
    let corners = (0..9)
        .map(|i| SourceCorner {
            primitive: 0,
            vertex: i,
        })
        .collect();
    let s = TriangleSoup::new(pos, vec![[0, 2, 1], [1, 3, 2], [2, 4, 3]], corners, None).unwrap();
    let (out, _) = oriented(&s);
    assert_eq!(out.corner_uv(f(0), 0), None);
    assert_eq!(out.corner_source(f(0), 1).unwrap().vertex, 2);
    assert_eq!(out.corner_source(f(0), 2).unwrap().vertex, 1);
}

#[test]
fn a_closed_shell_wound_outward_is_left_alone() {
    let (pos, tris) = tetrahedron();
    let s = soup(pos, &tris);
    let (out, done) = oriented(&s);
    assert_eq!(out, s);
    assert!(done.flipped.is_empty());
}

#[test]
fn a_closed_shell_wound_inward_is_turned_over() {
    let (pos, tris) = tetrahedron();
    let s = soup(pos, &reversed(&tris));
    let (out, done) = oriented(&s);
    assert_eq!(out.triangle_indices(), tris);
    assert_eq!(done.flipped, [f(0), f(1), f(2), f(3)]);
    assert_agrees_and_encloses(&out);
}

#[test]
fn a_seed_wound_against_a_closed_shell_nets_to_one_flip() {
    let (pos, mut tris) = tetrahedron();
    let want = tris.clone();
    tris[3] = [1, 3, 2];
    let (out, done) = oriented(&soup(pos, &tris));
    assert_eq!(out.triangle_indices(), want);
    assert_eq!(done.flipped, [f(3)]);
    assert_agrees_and_encloses(&out);
}

#[test]
fn each_component_is_judged_alone() {
    let (pos, tris) = tetrahedron();
    let mut positions = pos.clone();
    positions.extend(pos.iter().map(|&q| q + p(5.0, 0.0, 0.0)));
    let mut triangles = tris.clone();
    triangles.extend(reversed(&tris).into_iter().map(|t| t.map(|i| i + 4)));
    let (out, done) = oriented(&soup(positions, &triangles));
    assert_eq!(done.flipped, [f(4), f(5), f(6), f(7)]);
    assert_agrees_and_encloses(&out);
}

#[test]
fn a_shell_touching_another_only_at_a_vertex_is_judged_alone() {
    let (pos, tris) = tetrahedron();
    let mut positions = pos.clone();
    positions.extend(pos.iter().skip(1).map(|&q| q * -1.0));
    let mut triangles = tris.clone();
    triangles.extend(
        tris.iter()
            .map(|t| t.map(|i| if i == 0 { 0 } else { i + 3 })),
    );
    let (out, done) = oriented(&soup(positions, &triangles));
    assert_eq!(done.flipped, [f(4), f(5), f(6), f(7)]);
    assert_agrees_and_encloses(&out);
}

/// Five faces in a ring, each sharing an edge with the next, with a half twist. Vertex 0
/// is pulled out so face 4, which has it in the middle, is the largest.
fn mobius() -> TriangleSoup {
    let radii = [3.0, 1.0, 2.0, 1.0, 1.0];
    let positions = radii
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let angle = std::f64::consts::TAU * i as f64 / 5.0;
            p(r * angle.cos(), r * angle.sin(), 0.0)
        })
        .collect();
    soup(
        positions,
        &[[0, 1, 2], [1, 2, 3], [2, 3, 4], [3, 4, 0], [4, 0, 1]],
    )
}

#[test]
fn a_twisted_ring_reports_the_one_edge_where_the_walk_meets_itself() {
    let s = mobius();
    let (out, done) = oriented(&s);
    assert_eq!(done.flipped, [f(0), f(3)]);
    assert_eq!(done.conflicts, [[v(2), v(3)]]);
    assert_eq!(validate(&out).winding_conflicts(), done.conflicts);
}

/// The six-vertex projective plane: closed, every edge on two faces, not orientable.
fn projective_plane() -> TriangleSoup {
    let positions = vec![
        p(0.0, 0.0, 2.0),
        p(2.0, 0.0, 0.5),
        p(0.6, 1.9, 0.4),
        p(-1.6, 1.2, 0.7),
        p(-1.4, -1.5, 0.3),
        p(0.9, -1.8, 0.6),
    ];
    let tris = [
        [0, 1, 2],
        [0, 2, 3],
        [0, 3, 4],
        [0, 4, 5],
        [0, 5, 1],
        [1, 2, 4],
        [2, 3, 5],
        [3, 4, 1],
        [4, 5, 2],
        [5, 1, 3],
    ];
    soup(positions, &tris)
}

#[test]
fn a_closed_surface_that_cannot_be_oriented_is_not_judged_by_volume() {
    let s = projective_plane();
    let (out, done) = oriented(&s);
    assert!(!done.conflicts.is_empty());
    let mirror = soup(s.positions().to_vec(), &reversed(s.triangle_indices()));
    let (mirror_out, _) = oriented(&mirror);
    assert_eq!(
        mirror_out.triangle_indices(),
        reversed(out.triangle_indices())
    );
}

#[test]
fn the_larger_of_two_equal_faces_is_the_lower_numbered() {
    let s = soup(
        vec![
            p(0.0, 0.0, 0.0),
            p(1.0, 0.0, 0.0),
            p(0.0, 1.0, 0.0),
            p(1.0, 1.0, 0.0),
        ],
        &[[0, 1, 2], [1, 2, 3]],
    );
    let (_, done) = oriented(&s);
    assert_eq!(done.flipped, [f(1)]);
}

#[test]
fn the_layers_of_a_sheet_are_left_alone() {
    let (pos, _) = strip();
    let one = soup(pos.clone(), &[[0, 1, 2], [1, 0, 2]]);
    let two = soup(
        vec![
            p(0.0, 0.0, 0.0),
            p(1.0, 0.0, 0.0),
            p(1.0, 1.0, 0.0),
            p(0.0, 1.0, 0.0),
        ],
        &[[0, 1, 2], [0, 2, 3], [0, 3, 2], [2, 1, 0]],
    );
    for s in [one, two] {
        let (out, done) = oriented(&s);
        assert_eq!(out, s);
        assert!(done.flipped.is_empty());
        assert!(done.conflicts.is_empty());
    }
}

#[test]
fn faces_on_an_edge_with_more_than_two_are_not_walked() {
    let s = soup(
        vec![
            p(0.0, 0.0, 0.0),
            p(1.0, 0.0, 0.0),
            p(0.0, 1.0, 0.0),
            p(0.0, 0.0, 1.0),
            p(0.0, -1.0, 0.0),
        ],
        &[[0, 1, 2], [0, 1, 3], [0, 1, 4]],
    );
    let (out, done) = oriented(&s);
    assert_eq!(out, s);
    assert!(done.flipped.is_empty());
    assert!(done.conflicts.is_empty());
}

#[test]
fn a_soup_with_no_faces_is_returned_as_it_is() {
    let s = TriangleSoup::new(vec![p(0.0, 0.0, 0.0)], Vec::new(), Vec::new(), None).unwrap();
    let t = ModelTolerances::for_diagonal(1.0).unwrap();
    let (out, done) = orient(&s, &t).unwrap();
    assert_eq!(out, s);
    assert!(done.flipped.is_empty());
}

#[test]
fn a_face_that_repeats_a_vertex_is_refused() {
    let (pos, _) = strip();
    let s = soup(pos, &[[0, 1, 2], [1, 1, 2]]);
    let err = orient(&s, &tol(&s)).unwrap_err();
    assert_eq!(err, MeshError::DegenerateTriangle { triangle: f(1) });
}
