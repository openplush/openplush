//! `TriangleSoup` (T024): the import stage's output. It guarantees finite coordinates,
//! in-range indices, and one provenance record per corner (data-model section 1),
//! and it carries defects through untouched: reporting them is validation's job.

use openplush_geom::{DVec2, DVec3};
use openplush_mesh::{CornerArray, FaceId, MeshError, SourceCorner, TriangleSoup, VertexId};

fn corner(primitive: u32, vertex: u32) -> SourceCorner {
    SourceCorner { primitive, vertex }
}

fn quad() -> (Vec<DVec3>, Vec<[u32; 3]>, Vec<SourceCorner>) {
    let p = vec![
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(1.0, 0.0, 0.0),
        DVec3::new(1.0, 1.0, 0.0),
        DVec3::new(0.0, 1.0, 0.0),
    ];
    let t = vec![[0, 1, 2], [0, 2, 3]];
    let c = vec![
        corner(0, 0),
        corner(0, 1),
        corner(0, 2),
        corner(0, 0),
        corner(0, 2),
        corner(0, 3),
    ];
    (p, t, c)
}

#[test]
fn builds_and_exposes_triangles_and_provenance() -> Result<(), MeshError> {
    let (p, t, c) = quad();
    let soup = TriangleSoup::new(p, t, c, None)?;
    assert_eq!(soup.vertex_count(), 4);
    assert_eq!(soup.triangle_count(), 2);
    assert_eq!(
        soup.triangle(FaceId::new(1)),
        Some([VertexId::new(0), VertexId::new(2), VertexId::new(3)])
    );
    assert_eq!(soup.triangle(FaceId::new(2)), None);
    assert_eq!(
        soup.position(VertexId::new(2)),
        Some(DVec3::new(1.0, 1.0, 0.0))
    );
    assert_eq!(soup.corner_source(FaceId::new(1), 2), Some(corner(0, 3)));
    assert_eq!(soup.corner_uv(FaceId::new(1), 2), None, "no UVs supplied");
    Ok(())
}

#[test]
fn keeps_per_corner_uvs() -> Result<(), MeshError> {
    let (p, t, c) = quad();
    // A UV seam: vertex 0 has a different UV in each triangle. Per-corner storage keeps both.
    let uv = vec![
        Some(DVec2::new(0.0, 0.0)),
        Some(DVec2::new(1.0, 0.0)),
        Some(DVec2::new(1.0, 1.0)),
        Some(DVec2::new(0.5, 0.0)),
        None,
        Some(DVec2::new(0.0, 1.0)),
    ];
    let soup = TriangleSoup::new(p, t, c, Some(uv))?;
    assert_eq!(
        soup.corner_uv(FaceId::new(0), 0),
        Some(DVec2::new(0.0, 0.0))
    );
    assert_eq!(
        soup.corner_uv(FaceId::new(1), 0),
        Some(DVec2::new(0.5, 0.0))
    );
    assert_eq!(
        soup.corner_uv(FaceId::new(1), 1),
        None,
        "a corner may lack a UV"
    );
    Ok(())
}

#[test]
fn rejects_non_finite_coordinates_naming_the_vertex() {
    for (vertex, bad) in [(2, f64::NAN), (1, f64::INFINITY), (3, f64::NEG_INFINITY)] {
        let (mut p, t, c) = quad();
        p[vertex].y = bad;
        assert_eq!(
            TriangleSoup::new(p, t, c, None).err(),
            Some(MeshError::NonFiniteVertex {
                vertex: VertexId::new(vertex as u32)
            }),
            "{bad} at vertex {vertex}"
        );
    }
}

#[test]
fn rejects_out_of_range_indices_naming_the_triangle() {
    let (p, mut t, c) = quad();
    // Index 4 == vertex_count: the first out-of-range value.
    t[1] = [0, 2, 4];
    assert_eq!(
        TriangleSoup::new(p, t, c, None).err(),
        Some(MeshError::IndexOutOfRange {
            triangle: FaceId::new(1),
            index: 4,
            vertex_count: 4
        })
    );
}

#[test]
fn rejects_corner_arrays_of_the_wrong_length() {
    let (p, t, mut c) = quad();
    c.pop();
    assert_eq!(
        TriangleSoup::new(p.clone(), t.clone(), c, None).err(),
        Some(MeshError::CornerCountMismatch {
            array: CornerArray::Sources,
            expected: 6,
            found: 5
        })
    );
    let (_, _, c) = quad();
    let uv = vec![None; 4];
    assert_eq!(
        TriangleSoup::new(p, t, c, Some(uv)).err(),
        Some(MeshError::CornerCountMismatch {
            array: CornerArray::Uvs,
            expected: 6,
            found: 4
        })
    );
}

#[test]
fn carries_defects_through_for_validation_to_report() -> Result<(), MeshError> {
    // A repeated index (a degenerate triangle) is a defect, not a structural error.
    let (p, mut t, c) = quad();
    t[0] = [0, 0, 1];
    let soup = TriangleSoup::new(p, t, c, None)?;
    assert_eq!(
        soup.triangle(FaceId::new(0)),
        Some([VertexId::new(0), VertexId::new(0), VertexId::new(1)])
    );
    Ok(())
}

#[test]
fn empty_soup_is_valid() -> Result<(), MeshError> {
    let soup = TriangleSoup::new(vec![], vec![], vec![], None)?;
    assert_eq!(soup.triangle_count(), 0);
    assert_eq!(soup.bbox_diagonal(), None);
    Ok(())
}

#[test]
fn bounding_box_diagonal() -> Result<(), MeshError> {
    // A box from (0,0,0) to (1,2,2): every axis contributes, diagonal 3.
    let p = vec![
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(1.0, 0.0, 0.0),
        DVec3::new(0.0, 2.0, 0.0),
        DVec3::new(0.0, 0.0, 2.0),
    ];
    let t = vec![[0, 1, 2], [0, 2, 3]];
    let c = vec![corner(0, 0); 6];
    let soup = TriangleSoup::new(p.clone(), t.clone(), c.clone(), None)?;
    assert_eq!(soup.bbox_diagonal(), Some(3.0));
    // An unused vertex far away does not inflate the tolerances derived from it.
    let mut stray = p;
    stray.push(DVec3::new(1e6, 1e6, 1e6));
    assert_eq!(
        TriangleSoup::new(stray, t, c, None)?.bbox_diagonal(),
        Some(3.0)
    );
    assert_eq!(
        TriangleSoup::new(vec![], vec![], vec![], None)?.bbox_diagonal(),
        None
    );
    Ok(())
}

#[test]
fn far_out_of_range_faces_are_none_not_an_overflow() -> Result<(), MeshError> {
    // On wasm32 usize is 32 bits, so `face * 3` must not be computed unchecked.
    let (p, t, c) = quad();
    let uv = Some(vec![None; c.len()]);
    let soup = TriangleSoup::new(p, t, c, uv)?;
    let far = FaceId::new(u32::MAX);
    assert_eq!(soup.triangle(far), None);
    assert_eq!(soup.corner_source(far, 2), None);
    assert_eq!(soup.corner_uv(far, 2), None);
    Ok(())
}

#[test]
fn corner_index_three_is_out_of_range() -> Result<(), MeshError> {
    // Not a back door into the next triangle's corner 0.
    let (p, t, c) = quad();
    let uv = Some(vec![Some(DVec2::new(0.0, 0.0)); c.len()]);
    let soup = TriangleSoup::new(p, t, c, uv)?;
    assert_eq!(soup.corner_source(FaceId::new(0), 3), None);
    assert_eq!(soup.corner_uv(FaceId::new(0), 3), None);
    Ok(())
}

#[test]
fn errors_are_reported_in_a_documented_order() {
    // Corner-array lengths, then coordinates, then indices: with every defect at
    // once, the length mismatch wins, and so on down the list.
    let (mut p, mut t, mut c) = quad();
    p[0].x = f64::NAN;
    t[0] = [0, 1, 9];
    c.pop();
    assert!(matches!(
        TriangleSoup::new(p.clone(), t.clone(), c, None),
        Err(MeshError::CornerCountMismatch { .. })
    ));
    let (_, _, c) = quad();
    assert!(matches!(
        TriangleSoup::new(p, t.clone(), c.clone(), None),
        Err(MeshError::NonFiniteVertex { .. })
    ));
    let (p, _, _) = quad();
    assert!(matches!(
        TriangleSoup::new(p, t, c, None),
        Err(MeshError::IndexOutOfRange { .. })
    ));
}
