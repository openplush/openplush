use std::f64::consts::TAU;

use openplush_geom::DVec3;
use openplush_mesh::{FaceId, VertexId};

use super::intrinsic;
use crate::boundary::BoundaryError;

#[test]
fn cotangent_weights_reproduce_linear_functions_on_a_flat_fan() -> Result<(), BoundaryError> {
    // On a flat mesh the cotangent Laplacian of any linear function is zero at an interior
    // vertex: sum over neighbours j of w_cj (p_c - p_j) = 0. A weight read from the wrong
    // corner breaks that on an irregular fan.
    let ring = [
        (1.0, 0.1),
        (0.4, 0.9),
        (-0.7, 1.1),
        (-1.2, -0.2),
        (-0.3, -0.8),
        (0.9, -0.9),
    ];
    let (u, v) = (DVec3::new(0.6, 0.0, 0.8), DVec3::new(0.0, 1.0, 0.0));
    let at = |(x, y): (f64, f64)| u * x + v * y;
    let xyz: Vec<DVec3> = std::iter::once(at((0.1, 0.05)))
        .chain(ring.map(at))
        .collect();
    let faces: Vec<(FaceId, [VertexId; 3])> = (0..6)
        .map(|i| {
            (
                FaceId::new(i),
                [0, 1 + i, 1 + (i + 1) % 6].map(VertexId::new),
            )
        })
        .collect();
    let geometry = intrinsic(&xyz, &faces)?;
    assert!((geometry.angle_sum[&0] - TAU).abs() <= 1e-12);
    let pull = (1..=6).fold(DVec3::ZERO, |sum, j| {
        sum + (xyz[0] - xyz[j as usize]) * geometry.weight[&(0, j)]
    });
    assert!(pull.length() <= 1e-12, "{pull:?}");
    Ok(())
}
