use openplush_geom::GeomError;
use openplush_geom::units::Mm;
use openplush_seams::SeamChainId;

use crate::{FabricId, PieceId, StretchDirection};
use thiserror::Error;

/// Every way `openplush-pattern` refuses its input or its own result.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum PatternError {
    /// The two sides of a seam chain differ by more than the tolerance.
    #[error(
        "seam chain {}: sides {:.3} mm and {:.3} mm differ by more than {:.3} mm",
        .chain.index(), .a.get(), .b.get(), .tolerance.get()
    )]
    SeamParity {
        /// The chain.
        chain: SeamChainId,
        /// One side's length.
        a: Mm,
        /// The other side's length.
        b: Mm,
        /// The largest allowed difference.
        tolerance: Mm,
    },
    /// An outline needs at least three points to enclose an area.
    #[error("an outline needs at least 3 points, got {points}")]
    OutlineTooShort {
        /// How many points were given.
        points: usize,
    },
    /// An outline point with a NaN or infinite coordinate.
    #[error("outline point {point} is not finite")]
    OutlineNotFinite {
        /// The point's index.
        point: usize,
    },
    /// Two edges that cross, touch, overlap, or fold back, or a repeated point. Edge `i` runs from point `i` to point `i + 1`.
    #[error("outline edges {edge} and {other} cross, touch, overlap or fold back")]
    OutlineNotSimple {
        /// The first edge.
        edge: usize,
        /// The second edge; equal to `edge` for a repeated point.
        other: usize,
    },
    /// A simple outline wound clockwise; it is refused, never reversed.
    #[error("outline is wound clockwise")]
    OutlineClockwise,
    /// Coordinates no exact predicate can judge.
    #[error("outline coordinates cannot be judged exactly: {source}")]
    OutlineUnjudgeable {
        /// The predicate's refusal.
        source: GeomError,
    },
    /// A piece must be cut at least once.
    #[error("a piece must be cut at least once")]
    ZeroCutQuantity,
    /// More copies than any plush needs: a typo or a corrupt project file.
    #[error(
        "a piece can be cut at most {} times, got {total}",
        crate::CutCount::MAX_TOTAL
    )]
    TooManyCuts {
        /// Copies as printed plus copies flipped.
        total: u64,
    },
    /// A piece needs at least one allowed rotation.
    #[error("a piece needs at least one allowed rotation")]
    NoRotations,
    /// A stretch percentage that is negative or not finite.
    #[error(
        "fabric {}: a {} stretch of {percent}% is not usable (must be finite and not negative)",
        .fabric.index(), direction_name(*.direction)
    )]
    BadStretch {
        /// The fabric.
        fabric: FabricId,
        /// Which stretch.
        direction: StretchDirection,
        /// The value given.
        percent: f64,
    },
    /// A fabric roll must have a width.
    #[error("fabric {}: the roll width must be greater than zero", .fabric.index())]
    ZeroRollWidth {
        /// The fabric.
        fabric: FabricId,
    },
    /// Zero allowance is only for fabric whose cut edge does not fray.
    #[error("fabric {}: a fabric that frays needs a seam allowance greater than zero", .fabric.index())]
    ZeroAllowanceFrays {
        /// The fabric.
        fabric: FabricId,
    },
    /// Pile with a length lies one way, so it has a nap.
    #[error("fabric {}: a pile fabric with a pile length has a nap", .fabric.index())]
    PileWithoutNap {
        /// The fabric.
        fabric: FabricId,
    },
    /// A model length that is zero, negative or not finite.
    #[error("a model length of {model_length} units is not usable (must be finite and positive)")]
    BadModelLength {
        /// The length given, in model units.
        model_length: f64,
    },
    /// A finished size whose scale or longest dimension is zero or not finite.
    #[error("the finished size cannot be reached: {source}")]
    BadScale {
        /// Why the scale or length was refused.
        source: GeomError,
    },
    /// Allowances must come one per sewing-line edge.
    #[error("piece {}: {allowances} allowances for {edges} edges", .piece.index())]
    AllowanceCount {
        /// The piece.
        piece: PieceId,
        /// Edges on the sewing line.
        edges: usize,
        /// Allowances given.
        allowances: usize,
    },
    /// The guard past a zero-allowance edge (a fold or an edge-stitched edge) reaches into the
    /// piece itself. Edge `i` runs from sewing-line point `i` to the next.
    #[error("piece {}: the fold or edge-stitched edge {edge} faces another part of the piece too closely", .piece.index())]
    FoldConflict {
        /// The piece.
        piece: PieceId,
        /// The zero-allowance edge.
        edge: usize,
    },
    /// The allowances leave the cut line in more than one part.
    #[error("piece {}: the seam allowances split the cut line into separate parts", .piece.index())]
    AllowanceSplitsPiece {
        /// The piece.
        piece: PieceId,
    },
    /// The allowances close off a gap inside the cut line, such as a pocket's mouth.
    #[error("piece {}: the seam allowances close off a gap inside the piece", .piece.index())]
    AllowanceEnclosesGap {
        /// The piece.
        piece: PieceId,
    },
    /// The allowances make a cut line that touches or crosses itself.
    #[error("piece {}: the seam allowances make a cut line that touches itself", .piece.index())]
    AllowanceNotSimple {
        /// The piece.
        piece: PieceId,
    },
    /// An allowed rotation that is NaN or infinite.
    #[error("rotation {index} is not finite")]
    NonFiniteRotation {
        /// The rotation's index.
        index: usize,
    },
    /// A notch whose point is not finite, whose direction is zero or not finite, or with no marks.
    #[error("seam chain {}: a notch needs a finite point, a direction and at least one mark", .chain.index())]
    BadNotch {
        /// The chain it was made for.
        chain: SeamChainId,
    },
    /// A seam so long it would need more than 1000 notches.
    #[error(
        "seam chain {}: {:.0} mm is too long to notch (at most 1000 notches)",
        .chain.index(), .length.get()
    )]
    TooManyNotches {
        /// The chain.
        chain: SeamChainId,
        /// Its first side's length.
        length: Mm,
    },
}

fn direction_name(direction: StretchDirection) -> &'static str {
    match direction {
        StretchDirection::Grain => "grain",
        StretchDirection::Cross => "cross",
        StretchDirection::Bias => "bias",
    }
}
