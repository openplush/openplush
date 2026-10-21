//! The only public engine API: staged and memoized.

#![warn(missing_docs)]

mod chart;
mod error;
mod flatten;
mod import;
mod locations;
mod pattern;
mod repair;
mod stage;
mod warning;

pub use chart::chart;
pub use error::{EngineError, ErrorLocations};
pub use flatten::{Parameterization, flatten};
pub use import::{ImportFile, ImportFiles, import};
pub use openplush_flatten::FlattenError;
pub use openplush_geom::DVec2;
pub use openplush_geom::units::{Mm, Scale};
pub use openplush_io::Format;
pub use openplush_mesh::{
    ComponentId, CornerArray, Counted, DuplicateOrientation, EdgeId, FaceId, HalfEdgeId, LoopRef,
    ManifoldMesh, MaterialFault, MeshElement, MeshError, RepairPlan, RepairReport, SourceCorner,
    TriangleSoup, Unmeasurable, ValidationReport, VertexId,
};
pub use openplush_pattern::{
    CutCount, Fabric, FabricCategory, FabricId, FabricSpec, Outline, PatternError, PatternPiece,
    PieceId, StretchDirection,
};
/// The error [`ProposalId::from_hex`] and [`Proposal::new`] return.
pub use openplush_seams::SeamsError;
pub use openplush_seams::{
    ChainSide, ChartFailure, ChartId, ChartMesh, Proposal, ProposalId, ProposalKind, SeamBundle,
    SeamChainId, SeamKind, SeamReason, SeamSet, SewingMap,
};
pub use pattern::{PatternInput, PatternOptions, pattern};
pub use repair::{repair, validate};
pub use stage::{StageResult, Staged};
pub use warning::{NonBlank, Remedy, Warning, WarningCode, WarningSet, WarningTarget};
