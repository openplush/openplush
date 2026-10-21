//! Page tiling and the nesting adapter.
//!
//! What this crate holds today is in `docs/reference/crates/openplush-layout.md`; what it
//! will own is in `crates/AGENTS.md`.

mod error;
mod options;
pub mod pages;
mod sheet;
pub mod shelf;

pub use error::LayoutError;
pub use options::LayoutOptions;
pub use sheet::{Placement, Sheet};
