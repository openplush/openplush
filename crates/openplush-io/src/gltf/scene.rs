//! The drawn nodes of the scene a renderer would draw, each with its world transform.

use ::gltf::scene::Transform;
use ::gltf::{Document, Node};
use openplush_geom::DVec3;
use openplush_geom::epsilon::UNIT_QUATERNION_TOLERANCE;

/// A 4x4 transform in column-major order, `m[column][row]`, as glTF stores it.
pub(super) type Mat = [[f64; 4]; 4];

const IDENTITY: Mat = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

/// The file's `scene`, else scene 0, walked depth first from its roots, children in array
/// order; every node with its world transform.
pub(super) fn walk(doc: &Document) -> Result<Vec<(Node<'_>, Mat)>, String> {
    let scene = doc
        .default_scene()
        .or_else(|| doc.scenes().next())
        .ok_or("the file has no scene to draw")?;
    let mut seen = vec![false; doc.nodes().len()];
    let mut out = Vec::new();
    let mut stack: Vec<(Node<'_>, Mat)> = scene.nodes().map(|n| (n, IDENTITY)).collect();
    stack.reverse();
    while let Some((node, parent)) = stack.pop() {
        // INVARIANT: node indices are in range, as the crate's validation checked.
        if std::mem::replace(&mut seen[node.index()], true) {
            return Err(format!(
                "node {} is reached twice, so the node hierarchy is not a tree",
                node.index()
            ));
        }
        let world = mul(&parent, &local(&node)?);
        let first = stack.len();
        stack.extend(node.children().map(|c| (c, world)));
        stack[first..].reverse();
        out.push((node, world));
    }
    Ok(out)
}

/// The node's own transform. A `matrix` is read as written, since it may scale on purpose.
fn local(node: &Node<'_>) -> Result<Mat, String> {
    match node.transform() {
        Transform::Matrix { matrix } => Ok(matrix.map(|column| column.map(f64::from))),
        Transform::Decomposed {
            translation,
            rotation,
            scale,
        } => {
            let q = rotation.map(f64::from);
            let squared: f64 = q.iter().map(|c| c * c).sum();
            if (squared - 1.0).abs() > UNIT_QUATERNION_TOLERANCE {
                return Err(format!(
                    "node {}: the rotation {rotation:?} is not a unit quaternion (its squared length is {squared})",
                    node.index()
                ));
            }
            Ok(trs(translation.map(f64::from), q, scale.map(f64::from)))
        }
    }
}

/// Translation times rotation (a unit quaternion `[x, y, z, w]`) times scale.
fn trs(t: [f64; 3], [x, y, z, w]: [f64; 4], s: [f64; 3]) -> Mat {
    let r = [
        [
            1.0 - 2.0 * (y * y + z * z),
            2.0 * (x * y + z * w),
            2.0 * (x * z - y * w),
        ],
        [
            2.0 * (x * y - z * w),
            1.0 - 2.0 * (x * x + z * z),
            2.0 * (y * z + x * w),
        ],
        [
            2.0 * (x * z + y * w),
            2.0 * (y * z - x * w),
            1.0 - 2.0 * (x * x + y * y),
        ],
    ];
    let column = |c: usize| {
        let [a, b, d] = r[c].map(|v| v * s[c]);
        [a, b, d, 0.0]
    };
    [column(0), column(1), column(2), [t[0], t[1], t[2], 1.0]]
}

fn mul(a: &Mat, b: &Mat) -> Mat {
    let mut m = [[0.0; 4]; 4];
    for (column, out) in m.iter_mut().enumerate() {
        for (row, cell) in out.iter_mut().enumerate() {
            *cell = (0..4).map(|k| a[k][row] * b[column][k]).sum();
        }
    }
    m
}

/// `p` moved by `m`.
pub(super) fn apply(m: &Mat, [x, y, z]: [f64; 3]) -> DVec3 {
    let row = |r: usize| m[0][r] * x + m[1][r] * y + m[2][r] * z + m[3][r];
    DVec3::new(row(0), row(1), row(2))
}

/// True when `m` turns the space inside out, so a triangle's winding must be reversed.
pub(super) fn mirrors(m: &Mat) -> bool {
    let [a, b, c] = [m[0], m[1], m[2]];
    let det = a[0] * (b[1] * c[2] - b[2] * c[1]) - b[0] * (a[1] * c[2] - a[2] * c[1])
        + c[0] * (a[1] * b[2] - a[2] * b[1]);
    det < 0.0
}
