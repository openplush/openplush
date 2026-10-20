use openplush_mesh::ManifoldMesh;
use openplush_seams::{
    Chart, ChartMesh, SeamGraph, SeamSet, SeamsError, SewingMap, charts, check_disk, check_fold,
    close_chart,
};

use crate::{EngineError, StageResult, Staged, WarningSet};

/// Splits `mesh` along `seams` into charts cut open for flattening, and the sewing map joining
/// their seam sides; the first chart that is folded or not a disk, or seam that cannot be sewn,
/// is the error.
///
/// # Errors
/// `Seams` for a seam on an edge the mesh lacks, then per chart in ID order `FoldedChart` or
/// `ChartNotDisk` (with the cut that fixes it, if one was found), then `Seams` for a seam chain
/// that has no partner to sew to.
pub fn chart(mesh: &ManifoldMesh, seams: &SeamSet) -> StageResult<(Vec<ChartMesh>, SewingMap)> {
    let found = charts(mesh, seams)?;
    for c in &found {
        check_chart(mesh, seams, c)?;
    }
    let pieces = found
        .iter()
        .map(|c| ChartMesh::new(mesh, seams, c))
        .collect();
    let groups = mesh.edge_copies();
    let graph = SeamGraph::with_copies(mesh, seams, groups)?;
    let map = SewingMap::new(mesh, seams, &graph, groups)?;
    Ok(Staged {
        value: (pieces, map),
        warnings: WarningSet::default(),
    })
}

/// Fold before topology: a folded chart's remedy is the model, so no cuts are proposed for it.
fn check_chart(mesh: &ManifoldMesh, seams: &SeamSet, chart: &Chart) -> Result<(), EngineError> {
    match check_fold(mesh, seams, chart) {
        Ok(()) => {}
        Err(SeamsError::FoldedChart { chart, edge }) => {
            return Err(EngineError::FoldedChart { chart, edge });
        }
        Err(other) => return Err(other.into()),
    }
    let Err(failure) = check_disk(mesh, seams, chart) else {
        return Ok(());
    };
    // `ChartNotClosed`, its one refusal: no cut found leaves a disk, so none is offered.
    let proposals = close_chart(mesh, seams, chart)
        .ok()
        .flatten()
        .into_iter()
        .collect();
    Err(EngineError::ChartNotDisk {
        chart: chart.id(),
        failure,
        proposals,
    })
}
