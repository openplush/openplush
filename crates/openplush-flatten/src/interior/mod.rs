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

use self::energy::{Rest, energy, rest_shapes};
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
pub(crate) struct Solve<'a> {
    chart: ChartId,
    faces: Faces<'a>,
    rest: Vec<Rest>,
    uv: Vec<DVec2>,
    energy: f64,
    system: Option<System>,
    iterations: u32,
    settled: bool,
}

impl<'a> Solve<'a> {
    /// Prescribes the boundary and fills the interior from the first start that flips no face.
    pub(crate) fn start(
        chart: ChartId,
        xyz: &[DVec3],
        faces: Faces<'a>,
    ) -> Result<Self, FlattenError> {
        let ring = boundary::prescribe(xyz, faces).map_err(|e| refuse(chart, e))?;
        let rest =
            rest_shapes(xyz, faces).map_err(|face| FlattenError::Degenerate { chart, face })?;
        let pinned: Vec<VertexId> = ring.iter().map(|&(v, _)| v).collect();
        let free = Free::new(xyz.len(), faces, &pinned);
        let mut uv = vec![DVec2::ZERO; xyz.len()];
        for &(v, p) in &ring {
            // INVARIANT: `prescribe` accepted the faces, so every ring vertex is in `xyz`.
            uv[v.index() as usize] = p;
        }
        let cotangent = boundary::cotangent_weights(xyz, faces).map_err(|e| refuse(chart, e))?;
        let uv = start::fill(chart, faces, &free, &cotangent, &uv)?;
        let energy = energy(&rest, faces, &uv).ok_or(FlattenError::SolveFailed { chart })?;
        let system = (free.count > 0)
            .then(|| System::new(faces, &free))
            .transpose()
            .map_err(|e| solve_failed(chart, e))?;
        Ok(Self {
            chart,
            faces,
            rest,
            uv,
            energy,
            settled: system.is_none(),
            system,
            iterations: 0,
        })
    }

    /// One SLIM iteration: a solve, then a line search that never flips a face.
    pub(crate) fn iterate(&mut self) -> Result<Step, FlattenError> {
        let Some(system) = self.system.as_ref().filter(|_| !self.settled) else {
            return Ok(Step::Settled);
        };
        self.iterations += 1;
        let target = system
            .target(&self.rest, self.faces, &self.uv)
            .map_err(|e| solve_failed(self.chart, e))?;
        let dir: Vec<DVec2> = target.iter().zip(&self.uv).map(|(t, u)| *t - *u).collect();
        let gain = descend(&self.rest, self.faces, &self.uv, &dir, self.energy).map(|(uv, e)| {
            let gain = (self.energy - e) / self.energy;
            (self.uv, self.energy) = (uv, e);
            gain
        });
        self.settled = self.iterations >= MAX_ITERATIONS || gain.is_none_or(|g| g < SETTLED_BELOW);
        Ok(if self.settled {
            Step::Settled
        } else {
            Step::Improved
        })
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

/// The chart's layout once the iteration settles.
pub(crate) fn flatten(
    chart: ChartId,
    xyz: &[DVec3],
    faces: Faces<'_>,
) -> Result<Vec<(VertexId, DVec2)>, FlattenError> {
    let mut solve = Solve::start(chart, xyz, faces)?;
    while solve.iterate()? == Step::Improved {}
    Ok(solve.layout())
}

/// The `FlattenError` for a boundary refusal. A face naming a missing vertex is reported as
/// unmeasurable, since it has no rest shape.
fn refuse(chart: ChartId, error: BoundaryError) -> FlattenError {
    match error {
        BoundaryError::Empty => FlattenError::EmptyChart { chart },
        BoundaryError::BadVertex { face } | BoundaryError::Degenerate { face } => {
            FlattenError::Degenerate { chart, face }
        }
        BoundaryError::NotADisk => FlattenError::NotADisk { chart },
        BoundaryError::Solve(e) => solve_failed(chart, e),
        BoundaryError::BadInput | BoundaryError::NotClosed => {
            FlattenError::BoundaryNotRealizable { chart }
        }
    }
}

fn solve_failed(chart: ChartId, _refusal: SolveError) -> FlattenError {
    FlattenError::SolveFailed { chart }
}

#[cfg(test)]
mod tests;
