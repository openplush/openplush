//! Robust geometric predicates.
//!
//! Each predicate returns the **exact** sign of a determinant, computed with
//! Shewchuk's adaptive-precision arithmetic (the `robust` crate; ADR-0002). Naive
//! f64 evaluation returns wrong and mutually inconsistent signs near degeneracy,
//! which is exactly where meshing and flattening code makes topological decisions.
//! The magnitude is not exact and is not exposed.

use crate::{DVec2, DVec3, GeomError};

/// The exact sign of a predicate's determinant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Sign {
    /// Strictly negative.
    Negative,
    /// Exactly zero: a true degeneracy (collinear, cocircular, coplanar).
    Zero,
    /// Strictly positive.
    Positive,
}

impl Sign {
    fn of(value: f64) -> Self {
        if value > 0.0 {
            Self::Positive
        } else if value < 0.0 {
            Self::Negative
        } else {
            Self::Zero
        }
    }
}

fn c2(p: DVec2) -> robust::Coord<f64> {
    robust::Coord { x: p.x, y: p.y }
}

fn c3(p: DVec3) -> robust::Coord3D<f64> {
    robust::Coord3D {
        x: p.x,
        y: p.y,
        z: p.z,
    }
}

/// Orientation of the triangle (a, b, c): `Positive` if counter-clockwise,
/// `Negative` if clockwise, `Zero` if exactly collinear.
///
/// Errors with [`GeomError::NonFinite`] if any coordinate is NaN or infinite.
pub fn orient2d(a: DVec2, b: DVec2, c: DVec2) -> Result<Sign, GeomError> {
    if !(a.is_finite() && b.is_finite() && c.is_finite()) {
        return Err(GeomError::NonFinite);
    }
    Ok(Sign::of(robust::orient2d(c2(a), c2(b), c2(c))))
}

/// Where `d` lies relative to the circle through a, b, c (which must be
/// counter-clockwise for this reading): `Positive` inside, `Negative` outside,
/// `Zero` exactly on it. If a, b, c are clockwise the sign flips.
///
/// Errors with [`GeomError::NonFinite`] if any coordinate is NaN or infinite.
pub fn incircle(a: DVec2, b: DVec2, c: DVec2, d: DVec2) -> Result<Sign, GeomError> {
    if !(a.is_finite() && b.is_finite() && c.is_finite() && d.is_finite()) {
        return Err(GeomError::NonFinite);
    }
    Ok(Sign::of(robust::incircle(c2(a), c2(b), c2(c), c2(d))))
}

/// Which side of the plane through a, b, c the point `d` lies on: `Positive` if
/// below, where "below" means a, b, c appear counter-clockwise when viewed from
/// above (Shewchuk's convention); `Negative` above; `Zero` exactly coplanar.
///
/// Errors with [`GeomError::NonFinite`] if any coordinate is NaN or infinite.
pub fn orient3d(a: DVec3, b: DVec3, c: DVec3, d: DVec3) -> Result<Sign, GeomError> {
    if !(a.is_finite() && b.is_finite() && c.is_finite() && d.is_finite()) {
        return Err(GeomError::NonFinite);
    }
    Ok(Sign::of(robust::orient3d(c3(a), c3(b), c3(c), c3(d))))
}
