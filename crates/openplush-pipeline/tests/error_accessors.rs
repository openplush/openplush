//! `EngineError`'s wire code, locations and proposals, as the binding reads them.

use std::collections::BTreeMap;
use std::error::Error;

use openplush_pipeline::{
    ChartFailure, ChartId, EdgeId, EngineError, ErrorLocations, FaceId, Format, PieceId, Proposal,
    ProposalId, ProposalKind, SeamChainId, SeamKind, SeamsError, SourceCorner, VertexId,
};
use openplush_pipeline::{
    CornerArray, Counted, HalfEdgeId, LoopRef, MaterialFault, MeshElement, MeshError,
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
            EngineError::Mesh(MeshError::IsolatedVertex {
                vertex: VertexId::new(2),
            }),
            "Mesh",
        ),
        (
            EngineError::ChartNotDisk {
                chart: ChartId::new(3),
                failure: ChartFailure::MultipleBoundaries { count: 2 },
                proposals: vec![dart(3, 7)?],
            },
            "ChartNotDisk",
        ),
        (
            EngineError::FoldedChart {
                chart: ChartId::new(3),
                edge: EdgeId::new(7),
            },
            "FoldedChart",
        ),
        (
            EngineError::Seams(SeamsError::UnknownEdge {
                edge: EdgeId::new(8),
            }),
            "Seams",
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
fn flip_locates_its_chart_and_faces_sorted_without_duplicates() {
    let e = EngineError::Flip {
        chart: ChartId::new(1),
        faces: vec![
            FaceId::new(40),
            FaceId::new(20),
            FaceId::new(21),
            FaceId::new(20),
        ],
    };
    let expected = ErrorLocations {
        faces: vec![FaceId::new(20), FaceId::new(21), FaceId::new(40)],
        charts: vec![ChartId::new(1)],
        ..ErrorLocations::default()
    };
    assert_eq!(e.locations(), expected);
}

#[test]
fn non_finite_input_locates_source_vertices_sorted_without_duplicates() {
    let late = SourceCorner {
        primitive: 2,
        vertex: 0,
    };
    let early = SourceCorner {
        primitive: 0,
        vertex: 9,
    };
    let e = EngineError::NonFiniteInput {
        vertices: vec![late, early, late],
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
fn folded_chart_locates_its_chart_and_edge() {
    let e = EngineError::FoldedChart {
        chart: ChartId::new(3),
        edge: EdgeId::new(7),
    };
    let expected = ErrorLocations {
        edges: vec![EdgeId::new(7)],
        charts: vec![ChartId::new(3)],
        ..ErrorLocations::default()
    };
    assert_eq!(e.locations(), expected);
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

#[test]
fn boundary_not_realizable_locates_its_chart_not_its_darts() -> Result<(), Box<dyn Error>> {
    let e = EngineError::BoundaryNotRealizable {
        chart: ChartId::new(6),
        residual_mm: 1.25,
        proposals: vec![dart(6, 2)?],
    };
    let expected = ErrorLocations {
        charts: vec![ChartId::new(6)],
        ..ErrorLocations::default()
    };
    assert_eq!(e.locations(), expected);
    Ok(())
}

#[test]
fn seam_parity_and_offset_failed_locate_their_own_element() {
    let parity = EngineError::SeamParity {
        chain: SeamChainId::new(6),
        a_mm: 10.0,
        b_mm: 12.0,
        tolerance_mm: 1.0,
    };
    let chain = ErrorLocations {
        chains: vec![SeamChainId::new(6)],
        ..ErrorLocations::default()
    };
    assert_eq!(parity.locations(), chain);

    let offset = EngineError::OffsetFailed {
        piece: PieceId::new(2),
        detail: "self-intersection".into(),
    };
    let piece = ErrorLocations {
        pieces: vec![PieceId::new(2)],
        ..ErrorLocations::default()
    };
    assert_eq!(offset.locations(), piece);
}

fn seams_locations(inner: SeamsError) -> ErrorLocations {
    EngineError::Seams(inner).locations()
}

#[test]
fn seams_locates_the_elements_its_inner_error_names() {
    let (edge, chart, chain, vertex) = (
        EdgeId::new(4),
        ChartId::new(2),
        SeamChainId::new(6),
        VertexId::new(9),
    );
    let only_edge = ErrorLocations {
        edges: vec![edge],
        ..ErrorLocations::default()
    };
    for inner in [
        SeamsError::MandatorySeam { edge },
        SeamsError::UnknownEdge { edge },
        SeamsError::NotASeam { edge },
        SeamsError::NotAWalk { edge },
    ] {
        assert_eq!(seams_locations(inner.clone()), only_edge, "{inner:?}");
    }
    assert_eq!(
        seams_locations(SeamsError::ChartNotClosed { chart }),
        ErrorLocations {
            charts: vec![chart],
            ..ErrorLocations::default()
        }
    );
    assert_eq!(
        seams_locations(SeamsError::FoldedChart { chart, edge }),
        ErrorLocations {
            edges: vec![edge],
            charts: vec![chart],
            ..ErrorLocations::default()
        }
    );
    for inner in [
        SeamsError::UnpairedBoundarySeam { chain },
        SeamsError::CopyGroupMismatch { chain },
    ] {
        assert_eq!(
            seams_locations(inner.clone()),
            ErrorLocations {
                chains: vec![chain],
                ..ErrorLocations::default()
            },
            "{inner:?}"
        );
    }
    for inner in [
        SeamsError::UnknownVertex { vertex },
        SeamsError::DartNotAnchored { vertex },
        SeamsError::DartTouchesSeam { vertex },
    ] {
        assert_eq!(
            seams_locations(inner.clone()),
            ErrorLocations {
                vertices: vec![vertex],
                ..ErrorLocations::default()
            },
            "{inner:?}"
        );
    }
}

#[test]
fn seams_locates_both_picks_of_a_missing_path_sorted() {
    let (a, b) = (VertexId::new(9), VertexId::new(3));
    assert_eq!(
        seams_locations(SeamsError::NoPath { from: a, to: b }),
        ErrorLocations {
            vertices: vec![b, a],
            ..ErrorLocations::default()
        }
    );
}

#[test]
fn seams_refusals_about_a_proposal_or_dart_as_a_whole_locate_nothing() {
    for inner in [
        SeamsError::EmptyProposal,
        SeamsError::EmptyDart,
        SeamsError::BadProposalId { text: "x".into() },
    ] {
        assert_eq!(
            seams_locations(inner.clone()),
            ErrorLocations::default(),
            "{inner:?}"
        );
    }
}

#[test]
fn seams_offers_no_proposals() {
    let e = EngineError::Seams(SeamsError::EmptyProposal);
    assert!(e.proposals().is_empty());
}

fn mesh_locations(inner: MeshError) -> ErrorLocations {
    EngineError::Mesh(inner).locations()
}

#[test]
fn mesh_locates_the_soup_faces_its_inner_error_names_sorted() {
    let (f, g) = (FaceId::new(7), FaceId::new(2));
    for inner in [
        MeshError::IndexOutOfRange {
            triangle: f,
            index: 9,
            vertex_count: 3,
        },
        MeshError::DegenerateTriangle { triangle: f },
        MeshError::ZeroAreaTriangle { triangle: f },
    ] {
        assert_eq!(
            mesh_locations(inner),
            ErrorLocations {
                faces: vec![f],
                ..ErrorLocations::default()
            },
            "{inner:?}"
        );
    }
    assert_eq!(
        mesh_locations(MeshError::DuplicateTriangle {
            triangle: f,
            first: g
        }),
        ErrorLocations {
            faces: vec![g, f],
            ..ErrorLocations::default()
        }
    );
}

#[test]
fn mesh_locates_the_vertices_its_inner_error_names_sorted() {
    let (v, w) = (VertexId::new(5), VertexId::new(1));
    let one = ErrorLocations {
        vertices: vec![v],
        ..ErrorLocations::default()
    };
    for inner in [
        MeshError::NonFiniteVertex { vertex: v },
        MeshError::IsolatedVertex { vertex: v },
        MeshError::NonManifoldVertex { vertex: v },
    ] {
        assert_eq!(mesh_locations(inner), one, "{inner:?}");
    }
    let both = ErrorLocations {
        vertices: vec![w, v],
        ..ErrorLocations::default()
    };
    for inner in [
        MeshError::NonManifoldEdge { a: v, b: w },
        MeshError::InconsistentOrientation { a: v, b: w },
    ] {
        assert_eq!(mesh_locations(inner), both, "{inner:?}");
    }
    assert_eq!(
        mesh_locations(MeshError::PairingMismatch {
            face: FaceId::new(3),
            a: v,
            b: w
        }),
        ErrorLocations {
            faces: vec![FaceId::new(3)],
            vertices: vec![w, v],
            ..ErrorLocations::default()
        }
    );
}

#[test]
fn mesh_locates_a_loop_by_its_source_corner() {
    let corner = SourceCorner {
        primitive: 1,
        vertex: 4,
    };
    let expected = ErrorLocations {
        source_vertices: vec![corner],
        ..ErrorLocations::default()
    };
    for inner in [
        MeshError::UnknownHoleLoop {
            loop_ref: LoopRef::new(corner),
        },
        MeshError::HoleFillUnavailable {
            loop_ref: LoopRef::new(corner),
        },
    ] {
        assert_eq!(mesh_locations(inner), expected, "{inner:?}");
    }
}

#[test]
fn mesh_locates_a_broken_invariant_by_its_element() {
    let broken = |element| MeshError::BrokenInvariant {
        invariant: 2,
        element,
    };
    assert_eq!(
        mesh_locations(broken(MeshElement::Vertex(VertexId::new(4)))).vertices,
        [VertexId::new(4)]
    );
    assert_eq!(
        mesh_locations(broken(MeshElement::Face(FaceId::new(6)))).faces,
        [FaceId::new(6)]
    );
    // A half-edge is located by its edge: half-edges 6 and 7 are edge 3.
    assert_eq!(
        mesh_locations(broken(MeshElement::HalfEdge(HalfEdgeId::new(7)))).edges,
        [EdgeId::new(3)]
    );
    assert_eq!(
        mesh_locations(broken(MeshElement::Storage("he_next"))),
        ErrorLocations::default()
    );
}

#[test]
fn mesh_refusals_about_the_whole_soup_locate_nothing() {
    for inner in [
        MeshError::CornerCountMismatch {
            array: CornerArray::Uvs,
            expected: 6,
            found: 3,
        },
        MeshError::TooLarge {
            what: Counted::Vertices,
            count: 1 << 33,
        },
        MeshError::InvalidMaterial {
            primitive: 0,
            fault: MaterialFault::Missing,
        },
        MeshError::NoTriangles,
        MeshError::UnusableExtent,
    ] {
        assert_eq!(
            mesh_locations(inner),
            ErrorLocations::default(),
            "{inner:?}"
        );
    }
}

#[test]
fn mesh_offers_no_proposals() {
    assert!(
        EngineError::Mesh(MeshError::NoTriangles)
            .proposals()
            .is_empty()
    );
}
