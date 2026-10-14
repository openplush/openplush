//! `SeamSet`, cut-graph completion, darts, symmetry.

mod error;
mod ids;
mod kind;
mod proposal;

pub use error::SeamsError;
pub use ids::{ChartId, SeamChainId};
pub use kind::{ChartFailure, SeamKind};
pub use proposal::{Proposal, ProposalId, ProposalKind, SeamReason};
