use openplush_pipeline::{EngineError, ErrorLocations};
use serde::Serialize;

use super::locations::Locations;
use super::proposal::ProposalDto;

/// An error as the shell receives it: a stable code, plain text, and where.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EngineErrorDto {
    /// An `EngineError` variant name or a [`super::BindingCode`].
    code: String,
    /// Shown to the maker as is.
    message: String,
    /// Absent when the error names no element.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    locations: Option<Locations>,
    /// Absent when the error offers no fix.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    proposals: Option<Vec<ProposalDto>>,
}

impl From<&EngineError> for EngineErrorDto {
    fn from(e: &EngineError) -> Self {
        let located = e.locations();
        let locations = (located != ErrorLocations::default()).then(|| Locations::from(&located));
        let proposals = e.proposals();
        Self {
            code: e.code().to_owned(),
            message: e.to_string(),
            locations,
            proposals: (!proposals.is_empty())
                .then(|| proposals.iter().map(ProposalDto::from).collect()),
        }
    }
}
