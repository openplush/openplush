use std::path::Path;

use openplush_io::{IoError, obj, ply, stl};
use openplush_mesh::TriangleSoup;

use crate::{EngineError, StageResult, Staged, WarningSet};

/// The file to read and every file the shell can supply beside it, by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportFiles {
    /// The file the maker opened; its extension picks the reader.
    pub primary: ImportFile,
    /// The files beside it, looked up by the name the primary refers to them with.
    pub others: Vec<ImportFile>,
}

/// One file of an [`ImportFiles`] set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportFile {
    /// A relative path with `/` separators.
    pub name: String,
    /// The file's contents.
    pub bytes: Vec<u8>,
}

/// Reads the primary file with the reader its extension names, ASCII case-insensitively.
///
/// # Errors
/// `UnknownFormat` when no reader takes the extension; otherwise the reader's refusal:
/// `Parse`, `NonFiniteInput`, or `Mesh`.
pub fn import(files: &ImportFiles) -> StageResult<TriangleSoup> {
    let name = &files.primary.name;
    let read = reader(name).ok_or_else(|| EngineError::UnknownFormat { name: name.clone() })?;
    Ok(Staged {
        value: read(&files.primary.bytes).map_err(refusal)?,
        warnings: WarningSet::default(),
    })
}

type Reader = fn(&[u8]) -> Result<TriangleSoup, IoError>;

/// The reader the final path segment's extension names.
fn reader(name: &str) -> Option<Reader> {
    let segment = name.rsplit('/').next().unwrap_or(name);
    let extension = Path::new(segment).extension()?.to_str()?;
    let readers: [(&str, Reader); 3] = [("obj", obj::read), ("stl", stl::read), ("ply", ply::read)];
    readers
        .into_iter()
        .find(|(e, _)| extension.eq_ignore_ascii_case(e))
        .map(|(_, read)| read)
}

fn refusal(e: IoError) -> EngineError {
    match e {
        IoError::Parse { format, detail } => EngineError::Parse { format, detail },
        IoError::NonFiniteInput { vertices } => EngineError::NonFiniteInput { vertices },
        IoError::Mesh(inner) => EngineError::Mesh(inner),
    }
}

#[cfg(test)]
mod tests {
    use openplush_mesh::{FaceId, MeshError};

    use super::*;

    #[test]
    fn a_soup_the_mesh_crate_refuses_is_mesh_carrying_that_refusal() {
        let refused = MeshError::IndexOutOfRange {
            triangle: FaceId::new(4),
            index: 7,
            vertex_count: 3,
        };
        assert_eq!(
            refusal(IoError::Mesh(refused.clone())),
            EngineError::Mesh(refused)
        );
    }
}
