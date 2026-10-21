//! The seam-set file: the `openplush-seam-set` format of `fixtures/README.md`, version 1.

use std::collections::{BTreeMap, BTreeSet};

use openplush_pipeline::{HalfEdgeId, ManifoldMesh, SeamKind, SeamSet, VertexId};
use serde_json::Value;

/// One seam as the file gives it: two source vertex indices, counted from the file's base.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Seam {
    edge: [u64; 2],
    kind: SeamKind,
}

/// The file's seams, in file order, with the file's index base.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SeamFile {
    base: u64,
    seams: Vec<Seam>,
}

/// Reads the file's JSON; a refusal says which field is wrong.
pub(crate) fn parse(bytes: &[u8]) -> Result<SeamFile, String> {
    let root: Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if root["format"] != "openplush-seam-set" {
        return Err("\"format\" is not \"openplush-seam-set\"".into());
    }
    if root["version"].as_u64() != Some(1) {
        return Err("\"version\" is not 1".into());
    }
    let base = match root["vertexIndexBase"].as_u64() {
        Some(b @ (0 | 1)) => b,
        _ => return Err("\"vertexIndexBase\" is not 0 or 1".into()),
    };
    let seams = root["seams"]
        .as_array()
        .ok_or("\"seams\" is not a list")?
        .iter()
        .enumerate()
        .map(|(n, s)| seam(s).map_err(|why| format!("seam {n}: {why}")))
        .collect::<Result<_, _>>()?;
    Ok(SeamFile { base, seams })
}

fn seam(value: &Value) -> Result<Seam, String> {
    let ends: Vec<u64> = value["edge"]
        .as_array()
        .map(|e| e.iter().filter_map(Value::as_u64).collect())
        .unwrap_or_default();
    let edge = <[u64; 2]>::try_from(ends).map_err(|_| "\"edge\" is not two integer indices")?;
    let kind = match value["kind"].as_str() {
        Some("cut") => SeamKind::Cut,
        Some("dart") => SeamKind::Dart,
        Some("opening") => SeamKind::Opening,
        Some("fabricBoundary") => SeamKind::FabricBoundary,
        Some("mandatory") => SeamKind::Mandatory,
        _ => return Err("\"kind\" is not a seam kind".into()),
    };
    Ok(Seam { edge, kind })
}

/// Why a seam cannot be placed on the mesh.
#[derive(Debug)]
pub(crate) enum Unplaced {
    /// The text, naming the seam by its place in the file.
    Seam(String),
    /// The seam set refused the edge.
    Set(openplush_pipeline::SeamsError),
}

/// The seams on `mesh`: each index names the source vertex every mesh vertex records through
/// `corner_source`, so an index must name exactly one mesh vertex, and the two an edge.
pub(crate) fn apply(file: &SeamFile, mesh: &ManifoldMesh) -> Result<SeamSet, Unplaced> {
    let from = sources(mesh);
    let mut set = SeamSet::new();
    for (n, seam) in file.seams.iter().enumerate() {
        let vertex = |i: u64| -> Result<VertexId, Unplaced> {
            let shown = i + file.base;
            let found: Vec<VertexId> = from.get(&i).into_iter().flatten().copied().collect();
            match found[..] {
                [v] => Ok(v),
                [] => Err(Unplaced::Seam(format!(
                    "seam {n}: no vertex {shown} in the model"
                ))),
                _ => Err(Unplaced::Seam(format!(
                    "seam {n}: vertex {shown} is more than one vertex of the repaired model"
                ))),
            }
        };
        let [a, b] = seam.edge.map(|i| i.checked_sub(file.base));
        let (Some(a), Some(b)) = (a, b) else {
            return Err(Unplaced::Seam(format!(
                "seam {n}: an index is below the file's base"
            )));
        };
        let (va, vb) = (vertex(a)?, vertex(b)?);
        let edge = mesh
            .outgoing(va)
            .find(|&h| mesh.dest(h) == vb)
            .ok_or_else(|| {
                Unplaced::Seam(format!(
                    "seam {n}: vertices {} and {} share no edge",
                    a + file.base,
                    b + file.base
                ))
            })?
            .edge();
        set.insert(edge, seam.kind).map_err(Unplaced::Set)?;
    }
    Ok(set)
}

/// Every source vertex index and the mesh vertices whose corners came from it.
fn sources(mesh: &ManifoldMesh) -> BTreeMap<u64, BTreeSet<VertexId>> {
    let mut from: BTreeMap<u64, BTreeSet<VertexId>> = BTreeMap::new();
    // Lossless: half-edge IDs are u32.
    for h in (0..mesh.half_edge_count() as u32).map(HalfEdgeId::new) {
        if let Some(corner) = mesh.corner_source(h) {
            from.entry(u64::from(corner.vertex))
                .or_default()
                .insert(mesh.origin(h));
        }
    }
    from
}
