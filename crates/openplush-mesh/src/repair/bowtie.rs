//! Vertices whose faces form more than one fan, split into one vertex per fan.

use crate::{MeshError, TriangleSoup, VertexId};

/// The vertices a split added, by the IDs of the soup it was given.
#[derive(Debug, Default)]
pub(crate) struct Split {
    /// Each added vertex, sorted by the copy.
    pub(crate) copies: Vec<VertexCopy>,
}

/// A vertex added so one fan of faces no longer shares `of` with the others.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VertexCopy {
    pub(crate) of: VertexId,
    pub(crate) copy: VertexId,
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "called by into_manifold, not yet written")
)]
pub(crate) fn split(soup: &TriangleSoup) -> Result<(TriangleSoup, Split), MeshError> {
    Ok((soup.clone(), Split::default()))
}

#[cfg(test)]
mod tests;
