//! 4x4 affine transforms in f64, as scene readers compose them.

use openplush_geom::DVec3;

/// A 4x4 transform in column-major order, `m[column][row]`, as glTF stores it.
pub(crate) type Mat = [[f64; 4]; 4];

pub(crate) const IDENTITY: Mat = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

/// `a` after `b`: a point moved by the product is moved by `b`, then by `a`.
pub(crate) fn mul(a: &Mat, b: &Mat) -> Mat {
    let mut m = [[0.0; 4]; 4];
    for (column, out) in m.iter_mut().enumerate() {
        for (row, cell) in out.iter_mut().enumerate() {
            *cell = (0..4).map(|k| a[k][row] * b[column][k]).sum();
        }
    }
    m
}

/// `p` moved by `m`.
pub(crate) fn apply(m: &Mat, [x, y, z]: [f64; 3]) -> DVec3 {
    let row = |r: usize| m[0][r] * x + m[1][r] * y + m[2][r] * z + m[3][r];
    DVec3::new(row(0), row(1), row(2))
}

/// True when `m` turns the space inside out, so a triangle's winding must be reversed.
pub(crate) fn mirrors(m: &Mat) -> bool {
    let [a, b, c] = [m[0], m[1], m[2]];
    let det = a[0] * (b[1] * c[2] - b[2] * c[1]) - b[0] * (a[1] * c[2] - a[2] * c[1])
        + c[0] * (a[1] * b[2] - a[2] * b[1]);
    det < 0.0
}
