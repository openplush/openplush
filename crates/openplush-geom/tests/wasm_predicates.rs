//! The predicates on wasm32 as well as natively (Constitution V).
//!
//! `tests/predicates.rs` runs natively only (its proptest dependency does not build
//! for wasm32). This runs `robust` and the power-of-two normalization on both
//! targets, on inputs whose exact sign is known analytically, including the ones
//! that overflow or underflow without normalization.

use openplush_geom::predicates::{incircle, orient2d, orient3d};
use openplush_geom::{DVec2, DVec3, GeomError, Sign};
use std::hint::black_box;

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn predicates_give_the_exact_sign_on_every_target() {
    let v2 = |x: f64, y: f64| DVec2::new(black_box(x), black_box(y));
    let v3 = |x: f64, y: f64, z: f64| DVec3::new(black_box(x), black_box(y), black_box(z));
    let x = 0.1_f64;

    // On the line y = x exactly, then one ULP above it: Zero, then Positive
    // (cross product = y - x).
    let (a, b) = (v2(0.0, 0.0), v2(1.0, 1.0));
    assert_eq!(orient2d(a, b, v2(x, x)), Ok(Sign::Zero));
    assert_eq!(orient2d(a, b, v2(x, x.next_up())), Ok(Sign::Positive));
    assert_eq!(orient2d(a, b, v2(x, x.next_down())), Ok(Sign::Negative));

    // Overflow and underflow without normalization; all exactly Positive.
    let big = 1e200;
    assert_eq!(
        incircle(v2(big, 0.0), v2(0.0, big), v2(-big, 0.0), v2(0.0, 0.0)),
        Ok(Sign::Positive)
    );
    let big3 = 1e110;
    assert_eq!(
        orient3d(
            v3(0.0, 0.0, 0.0),
            v3(big3, 0.0, 0.0),
            v3(0.0, big3, 0.0),
            v3(0.0, 0.0, -big3)
        ),
        Ok(Sign::Positive)
    );
    let tiny = 1e-200;
    assert_eq!(
        orient2d(v2(0.0, 0.0), v2(tiny, 0.0), v2(0.0, tiny)),
        Ok(Sign::Positive)
    );

    // Refusals are the same on every target too.
    assert_eq!(
        orient2d(v2(1.0, 0.0), v2(0.0, 1.0), v2(1e-60, 0.0)),
        Err(GeomError::UnsupportedMagnitude)
    );
    assert_eq!(
        orient2d(v2(f64::NAN, 0.0), v2(0.0, 1.0), v2(1.0, 0.0)),
        Err(GeomError::NonFinite)
    );
}
