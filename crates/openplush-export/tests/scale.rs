//! The scale-check square on every PDF page measures exactly 50.00 mm in the file, read back by
//! an independent content-stream reader.

#[allow(
    dead_code,
    reason = "each test file reads only part of the shared PDF reader"
)]
mod common;

use common::{Page, Setup, pages};
use openplush_export::pdf::write;
use openplush_geom::DVec2;
use openplush_geom::units::Mm;
use openplush_layout::pages::{MARGIN_MM, Paper, tile};
use openplush_layout::shelf::pack;
use openplush_layout::{LayoutError, LayoutOptions, Sheet};
use openplush_pattern::{CutCount, Outline, PatternPiece, PieceId};

/// A 300 × 500 mm rectangle cut on a 320 mm roll: two pages across and two or three down.
fn big() -> Result<(Sheet, Vec<PatternPiece>), Setup> {
    let outline = Outline::new(vec![
        DVec2::new(0.0, 0.0),
        DVec2::new(300.0, 0.0),
        DVec2::new(300.0, 500.0),
        DVec2::new(0.0, 500.0),
    ])?;
    let piece = PatternPiece::new(
        PieceId::new(0),
        outline.clone(),
        outline,
        CutCount::new(1, 0)?,
        vec![0.0],
    )?;
    let opts = LayoutOptions::new(Mm::try_new(320.0)?, Mm::try_new(0.0)?, false)?;
    let pieces = vec![piece];
    let sheet = pack(&pieces, &opts)?
        .into_iter()
        .next()
        .ok_or(LayoutError::NoPieces)?;
    Ok((sheet, pieces))
}

/// The page's stroked squares of side `side` mm, as their lowest corners.
fn squares(page: &Page, side: f64) -> Vec<DVec2> {
    page.strokes
        .iter()
        .filter_map(|s| {
            let p = &s.points;
            let min = p.iter().fold(DVec2::new(f64::MAX, f64::MAX), |a, q| {
                DVec2::new(a.x.min(q.x), a.y.min(q.y))
            });
            let max = p.iter().fold(DVec2::new(f64::MIN, f64::MIN), |a, q| {
                DVec2::new(a.x.max(q.x), a.y.max(q.y))
            });
            let square = p.len() == 4
                && ((max.x - min.x) - side).abs() <= 0.005
                && ((max.y - min.y) - side).abs() <= 0.005;
            square.then_some(min)
        })
        .collect()
}

#[test]
fn every_page_has_one_50_mm_square_with_a_one_inch_square_in_its_top_left_corner()
-> Result<(), Setup> {
    for paper in [Paper::A4, Paper::Letter] {
        let (sheet, pieces) = big()?;
        let tiling = tile(&sheet, paper)?;
        let read = pages(&write(&sheet, &pieces, &tiling)?)?;
        assert!(read.len() >= 4, "{} pages", read.len());
        assert_eq!(read.len(), tiling.pages().len());
        for page in &read {
            let [big] = squares(page, 50.0)[..] else {
                return Err(format!("not one 50 mm square: {:?}", squares(page, 50.0)).into());
            };
            let [inch] = squares(page, 25.4)[..] else {
                return Err(format!("not one 1 in square: {:?}", squares(page, 25.4)).into());
            };
            // Top left inside the margin, inset by the 15 mm overlap.
            let top = page.size.y - MARGIN_MM - 15.0;
            assert!((big.x - (MARGIN_MM + 15.0)).abs() <= 0.005, "{big:?}");
            assert!((big.y - (top - 50.0)).abs() <= 0.005, "{big:?}");
            assert!((inch.x - big.x).abs() <= 0.005, "{inch:?}");
            assert!((inch.y - (top - 25.4)).abs() <= 0.005, "{inch:?}");
        }
    }
    Ok(())
}
