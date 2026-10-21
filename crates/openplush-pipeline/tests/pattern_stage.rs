//! The pattern stage: one piece per chart in millimetres, its sewing line from the chart's flat
//! boundary and its cut line at the fabric's allowance; every seam's sides checked first; inputs
//! from different runs refused. Meshes are written as OBJ and repaired, as a shell would.

use std::f64::consts::{PI, TAU};
use std::fmt::Write as _;

use openplush_io::obj;
use openplush_mesh::{RepairPlan, into_manifold};
use openplush_pipeline::{
    ChartId, ChartMesh, DVec2, EngineError, Fabric, FabricCategory, FabricId, FabricSpec,
    ManifoldMesh, Mm, Parameterization, PatternError, PatternInput, PatternOptions, PatternPiece,
    PieceId, Scale, SeamKind, SeamSet, SewingMap, VertexId, chart, flatten, pattern,
};

type Setup = Box<dyn std::error::Error>;

const CAPSULE_OBJ: &str = include_str!("../../../fixtures/meshes/capsule.obj");
const CAPSULE_SEAMS: &str = include_str!("../../../fixtures/seams/capsule.json");

/// Well above the rounding of a 2-triangle flatten scaled to 100 mm, far below a millimetre.
const SAME_MM: f64 = 1e-6;

fn mesh_of(obj: &str) -> Result<ManifoldMesh, Setup> {
    let soup = obj::read(obj.as_bytes())?;
    Ok(into_manifold(&soup, &RepairPlan::default())?.0)
}

fn fabric(allowance_mm: f64) -> Result<Fabric, Setup> {
    Ok(Fabric::new(FabricSpec {
        id: FabricId::new(0),
        name: "cotton".into(),
        category: FabricCategory::Woven,
        stretch_grain_pct: 0.0,
        stretch_cross_pct: 0.0,
        stretch_bias_pct: 0.0,
        nap: false,
        pile_len: Mm::try_new(0.0)?,
        thickness: Mm::try_new(0.3)?,
        frays: true,
        default_allowance: Mm::try_new(allowance_mm)?,
        roll_width: Mm::try_new(1500.0)?,
    })?)
}

fn options(mm_per_unit: f64, allowance_mm: f64) -> Result<PatternOptions, Setup> {
    Ok(PatternOptions {
        scale: Scale::mm_per_model_unit(mm_per_unit)?,
        fabric: fabric(allowance_mm)?,
    })
}

/// A chart run through the chart and flatten stages.
struct Run {
    mesh: ManifoldMesh,
    charts: Vec<ChartMesh>,
    sewing: SewingMap,
    layouts: Vec<Parameterization>,
}

impl Run {
    fn of(mesh: ManifoldMesh, seams: &SeamSet) -> Result<Self, Setup> {
        let (charts, sewing) = chart(&mesh, seams)?.value;
        let layouts = flatten(&mesh, &charts)?.value;
        Ok(Self {
            mesh,
            charts,
            sewing,
            layouts,
        })
    }

    fn input(&self) -> PatternInput<'_> {
        PatternInput {
            mesh: &self.mesh,
            charts: &self.charts,
            sewing: &self.sewing,
            layouts: &self.layouts,
        }
    }
}

fn edge_lengths(points: &[DVec2]) -> Vec<f64> {
    let n = points.len();
    (0..n)
        .map(|i| (points[(i + 1) % n] - points[i]).length())
        .collect()
}

fn area(points: &[DVec2]) -> f64 {
    let n = points.len();
    (0..n)
        .map(|i| points[i].perp_dot(points[(i + 1) % n]))
        .sum::<f64>()
        / 2.0
}

#[test]
fn a_unit_square_at_100_mm_a_unit_is_100_mm_square_and_112_mm_cut_at_6_mm() -> Result<(), Setup> {
    let run = Run::of(
        mesh_of("v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3\nf 1 3 4\n")?,
        &SeamSet::new(),
    )?;
    let opts = options(100.0, 6.0)?;
    let [piece] = &pattern(&run.input(), &opts)?.value[..] else {
        panic!("one piece");
    };
    assert_eq!(piece.id(), PieceId::new(0));
    let sewing = edge_lengths(piece.sewing_line().points());
    assert_eq!(sewing.len(), 4, "{sewing:?}");
    assert!(
        sewing.iter().all(|l| (l - 100.0).abs() < SAME_MM),
        "{sewing:?}"
    );
    // The overlay may keep a corner from a rounding-sized jog along a side, so the cut line is
    // judged by its perimeter and area, not its point count.
    let cut = piece.cut_line().points();
    let perimeter: f64 = edge_lengths(cut).iter().sum();
    assert!((perimeter - 4.0 * 112.0).abs() < SAME_MM, "{cut:?}");
    assert!((area(cut) - 112.0 * 112.0).abs() < SAME_MM, "{cut:?}");
    assert_eq!(piece.cuts().as_printed(), 1);
    assert_eq!(piece.cuts().flipped(), 0);
    assert_eq!(piece.rotations_deg(), opts.fabric.rotations_deg());
    Ok(())
}

#[test]
fn the_sewing_line_starts_at_the_first_boundary_side_of_the_lowest_face() -> Result<(), Setup> {
    let run = Run::of(
        mesh_of("v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3\nf 1 3 4\n")?,
        &SeamSet::new(),
    )?;
    let [piece] = &pattern(&run.input(), &options(100.0, 6.0)?)?.value[..] else {
        panic!("one piece");
    };
    // The square's diagonal is its one interior edge; face 0's first side not on it starts the line.
    let faces = run.charts[0].faces();
    let interior = |a: VertexId, b: VertexId| {
        faces
            .iter()
            .filter(|(_, c)| c.contains(&a) && c.contains(&b))
            .count()
            == 2
    };
    let [a, b, c] = faces[0].1;
    let (start, _) = [(a, b), (b, c), (c, a)]
        .into_iter()
        .find(|&(p, q)| !interior(p, q))
        .ok_or("face 0 has a boundary side")?;
    let want = run.layouts[0].uv()[start.index() as usize] * 100.0;
    assert_eq!(piece.sewing_line().points()[0], want);
    Ok(())
}

#[test]
fn a_layout_beyond_the_charts_is_refused_as_a_mismatch() -> Result<(), Setup> {
    let run = Run::of(
        mesh_of("v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3\nf 1 3 4\n")?,
        &SeamSet::new(),
    )?;
    let twice = [run.layouts[0].clone(), run.layouts[0].clone()];
    let input = PatternInput {
        layouts: &twice,
        ..run.input()
    };
    assert_eq!(
        pattern(&input, &options(1.0, 6.0)?).expect_err("an extra layout"),
        EngineError::StageInputMismatch {
            chart: ChartId::new(0)
        }
    );
    Ok(())
}

#[test]
fn no_charts_make_no_pieces() -> Result<(), Setup> {
    let mesh = mesh_of("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n")?;
    let (_, sewing) = chart(&mesh, &SeamSet::new())?.value;
    let input = PatternInput {
        mesh: &mesh,
        charts: &[],
        sewing: &sewing,
        layouts: &[],
    };
    assert!(pattern(&input, &options(1.0, 6.0)?)?.value.is_empty());
    Ok(())
}

/// The capsule with its reference seams; seam edges name OBJ vertices by file order, matched to
/// the repaired mesh by position.
fn capsule() -> Result<Run, Setup> {
    let mesh = mesh_of(CAPSULE_OBJ)?;
    let positions: Vec<[f64; 3]> = CAPSULE_OBJ
        .lines()
        .filter_map(|line| line.strip_prefix("v "))
        .map(|rest| {
            let c: Vec<f64> = rest
                .split_whitespace()
                .map(str::parse)
                .collect::<Result<_, _>>()?;
            Ok([c[0], c[1], c[2]])
        })
        .collect::<Result<_, std::num::ParseFloatError>>()?;
    let vertex = |i: usize| -> Result<VertexId, Setup> {
        (0..u32::try_from(mesh.vertex_count())?)
            .map(VertexId::new)
            .find(|&v| {
                let p = mesh.position(v);
                [p.x, p.y, p.z] == positions[i]
            })
            .ok_or_else(|| format!("no vertex for OBJ index {i}").into())
    };
    let mut seams = SeamSet::new();
    for pair in CAPSULE_SEAMS.split("\"edge\": [").skip(1) {
        let (a, b) = pair
            .split(']')
            .next()
            .and_then(|s| s.split_once(','))
            .ok_or("malformed edge")?;
        let (a, b) = (vertex(a.trim().parse()?)?, vertex(b.trim().parse()?)?);
        let edge = mesh
            .outgoing(a)
            .find(|&h| mesh.dest(h) == b)
            .ok_or("no such edge")?
            .edge();
        seams.insert(edge, SeamKind::Cut)?;
    }
    Run::of(mesh, &seams)
}

#[test]
fn the_capsule_makes_one_piece_per_chart_in_chart_order() -> Result<(), Setup> {
    let run = capsule()?;
    let pieces = pattern(&run.input(), &options(1.0, 6.0)?)?.value;
    let ids: Vec<u32> = pieces.iter().map(|p| p.id().index()).collect();
    let charts: Vec<u32> = run.charts.iter().map(|c| c.chart().index()).collect();
    assert_eq!(ids, charts);
    assert_eq!(ids.len(), 3);
    Ok(())
}

/// Where the chart vertex at the start or end of `h` lies on its piece's sewing line, found by
/// the test from the chart's faces rather than by the stage.
fn flat_end(run: &Run, h: openplush_pipeline::HalfEdgeId, end: usize) -> Option<(usize, VertexId)> {
    let face = run.mesh.face(h)?;
    let c = run
        .charts
        .iter()
        .position(|c| c.faces().iter().any(|(f, _)| *f == face))?;
    let (_, corners) = run.charts[c].faces().iter().find(|(f, _)| *f == face)?;
    let first = run.mesh.face_half_edge(face);
    let k = [first, run.mesh.next(first), run.mesh.prev(first)]
        .iter()
        .position(|&s| s == h)?;
    Some((c, corners[(k + end) % 3]))
}

#[test]
fn every_capsule_seam_s_sides_on_the_pieces_match_its_3d_length() -> Result<(), Setup> {
    let run = capsule()?;
    let mm_per_unit = 2.0;
    let pieces: Vec<PatternPiece> = pattern(&run.input(), &options(mm_per_unit, 6.0)?)?.value;
    assert!(!run.sewing.bundles().is_empty());
    for bundle in run.sewing.bundles() {
        for side in bundle.sides() {
            let (mut flat, mut model) = (0.0, 0.0);
            for &h in side.half_edges() {
                let (c, a) = flat_end(&run, h, 0).ok_or("half-edge outside every chart")?;
                let (_, b) = flat_end(&run, h, 1).ok_or("half-edge outside every chart")?;
                let at = |v: VertexId| run.layouts[c].uv()[v.index() as usize] * mm_per_unit;
                let line = pieces[c].sewing_line().points();
                assert!(line.contains(&at(a)) && line.contains(&at(b)));
                flat += (at(b) - at(a)).length();
                model += (run.mesh.position(run.mesh.dest(h))
                    - run.mesh.position(run.mesh.origin(h)))
                .length();
            }
            let want = model * mm_per_unit;
            assert!(
                (flat - want).abs() <= f64::max(1.0, 0.005 * want),
                "chain {}: flat {flat} mm, 3D {want} mm",
                bundle.chain().index()
            );
        }
    }
    Ok(())
}

#[test]
fn layouts_in_another_order_are_refused_as_a_mismatch() -> Result<(), Setup> {
    let run = capsule()?;
    let reversed: Vec<Parameterization> = run.layouts.iter().rev().cloned().collect();
    let input = PatternInput {
        layouts: &reversed,
        ..run.input()
    };
    let err = pattern(&input, &options(1.0, 6.0)?).expect_err("reordered layouts");
    assert_eq!(
        err,
        EngineError::StageInputMismatch {
            chart: ChartId::new(0)
        }
    );
    Ok(())
}

#[test]
fn layouts_or_a_sewing_map_from_another_mesh_are_refused_as_a_mismatch() -> Result<(), Setup> {
    let run = capsule()?;
    let square = Run::of(
        mesh_of("v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3\nf 1 3 4\n")?,
        &SeamSet::new(),
    )?;
    let opts = options(1.0, 6.0)?;
    let foreign_layouts = PatternInput {
        layouts: &square.layouts,
        ..run.input()
    };
    assert_eq!(
        pattern(&foreign_layouts, &opts).expect_err("the square's layouts"),
        EngineError::StageInputMismatch {
            chart: ChartId::new(0)
        }
    );
    let foreign_sewing = PatternInput {
        sewing: &run.sewing,
        ..square.input()
    };
    assert!(matches!(
        pattern(&foreign_sewing, &opts),
        Err(EngineError::StageInputMismatch { .. })
    ));
    Ok(())
}

/// A strip `width` wide winding 1.5 turns up a helix of radius 10, rising 3 a turn: it lays flat
/// into an annular strip that goes round more than once, so its flat boundary overlaps itself.
fn helical_ramp() -> String {
    let (steps, per_turn, width) = (108, 72.0, 2.0);
    let mut s = String::new();
    for k in 0..=steps {
        let t = f64::from(k) * TAU / per_turn;
        for r in [10.0, 10.0 + width] {
            let _ = writeln!(s, "v {} {} {}", r * t.cos(), r * t.sin(), 3.0 * t / TAU);
        }
    }
    for k in 0..steps {
        let (i, o, i2, o2) = (2 * k + 1, 2 * k + 2, 2 * k + 3, 2 * k + 4);
        let _ = writeln!(s, "f {i} {o} {o2}\nf {i} {o2} {i2}");
    }
    s
}

#[test]
fn a_piece_whose_flat_boundary_overlaps_itself_is_refused_naming_the_piece() -> Result<(), Setup> {
    let run = Run::of(mesh_of(&helical_ramp())?, &SeamSet::new())?;
    let err = pattern(&run.input(), &options(1.0, 6.0)?).expect_err("an overlapping piece");
    assert!(
        matches!(
            err,
            EngineError::Pattern {
                piece: Some(p),
                source: PatternError::OutlineNotSimple { .. },
            } if p == PieceId::new(0)
        ),
        "{err:?}"
    );
    Ok(())
}

/// A spherical cap of radius 10 out to 60° from its pole, in `rings` rings of 16, with the rim
/// vertex at angle 0 first in its ring.
fn cap(rings: u32) -> String {
    let n = 16;
    let mut s = String::from("v 0 0 10\n");
    for k in 1..=rings {
        let polar = f64::from(k) * (PI / 3.0) / f64::from(rings);
        for j in 0..n {
            let a = f64::from(j) * TAU / f64::from(n);
            let (x, y, z) = (polar.sin() * a.cos(), polar.sin() * a.sin(), polar.cos());
            let _ = writeln!(s, "v {} {} {}", 10.0 * x, 10.0 * y, 10.0 * z);
        }
    }
    let at = |k: u32, j: u32| if k == 0 { 1 } else { 2 + (k - 1) * n + j % n };
    for j in 0..n {
        let _ = writeln!(s, "f {} {} {}", at(0, 0), at(1, j), at(1, j + 1));
    }
    for k in 1..rings {
        for j in 0..n {
            let (a, b, c, d) = (at(k, j), at(k + 1, j), at(k + 1, j + 1), at(k, j + 1));
            let _ = writeln!(s, "f {a} {b} {c}\nf {a} {c} {d}");
        }
    }
    s
}

#[test]
fn a_dart_on_a_curved_piece_opens_into_a_wedge() -> Result<(), Setup> {
    let rings = 4;
    let mesh = mesh_of(&cap(rings))?;
    let on_meridian = |k: u32| -> Result<VertexId, Setup> {
        let polar = f64::from(k) * (PI / 3.0) / f64::from(rings);
        (0..u32::try_from(mesh.vertex_count())?)
            .map(VertexId::new)
            .find(|&v| {
                let p = mesh.position(v);
                p.y == 0.0 && (p.x - 10.0 * polar.sin()).abs() < 1e-9 && p.x > 0.0
            })
            .ok_or_else(|| format!("no meridian vertex on ring {k}").into())
    };
    let mut seams = SeamSet::new();
    for k in [rings, rings - 1] {
        let (a, b) = (on_meridian(k)?, on_meridian(k - 1)?);
        let edge = mesh
            .outgoing(a)
            .find(|&h| mesh.dest(h) == b)
            .ok_or("no meridian edge")?
            .edge();
        seams.insert(edge, SeamKind::Dart)?;
    }
    let rim = on_meridian(rings)?;
    let run = Run::of(mesh, &seams)?;
    let [piece] = &pattern(&run.input(), &options(10.0, 6.0)?)?.value[..] else {
        panic!("one piece");
    };
    // The rim vertex on the dart is two chart vertices, one per side of the slit; open, they lie
    // apart on the sewing line.
    let sides: Vec<DVec2> = run.charts[0]
        .vtx_to_global()
        .iter()
        .enumerate()
        .filter(|(_, g)| **g == rim)
        .map(|(l, _)| run.layouts[0].uv()[l] * 10.0)
        .collect();
    let [a, b] = sides[..] else {
        panic!("the rim vertex on the dart is two chart vertices: {sides:?}");
    };
    assert!((a - b).length() > 1.0, "the dart stays shut: {a:?} {b:?}");
    let line = piece.sewing_line().points();
    assert!(line.contains(&a) && line.contains(&b));
    Ok(())
}
