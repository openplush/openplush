//! A chart cut open along its seams, as the flat arrays the flattener reads.

use openplush_geom::DVec3;
use openplush_mesh::{FaceId, HalfEdgeId, ManifoldMesh, VertexId};

use crate::chart::{cut_test, sides};
use crate::{Chart, ChartId, SeamSet};

/// A chart cut open along its seams: a mesh vertex is copied once per fan of chart faces between
/// cuts, so a seam through the chart separates its sides while a dart tip stays one vertex.
#[derive(Debug, Clone, PartialEq)]
pub struct ChartMesh {
    chart: ChartId,
    xyz: Vec<DVec3>,
    faces: Vec<(FaceId, [VertexId; 3])>,
    vtx_to_global: Vec<VertexId>,
}

impl ChartMesh {
    /// `chart`, from `charts(mesh, seams)`, cut open along `seams`. Local vertices are numbered in
    /// order of first appearance over the faces in ascending order, corners in each face's order.
    pub fn new(mesh: &ManifoldMesh, seams: &SeamSet, chart: &Chart) -> Self {
        let cut = cut_test(mesh, seams, chart);
        let mut local: Vec<Option<VertexId>> = vec![None; mesh.half_edge_count()];
        let mut vtx_to_global = Vec::new();
        let mut faces = Vec::with_capacity(chart.faces().len());
        for &f in chart.faces() {
            let corners = sides(mesh, f).map(|side| {
                local[side.index() as usize].unwrap_or_else(|| {
                    // Lossless: local vertices are at most the chart's corners, which fit in u32.
                    let l = VertexId::new(vtx_to_global.len() as u32);
                    vtx_to_global.push(mesh.origin(side));
                    for corner in fan(mesh, side, &cut) {
                        local[corner.index() as usize] = Some(l);
                    }
                    l
                })
            });
            faces.push((f, corners));
        }
        Self {
            chart: chart.id(),
            xyz: vtx_to_global.iter().map(|&v| mesh.position(v)).collect(),
            faces,
            vtx_to_global,
        }
    }

    /// The chart it cuts open.
    pub fn chart(&self) -> ChartId {
        self.chart
    }

    /// Each local vertex's model-space position.
    pub fn xyz(&self) -> &[DVec3] {
        &self.xyz
    }

    /// Each chart face, ascending, with its corners as local vertices in the face's order.
    pub fn faces(&self) -> &[(FaceId, [VertexId; 3])] {
        &self.faces
    }

    /// Each local vertex's mesh vertex.
    pub fn vtx_to_global(&self) -> &[VertexId] {
        &self.vtx_to_global
    }
}

/// The corners, as their outgoing half-edges, that share `start`'s origin without a cut between
/// them: the fan is walked both ways about the origin until a cut, or round to `start`.
fn fan(
    mesh: &ManifoldMesh,
    start: HalfEdgeId,
    cut: impl Fn(HalfEdgeId) -> bool,
) -> Vec<HalfEdgeId> {
    let mut corners = vec![start];
    let mut h = start;
    while !cut(h) {
        h = mesh.next(h.twin());
        if h == start {
            return corners;
        }
        corners.push(h);
    }
    let mut h = start;
    while !cut(mesh.prev(h)) {
        h = mesh.prev(h).twin();
        corners.push(h);
    }
    corners
}
