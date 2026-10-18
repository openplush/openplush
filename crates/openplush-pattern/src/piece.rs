use crate::{CutCount, Outline, PatternError, PieceId};

/// One pattern template: its two outlines, how many copies to cut, and the rotations layout may use.
#[derive(Debug, Clone, PartialEq)]
pub struct PatternPiece {
    id: PieceId,
    sewing_line: Outline,
    cut_line: Outline,
    cuts: CutCount,
    rotations_deg: Vec<f64>,
}

impl PatternPiece {
    /// Refuses an empty or non-finite rotation set.
    pub fn new(
        id: PieceId,
        sewing_line: Outline,
        cut_line: Outline,
        cuts: CutCount,
        rotations_deg: Vec<f64>,
    ) -> Result<Self, PatternError> {
        if rotations_deg.is_empty() {
            return Err(PatternError::NoRotations);
        }
        if let Some(index) = rotations_deg.iter().position(|r| !r.is_finite()) {
            return Err(PatternError::NonFiniteRotation { index });
        }
        Ok(Self {
            id,
            sewing_line,
            cut_line,
            cuts,
            rotations_deg,
        })
    }

    /// The piece's ID.
    pub fn id(&self) -> PieceId {
        self.id
    }

    /// Where the pieces are sewn together.
    pub fn sewing_line(&self) -> &Outline {
        &self.sewing_line
    }

    /// Where the fabric is cut.
    pub fn cut_line(&self) -> &Outline {
        &self.cut_line
    }

    /// How many copies to cut, as printed and flipped.
    pub fn cuts(&self) -> CutCount {
        self.cuts
    }

    /// The rotations, in degrees, layout may place the piece at; never empty.
    pub fn rotations_deg(&self) -> &[f64] {
        &self.rotations_deg
    }
}
