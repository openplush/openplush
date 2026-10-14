//! `SeamSet`, cut-graph completion, darts, symmetry.
//!
//! So far this crate holds the types of data-model section 13a that seams own
//! (T175): chart and chain IDs, `SeamKind`, `ChartFailure`, and the proposal
//! types every engine-performable remedy uses. The algorithms come with T069
//! onward. Responsibilities and the dependency DAG are in
//! `specs/001-pattern-pipeline/plan.md` and `crates/AGENTS.md`.

mod error;
mod ids;
mod kind;
mod proposal;

pub use error::SeamsError;
pub use ids::{ChartId, SeamChainId};
pub use kind::{ChartFailure, SeamKind};
pub use proposal::{Proposal, ProposalId, ProposalKind, SeamReason};
