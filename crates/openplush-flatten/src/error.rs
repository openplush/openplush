use openplush_geom::GeomError;
use openplush_mesh::FaceId;
use openplush_seams::ChartId;
use thiserror::Error;

/// Every way `openplush-flatten` refuses its input or its own result.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum FlattenError {
    /// A chart with no faces has nothing to verify.
    #[error("chart {} has no faces", .chart.index())]
    EmptyChart {
        /// The chart.
        chart: ChartId,
    },
    /// Faces whose flat orientation is not strictly positive (det J ≤ 0).
    #[error("chart {} flattens with {} inverted faces", .chart.index(), .faces.len())]
    Flip {
        /// The chart.
        chart: ChartId,
        /// Every inverted face, in input order.
        faces: Vec<FaceId>,
    },
    /// A face whose flat coordinates no exact predicate can judge.
    #[error("face {} of chart {} has coordinates the orientation test refuses: {source}", .face.index(), .chart.index())]
    Unjudgeable {
        /// The chart.
        chart: ChartId,
        /// The face.
        face: FaceId,
        /// The predicate's refusal.
        source: GeomError,
    },
    /// A face with zero, non-finite or overflowing area, or a vertex with no position: it has no rest shape.
    #[error("face {} of chart {} has no measurable area", .face.index(), .chart.index())]
    Degenerate {
        /// The chart.
        chart: ChartId,
        /// The face.
        face: FaceId,
    },
    /// The flattener's own guard: the chart is not one connected, consistently wound surface
    /// with a single boundary loop and Euler characteristic 1.
    #[error("chart {} is not a disk", .chart.index())]
    NotADisk {
        /// The chart.
        chart: ChartId,
    },
    /// A sparse solve was refused (not positive definite, not finite, or out of memory).
    #[error("a linear solve for chart {} failed", .chart.index())]
    SolveFailed {
        /// The chart.
        chart: ChartId,
    },
    /// The boundary could not be closed at its 3D edge lengths.
    #[error("the boundary of chart {} cannot be laid flat at its 3D edge lengths", .chart.index())]
    BoundaryNotRealizable {
        /// The chart.
        chart: ChartId,
    },
}
