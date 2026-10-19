//! Closing a chart that is not a disk: one proposal of `Cut` edges that, accepted, leaves the
//! chart one disk. Each expectation is worked out from the surface by hand.

#[allow(
    dead_code,
    reason = "each test file uses only some of the shared meshes"
)]
mod common;

use common::{Grid, Octahedron, ROW_1, Setup};
use openplush_geom::DVec3;
use openplush_mesh::{EdgeId, ManifoldMesh};
use openplush_seams::{
    Chart, ChartFailure, Proposal, ProposalKind, SeamKind, SeamSet, charts, check_disk, close_chart,
};

/// The proposal for `chart`, checked: it names the chart and `check_disk`'s failure, adds only
/// `Cut`s on edges that are not seams, and, accepted, leaves the same faces one chart that is a
/// disk. Returns the edges it adds.
fn closed(mesh: &ManifoldMesh, seams: &SeamSet, chart: &Chart) -> Result<Vec<EdgeId>, Setup> {
    let failure = check_disk(mesh, seams, chart)
        .err()
        .ok_or("already a disk")?;
    let proposal: Proposal = close_chart(mesh, seams, chart)?.ok_or("no proposal")?;
    let want = ProposalKind::CloseChart {
        chart: chart.id(),
        failure,
    };
    assert_eq!(proposal.kind(), &want);
    assert!(!proposal.explanation().is_empty());
    let mut accepted = seams.clone();
    for (&edge, &kind) in proposal.add() {
        assert_eq!(kind, SeamKind::Cut);
        assert_eq!(seams.get(edge), None, "{edge:?} is already a seam");
        accepted.insert(edge, kind)?;
    }
    let after = charts(mesh, &accepted)?;
    let same = after
        .iter()
        .find(|c| c.faces() == chart.faces())
        .ok_or("the cuts split the chart")?;
    assert_eq!(check_disk(mesh, &accepted, same), Ok(()));
    Ok(proposal.add().keys().copied().collect())
}

/// The only chart of `seams` on `mesh`.
fn only(mesh: &ManifoldMesh, seams: &SeamSet) -> Result<Chart, Setup> {
    let mut all = charts(mesh, seams)?;
    assert_eq!(all.len(), 1);
    all.pop().ok_or_else(|| "no chart".into())
}

/// The explanation of `chart`'s proposal.
fn explanation(mesh: &ManifoldMesh, seams: &SeamSet, chart: &Chart) -> Result<String, Setup> {
    let proposal = close_chart(mesh, seams, chart)?.ok_or("no proposal")?;
    Ok(proposal.explanation().to_owned())
}

/// Equal to rounding: the grid's points come from sines and cosines.
fn assert_close(got: f64, want: f64) {
    assert!((got - want).abs() <= 1e-9 * want, "{got} is not {want}");
}

fn length(grid: &Grid, edges: &[EdgeId]) -> Result<f64, Setup> {
    let mesh = &grid.mesh;
    let mut total = 0.0;
    for &edge in edges {
        let h = (0..u32::try_from(mesh.half_edge_count())?)
            .map(openplush_mesh::HalfEdgeId::new)
            .find(|h| h.edge() == edge)
            .ok_or("no half-edge")?;
        total += (mesh.position(mesh.dest(h)) - mesh.position(mesh.origin(h))).length();
    }
    Ok(total)
}

#[test]
fn a_disk_needs_no_proposal() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 2)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Cut, &ROW_1)?;
    for chart in charts(&grid.mesh, &seams)? {
        assert_eq!(close_chart(&grid.mesh, &seams, &chart)?, None);
    }
    Ok(())
}

#[test]
fn a_closed_chart_is_cut_between_its_two_sharpest_points() -> Result<(), Setup> {
    // The tall top and the deep bottom have the most angle defect; equator point 0 is nearer
    // both than the others, so the one shortest path runs top, point 0, bottom.
    let solid = Octahedron::with_points([
        DVec3::new(8.0, 0.0, 0.0),
        DVec3::new(0.0, 10.0, 0.0),
        DVec3::new(-10.0, 0.0, 0.0),
        DVec3::new(0.0, -10.0, 0.0),
        DVec3::new(0.0, 0.0, 30.0),
        DVec3::new(0.0, 0.0, -20.0),
    ])?;
    let seams = SeamSet::new();
    let chart = only(&solid.mesh, &seams)?;
    let mut want = vec![solid.edge(0, 4)?, solid.edge(0, 5)?];
    want.sort();
    assert_eq!(closed(&solid.mesh, &seams, &chart)?, want);
    assert_eq!(
        explanation(&solid.mesh, &seams, &chart)?,
        "This part is closed all round, so it cannot lie flat as one piece. Cutting along 2 more \
         edges opens it into one piece that can."
    );
    Ok(())
}

#[test]
fn a_closed_chart_whose_points_all_tie_is_cut_between_its_two_lowest() -> Result<(), Setup> {
    // Every face is a copy of the others under axis swaps and sign flips, so every defect is
    // the same number; points 0 and 1 are the first two vertices of the lowest face.
    let solid = Octahedron::new()?;
    let seams = SeamSet::new();
    let chart = only(&solid.mesh, &seams)?;
    assert_eq!(closed(&solid.mesh, &seams, &chart)?, [solid.edge(0, 1)?]);
    Ok(())
}

#[test]
fn an_open_tube_is_cut_once_along_its_length() -> Result<(), Setup> {
    let grid = Grid::tube(4, 2)?;
    let seams = SeamSet::new();
    let chart = only(&grid.mesh, &seams)?;
    let cuts = closed(&grid.mesh, &seams, &chart)?;
    // Two straight rungs of 10 each; any diagonal is longer.
    assert_eq!(cuts.len(), 2);
    assert_eq!(length(&grid, &cuts)?, 20.0);
    assert_eq!(
        explanation(&grid.mesh, &seams, &chart)?,
        "This part has 2 separate open edges, so it cannot lie flat as one piece. Cutting along \
         2 more edges opens it into one piece that can."
    );
    Ok(())
}

#[test]
fn an_interior_slit_is_joined_to_the_edge_by_one_edge() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 2)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Cut, &[(1, 1), (2, 1), (3, 1)])?;
    let chart = only(&grid.mesh, &seams)?;
    let cuts = closed(&grid.mesh, &seams, &chart)?;
    assert_eq!(cuts.len(), 1);
    assert_eq!(length(&grid, &cuts)?, 10.0);
    assert!(explanation(&grid.mesh, &seams, &chart)?.contains("Cutting along one more edge "));
    Ok(())
}

#[test]
fn three_loops_are_joined_by_the_two_shortest_paths() -> Result<(), Setup> {
    // Two slits, each one rung from the sheet's edge and far from the other.
    let grid = Grid::sheet(6, 4)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Cut, &[(1, 1), (2, 1)])?;
    grid.seam(&mut seams, SeamKind::Cut, &[(4, 3), (5, 3)])?;
    let chart = only(&grid.mesh, &seams)?;
    let cuts = closed(&grid.mesh, &seams, &chart)?;
    assert_eq!(cuts.len(), 2);
    assert_eq!(length(&grid, &cuts)?, 20.0);
    Ok(())
}

#[test]
fn only_the_chart_that_is_not_a_disk_gets_a_proposal() -> Result<(), Setup> {
    // A 4 by 3 sheet cut across row 1, with a slit inside the upper chart.
    let grid = Grid::sheet(4, 3)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Cut, &ROW_1)?;
    grid.seam(&mut seams, SeamKind::Cut, &[(1, 2), (2, 2), (3, 2)])?;
    let all = charts(&grid.mesh, &seams)?;
    assert_eq!(all.len(), 2);
    let verdicts: Vec<bool> = all
        .iter()
        .map(|c| close_chart(&grid.mesh, &seams, c).map(|p| p.is_some()))
        .collect::<Result<_, _>>()?;
    let failing: Vec<bool> = all
        .iter()
        .map(|c| check_disk(&grid.mesh, &seams, c).is_err())
        .collect();
    assert_eq!(verdicts, failing);
    assert_eq!(failing.iter().filter(|&&f| f).count(), 1);
    for chart in all
        .iter()
        .filter(|c| check_disk(&grid.mesh, &seams, c).is_err())
    {
        closed(&grid.mesh, &seams, chart)?;
    }
    Ok(())
}

#[test]
fn a_torus_is_cut_along_two_loops() -> Result<(), Setup> {
    let grid = Grid::torus(6, 4)?;
    let seams = SeamSet::new();
    let chart = only(&grid.mesh, &seams)?;
    assert_eq!(
        check_disk(&grid.mesh, &seams, &chart),
        Err(ChartFailure::PositiveGenus { genus: 1 })
    );
    let cuts = closed(&grid.mesh, &seams, &chart)?;
    // Rows sit at tube angles 0, 90, 180 and 270 degrees, so ring radii 28, 20, 12 and 20; a
    // rung is 8√2 and a row-1-to-2 or row-2-to-3 diagonal √368. From the root, point (0, 0), the
    // shortest loop round the tube is column 0's four rungs. The shortest round the hole goes up
    // one rung, along a diagonal to the inner ring, four of its six 12-long sides, a diagonal
    // back to row 3 and a rung home: the two rungs are column 0's.
    assert_close(
        length(&grid, &cuts)?,
        4.0 * 8.0 * 2f64.sqrt() + 2.0 * 368f64.sqrt() + 4.0 * 12.0,
    );
    assert!(explanation(&grid.mesh, &seams, &chart)?.contains("wraps round a handle,"));
    Ok(())
}

#[test]
fn a_slit_torus_is_cut_from_its_slit() -> Result<(), Setup> {
    let grid = Grid::torus(6, 4)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Cut, &[(1, 1), (2, 1)])?;
    let chart = only(&grid.mesh, &seams)?;
    assert_eq!(
        check_disk(&grid.mesh, &seams, &chart),
        Err(ChartFailure::PositiveGenus { genus: 1 })
    );
    let cuts = closed(&grid.mesh, &seams, &chart)?;
    // The tree grows from the slit on row 1. Round the tube: column 1's four rungs. Round the
    // hole, from (2, 1): a diagonal down to the inner ring, four of its 12-long sides to (1, 2),
    // and column 1's rung back up, already cut.
    assert_close(
        length(&grid, &cuts)?,
        4.0 * 8.0 * 2f64.sqrt() + 368f64.sqrt() + 4.0 * 12.0,
    );
    Ok(())
}

#[test]
fn a_torus_with_two_slits_is_joined_then_cut_along_its_handle() -> Result<(), Setup> {
    let grid = Grid::torus(6, 4)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Cut, &[(1, 1), (2, 1)])?;
    grid.seam(&mut seams, SeamKind::Cut, &[(4, 3), (5, 3)])?;
    let chart = only(&grid.mesh, &seams)?;
    assert_eq!(
        check_disk(&grid.mesh, &seams, &chart),
        Err(ChartFailure::PositiveGenus { genus: 1 })
    );
    closed(&grid.mesh, &seams, &chart)?;
    Ok(())
}

#[test]
fn the_same_chart_gets_the_same_proposal() -> Result<(), Setup> {
    let grid = Grid::torus(6, 4)?;
    let seams = SeamSet::new();
    let chart = only(&grid.mesh, &seams)?;
    assert_eq!(
        close_chart(&grid.mesh, &seams, &chart)?,
        close_chart(&grid.mesh, &seams, &chart)?
    );
    Ok(())
}
