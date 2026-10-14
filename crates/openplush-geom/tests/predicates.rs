//! Robust predicates against an exact oracle (Constitution IV).
//!
//! Every f64 is a dyadic rational, so each predicate's determinant can be evaluated
//! exactly with big rationals. The oracle shares no code with the implementation.
//! Inputs are built to be nearly degenerate, which is where naive f64 evaluation
//! returns the wrong sign and where meshing code actually breaks.
//!
//! Native only: its dev-dependencies don't build for wasm32 (see Cargo.toml).
#![cfg(not(target_arch = "wasm32"))]

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{Signed, Zero};
use openplush_geom::predicates::{incircle, orient2d, orient3d};
use openplush_geom::{DVec2, DVec3, GeomError, Sign};
use proptest::prelude::*;

/// The exact rational value of a finite f64, decoded from its IEEE-754 bits:
/// (-1)^sign * mantissa * 2^exponent. Independent of any library float conversion.
/// Inputs here are always finite (the generators never produce NaN or infinity).
fn q(x: f64) -> BigRational {
    let bits = x.to_bits();
    let negative = bits >> 63 == 1;
    let biased = ((bits >> 52) & 0x7ff) as i32;
    let fraction = bits & ((1u64 << 52) - 1);
    let (mantissa, exponent) = if biased == 0 {
        (fraction, -1074) // subnormal (or zero)
    } else {
        (fraction | (1u64 << 52), biased - 1075)
    };
    let m = BigInt::from(mantissa);
    let value = if exponent >= 0 {
        BigRational::from_integer(m << exponent.unsigned_abs())
    } else {
        BigRational::new(m, BigInt::from(1) << exponent.unsigned_abs())
    };
    if negative { -value } else { value }
}

fn sign(x: &BigRational) -> Sign {
    if x.is_zero() {
        Sign::Zero
    } else if x.is_positive() {
        Sign::Positive
    } else {
        Sign::Negative
    }
}

fn exact_orient2d(a: DVec2, b: DVec2, c: DVec2) -> Sign {
    let (ax, ay, bx, by, cx, cy) = (q(a.x), q(a.y), q(b.x), q(b.y), q(c.x), q(c.y));
    sign(&((&bx - &ax) * (&cy - &ay) - (&by - &ay) * (&cx - &ax)))
}

fn exact_orient3d(a: DVec3, b: DVec3, c: DVec3, d: DVec3) -> Sign {
    let r = |p: DVec3| [q(p.x) - q(d.x), q(p.y) - q(d.y), q(p.z) - q(d.z)];
    let (u, v, w) = (r(a), r(b), r(c));
    let det = &u[0] * (&v[1] * &w[2] - &v[2] * &w[1]) - &u[1] * (&v[0] * &w[2] - &v[2] * &w[0])
        + &u[2] * (&v[0] * &w[1] - &v[1] * &w[0]);
    sign(&det)
}

fn exact_incircle(a: DVec2, b: DVec2, c: DVec2, d: DVec2) -> Sign {
    let lift = |p: DVec2| {
        let (x, y) = (q(p.x) - q(d.x), q(p.y) - q(d.y));
        let l = &x * &x + &y * &y;
        [x, y, l]
    };
    let (u, v, w) = (lift(a), lift(b), lift(c));
    let det = &u[0] * (&v[1] * &w[2] - &v[2] * &w[1]) - &u[1] * (&v[0] * &w[2] - &v[2] * &w[0])
        + &u[2] * (&v[0] * &w[1] - &v[1] * &w[0]);
    sign(&det)
}

/// Kettner et al. (2008), "Classroom examples of robustness problems": walking a
/// point across a 256 x 256 grid of adjacent doubles near a line through two far
/// points. Naive orient2d gives wrong and inconsistent signs here.
#[test]
fn orient2d_is_exact_on_the_classroom_grid() {
    let b = DVec2::new(12.0, 12.0);
    let c = DVec2::new(24.0, 24.0);
    let ulp = 2f64.powi(-53);
    let mut naive_wrong = 0;
    for i in 0..256 {
        for j in 0..256 {
            let a = DVec2::new(0.5 + f64::from(i) * ulp, 0.5 + f64::from(j) * ulp);
            let want = exact_orient2d(a, b, c);
            assert_eq!(orient2d(a, b, c), Ok(want), "point {i},{j}");
            let naive = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
            let naive_sign = if naive > 0.0 {
                Sign::Positive
            } else if naive < 0.0 {
                Sign::Negative
            } else {
                Sign::Zero
            };
            naive_wrong += usize::from(naive_sign != want);
        }
    }
    // Proves the grid really is adversarial; otherwise the test above proves nothing.
    assert!(
        naive_wrong > 0,
        "naive evaluation was never wrong; the test inputs are too easy"
    );
}

#[test]
fn exact_degeneracies_are_zero() {
    let a = DVec2::new(0.1, 0.1);
    assert_eq!(orient2d(a, a, DVec2::new(3.0, 4.0)), Ok(Sign::Zero));
    let (p, r, s) = (
        DVec2::new(0.0, 0.0),
        DVec2::new(1.0, 0.0),
        DVec2::new(0.0, 1.0),
    );
    assert_eq!(
        incircle(p, r, s, DVec2::new(1.0, 1.0)),
        Ok(Sign::Zero),
        "four corners of a unit square"
    );
    let z = DVec3::ZERO;
    assert_eq!(
        orient3d(DVec3::X, DVec3::Y, DVec3::new(1.0, 1.0, 0.0), z),
        Ok(Sign::Zero)
    );
}

#[test]
fn sign_conventions() {
    let (a, b, c) = (
        DVec2::new(0.0, 0.0),
        DVec2::new(1.0, 0.0),
        DVec2::new(0.0, 1.0),
    );
    assert_eq!(
        orient2d(a, b, c),
        Ok(Sign::Positive),
        "counter-clockwise is positive"
    );
    assert_eq!(orient2d(a, c, b), Ok(Sign::Negative));
    assert_eq!(
        incircle(a, b, c, DVec2::new(0.25, 0.25)),
        Ok(Sign::Positive),
        "inside is positive"
    );
    assert_eq!(incircle(a, b, c, DVec2::new(5.0, 5.0)), Ok(Sign::Negative));
    let below = DVec3::new(0.0, 0.0, -1.0);
    assert_eq!(
        orient3d(DVec3::ZERO, DVec3::X, DVec3::Y, below),
        Ok(Sign::Positive),
        "below a ccw triangle is positive"
    );
}

#[test]
fn non_finite_input_is_an_error_not_a_sign() {
    // Every argument, every component, every kind of non-finite value: a check
    // dropped for any one of them must fail here.
    let base2 = [DVec2::ZERO, DVec2::X, DVec2::Y, DVec2::new(0.25, 0.25)];
    let base3 = [DVec3::ZERO, DVec3::X, DVec3::Y, DVec3::new(0.2, 0.2, 0.2)];
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for arg in 0..4 {
            for comp in 0..3 {
                let mut p2 = base2;
                if comp < 2 {
                    let q = &mut p2[arg];
                    if comp == 0 {
                        q.x = bad
                    } else {
                        q.y = bad
                    }
                    if arg < 3 {
                        assert_eq!(orient2d(p2[0], p2[1], p2[2]), Err(GeomError::NonFinite));
                    }
                    assert_eq!(
                        incircle(p2[0], p2[1], p2[2], p2[3]),
                        Err(GeomError::NonFinite)
                    );
                }
                let mut p3 = base3;
                let q = &mut p3[arg];
                match comp {
                    0 => q.x = bad,
                    1 => q.y = bad,
                    _ => q.z = bad,
                }
                assert_eq!(
                    orient3d(p3[0], p3[1], p3[2], p3[3]),
                    Err(GeomError::NonFinite)
                );
            }
        }
    }
}

/// 2^k, exactly, for k in [-1022, 1023].
fn pow2(k: i32) -> f64 {
    f64::from_bits(((k + 1023) as u64) << 52)
}

#[test]
fn finite_inputs_that_overflow_or_underflow_inside_the_predicate_still_get_the_exact_sign() {
    // Regressions from review of #38: robust alone returns NaN (read as Zero) for
    // the first two and underflows to 0 for the third. Every exact sign is Positive.
    let (a, b, c, d) = (
        DVec2::new(1e200, 0.0),
        DVec2::new(0.0, 1e200),
        DVec2::new(-1e200, 0.0),
        DVec2::ZERO,
    );
    assert_eq!(exact_incircle(a, b, c, d), Sign::Positive);
    assert_eq!(incircle(a, b, c, d), Ok(Sign::Positive));

    let (a, b, c, d) = (
        DVec3::ZERO,
        DVec3::new(1e110, 0.0, 0.0),
        DVec3::new(0.0, 1e110, 0.0),
        DVec3::new(0.0, 0.0, -1e110),
    );
    assert_eq!(exact_orient3d(a, b, c, d), Sign::Positive);
    assert_eq!(orient3d(a, b, c, d), Ok(Sign::Positive));

    let (a, b, c) = (
        DVec2::ZERO,
        DVec2::new(1e-200, 0.0),
        DVec2::new(0.0, 1e-200),
    );
    assert_eq!(exact_orient2d(a, b, c), Sign::Positive);
    assert_eq!(orient2d(a, b, c), Ok(Sign::Positive));

    // All-subnormal input is exact too.
    let t = f64::MIN_POSITIVE / 8.0;
    assert_eq!(
        orient2d(DVec2::ZERO, DVec2::new(t, 0.0), DVec2::new(0.0, t)),
        Ok(Sign::Positive)
    );
    assert_eq!(
        orient2d(DVec2::ZERO, DVec2::ZERO, DVec2::ZERO),
        Ok(Sign::Zero)
    );
}

#[test]
fn coordinates_spanning_more_than_2_pow_150_are_refused_not_guessed() {
    // Relative to the largest |coordinate|, a nonzero one below 2^-150 could
    // underflow inside the adaptive arithmetic, so the sign is not guaranteed.
    let (a, b) = (DVec2::new(1.0, 0.0), DVec2::new(0.0, 1.0));
    let edge = DVec2::new(pow2(-149), pow2(-149));
    assert_eq!(orient2d(a, b, edge), Ok(exact_orient2d(a, b, edge)));
    let below = DVec2::new(pow2(-160), 0.0);
    assert_eq!(orient2d(a, b, below), Err(GeomError::UnsupportedMagnitude));
    assert_eq!(
        orient3d(
            DVec3::X,
            DVec3::Y,
            DVec3::Z,
            DVec3::new(pow2(-160), 0.0, 0.0)
        ),
        Err(GeomError::UnsupportedMagnitude)
    );
}

fn coord() -> impl Strategy<Value = f64> {
    -1e3f64..1e3
}

/// A point within a few ULPs of the segment a-b: the hard case.
fn near_line(a: DVec2, b: DVec2, t: f64, wiggle: (i32, i32)) -> DVec2 {
    let p = a + (b - a) * t;
    let nudge = |x: f64, k: i32| {
        let mut v = x;
        for _ in 0..k.unsigned_abs() {
            v = if k > 0 { v.next_up() } else { v.next_down() };
        }
        v
    };
    DVec2::new(nudge(p.x, wiggle.0), nudge(p.y, wiggle.1))
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 2_000, rng_seed: proptest::test_runner::RngSeed::Fixed(20261014), ..ProptestConfig::default() })]

    #[test]
    fn orient2d_matches_exact_near_collinear(ax in coord(), ay in coord(), bx in coord(), by in coord(),
                                             t in -2.0f64..3.0, wx in -4i32..=4, wy in -4i32..=4) {
        let (a, b) = (DVec2::new(ax, ay), DVec2::new(bx, by));
        let c = near_line(a, b, t, (wx, wy));
        prop_assert_eq!(orient2d(a, b, c), Ok(exact_orient2d(a, b, c)));
    }

    #[test]
    fn incircle_matches_exact_near_cocircular(cx in coord(), cy in coord(), r in 1e-2f64..1e3,
                                              t in proptest::array::uniform4(0.0f64..std::f64::consts::TAU),
                                              w in -3i32..=3) {
        let on = |angle: f64| DVec2::new(cx + r * angle.cos(), cy + r * angle.sin());
        // Move d by |w| ULPs (0 leaves it as rounded onto the circle).
        let d = near_line(on(t[3]), on(t[3]), 0.0, (w, 0));
        let (a, b, c) = (on(t[0]), on(t[1]), on(t[2]));
        prop_assert_eq!(incircle(a, b, c, d), Ok(exact_incircle(a, b, c, d)));
    }

    #[test]
    fn orient3d_matches_exact_near_coplanar(p in proptest::array::uniform3(proptest::array::uniform3(coord())),
                                            s in -2.0f64..3.0, t in -2.0f64..3.0, k in -4i32..=4) {
        let [a, b, c] = p.map(|v| DVec3::new(v[0], v[1], v[2]));
        let mut d = a + (b - a) * s + (c - a) * t;
        for _ in 0..k.unsigned_abs() {
            d.z = if k > 0 { d.z.next_up() } else { d.z.next_down() };
        }
        prop_assert_eq!(orient3d(a, b, c, d), Ok(exact_orient3d(a, b, c, d)));
    }
}

#[test]
fn oracle_self_check() {
    // The oracle must itself agree with obvious cases, or the property tests above are
    // comparing against nonsense.
    assert_eq!(
        exact_orient2d(DVec2::ZERO, DVec2::X, DVec2::Y),
        Sign::Positive
    );
    assert_eq!(
        exact_incircle(DVec2::ZERO, DVec2::X, DVec2::Y, DVec2::new(0.25, 0.25)),
        Sign::Positive
    );
    assert_eq!(
        exact_orient3d(DVec3::ZERO, DVec3::X, DVec3::Y, DVec3::new(0.0, 0.0, -1.0)),
        Sign::Positive
    );
    let _ = BigInt::zero();
}

/// Deterministic pseudo-random sampler (no extra dependency): SplitMix64.
struct Sample(u64);
impl Sample {
    fn next_f64(&mut self, lo: f64, hi: f64) -> f64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        lo + (hi - lo) * ((z >> 11) as f64 / (1u64 << 53) as f64)
    }
}

/// The incircle and orient3d property tests are only meaningful if their inputs
/// defeat naive evaluation. Check that, with the same construction, naive f64 gets
/// the sign wrong at least sometimes.
#[test]
fn near_degenerate_generators_defeat_naive_evaluation() {
    let mut s = Sample(7);
    let (mut incircle_wrong, mut orient3d_wrong) = (0, 0);
    for _ in 0..3_000 {
        let (cx, cy, r) = (
            s.next_f64(-1e3, 1e3),
            s.next_f64(-1e3, 1e3),
            s.next_f64(1e-2, 1e3),
        );
        let on = |t: f64| DVec2::new(cx + r * t.cos(), cy + r * t.sin());
        let tau = std::f64::consts::TAU;
        let (a, b, c) = (
            on(s.next_f64(0.0, tau)),
            on(s.next_f64(0.0, tau)),
            on(s.next_f64(0.0, tau)),
        );
        let d = on(s.next_f64(0.0, tau));
        let lift = |p: DVec2| {
            let (x, y) = (p.x - d.x, p.y - d.y);
            [x, y, x * x + y * y]
        };
        let (u, v, w) = (lift(a), lift(b), lift(c));
        let naive = u[0] * (v[1] * w[2] - v[2] * w[1]) - u[1] * (v[0] * w[2] - v[2] * w[0])
            + u[2] * (v[0] * w[1] - v[1] * w[0]);
        incircle_wrong += usize::from(Sign::of_f64(naive) != exact_incircle(a, b, c, d));

        let p = |s: &mut Sample| {
            DVec3::new(
                s.next_f64(-1e3, 1e3),
                s.next_f64(-1e3, 1e3),
                s.next_f64(-1e3, 1e3),
            )
        };
        let (a3, b3, c3) = (p(&mut s), p(&mut s), p(&mut s));
        let d3 = a3 + (b3 - a3) * s.next_f64(-2.0, 3.0) + (c3 - a3) * s.next_f64(-2.0, 3.0);
        let r3 = |q: DVec3| q - d3;
        let (u3, v3, w3) = (r3(a3), r3(b3), r3(c3));
        let naive3 = u3.dot(v3.cross(w3));
        orient3d_wrong += usize::from(Sign::of_f64(naive3) != exact_orient3d(a3, b3, c3, d3));
    }
    assert!(
        incircle_wrong > 0,
        "naive incircle was never wrong: the generator is too easy"
    );
    assert!(
        orient3d_wrong > 0,
        "naive orient3d was never wrong: the generator is too easy"
    );
}

trait SignOf {
    fn of_f64(x: f64) -> Sign;
}
impl SignOf for Sign {
    fn of_f64(x: f64) -> Sign {
        if x > 0.0 {
            Sign::Positive
        } else if x < 0.0 {
            Sign::Negative
        } else {
            Sign::Zero
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 500, rng_seed: proptest::test_runner::RngSeed::Fixed(20261015), ..ProptestConfig::default() })]

    /// Scaling by 2^k is exact and the predicates are homogeneous, so the exact sign
    /// must survive any k for which the inputs stay normal, including scales where
    /// robust alone would overflow (k > 0) or underflow (k < 0).
    #[test]
    fn predicates_stay_exact_at_any_power_of_two_scale(ax in coord(), ay in coord(), bx in coord(), by in coord(),
                                                       t in -2.0f64..3.0, wx in -4i32..=4, wy in -4i32..=4,
                                                       z in proptest::array::uniform3(coord()), k in -700i32..=700) {
        let s = pow2(k);
        let (a, b) = (DVec2::new(ax, ay) * s, DVec2::new(bx, by) * s);
        let c = near_line(a, b, t, (wx, wy));
        prop_assert_eq!(orient2d(a, b, c), Ok(exact_orient2d(a, b, c)));
        let d = near_line(a, c, 0.5, (wy, wx));
        prop_assert_eq!(incircle(a, b, c, d), Ok(exact_incircle(a, b, c, d)));
        let lift = |p: DVec2, h: f64| DVec3::new(p.x, p.y, h * s);
        let (a3, b3, c3) = (lift(a, z[0]), lift(b, z[1]), lift(c, z[2]));
        let d3 = lift(near_line(a, b, 0.5, (wx, 0)), (z[0] + z[1]) * 0.5);
        prop_assert_eq!(orient3d(a3, b3, c3, d3), Ok(exact_orient3d(a3, b3, c3, d3)));
    }
}
