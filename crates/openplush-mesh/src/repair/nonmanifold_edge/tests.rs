//! Edge cutting, checked against fan, pairing and connectivity oracles.

use openplush_geom::{DVec2, DVec3};

use super::{PairedEdge, pair_edges};
use crate::{FaceId, MeshError, SourceCorner, TriangleSoup, VertexId};

fn f(i: u32) -> FaceId {
    FaceId::new(i)
}

fn v(i: u32) -> VertexId {
    VertexId::new(i)
}

/// Corner `i` has source vertex `i` and UV (i, 0.5).
fn soup_of(positions: Vec<DVec3>, triangles: &[[u32; 3]]) -> TriangleSoup {
    let n = triangles.len() * 3;
    let corners = (0..n)
        .map(|i| SourceCorner {
            primitive: 0,
            vertex: i as u32,
        })
        .collect();
    let uvs = (0..n).map(|i| Some(DVec2::new(i as f64, 0.5))).collect();
    TriangleSoup::new(positions, triangles.to_vec(), corners, Some(uvs)).unwrap()
}

/// Vertices 0 and 1 are the ends of the edge along x; vertex 2 + k lies
/// `angles[k]` degrees round it, in the y-z plane. A `true` face walks the edge
/// from 0 to 1.
fn book(angles: &[f64], forward: &[bool]) -> TriangleSoup {
    let mut positions = vec![DVec3::new(0.0, 0.0, 0.0), DVec3::new(1.0, 0.0, 0.0)];
    for a in angles {
        let r = a.to_radians();
        positions.push(DVec3::new(0.5, r.cos(), r.sin()));
    }
    let tris: Vec<[u32; 3]> = forward
        .iter()
        .enumerate()
        .map(|(k, &fwd)| {
            let c = 2 + k as u32;
            if fwd { [0, 1, c] } else { [1, 0, c] }
        })
        .collect();
    soup_of(positions, &tris)
}

/// The only paired edge of `s`.
fn only_edge(s: &TriangleSoup, sheets: &[[FaceId; 2]]) -> PairedEdge {
    let mut edges = pair_edges(s, sheets).unwrap();
    assert_eq!(edges.len(), 1);
    edges.remove(0)
}

fn pairs(s: &TriangleSoup) -> Vec<[FaceId; 2]> {
    only_edge(s, &[]).pairs
}

fn square_sheet() -> TriangleSoup {
    soup_of(
        vec![
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(1.0, 0.0, 0.0),
            DVec3::new(1.0, 1.0, 0.0),
            DVec3::new(0.0, 1.0, 0.0),
        ],
        &[[0, 1, 2], [0, 2, 3], [0, 3, 2], [2, 1, 0]],
    )
}

#[test]
fn a_manifold_soup_has_no_paired_edges() {
    let s = book(&[0.0, 180.0], &[true, false]);
    assert!(pair_edges(&s, &[]).unwrap().is_empty());
}

#[test]
fn pairs_the_nearest_faces_round_the_edge() {
    let s = book(&[0.0, 85.0, 95.0, 180.0], &[true, false, true, false]);
    let edge = only_edge(&s, &[]);
    assert_eq!(edge.vertices, [v(0), v(1)]);
    assert_eq!(edge.pairs, [[f(0), f(1)], [f(2), f(3)]]);
    assert_eq!(edge.unpaired, None);
}

#[test]
fn the_smaller_total_angle_between_pairs_decides_when_both_pairings_walk_opposite_ways() {
    let s = book(&[0.0, 10.0, 20.0, 200.0], &[true, false, true, false]);
    assert_eq!(pairs(&s), [[f(0), f(3)], [f(1), f(2)]]);
}

#[test]
fn a_tie_keeps_the_first_pairing() {
    let s = book(&[0.0, 90.0, 180.0, 270.0], &[true, false, true, false]);
    assert_eq!(pairs(&s), [[f(0), f(1)], [f(2), f(3)]]);
}

#[test]
fn prefers_pairs_that_walk_the_edge_opposite_ways_to_closer_ones() {
    let s = book(&[0.0, 85.0, 95.0, 180.0], &[true, true, false, false]);
    assert_eq!(pairs(&s), [[f(0), f(3)], [f(1), f(2)]]);
}

#[test]
fn an_odd_face_is_left_unpaired() {
    let s = book(&[0.0, 120.0, 240.0], &[true, false, true]);
    let edge = only_edge(&s, &[]);
    assert_eq!(edge.pairs, [[f(1), f(2)]]);
    assert_eq!(edge.unpaired, Some(f(0)));
}

#[test]
fn an_odd_count_tries_leaving_out_every_face() {
    let s = book(&[0.0, 120.0, 240.0], &[true, false, false]);
    let edge = only_edge(&s, &[]);
    assert_eq!(edge.pairs, [[f(0), f(1)]]);
    assert_eq!(edge.unpaired, Some(f(2)));
}

#[test]
fn a_sheet_pairs_each_face_with_its_own_layer() {
    let s = square_sheet();
    let with_layers = only_edge(&s, &[[f(0), f(3)], [f(1), f(2)]]);
    assert_eq!(with_layers.vertices, [v(0), v(2)]);
    assert_eq!(with_layers.pairs, [[f(0), f(1)], [f(2), f(3)]]);
    // Without the flag the coincident layers look like the closest pair.
    assert_eq!(pairs(&s), [[f(0), f(3)], [f(1), f(2)]]);
}

#[test]
fn a_layer_pair_is_never_paired_whichever_face_comes_first_round_the_ring() {
    // Faces 0 and 1 are the layers of one triangle, wound opposite ways.
    let s = soup_of(
        vec![
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(1.0, 0.0, 0.0),
            DVec3::new(0.5, 0.0, 1.0),
            DVec3::new(0.5, 0.0, 1.0),
            DVec3::new(0.5, 0.0, -1.0),
            DVec3::new(0.5, 0.0, -1.0),
        ],
        &[[1, 0, 2], [0, 1, 3], [0, 1, 4], [1, 0, 5]],
    );
    let edge = only_edge(&s, &[[f(0), f(1)]]);
    assert_eq!(edge.pairs, [[f(0), f(2)], [f(1), f(3)]]);
}

#[test]
fn a_face_with_no_area_beside_the_edge_is_refused() {
    let mut s = book(&[0.0, 85.0, 95.0], &[true, false, true]);
    let mut positions = s.positions().to_vec();
    positions[3] = DVec3::new(0.5, 0.0, 0.0);
    s = soup_of(positions, s.triangle_indices());
    assert_eq!(
        pair_edges(&s, &[]).unwrap_err(),
        MeshError::ZeroAreaTriangle { triangle: f(1) }
    );
}

#[test]
fn an_offset_that_overflows_is_refused() {
    let mut s = book(&[0.0, 85.0, 95.0], &[true, false, true]);
    let mut positions = s.positions().to_vec();
    positions[3] = DVec3::new(0.5, 1e308, -1e308);
    s = soup_of(positions, s.triangle_indices());
    assert_eq!(
        pair_edges(&s, &[]).unwrap_err(),
        MeshError::ZeroAreaTriangle { triangle: f(1) }
    );
}
