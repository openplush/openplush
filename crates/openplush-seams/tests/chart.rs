//! Charts: the faces reachable from each other without crossing a seam, numbered by their lowest
//! face.

#[allow(
    dead_code,
    reason = "each test file uses only some of the shared meshes"
)]
mod common;

use common::{Grid, ROW_1, Setup};
use openplush_mesh::{EdgeId, FaceId, HalfEdgeId, ManifoldMesh};
use openplush_seams::{Chart, SeamKind, SeamSet, SeamsError, charts};

fn sides(mesh: &ManifoldMesh, f: FaceId) -> [HalfEdgeId; 3] {
    let h = mesh.face_half_edge(f);
    [h, mesh.next(h), mesh.next(mesh.next(h))]
}

/// Every face in exactly one chart, listed in ascending order; charts in ID order, each
/// numbered by its lowest face; faces across an edge with no seam in the same chart; and each
/// chart connected without crossing a seam.
fn well_formed(mesh: &ManifoldMesh, seams: &SeamSet, charts: &[Chart]) {
    let n = mesh.face_count();
    let mut owner = vec![usize::MAX; n];
    for (i, chart) in charts.iter().enumerate() {
        let faces = chart.faces();
        assert!(faces.is_sorted() && !faces.is_empty(), "{chart:?}");
        assert_eq!(chart.id().index(), faces[0].index(), "{chart:?}");
        for f in faces {
            let slot = &mut owner[f.index() as usize];
            assert_eq!(*slot, usize::MAX, "{f:?} is in two charts");
            *slot = i;
        }
    }
    assert!(
        owner.iter().all(|&o| o != usize::MAX),
        "every face is in a chart"
    );
    let ids: Vec<_> = charts.iter().map(Chart::id).collect();
    assert!(ids.is_sorted(), "{ids:?}");
    let crossable = |h: HalfEdgeId| seams.get(h.edge()).is_none() && mesh.face(h.twin()).is_some();
    for chart in charts {
        let mut reached = vec![chart.faces()[0]];
        let mut i = 0;
        while let Some(&f) = reached.get(i) {
            for h in sides(mesh, f) {
                let Some(g) = mesh.face(h.twin()) else {
                    continue;
                };
                if crossable(h) {
                    assert_eq!(owner[g.index() as usize], owner[f.index() as usize]);
                    if !reached.contains(&g) {
                        reached.push(g);
                    }
                }
            }
            i += 1;
        }
        reached.sort();
        assert_eq!(reached, chart.faces(), "{chart:?} is one connected region");
    }
}

fn charted(grid: &Grid, seams: &SeamSet) -> Result<Vec<Chart>, Setup> {
    let found = charts(&grid.mesh, seams)?;
    well_formed(&grid.mesh, seams, &found);
    Ok(found)
}

fn sizes(charts: &[Chart]) -> Vec<usize> {
    charts.iter().map(|c| c.faces().len()).collect()
}

#[test]
fn a_sheet_with_no_seams_is_one_chart_numbered_by_its_lowest_face() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 2)?;
    let found = charted(&grid, &SeamSet::new())?;
    assert_eq!(sizes(&found), [16]);
    assert_eq!(found[0].id().index(), 0);
    Ok(())
}

#[test]
fn a_seam_across_a_sheet_makes_two_charts() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 2)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Cut, &ROW_1)?;
    assert_eq!(sizes(&charted(&grid, &seams)?), [8, 8]);
    Ok(())
}

#[test]
fn two_crossing_seams_make_four_charts() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 2)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Cut, &ROW_1)?;
    grid.seam(&mut seams, SeamKind::Cut, &[(2, 0), (2, 1), (2, 2)])?;
    assert_eq!(sizes(&charted(&grid, &seams)?), [4, 4, 4, 4]);
    Ok(())
}

#[test]
fn a_dart_slits_a_chart_without_splitting_it() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 2)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Dart, &[(2, 0), (2, 1)])?;
    assert_eq!(sizes(&charted(&grid, &seams)?), [16]);
    Ok(())
}

#[test]
fn a_seam_that_stops_short_of_crossing_splits_nothing() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 2)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Cut, &ROW_1[..4])?;
    assert_eq!(sizes(&charted(&grid, &seams)?), [16]);
    Ok(())
}

#[test]
fn a_seam_on_the_boundary_splits_nothing() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 2)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Mandatory, &[(0, 0), (1, 0), (2, 0)])?;
    assert_eq!(sizes(&charted(&grid, &seams)?), [16]);
    Ok(())
}

#[test]
fn a_ring_round_a_tube_makes_two_charts_and_no_ring_makes_one() -> Result<(), Setup> {
    let grid = Grid::tube(4, 2)?;
    assert_eq!(sizes(&charted(&grid, &SeamSet::new())?), [16]);
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Cut, &ROW_1)?;
    assert_eq!(sizes(&charted(&grid, &seams)?), [8, 8]);
    Ok(())
}

#[test]
fn a_seam_on_an_edge_the_mesh_does_not_have_is_refused() -> Result<(), Setup> {
    let grid = Grid::sheet(2, 1)?;
    let edge = EdgeId::new(u32::try_from(grid.mesh.edge_count())?);
    let mut seams = SeamSet::new();
    seams.insert(edge, SeamKind::Cut)?;
    assert_eq!(
        charts(&grid.mesh, &seams).err(),
        Some(SeamsError::UnknownEdge { edge })
    );
    Ok(())
}
