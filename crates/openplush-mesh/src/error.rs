use thiserror::Error;

use crate::{FaceId, MeshElement, VertexId};

/// Every structural refusal in this crate. Each variant names the element at
/// fault, so the UI can highlight it (crates/AGENTS.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum MeshError {
    /// A vertex position contains NaN or infinity. The import stage guarantees
    /// finite coordinates, so this is a bug or corrupt input upstream of it.
    #[error("vertex {vertex:?} has a non-finite coordinate")]
    NonFiniteVertex {
        /// The offending vertex.
        vertex: VertexId,
    },
    /// A triangle refers to a vertex that does not exist.
    #[error(
        "triangle {triangle:?} uses vertex index {index}, but there are only {vertex_count} vertices"
    )]
    IndexOutOfRange {
        /// Triangle position in the input.
        triangle: FaceId,
        /// The out-of-range index.
        index: u32,
        /// How many vertices exist.
        vertex_count: usize,
    },
    /// A per-corner array (provenance or UVs) does not have three entries per triangle.
    #[error("expected {expected} per-corner {array:?} (three per triangle), found {found}")]
    CornerCountMismatch {
        /// Which per-corner array has the wrong length.
        array: CornerArray,
        /// Three times the triangle count.
        expected: usize,
        /// The length supplied.
        found: usize,
    },
    /// More vertices or triangles than a `u32` index can address.
    #[error("{count} {what:?} exceed the u32 index space")]
    TooLarge {
        /// What was counted.
        what: Counted,
        /// The element count.
        count: usize,
    },
    /// A triangle uses the same vertex twice. Repair removes these; a manifold
    /// mesh cannot contain one.
    #[error("triangle {triangle:?} repeats a vertex")]
    DegenerateTriangle {
        /// The triangle.
        triangle: FaceId,
    },
    /// A triangle's area is at or below the model's area tolerance
    /// (`ModelTolerances::area`, data-model section 11).
    #[error("triangle {triangle:?} has (near-)zero area")]
    ZeroAreaTriangle {
        /// The triangle.
        triangle: FaceId,
    },
    /// A vertex no triangle uses. It has no one-ring, so it has no place in a
    /// half-edge mesh.
    #[error("vertex {vertex:?} is not used by any triangle")]
    IsolatedVertex {
        /// The vertex.
        vertex: VertexId,
    },
    /// More than two triangles share an edge.
    #[error("edge between {a:?} and {b:?} is shared by more than two triangles")]
    NonManifoldEdge {
        /// The edge's lower-indexed endpoint.
        a: VertexId,
        /// The edge's higher-indexed endpoint.
        b: VertexId,
    },
    /// Two triangles traverse their shared edge in the same direction, so their
    /// normals disagree.
    #[error("triangles sharing the edge between {a:?} and {b:?} have opposite orientations")]
    InconsistentOrientation {
        /// The edge's lower-indexed endpoint.
        a: VertexId,
        /// The edge's higher-indexed endpoint.
        b: VertexId,
    },
    /// A vertex's triangles do not form a single fan (disk or half-disk): for
    /// example two cones touching at their tips, or a bowtie.
    #[error("vertex {vertex:?} is non-manifold (its triangles form more than one fan)")]
    NonManifoldVertex {
        /// The vertex.
        vertex: VertexId,
    },
    /// `ManifoldMesh::debug_validate` found a broken data-model section 2 invariant.
    /// Construction never produces this; it means a bug in code that edits the mesh.
    #[error("half-edge invariant {invariant} is broken at {element:?}")]
    BrokenInvariant {
        /// Which invariant (1-5 as numbered in data-model section 2; 0 for the
        /// structural checks around them: array lengths, the boundary-vertex rule).
        invariant: u8,
        /// Where.
        element: MeshElement,
    },
}

/// The per-corner arrays a soup is built from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CornerArray {
    /// Provenance records (`SourceCorner`).
    Sources,
    /// Texture coordinates.
    Uvs,
}

/// What a [`MeshError::TooLarge`] counted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Counted {
    /// Vertices.
    Vertices,
    /// Triangles.
    Triangles,
    /// Half-edges (twice the edge count).
    HalfEdges,
}
