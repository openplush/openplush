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

/// A half-edge index. Twins are allocated in pairs `(2k, 2k + 1)`, so the twin and
/// the undirected edge are bit operations, not lookups (data-model section 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HalfEdgeId(u32);

impl HalfEdgeId {
    /// Wraps a raw index.
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// The raw index.
    pub const fn index(self) -> u32 {
        self.0
    }

    /// The oppositely directed half-edge of the same edge. Always valid: boundary
    /// half-edges are explicit, so every half-edge has a twin.
    #[inline]
    pub const fn twin(self) -> Self {
        Self(self.0 ^ 1)
    }

    /// The undirected edge this half-edge belongs to.
    #[inline]
    pub const fn edge(self) -> EdgeId {
        EdgeId(self.0 >> 1)
    }
}

/// An undirected edge index: a half-edge index shifted right by one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EdgeId(u32);

impl EdgeId {
    /// Wraps a raw index.
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// The raw index.
    pub const fn index(self) -> u32 {
        self.0
    }

    /// The edge's first half-edge; its twin is the second.
    pub const fn half_edge(self) -> HalfEdgeId {
        HalfEdgeId(self.0 << 1)
    }
}
