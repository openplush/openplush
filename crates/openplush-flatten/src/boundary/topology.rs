//! The chart's one boundary loop, after checking the chart is a disk.

use std::collections::{BTreeMap, BTreeSet};

use openplush_mesh::{FaceId, VertexId};

use super::BoundaryError;

/// The boundary loop from its lowest vertex, in the faces' winding. Refuses anything but a
/// connected, consistently wound surface with one boundary loop and Euler characteristic 1.
pub(super) fn boundary_loop(faces: &[(FaceId, [VertexId; 3])]) -> Result<Vec<u32>, BoundaryError> {
    let mut directed = BTreeSet::new();
    for (_, t) in faces {
        let [a, b, c] = t.map(VertexId::index);
        for edge in [(a, b), (b, c), (c, a)] {
            if !directed.insert(edge) {
                return Err(BoundaryError::NotADisk);
            }
        }
    }
    let boundary: Vec<(u32, u32)> = directed
        .iter()
        .copied()
        .filter(|&(a, b)| !directed.contains(&(b, a)))
        .collect();
    // A vertex with two boundary edges out (a pinch) keeps only one in the map; the walk then
    // misses an edge, and `follow` refuses it for not using all of them.
    let ring = follow(&boundary.iter().copied().collect(), boundary.len())?;
    let vertices: BTreeSet<u32> = directed.iter().map(|&(a, _)| a).collect();
    // Each interior edge is in `directed` twice and each boundary edge once.
    let edges = (directed.len() + boundary.len()) / 2;
    let euler = vertices.len() as i128 - edges as i128 + faces.len() as i128;
    if euler != 1 || !connected(&directed, ring[0], vertices.len()) {
        return Err(BoundaryError::NotADisk);
    }
    Ok(ring)
}

/// The loop from the lowest boundary vertex, refused unless it uses all `edges` boundary edges.
fn follow(next: &BTreeMap<u32, u32>, edges: usize) -> Result<Vec<u32>, BoundaryError> {
    let (&start, _) = next.first_key_value().ok_or(BoundaryError::NotADisk)?;
    let mut ring = vec![start];
    let mut at = start;
    loop {
        // INVARIANT: with no directed edge repeated, every vertex has as many boundary edges
        // out as in, so the end of a boundary edge starts one.
        at = next[&at];
        if at == start {
            break;
        }
        if ring.len() == edges {
            return Err(BoundaryError::NotADisk);
        }
        ring.push(at);
    }
    if ring.len() == edges {
        Ok(ring)
    } else {
        Err(BoundaryError::NotADisk)
    }
}

/// Whether every one of the `count` vertices is reachable from `from` along an edge.
fn connected(directed: &BTreeSet<(u32, u32)>, from: u32, count: usize) -> bool {
    let mut seen = BTreeSet::from([from]);
    let mut stack = vec![from];
    while let Some(v) = stack.pop() {
        for &(_, w) in directed.range((v, 0)..=(v, u32::MAX)) {
            if seen.insert(w) {
                stack.push(w);
            }
        }
    }
    seen.len() == count
}

#[cfg(test)]
mod tests;
