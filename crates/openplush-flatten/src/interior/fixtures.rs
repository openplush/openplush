//! Solve states other modules' tests need and cannot reach from a chart.

use std::f64::consts::PI;

use openplush_geom::{DVec2, DVec3};
use openplush_mesh::VertexId;

use super::Solve;
use crate::boundary::fixtures::{Faces, grid};

/// Puts vertex `v` at `to`, bypassing the line search.
pub(crate) fn place(solve: &mut Solve, v: VertexId, to: DVec2) {
    // INVARIANT: tests pass a vertex of the chart, which has a position.
    solve.uv[v.index() as usize] = to;
}

/// Lowers the iteration cap to `at`, so a test reaches it in a few iterations.
pub(crate) fn cap(solve: &mut Solve, at: u32) {
    solve.cap = at;
}

/// Half of a unit cylinder, 0.4 per ring high: developable, so its fill is the isometry.
pub(crate) fn half_cylinder() -> (Vec<DVec3>, Faces) {
    let step = PI / 8.0;
    let xyz = (0..=3)
        .flat_map(|r| (0..=8).map(move |c| (c, r)))
        .map(|(c, r)| {
            let a = f64::from(c) * step;
            DVec3::new(libm::cos(a), libm::sin(a), 0.4 * f64::from(r))
        })
        .collect();
    (xyz, grid(8, 3))
}
