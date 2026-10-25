//! Pattern pieces: allowance, notches, labels, grain, piece dedupe.

pub mod allowance;
mod cut;
pub mod dedupe;
mod error;
mod fabric;
pub mod label;
pub mod notch;
mod outline;
pub mod parity;
mod piece;
pub mod scale;

pub use cut::CutCount;
pub use error::PatternError;
pub use fabric::{Fabric, FabricCategory, FabricSpec, StretchDirection};
pub use label::{Label, SeamLabel};
pub use notch::Notch;
pub use outline::Outline;
pub use piece::PatternPiece;

use openplush_mesh::id_newtype;

id_newtype! {
    /// A pattern piece: one per chart, numbered as its chart. When mirror-image
    /// pieces collapse into one template, the template lists its member `PieceId`s
    /// and is named by the lowest. Stable for one `SeamSet`.
    PieceId
}

id_newtype! {
    /// A fabric profile within one project.
    FabricId
}
