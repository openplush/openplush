//! Sparse symmetric positive-definite solves: the structure is analysed once, then each new set
//! of values is factored and solved without redoing that analysis.

use faer::linalg::solvers::Solve as _;
use faer::prelude::Reborrow as _;
use faer::sparse::linalg::LltError;
use faer::sparse::linalg::solvers::{Llt, SymbolicLlt};
use faer::sparse::{Argsort, Pair, SparseColMat, SymbolicSparseColMat};
use faer::{Mat, Side};

/// Why a solve was refused. The caller maps it to a `FlattenError` naming its chart.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SolveError {
    /// A system with no unknowns.
    Empty,
    /// An entry above the diagonal, outside the matrix, or given twice.
    BadEntry { row: usize, col: usize },
    /// A value or right-hand side list of the wrong length.
    Length { expected: usize, got: usize },
    /// A NaN or infinite value or right-hand side.
    NotFinite,
    /// A matrix with a non-positive pivot: not symmetric positive definite.
    NotPositiveDefinite,
    /// The solver could not allocate or index the system.
    Resources,
}

/// The analysed structure of an `n` by `n` symmetric matrix given by its lower triangle.
pub(crate) struct Pattern {
    n: usize,
    entries: usize,
    structure: SymbolicSparseColMat<usize>,
    order: Argsort<usize>,
    symbolic: SymbolicLlt<usize>,
}

/// Analyses the lower-triangle entries `(row, col)`, `row >= col`, each given once.
pub(crate) fn analyse(n: usize, lower: &[(usize, usize)]) -> Result<Pattern, SolveError> {
    if n == 0 {
        return Err(SolveError::Empty);
    }
    let mut seen = std::collections::BTreeSet::new();
    for &(row, col) in lower {
        if row >= n || col > row || !seen.insert((row, col)) {
            return Err(SolveError::BadEntry { row, col });
        }
    }
    let pairs: Vec<Pair<usize, usize>> = lower.iter().map(|&(r, c)| Pair::new(r, c)).collect();
    let (structure, order) = SymbolicSparseColMat::try_new_from_indices(n, n, &pairs)
        .map_err(|_| SolveError::Resources)?;
    let symbolic =
        SymbolicLlt::try_new(structure.rb(), Side::Lower).map_err(|_| SolveError::Resources)?;
    Ok(Pattern {
        n,
        entries: lower.len(),
        structure,
        order,
        symbolic,
    })
}

impl Pattern {
    /// Factors the matrix whose lower-triangle values follow `analyse`'s entry order.
    pub(crate) fn factor(&self, values: &[f64]) -> Result<Factor, SolveError> {
        if values.len() != self.entries {
            return Err(SolveError::Length {
                expected: self.entries,
                got: values.len(),
            });
        }
        if !values.iter().all(|v| v.is_finite()) {
            return Err(SolveError::NotFinite);
        }
        let matrix = SparseColMat::new_from_argsort(self.structure.clone(), &self.order, values)
            .map_err(|_| SolveError::Resources)?;
        let llt = Llt::try_new_with_symbolic(self.symbolic.clone(), matrix.rb(), Side::Lower)
            .map_err(|e| match e {
                LltError::Numeric(_) => SolveError::NotPositiveDefinite,
                LltError::Generic(_) => SolveError::Resources,
            })?;
        Ok(Factor { n: self.n, llt })
    }
}

/// A factored matrix, ready to solve any number of right-hand sides.
pub(crate) struct Factor {
    n: usize,
    llt: Llt<usize, f64>,
}

impl Factor {
    /// Solves `A x = rhs`.
    pub(crate) fn solve(&self, rhs: &[f64]) -> Result<Vec<f64>, SolveError> {
        if rhs.len() != self.n {
            return Err(SolveError::Length {
                expected: self.n,
                got: rhs.len(),
            });
        }
        if !rhs.iter().all(|v| v.is_finite()) {
            return Err(SolveError::NotFinite);
        }
        let b = Mat::<f64>::from_fn(self.n, 1, |r, _| rhs[r]);
        let x = self.llt.solve(&b);
        Ok((0..self.n).map(|r| x[(r, 0)]).collect())
    }
}

#[cfg(test)]
mod tests;
