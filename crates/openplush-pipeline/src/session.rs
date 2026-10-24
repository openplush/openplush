mod memo;
mod proposals;

use std::collections::BTreeMap;

use openplush_flatten::{FlattenError, FlattenSession, StoppedBy};
use openplush_geom::DVec2;
use openplush_mesh::ManifoldMesh;
use openplush_seams::{ChartId, ChartMesh, SeamSet, SewingMap};

use self::memo::{Memo, key};
use crate::flatten::{refusal, settled, staged};
use crate::{EngineError, Parameterization, StageResult, chart};

/// What a [`Pipeline::flatten_step`] left behind.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum Progress {
    /// `chart`'s flatten took a step and may take more.
    Working {
        /// The chart being laid flat.
        chart: ChartId,
        /// Its steps so far.
        iteration: u32,
        /// Its area-weighted mean symmetric Dirichlet energy now (4 is an isometry).
        energy: f64,
    },
    /// `chart` is laid flat, and another chart still has no layout.
    ChartDone {
        /// The chart just laid flat.
        chart: ChartId,
    },
    /// Every chart has a layout.
    Done,
}

/// One repaired mesh, its current seams and the charts they make, and each chart's layout,
/// flattened a step at a time and remembered by the chart's content, so a seam edit flattens
/// again only the charts it changed.
pub struct Pipeline {
    mesh: ManifoldMesh,
    seams: SeamSet,
    charts: StageResult<(Vec<ChartMesh>, SewingMap)>,
    /// Each chart's memo key, in chart order; empty when the chart stage refused.
    keys: Vec<u64>,
    memo: Memo,
    stepping: Stepping,
}

/// Where the flatten of the current charts stands.
enum Stepping {
    /// No chart is part-way through.
    Idle,
    /// The chart at `at`, the lowest without a layout, is part-way through.
    Flattening {
        at: usize,
        session: Box<FlattenSession>,
    },
    /// A chart was refused; nothing more is tried until the seams change.
    Refused(EngineError),
}

impl Pipeline {
    /// A session over `mesh` with no seams.
    #[must_use]
    pub fn new(mesh: ManifoldMesh) -> Self {
        let seams = SeamSet::new();
        let charts = chart(&mesh, &seams);
        let keys = pieces(&charts).iter().map(key).collect();
        Self {
            mesh,
            seams,
            charts,
            keys,
            memo: Memo::default(),
            stepping: Stepping::Idle,
        }
    }

    /// Replaces the seams and cuts the charts again, and returns, in ID order, each chart whose
    /// content no chart of the previous seams had. A chart the edit left as it was keeps its
    /// layout; a layout for a chart the seams no longer make is dropped, as is any flatten in
    /// progress and any refusal kept.
    pub fn set_seams(&mut self, seams: &SeamSet) -> Vec<ChartId> {
        let charts = chart(&self.mesh, seams);
        let keys: Vec<u64> = pieces(&charts).iter().map(key).collect();
        let fresh = fresh(pieces(&self.charts), &self.keys, pieces(&charts), &keys);
        self.memo.retain(pieces(&charts), &keys);
        (self.charts, self.keys, self.stepping) = (charts, keys, Stepping::Idle);
        self.seams = seams.clone();
        fresh
    }

    /// The chart stage's result for the current seams: the charts and their sewing map, or its
    /// refusal.
    pub fn charts(&self) -> &StageResult<(Vec<ChartMesh>, SewingMap)> {
        &self.charts
    }

    /// Takes one flatten step on the lowest-ID chart without a layout, starting its flatten if
    /// none is in progress; `Done` once every chart has a layout.
    ///
    /// # Errors
    /// The chart stage's refusal, when it refused the seams; otherwise the chart's refusal,
    /// mapped as [`flatten`](crate::flatten()) maps it (`Flip`, `BoundaryNotRealizable` or
    /// `Flatten`), which every later step returns too, trying nothing, until the next
    /// [`Pipeline::set_seams`].
    pub fn flatten_step(&mut self) -> Result<Progress, EngineError> {
        let charts = &self.charts.as_ref().map_err(Clone::clone)?.value.0;
        if let Stepping::Refused(kept) = &self.stepping {
            return Err(kept.clone());
        }
        let Some((at, chart, key)) = unlaid(&self.memo, charts, &self.keys) else {
            return Ok(Progress::Done);
        };
        let held = std::mem::replace(&mut self.stepping, Stepping::Idle);
        let session = match held {
            // INVARIANT: only this method starts a session, always for the lowest chart without
            // a layout, and `set_seams` drops it, so a held session is this chart's.
            Stepping::Flattening { at: was, session } if was == at => Ok(session),
            _ => FlattenSession::new(chart.chart(), chart.xyz(), chart.faces().to_vec())
                .map(Box::new),
        };
        match session.and_then(|s| advance(s, chart.chart())) {
            Ok(Advance::Working {
                session,
                iteration,
                energy,
            }) => {
                self.stepping = Stepping::Flattening { at, session };
                Ok(Progress::Working {
                    chart: chart.chart(),
                    iteration,
                    energy,
                })
            }
            Ok(Advance::Laid(uv, stopped)) => {
                self.memo.insert(key, chart.clone(), uv, stopped);
                Ok(match unlaid(&self.memo, charts, &self.keys) {
                    Some(_) => Progress::ChartDone {
                        chart: chart.chart(),
                    },
                    None => Progress::Done,
                })
            }
            Err(e) => {
                let refused = refusal(&self.mesh, chart, e);
                self.stepping = Stepping::Refused(refused.clone());
                Err(refused)
            }
        }
    }

    /// Steps every chart without a layout to the end and returns every chart's layout, in
    /// chart order, as [`flatten`](crate::flatten()) would, with its warnings: a chart laid
    /// from the memo warns as it did when it was flattened. It raises no `piece-compressed`: the
    /// pattern stage raises that, with the fabric.
    ///
    /// # Errors
    /// The first refusal [`Pipeline::flatten_step`] returns.
    pub fn parameterizations(&mut self) -> StageResult<Vec<Parameterization>> {
        while self.flatten_step()? != Progress::Done {}
        let charts = &self.charts.as_ref().map_err(Clone::clone)?.value.0;
        let laid = charts
            .iter()
            .zip(&self.keys)
            .map(|(chart, &key)| {
                // INVARIANT: stepping reached `Done`, so every chart has a layout.
                let (uv, stopped) = self.memo.get(key, chart).ok_or(EngineError::Flatten(
                    FlattenError::SolveFailed {
                        chart: chart.chart(),
                    },
                ))?;
                Ok((uv.to_vec(), stopped))
            })
            .collect::<Result<_, EngineError>>()?;
        staged(charts, laid)
    }
}

/// What one flatten step did to a chart's session.
enum Advance {
    Working {
        session: Box<FlattenSession>,
        iteration: u32,
        energy: f64,
    },
    Laid(Vec<DVec2>, StoppedBy),
}

fn advance(mut session: Box<FlattenSession>, chart: ChartId) -> Result<Advance, FlattenError> {
    match session.step()? {
        openplush_flatten::Progress::Working { iteration, energy } => Ok(Advance::Working {
            session,
            iteration,
            energy,
        }),
        openplush_flatten::Progress::Done => {
            settled(*session, chart).map(|(uv, stopped)| Advance::Laid(uv, stopped))
        }
        // INVARIANT: a session reports only `Working` and `Done`; anything else is refused
        // rather than guessed at.
        _ => Err(FlattenError::SolveFailed { chart }),
    }
}

/// The charts of a chart stage's result; none when it refused.
fn pieces(charts: &StageResult<(Vec<ChartMesh>, SewingMap)>) -> &[ChartMesh] {
    charts.as_ref().map_or(&[], |staged| &staged.value.0)
}

/// The lowest-ID chart with no layout in `memo`, with its index and key.
fn unlaid<'c>(
    memo: &Memo,
    charts: &'c [ChartMesh],
    keys: &[u64],
) -> Option<(usize, &'c ChartMesh, u64)> {
    charts
        .iter()
        .zip(keys)
        .enumerate()
        .find(|(_, (chart, key))| memo.get(**key, chart).is_none())
        .map(|(at, (chart, &key))| (at, chart, key))
}

/// The IDs of the charts in `now` whose content no chart in `before` has.
fn fresh(
    before: &[ChartMesh],
    before_keys: &[u64],
    now: &[ChartMesh],
    keys: &[u64],
) -> Vec<ChartId> {
    let mut known: BTreeMap<u64, Vec<&ChartMesh>> = BTreeMap::new();
    for (chart, &key) in before.iter().zip(before_keys) {
        known.entry(key).or_default().push(chart);
    }
    now.iter()
        .zip(keys)
        .filter(|(chart, key)| !known.get(key).is_some_and(|old| old.contains(chart)))
        .map(|(chart, _)| chart.chart())
        .collect()
}

#[cfg(test)]
mod tests;
