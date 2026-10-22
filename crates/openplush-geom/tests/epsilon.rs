//! The tolerance policy: model-space epsilons scale with
//! the bounding-box diagonal; physical tolerances are absolute millimetres; the two
//! meet only through a named conversion (AGENTS.md, units).

use openplush_geom::epsilon::{
    FOLD_ANGLE, MAX_DIAGONAL, MIN_DIAGONAL, ModelTolerances, SeamParity, UNIT_QUATERNION_TOLERANCE,
    UV_CUT,
};
use openplush_geom::units::{Mm, Scale};
use openplush_geom::{DVec2, GeomError};

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
    // Outside [MIN_DIAGONAL, MAX_DIAGONAL] the area tolerance underflows to zero or
    // a subnormal, or cross products of coordinates overflow.
    for diagonal in [
        0.0,
        -1.0,
        f64::NAN,
        f64::INFINITY,
        f64::MIN_POSITIVE / 2.0,
        f64::MIN_POSITIVE,
        1e-160,
        MIN_DIAGONAL / 2.0,
        MAX_DIAGONAL * 2.0,
        1e300,
        f64::MAX,
    ] {
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
fn tolerances_at_the_ends_of_the_diagonal_band_are_normal_numbers() {
    for diagonal in [MIN_DIAGONAL, MAX_DIAGONAL] {
        let t = ModelTolerances::for_diagonal(diagonal).expect("band edge is valid");
        assert!(
            t.weld().is_normal() && t.area().is_normal(),
            "at {diagonal}"
        );
        // A cross product of two edge vectors spanning the whole model stays finite.
        assert!((2.0 * diagonal * diagonal).is_finite());
    }
}

fn mm(x: f64) -> Result<Mm, GeomError> {
    Mm::try_new(x)
}

#[test]
fn seam_parity_is_the_larger_of_one_millimetre_and_half_a_percent() -> Result<(), GeomError> {
    let parity = SeamParity::DEFAULT;
    assert_eq!(parity.allowed(mm(100.0)?), mm(1.0)?);
    assert_eq!(parity.allowed(mm(200.0)?), mm(1.0)?);
    assert_eq!(parity.allowed(mm(400.0)?), mm(2.0)?);
    assert!(parity.within(mm(400.0)?, mm(401.9)?));
    assert!(!parity.within(mm(400.0)?, mm(402.1)?));
    assert!(parity.within(mm(50.0)?, mm(49.0)?));
    assert!(!parity.within(mm(50.0)?, mm(48.9)?));
    Ok(())
}

#[test]
fn seam_parity_measures_against_the_longer_side() -> Result<(), GeomError> {
    // 2.005 mm apart: inside 0.5% of 402.005 (2.010) but outside 0.5% of 400 (2.0).
    let parity = SeamParity::DEFAULT;
    assert!(parity.within(mm(400.0)?, mm(402.005)?));
    assert!(parity.within(mm(402.005)?, mm(400.0)?));
    Ok(())
}

#[test]
fn a_length_is_finite_and_not_negative() {
    for bad in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -1.0,
        -f64::MIN_POSITIVE,
    ] {
        assert!(
            matches!(Mm::try_new(bad), Err(GeomError::InvalidLength { .. })),
            "length {bad} must be rejected"
        );
    }
    assert_eq!(Mm::try_new(0.0).map(Mm::get), Ok(0.0));
}

#[test]
fn model_lengths_become_millimetres_through_a_scale() -> Result<(), GeomError> {
    let scale = Scale::mm_per_model_unit(25.4).expect("inches to mm");
    assert_eq!(scale.to_mm(2.0), Ok(mm(50.8)?));
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::MIN_POSITIVE / 2.0] {
        assert!(
            matches!(
                Scale::mm_per_model_unit(bad),
                Err(GeomError::InvalidScale { .. })
            ),
            "scale {bad} must be rejected"
        );
    }
    Ok(())
}

#[test]
fn a_conversion_that_overflows_or_goes_negative_is_an_error() {
    let scale = Scale::mm_per_model_unit(1e300).expect("large but valid");
    assert!(matches!(
        scale.to_mm(1e300),
        Err(GeomError::InvalidLength { .. })
    ));
    let unit = Scale::mm_per_model_unit(1.0).expect("valid");
    assert!(matches!(
        unit.to_mm(-1.0),
        Err(GeomError::InvalidLength { .. })
    ));
    assert!(matches!(
        unit.to_mm(f64::NAN),
        Err(GeomError::InvalidLength { .. })
    ));
}

#[test]
fn a_flat_point_becomes_millimetres_through_a_scale_whatever_its_sign() -> Result<(), GeomError> {
    let scale = Scale::mm_per_model_unit(100.0)?;
    assert_eq!(
        scale.point_to_mm(DVec2::new(1.5, -0.25)),
        DVec2::new(150.0, -25.0)
    );
    let huge = Scale::mm_per_model_unit(1e300)?;
    assert!(!huge.point_to_mm(DVec2::new(1e300, 0.0)).is_finite());
    Ok(())
}

#[test]
fn uv_cut_is_1e_minus_5_between_f32_noise_and_the_tightest_measured_cut() {
    assert_eq!(UV_CUT, 1e-5);
    // f32 spacing at the top of a UDIM tile row (u near 10) is below the tolerance
    // by a factor of ten, so bit-level export noise never reads as a cut.
    let spacing_at_ten = f64::from(f32::from_bits(10.0_f32.to_bits() + 1) - 10.0_f32);
    assert!(
        std::hint::black_box(UV_CUT) > 10.0 * spacing_at_ten,
        "{spacing_at_ten}"
    );
    // The tightest cut measured on Blender 5.2 UV spheres, a slit near its tip, was 1.81e-4.
    let tightest_cut = std::hint::black_box(1.81e-4);
    assert!(UV_CUT * 18.0 <= tightest_cut);
}

#[test]
fn fold_angle_is_five_degrees_between_a_closed_dart_and_a_wrapped_cap() {
    assert_eq!(FOLD_ANGLE, 5.0_f64.to_radians());
    // The sharpest crease a plush carries, a closed dart edge, leaves its faces about 10°
    // apart; a sphere cap wrapped past its pole (S7) leaves them under 0.12° from antiparallel.
    let closed_dart = std::hint::black_box(10.0_f64.to_radians());
    let wrapped_cap = std::hint::black_box(0.12_f64.to_radians());
    assert!(FOLD_ANGLE * 2.0 <= closed_dart);
    assert!(FOLD_ANGLE > wrapped_cap * 40.0);
}

#[test]
fn unit_quaternion_tolerance_is_1e_minus_6_above_f32_normalisation_rounding() {
    assert_eq!(UNIT_QUATERNION_TOLERANCE, 1e-6);
    // A writer that normalises in f32 leaves |q|² a few ulps of 1.0 off; four is 4.8e-7.
    assert!(std::hint::black_box(UNIT_QUATERNION_TOLERANCE) > 4.0 * f64::from(f32::EPSILON));
}
