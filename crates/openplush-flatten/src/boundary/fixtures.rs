//! Chart faces shared by the boundary tests.

use openplush_mesh::{FaceId, VertexId};

pub(super) type Faces = Vec<(FaceId, [VertexId; 3])>;

/// The triangles, numbered from face 0.
pub(super) fn faces(tris: &[[u32; 3]]) -> Faces {
    (0..)
        .zip(tris)
        .map(|(f, t)| (FaceId::new(f), t.map(VertexId::new)))
        .collect()
}

/// A `cols` by `rows` grid of quads, two counter-clockwise triangles each in (column, row),
/// on vertex `row * (cols + 1) + col`.
pub(super) fn grid(cols: u32, rows: u32) -> Faces {
    let id = |c: u32, r: u32| r * (cols + 1) + c;
    let tris: Vec<[u32; 3]> = (0..rows)
        .flat_map(|r| (0..cols).map(move |c| (c, r)))
        .flat_map(|(c, r)| {
            let (a, b, d, e) = (id(c, r), id(c + 1, r), id(c + 1, r + 1), id(c, r + 1));
            [[a, b, d], [a, d, e]]
        })
        .collect();
    faces(&tris)
}

/// A closed torus, 4 quads around by 3 across, on vertices from `base`, faces from 100.
pub(super) fn torus(base: u32) -> Faces {
    let id = |i: u32, j: u32| base + (j % 3) * 4 + i % 4;
    let tris: Vec<[u32; 3]> = (0..3)
        .flat_map(|j| (0..4).map(move |i| (i, j)))
        .flat_map(|(i, j)| {
            let (a, b, d, e) = (id(i, j), id(i + 1, j), id(i + 1, j + 1), id(i, j + 1));
            [[a, b, d], [a, d, e]]
        })
        .collect();
    faces(&tris)
        .into_iter()
        .map(|(f, t)| (FaceId::new(f.index() + 100), t))
        .collect()
}
