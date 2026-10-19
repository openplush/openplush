//! The scale-check square: 50.00 mm with a 1 in square inside, on every page, clear of the overlap bands.

use openplush_geom::DVec2;
use openplush_geom::units::Mm;
use openplush_layout::pages::{INCH_SQUARE_MM, PageTiling, Paper, SCALE_SQUARE_MM, tile};
use openplush_layout::shelf::pack;
use openplush_layout::{LayoutError, LayoutOptions};
use openplush_pattern::{CutCount, Outline, PatternPiece, PieceId};

type Setup = Box<dyn std::error::Error>;

fn tiling(roll: f64, length: f64, paper: Paper) -> Result<PageTiling, Setup> {
    let outline = Outline::new(vec![
        DVec2::new(0.0, 0.0),
        DVec2::new(10.0, 0.0),
        DVec2::new(10.0, length),
        DVec2::new(0.0, length),
    ])?;
    let piece = PatternPiece::new(
        PieceId::new(0),
        outline.clone(),
        outline,
        CutCount::new(1, 0)?,
        vec![0.0],
    )?;
    let opts = LayoutOptions::new(Mm::try_new(roll)?, Mm::try_new(0.0)?, false)?;
    let sheet = pack(&[piece], &opts)?
        .into_iter()
        .next()
        .ok_or(LayoutError::NoPieces)?;
    Ok(tile(&sheet, paper)?)
}

#[test]
fn the_squares_are_50_mm_and_one_inch() {
    assert_eq!(SCALE_SQUARE_MM, 50.0);
    assert_eq!(INCH_SQUARE_MM, 25.4);
}

#[test]
fn every_page_has_its_square_at_the_top_left_just_inside_the_bands_it_shares_with_earlier_pages()
-> Result<(), Setup> {
    let t = tiling(400.0, 600.0, Paper::A4)?;
    for p in t.pages() {
        assert_eq!(
            p.scale_square_min(),
            DVec2::new(p.window_min().x + 15.0, p.window_max().y - 15.0 - 50.0),
            "{}",
            p.label()
        );
    }
    Ok(())
}

#[test]
fn the_square_is_clear_of_every_overlap_band_and_mark() -> Result<(), Setup> {
    for paper in [Paper::A4, Paper::Letter] {
        let t = tiling(400.0, 600.0, paper)?;
        for p in t.pages() {
            let lo = p.scale_square_min();
            let hi = lo + DVec2::new(SCALE_SQUARE_MM, SCALE_SQUARE_MM);
            // Bands: 15 mm in from each edge of the printable area.
            assert!(lo.x >= p.window_min().x + 15.0 && lo.y >= p.window_min().y + 15.0);
            assert!(hi.x <= p.window_max().x - 15.0 && hi.y <= p.window_max().y - 15.0);
            for m in t.marks() {
                let hit = m.x >= lo.x && m.x <= hi.x && m.y >= lo.y && m.y <= hi.y;
                assert!(!hit, "{} square covers mark {m:?}", p.label());
            }
        }
    }
    Ok(())
}
