//! Triangle soups, validation, repair, and the half-edge `ManifoldMesh`.
//!
//! This crate owns the import-side representations of data-model section 1. Each
//! stage's type guarantees what the next stage relies on: a `TriangleSoup` has
//! finite coordinates and valid indices but may carry any mesh defect; a
//! `ManifoldMesh` (later tasks) satisfies the section 2 invariants and can only be
//! obtained through repair.

mod error;
mod halfedge;
mod ids;
mod soup;

pub use error::{CornerArray, Counted, MeshError};
pub use halfedge::{ManifoldMesh, Outgoing};
pub use ids::{EdgeId, FaceId, HalfEdgeId, VertexId};
pub use soup::{SourceCorner, TriangleSoup};
