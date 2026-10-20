//! Which copy of an edge each face uses: the copies a pairing makes on an over-shared edge,
//! and the two a conflict cut makes on a two-face edge.

use std::collections::BTreeMap;

use crate::repair::orientation::Conflict;
use crate::{FaceId, MeshError, PairedEdge, VertexId};

/// Which copy of an edge a face uses, for the faces a pairing names: keyed by (face,
/// lower, higher) vertex. A face not here uses copy 0.
pub(super) type Copies = BTreeMap<(u32, u32, u32), u32>;

/// A triangle's three directed edges, corner i to corner i + 1.
pub(super) fn corners_out(&[a, b, c]: &[u32; 3]) -> [[u32; 2]; 3] {
    [[a, b], [b, c], [c, a]]
}

/// The key of the edge triangle `t` crosses from `a` to `b`.
pub(super) fn edge_key(copies: &Copies, t: usize, a: u32, b: u32) -> (u32, u32, u32) {
    let (lo, hi) = (a.min(b), a.max(b));
    let copy = copies.get(&(t as u32, lo, hi)).copied().unwrap_or(0);
    (lo, hi, copy)
}

/// Gives each pair of an edge's pairing a copy of its own (1, 2, ...) and a face left
/// unpaired the next. A pairing must be on an edge with more than two faces and name
/// every one of them, once; otherwise it would cut an edge the soup does not need cut.
pub(super) fn edge_copies(tris: &[[u32; 3]], pairings: &[PairedEdge]) -> Result<Copies, MeshError> {
    let mut users: BTreeMap<(u32, u32), Vec<u32>> = BTreeMap::new();
    for (t, tri) in tris.iter().enumerate() {
        for [x, y] in corners_out(tri) {
            // Lossless: the face count fits in u32 (`TriangleSoup::new`).
            users
                .entry((x.min(y), x.max(y)))
                .or_default()
                .push(t as u32);
        }
    }
    let mut copies = Copies::new();
    for pairing in pairings {
        let [a, b] = pairing.vertices.map(VertexId::index);
        let on_edge = users.get(&(a, b)).map_or(&[][..], Vec::as_slice);
        let groups: Vec<Vec<FaceId>> = pairing
            .pairs
            .iter()
            .map(|pair| pair.to_vec())
            .chain(pairing.unpaired.map(|face| vec![face]))
            .collect();
        let named: Vec<FaceId> = groups.iter().flatten().copied().collect();
        let wrong = |face| MeshError::PairingMismatch {
            face,
            a: VertexId::new(a),
            b: VertexId::new(b),
        };
        for (copy, group) in (1u32..).zip(&groups) {
            for &face in group {
                let fresh = copies.insert((face.index(), a, b), copy).is_none();
                if !on_edge.contains(&face.index()) || !fresh {
                    return Err(wrong(face));
                }
            }
        }
        if on_edge.len() <= 2 {
            if let Some(&face) = named.first() {
                return Err(wrong(face));
            }
        } else if let Some(&left) = on_edge
            .iter()
            .find(|&&t| !named.iter().any(|n| n.index() == t))
        {
            return Err(wrong(FaceId::new(left)));
        }
    }
    Ok(copies)
}

/// Whether triangle `tri` has the edge between `a` and `b`.
fn has_edge(tri: &[u32; 3], a: u32, b: u32) -> bool {
    corners_out(tri)
        .iter()
        .any(|&[x, y]| (x.min(y), x.max(y)) == (a, b))
}

/// Gives each face of a conflict edge a copy of its own. A conflict must name exactly the
/// two faces of a two-face edge; otherwise it would cut an edge the soup does not need cut.
pub(super) fn cut_copies(
    tris: &[[u32; 3]],
    conflicts: &[Conflict],
    copies: &mut Copies,
) -> Result<(), MeshError> {
    for conflict in conflicts {
        let [a, b] = conflict.vertices.map(VertexId::index);
        let wrong = |face| MeshError::PairingMismatch {
            face,
            a: VertexId::new(a),
            b: VertexId::new(b),
        };
        if tris.iter().filter(|tri| has_edge(tri, a, b)).count() != 2 {
            return Err(wrong(conflict.faces[0]));
        }
        for (copy, &face) in (1u32..).zip(&conflict.faces) {
            let uses = tris
                .get(face.index() as usize)
                .is_some_and(|tri| has_edge(tri, a, b));
            if !uses || copies.insert((face.index(), a, b), copy).is_some() {
                return Err(wrong(face));
            }
        }
    }
    Ok(())
}
