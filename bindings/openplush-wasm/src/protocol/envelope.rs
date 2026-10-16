use serde::{Deserialize, Serialize};

use super::{EngineErrorDto, Progress};

/// Correlates a request with its responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(transparent)]
pub struct RequestId(u32);

/// A message from the shell, correlated with its responses by `id`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct Request {
    id: RequestId,
    #[serde(flatten)]
    op: Op,
}

impl Request {
    /// The `id` every response to this request carries.
    pub fn id(&self) -> RequestId {
        self.id
    }

    /// What the shell asks for.
    pub fn op(&self) -> &Op {
        &self.op
    }
}

/// An operation and its payload.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(
    tag = "op",
    content = "payload",
    rename_all = "camelCase",
    deny_unknown_fields
)]
#[non_exhaustive]
pub enum Op {
    /// Stop the request with id `target` at its next step boundary.
    Cancel {
        /// The `id` of the request to stop.
        target: RequestId,
    },
}

/// A message to the shell: one terminal `Success` or `Failure` per request,
/// after zero or more `Progress` updates.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(untagged)]
pub enum Response<T> {
    /// The operation's result.
    Success(Success<T>),
    /// Why the operation produced no result.
    Failure(Failure),
    /// Advisory; the shell may drop it.
    Progress(ProgressUpdate),
}

impl<T> Response<T> {
    /// The terminal message of an operation that produced `result`.
    pub fn success(id: RequestId, result: T) -> Self {
        Self::Success(Success {
            id,
            ok: true,
            result,
            warnings: Vec::new(),
        })
    }

    /// The terminal message of an operation that failed.
    pub fn failure(id: RequestId, error: EngineErrorDto) -> Self {
        Self::Failure(Failure {
            id,
            ok: false,
            error: Box::new(error),
        })
    }

    /// A step's progress, before the terminal message.
    pub fn progress(id: RequestId, progress: Progress) -> Self {
        Self::Progress(ProgressUpdate { id, progress })
    }
}

/// A completed operation's result. `ok` is always `true`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct Success<T> {
    id: RequestId,
    // The TS literal lets the shell narrow on `ok`; the one constructor keeps it true.
    #[cfg_attr(test, ts(type = "true"))]
    ok: bool,
    result: T,
    warnings: Vec<WarningDto>,
}

/// A failed operation's error. `ok` is always `false`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct Failure {
    id: RequestId,
    // The TS literal lets the shell narrow on `ok`; the one constructor keeps it false.
    #[cfg_attr(test, ts(type = "false"))]
    ok: bool,
    error: Box<EngineErrorDto>,
}

/// A request's progress.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ProgressUpdate {
    id: RequestId,
    progress: Progress,
}

/// A warning for the maker. Uninhabited: the engine raises none the shell shows,
/// so every success carries `warnings: []`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub enum WarningDto {}
