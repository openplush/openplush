//! Edges with more than two faces: which faces share which copy of the edge.

mod pairing;

use std::collections::BTreeSet;

use pairing::Pairing;

use crate::{FaceId, MeshError, TriangleSoup, VertexId};

/// An edge with more than two faces, and how its faces pair up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairedEdge {
    /// The ends, smaller first.
    pub vertices: [VertexId; 2],
    /// Each pair of faces that share their own copy of the edge, the lower face
    /// ID first, sorted by it.
    pub pairs: Vec<[FaceId; 2]>,
    /// The face left without a partner when the count is odd; its copy is a
    /// boundary edge.
    pub unpaired: Option<FaceId>,
}

/// Pairs the faces on every edge with more than two, without changing the soup:
/// a pair sharing a copy of the edge is a manifold edge the half-edge build must
/// make, even where the copies join the same two vertices. `sheets` are the pairs
/// of coincident layers by this soup's face IDs: `degenerate::Removed::sheets` names
/// the faces of the soup `remove` was given, so map them through `Removed::origin`
/// first, or no layer is recognised. Requires repeated-index
/// and zero-area faces gone (`degenerate::remove`, `cap::stitch`); a face with no
/// area beside such an edge is refused with its error.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "called by into_manifold, not yet written")
)]
pub(crate) fn pair_edges(
    soup: &TriangleSoup,
    sheets: &[[FaceId; 2]],
) -> Result<Vec<PairedEdge>, MeshError> {
    let tris = soup.triangle_indices();
    // Lossless: the face count fits in u32 (`TriangleSoup::new`).
    let faces: Vec<u32> = (0..tris.len() as u32).collect();
    let sheets: BTreeSet<[u32; 2]> = sheets.iter().map(|[x, y]| [x.index(), y.index()]).collect();
    crate::validate::edges(tris, &faces)
        .iter()
        .filter(|(_, uses)| uses.len() > 2)
        .map(|(&edge, uses)| {
            let pairing = pairing::pair(soup.positions(), tris, edge, uses, &sheets)?;
            Ok(PairedEdge::new(edge, &pairing))
        })
        .collect()
}

impl PairedEdge {
    fn new([a, b]: [u32; 2], pairing: &Pairing) -> Self {
        Self {
            vertices: [VertexId::new(a), VertexId::new(b)],
            pairs: pairing
                .pairs
                .iter()
                .map(|&[x, y]| [FaceId::new(x), FaceId::new(y)])
                .collect(),
            unpaired: pairing.unpaired.map(FaceId::new),
        }
    }
}

#[cfg(test)]
mod tests;
