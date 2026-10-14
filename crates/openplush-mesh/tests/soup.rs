//! `TriangleSoup` (T024): the import stage's output. It guarantees finite coordinates,
//! in-range indices, and one provenance record per corner (data-model section 1),
//! and it carries defects through untouched: reporting them is validation's job.

use openplush_geom::{DVec2, DVec3};
use openplush_mesh::{MeshError, SourceCorner, TriangleSoup, VertexId};

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
        soup.triangle(1),
        Some([VertexId::new(0), VertexId::new(2), VertexId::new(3)])
    );
    assert_eq!(soup.triangle(2), None);
    assert_eq!(
        soup.position(VertexId::new(2)),
        Some(DVec3::new(1.0, 1.0, 0.0))
    );
    assert_eq!(soup.corner_source(1, 2), Some(corner(0, 3)));
    assert_eq!(soup.corner_uv(1, 2), None, "no UVs supplied");
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
    assert_eq!(soup.corner_uv(0, 0), Some(DVec2::new(0.0, 0.0)));
    assert_eq!(soup.corner_uv(1, 0), Some(DVec2::new(0.5, 0.0)));
    assert_eq!(soup.corner_uv(1, 1), None, "a corner may lack a UV");
    Ok(())
}

#[test]
fn rejects_non_finite_coordinates_naming_the_vertex() {
    let (mut p, t, c) = quad();
    p[2].y = f64::NAN;
    assert_eq!(
        TriangleSoup::new(p, t, c, None).err(),
        Some(MeshError::NonFiniteVertex {
            vertex: VertexId::new(2)
        })
    );
}

#[test]
fn rejects_out_of_range_indices_naming_the_triangle() {
    let (p, mut t, c) = quad();
    t[1] = [0, 2, 9];
    assert_eq!(
        TriangleSoup::new(p, t, c, None).err(),
        Some(MeshError::IndexOutOfRange {
            triangle: 1,
            index: 9,
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
            expected: 6,
            found: 5
        })
    );
    let (_, _, c) = quad();
    let uv = vec![None; 4];
    assert_eq!(
        TriangleSoup::new(p, t, c, Some(uv)).err(),
        Some(MeshError::CornerCountMismatch {
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
        soup.triangle(0),
        Some([VertexId::new(0), VertexId::new(0), VertexId::new(1)])
    );
    Ok(())
}

#[test]
fn empty_soup_is_valid() -> Result<(), MeshError> {
    let soup = TriangleSoup::new(vec![], vec![], vec![], None)?;
    assert_eq!(soup.triangle_count(), 0);
    Ok(())
}

#[test]
fn bounding_box_diagonal() -> Result<(), MeshError> {
    let (p, t, c) = quad();
    let soup = TriangleSoup::new(p, t, c, None)?;
    assert_eq!(soup.bbox_diagonal(), Some(2f64.sqrt()));
    assert_eq!(
        TriangleSoup::new(vec![], vec![], vec![], None)?.bbox_diagonal(),
        None
    );
    Ok(())
}
