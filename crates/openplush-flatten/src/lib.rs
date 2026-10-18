//! Charts, Tutte/LSCM initialization, SLIM/ARAP, distortion metrics.
//!
//! What each crate holds today is in `docs/reference/crates.md`; what it will own is in
//! `crates/AGENTS.md`.

mod error;
pub mod injectivity;
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "the interior solve is the first caller")
)]
mod solver;

pub use error::FlattenError;
