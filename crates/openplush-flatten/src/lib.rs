//! Charts, Tutte/LSCM initialization, SLIM/ARAP, distortion metrics.
//!
//! What each crate holds today is in `docs/reference/crates.md`; what it will own is in
//! `crates/AGENTS.md`.

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "boundary prescription is the first caller")
)]
mod boundary;
mod error;
pub mod injectivity;

pub use error::FlattenError;
