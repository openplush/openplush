use serde::Serialize;

/// Error codes the binding and its host raise, beside `EngineError`'s variant names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub enum BindingCode {
    /// The handle was released, or belongs to an earlier worker.
    StaleHandle,
    /// The engine trapped. Listed for the TypeScript generator only: the worker host
    /// raises it, never Rust, because a trapped instance runs no more code.
    EngineCrashed,
}
