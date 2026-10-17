//! Defect detection on a `TriangleSoup`: reports, never changes, the soup.

mod coincident;
mod degenerate;
mod disjoint;
mod nonmanifold;
mod orientation;
mod report;
mod selfintersect;
mod topology;

use std::collections::BTreeMap;

use openplush_geom::epsilon::ModelTolerances;

pub use report::{
    Duplicate, DuplicateOrientation, NonManifoldEdge, Unmeasurable, ValidationReport,
};

use crate::TriangleSoup;

/// One face's use of an edge: `forward` when it walks the edge from the smaller vertex.
#[derive(Debug, Clone, Copy)]
struct EdgeUse {
    face: u32,
    forward: bool,
}

/// Every edge of the kept faces by its welded ends, smaller first.
type Edges = BTreeMap<[u32; 2], Vec<EdgeUse>>;

/// Locates every defect class. Never fails: a soup
/// with no triangles, or an extent tolerances cannot be derived for, is reported as
/// [`Unmeasurable`] with nothing else judged.
pub fn validate(soup: &TriangleSoup) -> ValidationReport {
    let Some(diagonal) = soup.bbox_diagonal() else {
        return ValidationReport::unmeasured(Unmeasurable::NoTriangles);
    };
    let Ok(tol) = ModelTolerances::for_diagonal(diagonal) else {
        return ValidationReport::unmeasured(Unmeasurable::Extent { diagonal });
    };
    let weld = coincident::weld(soup, tol.weld());
    // INVARIANT: `rep` has one entry per soup vertex, and soup indices are in range.
    let tris: Vec<[u32; 3]> = soup
        .triangle_indices()
        .iter()
        .map(|t| t.map(|v| weld.rep[v as usize]))
        .collect();
    let degenerate = degenerate::degenerate(soup, &tris, &tol);
    let edges = edges(&tris, &degenerate.kept);
    let components = topology::components(soup.vertex_count(), &tris, &degenerate.kept);
    ValidationReport {
        unmeasurable: None,
        unwelded: weld.groups,
        non_manifold_edges: nonmanifold::edges(&edges),
        non_manifold_vertices: nonmanifold::vertices(&edges, &tris, &degenerate.kept),
        winding_conflicts: orientation::conflicts(&edges),
        non_orientable: orientation::non_orientable(&edges, &tris, &degenerate.kept),
        inside_out: orientation::inside_out(
            &edges,
            &tris,
            &degenerate.kept,
            soup.positions(),
            diagonal,
        ),
        boundary_loops: topology::boundary_loops(&edges),
        self_intersections: selfintersect::crossings(
            &tris,
            &degenerate.kept,
            soup.positions(),
            &tol,
        ),
        components,
        zero_area: degenerate.zero_area,
        repeated_index: degenerate.repeated_index,
        duplicates: degenerate.duplicates,
    }
}

fn edges(tris: &[[u32; 3]], kept: &[u32]) -> Edges {
    let mut edges = Edges::new();
    for &face in kept {
        // INVARIANT: kept faces index `tris`.
        let [a, b, c] = tris[face as usize];
        for [x, y] in [[a, b], [b, c], [c, a]] {
            let (key, forward) = if x < y {
                ([x, y], true)
            } else {
                ([y, x], false)
            };
            edges
                .entry(key)
                .or_default()
                .push(EdgeUse { face, forward });
        }
    }
    edges
}
