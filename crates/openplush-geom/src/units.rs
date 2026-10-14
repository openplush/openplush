//! Physical units. Model coordinates are unitless numbers until a [`Scale`] turns a
//! length into millimetres; [`Mm`] is the only type a physical tolerance accepts, so
//! model units and millimetres cannot meet without the named conversion
//! (AGENTS.md: units).

use crate::GeomError;

/// A length in millimetres.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Mm(f64);

impl Mm {
    /// Wraps a value already in millimetres.
    pub const fn new(mm: f64) -> Self {
        Self(mm)
    }

    /// The value in millimetres.
    pub const fn get(self) -> f64 {
        self.0
    }
}

/// Millimetres per model unit: the single uniform factor applied at the pattern
/// stage (data-model section 10). Always finite and positive.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scale(f64);

impl Scale {
    /// A scale of `mm_per_unit` millimetres per model unit. Errors on zero,
    /// negative, or non-finite input instead of substituting a default.
    pub fn mm_per_model_unit(mm_per_unit: f64) -> Result<Self, GeomError> {
        if mm_per_unit.is_finite() && mm_per_unit > 0.0 {
            Ok(Self(mm_per_unit))
        } else {
            Err(GeomError::InvalidScale { mm_per_unit })
        }
    }

    /// Converts a model-space length to millimetres.
    pub fn to_mm(self, model_length: f64) -> Mm {
        Mm(model_length * self.0)
    }
}
