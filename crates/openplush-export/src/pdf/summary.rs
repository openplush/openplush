//! The warning summary's pages: as many lines as fit under the cover's page map, which keeps at
//! least half its room, then the rest on pages after the last tile, so a tile's page number never
//! depends on the warnings.

use openplush_geom::DVec2;
use openplush_layout::pages::MARGIN_MM;
use pdf_writer::{Content, Name, Str};

use super::PT_PER_MM;
use crate::warning_summary::{INDENT_COLUMNS, Line};

/// Courier, a standard font with one advance for every glyph, so wrapping by characters fits.
pub(super) const MONO: Name<'static> = Name(b"F2");
const TEXT_MM: f64 = 3.0;
const PITCH_MM: f64 = 4.0;
/// Courier's advance: 600 of 1000 units of its size.
const ADVANCE: f64 = 0.6;
/// Between the page map and the summary under it.
const GAP_MM: f64 = 5.0;
/// The share of the cover's room under the scale squares that the page map always keeps.
const MAP_SHARE: f64 = 0.5;

/// How many characters fit across the printable width of paper `size`.
pub(super) fn columns(size: DVec2) -> usize {
    // INVARIANT: Letter and A4 are both far wider than the margins and an indent.
    ((size.x - 2.0 * MARGIN_MM) / (ADVANCE * TEXT_MM)).floor() as usize
}

/// The summary split across the cover and the pages after the last tile.
pub(super) struct Split {
    /// The cover's lines, ending with where the rest continues when there is a rest.
    pub(super) cover: Vec<Line>,
    /// How much of the cover's room the summary takes from the map, gap included.
    pub(super) height: f64,
    /// Each continuation page's lines, heading first.
    pub(super) pages: Vec<Vec<Line>>,
}

/// The lines of `count` rows under a heading, split for a cover whose room under the squares is
/// `room` high, on paper `size`; the first continuation page is printed page `first_after`.
pub(super) fn split(
    rows: Vec<Line>,
    count: usize,
    room: f64,
    size: DVec2,
    first_after: usize,
) -> Split {
    if rows.is_empty() {
        return Split {
            cover: Vec::new(),
            height: 0.0,
            pages: Vec::new(),
        };
    }
    let mut lines = vec![Line::plain(&format!("Unresolved warnings: {count}"))];
    lines.extend(rows);
    let fits = |height: f64| (height / PITCH_MM).floor() as usize;
    let most = room * (1.0 - MAP_SHARE) - GAP_MM;
    if lines.len() <= fits(most) {
        let height = lines.len() as f64 * PITCH_MM + GAP_MM;
        return Split {
            cover: lines,
            height,
            pages: Vec::new(),
        };
    }
    // INVARIANT: both papers leave room for many lines, so neither count reaches zero.
    let on_cover = fits(most) - 1;
    let per_page = fits(size.y - 2.0 * MARGIN_MM) - 1;
    let rest = lines.split_off(on_cover);
    lines.push(Line::plain(&format!("Continued on page {first_after}.")));
    let pages = rest
        .chunks(per_page)
        .map(|chunk| {
            let mut page = vec![Line::plain("Unresolved warnings, continued")];
            page.extend_from_slice(chunk);
            page
        })
        .collect();
    Split {
        height: lines.len() as f64 * PITCH_MM + GAP_MM,
        cover: lines,
        pages,
    }
}

/// The cover's lines along the bottom of its printable area, in page millimetres.
pub(super) fn draw_cover(c: &mut Content, split: &Split) {
    draw(c, &split.cover, MARGIN_MM + split.height - GAP_MM);
}

/// Draws `lines` in page millimetres, the first one's top at `top`.
fn draw(c: &mut Content, lines: &[Line], top: f64) {
    let indent = INDENT_COLUMNS as f64 * ADVANCE * TEXT_MM;
    for (i, line) in lines.iter().enumerate() {
        let x = MARGIN_MM + if line.indent { indent } else { 0.0 };
        let y = top - TEXT_MM - i as f64 * PITCH_MM;
        c.begin_text();
        c.set_font(MONO, TEXT_MM as f32);
        c.set_text_matrix([1.0, 0.0, 0.0, 1.0, x as f32, y as f32]);
        c.show(Str(&line.text));
        c.end_text();
    }
}

/// A continuation page on paper `size`: its lines from the top of the printable area.
pub(super) fn page(lines: &[Line], size: DVec2) -> Vec<u8> {
    let k = PT_PER_MM as f32;
    let mut c = Content::new();
    c.save_state();
    c.transform([k, 0.0, 0.0, k, 0.0, 0.0]);
    draw(&mut c, lines, size.y - MARGIN_MM);
    c.restore_state();
    c.finish().to_vec()
}
