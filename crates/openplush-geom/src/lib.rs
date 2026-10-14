//! f64 vectors and the epsilon policy.
//!
//! The bottom of the engine's crate DAG: everything geometric depends on these
//! types, so they stay small, dependency-light, and deterministic. Plain IEEE-754
//! arithmetic only (no fused multiply-add unless written explicitly), so native and
//! WASM builds compute the same bits (Constitution V).

pub mod epsilon;
mod error;
pub mod units;
mod vector;

pub use error::GeomError;
pub use vector::{DVec2, DVec3};
