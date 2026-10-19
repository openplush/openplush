//! One sheet as a tiled PDF at true scale: a page per tile, each showing its window of the sheet.
//!
//! Geometry stays in sheet millimetres under one `cm` per page, which maps the page's window onto
//! its printable area at 72 / 25.4 points per millimetre. Sheets and PDF are both y-up, so nothing
//! is flipped. No date, producer or file ID is written, so the same input gives the same bytes.

use openplush_geom::DVec2;
use openplush_layout::pages::{INCH_SQUARE_MM, MARGIN_MM, Page, PageTiling, SCALE_SQUARE_MM};
use openplush_layout::{Placement, Sheet};
use openplush_pattern::{Outline, PatternPiece};
use pdf_writer::{Content, Pdf, Rect, Ref};

use crate::ExportError;
use crate::svg::index;

const PT_PER_MM: f64 = 72.0 / 25.4;
const CUT_MM: f32 = 0.5;
const SEW_MM: f32 = 0.3;
const SEW_DASH_MM: [f32; 2] = [3.0, 2.0];
const SQUARE_LINE_MM: f32 = 0.3;

/// The sheet tiled as `tiling` says, which must have been made from this sheet: every placed copy's
/// solid cut line then dashed sewing line, and the scale-check squares, on every page.
pub fn write(
    sheet: &Sheet,
    pieces: &[PatternPiece],
    tiling: &PageTiling,
) -> Result<Vec<u8>, ExportError> {
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
    let size = tiling.paper().size_mm();
    let (catalog, tree) = (Ref::new(1), Ref::new(2));
    // INVARIANT: `tile` caps a tiling at 26 × 26 pages, so every reference fits in an i32.
    let page_ref = |i: usize| Ref::new(3 + 2 * i as i32);
    let count = tiling.pages().len();
    let mut pdf = Pdf::new();
    pdf.catalog(catalog).pages(tree);
    pdf.pages(tree)
        .kids((0..count).map(page_ref))
        .count(count as i32);
    for (i, page) in tiling.pages().iter().enumerate() {
        let (id, content) = (page_ref(i), Ref::new(page_ref(i).get() + 1));
        let mut writer = pdf.page(id);
        writer.media_box(Rect::new(0.0, 0.0, pt(size.x), pt(size.y)));
        writer.parent(tree);
        writer.contents(content);
        drop(writer);
        pdf.stream(content, &draw(page, &placed));
    }
    Ok(pdf.finish())
}

/// The page's content: its window of the sheet, clipped to the printable area, then its squares.
fn draw(page: &Page, placed: &[(&Placement, &PatternPiece)]) -> Vec<u8> {
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
    }
    c.set_line_width(SQUARE_LINE_MM);
    c.set_dash_pattern([], 0.0);
    let corner = page.scale_square_min();
    square(&mut c, corner, SCALE_SQUARE_MM);
    // The inch square shares the scale square's top-left corner.
    let inch = corner + DVec2::new(0.0, SCALE_SQUARE_MM - INCH_SQUARE_MM);
    square(&mut c, inch, INCH_SQUARE_MM);
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

fn square(c: &mut Content, min: DVec2, side: f64) {
    c.rect(min.x as f32, min.y as f32, side as f32, side as f32);
    c.stroke();
}

fn pt(mm: f64) -> f32 {
    (mm * PT_PER_MM) as f32
}
