use openplush_geom::{DVec2, DVec3};
use openplush_mesh::{FaceId, VertexId};

use super::{energy, max_step, rest_shapes};

type Faces = Vec<(FaceId, [VertexId; 3])>;

fn faces(tris: &[[u32; 3]]) -> Faces {
    (0..)
        .zip(tris)
        .map(|(f, t)| (FaceId::new(f), t.map(VertexId::new)))
        .collect()
}

fn uv(points: &[(f64, f64)]) -> Vec<DVec2> {
    points.iter().map(|&(x, y)| DVec2::new(x, y)).collect()
}

/// A unit right triangle tilted out of the xy-plane, and its flat layout.
fn tilted() -> (Vec<DVec3>, Faces, Vec<DVec2>) {
    let (u, v) = (DVec3::new(0.6, 0.0, 0.8), DVec3::new(0.0, 1.0, 0.0));
    let xyz = vec![DVec3::ZERO, u, v];
    (
        xyz,
        faces(&[[0, 1, 2]]),
        uv(&[(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)]),
    )
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-12 * b.abs().max(1.0)
}

#[test]
fn an_isometric_layout_has_the_minimum_energy_of_four() -> Result<(), FaceId> {
    let (xyz, f, flat) = tilted();
    let rest = rest_shapes(&xyz, &f)?;
    // Rotated and moved in the plane: still an isometry.
    let moved: Vec<DVec2> = flat
        .iter()
        .map(|p| DVec2::new(3.0 - p.y, 2.0 + p.x))
        .collect();
    for layout in [flat, moved] {
        assert!(matches!(energy(&rest, &f, &layout), Some(e) if near(e, 4.0)));
    }
    Ok(())
}

#[test]
fn a_uniform_scale_costs_both_ways() -> Result<(), FaceId> {
    // J = s I: |J|² = 2 s², |J⁻¹|² = 2 / s², so the energy is 2 s² + 2 / s².
    let (xyz, f, flat) = tilted();
    let rest = rest_shapes(&xyz, &f)?;
    for s in [2.0, 0.5] {
        let scaled: Vec<DVec2> = flat.iter().map(|&p| p * s).collect();
        let want = 2.0 * s * s + 2.0 / (s * s);
        assert!(matches!(energy(&rest, &f, &scaled), Some(e) if near(e, want)));
    }
    Ok(())
}

#[test]
fn faces_count_by_their_3d_area() -> Result<(), FaceId> {
    // Face 0 (area 1/2) lies flat as it is; face 1 (area 2) is drawn at half size, so
    // its energy is 2·(1/4) + 2·4 = 8.5. The mean weights them 1 : 4.
    let xyz = vec![
        DVec3::ZERO,
        DVec3::new(1.0, 0.0, 0.0),
        DVec3::new(0.0, 1.0, 0.0),
        DVec3::new(10.0, 0.0, 0.0),
        DVec3::new(12.0, 0.0, 0.0),
        DVec3::new(10.0, 2.0, 0.0),
    ];
    let f = faces(&[[0, 1, 2], [3, 4, 5]]);
    let layout = uv(&[
        (0.0, 0.0),
        (1.0, 0.0),
        (0.0, 1.0),
        (5.0, 0.0),
        (6.0, 0.0),
        (5.0, 1.0),
    ]);
    let want = (0.5 * 4.0 + 2.0 * 8.5) / 2.5;
    let rest = rest_shapes(&xyz, &f)?;
    assert!(matches!(energy(&rest, &f, &layout), Some(e) if near(e, want)));
    Ok(())
}

#[test]
fn a_flipped_or_collapsed_face_has_no_energy() -> Result<(), FaceId> {
    let (xyz, f, _) = tilted();
    let rest = rest_shapes(&xyz, &f)?;
    let flipped = uv(&[(0.0, 0.0), (0.0, 1.0), (1.0, 0.0)]);
    let collapsed = uv(&[(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)]);
    assert_eq!(energy(&rest, &f, &flipped), None);
    assert_eq!(energy(&rest, &f, &collapsed), None);
    Ok(())
}

#[test]
fn a_layout_whose_energy_overflows_has_none() -> Result<(), FaceId> {
    let (xyz, f, flat) = tilted();
    let rest = rest_shapes(&xyz, &f)?;
    let huge: Vec<DVec2> = flat.iter().map(|&p| p * 1e200).collect();
    assert_eq!(energy(&rest, &f, &huge), None);
    Ok(())
}

#[test]
fn a_face_with_no_rest_shape_is_refused() {
    let f = faces(&[[0, 1, 2]]);
    let flat3 = [
        DVec3::ZERO,
        DVec3::new(1.0, 0.0, 0.0),
        DVec3::new(2.0, 0.0, 0.0),
    ];
    let nan = [
        DVec3::ZERO,
        DVec3::new(f64::NAN, 0.0, 0.0),
        DVec3::new(0.0, 1.0, 0.0),
    ];
    // Edge lengths stay finite, but their cross product, twice the area, overflows.
    let huge = [
        DVec3::ZERO,
        DVec3::new(1e150, 0.0, 0.0),
        DVec3::new(0.0, 1e160, 0.0),
    ];
    for xyz in [flat3, nan, huge] {
        assert_eq!(rest_shapes(&xyz, &f).err(), Some(FaceId::new(0)));
    }
}

#[test]
fn the_step_stops_where_the_first_face_would_collapse() {
    let f = faces(&[[0, 1, 2]]);
    let start = uv(&[(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)]);
    // Corner 2 moving straight down reaches edge 0–1 at t = 1/2 (linear det).
    let down = uv(&[(0.0, 0.0), (0.0, 0.0), (0.0, -2.0)]);
    assert!(near(max_step(&f, &start, &down), 0.5));
    // Corners 1 and 2 both moving to the origin: det = (1 - t)², a double root at t = 1.
    let shrink = uv(&[(0.0, 0.0), (-1.0, 0.0), (0.0, -1.0)]);
    assert!(near(max_step(&f, &start, &shrink), 1.0));
    // det = 1 + t - 2t²: one root on each side of zero, and the positive one is q / a.
    let mixed = uv(&[(0.0, 0.0), (0.0, 1.0), (2.0, 1.0)]);
    assert!(near(max_step(&f, &start, &mixed), 1.0));
    // Moving away from collapse, or not deforming at all, never flips.
    let grow = uv(&[(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)]);
    let slide = uv(&[(3.0, 1.0); 3]);
    assert_eq!(max_step(&f, &start, &grow), f64::INFINITY);
    assert_eq!(max_step(&f, &start, &slide), f64::INFINITY);
    // A direction that is not finite allows no step at all.
    let broken = uv(&[(0.0, 0.0), (f64::NAN, 0.0), (0.0, 0.0)]);
    assert_eq!(max_step(&f, &start, &broken), 0.0);
}

#[test]
fn the_step_is_the_smallest_over_all_faces() {
    let f = faces(&[[0, 1, 2], [3, 4, 5]]);
    let start = uv(&[
        (0.0, 0.0),
        (1.0, 0.0),
        (0.0, 1.0),
        (5.0, 0.0),
        (6.0, 0.0),
        (5.0, 1.0),
    ]);
    let dir = uv(&[
        (0.0, 0.0),
        (0.0, 0.0),
        (0.0, -2.0),
        (0.0, 0.0),
        (0.0, 0.0),
        (0.0, -4.0),
    ]);
    assert!(near(max_step(&f, &start, &dir), 0.25));
}
