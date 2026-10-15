//! Option A: the page as an SVG document, converted by `svg2pdf`.

use std::fmt::Write as _;

use svg2pdf::{ConversionOptions, PageOptions, usvg};

use crate::{LABEL_MM, LABELS, PAGE_MM, SQUARE_AT_MM, SQUARE_MM, STROKE_MM};

/// usvg resolves `mm` lengths at the CSS 96 px per inch; svg2pdf's default
/// `PageOptions` assumes 72, which would print everything 4/3 too large.
pub const CSS_DPI: f32 = 96.0;

/// The page as SVG: 1 user unit = 1 mm, the same convention as the SVG export.
pub fn page_svg() -> String {
    let [w, h] = PAGE_MM;
    let mut s = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}mm\" height=\"{h}mm\" \
         viewBox=\"0 0 {w} {h}\">\n<path fill=\"none\" stroke=\"#000\" \
         stroke-width=\"{STROKE_MM}\" d=\""
    );
    for (i, [x, y]) in crate::piece_outline().iter().enumerate() {
        let cmd = if i == 0 { 'M' } else { 'L' };
        // INVARIANT: fmt::Write into a String cannot fail.
        write!(s, "{cmd}{x:.4} {y:.4} ").expect("String write");
    }
    let [sx, sy] = SQUARE_AT_MM;
    write!(
        s,
        "Z\"/>\n<rect x=\"{sx}\" y=\"{sy}\" width=\"{SQUARE_MM}\" height=\"{SQUARE_MM}\" \
         fill=\"none\" stroke=\"#000\" stroke-width=\"{STROKE_MM}\"/>\n"
    )
    .expect("String write");
    for label in &LABELS {
        let [x, y] = label.at_mm;
        writeln!(
            s,
            "<text x=\"{x}\" y=\"{y}\" font-family=\"Helvetica\" font-size=\"{LABEL_MM}\">{}</text>",
            label.text
        )
        .expect("String write");
    }
    s.push_str("</svg>\n");
    s
}

/// One-page PDF from [`page_svg`] at the correct CSS resolution.
pub fn page_svg2pdf() -> Result<Vec<u8>, String> {
    page_svg2pdf_at_dpi(CSS_DPI)
}

/// As [`page_svg2pdf`] with an explicit `PageOptions::dpi`, to measure the default.
pub fn page_svg2pdf_at_dpi(dpi: f32) -> Result<Vec<u8>, String> {
    let tree =
        usvg::Tree::from_str(&page_svg(), &usvg::Options::default()).map_err(|e| e.to_string())?;
    svg2pdf::to_pdf(&tree, ConversionOptions::default(), PageOptions { dpi })
        .map_err(|e| e.to_string())
}
