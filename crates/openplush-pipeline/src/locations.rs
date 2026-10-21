use openplush_flatten::FlattenError;
use openplush_mesh::{MeshElement, MeshError};

use crate::ErrorLocations;
use crate::error::sorted;

/// The soup elements `error` names; empty for a refusal about the soup as a whole.
pub(crate) fn mesh_locations(error: &MeshError) -> ErrorLocations {
    let none = ErrorLocations::default();
    match *error {
        MeshError::IndexOutOfRange { triangle, .. }
        | MeshError::DegenerateTriangle { triangle }
        | MeshError::ZeroAreaTriangle { triangle } => ErrorLocations {
            faces: vec![triangle],
            ..none
        },
        MeshError::DuplicateTriangle { triangle, first } => ErrorLocations {
            faces: sorted(&[triangle, first]),
            ..none
        },
        MeshError::NonFiniteVertex { vertex }
        | MeshError::IsolatedVertex { vertex }
        | MeshError::NonManifoldVertex { vertex } => ErrorLocations {
            vertices: vec![vertex],
            ..none
        },
        MeshError::NonManifoldEdge { a, b } | MeshError::InconsistentOrientation { a, b } => {
            ErrorLocations {
                vertices: sorted(&[a, b]),
                ..none
            }
        }
        MeshError::PairingMismatch { face, a, b } => ErrorLocations {
            faces: vec![face],
            vertices: sorted(&[a, b]),
            ..none
        },
        MeshError::UnknownHoleLoop { loop_ref }
        | MeshError::HoleNotFillable { loop_ref }
        | MeshError::HoleOnSplit { loop_ref }
        | MeshError::HoleTooLarge { loop_ref, .. } => ErrorLocations {
            source_vertices: vec![loop_ref.corner()],
            ..none
        },
        MeshError::BrokenInvariant { element, .. } => element_location(element),
        MeshError::CornerCountMismatch { .. }
        | MeshError::TooLarge { .. }
        | MeshError::InvalidMaterial { .. }
        | MeshError::NoTriangles
        | MeshError::UnusableExtent => none,
    }
}

/// The chart `error` names and, where it has one, its face or inverted faces.
pub(crate) fn flatten_locations(error: &FlattenError) -> ErrorLocations {
    let none = ErrorLocations::default();
    match error {
        FlattenError::Unjudgeable { chart, face, .. }
        | FlattenError::Degenerate { chart, face } => ErrorLocations {
            faces: vec![*face],
            charts: vec![*chart],
            ..none
        },
        FlattenError::Flip { chart, faces } => ErrorLocations {
            faces: sorted(faces),
            charts: vec![*chart],
            ..none
        },
        FlattenError::EmptyChart { chart }
        | FlattenError::NotADisk { chart }
        | FlattenError::SolveFailed { chart }
        | FlattenError::BoundaryNotRealizable { chart, .. } => ErrorLocations {
            charts: vec![*chart],
            ..none
        },
        _ => none,
    }
}

fn element_location(element: MeshElement) -> ErrorLocations {
    let none = ErrorLocations::default();
    match element {
        MeshElement::Vertex(v) => ErrorLocations {
            vertices: vec![v],
            ..none
        },
        MeshElement::HalfEdge(h) => ErrorLocations {
            edges: vec![h.edge()],
            ..none
        },
        MeshElement::Face(f) => ErrorLocations {
            faces: vec![f],
            ..none
        },
        MeshElement::Storage(_) => none,
    }
}
