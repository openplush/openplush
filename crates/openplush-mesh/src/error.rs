use thiserror::Error;

use crate::{FaceId, VertexId};

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
    #[error("expected {expected} per-corner entries (three per triangle), found {found}")]
    CornerCountMismatch {
        /// Three times the triangle count.
        expected: usize,
        /// The length supplied.
        found: usize,
    },
    /// More vertices or triangles than a `u32` index can address.
    #[error("{count} elements exceed the u32 index space")]
    TooLarge {
        /// The element count.
        count: usize,
    },
}
