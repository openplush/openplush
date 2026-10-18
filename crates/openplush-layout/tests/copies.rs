//! Fabric and print layouts from one piece list: the fabric gets every copy, each flipped
//! copy mirrored; the print gets one template per piece.

use openplush_geom::DVec2;
use openplush_geom::units::Mm;
use openplush_layout::shelf::{pack, pack_templates};
use openplush_layout::{LayoutError, LayoutOptions, Placement, Sheet};
use openplush_pattern::{CutCount, Outline, PatternPiece, PieceId};

type Setup = Box<dyn std::error::Error>;

/// An L shape, so a flipped copy cannot pass for an unflipped one.
fn ell() -> Result<Outline, Setup> {
    let pts = [
        (0.0, 0.0),
        (30.0, 0.0),
        (30.0, 10.0),
        (10.0, 10.0),
        (10.0, 20.0),
        (0.0, 20.0),
    ];
    Ok(Outline::new(
        pts.iter().map(|&(x, y)| DVec2::new(x, y)).collect(),
    )?)
}

fn piece(id: u32, as_printed: u32, flipped: u32) -> Result<PatternPiece, Setup> {
    Ok(PatternPiece::new(
        PieceId::new(id),
        ell()?,
        ell()?,
        CutCount::new(as_printed, flipped)?,
        vec![0.0],
    )?)
}

fn opts(pile_mirrored: bool) -> Result<LayoutOptions, Setup> {
    Ok(LayoutOptions::new(
        Mm::try_new(500.0)?,
        Mm::try_new(5.0)?,
        pile_mirrored,
    )?)
}

fn only(sheets: Vec<Sheet>) -> Result<Sheet, Setup> {
    sheets
        .into_iter()
        .next()
        .ok_or_else(|| LayoutError::NoPieces.into())
}

fn flags(sheet: &Sheet) -> Vec<(u32, u32, bool)> {
    sheet
        .placements()
        .iter()
        .map(|p| (p.piece().index(), p.copy(), p.mirrored()))
        .collect()
}

#[test]
fn the_fabric_gets_every_copy_and_mirrors_only_the_flipped_ones() -> Result<(), Setup> {
    let s = only(pack(&[piece(0, 2, 1)?], &opts(false)?)?)?;
    assert_eq!(flags(&s), [(0, 0, false), (0, 1, false), (0, 2, true)]);
    Ok(())
}

#[test]
fn printing_pile_mirrored_mirrors_the_fabric_plan_too_so_flipped_copies_come_back()
-> Result<(), Setup> {
    // The plan is drawn as the maker traces it: on the wrong side when the print is mirrored.
    let s = only(pack(&[piece(0, 1, 1)?], &opts(true)?)?)?;
    assert_eq!(flags(&s), [(0, 0, true), (0, 1, false)]);
    Ok(())
}

#[test]
fn a_flipped_copy_is_the_mirror_image_of_an_unflipped_one() -> Result<(), Setup> {
    let s = only(pack(&[piece(0, 1, 1)?], &opts(false)?)?)?;
    let shape = |p: &Placement| -> Result<Vec<DVec2>, Setup> {
        let pts: Vec<DVec2> = ell()?.points().iter().map(|&q| p.apply(q)).collect();
        let min_x = pts.iter().map(|q| q.x).fold(f64::INFINITY, f64::min);
        let min_y = pts.iter().map(|q| q.y).fold(f64::INFINITY, f64::min);
        Ok(pts
            .iter()
            .map(|q| DVec2::new(q.x - min_x, q.y - min_y))
            .collect())
    };
    let (a, b) = (shape(&s.placements()[0])?, shape(&s.placements()[1])?);
    // Mirroring the unflipped copy about its own width gives the flipped one, point for point.
    let width = a.iter().map(|q| q.x).fold(0.0, f64::max);
    let mirrored: Vec<DVec2> = a.iter().map(|q| DVec2::new(width - q.x, q.y)).collect();
    assert_eq!(mirrored, b);
    Ok(())
}

#[test]
fn copies_never_overlap_whether_flipped_or_not() -> Result<(), Setup> {
    let s = only(pack(&[piece(0, 2, 2)?, piece(1, 0, 3)?], &opts(false)?)?)?;
    let boxes: Vec<(DVec2, DVec2)> = s
        .placements()
        .iter()
        .map(|p| -> Result<(DVec2, DVec2), Setup> {
            let pts: Vec<DVec2> = ell()?.points().iter().map(|&q| p.apply(q)).collect();
            let xs = pts.iter().map(|q| q.x);
            let ys = pts.iter().map(|q| q.y);
            let lo = DVec2::new(
                xs.clone().fold(f64::INFINITY, f64::min),
                ys.clone().fold(f64::INFINITY, f64::min),
            );
            let hi = DVec2::new(
                xs.fold(f64::NEG_INFINITY, f64::max),
                ys.fold(f64::NEG_INFINITY, f64::max),
            );
            Ok((lo, hi))
        })
        .collect::<Result<_, _>>()?;
    assert_eq!(boxes.len(), 7);
    for (i, a) in boxes.iter().enumerate() {
        assert!(
            a.0.x >= 0.0 && a.0.y >= 0.0 && a.1.x <= 500.0,
            "copy {i} leaves the roll"
        );
        for b in &boxes[i + 1..] {
            let apart = a.1.x <= b.0.x || b.1.x <= a.0.x || a.1.y <= b.0.y || b.1.y <= a.0.y;
            assert!(apart, "{a:?} overlaps {b:?}");
        }
    }
    Ok(())
}

#[test]
fn the_print_gets_one_template_per_piece_whatever_its_counts() -> Result<(), Setup> {
    let pieces = [piece(0, 3, 1)?, piece(1, 0, 2)?, piece(2, 1, 0)?];
    let s = only(pack_templates(&pieces, &opts(false)?)?)?;
    let mut got = flags(&s);
    got.sort();
    assert_eq!(got, [(0, 0, false), (1, 0, false), (2, 0, false)]);
    Ok(())
}

#[test]
fn the_print_follows_the_pile_mirroring_choice() -> Result<(), Setup> {
    let s = only(pack_templates(&[piece(0, 1, 1)?], &opts(true)?)?)?;
    assert_eq!(flags(&s), [(0, 0, true)]);
    Ok(())
}

#[test]
fn no_pieces_is_refused_for_the_print_too() -> Result<(), Setup> {
    assert_eq!(
        pack_templates(&[], &opts(false)?),
        Err(LayoutError::NoPieces)
    );
    Ok(())
}
