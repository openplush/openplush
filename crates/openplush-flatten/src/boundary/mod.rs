//! Boundary prescription: each chart's boundary laid flat with every edge at its 3D length,
//! so the two sides of a seam agree by construction (Sawhney & Crane 2017, lengths held fixed).

mod closure;

/// Why a boundary could not be laid flat. The caller maps it to a `FlattenError` naming its chart.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum BoundaryError {
    /// Fewer than three edges, a length that is not finite and positive, a non-finite
    /// turning angle, or lengths and angles of different counts.
    BadInput,
    /// No change to the turning angles closed the polygon.
    NotClosed,
}
