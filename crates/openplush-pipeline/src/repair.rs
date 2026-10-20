use openplush_mesh::{ManifoldMesh, RepairPlan, RepairReport, TriangleSoup, ValidationReport};

use crate::{StageResult, Staged, WarningSet};

/// Every defect in `soup` that repair would meet, changing nothing.
#[must_use]
pub fn validate(soup: &TriangleSoup) -> Staged<ValidationReport> {
    Staged {
        value: openplush_mesh::validate(soup),
        warnings: WarningSet::default(),
    }
}

/// Repairs `soup` as `plan` says into a manifold mesh, with the report of every change.
///
/// # Errors
/// `Mesh`, carrying the mesh crate's refusal, in its order.
pub fn repair(soup: &TriangleSoup, plan: &RepairPlan) -> StageResult<(ManifoldMesh, RepairReport)> {
    Ok(Staged {
        value: openplush_mesh::into_manifold(soup, plan)?,
        warnings: WarningSet::default(),
    })
}
