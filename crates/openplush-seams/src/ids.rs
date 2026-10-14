/// A chart: a connected region of faces between seams, numbered by its lowest
/// `FaceId`. Stable for one `SeamSet`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChartId(u32);

impl ChartId {
    /// Wraps a raw index.
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// The raw index.
    pub const fn index(self) -> u32 {
        self.0
    }
}

/// A seam chain: a maximal path of seam edges between non-chain
/// nodes, numbered by its lowest `EdgeId`. Stable for one `SeamSet`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SeamChainId(u32);

impl SeamChainId {
    /// Wraps a raw index.
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// The raw index.
    pub const fn index(self) -> u32 {
        self.0
    }
}
