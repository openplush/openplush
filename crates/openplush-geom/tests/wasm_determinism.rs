//! IEEE-754 determinism canary and wasm test-harness smoke test.
//!
//! A prerequisite for the native-vs-WASM export hash comparison, not that test
//! itself (Constitution V). That comparison only makes sense if basic IEEE-754
//! arithmetic gives bit-identical results on both targets. Add, multiply, divide,
//! and sqrt are correctly rounded by the standard, so they must match. The
//! expected bit patterns were computed independently (CPython `struct.pack`), not
//! by this crate.
//!
//! Transcendentals (`sin`, `exp`, `powf`, …) are deliberately absent: they call
//! the platform libm and are NOT guaranteed identical across targets. Engine code
//! on an output path must not depend on them without a portable implementation.
//!
//! This is also the smoke test that proves the `wasm-bindgen-test` harness in CI
//! runs. Natively it is an ordinary `#[test]`: `#[wasm_bindgen_test]` alone is a
//! silent no-op off wasm, which would make this pass without running.

use std::hint::black_box;

fn bits(x: f64) -> u64 {
    x.to_bits()
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn correctly_rounded_ops_are_bit_identical_across_targets() {
    // black_box keeps rustc from folding these at compile time, so the target's
    // own floating-point unit computes them.
    let (a, b, three, two) = (
        black_box(0.1_f64),
        black_box(0.2_f64),
        black_box(3.0_f64),
        black_box(2.0_f64),
    );

    assert_eq!(bits(a + b), 0x3fd3_3333_3333_3334, "0.1 + 0.2");
    assert_eq!(bits(a * three), 0x3fd3_3333_3333_3334, "0.1 * 3");
    assert_eq!(
        bits(black_box(1.0_f64) / three),
        0x3fd5_5555_5555_5555,
        "1 / 3"
    );
    assert_eq!(bits(two.sqrt()), 0x3ff6_a09e_667f_3bcd, "sqrt(2)");
    assert_eq!(
        bits(black_box(-0.0_f64) * black_box(1.0_f64)),
        0x8000_0000_0000_0000,
        "-0 * 1 keeps the sign"
    );
    assert_eq!(
        bits(a.mul_add(black_box(10.0), black_box(-1.0))),
        0x3c90_0000_0000_0000,
        "fma(0.1, 10, -1) is fused"
    );
}
