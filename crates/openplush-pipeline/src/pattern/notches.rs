//! Each piece's match notches, laid out from the sewing map's sides.

use std::collections::{BTreeMap, BTreeSet};

use openplush_geom::DVec2;
use openplush_geom::units::{Mm, Scale};
use openplush_mesh::VertexId;
use openplush_pattern::{Notch, PatternError, PatternPiece, PieceId, notch};
use openplush_seams::{ChainSide, SeamChainId};

use super::group::Alike;
use super::{PatternInput, flat_side};
use crate::EngineError;

/// A bundle's lead chain and its sides.
pub(super) type Bundle = (SeamChainId, Vec<Side>);

/// One side laid flat, in walking order: its chart's index, its chart vertices, and per edge its
/// two ends in mm and the unit direction out of the piece.
pub(super) struct Side {
    pub(super) chart: usize,
    vertices: Vec<VertexId>,
    edges: Vec<(DVec2, DVec2, DVec2)>,
}

impl Side {
    // INVARIANT: `side` builds a side from at least one edge, so it has two or more vertices
    // and these four never miss.
    fn first(&self) -> VertexId {
        self.vertices[0]
    }

    fn last(&self) -> VertexId {
        self.vertices[self.vertices.len() - 1]
    }

    fn start(&self) -> DVec2 {
        self.edges[0].0
    }

    fn end(&self) -> DVec2 {
        self.edges[self.edges.len() - 1].1
    }

    /// Its chart vertices in walking order.
    pub(super) fn vertices(&self) -> &[VertexId] {
        &self.vertices
    }

    fn length(&self) -> f64 {
        self.edges.iter().map(|(a, b, _)| (*b - *a).length()).sum()
    }

    /// The point `t` of the way along, and the outward direction of the edge it is on.
    pub(super) fn at(&self, t: f64) -> (DVec2, DVec2) {
        let mut left = t * self.length();
        for &(a, b, out) in &self.edges {
            let len = (b - a).length();
            if left <= len {
                return (a + (b - a) * (left / len), out);
            }
            left -= len;
        }
        // INVARIANT: a side has at least one edge, and `t` is under 1.
        let &(_, b, out) = self.edges.last().unwrap_or_else(|| unreachable!());
        (b, out)
    }
}

/// Every bundle's lead chain with its sides laid flat, in lead-chain order.
///
/// # Errors
/// `StageInputMismatch` as `pattern`.
pub(super) fn sides(input: &PatternInput<'_>, scale: Scale) -> Result<Vec<Bundle>, EngineError> {
    let mut bundles = Vec::new();
    for bundle in input.sewing.bundles() {
        let sides = bundle
            .sides()
            .iter()
            .map(|s| side(input, s, scale))
            .collect::<Result<Vec<_>, _>>()?;
        bundles.push((bundle.chain(), sides));
    }
    Ok(bundles)
}

/// The notches of each chart's piece, in chart order. Every side of a bundle gets a notch at
/// each end and at the same fractions of its length, with the bundle's mark count; an end shared
/// by two seams on one piece is notched once, for the lower lead chain.
///
/// Chains `alike` maps to one chain, the same seam on matching pieces, share a mark count.
///
/// # Errors
/// `Pattern` with `TooManyNotches` for a seam too long to notch; `StageInputMismatch` as
/// `pattern`.
pub(super) fn notches(
    input: &PatternInput<'_>,
    bundles: &[Bundle],
    pieces: &[PatternPiece],
    alike: &Alike,
) -> Result<Vec<Vec<Notch>>, EngineError> {
    let marks = notch::mark_counts(&meets(bundles, alike));
    let mut out = vec![Vec::new(); input.charts.len()];
    let mut ended = BTreeSet::new();
    for (chain, sides) in bundles {
        let Some(first) = sides.first() else {
            continue;
        };
        let first_piece = Some(piece_of(input, first));
        let length = Mm::try_new(first.length())
            .map_err(|source| refused(first_piece, PatternError::BadScale { source }))?;
        let fractions =
            notch::fractions(*chain, length).map_err(|source| refused(first_piece, source))?;
        // INVARIANT: `meets` holds every lead chain, so each has a count.
        let count = marks
            .get(&alike.of(*chain))
            .copied()
            .unwrap_or_else(|| unreachable!());
        for s in sides {
            let id = piece_of(input, s);
            let piece = &mut out[s.chart];
            for (p, v) in [(s.start(), s.first()), (s.end(), s.last())] {
                if ended.insert((s.chart, v)) {
                    piece.push(corner(input, pieces, s, *chain, p)?);
                }
            }
            for &t in &fractions {
                piece.push(made(id, *chain, s.at(t), count)?);
            }
        }
    }
    Ok(out)
}

fn made(
    piece: PieceId,
    chain: SeamChainId,
    (at, out): (DVec2, DVec2),
    marks: u32,
) -> Result<Notch, EngineError> {
    Notch::new(at, out, chain, marks).map_err(|source| refused(Some(piece), source))
}

/// The single notch at `p`, an end of `side` and so a corner of its piece's sewing line, pointing
/// out between the corner's two edges.
fn corner(
    input: &PatternInput<'_>,
    pieces: &[PatternPiece],
    side: &Side,
    chain: SeamChainId,
    p: DVec2,
) -> Result<Notch, EngineError> {
    let line = pieces[side.chart].sewing_line();
    let at = line
        .points()
        .iter()
        .position(|&q| q == p)
        .ok_or(EngineError::StageInputMismatch {
            chart: input.charts[side.chart].chart(),
        })?;
    // A zero direction, which a simple outline never gives, is refused by `Notch::new`.
    let out = line.outward_at(at).unwrap_or(DVec2::new(0.0, 0.0));
    made(piece_of(input, side), chain, (p, out), 1)
}

fn piece_of(input: &PatternInput<'_>, side: &Side) -> PieceId {
    PieceId::new(input.charts[side.chart].chart().index())
}

fn refused(piece: Option<PieceId>, source: PatternError) -> EngineError {
    EngineError::Pattern { piece, source }
}

/// Each chain, as `alike` maps it, with the chains whose sides end at a chart vertex where one
/// of its own does.
fn meets(bundles: &[Bundle], alike: &Alike) -> BTreeMap<SeamChainId, BTreeSet<SeamChainId>> {
    let mut at: BTreeMap<(usize, VertexId), BTreeSet<SeamChainId>> = BTreeMap::new();
    for (chain, sides) in bundles {
        for s in sides {
            for v in [s.first(), s.last()] {
                at.entry((s.chart, v)).or_default().insert(alike.of(*chain));
            }
        }
    }
    let mut meets: BTreeMap<_, BTreeSet<_>> = bundles
        .iter()
        .map(|(c, _)| (alike.of(*c), BTreeSet::new()))
        .collect();
    for chains in at.values() {
        for &a in chains {
            for &b in chains {
                if a != b {
                    meets.entry(a).or_default().insert(b);
                }
            }
        }
    }
    meets
}

/// `side` laid out in its chart, in mm, its edges put in walking order end to start.
fn side(input: &PatternInput<'_>, side: &ChainSide, scale: Scale) -> Result<Side, EngineError> {
    let unmatched = || EngineError::StageInputMismatch {
        chart: side.chart(),
    };
    let chart = input
        .charts
        .iter()
        .position(|c| c.chart() == side.chart())
        .ok_or_else(unmatched)?;
    let (mesh_chart, uv) = (&input.charts[chart], input.layouts[chart].uv());
    let mut flat = Vec::new();
    for &h in side.half_edges() {
        let (a, b) = flat_side(input.mesh, mesh_chart, h).ok_or_else(unmatched)?;
        let at = |v: VertexId| uv.get(v.index() as usize).map(|&p| scale.point_to_mm(p));
        let (Some(pa), Some(pb)) = (at(a), at(b)) else {
            return Err(unmatched());
        };
        // The piece lies left of each chart face side, so out is to the right.
        let d = pb - pa;
        // A zero edge, which a layout with every det J > 0 never has, leaves a zero direction that
        // `Notch::new` refuses.
        let out = DVec2::new(d.y, -d.x)
            .normalize()
            .unwrap_or(DVec2::new(0.0, 0.0));
        flat.push((a, b, pa, pb, out));
    }
    // A side's half-edges each run one way along their chart faces, which may be against the walk.
    let backward = flat.len() > 1 && flat[0].1 != flat[1].0;
    let mut vertices = Vec::new();
    let mut edges = Vec::new();
    for (i, &(a, b, pa, pb, out)) in flat.iter().enumerate() {
        let (va, vb, from, to) = if backward {
            (b, a, pb, pa)
        } else {
            (a, b, pa, pb)
        };
        if i == 0 {
            vertices.push(va);
        } else if vertices.last() != Some(&va) {
            return Err(unmatched());
        }
        vertices.push(vb);
        edges.push((from, to, out));
    }
    if edges.is_empty() {
        return Err(unmatched());
    }
    Ok(Side {
        chart,
        vertices,
        edges,
    })
}

#[cfg(test)]
mod tests;
