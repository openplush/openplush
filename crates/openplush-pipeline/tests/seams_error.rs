//! A shell names the error a proposal-ID parse returns through this crate alone.

use openplush_pipeline::{ChartId, EdgeId, ProposalId, SeamChainId, SeamsError, VertexId};

#[test]
fn a_bad_proposal_id_is_a_pipeline_nameable_error() {
    let err: SeamsError = ProposalId::from_hex("XYZ").expect_err("not 16 lowercase hex digits");
    assert_eq!(
        err,
        SeamsError::BadProposalId {
            text: "XYZ".to_owned()
        }
    );
}

#[test]
fn seams_error_messages_are_pinned() {
    let bad = ProposalId::from_hex("XYZ").expect_err("not 16 lowercase hex digits");
    assert_eq!(
        bad.to_string(),
        r#""XYZ" is not a proposal id (16 lowercase hex digits)"#
    );
    assert_eq!(
        SeamsError::EmptyProposal.to_string(),
        "a proposal must add at least one seam edge"
    );
    assert_eq!(
        SeamsError::MandatorySeam {
            edge: EdgeId::new(4)
        }
        .to_string(),
        "a mandatory seam cannot be removed, replaced, or placed over another seam (edge EdgeId(4))"
    );
    assert_eq!(
        SeamsError::UnknownEdge {
            edge: EdgeId::new(9)
        }
        .to_string(),
        "a seam is on edge EdgeId(9), which the mesh does not have"
    );
    assert_eq!(
        SeamsError::ChartNotClosed {
            chart: ChartId::new(3)
        }
        .to_string(),
        "the cuts found for chart ChartId(3) do not leave it one disk"
    );
    assert_eq!(
        SeamsError::NotASeam {
            edge: EdgeId::new(7)
        }
        .to_string(),
        "edge EdgeId(7) has no seam to mark"
    );
    assert_eq!(
        SeamsError::FoldedChart {
            chart: ChartId::new(2),
            edge: EdgeId::new(17)
        }
        .to_string(),
        "chart ChartId(2) lies back over itself across edge EdgeId(17)"
    );
    assert_eq!(
        SeamsError::UnpairedBoundarySeam {
            chain: SeamChainId::new(5)
        }
        .to_string(),
        "seam chain SeamChainId(5) runs along the mesh boundary with no recorded partner"
    );
    assert_eq!(
        SeamsError::UnknownVertex {
            vertex: VertexId::new(8)
        }
        .to_string(),
        "vertex VertexId(8) is not one of the mesh's"
    );
    assert_eq!(
        SeamsError::NoPath {
            from: VertexId::new(1),
            to: VertexId::new(6)
        }
        .to_string(),
        "no path of edges joins vertex VertexId(1) to vertex VertexId(6)"
    );
}
