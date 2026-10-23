use std::f64::consts::FRAC_PI_2;

use openplush_geom::{DVec2, DVec3};
use openplush_mesh::VertexId;
use openplush_seams::ChartId;

use super::{FlattenSession, Progress};
use crate::boundary::fixtures::{Faces, faces, grid, rings};
use crate::interior::fixtures::{cap, half_cylinder, place};
use crate::interior::{Solve, Step};
use crate::{FlattenError, StoppedBy};

const CHART: ChartId = ChartId::new(9);

fn hemisphere() -> (Vec<DVec3>, Faces) {
    rings(16, |r, a| {
        let theta = FRAC_PI_2 * r;
        DVec3::new(
            libm::sin(theta) * libm::cos(a),
            libm::sin(theta) * libm::sin(a),
            libm::cos(theta),
        )
    })
}

fn bits(layout: &[(VertexId, DVec2)]) -> Vec<(u32, u64, u64)> {
    layout
        .iter()
        .map(|(v, p)| (v.index(), p.x.to_bits(), p.y.to_bits()))
        .collect()
}

#[test]
fn stepping_to_done_gives_the_layout_the_solve_settles_on() -> Result<(), FlattenError> {
    let (xyz, f) = hemisphere();
    let mut solve = Solve::start(CHART, &xyz, f.clone())?;
    while solve.iterate()? == Step::Improved {}
    let mut session = FlattenSession::new(CHART, &xyz, f.clone())?;
    while session.step()? != Progress::Done {}
    assert_eq!(
        session.finish().map(|(l, _)| bits(&l)),
        Some(bits(&solve.layout()))
    );
    Ok(())
}

#[test]
fn each_working_step_counts_one_iteration_and_never_raises_the_energy() -> Result<(), FlattenError>
{
    let (xyz, f) = hemisphere();
    let mut session = FlattenSession::new(CHART, &xyz, f.clone())?;
    let (mut expected, mut last) = (1, f64::INFINITY);
    while let Progress::Working { iteration, energy } = session.step()? {
        assert_eq!(iteration, expected);
        assert!(energy <= last);
        (expected, last) = (expected + 1, energy);
        assert!(expected <= 200, "the solve must settle");
    }
    assert!(expected > 2, "the hemisphere needs several iterations");
    Ok(())
}

#[test]
fn nothing_is_handed_over_before_done() -> Result<(), FlattenError> {
    let (xyz, f) = hemisphere();
    let fresh = FlattenSession::new(CHART, &xyz, f.clone())?;
    assert_eq!(fresh.finish(), None);
    let mut midway = FlattenSession::new(CHART, &xyz, f.clone())?;
    assert!(matches!(midway.step()?, Progress::Working { .. }));
    assert_eq!(midway.finish(), None);
    Ok(())
}

#[test]
fn done_stays_done_and_the_layout_is_whole() -> Result<(), FlattenError> {
    let (xyz, f) = hemisphere();
    let mut session = FlattenSession::new(CHART, &xyz, f.clone())?;
    while session.step()? != Progress::Done {}
    assert_eq!(session.step()?, Progress::Done);
    assert_eq!(session.finish().map(|(l, _)| l.len()), Some(xyz.len()));
    Ok(())
}

#[test]
fn a_chart_with_nothing_to_move_is_done_on_the_first_step() -> Result<(), FlattenError> {
    let xyz = [
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(3.0, 0.0, 0.0),
        DVec3::new(3.0, 0.0, 4.0),
    ];
    let f = faces(&[[0, 1, 2]]);
    let mut session = FlattenSession::new(CHART, &xyz, f.clone())?;
    assert_eq!(session.step()?, Progress::Done);
    assert_eq!(session.finish().map(|(l, _)| l.len()), Some(3));
    Ok(())
}

#[test]
fn a_chart_that_cannot_start_is_refused_up_front() {
    let xyz: Vec<DVec3> = (0..=3)
        .flat_map(|r| (0..=3).map(move |c| DVec3::new(f64::from(c), f64::from(r), 0.0)))
        .collect();
    let annulus: Faces = grid(3, 3)
        .into_iter()
        .filter(|(f, _)| f.index() / 2 != 4)
        .collect();
    assert!(matches!(
        FlattenSession::new(CHART, &xyz, annulus),
        Err(FlattenError::NotADisk { chart }) if chart == CHART
    ));
}

#[test]
fn the_reported_energy_is_the_solves_own() -> Result<(), FlattenError> {
    let (xyz, f) = hemisphere();
    let mut reference = Solve::start(CHART, &xyz, f.clone())?;
    let mut session = FlattenSession::new(CHART, &xyz, f.clone())?;
    while let Progress::Working { energy, .. } = session.step()? {
        assert_eq!(reference.iterate()?, Step::Improved);
        assert_eq!(energy.to_bits(), reference.energy().to_bits());
    }
    Ok(())
}

/// Everything a step could change, as bits.
fn state(session: &FlattenSession) -> (u32, u64, Vec<(u32, u64, u64)>, bool) {
    (
        session.solve.iterations(),
        session.solve.energy().to_bits(),
        bits(&session.solve.layout()),
        session.done,
    )
}

#[test]
fn a_step_that_fails_leaves_the_session_as_it_was() -> Result<(), FlattenError> {
    let (xyz, f) = hemisphere();
    let mut solve = Solve::start(CHART, &xyz, f)?;
    // A free vertex at NaN makes every value it touches NaN, which the factorisation refuses.
    place(&mut solve, VertexId::new(0), DVec2::new(f64::NAN, f64::NAN));
    let mut session = FlattenSession { solve, done: false };
    let before = state(&session);
    assert_eq!(
        session.step(),
        Err(FlattenError::SolveFailed { chart: CHART })
    );
    assert_eq!(state(&session), before);
    // A failed step must not settle the solve, or the next step would hand out the layout.
    assert_eq!(
        session.step(),
        Err(FlattenError::SolveFailed { chart: CHART })
    );
    assert_eq!(session.finish(), None);
    Ok(())
}

/// Why a session stepped to `Done` stopped, as `finish` reports it.
fn stopped_by(mut session: FlattenSession) -> Result<Option<StoppedBy>, FlattenError> {
    while session.step()? != Progress::Done {}
    Ok(session.finish().map(|(_, stopped)| stopped))
}

#[test]
fn finish_reports_a_stop_at_the_cap() -> Result<(), FlattenError> {
    let (xyz, f) = hemisphere();
    let mut solve = Solve::start(CHART, &xyz, f)?;
    cap(&mut solve, 3);
    let session = FlattenSession { solve, done: false };
    assert_eq!(stopped_by(session)?, Some(StoppedBy::IterationCap));
    Ok(())
}

#[test]
fn finish_reports_a_negligible_gain_as_settled() -> Result<(), FlattenError> {
    let (xyz, f) = hemisphere();
    let session = FlattenSession::new(CHART, &xyz, f)?;
    assert_eq!(stopped_by(session)?, Some(StoppedBy::Settled));
    Ok(())
}

#[test]
fn finish_reports_a_chart_already_at_its_isometry_as_no_descent() -> Result<(), FlattenError> {
    let (xyz, f) = half_cylinder();
    let session = FlattenSession::new(CHART, &xyz, f)?;
    assert_eq!(stopped_by(session)?, Some(StoppedBy::NoDescent));
    Ok(())
}
