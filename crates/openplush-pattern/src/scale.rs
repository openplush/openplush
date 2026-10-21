//! Real-world size: the scale that makes a model its finished size, and whether that size is supported.

use openplush_geom::units::{Mm, Scale};

use crate::PatternError;

/// Smallest supported longest finished dimension: the smallest plush that can still be turned and stuffed.
pub const MIN_MM: f64 = 75.0;
/// Largest supported longest finished dimension; wearable scale is out of scope.
pub const MAX_MM: f64 = 1000.0;

/// How the maker sets the finished size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SizeTarget {
    /// The model's longest dimension finishes at this length.
    LongestDimension(Mm),
    /// A distance of `model_length` model units, measured on the model, finishes at `finished`.
    Reference {
        /// The distance in model units.
        model_length: f64,
        /// What it measures once made.
        finished: Mm,
    },
}

/// Where the longest finished dimension falls against 75 mm to 1000 mm, both ends included.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeRange {
    /// Under 75 mm.
    Below,
    /// 75 mm to 1000 mm.
    Within,
    /// Over 1000 mm.
    Above,
}

/// The scale for `target` on a model whose longest dimension is `model_longest` model units.
///
/// Errors on a model length that is not finite and positive, and on a scale or finished size
/// that is zero or not finite. Outside the supported range is a `SizeRange`, never an error.
pub fn size(model_longest: f64, target: SizeTarget) -> Result<(Scale, SizeRange), PatternError> {
    model_length(model_longest)?;
    let (scale, finished_longest) = match target {
        // Judged on the target itself, so a rounded product never moves a boundary size out of range.
        SizeTarget::LongestDimension(finished) => (per_unit(finished, model_longest)?, finished),
        SizeTarget::Reference {
            model_length: length,
            finished,
        } => {
            let scale = per_unit(finished, model_length(length)?)?;
            let longest = scale
                .to_mm(model_longest)
                .map_err(|source| PatternError::BadScale { source })?;
            (scale, longest)
        }
    };
    Ok((scale, range(finished_longest)))
}

fn model_length(length: f64) -> Result<f64, PatternError> {
    if length.is_finite() && length > 0.0 {
        Ok(length)
    } else {
        Err(PatternError::BadModelLength {
            model_length: length,
        })
    }
}

fn per_unit(finished: Mm, model_length: f64) -> Result<Scale, PatternError> {
    Scale::mm_per_model_unit(finished.get() / model_length)
        .map_err(|source| PatternError::BadScale { source })
}

fn range(longest: Mm) -> SizeRange {
    let mm = longest.get();
    if mm < MIN_MM {
        SizeRange::Below
    } else if mm > MAX_MM {
        SizeRange::Above
    } else {
        SizeRange::Within
    }
}
