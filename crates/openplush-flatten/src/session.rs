//! A chart's flatten as a series of bounded steps, so a single-threaded worker can yield
//! between them, report progress and give up (by dropping the session) without ever holding a
//! half-flattened layout.

use openplush_geom::{DVec2, DVec3};
use openplush_mesh::{FaceId, VertexId};
use openplush_seams::ChartId;

use crate::FlattenError;
use crate::interior::{Solve, Step};

/// What a step left behind.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum Progress {
    /// Another step may lower the energy.
    Working {
        /// Steps taken so far.
        iteration: u32,
        /// The area-weighted mean symmetric Dirichlet energy now (4 is an isometry).
        energy: f64,
    },
    /// The layout is settled and can be taken with [`FlattenSession::finish`].
    Done,
}

/// Why a chart's flatten stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum StoppedBy {
    /// It reached the 200-iteration cap; when the last iteration also gained too little to
    /// go on, it is still the cap. The layout is valid but may be less relaxed than it could be.
    IterationCap,
    /// An iteration lowered the energy by less than 1e-9 of its value.
    Settled,
    /// No step tried lowered the energy, or the chart has no interior vertex to move.
    NoDescent,
}

/// One chart's flatten in progress, owning the chart's faces so a caller can hold it between
/// steps.
pub struct FlattenSession {
    solve: Solve,
    done: bool,
}

impl FlattenSession {
    /// Prescribes the boundary and fills the interior; the first `step` iterates from there.
    /// `faces` index `xyz`.
    pub fn new(
        chart: ChartId,
        xyz: &[DVec3],
        faces: Vec<(FaceId, [VertexId; 3])>,
    ) -> Result<Self, FlattenError> {
        Ok(Self {
            solve: Solve::start(chart, xyz, faces)?,
            done: false,
        })
    }

    /// One SLIM iteration. After `Done`, every further step is `Done` and changes nothing.
    pub fn step(&mut self) -> Result<Progress, FlattenError> {
        if !self.done {
            self.done = self.solve.iterate()? == Step::Settled;
        }
        Ok(if self.done {
            Progress::Done
        } else {
            Progress::Working {
                iteration: self.solve.iterations(),
                energy: self.solve.energy(),
            }
        })
    }

    /// Every vertex of the chart with its flat position, in vertex order, and why the flatten
    /// stopped; `None` until a step has returned `Done`, so a half-flattened layout is never
    /// handed out.
    pub fn finish(self) -> Option<(Vec<(VertexId, DVec2)>, StoppedBy)> {
        let stopped = self.solve.stopped_by().filter(|_| self.done)?;
        Some((self.solve.layout(), stopped))
    }
}

#[cfg(test)]
mod tests;
