//! The crate-private rebuilders keep the file-level fields.

use openplush_geom::DVec3;

use crate::{MaterialIndex, SourceCorner, SourceMaterials, TriangleSoup};

fn with_file_fields() -> (TriangleSoup, SourceMaterials) {
    let p = (0..4).map(|i| DVec3::new(f64::from(i), 0.0, 0.0)).collect();
    let corners = (0..6)
        .map(|i| SourceCorner {
            primitive: 0,
            vertex: i,
        })
        .collect();
    let m = SourceMaterials::new(vec!["red".to_owned()], vec![Some(MaterialIndex::new(0))]);
    let soup = TriangleSoup::new(p, vec![[0, 1, 2], [1, 2, 3]], corners, None)
        .unwrap()
        .with_materials(m.clone(), 3)
        .unwrap();
    (soup, m)
}

#[test]
fn reindexed_with_faces_and_retaining_keep_materials_and_extra_uv_sets() {
    let (soup, m) = with_file_fields();
    let corners = soup.corner_sources().to_vec();
    let rebuilt = [
        soup.reindexed(&[0, 1, 2, 3]),
        soup.with_faces(soup.triangle_indices().to_vec(), corners, None),
        soup.retaining(&[1]),
    ];
    for s in rebuilt {
        assert_eq!(s.materials(), &m);
        assert_eq!(s.extra_uv_sets(), 3);
    }
}
