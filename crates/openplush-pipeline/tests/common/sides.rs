//! Stage runs through `pattern`, and each seam side walked from the chart faces, independent of
//! the stage, for the notch and label tests.

use std::f64::consts::PI;

use openplush_io::obj;
use openplush_mesh::{RepairPlan, into_manifold};
use openplush_pipeline::{
    ChainSide, ChartMesh, DVec2, HalfEdgeId, ManifoldMesh, Parameterization, PatternInput,
    PatternOptions, PatternPiece, Scale, SeamKind, SeamSet, SewingMap, VertexId, chart, flatten,
    pattern,
};

use super::{Setup, fabric, reference_seams};

/// Far below a millimetre, well above the rounding of a scaled layout.
pub(crate) const SAME_MM: f64 = 1e-6;

pub(crate) struct Run {
    pub(crate) mesh: ManifoldMesh,
    pub(crate) charts: Vec<ChartMesh>,
    pub(crate) sewing: SewingMap,
    pub(crate) layouts: Vec<Parameterization>,
    pub(crate) mm_per_unit: f64,
    pub(crate) pieces: Vec<PatternPiece>,
}

pub(crate) fn run(mesh: ManifoldMesh, seams: &SeamSet, mm_per_unit: f64) -> Result<Run, Setup> {
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

pub(crate) fn mesh_of(obj: &str) -> Result<ManifoldMesh, Setup> {
    let soup = obj::read(obj.as_bytes())?;
    Ok(into_manifold(&soup, &RepairPlan::default())?.0)
}

pub(crate) fn reference(obj: &str, seams: &str, mm_per_unit: f64) -> Result<Run, Setup> {
    let mesh = mesh_of(obj)?;
    let set = reference_seams(obj, seams, &mesh)?;
    run(mesh, &set, mm_per_unit)
}

/// A spherical cap with a two-edge dart up a meridian from its rim.
pub(crate) fn dart() -> Result<Run, Setup> {
    let rings = 4;
    let mesh = mesh_of(&super::cap(rings))?;
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
pub(crate) struct Path {
    pub(crate) chart: usize,
    pub(crate) points: Vec<DVec2>,
}

impl Path {
    pub(crate) fn length(&self) -> f64 {
        self.points.windows(2).map(|w| (w[1] - w[0]).length()).sum()
    }

    /// The point `t` of the way along.
    pub(crate) fn point_at(&self, t: f64) -> DVec2 {
        let mut left = t * self.length();
        for w in self.points.windows(2) {
            let len = (w[1] - w[0]).length();
            if left <= len {
                return w[0] + (w[1] - w[0]) * (left / len);
            }
            left -= len;
        }
        self.points[self.points.len() - 1]
    }

    /// The fraction of the length at which `p` lies on the path, if it lies on it.
    pub(crate) fn fraction_of(&self, p: DVec2) -> Option<f64> {
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
pub(crate) fn flat_side(run: &Run, h: HalfEdgeId) -> Option<(usize, VertexId, VertexId)> {
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

pub(crate) fn path(run: &Run, side: &ChainSide) -> Result<Path, Setup> {
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

pub(crate) fn inside(polygon: &[DVec2], p: DVec2) -> bool {
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

impl Run {
    /// The run's stage inputs, for a later stage.
    pub(crate) fn input(&self) -> PatternInput<'_> {
        PatternInput {
            mesh: &self.mesh,
            charts: &self.charts,
            sewing: &self.sewing,
            layouts: &self.layouts,
        }
    }

    /// The options `run` used.
    pub(crate) fn options(&self) -> Result<PatternOptions, Setup> {
        Ok(PatternOptions {
            scale: Scale::mm_per_model_unit(self.mm_per_unit)?,
            fabric: fabric(6.0)?,
        })
    }
}
