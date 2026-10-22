//! A layout the session reuses from its memo is byte-identical to one computed afresh, over
//! random heights and random edits between two seam sets.
//!
//! Native only: proptest does not build for wasm32 (see Cargo.toml).
#![cfg(not(target_arch = "wasm32"))]

#[allow(
    dead_code,
    reason = "each test file uses only some of the shared helpers"
)]
#[path = "common/grid.rs"]
mod grid;

use grid::{Setup, column_cut, grid, mesh_of, seam};
use openplush_pipeline::{
    ChartId, ManifoldMesh, Parameterization, Pipeline, SeamKind, SeamSet, chart, flatten,
};
use proptest::prelude::*;

const COLS: u32 = 4;
const ROWS: u32 = 2;

/// Full cuts up the columns set in the low three bits of `choice`, and a dart up into the
/// last interior point when the fourth bit is set and that point is not on a cut.
fn seams_of(mesh: &ManifoldMesh, choice: u8) -> Result<SeamSet, Setup> {
    let mut seams = SeamSet::new();
    for c in 1..COLS {
        if choice & (1 << (c - 1)) != 0 {
            column_cut(mesh, &mut seams, c, ROWS)?;
        }
    }
    if choice & 0b1000 != 0 && choice & 0b0100 == 0 {
        seam(
            mesh,
            &mut seams,
            SeamKind::Dart,
            &[(COLS - 1, 0), (COLS - 1, 1)],
        )?;
    }
    Ok(seams)
}

fn bits(layouts: &[Parameterization]) -> Vec<(ChartId, Vec<[u64; 2]>)> {
    layouts
        .iter()
        .map(|l| {
            let uv = l.uv().iter().map(|p| [p.x.to_bits(), p.y.to_bits()]);
            (l.chart(), uv.collect())
        })
        .collect()
}

fn check(heights: &[f64], before: u8, after: u8) -> Result<(), Setup> {
    let width = COLS + 1;
    let mesh = mesh_of(&grid(COLS, ROWS, |c, r| heights[(r * width + c) as usize]))?;
    let (first, second) = (seams_of(&mesh, before)?, seams_of(&mesh, after)?);

    let mut edited = Pipeline::new(mesh.clone());
    edited.set_seams(&first);
    edited.parameterizations()?;
    let renewed = edited.set_seams(&second);
    let reused = edited.parameterizations()?.value;

    let mut fresh = Pipeline::new(mesh.clone());
    fresh.set_seams(&second);
    let computed = fresh.parameterizations()?.value;

    let (charts, _) = chart(&mesh, &second)?.value;
    let staged = flatten(&mesh, &charts)?.value;
    assert_eq!(bits(&reused), bits(&computed), "renewed {renewed:?}");
    assert_eq!(bits(&reused), bits(&staged));
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 24, rng_seed: proptest::test_runner::RngSeed::Fixed(20261021), ..ProptestConfig::default() })]

    #[test]
    fn a_layout_reused_from_the_memo_is_byte_identical_to_one_recomputed(
        heights in prop::collection::vec(0.0_f64..3.0, ((COLS + 1) * (ROWS + 1)) as usize),
        before in 0_u8..16,
        after in 0_u8..16,
    ) {
        check(&heights, before, after).map_err(|e| TestCaseError::fail(e.to_string()))?;
    }
}
