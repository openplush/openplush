//! `metrics::distortion`: each face's singular values against hand-computed maps.

use std::f64::consts::PI;

use openplush_flatten::FlattenError;
use openplush_flatten::metrics::{Distortion, distortion};
use openplush_geom::{DVec2, DVec3};
use openplush_mesh::{FaceId, VertexId};
use openplush_seams::ChartId;

type Faces = Vec<(FaceId, [VertexId; 3])>;

const CHART: ChartId = ChartId::new(4);

/// The right isosceles triangle with unit legs, in the plane z = 0.
fn right_triangle() -> (Vec<DVec3>, Faces) {
    let xyz = vec![DVec3::ZERO, DVec3::X, DVec3::Y];
    (xyz, vec![face(0, [0, 1, 2])])
}

fn face(id: u32, t: [u32; 3]) -> (FaceId, [VertexId; 3]) {
    (FaceId::new(id), t.map(VertexId::new))
}

fn uv(points: &[(f64, f64)]) -> Vec<DVec2> {
    points.iter().map(|&(x, y)| DVec2::new(x, y)).collect()
}

fn one(xyz: &[DVec3], faces: &Faces, flat: &[DVec2]) -> Result<Distortion, FlattenError> {
    let measured = distortion(CHART, xyz, faces, flat)?;
    assert_eq!(measured.len(), 1);
    Ok(measured[0].1)
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1e-12 * expected.abs().max(1.0),
        "{actual} is not {expected}"
    );
}

/// Stretch, compression, area ratio and conformal ratio, in that order.
fn assert_values(d: Distortion, expected: [f64; 4]) {
    let actual = [
        d.stretch(),
        d.compression(),
        d.area_ratio(),
        d.conformal_ratio(),
    ];
    for (a, e) in actual.into_iter().zip(expected) {
        assert_close(a, e);
    }
}

#[test]
fn an_isometry_neither_stretches_nor_compresses() -> Result<(), FlattenError> {
    let (xyz, faces) = right_triangle();
    // The same triangle turned by 30° and moved.
    let (s, c) = (PI / 6.0).sin_cos();
    let flat = uv(&[(5.0, -2.0), (5.0 + c, -2.0 + s), (5.0 - s, -2.0 + c)]);
    assert_values(one(&xyz, &faces, &flat)?, [1.0, 1.0, 1.0, 1.0]);
    Ok(())
}

#[test]
fn doubling_one_leg_stretches_by_two() -> Result<(), FlattenError> {
    let (xyz, faces) = right_triangle();
    let flat = uv(&[(0.0, 0.0), (2.0, 0.0), (0.0, 1.0)]);
    assert_values(one(&xyz, &faces, &flat)?, [2.0, 1.0, 2.0, 2.0]);
    Ok(())
}

#[test]
fn halving_one_leg_compresses_by_half() -> Result<(), FlattenError> {
    let (xyz, faces) = right_triangle();
    let flat = uv(&[(0.0, 0.0), (1.0, 0.0), (0.0, 0.5)]);
    assert_values(one(&xyz, &faces, &flat)?, [1.0, 0.5, 0.5, 2.0]);
    Ok(())
}

#[test]
fn a_unit_shear_has_the_golden_ratio_as_its_singular_values() -> Result<(), FlattenError> {
    let (xyz, faces) = right_triangle();
    // J = [[1, 1], [0, 1]]: σ₁σ₂ = det = 1 and σ₁² + σ₂² = |J|² = 3, so σ = (√5 ± 1) / 2.
    let flat = uv(&[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)]);
    let phi = f64::midpoint(1.0, 5.0_f64.sqrt());
    assert_values(one(&xyz, &faces, &flat)?, [phi, 1.0 / phi, 1.0, phi * phi]);
    Ok(())
}

#[test]
fn a_face_tilted_in_3d_is_measured_in_its_own_plane() -> Result<(), FlattenError> {
    // Legs √2 (along x = z) and 1 (along y), at a right angle.
    let xyz = vec![DVec3::ZERO, DVec3::new(1.0, 0.0, 1.0), DVec3::Y];
    let faces = vec![face(0, [0, 1, 2])];
    let flat = uv(&[(0.0, 0.0), (2.0_f64.sqrt(), 0.0), (0.0, 3.0)]);
    assert_values(one(&xyz, &faces, &flat)?, [3.0, 1.0, 3.0, 3.0]);
    Ok(())
}

/// A faceted half-cylinder of radius 1, `columns` facets around and `rows` high, and its exact
/// unrolling: each facet is a flat rectangle one chord wide.
fn half_cylinder(columns: u32, rows: u32) -> (Vec<DVec3>, Faces, Vec<DVec2>) {
    let step = PI / f64::from(columns);
    let chord = 2.0 * (step / 2.0).sin();
    let grid = (0..=rows).flat_map(|r| (0..=columns).map(move |c| (c, r)));
    let xyz = grid
        .clone()
        .map(|(c, r)| {
            let a = f64::from(c) * step;
            DVec3::new(a.cos(), a.sin(), f64::from(r))
        })
        .collect();
    let unrolled = grid
        .map(|(c, r)| DVec2::new(f64::from(c) * chord, f64::from(r)))
        .collect();
    let at = |c: u32, r: u32| r * (columns + 1) + c;
    let faces = (0..rows)
        .flat_map(|r| (0..columns).map(move |c| (c, r)))
        .flat_map(|(c, r)| {
            let (a, b, d, e) = (at(c, r), at(c + 1, r), at(c + 1, r + 1), at(c, r + 1));
            [[a, b, d], [a, d, e]]
        })
        .enumerate()
        .map(|(i, t)| face(u32::try_from(i).unwrap_or(u32::MAX), t))
        .collect();
    (xyz, faces, unrolled)
}

#[test]
fn a_stretched_unrolling_reports_the_stretch_on_every_face_in_input_order()
-> Result<(), FlattenError> {
    let (xyz, faces, unrolled) = half_cylinder(8, 3);
    // Twice as long around, half as tall: σ = (2, 0.5), area kept, conformal ratio 4.
    let flat: Vec<DVec2> = unrolled
        .iter()
        .map(|p| DVec2::new(2.0 * p.x, 0.5 * p.y))
        .collect();
    let measured = distortion(CHART, &xyz, &faces, &flat)?;
    assert_eq!(
        measured.iter().map(|(f, _)| *f).collect::<Vec<_>>(),
        faces.iter().map(|(f, _)| *f).collect::<Vec<_>>()
    );
    for (_, d) in measured {
        assert_values(d, [2.0, 0.5, 1.0, 4.0]);
    }
    Ok(())
}

#[test]
fn an_exact_unrolling_is_an_isometry_on_every_face() -> Result<(), FlattenError> {
    let (xyz, faces, unrolled) = half_cylinder(8, 3);
    for (_, d) in distortion(CHART, &xyz, &faces, &unrolled)? {
        assert_values(d, [1.0, 1.0, 1.0, 1.0]);
    }
    Ok(())
}

#[test]
fn a_chart_with_no_faces_is_refused() {
    assert_eq!(
        distortion(CHART, &[], &[], &[]),
        Err(FlattenError::EmptyChart { chart: CHART })
    );
}

#[test]
fn every_flipped_or_collapsed_face_is_named_in_input_order() {
    let (one_face, _) = right_triangle();
    let xyz: Vec<DVec3> = one_face.iter().cycle().take(9).copied().collect();
    let faces = vec![
        face(10, [0, 1, 2]),
        face(11, [3, 4, 5]),
        face(12, [6, 7, 8]),
    ];
    // Face 10 mirrored, face 11 kept, face 12 collapsed onto a line.
    let flat = uv(&[
        (0.0, 0.0),
        (1.0, 0.0),
        (0.0, -1.0),
        (0.0, 0.0),
        (1.0, 0.0),
        (0.0, 1.0),
        (0.0, 0.0),
        (1.0, 0.0),
        (2.0, 0.0),
    ]);
    assert_eq!(
        distortion(CHART, &xyz, &faces, &flat),
        Err(FlattenError::Flip {
            chart: CHART,
            faces: vec![FaceId::new(10), FaceId::new(12)],
        })
    );
}

#[test]
fn a_face_with_no_3d_area_is_degenerate() {
    let xyz = vec![DVec3::ZERO, DVec3::X, DVec3::new(2.0, 0.0, 0.0)];
    let faces = vec![face(5, [0, 1, 2])];
    let flat = uv(&[(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)]);
    assert_eq!(
        distortion(CHART, &xyz, &faces, &flat),
        Err(FlattenError::Degenerate {
            chart: CHART,
            face: FaceId::new(5),
        })
    );
}

#[test]
fn a_face_naming_a_vertex_with_no_position_is_degenerate() {
    let (xyz, _) = right_triangle();
    let flat = uv(&[(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)]);
    for (t, positions, layout) in [
        ([0, 1, 3], &xyz[..], &flat[..]),
        ([0, 1, 2], &xyz[..], &flat[..2]),
    ] {
        assert_eq!(
            distortion(CHART, positions, &[face(6, t)], layout),
            Err(FlattenError::Degenerate {
                chart: CHART,
                face: FaceId::new(6),
            })
        );
    }
}

#[test]
fn a_face_with_a_non_finite_flat_position_is_degenerate() {
    let (xyz, faces) = right_triangle();
    for bad in [f64::NAN, f64::INFINITY] {
        let flat = uv(&[(0.0, 0.0), (bad, 0.0), (0.0, 1.0)]);
        assert_eq!(
            distortion(CHART, &xyz, &faces, &flat),
            Err(FlattenError::Degenerate {
                chart: CHART,
                face: FaceId::new(0),
            })
        );
    }
}

#[test]
fn a_face_whose_flat_scale_overflows_is_degenerate() {
    let (xyz, faces) = right_triangle();
    let flat = uv(&[(0.0, 0.0), (f64::MAX, 0.0), (0.0, f64::MAX)]);
    assert_eq!(
        distortion(CHART, &xyz, &faces, &flat),
        Err(FlattenError::Degenerate {
            chart: CHART,
            face: FaceId::new(0),
        })
    );
}

#[test]
fn a_thin_face_keeps_its_compression_exact() -> Result<(), FlattenError> {
    let (xyz, faces) = right_triangle();
    let flat = uv(&[(0.0, 0.0), (1e8, 0.0), (0.0, 1e-9)]);
    assert_values(one(&xyz, &faces, &flat)?, [1e8, 1e-9, 0.1, 1e17]);
    Ok(())
}

#[test]
fn a_face_with_no_finite_singular_values_is_degenerate_not_flipped() {
    let (xyz, faces) = right_triangle();
    for flat in [
        // det J overflows while σ₁ stays finite.
        uv(&[(0.0, 0.0), (1e200, 0.0), (0.0, 1e200)]),
        // σ₂ underflows to zero.
        uv(&[(0.0, 0.0), (1.0, 0.0), (1e10, 5e-324)]),
    ] {
        assert_eq!(
            distortion(CHART, &xyz, &faces, &flat),
            Err(FlattenError::Degenerate {
                chart: CHART,
                face: FaceId::new(0),
            })
        );
    }
}

#[test]
fn an_infinite_jacobian_is_degenerate_even_when_its_det_is_negative() {
    // A skewed rest shape, so no Jacobian entry is ∞ · 0: det J is -∞, not NaN.
    let xyz = vec![DVec3::ZERO, DVec3::X, DVec3::new(1.0, 1.0, 0.0)];
    let flat = uv(&[(0.0, 0.0), (f64::INFINITY, -1.0), (0.0, -3.0)]);
    assert_eq!(
        distortion(CHART, &xyz, &[face(0, [0, 1, 2])], &flat),
        Err(FlattenError::Degenerate {
            chart: CHART,
            face: FaceId::new(0),
        })
    );
}

#[test]
fn a_degenerate_face_is_reported_over_an_earlier_flip() {
    let xyz = vec![DVec3::ZERO, DVec3::X, DVec3::Y, DVec3::new(1.0, 1.0, 0.0)];
    // Face 1 mirrored; face 2 has a 3D shape but vertex 3 has no flat position.
    let flat = uv(&[(0.0, 0.0), (1.0, 0.0), (0.0, -1.0)]);
    let faces = vec![face(1, [0, 1, 2]), face(2, [1, 3, 2])];
    assert_eq!(
        distortion(CHART, &xyz, &faces, &flat),
        Err(FlattenError::Degenerate {
            chart: CHART,
            face: FaceId::new(2),
        })
    );
}
