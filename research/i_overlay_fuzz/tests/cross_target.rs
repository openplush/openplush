//! Same assertions natively (`cargo test --release`) and on wasm32 (`cargo test --release
//! --target wasm32-unknown-unknown`, through wasm-bindgen-test-runner under Node).
//! `CORPUS_200` was printed by the native release binary, so a passing wasm run produced
//! byte-identical i_overlay output for every method on the first 200 cases.

use i_overlay_fuzz::allowance::{
    fold_guards, merge_short_edges, pieces, strips, uniform_outline,
};
use i_overlay_fuzz::generate::case;
use i_overlay_fuzz::geom::{self, P, is_simple_ccw, orient, signed_area};
use i_overlay_fuzz::oracle::{Fault, check_strips};
use i_overlay_fuzz::{Method, SEED, corpus_hash, run_case};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

const CORPUS_200: u64 = 17_622_220_365_183_590_366;

const SQUARE: [P; 4] = [[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]];

fn area(shapes: &[Vec<Vec<P>>]) -> f64 {
    shapes.iter().flatten().map(|c| signed_area(c)).sum()
}

/// Catches a native/wasm divergence anywhere in i_overlay or the construction.
#[test]
fn first_200_cases_match_native() {
    assert_eq!(corpus_hash(SEED, 200), CORPUS_200);
}

/// Catches hidden state (statics, hash seeds) leaking into the output.
#[test]
fn same_input_twice_gives_identical_output() {
    assert_eq!(corpus_hash(SEED, 50), corpus_hash(SEED, 50));
}

/// Catches a wrong strip or miter: 100² + 4 × 100 × 6 + 4 square 6 × 6 miter corners.
#[test]
fn uniform_square_has_the_mitered_area() {
    let a = [6.0; 4];
    for shapes in [
        strips::<i32>(&SQUARE, &a),
        strips::<i64>(&SQUARE, &a),
        uniform_outline::<i32>(&SQUARE, 6.0),
    ] {
        assert_eq!(shapes.len(), 1);
        assert!(
            (area(&shapes) - 12_544.0).abs() < 1e-3,
            "area {}",
            area(&shapes)
        );
    }
}

/// Catches allowance leaking past a fold: the two corners beside the zero edge add nothing,
/// so the cut line runs straight along the fold line.
#[test]
fn fold_edge_stays_on_the_cut_line() {
    let a = [0.0, 6.0, 6.0, 6.0];
    let shapes = strips::<i64>(&SQUARE, &a);
    assert!((area(&shapes) - (10_000.0 + 3.0 * 600.0 + 2.0 * 36.0)).abs() < 1e-6);
    let v = check_strips(&SQUARE, &a, &shapes);
    assert!(v.faults.is_empty(), "{:?}", v.faults);
}

/// Catches an oracle without teeth: the bare outline misses every allowance, and a stray
/// square far away is extra and disconnected.
#[test]
fn oracle_flags_missing_and_extra_area() {
    let a = [6.0; 4];
    let bare = vec![vec![SQUARE.to_vec()]];
    assert!(
        check_strips(&SQUARE, &a, &bare)
            .faults
            .contains(&Fault::Missing)
    );
    let mut stray = strips::<i64>(&SQUARE, &a);
    stray.push(vec![vec![
        [300.0, 300.0],
        [310.0, 300.0],
        [310.0, 310.0],
        [300.0, 310.0],
    ]]);
    let faults = check_strips(&SQUARE, &a, &stray).faults;
    assert!(
        faults.contains(&Fault::Disconnected) && faults.contains(&Fault::Extra),
        "{faults:?}"
    );
}

/// Catches generator drift: every case is a valid input, and every piece and guard the
/// membership oracle relies on is convex and counter-clockwise.
#[test]
fn corpus_inputs_are_valid_and_pieces_convex() {
    for i in 0..300 {
        let c = case(SEED, i);
        // A near-straight miter wedge can be non-convex by float rounding (~1e-22 mm² seen),
        // far below the oracle's ambiguity band.
        let tol = 1e-12 * geom::bbox_diag(c.outline.iter().copied()).powi(2);
        let convex = |q: &Vec<P>| {
            (0..q.len()).all(|k| orient(q[k], q[(k + 1) % q.len()], q[(k + 2) % q.len()]) >= -tol)
        };
        assert!(is_simple_ccw(&c.outline), "case {i}");
        assert_eq!(c.outline.len(), c.allowance.len());
        let all = pieces(&c.outline, &c.allowance)
            .into_iter()
            .chain(fold_guards(&c.outline, &c.allowance));
        assert!(
            all.into_iter().all(|q| convex(&q) && signed_area(&q) > 0.0),
            "case {i}"
        );
    }
}

/// Catches an allowance shifted onto the wrong edge when a short edge is merged away,
/// including across the wrap-around.
#[test]
fn merging_short_edges_keeps_each_edge_its_allowance() {
    let outline = [
        [0.0, 0.0],
        [100.0, 0.0],
        [100.0, 1e-9],
        [100.0, 100.0],
        [0.0, 100.0],
    ];
    let (o, a) = merge_short_edges(&outline, &[1.0, 2.0, 3.0, 4.0, 5.0], 1e-6);
    assert_eq!(
        o,
        vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]]
    );
    assert_eq!(a, vec![1.0, 3.0, 4.0, 5.0]);
    let wrap = [
        [0.0, 0.0],
        [100.0, 0.0],
        [100.0, 100.0],
        [0.0, 100.0],
        [0.0, 1e-9],
    ];
    let (o, a) = merge_short_edges(&wrap, &[1.0, 2.0, 3.0, 4.0, 5.0], 1e-6);
    assert_eq!(
        o,
        vec![[100.0, 0.0], [100.0, 100.0], [0.0, 100.0], [0.0, 1e-9]]
    );
    assert_eq!(a, vec![2.0, 3.0, 4.0, 1.0]);
}

/// Catches i_overlay disagreeing with the independent reference, or panicking, on the strips.
#[test]
fn no_strip_result_in_the_first_200_loses_or_gains_area() {
    for i in 0..200 {
        let r = run_case(SEED, i);
        for (_, faults) in r
            .faults
            .iter()
            .filter(|(m, _)| **m != Method::UniformOutlineI32)
        {
            assert!(
                !faults.contains(&Fault::Missing) && !faults.contains(&Fault::Extra),
                "case {i}: {faults:?}"
            );
            assert!(
                !faults.iter().any(|f| matches!(f, Fault::Panic(_))),
                "case {i}: {faults:?}"
            );
        }
    }
}
