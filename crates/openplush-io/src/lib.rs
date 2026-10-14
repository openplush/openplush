//! Bytes to `TriangleSoup`: OBJ, glTF, STL, PLY.
//!
//! Scaffold, plus the type other crates name (T175). Responsibilities and the dependency DAG are in
//! `specs/001-pattern-pipeline/plan.md` and `crates/AGENTS.md`.

/// A file format the importer reads (data-model section 13a). Named in
/// `EngineError::Parse` so the maker knows which reader refused the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Format {
    /// Wavefront OBJ, with MTL and textures (T050).
    Obj,
    /// glTF 2.0, JSON with separate buffers (T051).
    Gltf,
    /// glTF 2.0, binary container (T051).
    Glb,
    /// STL, ASCII or binary (T052).
    Stl,
    /// PLY, ASCII or binary (T052).
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
