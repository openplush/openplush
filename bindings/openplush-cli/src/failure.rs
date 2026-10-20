use std::fmt;
use std::io;
use std::path::PathBuf;

use openplush_pipeline::EngineError;

/// Why a command wrote nothing: exit 1, with this on stderr after `openplush-cli: `.
#[derive(Debug)]
pub(crate) enum Failure {
    /// A file could not be read or written.
    Io { path: PathBuf, source: io::Error },
    /// The files beside a model add up to more than the CLI reads.
    TooLarge {
        model: PathBuf,
        bytes: u64,
        cap: u64,
    },
    /// The engine refused.
    Engine(EngineError),
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "{}: {source}", path.display()),
            Self::TooLarge { model, bytes, cap } => write!(
                f,
                "{} and the files beside it are {bytes} bytes, over the {cap}-byte limit",
                model.display()
            ),
            Self::Engine(e) => write!(f, "{}: {e}", e.code()),
        }
    }
}

impl From<EngineError> for Failure {
    fn from(e: EngineError) -> Self {
        Self::Engine(e)
    }
}

/// `source`, from reading or writing `path`.
pub(crate) fn io(path: impl Into<PathBuf>) -> impl FnOnce(io::Error) -> Failure {
    let path = path.into();
    move |source| Failure::Io { path, source }
}
