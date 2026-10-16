use openplush_pipeline::Proposal;
use serde::Serialize;

/// A seam edit the maker can preview and accept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ProposalDto {
    /// 16 lowercase hex digits; what `applyProposal` takes back.
    id: String,
    /// The edges it makes seams, ascending.
    edges: Vec<u32>,
    /// Plain language: what it fixes and what it costs.
    explanation: String,
}

impl From<&Proposal> for ProposalDto {
    fn from(p: &Proposal) -> Self {
        Self {
            id: p.id().to_hex(),
            edges: p.add().keys().map(|e| e.index()).collect(),
            explanation: p.explanation().to_owned(),
        }
    }
}
