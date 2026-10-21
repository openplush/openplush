//! Seam parity, which valid layouts cannot fail: a layout is stretched by hand to reach it.
#![allow(clippy::expect_used, clippy::panic, reason = "test setup")]

use openplush_geom::units::{Mm, Scale};
use openplush_io::obj;
use openplush_mesh::{ManifoldMesh, RepairPlan, VertexId, into_manifold};
use openplush_pattern::{Fabric, FabricCategory, FabricId, FabricSpec};
use openplush_seams::{ChartMesh, SeamKind, SeamSet, SewingMap};

use super::{PatternInput, PatternOptions, pattern};
use crate::{EngineError, Parameterization, chart, flatten};

/// Two 10-unit squares side by side, cut apart along the edge they share.
const PAIR: &str = "v 0 0 0\nv 10 0 0\nv 20 0 0\nv 0 10 0\nv 10 10 0\nv 20 10 0\n\
f 1 2 5\nf 1 5 4\nf 2 3 6\nf 2 6 5\n";

fn pair() -> (
    ManifoldMesh,
    Vec<ChartMesh>,
    SewingMap,
    Vec<Parameterization>,
) {
    let soup = obj::read(PAIR.as_bytes()).expect("the pair reads");
    let (mesh, _) = into_manifold(&soup, &RepairPlan::default()).expect("the pair repairs");
    let at = |x: f64, y: f64| {
        (0..mesh.vertex_count())
            .map(|v| VertexId::new(u32::try_from(v).expect("small")))
            .find(|&v| (mesh.position(v).x, mesh.position(v).y) == (x, y))
            .expect("a vertex there")
    };
    let (a, b) = (at(10.0, 0.0), at(10.0, 10.0));
    let shared = mesh
        .outgoing(a)
        .find(|&h| mesh.dest(h) == b)
        .expect("the shared edge")
        .edge();
    let mut seams = SeamSet::new();
    seams.insert(shared, SeamKind::Cut).expect("a cut");
    let (charts, sewing) = chart(&mesh, &seams).expect("two charts").value;
    let layouts = flatten(&mesh, &charts).expect("both flatten").value;
    (mesh, charts, sewing, layouts)
}

fn options() -> PatternOptions {
    let mm = |v| Mm::try_new(v).expect("a length");
    PatternOptions {
        scale: Scale::mm_per_model_unit(1.0).expect("a scale"),
        fabric: Fabric::new(FabricSpec {
            id: FabricId::new(0),
            name: "cotton".into(),
            category: FabricCategory::Woven,
            stretch_grain_pct: 0.0,
            stretch_cross_pct: 0.0,
            stretch_bias_pct: 0.0,
            nap: false,
            pile_len: mm(0.0),
            thickness: mm(0.3),
            frays: true,
            default_allowance: mm(6.0),
            roll_width: mm(1500.0),
        })
        .expect("a fabric"),
    }
}

/// The pair with the second chart's layout scaled by `x` across and `y` up.
fn stretched(x: f64, y: f64) -> Result<(), EngineError> {
    let (mesh, charts, sewing, mut layouts) = pair();
    for p in &mut layouts[1].uv {
        p.x *= x;
        p.y *= y;
    }
    let input = PatternInput {
        mesh: &mesh,
        charts: &charts,
        sewing: &sewing,
        layouts: &layouts,
    };
    pattern(&input, &options()).map(|_| ())
}

fn lengths(e: &EngineError) -> [f64; 3] {
    let EngineError::SeamParity {
        a_mm,
        b_mm,
        tolerance_mm,
        ..
    } = e
    else {
        panic!("SeamParity, not {e:?}");
    };
    let (short, long) = if a_mm < b_mm {
        (a_mm, b_mm)
    } else {
        (b_mm, a_mm)
    };
    [*short, *long, *tolerance_mm]
}

/// Equal within rounding of a 2-triangle flatten.
fn assert_lengths(e: &EngineError, want: [f64; 3]) {
    let got = lengths(e);
    assert!(
        got.iter().zip(want).all(|(g, w)| (g - w).abs() < 1e-9),
        "{got:?} against {want:?}"
    );
}

#[test]
fn the_unstretched_pair_passes() {
    stretched(1.0, 1.0).expect("valid layouts pass parity");
}

#[test]
fn a_side_half_again_as_long_is_seam_parity_with_all_four_lengths() {
    let e = stretched(1.5, 1.5).expect_err("10 against 15 mm");
    assert_lengths(&e, [10.0, 15.0, 1.0]);
    assert_eq!(e.locations().chains.len(), 1);
}

#[test]
fn a_side_within_the_tolerance_passes() {
    stretched(1.09, 1.09).expect("10 against 10.9 mm is within 1 mm");
}

#[test]
fn parity_is_checked_before_any_outline() {
    // Mirrored, the second chart's outline is clockwise, which the outline check would refuse.
    let e = stretched(-1.5, 1.5).expect_err("parity first");
    assert_lengths(&e, [10.0, 15.0, 1.0]);
}
