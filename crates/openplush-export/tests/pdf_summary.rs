//! The warning summary: rows of text printed under the cover's page map, which
//! shrinks to make room; rows that do not fit continue on pages after the last tile, and the cover
//! says on which page.

#[allow(
    dead_code,
    reason = "each test file reads only part of the shared PDF reader"
)]
mod common;

use common::{Page, Setup, pages, rectangle};
use openplush_export::pdf::write;
use openplush_export::{ExportError, SummaryRow};
use openplush_layout::pages::{MARGIN_MM, Paper, SCALE_SQUARE_MM, tile};

/// Courier's advance, as a share of its size: every glyph is 600 of 1000 units wide.
const COURIER_ADVANCE: f64 = 0.6;

fn row(i: usize, explanation: &str) -> SummaryRow {
    SummaryRow::new(
        format!("seam-parity-{i}"),
        format!("chain A{i}"),
        explanation.to_owned(),
        format!("Move seam {i} by 2 mm"),
    )
}

/// The cover and its tile count, for `rows` on a 300 by 500 mm sheet tiled on A4.
fn printed(rows: &[SummaryRow]) -> Result<(Vec<Page>, usize), Setup> {
    let (sheet, pieces) = rectangle(300.0, 500.0)?;
    let tiling = tile(&sheet, Paper::A4)?;
    Ok((
        pages(&write(&sheet, &pieces, &tiling, rows)?)?,
        tiling.pages().len(),
    ))
}

/// The texts the summary drew on `page`: everything at Courier's line size.
fn summary(page: &Page) -> Vec<&common::Text> {
    let furniture = ["50 mm", "1 in", "Page map"];
    page.texts
        .iter()
        .filter(|t| !furniture.contains(&t.text.as_str()) && !is_tile_label(&t.text))
        .collect()
}

fn is_tile_label(s: &str) -> bool {
    let mut chars = s.chars();
    chars.next().is_some_and(|c| c.is_ascii_uppercase()) && chars.all(|c| c.is_ascii_digit())
}

/// Every summary line on `page` sits inside the printable area, fits its width in Courier, and
/// runs top to bottom without two lines overlapping.
fn laid_out(page: &Page) {
    let lines = summary(page);
    let right = page.size.x - MARGIN_MM;
    for pair in lines.windows(2) {
        assert!(
            pair[1].at.y + pair[1].size <= pair[0].at.y + 1e-6,
            "{pair:?}"
        );
    }
    for t in lines {
        let width = t.text.chars().count() as f64 * COURIER_ADVANCE * t.size;
        assert!(t.at.x >= MARGIN_MM - 1e-6, "{t:?}");
        assert!(t.at.x + width <= right + 1e-6, "{t:?} is {width} mm wide");
        assert!(t.at.y >= MARGIN_MM - 1e-6, "{t:?}");
    }
}

/// The lowest point of the cover's black page-map rectangles.
fn map_bottom(cover: &Page) -> f64 {
    cover
        .strokes
        .iter()
        .filter(|s| s.points.len() == 4 && s.gray == 0.0)
        .filter(|s| {
            s.points
                .iter()
                .all(|p| p.y < cover.size.y - MARGIN_MM - SCALE_SQUARE_MM)
        })
        .flat_map(|s| s.points.iter().map(|p| p.y))
        .fold(f64::MAX, f64::min)
}

fn joined(lines: &[&common::Text]) -> String {
    lines
        .iter()
        .map(|t| t.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn with_no_warnings_the_cover_prints_no_summary_and_no_page_is_added() -> Result<(), Setup> {
    let (read, tiles) = printed(&[])?;
    assert_eq!(read.len(), 1 + tiles);
    assert!(summary(&read[0]).is_empty(), "{:?}", summary(&read[0]));
    // A tall sheet's map is fitted by its height, so with no summary it reaches the margin.
    let bottom = map_bottom(&read[0]);
    assert!((bottom - MARGIN_MM).abs() <= 1e-3, "{bottom}");
    Ok(())
}

#[test]
fn a_few_warnings_print_under_a_shrunk_map_on_the_cover() -> Result<(), Setup> {
    let rows = [
        row(1, "The two sides differ by 3 mm."),
        row(2, "Too tight to sew."),
    ];
    let (read, tiles) = printed(&rows)?;
    assert_eq!(read.len(), 1 + tiles, "no page added");
    let cover = &read[0];
    let lines = summary(cover);
    laid_out(cover);
    assert_eq!(lines[0].text, "Unresolved warnings: 2");
    let text = joined(&lines);
    for (i, want) in [
        (1, "The two sides differ by 3 mm."),
        (2, "Too tight to sew."),
    ] {
        assert!(
            text.contains(&format!("{i}. seam-parity-{i}: chain A{i}")),
            "{text}"
        );
        assert!(text.contains(want), "{text}");
        assert!(
            text.contains(&format!("Fix: Move seam {i} by 2 mm")),
            "{text}"
        );
    }
    let top = lines[0].at.y + lines[0].size;
    assert!(top < map_bottom(cover), "the summary sits under the map");
    Ok(())
}

#[test]
fn a_long_explanation_wraps_at_word_breaks_within_the_printable_width() -> Result<(), Setup> {
    let words: Vec<String> = (0..120).map(|i| format!("word{i}")).collect();
    let long = words.join(" ");
    let (read, _) = printed(&[row(1, &long)])?;
    let cover = &read[0];
    laid_out(cover);
    let lines = summary(cover);
    assert!(lines.len() > 4, "wrapped: {lines:?}");
    let text = joined(&lines);
    assert!(
        text.contains(&long),
        "every word, in order, split only at spaces"
    );
    Ok(())
}

#[test]
fn rows_that_do_not_fit_continue_after_the_last_tile() -> Result<(), Setup> {
    let rows: Vec<SummaryRow> = (1..=150).map(|i| row(i, "Check this seam.")).collect();
    let (read, tiles) = printed(&rows)?;
    let first_continued = 2 + tiles;
    assert!(read.len() > 1 + tiles, "pages were added: {}", read.len());
    for tile in &read[1..=tiles] {
        assert!(summary(tile).is_empty(), "tiles carry no summary");
    }
    let cover = &read[0];
    laid_out(cover);
    let cover_lines = summary(cover);
    let note = cover_lines.last().ok_or("an empty cover summary")?;
    assert_eq!(note.text, format!("Continued on page {first_continued}."));
    let mut text = joined(&cover_lines[..cover_lines.len() - 1]);
    for page in &read[1 + tiles..] {
        laid_out(page);
        let lines = summary(page);
        assert_eq!(lines[0].text, "Unresolved warnings, continued");
        text.push(' ');
        text.push_str(&joined(&lines[1..]));
    }
    let mut from = 0;
    for i in 1..=150 {
        let at = text[from..]
            .find(&format!("{i}. seam-parity-{i}: chain A{i}"))
            .ok_or(format!("row {i} missing or out of order"))?;
        from += at;
    }
    // The map still gets at least half the room below the squares.
    let room_top = cover.size.y - MARGIN_MM - SCALE_SQUARE_MM;
    let map_height = room_top - map_bottom(cover);
    assert!(
        map_height >= (room_top - MARGIN_MM) / 2.0 - 5.0 - 1e-3,
        "{map_height}"
    );
    Ok(())
}

#[test]
fn latin_1_text_is_written_as_one_winansi_byte_a_character() -> Result<(), Setup> {
    let (sheet, pieces) = rectangle(100.0, 100.0)?;
    let tiling = tile(&sheet, Paper::A4)?;
    let bytes = write(&sheet, &pieces, &tiling, &[row(1, "Caf\u{e9} seam.")])?;
    // pdf-writer writes a string with any byte above ASCII in hex.
    let hex = String::from_utf8_lossy(&bytes).to_ascii_uppercase();
    assert!(hex.contains("436166E9"), "é as WinAnsi E9");
    assert!(!hex.contains("436166C3A9"), "not as UTF-8");
    Ok(())
}

#[test]
fn a_character_helvetica_and_courier_cannot_print_is_refused() -> Result<(), Setup> {
    let (sheet, pieces) = rectangle(100.0, 100.0)?;
    let tiling = tile(&sheet, Paper::A4)?;
    let bad = row(1, "Off by \u{221a}2 mm.");
    assert_eq!(
        write(&sheet, &pieces, &tiling, &[bad]),
        Err(ExportError::Unprintable {
            text: "Off by \u{221a}2 mm.".to_owned(),
            character: '\u{221a}',
        })
    );
    Ok(())
}

#[test]
fn the_same_warnings_give_the_same_bytes() -> Result<(), Setup> {
    let (sheet, pieces) = rectangle(300.0, 500.0)?;
    let tiling = tile(&sheet, Paper::A4)?;
    let rows: Vec<SummaryRow> = (1..=80).map(|i| row(i, "Check this seam.")).collect();
    assert_eq!(
        write(&sheet, &pieces, &tiling, &rows)?,
        write(&sheet, &pieces, &tiling, &rows)?
    );
    Ok(())
}

#[test]
fn a_summary_line_starts_where_its_row_does_and_wraps_indented() -> Result<(), Setup> {
    let (read, _) = printed(&[row(1, &["long"; 60].join(" "))])?;
    let lines = summary(&read[0]);
    let first = lines
        .iter()
        .find(|t| t.text.starts_with("1. "))
        .ok_or("row 1")?;
    let after: Vec<_> = lines
        .iter()
        .skip_while(|t| !t.text.starts_with("1. "))
        .skip(1)
        .collect();
    assert!(!after.is_empty());
    for t in after {
        assert!(
            t.at.x > first.at.x + 1e-3,
            "{t:?} is indented under {first:?}"
        );
    }
    Ok(())
}

#[test]
fn courier_is_declared_and_on_every_page_only_when_there_is_a_summary() -> Result<(), Setup> {
    let (sheet, pieces) = rectangle(100.0, 100.0)?;
    let tiling = tile(&sheet, Paper::A4)?;
    let pages = 1 + tiling.pages().len();
    // The fonts declared, and the pages whose resources name the summary's font.
    let courier = |rows: &[SummaryRow]| -> Result<(usize, usize), Setup> {
        let text = String::from_utf8_lossy(&write(&sheet, &pieces, &tiling, rows)?).into_owned();
        let named = text
            .split("/F2 ")
            .skip(1)
            .filter(|rest| {
                let digits = rest.chars().take_while(char::is_ascii_digit).count();
                digits > 0 && rest[digits..].starts_with(" 0 R")
            })
            .count();
        Ok((text.matches("/BaseFont /Courier").count(), named))
    };
    assert_eq!(courier(&[])?, (0, 0));
    assert_eq!(courier(&[row(1, "Check this seam.")])?, (1, pages));
    Ok(())
}
