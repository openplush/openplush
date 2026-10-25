//! Scratch folders shared by the CLI's test files.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

/// A fresh directory for one test's files, removed when the test ends.
pub(crate) struct Scratch(pub(crate) PathBuf);

impl std::ops::Deref for Scratch {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

static NEXT: AtomicU32 = AtomicU32::new(0);

/// A new folder named after `test`; a count in the path keeps two calls with one name apart,
/// since parallel tests that shared a folder deleted each other's files.
pub(crate) fn scratch(test: &str) -> std::io::Result<Scratch> {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("openplush-cli-{}-{n}-{test}", std::process::id()));
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    }
    fs::create_dir_all(&dir)?;
    Ok(Scratch(dir))
}
