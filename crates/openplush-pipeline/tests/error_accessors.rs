//! `EngineError`'s wire code, locations and proposals, as the binding reads them.

use std::collections::BTreeMap;
use std::error::Error;

use openplush_pipeline::{
    ChartFailure, ChartId, EdgeId, EngineError, ErrorLocations, FaceId, Format, PieceId, Proposal,
    ProposalId, ProposalKind, SeamChainId, SeamKind, SourceCorner, VertexId,
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

/// One value of every variant, paired with its expected code.
fn every_variant() -> Result<Vec<(EngineError, &'static str)>, Box<dyn Error>> {
    let mut all = import_and_seam_variants()?;
    all.extend(flatten_and_later_variants());
    Ok(all)
}

fn import_and_seam_variants() -> Result<Vec<(EngineError, &'static str)>, Box<dyn Error>> {
    Ok(vec![
        (
            EngineError::Parse {
                format: Format::Obj,
                detail: "bad face".into(),
            },
            "Parse",
        ),
        (
            EngineError::NonFiniteInput {
                vertices: vec![SourceCorner {
                    primitive: 0,
                    vertex: 1,
                }],
            },
            "NonFiniteInput",
        ),
        (
            EngineError::NotManifold {
                edges: vec![EdgeId::new(2)],
                vertices: vec![],
            },
            "NotManifold",
        ),
        (
            EngineError::ChartNotDisk {
                chart: ChartId::new(3),
                failure: ChartFailure::MultipleBoundaries { count: 2 },
                proposals: vec![dart(3, 7)?],
            },
            "ChartNotDisk",
        ),
    ])
}

fn flatten_and_later_variants() -> Vec<(EngineError, &'static str)> {
    vec![
        (
            EngineError::Flip {
                chart: ChartId::new(1),
                faces: vec![FaceId::new(4)],
            },
            "Flip",
        ),
        (
            EngineError::BoundaryNotRealizable {
                chart: ChartId::new(5),
                residual_mm: 0.5,
                proposals: vec![],
            },
            "BoundaryNotRealizable",
        ),
        (
            EngineError::SeamParity {
                chain: SeamChainId::new(6),
                a_mm: 10.0,
                b_mm: 12.0,
                tolerance_mm: 1.0,
            },
            "SeamParity",
        ),
        (
            EngineError::OffsetFailed {
                piece: PieceId::new(2),
                detail: "self-intersection".into(),
            },
            "OffsetFailed",
        ),
        (
            EngineError::ProposalNotOffered {
                id: ProposalId::from_raw(9),
            },
            "ProposalNotOffered",
        ),
        (EngineError::Cancelled, "Cancelled"),
    ]
}

#[test]
fn code_is_the_variant_name() -> Result<(), Box<dyn Error>> {
    for (e, code) in every_variant()? {
        assert_eq!(e.code(), code, "{e:?}");
    }
    Ok(())
}

#[test]
fn only_parse_and_cancelled_have_no_locations() -> Result<(), Box<dyn Error>> {
    for (e, code) in every_variant()? {
        let unlocated = e.locations() == ErrorLocations::default();
        assert_eq!(
            unlocated,
            matches!(code, "Parse" | "Cancelled"),
            "{code}: {:?}",
            e.locations()
        );
    }
    Ok(())
}

#[test]
fn not_manifold_locates_its_edges_and_vertices_sorted() {
    let e = EngineError::NotManifold {
        edges: vec![EdgeId::new(8), EdgeId::new(3)],
        vertices: vec![VertexId::new(5), VertexId::new(1)],
    };
    let expected = ErrorLocations {
        edges: vec![EdgeId::new(3), EdgeId::new(8)],
        vertices: vec![VertexId::new(1), VertexId::new(5)],
        ..ErrorLocations::default()
    };
    assert_eq!(e.locations(), expected);
}

#[test]
fn flip_locates_its_chart_and_faces() {
    let e = EngineError::Flip {
        chart: ChartId::new(1),
        faces: vec![FaceId::new(40), FaceId::new(20), FaceId::new(21)],
    };
    let expected = ErrorLocations {
        faces: vec![FaceId::new(20), FaceId::new(21), FaceId::new(40)],
        charts: vec![ChartId::new(1)],
        ..ErrorLocations::default()
    };
    assert_eq!(e.locations(), expected);
}

#[test]
fn non_finite_input_locates_source_vertices_sorted() {
    let late = SourceCorner {
        primitive: 2,
        vertex: 0,
    };
    let early = SourceCorner {
        primitive: 0,
        vertex: 9,
    };
    let e = EngineError::NonFiniteInput {
        vertices: vec![late, early],
    };
    let expected = ErrorLocations {
        source_vertices: vec![early, late],
        ..ErrorLocations::default()
    };
    assert_eq!(e.locations(), expected);
}

#[test]
fn chart_not_disk_locates_its_chart_not_its_proposals_edges() -> Result<(), Box<dyn Error>> {
    let e = EngineError::ChartNotDisk {
        chart: ChartId::new(4),
        failure: ChartFailure::MultipleBoundaries { count: 2 },
        proposals: vec![dart(4, 11)?],
    };
    let expected = ErrorLocations {
        charts: vec![ChartId::new(4)],
        ..ErrorLocations::default()
    };
    assert_eq!(e.locations(), expected);
    Ok(())
}

#[test]
fn proposal_not_offered_locates_the_proposal() {
    let id = ProposalId::from_raw(0x2b32_133a_f65b_aa57);
    let expected = ErrorLocations {
        proposal: Some(id),
        ..ErrorLocations::default()
    };
    assert_eq!(EngineError::ProposalNotOffered { id }.locations(), expected);
}

#[test]
fn proposals_come_from_the_two_variants_that_carry_them() -> Result<(), Box<dyn Error>> {
    let fix = dart(4, 11)?;
    let not_disk = EngineError::ChartNotDisk {
        chart: ChartId::new(4),
        failure: ChartFailure::MultipleBoundaries { count: 2 },
        proposals: vec![fix.clone()],
    };
    assert_eq!(not_disk.proposals(), [fix]);

    let darts = vec![dart(6, 2)?, dart(6, 5)?];
    let not_realizable = EngineError::BoundaryNotRealizable {
        chart: ChartId::new(6),
        residual_mm: 1.25,
        proposals: darts.clone(),
    };
    assert_eq!(not_realizable.proposals(), darts.as_slice());

    let flip = EngineError::Flip {
        chart: ChartId::new(1),
        faces: vec![FaceId::new(2)],
    };
    assert!(flip.proposals().is_empty());
    Ok(())
}
