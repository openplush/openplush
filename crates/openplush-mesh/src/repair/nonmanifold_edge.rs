//! Edges with more than two faces: which faces share which copy of the edge.

use crate::{FaceId, MeshError, TriangleSoup, VertexId};

/// An edge with more than two faces, and how its faces pair up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PairedEdge {
    /// The ends, smaller first.
    pub(crate) vertices: [VertexId; 2],
    /// Each pair of faces that share their own copy of the edge, the lower face
    /// ID first, sorted by it.
    pub(crate) pairs: Vec<[FaceId; 2]>,
    /// The face left without a partner when the count is odd; its copy is a
    /// boundary edge.
    pub(crate) unpaired: Option<FaceId>,
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "called by into_manifold, not yet written")
)]
pub(crate) fn pair_edges(
    _soup: &TriangleSoup,
    _sheets: &[[FaceId; 2]],
) -> Result<Vec<PairedEdge>, MeshError> {
    Ok(Vec::new())
}

#[cfg(test)]
mod tests;
