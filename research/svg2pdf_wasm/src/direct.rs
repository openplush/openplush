//! Option B: the page written directly with `pdf-writer`.

use pdf_writer::{Content, Finish, Name, Pdf, Rect, Ref, Str};

use crate::{LABEL_MM, LABELS, PAGE_MM, PT_PER_MM, SQUARE_AT_MM, SQUARE_MM, STROKE_MM};

/// One-page PDF. Geometry stays in mm under a single `cm`; text uses the
/// standard-14 Helvetica, which viewers supply, so no font is embedded.
pub fn page_direct() -> Vec<u8> {
    let (catalog, pages, page, content, font) = (
        Ref::new(1),
        Ref::new(2),
        Ref::new(3),
        Ref::new(4),
        Ref::new(5),
    );
    let k = PT_PER_MM as f32;
    let [w, h] = PAGE_MM.map(|v| (v * PT_PER_MM) as f32);

    let mut pdf = Pdf::new();
    pdf.catalog(catalog).pages(pages);
    pdf.pages(pages).kids([page]).count(1);
    let mut p = pdf.page(page);
    p.media_box(Rect::new(0.0, 0.0, w, h));
    p.parent(pages);
    p.contents(content);
    p.resources().fonts().pair(Name(b"F1"), font);
    p.finish();
    pdf.type1_font(font)
        .base_font(Name(b"Helvetica"))
        .encoding_predefined(Name(b"WinAnsiEncoding"));

    let mut c = Content::new();
    c.save_state();
    // Page mm, y down → PDF points, y up.
    c.transform([k, 0.0, 0.0, -k, 0.0, h]);
    c.set_line_width(STROKE_MM as f32);
    let outline = crate::piece_outline();
    if let Some((first, rest)) = outline.split_first() {
        c.move_to(first[0] as f32, first[1] as f32);
        for v in rest {
            c.line_to(v[0] as f32, v[1] as f32);
        }
        c.close_and_stroke();
    }
    let [sx, sy] = SQUARE_AT_MM.map(|v| v as f32);
    c.rect(sx, sy, SQUARE_MM as f32, SQUARE_MM as f32);
    c.stroke();
    c.restore_state();

    for label in &LABELS {
        let [x, y] = label.at_mm;
        let (tx, ty) = ((x * PT_PER_MM) as f32, h - (y * PT_PER_MM) as f32);
        c.begin_text();
        c.set_font(Name(b"F1"), (LABEL_MM * PT_PER_MM) as f32);
        c.set_text_matrix([1.0, 0.0, 0.0, 1.0, tx, ty]);
        c.show(Str(label.text.as_bytes()));
        c.end_text();
    }
    pdf.stream(content, &c.finish());
    pdf.finish()
}
