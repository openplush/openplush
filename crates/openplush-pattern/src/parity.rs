use openplush_geom::epsilon::SeamParity;
use openplush_geom::units::Mm;
use openplush_seams::SeamChainId;

use crate::PatternError;

/// Passes if the two sides of `chain` agree within max(1.0 mm, 0.5% of the longer side).
pub fn check(chain: SeamChainId, a: Mm, b: Mm) -> Result<(), PatternError> {
    if SeamParity::DEFAULT.within(a, b) {
        return Ok(());
    }
    let longer = if a >= b { a } else { b };
    Err(PatternError::SeamParity {
        chain,
        a,
        b,
        tolerance: SeamParity::DEFAULT.allowed(longer),
    })
}
