//! ManifoldMesh construction (T025).
//!
//! These live inside the crate because `ManifoldMesh` has no public constructor
//! (data-model section 1: only repair may produce one). The builder under test,
//! `ManifoldMesh::from_oriented_triangles`, is what repair (T067) calls once the soup
//! is clean.

use openplush_geom::DVec3;
use openplush_geom::epsilon::ModelTolerances;

use super::{HalfEdgeId, ManifoldMesh};
use crate::{MeshError, SourceCorner, TriangleSoup, VertexId};

fn soup(positions: Vec<DVec3>, triangles: Vec<[u32; 3]>) -> TriangleSoup {
    let corners = (0..triangles.len() * 3)
        .map(|i| SourceCorner {
            primitive: 0,
            vertex: i as u32,
        })
        .collect();
    TriangleSoup::new(positions, triangles, corners, None).unwrap()
}

fn tol(s: &TriangleSoup) -> ModelTolerances {
    ModelTolerances::for_diagonal(s.bbox_diagonal().unwrap()).unwrap()
}

fn build(positions: Vec<DVec3>, triangles: Vec<[u32; 3]>) -> Result<ManifoldMesh, MeshError> {
    let s = soup(positions, triangles);
    ManifoldMesh::from_oriented_triangles(&s, &tol(&s))
}

fn p(x: f64, y: f64, z: f64) -> DVec3 {
    DVec3::new(x, y, z)
}

/// A unit square split into two triangles: 4 vertices, 5 edges, 2 faces, one boundary loop.
fn square() -> (Vec<DVec3>, Vec<[u32; 3]>) {
    (
        vec![p(0., 0., 0.), p(1., 0., 0.), p(1., 1., 0.), p(0., 1., 0.)],
        vec![[0, 1, 2], [0, 2, 3]],
    )
}

/// A closed tetrahedron, outward-facing.
fn tetrahedron() -> (Vec<DVec3>, Vec<[u32; 3]>) {
    (
        vec![p(0., 0., 0.), p(1., 0., 0.), p(0., 1., 0.), p(0., 0., 1.)],
        vec![[0, 2, 1], [0, 1, 3], [1, 2, 3], [0, 3, 2]],
    )
}

#[test]
fn twins_are_paired_indices() {
    let h = HalfEdgeId::new(6);
    assert_eq!(h.twin(), HalfEdgeId::new(7));
    assert_eq!(h.twin().twin(), h);
    assert_eq!(h.edge(), h.twin().edge());
}

#[test]
fn square_counts_and_boundary() {
    let (v, t) = square();
    let m = build(v, t).unwrap();
    assert_eq!(
        (m.vertex_count(), m.edge_count(), m.face_count()),
        (4, 5, 2)
    );
    assert_eq!(m.half_edge_count(), 10);
    assert!((0..4).all(|i| m.is_boundary_vertex(VertexId::new(i))));
    let loops = m.boundary_loops();
    assert_eq!(loops.len(), 1);
    assert_eq!(loops[0].len(), 4);
}

#[test]
fn boundary_vertex_points_at_its_boundary_half_edge() {
    let (v, t) = square();
    let m = build(v, t).unwrap();
    for i in 0..4 {
        let h = m.vertex_out(VertexId::new(i));
        assert_eq!(
            m.face(h),
            None,
            "v_out of a boundary vertex must be the boundary half-edge"
        );
        assert_eq!(m.origin(h), VertexId::new(i));
    }
}

#[test]
fn closed_tetrahedron_has_no_boundary() {
    let (v, t) = tetrahedron();
    let m = build(v, t).unwrap();
    assert_eq!(
        (m.vertex_count(), m.edge_count(), m.face_count()),
        (4, 6, 4)
    );
    assert!(m.boundary_loops().is_empty());
    assert!((0..4).all(|i| !m.is_boundary_vertex(VertexId::new(i))));
}

#[test]
fn one_ring_visits_every_neighbour_once() {
    let (v, t) = tetrahedron();
    let m = build(v, t).unwrap();
    let ring: Vec<VertexId> = m.outgoing(VertexId::new(0)).map(|h| m.dest(h)).collect();
    let mut sorted = ring.clone();
    sorted.sort();
    assert_eq!(
        sorted,
        vec![VertexId::new(1), VertexId::new(2), VertexId::new(3)]
    );
}

#[test]
fn corners_keep_their_provenance() {
    let (v, t) = square();
    let m = build(v, t).unwrap();
    // Triangle 1 is [0, 2, 3]; its corner 1 (vertex 2) was source vertex 4.
    let f = crate::FaceId::new(1);
    let h = m.face_half_edge(f);
    let corners: Vec<(VertexId, Option<SourceCorner>)> = (0..3)
        .scan(h, |h, _| {
            let out = (m.origin(*h), m.corner_source(*h));
            *h = m.next(*h);
            Some(out)
        })
        .collect();
    assert!(corners.contains(&(
        VertexId::new(2),
        Some(SourceCorner {
            primitive: 0,
            vertex: 4
        })
    )));
}

#[test]
fn bowtie_vertex_is_refused() {
    // Two triangles touching only at vertex 0.
    let v = vec![
        p(0., 0., 0.),
        p(1., 0., 0.),
        p(1., 1., 0.),
        p(-1., 0., 0.),
        p(-1., -1., 0.),
    ];
    assert_eq!(
        build(v, vec![[0, 1, 2], [0, 3, 4]]).err(),
        Some(MeshError::NonManifoldVertex {
            vertex: VertexId::new(0)
        })
    );
}

#[test]
fn edge_with_three_faces_is_refused() {
    let v = vec![
        p(0., 0., 0.),
        p(1., 0., 0.),
        p(0., 1., 0.),
        p(0., -1., 0.),
        p(0., 0., 1.),
    ];
    let r = build(v, vec![[0, 1, 2], [1, 0, 3], [0, 1, 4]]);
    assert!(matches!(r, Err(MeshError::NonManifoldEdge { .. })), "{r:?}");
}

#[test]
fn flipped_neighbour_is_refused() {
    let (v, mut t) = square();
    t[1] = [0, 3, 2]; // same winding direction on the shared edge 0-2
    assert!(matches!(
        build(v, t),
        Err(MeshError::InconsistentOrientation { .. })
    ));
}

#[test]
fn degenerate_and_zero_area_triangles_are_refused() {
    let (v, mut t) = square();
    t[0] = [0, 0, 2];
    assert_eq!(
        build(v, t).err(),
        Some(MeshError::DegenerateTriangle {
            triangle: crate::FaceId::new(0)
        })
    );
    let v = vec![p(0., 0., 0.), p(1., 0., 0.), p(2., 0., 0.), p(0., 1., 0.)];
    assert_eq!(
        build(v, vec![[0, 1, 2], [0, 2, 3]]).err(),
        Some(MeshError::ZeroAreaTriangle {
            triangle: crate::FaceId::new(0)
        })
    );
}

#[test]
fn two_closed_fans_sharing_a_vertex_are_refused() {
    // Two tetrahedra touching tip to tip at vertex 0. Every edge has two faces and
    // there is no boundary anywhere, so only the single-fan check can catch it.
    let (mut v, mut t) = tetrahedron();
    v.extend([p(-1., 0., 0.), p(0., -1., 0.), p(0., 0., -1.)]);
    t.extend(
        tetrahedron()
            .1
            .iter()
            .map(|tri| tri.map(|i| if i == 0 { 0 } else { i + 3 })),
    );
    assert_eq!(
        build(v, t).err(),
        Some(MeshError::NonManifoldVertex {
            vertex: VertexId::new(0)
        })
    );
}

#[test]
fn isolated_vertex_is_refused() {
    let (mut v, t) = square();
    v.push(p(5., 5., 5.));
    assert_eq!(
        build(v, t).err(),
        Some(MeshError::IsolatedVertex {
            vertex: VertexId::new(4)
        })
    );
}
