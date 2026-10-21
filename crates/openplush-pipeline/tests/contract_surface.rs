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

#[test]
fn import_takes_a_file_set_and_returns_a_soup() {
    use openplush_pipeline::{ImportFiles, TriangleSoup, import};
    let _: fn(&ImportFiles) -> StageResult<TriangleSoup> = import;
}

#[test]
fn pattern_takes_the_charts_with_their_layouts_and_options_and_returns_pieces() {
    use openplush_pipeline::{PatternInput, PatternOptions, PatternPiece, pattern};
    let _: fn(&PatternInput<'_>, &PatternOptions) -> StageResult<Vec<PatternPiece>> = pattern;
}

#[test]
fn size_takes_the_mesh_and_a_target_and_returns_the_scale() {
    use openplush_pipeline::{Scale, SizeTarget, size};
    let _: fn(&ManifoldMesh, SizeTarget) -> StageResult<Scale> = size;
}
