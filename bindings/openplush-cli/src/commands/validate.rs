//! `validate <MODEL> --json <PATH>`: the health report, written as JSON.

use std::fs;
use std::path::Path;

use openplush_pipeline::{
    DuplicateOrientation, FaceId, Unmeasurable, ValidationReport, VertexId, import, validate,
};
use serde_json::{Value, json};

use crate::failure::{Failure, io};
use crate::input::{CAP, files};

/// Writes `model`'s report to `out`; validating never fails, so only import can refuse.
pub(crate) fn run(model: &Path, out: &Path) -> Result<(), Failure> {
    let soup = import(&files(model, CAP)?)?.value;
    let report = validate(&soup).value;
    fs::write(out, report_json(&report).to_string()).map_err(io(out))
}

/// Every list under its accessor's name, IDs as integers.
fn report_json(r: &ValidationReport) -> Value {
    json!({
        "unmeasurable": unmeasurable(r.unmeasurable()),
        "unwelded": r.unwelded().iter().map(|g| vertices(g)).collect::<Vec<_>>(),
        "zero_area": faces(r.zero_area()),
        "repeated_index": faces(r.repeated_index()),
        "duplicates": r.duplicates().iter().map(|d| json!({
            "face": d.face.index(),
            "of": d.of.index(),
            "orientation": match d.orientation {
                DuplicateOrientation::Same => "same",
                DuplicateOrientation::Opposite => "opposite",
            },
        })).collect::<Vec<_>>(),
        "non_manifold_edges": r.non_manifold_edges().iter().map(|e| json!({
            "vertices": vertices(&e.vertices),
            "faces": faces(&e.faces),
        })).collect::<Vec<_>>(),
        "non_manifold_vertices": vertices(r.non_manifold_vertices()),
        "winding_conflicts": r.winding_conflicts().iter().map(|p| vertices(p)).collect::<Vec<_>>(),
        "non_orientable": r.non_orientable().iter().map(|p| vertices(p)).collect::<Vec<_>>(),
        "inside_out": r.inside_out().iter().map(|c| faces(c)).collect::<Vec<_>>(),
        "boundary_loops": r.boundary_loops().iter().map(|l| vertices(l)).collect::<Vec<_>>(),
        "components": r.components().iter().map(|c| faces(c)).collect::<Vec<_>>(),
        "self_intersections": r.self_intersections().iter().map(|p| faces(p)).collect::<Vec<_>>(),
    })
}

fn unmeasurable(why: Option<Unmeasurable>) -> Value {
    match why {
        None => Value::Null,
        Some(Unmeasurable::NoTriangles) => json!("no_triangles"),
        Some(Unmeasurable::Extent { diagonal }) => json!({ "extent": { "diagonal": diagonal } }),
    }
}

/// Vertex IDs as their integer indices.
fn vertices(list: &[VertexId]) -> Vec<u32> {
    list.iter().map(|v| v.index()).collect()
}

/// Face IDs as their integer indices.
fn faces(list: &[FaceId]) -> Vec<u32> {
    list.iter().map(|f| f.index()).collect()
}
