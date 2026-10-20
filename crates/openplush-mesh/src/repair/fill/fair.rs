//! Fairing: the patch's added vertices solve the uniform bilaplacian `K² x = 0` with the rim
//! and its outer ring fixed, so the patch meets the surface with matching slope. `K` is the
//! combinatorial graph Laplacian, so the system `K_{P,I}ᵀ K_{P,I}` over the added vertices
//! `I` is symmetric positive definite once the rim is fixed, and one Cholesky factor serves
//! all three coordinates.

use std::collections::BTreeMap;

use faer::linalg::solvers::Solve as _;
use faer::prelude::Reborrow as _;
use faer::sparse::linalg::solvers::{Llt, SymbolicLlt};
use faer::sparse::{Pair, SparseColMat, SymbolicSparseColMat};
use faer::{Mat, Side};
use openplush_geom::DVec3;

use super::refine::Patch;

/// Moves `patch`'s added vertices to the bilaplacian's solution. `outer[i]` is the positions
/// of rim vertex `i`'s mesh neighbours outside the patch. `None` when the solve fails or
/// leaves a coordinate non-finite.
pub(super) fn fair(patch: &mut Patch, outer: &[Vec<DVec3>]) -> Option<()> {
    let unknowns = patch.pos.len() - patch.rim;
    if unknowns == 0 {
        return Some(());
    }
    let rows = laplacian(patch, outer);
    let (lower, rhs) = system(patch, &rows);
    let x = solve(unknowns, &lower, &rhs)?;
    for (i, p) in x.into_iter().enumerate() {
        patch.pos[patch.rim + i] = p;
    }
    Some(())
}

/// A row of `K` for one patch vertex: the vertex's degree on itself and -1 on each neighbour,
/// a neighbour being a patch vertex by index or an outer-ring point by position.
struct Row {
    degree: f64,
    patch: Vec<usize>,
    outer: Vec<DVec3>,
}

fn laplacian(patch: &Patch, outer: &[Vec<DVec3>]) -> Vec<Row> {
    let mut neighbours: Vec<Vec<usize>> = vec![Vec::new(); patch.pos.len()];
    for &[a, b, c] in &patch.tris {
        for (u, v) in [(a, b), (b, c), (c, a)] {
            neighbours[u].push(v);
            neighbours[v].push(u);
        }
    }
    neighbours
        .into_iter()
        .enumerate()
        .map(|(v, mut near)| {
            near.sort();
            near.dedup();
            let outer = if v < patch.rim {
                outer[v].clone()
            } else {
                Vec::new()
            };
            Row {
                // Lossless: a vertex has far fewer than 2^53 neighbours.
                degree: (near.len() + outer.len()) as f64,
                patch: near,
                outer,
            }
        })
        .collect()
}

/// `K_{P,I}ᵀ K_{P,I}`'s lower triangle, by (row, column) over the unknowns, and the right-hand
/// side `-K_{P,I}ᵀ K_{P,F} x_F`.
type System = (BTreeMap<(usize, usize), f64>, Vec<DVec3>);

fn system(patch: &Patch, rows: &[Row]) -> System {
    let rim = patch.rim;
    let mut lower = BTreeMap::new();
    let mut rhs = vec![DVec3::ZERO; patch.pos.len() - rim];
    for (p, row) in rows.iter().enumerate() {
        // Row p of K, split into its unknown entries and the product of its fixed ones.
        let mut free: Vec<(usize, f64)> = Vec::new();
        let mut fixed = DVec3::ZERO;
        let mut add = |v: usize, k: f64| {
            if v >= rim {
                free.push((v - rim, k));
            } else {
                fixed += patch.pos[v] * k;
            }
        };
        add(p, row.degree);
        for &q in &row.patch {
            add(q, -1.0);
        }
        for &o in &row.outer {
            fixed -= o;
        }
        for &(i, ki) in &free {
            rhs[i] -= fixed * ki;
            for &(j, kj) in &free {
                if j <= i {
                    *lower.entry((i, j)).or_insert(0.0) += ki * kj;
                }
            }
        }
    }
    (lower, rhs)
}

fn solve(n: usize, lower: &BTreeMap<(usize, usize), f64>, rhs: &[DVec3]) -> Option<Vec<DVec3>> {
    let pairs: Vec<Pair<usize, usize>> = lower.keys().map(|&(r, c)| Pair::new(r, c)).collect();
    let values: Vec<f64> = lower.values().copied().collect();
    let (structure, order) = SymbolicSparseColMat::try_new_from_indices(n, n, &pairs).ok()?;
    let symbolic = SymbolicLlt::try_new(structure.rb(), Side::Lower).ok()?;
    let matrix = SparseColMat::new_from_argsort(structure, &order, &values).ok()?;
    let llt = Llt::try_new_with_symbolic(symbolic, matrix.rb(), Side::Lower).ok()?;
    let b = Mat::<f64>::from_fn(n, 3, |r, c| [rhs[r].x, rhs[r].y, rhs[r].z][c]);
    let x = llt.solve(&b);
    let out: Vec<DVec3> = (0..n)
        .map(|r| DVec3::new(x[(r, 0)], x[(r, 1)], x[(r, 2)]))
        .collect();
    out.iter().all(|p| p.is_finite()).then_some(out)
}

#[cfg(test)]
mod tests;
