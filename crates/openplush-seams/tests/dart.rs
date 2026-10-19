//! Darts: a seam drawn from a seam or the mesh boundary to a tip inside a piece. The tip stays one
//! vertex, so the dart slits the piece rather than splitting it; a walk that would split or
//! cross is refused and nothing changes.

#[allow(
    dead_code,
    reason = "each test file uses only some of the shared meshes"
)]
mod common;

use common::{Grid, ROW_1, Setup};
use openplush_mesh::EdgeId;
use openplush_seams::{
    SeamGraph, SeamKind, SeamNode, SeamSet, SeamsError, charts, check_disk, mark_dart,
};

/// The edges of the walk through `points`.
fn walk(grid: &Grid, points: &[(u32, u32)]) -> Result<Vec<EdgeId>, Setup> {
    points
        .windows(2)
        .map(|p| grid.edge(p[0], p[1]))
        .collect()
}

/// A dart's edges are `Dart`, its tip is a `DartTip` node, and the piece is still one disk.
fn is_a_slit(grid: &Grid, seams: &SeamSet, edges: &[EdgeId], tip: (u32, u32)) -> Result<(), Setup> {
    for &e in edges {
        assert_eq!(seams.get(e), Some(SeamKind::Dart), "{e:?}");
    }
    let graph = SeamGraph::new(&grid.mesh, seams)?;
    let tip = grid.at(tip.0, tip.1)?;
    assert!(
        graph.nodes().any(|(v, node)| v == tip && node == SeamNode::DartTip),
        "{tip:?} is the dart tip"
    );
    let charts = charts(&grid.mesh, seams)?;
    assert_eq!(charts.len(), 1, "a slit, not a split");
    assert_eq!(check_disk(&grid.mesh, seams, &charts[0]), Ok(()));
    Ok(())
}

#[test]
fn a_dart_from_the_boundary_slits_the_sheet() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 3)?;
    let mut seams = SeamSet::new();
    let edges = walk(&grid, &[(2, 0), (2, 1), (2, 2)])?;
    mark_dart(&grid.mesh, &mut seams, &edges)?;
    is_a_slit(&grid, &seams, &edges, (2, 2))
}

#[test]
fn a_dart_drawn_tip_first_is_the_same_dart() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 3)?;
    let mut seams = SeamSet::new();
    let edges = walk(&grid, &[(2, 2), (2, 1), (2, 0)])?;
    mark_dart(&grid.mesh, &mut seams, &edges)?;
    is_a_slit(&grid, &seams, &edges, (2, 2))
}

#[test]
fn a_dart_from_an_existing_seam_slits_one_side_of_it() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 3)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Cut, &ROW_1)?;
    let edges = walk(&grid, &[(2, 1), (2, 2)])?;
    mark_dart(&grid.mesh, &mut seams, &edges)?;
    for &e in &edges {
        assert_eq!(seams.get(e), Some(SeamKind::Dart));
    }
    let graph = SeamGraph::new(&grid.mesh, &seams)?;
    let tip = grid.at(2, 2)?;
    assert!(graph.nodes().any(|(v, n)| v == tip && n == SeamNode::DartTip));
    // The row still splits the sheet in two; the dart slits the upper piece.
    let charts = charts(&grid.mesh, &seams)?;
    assert_eq!(charts.len(), 2);
    for chart in &charts {
        assert_eq!(check_disk(&grid.mesh, &seams, chart), Ok(()));
    }
    Ok(())
}

#[test]
fn a_walk_touching_no_seam_or_boundary_is_not_anchored() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 3)?;
    let mut seams = SeamSet::new();
    let edges = walk(&grid, &[(1, 1), (2, 1)])?;
    assert_eq!(
        mark_dart(&grid.mesh, &mut seams, &edges),
        Err(SeamsError::DartNotAnchored {
            vertex: grid.at(1, 1)?
        })
    );
    assert!(seams.is_empty(), "nothing changes");
    Ok(())
}

#[test]
fn a_walk_from_boundary_to_boundary_would_split_so_it_is_refused() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 2)?;
    let mut seams = SeamSet::new();
    let edges = walk(&grid, &[(2, 0), (2, 1), (2, 2)])?;
    assert_eq!(
        mark_dart(&grid.mesh, &mut seams, &edges),
        Err(SeamsError::DartTouchesSeam {
            vertex: grid.at(2, 2)?
        })
    );
    assert!(seams.is_empty());
    Ok(())
}

#[test]
fn a_walk_across_an_existing_seam_is_refused_at_the_crossing() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 3)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Cut, &ROW_1)?;
    let before = seams.clone();
    let edges = walk(&grid, &[(2, 0), (2, 1), (2, 2)])?;
    assert_eq!(
        mark_dart(&grid.mesh, &mut seams, &edges),
        Err(SeamsError::DartTouchesSeam {
            vertex: grid.at(2, 1)?
        })
    );
    assert_eq!(seams, before);
    Ok(())
}

#[test]
fn a_single_edge_takes_its_anchored_end_as_the_base() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 3)?;
    let mut seams = SeamSet::new();
    let edges = walk(&grid, &[(2, 1), (2, 0)])?;
    mark_dart(&grid.mesh, &mut seams, &edges)?;
    is_a_slit(&grid, &seams, &edges, (2, 1))
}

#[test]
fn edges_that_do_not_join_or_that_loop_are_not_a_walk() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 3)?;
    let mut seams = SeamSet::new();
    assert_eq!(
        mark_dart(&grid.mesh, &mut seams, &[]),
        Err(SeamsError::EmptyDart)
    );
    let apart = [grid.edge((0, 0), (1, 0))?, grid.edge((2, 0), (3, 0))?];
    assert_eq!(
        mark_dart(&grid.mesh, &mut seams, &apart),
        Err(SeamsError::NotAWalk { edge: apart[1] })
    );
    let looped = walk(&grid, &[(0, 0), (1, 0), (1, 1), (0, 0)])?;
    assert_eq!(
        mark_dart(&grid.mesh, &mut seams, &looped),
        Err(SeamsError::NotAWalk { edge: looped[2] })
    );
    assert!(seams.is_empty());
    Ok(())
}

#[test]
fn an_edge_the_mesh_does_not_have_is_refused() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 3)?;
    let mut seams = SeamSet::new();
    let beyond = EdgeId::new(u32::try_from(grid.mesh.edge_count())?);
    assert_eq!(
        mark_dart(&grid.mesh, &mut seams, &[beyond]),
        Err(SeamsError::UnknownEdge { edge: beyond })
    );
    Ok(())
}
