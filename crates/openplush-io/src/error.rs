use openplush_mesh::{MeshError, SourceCorner};
use thiserror::Error;

use crate::Format;

/// Every way an import fails.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum IoError {
    /// The reader for `format` could not read the file.
    #[error("could not read the {} file: {detail}", .format.name())]
    Parse {
        /// The reader that refused the file.
        format: Format,
        /// What it refused and where, in the reader's words.
        detail: String,
    },
    /// Source vertices with a NaN or infinite coordinate; import refuses rather than drops them.
    #[error("source vertices with a NaN or infinite coordinate: {}", .vertices.len())]
    NonFiniteInput {
        /// Every non-finite source vertex, used by a face or not, in file order.
        vertices: Vec<SourceCorner>,
    },
    /// The file refers to another file the shell did not supply beside it.
    #[error("the file refers to \"{uri}\", which was not supplied beside it")]
    MissingResource {
        /// The reference, as the file writes it.
        uri: String,
    },
    /// The triangles do not fit a `TriangleSoup`.
    #[error(transparent)]
    Mesh(#[from] MeshError),
}
