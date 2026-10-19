//! Charts, Tutte/LSCM initialization, SLIM/ARAP, distortion metrics.
//!
//! What each crate holds today is in `docs/reference/crates.md`; what it will own is in
//! `crates/AGENTS.md`.

mod boundary;
mod error;
mod fallback;
pub mod injectivity;
mod interior;
mod session;
mod solver;

pub use error::FlattenError;
pub use session::{FlattenSession, Progress};
