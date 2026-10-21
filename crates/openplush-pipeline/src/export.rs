use openplush_export::{pdf, svg};
use openplush_layout::Sheet;
use openplush_layout::pages::{Paper, tile};
use openplush_pattern::PatternPiece;

use crate::warning::summary::rows;
use crate::{EngineError, StageResult, Staged, WarningSet};

/// The sheet as SVG at true scale, 1 user unit to 1 mm, with every warning printed under it.
///
/// # Errors
/// `Export`, naming the piece, when the sheet places a piece not in `pieces` or one is given
/// twice.
pub fn export_svg(
    sheet: &Sheet,
    pieces: &[PatternPiece],
    warnings: &WarningSet,
) -> StageResult<Vec<u8>> {
    let value = svg::write(sheet, pieces, &rows(warnings)).map_err(EngineError::Export)?;
    Ok(Staged {
        value,
        warnings: WarningSet::default(),
    })
}

/// The sheet tiled on `paper` as PDF: a cover with the scale check, the page map and every
/// warning, then one page per tile.
///
/// # Errors
/// `Layout` when the sheet needs more than 26 pages across or down; `Export`, naming the piece,
/// when the sheet places a piece not in `pieces` or one is given twice.
pub fn export_pdf(
    sheet: &Sheet,
    pieces: &[PatternPiece],
    paper: Paper,
    warnings: &WarningSet,
) -> StageResult<Vec<u8>> {
    let tiling = tile(sheet, paper).map_err(EngineError::Layout)?;
    let value = pdf::write(sheet, pieces, &tiling, &rows(warnings)).map_err(EngineError::Export)?;
    Ok(Staged {
        value,
        warnings: WarningSet::default(),
    })
}
