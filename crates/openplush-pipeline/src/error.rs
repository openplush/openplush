use std::collections::BTreeSet;

use openplush_io::Format;
use openplush_mesh::{EdgeId, FaceId, MeshError, SourceCorner, VertexId};
use openplush_pattern::PieceId;
use openplush_seams::{ChartFailure, ChartId, Proposal, ProposalId, SeamChainId, SeamsError};
use thiserror::Error;

use crate::locations::mesh_locations;

/// A violated invariant: the stage produced no output. Every variant but
/// `Cancelled` names where the problem is, so the UI can highlight it.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum EngineError {
    /// The importer for `format` could not read the file.
    #[error("could not read the {} file: {detail}", .format.name())]
    Parse {
        /// The reader that refused the file.
        format: Format,
        /// What it refused, in the reader's words.
        detail: String,
    },
    /// No reader takes the primary file's extension; no reader ran.
    #[error("cannot read \"{name}\": no reader for this file type")]
    UnknownFormat {
        /// The primary file's name, as the shell gave it.
        name: String,
    },
    /// The file refers to another file the shell did not supply beside it.
    #[error("the file refers to \"{uri}\", which was not supplied beside it")]
    MissingResource {
        /// The reference, as the file writes it.
        uri: String,
    },
    /// Source vertices with a NaN or infinite coordinate; import refuses rather than drops them.
    #[error("{} source vertices have a NaN or infinite coordinate", .vertices.len())]
    NonFiniteInput {
        /// Every non-finite source vertex, used by a face or not.
        vertices: Vec<SourceCorner>,
    },
    /// Repair refused the soup; the inner error says how, naming the soup's own elements.
    #[error(transparent)]
    Mesh(#[from] MeshError),
    /// A chart is not a topological disk, so it cannot flatten as one piece.
    #[error("chart {} is not a topological disk", .chart.index())]
    ChartNotDisk {
        /// The chart.
        chart: ChartId,
        /// Why it is not a disk.
        failure: ChartFailure,
        /// The minimal extra seams that would make it one.
        proposals: Vec<Proposal>,
    },
    /// A chart lies back over itself, so it cannot flatten; refused before flattening.
    #[error("chart {} lies back over itself across edge {}", .chart.index(), .edge.index())]
    FoldedChart {
        /// The chart.
        chart: ChartId,
        /// The lowest edge it folds across.
        edge: EdgeId,
    },
    /// The seams cannot be applied to the mesh; the inner error says how and where.
    #[error(transparent)]
    Seams(#[from] SeamsError),
    /// Flattening inverted faces (det J ≤ 0).
    #[error("chart {} flattens with {} inverted faces", .chart.index(), .faces.len())]
    Flip {
        /// The chart.
        chart: ChartId,
        /// The inverted faces.
        faces: Vec<FaceId>,
    },
    /// The prescribed seam lengths cannot close into a flat outline.
    #[error(
        "chart {}'s seam lengths cannot close flat (residual {residual_mm:.3} mm)",
        .chart.index()
    )]
    BoundaryNotRealizable {
        /// The chart.
        chart: ChartId,
        /// How far the outline fails to close, in millimetres.
        residual_mm: f64,
        /// The darts that would let it close.
        proposals: Vec<Proposal>,
    },
    /// The two sides of a seam chain differ by more than the tolerance; a flattener bug.
    #[error(
        "seam chain {}: sides {a_mm:.3} mm and {b_mm:.3} mm differ by more than {tolerance_mm:.3} mm",
        .chain.index()
    )]
    SeamParity {
        /// The chain.
        chain: SeamChainId,
        /// One side's length, in millimetres.
        a_mm: f64,
        /// The other side's length, in millimetres.
        b_mm: f64,
        /// The largest allowed difference, in millimetres.
        tolerance_mm: f64,
    },
    /// The seam allowance could not be offset from the piece outline.
    #[error("seam allowance for piece {} failed: {detail}", .piece.index())]
    OffsetFailed {
        /// The piece.
        piece: PieceId,
        /// Why the offset failed.
        detail: String,
    },
    /// A proposal was applied that the current state does not offer; nothing was applied.
    #[error("proposal {} is no longer offered", .id.to_hex())]
    ProposalNotOffered {
        /// The proposal asked for.
        id: ProposalId,
    },
    /// The caller cancelled a session; no partial output exists.
    #[error("cancelled")]
    Cancelled,
}

impl EngineError {
    /// The variant name, e.g. `"ChartNotDisk"`; stable on the wire.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::Parse { .. } => "Parse",
            Self::UnknownFormat { .. } => "UnknownFormat",
            Self::MissingResource { .. } => "MissingResource",
            Self::NonFiniteInput { .. } => "NonFiniteInput",
            Self::Mesh(_) => "Mesh",
            Self::ChartNotDisk { .. } => "ChartNotDisk",
            Self::FoldedChart { .. } => "FoldedChart",
            Self::Seams(_) => "Seams",
            Self::Flip { .. } => "Flip",
            Self::BoundaryNotRealizable { .. } => "BoundaryNotRealizable",
            Self::SeamParity { .. } => "SeamParity",
            Self::OffsetFailed { .. } => "OffsetFailed",
            Self::ProposalNotOffered { .. } => "ProposalNotOffered",
            Self::Cancelled => "Cancelled",
        }
    }

    /// Where the problem is; empty for `Parse`, `UnknownFormat`, `MissingResource` and
    /// `Cancelled`.
    #[must_use]
    pub fn locations(&self) -> ErrorLocations {
        let none = ErrorLocations::default();
        match self {
            Self::Parse { .. }
            | Self::UnknownFormat { .. }
            | Self::MissingResource { .. }
            | Self::Cancelled => none,
            Self::NonFiniteInput { vertices } => ErrorLocations {
                source_vertices: sorted(vertices),
                ..none
            },
            Self::Mesh(inner) => mesh_locations(inner),
            // Proposal edges are not locations: proposals travel separately, as fixes.
            Self::ChartNotDisk { chart, .. } | Self::BoundaryNotRealizable { chart, .. } => {
                ErrorLocations {
                    charts: vec![*chart],
                    ..none
                }
            }
            Self::FoldedChart { chart, edge } => ErrorLocations {
                edges: vec![*edge],
                charts: vec![*chart],
                ..none
            },
            Self::Seams(inner) => seams_locations(inner),
            Self::Flip { chart, faces } => ErrorLocations {
                faces: sorted(faces),
                charts: vec![*chart],
                ..none
            },
            Self::SeamParity { chain, .. } => ErrorLocations {
                chains: vec![*chain],
                ..none
            },
            Self::OffsetFailed { piece, .. } => ErrorLocations {
                pieces: vec![*piece],
                ..none
            },
            Self::ProposalNotOffered { id } => ErrorLocations {
                proposal: Some(*id),
                ..none
            },
        }
    }

    /// The seams that would fix the problem; empty for variants that offer none.
    #[must_use]
    pub fn proposals(&self) -> &[Proposal] {
        match self {
            Self::ChartNotDisk { proposals, .. }
            | Self::BoundaryNotRealizable { proposals, .. } => proposals,
            Self::Parse { .. }
            | Self::UnknownFormat { .. }
            | Self::MissingResource { .. }
            | Self::NonFiniteInput { .. }
            | Self::Mesh(_)
            | Self::FoldedChart { .. }
            | Self::Seams(_)
            | Self::Flip { .. }
            | Self::SeamParity { .. }
            | Self::OffsetFailed { .. }
            | Self::ProposalNotOffered { .. }
            | Self::Cancelled => &[],
        }
    }
}

/// The elements an [`EngineError`] names, by category; each list is sorted and without duplicates.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ErrorLocations {
    /// Faces of the repaired mesh; for `Mesh`, of the soup repair was given.
    pub faces: Vec<FaceId>,
    /// Edges of the repaired mesh.
    pub edges: Vec<EdgeId>,
    /// Vertices of the repaired mesh; for `Mesh`, of the soup repair was given.
    pub vertices: Vec<VertexId>,
    /// Pattern pieces.
    pub pieces: Vec<PieceId>,
    /// Charts.
    pub charts: Vec<ChartId>,
    /// Seam chains.
    pub chains: Vec<SeamChainId>,
    /// Vertices of the imported file, before repair.
    pub source_vertices: Vec<SourceCorner>,
    /// The proposal asked for.
    pub proposal: Option<ProposalId>,
}

/// The elements `error` names; empty for a refusal about a whole proposal or dart.
fn seams_locations(error: &SeamsError) -> ErrorLocations {
    let none = ErrorLocations::default();
    match error {
        SeamsError::MandatorySeam { edge }
        | SeamsError::UnknownEdge { edge }
        | SeamsError::NotASeam { edge }
        | SeamsError::NotAWalk { edge } => ErrorLocations {
            edges: vec![*edge],
            ..none
        },
        SeamsError::ChartNotClosed { chart } => ErrorLocations {
            charts: vec![*chart],
            ..none
        },
        SeamsError::FoldedChart { chart, edge } => ErrorLocations {
            edges: vec![*edge],
            charts: vec![*chart],
            ..none
        },
        SeamsError::UnpairedBoundarySeam { chain } | SeamsError::CopyGroupMismatch { chain } => {
            ErrorLocations {
                chains: vec![*chain],
                ..none
            }
        }
        SeamsError::UnknownVertex { vertex }
        | SeamsError::DartNotAnchored { vertex }
        | SeamsError::DartTouchesSeam { vertex } => ErrorLocations {
            vertices: vec![*vertex],
            ..none
        },
        SeamsError::NoPath { from, to } => ErrorLocations {
            vertices: sorted(&[*from, *to]),
            ..none
        },
        _ => none,
    }
}

pub(crate) fn sorted<T: Ord + Copy>(ids: &[T]) -> Vec<T> {
    ids.iter()
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
