//! `pdf::write`'s page furniture: registration crosshairs on both pages of every shared edge, each
//! page's label, and the labels on the scale squares.

#[allow(
    dead_code,
    reason = "each test file reads only part of the shared PDF reader"
)]
mod common;

use common::{Page, Setup, rectangle, tiles};
use openplush_export::pdf::write;
use openplush_geom::DVec2;
use openplush_layout::Sheet;
use openplush_layout::pages::{MARGIN_MM, PageTiling, Paper, tile};
use openplush_pattern::PatternPiece;

/// A `w` × `h` mm rectangle on a roll just wider, and its tiling on A4.
fn tiled(w: f64, h: f64) -> Result<(Sheet, Vec<PatternPiece>, PageTiling), Setup> {
    let (sheet, pieces) = rectangle(w, h)?;
    let tiling = tile(&sheet, Paper::A4)?;
    Ok((sheet, pieces, tiling))
}

/// The page's crosshairs: pairs of 10 mm strokes, one level and one upright, crossing at their
/// middles. Returns the crossing points.
fn crosshairs(page: &Page) -> Vec<DVec2> {
    let segment = |p: &[DVec2]| (p.len() == 2).then(|| (p[0], p[1]));
    let level: Vec<_> = page
        .strokes
        .iter()
        .filter_map(|s| segment(&s.points))
        .filter(|(a, b)| (a.y - b.y).abs() < 1e-3 && ((a.x - b.x).abs() - 10.0).abs() < 1e-3)
        .collect();
    let upright: Vec<_> = page
        .strokes
        .iter()
        .filter_map(|s| segment(&s.points))
        .filter(|(a, b)| (a.x - b.x).abs() < 1e-3 && ((a.y - b.y).abs() - 10.0).abs() < 1e-3)
        .collect();
    level
        .iter()
        .map(|(a, b)| (*a + *b) * 0.5)
        .filter(|m| {
            upright
                .iter()
                .any(|(c, d)| ((*c + *d) * 0.5 - *m).length() < 1e-3)
        })
        .collect()
}

fn on_page(sheet_point: DVec2, window_min: DVec2) -> DVec2 {
    sheet_point - window_min + DVec2::new(MARGIN_MM, MARGIN_MM)
}

#[test]
fn every_mark_is_a_crosshair_on_each_page_whose_window_holds_it() -> Result<(), Setup> {
    let (sheet, pieces, tiling) = tiled(300.0, 500.0)?;
    assert!(!tiling.marks().is_empty());
    let read = tiles(&write(&sheet, &pieces, &tiling, &[])?)?;
    let mut seen = vec![0; tiling.marks().len()];
    for (page, tile) in read.iter().zip(tiling.pages()) {
        let drawn = crosshairs(page);
        let (lo, hi) = (tile.window_min(), tile.window_max());
        let inside: Vec<(usize, DVec2)> = tiling
            .marks()
            .iter()
            .enumerate()
            .filter(|(_, m)| lo.x <= m.x && m.x <= hi.x && lo.y <= m.y && m.y <= hi.y)
            .map(|(i, m)| (i, on_page(*m, lo)))
            .collect();
        for (i, at) in &inside {
            assert!(
                drawn.iter().any(|d| (*d - *at).length() < 1e-3),
                "mark {i} at {at:?}"
            );
            seen[*i] += 1;
        }
        assert_eq!(drawn.len(), inside.len(), "{drawn:?}");
    }
    // Each mark sits mid-band on an edge two pages share.
    assert!(seen.iter().all(|&n| n == 2), "{seen:?}");
    Ok(())
}

#[test]
fn a_single_page_has_no_crosshairs() -> Result<(), Setup> {
    let (sheet, pieces, tiling) = tiled(100.0, 100.0)?;
    let read = tiles(&write(&sheet, &pieces, &tiling, &[])?)?;
    assert_eq!(read.len(), 1);
    assert!(crosshairs(&read[0]).is_empty());
    Ok(())
}

#[test]
fn every_page_is_labelled_right_of_its_scale_square() -> Result<(), Setup> {
    let (sheet, pieces, tiling) = tiled(300.0, 500.0)?;
    let read = tiles(&write(&sheet, &pieces, &tiling, &[])?)?;
    for (page, tile) in read.iter().zip(tiling.pages()) {
        let square = on_page(tile.scale_square_min(), tile.window_min());
        let labels: Vec<_> = page
            .texts
            .iter()
            .filter(|t| t.text == tile.label())
            .collect();
        let [label] = labels[..] else {
            return Err(format!("{}: {:?}", tile.label(), page.texts).into());
        };
        assert!((label.size - 5.0).abs() < 1e-3, "{label:?}");
        // Baseline 5 mm right of the square, its cap height level with the square's top.
        assert!((label.at.x - (square.x + 55.0)).abs() < 1e-3, "{label:?}");
        assert!((label.at.y - (square.y + 45.0)).abs() < 1e-3, "{label:?}");
    }
    Ok(())
}

#[test]
fn the_squares_say_what_they_measure() -> Result<(), Setup> {
    let (sheet, pieces, tiling) = tiled(300.0, 500.0)?;
    let read = tiles(&write(&sheet, &pieces, &tiling, &[])?)?;
    for (page, tile) in read.iter().zip(tiling.pages()) {
        let square = on_page(tile.scale_square_min(), tile.window_min());
        let find = |text: &str| page.texts.iter().find(|t| t.text == text);
        let metric = find("50 mm").ok_or("no 50 mm label")?;
        let inch = find("1 in").ok_or("no 1 in label")?;
        // Inside the 50 mm square, below the inch square; inside the inch square.
        let inch_square_y = square.y + 50.0 - 25.4;
        assert!(
            metric.at.x > square.x && metric.at.x < square.x + 50.0,
            "{metric:?}"
        );
        assert!(
            metric.at.y > square.y && metric.at.y < inch_square_y,
            "{metric:?}"
        );
        assert!(
            inch.at.x > square.x && inch.at.x < square.x + 25.4,
            "{inch:?}"
        );
        assert!(
            inch.at.y > inch_square_y && inch.at.y < square.y + 50.0,
            "{inch:?}"
        );
        assert!((metric.size - 3.0).abs() < 1e-3 && (inch.size - 3.0).abs() < 1e-3);
    }
    Ok(())
}
