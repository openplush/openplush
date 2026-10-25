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
use openplush_layout::pages::{Paper, tile};
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
    // and SVG's y runs down: the block is centred on x = 36, its lines 5 mm apart downwards.
    for (_, at) in &labels[..3] {
        assert!((at.x - 36.0).abs() <= 1e-3, "{at:?}");
    }
    assert!((labels[1].1.y - labels[0].1.y - 5.0).abs() <= 1e-3);
    assert!((labels[2].1.y - labels[1].1.y - 5.0).abs() <= 1e-3);
    let middle = (labels[0].1.y + labels[2].1.y) / 2.0;
    assert!((middle - 36.0).abs() <= 3.5, "the block's middle {middle}");
    assert!(text.contains("text-anchor=\"middle\""));
    let seam = labels[3].1;
    assert!(
        (seam.x - 36.0).abs() <= 1e-3 && (seam.y - (72.0 - 10.0)).abs() <= 2.5,
        "{seam:?}"
    );
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
