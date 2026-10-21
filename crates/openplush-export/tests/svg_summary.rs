//! The warning summary in the SVG: the same rows as the PDF cover, as lines of text under the
//! sheet, so a printed SVG carries its warnings too. The pieces never move for it.

use openplush_export::svg::write;
use openplush_export::{ExportError, SummaryRow};
use openplush_geom::DVec2;
use openplush_geom::units::Mm;
use openplush_layout::shelf::pack;
use openplush_layout::{LayoutOptions, Sheet};
use openplush_pattern::{CutCount, Outline, PatternPiece, PieceId};

type Setup = Box<dyn std::error::Error>;

/// One `w` by `h` mm rectangle piece, packed on a roll `roll` mm wide with a 5 mm gap.
fn rectangle(w: f64, h: f64, roll: f64) -> Result<(Sheet, Vec<PatternPiece>), Setup> {
    let outline = Outline::new(vec![
        DVec2::new(0.0, 0.0),
        DVec2::new(w, 0.0),
        DVec2::new(w, h),
        DVec2::new(0.0, h),
    ])?;
    let piece = PatternPiece::new(
        PieceId::new(0),
        outline.clone(),
        outline,
        CutCount::new(1, 0)?,
        vec![0.0],
    )?;
    let opts = LayoutOptions::new(Mm::try_new(roll)?, Mm::try_new(5.0)?, false)?;
    let sheet = pack(std::slice::from_ref(&piece), &opts)?
        .into_iter()
        .next()
        .ok_or("one sheet")?;
    Ok((sheet, vec![piece]))
}

fn import_ignored() -> SummaryRow {
    SummaryRow::new(
        "import-ignored".into(),
        "the whole pattern".into(),
        "The file has an animation, which was not read.".into(),
        "Nothing to change; the pattern uses the model at rest.".into(),
    )
}

fn svg(sheet: &Sheet, pieces: &[PatternPiece], rows: &[SummaryRow]) -> Result<String, Setup> {
    Ok(String::from_utf8(write(sheet, pieces, rows)?)?)
}

/// The `name="…"` attribute of the root element, in user units (mm).
fn size(svg: &str, name: &str) -> Result<f64, Setup> {
    let after = svg
        .split(&format!(" {name}=\""))
        .nth(1)
        .ok_or("no such attribute")?;
    Ok(after.split("mm\"").next().ok_or("no unit")?.parse()?)
}

/// Every text element's content, in document order, with XML escapes left as written.
fn texts(svg: &str) -> Vec<&str> {
    svg.split("<text")
        .skip(1)
        .filter_map(|t| {
            t.split_once('>')
                .and_then(|(_, rest)| rest.split_once("</text>"))
        })
        .map(|(content, _)| content)
        .collect()
}

/// Every path element, as written.
fn paths(svg: &str) -> Vec<&str> {
    svg.split("<path")
        .skip(1)
        .map(|p| p.split("/>").next().unwrap_or(p))
        .collect()
}

#[test]
fn no_rows_add_nothing_to_the_sheet() -> Result<(), Setup> {
    let (sheet, pieces) = rectangle(100.0, 50.0, 200.0)?;
    let plain = svg(&sheet, &pieces, &[])?;
    assert!(texts(&plain).is_empty());
    assert_eq!(size(&plain, "height")?, sheet.length_mm());
    assert_eq!(size(&plain, "width")?, sheet.width_mm());
    Ok(())
}

#[test]
fn a_row_prints_under_the_sheet_without_moving_any_piece() -> Result<(), Setup> {
    let (sheet, pieces) = rectangle(100.0, 50.0, 200.0)?;
    let plain = svg(&sheet, &pieces, &[])?;
    let warned = svg(&sheet, &pieces, &[import_ignored()])?;
    let lines = texts(&warned);
    assert_eq!(lines.first(), Some(&"1. import-ignored: the whole pattern"));
    assert!(lines.contains(&"Fix: Nothing to change; the pattern uses the model at rest."));
    assert!(size(&warned, "height")? > sheet.length_mm());
    // The y axis runs down from the top, so a taller document keeps every piece where it was.
    assert_eq!(paths(&warned), paths(&plain));
    Ok(())
}

#[test]
fn every_line_sits_below_the_sheet_and_inside_the_document() -> Result<(), Setup> {
    let (sheet, pieces) = rectangle(100.0, 50.0, 200.0)?;
    let warned = svg(&sheet, &pieces, &[import_ignored(), import_ignored()])?;
    let height = size(&warned, "height")?;
    let ys: Vec<f64> = warned
        .split("<text")
        .skip(1)
        .map(|t| {
            let y = t.split(" y=\"").nth(1).and_then(|s| s.split('"').next());
            y.ok_or("no y").map(str::parse::<f64>)
        })
        .collect::<Result<Result<_, _>, _>>()??;
    assert_eq!(ys.len(), texts(&warned).len());
    assert!(
        ys.iter().all(|&y| y > sheet.length_mm() && y < height),
        "{ys:?}"
    );
    assert!(ys.windows(2).all(|w| w[0] < w[1]), "{ys:?}");
    Ok(())
}

#[test]
fn text_is_escaped_for_xml() -> Result<(), Setup> {
    let (sheet, pieces) = rectangle(100.0, 50.0, 200.0)?;
    let row = SummaryRow::new(
        "code".into(),
        "piece <1> & piece \"2\"".into(),
        "x".into(),
        "y".into(),
    );
    let warned = svg(&sheet, &pieces, &[row])?;
    assert_eq!(
        texts(&warned).first(),
        Some(&"1. code: piece &lt;1&gt; &amp; piece \"2\"")
    );
    Ok(())
}

#[test]
fn a_narrow_sheet_widens_the_document_to_the_text_and_keeps_its_pieces() -> Result<(), Setup> {
    let (sheet, pieces) = rectangle(20.0, 50.0, 30.0)?;
    let plain = svg(&sheet, &pieces, &[])?;
    let warned = svg(&sheet, &pieces, &[import_ignored()])?;
    assert!(size(&warned, "width")? > sheet.width_mm());
    assert_eq!(paths(&warned), paths(&plain));
    Ok(())
}

#[test]
fn text_the_summary_cannot_carry_is_refused() -> Result<(), Setup> {
    let (sheet, pieces) = rectangle(100.0, 50.0, 200.0)?;
    let row = SummaryRow::new("code".into(), "片".into(), "x".into(), "y".into());
    assert!(matches!(
        write(&sheet, &pieces, &[row]),
        Err(ExportError::Unprintable {
            character: '片',
            ..
        })
    ));
    Ok(())
}

#[test]
fn the_same_rows_write_the_same_bytes() -> Result<(), Setup> {
    let (sheet, pieces) = rectangle(100.0, 50.0, 200.0)?;
    let rows = [import_ignored()];
    assert_eq!(
        write(&sheet, &pieces, &rows)?,
        write(&sheet, &pieces, &rows)?
    );
    Ok(())
}
