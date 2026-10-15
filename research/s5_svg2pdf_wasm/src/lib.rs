//! Spike S5: write the same pattern page to PDF two ways, (A) SVG through
//! `svg2pdf` and (B) directly with `pdf-writer`, so native and wasm32 output can
//! be compared byte for byte and the scale-check square measured.
//!
//! Page space is millimetres, origin top-left, y down (the SVG convention).

mod direct;
mod via_svg;

pub use direct::page_direct;
pub use via_svg::{page_svg, page_svg2pdf, page_svg2pdf_at_dpi};

/// PDF points per millimetre: 72 pt per inch, 25.4 mm per inch.
pub const PT_PER_MM: f64 = 72.0 / 25.4;

/// Page size in mm: A3 landscape, two A4 sheets' worth of pattern piece.
pub const PAGE_MM: [f64; 2] = [420.0, 297.0];

/// Scale-check square: top-left corner and side, in mm.
pub const SQUARE_AT_MM: [f64; 2] = [10.0, 237.0];
pub const SQUARE_MM: f64 = 50.0;

/// Outline vertex count: a few hundred, like a real flattened piece boundary.
pub const OUTLINE_VERTICES: usize = 400;

pub const STROKE_MM: f64 = 0.3;
pub const LABEL_MM: f64 = 6.0;

/// A text label: baseline-left position in mm and its text (ASCII only).
pub struct Label {
    pub at_mm: [f64; 2],
    pub text: &'static str,
}

pub const LABELS: [Label; 3] = [
    Label {
        at_mm: [222.0, 150.0],
        text: "A3",
    },
    Label {
        at_mm: [200.0, 160.0],
        text: "Body side",
    },
    Label {
        at_mm: [64.0, 287.0],
        text: "50 mm",
    },
];

/// Closed lobed outline in mm, counter-clockwise in y-down page space.
pub fn piece_outline() -> Vec<[f64; 2]> {
    (0..OUTLINE_VERTICES)
        .map(|i| {
            let t = std::f64::consts::TAU * i as f64 / OUTLINE_VERTICES as f64;
            let r = 1.0 + 0.15 * (3.0 * t).cos() + 0.05 * (7.0 * t).sin();
            [230.0 + 150.0 * r * t.cos(), 150.0 + 110.0 * r * t.sin()]
        })
        .collect()
}

/// FNV-1a over the PDF bytes; equal bytes give equal hashes.
pub fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}
