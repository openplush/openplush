//! The import set for a model file: the model, and for `.gltf` the files that may hold its
//! buffers and images.

use std::fs;
use std::path::Path;

use openplush_pipeline::{ImportFile, ImportFiles};

use crate::failure::{Failure, io};

/// The most a model and the files beside it may add up to, in bytes.
pub(crate) const CAP: u64 = 1 << 30;

/// `model`, and for a `.gltf` every regular file below its directory, named relative to it
/// with `/` and listed in byte order of those names. Symlinks are not followed.
///
/// # Errors
/// `Io` for a file or directory that cannot be read; `TooLarge` as soon as the set passes
/// `cap`, before the file that passes it is read.
pub(crate) fn files(model: &Path, cap: u64) -> Result<ImportFiles, Failure> {
    let mut set = Gather {
        model,
        cap,
        total: 0,
        others: Vec::new(),
    };
    let bytes = set.read(model)?;
    let gltf = model
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("gltf"));
    if gltf {
        let dir = model
            .parent()
            .filter(|d| !d.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        set.walk(dir, "")?;
        set.others
            .sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
    }
    let name = model
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(ImportFiles {
        primary: ImportFile { name, bytes },
        others: set.others,
    })
}

struct Gather<'p> {
    model: &'p Path,
    cap: u64,
    total: u64,
    others: Vec<ImportFile>,
}

impl Gather<'_> {
    /// `path`'s bytes, counted against the cap.
    fn read(&mut self, path: &Path) -> Result<Vec<u8>, Failure> {
        let size = fs::metadata(path).map_err(io(path))?.len();
        self.total = self.total.saturating_add(size);
        if self.total > self.cap {
            return Err(Failure::TooLarge {
                model: self.model.to_path_buf(),
                bytes: self.total,
                cap: self.cap,
            });
        }
        fs::read(path).map_err(io(path))
    }

    /// Every regular file below `dir` but the model, named `prefix` plus its path from `dir`.
    fn walk(&mut self, dir: &Path, prefix: &str) -> Result<(), Failure> {
        for entry in fs::read_dir(dir).map_err(io(dir))? {
            let entry = entry.map_err(io(dir))?;
            let path = entry.path();
            // `file_type` does not follow a symlink, so a link is neither a file nor a directory.
            let kind = entry.file_type().map_err(io(&path))?;
            let name = format!("{prefix}{}", entry.file_name().to_string_lossy());
            if kind.is_dir() {
                self.walk(&path, &format!("{name}/"))?;
            } else if kind.is_file()
                && !(prefix.is_empty()
                    && Some(entry.file_name().as_os_str()) == self.model.file_name())
            {
                let bytes = self.read(&path)?;
                self.others.push(ImportFile { name, bytes });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
