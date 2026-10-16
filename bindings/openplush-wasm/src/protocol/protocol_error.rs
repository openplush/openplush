use openplush_pipeline::ChartId;

/// A value the binding refuses to put on the wire.
#[derive(Debug, Clone, Copy, thiserror::Error)]
#[non_exhaustive]
pub enum ProtocolError {
    /// JSON has no NaN or infinity, and the TypeScript type says `number`.
    #[error("chart {}: progress maxDistortion is {value}, not a finite number", chart.index())]
    NonFiniteMaxDistortion {
        /// The chart being flattened.
        chart: ChartId,
        /// The refused value.
        value: f64,
    },
}
