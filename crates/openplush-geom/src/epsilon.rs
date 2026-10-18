//! The tolerance policy.
//!
//! Two kinds of tolerance, kept apart by type:
//! - **Model-space** tolerances ([`ModelTolerances`]) scale with the model's
//!   bounding-box diagonal, so the same mesh at any size makes the same decisions
//!   (flattening is scale-equivariant; crates/AGENTS.md).
//! - **Physical** tolerances ([`SeamParity`]) are in millimetres after scaling,
//!   because a sewist's 1 mm is 1 mm whatever the model's units.
//!
//! Algorithm code takes its epsilons from here; no literal `1e-9`s elsewhere.

use crate::GeomError;
use crate::units::Mm;

/// Vertices closer than this fraction of the bbox diagonal are welded.
pub const WELD_RELATIVE: f64 = 1e-6;
/// Triangles with area below this fraction of diagonal² are degenerate.
pub const AREA_RELATIVE_SQ: f64 = 1e-12;
/// The smallest bounding-box diagonal tolerances can be derived for. Below it the
/// area tolerance (`1e-12 * diagonal²`) is subnormal or zero, so zero-area
/// triangles would pass the degeneracy check.
pub const MIN_DIAGONAL: f64 = 1e-100;
/// The largest bounding-box diagonal. Above it, cross products of coordinates can
/// overflow to infinity.
pub const MAX_DIAGONAL: f64 = 1e100;
/// A flattened triangle needs det J above this to count as unflipped. det J is a
/// ratio of areas, so this threshold is dimensionless and does not scale.
pub const MIN_DET_J: f64 = 1e-10;
/// Corner UVs on the two sides of an edge differing by more than this make it a UV
/// cut. UV space is dimensionless, so the value is neither model-relative nor
/// physical. It is about two orders above `f32` spacing near 1.0, where an exporter's
/// bit-identical uncut UVs live, and far below any real cut.
pub const UV_CUT: f64 = 1e-5;

/// Model-space tolerances for one model, derived from its bounding-box diagonal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelTolerances {
    diagonal: f64,
}

impl ModelTolerances {
    /// Tolerances for a model whose bounding-box diagonal is `diagonal` (model units).
    /// Errors outside [`MIN_DIAGONAL`, `MAX_DIAGONAL`] (zero, negative, and
    /// non-finite included): such a model is degenerate or out of range, and
    /// inventing a tolerance for it would hide that.
    pub fn for_diagonal(diagonal: f64) -> Result<Self, GeomError> {
        if (MIN_DIAGONAL..=MAX_DIAGONAL).contains(&diagonal) {
            Ok(Self { diagonal })
        } else {
            Err(GeomError::InvalidDiagonal { diagonal })
        }
    }

    /// The bounding-box diagonal these tolerances were derived from.
    pub fn diagonal(&self) -> f64 {
        self.diagonal
    }

    /// Weld distance, in model units.
    pub fn weld(&self) -> f64 {
        WELD_RELATIVE * self.diagonal
    }

    /// Minimum non-degenerate triangle area, in model units².
    pub fn area(&self) -> f64 {
        AREA_RELATIVE_SQ * self.diagonal * self.diagonal
    }

    /// Minimum det J for an unflipped flattened triangle (dimensionless).
    pub fn min_det_j(&self) -> f64 {
        MIN_DET_J
    }
}

/// How closely the two sides of a seam must agree in length, after scaling: the
/// larger of an absolute floor and a fraction of the seam length (max(1.0 mm,
/// 0.5%)). Parity is a hard contract (boundary lengths are prescribed): exceeding this
/// is a typed error downstream, never a warning.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SeamParity {
    floor: Mm,
    ratio: f64,
}

impl SeamParity {
    /// max(1.0 mm, 0.5% of the seam length).
    pub const DEFAULT: Self = Self {
        floor: Mm::new(1.0),
        ratio: 0.005,
    };

    /// The allowed difference for a seam of length `length`. Both operands are
    /// finite and not negative (an [`Mm`] cannot be otherwise), so the result is too.
    pub fn allowed(&self, length: Mm) -> Mm {
        Mm::new(self.floor.get().max(self.ratio * length.get()))
    }

    /// True if two seam-side lengths agree within the allowance for the longer side.
    pub fn within(&self, a: Mm, b: Mm) -> bool {
        let longer = if a.get() >= b.get() { a } else { b };
        (a.get() - b.get()).abs() <= self.allowed(longer).get()
    }
}
