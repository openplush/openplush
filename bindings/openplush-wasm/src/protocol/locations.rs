use openplush_pipeline::{ErrorLocations, SourceCorner};
use serde::Serialize;

/// The elements an error names, by category; a category with none is absent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub(crate) struct Locations {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    faces: Option<Vec<u32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    edges: Option<Vec<u32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    vertices: Option<Vec<u32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pieces: Option<Vec<u32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    charts: Option<Vec<u32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    chains: Option<Vec<u32>>,
    /// Positions in the imported file, before repair.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    source_vertices: Option<Vec<SourceVertex>>,
    /// A `ProposalId`, as 16 lowercase hex digits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    proposal: Option<String>,
}

/// A vertex of the imported file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub(crate) struct SourceVertex {
    primitive: u32,
    vertex: u32,
}

impl From<&ErrorLocations> for Locations {
    fn from(l: &ErrorLocations) -> Self {
        // No `..`: a category added to `ErrorLocations` must fail to compile here, not vanish.
        let ErrorLocations {
            faces,
            edges,
            vertices,
            pieces,
            charts,
            chains,
            source_vertices,
            proposal,
        } = l;
        Self {
            faces: ids(faces, |id| id.index()),
            edges: ids(edges, |id| id.index()),
            vertices: ids(vertices, |id| id.index()),
            pieces: ids(pieces, |id| id.index()),
            charts: ids(charts, |id| id.index()),
            chains: ids(chains, |id| id.index()),
            source_vertices: ids(source_vertices, |&SourceCorner { primitive, vertex }| {
                SourceVertex { primitive, vertex }
            }),
            proposal: proposal.map(|id| id.to_hex()),
        }
    }
}

/// `None` for an empty list, so the category is left out rather than sent empty.
fn ids<T, U>(list: &[T], wire: impl Fn(&T) -> U) -> Option<Vec<U>> {
    (!list.is_empty()).then(|| list.iter().map(wire).collect())
}
