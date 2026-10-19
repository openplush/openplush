use openplush_mesh::{EdgeId, VertexId};

use crate::{ChartId, SeamChainId};
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
    /// A seam chain along mesh-boundary edges with no record of which boundary copy it pairs
    /// with; it has one side and is never paired with itself.
    #[error("seam chain {chain:?} runs along the mesh boundary with no recorded partner")]
    UnpairedBoundarySeam {
        /// The chain.
        chain: SeamChainId,
    },
    /// A picked vertex the mesh does not have.
    #[error("vertex {vertex:?} is not one of the mesh's")]
    UnknownVertex {
        /// The vertex.
        vertex: VertexId,
    },
    /// No edge walk joins the two picks: they are on separate pieces of the mesh.
    #[error("no path of edges joins vertex {from:?} to vertex {to:?}")]
    NoPath {
        /// The first pick.
        from: VertexId,
        /// The second pick.
        to: VertexId,
    },
    /// A dart with no edges.
    #[error("a dart needs at least one edge")]
    EmptyDart,
    /// An edge that does not share a vertex with the edge before it, or returns to a vertex the
    /// walk already has.
    #[error("edge {edge:?} does not continue the walk")]
    NotAWalk {
        /// The edge.
        edge: EdgeId,
    },
    /// Neither end of the walk is on a seam or the mesh boundary, so it is not a dart.
    #[error("a dart starts on a seam or the mesh boundary; neither end does (vertex {vertex:?})")]
    DartNotAnchored {
        /// The walk's first vertex.
        vertex: VertexId,
    },
    /// A vertex past the base lies on a seam or the mesh boundary, so the dart would split the
    /// piece or cross a seam.
    #[error("a dart's tip and inner vertices touch no seam or boundary; vertex {vertex:?} does")]
    DartTouchesSeam {
        /// The first such vertex along the walk.
        vertex: VertexId,
    },
}
