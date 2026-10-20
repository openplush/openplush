//! What the fill reads from the mesh around a loop, on the open cube.

use openplush_geom::DVec3;
use openplush_geom::epsilon::ModelTolerances;

use super::{face_normal, outer_ring};
use crate::{HalfEdgeId, ManifoldMesh, SourceCorner, TriangleSoup, VertexId};

/// A unit cube missing its top, so corners 4 to 7 bound a hole.
fn open_cube() -> ManifoldMesh {
    let p = DVec3::new;
    let positions = vec![
        p(0., 0., 0.),
        p(1., 0., 0.),
        p(1., 1., 0.),
        p(0., 1., 0.),
        p(0., 0., 1.),
        p(1., 0., 1.),
        p(1., 1., 1.),
        p(0., 1., 1.),
    ];
    let triangles = vec![
        [0, 2, 1],
        [0, 3, 2],
        [0, 1, 5],
        [0, 5, 4],
        [1, 2, 6],
        [1, 6, 5],
        [2, 3, 7],
        [2, 7, 6],
        [3, 0, 4],
        [3, 4, 7],
    ];
    let corners = (0..30)
        .map(|vertex| SourceCorner {
            primitive: 0,
            vertex,
        })
        .collect();
    let soup = TriangleSoup::new(positions, triangles, corners, None).unwrap();
    ManifoldMesh::from_oriented_triangles(
        &soup,
        &ModelTolerances::for_diagonal(3f64.sqrt()).unwrap(),
    )
    .unwrap()
}

fn rim_edge(mesh: &ManifoldMesh, from: u32, to: u32) -> HalfEdgeId {
    mesh.boundary_loops()
        .into_iter()
        .flatten()
        .find(|&h| mesh.origin(h) == VertexId::new(from) && mesh.dest(h) == VertexId::new(to))
        .unwrap()
}

#[test]
fn the_normal_across_a_rim_edge_is_the_outward_normal_of_the_face_there() {
    let mesh = open_cube();
    let h = rim_edge(&mesh, 4, 5);
    assert_eq!(face_normal(&mesh, h.twin()), DVec3::new(0.0, -1.0, 0.0));
}

#[test]
fn a_rim_vertex_s_outer_ring_leaves_out_the_loop_s_own_vertices() {
    let mesh = open_cube();
    let rim = [4, 5, 6, 7].map(VertexId::new);
    assert_eq!(
        outer_ring(&mesh, VertexId::new(4), &rim),
        vec![DVec3::new(0.0, 0.0, 0.0), DVec3::new(0.0, 1.0, 0.0)]
    );
}
