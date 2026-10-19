//! Boundary prescription: each chart's boundary laid flat with every edge at its 3D length,
//! so the two sides of a seam agree by construction (Sawhney & Crane 2017, lengths held fixed).

mod closure;
mod curvature;
#[cfg(test)]
pub(crate) mod fixtures;
mod topology;

use std::collections::BTreeMap;

use openplush_geom::{DVec2, DVec3};
use openplush_mesh::{FaceId, VertexId};

use crate::solver::SolveError;

/// Why a boundary could not be laid flat. The caller maps it to a `FlattenError` naming its chart.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum BoundaryError {
    /// A chart with no faces.
    Empty,
    /// A face naming a vertex with no position.
    BadVertex { face: FaceId },
    /// A face with zero or non-finite area.
    Degenerate { face: FaceId },
    /// Not one connected, consistently wound surface with a single boundary loop and Euler
    /// characteristic 1.
    NotADisk,
    /// The interior curvature system could not be solved.
    Solve(SolveError),
    /// Fewer than three edges, a length that is not finite and positive, a non-finite
    /// turning angle, or lengths and angles of different counts.
    BadInput,
    /// No change to the turning angles closed the polygon.
    NotClosed,
}

impl From<SolveError> for BoundaryError {
    fn from(e: SolveError) -> Self {
        Self::Solve(e)
    }
}

/// Each edge's cotangent weight, keyed smaller vertex first (see `curvature::Intrinsic`).
pub(crate) fn cotangent_weights(
    xyz: &[DVec3],
    faces: &[(FaceId, [VertexId; 3])],
) -> Result<BTreeMap<(u32, u32), f64>, BoundaryError> {
    Ok(curvature::intrinsic(xyz, faces)?.weight)
}

/// A chart's boundary as the interior curvature would turn it, before any closing.
pub(crate) struct Open {
    /// The boundary loop, from its lowest vertex in the faces' winding.
    pub(crate) ring: Vec<u32>,
    /// How far that polygon ends from where it starts, in the chart's units.
    pub(crate) gap: f64,
    /// Each used vertex's sum of corner angles.
    pub(crate) angle_sum: BTreeMap<u32, f64>,
}

/// Everything `prescribe` and `open_polygon` share: the loop, its 3D edge lengths and the
/// turning the interior asks for.
struct Setup {
    geometry: curvature::Intrinsic,
    ring: Vec<u32>,
    lengths: Vec<f64>,
    target: Vec<f64>,
}

fn setup(xyz: &[DVec3], faces: &[(FaceId, [VertexId; 3])]) -> Result<Setup, BoundaryError> {
    let geometry = curvature::intrinsic(xyz, faces)?;
    let ring = topology::boundary_loop(faces)?;
    let target = curvature::turning(&geometry, &ring)?;
    // INVARIANT: `intrinsic` found every vertex of every face in `xyz`.
    let at = |v: u32| xyz[v as usize];
    let lengths = ring
        .iter()
        .zip(ring.iter().cycle().skip(1))
        .map(|(&a, &b)| (at(b) - at(a)).length())
        .collect();
    Ok(Setup {
        geometry,
        ring,
        lengths,
        target,
    })
}

/// The boundary polygon at the interior's own turning, with how far it misses closing.
pub(crate) fn open_polygon(
    xyz: &[DVec3],
    faces: &[(FaceId, [VertexId; 3])],
) -> Result<Open, BoundaryError> {
    let s = setup(xyz, faces)?;
    Ok(Open {
        gap: closure::gap(&s.lengths, &s.target),
        angle_sum: s.geometry.angle_sum,
        ring: s.ring,
    })
}

/// The chart's boundary corners laid flat, from its lowest vertex in the faces' winding
/// (counter-clockwise), each edge exactly its 3D length. `faces` index `xyz`.
pub(crate) fn prescribe(
    xyz: &[DVec3],
    faces: &[(FaceId, [VertexId; 3])],
) -> Result<Vec<(VertexId, DVec2)>, BoundaryError> {
    let Setup {
        ring,
        lengths,
        target,
        ..
    } = setup(xyz, faces)?;
    let kappa = closure::close(&lengths, &target)?;
    let corners = closure::walk(&lengths, &kappa);
    Ok(ring.into_iter().map(VertexId::new).zip(corners).collect())
}

#[cfg(test)]
mod tests;
