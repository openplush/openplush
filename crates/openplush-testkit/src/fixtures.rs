//! Fixture loader: `fixtures/<kind>/<name>.<ext>` by kind and name.

use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;

/// A fixture directory and the file extension its fixtures share.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Clean models: `fixtures/meshes/*.obj`.
    Mesh,
    /// One model per defect class: `fixtures/defects/*.obj`.
    Defect,
    /// Reference seam sets, named after their mesh: `fixtures/seams/*.json`.
    Seams,
}

impl Kind {
    fn dir(self) -> &'static str {
        match self {
            Self::Mesh => "meshes",
            Self::Defect => "defects",
            Self::Seams => "seams",
        }
    }

    fn extension(self) -> &'static str {
        match self {
            Self::Mesh | Self::Defect => "obj",
            Self::Seams => "json",
        }
    }
}

/// Every way loading a fixture fails.
#[derive(Debug, Error)]
pub enum FixtureError {
    /// A name must be a bare file stem, so it cannot reach outside its directory.
    #[error(
        "{name:?} is not a fixture name (a file stem without '/', '\\', '.' prefix or extension)"
    )]
    BadName {
        /// The rejected name.
        name: String,
    },
    /// No fixture file at the path the name resolves to.
    #[error("no fixture at {}", path.display())]
    NotFound {
        /// The path looked up.
        path: PathBuf,
        /// The underlying error.
        source: io::Error,
    },
    /// An entry with a fixture's extension that is not a regular file.
    #[error("{} is not a file", path.display())]
    NotAFile {
        /// The entry.
        path: PathBuf,
    },
    /// The file or directory exists but could not be read.
    #[error("could not read {}: {source}", path.display())]
    Read {
        /// The path being read.
        path: PathBuf,
        /// The underlying error.
        source: io::Error,
    },
}

/// The bytes of the fixture `name` of `kind`.
pub fn load(kind: Kind, name: &str) -> Result<Vec<u8>, FixtureError> {
    check_name(name)?;
    let path = kind_dir(kind).join(format!("{name}.{}", kind.extension()));
    fs::read(&path).map_err(|source| io_error(path, source))
}

/// The names of every fixture of `kind`, sorted, so a test can run on each one.
/// Every entry with the kind's extension counts: a stem [`load`] would refuse is
/// `BadName` and a directory is `NotAFile`, never skipped.
pub fn names(kind: Kind) -> Result<Vec<String>, FixtureError> {
    names_in(&kind_dir(kind), kind.extension())
}

fn names_in(dir: &Path, extension: &str) -> Result<Vec<String>, FixtureError> {
    let entries = fs::read_dir(dir).map_err(|source| io_error(dir.to_owned(), source))?;
    let mut names = Vec::new();
    for entry in entries {
        let path = entry
            .map_err(|source| io_error(dir.to_owned(), source))?
            .path();
        if path.extension() != Some(OsStr::new(extension)) {
            continue;
        }
        let stem = path.file_stem().unwrap_or_default();
        let Some(stem) = stem.to_str() else {
            return Err(FixtureError::BadName {
                name: stem.to_string_lossy().into_owned(),
            });
        };
        check_name(stem)?;
        if !path.is_file() {
            return Err(FixtureError::NotAFile { path });
        }
        names.push(stem.to_owned());
    }
    names.sort();
    Ok(names)
}

fn kind_dir(kind: Kind) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(kind.dir())
}

fn check_name(name: &str) -> Result<(), FixtureError> {
    let bare = !name.is_empty() && !name.starts_with('.') && !name.contains(['/', '\\', '.']);
    if bare {
        Ok(())
    } else {
        Err(FixtureError::BadName {
            name: name.to_owned(),
        })
    }
}

fn io_error(path: PathBuf, source: io::Error) -> FixtureError {
    if source.kind() == io::ErrorKind::NotFound {
        FixtureError::NotFound { path, source }
    } else {
        FixtureError::Read { path, source }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{FixtureError, names_in};

    #[test]
    fn a_directory_with_a_fixture_extension_is_refused_not_listed() {
        let dir = std::env::temp_dir().join(format!("openplush-names-{}", std::process::id()));
        fs::create_dir_all(dir.join("nested.obj")).expect("scratch dir");
        fs::write(dir.join("ok.obj"), b"v 0 0 0\n").expect("scratch file");
        let result = names_in(&dir, "obj");
        fs::remove_dir_all(&dir).expect("cleanup");
        assert!(
            matches!(result, Err(FixtureError::NotAFile { .. })),
            "{result:?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_non_utf8_stem_is_a_bad_name() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        let dir = std::env::temp_dir().join(format!("openplush-utf8-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("scratch dir");
        fs::write(dir.join(OsStr::from_bytes(b"bad\xff.obj")), b"").expect("scratch file");
        let result = names_in(&dir, "obj");
        fs::remove_dir_all(&dir).expect("cleanup");
        assert!(
            matches!(result, Err(FixtureError::BadName { .. })),
            "{result:?}"
        );
    }
}
