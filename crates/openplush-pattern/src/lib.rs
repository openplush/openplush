//! Pattern pieces: allowance, notches, labels, grain, piece dedupe.
//!
//! Scaffold, plus the type other crates name (T175). Responsibilities and the dependency DAG are in
//! `specs/001-pattern-pipeline/plan.md` and `crates/AGENTS.md`.

/// A pattern piece: one per chart, numbered as its chart (data-model section
/// 13a). When mirror-image pieces collapse into one template (FR-034, T116), the
/// template lists its member `PieceId`s and is named by the lowest. Stable for one
/// `SeamSet`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PieceId(u32);

impl PieceId {
    /// Wraps a raw index.
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// The raw index.
    pub const fn index(self) -> u32 {
        self.0
    }
}
