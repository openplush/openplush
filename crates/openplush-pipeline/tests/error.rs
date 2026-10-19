//! `EngineError`: every variant locates its problem, and its message is pinned.
//! Everything is named through `openplush_pipeline`, as a shell would.

use std::collections::BTreeMap;
use std::error::Error;

use openplush_pipeline::{
    ChartFailure, ChartId, EdgeId, EngineError, FaceId, Format, PieceId, Proposal, ProposalId,
    ProposalKind, SeamChainId, SeamKind, SourceCorner, VertexId,
};

fn dart(chart: u32, edge: u32) -> Result<Proposal, Box<dyn Error>> {
    let add = BTreeMap::from([(EdgeId::new(edge), SeamKind::Dart)]);
    Proposal::new(
        ProposalKind::Dart {
            chart: ChartId::new(chart),
        },
        add,
        "a dart at the crown".into(),
    )
    .map_err(Into::into)
}

#[test]
fn parse_names_the_format_that_refused_the_file() {
    let e = EngineError::Parse {
        format: Format::Glb,
        detail: "truncated chunk header".into(),
    };
    assert!(matches!(
        e,
        EngineError::Parse {
            format: Format::Glb,
            ..
        }
    ));
    assert_eq!(
        e.to_string(),
        "could not read the GLB file: truncated chunk header"
    );
}

#[test]
fn non_finite_input_names_every_source_vertex() {
    let vertices = vec![
        SourceCorner {
            primitive: 0,
            vertex: 4,
        },
        SourceCorner {
            primitive: 2,
            vertex: 9,
        },
    ];
    let e = EngineError::NonFiniteInput {
        vertices: vertices.clone(),
    };
    assert!(matches!(&e, EngineError::NonFiniteInput { vertices: v } if *v == vertices));
    assert_eq!(
        e.to_string(),
        "2 source vertices have a NaN or infinite coordinate"
    );
}

#[test]
fn not_manifold_names_its_edges_and_vertices() {
    let e = EngineError::NotManifold {
        edges: vec![EdgeId::new(3), EdgeId::new(8)],
        vertices: vec![VertexId::new(5)],
    };
    assert!(matches!(
        &e,
        EngineError::NotManifold { edges, vertices }
            if *edges == [EdgeId::new(3), EdgeId::new(8)] && *vertices == [VertexId::new(5)]
    ));
    assert_eq!(
        e.to_string(),
        "the mesh is not manifold at 2 edges and 1 vertices"
    );
}

#[test]
fn chart_not_disk_names_the_chart_and_carries_its_proposals() -> Result<(), Box<dyn Error>> {
    let fix = dart(4, 11)?;
    let e = EngineError::ChartNotDisk {
        chart: ChartId::new(4),
        failure: ChartFailure::MultipleBoundaries { count: 2 },
        proposals: vec![fix.clone()],
    };
    assert!(matches!(
        &e,
        EngineError::ChartNotDisk { chart, failure: ChartFailure::MultipleBoundaries { count: 2 }, proposals }
            if *chart == ChartId::new(4) && *proposals == [fix.clone()]
    ));
    assert_eq!(e.to_string(), "chart 4 is not a topological disk");
    Ok(())
}

#[test]
fn flip_names_the_chart_and_its_inverted_faces() {
    let e = EngineError::Flip {
        chart: ChartId::new(1),
        faces: vec![FaceId::new(20), FaceId::new(21), FaceId::new(40)],
    };
    assert!(matches!(
        &e,
        EngineError::Flip { chart, faces }
            if *chart == ChartId::new(1) && faces.len() == 3 && faces[2] == FaceId::new(40)
    ));
    assert_eq!(e.to_string(), "chart 1 flattens with 3 inverted faces");
}

#[test]
fn folded_chart_names_the_chart_and_the_edge_it_folds_across() {
    let e = EngineError::FoldedChart {
        chart: ChartId::new(2),
        edge: EdgeId::new(17),
    };
    assert!(matches!(
        &e,
        EngineError::FoldedChart { chart, edge }
            if *chart == ChartId::new(2) && *edge == EdgeId::new(17)
    ));
    assert_eq!(e.to_string(), "chart 2 lies back over itself across edge 17");
}

#[test]
fn boundary_not_realizable_names_the_chart_and_carries_its_darts() -> Result<(), Box<dyn Error>> {
    let fix = dart(6, 2)?;
    let e = EngineError::BoundaryNotRealizable {
        chart: ChartId::new(6),
        residual_mm: 1.25,
        proposals: vec![fix.clone()],
    };
    assert!(matches!(
        &e,
        EngineError::BoundaryNotRealizable { chart, proposals, .. }
            if *chart == ChartId::new(6) && *proposals == [fix.clone()]
    ));
    assert_eq!(
        e.to_string(),
        "chart 6's seam lengths cannot close flat (residual 1.250 mm)"
    );
    Ok(())
}

#[test]
fn seam_parity_names_the_chain_and_both_lengths() {
    let e = EngineError::SeamParity {
        chain: SeamChainId::new(12),
        a_mm: 101.5,
        b_mm: 99.0,
        tolerance_mm: 1.0,
    };
    assert!(matches!(
        &e,
        EngineError::SeamParity { chain, .. } if *chain == SeamChainId::new(12)
    ));
    assert_eq!(
        e.to_string(),
        "seam chain 12: sides 101.500 mm and 99.000 mm differ by more than 1.000 mm"
    );
}

#[test]
fn offset_failed_names_the_piece() {
    let e = EngineError::OffsetFailed {
        piece: PieceId::new(3),
        detail: "the allowance crosses itself at a tight curve".into(),
    };
    assert!(matches!(
        &e,
        EngineError::OffsetFailed { piece, .. } if *piece == PieceId::new(3)
    ));
    assert_eq!(
        e.to_string(),
        "seam allowance for piece 3 failed: the allowance crosses itself at a tight curve"
    );
}

#[test]
fn proposal_not_offered_names_the_proposal_in_its_wire_form() {
    let id = ProposalId::from_raw(0x2b32_133a_f65b_aa57);
    let e = EngineError::ProposalNotOffered { id };
    assert!(matches!(&e, EngineError::ProposalNotOffered { id: got } if *got == id));
    assert_eq!(
        e.to_string(),
        "proposal 2b32133af65baa57 is no longer offered"
    );
}

#[test]
fn cancelled_has_nothing_to_locate() {
    // Cooperative cancellation is the caller's act, not a defect in the model.
    assert_eq!(EngineError::Cancelled.to_string(), "cancelled");
}
