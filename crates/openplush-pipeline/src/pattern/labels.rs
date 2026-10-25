//! Each piece's label: its block, and a seam label at the middle of each of its seam sides.

use std::collections::{BTreeMap, BTreeSet};

use openplush_pattern::label::{block, seam_text};
use openplush_pattern::{Label, PatternPiece, PieceId, SeamLabel};

use super::notches::Bundle;
use crate::EngineError;

/// How far inside its side a seam label sits, clear of the notch ticks across the allowance.
const SEAM_INSET_MM: f64 = 4.0;

/// The label of each piece, in chart order: the block at the piece's label point, naming the
/// pieces it shares a seam with other than itself, and a seam label `SEAM_INSET_MM` inside the
/// middle of each of its seam sides.
///
/// # Errors
/// `Pattern` with `BadLabelPoint` for a point that is not finite.
pub(super) fn labels(
    bundles: &[Bundle],
    pieces: &[PatternPiece],
    fabric: &str,
) -> Result<Vec<Label>, EngineError> {
    let mut partners: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    let mut seams: BTreeMap<usize, Vec<SeamLabel>> = BTreeMap::new();
    for (chain, sides) in bundles {
        let charts: BTreeSet<usize> = sides.iter().map(|s| s.chart).collect();
        for s in sides {
            partners
                .entry(s.chart)
                .or_default()
                .extend(charts.iter().filter(|&&c| c != s.chart));
            let (mid, edge_out) = s.at(0.5);
            // A midpoint on a corner goes in along the corner's bisector, as one edge's normal
            // can lead out past the other edge.
            let line = pieces[s.chart].sewing_line();
            let out = line
                .points()
                .iter()
                .position(|&q| q == mid)
                .and_then(|i| line.outward_at(i))
                .unwrap_or(edge_out);
            let label = SeamLabel::new(mid - out * SEAM_INSET_MM, seam_text(*chain))
                .map_err(|source| refused(&pieces[s.chart], source))?;
            seams.entry(s.chart).or_default().push(label);
        }
    }
    pieces
        .iter()
        .enumerate()
        .map(|(c, piece)| {
            let sews_to: Vec<PieceId> = partners
                .get(&c)
                .into_iter()
                .flatten()
                .map(|&p| pieces[p].id())
                .collect();
            let lines = block(piece.id(), piece.cuts(), fabric, &sews_to);
            let own = seams.remove(&c).unwrap_or_default();
            Label::new(piece.sewing_line().label_point(), lines, own)
                .map_err(|source| refused(piece, source))
        })
        .collect()
}

fn refused(piece: &PatternPiece, source: openplush_pattern::PatternError) -> EngineError {
    EngineError::Pattern {
        piece: Some(piece.id()),
        source,
    }
}
