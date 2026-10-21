use openplush_pattern::PieceId;
use thiserror::Error;

/// Every way `openplush-export` refuses its input.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum ExportError {
    /// A placement names a piece that was not passed in.
    #[error("the sheet places piece {}, which was not given", .piece.index())]
    UnknownPiece {
        /// The piece.
        piece: PieceId,
    },
    /// Two pieces share an ID, so a placement could draw either.
    #[error("piece {} was given more than once", .piece.index())]
    DuplicatePiece {
        /// The piece.
        piece: PieceId,
    },
    /// Summary text has a character outside Latin-1: the PDF's standard fonts read WinAnsi, and
    /// the SVG keeps to the same rule so both print the same lines.
    #[error("{text:?} has {character:?}, which is outside the Latin-1 the exports can print")]
    Unprintable {
        /// The text, as given.
        text: String,
        /// Its first such character.
        character: char,
    },
}
