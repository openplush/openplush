//! The shortest surface path between two picked vertices, along mesh edges, for seam drawing.

use openplush_mesh::{EdgeId, ManifoldMesh, VertexId};

use crate::{SeamsError, dijkstra};

/// The shortest edge walk from `from` to `to` by 3D length, as its edges in walking order; empty
/// when they are the same vertex. Among equal walks the same picks give the same one: ties go to
/// the lower vertex, then the lower edge.
///
/// # Errors
/// `UnknownVertex` for a pick `mesh` does not have; `NoPath` when no edges join the picks.
pub fn shortest_path(
    mesh: &ManifoldMesh,
    from: VertexId,
    to: VertexId,
) -> Result<Vec<EdgeId>, SeamsError> {
    for vertex in [from, to] {
        if vertex.index() as usize >= mesh.vertex_count() {
            return Err(SeamsError::UnknownVertex { vertex });
        }
    }
    let adjacent: Vec<Vec<(usize, EdgeId)>> = (0..mesh.vertex_count())
        .map(|v| {
            // Lossless: the vertex count fits in u32 (checked when the mesh is built).
            let mut out: Vec<(usize, EdgeId)> = mesh
                .outgoing(VertexId::new(v as u32))
                .map(|h| (mesh.dest(h).index() as usize, h.edge()))
                .collect();
            out.sort();
            out
        })
        .collect();
    let length = |e: EdgeId| {
        let h = e.half_edge();
        (mesh.position(mesh.dest(h)) - mesh.position(mesh.origin(h))).length()
    };
    // Searched from `to`, so the tree path up from `from` runs in walking order.
    let (goal, start) = (from.index() as usize, to.index() as usize);
    let (tree, hit) = dijkstra::search(&adjacent, length, &[start], |v| v == goal);
    match hit {
        Some(v) => Ok(tree.path_to(v).collect()),
        None => Err(SeamsError::NoPath { from, to }),
    }
}
