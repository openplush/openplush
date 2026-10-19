//! `SeamSet`, cut-graph completion, darts, symmetry.

mod chart;
mod error;
mod graph;
mod ids;
mod kind;
mod proposal;
mod set;
mod topology;

pub use chart::{Chart, charts};
pub use error::SeamsError;
pub use graph::{SeamChain, SeamGraph, SeamNode};
pub use ids::{ChartId, SeamChainId};
pub use kind::{ChartFailure, SeamKind};
pub use proposal::{Proposal, ProposalId, ProposalKind, SeamReason};
pub use set::SeamSet;
pub use topology::check_disk;
