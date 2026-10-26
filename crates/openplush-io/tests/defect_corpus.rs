//! The defect corpus: every clean fixture validates with no defect, and every defect
//! fixture, read as OBJ, trips exactly its own detector. It lives here because
//! `openplush-io` already depends on `openplush-mesh`; the reverse would be a cycle.

use openplush_mesh::{ValidationReport, validate};
use openplush_testkit::fixtures::{self, Kind};

/// The detectors each defect fixture must trip, and no others, in report order; where it is
/// more than one, `research/fixtures_gen/README.md` says why.
const CORPUS: [(&str, &[&str]); 12] = [
    ("bowtie-vertex", &["non_manifold_vertices"]),
    ("duplicate-face", &["duplicates"]),
    ("flipped-normals", &["winding_conflicts"]),
    ("hole", &["boundary_loops"]),
    ("multi-component", &["multiple_components"]),
    ("non-manifold-edge", &["non_manifold_edges"]),
    (
        "non-orientable",
        &["winding_conflicts", "non_orientable", "boundary_loops"],
    ),
    (
        "opposite-duplicate-face",
        &["duplicates", "non_manifold_edges"],
    ),
    ("repeated-index-face", &["zero_area", "repeated_index"]),
    ("self-intersection", &["self_intersections"]),
    ("unwelded-split-vertices", &["unwelded"]),
    ("zero-area-face", &["zero_area"]),
];

/// Every non-empty list in the report, by name.
fn defects(report: &ValidationReport) -> Vec<&'static str> {
    let lists = [
        ("unwelded", report.unwelded().is_empty()),
        ("zero_area", report.zero_area().is_empty()),
        ("repeated_index", report.repeated_index().is_empty()),
        ("duplicates", report.duplicates().is_empty()),
        ("non_manifold_edges", report.non_manifold_edges().is_empty()),
        (
            "non_manifold_vertices",
            report.non_manifold_vertices().is_empty(),
        ),
        ("winding_conflicts", report.winding_conflicts().is_empty()),
        ("non_orientable", report.non_orientable().is_empty()),
        ("inside_out", report.inside_out().is_empty()),
        ("boundary_loops", report.boundary_loops().is_empty()),
        ("multiple_components", report.components().len() <= 1),
        ("self_intersections", report.self_intersections().is_empty()),
        ("unmeasurable", report.unmeasurable().is_none()),
    ];
    lists
        .iter()
        .filter(|(_, empty)| !empty)
        .map(|(name, _)| *name)
        .collect()
}

type Outcome<T> = Result<T, Box<dyn std::error::Error>>;

fn report(kind: Kind, name: &str) -> Outcome<ValidationReport> {
    let bytes = fixtures::load(kind, name)?;
    Ok(validate(&openplush_io::obj::read(&bytes)?))
}

#[test]
fn every_clean_fixture_validates_with_no_defect() -> Outcome<()> {
    for name in fixtures::names(Kind::Mesh)? {
        let report = report(Kind::Mesh, &name)?;
        assert_eq!(defects(&report), Vec::<&str>::new(), "{name}");
    }
    Ok(())
}

#[test]
fn every_defect_fixture_trips_exactly_its_own_detector() -> Outcome<()> {
    let names = fixtures::names(Kind::Defect)?;
    let listed: Vec<&str> = CORPUS.iter().map(|(name, _)| *name).collect();
    assert_eq!(
        names, listed,
        "the corpus and fixtures/defects must list the same models"
    );
    for (name, detectors) in CORPUS {
        assert_eq!(defects(&report(Kind::Defect, name)?), detectors, "{name}");
    }
    Ok(())
}
