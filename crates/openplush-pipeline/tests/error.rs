//! `EngineError`: every variant locates its problem, and its message is pinned.
//! Everything is named through `openplush_pipeline`, as a shell would.

use std::collections::BTreeMap;
use std::error::Error;

use openplush_pipeline::MeshError;
use openplush_pipeline::{
    ChartFailure, ChartId, EdgeId, EngineError, ExportError, FaceId, Format, LayoutError,
    PatternError, PieceId, Proposal, ProposalId, ProposalKind, SeamChainId, SeamKind, SeamsError,
    SourceCorner, VertexId,
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
fn mesh_prints_its_inner_errors_message() {
    let inner = MeshError::NonManifoldEdge {
        a: VertexId::new(1),
        b: VertexId::new(4),
    };
    let e = EngineError::Mesh(inner);
    assert_eq!(e.to_string(), inner.to_string());
    assert_eq!(
        e.to_string(),
        "edge between VertexId(1) and VertexId(4) is shared by more than two triangles"
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
    assert_eq!(
        e.to_string(),
        "chart 2 lies back over itself across edge 17"
    );
}

#[test]
fn boundary_not_realizable_names_the_chart_and_carries_its_darts() -> Result<(), Box<dyn Error>> {
    let fix = dart(6, 2)?;
    let e = EngineError::BoundaryNotRealizable {
        chart: ChartId::new(6),
        residual: 1.25,
        proposals: vec![fix.clone()],
    };
    assert!(matches!(
        &e,
        EngineError::BoundaryNotRealizable { chart, proposals, .. }
            if *chart == ChartId::new(6) && *proposals == [fix.clone()]
    ));
    assert_eq!(
        e.to_string(),
        "chart 6's seam lengths cannot close flat (residual 1.250 in model units)"
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
fn pattern_carries_its_refusal_whole_and_says_it_in_the_refusal_s_words() {
    let e = EngineError::Pattern {
        piece: Some(PieceId::new(3)),
        source: PatternError::FoldConflict {
            piece: PieceId::new(3),
            edge: 7,
        },
    };
    assert!(matches!(
        &e,
        EngineError::Pattern { piece: Some(piece), source: PatternError::FoldConflict { edge: 7, .. } }
            if *piece == PieceId::new(3)
    ));
    assert_eq!(
        e.to_string(),
        "piece 3: the fold or edge-stitched edge 7 faces another part of the piece too closely"
    );
    let outline = EngineError::Pattern {
        piece: Some(PieceId::new(1)),
        source: PatternError::OutlineNotSimple { edge: 2, other: 9 },
    };
    assert_eq!(
        outline.to_string(),
        "outline edges 2 and 9 cross, touch, overlap or fold back"
    );
}

#[test]
fn stage_input_mismatch_names_the_chart() {
    let e = EngineError::StageInputMismatch {
        chart: ChartId::new(4),
    };
    assert_eq!(
        e.to_string(),
        "chart 4: the layouts or sewing map given do not belong to the charts given"
    );
}

#[test]
fn layout_carries_its_refusal_whole_and_locates_a_piece_too_wide() {
    let e = EngineError::Layout(LayoutError::PieceTooWide {
        piece: PieceId::new(1),
        width_mm: 112.0,
        roll_mm: 50.0,
    });
    assert_eq!(
        e.to_string(),
        "piece 1 is 112.000 mm wide but the roll is 50.000 mm"
    );
    assert_eq!(e.locations().pieces, vec![PieceId::new(1)]);
    let rotation = EngineError::Layout(LayoutError::UnsupportedRotation {
        piece: PieceId::new(2),
        rotation_deg: 45.0,
    });
    assert_eq!(rotation.locations().pieces, vec![PieceId::new(2)]);
    for e in [
        LayoutError::NoPieces,
        LayoutError::BadRollWidth,
        LayoutError::TooManyPages {
            width_mm: 9000.0,
            length_mm: 9000.0,
        },
    ] {
        assert!(EngineError::Layout(e).locations().pieces.is_empty());
    }
}

#[test]
fn export_carries_its_refusal_whole_and_locates_the_piece_it_names() {
    let unknown = EngineError::Export(ExportError::UnknownPiece {
        piece: PieceId::new(3),
    });
    assert_eq!(
        unknown.to_string(),
        "the sheet places piece 3, which was not given"
    );
    assert_eq!(unknown.locations().pieces, vec![PieceId::new(3)]);
    let twice = EngineError::Export(ExportError::DuplicatePiece {
        piece: PieceId::new(5),
    });
    assert_eq!(twice.to_string(), "piece 5 was given more than once");
    assert_eq!(twice.locations().pieces, vec![PieceId::new(5)]);
    let unprintable = EngineError::Export(ExportError::Unprintable {
        text: "\u{2192}".into(),
        character: '\u{2192}',
    });
    assert_eq!(
        unprintable.to_string(),
        "\"\u{2192}\" has '\u{2192}', which is outside the Latin-1 the exports can print"
    );
    assert!(unprintable.locations().pieces.is_empty());
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

#[test]
fn seams_prints_its_inner_errors_message() {
    let inner = SeamsError::UnpairedBoundarySeam {
        chain: SeamChainId::new(3),
    };
    let e = EngineError::Seams(inner.clone());
    assert_eq!(e.to_string(), inner.to_string());
    assert_eq!(
        e.to_string(),
        "seam chain SeamChainId(3) runs along the mesh boundary with no recorded partner"
    );
}
