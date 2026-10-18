//! Pattern pieces: allowance, notches, labels, grain, piece dedupe.

mod error;
pub mod parity;

pub use error::PatternError;

use openplush_mesh::id_newtype;

id_newtype! {
    /// A pattern piece: one per chart, numbered as its chart. When mirror-image
    /// pieces collapse into one template, the template lists its member `PieceId`s
    /// and is named by the lowest. Stable for one `SeamSet`.
    PieceId
}
