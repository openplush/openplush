//! Notches on the print: each one drawn as `marks` ticks side by side, 1.5 mm apart, each running
//! from the sewing line out to the cut line, and at least 3 mm long where the allowance is
//! narrower; the same ticks in the SVG and on the PDF tiles.
#![allow(
    dead_code,
    reason = "each test file reads only part of the shared PDF reader"
)]

use openplush_export::{pdf, svg};
use openplush_geom::DVec2;
use openplush_geom::units::Mm;
use openplush_layout::pages::{Paper, tile};
use openplush_layout::shelf::pack;
use openplush_layout::{LayoutError, LayoutOptions, Sheet};
use openplush_pattern::{CutCount, Notch, Outline, PatternPiece, PieceId};
use openplush_seams::SeamChainId;

mod common;

use common::{Setup, tiles};

const SAME_MM: f64 = 1e-3;

fn square(min: f64, max: f64) -> Result<Outline, Setup> {
    Ok(Outline::new(vec![
        DVec2::new(min, min),
        DVec2::new(max, min),
        DVec2::new(max, max),
        DVec2::new(min, max),
    ])?)
}

/// A 40 mm square sewn 6 mm inside its cut line, with a single notch on its bottom edge and a
/// triple one on its right edge; `allowance` 0 cuts it on its sewing line.
fn piece(allowance: f64) -> Result<PatternPiece, Setup> {
    let chain = SeamChainId::new(0);
    let notches = vec![
        Notch::new(DVec2::new(20.0, 0.0), DVec2::new(0.0, -1.0), chain, 1)?,
        Notch::new(DVec2::new(40.0, 20.0), DVec2::new(1.0, 0.0), chain, 3)?,
    ];
    Ok(PatternPiece::new(
        PieceId::new(0),
        square(0.0, 40.0)?,
        square(-allowance, 40.0 + allowance)?,
        CutCount::new(1, 0)?,
        vec![0.0],
    )?
    .with_notches(notches))
}

fn sheet(pieces: &[PatternPiece]) -> Result<Sheet, Setup> {
    let opts = LayoutOptions::new(Mm::try_new(100.0)?, Mm::try_new(0.0)?, false)?;
    Ok(pack(pieces, &opts)?
        .into_iter()
        .next()
        .ok_or(LayoutError::NoPieces)?)
}

/// Each tick's two ends, from the `notch` path's `M x y L x y` pairs.
fn svg_ticks(svg: &str) -> Result<Vec<[DVec2; 2]>, Setup> {
    let chunk = svg
        .split("<path class=\"notch\" d=\"")
        .nth(1)
        .ok_or("no notch path")?;
    let d = chunk.split('"').next().ok_or("unterminated d")?;
    let nums: Vec<f64> = d
        .split(' ')
        .filter(|t| !matches!(*t, "M" | "L"))
        .map(str::parse)
        .collect::<Result<_, _>>()?;
    Ok(nums
        .chunks(4)
        .map(|c| [DVec2::new(c[0], c[1]), DVec2::new(c[2], c[3])])
        .collect())
}

/// The ticks' lengths, sorted, and the gaps between the three ticks across the right edge.
fn assert_shape(ticks: &[[DVec2; 2]], length: f64) {
    assert_eq!(ticks.len(), 4, "{ticks:?}");
    for [a, b] in ticks {
        assert!(
            ((*b - *a).length() - length).abs() <= SAME_MM,
            "{a:?} {b:?}"
        );
    }
    let mut level: Vec<[DVec2; 2]> = ticks
        .iter()
        .copied()
        .filter(|[a, b]| (a.y - b.y).abs() <= SAME_MM)
        .collect();
    assert_eq!(level.len(), 3, "three ticks across the right edge");
    level.sort_by(|p, q| p[0].y.total_cmp(&q[0].y));
    for w in level.windows(2) {
        assert!(((w[1][0].y - w[0][0].y).abs() - 1.5).abs() <= SAME_MM);
    }
}

#[test]
fn each_notch_is_drawn_as_its_marks_across_the_allowance_in_the_svg() -> Result<(), Setup> {
    let pieces = [piece(6.0)?];
    let s = sheet(&pieces)?;
    let text = String::from_utf8(svg::write(&s, &pieces, &[])?)?;
    assert!(text.contains(".notch{fill:none;stroke:#000;stroke-width:0.4}"));
    let ticks = svg_ticks(&text)?;
    assert_shape(&ticks, 6.0);
    // The packed piece's cut line starts at the sheet's origin, so its sewing line is 6 mm in
    // and the bottom notch at x = 26; SVG's y runs down from the 52 mm sheet's far end.
    let bottom = ticks
        .iter()
        .find(|[a, b]| (a.x - b.x).abs() <= SAME_MM)
        .ok_or("no upright tick")?;
    assert!(
        (bottom[0] - DVec2::new(26.0, 52.0 - 6.0)).length() <= SAME_MM,
        "{bottom:?}"
    );
    assert!(
        (bottom[1] - DVec2::new(26.0, 52.0)).length() <= SAME_MM,
        "{bottom:?}"
    );
    Ok(())
}

#[test]
fn a_narrow_allowance_still_gets_a_3_mm_tick() -> Result<(), Setup> {
    let pieces = [piece(0.0)?];
    let s = sheet(&pieces)?;
    let text = String::from_utf8(svg::write(&s, &pieces, &[])?)?;
    assert_shape(&svg_ticks(&text)?, 3.0);
    Ok(())
}

#[test]
fn a_piece_with_no_notches_draws_no_notch_path() -> Result<(), Setup> {
    let bare = PatternPiece::new(
        PieceId::new(0),
        square(0.0, 40.0)?,
        square(-6.0, 46.0)?,
        CutCount::new(1, 0)?,
        vec![0.0],
    )?;
    let pieces = [bare];
    let s = sheet(&pieces)?;
    let text = String::from_utf8(svg::write(&s, &pieces, &[])?)?;
    assert!(!text.contains("<path class=\"notch\""));
    Ok(())
}

#[test]
fn the_pdf_draws_the_same_ticks_solid_and_0_4_mm_wide() -> Result<(), Setup> {
    let pieces = [piece(6.0)?];
    let s = sheet(&pieces)?;
    let tiling = tile(&s, Paper::A4)?;
    let pages = tiles(&pdf::write(&s, &pieces, &tiling, &[])?)?;
    let [page] = &pages[..] else {
        return Err(format!("one tile, got {}", pages.len()).into());
    };
    let ticks: Vec<[DVec2; 2]> = page
        .strokes
        .iter()
        .filter(|st| (st.width - 0.4).abs() <= SAME_MM && st.points.len() == 2)
        .map(|st| {
            assert!(st.dash.is_empty(), "{st:?}");
            [st.points[0], st.points[1]]
        })
        .collect();
    assert_shape(&ticks, 6.0);
    Ok(())
}
