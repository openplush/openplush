//! The only public engine API: staged and memoized.

#![warn(missing_docs)]

mod chart;
mod error;
mod stage;
mod warning;

pub use chart::chart;
pub use error::{EngineError, ErrorLocations};
pub use openplush_io::Format;
pub use openplush_mesh::{ComponentId, EdgeId, FaceId, ManifoldMesh, SourceCorner, VertexId};
pub use openplush_pattern::PieceId;
/// The error [`ProposalId::from_hex`] and [`Proposal::new`] return.
pub use openplush_seams::SeamsError;
pub use openplush_seams::{
    ChainSide, ChartFailure, ChartId, ChartMesh, Proposal, ProposalId, ProposalKind, SeamBundle,
    SeamChainId, SeamKind, SeamReason, SeamSet, SewingMap,
};
pub use stage::{StageResult, Staged};
pub use warning::{NonBlank, Remedy, Warning, WarningCode, WarningSet, WarningTarget};
