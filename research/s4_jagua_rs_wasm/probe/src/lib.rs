//! One export with the same signature in both builds, so the size delta is
//! what the harness and jagua-rs add.

use wasm_bindgen::prelude::wasm_bindgen;

/// Used strip length in mm, or NaN on a harness error.
#[cfg(feature = "jagua")]
#[wasm_bindgen]
pub fn nest_length(half_turn: bool) -> f32 {
    use s4_jagua_rs_wasm::{RotationSet, nest};
    let set = if half_turn {
        RotationSet::HalfTurn
    } else {
        RotationSet::Fixed
    };
    nest(set).map_or(f32::NAN, |n| n.length)
}

/// Baseline: the same export with no work behind it.
#[cfg(not(feature = "jagua"))]
#[wasm_bindgen]
pub fn nest_length(half_turn: bool) -> f32 {
    f32::from(u8::from(half_turn))
}
