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

/// One chart's flatten in progress.
pub struct FlattenSession<'a> {
    solve: Solve<'a>,
    done: bool,
}

impl<'a> FlattenSession<'a> {
    /// Prescribes the boundary and fills the interior; the first `step` iterates from there.
    /// `faces` index `xyz`.
    pub fn new(
        chart: ChartId,
        xyz: &[DVec3],
        faces: &'a [(FaceId, [VertexId; 3])],
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

    /// Every vertex of the chart with its flat position, in vertex order; `None` until a step
    /// has returned `Done`, so a half-flattened layout is never handed out.
    pub fn finish(self) -> Option<Vec<(VertexId, DVec2)>> {
        self.done.then(|| self.solve.layout())
    }
}

#[cfg(test)]
mod tests;
