use super::{SolveError, analyse};

type Lower = Vec<(usize, usize, f64)>;

fn split(entries: &Lower) -> (Vec<(usize, usize)>, Vec<f64>) {
    (
        entries.iter().map(|&(r, c, _)| (r, c)).collect(),
        entries.iter().map(|&(_, _, v)| v).collect(),
    )
}

fn solve(n: usize, entries: &Lower, rhs: &[f64]) -> Result<Vec<f64>, SolveError> {
    let (at, values) = split(entries);
    analyse(n, &at)?.factor(&values)?.solve(rhs)
}

/// A shifted 2D grid Laplacian's lower triangle: symmetric positive definite.
fn grid(side: usize) -> Lower {
    let mut out = Vec::new();
    for i in 0..side {
        for j in 0..side {
            let r = i * side + j;
            let mut degree = 0.0;
            for (ni, nj) in [
                (i.wrapping_sub(1), j),
                (i + 1, j),
                (i, j.wrapping_sub(1)),
                (i, j + 1),
            ] {
                if ni < side && nj < side {
                    degree += 1.0;
                    let c = ni * side + nj;
                    if c < r {
                        out.push((r, c, -1.0));
                    }
                }
            }
            out.push((r, r, degree + 0.01));
        }
    }
    out
}

/// Largest |A x − b|, with A rebuilt from the lower triangle by hand, not by the solver.
fn residual(n: usize, entries: &Lower, x: &[f64], b: &[f64]) -> f64 {
    let mut ax = vec![0.0; n];
    for &(r, c, v) in entries {
        ax[r] += v * x[c];
        if r != c {
            ax[c] += v * x[r];
        }
    }
    ax.iter()
        .zip(b)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f64::max)
}

#[test]
fn a_two_by_two_system_is_solved_exactly() -> Result<(), SolveError> {
    // [[4, 1], [1, 3]] x = [1, 2] has x = (1/11, 7/11).
    let x = solve(2, &vec![(0, 0, 4.0), (1, 0, 1.0), (1, 1, 3.0)], &[1.0, 2.0])?;
    assert!((x[0] - 1.0 / 11.0).abs() < 1e-15 && (x[1] - 7.0 / 11.0).abs() < 1e-15);
    Ok(())
}

#[test]
fn a_grid_solves_to_a_small_residual() -> Result<(), SolveError> {
    let entries = grid(30);
    let b: Vec<f64> = (0..900).map(|r| ((r % 7) as f64) - 3.0).collect();
    let x = solve(900, &entries, &b)?;
    assert!(residual(900, &entries, &x, &b) < 1e-10);
    Ok(())
}

#[test]
fn one_analysis_serves_new_values() -> Result<(), SolveError> {
    let (at, values) = split(&grid(10));
    let pattern = analyse(100, &at)?;
    let b = vec![1.0; 100];
    for scale in [1.0, 2.5] {
        let scaled: Vec<f64> = values.iter().map(|v| v * scale).collect();
        let x = pattern.factor(&scaled)?.solve(&b)?;
        let entries: Lower = at
            .iter()
            .zip(&scaled)
            .map(|(&(r, c), &v)| (r, c, v))
            .collect();
        assert!(residual(100, &entries, &x, &b) < 1e-10, "scale {scale}");
    }
    Ok(())
}

#[test]
fn the_same_system_gives_the_same_bits() -> Result<(), SolveError> {
    let entries = grid(20);
    let b: Vec<f64> = (0..400).map(|r| (r as f64).sqrt()).collect();
    let bits = |x: Vec<f64>| x.into_iter().map(f64::to_bits).collect::<Vec<_>>();
    assert_eq!(
        bits(solve(400, &entries, &b)?),
        bits(solve(400, &entries, &b)?)
    );
    Ok(())
}

#[test]
fn entries_above_the_diagonal_outside_the_matrix_or_repeated_are_refused() {
    for (bad, at) in [
        ((0, 1), vec![(0, 0), (0, 1), (1, 1)]),
        ((2, 0), vec![(0, 0), (2, 0), (1, 1)]),
        ((1, 0), vec![(0, 0), (1, 0), (1, 0), (1, 1)]),
    ] {
        assert!(
            matches!(analyse(2, &at), Err(SolveError::BadEntry { row, col }) if (row, col) == bad),
            "{at:?}"
        );
    }
}

#[test]
fn an_empty_system_is_refused() {
    assert!(matches!(analyse(0, &[]), Err(SolveError::Empty)));
}

#[test]
fn values_and_right_hand_sides_must_match_the_system() -> Result<(), SolveError> {
    let pattern = analyse(2, &[(0, 0), (1, 1)])?;
    assert!(matches!(
        pattern.factor(&[1.0]),
        Err(SolveError::Length {
            expected: 2,
            got: 1
        })
    ));
    let factor = pattern.factor(&[1.0, 1.0])?;
    assert!(matches!(
        factor.solve(&[1.0, 2.0, 3.0]),
        Err(SolveError::Length {
            expected: 2,
            got: 3
        })
    ));
    Ok(())
}

#[test]
fn a_value_or_right_hand_side_that_is_not_finite_is_refused() -> Result<(), SolveError> {
    let pattern = analyse(2, &[(0, 0), (1, 1)])?;
    for bad in [f64::NAN, f64::INFINITY] {
        assert!(matches!(
            pattern.factor(&[1.0, bad]),
            Err(SolveError::NotFinite)
        ));
        let factor = pattern.factor(&[1.0, 1.0])?;
        assert!(matches!(
            factor.solve(&[bad, 0.0]),
            Err(SolveError::NotFinite)
        ));
    }
    Ok(())
}

#[test]
fn a_matrix_that_is_not_positive_definite_is_refused() -> Result<(), SolveError> {
    let pattern = analyse(2, &[(0, 0), (1, 0), (1, 1)])?;
    // [[1, 5], [5, 1]] has a negative eigenvalue.
    assert!(matches!(
        pattern.factor(&[1.0, 5.0, 1.0]),
        Err(SolveError::NotPositiveDefinite)
    ));
    Ok(())
}

#[test]
fn a_missing_diagonal_entry_is_not_positive_definite() -> Result<(), SolveError> {
    let pattern = analyse(2, &[(0, 0), (1, 0)])?;
    assert!(matches!(
        pattern.factor(&[1.0, 0.5]),
        Err(SolveError::NotPositiveDefinite)
    ));
    Ok(())
}

#[test]
fn a_solution_that_overflows_is_refused() {
    // 1e300 / 1e-300 is past f64's range, so the exact answer has no finite value.
    assert_eq!(
        solve(1, &vec![(0, 0, 1e-300)], &[1e300]),
        Err(SolveError::NotFinite)
    );
}
