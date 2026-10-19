//! The cut line: the sewing line grown by a strip along each edge, each edge its own width,
//! with nothing past a zero-allowance edge (a fold or an edge-stitched edge).

use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::float::overlay::FloatOverlay;
use openplush_geom::DVec2;
use openplush_geom::epsilon::WELD_RELATIVE;
use openplush_geom::units::Mm;

use crate::{Outline, PatternError, PieceId};

type P = [f64; 2];

/// How deep a zero-allowance edge's guard reaches, in multiples of the widest allowance: as far
/// as any strip or corner join can.
const GUARD_DEPTH: f64 = 4.0;

/// The cut line around `sewing`; `allowances[i]` is edge `i`'s, the edge from point `i` to the
/// next. All zero gives the sewing line itself.
///
/// # Errors
/// `AllowanceCount` unless there is one allowance per edge; `FoldConflict` when a zero edge's
/// guard reaches into the piece; `AllowanceSplitsPiece`, `AllowanceEnclosesGap` or
/// `AllowanceNotSimple` when the allowances cannot make one simple cut line.
pub fn cut_line(
    piece: PieceId,
    sewing: &Outline,
    allowances: &[Mm],
) -> Result<Outline, PatternError> {
    let edges = sewing.points().len();
    if allowances.len() != edges {
        return Err(PatternError::AllowanceCount {
            piece,
            edges,
            allowances: allowances.len(),
        });
    }
    let widest = allowances.iter().map(|a| a.get()).fold(0.0, f64::max);
    if widest == 0.0 {
        return Ok(sewing.clone());
    }
    let not_simple = PatternError::AllowanceNotSimple { piece };
    let diagonal = diagonal(sewing.points());
    let edges = merge_short_edges(sewing.points(), allowances, WELD_RELATIVE * diagonal);
    // Merging can make a simple outline touch itself.
    Outline::new(edges.iter().map(|e| e.start).collect()).map_err(|_| not_simple.clone())?;
    let outline: Vec<P> = edges.iter().map(|e| array(e.start)).collect();
    // Every edge has a direction: the merged outline has no repeated point.
    let guards = guards(&edges, GUARD_DEPTH * widest).ok_or(not_simple.clone())?;
    // A guard that only touches the piece overlaps it by rounding alone.
    let touching = WELD_RELATIVE * diagonal * diagonal;
    if let Some((edge, _)) = guards
        .iter()
        .find(|(_, g)| overlap_area(&outline, g) > touching)
    {
        return Err(PatternError::FoldConflict { piece, edge: *edge });
    }
    let strips: Option<Vec<Vec<P>>> = edges.iter().zip(next(&edges)).filter_map(strip).collect();
    let mut subject = vec![outline];
    subject.extend(strips.ok_or(not_simple.clone())?);
    let clip: Vec<Vec<P>> = guards.into_iter().map(|(_, g)| g).collect();
    // One overlay, so every input shares one integer grid: two would leave slivers along folds.
    let shapes = FloatOverlay::<P, i64>::from_subj_and_clip(&subject, &clip)
        .overlay(OverlayRule::Difference, FillRule::NonZero);
    let [shape] = shapes.as_slice() else {
        return Err(PatternError::AllowanceSplitsPiece { piece });
    };
    let [outer] = shape.as_slice() else {
        return Err(PatternError::AllowanceEnclosesGap { piece });
    };
    Outline::new(outer.iter().map(|&[x, y]| DVec2::new(x, y)).collect()).map_err(|_| not_simple)
}

/// An edge of the outline being grown: where it starts, its allowance in mm, and its index in
/// the sewing line.
#[derive(Debug, Clone, Copy)]
struct Edge {
    start: DVec2,
    width: f64,
    index: usize,
}

/// Each edge with the one after it, the last with the first.
fn next(edges: &[Edge]) -> impl Iterator<Item = &Edge> {
    edges.iter().cycle().skip(1)
}

/// Drops every edge shorter than `shortest`, and its end point; the edge it ran into now starts
/// where it did and keeps its own allowance. Such an edge is too short for its strip to survive
/// the overlay's integer grid. Stops at three edges.
fn merge_short_edges(points: &[DVec2], allowances: &[Mm], shortest: f64) -> Vec<Edge> {
    let mut edges: Vec<Edge> = points
        .iter()
        .zip(allowances)
        .enumerate()
        .map(|(index, (&start, width))| Edge {
            start,
            width: width.get(),
            index,
        })
        .collect();
    let mut i = 0;
    while i < edges.len() && edges.len() > 3 {
        let j = (i + 1) % edges.len();
        if (edges[i].start - edges[j].start).length() < shortest {
            edges[i] = Edge {
                start: edges[i].start,
                ..edges[j]
            };
            edges.remove(j);
            // Removing the first edge moved this one down a place.
            i -= usize::from(j < i);
        } else {
            i += 1;
        }
    }
    edges
}

/// The rectangle on the outer side of `edge`, as deep as its allowance; nothing at zero.
fn strip((edge, end): (&Edge, &Edge)) -> Option<Option<Vec<P>>> {
    (edge.width > 0.0).then(|| rectangle(edge.start, end.start, edge.width))
}

/// A rectangle on the outer side of every zero-allowance edge, `depth` deep, with that edge's
/// index in the sewing line: no fabric may lie there.
fn guards(edges: &[Edge], depth: f64) -> Option<Vec<(usize, Vec<P>)>> {
    edges
        .iter()
        .zip(next(edges))
        .filter(|(edge, _)| edge.width == 0.0)
        .map(|(edge, end)| Some((edge.index, rectangle(edge.start, end.start, depth)?)))
        .collect()
}

/// `a`, `b`, and the two points `depth` out from them on the right of `a → b`, which is the
/// outside of a counter-clockwise outline. Counter-clockwise itself; `None` when `a == b`.
fn rectangle(a: DVec2, b: DVec2, depth: f64) -> Option<Vec<P>> {
    let d = (b - a).normalize()?;
    let out = DVec2::new(d.y, -d.x) * depth;
    Some([a, a + out, b + out, b].into_iter().map(array).collect())
}

fn array(p: DVec2) -> P {
    [p.x, p.y]
}

/// The area `guard` and the outline share.
fn overlap_area(outline: &[P], guard: &[P]) -> f64 {
    FloatOverlay::<P, i64>::from_subj_and_clip(outline, guard)
        .overlay(OverlayRule::Intersect, FillRule::NonZero)
        .iter()
        .flatten()
        .map(|contour| signed_area(contour))
        .sum()
}

/// Positive for a counter-clockwise contour, negative for a hole.
fn signed_area(contour: &[P]) -> f64 {
    let n = contour.len();
    let twice: f64 = (0..n)
        .map(|i| {
            let ([x0, y0], [x1, y1]) = (contour[i], contour[(i + 1) % n]);
            x0 * y1 - x1 * y0
        })
        .sum();
    twice / 2.0
}

/// The diagonal of the points' bounding box.
fn diagonal(points: &[DVec2]) -> f64 {
    // INVARIANT: an `Outline` has at least three points.
    let (mut min, mut max) = (points[0], points[0]);
    for p in points {
        min = DVec2::new(min.x.min(p.x), min.y.min(p.y));
        max = DVec2::new(max.x.max(p.x), max.y.max(p.y));
    }
    (max - min).length()
}
