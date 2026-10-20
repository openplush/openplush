//! The stage signatures in the engine API contract compile and are callable as documented.

use openplush_pipeline::{ChartMesh, ManifoldMesh, SeamSet, SewingMap, StageResult, chart};

type Charted = StageResult<(Vec<ChartMesh>, SewingMap)>;

#[test]
fn chart_takes_a_mesh_and_seams_and_returns_charts_with_their_sewing_map() {
    let _: fn(&ManifoldMesh, &SeamSet) -> Charted = chart;
}
