//! The same sheet gives the same PDF bytes natively and on wasm32 (Constitution V, ADR 0008).
//!
//! The pinned hash was computed natively; the wasm32 run must reproduce it. It changes only when
//! the PDF itself does, and then a person decides whether the new bytes are right.

use openplush_export::pdf::write;
use openplush_geom::DVec2;
use openplush_geom::units::Mm;
use openplush_layout::pages::{Paper, tile};
use openplush_layout::shelf::pack;
use openplush_layout::{LayoutError, LayoutOptions};
use openplush_pattern::{CutCount, Outline, PatternPiece, PieceId};

type Setup = Box<dyn std::error::Error>;

/// A sewing line with an off-axis edge inside a larger cut line, three copies on a 320 mm roll,
/// tiled on A4: several pages, crosshairs and labels, and coordinates that are not round.
fn pdf() -> Result<Vec<u8>, Setup> {
    let outline = |s: f64| {
        Outline::new(vec![
            DVec2::new(0.0, 0.0),
            DVec2::new(170.3 * s, 0.0),
            DVec2::new(130.7 * s, 211.9 * s),
            DVec2::new(0.0, 160.1 * s),
        ])
    };
    let sewing = Outline::new(
        outline(1.0)?
            .points()
            .iter()
            .map(|p| *p * 0.9 + DVec2::new(7.7, 7.7))
            .collect(),
    )?;
    let piece = PatternPiece::new(
        PieceId::new(0),
        sewing,
        outline(1.0)?,
        CutCount::new(2, 1)?,
        vec![0.0],
    )?;
    let opts = LayoutOptions::new(Mm::try_new(320.0)?, Mm::try_new(5.0)?, false)?;
    let pieces = [piece];
    let sheet = pack(&pieces, &opts)?
        .into_iter()
        .next()
        .ok_or(LayoutError::NoPieces)?;
    Ok(write(&sheet, &pieces, &tile(&sheet, Paper::A4)?, &[])?)
}

/// 64-bit FNV-1a.
fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn the_pdf_bytes_are_the_same_on_every_target() {
    let bytes = pdf().map_err(|e| e.to_string());
    assert!(bytes.is_ok(), "{bytes:?}");
    let bytes = bytes.unwrap_or_default();
    assert_eq!((bytes.len(), fnv(&bytes)), (9214, 0x142a_ec89_3095_8186));
}
