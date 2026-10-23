use std::collections::BTreeSet;

use openplush_seams::{ChartId, Proposal, ProposalId, SeamSet};

use super::{Pipeline, Stepping};
use crate::{EngineError, Remedy};

impl Pipeline {
    /// Every proposal the current state offers, each ID once: the cuts the chart stage's
    /// `ChartNotDisk` carries, the darts of a kept `BoundaryNotRealizable`, then each
    /// `Actionable` remedy of the current warnings, in that order.
    #[must_use]
    pub fn proposals(&self) -> Vec<Proposal> {
        let refusals = [
            self.charts.as_ref().err(),
            match &self.stepping {
                Stepping::Refused(kept) => Some(kept),
                _ => None,
            },
        ];
        let carried = refusals
            .into_iter()
            .flatten()
            .flat_map(EngineError::proposals);
        let remedies = self
            .charts
            .iter()
            .flat_map(|staged| staged.warnings.iter())
            .filter_map(|w| match w.remedy() {
                Remedy::Actionable { proposal } => Some(proposal),
                Remedy::Advisory { .. } => None,
            });
        let mut seen = BTreeSet::new();
        carried
            .chain(remedies)
            .filter(|p| seen.insert(p.id()))
            .cloned()
            .collect()
    }

    /// The current seams with the offered proposal `id`'s edges added; an edge that already
    /// has a seam keeps its kind, since proposals only add seams. The session is unchanged.
    ///
    /// # Errors
    /// `ProposalNotOffered` when [`Pipeline::proposals`] does not offer `id`.
    pub fn preview(&self, id: ProposalId) -> Result<SeamSet, EngineError> {
        let offered = self.proposals();
        let proposal = offered
            .iter()
            .find(|p| p.id() == id)
            .ok_or(EngineError::ProposalNotOffered { id })?;
        let mut seams = self.seams.clone();
        for (&edge, &kind) in proposal.add() {
            if seams.get(edge).is_none() {
                seams.insert(edge, kind)?;
            }
        }
        Ok(seams)
    }

    /// Sets [`Pipeline::preview`]'s seams for `id`, and returns what
    /// [`Pipeline::set_seams`] returns for them.
    ///
    /// # Errors
    /// `ProposalNotOffered` when `id` is not offered; nothing is changed.
    pub fn apply_proposal(&mut self, id: ProposalId) -> Result<Vec<ChartId>, EngineError> {
        let seams = self.preview(id)?;
        Ok(self.set_seams(&seams))
    }
}
