//! The same solves on native and wasm32 must give bit-identical results (Constitution V).
//! The expected fingerprints are what `cargo run --release` printed natively.

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

const EXPECTED: [(usize, u64); 3] = [
    (10, 0xe433_e5d0_6e73_a7b4),
    (50, 0x55a5_307d_75b4_6427),
    (100, 0x20ae_d9c3_f662_44a3),
];

#[test]
fn every_solve_matches_the_native_fingerprint() {
    for (n, want) in EXPECTED {
        let x = faer_gates::solve(n);
        assert_eq!(faer_gates::fingerprint(&x), want, "n = {n}");
    }
}

#[test]
fn every_solve_is_accurate_against_a_stencil_recomputed_without_faer() {
    for (n, _) in EXPECTED {
        let x = faer_gates::solve(n);
        assert!(faer_gates::residual(n, &x) < 1e-10, "n = {n}");
    }
}
