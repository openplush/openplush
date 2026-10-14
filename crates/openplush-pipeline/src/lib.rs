//! The only public engine API: staged and memoized.

#![warn(missing_docs)]

mod error;
mod warning;

pub use error::EngineError;
pub use openplush_io::Format;
pub use openplush_mesh::{ComponentId, EdgeId, FaceId, SourceCorner, VertexId};
pub use openplush_pattern::PieceId;
pub use openplush_seams::{
    ChartFailure, ChartId, Proposal, ProposalId, ProposalKind, SeamChainId, SeamKind, SeamReason,
};
pub use warning::{NonBlank, Remedy, Warning, WarningCode, WarningSet, WarningTarget};
