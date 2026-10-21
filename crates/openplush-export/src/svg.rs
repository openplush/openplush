//! One sheet as SVG at true scale: 1 user unit is 1 mm.

use std::collections::BTreeMap;

use openplush_geom::DVec2;
use openplush_layout::{Placement, Sheet};
use openplush_pattern::{Outline, PatternPiece, PieceId};

use crate::warning_summary::{INDENT_COLUMNS, Line, lines};
use crate::{ExportError, SummaryRow};

const STYLE: &str = ".cut{fill:none;stroke:#000;stroke-width:0.5}\
.sew{fill:none;stroke:#000;stroke-width:0.3;stroke-dasharray:3 2}";

/// Summary text size in mm, about 10 pt, and the pitch of its lines.
const TEXT_MM: f64 = 3.5;
const LINE_MM: f64 = 5.0;
/// Space between the sheet and the summary, and round the summary.
const GAP_MM: f64 = 10.0;
/// A monospace glyph's advance as a share of its size (Courier's 600 of 1000 units), so lines
/// wrapped by characters fit the width they were wrapped to.
const ADVANCE: f64 = 0.6;
/// The narrowest the summary wraps to, about 126 mm; a narrower sheet's document widens to it.
const MIN_COLUMNS: usize = 60;

/// The sheet with a solid cut line and a dashed sewing line for every placed copy, in placement
/// order, then the `summary` rows as lines of text under it.
///
/// Sheets are y-up and SVG is y-down, so y is flipped about the sheet's length: the print reads
/// the right way up, never mirrored. Coordinates are written to 0.001 mm. The summary only
/// lengthens the document, and widens it for a sheet too narrow for its lines; no piece moves.
///
/// # Errors
/// `UnknownPiece`, `DuplicatePiece`, and `Unprintable` for summary text outside Latin-1.
pub fn write(
    sheet: &Sheet,
    pieces: &[PatternPiece],
    summary: &[SummaryRow],
) -> Result<Vec<u8>, ExportError> {
    let by_id = index(pieces)?;
    let length = sheet.length_mm();
    let columns = MIN_COLUMNS.max(columns_in(sheet.width_mm() - 2.0 * GAP_MM));
    let notes = lines(summary, columns)?;
    let (width, height) = if notes.is_empty() {
        (sheet.width_mm(), length)
    } else {
        (
            sheet.width_mm().max(columns_wide(columns) + 2.0 * GAP_MM),
            baseline(length, notes.len() - 1) + GAP_MM,
        )
    };
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width:.3}mm\" height=\"{height:.3}mm\" viewBox=\"0 0 {width:.3} {height:.3}\">\n"
    ));
    out.push_str(&format!("<style>{STYLE}</style>\n"));
    for placement in sheet.placements() {
        let piece = by_id
            .get(&placement.piece())
            .ok_or(ExportError::UnknownPiece {
                piece: placement.piece(),
            })?;
        path(&mut out, "cut", piece.cut_line(), placement, length);
        path(&mut out, "sew", piece.sewing_line(), placement, length);
    }
    for (i, line) in notes.iter().enumerate() {
        text(&mut out, line, baseline(length, i));
    }
    out.push_str("</svg>\n");
    Ok(out.into_bytes())
}

pub(crate) fn index(
    pieces: &[PatternPiece],
) -> Result<BTreeMap<PieceId, &PatternPiece>, ExportError> {
    let mut by_id = BTreeMap::new();
    for piece in pieces {
        if by_id.insert(piece.id(), piece).is_some() {
            return Err(ExportError::DuplicatePiece { piece: piece.id() });
        }
    }
    Ok(by_id)
}

/// One closed outline as a path, placed on the sheet and flipped into SVG's y-down frame.
fn path(out: &mut String, class: &str, outline: &Outline, placement: &Placement, length: f64) {
    out.push_str(&format!("<path class=\"{class}\" d=\""));
    for (i, &p) in outline.points().iter().enumerate() {
        let q = placement.apply(p);
        let flipped = DVec2::new(q.x, length - q.y);
        let command = if i == 0 { "M" } else { " L" };
        out.push_str(&format!("{command} {:.3} {:.3}", flipped.x, flipped.y));
    }
    out.push_str(" Z\"/>\n");
}

/// Whole monospace columns in `mm`; none when it is not positive.
fn columns_in(mm: f64) -> usize {
    // Truncation is the point: a partial column holds no character. Lossless for any sheet that
    // fits in memory, and a negative width saturates to zero.
    (mm / (ADVANCE * TEXT_MM)).floor() as usize
}

fn columns_wide(columns: usize) -> f64 {
    // Lossless: a column count is far below 2^52.
    columns as f64 * ADVANCE * TEXT_MM
}

/// Line `i`'s baseline under a sheet `length` mm long, in SVG's y-down frame.
fn baseline(length: f64, i: usize) -> f64 {
    // Lossless: a line count is far below 2^52.
    length + GAP_MM + TEXT_MM + i as f64 * LINE_MM
}

/// One summary line as a text element at baseline `y`, its WinAnsi bytes as the Latin-1
/// characters they are, escaped for XML.
fn text(out: &mut String, line: &Line, y: f64) {
    let indent = if line.indent { INDENT_COLUMNS } else { 0 };
    let x = GAP_MM + columns_wide(indent);
    out.push_str(&format!(
        "<text x=\"{x:.3}\" y=\"{y:.3}\" font-family=\"monospace\" font-size=\"{TEXT_MM:.3}\">"
    ));
    for c in line.text.iter().map(|&b| char::from(b)) {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            c => out.push(c),
        }
    }
    out.push_str("</text>\n");
}
