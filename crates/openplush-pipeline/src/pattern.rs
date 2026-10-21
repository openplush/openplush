use std::collections::{BTreeMap, BTreeSet};

use openplush_geom::units::Scale;
use openplush_mesh::{HalfEdgeId, ManifoldMesh, VertexId};
use openplush_pattern::{CutCount, Fabric, Outline, PatternPiece, PieceId, allowance};
use openplush_seams::{ChartMesh, SeamBundle, SewingMap};

use crate::{EngineError, Parameterization, StageResult, Staged, WarningSet};

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
/// # Errors
/// `StageInputMismatch` when the layouts or the sewing map are not the charts' own; then, per
/// chart in order, `Pattern` naming the piece when its sewing line or cut line is refused. No
/// pieces are returned with an error.
pub fn pattern(input: &PatternInput<'_>, opts: &PatternOptions) -> StageResult<Vec<PatternPiece>> {
    check_input(input)?;
    let value = input
        .charts
        .iter()
        .zip(input.layouts)
        .map(|(chart, layout)| piece(chart, layout, opts))
        .collect::<Result<_, _>>()?;
    Ok(Staged {
        value,
        warnings: WarningSet::default(),
    })
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

#[cfg(test)]
mod tests;
