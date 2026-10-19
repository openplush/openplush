//! How far a layout stretches and compresses each face: the singular values of the Jacobian
//! from the face's 3D shape to its flat one. Raw values only; tolerances are the fabric's.

use openplush_geom::{DVec2, DVec3};
use openplush_mesh::{FaceId, VertexId};
use openplush_seams::ChartId;

use crate::FlattenError;
use crate::interior::{Rest, rest_shapes};

/// One face's singular values, largest first; both positive.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Distortion {
    stretch: f64,
    compression: f64,
}

impl Distortion {
    /// The largest length ratio flat to 3D, σ₁ (above 1 is stretch).
    pub fn stretch(&self) -> f64 {
        self.stretch
    }

    /// The smallest length ratio flat to 3D, σ₂ (below 1 is compression).
    pub fn compression(&self) -> f64 {
        self.compression
    }

    /// Flat area over 3D area, σ₁σ₂.
    pub fn area_ratio(&self) -> f64 {
        self.stretch * self.compression
    }

    /// How far the face's angles change, σ₁/σ₂ (1 is angle-preserving).
    pub fn conformal_ratio(&self) -> f64 {
        self.stretch / self.compression
    }
}

/// Each face's distortion, in input order. `uv` is indexed like `xyz`.
///
/// # Errors
/// `EmptyChart` with no faces; `Degenerate` for the first face with no 3D area, a vertex with
/// no position in `xyz` or `uv`, or a non-finite Jacobian; otherwise `Flip` naming every face
/// with det J ≤ 0.
pub fn distortion(
    chart: ChartId,
    xyz: &[DVec3],
    faces: &[(FaceId, [VertexId; 3])],
    uv: &[DVec2],
) -> Result<Vec<(FaceId, Distortion)>, FlattenError> {
    if faces.is_empty() {
        return Err(FlattenError::EmptyChart { chart });
    }
    let rest = rest_shapes(xyz, faces).map_err(|face| FlattenError::Degenerate { chart, face })?;
    let mut measured = Vec::with_capacity(faces.len());
    let mut flipped = Vec::new();
    for (r, &(face, t)) in rest.iter().zip(faces) {
        match measure(r, uv, &t) {
            Some(Measure::Kept(d)) => measured.push((face, d)),
            Some(Measure::Flipped) => flipped.push(face),
            None => return Err(FlattenError::Degenerate { chart, face }),
        }
    }
    if flipped.is_empty() {
        Ok(measured)
    } else {
        Err(FlattenError::Flip {
            chart,
            faces: flipped,
        })
    }
}

enum Measure {
    Kept(Distortion),
    Flipped,
}

/// `None` when a corner has no flat position or the Jacobian is not finite.
fn measure(rest: &Rest, uv: &[DVec2], t: &[VertexId; 3]) -> Option<Measure> {
    if t.iter().any(|v| v.index() as usize >= uv.len()) {
        return None;
    }
    let [[a, b], [c, d]] = rest.jacobian(uv, t);
    if ![a, b, c, d].iter().all(|v| v.is_finite()) {
        return None;
    }
    let det = a * d - b * c;
    if det <= 0.0 {
        return Some(Measure::Flipped);
    }
    // J = Rot(φ) diag(q + r, q − r) Rot(θ), the closed-form 2×2 SVD `interior::proxy` uses.
    let q = libm::hypot((a + d) / 2.0, (c - b) / 2.0);
    let r = libm::hypot((a - d) / 2.0, (c + b) / 2.0);
    let stretch = q + r;
    // det / σ₁ rather than q − r, which cancels when the face is nearly collapsed.
    let compression = det / stretch;
    // An overflowing det or stretch, or an underflowing quotient, leaves nothing measurable.
    (compression.is_finite() && compression > 0.0).then_some(Measure::Kept(Distortion {
        stretch,
        compression,
    }))
}
