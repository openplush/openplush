//! SVG and PDF writers, to bytes.
//!
//! What each crate holds today is in `docs/reference/crates.md`; what it will own is in
//! `crates/AGENTS.md`.

mod error;
pub mod pdf;
pub mod svg;

pub use error::ExportError;
