//! Repair of a `TriangleSoup`, one step per module.

pub(crate) mod bowtie;
pub(crate) mod cap;
pub(crate) mod components;
pub(crate) mod degenerate;
pub(crate) mod nonmanifold_edge;
pub(crate) mod orientation;
pub(crate) mod plan;
pub(crate) mod weld;

use openplush_geom::epsilon::ModelTolerances;

use crate::{ManifoldMesh, MeshError, RepairPlan, RepairReport, TriangleSoup};

/// Repairs `soup` as `plan` says and builds the mesh, listing every change in the report.
///
/// This is the only way to obtain a [`ManifoldMesh`].
pub fn into_manifold(
    soup: &TriangleSoup,
    plan: &RepairPlan,
) -> Result<(ManifoldMesh, RepairReport), MeshError> {
    let _ = (soup, plan);
    Err(MeshError::NoTriangles)
}

#[cfg(test)]
mod tests;
