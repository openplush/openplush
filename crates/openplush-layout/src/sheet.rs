use openplush_geom::DVec2;
use openplush_pattern::PieceId;

/// One copy of a piece on a sheet: mirrored about the y-axis if `mirrored`, then rotated counter-clockwise, then moved by `offset`.
#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    pub(crate) piece: PieceId,
    pub(crate) copy: u32,
    pub(crate) rotation_deg: f64,
    pub(crate) offset: DVec2,
    pub(crate) mirrored: bool,
}

impl Placement {
    /// The piece placed.
    pub fn piece(&self) -> PieceId {
        self.piece
    }

    /// Which copy of the piece this is, from 0.
    pub fn copy(&self) -> u32 {
        self.copy
    }

    /// The rotation, in degrees.
    pub fn rotation_deg(&self) -> f64 {
        self.rotation_deg
    }

    /// The translation, in millimetres, applied after mirroring and rotating.
    pub fn offset(&self) -> DVec2 {
        self.offset
    }

    /// Whether the piece is mirrored about the y-axis before rotating.
    pub fn mirrored(&self) -> bool {
        self.mirrored
    }
}

/// A rectangle of fabric holding placed pieces: as wide as the roll, as long as the placements need.
///
/// Coordinates are y-up, like the outlines placed on it, so its top edge is `y = length_mm`.
#[derive(Debug, Clone, PartialEq)]
pub struct Sheet {
    pub(crate) width_mm: f64,
    pub(crate) length_mm: f64,
    pub(crate) placements: Vec<Placement>,
}

impl Sheet {
    /// The roll width, in millimetres.
    pub fn width_mm(&self) -> f64 {
        self.width_mm
    }

    /// The fabric length the placements use, in millimetres.
    pub fn length_mm(&self) -> f64 {
        self.length_mm
    }

    /// Every copy of every piece, in placement order.
    pub fn placements(&self) -> &[Placement] {
        &self.placements
    }
}
