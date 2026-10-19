//! The turning the boundary would take if the interior lay flat with the boundary unscaled:
//! BFF's discrete Yamabe step with every boundary scale factor at zero (Sawhney & Crane 2017).

use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::{PI, TAU};

use openplush_geom::DVec3;
use openplush_mesh::{FaceId, VertexId};

use super::BoundaryError;
use crate::solver;

/// The intrinsic quantities the curvature step needs.
pub(super) struct Intrinsic {
    /// Each used vertex's sum of corner angles.
    pub(super) angle_sum: BTreeMap<u32, f64>,
    /// Each edge's cotangent weight (half the cotangents of the angles opposite it), keyed
    /// smaller vertex first; with them the cotangent Laplacian is positive semidefinite.
    pub(super) weight: BTreeMap<(u32, u32), f64>,
}

/// Corner angles and cotangent weights. Refuses a face naming a vertex outside `xyz` and a
/// face with zero or non-finite area, whose angles are undefined.
pub(super) fn intrinsic(
    xyz: &[DVec3],
    faces: &[(FaceId, [VertexId; 3])],
) -> Result<Intrinsic, BoundaryError> {
    if faces.is_empty() {
        return Err(BoundaryError::Empty);
    }
    let mut out = Intrinsic {
        angle_sum: BTreeMap::new(),
        weight: BTreeMap::new(),
    };
    for &(face, t) in faces {
        let t = t.map(VertexId::index);
        let [Some(pa), Some(pb), Some(pc)] = t.map(|v| xyz.get(v as usize).copied()) else {
            return Err(BoundaryError::BadVertex { face });
        };
        let p = [pa, pb, pc];
        for k in 0..3 {
            let (a, b, c) = (k, (k + 1) % 3, (k + 2) % 3);
            let (e1, e2) = (p[b] - p[a], p[c] - p[a]);
            let (sin, cos) = (e1.cross(e2).length(), e1.dot(e2));
            if !(sin > 0.0 && sin.is_finite() && cos.is_finite()) {
                return Err(BoundaryError::Degenerate { face });
            }
            *out.angle_sum.entry(t[a]).or_default() += libm::atan2(sin, cos);
            let edge = (t[b].min(t[c]), t[b].max(t[c]));
            *out.weight.entry(edge).or_default() += 0.5 * cos / sin;
        }
    }
    Ok(out)
}

/// The turning at each corner of `ring`. Interior conformal factors `u` solve
/// `L_II u = -K_I` (zero Gaussian curvature inside), and each boundary corner then turns by
/// its own geodesic curvature plus `(L_BI u)`; the turns total 2π by Gauss–Bonnet.
pub(super) fn turning(geometry: &Intrinsic, ring: &[u32]) -> Result<Vec<f64>, BoundaryError> {
    let on_ring: BTreeSet<u32> = ring.iter().copied().collect();
    let inner: BTreeMap<u32, usize> = geometry
        .angle_sum
        .keys()
        .filter(|v| !on_ring.contains(v))
        .zip(0..)
        .map(|(&v, i)| (v, i))
        .collect();
    let u = if inner.is_empty() {
        Vec::new()
    } else {
        conformal_factors(geometry, &inner)?
    };
    // INVARIANT: `ring` and `inner` hold only vertices of faces, which all have angle sums.
    let mut kappa: BTreeMap<u32, f64> = ring
        .iter()
        .map(|&v| (v, PI - geometry.angle_sum[&v]))
        .collect();
    for (&(a, b), &w) in &geometry.weight {
        for (outer, other) in [(a, b), (b, a)] {
            if let (Some(k), Some(&i)) = (kappa.get_mut(&outer), inner.get(&other)) {
                // INVARIANT: `u` has one entry per interior vertex, numbered as in `inner`.
                *k -= w * u[i];
            }
        }
    }
    // INVARIANT: `kappa` has a key for every vertex of `ring`.
    Ok(ring.iter().map(|v| kappa[v]).collect())
}

/// Solves `L_II u = -K_I` over the interior vertices `inner`, numbered in vertex order.
fn conformal_factors(
    geometry: &Intrinsic,
    inner: &BTreeMap<u32, usize>,
) -> Result<Vec<f64>, BoundaryError> {
    let mut diagonal = vec![0.0; inner.len()];
    let mut lower = Vec::new();
    let mut off = Vec::new();
    for (&(a, b), &w) in &geometry.weight {
        let (ia, ib) = (inner.get(&a), inner.get(&b));
        for &i in ia.iter().chain(&ib) {
            diagonal[*i] += w;
        }
        if let (Some(&i), Some(&j)) = (ia, ib) {
            lower.push((i.max(j), i.min(j)));
            off.push(-w);
        }
    }
    let entries: Vec<(usize, usize)> = (0..inner.len()).map(|i| (i, i)).chain(lower).collect();
    let values: Vec<f64> = diagonal.into_iter().chain(off).collect();
    // INVARIANT: every interior vertex is a vertex of a face, so it has an angle sum.
    let rhs: Vec<f64> = inner.keys().map(|v| geometry.angle_sum[v] - TAU).collect();
    Ok(solver::analyse(inner.len(), &entries)?
        .factor(&values)?
        .solve(&rhs)?)
}

#[cfg(test)]
mod tests;
