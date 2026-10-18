use openplush_geom::GeomError;
use openplush_geom::units::Mm;
use openplush_seams::SeamChainId;
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
    /// A piece needs at least one allowed rotation.
    #[error("a piece needs at least one allowed rotation")]
    NoRotations,
    /// An allowed rotation that is NaN or infinite.
    #[error("rotation {index} is not finite")]
    NonFiniteRotation {
        /// The rotation's index.
        index: usize,
    },
}
