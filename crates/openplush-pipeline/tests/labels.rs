//! Piece labels from the pattern stage: a block inside each piece naming it, its cuts, its fabric
//! and the pieces it sews to, and a seam label inside the piece at the middle of each seam side.
//! Partners and side midpoints are found here from the sewing map, not by the stage.

use std::collections::{BTreeMap, BTreeSet};

#[allow(dead_code, reason = "each test file uses part of the shared fixtures")]
mod common;

use common::sides::{Run, dart, inside, path, reference};
use common::{CAPSULE_OBJ, CAPSULE_SEAMS, CUBE_OBJ, CUBE_SEAMS, Setup};

/// How far inside its side a seam label sits, and how near the side's midpoint it must be.
const INSET_MM: f64 = 4.0;

/// The chart indices each chart shares a seam with, itself left out.
fn partners(run: &Run) -> Result<BTreeMap<usize, BTreeSet<usize>>, Setup> {
    let mut out: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    for bundle in run.sewing.bundles() {
        let charts: BTreeSet<usize> = bundle
            .sides()
            .iter()
            .map(|s| path(run, s).map(|p| p.chart))
            .collect::<Result<_, _>>()?;
        for &a in &charts {
            out.entry(a)
                .or_default()
                .extend(charts.iter().filter(|&&b| b != a));
        }
    }
    Ok(out)
}

fn sews_to(others: &BTreeSet<u32>) -> Option<String> {
    let list: Vec<String> = others.iter().map(ToString::to_string).collect();
    match list.len() {
        0 => None,
        1 => Some(format!("Sews to piece {}", list[0])),
        _ => Some(format!("Sews to pieces {}", list.join(", "))),
    }
}

fn assert_blocks(run: &Run) -> Result<(), Setup> {
    let partners = partners(run)?;
    let id = |c: &usize| run.pieces[*c].id().index();
    for (c, piece) in run.pieces.iter().enumerate() {
        let label = piece.label().ok_or("a piece with no label")?;
        let mut want = vec![
            format!("Piece {}", piece.id().index()),
            "Cut 1".to_owned(),
            "cotton".to_owned(),
        ];
        let ids: BTreeSet<u32> = partners.get(&c).into_iter().flatten().map(id).collect();
        want.extend(sews_to(&ids));
        assert_eq!(label.lines(), want);
        assert!(
            inside(piece.sewing_line().points(), label.at()),
            "{label:?}"
        );
    }
    Ok(())
}

/// Each seam side has its own label, `Seam N` for its lead chain, inside its piece and the
/// inset away from its side's midpoint.
fn assert_seam_labels(run: &Run) -> Result<(), Setup> {
    let mut want: BTreeMap<usize, usize> = BTreeMap::new();
    for bundle in run.sewing.bundles() {
        let text = format!("Seam {}", bundle.chain().index());
        for side in bundle.sides() {
            let p = path(run, side)?;
            *want.entry(p.chart).or_default() += 1;
            let mid = p.point_at(0.5);
            let label = run.pieces[p.chart].label().ok_or("no label")?;
            let near = label
                .seams()
                .iter()
                .filter(|s| s.text() == text)
                .any(|s| ((s.at() - mid).length() - INSET_MM).abs() <= 1e-6);
            assert!(
                near,
                "{text}: no label {INSET_MM} mm from {mid:?} in {:?}",
                label.seams()
            );
        }
    }
    for (c, piece) in run.pieces.iter().enumerate() {
        let label = piece.label().ok_or("no label")?;
        assert_eq!(label.seams().len(), want.get(&c).copied().unwrap_or(0));
        for s in label.seams() {
            assert!(inside(piece.sewing_line().points(), s.at()), "{s:?}");
        }
    }
    Ok(())
}

#[test]
fn every_capsule_piece_is_labelled_with_its_partners_and_seams() -> Result<(), Setup> {
    let run = reference(CAPSULE_OBJ, CAPSULE_SEAMS, 5.0)?;
    assert_blocks(&run)?;
    assert_seam_labels(&run)
}

#[test]
fn every_cube_piece_is_labelled_with_its_partners_and_seams() -> Result<(), Setup> {
    let run = reference(CUBE_OBJ, CUBE_SEAMS, 5.0)?;
    assert_blocks(&run)?;
    assert_seam_labels(&run)
}

#[test]
fn a_dart_labels_both_its_sides_but_is_no_partner_of_its_own_piece() -> Result<(), Setup> {
    let run = dart()?;
    assert_blocks(&run)?;
    assert_seam_labels(&run)
}
