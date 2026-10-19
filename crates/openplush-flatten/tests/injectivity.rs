//! `injectivity::verify`: every flat face strictly counter-clockwise, exact predicate.

use openplush_flatten::FlattenError;
use openplush_flatten::injectivity::verify;
use openplush_geom::{DVec2, GeomError};
use openplush_mesh::FaceId;
use openplush_seams::ChartId;

type Tri = (FaceId, [DVec2; 3]);

const CHART: ChartId = ChartId::new(7);

fn tri(face: u32, p: [(f64, f64); 3]) -> Tri {
    (FaceId::new(face), p.map(|(x, y)| DVec2::new(x, y)))
}

fn ccw(face: u32) -> Tri {
    tri(face, [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)])
}

fn cw(face: u32) -> Tri {
    tri(face, [(0.0, 0.0), (0.0, 1.0), (1.0, 0.0)])
}

#[test]
fn all_counter_clockwise_faces_pass() {
    assert_eq!(verify(CHART, &[ccw(0), ccw(1), ccw(2)]), Ok(()));
}

#[test]
fn every_clockwise_face_is_listed_in_input_order() {
    let faces = [ccw(0), cw(5), ccw(1), cw(3)];
    assert_eq!(
        verify(CHART, &faces),
        Err(FlattenError::Flip {
            chart: CHART,
            faces: vec![FaceId::new(5), FaceId::new(3)],
        })
    );
}

#[test]
fn a_collinear_face_is_a_flip() {
    let flat = tri(4, [(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)]);
    assert_eq!(
        verify(CHART, &[ccw(0), flat]),
        Err(FlattenError::Flip {
            chart: CHART,
            faces: vec![FaceId::new(4)]
        })
    );
}

#[test]
fn a_repeated_corner_is_a_flip() {
    let point = tri(2, [(1.0, 1.0), (1.0, 1.0), (3.0, 0.0)]);
    assert_eq!(
        verify(CHART, &[point]),
        Err(FlattenError::Flip {
            chart: CHART,
            faces: vec![FaceId::new(2)]
        })
    );
}

// Exactly counter-clockwise, yet the naive f64 determinant is 0.0.
const SLIVER: [(f64, f64); 3] = [(0.5, 0.500_000_000_000_000_1), (12.0, 12.0), (24.0, 24.0)];

fn naive_det([a, b, c]: [(f64, f64); 3]) -> f64 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

#[test]
fn a_sliver_the_naive_determinant_rounds_to_zero_still_passes() {
    assert_eq!(
        naive_det(SLIVER),
        0.0,
        "premise: naive arithmetic cannot see it"
    );
    assert_eq!(verify(CHART, &[tri(0, SLIVER)]), Ok(()));
}

#[test]
fn a_mirrored_sliver_is_a_flip() {
    let [a, b, c] = SLIVER;
    assert_eq!(
        verify(CHART, &[tri(0, [a, c, b])]),
        Err(FlattenError::Flip {
            chart: CHART,
            faces: vec![FaceId::new(0)]
        })
    );
}

#[test]
fn no_faces_is_refused_not_passed() {
    assert_eq!(
        verify(CHART, &[]),
        Err(FlattenError::EmptyChart { chart: CHART })
    );
}

#[test]
fn a_non_finite_coordinate_is_unjudgeable_and_names_its_face() {
    for bad in [f64::NAN, f64::INFINITY] {
        let faces = [ccw(0), tri(9, [(0.0, 0.0), (bad, 0.0), (0.0, 1.0)])];
        assert_eq!(
            verify(CHART, &faces),
            Err(FlattenError::Unjudgeable {
                chart: CHART,
                face: FaceId::new(9),
                source: GeomError::NonFinite,
            })
        );
    }
}

#[test]
fn coordinates_spanning_too_many_magnitudes_are_unjudgeable() {
    let faces = [tri(1, [(0.0, 0.0), (1e300, 0.0), (1e-300, 1.0)])];
    assert_eq!(
        verify(CHART, &faces),
        Err(FlattenError::Unjudgeable {
            chart: CHART,
            face: FaceId::new(1),
            source: GeomError::UnsupportedMagnitude,
        })
    );
}

#[test]
fn an_unjudgeable_face_wins_over_an_earlier_flip() {
    let faces = [cw(0), tri(1, [(f64::NAN, 0.0), (1.0, 0.0), (0.0, 1.0)])];
    assert!(matches!(
        verify(CHART, &faces),
        Err(FlattenError::Unjudgeable { .. })
    ));
}

#[test]
fn translating_or_scaling_a_chart_does_not_change_the_verdict() {
    for (s, dx) in [(1e-6, 0.0), (1e6, 1e9), (1.0, -1e12)] {
        let f = |t: Tri| -> Tri { (t.0, t.1.map(|p| DVec2::new(p.x * s + dx, p.y * s + dx))) };
        assert_eq!(verify(CHART, &[f(ccw(0))]), Ok(()), "scale {s}, shift {dx}");
        assert!(verify(CHART, &[f(cw(0))]).is_err(), "scale {s}, shift {dx}");
    }
}
