//! A rigid motion of the chart changes nothing the flattener reports: the same number of
//! iterations, the same energy, the same layout up to a rigid motion of the plane. A developable
//! chart would pass trivially, so the chart is a hemisphere, which no layout keeps isometric.

use std::f64::consts::{FRAC_PI_2, TAU};

use openplush_flatten::{FlattenError, FlattenSession, Progress};
use openplush_geom::epsilon::ModelTolerances;
use openplush_geom::{DVec2, DVec3, GeomError};
use openplush_mesh::{FaceId, VertexId};
use openplush_seams::ChartId;

type Faces = Vec<(FaceId, [VertexId; 3])>;

const CHART: ChartId = ChartId::new(4);
const RINGS: u32 = 16;

/// The unit northern hemisphere as concentric rings around the pole: ring `k` has `6k`
/// vertices from vertex `1 + 3k(k − 1)`, faces counter-clockwise seen from outside.
fn hemisphere() -> (Vec<DVec3>, Faces) {
    let ring = |k: u32| (1 + 3 * k * (k - 1)..1 + 3 * k * (k + 1)).collect::<Vec<u32>>();
    let mut xyz = vec![DVec3::Z];
    for k in 1..=RINGS {
        let theta = FRAC_PI_2 * f64::from(k) / f64::from(RINGS);
        xyz.extend((0..6 * k).map(|j| {
            let a = TAU * f64::from(j) / f64::from(6 * k);
            DVec3::new(
                libm::sin(theta) * libm::cos(a),
                libm::sin(theta) * libm::sin(a),
                libm::cos(theta),
            )
        }));
    }
    let mut tris: Vec<[u32; 3]> = (0..6).map(|j| [0, 1 + j, 1 + (j + 1) % 6]).collect();
    for k in 2..=RINGS {
        band(&ring(k - 1), &ring(k), &mut tris);
    }
    let faces = (0..)
        .zip(tris)
        .map(|(f, t)| (FaceId::new(f), t.map(VertexId::new)))
        .collect();
    (xyz, faces)
}

/// The band between two rings, walking both by angle.
fn band(inner: &[u32], outer: &[u32], tris: &mut Vec<[u32; 3]>) {
    let (n_in, n_out) = (inner.len(), outer.len());
    let (mut i, mut j) = (0, 0);
    while i < n_in || j < n_out {
        let a_in = (i + 1) as f64 / n_in as f64;
        let a_out = (j + 1) as f64 / n_out as f64;
        if j < n_out && (i >= n_in || a_out <= a_in) {
            tris.push([inner[i % n_in], outer[j], outer[(j + 1) % n_out]]);
            j += 1;
        } else {
            tris.push([inner[i % n_in], outer[j % n_out], inner[(i + 1) % n_in]]);
            i += 1;
        }
    }
}

/// Rodrigues' rotation by `angle` about the unit `axis`.
fn rotate(p: DVec3, axis: DVec3, angle: f64) -> DVec3 {
    let (sin, cos) = libm::sincos(angle);
    p * cos + axis.cross(p) * sin + axis * axis.dot(p) * (1.0 - cos)
}

/// The iterations to `Done`, the energy of the last working step, and the layout.
fn flatten(xyz: &[DVec3], faces: &Faces) -> Result<(u32, f64, Vec<DVec2>), FlattenError> {
    let mut session = FlattenSession::new(CHART, xyz, faces.clone())?;
    let (mut iterations, mut last) = (0, f64::NAN);
    while let Progress::Working { iteration, energy } = session.step()? {
        (iterations, last) = (iteration, energy);
    }
    let layout = session.finish().unwrap_or_default();
    assert_eq!(layout.len(), xyz.len(), "every vertex is laid out");
    Ok((
        iterations,
        last,
        layout.into_iter().map(|(_, p)| p).collect(),
    ))
}

fn diagonal(xyz: &[DVec3]) -> f64 {
    let span = |c: fn(&DVec3) -> f64| {
        let (lo, hi) = xyz
            .iter()
            .map(c)
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
                (lo.min(v), hi.max(v))
            });
        hi - lo
    };
    DVec3::new(span(|p| p.x), span(|p| p.y), span(|p| p.z)).length()
}

#[test]
fn a_rotated_and_moved_hemisphere_flattens_the_same() -> Result<(), String> {
    let (xyz, faces) = hemisphere();
    let axis = DVec3::new(1.0, 2.0, 3.0) / 14f64.sqrt();
    let shift = DVec3::new(1.0, -1.0, 1.0) / 3f64.sqrt() * (10.0 * diagonal(&xyz));
    let moved: Vec<DVec3> = xyz
        .iter()
        .map(|&p| rotate(p, axis, 37f64.to_radians()) + shift)
        .collect();
    let tol =
        ModelTolerances::for_diagonal(diagonal(&xyz)).map_err(|e: GeomError| e.to_string())?;
    let (n, e, uv) = flatten(&xyz, &faces).map_err(|e| e.to_string())?;
    let (m, f, wv) = flatten(&moved, &faces).map_err(|e| e.to_string())?;
    assert!(n > 2, "the hemisphere must need several iterations: {n}");
    assert_eq!(n, m, "iteration counts");
    assert!((e - f).abs() <= 1e-12 * e, "energies {e} and {f}");
    for (_, t) in &faces {
        let t = t.map(|v| v.index() as usize);
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            let (here, there) = ((uv[b] - uv[a]).length(), (wv[b] - wv[a]).length());
            assert!(
                (here - there).abs() <= tol.weld(),
                "edge {a}-{b}: {here} and {there}"
            );
        }
    }
    Ok(())
}
