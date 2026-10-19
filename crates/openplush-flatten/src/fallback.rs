//! A boundary that will not close: how far it misses, and a dart that would relieve it. Parity
//! is never relaxed; the maker cuts instead.

use std::collections::{BTreeMap, BTreeSet};

use openplush_geom::DVec3;
use openplush_mesh::{FaceId, VertexId};
use openplush_seams::ChartId;

use crate::FlattenError;
use crate::boundary::{self, Open};

type Faces<'a> = &'a [(FaceId, [VertexId; 3])];

/// The `BoundaryNotRealizable` for a chart, or the refusal that stops it being measured.
pub(crate) fn not_realizable(chart: ChartId, xyz: &[DVec3], faces: Faces<'_>) -> FlattenError {
    let open = match boundary::open_polygon(xyz, faces) {
        Ok(open) => open,
        Err(e) => return crate::interior::refuse_measurement(chart, e),
    };
    FlattenError::BoundaryNotRealizable {
        chart,
        residual: open.gap,
        dart: dart(xyz, faces, &open),
    }
}

/// The shortest edge path from the rim to the interior vertex with the most angle defect
/// (the smallest vertex number on a tie), rim end first.
fn dart(xyz: &[DVec3], faces: Faces<'_>, open: &Open) -> Option<Vec<VertexId>> {
    let rim: BTreeSet<u32> = open.ring.iter().copied().collect();
    let tip = open
        .angle_sum
        .iter()
        .filter(|(v, _)| !rim.contains(v))
        // The least angle sum is the most defect; `min_by` keeps the first, the smallest vertex.
        .min_by(|a, b| a.1.total_cmp(b.1))
        .map(|(&v, _)| v)?;
    let mut neighbours: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    for (_, t) in faces {
        let t = t.map(VertexId::index);
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            neighbours.entry(a).or_default().insert(b);
            neighbours.entry(b).or_default().insert(a);
        }
    }
    let length = |a: u32, b: u32| (xyz[a as usize] - xyz[b as usize]).length();
    let mut dist: BTreeMap<u32, f64> = rim.iter().map(|&v| (v, 0.0)).collect();
    let mut before: BTreeMap<u32, u32> = BTreeMap::new();
    // Distances are non-negative, so their bit patterns order like the numbers.
    let mut queue: BTreeSet<(u64, u32)> = rim.iter().map(|&v| (0.0_f64.to_bits(), v)).collect();
    while let Some((bits, v)) = queue.pop_first() {
        if v == tip {
            break;
        }
        let here = f64::from_bits(bits);
        for &w in neighbours.get(&v).into_iter().flatten() {
            let through = here + length(v, w);
            if dist.get(&w).is_none_or(|&d| through < d) {
                if let Some(&old) = dist.get(&w) {
                    queue.remove(&(old.to_bits(), w));
                }
                dist.insert(w, through);
                before.insert(w, v);
                queue.insert((through.to_bits(), w));
            }
        }
    }
    dist.get(&tip)?;
    let mut path = vec![tip];
    while let Some(&v) = before.get(path.last()?) {
        path.push(v);
    }
    path.reverse();
    Some(path.into_iter().map(VertexId::new).collect())
}

#[cfg(test)]
mod tests;
