//! Robust geometric predicates.
//!
//! Each predicate returns the **exact** sign of a determinant, computed with
//! Shewchuk's adaptive-precision arithmetic (the `robust` crate; ADR-0002). Naive
//! f64 evaluation returns wrong and mutually inconsistent signs near degeneracy,
//! which is exactly where meshing and flattening code makes topological decisions.
//! The magnitude is not exact and is not exposed.
//!
//! `robust` is exact only while no intermediate overflows or underflows. Inputs are
//! first scaled by a power of two (exact, and sign-preserving because each
//! predicate is homogeneous), which removes overflow; inputs whose magnitudes
//! span more than 2^150 are refused, which removes underflow. So a sign is either
//! exact or a typed error, never a guess.

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
    /// `None` for NaN, which normalization makes unreachable; kept so a
    /// regression cannot turn NaN into `Zero` (a false degeneracy).
    fn of(value: f64) -> Option<Self> {
        if value > 0.0 {
            Some(Self::Positive)
        } else if value < 0.0 {
            Some(Self::Negative)
        } else if value == 0.0 {
            Some(Self::Zero)
        } else {
            None
        }
    }
}

/// Smallest nonzero |coordinate| accepted, relative to the largest (2^-150). With
/// coordinates in [2^-150, 1), the smallest product of four coordinate-difference
/// tails is about 2^-808 and its rounding error about 2^-861: still normal, so
/// the adaptive arithmetic never underflows.
const MIN_RELATIVE: f64 = 7.006_492_321_624_085e-46; // 2^-150

/// 2^k exactly, for k in [-1022, 1023].
fn pow2(k: i32) -> f64 {
    // INVARIANT: callers keep k in [-1022, 1023], so the biased exponent is in
    // [1, 2046], a normal number.
    f64::from_bits(((k + 1023) as u64) << 52)
}

/// Scales the coordinates by one power of two so the largest magnitude lands in
/// [0.5, 1). That is exact, and every predicate here is homogeneous (degree 2, 3,
/// or 4), so the sign is unchanged. Afterwards nothing can overflow, and the
/// threshold check rules out underflow. `None` if every coordinate is zero.
fn normalize<const N: usize>(mut xs: [f64; N]) -> Result<Option<[f64; N]>, GeomError> {
    if xs.iter().any(|x| !x.is_finite()) {
        return Err(GeomError::NonFinite);
    }
    let mut max = xs.iter().fold(0.0f64, |m, x| m.max(x.abs()));
    if max == 0.0 {
        return Ok(None);
    }
    if max < f64::MIN_POSITIVE {
        // All subnormal (or zero): scale into the normal range first. Exact, as
        // the products are representable.
        let up = pow2(600);
        xs.iter_mut().for_each(|x| *x *= up);
        max *= up;
    }
    // max is normal, in [2^e, 2^(e+1)); scale by 2^-(e+1) in two exact halves,
    // so neither factor leaves pow2's range.
    let e = ((max.to_bits() >> 52) & 0x7ff) as i32 - 1023;
    let k = -(e + 1);
    let (k1, k2) = (k / 2, k - k / 2);
    for x in &mut xs {
        *x = *x * pow2(k1) * pow2(k2);
        if *x != 0.0 && x.abs() < MIN_RELATIVE {
            return Err(GeomError::UnsupportedMagnitude);
        }
    }
    Ok(Some(xs))
}

fn sign(value: f64) -> Result<Sign, GeomError> {
    Sign::of(value).ok_or(GeomError::UnsupportedMagnitude)
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
/// Errors with [`GeomError::NonFinite`] if any coordinate is NaN or infinite, and
/// with [`GeomError::UnsupportedMagnitude`] if a nonzero coordinate is below 2^-150
/// times the largest.
pub fn orient2d(a: DVec2, b: DVec2, c: DVec2) -> Result<Sign, GeomError> {
    let Some([ax, ay, bx, by, cx, cy]) = normalize([a.x, a.y, b.x, b.y, c.x, c.y])? else {
        return Ok(Sign::Zero);
    };
    sign(robust::orient2d(
        c2(DVec2::new(ax, ay)),
        c2(DVec2::new(bx, by)),
        c2(DVec2::new(cx, cy)),
    ))
}

/// Where `d` lies relative to the circle through a, b, c (which must be
/// counter-clockwise for this reading): `Positive` inside, `Negative` outside,
/// `Zero` exactly on it. If a, b, c are clockwise the sign flips.
///
/// Errors with [`GeomError::NonFinite`] if any coordinate is NaN or infinite, and
/// with [`GeomError::UnsupportedMagnitude`] if a nonzero coordinate is below 2^-150
/// times the largest.
pub fn incircle(a: DVec2, b: DVec2, c: DVec2, d: DVec2) -> Result<Sign, GeomError> {
    let Some([ax, ay, bx, by, cx, cy, dx, dy]) =
        normalize([a.x, a.y, b.x, b.y, c.x, c.y, d.x, d.y])?
    else {
        return Ok(Sign::Zero);
    };
    sign(robust::incircle(
        c2(DVec2::new(ax, ay)),
        c2(DVec2::new(bx, by)),
        c2(DVec2::new(cx, cy)),
        c2(DVec2::new(dx, dy)),
    ))
}

/// Which side of the plane through a, b, c the point `d` lies on: `Positive` if
/// below, where "below" means a, b, c appear counter-clockwise when viewed from
/// above (Shewchuk's convention); `Negative` above; `Zero` exactly coplanar.
///
/// Errors with [`GeomError::NonFinite`] if any coordinate is NaN or infinite, and
/// with [`GeomError::UnsupportedMagnitude`] if a nonzero coordinate is below 2^-150
/// times the largest.
pub fn orient3d(a: DVec3, b: DVec3, c: DVec3, d: DVec3) -> Result<Sign, GeomError> {
    let Some([ax, ay, az, bx, by, bz, cx, cy, cz, dx, dy, dz]) =
        normalize([a.x, a.y, a.z, b.x, b.y, b.z, c.x, c.y, c.z, d.x, d.y, d.z])?
    else {
        return Ok(Sign::Zero);
    };
    sign(robust::orient3d(
        c3(DVec3::new(ax, ay, az)),
        c3(DVec3::new(bx, by, bz)),
        c3(DVec3::new(cx, cy, cz)),
        c3(DVec3::new(dx, dy, dz)),
    ))
}
