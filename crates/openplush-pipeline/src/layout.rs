use openplush_layout::shelf::pack_templates;
use openplush_layout::{LayoutOptions, Sheet};
use openplush_pattern::PatternPiece;

use crate::{EngineError, StageResult, Staged, WarningSet};

/// The print: one template of every piece, packed on a roll `opts.roll_width` wide.
///
/// # Errors
/// `Layout`, naming the piece for one too wide for the roll or with a rotation the placer cannot
/// apply, and naming nothing when there are no pieces.
pub fn layout(pieces: &[PatternPiece], opts: &LayoutOptions) -> StageResult<Vec<Sheet>> {
    let value = pack_templates(pieces, opts).map_err(EngineError::Layout)?;
    Ok(Staged {
        value,
        warnings: WarningSet::default(),
    })
}
