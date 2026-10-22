//! Developable charts flatten through `FlattenSession` with no distortion: the oracle that
//! catches a flattener that is confidently wrong.

use std::f64::consts::PI;

use openplush_flatten::{FlattenError, FlattenSession, Progress};
use openplush_geom::{DVec2, DVec3};
use openplush_mesh::{FaceId, VertexId};
use openplush_seams::ChartId;
use openplush_testkit::oracles::{Chart, OracleFailure, area_conserved, developable, injective};

type Faces = Vec<(FaceId, [VertexId; 3])>;

const CHART: ChartId = ChartId::new(7);

/// These charts lay out exact to rounding (every singular value within 1e-13 of 1); 1e-9 leaves
/// headroom for rounding and still fails a layout off by a millionth.
const TOL: f64 = 1e-9;

/// A `cols` by `rows` grid on `place(u, v)` with `u, v` in [0, 1], two counter-clockwise
/// triangles per quad on vertex `row * (cols + 1) + col`.
fn grid(cols: u32, rows: u32, place: impl Fn(f64, f64) -> DVec3) -> (Vec<DVec3>, Faces) {
    let xyz = (0..=rows)
        .flat_map(|r| (0..=cols).map(move |c| (c, r)))
        .map(|(c, r)| {
            place(
                f64::from(c) / f64::from(cols),
                f64::from(r) / f64::from(rows),
            )
        })
        .collect();
    let id = |c: u32, r: u32| r * (cols + 1) + c;
    let faces = (0..rows)
        .flat_map(|r| (0..cols).map(move |c| (c, r)))
        .flat_map(|(c, r)| {
            let (a, b, d, e) = (id(c, r), id(c + 1, r), id(c + 1, r + 1), id(c, r + 1));
            [[a, b, d], [a, d, e]]
        })
        .zip(0..)
        .map(|(t, f)| (FaceId::new(f), t.map(VertexId::new)))
        .collect();
    (xyz, faces)
}

/// Steps the session to `Done` and returns each vertex's flat position, in vertex order.
fn flatten(xyz: &[DVec3], faces: &Faces) -> Result<Vec<DVec2>, FlattenError> {
    let mut session = FlattenSession::new(CHART, xyz, faces.clone())?;
    while session.step()? != Progress::Done {}
    let layout = session.finish().unwrap_or_default();
    assert_eq!(layout.len(), xyz.len(), "every vertex is laid out");
    let mut uv = vec![DVec2::new(f64::NAN, f64::NAN); xyz.len()];
    for (v, p) in layout {
        uv[v.index() as usize] = p;
    }
    Ok(uv)
}

fn assert_isometric(xyz: &[DVec3], faces: &Faces) -> Result<(), FlattenError> {
    let uv = flatten(xyz, faces)?;
    let corners: Vec<[VertexId; 3]> = faces.iter().map(|&(_, t)| t).collect();
    let chart = Chart {
        xyz,
        uv: &uv,
        faces: &corners,
    };
    let checks: [Result<(), OracleFailure>; 3] = [
        injective(&chart),
        developable(&chart, TOL),
        area_conserved(&chart, TOL),
    ];
    assert_eq!(checks, [Ok(()), Ok(()), Ok(())]);
    Ok(())
}

#[test]
fn a_cylinder_strip_unrolls_without_distortion() -> Result<(), FlattenError> {
    // Three quarters of the way round a 40 mm radius, 60 mm tall: every quad is a flat
    // rectangle, so the faceted strip is exactly developable.
    let (xyz, faces) = grid(18, 4, |u, v| {
        let a = 1.5 * PI * u;
        DVec3::new(40.0 * a.cos(), 40.0 * a.sin(), 60.0 * v)
    });
    assert_isometric(&xyz, &faces)
}

#[test]
fn a_cone_strip_unrolls_without_distortion() -> Result<(), FlattenError> {
    // A frustum of a cone with a 30° half-angle, 20 to 80 mm from the apex, two thirds of the
    // way round. Each quad lies on two generators through the apex, so it is a flat trapezoid.
    let (sin, cos) = (PI / 6.0).sin_cos();
    let (xyz, faces) = grid(16, 5, |u, v| {
        let a = 4.0 * PI / 3.0 * u;
        let s = 20.0 + 60.0 * v;
        DVec3::new(s * sin * a.cos(), s * sin * a.sin(), s * cos)
    });
    assert_isometric(&xyz, &faces)
}

#[test]
fn a_folded_flat_sheet_unfolds_without_distortion() -> Result<(), FlattenError> {
    // A 100 by 50 mm sheet folded 90° along x = 0, which is a column of grid vertices.
    let (xyz, faces) = grid(10, 5, |u, v| {
        let (x, y) = (100.0 * (u - 0.5), 50.0 * v);
        if x <= 0.0 {
            DVec3::new(x, y, 0.0)
        } else {
            DVec3::new(0.0, y, x)
        }
    });
    assert_isometric(&xyz, &faces)
}

#[test]
fn a_sheet_folded_twice_unfolds_without_distortion() -> Result<(), FlattenError> {
    // Folded 120° up at x = -20 and 120° down at x = 20 mm into a zigzag.
    let (sin, cos) = (PI / 3.0).sin_cos();
    let (xyz, faces) = grid(10, 4, |u, v| {
        let (x, y) = (100.0 * (u - 0.5), 40.0 * v);
        if x < -20.0 {
            let t = -20.0 - x;
            DVec3::new(-20.0 - t * cos, y, t * sin)
        } else if x > 20.0 {
            let t = x - 20.0;
            DVec3::new(20.0 + t * cos, y, -t * sin)
        } else {
            DVec3::new(x, y, 0.0)
        }
    });
    assert_isometric(&xyz, &faces)
}
