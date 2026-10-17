use std::collections::BTreeMap;

use openplush_geom::DVec3;

use super::disjoint::DisjointSet;
use crate::{TriangleSoup, VertexId};

/// Used vertices merged within the weld distance: each vertex's representative (the
/// smallest index it is welded to) and every group of two or more.
pub(super) struct Weld {
    pub(super) rep: Vec<u32>,
    pub(super) groups: Vec<Vec<VertexId>>,
}

/// Joins any two used vertices within `distance`, transitively, as repair's weld
/// will. Vertices are bucketed into cubic cells `distance` wide, measured from the
/// used vertices' lowest corner, and each is compared only with its own and the 26
/// neighbouring cells: a sort along one axis would compare every pair in a flat,
/// axis-aligned model.
// INVARIANT: soup indices are in range (`TriangleSoup::new`).
pub(super) fn weld(soup: &TriangleSoup, distance: f64) -> Weld {
    let positions = soup.positions();
    let mut used = vec![false; positions.len()];
    for &i in soup.triangle_indices().iter().flatten() {
        used[i as usize] = true;
    }
    // Lossless: the vertex count fits in u32 (`TriangleSoup::new`).
    let order: Vec<u32> = (0..positions.len() as u32)
        .filter(|&i| used[i as usize])
        .collect();
    let low = order.iter().map(|&i| positions[i as usize]).fold(
        DVec3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY),
        |m, p| DVec3::new(m.x.min(p.x), m.y.min(p.y), m.z.min(p.z)),
    );
    // Every coordinate offset is at most the diagonal, about 1e6 weld distances, so
    // the cell coordinates are small integers.
    let cell = |p: DVec3| {
        let d = (p - low) * (1.0 / distance);
        [d.x.floor() as i64, d.y.floor() as i64, d.z.floor() as i64]
    };
    let mut cells: BTreeMap<[i64; 3], Vec<u32>> = BTreeMap::new();
    for &i in &order {
        cells
            .entry(cell(positions[i as usize]))
            .or_default()
            .push(i);
    }
    let mut set = DisjointSet::new(positions.len());
    for &a in &order {
        let pa = positions[a as usize];
        let [x, y, z] = cell(pa);
        for neighbour in neighbours([x, y, z]) {
            for &b in cells.get(&neighbour).into_iter().flatten() {
                if b > a && (positions[b as usize] - pa).length() <= distance {
                    set.union(a, b);
                }
            }
        }
    }
    let rep: Vec<u32> = (0..positions.len() as u32).map(|i| set.find(i)).collect();
    let mut groups: BTreeMap<u32, Vec<VertexId>> = BTreeMap::new();
    for &i in &order {
        groups
            .entry(rep[i as usize])
            .or_default()
            .push(VertexId::new(i));
    }
    let groups = groups.into_values().filter(|g| g.len() > 1).collect();
    Weld { rep, groups }
}

fn neighbours([x, y, z]: [i64; 3]) -> impl Iterator<Item = [i64; 3]> {
    (-1..=1).flat_map(move |dx| {
        (-1..=1).flat_map(move |dy| (-1..=1).map(move |dz| [x + dx, y + dy, z + dz]))
    })
}
