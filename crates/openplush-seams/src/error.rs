use thiserror::Error;

/// Every way this crate refuses its input.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SeamsError {
    /// A proposal must add at least one seam edge (data-model section 13a).
    #[error("a proposal must add at least one seam edge")]
    EmptyProposal,
    /// Text that is not a proposal ID: exactly 16 lowercase hex digits.
    #[error("{text:?} is not a proposal id (16 lowercase hex digits)")]
    BadProposalId {
        /// The rejected text.
        text: String,
    },
}
