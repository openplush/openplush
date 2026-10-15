//! Oracles independent of the flattener and pattern code: each
//! recomputes its property from raw coordinates, so a bug in the code under test
//! cannot also hide in its check.

use openplush_geom::predicates::orient2d;
use openplush_geom::{DVec2, DVec3, Sign};
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
        if !(face_3d > 0.0) {
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
    let (q, r) = (e.hypot(h), f.hypot(g));
    Some((q + r, q - r))
}
