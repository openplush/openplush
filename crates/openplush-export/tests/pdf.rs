//! `pdf::write`: one page per tile, each showing its window of the sheet at true scale.

#[allow(
    dead_code,
    reason = "each test file reads only part of the shared PDF reader"
)]
mod common;

use common::{Setup, pages};
use openplush_export::ExportError;
use openplush_export::pdf::write;
use openplush_geom::DVec2;
use openplush_geom::units::Mm;
use openplush_layout::pages::{MARGIN_MM, Paper, tile};
use openplush_layout::shelf::pack;
use openplush_layout::{LayoutError, LayoutOptions, Sheet};
use openplush_pattern::{CutCount, Outline, PatternPiece, PieceId};

/// An L, `scale` mm per unit, from the origin.
fn ell(scale: f64) -> Result<Outline, Setup> {
    let pts = [
        (0.0, 0.0),
        (3.0, 0.0),
        (3.0, 1.0),
        (1.0, 1.0),
        (1.0, 2.0),
        (0.0, 2.0),
    ];
    Ok(Outline::new(
        pts.iter()
            .map(|&(x, y)| DVec2::new(x * scale, y * scale))
            .collect(),
    )?)
}

/// A sewing line that is an L at 10 mm per unit inside a cut line at 20 mm per unit.
fn piece(id: u32, cuts: u32) -> Result<PatternPiece, Setup> {
    let sewing = Outline::new(
        ell(10.0)?
            .points()
            .iter()
            .map(|p| *p + DVec2::new(5.0, 5.0))
            .collect(),
    )?;
    Ok(PatternPiece::new(
        PieceId::new(id),
        sewing,
        ell(20.0)?,
        CutCount::new(cuts, 0)?,
        vec![0.0],
    )?)
}

fn sheet(pieces: &[PatternPiece], roll: f64) -> Result<Sheet, Setup> {
    let opts = LayoutOptions::new(Mm::try_new(roll)?, Mm::try_new(5.0)?, false)?;
    Ok(pack(pieces, &opts)?
        .into_iter()
        .next()
        .ok_or(LayoutError::NoPieces)?)
}

fn close(a: DVec2, b: DVec2) -> bool {
    (a - b).length() <= 1e-3
}

#[test]
fn each_page_is_its_paper_size() -> Result<(), Setup> {
    let pieces = [piece(0, 1)?];
    let s = sheet(&pieces, 150.0)?;
    for (paper, size) in [
        (Paper::A4, DVec2::new(210.0, 297.0)),
        (Paper::Letter, DVec2::new(215.9, 279.4)),
    ] {
        let read = pages(&write(&s, &pieces, &tile(&s, paper)?)?)?;
        assert_eq!(read.len(), 1);
        assert!(close(read[0].size, size), "{:?}", read[0].size);
    }
    Ok(())
}

#[test]
fn a_page_shows_only_its_printable_area() -> Result<(), Setup> {
    let pieces = [piece(0, 1)?];
    let s = sheet(&pieces, 150.0)?;
    let read = pages(&write(&s, &pieces, &tile(&s, Paper::A4)?)?)?;
    let [clip] = &read[0].clips[..] else {
        return Err(format!("not one clip: {:?}", read[0].clips).into());
    };
    let corners = [
        DVec2::new(MARGIN_MM, MARGIN_MM),
        DVec2::new(210.0 - MARGIN_MM, MARGIN_MM),
        DVec2::new(210.0 - MARGIN_MM, 297.0 - MARGIN_MM),
        DVec2::new(MARGIN_MM, 297.0 - MARGIN_MM),
    ];
    assert_eq!(clip.len(), 4);
    assert!(
        corners.iter().all(|c| clip.iter().any(|p| close(*p, *c))),
        "{clip:?}"
    );
    Ok(())
}

/// Every placed outline as the page should show it: shifted from its window into the margin.
fn expected(s: &Sheet, pieces: &[PatternPiece], window_min: DVec2) -> Vec<Vec<DVec2>> {
    let mut out = Vec::new();
    for pl in s.placements() {
        let Some(p) = pieces.iter().find(|p| p.id() == pl.piece()) else {
            continue;
        };
        for outline in [p.cut_line(), p.sewing_line()] {
            let shown = outline
                .points()
                .iter()
                .map(|&q| pl.apply(q) - window_min + DVec2::new(MARGIN_MM, MARGIN_MM));
            out.push(shown.collect());
        }
    }
    out
}

#[test]
fn every_copy_is_drawn_where_the_sheet_puts_it_cut_line_first() -> Result<(), Setup> {
    let pieces = [piece(0, 2)?, piece(1, 1)?];
    let s = sheet(&pieces, 150.0)?;
    let tiling = tile(&s, Paper::A4)?;
    let read = pages(&write(&s, &pieces, &tiling)?)?;
    let want = expected(&s, &pieces, tiling.pages()[0].window_min());
    // Two outlines per copy, then the two squares.
    let outlines = &read[0].strokes[..want.len()];
    assert_eq!(read[0].strokes.len(), want.len() + 2);
    for (drawn, w) in outlines.iter().zip(&want) {
        assert_eq!(drawn.points.len(), w.len());
        assert!(
            drawn.points.iter().zip(w).all(|(a, b)| close(*a, *b)),
            "{drawn:?} vs {w:?}"
        );
    }
    Ok(())
}

#[test]
fn cut_lines_are_solid_and_sewing_lines_dashed() -> Result<(), Setup> {
    let pieces = [piece(0, 1)?];
    let s = sheet(&pieces, 150.0)?;
    let read = pages(&write(&s, &pieces, &tile(&s, Paper::A4)?)?)?;
    let [cut, sew, ..] = &read[0].strokes[..] else {
        return Err("too few strokes".into());
    };
    assert!(
        (cut.width - 0.5).abs() <= 1e-4 && cut.dash.is_empty(),
        "{cut:?}"
    );
    assert!((sew.width - 0.3).abs() <= 1e-4, "{sew:?}");
    assert!(
        sew.dash.len() == 2
            && (sew.dash[0] - 3.0).abs() <= 1e-4
            && (sew.dash[1] - 2.0).abs() <= 1e-4,
        "{sew:?}"
    );
    Ok(())
}

#[test]
fn a_page_further_down_shows_the_sheet_further_down() -> Result<(), Setup> {
    // A 300 × 500 rectangle spans several pages; each draws it shifted by its own window.
    let rect = Outline::new(vec![
        DVec2::new(0.0, 0.0),
        DVec2::new(300.0, 0.0),
        DVec2::new(300.0, 500.0),
        DVec2::new(0.0, 500.0),
    ])?;
    let pieces = [PatternPiece::new(
        PieceId::new(0),
        rect.clone(),
        rect,
        CutCount::new(1, 0)?,
        vec![0.0],
    )?];
    let s = sheet(&pieces, 320.0)?;
    let tiling = tile(&s, Paper::A4)?;
    let read = pages(&write(&s, &pieces, &tiling)?)?;
    for (page, tile) in read.iter().zip(tiling.pages()) {
        let want = expected(&s, &pieces, tile.window_min());
        assert!(
            page.strokes[0]
                .points
                .iter()
                .zip(&want[0])
                .all(|(a, b)| close(*a, *b))
        );
    }
    Ok(())
}

#[test]
fn the_same_input_gives_the_same_bytes_with_no_date_or_id() -> Result<(), Setup> {
    let pieces = [piece(0, 2)?];
    let s = sheet(&pieces, 150.0)?;
    let tiling = tile(&s, Paper::Letter)?;
    let (a, b) = (write(&s, &pieces, &tiling)?, write(&s, &pieces, &tiling)?);
    assert_eq!(a, b);
    let text = String::from_utf8_lossy(&a);
    for key in ["/CreationDate", "/ModDate", "/ID", "/Info", "/Producer"] {
        assert!(!text.contains(key), "{key}");
    }
    Ok(())
}

#[test]
fn a_placement_of_a_piece_not_given_is_refused() -> Result<(), Setup> {
    let pieces = [piece(0, 1)?];
    let s = sheet(&pieces, 150.0)?;
    let tiling = tile(&s, Paper::A4)?;
    assert_eq!(
        write(&s, &[], &tiling),
        Err(ExportError::UnknownPiece {
            piece: PieceId::new(0)
        })
    );
    let twice = [piece(0, 1)?, piece(0, 1)?];
    assert_eq!(
        write(&s, &twice, &tiling),
        Err(ExportError::DuplicatePiece {
            piece: PieceId::new(0)
        })
    );
    Ok(())
}
