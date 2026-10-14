//! The tolerance policy of data-model section 11: model-space epsilons scale with
//! the bounding-box diagonal; physical tolerances are absolute millimetres; the two
//! meet only through a named conversion (AGENTS.md, units).

use openplush_geom::GeomError;
use openplush_geom::epsilon::{ModelTolerances, SeamParity};
use openplush_geom::units::{Mm, Scale};

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-15 * b.abs()
}

#[test]
fn model_tolerances_scale_with_the_diagonal() {
    for diagonal in [1e-3, 1.0, 10.0, 2_000.0] {
        let t = ModelTolerances::for_diagonal(diagonal).expect("valid diagonal");
        assert!(close(t.weld(), 1e-6 * diagonal), "weld at {diagonal}");
        assert!(
            close(t.area(), 1e-12 * diagonal * diagonal),
            "area at {diagonal}"
        );
        assert_eq!(t.diagonal(), diagonal);
    }
}

#[test]
fn jacobian_threshold_is_dimensionless() {
    // det J compares two areas, so its threshold must not change with scale.
    let small = ModelTolerances::for_diagonal(1e-3).expect("valid");
    let large = ModelTolerances::for_diagonal(1e3).expect("valid");
    assert_eq!(small.min_det_j(), 1e-10);
    assert_eq!(large.min_det_j(), 1e-10);
}

#[test]
fn degenerate_diagonals_are_a_typed_error_not_a_clamp() {
    for diagonal in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::MIN_POSITIVE / 2.0] {
        assert!(
            matches!(
                ModelTolerances::for_diagonal(diagonal),
                Err(GeomError::InvalidDiagonal { .. })
            ),
            "diagonal {diagonal} must be rejected"
        );
    }
}

#[test]
fn seam_parity_is_the_larger_of_one_millimetre_and_half_a_percent() {
    let parity = SeamParity::DEFAULT;
    assert_eq!(parity.allowed(Mm::new(100.0)), Mm::new(1.0));
    assert_eq!(parity.allowed(Mm::new(200.0)), Mm::new(1.0));
    assert_eq!(parity.allowed(Mm::new(400.0)), Mm::new(2.0));
    assert!(parity.within(Mm::new(400.0), Mm::new(401.9)));
    assert!(!parity.within(Mm::new(400.0), Mm::new(402.1)));
    assert!(parity.within(Mm::new(50.0), Mm::new(49.0)));
    assert!(!parity.within(Mm::new(50.0), Mm::new(48.9)));
}

#[test]
fn model_lengths_become_millimetres_only_through_a_scale() {
    let scale = Scale::mm_per_model_unit(25.4).expect("inches to mm");
    assert_eq!(scale.to_mm(2.0), Mm::new(50.8));
    assert!(Scale::mm_per_model_unit(0.0).is_err());
    assert!(Scale::mm_per_model_unit(f64::NAN).is_err());
    assert!(Scale::mm_per_model_unit(-1.0).is_err());
}
