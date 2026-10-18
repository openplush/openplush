//! `pages::tile`: overlapping print pages covering a sheet, 15 mm overlap inside a 10 mm margin.

use openplush_geom::DVec2;
use openplush_geom::units::Mm;
use openplush_layout::pages::{Paper, tile};
use openplush_layout::shelf::pack;
use openplush_layout::{LayoutError, LayoutOptions, Sheet};
use openplush_pattern::{Outline, PatternPiece, PieceId};

type Setup = Box<dyn std::error::Error>;

/// A sheet `roll` wide and `length` long, made the only way a sheet is made: by packing one piece.
fn sheet(roll: f64, length: f64) -> Result<Sheet, Setup> {
    let w = roll.min(10.0);
    let outline = Outline::new(vec![
        DVec2::new(0.0, 0.0),
        DVec2::new(w, 0.0),
        DVec2::new(w, length),
        DVec2::new(0.0, length),
    ])?;
    let piece = PatternPiece::new(PieceId::new(0), outline.clone(), outline, 1, vec![0.0])?;
    let opts = LayoutOptions::new(Mm::try_new(roll)?, Mm::try_new(0.0)?, false)?;
    pack(&[piece], &opts)?
        .into_iter()
        .next()
        .ok_or_else(|| LayoutError::NoPieces.into())
}

#[test]
fn a_small_sheet_fits_on_one_a4_page() -> Result<(), Setup> {
    let t = tile(&sheet(100.0, 150.0)?, Paper::A4)?;
    assert_eq!((t.rows(), t.columns()), (1, 1));
    let p = &t.pages()[0];
    assert_eq!((p.row(), p.column()), (0, 0));
    // Row A hangs from the sheet's top edge, 150 mm up.
    assert_eq!(p.window_min(), DVec2::new(0.0, -127.0));
    assert_eq!(p.window_max(), DVec2::new(190.0, 150.0));
    Ok(())
}

#[test]
fn letter_prints_inside_a_10_mm_margin() -> Result<(), Setup> {
    let t = tile(&sheet(100.0, 150.0)?, Paper::Letter)?;
    let p = &t.pages()[0];
    let size = p.window_max() - p.window_min();
    assert!((size.x - 195.9).abs() < 1e-9 && (size.y - 259.4).abs() < 1e-9);
    assert_eq!(t.paper(), Paper::Letter);
    Ok(())
}

#[test]
fn pages_run_row_by_row_and_overlap_by_15_mm() -> Result<(), Setup> {
    // A4 prints 190 by 277 mm; each next page starts 15 mm before the last one ends.
    let t = tile(&sheet(400.0, 600.0)?, Paper::A4)?;
    assert_eq!((t.rows(), t.columns()), (3, 3));
    let order: Vec<(u32, u32)> = t.pages().iter().map(|p| (p.row(), p.column())).collect();
    let expected: Vec<(u32, u32)> = (0..3).flat_map(|r| (0..3).map(move |c| (r, c))).collect();
    assert_eq!(order, expected);
    let p = &t.pages()[5];
    assert_eq!((p.row(), p.column()), (1, 2));
    assert_eq!(p.window_min(), DVec2::new(350.0, 61.0));
    assert_eq!(p.window_max(), DVec2::new(540.0, 338.0));
    Ok(())
}

/// Whether `n` pages of `printable` mm, each overlapping the last by 15 mm, reach `extent`.
fn covers(n: u32, extent: f64, printable: f64) -> bool {
    n >= 1 && f64::from(n - 1) * (printable - 15.0) + printable >= extent
}

#[test]
fn the_grid_is_the_fewest_pages_that_cover_the_sheet() -> Result<(), Setup> {
    for (w, l) in [
        (10.0, 10.0),
        (189.0, 276.0),
        (191.0, 278.0),
        (366.0, 540.0),
        (1500.0, 2200.0),
    ] {
        let t = tile(&sheet(w, l)?, Paper::A4)?;
        let (c, r) = (t.columns(), t.rows());
        assert!(covers(c, w, 190.0) && !covers(c - 1, w, 190.0), "width {w}");
        assert!(
            covers(r, l, 277.0) && !covers(r - 1, l, 277.0),
            "length {l}"
        );
        assert_eq!(t.pages().len(), (r * c) as usize);
        let last = &t.pages()[t.pages().len() - 1];
        assert!(last.window_max().x >= w && last.window_min().y <= 0.0);
    }
    Ok(())
}

#[test]
fn up_to_26_pages_across_and_down_is_allowed() -> Result<(), Setup> {
    // 190 + 25 * 175 = 4565 mm across; 277 + 25 * 262 = 6827 mm down.
    let t = tile(&sheet(4565.0, 6827.0)?, Paper::A4)?;
    assert_eq!((t.rows(), t.columns()), (26, 26));
    Ok(())
}

#[test]
fn a_27th_page_either_way_is_refused() -> Result<(), Setup> {
    for (w, l) in [(4566.0, 100.0), (100.0, 6828.0)] {
        assert_eq!(
            tile(&sheet(w, l)?, Paper::A4),
            Err(LayoutError::TooManyPages {
                width_mm: w,
                length_mm: l
            })
        );
    }
    Ok(())
}

#[test]
fn too_many_pages_says_what_is_wrong() {
    let err = LayoutError::TooManyPages {
        width_mm: 4566.0,
        length_mm: 100.0,
    };
    assert_eq!(
        err.to_string(),
        "a 4566.000 by 100.000 mm sheet needs more than 26 pages across or down"
    );
}

#[test]
fn a_wide_short_sheet_is_one_row_of_pages() -> Result<(), Setup> {
    let t = tile(&sheet(400.0, 150.0)?, Paper::A4)?;
    assert_eq!((t.rows(), t.columns()), (1, 3));
    assert_eq!(t.pages()[2].window_min(), DVec2::new(350.0, -127.0));
    Ok(())
}

#[test]
fn rows_run_down_from_the_top_edge_each_repeating_15_mm_at_its_bottom() -> Result<(), Setup> {
    // Sheets are y-up, so the top edge is the highest y and row A starts there.
    let t = tiling_columns(&sheet(100.0, 600.0)?)?;
    assert_eq!(t[0].window_max().y, 600.0);
    for pair in t.windows(2) {
        assert_eq!(pair[1].window_max().y, pair[0].window_min().y + 15.0);
        assert!(pair[1].window_min().y < pair[0].window_min().y);
    }
    Ok(())
}

/// The pages of a one-column tiling on A4, top row first.
fn tiling_columns(s: &Sheet) -> Result<Vec<openplush_layout::pages::Page>, Setup> {
    let t = tile(s, Paper::A4)?;
    assert_eq!(t.columns(), 1);
    Ok(t.pages().to_vec())
}
