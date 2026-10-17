use std::collections::BTreeMap;

use super::{EdgeUse, Edges};
use crate::disjoint::DisjointSet;
use crate::{FaceId, VertexId};

/// Edges with more than two faces, faces in ID order.
pub(super) fn edges(edges: &Edges) -> Vec<super::NonManifoldEdge> {
    edges
        .iter()
        .filter(|(_, uses)| uses.len() > 2)
        .map(|(&[a, b], uses)| {
            let mut faces: Vec<FaceId> = uses.iter().map(|u| FaceId::new(u.face)).collect();
            faces.sort();
            super::NonManifoldEdge {
                vertices: [VertexId::new(a), VertexId::new(b)],
                faces,
            }
        })
        .collect()
}

/// Vertices whose faces split into more than one fan, faces joined when they share an
/// edge at the vertex, however many faces that edge has.
pub(super) fn vertices(edges: &Edges, tris: &[[u32; 3]], kept: &[u32]) -> Vec<VertexId> {
    let mut incident: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for &face in kept {
        // INVARIANT: kept faces index `tris`.
        for v in tris[face as usize] {
            incident.entry(v).or_default().push(face);
        }
    }
    let mut at_vertex: BTreeMap<u32, Vec<&[EdgeUse]>> = BTreeMap::new();
    for (&[a, b], uses) in edges {
        at_vertex.entry(a).or_default().push(uses);
        at_vertex.entry(b).or_default().push(uses);
    }
    let mut out = Vec::new();
    for (v, faces) in &incident {
        let mut fans = DisjointSet::new(faces.len());
        let local = |face: u32| faces.binary_search(&face).ok();
        for uses in at_vertex.get(v).into_iter().flatten() {
            let mut members = uses.iter().filter_map(|u| local(u.face));
            if let Some(first) = members.next() {
                for other in members {
                    // Lossless: a vertex has fewer faces than the soup, whose count fits in u32.
                    fans.union(first as u32, other as u32);
                }
            }
        }
        let roots = (0..faces.len() as u32)
            .filter(|&i| fans.find(i) == i)
            .count();
        if roots > 1 {
            out.push(VertexId::new(*v));
        }
    }
    out
}
