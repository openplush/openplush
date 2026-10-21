//! Interior flattening: the boundary held where `boundary::prescribe` put it, the interior
//! moved to lower the symmetric Dirichlet energy without ever flipping a face.

mod energy;
mod free;
mod proxy;
mod slim;
mod start;

use std::collections::BTreeSet;

use openplush_geom::{DVec2, DVec3};
use openplush_mesh::{FaceId, VertexId};
use openplush_seams::ChartId;

pub(crate) use self::energy::{Rest, rest_shapes};

use self::energy::energy;
use self::free::Free;
use self::slim::{System, descend};
use crate::FlattenError;
use crate::boundary::{self, BoundaryError};
use crate::solver::SolveError;

/// Iterations before the solve settles with the layout it has; every one keeps all faces
/// unflipped, so reaching the cap is a valid, if less relaxed, layout.
const MAX_ITERATIONS: u32 = 200;
/// A step that lowers the energy by less than this fraction of it settles the solve.
const SETTLED_BELOW: f64 = 1e-9;

type Faces<'a> = &'a [(FaceId, [VertexId; 3])];

/// Whether another iteration can still help.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Step {
    /// The energy fell by more than the settling fraction.
    Improved,
    /// No further iteration will change the layout.
    Settled,
}

/// One chart's SLIM solve, advanced an iteration at a time.
pub(crate) struct Solve {
    chart: ChartId,
    faces: Vec<(FaceId, [VertexId; 3])>,
    rest: Vec<Rest>,
    uv: Vec<DVec2>,
    energy: f64,
    system: Option<System>,
    iterations: u32,
    /// Iterations allowed in all; a field so tests can reach the cap.
    cap: u32,
    settled: bool,
}

impl Solve {
    /// Prescribes the boundary and fills the interior from the first start that flips no face.
    pub(crate) fn start(
        chart: ChartId,
        xyz: &[DVec3],
        chart_faces: Vec<(FaceId, [VertexId; 3])>,
    ) -> Result<Self, FlattenError> {
        let faces = &chart_faces[..];
        let ring = boundary::prescribe(xyz, faces).map_err(|e| refuse(chart, xyz, faces, e))?;
        let rest =
            rest_shapes(xyz, faces).map_err(|face| FlattenError::Degenerate { chart, face })?;
        let pinned: Vec<VertexId> = ring.iter().map(|&(v, _)| v).collect();
        let free = Free::new(xyz.len(), faces, &pinned);
        let mut uv = vec![DVec2::ZERO; xyz.len()];
        for &(v, p) in &ring {
            // INVARIANT: `prescribe` accepted the faces, so every ring vertex is in `xyz`.
            uv[v.index() as usize] = p;
        }
        let cotangent =
            boundary::cotangent_weights(xyz, faces).map_err(|e| refuse(chart, xyz, faces, e))?;
        let uv = start::fill(chart, faces, &free, &cotangent, &uv)?;
        let energy = energy(&rest, faces, &uv).ok_or(FlattenError::SolveFailed { chart })?;
        let system = (free.count > 0)
            .then(|| System::new(faces, &free))
            .transpose()
            .map_err(|e| solve_failed(chart, e))?;
        Ok(Self {
            chart,
            faces: chart_faces,
            rest,
            uv,
            energy,
            settled: system.is_none(),
            system,
            iterations: 0,
            cap: MAX_ITERATIONS,
        })
    }

    /// One SLIM iteration: a solve, then a line search that never flips a face.
    pub(crate) fn iterate(&mut self) -> Result<Step, FlattenError> {
        let Some(system) = self.system.as_ref().filter(|_| !self.settled) else {
            return Ok(Step::Settled);
        };
        self.iterations += 1;
        let target = system
            .target(&self.rest, &self.faces, &self.uv)
            .map_err(|e| solve_failed(self.chart, e))?;
        let dir: Vec<DVec2> = target.iter().zip(&self.uv).map(|(t, u)| *t - *u).collect();
        let gain = descend(&self.rest, &self.faces, &self.uv, &dir, self.energy).map(|(uv, e)| {
            let gain = (self.energy - e) / self.energy;
            (self.uv, self.energy) = (uv, e);
            gain
        });
        self.settled = self.iterations >= self.cap || gain.is_none_or(|g| g < SETTLED_BELOW);
        Ok(if self.settled {
            Step::Settled
        } else {
            Step::Improved
        })
    }

    /// Iterations taken so far.
    pub(crate) fn iterations(&self) -> u32 {
        self.iterations
    }

    /// The area-weighted mean symmetric Dirichlet energy of the current layout.
    pub(crate) fn energy(&self) -> f64 {
        self.energy
    }

    /// Every vertex of the chart's faces with its position, in vertex order.
    pub(crate) fn layout(&self) -> Vec<(VertexId, DVec2)> {
        let used: BTreeSet<u32> = self
            .faces
            .iter()
            .flat_map(|(_, t)| t.map(VertexId::index))
            .collect();
        // INVARIANT: `start` checked that every face vertex has a position, and `uv` has one
        // entry per position.
        used.into_iter()
            .map(|v| (VertexId::new(v), self.uv[v as usize]))
            .collect()
    }
}

/// The `FlattenError` for a boundary refusal. A face naming a missing vertex is reported as
/// unmeasurable, since it has no rest shape; a boundary that will not close is measured.
pub(crate) fn refuse(
    chart: ChartId,
    xyz: &[DVec3],
    faces: Faces<'_>,
    error: BoundaryError,
) -> FlattenError {
    match error {
        BoundaryError::BadInput | BoundaryError::NotClosed => {
            crate::fallback::not_realizable(chart, xyz, faces)
        }
        other => refuse_measurement(chart, other),
    }
}

/// The refusals that need nothing but the chart.
pub(crate) fn refuse_measurement(chart: ChartId, error: BoundaryError) -> FlattenError {
    match error {
        BoundaryError::Empty => FlattenError::EmptyChart { chart },
        BoundaryError::BadVertex { face } | BoundaryError::Degenerate { face } => {
            FlattenError::Degenerate { chart, face }
        }
        BoundaryError::NotADisk => FlattenError::NotADisk { chart },
        BoundaryError::Solve(e) => solve_failed(chart, e),
        // INVARIANT: `open_polygon` never closes anything, so it never returns these.
        BoundaryError::BadInput | BoundaryError::NotClosed => FlattenError::SolveFailed { chart },
    }
}

fn solve_failed(chart: ChartId, _refusal: SolveError) -> FlattenError {
    FlattenError::SolveFailed { chart }
}

#[cfg(test)]
mod tests;
