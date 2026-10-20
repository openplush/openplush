use std::path::Path;

use openplush_io::gltf::{self, Ignored, Imported};
use openplush_io::{IoError, obj, ply, stl};
use openplush_mesh::TriangleSoup;

use crate::{
    EngineError, NonBlank, Remedy, StageResult, Staged, Warning, WarningCode, WarningSet,
    WarningTarget,
};

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
/// What a glTF reader skipped (skins, morph targets, animations, point and line
/// primitives) is one `import-ignored` warning.
///
/// # Errors
/// `UnknownFormat` when no reader takes the extension; otherwise the reader's refusal:
/// `Parse`, `MissingResource`, `NonFiniteInput`, or `Mesh`.
pub fn import(files: &ImportFiles) -> StageResult<TriangleSoup> {
    let name = &files.primary.name;
    let bytes = &files.primary.bytes;
    let imported = match reader(name) {
        Some(Reader::Plain(read)) => read(bytes).map(|soup| Imported {
            soup,
            ignored: Vec::new(),
        }),
        Some(Reader::Gltf) => gltf::read(name, bytes, &|wanted| beside(files, wanted)),
        None => return Err(EngineError::UnknownFormat { name: name.clone() }),
    }
    .map_err(refusal)?;
    let mut warnings = WarningSet::default();
    if !imported.ignored.is_empty() {
        warnings.insert(ignored_warning(&imported.ignored));
    }
    Ok(Staged {
        value: imported.soup,
        warnings,
    })
}

enum Reader {
    Plain(fn(&[u8]) -> Result<TriangleSoup, IoError>),
    Gltf,
}

/// The reader the final path segment's extension names.
fn reader(name: &str) -> Option<Reader> {
    let segment = name.rsplit('/').next().unwrap_or(name);
    let extension = Path::new(segment).extension()?.to_str()?;
    let readers = [
        ("obj", Reader::Plain(obj::read)),
        ("stl", Reader::Plain(stl::read)),
        ("ply", Reader::Plain(ply::read)),
        ("gltf", Reader::Gltf),
        ("glb", Reader::Gltf),
    ];
    readers
        .into_iter()
        .find(|(e, _)| extension.eq_ignore_ascii_case(e))
        .map(|(_, read)| read)
}

/// The first supplied file named exactly `wanted`.
fn beside<'f>(files: &'f ImportFiles, wanted: &str) -> Option<&'f [u8]> {
    files
        .others
        .iter()
        .find(|f| f.name == wanted)
        .map(|f| f.bytes.as_slice())
}

fn refusal(e: IoError) -> EngineError {
    match e {
        IoError::Parse { format, detail } => EngineError::Parse { format, detail },
        IoError::NonFiniteInput { vertices } => EngineError::NonFiniteInput { vertices },
        IoError::MissingResource { uri } => EngineError::MissingResource { uri },
        IoError::Mesh(inner) => EngineError::Mesh(inner),
    }
}

fn ignored_warning(ignored: &[Ignored]) -> Warning {
    // INVARIANT: both texts are non-blank literals or built around them.
    let text = |t: String| NonBlank::new(t).unwrap_or_else(|| unreachable!());
    Warning::new(
        WarningCode::ImportIgnored,
        WarningTarget::Project,
        text(ignored_text(ignored)),
        Remedy::Advisory {
            what_to_change: text(
                "To pattern a posed or morphed shape, apply the pose or morph in your 3D tool \
                 and export again."
                    .into(),
            ),
        },
    )
}

fn ignored_text(ignored: &[Ignored]) -> String {
    let parts: Vec<String> = ignored
        .iter()
        .map(|i| match *i {
            Ignored::Skin { node } => format!("the skin on node {node}, read in its bind pose"),
            Ignored::MorphTargets { node, mesh } => {
                format!("the morph targets of mesh {mesh} (node {node}), read as the base mesh")
            }
            Ignored::Animation { animation } => format!("animation {animation}, not played"),
            Ignored::Primitive {
                node,
                mesh,
                primitive,
            } => format!(
                "primitive {primitive} of mesh {mesh} (node {node}), which is points or lines"
            ),
        })
        .collect();
    format!(
        "The file has parts the pattern does not follow: {}. The pattern is made from the \
         triangles as modelled.",
        parts.join("; ")
    )
}

#[cfg(test)]
mod tests {
    use openplush_io::gltf::Ignored;
    use openplush_mesh::{FaceId, MeshError};

    use super::*;

    #[test]
    fn each_kind_of_skipped_part_is_named() {
        assert_eq!(
            ignored_text(&[
                Ignored::Skin { node: 2 },
                Ignored::MorphTargets { node: 2, mesh: 1 },
                Ignored::Animation { animation: 0 },
            ]),
            "The file has parts the pattern does not follow: the skin on node 2, read in its \
             bind pose; the morph targets of mesh 1 (node 2), read as the base mesh; animation \
             0, not played. The pattern is made from the triangles as modelled."
        );
    }

    #[test]
    fn a_soup_the_mesh_crate_refuses_is_mesh_carrying_that_refusal() {
        let refused = MeshError::IndexOutOfRange {
            triangle: FaceId::new(4),
            index: 7,
            vertex_count: 3,
        };
        assert_eq!(refusal(IoError::Mesh(refused)), EngineError::Mesh(refused));
    }
}
