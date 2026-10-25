//! SVG and PDF writers, to bytes.
//!
//! What this crate holds today is in `docs/reference/crates/openplush-export.md`; what it
//! will own is in `crates/AGENTS.md`.

mod error;
mod notch;
pub mod pdf;
pub mod svg;
mod warning_summary;

pub use error::ExportError;
pub use warning_summary::SummaryRow;
