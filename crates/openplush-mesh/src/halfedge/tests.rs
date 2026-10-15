//! ManifoldMesh construction and invariants.
//!
//! These live inside the crate because `ManifoldMesh` has no public constructor
//! (only repair may produce one). The builder under test,
//! `ManifoldMesh::from_oriented_triangles`, is what repair calls once the soup
//! is clean.

use openplush_geom::epsilon::ModelTolerances;
use openplush_geom::{DVec2, DVec3};

use super::{HalfEdgeId, ManifoldMesh};
use crate::{MeshError, SourceCorner, TriangleSoup, VertexId};

pub(super) fn soup(positions: Vec<DVec3>, triangles: Vec<[u32; 3]>) -> TriangleSoup {
    let corners = (0..triangles.len() * 3)
        .map(|i| SourceCorner {
            primitive: 0,
            vertex: i as u32,
        })
        .collect();
    TriangleSoup::new(positions, triangles, corners, None).unwrap()
}

pub(super) fn tol(s: &TriangleSoup) -> ModelTolerances {
    ModelTolerances::for_diagonal(s.bbox_diagonal().unwrap()).unwrap()
}

fn build(positions: Vec<DVec3>, triangles: Vec<[u32; 3]>) -> Result<ManifoldMesh, MeshError> {
    let s = soup(positions, triangles);
    ManifoldMesh::from_oriented_triangles(&s, &tol(&s))
}

pub(super) fn p(x: f64, y: f64, z: f64) -> DVec3 {
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
    m.debug_validate(&tol(&soup(square().0, square().1)))
        .unwrap();
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
    let m = build(v.clone(), t.clone()).unwrap();
    assert_eq!(
        (m.vertex_count(), m.edge_count(), m.face_count()),
        (4, 6, 4)
    );
    assert!(m.boundary_loops().is_empty());
    assert!((0..4).all(|i| !m.is_boundary_vertex(VertexId::new(i))));
    m.debug_validate(&tol(&soup(v, t))).unwrap();
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

#[test]
fn debug_validate_catches_a_corrupted_mesh() {
    let (v, t) = square();
    let s = soup(v, t);
    let mut m = ManifoldMesh::from_oriented_triangles(&s, &tol(&s)).unwrap();
    m.corrupt_next_for_test(HalfEdgeId::new(0));
    assert!(m.debug_validate(&tol(&s)).is_err());
}

// ---- Review of #40 --------------------------------------------------------

#[test]
fn a_sub_tolerance_sliver_is_refused_at_any_scale() {
    // At a diagonal near 1e90 the area tolerance is ~2e168, but |e1 x e2|^2
    // overflows to inf, which once compared as "inf > tolerance" and let this
    // 1e160-area triangle through.
    let v = vec![
        p(0., 0., 0.),
        p(1e90, 0., 0.),
        p(0., 1e90, 0.),
        p(0., 2e70, 0.),
    ];
    assert_eq!(
        build(v, vec![[0, 1, 2], [1, 0, 3]]).err(),
        Some(MeshError::ZeroAreaTriangle {
            triangle: crate::FaceId::new(1)
        })
    );
}

#[test]
fn the_area_threshold_is_half_the_cross_product_against_the_tolerance() {
    // A sliver hanging off a unit square's bottom edge: area h/2 against a
    // tolerance of about 1e-12 * 2 = 2e-12, so the threshold is h = 4e-12.
    let with_sliver = |h: f64| {
        let (mut v, mut t) = square();
        v.push(p(0.5, -h, 0.));
        t.push([1, 0, 4]);
        build(v, t)
    };
    assert!(with_sliver(5e-12).is_ok());
    assert_eq!(
        with_sliver(3e-12).err(),
        Some(MeshError::ZeroAreaTriangle {
            triangle: crate::FaceId::new(2)
        })
    );
}

#[test]
fn duplicate_triangles_are_refused_as_duplicates() {
    // Same vertices in either winding: a coincident "pillow" is not a surface a
    // pattern can come from, and the orientation error would mislead.
    let v = vec![p(0., 0., 0.), p(1., 0., 0.), p(0., 1., 0.)];
    for second in [[0, 2, 1], [1, 2, 0], [0, 1, 2]] {
        assert_eq!(
            build(v.clone(), vec![[0, 1, 2], second]).err(),
            Some(MeshError::DuplicateTriangle {
                triangle: crate::FaceId::new(1),
                first: crate::FaceId::new(0),
            }),
            "{second:?}"
        );
    }
}

#[test]
fn outgoing_rotates_counter_clockwise_from_vertex_out() {
    // A fan of four counter-clockwise triangles around vertex 4.
    let v = vec![
        p(1., 0., 0.),
        p(0., 1., 0.),
        p(-1., 0., 0.),
        p(0., -1., 0.),
        p(0., 0., 0.),
    ];
    let m = build(v, vec![[4, 0, 1], [4, 1, 2], [4, 2, 3], [4, 3, 0]]).unwrap();
    let ring: Vec<u32> = m
        .outgoing(VertexId::new(4))
        .map(|h| m.dest(h).index())
        .collect();
    assert_eq!(ring, vec![0, 1, 2, 3]);
    // At a boundary vertex the walk starts on the boundary half-edge, then sweeps
    // counter-clockwise: vertex 0 of the square sees 3 (boundary), then 1, then 2.
    let (v, t) = square();
    let m = build(v, t).unwrap();
    let ring: Vec<u32> = m
        .outgoing(VertexId::new(0))
        .map(|h| m.dest(h).index())
        .collect();
    assert_eq!(ring, vec![3, 1, 2]);
}

#[test]
fn face_half_edge_leaves_the_first_corner() {
    let (v, t) = square();
    let m = build(v, t.clone()).unwrap();
    for (f, tri) in t.iter().enumerate() {
        let h = m.face_half_edge(crate::FaceId::new(f as u32));
        assert_eq!(m.origin(h), VertexId::new(tri[0]));
        assert_eq!(m.dest(h), VertexId::new(tri[1]));
    }
}

#[test]
fn corner_uvs_survive_the_build() {
    let (v, t) = square();
    let corners = (0..6)
        .map(|i| SourceCorner {
            primitive: 0,
            vertex: i,
        })
        .collect();
    let uvs = (0..6)
        .map(|i| (i != 4).then(|| DVec2::new(f64::from(i), 0.5)))
        .collect();
    let s = TriangleSoup::new(v, t, corners, Some(uvs)).unwrap();
    let m = ManifoldMesh::from_oriented_triangles(&s, &tol(&s)).unwrap();
    for f in 0..2u32 {
        let mut h = m.face_half_edge(crate::FaceId::new(f));
        for corner in 0..3 {
            let i = f * 3 + corner;
            let want = (i != 4).then(|| DVec2::new(f64::from(i), 0.5));
            assert_eq!(m.corner_uv(h), want, "face {f} corner {corner}");
            h = m.next(h);
        }
    }
}

#[test]
fn an_interior_vertex_points_at_its_lowest_outgoing_half_edge() {
    let (v, t) = tetrahedron();
    let m = build(v, t).unwrap();
    for i in 0..4 {
        let lowest = m.outgoing(VertexId::new(i)).min().unwrap();
        assert_eq!(m.vertex_out(VertexId::new(i)), lowest);
    }
}

#[test]
fn the_lowest_non_manifold_vertex_is_reported() {
    // Bowties at vertex 0 (triangles A and B) and at vertex 3 (B and C).
    let v = vec![
        p(0., 0., 0.),
        p(1., 0., 0.),
        p(1., 1., 0.),
        p(-1., 0., 0.),
        p(-1., -1., 0.),
        p(-2., 1., 0.),
        p(-3., 0., 0.),
    ];
    assert_eq!(
        build(v, vec![[0, 1, 2], [0, 3, 4], [3, 5, 6]]).err(),
        Some(MeshError::NonManifoldVertex {
            vertex: VertexId::new(0)
        })
    );
}

#[test]
fn a_disk_plus_a_separate_half_disk_at_one_vertex_is_refused() {
    let (mut v, mut t) = tetrahedron();
    v.extend([p(-1., -1., -1.), p(-1., -2., -1.)]);
    t.push([0, 4, 5]);
    assert_eq!(
        build(v, t).err(),
        Some(MeshError::NonManifoldVertex {
            vertex: VertexId::new(0)
        })
    );
}

// ---- Review of #41: one corruption per check, exact location ---------------

use crate::MeshElement;

fn broken(invariant: u8, element: MeshElement) -> Option<MeshError> {
    Some(MeshError::BrokenInvariant { invariant, element })
}

fn built_square() -> (ManifoldMesh, ModelTolerances) {
    let (v, t) = square();
    let s = soup(v, t);
    let t = tol(&s);
    (ManifoldMesh::from_oriented_triangles(&s, &t).unwrap(), t)
}

#[test]
fn invariant_0_a_short_array_is_named() {
    let (mut m, t) = built_square();
    m.he_prev.pop();
    assert_eq!(
        m.debug_validate(&t).err(),
        broken(0, MeshElement::Storage("he_prev"))
    );
    let (mut m, t) = built_square();
    m.v_out.pop();
    assert_eq!(
        m.debug_validate(&t).err(),
        broken(0, MeshElement::Storage("v_out"))
    );
}

#[test]
fn invariant_0_an_out_of_range_link_is_located() {
    let (mut m, t) = built_square();
    m.he_next[3] = HalfEdgeId::new(999);
    assert_eq!(
        m.debug_validate(&t).err(),
        broken(0, MeshElement::HalfEdge(HalfEdgeId::new(3)))
    );
}

#[test]
fn invariant_0_the_boundary_vertex_rule() {
    let (mut m, t) = built_square();
    let interior = m
        .outgoing(VertexId::new(0))
        .find(|&h| m.face(h).is_some())
        .unwrap();
    m.v_out[0] = interior;
    assert_eq!(
        m.debug_validate(&t).err(),
        broken(0, MeshElement::Vertex(VertexId::new(0)))
    );
}

#[test]
fn invariant_1_next_of_prev() {
    let (mut m, t) = built_square();
    let h = HalfEdgeId::new(0);
    m.corrupt_next_for_test(h);
    // next(h) now skips a corner, so h no longer ends where next(h) starts. h is
    // the lowest index involved, so it is the first violation reported.
    assert_eq!(
        m.debug_validate(&t).err(),
        broken(1, MeshElement::HalfEdge(h))
    );
}

#[test]
fn invariant_2_every_face_loop_has_length_three() {
    // Label the square's boundary loop (length 4) as face 0: every link stays
    // consistent, so only the loop-length check can see it.
    let (mut m, t) = built_square();
    let boundary: Vec<usize> = (0..m.half_edge_count())
        .filter(|&i| m.he_face[i].is_none())
        .collect();
    for &i in &boundary {
        m.he_face[i] = Some(crate::FaceId::new(0));
    }
    let lowest = HalfEdgeId::new(boundary[0] as u32);
    assert_eq!(
        m.debug_validate(&t).err(),
        broken(2, MeshElement::HalfEdge(lowest))
    );
}

fn hs(ix: &[u32]) -> Vec<HalfEdgeId> {
    ix.iter().copied().map(HalfEdgeId::new).collect()
}

fn vs(ix: &[u32]) -> Vec<VertexId> {
    ix.iter().copied().map(VertexId::new).collect()
}

fn fs(ix: &[Option<u32>]) -> Vec<Option<crate::FaceId>> {
    ix.iter().map(|f| f.map(crate::FaceId::new)).collect()
}

/// Triangles (0,1,2) and (0,3,4) share only vertex 0. Linked as a single boundary
/// loop, every link checks out and the rotation at vertex 0 visits all four of its
/// half-edges, so only "one outgoing boundary half-edge per vertex" catches it.
fn linked_bowtie() -> ManifoldMesh {
    let (f0, f1) = (Some(0), Some(1));
    ManifoldMesh {
        pos: vec![
            p(0., 0., 0.),
            p(1., 0., 0.),
            p(1., 1., 0.),
            p(-1., 0., 0.),
            p(-1., -1., 0.),
        ],
        v_out: hs(&[5, 1, 3, 7, 9]),
        // Edges 0-1, 1-2, 2-0 (face 0) and 0-3, 3-4, 4-0 (face 1); odd = boundary.
        he_origin: vs(&[0, 1, 1, 2, 2, 0, 0, 3, 3, 4, 4, 0]),
        he_face: fs(&[f0, None, f0, None, f0, None, f1, None, f1, None, f1, None]),
        he_next: hs(&[2, 11, 4, 1, 0, 3, 8, 5, 10, 7, 6, 9]),
        he_prev: hs(&[4, 3, 0, 5, 2, 7, 10, 9, 6, 11, 8, 1]),
        f_he: hs(&[0, 6]),
        corner_src: vec![None; 12],
        corner_uv: vec![None; 12],
    }
}

fn bowtie_tolerances(m: &ManifoldMesh) -> ModelTolerances {
    tol(&soup(m.pos.clone(), vec![[0, 1, 2], [0, 3, 4]]))
}

#[test]
fn invariant_3_a_bowtie_with_one_linked_boundary_loop() {
    let m = linked_bowtie();
    assert_eq!(
        m.debug_validate(&bowtie_tolerances(&m)).err(),
        broken(3, MeshElement::Vertex(VertexId::new(0)))
    );
}

#[test]
fn invariant_5_a_face_collapsed_by_moving_a_vertex() {
    let (mut m, t) = built_square();
    m.pos[2] = m.pos[0];
    assert_eq!(
        m.debug_validate(&t).err(),
        broken(5, MeshElement::Face(crate::FaceId::new(0)))
    );
}

#[test]
fn invariant_0_an_odd_half_edge_count() {
    let (mut m, t) = built_square();
    let h = HalfEdgeId::new(0);
    m.he_next.push(h);
    m.he_prev.push(h);
    m.he_origin.push(VertexId::new(0));
    m.he_face.push(None);
    m.corner_src.push(None);
    m.corner_uv.push(None);
    assert_eq!(
        m.debug_validate(&t).err(),
        broken(0, MeshElement::Storage("he_next"))
    );
}

#[test]
fn invariant_0_face_half_edge_outside_its_face() {
    let (mut m, t) = built_square();
    m.f_he[1] = m.f_he[0];
    assert_eq!(
        m.debug_validate(&t).err(),
        broken(0, MeshElement::Face(crate::FaceId::new(1)))
    );
}

/// Appends three edges forming a second, consistently linked triangle 0-1-2 that
/// also claims face 0. Every link and loop length checks out; only the count
/// (three faced half-edges per face) sees the extra loop.
fn push_stray_triangle(m: &mut ManifoldMesh) {
    let base = m.half_edge_count() as u32;
    let h = |k: u32| HalfEdgeId::new(base + k);
    let v = VertexId::new;
    // Edge k: interior half-edge base+2k (face 0), boundary twin base+2k+1.
    // Interior loop 0->1->2->0; boundary loop of the twins 1->0, 0->2, 2->1.
    let origins = [v(0), v(1), v(1), v(2), v(2), v(0)];
    let nexts = [h(2), h(5), h(4), h(1), h(0), h(3)];
    let prevs = [h(4), h(3), h(0), h(5), h(2), h(1)];
    for k in 0..6 {
        m.he_origin.push(origins[k]);
        m.he_next.push(nexts[k]);
        m.he_prev.push(prevs[k]);
        m.he_face.push((k % 2 == 0).then(|| crate::FaceId::new(0)));
        m.corner_src.push(None);
        m.corner_uv.push(None);
    }
}

#[test]
fn invariant_2_a_stray_triangle_loop_reusing_a_face_label() {
    let (mut m, t) = built_square();
    push_stray_triangle(&mut m);
    assert_eq!(
        m.debug_validate(&t).err(),
        broken(2, MeshElement::Storage("he_face"))
    );
}

/// A "triangle" 0->1->0->0 whose loop runs along edge 0-1 and back, closed by a
/// self-loop edge at vertex 0. Links, loop length and counts are consistent; the
/// edge 0-1 has face 0 on both sides.
fn one_face_on_both_sides() -> ManifoldMesh {
    ManifoldMesh {
        pos: vec![p(0., 0., 0.), p(1., 0., 0.)],
        v_out: hs(&[3, 1]),
        he_origin: vs(&[0, 1, 0, 0]),
        he_face: fs(&[Some(0), Some(0), Some(0), None]),
        he_next: hs(&[1, 2, 0, 3]),
        he_prev: hs(&[2, 0, 1, 3]),
        f_he: hs(&[0]),
        corner_src: vec![None; 4],
        corner_uv: vec![None; 4],
    }
}

#[test]
fn invariant_4_both_sides_of_an_edge_in_one_face() {
    let m = one_face_on_both_sides();
    let t = ModelTolerances::for_diagonal(1.0).unwrap();
    assert_eq!(
        m.debug_validate(&t).err(),
        broken(4, MeshElement::HalfEdge(HalfEdgeId::new(0)))
    );
}

// ---- Two violations at once: the earlier check's error is reported ----------

#[test]
fn order_an_odd_half_edge_count_before_a_short_array() {
    let (mut m, t) = built_square();
    let h = HalfEdgeId::new(0);
    m.he_next.push(h);
    m.he_prev.push(h);
    m.he_origin.push(VertexId::new(0));
    m.he_face.push(None);
    m.corner_src.push(None);
    m.corner_uv.push(None);
    m.v_out.pop();
    assert_eq!(
        m.debug_validate(&t).err(),
        broken(0, MeshElement::Storage("he_next"))
    );
}

#[test]
fn order_a_short_array_before_an_out_of_range_link() {
    let (mut m, t) = built_square();
    m.he_next[3] = HalfEdgeId::new(999);
    m.he_prev.pop();
    assert_eq!(
        m.debug_validate(&t).err(),
        broken(0, MeshElement::Storage("he_prev"))
    );
}

#[test]
fn order_an_out_of_range_link_before_a_misplaced_face_half_edge() {
    let (mut m, t) = built_square();
    m.f_he[1] = m.f_he[0];
    m.he_next[3] = HalfEdgeId::new(999);
    assert_eq!(
        m.debug_validate(&t).err(),
        broken(0, MeshElement::HalfEdge(HalfEdgeId::new(3)))
    );
}

#[test]
fn order_a_misplaced_face_half_edge_before_invariant_1() {
    let (mut m, t) = built_square();
    m.corrupt_next_for_test(HalfEdgeId::new(0));
    m.f_he[1] = m.f_he[0];
    assert_eq!(
        m.debug_validate(&t).err(),
        broken(0, MeshElement::Face(crate::FaceId::new(1)))
    );
}

#[test]
fn order_invariant_1_before_invariant_2() {
    // The invariant 1 break sits at a higher half-edge than the invariant 2 one,
    // so a single pass over half-edges would report invariant 2 instead.
    let (mut m, t) = built_square();
    let boundary: Vec<usize> = (0..m.half_edge_count())
        .filter(|&i| m.he_face[i].is_none())
        .collect();
    for &i in &boundary {
        m.he_face[i] = Some(crate::FaceId::new(0));
    }
    let late = m.next(m.face_half_edge(crate::FaceId::new(1)));
    assert!(late.index() as usize > boundary[0]);
    m.corrupt_next_for_test(late);
    assert_eq!(
        m.debug_validate(&t).err(),
        broken(1, MeshElement::HalfEdge(late))
    );
}

#[test]
fn order_invariant_2_before_invariant_4() {
    let mut m = one_face_on_both_sides();
    m.pos.push(p(0., 1., 0.));
    m.v_out.push(HalfEdgeId::new(0));
    push_stray_triangle(&mut m);
    let t = ModelTolerances::for_diagonal(1.0).unwrap();
    assert_eq!(
        m.debug_validate(&t).err(),
        broken(2, MeshElement::Storage("he_face"))
    );
}

#[test]
fn order_invariant_4_before_invariant_5() {
    // A face holding both sides of an edge visits at most two distinct vertices,
    // so it is always a zero-area sliver too: one corruption breaks both.
    let m = one_face_on_both_sides();
    let t = ModelTolerances::for_diagonal(1.0).unwrap();
    let (a, b) = (m.pos[0], m.pos[1]);
    assert!(super::is_sliver(a, b, a, &t));
    assert_eq!(
        m.debug_validate(&t).err(),
        broken(4, MeshElement::HalfEdge(HalfEdgeId::new(0)))
    );
}

#[test]
fn order_invariant_5_before_the_boundary_vertex_rule() {
    let (mut m, t) = built_square();
    let interior = m
        .outgoing(VertexId::new(0))
        .find(|&h| m.face(h).is_some())
        .unwrap();
    m.v_out[0] = interior;
    m.pos[2] = m.pos[0];
    assert_eq!(
        m.debug_validate(&t).err(),
        broken(5, MeshElement::Face(crate::FaceId::new(0)))
    );
}

#[test]
fn order_the_boundary_vertex_rule_before_invariant_3_at_one_vertex() {
    let mut m = linked_bowtie();
    m.v_out[0] = HalfEdgeId::new(0);
    assert_eq!(
        m.debug_validate(&bowtie_tolerances(&m)).err(),
        broken(0, MeshElement::Vertex(VertexId::new(0)))
    );
}

#[test]
fn order_vertex_checks_run_vertex_by_vertex() {
    // Invariant 3 at vertex 0 wins over the boundary-vertex rule at vertex 1.
    let mut m = linked_bowtie();
    m.v_out[1] = HalfEdgeId::new(2);
    assert_eq!(
        m.debug_validate(&bowtie_tolerances(&m)).err(),
        broken(3, MeshElement::Vertex(VertexId::new(0)))
    );
}
