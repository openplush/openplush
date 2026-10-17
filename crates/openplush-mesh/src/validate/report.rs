use crate::{FaceId, VertexId};

/// Every defect `validate` found, each located by soup IDs. Topology is judged as
/// repair will see it: coincident vertices welded, repeated-index faces and later
/// same-wound copies left out, both layers of a double-sided sheet kept. A vertex there is named by its weld group's
/// smallest ID. Every list is sorted.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ValidationReport {
    pub(super) unmeasurable: Option<Unmeasurable>,
    pub(super) unwelded: Vec<Vec<VertexId>>,
    pub(super) zero_area: Vec<FaceId>,
    pub(super) repeated_index: Vec<FaceId>,
    pub(super) duplicates: Vec<Duplicate>,
    pub(super) non_manifold_edges: Vec<NonManifoldEdge>,
    pub(super) non_manifold_vertices: Vec<VertexId>,
    pub(super) winding_conflicts: Vec<[VertexId; 2]>,
    pub(super) non_orientable: Vec<[VertexId; 2]>,
    pub(super) inside_out: Vec<Vec<FaceId>>,
    pub(super) boundary_loops: Vec<Vec<VertexId>>,
    pub(super) components: Vec<Vec<FaceId>>,
    pub(super) self_intersections: Vec<[FaceId; 2]>,
}

/// Why a soup has no model-space tolerances, so nothing else could be judged.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Unmeasurable {
    /// The soup has no triangles.
    NoTriangles,
    /// The used vertices' bounding-box diagonal is outside what tolerances allow
    /// (`openplush_geom::epsilon`): all at one point, or far too large.
    Extent {
        /// The diagonal, in model units.
        diagonal: f64,
    },
}

/// A face whose welded vertices repeat an earlier face's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Duplicate {
    /// The later copy.
    pub face: FaceId,
    /// The first face with these vertices wound the copy's way, or, for an
    /// opposite copy, the first wound the other way.
    pub of: FaceId,
    /// Whether the copy turns the same way; opposite copies make a double-sided sheet.
    pub orientation: DuplicateOrientation,
}

/// How a duplicate face is wound relative to the face it repeats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DuplicateOrientation {
    /// The same winding.
    Same,
    /// The opposite winding.
    Opposite,
}

/// An edge used by more than two faces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonManifoldEdge {
    /// Its ends, smaller ID first.
    pub vertices: [VertexId; 2],
    /// Every face using it.
    pub faces: Vec<FaceId>,
}

impl ValidationReport {
    pub(super) fn unmeasured(why: Unmeasurable) -> Self {
        Self {
            unmeasurable: Some(why),
            ..Self::default()
        }
    }

    /// Set when nothing could be judged; every other list is then empty.
    pub fn unmeasurable(&self) -> Option<Unmeasurable> {
        self.unmeasurable
    }

    /// Groups of used vertices within the weld distance of each other.
    pub fn unwelded(&self) -> &[Vec<VertexId>] {
        &self.unwelded
    }

    /// Faces with area at or below the model's area tolerance, repeated-index ones included.
    pub fn zero_area(&self) -> &[FaceId] {
        &self.zero_area
    }

    /// Faces naming one welded vertex twice.
    pub fn repeated_index(&self) -> &[FaceId] {
        &self.repeated_index
    }

    /// Later copies of a face, with the face they repeat.
    pub fn duplicates(&self) -> &[Duplicate] {
        &self.duplicates
    }

    /// Edges with more than two faces.
    pub fn non_manifold_edges(&self) -> &[NonManifoldEdge] {
        &self.non_manifold_edges
    }

    /// Vertices whose faces form more than one fan joined through shared edges.
    pub fn non_manifold_vertices(&self) -> &[VertexId] {
        &self.non_manifold_vertices
    }

    /// Two-face edges both faces traverse in the same direction.
    pub fn winding_conflicts(&self) -> &[[VertexId; 2]] {
        &self.winding_conflicts
    }

    /// Two-face edges where no choice of face flips makes the winding agree: the
    /// component is non-orientable.
    pub fn non_orientable(&self) -> &[[VertexId; 2]] {
        &self.non_orientable
    }

    /// Closed shells (faces joined across two-face edges) whose signed volume is not
    /// positive, by their faces.
    pub fn inside_out(&self) -> &[Vec<FaceId>] {
        &self.inside_out
    }

    /// Loops of one-face edges in their faces' direction, split where a vertex
    /// repeats, each from its smallest vertex.
    pub fn boundary_loops(&self) -> &[Vec<VertexId>] {
        &self.boundary_loops
    }

    /// Vertex-connected components, by their faces; more than one is a defect.
    pub fn components(&self) -> &[Vec<FaceId>] {
        &self.components
    }

    /// Pairs of faces sharing no edge that cross along a segment longer than the weld
    /// distance, smaller face first. Repair only warns about these.
    pub fn self_intersections(&self) -> &[[FaceId; 2]] {
        &self.self_intersections
    }
}
