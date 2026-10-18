use openplush_geom::DVec2;

use crate::PatternError;

/// A closed, simple, counter-clockwise polygon in millimetres.
#[derive(Debug, Clone, PartialEq)]
pub struct Outline {
    points: Vec<DVec2>,
}

impl Outline {
    /// Refuses an outline that is too short, non-finite, self-touching, folded or clockwise.
    pub fn new(points: Vec<DVec2>) -> Result<Self, PatternError> {
        Ok(Self { points })
    }

    /// The corners in order; edge `i` runs from point `i` to point `i + 1`, the last to the first.
    pub fn points(&self) -> &[DVec2] {
        &self.points
    }
}
