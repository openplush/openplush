//! The two linear starts for the SLIM iteration: the interior harmonic with cotangent weights,
//! then with uniform ones (Tutte), the boundary held where `boundary::prescribe` put it.

use std::collections::BTreeMap;

use openplush_geom::DVec2;
use openplush_mesh::{FaceId, VertexId};
use openplush_seams::ChartId;

use super::free::Free;
use crate::FlattenError;
use crate::injectivity;
use crate::solver::{self, SolveError};

type Faces<'a> = &'a [(FaceId, [VertexId; 3])];

/// The first start that flips no face, or the refusal of the start with fewer flipped faces
/// (the first on a tie).
pub(super) fn fill(
    chart: ChartId,
    faces: Faces<'_>,
    free: &Free,
    cotangent: &BTreeMap<(u32, u32), f64>,
    uv: &[DVec2],
) -> Result<Vec<DVec2>, FlattenError> {
    let solve =
        |weights| harmonic(free, weights, uv).map_err(|_| FlattenError::SolveFailed { chart });
    let mut starts = vec![solve(cotangent)?];
    let first = verify(chart, faces, &starts[0]);
    if first.is_ok() {
        return Ok(starts.swap_remove(0));
    }
    let uniform: BTreeMap<(u32, u32), f64> = cotangent.keys().map(|&e| (e, 1.0)).collect();
    starts.push(solve(&uniform)?);
    let second = verify(chart, faces, &starts[1]);
    let pick = choose([first, second])?;
    Ok(starts.swap_remove(pick))
}

/// Which start to use: the first that passed, else the refusal of the one with fewer flips.
pub(super) fn choose(verdicts: [Result<(), FlattenError>; 2]) -> Result<usize, FlattenError> {
    let [first, second] = verdicts;
    match (first, second) {
        (Ok(()), _) => Ok(0),
        (Err(FlattenError::Flip { .. }), Ok(())) => Ok(1),
        (Err(FlattenError::Flip { chart, faces: a }), Err(FlattenError::Flip { faces: b, .. })) => {
            Err(FlattenError::Flip {
                chart,
                faces: if b.len() < a.len() { b } else { a },
            })
        }
        (Err(FlattenError::Flip { .. }), Err(other)) | (Err(other), _) => Err(other),
    }
}

fn verify(chart: ChartId, faces: Faces<'_>, uv: &[DVec2]) -> Result<(), FlattenError> {
    let flat: Vec<(FaceId, [DVec2; 3])> = faces
        .iter()
        .map(|&(f, t)| (f, t.map(|v| uv[v.index() as usize])))
        .collect();
    injectivity::verify(chart, &flat)
}

/// `uv` with each free vertex moved to the weighted average of its neighbours: the solution of
/// `L_II u = -L_IB u_B` for both coordinates, from one factorisation.
pub(super) fn harmonic(
    free: &Free,
    weights: &BTreeMap<(u32, u32), f64>,
    uv: &[DVec2],
) -> Result<Vec<DVec2>, SolveError> {
    if free.count == 0 {
        return Ok(uv.to_vec());
    }
    let mut diagonal = vec![0.0; free.count];
    let mut rhs = [vec![0.0; free.count], vec![0.0; free.count]];
    let mut lower = Vec::new();
    let mut off = Vec::new();
    for (&(a, b), &w) in weights {
        let (sa, sb) = (free.of(VertexId::new(a)), free.of(VertexId::new(b)));
        for (this, other, at) in [(sa, sb, b), (sb, sa, a)] {
            let Some(i) = this else { continue };
            diagonal[i] += w;
            if other.is_none() {
                // INVARIANT: weights are keyed by vertices of faces, which have positions.
                let p = uv[at as usize];
                rhs[0][i] += w * p.x;
                rhs[1][i] += w * p.y;
            }
        }
        if let (Some(i), Some(j)) = (sa, sb) {
            lower.push((i.max(j), i.min(j)));
            off.push(-w);
        }
    }
    let entries: Vec<(usize, usize)> = (0..free.count).map(|i| (i, i)).chain(lower).collect();
    let values: Vec<f64> = diagonal.into_iter().chain(off).collect();
    let factor = solver::analyse(free.count, &entries)?.factor(&values)?;
    let (x, y) = (factor.solve(&rhs[0])?, factor.solve(&rhs[1])?);
    let mut out = uv.to_vec();
    for (v, slot) in free.slot.iter().enumerate() {
        if let Some(i) = *slot {
            out[v] = DVec2::new(x[i], y[i]);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests;
