//! `into_manifold` end to end: the mesh it builds and what its report says.

use openplush_geom::DVec3;

use super::into_manifold;
use crate::{
    FaceId, ManifoldMesh, MeshError, RepairPlan, RepairReport, SourceCorner, TriangleSoup, VertexId,
};

fn f(i: u32) -> FaceId {
    FaceId::new(i)
}

fn v(i: u32) -> VertexId {
    VertexId::new(i)
}

fn p(x: f64, y: f64, z: f64) -> DVec3 {
    DVec3::new(x, y, z)
}

/// Corner `i` has source vertex `i`.
fn soup(positions: Vec<DVec3>, triangles: &[[u32; 3]]) -> TriangleSoup {
    let corners = (0..triangles.len() * 3)
        .map(|i| SourceCorner {
            primitive: 0,
            vertex: i as u32,
        })
        .collect();
    TriangleSoup::new(positions, triangles.to_vec(), corners, None).unwrap()
}

fn repair(s: &TriangleSoup) -> Result<(ManifoldMesh, RepairReport), MeshError> {
    into_manifold(s, &RepairPlan::default())
}

fn tetra_positions() -> Vec<DVec3> {
    vec![p(0., 0., 0.), p(1., 0., 0.), p(0., 1., 0.), p(0., 0., 1.)]
}

/// Outward-wound.
const TETRA: [[u32; 3]; 4] = [[0, 2, 1], [0, 1, 3], [1, 2, 3], [0, 3, 2]];

fn nothing_to_report(r: &RepairReport) -> bool {
    r.welded().is_empty()
        && r.repeated_index_faces().is_empty()
        && r.duplicate_faces().is_empty()
        && r.sheets().is_empty()
        && r.stitched_caps().is_empty()
        && r.retriangulated_faces().is_empty()
        && r.paired_edges().is_empty()
        && r.vertex_copies().is_empty()
        && r.flipped_faces().is_empty()
        && r.orientation_conflicts().is_empty()
        && r.holes().is_empty()
        && r.mandatory_seams().is_empty()
}

#[test]
fn a_clean_closed_surface_builds_and_the_report_has_nothing_but_its_component() {
    let (mesh, report) = repair(&soup(tetra_positions(), &TETRA)).unwrap();
    assert_eq!(mesh.face_count(), 4);
    assert_eq!(mesh.vertex_count(), 4);
    assert!(nothing_to_report(&report));
    assert_eq!(report.components().len(), 1);
    assert!(!report.components()[0].detail_candidate);
}

#[test]
fn an_empty_soup_and_a_pointlike_one_are_refused() {
    let none = TriangleSoup::new(vec![p(0., 0., 0.)], Vec::new(), Vec::new(), None).unwrap();
    assert_eq!(repair(&none).unwrap_err(), MeshError::NoTriangles);
    let point = soup(vec![p(1., 1., 1.); 3], &[[0, 1, 2]]);
    assert_eq!(repair(&point).unwrap_err(), MeshError::UnusableExtent);
}

#[test]
fn coincident_vertices_are_welded_and_listed() {
    // Each tetra face on its own three vertices.
    let base = tetra_positions();
    let mut positions = Vec::new();
    let mut triangles = Vec::new();
    for (i, tri) in TETRA.iter().enumerate() {
        positions.extend(tri.map(|k| base[k as usize]));
        triangles.push([3 * i as u32, 3 * i as u32 + 1, 3 * i as u32 + 2]);
    }
    let (mesh, report) = repair(&soup(positions, &triangles)).unwrap();
    assert_eq!(mesh.vertex_count(), 4);
    assert_eq!(report.welded().len(), 4);
    assert!(report.welded().iter().all(|g| g.len() == 3));
}

#[test]
fn an_inside_out_shell_is_turned_over_and_every_flip_is_listed() {
    let reversed: Vec<[u32; 3]> = TETRA.iter().map(|&[a, b, c]| [a, c, b]).collect();
    let (_, report) = repair(&soup(tetra_positions(), &reversed)).unwrap();
    assert_eq!(report.flipped_faces(), [f(0), f(1), f(2), f(3)]);
}

#[test]
fn a_dropped_face_is_listed_by_its_input_id_and_the_rest_keep_their_order() {
    // Face 1 repeats a vertex.
    let mut triangles = TETRA.to_vec();
    triangles.insert(1, [1, 1, 2]);
    let (mesh, report) = repair(&soup(tetra_positions(), &triangles)).unwrap();
    assert_eq!(report.repeated_index_faces(), [f(1)]);
    assert_eq!(mesh.face_count(), 4);
}

#[test]
fn a_repeated_index_face_reaches_the_build_when_the_plan_keeps_degenerates() {
    let mut triangles = TETRA.to_vec();
    triangles.insert(1, [1, 1, 2]);
    let plan = RepairPlan {
        drop_degenerates: false,
        ..RepairPlan::default()
    };
    assert_eq!(
        into_manifold(&soup(tetra_positions(), &triangles), &plan).unwrap_err(),
        MeshError::DegenerateTriangle { triangle: f(1) }
    );
}

#[test]
fn an_open_surface_builds_and_its_loop_is_reported_unfilled() {
    let s = soup(
        vec![p(0., 0., 0.), p(1., 0., 0.), p(1., 1., 0.), p(0., 1., 0.)],
        &[[0, 1, 2], [0, 2, 3]],
    );
    let (mesh, report) = repair(&s).unwrap();
    assert_eq!(mesh.boundary_loops().len(), 1);
    assert_eq!(report.holes().len(), 1);
    assert!(!report.holes()[0].filled());
    assert_eq!(
        report.holes()[0].loop_ref().corner(),
        SourceCorner {
            primitive: 0,
            vertex: 0
        }
    );
}

#[test]
fn a_double_sided_sheet_builds_with_its_pairings_and_sheets_listed() {
    let s = soup(
        vec![p(0., 0., 0.), p(1., 0., 0.), p(1., 1., 0.), p(0., 1., 0.)],
        &[[0, 1, 2], [0, 2, 3], [0, 3, 2], [2, 1, 0]],
    );
    let (mesh, report) = repair(&s).unwrap();
    assert_eq!(mesh.edge_count(), 6);
    assert!(mesh.boundary_loops().is_empty());
    assert_eq!(report.sheets(), [[f(0), f(3)], [f(1), f(2)]]);
    assert_eq!(report.paired_edges().len(), 1);
    assert!(report.mandatory_seams().is_empty());
}

#[test]
fn a_sheet_with_an_interior_vertex_gets_a_vertex_copy_for_its_second_layer() {
    let s = soup(
        vec![p(0., 0., 0.), p(1., 0., 0.), p(0., 1., 0.), p(-1., -1., 0.)],
        &[
            [0, 1, 2],
            [0, 2, 3],
            [0, 3, 1],
            [0, 2, 1],
            [0, 3, 2],
            [0, 1, 3],
        ],
    );
    let (mesh, report) = repair(&s).unwrap();
    assert_eq!(mesh.vertex_count(), 5);
    assert_eq!(report.vertex_copies().len(), 1);
    assert_eq!(report.vertex_copies()[0].of, v(0));
    assert_eq!(report.vertex_copies()[0].copy, v(4));
    assert!(mesh.boundary_loops().is_empty());
}

#[test]
fn a_fin_on_a_body_becomes_boundary_edges_that_are_mandatory_seams() {
    // The tetrahedron with a third face on its edge 0-1 (a fin), which already has two.
    let mut positions = tetra_positions();
    positions.push(p(0.5, -1., 0.));
    let mut triangles = TETRA.to_vec();
    triangles.push([0, 1, 4]);
    let (mesh, report) = repair(&soup(positions, &triangles)).unwrap();
    assert_eq!(report.paired_edges().len(), 1);
    assert!(!report.mandatory_seams().is_empty());
    assert!(!mesh.boundary_loops().is_empty());
}

#[test]
fn a_twisted_ring_reports_its_conflict_edge_as_a_mandatory_seam() {
    let radii = [3.0, 1.0, 2.0, 1.0, 1.0];
    let positions = radii
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let a = std::f64::consts::TAU * i as f64 / 5.0;
            p(r * a.cos(), r * a.sin(), 0.0)
        })
        .collect();
    let s = soup(
        positions,
        &[[0, 1, 2], [1, 2, 3], [2, 3, 4], [3, 4, 0], [4, 0, 1]],
    );
    let (_, report) = repair(&s).unwrap();
    assert_eq!(report.orientation_conflicts(), [[v(2), v(3)]]);
    assert!(report.mandatory_seams().contains(&[v(2), v(3)]));
}

#[test]
fn a_tiny_separate_piece_is_a_detail_candidate() {
    let mut positions = tetra_positions()
        .iter()
        .map(|&q| q * 100.)
        .collect::<Vec<_>>();
    let small = tetra_positions();
    positions.extend(small.iter().map(|&q| q * 0.1 + p(300., 0., 0.)));
    let mut triangles = TETRA.to_vec();
    triangles.extend(TETRA.iter().map(|t| t.map(|i| i + 4)));
    let (_, report) = repair(&soup(positions, &triangles)).unwrap();
    let flags: Vec<bool> = report
        .components()
        .iter()
        .map(|c| c.detail_candidate)
        .collect();
    assert_eq!(flags, [false, true]);
    assert_eq!(report.components()[1].faces, [f(4), f(5), f(6), f(7)]);
}

#[test]
fn a_plan_that_fills_a_loop_is_refused_until_filling_exists_and_a_missing_loop_is_named() {
    let s = soup(
        vec![p(0., 0., 0.), p(1., 0., 0.), p(1., 1., 0.), p(0., 1., 0.)],
        &[[0, 1, 2], [0, 2, 3]],
    );
    let named = |vertex| {
        let mut plan = RepairPlan::default();
        plan.fill_holes.insert(crate::LoopRef::new(SourceCorner {
            primitive: 0,
            vertex,
        }));
        plan
    };
    assert!(matches!(
        into_manifold(&s, &named(0)).unwrap_err(),
        MeshError::HoleFillUnavailable { .. }
    ));
    assert!(matches!(
        into_manifold(&s, &named(9)).unwrap_err(),
        MeshError::UnknownHoleLoop { .. }
    ));
}
