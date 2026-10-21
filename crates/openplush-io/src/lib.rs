//! Bytes to `TriangleSoup`: OBJ, glTF, STL, PLY.

mod affine;
mod error;
pub mod gltf;
mod imported;
pub mod obj;
pub mod ply;
pub mod stl;
mod triangulate;

pub use error::IoError;
pub use imported::{Ignored, Imported};

/// A file format the importer reads. Named in
/// `EngineError::Parse` so the maker knows which reader refused the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Format {
    /// Wavefront OBJ: geometry and texture coordinates.
    Obj,
    /// glTF 2.0, JSON with separate buffers.
    Gltf,
    /// glTF 2.0, binary container.
    Glb,
    /// STL, ASCII or binary.
    Stl,
    /// PLY, ASCII or binary.
    Ply,
}

impl Format {
    /// The name shown to the maker.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Obj => "OBJ",
            Self::Gltf => "glTF",
            Self::Glb => "GLB",
            Self::Stl => "STL",
            Self::Ply => "PLY",
        }
    }
}
