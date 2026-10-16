//! The JSON shape of the error DTO and the binding's own codes.

use std::collections::BTreeMap;
use std::error::Error;

use openplush_pipeline::{
    ChartFailure, ChartId, EdgeId, EngineError, FaceId, Format, PieceId, Proposal, ProposalId,
    ProposalKind, SeamChainId, SeamKind, SourceCorner, VertexId,
};
use openplush_wasm::protocol::{BindingCode, EngineErrorDto};
use serde_json::{Value, json};

fn wire(e: &EngineError) -> Result<Value, serde_json::Error> {
    serde_json::to_value(EngineErrorDto::from(e))
}

fn dart(chart: u32, edges: &[u32]) -> Result<Proposal, Box<dyn Error>> {
    let add = edges
        .iter()
        .map(|&e| (EdgeId::new(e), SeamKind::Dart))
        .collect::<BTreeMap<_, _>>();
    let kind = ProposalKind::Dart {
        chart: ChartId::new(chart),
    };
    Ok(Proposal::new(kind, add, "a dart at the crown".into())?)
}

#[test]
fn binding_codes_serialize_as_their_names() -> Result<(), serde_json::Error> {
    assert_eq!(
        serde_json::to_value(BindingCode::StaleHandle)?,
        json!("StaleHandle")
    );
    assert_eq!(
        serde_json::to_value(BindingCode::EngineCrashed)?,
        json!("EngineCrashed")
    );
    Ok(())
}

#[test]
fn parse_and_cancelled_have_no_locations_key() -> Result<(), serde_json::Error> {
    let parse = EngineError::Parse {
        format: Format::Obj,
        detail: "bad face".into(),
    };
    assert_eq!(
        wire(&parse)?,
        json!({ "code": "Parse", "message": "could not read the OBJ file: bad face" })
    );
    assert_eq!(
        wire(&EngineError::Cancelled)?,
        json!({ "code": "Cancelled", "message": "cancelled" })
    );
    Ok(())
}

#[test]
fn non_finite_input_locates_source_vertices() -> Result<(), serde_json::Error> {
    let e = EngineError::NonFiniteInput {
        vertices: vec![
            SourceCorner {
                primitive: 2,
                vertex: 0,
            },
            SourceCorner {
                primitive: 0,
                vertex: 9,
            },
        ],
    };
    let expected = json!({
        "code": "NonFiniteInput",
        "message": "2 source vertices have a NaN or infinite coordinate",
        "locations": { "sourceVertices": [
            { "primitive": 0, "vertex": 9 },
            { "primitive": 2, "vertex": 0 },
        ] },
    });
    assert_eq!(wire(&e)?, expected);
    Ok(())
}

#[test]
fn not_manifold_omits_its_empty_vertex_list() -> Result<(), serde_json::Error> {
    let e = EngineError::NotManifold {
        edges: vec![EdgeId::new(8), EdgeId::new(3)],
        vertices: vec![],
    };
    let expected = json!({
        "code": "NotManifold",
        "message": "the mesh is not manifold at 2 edges and 0 vertices",
        "locations": { "edges": [3, 8] },
    });
    assert_eq!(wire(&e)?, expected);
    Ok(())
}

#[test]
fn not_manifold_locates_vertices_too() -> Result<(), serde_json::Error> {
    let e = EngineError::NotManifold {
        edges: vec![EdgeId::new(1)],
        vertices: vec![VertexId::new(5)],
    };
    let expected = json!({
        "code": "NotManifold",
        "message": "the mesh is not manifold at 1 edges and 1 vertices",
        "locations": { "edges": [1], "vertices": [5] },
    });
    assert_eq!(wire(&e)?, expected);
    Ok(())
}

#[test]
fn chart_not_disk_carries_its_proposals() -> Result<(), Box<dyn Error>> {
    let fix = dart(3, &[11, 7])?;
    let e = EngineError::ChartNotDisk {
        chart: ChartId::new(3),
        failure: ChartFailure::MultipleBoundaries { count: 2 },
        proposals: vec![fix.clone()],
    };
    let expected = json!({
        "code": "ChartNotDisk",
        "message": "chart 3 is not a topological disk",
        "locations": { "charts": [3] },
        "proposals": [{
            "id": fix.id().to_hex(),
            "edges": [7, 11],
            "explanation": "a dart at the crown",
        }],
    });
    assert_eq!(wire(&e)?, expected);
    Ok(())
}

#[test]
fn boundary_not_realizable_without_darts_has_no_proposals_key() -> Result<(), serde_json::Error> {
    let e = EngineError::BoundaryNotRealizable {
        chart: ChartId::new(5),
        residual_mm: 0.5,
        proposals: vec![],
    };
    let expected = json!({
        "code": "BoundaryNotRealizable",
        "message": "chart 5's seam lengths cannot close flat (residual 0.500 mm)",
        "locations": { "charts": [5] },
    });
    assert_eq!(wire(&e)?, expected);
    Ok(())
}

#[test]
fn flip_locates_faces_and_chart() -> Result<(), serde_json::Error> {
    let e = EngineError::Flip {
        chart: ChartId::new(1),
        faces: vec![FaceId::new(40), FaceId::new(20)],
    };
    let expected = json!({
        "code": "Flip",
        "message": "chart 1 flattens with 2 inverted faces",
        "locations": { "faces": [20, 40], "charts": [1] },
    });
    assert_eq!(wire(&e)?, expected);
    Ok(())
}

#[test]
fn seam_parity_locates_its_chain() -> Result<(), serde_json::Error> {
    let e = EngineError::SeamParity {
        chain: SeamChainId::new(6),
        a_mm: 10.0,
        b_mm: 12.0,
        tolerance_mm: 1.0,
    };
    let expected = json!({
        "code": "SeamParity",
        "message": "seam chain 6: sides 10.000 mm and 12.000 mm differ by more than 1.000 mm",
        "locations": { "chains": [6] },
    });
    assert_eq!(wire(&e)?, expected);
    Ok(())
}

#[test]
fn offset_failed_locates_its_piece() -> Result<(), serde_json::Error> {
    let e = EngineError::OffsetFailed {
        piece: PieceId::new(2),
        detail: "self-intersection".into(),
    };
    let expected = json!({
        "code": "OffsetFailed",
        "message": "seam allowance for piece 2 failed: self-intersection",
        "locations": { "pieces": [2] },
    });
    assert_eq!(wire(&e)?, expected);
    Ok(())
}

#[test]
fn proposal_not_offered_locates_the_proposal_as_hex() -> Result<(), serde_json::Error> {
    let e = EngineError::ProposalNotOffered {
        id: ProposalId::from_raw(0x2b32_133a_f65b_aa57),
    };
    let expected = json!({
        "code": "ProposalNotOffered",
        "message": "proposal 2b32133af65baa57 is no longer offered",
        "locations": { "proposal": "2b32133af65baa57" },
    });
    assert_eq!(wire(&e)?, expected);
    Ok(())
}

#[test]
fn a_small_proposal_id_keeps_its_leading_zeros() -> Result<(), serde_json::Error> {
    let e = EngineError::ProposalNotOffered {
        id: ProposalId::from_raw(9),
    };
    assert_eq!(
        wire(&e)?["locations"],
        json!({ "proposal": "0000000000000009" })
    );
    Ok(())
}
