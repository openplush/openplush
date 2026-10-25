//! Piece labels placed on the sheet, ready for either writer.

use openplush_geom::DVec2;
use openplush_layout::Placement;
use openplush_pattern::PatternPiece;

use crate::ExportError;
use crate::warning_summary::latin1;

/// The label block's text size and line pitch, and a seam label's text size, in mm.
const BLOCK_MM: f64 = 3.5;
const PITCH_MM: f64 = 5.0;
const SEAM_MM: f64 = 2.5;
/// A baseline sits this share of the text size below the point the line is centred on, about
/// half the cap height, so the letters are centred on it.
const DROP: f64 = 0.35;

/// One line of label text: its WinAnsi bytes, the middle of its baseline on the sheet (y-up), and
/// its size in mm.
pub(crate) struct Placed {
    pub(crate) text: Vec<u8>,
    pub(crate) at: DVec2,
    pub(crate) size: f64,
}

/// Every line of `piece`'s label as `placement` puts it: the block's lines one `PITCH_MM` under
/// another, centred on the label point, then each seam label centred on its point. Text stays
/// upright whatever the placement's turn or mirror; only its point moves. None without a label.
///
/// # Errors
/// `Unprintable` for label text outside Latin-1.
pub(crate) fn placed(
    piece: &PatternPiece,
    placement: &Placement,
) -> Result<Vec<Placed>, ExportError> {
    let Some(label) = piece.label() else {
        return Ok(Vec::new());
    };
    let centre = placement.apply(label.at());
    // Lossless: a label has a handful of lines.
    #[allow(clippy::cast_precision_loss)]
    let half = (label.lines().len().saturating_sub(1)) as f64 / 2.0;
    let mut out = Vec::new();
    for (i, line) in label.lines().iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let dy = (half - i as f64) * PITCH_MM - DROP * BLOCK_MM;
        out.push(Placed {
            text: latin1(line)?,
            at: centre + DVec2::new(0.0, dy),
            size: BLOCK_MM,
        });
    }
    for seam in label.seams() {
        out.push(Placed {
            text: latin1(seam.text())?,
            at: placement.apply(seam.at()) - DVec2::new(0.0, DROP * SEAM_MM),
            size: SEAM_MM,
        });
    }
    Ok(out)
}
