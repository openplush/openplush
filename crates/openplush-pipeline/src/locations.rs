use openplush_export::ExportError;
use openplush_flatten::FlattenError;
use openplush_layout::LayoutError;
use openplush_mesh::{MeshElement, MeshError};
use openplush_pattern::PieceId;

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
        | MeshError::AmbiguousHoleLoop { loop_ref }
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

/// The piece a layout refusal names; a refusal about the roll, the page count or the empty set
/// names none.
pub(crate) fn layout_locations(error: &LayoutError) -> ErrorLocations {
    let piece = match error {
        LayoutError::PieceTooWide { piece, .. }
        | LayoutError::UnsupportedRotation { piece, .. } => Some(*piece),
        _ => None,
    };
    pieces(piece)
}

/// The piece an export refusal names; `Unprintable` names text, not an element.
pub(crate) fn export_locations(error: &ExportError) -> ErrorLocations {
    let piece = match error {
        ExportError::UnknownPiece { piece } | ExportError::DuplicatePiece { piece } => Some(*piece),
        _ => None,
    };
    pieces(piece)
}

fn pieces(piece: Option<PieceId>) -> ErrorLocations {
    ErrorLocations {
        pieces: piece.into_iter().collect(),
        ..ErrorLocations::default()
    }
}
