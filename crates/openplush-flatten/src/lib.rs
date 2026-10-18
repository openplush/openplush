//! Charts, Tutte/LSCM initialization, SLIM/ARAP, distortion metrics.
//!
//! Only the post-flatten injectivity check exists so far. What each crate holds
//! today is in `docs/reference/crates.md`; what it will own is in `crates/AGENTS.md`.

mod error;
pub mod injectivity;

pub use error::FlattenError;
