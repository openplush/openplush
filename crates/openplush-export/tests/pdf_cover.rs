//! `pdf::write`'s cover page: the scale check and a map of the tiles, ahead of the tiles.

#[allow(
    dead_code,
    reason = "each test file reads only part of the shared PDF reader"
)]
mod common;

use common::{Page, Setup, pages, rectangle};
use openplush_export::pdf::write;
use openplush_geom::DVec2;
use openplush_layout::Sheet;
use openplush_layout::pages::{
    INCH_SQUARE_MM, MARGIN_MM, PageTiling, Paper, SCALE_SQUARE_MM, tile,
};
use openplush_pattern::PatternPiece;

/// Tall, single-page and wide sheets, so the map is fitted by its height, by a page that is
/// nearly the cover's size, and by its width.
const CASES: [(f64, f64, Paper); 3] = [
    (300.0, 500.0, Paper::A4),
    (100.0, 100.0, Paper::Letter),
    (700.0, 150.0, Paper::A4),
];

const FURNITURE: [&str; 3] = ["50 mm", "1 in", "Page map"];

struct Case {
    sheet: Sheet,
    pieces: Vec<PatternPiece>,
    tiling: PageTiling,
    cover: Page,
    tiles: usize,
}

fn case(w: f64, h: f64, paper: Paper) -> Result<Case, Setup> {
    let (sheet, pieces) = rectangle(w, h)?;
    let tiling = tile(&sheet, paper)?;
    let mut read = pages(&write(&sheet, &pieces, &tiling)?)?;
    let tiles = read.len().checked_sub(1).ok_or("no pages")?;
    let cover = read.remove(0);
    Ok(Case {
        sheet,
        pieces,
        tiling,
        cover,
        tiles,
    })
}

fn close(a: DVec2, b: DVec2) -> bool {
    (a - b).length() <= 1e-3
}

fn bounds(points: &[DVec2]) -> (DVec2, DVec2) {
    let start = (
        DVec2::new(f64::MAX, f64::MAX),
        DVec2::new(f64::MIN, f64::MIN),
    );
    points.iter().fold(start, |(lo, hi), p| {
        (
            DVec2::new(lo.x.min(p.x), lo.y.min(p.y)),
            DVec2::new(hi.x.max(p.x), hi.y.max(p.y)),
        )
    })
}

/// Every black four-point stroke but the scale squares, as its bounds.
fn map_rectangles(cover: &Page) -> Vec<(DVec2, DVec2)> {
    let square = |side: DVec2| {
        [SCALE_SQUARE_MM, INCH_SQUARE_MM]
            .iter()
            .any(|&s| (side.x - s).abs() <= 1e-3 && (side.y - s).abs() <= 1e-3)
    };
    cover
        .strokes
        .iter()
        .filter(|s| s.points.len() == 4 && s.gray == 0.0)
        .map(|s| bounds(&s.points))
        .filter(|&(lo, hi)| !square(hi - lo))
        .collect()
}

/// Where the map puts sheet millimetres on the cover: `origin + scale * (p - low)`, `low` being
/// the lowest corner any tile shows. Found from the rectangles' extent, and checked uniform.
struct Map {
    low: DVec2,
    origin: DVec2,
    scale: f64,
}

impl Map {
    fn of(c: &Case) -> Result<Self, Setup> {
        let corners: Vec<DVec2> = c
            .tiling
            .pages()
            .iter()
            .flat_map(|p| [p.window_min(), p.window_max()])
            .collect();
        let (low, high) = bounds(&corners);
        let drawn: Vec<DVec2> = map_rectangles(&c.cover)
            .into_iter()
            .flat_map(|(lo, hi)| [lo, hi])
            .collect();
        let (origin, top) = bounds(&drawn);
        let scale = (top.x - origin.x) / (high.x - low.x);
        let other = (top.y - origin.y) / (high.y - low.y);
        if !(scale > 0.0 && (scale - other).abs() <= 1e-5) {
            return Err(format!("not one scale: {scale} across, {other} down").into());
        }
        Ok(Self { low, origin, scale })
    }

    fn at(&self, p: DVec2) -> DVec2 {
        self.origin + (p - self.low) * self.scale
    }
}

fn has_rectangle(rects: &[(DVec2, DVec2)], lo: DVec2, hi: DVec2) -> bool {
    rects.iter().any(|&(a, b)| close(a, lo) && close(b, hi))
}

#[test]
fn the_cover_comes_first_on_the_same_paper_and_every_tile_follows() -> Result<(), Setup> {
    for (w, h, paper) in CASES {
        let c = case(w, h, paper)?;
        assert_eq!(c.tiles, c.tiling.pages().len());
        assert!(close(c.cover.size, paper.size_mm()), "{:?}", c.cover.size);
        assert!(c.cover.clips.is_empty(), "the cover clips nothing");
    }
    Ok(())
}

#[test]
fn the_cover_has_the_scale_squares_at_the_top_left_of_its_printable_area() -> Result<(), Setup> {
    let c = case(300.0, 500.0, Paper::A4)?;
    let top = c.cover.size.y - MARGIN_MM;
    let square = |side: f64| {
        let (lo, hi) = (
            DVec2::new(MARGIN_MM, top - side),
            DVec2::new(MARGIN_MM + side, top),
        );
        c.cover
            .strokes
            .iter()
            .filter(|s| s.points.len() == 4)
            .map(|s| bounds(&s.points))
            .any(|(a, b)| close(a, lo) && close(b, hi))
    };
    assert!(square(SCALE_SQUARE_MM) && square(INCH_SQUARE_MM));
    let label = |text: &str| c.cover.texts.iter().find(|t| t.text == text);
    assert!(label("50 mm").is_some() && label("1 in").is_some());
    let name = label("Page map").ok_or("the cover is named")?;
    assert!(name.at.x > MARGIN_MM + SCALE_SQUARE_MM, "{name:?}");
    assert!(
        top - SCALE_SQUARE_MM < name.at.y && name.at.y < top,
        "{name:?}"
    );
    Ok(())
}

#[test]
fn the_map_draws_the_sheet_and_every_tile_window_at_one_scale() -> Result<(), Setup> {
    for (w, h, paper) in CASES {
        let c = case(w, h, paper)?;
        let map = Map::of(&c)?;
        let rects = map_rectangles(&c.cover);
        assert_eq!(rects.len(), c.tiling.pages().len() + 1, "{rects:?}");
        for page in c.tiling.pages() {
            let (lo, hi) = (map.at(page.window_min()), map.at(page.window_max()));
            assert!(has_rectangle(&rects, lo, hi), "{}: {rects:?}", page.label());
        }
        let size = DVec2::new(c.sheet.width_mm(), c.sheet.length_mm());
        assert!(has_rectangle(&rects, map.at(DVec2::ZERO), map.at(size)));
    }
    Ok(())
}

#[test]
fn the_map_fills_the_printable_area_below_the_squares() -> Result<(), Setup> {
    for (w, h, paper) in CASES {
        let c = case(w, h, paper)?;
        let map = Map::of(&c)?;
        let drawn: Vec<DVec2> = map_rectangles(&c.cover)
            .into_iter()
            .flat_map(|(lo, hi)| [lo, hi])
            .collect();
        let (lo, hi) = bounds(&drawn);
        let page = c.cover.size;
        let below_squares = page.y - MARGIN_MM - SCALE_SQUARE_MM;
        assert!(
            lo.x >= MARGIN_MM - 1e-3 && lo.y >= MARGIN_MM - 1e-3,
            "{lo:?}"
        );
        assert!(
            hi.x <= page.x - MARGIN_MM + 1e-3 && hi.y < below_squares,
            "{hi:?}"
        );
        // Fitted: it reaches the printable area's side or its bottom.
        let across = (hi.x - lo.x - (page.x - 2.0 * MARGIN_MM)).abs() <= 1e-3;
        let down = (lo.y - MARGIN_MM).abs() <= 1e-3;
        assert!(across || down, "{lo:?} {hi:?} at scale {}", map.scale);
    }
    Ok(())
}

#[test]
fn each_tile_label_sits_inside_its_window_on_the_map() -> Result<(), Setup> {
    for (w, h, paper) in CASES {
        let c = case(w, h, paper)?;
        let map = Map::of(&c)?;
        let labels: Vec<_> = c
            .cover
            .texts
            .iter()
            .filter(|t| !FURNITURE.contains(&t.text.as_str()))
            .collect();
        assert_eq!(labels.len(), c.tiling.pages().len(), "{labels:?}");
        for page in c.tiling.pages() {
            let (lo, hi) = (map.at(page.window_min()), map.at(page.window_max()));
            let label = labels
                .iter()
                .find(|t| t.text == page.label())
                .ok_or(page.label())?;
            let inside = lo.x < label.at.x && label.at.x < hi.x;
            let below_top = lo.y < label.at.y && label.at.y + label.size < hi.y;
            assert!(inside && below_top, "{label:?} in {lo:?}..{hi:?}");
        }
    }
    Ok(())
}

#[test]
fn every_placed_copy_is_drawn_faint_on_the_map() -> Result<(), Setup> {
    for (w, h, paper) in CASES {
        let c = case(w, h, paper)?;
        let map = Map::of(&c)?;
        let faint: Vec<_> = c.cover.strokes.iter().filter(|s| s.gray > 0.0).collect();
        let placements = c.sheet.placements();
        assert_eq!(faint.len(), placements.len());
        for (stroke, placement) in faint.iter().zip(placements) {
            assert!(stroke.gray < 1.0, "visible on white paper");
            let piece = c
                .pieces
                .iter()
                .find(|p| p.id() == placement.piece())
                .ok_or("placed piece")?;
            let want: Vec<DVec2> = piece
                .cut_line()
                .points()
                .iter()
                .map(|&p| map.at(placement.apply(p)))
                .collect();
            assert_eq!(stroke.points.len(), want.len());
            assert!(stroke.points.iter().zip(&want).all(|(a, b)| close(*a, *b)));
        }
    }
    Ok(())
}
