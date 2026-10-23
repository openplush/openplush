//! The three ways a solve settles, each reached and told apart by what the solve shows: the
//! iteration count, and whether and how far the last iteration moved the energy; and each
//! reported as the reason it gives.

use openplush_geom::DVec3;

use super::super::{MAX_ITERATIONS, SETTLED_BELOW, Solve, Step};
use super::{CHART, faces, half_cylinder, hemisphere};
use crate::{FlattenError, StoppedBy};

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
    assert_eq!(solve.stopped_by(), Some(StoppedBy::IterationCap));
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
    assert_eq!(solve.stopped_by(), Some(StoppedBy::Settled));
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
    assert_eq!(solve.stopped_by(), Some(StoppedBy::NoDescent));
    Ok(())
}

#[test]
fn a_solve_still_iterating_has_no_reason_to_stop() -> Result<(), FlattenError> {
    let (xyz, f) = hemisphere();
    let mut solve = Solve::start(CHART, &xyz, f)?;
    assert_eq!(solve.stopped_by(), None);
    assert_eq!(solve.iterate()?, Step::Improved);
    assert_eq!(solve.stopped_by(), None);
    Ok(())
}

#[test]
fn a_chart_with_nothing_to_move_stops_on_no_descent() -> Result<(), FlattenError> {
    let (xyz, f) = (
        [DVec3::ZERO, DVec3::X * 3.0, DVec3::new(3.0, 0.0, 4.0)],
        faces(&[[0, 1, 2]]),
    );
    let mut solve = Solve::start(CHART, &xyz, f)?;
    assert_eq!(solve.iterate()?, Step::Settled);
    assert_eq!(solve.iterations(), 0);
    assert_eq!(solve.stopped_by(), Some(StoppedBy::NoDescent));
    Ok(())
}

#[test]
fn the_cap_and_a_negligible_gain_on_one_step_count_as_the_cap() -> Result<(), FlattenError> {
    let (xyz, f) = hemisphere();
    let mut free = Solve::start(CHART, &xyz, f.clone())?;
    settle(&mut free)?;
    let mut tied = Solve::start(CHART, &xyz, f)?;
    tied.cap = free.iterations();
    let (before, after) = settle(&mut tied)?;
    assert_eq!(tied.iterations(), free.iterations());
    assert!(
        (before - after) / before < SETTLED_BELOW,
        "{before} → {after}"
    );
    assert_eq!(tied.stopped_by(), Some(StoppedBy::IterationCap));
    Ok(())
}
