//! Charts, Tutte/LSCM initialization, SLIM/ARAP, distortion metrics.
//!
//! What this crate holds today is in `docs/reference/crates/openplush-flatten.md`; what it
//! will own is in `crates/AGENTS.md`.

mod boundary;
mod error;
mod fallback;
pub mod injectivity;
mod interior;
pub mod metrics;
mod session;
mod solver;
pub mod summary;

pub use error::FlattenError;
pub use session::{FlattenSession, Progress, StoppedBy};
