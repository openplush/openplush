//! One chart's distortion in a few numbers: its worst faces and its area-weighted measures.
//! Raw values only; tolerances are the fabric's.

use std::cmp::Ordering;

use openplush_geom::{DVec2, DVec3};
use openplush_mesh::{FaceId, VertexId};
use openplush_seams::ChartId;

use crate::metrics::{Distortion, distortion_and_area};
use crate::{FlattenError, StoppedBy};

/// The share of the area the tails cover. The single worst face keeps moving as a mesh is
/// refined; the worst 5% of the area settles.
const TAIL: f64 = 0.05;

/// A chart's distortion summary.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Summary {
    most_stretched: (FaceId, f64),
    most_compressed: (FaceId, f64),
    tail_stretch: f64,
    tail_compression: f64,
    stretch_excess: f64,
    compression_deficit: f64,
    area_ratio: f64,
    stopped_by: StoppedBy,
}

impl Summary {
    /// The face with the largest σ₁, and that σ₁; the first in input order on a tie.
    pub fn most_stretched(&self) -> (FaceId, f64) {
        self.most_stretched
    }

    /// The face with the smallest σ₂, and that σ₂; the first in input order on a tie.
    pub fn most_compressed(&self) -> (FaceId, f64) {
        self.most_compressed
    }

    /// The largest σ₁ such that the faces stretched at least that much cover 5% of the area.
    pub fn tail_stretch(&self) -> f64 {
        self.tail_stretch
    }

    /// The smallest σ₂ such that the faces compressed at least that much cover 5% of the area.
    pub fn tail_compression(&self) -> f64 {
        self.tail_compression
    }

    /// Σ A·max(0, σ₁ − 1) / ΣA over 3D areas A.
    pub fn stretch_excess(&self) -> f64 {
        self.stretch_excess
    }

    /// Σ A·max(0, 1 − σ₂) / ΣA over 3D areas A.
    pub fn compression_deficit(&self) -> f64 {
        self.compression_deficit
    }

    /// Total flat area over total 3D area.
    pub fn area_ratio(&self) -> f64 {
        self.area_ratio
    }

    /// Why the flatten that made the layout stopped, as given to [`summarize`].
    pub fn stopped_by(&self) -> StoppedBy {
        self.stopped_by
    }
}

/// `chart`'s summary. `uv` is indexed like `xyz`; `stopped_by` is why the flatten that made it
/// stopped, which the layout alone cannot show.
///
/// # Errors
/// Those of [`distortion`](crate::metrics::distortion).
pub fn summarize(
    chart: ChartId,
    xyz: &[DVec3],
    faces: &[(FaceId, [VertexId; 3])],
    uv: &[DVec2],
    stopped_by: StoppedBy,
) -> Result<Summary, FlattenError> {
    let measured = distortion_and_area(chart, xyz, faces, uv)?;
    let stretch = extremes(&measured, Distortion::stretch, |a, b| b.total_cmp(a));
    let compression = extremes(&measured, Distortion::compression, f64::total_cmp);
    let (Some((most_stretched, tail_stretch)), Some((most_compressed, tail_compression))) =
        (stretch, compression)
    else {
        return Err(FlattenError::EmptyChart { chart });
    };
    Ok(Summary {
        most_stretched,
        most_compressed,
        tail_stretch,
        tail_compression,
        stretch_excess: mean(&measured, |d| (d.stretch() - 1.0).max(0.0)),
        compression_deficit: mean(&measured, |d| (1.0 - d.compression()).max(0.0)),
        area_ratio: mean(&measured, Distortion::area_ratio),
        stopped_by,
    })
}

type Weighted = (FaceId, Distortion, f64);

/// The worst face by `order` and the value at which the worst faces first cover `TAIL` of the
/// area; `None` with no faces.
fn extremes(
    faces: &[Weighted],
    value: fn(&Distortion) -> f64,
    order: fn(&f64, &f64) -> Ordering,
) -> Option<((FaceId, f64), f64)> {
    let mut sorted: Vec<(FaceId, f64, f64)> =
        faces.iter().map(|(f, d, w)| (*f, value(d), *w)).collect();
    // Stable, so a tie keeps input order and the worst is the first such face.
    sorted.sort_by(|a, b| order(&a.1, &b.1));
    let threshold = TAIL * sorted.iter().map(|s| s.2).sum::<f64>();
    let mut covered = 0.0;
    // The threshold is under the sorted list's own total, so the last face always reaches it.
    let tail = sorted.iter().find(|s| {
        covered += s.2;
        covered >= threshold
    })?;
    sorted.first().map(|s| ((s.0, s.1), tail.1))
}

/// `value` averaged by 3D area, updated face by face so no partial sum of σ overflows.
fn mean(faces: &[Weighted], value: impl Fn(&Distortion) -> f64) -> f64 {
    let (mut total, mut mean) = (0.0, 0.0);
    for (_, d, w) in faces {
        // Every 3D area is positive, so `total` is too.
        total += w;
        mean += w / total * (value(d) - mean);
    }
    mean
}
