/// Defines a public `u32` index newtype with `new`, `index`, and the derives every ID shares; only for IDs other crates use.
#[macro_export]
macro_rules! id_newtype {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(u32);

        impl $name {
            /// Wraps a raw index.
            pub const fn new(index: u32) -> Self {
                Self(index)
            }

            /// The raw index.
            pub const fn index(self) -> u32 {
                self.0
            }
        }
    };
}

id_newtype! {
    /// A vertex index. A newtype, never a raw integer, across every API
    /// (crates/AGENTS.md).
    VertexId
}

id_newtype! {
    /// A face (triangle) index.
    FaceId
}

id_newtype! {
    /// A half-edge index. Twins are allocated in pairs `(2k, 2k + 1)`, so the twin and
    /// the undirected edge are bit operations, not lookups.
    HalfEdgeId
}

impl HalfEdgeId {
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

id_newtype! {
    /// An undirected edge index: a half-edge index shifted right by one.
    EdgeId
}

impl EdgeId {
    /// The edge's first half-edge; its twin is the second.
    pub const fn half_edge(self) -> HalfEdgeId {
        HalfEdgeId(self.0 << 1)
    }
}

/// Any mesh element, for errors that can point at more than one kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshElement {
    /// A vertex.
    Vertex(VertexId),
    /// A half-edge.
    HalfEdge(HalfEdgeId),
    /// A face.
    Face(FaceId),
    /// A whole per-element array, named, whose length disagrees with the others.
    Storage(&'static str),
}

id_newtype! {
    /// A connected component of a repaired mesh, numbered by its lowest `FaceId`.
    /// Created by repair; stable for one repaired mesh.
    ComponentId
}
