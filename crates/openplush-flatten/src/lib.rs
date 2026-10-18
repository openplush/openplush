//! Charts, Tutte/LSCM initialization, SLIM/ARAP, distortion metrics.
//!
//! What each crate holds today is in `docs/reference/crates.md`; what it will own is in
//! `crates/AGENTS.md`.

mod boundary;
mod error;
pub mod injectivity;
#[cfg_attr(not(test), expect(dead_code, reason = "no caller outside tests"))]
mod interior;
mod solver;

pub use error::FlattenError;
