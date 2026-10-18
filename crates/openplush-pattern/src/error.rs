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
}
