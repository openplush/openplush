//! Connected components of a soup, and which are small enough to be detail parts.

use crate::validate::topology;
use crate::{FaceId, TriangleSoup};

/// A set of faces joined through shared vertices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    /// Ascending.
    pub faces: Vec<FaceId>,
    /// Area under [`DETAIL_AREA_FRACTION`] of the soup's total.
    pub detail_candidate: bool,
}

/// A component with less than this fraction of the total area is a candidate detail part.
pub(crate) const DETAIL_AREA_FRACTION: f64 = 0.01;

/// The soup's components in order of their first face. A sheet's two layers both count
/// towards a component's area.
#[cfg_attr(not(test), expect(dead_code, reason = "called by into_manifold"))]
pub(crate) fn split(soup: &TriangleSoup) -> Vec<Component> {
    let tris = soup.triangle_indices();
    // Lossless: the face count fits in u32 (`TriangleSoup::new`).
    let faces: Vec<u32> = (0..tris.len() as u32).collect();
    let groups = topology::components(soup.vertex_count(), tris, &faces);
    let area = |face: &FaceId| {
        // INVARIANT: the groups hold faces of `soup`, whose vertices index its positions.
        let [a, b, c] = tris[face.index() as usize].map(|v| soup.positions()[v as usize]);
        0.5 * (b - a).cross(c - a).length()
    };
    let areas: Vec<f64> = groups.iter().map(|g| g.iter().map(area).sum()).collect();
    let total: f64 = areas.iter().sum();
    groups
        .into_iter()
        .zip(areas)
        .map(|(faces, area)| Component {
            faces,
            detail_candidate: area < DETAIL_AREA_FRACTION * total,
        })
        .collect()
}

#[cfg(test)]
mod tests;
