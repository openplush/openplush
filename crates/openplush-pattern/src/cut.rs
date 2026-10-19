use crate::PatternError;

/// How many copies of a template to cut as printed and how many with the template flipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CutCount {
    as_printed: u32,
    flipped: u32,
}

impl CutCount {
    /// The most copies of one template a pattern may ask for, both counts together.
    pub(crate) const MAX_TOTAL: u32 = 100;

    /// Refuses a total of zero or over 100; either count alone may be zero.
    pub fn new(as_printed: u32, flipped: u32) -> Result<Self, PatternError> {
        let total = u64::from(as_printed) + u64::from(flipped);
        if total == 0 {
            return Err(PatternError::ZeroCutQuantity);
        }
        if total > u64::from(Self::MAX_TOTAL) {
            return Err(PatternError::TooManyCuts { total });
        }
        Ok(Self {
            as_printed,
            flipped,
        })
    }

    /// Copies cut with the template as printed.
    pub fn as_printed(self) -> u32 {
        self.as_printed
    }

    /// Copies cut with the template flipped over.
    pub fn flipped(self) -> u32 {
        self.flipped
    }

    /// Every copy, 1 to 100.
    pub fn total(self) -> u32 {
        self.as_printed + self.flipped
    }
}
