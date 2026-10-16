use openplush_pipeline::ChartId;
use serde::Serialize;

use super::ProtocolError;

/// How far a step-wise operation has got.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[non_exhaustive]
pub enum Progress {
    /// A chart is still being flattened.
    Working(Working),
    /// The operation finished; its result follows in the terminal message.
    Done,
}

/// One flattening iteration's state. `max_distortion` is always finite.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Working {
    chart: u32,
    iteration: u32,
    max_distortion: f64,
}

impl Working {
    /// Refuses a non-finite distortion rather than clamping it.
    ///
    /// # Errors
    /// [`ProtocolError::NonFiniteMaxDistortion`] for NaN or ±∞, which JSON would send as `null`.
    pub fn new(chart: ChartId, iteration: u32, max_distortion: f64) -> Result<Self, ProtocolError> {
        if !max_distortion.is_finite() {
            return Err(ProtocolError::NonFiniteMaxDistortion {
                chart,
                value: max_distortion,
            });
        }
        Ok(Self {
            chart: chart.index(),
            iteration,
            max_distortion,
        })
    }
}
