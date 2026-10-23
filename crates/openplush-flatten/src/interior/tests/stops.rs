//! The three ways a solve settles, each reached and told apart by what the solve shows: the
//! iteration count, and whether and how far the last iteration moved the energy.

use super::super::{MAX_ITERATIONS, SETTLED_BELOW, Solve, Step};
use super::{CHART, half_cylinder, hemisphere};
use crate::FlattenError;

/// The energy before and after the iteration that settled the solve.
fn settle(solve: &mut Solve) -> Result<(f64, f64), FlattenError> {
    loop {
        let before = solve.energy();
        if solve.iterate()? == Step::Settled {
            return Ok((before, solve.energy()));
        }
    }
}

#[test]
fn the_cap_stops_a_solve_that_is_still_gaining() -> Result<(), FlattenError> {
    let (xyz, f) = hemisphere();
    let mut solve = Solve::start(CHART, &xyz, f)?;
    solve.cap = 3;
    let (before, after) = settle(&mut solve)?;
    assert_eq!(solve.iterations(), 3);
    assert!(
        (before - after) / before >= SETTLED_BELOW,
        "{before} → {after}"
    );
    Ok(())
}

#[test]
fn a_negligible_gain_stops_a_solve_that_still_moves() -> Result<(), FlattenError> {
    let (xyz, f) = hemisphere();
    let mut solve = Solve::start(CHART, &xyz, f)?;
    let (before, after) = settle(&mut solve)?;
    assert!(solve.iterations() < MAX_ITERATIONS);
    assert_ne!(before.to_bits(), after.to_bits());
    let gain = (before - after) / before;
    assert!(gain > 0.0 && gain < SETTLED_BELOW, "{gain}");
    Ok(())
}

#[test]
fn no_lower_step_stops_a_solve_that_starts_at_the_isometry() -> Result<(), FlattenError> {
    // A developable chart's fill is already the isometry (energy 4), so the line search
    // finds no step below it and the energy keeps its bits.
    let (xyz, f) = half_cylinder();
    let mut solve = Solve::start(CHART, &xyz, f)?;
    let (before, after) = settle(&mut solve)?;
    assert_eq!(solve.iterations(), 1);
    assert_eq!(before.to_bits(), after.to_bits());
    Ok(())
}
