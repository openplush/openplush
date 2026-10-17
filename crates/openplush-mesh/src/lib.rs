//! Triangle soups, validation, repair, and the half-edge `ManifoldMesh`.
//!
//! This crate owns the import-side mesh representations. Each
//! stage's type guarantees what the next stage relies on: a `TriangleSoup` has
//! finite coordinates and valid indices but may carry any mesh defect; a
//! `ManifoldMesh` satisfies the half-edge invariants and can only be
//! obtained through repair.

mod disjoint;
mod error;
mod halfedge;
mod ids;
mod repair;
mod soup;
mod validate;

pub use error::{CornerArray, Counted, MaterialFault, MeshError};
pub use halfedge::{ManifoldMesh, Outgoing};
pub use ids::{ComponentId, EdgeId, FaceId, HalfEdgeId, MaterialIndex, MeshElement, VertexId};
pub use soup::{SourceCorner, SourceMaterials, TriangleSoup};
pub use validate::{
    Duplicate, DuplicateOrientation, NonManifoldEdge, Unmeasurable, ValidationReport, validate,
};
