//! Every clean fixture, read as OBJ, validates with no defect. It lives here because
//! `openplush-io` already depends on `openplush-mesh`; the reverse would be a cycle.

use openplush_mesh::validate;
use openplush_testkit::fixtures::{self, Kind};

#[test]
fn every_clean_fixture_validates_with_no_defect() {
    for name in fixtures::names(Kind::Mesh).expect("fixture directory") {
        let bytes = fixtures::load(Kind::Mesh, &name).expect("fixture");
        let soup = openplush_io::obj::read(&bytes).expect("fixtures read");
        let report = validate(&soup);
        let clean = report.unmeasurable().is_none()
            && report.unwelded().is_empty()
            && report.zero_area().is_empty()
            && report.repeated_index().is_empty()
            && report.duplicates().is_empty()
            && report.non_manifold_edges().is_empty()
            && report.non_manifold_vertices().is_empty()
            && report.winding_conflicts().is_empty()
            && report.non_orientable().is_empty()
            && report.inside_out().is_empty()
            && report.boundary_loops().is_empty()
            && report.components().len() == 1;
        assert!(clean, "{name}: {report:?}");
    }
}
