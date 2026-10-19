//! `SeamSet`, cut-graph completion, darts, symmetry.

mod chart;
mod chart_mesh;
mod dart;
mod dijkstra;
mod error;
mod fold;
mod graph;
mod ids;
mod kind;
mod opening;
mod path;
mod paths;
mod proposal;
mod proposals;
mod set;
mod sewing_map;
mod topology;

pub use chart::{Chart, charts};
pub use chart_mesh::ChartMesh;
pub use dart::mark_dart;
pub use error::SeamsError;
pub use fold::check_fold;
pub use graph::{SeamChain, SeamGraph, SeamNode};
pub use ids::{ChartId, SeamChainId};
pub use kind::{ChartFailure, SeamKind};
pub use opening::mark_opening;
pub use path::shortest_path;
pub use proposal::{Proposal, ProposalId, ProposalKind, SeamReason};
pub use proposals::close_chart;
pub use set::SeamSet;
pub use sewing_map::{ChainSide, SeamBundle, SewingMap};
pub use topology::check_disk;
