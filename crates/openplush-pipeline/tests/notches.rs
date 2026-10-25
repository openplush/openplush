//! Match notches: on every side of every seam, at both ends and at the same fractions of its length,
//! at most 50 mm apart, with a mark count that differs from every seam it meets on a piece. The
//! sides are walked from the chart faces here, not by the stage.

use std::collections::{BTreeMap, BTreeSet};

use openplush_pipeline::{PatternPiece, SeamChainId};

#[allow(dead_code, reason = "each test file uses part of the shared fixtures")]
mod common;

use common::sides::{Path, Run, SAME_MM, dart, inside, path, reference};
use common::{CAPSULE_OBJ, CAPSULE_SEAMS, CUBE_OBJ, CUBE_SEAMS, Setup};

/// The longest gap allowed between neighbouring notches on one side.
const SPACING_MM: f64 = 50.0;

/// Per piece and point on its sewing line, the chain and mark count of each side ending there.
type Ends = BTreeMap<(usize, [u64; 2]), Vec<(u32, u32)>>;

/// The fractions along `path` of `chain`'s notches on `piece` between the path's ends, sorted,
/// and the marks they all carry.
fn inner_notches(path: &Path, piece: &PatternPiece, chain: SeamChainId) -> (Vec<f64>, u32) {
    let (start, end) = (path.points[0], path.points[path.points.len() - 1]);
    let mut marks = None;
    let mut inner = Vec::new();
    for n in piece.notches().iter().filter(|n| n.chain() == chain) {
        let at_end = (n.at() - start).length() <= SAME_MM || (n.at() - end).length() <= SAME_MM;
        let Some(t) = path.fraction_of(n.at()).filter(|_| !at_end) else {
            continue;
        };
        assert_eq!(
            *marks.get_or_insert(n.marks()),
            n.marks(),
            "chain {chain:?}"
        );
        inner.push(t);
    }
    inner.sort_by(f64::total_cmp);
    assert!(
        !inner.is_empty(),
        "chain {chain:?} has no notch between its ends"
    );
    (inner, marks.unwrap_or(0))
}

fn assert_spaced(path: &Path, inner: &[f64], chain: SeamChainId) {
    let mut stops = vec![0.0];
    stops.extend(inner);
    stops.push(1.0);
    for w in stops.windows(2) {
        let gap = (w[1] - w[0]) * path.length();
        assert!(
            gap <= SPACING_MM + SAME_MM,
            "chain {chain:?}: notches {gap} mm apart"
        );
    }
}

fn assert_same_fractions(want: &[f64], got: &[f64], chain: SeamChainId) {
    assert_eq!(want.len(), got.len(), "chain {chain:?}");
    for (a, b) in want.iter().zip(got) {
        assert!((a - b).abs() <= 1e-9, "chain {chain:?}: {a} vs {b}");
    }
}

/// Every notch claim this file makes, for one run.
fn assert_notched(run: &Run) -> Result<(), Setup> {
    assert!(!run.sewing.bundles().is_empty());
    let mut ends = Ends::new();
    for bundle in run.sewing.bundles() {
        let chain = bundle.chain();
        let mut first: Option<Vec<f64>> = None;
        for side in bundle.sides() {
            let path = path(run, side)?;
            let piece = &run.pieces[path.chart];
            let (inner, marks) = inner_notches(&path, piece, chain);
            assert_spaced(&path, &inner, chain);
            match &first {
                None => first = Some(inner),
                Some(want) => assert_same_fractions(want, &inner, chain),
            }
            for p in [path.points[0], path.points[path.points.len() - 1]] {
                let at_end: Vec<_> = piece
                    .notches()
                    .iter()
                    .filter(|n| (n.at() - p).length() <= SAME_MM)
                    .collect();
                assert!(
                    !at_end.is_empty(),
                    "chain {chain:?}: no notch at its end {p:?}"
                );
                assert!(at_end.iter().all(|n| n.marks() == 1), "{at_end:?}");
                ends.entry((path.chart, [p.x.to_bits(), p.y.to_bits()]))
                    .or_default()
                    .push((chain.index(), marks));
            }
        }
    }
    assert_neighbours_differ(&ends);
    assert_pieces(run);
    Ok(())
}

/// Seams that meet on a piece differ, and each has exactly the least count, from 1, that no
/// lower seam it meets has: no more than it needs.
fn assert_neighbours_differ(ends: &Ends) {
    let mut meets: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    let mut marks = BTreeMap::new();
    for ((chart, _), chains) in ends {
        for (i, &(a, ma)) in chains.iter().enumerate() {
            marks.insert(a, ma);
            meets.entry(a).or_default();
            for &(b, mb) in &chains[i + 1..] {
                if a != b {
                    assert_ne!(ma, mb, "chains {a} and {b} meet on piece {chart}");
                    meets.entry(a).or_default().insert(b);
                    meets.entry(b).or_default().insert(a);
                }
            }
        }
    }
    let mut want: BTreeMap<u32, u32> = BTreeMap::new();
    for (&chain, others) in &meets {
        let taken: BTreeSet<u32> = others.iter().filter_map(|o| want.get(o).copied()).collect();
        let least = (1..)
            .find(|c| !taken.contains(c))
            .ok_or("a count")
            .unwrap_or(0);
        want.insert(chain, least);
    }
    assert_eq!(marks, want);
}

/// Each notch has marks, points out of its piece, and sits apart from the piece's others.
fn assert_pieces(run: &Run) {
    for piece in &run.pieces {
        let line = piece.sewing_line().points();
        for (i, n) in piece.notches().iter().enumerate() {
            assert!(n.marks() >= 1);
            assert!((n.outward().length() - 1.0).abs() <= 1e-9);
            assert!(
                !inside(line, n.at() + n.outward() * 0.5),
                "{n:?} points inward"
            );
            for m in &piece.notches()[i + 1..] {
                assert!(
                    (n.at() - m.at()).length() > SAME_MM,
                    "{n:?} and {m:?} coincide"
                );
            }
        }
    }
}

#[test]
fn every_capsule_seam_is_notched_alike_on_both_sides() -> Result<(), Setup> {
    let run = reference(CAPSULE_OBJ, CAPSULE_SEAMS, 5.0)?;
    assert_notched(&run)?;
    let most = run
        .pieces
        .iter()
        .flat_map(PatternPiece::notches)
        .filter(|n| n.marks() > 1)
        .count();
    assert!(most > 0, "no seam needs more than one mark");
    Ok(())
}

#[test]
fn every_cube_seam_is_notched_alike_on_both_sides() -> Result<(), Setup> {
    assert_notched(&reference(CUBE_OBJ, CUBE_SEAMS, 5.0)?)
}

#[test]
fn both_sides_of_a_dart_are_notched_alike() -> Result<(), Setup> {
    assert_notched(&dart()?)
}

#[test]
fn a_short_seam_still_gets_a_notch_between_its_ends() -> Result<(), Setup> {
    let run = reference(CAPSULE_OBJ, CAPSULE_SEAMS, 0.5)?;
    let short = run.sewing.bundles().iter().any(|b| {
        b.sides()
            .iter()
            .all(|s| path(&run, s).is_ok_and(|p| p.length() <= SPACING_MM))
    });
    assert!(short, "no seam of 50 mm or less");
    assert_notched(&run)
}

#[test]
fn the_same_input_notches_to_the_same_bits() -> Result<(), Setup> {
    let a = reference(CAPSULE_OBJ, CAPSULE_SEAMS, 5.0)?;
    let b = reference(CAPSULE_OBJ, CAPSULE_SEAMS, 5.0)?;
    let bits = |r: &Run| -> Vec<(u32, u64, u64, u32)> {
        r.pieces
            .iter()
            .flat_map(PatternPiece::notches)
            .map(|n| {
                (
                    n.chain().index(),
                    n.at().x.to_bits(),
                    n.at().y.to_bits(),
                    n.marks(),
                )
            })
            .collect()
    };
    assert_eq!(bits(&a), bits(&b));
    Ok(())
}
