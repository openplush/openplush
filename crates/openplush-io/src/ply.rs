//! PLY geometry, ASCII or binary.

use openplush_mesh::TriangleSoup;

use crate::{Format, IoError};

/// Reads PLY bytes into a soup.
pub fn read(_bytes: &[u8]) -> Result<TriangleSoup, IoError> {
    Err(IoError::Parse {
        format: Format::Ply,
        detail: String::new(),
    })
}
