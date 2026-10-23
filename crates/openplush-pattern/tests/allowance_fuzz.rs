//! `allowance::cut_line` against the seeded corpus from `research/i_overlay_fuzz/`. Every cut line it accepts must be the
//! reference region, (outline ∪ strips ∪ joins) − guards, judged point by point by an oracle that
//! uses no boolean, and reach every edge's full allowance.

mod fuzz;

use std::collections::BTreeMap;

use fuzz::generate::case;
use fuzz::geom::bbox_diag;
use fuzz::oracle::{Fault, check_strips};
use fuzz::reference::merge_short_edges;
use openplush_geom::DVec2;
use openplush_geom::epsilon::WELD_RELATIVE;
use openplush_geom::units::Mm;
use openplush_pattern::allowance::cut_line;
use openplush_pattern::{Outline, PatternError, PieceId};

type Setup = Box<dyn std::error::Error>;

/// That corpus's seed, so these are the first cases of the spike's own corpus.
const SEED: u64 = 0x0057_3F00_2026_1015;
const CASES: u64 = 1000;
/// The worst allowance shortfall on an edge longer than the grid, in mm; the spike measured 1.3e-10.
const SHORTFALL_MM: f64 = 1e-6;

/// What happened to one case.
fn outcome(index: u64) -> Result<String, Setup> {
    let c = case(SEED, index);
    let sewing = Outline::new(c.outline.iter().map(|&[x, y]| DVec2::new(x, y)).collect())?;
    let allowances = c
        .allowance
        .iter()
        .map(|&a| Mm::try_new(a))
        .collect::<Result<Vec<_>, _>>()?;
    let cut = match cut_line(PieceId::new(0), &sewing, &allowances) {
        Ok(cut) => cut,
        Err(e) => return Ok(refusal(&e).to_owned()),
    };
    // The oracle judges the outline the engine grew: merged as the engine merges it.
    let shortest = WELD_RELATIVE * bbox_diag(c.outline.iter().copied());
    let (outline, allowance) = merge_short_edges(&c.outline, &c.allowance, shortest);
    let drawn = vec![vec![cut.points().iter().map(|p| [p.x, p.y]).collect()]];
    let verdict = check_strips(&outline, &allowance, &drawn);
    // A guard the oracle sees touching the outline by rounding is what the engine allows for.
    let faults: Vec<Fault> = verdict
        .faults
        .into_iter()
        .filter(|f| *f != Fault::GuardHitsOutline)
        .collect();
    if !faults.is_empty() || verdict.shortfall_long > SHORTFALL_MM {
        return Err(format!("case {index}: {faults:?}, short {}", verdict.shortfall_long).into());
    }
    Ok("accepted".to_owned())
}

fn refusal(e: &PatternError) -> &'static str {
    match e {
        PatternError::FoldConflict { .. } => "FoldConflict",
        PatternError::AllowanceSplitsPiece { .. } => "AllowanceSplitsPiece",
        PatternError::AllowanceEnclosesGap { .. } => "AllowanceEnclosesGap",
        PatternError::AllowanceNotSimple { .. } => "AllowanceNotSimple",
        _ => "other",
    }
}

#[test]
fn every_accepted_cut_line_is_the_reference_region() -> Result<(), Setup> {
    let mut tally: BTreeMap<String, u32> = BTreeMap::new();
    for index in 0..CASES {
        *tally.entry(outcome(index)?).or_default() += 1;
    }
    // The corpus is adversarial by design (a zero allowance on one edge in four, needles, 0.01 mm
    // pieces with 10 mm allowances), so most refusals are real conflicts in the request. The tally
    // is pinned: a change means the construction changed, and a person decides whether that is
    // right.
    let expected = [
        ("AllowanceEnclosesGap", 12),
        ("AllowanceSplitsPiece", 44),
        ("FoldConflict", 464),
        ("accepted", 480),
    ];
    let expected: BTreeMap<String, u32> =
        expected.iter().map(|&(k, v)| (k.to_owned(), v)).collect();
    assert_eq!(tally, expected);
    Ok(())
}
