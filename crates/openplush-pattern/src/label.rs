//! Piece labels: the block of text naming a piece, and the label on each side of each seam.

use openplush_geom::DVec2;
use openplush_seams::SeamChainId;

use crate::{CutCount, PatternError, PieceId};

/// A seam's name at the middle of one side of it, in the piece's millimetres.
#[derive(Debug, Clone, PartialEq)]
pub struct SeamLabel {
    at: DVec2,
    text: String,
}

impl SeamLabel {
    /// Refuses a point that is not finite.
    pub fn new(at: DVec2, text: String) -> Result<Self, PatternError> {
        if !at.is_finite() {
            return Err(PatternError::BadLabelPoint);
        }
        Ok(Self { at, text })
    }

    /// Where the text is centred.
    pub fn at(&self) -> DVec2 {
        self.at
    }

    /// The seam's name.
    pub fn text(&self) -> &str {
        &self.text
    }
}

/// A piece's label: its block of lines centred on one point inside it, and its seam labels.
#[derive(Debug, Clone, PartialEq)]
pub struct Label {
    at: DVec2,
    lines: Vec<String>,
    seams: Vec<SeamLabel>,
}

impl Label {
    /// Refuses a point that is not finite.
    pub fn new(at: DVec2, lines: Vec<String>, seams: Vec<SeamLabel>) -> Result<Self, PatternError> {
        if !at.is_finite() {
            return Err(PatternError::BadLabelPoint);
        }
        Ok(Self { at, lines, seams })
    }

    /// Where the block is centred.
    pub fn at(&self) -> DVec2 {
        self.at
    }

    /// The block's lines, top first.
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// One per side of a seam on the piece.
    pub fn seams(&self) -> &[SeamLabel] {
        &self.seams
    }
}

/// The block's lines: the piece, its cuts, its fabric, and the pieces it sews to when there are
/// any, numbered as the warning summary numbers them.
pub fn block(id: PieceId, cuts: CutCount, fabric: &str, sews_to: &[PieceId]) -> Vec<String> {
    let cut = match (cuts.as_printed(), cuts.flipped()) {
        (n, 0) => format!("Cut {n}"),
        (0, f) => format!("Cut {f} flipped"),
        (n, f) => format!("Cut {} ({n} + {f} flipped)", n + f),
    };
    let mut lines = vec![format!("Piece {}", id.index()), cut, fabric.to_owned()];
    let names: Vec<String> = sews_to.iter().map(|p| p.index().to_string()).collect();
    match names.len() {
        0 => {}
        1 => lines.push(format!("Sews to piece {}", names[0])),
        _ => lines.push(format!("Sews to pieces {}", names.join(", "))),
    }
    lines
}

/// A seam's name on the print, as the warning summary names its chain.
pub fn seam_text(chain: SeamChainId) -> String {
    format!("Seam {}", chain.index())
}
