//! Connected components of a soup, and which are small enough to be detail parts.

use crate::{FaceId, TriangleSoup};

/// A set of faces joined through shared vertices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Component {
    /// Ascending.
    pub(crate) faces: Vec<FaceId>,
    /// Area under [`DETAIL_AREA_FRACTION`] of the soup's total.
    pub(crate) detail_candidate: bool,
}

/// A component with less than this fraction of the total area is a candidate detail part.
pub(crate) const DETAIL_AREA_FRACTION: f64 = 0.01;

/// The soup's components in order of their first face.
#[cfg_attr(not(test), expect(dead_code, reason = "called by into_manifold"))]
pub(crate) fn split(_soup: &TriangleSoup) -> Vec<Component> {
    Vec::new()
}

#[cfg(test)]
mod tests;
