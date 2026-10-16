use serde::Serialize;

/// A mesh the worker owns; the shell holds only its number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
// The brand exists only in TypeScript, so a MeshHandle cannot be passed where another handle is expected.
#[cfg_attr(
    test,
    derive(ts_rs::TS),
    ts(export, type = "{ readonly __brand: 'mesh'; readonly n: number }")
)]
pub struct MeshHandle {
    n: u32,
}

// The `ts(type)` above writes `n: number` by hand; this stops compiling if `n` stops being a `u32`.
const _: fn(MeshHandle) -> u32 = |h| h.n;

impl MeshHandle {
    /// Wraps a handle number the worker issued.
    pub fn new(n: u32) -> Self {
        Self { n }
    }
}

/// A pipeline the worker owns; the shell holds only its number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
// The brand exists only in TypeScript, so a PipelineHandle cannot be passed where another handle is expected.
#[cfg_attr(
    test,
    derive(ts_rs::TS),
    ts(export, type = "{ readonly __brand: 'pipeline'; readonly n: number }")
)]
pub struct PipelineHandle {
    n: u32,
}

// The `ts(type)` above writes `n: number` by hand; this stops compiling if `n` stops being a `u32`.
const _: fn(PipelineHandle) -> u32 = |h| h.n;

impl PipelineHandle {
    /// Wraps a handle number the worker issued.
    pub fn new(n: u32) -> Self {
        Self { n }
    }
}
