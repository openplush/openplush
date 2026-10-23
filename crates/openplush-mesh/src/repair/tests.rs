//! `into_manifold` end to end: the mesh it builds and what its report says.

use openplush_geom::DVec3;

use super::into_manifold;
use crate::{
    EdgeId, FaceId, ManifoldMesh, MeshError, RepairPlan, RepairReport, SourceCorner, TriangleSoup,
    VertexId,
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
        && r.edge_copies().is_empty()
}

#[test]
fn a_clean_closed_surface_builds_and_the_report_has_nothing_but_its_component() {
    let (mesh, report) = repair(&soup(tetra_positions(), &TETRA)).unwrap();
    assert_eq!(mesh.face_count(), 4);
    assert_eq!(mesh.vertex_count(), 4);
    assert!(nothing_to_report(&report));
    assert!(mesh.edge_copies().is_empty());
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
    assert_eq!(mesh.edge_copies(), report.edge_copies());
    // The diagonal's two copies are one group, both interior.
    let [group] = report.edge_copies() else {
        panic!("{:?}", report.edge_copies());
    };
    assert_eq!(group.len(), 2);
    for &e in group {
        let h = e.half_edge();
        assert!(
            mesh.face(h).is_some() && mesh.face(h.twin()).is_some(),
            "{e:?}"
        );
    }
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
    // One group: the body pair's interior copy and the fin's boundary copy, ascending.
    let [group] = report.edge_copies() else {
        panic!("{:?}", report.edge_copies());
    };
    let boundary = |e: &EdgeId| {
        let h = e.half_edge();
        mesh.face(h).is_none() || mesh.face(h.twin()).is_none()
    };
    assert_eq!(group.len(), 2);
    assert!(group[0] < group[1]);
    assert_eq!(group.iter().filter(|e| boundary(e)).count(), 1);
}

/// Five triangles round a ring whose last one joins back twisted: no winding agrees on edge 2-3.
fn twisted_ring() -> TriangleSoup {
    let radii = [3.0, 1.0, 2.0, 1.0, 1.0];
    let positions = radii
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let a = std::f64::consts::TAU * i as f64 / 5.0;
            p(r * libm::cos(a), r * libm::sin(a), 0.0)
        })
        .collect();
    soup(
        positions,
        &[[0, 1, 2], [1, 2, 3], [2, 3, 4], [3, 4, 0], [4, 0, 1]],
    )
}

#[test]
fn a_twisted_ring_is_cut_at_its_conflict_edge_into_an_open_strip_with_a_forced_seam() {
    let (mesh, report) = repair(&twisted_ring()).unwrap();
    assert_eq!(mesh.face_count(), 5);
    // The cut edge's two ends each split into two fans, so the strip has seven vertices and
    // one boundary loop.
    assert_eq!(mesh.vertex_count(), 7);
    assert_eq!(mesh.boundary_loops().len(), 1);
    assert_eq!(report.orientation_conflicts().len(), 1);
    assert_eq!(report.flipped_faces(), [f(0), f(3)]);
    // The two copies of the cut edge are both boundaries, both mandatory seams, one group.
    let [group] = report.edge_copies() else {
        panic!("{:?}", report.edge_copies());
    };
    assert_eq!(group.len(), 2);
    for &e in group {
        let h = e.half_edge();
        assert!(
            mesh.face(h).is_none() || mesh.face(h.twin()).is_none(),
            "{e:?} is a boundary"
        );
        let mut ends = [mesh.origin(h), mesh.dest(h)];
        ends.sort();
        assert!(report.mandatory_seams().contains(&ends), "{e:?} is a seam");
    }
    assert_eq!(report.mandatory_seams().len(), 2);
    // The conflict is listed by the copy its lower face kept: face 1 keeps vertices 2 and 3.
    assert_eq!(report.orientation_conflicts(), [[v(2), v(3)]]);
}

#[test]
fn with_vertex_splitting_off_the_build_cuts_the_conflict_and_refuses_its_shared_ends() {
    // Nothing separates the cut's ends into fans, so the build itself cuts the edge and then
    // refuses the two-fan vertex that leaves; the conflict is never left uncut.
    let plan = without(|p| p.split_bowties = false);
    assert_eq!(
        into_manifold(&twisted_ring(), &plan).unwrap_err(),
        MeshError::NonManifoldVertex { vertex: v(2) }
    );
}

#[test]
fn a_copy_groups_edges_are_ascending_whatever_order_their_faces_came_in() {
    // The fin listed first gets the lower edge IDs, so the group must still be sorted.
    let mut positions = tetra_positions();
    positions.push(p(0.5, -1., 0.));
    let mut triangles = vec![[0, 1, 4]];
    triangles.extend(TETRA);
    let (_, report) = repair(&soup(positions, &triangles)).unwrap();
    let [group] = report.edge_copies() else {
        panic!("{:?}", report.edge_copies());
    };
    assert_eq!(group.len(), 2);
    assert!(group[0] < group[1], "{group:?}");
}

#[test]
fn copy_groups_come_in_order_of_their_first_edge_whether_cut_or_paired() {
    // A twisted ring (vertices 0 to 4) beside a fin on a tetrahedron moved to x + 10: the cut's
    // group has the lower edges, and a pairing's group listed first would come out last unsorted.
    let ring = twisted_ring();
    let mut positions = ring.positions().to_vec();
    positions.extend(
        tetra_positions()
            .into_iter()
            .map(|q| p(q.x + 10., q.y, q.z)),
    );
    positions.push(p(10.5, -1., 0.));
    let mut triangles = ring.triangle_indices().to_vec();
    triangles.extend(TETRA.iter().map(|t| t.map(|i| i + 5)));
    triangles.push([5, 6, 9]);
    let (_, report) = repair(&soup(positions, &triangles)).unwrap();
    let groups = report.edge_copies();
    assert_eq!(groups.len(), 2, "{groups:?}");
    assert!(groups[0][0] < groups[1][0], "{groups:?}");
    assert!(
        groups.iter().all(|g| g.len() == 2 && g[0] < g[1]),
        "{groups:?}"
    );
}

#[test]
fn a_twisted_ring_is_still_refused_when_the_plan_does_not_orient() {
    let plan = without(|p| p.fix_orientation = false);
    assert!(matches!(
        into_manifold(&twisted_ring(), &plan).unwrap_err(),
        MeshError::InconsistentOrientation { .. }
    ));
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
fn a_plan_that_fills_a_loop_fills_it_and_a_missing_loop_is_named() {
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
    let (_, report) = into_manifold(&s, &named(0)).unwrap();
    assert!(report.holes()[0].filled());
    assert!(!report.holes()[0].added_faces().is_empty());
    assert!(matches!(
        into_manifold(&s, &named(9)).unwrap_err(),
        MeshError::UnknownHoleLoop { .. }
    ));
}

#[test]
fn a_cap_vertex_a_few_weld_distances_from_a_corner_leaves_a_sliver_the_build_refuses() {
    // The cap 0-3-1 puts vertex 3 on the long edge 0-1, 1.2 weld distances from corner 0.
    // Stitching splits face 0 there, and the piece at corner 0 has a base of 1.2e-6 and a
    // height of 1.2e-6: under the area tolerance.
    let s = soup(
        vec![
            p(0., 0., 0.),
            p(1., 0., 0.),
            p(0., 1.2e-6, 0.),
            p(1.2e-6, 0., 0.),
        ],
        &[[0, 1, 2], [0, 3, 1]],
    );
    assert!(matches!(
        repair(&s).unwrap_err(),
        MeshError::ZeroAreaTriangle { .. }
    ));
}

fn without(change: fn(&mut RepairPlan)) -> RepairPlan {
    let mut plan = RepairPlan::default();
    change(&mut plan);
    plan
}

fn unwelded_tetra() -> TriangleSoup {
    let base = tetra_positions();
    let mut positions = Vec::new();
    let mut triangles = Vec::new();
    for (i, tri) in TETRA.iter().enumerate() {
        positions.extend(tri.map(|k| base[k as usize]));
        triangles.push([3 * i as u32, 3 * i as u32 + 1, 3 * i as u32 + 2]);
    }
    soup(positions, &triangles)
}

fn square_sheet() -> TriangleSoup {
    soup(
        vec![p(0., 0., 0.), p(1., 0., 0.), p(1., 1., 0.), p(0., 1., 0.)],
        &[[0, 1, 2], [0, 2, 3], [0, 3, 2], [2, 1, 0]],
    )
}

const FAN: [[u32; 3]; 6] = [
    [0, 1, 2],
    [0, 2, 3],
    [0, 3, 1],
    [0, 2, 1],
    [0, 3, 2],
    [0, 1, 3],
];

fn fan_positions() -> Vec<DVec3> {
    vec![p(0., 0., 0.), p(1., 0., 0.), p(0., 1., 0.), p(-1., -1., 0.)]
}

#[test]
fn each_step_the_plan_turns_off_is_skipped() {
    // Left unwelded, the faces stay four separate triangles.
    let (mesh, report) = into_manifold(&unwelded_tetra(), &without(|p| p.weld = false)).unwrap();
    assert_eq!(mesh.vertex_count(), 12);
    assert!(report.welded().is_empty());
    // Left unsplit, the sheet's interior vertex is a bowtie.
    let sheet = soup(fan_positions(), &FAN);
    assert_eq!(
        into_manifold(&sheet, &without(|p| p.split_bowties = false)).unwrap_err(),
        MeshError::NonManifoldVertex { vertex: v(0) }
    );
    // Left unturned, the inside-out shell is built as it is.
    let reversed: Vec<[u32; 3]> = TETRA.iter().map(|&[a, b, c]| [a, c, b]).collect();
    let inside_out = soup(tetra_positions(), &reversed);
    let (_, report) = into_manifold(&inside_out, &without(|p| p.fix_orientation = false)).unwrap();
    assert!(report.flipped_faces().is_empty());
    // Left unpaired, the sheet's diagonal has four faces.
    assert_eq!(
        into_manifold(
            &square_sheet(),
            &without(|p| p.split_non_manifold_edges = false)
        )
        .unwrap_err(),
        MeshError::NonManifoldEdge { a: v(0), b: v(2) }
    );
    // Left alone, the sheet's two layers are duplicates to the build.
    assert_eq!(
        into_manifold(&square_sheet(), &without(|p| p.drop_degenerates = false)).unwrap_err(),
        MeshError::DuplicateTriangle {
            triangle: f(2),
            first: f(1)
        }
    );
}

#[test]
fn a_repeated_copy_of_a_face_is_dropped_and_listed_by_input_id() {
    let mut triangles = TETRA.to_vec();
    triangles.push(TETRA[0]);
    let (mesh, report) = repair(&soup(tetra_positions(), &triangles)).unwrap();
    assert_eq!(mesh.face_count(), 4);
    assert_eq!(report.duplicate_faces().len(), 1);
    assert_eq!(report.duplicate_faces()[0].face, f(4));
    assert_eq!(report.duplicate_faces()[0].of, f(0));
}

#[test]
fn ids_are_the_inputs_after_a_leading_face_is_dropped() {
    let reversed: Vec<[u32; 3]> = TETRA.iter().map(|&[a, b, c]| [a, c, b]).collect();
    let mut triangles = vec![[0, 0, 1]];
    triangles.extend(reversed);
    let (_, report) = repair(&soup(tetra_positions(), &triangles)).unwrap();
    assert_eq!(report.repeated_index_faces(), [f(0)]);
    assert_eq!(report.flipped_faces(), [f(1), f(2), f(3), f(4)]);
    assert_eq!(report.components()[0].faces, [f(1), f(2), f(3), f(4)]);
}

#[test]
fn a_cap_and_the_face_it_splits_are_listed_by_input_id() {
    // Face 3 is a cap whose middle vertex 4 lies on the edge 0-1 of face 1.
    let s = soup(
        vec![
            p(0., 0., 0.),
            p(1., 0., 0.),
            p(1., 1., 0.),
            p(0., 1., 0.),
            p(0.5, 0., 0.),
        ],
        &[[0, 0, 1], [0, 1, 2], [0, 2, 3], [0, 4, 1]],
    );
    let (mesh, report) = repair(&s).unwrap();
    assert_eq!(report.repeated_index_faces(), [f(0)]);
    assert_eq!(report.stitched_caps(), [f(3)]);
    assert_eq!(report.retriangulated_faces(), [f(1)]);
    assert_eq!(report.components()[0].faces, [f(1), f(2)]);
    assert_eq!(mesh.face_count(), 3);
}

#[test]
fn the_report_names_the_pairing_and_seams_in_input_ids_and_mesh_vertices() {
    // Vertex 0 is unused, face 0 is dropped, and face 5 is a fin on edge 1-2.
    let mut positions = vec![p(0.3, 0.3, 0.3)];
    positions.extend(tetra_positions());
    positions.push(p(0.5, -1., 0.));
    let mut triangles = vec![[1, 1, 2]];
    triangles.extend(TETRA.iter().map(|t| t.map(|i| i + 1)));
    triangles.push([1, 2, 5]);
    let (mesh, report) = repair(&soup(positions, &triangles)).unwrap();
    assert_eq!(report.repeated_index_faces(), [f(0)]);
    // The pairing names the input's vertices and faces.
    assert_eq!(report.paired_edges().len(), 1);
    let pairing = &report.paired_edges()[0];
    assert_eq!(pairing.vertices, [v(1), v(2)]);
    assert_eq!(pairing.pairs, [[f(1), f(2)]]);
    assert_eq!(pairing.unpaired, Some(f(5)));
    // The copies and the seam are the mesh's: the unused vertex is gone.
    assert_eq!(
        (report.vertex_copies()[0].of, report.vertex_copies()[0].copy),
        (v(0), v(5))
    );
    assert_eq!(
        (report.vertex_copies()[1].of, report.vertex_copies()[1].copy),
        (v(1), v(6))
    );
    assert_eq!(report.mandatory_seams(), [[v(5), v(6)]]);
    assert_eq!((mesh.vertex_count(), mesh.face_count()), (7, 5));
}

#[test]
fn a_fin_left_unsplit_is_refused_not_built() {
    // The unpaired fin keeps the body's vertices, so the build refuses the shared fan.
    let mut positions = tetra_positions();
    positions.push(p(0.5, -1., 0.));
    let mut triangles = TETRA.to_vec();
    triangles.push([0, 1, 4]);
    let plan = without(|p| p.split_bowties = false);
    assert!(into_manifold(&soup(positions, &triangles), &plan).is_err());
}

#[test]
fn a_leading_unused_vertex_does_not_move_the_copy_a_sheet_gets() {
    let mut positions = vec![p(0.3, 0.3, 0.3)];
    positions.extend(fan_positions());
    let triangles: Vec<[u32; 3]> = FAN.iter().map(|t| t.map(|i| i + 1)).collect();
    let (_, report) = repair(&soup(positions, &triangles)).unwrap();
    assert_eq!(report.vertex_copies().len(), 1);
    assert_eq!(report.vertex_copies()[0].of, v(0));
    assert_eq!(report.vertex_copies()[0].copy, v(4));
}

#[test]
fn a_plan_naming_a_real_loop_and_a_missing_one_reports_the_missing_one() {
    let s = soup(
        vec![p(0., 0., 0.), p(1., 0., 0.), p(1., 1., 0.), p(0., 1., 0.)],
        &[[0, 1, 2], [0, 2, 3]],
    );
    let corner = |vertex| SourceCorner {
        primitive: 0,
        vertex,
    };
    let mut plan = RepairPlan::default();
    plan.fill_holes.insert(crate::LoopRef::new(corner(0)));
    plan.fill_holes.insert(crate::LoopRef::new(corner(9)));
    assert_eq!(
        into_manifold(&s, &plan).unwrap_err(),
        MeshError::UnknownHoleLoop {
            loop_ref: crate::LoopRef::new(corner(9))
        }
    );
}

#[test]
fn sheets_found_with_degenerates_kept_are_not_reported_and_not_built() {
    let (mesh, report) = repair(&square_sheet()).unwrap();
    assert_eq!(report.sheets().len(), 2);
    assert_eq!(mesh.face_count(), 4);
    let plan = without(|p| p.drop_degenerates = false);
    assert!(into_manifold(&square_sheet(), &plan).is_err());
}

#[test]
fn halves_of_one_stitched_face_are_not_reported_as_a_pairing_the_input_had() {
    // A one-triangle sheet whose edge 0-1 gets a cap vertex: both layers are split.
    let s = soup(
        vec![p(0., 0., 0.), p(1., 0., 0.), p(0., 1., 0.), p(0.5, 0., 0.)],
        &[[0, 1, 2], [0, 2, 1], [0, 3, 1]],
    );
    let (mesh, report) = repair(&s).unwrap();
    assert_eq!(mesh.face_count(), 4);
    assert!(report.paired_edges().is_empty());
}
