//! The chart oracles must catch what they exist to catch, and pass what is
//! correct, or every flattening test built on them proves nothing.

use std::f64::consts::PI;

use openplush_geom::{DVec2, DVec3};
use openplush_mesh::{FaceId, VertexId};
use openplush_testkit::oracles::{Chart, OracleFailure, area_conserved, developable, injective};

type Mesh = (Vec<DVec3>, Vec<DVec2>, Vec<[VertexId; 3]>);

/// A strip of a radius-`r` cylinder, `n` quads around, and its exact unrolling.
fn cylinder_strip(r: f64, n: u32) -> Mesh {
    let (mut xyz, mut uv, mut faces) = (Vec::new(), Vec::new(), Vec::new());
    let sweep = PI / 2.0;
    for i in 0..=n {
        let t = sweep * f64::from(i) / f64::from(n);
        for z in [0.0, 10.0] {
            xyz.push(DVec3::new(r * t.cos(), r * t.sin(), z));
            uv.push(DVec2::new(r * t, z));
        }
    }
    let v = VertexId::new;
    for i in 0..n {
        let (a, b, c, d) = (2 * i, 2 * i + 1, 2 * i + 2, 2 * i + 3);
        faces.push([v(a), v(c), v(d)]);
        faces.push([v(a), v(d), v(b)]);
    }
    (xyz, uv, faces)
}

/// One triangle from 3D corners and their flat positions.
fn triangle(xyz: [(f64, f64, f64); 3], uv: [(f64, f64); 3]) -> Mesh {
    (
        xyz.map(|(x, y, z)| DVec3::new(x, y, z)).to_vec(),
        uv.map(|(x, y)| DVec2::new(x, y)).to_vec(),
        vec![[VertexId::new(0), VertexId::new(1), VertexId::new(2)]],
    )
}

fn chart<'a>((xyz, uv, faces): &'a Mesh) -> Chart<'a> {
    Chart { xyz, uv, faces }
}

const RIGHT: [(f64, f64, f64); 3] = [(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 1.0, 0.0)];

#[test]
fn an_unrolled_cylinder_is_developable_injective_and_area_preserving() {
    // The chord, not the arc, is what a flat triangle spans, so the exact
    // unrolling stretches each triangle by arc/chord = 1.0016 at 8 segments.
    let strip = cylinder_strip(20.0, 8);
    assert_eq!(developable(&chart(&strip), 0.01), Ok(()));
    assert_eq!(injective(&chart(&strip)), Ok(()));
    assert_eq!(area_conserved(&chart(&strip), 0.01), Ok(()));
}

#[test]
fn an_isometry_of_a_flat_sheet_has_zero_distortion() {
    // A 3-4-5 triangle rotated by 30 degrees and translated.
    let (s, co) = (0.5_f64, 0.75_f64.sqrt());
    let at = |x: f64, y: f64| (co * x - s * y + 7.0, s * x + co * y - 2.0);
    let sheet = triangle(
        [(0.0, 0.0, 0.0), (3.0, 0.0, 0.0), (0.0, 4.0, 0.0)],
        [at(0.0, 0.0), at(3.0, 0.0), at(0.0, 4.0)],
    );
    assert_eq!(developable(&chart(&sheet), 1e-12), Ok(()));
    assert_eq!(area_conserved(&chart(&sheet), 1e-12), Ok(()));
}

#[test]
fn a_stretched_chart_fails_developable_at_its_first_face() {
    let mut strip = cylinder_strip(20.0, 8);
    strip.1.iter_mut().for_each(|p| p.x *= 1.05);
    match developable(&chart(&strip), 0.01) {
        Err(OracleFailure::Distorted {
            face, sigma_max, ..
        }) => {
            assert_eq!(face, FaceId::new(0));
            assert!((sigma_max - 1.05 * 1.001_607).abs() < 1e-5, "{sigma_max}");
        }
        other => panic!("expected Distorted, got {other:?}"),
    }
    assert!(matches!(
        area_conserved(&chart(&strip), 0.01),
        Err(OracleFailure::AreaChanged { .. })
    ));
}

#[test]
fn a_uniformly_shrunk_chart_fails_developable() {
    let mut strip = cylinder_strip(20.0, 8);
    strip.1.iter_mut().for_each(|p| *p = *p * 0.95);
    match developable(&chart(&strip), 0.01) {
        Err(OracleFailure::Distorted { sigma_min, .. }) => assert!(sigma_min < 0.99, "{sigma_min}"),
        other => panic!("expected Distorted, got {other:?}"),
    }
}

#[test]
fn a_chart_shrunk_along_one_axis_fails_on_its_smaller_singular_value() {
    let mut strip = cylinder_strip(20.0, 8);
    strip.1.iter_mut().for_each(|p| p.y *= 0.95);
    match developable(&chart(&strip), 0.01) {
        Err(OracleFailure::Distorted {
            sigma_max,
            sigma_min,
            ..
        }) => {
            assert!((sigma_max - 1.0).abs() <= 0.01, "{sigma_max}");
            assert!((sigma_min - 0.95).abs() < 1e-3, "{sigma_min}");
        }
        other => panic!("expected Distorted, got {other:?}"),
    }
}

#[test]
fn a_mirrored_chart_is_inverted_distorted_and_loses_its_area() {
    let mut strip = cylinder_strip(20.0, 8);
    strip.1.iter_mut().for_each(|p| p.x = -p.x);
    match injective(&chart(&strip)) {
        Err(OracleFailure::Inverted { faces }) => assert_eq!(faces.len(), 16),
        other => panic!("expected Inverted, got {other:?}"),
    }
    match developable(&chart(&strip), 0.01) {
        Err(OracleFailure::Distorted { sigma_min, .. }) => assert!(sigma_min < 0.0, "{sigma_min}"),
        other => panic!("expected Distorted, got {other:?}"),
    }
    match area_conserved(&chart(&strip), 0.01) {
        Err(OracleFailure::AreaChanged { area_2d, .. }) => assert!(area_2d < 0.0, "{area_2d}"),
        other => panic!("expected AreaChanged, got {other:?}"),
    }
}

#[test]
fn a_degenerate_flat_face_is_not_injective() {
    let flat = triangle(RIGHT, [(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)]);
    let faces = vec![FaceId::new(0)];
    assert_eq!(
        injective(&chart(&flat)),
        Err(OracleFailure::Inverted { faces })
    );
}

#[test]
fn a_degenerate_3d_face_has_no_defined_distortion() {
    let needle = triangle(
        [(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (2.0, 0.0, 0.0)],
        [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)],
    );
    let face = FaceId::new(0);
    assert_eq!(
        developable(&chart(&needle), 0.5),
        Err(OracleFailure::Degenerate { face })
    );
}

#[test]
fn a_nan_flat_position_fails_every_chart_oracle() {
    let bad = triangle(RIGHT, [(0.0, 0.0), (f64::NAN, 0.0), (0.0, 1.0)]);
    assert!(matches!(
        injective(&chart(&bad)),
        Err(OracleFailure::Inverted { .. })
    ));
    assert!(matches!(
        developable(&chart(&bad), 0.01),
        Err(OracleFailure::Distorted { .. })
    ));
    assert!(matches!(
        area_conserved(&chart(&bad), 0.01),
        Err(OracleFailure::AreaChanged { .. })
    ));
}

#[test]
fn a_nan_infinite_or_negative_tolerance_is_refused() {
    let ok = triangle(RIGHT, [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)]);
    for tol in [f64::NAN, f64::INFINITY, -0.01] {
        let bad = |r| matches!(r, Err(OracleFailure::BadTolerance { tol: t }) if t.total_cmp(&tol).is_eq());
        assert!(bad(developable(&chart(&ok), tol)), "{tol}");
        assert!(bad(area_conserved(&chart(&ok), tol)), "{tol}");
    }
}

#[test]
fn an_empty_chart_does_not_pass_vacuously() {
    let empty: Mesh = (Vec::new(), Vec::new(), Vec::new());
    assert_eq!(injective(&chart(&empty)), Err(OracleFailure::Empty));
    assert_eq!(developable(&chart(&empty), 0.01), Err(OracleFailure::Empty));
    assert_eq!(
        area_conserved(&chart(&empty), 0.01),
        Err(OracleFailure::Empty)
    );
}

#[test]
fn malformed_input_is_a_failure_not_a_panic() {
    let mut short = triangle(RIGHT, [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)]);
    short.1.pop();
    assert_eq!(
        injective(&chart(&short)),
        Err(OracleFailure::LengthMismatch { xyz: 3, uv: 2 })
    );
    let mut far = triangle(RIGHT, [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)]);
    far.2[0][2] = VertexId::new(9);
    let face = FaceId::new(0);
    assert_eq!(
        developable(&chart(&far), 0.1),
        Err(OracleFailure::BadIndex { face })
    );
}
