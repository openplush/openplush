use std::f64::consts::FRAC_PI_2;

use openplush_geom::DVec3;
use openplush_mesh::VertexId;
use openplush_seams::ChartId;

use super::{FlattenSession, Progress};
use crate::FlattenError;
use crate::boundary::fixtures::{Faces, faces, grid, rings};
use crate::interior::{Solve, Step};

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

fn bits(layout: &[(VertexId, openplush_geom::DVec2)]) -> Vec<(u32, u64, u64)> {
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
        session.finish().map(|l| bits(&l)),
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
    assert_eq!(session.finish().map(|l| l.len()), Some(xyz.len()));
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
    assert_eq!(session.finish().map(|l| l.len()), Some(3));
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
