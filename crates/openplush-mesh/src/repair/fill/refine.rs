//! Liepa 2003's refinement: split patch triangles at their centroids until their size matches
//! the edge lengths around the rim, swapping edges after each round towards a Delaunay patch.

use std::collections::BTreeMap;

use openplush_geom::DVec3;
use openplush_geom::epsilon::ModelTolerances;

use crate::halfedge::is_sliver;

/// Liepa's density factor: a triangle splits when its centroid is further than σ/α from
/// each of its corners, σ being the local edge length.
const ALPHA: f64 = std::f64::consts::SQRT_2;

/// Bounds the rounds of splitting, and the swaps per round per triangle, so a patch that
/// does not settle is refused instead of looping.
const MAX_ROUNDS: usize = 64;
const SWAPS_PER_TRIANGLE: usize = 16;

/// A patch being refined: its first `rim` vertices are the loop's, in loop order; the rest
/// it added.
pub(super) struct Patch {
    pub(super) pos: Vec<DVec3>,
    /// Per vertex, the mean length of the edges around it.
    pub(super) sigma: Vec<f64>,
    pub(super) rim: usize,
    pub(super) tris: Vec<[usize; 3]>,
}

/// Refines `patch` in place; `joined(i, k)` says whether the mesh already joins rim vertices
/// `i < k`. `None` when it does not settle within the bounds.
pub(super) fn refine(
    patch: &mut Patch,
    joined: &dyn Fn(usize, usize) -> bool,
    tol: &ModelTolerances,
) -> Option<()> {
    for _ in 0..MAX_ROUNDS {
        if !split(patch) {
            return Some(());
        }
        relax(patch, joined, tol)?;
    }
    None
}

/// Splits every triangle the density rule says is too large, in index order; true if any.
fn split(patch: &mut Patch) -> bool {
    let mut any = false;
    for t in 0..patch.tris.len() {
        let [a, b, c] = patch.tris[t];
        let centroid = (patch.pos[a] + patch.pos[b] + patch.pos[c]) / 3.0;
        let sigma = (patch.sigma[a] + patch.sigma[b] + patch.sigma[c]) / 3.0;
        let large = [a, b, c].iter().all(|&v| {
            let d = ALPHA * (centroid - patch.pos[v]).length();
            d > sigma && d > patch.sigma[v]
        });
        if large {
            let q = patch.pos.len();
            patch.pos.push(centroid);
            patch.sigma.push(sigma);
            patch.tris[t] = [a, b, q];
            patch.tris.push([b, c, q]);
            patch.tris.push([c, a, q]);
            any = true;
        }
    }
    any
}

/// Swaps edges until none fails the Delaunay test, lowest edge first; `None` past the bound.
fn relax(
    patch: &mut Patch,
    joined: &dyn Fn(usize, usize) -> bool,
    tol: &ModelTolerances,
) -> Option<()> {
    for _ in 0..SWAPS_PER_TRIANGLE * patch.tris.len() {
        let Some((t1, t2, c, d)) = swappable(patch, joined, tol) else {
            return Some(());
        };
        let [a, b] = shared(patch.tris[t1], c);
        patch.tris[t1] = [a, d, c];
        patch.tris[t2] = [d, b, c];
    }
    None
}

/// The first edge, in vertex order, whose two triangles `(a, b, c)` and `(b, a, d)` would be
/// better as `(a, d, c)` and `(d, b, c)`: the angles opposite it sum past π, chord `c`–`d`
/// is new, and neither new triangle is a sliver or folds against the other.
fn swappable(
    patch: &Patch,
    joined: &dyn Fn(usize, usize) -> bool,
    tol: &ModelTolerances,
) -> Option<(usize, usize, usize, usize)> {
    let edges = edges(&patch.tris);
    for sides in edges.values() {
        let &[(t1, c), (t2, d)] = sides.as_slice() else {
            continue;
        };
        let key = (c.min(d), c.max(d));
        let exists = edges.contains_key(&key) || (key.1 < patch.rim && joined(key.0, key.1));
        if exists {
            continue;
        }
        let [a, b] = shared(patch.tris[t1], c);
        let p = |i: usize| patch.pos[i];
        if cot(p(c), p(a), p(b)) + cot(p(d), p(a), p(b)) >= 0.0 {
            continue;
        }
        let n1 = (p(d) - p(a)).cross(p(c) - p(a));
        let n2 = (p(b) - p(d)).cross(p(c) - p(d));
        if is_sliver(p(a), p(d), p(c), tol) || is_sliver(p(d), p(b), p(c), tol) || n1.dot(n2) <= 0.0
        {
            continue;
        }
        return Some((t1, t2, c, d));
    }
    None
}

/// Per undirected edge, the triangles on it, each with its third vertex, in triangle order.
fn edges(tris: &[[usize; 3]]) -> BTreeMap<(usize, usize), Vec<(usize, usize)>> {
    let mut out: BTreeMap<(usize, usize), Vec<(usize, usize)>> = BTreeMap::new();
    for (t, &[a, b, c]) in tris.iter().enumerate() {
        for (u, v, w) in [(a, b, c), (b, c, a), (c, a, b)] {
            out.entry((u.min(v), u.max(v))).or_default().push((t, w));
        }
    }
    out
}

/// The two vertices of `tri` other than `apex`, in the triangle's own order after it.
fn shared(tri: [usize; 3], apex: usize) -> [usize; 2] {
    // INVARIANT: `apex` is a vertex of `tri`: `edges` recorded it as that triangle's third.
    let k = tri.iter().position(|&v| v == apex).unwrap_or(0);
    [tri[(k + 1) % 3], tri[(k + 2) % 3]]
}

/// The cotangent of the angle at `at` between the directions to `p` and `q`.
fn cot(at: DVec3, p: DVec3, q: DVec3) -> f64 {
    let (u, v) = (p - at, q - at);
    u.dot(v) / u.cross(v).length()
}
