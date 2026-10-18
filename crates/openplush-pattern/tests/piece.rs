//! `PatternPiece::new`: a cut quantity of at least one and a usable rotation set.

use openplush_geom::DVec2;
use openplush_pattern::{Outline, PatternError, PatternPiece, PieceId};

fn square(side: f64) -> Result<Outline, PatternError> {
    Outline::new(vec![
        DVec2::new(0.0, 0.0),
        DVec2::new(side, 0.0),
        DVec2::new(side, side),
        DVec2::new(0.0, side),
    ])
}

fn piece(cuts: u32, rotations: Vec<f64>) -> Result<PatternPiece, PatternError> {
    PatternPiece::new(
        PieceId::new(4),
        square(8.0)?,
        square(10.0)?,
        cuts,
        rotations,
    )
}

#[test]
fn a_piece_reports_what_it_was_built_from() -> Result<(), PatternError> {
    let p = piece(2, vec![0.0, 180.0])?;
    assert_eq!(p.id(), PieceId::new(4));
    assert_eq!(p.sewing_line(), &square(8.0)?);
    assert_eq!(p.cut_line(), &square(10.0)?);
    assert_eq!(p.cut_quantity(), 2);
    assert_eq!(p.rotations_deg(), [0.0, 180.0]);
    Ok(())
}

#[test]
fn zero_cuts_is_refused() {
    assert_eq!(piece(0, vec![0.0]), Err(PatternError::ZeroCutQuantity));
}

#[test]
fn one_cut_and_one_rotation_is_the_smallest_valid_piece() {
    assert!(piece(1, vec![0.0]).is_ok());
}

#[test]
fn no_rotations_is_refused() {
    assert_eq!(piece(1, vec![]), Err(PatternError::NoRotations));
}

#[test]
fn a_non_finite_rotation_names_its_index() {
    for bad in [f64::NAN, f64::INFINITY] {
        assert_eq!(
            piece(1, vec![0.0, bad]),
            Err(PatternError::NonFiniteRotation { index: 1 })
        );
    }
}

#[test]
fn the_new_errors_say_what_is_wrong() {
    let cases = [
        (
            PatternError::OutlineTooShort { points: 2 },
            "an outline needs at least 3 points, got 2",
        ),
        (
            PatternError::OutlineNotFinite { point: 5 },
            "outline point 5 is not finite",
        ),
        (
            PatternError::OutlineNotSimple { edge: 1, other: 3 },
            "outline edges 1 and 3 cross, touch, overlap or fold back",
        ),
        (PatternError::OutlineClockwise, "outline is wound clockwise"),
        (
            PatternError::ZeroCutQuantity,
            "a piece must be cut at least once",
        ),
        (
            PatternError::NoRotations,
            "a piece needs at least one allowed rotation",
        ),
        (
            PatternError::NonFiniteRotation { index: 2 },
            "rotation 2 is not finite",
        ),
    ];
    for (err, text) in cases {
        assert_eq!(err.to_string(), text);
    }
}
