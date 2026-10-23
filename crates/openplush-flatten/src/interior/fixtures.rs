//! Solve states other modules' tests need and cannot reach from a chart.

use openplush_geom::DVec2;
use openplush_mesh::VertexId;

use super::Solve;

/// Puts vertex `v` at `to`, bypassing the line search.
pub(crate) fn place(solve: &mut Solve, v: VertexId, to: DVec2) {
    // INVARIANT: tests pass a vertex of the chart, which has a position.
    solve.uv[v.index() as usize] = to;
}
