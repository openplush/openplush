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
    /// A length in millimetres that is negative or not finite, either given directly
    /// or produced by a conversion that overflowed.
    #[error("length of {mm} mm is not usable (must be finite and not negative)")]
    InvalidLength {
        /// The rejected value.
        mm: f64,
    },
    /// A model-to-millimetre scale that is zero, negative, subnormal, or not finite.
    #[error("scale of {mm_per_unit} mm per model unit is not usable (must be finite and positive)")]
    InvalidScale {
        /// The rejected value.
        mm_per_unit: f64,
    },
    /// A predicate's coordinates span too many orders of magnitude for its sign to
    /// be guaranteed exact: some nonzero coordinate is below 2^-150 times the
    /// largest. Real models never get near this; it means corrupt or synthetic
    /// input, and a guessed sign would be silent wrong output.
    #[error("geometric predicate coordinates span more than 2^150 in magnitude")]
    UnsupportedMagnitude,
    /// A predicate received a NaN or infinite coordinate. Imported geometry is
    /// guaranteed finite, so this means an upstream stage produced bad data.
    #[error("geometric predicate received a non-finite coordinate")]
    NonFinite,
}
