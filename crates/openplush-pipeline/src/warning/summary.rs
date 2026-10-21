use openplush_export::SummaryRow;

use super::{Remedy, WarningSet, WarningTarget};

/// One row per warning, in the set's order: every warning prints, acknowledged or not.
pub(crate) fn rows(warnings: &WarningSet) -> Vec<SummaryRow> {
    warnings
        .iter()
        .map(|w| {
            SummaryRow::new(
                w.code().as_str().to_owned(),
                target_words(w.target()),
                w.explanation().to_owned(),
                remedy_text(w.remedy()).to_owned(),
            )
        })
        .collect()
}

fn target_words(target: WarningTarget) -> String {
    match target {
        WarningTarget::Face(id) => format!("face {}", id.index()),
        WarningTarget::Edge(id) => format!("edge {}", id.index()),
        WarningTarget::Chain(id) => format!("seam chain {}", id.index()),
        WarningTarget::Piece(id) => format!("piece {}", id.index()),
        WarningTarget::Component(id) => format!("component {}", id.index()),
        WarningTarget::Project => "the whole pattern".to_owned(),
    }
}

fn remedy_text(remedy: &Remedy) -> &str {
    match remedy {
        Remedy::Actionable { proposal } => proposal.explanation(),
        Remedy::Advisory { what_to_change } => what_to_change.as_str(),
    }
}
