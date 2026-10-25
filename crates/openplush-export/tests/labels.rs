//! Piece labels on the print: the block's lines centred on the label point, one under another,
//! and each seam label at its point, in the SVG and on the PDF tiles; text outside Latin-1 is
//! refused.
#![allow(
    dead_code,
    reason = "each test file reads only part of the shared PDF reader"
)]

use openplush_export::{ExportError, pdf, svg};
use openplush_geom::DVec2;
use openplush_geom::units::Mm;
use openplush_layout::pages::{MARGIN_MM, Paper, tile};
use openplush_layout::shelf::pack;
use openplush_layout::{LayoutError, LayoutOptions, Sheet};
use openplush_pattern::{CutCount, Label, Outline, PatternPiece, PieceId, SeamLabel};

mod common;

use common::{Setup, tiles};

fn square(min: f64, max: f64) -> Result<Outline, Setup> {
    Ok(Outline::new(vec![
        DVec2::new(min, min),
        DVec2::new(max, min),
        DVec2::new(max, max),
        DVec2::new(min, max),
    ])?)
}

/// A 60 mm square sewn 6 mm inside its cut line, labelled at its centre with `fabric` as its
/// third line, and one seam label 4 mm inside its bottom edge.
fn piece(fabric: &str) -> Result<PatternPiece, Setup> {
    let lines = ["Piece 0", "Cut 1", fabric].map(str::to_owned).to_vec();
    let seam = SeamLabel::new(DVec2::new(30.0, 4.0), "Seam 3".to_owned())?;
    let label = Label::new(DVec2::new(30.0, 30.0), lines, vec![seam])?;
    Ok(PatternPiece::new(
        PieceId::new(0),
        square(0.0, 60.0)?,
        square(-6.0, 66.0)?,
        CutCount::new(1, 0)?,
        vec![0.0],
    )?
    .with_label(label))
}

fn sheet(pieces: &[PatternPiece]) -> Result<Sheet, Setup> {
    let opts = LayoutOptions::new(Mm::try_new(100.0)?, Mm::try_new(0.0)?, false)?;
    Ok(pack(pieces, &opts)?
        .into_iter()
        .next()
        .ok_or(LayoutError::NoPieces)?)
}

/// Each `label` text element's text and anchor, in document order.
fn svg_labels(svg: &str) -> Result<Vec<(String, DVec2)>, Setup> {
    let mut out = Vec::new();
    for chunk in svg.split("<text class=\"label\"").skip(1) {
        let attr = |name: &str| -> Result<f64, Setup> {
            let v = chunk
                .split(&format!(" {name}=\""))
                .nth(1)
                .and_then(|s| s.split('"').next())
                .ok_or("missing attribute")?;
            Ok(v.parse()?)
        };
        let text = chunk
            .split('>')
            .nth(1)
            .and_then(|s| s.split("</text").next())
            .ok_or("no text")?;
        out.push((text.to_owned(), DVec2::new(attr("x")?, attr("y")?)));
    }
    Ok(out)
}

#[test]
fn the_svg_centres_the_block_on_the_label_point_and_places_each_seam_label() -> Result<(), Setup> {
    let pieces = [piece("cotton")?];
    let s = sheet(&pieces)?;
    let text = String::from_utf8(svg::write(&s, &pieces, &[])?)?;
    let labels = svg_labels(&text)?;
    let texts: Vec<&str> = labels.iter().map(|(t, _)| t.as_str()).collect();
    assert_eq!(texts, ["Piece 0", "Cut 1", "cotton", "Seam 3"]);
    // Packed with its cut line at the origin, the piece's centre is at (36, 36) on a 72 mm sheet,
    // and SVG's y runs down. The middle line is centred on it: its baseline is 0.35 of 3.5 mm
    // below, and the others 5 mm above and below that. The seam label at (36, 10) on the sheet
    // drops 0.35 of 2.5 mm.
    let want = [
        DVec2::new(36.0, 32.225),
        DVec2::new(36.0, 37.225),
        DVec2::new(36.0, 42.225),
        DVec2::new(36.0, 62.875),
    ];
    for ((t, at), w) in labels.iter().zip(want) {
        assert!((*at - w).length() <= 1e-3, "{t}: {at:?}, want {w:?}");
    }
    assert!(text.contains("text-anchor=\"middle\""));
    assert_eq!(text.matches("font-size=\"3.500\" text-anchor").count(), 3);
    assert_eq!(text.matches("font-size=\"2.500\" text-anchor").count(), 1);
    Ok(())
}

#[test]
fn each_run_of_whitespace_in_label_text_prints_as_one_space() -> Result<(), Setup> {
    let pieces = [piece("faux\t\n  fur")?];
    let s = sheet(&pieces)?;
    let text = String::from_utf8(svg::write(&s, &pieces, &[])?)?;
    assert!(text.contains(">faux fur</text>"), "{text}");
    Ok(())
}

#[test]
fn the_pdf_places_each_label_line_where_the_svg_does() -> Result<(), Setup> {
    let pieces = [piece("cotton")?];
    let s = sheet(&pieces)?;
    let tiling = tile(&s, Paper::A4)?;
    let pages = tiles(&pdf::write(&s, &pieces, &tiling, &[])?)?;
    let [page] = &pages[..] else {
        return Err(format!("one tile, got {}", pages.len()).into());
    };
    // Sheet millimetres land on the page at the margin, less the tile window's corner; a line
    // starts half its estimated width, 0.55 of its size per character, left of its centre.
    let shift = DVec2::new(MARGIN_MM, MARGIN_MM) - tiling.pages()[0].window_min();
    let want = [
        ("Piece 0", DVec2::new(36.0, 39.775), 3.5),
        ("Cut 1", DVec2::new(36.0, 34.775), 3.5),
        ("cotton", DVec2::new(36.0, 29.775), 3.5),
        ("Seam 3", DVec2::new(36.0, 9.125), 2.5),
    ];
    for (text, centre, size) in want {
        // Lossless: a few characters.
        #[allow(clippy::cast_precision_loss)]
        let half = text.len() as f64 * 0.55 * size / 2.0;
        let start = centre - DVec2::new(half, 0.0) + shift;
        let found = page.texts.iter().find(|t| t.text == text).ok_or(text)?;
        assert!(
            (found.at - start).length() <= 0.01,
            "{text}: {:?}, want {start:?}",
            found.at
        );
        assert!(
            (found.size - size).abs() <= 0.01,
            "{text}: size {}",
            found.size
        );
    }
    Ok(())
}

#[test]
fn a_piece_with_no_label_writes_no_label_text() -> Result<(), Setup> {
    let bare = PatternPiece::new(
        PieceId::new(0),
        square(0.0, 60.0)?,
        square(-6.0, 66.0)?,
        CutCount::new(1, 0)?,
        vec![0.0],
    )?;
    let pieces = [bare];
    let s = sheet(&pieces)?;
    let text = String::from_utf8(svg::write(&s, &pieces, &[])?)?;
    assert!(!text.contains("class=\"label\""));
    Ok(())
}

#[test]
fn the_pdf_prints_every_label_line_on_the_tile() -> Result<(), Setup> {
    let pieces = [piece("cotton")?];
    let s = sheet(&pieces)?;
    let pages = tiles(&pdf::write(&s, &pieces, &tile(&s, Paper::A4)?, &[])?)?;
    let [page] = &pages[..] else {
        return Err(format!("one tile, got {}", pages.len()).into());
    };
    for want in ["Piece 0", "Cut 1", "cotton", "Seam 3"] {
        assert!(
            page.texts.iter().any(|t| t.text == want),
            "{want} missing from {:?}",
            page.texts
        );
    }
    Ok(())
}

#[test]
fn label_text_outside_latin_1_is_refused_by_both_writers() -> Result<(), Setup> {
    let pieces = [piece("faux fur \u{1F43E}")?];
    let s = sheet(&pieces)?;
    let refused = |r: Result<Vec<u8>, ExportError>| {
        matches!(
            r,
            Err(ExportError::Unprintable {
                character: '\u{1F43E}',
                ..
            })
        )
    };
    assert!(refused(svg::write(&s, &pieces, &[])));
    assert!(refused(pdf::write(&s, &pieces, &tile(&s, Paper::A4)?, &[])));
    Ok(())
}
