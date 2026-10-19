use openplush_mesh::EdgeId;

use crate::ChartId;
use thiserror::Error;

/// Every way `openplush-seams` refuses its input.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum SeamsError {
    /// A proposal must add at least one seam edge.
    #[error("a proposal must add at least one seam edge")]
    EmptyProposal,
    /// A mandatory seam can be neither removed nor replaced, and none is placed over
    /// another seam.
    #[error(
        "a mandatory seam cannot be removed, replaced, or placed over another seam (edge {edge:?})"
    )]
    MandatorySeam {
        /// The edge.
        edge: EdgeId,
    },
    /// A seam on an edge the mesh does not have.
    #[error("a seam is on edge {edge:?}, which the mesh does not have")]
    UnknownEdge {
        /// The edge.
        edge: EdgeId,
    },
    /// Text that is not a proposal ID: exactly 16 lowercase hex digits.
    #[error("{text:?} is not a proposal id (16 lowercase hex digits)")]
    BadProposalId {
        /// The rejected text.
        text: String,
    },
    /// The cuts found to close a chart do not leave it one disk, so none is offered.
    #[error("the cuts found for chart {chart:?} do not leave it one disk")]
    ChartNotClosed {
        /// The chart.
        chart: ChartId,
    },
    /// An edge to mark has no seam, as when its chain comes from an older seam graph.
    #[error("edge {edge:?} has no seam to mark")]
    NotASeam {
        /// The edge.
        edge: EdgeId,
    },
    /// Two faces of a chart lie back over each other across an edge no seam cuts.
    #[error("chart {chart:?} lies back over itself across edge {edge:?}")]
    FoldedChart {
        /// The chart.
        chart: ChartId,
        /// The lowest edge it folds across.
        edge: EdgeId,
    },
}
