//! Fixture loader: `fixtures/<kind>/<name>.<ext>` by kind and name.

use std::fs;
use std::io;
use std::path::PathBuf;

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
        "{name:?} is not a fixture name (a file stem without '/', '\\\\', '.' prefix or extension)"
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
/// A file whose stem [`load`] would refuse is a `BadName` error, not skipped.
pub fn names(kind: Kind) -> Result<Vec<String>, FixtureError> {
    let dir = kind_dir(kind);
    let entries = fs::read_dir(&dir).map_err(|source| io_error(dir.clone(), source))?;
    let mut names = Vec::new();
    for entry in entries {
        let path = entry
            .map_err(|source| io_error(dir.clone(), source))?
            .path();
        if path.extension().and_then(|e| e.to_str()) == Some(kind.extension())
            && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
        {
            check_name(stem)?;
            names.push(stem.to_owned());
        }
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
