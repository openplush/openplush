//! STL geometry, binary or ASCII.
//!
//! STL stores each triangle's corners, so a soup has three vertices per triangle in file
//! order and the weld joins them. A corner's provenance is its primitive and its vertex
//! position: primitive 0 for a binary file, and the solid's index for ASCII. Facet
//! normals and binary attribute bytes are not read; no UVs or materials exist.

mod ascii;

use openplush_geom::DVec3;
use openplush_mesh::{SourceCorner, SourceMaterials, TriangleSoup};

use crate::{Format, IoError};

const HEADER: usize = 80;
const COUNT_END: usize = 84;
const RECORD: usize = 50;

/// Triangle corners in file order, three per triangle, with the solid each belongs to.
struct Corners {
    positions: Vec<DVec3>,
    solid_of_triangle: Vec<u32>,
    solids: u32,
}

/// Reads STL bytes into a soup.
///
/// A file is binary when its length is exactly `84 + 50 * n` for the `n` its header
/// declares, otherwise ASCII text starting with `solid`.
///
/// Errors: [`IoError::Parse`] for a file that is neither, or ASCII text it cannot read
/// (its detail then starts `line N:`); [`IoError::NonFiniteInput`] naming every vertex
/// with a NaN or infinite coordinate.
pub fn read(bytes: &[u8]) -> Result<TriangleSoup, IoError> {
    let declared = declared_len(bytes);
    let corners = if declared == Some(bytes.len() as u64) {
        binary(bytes)
    } else {
        let text = std::str::from_utf8(bytes)
            .ok()
            .map(|t| t.trim_start_matches('\u{feff}'))
            .filter(|t| {
                t.trim_start()
                    .get(..5)
                    .is_some_and(|s| s.eq_ignore_ascii_case("solid"))
            });
        match text {
            Some(text) => ascii::read(text)?,
            None => return Err(parse_error(not_stl(bytes.len(), declared))),
        }
    };
    build(corners)
}

/// The file length the header declares, if there is a header.
fn declared_len(bytes: &[u8]) -> Option<u64> {
    let count = bytes.get(HEADER..COUNT_END)?;
    let count = u32::from_le_bytes(count.try_into().ok()?);
    Some(COUNT_END as u64 + RECORD as u64 * u64::from(count))
}

fn not_stl(len: usize, declared: Option<u64>) -> String {
    match declared {
        Some(declared) => format!(
            "{len} bytes are neither ASCII text starting with `solid` nor a binary file of the {declared} bytes its header declares"
        ),
        None => format!(
            "{len} bytes are too short for a binary header and not ASCII text starting with `solid`"
        ),
    }
}

fn parse_error(detail: String) -> IoError {
    IoError::Parse {
        format: Format::Stl,
        detail,
    }
}

fn binary(bytes: &[u8]) -> Corners {
    let coordinate = |chunk: &[u8]| {
        let mut le = [0u8; 4];
        le.copy_from_slice(chunk);
        f64::from(f32::from_le_bytes(le))
    };
    let positions: Vec<DVec3> = bytes[COUNT_END..]
        .chunks_exact(RECORD)
        .flat_map(|record| {
            // A record is a 12-byte normal, three 12-byte vertices and a 2-byte attribute.
            record[12..48].chunks_exact(12).map(move |v| {
                DVec3::new(
                    coordinate(&v[0..4]),
                    coordinate(&v[4..8]),
                    coordinate(&v[8..12]),
                )
            })
        })
        .collect();
    Corners {
        solid_of_triangle: vec![0; positions.len() / 3],
        positions,
        solids: 1,
    }
}

fn build(corners: Corners) -> Result<TriangleSoup, IoError> {
    let Corners {
        positions,
        solid_of_triangle,
        solids,
    } = corners;
    let count = positions.len();
    let Ok(last) = u32::try_from(count) else {
        return Err(parse_error(format!(
            "{count} vertices exceed the u32 index space"
        )));
    };
    let non_finite: Vec<SourceCorner> = positions
        .iter()
        .zip(0..last)
        .filter(|(p, _)| !p.is_finite())
        .map(|(_, vertex)| SourceCorner {
            primitive: solid_of_triangle[vertex as usize / 3],
            vertex,
        })
        .collect();
    if !non_finite.is_empty() {
        return Err(IoError::NonFiniteInput {
            vertices: non_finite,
        });
    }
    let triangles = (0..last / 3)
        .map(|t| [3 * t, 3 * t + 1, 3 * t + 2])
        .collect();
    let sources = (0..last)
        .map(|vertex| SourceCorner {
            primitive: solid_of_triangle[vertex as usize / 3],
            vertex,
        })
        .collect();
    let materials = SourceMaterials::new(Vec::new(), vec![None; solids as usize]);
    Ok(TriangleSoup::new(positions, triangles, sources, None)?.with_materials(materials, 0)?)
}
