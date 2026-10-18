//! STL geometry, binary or ASCII.

use openplush_mesh::TriangleSoup;

use crate::{Format, IoError};

/// Reads STL bytes into a soup.
pub fn read(_bytes: &[u8]) -> Result<TriangleSoup, IoError> {
    Err(IoError::Parse {
        format: Format::Stl,
        detail: String::new(),
    })
}
