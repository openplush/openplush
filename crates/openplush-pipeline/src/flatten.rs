use std::collections::BTreeMap;

use openplush_flatten::summary::summarize;
use openplush_flatten::{FlattenError, FlattenSession, Progress, StoppedBy};
use openplush_geom::DVec2;
use openplush_mesh::{EdgeId, ManifoldMesh, VertexId};
use openplush_pattern::PieceId;
use openplush_seams::{ChartId, ChartMesh, Proposal, ProposalKind, SeamKind};

use crate::{
    EngineError, NonBlank, Remedy, StageResult, Staged, Warning, WarningCode, WarningSet,
    WarningTarget,
};

/// One chart laid flat.
#[derive(Debug, Clone, PartialEq)]
pub struct Parameterization {
    pub(crate) chart: ChartId,
    pub(crate) uv: Vec<DVec2>,
    /// The layout's area-weighted mean compression deficit, `Summary::compression_deficit`: the
    /// mean, since the worst face keeps falling as a mesh is refined.
    pub(crate) compression_deficit: f64,
}

impl Parameterization {
    /// Which of the chart stage's charts these positions belong to.
    #[must_use]
    pub fn chart(&self) -> ChartId {
        self.chart
    }

    /// One flat position per chart vertex, indexed like [`ChartMesh::xyz`], in model units.
    #[must_use]
    pub fn uv(&self) -> &[DVec2] {
        &self.uv
    }
}

/// Lays each chart flat, in the order given, stepping each chart's session to the end; `mesh`
/// is the one the charts were cut from, read only to name a dart's edges.
///
/// Raises `flatten-iteration-cap` on the piece of each chart whose flatten stopped at the cap,
/// keeping each layout's mean compression deficit for the pattern stage to judge.
///
/// # Errors
/// The first chart's refusal, and no layout for any chart: `Flip` naming the inverted faces,
/// `BoundaryNotRealizable` with its residual and the dart that would relieve it as a proposal,
/// or `Flatten` carrying any other refusal whole, measuring a layout's distortion included.
pub fn flatten(mesh: &ManifoldMesh, charts: &[ChartMesh]) -> StageResult<Vec<Parameterization>> {
    let laid = charts
        .iter()
        .map(|c| lay_flat(c).map_err(|e| refusal(mesh, c, e)))
        .collect::<Result<_, _>>()?;
    staged(charts, laid)
}

/// The layouts of `charts`, from each one's flat positions and stop in the same order, each
/// measured, with the warnings their stops raise.
///
/// # Errors
/// `Flatten` if a layout cannot be measured against its chart.
pub(crate) fn staged(
    charts: &[ChartMesh],
    laid: Vec<(Vec<DVec2>, StoppedBy)>,
) -> StageResult<Vec<Parameterization>> {
    let mut warnings = WarningSet::default();
    let mut value = Vec::with_capacity(laid.len());
    for (chart, (uv, stopped)) in charts.iter().zip(laid) {
        let summary = summarize(chart.chart(), chart.xyz(), chart.faces(), &uv, stopped)
            .map_err(EngineError::Flatten)?;
        if let Some(w) = stop_warning(chart.chart(), stopped) {
            warnings.insert(w);
        }
        value.push(Parameterization {
            chart: chart.chart(),
            uv,
            compression_deficit: summary.compression_deficit(),
        });
    }
    Ok(Staged { value, warnings })
}

/// The warning a chart's stop raises: `flatten-iteration-cap` on its piece, numbered as the
/// chart, for a stop at the cap; nothing for any other stop.
pub(crate) fn stop_warning(chart: ChartId, stopped: StoppedBy) -> Option<Warning> {
    if stopped != StoppedBy::IterationCap {
        return None;
    }
    // INVARIANT: both texts are non-blank literals.
    let text = |t: &str| NonBlank::new(t.to_owned()).unwrap_or_else(|| unreachable!());
    Some(Warning::new(
        WarningCode::FlattenIterationCap,
        WarningTarget::Piece(PieceId::new(chart.index())),
        text(
            "This piece stopped flattening at the iteration limit before it settled, so the \
             distortion shown may be more than the fabric must take.",
        ),
        Remedy::Advisory {
            what_to_change: text(
                "Add a seam or a dart in this piece, or keep it as it is: the layout is valid, \
                 only less relaxed than it could be.",
            ),
        },
    ))
}

#[cfg(test)]
impl Parameterization {
    /// A layout written by hand, uncompressed, to reach states no flatten produces.
    pub(crate) fn by_hand(chart: ChartId, uv: Vec<DVec2>) -> Self {
        Self {
            chart,
            uv,
            compression_deficit: 0.0,
        }
    }

    pub(crate) fn uv_mut(&mut self) -> &mut [DVec2] {
        &mut self.uv
    }
}

fn lay_flat(chart: &ChartMesh) -> Result<(Vec<DVec2>, StoppedBy), FlattenError> {
    let mut session = FlattenSession::new(chart.chart(), chart.xyz(), chart.faces().to_vec())?;
    while !matches!(session.step()?, Progress::Done) {}
    settled(session, chart.chart())
}

/// The flat positions of a session that has stepped to `Done`, indexed like the chart's `xyz`,
/// and why it stopped.
pub(crate) fn settled(
    session: FlattenSession,
    chart: ChartId,
) -> Result<(Vec<DVec2>, StoppedBy), FlattenError> {
    // INVARIANT: a session that has stepped to `Done` finishes, with every chart vertex once in
    // vertex order, so the positions alone are indexed like `xyz`.
    let (layout, stopped) = session
        .finish()
        .ok_or(FlattenError::SolveFailed { chart })?;
    Ok((layout.into_iter().map(|(_, p)| p).collect(), stopped))
}

/// `error`, from flattening `chart`, as the stage reports it.
pub(crate) fn refusal(mesh: &ManifoldMesh, chart: &ChartMesh, error: FlattenError) -> EngineError {
    match error {
        FlattenError::Flip { chart, faces } => EngineError::Flip { chart, faces },
        FlattenError::BoundaryNotRealizable {
            chart: id,
            residual,
            dart,
        } => EngineError::BoundaryNotRealizable {
            chart: id,
            residual,
            proposals: dart
                .and_then(|path| dart_proposal(mesh, chart, &path))
                .into_iter()
                .collect(),
        },
        other => EngineError::Flatten(other),
    }
}

/// The dart along `path`, a walk over `chart`'s vertices, as one proposal of its mesh edges;
/// `None` for a path with no edge.
fn dart_proposal(mesh: &ManifoldMesh, chart: &ChartMesh, path: &[VertexId]) -> Option<Proposal> {
    let mut add = BTreeMap::new();
    for pair in path.windows(2) {
        let edge = chart_edge(mesh, chart, pair[0], pair[1]);
        // INVARIANT: the flattener walks the chart's own edges, so every pair has a chart face;
        // a release build offers no dart rather than a partial one.
        debug_assert!(edge.is_some(), "dart step {pair:?} is not a chart edge");
        add.insert(edge?, SeamKind::Dart);
    }
    if add.is_empty() {
        return None;
    }
    let explanation = format!(
        "A dart of {} edges lets this piece's seam lengths close flat.",
        add.len()
    );
    // INVARIANT: `Proposal::new` refuses only an empty `add`, ruled out above.
    Proposal::new(
        ProposalKind::Dart {
            chart: chart.chart(),
        },
        add,
        explanation,
    )
    .ok()
}

/// The mesh edge between chart vertices `a` and `b`, found through a chart face that has both,
/// so where the mesh holds the pair as parallel copies it is the copy inside the chart.
fn chart_edge(mesh: &ManifoldMesh, chart: &ChartMesh, a: VertexId, b: VertexId) -> Option<EdgeId> {
    let to_global = chart.vtx_to_global();
    let (ga, gb) = (*to_global.get(ix(a))?, *to_global.get(ix(b))?);
    let (face, _) = chart
        .faces()
        .iter()
        .find(|(_, corners)| corners.contains(&a) && corners.contains(&b))?;
    let first = mesh.face_half_edge(*face);
    let sides = [first, mesh.next(first), mesh.next(mesh.next(first))];
    sides
        .into_iter()
        .find(|&h| {
            let ends = (mesh.origin(h), mesh.dest(h));
            ends == (ga, gb) || ends == (gb, ga)
        })
        .map(|h| h.edge())
}

#[inline]
fn ix(v: VertexId) -> usize {
    // Lossless: usize is at least 32 bits on every supported target.
    v.index() as usize
}

#[cfg(test)]
mod tests;
