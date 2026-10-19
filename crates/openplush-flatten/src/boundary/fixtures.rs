//! Chart faces shared by the boundary tests.

use openplush_mesh::{FaceId, VertexId};

pub(crate) type Faces = Vec<(FaceId, [VertexId; 3])>;

/// The triangles, numbered from face 0.
pub(crate) fn faces(tris: &[[u32; 3]]) -> Faces {
    (0..)
        .zip(tris)
        .map(|(f, t)| (FaceId::new(f), t.map(VertexId::new)))
        .collect()
}

/// A `cols` by `rows` grid of quads, two counter-clockwise triangles each in (column, row),
/// on vertex `row * (cols + 1) + col`.
pub(crate) fn grid(cols: u32, rows: u32) -> Faces {
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
pub(crate) fn torus(base: u32) -> Faces {
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

/// Concentric rings around vertex 0: ring `k` has `6k` vertices from vertex `1 + 3k(k-1)`,
/// placed at radius `k / rings` and angle `j / 6k` of a turn by `place(radius, angle)`. Faces
/// are counter-clockwise.
pub(crate) fn rings(
    count: u32,
    place: impl Fn(f64, f64) -> openplush_geom::DVec3,
) -> (Vec<openplush_geom::DVec3>, Faces) {
    let mut xyz = vec![place(0.0, 0.0)];
    let mut ids: Vec<Vec<u32>> = vec![vec![0]];
    for k in 1..=count {
        let first = 1 + 3 * k * (k - 1);
        for j in 0..6 * k {
            let turn = f64::from(j) / f64::from(6 * k);
            xyz.push(place(
                f64::from(k) / f64::from(count),
                std::f64::consts::TAU * turn,
            ));
        }
        ids.push((first..first + 6 * k).collect());
    }
    let mut tris: Vec<[u32; 3]> = (0..6)
        .map(|j| [0, ids[1][j], ids[1][(j + 1) % 6]])
        .collect();
    for k in 2..=count as usize {
        zip(&ids[k - 1], &ids[k], &mut tris);
    }
    (xyz, faces(&tris))
}

/// The band between two rings, walking both by angle.
fn zip(inner: &[u32], outer: &[u32], tris: &mut Vec<[u32; 3]>) {
    let (n_in, n_out) = (inner.len(), outer.len());
    let (mut i, mut j) = (0, 0);
    while i < n_in || j < n_out {
        let a_in = (i + 1) as f64 / n_in as f64;
        let a_out = (j + 1) as f64 / n_out as f64;
        if j < n_out && (i >= n_in || a_out <= a_in) {
            tris.push([inner[i % n_in], outer[j], outer[(j + 1) % n_out]]);
            j += 1;
        } else {
            tris.push([inner[i % n_in], outer[j % n_out], inner[(i + 1) % n_in]]);
            i += 1;
        }
    }
}
