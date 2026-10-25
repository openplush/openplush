//! Match notches: on every side of every seam, at both ends and at the same fractions of its length,
//! at most 50 mm apart, with a mark count that differs from every seam it meets on a piece. The
//! sides are walked from the chart faces here, not by the stage.

use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::PI;

use openplush_io::obj;
use openplush_mesh::{RepairPlan, into_manifold};
use openplush_pipeline::{
    ChainSide, ChartMesh, DVec2, HalfEdgeId, ManifoldMesh, Parameterization, PatternInput,
    PatternOptions, PatternPiece, Scale, SeamChainId, SeamKind, SeamSet, SewingMap, VertexId,
    chart, flatten, pattern,
};

#[allow(dead_code, reason = "each test file uses part of the shared fixtures")]
mod common;

use common::{CAPSULE_OBJ, CAPSULE_SEAMS, CUBE_OBJ, CUBE_SEAMS, Setup, fabric, reference_seams};

/// Far below a millimetre, well above the rounding of a scaled layout.
const SAME_MM: f64 = 1e-6;

/// The longest gap allowed between neighbouring notches on one side.
const SPACING_MM: f64 = 50.0;

struct Run {
    mesh: ManifoldMesh,
    charts: Vec<ChartMesh>,
    sewing: SewingMap,
    layouts: Vec<Parameterization>,
    mm_per_unit: f64,
    pieces: Vec<PatternPiece>,
}

fn run(mesh: ManifoldMesh, seams: &SeamSet, mm_per_unit: f64) -> Result<Run, Setup> {
    let (charts, sewing) = chart(&mesh, seams)?.value;
    let layouts = flatten(&mesh, &charts)?.value;
    let input = PatternInput {
        mesh: &mesh,
        charts: &charts,
        sewing: &sewing,
        layouts: &layouts,
    };
    let opts = PatternOptions {
        scale: Scale::mm_per_model_unit(mm_per_unit)?,
        fabric: fabric(6.0)?,
    };
    let pieces = pattern(&input, &opts)?.value;
    Ok(Run {
        mesh,
        charts,
        sewing,
        layouts,
        mm_per_unit,
        pieces,
    })
}

fn mesh_of(obj: &str) -> Result<ManifoldMesh, Setup> {
    let soup = obj::read(obj.as_bytes())?;
    Ok(into_manifold(&soup, &RepairPlan::default())?.0)
}

fn reference(obj: &str, seams: &str, mm_per_unit: f64) -> Result<Run, Setup> {
    let mesh = mesh_of(obj)?;
    let set = reference_seams(obj, seams, &mesh)?;
    run(mesh, &set, mm_per_unit)
}

/// A spherical cap with a two-edge dart up a meridian from its rim.
fn dart() -> Result<Run, Setup> {
    let rings = 4;
    let mesh = mesh_of(&common::cap(rings))?;
    let on_meridian = |k: u32| -> Result<VertexId, Setup> {
        let polar = f64::from(k) * (PI / 3.0) / f64::from(rings);
        (0..u32::try_from(mesh.vertex_count())?)
            .map(VertexId::new)
            .find(|&v| {
                let p = mesh.position(v);
                p.y == 0.0 && (p.x - 10.0 * libm::sin(polar)).abs() < 1e-9 && p.x > 0.0
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
    run(mesh, &seams, 10.0)
}

/// One side laid out on its piece: the chart index and its points in mm in walking order.
struct Path {
    chart: usize,
    points: Vec<DVec2>,
}

impl Path {
    fn length(&self) -> f64 {
        self.points.windows(2).map(|w| (w[1] - w[0]).length()).sum()
    }

    /// The fraction of the length at which `p` lies on the path, if it lies on it.
    fn fraction_of(&self, p: DVec2) -> Option<f64> {
        let mut walked = 0.0;
        for w in self.points.windows(2) {
            let (a, b) = (w[0], w[1]);
            let len = (b - a).length();
            let t = ((p - a).dot(b - a) / (len * len)).clamp(0.0, 1.0);
            if (a + (b - a) * t - p).length() <= SAME_MM {
                return Some((walked + t * len) / self.length());
            }
            walked += len;
        }
        None
    }
}

/// The chart index and the chart vertices at the start and end of `h`.
fn flat_side(run: &Run, h: HalfEdgeId) -> Option<(usize, VertexId, VertexId)> {
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
    Some((c, corners[k], corners[(k + 1) % 3]))
}

fn path(run: &Run, side: &ChainSide) -> Result<Path, Setup> {
    let mut segments = Vec::new();
    for &h in side.half_edges() {
        let (c, a, b) = flat_side(run, h).ok_or("half-edge outside every chart")?;
        let at = |v: VertexId| run.layouts[c].uv()[v.index() as usize] * run.mm_per_unit;
        let flat = (at(b) - at(a)).length();
        let model = (run.mesh.position(run.mesh.dest(h)) - run.mesh.position(run.mesh.origin(h)))
            .length()
            * run.mm_per_unit;
        // Each edge keeps its 3D length, so an equal fraction is the same point of the seam.
        assert!(
            (flat - model).abs() <= f64::max(1.0, 0.005 * model),
            "edge flat {flat} mm, 3D {model} mm"
        );
        segments.push((c, at(a), at(b)));
    }
    let chart = segments.first().ok_or("a side with no edges")?.0;
    let forward = segments.len() < 2 || segments[0].2 == segments[1].1;
    let mut points = Vec::new();
    for (i, &(_, a, b)) in segments.iter().enumerate() {
        let (from, to) = if forward { (a, b) } else { (b, a) };
        if i == 0 {
            points.push(from);
        }
        points.push(to);
    }
    Ok(Path { chart, points })
}

fn inside(polygon: &[DVec2], p: DVec2) -> bool {
    let n = polygon.len();
    (0..n)
        .filter(|&i| {
            let (a, b) = (polygon[i], polygon[(i + 1) % n]);
            (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x)
        })
        .count()
        % 2
        == 1
}

/// Per piece and point on its sewing line, the chain and mark count of each side ending there.
type Ends = BTreeMap<(usize, [u64; 2]), Vec<(u32, u32)>>;

/// The fractions along `path` of `chain`'s notches on `piece` between the path's ends, sorted,
/// and the marks they all carry.
fn inner_notches(path: &Path, piece: &PatternPiece, chain: SeamChainId) -> (Vec<f64>, u32) {
    let (start, end) = (path.points[0], path.points[path.points.len() - 1]);
    let mut marks = None;
    let mut inner = Vec::new();
    for n in piece.notches().iter().filter(|n| n.chain() == chain) {
        let at_end = (n.at() - start).length() <= SAME_MM || (n.at() - end).length() <= SAME_MM;
        let Some(t) = path.fraction_of(n.at()).filter(|_| !at_end) else {
            continue;
        };
        assert_eq!(
            *marks.get_or_insert(n.marks()),
            n.marks(),
            "chain {chain:?}"
        );
        inner.push(t);
    }
    inner.sort_by(f64::total_cmp);
    assert!(
        !inner.is_empty(),
        "chain {chain:?} has no notch between its ends"
    );
    (inner, marks.unwrap_or(0))
}

fn assert_spaced(path: &Path, inner: &[f64], chain: SeamChainId) {
    let mut stops = vec![0.0];
    stops.extend(inner);
    stops.push(1.0);
    for w in stops.windows(2) {
        let gap = (w[1] - w[0]) * path.length();
        assert!(
            gap <= SPACING_MM + SAME_MM,
            "chain {chain:?}: notches {gap} mm apart"
        );
    }
}

fn assert_same_fractions(want: &[f64], got: &[f64], chain: SeamChainId) {
    assert_eq!(want.len(), got.len(), "chain {chain:?}");
    for (a, b) in want.iter().zip(got) {
        assert!((a - b).abs() <= 1e-9, "chain {chain:?}: {a} vs {b}");
    }
}

/// Every notch claim this file makes, for one run.
fn assert_notched(run: &Run) -> Result<(), Setup> {
    assert!(!run.sewing.bundles().is_empty());
    let mut ends = Ends::new();
    for bundle in run.sewing.bundles() {
        let chain = bundle.chain();
        let mut first: Option<Vec<f64>> = None;
        for side in bundle.sides() {
            let path = path(run, side)?;
            let piece = &run.pieces[path.chart];
            let (inner, marks) = inner_notches(&path, piece, chain);
            assert_spaced(&path, &inner, chain);
            match &first {
                None => first = Some(inner),
                Some(want) => assert_same_fractions(want, &inner, chain),
            }
            for p in [path.points[0], path.points[path.points.len() - 1]] {
                let at_end: Vec<_> = piece
                    .notches()
                    .iter()
                    .filter(|n| (n.at() - p).length() <= SAME_MM)
                    .collect();
                assert!(
                    !at_end.is_empty(),
                    "chain {chain:?}: no notch at its end {p:?}"
                );
                assert!(at_end.iter().all(|n| n.marks() == 1), "{at_end:?}");
                ends.entry((path.chart, [p.x.to_bits(), p.y.to_bits()]))
                    .or_default()
                    .push((chain.index(), marks));
            }
        }
    }
    assert_neighbours_differ(&ends);
    assert_pieces(run);
    Ok(())
}

/// Seams that meet on a piece differ, and each has exactly the least count, from 1, that no
/// lower seam it meets has: no more than it needs.
fn assert_neighbours_differ(ends: &Ends) {
    let mut meets: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    let mut marks = BTreeMap::new();
    for ((chart, _), chains) in ends {
        for (i, &(a, ma)) in chains.iter().enumerate() {
            marks.insert(a, ma);
            meets.entry(a).or_default();
            for &(b, mb) in &chains[i + 1..] {
                if a != b {
                    assert_ne!(ma, mb, "chains {a} and {b} meet on piece {chart}");
                    meets.entry(a).or_default().insert(b);
                    meets.entry(b).or_default().insert(a);
                }
            }
        }
    }
    let mut want: BTreeMap<u32, u32> = BTreeMap::new();
    for (&chain, others) in &meets {
        let taken: BTreeSet<u32> = others.iter().filter_map(|o| want.get(o).copied()).collect();
        let least = (1..)
            .find(|c| !taken.contains(c))
            .ok_or("a count")
            .unwrap_or(0);
        want.insert(chain, least);
    }
    assert_eq!(marks, want);
}

/// Each notch has marks, points out of its piece, and sits apart from the piece's others.
fn assert_pieces(run: &Run) {
    for piece in &run.pieces {
        let line = piece.sewing_line().points();
        for (i, n) in piece.notches().iter().enumerate() {
            assert!(n.marks() >= 1);
            assert!((n.outward().length() - 1.0).abs() <= 1e-9);
            assert!(
                !inside(line, n.at() + n.outward() * 0.5),
                "{n:?} points inward"
            );
            for m in &piece.notches()[i + 1..] {
                assert!(
                    (n.at() - m.at()).length() > SAME_MM,
                    "{n:?} and {m:?} coincide"
                );
            }
        }
    }
}

#[test]
fn every_capsule_seam_is_notched_alike_on_both_sides() -> Result<(), Setup> {
    let run = reference(CAPSULE_OBJ, CAPSULE_SEAMS, 5.0)?;
    assert_notched(&run)?;
    let most = run
        .pieces
        .iter()
        .flat_map(PatternPiece::notches)
        .filter(|n| n.marks() > 1)
        .count();
    assert!(most > 0, "no seam needs more than one mark");
    Ok(())
}

#[test]
fn every_cube_seam_is_notched_alike_on_both_sides() -> Result<(), Setup> {
    assert_notched(&reference(CUBE_OBJ, CUBE_SEAMS, 5.0)?)
}

#[test]
fn both_sides_of_a_dart_are_notched_alike() -> Result<(), Setup> {
    assert_notched(&dart()?)
}

#[test]
fn a_short_seam_still_gets_a_notch_between_its_ends() -> Result<(), Setup> {
    let run = reference(CAPSULE_OBJ, CAPSULE_SEAMS, 0.5)?;
    let short = run.sewing.bundles().iter().any(|b| {
        b.sides()
            .iter()
            .all(|s| path(&run, s).is_ok_and(|p| p.length() <= SPACING_MM))
    });
    assert!(short, "no seam of 50 mm or less");
    assert_notched(&run)
}

#[test]
fn the_same_input_notches_to_the_same_bits() -> Result<(), Setup> {
    let a = reference(CAPSULE_OBJ, CAPSULE_SEAMS, 5.0)?;
    let b = reference(CAPSULE_OBJ, CAPSULE_SEAMS, 5.0)?;
    let bits = |r: &Run| -> Vec<(u32, u64, u64, u32)> {
        r.pieces
            .iter()
            .flat_map(PatternPiece::notches)
            .map(|n| {
                (
                    n.chain().index(),
                    n.at().x.to_bits(),
                    n.at().y.to_bits(),
                    n.marks(),
                )
            })
            .collect()
    };
    assert_eq!(bits(&a), bits(&b));
    Ok(())
}
