//! Fabric profiles: what a fabric is, checked once, and the layout rules that follow from it.

use openplush_geom::units::Mm;

use crate::{FabricId, PatternError};

/// The kind of fabric, which decides how a piece may turn and which warnings apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FabricCategory {
    /// Woven, such as quilting cotton.
    Woven,
    /// Knit, such as fleece.
    Knit,
    /// Pile, such as minky, velboa and faux fur.
    Pile,
    /// Felt, which has no grain.
    Felt,
}

/// A direction across the fabric that stretch is measured in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StretchDirection {
    /// Along the grain.
    Grain,
    /// Across the grain.
    Cross,
    /// At 45 degrees to the grain.
    Bias,
}

/// Everything a maker records about a fabric; `Fabric::new` checks it.
#[derive(Debug, Clone, PartialEq)]
pub struct FabricSpec {
    /// The fabric's ID within the project.
    pub id: FabricId,
    /// The maker's name for it.
    pub name: String,
    /// The kind of fabric.
    pub category: FabricCategory,
    /// Percent elongation along the grain under normal hand tension; drives warnings only.
    pub stretch_grain_pct: f64,
    /// Percent elongation across the grain.
    pub stretch_cross_pct: f64,
    /// Percent elongation on the bias.
    pub stretch_bias_pct: f64,
    /// Whether the fabric has a nap, so a piece upside down looks different.
    pub nap: bool,
    /// Pile length; zero for fabric without pile.
    pub pile_len: Mm,
    /// Thickness, recorded only.
    pub thickness: Mm,
    /// Whether a cut edge frays.
    pub frays: bool,
    /// Seam allowance used when no other is set.
    pub default_allowance: Mm,
    /// Width of the roll.
    pub roll_width: Mm,
}

/// A fabric profile that obeys every fabric invariant.
#[derive(Debug, Clone, PartialEq)]
pub struct Fabric(FabricSpec);

impl Fabric {
    /// Refuses a negative or non-finite stretch, a zero roll width, a zero allowance on fabric
    /// that frays, and pile with a length but no nap. Never adjusts a value.
    pub fn new(spec: FabricSpec) -> Result<Self, PatternError> {
        let fabric = spec.id;
        for (direction, percent) in [
            (StretchDirection::Grain, spec.stretch_grain_pct),
            (StretchDirection::Cross, spec.stretch_cross_pct),
            (StretchDirection::Bias, spec.stretch_bias_pct),
        ] {
            if !(percent.is_finite() && percent >= 0.0) {
                return Err(PatternError::BadStretch {
                    fabric,
                    direction,
                    percent,
                });
            }
        }
        if spec.roll_width.get() == 0.0 {
            return Err(PatternError::ZeroRollWidth { fabric });
        }
        if spec.frays && spec.default_allowance.get() == 0.0 {
            return Err(PatternError::ZeroAllowanceFrays { fabric });
        }
        if spec.category == FabricCategory::Pile && spec.pile_len.get() > 0.0 && !spec.nap {
            return Err(PatternError::PileWithoutNap { fabric });
        }
        Ok(Self(spec))
    }

    /// The fabric's ID.
    pub fn id(&self) -> FabricId {
        self.0.id
    }

    /// The checked values.
    pub fn spec(&self) -> &FabricSpec {
        &self.0
    }

    /// The rotations, in degrees, a piece cut from this fabric may be placed at; nap outranks category.
    pub fn rotations_deg(&self) -> Vec<f64> {
        if self.0.nap {
            vec![0.0]
        } else if self.0.category == FabricCategory::Felt {
            vec![0.0, 90.0, 180.0, 270.0]
        } else {
            vec![0.0, 180.0]
        }
    }

    /// Whether export must ask the maker to print mirrored or as-is; it never has a default.
    pub fn asks_mirroring(&self) -> bool {
        self.0.category == FabricCategory::Pile
    }
}
