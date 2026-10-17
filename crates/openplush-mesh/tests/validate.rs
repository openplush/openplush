//! `validate`: every defect of a soup located, judged as repair will see the mesh:
//! coincident vertices welded and duplicate faces dropped first, nothing changed.

use openplush_geom::DVec3;
use openplush_mesh::{
    Duplicate, DuplicateOrientation, FaceId, MeshError, NonManifoldEdge, SourceCorner,
    TriangleSoup, Unmeasurable, ValidationReport, VertexId, validate,
};

fn soup(positions: &[[f64; 3]], triangles: &[[u32; 3]]) -> Result<TriangleSoup, MeshError> {
    let corners = triangles
        .iter()
        .flatten()
        .map(|&vertex| SourceCorner {
            primitive: 0,
            vertex,
        })
        .collect();
    let positions = positions
        .iter()
        .map(|&[x, y, z]| DVec3::new(x, y, z))
        .collect();
    TriangleSoup::new(positions, triangles.to_vec(), corners, None)
}

fn v(i: u32) -> VertexId {
    VertexId::new(i)
}

fn f(i: u32) -> FaceId {
    FaceId::new(i)
}

const CUBE_POSITIONS: [[f64; 3]; 8] = [
    [0.0, 0.0, 0.0],
    [1.0, 0.0, 0.0],
    [1.0, 1.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 0.0, 1.0],
    [1.0, 0.0, 1.0],
    [1.0, 1.0, 1.0],
    [0.0, 1.0, 1.0],
];

// Wound counter-clockwise seen from outside.
const CUBE: [[u32; 3]; 12] = [
    [0, 2, 1],
    [0, 3, 2],
    [4, 5, 6],
    [4, 6, 7],
    [0, 1, 5],
    [0, 5, 4],
    [1, 2, 6],
    [1, 6, 5],
    [2, 3, 7],
    [2, 7, 6],
    [3, 0, 4],
    [3, 4, 7],
];

fn cube_with(extra: &[[u32; 3]]) -> Vec<[u32; 3]> {
    CUBE.iter().chain(extra).copied().collect()
}

/// Every list in the report, by name, so a test can say which ones are non-empty.
fn defects(report: &ValidationReport) -> Vec<&'static str> {
    let lists = [
        ("unwelded", report.unwelded().is_empty()),
        ("zero_area", report.zero_area().is_empty()),
        ("repeated_index", report.repeated_index().is_empty()),
        ("duplicates", report.duplicates().is_empty()),
        ("non_manifold_edges", report.non_manifold_edges().is_empty()),
        (
            "non_manifold_vertices",
            report.non_manifold_vertices().is_empty(),
        ),
        ("winding_conflicts", report.winding_conflicts().is_empty()),
        ("non_orientable", report.non_orientable().is_empty()),
        ("inside_out", report.inside_out().is_empty()),
        ("boundary_loops", report.boundary_loops().is_empty()),
        ("multiple_components", report.components().len() <= 1),
        ("self_intersections", report.self_intersections().is_empty()),
        ("unmeasurable", report.unmeasurable().is_none()),
    ];
    lists
        .iter()
        .filter(|(_, empty)| !empty)
        .map(|(name, _)| *name)
        .collect()
}

#[test]
fn a_closed_outward_cube_has_no_defect() -> Result<(), MeshError> {
    let report = validate(&soup(&CUBE_POSITIONS, &CUBE)?);
    assert_eq!(defects(&report), Vec::<&str>::new());
    assert_eq!(report.components(), [(0..12).map(f).collect::<Vec<_>>()]);
    Ok(())
}

#[test]
fn reports_coincident_vertices_and_judges_topology_welded() -> Result<(), MeshError> {
    // Vertex 8 sits within the weld distance of vertex 6 and replaces it in two faces.
    let mut positions = CUBE_POSITIONS.to_vec();
    positions.push([1.0, 1.0, 1.0 + 1e-9]);
    let mut tris = CUBE.to_vec();
    tris[2] = [4, 5, 8];
    tris[3] = [4, 8, 7];
    let report = validate(&soup(&positions, &tris)?);
    assert_eq!(report.unwelded(), [vec![v(6), v(8)]]);
    assert_eq!(defects(&report), ["unwelded"]);
    Ok(())
}

#[test]
fn reports_zero_area_and_repeated_index_faces() -> Result<(), MeshError> {
    let mut positions = CUBE_POSITIONS.to_vec();
    positions.push([0.5, 0.0, 0.0]);
    positions.push([0.0, 0.0, 0.0]);
    // A sliver along edge 0-1; a face naming vertex 2 twice; a face whose corners 0
    // and 9 are coincident, so welded it repeats a vertex.
    let tris = cube_with(&[[0, 8, 1], [2, 2, 3], [9, 0, 4]]);
    let report = validate(&soup(&positions, &tris)?);
    assert_eq!(report.zero_area(), [f(12), f(13), f(14)]);
    assert_eq!(report.repeated_index(), [f(13), f(14)]);
    Ok(())
}

#[test]
fn reports_duplicates_with_their_orientation_and_drops_them_from_topology() -> Result<(), MeshError>
{
    let report = validate(&soup(&CUBE_POSITIONS, &cube_with(&[[2, 1, 0]]))?);
    let same = Duplicate {
        face: f(12),
        of: f(0),
        orientation: DuplicateOrientation::Same,
    };
    assert_eq!(report.duplicates(), [same]);
    assert_eq!(defects(&report), ["duplicates"]);

    let report = validate(&soup(&CUBE_POSITIONS, &cube_with(&[[0, 1, 2]]))?);
    assert_eq!(
        report.duplicates()[0].orientation,
        DuplicateOrientation::Opposite
    );
    Ok(())
}

#[test]
fn reports_an_edge_with_more_than_two_faces() -> Result<(), MeshError> {
    let positions = [
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, 0.0, 1.0],
    ];
    let report = validate(&soup(&positions, &[[0, 1, 2], [1, 0, 3], [0, 1, 4]])?);
    let edge = NonManifoldEdge {
        vertices: [v(0), v(1)],
        faces: vec![f(0), f(1), f(2)],
    };
    assert_eq!(report.non_manifold_edges(), [edge]);
    assert!(report.non_manifold_vertices().is_empty());
    Ok(())
}

#[test]
fn reports_a_vertex_whose_faces_form_two_fans() -> Result<(), MeshError> {
    let positions = [
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [-1.0, 0.0, 0.0],
        [0.0, -1.0, 0.0],
    ];
    let report = validate(&soup(&positions, &[[0, 1, 2], [0, 3, 4]])?);
    assert_eq!(report.non_manifold_vertices(), [v(0)]);
    Ok(())
}

#[test]
fn traces_a_hole_as_a_boundary_loop_from_its_lowest_vertex() -> Result<(), MeshError> {
    let tris: Vec<[u32; 3]> = CUBE[2..].to_vec();
    let report = validate(&soup(&CUBE_POSITIONS, &tris)?);
    assert_eq!(report.boundary_loops(), [vec![v(0), v(1), v(2), v(3)]]);
    assert_eq!(defects(&report), ["boundary_loops"]);
    Ok(())
}

#[test]
fn reports_the_edges_of_a_face_wound_against_its_neighbours() -> Result<(), MeshError> {
    let mut tris = CUBE.to_vec();
    tris[2] = [4, 6, 5];
    let report = validate(&soup(&CUBE_POSITIONS, &tris)?);
    assert_eq!(
        report.winding_conflicts(),
        [[v(4), v(5)], [v(4), v(6)], [v(5), v(6)]]
    );
    assert_eq!(defects(&report), ["winding_conflicts"]);
    Ok(())
}

#[test]
fn reports_a_closed_component_wound_inside_out() -> Result<(), MeshError> {
    let tris: Vec<[u32; 3]> = CUBE.iter().map(|&[a, b, c]| [a, c, b]).collect();
    let report = validate(&soup(&CUBE_POSITIONS, &tris)?);
    assert_eq!(report.inside_out(), [(0..12).map(f).collect::<Vec<_>>()]);
    assert_eq!(defects(&report), ["inside_out"]);
    Ok(())
}

#[test]
fn reports_a_surface_no_flip_can_orient() -> Result<(), MeshError> {
    // A Möbius strip of five quads: the last quad joins the first with the sides swapped.
    let n = 5;
    let mut positions = Vec::new();
    for i in 0..n {
        let theta = std::f64::consts::TAU * f64::from(i) / f64::from(n);
        let phi = theta / 2.0;
        let (c, s) = (theta.cos(), theta.sin());
        for side in [1.0, -1.0] {
            let w = 0.3 * side;
            positions.push([
                c * (2.0 + w * phi.cos()),
                s * (2.0 + w * phi.cos()),
                w * phi.sin(),
            ]);
        }
    }
    let mut tris = Vec::new();
    for i in 0..n {
        let (a, b) = (2 * i, 2 * i + 1);
        let (c, d) = if i + 1 < n {
            (2 * i + 3, 2 * i + 2)
        } else {
            (0, 1)
        };
        tris.push([a, b, c]);
        tris.push([a, c, d]);
    }
    let report = validate(&soup(&positions, &tris)?);
    assert!(!report.non_orientable().is_empty(), "{report:?}");
    assert!(report.inside_out().is_empty());
    Ok(())
}

#[test]
fn lists_each_vertex_connected_component() -> Result<(), MeshError> {
    let mut positions = CUBE_POSITIONS.to_vec();
    positions.extend([
        [3.0, 0.0, 0.0],
        [4.0, 0.0, 0.0],
        [3.0, 1.0, 0.0],
        [3.0, 0.0, 1.0],
    ]);
    let tris = cube_with(&[[8, 10, 9], [8, 9, 11], [9, 10, 11], [10, 8, 11]]);
    let report = validate(&soup(&positions, &tris)?);
    let expected = [
        (0..12).map(f).collect::<Vec<_>>(),
        (12..16).map(f).collect(),
    ];
    assert_eq!(report.components(), expected);
    assert_eq!(defects(&report), ["multiple_components"]);
    Ok(())
}

#[test]
fn reports_a_soup_it_cannot_measure_and_nothing_else() -> Result<(), MeshError> {
    let report = validate(&soup(&[[0.0, 0.0, 0.0]], &[])?);
    assert_eq!(report.unmeasurable(), Some(Unmeasurable::NoTriangles));
    assert_eq!(defects(&report), ["unmeasurable"]);

    let report = validate(&soup(&[[1.0, 1.0, 1.0]], &[[0, 0, 0]])?);
    assert_eq!(
        report.unmeasurable(),
        Some(Unmeasurable::Extent { diagonal: 0.0 })
    );
    assert_eq!(defects(&report), ["unmeasurable"]);
    Ok(())
}

#[test]
fn ignores_unused_vertices() -> Result<(), MeshError> {
    let mut positions = CUBE_POSITIONS.to_vec();
    positions.push([0.0, 0.0, 0.0]);
    positions.push([50.0, 50.0, 50.0]);
    let report = validate(&soup(&positions, &CUBE)?);
    assert_eq!(defects(&report), Vec::<&str>::new());
    Ok(())
}

fn inverted(tris: &[[u32; 3]]) -> Vec<[u32; 3]> {
    tris.iter().map(|&[a, b, c]| [a, c, b]).collect()
}

#[test]
fn welds_vertices_exactly_the_weld_distance_apart() -> Result<(), MeshError> {
    // Diagonal exactly 5, so the weld distance is 1e-6 * 5 and vertex 3 sits on it.
    let w = 1e-6 * 5.0;
    let positions = [
        [0.0, 0.0, 0.0],
        [3.0, 0.0, 0.0],
        [0.0, 4.0, 0.0],
        [w, 0.0, 0.0],
    ];
    let report = validate(&soup(&positions, &[[0, 1, 2], [3, 1, 2]])?);
    assert_eq!(report.unwelded(), [vec![v(0), v(3)]]);
    Ok(())
}

#[test]
fn welds_vertices_that_differ_in_x_and_lists_the_group_sorted() -> Result<(), MeshError> {
    // Vertex 8 lies just below vertex 0 in x, so the x sweep meets it first.
    let mut positions = CUBE_POSITIONS.to_vec();
    positions.push([-1e-9, 0.0, 0.0]);
    let mut tris = CUBE.to_vec();
    tris[0] = [8, 2, 1];
    tris[1] = [8, 3, 2];
    let report = validate(&soup(&positions, &tris)?);
    assert_eq!(report.unwelded(), [vec![v(0), v(8)]]);
    assert_eq!(defects(&report), ["unwelded"]);
    Ok(())
}

#[test]
fn names_a_welded_vertex_by_its_smallest_id() -> Result<(), MeshError> {
    let mut positions = CUBE_POSITIONS.to_vec();
    positions.push([1.0, 1.0, 1.0 + 1e-9]);
    let mut tris = CUBE.to_vec();
    tris[2] = [4, 8, 5];
    tris[3] = [4, 8, 7];
    let report = validate(&soup(&positions, &tris)?);
    assert_eq!(
        report.winding_conflicts(),
        [[v(4), v(5)], [v(4), v(6)], [v(5), v(6)]]
    );
    Ok(())
}

#[test]
fn a_vertex_repeated_in_any_two_corners_is_a_repeated_index() -> Result<(), MeshError> {
    let tris = cube_with(&[[0, 4, 4], [4, 0, 4]]);
    let report = validate(&soup(&CUBE_POSITIONS, &tris)?);
    assert_eq!(report.repeated_index(), [f(12), f(13)]);
    Ok(())
}

#[test]
fn repeated_index_faces_are_left_out_of_topology() -> Result<(), MeshError> {
    let report = validate(&soup(&CUBE_POSITIONS, &cube_with(&[[2, 2, 3]]))?);
    assert_eq!(defects(&report), ["zero_area", "repeated_index"]);
    Ok(())
}

#[test]
fn an_exact_or_rotated_copy_is_a_same_orientation_duplicate() -> Result<(), MeshError> {
    for copy in [[0, 2, 1], [1, 0, 2]] {
        let report = validate(&soup(&CUBE_POSITIONS, &cube_with(&[copy]))?);
        let same = Duplicate {
            face: f(12),
            of: f(0),
            orientation: DuplicateOrientation::Same,
        };
        assert_eq!(report.duplicates(), [same], "{copy:?}");
    }
    Ok(())
}

#[test]
fn a_flat_closed_component_is_inside_out() -> Result<(), MeshError> {
    // Zero signed volume is "not positive".
    let positions = [
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
    ];
    let tris = [[0, 1, 2], [0, 3, 1], [0, 2, 3], [1, 3, 2]];
    let report = validate(&soup(&positions, &tris)?);
    assert_eq!(report.inside_out(), [(0..4).map(f).collect::<Vec<_>>()]);
    Ok(())
}

#[test]
fn judges_each_shell_on_either_side_of_a_non_manifold_edge() -> Result<(), MeshError> {
    // An inverted tetrahedron shares edge 0-1 of an inverted cube, which then has four
    // faces; each shell is closed on its own and inside-out.
    let mut positions = CUBE_POSITIONS.to_vec();
    positions.extend([[0.5, -1.0, 0.0], [0.5, -0.5, -1.0]]);
    let mut tris = inverted(&CUBE);
    tris.extend(inverted(&[[0, 1, 9], [1, 8, 9], [8, 0, 9], [0, 8, 1]]));
    let report = validate(&soup(&positions, &tris)?);
    assert!(!report.non_manifold_edges().is_empty());
    let expected = [
        (0..12).map(f).collect::<Vec<_>>(),
        (12..16).map(f).collect(),
    ];
    assert_eq!(report.inside_out(), expected);
    Ok(())
}

#[test]
fn reports_an_inside_out_shell_touching_the_rest_at_a_vertex() -> Result<(), MeshError> {
    // A half-size cube, wound inward, touches the outward cube at its corner 6.
    let mut positions = CUBE_POSITIONS.to_vec();
    positions.extend(
        CUBE_POSITIONS
            .iter()
            .skip(1)
            .map(|&[x, y, z]| [1.0 + 0.5 * x, 1.0 + 0.5 * y, 1.0 + 0.5 * z]),
    );
    // The small cube's corner 0 is vertex 6; its corners 1..7 are vertices 8..14.
    let small = |i: u32| if i == 0 { 6 } else { i + 7 };
    let mut tris = CUBE.to_vec();
    tris.extend(inverted(&CUBE).iter().map(|t| t.map(small)));
    let report = validate(&soup(&positions, &tris)?);
    assert_eq!(report.non_manifold_vertices(), [v(6)]);
    assert_eq!(report.inside_out(), [(12..24).map(f).collect::<Vec<_>>()]);
    Ok(())
}

#[test]
fn splits_a_pinched_boundary_into_one_loop_per_side_whatever_the_labels() -> Result<(), MeshError> {
    let positions = [
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [2.0, 1.0, 0.0],
        [9.0, 9.0, 9.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
    ];
    let report = validate(&soup(&positions, &[[0, 4, 5], [4, 1, 2]])?);
    assert_eq!(
        report.boundary_loops(),
        [vec![v(0), v(4), v(5)], vec![v(1), v(2), v(4)]]
    );
    let relabelled = [
        [1.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [2.0, 1.0, 0.0],
        [9.0, 9.0, 9.0],
        [0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
    ];
    let report = validate(&soup(&relabelled, &[[4, 0, 5], [0, 1, 2]])?);
    assert_eq!(report.boundary_loops().len(), 2);
    Ok(())
}

#[test]
fn an_open_inverted_component_is_not_judged_inside_out() -> Result<(), MeshError> {
    let report = validate(&soup(&CUBE_POSITIONS, &inverted(&CUBE[2..]))?);
    assert_eq!(defects(&report), ["boundary_loops"]);
    Ok(())
}

#[test]
fn a_thin_outward_component_at_the_smallest_extent_is_not_inside_out() -> Result<(), MeshError> {
    // Unscaled, this tetrahedron's signed volume (about 1e-330) underflows to zero.
    let s = 1e-100;
    let positions = [
        [0.0, 0.0, 0.0],
        [s, 0.0, 0.0],
        [0.0, s, 0.0],
        [0.3 * s, 0.3 * s, 1e-30 * s],
    ];
    let tris = [[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]];
    let report = validate(&soup(&positions, &tris)?);
    assert_eq!(defects(&report), Vec::<&str>::new());
    Ok(())
}

#[test]
fn judges_inside_out_far_from_the_origin() -> Result<(), MeshError> {
    let positions: Vec<[f64; 3]> = CUBE_POSITIONS.iter().map(|p| p.map(|c| c + 1e9)).collect();
    let report = validate(&soup(&positions, &inverted(&CUBE))?);
    assert_eq!(report.inside_out(), [(0..12).map(f).collect::<Vec<_>>()]);
    let report = validate(&soup(&positions, &CUBE)?);
    assert!(report.inside_out().is_empty());
    Ok(())
}

#[test]
fn boundary_loops_through_a_shared_vertex_take_the_smallest_next_vertex() -> Result<(), MeshError> {
    let positions = [
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [-1.0, 0.0, 0.0],
        [0.0, -1.0, 0.0],
    ];
    let report = validate(&soup(&positions, &[[0, 1, 2], [0, 3, 4]])?);
    assert_eq!(
        report.boundary_loops(),
        [vec![v(0), v(1), v(2)], vec![v(0), v(3), v(4)]]
    );
    Ok(())
}

#[test]
fn faces_sharing_only_their_last_corner_are_one_component() -> Result<(), MeshError> {
    let positions = [
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [-1.0, 1.0, 0.0],
        [0.0, 2.0, 0.0],
    ];
    let report = validate(&soup(&positions, &[[0, 1, 2], [3, 4, 2]])?);
    assert_eq!(report.components(), [vec![f(0), f(1)]]);
    Ok(())
}

#[test]
fn reports_each_pair_of_faces_that_cross() -> Result<(), MeshError> {
    // A triangle stabs up through the bottom side, across both of its triangles.
    let mut positions = CUBE_POSITIONS.to_vec();
    positions.extend([[0.5, 0.4, -0.5], [0.5, 0.6, -0.5], [0.5, 0.5, 0.5]]);
    let report = validate(&soup(&positions, &cube_with(&[[8, 9, 10]]))?);
    assert_eq!(report.self_intersections(), [[f(0), f(12)], [f(1), f(12)]]);
    Ok(())
}

#[test]
fn does_not_report_faces_that_touch_at_a_point_or_meet_only_at_a_shared_vertex()
-> Result<(), MeshError> {
    // One triangle's corner rests on the bottom side; another shares cube vertex 0.
    let mut positions = CUBE_POSITIONS.to_vec();
    positions.extend([[0.5, 0.25, 0.0], [0.5, 0.25, -1.0], [0.6, 0.25, -1.0]]);
    positions.extend([[0.5, 0.5, 0.5], [0.2, 0.5, 0.8]]);
    let report = validate(&soup(&positions, &cube_with(&[[8, 9, 10], [0, 11, 12]]))?);
    assert!(report.self_intersections().is_empty(), "{report:?}");
    Ok(())
}

#[test]
fn reports_faces_crossing_along_a_segment_that_ends_on_both_boundaries() -> Result<(), MeshError> {
    // No edge passes through the other face's interior: the crossing runs from
    // (0, 0, 0) to (2, 0, 0), each end on an edge of both faces.
    let positions = [
        [0.0, -1.0, 0.0],
        [0.0, 1.0, 0.0],
        [4.0, -1.0, 0.0],
        [0.0, 0.0, -1.0],
        [0.0, 0.0, 1.0],
        [4.0, 0.0, -1.0],
    ];
    let report = validate(&soup(&positions, &[[0, 1, 2], [3, 4, 5]])?);
    assert_eq!(report.self_intersections(), [[f(0), f(1)]]);
    Ok(())
}

#[test]
fn reports_two_grid_snapped_boxes_that_overlap() -> Result<(), MeshError> {
    // Offsets that leave faces coplanar meet only edge-on or in one plane, which a
    // pair of faces cannot tell from touching; these cross transversely.
    for offset in [[0.5, 0.5, 0.5], [0.25, 0.5, 0.75]] {
        let mut positions = CUBE_POSITIONS.to_vec();
        positions.extend(
            CUBE_POSITIONS
                .iter()
                .map(|p| [p[0] + offset[0], p[1] + offset[1], p[2] + offset[2]]),
        );
        let shifted = CUBE.iter().map(|t| t.map(|v| v + 8));
        let tris: Vec<[u32; 3]> = CUBE.iter().copied().chain(shifted).collect();
        let report = validate(&soup(&positions, &tris)?);
        assert!(!report.self_intersections().is_empty(), "offset {offset:?}");
    }
    Ok(())
}

#[test]
fn reports_a_fold_between_faces_that_share_a_vertex() -> Result<(), MeshError> {
    // The second face's far edge passes through the first face at (0.75, 0.75, 0).
    let positions = [
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [1.0, 0.5, -1.0],
        [0.5, 1.0, 1.0],
    ];
    let report = validate(&soup(&positions, &[[0, 1, 2], [0, 3, 4]])?);
    assert_eq!(report.self_intersections(), [[f(0), f(1)]]);
    Ok(())
}

#[test]
fn reports_every_face_through_one_large_face_in_face_order() -> Result<(), MeshError> {
    // Twenty fins, one per unit along x, each pierce face 0 well inside its edges.
    let mut positions = vec![[-10.0, -10.0, 0.0], [30.0, -10.0, 0.0], [-10.0, 30.0, 0.0]];
    let mut triangles = vec![[0, 1, 2]];
    for k in 0..20u32 {
        let x = f64::from(k);
        positions.extend([[x, 0.0, -1.0], [x + 0.3, 0.0, -1.0], [x + 0.1, 0.0, 1.0]]);
        triangles.push([3 + 3 * k, 4 + 3 * k, 5 + 3 * k]);
    }
    let report = validate(&soup(&positions, &triangles)?);
    let expected: Vec<[FaceId; 2]> = (1..=20).map(|g| [f(0), f(g)]).collect();
    assert_eq!(report.self_intersections(), expected.as_slice());
    Ok(())
}

#[test]
fn reports_a_crossing_found_through_one_edge_alone() -> Result<(), MeshError> {
    // One corner rests inside the flat triangle, so only the edge joining the other
    // two passes through it; each rotation puts that edge in a different slot.
    let positions = [
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [0.5, 0.5, -1.0],
        [0.6, 0.4, 0.0],
        [0.5, 0.5, 1.0],
    ];
    for fin in [[3, 4, 5], [5, 3, 4], [4, 5, 3]] {
        let report = validate(&soup(&positions, &[[0, 1, 2], fin])?);
        assert_eq!(report.self_intersections(), [[f(0), f(1)]], "{fin:?}");
    }
    Ok(())
}

#[test]
fn does_not_report_an_edge_that_crosses_the_plane_on_the_other_face_edge() -> Result<(), MeshError>
{
    // The upright triangle's edge meets the flat one exactly on its edge y = 0.
    let positions = [
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [1.0, 0.0, -1.0],
        [1.0, 0.0, 1.0],
        [1.0, -1.0, 0.0],
    ];
    let report = validate(&soup(&positions, &[[0, 1, 2], [3, 4, 5]])?);
    assert!(report.self_intersections().is_empty(), "{report:?}");
    Ok(())
}

#[test]
fn reports_crossings_by_soup_face_id_when_a_duplicate_is_dropped() -> Result<(), MeshError> {
    // Face 12 repeats face 0 and is not judged; the stab is face 13, not 12.
    let mut positions = CUBE_POSITIONS.to_vec();
    positions.extend([[0.5, 0.4, -0.5], [0.5, 0.6, -0.5], [0.5, 0.5, 0.5]]);
    let report = validate(&soup(&positions, &cube_with(&[[0, 2, 1], [8, 9, 10]]))?);
    assert_eq!(report.self_intersections(), [[f(0), f(13)], [f(1), f(13)]]);
    Ok(())
}

#[test]
fn does_not_report_a_face_that_dips_through_another_by_less_than_the_weld_distance()
-> Result<(), MeshError> {
    // One corner sits 1e-45 below the flat triangle, far inside the weld distance,
    // so the faces only touch.
    let positions = [
        [0.0, 0.0, 0.0],
        [0.9, 0.0, 0.0],
        [0.0, 0.9, 0.0],
        [0.1, 0.1, -1e-45],
        [0.1, 0.1, 1.0],
        [0.2, 0.1, 1.0],
    ];
    let report = validate(&soup(&positions, &[[0, 1, 2], [3, 4, 5]])?);
    assert!(report.self_intersections().is_empty(), "{report:?}");
    Ok(())
}

#[test]
fn does_not_report_a_face_resting_edge_on_against_another() -> Result<(), MeshError> {
    // The upright face's bottom edge lies inside the flat face; the rest of it stands
    // above, so the two only touch. Each of its three edges takes the bottom in turn.
    let flat = [[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]];
    let upright = [[0.2, 0.2, 0.0], [0.8, 0.2, 0.0], [0.5, 0.2, 1.0]];
    for turn in 0..3 {
        let mut positions = flat.to_vec();
        positions.extend((0..3).map(|i| upright[(i + turn) % 3]));
        let report = validate(&soup(&positions, &[[0, 1, 2], [3, 4, 5]])?);
        assert!(
            report.self_intersections().is_empty(),
            "turn {turn}: {report:?}"
        );
    }
    Ok(())
}

const SQUARE: [[f64; 3]; 4] = [
    [0.0, 0.0, 0.0],
    [1.0, 0.0, 0.0],
    [1.0, 1.0, 0.0],
    [0.0, 1.0, 0.0],
];

#[test]
fn judges_both_layers_of_a_double_sided_sheet_as_repair_keeps_them() -> Result<(), MeshError> {
    let report = validate(&soup(
        &SQUARE,
        &[[0, 1, 2], [0, 2, 3], [0, 3, 2], [2, 1, 0]],
    )?);
    let opposite = |face, of| Duplicate {
        face: f(face),
        of: f(of),
        orientation: DuplicateOrientation::Opposite,
    };
    assert_eq!(report.duplicates(), [opposite(2, 1), opposite(3, 0)]);
    // The layers meet along the rim, so nothing is open; inside, each layer
    // brings its two faces to the diagonal.
    assert!(report.boundary_loops().is_empty());
    let diagonal = NonManifoldEdge {
        vertices: [v(0), v(2)],
        faces: vec![f(0), f(1), f(2), f(3)],
    };
    assert_eq!(report.non_manifold_edges(), [diagonal]);
    Ok(())
}

#[test]
fn a_copy_of_a_sheets_second_layer_repeats_that_layer() -> Result<(), MeshError> {
    let report = validate(&soup(&SQUARE, &[[0, 1, 2], [2, 1, 0], [1, 0, 2]])?);
    let dup = |face, of, orientation| Duplicate {
        face: f(face),
        of: f(of),
        orientation,
    };
    assert_eq!(
        report.duplicates(),
        [
            dup(1, 0, DuplicateOrientation::Opposite),
            dup(2, 1, DuplicateOrientation::Same)
        ]
    );
    assert!(report.boundary_loops().is_empty());
    Ok(())
}
