//! `Placement::apply`: where a point of a piece's outline lands on the sheet.

use openplush_geom::DVec2;
use openplush_geom::units::Mm;
use openplush_layout::shelf::pack;
use openplush_layout::{LayoutError, LayoutOptions, Placement};
use openplush_pattern::{CutCount, Outline, PatternPiece, PieceId};

type Setup = Box<dyn std::error::Error>;

/// An L-shaped outline, so every turn and mirror gives a different shape.
fn ell() -> Result<Outline, Setup> {
    Ok(Outline::new(vec![
        DVec2::new(0.0, 0.0),
        DVec2::new(30.0, 0.0),
        DVec2::new(30.0, 10.0),
        DVec2::new(10.0, 10.0),
        DVec2::new(10.0, 20.0),
        DVec2::new(0.0, 20.0),
    ])?)
}

fn placed(rotation: f64, mirrored: bool) -> Result<Placement, Setup> {
    let piece = PatternPiece::new(
        PieceId::new(0),
        ell()?,
        ell()?,
        CutCount::new(1, 0)?,
        vec![rotation],
    )?;
    let opts = LayoutOptions::new(Mm::try_new(500.0)?, Mm::try_new(0.0)?, mirrored)?;
    let sheet = pack(&[piece], &opts)?
        .into_iter()
        .next()
        .ok_or(LayoutError::NoPieces)?;
    sheet
        .placements()
        .first()
        .cloned()
        .ok_or_else(|| LayoutError::NoPieces.into())
}

/// The documented transform, restated: mirror about the y-axis, turn counter-clockwise, then move.
fn expected(p: DVec2, placement: &Placement) -> DVec2 {
    let q = if placement.mirrored() {
        DVec2::new(-p.x, p.y)
    } else {
        p
    };
    let turn = placement.rotation_deg().rem_euclid(360.0);
    let q = if turn == 90.0 {
        DVec2::new(-q.y, q.x)
    } else if turn == 180.0 {
        DVec2::new(-q.x, -q.y)
    } else if turn == 270.0 {
        DVec2::new(q.y, -q.x)
    } else {
        q
    };
    q + placement.offset()
}

#[test]
fn apply_mirrors_then_turns_then_moves() -> Result<(), Setup> {
    for rotation in [0.0, 90.0, 180.0, 270.0, -90.0, 450.0] {
        for mirrored in [false, true] {
            let placement = placed(rotation, mirrored)?;
            for &p in ell()?.points() {
                assert_eq!(
                    placement.apply(p),
                    expected(p, &placement),
                    "{rotation} deg, mirrored {mirrored}, point {p:?}"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn a_placed_piece_starts_at_the_sheet_corner() -> Result<(), Setup> {
    for rotation in [0.0, 90.0, 180.0, 270.0] {
        let placement = placed(rotation, true)?;
        let points: Vec<DVec2> = ell()?
            .points()
            .iter()
            .map(|&p| placement.apply(p))
            .collect();
        let min_x = points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
        let min_y = points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
        assert_eq!((min_x, min_y), (0.0, 0.0), "{rotation} deg");
    }
    Ok(())
}
