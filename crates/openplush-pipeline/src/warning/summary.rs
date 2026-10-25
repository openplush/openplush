use std::fmt;

use openplush_export::SummaryRow;

use super::{Remedy, WarningSet, WarningTarget};

/// One row per warning, in the set's order: every warning prints, acknowledged or not.
pub(crate) fn rows(warnings: &WarningSet) -> Vec<SummaryRow> {
    warnings
        .iter()
        .map(|w| {
            SummaryRow::new(
                w.code().as_str().to_owned(),
                w.target().to_string(),
                w.explanation().to_owned(),
                w.remedy().text().to_owned(),
            )
        })
        .collect()
}

impl fmt::Display for WarningTarget {
    /// The target in the words the print uses: `piece 3`, `pieces 2 and 7`, `the whole pattern`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Face(id) => write!(f, "face {}", id.index()),
            Self::Edge(id) => write!(f, "edge {}", id.index()),
            Self::Chain(id) => write!(f, "seam chain {}", id.index()),
            Self::Piece(id) => write!(f, "piece {}", id.index()),
            Self::Pieces(a, b) => write!(f, "pieces {} and {}", a.index(), b.index()),
            Self::Component(id) => write!(f, "component {}", id.index()),
            Self::Project => f.write_str("the whole pattern"),
        }
    }
}

impl Remedy {
    /// What to do, in plain language: the proposal's explanation or the advice.
    pub fn text(&self) -> &str {
        match self {
            Self::Actionable { proposal } => proposal.explanation(),
            Self::Advisory { what_to_change } => what_to_change.as_str(),
        }
    }
}

#[cfg(test)]
mod tests;
