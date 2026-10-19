//! The disk check: a chart flattens only if, cut open along its seams, it is a disk (genus 0, one
//! boundary loop). Each expectation is worked out from the surface's topology by hand.

#[allow(
    dead_code,
    reason = "each test file uses only some of the shared meshes"
)]
mod common;

use common::{Grid, Octahedron, ROW_1, Setup};
use openplush_mesh::ManifoldMesh;
use openplush_seams::{ChartFailure, SeamKind, SeamSet, charts, check_disk};

/// Every chart's verdict, in chart order.
fn verdicts(mesh: &ManifoldMesh, seams: &SeamSet) -> Result<Vec<Result<(), ChartFailure>>, Setup> {
    Ok(charts(mesh, seams)?
        .iter()
        .map(|chart| check_disk(mesh, seams, chart))
        .collect())
}

const DISK: Result<(), ChartFailure> = Ok(());

#[test]
fn a_sheet_is_a_disk_and_stays_one_when_cut_across() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 2)?;
    assert_eq!(verdicts(&grid.mesh, &SeamSet::new())?, [DISK]);
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Cut, &ROW_1)?;
    assert_eq!(verdicts(&grid.mesh, &seams)?, [DISK, DISK]);
    Ok(())
}

#[test]
fn a_dart_from_the_edge_extends_the_one_boundary() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 2)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Dart, &[(2, 0), (2, 1)])?;
    assert_eq!(verdicts(&grid.mesh, &seams)?, [DISK]);
    Ok(())
}

#[test]
fn a_slit_that_touches_no_boundary_opens_a_second_loop() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 2)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Cut, &[(1, 1), (2, 1), (3, 1)])?;
    let want = Err(ChartFailure::MultipleBoundaries { count: 2 });
    assert_eq!(verdicts(&grid.mesh, &seams)?, [want]);
    Ok(())
}

#[test]
fn an_open_tube_has_two_boundaries_until_it_is_cut_lengthwise() -> Result<(), Setup> {
    let grid = Grid::tube(4, 2)?;
    let want = Err(ChartFailure::MultipleBoundaries { count: 2 });
    assert_eq!(verdicts(&grid.mesh, &SeamSet::new())?, [want]);
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Cut, &[(0, 0), (0, 1), (0, 2)])?;
    assert_eq!(verdicts(&grid.mesh, &seams)?, [DISK]);
    Ok(())
}

#[test]
fn a_closed_octahedron_is_closed_until_one_seam_edge_slits_it() -> Result<(), Setup> {
    let solid = Octahedron::new()?;
    assert_eq!(
        verdicts(&solid.mesh, &SeamSet::new())?,
        [Err(ChartFailure::Closed)]
    );
    let mut seams = SeamSet::new();
    seams.insert(solid.edge(0, 4)?, SeamKind::Cut)?;
    assert_eq!(verdicts(&solid.mesh, &seams)?, [DISK]);
    // A second slit, apart from the first, is a second boundary.
    seams.insert(solid.edge(2, 5)?, SeamKind::Cut)?;
    let want = Err(ChartFailure::MultipleBoundaries { count: 2 });
    assert_eq!(verdicts(&solid.mesh, &seams)?, [want]);
    Ok(())
}

#[test]
fn an_equator_cuts_an_octahedron_into_two_disks() -> Result<(), Setup> {
    let solid = Octahedron::new()?;
    let mut seams = SeamSet::new();
    for i in 0..4 {
        seams.insert(solid.edge(i, (i + 1) % 4)?, SeamKind::Cut)?;
    }
    assert_eq!(verdicts(&solid.mesh, &seams)?, [DISK, DISK]);
    Ok(())
}

#[test]
fn a_torus_reports_its_handle_before_its_missing_boundary() -> Result<(), Setup> {
    let grid = Grid::torus(3, 3)?;
    let want = Err(ChartFailure::PositiveGenus { genus: 1 });
    assert_eq!(verdicts(&grid.mesh, &SeamSet::new())?, [want]);
    Ok(())
}

#[test]
fn one_ring_opens_a_torus_into_a_tube_and_a_second_cut_into_a_disk() -> Result<(), Setup> {
    let grid = Grid::torus(3, 3)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Cut, &[(0, 0), (0, 1), (0, 2), (0, 3)])?;
    let want = Err(ChartFailure::MultipleBoundaries { count: 2 });
    assert_eq!(verdicts(&grid.mesh, &seams)?, [want]);
    grid.seam(&mut seams, SeamKind::Cut, &[(0, 0), (1, 0), (2, 0), (3, 0)])?;
    assert_eq!(verdicts(&grid.mesh, &seams)?, [DISK]);
    Ok(())
}

#[test]
fn a_slit_torus_keeps_its_handle_and_a_handle_outranks_extra_boundaries() -> Result<(), Setup> {
    let grid = Grid::torus(3, 3)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Cut, &[(0, 0), (1, 1)])?;
    let want = Err(ChartFailure::PositiveGenus { genus: 1 });
    assert_eq!(verdicts(&grid.mesh, &seams)?, [want]);
    grid.seam(&mut seams, SeamKind::Cut, &[(2, 1), (2, 2)])?;
    assert_eq!(verdicts(&grid.mesh, &seams)?, [want]);
    Ok(())
}

#[test]
fn two_rings_cut_a_torus_into_two_tubes() -> Result<(), Setup> {
    let grid = Grid::torus(3, 3)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Cut, &[(0, 0), (0, 1), (0, 2), (0, 3)])?;
    grid.seam(&mut seams, SeamKind::Cut, &[(1, 0), (1, 1), (1, 2), (1, 3)])?;
    let want = Err(ChartFailure::MultipleBoundaries { count: 2 });
    assert_eq!(verdicts(&grid.mesh, &seams)?, [want, want]);
    Ok(())
}
