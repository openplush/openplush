//! `parity::check`: both sides of a seam within max(1.0 mm, 0.5% of the longer side).

use openplush_geom::GeomError;
use openplush_geom::units::Mm;
use openplush_pattern::PatternError;
use openplush_pattern::parity::check;
use openplush_seams::SeamChainId;

const CHAIN: SeamChainId = SeamChainId::new(3);

fn verdict(a: f64, b: f64) -> Result<Result<(), PatternError>, GeomError> {
    Ok(check(CHAIN, Mm::try_new(a)?, Mm::try_new(b)?))
}

fn refused(a: f64, b: f64, tolerance: f64) -> Result<Result<(), PatternError>, GeomError> {
    Ok(Err(PatternError::SeamParity {
        chain: CHAIN,
        a: Mm::try_new(a)?,
        b: Mm::try_new(b)?,
        tolerance: Mm::try_new(tolerance)?,
    }))
}

#[test]
fn equal_sides_pass() {
    assert_eq!(verdict(120.0, 120.0), Ok(Ok(())));
}

#[test]
fn zero_length_sides_pass() {
    assert_eq!(verdict(0.0, 0.0), Ok(Ok(())));
}

#[test]
fn a_short_seam_gets_the_one_millimetre_floor() {
    assert_eq!(verdict(100.0, 101.0), Ok(Ok(())));
    assert_eq!(verdict(100.0, 101.5), refused(100.0, 101.5, 1.0));
}

#[test]
fn a_long_seam_gets_half_a_percent() {
    assert_eq!(verdict(1000.0, 1005.0), Ok(Ok(())));
    assert_eq!(
        verdict(1000.0, 1010.0),
        refused(1000.0, 1010.0, 1010.0 * 0.005)
    );
}

#[test]
fn the_tolerance_comes_from_the_longer_side() {
    // 5.01 mm apart: over 0.5% of the shorter side (5.0), under 0.5% of the longer (5.02505).
    assert_eq!(verdict(1000.0, 1005.01), Ok(Ok(())));
}

#[test]
fn the_verdict_does_not_depend_on_which_side_is_which() {
    assert_eq!(verdict(1005.01, 1000.0), Ok(Ok(())));
    assert_eq!(verdict(101.5, 100.0), refused(101.5, 100.0, 1.0));
}

#[test]
fn a_difference_just_past_the_floor_is_refused() {
    assert_eq!(verdict(10.0, 11.0), Ok(Ok(())));
    assert!(matches!(
        verdict(10.0, 11.0 + 1e-9),
        Ok(Err(PatternError::SeamParity { .. }))
    ));
}
