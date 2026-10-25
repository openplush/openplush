//! The `templates` stage: identical and mirror-image pieces made one template with its cut count.

use std::collections::BTreeMap;

use openplush_mesh::VertexId;
use openplush_pattern::dedupe::Facing;
use openplush_pattern::{CutCount, PatternPiece, PieceId};
use openplush_seams::ChartId;

use super::group::{EdgeSeam, Grouping, Near, alike, group, same_mm};
use super::notches::{Bundle, sides};
use super::{PatternInput, PatternOptions, boundary_loop, check_input, labels, mismatch, notches};
use crate::{
    EngineError, NonBlank, Remedy, StageResult, Staged, Warning, WarningCode, WarningSet,
    WarningTarget,
};

/// `pattern`'s pieces, made templates: each piece joins the first lower-numbered template it
/// lies on within the seam-parity floor, as printed or else flipped, when their seams sew to the
/// same templates edge for edge; its template's cut count gains a copy as printed or flipped.
/// Templates come out in piece order, with notches whose mark counts match on matched seams and
/// labels naming templates, and a matched edge's seam names joined (`Seam 12 / 40`).
///
/// Raises `near-mirror-pieces` on each pair of templates that are mirror images to within 5 mm
/// but further apart than the floor. Each warning in `earlier` (the run's warnings so far) on a
/// piece that joined a template is moved to that template, so it names a piece the print shows;
/// on an error `earlier` is unchanged.
///
/// # Errors
/// `StageInputMismatch` when the inputs are not one run's or `pieces` are not `pattern`'s for
/// it, one per chart in chart order; then as `pattern`'s notches and labels, and `Pattern` with
/// `TooManyCuts` for a template of more than 100 copies.
pub fn templates(
    input: &PatternInput<'_>,
    pieces: &[PatternPiece],
    opts: &PatternOptions,
    earlier: &mut WarningSet,
) -> StageResult<Vec<PatternPiece>> {
    check_input(input)?;
    for (i, chart) in input.charts.iter().enumerate() {
        if pieces
            .get(i)
            .is_none_or(|p| p.id() != PieceId::new(chart.chart().index()))
        {
            return Err(mismatch(chart));
        }
    }
    if let Some(extra) = pieces.get(input.charts.len()) {
        return Err(EngineError::StageInputMismatch {
            chart: ChartId::new(extra.id().index()),
        });
    }
    let bundles = sides(input, opts.scale)?;
    let edges = edge_seams(input, &bundles)?;
    let outlines: Vec<_> = pieces.iter().map(PatternPiece::sewing_line).collect();
    let grouping = group(&outlines, &edges);
    let alike = alike(&grouping, &edges);
    let counted = counted(pieces, &grouping)?;
    let notches = notches::notches(input, &bundles, &counted, &alike)?;
    let labels = labels::labels(&bundles, &counted, &opts.fabric.spec().name, &alike)?;
    let value = counted
        .into_iter()
        .zip(notches)
        .zip(labels)
        .zip(&grouping.members)
        .filter(|(_, m)| m.fit.is_none())
        .map(|(((p, n), l), _)| p.with_notches(n).with_label(l))
        .collect();
    earlier.retarget(|t| match t {
        WarningTarget::Piece(id) => pieces
            .iter()
            .position(|p| p.id() == id)
            .and_then(|i| pieces.get(grouping.members.get(i)?.template))
            .map_or(t, |p| WarningTarget::Piece(p.id())),
        _ => t,
    });
    let mut warnings = WarningSet::default();
    for near in &grouping.near {
        warnings.insert(near_warning(pieces, near));
    }
    Ok(Staged { value, warnings })
}

/// Every piece, each template with its copies counted as printed and flipped.
fn counted(pieces: &[PatternPiece], grouping: &Grouping) -> Result<Vec<PatternPiece>, EngineError> {
    let mut counts: BTreeMap<usize, (u32, u32)> = BTreeMap::new();
    for m in &grouping.members {
        let (printed, flipped) = counts.entry(m.template).or_default();
        match m.fit.map(|f| f.facing()) {
            None | Some(Facing::AsPrinted) => *printed += 1,
            Some(Facing::Flipped) => *flipped += 1,
        }
    }
    pieces
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let (printed, flipped) = counts.get(&i).copied().unwrap_or((1, 0));
            let refused = |source| EngineError::Pattern {
                piece: Some(p.id()),
                source,
            };
            let cuts = CutCount::new(printed, flipped).map_err(refused)?;
            PatternPiece::new(
                p.id(),
                p.sewing_line().clone(),
                p.cut_line().clone(),
                cuts,
                p.rotations_deg().to_vec(),
            )
            .map_err(refused)
        })
        .collect()
}

/// Per chart, what sews along each edge of its sewing line, edge `i` from corner `i` of the
/// chart's boundary walk to the next.
fn edge_seams(
    input: &PatternInput<'_>,
    bundles: &[Bundle],
) -> Result<Vec<Vec<EdgeSeam>>, EngineError> {
    let mut on: BTreeMap<(usize, VertexId, VertexId), EdgeSeam> = BTreeMap::new();
    for (chain, sides) in bundles {
        for (k, s) in sides.iter().enumerate() {
            let partners = sides
                .iter()
                .enumerate()
                .filter(|&(j, _)| j != k)
                .map(|(_, o)| o.chart)
                .collect();
            let seam = EdgeSeam {
                chain: Some(*chain),
                partners,
            };
            for w in s.vertices().windows(2) {
                let key = (s.chart, w[0].min(w[1]), w[0].max(w[1]));
                on.insert(key, seam.clone());
            }
        }
    }
    input
        .charts
        .iter()
        .enumerate()
        .map(|(c, chart)| {
            let walk = boundary_loop(chart).ok_or_else(|| mismatch(chart))?;
            let n = walk.len();
            Ok((0..n)
                .map(|i| {
                    let (u, v) = (walk[i], walk[(i + 1) % n]);
                    on.get(&(c, u.min(v), u.max(v)))
                        .cloned()
                        .unwrap_or_default()
                })
                .collect())
        })
        .collect()
}

/// `near-mirror-pieces` for templates `near.a` and `near.b`.
fn near_warning(pieces: &[PatternPiece], near: &Near) -> Warning {
    let (a, b) = (pieces[near.a].id(), pieces[near.b].id());
    // INVARIANT: both texts are non-blank.
    let text = |t: String| NonBlank::new(t).unwrap_or_else(|| unreachable!());
    let gap = near.gap_mm;
    let what_to_change = if near.seams_mirror {
        format!(
            "The model's shape differs by up to {gap:.1} mm between them, so they stay two \
             pieces; make that part of the model symmetric to cut one piece twice."
        )
    } else {
        format!(
            "Draw the seams on these two pieces so they mirror each other; if their shapes then \
             match within {} mm, they print as one piece cut twice.",
            same_mm()
        )
    };
    Warning::new(
        WarningCode::NearMirrorPieces,
        WarningTarget::Pieces(a, b),
        text(format!(
            "Pieces {} and {} are mirror images to within {gap:.1} mm, so they print as two \
             pieces instead of one piece cut twice.",
            a.index(),
            b.index()
        )),
        Remedy::Advisory {
            what_to_change: text(what_to_change),
        },
    )
}

#[cfg(test)]
mod tests;
