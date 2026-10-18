use openplush_geom::units::Mm;

use crate::LayoutError;

/// Where pieces may go: the roll width, the clearance between cut lines, and whether to print mirrored.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutOptions {
    pub(crate) roll_width: Mm,
    pub(crate) gap: Mm,
    pub(crate) mirrored: bool,
}

impl LayoutOptions {
    /// Refuses a roll width of zero.
    pub fn new(roll_width: Mm, gap: Mm, mirrored: bool) -> Result<Self, LayoutError> {
        if roll_width.get() <= 0.0 {
            return Err(LayoutError::BadRollWidth);
        }
        Ok(Self {
            roll_width,
            gap,
            mirrored,
        })
    }
}
