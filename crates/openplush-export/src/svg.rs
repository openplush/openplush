//! One sheet as SVG at true scale: 1 user unit is 1 mm.

use std::collections::BTreeMap;

use openplush_geom::DVec2;
use openplush_layout::{Placement, Sheet};
use openplush_pattern::{Outline, PatternPiece, PieceId};

use crate::ExportError;

const STYLE: &str = ".cut{fill:none;stroke:#000;stroke-width:0.5}\
.sew{fill:none;stroke:#000;stroke-width:0.3;stroke-dasharray:3 2}";

/// The sheet with a solid cut line and a dashed sewing line for every placed copy, in placement order.
///
/// Sheets are y-up and SVG is y-down, so y is flipped about the sheet's length: the print reads
/// the right way up, never mirrored. Coordinates are written to 0.001 mm.
pub fn write(sheet: &Sheet, pieces: &[PatternPiece]) -> Result<Vec<u8>, ExportError> {
    let by_id = index(pieces)?;
    let (width, length) = (sheet.width_mm(), sheet.length_mm());
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width:.3}mm\" height=\"{length:.3}mm\" viewBox=\"0 0 {width:.3} {length:.3}\">\n"
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
    out.push_str("</svg>\n");
    Ok(out.into_bytes())
}

fn index(pieces: &[PatternPiece]) -> Result<BTreeMap<PieceId, &PatternPiece>, ExportError> {
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
