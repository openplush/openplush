//! A corrupted chart is refused naming the face it broke: a vertex at NaN or infinity, or a
//! face naming a vertex with no position, is `Degenerate` at the lowest face that touches it,
//! whether the vertex is on the boundary or inside.

use openplush_flatten::{FlattenError, FlattenSession};
use openplush_geom::DVec3;
use openplush_mesh::{FaceId, VertexId};
use openplush_seams::ChartId;

type Faces = Vec<(FaceId, [VertexId; 3])>;

const CHART: ChartId = ChartId::new(2);
const COLS: u32 = 4;
const ROWS: u32 = 3;

/// A saddle-shaped grid, two counter-clockwise triangles per quad on vertex
/// `row * (COLS + 1) + col`: curved, with boundary and interior vertices.
fn saddle() -> (Vec<DVec3>, Faces) {
    let xyz = (0..=ROWS)
        .flat_map(|r| (0..=COLS).map(move |c| (f64::from(c), f64::from(r))))
        .map(|(x, y)| DVec3::new(x, y, 0.1 * (x - 2.0) * (y - 1.5)))
        .collect();
    let id = |c: u32, r: u32| r * (COLS + 1) + c;
    let faces = (0..ROWS)
        .flat_map(|r| (0..COLS).map(move |c| (c, r)))
        .flat_map(|(c, r)| {
            let (a, b, d, e) = (id(c, r), id(c + 1, r), id(c + 1, r + 1), id(c, r + 1));
            [[a, b, d], [a, d, e]]
        })
        .zip(0..)
        .map(|(t, f)| (FaceId::new(f), t.map(VertexId::new)))
        .collect();
    (xyz, faces)
}

fn refusal(xyz: &[DVec3], faces: &Faces) -> Option<FlattenError> {
    FlattenSession::new(CHART, xyz, faces.clone()).err()
}

#[test]
fn a_vertex_at_nan_or_infinity_is_refused_at_its_lowest_face() {
    let (xyz, faces) = saddle();
    assert_eq!(refusal(&xyz, &faces), None, "the clean chart starts");
    for v in 0..xyz.len() {
        let first = faces
            .iter()
            .filter(|(_, t)| t.iter().any(|w| w.index() as usize == v))
            .map(|&(f, _)| f)
            .min();
        assert!(first.is_some(), "vertex {v} is used");
        for bad in [f64::NAN, f64::INFINITY] {
            let mut broken = xyz.clone();
            broken[v] = DVec3::new(bad, broken[v].y, broken[v].z);
            assert_eq!(
                refusal(&broken, &faces),
                first.map(|face| FlattenError::Degenerate { chart: CHART, face }),
                "vertex {v} at {bad}"
            );
        }
    }
}

#[test]
fn a_face_naming_a_missing_vertex_is_refused_by_that_face() {
    let (xyz, faces) = saddle();
    let missing = VertexId::new(xyz.len() as u32);
    for (i, &(face, _)) in faces.iter().enumerate() {
        for corner in 0..3 {
            let mut broken = faces.clone();
            broken[i].1[corner] = missing;
            assert_eq!(
                refusal(&xyz, &broken),
                Some(FlattenError::Degenerate { chart: CHART, face }),
                "face {} corner {corner}",
                face.index()
            );
        }
    }
}
