use openplush_mesh::EdgeId;
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
}
