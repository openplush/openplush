use serde::Serialize;

/// A mesh the worker owns; the shell holds only its number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct MeshHandle {
    n: u32,
}

impl MeshHandle {
    /// Wraps a handle number the worker issued.
    pub fn new(n: u32) -> Self {
        Self { n }
    }
}

/// A pipeline the worker owns; the shell holds only its number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct PipelineHandle {
    n: u32,
}

impl PipelineHandle {
    /// Wraps a handle number the worker issued.
    pub fn new(n: u32) -> Self {
        Self { n }
    }
}
