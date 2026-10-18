//! Building with pairings: parallel edges for a double-sided sheet, and the refusals.

use super::ManifoldMesh;
use super::tests::{p, soup, tol};
use crate::repair::degenerate::remove;
use crate::repair::nonmanifold_edge::pair_edges;
use crate::{FaceId, MeshError, PairedEdge, TriangleSoup, VertexId};

fn f(i: u32) -> FaceId {
    FaceId::new(i)
}

fn v(i: u32) -> VertexId {
    VertexId::new(i)
}

fn build(
    s: &TriangleSoup,
    pairings: &[PairedEdge],
    sheets: &[[FaceId; 2]],
) -> Result<ManifoldMesh, MeshError> {
    ManifoldMesh::from_paired_triangles(s, &tol(s), pairings, sheets)
}

/// A unit square, both sides: faces 0 and 1 are one layer, 3 and 2 the other, and
/// the diagonal 0-2 has four faces.
fn square_sheet() -> TriangleSoup {
    soup(
        vec![p(0., 0., 0.), p(1., 0., 0.), p(1., 1., 0.), p(0., 1., 0.)],
        vec![[0, 1, 2], [0, 2, 3], [0, 3, 2], [2, 1, 0]],
    )
}

fn triangle_sheet() -> TriangleSoup {
    soup(
        vec![p(0., 0., 0.), p(1., 0., 0.), p(0., 1., 0.)],
        vec![[0, 1, 2], [2, 1, 0]],
    )
}

#[test]
fn a_one_triangle_sheet_builds_as_a_closed_pillow() {
    let mesh = build(&triangle_sheet(), &[], &[[f(0), f(1)]]).unwrap();
    assert_eq!(mesh.face_count(), 2);
    assert_eq!(mesh.edge_count(), 3);
    assert!(mesh.boundary_loops().is_empty());
}

#[test]
fn the_same_two_triangles_are_a_duplicate_unless_they_are_a_sheet() {
    assert_eq!(
        build(&triangle_sheet(), &[], &[]).unwrap_err(),
        MeshError::DuplicateTriangle {
            triangle: f(1),
            first: f(0)
        }
    );
    // Declaring some other pair a sheet does not excuse this one.
    let wrong = [[f(0), f(0)]];
    assert!(build(&triangle_sheet(), &[], &wrong).is_err());
}

#[test]
fn a_sheets_diagonal_gets_one_edge_per_layer() {
    let s = square_sheet();
    let (_, removed) = remove(&s);
    let pairings = pair_edges(&s, &removed.sheets).unwrap();
    let mesh = build(&s, &pairings, &removed.sheets).unwrap();
    // Four rim edges and two copies of the diagonal: a sphere, V - E + F = 2.
    assert_eq!(mesh.edge_count(), 6);
    assert_eq!(
        mesh.vertex_count() + mesh.face_count() - mesh.edge_count(),
        2
    );
    assert!(mesh.boundary_loops().is_empty());
}

#[test]
fn the_diagonal_without_its_pairing_is_still_non_manifold() {
    let s = square_sheet();
    let (_, removed) = remove(&s);
    assert_eq!(
        build(&s, &[], &removed.sheets).unwrap_err(),
        MeshError::NonManifoldEdge { a: v(0), b: v(2) }
    );
}

#[test]
fn a_pair_that_walks_the_edge_the_same_way_is_inconsistent() {
    let s = square_sheet();
    let (_, removed) = remove(&s);
    let same_way = PairedEdge {
        vertices: [v(0), v(2)],
        pairs: vec![[f(0), f(2)], [f(1), f(3)]],
        unpaired: None,
    };
    assert_eq!(
        build(&s, &[same_way], &removed.sheets).unwrap_err(),
        MeshError::InconsistentOrientation { a: v(0), b: v(2) }
    );
}

#[test]
fn a_pairing_must_name_faces_that_use_the_edge_once_each() {
    let s = square_sheet();
    let (_, removed) = remove(&s);
    let pairing = |vertices, pairs| PairedEdge {
        vertices,
        pairs,
        unpaired: None,
    };
    // Face 0 is [0, 1, 2]: it has no edge 1-3.
    assert_eq!(
        build(
            &s,
            &[pairing([v(1), v(3)], vec![[f(0), f(1)]])],
            &removed.sheets
        )
        .unwrap_err(),
        MeshError::PairingMismatch {
            face: f(0),
            a: v(1),
            b: v(3)
        }
    );
    // Face 0 twice on one edge.
    assert_eq!(
        build(
            &s,
            &[pairing([v(0), v(2)], vec![[f(0), f(1)], [f(0), f(2)]])],
            &removed.sheets
        )
        .unwrap_err(),
        MeshError::PairingMismatch {
            face: f(0),
            a: v(0),
            b: v(2)
        }
    );
}

#[test]
fn an_unpaired_face_gets_its_own_copy_and_a_fan_it_does_not_rejoin_is_refused() {
    // The sheet of one triangle, plus a third face on its edge 0-1.
    let s = soup(
        vec![p(0., 0., 0.), p(1., 0., 0.), p(0., 1., 0.), p(0., 0., 1.)],
        vec![[0, 1, 2], [2, 1, 0], [0, 1, 3]],
    );
    let pairing = PairedEdge {
        vertices: [v(0), v(1)],
        pairs: vec![[f(0), f(1)]],
        unpaired: Some(f(2)),
    };
    assert_eq!(
        build(&s, &[pairing], &[[f(0), f(1)]]).unwrap_err(),
        MeshError::NonManifoldVertex { vertex: v(0) }
    );
}

#[test]
fn without_pairings_it_builds_what_from_oriented_triangles_builds() {
    let s = soup(
        vec![p(0., 0., 0.), p(1., 0., 0.), p(0., 1., 0.), p(0., 0., 1.)],
        vec![[0, 2, 1], [0, 1, 3], [1, 2, 3], [0, 3, 2]],
    );
    let plain = ManifoldMesh::from_oriented_triangles(&s, &tol(&s)).unwrap();
    let paired = build(&s, &[], &[]).unwrap();
    assert_eq!(plain.edge_count(), paired.edge_count());
    assert_eq!(plain.face_count(), paired.face_count());
}

fn pairing(
    vertices: [VertexId; 2],
    pairs: Vec<[FaceId; 2]>,
    unpaired: Option<FaceId>,
) -> PairedEdge {
    PairedEdge {
        vertices,
        pairs,
        unpaired,
    }
}

#[test]
fn a_pairing_that_leaves_a_face_of_the_edge_out_is_refused() {
    // The sheet's diagonal 0-2 plus a fifth face on it that the pairing leaves out.
    let s = soup(
        vec![
            p(0., 0., 0.),
            p(1., 0., 0.),
            p(1., 1., 0.),
            p(0., 1., 0.),
            p(0., 0., 1.),
        ],
        vec![[0, 1, 2], [0, 2, 3], [0, 3, 2], [2, 1, 0], [0, 2, 4]],
    );
    let pairs = pairing([v(0), v(2)], vec![[f(0), f(1)], [f(2), f(3)]], None);
    // A pairing must name every face on the edge it pairs.
    assert_eq!(
        build(&s, &[pairs], &[[f(0), f(3)], [f(1), f(2)]]).unwrap_err(),
        MeshError::PairingMismatch {
            face: f(4),
            a: v(0),
            b: v(2)
        }
    );
}

#[test]
fn an_unpaired_face_gets_a_boundary_copy_of_its_own() {
    let s = square_sheet();
    let sheets = [[f(0), f(3)], [f(1), f(2)]];
    let all = pairing([v(0), v(2)], vec![[f(0), f(1)]], Some(f(2)));
    // Face 3 is on the edge too, and left out.
    assert_eq!(
        build(&s, &[all], &sheets).unwrap_err(),
        MeshError::PairingMismatch {
            face: f(3),
            a: v(0),
            b: v(2)
        }
    );
}

#[test]
fn a_pairing_on_an_edge_with_two_faces_is_refused_rather_than_cutting_it() {
    let s = soup(
        vec![p(0., 0., 0.), p(1., 0., 0.), p(0., 1., 0.), p(0., 0., 1.)],
        vec![[0, 2, 1], [0, 1, 3], [1, 2, 3], [0, 3, 2]],
    );
    let cut = pairing([v(0), v(1)], vec![], Some(f(0)));
    assert_eq!(
        build(&s, &[cut], &[]).unwrap_err(),
        MeshError::PairingMismatch {
            face: f(0),
            a: v(0),
            b: v(1)
        }
    );
}

#[test]
fn an_unpaired_face_must_use_the_edge_and_a_pair_must_name_a_real_face() {
    let s = square_sheet();
    let sheets = [[f(0), f(3)], [f(1), f(2)]];
    assert_eq!(
        build(&s, &[pairing([v(1), v(3)], vec![], Some(f(0)))], &sheets).unwrap_err(),
        MeshError::PairingMismatch {
            face: f(0),
            a: v(1),
            b: v(3)
        }
    );
    assert_eq!(
        build(
            &s,
            &[pairing([v(0), v(2)], vec![[f(0), f(99)]], None)],
            &sheets
        )
        .unwrap_err(),
        MeshError::PairingMismatch {
            face: f(99),
            a: v(0),
            b: v(2)
        }
    );
    // Face 1 is [0, 2, 3]: it has no edge 0-1.
    assert_eq!(
        build(
            &s,
            &[pairing([v(0), v(1)], vec![[f(1), f(3)]], None)],
            &sheets
        )
        .unwrap_err(),
        MeshError::PairingMismatch {
            face: f(1),
            a: v(0),
            b: v(1)
        }
    );
}
