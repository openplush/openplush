//! The session's proposals: what the current state offers, previewed as a seam set and applied
//! only while it is still offered.

#[allow(
    dead_code,
    reason = "each test file uses only some of the shared helpers"
)]
#[path = "common/grid.rs"]
mod grid;

use grid::{Setup, bumpy, column_cut, mesh_of, tetrahedron_then_grid};
use openplush_pipeline::{EngineError, Pipeline, ProposalId, SeamSet, chart};

/// A closed tetrahedron beside an open grid, with no seams: the chart stage refuses the
/// tetrahedron as not a disk and carries the cut that opens it.
fn closed() -> Result<(Pipeline, Vec<openplush_pipeline::Proposal>), Setup> {
    let mesh = tetrahedron_then_grid()?;
    let Err(refusal) = chart(&mesh, &SeamSet::new()) else {
        return Err("a closed tetrahedron is not a disk".into());
    };
    let offered = refusal.proposals().to_vec();
    if offered.is_empty() {
        return Err("the chart stage carries a cut".into());
    }
    Ok((Pipeline::new(mesh), offered))
}

#[test]
fn the_chart_stages_cuts_are_offered() -> Result<(), Setup> {
    let (pipeline, offered) = closed()?;
    assert_eq!(pipeline.proposals(), offered);
    Ok(())
}

#[test]
fn a_session_whose_charts_flatten_offers_nothing() -> Result<(), Setup> {
    let mesh = mesh_of(&bumpy(6, 2, 3.0))?;
    let mut seams = SeamSet::new();
    column_cut(&mesh, &mut seams, 3, 2)?;
    let mut pipeline = Pipeline::new(mesh);
    pipeline.set_seams(&seams);
    pipeline.parameterizations()?;
    assert_eq!(pipeline.proposals(), vec![]);
    Ok(())
}

#[test]
fn a_preview_is_the_seams_with_the_proposals_edges_and_changes_nothing() -> Result<(), Setup> {
    let (pipeline, offered) = closed()?;
    let before = pipeline.charts().clone();
    let preview = pipeline.preview(offered[0].id())?;
    let expected: Vec<_> = offered[0].add().iter().map(|(&e, &k)| (e, k)).collect();
    assert_eq!(preview.iter().collect::<Vec<_>>(), expected);
    assert_eq!(pipeline.charts(), &before);
    assert_eq!(pipeline.proposals(), offered);
    Ok(())
}

#[test]
fn applying_a_proposal_sets_its_preview_as_the_seams() -> Result<(), Setup> {
    let (mut pipeline, offered) = closed()?;
    let id = offered[0].id();
    let preview = pipeline.preview(id)?;
    let Err(_) = pipeline.charts() else {
        return Err("refused before".into());
    };
    let mut other = Pipeline::new(tetrahedron_then_grid()?);
    let named = other.set_seams(&preview);
    assert_eq!(pipeline.apply_proposal(id)?, named);
    assert_eq!(pipeline.charts(), other.charts());
    assert!(pipeline.charts().is_ok(), "the cut opens the tetrahedron");
    Ok(())
}

#[test]
fn a_proposal_not_offered_is_refused_and_changes_nothing() -> Result<(), Setup> {
    let (mut pipeline, offered) = closed()?;
    let stranger = ProposalId::from_raw(offered[0].id().raw() ^ 1);
    let refused = Err(EngineError::ProposalNotOffered { id: stranger });
    let before = pipeline.charts().clone();
    assert_eq!(pipeline.preview(stranger), refused);
    assert_eq!(
        pipeline.apply_proposal(stranger),
        refused.map(|_: SeamSet| vec![])
    );
    assert_eq!(pipeline.charts(), &before);
    assert_eq!(pipeline.proposals(), offered);
    Ok(())
}

#[test]
fn an_applied_proposal_is_no_longer_offered() -> Result<(), Setup> {
    let (mut pipeline, offered) = closed()?;
    let id = offered[0].id();
    pipeline.apply_proposal(id)?;
    let after = pipeline.charts().clone();
    assert_eq!(
        pipeline.apply_proposal(id),
        Err(EngineError::ProposalNotOffered { id })
    );
    assert_eq!(pipeline.charts(), &after);
    Ok(())
}
