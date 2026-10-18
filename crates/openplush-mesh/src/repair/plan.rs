//! What the maker chose for repair, and what repair did or found.

use std::collections::BTreeSet;

use super::bowtie::VertexCopy;
use super::components::Component;
use super::degenerate::DroppedCopy;
use super::nonmanifold_edge::PairedEdge;
use crate::{FaceId, SourceCorner, VertexId};

/// A boundary loop, named by the lowest source corner among its vertices, so the name is
/// the same on every replay whatever the repaired numbering is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LoopRef(SourceCorner);

impl LoopRef {
    /// The loop whose lowest source corner is `corner`.
    pub fn new(corner: SourceCorner) -> Self {
        Self(corner)
    }

    /// The lowest source corner among the loop's vertices.
    pub fn corner(self) -> SourceCorner {
        self.0
    }
}

/// The repair choices the maker made, replayed on load. Every step is on by default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepairPlan {
    /// Merge coincident vertices.
    pub weld: bool,
    /// Drop repeated-index faces and duplicates, and stitch caps.
    pub drop_degenerates: bool,
    /// Pair the faces on edges with more than two.
    pub split_non_manifold_edges: bool,
    /// Split a vertex whose faces form more than one fan.
    pub split_bowties: bool,
    /// Turn faces to agree and closed shells outward.
    pub fix_orientation: bool,
    /// Loops to fill. A loop not listed stays an opening.
    pub fill_holes: BTreeSet<LoopRef>,
}

impl Default for RepairPlan {
    fn default() -> Self {
        Self {
            weld: true,
            drop_degenerates: true,
            split_non_manifold_edges: true,
            split_bowties: true,
            fix_orientation: true,
            fill_holes: BTreeSet::new(),
        }
    }
}

/// A boundary loop, whether the plan filled it, and the faces a fill added.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoleReport {
    pub(crate) loop_ref: LoopRef,
    pub(crate) filled: bool,
    pub(crate) added_faces: Vec<FaceId>,
}

impl HoleReport {
    /// The loop.
    pub fn loop_ref(&self) -> LoopRef {
        self.loop_ref
    }

    /// Whether the plan named the loop and it was filled.
    pub fn filled(&self) -> bool {
        self.filled
    }

    /// The faces the fill added, by the repaired mesh's IDs; empty if not filled.
    pub fn added_faces(&self) -> &[FaceId] {
        &self.added_faces
    }
}

/// Everything repair did or found. Removals, flips and pairings name faces and vertices
/// of the soup given to repair; additions name those of the repaired mesh.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RepairReport {
    pub(crate) welded: Vec<Vec<VertexId>>,
    pub(crate) repeated_index_faces: Vec<FaceId>,
    pub(crate) duplicate_faces: Vec<DroppedCopy>,
    pub(crate) sheets: Vec<[FaceId; 2]>,
    pub(crate) stitched_caps: Vec<FaceId>,
    pub(crate) retriangulated_faces: Vec<FaceId>,
    pub(crate) paired_edges: Vec<PairedEdge>,
    pub(crate) vertex_copies: Vec<VertexCopy>,
    pub(crate) flipped_faces: Vec<FaceId>,
    pub(crate) orientation_conflicts: Vec<[VertexId; 2]>,
    pub(crate) components: Vec<Component>,
    pub(crate) holes: Vec<HoleReport>,
    pub(crate) mandatory_seams: Vec<[VertexId; 2]>,
}

impl RepairReport {
    /// Each group of vertices merged into one.
    pub fn welded(&self) -> &[Vec<VertexId>] {
        &self.welded
    }

    /// Faces dropped for repeating a vertex.
    pub fn repeated_index_faces(&self) -> &[FaceId] {
        &self.repeated_index_faces
    }

    /// Faces dropped as same-wound copies of an earlier face.
    pub fn duplicate_faces(&self) -> &[DroppedCopy] {
        &self.duplicate_faces
    }

    /// Pairs of coincident opposite-wound faces kept as the layers of a sheet.
    pub fn sheets(&self) -> &[[FaceId; 2]] {
        &self.sheets
    }

    /// Zero-area faces dropped after their vertices were put on their neighbours' edges.
    pub fn stitched_caps(&self) -> &[FaceId] {
        &self.stitched_caps
    }

    /// Faces re-triangulated with a cap's vertex on an edge.
    pub fn retriangulated_faces(&self) -> &[FaceId] {
        &self.retriangulated_faces
    }

    /// Edges with more than two faces, and how their faces pair.
    pub fn paired_edges(&self) -> &[PairedEdge] {
        &self.paired_edges
    }

    /// Vertices added to separate fans.
    pub fn vertex_copies(&self) -> &[VertexCopy] {
        &self.vertex_copies
    }

    /// Faces whose corners were reversed.
    pub fn flipped_faces(&self) -> &[FaceId] {
        &self.flipped_faces
    }

    /// Edges whose two faces no flips make agree.
    pub fn orientation_conflicts(&self) -> &[[VertexId; 2]] {
        &self.orientation_conflicts
    }

    /// Each connected component, with whether it is a candidate detail part.
    pub fn components(&self) -> &[Component] {
        &self.components
    }

    /// Each boundary loop, filled or left open.
    pub fn holes(&self) -> &[HoleReport] {
        &self.holes
    }

    /// Edges that must be seams because repair made them boundaries.
    pub fn mandatory_seams(&self) -> &[[VertexId; 2]] {
        &self.mandatory_seams
    }
}

#[cfg(test)]
mod tests;
