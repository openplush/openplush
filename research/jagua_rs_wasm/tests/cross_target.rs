//! Same assertions natively (`cargo test`) and on wasm32 (`cargo test --target
//! wasm32-unknown-unknown`, through wasm-bindgen-test-runner under Node).
//! The expected values were produced by the native release binary; a wasm run
//! that passes therefore produced bit-identical placements.

use std::f32::consts::PI;

use jagua_rs::geometry::DTransformation;
use jagua_rs::geometry::geo_enums::RotationRange;
use jagua_rs_wasm::{RotationSet, import_piece, is_half_turn, nest, pieces};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

const FIXED_LENGTH: f32 = 186.0;
const FIXED_HASH: u64 = 1852278804236050536;
const HALF_TURN_LENGTH: f32 = 156.0;
const HALF_TURN_HASH: u64 = 17964957110741447323;

fn first_outline() -> Vec<(f32, f32)> {
    pieces().swap_remove(1).1
}

/// {0°} must not silently widen to a rotation range.
#[test]
fn fixed_set_imports_as_no_rotation() {
    let item = import_piece(1, &first_outline(), RotationSet::Fixed, None).unwrap();
    let o = item.allowed_orientations();
    assert_eq!(o.rotations(), &RotationRange::None);
    assert!(o.rotations_after_reflection().is_none());
    assert!(o.allows(&DTransformation::new(0.0, (5.0, 5.0))));
    assert!(!o.allows(&DTransformation::new(PI, (5.0, 5.0))));
}

/// {0°, 180°} must stay a two-element discrete set, in radians.
#[test]
fn half_turn_set_imports_as_two_discrete_angles() {
    let item = import_piece(1, &first_outline(), RotationSet::HalfTurn, None).unwrap();
    let o = item.allowed_orientations();
    assert_eq!(o.rotations(), &RotationRange::Discrete(vec![0.0, PI]));
    assert!(o.allows(&DTransformation::new(PI, (0.0, 0.0))));
    assert!(!o.allows(&DTransformation::new(PI / 2.0, (0.0, 0.0))));
}

/// A napped mirror copy reflects across the axis parallel to the nap (local +Y,
/// 90°): x-axis reflection then a half turn, so the nap still points +Y.
#[test]
fn nap_parallel_mirror_is_reflection_plus_half_turn() {
    let item = import_piece(1, &first_outline(), RotationSet::Fixed, Some(90.0)).unwrap();
    let o = item.allowed_orientations();
    assert_eq!(
        o.rotations_after_reflection(),
        Some(&RotationRange::Discrete(vec![PI]))
    );
    let mirrored = DTransformation::new(PI, (0.0, 0.0)).with_reflection(true);
    assert!(o.allows(&mirrored));
    assert!(!o.allows(&DTransformation::new(0.0, (0.0, 0.0)).with_reflection(true)));
}

#[test]
fn fixed_nest_never_rotates_and_matches_native() {
    let n = nest(RotationSet::Fixed).unwrap();
    assert_eq!(n.placements.len(), pieces().len());
    assert!(n.placements.iter().all(|p| p.rotation == 0.0));
    assert_eq!((n.length, n.hash), (FIXED_LENGTH, FIXED_HASH));
    assert_eq!(
        nest(RotationSet::Fixed).unwrap(),
        n,
        "second run in the same process differs"
    );
}

#[test]
fn half_turn_nest_uses_the_half_turn_and_matches_native() {
    let n = nest(RotationSet::HalfTurn).unwrap();
    assert_eq!(n.placements.len(), pieces().len());
    assert!(
        n.placements
            .iter()
            .all(|p| p.rotation == 0.0 || is_half_turn(p.rotation))
    );
    assert!(n.placements.iter().any(|p| is_half_turn(p.rotation)));
    assert!(
        n.length < FIXED_LENGTH,
        "the extra rotation should save fabric here"
    );
    assert_eq!((n.length, n.hash), (HALF_TURN_LENGTH, HALF_TURN_HASH));
}
