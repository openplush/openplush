use thiserror::Error;

/// Every way this crate refuses its input. Values are carried so the caller can
/// report exactly what was wrong; nothing here clamps or substitutes a default.
#[derive(Debug, Clone, Copy, PartialEq, Error)]
pub enum GeomError {
    /// A bounding-box diagonal that cannot define model-space tolerances: zero,
    /// negative, subnormal, or not finite. A degenerate model, not a tolerance problem.
    #[error(
        "bounding-box diagonal {diagonal} cannot define tolerances (must be finite and positive)"
    )]
    InvalidDiagonal {
        /// The rejected value.
        diagonal: f64,
    },
    /// A model-to-millimetre scale that is zero, negative, or not finite.
    #[error("scale of {mm_per_unit} mm per model unit is not usable (must be finite and positive)")]
    InvalidScale {
        /// The rejected value.
        mm_per_unit: f64,
    },
}
