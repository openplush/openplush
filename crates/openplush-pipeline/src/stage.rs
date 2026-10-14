use crate::{EngineError, WarningSet};

/// A stage's valid output with the warnings raised making it.
#[derive(Debug, Clone, PartialEq)]
pub struct Staged<T> {
    /// The output.
    pub value: T,
    /// Warnings raised producing it; may be empty, and never block.
    pub warnings: WarningSet,
}

/// Every stage's result: output with its warnings, or the violated invariant and no output.
pub type StageResult<T> = Result<Staged<T>, EngineError>;
