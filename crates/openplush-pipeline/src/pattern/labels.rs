//! Each piece's label: its block, and a seam label at the middle of each of its seam sides.

use std::collections::{BTreeMap, BTreeSet};

use openplush_pattern::label::{block, seam_text};
use openplush_pattern::{Label, PatternError, PatternPiece, PieceId, SeamLabel};
use openplush_seams::SeamChainId;

use super::group::Alike;
use super::notches::{Bundle, Side};
use crate::EngineError;

/// How far inside its side a seam label sits, clear of the notch ticks across the allowance.
const SEAM_INSET_MM: f64 = 4.0;

/// The label of each piece, in chart order: the block at the piece's label point, naming the
/// templates of the pieces it shares a seam with other than itself, and a seam label
/// `SEAM_INSET_MM` inside the middle of each of its seam sides, naming the seam and those
/// `alike` matches to it on the piece's members.
///
/// # Errors
/// `Pattern` with `BadLabelPoint` for a point that is not finite.
pub(super) fn labels(
    bundles: &[Bundle],
    pieces: &[PatternPiece],
    fabric: &str,
    alike: &Alike,
) -> Result<Vec<Label>, EngineError> {
    let mut partners: BTreeMap<usize, BTreeSet<PieceId>> = BTreeMap::new();
    let mut seams: BTreeMap<usize, Vec<SeamLabel>> = BTreeMap::new();
    for (chain, sides) in bundles {
        let charts: BTreeSet<usize> = sides.iter().map(|s| s.chart).collect();
        for s in sides {
            let others = charts.iter().filter(|&&c| c != s.chart);
            partners
                .entry(s.chart)
                .or_default()
                .extend(others.map(|&c| pieces[alike.template_of(c)].id()));
            let label = seam_label(s, &pieces[s.chart], &alike.names(s.chart, *chain))?;
            seams.entry(s.chart).or_default().push(label);
        }
    }
    pieces
        .iter()
        .enumerate()
        .map(|(c, piece)| {
            let sews_to: Vec<PieceId> = partners.remove(&c).into_iter().flatten().collect();
            let lines = block(piece.id(), piece.cuts(), fabric, &sews_to);
            let own = seams.remove(&c).unwrap_or_default();
            Label::new(piece.sewing_line().label_point(), lines, own)
                .map_err(|source| refused(piece, source))
        })
        .collect()
}

/// The label of `side`, on `piece`, naming `chains`.
fn seam_label(
    side: &Side,
    piece: &PatternPiece,
    chains: &[SeamChainId],
) -> Result<SeamLabel, EngineError> {
    let (mid, edge_out) = side.at(0.5);
    // A midpoint on a corner goes in along the corner's bisector, as one edge's normal can lead
    // out past the other edge.
    let line = piece.sewing_line();
    let out = line
        .points()
        .iter()
        .position(|&q| q == mid)
        .and_then(|i| line.outward_at(i))
        .unwrap_or(edge_out);
    SeamLabel::new(mid - out * SEAM_INSET_MM, seam_text(chains))
        .map_err(|source| refused(piece, source))
}

fn refused(piece: &PatternPiece, source: PatternError) -> EngineError {
    EngineError::Pattern {
        piece: Some(piece.id()),
        source,
    }
}
