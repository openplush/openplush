use std::collections::{BTreeMap, BTreeSet};

use openplush_geom::units::{Mm, Scale};
use openplush_mesh::{HalfEdgeId, ManifoldMesh, VertexId};
use openplush_pattern::{
    CutCount, Fabric, FabricSpec, Outline, PatternError, PatternPiece, PieceId, allowance, parity,
};
use openplush_seams::{ChainSide, ChartMesh, SeamBundle, SewingMap};

mod group;
mod labels;
mod notches;

use crate::{
    EngineError, NonBlank, Parameterization, Remedy, StageResult, Staged, Warning, WarningCode,
    WarningSet, WarningTarget,
};

/// What the pattern stage reads: one run's charts and sewing map, and their layouts.
#[derive(Debug, Clone, Copy)]
pub struct PatternInput<'a> {
    /// The repaired mesh the charts were cut from, read to place each sewing side in its chart.
    pub mesh: &'a ManifoldMesh,
    /// The chart stage's charts, in chart order.
    pub charts: &'a [ChartMesh],
    /// The chart stage's sewing map, made with those charts.
    pub sewing: &'a SewingMap,
    /// The flatten stage's layouts, one per chart in the same order.
    pub layouts: &'a [Parameterization],
}

/// The maker's choices the pattern stage applies to every piece.
#[derive(Debug, Clone, PartialEq)]
pub struct PatternOptions {
    /// Millimetres per model unit.
    pub scale: Scale,
    /// The fabric every piece is cut from; its default allowance is every edge's.
    pub fabric: Fabric,
}

/// One piece per chart, in chart order, in millimetres: the chart's flat boundary as the sewing
/// line, counter-clockwise, and the cut line around it.
///
/// Each piece carries the match notches on its sides of every seam: one at each end of a side,
/// once where two seams meet, and more evenly spaced between, at most
/// [`notch::SPACING_MM`](openplush_pattern::notch::SPACING_MM) apart and at least one, at the same
/// fractions of the length on every side of the seam. Those between the ends carry the seam's
/// mark count, the least from 1 that no seam it meets on a piece has, in lead-chain order.
///
/// Each piece also carries its label: a block at `Outline::label_point` naming the piece, its
/// cuts, the fabric and the pieces it shares a seam with (`label::block`), and the seam's name
/// (`label::seam_text`) 4 mm inside the middle of each of its seam sides.
///
/// Raises `piece-compressed` on each piece whose layout needs more stretch to fill its surface
/// than the fabric gives.
///
/// # Errors
/// `StageInputMismatch` when the layouts or the sewing map are not the charts' own; then
/// `SeamParity` for the first bundle, in lead-chain order, with a side whose flat length differs
/// from its first side's, a flattener bug; then, per chart in order, `Pattern` naming the piece
/// when its sewing line or cut line is refused; then `Pattern` with `TooManyNotches` for the
/// first seam, in lead-chain order, that would need more than 1000 notches; then `Pattern` with
/// `BadLabelPoint`, naming the piece, for a label point that is not finite. No pieces are
/// returned with an error.
pub fn pattern(input: &PatternInput<'_>, opts: &PatternOptions) -> StageResult<Vec<PatternPiece>> {
    check_input(input)?;
    // Parity first: it checks the flattener's own postcondition, so a broken layout surfaces as
    // the bug it is, not as an outline refusal asking the maker to change the model.
    for bundle in input.sewing.bundles() {
        check_parity(input, bundle, opts.scale)?;
    }
    let pieces = input
        .charts
        .iter()
        .zip(input.layouts)
        .map(|(chart, layout)| piece(chart, layout, opts))
        .collect::<Result<Vec<_>, _>>()?;
    let bundles = notches::sides(input, opts.scale)?;
    let notches = notches::notches(input, &bundles, &pieces)?;
    let labels = labels::labels(&bundles, &pieces, &opts.fabric.spec().name)?;
    let value = pieces
        .into_iter()
        .zip(notches)
        .zip(labels)
        .map(|((p, n), l)| p.with_notches(n).with_label(l))
        .collect();
    let mut warnings = WarningSet::default();
    for w in input
        .layouts
        .iter()
        .filter_map(|l| squeeze_warning(l, opts.fabric.spec()))
    {
        warnings.insert(w);
    }
    Ok(Staged { value, warnings })
}

/// Each layout is its chart's, with one position per chart vertex, and each sewing side lies in
/// the faces of a chart given.
fn check_input(input: &PatternInput<'_>) -> Result<(), EngineError> {
    for (i, chart) in input.charts.iter().enumerate() {
        let own = input
            .layouts
            .get(i)
            .is_some_and(|l| l.chart() == chart.chart() && l.uv().len() == chart.xyz().len());
        if !own {
            return Err(mismatch(chart));
        }
    }
    if let Some(extra) = input.layouts.get(input.charts.len()) {
        return Err(EngineError::StageInputMismatch {
            chart: extra.chart(),
        });
    }
    for side in input.sewing.bundles().iter().flat_map(SeamBundle::sides) {
        let inside = input
            .charts
            .iter()
            .find(|c| c.chart() == side.chart())
            .is_some_and(|c| {
                side.half_edges()
                    .iter()
                    .all(|&h| flat_side(input.mesh, c, h).is_some())
            });
        if !inside {
            return Err(EngineError::StageInputMismatch {
                chart: side.chart(),
            });
        }
    }
    Ok(())
}

/// Every side of `bundle` has its first side's flat length, within the parity tolerance.
fn check_parity(
    input: &PatternInput<'_>,
    bundle: &SeamBundle,
    scale: Scale,
) -> Result<(), EngineError> {
    let mut sides = bundle.sides().iter();
    let Some(first) = sides.next() else {
        return Ok(());
    };
    let first = flat_length(input, first, scale)?;
    for side in sides {
        let other = flat_length(input, side, scale)?;
        parity::check(bundle.chain(), first, other).map_err(|e| match e {
            PatternError::SeamParity {
                chain,
                a,
                b,
                tolerance,
            } => EngineError::SeamParity {
                chain,
                a_mm: a.get(),
                b_mm: b.get(),
                tolerance_mm: tolerance.get(),
            },
            other => EngineError::Pattern {
                piece: None,
                source: other,
            },
        })?;
    }
    Ok(())
}

/// `side`'s length in its chart's layout, summed in model units and converted once.
fn flat_length(
    input: &PatternInput<'_>,
    side: &ChainSide,
    scale: Scale,
) -> Result<Mm, EngineError> {
    let unmatched = EngineError::StageInputMismatch {
        chart: side.chart(),
    };
    // INVARIANT: `check_input` found each side's chart among the charts, with its own layout at
    // the same index and every half-edge on its faces; a miss is the mismatch it ruled out.
    let c = input
        .charts
        .iter()
        .position(|c| c.chart() == side.chart())
        .ok_or_else(|| unmatched.clone())?;
    let (chart, uv) = (&input.charts[c], input.layouts[c].uv());
    let mut model = 0.0;
    for &h in side.half_edges() {
        let (a, b) = flat_side(input.mesh, chart, h).ok_or_else(|| unmatched.clone())?;
        let (pa, pb) = (uv.get(a.index() as usize), uv.get(b.index() as usize));
        let (Some(&pa), Some(&pb)) = (pa, pb) else {
            return Err(unmatched);
        };
        model += (pb - pa).length();
    }
    scale.to_mm(model).map_err(|source| EngineError::Pattern {
        piece: Some(PieceId::new(side.chart().index())),
        source: PatternError::BadScale { source },
    })
}

fn mismatch(chart: &ChartMesh) -> EngineError {
    EngineError::StageInputMismatch {
        chart: chart.chart(),
    }
}

/// The chart vertices `h` runs between, from its start to its end; `None` when `h` is not on a
/// face of `chart` in `mesh`.
fn flat_side(
    mesh: &ManifoldMesh,
    chart: &ChartMesh,
    h: HalfEdgeId,
) -> Option<(VertexId, VertexId)> {
    // Lossless: usize is at least 32 bits on every supported target.
    if h.index() as usize >= mesh.half_edge_count() {
        return None;
    }
    let face = mesh.face(h)?;
    let faces = chart.faces();
    let (_, corners) = faces[faces.binary_search_by_key(&face, |(f, _)| *f).ok()?];
    let first = mesh.face_half_edge(face);
    let k = [first, mesh.next(first), mesh.prev(first)]
        .iter()
        .position(|&s| s == h)?;
    Some((corners[k], corners[(k + 1) % 3]))
}

fn piece(
    chart: &ChartMesh,
    layout: &Parameterization,
    opts: &PatternOptions,
) -> Result<PatternPiece, EngineError> {
    let id = PieceId::new(chart.chart().index());
    let refused = |source| EngineError::Pattern {
        piece: Some(id),
        source,
    };
    // A layout exists only for a chart the flattener took as a disk, so a chart with no single
    // boundary loop cannot be this layout's.
    let walk = boundary_loop(chart).ok_or_else(|| mismatch(chart))?;
    let points = walk
        .iter()
        .map(|v| layout.uv().get(v.index() as usize).copied())
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| mismatch(chart))?;
    let sewing = Outline::new(
        points
            .into_iter()
            .map(|p| opts.scale.point_to_mm(p))
            .collect(),
    )
    .map_err(refused)?;
    let allowances = vec![opts.fabric.spec().default_allowance; sewing.points().len()];
    let cut = allowance::cut_line(id, &sewing, &allowances).map_err(refused)?;
    let cuts = CutCount::new(1, 0).map_err(refused)?;
    PatternPiece::new(id, sewing, cut, cuts, opts.fabric.rotations_deg()).map_err(refused)
}

/// The chart's boundary as chart vertices, interior on the left, from the first boundary side of
/// the lowest face that has one; `None` unless the boundary is exactly one loop.
fn boundary_loop(chart: &ChartMesh) -> Option<Vec<VertexId>> {
    let sides: Vec<(VertexId, VertexId)> = chart
        .faces()
        .iter()
        .flat_map(|&(_, [a, b, c])| [(a, b), (b, c), (c, a)])
        .collect();
    let all: BTreeSet<_> = sides.iter().copied().collect();
    let boundary: Vec<_> = sides
        .into_iter()
        .filter(|&(a, b)| !all.contains(&(b, a)))
        .collect();
    let mut next = BTreeMap::new();
    for &(a, b) in &boundary {
        if next.insert(a, b).is_some() {
            return None;
        }
    }
    let start = boundary.first()?.0;
    let mut walk = vec![start];
    let mut at = *next.get(&start)?;
    while at != start && walk.len() < boundary.len() {
        walk.push(at);
        at = *next.get(&at)?;
    }
    (at == start && walk.len() == boundary.len()).then_some(walk)
}

/// The stretch, as a fraction, that fills a layout back to its surface: a piece compressed by
/// `deficit` must grow by `1 / (1 − deficit)` to cover it.
fn needed_stretch(deficit: f64) -> f64 {
    1.0 / (1.0 - deficit) - 1.0
}

/// The stretch, as a fraction, a piece may count on from `spec`: its lesser direction, since the
/// deficit does not say which way the piece is squeezed, and never under 5%, so a fabric
/// recorded as not stretching still takes a slight squeeze.
fn available_stretch(spec: &FabricSpec) -> f64 {
    (spec.stretch_grain_pct.min(spec.stretch_cross_pct) / 100.0).max(0.05)
}

/// `piece-compressed` on the piece numbered as `layout`'s chart when it needs more stretch than
/// `spec` gives; nothing otherwise.
fn squeeze_warning(layout: &Parameterization, spec: &FabricSpec) -> Option<Warning> {
    let (needed, available) = (
        needed_stretch(layout.compression_deficit),
        available_stretch(spec),
    );
    if needed <= available {
        return None;
    }
    // INVARIANT: both texts are non-blank.
    let text = |t: String| NonBlank::new(t).unwrap_or_else(|| unreachable!());
    Some(Warning::new(
        WarningCode::PieceCompressed,
        WarningTarget::Piece(PieceId::new(layout.chart.index())),
        text(format!(
            "This piece must stretch about {:.0}% to fill its shape, more than this fabric's \
             {:.0}%, so it will sew up flatter than the model.",
            needed * 100.0,
            available * 100.0
        )),
        Remedy::Advisory {
            what_to_change: text("Add a dart or a seam in this piece.".to_owned()),
        },
    ))
}

#[cfg(test)]
mod tests;
