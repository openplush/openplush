use openplush_io::Format;
use openplush_mesh::{EdgeId, FaceId, SourceCorner, VertexId};
use openplush_pattern::PieceId;
use openplush_seams::{ChartFailure, ChartId, Proposal, ProposalId, SeamChainId};
use thiserror::Error;

/// A violated invariant: the stage produced no output. Every variant but
/// `Cancelled` names where the problem is, so the UI can highlight it.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum EngineError {
    /// The importer for `format` could not read the file.
    #[error("could not read the {} file: {detail}", .format.name())]
    Parse {
        /// The reader that refused the file.
        format: Format,
        /// What it refused, in the reader's words.
        detail: String,
    },
    /// Source vertices with a NaN or infinite coordinate; import refuses rather than drops them.
    #[error("{} source vertices have a NaN or infinite coordinate", .vertices.len())]
    NonFiniteInput {
        /// Every non-finite source vertex, used by a face or not.
        vertices: Vec<SourceCorner>,
    },
    /// The mesh is not manifold where repair needed it to be.
    #[error(
        "the mesh is not manifold at {} edges and {} vertices",
        .edges.len(),
        .vertices.len()
    )]
    NotManifold {
        /// Non-manifold edges.
        edges: Vec<EdgeId>,
        /// Non-manifold vertices.
        vertices: Vec<VertexId>,
    },
    /// A chart is not a topological disk, so it cannot flatten as one piece.
    #[error("chart {} is not a topological disk", .chart.index())]
    ChartNotDisk {
        /// The chart.
        chart: ChartId,
        /// Why it is not a disk.
        failure: ChartFailure,
        /// The minimal extra seams that would make it one.
        proposals: Vec<Proposal>,
    },
    /// Flattening inverted faces (det J ≤ 0).
    #[error("chart {} flattens with {} inverted faces", .chart.index(), .faces.len())]
    Flip {
        /// The chart.
        chart: ChartId,
        /// The inverted faces.
        faces: Vec<FaceId>,
    },
    /// The prescribed seam lengths cannot close into a flat outline.
    #[error(
        "chart {}'s seam lengths cannot close flat (residual {residual_mm:.3} mm)",
        .chart.index()
    )]
    BoundaryNotRealizable {
        /// The chart.
        chart: ChartId,
        /// How far the outline fails to close, in millimetres.
        residual_mm: f64,
        /// The darts that would let it close.
        proposals: Vec<Proposal>,
    },
    /// The two sides of a seam chain differ by more than the tolerance; a flattener bug.
    #[error(
        "seam chain {}: sides {a_mm:.3} mm and {b_mm:.3} mm differ by more than {tolerance_mm:.3} mm",
        .chain.index()
    )]
    SeamParity {
        /// The chain.
        chain: SeamChainId,
        /// One side's length, in millimetres.
        a_mm: f64,
        /// The other side's length, in millimetres.
        b_mm: f64,
        /// The largest allowed difference, in millimetres.
        tolerance_mm: f64,
    },
    /// The seam allowance could not be offset from the piece outline.
    #[error("seam allowance for piece {} failed: {detail}", .piece.index())]
    OffsetFailed {
        /// The piece.
        piece: PieceId,
        /// Why the offset failed.
        detail: String,
    },
    /// A proposal was applied that the current state does not offer; nothing was applied.
    #[error("proposal {} is no longer offered", .id.to_hex())]
    ProposalNotOffered {
        /// The proposal asked for.
        id: ProposalId,
    },
    /// The caller cancelled a session; no partial output exists.
    #[error("cancelled")]
    Cancelled,
}
