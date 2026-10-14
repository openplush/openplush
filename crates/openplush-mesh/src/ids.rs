/// A vertex index. A newtype, never a raw integer, across every API
/// (crates/AGENTS.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VertexId(u32);

impl VertexId {
    /// Wraps a raw index.
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// The raw index.
    pub const fn index(self) -> u32 {
        self.0
    }
}

/// A face (triangle) index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FaceId(u32);

impl FaceId {
    /// Wraps a raw index.
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// The raw index.
    pub const fn index(self) -> u32 {
        self.0
    }
}
