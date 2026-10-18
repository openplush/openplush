//! Bowtie splitting, checked against fan, position and provenance oracles.

use openplush_geom::{DVec2, DVec3};

use super::{VertexCopy, split};
use crate::PairedEdge;
use crate::repair::degenerate::remove;
use crate::repair::nonmanifold_edge::pair_edges;
use crate::{FaceId, MeshError, SourceCorner, TriangleSoup, VertexId};

fn v(i: u32) -> VertexId {
    VertexId::new(i)
}

/// `n` vertices on a line; corner `i` has source vertex `i` and UV (i, 0.5).
fn soup(n: usize, triangles: &[[u32; 3]]) -> TriangleSoup {
    let positions = (0..n)
        .map(|i| DVec3::new(i as f64, (i * i) as f64, 0.0))
        .collect();
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

#[test]
fn a_fan_of_triangles_round_a_vertex_is_left_alone() {
    let s = soup(5, &[[0, 1, 2], [0, 2, 3], [0, 3, 4]]);
    let (out, split) = split(&s, &[]).unwrap();
    assert_eq!(out, s);
    assert!(split.copies.is_empty());
}

#[test]
fn two_triangles_meeting_at_a_vertex_get_a_vertex_each() {
    let s = soup(5, &[[0, 1, 2], [0, 3, 4]]);
    let (out, split) = split(&s, &[]).unwrap();
    assert_eq!(out.triangle_indices(), [[0, 1, 2], [5, 3, 4]]);
    assert_eq!(
        split.copies,
        [VertexCopy {
            of: v(0),
            copy: v(5)
        }]
    );
    assert_eq!(out.position(v(5)), s.position(v(0)));
    assert_eq!(out.corner_sources(), s.corner_sources());
    assert_eq!(out.corner_uvs(), s.corner_uvs());
}

#[test]
fn the_fan_with_the_lowest_corner_keeps_the_vertex() {
    // Faces 0 and 2 meet along 2-0; face 1 touches them only at 0.
    let s = soup(6, &[[1, 2, 0], [0, 3, 4], [2, 0, 5]]);
    let (out, _) = split(&s, &[]).unwrap();
    let t = out.triangle_indices();
    assert_eq!((t[0][2], t[2][1]), (0, 0));
    assert_eq!(t[1][0], 6);
}

#[test]
fn three_fans_get_two_copies_in_order() {
    let s = soup(7, &[[0, 1, 2], [0, 3, 4], [0, 5, 6]]);
    let (out, split) = split(&s, &[]).unwrap();
    assert_eq!(out.triangle_indices(), [[0, 1, 2], [7, 3, 4], [8, 5, 6]]);
    assert_eq!(
        split.copies,
        [
            VertexCopy {
                of: v(0),
                copy: v(7)
            },
            VertexCopy {
                of: v(0),
                copy: v(8)
            }
        ]
    );
}

#[test]
fn faces_sharing_an_edge_with_more_than_two_faces_stay_one_fan() {
    let s = soup(6, &[[0, 1, 2], [1, 0, 3], [0, 1, 4]]);
    let (out, split) = split(&s, &[]).unwrap();
    assert_eq!(out, s);
    assert!(split.copies.is_empty());
}

#[test]
fn a_vertex_is_split_only_where_its_own_faces_part() {
    // Vertex 0 has two fans; the faces at vertex 2 are joined by edges 2-6 and 1-2.
    let s = soup(7, &[[0, 1, 2], [0, 3, 4], [2, 5, 6], [2, 6, 1]]);
    let (out, split) = split(&s, &[]).unwrap();
    let pairs: Vec<_> = split.copies.iter().map(|c| (c.of, c.copy)).collect();
    assert_eq!(pairs, [(v(0), v(7))]);
    assert_eq!(
        out.triangle_indices(),
        [[0, 1, 2], [7, 3, 4], [2, 5, 6], [2, 6, 1]]
    );
}

#[test]
fn every_bowtie_vertex_is_split_and_copies_come_in_vertex_order() {
    let s = soup(8, &[[0, 1, 2], [0, 3, 4], [5, 6, 7], [5, 3, 1]]);
    let (out, split) = split(&s, &[]).unwrap();
    let pairs: Vec<_> = split.copies.iter().map(|c| (c.of, c.copy)).collect();
    assert_eq!(
        pairs,
        [(v(0), v(8)), (v(1), v(9)), (v(3), v(10)), (v(5), v(11))]
    );
    for c in &split.copies {
        assert_eq!(out.position(c.copy), out.position(c.of));
    }
}

#[test]
fn a_soup_with_no_faces_comes_back_unchanged() {
    let s = soup(3, &[]);
    let (out, split) = split(&s, &[]).unwrap();
    assert_eq!(out, s);
    assert!(split.copies.is_empty());
}

#[test]
fn a_face_that_repeats_a_vertex_is_refused() {
    let s = soup(4, &[[0, 2, 3], [0, 0, 1]]);
    assert_eq!(
        split(&s, &[]).unwrap_err(),
        MeshError::DegenerateTriangle {
            triangle: FaceId::new(1)
        }
    );
}

fn f(i: u32) -> FaceId {
    FaceId::new(i)
}

fn pairing(vertices: [u32; 2], pairs: &[[u32; 2]], unpaired: Option<u32>) -> PairedEdge {
    PairedEdge {
        vertices: vertices.map(v),
        pairs: pairs.iter().map(|&[a, b]| [f(a), f(b)]).collect(),
        unpaired: unpaired.map(f),
    }
}

/// The most faces any edge has in `s`.
fn most_faces_on_an_edge(s: &TriangleSoup) -> usize {
    let faces: Vec<u32> = (0..s.triangle_count() as u32).collect();
    crate::validate::edges(s.triangle_indices(), &faces)
        .values()
        .map(Vec::len)
        .max()
        .unwrap_or(0)
}

#[test]
fn a_face_left_unpaired_on_a_three_face_edge_is_split_off_at_both_ends() {
    let s = soup(5, &[[0, 1, 2], [1, 0, 3], [0, 1, 4]]);
    let pairs = [pairing([0, 1], &[[0, 1]], Some(2))];
    let (out, split) = split(&s, &pairs).unwrap();
    assert_eq!(
        split.copies,
        [
            VertexCopy {
                of: v(0),
                copy: v(5)
            },
            VertexCopy {
                of: v(1),
                copy: v(6)
            }
        ]
    );
    assert_eq!(out.triangle_indices(), [[0, 1, 2], [1, 0, 3], [5, 6, 4]]);
    assert_eq!(most_faces_on_an_edge(&out), 2);
}

#[test]
fn the_two_pairs_of_a_four_face_edge_part_when_nothing_rejoins_them() {
    let s = soup(6, &[[0, 1, 2], [1, 0, 3], [0, 1, 4], [1, 0, 5]]);
    let pairs = [pairing([0, 1], &[[0, 1], [2, 3]], None)];
    let (out, split) = split(&s, &pairs).unwrap();
    assert_eq!(split.copies.len(), 2);
    assert_eq!(
        out.triangle_indices(),
        [[0, 1, 2], [1, 0, 3], [6, 7, 4], [7, 6, 5]]
    );
    assert_eq!(most_faces_on_an_edge(&out), 2);
}

#[test]
fn faces_on_an_over_shared_edge_the_pairings_do_not_name_stay_one_fan() {
    let s = soup(5, &[[0, 1, 2], [1, 0, 3], [0, 1, 4]]);
    let (out, split) = split(&s, &[]).unwrap();
    assert_eq!(out, s);
    assert!(split.copies.is_empty());
}

#[test]
fn a_square_sheet_keeps_its_rim_vertices_single() {
    let s = soup(4, &[[0, 1, 2], [0, 2, 3], [0, 3, 2], [2, 1, 0]]);
    let sheets = [[f(0), f(3)], [f(1), f(2)]];
    let pairs = pair_edges(&s, &sheets).unwrap();
    let (out, split) = split(&s, &pairs).unwrap();
    assert!(split.copies.is_empty());
    assert_eq!(out, s);
}

#[test]
fn a_sheets_interior_vertex_gets_a_copy_for_the_second_layer() {
    // Three triangles round vertex 0, both sides: faces 0-2 one layer, 3-5 the other.
    let s = soup(
        4,
        &[
            [0, 1, 2],
            [0, 2, 3],
            [0, 3, 1],
            [0, 2, 1],
            [0, 3, 2],
            [0, 1, 3],
        ],
    );
    let (_, removed) = remove(&s);
    let pairs = pair_edges(&s, &removed.sheets).unwrap();
    let (out, split) = split(&s, &pairs).unwrap();
    assert_eq!(
        split.copies,
        [VertexCopy {
            of: v(0),
            copy: v(4)
        }]
    );
    assert_eq!(
        out.triangle_indices(),
        [
            [0, 1, 2],
            [0, 2, 3],
            [0, 3, 1],
            [4, 2, 1],
            [4, 3, 2],
            [4, 1, 3]
        ]
    );
    assert_eq!(most_faces_on_an_edge(&out), 2);
}
