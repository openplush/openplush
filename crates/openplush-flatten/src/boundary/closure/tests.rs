use std::f64::consts::{PI, TAU};

use openplush_geom::DVec2;

use super::{close, conditions, damped, norm, step, walk};
use crate::boundary::BoundaryError;

fn near(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

/// The point the walk would reach after its last edge: the start again iff the polygon closes.
fn end_point(lengths: &[f64], kappa: &[f64]) -> DVec2 {
    let mut heading = 0.0;
    let mut p = DVec2::ZERO;
    for (i, &l) in lengths.iter().enumerate() {
        if i > 0 {
            heading += kappa[i];
        }
        p += DVec2::new(l * heading.cos(), l * heading.sin());
    }
    p
}

#[test]
fn a_rigid_triangle_closes_at_its_only_turning_angles_whatever_the_target()
-> Result<(), BoundaryError> {
    // Edges 3, 4, 5 in order make a right triangle (0,0), (3,0), (3,4): a triangle's
    // lengths fix its angles, so any target must land on these.
    let lengths = [3.0, 4.0, 5.0];
    let kappa = close(&lengths, &[2.0, 2.0, 2.0])?;
    let expected = [PI - 4.0_f64.atan2(3.0), PI / 2.0, PI - 3.0_f64.atan2(4.0)];
    for (k, e) in kappa.iter().zip(expected) {
        assert!(near(*k, e, 1e-12), "{kappa:?}");
    }
    let points = walk(&lengths, &kappa);
    for (p, e) in points.iter().zip([(0.0, 0.0), (3.0, 0.0), (3.0, 4.0)]) {
        assert!(near(p.x, e.0, 1e-12) && near(p.y, e.1, 1e-12), "{points:?}");
    }
    Ok(())
}

#[test]
fn an_even_shift_from_a_regular_polygon_is_undone() -> Result<(), BoundaryError> {
    // Equal edges with equal turning: the regular polygon is feasible and differs from
    // the target by a multiple of the all-ones direction, so it is the closest one.
    let n = 9;
    let lengths = vec![2.0; n];
    let shifted = vec![TAU / n as f64 + 0.05; n];
    for k in close(&lengths, &shifted)? {
        assert!(near(k, TAU / n as f64, 1e-12));
    }
    Ok(())
}

#[test]
fn a_target_that_already_closes_is_returned_unchanged() -> Result<(), BoundaryError> {
    let lengths = [1.0, 2.0, 1.0, 2.0];
    let target = [PI / 2.0; 4];
    let kappa = close(&lengths, &target)?;
    for (k, t) in kappa.iter().zip(target) {
        assert!(near(*k, t, 1e-15));
    }
    Ok(())
}

#[test]
fn an_open_target_closes_without_changing_a_length() -> Result<(), BoundaryError> {
    let lengths: Vec<f64> = (0..24).map(|i| 1.0 + 0.37 * f64::from(i % 5)).collect();
    let perimeter: f64 = lengths.iter().sum();
    let target: Vec<f64> = (0..24)
        .map(|i| TAU / 24.0 + 0.3 * (f64::from(i) * 0.25).sin())
        .collect();
    assert!(end_point(&lengths, &target).length() > 0.1 * perimeter);
    let kappa = close(&lengths, &target)?;
    assert!(near(kappa.iter().sum(), TAU, 1e-12));
    assert!(end_point(&lengths, &kappa).length() <= 1e-12 * perimeter);
    let points = walk(&lengths, &kappa);
    assert_eq!(points.len(), lengths.len());
    for (i, &l) in lengths.iter().enumerate() {
        let next = points[(i + 1) % points.len()];
        assert!(
            near((next - points[i]).length(), l, 1e-12 * perimeter),
            "edge {i}"
        );
    }
    Ok(())
}

#[test]
fn the_walk_starts_at_the_origin_heading_along_x() {
    let points = walk(&[2.0, 2.0, 2.0, 2.0], &[PI / 2.0; 4]);
    let expected = [(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (0.0, 2.0)];
    for (p, e) in points.iter().zip(expected) {
        assert!(near(p.x, e.0, 1e-15) && near(p.y, e.1, 1e-15), "{points:?}");
    }
}

#[test]
fn the_same_input_closes_to_the_same_bits() -> Result<(), BoundaryError> {
    let lengths: Vec<f64> = (0..50).map(|i| 1.0 + 0.1 * f64::from(i % 7)).collect();
    let target: Vec<f64> = (0..50)
        .map(|i| TAU / 50.0 + 0.1 * (f64::from(i) * 0.9).cos())
        .collect();
    let a = close(&lengths, &target)?;
    let b = close(&lengths, &target)?;
    assert_eq!(
        a.iter().map(|k| k.to_bits()).collect::<Vec<_>>(),
        b.iter().map(|k| k.to_bits()).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn closing_does_not_depend_on_scale() -> Result<(), BoundaryError> {
    let lengths: Vec<f64> = (0..24).map(|i| 1.0 + 0.37 * f64::from(i % 5)).collect();
    let target: Vec<f64> = (0..24)
        .map(|i| TAU / 24.0 + 0.3 * (f64::from(i) * 0.25).sin())
        .collect();
    let unscaled = close(&lengths, &target)?;
    for s in [1e-9, 1e-6, 1e3, 1e9] {
        let scaled: Vec<f64> = lengths.iter().map(|l| l * s).collect();
        for (a, b) in close(&scaled, &target)?.iter().zip(&unscaled) {
            assert!(near(*a, *b, 1e-12), "scale {s}");
        }
    }
    Ok(())
}

#[test]
fn a_long_irregular_boundary_closes() -> Result<(), BoundaryError> {
    let n = 2000;
    let lengths: Vec<f64> = (0..n)
        .map(|i| 1.0 + 0.9 * (f64::from(i) * 2.3).sin())
        .collect();
    let target: Vec<f64> = (0..n)
        .map(|i| TAU / f64::from(n) + 0.01 * (f64::from(i) * 0.7).cos())
        .collect();
    let kappa = close(&lengths, &target)?;
    let perimeter: f64 = lengths.iter().sum();
    assert!(end_point(&lengths, &kappa).length() <= 1e-10 * perimeter);
    Ok(())
}

#[test]
fn one_step_near_a_closed_polygon_squares_the_error() {
    // Newton-like steps converge quadratically; half steps or a scaled condition only linearly.
    let unit = vec![1.0 / 9.0; 9];
    let kappa: Vec<f64> = (0..9)
        .map(|i| TAU / 9.0 + 1e-3 * (1.3 * f64::from(i)).sin())
        .collect();
    let (r0, jacobian) = conditions(&unit, &kappa);
    let e0 = norm(r0);
    let full = step(&kappa, &kappa, r0, &jacobian).unwrap_or_default();
    let next = damped(&unit, &kappa, &full, e0).unwrap_or_default();
    assert_eq!(next.len(), 9);
    assert!(norm(conditions(&unit, &next).0) <= e0 * e0);
}

#[test]
fn a_polygon_still_open_when_the_steps_run_out_is_refused() {
    // Just past degenerate: each step closes a little, never enough within the step limit.
    assert_eq!(
        close(&[1.0, 1.0, 1.0, 3.0 + 1e-9], &[1.5; 4]),
        Err(BoundaryError::NotClosed)
    );
}

#[test]
fn lengths_no_polygon_can_have_are_refused() {
    // One edge longer than the other two together: no turning angles close it.
    assert_eq!(
        close(&[1.0, 1.0, 3.0], &[2.0; 3]),
        Err(BoundaryError::NotClosed)
    );
}

#[test]
fn input_that_is_not_a_polygon_is_refused() {
    let bad = BoundaryError::BadInput;
    assert_eq!(close(&[1.0, 1.0], &[PI, PI]), Err(bad.clone()));
    assert_eq!(close(&[1.0; 3], &[2.0; 4]), Err(bad.clone()));
    assert_eq!(close(&[1.0, 0.0, 1.0], &[2.0; 3]), Err(bad.clone()));
    assert_eq!(close(&[1.0, -1.0, 1.0], &[2.0; 3]), Err(bad.clone()));
    assert_eq!(close(&[1.0, f64::NAN, 1.0], &[2.0; 3]), Err(bad.clone()));
    assert_eq!(
        close(&[1.0, f64::INFINITY, 1.0], &[2.0; 3]),
        Err(bad.clone())
    );
    assert_eq!(close(&[1.0; 3], &[2.0, f64::INFINITY, 2.0]), Err(bad));
}

#[test]
fn the_gap_is_how_far_the_edges_end_from_where_they_start() {
    use super::gap;
    // A unit square turning by a quarter at each corner closes; losing one turn leaves the
    // last two edges pointing the same way, so the end lands √2 edge lengths from the start.
    let square = [1.0; 4];
    let quarter = std::f64::consts::FRAC_PI_2;
    assert!(gap(&square, &[quarter; 4]) <= 1e-15);
    assert!((gap(&square, &[quarter, quarter, quarter, 0.0]) - 2.0_f64.sqrt()).abs() <= 1e-15);
    // Edge lengths scale the gap.
    assert!(
        (gap(&[3.0; 4], &[quarter, quarter, quarter, 0.0]) - 3.0 * 2.0_f64.sqrt()).abs() <= 1e-14
    );
}
