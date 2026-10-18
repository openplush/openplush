//! Winding: every face made to agree with its neighbours, then each closed shell to
//! enclose a positive volume.

use openplush_geom::epsilon::ModelTolerances;

use crate::{FaceId, MeshError, TriangleSoup, VertexId};

/// What orienting changed, by the IDs of the soup it was given.
#[derive(Debug, Default)]
pub(crate) struct Oriented {
    /// Faces whose corners were reversed, sorted.
    pub(crate) flipped: Vec<FaceId>,
    /// Edges, smaller end first and sorted, whose two faces no flips make agree.
    pub(crate) conflicts: Vec<[VertexId; 2]>,
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "called by into_manifold, not yet written")
)]
pub(crate) fn orient(
    soup: &TriangleSoup,
    _tol: &ModelTolerances,
) -> Result<(TriangleSoup, Oriented), MeshError> {
    Ok((soup.clone(), Oriented::default()))
}

#[cfg(test)]
mod tests;
