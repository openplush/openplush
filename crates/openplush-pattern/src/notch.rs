//! Match notches: where along a seam side they sit, and how many marks each seam's notches carry.

use std::collections::{BTreeMap, BTreeSet};

use openplush_geom::DVec2;
use openplush_geom::units::Mm;
use openplush_seams::SeamChainId;

use crate::PatternError;

/// The longest gap, in mm, between neighbouring notches on one side.
pub const SPACING_MM: f64 = 50.0;

/// The most notches between one side's ends: a 50 m seam, far past any plush.
const MAX_INNER: u32 = 1000;

/// A match notch on a piece's sewing line, in the piece's millimetres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Notch {
    at: DVec2,
    outward: DVec2,
    chain: SeamChainId,
    marks: u32,
}

impl Notch {
    /// `outward` is scaled to unit length. Refuses a point that is not finite, a direction that
    /// is zero or not finite, and zero marks.
    pub fn new(
        at: DVec2,
        outward: DVec2,
        chain: SeamChainId,
        marks: u32,
    ) -> Result<Self, PatternError> {
        let unit = outward.normalize();
        match unit {
            Some(outward) if at.is_finite() && marks > 0 => Ok(Self {
                at,
                outward,
                chain,
                marks,
            }),
            _ => Err(PatternError::BadNotch { chain }),
        }
    }

    /// Where it meets the sewing line.
    pub fn at(&self) -> DVec2 {
        self.at
    }

    /// The unit direction out of the piece, across the allowance.
    pub fn outward(&self) -> DVec2 {
        self.outward
    }

    /// The lead chain of the seam it matches along.
    pub fn chain(&self) -> SeamChainId {
        self.chain
    }

    /// How many marks are cut side by side: 1 at a seam's ends, its seam's mark count between.
    pub fn marks(&self) -> u32 {
        self.marks
    }
}

/// The fractions of its length, strictly between its ends, at which every side of `chain` is
/// notched when its first side is `length` long: evenly spaced at most `SPACING_MM` apart, and at
/// least the midpoint. Even spacing reads the same from either end, so the sides agree whichever
/// way each runs.
///
/// # Errors
/// `TooManyNotches` past 1000 notches between the ends.
pub fn fractions(chain: SeamChainId, length: Mm) -> Result<Vec<f64>, PatternError> {
    let gaps = (length.get() / SPACING_MM).ceil().max(2.0);
    if gaps > f64::from(MAX_INNER + 1) {
        return Err(PatternError::TooManyNotches { chain, length });
    }
    // Lossless: `gaps` is a whole number from 2 to 1001.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let gaps = gaps as u32;
    Ok((1..gaps).map(|k| f64::from(k) / f64::from(gaps)).collect())
}

/// Each chain's mark count: the least, from 1, that no chain it meets has, chains taken in
/// ascending order, so two seams that meet on a piece never carry the same count. `meets` holds
/// every chain, each with the chains it meets.
pub fn mark_counts(
    meets: &BTreeMap<SeamChainId, BTreeSet<SeamChainId>>,
) -> BTreeMap<SeamChainId, u32> {
    let mut counts = BTreeMap::new();
    for (&chain, others) in meets {
        let taken: BTreeSet<u32> = others
            .iter()
            .filter_map(|o| counts.get(o).copied())
            .collect();
        let count = (1..).find(|c| !taken.contains(c)).unwrap_or(u32::MAX);
        counts.insert(chain, count);
    }
    counts
}
