//! The near-mirror warning's target and wording, for seams that mirror and seams that do not.
#![allow(clippy::expect_used, clippy::panic, reason = "test setup")]

use openplush_geom::DVec2;
use openplush_pattern::{CutCount, Outline, PatternPiece, PieceId};

use super::{counted, near_warning};
use crate::pattern::group::Near;
use crate::pattern::group::{EdgeSeam, group};
use crate::{Remedy, WarningCode, WarningTarget};

fn piece(id: u32) -> PatternPiece {
    let square = Outline::new(vec![
        DVec2::new(0.0, 0.0),
        DVec2::new(1.0, 0.0),
        DVec2::new(1.0, 1.0),
        DVec2::new(0.0, 1.0),
    ])
    .expect("valid");
    PatternPiece::new(
        PieceId::new(id),
        square.clone(),
        square,
        CutCount::new(1, 0).expect("valid"),
        vec![0.0],
    )
    .expect("valid")
}

fn advice(remedy: &Remedy) -> &str {
    match remedy {
        Remedy::Advisory { what_to_change } => what_to_change.as_str(),
        Remedy::Actionable { .. } => panic!("advisory only"),
    }
}

#[test]
fn a_near_mirror_names_both_pieces_and_the_gap() {
    let pieces = [piece(2), piece(7)];
    let near = Near {
        a: 0,
        b: 1,
        gap_mm: 2.44,
        seams_mirror: false,
    };
    let w = near_warning(&pieces, &near);
    assert_eq!(w.code(), WarningCode::NearMirrorPieces);
    assert_eq!(w.code().as_str(), "near-mirror-pieces");
    assert_eq!(
        w.target(),
        WarningTarget::Pieces(PieceId::new(2), PieceId::new(7))
    );
    assert_eq!(
        w.explanation(),
        "Pieces 2 and 7 are mirror images to within 2.4 mm, so they print as two pieces instead \
         of one piece cut twice."
    );
    assert!(advice(w.remedy()).starts_with("Draw the seams on these two pieces"));
}

#[test]
fn when_the_seams_already_mirror_the_shape_is_what_differs() {
    let pieces = [piece(2), piece(7)];
    let near = Near {
        a: 0,
        b: 1,
        gap_mm: 4.96,
        seams_mirror: true,
    };
    let w = near_warning(&pieces, &near);
    assert!(advice(w.remedy()).starts_with("The model's shape differs by up to 5.0 mm"));
}

fn shaped(id: u32, points: &[(f64, f64)]) -> PatternPiece {
    let line =
        Outline::new(points.iter().map(|&(x, y)| DVec2::new(x, y)).collect()).expect("valid");
    PatternPiece::new(
        PieceId::new(id),
        line.clone(),
        line,
        CutCount::new(1, 0).expect("valid"),
        vec![0.0],
    )
    .expect("valid")
}

#[test]
fn a_template_counts_its_copies_as_printed_and_flipped() {
    let ell = [
        (0.0, 0.0),
        (90.0, 0.0),
        (90.0, 30.0),
        (30.0, 30.0),
        (30.0, 60.0),
        (0.0, 60.0),
    ];
    let mirrored: Vec<(f64, f64)> = ell.iter().rev().map(|&(x, y)| (-x, y)).collect();
    let pieces = [shaped(0, &ell), shaped(1, &ell), shaped(2, &mirrored)];
    let outlines: Vec<_> = pieces.iter().map(PatternPiece::sewing_line).collect();
    let bare = || vec![EdgeSeam::default(); 6];
    let grouping = group(&outlines, &[bare(), bare(), bare()]);
    let counted = counted(&pieces, &grouping).expect("counts");
    let cuts = counted[0].cuts();
    assert_eq!((cuts.as_printed(), cuts.flipped()), (2, 1));
}
