//! Step 7, classification: each boundary loop of the repaired mesh is an opening unless the plan
//! lists it to fill, so a loop is never filled without being asked: a silent fill is the
//! invented geometry the engine refuses to produce.

use crate::{HalfEdgeId, LoopRef, ManifoldMesh, MeshError, RepairPlan};

/// A boundary loop, named for the plan, with whether the plan lists it as accidental (to fill).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Hole {
    pub(crate) loop_ref: LoopRef,
    pub(crate) accidental: bool,
    /// Its boundary half-edges, in `next` order.
    pub(crate) half_edges: Vec<HalfEdgeId>,
}

/// Every boundary loop of `mesh`, in order of its lowest half-edge, classified by
/// `plan.fill_holes`. A loop is named by the lowest source corner among its vertices, so a stored
/// plan names the same loop on every replay.
///
/// # Errors
/// `UnknownHoleLoop`, naming the lowest, when the plan lists a loop the mesh does not have.
pub(crate) fn classify(mesh: &ManifoldMesh, plan: &RepairPlan) -> Result<Vec<Hole>, MeshError> {
    let holes: Vec<Hole> = mesh
        .boundary_loops()
        .into_iter()
        .filter_map(|half_edges| {
            let loop_ref = half_edges
                .iter()
                // The twin of a boundary half-edge has a face, whose corner at its origin is
                // the boundary half-edge's destination.
                .filter_map(|&h| mesh.corner_source(h.twin()))
                .min()
                .map(LoopRef::new)?;
            Some(Hole {
                loop_ref,
                accidental: plan.fill_holes.contains(&loop_ref),
                half_edges,
            })
        })
        .collect();
    if let Some(&loop_ref) = plan
        .fill_holes
        .iter()
        .find(|l| !holes.iter().any(|h| h.loop_ref == **l))
    {
        return Err(MeshError::UnknownHoleLoop { loop_ref });
    }
    Ok(holes)
}

#[cfg(test)]
mod tests;
