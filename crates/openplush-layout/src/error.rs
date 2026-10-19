use openplush_pattern::PieceId;
use thiserror::Error;

/// Every way `openplush-layout` refuses its input.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum LayoutError {
    /// A fabric roll must have a width.
    #[error("the roll width must be greater than zero")]
    BadRollWidth,
    /// There is nothing to lay out.
    #[error("there are no pieces to lay out")]
    NoPieces,
    /// A piece is wider than the roll in its allowed orientation.
    #[error(
        "piece {} is {width_mm:.3} mm wide but the roll is {roll_mm:.3} mm",
        .piece.index()
    )]
    PieceTooWide {
        /// The piece.
        piece: PieceId,
        /// Its width after rotation.
        width_mm: f64,
        /// The roll width.
        roll_mm: f64,
    },
    /// A sheet that needs more than 26 pages across or down to print.
    #[error("a {width_mm:.3} by {length_mm:.3} mm sheet needs more than 26 pages across or down")]
    TooManyPages {
        /// The sheet's width.
        width_mm: f64,
        /// The sheet's length.
        length_mm: f64,
    },
    /// An allowed rotation that is not a multiple of 90 degrees, which this placer cannot apply exactly.
    #[error("piece {} allows a rotation of {rotation_deg} degrees; only multiples of 90 are supported", .piece.index())]
    UnsupportedRotation {
        /// The piece.
        piece: PieceId,
        /// The first allowed rotation.
        rotation_deg: f64,
    },
}
