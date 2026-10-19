//! The cover page: the scale check, and a map of the sheet showing each tile's window with its
//! label over the placed pieces, so the maker can lay the printed tiles out before taping them.

use openplush_geom::DVec2;
use openplush_layout::pages::{MARGIN_MM, Page, PageTiling, SCALE_SQUARE_MM};
use openplush_layout::{Placement, Sheet};
use openplush_pattern::PatternPiece;
use pdf_writer::Content;

use super::summary::{self, Split};
use super::{PT_PER_MM, outline, squares, text};

const COVER_LABEL: &str = "Page map";
/// Between the scale square and the top of the map.
const MAP_GAP_MM: f64 = 5.0;
const WINDOW_LINE_MM: f64 = 0.2;
const SHEET_LINE_MM: f64 = 0.4;
const PIECE_LINE_MM: f64 = 0.2;
/// Light enough to read the tile labels over, dark enough to print on any printer.
const PIECE_GRAY: f32 = 0.6;
const TILE_LABEL_MM: f64 = 4.0;
/// How far a tile's label sits in from its window's top-left corner.
const TILE_LABEL_INSET_MM: f64 = 2.0;

/// How high the cover's room under the scale squares is, on paper `size`.
pub(super) fn room(size: DVec2) -> f64 {
    size.y - 2.0 * MARGIN_MM - SCALE_SQUARE_MM - MAP_GAP_MM
}

/// The cover's content, for the sheet tiled as `tiling` says, with `summary`'s cover lines under
/// the map.
pub(super) fn draw(
    sheet: &Sheet,
    placed: &[(&Placement, &PatternPiece)],
    tiling: &PageTiling,
    summary: &Split,
) -> Vec<u8> {
    let paper = tiling.paper().size_mm();
    let corner = DVec2::new(MARGIN_MM, paper.y - MARGIN_MM - SCALE_SQUARE_MM);
    let k = PT_PER_MM as f32;
    let mut c = Content::new();
    c.save_state();
    c.transform([k, 0.0, 0.0, k, 0.0, 0.0]);
    squares(&mut c, corner, COVER_LABEL);
    if !summary.cover.is_empty() {
        summary::draw_cover(&mut c, summary);
    }
    c.restore_state();
    let room_top = corner.y - MAP_GAP_MM;
    let room = DVec2::new(paper.x - 2.0 * MARGIN_MM, room(paper) - summary.height);
    let fit = Fit::new(tiling.pages(), DVec2::new(MARGIN_MM, room_top), room);
    c.save_state();
    let (s, at) = ((fit.scale * PT_PER_MM) as f32, fit.offset * PT_PER_MM);
    c.transform([s, 0.0, 0.0, s, at.x as f32, at.y as f32]);
    map(&mut c, sheet, placed, tiling.pages(), fit.scale);
    c.restore_state();
    c.finish().to_vec()
}

/// The map scale and the page offset, in millimetres, that fit every tile's window into `room`
/// with its top-left corner at `top_left`: a sheet point `p` lands at `offset + p * scale`.
struct Fit {
    scale: f64,
    offset: DVec2,
}

impl Fit {
    fn new(pages: &[Page], top_left: DVec2, room: DVec2) -> Self {
        // INVARIANT: a tiling has at least one page, and every window has a positive size.
        let (mut min, mut max) = (pages[0].window_min(), pages[0].window_max());
        for p in pages {
            let (lo, hi) = (p.window_min(), p.window_max());
            min = DVec2::new(min.x.min(lo.x), min.y.min(lo.y));
            max = DVec2::new(max.x.max(hi.x), max.y.max(hi.y));
        }
        let span = max - min;
        let scale = (room.x / span.x).min(room.y / span.y);
        Self {
            scale,
            offset: top_left - DVec2::new(min.x, max.y) * scale,
        }
    }
}

/// The faint pieces, the sheet's outline, then every tile's window and label, in sheet
/// millimetres drawn at `scale`; line widths and text sizes are divided by it to print as named.
fn map(
    c: &mut Content,
    sheet: &Sheet,
    placed: &[(&Placement, &PatternPiece)],
    pages: &[Page],
    scale: f64,
) {
    let on_paper = |mm: f64| (mm / scale) as f32;
    c.set_stroke_gray(PIECE_GRAY);
    c.set_line_width(on_paper(PIECE_LINE_MM));
    for (placement, piece) in placed {
        outline(c, piece.cut_line(), placement);
    }
    c.set_stroke_gray(0.0);
    c.set_line_width(on_paper(SHEET_LINE_MM));
    c.rect(0.0, 0.0, sheet.width_mm() as f32, sheet.length_mm() as f32);
    c.stroke();
    c.set_line_width(on_paper(WINDOW_LINE_MM));
    let inset = TILE_LABEL_INSET_MM / scale;
    for page in pages {
        let (lo, hi) = (page.window_min(), page.window_max());
        let span = hi - lo;
        c.rect(lo.x as f32, lo.y as f32, span.x as f32, span.y as f32);
        c.stroke();
        let at = DVec2::new(lo.x + inset, hi.y - inset - TILE_LABEL_MM / scale);
        text(c, &page.label(), at, on_paper(TILE_LABEL_MM));
    }
}
