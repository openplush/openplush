//! Winding: every face made to agree with its neighbours, then each closed shell to
//! enclose a positive volume.

use std::collections::BTreeSet;

use openplush_geom::epsilon::ModelTolerances;

use crate::validate::orientation::{closed_volumes, walk};
use crate::{FaceId, MeshError, TriangleSoup, VertexId};

/// What orienting changed, by the IDs of the soup it was given.
#[derive(Debug, Default)]
pub(crate) struct Oriented {
    /// Faces whose corners were reversed, sorted.
    #[cfg_attr(not(test), expect(dead_code, reason = "read by into_manifold"))]
    pub(crate) flipped: Vec<FaceId>,
    /// Edges, smaller end first and sorted, whose two faces no flips make agree.
    #[cfg_attr(not(test), expect(dead_code, reason = "read by into_manifold"))]
    pub(crate) conflicts: Vec<[VertexId; 2]>,
}

/// Turns faces over so every pair across a two-face edge agrees, walking from the
/// largest face of each component (lowest ID among equals), then turns over each
/// closed shell whose volume is negative. A shell holding a conflict edge has no
/// inside to judge and keeps the walk's result. Edges with more than two faces are
/// not walked, so a sheet's layers, which share every edge, keep their winding. A
/// face that repeats a vertex is refused (`degenerate::remove` drops them first).
#[cfg_attr(not(test), expect(dead_code, reason = "called by into_manifold"))]
pub(crate) fn orient(
    soup: &TriangleSoup,
    tol: &ModelTolerances,
) -> Result<(TriangleSoup, Oriented), MeshError> {
    let tris = soup.triangle_indices();
    if let Some(f) = tris
        .iter()
        .position(|&[a, b, c]| a == b || b == c || c == a)
    {
        return Err(MeshError::DegenerateTriangle {
            // Lossless: the face count fits in u32 (`TriangleSoup::new`).
            triangle: FaceId::new(f as u32),
        });
    }
    // Lossless: the face count fits in u32 (`TriangleSoup::new`).
    let faces: Vec<u32> = (0..tris.len() as u32).collect();
    let edges = crate::validate::edges(tris, &faces);
    let walk = walk(&edges, tris, &by_area_descending(soup));
    let mut turn: Vec<bool> = walk.flip.iter().map(|&flip| flip == Some(true)).collect();
    let walked: Vec<[u32; 3]> = tris
        .iter()
        .zip(&turn)
        .map(|(&t, &t_turn)| turned(t, t_turn))
        .collect();
    let stuck: BTreeSet<u32> = walk
        .conflicts
        .iter()
        .flat_map(|key| edges[key].iter().map(|u| u.face))
        .collect();
    let volumes = closed_volumes(&edges, &walked, &faces, soup.positions(), tol.diagonal());
    for (shell, volume) in volumes {
        if volume < 0.0 && !shell.iter().any(|f| stuck.contains(f)) {
            shell.into_iter().for_each(|f| turn[f as usize] ^= true);
        }
    }
    let done = Oriented {
        // Lossless: the face count fits in u32 (`TriangleSoup::new`).
        flipped: (0..turn.len())
            .filter(|&f| turn[f])
            .map(|f| FaceId::new(f as u32))
            .collect(),
        conflicts: walk
            .conflicts
            .into_iter()
            .map(|[a, b]| [VertexId::new(a), VertexId::new(b)])
            .collect(),
    };
    Ok((turn_faces(soup, &turn), done))
}

/// Face IDs, largest area first, equal areas by ID.
fn by_area_descending(soup: &TriangleSoup) -> Vec<u32> {
    let pos = soup.positions();
    let area = |&[a, b, c]: &[u32; 3]| {
        let (a, b, c) = (pos[a as usize], pos[b as usize], pos[c as usize]);
        (b - a).cross(c - a).length_squared()
    };
    let areas: Vec<f64> = soup.triangle_indices().iter().map(area).collect();
    // Lossless: the face count fits in u32 (`TriangleSoup::new`).
    let mut order: Vec<u32> = (0..areas.len() as u32).collect();
    order.sort_by(|&x, &y| {
        areas[y as usize]
            .total_cmp(&areas[x as usize])
            .then(x.cmp(&y))
    });
    order
}

fn turned([a, b, c]: [u32; 3], turn: bool) -> [u32; 3] {
    if turn { [a, c, b] } else { [a, b, c] }
}

/// Swaps the second and third of each marked face's three slots.
fn swap_turned<T>(slots: &mut [T], turn: &[bool]) {
    for (face, _) in turn.iter().enumerate().filter(|&(_, &t)| t) {
        slots.swap(face * 3 + 1, face * 3 + 2);
    }
}

/// The soup with each marked face's second and third corners (and their records) swapped.
fn turn_faces(soup: &TriangleSoup, turn: &[bool]) -> TriangleSoup {
    let triangles = soup
        .triangle_indices()
        .iter()
        .zip(turn)
        .map(|(&t, &t_turn)| turned(t, t_turn))
        .collect();
    let mut sources = soup.corner_sources().to_vec();
    swap_turned(&mut sources, turn);
    let uvs = soup.corner_uvs().map(|uvs| {
        let mut uvs = uvs.to_vec();
        swap_turned(&mut uvs, turn);
        uvs
    });
    soup.with_faces(triangles, sources, uvs)
}

#[cfg(test)]
mod tests;
