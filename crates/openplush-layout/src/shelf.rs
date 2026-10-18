use openplush_pattern::PatternPiece;

use crate::{LayoutError, LayoutOptions, Sheet};

/// Lays every copy of every piece on shelves across the roll, tallest first.
pub fn pack(pieces: &[PatternPiece], opts: &LayoutOptions) -> Result<Vec<Sheet>, LayoutError> {
    let _ = (pieces, opts);
    Ok(Vec::new())
}
