use thiserror::Error;

/// Every way reading a project document fails.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ProjectError {
    /// A provenance key is not in the canonical form the engine writes.
    #[error("\"{text}\" is not a provenance key")]
    BadKey {
        /// The text given.
        text: String,
    },
}
