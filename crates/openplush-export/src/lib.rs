//! SVG and PDF writers, to bytes.
//!
//! What each crate holds today is in `docs/reference/crates.md`; what it will own is in
//! `crates/AGENTS.md`.

mod error;
pub mod pdf;
pub mod svg;
mod warning_summary;

pub use error::ExportError;
pub use warning_summary::SummaryRow;
