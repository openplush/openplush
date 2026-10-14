//! `DVec2` / `DVec3` arithmetic: exact wherever IEEE-754 is exact, and no silent
//! fix-ups (a zero or non-finite vector does not normalise to something plausible).

use openplush_geom::{DVec2, DVec3};

#[test]
fn componentwise_arithmetic_is_exact() {
    let a = DVec3::new(1.0, 2.0, 3.0);
    let b = DVec3::new(0.5, -4.0, 8.0);
    assert_eq!(a + b, DVec3::new(1.5, -2.0, 11.0));
    assert_eq!(a - b, DVec3::new(0.5, 6.0, -5.0));
    assert_eq!(a * 2.0, DVec3::new(2.0, 4.0, 6.0));
    assert_eq!(a / 2.0, DVec3::new(0.5, 1.0, 1.5));
    assert_eq!(-a, DVec3::new(-1.0, -2.0, -3.0));
    assert_eq!(
        DVec2::new(1.0, 2.0) + DVec2::new(3.0, -5.0),
        DVec2::new(4.0, -3.0)
    );
}

#[test]
fn dot_and_cross_follow_the_right_hand_rule() {
    assert_eq!(DVec3::X.cross(DVec3::Y), DVec3::Z);
    assert_eq!(DVec3::Y.cross(DVec3::Z), DVec3::X);
    assert_eq!(DVec3::Y.cross(DVec3::X), -DVec3::Z);
    let a = DVec3::new(2.0, -3.0, 5.0);
    let b = DVec3::new(-1.0, 4.0, 7.0);
    assert_eq!(a.dot(b), -2.0 - 12.0 + 35.0);
    // Small integers: every product is exact, so these hold bit for bit.
    assert_eq!(a.cross(b), -b.cross(a));
    assert_eq!(a.dot(a.cross(b)), 0.0);
    assert_eq!(DVec2::new(1.0, 0.0).perp_dot(DVec2::new(0.0, 1.0)), 1.0);
    assert_eq!(DVec2::new(0.0, 1.0).perp_dot(DVec2::new(1.0, 0.0)), -1.0);
}

#[test]
fn length() {
    assert_eq!(DVec3::new(3.0, 4.0, 0.0).length(), 5.0);
    assert_eq!(DVec3::new(2.0, 3.0, 6.0).length_squared(), 49.0);
    assert_eq!(DVec2::new(6.0, 8.0).length(), 10.0);
}

#[test]
fn normalize_refuses_zero_and_non_finite_vectors() {
    assert_eq!(DVec3::ZERO.normalize(), None);
    assert_eq!(DVec3::new(f64::NAN, 0.0, 1.0).normalize(), None);
    assert_eq!(DVec3::new(f64::INFINITY, 0.0, 0.0).normalize(), None);
    assert_eq!(DVec2::ZERO.normalize(), None);
    let n = DVec3::new(0.0, 0.0, 1e-300).normalize();
    assert_eq!(
        n,
        Some(DVec3::Z),
        "a tiny but non-zero vector still has a direction"
    );
    let u = DVec3::new(3.0, 0.0, 4.0).normalize();
    assert_eq!(u, Some(DVec3::new(0.6, 0.0, 0.8)));
}

#[test]
fn finiteness() {
    assert!(DVec3::new(1.0, 2.0, 3.0).is_finite());
    assert!(!DVec3::new(1.0, f64::NAN, 3.0).is_finite());
    assert!(!DVec2::new(f64::NEG_INFINITY, 0.0).is_finite());
}
