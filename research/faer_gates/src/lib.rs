//! A sparse Cholesky solve of the kind SLIM's interior step needs (001 T080): a 2D grid
//! Laplacian plus a small diagonal shift, symmetric positive definite, solved with faer's
//! sparse LLT. The result is fingerprinted bit for bit so native and wasm32 can be compared.

use faer::sparse::{SparseColMat, Triplet};
use faer::{Mat, Side};

/// FNV-1a over the bit patterns of `values`, in order.
pub fn fingerprint(values: &[f64]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for v in values {
        for b in v.to_bits().to_le_bytes() {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    }
    h
}

/// Solves (L + 0.01 I) x = b on an `n` by `n` grid, with b fixed by index only.
pub fn solve(n: usize) -> Vec<f64> {
    let idx = |i: usize, j: usize| i * n + j;
    let mut t = Vec::new();
    for i in 0..n {
        for j in 0..n {
            let mut degree = 0.0;
            for (di, dj) in [(0isize, 1isize), (1, 0), (0, -1), (-1, 0)] {
                let (ni, nj) = (i as isize + di, j as isize + dj);
                if ni >= 0 && nj >= 0 && (ni as usize) < n && (nj as usize) < n {
                    t.push(Triplet::new(idx(i, j), idx(ni as usize, nj as usize), -1.0));
                    degree += 1.0;
                }
            }
            t.push(Triplet::new(idx(i, j), idx(i, j), degree + 0.01));
        }
    }
    let size = n * n;
    let a = SparseColMat::<usize, f64>::try_new_from_triplets(size, size, &t)
        .expect("spike: grid triplets are in range");
    let b = Mat::<f64>::from_fn(size, 1, |r, _| ((r % 7) as f64) - 3.0 + 0.125 * ((r % 3) as f64));
    let llt = a.sp_cholesky(Side::Lower).expect("spike: the shifted Laplacian is SPD");
    let x = faer::linalg::solvers::Solve::solve(&llt, &b);
    (0..size).map(|r| x[(r, 0)]).collect()
}

/// Largest |A x - b| over rows, recomputed from the stencil without faer.
pub fn residual(n: usize, x: &[f64]) -> f64 {
    let mut worst: f64 = 0.0;
    for i in 0..n {
        for j in 0..n {
            let r = i * n + j;
            let mut ax = 0.01 * x[r];
            for (di, dj) in [(0isize, 1isize), (1, 0), (0, -1), (-1, 0)] {
                let (ni, nj) = (i as isize + di, j as isize + dj);
                if ni >= 0 && nj >= 0 && (ni as usize) < n && (nj as usize) < n {
                    ax += x[r] - x[ni as usize * n + nj as usize];
                }
            }
            let b = ((r % 7) as f64) - 3.0 + 0.125 * ((r % 3) as f64);
            worst = worst.max((ax - b).abs());
        }
    }
    worst
}
