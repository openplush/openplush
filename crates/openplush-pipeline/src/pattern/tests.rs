//! Seam parity, which valid layouts cannot fail, a chart no flatten would lay out, and the squeeze
//! warning's threshold and text: layouts are stretched or written by hand to reach them.
#![allow(clippy::expect_used, clippy::panic, reason = "test setup")]

use std::fmt::Write as _;

use openplush_geom::DVec2;
use openplush_geom::units::{Mm, Scale};
use openplush_io::obj;
use openplush_mesh::{ManifoldMesh, RepairPlan, VertexId, into_manifold};
use openplush_pattern::{Fabric, FabricCategory, FabricId, FabricSpec, PatternError, PieceId};
use openplush_seams::{ChartId, ChartMesh, SeamGraph, SeamKind, SeamSet, SewingMap, charts};

use super::{PatternInput, PatternOptions, needed_stretch, pattern, squeeze_warning};
use crate::{EngineError, Parameterization, Remedy, WarningTarget, chart, flatten};

/// One run's chart and flatten outputs, for its layouts to be changed by hand.
struct Run {
    mesh: ManifoldMesh,
    charts: Vec<ChartMesh>,
    sewing: SewingMap,
    layouts: Vec<Parameterization>,
}

impl Run {
    fn pattern(&self, mm_per_unit: f64) -> Result<(), EngineError> {
        let input = PatternInput {
            mesh: &self.mesh,
            charts: &self.charts,
            sewing: &self.sewing,
            layouts: &self.layouts,
        };
        pattern(&input, &options(mm_per_unit)).map(|_| ())
    }

    /// Scales chart `chart`'s layout by `x` across and `y` up.
    fn stretch(&mut self, chart: ChartId, x: f64, y: f64) {
        let layout = self
            .layouts
            .iter_mut()
            .find(|l| l.chart() == chart)
            .expect("a layout for the chart");
        for p in layout.uv_mut() {
            p.x *= x;
            p.y *= y;
        }
    }
}

fn mesh_of(obj: &str) -> ManifoldMesh {
    let soup = obj::read(obj.as_bytes()).expect("the mesh reads");
    into_manifold(&soup, &RepairPlan::default())
        .expect("the mesh repairs")
        .0
}

/// Two columns of `rows` 10-unit squares, cut apart along the `rows` edges between them.
fn pair(rows: u32) -> Run {
    let mut s = String::new();
    for r in 0..=rows {
        for c in 0..3 {
            let _ = writeln!(s, "v {} {} 0", 10 * c, 10 * r);
        }
    }
    let id = |c: u32, r: u32| r * 3 + c + 1;
    for r in 0..rows {
        for c in 0..2 {
            let (a, b, d, e) = (id(c, r), id(c + 1, r), id(c + 1, r + 1), id(c, r + 1));
            let _ = writeln!(s, "f {a} {b} {d}\nf {a} {d} {e}");
        }
    }
    let mesh = mesh_of(&s);
    let at = |y: f64| {
        (0..mesh.vertex_count())
            .map(|v| VertexId::new(u32::try_from(v).expect("small")))
            .find(|&v| (mesh.position(v).x, mesh.position(v).y) == (10.0, y))
            .expect("a vertex there")
    };
    let mut seams = SeamSet::new();
    for r in 0..rows {
        let (a, b) = (at(f64::from(10 * r)), at(f64::from(10 * (r + 1))));
        let edge = mesh
            .outgoing(a)
            .find(|&h| mesh.dest(h) == b)
            .expect("an edge");
        seams.insert(edge.edge(), SeamKind::Cut).expect("a cut");
    }
    run(mesh, &seams)
}

fn run(mesh: ManifoldMesh, seams: &SeamSet) -> Run {
    let (charts, sewing) = chart(&mesh, seams).expect("charts").value;
    let layouts = flatten(&mesh, &charts).expect("every chart flattens").value;
    Run {
        mesh,
        charts,
        sewing,
        layouts,
    }
}

/// Three 10 by 10 flaps sharing one edge, a fin, which repair splits into copies sewn as one
/// bundle of three sides.
fn fin() -> Run {
    let mesh = mesh_of(
        "v 0 0 0\nv 0 0 10\nv 10 0 0\nv 10 0 10\nv -5 9 0\nv -5 9 10\nv -5 -9 0\nv -5 -9 10\n\
         f 1 3 4\nf 1 4 2\nf 2 6 5\nf 2 5 1\nf 1 7 8\nf 1 8 2\n",
    );
    let mut seams = SeamSet::new();
    for &e in mesh.edge_copies().iter().flatten() {
        seams.insert(e, SeamKind::Mandatory).expect("a repair seam");
    }
    run(mesh, &seams)
}

fn options(mm_per_unit: f64) -> PatternOptions {
    let mm = |v| Mm::try_new(v).expect("a length");
    PatternOptions {
        scale: Scale::mm_per_model_unit(mm_per_unit).expect("a scale"),
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

/// `SeamParity`'s two lengths, shorter first, and its tolerance, within rounding of a flatten.
fn assert_parity(result: Result<(), EngineError>, want: [f64; 3]) {
    let Err(EngineError::SeamParity {
        a_mm,
        b_mm,
        tolerance_mm,
        ..
    }) = result
    else {
        panic!("SeamParity, not {result:?}");
    };
    let got = [a_mm.min(b_mm), a_mm.max(b_mm), tolerance_mm];
    assert!(
        got.iter().zip(want).all(|(g, w)| (g - w).abs() < 1e-9),
        "{got:?} against {want:?}"
    );
}

#[test]
fn the_unstretched_pair_passes() {
    pair(1).pattern(1.0).expect("valid layouts pass parity");
}

#[test]
fn a_side_half_again_as_long_is_seam_parity_with_all_four_lengths() {
    let mut run = pair(1);
    run.stretch(run.charts[1].chart(), 1.5, 1.5);
    let result = run.pattern(1.0);
    assert!(matches!(&result, Err(e) if e.locations().chains.len() == 1));
    assert_parity(result, [10.0, 15.0, 1.0]);
}

#[test]
fn a_side_within_the_tolerance_passes() {
    let mut run = pair(1);
    run.stretch(run.charts[1].chart(), 1.09, 1.09);
    run.pattern(1.0).expect("10 against 10.9 mm is within 1 mm");
}

#[test]
fn parity_is_judged_in_millimetres_after_the_scale() {
    let mut run = pair(1);
    run.stretch(run.charts[1].chart(), 1.09, 1.09);
    assert_parity(run.pattern(10.0), [100.0, 109.0, 1.0]);
}

#[test]
fn a_side_s_length_is_every_edge_along_it() {
    let mut run = pair(2);
    run.stretch(run.charts[1].chart(), 1.5, 1.5);
    assert_parity(run.pattern(1.0), [20.0, 30.0, 1.0]);
}

#[test]
fn every_side_of_a_bundle_is_held_to_its_first_side() {
    let mut run = fin();
    let [first, second, third] = run.sewing.bundles()[0].sides() else {
        panic!("a fin is one bundle of three sides");
    };
    let charts = [first.chart(), second.chart(), third.chart()];
    run.pattern(1.0).expect("the fin as flattened passes");
    // Each side is within the tolerance of the one before it, but the third is not of the first.
    run.stretch(charts[1], 1.09, 1.09);
    run.stretch(charts[2], 1.18, 1.18);
    assert_parity(run.pattern(1.0), [10.0, 11.8, 1.0]);
}

#[test]
fn parity_is_checked_before_any_outline() {
    // Mirrored, the second chart's outline is clockwise, which the outline check would refuse.
    let mut run = pair(1);
    run.stretch(run.charts[1].chart(), -1.5, 1.5);
    assert_parity(run.pattern(1.0), [10.0, 15.0, 1.0]);
}

#[test]
fn a_side_too_long_for_the_scale_is_refused_naming_its_piece() {
    let result = pair(1).pattern(1e308);
    assert!(
        matches!(
            result,
            Err(EngineError::Pattern {
                piece: Some(_),
                source: PatternError::BadScale { .. },
            })
        ),
        "{result:?}"
    );
}

#[test]
fn a_chart_with_two_boundary_loops_cannot_be_its_layout_s() {
    // A 30-unit square with a 10-unit hole: one chart, two boundary loops, so no flatten lays it
    // out; its layout is written by hand.
    let mesh = mesh_of(
        "v 0 0 0\nv 30 0 0\nv 30 30 0\nv 0 30 0\nv 10 10 0\nv 20 10 0\nv 20 20 0\nv 10 20 0\n\
         f 1 2 6\nf 1 6 5\nf 2 3 7\nf 2 7 6\nf 3 4 8\nf 3 8 7\nf 4 1 5\nf 4 5 8\n",
    );
    let seams = SeamSet::new();
    let found = charts(&mesh, &seams).expect("one chart");
    let ring = ChartMesh::new(&mesh, &seams, &found[0]);
    let graph = SeamGraph::with_copies(&mesh, &seams, mesh.edge_copies()).expect("no seams");
    let sewing = SewingMap::new(&mesh, &seams, &graph, mesh.edge_copies()).expect("no bundles");
    let layout = Parameterization::by_hand(
        ring.chart(),
        ring.xyz().iter().map(|p| DVec2::new(p.x, p.y)).collect(),
    );
    let input = PatternInput {
        mesh: &mesh,
        charts: std::slice::from_ref(&ring),
        sewing: &sewing,
        layouts: std::slice::from_ref(&layout),
    };
    assert_eq!(
        pattern(&input, &options(1.0)).map(|_| ()),
        Err(EngineError::StageInputMismatch {
            chart: ring.chart()
        })
    );
}

#[test]
fn a_piece_needing_more_stretch_than_its_fabric_warns_with_both_and_one_needing_less_does_not() {
    let woven = options(1.0).fabric;
    let layout = |deficit| Parameterization {
        compression_deficit: deficit,
        ..Parameterization::by_hand(ChartId::new(4), Vec::new())
    };
    assert_eq!(needed_stretch(0.2), 0.25);
    assert_eq!(squeeze_warning(&layout(0.04), woven.spec()), None);
    let w = squeeze_warning(&layout(0.364), woven.spec()).expect("a warning");
    assert_eq!(w.code().as_str(), "piece-compressed");
    assert_eq!(w.target(), WarningTarget::Piece(PieceId::new(4)));
    assert_eq!(
        w.explanation(),
        "This piece must stretch about 57% to fill its shape, more than this fabric's 5%, so it \
         will sew up flatter than the model."
    );
    let Remedy::Advisory { what_to_change } = w.remedy() else {
        panic!("advisory: {:?}", w.remedy());
    };
    assert_eq!(
        what_to_change.as_str(),
        "Add a dart or a seam in this piece."
    );
}

#[test]
fn the_fabric_gives_its_lesser_stretch_never_under_5_percent_and_equal_need_does_not_warn() {
    let fabric = |grain: f64, cross: f64| {
        Fabric::new(FabricSpec {
            stretch_grain_pct: grain,
            stretch_cross_pct: cross,
            ..options(1.0).fabric.spec().clone()
        })
        .expect("a fabric")
    };
    let warns = |deficit, grain, cross| {
        let layout = Parameterization {
            compression_deficit: deficit,
            ..Parameterization::by_hand(ChartId::new(4), Vec::new())
        };
        squeeze_warning(&layout, fabric(grain, cross).spec()).map(|w| w.explanation().to_owned())
    };
    // A deficit of 0.2 needs exactly 25%.
    assert_eq!(warns(0.2, 25.0, 25.0), None);
    assert!(warns(0.2, 24.0, 24.0).is_some());
    assert_eq!(
        warns(0.2, 10.0, 30.0).as_deref(),
        Some(
            "This piece must stretch about 25% to fill its shape, more than this fabric's 10%, \
             so it will sew up flatter than the model."
        )
    );
    // A deficit of 0.04 needs about 4.2%: under the 5% floor, over 3%.
    assert_eq!(warns(0.04, 3.0, 3.0), None);
    assert_eq!(
        warns(0.364, 3.0, 3.0).as_deref(),
        Some(
            "This piece must stretch about 57% to fill its shape, more than this fabric's 5%, so \
             it will sew up flatter than the model."
        )
    );
}
