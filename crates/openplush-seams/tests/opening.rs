//! Marking a seam chain as the stuffing opening: every edge of it becomes an `Opening`, or, if
//! any cannot, none does.

#[allow(
    dead_code,
    reason = "each test file uses only some of the shared meshes"
)]
mod common;

use common::{Grid, ROW_1, Setup};
use openplush_seams::{SeamGraph, SeamKind, SeamSet, SeamsError, mark_opening};

#[test]
fn a_cut_chain_becomes_an_opening_and_nothing_else_changes() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 2)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Cut, &ROW_1)?;
    grid.seam(&mut seams, SeamKind::Dart, &[(2, 0), (2, 1)])?;
    let graph = SeamGraph::new(&grid.mesh, &seams)?;
    let chain = graph
        .chains()
        .iter()
        .find(|c| c.kind() == SeamKind::Cut)
        .ok_or("no cut chain")?;
    let before = seams.clone();
    mark_opening(&mut seams, chain)?;
    for (edge, kind) in before.iter() {
        let want = if chain.edges().contains(&edge) {
            SeamKind::Opening
        } else {
            kind
        };
        assert_eq!(seams.get(edge), Some(want), "{edge:?}");
    }
    assert_eq!(seams.len(), before.len());
    Ok(())
}

#[test]
fn marking_an_opening_again_changes_nothing() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 2)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Opening, &ROW_1)?;
    let graph = SeamGraph::new(&grid.mesh, &seams)?;
    let before = seams.clone();
    mark_opening(&mut seams, &graph.chains()[0])?;
    assert_eq!(seams, before);
    Ok(())
}

#[test]
fn a_mandatory_chain_is_refused_and_the_set_kept() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 2)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Mandatory, &ROW_1)?;
    let graph = SeamGraph::new(&grid.mesh, &seams)?;
    let chain = &graph.chains()[0];
    let before = seams.clone();
    assert_eq!(
        mark_opening(&mut seams, chain),
        Err(SeamsError::MandatorySeam {
            edge: chain.edges()[0]
        })
    );
    assert_eq!(seams, before);
    Ok(())
}

#[test]
fn a_chain_with_an_edge_that_is_no_seam_here_is_refused_and_the_set_kept() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 2)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Cut, &ROW_1)?;
    let graph = SeamGraph::new(&grid.mesh, &seams)?;
    let chain = &graph.chains()[0];
    // The set has moved on since the graph was built: the chain's last edge is no seam now.
    let gone = *chain.edges().last().ok_or("an empty chain")?;
    seams.remove(gone)?;
    let before = seams.clone();
    assert_eq!(
        mark_opening(&mut seams, chain),
        Err(SeamsError::NotASeam { edge: gone })
    );
    assert_eq!(seams, before);
    Ok(())
}
