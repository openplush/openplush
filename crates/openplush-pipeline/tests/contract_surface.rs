//! The stage signatures in the engine API contract compile and are callable as documented.

use openplush_pipeline::{ChartMesh, ManifoldMesh, SeamSet, SewingMap, StageResult, chart};

type Charted = StageResult<(Vec<ChartMesh>, SewingMap)>;

#[test]
fn chart_takes_a_mesh_and_seams_and_returns_charts_with_their_sewing_map() {
    let _: fn(&ManifoldMesh, &SeamSet) -> Charted = chart;
}

#[test]
fn validate_and_repair_take_a_soup_and_return_its_report_and_mesh() {
    use openplush_pipeline::{
        RepairPlan, RepairReport, Staged, TriangleSoup, ValidationReport, repair, validate,
    };
    let _: fn(&TriangleSoup) -> Staged<ValidationReport> = validate;
    let _: fn(&TriangleSoup, &RepairPlan) -> StageResult<(ManifoldMesh, RepairReport)> = repair;
}
