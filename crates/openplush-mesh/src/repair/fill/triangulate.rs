//! Liepa 2003's hole triangulation: of every triangulation of a boundary loop, the one whose
//! largest dihedral angle is smallest, then whose area is smallest, by dynamic programming
//! over the loop's vertices.

use std::cmp::Ordering;

use openplush_geom::DVec3;
use openplush_geom::epsilon::ModelTolerances;

use crate::halfedge::is_sliver;

/// A loop to triangulate.
pub(super) struct Rim<'a> {
    /// The loop's vertices in loop order; edge `i` runs from vertex `i` to vertex `i + 1`,
    /// the last back to the first.
    pub(super) points: &'a [DVec3],
    /// The unit normal of the mesh face across each loop edge.
    pub(super) across: &'a [DVec3],
    /// Whether the mesh already joins two loop vertices, `i < k`, by an edge.
    pub(super) joined: &'a dyn Fn(usize, usize) -> bool,
}

/// A triangulation's cost: the largest dihedral angle, as one minus the cosine between unit
/// normals (so 0 is flat and 2 is folded back), then the total area. Comparing cosines keeps
/// the choice free of `acos`, whose last bit differs between platforms.
#[derive(Clone, Copy)]
struct Weight {
    angle: f64,
    area: f64,
}

impl Weight {
    fn cmp(self, other: Self) -> Ordering {
        self.angle
            .total_cmp(&other.angle)
            .then(self.area.total_cmp(&other.area))
    }
}

/// The best triangulation of the sub-loop from vertex `i` to vertex `k`, and its apex.
#[derive(Clone, Copy)]
struct Cell {
    weight: Weight,
    apex: usize,
}

/// The triangles, as loop indices `[i, m, k]` with `i < m < k`, each wound the loop's way; `None`
/// when every triangulation reuses an existing edge as a chord or has a triangle with no area.
pub(super) fn triangulate(rim: &Rim<'_>, tol: &ModelTolerances) -> Option<Vec<[usize; 3]>> {
    let n = rim.points.len();
    let mut best: Vec<Option<Cell>> = vec![None; n * n];
    for i in 0..n - 1 {
        best[i * n + i + 1] = Some(Cell {
            weight: Weight {
                angle: 0.0,
                area: 0.0,
            },
            apex: usize::MAX,
        });
    }
    for gap in 2..n {
        for i in 0..n - gap {
            let k = i + gap;
            // A chord the mesh already has would make an edge of three or more faces.
            if gap < n - 1 && (rim.joined)(i, k) {
                continue;
            }
            best[i * n + k] = cell(rim, &best, i, k, tol);
        }
    }
    let mut out = Vec::new();
    let mut todo = vec![(0, n - 1)];
    while let Some((i, k)) = todo.pop() {
        if k - i < 2 {
            continue;
        }
        let m = best[i * n + k]?.apex;
        out.push([i, m, k]);
        todo.push((i, m));
        todo.push((m, k));
    }
    Some(out)
}

/// The cheapest apex for the sub-loop `i..=k`, ties to the lowest.
fn cell(
    rim: &Rim<'_>,
    best: &[Option<Cell>],
    i: usize,
    k: usize,
    tol: &ModelTolerances,
) -> Option<Cell> {
    let n = rim.points.len();
    let mut chosen: Option<Cell> = None;
    for m in i + 1..k {
        let (Some(left), Some(right)) = (best[i * n + m], best[m * n + k]) else {
            continue;
        };
        let [a, b, c] = [rim.points[i], rim.points[m], rim.points[k]];
        if is_sliver(a, b, c, tol) {
            continue;
        }
        let cross = (b - a).cross(c - a);
        let Some(normal) = cross.normalize() else {
            continue;
        };
        let bend = |other: DVec3| 1.0 - normal.dot(other);
        let mut angle = left
            .weight
            .angle
            .max(right.weight.angle)
            .max(bend(neighbour(rim, best, i, m)))
            .max(bend(neighbour(rim, best, m, k)));
        if i == 0 && k == n - 1 {
            angle = angle.max(bend(rim.across[n - 1]));
        }
        let weight = Weight {
            angle,
            area: left.weight.area + right.weight.area + 0.5 * cross.length(),
        };
        if chosen.is_none_or(|c| weight.cmp(c.weight) == Ordering::Less) {
            chosen = Some(Cell { weight, apex: m });
        }
    }
    chosen
}

/// The unit normal across edge `a`–`b` (`a < b`) from the triangle being chosen: the mesh
/// face for a loop edge, else the sub-loop's own apex triangle.
fn neighbour(rim: &Rim<'_>, best: &[Option<Cell>], a: usize, b: usize) -> DVec3 {
    let n = rim.points.len();
    if b == a + 1 {
        return rim.across[a];
    }
    // INVARIANT: the caller found both sub-loop cells set, and a set cell of gap two or more
    // has an apex strictly between its ends.
    let apex = best[a * n + b].map_or(a + 1, |c| c.apex);
    let [p, q, r] = [rim.points[a], rim.points[apex], rim.points[b]];
    // INVARIANT: a chosen triangle is not a sliver, so its normal has a direction.
    (q - p).cross(r - p).normalize().unwrap_or(DVec3::ZERO)
}

#[cfg(test)]
mod tests;
