//! One sheet as a tiled PDF at true scale: a cover page, then a page per tile, each showing its
//! window of the sheet.
//!
//! Geometry stays in sheet millimetres under one `cm` per page, which maps the page's window onto
//! its printable area at 72 / 25.4 points per millimetre. Sheets and PDF are both y-up, so nothing
//! is flipped. No date, producer or file ID is written, so the same input gives the same bytes.

use openplush_geom::DVec2;
use openplush_layout::pages::{INCH_SQUARE_MM, MARGIN_MM, Page, PageTiling, SCALE_SQUARE_MM};
use openplush_layout::{Placement, Sheet};
use openplush_pattern::{Outline, PatternPiece};
use pdf_writer::{Content, Name, Pdf, Rect, Ref, Str};

use crate::label::Placed;
use crate::svg::index;
use crate::warning_summary::lines;
use crate::{ExportError, SummaryRow, label, notch};

mod cover;
mod summary;

const PT_PER_MM: f64 = 72.0 / 25.4;
const CUT_MM: f32 = 0.5;
const SEW_MM: f32 = 0.3;
const SEW_DASH_MM: [f32; 2] = [3.0, 2.0];
const SQUARE_LINE_MM: f32 = 0.3;
/// Each crosshair arm reaches this far from the mark, so the cross fits the 15 mm overlap band.
const MARK_ARM_MM: f64 = 5.0;
const MARK_LINE_MM: f32 = 0.2;
const NOTCH_MM: f32 = 0.4;
const LABEL_MM: f32 = 5.0;
const SQUARE_LABEL_MM: f32 = 3.0;
/// Helvetica is one of the PDF standard fonts, so the viewer supplies it and nothing is embedded.
const FONT: Name<'static> = Name(b"F1");

/// The sheet tiled as `tiling` says, which must have been made from this sheet. First a cover
/// page with the labelled scale-check squares, a map of the tiles and, under it, the `summary` of
/// unresolved warnings; then on every tile's page, every placed copy's solid cut line, dashed
/// sewing line, notch ticks and label, a crosshair at each registration mark the page shows,
/// the labelled scale-check squares and the page's label; then any summary rows the cover had
/// no room for.
///
/// # Errors
/// `UnknownPiece`, `DuplicatePiece`, and `Unprintable` for summary or label text outside
/// Latin-1.
pub fn write(
    sheet: &Sheet,
    pieces: &[PatternPiece],
    tiling: &PageTiling,
    summary: &[SummaryRow],
) -> Result<Vec<u8>, ExportError> {
    let (placed, labels) = placed(sheet, pieces)?;
    let size = tiling.paper().size_mm();
    let rows = lines(summary, summary::columns(size))?;
    let first_after = tiling.pages().len() + 2;
    let split = summary::split(rows, summary.len(), cover::room(size), size, first_after);
    let (catalog, tree, font) = (Ref::new(1), Ref::new(2), Ref::new(3));
    // INVARIANT: `tile` caps a tiling at 26 × 26 pages, and summary pages hold dozens of lines
    // each, so every reference fits in an i32 for any summary that fits in memory.
    let page_ref = |i: usize| Ref::new(4 + 2 * i as i32);
    let count = tiling.pages().len() + 1 + split.pages.len();
    let mono = (!summary.is_empty()).then(|| page_ref(count));
    let mut pdf = Pdf::new();
    let mut cat = pdf.catalog(catalog);
    cat.pages(tree);
    // pdf-writer has no typed /PrintScaling; /None keeps "fit to page" from shrinking the pattern.
    cat.viewer_preferences()
        .pair(Name(b"PrintScaling"), Name(b"None"));
    drop(cat);
    pdf.pages(tree)
        .kids((0..count).map(page_ref))
        .count(count as i32);
    let cover = cover::draw(sheet, &placed, tiling, &split);
    let tiles = tiling
        .pages()
        .iter()
        .map(|p| draw(p, &placed, &labels, tiling.marks()));
    let after = split.pages.iter().map(|lines| summary::page(lines, size));
    for (i, stream) in std::iter::once(cover).chain(tiles).chain(after).enumerate() {
        let (id, content) = (page_ref(i), Ref::new(page_ref(i).get() + 1));
        let mut writer = pdf.page(id);
        writer.media_box(Rect::new(0.0, 0.0, pt(size.x), pt(size.y)));
        writer.parent(tree);
        writer.contents(content);
        let mut resources = writer.resources();
        let mut fonts = resources.fonts();
        fonts.pair(FONT, font);
        if let Some(mono) = mono {
            fonts.pair(summary::MONO, mono);
        }
        drop(fonts);
        drop(resources);
        drop(writer);
        pdf.stream(content, &stream);
    }
    pdf.type1_font(font)
        .base_font(Name(b"Helvetica"))
        .encoding_predefined(Name(b"WinAnsiEncoding"));
    if let Some(mono) = mono {
        pdf.type1_font(mono)
            .base_font(Name(b"Courier"))
            .encoding_predefined(Name(b"WinAnsiEncoding"));
    }
    Ok(pdf.finish())
}

/// Each placement with its piece, and each copy's label lines, in placement order.
type Copies<'a> = (Vec<(&'a Placement, &'a PatternPiece)>, Vec<Vec<Placed>>);

fn placed<'a>(sheet: &'a Sheet, pieces: &'a [PatternPiece]) -> Result<Copies<'a>, ExportError> {
    let by_id = index(pieces)?;
    let placed = sheet
        .placements()
        .iter()
        .map(|pl| {
            let piece = by_id
                .get(&pl.piece())
                .ok_or(ExportError::UnknownPiece { piece: pl.piece() })?;
            Ok((pl, *piece))
        })
        .collect::<Result<Vec<_>, ExportError>>()?;
    let labels = placed
        .iter()
        .map(|(pl, piece)| label::placed(piece, pl))
        .collect::<Result<Vec<_>, _>>()?;
    Ok((placed, labels))
}

/// The page's content: its window of the sheet, clipped to the printable area, then the
/// registration marks in that window, the squares and the labels.
fn draw(
    page: &Page,
    placed: &[(&Placement, &PatternPiece)],
    labels: &[Vec<Placed>],
    marks: &[DVec2],
) -> Vec<u8> {
    let (min, max) = (page.window_min(), page.window_max());
    let k = PT_PER_MM as f32;
    let mut c = Content::new();
    c.save_state();
    c.transform([k, 0.0, 0.0, k, pt(MARGIN_MM - min.x), pt(MARGIN_MM - min.y)]);
    let span = max - min;
    c.rect(min.x as f32, min.y as f32, span.x as f32, span.y as f32);
    c.clip_nonzero();
    c.end_path();
    for (placement, piece) in placed {
        c.set_line_width(CUT_MM);
        c.set_dash_pattern([], 0.0);
        outline(&mut c, piece.cut_line(), placement);
        c.set_line_width(SEW_MM);
        c.set_dash_pattern(SEW_DASH_MM, 0.0);
        outline(&mut c, piece.sewing_line(), placement);
        let ticks = notch::ticks(piece);
        if !ticks.is_empty() {
            c.set_line_width(NOTCH_MM);
            c.set_dash_pattern([], 0.0);
        }
        for [a, b] in ticks {
            let (a, b) = (placement.apply(a), placement.apply(b));
            c.move_to(a.x as f32, a.y as f32);
            c.line_to(b.x as f32, b.y as f32);
            c.stroke();
        }
    }
    for line in labels.iter().flatten() {
        label_text(&mut c, line);
    }
    c.set_dash_pattern([], 0.0);
    c.set_line_width(MARK_LINE_MM);
    let shown = |m: &&DVec2| min.x <= m.x && m.x <= max.x && min.y <= m.y && m.y <= max.y;
    for &mark in marks.iter().filter(shown) {
        crosshair(&mut c, mark);
    }
    squares(&mut c, page.scale_square_min(), &page.label());
    c.restore_state();
    c.finish().to_vec()
}

/// One closed outline, placed on the sheet, stroked.
fn outline(c: &mut Content, outline: &Outline, placement: &Placement) {
    let mut points = outline.points().iter().map(|&p| placement.apply(p));
    if let Some(first) = points.next() {
        c.move_to(first.x as f32, first.y as f32);
        for p in points {
            c.line_to(p.x as f32, p.y as f32);
        }
        c.close_and_stroke();
    }
}

fn crosshair(c: &mut Content, at: DVec2) {
    let (x, y, r) = (at.x as f32, at.y as f32, MARK_ARM_MM as f32);
    c.move_to(x - r, y);
    c.line_to(x + r, y);
    c.stroke();
    c.move_to(x, y - r);
    c.line_to(x, y + r);
    c.stroke();
}

/// The 50 mm square from `corner` with the inch square in its top-left corner, each labelled
/// inside, and `label` 5 mm to the right, its baseline one label size below the square's top.
fn squares(c: &mut Content, corner: DVec2, label: &str) {
    c.set_line_width(SQUARE_LINE_MM);
    square(c, corner, SCALE_SQUARE_MM);
    let inch = corner + DVec2::new(0.0, SCALE_SQUARE_MM - INCH_SQUARE_MM);
    square(c, inch, INCH_SQUARE_MM);
    let side = SCALE_SQUARE_MM;
    text(
        c,
        "50 mm",
        corner + DVec2::new(side / 2.0, 3.0),
        SQUARE_LABEL_MM,
    );
    text(c, "1 in", inch + DVec2::new(3.0, 3.0), SQUARE_LABEL_MM);
    let at = corner + DVec2::new(side + 5.0, side - f64::from(LABEL_MM));
    text(c, label, at, LABEL_MM);
}

/// ASCII only: Helvetica's WinAnsi encoding is ASCII's superset, and every label here is ASCII.
fn text(c: &mut Content, s: &str, at: DVec2, size: f32) {
    c.begin_text();
    c.set_font(FONT, size);
    c.set_text_matrix([1.0, 0.0, 0.0, 1.0, at.x as f32, at.y as f32]);
    c.show(Str(s.as_bytes()));
    c.end_text();
}

fn square(c: &mut Content, min: DVec2, side: f64) {
    c.rect(min.x as f32, min.y as f32, side as f32, side as f32);
    c.stroke();
}

fn pt(mm: f64) -> f32 {
    (mm * PT_PER_MM) as f32
}

/// Helvetica's average advance as a share of its size, to centre a line without its metrics; the
/// centring is off by about a character on unusual text.
const ADVANCE: f64 = 0.55;

/// One label line in Helvetica, centred on its point by an estimate of its width.
fn label_text(c: &mut Content, line: &Placed) {
    // Lossless: a label line is far shorter than 2^52 bytes.
    #[allow(clippy::cast_precision_loss)]
    let half = line.text.len() as f64 * ADVANCE * line.size / 2.0;
    c.begin_text();
    c.set_font(FONT, line.size as f32);
    let at = line.at - DVec2::new(half, 0.0);
    c.set_text_matrix([1.0, 0.0, 0.0, 1.0, at.x as f32, at.y as f32]);
    c.show(Str(&line.text));
    c.end_text();
}
