//! Physical units. Model coordinates are unitless numbers until a [`Scale`] turns a
//! length into millimetres. [`Mm`] is the only type a physical tolerance accepts, so
//! a model length cannot be compared with a millimetre tolerance by accident
//! (AGENTS.md: units). Nothing stops a caller from passing a model length to
//! [`Mm::try_new`] on purpose; the type makes the conversion visible, not impossible.

use crate::GeomError;

/// A length in millimetres. Always finite and not negative, so comparisons
/// between lengths (seam parity, allowances) can never be fooled by a NaN.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Mm(f64);

impl Mm {
    /// A length of `mm` millimetres. Errors on a negative or non-finite value.
    pub fn try_new(mm: f64) -> Result<Self, GeomError> {
        if mm.is_finite() && mm >= 0.0 {
            Ok(Self(mm))
        } else {
            Err(GeomError::InvalidLength { mm })
        }
    }

    /// For constants and values this crate has already checked.
    /// Precondition: `mm` is finite and not negative.
    pub(crate) const fn new(mm: f64) -> Self {
        Self(mm)
    }

    /// The value in millimetres.
    pub const fn get(self) -> f64 {
        self.0
    }
}

/// Millimetres per model unit: the single uniform factor applied at the pattern
/// stage. Always finite, positive, and normal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scale(f64);

impl Scale {
    /// A scale of `mm_per_unit` millimetres per model unit. Errors on zero,
    /// negative, subnormal, or non-finite input instead of substituting a default.
    pub fn mm_per_model_unit(mm_per_unit: f64) -> Result<Self, GeomError> {
        if mm_per_unit.is_finite() && mm_per_unit >= f64::MIN_POSITIVE {
            Ok(Self(mm_per_unit))
        } else {
            Err(GeomError::InvalidScale { mm_per_unit })
        }
    }

    /// Converts a model-space length to millimetres. Errors if the length is
    /// negative or not finite, or if the product overflows.
    pub fn to_mm(self, model_length: f64) -> Result<Mm, GeomError> {
        Mm::try_new(model_length * self.0)
    }
}
