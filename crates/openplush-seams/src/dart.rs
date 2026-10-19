//! Darts: a seam from a seam or the mesh boundary to a tip inside a piece, a slit rather than a
//! separation.

use openplush_mesh::{EdgeId, ManifoldMesh, VertexId};

use crate::{SeamKind, SeamSet, SeamsError};

/// Marks the walk `edges` a dart in `seams`, or changes nothing. The walk's base is whichever end
/// is anchored (on a seam or the mesh boundary); its other vertices must touch neither, so the
/// dart ends inside the piece and never splits it. Either direction of drawing is accepted.
///
/// # Errors
/// `EmptyDart`; `UnknownEdge`; `NotAWalk`, naming the first edge that does not continue from the
/// one before or revisits a vertex; `DartNotAnchored`, naming the walk's first vertex as drawn
/// (a single edge's lower end), when neither end is anchored; `DartTouchesSeam`, naming the first
/// inner vertex or the tip that is.
pub fn mark_dart(
    mesh: &ManifoldMesh,
    seams: &mut SeamSet,
    edges: &[EdgeId],
) -> Result<(), SeamsError> {
    let (&first, rest) = edges.split_first().ok_or(SeamsError::EmptyDart)?;
    if let Some(&edge) = edges
        .iter()
        .find(|e| e.index() as usize >= mesh.edge_count())
    {
        return Err(SeamsError::UnknownEdge { edge });
    }
    let anchored = |v: VertexId| {
        mesh.is_boundary_vertex(v) || mesh.outgoing(v).any(|h| seams.get(h.edge()).is_some())
    };
    let mut vertices = walk(mesh, first, rest)?;
    // Drawn from the tip outward is the same dart.
    if !anchored(vertices[0]) && anchored(vertices[vertices.len() - 1]) {
        vertices.reverse();
    }
    let (base, inner) = (vertices[0], &vertices[1..]);
    if !anchored(base) {
        return Err(SeamsError::DartNotAnchored { vertex: base });
    }
    if let Some(&vertex) = inner.iter().find(|&&v| anchored(v)) {
        return Err(SeamsError::DartTouchesSeam { vertex });
    }
    for &e in edges {
        seams.insert(e, SeamKind::Dart)?;
    }
    Ok(())
}

/// The vertices of the walk `first` then `rest`, as drawn: from the end its first edge does not
/// share with the next, or, for a single edge, from its lower end.
fn walk(mesh: &ManifoldMesh, first: EdgeId, rest: &[EdgeId]) -> Result<Vec<VertexId>, SeamsError> {
    let ends = |e: EdgeId| {
        let h = e.half_edge();
        [mesh.origin(h), mesh.dest(h)]
    };
    let [a, b] = ends(first);
    let start = match rest.first() {
        None => a.min(b),
        Some(&next) if ends(next).contains(&a) => b,
        Some(_) => a,
    };
    let mut vertices = vec![start, if start == a { b } else { a }];
    for &e in rest {
        let at = vertices[vertices.len() - 1];
        let [p, q] = ends(e);
        let to = if p == at {
            q
        } else if q == at {
            p
        } else {
            return Err(SeamsError::NotAWalk { edge: e });
        };
        if vertices.contains(&to) {
            return Err(SeamsError::NotAWalk { edge: e });
        }
        vertices.push(to);
    }
    Ok(vertices)
}
