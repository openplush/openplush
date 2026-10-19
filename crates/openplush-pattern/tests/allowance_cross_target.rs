//! The same requests give the same cut lines, bit for bit, natively and on wasm32 (Constitution V,
//! ADR 0006), over the first cases of S3's seeded corpus.
//!
//! The pinned hash was computed natively; the wasm32 run must reproduce it. It changes only when
//! the construction does, and then a person decides whether the new cut lines are right.

#[allow(dead_code, reason = "this file uses only the corpus generator")]
mod fuzz;

use fuzz::generate::case;
use openplush_geom::DVec2;
use openplush_geom::units::Mm;
use openplush_pattern::allowance::cut_line;
use openplush_pattern::{Outline, PatternError, PieceId};

/// S3's corpus seed.
const SEED: u64 = 0x0057_3F00_2026_1015;
const CASES: u64 = 200;

/// 64-bit FNV-1a, fed one word at a time.
fn fnv(hash: u64, word: u64) -> u64 {
    word.to_le_bytes().iter().fold(hash, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// Every cut line's coordinate bits, or which refusal, case by case.
fn corpus_hash() -> Result<u64, String> {
    let mut hash = 0xcbf2_9ce4_8422_2325;
    for index in 0..CASES {
        let c = case(SEED, index);
        let points = c.outline.iter().map(|&[x, y]| DVec2::new(x, y)).collect();
        let sewing = Outline::new(points).map_err(|e| e.to_string())?;
        let allowances = c
            .allowance
            .iter()
            .map(|&a| Mm::try_new(a).map_err(|e| e.to_string()))
            .collect::<Result<Vec<_>, _>>()?;
        hash = match cut_line(PieceId::new(0), &sewing, &allowances) {
            Ok(cut) => cut.points().iter().fold(fnv(hash, 0), |h, p| {
                fnv(fnv(h, p.x.to_bits()), p.y.to_bits())
            }),
            Err(e) => fnv(hash, 1 + refusal(&e)),
        };
    }
    Ok(hash)
}

fn refusal(e: &PatternError) -> u64 {
    match e {
        PatternError::FoldConflict { edge, .. } => 10 + *edge as u64,
        PatternError::AllowanceSplitsPiece { .. } => 1,
        PatternError::AllowanceEnclosesGap { .. } => 2,
        PatternError::AllowanceNotSimple { .. } => 3,
        _ => 4,
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn the_cut_lines_are_the_same_on_every_target() {
    assert_eq!(corpus_hash(), Ok(0x3e40_a53a_50ee_01ba));
}
