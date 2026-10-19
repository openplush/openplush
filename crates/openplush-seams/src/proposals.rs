//! The cuts that close a chart that is not a disk.

use std::collections::BTreeMap;

use openplush_mesh::{EdgeId, ManifoldMesh};

use crate::paths::CutOpen;
use crate::{
    Chart, ChartFailure, Proposal, ProposalKind, SeamKind, SeamSet, SeamsError, charts, check_disk,
};

/// One proposal of `Cut` edges that leaves `chart`, from `charts(mesh, seams)`, a disk, or `None`
/// when it is one. Every search runs over the chart cut open along `seams`, on edges with a chart
/// face on both sides, each costing its 3D length. A closed chart is cut between its two vertices
/// of largest angle defect; otherwise its boundary loops are joined into one, then its handles
/// cut by tree-cotree.
///
/// # Errors
/// `ChartNotClosed` when the cuts found do not leave the chart's faces one disk; it is never
/// offered. `MandatorySeam` cannot arise, as only edges that are not seams are cut.
pub fn close_chart(
    mesh: &ManifoldMesh,
    seams: &SeamSet,
    chart: &Chart,
) -> Result<Option<Proposal>, SeamsError> {
    let Err(failure) = check_disk(mesh, seams, chart) else {
        return Ok(None);
    };
    let not_closed = SeamsError::ChartNotClosed { chart: chart.id() };
    let mut working = seams.clone();
    let mut add = BTreeMap::new();
    if failure == ChartFailure::Closed {
        let found = CutOpen::new(mesh, &working, chart).between_sharpest();
        cut(&mut working, &mut add, found.ok_or(not_closed.clone())?)?;
    } else {
        let found = CutOpen::new(mesh, &working, chart).joining_loops();
        cut(&mut working, &mut add, found.ok_or(not_closed.clone())?)?;
        if let Err(ChartFailure::PositiveGenus { .. }) = check_disk(mesh, &working, chart) {
            let found = CutOpen::new(mesh, &working, chart).handle_loops();
            cut(&mut working, &mut add, found.ok_or(not_closed.clone())?)?;
        }
    }
    let disk = charts(mesh, &working)?
        .iter()
        .find(|c| c.faces() == chart.faces())
        .is_some_and(|c| check_disk(mesh, &working, c).is_ok());
    if !disk {
        return Err(not_closed);
    }
    let explanation = explain(failure, add.len());
    let kind = ProposalKind::CloseChart {
        chart: chart.id(),
        failure,
    };
    Proposal::new(kind, add, explanation).map(Some)
}

fn cut(
    working: &mut SeamSet,
    add: &mut BTreeMap<EdgeId, SeamKind>,
    edges: Vec<EdgeId>,
) -> Result<(), SeamsError> {
    for edge in edges {
        if add.insert(edge, SeamKind::Cut).is_none() {
            working.insert(edge, SeamKind::Cut)?;
        }
    }
    Ok(())
}

fn explain(failure: ChartFailure, edges: usize) -> String {
    let why = match failure {
        ChartFailure::Closed => "is closed all round".to_owned(),
        ChartFailure::MultipleBoundaries { count } => format!("has {count} separate open edges"),
        ChartFailure::PositiveGenus { genus: 1 } => "wraps round a handle".to_owned(),
        ChartFailure::PositiveGenus { genus } => format!("wraps round {genus} handles"),
    };
    let edges = match edges {
        1 => "one more edge".to_owned(),
        n => format!("{n} more edges"),
    };
    format!(
        "This part {why}, so it cannot lie flat as one piece. Cutting along {edges} opens it into \
         one piece that can."
    )
}
