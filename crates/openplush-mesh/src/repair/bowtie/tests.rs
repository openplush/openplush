//! Bowtie splitting, checked against fan, position and provenance oracles.

use openplush_geom::{DVec2, DVec3};

use super::{VertexCopy, split};
use crate::{SourceCorner, TriangleSoup, VertexId};

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
    let (out, split) = split(&s).unwrap();
    assert_eq!(out, s);
    assert!(split.copies.is_empty());
}

#[test]
fn two_triangles_meeting_at_a_vertex_get_a_vertex_each() {
    let s = soup(5, &[[0, 1, 2], [0, 3, 4]]);
    let (out, split) = split(&s).unwrap();
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
    let (out, _) = split(&s).unwrap();
    let t = out.triangle_indices();
    assert_eq!((t[0][2], t[2][1]), (0, 0));
    assert_eq!(t[1][0], 6);
}

#[test]
fn three_fans_get_two_copies_in_order() {
    let s = soup(7, &[[0, 1, 2], [0, 3, 4], [0, 5, 6]]);
    let (out, split) = split(&s).unwrap();
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
    let (out, split) = split(&s).unwrap();
    assert_eq!(out, s);
    assert!(split.copies.is_empty());
}

#[test]
fn a_vertex_is_split_only_where_its_own_faces_part() {
    // Vertex 0 has two fans; the faces at vertex 2 are joined by edges 2-6 and 1-2.
    let s = soup(7, &[[0, 1, 2], [0, 3, 4], [2, 5, 6], [2, 6, 1]]);
    let (out, split) = split(&s).unwrap();
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
    let (out, split) = split(&s).unwrap();
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
    let (out, split) = split(&s).unwrap();
    assert_eq!(out, s);
    assert!(split.copies.is_empty());
}
