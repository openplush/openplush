//! Oracles independent of the flattener and pattern code: each
//! recomputes its property from raw coordinates, so a bug in the code under test
//! cannot also hide in its check.

use openplush_geom::predicates::orient2d;
use openplush_geom::units::Mm;
use openplush_geom::{DVec2, DVec3, GeomError, Sign};
use openplush_mesh::{FaceId, VertexId};

/// A flattened chart as plain arrays: `uv[i]` is where `xyz[i]` lands.
#[derive(Debug, Clone, Copy)]
pub struct Chart<'a> {
    /// 3D positions.
    pub xyz: &'a [DVec3],
    /// 2D positions, one per 3D position.
    pub uv: &'a [DVec2],
    /// Triangles, counter-clockwise in `uv` when correctly flattened.
    pub faces: &'a [[VertexId; 3]],
}

/// What an oracle found, located so a failing test names the culprit.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum OracleFailure {
    /// `xyz` and `uv` differ in length.
    LengthMismatch {
        /// `xyz.len()`.
        xyz: usize,
        /// `uv.len()`.
        uv: usize,
    },
    /// The chart has no faces, so there is nothing to check.
    Empty,
    /// A tolerance that is NaN, infinite or negative.
    BadTolerance {
        /// The tolerance passed.
        tol: f64,
    },
    /// A face names a vertex outside the arrays.
    BadIndex {
        /// The face.
        face: FaceId,
    },
    /// Faces whose flattened orientation is not strictly positive (det J ≤ 0).
    Inverted {
        /// Every such face, in index order.
        faces: Vec<FaceId>,
    },
    /// A face with zero or non-finite 3D area, where distortion is undefined.
    Degenerate {
        /// The face.
        face: FaceId,
    },
    /// The first face whose signed singular values leave `[1 - tol, 1 + tol]`.
    Distorted {
        /// The face.
        face: FaceId,
        /// Largest singular value of its Jacobian.
        sigma_max: f64,
        /// Smallest singular value of its Jacobian, negative if the face is mirrored.
        sigma_min: f64,
    },
    /// Total signed flat area differs from total 3D area by more than the tolerance.
    AreaChanged {
        /// Sum of 3D face areas.
        area_3d: f64,
        /// Sum of signed flat face areas.
        area_2d: f64,
    },
    /// The outline has fewer than three points, a non-finite point, a zero-length
    /// or doubled-back edge, two edges that cross or touch, or a pair of edges the
    /// exact predicates cannot decide. Indices are edge starts.
    OutlineNotSimple {
        /// The first offending edge.
        a: usize,
        /// The edge it meets, or `a` itself.
        b: usize,
    },
    /// The outline is simple but wound clockwise.
    Clockwise,
    /// Two seam sides differ by more than the parity allowance.
    ParityExceeded {
        /// One side.
        a: Mm,
        /// The other side.
        b: Mm,
        /// The allowance for the longer side.
        allowed_mm: f64,
    },
}

/// Every face is strictly positively oriented in `uv` (exact predicate).
pub fn injective(chart: &Chart<'_>) -> Result<(), OracleFailure> {
    let mut inverted = Vec::new();
    for (face, [a, b, c]) in corners(chart)? {
        let (pa, pb, pc) = (chart.uv[a], chart.uv[b], chart.uv[c]);
        // A predicate error (non-finite or unsupported magnitude) is never a valid
        // flattening; count it as inverted.
        if orient2d(pa, pb, pc) != Ok(Sign::Positive) {
            inverted.push(face);
        }
    }
    if inverted.is_empty() {
        Ok(())
    } else {
        Err(OracleFailure::Inverted { faces: inverted })
    }
}

/// Every face maps with both signed singular values in `[1 - tol, 1 + tol]`: an
/// isometry face by face, which a correctly flattened developable surface achieves.
/// A mirrored face fails, but only up to rounding; pair with [`injective`] for an
/// exact orientation check.
pub fn developable(chart: &Chart<'_>, tol: f64) -> Result<(), OracleFailure> {
    check_tolerance(tol)?;
    for (face, [a, b, c]) in corners(chart)? {
        let (sigma_max, sigma_min) =
            singular_values(chart, [a, b, c]).ok_or(OracleFailure::Degenerate { face })?;
        // Negated so a NaN singular value fails.
        if !(1.0 - tol <= sigma_min && sigma_max <= 1.0 + tol) {
            return Err(OracleFailure::Distorted {
                face,
                sigma_max,
                sigma_min,
            });
        }
    }
    Ok(())
}

/// Total signed flat area is within `rel_tol` of total 3D area, so a mirrored
/// chart fails.
pub fn area_conserved(chart: &Chart<'_>, rel_tol: f64) -> Result<(), OracleFailure> {
    check_tolerance(rel_tol)?;
    let (mut area_3d, mut area_2d) = (0.0, 0.0);
    for (face, [a, b, c]) in corners(chart)? {
        let (p, q, r) = (chart.xyz[a], chart.xyz[b], chart.xyz[c]);
        let face_3d = (q - p).cross(r - p).length() / 2.0;
        // A face with no 3D area makes the ratio meaningless, and 0 <= tol * 0 would pass.
        if face_3d.is_nan() || face_3d <= 0.0 {
            return Err(OracleFailure::Degenerate { face });
        }
        area_3d += face_3d;
        let (u, v, w) = (chart.uv[a], chart.uv[b], chart.uv[c]);
        area_2d += (v - u).perp_dot(w - u) / 2.0;
    }
    if (area_2d - area_3d).abs() <= rel_tol * area_3d {
        Ok(())
    } else {
        Err(OracleFailure::AreaChanged { area_3d, area_2d })
    }
}

/// The closed polygon `outline` is simple (no two edges cross or touch, none has
/// zero length or doubles back) and counter-clockwise, so it can be cut in one
/// piece. Every decision is an exact predicate; one the predicates refuse fails.
pub fn outline_simple(outline: &[DVec2]) -> Result<(), OracleFailure> {
    let n = outline.len();
    let not_simple = |a, b| Err(OracleFailure::OutlineNotSimple { a, b });
    // INVARIANT: every index below is reduced mod n, and n >= 3 past this check.
    let edge = |i: usize| (outline[i % n], outline[(i + 1) % n]);
    if n < 3 {
        return not_simple(0, 0);
    }
    if let Some(a) = outline.iter().position(|p| !p.is_finite()) {
        return not_simple(a, a);
    }
    if let Some(a) = (0..n).find(|&a| edge(a).0 == edge(a).1) {
        return not_simple(a, a);
    }
    for a in 0..n {
        let (p, q) = edge(a);
        if !turns_cleanly(p, q, edge(a + 1).1) {
            return not_simple(a, (a + 1) % n);
        }
        // Edge n - 1 is adjacent to edge 0, so it is skipped for edge 0 only.
        let end = if a == 0 { n - 1 } else { n };
        for b in a + 2..end {
            let (r, s) = edge(b);
            if !matches!(segments_meet(p, q, r, s), Ok(false)) {
                return not_simple(a, b);
            }
        }
    }
    // The lowest, then leftmost, vertex of a simple polygon is convex, so its turn
    // is the polygon's orientation.
    let lowest = |i: usize, j: usize| {
        let (pi, pj) = (outline[i], outline[j]);
        pi.y.total_cmp(&pj.y).then(pi.x.total_cmp(&pj.x)).is_lt()
    };
    let v = (1..n).fold(0, |best, i| if lowest(i, best) { i } else { best });
    let prev = (v + n - 1) % n;
    match orient2d(outline[prev], outline[v], outline[(v + 1) % n]) {
        Ok(Sign::Positive) => Ok(()),
        Ok(Sign::Negative) => Err(OracleFailure::Clockwise),
        // Unreachable: the same corner passed `turns_cleanly` above.
        Ok(Sign::Zero) | Err(_) => not_simple(prev, v),
    }
}

/// The two sides of a seam agree within max(1.0 mm, 0.5% of the longer side).
/// Restated from the spec here rather than calling the engine's `SeamParity`,
/// so a wrong constant there cannot pass its own check.
pub fn seam_parity(a: Mm, b: Mm) -> Result<(), OracleFailure> {
    let allowed_mm = f64::max(1.0, 0.005 * a.get().max(b.get()));
    if (a.get() - b.get()).abs() <= allowed_mm {
        Ok(())
    } else {
        Err(OracleFailure::ParityExceeded { a, b, allowed_mm })
    }
}

fn check_tolerance(tol: f64) -> Result<(), OracleFailure> {
    if tol.is_finite() && tol >= 0.0 {
        Ok(())
    } else {
        Err(OracleFailure::BadTolerance { tol })
    }
}

/// Each face with its corner indices, checked against both arrays.
fn corners(chart: &Chart<'_>) -> Result<Vec<(FaceId, [usize; 3])>, OracleFailure> {
    if chart.xyz.len() != chart.uv.len() {
        return Err(OracleFailure::LengthMismatch {
            xyz: chart.xyz.len(),
            uv: chart.uv.len(),
        });
    }
    if chart.faces.is_empty() {
        return Err(OracleFailure::Empty);
    }
    let mut out = Vec::with_capacity(chart.faces.len());
    for (i, tri) in chart.faces.iter().enumerate() {
        let face = FaceId::new(u32::try_from(i).map_err(|_| OracleFailure::BadIndex {
            face: FaceId::new(u32::MAX),
        })?);
        let idx = tri.map(|v| usize::try_from(v.index()).unwrap_or(usize::MAX));
        if idx.iter().any(|&k| k >= chart.xyz.len()) {
            return Err(OracleFailure::BadIndex { face });
        }
        out.push((face, idx));
    }
    Ok(out)
}

/// Signed singular values (max, min) of the map from a face's 3D plane to `uv`;
/// `min` is negative when the map reflects. `None` if the 3D face has no area.
fn singular_values(chart: &Chart<'_>, [a, b, c]: [usize; 3]) -> Option<(f64, f64)> {
    let (e1, e2) = (chart.xyz[b] - chart.xyz[a], chart.xyz[c] - chart.xyz[a]);
    let ex = e1.normalize()?;
    let ey = e1.cross(e2).cross(e1).normalize()?;
    // The face in its own plane: (q1x, 0) and (q2x, q2y), with q2y > 0.
    let (q1x, q2x, q2y) = (e1.dot(ex), e2.dot(ex), e2.dot(ey));
    let (d1, d2) = (chart.uv[b] - chart.uv[a], chart.uv[c] - chart.uv[a]);
    // J = [d1 d2] * inverse([[q1x, q2x], [0, q2y]]).
    let (j00, j10) = (d1.x / q1x, d1.y / q1x);
    let (j01, j11) = ((d2.x - j00 * q2x) / q2y, (d2.y - j10 * q2x) / q2y);
    // Closed form for a 2x2 matrix's singular values (Blinn 1996); q - r carries
    // the sign of det J.
    let (e, f) = ((j00 + j11) / 2.0, (j00 - j11) / 2.0);
    let (g, h) = ((j10 + j01) / 2.0, (j10 - j01) / 2.0);
    let (q, r) = (libm::hypot(e, h), libm::hypot(f, g));
    Some((q + r, q - r))
}

/// The corner `p -> q -> next` does not fold `next` back over `pq`. Precondition:
/// `p != q` and `q != next`. A predicate error counts as folding.
fn turns_cleanly(p: DVec2, q: DVec2, next: DVec2) -> bool {
    match orient2d(p, q, next) {
        Ok(Sign::Zero) => !(within_box(q, p, next) || within_box(q, next, p)),
        Ok(_) => true,
        Err(_) => false,
    }
}

/// Closed segments `pq` and `rs` share at least one point (exact predicates).
fn segments_meet(p: DVec2, q: DVec2, r: DVec2, s: DVec2) -> Result<bool, GeomError> {
    let (o1, o2) = (orient2d(p, q, r)?, orient2d(p, q, s)?);
    let (o3, o4) = (orient2d(r, s, p)?, orient2d(r, s, q)?);
    let opposite = |x: Sign, y: Sign| x != Sign::Zero && y != Sign::Zero && x != y;
    if opposite(o1, o2) && opposite(o3, o4) {
        return Ok(true);
    }
    Ok((o1 == Sign::Zero && within_box(p, q, r))
        || (o2 == Sign::Zero && within_box(p, q, s))
        || (o3 == Sign::Zero && within_box(r, s, p))
        || (o4 == Sign::Zero && within_box(r, s, q)))
}

/// `c`, known collinear with `ab`, lies on the closed segment `ab`.
fn within_box(a: DVec2, b: DVec2, c: DVec2) -> bool {
    a.x.min(b.x) <= c.x && c.x <= a.x.max(b.x) && a.y.min(b.y) <= c.y && c.y <= a.y.max(b.y)
}
