//! One SLIM step: the weighted least-squares problem that pulls every face toward its closest
//! rotation, solved for the free vertices, and the line search that keeps it a descent
//! (Rabinovich et al. 2017).

use std::collections::{BTreeMap, BTreeSet};

use openplush_geom::DVec2;
use openplush_mesh::{FaceId, VertexId};

use super::energy::{Rest, energy, max_step};
use super::free::Free;
use super::proxy::{Proxy, proxy};
use crate::solver::{Pattern, SolveError, analyse};

type Faces<'a> = &'a [(FaceId, [VertexId; 3])];

/// Fraction of the longest flip-free step a trial takes, so no face lands on collapse.
const STEP_FRACTION: f64 = 0.8;
/// Halvings of a step before the search gives up; 2⁻⁴⁰ of a step moves nothing.
const HALVINGS: u32 = 40;

/// The sparsity of the weighted problem, fixed for a chart and analysed once. Unknowns are the
/// free vertices' coordinates, vertex `s` at `2s` (x) and `2s + 1` (y).
pub(super) struct System {
    free: Vec<Option<usize>>,
    pattern: Pattern,
    /// Position of each lower-triangle entry `(row, col)` in the value list.
    entries: BTreeMap<(usize, usize), usize>,
    unknowns: usize,
}

/// A face's share of the normal equations: `area · P[k][l] · S[p][q]` couples coordinate `k`
/// of corner `p` to coordinate `l` of corner `q`, `pull` is the right-hand side.
struct Terms {
    area: f64,
    stiffness: [[f64; 3]; 3],
    weight: [[f64; 2]; 2],
    pull: [[f64; 3]; 2],
}

impl System {
    pub(super) fn new(faces: Faces<'_>, free: &Free) -> Result<Self, SolveError> {
        let mut lower = BTreeSet::new();
        for (_, t) in faces {
            for (p, q) in corner_pairs() {
                let (Some(sp), Some(sq)) = (free.of(t[p]), free.of(t[q])) else {
                    continue;
                };
                for (k, l) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
                    let (row, col) = (2 * sp + k, 2 * sq + l);
                    if row >= col {
                        lower.insert((row, col));
                    }
                }
            }
        }
        let list: Vec<(usize, usize)> = lower.into_iter().collect();
        let unknowns = 2 * free.count;
        Ok(Self {
            free: free.slot.clone(),
            pattern: analyse(unknowns, &list)?,
            entries: list.iter().copied().zip(0..).collect(),
            unknowns,
        })
    }

    /// `uv` with every free vertex moved to minimise the proxy at the current Jacobians.
    pub(super) fn target(
        &self,
        rest: &[Rest],
        faces: Faces<'_>,
        uv: &[DVec2],
    ) -> Result<Vec<DVec2>, SolveError> {
        let mut values = vec![0.0; self.entries.len()];
        let mut rhs = vec![0.0; self.unknowns];
        for (r, (_, t)) in rest.iter().zip(faces) {
            let terms = terms(r, &proxy(r.jacobian(uv, t)));
            self.add(&terms, t, uv, (&mut values, &mut rhs));
        }
        let x = self.pattern.factor(&values)?.solve(&rhs)?;
        let mut out = uv.to_vec();
        for (v, slot) in self.free.iter().enumerate() {
            if let Some(s) = *slot {
                out[v] = DVec2::new(x[2 * s], x[2 * s + 1]);
            }
        }
        Ok(out)
    }

    fn slot(&self, v: VertexId) -> Option<usize> {
        self.free.get(v.index() as usize).copied().flatten()
    }

    fn add(
        &self,
        terms: &Terms,
        t: &[VertexId; 3],
        uv: &[DVec2],
        (values, rhs): (&mut [f64], &mut [f64]),
    ) {
        for p in 0..3 {
            let Some(sp) = self.slot(t[p]) else { continue };
            for k in 0..2 {
                let row = 2 * sp + k;
                rhs[row] += terms.pull[k][p];
                for q in 0..3 {
                    for l in 0..2 {
                        let c = terms.area * terms.weight[k][l] * terms.stiffness[p][q];
                        match self.slot(t[q]) {
                            Some(sq) if row >= 2 * sq + l => {
                                // INVARIANT: `new` listed every such pair of free corners.
                                values[self.entries[&(row, 2 * sq + l)]] += c;
                            }
                            Some(_) => {}
                            None => {
                                let at = uv[t[q].index() as usize];
                                rhs[row] -= c * [at.x, at.y][l];
                            }
                        }
                    }
                }
            }
        }
    }
}

fn corner_pairs() -> impl Iterator<Item = (usize, usize)> {
    (0..3).flat_map(|p| (0..3).map(move |q| (p, q)))
}

fn terms(rest: &Rest, proxy: &Proxy) -> Terms {
    let c = rest.operator();
    let area = rest.area();
    let stiffness = [0, 1, 2].map(|p| [0, 1, 2].map(|q| c[p][0] * c[q][0] + c[p][1] * c[q][1]));
    // (R Cᵀ)[l][p] = Σ_m R[l][m] C[p][m], then weighted: Σ_l P[k][l] (R Cᵀ)[l][p].
    let rc = |l: usize, p: usize| proxy.rotation[l][0] * c[p][0] + proxy.rotation[l][1] * c[p][1];
    let pull = [0, 1].map(|k| {
        [0, 1, 2].map(|p| area * (proxy.weight[k][0] * rc(0, p) + proxy.weight[k][1] * rc(1, p)))
    });
    Terms {
        area,
        stiffness,
        weight: proxy.weight,
        pull,
    }
}

/// `uv` moved along `dir` by the longest tried step that lowers the energy below `current`
/// and flips no face, with that energy; `None` when no tried step does.
pub(super) fn descend(
    rest: &[Rest],
    faces: Faces<'_>,
    uv: &[DVec2],
    dir: &[DVec2],
    current: f64,
) -> Option<(Vec<DVec2>, f64)> {
    let mut step = (STEP_FRACTION * max_step(faces, uv, dir)).min(1.0);
    for _ in 0..HALVINGS {
        let trial: Vec<DVec2> = uv.iter().zip(dir).map(|(u, d)| *u + *d * step).collect();
        match energy(rest, faces, &trial) {
            Some(e) if e < current => return Some((trial, e)),
            _ => step *= 0.5,
        }
    }
    None
}

#[cfg(test)]
mod tests;
