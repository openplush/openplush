//! `Outline::new`: closed, simple, counter-clockwise, judged by exact predicates.

use openplush_geom::{DVec2, GeomError};
use openplush_pattern::{Outline, PatternError};

fn pts(p: &[(f64, f64)]) -> Vec<DVec2> {
    p.iter().map(|&(x, y)| DVec2::new(x, y)).collect()
}

fn refused(p: &[(f64, f64)]) -> Result<Outline, PatternError> {
    Outline::new(pts(p))
}

fn not_simple(edge: usize, other: usize) -> Result<Outline, PatternError> {
    Err(PatternError::OutlineNotSimple { edge, other })
}

const SQUARE: [(f64, f64); 4] = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];

#[test]
fn a_counter_clockwise_square_is_kept_as_given() {
    let o = refused(&SQUARE);
    assert_eq!(o.as_ref().map(|o| o.points().to_vec()), Ok(pts(&SQUARE)));
}

#[test]
fn a_clockwise_square_is_refused_not_reversed() {
    let cw = [(0.0, 0.0), (0.0, 10.0), (10.0, 10.0), (10.0, 0.0)];
    assert_eq!(refused(&cw), Err(PatternError::OutlineClockwise));
}

#[test]
fn a_concave_outline_is_judged_by_its_winding_not_its_first_corner() {
    let l = [
        (0.0, 0.0),
        (6.0, 0.0),
        (6.0, 2.0),
        (2.0, 2.0),
        (2.0, 6.0),
        (0.0, 6.0),
    ];
    assert!(refused(&l).is_ok());
    let mut rev = l;
    rev.reverse();
    assert_eq!(refused(&rev), Err(PatternError::OutlineClockwise));
}

#[test]
fn fewer_than_three_points_is_too_short() {
    for n in 0..3 {
        assert_eq!(
            refused(&SQUARE[..n]),
            Err(PatternError::OutlineTooShort { points: n })
        );
    }
}

#[test]
fn a_non_finite_coordinate_names_its_point() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let p = [(0.0, 0.0), (10.0, 0.0), (bad, 10.0), (0.0, 10.0)];
        assert_eq!(
            refused(&p),
            Err(PatternError::OutlineNotFinite { point: 2 })
        );
    }
}

#[test]
fn a_repeated_point_is_not_simple_and_names_its_edge() {
    let p = [
        (0.0, 0.0),
        (10.0, 0.0),
        (10.0, 0.0),
        (10.0, 10.0),
        (0.0, 10.0),
    ];
    assert_eq!(refused(&p), not_simple(1, 1));
}

#[test]
fn a_closing_duplicate_of_the_first_point_is_not_simple() {
    let p = [
        (0.0, 0.0),
        (10.0, 0.0),
        (10.0, 10.0),
        (0.0, 10.0),
        (0.0, 0.0),
    ];
    assert_eq!(refused(&p), not_simple(4, 4));
}

#[test]
fn crossing_edges_are_not_simple() {
    let bowtie = [(0.0, 0.0), (10.0, 10.0), (10.0, 0.0), (0.0, 10.0)];
    assert_eq!(refused(&bowtie), not_simple(0, 2));
}

#[test]
fn a_vertex_touching_another_edge_is_not_simple() {
    let p = [
        (0.0, 0.0),
        (10.0, 0.0),
        (10.0, 10.0),
        (5.0, 0.0),
        (0.0, 10.0),
    ];
    assert_eq!(refused(&p), not_simple(0, 3));
}

#[test]
fn a_spike_that_doubles_back_is_not_simple() {
    let p = [
        (0.0, 0.0),
        (10.0, 0.0),
        (15.0, 0.0),
        (10.0, 0.0),
        (10.0, 10.0),
        (0.0, 10.0),
    ];
    assert!(matches!(
        refused(&p),
        Err(PatternError::OutlineNotSimple { .. })
    ));
}

#[test]
fn three_collinear_points_enclose_nothing() {
    let p = [(0.0, 0.0), (5.0, 0.0), (10.0, 0.0)];
    assert!(matches!(
        refused(&p),
        Err(PatternError::OutlineNotSimple { .. })
    ));
}

#[test]
fn a_straight_run_through_an_extra_point_is_fine() {
    let p = [
        (0.0, 0.0),
        (5.0, 0.0),
        (10.0, 0.0),
        (10.0, 10.0),
        (0.0, 10.0),
    ];
    assert!(refused(&p).is_ok());
}

#[test]
fn a_sliver_naive_arithmetic_calls_flat_is_still_accepted() {
    // Exactly counter-clockwise, naive f64 determinant 0.0.
    let p = [(0.5, 0.500_000_000_000_000_1), (12.0, 12.0), (24.0, 24.0)];
    assert!(refused(&p).is_ok());
}

#[test]
fn coordinates_spanning_too_many_magnitudes_are_unjudgeable() {
    let p = [(0.0, 0.0), (1e300, 0.0), (1e-300, 1.0)];
    assert_eq!(
        refused(&p),
        Err(PatternError::OutlineUnjudgeable {
            source: GeomError::UnsupportedMagnitude
        })
    );
}

#[test]
fn scale_and_shift_do_not_change_the_verdict() {
    for (s, d) in [(1e-3, 0.0), (1e3, 1e6), (1.0, -1e9)] {
        let f = |p: &[(f64, f64)]| -> Vec<(f64, f64)> {
            p.iter().map(|&(x, y)| (x * s + d, y * s + d)).collect()
        };
        assert!(refused(&f(&SQUARE)).is_ok(), "scale {s}, shift {d}");
        let cw = [(0.0, 0.0), (0.0, 10.0), (10.0, 10.0), (10.0, 0.0)];
        assert_eq!(
            refused(&f(&cw)),
            Err(PatternError::OutlineClockwise),
            "scale {s}, shift {d}"
        );
    }
}
