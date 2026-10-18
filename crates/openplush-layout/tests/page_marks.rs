//! Page labels and registration marks: each mark prints on both pages that share its overlap band.

use openplush_geom::DVec2;
use openplush_geom::units::Mm;
use openplush_layout::pages::{Page, PageTiling, Paper, tile};
use openplush_layout::shelf::pack;
use openplush_layout::{LayoutError, LayoutOptions};
use openplush_pattern::{Outline, PatternPiece, PieceId};

type Setup = Box<dyn std::error::Error>;

fn tiling(roll: f64, length: f64, paper: Paper) -> Result<PageTiling, Setup> {
    let outline = Outline::new(vec![
        DVec2::new(0.0, 0.0),
        DVec2::new(10.0, 0.0),
        DVec2::new(10.0, length),
        DVec2::new(0.0, length),
    ])?;
    let piece = PatternPiece::new(PieceId::new(0), outline.clone(), outline, 1, vec![0.0])?;
    let opts = LayoutOptions::new(Mm::try_new(roll)?, Mm::try_new(0.0)?, false)?;
    let sheet = pack(&[piece], &opts)?
        .into_iter()
        .next()
        .ok_or(LayoutError::NoPieces)?;
    Ok(tile(&sheet, paper)?)
}

fn inside(p: DVec2, page: &Page) -> bool {
    let (lo, hi) = (page.window_min(), page.window_max());
    p.x >= lo.x && p.x <= hi.x && p.y >= lo.y && p.y <= hi.y
}

#[test]
fn pages_are_labelled_by_row_letter_and_column_number() -> Result<(), Setup> {
    let t = tiling(400.0, 600.0, Paper::A4)?;
    let labels: Vec<String> = t.pages().iter().map(Page::label).collect();
    assert_eq!(
        labels,
        ["A1", "A2", "A3", "B1", "B2", "B3", "C1", "C2", "C3"]
    );
    Ok(())
}

#[test]
fn the_26th_row_is_z() -> Result<(), Setup> {
    let t = tiling(100.0, 6827.0, Paper::A4)?;
    let last = t.pages().last().map(Page::label);
    assert_eq!(last.as_deref(), Some("Z1"));
    Ok(())
}

#[test]
fn one_page_has_no_registration_marks() -> Result<(), Setup> {
    assert!(tiling(100.0, 150.0, Paper::A4)?.marks().is_empty());
    Ok(())
}

#[test]
fn every_mark_prints_on_exactly_two_pages_inside_their_shared_band() -> Result<(), Setup> {
    for paper in [Paper::A4, Paper::Letter] {
        let t = tiling(400.0, 600.0, paper)?;
        // Two marks per shared edge: 3 rows of 2 vertical edges, 2 horizontal edges of 3 columns.
        assert_eq!(t.marks().len(), 2 * (3 * 2 + 2 * 3));
        for &m in t.marks() {
            let on: Vec<&Page> = t.pages().iter().filter(|p| inside(m, p)).collect();
            assert_eq!(on.len(), 2, "{m:?} on {paper:?}");
            let (a, b) = (on[0], on[1]);
            // Within the band: no further than 15 mm in from the right or bottom edge of the earlier page.
            let band = if a.row() == b.row() {
                a.window_max().x - m.x
            } else {
                m.y - a.window_min().y
            };
            assert!(
                (0.0..=15.0).contains(&band),
                "{m:?} is {band} mm into the band"
            );
        }
    }
    Ok(())
}

#[test]
fn marks_sit_mid_band_a_quarter_and_three_quarters_along_the_edge() -> Result<(), Setup> {
    // A4 prints 190 by 277 mm with a 175 by 262 mm step; rows hang down from the sheet's top edge.
    let t = tiling(300.0, 150.0, Paper::A4)?;
    assert_eq!(
        t.marks(),
        [DVec2::new(182.5, 80.75), DVec2::new(182.5, -57.75)]
    );
    let t = tiling(100.0, 400.0, Paper::A4)?;
    assert_eq!(
        t.marks(),
        [DVec2::new(47.5, 130.5), DVec2::new(142.5, 130.5)]
    );
    Ok(())
}

#[test]
fn marks_list_the_edges_between_columns_row_by_row_then_those_between_rows() -> Result<(), Setup> {
    let t = tiling(300.0, 400.0, Paper::A4)?;
    let expected = [
        (182.5, 330.75),
        (182.5, 192.25),
        (182.5, 68.75),
        (182.5, -69.75),
        (47.5, 130.5),
        (142.5, 130.5),
        (222.5, 130.5),
        (317.5, 130.5),
    ]
    .map(|(x, y)| DVec2::new(x, y));
    assert_eq!(t.marks(), expected);
    Ok(())
}
